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

use crate::io::command::Standard::*;
use crate::io::EngineUGI;
use crate::io::ProgramStatus::Run;
use crate::io::Protocol::Interactive;
use crate::io::SearchType::{Bench, Normal, Perft, SplitPerft};
use arrayvec::ArrayVec;
use colored::Colorize;
use gears::general::board::Board;
use gears::general::common::anyhow::anyhow;
use gears::general::common::{NamedEntity, Res};
use gears::output::Message::Warning;
use gears::MatchStatus::NotStarted;
use gears::Quitting::{QuitMatch, QuitProgram};
use std::fmt::{Display, Formatter};
use std::iter::Peekable;
use std::str::SplitWhitespace;

#[derive(Debug, Eq, PartialEq)]
pub enum Standard {
    All,
    UgiNotUci,
    Custom,
}

#[derive(Debug)]
pub struct Command<B: Board> {
    pub primary_name: &'static str,
    pub other_names: ArrayVec<&'static str, 4>,
    pub help_text: Option<&'static str>,
    pub standard: Standard,
    pub func: fn(
        &mut EngineUGI<B>,
        remaining_input: &mut Peekable<SplitWhitespace>,
        _cmd: &str,
    ) -> Res<()>,
}

impl<B: Board> Display for Command<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{0}: {1}.",
            self.primary_name.bold(),
            self.help_text.unwrap()
        )
    }
}

impl<B: Board> NamedEntity for Command<B> {
    fn short_name(&self) -> String {
        self.primary_name.to_string()
    }

    fn long_name(&self) -> String {
        self.short_name()
    }

    fn description(&self) -> Option<String> {
        self.help_text.map(|s| s.to_string())
    }

    fn matches(&self, name: &str) -> bool {
        name.eq_ignore_ascii_case(self.primary_name)
            || self
                .other_names
                .iter()
                .any(|n| n.eq_ignore_ascii_case(name))
    }
}

macro_rules! command {
    ($primary:ident, [$($other:ident),*], $std:expr, $($help:expr)?, $fun:expr) => {
        {
            #[allow(unused)]
            let mut help_text = None;
            $(help_text = Some($help))?;
            Command {
                primary_name: stringify!($primary),
                other_names: ArrayVec::from_iter([$(stringify!($other),)*]),
                standard: $std,
                func: $fun,
                help_text,
            }
        }
    };
}

pub fn ugi_commands<B: Board>() -> Vec<Command<B>> {
    vec![
        // put time-critical commands at the top
        command!(
            go,
            [g, search],
            All,
            "Start the search. Optionally takes a position and a mode such as `perft`",
            |ugi, words, _| { ugi.handle_go(Normal, words) }
        ),
        command!(
            stop,
            [],
            All,
            "Stop the current search. No effect if not searching",
            |ugi, _, _| {
                ugi.state.engine.send_stop(false);
                Ok(())
            }
        ),
        command!(
            position,
            [pos, p],
            All,
            "Set the current position",
            |ugi, words, _| ugi.state.handle_position(words)
        ),
        command!(
            ugi,
            [uci, uai],
            All,
            "Starts UGI mode, ends interactive mode (can be re-enabled with `interactive`)",
            |ugi, _, proto| ugi.handle_ugi(proto)
        ),
        command!(
            ponderhit,
            [],
            All,
            "Stop pondering and start a normal search",
            |ugi, _, cmd| ugi.start_search(
                Normal,
                ugi.state.ponder_limit.ok_or_else(|| {
                    anyhow!(
                        "The engine received a '{}' command but wasn't pondering",
                        cmd.bold()
                    )
                })?,
                ugi.state.board,
                None,
                ugi.multi_pv,
            )
        ),
        command!(
            isready,
            [],
            All,
            "Queries if the engine is ready. The engine responds with 'readyok'",
            |ugi, _, _| {
                ugi.write_ugi("readyok");
                Ok(())
            }
        ),
        command!(
            setoption,
            [],
            All,
            "Sets an engine option",
            |ugi, words, _| ugi.handle_setoption(words)
        ),
        command!(
            uginewgame,
            [ucinewgame, uainewgame, clear],
            All,
            "Resets the internal engine state (doesn't reset engine options)",
            |ugi, _, _| {
                ugi.state.engine.send_forget()?;
                ugi.state.status = Run(NotStarted);
                Ok(())
            }
        ),
        command!(
            register,
            [],
            All,
            "UCI command for copy-protected engines, doesn't apply here",
            |ugi, _, _| {
                ugi.write_message(
                    Warning,
                    &format!("{} isn't supported and will be ignored", "register".red()),
                );
                return Ok(());
            }
        ),
        command!(
            flip,
            [],
            Custom,
            "Flips the side to move, unless this results in an illegal position",
            |ugi, _, _| {
                ugi.state.board = ugi.state.board.make_nullmove().ok_or(anyhow!(
                    "Could not flip the side to move (board: '{}'",
                    ugi.state.board.as_fen().bold()
                ))?;
                Ok(())
            }
        ),
        command!(
            quit,
            [],
            All,
            "Exits the program immediately",
            |ugi, _, _| ugi.quit(QuitProgram)
        ),
        command!(
            quit_match,
            [end_game, qm],
            Custom,
            "Quits the current match and, if `play` has been used, returns to the previous match",
            |ugi, _, _| ugi.quit(QuitMatch)
        ),
        command!(
            query,
            [],
            UgiNotUci,
            "Answer a query about the current match state",
            |ugi, words, _| ugi.handle_query(words)
        ),
        command!(
            option,
            [info],
            Custom,
            "Prints information about the current options. Optionally takes an option name",
            |ugi, words, _| {
                ugi.write_ugi(&ugi.write_option(words)?);
                Ok(())
            }
        ),
        command!(
            output,
            [o],
            Custom,
            "Adds outputs. Use `remove (all)` to remove specified outputs",
            |ugi, words, _| ugi.handle_output(words)
        ),
        command!(
            print,
            [show, s, display],
            Custom,
            "Display the specified / current position with specified / enabled outputs",
            |ugi, words, _| ugi.handle_print(words)
        ),
        command!(
            log,
            [],
            Custom,
            "Enables logging. Can optionally specify a file name, `stdout` / `stderr` or `off`",
            |ugi, words, _| ugi.handle_log(words)
        ),
        command!(
            debug,
            [d],
            Custom,
            "Turns on logging, continue-on-error mode, and additional output. Use `off` to disable",
            |ugi, words, _| ugi.handle_debug(words)
        ),
        command!(
            interactive,
            [i, human],
            Custom,
            "Starts interactive mode, undoes `ugi`. In this mode, errors aren't fatal",
            |ugi, _, _| {
                ugi.state.protocol = Interactive;
                Ok(())
            }
        ),
        command!(
            engine,
            [],
            Custom,
            "Sets the current engine, e.g. `caps-piston`, `gaps`, and optionally the game",
            |ugi, words, _| ugi.handle_engine(words)
        ),
        command!(
            set_eval,
            [],
            Custom,
            "Sets the eval for the current engine. Doesn't reset the internal engine state",
            |ugi, words, _| ugi.handle_set_eval(words)
        ),
        command!(
            play,
            [game],
            Custom,
            "Starts a new match, possibly of a new game, optionally setting a new engine",
            |ugi, words, _| ugi.handle_play(words)
        ),
        command!(
            perft,
            [],
            Custom,
            "Internal movegen test on current / bench positions",
            |ugi, words, _| ugi.handle_go(Perft, words)
        ),
        command!(
            splitperft,
            [sp],
            Custom,
            "Internal movegen test on current / bench positions",
            |ugi, words, _| ugi.handle_go(SplitPerft, words)
        ),
        command!(
            bench,
            [],
            Custom,
            "Internal search test on current / bench positions. Same arguments as `go`",
            |ugi, words, _| ugi.handle_go(Bench, words)
        ),
        command!(
            eval,
            [e, static_eval],
            Custom,
            "Print the static eval (i.e., no search) of a position",
            |ugi, words, _| ugi.handle_eval_or_tt(true, words)
        ),
        command!(
            tt,
            [tt_entry],
            Custom,
            "Print the TT entry for a position",
            |ugi, words, _| ugi.handle_eval_or_tt(false, words)
        ),
        command!(
            list,
            [],
            Custom,
            "Lists available options for a command",
            |ugi, words, _| ugi.handle_list(words)
        ),
        command!(help, [h], Custom, "Prints a help message", |ugi, _, _| {
            ugi.print_help(); // TODO: allow help <command> to print a help message for a command
            Ok(())
        }),
    ]
}
