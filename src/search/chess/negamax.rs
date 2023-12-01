use std::ops::{Deref, DerefMut};
use std::time::{Duration, Instant};

use itertools::Itertools;
use rand::thread_rng;

use crate::eval::Eval;
use crate::games::chess::moves::ChessMove;
use crate::games::chess::pieces::UncoloredChessPiece::Empty;
use crate::games::chess::{ChessMoveList, Chessboard};
use crate::games::{Board, BoardHistory, ColoredPiece};
use crate::search::{
    game_result_to_score, should_stop, stop_engine, BenchResult, Engine, EngineOptionType,
    EngineState, InfoCallback, Score, SearchInfo, SearchLimit, SearchResult, SearchStateWithPv,
    Searcher, TimeControl, SCORE_LOST, SCORE_TIME_UP, SCORE_WON,
};

const DEPTH_SOFT_LIMIT: usize = 100;
const DEPTH_HARD_LIMIT: usize = 128;

#[derive(Debug)]
struct History([i32; 64 * 64]);

impl Default for History {
    fn default() -> Self {
        History([0; 64 * 64])
    }
}

impl Deref for History {
    type Target = [i32; 64 * 64];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for History {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Default)]
pub struct Negamax<E: Eval<Chessboard>> {
    state: SearchStateWithPv<Chessboard, DEPTH_HARD_LIMIT>,
    eval: E,
    history: History,
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
            elapsed >= hard_limit.min(tc.remaining / 32 + tc.increment / 2)
        }
    }

    fn search(&mut self, pos: Chessboard, limit: SearchLimit) -> SearchResult<Chessboard> {
        self.state = SearchStateWithPv::initial_state(pos, self.state.info_callback);
        let mut chosen_move = self.state.best_move;
        let max_depth = DEPTH_SOFT_LIMIT.min(limit.depth) as isize;
        self.history = History::default();

        println!(
            "starting search with limit {time} ms, {fixed} fixed, {depth} depth, {nodes} nodes",
            time = limit.tc.remaining.as_millis(),
            depth = limit.depth,
            nodes = limit.nodes,
            fixed = limit.fixed_time.as_millis()
        );

        for depth in 1..=max_depth {
            self.state.depth = depth as usize;
            let iteration_score = self.negamax(pos, limit, 0, depth, SCORE_LOST, SCORE_WON);
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

        if pos.is_2fold_repetition(&self.state.history) || pos.is_50mr_draw() {
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
        let mut num_children = 0;

        let all_moves = self.order_moves(pos.pseudolegal_moves(), &pos);
        for mov in all_moves {
            let new_pos = pos.make_move(mov, Some(&mut self.state.history));
            if new_pos.is_none() {
                continue; // illegal pseudolegal move
            }
            self.state.nodes += 1;
            num_children += 1;

            let score = -self.negamax(new_pos.unwrap(), limit, ply + 1, depth - 1, -beta, -alpha);

            self.state.history.pop(&new_pos.unwrap());

            if self.state.search_cancelled || should_stop(&limit, self, self.state.start_time) {
                self.state.search_cancelled = true;
                return SCORE_TIME_UP;
            }

            best_score = best_score.max(score);
            if score <= alpha {
                continue;
            }
            alpha = score;
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
            self.history[mov.from_to_square()] += (depth * depth) as i32;
            break;
        }
        if num_children == 0 {
            game_result_to_score(pos.no_moves_result(Some(&self.state.history)), ply)
        } else {
            best_score
        }
    }

    fn qsearch(&mut self, pos: Chessboard, mut alpha: Score, beta: Score, ply: usize) -> Score {
        let mut best_score = self.eval.eval(pos);
        if best_score >= beta {
            return best_score;
        }
        alpha = alpha.max(best_score);

        let captures = self.order_moves(pos.pseudolegal_captures(), &pos);
        for mov in captures {
            let new_pos = pos.make_move(mov, Some(&mut self.state.history));
            if new_pos.is_none() {
                continue;
            }
            let score = -self.qsearch(new_pos.unwrap(), -beta, -alpha, ply + 1);
            best_score = best_score.max(score);
            if score <= alpha {
                continue;
            }
            alpha = score;
            if score >= beta {
                break;
            }
        }
        best_score
    }

    fn order_moves(&self, mut moves: ChessMoveList, board: &Chessboard) -> ChessMoveList {
        /// The move list is iterated backwards, which is why better moves get higher scores
        let score_function = |mov: &ChessMove| {
            let captured = mov.captured(board);
            if captured == Empty {
                self.history[mov.from_to_square()]
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
        self.state = SearchStateWithPv::initial_state(pos, self.state.info_callback);
        let mut limit = SearchLimit::infinite();
        limit.depth = DEPTH_SOFT_LIMIT.min(depth);
        self.state.depth = limit.depth;
        self.negamax(pos, limit, 0, limit.depth as isize, SCORE_LOST, SCORE_WON);
        self.state.to_bench_res()
    }

    fn default_bench_depth(&self) -> usize {
        6
    }

    fn stop(&mut self) -> Result<SearchResult<Chessboard>, String> {
        stop_engine(
            &self.state.initial_pos,
            self.state.best_move,
            self.state.score,
        )
    }

    fn set_info_callback(&mut self, f: InfoCallback<Chessboard>) {
        self.state.info_callback = f;
    }

    fn search_info(&self) -> SearchInfo<Chessboard> {
        self.state.to_info()
    }

    fn forget(&mut self) {
        self.state.forget();
        self.history = History::default();
    }

    fn nodes(&self) -> u64 {
        self.state.nodes
    }

    fn set_option(&mut self, option: &str, value: &str) -> Result<(), String> {
        Err(format!("Searcher {0} doesn't implement any options, so can't set option '{option}' to '{value}'", self.name()))
    }

    fn get_options(&self) -> Vec<EngineOptionType> {
        return Vec::default();
    }
}
