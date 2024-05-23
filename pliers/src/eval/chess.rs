use crate::eval::chess::PhaseType::*;
use crate::gd::{Feature, Float, PhaseMultiplier, Trace, Weight, Weights};
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

enum PhaseType {
    Mg,
    Eg,
}

pub fn chess_phase(pos: &Chessboard) -> Float {
    let phase: usize = UncoloredChessPiece::non_king_pieces()
        .map(|piece| pos.piece_bb(piece).num_set_bits() * CHESS_PHASE_VALUES[piece as usize])
        .sum();
    phase as Float / 24.0
}

fn to_feature_idx(piece: UncoloredChessPiece, color: Color, square: ChessSquare) -> usize {
    let sq_idx = match color {
        Color::White => square.flip().index(),
        Color::Black => square.index(),
    };
    NUM_SQUARES * piece as usize + sq_idx
}

fn psqt_features(pos: &Chessboard, trace: &mut Trace) {
    assert_eq!(trace.white.len(), trace.black.len());
    let base_idx = trace.black.len();
    trace
        .white
        .append(&mut vec![0; NUM_CHESS_PIECES * NUM_SQUARES]);
    trace
        .black
        .append(&mut vec![0; NUM_CHESS_PIECES * NUM_SQUARES]);
    trace.phase = chess_phase(pos);
    for color in Color::iter() {
        for piece in UncoloredChessPiece::pieces() {
            let mut bb = pos.colored_piece_bb(color, piece);
            while bb.has_set_bit() {
                let square = ChessSquare::new(bb.pop_lsb());
                let idx = base_idx + to_feature_idx(piece, color, square);
                trace.increment(idx, color);
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
    for piece in UncoloredChessPiece::pieces() {
        for phase in 0..2 {
            writeln!(f, "\t// {0} {1}", piece.name(), phase_names[phase])?;
            write!(f, "\t[")?;
            for square in 0..NUM_SQUARES {
                if square % 8 == 0 {
                    writeln!(f)?;
                    write!(f, "\t\t")?;
                }
                write!(
                    f,
                    "{:4}, ",
                    weights[64 * 2 * piece as usize + 2 * square + phase].rounded()
                )?;
            }
            writeln!(f)?;
            writeln!(f, "\t],")?;
        }
    }
    writeln!(f, "];")?;
    Ok(())
}
