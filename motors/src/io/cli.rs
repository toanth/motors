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
use gears::cli::{get_next_arg, parse_output, ArgIter, Game};
use gears::colored::Colorize;
use gears::general::common::Res;
use gears::OutputArgs;
use std::env;
use std::process::exit;
use std::str::FromStr;

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
    /// Should the engine start in interactive mode? The default is `true`; upon receiving a `ugi` command,
    /// the engine switches to non-interactive mode.
    pub interactive: bool,

    pub pos_name: Option<String>,

    pub cmds: Vec<String>,
}

impl EngineOpts {
    pub fn for_game(game: Game, debug: bool) -> Self {
        Self {
            game,
            engine: "default".to_string(),
            outputs: vec![],
            debug,
            interactive: true,
            pos_name: None,
            cmds: vec![],
        }
    }
}

fn parse_option(args: &mut ArgIter, opts: &mut EngineOpts) -> Res<()> {
    let mut key = args.next().unwrap_or_default().clone();
    // since we already accept -<long> in monitors for cutechess compatibility,
    // we might as well also accept it in motors.
    if key.starts_with("--") {
        _ = key.remove(0);
    }
    match key.as_str() {
        "-engine" | "-e" => opts.engine = get_next_arg(args, "engine")?,
        "-game" | "-g" => opts.game = Game::from_str(&get_next_arg(args, "engine")?.to_lowercase())?,
        "-debug" | "-d" => opts.debug = true,
        "-non-interactive" => opts.interactive = false,
        "-additional-output" | "-output" | "-o" => parse_output(args, &mut opts.outputs)?,
        "-help" => {
            print_help();
            exit(0);
        }
        _ => {
            opts.cmds.push(key);
            opts.cmds.push("wait".to_string());
        }
    }
    Ok(())
}

pub fn parse_cli(mut args: ArgIter) -> Res<EngineOpts> {
    let mut res = EngineOpts::for_game(Game::default(), false);
    if env::var("NO_COLOR").is_ok() {
        res.interactive = false;
    }
    while args.peek().is_some() {
        parse_option(&mut args, &mut res)?;
    }
    if !res.cmds.is_empty() {
        res.cmds.push("quit".to_string());
    }
    Ok(res)
}

// TODO: Use commands
fn print_help() {
    println!("`motors`, a collection of engines for various games, building on the `gears` crate.\
    \n\nBy default, this program starts the chess engine `CAPS` with the `LiTE` eval function.\
    \nAs an UCI engine, it's supposed to be used with a chess GUI, although it should be comparatively pleasant to manually interact with.
    There are a number of flags to change the default behavior (all of this can also be changed at runtime, though most GUIs won't make that easy):\
    \n--{game} sets the game. Currently, only `chess`, `ataxx`, `mnk`, `utt` and 'fairy' are supported; `chess` is the default.\
    \n--{engine} sets the engine, and optionally the eval. For example, `caps-lite` sets the default engine CAPS with the default eval LiTE,\
    and `random` sets the engine to be a random mover. Obviously, the engine must be valid for the selected game.\
    \n--{position} sets the position. Accepts the same syntax as UGI commands, e.g. 'position kiwipete' or 'p f <fen> m e2e4'. Ignored for 'bench'.\
    Use quotes around the argument.\
    \n--{debug} turns on debug mode, which makes the engine continue on errors and log all communications.\
    \n--{ni} makes the engine start in non-interactive mode. Try this if the engine can't be used with a GUI. Setting the NO_COLOR environment variable also does this.\
    \n--{add_outputs} can be used to determine how the engine prints extra information; it's mostly useful for development but can also be used to export PGNs, for example.\
    \n--{bench}, --{perft} and --{splitperft} are useful for testing the engine and move generation speed,\
    `bench` is also useful to get a \"hash\" of the search tree explored by the engine.\
    \n{other} arguments are interpreted as UGI commands followed by a `{wait}` command.\
    Typing '{help}' while the program is running will also show help messages",
        game = "game".underline().bold(),
        engine = "engine".underline().bold(),
        debug = "debug".underline().bold(),
        add_outputs = "additional-outputs".underline().bold(),
        bench = "bench".underline().bold(),
        perft = "perft".underline().bold(),
        splitperft = "splitperft".underline().bold(),
        help = "help".underline().bold(),
        ni = "non-interactive".underline().bold(),
        position = "position".underline().bold(),
        other = "All other".bold(),
        wait = "wait".bold(),
    )
}
