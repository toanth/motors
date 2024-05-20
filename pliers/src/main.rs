use crate::eval::chess::caps_hce_eval::CapsHceEval;
use crate::eval::chess::material_only_eval::MaterialOnlyEval;
use crate::eval::chess::piston_eval::PistonEval;
use crate::eval::Eval;
use crate::gd::{do_optimize, Adam, EvalScale, Optimizer, SimpleGDOptimizer};
use crate::load_data::{parse_res_to_dataset, FenReader};
use gears::games::chess::Chessboard;
use gears::games::Board;
use gears::general::common::Res;
use std::fs::File;
use std::path::Path;

pub mod eval;
pub mod gd;
pub mod load_data;

pub fn optimize<B: Board, E: Eval<B>, O: Optimizer>(file_name: &str) -> Res<()> {
    let file = File::open(Path::new(file_name))
        .map_err(|err| format!("Could not open file '{file_name}': {err}"))?;
    let features = FenReader::<B>::load_from_file(file)?;
    let dataset = parse_res_to_dataset::<B, E>(&features);
    drop(features);
    let scale = EvalScale::default();
    let mut optimizer = O::new(&dataset, scale);
    let weights = do_optimize(&dataset, scale, 1000, E::formatter(), &mut optimizer);
    println!("{}", E::formatter().with_weights(weights));
    Ok(())
}

fn main() {
    if let Err(err) =
        optimize::<Chessboard, PistonEval, Adam>("pliers/datasets/chess/quiet-labeled.v7.epd")
    // optimize::<Chessboard, PistonEval, Adam>("pliers/datasets/chess/lichess-big3-resolved.book")
    {
        eprintln!("{err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::chess::material_only_eval::MaterialOnlyEval;
    use crate::gd::{
        adam_optimize, cp_eval_for_weights, cp_to_wr, loss, Adam, CpScore, FeatureT, Float, Outcome,
    };
    use crate::load_data::parse_from_str;
    use gears::games::chess::pieces::{ColoredChessPiece, UncoloredChessPiece};
    use gears::games::chess::zobrist::NUM_PIECE_SQUARE_ENTRIES;
    use gears::games::Color::White;
    use gears::games::{AbstractPieceType, ColoredPieceType};

    #[test]
    pub fn two_chess_positions_test() {
        let positions = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 [0.5]
        7k/8/8/8/8/8/8/R6K w - - 0 1 [1-0]";
        let positions = FenReader::<Chessboard>::load_from_str(positions).unwrap();
        assert_eq!(positions.len(), 2);
        let positions = parse_res_to_dataset::<Chessboard, CapsHceEval>(&positions);
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0].outcome, Outcome::new(0.5));
        assert_eq!(positions[1].outcome, Outcome::new(1.0));
        // TODO: This breaks as soon as there are new eval features, so write a psqt-only tuner and use that.
        assert_eq!(positions[1].position.len(), NUM_PIECE_SQUARE_ENTRIES);
        // the kings are on mirrored positions and cancel each other out
        assert_eq!(
            positions[1]
                .position
                .iter()
                .filter(|f| f.0 != 0 as FeatureT)
                .count(),
            1
        );
        let eval_scale = EvalScale::default();
        let startpos_weights =
            adam_optimize(&positions[0..1], eval_scale, 100, CapsHceEval::formatter());
        let startpos_eval = cp_eval_for_weights(&startpos_weights, &positions[0].position);
        assert_eq!(startpos_eval, CpScore(0.0));
        let weights = adam_optimize(&positions, eval_scale, 100, CapsHceEval::formatter());
        let loss = loss(&weights, &positions, eval_scale);
        assert!(loss <= 0.01);
    }

    #[test]
    pub fn chess_piece_values_test() {
        let piece_val = |piece| match piece {
            UncoloredChessPiece::Pawn => 1,
            UncoloredChessPiece::Knight => 3,
            UncoloredChessPiece::Bishop => 3,
            UncoloredChessPiece::Rook => 5,
            UncoloredChessPiece::Queen => 9,
            _ => panic!("not a non-king piece"),
        };
        let eval_scale = EvalScale(10.0);
        let mut fens = String::default();
        for piece in UncoloredChessPiece::non_king_pieces() {
            let str = format!(
                "8/7{0}/8/8/8/k7/8/K7 w - - 0 1 | {1}\n",
                ColoredChessPiece::new(White, piece).to_ascii_char(),
                cp_to_wr(CpScore(piece_val(piece) as Float), eval_scale),
            );
            fens += &str;
        }
        let datapoints = parse_from_str::<Chessboard, MaterialOnlyEval>(&fens).unwrap();
        let weights = Adam::new(&datapoints, eval_scale).optimize(&datapoints, eval_scale, 2000);
        assert_eq!(weights.len(), 5);
        let weight = weights[0];
        for piece in UncoloredChessPiece::non_king_pieces() {
            let ratio = weights[piece as usize].0 / weights[0].0;
            assert!((ratio - piece_val(piece) as Float).abs() <= 0.1);
        }
    }
}
