//!  `PiSTOn` is a Piece Square Table Only evaluation function.
//! There is a weight for each of the 6 * 2 * 64 piece, phase, square combinations.

use crate::eval::chess::{psqt_trace, write_psqts};
use crate::eval::{Eval, WeightsInterpretation, changed_at_least};
use crate::gd::{Weight, Weights};
use crate::load_data::NoFilter;
use crate::trace::TraceTrait;
use gears::games::chess::Chessboard;
use gears::games::chess::zobrist::NUM_PIECE_SQUARE_ENTRIES;
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

    type Filter = NoFilter;

    fn feature_trace(pos: &Chessboard) -> impl TraceTrait {
        psqt_trace(pos)
    }
}
