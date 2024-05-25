use crate::eval::Eval;
use crate::gd::{Datapoint, Dataset, Float, Outcome};
use colored::Colorize;
use gears::games::Board;
use gears::general::common::{parse_fp_from_str, Res};
use std::fmt::Pointer;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::marker::PhantomData;
use std::path::Path;
use std::str::SplitWhitespace;

const WDL_MAP: [(&str, Float); 4] = [
    ("0-1", 0.0),
    ("1/2-1/2", 0.5),
    ("0.5-0.5", 0.5),
    ("1-0", 1.0),
];

pub struct ParseResult<B: Board> {
    pub pos: B,
    pub outcome: Outcome,
}

pub trait Filter<B: Board> {
    /// Returns an iterator because it's possible for a `Filter` to return more than one position per input position.
    /// Filtering could also include running a low-depth search with an engine to relabel the outcome.
    fn filter(pos: ParseResult<B>) -> impl IntoIterator<Item = ParseResult<B>>;
}

pub struct NoFilter {}

impl<B: Board> Filter<B> for NoFilter {
    fn filter(pos: ParseResult<B>) -> impl IntoIterator<Item = ParseResult<B>> {
        [pos]
    }
}

/// Avoids having to specify the generic Board argument each time.
#[derive(Default)]
pub struct FenReader<B: Board, E: Eval<B>> {
    _phantom_data: PhantomData<B>,
    _phantom_data2: PhantomData<E>,
}

impl<B: Board, E: Eval<B>> FenReader<B, E> {
    fn parse_wdl(input: &mut SplitWhitespace) -> Res<Outcome> {
        // This would be a great time to use the `.remainder()` method, but that isn't stable :/
        let wdl = input.next().ok_or_else(|| "Missing wdl".to_string())?;
        const IGNORED: [char; 10] = ['\"', '\'', '[', ']', '(', ')', '{', '}', ' ', '\t'];
        let wdl = wdl.trim_matches(&IGNORED);
        for (key, value) in WDL_MAP {
            if wdl.starts_with(key) {
                return Ok(Outcome::new(value));
            }
        }
        if let Ok(parsed) = parse_fp_from_str(wdl, "wdl") {
            return Ok(Outcome::new(parsed));
        }
        Err(format!("'{}' is not a valid wdl", wdl.red()))
    }

    fn read_annotated_fen(input: &str) -> Res<ParseResult<B>> {
        let mut input = input.split_whitespace();
        let pos = B::read_fen_and_advance_input(&mut input)?;
        // skip up to one token between the end of the fen and the wdl
        let outcome =
            Self::parse_wdl(&mut input).or_else(|err| Self::parse_wdl(&mut input).or(Err(err)))?;
        Ok(ParseResult { pos, outcome })
    }

    fn load_datapoint_from_annotated_fen(
        input: &str,
        line_num: usize,
        dataset: &mut Dataset<E::D>,
    ) -> Res<()> {
        let parse_res = Self::read_annotated_fen(input).map_err(|err| {
            format!(
                "Error in line {0}: Couldn't parse FEN '{1}': {err}",
                line_num + 1,
                input.bold()
            )
        })?;
        for datapoint in E::Filter::filter(parse_res) {
            dataset
                .datapoints
                .push(E::extract_features(&datapoint.pos, datapoint.outcome))
        }
        Ok(())
    }

    pub fn load_from_str(annotated_fens: &str) -> Res<Dataset<E::D>> {
        let mut res = Dataset::new(E::NUM_WEIGHTS);
        for (idx, line) in annotated_fens.lines().enumerate() {
            Self::load_datapoint_from_annotated_fen(line, idx, &mut res)?;
        }
        Ok(res)
    }

    pub fn load_from_file(file_name: &str) -> Res<Dataset<E::D>> {
        let file = File::open(Path::new(file_name))
            .map_err(|err| format!("Could not open file '{file_name}': {err}"))?;
        let file = BufReader::new(file);
        println!("Loading FENs from file '{}'", file_name.bold());
        let reader = BufReader::new(file);
        let mut res = Dataset::new(E::NUM_WEIGHTS);
        let mut line_num = 0;
        for line in reader.lines() {
            line_num += 1;
            let line = line.map_err(|err| format!("Failed to read line {line_num}: {err}"))?;
            Self::load_datapoint_from_annotated_fen(&line, line_num - 1, &mut res)?;
            if line_num % 100_000 == 0 {
                println!("Loading...  Read {line_num} lines so far");
            }
        }
        println!(
            "Read {line_num} fens in total, after filtering there are {} positions",
            res.datapoints.len()
        );
        Ok(res)
    }
}
