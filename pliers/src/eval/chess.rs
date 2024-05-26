use crate::eval::chess::PhaseType::*;
use crate::gd::{Feature, Float, PhaseMultiplier, SimpleTrace, Weight, Weights};
use crate::load_data::{Filter, ParseResult};
use gears::games::chess::pieces::{UncoloredChessPiece, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::chess::zobrist::NUM_PIECE_SQUARE_ENTRIES;
use gears::games::chess::Chessboard;
use gears::games::Color;
use gears::games::Color::White;
use gears::general::bitboards::RawBitboard;
use std::f32::consts::E;
use std::fmt::{Display, Formatter};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

pub mod caps_hce_eval;
pub mod material_only_eval;
pub mod piston_eval;

pub struct SkipChecks {}

impl Filter<Chessboard> for SkipChecks {
    fn filter(pos: ParseResult<Chessboard>) -> Option<ParseResult<Chessboard>> {
        if pos.pos.is_in_check() {
            None
        } else {
            Some(pos)
        }
    }
}

// TODO: Qsearch filter

const NUM_PHASES: usize = 2;
const CHESS_PHASE_VALUES: [usize; NUM_CHESS_PIECES] = [0, 1, 1, 2, 4, 0];

const NUM_PSQT_FEATURES: usize = NUM_CHESS_PIECES * NUM_SQUARES;

#[derive(Debug, Copy, Clone, EnumIter)]
enum PhaseType {
    Mg,
    Eg,
}

impl Display for PhaseType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Mg => write!(f, "mg"),
            Eg => write!(f, "eg"),
        }
    }
}

pub fn chess_phase(pos: &Chessboard) -> Float {
    let phase: usize = UncoloredChessPiece::non_king_pieces()
        .map(|piece| pos.piece_bb(piece).num_set_bits() * CHESS_PHASE_VALUES[piece as usize])
        .sum();
    phase as Float / 24.0
}

fn to_feature_idx(piece: UncoloredChessPiece, color: Color, square: ChessSquare) -> usize {
    NUM_SQUARES * piece as usize + square.flip_if(color == White).index()
}

fn psqt_trace(pos: &Chessboard) -> SimpleTrace {
    let mut trace = SimpleTrace::for_features(NUM_PSQT_FEATURES);
    trace.phase = chess_phase(pos);
    for color in Color::iter() {
        for piece in UncoloredChessPiece::pieces() {
            let mut bb = pos.colored_piece_bb(color, piece);
            while bb.has_set_bit() {
                let square = ChessSquare::new(bb.pop_lsb());
                let idx = to_feature_idx(piece, color, square);
                trace.increment(idx, color);
            }
        }
    }
    trace
}

/// Apply a simple blur on the PSQTs to reduce noise.
#[rustfmt::skip]
const BLOOM: [[Float; 3]; 3] = [
    [0.05, 0.1,  0.05],
    [0.1,  0.4,  0.1],
    [0.05, 0.1,  0.05]
];

fn get(
    weights: &[Weight],
    piece_idx: usize,
    square: usize,
    rank_delta: isize,
    file_delta: isize,
    phase: PhaseType,
) -> Float {
    let rank = (square as isize / 8 + rank_delta).clamp(0, 7);
    let file = (square as isize % 8 + file_delta).clamp(0, 7);
    let square = rank * 8 + file;
    let bloom_multiplier = BLOOM[(rank_delta + 1) as usize][(file_delta + 1) as usize];
    weights[64 * 2 * piece_idx + 2 * square as usize + phase as usize].0 * bloom_multiplier
}

fn write_phased_psqt(
    f: &mut Formatter<'_>,
    weights: &[Weight],
    piece_idx: usize,
    piece_name: &str,
) -> std::fmt::Result {
    const TAB: &str = "    "; // Use 4 spaces for a tab.
    for phase in PhaseType::iter() {
        writeln!(f, "{TAB}// {piece_name} {phase}")?;
        write!(f, "{TAB}[")?;
        for square in 0..NUM_SQUARES {
            if square % 8 == 0 {
                writeln!(f)?;
                write!(f, "{TAB}{TAB}")?;
            }
            let mut val = 0.0;
            for rank_delta in -1..=1 {
                for file_delta in -1..1 {
                    val += get(weights, piece_idx, square, rank_delta, file_delta, phase);
                }
            }

            write!(f, "{:4}, ", val.round())?;
        }
        writeln!(f)?;
        writeln!(f, "{TAB}],")?;
    }
    Ok(())
}

fn write_psqts(f: &mut Formatter<'_>, weights: &[Weight]) -> std::fmt::Result {
    writeln!(
        f,
        "const PSQTS: [[i32; NUM_SQUARES]; NUM_CHESS_PIECES * 2] = ["
    )?;
    for piece in UncoloredChessPiece::pieces() {
        write_phased_psqt(f, weights, piece as usize, piece.name())?;
    }
    writeln!(f, "];")?;
    Ok(())
}
