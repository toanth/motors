use crate::eval::chess::{psqt_trace, write_psqts, NUM_PHASES};
use crate::eval::{changed_at_least, Eval, WeightsInterpretation};
use crate::gd::{SimpleTrace, TaperedDatapoint, TraceTrait, Weight, Weights};
use crate::load_data::NoFilter;
use gears::games::chess::zobrist::NUM_PIECE_SQUARE_ENTRIES;
use gears::games::chess::Chessboard;
use std::fmt::Formatter;

#[derive(Debug, Default)]
pub struct PistonEval {}

impl WeightsInterpretation for PistonEval {
    fn display_impl(&self) -> fn(&mut Formatter, &Weights, &[Weight]) -> std::fmt::Result {
        |f: &mut Formatter<'_>, weights: &Weights, old_weights: &[Weight]| {
            write_psqts(f, weights, &changed_at_least(-1.0, weights, old_weights))
        }
    }
}

impl Eval<Chessboard> for PistonEval {
    const NUM_WEIGHTS: usize = Self::NUM_FEATURES * NUM_PHASES;

    const NUM_FEATURES: usize = NUM_PIECE_SQUARE_ENTRIES;

    type D = TaperedDatapoint;
    type Filter = NoFilter;

    fn feature_trace(pos: &Chessboard) -> impl TraceTrait {
        psqt_trace(pos)
    }
}
