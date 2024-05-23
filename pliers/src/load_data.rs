use crate::eval::Eval;
use crate::gd::{Dataset, Float, Outcome};
use colored::Colorize;
use gears::games::Board;
use gears::general::common::{parse_fp_from_str, Res};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::marker::PhantomData;
use std::str::SplitWhitespace;

const WDL_MAP: [(&str, Float); 4] = [
    ("0-1", 0.0),
    ("1/2-1/2", 0.5),
    ("0.5-0.5", 0.5),
    ("1-0", 1.0),
];

pub struct ParseResult<B: Board> {
    pos: B,
    outcome: Outcome,
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
                "Error in line {line_num}: Couldn't parse FEN '{}': {err}",
                input.bold()
            )
        })?;
        // in the future, this would be a good place to filter the dataset.
        dataset
            .datapoints
            .push(E::extract_features(&parse_res.pos, parse_res.outcome));
        Ok(())
    }

    pub fn load_from_str(annotated_fens: &str) -> Res<Dataset<E::D>> {
        let mut res = Dataset::new(E::NUM_FEATURES);
        for (idx, line) in annotated_fens.lines().enumerate() {
            Self::load_datapoint_from_annotated_fen(line, idx, &mut res)?;
        }
        Ok(res)
    }

    pub fn load_from_file(file: File) -> Res<Dataset<E::D>> {
        let reader = BufReader::new(file);
        let mut res = Dataset::new(E::NUM_FEATURES);
        let mut line_no = 1;
        for (idx, line) in reader.lines().enumerate() {
            let line = line.map_err(|err| format!("Failed to read line {line_no}: {err}"))?;
            Self::load_datapoint_from_annotated_fen(&line, idx, &mut res)?;
            if line_no % 100_000 == 0 {
                println!("Loading...  Loaded {line_no} fens so far");
            }
            line_no += 1;
        }
        println!("Loaded {line_no} fens in total");
        Ok(res)
    }
}
