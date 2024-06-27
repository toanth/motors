use dyn_clone::DynClone;
use std::fmt::Debug;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

use gears::games::Color::{Black, White};
use gears::games::{Board, Color};
use gears::general::common::StaticallyNamedEntity;
use gears::score::{PhaseType, PhasedScore, Score};

pub mod rand_eval;

#[cfg(feature = "chess")]
pub mod chess;
#[cfg(feature = "mnk")]
pub mod mnk;

pub trait Eval<B: Board>: Debug + Send + StaticallyNamedEntity + DynClone + 'static {
    fn eval(&mut self, pos: &B) -> Score;

    fn eval_incremental(&mut self, _old_pos: &B, _mov: B::Move, new_pos: &B, _ply: usize) -> Score {
        self.eval(new_pos)
    }
}

pub trait ScoreType:
    Debug
    + Default
    + Clone
    + Send
    + Eq
    + PartialEq
    + Add<Output = Self>
    + AddAssign
    + Sub<Output = Self>
    + SubAssign
    + Neg<Output = Self>
    + Mul<usize, Output = Self>
    + 'static
{
    type Finalized: Default;

    fn finalize(
        self,
        phase: PhaseType,
        max_phase: PhaseType,
        color: Color,
        tempo: Self::Finalized,
    ) -> Self::Finalized;
}

impl ScoreType for PhasedScore {
    type Finalized = Score;

    fn finalize(
        self,
        phase: PhaseType,
        max_phase: PhaseType,
        color: Color,
        tempo: Self::Finalized,
    ) -> Score {
        let score = self.taper(phase, max_phase);
        tempo
            + match color {
                White => score,
                Black => -score,
            }
    }
}
