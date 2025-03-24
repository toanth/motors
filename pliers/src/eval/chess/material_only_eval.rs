//! A simple material-only eval that tunes piece weights.

use crate::eval::{Eval, WeightsInterpretation};
use crate::gd::{Weight, Weights};
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
    fn num_features() -> usize {
        NUM_CHESS_PIECES - 1
    }

    type Filter = NoFilter;

    fn feature_trace(pos: &Chessboard) -> impl TraceTrait {
        let mut trace = SimpleTrace::for_features(Self::num_features(), 1.0);
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
    use crate::gd::{Dataset, Outcome};
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
        let mut dataset = Dataset::new(2);
        MaterialOnlyEval::extract_features(&board, Outcome::new(1.0), &mut dataset);
        let dp = dataset.as_batch().datapoint_iter().next().unwrap();
        let mut features = dp.features();
        assert_eq!(features.clone().count(), 2);
        assert_eq!(features.next().unwrap().weight, 1.0);
        assert_eq!(features.next().unwrap().weight, 0.0);
    }
}
