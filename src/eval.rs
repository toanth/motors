use std::fmt::Debug;

use crate::games::Board;
use crate::search::Score;

pub mod rand_eval;

pub mod chess;
pub mod mnk;

pub trait Eval<B: Board>: Debug + Default + Send + 'static {
    fn eval(&self, pos: B) -> Score;
}
