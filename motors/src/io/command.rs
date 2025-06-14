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
use crate::io::SearchType::{Auto, Bench, Normal, Perft, Ponder, SplitPerft};
use crate::io::autocomplete::AutoCompleteState;
use crate::io::command::Standard::*;
use crate::io::{AbstractEngineUgiState, EngineUGI, SearchType};
use gears::GameResult;
use gears::MatchStatus::{Ongoing, Over};
use gears::ProgramStatus::Run;
use gears::Quitting::{QuitMatch, QuitProgram};
use gears::arrayvec::ArrayVec;
use gears::cli::Game;
use gears::colored::Colorize;
use gears::games::CharType::{Ascii, Unicode};
use gears::games::{AbstractPieceType, Color, ColoredPiece, Size};
use gears::general::board::{Board, BoardHelpers, ColPieceTypeOf, Strictness};
use gears::general::common::anyhow::{anyhow, bail};
use gears::general::common::{
    Name, NamedEntity, Res, Tokens, parse_duration_ms, parse_int, parse_int_from_str, tokens,
};
use gears::general::move_list::MoveList;
use gears::general::moves::{ExtendedFormat, Move};
use gears::itertools::Itertools;
use gears::output::Message::Warning;
use gears::output::OutputOpts;
use gears::search::{Depth, NodesLimit, SearchLimit};
use gears::ugi::{EngineOption, EngineOptionType, only_load_ugi_position};
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::time::{Duration, Instant};
use strum::IntoEnumIterator;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Standard {
    All,
    UgiNotUci,
    Custom,
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
    pub help_text: Option<String>,
    pub standard: Standard,
    pub autocomplete_recurse: bool,
    pub func: fn(&mut dyn AbstractEngineUgiState, remaining_input: &mut Tokens, _cmd: &str) -> Res<()>,
    sub_commands: SubCommandsFn,
}

impl Command {
    pub fn standard(&self) -> Standard {
        self.standard
    }

    pub fn func(&self) -> fn(&mut dyn AbstractEngineUgiState, &mut Tokens, &str) -> Res<()> {
        self.func
    }

    pub(super) fn sub_commands(&self, state: &mut dyn AutoCompleteState) -> CommandList {
        self.sub_commands.call(state)
    }

    pub(super) fn autocomplete_recurse(&self) -> bool {
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
        self.help_text.clone()
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
        command!([stringify!($primary) $(, stringify!($other))*], $std, $help, $fun $(, -->$subcmd)? $(, recurse=$recurse)?)
    };
    ([$primary:expr $(, $other:expr)*], $std:expr, $help:expr, $fun:expr
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
                primary_name: $primary.to_string(),
                other_names: ArrayVec::from_iter([$($other.to_string(),)*]),
                standard: $std,
                func: $fun,
                help_text: Some($help.to_string()),
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
            |ugi: &mut dyn AbstractEngineUgiState, words, _| { ugi.handle_go(Normal, words) },
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
            --> |state| state.option_subcmds(false)
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
            option | info | listoptions,
            Custom,
            "Prints information about the current options. Optionally takes an option name",
            |ugi, words, _| {
                ugi.write_ugi(&format_args!("{}", ugi.options_text(words)?));
                Ok(())
            },
            --> |state| state.option_subcmds(true)
        ),
        command!(
            engine_state,
            Custom,
            "Prints information about the internal engine state, if supported",
            |ugi, _, _| ugi.handle_engine_print()
        ),
        command!(move_eval | me, Custom, "How the internal engine state considers this move, if supported",
            |ugi, words, _| {
                ugi.handle_move_eval(words)
            },
        --> |state| state.moves_subcmds(false, false)),
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
            engine | eng | searcher,
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
        command!(wait, Custom, "Wait until the current search is done before executing commands", |ugi, words, _| ugi
            .handle_wait(words)),
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
            --> |_| select_command::<Game>(&Game::iter().map(Box::new).collect_vec())
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
            |ugi, words, _| ugi.handle_gb(words)
        ),
        command!(
            place | place_piece | put,
            Custom,
            "Places a piece of the given color on the given square, e.g. 'place white pawn e4",
            |ugi, words, _| ugi.handle_place_piece(words),
            --> |state| state.piece_subcmds()
        ),
        command!(
            remove | remove_piece | rm,
            Custom,
            "Removes the piece at the given square, e.g. 'remove e2'",
            |ugi, words, _| ugi.handle_remove_piece(words),
            --> |state| state.coords_subcmds(false, true)
        ),
        command!(
            move_piece,
            Custom,
            "Moves the piece on the first given square to the second given square, e.g. 'move a1 a2'",
            |ugi, words, _| ugi.handle_move_piece(words),
            --> |state| state.coords_subcmds(true, true)
        ),
        command!(
            random_pos | randomize | rand,
            Custom,
            "Creates a new random position. No guarantees about the probability distribution",
            |ugi, words, _| { ugi.handle_randomize(words) },
            --> |state| state.randomize_subcmds()
        ),
        command!(
            auto,
            Custom,
            "Search like 'go', then play the chosen move. Blocks until the search is complete",
            |ugi, words, _| { ugi.handle_go(Auto, words) },
            --> |state| state.go_subcmds(Auto),
            recurse = true
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
            splitperft | sp | split,
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
    fn set_engine(&mut self, words: &mut Tokens) -> Res<()>;
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
        if search_type == Bench {
            self.generic.limit.depth = self.generic.default_bench_depth;
        } else if search_type == Perft || search_type == SplitPerft {
            self.generic.limit.depth = self.generic.default_perft_depth;
        }
        if let Some(words) = depth_words {
            accept_depth(&mut self.generic.limit, words)?;
        }
        Ok(())
    }

    fn set_engine(&mut self, words: &mut Tokens) -> Res<()> {
        let Some(engine_name) = words.peek() else {
            bail!("Expected engine name after 'engine' go option");
        };
        self.generic.engine_name = Some(engine_name.to_string());
        _ = words.next();
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
    pub unique: bool,
    pub move_overhead: Duration,
    pub strictness: Strictness,
    pub override_hash_size: Option<usize>,
    pub engine_name: Option<String>,
    default_bench_depth: Depth,
    default_perft_depth: Depth,
}

impl<B: Board> GoState<B> {
    pub fn default_depth_limit(ugi: &EngineUGI<B>, search_type: SearchType) -> Depth {
        match search_type {
            Bench => ugi.state.engine.get_engine_info().default_bench_depth(),
            Perft | SplitPerft => ugi.state.pos().default_perft_depth(),
            // "infinite" is the identity element of the bounded semilattice of `go` options
            _ => SearchLimit::infinite().depth,
        }
    }
    pub fn new(ugi: &EngineUGI<B>, search_type: SearchType, start_time: Instant) -> Self {
        let mut limit = SearchLimit::depth(Self::default_depth_limit(ugi, search_type));
        limit.start_time = start_time;
        let bench_limit = Self::default_depth_limit(ugi, Bench);
        let perft_limit = Self::default_depth_limit(ugi, Perft);
        Self::new_for_pos(
            ugi.state.pos().clone(),
            limit,
            ugi.strictness,
            ugi.move_overhead,
            search_type,
            bench_limit,
            perft_limit,
        )
    }

    pub fn new_for_pos(
        pos: B,
        limit: SearchLimit,
        strictness: Strictness,
        move_overhead: Duration,
        search_type: SearchType,
        default_bench_depth: Depth,
        default_perft_depth: Depth,
    ) -> Self {
        Self {
            generic: GenericGoState {
                limit,
                is_first: pos.active_player().is_first(),
                multi_pv: 1,
                threads: None,
                search_type,
                complete: false,
                unique: false,
                move_overhead,
                strictness,
                override_hash_size: None,
                engine_name: None,
                default_bench_depth,
                default_perft_depth,
            },
            search_moves: None,
            pos,
        }
    }
    pub(super) fn start_time(&self) -> Instant {
        self.generic.limit.start_time
    }
}

pub(super) fn accept_depth(limit: &mut SearchLimit, words: &mut Tokens) -> Res<()> {
    if let Some(word) = words.peek() {
        if let Ok(number) = parse_int_from_str(word, "depth") {
            limit.depth = Depth::try_new(number)?;
            _ = words.next();
        }
    }
    Ok(())
}

pub(super) fn depth_cmd() -> Command {
    command!(depth | d, All, "Maximum search depth in plies (a.k.a. half-moves)", |state, words, _| {
        state.go_state_mut().limit_mut().depth = Depth::try_new(parse_int(words, "depth number")?)?;
        Ok(())
    })
}

pub(super) fn go_options<B: Board>(mode: Option<SearchType>) -> CommandList {
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
pub(super) fn go_options_impl(
    mode: Option<SearchType>,
    color_chars: [char; 2],
    color_names: [String; 2],
) -> CommandList {
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
                help_text: Some(format!("Remaining time in ms for {}", color_names[0])),
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
                help_text: Some(format!("Remaining time in ms for {}", color_names[1])),
                standard: All,
                autocomplete_recurse: false,
                func: |state, words, _| state.go_state_mut().set_time(words, false, false, "p2time"),
                sub_commands: SubCommandsFn::default(),
            },
            Command {
                primary_name: format!("{}inc", color_chars[0]),
                other_names: ArrayVec::from_iter([format!("{}i", color_chars[0]), "p1inc".to_string()]),
                help_text: Some(format!("Increment in ms for {}", color_names[0])),
                standard: All,
                autocomplete_recurse: false,
                func: |state, words, _| state.go_state_mut().set_time(words, true, true, "p1inc"),
                sub_commands: SubCommandsFn::default(),
            },
            Command {
                primary_name: format!("{}inc", color_chars[1]),
                other_names: ArrayVec::from_iter([format!("{}i", color_chars[1]), "p2inc".to_string()]),
                help_text: Some(format!("Increment in ms for {}", color_names[1])),
                standard: All,
                autocomplete_recurse: false,
                func: |state, words, _| state.go_state_mut().set_time(words, false, true, "p2inc"),
                sub_commands: SubCommandsFn::default(),
            },
            command!(movestogo | mtg, All, "Full moves until the time control is reset", |state, words, _| {
                let moves_to_go: isize = parse_int(words, "'movestogo' number")?;
                if moves_to_go < 0 {
                    state.write_message(
                        Warning,
                        &format_args!("Negative 'movestogo' number ({moves_to_go}), ignoring this option"),
                    );
                } else {
                    state.go_state_mut().limit_mut().tc.moves_to_go = Some(moves_to_go as usize);
                }
                Ok(())
            }),
            command!(nodes | n, All, "Maximum number of nodes to search", |state, words, _| {
                state.go_state_mut().limit_mut().nodes = NodesLimit::new(parse_int(words, "node count")?)
                    .ok_or_else(|| anyhow!("node count can't be zero"))?;
                Ok(())
            }),
            command!(
                softnodes | sn,
                Custom,
                "Don't increase the depth after this limit has been reached",
                |state, words, _| {
                    state.go_state_mut().limit_mut().soft_nodes =
                        NodesLimit::new(parse_int(words, "soft nodes limit")?)
                            .ok_or_else(|| anyhow!("soft nodes limit can't be zero"))?;
                    Ok(())
                }
            ),
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
            command!(
                prove | proof | pr,
                Custom,
                "Do a proof number search to find a forced win, similar to using 'engine proof'", // TODO: Support 'prove loss' and 'proof draw'
                |state, _, _| state.go_state_mut().set_engine(&mut tokens("proof"))
            ),
            command!(
                engine | eng | e,
                Custom,
                "Use the given engine without modifying the current engine's state",
                |state, words, _| state.go_state_mut().set_engine(words)
            ),
        ];
        // this checks only the mode that `go_options` is called for, but it can be changed through args (eg `go perft`),
        // which is why there's another check when actually handling it. Still, the first check prevents it from showing up in completion suggestion.
        if mode.is_none_or(|m| [Bench, Perft].iter().contains(&m)) {
            res.push(command!(complete | all, Custom, "Run bench / perft on all bench positions", |state, _, _| {
                if ![Bench, Perft].contains(&state.go_state_mut().get_mut().search_type) {
                    bail!("The 'all' option can only be used with 'bench' or 'perft' searches")
                }
                state.go_state_mut().get_mut().complete = true;
                Ok(())
            }));
        }
        if mode.is_none_or(|m| m == Perft) {
            res.push(command!(unique, Custom, "Only count unique positions in perft", |state, _, _| {
                if state.go_state_mut().get_mut().search_type != Perft {
                    bail!("The 'all' option can only be used with 'perft' searches")
                }
                state.go_state_mut().get_mut().unique = true;
                Ok(())
            }));
        }
        res.append(&mut additional);
    }
    res
}

pub(super) fn query_options<B: Board>() -> CommandList {
    // TODO: See go_options, doesn't update the chars
    query_options_impl(B::default().color_chars())
}

pub(super) fn query_options_impl(color_chars: [char; 2]) -> CommandList {
    vec![
        command!(gameover, UgiNotUci, "Is the game over?", |ugi, _, _| {
            ugi.write_response(&matches!(ugi.status(), Run(Ongoing)).to_string())
        }),
        Command {
            primary_name: "p1turn".to_string(),
            other_names: ArrayVec::from_iter([format!("{}turn", color_chars[0])]),
            help_text: Some("Is it the first player's turn?".to_string()),
            standard: UgiNotUci,
            autocomplete_recurse: false,
            func: |ugi, _, _| ugi.write_is_player(true),
            sub_commands: SubCommandsFn::default(),
        },
        Command {
            primary_name: "p2turn".to_string(),
            other_names: ArrayVec::from_iter([format!("{}turn", color_chars[1])]),
            help_text: Some("Is it the second player's turn?".to_string()),
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

// only used for autocompletion
fn bool_options() -> CommandList {
    vec![
        command!(["on", "true", "1"], Custom, "Enable", |_, _, _| Ok(())),
        command!(["off", "false", "0"], Custom, "Disable", |_, _, _| Ok(())),
    ]
}

pub(super) fn position_options<B: Board>(pos: Option<&B>, accept_pos_word: bool) -> CommandList {
    let mut res = generic_go_options(accept_pos_word);
    for p in B::name_to_pos_map() {
        let c = Command {
            primary_name: p.short_name(),
            other_names: Default::default(),
            help_text: Some(p.description().unwrap_or(format!("Load a custom position called '{}'", p.short_name()))),
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

pub(super) fn move_command(recurse: bool) -> Command {
    pos_command!(
        moves | mv,
        All,
        "Apply moves to the specified position",
        |_, _, _| Ok(()),
        --> move |state| state.moves_subcmds(false, recurse)
    )
}

pub(super) fn moves_options<B: Board>(pos: &B, recurse: bool) -> CommandList {
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
            help_text: Some(format!("Play move '{}'", mov.compact_formatter(pos).to_string().bold())),
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

pub(super) fn coords_options<B: Board>(pos: &B, ac_coords: bool, only_occupied: bool) -> CommandList {
    let mut res = vec![];
    for c in pos.size().valid_coordinates() {
        if only_occupied && pos.is_empty(c) {
            continue;
        }
        let n = Name { short: c.to_string(), long: c.to_string(), description: None };
        let mut cmd = named_entity_to_command(&n);
        let piece = pos.colored_piece_on(c).colored_piece_type();
        if pos.is_empty(c) {
            cmd.help_text = Some("Currently empty".to_string());
        } else {
            cmd.help_text = Some(format!("Currently occupied by: {}", piece.name(&pos.settings()).as_ref()));
        }
        if ac_coords {
            cmd.sub_commands = SubCommandsFn(Some(Box::new(|state| state.coords_subcmds(false, false))))
        }
        res.push(cmd);
    }
    res
}

pub(super) fn piece_options<B: Board>(pos: &B) -> CommandList {
    let mut res = vec![];
    let settings = pos.settings();
    for p in ColPieceTypeOf::<B>::non_empty(&settings) {
        let name = p.name(&settings).as_ref().to_string();
        let n = Name { short: name.clone(), long: name.clone(), description: None };
        let mut cmd = named_entity_to_command(&n);
        let list = [p.to_char(Ascii, &settings), p.to_char(Unicode, &settings)];
        for c in list.iter().sorted().dedup() {
            cmd.other_names.push(c.to_string());
        }
        cmd.sub_commands = SubCommandsFn(Some(Box::new(|state| state.coords_subcmds(false, false))));
        res.push(cmd);
    }
    res
}

pub(super) fn named_entity_to_command(entity: &dyn NamedEntity) -> Command {
    let primary_name = entity.short_name();
    let mut other_names = ArrayVec::default();
    if !entity.long_name().eq_ignore_ascii_case(&primary_name) {
        other_names.push(entity.long_name());
    }
    Command {
        primary_name,
        other_names,
        help_text: entity.description(),
        standard: Custom,
        autocomplete_recurse: false,
        func: |_, _, _| Ok(()),
        sub_commands: SubCommandsFn::default(),
    }
}

pub(super) fn select_command<T: NamedEntity + ?Sized>(list: &[Box<T>]) -> CommandList {
    let mut res: CommandList = vec![];
    for entity in list {
        res.push(named_entity_to_command(entity.upcast()));
    }
    res
}

fn option_values(val: &EngineOptionType) -> CommandList {
    match &val {
        EngineOptionType::Check(_) => bool_options(),
        EngineOptionType::Spin(_) => vec![],
        EngineOptionType::Combo(c) => c
            .options
            .iter()
            .map(|o| {
                let name = Name::from_name(o);
                named_entity_to_command(&name)
            })
            .collect_vec(),
        EngineOptionType::Button => vec![],
        EngineOptionType::UString(_) => vec![],
    }
}

fn option_to_cmd(option: EngineOption, only_name: bool) -> Command {
    use fmt::Write;
    let val = option.value;
    let mut cmd = named_entity_to_command(&option.name);
    let mut text = cmd.help_text.unwrap_or_default();
    write!(&mut text, ". {0}{1}", "Current value: ".dimmed(), val.value_to_str()).unwrap();
    cmd.help_text = Some(text);
    let values: SubCommandFnT = Box::new(move |_| {
        let mut res = option_values(&val);
        let val2 = val.clone();
        let mut value = command!(value, Custom, "", |_,_,_| Ok(()), --> move |_| option_values(&val2));
        let name = option.name.clone();
        value.help_text = Some(format!("Set the value of the option '{name}'"));
        res.push(value);
        res
    });
    if !only_name {
        cmd.sub_commands = SubCommandsFn(Some(values));
    }
    cmd
}

pub(super) fn options_options(ac: &dyn AutoCompleteState, accept_name_word: bool, only_name: bool) -> CommandList {
    let options = ac.options();
    let mut res = options.iter().map(|o| option_to_cmd(o.clone(), only_name)).collect_vec();
    if accept_name_word {
        let cmd = command!(name | n, All, "Select an option name", |_, _, _| Ok(()), --> move |state| options_options(state, false, only_name));
        res.push(cmd);
    }
    res
}
