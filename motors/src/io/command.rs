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
use crate::io::SearchType::{Bench, Normal, Perft, Ponder, SplitPerft};
use crate::io::{AbstractEngineUgi, EngineUGI, SearchType};
use crate::search::{AbstractEvalBuilder, AbstractSearcherBuilder, EngineInfo, EvalList, SearcherList};
use colored::Colorize;
use edit_distance::edit_distance;
use gears::arrayvec::ArrayVec;
use gears::cli::Game;
use gears::games::{Color, OutputList};
use gears::general::board::{Board, BoardHelpers, Strictness};
use gears::general::common::anyhow::{anyhow, bail};
use gears::general::common::{
    parse_duration_ms, parse_int, parse_int_from_str, tokens, Name, NamedEntity, Res, Tokens,
};
use gears::general::move_list::MoveList;
use gears::general::moves::{ExtendedFormat, Move};
use gears::output::Message::Warning;
use gears::output::{Message, OutputBuilder, OutputOpts};
use gears::search::{Depth, NodesLimit, SearchLimit};
use gears::ugi::{only_load_ugi_position, EngineOptionName};
use gears::MatchStatus::{Ongoing, Over};
use gears::ProgramStatus::Run;
use gears::Quitting::{QuitMatch, QuitProgram};
use gears::{GameResult, ProgramStatus, Quitting};
use inquire::autocompletion::Replacement;
use inquire::{Autocomplete, CustomUserError};
use itertools::Itertools;
use rand::prelude::IndexedRandom;
use rand::{rng, Rng};
use std::fmt;
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

#[derive(Debug, Clone)]
pub struct ACState<B: Board> {
    pub go_state: GoState<B>,
    outputs: Rc<OutputList<B>>,
    searchers: Rc<SearcherList<B>>,
    evals: Rc<EvalList<B>>,
    info: Arc<Mutex<EngineInfo>>,
}

impl<B: Board> ACState<B> {
    fn pos(&self) -> &B {
        &self.go_state.pos
    }
}

/// The point of this Visitor-like pattern is to minimize the amount of generic code to improve compile times:
/// It means that all commands are completely independent of the generic `Board` parameter; everything board-specific
/// is handled in this trait.
trait AutoCompleteState: Debug {
    fn go_subcmds(&self, search_type: SearchType) -> CommandList;
    fn pos_subcmds(&self, accept_pos: bool) -> CommandList;
    fn option_subcmds(&self, allow_value: bool) -> CommandList;
    fn moves_subcmds(&self, allow_moves_word: bool, recurse: bool) -> CommandList;
    fn query_subcmds(&self) -> CommandList;
    fn output_subcmds(&self) -> CommandList;
    fn print_subcmds(&self) -> CommandList;
    fn engine_subcmds(&self) -> CommandList;
    fn set_eval_subcmds(&self) -> CommandList;
    fn make_move(&mut self, mov: &str);
}

impl<B: Board> AutoCompleteState for ACState<B> {
    fn go_subcmds(&self, search_type: SearchType) -> CommandList {
        go_options::<B>(Some(search_type))
    }
    fn pos_subcmds(&self, accept_pos: bool) -> CommandList {
        position_options(Some(self.pos()), accept_pos)
    }
    fn option_subcmds(&self, allow_value: bool) -> CommandList {
        options_options(self.info.clone(), true, allow_value)
    }
    fn moves_subcmds(&self, allow_moves_word: bool, recurse: bool) -> CommandList {
        let mut res = moves_options(self.pos(), recurse);
        if allow_moves_word {
            res.push(move_command(recurse));
        }
        res
    }
    fn query_subcmds(&self) -> CommandList {
        query_options::<B>()
    }
    fn output_subcmds(&self) -> CommandList {
        add(
            select_command::<dyn OutputBuilder<B>>(self.outputs.as_slice(), false),
            vec![
                named_entity_to_command(
                    &Name {
                        short: "remove".to_string(),
                        long: "remove".to_string(),
                        description: Some("Remove the specified output, or all if not given".to_string()),
                    },
                    false,
                ),
                named_entity_to_command(
                    &Name {
                        short: "add".to_string(),
                        long: "add".to_string(),
                        description: Some("Add an output without changing existing outputs".to_string()),
                    },
                    false,
                ),
            ],
        )
    }
    fn print_subcmds(&self) -> CommandList {
        add(
            select_command::<dyn OutputBuilder<B>>(self.outputs.as_slice(), false),
            position_options(Some(self.pos()), true),
        )
    }
    fn engine_subcmds(&self) -> CommandList {
        select_command::<dyn AbstractSearcherBuilder<B>>(self.searchers.as_slice(), false)
    }
    fn set_eval_subcmds(&self) -> CommandList {
        select_command::<dyn AbstractEvalBuilder<B>>(self.evals.as_slice(), false)
    }

    fn make_move(&mut self, mov: &str) {
        let Ok(mov) = B::Move::from_text(mov, self.pos()) else {
            return;
        };
        if let Some(new) = self.pos().clone().make_move(mov) {
            self.go_state.pos = new;
        }
    }
}

impl<B: Board> AbstractEngineUgi for ACState<B> {
    fn options_text(&self, _words: &mut Tokens) -> Res<String> {
        Ok(String::new())
    }
    fn write_ugi(&mut self, _message: &fmt::Arguments) {
        /*do nothing*/
    }
    fn write_message(&mut self, _message: Message, _msg: &fmt::Arguments) {
        /*do nothing*/
    }
    fn write_response(&mut self, _msg: &str) -> Res<()> {
        Ok(())
    }
    fn status(&self) -> &ProgramStatus {
        &Run(Ongoing)
    }
    fn go_state_mut(&mut self) -> &mut dyn AbstractGoState {
        &mut self.go_state
    }

    fn load_go_state_pos(&mut self, name: &str, words: &mut Tokens) -> Res<()> {
        self.go_state.load_pos(name, words, true)
    }

    fn handle_ugi(&mut self, _proto: &str) -> Res<()> {
        Ok(())
    }
    fn handle_uginewgame(&mut self) -> Res<()> {
        Ok(())
    }
    fn handle_pos(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_go(&mut self, _initial_search_type: SearchType, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_stop(&mut self, _suppress_best_move: bool) -> Res<()> {
        Ok(())
    }
    fn handle_ponderhit(&mut self) -> Res<()> {
        Ok(())
    }
    fn handle_setoption(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_interactive(&mut self) -> Res<()> {
        Ok(())
    }
    fn handle_debug(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_log(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_output(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_print(&mut self, _words: &mut Tokens, _opts: OutputOpts) -> Res<()> {
        Ok(())
    }
    fn handle_engine_print(&mut self) -> Res<()> {
        Ok(())
    }
    fn handle_eval_or_tt(&mut self, _eval: bool, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_engine(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_set_eval(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn load_pgn(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_flip(&mut self) -> Res<()> {
        self.go_state.pos = self.go_state.pos.clone().make_nullmove().ok_or(anyhow!(""))?;
        Ok(())
    }
    fn handle_query(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_play(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }

    fn handle_assist(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }

    fn handle_undo(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }

    fn handle_prev(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }

    fn print_help(&mut self) -> Res<()> {
        Ok(())
    }
    fn write_is_player(&mut self, _is_first: bool) -> Res<()> {
        Ok(())
    }
    fn respond_game(&mut self) -> Res<()> {
        Ok(())
    }
    fn respond_engine(&mut self) -> Res<()> {
        Ok(())
    }
    fn handle_quit(&mut self, _typ: Quitting) -> Res<()> {
        Ok(())
    }
}

#[allow(type_alias_bounds)]
pub type CommandList = Vec<Command>;

fn display_cmd(f: &mut Formatter<'_>, cmd: &Command) -> fmt::Result {
    if let Some(desc) = cmd.description() {
        write!(f, "{}: {desc}.", cmd.short_name().bold())
    } else {
        write!(f, "{}", cmd.short_name().bold())
    }
}

type SubCommandFnT = Box<dyn Fn(&mut dyn AutoCompleteState) -> CommandList>;
#[derive(Default)]
struct SubCommandsFn(Option<SubCommandFnT>);

impl Debug for SubCommandsFn {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<subcommands>")
    }
}

impl SubCommandsFn {
    pub fn new(cmd: fn(&mut dyn AutoCompleteState) -> CommandList) -> Self {
        Self(Some(Box::new(cmd)))
    }

    fn call(&self, state: &mut dyn AutoCompleteState) -> CommandList {
        match &self.0 {
            None => vec![],
            Some(f) => f(state),
        }
    }
}

#[derive(Debug)]
pub struct Command {
    pub primary_name: String,
    pub other_names: ArrayVec<String, 4>,
    pub help_text: String,
    pub standard: Standard,
    pub autocomplete_recurse: bool,
    pub func: fn(&mut dyn AbstractEngineUgi, remaining_input: &mut Tokens, _cmd: &str) -> Res<()>,
    sub_commands: SubCommandsFn,
}

impl Command {
    pub fn standard(&self) -> Standard {
        self.standard
    }

    pub fn func(&self) -> fn(&mut dyn AbstractEngineUgi, &mut Tokens, &str) -> Res<()> {
        self.func
    }

    fn sub_commands(&self, state: &mut dyn AutoCompleteState) -> CommandList {
        self.sub_commands.call(state)
    }

    fn autocomplete_recurse(&self) -> bool {
        self.autocomplete_recurse
    }
}

impl NamedEntity for Command {
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
        name.eq_ignore_ascii_case(&self.primary_name) || self.other_names.iter().any(|n| n.eq_ignore_ascii_case(name))
    }

    fn autocomplete_badness(&self, input: &str, matcher: fn(&str, &str) -> isize) -> isize {
        matcher(input, &self.primary_name).min(
            self.other_names
                .iter()
                // prefer primary matches
                .map(|name| 1 + matcher(input, name))
                .min()
                .unwrap_or(isize::MAX),
        )
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        display_cmd(f, self)
    }
}

macro_rules! command {
    ($primary:ident $(| $other:ident)*, $std:expr, $help:expr, $fun:expr
    $(, -->$subcmd:expr)? $(, recurse=$recurse:expr)?) => {
        {
            #[allow(unused_mut, unused_assignments)]
            let mut sub_commands = SubCommandsFn::default();
            $(
                sub_commands.0 = Some(Box::new($subcmd));
            )?

            #[allow(unused_mut, unused_assignments)]
            let mut autocomplete_recurse = false;
            $(
                autocomplete_recurse = $recurse;
            )?

            Command {
                primary_name: stringify!($primary).to_string(),
                other_names: ArrayVec::from_iter([$(stringify!($other).to_string(),)*]),
                standard: $std,
                func: $fun,
                help_text: $help.to_string(),
                sub_commands,
                autocomplete_recurse,
            }
        }
    };
}

/// All commands type erase the board type in order to speed up compilation
#[expect(clippy::too_many_lines)]
pub fn ugi_commands() -> CommandList {
    vec![
        // put time-critical commands at the top
        command!(
            go | g | search,
            All,
            "Start the search. Optionally takes a position and a mode such as `perft`",
            |ugi: &mut dyn AbstractEngineUgi, words, _| { ugi.handle_go(Normal, words) },
            --> |state: &mut dyn AutoCompleteState| state.go_subcmds(Normal),
            recurse = true
        ),
        command!(stop, All, "Stop the current search. No effect if not searching", |ugi, _, _| ugi.handle_stop(false)),
        command!(
            position | pos | p,
            All,
            "Set the current position",
            |ugi, words, _| ugi.handle_pos(words),
            --> |state| state.pos_subcmds(false)
        ),
        command!(
            ugi | uci | uai,
            All,
            "Starts UGI mode, ends interactive mode (can be re-enabled with `interactive`)",
            |ugi, _, proto| ugi.handle_ugi(proto)
        ),
        command!(ponderhit, All, "Stop pondering and start a normal search", |ugi, _, _| ugi.handle_ponderhit()),
        command!(isready, All, "Queries if the engine is ready. The engine responds with 'readyok'", |ugi, _, _| {
            ugi.write_ugi(&format_args!("readyok"));
            Ok(())
        }),
        command!(
            setoption | so,
            All,
            "Sets an engine option",
            |ugi, words, _| ugi.handle_setoption(words),
            --> |state| state.option_subcmds(true)
        ),
        command!(
            uginewgame | ucinewgame | uainewgame | clear,
            All,
            "Resets the internal engine state (doesn't reset engine options)",
            |ugi, _, _| ugi.handle_uginewgame()
        ),
        command!(register, All, "UCI command for copy-protected engines, doesn't apply here", |ugi, _, _| {
            ugi.write_message(Warning, &format_args!("{} isn't supported and will be ignored", "register".red()));
            Ok(())
        }),
        command!(
            flip | null,
            Custom,
            "Flips the side to move, unless this results in an illegal position",
            |ugi, _, _| ugi.handle_flip()
        ),
        command!(quit, All, "Exits the program immediately", |ugi, _, _| {
            if cfg!(feature = "fuzzing") {
                eprintln!("Fuzzing is enabled, ignoring 'quit' command");
                return Ok(());
            }
            ugi.handle_quit(QuitProgram)
        }),
        command!(
            quit_match | end_game | qm,
            Custom,
            "Quits the current match and, if `play` has been used, returns to the previous match",
            |ugi, _, _| {
                if cfg!(feature = "fuzzing") {
                    eprintln!("Fuzzing is enabled, ignoring 'quitmatch' command");
                    return Ok(());
                }
                ugi.handle_quit(QuitMatch)
            }
        ),
        command!(
            query | q,
            UgiNotUci,
            "Answer a query about the current match state",
            |ugi, words, _| ugi.handle_query(words),
            --> |state| state.query_subcmds()
        ),
        command!(
            option | info,
            Custom,
            "Prints information about the current options. Optionally takes an option name",
            |ugi, words, _| {
                ugi.write_ugi(&format_args!("{}", ugi.options_text(words)?));
                Ok(())
            },
            --> |state| state.option_subcmds(false)
        ),
        command!(
            engine_state,
            Custom,
            "Prints information about the internal engine state, if supported",
            |ugi, _, _| ugi.handle_engine_print()
        ),
        command!(
            output | o,
            Custom,
            "Sets outputs, which are used to print the game state. Permanent version of 'show'",
            |ugi, words, _| ugi.handle_output(words),
            --> |state| state.output_subcmds(),
            recurse = true
        ),
        command!(
            print | show | s | display,
            Custom,
            "Display the specified / current position with specified / enabled outputs or 'prettyascii' if no output is set",
            |ugi, words, _| ugi.handle_print(words, OutputOpts::default()),
            --> |state| state.print_subcmds(),
            recurse = true
        ),
        command!(
            log,
            Custom,
            "Enables logging. Can optionally specify a file name, `stdout` / `stderr` or `off`",
            |ugi, words, _| ugi.handle_log(words)
        ),
        command!(
            debug | d,
            Custom,
            "Turns on logging, continue-on-error mode, and additional output. Use `off` to disable",
            |ugi, words, _| ugi.handle_debug(words)
        ),
        command!(
            interactive | i | human,
            Custom,
            "Starts interactive mode, undoes `ugi`. In this mode, errors aren't fatal",
            |ugi, _, _| ugi.handle_interactive()
        ),
        command!(
            engine,
            Custom,
            "Sets the current engine, e.g. `caps-piston`, `gaps`, and optionally the game",
            |ugi, words, _| ugi.handle_engine(words),
            --> |state| state.engine_subcmds()
        ),
        command!(
            set_eval | se,
            Custom,
            "Sets the eval for the current engine. Doesn't reset the internal engine state",
            |ugi, words, _| ugi.handle_set_eval(words),
            --> |state| state.set_eval_subcmds()
        ),
        command!(load_pgn | pgn, Custom, "Loads a PGN from a given file, or opens a text editor", |ugi, words, _| {
            ugi.load_pgn(words)
        }),
        command!(
            play | game,
            Custom,
            "Starts a new match, possibly of a new game, optionally setting a new engine and position",
            |ugi, words, _| {
                if cfg!(feature = "fuzzing") {
                    eprintln!("Fuzzing is enabled, ignoring 'play'");
                    return Ok(())
                }
                ugi.handle_play(words)
            },
            --> |_| select_command::<Game>(&Game::iter().map(Box::new).collect_vec(), false)
        ),
        command!(
            idk | assist | respond,
            Custom,
            "Lets the engine play a move, or use 'on'/'off' to enable/disable automatic response",
            |ugi, words, _| ugi.handle_assist(words)
        ),
        command!(undo | take_back | u, Custom, "Undoes 1 or a given number of halfmoves", |ugi, words, _| {
            ugi.handle_undo(words)
        }),
        command!(
            go_back | gb,
            Custom,
            "Set the position to the previous 'position' command, like 'p old', and removes later positions",
            |ugi, words, _| ugi.handle_prev(words)
        ),
        command!(
            perft,
            Custom,
            "Internal movegen test on current / bench positions",
            |ugi, words, _| ugi.handle_go(Perft, words),
            --> |state| state.go_subcmds(Perft),
            recurse = true
        ),
        command!(
            splitperft | sp,
            Custom,
            "Internal movegen test on current / bench positions",
            |ugi, words, _| ugi.handle_go(SplitPerft, words),
            --> |state| state.go_subcmds(SplitPerft),
            recurse = true
        ),
        command!(
            bench,
            Custom,
            "Internal search test on current / bench positions. Same arguments as `go`",
            |ugi, words, _| ugi.handle_go(Bench, words),
            --> |state| state.go_subcmds(Bench),
            recurse = true
        ),
        command!(
            eval | e | static_eval,
            Custom,
            "Print the static eval (i.e., no search) of a position",
            |ugi, words, _| ugi.handle_eval_or_tt(true, words),
            --> |state| state.pos_subcmds(true),
            recurse = true
        ),
        command!(
            tt | tt_entry,
            Custom,
            "Print the TT entry for a position",
            |ugi, words, _| ugi.handle_eval_or_tt(false, words),
            --> |state| state.pos_subcmds(true),
            recurse = true
        ),
        command!(help | h, Custom, "Prints a help message", |ugi, _, _| {
            ugi.print_help() // TODO: allow help <command> to print a help message for a command
        }),
    ]
}

pub trait AbstractGoState: Debug {
    fn set_searchmoves(&mut self, words: &mut Tokens) -> Res<()>;
    fn set_time(&mut self, words: &mut Tokens, first: bool, inc: bool, name: &str) -> Res<()>;
    fn override_hash(&mut self, words: &mut Tokens) -> Res<()>;
    fn limit_mut(&mut self) -> &mut SearchLimit;
    fn get_mut(&mut self) -> &mut GenericGoState;
    fn load_pos(&mut self, name: &str, words: &mut Tokens, allow_partial: bool) -> Res<()>;
    fn set_search_type(&mut self, search_type: SearchType, depth_words: Option<&mut Tokens>) -> Res<()>;
}

impl<B: Board> AbstractGoState for GoState<B> {
    fn set_searchmoves(&mut self, words: &mut Tokens) -> Res<()> {
        let mut search_moves = vec![];
        while let Some(mov) = words.peek().and_then(|m| B::Move::from_text(m, &self.pos).ok()) {
            _ = words.next().unwrap();
            search_moves.push(mov);
        }
        if search_moves.is_empty() {
            bail!("No valid moves after 'searchmoves' command");
        }
        self.search_moves = Some(search_moves);
        Ok(())
    }

    fn set_time(&mut self, words: &mut Tokens, first: bool, inc: bool, name: &str) -> Res<()> {
        let time = parse_duration_ms(words, name)?;
        // always parse the duration, even if it isn't relevant
        if self.generic.is_first == first {
            if inc {
                self.generic.limit.tc.increment = time;
            } else {
                self.generic.limit.tc.remaining = time;
            }
        }
        Ok(())
    }

    fn override_hash(&mut self, words: &mut Tokens) -> Res<()> {
        self.generic.override_hash_size = Some(parse_int(words, "TT size in MB")?);
        Ok(())
    }

    fn limit_mut(&mut self) -> &mut SearchLimit {
        &mut self.generic.limit
    }

    fn get_mut(&mut self) -> &mut GenericGoState {
        &mut self.generic
    }

    fn load_pos(&mut self, name: &str, words: &mut Tokens, allow_partial: bool) -> Res<()> {
        self.pos = only_load_ugi_position(name, words, &self.pos, self.generic.strictness, true, allow_partial)?;
        Ok(())
    }

    fn set_search_type(&mut self, search_type: SearchType, depth_words: Option<&mut Tokens>) -> Res<()> {
        self.generic.search_type = search_type;
        if let Some(words) = depth_words {
            accept_depth(&mut self.generic.limit, words)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct GoState<B: Board> {
    pub generic: GenericGoState,
    pub search_moves: Option<Vec<B::Move>>,
    pub pos: B,
}

#[derive(Debug, Clone)]
pub struct GenericGoState {
    pub limit: SearchLimit,
    pub is_first: bool,
    pub multi_pv: usize,
    pub threads: Option<usize>,
    pub search_type: SearchType,
    pub complete: bool,
    pub move_overhead: Duration,
    pub strictness: Strictness,
    pub override_hash_size: Option<usize>,
}

impl<B: Board> GoState<B> {
    pub fn new(ugi: &EngineUGI<B>, search_type: SearchType) -> Self {
        let limit = match search_type {
            Bench => SearchLimit::depth(ugi.state.engine.get_engine_info().default_bench_depth()),
            Perft | SplitPerft => SearchLimit::depth(ugi.state.pos().default_perft_depth()),
            // "infinite" is the identity element of the bounded semilattice of `go` options
            _ => SearchLimit::infinite(),
        };
        Self::new_for_pos(ugi.state.pos().clone(), limit, ugi.strictness, ugi.move_overhead, search_type)
    }

    pub fn new_for_pos(
        pos: B,
        limit: SearchLimit,
        strictness: Strictness,
        move_overhead: Duration,
        search_type: SearchType,
    ) -> Self {
        Self {
            generic: GenericGoState {
                limit,
                is_first: pos.active_player().is_first(),
                multi_pv: 1,
                threads: None,
                search_type,
                complete: false,
                move_overhead,
                strictness,
                override_hash_size: None,
            },
            search_moves: None,
            pos,
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

pub fn depth_cmd() -> Command {
    command!(depth | d, All, "Maximum search depth in plies (a.k.a. half-moves)", |state, words, _| {
        state.go_state_mut().limit_mut().depth = Depth::try_new(parse_int(words, "depth number")?)?;
        Ok(())
    })
}

pub fn go_options<B: Board>(mode: Option<SearchType>) -> CommandList {
    // TODO: This doesn't update the colors when they are changed at runtime in the Fairy board,
    // so even though the FEN will parse e.g. x/o it'll still be wtime/btime.
    let pos = B::default();
    let mut res = go_options_impl(mode, pos.color_chars(), pos.color_names());

    // We don't want to allow `go e4` or `go moves e4` for two reasons: Because that's a bit confusing, and because it would make the number of
    // `go` commands depend on the position, which means that we couldn't precompute the commands in`UgiGui`.
    // Instead, it needs to be spelled as `g c e4`, `go position current moves e4`, etc
    res.append(&mut position_options::<B>(None, true));
    res
}

#[expect(clippy::too_many_lines)]
pub fn go_options_impl(mode: Option<SearchType>, color_chars: [char; 2], color_names: [String; 2]) -> CommandList {
    let mut res = vec![depth_cmd()];
    if !matches!(mode.unwrap_or(Normal), Perft | SplitPerft) {
        let mut additional: CommandList = vec![
            Command {
                primary_name: format!("{}time", color_chars[0]),
                other_names: ArrayVec::from_iter([
                    format!("{}t", color_chars[0]),
                    "p1time".to_string(),
                    "p1t".to_string(),
                ]),
                help_text: format!("Remaining time in ms for {}", color_names[0]),
                standard: All,
                autocomplete_recurse: false,
                func: |state, words, _| state.go_state_mut().set_time(words, true, false, "p1time"),
                sub_commands: SubCommandsFn::default(),
            },
            Command {
                primary_name: format!("{}time", color_chars[1]),
                other_names: ArrayVec::from_iter([
                    format!("{}t", color_chars[1]),
                    "p2time".to_string(),
                    "p2t".to_string(),
                ]),
                help_text: format!("Remaining time in ms for {}", color_names[1]),
                standard: All,
                autocomplete_recurse: false,
                func: |state, words, _| state.go_state_mut().set_time(words, false, false, "p2time"),
                sub_commands: SubCommandsFn::default(),
            },
            Command {
                primary_name: format!("{}inc", color_chars[0]),
                other_names: ArrayVec::from_iter([format!("{}i", color_chars[0]), "p1inc".to_string()]),
                help_text: format!("Increment in ms for {}", color_names[0]),
                standard: All,
                autocomplete_recurse: false,
                func: |state, words, _| state.go_state_mut().set_time(words, true, true, "p1inc"),
                sub_commands: SubCommandsFn::default(),
            },
            Command {
                primary_name: format!("{}inc", color_chars[1]),
                other_names: ArrayVec::from_iter([format!("{}i", color_chars[1]), "p2inc".to_string()]),
                help_text: format!("Increment in ms for {}", color_names[1]),
                standard: All,
                autocomplete_recurse: false,
                func: |state, words, _| state.go_state_mut().set_time(words, false, true, "p2inc"),
                sub_commands: SubCommandsFn::default(),
            },
            command!(movestogo | mtg, All, "Full moves until the time control is reset", |state, words, _| {
                state.go_state_mut().limit_mut().tc.moves_to_go = Some(parse_int(words, "'movestogo' number")?);
                Ok(())
            }),
            command!(nodes | n, All, "Maximum number of nodes to search", |state, words, _| {
                state.go_state_mut().limit_mut().nodes = NodesLimit::new(parse_int(words, "node count")?)
                    .ok_or_else(|| anyhow!("node count can't be zero"))?;
                Ok(())
            }),
            command!(mate | m, All, "Maximum depth in moves until a mate has to be found", |state, words, _| {
                let depth: isize = parse_int(words, "mate move count")?;
                state.go_state_mut().limit_mut().mate = Depth::try_new(depth * 2)?; // 'mate' is given in moves instead of plies
                Ok(())
            }),
            command!(movetime | mt | time, All, "Maximum time in ms", |state, words, _| {
                let generic = state.go_state_mut().get_mut();
                let limit = &mut generic.limit;
                limit.fixed_time = parse_duration_ms(words, "time per move in milliseconds")?;
                limit.fixed_time = limit.fixed_time.saturating_sub(generic.move_overhead).max(Duration::from_millis(1));
                Ok(())
            }),
            command!(infinite | inf, All, "Search until receiving `stop`, the default mode", |state, _, _| {
                *state.go_state_mut().limit_mut() = SearchLimit::infinite();
                Ok(())
            }),
            command!(
                searchmoves | sm,
                All,
                "Only consider the specified moves",
                |state, words, _| state.go_state_mut().set_searchmoves(words),
                --> |state| state.moves_subcmds(false, false),
                recurse = true
            ),
            command!(
                multipv | mpv,
                Custom,
                "Find the n best moves, temporarily overwriting the 'multipv' engine option",
                |state, words, _| {
                    state.go_state_mut().get_mut().multi_pv = parse_int(words, "multipv")?;
                    Ok(())
                }
            ),
            command!(
                threads | t,
                Custom,
                "Search with n threads in parallel, temporarily overwriting the 'threads' engine option",
                |state, words, _| {
                    state.go_state_mut().get_mut().threads = Some(parse_int(words, "threads")?);
                    Ok(())
                }
            ),
            command!(hash | h | tt, Custom, "Search with a temporary TT of n MiB", |state, words, _| state
                .go_state_mut()
                .override_hash(words)),
            command!(ponder, All, "Search on the opponent's time", |state, _, _| state
                .go_state_mut()
                .set_search_type(Ponder, None)),
            command!(perft | pt, Custom, "Movegen test: Make all legal moves up to a depth", |state, words, _| state
                .go_state_mut()
                .set_search_type(Perft, Some(words))),
            command!(
                splitperft | sp,
                Custom,
                "Movegen test: Print perft number for each legal move",
                |state, words, _| state.go_state_mut().set_search_type(SplitPerft, Some(words))
            ),
            command!(
                bench | b,
                Custom,
                "Search test: Print info about nodes, nps, and hash of search",
                |state, words, _| state.go_state_mut().set_search_type(Bench, Some(words))
            ),
        ];
        res.append(&mut additional);
    }
    if matches!(mode.unwrap_or(Bench), Bench | Perft) {
        let complete_option =
            command!(complete | all, Custom, "Run bench / perft on all bench positions", |state, _, _| {
                state.go_state_mut().get_mut().complete = true;
                Ok(())
            });
        res.push(complete_option);
    }
    res
}

pub fn query_options<B: Board>() -> CommandList {
    // TODO: See go_options, doesn't update the chars
    query_options_impl(B::default().color_chars())
}

pub fn query_options_impl(color_chars: [char; 2]) -> CommandList {
    vec![
        command!(gameover, UgiNotUci, "Is the game over?", |ugi, _, _| {
            ugi.write_response(&matches!(ugi.status(), Run(Ongoing)).to_string())
        }),
        Command {
            primary_name: "p1turn".to_string(),
            other_names: ArrayVec::from_iter([format!("{}turn", color_chars[0])]),
            help_text: "Is it the first player's turn?".to_string(),
            standard: UgiNotUci,
            autocomplete_recurse: false,
            func: |ugi, _, _| ugi.write_is_player(true),
            sub_commands: SubCommandsFn::default(),
        },
        Command {
            primary_name: "p2turn".to_string(),
            other_names: ArrayVec::from_iter([format!("{}turn", color_chars[1])]),
            help_text: "Is it the second player's turn?".to_string(),
            standard: UgiNotUci,
            autocomplete_recurse: false,
            func: |ugi, _, _| ugi.write_is_player(false),
            sub_commands: SubCommandsFn::default(),
        },
        command!(result | res, UgiNotUci, "The result of the current match", |ugi, _, _| {
            let response = match &ugi.status() {
                Run(Over(res)) => match res.result {
                    GameResult::P1Win => "p1win",
                    GameResult::P2Win => "p2win",
                    GameResult::Draw => "draw",
                    GameResult::Aborted => "aborted",
                },
                _ => "none",
            };
            ugi.write_response(response)
        }),
        command!(game | g, Custom, "The current game", |ugi, _, _| ugi.respond_game()),
        command!(engine | e | name, Custom, "The name of the engine", |ugi, _, _| ugi.respond_engine()),
    ]
}

macro_rules! pos_command {
    ($primary:ident $( | $other:ident)*, $std:expr, $help:expr $(, = $pos:expr)?, $func:expr  $(, ($ACState:ty) $state:expr)? $(, -->$subcmd:expr)? $(, [] $autocomplete_fn:expr)? $(, recurse=$recurse:expr)?) => {
        command!($primary $(| $other)*, $std, $help $(, = $pos)?, $func $(, ($ACState) $state)? $(, -->$subcmd)? $(, [] $autocomplete_fn)? $(, recurse=$recurse)?)
    }
}

fn generic_go_options(accept_pos_word: bool) -> CommandList {
    // TODO: The first couple of options don't depend on B, move in new function?
    let mut res = vec![
        pos_command!(
            fen | f,
            All,
            "Load a positions from a FEN",
            |state, words, _| state.load_go_state_pos("fen", words),
            --> |state| state.moves_subcmds(true, true),
            // TODO: Set position based on the FEN
            recurse = true
        ),
        pos_command!(
            startpos | s,
            All,
            "Load the starting position",
            |state, words, _| {
                state.load_go_state_pos("startpos", words)
            },
            --> |state| state.moves_subcmds(true, true)
        ),
        pos_command!(
            current | c,
            Custom,
            "Current position, useful in combination with 'moves'",
            |state, words, _| state.load_go_state_pos("current", words),
            --> |state| state.moves_subcmds(true, true)
        ),
        command!(
            old | o | previous,
            Custom,
            "Previous 'position' command, could be from a different match (not the same as undoing a move, see 'undo')",
            |state, words, _| state.load_go_state_pos("old", words),
            --> |state| state.moves_subcmds(true, true)
        ),
    ];
    if accept_pos_word {
        res.push(pos_command!(
            position | pos | p,
            Custom,
            "Followed by `fen <fen>`, a position name or a move",
            |_, _, _| Ok(()),
            --> |state| state.pos_subcmds(false)
        ))
    }
    res
}

pub fn position_options<B: Board>(pos: Option<&B>, accept_pos_word: bool) -> CommandList {
    let mut res = generic_go_options(accept_pos_word);
    for p in B::name_to_pos_map() {
        let c = Command {
            primary_name: p.short_name(),
            other_names: Default::default(),
            help_text: p.description().unwrap_or(format!("Load a custom position called '{}'", p.short_name())),
            standard: Custom,
            autocomplete_recurse: false,
            func: |state, words, name| state.load_go_state_pos(name, words),
            sub_commands: SubCommandsFn::new(|state| state.moves_subcmds(true, true)),
        };
        res.push(c);
    }
    res.push(move_command(false));
    if let Some(pos) = pos {
        res.append(&mut moves_options(pos, true))
    }
    res
}

pub fn move_command(recurse: bool) -> Command {
    pos_command!(
        moves | mv,
        All,
        "Apply moves to the specified position",
        |_, _, _| Ok(()),
        --> move |state| state.moves_subcmds(false, recurse)
    )
}

pub fn moves_options<B: Board>(pos: &B, recurse: bool) -> CommandList {
    let mut res: CommandList = vec![];
    for mov in pos.legal_moves_slow().iter_moves() {
        let primary_name = mov.compact_formatter(pos).to_string();
        let mut other_names = ArrayVec::default();
        let extended = mov.to_extended_text(pos, ExtendedFormat::Standard);
        if extended != primary_name {
            other_names.push(extended);
        }
        let cmd = Command {
            primary_name: primary_name.clone(),
            other_names,
            help_text: format!("Play move '{}'", mov.compact_formatter(pos).to_string().bold()),
            standard: All,
            autocomplete_recurse: false,
            func: |_, _, _| Ok(()),
            sub_commands: SubCommandsFn(Some(Box::new(move |state| {
                if recurse {
                    state.make_move(&primary_name);
                    state.moves_subcmds(false, true)
                } else {
                    vec![]
                }
            }))),
        };
        res.push(cmd);
    }
    res
}

pub fn named_entity_to_command(entity: &dyn NamedEntity, value_autocomplete: bool) -> Command {
    let primary_name = entity.short_name();
    let primary_name2 = primary_name.clone();
    let mut other_names = ArrayVec::default();
    if !entity.long_name().eq_ignore_ascii_case(&primary_name) {
        other_names.push(entity.long_name());
    }
    let mut sub_commands = SubCommandsFn::default();
    if value_autocomplete {
        sub_commands = SubCommandsFn(Some(Box::new(move |_| {
            let name = Name {
                short: "value".to_string(),
                long: "value".to_string(),
                description: Some(format!("Set the value of '{primary_name2}'")),
            };
            vec![named_entity_to_command(&name, false)]
        })));
    }
    Command {
        primary_name,
        other_names,
        help_text: entity.description().unwrap_or_else(|| "<No Description>".to_string()),
        standard: Custom,
        autocomplete_recurse: false,
        func: |_, _, _| Ok(()),
        sub_commands,
    }
}

pub fn select_command<T: NamedEntity + ?Sized>(list: &[Box<T>], value_autocomplete: bool) -> CommandList {
    let mut res: CommandList = vec![];
    for entity in list {
        res.push(named_entity_to_command(entity.upcast(), value_autocomplete));
    }
    res
}

pub fn options_options(info: Arc<Mutex<EngineInfo>>, accept_name_word: bool, allow_value: bool) -> CommandList {
    let mut res: CommandList =
        select_command(EngineOptionName::iter().dropping_back(1).map(Box::new).collect_vec().as_slice(), allow_value);
    for info in info.lock().unwrap().additional_options() {
        res.push(named_entity_to_command(&info.name, allow_value));
    }
    if accept_name_word {
        let info_clone = info.clone();
        // insert now so that the autocompletion won't be changed
        let cmd = command!(name | n, All, "Select an option name", |_, _, _| Ok(()), --> move |_| options_options(info_clone.clone(), false, allow_value));
        res.insert(0, cmd);
    }
    res
}

#[derive(Debug, Clone)]
pub struct CommandAutocomplete<B: Board> {
    // Rc because the Autocomplete trait requires DynClone and invokes `clone` on every prompt call
    pub list: Rc<CommandList>,
    pub state: ACState<B>,
}

impl<B: Board> CommandAutocomplete<B> {
    pub fn new(ugi: &EngineUGI<B>) -> Self {
        let state = ACState {
            go_state: GoState::new(ugi, Normal),
            outputs: ugi.output_factories.clone(),
            searchers: ugi.searcher_factories.clone(),
            evals: ugi.eval_factories.clone(),
            info: ugi.state.engine.get_engine_info_arc(),
        };
        Self { list: Rc::new(ugi_commands()), state }
    }
}

fn distance(input: &str, name: &str) -> isize {
    if input.eq_ignore_ascii_case(name) {
        0
    } else {
        // bonus if the case matches exactly for a prefix, so `B` is more likely to be `Bb4` than `b4`.
        let bonus = if input.starts_with(name) { 1 } else { 0 };
        let lowercase_name = name.to_lowercase();
        let input = input.to_lowercase();
        let prefix = &lowercase_name.as_bytes()[..input.len().min(lowercase_name.len())];
        2 * (2 + edit_distance(&input, from_utf8(prefix).unwrap_or(name)) as isize - bonus)
    }
}

fn push(completions: &mut Vec<(isize, Completion)>, word: &str, node: &Command) {
    completions.push((
        node.autocomplete_badness(word, distance),
        Completion { name: node.short_name(), text: completion_text(node, word) },
    ));
}

/// Recursively go through all commands that have been typed so far and add completions.
/// `node` is the command we're currently looking at, `rest` are the tokens after that,
/// and `to_complete` is the last typed token or `""`, which is the one that should be completed
fn completions_for<B: Board>(
    node: &Command,
    state: &mut ACState<B>,
    rest: &mut Tokens,
    to_complete: &str,
) -> Vec<(isize, Completion)> {
    let mut res: Vec<(isize, Completion)> = vec![];
    let mut next_token = rest.peek().copied();
    // ignore all other suggestions if the last complete token requires a subcommand
    // compute this before `next_token` might be changed in the loop
    let add_subcommands = next_token.is_none_or(|n| n == to_complete) || node.autocomplete_recurse();
    loop {
        let mut found_subcommand = false;
        for child in &node.sub_commands(state) {
            // If this command is the last complete token or can recurse, add all subcommands to completions
            if add_subcommands {
                push(&mut res, to_complete, child);
            }
            // if the next token is a subcommand of this command, add suggestions for it.
            // This consumes tokens, so check all remaining subcommands again for the remaining input
            if next_token.is_some_and(|name| child.matches(name)) {
                found_subcommand = true;
                _ = rest.next(); // eat the token for the subcommand
                let mut state = state.clone();
                // possibly change the autocomplete state
                _ = child.func()(&mut state, rest, next_token.unwrap());
                let mut new_completions = completions_for(child, &mut state, rest, to_complete);
                next_token = rest.peek().copied();
                res.append(&mut new_completions);
            }
        }
        if !found_subcommand {
            break;
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

fn completion_text(n: &Command, word: &str) -> String {
    use std::fmt::Write;
    let name = &n.primary_name;
    let mut res = format!("{}", underline_match(name, word).bold());
    for name in &n.other_names {
        write!(&mut res, " | {}", underline_match(name, word)).unwrap();
    }
    write!(&mut res, ":  {}", n.help_text).unwrap();
    res
}

#[derive(Eq, PartialEq)]
struct Completion {
    name: String,
    text: String,
}

/// top-level function for completion suggestions, calls the recursive completions() function
fn suggestions<B: Board>(autocomplete: &CommandAutocomplete<B>, input: &str) -> Vec<Completion> {
    let mut words = tokens(input);
    let Some(cmd_name) = words.next() else {
        return vec![];
    };
    let to_complete =
        if input.ends_with(|s: char| s.is_whitespace()) { "" } else { input.split_whitespace().last().unwrap() };
    let complete_first_token = words.peek().is_none() && !to_complete.is_empty();

    let mut res = vec![];
    if !(complete_first_token && to_complete == "?") {
        for cmd in autocomplete.list.iter() {
            if complete_first_token {
                push(&mut res, to_complete, cmd);
            } else if cmd.matches(cmd_name) {
                let mut new = completions_for(cmd, &mut autocomplete.state.clone(), &mut words, to_complete);
                res.append(&mut new);
            }
        }
    }
    if complete_first_token {
        let moves = moves_options(autocomplete.state.pos(), false);
        for mov in &moves {
            push(&mut res, to_complete, mov);
        }
    }
    res.sort_by_key(|(val, name)| (*val, name.name.clone()));
    if let Some(min) = res.first().map(|(val, _name)| *val) {
        res.into_iter().dedup().take_while(|(val, _text)| *val <= min).map(|(_val, text)| text).collect()
    } else {
        vec![]
    }
}

impl<B: Board> Autocomplete for CommandAutocomplete<B> {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, CustomUserError> {
        Ok(suggestions(self, input).into_iter().map(|c| c.text).collect())
    }

    fn get_completion(
        &mut self,
        input: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<Replacement, CustomUserError> {
        let replacement = {
            let suggestions = suggestions(self, input);
            if let Some(suggestion) = &highlighted_suggestion {
                suggestions.into_iter().find(|s| *s.text == *suggestion).map(|s| s.name)
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
pub fn random_command<B: Board>(initial: String, ac: &mut CommandAutocomplete<B>, depth: usize) -> String {
    let mut res = initial;
    for i in 0..depth {
        res.push(' ');
        let s = suggestions(ac, &res);
        let s = s.choose(&mut rng());
        if rng().random_range(0..7) == 0 {
            res += &rng().random_range(-1000..10_000).to_string();
        } else if depth == 0 || s.is_none() {
            return res;
        } else {
            res += &s.unwrap().name;
        }
    }
    res
}
