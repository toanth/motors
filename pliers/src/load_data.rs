//! Everything related to loading and converting lists of annotated FENs into a [`Dataset`].

use crate::eval::{Eval, WeightsInterpretation, count_occurrences};
use crate::gd::{Batch, Datapoint, Dataset, Float, Outcome, PosIndex};
use crate::load_data::Perspective::{SideToMove, White};
use crate::trace::TraceTrait;
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

    fn read_annotated_fen(input: &str, perspective: Perspective, weight: Float) -> Res<ParseResult<B>> {
        let mut input = tokens(input);
        let pos = B::read_fen_and_advance_input(&mut input, Relaxed)?;
        // skip up to one token between the end of the fen and the wdl
        let mut outcome = Self::parse_wdl(&mut input).or_else(|err| Self::parse_wdl(&mut input).or(Err(err)))?;
        if perspective == SideToMove && pos.active_player() == B::Color::second() {
            outcome.0 = 1.0 - outcome.0;
        }
        Ok(ParseResult { pos, outcome, weight })
    }

    fn load_datapoint_from_annotated_fen(
        input: &str,
        line_num: usize,
        perspective: Perspective,
        index: PosIndex,
        weight: Float,
        dataset: &mut Dataset<E::D>,
    ) -> Res<()> {
        let parse_res = Self::read_annotated_fen(input, perspective, weight)
            .map_err(|err| anyhow!("Error in line {0}: Couldn't parse FEN '{1}': {err}", line_num + 1, input.bold()))?;
        for datapoint in E::Filter::filter(parse_res) {
            dataset.push(E::extract_features(&datapoint.pos, datapoint.outcome, index, datapoint.weight));
        }
        Ok(())
    }

    fn load_from_file_impl(input_file: &AnnotatedFenFile, fens_read_so_far: usize) -> Res<Dataset<E::D>> {
        let file = File::open(Path::new(&input_file.path))
            .map_err(|err| anyhow!("Could not open file '{}': {err}", input_file.path))?;
        let file = BufReader::new(file);
        let perspective = input_file.perspective;
        let weight = input_file.weight.unwrap_or(1.0);
        println!(
            "Loading FENs from file '{0}' (Outcomes are {perspective} relative), sampling weight: {weight:.1}",
            input_file.path.as_str().bold()
        );
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
                let index = PosIndex(line_num + fens_read_so_far);
                Self::load_datapoint_from_annotated_fen(&line, line_num, perspective, index, weight, &mut dataset)?;
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

    /// Load FENs from a [`&str`] instead of a file.
    ///
    /// This is primarily intended for debugging and small examples.
    pub fn load_from_str(annotated_fens: &str, perspective: Perspective) -> Res<Dataset<E::D>> {
        let mut res = Dataset::new(E::num_weights());
        for (idx, line) in annotated_fens.lines().enumerate() {
            let fens_read_so_far = PosIndex(res.data().len());
            Self::load_datapoint_from_annotated_fen(line, idx, perspective, fens_read_so_far, 1.0, &mut res)?;
        }
        Ok(res)
    }

    /// Load annotated FENs from a file.
    ///
    /// Regularly prints ou the number of loaded FENs.
    /// Fails if there is any invalid FEN in the dataset.
    #[allow(unused)]
    pub fn load_from_file(input_file: &AnnotatedFenFile) -> Res<Dataset<E::D>> {
        Self::load_from_file_impl(input_file, 0)
    }

    pub fn load_from_file_list(files: &[AnnotatedFenFile], remove_uncommon: Option<Float>) -> Res<Dataset<E::D>> {
        let mut res = Dataset::new(E::num_weights());
        let mut lines_prefix_sum: Vec<usize> = vec![0];
        let mut read_so_far = 0;
        for file in files {
            let file_res = Self::load_from_file_impl(file, read_so_far)?;
            read_so_far = file_res.data().last().map(|e| e.index().0 + 1).unwrap_or(0);
            lines_prefix_sum.push(read_so_far);
            res.union(file_res);
        }
        if let Some(max_occurrence) = remove_uncommon {
            let uncommon = list_uncommon::<E::D, E>(res.as_batch(), max_occurrence);
            println!(
                "Removing {0} positions with uncommon features: (<= {1} abs occurrence sum)",
                uncommon.len(),
                max_occurrence
            );
            for rare_feature in &uncommon {
                let file = lines_prefix_sum.partition_point(|&i| i <= rare_feature.data_point_idx.0) - 1;
                let idx = rare_feature.data_point_idx.0 - lines_prefix_sum[file];
                let file = &files[file].path;
                let reader = BufReader::new(File::open(Path::new(file))?);
                let line = reader.lines().nth(idx).unwrap()?;
                assert!(
                    rare_feature.feature_idx < E::num_features(),
                    "{0} {1}",
                    rare_feature.feature_idx,
                    E::num_features()
                );
                let pos = Self::read_annotated_fen(&line, White, 1.0).unwrap().pos;
                let features = E::feature_trace(&pos).as_features(0);
                assert!(features.iter().any(|f| f.idx() == rare_feature.feature_idx));
                println!(
                    "Feature: {0},\t appears {1:.5} times in \tFEN: {line}",
                    E::feature_name(rare_feature.feature_idx),
                    rare_feature.count
                );
            }
            res.remove(&uncommon);
            println!("There are {} data points left", res.data().len());
        }
        Ok(res)
    }
}

/// The index of a data point in a batch and the index of a feature in that data point
pub struct FeatureAppearance {
    /// The index of the data point in the batch
    pub data_point_idx: PosIndex,
    /// The index of the feature in that data point
    pub feature_idx: usize,
    /// How often the feature appeared, from white's perspective
    pub count: Float,
}

/// Return a list of the indices of all data points where one of the gives `features` appears
pub fn list_with_features<D: Datapoint>(batch: Batch<D>, features: Vec<usize>) -> Vec<FeatureAppearance> {
    let mut res = vec![];
    assert!(features.is_sorted());
    for d in batch.datapoints.iter() {
        debug_assert!(d.weights().map(|f| f.idx).is_sorted());
        for f in d.weights() {
            if let Ok(i) = features.binary_search(&D::feature_idx(f)) {
                res.push(FeatureAppearance { data_point_idx: d.index(), feature_idx: features[i], count: f.weight });
                break;
            }
        }
    }
    res
}

/// Return a list of the indices of all data points where a rare feature appears, where "rare" means
/// any feature that occurs at most `max_occurrence` times in the entire batch.
pub fn list_uncommon<D: Datapoint, E: WeightsInterpretation>(
    batch: Batch<D>,
    max_occurrence: Float,
) -> Vec<FeatureAppearance> {
    let mut features = vec![];
    let occurrences = count_occurrences(batch);
    let mut feature_occurrences = vec![];
    for (idx, &num) in occurrences.iter().enumerate() {
        if 0.0 < num && num <= max_occurrence {
            features.push((idx / D::num_weights_per_feature(), num));
            feature_occurrences.push(num);
        }
    }
    features = features.into_iter().dedup_by(|a, b| a.0 == b.0).collect_vec();
    let num_not_appearing = occurrences.into_iter().filter(|&n| n == 0.0).count();
    println!(
        "There are {} uncommon features. {num_not_appearing} weights never appear. Uncommon features:",
        features.len()
    );
    for &(idx, sum) in &features {
        println!("{0},\t abs weight sum: {1}", E::feature_name(idx), sum);
    }
    let features = features.into_iter().map(|x| x.0).collect_vec();
    list_with_features(batch, features)
}
