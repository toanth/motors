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
use gears::games::ataxx::{AtaxxBoard, AtaxxColor};
use gears::general::bitboards::RawBitboard;
use gears::general::common::StaticallyNamedEntity;
use gears::score::{Score, ScoreT};
use std::fmt::Display;

/// `BAtE` (Basic Ataxx Eval) is a piece-counting eval for ataxx
#[derive(Debug, Copy, Clone, Default)]
pub struct Bate {}

impl StaticallyNamedEntity for Bate {
    fn static_short_name() -> impl Display
    where
        Self: Sized,
    {
        "bate"
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        "BAte: Basic Ataxx Eval".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "A simple, classical evaluation for Ataxx, which counts the number of pieces per side".to_string()
    }
}

impl Eval<AtaxxBoard> for Bate {
    fn eval(&mut self, pos: &AtaxxBoard, _ply: usize, _engine: AtaxxColor) -> Score {
        let diff = pos.active_bb().num_ones() as ScoreT - pos.inactive_bb().num_ones() as ScoreT;
        // multiply by 10 so that scores are somewhat more spread out, similar to scores in other games
        Score(diff * 10)
    }
}
