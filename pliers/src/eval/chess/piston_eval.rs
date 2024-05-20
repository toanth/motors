use crate::eval::chess::{chess_phase, psqt_features, write_psqt, NUM_PHASES};
use crate::eval::Eval;
use crate::gd::{Feature, Float, Position, Weights};
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

impl Eval<Chessboard> for PistonEval {
    const NUM_FEATURES: usize = NUM_PIECE_SQUARE_ENTRIES * NUM_PHASES;

    fn features(pos: &Chessboard) -> Position {
        let mut res = vec![Feature::default(); Self::NUM_FEATURES];
        psqt_features(pos, &mut res);
        res
    }

    fn format_impl(f: &mut Formatter<'_>, weights: &Weights) -> std::fmt::Result {
        write_psqt(f, weights)
    }
}
