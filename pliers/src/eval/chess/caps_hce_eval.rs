use crate::eval::chess::caps_hce_eval::FileOpenness::*;
use crate::eval::chess::{
    psqt_trace, write_phased_psqt, write_psqts, PhaseType, SkipChecks, NUM_PHASES,
    NUM_PSQT_FEATURES,
};
use crate::eval::{Eval, WeightFormatter};
use crate::gd::{
    Datapoint, EvalScale, Feature, Float, Outcome, PhaseMultiplier, SimpleTrace, TaperedDatapoint,
    TraceTrait, Weights,
};
use crate::load_data::NoFilter;
use gears::games::chess::pieces::UncoloredChessPiece::{King, Pawn, Rook};
use gears::games::chess::pieces::{UncoloredChessPiece, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::chess::zobrist::NUM_PIECE_SQUARE_ENTRIES;
use gears::games::chess::Chessboard;
use gears::games::Color::White;
use gears::games::{Board, Color, DimT};
use gears::general::bitboards::chess::{ChessBitboard, A_FILE};
use gears::general::bitboards::{Bitboard, RawBitboard};
use std::fmt::Formatter;
use strum::IntoEnumIterator;

#[derive(Debug, Default)]
struct Trace {
    psqt: SimpleTrace,
    passed_pawns: SimpleTrace,
    kings: SimpleTrace,
    rooks: SimpleTrace,
}

impl TraceTrait for Trace {
    fn as_features(&self, mut idx_offset: usize) -> Vec<Feature> {
        let mut res = self.psqt.as_features(idx_offset);
        idx_offset += NUM_PSQT_FEATURES;
        res.append(&mut self.passed_pawns.as_features(idx_offset));
        idx_offset += NUM_PASSED_PAWN_FEATURES;
        res.append(&mut self.rooks.as_features(idx_offset));
        idx_offset += NUM_ROOK_OPENNESS_FEATURES;
        res.append(&mut self.kings.as_features(idx_offset));
        res
    }

    fn phase(&self) -> Float {
        self.psqt.phase
    }
}

#[derive(Debug, Default)]
pub struct CapsHceEval {}

impl WeightFormatter for CapsHceEval {
    fn format_impl(&self) -> (fn(&mut Formatter, &Weights) -> std::fmt::Result) {
        |f: &mut Formatter<'_>, weights: &Weights| {
            assert_eq!(weights.len(), Self::NUM_WEIGHTS);
            write_psqts(f, weights)?;
            writeln!(f, "\n#[rustfmt::skip]")?;
            writeln!(f, "const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [")?;
            write_phased_psqt(
                f,
                &weights[NUM_PHASES * NUM_PSQT_FEATURES..],
                0,
                "passed pawns",
            )?;
            writeln!(f, "];")?;
            let mut idx = (NUM_PSQT_FEATURES + NUM_PASSED_PAWN_FEATURES) * NUM_PHASES;
            for piece in ["ROOK", "KING"] {
                for openness in ["OPEN", "SEMIOPEN", "CLOSED"] {
                    for phase in PhaseType::iter() {
                        writeln!(
                            f,
                            "const {piece}_{openness}_FILE_{}: i32 = {value};",
                            phase.to_string().to_ascii_uppercase(),
                            value = weights[idx].rounded()
                        )?;
                        idx += 1;
                    }
                }
            }
            assert_eq!(idx, Self::NUM_WEIGHTS);
            Ok(())
        }
    }
}

const NUM_ROOK_OPENNESS_FEATURES: usize = 3;
const NUM_KING_OPENNESS_FEATURES: usize = 3;
const NUM_PASSED_PAWN_FEATURES: usize = NUM_SQUARES;

impl Eval<Chessboard> for CapsHceEval {
    const NUM_WEIGHTS: usize = Self::NUM_FEATURES * NUM_PHASES;

    const NUM_FEATURES: usize = NUM_PIECE_SQUARE_ENTRIES
        + NUM_PASSED_PAWN_FEATURES
        + NUM_ROOK_OPENNESS_FEATURES
        + NUM_KING_OPENNESS_FEATURES;

    type D = TaperedDatapoint;
    type Filter = SkipChecks;

    fn feature_trace(pos: &Chessboard) -> Trace {
        Self::trace(pos)
    }

    fn eval_scale() -> EvalScale {
        EvalScale(130.0)
    }
}

#[derive(Debug, Eq, PartialEq)]
enum FileOpenness {
    Open,
    Closed,
    SemiOpen,
    SemiClosed,
}

impl CapsHceEval {
    fn file_openness(
        file: DimT,
        our_pawns: ChessBitboard,
        their_pawns: ChessBitboard,
    ) -> FileOpenness {
        let file = ChessBitboard::file_no(file);
        if (file & our_pawns).is_zero() && (file & their_pawns).is_zero() {
            Open
        } else if (file & our_pawns).is_zero() {
            SemiOpen
        } else if (file & our_pawns).has_set_bit() && (file & their_pawns).has_set_bit() {
            Closed
        } else {
            SemiClosed
        }
    }

    fn trace(pos: &Chessboard) -> Trace {
        let mut trace = Trace::default();
        trace.psqt = psqt_trace(pos);
        trace.rooks = SimpleTrace::for_features(NUM_ROOK_OPENNESS_FEATURES);
        trace.kings = SimpleTrace::for_features(NUM_KING_OPENNESS_FEATURES);
        trace.passed_pawns = SimpleTrace::for_features(NUM_PASSED_PAWN_FEATURES);
        for color in Color::iter() {
            let our_pawns = pos.colored_piece_bb(color, Pawn);
            let their_pawns = pos.colored_piece_bb(color.other(), Pawn);

            let mut pawns = our_pawns; // TODO: impl IntoIter for Bitboard

            while pawns.has_set_bit() {
                let idx = pawns.pop_lsb();
                // Passed pawns.
                let in_front = if color == White {
                    A_FILE << (idx + 8)
                } else {
                    A_FILE >> (64 - idx)
                };
                let blocking = in_front | in_front.west() | in_front.east();
                if (in_front & our_pawns).is_zero() && (blocking & their_pawns).is_zero() {
                    let square = ChessSquare::new(idx).flip_if(color == White).index();
                    trace.passed_pawns.increment(square, color);
                }
            }
            // Rooks on (semi)open/closed files (semi-closed files are handled by adjusting the base rook values during tuning)
            let mut rooks = pos.colored_piece_bb(color, Rook);
            while rooks.has_set_bit() {
                let idx = rooks.pop_lsb();
                let openness =
                    Self::file_openness(ChessSquare::new(idx).file(), our_pawns, their_pawns);
                if openness != SemiClosed {
                    // part of the normal piece value (i.e. part of the rook PSQT)
                    trace.rooks.increment(openness as usize, color);
                }
            }
            // King on (semi)open/closed file
            let king_file = pos.king_square(color).file();
            let openness = Self::file_openness(king_file, our_pawns, their_pawns);
            if openness != SemiClosed {
                trace.kings.increment(openness as usize, color);
            }
        }
        trace
    }
}
