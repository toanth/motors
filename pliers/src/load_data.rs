//! Everything related to loading and converting lists of annotated FENs into a [`Dataset`].

use crate::eval::{Eval, WeightsInterpretation, count_occurrences};
use crate::gd::{Batch, Dataset, Entry, EntryIdxT, Float, Outcome};
use crate::load_data::Perspective::SideToMove;
use derive_more::Display;
use gears::GameResult;
use gears::colored::Colorize;
use gears::games::Color;
use gears::general::board::Board;
use gears::general::board::Strictness::Relaxed;
use gears::general::common::anyhow::{anyhow, bail};
use gears::general::common::{Res, Tokens, parse_fp_from_str, tokens};
use gears::itertools::Itertools;
use rayon::iter::ParallelIterator;
use rayon::prelude::ParallelBridge;
use serde::Deserialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::marker::PhantomData;
use std::path::Path;
use std::str::FromStr;

/// A parsed FEN with metadata.
///
/// The weight is inherited from the dataset but can also be changed by the [`Filter`], just like all members.
pub struct ParseResult<B: Board> {
    /// The loaded position.
    pub pos: B,
    /// The predicted winrate or WDL result.
    pub outcome: Outcome,
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
/// If this is [`White`](Self::White), a value of `1.0` is interpreted as a win for white (the default).
/// If this is [`SideToMove`](Self::SideToMove), a value of `1.0` is interpreted as a win for the current player.
#[derive(Debug, Copy, Default, Clone, PartialEq, Eq, Display, Deserialize)]
pub enum Perspective {
    /// Scores are from white's perspective.
    #[default]
    White,
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

    fn read_annotated_fen(input: &str, perspective: Perspective) -> Res<ParseResult<B>> {
        let mut input = tokens(input);
        let pos = B::read_fen_and_advance_input(&mut input, Relaxed)?;
        // skip up to one token between the end of the fen and the wdl
        let mut outcome = Self::parse_wdl(&mut input).or_else(|err| Self::parse_wdl(&mut input).or(Err(err)))?;
        if perspective == SideToMove && pos.active_player() == B::Color::second() {
            outcome.0 = 1.0 - outcome.0;
        }
        Ok(ParseResult { pos, outcome })
    }

    fn load_datapoint_from_annotated_fen(
        input: &str,
        line_num: usize,
        perspective: Perspective,
        dataset: &mut Dataset,
    ) -> Res<()> {
        let parse_res = Self::read_annotated_fen(input, perspective)
            .map_err(|err| anyhow!("Error in line {0}: Couldn't parse FEN '{1}': {err}", line_num + 1, input.bold()))?;
        for datapoint in E::Filter::filter(parse_res) {
            E::extract_features(&datapoint.pos, datapoint.outcome, dataset);
        }
        Ok(())
    }

    fn load_from_file_impl(input_file: &AnnotatedFenFile) -> Res<Dataset> {
        let file = File::open(Path::new(&input_file.path))
            .map_err(|err| anyhow!("Could not open file '{}': {err}", input_file.path))?;
        let file = BufReader::new(file);
        let perspective = input_file.perspective;
        println!("Loading FENs from file '{0}' (Outcomes are {perspective} relative)", input_file.path.as_str().bold());
        let reader = BufReader::new(file);
        let id = || (Dataset::new(E::num_weights()), 0);
        let (dataset, num_lines) = reader
            .lines()
            .enumerate()
            .par_bridge()
            .try_fold(id, |(mut dataset, num_lines_so_far), (line_num, line)| {
                if line_num % 100_000 == 0 {
                    println!("Loading...  Read {line_num} lines so far for this file");
                }
                let line = line.map_err(|err| anyhow!("Failed to read line {line_num}: {err}"))?;
                Self::load_datapoint_from_annotated_fen(&line, line_num, perspective, &mut dataset)?;
                Ok((dataset, num_lines_so_far + 1))
            })
            .try_reduce(id, |(mut a, a_lines), (b, b_lines)| {
                a.union(b);
                let res: Res<(Dataset, i32)> = Ok((a, a_lines + b_lines));
                res
            })?;
        println!("Read {num_lines} fens in total, after filtering there are {} positions", dataset.data().len());
        Ok(dataset)
    }

    /// Load FENs from a [`&str`] instead of a file.
    ///
    /// This is primarily intended for debugging and small examples.
    pub fn load_from_str(annotated_fens: &str, perspective: Perspective) -> Res<Dataset> {
        let mut res = Dataset::new(E::num_weights());
        for (idx, line) in annotated_fens.lines().enumerate() {
            Self::load_datapoint_from_annotated_fen(line, idx, perspective, &mut res)?;
        }
        Ok(res)
    }

    /// Load annotated FENs from a file.
    ///
    /// Regularly prints ou the number of loaded FENs.
    /// Fails if there is any invalid FEN in the dataset.
    #[allow(unused)]
    pub fn load_from_file(input_file: &AnnotatedFenFile) -> Res<Dataset> {
        Self::load_from_file_impl(input_file)
    }

    pub fn load_from_file_list(files: &[AnnotatedFenFile], remove_uncommon: Option<Float>) -> Res<Dataset> {
        let mut res = Dataset::new(E::num_weights());
        for file in files {
            let file_res = Self::load_from_file_impl(file)?;
            res.union(file_res);
        }
        if let Some(max_occurrence) = remove_uncommon {
            let uncommon = list_uncommon::<E>(res.as_batch(), max_occurrence);
            println!(
                "Removing {0} positions with uncommon features: (<= {1} abs occurrence sum)",
                uncommon.len(),
                max_occurrence
            );
            res.remove(&uncommon);
            println!("There are {} data points left", res.data().len());
        }
        Ok(res)
    }
}

/// The index of a data point in a batch and the index of a feature in that data point
pub struct FeatureAppearance {
    /// All entries that belong to this data point.
    pub entries: Vec<Entry>,
    /// The index of the weight in that data point
    pub weight_idx: usize,
    /// The index in the list of all entries of the batch at which the entries of this data point start.
    pub global_start_idx: EntryIdxT,
}

/// Return a list of the indices of all data points where one of the gives `features` appears
pub fn list_with_features(batch: Batch, features: Vec<usize>) -> Vec<FeatureAppearance> {
    let mut res = vec![];
    assert!(features.is_sorted());
    for dp in batch.datapoints.iter() {
        let d = batch.entries_of(dp);
        debug_assert!(d.into_iter().map(|f| f.idx).is_sorted());
        for e in d {
            if features.binary_search(&e.idx).is_ok() {
                let appearance = FeatureAppearance {
                    entries: d.into_iter().copied().collect_vec(),
                    weight_idx: e.idx,
                    global_start_idx: dp.start_idx,
                };
                res.push(appearance);
                break;
            }
        }
    }
    res
}

/// Return a list of the indices of all data points where a rare feature appears, where "rare" means
/// any feature that occurs at most `max_occurrence` times in the entire batch.
pub fn list_uncommon<E: WeightsInterpretation>(batch: Batch, max_occurrence: Float) -> Vec<FeatureAppearance> {
    let mut entries = vec![];
    let occurrences = count_occurrences(batch);
    let mut feature_occurrences = vec![];
    for (idx, &num) in occurrences.iter().enumerate() {
        if 0.0 < num && num <= max_occurrence {
            entries.push((idx, num));
            feature_occurrences.push(num);
        }
    }
    let num_not_appearing = occurrences.into_iter().filter(|&n| n == 0.0).count();
    println!(
        "There are {} uncommon features. {num_not_appearing} weights never appear. Uncommon features:",
        entries.len()
    );
    for &(idx, sum) in &entries {
        println!("{0},\t abs weight sum: {1}", E::feature_name(idx), sum);
    }
    let features = entries.into_iter().map(|x| x.0).collect_vec();
    list_with_features(batch, features)
}
