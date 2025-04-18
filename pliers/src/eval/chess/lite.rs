//! The hand-crafted eval used by the `caps` chess engine.

use crate::eval::EvalScale::Scale;
use crate::eval::chess::lite::LiteFeatureSubset::*;
use crate::eval::chess::{SkipChecks, write_phased_psqt, write_psqts};
use crate::eval::{
    Eval, EvalScale, WeightsInterpretation, changed_at_least, write_2d_range_phased, write_phased, write_range_phased,
};
use crate::gd::{Float, Weight, Weights};
use crate::trace::{FeatureSubSet, SingleFeature, SparseTrace, TraceTrait};
use gears::games::DimT;
use gears::games::chess::ChessColor::White;
use gears::games::chess::pieces::ChessPieceType::*;
use gears::games::chess::pieces::{ChessPieceType, NUM_CHESS_PIECES};
use gears::games::chess::see::SEE_SCORES;
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::chess::{ChessColor, Chessboard};
use gears::general::common::StaticallyNamedEntity;
use motors::eval::chess::FileOpenness::*;
use motors::eval::chess::lite::GenericLiTEval;
use motors::eval::chess::lite_values::{LiteValues, MAX_MOBILITY};
use motors::eval::chess::{FileOpenness, NUM_PAWN_SHIELD_CONFIGURATIONS};
use std::fmt;
use std::fmt::{Display, Formatter};
use std::iter::Iterator;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, FromRepr};

#[derive(Debug, Default, Copy, Clone)]
struct LiTETrace {}

/// All features considered by LiTE.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, EnumIter, FromRepr, derive_more::Display)]
pub enum LiteFeatureSubset {
    Psqt,
    BishopPair,
    BadBishop,
    RookOpenness,
    KingOpenness,
    BishopOpenness,
    PawnAdvancedCenter,
    PawnPassiveCenter,
    PawnShield,
    PawnlessFlank,
    StoppablePasser,
    CloseKingPasser,
    ImmobilePasser,
    ProtectedPasser,
    PassedPawn,
    CandidatePasser,
    UnsupportedPawn,
    DoubledPawn,
    Phalanx,
    PawnProtection,
    PawnAttacks,
    PawnAdvanceThreat,
    Mobility,
    Threat,
    Defense,
    KingZoneAttack,
    CanGiveCheck,
    CheckStm,
    DiscoveredCheckStm,
    DiscoveredCheck,
    Pin,
}

impl FeatureSubSet for LiteFeatureSubset {
    fn num_features(self) -> usize {
        match self {
            Psqt => NUM_SQUARES * NUM_CHESS_PIECES,
            BishopPair => 1,
            BadBishop => 9,
            RookOpenness => 3,
            KingOpenness => 3,
            BishopOpenness => 4 * 8,
            PawnAdvancedCenter => 1 << 6,
            PawnPassiveCenter => 1 << 6,
            PawnShield => NUM_PAWN_SHIELD_CONFIGURATIONS,
            StoppablePasser => 1,
            CloseKingPasser => 1,
            ImmobilePasser => 1,
            ProtectedPasser => 1,
            PassedPawn => NUM_SQUARES,
            CandidatePasser => 6,
            UnsupportedPawn => 1,
            DoubledPawn => 1,
            Phalanx => 6,
            PawnlessFlank => 1,
            PawnProtection => NUM_CHESS_PIECES,
            PawnAttacks => NUM_CHESS_PIECES,
            PawnAdvanceThreat => NUM_CHESS_PIECES,
            Mobility => (MAX_MOBILITY + 1) * (NUM_CHESS_PIECES - 1),
            Threat => (NUM_CHESS_PIECES - 1) * NUM_CHESS_PIECES,
            Defense => (NUM_CHESS_PIECES - 1) * NUM_CHESS_PIECES,
            KingZoneAttack => NUM_CHESS_PIECES,
            CanGiveCheck => NUM_CHESS_PIECES - 1,
            CheckStm => 1,
            DiscoveredCheckStm => 1,
            DiscoveredCheck => NUM_CHESS_PIECES,
            Pin => NUM_CHESS_PIECES - 1,
        }
    }

    fn start_idx(self) -> usize {
        Self::iter().take_while(|x| *x != self).map(|x| x.num_features()).sum()
    }

    fn write(self, f: &mut Formatter, weights: &Weights, special: &[bool]) -> fmt::Result {
        match self {
            Psqt => {
                return write_psqts(f, weights, special);
            }
            BishopPair => {
                write!(f, "\nconst BISHOP_PAIR: PhasedScore = ")?;
            }
            BadBishop => {
                write!(f, "const BAD_BISHOP: [PhasedScore; 9] = ")?;
            }
            RookOpenness => {
                for (i, openness) in ["OPEN", "CLOSED", "SEMIOPEN"].iter().enumerate() {
                    write!(f, "const ROOK_{openness}_FILE: PhasedScore = ")?;
                    write_phased(f, weights, self.start_idx() + i, special)?;
                    writeln!(f, ";")?;
                }
                return Ok(());
            }
            KingOpenness => {
                for (i, openness) in ["OPEN", "CLOSED", "SEMIOPEN"].iter().enumerate() {
                    write!(f, "const KING_{openness}_FILE: PhasedScore = ")?;
                    write_phased(f, weights, self.start_idx() + i, special)?;
                    writeln!(f, ";")?;
                }
                return Ok(());
            }
            BishopOpenness => {
                writeln!(f, "#[rustfmt::skip]")?;
                writeln!(f, "const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [")?;
                for openness in FileOpenness::iter() {
                    write!(f, "    // {openness}\n    [")?;
                    write_range_phased(f, weights, self.start_idx() + 8 * openness as usize, 8, special, false)?;
                    writeln!(f, "],")?;
                }
                return writeln!(f, "];");
            }
            PawnAdvancedCenter => {
                writeln!(f, "const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = ")?;
            }
            PawnPassiveCenter => {
                writeln!(f, "const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = ")?;
            }
            PawnShield => {
                writeln!(f, "const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [")?;
                for i in 0..NUM_PAWN_SHIELD_CONFIGURATIONS {
                    let config = if i < 1 << 6 {
                        format!("{i:#06b}")
                    } else if i < (1 << 6) + (1 << 4) {
                        format!("{:#04b}", i - (1 << 6))
                    } else {
                        format!("{:#04b}", i - (1 << 6) - (1 << 4))
                    };
                    write_phased(f, weights, self.start_idx() + i, special)?;
                    write!(f, " /*{config}*/, ")?;
                }
                return writeln!(f, "];");
            }
            StoppablePasser => {
                writeln!(f, "const STOPPABLE_PASSER: PhasedScore = ")?;
            }
            CloseKingPasser => {
                writeln!(f, "const CLOSE_KING_PASSER: PhasedScore = ")?;
            }
            ImmobilePasser => {
                write!(f, "const IMMOBILE_PASSER: PhasedScore = ")?;
            }
            ProtectedPasser => {
                write!(f, "const PROTECTED_PASSER: PhasedScore = ")?;
            }
            PassedPawn => {
                writeln!(f, "\n#[rustfmt::skip]")?;
                write!(f, "const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = ")?;
                return write_phased_psqt(f, weights, special, None, self.start_idx());
            }
            CandidatePasser => {
                writeln!(f, "const CANDIDATE_PASSER: [PhasedScore; 6] = ")?;
            }
            UnsupportedPawn => {
                write!(f, "const UNSUPPORTED_PAWN: PhasedScore = ")?;
            }
            DoubledPawn => {
                write!(f, "const DOUBLED_PAWN: PhasedScore = ")?;
            }
            Phalanx => {
                write!(f, "const PHALANX: [PhasedScore; 6] = ")?;
            }
            PawnlessFlank => {
                write!(f, "const PAWNLESS_FLANK: PhasedScore = ")?;
            }
            PawnProtection => {
                write!(f, "const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = ")?;
            }
            PawnAttacks => {
                write!(f, "const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = ")?;
            }
            PawnAdvanceThreat => {
                write!(f, "const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] = ")?;
            }
            Mobility => {
                writeln!(f, "\npub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;")?;
                writeln!(f, "const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [")?;
                for _piece in ChessPieceType::non_pawn_pieces() {
                    write_range_phased(
                        f,
                        weights,
                        self.start_idx() + (_piece as usize - 1) * (MAX_MOBILITY + 1),
                        MAX_MOBILITY + 1,
                        special,
                        true,
                    )?;
                    writeln!(f, ",")?;
                }
                return writeln!(f, "];");
            }
            Threat => {
                writeln!(f, "const THREATS: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = ")?;
                return write_2d_range_phased(
                    f,
                    weights,
                    self.start_idx(),
                    NUM_CHESS_PIECES,
                    NUM_CHESS_PIECES - 1,
                    special,
                );
            }
            Defense => {
                writeln!(f, "const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = ")?;
                return write_2d_range_phased(
                    f,
                    weights,
                    self.start_idx(),
                    NUM_CHESS_PIECES,
                    NUM_CHESS_PIECES - 1,
                    special,
                );
            }
            KingZoneAttack => {
                write!(f, "const KING_ZONE_ATTACK: [PhasedScore; 6] = ")?;
            }
            CanGiveCheck => {
                write!(f, "const CAN_GIVE_CHECK: [PhasedScore; 5] = ")?;
            }
            CheckStm => {
                write!(f, "const CHECK_STM: PhasedScore = ")?;
            }
            DiscoveredCheckStm => {
                write!(f, "const DISCOVERED_CHECK_STM: PhasedScore = ")?;
            }
            DiscoveredCheck => {
                write!(f, "const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] = ")?;
            }
            Pin => {
                write!(f, "const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = ")?;
            }
        }
        write_range_phased(f, weights, self.start_idx(), self.num_features(), special, true)?;
        writeln!(f, ";")
    }
}

impl LiTETrace {}

impl StaticallyNamedEntity for LiTETrace {
    fn static_short_name() -> impl Display
    where
        Self: Sized,
    {
        "tune lite"
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        Self::static_short_name().to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        Self::static_long_name()
    }
}

impl LiteValues for LiTETrace {
    type Score = SparseTrace;

    fn psqt(&self, square: ChessSquare, piece: ChessPieceType, color: ChessColor) -> SingleFeature {
        let square = square.flip_if(color == White);
        let idx = square.bb_idx() + piece as usize * NUM_SQUARES;
        SingleFeature::new(Psqt, idx)
    }

    fn passed_pawn(square: ChessSquare) -> SingleFeature {
        let idx = square.bb_idx();
        SingleFeature::new(PassedPawn, idx)
    }

    fn stoppable_passer() -> SingleFeature {
        SingleFeature::new(StoppablePasser, 0)
    }

    fn close_king_passer() -> SingleFeature {
        SingleFeature::new(CloseKingPasser, 0)
    }

    fn immobile_passer() -> SingleFeature {
        SingleFeature::new(ImmobilePasser, 0)
    }

    fn passer_protection() -> SingleFeature {
        SingleFeature::new(ProtectedPasser, 0)
    }

    fn candidate_passer(rank: DimT) -> SingleFeature {
        SingleFeature::new(CandidatePasser, rank as usize)
    }

    fn unsupported_pawn() -> SingleFeature {
        SingleFeature::new(UnsupportedPawn, 0)
    }

    fn doubled_pawn() -> SingleFeature {
        SingleFeature::new(DoubledPawn, 0)
    }

    fn phalanx(rank: DimT) -> SingleFeature {
        SingleFeature::new(Phalanx, rank as usize)
    }

    fn bishop_pair() -> SingleFeature {
        SingleFeature::new(BishopPair, 0)
    }

    fn bad_bishop(num_pawns: usize) -> SingleFeature {
        SingleFeature::new(BadBishop, num_pawns)
    }

    fn rook_openness(openness: FileOpenness) -> SingleFeature {
        if openness == SemiClosed {
            return SingleFeature::no_feature(RookOpenness);
        }
        SingleFeature::new(RookOpenness, openness as usize)
    }

    fn king_openness(openness: FileOpenness) -> SingleFeature {
        if openness == SemiClosed {
            return SingleFeature::no_feature(KingOpenness);
        }
        SingleFeature::new(KingOpenness, openness as usize)
    }

    fn bishop_openness(openness: FileOpenness, len: usize) -> SingleFeature {
        debug_assert!(len <= 8);
        let idx = openness as usize * 8 + len - 1;
        SingleFeature::new(BishopOpenness, idx)
    }

    fn pawn_advanced_center(config: usize) -> SingleFeature {
        SingleFeature::new(PawnAdvancedCenter, config)
    }

    fn pawn_passive_center(config: usize) -> SingleFeature {
        SingleFeature::new(PawnPassiveCenter, config)
    }

    fn pawn_shield(&self, _color: ChessColor, config: usize) -> SingleFeature {
        SingleFeature::new(PawnShield, config)
    }

    fn pawnless_flank() -> SingleFeature {
        SingleFeature::new(PawnlessFlank, 0)
    }

    fn pawn_protection(piece: ChessPieceType) -> SingleFeature {
        SingleFeature::new(PawnProtection, piece as usize)
    }

    fn pawn_attack(piece: ChessPieceType) -> SingleFeature {
        // For example a pawn attacking another pawn is itself attacked by a pawn, but since a pawn could be attacking
        // two pawns at once this doesn't have to mean that the resulting feature count is zero. So manually exclude this
        // because pawns attacking pawns don't necessarily create an immediate thread like pawns attacking pieces.
        if piece == Pawn {
            return SingleFeature::no_feature(PawnAttacks);
        }
        SingleFeature::new(PawnAttacks, piece as usize)
    }

    fn pawn_advance_threat(piece: ChessPieceType) -> SingleFeature {
        SingleFeature::new(PawnAdvanceThreat, piece as usize)
    }

    fn mobility(piece: ChessPieceType, mobility: usize) -> SingleFeature {
        let idx = (piece as usize - 1) * (MAX_MOBILITY + 1) + mobility;
        SingleFeature::new(Mobility, idx)
    }

    fn threats(attacking: ChessPieceType, targeted: ChessPieceType) -> SingleFeature {
        let idx = (attacking as usize - 1) * NUM_CHESS_PIECES + targeted as usize;
        SingleFeature::new(Threat, idx)
    }

    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> SingleFeature {
        let idx = (protecting as usize - 1) * NUM_CHESS_PIECES + target as usize;
        SingleFeature::new(Defense, idx)
    }

    fn king_zone_attack(attacking: ChessPieceType) -> SingleFeature {
        SingleFeature::new(KingZoneAttack, attacking as usize)
    }

    fn can_give_check(piece: ChessPieceType) -> SingleFeature {
        SingleFeature::new(CanGiveCheck, piece as usize)
    }

    fn pin(piece: ChessPieceType) -> SingleFeature {
        SingleFeature::new(Pin, piece as usize)
    }

    fn discovered_check(piece: ChessPieceType) -> SingleFeature {
        SingleFeature::new(DiscoveredCheck, piece as usize)
    }

    fn discovered_check_stm() -> SingleFeature {
        SingleFeature::new(DiscoveredCheckStm, 0)
    }

    fn check_stm() -> SingleFeature {
        SingleFeature::new(CheckStm, 0)
    }
}

#[derive(Debug, Default)]
/// Tuning the chess Linear Tuned Eval (`LiTE`) values.
/// This is done by re-using the generic eval function but instantiating it with a trace instead of a score.
pub struct TuneLiTEval {}

impl WeightsInterpretation for TuneLiTEval {
    #[allow(clippy::too_many_lines)]
    fn display(&self) -> fn(&mut Formatter, &Weights, &[Weight]) -> std::fmt::Result {
        |f: &mut Formatter<'_>, weights: &Weights, old_weights: &[Weight]| {
            let special = changed_at_least(-1.0, weights, old_weights);
            assert_eq!(weights.len(), Self::num_weights());
            for subset in LiteFeatureSubset::iter() {
                subset.write(f, weights, &special)?
            }
            Ok(())
        }
    }

    fn feature_name(weight_idx: usize) -> String {
        let feature_idx = weight_idx / 2;
        let feature = LiteFeatureSubset::iter().rfind(|f| f.start_idx() <= feature_idx).unwrap();
        format!("{feature} index {}", feature_idx - feature.start_idx())
    }

    fn eval_scale(&self) -> EvalScale {
        Scale(120.0)
    }

    fn retune_from_zero(&self) -> bool {
        false
    }

    fn initial_weights(&self) -> Option<Weights> {
        let mut weights = vec![Weight(0.0); Self::num_weights()];
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
    fn num_features() -> usize {
        LiteFeatureSubset::iter().map(|f| f.num_features()).sum()
    }

    type Filter = SkipChecks;

    fn feature_trace(pos: &Chessboard) -> impl TraceTrait {
        GenericLiTEval::<LiTETrace>::default().do_eval(pos)
    }
}
