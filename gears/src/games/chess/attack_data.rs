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
use crate::games::chess::pieces::{ChessPieceType, NUM_CHESS_PIECES, NUM_COLORS};
use crate::games::chess::ChessColor::*;
use crate::games::chess::{ChessColor, Chessboard, SliderMove};
use crate::general::bitboards::chess::{ChessBitboard, RAYS_EXCLUSIVE};
use crate::general::bitboards::RawBitboard;
use crate::general::board::{Board, ExternalData};
use arrayvec::ArrayVec;

#[derive(Debug, Default)]
pub struct AttacksForColor {
    pub all: ChessBitboard,
    pub sliders: ArrayVec<ChessBitboard, { 2 + 2 + 1 + 8 }>,
}

#[derive(Debug, Copy, Clone)]
pub struct CheckRes {
    pub checking_squares: [ChessBitboard; NUM_CHESS_PIECES],
    pub pinned: ChessBitboard,
}

#[derive(Debug, Default)]
pub struct Attacks {
    for_color: [AttacksForColor; NUM_COLORS],
    /// Bitboard of all pieces that are putting a king in check.
    /// Due to the rules of chess, all of these pieces must belong to the inactive player.
    pub checkers: ChessBitboard,
    /// This includes pinned pieces of both sides, but doesn't try to cover all cases:
    /// Whenever a bit is set, that piece is pinned, but there are some en passant edge cases where a piece is pinned
    /// even though the corresponding bit is not set.
    pub pinned: ChessBitboard,
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

    pub fn compute_pinned(pos: &Chessboard, color: ChessColor) -> ChessBitboard {
        let square = pos.king_square(color);
        let mut pinned = ChessBitboard::default();
        for slider in [Bishop, Rook] {
            let slider_move = if slider == Bishop {
                SliderMove::Bishop
            } else {
                SliderMove::Rook
            };
            let bb = pos.piece_bb(slider) | pos.piece_bb(Queen);
            let blockers = pos.colored_bb(!color);
            let potentially_pinning = pos.slider_attacks_from(square, slider_move, blockers) & bb;
            for piece in potentially_pinning.ones() {
                let ray = RAYS_EXCLUSIVE[square.bb_idx()][piece.bb_idx()];
                let blocking = ray & pos.colored_bb(color);
                if blocking.is_single_piece() {
                    pinned |= blocking;
                }
            }
        }
        pinned
    }

    // compute all squares where a hypothetical piece of `!color` would put the `color` king in check
    pub fn compute_checking_squares(pos: &Chessboard, color: ChessColor) -> CheckRes {
        let mut result = [ChessBitboard::default(); NUM_CHESS_PIECES];
        let square = pos.king_square(color);
        let occupied = pos.occupied_bb();
        // we are in check from a pawn on square x if our own pawn on our king square would attack x.
        result[Pawn as usize] = Chessboard::single_pawn_captures(color, square);
        result[Knight as usize] = Chessboard::knight_attacks_from(square);
        result[Bishop as usize] = pos.slider_attacks_from(square, SliderMove::Bishop, occupied);
        result[Rook as usize] = pos.slider_attacks_from(square, SliderMove::Rook, occupied);
        result[Queen as usize] = result[Rook as usize] | result[Bishop as usize];
        // Kings can never give check
        let pinned = Self::compute_pinned(pos, color);
        CheckRes {
            checking_squares: result,
            pinned,
        }
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

    pub fn gen_checkers(&mut self, pos: &Chessboard) {
        let check_info = Self::compute_checking_squares(pos, pos.active_player);
        for piece in ChessPieceType::pieces() {
            self.checkers |= check_info.checking_squares[piece as usize]
                & pos.colored_piece_bb(pos.inactive_player(), piece);
        }
        self.pinned |= check_info.pinned;
    }

    pub fn generate_attack_data(&mut self, pos: &Chessboard) {
        self.generate_attacks_for(pos, White);
        self.generate_attacks_for(pos, Black);
        self.gen_checkers(pos);
        self.pinned = Self::compute_pinned(pos, White) | Self::compute_pinned(pos, Black);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::games::chess::squares::ChessSquare;
    use crate::games::chess::{ChessMoveList, Chessboard};
    use crate::general::board::Board;
    use crate::general::perft::perft;
    use crate::search::Depth;
    use std::str::FromStr;

    #[test]
    fn simple_perft_test() {
        for pos in Chessboard::bench_positions() {
            let p1 = perft(Depth::try_new(3).unwrap(), pos, false);
            let p2 = perft(Depth::try_new(3).unwrap(), pos, true);
            assert_eq!(p1.depth, p2.depth);
            assert_eq!(p1.nodes, p2.nodes);
        }
    }

    #[test]
    fn pinned_test() {
        let pos = Chessboard::from_name("puzzle").unwrap();
        let data = Attacks::init_manually(&pos);
        assert_eq!(data.pinned, ChessSquare::from_str("c7").unwrap().bb());
        let pos = Chessboard::from_name("unusual")
            .unwrap()
            .flip_side_to_move()
            .unwrap();
        let data = Attacks::init_manually(&pos);
        assert_eq!(data.pinned, ChessSquare::from_str("d1").unwrap().bb());
    }

    #[test]
    #[should_panic]
    fn invalid_attack_data_test() {
        let pos = Chessboard::from_name("kiwipete").unwrap();
        let mut data = Attacks::init_manually(&pos);
        data.for_color[White as usize].sliders[0] ^= ChessBitboard::from_u64(!0);
        let mut list = ChessMoveList::default();
        pos.gen_pseudolegal(&mut list, Some(&data)); // might already panic
        assert_eq!(list.len(), pos.pseudolegal_moves().len());
    }

    #[test]
    #[should_panic]
    fn invalid_checker_test() {
        let pos = Chessboard::from_name("kiwipete").unwrap();
        let mut data = Attacks::init_manually(&pos);
        data.checkers |= pos.colored_piece_bb(Black, Queen); // set an invalid checkers bb
        let mut list = ChessMoveList::default();
        pos.gen_pseudolegal(&mut list, Some(&data)); // should panic
    }
}
