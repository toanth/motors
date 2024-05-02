use std::cmp::min;
use std::time::{Duration, Instant};

use derive_more::{Deref, DerefMut};
use rand::thread_rng;

use gears::games::chess::moves::ChessMove;
use gears::games::chess::pieces::UncoloredChessPiece::Empty;
use gears::games::chess::{ChessMoveList, Chessboard};
use gears::games::{Board, BoardHistory, ColoredPiece, ZobristRepetition2Fold};
use gears::general::common::{NamedEntity, Res, StaticallyNamedEntity};
use gears::output::Message::Debug;
use gears::search::{
    game_result_to_score, Depth, Score, SearchLimit, SearchResult, TimeControl, MAX_SCORE_LOST,
    MIN_SCORE_WON, NO_SCORE_YET, SCORE_LOST, SCORE_TIME_UP, SCORE_WON,
};
use gears::ugi::EngineOptionName::{Hash, Threads};
use gears::ugi::EngineOptionType::Spin;
use gears::ugi::{EngineOption, UgiSpin};

use crate::eval::Eval;
use crate::search::multithreading::SearchSender;
use crate::search::tt::{TTEntry, TT};
use crate::search::NodeType::*;
use crate::search::{
    ABSearchState, BenchResult, Benchable, CustomInfo, Engine, EngineInfo, NodeType, Pv,
    SearchStackEntry, SearchState,
};

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

#[derive(Debug, Clone, Default)]
struct Additional {
    history: HistoryHeuristic,
}

impl CustomInfo for Additional {}

#[derive(Debug, Default, Copy, Clone)]
struct CapsSearchStackEntry {
    killer: ChessMove,
    pv: Pv<Chessboard, { DEPTH_HARD_LIMIT.get() }>,
}

impl SearchStackEntry<Chessboard> for CapsSearchStackEntry {
    fn pv(&self) -> Option<&[ChessMove]> {
        Some(&self.pv.list[0..self.pv.length])
    }
}

type State = ABSearchState<Chessboard, CapsSearchStackEntry, Additional>;

/// Chess-playing Alpha-beta Pruning Search, or in short, CAPS.
/// Larger than SᴍᴀʟʟCᴀᴘꜱ.
#[derive(Debug)]
pub struct Caps<E: Eval<Chessboard>> {
    state: State,
    eval: E,
    tt: TT,
}

impl<E: Eval<Chessboard>> Default for Caps<E> {
    fn default() -> Self {
        Self {
            state: ABSearchState::new(DEPTH_HARD_LIMIT),
            eval: E::default(),
            tt: TT::default(),
        }
    }
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
        "Chess-playing Alpha-beta Pruning Search (CAPS), a chess engine. Currently very early in development and not yet all that strong (but still > 2k elo). Much larger than SᴍᴀʟʟCᴀᴘꜱ"
    }
}

// impl<E: Eval<Chessboard>> EngineBase for Caps<E> {}

impl<E: Eval<Chessboard>> Benchable<Chessboard> for Caps<E> {
    fn bench(&mut self, pos: Chessboard, depth: Depth) -> BenchResult {
        self.state.new_search(ZobristRepetition2Fold::default());
        let mut limit = SearchLimit::infinite();
        limit.depth = DEPTH_SOFT_LIMIT.min(depth);
        self.state.depth = limit.depth;
        let _ = self.search_from_pos(pos, limit);
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
            default_bench_depth: Depth::new(9),
            options,
            description: "CAPS (Chess Alpha-beta Pruning Search), a negamax-based chess engine"
                .to_string(),
        }
    }
}

impl<E: Eval<Chessboard>> Engine<Chessboard> for Caps<E> {
    fn set_tt(&mut self, tt: TT) {
        self.tt = tt;
    }

    fn do_search(
        &mut self,
        pos: Chessboard,
        mut limit: SearchLimit,
        sender: &mut SearchSender<Chessboard>,
    ) -> Res<SearchResult<Chessboard>> {
        limit.fixed_time = min(limit.fixed_time, limit.tc.remaining);
        let soft_limit = limit
            .fixed_time
            .min(limit.tc.remaining / 32 + limit.tc.increment / 2)
            .min(limit.tc.remaining / 4);

        sender.send_message(Debug, &format!(
            "Starting search with limit {time}ms, {incr}ms increment, max {fixed}ms, mate in {mate} plies, max depth {depth}, max {nodes} nodes, soft limit {soft}ms",
            time = limit.tc.remaining.as_millis(),
            incr = limit.tc.increment.as_millis(),
            mate = limit.mate.get(),
            depth = limit.depth.get(),
            nodes = limit.nodes.get(),
            fixed = limit.fixed_time.as_millis(),
            soft = soft_limit.as_millis(),
        ));

        let chosen_move = match self.aspiration(pos, limit, soft_limit, sender) {
            Some(mov) => mov,
            None => {
                eprintln!("Warning: Not even a single iteration finished");
                let mut rng = thread_rng();
                pos.random_legal_move(&mut rng)
                    .expect("search() called in a position with no legal moves")
            }
        };
        Ok(SearchResult::move_and_score(chosen_move, self.state.score))
    }

    fn time_up(&self, tc: TimeControl, fixed_time: Duration, start_time: Instant) -> bool {
        if self.state.nodes % 1024 != 0 {
            false
        } else {
            let elapsed = start_time.elapsed();
            // divide by 4 unless moves to go is very small, but don't divide by 1 (or zero) to avoid timeouts
            let divisor = tc.moves_to_go.unwrap_or(usize::MAX).clamp(2, 4) as u32;
            // Because fixed_time has been clamped to at most tc.remaining, this can never lead to timeouts
            // (assuming the move overhead is set correctly)
            elapsed >= fixed_time.min(tc.remaining / divisor + tc.increment / 2)
        }
    }

    fn search_state(&self) -> &impl SearchState<Chessboard> {
        &self.state
    }

    fn search_state_mut(&mut self) -> &mut impl SearchState<Chessboard> {
        &mut self.state
    }

    fn get_static_eval(&mut self, pos: Chessboard) -> Score {
        self.eval.eval(pos)
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
    /// Aspiration Windows (AW): Assume that the score will be close to the score from the previous iteration
    /// of Iterative Deepening, so use alpha, beta bounds around that score to prune more aggressively.
    /// This means that it's possible for the root to fail low (or high), which is always something to consider:
    /// For example, the best move is not trustworthy if the root failed low (but because the TT move is ordered first,
    /// and the TT move at the root is always `state.best_move` (there can be no collisions because it's written to last),
    /// it should in theory still be trustworthy if the root failed high)
    fn aspiration(
        &mut self,
        pos: Chessboard,
        limit: SearchLimit,
        soft_limit: Duration,
        sender: &mut SearchSender<Chessboard>,
    ) -> Option<ChessMove> {
        let mut chosen_move = self.state.best_move;
        let max_depth = DEPTH_SOFT_LIMIT.min(limit.depth).get() as isize;

        let mut alpha = SCORE_LOST;
        let mut beta = SCORE_WON;
        self.state.depth = Depth::new(1);

        let mut window_radius = Score(20);

        loop {
            let iteration_score = self.negamax(
                pos,
                limit,
                0,
                self.state.depth.get() as isize,
                alpha,
                beta,
                sender,
            );
            assert!(
                !(iteration_score != SCORE_TIME_UP
                    && iteration_score
                        .plies_until_game_over()
                        .is_some_and(|x| x <= 0)),
                "score {0} depth {1}",
                iteration_score.0,
                self.state.depth.get(),
            );
            sender.send_message(
                Debug,
                &format!(
                    "depth {depth}, score {0}, radius {1}, interval ({2}, {3})",
                    iteration_score.0,
                    window_radius.0,
                    alpha.0,
                    beta.0,
                    depth = self.state.depth.get()
                ),
            );
            if self.state.search_cancelled() {
                break;
            }
            self.state.score = iteration_score;
            if iteration_score > alpha && iteration_score < beta {
                sender.send_search_info(self.search_info()); // do this before incrementing the depth
                self.state.depth += Depth::new(1);
                // make sure that alpha and beta are at least 2 apart, to recognize PV nodes.
                window_radius = Score(1.max(window_radius.0 / 2));
            } else {
                window_radius.0 *= 3;
            }
            alpha = (iteration_score - window_radius).max(SCORE_LOST);
            beta = (iteration_score + window_radius).min(SCORE_WON);
            // incomplete iterations and root nodes that failed low don't overwrite the `state.best_move`,
            // so it should be fine to unconditionally assign it to `chosen_move`
            chosen_move = self.state.best_move;
            if self.should_not_start_next_iteration(soft_limit, max_depth, limit.mate) {
                break;
            }
        }
        self.state.depth -= Depth::new(1); // don't print a depth one larger than what was actually searched.
        chosen_move
    }

    fn negamax(
        &mut self,
        pos: Chessboard,
        limit: SearchLimit,
        ply: usize,
        mut depth: isize,
        mut alpha: Score,
        beta: Score,
        sender: &SearchSender<Chessboard>,
    ) -> Score {
        debug_assert!(alpha < beta);
        debug_assert!(ply <= DEPTH_HARD_LIMIT.get());
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
        if depth <= 0 || ply >= DEPTH_HARD_LIMIT.get() {
            return self.qsearch(pos, alpha, beta, ply);
        }
        let can_prune = !is_pvs_pv_node && !in_check;

        let mut best_score = NO_SCORE_YET;
        let mut bound_so_far = UpperBound;

        let tt_entry: TTEntry<Chessboard> = self.tt.load(pos.zobrist_hash(), ply);
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

        // IIR (Internal Iterative Reductions): If we don't have a TT move, this node will likely take a long time
        // because the move ordering won't be great, so don't spend too much time on this node.
        // Instead, search it with reduced depth to fill the TT entry so that we can re-search it faster the next time
        // we see this node.
        if depth > 4 && best_move == ChessMove::default() {
            depth -= 1;
        }

        // RFP (Reverse Futility Pruning): If eval is far above beta, it's likely that our opponent
        // blundered in a previous move of the search, so if the depth is low, don't even bother searching further.
        if can_prune {
            if depth < 4 && eval >= beta + Score(80 * depth as i32) {
                return eval;
            }

            // NMP (Null Move Pruning). If static eval of our position is above beta, this node probably isn't that interesting.
            // To test this hypothesis, do a null move and perform a search with reduced depth; if the result is still
            // above beta, then it's very likely that the score would have been above beta if we had played a move,
            // so simply return the nmp score. This is based on the null move observation (there are very few zugzwang positions).
            // A more careful implementation would do a verification search to check for zugzwang, and possibly avoid even trying
            // nmp in a position with no pieces except the king and pawns.
            // TODO: Verification search.
            if depth >= 3 && eval >= beta {
                self.state.board_history.push(&pos);
                let new_pos = pos.make_nullmove().unwrap();
                let reduction = 3 + depth / 4;
                let score = -self.negamax(
                    new_pos,
                    limit,
                    ply + 1,
                    depth - 1 - reduction,
                    -beta,
                    -beta + 1,
                    sender,
                );
                self.state.board_history.pop(&pos);
                if score >= beta {
                    return score.min(MIN_SCORE_WON);
                }
            }
        }

        let mut children_visited = 0;
        let mut num_quiets_visited = 0;

        let all_moves = self.order_moves(pos.pseudolegal_moves(), &pos, best_move, ply);
        for mov in all_moves {
            // LMP (Late Move Pruning): Trust the move ordering and assume that moves ordered late aren't very interesting,
            // so don't even bother looking at them in the last few layers.
            // FP (Futility Pruning): If the static eval is far below alpha,
            // then it's unlikely that a quiet move can raise alpha: We've probably blundered at some prior point in search,
            // so cut our losses and return. This has the potential of missing sacrificing mate combinations, though.
            if can_prune
                && best_score > MAX_SCORE_LOST
                && depth <= 3
                && (num_quiets_visited >= 16 * depth
                    || (eval + Score((300 + 64 * depth) as i32) < alpha && num_quiets_visited > 0))
            // quiets are ordered last, so this move is ordered very late
            {
                break;
            }

            let new_pos = pos.make_move(mov);
            if new_pos.is_none() {
                continue; // illegal pseudolegal move
            }
            let new_pos = new_pos.unwrap();
            self.state.nodes += 1;
            children_visited += 1;
            if !mov.is_tactical(&pos) {
                num_quiets_visited += 1;
            }

            // O(1). Resets the child's pv length so that it's not the maximum length it used to be.
            self.state.search_stack[ply + 1].pv.clear();

            let debug_history_len = self.state.board_history.0 .0.len();

            self.state.board_history.push(&pos);
            // PVS (Principal Variation Search): Assume that the TT move is the best move, so we only need to prove
            // that the other moves are worse, which we can do with a zero window search. Should this assumption fail,
            // re-search with a full window.
            // A better (but very slightly more complicated) implementation would be to do 2 researches, first with a
            // null window but the full depth, and only then without a null window and at the full depth.
            let mut score;
            if children_visited == 1 {
                score = -self.negamax(new_pos, limit, ply + 1, depth - 1, -beta, -alpha, sender);
            } else {
                // LMR (Late Move Reductions): Trust the move ordering (mostly the quiet history heuristic, at least currently)
                // and assume that moves ordered later are worse. Therefore, we can do a reduced-depth search with a null window
                // to verify our belief.
                let mut reduction = 0;
                if !in_check && num_quiets_visited > 4 && depth >= 4 {
                    reduction = 1 + depth / 8; // This is a very basic implementation. TODO: Make more complex eventually.
                    if !is_pvs_pv_node {
                        reduction += 1;
                    }
                }

                score = -self.negamax(
                    new_pos,
                    limit,
                    ply + 1,
                    depth - 1 - reduction,
                    -(alpha + 1),
                    -alpha,
                    sender,
                );
                // If the score turned out to be better than expected (at least `alpha`), this might just be because
                // of the reduced depth. So do a full-depth search first, but don't use the full window quite yet.
                if alpha < score {
                    score = -self.negamax(
                        new_pos,
                        limit,
                        ply + 1,
                        depth - 1,
                        -(alpha + 1),
                        -alpha,
                        sender,
                    );
                    // If the full-depth search also performed better than expected, do a full-depth search with the
                    // full window to find the true score. If the score was at least `beta`, don't search again
                    // -- this move is probably already too good, so don't waste more time finding out how good it is exactly.
                    if alpha < score && score < beta {
                        score = -self.negamax(
                            new_pos,
                            limit,
                            ply + 1,
                            depth - 1,
                            -beta,
                            -alpha,
                            sender,
                        );
                    }
                }
            }

            self.state.board_history.pop(&pos);

            debug_assert_eq!(
                self.state.board_history.0.0.len(),
                debug_history_len,
                "depth {depth} ply {ply} old len {debug_history_len} new len {} child {children_visited}", self.state.board_history.0.0.len()
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

            let split = self.state.search_stack.split_at_mut(ply + 1);
            let pv = &mut split.0.last_mut().unwrap().pv;
            let child_pv = &split.1[0].pv;
            pv.push(ply, best_move, child_pv);

            if score < beta {
                continue;
            }
            bound_so_far = LowerBound;
            if mov.is_tactical(&pos) {
                break;
            }
            // Update various heuristics, TODO: More (killers, history gravity, etc)
            let entry = &mut self.state.search_stack[ply];
            self.state.custom.history[mov.from_to_square()] += (depth * depth) as i32;
            entry.killer = mov;
            break;
        }

        if ply == 0 {
            assert_ne!(children_visited, 0);
            self.state.best_move = Some(best_move);
        } else if children_visited == 0 {
            // TODO: Merge cached in-check branch
            return game_result_to_score(pos.no_moves_result(), ply);
        }

        // in case of a collision, if there's no best_move to store because the node failed low, make that clear by
        // storing a null move instead of the random move that was stored there previously. This helps IIR.
        if bound_so_far == UpperBound && pos.zobrist_hash() != tt_entry.hash {
            best_move = ChessMove::default();
        }
        let tt_entry: TTEntry<Chessboard> = TTEntry::new(
            pos.zobrist_hash(),
            best_score,
            best_move,
            depth,
            bound_so_far,
        );
        // TODO: eventually test that not overwriting PV nodes unless the depth is quite a bit greater gains
        // Store the results in the TT, always replacing the previous entry. Note that the TT move is only overwritten
        // if this node was an exact or fail high node or if there was a collision.
        self.tt.store(tt_entry, ply);

        best_score
    }

    /// Search only "tactical" moves to quieten down the position before calling eval.
    fn qsearch(&mut self, pos: Chessboard, mut alpha: Score, beta: Score, ply: usize) -> Score {
        // The stand pat check. Since we're not looking at all moves, it's very likely that there's a move we didn't
        // look at that doesn't make our position worse, so we don't want to assume that we have to play a capture.
        let mut best_score = self.eval.eval(pos);
        let mut bound_so_far = UpperBound;
        if best_score >= beta {
            return best_score;
        }

        self.state.sel_depth = self.state.sel_depth.max(ply);

        // TODO: stand pat is SCORE_LOST when in check, generate evasions?
        alpha = alpha.max(best_score);
        // TODO: Using the TT for move ordering in qsearch was mostly elo-neutral, so retest that eventually
        // do TT cutoffs with alpha already raised by the stand pat check, because that relies on the null move observation
        // but if there's a TT entry from normal search that's worse than the stand pat score, we should trust that more.
        let tt_entry: TTEntry<Chessboard> = self.tt.load(pos.zobrist_hash(), ply);

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

        let captures =
            self.order_moves(pos.tactical_pseudolegal(), &pos, ChessMove::default(), ply);
        for mov in captures {
            debug_assert!(mov.is_tactical(&pos));
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
        // see main search, don't store a random move in the TT entry.
        if bound_so_far == UpperBound && pos.zobrist_hash() != tt_entry.hash {
            best_move = ChessMove::default();
        }
        let tt_entry: TTEntry<Chessboard> =
            TTEntry::new(pos.zobrist_hash(), best_score, best_move, 0, bound_so_far);
        self.tt.store(tt_entry, ply);
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
            } else if *mov == self.state.search_stack[ply].killer {
                i32::MAX - 200
            } else if captured == Empty {
                self.state.custom.history[mov.from_to_square()]
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
    use gears::games::{Move, ZobristHistoryBase};
    use gears::search::Nodes;

    use crate::eval::chess::hce::HandCraftedEval;
    use crate::eval::chess::pst_only::PstOnlyEval;
    use crate::eval::rand_eval::RandEval;

    use super::*;

    #[test]
    fn mate_in_one_test() {
        let board = Chessboard::from_fen("4k3/8/4K3/8/8/8/8/6R1 w - - 0 1").unwrap();
        // run multiple times to get different random numbers from the eval function
        for depth in 1..=3 {
            for _ in 0..100 {
                let mut engine = Caps::<RandEval>::default();
                let res = engine
                    .search(
                        board,
                        SearchLimit::depth(Depth::new(depth)),
                        ZobristHistoryBase::default(),
                        &mut SearchSender::no_sender(),
                    )
                    .unwrap();
                assert!(res.score.unwrap().is_game_won_score());
                assert_eq!(res.score.unwrap().plies_until_game_won(), Some(1));
            }
        }
    }

    #[test]
    fn simple_search_test() {
        let list = [
            (
                "r2q1r2/ppp1pkb1/2n1p1pp/2N1P3/2pP2Q1/2P1B2P/PP3PP1/R4RK1 b - - 1 18",
                -500,
                -100,
            ),
            (
                "r1bqkbnr/3n2p1/2p1pp1p/pp1p3P/P2P4/1PP1PNP1/1B3P2/RN1QKB1R w KQkq - 0 14",
                90,
                300,
            ),
        ];
        for (fen, min, max) in list {
            let pos = Chessboard::from_fen(fen).unwrap();
            let mut engine = Caps::<PstOnlyEval>::default();
            let res = engine
                .search_from_pos(pos, SearchLimit::nodes(Nodes::new(50_000).unwrap()))
                .unwrap();
            assert!(res.score.is_some_and(|score| score > Score(min)));
            assert!(res.score.is_some_and(|score| score < Score(max)));
        }
    }

    #[test]
    fn lucena_test() {
        let pos = Chessboard::from_name("lucena").unwrap();
        let mut engine = Caps::<PstOnlyEval>::default();
        let res = engine
            .search_from_pos(pos, SearchLimit::depth(Depth::new(7)))
            .unwrap();
        // TODO: More aggressive bound once the engine is stronger
        assert!(res.score.unwrap() >= Score(200));
    }

    #[test]
    fn philidor_test() {
        let pos = Chessboard::from_name("philidor").unwrap();
        let mut engine = Caps::<HandCraftedEval>::default();
        let res = engine.search_from_pos(pos, SearchLimit::nodes(Nodes::new(100_000).unwrap()));
        // TODO: More aggressive bound once the engine is stronger
        assert!(res.unwrap().score.unwrap().abs() <= Score(200));
    }

    #[test]
    #[cfg(not(debug_assertions))]
    /// puzzles that are reasonably challenging for most humans, but shouldn't be difficult for the engine
    fn mate_test() {
        let fens = [
            ("8/5K2/4N2k/2B5/5pP1/1np2n2/1p6/r2R4 w - - 0 1", "d1d5"),
            ("5rk1/r5p1/2b2p2/3q1N2/6Q1/3B2P1/5P2/6KR w - - 0 1", "f5h6"),
            (
                "2rk2nr/R1pnp3/5b2/5P2/BpPN1Q2/pPq5/P7/1K4R1 w - - 0 1",
                "f4c7",
            ),
            ("k2r3r/PR6/1K6/3R4/8/5np1/B6p/8 w - - 0 1", "d5d8"),
            ("3n3R/8/3p1pp1/r2bk3/8/4NPP1/p3P1KP/1r1R4 w - - 0 1", "h8e8"),
            ("7K/k7/p1R5/4N1q1/8/6rb/5r2/1R6 w - - 0 1", "c6c7"),
            (
                "rkr5/3n1p2/1pp1b3/NP4p1/3PPn1p/QN1B1Pq1/2P5/R6K w - - 0 1",
                "a5c6",
            ),
            ("1kr5/4R3/pP6/1n2N3/3p4/2p5/1r6/4K2R w K - 0 1", "h1h8"),
            ("1k6/1bpQN3/1p6/p7/6p1/2NP1nP1/5PK1/4q3 w - - 0 1", "d7d8"),
            (
                "1k4r1/pb1p4/1p1P4/1P3r1p/1N2Q3/6Pq/4BP1P/4R1K1 w - - 0 1",
                "b4a6",
            ),
        ];
        for (fen, mov) in fens {
            let pos = Chessboard::from_fen(fen).unwrap();
            let mut engine = Caps::<HandCraftedEval>::default();
            let mut limit = SearchLimit::depth(Depth::new(18));
            limit.mate = Depth::new(10);
            limit.fixed_time = Duration::from_secs(2);
            let res = engine
                .search_from_pos(pos, SearchLimit::depth(Depth::new(15)))
                .unwrap();
            println!(
                "chosen move {0}, fen {1}",
                res.chosen_move.to_extended_text(&pos),
                pos.as_fen()
            );
            assert!(res.score.unwrap().is_game_won_score());
            assert_eq!(res.chosen_move.to_compact_text(), mov);
        }
    }
}
