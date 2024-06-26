use arrayvec::ArrayVec;
use std::cmp::min;
use std::mem::take;
use std::time::{Duration, Instant};

use derive_more::{Deref, DerefMut, Index, IndexMut};
use itertools::Itertools;
use rand::thread_rng;

use crate::eval::chess::lite::LiTEval;
use gears::games::chess::moves::ChessMove;
use gears::games::chess::see::SeeScore;
use gears::games::chess::{Chessboard, MAX_CHESS_MOVES_IN_POS};
use gears::games::{n_fold_repetition, Board, BoardHistory, Color, Move, ZobristHistory};
use gears::general::common::Description::NoDescription;
use gears::general::common::{select_name_static, Res, StaticallyNamedEntity};
use gears::output::Message::Debug;
use gears::score::{
    game_result_to_score, ScoreT, MAX_SCORE_LOST, MIN_SCORE_WON, NO_SCORE_YET, SCORE_LOST,
    SCORE_TIME_UP,
};
use gears::search::*;
use gears::ugi::EngineOptionName::*;
use gears::ugi::EngineOptionType::Spin;
use gears::ugi::{EngineOption, EngineOptionName, EngineOptionType, UgiCheck, UgiSpin};

use crate::eval::Eval;
use crate::search::move_picker::MovePicker;
use crate::search::statistics::SearchType;
use crate::search::statistics::SearchType::{MainSearch, Qsearch};
use crate::search::tt::{TTEntry, TT};
use crate::search::*;

/// The maximum value of the `depth` parameter, i.e. the maximum number of Iterative Deepening iterations.
const DEPTH_SOFT_LIMIT: Depth = Depth::new(100);
/// The maximum value of the `ply` parameter, i.e. the maximum depth (in plies) before qsearch is reached
const DEPTH_HARD_LIMIT: Depth = Depth::new(128);

const HIST_DIVISOR: i32 = 1024;
/// The TT move and good captures have a higher score, all other moves have a lower score.
const KILLER_SCORE: MoveScore = MoveScore(i32::MAX - 100 * HIST_DIVISOR);

/// Updates the history using the History Gravity technique,
/// which keeps history scores from growing arbitrarily large and scales the bonus/malus depending on how
/// "unexpected" they are, i.e. by how much they differ from the current history scores.
fn update_history_score(entry: &mut i32, bonus: i32) {
    // The maximum history score magnitude can be slightly larger than the divisor due to rounding errors.
    // The `.abs()` call is necessary to correctly handle history malus.
    let bonus = bonus - bonus.abs() * *entry / HIST_DIVISOR; // bonus can also be negative
    *entry += bonus;
}

/// Quiet History Heuristic: Give bonuses to quiet moves that causes a beta cutoff a maluses to quiet moves that were tried
/// but didn't cause a beta cutoff. Order all non-TT non-killer moves based on that (as well as based on the continuation
/// history)
#[derive(Debug, Clone, Deref, DerefMut, Index, IndexMut)]
struct HistoryHeuristic([i32; 64 * 64]);

impl HistoryHeuristic {
    fn update(&mut self, mov: ChessMove, bonus: i32) {
        update_history_score(&mut self[mov.from_to_square()], bonus);
    }
}

impl Default for HistoryHeuristic {
    fn default() -> Self {
        HistoryHeuristic([0; 64 * 64])
    }
}

/// Capture History Heuristic: Same as quiet history heuristic, but for captures.
#[derive(Debug, Clone, Index, IndexMut)]
struct CaptHist([[[i32; 64]; 6]; 2]);

impl CaptHist {
    fn update(&mut self, mov: ChessMove, color: Color, bonus: i32) {
        let entry =
            &mut self[color as usize][mov.uncolored_piece() as usize][mov.dest_square().bb_idx()];
        update_history_score(entry, bonus)
    }
    fn get(&self, mov: ChessMove, color: Color) -> MoveScore {
        MoveScore(self[color as usize][mov.uncolored_piece() as usize][mov.dest_square().bb_idx()])
    }
}

impl Default for CaptHist {
    fn default() -> Self {
        Self([[[0; 64]; 6]; 2])
    }
}

/// Continuation history. Many moves have a "natural" response, so use that for move ordering:
/// Instead of only learning which quiet moves are good, learn which quiet moves are good after our
/// opponent played a given move.
#[derive(Debug, Clone, Deref, DerefMut, Index, IndexMut)]
struct ContHist(Vec<i32>); // Can't store this on the stack because it's too large.

impl ContHist {
    fn idx(mov: ChessMove, prev_move: ChessMove, color: Color) -> usize {
        (mov.uncolored_piece() as usize + mov.dest_square().bb_idx() * 6)
            + (prev_move.uncolored_piece() as usize + prev_move.dest_square().bb_idx() * 6) * 64 * 6
            + color as usize * 64 * 6 * 64 * 6
    }
    fn update(&mut self, mov: ChessMove, prev_mov: ChessMove, bonus: i32, color: Color) {
        let entry = &mut self[Self::idx(mov, prev_mov, color)];
        update_history_score(entry, bonus);
    }
    fn score(&self, mov: ChessMove, prev_move: ChessMove, color: Color) -> i32 {
        self[Self::idx(mov, prev_move, color)]
    }
}

impl Default for ContHist {
    fn default() -> Self {
        ContHist(vec![0; 2 * 6 * 64 * 6 * 64])
    }
}

#[derive(Debug, Clone, Default)]
struct Additional {
    history: HistoryHeuristic,
    cont_hist: ContHist,
    capt_hist: CaptHist,
    tt: TT,
    original_board_hist: ZobristHistory<Chessboard>,
}

impl CustomInfo for Additional {
    fn tt(&self) -> Option<&TT> {
        Some(&self.tt)
    }

    fn new_search(&mut self) {
        // don't update history values, malus and gravity already take care of that
    }

    fn forget(&mut self) {
        for value in self.history.iter_mut() {
            *value = 0;
        }
        for value in self.capt_hist.0.iter_mut().flatten().flatten() {
            *value = 0;
        }
        for value in self.cont_hist.iter_mut() {
            *value = 0;
        }
    }
}

#[derive(Debug, Default, Clone)]
struct CapsSearchStackEntry {
    killer: ChessMove,
    pv: Pv<Chessboard, { DEPTH_HARD_LIMIT.get() }>,
    tried_moves: ArrayVec<ChessMove, MAX_CHESS_MOVES_IN_POS>,
    pos: Chessboard,
    eval: Score,
}

impl SearchStackEntry<Chessboard> for CapsSearchStackEntry {
    fn pv(&self) -> Option<&[ChessMove]> {
        Some(&self.pv.list[0..self.pv.length])
    }
}

impl CapsSearchStackEntry {
    /// If this entry has a lower ply number than the current node, this is the tree edge that leads towards the current node.
    fn last_tried_move(&self) -> ChessMove {
        *self.tried_moves.last().unwrap()
    }
}

type CapsState = ABSearchState<Chessboard, CapsSearchStackEntry, Additional>;

type DefaultEval = LiTEval;

/// Chess-playing Alpha-beta Pruning Search, or in short, CAPS.
/// Larger than SᴍᴀʟʟCᴀᴘꜱ.
#[derive(Debug)]
pub struct Caps {
    state: CapsState,
    eval: Box<dyn Eval<Chessboard>>,
}

impl Default for Caps {
    fn default() -> Self {
        // TODO: Make sure this doesn't inadvertently make other threads use a different eval
        Self::with_eval(Box::new(DefaultEval::default()))
    }
}

impl StaticallyNamedEntity for Caps {
    fn static_short_name() -> impl Display
    where
        Self: Sized,
    {
        "CAPS"
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        format!("CAPS: Chess-playing Alpha-beta Pruning Search",)
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "Chess-playing Alpha-beta Pruning Search (CAPS), a chess engine. \
        Currently early in development, but still around 3k elo with a hand crafted eval. \
        Much larger than SᴍᴀʟʟCᴀᴘꜱ"
            .to_string()
    }
}

impl Benchable<Chessboard> for Caps {
    fn bench(&mut self, pos: Chessboard, depth: Depth) -> BenchResult {
        self.state.forget(true);
        let mut limit = SearchLimit::infinite();
        limit.depth = DEPTH_SOFT_LIMIT.min(depth);
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
                    max: Some(10_000_000), // use at most 10 terabytes (should be enough for anybody™)
                }),
            },
            EngineOption {
                name: Threads,
                value: Spin(UgiSpin {
                    val: 1,
                    default: Some(1),
                    min: Some(1),
                    max: Some(1000),
                }),
            },
            EngineOption {
                name: EngineOptionName::Other("UCI_Chess960".to_string()),
                value: EngineOptionType::Check(UgiCheck {
                    val: true,
                    default: Some(true),
                }),
            },
        ];
        EngineInfo::new(self, self.eval.as_ref(), "0.1.0", Depth::new(12), options)
    }

    fn set_option(&mut self, option: EngineOptionName, _value: String) -> Res<()> {
        let name = option.name().to_string();
        if let EngineOptionName::Other(name) = option {
            if name == "UCI_Chess960" {
                return Ok(());
            }
        }
        select_name_static(
            &name,
            self.engine_info().options.iter(),
            "uci option",
            "chess",
            NoDescription,
        )?; // only called to produce an error message
        Err("Unrecognized option name. Spelling error?".to_string())
    }
}

impl Engine<Chessboard> for Caps {
    fn set_tt(&mut self, tt: TT) {
        self.state.custom.tt = tt;
    }

    fn set_eval(&mut self, eval: Box<dyn Eval<Chessboard>>) {
        self.eval = eval;
    }

    fn do_search(
        &mut self,
        pos: Chessboard,
        mut limit: SearchLimit,
    ) -> Res<SearchResult<Chessboard>> {
        limit.fixed_time = min(limit.fixed_time, limit.tc.remaining);
        let soft_limit = limit
            .fixed_time
            .min((limit.tc.remaining.saturating_sub(limit.tc.increment)) / 32 + limit.tc.increment)
            .min(limit.tc.remaining / 4);

        // TODO: Use lambda for lazy evaluation in case debug is off
        self.state.sender.send_message(Debug, &format!(
            "Starting search with limit {time}ms, {incr}ms increment, max {fixed}ms, mate in {mate} plies, max depth {depth}, max {nodes} nodes, soft limit {soft}ms",
            time = limit.tc.remaining.as_millis(),
            incr = limit.tc.increment.as_millis(),
            mate = limit.mate.get(),
            depth = limit.depth.get(),
            nodes = limit.nodes.get(),
            fixed = limit.fixed_time.as_millis(),
            soft = soft_limit.as_millis(),
        ));
        // Use 3fold repetition detection for positions before and including the root node and 2fold for positions during search.
        self.state.custom.original_board_hist = take(&mut self.state.board_history);
        self.state.custom.original_board_hist.push(&pos);

        let chosen_move = match self.aspiration(pos, limit, soft_limit) {
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
        debug_assert!(self.state.main_search_nodes() % DEFAULT_CHECK_TIME_INTERVAL == 0);
        let elapsed = start_time.elapsed();
        // divide by 4 unless moves to go is very small, but don't divide by 1 (or zero) to avoid timeouts
        let divisor = tc.moves_to_go.unwrap_or(usize::MAX).clamp(2, 4) as u32;
        // Because fixed_time has been clamped to at most tc.remaining, this can never lead to timeouts
        // (assuming the move overhead is set correctly)
        elapsed >= fixed_time.min(tc.remaining / divisor + tc.increment)
    }

    #[inline(always)]
    fn search_state(&self) -> &impl SearchState<Chessboard> {
        &self.state
    }

    #[inline(always)]
    fn search_state_mut(&mut self) -> &mut impl SearchState<Chessboard> {
        &mut self.state
    }

    fn can_use_multiple_threads() -> bool
    where
        Self: Sized,
    {
        true
    }
    fn with_eval(eval: Box<dyn Eval<Chessboard>>) -> Self {
        Self {
            state: ABSearchState::new(DEPTH_HARD_LIMIT),
            eval,
        }
    }

    fn static_eval(&mut self, pos: Chessboard) -> Score {
        self.eval.eval(&pos)
    }
}

#[allow(clippy::too_many_arguments)]
impl Caps {
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
    ) -> Option<ChessMove> {
        let mut chosen_move = self.state.best_move;
        let max_depth = DEPTH_SOFT_LIMIT.min(limit.depth).get() as isize;

        let mut alpha = SCORE_LOST;
        let mut beta = SCORE_WON;

        let mut window_radius = Score(20);

        self.state.statistics.next_id_iteration();

        loop {
            let iteration_score = self.negamax(
                pos,
                limit,
                0,
                self.state.depth().get() as isize,
                alpha,
                beta,
                Exact,
            );
            assert!(
                !(iteration_score != SCORE_TIME_UP
                    && iteration_score
                        .plies_until_game_over()
                        .is_some_and(|x| x <= 0)),
                "score {0} depth {1}",
                iteration_score.0,
                self.state.depth().get(),
            );
            self.state.sender.send_message(
                Debug,
                &format!(
                    "depth {depth}, score {0}, radius {1}, interval ({2}, {3})",
                    iteration_score.0,
                    window_radius.0,
                    alpha.0,
                    beta.0,
                    depth = self.state.depth().get()
                ),
            );
            if self.state.search_cancelled() {
                break;
            }
            self.state.score = iteration_score;
            if iteration_score > alpha && iteration_score < beta {
                self.state.sender.send_search_info(self.search_info()); // do this before incrementing the depth
                                                                        // make sure that alpha and beta are at least 2 apart, to recognize PV nodes.
                window_radius = Score(1.max(window_radius.0 / 2));
                self.state.statistics.aw_exact(); // increases the depth
            } else {
                window_radius.0 *= 3;
                if iteration_score <= alpha {
                    self.state.statistics.aw_fail_low();
                } else {
                    self.state.statistics.aw_fail_high()
                }
            }
            alpha = (iteration_score - window_radius).max(SCORE_LOST);
            beta = (iteration_score + window_radius).min(SCORE_WON);
            // incomplete iterations and root nodes that failed low don't overwrite the `state.best_move`,
            // so it should be fine to unconditionally assign it to `chosen_move`
            chosen_move = self.state.best_move;
            if self.should_not_start_next_iteration(soft_limit, max_depth, limit.mate) {
                self.state.statistics.soft_limit_stop();
                break;
            }
        }
        chosen_move
    }

    /// Recursive search function, the most important part of the engine. If the computed score of the current position
    /// lies within the open interval `(alpha, beta)`, return the score. Otherwise, the returned score might not be exact,
    /// but could be closer to the window than the true score. On top of that, there are **many** additional techniques
    /// that can mess with the returned score, so that it's best not to assume too much: For example, it's not unlikely
    /// that a re-search with the same depth returns a different score. Because of PVS, `alpha` is `beta - 1` in almost
    /// all nodes, and most nodes either get cut off before reaching the move loop or produce a beta cutoff after
    /// the first move.
    fn negamax(
        &mut self,
        pos: Chessboard,
        limit: SearchLimit,
        ply: usize,
        mut depth: isize,
        mut alpha: Score,
        beta: Score,
        mut expected_node_type: NodeType,
    ) -> Score {
        debug_assert!(alpha < beta);
        debug_assert!(ply <= DEPTH_HARD_LIMIT.get());
        debug_assert!(depth <= DEPTH_SOFT_LIMIT.get() as isize);
        debug_assert!(self.state.board_history.0.len() >= ply);
        self.state
            .statistics
            .count_node_started(MainSearch, ply, false);

        let mut tt = self.state.custom.tt.clone();

        let root = ply == 0;
        let is_pv_node = expected_node_type == Exact; // TODO: Make this a generic argument of search?
        debug_assert!(!root || is_pv_node); // root implies pv node
        debug_assert!(alpha + 1 == beta || is_pv_node); // alpha + 1 < beta implies Exact node

        let ply_100_ctr = pos.halfmove_repetition_clock();
        if !root
            && (n_fold_repetition(2, &self.state.board_history, &pos, ply_100_ctr)
                || n_fold_repetition(
                    3,
                    &self.state.custom.original_board_hist,
                    &pos,
                    ply_100_ctr.saturating_sub(ply),
                )
                || pos.is_50mr_draw()
                || pos.has_insufficient_material())
        {
            return Score(0);
        }
        let in_check = pos.is_in_check();
        // Check extensions. Increase the depth by 1 if in check.
        // Do this before deciding whether to drop into qsearch.
        if in_check {
            self.state.statistics.in_check();
            depth += 1;
        }
        if depth <= 0 || ply >= DEPTH_HARD_LIMIT.get() {
            return self.qsearch(pos, alpha, beta, ply);
        }
        let can_prune = !is_pv_node && !in_check;

        let mut best_score = NO_SCORE_YET;
        let mut bound_so_far = FailLow;

        // In case of a collision, if there's no best_move to store because the node failed low,
        // store a null move in the TT. This helps IIR.
        let mut best_move = ChessMove::default();
        let mut eval = self.eval(pos, ply);
        if let Some(tt_entry) = tt.load::<Chessboard>(pos.zobrist_hash(), ply) {
            let bound = tt_entry.bound();
            debug_assert_eq!(tt_entry.hash, pos.zobrist_hash());

            // TT cutoffs. If we've already seen this position, and the TT entry has more valuable information (higher depth),
            // and we're not a PV node, and the saved score is either exact or at least known to be outside (alpha, beta),
            // simply return it.
            if !is_pv_node
                && tt_entry.depth as isize >= depth
                && ((tt_entry.score >= beta && bound == FailHigh)
                    || (tt_entry.score <= alpha && bound == FailLow)
                    || bound == Exact)
            {
                self.state.statistics.tt_cutoff(MainSearch, bound);
                return tt_entry.score;
            }
            // Even though we didn't get a cutoff from the TT, we can still use the score and bound to update our guess
            // at what the type of this node is going to be.
            if !is_pv_node {
                if bound == Exact {
                    expected_node_type = if tt_entry.score <= alpha {
                        FailLow
                    } else {
                        FailHigh
                    }
                } else {
                    // TODO: Base instead on relation between tt score and window?
                    // Or only update if the difference between tt score and the window is large?
                    expected_node_type = bound;
                }
            }

            best_move = tt_entry.mov;
            // The TT score is backed by a search, so it should be more trustworthy than a simple call to static eval.s
            if !tt_entry.score.is_game_over_score()
                && (bound == Exact
                    || (bound == FailHigh && tt_entry.score >= eval)
                    || (bound == FailLow && tt_entry.score <= eval))
            {
                eval = tt_entry.score;
            }
        } else {
            self.state.statistics.tt_miss(MainSearch);
        };

        self.record_pos(pos, eval, ply);

        // like the commonly used `improving` and `regressing`, these variables compare the current static eval with
        // the static eval 2 plies ago to recognize blunders. Conceptually, `improving` and `regressing` can be seen as
        // a prediction for how the eval is going to evolve, while these variables are more about cutting early after bad moves.
        // TODO: Currently, this uses the TT score when possible. Think about if there are unintended consequences.
        let they_blundered = ply >= 2 && eval - self.state.search_stack[ply - 2].eval > Score(50);
        let we_blundered = ply >= 2 && eval - self.state.search_stack[ply - 2].eval < Score(-50);
        debug_assert!(!eval.is_game_over_score());
        // IIR (Internal Iterative Reductions): If we don't have a TT move, this node will likely take a long time
        // because the move ordering won't be great, so don't spend too much time on this node.
        // Instead, search it with reduced depth to fill the TT entry so that we can re-search it faster the next time
        // we see this node. If there was no TT entry because the node failed low, this node probably isn't that interesting,
        // so reducing the depth also makes sense in this case.
        if depth > 4 && best_move == ChessMove::default() {
            depth -= 1;
        }

        if can_prune {
            // RFP (Reverse Futility Pruning): If eval is far above beta, it's likely that our opponent
            // blundered in a previous move of the search, so if the depth is low, don't even bother searching further.
            // Use `they_blundered` to better distinguish between blunders by our opponent and a generally good static eval
            // relative to `beta` --  there may be other positional factors that aren't being reflected by the static eval,
            // (like imminent threads) so don't prune too aggressively if our opponent hasn't blundered.
            // Be more careful about pruning too aggressively if the node is expected to fail low -- we should not rfp
            // a true fail low node, but our expectation may also be wrong.
            let mut margin = (150 - (they_blundered as ScoreT * 64)) * depth as ScoreT;
            if expected_node_type == FailHigh {
                margin /= 2;
            }
            if depth < 4 && eval >= beta + Score(margin) {
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
                // `make_nullmove` resets the 50mr counter, so we don't consider positions after a nullmove as repetitions,
                // but we can still get TT cutoffs
                self.state.board_history.push(&pos);
                let new_pos = pos.make_nullmove().unwrap();
                // necessary to recognize the null move and to make `last_tried_move()` not panic
                self.state.search_stack[ply]
                    .tried_moves
                    .push(ChessMove::default());
                let reduction = 3 + depth / 4 + they_blundered as isize;
                let score = -self.negamax(
                    new_pos,
                    limit,
                    ply + 1,
                    depth - 1 - reduction,
                    -beta,
                    -beta + 1,
                    FailLow, // the child node is expected to fail low, leading to a fail high in this node
                );
                self.state.search_stack[ply].tried_moves.pop();
                self.state.board_history.pop();
                if score >= beta {
                    return score.min(MIN_SCORE_WON);
                }
            }
        }

        // An uninteresting move is a quiet move or bad capture unless it's the TT or killer move
        // (i.e. it's every move that gets ordered after the killer). The name is a bit dramatic, the first few of those
        // can still be good candidates to explore.
        let mut num_uninteresting_visited = 0;
        debug_assert!(self.state.search_stack[ply].tried_moves.is_empty());

        let mut move_picker =
            MovePicker::<Chessboard, MAX_CHESS_MOVES_IN_POS>::new(pos, best_move, false);
        let move_scorer = CapsMoveScorer { board: pos, ply };
        while let Some((mov, move_score)) = move_picker.next(&move_scorer, &self.state) {
            // LMP (Late Move Pruning): Trust the move ordering and assume that moves ordered late aren't very interesting,
            // so don't even bother looking at them in the last few layers.
            // FP (Futility Pruning): If the static eval is far below alpha,
            // then it's unlikely that a quiet move can raise alpha: We've probably blundered at some prior point in search,
            // so cut our losses and return. This has the potential of missing sacrificing mate combinations, though.
            let fp_margin = if we_blundered {
                200 + 32 * depth
            } else {
                300 + 64 * depth
            };
            let mut lmp_threshold = if we_blundered {
                6 + 4 * depth
            } else {
                8 + 8 * depth
            };
            // LMP faster if we expect to fail low anyway
            if expected_node_type == FailLow {
                lmp_threshold -= lmp_threshold / 4;
            }
            if can_prune
                && best_score > MAX_SCORE_LOST
                && depth <= 3
                && (num_uninteresting_visited >= lmp_threshold
                    || (eval + Score(fp_margin as ScoreT) < alpha && move_score < KILLER_SCORE))
            {
                break;
            }

            let Some(new_pos) = pos.make_move(mov) else {
                continue; // illegal pseudolegal move
            };
            if move_score < KILLER_SCORE {
                num_uninteresting_visited += 1;
            }

            // O(1). Resets the child's pv length so that it's not the maximum length it used to be.
            // TODO: Do this in `record_move`?
            self.state.search_stack[ply + 1].pv.clear();

            let debug_history_len = self.state.board_history.len();

            self.record_move(mov, pos, ply, MainSearch);
            // PVS (Principal Variation Search): Assume that the TT move is the best move, so we only need to prove
            // that the other moves are worse, which we can do with a zero window search. Should this assumption fail,
            // re-search with a full window.
            let mut score;
            if self.state.search_stack[ply].tried_moves.len() == 1 {
                score = -self.negamax(
                    new_pos,
                    limit,
                    ply + 1,
                    depth - 1,
                    -beta,
                    -alpha,
                    expected_node_type.inverse(),
                );
            } else {
                // LMR (Late Move Reductions): Trust the move ordering (quiet history, continuation history and capture history heuristics)
                // and assume that moves ordered later are worse. Therefore, we can do a reduced-depth search with a null window
                // to verify our belief.
                // I think it's common to have a minimum depth for doing LMR, but not having that gained elo.
                let mut reduction = 0;
                if !in_check && num_uninteresting_visited > 2 {
                    reduction = 2 + depth / 8 + (num_uninteresting_visited - 2) / 8;
                    // Reduce bad captures and quiet moves with bad combined history scores more.
                    if move_score < -MoveScore(HIST_DIVISOR / 4) {
                        reduction += 1;
                    } else if move_score > MoveScore(HIST_DIVISOR / 4) {
                        // Since the TT and killer move and good captures are not lmr'ed,
                        // this only applies to quiet moves with a good combined history score.
                        reduction -= 1;
                    }
                    if !is_pv_node {
                        reduction += 1;
                    }
                }
                // this ensures that check extensions prevent going into qsearch while in check
                reduction = reduction.min(depth - 1);

                score = -self.negamax(
                    new_pos,
                    limit,
                    ply + 1,
                    depth - 1 - reduction,
                    -(alpha + 1),
                    -alpha,
                    FailHigh,
                );
                // If the score turned out to be better than expected (at least `alpha`), this might just be because
                // of the reduced depth. So do a full-depth search first, but don't use the full window quite yet.
                if alpha < score && reduction > 0 {
                    self.state.statistics.lmr_first_retry();
                    score = -self.negamax(
                        new_pos,
                        limit,
                        ply + 1,
                        depth - 1,
                        -(alpha + 1),
                        -alpha,
                        FailHigh, // we still expect a fail high here
                    );
                }
                // If the full-depth search also performed better than expected, do a full-depth search with the
                // full window to find the true score. If the score was at least `beta`, don't search again
                // -- this move is probably already too good, so don't waste more time finding out how good it is exactly.
                if alpha < score && score < beta {
                    debug_assert_eq!(expected_node_type, Exact);
                    self.state.statistics.lmr_second_retry();
                    score = -self.negamax(new_pos, limit, ply + 1, depth - 1, -beta, -alpha, Exact);
                }
            }

            self.undo_move();

            debug_assert_eq!(
                self.state.board_history.len(),
                debug_history_len,
                "depth {depth} ply {ply} old len {debug_history_len} new len {0} child {1}",
                self.state.board_history.len(),
                self.state.search_stack[ply].tried_moves.len()
            );
            // Check for cancellation right after searching a move to avoid storing incorrect information in the TT.
            if self.should_stop(limit) {
                return SCORE_TIME_UP;
            }
            debug_assert!(score.0.abs() <= SCORE_WON.0, "score {} ply {ply}", score.0);

            best_score = best_score.max(score);
            // Save indentation by using `continue` instead of nested if statements.
            if score <= alpha {
                continue;
            }
            // We've raised alpha. For most nodes, this results in an immediate beta cutoff because we're using a null window.
            alpha = score;
            // Only set best_move on raising `alpha` instead of `best_score` because fail low nodes should store the
            // default move, which is either the TT move (if there was a TT hit) or the null move.
            best_move = mov;

            // Update the PV. We only need to do that for PV nodes (could even only do that for non-fail highs, although that would
            // truncate the pv on an aw fail high and it relies on details of this implementation), but for some reason,
            // that resulted in a bench slowdown, so for now we're doing that everywhere. TODO: Retest this eventually.
            let split = self.state.search_stack.split_at_mut(ply + 1);
            let pv = &mut split.0.last_mut().unwrap().pv;
            let child_pv = &split.1[0].pv;
            pv.push(ply, best_move, child_pv);

            if score < beta {
                // We're in a PVS PV node and this move raised alpha but didn't cause a fail high, so look at the other moves.
                // PVS PV nodes are rare
                bound_so_far = Exact;
                continue;
            }
            // Beta cutoff. Update history and killer for quiet moves, then break out of the moves loop.
            bound_so_far = FailHigh;
            self.update_histories_and_killer(&pos, mov, depth, ply, pos.active_player());
            break;
        }

        // Update statistics for this node as soon as we know the node type, before returning.
        self.state.statistics.count_complete_node(
            MainSearch,
            bound_so_far,
            depth,
            ply,
            self.state.search_stack[ply].tried_moves.len(),
        );

        if ply == 0 {
            debug_assert!(!self.state.search_stack[ply].tried_moves.is_empty());
            self.state.best_move = Some(best_move);
        } else if self.state.search_stack[ply].tried_moves.is_empty() {
            // TODO: Merge cached in-check branch
            return game_result_to_score(pos.no_moves_result(), ply);
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
        tt.store(tt_entry, ply);

        best_score
    }

    fn update_histories_and_killer(
        &mut self,
        pos: &Chessboard,
        mov: ChessMove,
        depth: isize,
        ply: usize,
        color: Color,
    ) {
        let (before, now) = self.state.search_stack.split_at_mut(ply);
        let entry = &mut now[0];
        let bonus = (depth * depth) as i32;
        if mov.is_tactical(pos) {
            for disappointing in entry
                .tried_moves
                .iter()
                .dropping_back(1)
                .filter(|m| m.is_tactical(pos))
            {
                self.state
                    .custom
                    .capt_hist
                    .update(*disappointing, color, -bonus);
            }
            self.state.custom.capt_hist.update(mov, color, -bonus);
            return;
        }
        entry.killer = mov;
        for disappointing in entry
            .tried_moves
            .iter()
            .dropping_back(1)
            .filter(|m| !m.is_tactical(pos))
        {
            self.state.custom.history.update(*disappointing, -bonus);
        }
        self.state.custom.history.update(mov, bonus);
        if ply > 0 {
            let predecessor = before.last_mut().unwrap();
            let prev_move = predecessor.last_tried_move();
            if prev_move == ChessMove::default() {
                return; // Ignore NMP null moves
            }
            self.state
                .custom
                .cont_hist
                .update(mov, prev_move, bonus, color);
            for disappointing in entry
                .tried_moves
                .iter()
                .dropping_back(1)
                .filter(|m| !m.is_tactical(pos))
            {
                self.state
                    .custom
                    .cont_hist
                    .update(*disappointing, prev_move, -bonus, color);
            }
        }
    }

    /// Search only "tactical" moves to quieten down the position before calling eval
    fn qsearch(&mut self, pos: Chessboard, mut alpha: Score, beta: Score, ply: usize) -> Score {
        self.state.statistics.count_node_started(Qsearch, ply, true);
        // The stand pat check. Since we're not looking at all moves, it's very likely that there's a move we didn't
        // look at that doesn't make our position worse, so we don't want to assume that we have to play a capture.
        let mut best_score = self.eval(pos, ply);
        let mut bound_so_far = FailLow;
        if best_score >= beta {
            return best_score;
        }

        // TODO: stand pat is SCORE_LOST when in check, generate evasions?
        alpha = alpha.max(best_score);

        // see main search, store an invalid random move in the TT entry if all moves failed low.
        let mut best_move = ChessMove::default();

        // do TT cutoffs with alpha already raised by the stand pat check, because that relies on the null move observation
        // but if there's a TT entry from normal search that's worse than the stand pat score, we should trust that more.
        if let Some(tt_entry) = self
            .state
            .custom
            .tt
            .load::<Chessboard>(pos.zobrist_hash(), ply)
        {
            debug_assert_eq!(tt_entry.hash, pos.zobrist_hash());
            let bound = tt_entry.bound();
            // depth 0 drops immediately to qsearch, so a depth 0 entry always comes from qsearch.
            // However, if we've already done qsearch on this position, we can just re-use the result,
            // so there is no point in checking the depth at all
            if (bound == FailHigh && tt_entry.score >= beta)
                || (bound == FailLow && tt_entry.score <= alpha)
                || bound == Exact
            {
                self.state.statistics.tt_cutoff(Qsearch, bound);
                return tt_entry.score;
            }
            best_move = tt_entry.mov;
        }
        self.record_pos(pos, best_score, ply);

        let mut move_picker: MovePicker<Chessboard, MAX_CHESS_MOVES_IN_POS> =
            MovePicker::new(pos, best_move, true);
        let move_scorer = CapsMoveScorer { board: pos, ply };
        let mut children_visited = 0;
        while let Some((mov, score)) = move_picker.next(&move_scorer, &self.state) {
            debug_assert!(mov.is_tactical(&pos));
            if score < MoveScore(0) {
                // qsearch see pruning: If the move has a negative SEE score, don't even bother playing it in qsearch.
                break;
            }
            let Some(new_pos) =
                pos.make_move_and_prefetch_tt(mov, |hash| self.state.custom.tt.prefetch(hash))
            else {
                continue;
            };
            self.record_move(mov, pos, ply, Qsearch);
            children_visited += 1;
            let score = -self.qsearch(new_pos, -beta, -alpha, ply + 1);
            self.undo_move();
            best_score = best_score.max(score);
            if score <= alpha {
                continue;
            }
            bound_so_far = Exact;
            alpha = score;
            best_move = mov;
            if score >= beta {
                bound_so_far = FailHigh;
                break;
            }
        }
        self.state
            .statistics
            .count_complete_node(Qsearch, bound_so_far, 0, ply, children_visited);

        let tt_entry: TTEntry<Chessboard> =
            TTEntry::new(pos.zobrist_hash(), best_score, best_move, 0, bound_so_far);
        self.state.custom.tt.store(tt_entry, ply);
        best_score
    }

    fn eval(&mut self, pos: Chessboard, ply: usize) -> Score {
        if ply == 0 {
            self.eval.eval(&pos)
        } else {
            let old_pos = &self.state.search_stack[ply - 1].pos;
            let mov = &self.state.search_stack[ply - 1].last_tried_move();
            self.eval.eval_incremental(old_pos, *mov, &pos, ply)
        }
    }

    fn record_pos(&mut self, pos: Chessboard, eval: Score, ply: usize) {
        self.state.search_stack[ply].pos = pos;
        self.state.search_stack[ply].eval = eval;
        self.state.search_stack[ply].tried_moves.clear();
    }

    fn record_move(&mut self, mov: ChessMove, old_pos: Chessboard, ply: usize, typ: SearchType) {
        self.state.board_history.push(&old_pos);
        self.state.search_stack[ply].tried_moves.push(mov);
        self.state.statistics.count_legal_make_move(typ);
    }

    fn undo_move(&mut self) {
        self.state.board_history.pop();
    }
}

struct CapsMoveScorer {
    board: Chessboard,
    ply: usize,
}

impl MoveScorer<Chessboard> for CapsMoveScorer {
    type State = CapsState;

    /// Order moves so that the most promising moves are searched first.
    /// The most promising move is always the TT move, because that is backed up by search.
    /// After that follow various heuristics.
    fn score_move(&self, mov: ChessMove, state: &CapsState) -> MoveScore {
        // The move list is iterated backwards, which is why better moves get higher scores
        // No need to check against the TT move because that's already handled by the move picker
        if mov == state.search_stack[self.ply].killer {
            KILLER_SCORE
        } else if !mov.is_tactical(&self.board) {
            let conthist_score = if self.ply > 0 {
                let prev_move = state.search_stack[self.ply - 1].last_tried_move();
                state
                    .custom
                    .cont_hist
                    .score(mov, prev_move, self.board.active_player())
            } else {
                0
            };
            MoveScore(state.custom.history[mov.from_to_square()] + conthist_score)
        } else {
            let captured = mov.captured(&self.board);
            let base_val = if self.board.see_at_least(mov, SeeScore(0)) {
                MoveScore::MAX - MoveScore(HIST_DIVISOR * 50)
            } else {
                MoveScore::MIN + MoveScore(HIST_DIVISOR * 50)
            };
            let hist_val = state.custom.capt_hist.get(mov, self.board.active_player());
            base_val + MoveScore(captured as i32 * HIST_DIVISOR * 2) + hist_val
        }
    }
}

#[cfg(test)]
mod tests {
    use gears::games::chess::Chessboard;
    use gears::games::ZobristHistory;
    use gears::search::NodesLimit;

    use crate::eval::chess::lite::LiTEval;
    use crate::eval::chess::piston::PistonEval;
    use crate::eval::rand_eval::RandEval;

    use super::*;

    #[test]
    fn mate_in_one_test() {
        let board = Chessboard::from_fen("4k3/8/4K3/8/8/8/8/6R1 w - - 0 1").unwrap();
        // run multiple times to get different random numbers from the eval function
        for depth in 1..=3 {
            for _ in 0..100 {
                let mut engine = Caps::for_eval::<RandEval>();
                let res = engine
                    .search(
                        board,
                        SearchLimit::depth(Depth::new(depth)),
                        ZobristHistory::default(),
                        SearchSender::no_sender(),
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
            let mut engine = Caps::for_eval::<PistonEval>();
            let res = engine
                .search_from_pos(pos, SearchLimit::nodes(NodesLimit::new(50_000).unwrap()))
                .unwrap();
            assert!(res.score.is_some_and(|score| score > Score(min)));
            assert!(res.score.is_some_and(|score| score < Score(max)));
        }
    }

    #[test]
    fn lucena_test() {
        let pos = Chessboard::from_name("lucena").unwrap();
        let mut engine = Caps::for_eval::<PistonEval>();
        let res = engine
            .search_from_pos(pos, SearchLimit::depth(Depth::new(7)))
            .unwrap();
        // TODO: More aggressive bound once the engine is stronger
        assert!(res.score.unwrap() >= Score(200));
    }

    #[test]
    fn philidor_test() {
        let pos = Chessboard::from_name("philidor").unwrap();
        let mut engine = Caps::for_eval::<LiTEval>();
        let res =
            engine.search_from_pos(pos, SearchLimit::nodes(NodesLimit::new(100_000).unwrap()));
        // TODO: More aggressive bound once the engine is stronger
        assert!(res.unwrap().score.unwrap().abs() <= Score(200));
    }

    #[test]
    fn kiwipete_test() {
        let pos = Chessboard::from_name("kiwipete").unwrap();
        let mut engine = Caps::for_eval::<LiTEval>();
        let res = engine
            .search_from_pos(pos, SearchLimit::nodes(NodesLimit::new(12_345).unwrap()))
            .unwrap();

        assert!(res.score.unwrap().abs() <= Score(64));
        assert_eq!(
            res.chosen_move,
            ChessMove::from_compact_text("e2a6", &pos).unwrap()
        );
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
            let mut engine = Caps::<LiTEval>::default();
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
