use crate::eval::Eval;
use crate::gd::{Datapoint, Dataset, Float, Outcome, Position};
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
pub struct FenReader<B: Board> {
    _phantom_data: PhantomData<B>,
}

impl<B: Board> FenReader<B> {
    fn parse_wdl(input: &mut SplitWhitespace) -> Res<Outcome> {
        // This would be a great time to use the `.remainder()` method, but that isn't stable :/
        let wdl = input.next().ok_or_else(|| "Missing wdl".to_string())?;
        const IGNORED: [char; 7] = ['\"', '\'', '[', '(', '{', ' ', '\t'];
        let wdl = wdl.trim_start_matches(&IGNORED);
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

    fn read_fen_and_res(input: &str) -> Res<ParseResult<B>> {
        let mut input = input.split_whitespace();
        let pos = B::read_fen_and_advance_input(&mut input)?;
        // skip up to one token between the end of the fen and the wdl
        let outcome =
            Self::parse_wdl(&mut input).or_else(|err| Self::parse_wdl(&mut input).or(Err(err)))?;
        Ok(ParseResult { pos, outcome })
    }

    pub fn load_from_str(annotated_fens: &str) -> Res<Vec<ParseResult<B>>> {
        let mut res = vec![];
        for line in annotated_fens.lines() {
            res.push(Self::read_fen_and_res(line)?);
        }
        Ok(res)
    }

    pub fn load_from_file(file: File) -> Res<Vec<ParseResult<B>>> {
        let reader = BufReader::new(file);
        let mut res = vec![];
        let mut line_no = 1;
        for line in reader.lines() {
            let line = line.map_err(|err| format!("Failed to read line {line_no}: {err}"))?;
            let line = Self::read_fen_and_res(line.as_str()).map_err(|err| {
                format!(
                    "Error in line {line_no}: Couldn't parse FEN '{}': {err}",
                    line.bold()
                )
            })?;
            res.push(line);
            if line_no % 10_000 == 0 {
                println!("Loading...  Loaded {line_no} fens so far");
            }
            line_no += 1;
        }
        println!("Loaded {line_no} fens in total");
        Ok(res)
    }
}

pub fn parse_res_to_dataset<B: Board, E: Eval<B>>(parsed: &[ParseResult<B>]) -> Dataset {
    let mut res = Vec::with_capacity(parsed.len());
    let mut num_parsed = 0;
    for p in parsed {
        let position = E::features(&p.pos);
        res.push(Datapoint {
            position,
            outcome: p.outcome,
        });
        num_parsed += 1;
        if num_parsed % 10_000 == 0 {
            println!("Parsed {num_parsed} positions");
        }
    }
    res
}

pub fn parse_from_str<B: Board, E: Eval<B>>(fens: &str) -> Res<Dataset> {
    let res = FenReader::<B>::load_from_str(fens)?;
    Ok(parse_res_to_dataset::<B, E>(&res))
}

pub fn parse_from_file<B: Board, E: Eval<B>>(file: File) -> Res<Dataset> {
    let res = FenReader::<B>::load_from_file(file)?;
    Ok(parse_res_to_dataset::<B, E>(&res))
}
