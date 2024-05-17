use crate::eval::Eval;
use crate::gd::{Datapoint, Dataset, Float, Outcome, Position};
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
        const IGNORED: [char; 10] = ['\"', '\'', '[', ']', '(', ')', '{', '}', ' ', '\t'];
        let wdl = wdl.trim_matches(&IGNORED);
        for (key, value) in WDL_MAP {
            if key == wdl {
                return Ok(Outcome::new(value));
            }
        }
        if let Ok(parsed) = parse_fp_from_str(wdl, "wdl") {
            return Ok(Outcome::new(parsed));
        }
        Err(format!("'{wdl}' is not a valid wdl"))
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
        let mut line_no = 0;
        for line in reader.lines() {
            res.push(Self::read_fen_and_res(
                line.map_err(|err| err.to_string())?.as_str(),
            )?);
            line_no += 1;
            if line_no % 10_000 == 0 {
                println!("Loading...  Loaded {line_no} fens so far");
            }
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
