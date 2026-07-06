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

fn parse_option(args: &mut ArgIter, opts: &mut EngineOpts, quit_at_end: &mut bool) -> Res<()> {
    let mut key = args.next().unwrap_or_default().clone();
    // since we already accept -<long> in monitors for cutechess compatibility,
    // we might as well also accept it in motors.
    if key.starts_with("--") {
        _ = key.remove(0);
    }
    match key.as_str() {
        "-engine" | "-e" => opts.engine = get_next_arg(args, "engine")?,
        "-game" | "-g" => opts.game = Game::from_str(&get_next_arg(args, "engine")?.to_lowercase())?,
        "-dont-quit" => *quit_at_end = false,
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
    let mut quit_at_end = true;
    let mut res = EngineOpts::for_game(Game::default(), false);
    if env::var("NO_COLOR").is_ok() {
        res.interactive = false;
    }
    while args.peek().is_some() {
        parse_option(&mut args, &mut res, &mut quit_at_end)?;
    }
    if !res.cmds.is_empty() && quit_at_end {
        res.cmds.push("quit".to_string());
    }
    Ok(res)
}

// TODO: Use commands
fn print_help() {
    println!(
        "`motors`, a collection of engines for various games, building on the `gears` crate.\
    \n\nBy default, this program starts the chess engine `CAPS` with the `LiTE` eval function.\
    \nAs an UCI engine, it's supposed to be used with a chess GUI, although it should be comparatively pleasant to manually interact with.\
    \nThere are a number of flags to change the default behavior (all of this can also be changed at runtime, though most GUIs won't make that easy):\
    \n--{game} sets the game. Currently, only `chess`, `ataxx`, `mnk`, `utt` and 'fairy' are supported; `chess` is the default.\
    \n--{engine} sets the engine, and optionally the eval. For example, `caps-lite` sets the default engine CAPS with the default eval LiTE,\
    and `random` sets the engine to be a random mover. Obviously, the engine must be valid for the selected game.\
    \n--{dont_quit} means that the engine won't quit after it has finished executing UGI commands given as command line arguments. \
    Has no effect if there are no UGI commands as command line flags.\
    \n--{debug} turns on debug mode, which makes the engine continue on errors, log all communications, and print debug info to stderr.\
    \n--{ni} makes the engine start in non-interactive mode. Try this if the engine can't be used with a GUI. Setting the NO_COLOR environment variable also does this.\
    \n--{add_outputs} can be used to determine how the engine prints extra information; it's mostly useful for development but can also be used to export PGNs, for example.\
    \n{other} arguments are interpreted as UGI commands followed by a `{wait}` command. For example, \"position lucena\" \"go depth 20\" \"tt\" makes the engine \
    search in the 'lucena' position and print the TT entry for that position when done; \"bench\" will run bench, and so on. \
    Afterwards it will quit, unless the '--{dont_quit}' flag is present.\
    \nTyping '{help}' while the program is running will also show help messages",
        game = "game".underline().bold(),
        engine = "engine".underline().bold(),
        dont_quit = "dont-quit".underline().bold(),
        debug = "debug".underline().bold(),
        add_outputs = "additional-outputs".underline().bold(),
        help = "help".underline().bold(),
        ni = "non-interactive".underline().bold(),
        other = "All other".bold(),
        wait = "wait".bold(),
    )
}
