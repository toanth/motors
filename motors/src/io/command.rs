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
use crate::search::{
    AbstractEvalBuilder, AbstractSearcherBuilder, EngineInfo, EvalList, SearcherList,
};
use edit_distance::edit_distance;
use gears::arrayvec::ArrayVec;
use gears::cli::Game;
use gears::crossterm::style::Stylize;
use gears::games::{Color, OutputList, ZobristHistory};
use gears::general::board::Strictness::Relaxed;
use gears::general::board::{Board, Strictness};
use gears::general::common::anyhow::anyhow;
use gears::general::common::{
    parse_duration_ms, parse_int, parse_int_from_str, tokens, Name, NamedEntity, Res, Tokens,
};
use gears::general::move_list::MoveList;
use gears::general::moves::{ExtendedFormat, Move};
use gears::output::Message::Warning;
use gears::output::OutputBuilder;
use gears::search::{Depth, NodesLimit, SearchLimit};
use gears::ugi::{load_ugi_position, parse_ugi_position_part, EngineOptionName};
use gears::GameResult;
use gears::MatchStatus::{NotStarted, Ongoing, Over};
use gears::Quitting::{QuitMatch, QuitProgram};
use inquire::autocompletion::Replacement;
use inquire::{Autocomplete, CustomUserError};
use itertools::Itertools;
use rand::prelude::IndexedRandom;
use rand::{thread_rng, Rng};
use std::fmt::{Debug, Display, Formatter};
use std::iter::once;
use std::rc::Rc;
use std::str::from_utf8;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use strum::IntoEnumIterator;

fn add<T>(mut a: Vec<T>, mut b: Vec<T>) -> Vec<T> {
    a.append(&mut b);
    a
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Standard {
    All,
    UgiNotUci,
    Custom,
}

pub trait CommandState: Debug {
    type B: Board;
}

impl<B: Board> CommandState for EngineUGI<B> {
    type B = B;
}

impl<B: Board> CommandState for B {
    type B = B;
}

#[allow(type_alias_bounds)]
pub type SubCommandList<B: Board> = Vec<Box<dyn AbstractCommand<B>>>;

/// This is used for command autocompletion, where there is no need to actually execute the command.
/// This means that the State doesn't need to be known.
pub trait AbstractCommand<B: Board>: NamedEntity + Display {
    fn standard(&self) -> Standard;

    fn sub_commands(&self, state: ACState<B>) -> SubCommandList<B>;

    fn change_autocomplete_state(&self, state: ACState<B>) -> ACState<B>;

    fn autocomplete_recurse(&self) -> bool;

    fn set_autocompletions(&mut self, func: SubCommandsFn<B>);

    fn secondary_names(&self) -> Vec<String>;
}

pub trait CommandTrait<State: CommandState>: AbstractCommand<State::B> {
    fn func(&self) -> fn(&mut State, remaining_input: &mut Tokens, _cmd: &str) -> Res<()>;

    // TODO: The upcast methods should be unnecessary now
    fn upcast_box(self: Box<Self>) -> Box<dyn AbstractCommand<State::B>>;

    fn upcast_ref(&self) -> &dyn AbstractCommand<State::B>;
}

// TODO: Needed?
fn display_cmd<S: CommandState>(
    f: &mut Formatter<'_>,
    cmd: &dyn CommandTrait<S>,
) -> std::fmt::Result {
    if let Some(desc) = cmd.description() {
        write!(f, "{}: {desc}.", cmd.short_name().bold())
    } else {
        write!(f, "{}", cmd.short_name().bold())
    }
}

struct AutoCompleteFunc<B: Board>(pub Box<dyn Fn(ACState<B>) -> ACState<B>>);

impl<B: Board> Debug for AutoCompleteFunc<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "<autocomplete>")
    }
}

impl<B: Board> Default for AutoCompleteFunc<B> {
    fn default() -> Self {
        Self(Box::new(|x| x))
    }
}

pub struct SubCommandsFn<B: Board>(Box<dyn Fn(ACState<B>) -> SubCommandList<B>>);

impl<B: Board> Debug for SubCommandsFn<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "<subcommands>")
    }
}

impl<B: Board> Default for SubCommandsFn<B> {
    fn default() -> Self {
        Self(Box::new(|_state| vec![]))
    }
}

impl<B: Board> SubCommandsFn<B> {
    pub fn new<
        S: CommandState<B = B>,
        C: CommandTrait<S> + ?Sized,
        F: Fn(ACState<B>) -> Vec<Box<C>> + 'static,
    >(
        cmd: F,
    ) -> Self {
        Self(Box::new(move |state: ACState<B>| {
            cmd(state)
                .into_iter()
                .map(|state| state.upcast_box())
                .collect_vec()
        }))
    }
}

#[derive(Debug)]
pub struct Command<State: CommandState> {
    pub primary_name: String,
    pub other_names: ArrayVec<String, 4>,
    pub help_text: String,
    pub standard: Standard,
    pub autocomplete_recurse: bool,
    pub func: fn(&mut State, remaining_input: &mut Tokens, _cmd: &str) -> Res<()>,
    change_ac_state: AutoCompleteFunc<State::B>,
    sub_commands: SubCommandsFn<State::B>,
}

impl<State: CommandState> NamedEntity for Command<State> {
    fn short_name(&self) -> String {
        self.primary_name.clone()
    }

    fn long_name(&self) -> String {
        self.short_name()
    }

    fn description(&self) -> Option<String> {
        Some(self.help_text.clone())
    }

    fn matches(&self, name: &str) -> bool {
        name.eq_ignore_ascii_case(&self.primary_name)
            || self
                .other_names
                .iter()
                .any(|n| n.eq_ignore_ascii_case(name))
    }

    fn autocomplete_badness(&self, input: &str, matcher: fn(&str, &str) -> usize) -> usize {
        matcher(input, &self.primary_name).min(
            self.other_names
                .iter()
                // prefer primary matches
                .map(|name| 1 + matcher(input, name))
                .min()
                .unwrap_or(usize::MAX),
        )
    }
}

impl<State: CommandState + 'static> Display for Command<State> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        display_cmd(f, self)
    }
}

impl<State: CommandState + 'static> AbstractCommand<State::B> for Command<State> {
    fn standard(&self) -> Standard {
        self.standard
    }

    fn sub_commands(&self, state: ACState<State::B>) -> SubCommandList<State::B> {
        self.sub_commands.0(state)
    }

    fn change_autocomplete_state(&self, state: ACState<State::B>) -> ACState<State::B> {
        self.change_ac_state.0(state)
    }

    fn autocomplete_recurse(&self) -> bool {
        self.autocomplete_recurse
    }

    fn set_autocompletions(&mut self, func: SubCommandsFn<State::B>) {
        self.sub_commands = func;
    }

    fn secondary_names(&self) -> Vec<String> {
        self.other_names.iter().cloned().collect_vec()
    }
}

impl<State: CommandState + 'static> CommandTrait<State> for Command<State> {
    fn func(&self) -> fn(&mut State, &mut Tokens, &str) -> Res<()> {
        self.func
    }

    fn upcast_box(self: Box<Self>) -> Box<dyn AbstractCommand<State::B>> {
        self
    }

    fn upcast_ref(&self) -> &dyn AbstractCommand<State::B> {
        self
    }
}

pub type CommandList<State> = Vec<Box<dyn CommandTrait<State>>>;

macro_rules! command {
    ($State:ty, $primary:ident $(| $other:ident)*, $std:expr, $help:expr , $fun:expr $(, ->$subcmd:expr)? $(, [] $autocomplete_fn:expr)? $(, recurse=$recurse:expr)?) => {
        {
            #[allow(unused_mut, unused_assignments)]
            let mut sub_commands = SubCommandsFn::default();
            $(
                sub_commands.0 = Box::new(|this| ($subcmd)(this)
                    .into_iter()
                    .map(|x| x.upcast_box())
                    .collect());
            )?

            #[allow(unused_mut, unused_assignments)]
            let mut autocomplete_func = AutoCompleteFunc::default();
            $(
                autocomplete_func.0 = Box::new($autocomplete_fn);
            )?

            #[allow(unused_mut, unused_assignments)]
            let mut autocomplete_recurse = false;
            $(
                autocomplete_recurse = $recurse;
            )?

            let cmd = Command::<$State> {
                primary_name: stringify!($primary).to_string(),
                other_names: ArrayVec::from_iter([$(stringify!($other).to_string(),)*]),
                standard: $std,
                func: $fun,
                help_text: $help.to_string(),
                sub_commands,
                change_ac_state:autocomplete_func,
                autocomplete_recurse,
            };
            Box::new(cmd)
        }
    };
}

macro_rules! ugi_command {
    ($primary:ident $(| $other:ident)*, $std:expr, $help:expr, $fun:expr $(, -> $subcmd:expr)? $(, [] $autocomplete_fn:expr)? $(, recurse=$recurse:expr)?) => {
        command!(EngineUGI<B>, $primary $(| $other)*, $std, $help, $fun $(, ->$subcmd)? $(, [] $autocomplete_fn)? $(, recurse=$recurse)?)
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
            -> |_| go_options::<B>(),
            recurse = true
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
            |ugi, words, _| ugi.handle_position(words),
            -> |_| position_options::<B>(false)
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
            |ugi, _, cmd| {
                let mut go_state = GoState::new(ugi, Normal, ugi.move_overhead);
                go_state.limit = ugi.state.ponder_limit.ok_or_else(|| {
                    anyhow!(
                        "The engine received a '{}' command but wasn't pondering",
                        cmd.bold()
                    )
                })?;
                ugi.start_search(go_state)
            }
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
        ugi_command!(
            setoption | so,
            All,
            "Sets an engine option",
            |ugi, words, _| ugi.handle_setoption(words),
            -> |state: ACState<B>| options_options::<B, true>(state.info.clone(), true)
        ),
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
                Ok(())
            }
        ),
        ugi_command!(
            flip,
            Custom,
            "Flips the side to move, unless this results in an illegal position",
            |ugi, _, _| {
                // TODO: Update move history by calling a proper method of ugi
                ugi.state.board = ugi.state.board.make_nullmove().ok_or(anyhow!(
                    "Could not flip the side to move (board: '{}'",
                    ugi.state.board.as_fen().bold()
                ))?;
                ugi.print_board();
                Ok(())
            }
        ),
        ugi_command!(quit, All, "Exits the program immediately", |ugi, _, _| {
            if cfg!(feature = "fuzzing") {
                eprintln!("Fuzzing is enabled, ignoring 'quit' command");
                return Ok(());
            }
            ugi.quit(QuitProgram)
        }),
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
            -> |_| query_options::<B>()
        ),
        ugi_command!(
            option | info,
            Custom,
            "Prints information about the current options. Optionally takes an option name",
            |ugi, words, _| {
                ugi.write_ugi(&ugi.write_option(words)?);
                Ok(())
            },
            -> |state: ACState<B>| options_options::<B, false>(state.info.clone(), true)
        ),
        ugi_command!(
            output | o,
            Custom,
            "Sets outputs. Use `remove (all)` to remove specified outputs, 'add' to use multiple",
            |ugi, words, _| ugi.handle_output(words),
            -> |state: ACState<B>| select_command::<B, dyn OutputBuilder<B>>(state.outputs.as_slice())
        ),
        ugi_command!(
            print | show | s | display,
            Custom,
            "Display the specified / current position with specified / enabled outputs",
            |ugi, words, _| ugi.handle_print(words),
            -> |state: ACState<B>| add(position_options::<B>(true), select_command::<B, dyn OutputBuilder<B>>(state.outputs.as_slice())),
            recurse=true
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
                ugi.output().pretty = true;
                Ok(())
            }
        ),
        ugi_command!(
            engine,
            Custom,
            "Sets the current engine, e.g. `caps-piston`, `gaps`, and optionally the game",
            |ugi, words, _| ugi.handle_engine(words),
            -> |state: ACState<B>| select_command::<B, dyn AbstractSearcherBuilder<B>>(state.searchers.as_slice())
        ),
        ugi_command!(
            set_eval | se,
            Custom,
            "Sets the eval for the current engine. Doesn't reset the internal engine state",
            |ugi, words, _| ugi.handle_set_eval(words),
            -> |state: ACState<B>| select_command::<B, dyn AbstractEvalBuilder<B>>(state.evals.as_slice())
        ),
        ugi_command!(
            play | game,
            Custom,
            "Starts a new match, possibly of a new game, optionally setting a new engine and position",
            |ugi, words, _| ugi.handle_play(words),
            -> |_| select_command::<B, Game>(&Game::iter().map(Box::new).collect_vec())
        ),
        ugi_command!(
            perft,
            Custom,
            "Internal movegen test on current / bench positions",
            |ugi, words, _| ugi.handle_go(Perft, words),
            -> |_state: ACState<B>| position_options::<B>(true),
            recurse=true
        ),
        ugi_command!(
            splitperft | sp,
            Custom,
            "Internal movegen test on current / bench positions",
            |ugi, words, _| ugi.handle_go(SplitPerft, words),
            -> |_| position_options::<B>(true),
            recurse=true
        ),
        ugi_command!(
            bench,
            Custom,
            "Internal search test on current / bench positions. Same arguments as `go`",
            |ugi, words, _| ugi.handle_go(Bench, words),
            -> |_| go_options::<B>(),
            recurse=true
        ),
        ugi_command!(
            eval | e | static_eval,
            Custom,
            "Print the static eval (i.e., no search) of a position",
            |ugi, words, _| ugi.handle_eval_or_tt(true, words),
            -> |_| position_options::<B>(true),
            recurse=true
        ),
        ugi_command!(
            tt | tt_entry,
            Custom,
            "Print the TT entry for a position",
            |ugi, words, _| ugi.handle_eval_or_tt(false, words),
            -> |_| position_options::<B>(true),
            recurse=true
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
    pub threads: Option<usize>,
    pub search_moves: Option<Vec<B::Move>>,
    pub cont: bool,
    pub reading_moves: bool,
    pub search_type: SearchType,
    pub complete: bool,
    pub strictness: Strictness,
    pub board: B,
    pub board_hist: ZobristHistory<B>,
    pub move_overhead: Duration,
}

impl<B: Board> CommandState for GoState<B> {
    type B = B;
}

impl<B: Board> GoState<B> {
    pub fn new(ugi: &EngineUGI<B>, search_type: SearchType, move_overhead: Duration) -> Self {
        let limit = match search_type {
            Bench => SearchLimit::depth(ugi.state.engine.get_engine_info().default_bench_depth()),
            Perft | SplitPerft => SearchLimit::depth(ugi.state.board.default_perft_depth()),
            _ => SearchLimit::infinite(),
        };
        Self {
            // "infinite" is the identity element of the bounded semilattice of `go` options
            limit,
            is_first: ugi.state.board.active_player().is_first(),
            multi_pv: ugi.multi_pv,
            threads: None,
            search_moves: None,
            cont: false,
            reading_moves: false,
            search_type,
            complete: false,
            strictness: ugi.strictness,
            board: ugi.state.board,
            board_hist: ugi.state.board_hist.clone(),
            move_overhead,
        }
    }
}

pub fn accept_depth(limit: &mut SearchLimit, words: &mut Tokens) -> Res<()> {
    if let Some(word) = words.peek() {
        if let Ok(number) = parse_int_from_str(word, "depth") {
            limit.depth = Depth::try_new(number)?;
            _ = words.next();
        }
    }
    Ok(())
}

macro_rules! go_command {
    ($primary:ident $( | $other:ident)*, $std:expr, $help:expr, $fun:expr $(, ->$subcmd:expr)?) => {
        command!(GoState<B>, $primary $(| $other)*, $std, $help, $fun $(, ->$subcmd)?)
    }
}

#[expect(clippy::too_many_lines)]
pub fn go_options<B: Board>() -> CommandList<GoState<B>> {
    let mut res: CommandList<GoState<B>> = vec![
        Box::new(Command::<GoState<B>> {
            primary_name: format!("{}time", B::Color::first().ascii_color_char()),
            other_names: ArrayVec::from_iter([
                format!("{}t", B::Color::first().ascii_color_char()),
                "p1time".to_string(),
                "p1t".to_string(),
            ]),
            help_text: format!("Remaining time in ms for {}", B::Color::first()),
            standard: All,
            autocomplete_recurse: false,
            func: |go, words, _| {
                let time = parse_duration_ms(words, "p1time")?;
                // always parse the duration, even if it isn't relevant
                if go.is_first {
                    go.limit.tc.remaining = time;
                }
                Ok(())
            },
            change_ac_state: AutoCompleteFunc::default(),
            sub_commands: SubCommandsFn::default(),
        }),
        Box::new(Command::<GoState<B>> {
            primary_name: format!("{}time", B::Color::second().ascii_color_char()),
            other_names: ArrayVec::from_iter([
                format!("{}t", B::Color::second().ascii_color_char()),
                "p2time".to_string(),
                "p2t".to_string(),
            ]),
            help_text: format!("Remaining time in ms for {}", B::Color::second()),
            standard: All,
            autocomplete_recurse: false,
            func: |go, words, _| {
                let time = parse_duration_ms(words, "p2time")?;
                // always parse the duration, even if it isn't relevant
                if !go.is_first {
                    go.limit.tc.remaining = time;
                }
                Ok(())
            },

            change_ac_state: AutoCompleteFunc::default(),
            sub_commands: SubCommandsFn::default(),
        }),
        Box::new(Command::<GoState<B>> {
            primary_name: format!("{}inc", B::Color::first().ascii_color_char()),
            other_names: ArrayVec::from_iter([
                format!("{}i", B::Color::first().ascii_color_char()),
                "p1inc".to_string(),
            ]),
            help_text: format!("Increment in ms for {}", B::Color::first()),
            standard: All,
            autocomplete_recurse: false,
            func: |go, words, _| {
                let increment = parse_duration_ms(words, "p1inc")?;
                // always parse the duration, even if it isn't relevant
                if go.is_first {
                    go.limit.tc.increment = increment;
                }
                Ok(())
            },

            change_ac_state: AutoCompleteFunc::default(),
            sub_commands: SubCommandsFn::default(),
        }),
        Box::new(Command::<GoState<B>> {
            primary_name: format!("{}inc", B::Color::second().ascii_color_char()),
            other_names: ArrayVec::from_iter([
                format!("{}i", B::Color::second().ascii_color_char()),
                "p2inc".to_string(),
            ]),
            help_text: format!("Increment in ms for {}", B::Color::second()),
            standard: All,
            autocomplete_recurse: false,
            func: |go, words, _| {
                let increment = parse_duration_ms(words, "p2inc")?;
                // always parse the duration, even if it isn't relevant
                if !go.is_first {
                    go.limit.tc.increment = increment;
                }
                Ok(())
            },
            change_ac_state: AutoCompleteFunc::default(),
            sub_commands: SubCommandsFn::default(),
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
                opts.limit.depth = Depth::try_new(parse_int(words, "depth number")?)?;
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
                let depth: isize = parse_int(words, "mate move count")?;
                opts.limit.mate = Depth::try_new(depth * 2)?; // 'mate' is given in moves instead of plies
                Ok(())
            }
        ),
        go_command!(
            movetime | mt | time,
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
            Custom,
            "Find the n best moves, temporarily overwriting the 'multipv' engine option",
            |opts, words, _| {
                opts.multi_pv = parse_int(words, "multipv")?;
                Ok(())
            }
        ),
        go_command!(
            threads | t,
            Custom,
            "Search with n threads in parallel, temporarily overwriting the 'threads' engine option",
            |opts, words, _| {
                opts.threads = Some(parse_int(words, "threads")?);
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
                accept_depth(&mut opts.limit, words)?;
                Ok(())
            }
        ),
        go_command!(
            splitperft | sp,
            Custom,
            "Movegen test: Print perft number for each legal move",
            |opts, words, _| {
                opts.search_type = SplitPerft;
                accept_depth(&mut opts.limit, words)?;
                Ok(())
            }
        ),
        go_command!(
            bench | b,
            Custom,
            "Search test: Print info about nodes, nps, and hash of search",
            |opts, words, _| {
                opts.search_type = Bench;
                accept_depth(&mut opts.limit, words)?;
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
    ];
    for (cmd, cmd_cpy) in position_options::<B>(true)
        .into_iter()
        .zip(position_options::<B>(true))
    {
        let cmd = Command::<GoState<B>> {
            primary_name: cmd.short_name(),
            other_names: cmd.secondary_names().into_iter().collect(),
            help_text: cmd.description().unwrap(),
            standard: Custom,
            autocomplete_recurse: false,
            func: |opts, words, first_word| {
                opts.board =
                    load_ugi_position(first_word, words, true, opts.strictness, &opts.board)?;
                Ok(())
            },
            change_ac_state: AutoCompleteFunc(Box::new(move |state: ACState<B>| {
                cmd.change_autocomplete_state(state)
            })),
            sub_commands: SubCommandsFn(Box::new(move |state: ACState<B>| {
                cmd_cpy.sub_commands(state)
            })),
        };
        res.push(Box::new(cmd));
    }
    res
}

pub fn query_options<B: Board>() -> CommandList<EngineUGI<B>> {
    vec![
        ugi_command!(gameover, UgiNotUci, "Is the game over?", |ugi, _, _| {
            ugi.output()
                .write_response(&matches!(ugi.state.status, Run(Ongoing)).to_string());
            Ok(())
        }),
        Box::new(Command::<EngineUGI<B>> {
            primary_name: "p1turn".to_string(),
            other_names: ArrayVec::from_iter([format!(
                "{}turn",
                B::Color::first().ascii_color_char()
            )]),
            help_text: "Is it the first player's turn?".to_string(),
            standard: UgiNotUci,
            autocomplete_recurse: false,
            func: |ugi, _, _| {
                ugi.output()
                    .write_response(&(ugi.state.board.active_player().is_first()).to_string());
                Ok(())
            },

            change_ac_state: AutoCompleteFunc::default(),
            sub_commands: SubCommandsFn::default(),
        }),
        Box::new(Command::<EngineUGI<B>> {
            primary_name: "p2turn".to_string(),
            other_names: ArrayVec::from_iter([format!(
                "{}turn",
                B::Color::second().ascii_color_char()
            )]),
            help_text: "Is it the second player's turn?".to_string(),
            standard: UgiNotUci,
            autocomplete_recurse: false,
            func: |ugi, _, _| {
                ugi.output()
                    .write_response(&(!ugi.state.board.active_player().is_first()).to_string());
                Ok(())
            },

            change_ac_state: AutoCompleteFunc::default(),
            sub_commands: SubCommandsFn::default(),
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
                let info = ugi.state.engine.get_engine_info();
                let name = info.long_name();
                let description = info.description().unwrap_or_default();
                drop(info);
                ugi.write_ugi(&format!("{name}\n{description}",));
                Ok(())
            }
        ),
    ]
}

macro_rules! misc_command {
    ($primary:ident $( | $other:ident)*, $std:expr, $help:expr $(, = $pos:expr)?, $func:expr $(, ->$subcmd:expr)? $(, [] $autocomplete_fn:expr)?) => {
        command!(B, $primary $(| $other)*, $std, $help $(, = $pos)?, $func $(, ->$subcmd)? $(, [] $autocomplete_fn)?)
    }
}

macro_rules! pos_command {
    ($primary:ident $( | $other:ident)*, $std:expr, $help:expr $(, = $pos:expr)?, $func:expr $(, ->$subcmd:expr)? $(, [] $autocomplete_fn:expr)?) => {
        command!(B, $primary $(| $other)*, $std, $help $(, = $pos)?, $func $(, ->$subcmd)? $(, [] $autocomplete_fn)?)
    }
}

pub fn position_options<B: Board>(accept_pos_word: bool) -> CommandList<B> {
    let mut res: CommandList<B> = vec![
        pos_command!(
            fen | f,
            All,
            "Load a positions from a FEN",
            |pos, words, _| {
                *pos = parse_ugi_position_part("fen", words, false, pos, Relaxed)?;
                Ok(())
            },
            -> |state: ACState<B>| moves_options(state.pos, true)
        ),
        pos_command!(
            startpos | s,
            All,
            "Load the starting position",
            |pos, _, _| {
                *pos = B::startpos();
                Ok(())
            },
            -> |state: ACState<B>| moves_options(state.pos, true),
            [] |mut state: ACState<B>| {
                state.pos = B::default();
                state
            }
        ),
        pos_command!(
            current | c,
            Custom,
            "Current position, useful in combination with 'moves'",
            |_, _, _| Ok(()),
            -> |state: ACState<B>| moves_options(state.pos, true)
        ),
    ];
    if accept_pos_word {
        res.push(pos_command!(
            position | pos | p,
            Custom,
            "Followed by `fen <fen>` or a position name",
            |_, _, _| Ok(()),
            -> |_| position_options::<B>(false)
        ))
    }
    for p in B::name_to_pos_map() {
        let func = p.val;
        let c = Box::new(Command {
            primary_name: p.short_name(),
            other_names: Default::default(),
            help_text: p.description().unwrap_or(format!(
                "Load a custom position called '{}'",
                p.short_name()
            )),
            standard: Custom,
            autocomplete_recurse: false,
            func: |pos, _, name| {
                *pos = B::from_name(name)?;
                Ok(())
            },
            change_ac_state: AutoCompleteFunc(Box::new(move |mut state| {
                state.pos = func();
                state
            })),
            sub_commands: SubCommandsFn::new(|state: ACState<B>| moves_options(state.pos, true)),
        });
        res.push(c);
    }
    res
}

pub fn moves_options<B: Board>(pos: B, allow_moves_word: bool) -> CommandList<B> {
    let mut res: CommandList<B> = vec![];
    if allow_moves_word {
        res.push(pos_command!(
            moves | m,
            All,
            "Apply moves to the specified position",
            |_, _, _| Ok(()),
            -> |state: ACState<B>| moves_options(state.pos, false)
        ));
    }
    for mov in pos.legal_moves_slow().iter_moves() {
        let primary_name = mov.to_string();
        let mut other_names = ArrayVec::default();
        let extended = mov.to_extended_text(&pos, ExtendedFormat::Standard);
        if extended != primary_name {
            other_names.push(extended);
        }
        let the_move = *mov;
        let cmd = Command {
            primary_name,
            other_names,
            help_text: format!("Play move '{}'", mov.to_string().bold()),
            standard: All,
            autocomplete_recurse: false,
            func: |_, _, _| Ok(()),
            change_ac_state: AutoCompleteFunc(Box::new(move |mut state: ACState<B>| {
                state.pos = state.pos.make_move(the_move).unwrap_or(state.pos);
                state
            })),
            sub_commands: SubCommandsFn::new(move |state: ACState<B>| {
                // let pos = state.pos.make_move(the_move).unwrap_or(state.pos);
                moves_options(state.pos, false)
            }),
        };
        res.push(Box::new(cmd));
    }
    res
}

pub fn named_entity_to_command<B: Board, T: NamedEntity + ?Sized>(
    entity: &T,
) -> Box<dyn CommandTrait<B>> {
    let primary_name = entity.short_name();
    let mut other_names = ArrayVec::default();
    if !entity.long_name().eq_ignore_ascii_case(&primary_name) {
        other_names.push(entity.long_name());
    }
    let cmd = Command {
        primary_name,
        other_names,
        help_text: entity
            .description()
            .unwrap_or_else(|| "<No Description>".to_string()),
        standard: Custom,
        autocomplete_recurse: false,
        func: |_, _, _| Ok(()),
        change_ac_state: AutoCompleteFunc::default(),
        sub_commands: SubCommandsFn::default(),
    };
    Box::new(cmd)
}

pub fn select_command<B: Board, T: NamedEntity + ?Sized>(list: &[Box<T>]) -> CommandList<B> {
    let mut res: CommandList<B> = vec![];
    for entity in list {
        res.push(named_entity_to_command(entity.as_ref()));
    }
    res
}

pub fn options_options<B: Board, const VALUE: bool>(
    info: Arc<Mutex<EngineInfo>>,
    accept_name_word: bool,
) -> CommandList<B> {
    let mut res: CommandList<B> = select_command(
        EngineOptionName::iter()
            .dropping_back(1)
            .map(Box::new)
            .collect_vec()
            .as_slice(),
    );
    for info in info.lock().unwrap().additional_options() {
        res.push(named_entity_to_command(&info.name));
    }
    if VALUE {
        for opt in &mut res {
            let completion = SubCommandsFn(Box::new(|_| {
                let name = Name {
                    short: "value".to_string(),
                    long: "value".to_string(),
                    description: Some("Set the value".to_string()),
                };
                vec![named_entity_to_command::<B, Name>(&name).upcast_box()]
            }));
            opt.set_autocompletions(completion);
        }
    }
    if accept_name_word {
        // insert now so that the autocompletion won't be changed
        let cmd = misc_command!(
            name | n,
            All,
            "Select an option name",
            |_, _, _| Ok(()),
            -> |state: ACState<B>| options_options::<B, VALUE>(state.info.clone(), false)
        );
        res.insert(0, cmd);
    }
    res
}

#[derive(Debug, Clone)]
pub struct ACState<B: Board> {
    pub pos: B,
    outputs: Rc<OutputList<B>>,
    searchers: Rc<SearcherList<B>>,
    evals: Rc<EvalList<B>>,
    info: Arc<Mutex<EngineInfo>>,
}

#[derive(Debug, Clone)]
pub struct CommandAutocomplete<B: Board> {
    // Rc because the Autocomplete trait requires DynClone and invokes `clone` on every prompt call
    pub list: Rc<CommandList<EngineUGI<B>>>,
    pub state: ACState<B>,
}

impl<B: Board> CommandAutocomplete<B> {
    pub fn new(list: CommandList<EngineUGI<B>>, ugi: &EngineUGI<B>) -> Self {
        Self {
            list: Rc::new(list),
            state: ACState {
                pos: ugi.state.board,
                outputs: ugi.output_factories.clone(),
                searchers: ugi.searcher_factories.clone(),
                evals: ugi.eval_factories.clone(),
                info: ugi.state.engine.get_engine_info_arc(),
            },
        }
    }
}

fn distance(input: &str, name: &str) -> usize {
    if input.eq_ignore_ascii_case(name) {
        0
    } else {
        let lowercase_name = name.to_lowercase();
        let input = input.to_lowercase();
        let prefix = &lowercase_name.as_bytes()[..input.len().min(lowercase_name.len())];
        2 + edit_distance(&input, from_utf8(prefix).unwrap_or(name))
    }
}

fn push<B: Board, T: AbstractCommand<B> + ?Sized>(
    completions: &mut Vec<(usize, Completion)>,
    word: &str,
    node: &T,
) {
    completions.push((
        node.autocomplete_badness(word, distance),
        Completion {
            name: node.short_name(),
            text: completion_text(node, word),
        },
    ));
}

fn completions<B: Board>(
    node: &dyn AbstractCommand<B>,
    state: ACState<B>,
    mut rest: Tokens,
    to_complete: &str,
) -> Vec<(usize, Completion)> {
    let mut res = vec![];
    let next = rest.peek().copied();
    for child in node.sub_commands(state.clone()) {
        // TODO: Use is_none_or in Rust 1.82
        if next.is_none() || next.is_some_and(|n| n == to_complete) || node.autocomplete_recurse() {
            push(&mut res, to_complete, child.as_ref());
        }
        if next.is_some_and(|name| child.matches(name)) {
            let mut rest = rest.clone();
            _ = rest.next();
            let new_state = child.change_autocomplete_state(state.clone());
            res.append(&mut completions(
                child.as_ref(),
                new_state,
                rest,
                to_complete,
            ));
        }
    }
    res
}

fn underline_match(name: &str, word: &str) -> String {
    if name == word {
        format!("{}", name.underlined())
    } else {
        name.to_string()
    }
}

fn completion_text<B: Board, T: AbstractCommand<B> + ?Sized>(n: &T, word: &str) -> String {
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
    let mut words = tokens(input);
    let Some(cmd_name) = words.next() else {
        return vec![];
    };
    let to_complete = if input.ends_with(|s: char| s.is_whitespace()) {
        ""
    } else {
        input.split_whitespace().last().unwrap()
    };
    let should_complete_this = words.peek().is_none() && !to_complete.is_empty();

    let mut res = vec![];
    if !(should_complete_this && to_complete == "?") {
        for cmd in autocomplete.list.iter() {
            if should_complete_this {
                push(&mut res, to_complete, cmd.as_ref());
            } else if cmd.matches(cmd_name) {
                let mut new = completions(
                    cmd.upcast_ref(),
                    autocomplete.state.clone(),
                    words.clone(),
                    to_complete,
                );
                res.append(&mut new);
            }
        }
    }
    if should_complete_this {
        let moves = moves_options(autocomplete.state.pos, false);
        for mov in moves {
            push(&mut res, to_complete, mov.upcast_box().as_ref());
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

// Useful for generating a fuzz testing corpus
#[allow(unused)]
pub fn random_command<B: Board>(
    initial: String,
    ac: &mut CommandAutocomplete<B>,
    depth: usize,
) -> String {
    let mut res = initial;
    for i in 0..depth {
        res.push(' ');
        let s = suggestions(ac, &res);
        let s = s.choose(&mut thread_rng());
        if thread_rng().gen_range(0..7) == 0 {
            res += &thread_rng().gen_range(-42..10_000).to_string();
        } else if depth <= 0 || s.is_none() {
            return res;
        } else {
            res += &s.unwrap().name;
        }
    }
    res
}
