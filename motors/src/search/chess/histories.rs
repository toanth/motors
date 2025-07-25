/*
 *  Motors, a collection of board game engines.
 *  Copyright (C) 2024 ToTheAnd
 *
 *  Motors is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Motors is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Motors. If not, see <https://www.gnu.org/licenses/>.
 */
use crate::io::ugi_output::{color_for_score, score_gradient};
use crate::search::MoveScore;
use crate::search::chess::caps_values::cc;
use crate::search::chess::caps_values::cc::corrhist_max;
use derive_more::{Deref, DerefMut, Index, IndexMut};
use gears::colored::Colorize;
use gears::games::chess::moves::ChessMove;
use gears::games::chess::moves::ChessMoveFlags::NormalMove;
use gears::games::chess::pieces::NUM_CHESS_PIECES;
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::chess::{ChessColor, Chessboard};
use gears::games::{Color, NUM_COLORS};
use gears::general::bitboards::chessboard::ChessBitboard;
use gears::general::bitboards::{Bitboard, RawBitboard};
use gears::general::board::Board;
use gears::general::moves::Move;
use gears::itertools::Itertools;
use gears::output::OutputOpts;
use gears::output::text_output::AdaptFormatter;
use gears::score::{MAX_NORMAL_SCORE, MIN_NORMAL_SCORE, Score, ScoreT};

pub(super) type HistScoreT = i16;

pub(super) const HIST_DIVISOR: HistScoreT = 1024;

/// Updates the history using the History Gravity technique,
/// which keeps history scores from growing arbitrarily large and scales the bonus/malus depending on how
/// "unexpected" they are, i.e. by how much they differ from the current history scores.
fn update_history_score(entry: &mut HistScoreT, bonus: HistScoreT) {
    let bonus = bonus.clamp(-HIST_DIVISOR, HIST_DIVISOR);
    let bonus = bonus as i32;
    let e = *entry as i32;
    let bonus = (bonus - bonus.abs() * e / HIST_DIVISOR as i32) as i16; // bonus can also be negative
    *entry += bonus;
}

/// Quiet History Heuristic: Give bonuses to quiet moves that causes a beta cutoff a maluses to quiet moves that were tried
/// but didn't cause a beta cutoff. Order all non-TT non-killer moves based on that (as well as based on the continuation
/// history)
#[derive(Debug, Clone, Deref, DerefMut, Index, IndexMut)]
pub(super) struct HistoryHeuristic(Box<[[HistScoreT; 64 * 64]; 4]>);

impl HistoryHeuristic {
    pub(super) fn update(&mut self, mov: ChessMove, threats: ChessBitboard, bonus: HistScoreT) {
        let mut threats_idx = threats.is_bit_set(mov.src_square()) as usize;
        threats_idx = threats_idx * 2 + threats.is_bit_set(mov.dest_square()) as usize;
        update_history_score(&mut self[threats_idx][mov.from_to_square()], bonus);
    }

    pub(super) fn score(&self, mov: ChessMove, threats: ChessBitboard) -> isize {
        let mut threats_idx = threats.is_bit_set(mov.src_square()) as usize;
        threats_idx = threats_idx * 2 + threats.is_bit_set(mov.dest_square()) as usize;
        self[threats_idx][mov.from_to_square()] as isize
    }
}

impl Default for HistoryHeuristic {
    fn default() -> Self {
        HistoryHeuristic(Box::new([[0; 64 * 64]; 4]))
    }
}

/// Capture History Heuristic: Same as quiet history heuristic, but for captures.
#[derive(Debug, Clone)]
pub(super) struct CaptHist(Box<[[[[HistScoreT; 64]; 6]; 2]; NUM_COLORS]>);

impl CaptHist {
    pub(super) fn update(&mut self, mov: ChessMove, pos: &Chessboard, bonus: HistScoreT) {
        let us = pos.active_player();
        let defended = pos.threats().is_bit_set_at(mov.dest_square().bb_idx()) as usize;
        let entry = &mut self.0[us][defended][mov.piece_type(pos) as usize][mov.dest_square().bb_idx()];
        update_history_score(entry, bonus);
    }

    pub(super) fn get(&self, mov: ChessMove, pos: &Chessboard) -> MoveScore {
        let us = pos.active_player();
        let defended = pos.threats().is_bit_set_at(mov.dest_square().bb_idx()) as usize;
        MoveScore(self.0[us][defended][mov.piece_type(pos) as usize][mov.dest_square().bb_idx()])
    }

    pub(super) fn reset(&mut self) {
        for value in self.0.iter_mut().flatten().flatten().flatten() {
            *value = 0;
        }
    }
}

impl Default for CaptHist {
    fn default() -> Self {
        Self(Box::new([[[[0; 64]; 6]; 2]; NUM_COLORS]))
    }
}

/// Continuation history.
/// Used for Countermove History (CMH, 1 ply ago) and Follow-up Move History (FMH, 2 plies ago).
/// Unlike the main quiet history heuristic, this in indexed by the previous piece, previous target square,
/// current piece, current target square, and color.
#[derive(Debug, Clone, Deref, DerefMut, Index, IndexMut)]
pub(super) struct ContHist(Vec<HistScoreT>); // Can't store this on the stack because it's too large.

impl ContHist {
    fn idx(mov: ChessMove, pos: &Chessboard, prev_move: ChessMove, prev_pos: &Chessboard) -> usize {
        let color = pos.active_player();
        debug_assert!(mov.dest_square().bb_idx() < 64);
        debug_assert!((mov.piece_type(pos) as usize) < 6);
        debug_assert!(prev_move.dest_square().bb_idx() < 64);
        debug_assert!((prev_move.piece_type(prev_pos) as usize) < 6);
        (mov.piece_type(pos) as usize * 64 + mov.dest_square().bb_idx())
            + (prev_move.piece_type(prev_pos) as usize * 64 + prev_move.dest_square().bb_idx()) * 64 * 6
            + color as usize * 64 * 6 * 64 * 6
    }
    pub(super) fn update(
        &mut self,
        mov: ChessMove,
        pos: &Chessboard,
        prev_mov: ChessMove,
        prev_pos: &Chessboard,
        bonus: HistScoreT,
    ) {
        let entry = &mut self[Self::idx(mov, pos, prev_mov, prev_pos)];
        update_history_score(entry, bonus);
    }

    pub(super) fn score(&self, mov: ChessMove, pos: &Chessboard, prev_move: ChessMove, prev_pos: &Chessboard) -> isize {
        if prev_move.is_null() {
            return 0;
        }
        self[Self::idx(mov, pos, prev_move, prev_pos)] as isize
    }
}

impl Default for ContHist {
    fn default() -> Self {
        ContHist(vec![0; 2 * 6 * 64 * 6 * 64])
    }
}

// See <https://www.chessprogramming.org/Static_Evaluation_Correction_History>

// Code adapted from Sirius
const CORRHIST_SIZE: usize = 1 << 14;

const MAX_CORRHIST_VAL: isize = i16::MAX as isize;

const CORRHIST_SCALE: isize = 256;

#[derive(Debug, Clone)]
pub(super) struct CorrHist {
    pawns: Box<[[ScoreT; CORRHIST_SIZE]; NUM_COLORS]>,
    // the outer color index is the active player, the inner color is the color we're looking at
    nonpawns: Box<[[[ScoreT; NUM_COLORS]; CORRHIST_SIZE]; NUM_COLORS]>,
    minor: Box<[[ScoreT; CORRHIST_SIZE]; NUM_COLORS]>,
    continuation: Box<[[[ScoreT; NUM_CHESS_PIECES]; NUM_SQUARES]; NUM_COLORS]>,
}

impl Default for CorrHist {
    fn default() -> Self {
        CorrHist {
            pawns: Box::new([[0; CORRHIST_SIZE]; NUM_COLORS]),
            nonpawns: Box::new([[[0; NUM_COLORS]; CORRHIST_SIZE]; NUM_COLORS]),
            minor: Box::new([[0; CORRHIST_SIZE]; NUM_COLORS]),
            continuation: Box::new([[[0; NUM_CHESS_PIECES]; NUM_SQUARES]; NUM_COLORS]),
        }
    }
}

impl CorrHist {
    fn update_entry(entry: &mut ScoreT, weight: isize, bonus: isize) {
        let val = *entry as isize;
        // Idea of clamping the max update from Simbelmyne
        let new_val = ((val * (CORRHIST_SCALE - weight) + bonus * weight) / CORRHIST_SCALE)
            .clamp(val - MAX_CORRHIST_VAL / 4, val + MAX_CORRHIST_VAL / 4)
            .clamp(-MAX_CORRHIST_VAL, MAX_CORRHIST_VAL);
        *entry = new_val as ScoreT;
    }

    pub(super) fn reset(&mut self) {
        for value in self.pawns.iter_mut().flatten() {
            *value = 0;
        }
        for value in self.nonpawns.iter_mut().flatten().flatten() {
            *value = 0;
        }
        for value in self.continuation.iter_mut().flatten().flatten() {
            *value = 0;
        }
        for value in self.minor.iter_mut().flatten() {
            *value = 0;
        }
    }

    pub(super) fn update(
        &mut self,
        pos: &Chessboard,
        continued: Option<(ChessMove, &Chessboard)>,
        depth: isize,
        eval: Score,
        score: Score,
    ) {
        assert_eq!(depth % 128, 0); // TODO: Remove
        let color = pos.active_player();
        let weight = (cc::corrhist_offset() + depth).min(corrhist_max()) / 128;
        let bonus = (score - eval).0 as isize * CORRHIST_SCALE;
        let pawn_idx = pos.pawn_key().0 as usize % CORRHIST_SIZE;
        Self::update_entry(&mut self.pawns[color][pawn_idx], weight, bonus);
        let minor_idx = pos.minor_key().0 as usize % CORRHIST_SIZE;
        Self::update_entry(&mut self.minor[color][minor_idx], weight, bonus);
        for c in ChessColor::iter() {
            let nonpawn_idx = pos.nonpawn_key(c).0 as usize % CORRHIST_SIZE;
            Self::update_entry(&mut self.nonpawns[color][nonpawn_idx][c], weight, bonus);
        }
        if let Some((mov, pos)) = continued {
            let entry = &mut self.continuation[color][mov.dest_square().bb_idx()][mov.piece_type(pos) as usize];
            Self::update_entry(entry, weight, bonus);
        }
    }

    pub(super) fn correct(
        &mut self,
        pos: &Chessboard,
        continued: Option<(ChessMove, &Chessboard)>,
        raw: Score,
    ) -> Score {
        if raw.is_won_or_lost() {
            return raw;
        }
        let color = pos.active_player();
        let pawn_idx = pos.pawn_key().0 as usize % CORRHIST_SIZE;
        let mut correction = self.pawns[color][pawn_idx] as isize;
        let minor_idx = pos.minor_key().0 as usize % CORRHIST_SIZE;
        correction += self.minor[color][minor_idx] as isize;
        for c in ChessColor::iter() {
            let nonpawn_idx = pos.nonpawn_key(c).0 as usize % CORRHIST_SIZE;
            correction += self.nonpawns[color][nonpawn_idx][c] as isize * cc::nonpawn_corrhist_weight() / 1024;
        }
        if let Some((mov, pos)) = continued {
            correction += self.continuation[color][mov.dest_square().bb_idx()][mov.piece_type(pos) as usize] as isize
                * cc::contcorrhist_weight()
                / 1024;
        }
        let score = raw.0 as isize + correction / CORRHIST_SCALE;
        Score(score.clamp(MIN_NORMAL_SCORE.0 as isize, MAX_NORMAL_SCORE.0 as isize) as ScoreT)
    }
}

pub(super) fn write_single_hist_table(table: &HistoryHeuristic, pos: &Chessboard, flip: bool) -> String {
    let show_square = |from: ChessSquare| {
        let sum: i32 = ChessSquare::iter()
            .map(|to| {
                let mov =
                    if flip { ChessMove::new(to, from, NormalMove) } else { ChessMove::new(from, to, NormalMove) };
                table.score(mov, pos.threats()) as i32
            })
            .sum();
        sum as f64 / 64.0
    };
    let as_nums = ChessSquare::iter()
        .map(|sq| {
            let score = show_square(sq);
            format!("{score:^7.1}").color(color_for_score(Score((score * 4.0) as ScoreT), &score_gradient()))
        })
        .collect_vec();

    let formatter = Chessboard::default().pretty_formatter(None, None, OutputOpts::default());
    let mut formatter = AdaptFormatter {
        underlying: formatter,
        color_frame: Box::new(|_, col| col),
        display_piece: Box::new(move |sq, _, _| as_nums[sq.bb_idx()].to_string()),
        horizontal_spacer_interval: None,
        vertical_spacer_interval: None,
        square_width: Some(7),
    };
    let text =
        if flip { "Main History Destination Square:\n" } else { "Main History Source Square:\n" }.bold().to_string();
    text + &Chessboard::default().display_pretty(&mut formatter)
}
