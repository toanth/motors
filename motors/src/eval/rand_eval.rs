use rand::rngs::StdRng;
use rand::{thread_rng, Rng, SeedableRng};

use gears::games::Board;
use gears::search::{Score, MAX_NORMAL_SCORE, MIN_NORMAL_SCORE};

use crate::eval::Eval;

#[derive(Debug)]
pub struct RandEval {
    deterministic: bool,
}

impl Default for RandEval {
    fn default() -> Self {
        Self {
            deterministic: true,
        }
    }
}

impl<B: Board> Eval<B> for RandEval {
    fn eval(&self, pos: B) -> Score {
        if self.deterministic {
            Score(
                // deterministic and faster than seeding a rng while still being good enough
                (pos.zobrist_hash().0 % (MAX_NORMAL_SCORE.0 - MIN_NORMAL_SCORE.0) as u64) as i32
                    + MIN_NORMAL_SCORE.0,
            )
            // too slow (there's probably a way to do this faster while using the rng crate, but the above is good enough)
            // StdRng::seed_from_u64(pos.zobrist_hash().0)
            //     .gen_range(MIN_NORMAL_SCORE.0..=MAX_NORMAL_SCORE.0),
        } else {
            Score(thread_rng().gen_range(MIN_NORMAL_SCORE.0..=MAX_NORMAL_SCORE.0))
        }
    }
}
