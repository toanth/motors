use dyn_clone::DynClone;
use gears::games::Color;
use gears::general::board::Board;
use std::fmt::Debug;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

use gears::general::common::StaticallyNamedEntity;
use gears::score::{PhaseType, PhasedScore, Score};

pub mod rand_eval;

#[cfg(feature = "ataxx")]
pub mod ataxx;
#[cfg(feature = "chess")]
pub mod chess;
#[cfg(feature = "mnk")]
pub mod mnk;
#[cfg(feature = "uttt")]
pub mod uttt;

pub trait Eval<B: Board>: Debug + Send + StaticallyNamedEntity + DynClone + 'static {
    fn eval(&mut self, pos: &B) -> Score;

    fn eval_incremental(&mut self, _old_pos: &B, _mov: B::Move, new_pos: &B, _ply: usize) -> Score {
        self.eval(new_pos)
    }
}

/// There is only one implementation of this trait in this crate: [`PhasedScore`].
///
/// It should be easy to implement this for other scores, but the reason it's a trait is that in the [`pliers`] crate,
/// there is a trace that also implements this trait so that it can be used for tuning without needing to duplicate the
/// eval function.
pub trait ScoreType:
    Debug
    + Default
    + Clone
    + Send
    + Eq
    + PartialEq
    + Add<Output = Self>
    + AddAssign
    + Add<Self::SingleFeatureScore, Output = Self>
    + AddAssign<Self::SingleFeatureScore>
    + Sub<Output = Self>
    + SubAssign
    + Sub<Self::SingleFeatureScore, Output = Self>
    + SubAssign<Self::SingleFeatureScore>
    + Neg<Output = Self>
    + Mul<usize, Output = Self>
    + From<Self::SingleFeatureScore>
    + 'static
{
    type Finalized: Default;
    type SingleFeatureScore: Default + Mul<usize, Output = Self::SingleFeatureScore>;

    fn finalize<C: Color>(
        self,
        phase: PhaseType,
        max_phase: PhaseType,
        color: C,
        tempo: Self::Finalized,
    ) -> Self::Finalized;
}

impl ScoreType for PhasedScore {
    type Finalized = Score;
    type SingleFeatureScore = Self;

    fn finalize<C: Color>(
        self,
        phase: PhaseType,
        max_phase: PhaseType,
        color: C,
        tempo: Self::Finalized,
    ) -> Score {
        let score = self.taper(phase, max_phase);
        tempo + if color.is_first() { score } else { -score }
    }
}
