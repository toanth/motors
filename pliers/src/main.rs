use crate::eval::caps_hce_eval::CapsHceEval;
use crate::eval::Eval;
use crate::gd::{do_optimize, EvalScale};
use crate::load_data::{parse_res_to_dataset, FenReader};
use gears::games::chess::Chessboard;
use gears::games::Board;
use gears::general::common::Res;
use std::fs::File;
use std::path::Path;

pub mod eval;
pub mod gd;
pub mod load_data;

pub fn optimize<B: Board, E: Eval<B>>(file_name: &str) -> Res<()> {
    let file = File::open(Path::new(file_name))
        .map_err(|err| format!("Could not open file '{file_name}': {err}"))?;
    let features = FenReader::<B>::load_from_file(file)?;
    let dataset = parse_res_to_dataset::<B, E>(&features);
    drop(features);
    let weights = do_optimize(&dataset, EvalScale(10000.0), 100);
    println!("{}", E::formatter(weights));
    Ok(())
}

fn main() {
    if let Err(err) =
        optimize::<Chessboard, CapsHceEval>("pliers/datasets/chess/lichess-big3-resolved.book")
    {
        eprintln!("{err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gd::{loss, Outcome};
    use gears::games::chess::zobrist::NUM_PIECE_SQUARE_ENTRIES;

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
        assert_eq!(positions[1].position.iter().filter(|f| f.0 != 0).count(), 1);
        let eval_scale = EvalScale(1000.0);
        let weights = do_optimize(&positions, eval_scale, 100);
        let loss = loss(&weights, &positions, eval_scale);
        assert!(loss <= 0.01);
    }
}
