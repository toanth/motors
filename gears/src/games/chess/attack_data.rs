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
use crate::games::chess::pieces::NUM_COLORS;
use crate::games::chess::ChessColor;
use crate::games::chess::ChessColor::*;
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
}

impl ExternalData for Attacks {
    fn check_initialized(&self) -> Option<&Self> {
        if self.for_color[0].sliders.is_empty() {
            None
        } else {
            Some(&self)
        }
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
}
