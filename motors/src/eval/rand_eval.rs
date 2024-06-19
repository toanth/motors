use rand::{thread_rng, Rng};

use crate::eval::Eval;
use gears::games::Board;
use gears::general::common::StaticallyNamedEntity;
use gears::score::{Score, MAX_NORMAL_SCORE, MIN_NORMAL_SCORE};

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

impl StaticallyNamedEntity for RandEval {
    fn static_short_name() -> &'static str
    where
        Self: Sized,
    {
        "random"
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        "Random eval".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "Returns random normal (i.e. not game over) scores. Can either be deterministic or truly random".to_string()
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
