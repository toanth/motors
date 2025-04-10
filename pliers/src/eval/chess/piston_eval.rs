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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load_data::{FenReader, Perspective};
    use gears::games::Coordinates;
    use gears::games::chess::squares::{ChessSquare, ChessboardSize};
    use gears::general::board::{Board, BoardHelpers};

    #[test]
    fn philidor_test() {
        let board = Chessboard::from_name("philidor").unwrap();
        let fen = board.as_fen();
        let string = format!("{fen} [1.0]\n{fen} [1/2-1/2]");
        let dataset = FenReader::<Chessboard, PistonEval>::load_from_str(&string, Perspective::White).unwrap();
        let dp = dataset.as_batch().datapoint_iter().next().unwrap();
        let entries = dp.entries;
        assert_eq!(entries.len(), 5 * 2);
        let pawn_mg = entries[0];
        let pawn_sq = ChessSquare::from_chars('d', '5').unwrap();
        assert_eq!(pawn_mg.idx, pawn_sq.flip_up_down(ChessboardSize::default()).bb_idx() * 2);
        let pawn_eg = entries[1];
        assert_eq!(pawn_mg.idx + 1, pawn_eg.idx);
        let w1 = pawn_mg.weight;
        let w2 = pawn_eg.weight;
        assert_eq!(w1 + w2, 1.0, "{w1} {w2}");
    }
}
