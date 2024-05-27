use crate::eval::chess::caps_hce_eval::CapsHceEval;
use crate::eval::chess::material_only_eval::MaterialOnlyEval;
use crate::eval::chess::piston_eval::PistonEval;
use crate::eval::EvalScale::{InitialWeights, Scale};
use crate::eval::{Eval, EvalScale};
use crate::gd::{
    optimize_entire_batch, Adam, Batch, Datapoint, Dataset, Optimizer, ScalingFactor,
    TaperedDatapoint, Weights,
};
use crate::load_data::Perspective::White;
use crate::load_data::{AnnotatedFenFile, FenReader, Filter};
use gears::games::chess::Chessboard;
use gears::games::Board;
use gears::general::common::Res;
use serde_json::from_reader;
use std::env::args;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process::exit;

pub mod eval;
pub mod gd;
pub mod load_data;

/// The 'main function' of this library. You can call the functions below if you want more control,
/// but this is the easiest way to use the tuner. Simply call this function with your eval,
/// e.g. `run::<Chessboard, MaterialOnlyEval>()`. Make sure to provide a JSON file with a list of datasets.
/// The filenames in that JSON file should be either absolute or relative to the location of the JSON file.
pub fn run<B: Board, E: Eval<B>>() {
    if let Err(err) = try_to_run::<B, E>() {
        eprintln!("{err}");
        exit(1)
    }
}

pub fn try_to_run<B: Board, E: Eval<B>>() -> Res<()> {
    let files = get_datasets::<B>()?;
    optimize::<B, E>(files.as_ref())
}

pub fn get_datasets<B: Board>() -> Res<Vec<AnnotatedFenFile>> {
    let default_path = format!("pliers/datasets/{}/datasets.json", B::game_name());
    let json_file_path = args().nth(1).unwrap_or(default_path);
    let json_file_path = Path::new(&json_file_path);
    let json_file = File::open(json_file_path).map_err(|err| format!(
        "Could not open the dataset json file: {err}. Check that the path is correct, maybe try using an absolute path. \
        The current path is '{}'.", json_file_path.display()
    ))?;
    let mut files: Vec<AnnotatedFenFile> = from_reader(BufReader::new(json_file))
        .map_err(|err| format!("Couldn't read the JSON file: {err}"))?;

    if files.is_empty() {
        return Err(
            "The json file appears to be empty. Please add at least one dataset".to_string(),
        );
    }
    // Ideally, the `AnnotatedFenFile` would store a `PathBuf`, but that makes serialization more difficult.
    for file in files.iter_mut() {
        file.name = json_file_path
            .parent()
            .unwrap()
            .join(Path::new(&file.name))
            .to_str()
            .unwrap()
            .to_string();
    }
    Ok(files)
}

pub fn optimize<B: Board, E: Eval<B>>(file_list: &[AnnotatedFenFile]) -> Res<()> {
    optimize_for::<B, E, Adam>(file_list, 2000)
}

pub fn optimize_for<B: Board, E: Eval<B>, O: Optimizer<E::D>>(
    file_list: &[AnnotatedFenFile],
    num_epochs: usize,
) -> Res<()> {
    #[cfg(debug_assertions)]
    println!("Running in debug mode. Run in release mode for increased performance.");
    let mut dataset = Dataset::new(E::NUM_WEIGHTS);
    for file in file_list {
        dataset.union(FenReader::<B, E>::load_from_file(file)?);
    }
    let e = E::default();
    let batch = dataset.as_batch();
    let scale = e.eval_scale().to_scaling_factor(batch, &e);
    let mut optimizer = O::new(batch, scale);
    let weights = optimize_entire_batch(batch, scale, num_epochs, &e, &mut optimizer);
    println!(
        "Scaling factor: {scale:.2}, eval:\n{}",
        e.display(&weights, &[])
    );
    Ok(())
}

pub fn optimize_chess_eval<E: Eval<Chessboard>>(file_list: &[AnnotatedFenFile]) -> Res<()> {
    debug_eval_on_lucena::<E>();
    optimize::<Chessboard, E>(file_list)
}

/// Function intended for debugging the eval, uses a single simple position.
pub fn debug_eval_on_pos<B: Board, E: Eval<Chessboard>>(pos: B) {
    println!("\nSTARTING DEBUG POSITION OUTPUT:");
    let fen = format!("{} [1.0]", pos.as_fen());
    println!("(FEN: {fen}\n");
    let e = E::default();
    let dataset = FenReader::<Chessboard, E>::load_from_str(&fen, White).unwrap();
    let scale = match e.eval_scale() {
        Scale(scale) => scale,
        InitialWeights(_) => 100.0, // Tuning the scaling factor one a single position is just going to result in inf or 0.
    };
    let mut optimizer = Adam::new(dataset.as_batch(), scale);
    let weights = optimize_entire_batch(dataset.as_batch(), scale, 1, &e, &mut optimizer);
    assert_eq!(weights.len(), E::NUM_WEIGHTS);
    println!(
        "There are {0} weights and {1} out of {2} active features",
        weights.len(),
        dataset.data()[0].features().count(),
        E::NUM_FEATURES
    );
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
    use crate::eval::WeightsInterpretation;
    use crate::gd::{cp_eval_for_weights, cp_to_wr, loss, Adam, CpScore, Float, Outcome};
    use crate::load_data::Perspective::SideToMove;
    use gears::games::chess::pieces::{ColoredChessPiece, UncoloredChessPiece};
    use gears::games::chess::zobrist::NUM_PIECE_SQUARE_ENTRIES;
    use gears::games::Color::White;
    use gears::games::{AbstractPieceType, ColoredPieceType};

    #[test]
    pub fn two_chess_positions_test() {
        let positions = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 [0.5]
        7k/8/8/8/8/8/8/R6K w - - 0 1 [1-0]";
        let positions =
            FenReader::<Chessboard, PistonEval>::load_from_str(positions, SideToMove).unwrap();
        assert_eq!(positions.datapoints.len(), 2);
        assert_eq!(positions.datapoints[0].outcome, Outcome::new(0.5));
        assert_eq!(positions.datapoints[1].outcome, Outcome::new(1.0));
        assert_eq!(positions.weights_in_pos, NUM_PIECE_SQUARE_ENTRIES * 2);
        // the kings are on mirrored positions and cancel each other out
        assert_eq!(positions.datapoints[0].features.len(), 0);
        assert_eq!(positions.datapoints[1].features.len(), 1);
        let batch = positions.batch(0, 1);
        let eval_scale = 100.0;
        let mut optimizer = Adam::new(batch, eval_scale);
        let startpos_weights = optimize_entire_batch(
            batch,
            eval_scale,
            100,
            &PistonEval::default(),
            &mut optimizer,
        );
        let startpos_eval = cp_eval_for_weights(&startpos_weights, &positions.datapoints[0]);
        assert_eq!(startpos_eval, CpScore(0.0));
        let batch = positions.as_batch();
        let mut optimizer = Adam::new(batch, eval_scale);
        let weights = optimize_entire_batch(
            batch,
            eval_scale,
            100,
            &PistonEval::default(),
            &mut optimizer,
        );
        let loss = loss(&weights, batch, eval_scale);
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
        let eval_scale = 10.0;
        let mut fens = String::default();
        for piece in UncoloredChessPiece::non_king_pieces() {
            let str = format!(
                "8/7{0}/8/8/8/k7/8/K7 w - - 0 1 | {1}\n",
                ColoredChessPiece::new(White, piece).to_ascii_char(),
                cp_to_wr(CpScore(piece_val(piece) as Float), eval_scale),
            );
            fens += &str;
        }
        let datapoints =
            FenReader::<Chessboard, MaterialOnlyEval>::load_from_str(&fens, SideToMove).unwrap();
        let batch = datapoints.as_batch();
        let weights = Adam::new(batch, eval_scale).optimize_simple(batch, eval_scale, 2000);
        assert_eq!(weights.len(), 5);
        let weight = weights[0];
        for piece in UncoloredChessPiece::non_king_pieces() {
            let ratio = weights[piece as usize].0 / weights[0].0;
            assert!((ratio - piece_val(piece) as Float).abs() <= 0.1);
        }
    }
}
