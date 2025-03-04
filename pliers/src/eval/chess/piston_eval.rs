//!  `PiSTOn` is a Piece Square Table Only evaluation function.
//! There is a weight for each of the 6 * 2 * 64 piece, phase, square combinations.

use crate::eval::chess::{psqt_trace, write_psqts, NUM_PHASES};
use crate::eval::{changed_at_least, Eval, WeightsInterpretation};
use crate::gd::{TaperedDatapoint, Weight, Weights};
use crate::load_data::NoFilter;
use crate::trace::TraceTrait;
use gears::games::chess::zobrist::NUM_PIECE_SQUARE_ENTRIES;
use gears::games::chess::Chessboard;
use std::fmt::Formatter;

///  `PiSTOn` is a Piece Square Table Only evaluation function.
/// There is a weight for each of the 6 * 2 * 64 piece, phase, square combinations.
#[derive(Debug, Default)]
pub struct PistonEval {}

impl WeightsInterpretation for PistonEval {
    fn display(&self) -> fn(&mut Formatter, &Weights, &[Weight]) -> std::fmt::Result {
        |f: &mut Formatter<'_>, weights: &Weights, old_weights: &[Weight]| {
            write_psqts(f, weights, &changed_at_least(-1.0, weights, old_weights))
        }
    }
}

impl Eval<Chessboard> for PistonEval {
    fn num_features() -> usize {
        NUM_PIECE_SQUARE_ENTRIES
    }
    fn num_weights() -> usize {
        Self::num_features() * NUM_PHASES
    }

    type D = TaperedDatapoint;
    type Filter = NoFilter;

    fn feature_trace(pos: &Chessboard) -> impl TraceTrait {
        psqt_trace(pos)
    }
}
