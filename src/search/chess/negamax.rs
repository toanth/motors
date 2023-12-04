use std::ops::{Deref, DerefMut};
use std::time::{Duration, Instant};

use rand::thread_rng;

use crate::eval::Eval;
use crate::games::chess::moves::ChessMove;
use crate::games::chess::pieces::UncoloredChessPiece::Empty;
use crate::games::chess::{ChessMoveList, Chessboard};
use crate::games::{Board, BoardHistory, ColoredPiece, ZobristHistoryBase, ZobristRepetition2Fold};
use crate::general::common::parse_int_from_str;
use crate::search::tt::TTScoreType::{Exact, LowerBound, UpperBound};
use crate::search::tt::{TTEntry, TTScoreType, DEFAULT_HASH_SIZE_MB};
use crate::search::EngineOptionName::Hash;
use crate::search::{
    game_result_to_score, should_stop, BenchResult, Engine, EngineOptionName, EngineOptionType,
    EngineUciOptionType, InfoCallback, Score, SearchInfo, SearchLimit, SearchResult,
    SearchStateWithPv, Searcher, TimeControl, SCORE_LOST, SCORE_TIME_UP, SCORE_WON,
};

const DEPTH_SOFT_LIMIT: usize = 100;
const DEPTH_HARD_LIMIT: usize = 128;

#[derive(Debug)]
struct HistoryHeuristic([i32; 64 * 64]);

impl Default for HistoryHeuristic {
    fn default() -> Self {
        HistoryHeuristic([0; 64 * 64])
    }
}

impl Deref for HistoryHeuristic {
    type Target = [i32; 64 * 64];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for HistoryHeuristic {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Default, Debug)]
struct State {
    state: SearchStateWithPv<Chessboard, DEPTH_HARD_LIMIT>,
    history: HistoryHeuristic,
}

impl Deref for State {
    type Target = SearchStateWithPv<Chessboard, DEPTH_HARD_LIMIT>;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl DerefMut for State {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl State {
    fn forget(&mut self) {
        self.state.forget();
        self.history = HistoryHeuristic::default();
    }

    fn new_search(&mut self, history: ZobristRepetition2Fold) {
        self.state.new_search(history);
        self.history = HistoryHeuristic::default();
    }
}

#[derive(Debug, Default)]
pub struct Negamax<E: Eval<Chessboard>> {
    state: State,
    eval: E,
}

impl<E: Eval<Chessboard>> Searcher<Chessboard> for Negamax<E> {
    fn name(&self) -> &'static str {
        "Chess Negamax"
    }

    fn time_up(&self, tc: TimeControl, hard_limit: Duration, start_time: Instant) -> bool {
        if self.state.nodes % 1024 != 0 {
            false
        } else {
            let elapsed = start_time.elapsed();
            let divisor = tc.moves_to_go as u32 + 2;
            elapsed >= hard_limit.min(tc.remaining / divisor + tc.increment / 2)
        }
    }

    fn search(
        &mut self,
        pos: Chessboard,
        limit: SearchLimit,
        history: ZobristHistoryBase,
    ) -> SearchResult<Chessboard> {
        self.state.new_search(ZobristRepetition2Fold(history));
        let mut chosen_move = self.state.best_move;
        let max_depth = DEPTH_SOFT_LIMIT.min(limit.depth) as isize;

        println!(
            "starting search with limit {time} ms, {fixed} fixed, {depth} depth, {nodes} nodes, will take at most {max}ms",
            time = limit.tc.remaining.as_millis(),
            depth = limit.depth,
            nodes = limit.nodes,
            fixed = limit.fixed_time.as_millis(),
            max= (limit.tc.remaining / 32 + limit.tc.increment / 2).min(limit.fixed_time).as_millis(),
        );

        for depth in 1..=max_depth {
            self.state.depth = depth as usize;
            let iteration_score = self.negamax(pos, limit, 0, depth, SCORE_LOST, SCORE_WON);
            assert!(!iteration_score
                .plies_until_game_won()
                .is_some_and(|x| x == 0));
            if self.state.search_cancelled || should_stop(&limit, self, self.state.start_time) {
                break;
            }
            self.state.score = iteration_score;
            chosen_move = self.state.best_move; // only set now so that incomplete iterations are discarded
            self.state.info_callback.call(self.search_info())
        }

        SearchResult::move_and_score(
            chosen_move.unwrap_or_else(|| {
                eprintln!("Warning: Not even a single iteration finished");
                let mut rng = thread_rng();
                pos.random_legal_move(&mut rng)
                    .expect("search() called in a position with no legal moves")
            }),
            self.state.score,
        )
    }
}

impl<E: Eval<Chessboard>> Negamax<E> {
    fn negamax(
        &mut self,
        pos: Chessboard,
        limit: SearchLimit,
        ply: usize,
        mut depth: isize,
        mut alpha: Score,
        beta: Score,
    ) -> Score {
        debug_assert!(alpha < beta);
        debug_assert!(ply <= DEPTH_HARD_LIMIT * 2);
        debug_assert!(depth <= DEPTH_SOFT_LIMIT as isize);
        debug_assert_eq!(self.state.board_history.0 .0.len(), ply); // TODO: This should fail!!

        self.state.sel_depth = self.state.sel_depth.max(ply);

        let root = ply == 0;
        let pv_node = alpha + 1 > beta;
        let original_alpha = alpha;

        if !root && (self.state.board_history.is_repetition(&pos) || pos.is_50mr_draw()) {
            return Score(0);
        }
        let in_check = pos.is_in_check();
        if in_check {
            depth += 1;
        }
        if depth <= 0 {
            return self.qsearch(pos, alpha, beta, ply);
        }

        let mut best_score = SCORE_LOST;

        let tt_entry = self.state.tt.load(pos.zobrist_hash(), ply);
        let mut best_move = tt_entry.mov;

        if !root
            && tt_entry.bound != TTScoreType::Empty
            && tt_entry.hash == pos.zobrist_hash()
            && tt_entry.depth as isize >= depth
            && ((tt_entry.score >= beta && tt_entry.bound == LowerBound)
                || (tt_entry.score <= alpha && tt_entry.bound == UpperBound)
                || tt_entry.bound == Exact)
        {
            return tt_entry.score;
        }

        let mut num_children = 0;

        let all_moves = self.order_moves(pos.pseudolegal_moves(), &pos, best_move);
        for mov in all_moves {
            let new_pos = pos.make_move(mov);
            if new_pos.is_none() {
                continue; // illegal pseudolegal move
            }
            let new_pos = new_pos.unwrap();
            self.state.nodes += 1;
            num_children += 1;

            self.state.board_history.push(&pos);
            let mut score;
            if num_children == 1 {
                score = -self.negamax(new_pos, limit, ply + 1, depth - 1, -beta, -alpha);
            } else {
                score = -self.negamax(new_pos, limit, ply + 1, depth - 1, -beta, -beta + 1);
                if alpha < score && score < beta {
                    score = -self.negamax(new_pos, limit, ply + 1, depth - 1, -beta, -alpha);
                }
            }

            self.state.state.board_history.pop(&pos);

            if self.state.search_cancelled || should_stop(&limit, self, self.state.start_time) {
                self.state.search_cancelled = true;
                return SCORE_TIME_UP;
            }

            best_score = best_score.max(score);
            if score <= alpha {
                continue;
            }
            alpha = score;
            best_move = mov;
            // TODO: This lost a smallish 2 digit amount of elo, so retest eventually
            // (will probably be much less important as the search gets slower)
            self.state.pv_table.new_pv_move(ply, mov);
            if ply == 0 {
                self.state.best_move = Some(mov);
            }
            if score < beta {
                continue;
            }
            if mov.is_capture(&pos) {
                break;
            }
            self.state.history[mov.from_to_square()] += (depth * depth) as i32;
            break;
        }

        let bound = best_score.bound(original_alpha, beta);
        let tt_entry = TTEntry::new(best_score, best_move, depth, bound, pos.zobrist_hash());
        // TODO: eventually test that not overwriting PV nodes unless the depth is quite a bit greater gains
        self.state.tt.store(tt_entry, ply);

        if num_children == 0 {
            game_result_to_score(pos.no_moves_result(), ply)
        } else {
            best_score
        }
    }

    fn qsearch(&mut self, pos: Chessboard, mut alpha: Score, beta: Score, ply: usize) -> Score {
        let original_alpha = alpha;
        let mut best_score = self.eval.eval(pos);
        if best_score >= beta {
            return best_score;
        }

        self.state.sel_depth = self.state.sel_depth.max(ply);

        alpha = alpha.max(best_score);
        // TODO: Using the TT for move ordering in qsearch was mostly elo-neutral, so retest that eventually
        // do TT cutoffs with alpha already raised by the stand pat check, because that relies on the null move observation
        // but if there's a TT entry from normal search that's worse than the stand pat score, we should trust that more.
        let tt_entry = self.state.tt.load(pos.zobrist_hash(), ply);

        // depth 0 drops immediately to qsearch, so a depth 0 entry always comes from qsearch.
        // However, if we've already done qsearch on this position, we can just re-use the result,
        // so there is no point in checking the depth at all
        // if tt_entry.hash == pos.zobrist_hash()
        //     && tt_entry.bound != TTScoreType::Empty
        //     && ((tt_entry.bound == LowerBound && tt_entry.score >= beta)
        //         || (tt_entry.bound == UpperBound && tt_entry.score <= alpha)
        //         || tt_entry.bound == Exact)
        // {
        //     return tt_entry.score;
        // }

        let mut best_move = tt_entry.mov;

        let captures = self.order_moves(pos.noisy_pseudolegal(), &pos, ChessMove::default());
        for mov in captures {
            debug_assert!(mov.is_capture(&pos)); // TODO: Separate quiet / noisy moves instead of setting captures == noisy
            let new_pos = pos.make_move(mov);
            if new_pos.is_none() {
                continue;
            }
            // TODO: Also count qsearch nodes. Because of the nodes % 1024 check in timeouts, this requires also checking for timeouts in qsearch.
            self.state.board_history.push(&pos);
            let score = -self.qsearch(new_pos.unwrap(), -beta, -alpha, ply + 1);
            self.state.board_history.pop(&pos);
            best_score = best_score.max(score);
            if score <= alpha {
                continue;
            }
            alpha = score;
            best_move = mov;
            if score >= beta {
                break;
            }
        }
        let bound = best_score.bound(original_alpha, beta);
        let tt_entry = TTEntry::new(best_score, best_move, 0, bound, pos.zobrist_hash());
        self.state.tt.store(tt_entry, ply);
        best_score
    }

    fn order_moves(
        &self,
        mut moves: ChessMoveList,
        board: &Chessboard,
        tt_move: ChessMove,
    ) -> ChessMoveList {
        /// The move list is iterated backwards, which is why better moves get higher scores
        let score_function = |mov: &ChessMove| {
            let captured = mov.captured(board);
            if *mov == tt_move {
                i32::MAX
            } else if captured == Empty {
                self.state.history[mov.from_to_square()]
            } else {
                i32::MAX - 100 + captured as i32 * 10 - mov.piece(board).uncolored() as i32
            }
        };
        moves.as_mut_slice().sort_by_cached_key(score_function);
        moves
    }
}

impl<E: Eval<Chessboard>> Engine<Chessboard> for Negamax<E> {
    fn bench(&mut self, pos: Chessboard, depth: usize) -> BenchResult {
        self.state.new_search(ZobristRepetition2Fold::default());
        let mut limit = SearchLimit::infinite();
        limit.depth = DEPTH_SOFT_LIMIT.min(depth);
        self.state.depth = limit.depth;
        self.negamax(pos, limit, 0, limit.depth as isize, SCORE_LOST, SCORE_WON);
        self.state.to_bench_res()
    }

    fn default_bench_depth(&self) -> usize {
        6
    }

    fn stop(&mut self) {
        self.state.search_cancelled = true;
    }

    fn set_info_callback(&mut self, f: InfoCallback<Chessboard>) {
        self.state.info_callback = f;
    }

    fn search_info(&self) -> SearchInfo<Chessboard> {
        self.state.to_info()
    }

    fn forget(&mut self) {
        self.state.forget();
    }

    fn nodes(&self) -> u64 {
        self.state.nodes
    }

    fn set_option(&mut self, option: EngineOptionName, value: &str) -> Result<(), String> {
        match option {
            Hash => {
                let value: usize = parse_int_from_str(value, "hash size in MB")?;
                let size = value * 1000_000;
                self.state.tt.resize_bytes(size);
                Ok(())
            }
            x => Err(format!(
                "The option '{x}' is not supported by the engine {0}",
                self.name()
            )),
        }
    }

    fn get_options(&self) -> Vec<EngineOptionType> {
        vec![EngineOptionType {
            name: Hash,
            typ: EngineUciOptionType::Spin,
            default: Some(DEFAULT_HASH_SIZE_MB.to_string()),
            min: Some(0.to_string()),
            max: Some(1_000_000.to_string()), // use at most 1 terabyte (should be enough for anybody™)
            vars: vec![],
        }]
    }
}
