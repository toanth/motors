use crate::eval::chess::{psqt_features, write_psqt, NUM_PHASES};
use crate::eval::Eval;
use crate::gd::{Feature, Position, Weights};
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

impl Eval<Chessboard> for CapsHceEval {
    const NUM_FEATURES: usize = NUM_PIECE_SQUARE_ENTRIES * NUM_PHASES; // TODO: Add more features

    fn features(pos: &Chessboard) -> Position {
        let mut res = vec![Feature::default(); Self::NUM_FEATURES];
        psqt_features(pos, &mut res);
        res
    }

    fn format_impl(f: &mut Formatter<'_>, weights: &Weights) -> std::fmt::Result {
        write_psqt(f, weights)
    }
}
