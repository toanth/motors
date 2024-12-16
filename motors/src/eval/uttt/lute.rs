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

use crate::eval::Eval;
use gears::games::uttt::{UtttBoard, UtttSubSquare};
use gears::general::bitboards::RawBitboard;
use gears::general::board::{Board, BoardHelpers};
use gears::general::common::StaticallyNamedEntity;
use gears::score::Score;
use std::fmt::Display;

#[derive(Debug, Default, Copy, Clone)]
pub struct Lute {}

impl StaticallyNamedEntity for Lute {
    fn static_short_name() -> impl Display
    where
        Self: Sized,
    {
        "lute"
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        "LUTE: Linear Ultimate Tic-tac-toe Eval".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "A simple, classical eval function for Ultimate Tic-Tac-Toe, which uses Piece Square Tables"
            .to_string()
    }
}

const PSQT: [Score; 9] = [
    Score(20),
    Score(10),
    Score(20),
    Score(10),
    Score(30),
    Score(10),
    Score(20),
    Score(10),
    Score(20),
];

impl Eval<UtttBoard> for Lute {
    fn eval(&mut self, pos: &UtttBoard, _ply: usize) -> Score {
        let mut score = Score::default();
        for color in [pos.active_player(), pos.inactive_player()] {
            for sub_board in UtttSubSquare::iter() {
                if pos.is_sub_board_won(color, sub_board) {
                    score += PSQT[sub_board.bb_idx()] * 10;
                } else {
                    for sub_square in pos.sub_board(color, sub_board).one_indices() {
                        score += PSQT[sub_square];
                    }
                }
            }
            score = -score;
        }
        score
    }
}
