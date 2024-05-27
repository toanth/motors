use crate::eval::chess::{chess_phase, psqt_trace, write_psqts, NUM_PHASES};
use crate::eval::{changed_at_least, Eval, WeightFormatter};
use crate::gd::{
    Feature, Outcome, PhaseMultiplier, SimpleTrace, TaperedDatapoint, Weight, Weights,
};
use crate::load_data::NoFilter;
use gears::games::chess::pieces::{UncoloredChessPiece, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::chess::zobrist::NUM_PIECE_SQUARE_ENTRIES;
use gears::games::chess::Chessboard;
use gears::games::{Color, Coordinates};
use gears::general::bitboards::RawBitboard;
use std::fmt::Formatter;
use strum::IntoEnumIterator;

#[derive(Debug, Default)]
pub struct PistonEval {}

impl WeightFormatter for PistonEval {
    fn display_impl(&self) -> fn(&mut Formatter, &Weights, &[Weight]) -> std::fmt::Result {
        |f: &mut Formatter<'_>, weights: &Weights, old_weights: &[Weight]| {
            write_psqts(f, weights, &changed_at_least(5.0, weights, old_weights))
        }
    }
}

impl Eval<Chessboard> for PistonEval {
    const NUM_WEIGHTS: usize = Self::NUM_FEATURES * NUM_PHASES;

    const NUM_FEATURES: usize = NUM_PIECE_SQUARE_ENTRIES;

    type D = TaperedDatapoint;
    type Filter = NoFilter;

    fn feature_trace(pos: &Chessboard) -> SimpleTrace {
        psqt_trace(pos)
    }
}
