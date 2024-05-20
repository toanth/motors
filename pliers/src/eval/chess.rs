use crate::eval::chess::Phase::{Eg, Mg};
use crate::gd::{Feature, Float, Weight, Weights};
use gears::games::chess::pieces::{UncoloredChessPiece, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::chess::zobrist::NUM_PIECE_SQUARE_ENTRIES;
use gears::games::chess::Chessboard;
use gears::games::Color;
use gears::general::bitboards::RawBitboard;
use std::f32::consts::E;
use std::fmt::Formatter;
use strum::IntoEnumIterator;

pub mod caps_hce_eval;
pub mod material_only_eval;
pub mod piston_eval;

const NUM_PHASES: usize = 2;
const CHESS_PHASE_VALUES: [usize; NUM_CHESS_PIECES] = [0, 1, 1, 2, 4, 0];

enum Phase {
    Mg,
    Eg,
}

pub fn chess_phase(pos: &Chessboard) -> Float {
    let phase: usize = UncoloredChessPiece::non_king_pieces()
        .map(|piece| pos.piece_bb(piece).num_set_bits() * CHESS_PHASE_VALUES[piece as usize])
        .sum();
    phase as Float / 24.0
}

fn to_feature_idx(
    piece: UncoloredChessPiece,
    color: Color,
    square: ChessSquare,
    phase: Phase,
) -> usize {
    let sq_idx = match color {
        Color::White => square.flip().index(),
        Color::Black => square.index(),
    };
    NUM_SQUARES * (phase as usize + 2 * piece as usize) + sq_idx
}

fn psqt_features(pos: &Chessboard, features: &mut [Feature]) {
    let phase_scale = chess_phase(pos);
    for color in Color::iter() {
        let increment = match color {
            Color::White => 1.0,
            Color::Black => -1.0,
        };
        for piece in UncoloredChessPiece::pieces() {
            let mut bb = pos.colored_piece_bb(color, piece);
            while bb.has_set_bit() {
                let square = ChessSquare::new(bb.pop_lsb());
                let idx = to_feature_idx(piece, color, square, Mg);
                features[idx].0 += increment * phase_scale;
                let idx = to_feature_idx(piece, color, square, Eg);
                features[idx].0 += increment * (1.0 - phase_scale);
            }
        }
    }
}

fn write_psqt(f: &mut Formatter<'_>, weights: &[Weight]) -> std::fmt::Result {
    writeln!(
        f,
        "const PSQTS: [[i32; NUM_SQUARES]; NUM_CHESS_PIECES * 2] = ["
    )?;
    let phase_names = ["mg", "eg"];
    let mut idx = 0;
    for piece in UncoloredChessPiece::pieces() {
        for phase in 0..2 {
            writeln!(
                f,
                "\t// {0} {1} piece square table",
                piece.name(),
                phase_names[phase]
            )?;
            write!(f, "\t[")?;
            for square in 0..NUM_SQUARES {
                if square % 8 == 0 {
                    writeln!(f)?;
                    write!(f, "\t\t")?;
                }
                write!(f, "{:4}, ", weights[idx].rounded())?;
                idx += 1;
            }
            writeln!(f)?;
            writeln!(f, "\t],")?;
        }
    }
    writeln!(f, "];")?;
    Ok(())
}
