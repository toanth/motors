//! Contains chess evaluation functions, and some shared code that is generally useful for them,
//! such as the [`SkipChecks`] [`Filter`].
use crate::eval::write_phased_with_width;
use crate::gd::{Float, Weight};
use crate::load_data::{Filter, ParseResult};
use crate::trace::{BasicTrace, SimpleTrace, TraceNFeatures};
use gears::games::chess::pieces::{UncoloredChessPiece, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::chess::Chessboard;
use gears::games::Color;
use gears::games::Color::White;
use gears::general::bitboards::RawBitboard;
use motors::eval::chess::CHESS_PHASE_VALUES;
use std::fmt::Formatter;
use strum::IntoEnumIterator;

pub mod lite;
pub mod material_only_eval;
pub mod piston_eval;

/// Remove positions where the side to move is in check.
pub struct SkipChecks {}

impl Filter<Chessboard> for SkipChecks {
    #[allow(refining_impl_trait)]
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
    let phase: usize = UncoloredChessPiece::non_king_pieces()
        .map(|piece| pos.piece_bb(piece).num_ones() * CHESS_PHASE_VALUES[piece as usize])
        .sum();
    phase as Float / 24.0
}

fn to_feature_idx(piece: UncoloredChessPiece, color: Color, square: ChessSquare) -> usize {
    NUM_SQUARES * piece as usize + square.flip_if(color == White).bb_idx()
}

fn psqt_trace(pos: &Chessboard) -> TraceNFeatures<NUM_PSQT_FEATURES> {
    let mut trace = SimpleTrace::for_features(NUM_PSQT_FEATURES);
    trace.phase = chess_phase(pos);
    for color in Color::iter() {
        for piece in UncoloredChessPiece::pieces() {
            let bb = pos.colored_piece_bb(color, piece);
            for square in bb.ones() {
                let idx = to_feature_idx(piece, color, square);
                trace.increment(idx, color);
            }
        }
    }
    TraceNFeatures(trace)
}

fn index(piece_idx: usize, square: usize) -> usize {
    64 * piece_idx + square
}

fn write_phased_psqt(
    f: &mut Formatter<'_>,
    weights: &[Weight],
    special: &[bool],
    piece_idx: usize,
    piece_name: Option<&str>,
) -> std::fmt::Result {
    const TAB: &str = "    "; // Use 4 spaces for a tab.
    if let Some(piece) = piece_name {
        writeln!(f, "{TAB}// {piece}")?;
        write!(f, "{TAB}[")?;
    } else {
        writeln!(f, "[")?;
    }
    for square in 0..NUM_SQUARES {
        if square % 8 == 0 {
            writeln!(f)?;
            write!(f, "{TAB}{TAB}")?;
        }
        let idx = index(piece_idx, square);

        let str = write_phased_with_width(weights, idx, special, 4);
        write!(f, "{str},{TAB}")?;
    }
    writeln!(f)?;
    if piece_name.is_some() {
        writeln!(f, "{TAB}],")?;
    } else {
        writeln!(f, "{TAB}];")?;
    }

    Ok(())
}

fn write_psqts(
    f: &mut Formatter<'_>,
    weights: &[Weight],
    special_entries: &[bool],
) -> std::fmt::Result {
    writeln!(
        f,
        "const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = ["
    )?;
    for piece in UncoloredChessPiece::pieces() {
        write_phased_psqt(
            f,
            weights,
            special_entries,
            piece as usize,
            Some(piece.name()),
        )?;
    }
    writeln!(f, "];")?;
    Ok(())
}
