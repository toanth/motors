use crate::eval::{Eval, WeightsInterpretation};
use crate::gd::{NonTaperedDatapoint, Outcome, SimpleTrace, TraceTrait, Weight, Weights};
use crate::load_data::NoFilter;
use gears::games::chess::pieces::{UncoloredChessPiece, NUM_CHESS_PIECES};
use gears::games::chess::Chessboard;
use gears::games::Color;
use gears::general::bitboards::RawBitboard;
use std::fmt::Formatter;
use strum::IntoEnumIterator;

#[derive(Debug, Default)]
pub struct MaterialOnlyEval {}

impl WeightsInterpretation for MaterialOnlyEval {
    fn display_impl(&self) -> fn(&mut Formatter, &Weights, &[Weight]) -> std::fmt::Result {
        |f: &mut Formatter<'_>, weights: &Weights, _old_weights: &[Weight]| {
            for piece in UncoloredChessPiece::non_king_pieces() {
                writeln!(f, "{0}:\t{1}", piece.name(), weights[piece as usize])?
            }
            Ok(())
        }
    }
}

impl Eval<Chessboard> for MaterialOnlyEval {
    const NUM_WEIGHTS: usize = NUM_CHESS_PIECES - 1;
    const NUM_FEATURES: usize = Self::NUM_WEIGHTS;

    type D = NonTaperedDatapoint;
    type Filter = NoFilter;

    fn feature_trace(pos: &Chessboard) -> impl TraceTrait {
        let mut trace = SimpleTrace::for_features(Self::NUM_FEATURES);
        for color in Color::iter() {
            for piece in UncoloredChessPiece::non_king_pieces() {
                let num_pieces = pos.colored_piece_bb(color, piece).num_set_bits() as isize;
                trace.increment_by(piece as usize, color, num_pieces);
            }
        }
        trace
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gd::TraceTrait;
    use gears::games::chess::pieces::UncoloredChessPiece::Pawn;
    use gears::games::Board;

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
