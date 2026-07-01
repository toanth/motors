use std::cmp::min;
use std::fmt::Display;
use std::mem::take;
use std::ops::ControlFlow::{Break, Continue};
use std::ops::{ControlFlow, Deref, DerefMut};
use std::sync::atomic::Ordering::Relaxed;
use std::time::{Duration, Instant};

use crate::eval::chess::lite::{lc, LiTEval};
use crate::eval::Eval;
use crate::io::ugi_output::{color_for_score, score_gradient};
use crate::search::chess::caps_values::cc;
use crate::search::chess::histories::{ContHist, HistScoreT, HIST_DIVISOR};
use crate::search::chess::move_picker::{MovePicker, MovePickerStage, BAD_SEE_OFFSET};
use crate::search::chess::*;
use crate::search::multithreading::ThreadData;
use crate::search::tt::{ttc, TTEntry};
use crate::search::{
    AbstractSearchState, Engine, EngineInfo, MoveScore, NormalEngine, PVData, SearchState, SearchStateFor,
    DEFAULT_CHECK_TIME_INTERVAL,
};
use crate::send_debug_msg;
use gears::colored::Colorize;
use gears::games::chess::bitbase::{Bitbase, PAWN_V_KING_TABLE};
use gears::games::chess::moves::Move;
use gears::games::chess::pieces::PieceType::Pawn;
use gears::games::chess::see::SeeScore;
use gears::games::chess::upcoming_repetition::{UpcomingRepetitionTable, UPCOMING_REPETITION_TABLE};
use gears::games::chess::zobrist::ZOBRIST_KEYS;
use gears::games::chess::{unverified::UnverifiedBoard, Board};
use gears::games::BoardHistDyn;
use gears::general::bitboards::RawBitboardTrait;
use gears::general::board::Strictness::Strict;
use gears::general::board::{BitboardBoard, BoardTrait, UnverifiedBoardTrait};
use gears::general::common::Description::NoDescription;
use gears::general::common::{parse_int_from_str, select_name_static, Res, StaticallyNamedEntity};
use gears::general::move_list::InplaceMoveList;
use gears::general::moves::{MoveTrait, UntrustedMove};
use gears::itertools::Itertools;
use gears::num::traits::WrappingAdd;
use gears::score::{
    game_result_to_score, Score, ScoreT, BITBASE_LOSS, BITBASE_WIN, MAX_BETA, MAX_NORMAL_SCORE, MAX_SCORE_LOST,
    MIN_ALPHA, MIN_NORMAL_SCORE, NO_SCORE_YET, SCORE_LOST, SCORE_WON, UNPROVEN_LOSS, UNPROVEN_WIN,
};
use gears::search::NodeType::*;
use gears::search::*;
use gears::ugi::EngineOptionName::*;
use gears::ugi::{EngineOptionNameForProtocol, EngineOptionType};
use gears::PlayerResult;
use gears::PlayerResult::{Lose, Win};

type DefaultEval = LiTEval;

#[derive(Debug)]
struct Precomputed {
    upcoming_repetition: &'static UpcomingRepetitionTable,
    bitbase: &'static Bitbase,
}

/// Chess-playing Alpha-beta Pruning Search, or in short, CAPS.
/// Larger than SᴍᴀʟʟCᴀᴘꜱ.
#[derive(Debug)]
pub struct Caps {
    state: CapsState,
    eval: Box<dyn Eval<Board>>,
    precomputed: Precomputed,
}

impl Default for Caps {
    fn default() -> Self {
        let thread_data = ThreadData::single_and_no_output();
        Self::new(thread_data, Box::new(DefaultEval::default()))
    }
}

impl Deref for Caps {
    type Target = CapsState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl DerefMut for Caps {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
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
        "CAPS: Chess-playing Alpha-beta Pruning Search".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "Chess-playing Alpha-beta Pruning Search (CAPS), a superhuman chess engine with a hand crafted eval. \
        Much larger than SᴍᴀʟʟCᴀᴘꜱ"
            .to_string()
    }
}

impl Engine<Board> for Caps {
    type SearchStackEntry = CapsSearchStackEntry;
    type CustomInfo = CapsCustomInfo;

    fn new(thread_data: ThreadData<Board>, eval: Box<dyn Eval<Board>>) -> Self {
        let precomputed = Precomputed { upcoming_repetition: &UPCOMING_REPETITION_TABLE, bitbase: &PAWN_V_KING_TABLE };
        Self { state: SearchState::new(thread_data, DepthPly::new(SEARCH_STACK_LEN)), eval, precomputed }
    }

    fn static_eval(&mut self, pos: &Board, ply: usize) -> Score {
        self.eval.eval(pos, ply, self.params.pos.active_player())
    }

    fn max_bench_depth(&self) -> DepthPly {
        ID_ITERS_SOFT_LIMIT
    }

    fn search_state_dyn(&self) -> &dyn AbstractSearchState<Board> {
        &self.state
    }

    fn search_state_mut_dyn(&mut self) -> &mut dyn AbstractSearchState<Board> {
        &mut self.state
    }

    fn eval_move(&self, pos: &Board, mov: Move) -> Option<String> {
        debug_assert!(pos.is_move_pseudolegal(mov));
        let tt_move = self.tt().load::<Board>(pos, 0).and_then(|e| e.mov(pos));
        let move_picker = MovePicker::new(pos, 0, tt_move.unwrap_or_default(), false);
        let (descr, hist_score) = if mov.is_tactical(pos) {
            ("Capture History Score", self.capt_hist.get(mov, pos).0 as isize)
        } else {
            ("Main History Score", self.history.score(mov, pos.threats()))
        };
        let color = color_for_score(Score(hist_score as ScoreT), &score_gradient());
        let hist_score = hist_score.to_string().color(color);
        let move_score = move_picker.complete_move_score(mov, &self.state);
        let move_type =
            if self.tt().load::<Board>(pos, 0).is_some_and(|e| e.move_untrusted() == UntrustedMove::from_move(mov)) {
                "TT move"
            } else if move_score == KILLER_SCORE {
                "Killer move"
            } else if mov.is_tactical(pos) {
                if move_score < MoveScore(0) { "Losing Tactical Move" } else { "Winning Tactical Move" }
            } else {
                "Quiet Move"
            };
        let color = color_for_score(Score(move_score.0 as ScoreT), &score_gradient());
        let move_score = format!("{}", move_score.0).color(color);
        Some(format!("{move_type}\nTotal Move Score: {move_score}\n{descr}: {hist_score}"))
    }

    fn engine_info(&self) -> EngineInfo {
        let mut options = cc::ugi_options();
        options.append(&mut lc::ugi_options());
        options.append(&mut ttc::ugi_options());
        EngineInfo::new(
            self,
            self.eval.as_ref(),
            "0.1.0",
            DepthPly::new(14),
            NodesLimit::new(20_000).unwrap(),
            None,
            options,
        )
    }

    fn set_option(
        &mut self,
        option: EngineOptionNameForProtocol,
        _old_value: &mut EngineOptionType,
        value: String,
    ) -> Res<()> {
        let name = option.to_string();
        if let Other(name) = &option.name
            && let Ok(val) = parse_int_from_str(&value, "spsa option value")
        {
            if cc::set_value(name, val).is_ok() {
                return Ok(());
            } else if let Ok(()) = lc::set_value(name, val) {
                return Ok(());
            } else if let Ok(()) = ttc::set_value(name, val) {
                return Ok(());
            }
        }

        select_name_static(&name, self.engine_info().additional_options().iter(), "uci option", "chess", NoDescription)
            .map(|_| {}) // only called to produce an error message
    }

    fn print_spsa_params(&self) {
        for line in cc::ob_param_string() {
            println!("{line}");
        }
        for line in lc::ob_param_string() {
            println!("{line}");
        }
        for line in ttc::ob_param_string() {
            println!("{line}");
        }
        for line in lc::ob_param_string() {
            println!("{line}");
        }
        for line in ttc::ob_param_string() {
            println!("{line}");
        }
    }

    fn set_eval(&mut self, eval: Box<dyn Eval<Board>>) {
        self.eval = eval;
    }

    fn get_eval(&mut self) -> Option<&dyn Eval<Board>> {
        Some(self.eval.as_ref())
    }

    fn do_search(&mut self) -> SearchResult<Board> {
        let mut limit = self.params.limit;
        let pos = self.params.pos;
        limit.fixed_time = min(limit.fixed_time, limit.tc.remaining);
        self.ply_hard_limit = if limit.mate == 0 { PLY_HARD_LIMIT } else { limit.mate.abs() as usize };
        let soft_limit =
            limit.tc.remaining.saturating_sub(limit.tc.increment) * cc::soft_limit() / 1024 + limit.tc.increment;
        self.params.limit = limit;

        // Use 3fold repetition detection for positions before and including the root node and 2fold for positions during search.
        // Idea from pawnocchio:
        // Instead of actually looking for 3fold repetitions, we simply remove all non-repeated positions so far.
        let mut hist = take(&mut self.search_params_mut().history);
        self.repeated_before_root.clear();
        hist.0.reverse();
        hist.0.truncate(pos.ply_draw_clock());
        hist.push(pos.hash_pos());
        hist.0.sort_by_key(|hash| hash.0);
        for i in 1..hist.0.len() {
            if hist.0[i] == hist.0[i - 1] {
                self.repeated_before_root.push(hist.0[i]);
            }
        }
        debug_assert_eq!(self.uci_nodes(), 0);

        send_debug_msg!(
            self,
            "Starting search with limit {time} microseconds, {incr}ms increment, max {fixed}ms, mate in {mate} plies, max depth {depth}, \
            max {nodes} nodes, soft limit {soft}ms, {ignored} ignored moves. {elapsed} microseconds have already elapsed ({e2} since starting the search in this thread)",
            time = limit.tc.remaining.as_micros(),
            incr = limit.tc.increment.as_millis(),
            mate = limit.mate,
            depth = limit.depth.get(),
            nodes = limit.nodes.get(),
            fixed = limit.fixed_time.as_millis(),
            soft = soft_limit.as_millis(),
            ignored = self.excluded_moves.len(),
            elapsed = limit.start_time.elapsed().as_micros(),
            e2 = self.execution_start_time.elapsed().as_micros()
        );

        let needs_final_info = self.iterative_deepening(&pos, soft_limit);
        send_debug_msg!(self, "Finished iterative deepening; last search incomplete: {needs_final_info}");
        if needs_final_info || self.output_minimal() {
            // Send one final search info, but don't send empty PVs.
            // Even for aborted searches, we never send unproven mates.
            if !self.current_mpv_pv().is_empty() {
                self.send_search_info(true);
                send_debug_msg!(self, "Wrote final search info with best move {:?}", self.to_search_info(true).pv[0]);
            }
        }
        self.search_result()
    }
}

impl NormalEngine<Board> for Caps {
    fn search_state(&self) -> &SearchStateFor<Board, Self> {
        &self.state
    }

    fn search_state_mut(&mut self) -> &mut SearchStateFor<Board, Self> {
        &mut self.state
    }

    fn time_up(&self, tc: TimeControl, fixed_time: Duration, byoyomi: Duration, elapsed: Duration) -> bool {
        debug_assert_eq!(self.uci_nodes() % DEFAULT_CHECK_TIME_INTERVAL, 0);
        // TODO: Compute at the start of the search instead of every time:
        // Instead of storing a SearchLimit, store a different struct that contains soft and hard bounds
        let hard = (tc.remaining.saturating_sub(tc.increment)) * cc::hard_limit() as u32 / 1024 + tc.increment;
        // Because fixed_time has been clamped to at most tc.remaining, this can never lead to timeouts
        // (assuming the move overhead is set correctly)
        elapsed >= byoyomi + fixed_time.min(hard)
    }
}

#[allow(clippy::too_many_arguments)]
impl Caps {
    fn to_search_info(&self, final_info: bool) -> SearchInfo<'_, Board> {
        let mut info = self.state.to_search_info(final_info);
        // Loading a score from the TT can give us an incorrect mate that ignores the 50 move rule.
        // Because that is the only path-dependent effect that can give incorrect mate scores in chess, we simply
        // ignore it during search and fix it up here.
        if info.score.is_won_or_lost() {
            let mut pos = info.pos;
            for (i, &m) in info.pv.iter().enumerate() {
                pos = pos.make_move(m).unwrap();
                if pos.is_50mr_draw() {
                    info.score = info.score.clamp(UNPROVEN_LOSS, UNPROVEN_WIN);
                    info.pv = &info.pv[..i];
                    break;
                }
            }
            // This can happen if we're loading the TT entry of an earlier search, which comes from a higher depth than our current search
            if info.score.plies_until_game_over().is_some_and(|s| s as usize > info.pv.len()) {
                info.score = info.score.clamp(UNPROVEN_LOSS, UNPROVEN_WIN);
            }
        }
        info
    }

    fn send_search_info(&self, final_info: bool) {
        if let Some(mut output) = self.thread_data.thread_type.output() {
            let info = self.to_search_info(final_info);
            output.write_search_info(info);
        }
    }

    /// Iterative Deepening (ID): Do a depth 1 search, then a depth 2 search, then a depth 3 search, etc.
    /// This has two advantages: It allows the search to be stopped at any time, and it actually improves strength:
    /// The low-depth searches fill the TT and various heuristics, which improves move ordering and therefore results in
    /// better moves within the same time or nodes budget because the lower-depth searches are comparatively cheap.
    /// Returns true if the last iteration was incomplete
    fn iterative_deepening(&mut self, pos: &Board, soft_limit: Duration) -> bool {
        // let phase = pos.phase().clamp(0, 24);
        // let increment = (cc::min_depth_incremenet() * phase + cc::max_depth_incremenet() * (24 - phase)) / 24;
        // we multiply the depth limit by the depth increment to achieve a more consistent behavior of 'go depth'.
        let max_iter = self.limit().depth.get();
        let multi_pv = self.multi_pv();
        let mut soft_limit_scale = 1.0;

        self.multi_pvs.resize(multi_pv, PVData::default());
        let mut chosen_at_iter = InplaceMoveList::<Board, { ID_ITERS_SOFT_LIMIT.get() }>::default();

        for (iter, budget) in
            (cc::start_depth()..=(ID_ITERS_SOFT_LIMIT.get() * DEPTH_INCREMENT)).step_by(DEPTH_INCREMENT).enumerate()
        {
            if iter >= max_iter {
                break;
            }
            self.atomic().set_iteration(iter + 1);
            self.budget = Budget::new(budget);
            for pv_num in 0..multi_pv {
                self.current_pv_num = pv_num;
                self.cur_pv_data_mut().bound = None;
                let scaled_soft_limit = soft_limit.mul_f64(soft_limit_scale);
                self.atomic().reset_seldepth();
                let (keep_searching, needs_final_info, score) =
                    self.aspiration(pos, scaled_soft_limit, iter, budget as isize);

                let pv = &self.state.search_stack[0].pv;

                if !pv.is_empty() {
                    if self.current_pv_num == 0 {
                        let chosen_move = pv.get(0).unwrap();
                        let ponder_move = pv.get(1);
                        self.atomic().set_best_move(chosen_move);
                        self.atomic().set_ponder_move(ponder_move);
                    }
                    self.state.multi_pvs[self.state.current_pv_num].pv.assign_from(pv);
                    if let Some(score) = score {
                        debug_assert!(score.is_valid());
                        if pv_num == 0 {
                            self.atomic().set_score(score);
                        } else {
                            _ = self.atomic().get_score().fetch_max(score.0, Relaxed);
                        }
                    }
                }

                if keep_searching == Break(()) {
                    return needs_final_info;
                }
                if let Some(chosen_move) = self.search_stack[0].pv.get(0) {
                    self.excluded_moves.push(chosen_move);
                }
            }
            self.state.excluded_moves.truncate(self.excluded_moves.len() - multi_pv);
            let chosen = self.atomic().best_move();
            chosen_at_iter.push(chosen);
            if iter >= cc::move_stability_min_iters()
                && !is_duration_infinite(soft_limit)
                && chosen_at_iter.iter().dropping(iter * cc::move_stability_start() / 1024).all(|m| *m == chosen)
            {
                soft_limit_scale = cc::move_stability_factor() as f64 / 1000.0;
            } else {
                soft_limit_scale = 1.0;
            }
        }
        // count an additional node to keep the game reproducible
        _ = self.atomic().count_node();
        false
    }

    /// Aspiration Windows (AW): Assume that the score will be close to the score from the previous iteration
    /// of Iterative Deepening, so use alpha, beta bounds around that score to prune more aggressively.
    /// This means that it's possible for the root to fail low (or high), which is always something to consider:
    /// For example, the best move is not trustworthy if the root failed low (but because the TT move is ordered first,
    /// and the TT move at the root is always `state.best_move` (there can be no collisions because it's written to last),
    /// it is still trustworthy except for depth 1 or when doing a multipv search)
    fn aspiration(
        &mut self,
        pos: &Board,
        unscaled_soft_limit: Duration,
        iter: usize,
        budget: isize,
    ) -> (ControlFlow<()>, bool, Option<Score>) {
        let mut soft_limit_fail_low_extension = 1.0;
        let mut aw_budget = budget;
        let mut needs_final_info = false;
        loop {
            let alpha = self.cur_pv_data().alpha;
            let beta = self.cur_pv_data().beta;
            let mut window_radius = self.cur_pv_data().radius.0;
            // limit.fixed time is the min of the fixed time and the remaining time
            let mut soft_limit = unscaled_soft_limit.mul_f64(soft_limit_fail_low_extension);
            soft_limit_fail_low_extension = 1.0;
            if budget > cc::soft_limit_node_scale_min_budget() && self.multi_pvs.len() == 1 {
                let node_frac = self.root_move_nodes.frac_1024(self.cur_pv_data().pv.list[0], self.uci_nodes());
                debug_assert!((0..=1024).contains(&node_frac));
                soft_limit = soft_limit
                    .mul_f64(((1024 + 512 - node_frac) * cc::soft_limit_node_scale()) as f64 / (1024.0 * 1024.0));
            }
            let limit = self.params.limit.tc;
            let soft_limit = soft_limit
                .min(
                    (limit.remaining.saturating_sub(limit.increment)) * cc::soft_limit_clamp() / 1024 + limit.increment,
                )
                .min(self.params.limit.fixed_time);
            let elapsed = self.start_time().elapsed();
            if self.should_not_start_negamax(
                elapsed,
                soft_limit,
                self.limit().soft_nodes.get(),
                iter as isize,
                ID_ITERS_SOFT_LIMIT.isize(),
                self.limit().mate,
            ) {
                // increase the node counter by one to ensure the game is reproducible
                _ = self.atomic().count_node();
                send_debug_msg!(
                    self,
                    "Not starting negamax after {0} microseconds, {iter} iterations. PV: {1:?}, best move: {2:?}",
                    elapsed.as_micros(),
                    self.to_search_info(true).pv,
                    self.atomic().best_move()
                );
                debug_assert_eq!(self.pv_data()[0].pv.get(0).unwrap(), self.atomic().best_move());
                return (Break(()), needs_final_info, None);
            }
            send_debug_msg!(self, "Starting new aspiration window search after {} microseconds", elapsed.as_micros());
            needs_final_info = true;

            let asp_start_time = Instant::now();
            let Some(pv_score) = self.negamax(pos, 0, aw_budget, alpha, beta, Exact, None) else {
                send_debug_msg!(
                    self,
                    "Exiting aw window after reaching a stop condition in negamax, after {} microseconds",
                    self.start_time().elapsed().as_micros()
                );
                return (Break(()), true, None);
            };

            send_debug_msg!(
                self,
                "depth {budget}, score {0}, radius {window_radius}, interval ({1}, {2}) nodes {3}, elapsed microseconds: {4}",
                pv_score.0,
                alpha.0,
                beta.0,
                self.uci_nodes(),
                self.start_time().elapsed().as_micros()
            );

            let node_type = pv_score.node_type(alpha, beta);

            // we don't trust the best move in fail low nodes, but we still want to display an updated score
            self.cur_pv_data_mut().score = pv_score;
            self.cur_pv_data_mut().bound = Some(node_type);
            if node_type == FailLow {
                // In a fail low node, we didn't get any new information, and it's possible that we just discovered
                // a problem with our chosen move. So increase the soft limit such that we can gather more information.
                soft_limit_fail_low_extension = cc::soft_limit_fail_low_factor() as f64 / 1000.0;
                aw_budget = budget;
            } else if node_type == FailHigh && budget >= cc::fail_high_reduction_min_depth() {
                // If the search discovers an unexpectedly good move, it can take a long while to search it because the TT isn't filled
                // and because even with fail soft, scores tend to fall close to the aspiration window. So reduce the depth to speed this up.
                aw_budget = (aw_budget - cc::fail_high_reduction()).max(budget - cc::fail_high_max_reduction());
            }

            if cfg!(debug_assertions) {
                let pv = &self.search_stack[0].pv;
                if pos.calc_player_result(&self.params.history).is_some() {
                    assert!(pv.len() <= 1); // only check/stalemates are checked at the root
                } else if node_type == Exact {
                    debug_assert!(
                        // currently, it's possible to reduce the PV through IIR when the TT entry of a PV node gets overwritten,
                        // but that should be relatively rare. In the future, a better replacement policy might make this actually sound
                        self.multi_pv() > 1
                            || pv.len() + pv.len() / 4 + 5
                                >= self.ply_hard_limit.min(aw_budget as usize / DEPTH_INCREMENT)
                            || pv_score.is_won_lost_or_draw_score(),
                        "{aw_budget} {budget} {0} {pv_score} {1}",
                        pv.len(),
                        self.uci_nodes()
                    )
                }
                // assert this now because this doesn't hold for incomplete iterations
                debug_assert!(pv_score.plies_until_game_over().is_none_or(|p| p <= 500), "{pv_score}");
            }

            if node_type == Exact {
                window_radius = (window_radius + cc::aw_exact_add()) * cc::aw_exact_inv_div() / 1024;
            } else {
                let delta = pv_score.0.abs_diff(alpha.0);
                let delta = delta.min(pv_score.0.abs_diff(beta.0));
                let delta = delta.min(cc::aw_delta_max()) as i32;
                window_radius = SCORE_WON.0.min(window_radius * cc::aw_widening_factor() + delta);
            }
            self.cur_pv_data_mut().radius.0 = window_radius;
            self.cur_pv_data_mut().alpha = (pv_score - window_radius).max(MIN_ALPHA);
            self.cur_pv_data_mut().beta = (pv_score + window_radius).min(MAX_BETA);

            if node_type == Exact {
                self.send_search_info(false);
                return (Continue(()), true, Some(pv_score));
            } else if asp_start_time.elapsed().as_millis() >= 1000 {
                self.send_search_info(false);
                needs_final_info = false;
            }
        }
    }

    /// Recursive search function, the most important part of the engine. If the computed score of the current position
    /// lies within the open interval `(alpha, beta)`, return the score. Otherwise, the returned score might not be exact,
    /// but could be closer to the window than the true score. On top of that, there are **many** additional techniques
    /// that can mess with the returned score, so that it's best not to assume too much: For example, it's not unlikely
    /// that a re-search with the same depth returns a different score. Because of PVS, `alpha` is `beta - 1` in almost
    /// all nodes, and most nodes either get cut off before reaching the move loop or produce a beta cutoff after
    /// the first move.
    #[allow(clippy::too_many_lines)]
    fn negamax(
        &mut self,
        pos: &Board,
        ply: usize,
        mut depth: isize,
        mut alpha: Score,
        mut beta: Score,
        mut expected_node_type: NodeType,
        excluded_move: Option<Move>,
    ) -> Option<Score> {
        debug_assert!(alpha < beta, "{alpha} {beta} {pos} {ply} {depth}");
        debug_assert!([MIN_ALPHA, alpha, beta, MAX_BETA].is_sorted(), "{alpha} {beta} {ply} {depth} {pos}");
        debug_assert!(ply <= PLY_HARD_LIMIT, "{ply} {depth} {pos}");
        debug_assert!(depth <= ID_ITERS_SOFT_LIMIT.isize() * DEPTH_INCREMENT as isize, "{ply} {depth} {pos}"); // TODO: Remove?
        debug_assert!(self.params.history.len() >= ply, "{ply} {depth} {pos}, {:?}", self.params.history);
        // We have to increment the node counter as we're checking all other stop conditions in order to ensure games are reproducible
        // by their node counts
        if self.count_node_and_test_stop() {
            return None;
        }

        assert_eq!(depth % 128, 0); // TODO: Remove

        let root = ply == 0;
        let pv_node = expected_node_type == Exact; // TODO: Make this a generic argument of search?
        debug_assert!(!root || pv_node); // root implies pv node
        debug_assert!(alpha + 1 == beta || pv_node); // alpha + 1 < beta implies Exact node
        if pv_node {
            self.search_stack[ply].pv.clear();
        }

        let mut best_score = NO_SCORE_YET;
        let in_singular_search = excluded_move.is_some();

        // Always search all children at the root, even for draws or if a search limit has been reached
        if !root {
            // If there is a move that can repeat a position we've looked at during search, we are guaranteed at least a draw score.
            // So don't even bother searching other moves if the draw score would already cause a cutoff.
            if pos.has_upcoming_repetition(self.precomputed.upcoming_repetition, &self.params.history) {
                alpha = alpha.max(Score(0));
                best_score = Score(0);
            }
            // Mate Distance Pruning (MDP): If we've already found a mate in n, don't bother looking for longer mates.
            // This isn't intended to gain elo (since it only works in positions that are already won or lost)
            // but makes the engine better at finding shorter checkmates. Don't do MDP at the root because that can prevent us
            // from ever returning exact scores, since for a mate in 1 the score would always be exactly `beta`.
            if self.current_pv_num == 0 {
                alpha = alpha.max(game_result_to_score(Lose, ply));
                beta = beta.min(game_result_to_score(Win, ply));
            }
            if alpha >= beta {
                return Some(alpha);
            }
            if pos.is_50mr_draw() || pos.has_insufficient_material()
                // no need to check for twofold repetitions as that is already handled by the upcoming repetition detection
                || self.repeated_before_root.contains(&pos.hash_pos())
            {
                return Some(Score(0));
            }
        }

        let us = pos.active_player();
        let in_check = pos.is_in_check();
        // Check extensions. Increase the depth by 1 if in check.
        // Do this before deciding whether to drop into qsearch.
        if in_check {
            depth += cc::check_extension();
        }
        // self.ply_hard_limit is the min of the original limit.mate and DEPTH_HARD_LIMIT
        if depth <= 0 || ply >= self.ply_hard_limit {
            return self.qsearch(pos, alpha, beta, ply, pv_node);
        }

        let can_prune = !pv_node && !in_check;

        let mut bound_so_far = FailLow;

        let mut continued = None;
        if ply >= 2 {
            let entry = &self.state.search_stack[ply - 2];
            let mov = entry.last_tried_move();
            if !mov.is_null() {
                let piece = mov.piece_type(&entry.pos);
                continued = Some((mov, piece));
            }
        }

        // ************************
        // ***** Probe the TT *****
        // ************************

        // If we didn't get a move from the TT and there's no best_move to store because the node failed low,
        // store a null move in the TT. This helps IIR.
        let mut best_move = Move::default();
        // Don't initialize eval just yet to save work in case we get a TT cutoff
        let raw_eval;
        let mut eval;
        // the TT entry at the root is useless when doing a multipv search with pv num > 1
        let ignore_tt_entry = (root && self.multi_pvs.len() > 1) || in_singular_search;
        let old_entry = self.tt().load::<Board>(&pos, ply);
        if let Some(tt_entry) = old_entry
            && !ignore_tt_entry
        {
            let tt_bound = tt_entry.bound();
            debug_assert!(tt_entry.hash_part().equals(pos.hash_pos()));

            if let Some(tt_move) = tt_entry.mov(pos) {
                best_move = tt_move;
            }
            let tt_score = tt_entry.score();
            let tt_depth = tt_entry.depth() as isize;
            // TT cutoffs. If we've already seen this position, and the TT entry has more valuable information (higher depth),
            // and we're not a PV node, and the saved score is either exact or at least known to be outside (alpha, beta),
            // simply return it.
            let depth_diff = 0.max(depth - tt_depth);
            // idea from david: Do TT cutoffs even if the tt depth is lower than the current depth, as long as the tt bound is far above beta
            let beta_cutoff_threshold =
                beta + (depth_diff * depth_diff * cc::tt_cutoff_margin() / (128 * 128)) as ScoreT;
            if !pv_node && (tt_depth >= depth || (tt_bound != FailLow && tt_score >= beta_cutoff_threshold)) {
                if (tt_bound == NodeType::lower_bound() && tt_score >= beta_cutoff_threshold)
                    || (tt_bound == NodeType::upper_bound() && tt_score <= alpha)
                    || tt_bound == Exact
                {
                    // Idea from stormphrax
                    if tt_score >= beta
                        && !best_move.is_tactical(pos)
                        && self.search_stack[ply].pos.is_generated_move_pseudolegal(best_move)
                    {
                        self.search_stack[ply].killer = best_move;
                        self.update_histories(best_move, depth, ply, tt_score - beta);
                    }
                    return Some(tt_score);
                } else if depth <= cc::low_depth_tt_extension_depth() && tt_depth >= depth {
                    // also from stormphrax
                    depth += cc::tt_extension();
                }
            }
            // Even though we didn't get a cutoff from the TT, we can still use the score and bound to update our guess
            // at what the type of this node is going to be.
            if !pv_node {
                expected_node_type = if tt_bound != Exact {
                    tt_bound
                } else if tt_score <= alpha {
                    FailLow
                } else {
                    debug_assert!(tt_score >= beta); // we're using a null window
                    FailHigh
                }
            }
            raw_eval = tt_entry.raw_eval();
            eval = self.state.custom.corr_hist.correct(pos, continued, raw_eval);
            // The TT score is backed by a search, so it should be more trustworthy than a simple call to static eval.
            // Note that the TT score may be a mate score, so `eval` can also be a mate score. This doesn't currently
            // create any problems, but should be kept in mind.
            if tt_bound == Exact
                || (tt_bound == NodeType::lower_bound() && tt_score >= eval)
                || (tt_bound == NodeType::upper_bound() && tt_score <= eval)
            {
                eval = tt_score;
            }
        } else if in_singular_search {
            eval = self.search_stack[ply].eval;
            debug_assert!(!eval.is_won_or_lost());
            raw_eval = match old_entry {
                Some(e) => e.raw_eval(),
                None => self.eval(pos, ply), // this can happen if another thread overwrites the TT entry
            };
        } else {
            raw_eval = self.eval(pos, ply);
            eval = self.state.custom.corr_hist.correct(pos, continued, raw_eval);
        };

        self.record_pos(pos, eval, ply);

        // If the current position is noisy, we want to be more conservative with margins.
        // However, captures and promos are generally good moves, so if our eval is the static eval instead of adjusted from the TT,
        // a noisy condition would mean we're doing even better than expected. // TODO: Apply noisy for RFP etc only if eval is TT eval?
        // If it's from the TT, however, and the first move didn't produce a beta cutoff, we're probably worse than expected
        let pos_noisy = in_check || best_move.is_tactical(pos);

        // Like the commonly used `improving` and `regressing`, these variables compare the current static eval with
        // the static eval 2 plies ago to recognize blunders. Conceptually, `improving` and `regressing` can be seen as
        // a prediction for how the eval is going to evolve, while these variables are more about cutting early after bad moves.
        let they_blundered = ply >= 2 && eval - self.search_stack[ply - 2].eval > Score(cc::they_blundered_threshold());
        let we_blundered = ply >= 2 && eval - self.search_stack[ply - 2].eval < Score(cc::we_blundered_threshold());

        // *********************************************************
        // ***** Pre-move loop pruning (other than TT cutoffs) *****
        // *********************************************************

        let mut nmp_verif_score = None;
        if can_prune && !in_singular_search {
            // RFP (Reverse Futility Pruning): If eval is far above beta, it's likely that our opponent
            // blundered in a previous move of the search, so if the depth is low, don't even bother searching further.
            // Use `they_blundered` to better distinguish between blunders by our opponent and a generally good static eval
            // relative to `beta` --  there may be other positional factors that aren't being reflected by the static eval,
            // (like imminent threats) so don't prune too aggressively if our opponent hasn't blundered.
            // Be more careful about pruning too aggressively if the node is expected to fail low -- we should not rfp
            // a true fail low node, but our expectation may also be wrong.
            // TODO: introduce tunable constant without `()` (changes bench)
            let mut margin =
                (cc::rfp_base() - (ScoreT::from(they_blundered) * cc::rfp_blunder())) * (depth / 128) as ScoreT;
            if expected_node_type == FailHigh {
                // TODO: Multiplicative constant (changes bench)
                margin = margin * cc::rfp_fail_high() / 1024;
            }
            if let Some(entry) = old_entry
                && entry.score() <= eval
                && entry.bound() == NodeType::upper_bound()
            {
                margin += margin * cc::rfp_tt_upper_bound() / 1024;
            }
            if pos_noisy {
                margin += margin * cc::rfp_noisy_pos() / 1024;
            }
            debug_assert_ne!(ply, 0);
            let parent_move_score = self.search_stack[ply - 1].move_score;
            if parent_move_score < MoveScore(0) {
                margin -= margin / 4;
            }

            if depth <= cc::rfp_max_depth() && eval >= beta + Score(margin) {
                // Fail firm: Static eval isn't super trustworthy, and we can assume that scores are close to (alpha, beta).
                // So interpolate between eval and beta, which should lead to more stable scores.
                return Some(if eval.is_normal_score() && beta.is_normal_score() {
                    (eval * cc::rf_fail_firm_factor() + beta * (1024 - cc::rf_fail_firm_factor())) / 1024
                } else {
                    eval
                });
            }

            // Razoring. If the position appears hopeless, drop into qsearch immediately.
            // This obviously has the potential to miss quite a few tactics, so only do this at low depths and when the
            // difference between the static eval and alpha is really large, and also not when we could miss a mate from the TT.
            if depth <= cc::razor_max_depth()
                && eval + Score((cc::razor_depth_mult() * depth / 1024) as ScoreT) < alpha
                && !eval.is_proven_loss()
            {
                let qsearch_score = self.qsearch(pos, alpha, beta, ply, false)?;
                if qsearch_score <= alpha {
                    return Some(qsearch_score);
                }
                self.search_stack[ply].tried_moves.clear();

                // Since we're in a non-pv node, qsearch must have failed high. So assume that a normal search also fails high.
                expected_node_type = FailHigh;
                // Now that we have a qsearch score, use that instead of static eval if the eval isn't from the TT
                if old_entry.is_none() {
                    eval = qsearch_score;
                }
            }

            // NMP (Null Move Pruning). If static eval of our position is above beta, this node probably isn't that interesting.
            // To test this hypothesis, do a null move and perform a search with reduced depth; if the result is still
            // above beta, then it's very likely that the score would have been above beta if we had played a move,
            // so simply return the nmp score. This is based on the null move observation (there are very few zugzwang positions).
            // If we don't have non-pawn, non-king pieces, we're likely to be in zugzwang, so don't even try NMP.
            let has_nonpawns = (pos.active_player_bb() & !pos.piece_bb(Pawn)).more_than_one_bit_set();
            let nmp_threshold = beta;
            if depth >= cc::nmp_min_depth()
                && eval >= nmp_threshold
                && expected_node_type == FailHigh
                && !*self.nmp_disabled_for(us)
                && has_nonpawns
            {
                self.tt().prefetch(pos.hash_pos() ^ ZOBRIST_KEYS.side_to_move_key);
                // `make_nullmove` resets the 50mr counter, so we don't consider positions after a nullmove as repetitions,
                // but we can still get TT cutoffs
                self.params.history.push(pos.hash_pos());
                let new_pos = pos.make_nullmove().unwrap();
                // necessary to recognize the null move and to make `last_tried_move()` not panic
                self.search_stack[ply].tried_moves.push(Move::default());
                // TODO: Remove / 128 and * 128
                let reduction = cc::nmp_base()
                    + depth / 128 * cc::nmp_depth() / 1024 * 128
                    + isize::from(they_blundered) * cc::nmp_blunder()
                    + (eval - nmp_threshold + 1).0.ilog2().saturating_sub(8) as isize * 128;
                // the child node is expected to fail low, leading to a fail high in this node
                let nmp_res = self.negamax(&new_pos, ply + 1, depth - reduction, -beta, -beta + 1, FailLow, None);
                _ = self.search_stack[ply].tried_moves.pop();
                self.params.history.pop();
                let score = -nmp_res?;
                if score >= beta {
                    // For shallow depths, don't bother with doing a verification search to avoid useless re-searches,
                    // unless we'd be storing a mate score -- we really want to avoid storing unproven mates in the TT.
                    // It's possible to beat beta with a score of getting mated, so use `is_won_or_lost`
                    // instead of `is_proven_win`
                    if depth < cc::nmp_verif_depth() && !score.is_won_or_lost() {
                        return Some(score);
                    }
                    *self.nmp_disabled_for(us) = true;
                    // even though we don't do a null move, we still use the same reduction
                    nmp_verif_score = self.negamax(pos, ply, depth - reduction, beta - 1, beta, FailHigh, None);
                    self.search_stack[ply].tried_moves.clear();
                    *self.nmp_disabled_for(us) = false;
                    // The verification score is more trustworthy than the nmp score.
                    if nmp_verif_score.is_none_or(|score| score >= beta) {
                        return nmp_verif_score;
                    }
                }
            }
        }

        // Reverse Futility Reductions: A similar idea to RFP, but done at higher depths.
        // Here, NMP has failed but our eval is still looking great, so do a verification search and if that succeeds,
        // reduce the depth.
        if depth >= cc::rfr_min_depth()
            // TODO: Constant (changes bench)
            && eval >= beta + Score(32 * (depth / 128) as ScoreT) && !in_check && !root && nmp_verif_score.is_none()
        {
            let reduction = (depth / 128) / 2 * 128; // TODO: Turn into multiplcation with constant (changes bench)
            let score = self.negamax(pos, ply, depth - 128 - reduction, beta - 1, beta, FailHigh, None)?;
            if score >= beta {
                depth -= cc::rfr_reduction();
            }
            self.search_stack[ply].tried_moves.clear();
        }

        // IIR (Internal Iterative Reductions): If we don't have a TT move, this node will likely take a long time
        // because the move ordering won't be great, so don't spend too much time on it.
        // Instead, search it with reduced depth to fill the TT entry so that we can re-search it faster the next time
        // we see this node. If there was no TT entry because the node failed low, this node probably isn't that interesting,
        // so reducing the depth also makes sense in this case.
        if depth >= cc::iir_min_depth() && best_move == Move::default() {
            depth -= cc::iir_reduction();
        }

        self.maybe_send_currline(pos, alpha, beta, ply, None);

        // An uninteresting move is a quiet move or bad capture unless it's the TT or killer move
        // (i.e. it's every move that gets ordered after the killer). The name is a bit dramatic, the first few of those
        // can still be good candidates to explore.
        let mut num_uninteresting_visited = 0;
        debug_assert!(self.search_stack[ply].tried_moves.is_empty());

        // *************************
        // ***** The move loop *****
        // *************************

        debug_assert!(depth % 128 == 0); // TODO: Remove

        let mut move_picker = MovePicker::new(pos, ply, best_move, false);
        let mut child_depth = depth - 128;
        while let Some(sm) = move_picker.next(self) {
            let mov = sm.mov();
            let move_score = sm.score();
            self.tt().prefetch(pos.approx_hash_after(mov));
            if best_score > MAX_SCORE_LOST && !in_check && !root {
                // LMP (Late Move Pruning): Trust the move ordering and assume that moves ordered late aren't very interesting,
                // so don't even bother looking at them in the last few layers.
                // TODO: Use a quadratic formula and get rid of the max depth parameter
                let mut lmp_threshold = if we_blundered {
                    cc::lmp_blunder_base() + cc::lmp_blunder_scale() * depth
                } else {
                    cc::lmp_base() + cc::lmp_scale() * depth
                } / 1024;
                // LMP faster if we expect to fail low anyway
                if expected_node_type == FailLow {
                    lmp_threshold -= lmp_threshold * cc::lmp_fail_low() / 1024;
                }
                if depth <= cc::max_lmp_depth() && num_uninteresting_visited >= lmp_threshold {
                    break;
                }
                // FP (Futility Pruning): If eval is far below alpha,
                // then it's unlikely that a quiet move can raise alpha: We've probably blundered at some prior point in search,
                // so cut our losses and return.
                let fp_margin = if we_blundered {
                    cc::fp_blunder_base() + cc::fp_blunder_scale() * depth
                } else {
                    cc::fp_base() + cc::fp_scale() * depth
                } / 1024;
                if move_picker.stage() == MovePickerStage::Quiets
                    && eval + Score(fp_margin as ScoreT) < alpha
                    && alpha <= MAX_NORMAL_SCORE
                    && depth <= cc::max_fp_depth()
                {
                    move_picker.skip_quiets();
                    continue;
                }
                // History Pruning: At very low depth, don't play quiet moves with bad history scores
                assert_eq!(depth % 128, 0);
                // TODO: Remove '()', change order and use / 1024 instead of / 128 (changes bench)
                if move_picker.stage() == MovePickerStage::Quiets
                    && (move_score.0 as isize) < -150 * (depth / 128)
                    && depth <= cc::hist_pruning_max_depth()
                {
                    move_picker.skip_quiets();
                    continue;
                }
                // PVS SEE pruning: Don't play moves with bad SEE scores at low depth.
                // Be less aggressive with pruning captures to avoid overlooking tactics.
                let bad_tactical = move_score < MoveScore(-HIST_DIVISOR * 8);
                // TODO: Tunable constants, different divisors (changes bench)
                let mut see_threshold =
                    if bad_tactical { (-depth * depth * 50 / (128 * 128)) as i32 } else { (-depth * 80 / 128) as i32 };
                let bad_or_quiet_hist_score = if bad_tactical { move_score - BAD_SEE_OFFSET } else { move_score };
                see_threshold -= bad_or_quiet_hist_score.0 as i32 * cc::see_pruning_hist_mult() / 1024;
                if move_score < KILLER_SCORE
                    && depth <= cc::max_see_pruning_depth()
                    && !pos.see_at_least(mov, SeeScore(see_threshold))
                {
                    continue;
                }
            }

            if (root && self.excluded_moves.contains(&mov)) || excluded_move == Some(mov) {
                continue;
            }
            let new_pos = pos.play(mov);
            #[cfg(debug_assertions)]
            let debug_history_len = self.params.history.len();
            self.record_move(mov, pos, ply, move_score);
            let first_child = self.search_stack[ply].tried_moves.len() == 1;

            if root {
                if depth >= 1024 && self.limit().start_time.elapsed().as_millis() >= 3000 {
                    let move_num = self.search_stack[0].tried_moves.len();
                    // `qsearch` would give better results, but would make bench be nondeterministic
                    let score = -self.eval(&new_pos, 0);
                    self.send_currmove(mov, move_num, score, alpha, beta);
                }
            }
            if move_score < KILLER_SCORE {
                num_uninteresting_visited += 1;
            }
            let tactical = mov.is_tactical(pos);

            let nodes_before_move = self.state.uci_nodes();
            // PVS (Principal Variation Search): Assume that the TT move is the best move, so we only need to prove
            // that the other moves are worse, which we can do with a zero window search. Should this assumption fail,
            // re-search with a full window.
            let mut score;
            let mut child_alpha = -beta;
            let child_beta = -alpha;
            if first_child {
                // Singular Extensions (SE): If the TT move is far better than all other moves, extend it. To find out whether that is
                // the case, search all other moves to a low depth.
                let mut first_child_depth = child_depth - cc::first_child_reduction();
                if let Some(e) = old_entry
                    && depth >= cc::se_depth()
                    && mov == best_move
                    && e.bound() != FailLow
                    && e.depth() as isize >= depth - cc::se_depth_margin()
                    && !e.score().is_won_or_lost()
                    && !root
                {
                    debug_assert!(!in_singular_search);
                    self.search_stack[ply].tried_moves.clear();
                    self.params.history.pop();
                    let reduced_depth = (first_child_depth / 128 * cc::se_reduced_factor() / 1024) * 128;
                    let singular_beta =
                        (e.score() - Score(cc::se_beta_scale() * depth as ScoreT / 1024)).max(MIN_NORMAL_SCORE + 1);
                    let singular_score = self.negamax(
                        pos,
                        ply,
                        reduced_depth,
                        singular_beta - 1,
                        singular_beta,
                        FailLow,
                        Some(best_move),
                    );
                    self.search_stack[ply].tried_moves.clear();
                    let singular_score = singular_score?;
                    if singular_score < singular_beta {
                        first_child_depth += cc::se_extension();
                    } else if singular_score >= beta && !pv_node {
                        // Multi-Cut Pruning: If we fail high at low depth even without the TT move (which also failed high previously),
                        // chances are we'll fail high in a proper search. So don't bother searching and just fail high now.

                        return Some(singular_score);
                    }
                    self.record_move(mov, pos, ply, move_score);
                    #[cfg(debug_assertions)]
                    debug_assert_eq!(self.params.history.len(), debug_history_len + 1);
                }

                let child_node_type = expected_node_type.inverse();
                score = -self.negamax(
                    &new_pos,
                    ply + 1,
                    first_child_depth,
                    child_alpha,
                    child_beta,
                    child_node_type,
                    None,
                )?;
            } else {
                child_alpha = -(alpha + 1);
                // LMR (Late Move Reductions): Trust the move ordering (quiet history, continuation history and capture history heuristics)
                // and assume that moves ordered later are worse. Therefore, we can do a reduced-depth search with a null window
                // to verify our belief.
                // I think it's common to have a minimum depth for doing LMR, but not having that gained elo.
                let mut reduction = 0;
                if num_uninteresting_visited >= cc::lmr_min_uninteresting() {
                    // TODO: Constants (changes bench)
                    reduction = (depth / 128) / cc::lmr_depth_div() * 128
                        + (num_uninteresting_visited + 1).ilog2() as isize * cc::lmr_moves_mult()
                        + cc::lmr_const();
                    // Reduce bad captures and quiet moves with bad combined history scores more.
                    // todo: move out of num_uninteresting_visited >= ... condition
                    if move_score < MoveScore(cc::lmr_bad_hist()) {
                        reduction += cc::lmr_bad_hist_reduction();
                    } else if move_score > MoveScore(cc::lmr_good_hist()) {
                        // Since the TT and killer move and good captures are not lmr'ed,
                        // this only applies to quiet moves with a good combined history score.
                        reduction -= cc::lmr_good_hist_reduction();
                    }
                    if !pv_node {
                        reduction += cc::lmr_no_pv_reduction();
                    }
                    if we_blundered {
                        reduction += cc::lmr_we_blundered_reduction();
                    }
                    if new_pos.is_in_check() {
                        reduction -= cc::lmr_new_in_check_reduction();
                    }
                    if in_check {
                        reduction -= cc::lmr_in_check_reduction();
                    }
                }
                // Futility Reduction: If this move is not a TT move, good SEE capture or killer, and our eval is significantly
                // less than alpha, reduce.
                if !in_check
                    && depth >= cc::min_fr_depth()
                    && move_score < KILLER_SCORE
                    && eval + cc::fr_base() + ((depth * cc::fr_scale() / 1024) as ScoreT) < alpha
                {
                    // TODO: Constants, multiply instead of div (changes bench)
                    reduction += (1 + depth / 512).ilog2() as isize * cc::fr_mult();
                }
                // if the TT move is a capture and we didn't already fail high, it's likely that later moves are worse
                if !in_check && pos_noisy {
                    reduction += cc::tt_capt_reduction();
                }
                if tactical {
                    let hist = self.capt_hist.get(mov, pos);
                    if hist <= MoveScore(cc::lmr_bad_capthist()) {
                        reduction += cc::lmr_bad_capthist_reduction();
                    } else if hist >= MoveScore(cc::lmr_good_capthist()) {
                        reduction -= cc::lmr_good_capthist_reduction();
                    }
                }
                // this ensures that check extensions prevent going into qsearch while in check
                reduction = reduction.min(child_depth).max(0);

                score = -self.negamax(
                    &new_pos,
                    ply + 1,
                    child_depth - reduction,
                    child_alpha,
                    child_beta,
                    FailHigh,
                    None,
                )?;
                // If the score turned out to be better than expected (at least `alpha`), this might just be because
                // of the reduced depth. So do a full-depth search first, but don't use the full window quite yet.
                if alpha < score && reduction >= cc::min_reduction_research() {
                    // do deeper / shallower: Adjust the first re-search depth based on the result of the first search
                    let mut retry_depth = child_depth - cc::retry_base_reduction();
                    // TODO: Constants (changes bench)
                    if score > alpha + cc::do_deeper_base() + (depth * 4 / 128) as ScoreT {
                        retry_depth += cc::do_deeper_val();
                    } else if score < alpha + cc::do_shallower_base() {
                        retry_depth -= cc::do_shallower_val();
                    }
                    // we still expect the child to fail high here
                    score = -self.negamax(&new_pos, ply + 1, retry_depth, child_alpha, child_beta, FailHigh, None)?;
                }

                // If the full-depth null-window search performed better than expected, do a full-depth search with the
                // full window to find the true score.
                // This is only relevant for PV nodes, because all other nodes are searched with a null window anyway.
                // This is also necessary to ensure that the PV doesn't get truncated, because otherwise there could be nodes in
                // the PV that were not searched as PV nodes. So we make sure we're researching in PV nodes with beta == alpha + 1.
                if pv_node && child_beta - child_alpha == Score(1) && score > alpha {
                    score = -self.negamax(
                        &new_pos,
                        ply + 1,
                        child_depth - cc::third_search_reduction(),
                        -beta,
                        -alpha,
                        Exact,
                        None,
                    )?;
                }
            }

            self.undo_move();

            #[cfg(debug_assertions)]
            debug_assert_eq!(
                self.params.history.len(),
                debug_history_len,
                "depth {depth} ply {ply} old len {debug_history_len} new len {0} child {1}",
                self.params.history.len(),
                self.search_stack[ply].tried_moves.len()
            );

            if root {
                let n = self.state.uci_nodes();
                debug_assert!(n >= nodes_before_move, "{n} {nodes_before_move} {:?}", self.params);
                self.state.custom.root_move_nodes.update(mov, n - nodes_before_move);
                let move_num = self.search_stack[0].tried_moves.len() - 1;
                if move_num < 5 && self.limit().start_time.elapsed().as_millis() >= 3000 {
                    self.send_refutation(mov, score, move_num);
                }
            }
            if pv_node && first_child && !best_move.is_null() && score <= alpha {
                // we might get an AW fail, in which case we want to print as much of the "PV" as possible, not just the first 1 or 2 moves.
                let ([.., current], [child, ..]) = self.search_stack.split_at_mut(ply + 1) else { unreachable!() };
                current.pv.extend(mov, &child.pv);
            }
            debug_assert!(score.0.abs() <= SCORE_WON.0, "score {} ply {ply}", score.0);

            best_score = best_score.max(score);

            if score <= alpha {
                continue;
            }
            // We've raised alpha. For most nodes, this results in an immediate beta cutoff because we're using a null window.
            alpha = score;
            // Only set best_move on raising `alpha` instead of `best_score` because fail low nodes should store the
            // default move, which is either the TT move (if there was a TT hit) or the null move.
            best_move = mov;

            if !tactical {
                self.search_stack[ply].killer = best_move;
            }

            // Update the PV. We only need to do this for PV nodes (we could even only do this for non-fail highs,
            // if we didn't have to worry about aw fail high). At the root, we also update the best score so that
            // an aborted search will print an up-to-date score -- this ensures we never print a mating PV without
            // a matching mate score.
            if pv_node {
                if root {
                    self.cur_pv_data_mut().score = score;
                }
                let ([.., current], [child, ..]) = self.search_stack.split_at_mut(ply + 1) else { unreachable!() };
                let cur_pv = &mut current.pv;
                cur_pv.extend(best_move, &child.pv);
                if cfg!(debug_assertions) {
                    cur_pv.assert_valid(*pos);
                    if let Some(n) = best_score.plies_until_game_over()
                        && score < beta
                    {
                        assert!(n >= (cur_pv.len() + ply) as isize, "{best_score} {ply} {depth} '{pos}' {cur_pv:?}");
                    }
                }
            }

            if score < beta {
                // We're in a PVS PV node and this move raised alpha but didn't cause a fail high, so look at the other moves.
                // PVS PV nodes are rare
                bound_so_far = Exact;
                // idea from calvin: We don't expect another move to raise alpha, so we reduce
                if child_depth >= cc::alpha_raise_reduction_min_depth() && !score.is_proven_loss() {
                    child_depth -= cc::alpha_raise_reduction();
                }
                continue;
            }
            // Beta cutoff. Update history and killer for quiet moves, then break out of the move loop.
            bound_so_far = FailHigh;
            self.update_histories(mov, depth, ply, score - beta);
            break;
        }

        // ******************************************************
        // ***** After move loop, save some info and return *****
        // ******************************************************

        if self.search_stack[ply].tried_moves.is_empty() {
            if excluded_move.is_some() {
                // We didn't look at all the moves, so don't return an incorrect checkmate score.
                // But we still want to fail low.
                debug_assert!(alpha >= MIN_NORMAL_SCORE);
                return Some(alpha);
            }
            // TODO: Test storing to the TT
            return Some(game_result_to_score(pos.no_moves_result().unwrap(), ply));
        }

        if !(root && self.current_pv_num > 0) && !in_singular_search {
            // Store the results in the TT, always replacing the previous entry. Note that the TT move is only overwritten
            // if this node was an exact or fail high node or if there was a collision.
            let tt_entry: TTEntry<Board> =
                TTEntry::new(pos.hash_pos(), best_score, raw_eval, best_move, depth, bound_so_far, self.age());

            self.tt_mut().store(tt_entry, pos.hash_pos(), ply);
        }

        // Corrhist updates
        if !(in_check
            || in_singular_search
            || (!best_move.is_null() && best_move.is_tactical(pos))
            || (best_score <= eval && bound_so_far == NodeType::lower_bound())
            || (best_score >= eval && bound_so_far == NodeType::upper_bound()))
        {
            self.state.custom.corr_hist.update(pos, continued, depth, eval, best_score);
        }
        if ply > 0 && bound_so_far == FailLow {
            // give a smaller bonus to the parent's move if we fail low. This rewards PVS researches that don't cause a fail high in the parent.
            let parent_move = self.search_stack[ply - 1].last_tried_move();
            if !parent_move.is_null() {
                self.update_histories(
                    parent_move,
                    (depth / 128) / 2 * 128, // TODO: Constant (changes bench)
                    ply - 1,
                    (alpha - best_score) / 2,
                );
            }
        }
        Some(best_score)
    }

    /// Search only "tactical" moves to quieten down the position before calling eval
    fn qsearch(&mut self, pos: &Board, mut alpha: Score, beta: Score, ply: usize, pv_node: bool) -> Option<Score> {
        // updating seldepth only in qsearch meaningfully increased performance and was even measurable in a [0, 10] SPRT.
        // TODO: That's weird, retest
        self.atomic().update_seldepth(ply);

        let in_check = pos.is_in_check();
        // The stand pat check. Since we're not looking at all moves, it's very likely that there's a move we didn't
        // look at that doesn't make our position worse, so we don't want to assume that we have to play a capture.
        let raw_eval;
        let mut eval;
        let mut bound_so_far = FailLow;

        // see main search, store an invalid null move in the TT entry if all moves failed low.
        let mut best_move = Move::default();

        let mut continued = None;
        if ply >= 2 {
            let entry = &self.state.search_stack[ply - 2];
            let mov = entry.last_tried_move();
            if !mov.is_null() {
                let piece = mov.piece_type(&entry.pos);
                continued = Some((mov, piece));
            }
        }

        if pv_node {
            self.search_stack[ply].pv.clear();
        }

        // Don't do TT cutoffs with alpha already raised by the stand pat check, because that relies on the null move observation.
        // But if there's a TT entry from normal search that's worse than the stand pat score, we should trust that more.
        let old_entry = self.tt().load::<Board>(pos, ply);
        if let Some(tt_entry) = old_entry {
            debug_assert!(tt_entry.hash_part().equals(pos.hash_pos()));
            let bound = tt_entry.bound();
            let tt_score = tt_entry.score();
            // depth 0 drops immediately to qsearch, so a depth 0 entry always comes from qsearch.
            // However, if we've already done qsearch on this position, we can just re-use the result,
            // so there is no point in checking the depth at all unless we're in a pv node: In that case, we don't want to
            // cut our PV short by returning a score from the TT without a move to back it up
            if (bound == NodeType::lower_bound() && tt_score >= beta)
                || (bound == NodeType::upper_bound() && tt_score <= alpha)
                || bound == Exact
            {
                if !(pv_node || in_check) {
                    return Some(tt_score);
                }
            }
            raw_eval = tt_entry.raw_eval();
            eval = self.state.custom.corr_hist.correct(pos, continued, raw_eval);

            // even though qsearch never checks for game over conditions, it's still possible for it to load a checkmate score
            // and propagate that up to a qsearch parent node, where it gets saved with a depth of 0, so game over scores
            // with a depth of 0 in the TT are possible
            // exact scores should have already caused a cutoff
            // TODO: Removing the `&& !tt_entry.score.is_game_over_score()` condition here and in `negamax` *failed* a
            // nonregression SPRT with `[-7, 0]` bounds even though I don't know why, and those conditions make it fail
            // the re-search test case. So the conditions are still disabled for now,
            // test reintroducing them at some point in the future after I have TT aging!
            if (bound == NodeType::lower_bound() && tt_score >= eval)
                || (bound == NodeType::upper_bound() && tt_score <= eval)
            {
                // we don't want to return mate scores unless we can prove there is a mate by also returning a matching PV
                eval = tt_score;
            };
            if let Some(mov) = tt_entry.mov(pos) {
                best_move = mov;
            }
        } else {
            raw_eval = self.eval(pos, ply);
            eval = if in_check {
                SCORE_LOST + ply as ScoreT
            } else {
                self.state.custom.corr_hist.correct(pos, continued, raw_eval)
            };
        }
        let mut best_score = eval;

        // Saving to the TT is probably unnecessary since the score is either from the TT or just the static eval,
        // which is not very valuable. Also, the fact that there's no best move might have unfortunate interactions with
        // IIR, because it will make this fail-high node appear like a fail-low node. TODO: Test regardless, but probably
        // only after aging
        if best_score >= beta || ply >= SEARCH_STACK_LEN {
            return Some(best_score);
        }

        if best_score > alpha {
            bound_so_far = Exact;
            alpha = best_score;
        }
        self.record_pos(pos, best_score, ply);

        self.maybe_send_currline(pos, alpha, beta, ply, Some(best_score));

        let mut move_picker = MovePicker::new(pos, ply, best_move, !in_check);
        let mut children_visited = 0;
        while let Some(sm) = move_picker.next(&self.state) {
            let mov = sm.mov();
            let move_score = sm.score();
            debug_assert!(mov.is_tactical(pos) || pos.is_in_check(), "{mov:?} {pos}");
            self.tt().prefetch(pos.approx_hash_after(mov));
            if !best_score.is_proven_loss() {
                if move_score < MoveScore(0) || children_visited >= cc::qsearch_lmp() {
                    // qsearch see pruning and qsearch late move  pruning (lmp):
                    // If the move has a negative SEE score or if we've already looked at enough moves, don't even bother playing it in qsearch.
                    break;
                }
                let hist_score = self.capt_hist.get(mov, pos);
                // qsearch history pruning
                if hist_score < MoveScore(-500) {
                    break;
                }
            }
            let new_pos = pos.play(mov);
            // check nodes in qsearch to allow `go nodes n` to go exactly `n` nodes. Do this check here to avoid counting
            // falling into qsearch as two nodes
            if self.count_node_and_test_stop() {
                return None;
            }
            self.record_move(mov, pos, ply, move_score);
            children_visited += 1;
            let score = -self.qsearch(&new_pos, -beta, -alpha, ply + 1, pv_node)?;
            self.undo_move();
            best_score = best_score.max(score);
            if score <= alpha {
                continue;
            }
            bound_so_far = Exact;
            alpha = score;
            best_move = mov;
            // even if the child score came from a TT entry with depth > 0, we don't trust this node any more than now
            // because we haven't looked at all nodes
            if score >= beta {
                bound_so_far = FailHigh;
                break;
            }

            if pv_node {
                // Update the PV in qsearch so that we maintain the invariant that we print a mate score iff we print a PV ending in a mate.
                let ([.., current], [child, ..]) = self.search_stack.split_at_mut(ply + 1) else { unreachable!() };
                current.pv.extend(best_move, &child.pv);
                if let Some(n) = best_score.plies_until_game_over() {
                    assert!(n >= (current.pv.len() + ply) as isize, "{best_score} {ply} '{pos}' {:?}", current.pv);
                }
            }
        }

        let tt_entry: TTEntry<Board> =
            TTEntry::new(pos.hash_pos(), best_score, raw_eval, best_move, 0, bound_so_far, self.age());
        self.tt_mut().store(tt_entry, pos.hash_pos(), ply);
        Some(best_score)
    }

    fn eval(&mut self, pos: &Board, ply: usize) -> Score {
        let us = self.params.pos.active_player();
        let mut score = if ply == 0 {
            self.eval.eval(pos, 0, us)
        } else {
            let old_pos = &self.state.search_stack[ply - 1].pos;
            let mov = self.search_stack[ply - 1].last_tried_move();
            let res = self.eval.eval_incremental(old_pos, mov, pos, ply, us);
            debug_assert_eq!(res, self.eval.eval(pos, ply, us), "{pos} {mov:?} {old_pos} {ply}");
            res
        };
        // the score must not be in the mate score range unless the position includes too many pieces
        debug_assert!(
            !score.is_won_or_lost() || UnverifiedBoard::new(*pos).verify(Strict).is_err(),
            "{score} {0} {1}, {pos}",
            score.0,
            self.eval.eval(pos, ply, us)
        );
        score = if us == pos.active_player() {
            score.wrapping_add(&self.params.contempt)
        } else {
            score.wrapping_add(&-self.params.contempt)
        };
        if let Some(res) = pos.query_bitbase(self.precomputed.bitbase) {
            // because it's not useful to return the same won/lost score on all nodes, we interpolate with the static eval
            score = match res {
                Win => (score + BITBASE_WIN) / 2,
                Lose => (score + BITBASE_LOSS) / 2,
                PlayerResult::Draw => score / 2, // also reduces contempt on bitbase results
            };
        }
        score.clamp(MIN_NORMAL_SCORE, MAX_NORMAL_SCORE)
    }

    fn update_continuation_hist(
        mov: Move,
        prev_move: Move,
        bonus: HistScoreT,
        malus: HistScoreT,
        pos: &Board,
        prev_pos: &Board,
        hist: &mut ContHist,
        failed: &[Move],
    ) {
        if prev_move == Move::default() {
            return; // Ignore NMP null moves
        }
        hist.update(mov, pos, prev_move, prev_pos, bonus);
        for disappointing in failed.iter().dropping_back(1).filter(|m| !m.is_tactical(pos)) {
            hist.update(*disappointing, pos, prev_move, prev_pos, malus);
        }
    }

    fn update_histories(&mut self, mov: Move, depth: isize, ply: usize, score_diff: Score) {
        debug_assert!(score_diff >= Score(0));
        let (before, [entry, ..]) = self.state.search_stack.split_at_mut(ply) else { unreachable!() };
        let pos = &entry.pos;
        let threats = pos.threats();
        if mov.is_tactical(pos) {
            let noisy_bonus = (depth * cc::noisy_hist_depth_bonus() / 1024 + cc::noisy_hist_bonus_offset())
                as HistScoreT
                + ((score_diff.0 + 1).ilog2() * cc::noisy_hist_bonus_eval_diff()) as HistScoreT;
            let noisy_penalty = (-depth * cc::noisy_hist_depth_penalty() / 1024 - cc::noisy_hist_penalty_offset())
                as HistScoreT
                - ((score_diff.0 + 1).ilog2() * cc::noisy_hist_penalty_eval_diff()) as HistScoreT;
            for disappointing in entry.tried_moves.iter().dropping_back(1).filter(|m| m.is_tactical(pos)) {
                self.state.custom.capt_hist.update(*disappointing, pos, noisy_penalty);
            }
            self.state.custom.capt_hist.update(mov, pos, noisy_bonus);
            return;
        }
        let bonus = (depth * cc::hist_depth_bonus() / 1024 + cc::hist_bonus_offset()) as HistScoreT
            + ((score_diff.0 + 1).ilog2() * cc::hist_bonus_eval_diff()) as HistScoreT;
        let penalty = (-depth * cc::hist_depth_penalty() / 1024 - cc::hist_penalty_offset()) as HistScoreT
            - ((score_diff.0 + 1).ilog2() * cc::hist_penalty_eval_diff()) as HistScoreT;
        let noisy_penalty = (-depth * cc::noisy_hist_depth_penalty_bm_quiet() / 1024
            - cc::noisy_hist_penalty_offset_bm_quiet()) as HistScoreT
            - ((score_diff.0 + 1).ilog2() * cc::noisy_hist_penalty_eval_diff_bm_quiet()) as HistScoreT;
        for disappointing in entry.tried_moves.iter().dropping_back(1) {
            if disappointing.is_tactical(&pos) {
                self.state.custom.capt_hist.update(*disappointing, pos, noisy_penalty);
            } else {
                self.state.custom.history.update(*disappointing, threats, penalty);
            }
        }
        self.state.custom.history.update(mov, threats, bonus);
        if ply > 0 {
            let parent = before.last_mut().unwrap();
            Self::update_continuation_hist(
                mov,
                parent.last_tried_move(),
                bonus,
                penalty,
                pos,
                &parent.pos,
                &mut self.state.custom.countermove_hist,
                &entry.tried_moves,
            );
            if ply > 1 {
                let grandparent = &mut before[before.len() - 2];
                let fmh = &mut self.state.custom.follow_up_move_hist;
                Self::update_continuation_hist(
                    mov,
                    grandparent.last_tried_move(),
                    bonus,
                    penalty,
                    pos,
                    &grandparent.pos,
                    fmh,
                    &entry.tried_moves,
                );
            }
        }
    }

    fn record_pos(&mut self, pos: &Board, eval: Score, ply: usize) {
        self.search_stack[ply].pos = *pos;
        self.search_stack[ply].eval = eval;
        self.search_stack[ply].tried_moves.clear();
    }

    fn record_move(&mut self, mov: Move, old_pos: &Board, ply: usize, move_score: MoveScore) {
        self.params.history.push(old_pos.hash_pos());
        self.search_stack[ply].tried_moves.push(mov);
        self.search_stack[ply].move_score = move_score;
    }

    // gets skipped when aborting search, but that's fine
    fn undo_move(&mut self) {
        self.params.history.pop();
    }

    #[inline]
    fn maybe_send_currline(&mut self, pos: &Board, alpha: Score, beta: Score, ply: usize, score: Option<Score>) {
        if self.uci_nodes() % DEFAULT_CHECK_TIME_INTERVAL == 0
            && ply > 0
            && self.last_msg_time.elapsed().as_millis() >= 1000
        {
            // calling qsearch instead of eval would give better results, but it would also mean that benches are no longer
            // deterministic
            let score = score.unwrap_or_else(|| self.eval(pos, ply));
            let flip = pos.active_player() != self.params.pos.active_player();
            let score = score.flip_if(flip);
            let alpha = alpha.flip_if(flip);
            let beta = beta.flip_if(flip);
            self.send_currline(ply - 1, score, alpha.min(beta), beta.max(alpha));
        }
    }
}

#[cfg(test)]
mod tests {
    use gears::games::chess::Board;
    use gears::general::board::BoardHelpers;
    use gears::general::board::Strictness::{Relaxed, Strict};
    use gears::general::moves::UntrustedMove;
    use gears::search::NodesLimit;

    use super::*;
    use crate::eval::chess::lite::{KingGambot, LiTEval};
    use crate::eval::chess::material_only::MaterialOnlyEval;
    use crate::eval::chess::piston::PistonEval;
    use crate::eval::rand_eval::RandEval;
    use crate::search::generic::gaps::Gaps;
    use crate::search::tests::generic_engine_test;
    use crate::search::tt::{Age, TT};

    #[test]
    fn mate_in_one_test() {
        let board = Board::from_fen("4k3/8/4K3/8/8/8/8/6R1 w - - 0 1", Strict).unwrap();
        // run multiple times to get different random numbers from the eval function
        for depth in 1..=3 {
            for _ in 0..42 {
                let mut engine = Caps::for_eval::<RandEval>();
                let res = engine.search_with_new_tt(board, SearchLimit::depth(DepthPly::new(depth)));
                assert!(res.score.is_proven_win());
                assert_eq!(res.score.plies_until_game_won(), Some(1));
            }
        }
    }

    #[test]
    fn mate_in_two_test() {
        let fen = "6rk/PP1PPPnp/1N1BN2P/7R/4B3/2Q5/P3KP2/6R1 w - -";
        let pos = Board::from_fen(fen, Relaxed).unwrap();
        let mut caps = Caps::for_eval::<LiTEval>();
        let res = caps.search_with_new_tt(pos, SearchLimit::depth_(3));
        assert_eq!(res.score.plies_until_game_won(), Some(3));
        let state = caps.search_state();
        let pv_data = &state.pv_data()[0];
        assert_eq!(pv_data.score, res.score);
        assert_eq!(pv_data.pv.len(), 3);
        assert_eq!(res.chosen_move, pv_data.pv.list[0]);
        assert!(state.uci_nodes() <= 500);
        assert!(!state.thread_data.shared.currently_searching.load(std::sync::atomic::Ordering::Relaxed));
        assert_eq!(state.atomic().score(), res.score);
        assert_eq!(state.atomic().iterations().isize(), 3);
    }

    #[test]
    fn simple_search_test() {
        let list = [
            ("r2q1r2/ppp1pkb1/2n1p1pp/2N1P3/2pP2Q1/2P1B2P/PP3PP1/R4RK1 b - - 1 18", -500, -100),
            ("r1bqkbnr/3n2p1/2p1pp1p/pp1p3P/P2P4/1PP1PNP1/1B3P2/RN1QKB1R w KQkq - 0 14", 90, 300),
        ];
        for (fen, min, max) in list {
            let pos = Board::from_fen(fen, Strict).unwrap();
            let mut engine = Caps::for_eval::<PistonEval>();
            let res = engine.search_with_new_tt(pos, SearchLimit::nodes(NodesLimit::new(30_000).unwrap()));
            assert!(res.score > Score(min));
            assert!(res.score < Score(max));
        }
    }

    #[test]
    fn lucena_test() {
        let pos = Board::from_name("lucena").unwrap();
        let mut engine = Caps::for_eval::<PistonEval>();
        let res = engine.search_with_new_tt(pos, SearchLimit::depth(DepthPly::new(7)));
        // TODO: More aggressive bound once the engine is stronger
        assert!(res.score >= Score(200));
    }

    #[test]
    fn philidor_test() {
        let pos = Board::from_name("philidor").unwrap();
        let mut engine = Caps::for_eval::<LiTEval>();
        let res = engine.search_with_new_tt(pos, SearchLimit::nodes(NodesLimit::new(50_000).unwrap()));
        assert!(res.score.abs() <= Score(256));
    }

    #[test]
    fn kiwipete_test() {
        let pos = Board::from_name("kiwipete").unwrap();
        let mut engine = Caps::for_eval::<LiTEval>();
        let res = engine.search_with_new_tt(pos, SearchLimit::nodes(NodesLimit::new(12_345).unwrap()));
        let score = res.score;
        assert!(score.abs() <= Score(64), "{score}");
        assert!(
            [Move::from_compact_text("e2a6", &pos).unwrap(), Move::from_compact_text("d5e6", &pos).unwrap()]
                .contains(&res.chosen_move),
            "{}",
            res.chosen_move.compact_formatter(&pos)
        );
    }

    #[test]
    fn generic_test() {
        generic_engine_test(Caps::for_eval::<LiTEval>());
        generic_engine_test(Caps::for_eval::<RandEval>());
    }

    #[test]
    fn depth_1_nodes_test() {
        let tt = TT::new_with_mib(1);
        depth_1_nodes_test_impl(&mut Caps::for_eval::<RandEval>(), Some(tt.clone()));
        depth_1_nodes_test_impl(&mut Caps::for_eval::<MaterialOnlyEval>(), Some(tt.clone()));
        depth_1_nodes_test_impl(&mut Caps::for_eval::<PistonEval>(), Some(tt.clone()));
        depth_1_nodes_test_impl(&mut Caps::for_eval::<KingGambot>(), Some(tt.clone()));
        depth_1_nodes_test_impl(&mut Caps::for_eval::<LiTEval>(), Some(tt.clone()));
        depth_1_nodes_test_impl(&mut Gaps::for_eval::<RandEval>(), None);
    }

    fn depth_1_nodes_test_impl(engine: &mut dyn Engine<Board>, tt: Option<TT>) {
        for pos in Board::bench_positions() {
            let _ = engine.search_with_tt(pos, SearchLimit::depth_(1), tt.clone().unwrap_or_default());
            if pos.has_no_legal_moves() {
                continue;
            }
            let mut root_entry = TTEntry::<Board>::default();
            if let Some(tt) = tt.clone() {
                root_entry = tt.load(&pos, 0).unwrap();
                assert!(root_entry.depth() as usize <= 2 * DEPTH_INCREMENT); // possible extensions
                assert_eq!(root_entry.bound(), Exact);
                assert!(root_entry.mov(&pos).is_some());
            }
            let moves = pos.legal_moves();
            let nodes = engine.search_state_dyn().uci_nodes() as usize;
            let num_moves = moves.len();
            assert!(nodes > num_moves, "{nodes} {num_moves} {pos}"); // > because of extensions and re-searches
            if let Some(tt) = tt.clone() {
                for m in moves {
                    let new_pos = pos.play(m);
                    let entry = tt.load::<Board>(&new_pos, 1);
                    let Some(entry) = entry else {
                        continue; // it's possible that a position is not in the TT because qsearch didn't save it
                    };
                    assert!(entry.depth() as usize <= 2 * DEPTH_INCREMENT, "{entry:?} {new_pos}");
                    assert!(
                        new_pos.has_no_legal_moves() || -entry.score <= root_entry.score,
                        "root: {root_entry:?}\nchild: {entry:?}\nroot: {pos}\nchild: {new_pos}"
                    );
                }
            }
        }
    }

    #[test]
    fn only_one_move_test() {
        let fen = "B4QRb/8/8/8/2K3P1/5k2/8/b3RRNB b - - 0 1";
        let pos = Board::from_fen(fen, Relaxed).unwrap();
        assert!(pos.debug_verify_invariants(Strict).is_err());
        let mut caps = Caps::for_eval::<PistonEval>();
        let limit = SearchLimit::per_move(Duration::from_millis(999_999_999));
        let res = caps.search_with_new_tt(pos, limit);
        assert_eq!(res.chosen_move, Move::from_compact_text("f3g3", &pos).unwrap());
        assert_eq!(caps.atomic().iterations().get(), 1);
        assert!(caps.uci_nodes() <= 1000); // might be a bit more than 1 because of check extensions
    }

    #[test]
    fn mate_research_test() {
        let pos = Board::from_fen("k7/3B4/4N3/K7/8/8/8/8 w - - 16 9", Strict).unwrap();
        let mut caps = Caps::for_eval::<LiTEval>();
        let limit = SearchLimit::mate_in_moves(5);
        let res = caps.search_with_new_tt(pos, limit);
        assert!(res.score.is_proven_win());
        let nodes = caps.search_state().uci_nodes();
        let tt = caps.search_state().tt().clone();
        // Don't clear the internal state
        let second_search = caps.search_with_tt(pos, limit, tt.clone());
        assert!(second_search.score.is_proven_win());
        let second_search_nodes = caps.search_state().uci_nodes();
        assert!(second_search_nodes < nodes, "{second_search_nodes} {nodes}");
        let d3 = SearchLimit::depth(DepthPly::new(3));
        let d3_search = caps.search_with_tt(pos, d3, tt.clone());
        // at depth 3, we can't print a pv that ends in a mate, so we don't return a mate score
        assert!(!d3_search.score.is_proven_win(), "{}", d3_search.score.0);
        let d3_nodes = caps.search_state().uci_nodes();
        caps.forget();
        assert_eq!(caps.search_state().uci_nodes(), 0);
        let fresh_d3_search = caps.search_with_new_tt(pos, d3);
        assert!(!fresh_d3_search.score.is_won_or_lost(), "{}", fresh_d3_search.score.0);
        assert!(fresh_d3_search.score < MAX_NORMAL_SCORE, "{}", fresh_d3_search.score.0);
        let fresh_d3_nodes = caps.search_state().uci_nodes();
        assert!(fresh_d3_nodes > d3_nodes, "{fresh_d3_nodes} {d3_nodes}");
        caps.forget();
        _ = caps.search_with_new_tt(pos, d3);
        assert_eq!(caps.search_state().uci_nodes(), fresh_d3_nodes);
    }

    #[test]
    fn move_order_test() {
        let fen = "7k/8/8/8/p7/1p6/1R1r4/K7 w - - 4 3";
        let pos = Board::from_fen(fen, Relaxed).unwrap();
        let tt_move = Move::from_text("a1b1", &pos).unwrap();
        let tt = TT::default();
        let entry = TTEntry::new(pos.hash_pos(), Score(0), Score(-12), tt_move, 123, Exact, Age::default());
        tt.store::<Board>(entry, pos.hash_pos(), 0);
        let threats = pos.threats();
        let mut caps = Caps::default();
        let killer = Move::from_text("b2c2", &pos).unwrap();
        caps.search_stack[0].killer = killer;
        let hist_move = Move::from_text("b2b1", &pos).unwrap();
        caps.history.update(hist_move, threats, 1000);
        let bad_quiet = Move::from_text("b2a2", &pos).unwrap();
        caps.history.update(bad_quiet, threats, -1);
        let bad_capture = Move::from_text("b2b3", &pos).unwrap();
        caps.capt_hist.update(bad_capture, &pos, 100);

        let mut move_picker = MovePicker::new(&pos, 0, tt_move, false);
        let mut moves = vec![];
        let mut scores = vec![];
        while let Some(sm) = move_picker.next(&caps.state) {
            moves.push(sm.mov());
            scores.push(sm.score());
        }
        assert_eq!(moves.len(), 6);
        assert!(scores.is_sorted_by(|a, b| a > b), "{scores:?} {moves:?} {pos}");
        assert_eq!(scores[0], MoveScore::MAX);
        assert_eq!(moves[0], tt_move);
        let good_capture = Move::from_text("b2d2", &pos).unwrap();
        assert_eq!(moves[1], good_capture);
        assert_eq!(moves[2], killer);
        assert_eq!(moves[3], hist_move);
        assert_eq!(moves[4], bad_quiet);
        assert_eq!(moves[5], bad_capture);
        let search_res = caps.search_with_tt(pos, SearchLimit::depth_(1), tt.clone());
        assert_eq!(search_res.chosen_move, good_capture);
        assert!(search_res.score > Score(0));
        let tt_entry = tt.load::<Board>(&pos, 0).unwrap();
        assert_eq!(tt_entry.score, search_res.score.compact());
        assert_eq!(tt_entry.move_untrusted(), UntrustedMove::from_move(good_capture));
    }

    #[test]
    #[cfg(not(debug_assertions))]
    /// puzzles that are reasonably challenging for most humans, but shouldn't be too difficult for the engine
    fn mate_test() {
        use gears::general::moves::ExtendedFormat::Standard;
        let fens = [
            ("8/5K2/4N2k/2B5/5pP1/1np2n2/1p6/r2R4 w - - 0 1", "d1d5", 5),
            ("5rk1/r5p1/2b2p2/3q1N2/6Q1/3B2P1/5P2/6KR w - - 0 1", "f5h6", 5),
            ("2rk2nr/R1pnp3/5b2/5P2/BpPN1Q2/pPq5/P7/1K4R1 w - - 0 1", "f4c7", 6),
            ("k2r3r/PR6/1K6/3R4/8/5np1/B6p/8 w - - 0 1", "d5d8", 6),
            ("3n3R/8/3p1pp1/r2bk3/8/4NPP1/p3P1KP/1r1R4 w - - 0 1", "h8e8", 6),
            ("7K/k7/p1R5/4N1q1/8/6rb/5r2/1R6 w - - 0 1", "c6c7", 4),
            ("rkr5/3n1p2/1pp1b3/NP4p1/3PPn1p/QN1B1Pq1/2P5/R6K w - - 0 1", "a5c6", 7),
            ("1kr5/4R3/pP6/1n2N3/3p4/2p5/1r6/4K2R w K - 0 1", "h1h8", 7),
            ("1k6/1bpQN3/1p6/p7/6p1/2NP1nP1/5PK1/4q3 w - - 0 1", "d7d8", 8),
            ("1k4r1/pb1p4/1p1P4/1P3r1p/1N2Q3/6Pq/4BP1P/4R1K1 w - - 0 1", "b4a6", 10),
            ("rk6/p1r3p1/P3B1Kp/1p2B3/8/8/8/8 w - - 0 1", "e6d7", 5),
        ];
        for (fen, best_move, num_moves) in fens {
            let pos = Board::from_fen(fen, Relaxed).unwrap();
            let mut engine = Caps::for_eval::<LiTEval>();
            let limit = SearchLimit::mate_in_moves(num_moves);
            let res = engine.search_with_new_tt(pos, limit);
            let score = res.score;
            println!(
                "chosen move {0}, fen {1}, iters {2} seldepth {3}, time {4}ms",
                res.chosen_move.extended_formatter(&pos, Standard, None),
                pos.as_fen(),
                engine.iterations(),
                engine.seldepth(),
                engine.start_time().elapsed().as_millis()
            );
            assert!(score.is_proven_win());
            assert_eq!(res.chosen_move.compact_formatter(&pos).to_string(), best_move);
        }
    }
}
