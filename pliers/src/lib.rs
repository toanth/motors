use crate::eval::chess::caps_hce_eval::CapsHceEval;
use crate::eval::chess::material_only_eval::MaterialOnlyEval;
use crate::eval::chess::piston_eval::PistonEval;
use crate::eval::Eval;
use crate::gd::{
    optimize_entire_batch, Adam, Batch, Datapoint, Dataset, EvalScale, Optimizer, TaperedDatapoint,
    Weights,
};
use crate::load_data::{FenReader, Filter};
use gears::games::chess::Chessboard;
use gears::games::Board;
use gears::general::common::Res;
use std::fs::File;
use std::path::Path;

pub mod eval;
pub mod gd;
pub mod load_data;

pub fn optimize<B: Board, E: Eval<B>, O: Optimizer<E::D>>(file_list: &[String]) -> Res<()> {
    #[cfg(debug_assertions)]
    println!("Running in debug mode. Run in release mode for increased performance.");
    let mut dataset = Dataset::new(E::NUM_WEIGHTS);
    for file_name in file_list {
        dataset.union(FenReader::<B, E>::load_from_file(file_name)?);
    }
    let scale = EvalScale::default();
    let mut optimizer = O::new(dataset.as_batch(), scale);
    let e = E::default();
    let weights = optimize_entire_batch(dataset.as_batch(), scale, 2000, &e, &mut optimizer);
    println!("{}", e.formatter(&weights));
    Ok(())
}

pub fn optimize_chess_eval<E: Eval<Chessboard>>(file_list: &[String]) -> Res<()> {
    debug_eval_on_lucena::<E>();
    optimize::<Chessboard, E, Adam>(file_list)
}

/// Function intended for debugging the eval, uses a single simple position.
pub fn debug_eval_on_pos<B: Board, E: Eval<Chessboard>>(pos: B) {
    println!("STARTING DEBUG POSITION OUTPUT:");
    let fen = format!("{} [1.0]", pos.as_fen());
    let dataset = FenReader::<Chessboard, E>::load_from_str(&fen).unwrap();
    let scale = EvalScale::default();
    let mut optimizer = Adam::new(dataset.as_batch(), scale);
    let e = E::default();
    let _ = optimize_entire_batch(dataset.as_batch(), scale, 1, &e, &mut optimizer);
    println!("\nEND DEBUG POSITION OUTPUT\n");
}

pub fn debug_eval_on_lucena<E: Eval<Chessboard>>() {
    let pos = Chessboard::from_name("lucena").unwrap();
    debug_eval_on_pos::<Chessboard, E>(pos);
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
