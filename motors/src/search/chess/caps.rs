use std::cmp::min;
use std::mem::take;
use std::num::Wrapping;
use std::time::{Duration, Instant};

use crate::eval::chess::lite::LiTEval;
use crate::eval::Eval;
use crate::io::ugi_output::{color_for_score, score_gradient};
use crate::search::chess::caps_values::cc;
use crate::search::move_picker::MovePicker;
use crate::search::statistics::SearchType;
use crate::search::statistics::SearchType::{MainSearch, Qsearch};
use crate::search::tt::TTEntry;
use crate::search::*;
use derive_more::{Deref, DerefMut, Index, IndexMut};
use gears::arrayvec::ArrayVec;
use gears::games::chess::moves::{ChessMove, ChessMoveFlags};
use gears::games::chess::pieces::ChessPieceType::Pawn;
use gears::games::chess::see::SeeScore;
use gears::games::chess::squares::ChessSquare;
use gears::games::chess::{ChessColor, Chessboard, MAX_CHESS_MOVES_IN_POS};
use gears::games::{n_fold_repetition, BoardHistory, ZobristHash, ZobristHistory};
use gears::general::bitboards::RawBitboard;
use gears::general::common::Description::NoDescription;
use gears::general::common::{
    parse_bool_from_str, parse_int_from_str, select_name_static, Res, StaticallyNamedEntity,
};
use gears::general::move_list::EagerNonAllocMoveList;
use gears::general::moves::Move;
use gears::output::text_output::AdaptFormatter;
use gears::output::Message::Debug;
use gears::score::{
    game_result_to_score, ScoreT, MAX_BETA, MAX_NORMAL_SCORE, MAX_SCORE_LOST, MIN_ALPHA,
    NO_SCORE_YET,
};
use gears::search::NodeType::*;
use gears::search::*;
use gears::ugi::EngineOptionName::*;
use gears::ugi::EngineOptionType::Check;
use gears::ugi::{EngineOption, EngineOptionName, EngineOptionType, UgiCheck};
use gears::PlayerResult::{Lose, Win};
use itertools::Itertools;

/// The maximum value of the `depth` parameter, i.e. the maximum number of Iterative Deepening iterations.
const DEPTH_SOFT_LIMIT: Depth = Depth::new_unchecked(225);
/// The maximum value of the `ply` parameter, i.e. the maximum depth (in plies) before qsearch is reached
const DEPTH_HARD_LIMIT: Depth = Depth::new_unchecked(255);

/// Qsearch can't go more than 30 plies deep, so this prevents out of bounds accesses
const SEARCH_STACK_LEN: usize = DEPTH_HARD_LIMIT.get() + 30;

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
    fn update(&mut self, mov: ChessMove, color: ChessColor, bonus: i32) {
        let entry =
            &mut self[color as usize][mov.piece_type() as usize][mov.dest_square().bb_idx()];
        update_history_score(entry, bonus);
    }
    fn get(&self, mov: ChessMove, color: ChessColor) -> MoveScore {
        MoveScore(self[color as usize][mov.piece_type() as usize][mov.dest_square().bb_idx()])
    }
}

impl Default for CaptHist {
    fn default() -> Self {
        Self([[[0; 64]; 6]; 2])
    }
}

/// Continuation history.
/// Used for Countermove History (CMH, 1 ply ago) and Follow-up Move History (FMH, 2 plies ago).
/// Unlike the main quiet history heuristic, this in indexed by the previous piece, previous target square,
/// current piece, current target square, and color.
#[derive(Debug, Clone, Deref, DerefMut, Index, IndexMut)]
struct ContHist(Vec<i32>); // Can't store this on the stack because it's too large.

impl ContHist {
    fn idx(mov: ChessMove, prev_move: ChessMove, color: ChessColor) -> usize {
        (mov.piece_type() as usize + mov.dest_square().bb_idx() * 6)
            + (prev_move.piece_type() as usize + prev_move.dest_square().bb_idx() * 6) * 64 * 6
            + color as usize * 64 * 6 * 64 * 6
    }
    fn update(&mut self, mov: ChessMove, prev_mov: ChessMove, bonus: i32, color: ChessColor) {
        let entry = &mut self[Self::idx(mov, prev_mov, color)];
        update_history_score(entry, bonus);
    }
    fn score(&self, mov: ChessMove, prev_move: ChessMove, color: ChessColor) -> i32 {
        self[Self::idx(mov, prev_move, color)]
    }
}

impl Default for ContHist {
    fn default() -> Self {
        ContHist(vec![0; 2 * 6 * 64 * 6 * 64])
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
    original_board_hist: ZobristHistory<Chessboard>,
    nmp_disabled: [bool; 2],
    depth_hard_limit: usize,
}

impl CapsCustomInfo {
    fn nmp_disabled_for(&mut self, color: ChessColor) -> &mut bool {
        &mut self.nmp_disabled[color as usize]
    }
}

impl CustomInfo<Chessboard> for CapsCustomInfo {
    fn new_search(&mut self) {
        // don't update history values, malus and gravity already take care of that
    }

    fn hard_forget_except_tt(&mut self) {
        for value in self.history.iter_mut() {
            *value = 0;
        }
        for value in self.capt_hist.0.iter_mut().flatten().flatten() {
            *value = 0;
        }
        for value in self.countermove_hist.iter_mut() {
            *value = 0;
        }
        for value in self.follow_up_move_hist.iter_mut() {
            *value = 0;
        }
    }

    fn write_internal_info(&self) -> Option<String> {
        Some(
            write_single_hist_table(&self.history, false)
                + "\n"
                + &write_single_hist_table(&self.history, true),
        )
    }
}

fn write_single_hist_table(table: &HistoryHeuristic, flip: bool) -> String {
    let show_square = |from: ChessSquare| {
        let sum: i32 = ChessSquare::iter()
            .map(|to| {
                let idx = if flip {
                    ChessMove::new(to, from, ChessMoveFlags::QueenMove).from_to_square()
                } else {
                    ChessMove::new(from, to, ChessMoveFlags::QueenMove).from_to_square()
                };
                table.0[idx]
            })
            .sum();
        sum as f64 / 64.0
    };
    let as_nums = ChessSquare::iter()
        .map(|sq| {
            let score = show_square(sq);
            format!("{score:^7.1}").color(color_for_score(
                Score((score * 4.0) as ScoreT),
                &score_gradient(),
            ))
        })
        .collect_vec();

    let formatter = Chessboard::default().pretty_formatter(None, None);
    let mut formatter = AdaptFormatter {
        underlying: formatter,
        color_frame: Box::new(|_, col| col),
        display_piece: Box::new(move |sq, _, _| as_nums[sq.bb_idx()].to_string()),
        horizontal_spacer_interval: None,
        vertical_spacer_interval: None,
        square_width: Some(7),
    };
    let text = if flip {
        "Main History Destination Square:\n"
    } else {
        "Main History Source Square:\n"
    }
    .bold()
    .to_string();
    text + &Chessboard::default().display_pretty(&mut formatter)
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
        Some(&self.pv.list[0..self.pv.length])
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
        "Chess-playing Alpha-beta Pruning Search (CAPS), a chess engine. \
        Currently early in development, but still around 3k elo with a hand crafted eval. \
        Much larger than SᴍᴀʟʟCᴀᴘꜱ"
            .to_string()
    }
}

impl Engine<Chessboard> for Caps {
    type SearchStackEntry = CapsSearchStackEntry;
    type CustomInfo = CapsCustomInfo;

    fn with_eval(eval: Box<dyn Eval<Chessboard>>) -> Self {
        Self {
            state: SearchState::new(Depth::new_unchecked(SEARCH_STACK_LEN)),
            eval,
        }
    }

    fn static_eval(&mut self, pos: Chessboard, ply: usize) -> Score {
        self.eval.eval(&pos, ply)
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

    fn search_state(&self) -> &SearchStateFor<Chessboard, Self> {
        &self.state
    }

    fn search_state_mut(&mut self) -> &mut SearchStateFor<Chessboard, Self> {
        &mut self.state
    }

    fn engine_info(&self) -> EngineInfo {
        let mut options = vec![EngineOption {
            name: Other("UCI_Chess960".to_string()),
            value: Check(UgiCheck {
                val: true,
                default: Some(true),
            }),
        }];
        options.append(&mut cc::ugi_options());
        EngineInfo::new(
            self,
            self.eval.as_ref(),
            "0.1.0",
            Depth::new_unchecked(15),
            NodesLimit::new(20_000).unwrap(),
            None,
            options,
        )
    }

    fn time_up(&self, tc: TimeControl, fixed_time: Duration, start_time: Instant) -> bool {
        debug_assert!(self.state.uci_nodes() % DEFAULT_CHECK_TIME_INTERVAL == 0);
        let elapsed = start_time.elapsed();
        // divide by 4 unless moves to go is very small, but don't divide by 1 (or zero) to avoid timeouts
        // TODO: Compute at the start of the search instead of every time:
        // Instead of storing a SearchLimit, store a different struct that contains soft and hard bounds
        let divisor = tc
            .moves_to_go
            .unwrap_or(usize::MAX)
            .clamp(2, cc::hard_limit_div()) as u32;
        // Because fixed_time has been clamped to at most tc.remaining, this can never lead to timeouts
        // (assuming the move overhead is set correctly)
        elapsed >= fixed_time.min(tc.remaining / divisor + tc.increment)
    }

    fn set_option(
        &mut self,
        option: EngineOptionName,
        old_value: &mut EngineOptionType,
        value: String,
    ) -> Res<()> {
        let name = option.name().to_string();
        if let Other(name) = &option {
            if name.eq_ignore_ascii_case("uci_chess960") {
                let Check(check) = old_value else {
                    unreachable!()
                };
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
        select_name_static(
            &name,
            self.engine_info().additional_options().iter(),
            "uci option",
            "chess",
            NoDescription,
        )
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
        let mut limit = self.state.params.limit;
        let pos = self.state.params.pos;
        limit.fixed_time = min(limit.fixed_time, limit.tc.remaining);
        self.state.custom.depth_hard_limit = if limit.mate.get() == 0 {
            DEPTH_HARD_LIMIT.get()
        } else {
            limit.mate.get()
        };
        let soft_limit = limit
            .fixed_time
            .min(
                (limit.tc.remaining.saturating_sub(limit.tc.increment)) / cc::soft_limit_div()
                    + limit.tc.increment,
            )
            .min(limit.tc.remaining / cc::soft_limit_div_clamp());
        self.state.params.limit = limit;

        // Ideally, this would only evaluate the String argument if debug is on, but that's annoying to implement
        // and would still require synchronization because debug mode might be turned on while the engine is searching
        self.state.send_non_ugi(Debug, &format!(
            "Starting search with limit {time}ms, {incr}ms increment, max {fixed}ms, mate in {mate} plies, max depth {depth}, \
            max {nodes} nodes, soft limit {soft}ms, {ignored} ignored moves",
            time = limit.tc.remaining.as_millis(),
            incr = limit.tc.increment.as_millis(),
            mate = limit.mate.get(),
            depth = limit.depth.get(),
            nodes = limit.nodes.get(),
            fixed = limit.fixed_time.as_millis(),
            soft = soft_limit.as_millis(),
            ignored = self.state.excluded_moves.len(),
        ));
        // Use 3fold repetition detection for positions before and including the root node and 2fold for positions during search.
        self.state.custom.original_board_hist = take(&mut self.state.search_params_mut().history);
        self.state.custom.original_board_hist.push(&pos);

        self.iterative_deepening(pos, soft_limit)
    }
}

#[allow(clippy::too_many_arguments)]
impl Caps {
    fn prefetch(&self) -> impl Fn(ZobristHash) + '_ {
        |hash| self.state.tt().prefetch(hash)
    }

    /// Iterative Deepening (ID): Do a depth 1 search, then a depth 2 search, then a depth 3 search, etc.
    /// This has two advantages: It allows the search to be stopped at any time, and it actually improves strength:
    /// The low-depth searches fill the TT and various heuristics, which improves move ordering and therefore results in
    /// better moves within the same time or nodes budget because the lower-depth searches are comparatively cheap.
    fn iterative_deepening(
        &mut self,
        pos: Chessboard,
        soft_limit: Duration,
    ) -> SearchResult<Chessboard> {
        let max_depth = DEPTH_SOFT_LIMIT.min(self.limit().depth).isize();
        let multi_pv = self.state.multi_pv();
        let mut soft_limit_scale = 1.0;

        self.state.multi_pvs.resize(multi_pv, PVData::default());
        let mut chosen_at_depth =
            EagerNonAllocMoveList::<Chessboard, { DEPTH_SOFT_LIMIT.get() }>::default();

        for depth in 1..=max_depth {
            self.state.statistics.next_id_iteration();
            for pv_num in 0..multi_pv {
                self.state.current_pv_num = pv_num;
                self.state.current_pv_data_mut().bound = None;
                let mut pv_data = self.state.multi_pvs[pv_num];
                let keep_searching = self.aspiration(
                    pos,
                    soft_limit.mul_f64(soft_limit_scale),
                    depth,
                    &mut pv_data.alpha,
                    &mut pv_data.beta,
                    &mut pv_data.radius,
                    max_depth,
                );
                let chosen_move = self.state.search_stack[0].pv[0];
                self.state.multi_pvs[pv_num].alpha = pv_data.alpha;
                self.state.multi_pvs[pv_num].beta = pv_data.beta;
                self.state.multi_pvs[pv_num].radius = pv_data.radius;
                self.state.excluded_moves.push(chosen_move);
                if keep_searching {
                    self.search_state().send_search_info();
                } else {
                    // send one final search info, but don't send empty PVs or PVs from a fail high
                    // that would consist of only one move, and don't send a PV if it's
                    let pv = self.state.current_mpv_pv();
                    let immediately_aborted = self.state.depth().get() < depth as usize;
                    if !pv.is_empty() && (depth == 1 || pv.len() > 1) && !immediately_aborted {
                        self.search_state().send_search_info();
                    }
                    return self.state.search_result();
                }
            }
            self.state
                .excluded_moves
                .truncate(self.state.excluded_moves.len() - multi_pv);
            let chosen = self.state.best_move();
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

        self.state.search_result()
    }

    /// Aspiration Windows (AW): Assume that the score will be close to the score from the previous iteration
    /// of Iterative Deepening, so use alpha, beta bounds around that score to prune more aggressively.
    /// This means that it's possible for the root to fail low (or high), which is always something to consider:
    /// For example, the best move is not trustworthy if the root failed low (but because the TT move is ordered first,
    /// and the TT move at the root is always `state.best_move` (there can be no collisions because it's written to last),
    /// it should in theory still be trustworthy if the root failed high)
    fn aspiration(
        &mut self,
        pos: Chessboard,
        unscaled_soft_limit: Duration,
        depth: isize,
        alpha: &mut Score,
        beta: &mut Score,
        window_radius: &mut Score,
        max_depth: isize,
    ) -> bool {
        let mut soft_limit_scale = 1.0;
        loop {
            let soft_limit = unscaled_soft_limit.mul_f64(soft_limit_scale);
            soft_limit_scale = 1.0;
            if self.should_not_start_iteration(soft_limit, max_depth, self.limit().mate) {
                self.state.statistics.soft_limit_stop();
                return false;
            }
            self.state.atomic().set_depth(depth); // set depth now so that an immediate stop doesn't increment the depth
            self.state.atomic().count_node();
            let asp_start_time = Instant::now();
            let Some(pv_score) =
                self.negamax(pos, 0, self.state.depth().isize(), *alpha, *beta, Exact)
            else {
                return false;
            };

            self.state.send_non_ugi(
                Debug,
                &format!(
                    "depth {depth}, score {0}, radius {1}, interval ({2}, {3}) nodes {4}",
                    pv_score.0,
                    window_radius.0,
                    alpha.0,
                    beta.0,
                    self.state.uci_nodes(),
                    depth = self.state.depth().get()
                ),
            );

            let node_type = if pv_score <= *alpha {
                FailLow
            } else if pv_score >= *beta {
                FailHigh
            } else {
                Exact
            };
            self.state.current_pv_data_mut().bound = Some(node_type);

            let atomic = &self.state.params.atomic;
            let pv = &self.state.search_stack[0].pv;
            // we don't trust the best move in fail low nodes, but we still want to display an updated score
            self.state.multi_pvs[self.state.current_pv_num].score = pv_score;
            // adding ` && node_type != FailLow` gains elo, which is weird because this only prevents incomplete search iterations that have
            // already changed the PV from affecting the chosen move.
            if pv.length > 0 && node_type != FailLow {
                if self.state.current_pv_num == 0 {
                    let chosen_move = pv[0];
                    let ponder_move = pv.get(1);
                    atomic.set_best_move(chosen_move);
                    atomic.set_ponder_move(ponder_move);
                }
                self.state.multi_pvs[self.state.current_pv_num]
                    .pv
                    .assign_from(pv);
                // We can't really trust FailHigh scores. Even though we should still prefer a fail high move, we don't
                // want a mate limit condition to trigger, so we clamp the fail high score to MAX_NORMAL_SCORE.
                if self.state.current_pv_num == 0 {
                    if node_type == Exact {
                        atomic.set_score(pv_score); // can't be SCORE_TIME_UP or similar because that wouldn't be exact
                    } else if node_type == FailHigh && !self.state.stop_flag() {
                        // todo: stop flag condition necessary?
                        atomic.set_score(pv_score.min(MAX_NORMAL_SCORE));
                    }
                }
            }

            if node_type == FailLow {
                // In a fail low node, we didn't get any new information, and it's possible that we just discovered
                // a problem with our chosen move. So increase the soft limit such that we can gather more information.
                soft_limit_scale = cc::soft_limit_fail_low_factor() as f64 / 1000.0;
            }
            if cfg!(debug_assertions) {
                if pos.player_result_slow(&self.state.params.history).is_some() {
                    assert_eq!(pv.length, 0);
                } else {
                    match node_type {
                        FailHigh => debug_assert_eq!(pv.length, 1, "{pos} {node_type}"),
                        Exact => debug_assert!(
                            // currently, it's possible to reduce the PV through IIR when the TT entry of a PV node gets overwritten,
                            // but that should be relatively rare. In the future, a better replacement policy might make this actually sound
                            self.state.multi_pv() > 1
                                || pv.length + pv.length / 4
                                    >= self.state.custom.depth_hard_limit.min(depth as usize)
                                || pv_score.is_won_lost_or_draw_score(),
                            "{depth} {0} {pv_score} {1}",
                            pv.length,
                            self.state.uci_nodes()
                        ),
                        // We don't clear the PV on a fail low node so that we can still send a useful info
                        FailLow => {
                            debug_assert_eq!(0, pv.length);
                        }
                    }
                }
            }

            // assert this now because this doesn't hold for incomplete iterations
            debug_assert!(
                !pv_score.is_won_or_lost() || pv_score.plies_until_game_over().unwrap() <= 256,
                "{pv_score}"
            );

            self.state.statistics.aw_node_type(node_type);
            if node_type == Exact {
                *window_radius = Score((window_radius.0 + cc::aw_exact_add()) / cc::aw_exact_div());
            } else {
                let delta = pv_score.0.abs_diff(alpha.0);
                let delta = delta.min(pv_score.0.abs_diff(beta.0));
                let delta = delta.min(cc::aw_delta_max()) as i32;
                window_radius.0 = SCORE_WON
                    .0
                    .min(window_radius.0 * cc::aw_widening_factor() + delta);
            }
            *alpha = (pv_score - *window_radius).max(MIN_ALPHA);
            *beta = (pv_score + *window_radius).min(MAX_BETA);

            if node_type == Exact {
                return true;
            } else if asp_start_time.elapsed().as_millis() >= 1000 {
                self.state.send_search_info();
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
        debug_assert!(alpha < beta);
        debug_assert!(ply <= DEPTH_HARD_LIMIT.get());
        debug_assert!(depth <= DEPTH_SOFT_LIMIT.isize());
        debug_assert!(self.state.params.history.len() >= ply);
        self.state.statistics.count_node_started(MainSearch);

        let root = ply == 0;
        let is_pv_node = expected_node_type == Exact; // TODO: Make this a generic argument of search?
        debug_assert!(!root || is_pv_node); // root implies pv node
        debug_assert!(alpha + 1 == beta || is_pv_node); // alpha + 1 < beta implies Exact node
        if is_pv_node {
            self.state.search_stack[ply].pv.clear();
        }

        // Mate Distance Pruning (MDP): If we've already found a mate in n, don't bother looking for longer mates.
        // This isn't intended to gain elo (since it only works in positions that are already won or lost)
        // but makes the engine better at finding shorter checkmates. Don't do MDP at the root because that can prevent us
        // from ever returning exact scores, since for a mate in 1 the score would always be exactly `beta`.
        if !root {
            alpha = alpha.max(game_result_to_score(Lose, ply));
            beta = beta.min(game_result_to_score(Win, ply + 1));
            if alpha >= beta {
                return Some(alpha);
            }
        }

        let ply_100_ctr = pos.halfmove_repetition_clock();
        if !root
            && (n_fold_repetition(2, &self.state.params.history, &pos, ply_100_ctr)
                || n_fold_repetition(
                    3,
                    &self.state.custom.original_board_hist,
                    &pos,
                    ply_100_ctr.saturating_sub(ply),
                )
                || pos.is_50mr_draw()
                || pos.has_insufficient_material())
        {
            return Some(Score(0));
        }
        let in_check = pos.is_in_check();
        // Check extensions. Increase the depth by 1 if in check.
        // Do this before deciding whether to drop into qsearch.
        if in_check {
            self.state.statistics.in_check();
            depth += 1;
        }
        // limit.mate() is the min of the original limit.mate and DEPTH_HARD_LIMIT
        if depth <= 0 || ply >= self.state.custom.depth_hard_limit {
            return Some(self.qsearch(pos, alpha, beta, ply));
        }
        let can_prune = !is_pv_node && !in_check;

        let mut best_score = NO_SCORE_YET;
        let mut bound_so_far = FailLow;

        // In case of a collision, if there's no best_move to store because the node failed low,
        // store a null move in the TT. This helps IIR.
        let mut best_move = ChessMove::default();
        // Don't initialize eval just now to save work in case we get a TT cutoff
        let mut eval;
        // the TT entry at the root is useless when doing an actual multipv search
        let ignore_tt_entry = root && self.state.multi_pvs.len() > 1;
        let old_entry = self.state.tt().load::<Chessboard>(pos.zobrist_hash(), ply);
        if let Some(tt_entry) = old_entry {
            if !ignore_tt_entry {
                let tt_bound = tt_entry.bound();
                debug_assert_eq!(tt_entry.hash, pos.zobrist_hash());

                // TT cutoffs. If we've already seen this position, and the TT entry has more valuable information (higher depth),
                // and we're not a PV node, and the saved score is either exact or at least known to be outside (alpha, beta),
                // simply return it.
                if !is_pv_node
                    && tt_entry.depth as isize >= depth
                    && ((tt_entry.score() >= beta && tt_bound == NodeType::lower_bound())
                        || (tt_entry.score() <= alpha && tt_bound == NodeType::upper_bound())
                        || tt_bound == Exact)
                {
                    self.state.statistics.tt_cutoff(MainSearch, tt_bound);
                    return Some(tt_entry.score());
                }
                // Even though we didn't get a cutoff from the TT, we can still use the score and bound to update our guess
                // at what the type of this node is going to be.
                if !is_pv_node {
                    expected_node_type = if tt_bound != Exact {
                        // TODO: Base instead on relation between tt score and window?
                        // Or only update if the difference between tt score and the window is large?
                        tt_bound
                    } else if tt_entry.score() <= alpha {
                        FailLow
                    } else {
                        debug_assert!(tt_entry.score() >= beta); // we're using a null window
                        FailHigh
                    }
                }

                if let Some(tt_move) = tt_entry.mov.check_pseudolegal(&pos) {
                    best_move = tt_move;
                }
                eval = self.eval(pos, ply);
                // The TT score is backed by a search, so it should be more trustworthy than a simple call to static eval.
                // Note that the TT score may be a mate score, so `eval` can also be a mate score. This doesn't currently
                // create any problems, but should be kept in mind.
                if tt_bound == Exact
                    || (tt_bound == NodeType::lower_bound() && tt_entry.score() >= eval)
                    || (tt_bound == NodeType::upper_bound() && tt_entry.score() <= eval)
                {
                    eval = tt_entry.score();
                }
            } else {
                eval = self.eval(pos, ply);
            }
        } else {
            self.state.statistics.tt_miss(MainSearch);
            eval = self.eval(pos, ply);
        };

        self.record_pos(pos, eval, ply);

        // like the commonly used `improving` and `regressing`, these variables compare the current static eval with
        // the static eval 2 plies ago to recognize blunders. Conceptually, `improving` and `regressing` can be seen as
        // a prediction for how the eval is going to evolve, while these variables are more about cutting early after bad moves.
        // TODO: Currently, this uses the TT score when possible. Think about if there are unintended consequences.
        let they_blundered = ply >= 2
            && eval - self.state.search_stack[ply - 2].eval > Score(cc::they_blundered_threshold());
        let we_blundered = ply >= 2
            && eval - self.state.search_stack[ply - 2].eval < Score(cc::we_blundered_threshold());

        // IIR (Internal Iterative Reductions): If we don't have a TT move, this node will likely take a long time
        // because the move ordering won't be great, so don't spend too much time on this node.
        // Instead, search it with reduced depth to fill the TT entry so that we can re-search it faster the next time
        // we see this node. If there was no TT entry because the node failed low, this node probably isn't that interesting,
        // so reducing the depth also makes sense in this case.
        if depth >= cc::iir_min_depth() && best_move == ChessMove::default() {
            depth -= 1;
        }

        if can_prune {
            // RFP (Reverse Futility Pruning): If eval is far above beta, it's likely that our opponent
            // blundered in a previous move of the search, so if the depth is low, don't even bother searching further.
            // Use `they_blundered` to better distinguish between blunders by our opponent and a generally good static eval
            // relative to `beta` --  there may be other positional factors that aren't being reflected by the static eval,
            // (like imminent threats) so don't prune too aggressively if our opponent hasn't blundered.
            // Be more careful about pruning too aggressively if the node is expected to fail low -- we should not rfp
            // a true fail low node, but our expectation may also be wrong.
            let mut margin = (cc::rfp_base() - (ScoreT::from(they_blundered) * cc::rfp_blunder()))
                * depth as ScoreT;
            if expected_node_type == FailHigh {
                margin /= cc::rfp_fail_high_div();
            }
            if depth <= cc::rfp_max_depth() && eval >= beta + Score(margin) {
                return Some(eval);
            }

            // NMP (Null Move Pruning). If static eval of our position is above beta, this node probably isn't that interesting.
            // To test this hypothesis, do a null move and perform a search with reduced depth; if the result is still
            // above beta, then it's very likely that the score would have been above beta if we had played a move,
            // so simply return the nmp score. This is based on the null move observation (there are very few zugzwang positions).
            // If we don't have non-pawn, non-king pieces, we're likely to be in zugzwang, so don't even try NMP.
            let has_nonpawns =
                (pos.active_player_bb() & !pos.piece_bb(Pawn)).more_than_one_bit_set();
            let nmp_threshold =
                beta + ScoreT::from(expected_node_type == FailLow) * cc::nmp_fail_low();
            if depth >= cc::nmp_min_depth()
                && eval >= nmp_threshold
                && !*self.state.custom.nmp_disabled_for(pos.active_player())
                && has_nonpawns
            {
                // `make_nullmove` resets the 50mr counter, so we don't consider positions after a nullmove as repetitions,
                // but we can still get TT cutoffs
                self.state.params.history.push(&pos);
                let new_pos = pos.make_nullmove().unwrap();
                // necessary to recognize the null move and to make `last_tried_move()` not panic
                self.state.search_stack[ply]
                    .tried_moves
                    .push(ChessMove::default());
                let reduction =
                    cc::nmp_base() + depth / cc::nmp_depth_div() + isize::from(they_blundered);
                let nmp_res = self.negamax(
                    new_pos,
                    ply + 1,
                    depth - 1 - reduction,
                    -beta,
                    -beta + 1,
                    FailLow, // the child node is expected to fail low, leading to a fail high in this node
                );
                self.state.search_stack[ply].tried_moves.pop();
                self.state.params.history.pop();
                let score = -nmp_res?;
                if score >= beta {
                    // For shallow depths, don't bother with doing a verification search to avoid useless re-searches,
                    // unless we'd be storing a mate score -- we really want to avoid storing unproved mates in the TT.
                    // It's possible to beat beta with a score of getting mated, so use `is_game_over_score`
                    // instead of `is_game_won_score`
                    if depth < cc::nmp_verif_depth() && !score.is_won_or_lost() {
                        return Some(score);
                    }
                    *self.state.custom.nmp_disabled_for(pos.active_player()) = true;
                    // nmp was done with `depth - 1 - reduction`, but we're not doing a null move now, so technically we
                    // should use `depth - reduction`, but using `depth - 1 - reduction` is less expensive and good enough.
                    let verification_score =
                        self.negamax(pos, ply, depth - 1 - reduction, beta - 1, beta, FailHigh);
                    self.state.search_stack[ply].tried_moves.clear();
                    *self.state.custom.nmp_disabled_for(pos.active_player()) = false;
                    // The verification score is more trustworthy than the nmp score.
                    if verification_score.is_none_or(|score| score >= beta) {
                        return verification_score;
                    }
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
            if can_prune
                && best_score > MAX_SCORE_LOST
                && depth <= cc::max_move_loop_pruning_depth()
                && (num_uninteresting_visited >= lmp_threshold
                    || (eval + Score(fp_margin as ScoreT) < alpha && move_score < KILLER_SCORE))
            {
                break;
            }

            if ply == 0 && self.state.excluded_moves.contains(&mov) {
                continue;
            }
            let Some(new_pos) = pos.make_move_and_prefetch_tt(mov, self.prefetch()) else {
                continue; // illegal pseudolegal move
            };
            if move_score < KILLER_SCORE {
                num_uninteresting_visited += 1;
            }

            // O(1). Resets the child's pv length so that it's not the maximum length it used to be.
            // TODO: Do this in `record_move`?
            if let Some(s) = self.state.search_stack.get_mut(ply + 1) {
                s.pv.clear()
            }

            let debug_history_len = self.state.params.history.len(); // TODO: Remove

            self.record_move(mov, pos, ply, MainSearch);
            // PVS (Principal Variation Search): Assume that the TT move is the best move, so we only need to prove
            // that the other moves are worse, which we can do with a zero window search. Should this assumption fail,
            // re-search with a full window.
            let mut score;
            if self.state.search_stack[ply].tried_moves.len() == 1 {
                score = -self.negamax(
                    new_pos,
                    ply + 1,
                    depth - 1,
                    -beta,
                    -alpha,
                    expected_node_type.inverse(),
                )?;
            } else {
                // LMR (Late Move Reductions): Trust the move ordering (quiet history, continuation history and capture history heuristics)
                // and assume that moves ordered later are worse. Therefore, we can do a reduced-depth search with a null window
                // to verify our belief.
                // I think it's common to have a minimum depth for doing LMR, but not having that gained elo.
                let mut reduction = 0;
                if !in_check && num_uninteresting_visited >= cc::lmr_min_uninteresting() {
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
                }
                // this ensures that check extensions prevent going into qsearch while in check
                reduction = reduction.min(depth - 1);

                score = -self.negamax(
                    new_pos,
                    ply + 1,
                    depth - 1 - reduction,
                    -(alpha + 1),
                    -alpha,
                    FailHigh,
                )?;
                // If the score turned out to be better than expected (at least `alpha`), this might just be because
                // of the reduced depth. So do a full-depth search first, but don't use the full window quite yet.
                if alpha < score && reduction > 0 {
                    self.state.statistics.lmr_first_retry();
                    score = -self.negamax(
                        new_pos,
                        ply + 1,
                        depth - 1,
                        -(alpha + 1),
                        -alpha,
                        FailHigh, // we still expect a fail high here
                    )?;
                }
                // If the full-depth search also performed better than expected, do a full-depth search with the
                // full window to find the true score. If the score was at least `beta`, don't search again
                // -- this move is probably already too good, so don't waste more time finding out how good it is exactly.
                if alpha < score && score < beta {
                    debug_assert_eq!(expected_node_type, Exact);
                    self.state.statistics.lmr_second_retry();
                    score = -self.negamax(new_pos, ply + 1, depth - 1, -beta, -alpha, Exact)?;
                }
            }

            self.undo_move();

            debug_assert_eq!(
                self.state.params.history.len(),
                debug_history_len,
                "depth {depth} ply {ply} old len {debug_history_len} new len {0} child {1}",
                self.state.params.history.len(),
                self.state.search_stack[ply].tried_moves.len()
            );
            // Check for cancellation right after searching a move to avoid storing incorrect information
            // in the TT or PV.
            if self.should_stop() {
                // The current child's score is not trustworthy, but all already seen children are.
                // This only matters for the root; all other nodes get their return value ignored.
                // This should really be used in aspiration() by inspecting the PV, but somehow that loses elo
                return None;
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

            // Update the PV. We only need to do this for PV nodes (we could even only do this for non-fail highs,
            // if we didn't have to worry about aw fail high).
            if is_pv_node {
                let ([.., current], [child, ..]) = self.state.search_stack.split_at_mut(ply + 1)
                else {
                    unreachable!()
                };
                current.pv.extend(ply, best_move, &child.pv);
            }

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

        if self.state.search_stack[ply].tried_moves.is_empty() {
            // TODO: Merge cached in-check branch
            return Some(game_result_to_score(pos.no_moves_result(), ply));
        }

        let new_entry: TTEntry<Chessboard> = TTEntry::new(
            pos.zobrist_hash(),
            best_score,
            best_move,
            self.state.age,
            depth,
            bound_so_far,
        );
        // TODO: eventually test that not overwriting PV nodes unless the depth is quite a bit greater gains
        // Store the results in the TT, always replacing the previous entry. Note that the TT move is only overwritten
        // if this node was an exact or fail high node or if there was a collision.
        if !(root && self.state.current_pv_num > 0)
            && !old_entry.is_some_and(|e| Self::should_not_replace(&e, &new_entry))
        {
            self.state.tt_mut().store(new_entry, ply);
        }

        Some(best_score)
    }

    fn should_not_replace(old: &TTEntry<Chessboard>, new: &TTEntry<Chessboard>) -> bool {
        new.age == old.age && new.depth < old.depth
    }

    fn update_continuation_hist(
        mov: ChessMove,
        prev_move: ChessMove,
        bonus: i32,
        color: ChessColor,
        pos: &Chessboard,
        hist: &mut ContHist,
        failed: &[ChessMove],
    ) {
        if prev_move == ChessMove::default() {
            return; // Ignore NMP null moves
        }
        hist.update(mov, prev_move, bonus, color);
        for disappointing in failed
            .iter()
            .dropping_back(1)
            .filter(|m| !m.is_tactical(pos))
        {
            hist.update(*disappointing, prev_move, -bonus, color);
        }
    }

    fn update_histories_and_killer(
        &mut self,
        pos: &Chessboard,
        mov: ChessMove,
        depth: isize,
        ply: usize,
        color: ChessColor,
    ) {
        let (before, [entry, ..]) = self.state.search_stack.split_at_mut(ply) else {
            unreachable!()
        };
        let bonus = (depth * cc::hist_depth_bonus()) as i32;
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
                Self::update_continuation_hist(
                    mov,
                    grandparent.last_tried_move(),
                    bonus,
                    color,
                    pos,
                    &mut self.state.custom.follow_up_move_hist,
                    &entry.tried_moves,
                );
            }
        }
    }

    /// Search only "tactical" moves to quieten down the position before calling eval
    fn qsearch(&mut self, pos: Chessboard, mut alpha: Score, beta: Score, ply: usize) -> Score {
        self.state.statistics.count_node_started(Qsearch);
        // updating seldepth only in qsearch meaningfully increased performance and was even measurable in a [0, 10] SPRT.
        self.state.atomic().update_seldepth(ply);
        // The stand pat check. Since we're not looking at all moves, it's very likely that there's a move we didn't
        // look at that doesn't make our position worse, so we don't want to assume that we have to play a capture.
        let mut best_score;
        let mut bound_so_far = FailLow;

        // see main search, store an invalid random move in the TT entry if all moves failed low.
        let mut best_move = ChessMove::default();

        // Don't do TT cutoffs with alpha already raised by the stand pat check, because that relies on the null move observation.
        // But if there's a TT entry from normal search that's worse than the stand pat score, we should trust that more.
        let old_entry = self.state.tt().load::<Chessboard>(pos.zobrist_hash(), ply);
        if let Some(tt_entry) = old_entry {
            debug_assert_eq!(tt_entry.hash, pos.zobrist_hash());
            let bound = tt_entry.bound();
            // depth 0 drops immediately to qsearch, so a depth 0 entry always comes from qsearch.
            // However, if we've already done qsearch on this position, we can just re-use the result,
            // so there is no point in checking the depth at all
            if (bound == NodeType::lower_bound() && tt_entry.score() >= beta)
                || (bound == NodeType::upper_bound() && tt_entry.score() <= alpha)
                || bound == Exact
            {
                self.state.statistics.tt_cutoff(Qsearch, bound);
                return tt_entry.score();
            }
            best_score = self.eval(pos, ply);
            // If the TT score is an upper bound, it can't be worse than the stand pat score unless it's from a regular
            // search entry, i.e. depth is greater than 0.
            if bound == FailLow && tt_entry.score() < best_score {
                debug_assert!(tt_entry.depth > 0);
            }
            // even though qsearch never checks for game over conditions, it's still possible for it to load a checkmate score
            // and propagate that up to a qsearch parent node, where it gets saved with a depth of 0, so game over scores
            // with a depth of 0 in the TT are possible
            // exact scores should have already caused a cutoff
            // TODO: Removing the `&& !tt_entry.score.is_game_over_score()` condition here and in `negamax` *failed* a
            // nonregression SPRT with `[-7, 0]` bounds even though I don't know why, and those conditions make it fail
            // the re-search test case. So the conditions are still disabled for now,
            // test reintroducing them at some point in the future after I have TT aging!
            if (bound == NodeType::lower_bound() && tt_entry.score() >= best_score)
                || (bound == NodeType::upper_bound() && tt_entry.score() <= best_score)
            {
                best_score = tt_entry.score();
            };
            if let Some(mov) = tt_entry.mov.check_pseudolegal(&pos) {
                best_move = mov;
            }
        } else {
            best_score = self.eval(pos, ply);
        }
        // Saving to the TT is probably unnecessary since the score is either from the TT or just the static eval,
        // which is not very valuable. Also, the fact that there's no best move might have unfortunate interactions with
        // IIR, because it will make this fail-high node appear like a fail-low node. TODO: Test regardless, but probably
        // only after aging
        if best_score >= beta {
            return best_score;
        }
        // TODO: Set stand pat to SCORE_LOST when in check, generate evasions?
        if best_score > alpha {
            bound_so_far = Exact;
            alpha = best_score;
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
            let Some(new_pos) = pos.make_move(mov) else {
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
            // even if the child score came from a TT entry with depth > 0, we don't trust this node any more than now
            // because we haven't looked at all nodes
            if score >= beta {
                bound_so_far = FailHigh;
                break;
            }
        }
        self.state
            .statistics
            .count_complete_node(Qsearch, bound_so_far, 0, ply, children_visited);

        let tt_entry: TTEntry<Chessboard> = TTEntry::new(
            pos.zobrist_hash(),
            best_score,
            best_move,
            self.state.age,
            0,
            bound_so_far,
        );
        if !old_entry.is_some_and(|e| Self::should_not_replace(&e, &tt_entry)) {
            self.state.tt_mut().store(tt_entry, ply);
        }
        best_score
    }

    fn eval(&mut self, pos: Chessboard, ply: usize) -> Score {
        let res = if ply == 0 {
            self.eval.eval(&pos, 0)
        } else {
            let old_pos = &self.state.search_stack[ply - 1].pos;
            let mov = &self.state.search_stack[ply - 1].last_tried_move();
            self.eval.eval_incremental(old_pos, *mov, &pos, ply)
        };
        debug_assert!(
            !res.is_won_or_lost(),
            "{res} {0} {1}, {pos}",
            res.0,
            self.eval.eval(&pos, ply)
        );
        res
    }

    fn record_pos(&mut self, pos: Chessboard, eval: Score, ply: usize) {
        self.state.search_stack[ply].pos = pos;
        self.state.search_stack[ply].eval = eval;
        self.state.search_stack[ply].tried_moves.clear();
    }

    fn record_move(&mut self, mov: ChessMove, old_pos: Chessboard, ply: usize, typ: SearchType) {
        self.state.atomic().count_node();
        self.state.params.history.push(&old_pos);
        self.state.search_stack[ply].tried_moves.push(mov);
        self.state.statistics.count_legal_make_move(typ);
    }

    fn undo_move(&mut self) {
        self.state.params.history.pop();
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
    fn score_move(&self, mov: ChessMove, state: &CapsState) -> MoveScore {
        // The move list is iterated backwards, which is why better moves get higher scores
        // No need to check against the TT move because that's already handled by the move picker
        if mov == state.search_stack[self.ply].killer {
            KILLER_SCORE
        } else if !mov.is_tactical(&self.board) {
            let countermove_score = if self.ply > 0 {
                let prev_move = state.search_stack[self.ply - 1].last_tried_move();
                state
                    .custom
                    .countermove_hist
                    .score(mov, prev_move, self.board.active_player())
            } else {
                0
            };
            let follow_up_score = if self.ply > 1 {
                let prev_move = state.search_stack[self.ply - 2].last_tried_move();
                state
                    .custom
                    .follow_up_move_hist
                    .score(mov, prev_move, self.board.active_player())
            } else {
                0
            };
            MoveScore(
                state.custom.history[mov.from_to_square()]
                    + countermove_score
                    + follow_up_score / 2,
            )
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
    use gears::general::board::Strictness::{Relaxed, Strict};
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
                let res = engine
                    .search_with_new_tt(board, SearchLimit::depth(Depth::new_unchecked(depth)));
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
            let pos = Chessboard::from_fen(fen, Strict).unwrap();
            let mut engine = Caps::for_eval::<PistonEval>();
            let res = engine
                .search_with_new_tt(pos, SearchLimit::nodes(NodesLimit::new(30_000).unwrap()));
            assert!(res.score.is_some_and(|score| score > Score(min)));
            assert!(res.score.is_some_and(|score| score < Score(max)));
        }
    }

    #[test]
    fn lucena_test() {
        let pos = Chessboard::from_name("lucena").unwrap();
        let mut engine = Caps::for_eval::<PistonEval>();
        let res = engine.search_with_new_tt(pos, SearchLimit::depth(Depth::new_unchecked(7)));
        // TODO: More aggressive bound once the engine is stronger
        assert!(res.score.unwrap() >= Score(200));
    }

    #[test]
    fn philidor_test() {
        let pos = Chessboard::from_name("philidor").unwrap();
        let mut engine = Caps::for_eval::<LiTEval>();
        let res =
            engine.search_with_new_tt(pos, SearchLimit::nodes(NodesLimit::new(50_000).unwrap()));
        assert!(res.score.unwrap().abs() <= Score(200));
    }

    #[test]
    fn kiwipete_test() {
        let pos = Chessboard::from_name("kiwipete").unwrap();
        let mut engine = Caps::for_eval::<LiTEval>();
        let res =
            engine.search_with_new_tt(pos, SearchLimit::nodes(NodesLimit::new(12_345).unwrap()));
        let score = res.score.unwrap();
        assert!(score.abs() <= Score(64), "{score}");
        assert!(
            [
                ChessMove::from_compact_text("e2a6", &pos).unwrap(),
                ChessMove::from_compact_text("d5e6", &pos).unwrap()
            ]
            .contains(&res.chosen_move),
            "{}",
            res.chosen_move
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
    }

    // TODO: Eventually, make sure that GAPS also passed this
    fn depth_1_nodes_test(mut engine: Caps, tt: TT) {
        for pos in Chessboard::bench_positions() {
            let res = engine.search_with_tt(pos, SearchLimit::depth_(1), tt.clone());
            if pos.legal_moves_slow().is_empty() {
                continue;
            }
            let root_entry = tt.load(pos.zobrist_hash(), 0).unwrap();
            assert!(root_entry.depth <= 2); // possible extensions
            assert_eq!(root_entry.bound(), Exact);
            assert!(root_entry.mov.check_legal(&pos).is_some());
            let moves = pos.legal_moves_slow();
            assert!(engine.state.uci_nodes() as usize >= moves.len()); // >= because of extensions
            for m in moves {
                let new_pos = pos.make_move(m).unwrap();
                let entry = tt.load::<Chessboard>(new_pos.zobrist_hash(), 0);
                let Some(entry) = entry else {
                    continue; // it's possible that a position is not in the TT because qsearch didn't save it
                };
                assert!(entry.depth <= 1);
                assert!(-entry.score <= root_entry.score);
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
        assert_eq!(
            res.chosen_move,
            ChessMove::from_compact_text("f3g3", &pos).unwrap()
        );
        assert_eq!(caps.state.depth().get(), 1);
        assert!(caps.state.uci_nodes() <= 1000); // might be a bit more than 1 because of check extensions
    }

    #[test]
    fn mate_research_test() {
        let pos = Chessboard::from_fen("k7/3B4/4N3/K7/8/8/8/8 w - - 16 9", Strict).unwrap();
        let mut caps = Caps::for_eval::<LiTEval>();
        let limit = SearchLimit::mate_in_moves(5);
        let res = caps.search_with_new_tt(pos, limit);
        assert!(res.score.unwrap().is_game_won_score());
        let nodes = caps.search_state().uci_nodes();
        let tt = caps.search_state().tt().clone();
        // Don't clear the internal state
        let second_search = caps.search_with_tt(pos, limit, tt.clone());
        assert!(second_search.score.unwrap().is_game_won_score());
        let second_search_nodes = caps.search_state().uci_nodes();
        assert!(
            second_search_nodes * 2 < nodes,
            "{second_search_nodes} {nodes}"
        );
        let d3 = SearchLimit::depth(Depth::new_unchecked(3));
        let d3_search = caps.search_with_tt(pos, d3, tt.clone());
        assert!(
            d3_search.score.unwrap().is_game_won_score(),
            "{}",
            d3_search.score.unwrap().0
        );
        let d3_nodes = caps.search_state().uci_nodes();
        caps.forget();
        assert_eq!(caps.search_state().uci_nodes(), 0);
        let fresh_d3_search = caps.search_with_new_tt(pos, d3);
        assert!(
            !fresh_d3_search.score.unwrap().is_won_or_lost(),
            "{}",
            fresh_d3_search.score.unwrap().0
        );
        let fresh_d3_nodes = caps.search_state().uci_nodes();
        assert!(
            fresh_d3_nodes > d3_nodes + d3_nodes / 4,
            "{fresh_d3_nodes} {d3_nodes}"
        );
        caps.forget();
        _ = caps.search_with_new_tt(pos, d3);
        assert_eq!(caps.search_state().uci_nodes(), fresh_d3_nodes);
    }

    #[test]
    #[cfg(not(debug_assertions))]
    /// puzzles that are reasonably challenging for most humans, but shouldn't be too difficult for the engine
    fn mate_test() {
        let fens = [
            ("8/5K2/4N2k/2B5/5pP1/1np2n2/1p6/r2R4 w - - 0 1", "d1d5", 5),
            (
                "5rk1/r5p1/2b2p2/3q1N2/6Q1/3B2P1/5P2/6KR w - - 0 1",
                "f5h6",
                5,
            ),
            (
                "2rk2nr/R1pnp3/5b2/5P2/BpPN1Q2/pPq5/P7/1K4R1 w - - 0 1",
                "f4c7",
                6,
            ),
            ("k2r3r/PR6/1K6/3R4/8/5np1/B6p/8 w - - 0 1", "d5d8", 6),
            (
                "3n3R/8/3p1pp1/r2bk3/8/4NPP1/p3P1KP/1r1R4 w - - 0 1",
                "h8e8",
                6,
            ),
            ("7K/k7/p1R5/4N1q1/8/6rb/5r2/1R6 w - - 0 1", "c6c7", 4),
            (
                "rkr5/3n1p2/1pp1b3/NP4p1/3PPn1p/QN1B1Pq1/2P5/R6K w - - 0 1",
                "a5c6",
                7,
            ),
            ("1kr5/4R3/pP6/1n2N3/3p4/2p5/1r6/4K2R w K - 0 1", "h1h8", 7),
            (
                "1k6/1bpQN3/1p6/p7/6p1/2NP1nP1/5PK1/4q3 w - - 0 1",
                "d7d8",
                8,
            ),
            (
                "1k4r1/pb1p4/1p1P4/1P3r1p/1N2Q3/6Pq/4BP1P/4R1K1 w - - 0 1",
                "b4a6",
                10,
            ),
            ("rk6/p1r3p1/P3B1Kp/1p2B3/8/8/8/8 w - - 0 1", "e6d7", 5),
        ];
        for (fen, best_move, num_moves) in fens {
            let pos = Chessboard::from_fen(fen, Relaxed).unwrap();
            let mut engine = Caps::for_eval::<LiTEval>();
            let limit = SearchLimit::mate_in_moves(num_moves);
            let res = engine.search_with_new_tt(pos, limit);
            let score = res.score.unwrap();
            println!(
                "chosen move {0}, fen {1}, depth {2}, time {3}ms",
                res.chosen_move.extended_formatter(pos),
                pos.as_fen(),
                engine.state.depth(),
                engine.state.start_time.elapsed().as_millis()
            );
            assert!(score.is_game_won_score());
            assert_eq!(res.chosen_move.to_string(), best_move);
        }
    }
}
