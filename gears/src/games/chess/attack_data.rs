/*
 *  Gears, a collection of board games.
 *  Copyright (C) 2024 ToTheAnd
 *
 *  Gears is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Gears is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Gears. If not, see <https://www.gnu.org/licenses/>.
 */
use crate::games::chess::pieces::ChessPieceType::*;
use crate::games::chess::pieces::{NUM_CHESS_PIECES, NUM_COLORS};
use crate::games::chess::ChessColor::*;
use crate::games::chess::{ChessColor, Chessboard, SliderMove};
use crate::general::bitboards::chess::ChessBitboard;
use crate::general::board::ExternalData;
use arrayvec::ArrayVec;

#[derive(Debug, Default)]
pub struct AttacksForColor {
    pub all: ChessBitboard,
    pub sliders: ArrayVec<ChessBitboard, { 2 + 2 + 1 + 8 }>,
}

#[derive(Debug, Default)]
pub struct Attacks {
    for_color: [AttacksForColor; NUM_COLORS],
    /// Bitboard of all pieces that are putting a king in check.
    /// Due to the rules of chess, all of these pieces must belong to the inactive player.
    pub checkers: ChessBitboard,
}

impl ExternalData<Chessboard> for Attacks {
    fn check_initialized(&self) -> Option<&Self> {
        if self.for_color[0].sliders.is_empty() {
            None
        } else {
            Some(&self)
        }
    }

    fn init_manually(pos: &Chessboard) -> Self {
        let mut res = Self::default();
        res.generate_attack_data(pos);
        res
    }
}

impl Attacks {
    pub fn attacks_for(&self, color: ChessColor) -> ChessBitboard {
        self.for_color[color as usize].all
    }
    pub fn all_attacks(&self) -> ChessBitboard {
        self.attacks_for(White) | self.attacks_for(Black)
    }
    pub fn push_bitboard(&mut self, color: ChessColor, bitboard: ChessBitboard) {
        self.for_color[color as usize].sliders.push(bitboard);
    }
    pub fn set_attacks_for(&mut self, color: ChessColor, attacks: ChessBitboard) {
        self.for_color[color as usize].all = attacks;
    }
    pub fn for_color(&self, color: ChessColor) -> &AttacksForColor {
        &self.for_color[color as usize]
    }

    // compute all squares where a hypothetical piece of `!color` would put the `color` king in check
    pub fn compute_checking_squares(
        pos: &Chessboard,
        color: ChessColor,
    ) -> [ChessBitboard; NUM_CHESS_PIECES] {
        let mut result = [ChessBitboard::default(); NUM_CHESS_PIECES];
        let square = pos.king_square(color);
        // we are in check from a pawn on square x if our own pawn on our king square would attack x.
        result[Pawn as usize] = Chessboard::single_pawn_captures(color, square);
        result[Knight as usize] = Chessboard::knight_attacks_from(square);
        result[Bishop as usize] = pos.slider_attacks_from(square, SliderMove::Bishop, square.bb());
        result[Rook as usize] = pos.slider_attacks_from(square, SliderMove::Rook, square.bb());
        result[Queen as usize] = result[Rook as usize] | result[Bishop as usize];
        // Kings can never give check
        result
    }

    /// Not currently called in the actual eval because it throws away some information, but useful
    /// for running perft with attacks, to check that it's not bugged.
    pub fn generate_attacks_for(&mut self, pos: &Chessboard, color: ChessColor) {
        let mut all_attacks = pos.colored_piece_bb(color, Pawn).pawn_attacks(color);
        for slider in [Bishop, Rook, Queen] {
            for square in pos.colored_piece_bb(color, slider).ones() {
                let attacks = pos.attacks_no_castle_or_pawn_push(square, slider, color);
                self.push_bitboard(color, attacks);
                all_attacks |= attacks;
            }
        }
        for square in pos.colored_piece_bb(color, Knight).ones() {
            all_attacks |= pos.attacks_no_castle_or_pawn_push(square, Knight, color);
        }
        all_attacks |= pos.attacks_no_castle_or_pawn_push(pos.king_square(color), King, color);
    }

    /// Calls [`generate_attacks_for`] for white and black.
    pub fn generate_attack_data(&mut self, pos: &Chessboard) {
        self.generate_attacks_for(pos, White);
        self.generate_attacks_for(pos, Black);
    }
}
