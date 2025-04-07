use std::cmp::min;
use std::mem::take;
use std::sync::atomic::Ordering::Relaxed;
use std::time::{Duration, Instant};

use crate::eval::Eval;
use crate::eval::chess::lite::LiTEval;
use crate::search::chess::caps_values::cc;
use crate::search::chess::histories::{
    CaptHist, ContHist, CorrHist, HIST_DIVISOR, HistScoreT, HistoryHeuristic, write_single_hist_table,
};
use crate::search::move_picker::MovePicker;
use crate::search::statistics::SearchType;
use crate::search::statistics::SearchType::{MainSearch, Qsearch};
use crate::search::tt::TTEntry;
use crate::search::*;
use gears::PlayerResult::{Lose, Win};
use gears::arrayvec::ArrayVec;
use gears::games::chess::moves::ChessMove;
use gears::games::chess::pieces::ChessPieceType::Pawn;
use gears::games::chess::see::SeeScore;
use gears::games::chess::squares::NUM_SQUARES;
use gears::games::chess::{ChessColor, Chessboard, MAX_CHESS_MOVES_IN_POS, unverified::UnverifiedChessboard};
use gears::games::{BoardHistory, PosHash, ZobristHistory, n_fold_repetition};
use gears::general::bitboards::RawBitboard;
use gears::general::board::Strictness::Strict;
use gears::general::board::{BitboardBoard, UnverifiedBoard};
use gears::general::common::Description::NoDescription;
use gears::general::common::{Res, StaticallyNamedEntity, parse_bool_from_str, parse_int_from_str, select_name_static};
use gears::general::move_list::EagerNonAllocMoveList;
use gears::general::moves::Move;
use gears::itertools::Itertools;
use gears::output::Message::Debug;
use gears::score::{
    MAX_BETA, MAX_NORMAL_SCORE, MAX_SCORE_LOST, MIN_ALPHA, MIN_NORMAL_SCORE, NO_SCORE_YET, SCORE_LOST, ScoreT,
    game_result_to_score,
};
use gears::search::NodeType::*;
use gears::search::*;
use gears::ugi::EngineOptionName::*;
use gears::ugi::EngineOptionType::Check;
use gears::ugi::{EngineOption, EngineOptionName, EngineOptionType, UgiCheck};

/// The maximum value of the `depth` parameter, i.e. the maximum number of Iterative Deepening iterations.
const DEPTH_SOFT_LIMIT: Depth = Depth::new(225);
/// The maximum value of the `ply` parameter in main search, i.e. the maximum depth (in plies) before qsearch is reached
const DEPTH_HARD_LIMIT: Depth = Depth::new(255);

/// Qsearch can go more than 30 plies deeper than the depth hard limit if ther's more material on the board; in that case we simply
/// return the static eval.
const SEARCH_STACK_LEN: usize = DEPTH_HARD_LIMIT.get() + 30;

/// The TT move and good captures have a higher score, all other moves have a lower score.
const KILLER_SCORE: MoveScore = MoveScore(8 * HIST_DIVISOR);

#[derive(Debug, Clone)]
struct RootMoveNodes(Box<[[u64; NUM_SQUARES]; NUM_SQUARES]>);

impl Default for RootMoveNodes {
    fn default() -> Self {
        RootMoveNodes(Box::new([[0; NUM_SQUARES]; NUM_SQUARES]))
    }
}

impl RootMoveNodes {
    fn clear(&mut self) {
        for elem in self.0.iter_mut() {
            *elem = [0; NUM_SQUARES];
        }
    }
    fn update(&mut self, mov: ChessMove, nodes: u64) {
        self.0[mov.src_square().bb_idx()][mov.dest_square().bb_idx()] += nodes;
    }

    fn frac_1024(&self, best_move: ChessMove, total_nodes: u64) -> u64 {
        self.0[best_move.src_square().bb_idx()][best_move.dest_square().bb_idx()] * 1024 / total_nodes
    }
}

#[derive(Debug, Clone, Default)]
pub struct CapsCustomInfo {
    history: HistoryHeuristic,
    /// Many moves have a "natural" response, so use that for move ordering:
    /// Instead of only learning which quiet moves are good, learn which quiet moves are good after our
    /// opponent played a given move.
    countermove_hist: ContHist,
    /// Often, a move works because it is immediately followed by some other move, which completes the tactic.
    /// Keep track of such quiet follow-up moves. This is exactly the same as the countermove history, but considers
    /// our previous move instead of the opponent's previous move, i.e. the move 2 plies ago instead of 1 ply ago.
    follow_up_move_hist: ContHist,
    capt_hist: CaptHist,
    corr_hist: CorrHist,
    original_board_hist: ZobristHistory,
    nmp_disabled: [bool; 2],
    depth_hard_limit: usize,
    root_move_nodes: RootMoveNodes,
}

impl CapsCustomInfo {
    fn nmp_disabled_for(&mut self, color: ChessColor) -> &mut bool {
        &mut self.nmp_disabled[color]
    }
}

impl CustomInfo<Chessboard> for CapsCustomInfo {
    fn new_search(&mut self) {
        debug_assert!(!self.nmp_disabled[0]);
        debug_assert!(!self.nmp_disabled[1]);
        // don't update history values, malus and gravity already take care of that
        self.root_move_nodes.clear();
    }

    fn hard_forget_except_tt(&mut self) {
        for value in self.history.iter_mut().flatten() {
            *value = 0;
        }
        self.capt_hist.reset();
        for value in self.countermove_hist.iter_mut() {
            *value = 0;
        }
        for value in self.follow_up_move_hist.iter_mut() {
            *value = 0;
        }
        self.corr_hist.reset();
        self.root_move_nodes.clear();
    }

    fn write_internal_info(&self, pos: &Chessboard) -> Option<String> {
        Some(
            write_single_hist_table(&self.history, pos, false)
                + "\n"
                + &write_single_hist_table(&self.history, pos, true),
        )
    }
}

#[derive(Debug, Default, Clone)]
pub struct CapsSearchStackEntry {
    killer: ChessMove,
    pv: Pv<Chessboard, SEARCH_STACK_LEN>,
    tried_moves: ArrayVec<ChessMove, MAX_CHESS_MOVES_IN_POS>,
    pos: Chessboard,
    eval: Score,
}

impl SearchStackEntry<Chessboard> for CapsSearchStackEntry {
    fn pv(&self) -> Option<&[ChessMove]> {
        Some(self.pv.list.as_slice())
    }

    fn last_played_move(&self) -> Option<ChessMove> {
        self.tried_moves.last().copied()
    }
}

impl CapsSearchStackEntry {
    /// If this entry has a lower ply number than the current node, this is the tree edge that leads towards the current node.
    fn last_tried_move(&self) -> ChessMove {
        *self.tried_moves.last().unwrap()
    }
}

type CapsState = SearchState<Chessboard, CapsSearchStackEntry, CapsCustomInfo>;

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
        Self::with_eval(Box::new(DefaultEval::default()))
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

impl Engine<Chessboard> for Caps {
    type SearchStackEntry = CapsSearchStackEntry;
    type CustomInfo = CapsCustomInfo;

    fn with_eval(eval: Box<dyn Eval<Chessboard>>) -> Self {
        Self { state: SearchState::new(Depth::new(SEARCH_STACK_LEN)), eval }
    }

    fn static_eval(&mut self, pos: &Chessboard, ply: usize) -> Score {
        self.eval.eval(&pos, ply, self.params.pos.active_player())
    }

    fn max_bench_depth(&self) -> Depth {
        DEPTH_SOFT_LIMIT
    }

    fn search_state_dyn(&self) -> &dyn AbstractSearchState<Chessboard> {
        &self.state
    }

    fn search_state_mut_dyn(&mut self) -> &mut dyn AbstractSearchState<Chessboard> {
        &mut self.state
    }

    fn engine_info(&self) -> EngineInfo {
        let mut options = vec![EngineOption {
            name: Other("UCI_Chess960".to_string()),
            value: Check(UgiCheck { val: true, default: Some(true) }),
        }];
        options.append(&mut cc::ugi_options());
        EngineInfo::new(
            self,
            self.eval.as_ref(),
            "0.1.0",
            Depth::new(15),
            NodesLimit::new(20_000).unwrap(),
            None,
            options,
        )
    }

    fn set_option(&mut self, option: EngineOptionName, old_value: &mut EngineOptionType, value: String) -> Res<()> {
        let name = option.name().to_string();
        if let Other(name) = &option {
            if name.eq_ignore_ascii_case("uci_chess960") {
                let Check(check) = old_value else { unreachable!() };
                let value = parse_bool_from_str(&value, "UCI_Chess960")?;
                check.val = value;
                return Ok(());
            }
            if let Ok(val) = parse_int_from_str(&value, "spsa option value") {
                if let Ok(()) = cc::set_value(name, val) {
                    return Ok(());
                }
            }
        }
        select_name_static(&name, self.engine_info().additional_options().iter(), "uci option", "chess", NoDescription)
            .map(|_| {}) // only called to produce an error message
    }

    fn print_spsa_params(&self) {
        for line in cc::ob_param_string() {
            println!("{line}");
        }
    }

    fn set_eval(&mut self, eval: Box<dyn Eval<Chessboard>>) {
        self.eval = eval;
    }

    fn do_search(&mut self) -> SearchResult<Chessboard> {
        let mut limit = self.params.limit;
        let pos = self.params.pos;
        limit.fixed_time = min(limit.fixed_time, limit.tc.remaining);
        self.depth_hard_limit = if limit.mate.get() == 0 { DEPTH_HARD_LIMIT.get() } else { limit.mate.get() };
        let soft_limit =
            limit.tc.remaining.saturating_sub(limit.tc.increment) / cc::soft_limit_div() + limit.tc.increment;
        self.params.limit = limit;

        // Ideally, this would only evaluate the arguments if debug is on, but that's annoying to implement
        // and would still require synchronization because debug mode might be turned on while the engine is searching.
        // Fortunately, `format_args!` avoids heap allocations.
        self.state.send_non_ugi(Debug, &format_args!(
            "Starting search with limit {time}ms, {incr}ms increment, max {fixed}ms, mate in {mate} plies, max depth {depth}, \
            max {nodes} nodes, soft limit {soft}ms, {ignored} ignored moves",
            time = limit.tc.remaining.as_millis(),
            incr = limit.tc.increment.as_millis(),
            mate = limit.mate.get(),
            depth = limit.depth.get(),
            nodes = limit.nodes.get(),
            fixed = limit.fixed_time.as_millis(),
            soft = soft_limit.as_millis(),
            ignored = self.excluded_moves.len(),
        ));
        // Use 3fold repetition detection for positions before and including the root node and 2fold for positions during search.
        self.original_board_hist = take(&mut self.search_params_mut().history);
        self.original_board_hist.push(pos.hash_pos());

        let depth = self.iterative_deepening(pos, soft_limit);
        if depth.is_some() {
            // send one final search info, but don't send empty PVs
            let mut pv = self.current_mpv_pv();
            if pv.is_empty() {
                // if we didn't finish looking at the PV, use the PV from the last iteration
                pv = self.cur_pv_data().pv.list.as_slice();
            }
            if !pv.is_empty() {
                self.search_state().send_search_info();
            }
        }
        self.search_result()
    }
}

impl NormalEngine<Chessboard> for Caps {
    fn search_state(&self) -> &SearchStateFor<Chessboard, Self> {
        &self.state
    }

    fn search_state_mut(&mut self) -> &mut SearchStateFor<Chessboard, Self> {
        &mut self.state
    }

    fn time_up(&self, tc: TimeControl, fixed_time: Duration, start_time: Instant) -> bool {
        debug_assert!(self.uci_nodes() % DEFAULT_CHECK_TIME_INTERVAL == 0);
        let elapsed = start_time.elapsed();
        // TODO: Compute at the start of the search instead of every time:
        // Instead of storing a SearchLimit, store a different struct that contains soft and hard bounds
        let hard = (tc.remaining.saturating_sub(tc.increment)) * cc::inv_hard_limit_div() as u32 / 1024 + tc.increment;
        // Because fixed_time has been clamped to at most tc.remaining, this can never lead to timeouts
        // (assuming the move overhead is set correctly)
        elapsed >= fixed_time.min(hard)
    }
}

#[allow(clippy::too_many_arguments)]
impl Caps {
    fn prefetch(&self) -> impl Fn(PosHash) + '_ {
        |hash| self.tt().prefetch(hash)
    }

    /// Iterative Deepening (ID): Do a depth 1 search, then a depth 2 search, then a depth 3 search, etc.
    /// This has two advantages: It allows the search to be stopped at any time, and it actually improves strength:
    /// The low-depth searches fill the TT and various heuristics, which improves move ordering and therefore results in
    /// better moves within the same time or nodes budget because the lower-depth searches are comparatively cheap.
    fn iterative_deepening(&mut self, pos: Chessboard, soft_limit: Duration) -> Option<isize> {
        let max_depth = DEPTH_SOFT_LIMIT.min(self.limit().depth).isize();
        let multi_pv = self.multi_pv();
        let mut soft_limit_scale = 1.0;

        self.multi_pvs.resize(multi_pv, PVData::default());
        let mut chosen_at_depth = EagerNonAllocMoveList::<Chessboard, { DEPTH_SOFT_LIMIT.get() }>::default();

        for depth in 1..=max_depth {
            self.statistics.next_id_iteration();
            for pv_num in 0..multi_pv {
                self.current_pv_num = pv_num;
                self.cur_pv_data_mut().bound = None;
                let scaled_soft_limit = soft_limit.mul_f64(soft_limit_scale);
                let (keep_searching, depth, score) = self.aspiration(pos, scaled_soft_limit, depth, max_depth);

                let atomic = &self.state.params.atomic;
                let pv = &self.state.search_stack[0].pv;

                if !pv.is_empty() {
                    if self.current_pv_num == 0 {
                        let chosen_move = pv.get(0).unwrap();
                        let ponder_move = pv.get(1);
                        atomic.set_best_move(chosen_move);
                        atomic.set_ponder_move(ponder_move);
                    }
                    self.state.multi_pvs[self.state.current_pv_num].pv.assign_from(pv);
                    // We can't really trust FailHigh scores. Even though we should still prefer a fail high move, we don't
                    // want a mate limit condition to trigger, so we clamp the fail high score to MAX_NORMAL_SCORE.
                    if let Some(score) = score {
                        debug_assert!(score.is_valid());
                        if pv_num == 0 {
                            atomic.set_score(score);
                        } else {
                            _ = atomic.get_score_t().fetch_max(score.0, Relaxed);
                        }
                    }
                }

                if !keep_searching {
                    return depth;
                }
                if let Some(chosen_move) = self.search_stack[0].pv.get(0) {
                    self.excluded_moves.push(chosen_move);
                }
            }
            self.state.excluded_moves.truncate(self.excluded_moves.len() - multi_pv);
            let chosen = self.best_move();
            chosen_at_depth.push(chosen);
            if depth >= cc::move_stability_min_depth()
                && !is_duration_infinite(soft_limit)
                && chosen_at_depth
                    .iter()
                    .dropping(depth as usize / cc::move_stability_start_div())
                    .all(|m| *m == chosen)
            {
                soft_limit_scale = cc::move_stability_factor() as f64 / 1000.0;
            } else {
                soft_limit_scale = 1.0;
            }
        }
        None
    }

    /// Aspiration Windows (AW): Assume that the score will be close to the score from the previous iteration
    /// of Iterative Deepening, so use alpha, beta bounds around that score to prune more aggressively.
    /// This means that it's possible for the root to fail low (or high), which is always something to consider:
    /// For example, the best move is not trustworthy if the root failed low (but because the TT move is ordered first,
    /// and the TT move at the root is always `state.best_move` (there can be no collisions because it's written to last),
    /// it is still trustworthy except for depth 1 or when doing a multipv search)
    fn aspiration(
        &mut self,
        pos: Chessboard,
        unscaled_soft_limit: Duration,
        depth: isize,
        max_depth: isize,
    ) -> (bool, Option<isize>, Option<Score>) {
        let mut soft_limit_fail_low_extension = 1.0;
        let mut aw_depth = depth;
        loop {
            let alpha = self.cur_pv_data().alpha;
            let beta = self.cur_pv_data().beta;
            let mut window_radius = self.cur_pv_data().radius;
            let mut soft_limit = unscaled_soft_limit.mul_f64(soft_limit_fail_low_extension);
            soft_limit_fail_low_extension = 1.0;
            if depth > 8 && self.multi_pvs.len() == 1 {
                let node_frac = self.root_move_nodes.frac_1024(self.cur_pv_data().pv.list[0], self.uci_nodes());
                soft_limit = soft_limit
                    .mul_f64(((1024 + 512 - node_frac) * cc::soft_limit_node_scale()) as f64 / (1024.0 * 1024.0));
            }
            let limit = self.params.limit.tc;
            let soft_limit = soft_limit.min(
                (limit.remaining.saturating_sub(limit.increment)) * cc::inv_soft_limit_div_clamp() / 1024
                    + limit.increment,
            );
            if self.should_not_start_negamax(soft_limit, self.limit().soft_nodes.get(), max_depth, self.limit().mate) {
                self.statistics.soft_limit_stop();
                return (false, None, None);
            }
            self.atomic().set_depth(depth); // set depth now so that an immediate stop doesn't increment the depth
            if self.atomic().count_node() >= self.limit().nodes.get() {
                return (false, Some(depth - 1), None);
            }
            let asp_start_time = Instant::now();
            let Some(pv_score) = self.negamax(pos, 0, aw_depth, alpha, beta, Exact) else {
                return (false, Some(depth), None);
            };

            self.state.send_non_ugi(
                Debug,
                &format_args!(
                    "depth {depth}, score {0}, radius {1}, interval ({2}, {3}) nodes {4}",
                    pv_score.0,
                    window_radius.0,
                    alpha.0,
                    beta.0,
                    self.uci_nodes()
                ),
            );

            let node_type = pv_score.node_type(alpha, beta);

            // we don't trust the best move in fail low nodes, but we still want to display an updated score
            self.cur_pv_data_mut().score = pv_score;
            self.cur_pv_data_mut().bound = Some(node_type);
            if node_type == FailLow {
                // In a fail low node, we didn't get any new information, and it's possible that we just discovered
                // a problem with our chosen move. So increase the soft limit such that we can gather more information.
                soft_limit_fail_low_extension = cc::soft_limit_fail_low_factor() as f64 / 1000.0;
                aw_depth = depth;
            } else if node_type == FailHigh && depth >= 8 {
                // If the search discovers an unexpectedly good move, it can take a long while to search it because the TT isn't filled
                // and because even with fail soft, scores tend to fall close to the aspiration window. So reduce the depth to speed this up.
                aw_depth = (aw_depth - 1).max(depth - 2);
            }

            if cfg!(debug_assertions) {
                let pv = &self.search_stack[0].pv;
                if pos.player_result_slow(&self.params.history).is_some() {
                    assert_eq!(pv.len(), 0);
                } else {
                    match node_type {
                        FailHigh => debug_assert_eq!(pv.len(), 1, "{pos} {node_type}"),
                        Exact => debug_assert!(
                            // currently, it's possible to reduce the PV through IIR when the TT entry of a PV node gets overwritten,
                            // but that should be relatively rare. In the future, a better replacement policy might make this actually sound
                            self.multi_pv() > 1
                                || pv.len() + pv.len() / 4 + 5 >= self.depth_hard_limit.min(aw_depth as usize)
                                || pv_score.is_won_lost_or_draw_score(),
                            "{aw_depth} {depth} {0} {pv_score} {1}",
                            pv.len(),
                            self.uci_nodes()
                        ),
                        // We don't clear the PV on a fail low node so that we can still send a useful info
                        FailLow => {
                            debug_assert_eq!(0, pv.len());
                        }
                    }
                }
                // assert this now because this doesn't hold for incomplete iterations
                debug_assert!(
                    !pv_score.is_won_or_lost() || pv_score.plies_until_game_over().unwrap() <= 500,
                    "{pv_score}"
                );
            }

            self.statistics.aw_node_type(node_type);
            if node_type == Exact {
                window_radius = Score((window_radius.0 + cc::aw_exact_add()) / cc::aw_exact_div());
            } else {
                let delta = pv_score.0.abs_diff(alpha.0);
                let delta = delta.min(pv_score.0.abs_diff(beta.0));
                let delta = delta.min(cc::aw_delta_max()) as i32;
                window_radius.0 = SCORE_WON.0.min(window_radius.0 * cc::aw_widening_factor() + delta);
            }
            self.cur_pv_data_mut().radius = window_radius;
            self.cur_pv_data_mut().alpha = (pv_score - window_radius).max(MIN_ALPHA);
            self.cur_pv_data_mut().beta = (pv_score + window_radius).min(MAX_BETA);

            if node_type == Exact {
                self.send_search_info();
                return (true, Some(depth), Some(pv_score));
            } else if asp_start_time.elapsed().as_millis() >= 1000 {
                self.send_search_info();
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
        pos: Chessboard,
        ply: usize,
        mut depth: isize,
        mut alpha: Score,
        mut beta: Score,
        mut expected_node_type: NodeType,
    ) -> Option<Score> {
        debug_assert!(alpha < beta, "{alpha} {beta} {pos} {ply} {depth}");
        debug_assert!(ply <= DEPTH_HARD_LIMIT.get(), "{ply} {depth} {pos}");
        debug_assert!(depth <= DEPTH_SOFT_LIMIT.isize(), "{ply} {depth} {pos}");
        debug_assert!(self.params.history.len() >= ply, "{ply} {depth} {pos}, {:?}", self.params.history);
        self.statistics.count_node_started(MainSearch);

        let root = ply == 0;
        let is_pv_node = expected_node_type == Exact; // TODO: Make this a generic argument of search?
        debug_assert!(!root || is_pv_node); // root implies pv node
        debug_assert!(alpha + 1 == beta || is_pv_node); // alpha + 1 < beta implies Exact node
        if is_pv_node {
            self.search_stack[ply].pv.clear();
        }

        // Always search all children at the root, even for draws or if a search limit has been reached
        if !root {
            // Mate Distance Pruning (MDP): If we've already found a mate in n, don't bother looking for longer mates.
            // This isn't intended to gain elo (since it only works in positions that are already won or lost)
            // but makes the engine better at finding shorter checkmates. Don't do MDP at the root because that can prevent us
            // from ever returning exact scores, since for a mate in 1 the score would always be exactly `beta`.
            if self.current_pv_num == 0 {
                alpha = alpha.max(game_result_to_score(Lose, ply));
                beta = beta.min(game_result_to_score(Win, ply + 1));
                if alpha >= beta {
                    return Some(alpha);
                }
            }

            let ply_100_ctr = pos.ply_draw_clock();

            if pos.is_50mr_draw()
                || pos.has_insufficient_material()
                || n_fold_repetition(2, &self.params.history, pos.hash_pos(), ply_100_ctr)
                || n_fold_repetition(3, &self.original_board_hist, pos.hash_pos(), ply_100_ctr.saturating_sub(ply))
            {
                return Some(Score(0));
            }

            if self.should_stop() {
                return None;
            }
        }

        let in_check = pos.is_in_check();
        // Check extensions. Increase the depth by 1 if in check.
        // Do this before deciding whether to drop into qsearch.
        if in_check {
            self.statistics.in_check();
            depth += 1;
        }
        // limit.mate() is the min of the original limit.mate and DEPTH_HARD_LIMIT
        if depth <= 0 || ply >= self.depth_hard_limit {
            return self.qsearch(pos, alpha, beta, ply);
        }
        let can_prune = !is_pv_node && !in_check;

        let mut best_score = NO_SCORE_YET;
        let mut bound_so_far = FailLow;

        // ************************
        // ***** Probe the TT *****
        // ************************

        // If we didn't get a move from the TT and there's no best_move to store because the node failed low,
        // store a null move in the TT. This helps IIR.
        let mut best_move = ChessMove::default();
        // Don't initialize eval just yet to save work in case we get a TT cutoff
        let raw_eval;
        let mut eval;
        // the TT entry at the root is useless when doing an actual multipv search
        let ignore_tt_entry = root && self.multi_pvs.len() > 1;
        let old_entry = self.tt().load::<Chessboard>(pos.hash_pos(), ply);
        if let Some(tt_entry) = old_entry {
            if ignore_tt_entry {
                raw_eval = tt_entry.raw_eval(); // can still use the saved raw eval
                eval = raw_eval;
            } else {
                let tt_bound = tt_entry.bound();
                debug_assert_eq!(tt_entry.hash, pos.hash_pos());

                if let Some(tt_move) = tt_entry.mov.check_pseudolegal(&pos) {
                    best_move = tt_move;
                }
                let tt_score = tt_entry.score();
                // TT cutoffs. If we've already seen this position, and the TT entry has more valuable information (higher depth),
                // and we're not a PV node, and the saved score is either exact or at least known to be outside (alpha, beta),
                // simply return it.
                if !is_pv_node && tt_entry.depth as isize >= depth {
                    if (tt_score >= beta && tt_bound == NodeType::lower_bound())
                        || (tt_score <= alpha && tt_bound == NodeType::upper_bound())
                        || tt_bound == Exact
                    {
                        self.statistics.tt_cutoff(MainSearch, tt_bound);
                        // Idea from stormphrax
                        if tt_score >= beta && !best_move.is_null() && !best_move.is_tactical(&pos) {
                            self.update_histories_and_killer(&pos, best_move, depth, ply);
                        }
                        return Some(tt_score);
                    } else if depth <= 6 {
                        // also from stormphrax
                        depth += 1;
                    }
                }
                // Even though we didn't get a cutoff from the TT, we can still use the score and bound to update our guess
                // at what the type of this node is going to be.
                if !is_pv_node {
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
                eval = raw_eval;
                // The TT score is backed by a search, so it should be more trustworthy than a simple call to static eval.
                // Note that the TT score may be a mate score, so `eval` can also be a mate score. This doesn't currently
                // create any problems, but should be kept in mind.
                if tt_bound == Exact
                    || (tt_bound == NodeType::lower_bound() && tt_score >= raw_eval)
                    || (tt_bound == NodeType::upper_bound() && tt_score <= raw_eval)
                {
                    eval = tt_score;
                }
            }
        } else {
            self.statistics.tt_miss(MainSearch);
            raw_eval = self.eval(&pos, ply);
            eval = raw_eval;
        };
        eval = self.corr_hist.correct(&pos, eval);

        self.record_pos(pos, eval, ply);

        // If the current position is noisy, we want to be more conservative with margins.
        // However, captures and promos are generally good moves, so if our eval is the static eval instead of adjusted from the TT,
        // a noisy condition would mean we're doing even better than expected. // TODO: Apply noisy for RFP etc only if eval is TT eval?
        // If it's from the TT, however, and the first move didn't produce a beta cutoff, we're probably worse than expected
        let is_noisy = in_check || (best_move != ChessMove::default() && best_move.is_tactical(&pos));

        // Like the commonly used `improving` and `regressing`, these variables compare the current static eval with
        // the static eval 2 plies ago to recognize blunders. Conceptually, `improving` and `regressing` can be seen as
        // a prediction for how the eval is going to evolve, while these variables are more about cutting early after bad moves.
        let they_blundered = ply >= 2 && eval - self.search_stack[ply - 2].eval > Score(cc::they_blundered_threshold());
        let we_blundered = ply >= 2 && eval - self.search_stack[ply - 2].eval < Score(cc::we_blundered_threshold());

        // *********************************************************
        // ***** Pre-move loop pruning (other than TT cutoffs) *****
        // *********************************************************

        if can_prune {
            // RFP (Reverse Futility Pruning): If eval is far above beta, it's likely that our opponent
            // blundered in a previous move of the search, so if the depth is low, don't even bother searching further.
            // Use `they_blundered` to better distinguish between blunders by our opponent and a generally good static eval
            // relative to `beta` --  there may be other positional factors that aren't being reflected by the static eval,
            // (like imminent threats) so don't prune too aggressively if our opponent hasn't blundered.
            // Be more careful about pruning too aggressively if the node is expected to fail low -- we should not rfp
            // a true fail low node, but our expectation may also be wrong.
            let mut margin = (cc::rfp_base() - (ScoreT::from(they_blundered) * cc::rfp_blunder())) * depth as ScoreT;
            if expected_node_type == FailHigh {
                margin /= cc::rfp_fail_high_div();
            }
            if let Some(entry) = old_entry {
                if entry.score() <= eval && entry.bound() == NodeType::upper_bound() {
                    margin += margin / 4;
                }
            }
            if is_noisy {
                margin *= 2;
            }

            if depth <= cc::rfp_max_depth() && eval >= beta + Score(margin) {
                return Some(eval);
            }

            // Razoring. If the position appears hopeless, drop into qsearch immediately.
            // This obviously has the potential to miss quite a few tactics, so only do this at low depths and when the
            // difference between the static eval and alpha is really large, and also not when we could miss a mate from the TT.
            if depth <= 2 && eval + Score(512 * depth as ScoreT) < alpha && !eval.is_game_lost_score() {
                let qsearch_score = self.qsearch(pos, alpha, beta, ply)?;
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
            let nmp_threshold = beta + ScoreT::from(expected_node_type == FailLow) * cc::nmp_fail_low();
            if depth >= cc::nmp_min_depth()
                && eval >= nmp_threshold
                && !*self.nmp_disabled_for(pos.active_player())
                && has_nonpawns
            {
                // `make_nullmove` resets the 50mr counter, so we don't consider positions after a nullmove as repetitions,
                // but we can still get TT cutoffs
                self.params.history.push(pos.hash_pos());
                let new_pos = pos.make_nullmove().unwrap();
                // necessary to recognize the null move and to make `last_tried_move()` not panic
                self.search_stack[ply].tried_moves.push(ChessMove::default());
                let reduction = cc::nmp_base() + depth / cc::nmp_depth_div() + isize::from(they_blundered);
                // the child node is expected to fail low, leading to a fail high in this node
                let nmp_res = self.negamax(new_pos, ply + 1, depth - 1 - reduction, -beta, -beta + 1, FailLow);
                _ = self.search_stack[ply].tried_moves.pop();
                self.params.history.pop();
                let score = -nmp_res?;
                if score >= beta {
                    // For shallow depths, don't bother with doing a verification search to avoid useless re-searches,
                    // unless we'd be storing a mate score -- we really want to avoid storing unproved mates in the TT.
                    // It's possible to beat beta with a score of getting mated, so use `is_won_or_lost`
                    // instead of `is_game_won_score`
                    if depth < cc::nmp_verif_depth() && !score.is_won_or_lost() {
                        return Some(score);
                    }
                    *self.nmp_disabled_for(pos.active_player()) = true;
                    // nmp was done with `depth - 1 - reduction`, but we're not doing a null move now, so technically we
                    // should use `depth - reduction`, but using `depth - 1 - reduction` is less expensive and good enough.
                    let verification_score = self.negamax(pos, ply, depth - 1 - reduction, beta - 1, beta, FailHigh);
                    self.search_stack[ply].tried_moves.clear();
                    *self.nmp_disabled_for(pos.active_player()) = false;
                    // The verification score is more trustworthy than the nmp score.
                    if verification_score.is_none_or(|score| score >= beta) {
                        return verification_score;
                    }
                }
            }
        }

        // IIR (Internal Iterative Reductions): If we don't have a TT move, this node will likely take a long time
        // because the move ordering won't be great, so don't spend too much time on it.
        // Instead, search it with reduced depth to fill the TT entry so that we can re-search it faster the next time
        // we see this node. If there was no TT entry because the node failed low, this node probably isn't that interesting,
        // so reducing the depth also makes sense in this case.
        if depth >= cc::iir_min_depth() && best_move == ChessMove::default() {
            depth -= 1;
        }

        self.maybe_send_currline(&pos, alpha, beta, ply, None);

        // An uninteresting move is a quiet move or bad capture unless it's the TT or killer move
        // (i.e. it's every move that gets ordered after the killer). The name is a bit dramatic, the first few of those
        // can still be good candidates to explore.
        let mut num_uninteresting_visited = 0;
        debug_assert!(self.search_stack[ply].tried_moves.is_empty());

        // *************************
        // ***** The move loop *****
        // *************************

        let mut move_picker = MovePicker::<Chessboard, MAX_CHESS_MOVES_IN_POS>::new(pos, best_move, false);
        let move_scorer = CapsMoveScorer { board: pos, ply };
        while let Some((mov, move_score)) = move_picker.next(&move_scorer, self) {
            if can_prune && best_score > MAX_SCORE_LOST {
                // LMP (Late Move Pruning): Trust the move ordering and assume that moves ordered late aren't very interesting,
                // so don't even bother looking at them in the last few layers.
                // FP (Futility Pruning): If the static eval is far below alpha,
                // then it's unlikely that a quiet move can raise alpha: We've probably blundered at some prior point in search,
                // so cut our losses and return. This has the potential of missing sacrificing mate combinations, though.
                let fp_margin = if we_blundered {
                    cc::fp_blunder_base() + cc::fp_blunder_scale() * depth
                } else {
                    cc::fp_base() + cc::fp_scale() * depth
                };
                let mut lmp_threshold = if we_blundered {
                    cc::lmp_blunder_base() + cc::lmp_blunder_scale() * depth
                } else {
                    cc::lmp_base() + cc::lmp_scale() * depth
                };
                // LMP faster if we expect to fail low anyway
                if expected_node_type == FailLow {
                    lmp_threshold -= lmp_threshold / cc::lmp_fail_low_div();
                }
                if depth <= cc::max_move_loop_pruning_depth()
                    && (num_uninteresting_visited >= lmp_threshold
                        || (eval + Score(fp_margin as ScoreT) < alpha && move_score < KILLER_SCORE))
                {
                    break;
                }
                // History Pruning: At very low depth, don't play quiet moves with bad history scores. Skipping bad captures too gained elo.
                if (move_score.0 as isize) < -150 * depth && depth <= 3 {
                    break;
                }
                // PVS SEE pruning: Don't play moves with bad SEE score at low depth
                let see_threshold = -50 * depth as i32;
                if move_score < KILLER_SCORE && depth < 4 && !pos.see_at_least(mov, SeeScore(see_threshold)) {
                    continue;
                }
            }

            if root && self.excluded_moves.contains(&mov) {
                continue;
            }
            let Some(new_pos) = pos.make_move_and_prefetch_tt(mov, self.prefetch()) else {
                continue; // illegal pseudolegal move
            };
            #[cfg(debug_assertions)]
            let debug_history_len = self.params.history.len();
            self.record_move(mov, pos, ply, MainSearch);

            if root && depth >= 8 && self.start_time.elapsed().as_millis() >= 3000 {
                let move_num = self.search_stack[0].tried_moves.len();
                // `qsearch` would give better results, but would make bench be nondeterministic
                let score = -self.eval(&new_pos, 0);
                self.send_currmove(mov, move_num, score, alpha, beta);
            }
            if move_score < KILLER_SCORE {
                num_uninteresting_visited += 1;
            }

            let nodes_before_move = self.state.uci_nodes();
            // PVS (Principal Variation Search): Assume that the TT move is the best move, so we only need to prove
            // that the other moves are worse, which we can do with a zero window search. Should this assumption fail,
            // re-search with a full window.
            let mut score;
            let first_child = self.search_stack[ply].tried_moves.len() == 1;
            let mut child_alpha = -beta;
            let mut child_beta = -alpha;
            if first_child {
                // Idea from Nalwald
                if let Some(tt_entry) = old_entry {
                    if tt_entry.score() >= beta && tt_entry.bound() == FailHigh {
                        child_beta = -(beta - 1);
                    }
                }
                let child_node_type = expected_node_type.inverse();
                score = -self.negamax(new_pos, ply + 1, depth - 1, child_alpha, child_beta, child_node_type)?;
            } else {
                child_alpha = -(alpha + 1);
                // LMR (Late Move Reductions): Trust the move ordering (quiet history, continuation history and capture history heuristics)
                // and assume that moves ordered later are worse. Therefore, we can do a reduced-depth search with a null window
                // to verify our belief.
                // I think it's common to have a minimum depth for doing LMR, but not having that gained elo.
                let mut reduction = 0;
                if num_uninteresting_visited >= cc::lmr_min_uninteresting() {
                    reduction = depth / cc::lmr_depth_div()
                        + (num_uninteresting_visited + 1).ilog2() as isize
                        + cc::lmr_const();
                    // Reduce bad captures and quiet moves with bad combined history scores more.
                    if move_score < MoveScore(cc::lmr_bad_hist()) {
                        reduction += 1;
                    } else if move_score > MoveScore(cc::lmr_good_hist()) {
                        // Since the TT and killer move and good captures are not lmr'ed,
                        // this only applies to quiet moves with a good combined history score.
                        reduction -= 1;
                    }
                    if !is_pv_node {
                        reduction += 1;
                    }
                    if we_blundered {
                        reduction += 1;
                    }
                    if new_pos.is_in_check() {
                        reduction -= 1;
                    }
                    if in_check {
                        reduction -= 1;
                    }
                }
                // Futility Reduction: If this move is not a TT move, good SEE capture or killer, and our eval is significantly
                // less than alpha, reduce.
                if !in_check
                    && depth >= cc::min_fr_depth()
                    && move_score < KILLER_SCORE
                    && eval + cc::fr_base() + cc::fr_scale() * (depth as ScoreT) < alpha
                {
                    reduction += ((depth + 4) / 4).ilog2() as isize;
                }
                // if the TT move is a capture and we didn't already fail high, it's likely that later moves are worse
                if !in_check && is_noisy {
                    reduction += 1;
                }
                // this ensures that check extensions prevent going into qsearch while in check
                reduction = reduction.clamp(0, depth - 1);

                score = -self.negamax(new_pos, ply + 1, depth - 1 - reduction, child_alpha, child_beta, FailHigh)?;
                // If the score turned out to be better than expected (at least `alpha`), this might just be because
                // of the reduced depth. So do a full-depth search first, but don't use the full window quite yet.
                if alpha < score && reduction > 0 {
                    // do deeper / shallower: Adjust the first re-search depth based on the result of the first search
                    let mut retry_depth = depth - 1;
                    if score > alpha + 50 + 4 * depth as ScoreT {
                        retry_depth += 1;
                    } else if score < alpha + 10 {
                        retry_depth -= 1;
                    }
                    self.statistics.lmr_first_retry();
                    // we still expect the child to fail high here
                    score = -self.negamax(new_pos, ply + 1, retry_depth, child_alpha, child_beta, FailHigh)?;
                }
            }
            // If the full-depth null-window search performed better than expected, do a full-depth search with the
            // full window to find the true score.
            // This is only relevant for PV nodes, because all other nodes are searched with a null window anyway.
            // This is also necessary to ensure that the PV doesn't get truncated, because otherwise there could be nodes in
            // the PV that were not searched as PV nodes. So we make sure we're researching in PV nodes with beta == alpha + 1.
            if is_pv_node
                && child_beta - child_alpha == Score(1)
                && if first_child { score < beta } else { score > alpha }
            {
                self.statistics.lmr_second_retry();
                score = -self.negamax(new_pos, ply + 1, depth - 1, -beta, -alpha, Exact)?;
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
                self.state.custom.root_move_nodes.update(mov, self.state.uci_nodes() - nodes_before_move);
                let move_num = self.search_stack[0].tried_moves.len() - 1;
                if move_num < 5 && self.start_time.elapsed().as_millis() >= 3000 {
                    self.send_refutation(mov, score, move_num);
                }
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

            // Update the PV. We only need to do this for PV nodes (we could even only do this for non-fail highs,
            // if we didn't have to worry about aw fail high).
            if is_pv_node {
                let ([.., current], [child, ..]) = self.search_stack.split_at_mut(ply + 1) else { unreachable!() };
                current.pv.extend(best_move, &child.pv);
                if cfg!(debug_assertions)
                    && depth > 1
                    && self.params.thread_type.num_threads() == Some(1)
                    && score < beta
                    && !score.is_won_lost_or_draw_score()
                {
                    debug_assert_eq!(self.tt().load::<Chessboard>(new_pos.hash_pos(), ply + 1).unwrap().bound(), Exact);
                }
            }

            if score < beta {
                // We're in a PVS PV node and this move raised alpha but didn't cause a fail high, so look at the other moves.
                // PVS PV nodes are rare
                bound_so_far = Exact;
                continue;
            }
            // Beta cutoff. Update history and killer for quiet moves, then break out of the move loop.
            bound_so_far = FailHigh;
            self.update_histories_and_killer(&pos, mov, depth, ply);
            break;
        }

        // ******************************************************
        // ***** After move loop, save some info and return *****
        // ******************************************************

        // Update statistics for this node as soon as we know the node type, before returning.
        self.state.statistics.count_complete_node(
            MainSearch,
            bound_so_far,
            depth,
            ply,
            self.state.search_stack[ply].tried_moves.len(),
        );

        if self.search_stack[ply].tried_moves.is_empty() {
            return Some(game_result_to_score(pos.no_moves_result(), ply));
        }

        let tt_entry: TTEntry<Chessboard> =
            TTEntry::new(pos.hash_pos(), best_score, raw_eval, best_move, depth, bound_so_far);
        // Store the results in the TT, always replacing the previous entry. Note that the TT move is only overwritten
        // if this node was an exact or fail high node or if there was a collision.
        if !(root && self.current_pv_num > 0) {
            self.tt_mut().store(tt_entry, ply);
        }

        // Corrhist updates
        if !(in_check
            || best_move.is_tactical(&pos)
            || (best_score <= eval && bound_so_far == NodeType::lower_bound())
            || (best_score >= eval && bound_so_far == NodeType::upper_bound()))
        {
            self.corr_hist.update(&pos, depth, eval, best_score);
        }

        Some(best_score)
    }

    /// Search only "tactical" moves to quieten down the position before calling eval
    fn qsearch(&mut self, pos: Chessboard, mut alpha: Score, beta: Score, ply: usize) -> Option<Score> {
        self.statistics.count_node_started(Qsearch);
        // updating seldepth only in qsearch meaningfully increased performance and was even measurable in a [0, 10] SPRT.
        // TODO: That's weird, retest
        self.atomic().update_seldepth(ply);

        // check nodes in qsearch to allow `go nodes n` to go exactly `n` nodes
        if self.should_stop() {
            return None;
        }
        let in_check = pos.is_in_check();
        // The stand pat check. Since we're not looking at all moves, it's very likely that there's a move we didn't
        // look at that doesn't make our position worse, so we don't want to assume that we have to play a capture.
        let raw_eval;
        let mut eval;
        let mut bound_so_far = FailLow;

        // see main search, store an invalid null move in the TT entry if all moves failed low.
        let mut best_move = ChessMove::default();

        // Don't do TT cutoffs with alpha already raised by the stand pat check, because that relies on the null move observation.
        // But if there's a TT entry from normal search that's worse than the stand pat score, we should trust that more.
        if let Some(tt_entry) = self.tt().load::<Chessboard>(pos.hash_pos(), ply) {
            debug_assert_eq!(tt_entry.hash, pos.hash_pos());
            let bound = tt_entry.bound();
            let tt_score = tt_entry.score();
            // depth 0 drops immediately to qsearch, so a depth 0 entry always comes from qsearch.
            // However, if we've already done qsearch on this position, we can just re-use the result,
            // so there is no point in checking the depth at all
            if (bound == NodeType::lower_bound() && tt_score >= beta)
                || (bound == NodeType::upper_bound() && tt_score <= alpha)
                || bound == Exact
            {
                self.statistics.tt_cutoff(Qsearch, bound);
                return Some(tt_score);
            }
            raw_eval = tt_entry.raw_eval();
            eval = raw_eval;

            // even though qsearch never checks for game over conditions, it's still possible for it to load a checkmate score
            // and propagate that up to a qsearch parent node, where it gets saved with a depth of 0, so game over scores
            // with a depth of 0 in the TT are possible
            // exact scores should have already caused a cutoff
            // TODO: Removing the `&& !tt_entry.score.is_game_over_score()` condition here and in `negamax` *failed* a
            // nonregression SPRT with `[-7, 0]` bounds even though I don't know why, and those conditions make it fail
            // the re-search test case. So the conditions are still disabled for now,
            // test reintroducing them at some point in the future after I have TT aging!
            if (bound == NodeType::lower_bound() && tt_score >= raw_eval)
                || (bound == NodeType::upper_bound() && tt_score <= raw_eval)
            {
                eval = tt_score;
            };
            if let Some(mov) = tt_entry.mov.check_pseudolegal(&pos) {
                best_move = mov;
            }
        } else {
            raw_eval = if in_check { SCORE_LOST + ply as ScoreT } else { self.eval(&pos, ply) };
            eval = raw_eval;
        }
        let mut best_score = eval;
        if !in_check {
            best_score = self.corr_hist.correct(&pos, eval);
        }
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

        self.maybe_send_currline(&pos, alpha, beta, ply, Some(best_score));

        let mut move_picker: MovePicker<Chessboard, MAX_CHESS_MOVES_IN_POS> =
            MovePicker::new(pos, best_move, !in_check);
        let move_scorer = CapsMoveScorer { board: pos, ply };
        let mut children_visited = 0;
        while let Some((mov, score)) = move_picker.next(&move_scorer, &self.state) {
            debug_assert!(mov.is_tactical(&pos) || pos.is_in_check());
            if !eval.is_game_lost_score() && score < MoveScore(0) || children_visited >= 3 {
                // qsearch see pruning and qsearch late move  pruning (lmp):
                // If the move has a negative SEE score or if we've already looked at enough moves, don't even bother playing it in qsearch.
                break;
            }
            let Some(new_pos) = pos.make_move(mov) else {
                continue;
            };
            self.record_move(mov, pos, ply, Qsearch);
            children_visited += 1;
            let score = -self.qsearch(new_pos, -beta, -alpha, ply + 1)?;
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
        }
        self.statistics.count_complete_node(Qsearch, bound_so_far, 0, ply, children_visited);

        let tt_entry: TTEntry<Chessboard> =
            TTEntry::new(pos.hash_pos(), best_score, raw_eval, best_move, 0, bound_so_far);
        self.tt_mut().store(tt_entry, ply);
        Some(best_score)
    }

    fn eval(&mut self, pos: &Chessboard, ply: usize) -> Score {
        let us = self.params.pos.active_player();
        let res = if ply == 0 {
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
            !res.is_won_or_lost() || UnverifiedChessboard::new(*pos).verify(Strict).is_err(),
            "{res} {0} {1}, {pos}",
            res.0,
            self.eval.eval(pos, ply, us)
        );
        res.clamp(MIN_NORMAL_SCORE, MAX_NORMAL_SCORE)
    }

    fn update_continuation_hist(
        mov: ChessMove,
        prev_move: ChessMove,
        bonus: HistScoreT,
        color: ChessColor,
        pos: &Chessboard,
        hist: &mut ContHist,
        failed: &[ChessMove],
    ) {
        if prev_move == ChessMove::default() {
            return; // Ignore NMP null moves
        }
        hist.update(mov, prev_move, bonus, color);
        for disappointing in failed.iter().dropping_back(1).filter(|m| !m.is_tactical(pos)) {
            hist.update(*disappointing, prev_move, -bonus, color);
        }
    }

    fn update_histories_and_killer(&mut self, pos: &Chessboard, mov: ChessMove, depth: isize, ply: usize) {
        let color = pos.active_player();
        let (before, [entry, ..]) = self.state.search_stack.split_at_mut(ply) else { unreachable!() };
        let bonus = (depth * cc::hist_depth_bonus()) as HistScoreT;
        let threats = pos.threats();
        if mov.is_tactical(pos) {
            for disappointing in entry.tried_moves.iter().dropping_back(1).filter(|m| m.is_tactical(pos)) {
                self.state.custom.capt_hist.update(*disappointing, threats, color, -bonus);
            }
            self.state.custom.capt_hist.update(mov, threats, color, bonus);
            return;
        }
        entry.killer = mov;
        for disappointing in entry.tried_moves.iter().dropping_back(1).filter(|m| !m.is_tactical(pos)) {
            self.state.custom.history.update(*disappointing, threats, -bonus);
        }
        self.state.custom.history.update(mov, threats, bonus);
        if ply > 0 {
            let parent = before.last_mut().unwrap();
            Self::update_continuation_hist(
                mov,
                parent.last_tried_move(),
                bonus,
                color,
                pos,
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
                    color,
                    pos,
                    fmh,
                    &entry.tried_moves,
                );
            }
        }
    }

    fn record_pos(&mut self, pos: Chessboard, eval: Score, ply: usize) {
        self.search_stack[ply].pos = pos;
        self.search_stack[ply].eval = eval;
        self.search_stack[ply].tried_moves.clear();
    }

    fn record_move(&mut self, mov: ChessMove, old_pos: Chessboard, ply: usize, typ: SearchType) {
        _ = self.atomic().count_node();
        self.params.history.push(old_pos.hash_pos());
        self.search_stack[ply].tried_moves.push(mov);
        self.statistics.count_legal_make_move(typ);
    }

    // gets skipped when aborting search, but that's fine
    fn undo_move(&mut self) {
        self.params.history.pop();
    }

    #[inline]
    fn maybe_send_currline(&mut self, pos: &Chessboard, alpha: Score, beta: Score, ply: usize, score: Option<Score>) {
        if self.uci_nodes() % DEFAULT_CHECK_TIME_INTERVAL == 0 && self.last_msg_time.elapsed().as_millis() >= 1000 {
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

#[derive(Debug)]
struct CapsMoveScorer {
    board: Chessboard,
    ply: usize,
}

impl MoveScorer<Chessboard, Caps> for CapsMoveScorer {
    /// Order moves so that the most promising moves are searched first.
    /// The most promising move is always the TT move, because that is backed up by search.
    /// After that follow various heuristics.
    fn score_move_eager_part(&self, mov: ChessMove, state: &CapsState) -> MoveScore {
        // The move list is iterated backwards, which is why better moves get higher scores
        // No need to check against the TT move because that's already handled by the move picker
        if mov.is_tactical(&self.board) {
            let captured = mov.captured(&self.board);
            let base_val = MoveScore(HIST_DIVISOR * 10);
            let hist_val = state.capt_hist.get(mov, self.board.threats(), self.board.active_player());
            base_val + MoveScore(captured as i16 * HIST_DIVISOR) + hist_val
        } else if mov == state.search_stack[self.ply].killer {
            // `else` ensures that tactical moves can't be killers
            KILLER_SCORE
        } else {
            let countermove_score = if self.ply > 0 {
                let prev_move = state.search_stack[self.ply - 1].last_tried_move();
                state.countermove_hist.score(mov, prev_move, self.board.active_player())
            } else {
                0
            };
            let follow_up_score = if self.ply > 1 {
                let prev_move = state.search_stack[self.ply - 2].last_tried_move();
                state.follow_up_move_hist.score(mov, prev_move, self.board.active_player())
            } else {
                0
            };
            MoveScore(state.history.get(mov, self.board.threats()) + countermove_score + follow_up_score / 2)
        }
    }

    const DEFERRED_OFFSET: MoveScore = MoveScore(HIST_DIVISOR * -30);

    /// Only compute SEE scores for moves when we're actually trying to play them.
    /// Idea from Cosmo.
    fn defer_playing_move(&self, mov: ChessMove) -> bool {
        mov.is_tactical(&self.board) && !self.board.see_at_least(mov, SeeScore(0))
    }
}

#[cfg(test)]
mod tests {
    use gears::games::chess::ChessColor::White;
    use gears::games::chess::Chessboard;
    use gears::general::board::BoardHelpers;
    use gears::general::board::Strictness::{Relaxed, Strict};
    use gears::general::moves::UntrustedMove;
    use gears::search::NodesLimit;

    use crate::eval::chess::lite::{KingGambot, LiTEval};
    use crate::eval::chess::material_only::MaterialOnlyEval;
    use crate::eval::chess::piston::PistonEval;
    use crate::eval::rand_eval::RandEval;
    use crate::search::tests::generic_engine_test;

    use super::*;

    #[test]
    fn mate_in_one_test() {
        let board = Chessboard::from_fen("4k3/8/4K3/8/8/8/8/6R1 w - - 0 1", Strict).unwrap();
        // run multiple times to get different random numbers from the eval function
        for depth in 1..=3 {
            for _ in 0..42 {
                let mut engine = Caps::for_eval::<RandEval>();
                let res = engine.search_with_new_tt(board, SearchLimit::depth(Depth::new(depth)));
                assert!(res.score.is_game_won_score());
                assert_eq!(res.score.plies_until_game_won(), Some(1));
            }
        }
    }

    #[test]
    fn simple_search_test() {
        let list = [
            ("r2q1r2/ppp1pkb1/2n1p1pp/2N1P3/2pP2Q1/2P1B2P/PP3PP1/R4RK1 b - - 1 18", -500, -100),
            ("r1bqkbnr/3n2p1/2p1pp1p/pp1p3P/P2P4/1PP1PNP1/1B3P2/RN1QKB1R w KQkq - 0 14", 90, 300),
        ];
        for (fen, min, max) in list {
            let pos = Chessboard::from_fen(fen, Strict).unwrap();
            let mut engine = Caps::for_eval::<PistonEval>();
            let res = engine.search_with_new_tt(pos, SearchLimit::nodes(NodesLimit::new(30_000).unwrap()));
            assert!(res.score > Score(min));
            assert!(res.score < Score(max));
        }
    }

    #[test]
    fn lucena_test() {
        let pos = Chessboard::from_name("lucena").unwrap();
        let mut engine = Caps::for_eval::<PistonEval>();
        let res = engine.search_with_new_tt(pos, SearchLimit::depth(Depth::new(7)));
        // TODO: More aggressive bound once the engine is stronger
        assert!(res.score >= Score(200));
    }

    #[test]
    fn philidor_test() {
        let pos = Chessboard::from_name("philidor").unwrap();
        let mut engine = Caps::for_eval::<LiTEval>();
        let res = engine.search_with_new_tt(pos, SearchLimit::nodes(NodesLimit::new(50_000).unwrap()));
        assert!(res.score.abs() <= Score(256));
    }

    #[test]
    fn kiwipete_test() {
        let pos = Chessboard::from_name("kiwipete").unwrap();
        let mut engine = Caps::for_eval::<LiTEval>();
        let res = engine.search_with_new_tt(pos, SearchLimit::nodes(NodesLimit::new(12_345).unwrap()));
        let score = res.score;
        assert!(score.abs() <= Score(64), "{score}");
        assert!(
            [ChessMove::from_compact_text("e2a6", &pos).unwrap(), ChessMove::from_compact_text("d5e6", &pos).unwrap()]
                .contains(&res.chosen_move),
            "{}",
            res.chosen_move.compact_formatter(&pos)
        );
    }

    #[test]
    fn generic_test() {
        generic_engine_test(Caps::for_eval::<LiTEval>());
        generic_engine_test(Caps::for_eval::<RandEval>());
        let tt = TT::default();
        depth_1_nodes_test(Caps::for_eval::<RandEval>(), tt.clone());
        depth_1_nodes_test(Caps::for_eval::<MaterialOnlyEval>(), tt.clone());
        depth_1_nodes_test(Caps::for_eval::<PistonEval>(), tt.clone());
        depth_1_nodes_test(Caps::for_eval::<KingGambot>(), tt.clone());
        depth_1_nodes_test(Caps::for_eval::<LiTEval>(), tt.clone());
    }

    // TODO: Eventually, make sure that GAPS also passed this
    fn depth_1_nodes_test(mut engine: Caps, tt: TT) {
        for pos in Chessboard::bench_positions() {
            let _ = engine.search_with_tt(pos, SearchLimit::depth_(1), tt.clone());
            if pos.legal_moves_slow().is_empty() {
                continue;
            }
            let root_entry = tt.load(pos.hash_pos(), 0).unwrap();
            assert!(root_entry.depth <= 2); // possible extensions
            assert_eq!(root_entry.bound(), Exact);
            assert!(root_entry.mov.check_legal(&pos).is_some());
            let moves = pos.legal_moves_slow();
            assert!(engine.uci_nodes() as usize >= moves.len()); // >= because of extensions
            for m in moves {
                let new_pos = pos.make_move(m).unwrap();
                let entry = tt.load::<Chessboard>(new_pos.hash_pos(), 1);
                let Some(entry) = entry else {
                    continue; // it's possible that a position is not in the TT because qsearch didn't save it
                };
                assert!(entry.depth <= 2, "{entry:?} {new_pos}");
                assert!(-entry.score <= root_entry.score, "{entry:?}\n{root_entry:?}\n{new_pos}");
            }
        }
    }
    #[test]
    fn only_one_move_test() {
        let fen = "B4QRb/8/8/8/2K3P1/5k2/8/b3RRNB b - - 0 1";
        let pos = Chessboard::from_fen(fen, Relaxed).unwrap();
        assert!(pos.debug_verify_invariants(Strict).is_err());
        let mut caps = Caps::for_eval::<PistonEval>();
        let limit = SearchLimit::per_move(Duration::from_millis(999_999_999));
        let res = caps.search_with_new_tt(pos, limit);
        assert_eq!(res.chosen_move, ChessMove::from_compact_text("f3g3", &pos).unwrap());
        assert_eq!(caps.depth().get(), 1);
        assert!(caps.uci_nodes() <= 1000); // might be a bit more than 1 because of check extensions
    }

    #[test]
    fn mate_research_test() {
        let pos = Chessboard::from_fen("k7/3B4/4N3/K7/8/8/8/8 w - - 16 9", Strict).unwrap();
        let mut caps = Caps::for_eval::<LiTEval>();
        let limit = SearchLimit::mate_in_moves(5);
        let res = caps.search_with_new_tt(pos, limit);
        assert!(res.score.is_game_won_score());
        let nodes = caps.search_state().uci_nodes();
        let tt = caps.search_state().tt().clone();
        // Don't clear the internal state
        let second_search = caps.search_with_tt(pos, limit, tt.clone());
        assert!(second_search.score.is_game_won_score());
        let second_search_nodes = caps.search_state().uci_nodes();
        assert!(second_search_nodes * 2 < nodes, "{second_search_nodes} {nodes}");
        let d3 = SearchLimit::depth(Depth::new(3));
        let d3_search = caps.search_with_tt(pos, d3, tt.clone());
        assert!(d3_search.score.is_game_won_score(), "{}", d3_search.score.0);
        let d3_nodes = caps.search_state().uci_nodes();
        caps.forget();
        assert_eq!(caps.search_state().uci_nodes(), 0);
        let fresh_d3_search = caps.search_with_new_tt(pos, d3);
        assert!(!fresh_d3_search.score.is_won_or_lost(), "{}", fresh_d3_search.score.0);
        let fresh_d3_nodes = caps.search_state().uci_nodes();
        assert!(fresh_d3_nodes > d3_nodes + d3_nodes / 4, "{fresh_d3_nodes} {d3_nodes}");
        caps.forget();
        _ = caps.search_with_new_tt(pos, d3);
        assert_eq!(caps.search_state().uci_nodes(), fresh_d3_nodes);
    }

    #[test]
    fn move_order_test() {
        let fen = "7k/8/8/8/p7/1p6/1R1r4/K7 w - - 4 3";
        let pos = Chessboard::from_fen(fen, Relaxed).unwrap();
        let tt_move = ChessMove::from_text("a1b1", &pos).unwrap();
        let mut tt = TT::default();
        let entry = TTEntry::new(pos.hash_pos(), Score(0), Score(-12), tt_move, 123, Exact);
        tt.store::<Chessboard>(entry, 0);
        let threats = pos.threats();
        let mut caps = Caps::default();
        let killer = ChessMove::from_text("b2c2", &pos).unwrap();
        caps.search_stack[0].killer = killer;
        let hist_move = ChessMove::from_text("b2b1", &pos).unwrap();
        caps.history.update(hist_move, threats, 1000);
        let bad_quiet = ChessMove::from_text("b2a2", &pos).unwrap();
        caps.history.update(bad_quiet, threats, -1);
        let bad_capture = ChessMove::from_text("b2b3", &pos).unwrap();
        caps.capt_hist.update(bad_capture, threats, White, 100);

        let mut move_picker: MovePicker<Chessboard, MAX_CHESS_MOVES_IN_POS> = MovePicker::new(pos, tt_move, false);
        let move_scorer = CapsMoveScorer { board: pos, ply: 0 };
        let mut moves = vec![];
        let mut scores = vec![];
        while let Some((mov, score)) = move_picker.next(&move_scorer, &caps.state) {
            moves.push(mov);
            scores.push(score);
        }
        assert_eq!(moves.len(), 6);
        assert!(scores.is_sorted_by(|a, b| a > b), "{scores:?} {moves:?} {pos}");
        assert_eq!(scores[0], MoveScore::MAX);
        assert_eq!(moves[0], tt_move);
        let good_capture = ChessMove::from_text("b2d2", &pos).unwrap();
        assert_eq!(moves[1], good_capture);
        assert_eq!(moves[2], killer);
        assert_eq!(moves[3], hist_move);
        let illegal = ChessMove::from_text("a1a2", &pos).unwrap();
        assert!(!moves.contains(&illegal));
        assert!(!pos.is_pseudolegal_move_legal(illegal));
        assert_eq!(moves[4], bad_quiet);
        assert_eq!(moves[5], bad_capture);
        let search_res = caps.search_with_tt(pos, SearchLimit::depth_(1), tt.clone());
        assert_eq!(search_res.chosen_move, good_capture);
        assert!(search_res.score > Score(0));
        let tt_entry = tt.load::<Chessboard>(pos.hash_pos(), 0).unwrap();
        assert_eq!(tt_entry.score, search_res.score.compact());
        assert_eq!(tt_entry.mov, UntrustedMove::from_move(good_capture));
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
            let pos = Chessboard::from_fen(fen, Relaxed).unwrap();
            let mut engine = Caps::for_eval::<LiTEval>();
            let limit = SearchLimit::mate_in_moves(num_moves);
            let res = engine.search_with_new_tt(pos, limit);
            let score = res.score;
            println!(
                "chosen move {0}, fen {1}, depth {2}, time {3}ms",
                res.chosen_move.extended_formatter(&pos, Standard),
                pos.as_fen(),
                engine.depth(),
                engine.start_time.elapsed().as_millis()
            );
            assert!(score.is_game_won_score());
            assert_eq!(res.chosen_move.compact_formatter(&pos).to_string(), best_move);
        }
    }
}
