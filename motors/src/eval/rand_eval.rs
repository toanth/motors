use rand::{Rng, thread_rng};

use gears::games::Board;
use gears::search::{MAX_NORMAL_SCORE, MIN_NORMAL_SCORE, Score};

use crate::eval::Eval;

#[derive(Debug, Default)]
pub struct RandEval {}

impl<B: Board> Eval<B> for RandEval {
    fn eval(&self, _: B) -> Score {
        Score(thread_rng().gen_range(MIN_NORMAL_SCORE.0..MAX_NORMAL_SCORE.0))
    }
}
