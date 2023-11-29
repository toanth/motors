use rand::{thread_rng, Rng};

use crate::eval::Eval;
use crate::games::Board;
use crate::search::{Score, SCORE_LOST, SCORE_WON};

#[derive(Debug, Default)]
pub struct RandEval {}

impl<B: Board> Eval<B> for RandEval {
    fn eval(&self, _: B) -> Score {
        Score(thread_rng().gen_range(SCORE_LOST.0 + 1..SCORE_WON.0))
    }
}
