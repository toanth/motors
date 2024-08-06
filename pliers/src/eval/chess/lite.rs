//! The hand-crafted eval used by the `caps` chess engine.

use crate::eval::chess::{write_phased_psqt, write_psqts, SkipChecks, NUM_PSQT_FEATURES};
use crate::eval::EvalScale::Scale;
use crate::eval::{changed_at_least, write_phased, Eval, EvalScale, WeightsInterpretation};
use crate::gd::{Float, TaperedDatapoint, Weight, Weights};
use crate::trace::{SingleFeature, SparseTrace, TraceTrait};
use gears::games::chess::pieces::ChessPieceType::*;
use gears::games::chess::pieces::{ChessPieceType, NUM_CHESS_PIECES};
use gears::games::chess::see::SEE_SCORES;
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::chess::ChessColor::White;
use gears::games::chess::{ChessColor, Chessboard};
use motors::eval::chess::lite::GenericLiTEval;
use motors::eval::chess::lite_values::{LiteValues, MAX_MOBILITY};
use motors::eval::chess::FileOpenness::*;
use motors::eval::chess::{FileOpenness, NUM_PAWN_SHIELD_CONFIGURATIONS};
use motors::eval::ScoreType;
use std::fmt::Formatter;
use strum::IntoEnumIterator;

#[derive(Debug, Default, Copy, Clone)]
struct LiTETrace {}

impl LiTETrace {
    const ONE_BISHOP_PAIR_FEATURE: usize = 1;
    const NUM_ROOK_OPENNESS_FEATURES: usize = 3;
    const NUM_KING_OPENNESS_FEATURES: usize = 3;
    const NUM_BISHOP_OPENNESS_FEATURES: usize = 4 * 8;
    const NUM_PASSED_PAWN_FEATURES: usize = NUM_SQUARES;
    const NUM_UNSUPPORTED_PAWN_FEATURES: usize = 1;
    const NUM_PASSED_PAWNS_INSIDE_SQUARE_FEATURES: usize = 8;
    const NUM_DOUBLED_PAWN_FEATURES: usize = 1;
    const NUM_PAWN_PROTECTION_FEATURES: usize = NUM_CHESS_PIECES;
    const NUM_PAWN_ATTACKS_FEATURES: usize = NUM_CHESS_PIECES;
    const NUM_MOBILITY_FEATURES: usize = (MAX_MOBILITY + 1) * (NUM_CHESS_PIECES - 1);
    const NUM_THREAT_FEATURES: usize = (NUM_CHESS_PIECES - 1) * NUM_CHESS_PIECES;
    const NUM_DEFENSE_FEATURES: usize = (NUM_CHESS_PIECES - 1) * NUM_CHESS_PIECES;
    const NUM_KING_ZONE_ATTACK_FEATURES: usize = NUM_CHESS_PIECES;

    const PASSED_PAWN_OFFSET: usize = NUM_PSQT_FEATURES;
    const UNSUPPORTED_PAWN_OFFSET: usize =
        Self::PASSED_PAWN_OFFSET + Self::NUM_PASSED_PAWN_FEATURES;
    const DOUBLED_PAWN_OFFSET: usize =
        Self::UNSUPPORTED_PAWN_OFFSET + Self::NUM_UNSUPPORTED_PAWN_FEATURES;
    const PASSED_PAWN_INSIDE_SQUARE_OFFSEET: usize =
        Self::DOUBLED_PAWN_OFFSET + Self::NUM_DOUBLED_PAWN_FEATURES;
    const BISHOP_PAIR_OFFSET: usize =
        Self::PASSED_PAWN_INSIDE_SQUARE_OFFSEET + Self::NUM_PASSED_PAWNS_INSIDE_SQUARE_FEATURES;
    const ROOK_OPENNESS_OFFSET: usize = Self::BISHOP_PAIR_OFFSET + Self::ONE_BISHOP_PAIR_FEATURE;
    const KING_OPENNESS_OFFSET: usize =
        Self::ROOK_OPENNESS_OFFSET + Self::NUM_ROOK_OPENNESS_FEATURES;
    const BISHOP_OPENNESS_OFFSET: usize =
        Self::KING_OPENNESS_OFFSET + Self::NUM_KING_OPENNESS_FEATURES;
    const PAWN_SHIELD_OFFSET: usize =
        Self::BISHOP_OPENNESS_OFFSET + Self::NUM_BISHOP_OPENNESS_FEATURES;
    const PAWN_PROTECTION_OFFSET: usize = Self::PAWN_SHIELD_OFFSET + NUM_PAWN_SHIELD_CONFIGURATIONS;
    const PAWN_ATTACKS_OFFSET: usize =
        Self::PAWN_PROTECTION_OFFSET + Self::NUM_PAWN_PROTECTION_FEATURES;
    const MOBILITY_OFFSET: usize = Self::PAWN_ATTACKS_OFFSET + Self::NUM_PAWN_ATTACKS_FEATURES;
    const THREAT_OFFSET: usize = Self::MOBILITY_OFFSET + Self::NUM_MOBILITY_FEATURES;
    const DEFENSE_OFFSET: usize = Self::THREAT_OFFSET + Self::NUM_THREAT_FEATURES;
    const KING_ZONE_ATTACK_OFFSET: usize = Self::DEFENSE_OFFSET + Self::NUM_DEFENSE_FEATURES;

    const NUM_FEATURES: usize = Self::KING_ZONE_ATTACK_OFFSET + Self::NUM_KING_ZONE_ATTACK_FEATURES;
}

impl LiteValues for LiTETrace {
    type Score = SparseTrace;

    fn psqt(square: ChessSquare, piece: ChessPieceType, color: ChessColor) -> SingleFeature {
        let square = square.flip_if(color == White);
        let idx = square.bb_idx() + piece as usize * NUM_SQUARES;
        SingleFeature::new(idx)
    }

    fn passed_pawn(square: ChessSquare) -> SingleFeature {
        let idx = Self::PASSED_PAWN_OFFSET + square.bb_idx();
        SingleFeature::new(idx)
    }

    fn unsupported_pawn() -> <Self::Score as ScoreType>::SingleFeatureScore {
        let idx = Self::UNSUPPORTED_PAWN_OFFSET;
        SingleFeature::new(idx)
    }

    fn doubled_pawn() -> <Self::Score as ScoreType>::SingleFeatureScore {
        let idx = Self::DOUBLED_PAWN_OFFSET;
        SingleFeature::new(idx)
    }

    fn passed_pawns_outside_square_rule(
        distance: usize,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore {
        assert!(distance < Self::NUM_PASSED_PAWNS_INSIDE_SQUARE_FEATURES);
        let idx = Self::PASSED_PAWN_INSIDE_SQUARE_OFFSEET + distance;
        SingleFeature::new(idx)
    }

    fn bishop_pair() -> SingleFeature {
        let idx = Self::BISHOP_PAIR_OFFSET;
        SingleFeature::new(idx)
    }

    fn rook_openness(openness: FileOpenness) -> SingleFeature {
        if openness == SemiClosed {
            return SingleFeature::no_feature();
        }
        let idx = Self::ROOK_OPENNESS_OFFSET + openness as usize;
        SingleFeature::new(idx)
    }

    fn king_openness(openness: FileOpenness) -> SingleFeature {
        if openness == SemiClosed {
            return SingleFeature::no_feature();
        }
        let idx = Self::KING_OPENNESS_OFFSET + openness as usize;
        SingleFeature::new(idx)
    }

    fn bishop_openness(
        openness: FileOpenness,
        len: usize,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore {
        debug_assert!(len <= 8);
        let idx = Self::BISHOP_OPENNESS_OFFSET + openness as usize * 8 + len - 1;
        SingleFeature::new(idx)
    }

    fn pawn_shield(config: usize) -> SingleFeature {
        let idx = Self::PAWN_SHIELD_OFFSET + config;
        SingleFeature::new(idx)
    }

    fn pawn_protection(piece: ChessPieceType) -> SingleFeature {
        let idx = Self::PAWN_PROTECTION_OFFSET + piece as usize;
        SingleFeature::new(idx)
    }

    fn pawn_attack(piece: ChessPieceType) -> SingleFeature {
        // For example a pawn attacking another pawn is itself attacked by a pawn, but since a pawn could be attacking
        // two pawns at once this doesn't have to mean that the resulting feature count is zero. So manually exclude this
        // because pawns attacking pawns don't necessarily create an immediate thread like pawns attacking pieces.
        if piece == Pawn {
            return SingleFeature::no_feature();
        }
        let idx = Self::PAWN_ATTACKS_OFFSET + piece as usize;
        SingleFeature::new(idx)
    }

    fn mobility(piece: ChessPieceType, mobility: usize) -> SingleFeature {
        let idx = Self::MOBILITY_OFFSET + (piece as usize - 1) * (MAX_MOBILITY + 1) + mobility;
        SingleFeature::new(idx)
    }

    fn threats(attacking: ChessPieceType, targeted: ChessPieceType) -> SingleFeature {
        let idx =
            Self::THREAT_OFFSET + (attacking as usize - 1) * NUM_CHESS_PIECES + targeted as usize;
        SingleFeature::new(idx)
    }

    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> SingleFeature {
        let idx =
            Self::DEFENSE_OFFSET + (protecting as usize - 1) * NUM_CHESS_PIECES + target as usize;
        SingleFeature::new(idx)
    }

    fn king_zone_attack(
        attacking: ChessPieceType,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore {
        let idx = Self::KING_ZONE_ATTACK_OFFSET + attacking as usize;
        SingleFeature::new(idx)
    }
}

#[derive(Debug, Default)]
/// Tuning the chess Linear Tuned Eval (`LiTE`) values.
/// This is done by re-using the generic eval function but instantiating it with a trace instead of a score.
pub struct TuneLiTEval {}

impl WeightsInterpretation for TuneLiTEval {
    // TODO: Make shorter
    #[allow(clippy::too_many_lines)]
    fn display(&self) -> fn(&mut Formatter, &Weights, &[Weight]) -> std::fmt::Result {
        |f: &mut Formatter<'_>, weights: &Weights, old_weights: &[Weight]| {
            let special = changed_at_least(-1.0, weights, old_weights);
            assert_eq!(weights.len(), Self::NUM_WEIGHTS);

            write_psqts(f, weights, &special)?;
            writeln!(f, "\n#[rustfmt::skip]")?;
            write!(f, "const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =")?;
            write_phased_psqt(f, weights, &special, None, NUM_PSQT_FEATURES)?;
            let mut idx = LiTETrace::UNSUPPORTED_PAWN_OFFSET;

            writeln!(
                f,
                "const UNSUPPORTED_PAWN: PhasedScore = {};",
                write_phased(weights, idx, &special)
            )?;
            idx += 1;
            writeln!(
                f,
                "const DOUBLED_PAWN: PhasedScore = {};",
                write_phased(weights, idx, &special)
            )?;
            idx += 1;

            writeln!(
                f,
                "const PASSED_PAWN_INSIDE_SQUARE_RULE: [PhasedScore; 8] = ["
            )?;
            for _ in 0..LiTETrace::NUM_PASSED_PAWNS_INSIDE_SQUARE_FEATURES {
                write!(f, "{}, ", write_phased(weights, idx, &special))?;
                idx += 1;
            }
            writeln!(f, "];")?;

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
            writeln!(f, "#[rustfmt::skip]")?;
            writeln!(f, "const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [")?;
            for openness in FileOpenness::iter() {
                write!(f, "    // {openness}\n    [")?;
                for _len in 0..8 {
                    write!(f, "{}, ", write_phased(weights, idx, &special))?;
                    idx += 1;
                }
                writeln!(f, "], ")?;
            }
            writeln!(f, "];")?;

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
                idx += 1;
            }
            writeln!(f, "\n];")?;
            writeln!(
                f,
                " const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = ["
            )?;
            for _feature in 0..LiTETrace::NUM_PAWN_ATTACKS_FEATURES {
                write!(f, "{}, ", write_phased(weights, idx, &special))?;
                idx += 1;
            }
            writeln!(f, "\n];")?;
            writeln!(f, "\npub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;")?;
            writeln!(
                f,
                "const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = ["
            )?;
            for _piece in ChessPieceType::non_pawn_pieces() {
                write!(f, "[")?;
                for _mobility in 0..=MAX_MOBILITY {
                    write!(f, "{}, ", write_phased(weights, idx, &special))?;
                    idx += 1;
                }
                writeln!(f, "],")?;
            }
            writeln!(f, "];")?;
            for name in ["THREATS", "DEFENDED"] {
                writeln!(
                    f,
                    "const {name}: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = ["
                )?;
                for _piece in ChessPieceType::non_pawn_pieces() {
                    write!(f, "[")?;
                    for _threatened in ChessPieceType::pieces() {
                        write!(f, "{}, ", write_phased(weights, idx, &special))?;
                        idx += 1;
                    }
                    writeln!(f, "],")?;
                }
                writeln!(f, "];")?;
            }
            write!(f, "const KING_ZONE_ATTACK: [PhasedScore; 6] = [")?;
            for _piece in ChessPieceType::pieces() {
                write!(f, "{}, ", write_phased(weights, idx, &special))?;
                idx += 1;
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

    fn initial_weights(&self) -> Option<Weights> {
        let mut weights = vec![Weight(0.0); Self::NUM_WEIGHTS];
        for piece in ChessPieceType::non_king_pieces() {
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
        GenericLiTEval::<LiTETrace>::do_eval(pos)
    }
}
