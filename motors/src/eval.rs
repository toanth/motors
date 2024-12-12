use dyn_clone::DynClone;
use gears::games::Color;
use gears::general::board::Board;
use std::fmt::Debug;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

use gears::general::common::StaticallyNamedEntity;
use gears::score::{PhaseType, PhasedScore, Score, ScoreT};

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
    /// Eval the given board at the given depth in a search. To just eval a single position,
    /// `eval_simple` can be used, which sets `ply` and `data` to default values. Most eval functions completely ignore them.
    /// `ply` is used for keeping track of incremental updates, and `data` can be used to save computed results that will also
    /// help with generating and playing moves.
    fn eval(&mut self, pos: &B, _ply: usize, _data: &mut B::ExternalData) -> Score;

    /// Eval a single position. Calls [`eval`] with default values for `_ply` and `_data`.
    fn eval_simple(&mut self, pos: &B) -> Score {
        self.eval(pos, 0, &mut B::ExternalData::default())
    }

    /// Eval this position while potentially reusing information computed for the old position.
    /// The default implementation ignores `_old_pos` and simply forwards to [`eval`].
    fn eval_incremental(
        &mut self,
        _old_pos: &B,
        _mov: B::Move,
        new_pos: &B,
        ply: usize,
        data: &mut B::ExternalData,
    ) -> Score {
        self.eval(new_pos, ply, data)
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
