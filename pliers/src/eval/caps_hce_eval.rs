use crate::eval::Eval;
use crate::gd::{Feature, Position, Weights};
use gears::games::chess::pieces::UncoloredChessPiece;
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::chess::zobrist::NUM_PIECE_SQUARE_ENTRIES;
use gears::games::chess::Chessboard;
use gears::games::{Color, Coordinates};
use gears::general::bitboards::RawBitboard;
use std::fmt::Formatter;
use strum::IntoEnumIterator;

fn to_feature_idx(piece: UncoloredChessPiece, color: Color, square: ChessSquare) -> usize {
    let sq_idx = match color {
        Color::White => square.flip().index(),
        Color::Black => square.index(),
    };
    NUM_SQUARES * piece as usize + sq_idx
}

pub fn psqt(pos: &Chessboard) -> Vec<Feature> {
    let mut res = vec![Feature(0); NUM_PIECE_SQUARE_ENTRIES];
    for color in Color::iter() {
        let increment = match color {
            Color::White => 1,
            Color::Black => -1,
        };
        for piece in UncoloredChessPiece::pieces() {
            let mut bb = pos.colored_piece_bb(color, piece);
            while bb.has_set_bit() {
                let square = ChessSquare::new(bb.pop_lsb());
                res[to_feature_idx(piece, color, square)].0 += increment;
            }
        }
    }
    res
}

#[derive(Debug, Default)]
pub struct CapsHceEval {}

impl Eval<Chessboard> for CapsHceEval {
    const NUM_FEATURES: usize = NUM_PIECE_SQUARE_ENTRIES;

    fn features(pos: &Chessboard) -> Position {
        psqt(pos)
    }

    fn format_impl(f: &mut Formatter<'_>, weights: &Weights) -> std::fmt::Result {
        let mut idx = 0;
        for piece in UncoloredChessPiece::iter() {
            writeln!(f, "\t// {piece} piece square table")?;
            write!(f, "\t[")?;
            for square in 0..NUM_SQUARES {
                if square % 8 == 0 {
                    writeln!(f)?;
                    write!(f, "\t\t")?;
                }
                write!(f, "{:3}, ", weights[idx].rounded())?;
                idx += 1;
            }
            writeln!(f)?;
            write!(f, "\t],")?;
        }
        Ok(())
    }
}
