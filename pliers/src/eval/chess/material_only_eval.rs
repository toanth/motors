//! A simple material-only eval that tunes piece weights.

use crate::eval::{Eval, WeightsInterpretation};
use crate::gd::{NonTaperedDatapoint, Weight, Weights};
use crate::load_data::NoFilter;
use crate::trace::{BasicTrace, SimpleTrace, TraceTrait};
use gears::games::Color;
use gears::games::chess::pieces::{ChessPieceType, NUM_CHESS_PIECES};
use gears::games::chess::{ChessColor, Chessboard};
use gears::general::bitboards::RawBitboard;
use gears::general::board::BitboardBoard;
use std::fmt::Formatter;

/// A simple material-only eval that tunes piece weights.
#[derive(Debug, Default)]
pub struct MaterialOnlyEval {}

impl WeightsInterpretation for MaterialOnlyEval {
    fn display(&self) -> fn(&mut Formatter, &Weights, &[Weight]) -> std::fmt::Result {
        |f: &mut Formatter<'_>, weights: &Weights, _old_weights: &[Weight]| {
            for piece in ChessPieceType::non_king_pieces() {
                writeln!(f, "{0}:\t{1}", piece.to_name(), weights[piece as usize])?;
            }
            Ok(())
        }
    }
}

impl Eval<Chessboard> for MaterialOnlyEval {
    fn num_weights() -> usize {
        NUM_CHESS_PIECES - 1
    }
    fn num_features() -> usize {
        Self::num_weights()
    }

    type D = NonTaperedDatapoint;
    type Filter = NoFilter;

    fn feature_trace(pos: &Chessboard) -> impl TraceTrait {
        let mut trace = SimpleTrace::for_features(Self::num_features());
        for color in ChessColor::iter() {
            for piece in ChessPieceType::non_king_pieces() {
                let num_pieces = pos.col_piece_bb(color, piece).num_ones() as isize;
                trace.increment_by(piece as usize, color, num_pieces);
            }
        }
        trace
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gd::Outcome;
    use gears::games::chess::pieces::ChessPieceType::Pawn;
    use gears::general::board::Board;

    #[test]
    pub fn startpos_test() {
        let board = Chessboard::default();
        let features = MaterialOnlyEval::feature_trace(&board);
        assert_eq!(features.as_features(0).len(), 0);
    }

    #[test]
    pub fn lucena_test() {
        let board = Chessboard::from_name("lucena").unwrap();
        let features = MaterialOnlyEval::extract_features(&board, Outcome::new(1.0), 1.0).features;
        assert_eq!(features.len(), 1);
        for (i, f) in features.iter().enumerate() {
            assert_eq!(i, f.idx());
            if i == Pawn as usize {
                assert_eq!(f.float(), 1.0);
            } else {
                assert_eq!(f.float(), 0.0);
            }
        }
    }
}
