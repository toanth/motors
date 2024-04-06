use std::time::{Duration, Instant};

use derive_more::{Deref, DerefMut};
use rand::thread_rng;

use gears::games::{Board, BoardHistory, ColoredPiece, ZobristHistoryBase, ZobristRepetition2Fold};
use gears::games::chess::{Chessboard, ChessMoveList};
use gears::games::chess::moves::ChessMove;
use gears::games::chess::pieces::UncoloredChessPiece::Empty;
use gears::general::common::{NamedEntity, parse_int_from_str, Res, StaticallyNamedEntity};
use gears::search::{
    Depth, game_result_to_score, MIN_SCORE_WON, NO_SCORE_YET, Nodes, Score, SCORE_LOST, SCORE_TIME_UP,
    SCORE_WON, SearchInfo, SearchLimit, SearchResult, TimeControl,
};
use gears::ugi::{EngineOption, EngineOptionName, UgiSpin};
use gears::ugi::EngineOptionName::{Hash, Threads};
use gears::ugi::EngineOptionType::Spin;

use crate::eval::Eval;
use crate::search::{
    BasicSearchState, Benchable, BenchResult, Engine, EngineInfo, NodeType, Searching,
    SearchStateWithPv,
};
use crate::search::multithreading::{NoSender, SearchSender};
use crate::search::NodeType::*;
use crate::search::tt::TTEntry;

const DEPTH_SOFT_LIMIT: Depth = Depth::new(100);
const DEPTH_HARD_LIMIT: Depth = Depth::new(128);

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Deref, DerefMut)]
struct KillerHeuristic([ChessMove; DEPTH_HARD_LIMIT.get()]);

impl Default for KillerHeuristic {
    fn default() -> Self {
        KillerHeuristic([ChessMove::default(); DEPTH_HARD_LIMIT.get()])
    }
}

#[derive(Default, Debug, Clone, Deref, DerefMut)]
pub struct State {
    #[deref]
    #[deref_mut]
    state: SearchStateWithPv<Chessboard, { DEPTH_HARD_LIMIT.get() }>,
    history: HistoryHeuristic,
    killers: KillerHeuristic,
}

impl State {
    fn forget(&mut self) {
        self.state.forget();
        self.history = HistoryHeuristic::default();
        self.killers = KillerHeuristic::default();
    }

    fn new_search(&mut self, history: ZobristRepetition2Fold) {
        self.state.new_search(history);
        self.history = HistoryHeuristic::default();
        self.killers = KillerHeuristic::default();
    }
}

impl BasicSearchState<Chessboard> for State {
    fn nodes(&self) -> Nodes {
        Nodes::new(self.nodes).unwrap()
    }

    fn searching(&self) -> Searching {
        self.searching
    }

    fn set_searching(&mut self, searching: Searching) {
        self.state.set_searching(searching);
    }

    fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn quit(&mut self) {
        self.wrapped.quit()
    }

    fn depth(&self) -> Depth {
        self.depth
    }

    fn start_time(&self) -> Instant {
        self.start_time
    }

    fn score(&self) -> Score {
        self.score
    }

    fn forget(&mut self) {
        self.state.forget();
        self.history = HistoryHeuristic::default();
        self.killers = KillerHeuristic::default();
    }

    fn new_search(&mut self, history: ZobristRepetition2Fold) {
        self.state.new_search(history);
        self.history = HistoryHeuristic::default();
        self.killers = KillerHeuristic::default();
    }

    fn to_search_info(&self) -> SearchInfo<Chessboard> {
        self.state.to_search_info()
    }
}

/// Chess-playing Alpha-beta Pruning Search, or in short, CAPS.
/// Larger than SᴍᴀʟʟCᴀᴘꜱ.
#[derive(Debug, Default)]
pub struct Caps<E: Eval<Chessboard>> {
    state: State,
    eval: E,
}

impl<E: Eval<Chessboard>> StaticallyNamedEntity for Caps<E> {
    fn static_short_name() -> &'static str
    where
        Self: Sized,
    {
        "CAPS"
    }

    fn static_long_name() -> &'static str
    where
        Self: Sized,
    {
        "CAPS: Chess-playing Alpha-beta Pruning Search"
    }

    fn static_description() -> &'static str
    where
        Self: Sized,
    {
        "Chess-playing Alpha-beta Pruning Search (CAPS), a chess engine. Currently very early in development and not yet all that strong (but still > 2k elo). Much larger than SᴍᴀʟʟCᴀᴘꜱ."
    }
}

// impl<E: Eval<Chessboard>> EngineBase for Caps<E> {}

impl<E: Eval<Chessboard>> Benchable<Chessboard> for Caps<E> {
    fn bench(&mut self, pos: Chessboard, depth: Depth) -> BenchResult {
        self.state.new_search(ZobristRepetition2Fold::default());
        let mut limit = SearchLimit::infinite();
        limit.depth = DEPTH_SOFT_LIMIT.min(depth);
        self.state.depth = limit.depth;
        self.negamax(
            pos,
            limit,
            0,
            limit.depth.get() as isize,
            SCORE_LOST,
            SCORE_WON,
            false,
            &NoSender::default(),
        );
        // TODO: Handle stop command in bench?
        self.state.to_bench_res()
    }

    fn engine_info(&self) -> EngineInfo {
        let options = vec![
            EngineOption {
                name: Hash,
                value: Spin(UgiSpin {
                    val: 4,
                    default: Some(4),
                    min: Some(0),
                    max: Some(1_000_000), // use at most 1 terabyte (should be enough for anybody™)
                }),
            },
            EngineOption {
                name: Threads,
                value: Spin(UgiSpin {
                    val: 1,
                    default: Some(1),
                    min: Some(1),
                    max: Some(100),
                }),
            },
        ];
        EngineInfo {
            name: self.long_name().to_string(),
            version: "0.0.1".to_string(),
            default_bench_depth: Depth::new(6),
            options,
            description: "CAPS (Chess Alpha-beta Pruning Search), a negamax-based chess engine"
                .to_string(),
        }
    }

    fn set_option(&mut self, name: EngineOptionName, value: String) -> Res<()> {
        match name {
            Hash => {
                let value: usize = parse_int_from_str(&value, "hash size in mb")?;
                let size = value * 1_000_000;
                self.state.tt.resize_bytes(size);
                Ok(())
            }
            x => Err(format!(
                "The option '{x}' is not supported by the engine {0}",
                self.long_name()
            )),
        }
    }
}

impl<E: Eval<Chessboard>> Engine<Chessboard> for Caps<E> {
    type State = State;

    fn new(state: Self::State) -> Self {
        Self {
            state,
            eval: E::default(),
        }
    }

    fn search(
        &mut self,
        pos: Chessboard,
        limit: SearchLimit,
        history: ZobristHistoryBase,
        sender: &mut dyn SearchSender<Chessboard>,
    ) -> Res<SearchResult<Chessboard>> {
        self.state.new_search(ZobristRepetition2Fold(history));
        let mut chosen_move = self.state.best_move;
        let max_depth = DEPTH_SOFT_LIMIT.min(limit.depth).get() as isize;

        // eprintln!(
        //     "starting search with limit {time} ms, {fixed} fixed, {depth} depth, {nodes} nodes, will take at most {max}ms",
        //     time = limit.tc.remaining.as_millis(),
        //     depth = limit.depth,
        //     nodes = limit.nodes,
        //     fixed = limit.fixed_time.as_millis(),
        //     max= (limit.tc.remaining / 32 + limit.tc.increment / 2).min(limit.fixed_time).as_millis(),
        // );

        for depth in 1..=max_depth {
            self.state.depth = Depth::new(depth as usize);
            let iteration_score =
                self.negamax(pos, limit, 0, depth, SCORE_LOST, SCORE_WON, false, sender);
            let soft_limit = limit.fixed_time.min(limit.tc.remaining / 64);
            assert!(
                !(iteration_score != SCORE_TIME_UP
                    && iteration_score
                        .plies_until_game_over()
                        .is_some_and(|x| x <= 0)),
                "score {} depth {depth}",
                iteration_score.0
            );
            if self.state.search_cancelled() {
                break;
            }
            self.state.score = iteration_score;
            chosen_move = self.state.best_move; // only set now so that incomplete iterations are discarded
            sender.send_search_info(self.search_info());
            if self.state.start_time.elapsed() >= soft_limit {
                break;
            }
        }

        Ok(SearchResult::move_and_score(
            chosen_move.unwrap_or_else(|| {
                eprintln!("Warning: Not even a single iteration finished");
                let mut rng = thread_rng();
                pos.random_legal_move(&mut rng)
                    .expect("search() called in a position with no legal moves")
            }),
            self.state.score,
        ))
    }

    fn time_up(&self, tc: TimeControl, fixed_time: Duration, start_time: Instant) -> bool {
        if self.state.nodes % 1024 != 0 {
            false
        } else {
            let elapsed = start_time.elapsed();
            // divide by 4 unless moves to go is very small, but don't divide by 1 (or zero) to avoid timeouts
            let divisor = tc.moves_to_go.unwrap_or(usize::MAX).clamp(2, 4) as u32;
            // TODO: Technically, this can lead to timeouts if increment > remaining, although that can't happen
            // unless increment was > timeout since the start or there's a lot of move overhead
            elapsed >= fixed_time.min(tc.remaining / divisor + tc.increment / 2)
        }
    }

    fn search_state(&self) -> &Self::State {
        &self.state
    }

    fn search_state_mut(&mut self) -> &mut Self::State {
        &mut self.state
    }

    fn can_use_multiple_threads() -> bool
    where
        Self: Sized,
    {
        true
    }
}

#[allow(clippy::too_many_arguments)]
impl<E: Eval<Chessboard>> Caps<E> {
    fn negamax(
        &mut self,
        pos: Chessboard,
        limit: SearchLimit,
        ply: usize,
        mut depth: isize,
        mut alpha: Score,
        beta: Score,
        allow_nmp: bool, // TODO: Use search stack
        sender: &dyn SearchSender<Chessboard>,
    ) -> Score {
        debug_assert!(alpha < beta);
        debug_assert!(ply <= DEPTH_HARD_LIMIT.get() * 2);
        debug_assert!(depth <= DEPTH_SOFT_LIMIT.get() as isize);
        debug_assert!(self.state.board_history.0 .0.len() >= ply);

        self.state.sel_depth = self.state.sel_depth.max(ply);

        let root = ply == 0;
        let is_pvs_pv_node = alpha + 1 < beta; // TODO: Pass as parameter / generic? Probably not worth much elo
        debug_assert!(!root || is_pvs_pv_node); // root implies pv node

        if !root && (self.state.board_history.is_repetition(&pos) || pos.is_50mr_draw()) {
            return Score(0);
        }
        let in_check = pos.is_in_check();
        // Check extensions. Increase the depth by 1 if in check.
        // Do this before deciding whether to drop into qsearch.
        if in_check {
            depth += 1;
        }
        if depth <= 0 {
            return self.qsearch(pos, alpha, beta, ply);
        }
        let can_prune = !is_pvs_pv_node && !in_check;

        let mut best_score = NO_SCORE_YET;
        let mut bound_so_far = UpperBound;

        let tt_entry = self.state.tt.load(pos.zobrist_hash(), ply);
        let mut best_move = tt_entry.mov;
        let trust_tt_entry =
            tt_entry.bound != NodeType::Empty && tt_entry.hash == pos.zobrist_hash();

        // TT cutoffs. If we've already seen this position, and the TT entry has more valuable information (higher depth),
        // and we're not a PV node, and the saved score is either exact or at least known to be outside of [alpha, beta),
        // simply return it.
        if !is_pvs_pv_node
            && trust_tt_entry
            && tt_entry.depth as isize >= depth
            && ((tt_entry.score >= beta && tt_entry.bound == LowerBound)
                || (tt_entry.score <= alpha && tt_entry.bound == UpperBound)
                || tt_entry.bound == Exact)
        {
            return tt_entry.score;
        }

        let eval = self.eval.eval(pos);
        debug_assert!(!eval.is_game_over_score());
        //     match trust_tt_entry {
        //     true => tt_entry.score,
        //     false => self.eval.eval(pos),
        // };

        // Reverse Futility Pruning (RFP): If eval is far above beta, it's likely that our opponent
        // blundered in a previous move of the search, do if the depth is low, don't even bother searching further.
        if can_prune {
            if depth < 4 && eval >= beta + Score(80 * depth as i32) {
                return eval;
            }

            // Null Move Pruning (NMP). If static eval of our position is above beta, this node probably isn't that interesting.
            // To test this hypothesis, do a nullmove and perform a search with reduced depth; if the result is still
            // above beta, then it's very likely that the score would have been above beta if we had played a move,
            // so simply return the nmp score.
            if allow_nmp && depth >= 3 && eval >= beta {
                self.state.board_history.push(&pos);
                let new_pos = pos.make_nullmove().unwrap();
                let reduction = 1 + depth / 4;
                let score = -self.negamax(
                    new_pos,
                    limit,
                    ply + 1,
                    depth - 1 - reduction,
                    -beta,
                    -beta + 1,
                    false,
                    sender,
                );
                self.state.board_history.pop(&pos);
                if score >= beta {
                    return score.min(MIN_SCORE_WON);
                }
            }
        }

        let mut num_children = 0;

        let all_moves = self.order_moves(pos.pseudolegal_moves(), &pos, best_move, ply);
        for mov in all_moves {
            let new_pos = pos.make_move(mov);
            if new_pos.is_none() {
                continue; // illegal pseudolegal move
            }
            let new_pos = new_pos.unwrap();
            self.state.nodes += 1;
            num_children += 1;

            let debug_history_len = self.state.board_history.0 .0.len();

            self.state.board_history.push(&pos);
            // PVS: Assume that the TT move is the best move, so we only need to prove that the other moves are worse,
            // which we can do with a zero window search. Should this assumption fail, re-search with a full window.
            let mut score;
            if num_children == 1 {
                score = -self.negamax(
                    new_pos,
                    limit,
                    ply + 1,
                    depth - 1,
                    -beta,
                    -alpha,
                    true,
                    sender,
                );
            } else {
                score = -self.negamax(
                    new_pos,
                    limit,
                    ply + 1,
                    depth - 1,
                    -(alpha + 1),
                    -alpha,
                    true,
                    sender,
                );
                if alpha < score && score < beta {
                    score = -self.negamax(
                        new_pos,
                        limit,
                        ply + 1,
                        depth - 1,
                        -beta,
                        -alpha,
                        true,
                        sender,
                    );
                }
            }

            self.state.board_history.pop(&pos);

            debug_assert_eq!(
                self.state.board_history.0.0.len(),
                debug_history_len,
                "depth {depth} ply {ply} old len {debug_history_len} new len {} child {num_children}", self.state.board_history.0.0.len()
            );
            // Check for cancellation right after searching a move to avoid storing incorrect information in the TT.
            if self.should_stop(limit, sender) {
                return SCORE_TIME_UP;
            }
            debug_assert!(score.0.abs() <= SCORE_WON.0, "score {} ply {ply}", score.0);

            best_score = best_score.max(score);
            // Save indentation by using `continue` instead of nested if statements.
            if score <= alpha {
                continue;
            }
            bound_so_far = Exact;
            alpha = score;
            best_move = mov;
            if ply == 0 {
                self.state.best_move = Some(mov);
            }
            if score < beta {
                continue;
            }
            bound_so_far = LowerBound;
            if mov.is_capture(&pos) {
                // TODO: Run test with regression bounds for using is_noisy (can cache that)
                break;
            }
            // Update various heuristics, TODO: More (killers, history gravity, etc)
            self.state.history[mov.from_to_square()] += (depth * depth) as i32;
            self.state.killers[ply] = mov;
            break;
        }

        if num_children == 0 {
            // TODO: Merge cached in-check branch
            return game_result_to_score(pos.no_moves_result(), ply);
        }

        let tt_entry = TTEntry::new(
            best_score,
            best_move,
            depth,
            bound_so_far,
            pos.zobrist_hash(),
        );
        // TODO: eventually test that not overwriting PV nodes unless the depth is quite a bit greater gains
        // Store the results in the TT, always replacing the previous entry. Note that the TT move is only overwritten
        // if this node was an exact or fail high node.
        self.state.tt.store(tt_entry, ply);

        if bound_so_far == Exact {
            // TODO: Test if this loses elo (a different implementation used to lose a few elo points)
            self.state.pv_table.new_pv_move(ply, best_move);
        } else {
            self.state.pv_table.no_pv_move(ply);
        }

        best_score
    }

    /// Search only noisy moves to quieten down the position before calling eval.

    fn qsearch(&mut self, pos: Chessboard, mut alpha: Score, beta: Score, ply: usize) -> Score {
        // The stand pat check. Since we're not looking at all moves, it's very likely that there's a move we didn't
        // look at that doesn't make our position worse, so we don't want to assume that we have to play a capture.
        let mut best_score = self.eval.eval(pos);
        let mut bound_so_far = UpperBound;
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

        let captures = self.order_moves(pos.noisy_pseudolegal(), &pos, ChessMove::default(), ply);
        for mov in captures {
            debug_assert!(mov.is_noisy(&pos));
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
            bound_so_far = Exact;
            alpha = score;
            best_move = mov;
            if score >= beta {
                bound_so_far = LowerBound;
                break;
            }
        }
        let tt_entry = TTEntry::new(best_score, best_move, 0, bound_so_far, pos.zobrist_hash());
        self.state.tt.store(tt_entry, ply);
        debug_assert!(!best_score.is_game_over_score());
        best_score
    }

    /// Order moves so that the most promising moves are searched first.
    /// The most promising move is always the TT move, because that is backed up by search.
    /// After that follow various heuristics.

    fn order_moves(
        &self,
        mut moves: ChessMoveList,
        board: &Chessboard,
        tt_move: ChessMove,
        ply: usize,
    ) -> ChessMoveList {
        // The move list is iterated backwards, which is why better moves get higher scores
        let score_function = |mov: &ChessMove| {
            let captured = mov.captured(board);
            if *mov == tt_move {
                i32::MAX
            } else if *mov == self.state.killers[ply] {
                i32::MAX - 200
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

#[cfg(test)]
mod tests {
    use gears::games::chess::Chessboard;

    use super::*;

    #[test]
    fn simple_search_test() {
        let pos = Chessboard::from_fen(
            "r2q1r2/ppp1pkb1/2n1p1pp/2N1P3/2pP2Q1/2P1B2P/PP3PP1/R4RK1 b - - 1 18",
        );
        // TODO: Write test
    }
}
