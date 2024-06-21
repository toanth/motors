//! The hand-crafted eval used by the `caps` chess engine.

use crate::eval::chess::{
    psqt_trace, write_phased_psqt, write_psqts, SkipChecks, NUM_PHASES, NUM_PSQT_FEATURES,
};
use crate::eval::EvalScale::Scale;
use crate::eval::{changed_at_least, write_phased, Eval, EvalScale, WeightsInterpretation};
use crate::gd::{
    BasicTrace, Float, SingleFeatureTrace, TaperedDatapoint, TraceNFeatures, TraceTrait, Weight,
    Weights,
};
use gears::games::chess::pieces::UncoloredChessPiece::*;
use gears::games::chess::pieces::{UncoloredChessPiece, NUM_CHESS_PIECES};
use gears::games::chess::see::SEE_SCORES;
use gears::games::chess::squares::NUM_SQUARES;
use gears::games::chess::zobrist::NUM_PIECE_SQUARE_ENTRIES;
use gears::games::chess::Chessboard;
use gears::games::Color;
use gears::games::Color::*;
use gears::general::bitboards::chess::A_FILE;
use gears::general::bitboards::{Bitboard, RawBitboard};
use motors::eval::chess::hce::file_openness;
use motors::eval::chess::FileOpenness::SemiClosed;
use motors::eval::chess::{pawn_shield_idx, NUM_PAWN_SHIELD_CONFIGURATIONS};
use std::fmt::Formatter;
use strum::IntoEnumIterator;

#[derive(Debug, Default)]
struct Trace {
    psqt: TraceNFeatures<NUM_PIECE_SQUARE_ENTRIES>,
    passed_pawns: TraceNFeatures<NUM_PASSED_PAWN_FEATURES>,
    bishop_pair: SingleFeatureTrace,
    rooks: TraceNFeatures<NUM_ROOK_OPENNESS_FEATURES>,
    kings: TraceNFeatures<NUM_KING_OPENNESS_FEATURES>,
    pawn_shields: TraceNFeatures<NUM_PAWN_SHIELD_CONFIGURATIONS>,
    pawn_protection: TraceNFeatures<NUM_PAWN_PROTECTION_FEATURES>,
    pawn_attack: TraceNFeatures<NUM_PAWN_ATTACK_FEATURES>,
}

impl TraceTrait for Trace {
    fn nested_traces(&self) -> Vec<&dyn TraceTrait> {
        vec![
            &self.psqt,
            &self.passed_pawns,
            &self.bishop_pair,
            &self.rooks,
            &self.kings,
            &self.pawn_shields,
            &self.pawn_protection,
            &self.pawn_attack,
        ]
    }

    fn phase(&self) -> Float {
        self.psqt.phase()
    }
}

/// The hand-crafted eval used by the `caps` chess engine.
#[derive(Debug, Default)]
pub struct CapsHceEval {}

impl WeightsInterpretation for CapsHceEval {
    fn display(&self) -> fn(&mut Formatter, &Weights, &[Weight]) -> std::fmt::Result {
        |f: &mut Formatter<'_>, weights: &Weights, old_weights: &[Weight]| {
            let special = changed_at_least(-1.0, weights, old_weights);
            assert_eq!(weights.len(), Self::NUM_WEIGHTS);

            write_psqts(f, weights, &special)?;
            writeln!(f, "\n#[rustfmt::skip]")?;
            write!(f, "const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =")?;
            write_phased_psqt(
                f,
                &weights[NUM_PHASES * NUM_PSQT_FEATURES..],
                &special,
                0,
                None,
            )?;
            writeln!(f, "];")?;
            let mut idx = NUM_PSQT_FEATURES + NUM_PASSED_PAWN_FEATURES;

            writeln!(
                f,
                "const BISHOP_PAIR: PhasedScore = {};",
                write_phased(weights, idx, &special),
            )?;
            idx += 1;

            for piece in ["ROOK", "KING"] {
                for openness in ["OPEN", "CLOSED", "SEMIOPEN"] {
                    writeln!(
                        f,
                        "const {piece}_{openness}_FILE: PhasedScore = {};",
                        write_phased(weights, idx, &special)
                    )?;
                    idx += 1;
                }
            }
            writeln!(
                f,
                "const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = ["
            )?;
            for i in 0..NUM_PAWN_SHIELD_CONFIGURATIONS {
                let config = if i < 1 << 6 {
                    format!("{i:#06b}")
                } else if i < (1 << 6) + (1 << 4) {
                    format!("{:#04b}", i - (1 << 6))
                } else {
                    format!("{:#04b}", i - (1 << 6) - (1 << 4))
                };
                write!(f, "{} /*{config}*/, ", write_phased(weights, idx, &special),)?;
                idx += 1;
            }
            writeln!(f, "];")?;
            writeln!(
                f,
                " const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = ["
            )?;
            for _feature in 0..NUM_PAWN_PROTECTION_FEATURES {
                write!(f, "{}, ", write_phased(weights, idx, &special))?;
                idx += 1
            }
            writeln!(f, "\n];")?;
            writeln!(f, "const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [")?;
            for _feature in 0..NUM_PAWN_ATTACK_FEATURES {
                write!(f, "{}, ", write_phased(weights, idx, &special))?;
                idx += 1;
            }
            writeln!(f, "\n];")?;
            assert_eq!(idx, Self::NUM_FEATURES);
            Ok(())
        }
    }

    fn eval_scale(&self) -> EvalScale {
        Scale(120.0)
    }

    fn retune_from_zero(&self) -> bool {
        false
    }

    fn interpolate_decay(&self) -> Option<Float> {
        Some(0.99) // a relatively small value (far away from 1) because some pawn shield configurations are very uncommon
    }

    fn initial_weights(&self) -> Option<Weights> {
        let mut weights = vec![Weight(0.0); Self::NUM_WEIGHTS];
        for piece in UncoloredChessPiece::non_king_pieces() {
            let piece_val = Weight(SEE_SCORES[piece as usize].0 as Float);
            for square in 0..NUM_SQUARES {
                let i = piece as usize * 64 + square;
                weights[2 * i] = piece_val;
                weights[2 * i + 1] = piece_val;
            }
        }
        Some(Weights(weights))
    }
}

const NUM_ROOK_OPENNESS_FEATURES: usize = 3;
const NUM_KING_OPENNESS_FEATURES: usize = 3;
const NUM_PASSED_PAWN_FEATURES: usize = NUM_SQUARES;
const NUM_PAWN_PROTECTION_FEATURES: usize = NUM_CHESS_PIECES;
const NUM_PAWN_ATTACK_FEATURES: usize = NUM_CHESS_PIECES;
const ONE_BISHOP_PAIR_FEATURE: usize = 1;

impl Eval<Chessboard> for CapsHceEval {
    const NUM_WEIGHTS: usize = Self::NUM_FEATURES * NUM_PHASES;

    const NUM_FEATURES: usize = NUM_PIECE_SQUARE_ENTRIES
        + ONE_BISHOP_PAIR_FEATURE
        + NUM_PASSED_PAWN_FEATURES
        + NUM_ROOK_OPENNESS_FEATURES
        + NUM_KING_OPENNESS_FEATURES
        + NUM_PAWN_SHIELD_CONFIGURATIONS
        + NUM_PAWN_PROTECTION_FEATURES
        + NUM_PAWN_ATTACK_FEATURES;

    type D = TaperedDatapoint;
    type Filter = SkipChecks;

    fn feature_trace(pos: &Chessboard) -> impl TraceTrait {
        Self::trace(pos)
    }
}

impl CapsHceEval {
    #[allow(clippy::field_reassign_with_default)]
    fn trace(pos: &Chessboard) -> Trace {
        let mut trace = Trace::default();
        trace.psqt = psqt_trace(pos);
        for color in Color::iter() {
            let our_pawns = pos.colored_piece_bb(color, Pawn);
            let their_pawns = pos.colored_piece_bb(color.other(), Pawn);

            for pawn in our_pawns.ones() {
                // Passed pawns.
                let in_front =
                    (A_FILE << (pawn.flip_if(color == Black).bb_idx() + 8)).flip_if(color == Black);
                let blocking = in_front | in_front.west() | in_front.east();
                if (in_front & our_pawns).is_zero() && (blocking & their_pawns).is_zero() {
                    let square = pawn.flip_if(color == White).bb_idx();
                    trace.passed_pawns.increment(square, color);
                }
            }
            if pos.colored_piece_bb(color, Bishop).more_than_one_bit_set() {
                trace.bishop_pair.increment(0, color);
            }

            for piece in UncoloredChessPiece::pieces() {
                let pawn_attacks = our_pawns.pawn_attacks(color);
                let protected_by_pawns = pawn_attacks & pos.colored_piece_bb(color, piece);
                trace.pawn_protection.increment_by(
                    piece as usize,
                    color,
                    protected_by_pawns.num_ones() as isize,
                );
                // a pawn attacking another pawn is itself attacked by a pawn, but since a pawn could be attacking two pawns
                // at once this doesn't have to mean that the resulting feature count is zero. So manually exclude this
                // because pawns attacking pawns don't necessarily create an immediate thread like pawns attacking pieces.
                if piece != Pawn {
                    let attacked_by_pawns =
                        pawn_attacks & pos.colored_piece_bb(color.other(), piece);
                    trace.pawn_attack.increment_by(
                        piece as usize,
                        color,
                        attacked_by_pawns.num_ones() as isize,
                    );
                }
            }

            // Rooks on (semi)open/closed files (semi-closed files are handled by adjusting the base rook values during tuning)
            let rooks = pos.colored_piece_bb(color, Rook);
            for rook in rooks.ones() {
                let openness = file_openness(rook.file(), our_pawns, their_pawns);
                if openness != SemiClosed {
                    // part of the normal piece value (i.e. part of the rook PSQT)
                    trace.rooks.increment(openness as usize, color);
                }
            }
            // King on (semi)open/closed file
            let king_square = pos.king_square(color);
            let king_file = king_square.file();
            let openness = file_openness(king_file, our_pawns, their_pawns);
            if openness != SemiClosed {
                trace.kings.increment(openness as usize, color);
            }
            let pawn_shield = pawn_shield_idx(our_pawns, king_square, color);
            trace.pawn_shields.increment(pawn_shield, color);
        }
        trace
    }
}
