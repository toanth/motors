#![deny(missing_docs)]
#![deny(rustdoc::missing_crate_level_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]
#![deny(rustdoc::invalid_codeblock_attributes)]
#![deny(rustdoc::invalid_rust_codeblocks)]
//! [`pliers`](crate) is a handcrafted eval tuning crate built on top of the [`gears`] crate.
//!
//! It is designed to be extensible to new games, but provides very strong support for chess out of the box,
//! including a number of different example evaluation functions.
//! To use it, you need to write your own eval function by implementing the [`Eval`] trait.
//! Then, optimizing your eval weights with this library is as simple as calling the [`run`] function.
//!
//! # Example:
//!
//! ```no_run
//! # use gears::games::chess::Chessboard;
//! # use motors::eval::*;
//! # use pliers::*;
//! # use pliers::eval::chess::piston_eval::PistonEval;
//! type Eval = PistonEval;
//! fn main() {
//!     // Make sure the eval works as expected by running it on a single simple position.
//!     debug_eval_on_lucena::<Eval>();
//!     // Run the actual optimizer. This will periodically print out the current values as well as some
//!     // statistics. Runs for `DEFAULT_NUM_EPOCHS` or until the weights don't change much anymore.
//!     // Then, this will print out the final tuned weights and some additional information, like the number of times
//!     // each feature appeared in the training dataset. Note that post-processing steps like interpolating with initial
//!     // values are only performed for the final printed values.
//!     run::<Chessboard, Eval>();
//! }
//! ```
//!
//! # Example 2:
//!
//! This example calls the [`optimize_for`] function directly to achieve greater control over the optimization process.
//! There are even lower-level functions like [`optimize_dataset`] for yet greater control, but most users shouldn't
//! need to bother with them.
//! ```no_run
//! # use gears::games::ataxx::AtaxxBoard;
//! # use gears::general::common::Res;
//! # use pliers::*;
//! # use pliers::gd::*;
//! # use pliers::trace::*;
//! # use pliers::eval::*;
//! # use pliers::load_data::*;
//! # use pliers::load_datasets_from_json;
//! # use std::path::Path;
//! # use std::fmt::Formatter;
//!
//! # #[derive(Debug, Default)]
//! # struct MyAtaxxEval {}
//!
//! # impl WeightsInterpretation for MyAtaxxEval {
//! #    fn display(&self) -> fn(&mut Formatter, &Weights, &[Weight]) -> std::fmt::Result {
//! #        todo!()
//! #    }
//! # }
//!
//! # impl Eval<AtaxxBoard> for MyAtaxxEval {
//! #    fn num_weights() -> usize {
//! #        todo!()
//! #    }
//! #    fn num_features() -> usize {
//! #        todo!()
//! #    }
//!
//! #    type Filter = NoFilter;
//!
//! #    fn feature_trace(pos: &AtaxxBoard) -> impl TraceTrait {
//! #        SimpleTrace::default()
//! #    }
//! # }
//!
//! fn main() -> Res<()> {
//!     // Alternatively, use `get_dataset` to read the command line for the location of a
//!     // JSON file which contains the list of datasets or fallback to a game-specific location.
//!     let path = "Some/hardcoded/path/../consider/not/doing/this.json";
//!     let file_list = load_datasets_from_json(Path::new(path))?;
//!     optimize_for::<AtaxxBoard, MyAtaxxEval, SimpleGDOptimizer>(&file_list, 1234)?;
//!     Ok(())
//! }
//! ```
//!
//! [`pliers`](crate) is inspired by [this chess eval tuner](https://github.com/GediminasMasaitis/texel-tuner).
//! It is currently missing the option to include additional scores, but provides a number of additional features:
//! - Support for arbitrary board games built on top of the `gears` crate
//! - Easily extensible
//! - Faster tuning thanks to a sparse feature representation and faster automatic scaling factor selection.
//! - Better printing of tuned values, with changing values highlighted in red
//! - Prints more information in general, like the sample count, the maximum weight change, etc
//! - Some additional, albeit rarely needed, features

extern crate core;

use crate::eval::EvalScale::{InitialWeights, Scale};
use crate::eval::{Eval, count_occurrences, display};
use crate::gd::{DefaultOptimizer, Optimizer, Weight, Weights, optimize_dataset, print_optimized_weights};
use crate::load_data::Perspective::White;
use crate::load_data::{AnnotatedFenFile, FenReader};
use gears::colored::Colorize;
use gears::games::chess::Chessboard;
use gears::general::board::{Board, BoardHelpers};
use gears::general::common::Res;
use gears::general::common::anyhow::{anyhow, bail};
use serde_json::from_reader;
use std::env::args;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process::exit;

pub mod eval;
pub mod gd;
pub mod load_data;
pub mod trace;

const DEFAULT_NUM_EPOCHS: usize = 4000;

/// The 'main function' of this library.
///
/// You can call one of the functions below if you want more control,
/// but this is the easiest way to use the tuner. Simply call this function with your eval,
/// e.g. `run::<Chessboard, MaterialOnlyEval>()`. Make sure to provide a JSON file with a list of datasets.
/// The filenames in that JSON file should be either absolute or relative to the location of the JSON file.
pub fn run<B: Board, E: Eval<B>>() {
    if let Err(err) = try_to_run::<B, E>() {
        eprintln!("{err}");
        exit(1)
    }
}

/// like [`run`], but returns a `Res` instead of exiting on errors.
pub fn try_to_run<B: Board, E: Eval<B>>() -> Res<()> {
    let files = get_datasets::<B>()?;
    optimize::<B, E>(files.as_ref())
}

/// Load a list datasets from a JSON file.
///
/// The path to this file is extracted from the first command line argument, with a game-specific fallback
/// if no command line arguments are used.
pub fn get_datasets<B: Board>() -> Res<Vec<AnnotatedFenFile>> {
    let default_path = format!("pliers/datasets/{}/datasets.json", B::game_name());
    let json_file_path = args().nth(1).unwrap_or(default_path);
    let json_file_path = Path::new(&json_file_path);
    load_datasets_from_json(json_file_path)
}

/// Load a list of datasets from a JSON file.
///
/// Each dataset needs to have a `"path"` relative to the location of the JSON file.
/// Additionally, it can have a [`"perspective"`][load_data::Perspective] field that tells the tuner how to interpret the results.
/// The default value of this field is [`White`], but it is possible to specify
/// [`SideToMove`][load_data::Perspective::SideToMove] instead. The [`weight`][load_data::AnnotatedFenFile::weight] field
/// can be used to reduce the effect of lower-quality datasets. It is typically not needed.
pub fn load_datasets_from_json(json_file_path: &Path) -> Res<Vec<AnnotatedFenFile>> {
    let json_file = File::open(json_file_path).map_err(|err| anyhow!(
        "Could not open the dataset json file: {err}. Check that the path is correct, maybe try using an absolute path. \
        The current path is '{}'.", json_file_path.display()
    ))?;
    let mut files: Vec<AnnotatedFenFile> =
        from_reader(BufReader::new(json_file)).map_err(|err| anyhow!("Couldn't read the JSON file: {err}"))?;

    if files.is_empty() {
        bail!("The json file appears to be empty. Please add at least one dataset".to_string(),)
    }
    // Ideally, the `AnnotatedFenFile` would store a `PathBuf`, but that makes serialization more difficult.
    for file in &mut files {
        file.path = json_file_path.parent().unwrap().join(Path::new(&file.path)).to_str().unwrap().to_string();
    }
    Ok(files)
}

/// Optimize the eval with the [`DefaultOptimizer`] on the supplied `file_list`.
pub fn optimize<B: Board, E: Eval<B>>(file_list: &[AnnotatedFenFile]) -> Res<()> {
    optimize_for::<B, E, DefaultOptimizer>(file_list, DEFAULT_NUM_EPOCHS)
}

/// Optimize the eval with the given optimizer for the given number of epochs.
///
/// Runs the optimizer on the entire dataset.
pub fn optimize_for<B: Board, E: Eval<B>, O: Optimizer>(file_list: &[AnnotatedFenFile], num_epochs: usize) -> Res<()> {
    #[cfg(debug_assertions)]
    println!("Running in debug mode. Run in release mode for increased performance.");
    let mut dataset = FenReader::<B, E>::load_from_file_list(file_list, Some(20.0))?;
    let e = E::default();
    let batch = dataset.as_batch();
    let scale = e.eval_scale().to_scaling_factor(batch, &e);
    let mut optimizer = O::new(batch, scale);

    let average = batch.num_entries() as f64 / batch.num_datapoins() as f64;
    println!("\nAverage number of Entries per Position: {}\n", format!("{average:.3}").bold());

    let occurrences = Weights(count_occurrences(batch).into_iter().map(Weight).collect());
    println!("Occurrences:\n{}", display(&e, &occurrences, &[]));
    let weights = optimize_dataset(&mut dataset, scale, num_epochs, &e, &mut optimizer);
    print_optimized_weights(&weights, dataset.as_batch(), scale, &e);
    Ok(())
}

/// Convenience wrapper for [`optimize`] for chess.
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
    let mut dataset = FenReader::<Chessboard, E>::load_from_str(&fen, White).unwrap();
    assert_eq!(dataset.num_weights(), E::num_weights());
    let scale = match e.eval_scale() {
        Scale(scale) => scale,
        InitialWeights(_) => 100.0, // Tuning the scaling factor one a single position is just going to result in inf or 0.
    };
    let mut optimizer = DefaultOptimizer::new(dataset.as_batch(), scale);
    let weights = optimize_dataset(&mut dataset, scale, 1, &e, &mut optimizer);
    assert_eq!(weights.len(), E::num_weights());
    println!(
        "There are {0} out of {1} active weights",
        dataset.as_batch().datapoint_iter().next().unwrap().entries.len(),
        E::num_weights()
    );
    print_optimized_weights(&weights, dataset.as_batch(), scale, &e);
    println!("\nEND DEBUG POSITION OUTPUT\n");
}

/// Debug a chess eval on the lucena position.
pub fn debug_eval_on_lucena<E: Eval<Chessboard>>() {
    let pos = Chessboard::from_name("lucena").unwrap();
    debug_eval_on_pos::<Chessboard, E>(pos);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::chess::material_only_eval::MaterialOnlyEval;

    use crate::eval::chess::piston_eval::PistonEval;
    use crate::gd::{
        Adam, AdamW, CpScore, CrossEntropyLoss, Float, Outcome, QuadraticLoss, cp_eval_for_weights, cp_to_wr, loss_for,
        quadratic_sample_loss,
    };
    use crate::load_data::Perspective::SideToMove;
    use ChessPieceType::*;
    use gears::games::chess::ChessColor::White;
    use gears::games::chess::ChessSettings;
    use gears::games::chess::pieces::{ChessPieceType, ColoredChessPieceType};
    use gears::games::chess::zobrist::NUM_PIECE_SQUARE_ENTRIES;
    use gears::games::{AbstractPieceType, CharType, ColoredPieceType};

    #[test]
    pub fn two_chess_positions_test() {
        let positions = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 [0.5]
        7k/8/8/8/8/8/8/R6K w - - 0 1 [1-0]";
        let mut positions = FenReader::<Chessboard, PistonEval>::load_from_str(positions, SideToMove).unwrap();
        assert_eq!(positions.data().len(), 2);
        assert_eq!(positions.data()[0].outcome, Outcome::new(0.5));
        assert_eq!(positions.data()[1].outcome, Outcome::new(1.0));
        assert_eq!(positions.num_weights(), NUM_PIECE_SQUARE_ENTRIES * 2);
        let batch = positions.as_batch();
        // the kings are on mirrored positions and cancel each other out
        assert_eq!(batch.entries_at(0).len(), 0);
        assert_eq!(batch.entries_at(1).len(), 2); // 1 feature, 2 weights
        let batch = positions.batch(0, 1);
        let eval_scale = 100.0;
        let mut optimizer = AdamW::<CrossEntropyLoss>::new(batch, eval_scale);
        let startpos_weights =
            optimize_dataset(&mut positions, eval_scale, 100, &PistonEval::default(), &mut optimizer);
        let batch = positions.batch(0, 1);
        let startpos_eval = cp_eval_for_weights(&startpos_weights, batch.datapoint_iter().next().unwrap());
        assert_eq!(startpos_eval, CpScore(0.0));
        let mut optimizer = Adam::<QuadraticLoss>::new(positions.as_batch(), eval_scale);
        let weights = optimize_dataset(&mut positions, eval_scale, 500, &PistonEval::default(), &mut optimizer);
        let loss = loss_for(&weights, positions.as_batch(), eval_scale, quadratic_sample_loss);
        assert!(loss <= 0.01, "{loss}");
    }

    #[test]
    pub fn chess_piece_values_test() {
        let piece_val = |piece| match piece {
            Pawn => 1,
            Knight | Bishop => 3,
            Rook => 5,
            Queen => 9,
            _ => panic!("not a non-king piece"),
        };
        let eval_scale = 10.0;
        let mut fens = String::default();
        for piece in ChessPieceType::non_king_pieces() {
            let str = format!(
                "8/7{0}/8/8/8/k7/8/K7 w - - 0 1 | {1}\n",
                ColoredChessPieceType::new(White, piece).to_char(CharType::Ascii, &ChessSettings::default()),
                cp_to_wr(CpScore(piece_val(piece) as Float), eval_scale),
            );
            fens += &str;
        }
        let datapoints = FenReader::<Chessboard, MaterialOnlyEval>::load_from_str(&fens, SideToMove).unwrap();
        let batch = datapoints.as_batch();
        let weights = AdamW::<CrossEntropyLoss>::new(batch, eval_scale).optimize_simple(batch, eval_scale, 2000);
        assert_eq!(weights.len(), 5);
        let weight = weights[0];
        for piece in ChessPieceType::non_king_pieces() {
            let ratio = weights[piece as usize].0 / weight.0;
            assert!((ratio - piece_val(piece) as Float).abs() <= 0.1, "{ratio} {piece}");
        }
    }
}
