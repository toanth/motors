//! Everything related to loading and converting lists of annotated FENs into a [`Dataset`].

use crate::eval::Eval;
use crate::eval::chess::lite::{LITE_EVAL_SCALE, TuneLiTEval};
use crate::gd::{CpScore, Dataset, Float, Outcome, ScalingFactor, WrScore, cp_to_wr};
use crate::get_json_path;
use crate::load_data::Perspective::{FirstPlayer, SideToMove};
use derive_more::Display;
use gears::GameResult;
use gears::colored::Colorize;
use gears::games::Color;
use gears::games::chess::Chessboard;
use gears::general::board::Board;
use gears::general::board::Strictness::Relaxed;
use gears::general::common::anyhow::{anyhow, bail};
use gears::general::common::{Res, Tokens, anyhow, parse_fp_from_str, tokens};
use gears::search::SearchLimit;
use motors::search::Engine;
use motors::search::chess::caps::Caps;
use motors::search::tt::TT;
use rayon::iter::ParallelIterator;
use rayon::prelude::ParallelBridge;
use serde::Deserialize;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader, BufWriter};
use std::marker::PhantomData;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// A parsed FEN with metadata.
///
/// The weight is inherited from the dataset but can also be changed by the [`Filter`], just like all members.
pub struct ParseResult<B: Board> {
    /// The loaded position.
    pub pos: B,
    /// The predicted winrate or WDL result.
    pub outcome: Outcome,
    /// Setting a weight less than 1 can be used to make samples have a smaller effect.
    /// This can be useful if there is a small, high-quality dataset, and a large but lower-quality dataset.
    /// Usually, this should not be necessary. The better course of action is always to use better datasets.
    pub weight: Float,
}

/// Describes criteria tha FENs to be used for tuning.
///
/// The most basic implementation is [`NoFilter`], which simply accepts every fen.
/// Another, chess-specific, filter is [`SkipChecks`](super::eval::chess::SkipChecks), which removes positions where the side to move is in check.
pub trait Filter<B: Board> {
    /// Returns an iterator because it's possible for a [`Filter`] to return more than one position per input position.
    /// Filtering could also include running a low-depth search with an engine to relabel the outcome.
    fn filter(pos: ParseResult<B>) -> impl IntoIterator<Item = ParseResult<B>>;
}

/// Doesn't filter any positions, the neutral element of the [`Filter`] monoid.
pub struct NoFilter {}

impl<B: Board> Filter<B> for NoFilter {
    fn filter(pos: ParseResult<B>) -> impl IntoIterator<Item = ParseResult<B>> {
        [pos]
    }
}

/// How to interpret outcome annotations in a FEN.
///
/// If this is [`FirstPlayer`](Self::FirstPlayer), a value of `1.0` is interpreted as a win for the first player, e.g. white in chess.
/// If this is [`SideToMove`](Self::SideToMove), a value of `1.0` is interpreted as a win for the current player.
#[derive(Debug, Copy, Default, Clone, PartialEq, Eq, Display, Deserialize)]
pub enum Perspective {
    /// Scores are from white's perspective.
    #[default]
    FirstPlayer,
    /// Scores are from the perspective of the current player.
    SideToMove,
}

/// A file that consists of a list of annotated FENs.
///
/// Loaded from the JSON config file.
#[derive(Debug, Deserialize)]
pub struct AnnotatedFenFile {
    /// The path to the list of FENs.
    pub path: String,
    #[serde(default)]
    /// How to interpret result annotations.
    pub perspective: Perspective,
    /// If present, a FEN must contain both a WDL result and an eval (as second result). This is the interpolation factor:
    /// `score = (1-eval_weight) * wdl + eval_weight * eval`. If `None`, the eval is ignored
    pub eval_weight: Option<Float>,
    /// Optional weight used to reduce the impact of large but low-quality datasets when there is also a smaller but
    /// higher-quality dataset. Not usually necessary.
    pub weight: Option<Float>,
}

/// A struct to avoid having to specify the generic [`Board`] and [`Eval`] arguments each time.
#[derive(Default)]
pub(super) struct FenReader<B: Board, E: Eval<B>> {
    _phantom_data: PhantomData<B>,
    _phantom_data2: PhantomData<E>,
}

impl<B: Board, E: Eval<B>> FenReader<B, E> {
    fn parse_wdl(input: &mut Tokens) -> Res<Outcome> {
        const IGNORED: &[char] = &['\"', '\'', '[', ']', '(', ')', '{', '}', ' ', '\t'];
        // This would be a great time to use the `.remainder()` method, but that isn't stable :/
        let wdl = input.next().ok_or_else(|| anyhow!("Missing wdl"))?;
        let wdl = wdl.trim_matches(IGNORED);
        if let Some(result) = GameResult::from_str(wdl).ok().and_then(|val| val.check_finished()) {
            return Ok(Outcome::new(result.into()));
        }
        if let Ok(parsed) = parse_fp_from_str(wdl, "wdl") {
            return Ok(Outcome::new(parsed));
        }
        bail!("'{}' is not a valid wdl", wdl.red())
    }

    fn read_annotated_fen(
        input: &str,
        perspective: Perspective,
        weight: Float,
        interpolation: Option<Float>,
    ) -> Res<ParseResult<B>> {
        let mut input = tokens(input);
        let pos = B::read_fen_and_advance_input(&mut input, Relaxed)?;
        // skip up to one token between the end of the fen and the wdl
        let mut outcome = Self::parse_wdl(&mut input).or_else(|err| Self::parse_wdl(&mut input).or(Err(err)))?;
        if perspective == SideToMove && pos.active_player() == B::Color::second() {
            outcome.0 = 1.0 - outcome.0;
        }
        if let Some(interpolation) = interpolation {
            let mut eval = Self::parse_wdl(&mut input)
                .or_else(|err| bail!("Failed to parse eval after successfully parsing outcome: {err}"))?;
            if perspective == SideToMove && pos.active_player() == B::Color::second() {
                eval.0 = 1.0 - eval.0;
            }
            outcome.0 = outcome.0 * (1.0 - interpolation) + interpolation * eval.0;
        }
        Ok(ParseResult { pos, outcome, weight })
    }

    fn load_datapoint_from_annotated_fen(
        input: &str,
        line_num: usize,
        perspective: Perspective,
        weight: Float,
        dataset: &mut Dataset<E::D>,
        eval_weight: Option<Float>,
    ) -> Res<()> {
        let parse_res = Self::read_annotated_fen(input, perspective, weight, eval_weight)
            .map_err(|err| anyhow!("Error in line {0}: Couldn't parse FEN '{1}': {err}", line_num + 1, input.bold()))?;
        for datapoint in E::Filter::filter(parse_res) {
            dataset.push(E::extract_features(&datapoint.pos, datapoint.outcome, datapoint.weight));
        }
        Ok(())
    }

    /// Load FENs from a [`&str`] instead of a file.
    ///
    /// This is primarily intended for debugging and small examples.
    pub fn load_from_str(
        annotated_fens: &str,
        perspective: Perspective,
        eval_weight: Option<Float>,
    ) -> Res<Dataset<E::D>> {
        let mut res = Dataset::new(E::num_weights());
        for (idx, line) in annotated_fens.lines().enumerate() {
            Self::load_datapoint_from_annotated_fen(line, idx, perspective, 1.0, &mut res, eval_weight)?;
        }
        Ok(res)
    }

    /// Load annotated FENs from a file.
    ///
    /// Regularly prints ou the number of loaded FENs.
    /// Fails if there is any invalid FEN in the dataset.
    pub fn load_from_file(input_file: &AnnotatedFenFile) -> Res<Dataset<E::D>> {
        let file = File::open(Path::new(&input_file.path))
            .map_err(|err| anyhow!("Could not open file '{}': {err}", input_file.path))?;
        let reader = BufReader::new(file);
        let perspective = input_file.perspective;
        let weight = input_file.weight.unwrap_or(1.0);
        println!(
            "Loading FENs from file '{0}' (Outcomes are {perspective} relative), sampling weight: {weight:.1}",
            input_file.path.as_str().bold()
        );
        let id = || (Dataset::new(E::num_weights()), 0);
        let (dataset, num_lines) = reader
            .lines()
            .enumerate()
            .par_bridge()
            .try_fold(id, |(mut dataset, num_lines_so_far), (line_num, line)| {
                let line = line.map_err(|err| anyhow!("Failed to read line {line_num}: {err}"))?;
                Self::load_datapoint_from_annotated_fen(
                    &line,
                    line_num,
                    perspective,
                    weight,
                    &mut dataset,
                    input_file.eval_weight,
                )?;
                if line_num % 100_000 == 0 {
                    println!("Loading...  Read {line_num} lines so far");
                }
                Ok((dataset, num_lines_so_far + 1))
            })
            .try_reduce(id, |(mut a, a_lines), (b, b_lines)| {
                a.union(b);
                let res: Res<(Dataset<E::D>, i32)> = Ok((a, a_lines + b_lines));
                res
            })?;
        println!("Read {num_lines} fens in total, after filtering there are {} positions", dataset.data().len());
        Ok(dataset)
    }

    pub fn rescore<En: Engine<B> + Default>(
        input_file: AnnotatedFenFile,
        limit: SearchLimit,
        scale: ScalingFactor,
        output_file: &str,
    ) -> Res<()> {
        let file = File::open(Path::new(&input_file.path))
            .map_err(|err| anyhow!("Could not open input file '{}': {err}", input_file.path))?;
        let reader = BufReader::new(file);
        let output_file =
            File::create(output_file).map_err(|err| anyhow!("Could not open output file '{output_file}': {err}"))?;
        let writer = BufWriter::new(output_file);
        let writer = Arc::new(Mutex::new(writer));
        let perspective = input_file.perspective;
        println!(
            "Rescoring FENs from file '{0}' (Outcomes are {perspective} relative)",
            input_file.path.as_str().bold()
        );
        let tt = TT::minimal();
        let start_time = Instant::now();
        let f = || En::default();
        reader.lines().enumerate().par_bridge().try_for_each_init(f, |engine, (line_num, line)| {
            if line_num % 10_000 == 0 {
                println!("[{0}s] Rescored {line_num} FENs", start_time.elapsed().as_secs());
            }
            let line = line.map_err(|err| anyhow!("Failed to read line {line_num}: {err}"))?;
            let parsed = Self::read_annotated_fen(&line, perspective, 1.0, None)?;
            let res = engine.search_with_tt(parsed.pos.clone(), limit, tt.clone());
            let mut score = if res.score.is_game_lost_score() {
                WrScore(0.0)
            } else if res.score.is_game_won_score() {
                WrScore(1.0)
            } else {
                cp_to_wr(CpScore(res.score.0 as Float), scale)
            };
            if perspective == FirstPlayer && !parsed.pos.active_player().is_first() {
                score.0 = 1.0 - score.0;
            }
            let mut writer = writer.lock().unwrap();
            writeln!(&mut writer, "{line} [{score:.7}]")?;
            Ok::<(), anyhow::Error>(())
        })
    }
}

/// rescore the lichess-big3 dataset
pub fn rescore_lichess_with_caps() -> Res<()> {
    let json_path = get_json_path::<Chessboard>();
    let json_path = Path::new(&json_path);
    let prefix = json_path.parent().unwrap();
    let path = prefix.join(Path::new("lichess-big3-resolved.book")).to_str().unwrap().to_string();
    let out_path = prefix.join(Path::new("lichess-rescored.book")).to_str().unwrap().to_string();
    let file = AnnotatedFenFile { path, perspective: Perspective::default(), weight: None, eval_weight: None };
    let limit = SearchLimit::soft_nodes_(5_000).and(SearchLimit::nodes_(50_000));
    FenReader::<Chessboard, TuneLiTEval>::rescore::<Caps>(file, limit, LITE_EVAL_SCALE, &out_path)
}
