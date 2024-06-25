use dyn_clone::DynClone;
use std::fmt::Debug;

use gears::games::Board;
use gears::general::common::StaticallyNamedEntity;
use gears::score::Score;

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
