//! Contains chess evaluation functions, and some shared code that is generally useful for them,
//! such as the [`SkipChecks`] [`Filter`].
use crate::eval::write_phased_with_width;
use crate::gd::{Float, Weight};
use crate::load_data::{Filter, ParseResult};
use crate::trace::{BasicTrace, SimpleTrace, TraceNFeatures};
use gears::games::chess::pieces::{ChessPieceType, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::chess::ChessColor::White;
use gears::games::chess::{ChessColor, Chessboard};
use gears::games::Color;
use gears::general::bitboards::{Bitboard, RawBitboard};
use gears::general::board::BitboardBoard;
use motors::eval::chess::CHESS_PHASE_VALUES;
use std::fmt::Formatter;

pub mod lite;
pub mod material_only_eval;
pub mod piston_eval;

/// Remove positions where the side to move is in check.
pub struct SkipChecks {}

impl Filter<Chessboard> for SkipChecks {
    #[expect(refining_impl_trait)]
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

const NUM_PSQT_FEATURES: usize = NUM_CHESS_PIECES * NUM_SQUARES;

/// Computes the game phase based on the number of pieces on the board.
///
/// The start position has a phase value of 24, and a position without any non-pawn, non-king pieces
/// has a phase value of zero.
pub fn chess_phase(pos: &Chessboard) -> Float {
    let phase: usize = ChessPieceType::non_king_pieces()
        .map(|piece| pos.piece_bb(piece).num_ones() * CHESS_PHASE_VALUES[piece as usize])
        .sum();
    phase as Float / 24.0
}

fn to_feature_idx(piece: ChessPieceType, color: ChessColor, square: ChessSquare) -> usize {
    NUM_SQUARES * piece as usize + square.flip_if(color == White).bb_idx()
}

fn psqt_trace(pos: &Chessboard) -> TraceNFeatures<NUM_PSQT_FEATURES> {
    let mut trace = SimpleTrace::for_features(NUM_PSQT_FEATURES);
    trace.phase = chess_phase(pos);
    for color in ChessColor::iter() {
        for piece in ChessPieceType::pieces() {
            let bb = pos.colored_piece_bb(color, piece);
            for square in bb.ones() {
                let idx = to_feature_idx(piece, color, square);
                trace.increment(idx, color);
            }
        }
    }
    TraceNFeatures(trace)
}

fn write_phased_psqt(
    f: &mut Formatter<'_>,
    weights: &[Weight],
    special: &[bool],
    piece: Option<ChessPieceType>,
    offset: usize,
) -> std::fmt::Result {
    const TAB: &str = "    "; // Use 4 spaces for a tab.
    if let Some(piece) = piece {
        writeln!(f, "{TAB}// {}", piece.name())?;
        write!(f, "{TAB}[")?;
    } else {
        write!(f, "[")?;
    }
    for square in 0..NUM_SQUARES {
        if square % 8 == 0 {
            writeln!(f)?;
            write!(f, "{TAB}{TAB}")?;
        }
        let idx = offset + square;

        write_phased_with_width(f, weights, idx, special, 4)?;
        if square % 8 == 7 {
            write!(f, ",")?;
        } else {
            write!(f, ",{TAB}")?;
        }
    }
    writeln!(f)?;
    if piece.is_some() {
        writeln!(f, "{TAB}],")?;
    } else {
        writeln!(f, "];")?;
    }

    Ok(())
}

fn write_psqts(f: &mut Formatter<'_>, weights: &[Weight], special_entries: &[bool]) -> std::fmt::Result {
    writeln!(f, "const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [")?;
    for piece in ChessPieceType::pieces() {
        write_phased_psqt(f, weights, special_entries, Some(piece), 64 * piece as usize)?;
    }
    writeln!(f, "];")?;
    Ok(())
}
