use std::fmt::Debug;

use gears::games::Board;
use gears::general::common::StaticallyNamedEntity;
use gears::score::Score;

pub mod rand_eval;

#[cfg(feature = "chess")]
pub mod chess;
#[cfg(feature = "mnk")]
pub mod mnk;

pub trait Eval<B: Board>: Debug + Default + Send + StaticallyNamedEntity + 'static {
    fn eval(&self, pos: B) -> Score;
}
