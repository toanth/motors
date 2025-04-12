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
use crate::general::common::Res;
use crate::search::NodeType;
use crate::search::NodeType::{Exact, FailHigh, FailLow};
use anyhow::anyhow;
use derive_more::{Add, AddAssign, Neg, Sub, SubAssign};
use num::ToPrimitive;
use std::fmt::{Display, Formatter};
use std::ops::{Add, Div, DivAssign, Mul, MulAssign, Sub};

/// Valid scores fit into 16 bits, but it's possible to temporarily overflow that range with some operations,
/// e.g. when computing `score - previous_score`. So in order to avoid bugs related to that, simply use 32 bits.
pub type ScoreT = i32;

/// In some places, it's important to save space by using only the necessary 16 bits for a score.
pub type CompactScoreT = i16;

#[derive(Default, Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Hash, Add, Sub, Neg, AddAssign, SubAssign)]
#[must_use]
pub struct Score(pub ScoreT);

impl Display for Score {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(moves_until_over) = self.moves_until_game_won() {
            write!(f, "mate {moves_until_over}")
        } else {
            write!(f, "cp {0}", self.0) // TODO: WDL normalization
        }
    }
}

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

impl TryFrom<isize> for Score {
    type Error = anyhow::Error;

    fn try_from(value: isize) -> Res<Self> {
        let score = ScoreT::try_from(value)?;
        Score(score).verify_valid().ok_or_else(|| anyhow!("{score} is outside of the valid values for a Score"))
    }
}

impl Score {
    pub fn from_compact(compact: CompactScoreT) -> Self {
        Self(compact as ScoreT)
    }
    pub fn is_game_won_score(self) -> bool {
        self >= MIN_SCORE_WON
    }
    pub fn is_game_lost_score(self) -> bool {
        self <= MAX_SCORE_LOST
    }
    pub fn is_won_or_lost(self) -> bool {
        self.is_game_won_score() || self.is_game_lost_score()
    }
    // a draw implies score == 0, but score == 0 does not imply a draw
    pub fn is_won_lost_or_draw_score(self) -> bool {
        self.is_won_or_lost() || self.0 == 0
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
        self.plies_until_game_won().map(|n| (n as f32 / 2f32).ceil() as isize)
    }

    pub fn plies_until_game_over(self) -> Option<isize> {
        self.plies_until_game_won().map(isize::abs)
    }

    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }

    pub fn verify_valid(self) -> Option<Self> {
        if (self <= SCORE_WON && self >= SCORE_LOST) || self == SCORE_TIME_UP || self == NO_SCORE_YET {
            Some(self)
        } else {
            None
        }
    }

    pub fn is_valid(self) -> bool {
        self.verify_valid().is_some()
    }

    pub fn flip_if(self, flip: bool) -> Self {
        if flip { -self } else { self }
    }

    pub fn node_type(self, alpha: Score, beta: Score) -> NodeType {
        if self <= alpha {
            FailLow
        } else if self >= beta {
            FailHigh
        } else {
            Exact
        }
    }

    pub fn compact(self) -> CompactScoreT {
        self.0 as CompactScoreT
    }
}

/// `SCORE_WON` and `SCORE_LOST` need to fit into 16 bits for the tapered score to work,
/// and the open interval `(alpha, beta)` has to be able to contain them.
pub const MIN_ALPHA: Score = Score(-31_001);
pub const MAX_BETA: Score = Score(31_001);
pub const SCORE_LOST: Score = Score(-31_000);
pub const SCORE_WON: Score = Score(31_000);
pub const SCORE_TIME_UP: Score = Score(SCORE_LOST.0 - 1000);
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
            ((self.mg().0 as PhaseType * phase + self.eg().0 as PhaseType * (max_phase - phase)) / max_phase) as ScoreT,
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
        if cfg!(debug_assertions) {
            let mg = self.mg();
            let eg = self.eg();
            let mg_res = (mg.0 as isize * rhs.to_isize().unwrap()).try_into().unwrap();
            debug_assert!(is_valid_score(mg_res));
            let eg_res = (eg.0 as isize * rhs.to_isize().unwrap()).try_into().unwrap();
            debug_assert!(is_valid_score(eg_res));
        }
        self.0 *= rhs as ScoreT;
    }
}

impl Div<usize> for PhasedScore {
    type Output = Self;

    fn div(mut self, rhs: usize) -> Self::Output {
        self /= rhs;
        self
    }
}

impl DivAssign<usize> for PhasedScore {
    fn div_assign(&mut self, rhs: usize) {
        *self = Self::new(
            (self.mg().0 as CompactScoreT) / rhs as CompactScoreT,
            (self.eg().0 as CompactScoreT) / rhs as CompactScoreT,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use rand::prelude::SliceRandom;
    use rand::rng;

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
        v.shuffle(&mut rng());
        for ((mg_a, eg_a), (mg_b, eg_b)) in v.iter().copied().tuple_windows() {
            let taper_a = p(mg_a, eg_a);
            let taper_b = p(mg_b, eg_b);
            let mg_a = Score(ScoreT::from(mg_a));
            let mg_b = Score(ScoreT::from(mg_b));
            let eg_a = Score(ScoreT::from(eg_a));
            let eg_b = Score(ScoreT::from(eg_b));
            for op in 0..4 {
                let c = taper_b.mg().0.clamp(0, 90);
                let mut d = taper_b.eg().0 as usize;
                if d == 0 {
                    d = 1;
                }
                let func = |a: PhasedScore, b: PhasedScore| match op {
                    0 => a + b,
                    1 => a - b,
                    2 => a * c as usize,
                    _ => a / d,
                };
                let f2 = |a: Score, b: Score| match op {
                    0 => a + b,
                    1 => a - b,
                    2 => a * c,
                    _ => a / d as ScoreT,
                };
                let res = func(taper_a, taper_b);
                assert_eq!(res.mg(), f2(mg_a, mg_b), "{0} {mg_a} {mg_b} -- {1}, op `{2}`", res.mg(), res.0, op);
                assert_eq!(res.eg(), f2(eg_a, eg_b), "{0} {eg_a} {eg_b} -- {1}", res.eg(), res.0);
            }
            let op = taper_a * 3 / 2 - taper_b * 7;
            assert_eq!(op.mg(), mg_a * 3 / 2 - mg_b * 7);
            assert_eq!(op.eg(), eg_a * 3 / 2 - eg_b * 7);
        }
    }
}
