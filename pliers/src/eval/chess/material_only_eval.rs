use crate::eval::Eval;
use crate::gd::{Feature, FeatureT, Position, Weights};
use gears::games::chess::pieces::UncoloredChessPiece::King;
use gears::games::chess::pieces::{UncoloredChessPiece, NUM_CHESS_PIECES};
use gears::games::chess::Chessboard;
use gears::games::Color;
use gears::games::Color::White;
use gears::general::bitboards::RawBitboard;
use std::fmt::Formatter;
use strum::IntoEnumIterator;

#[derive(Debug, Default)]
pub struct MaterialOnlyEval {}

impl Eval<Chessboard> for MaterialOnlyEval {
    const NUM_FEATURES: usize = NUM_CHESS_PIECES - 1;

    fn features(pos: &Chessboard) -> Position {
        let mut res = vec![Feature::default(); Self::NUM_FEATURES];
        for color in Color::iter() {
            for piece in UncoloredChessPiece::non_king_pieces() {
                let num_pieces = pos.colored_piece_bb(color, piece).num_set_bits() as i8;
                if color == White {
                    res[piece as usize].0 += num_pieces as FeatureT;
                } else {
                    res[piece as usize].0 -= num_pieces as FeatureT;
                }
            }
        }
        res
    }

    fn format_impl(f: &mut Formatter<'_>, weights: &Weights) -> std::fmt::Result {
        for piece in UncoloredChessPiece::non_king_pieces() {
            writeln!(f, "{0}:\t{1}", piece.name(), weights[piece as usize])?
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gears::games::chess::pieces::UncoloredChessPiece::Pawn;
    use gears::games::Board;

    #[test]
    pub fn startpos_test() {
        let board = Chessboard::default();
        let features = MaterialOnlyEval::features(&board);
        assert_eq!(features.len(), 5);
        for f in features {
            assert_eq!(f.0, 0);
        }
    }

    pub fn lucena_test() {
        let board = Chessboard::from_name("lucena").unwrap();
        let features = MaterialOnlyEval::features(&board);
        assert_eq!(features.len(), 5);
        for (i, f) in features.iter().enumerate() {
            if i == Pawn as usize {
                assert_eq!(f.0, 1);
            } else {
                assert_eq!(f.0, 0);
            }
        }
    }
}
