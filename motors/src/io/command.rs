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
use crate::io::ProgramStatus::Run;
use crate::io::Protocol::Interactive;
use crate::io::SearchType::{Bench, Normal, Perft, Ponder, SplitPerft};
use crate::io::{EngineUGI, SearchType};
use arrayvec::ArrayVec;
use colored::Colorize;
use edit_distance::edit_distance;
use gears::games::Color;
use gears::general::board::Board;
use gears::general::common::anyhow::anyhow;
use gears::general::common::{parse_duration_ms, parse_int, parse_int_from_str, NamedEntity, Res};
use gears::output::Message::Warning;
use gears::search::{Depth, NodesLimit, SearchLimit};
use gears::ugi::load_ugi_position;
use gears::GameResult;
use gears::MatchStatus::{NotStarted, Ongoing, Over};
use gears::Quitting::{QuitMatch, QuitProgram};
use inquire::autocompletion::Replacement;
use inquire::{Autocomplete, CustomUserError};
use itertools::Itertools;
use std::fmt::{Debug, Display, Formatter};
use std::iter::{once, Peekable};
use std::rc::Rc;
use std::str::{from_utf8, SplitWhitespace};
use std::time::Duration;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Standard {
    All,
    UgiNotUci,
    Custom,
}

pub trait CommandTrait<State: Debug>: NamedEntity + Display {
    fn func(
        &self,
    ) -> fn(&mut State, remaining_input: &mut Peekable<SplitWhitespace>, _cmd: &str) -> Res<()>;

    fn standard(&self) -> Standard;

    fn upcast_box(self: Box<Self>) -> Box<dyn NamedEntity>;

    fn upcast_ref(&self) -> &dyn NamedEntity;
}

fn display_cmd<S: Debug>(f: &mut Formatter<'_>, cmd: &dyn CommandTrait<S>) -> std::fmt::Result {
    if let Some(desc) = cmd.description() {
        write!(f, "{}: {desc}.", cmd.short_name().bold())
    } else {
        write!(f, "{}", cmd.short_name().bold())
    }
}

#[derive(Debug)]
pub struct SimpleCommand<State: Debug> {
    pub primary_name: &'static str,
    pub other_names: ArrayVec<&'static str, 4>,
    pub help_text: &'static str,
    pub standard: Standard,
    pub func:
        fn(&mut State, remaining_input: &mut Peekable<SplitWhitespace>, _cmd: &str) -> Res<()>,
    pub sub_commands: Vec<Box<dyn NamedEntity>>,
}

impl<State: Debug> NamedEntity for SimpleCommand<State> {
    fn short_name(&self) -> String {
        self.primary_name.to_string()
    }

    fn long_name(&self) -> String {
        self.short_name()
    }

    fn description(&self) -> Option<String> {
        Some(self.help_text.to_string())
    }

    fn matches(&self, name: &str) -> bool {
        name.eq_ignore_ascii_case(self.primary_name)
            || self
                .other_names
                .iter()
                .any(|n| n.eq_ignore_ascii_case(name))
    }

    fn autocomplete_badness(&self, input: &str, matcher: fn(&str, &str) -> usize) -> usize {
        matcher(input, self.primary_name).min(
            self.other_names
                .iter()
                // prefer primary matches
                .map(|name| 1 + matcher(input, *name))
                .min()
                .unwrap_or(usize::MAX),
        )
    }

    fn sub_entities_completion(&self) -> &[Box<dyn NamedEntity>] {
        self.sub_commands.as_slice()
    }

    fn secondary_names(&self) -> Vec<String> {
        self.other_names.iter().map(|s| s.to_string()).collect_vec()
    }
}

impl<State: Debug + 'static> Display for SimpleCommand<State> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        display_cmd(f, self)
    }
}

impl<State: Debug + 'static> CommandTrait<State> for SimpleCommand<State> {
    fn func(&self) -> fn(&mut State, &mut Peekable<SplitWhitespace>, &str) -> Res<()> {
        self.func
    }

    fn standard(&self) -> Standard {
        self.standard
    }

    fn upcast_box(self: Box<Self>) -> Box<dyn NamedEntity> {
        self
    }

    fn upcast_ref(&self) -> &dyn NamedEntity {
        self
    }
}

pub struct GenericCommand<State: Debug> {
    primary_name: Box<dyn Fn(&Self) -> String>,
    other_names: Vec<Box<dyn Fn(&Self) -> String>>,
    help_text: Box<dyn Fn(&Self) -> String>,
    standard: Box<dyn Fn(&Self) -> Standard>,
    func: Box<
        dyn Fn(
            &Self,
        ) -> fn(
            &mut State,
            remaining_input: &mut Peekable<SplitWhitespace>,
            _cmd: &str,
        ) -> Res<()>,
    >,
    matches: Option<Box<dyn Fn(&Self, &str) -> bool>>,
}

impl<State: Debug> Debug for GenericCommand<State> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Command for '{}'", (self.primary_name)(self))
    }
}

impl<State: Debug> NamedEntity for GenericCommand<State> {
    fn short_name(&self) -> String {
        (self.primary_name)(self)
    }

    fn long_name(&self) -> String {
        self.short_name()
    }

    fn description(&self) -> Option<String> {
        Some((self.help_text)(self))
    }

    fn matches(&self, name: &str) -> bool {
        if let Some(func) = &self.matches {
            func(self, name)
        } else {
            name.eq_ignore_ascii_case(&self.short_name())
                || self
                    .other_names
                    .iter()
                    .any(|n| n(self).eq_ignore_ascii_case(name))
        }
    }

    fn autocomplete_badness(&self, input: &str, matcher: fn(&str, &str) -> usize) -> usize {
        matcher(input, &self.short_name()).min(
            self.other_names
                .iter()
                // prefer primary matches
                .map(|name| 1 + matcher(input, &name(self)))
                .min()
                .unwrap_or(usize::MAX),
        )
    }

    fn secondary_names(&self) -> Vec<String> {
        self.other_names
            .iter()
            .map(|s| s(self).to_string())
            .collect_vec()
    }
}

impl<State: Debug + 'static> Display for GenericCommand<State> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        display_cmd(f, self)
    }
}

impl<State: Debug + 'static> CommandTrait<State> for GenericCommand<State> {
    fn func(&self) -> fn(&mut State, &mut Peekable<SplitWhitespace>, &str) -> Res<()> {
        (self.func)(self)
    }

    fn standard(&self) -> Standard {
        (self.standard)(self)
    }

    fn upcast_box(self: Box<Self>) -> Box<dyn NamedEntity> {
        self
    }

    fn upcast_ref(&self) -> &dyn NamedEntity {
        self
    }
}

pub type CommandList<State> = Vec<Box<dyn CommandTrait<State>>>;

macro_rules! command {
    ($state:ty, $primary:ident $(| $other:ident)*, $std:expr, $help:expr, $fun:expr $(, ->$subcmd:expr)?) => {
        {
            #[allow(unused_mut, unused_assignments)]
            let mut sub_commands = vec![];
            $(sub_commands = ($subcmd).into_iter().map(|x| x.upcast_box()).collect();)?
            let cmd = SimpleCommand::<$state> {
                primary_name: stringify!($primary),
                other_names: ArrayVec::from_iter([$(stringify!($other),)*]),
                standard: $std,
                func: $fun,
                help_text: $help,
                sub_commands,
            };
            Box::new(cmd)
        }
    };
}

macro_rules! ugi_command {
    ($primary:ident $(| $other:ident)*, $std:expr, $help:expr, $fun:expr $(, -> $subcmd:expr)?) => {
        command!(EngineUGI<B>, $primary $(| $other)*, $std, $help, $fun $(, ->$subcmd)?)
    }
}

#[expect(clippy::too_many_lines)]
pub fn ugi_commands<B: Board>() -> CommandList<EngineUGI<B>> {
    vec![
        // put time-critical commands at the top
        ugi_command!(
            go | g | search,
            All,
            "Start the search. Optionally takes a position and a mode such as `perft`",
            |ugi, words, _| { ugi.handle_go(Normal, words) },
            -> go_options::<B>()
        ),
        ugi_command!(
            stop,
            All,
            "Stop the current search. No effect if not searching",
            |ugi, _, _| {
                ugi.state.engine.send_stop(false);
                Ok(())
            }
        ),
        ugi_command!(
            position | pos | p,
            All,
            "Set the current position",
            |ugi, words, _| ugi.handle_position(words) // TODO: position command
        ),
        ugi_command!(
            ugi | uci | uai,
            All,
            "Starts UGI mode, ends interactive mode (can be re-enabled with `interactive`)",
            |ugi, _, proto| ugi.handle_ugi(proto)
        ),
        ugi_command!(
            ponderhit,
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
        ugi_command!(
            isready,
            All,
            "Queries if the engine is ready. The engine responds with 'readyok'",
            |ugi, _, _| {
                ugi.write_ugi("readyok");
                Ok(())
            }
        ),
        ugi_command!(setoption, All, "Sets an engine option", |ugi, words, _| ugi
            .handle_setoption(words)),
        ugi_command!(
            uginewgame | ucinewgame | uainewgame | clear,
            All,
            "Resets the internal engine state (doesn't reset engine options)",
            |ugi, _, _| {
                ugi.state.engine.send_forget()?;
                ugi.state.status = Run(NotStarted);
                Ok(())
            }
        ),
        ugi_command!(
            register,
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
        ugi_command!(
            flip,
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
        ugi_command!(quit, All, "Exits the program immediately", |ugi, _, _| ugi
            .quit(QuitProgram)),
        ugi_command!(
            quit_match | end_game | qm,
            Custom,
            "Quits the current match and, if `play` has been used, returns to the previous match",
            |ugi, _, _| ugi.quit(QuitMatch)
        ),
        ugi_command!(
            query | q,
            UgiNotUci,
            "Answer a query about the current match state",
            |ugi, words, _| ugi.handle_query(words),
            -> query_commands::<B>()
        ),
        ugi_command!(
            option | info,
            Custom,
            "Prints information about the current options. Optionally takes an option name",
            |ugi, words, _| {
                ugi.write_ugi(&ugi.write_option(words)?);
                Ok(())
            }
        ),
        ugi_command!(
            output | o,
            Custom,
            "Adds outputs. Use `remove (all)` to remove specified outputs",
            |ugi, words, _| ugi.handle_output(words)
        ),
        ugi_command!(
            print | show | s | display,
            Custom,
            "Display the specified / current position with specified / enabled outputs",
            |ugi, words, _| ugi.handle_print(words)
        ),
        ugi_command!(
            log,
            Custom,
            "Enables logging. Can optionally specify a file name, `stdout` / `stderr` or `off`",
            |ugi, words, _| ugi.handle_log(words)
        ),
        ugi_command!(
            debug | d,
            Custom,
            "Turns on logging, continue-on-error mode, and additional output. Use `off` to disable",
            |ugi, words, _| ugi.handle_debug(words)
        ),
        ugi_command!(
            interactive | i | human,
            Custom,
            "Starts interactive mode, undoes `ugi`. In this mode, errors aren't fatal",
            |ugi, _, _| {
                ugi.state.protocol = Interactive;
                Ok(())
            }
        ),
        ugi_command!(
            engine,
            Custom,
            "Sets the current engine, e.g. `caps-piston`, `gaps`, and optionally the game",
            |ugi, words, _| ugi.handle_engine(words)
        ),
        ugi_command!(
            set_eval,
            Custom,
            "Sets the eval for the current engine. Doesn't reset the internal engine state",
            |ugi, words, _| ugi.handle_set_eval(words)
        ),
        ugi_command!(
            play | game,
            Custom,
            "Starts a new match, possibly of a new game, optionally setting a new engine",
            |ugi, words, _| ugi.handle_play(words)
        ),
        ugi_command!(
            perft,
            Custom,
            "Internal movegen test on current / bench positions",
            |ugi, words, _| ugi.handle_go(Perft, words)
        ),
        ugi_command!(
            splitperft | sp,
            Custom,
            "Internal movegen test on current / bench positions",
            |ugi, words, _| ugi.handle_go(SplitPerft, words)
        ),
        ugi_command!(
            bench,
            Custom,
            "Internal search test on current / bench positions. Same arguments as `go`",
            |ugi, words, _| ugi.handle_go(Bench, words)
        ),
        ugi_command!(
            eval | e | static_eval,
            Custom,
            "Print the static eval (i.e., no search) of a position",
            |ugi, words, _| ugi.handle_eval_or_tt(true, words)
        ),
        ugi_command!(
            tt | tt_entry,
            Custom,
            "Print the TT entry for a position",
            |ugi, words, _| ugi.handle_eval_or_tt(false, words)
        ),
        ugi_command!(
            list,
            Custom,
            "Lists available options for a command",
            |ugi, words, _| ugi.handle_list(words)
        ),
        ugi_command!(help | h, Custom, "Prints a help message", |ugi, _, _| {
            ugi.print_help(); // TODO: allow help <command> to print a help message for a command
            Ok(())
        }),
    ]
}

#[derive(Debug)]
pub struct GoState<B: Board> {
    pub limit: SearchLimit,
    pub is_first: bool,
    pub multi_pv: usize,
    pub search_moves: Option<Vec<B::Move>>,
    pub cont: bool,
    pub reading_moves: bool,
    pub search_type: SearchType,
    pub complete: bool,
    pub board: B,
    pub move_overhead: Duration,
}

impl<B: Board> GoState<B> {
    pub fn new(ugi: &EngineUGI<B>, search_type: SearchType, move_overhead: Duration) -> Self {
        let limit = match search_type {
            Bench => SearchLimit::depth(ugi.state.engine.engine_info().default_bench_depth()),
            Perft | SplitPerft => SearchLimit::depth(ugi.state.board.default_perft_depth()),
            _ => SearchLimit::infinite(),
        };
        Self {
            // "infinite" is the identity element of the bounded semilattice of `go` options
            limit,
            is_first: ugi.state.board.active_player().is_first(),
            multi_pv: ugi.multi_pv,
            search_moves: None,
            cont: false,
            reading_moves: false,
            search_type,
            complete: false,
            board: ugi.state.board,
            move_overhead,
        }
    }
}

pub fn accept_depth(limit: &mut SearchLimit, words: &mut Peekable<SplitWhitespace>) {
    if let Some(word) = words.peek() {
        if let Ok(number) = parse_int_from_str(word, "depth") {
            limit.depth = Depth::new(number);
            _ = words.next();
        }
    }
}

macro_rules! go_command {
    ($primary:ident $( | $other:ident)*, $std:expr, $help:expr, $fun:expr $(, ->$subcmd:expr)?) => {
        command!(GoState<B>, $primary $(| $other)*, $std, $help, $fun $(, $subcmd)?)
    }
}

#[expect(clippy::too_many_lines)]
pub fn go_options<B: Board>() -> CommandList<GoState<B>> {
    vec![
        Box::new(GenericCommand::<GoState<B>> {
            primary_name: Box::new(|_| format!("{}time", B::Color::first().ascii_color_char())),
            other_names: vec![
                Box::new(|_| format!("{}t", B::Color::first().ascii_color_char())),
                Box::new(|_| "p1time".to_string()),
                Box::new(|_| "p1t".to_string()),
            ],
            help_text: Box::new(|_| format!("Remaining time in ms for {}", B::Color::first())),
            standard: Box::new(|_| All),
            func: Box::new(|_| {
                |go, words, _| {
                    let time = parse_duration_ms(words, "p1time")?;
                    // always parse the duration, even if it isn't relevant
                    if go.is_first {
                        go.limit.tc.remaining = time;
                    }
                    Ok(())
                }
            }),
            matches: None,
        }),
        Box::new(GenericCommand::<GoState<B>> {
            primary_name: Box::new(|_| format!("{}time", B::Color::second().ascii_color_char())),
            other_names: vec![
                Box::new(|_| format!("{}t", B::Color::second().ascii_color_char())),
                Box::new(|_| "p2time".to_string()),
                Box::new(|_| "p2t".to_string()),
            ],
            help_text: Box::new(|_| format!("Remaining time in ms for {}", B::Color::second())),
            standard: Box::new(|_| All),
            func: Box::new(|_| {
                |go, words, _| {
                    let time = parse_duration_ms(words, "p2time")?;
                    // always parse the duration, even if it isn't relevant
                    if !go.is_first {
                        go.limit.tc.remaining = time;
                    }
                    Ok(())
                }
            }),
            matches: None,
        }),
        Box::new(GenericCommand::<GoState<B>> {
            primary_name: Box::new(|_| format!("{}inc", B::Color::first().ascii_color_char())),
            other_names: vec![
                Box::new(|_| format!("{}i", B::Color::first().ascii_color_char())),
                Box::new(|_| "p1inc".to_string()),
            ],
            help_text: Box::new(|_| format!("Increment in ms for {}", B::Color::first())),
            standard: Box::new(|_| All),
            func: Box::new(|_| {
                |go, words, _| {
                    let increment = parse_duration_ms(words, "p1inc")?;
                    // always parse the duration, even if it isn't relevant
                    if go.is_first {
                        go.limit.tc.increment = increment;
                    }
                    Ok(())
                }
            }),
            matches: None,
        }),
        Box::new(GenericCommand::<GoState<B>> {
            primary_name: Box::new(|_| format!("{}inc", B::Color::second().ascii_color_char())),
            other_names: vec![
                Box::new(|_| format!("{}i", B::Color::second().ascii_color_char())),
                Box::new(|_| "p2inc".to_string()),
            ],
            help_text: Box::new(|_| format!("Increment in ms for {}", B::Color::second())),
            standard: Box::new(|_| All),
            func: Box::new(|_| {
                |go, words, _| {
                    let increment = parse_duration_ms(words, "p2inc")?;
                    // always parse the duration, even if it isn't relevant
                    if !go.is_first {
                        go.limit.tc.increment = increment;
                    }
                    Ok(())
                }
            }),
            matches: None,
        }),
        go_command!(
            movestogo | mtg,
            All,
            "Moves until the time control is reset",
            |opts, words, _| {
                opts.limit.tc.moves_to_go = Some(parse_int(words, "'movestogo' number")?);
                Ok(())
            }
        ),
        go_command!(
            depth | d,
            All,
            "Maximum search depth in plies (a.k.a. half-moves)",
            |opts, words, _| {
                opts.limit.depth = Depth::new(parse_int(words, "depth number")?);
                Ok(())
            }
        ),
        go_command!(
            nodes | n,
            All,
            "Maximum number of nodes to search",
            |opts, words, _| {
                opts.limit.nodes = NodesLimit::new(parse_int(words, "node count")?)
                    .ok_or_else(|| anyhow!("node count can't be zero"))?;
                Ok(())
            }
        ),
        go_command!(
            mate | m,
            All,
            "Maximum depth in moves until a mate has to be found",
            |opts, words, _| {
                let depth: usize = parse_int(words, "mate move count")?;
                opts.limit.mate = Depth::new(depth * 2); // 'mate' is given in moves instead of plies
                Ok(())
            }
        ),
        go_command!(
            movetime | mt,
            All,
            "Maximum time in ms",
            |opts, words, _| {
                opts.limit.fixed_time = parse_duration_ms(words, "time per move in milliseconds")?;
                opts.limit.fixed_time = opts
                    .limit
                    .fixed_time
                    .saturating_sub(opts.move_overhead)
                    .max(Duration::from_millis(1));
                Ok(())
            }
        ),
        go_command!(
            infinite | inf,
            All,
            "Search until receiving `stop`, the default mode",
            |opts, _, _| {
                opts.limit = SearchLimit::infinite();
                Ok(())
            }
        ),
        go_command!(
            searchmoves | sm,
            All,
            "Only consider the specified moves",
            |opts, _, _| {
                opts.reading_moves = true;
                opts.search_moves = Some(vec![]);
                opts.cont = true;
                Ok(())
            }
        ),
        go_command!(
            multipv | mpv,
            All,
            "Find the k best moves",
            |opts, words, _| {
                opts.multi_pv = parse_int(words, "multipv")?;
                Ok(())
            }
        ),
        go_command!(
            ponder,
            All,
            "Search on the opponent's time",
            |opts, _, _| {
                opts.search_type = Ponder;
                Ok(())
            }
        ),
        go_command!(
            perft | pt,
            Custom,
            "Movegen test: Make all legal moves up to a depth",
            |opts, words, _| {
                opts.search_type = Perft;
                accept_depth(&mut opts.limit, words);
                Ok(())
            }
        ),
        go_command!(
            splitperft | sp,
            Custom,
            "Movegen test: Print perft number for each legal move",
            |opts, words, _| {
                opts.search_type = SplitPerft;
                accept_depth(&mut opts.limit, words);
                Ok(())
            }
        ),
        go_command!(
            bench | b,
            Custom,
            "Search test: Print info about nodes, nps, and hash of search",
            |opts, words, _| {
                opts.search_type = Bench;
                accept_depth(&mut opts.limit, words);
                Ok(())
            }
        ),
        go_command!(
            complete | all,
            Custom,
            "Run bench / perft on all bench positions",
            |opts, _, _| {
                opts.complete = true;
                Ok(())
            }
        ),
        // TODO: Maybe there's a way to reuse commands?
        go_command!(
            position | pos | p,
            Custom,
            "Search from a custom position",
            |opts, words, _| {
                opts.board = load_ugi_position(words, &opts.board)?;
                Ok(())
            }
        ),
        // TODO: Handle moves for searchmoves. Maybe not as command
        // Box::new(GenericCommand::<GoState<B>> {
        //     primary_name: Box::new(|_| "move".to_string()),
        //     other_names: vec![],
        //     help_text: Box::new(|_| "Input a whitespace-separated list of moves".to_string()),
        //     standard: Box::new(|_| Custom),
        //     func: Box::new(|_| {
        //         |go, _, word| {
        //             debug_assert!(go.reading_moves);
        //             let mov = B::Move::from_compact_text(word, &go.board).map_err(|err| {
        //                 anyhow!("{err}. '{}' is not a valid 'go' option.", word.bold())
        //             })?;
        //             go.search_moves.as_mut().unwrap().push(mov);
        //             go.cont = true;
        //             Ok(())
        //         }
        //     }),
        //     matches: None, /*Some(Box::new(|_, go, _word| go.reading_moves))*/
        // }),
    ]
}

pub fn query_commands<B: Board>() -> CommandList<EngineUGI<B>> {
    vec![
        ugi_command!(gameover, UgiNotUci, "Is the game over?", |ugi, _, _| {
            ugi.output()
                .write_response(&matches!(ugi.state.status, Run(Ongoing)).to_string());
            Ok(())
        }),
        Box::new(GenericCommand::<EngineUGI<B>> {
            primary_name: Box::new(|_| "p1turn".to_string()),
            other_names: vec![Box::new(|_| {
                format!("{}turn", B::Color::first().ascii_color_char())
            })],
            help_text: Box::new(|_| "Is it the first player's turn?".to_string()),
            standard: Box::new(|_| UgiNotUci),
            func: Box::new(|_| {
                |ugi, _, _| {
                    ugi.output()
                        .write_response(&(ugi.state.board.active_player().is_first()).to_string());
                    Ok(())
                }
            }),
            matches: None,
        }),
        Box::new(GenericCommand::<EngineUGI<B>> {
            primary_name: Box::new(|_| "p2turn".to_string()),
            other_names: vec![Box::new(|_| {
                format!("{}turn", B::Color::second().ascii_color_char())
            })],
            help_text: Box::new(|_| "Is it the second player's turn?".to_string()),
            standard: Box::new(|_| UgiNotUci),
            func: Box::new(|_| {
                |ugi, _, _| {
                    ugi.output()
                        .write_response(&(!ugi.state.board.active_player().is_first()).to_string());
                    Ok(())
                }
            }),
            matches: None,
        }),
        ugi_command!(
            result | res,
            UgiNotUci,
            "The result of the current match",
            |ugi, _, _| {
                let response = match &ugi.state.status {
                    Run(Over(res)) => match res.result {
                        GameResult::P1Win => "p1win",
                        GameResult::P2Win => "p2win",
                        GameResult::Draw => "draw",
                        GameResult::Aborted => "aborted",
                    },
                    _ => "none",
                };
                ugi.output().write_response(response);
                Ok(())
            }
        ),
        ugi_command!(game | g, Custom, "The current game", |ugi, _, _| {
            let board = ugi.state.board;
            ugi.write_ugi(&format!(
                "{0}\n{1}",
                &board.long_name(),
                board.description().unwrap_or_default()
            ));
            Ok(())
        }),
        ugi_command!(
            engine | e | name,
            Custom,
            "The name of the engine",
            |ugi, _, _| {
                let info = ugi.state.engine.engine_info();
                let name = info.long_name();
                let description = info.description().unwrap_or_default();
                drop(info);
                ugi.write_ugi(&format!("{name}\n{description}",));
                Ok(())
            }
        ),
    ]
}

#[derive(Debug, Clone)]
pub struct CommandAutocomplete<B: Board> {
    // Rc because the Autocomplete trait requires DynClone and invokes `clone` on every prompt call
    pub list: Rc<CommandList<EngineUGI<B>>>,
}

impl<B: Board> CommandAutocomplete<B> {
    pub fn new(list: CommandList<EngineUGI<B>>) -> Self {
        Self {
            list: Rc::new(list),
        }
    }
}

fn distance(input: &str, name: &str) -> usize {
    if input.eq_ignore_ascii_case(name) {
        0
    } else {
        let lowercase_name = name.to_lowercase();
        let prefix = &lowercase_name.as_bytes()[..input.len().min(lowercase_name.len())];
        2 + edit_distance(input, from_utf8(&prefix).unwrap_or(name))
    }
}

fn completions(
    node: &dyn NamedEntity,
    current: &str,
    mut rest: Peekable<SplitWhitespace>,
    to_complete: &str,
) -> Vec<(usize, Completion)> {
    let mut res = vec![];
    for child in node.sub_entities_completion() {
        res.push((
            child.autocomplete_badness(to_complete, distance),
            Completion {
                name: child.short_name(),
                text: completion_text(&**child, to_complete),
            },
        ));
        if child.matches(current) {
            if let Some(next) = rest.next() {
                res.append(&mut completions(&**child, next, rest.clone(), to_complete));
            }
        }
    }
    res
}

fn underline_match(name: &str, word: &str) -> String {
    if name == word {
        format!("{}", name.underline())
    } else {
        name.to_string()
    }
}

fn completion_text(n: &dyn NamedEntity, word: &str) -> String {
    use std::fmt::Write;
    let name = n.short_name();
    let mut res = format!("{}", underline_match(&name, word).bold());
    for name in n.secondary_names() {
        write!(&mut res, " | {}", underline_match(&name, word)).unwrap();
    }
    if let Some(desc) = n.description() {
        format!("{res}:  {}", desc)
    } else {
        res
    }
}

#[derive(Eq, PartialEq)]
struct Completion {
    name: String,
    text: String,
}

fn suggestions<B: Board>(
    autocomplete: &mut CommandAutocomplete<B>,
    input: &str,
) -> Vec<Completion> {
    let mut words = input.split_whitespace().peekable();
    let Some(cmd_name) = words.next() else {
        return vec![];
    };
    let to_complete = if input.ends_with(|s: char| s.is_whitespace()) {
        ""
    } else {
        input.split_whitespace().last().unwrap()
    };
    let complete_command = words.peek().is_none() && to_complete != "";

    let mut res = vec![];
    for cmd in autocomplete.list.iter() {
        if complete_command {
            res.push((
                cmd.autocomplete_badness(to_complete, distance),
                Completion {
                    name: cmd.short_name(),
                    text: completion_text(cmd.upcast_ref(), cmd_name),
                },
            ))
        } else if cmd.matches(cmd_name) {
            let mut new = completions(cmd.upcast_ref(), cmd_name, words.clone(), to_complete);
            res.append(&mut new);
        }
    }
    res.sort_by_key(|(val, _name)| *val);
    if let Some(min) = res.first().map(|(val, _name)| *val) {
        res.into_iter()
            .dedup()
            .take_while(|(val, _text)| *val == min)
            .map(|(_val, text)| text)
            .collect()
    } else {
        vec![]
    }
}

impl<B: Board> Autocomplete for CommandAutocomplete<B> {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, CustomUserError> {
        Ok(suggestions(self, input)
            .into_iter()
            .map(|c| c.text)
            .collect())
    }

    fn get_completion(
        &mut self,
        input: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<Replacement, CustomUserError> {
        let replacement = {
            let suggestions = suggestions(self, input);
            if let Some(suggestion) = &highlighted_suggestion {
                suggestions
                    .into_iter()
                    .find(|s| *s.text == *suggestion)
                    .map(|s| s.name)
            } else if suggestions.len() == 1 {
                Some(suggestions[0].name.clone())
            } else {
                None
            }
        };
        if let Some(r) = replacement {
            let mut keep_words = input.split_whitespace();
            if !input.ends_with(|c: char| c.is_whitespace()) {
                keep_words = keep_words.dropping_back(1);
            }
            let res: String = keep_words.chain(once(r.as_str())).join(" ");
            Ok(Some(res))
        } else {
            Ok(None)
        }
    }
}
