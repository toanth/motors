use crate::eval::chess::{psqt_features, write_psqt, NUM_PHASES};
use crate::eval::{Eval, FormatWeights, WeightFormatter};
use crate::gd::{
    Datapoint, Feature, Float, Outcome, PhaseMultiplier, TaperedDatapoint, Trace, Weights,
};
use gears::games::chess::pieces::{UncoloredChessPiece, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::chess::zobrist::NUM_PIECE_SQUARE_ENTRIES;
use gears::games::chess::Chessboard;
use gears::games::{Color, Coordinates};
use gears::general::bitboards::RawBitboard;
use std::fmt::Formatter;
use strum::IntoEnumIterator;

#[derive(Debug, Default)]
pub struct CapsHceEval {}

impl WeightFormatter for CapsHceEval {
    fn format_impl(&self) -> (fn(&mut Formatter, &Weights) -> std::fmt::Result) {
        // TODO: More features
        |f: &mut Formatter<'_>, weights: &Weights| write_psqt(f, weights)
    }
}

impl<'a> Eval<Chessboard> for CapsHceEval {
    const NUM_FEATURES: usize = NUM_PIECE_SQUARE_ENTRIES * NUM_PHASES; // TODO: Add more features

    type D = TaperedDatapoint;

    fn feature_trace(pos: &Chessboard) -> Trace {
        let mut trace = Trace::default();
        psqt_features(pos, &mut trace);
        // TODO: Add more features
        trace
    }
}
