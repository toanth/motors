/*
 *  Motors, a collection of board game engines.
 *  Copyright (C) 2024 ToTheAnd
 *
 *  Motors is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Motors is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Motors. If not, see <https://www.gnu.org/licenses/>.
 */

use colored::Colorize;
use std::process::exit;
use std::str::FromStr;

use crate::Mode;
use gears::cli::{get_next_arg, get_next_int, parse_output, ArgIter, Game};
use gears::general::common::anyhow::bail;
use gears::general::common::{parse_int_from_str, Res};
use gears::search::Depth;
use gears::OutputArgs;

use crate::Mode::{Bench, Engine, Perft};

#[derive(Debug, Clone)]
#[must_use]
pub struct EngineOpts {
    pub game: Game,
    /// The name of the engine
    pub engine: String,
    /// An output prints the current position after each move and is also used to show (error) messages.
    pub outputs: Vec<OutputArgs>,
    /// Used to debug the engine. Enables logging as if by using `logger` as additional output.
    pub debug: bool,

    pub mode: Mode,
}

impl EngineOpts {
    pub fn for_game(game: Game, debug: bool) -> Self {
        Self {
            game,
            engine: "default".to_string(),
            outputs: vec![],
            debug,
            mode: Engine,
        }
    }
}

fn parse_depth(args: &mut ArgIter) -> Res<Option<Depth>> {
    if let Some(next) = args.peek() {
        if next == "-d" || next == "--depth" {
            args.next();
            if args.peek().is_some_and(|a| a != "default") {
                return Ok(Some(Depth::new(get_next_int(args, "depth")?)));
            }
        } else if let Ok(val) = parse_int_from_str(next, "bench depth") {
            args.next();
            return Ok(Some(Depth::new(val)));
        }
    }
    Ok(None)
}

fn parse_bench(args: &mut ArgIter) -> Res<Option<Depth>> {
    parse_depth(args)
}

fn parse_perft(args: &mut ArgIter) -> Res<Option<Depth>> {
    parse_depth(args)
}

fn parse_option(args: &mut ArgIter, opts: &mut EngineOpts) -> Res<()> {
    let mut key = args.next().unwrap_or_default().clone();
    // since we already accept -<long> in monitors for cutechess compatibility,
    // we might as well also accept it in motors.
    if key.starts_with("--") {
        key.remove(0);
    }
    match key.as_str() {
        "bench" | "-bench" | "-b" | "b" => opts.mode = Bench(parse_bench(args)?, true),
        "bench-simple" | "-bench-simple" | "-bs" | "bs" => opts.mode = Bench(parse_bench(args)?, false),
        "perft" | "-perft" | "-p" => opts.mode = Perft(parse_perft(args)?),
        "-engine" | "-e" => opts.engine = get_next_arg(args, "engine")?,
        "-game" | "-g" => opts.game = Game::from_str(&get_next_arg(args, "engine")?.to_lowercase())?,
        "-debug" | "-d" => opts.debug = true,
        "-additional-output" | "-output" | "-o" => parse_output(args, &mut opts.outputs)?,
        "-help" => { print_help(); exit(0); },
        x => bail!("Unrecognized option '{x}'. Only 'bench', 'bench-simple', 'perft', '--engine', '--game', '--debug' and '--outputs' are valid.")
    }
    Ok(())
}

pub fn parse_cli(mut args: ArgIter) -> Res<EngineOpts> {
    let mut res = EngineOpts::for_game(Game::default(), false);
    while args.peek().is_some() {
        parse_option(&mut args, &mut res)?;
    }
    Ok(res)
}

fn print_help() {
    println!("`motors`, a collection of engines for various games, building on the `gears` crate.\
    \n\nBy default, this program starts the chess engine `CAPS` with the `LiTE` eval function.\
    \nAs an UCI engine, it's supposed to be used with a chess GUI, although it should be comparatively pleasant to manually interact with.
    There are a number of flags to change the default behavior (all of this can also be changed at runtime, though most GUIs won't make that easy):\
    \n--{0} sets the game. Currently, only `chess`, `ataxx` and `mnk` are supported; `chess` is the default.\
    \n--{1} sets the engine, and optionally the eval. For example, `caps-lite` sets the default engine CAPS with the default eval LiTE,\
    and `random` sets the engine to be a random mover. Obviously, the engine must be valid for the selected game.\
    \n--{2} turns on debug mode, which makes the engine continue on errors and log all communications.\
    \n--{3} can be used to determine how the engine prints extra information; it's mostly useful for development but can also be used to export PGNs, for example.\
    \n--{4} and --{5} are useful for testing the engine and move generation speed, respectively,\
    `bench` is also useful to get a \"hash\" of the search tree explored by the engine.\
    Typing '{6}' while the program is running will also show help messages",
    "game".bold(),
    "engine".bold(),
    "debug".bold(),
    "additional-outputs".bold(),
    "bench".bold(),
    "perft".bold(),
    "help".bold()
    )
}
