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

use crate::PlayerResult;
use derive_more::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use num::ToPrimitive;
use std::ops::Div;
use std::usize;

/// Anything related to search that is also used by `monitors`, and therefore doesn't belong in `motors`.

// TODO: Turn this into an enum that can also represent a win in n plies (and maybe a draw?)
#[derive(
    Default,
    Debug,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Copy,
    Clone,
    Add,
    Sub,
    Neg,
    AddAssign,
    SubAssign,
    derive_more::Display,
)]
pub struct Score(pub i32);

impl Add<i32> for Score {
    type Output = Score;

    fn add(self, rhs: i32) -> Self::Output {
        Score(self.0 + rhs)
    }
}

impl Sub<i32> for Score {
    type Output = Score;

    fn sub(self, rhs: i32) -> Self::Output {
        Score(self.0 - rhs)
    }
}

impl Mul<i32> for Score {
    type Output = Score;

    fn mul(self, rhs: i32) -> Self::Output {
        Score(self.0 * rhs)
    }
}

impl Div<i32> for Score {
    type Output = Score;

    fn div(self, rhs: i32) -> Self::Output {
        Score(self.0 / rhs)
    }
}

impl Score {
    pub fn is_game_won_score(self) -> bool {
        self >= MIN_SCORE_WON
    }
    pub fn is_game_lost_score(self) -> bool {
        self <= MAX_SCORE_LOST
    }
    pub fn is_game_over_score(self) -> bool {
        self.is_game_won_score() || self.is_game_lost_score()
    }
    /// Returns a negative number of plies if the game is lost
    pub fn plies_until_game_won(self) -> Option<isize> {
        if self.is_game_won_score() {
            Some((SCORE_WON - self).0 as isize)
        } else if self.is_game_lost_score() {
            Some((SCORE_LOST - self).0 as isize)
        } else {
            None
        }
    }
    /// Returns a negative number if the game is lost
    pub fn moves_until_game_won(self) -> Option<isize> {
        self.plies_until_game_won()
            .map(|n| (n as f32 / 2f32).ceil() as isize)
    }

    pub fn plies_until_game_over(self) -> Option<isize> {
        self.plies_until_game_won().map(|x| x.abs())
    }

    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }
}

/// `SCORE_WON` and `SCORE_LOST` need to fit into 16 bits for the tapered score to work
pub const SCORE_LOST: Score = Score(-31_000);
pub const SCORE_WON: Score = Score(31_000);
pub const SCORE_TIME_UP: Score = Score(SCORE_WON.0 + 1000);
// can't use + directly because derive_more's + isn't `const`
pub const MIN_SCORE_WON: Score = Score(SCORE_WON.0 - 1000);
pub const MAX_SCORE_LOST: Score = Score(SCORE_LOST.0 + 1000);
pub const MIN_NORMAL_SCORE: Score = Score(MAX_SCORE_LOST.0 + 1);
pub const MAX_NORMAL_SCORE: Score = Score(MIN_SCORE_WON.0 - 1);
pub const NO_SCORE_YET: Score = Score(SCORE_LOST.0 - 100);

pub fn game_result_to_score(res: PlayerResult, ply: usize) -> Score {
    match res {
        PlayerResult::Win => SCORE_WON - ply as i32,
        PlayerResult::Lose => SCORE_LOST + ply as i32,
        PlayerResult::Draw => Score(0),
    }
}

pub fn is_valid_score(score: i32) -> bool {
    score >= SCORE_LOST.0 && score <= SCORE_WON.0
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Add, AddAssign)]
pub struct TaperedScore(u32);

impl TaperedScore {
    pub fn new(mg: i16, eg: i16) -> Self {
        debug_assert!(is_valid_score(mg as i32));
        debug_assert!(is_valid_score(eg as i32));
        Self(((mg as u32) << 16) | (eg as u16) as u32)
    }
    pub fn mg(self) -> Score {
        // cast to i32 before the shift to get an arithmetic shift (i.e. sign extending)
        Score((self.0 as i32) >> 16)
    }

    pub fn eg(self) -> Score {
        // cast to i16 first to get a sign extending conversion
        Score(self.0 as i16 as i32)
    }
}

/// Same as [`TaperedScore::new`], but has a shorter name
pub fn taper(mg: i16, eg: i16) -> TaperedScore {
    TaperedScore::new(mg, eg)
}

impl Neg for TaperedScore {
    type Output = Self;

    fn neg(self) -> Self::Output {
        let inverted = !self.0;
        Self((inverted.wrapping_add(1) & 0xffff).wrapping_add(1 << 16))
    }
}

impl Mul<usize> for TaperedScore {
    type Output = TaperedScore;

    fn mul(mut self, rhs: usize) -> Self::Output {
        self *= rhs;
        self
    }
}

impl MulAssign<usize> for TaperedScore {
    fn mul_assign(&mut self, rhs: usize) {
        debug_assert!(is_valid_score(
            (self.mg().0 as isize * rhs.to_isize().unwrap())
                .to_i32()
                .unwrap()
        ));
        debug_assert!(is_valid_score(
            (self.eg().0 as isize * rhs.to_isize().unwrap())
                .to_i32()
                .unwrap()
        ));
        self.0 *= rhs as u32;
    }
}
