use gears::dyn_clone::DynClone;
use gears::games::Color;
use gears::general::board::Board;
use gears::general::common::StaticallyNamedEntity;
use gears::score::{PhaseType, PhasedScore, Score, ScoreT};
use std::fmt::Debug;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

pub mod rand_eval;

#[cfg(feature = "ataxx")]
pub mod ataxx;
#[cfg(feature = "chess")]
pub mod chess;
mod fairy;
#[cfg(feature = "mnk")]
pub mod mnk;
#[cfg(feature = "uttt")]
pub mod uttt;

pub trait Eval<B: Board>: Debug + Send + StaticallyNamedEntity + DynClone + 'static {
    /// Eval the given board at the given depth in a search. To just eval a single position,
    /// `ply` should be set to 0. Most eval functions completely ignore it.
    /// `engine` is used for asymmetric evals like king gambot.
    fn eval(&mut self, pos: &B, _ply: usize, _engine: B::Color) -> Score;

    fn eval_incremental(&mut self, _old_pos: &B, _mov: B::Move, new_pos: &B, ply: usize, engine: B::Color) -> Score {
        self.eval(new_pos, ply, engine)
    }

    /// How much larger do we expect variation in piece scores to be than variation in eval scores?
    /// This is used for coloring the eval score in the pretty 'eval' command, which removes each piece
    /// and prints the resulting eval delta. The value returned by this function doesn't have to be
    /// exact or calculated in any complex way, it just needs to be a rough ballpark estimate:
    /// For example, in chess, queen values are typically much larger than whole eval values,
    /// but in other games like ataxx or mnk, there isn't that much of a difference
    fn piece_scale(&self) -> ScoreT {
        2
    }
}

#[expect(type_alias_bounds)]
pub type SingleFeatureScore<S: ScoreType> = S::SingleFeatureScore;

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

    fn finalize<C: Color>(self, phase: PhaseType, max_phase: PhaseType, color: C, bonus: &[Self; 2])
    -> Self::Finalized;
}

impl ScoreType for PhasedScore {
    type Finalized = Score;
    type SingleFeatureScore = Self;

    fn finalize<C: Color>(mut self, phase: PhaseType, max_phase: PhaseType, color: C, bonus: &[Self; 2]) -> Score {
        if color.is_first() {
            self += bonus[0]
        } else {
            self -= bonus[1]
        }
        let score = self.taper(phase, max_phase);
        if color.is_first() { score } else { -score }
    }
}
