//! The hand-crafted eval used by the `caps` chess engine.

use crate::eval::chess::{
    write_phased_psqt, write_psqts, SkipChecks, NUM_PHASES, NUM_PSQT_FEATURES,
};
use crate::eval::EvalScale::Scale;
use crate::eval::{changed_at_least, write_phased, Eval, EvalScale, WeightsInterpretation};
use crate::gd::{Float, TaperedDatapoint, Weight, Weights};
use crate::trace::{SparseTrace, TraceTrait};
use gears::games::chess::pieces::UncoloredChessPiece::*;
use gears::games::chess::pieces::{UncoloredChessPiece, NUM_CHESS_PIECES};
use gears::games::chess::see::SEE_SCORES;
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::chess::Chessboard;
use gears::games::Color;
use gears::games::Color::*;
use motors::eval::chess::lite::GenericLiTEval;
use motors::eval::chess::lite_values::{LiteValues, MAX_MOBILITY};
use motors::eval::chess::FileOpenness::SemiClosed;
use motors::eval::chess::{FileOpenness, NUM_PAWN_SHIELD_CONFIGURATIONS};
use std::fmt::Formatter;

#[derive(Debug, Default, Copy, Clone)]
struct LiTETrace {}

impl LiTETrace {
    const ONE_BISHOP_PAIR_FEATURE: usize = 1;
    const NUM_ROOK_OPENNESS_FEATURES: usize = 3;
    const NUM_KING_OPENNESS_FEATURES: usize = 3;
    const NUM_PASSED_PAWN_FEATURES: usize = NUM_SQUARES;
    const NUM_PAWN_PROTECTION_FEATURES: usize = NUM_CHESS_PIECES;
    const NUM_PAWN_ATTACKS_FEATURES: usize = NUM_CHESS_PIECES;
    const NUM_MOBILITY_FEATURES: usize = (MAX_MOBILITY + 1) * (NUM_CHESS_PIECES - 1);
    const NUM_THREAT_FEATURES: usize = (NUM_CHESS_PIECES - 1) * NUM_CHESS_PIECES;
    const NUM_DEFENSE_FEATURES: usize = (NUM_CHESS_PIECES - 1) * NUM_CHESS_PIECES;

    const PASSED_PAWN_OFFSET: usize = NUM_PSQT_FEATURES;
    const BISHOP_PAIR_OFFSET: usize = Self::PASSED_PAWN_OFFSET + Self::NUM_PASSED_PAWN_FEATURES;
    const ROOK_OPENNESS_OFFSET: usize = Self::BISHOP_PAIR_OFFSET + Self::ONE_BISHOP_PAIR_FEATURE;
    const KING_OPENNESS_OFFSET: usize =
        Self::ROOK_OPENNESS_OFFSET + Self::NUM_ROOK_OPENNESS_FEATURES;
    const PAWN_SHIELD_OFFSET: usize = Self::KING_OPENNESS_OFFSET + Self::NUM_KING_OPENNESS_FEATURES;
    const PAWN_PROTECTION_OFFSET: usize = Self::PAWN_SHIELD_OFFSET + NUM_PAWN_SHIELD_CONFIGURATIONS;
    const PAWN_ATTACKS_OFFSET: usize =
        Self::PAWN_PROTECTION_OFFSET + Self::NUM_PAWN_PROTECTION_FEATURES;
    const MOBILITY_OFFSET: usize = Self::PAWN_ATTACKS_OFFSET + Self::NUM_PAWN_ATTACKS_FEATURES;
    const THREAT_OFFSET: usize = Self::MOBILITY_OFFSET + Self::NUM_MOBILITY_FEATURES;
    const DEFENSE_OFFSET: usize = Self::THREAT_OFFSET + Self::NUM_THREAT_FEATURES;

    const NUM_FEATURES: usize = Self::DEFENSE_OFFSET + Self::NUM_DEFENSE_FEATURES;
}

impl LiteValues for LiTETrace {
    type Score = SparseTrace;

    fn psqt(square: ChessSquare, piece: UncoloredChessPiece, color: Color) -> Self::Score {
        let square = square.flip_if(color == White);
        let idx = 0 + square.bb_idx() + piece as usize * NUM_SQUARES;
        SparseTrace::new(idx)
    }

    fn passed_pawn(square: ChessSquare) -> Self::Score {
        let idx = Self::PASSED_PAWN_OFFSET + square.bb_idx();
        SparseTrace::new(idx)
    }

    fn bishop_pair() -> Self::Score {
        let idx = Self::BISHOP_PAIR_OFFSET;
        SparseTrace::new(idx)
    }

    fn rook_openness(openness: FileOpenness) -> Self::Score {
        if openness == SemiClosed {
            return SparseTrace::default();
        }
        let idx = Self::ROOK_OPENNESS_OFFSET + openness as usize;
        SparseTrace::new(idx)
    }

    fn king_openness(openness: FileOpenness) -> Self::Score {
        if openness == SemiClosed {
            return SparseTrace::default();
        }
        let idx = Self::KING_OPENNESS_OFFSET + openness as usize;
        SparseTrace::new(idx)
    }

    fn pawn_shield(config: usize) -> Self::Score {
        let idx = Self::PAWN_SHIELD_OFFSET + config;
        SparseTrace::new(idx)
    }

    fn pawn_protection(piece: UncoloredChessPiece) -> Self::Score {
        let idx = Self::PAWN_PROTECTION_OFFSET + piece as usize;
        SparseTrace::new(idx)
    }

    fn pawn_attack(piece: UncoloredChessPiece) -> Self::Score {
        // For example a pawn attacking another pawn is itself attacked by a pawn, but since a pawn could be attacking
        // two pawns at once this doesn't have to mean that the resulting feature count is zero. So manually exclude this
        // because pawns attacking pawns don't necessarily create an immediate thread like pawns attacking pieces.
        if piece == Pawn {
            return SparseTrace::default();
        }
        let idx = Self::PAWN_ATTACKS_OFFSET + piece as usize;
        SparseTrace::new(idx)
    }

    fn mobility(piece: UncoloredChessPiece, mobility: usize) -> Self::Score {
        let idx = Self::MOBILITY_OFFSET + (piece as usize - 1) * (MAX_MOBILITY + 1) + mobility;
        SparseTrace::new(idx)
    }

    fn threats(attacking: UncoloredChessPiece, targeted: UncoloredChessPiece) -> Self::Score {
        let idx =
            Self::THREAT_OFFSET + (attacking as usize - 1) * NUM_CHESS_PIECES + targeted as usize;
        SparseTrace::new(idx)
    }

    fn defended(protecting: UncoloredChessPiece, target: UncoloredChessPiece) -> Self::Score {
        let idx =
            Self::DEFENSE_OFFSET + (protecting as usize - 1) * NUM_CHESS_PIECES + target as usize;
        SparseTrace::new(idx)
    }
}

#[derive(Debug, Default)]
/// Tuning the chess Linear Tuned Eval (LiTE) values.
/// This is done by re-using the generic eval function but instantiating it with a trace instead of a score.
pub struct TuneLiTEval {}

impl WeightsInterpretation for TuneLiTEval {
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
            let mut idx = LiTETrace::BISHOP_PAIR_OFFSET;

            writeln!(
                f,
                "\nconst BISHOP_PAIR: PhasedScore = {};",
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
            for _feature in 0..LiTETrace::NUM_PAWN_PROTECTION_FEATURES {
                write!(f, "{}, ", write_phased(weights, idx, &special))?;
                idx += 1
            }
            writeln!(f, "\n];")?;
            writeln!(
                f,
                " const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = ["
            )?;
            for _feature in 0..LiTETrace::NUM_PAWN_ATTACKS_FEATURES {
                write!(f, "{}, ", write_phased(weights, idx, &special))?;
                idx += 1
            }
            writeln!(f, "\n];")?;
            writeln!(f, "\npub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;")?;
            writeln!(
                f,
                "const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = ["
            )?;
            for _piece in UncoloredChessPiece::non_pawn_pieces() {
                write!(f, "[")?;
                for _mobility in 0..=MAX_MOBILITY {
                    write!(f, "{}, ", write_phased(weights, idx, &special))?;
                    idx += 1;
                }
                writeln!(f, "],")?;
            }
            writeln!(f, "];")?;
            writeln!(
                f,
                "const THREATS: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = ["
            )?;
            for _piece in UncoloredChessPiece::non_pawn_pieces() {
                write!(f, "[")?;
                for _threatened in UncoloredChessPiece::pieces() {
                    write!(f, "{}, ", write_phased(weights, idx, &special))?;
                    idx += 1;
                }
                writeln!(f, "],")?;
            }
            writeln!(f, "];")?;
            writeln!(
                f,
                "const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = ["
            )?;
            for _piece in UncoloredChessPiece::non_pawn_pieces() {
                write!(f, "[")?;
                for _threatened in UncoloredChessPiece::pieces() {
                    write!(f, "{}, ", write_phased(weights, idx, &special))?;
                    idx += 1;
                }
                writeln!(f, "],")?;
            }
            writeln!(f, "];")?;
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

impl Eval<Chessboard> for TuneLiTEval {
    const NUM_WEIGHTS: usize = Self::NUM_FEATURES * 2;
    const NUM_FEATURES: usize = LiTETrace::NUM_FEATURES;
    type D = TaperedDatapoint;
    type Filter = SkipChecks;

    fn feature_trace(pos: &Chessboard) -> impl TraceTrait {
        let res = GenericLiTEval::<LiTETrace>::do_eval(pos);
        res
    }
}
