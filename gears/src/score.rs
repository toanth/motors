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

//! Anything related to search that is also used by `monitors`, and therefore doesn't belong in `motors`.

use crate::PlayerResult;
use derive_more::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use num::ToPrimitive;
use std::fmt::{Display, Formatter};
use std::ops::Div;

/// Valid scores fit into 16 bits, but it's possible to temporarily overflow that range with some operations,
/// e.g. when computing `score - previous_score`. So in order to avoid bugs related to that, simply use 32 bits.
pub type ScoreT = i32;

/// In some places, it's important to save space by using only the necessary 16 bits for a score.
pub type CompactScoreT = i16;

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
#[must_use]
pub struct Score(pub ScoreT);

impl Add<ScoreT> for Score {
    type Output = Score;

    fn add(self, rhs: ScoreT) -> Self::Output {
        Score(self.0 + rhs)
    }
}

impl Sub<ScoreT> for Score {
    type Output = Score;

    fn sub(self, rhs: ScoreT) -> Self::Output {
        Score(self.0 - rhs)
    }
}

impl Mul<ScoreT> for Score {
    type Output = Score;

    fn mul(self, rhs: ScoreT) -> Self::Output {
        Score(self.0 * rhs)
    }
}

impl Div<ScoreT> for Score {
    type Output = Score;

    fn div(self, rhs: ScoreT) -> Self::Output {
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
        self.plies_until_game_won().map(isize::abs)
    }

    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }

    pub fn verify_valid(self) -> Option<Self> {
        if (self <= SCORE_WON && self >= SCORE_LOST)
            || self == SCORE_TIME_UP
            || self == NO_SCORE_YET
        {
            Some(self)
        } else {
            None
        }
    }
}

/// `SCORE_WON` and `SCORE_LOST` need to fit into 16 bits for the tapered score to work,
/// and the open interval `(alpha, beta)` has to be able to contain them.
pub const MIN_ALPHA: Score = Score(-31_001);
pub const MAX_BETA: Score = Score(31_001);
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
        PlayerResult::Win => SCORE_WON - ply as ScoreT,
        PlayerResult::Lose => SCORE_LOST + ply as ScoreT,
        PlayerResult::Draw => Score(0),
    }
}

pub const fn is_valid_score(score: ScoreT) -> bool {
    score >= SCORE_LOST.0 && score <= SCORE_WON.0
}

/// Uses a SWAR (SIMD Within A Register) technique to store and manipulate middlegame and endgame scores
/// at the same time, by treating them as the lower and upper half of a single value.
/// This improves performance, which is especially important because the eval of a typical a/b engine is hot.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Add, AddAssign, Sub, SubAssign, Neg)]
#[must_use]
pub struct PhasedScore(ScoreT);

impl Display for PhasedScore {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({0}, {1})", self.mg(), self.eg())
    }
}

pub type PhaseType = isize;

const COMPACT_SCORE_BITS: usize = CompactScoreT::BITS as usize;

impl PhasedScore {
    // `const`, unlike `default`
    pub const fn zero() -> Self {
        PhasedScore(0)
    }
    pub const fn new(mg: CompactScoreT, eg: CompactScoreT) -> Self {
        debug_assert!(is_valid_score(mg as ScoreT));
        debug_assert!(is_valid_score(eg as ScoreT));
        Self(((mg as ScoreT) << COMPACT_SCORE_BITS) + eg as ScoreT)
    }
    pub const fn underlying(self) -> ScoreT {
        self.0
    }

    pub const fn mg(self) -> Score {
        // The eg score could have overflown into the mg score, so add (1 << 15) to undo that overflow
        // with another potential overflow
        Score(((self.0 + (1 << (COMPACT_SCORE_BITS - 1))) >> COMPACT_SCORE_BITS) as ScoreT)
    }

    pub const fn eg(self) -> Score {
        Score(self.underlying() as CompactScoreT as ScoreT)
    }

    pub fn taper(self, phase: PhaseType, max_phase: PhaseType) -> Score {
        Score(
            ((self.mg().0 as PhaseType * phase + self.eg().0 as PhaseType * (max_phase - phase))
                / max_phase) as ScoreT,
        )
    }
}

/// Same as [`PhasedScore::new`], but has a shorter name
pub const fn p(mg: CompactScoreT, eg: CompactScoreT) -> PhasedScore {
    PhasedScore::new(mg, eg)
}

impl Mul<usize> for PhasedScore {
    type Output = PhasedScore;

    fn mul(mut self, rhs: usize) -> Self::Output {
        self *= rhs;
        self
    }
}

impl MulAssign<usize> for PhasedScore {
    fn mul_assign(&mut self, rhs: usize) {
        debug_assert!(is_valid_score(
            (self.mg().0 as isize * rhs.to_isize().unwrap())
                .try_into()
                .unwrap()
        ));
        debug_assert!(is_valid_score(
            (self.eg().0 as isize * rhs.to_isize().unwrap())
                .try_into()
                .unwrap()
        ));
        self.0 *= rhs as ScoreT;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use rand::prelude::SliceRandom;
    use rand::thread_rng;

    #[test]
    fn tapered_test() {
        let mut v = vec![];
        for i in -200..123 {
            for j in -321..99 {
                v.push((i, j));
                let phased = p(i, j);
                assert_eq!(
                    phased.mg().0,
                    ScoreT::from(i),
                    "{0} {i} {j} -- {1:X}, {2} {3}",
                    phased.mg(),
                    phased.underlying(),
                    phased.underlying() >> 16,
                    phased.underlying() & 0xffff,
                );
                assert_eq!(
                    phased.eg().0,
                    ScoreT::from(j),
                    "{0} {i} {j} -- {1:X}, {2} {3}",
                    phased.mg(),
                    phased.underlying(),
                    phased.underlying() >> 16,
                    phased.underlying() & 0xffff,
                );
            }
        }
        v.shuffle(&mut thread_rng());
        for ((mg_a, eg_a), (mg_b, eg_b)) in v.iter().copied().tuple_windows() {
            let taper_a = p(mg_a, eg_a);
            let taper_b = p(mg_b, eg_b);
            let mg_a = Score(ScoreT::from(mg_a));
            let mg_b = Score(ScoreT::from(mg_b));
            let eg_a = Score(ScoreT::from(eg_a));
            let eg_b = Score(ScoreT::from(eg_b));
            let sum = taper_a + taper_b;
            assert_eq!(
                sum.mg(),
                mg_a + mg_b,
                "{0} {mg_a} {mg_b} -- {1}",
                sum.mg(),
                sum.0
            );
            assert_eq!(
                sum.eg(),
                eg_a + eg_b,
                "{0} {eg_a} {eg_b} -- {1}",
                sum.eg(),
                sum.0
            );
            let op = taper_a * 3 - taper_b * 7;
            assert_eq!(op.mg(), mg_a * 3 - mg_b * 7);
            assert_eq!(op.eg(), eg_a * 3 - eg_b * 7);
        }
    }
}
