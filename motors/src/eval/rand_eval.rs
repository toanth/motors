use gears::general::board::Board;
use gears::rand::{Rng, rng};
use std::fmt::Display;

use crate::eval::Eval;
use gears::general::common::StaticallyNamedEntity;
use gears::score::{MAX_NORMAL_SCORE, MIN_NORMAL_SCORE, Score, ScoreT};

#[derive(Debug, Clone)]
pub struct RandEval {
    deterministic: bool,
}

impl Default for RandEval {
    fn default() -> Self {
        Self { deterministic: true }
    }
}

impl StaticallyNamedEntity for RandEval {
    fn static_short_name() -> impl Display
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
    fn eval(&mut self, pos: &B, _ply: usize, _engine: B::Color) -> Score {
        if self.deterministic {
            // deterministic and faster than seeding a rng while still being good enough
            let random = (pos.hash_pos().0 % (MAX_NORMAL_SCORE.0 as i64 - MIN_NORMAL_SCORE.0 as i64 + 1) as u64) as i64;
            Score((random + MIN_NORMAL_SCORE.0 as i64) as ScoreT)
            // too slow (there's probably a way to do this faster while using the rng crate, but the above is good enough)
            // StdRng::seed_from_u64(pos.zobrist_hash().0)
            //     .random_range(MIN_NORMAL_SCORE.0..=MAX_NORMAL_SCORE.0),
        } else {
            Score(rng().random_range(MIN_NORMAL_SCORE.0..=MAX_NORMAL_SCORE.0))
        }
    }
}
