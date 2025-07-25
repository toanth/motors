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
mod ascii_art;
mod autocomplete;
pub mod cli;
mod command;
mod input;
pub mod ugi_output;

use crate::eval::Eval;
use crate::io::SearchType::*;
use crate::io::ascii_art::print_as_ascii_art;
use crate::io::cli::EngineOpts;
use crate::io::command::Standard::Custom;
use crate::io::command::{
    AbstractGoState, CommandList, GoState, accept_depth, go_options, query_options, ugi_commands,
};
use crate::io::input::Input;
use crate::io::ugi_output::{AbstractUgiOutput, UgiOutput, color_for_score, pretty_score, score_gradient, suffix_for};
use crate::search::multithreading::EngineWrapper;
use crate::search::tt::{DEFAULT_HASH_SIZE_MB, TT, TTEntry};
use crate::search::{EvalList, SearchParams, SearcherList, run_bench_with};
use crate::{create_engine_box_from_str, create_engine_from_str, create_eval_from_str, create_match};
use gears::MatchStatus::*;
use gears::ProgramStatus::{Quit, Run};
use gears::Quitting::QuitProgram;
use gears::cli::select_game;
use gears::colored::Color::Red;
use gears::colored::Colorize;
use gears::games::CharType::Ascii;
use gears::games::chess::UCI_CHESS960;
use gears::games::{AbstractPieceType, BoardHistDyn, Color, ColoredPiece, ColoredPieceType, OutputList};
use gears::general::board::Strictness::{Relaxed, Strict};
use gears::general::board::{Board, BoardHelpers, ColPieceTypeOf, Strictness, Symmetry, UnverifiedBoard};
use gears::general::common::Description::{NoDescription, WithDescription};
use gears::general::common::anyhow::{anyhow, bail, ensure};
use gears::general::common::{
    NamedEntity, parse_bool_from_str, parse_duration_ms, parse_int_from_str, select_name_static, tokens,
    tokens_to_string,
};
use gears::general::common::{Res, Tokens};
use gears::general::moves::ExtendedFormat::{Alternative, Standard};
use gears::general::moves::Move;
use gears::general::perft::{SplitPerftRes, num_unique_positions_up_to, perft_for, split_perft};
use gears::itertools::Itertools;
use gears::output::Message::*;
use gears::output::logger::LoggerBuilder;
use gears::output::pgn::parse_pgn;
use gears::output::text_output::{AdaptFormatter, display_color};
use gears::output::{Message, OutputBox, OutputBuilder, OutputOpts};
use gears::rand::rng;
use gears::score::{Score, ScoreT};
use gears::search::{DepthPly, SearchLimit, TimeControl};
use gears::ugi::EngineOptionName::*;
use gears::ugi::EngineOptionType::*;
use gears::ugi::Protocol::{Interactive, UGI};
use gears::ugi::{
    EngineOption, EngineOptionName, EngineOptionNameForProto, Protocol, UgiCheck, UgiCombo, UgiSpin, UgiString,
    load_ugi_pos_simple,
};
use gears::{
    AbstractRun, AbstractUgiPosState, GameState, MatchState, MatchStatus, PlayerResult, ProgramStatus, Quitting,
    UgiPosState, output_builder_from_str,
};
use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter, Write};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::atomic::Ordering;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::sleep;
use std::time::{Duration, Instant};
use std::{fmt, fs};
use strum::IntoEnumIterator;

const DEFAULT_MOVE_OVERHEAD_MS: u64 = 50;
const MIN_CONTEMPT: i64 = -1000;
const MAX_CONTEMPT: i64 = 1000;

// TODO: Ensure this conforms to <https://expositor.dev/uci/doc/uci-draft-1.pdf>

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[must_use]
enum SearchType {
    Normal,
    Ponder,
    Bench,
    Perft,
    SplitPerft,
    Auto, // Like `Normal`, but done in this thread (i.e., blocks) and plays the chosen move instantly
}

impl Display for SearchType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Normal => "normal",
                SearchType::Ponder => "ponder",
                Perft => "perft",
                SplitPerft => "split perft",
                Bench => "bench",
                Auto => "auto",
            }
        )
    }
}

#[derive(Debug)]
struct EngineGameState<B: Board> {
    match_state: MatchState<B>,
    go_state: GoState<B>,
    game_name: String,
    protocol: Protocol,
    debug_mode: bool,
    ponder_limit: Option<SearchLimit>,
    engine: EngineWrapper<B>,
    /// The `go` command can take an engine option, which starts a search with the given engine without changing the normal engine.
    /// This temporary engine is largely ignored for most functionality, and at most one engine is allowed to search at the same time.
    temp_engine: Option<EngineWrapper<B>>,
    /// This doesn't have to be the UGI engine name. It often isn't, especially when two engines with
    /// the same name play against each other, such as in a SPRT. It should ideally be unique
    /// (the `monitors` client ensures that, but another GUI might not).
    display_name: String,
    opponent_name: Option<String>,
}

impl<B: Board> EngineGameState<B> {
    fn is_currently_searching(&self) -> bool {
        self.engine.main_atomic_search_data().currently_searching()
            || self.temp_engine.as_ref().is_some_and(|e| e.main_atomic_search_data().currently_searching())
    }
}

impl<B: Board> GameState<B> for EngineGameState<B> {
    fn initial_pos(&self) -> &B {
        &self.match_state.pos_before_moves
    }

    fn get_board(&self) -> &B {
        &self.board
    }

    fn game_name(&self) -> &str {
        &self.game_name
    }

    fn board_hist(&self) -> &dyn BoardHistDyn {
        &self.board_hist
    }

    fn move_history(&self) -> &[B::Move] {
        &self.mov_hist
    }

    fn match_status(&self) -> MatchStatus {
        match self.status.clone() {
            Run(status) => status,
            Quit(_) => MatchStatus::aborted(),
        }
    }

    fn name(&self) -> &str {
        &self.display_name
    }

    fn event(&self) -> String {
        format!("{0} {1} match", self.name(), self.game_name())
    }

    fn site(&self) -> &str {
        "?"
    }

    fn player_name(&self, color: B::Color) -> Option<String> {
        if color == self.board.inactive_player() { Some(self.name().to_string()) } else { self.opponent_name.clone() }
    }

    fn time(&self, _color: B::Color) -> Option<TimeControl> {
        // Technically, we get the time with 'go', but we can't trust it for the other player,
        // and we don't really need this for ourselves while we're thinking
        None
    }

    fn thinking_since(&self, _color: B::Color) -> Option<Instant> {
        None
    }

    // The reason for doing this weird loop instead of just letting the engine thread print
    // the engine state is that printing multiple lines while waiting for input
    // from enquire (used in the interactive ui) causes weird formatting issues
    fn print_engine_state(&self) -> Res<String> {
        self.engine.get_engine_info().internal_state_description = None;
        self.engine.send_print(self.go_state.pos.clone())?;
        let start = Instant::now();
        loop {
            let description = self.engine.get_engine_info().internal_state_description.take();
            if let Some(description) = description {
                return Ok(description);
            }
            sleep(Duration::from_millis(10));
            if start.elapsed().as_millis() > 200 {
                bail!("Failed to show internal engine state (can't be used when the engine is currently searching)");
            }
        }
    }

    fn print_engine_state_for_move(&self, pos: &B, mov: B::Move) -> Res<String> {
        self.engine.get_engine_info().internal_state_description = None;
        self.engine.send_print_move(pos.clone(), mov)?;
        let start = Instant::now();
        loop {
            let description = self.engine.get_engine_info().internal_state_description.take();
            if let Some(description) = description {
                return Ok(description);
            }
            sleep(Duration::from_millis(10));
            if start.elapsed().as_millis() > 200 {
                bail!(
                    "Failed to show internal engine state for move (can't be used when the engine is currently searching)"
                );
            }
        }
    }
}

impl<B: Board> Deref for EngineGameState<B> {
    type Target = MatchState<B>;

    fn deref(&self) -> &Self::Target {
        &self.match_state
    }
}

impl<B: Board> DerefMut for EngineGameState<B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.match_state
    }
}

#[derive(Debug)]
struct AllCommands {
    // ugi: CommandList<EngineUGI<B>>,
    // go: CommandList<GoState<B>>,
    // query: CommandList<EngineUGI<B>>,
    ugi: CommandList,
    go: CommandList,
    query: CommandList,
}

/// Implements both UGI and UCI.
#[derive(Debug)]
pub struct EngineUGI<B: Board> {
    state: EngineGameState<B>,
    commands: AllCommands,
    output: Arc<Mutex<UgiOutput<B>>>,
    output_factories: Rc<OutputList<B>>,
    searcher_factories: Rc<SearcherList<B>>,
    eval_factories: Rc<EvalList<B>>,
    move_overhead: Duration,
    strictness: Strictness,
    multi_pv: usize,
    allow_ponder: bool,
    contempt: Score,
    respond_to_move: bool,
    failed_cmd: Option<String>,
}

impl<B: Board> AbstractRun for EngineUGI<B> {
    fn run(&mut self) -> Quitting {
        // this can happen if the user ran a command via cli argument before starting the uci loop
        if let Quit(quitting) = &self.state.status {
            return *quitting;
        }
        self.ugi_loop()
    }

    fn handle_input(&mut self, input: &str) -> Res<()> {
        handle_ugi_input(self, tokens(input), &B::game_name())
    }

    fn quit(&mut self) -> Res<()> {
        self.handle_quit(QuitProgram)
    }
}

// A free function so that it does not depend on a generic parameter
fn handle_ugi_input(ugi: &mut dyn AbstractEngineUgi, mut words: Tokens, game_name: &str) -> Res<()> {
    // Set the start time as early as possible so that we don't overestimate the remaining time
    ugi.go_state_mut().limit_mut().start_time = Instant::now();
    ugi.write_ugi_input(words.clone());
    if ugi.fuzzing_mode() {
        ugi.write_ugi(&format_args!("Fuzzing input: [{}]", words.clone().join(" ")));
    }
    let words = &mut words;
    let Some(first_word) = words.next() else {
        return Ok(()); // ignore empty input
    };
    let cmd = match select_name_static(first_word, ugi.all_commands().ugi.iter(), "command", game_name, NoDescription) {
        Ok(cmd) => cmd,
        Err(err) => {
            let words_copy = words.clone();
            // These input options are not autocompleted (except for moves, which is done separately) and not selected using commands.
            // This allows precomputing commands, resolves potential conflicts with commands, and speeds up autocompletion
            if ugi.handle_move_fen_or_pgn(first_word, words)? {
                return Ok(());
            } else if first_word.eq_ignore_ascii_case("barbecue") {
                ugi.write_ugi_msg(&print_as_ascii_art("lol", 2));
            }
            ugi.write_message(
                Warning,
                &format_args!("{}", invalid_command_msg(ugi.is_interactive(), first_word, words, &err.to_string())),
            );
            ugi.set_failed_cmd(Some(tokens_to_string(first_word, words_copy)));
            return Ok(());
        }
    };

    // this does all the actual work of executing the command
    () = cmd.func()(ugi, words, first_word)?;

    if let Some(remaining) = words.next() {
        // can't reuse cmd because the borrow checker complains
        let cmd = select_name_static(first_word, ugi.all_commands().ugi.iter(), "command", game_name, NoDescription)?;
        ugi.write_message(
            Warning,
            &format_args!(
                "Ignoring trailing input starting with '{0}' after a valid '{1}' command",
                remaining.bold().red(),
                cmd.short_name().bold()
            ),
        );
    }
    Ok(())
}

impl<B: Board> EngineUGI<B> {
    pub fn create(
        opts: EngineOpts,
        mut selected_output_builders: OutputList<B>,
        all_output_builders: OutputList<B>,
        all_searchers: SearcherList<B>,
        all_evals: EvalList<B>,
    ) -> Res<Self> {
        let output = Arc::new(Mutex::new(UgiOutput::new(opts.interactive, opts.debug)));
        let board = match opts.pos_name {
            None => B::default(),
            Some(name) => load_ugi_pos_simple(&name, Relaxed, &B::default())?,
        };
        let engine = create_engine_from_str(&opts.engine, &all_searchers, &all_evals, output.clone(), TT::default())?;
        let display_name = engine.get_engine_info().short_name();
        let board_state = MatchState::new(board.clone());
        let protocol = if opts.interactive { Interactive } else { UGI };
        let move_overhead = Duration::from_millis(DEFAULT_MOVE_OVERHEAD_MS);
        let state = EngineGameState {
            match_state: board_state,
            go_state: GoState::new_for_pos(
                board,
                SearchLimit::infinite(),
                Relaxed,
                move_overhead,
                Normal,
                DepthPly::new(1),
                DepthPly::new(1),
            ),
            game_name: B::game_name(),
            protocol,
            debug_mode: opts.debug,
            ponder_limit: None,
            engine,
            temp_engine: None,
            display_name,
            opponent_name: None,
        };
        let err_msg_builder = output_builder_from_str("error", &all_output_builders)?;
        selected_output_builders.push(err_msg_builder);
        for builder in &mut selected_output_builders {
            output.lock().unwrap().additional_outputs.push(builder.for_engine(&state)?);
        }
        let settings = state.pos().settings_ref();
        let mut res = Self {
            state,
            commands: AllCommands {
                ugi: ugi_commands(),
                go: go_options::<B>(None, settings),
                query: query_options::<B>(),
            },
            output,
            output_factories: Rc::new(all_output_builders),
            searcher_factories: Rc::new(all_searchers),
            eval_factories: Rc::new(all_evals),
            move_overhead: Duration::from_millis(DEFAULT_MOVE_OVERHEAD_MS),
            strictness: Relaxed,
            multi_pv: 1,
            allow_ponder: false,
            contempt: Score(0),
            respond_to_move: true,
            failed_cmd: None,
        };
        if res.debug_mode() {
            res.handle_debug(&mut tokens(""))?;
        }
        if let Some(cmd) = opts.cmd {
            res.handle_input(&cmd)?;
        }
        Ok(res)
    }

    fn output(&self) -> MutexGuard<'_, UgiOutput<B>> {
        self.output.lock().unwrap()
    }

    fn ugi_loop(&mut self) -> Quitting {
        let game_name = B::game_name();
        self.write_message(Debug, &format_args!("Starting UGI loop (playing {})", &game_name));
        let text = format!("Motors: {}", self.state.game_name());
        let text = print_as_ascii_art(&text, 2);
        self.write_ugi(&format_args!("{}", text.dimmed()));
        self.write_engine_ascii_art();
        self.write_ugi(&format_args!(
            "[Type '{}' to change how the game state is displayed{}]",
            "output".bold(),
            ", e.g., 'output pretty' or 'output chess'".dimmed()
        ));
        if self.fuzzing_mode() {
            self.write_message(Warning, &format_args!("{}", "Fuzzing Mode Enabled!".bold()));
        }

        let (mut input, is_interactive) = Input::new(self.state.protocol == Interactive, self);
        if self.state.protocol == Interactive && !is_interactive {
            self.state.protocol = UGI; // Will be overwritten shortly, and isn't really used much anyway
        }
        loop {
            input.set_interactive(self.state.protocol == Interactive, self);
            let settings = self.state.pos().settings_ref();
            if settings != self.state.go_state.pos.settings_ref() {
                // make sure that after updating the variant in fairy, wtime/btime names are updated (e.g. to xtime/otime)
                self.commands.go = go_options::<B>(None, settings);
            }
            self.state.go_state.pos = self.state.pos().clone(); // set here because it's needed for autocompletion
            let input = match input.get_line(self) {
                Ok(input) => input,
                Err(err) => {
                    self.write_message(Error, &format_args!("{err}"));
                    break;
                }
            };
            self.failed_cmd = None;
            let res = handle_ugi_input(self, tokens(&input), &game_name);
            match res {
                Ok(()) => {
                    if let Quit(quitting) = &self.state.status {
                        return *quitting;
                    }
                }
                Err(err) => {
                    self.write_message(Error, &format_args!("{err}"));
                    if !self.continue_on_error() {
                        self.write_ugi(&format_args!("info error {err}"));
                    }
                    self.failed_cmd = Some(input);
                    // explicitly check this here so that continuing on error doesn't prevent us from quitting.
                    if let Quit(quitting) = self.state.status {
                        return quitting;
                    }
                    if self.continue_on_error() {
                        let interactive = if self.is_interactive() { "on" } else { "off" };
                        self.write_message(Debug, &format_args!("Continuing... (interactive mode is {interactive})"));
                        continue;
                    }
                    return QuitProgram;
                }
            }
        }
        QuitProgram
    }

    fn write_message(&mut self, typ: Message, msg: &fmt::Arguments) {
        self.output().write_message(typ, msg);
    }

    fn continue_on_error(&self) -> bool {
        self.state.debug_mode || self.state.protocol == Interactive
    }

    fn handle_move_input(&mut self, first_word: &str, rest: &mut Tokens) -> Res<bool> {
        let Ok(mov) = B::Move::from_text(first_word, &self.state.board) else {
            return Ok(false);
        };
        let mut state = self.state.clone();
        state.make_move(mov, true)?;
        let single_move = rest.peek().is_none();
        for word in rest {
            let mov = B::Move::from_text(word, &state.board)?;
            state.make_move(mov, true)?;
        }
        self.state.match_state = state;
        if print_game_over(self, true) {
            return Ok(true);
        }
        if single_move && self.respond_to_move {
            self.play_engine_move(None)?;
        }
        Ok(true)
    }

    fn play_engine_move(&mut self, params: Option<SearchParams<B>>) -> Res<()> {
        if self.is_interactive() {
            self.write_ugi_msg("Searching...");
        }
        let engine_name = self.state.engine.get_engine_info().short_name();
        let mut engine = create_engine_box_from_str(&engine_name, &self.searcher_factories, &self.eval_factories)?;
        let params = params.unwrap_or_else(|| {
            let limit = SearchLimit::per_move(Duration::from_millis(1_000));
            SearchParams::new_unshared(self.state.board.clone(), limit, self.state.board_hist.clone(), TT::default())
        });
        ensure!(
            !params.limit.is_infinite(),
            "Cannot use an infinite limit for this search, as that would block forever (can't use 'stop')"
        );
        let res = engine.search(params);
        ensure!(!res.chosen_move.is_null(), "Engine {engine_name} did not choose a move");
        if self.is_interactive() {
            self.write_ugi(&format_args!(
                "Playing move: {}",
                &res.chosen_move.to_extended_text(&self.state.board, Alternative).bold()
            ));
        }
        self.state.make_move(res.chosen_move, true)?;
        if self.is_interactive() {
            _ = print_game_over(self, false);
        }
        Ok(())
    }

    fn id(&self) -> String {
        let info = self.state.engine.get_engine_info();
        format!(
            "id name Motors -- Game {0} -- Engine {1}\nid author ToTheAnd",
            self.state.game_name(),
            info.long_name(),
        )
    }

    fn set_option(&mut self, name: EngineOptionNameForProto, value: String) -> Res<()> {
        match name.name {
            EngineOptionName::Ponder => {
                self.allow_ponder = parse_bool_from_str(&value, "ponder")?;
            }
            MoveOverhead => {
                self.move_overhead = parse_duration_ms(&mut tokens(&value), "move overhead")?;
            }
            MultiPv => {
                self.multi_pv = parse_int_from_str(&value, "multipv")?;
            }
            UCIChess960 => UCI_CHESS960.store(parse_bool_from_str(&value, "UCI_Chess960")?, SeqCst),
            UCIVariant => self.set_variant(&mut tokens(&value))?,
            UCIOpponent => {
                let mut words = value.split_whitespace();
                loop {
                    match words.next() {
                        None => {
                            break;
                        }
                        Some(word) if word.eq_ignore_ascii_case("computer") || word.eq_ignore_ascii_case("human") => {
                            self.state.opponent_name = Some(words.join(" "));
                            break;
                        }
                        _ => continue,
                    }
                }
            }
            UCIShowRefutations => self.output().show_refutation = parse_bool_from_str(&value, "show refutations")?,
            UCIShowCurrLine => {
                self.output().show_currline = parse_bool_from_str(&value, "show current line")?;
            }
            CurrlineNullmove => {
                self.output().currline_null_moves = parse_bool_from_str(&value, "show nullmoves in `currline`")?;
            }
            Minimal => self.output().minimal = parse_bool_from_str(&value, "minimal UCI output")?,
            Strictness => {
                self.strictness = if parse_bool_from_str(&value, "strictness")? { Strict } else { Relaxed };
            }
            RespondToMove => self.respond_to_move = parse_bool_from_str(&value, "respond to move")?,
            Contempt => {
                self.contempt.0 =
                    parse_int_from_str::<i64>(&value, "contempt")?.clamp(MIN_CONTEMPT, MAX_CONTEMPT) as ScoreT;
            }
            SetEngine => {
                self.handle_engine(&mut tokens(&value))?;
            }
            SetEval => {
                self.handle_set_eval(&mut tokens(&value))?;
            }
            Hash | Threads | UciElo | UCIEngineAbout | Other(_) => {
                let value = value.trim().to_string();
                self.state
                    .engine
                    .set_option(name.clone(), value.clone())
                    .or_else(|err| if name.name == Threads && value == "1" { Ok(()) } else { Err(err) })?;
            }
        }
        Ok(())
    }

    fn handle_setoption_impl(&mut self, words: &mut Tokens) -> Res<()> {
        if words.peek().is_some_and(|w| w.eq_ignore_ascii_case("name") || w.eq_ignore_ascii_case("n")) {
            _ = words.next();
        }
        let mut name = String::default();
        loop {
            let next_word = words.next().unwrap_or_default();
            if next_word.is_empty() {
                // input didn't contain 'value', so assume the first token is the name and the rest is the value
                let mut words = tokens(&name);
                let name = words.next().unwrap_or_default();
                let value = words.join(" ");
                let name = EngineOptionNameForProto::parse(name.trim(), self.protocol())?;
                return self.set_option(name, value);
            }
            if next_word.eq_ignore_ascii_case("value") {
                break;
            }
            name = name + " " + next_word;
        }
        let mut value = words.next().unwrap().to_string();
        loop {
            let next_word = words.next().unwrap_or_default();
            if next_word.is_empty() {
                break;
            }
            value = value + " " + next_word;
        }
        let name = EngineOptionNameForProto::parse(name.trim(), self.protocol()).unwrap();
        self.set_option(name, value)
    }

    fn handle_go_impl(&mut self, initial_search_type: SearchType, words: &mut Tokens) -> Res<()> {
        self.state.go_state = GoState::new(self, initial_search_type, self.state.go_state.start_time());

        if matches!(initial_search_type, Perft | SplitPerft | Bench) {
            accept_depth(self.go_state_mut().limit_mut(), words)?;
        }
        while let Some(option) = words.next() {
            let cmd = select_name_static(
                option,
                self.commands.go.iter(),
                "go option",
                &self.state.game_name,
                WithDescription,
            )?;
            cmd.func()(self, words, option)?;
        }
        let opts = &mut self.state.go_state;
        let limit = &mut opts.generic.limit;
        let remaining = &mut limit.tc.remaining;
        *remaining = remaining.saturating_sub(opts.generic.move_overhead).max(Duration::from_micros(500));

        if cfg!(feature = "fuzzing") {
            limit.fixed_time = limit.fixed_time.max(Duration::from_secs(1));
            if opts.generic.complete {
                limit.fixed_time = Duration::from_millis(10);
            }
            if matches!(opts.generic.search_type, Perft | SplitPerft) {
                let depth = if opts.generic.complete { 2 } else { 3 };
                limit.depth = limit.depth.min(DepthPly::new(depth));
            }
        }

        if opts.generic.complete && !matches!(opts.generic.search_type, Bench | Perft) {
            bail!(
                "The '{0}' options can only be used for '{1}' and '{2}' searches",
                "complete".bold(),
                "bench".bold(),
                "perft".bold()
            )
        }

        let opts = &self.state.go_state.generic;
        let limit = opts.limit;
        let board = self.state.go_state.pos.clone();
        match opts.search_type {
            Auto => {
                let params = SearchParams::with_output(
                    board.clone(),
                    limit,
                    self.state.board_hist.clone(),
                    self.state.engine.next_tt(),
                    self.state.go_state.search_moves.take(),
                    opts.multi_pv.saturating_sub(1),
                    self.output.clone(),
                    self.state.engine.get_engine_info_arc(),
                );
                return self.play_engine_move(Some(params));
            }
            Bench => {
                let bench_positions: Vec<B> = if opts.complete { B::bench_positions() } else { vec![board] };
                return self.bench(limit, &bench_positions);
            }
            Perft => {
                let positions = if opts.complete { B::bench_positions() } else { vec![board.clone()] };
                let threads = opts.threads.unwrap_or(0);
                if threads > 1 {
                    bail!("For 'perft' runs, the 'Threads' options can only be used to set threads to 1")
                }
                for i in 1..=limit.depth.get() {
                    if opts.unique {
                        self.output().write_ugi(&format_args!(
                            "# unique positions at depth {i}: {}",
                            num_unique_positions_up_to(DepthPly::new(i), board.clone()).to_string().bold()
                        ))
                    } else {
                        self.output()
                            .write_ugi(&format_args!("{}", perft_for(DepthPly::new(i), &positions, threads != 1)))
                    }
                }
            }
            SplitPerft => {
                if limit.depth.get() == 0 {
                    bail!("{} requires a depth of at least 1", "splitperft".bold())
                }
                let threads = opts.threads.unwrap_or(0);
                if threads > 1 {
                    bail!("For 'splitperft' runs, the 'Threads' options can only be used to set threads to 1")
                }
                let res = split_perft(limit.depth, board, threads != 1);
                self.write_ugi(&format_args!("{res}"));
                if self.go_state_mut().get_mut().compare {
                    compare_splitperft(self, res)?;
                }
            }
            _ => return self.start_search(),
        }
        Ok(())
    }

    fn start_search(&mut self) -> Res<()> {
        let opts = self.state.go_state.generic.clone();
        let tt = opts.override_hash_size.map(TT::new_with_mib);
        self.write_message(Debug, &format_args!("Starting {0} search with limit {1}", opts.search_type, opts.limit));
        let pos = self.state.go_state.pos.clone();
        if let Some(res) = pos.match_result_slow(&self.state.board_hist) {
            self.write_message(
                Warning,
                &format_args!(
                    "Starting a {3} search in position '{2}', but the game is already over. {0} ({1}).",
                    res.result,
                    res.reason,
                    pos.as_fen().bold(),
                    opts.search_type
                ),
            );
        }
        self.state.set_status(Run(Ongoing));
        let hist = self.state.board_hist.clone();
        let search_moves = self.state.go_state.search_moves.take();
        // Stop the temporary search, if it exists. This could take some time, but that's fine since there won't be a temporary engine
        // unless the user specifically requested it.
        self.state.temp_engine = None;
        let engine = if let Some(name) = &opts.engine_name {
            let temp_engine = create_engine_from_str(
                name,
                &self.searcher_factories,
                &self.eval_factories,
                self.output.clone(),
                tt.clone().unwrap_or_default(), // use default (i.e small) TT size to avoid using too much memory by mistake
            )?;
            self.state.temp_engine = Some(temp_engine);
            self.state.temp_engine.as_mut().unwrap()
        } else {
            &mut self.state.engine
        };
        match opts.search_type {
            // this keeps the current history even if we're searching a different position, but that's probably not a problem
            Normal => {
                // It doesn't matter if we got a ponderhit or a miss, we simply abort the ponder search and start a new search.
                if self.state.ponder_limit.is_some() && opts.engine_name.is_none() {
                    self.state.ponder_limit = None;
                    engine.send_stop(true); // aborts the pondering without printing a search result
                }
                engine.start_search(
                    pos,
                    opts.limit,
                    hist,
                    search_moves,
                    opts.multi_pv,
                    false,
                    opts.threads,
                    tt,
                    self.contempt,
                )?;
            }
            SearchType::Ponder => {
                if opts.engine_name.is_none() {
                    self.state.ponder_limit = Some(opts.limit);
                }
                engine.start_search(
                    pos,
                    SearchLimit::infinite(), //always allocate infinite time for pondering
                    hist,
                    search_moves,
                    opts.multi_pv, // don't ignore multi_pv in pondering mode
                    true,
                    opts.threads,
                    tt,
                    self.contempt,
                )?;
            }
            _ => unreachable!("Bench and (Split)Perft should have already been handled"),
        };
        Ok(())
    }

    fn bench(&mut self, limit: SearchLimit, positions: &[B]) -> Res<()> {
        let mut engine = create_engine_box_from_str(
            &self.state.engine.get_engine_info().short_name(),
            &self.searcher_factories,
            &self.eval_factories,
        )?;
        let second_limit = if positions.len() == 1 {
            None
        } else {
            let mut limit = limit;
            limit.depth = DepthPly::MAX;
            limit.nodes = self.state.engine.get_engine_info().default_bench_nodes();
            Some(limit)
        };
        let tt = self.state.go_state.generic.override_hash_size.map(TT::new_with_mib);
        let res = run_bench_with(engine.as_mut(), limit, second_limit, positions, tt);
        self.output().write_ugi(&format_args!("{res}"));
        Ok(())
    }

    #[cold]
    fn handle_eval_or_tt_impl(&mut self, eval: bool, words: &mut Tokens) -> Res<()> {
        let mut state = self.state.clone();
        if words.peek().is_some() {
            state.handle_position(words, true, Relaxed, true, false)?;
        }
        let text = if eval {
            let info = self.state.engine.get_engine_info();
            if let Some(eval_name) = info.eval() {
                let mut eval = create_eval_from_str(&eval_name.short_name(), &self.eval_factories)?.build();
                let eval_score = eval.eval(&state.board, 0, state.board.active_player());
                if self.is_interactive() {
                    let diagram = show_eval_pos(&state.board, state.last_move(), eval);
                    diagram
                        + &format!(
                            "Eval Score: {}\n",
                            pretty_score(eval_score, None, None, &score_gradient(), true, false)
                        )
                } else {
                    eval_score.0.to_string()
                }
            } else {
                format!("The engine '{}' doesn't have an eval function", info.short_name().bold())
            }
        } else if let Some(entry) = self.state.engine.tt_entry(&state.board) {
            format_tt_entry(state, entry)
        } else {
            "There is no TT entry for this position".bold().to_string()
        };
        self.write_ugi_msg(&text);
        Ok(())
    }

    #[cold]
    fn handle_query_impl(&mut self, words: &mut Tokens) -> Res<()> {
        let query = *words.peek().ok_or(anyhow!("Missing argument to '{}'", "query".bold()))?;
        match select_name_static(
            query,
            self.commands.query.iter(),
            "query option",
            self.state.game_name(),
            WithDescription,
        ) {
            Ok(cmd) => {
                _ = words.next();
                cmd.func()(self, words, query)
            }
            Err(err) => {
                if let Ok(opt) = self.options_text(words) {
                    self.write_ugi(&format_args!("{opt}"));
                    Ok(())
                } else {
                    bail!("{err}\nOr the name of an option.")
                }
            }
        }
    }

    fn select_output(&self, words: &mut Tokens) -> Res<Option<OutputBox<B>>> {
        let name = words.peek().copied().unwrap_or_default();
        let output = output_builder_from_str(name, &self.output_factories);
        match output {
            Ok(mut output) => {
                _ = words.next();
                Ok(Some(output.for_engine(&self.state)?))
            }
            Err(_) => {
                if self.output().additional_outputs.iter().any(|o| o.prints_board() && !o.is_logger()) {
                    Ok(None)
                } else {
                    // Even though "pretty" can look better than "prettyascii", it's also significantly more risky
                    // because how it looks very much depends on the terminal.
                    self.select_output(&mut tokens("prettyascii"))
                }
            }
        }
    }

    #[cold]
    fn handle_print_impl(&mut self, words: &mut Tokens, opts: OutputOpts) -> Res<()> {
        let output = self.select_output(words)?;
        let print = |this: &Self, output: Option<OutputBox<B>>, state| match output {
            None => {
                this.output().show(state, opts);
            }
            Some(mut output) => {
                output.show(state, opts);
            }
        };
        if words.peek().is_some() {
            let old_state = self.state.match_state.clone();
            if let Err(err) = self.state.handle_position(words, true, self.strictness, true, false) {
                self.state.match_state = old_state;
                return Err(err);
            }
            print(self, output, &self.state);
            self.state.match_state = old_state;
        } else {
            print(self, output, &self.state);
        }
        Ok(())
    }

    #[cold]
    fn handle_output_impl(&mut self, words: &mut Tokens) -> Res<()> {
        let mut next = words.next().unwrap_or_default();
        let output_ptr = self.output.clone();
        let mut output = output_ptr.lock().unwrap();
        if next.eq_ignore_ascii_case("remove") || next.eq_ignore_ascii_case("clear") {
            let next = words.next().unwrap_or("all");
            if next.eq_ignore_ascii_case("all") {
                output.additional_outputs.retain(|o| !o.prints_board());
            } else {
                output.additional_outputs.retain(|o| !o.short_name().eq_ignore_ascii_case(next));
            }
        } else if !output.additional_outputs.iter().any(|o| o.short_name().eq_ignore_ascii_case(next)) {
            if next.eq_ignore_ascii_case("add") {
                next = words.next().ok_or_else(|| anyhow!("Missing output name after 'add'"))?;
            } else {
                output.additional_outputs.retain(|o| !o.prints_board())
            }
            output.additional_outputs.push(
                output_builder_from_str(next, &self.output_factories)
                    .map_err(|err| {
                        anyhow!("{err}\nSpecial commands are '{0}' and '{1}'.", "remove".bold(), "add".bold())
                    })?
                    .for_engine(&self.state)?,
            );
            if self.is_interactive() {
                drop(output);
                self.print_board(OutputOpts::default());
            }
        }
        Ok(())
    }

    #[cold]
    fn handle_debug_impl(&mut self, words: &mut Tokens) -> Res<()> {
        match words.next().unwrap_or("on") {
            "on" => {
                self.state.debug_mode = true;
                // make sure to print all the messages that can be sent (adding an existing output is a no-op)
                self.handle_output(&mut tokens("error"))?;
                self.handle_output(&mut tokens("debug"))?;
                self.handle_output(&mut tokens("info"))?;
                self.output().set_debug(true);
                self.write_message(Debug, &format_args!("Debug mode enabled"));
                // don't change the log stream if it's already set
                if self.output().additional_outputs.iter().any(|o| o.is_logger()) {
                    Ok(())
                } else {
                    // In case of an error here, still keep the debug mode set.
                    self.handle_log(&mut tokens(""))
                        .map_err(|err| anyhow!("Couldn't set the debug log file: '{err}'"))?;
                    Ok(())
                }
            }
            "off" => {
                self.state.debug_mode = false;
                _ = self.handle_output(&mut tokens("remove debug"));
                _ = self.handle_output(&mut tokens("remove info"));
                self.write_message(Debug, &format_args!("Debug mode disabled"));
                self.output().set_debug(false);
                // don't remove the error output, as there is basically no reason to do so
                self.handle_log(&mut tokens("none"))
            }
            x => bail!("Invalid debug option '{x}'"),
        }
    }

    // This function doesn't depend on the generic parameter, and luckily the rust compiler is smart enough to
    // polymorphize the monomorphed functions again,i.e. it will only generate this function once. So no need to
    // manually move it into a context where it doesn't depend on `B`.
    #[cold]
    fn handle_log_impl(&mut self, words: &mut Tokens) -> Res<()> {
        self.output().additional_outputs.retain(|o| !o.is_logger());
        let next = words.peek().copied().unwrap_or_default();
        if next != "off" && next != "none" {
            let logger = LoggerBuilder::from_words(words).for_engine(&self.state)?;
            self.output().additional_outputs.push(logger);
        }
        // write the debug message after setting the logger so that it also gets logged.
        let name = self.output().additional_outputs.last().unwrap().output_name();
        self.write_message(Debug, &format_args!("Set the debug logfile to '{name}'",));
        Ok(())
    }

    #[cold]
    fn handle_engine_impl(&mut self, words: &mut Tokens) -> Res<()> {
        let Some(engine) = words.next() else {
            let info = self.state.engine.get_engine_info();
            let short = info.short_name();
            let long = info.long_name();
            let description = info.description().unwrap_or_default();
            drop(info);
            self.write_ugi(&format_args!(
                "\n{alias}: {short}\n{engine}: {long}\n{descr}: {description}",
                alias = "Alias".bold(),
                engine = "Engine".bold(),
                descr = "Description".bold(),
            ));
            return Ok(());
        };
        if let Some(game) = words.next() {
            return self.handle_play(&mut tokens(&format!("{game} {engine}")));
        }
        // catch invalid names before committing to shutting down the current engine
        let engine = create_engine_from_str(
            engine,
            &self.searcher_factories,
            &self.eval_factories,
            self.output.clone(),
            TT::new_with_mib(self.state.engine.next_tt().size_in_mib()),
        )?;
        let hash = self.state.engine.next_tt().size_in_mib();
        let threads = self.state.engine.num_threads();
        self.state.engine.send_quit()?;
        // This resets some engine options, but that's probably for the better since the new engine might not support those.
        // However, we make an exception for threads and hash
        self.state.engine = engine;
        // We set those options after changing the engine, so if we get an error this doesn't prevent us from using the new engine.
        let h = EngineOptionNameForProto { name: Hash, proto: self.protocol() };
        self.state.engine.set_option(h, hash.to_string())?;
        let t = EngineOptionNameForProto { name: Threads, proto: self.protocol() };
        self.state.engine.set_option(t, threads.to_string())?;
        self.write_engine_ascii_art();
        Ok(())
    }

    #[cold]
    fn handle_set_eval_impl(&mut self, words: &mut Tokens) -> Res<()> {
        let Some(name) = words.next() else {
            let name = self
                .state
                .engine
                .get_engine_info()
                .eval()
                .clone()
                .map_or_else(|| "<eval unused>".to_string(), |e| e.short_name());
            self.write_ugi(&format_args!("Current eval: {name}"));
            return Ok(());
        };
        let eval = create_eval_from_str(name, &self.eval_factories)?.build();
        self.state.engine.set_eval(eval)?;
        self.write_engine_ascii_art();
        Ok(())
    }

    fn set_variant(&mut self, words: &mut Tokens) -> Res<()> {
        let first = words.next().unwrap_or_default();
        self.state.match_state.handle_variant(first, words, self.protocol())
    }

    fn write_ugi_options(&self) -> String {
        self.get_options().iter().map(|opt| format!("option {opt}")).collect::<Vec<String>>().join("\n")
    }

    fn get_options(&self) -> Vec<EngineOption> {
        let engine_info = self.state.engine.get_engine_info();
        let engine = engine_info.engine().clone();
        let eval_name = engine_info.eval().as_ref().map(|i| i.short_name()).unwrap_or("<none>".to_string());
        let eval_long_name = engine_info.eval().as_ref().map(|e| e.long_name()).unwrap_or("<none>".to_string());
        let max_threads = engine_info.max_threads();
        drop(engine_info);
        // use a match to ensure at compile time we're not missing any option
        let mut res = vec![];
        for opt in EngineOptionName::iter() {
            let value = match opt {
                Hash => Spin(UgiSpin {
                    val: self.state.engine.next_tt().size_in_mib() as i64,
                    default: Some(DEFAULT_HASH_SIZE_MB as i64),
                    min: Some(0),
                    max: Some(10_000_000), // use at most 10 terabytes (should be enough for anybody™)
                }),
                Threads => Spin(UgiSpin {
                    val: self.state.engine.num_threads() as i64,
                    default: Some(1),
                    min: Some(1),
                    max: Some(max_threads as i64),
                }),
                EngineOptionName::Ponder => Check(UgiCheck { val: self.allow_ponder, default: Some(false) }),
                MultiPv => Spin(UgiSpin { val: 1, default: Some(1), min: Some(1), max: Some(256) }),
                // We accept chess960 positions even if the option is false, but we want to treat startpos as non-chess960 by default
                UCIChess960 => Check(UgiCheck { val: UCI_CHESS960.load(Ordering::Relaxed), default: Some(false) }),
                UCIVariant => {
                    if let Some(variants) = B::list_variants() {
                        Combo(UgiCombo {
                            val: variants.first().cloned().unwrap_or("<default>".to_string()),
                            default: variants.first().cloned(),
                            options: variants,
                        })
                    } else {
                        continue;
                    }
                }
                UciElo => continue, // currently not supported
                UCIOpponent => {
                    UString(UgiString { default: None, val: self.state.opponent_name.clone().unwrap_or_default() })
                }
                UCIEngineAbout => UString(UgiString {
                    val: String::new(),
                    default: Some(format!(
                        "Motors by ToTheAnd. Game: {2}. Engine: {0}. Eval: {1}  ",
                        engine.long,
                        eval_long_name,
                        self.state.game_name()
                    )),
                }),
                UCIShowRefutations => Check(UgiCheck { val: self.output().show_refutation, default: Some(false) }),
                UCIShowCurrLine => Check(UgiCheck { val: self.output().show_currline, default: Some(false) }),
                CurrlineNullmove => Check(UgiCheck { val: self.output().show_currline, default: Some(true) }),
                Minimal => Check(UgiCheck { val: self.output().minimal, default: Some(false) }),
                MoveOverhead => Spin(UgiSpin {
                    val: self.move_overhead.as_millis() as i64,
                    default: Some(DEFAULT_MOVE_OVERHEAD_MS as i64),
                    min: Some(0),
                    max: Some(10_000),
                }),
                Strictness => Check(UgiCheck { val: self.strictness == Strict, default: Some(false) }),
                RespondToMove => Check(UgiCheck { val: self.respond_to_move, default: Some(true) }),
                Contempt => Spin(UgiSpin {
                    val: self.contempt.0 as i64,
                    default: Some(0),
                    min: Some(MIN_CONTEMPT),
                    max: Some(MAX_CONTEMPT),
                }),
                SetEngine =>
                // We would like to send long names, but unfortunately GUIs struggle with that
                {
                    Combo(UgiCombo {
                        val: engine.short_name(),
                        default: Some(engine.short_name()),
                        options: self.searcher_factories.iter().map(|s| s.short_name()).collect_vec(),
                    })
                }
                SetEval => Combo(UgiCombo {
                    val: eval_name.clone(),
                    default: Some(eval_name.clone()),
                    options: self.eval_factories.iter().map(|e| e.short_name()).collect_vec(),
                }),
                Other(_) => continue,
            };
            let name = EngineOptionNameForProto { name: opt, proto: self.protocol() };
            res.push(EngineOption { name, value });
        }
        res.extend(self.state.engine.get_engine_info().additional_options());
        res
    }

    fn write_engine_ascii_art(&mut self) {
        if self.is_interactive() {
            let text = self.state.engine.get_engine_info().short_name();
            let text = print_as_ascii_art(&text, 2).cyan();
            self.write_ugi(&format_args!("{text}"));
        }
    }
}

/// This trait exists to allow erasing the type of the board where possible in order to reduce code bloat.
/// It is implemented both by `EngineUGI` and by the autocompletion state.
trait AbstractEngineUgiState: Debug {
    fn options_text(&self, words: &mut Tokens) -> Res<String>;

    fn write_ugi_msg(&mut self, msg: &str) {
        self.write_ugi(&format_args!("{msg}"))
    }

    fn write_ugi(&mut self, message: &fmt::Arguments);

    fn write_message(&mut self, message: Message, msg: &fmt::Arguments);

    fn write_response(&mut self, msg: &str) -> Res<()>;

    fn status(&self) -> &ProgramStatus;

    fn go_state_mut(&mut self) -> &mut dyn AbstractGoState;

    fn load_go_state_pos(&mut self, name: &str, words: &mut Tokens) -> Res<()>;

    fn handle_ugi(&mut self, proto: &str) -> Res<()>;

    fn handle_uginewgame(&mut self) -> Res<()>;

    fn handle_pos(&mut self, words: &mut Tokens) -> Res<()>;

    fn handle_go(&mut self, initial_search_type: SearchType, words: &mut Tokens) -> Res<()>;

    fn handle_stop(&mut self, suppress_best_move: bool) -> Res<()>;

    fn handle_ponderhit(&mut self) -> Res<()>;

    fn handle_setoption(&mut self, words: &mut Tokens) -> Res<()>;

    fn handle_interactive(&mut self) -> Res<()>;

    fn handle_debug(&mut self, words: &mut Tokens) -> Res<()>;

    fn handle_log(&mut self, words: &mut Tokens) -> Res<()>;

    fn handle_output(&mut self, words: &mut Tokens) -> Res<()>;

    fn handle_print(&mut self, words: &mut Tokens, opts: OutputOpts) -> Res<()>;

    fn handle_engine_print(&mut self) -> Res<()>;

    fn handle_move_eval(&mut self, words: &mut Tokens) -> Res<()>;

    fn handle_eval_or_tt(&mut self, eval: bool, words: &mut Tokens) -> Res<()>;

    fn handle_engine(&mut self, words: &mut Tokens) -> Res<()>;

    fn handle_set_eval(&mut self, words: &mut Tokens) -> Res<()>;

    fn load_pgn(&mut self, words: &mut Tokens) -> Res<()>;

    fn handle_flip(&mut self) -> Res<()>;

    fn handle_query(&mut self, words: &mut Tokens) -> Res<()>;

    fn handle_variant(&mut self, words: &mut Tokens) -> Res<()>;

    fn handle_wait(&mut self, words: &mut Tokens) -> Res<()>;

    fn handle_play(&mut self, words: &mut Tokens) -> Res<()>;

    fn handle_assist(&mut self, words: &mut Tokens) -> Res<()>;

    fn handle_undo(&mut self, words: &mut Tokens) -> Res<()>;

    fn handle_gb(&mut self, words: &mut Tokens) -> Res<()>;

    fn handle_place_piece(&mut self, words: &mut Tokens) -> Res<()>;

    fn handle_remove_piece(&mut self, words: &mut Tokens) -> Res<()>;

    fn handle_move_piece(&mut self, words: &mut Tokens) -> Res<()>;

    fn handle_randomize(&mut self, words: &mut Tokens) -> Res<()>;

    fn print_help(&mut self) -> Res<()>;

    fn write_is_player(&mut self, is_first: bool) -> Res<()>;

    fn respond_game(&mut self) -> Res<()>;

    fn respond_engine(&mut self) -> Res<()>;

    fn handle_quit(&mut self, typ: Quitting) -> Res<()>;
}

impl<B: Board> AbstractEngineUgiState for EngineUGI<B> {
    fn options_text(&self, words: &mut Tokens) -> Res<String> {
        write_options_impl(
            self.get_options(),
            &self.state.engine.get_engine_info().short_name(),
            &B::game_name(),
            words,
        )
    }

    fn write_ugi(&mut self, message: &fmt::Arguments) {
        self.output().write_ugi(message);
    }

    fn write_message(&mut self, message: Message, msg: &fmt::Arguments) {
        self.output().write_message(message, msg);
    }

    fn write_response(&mut self, msg: &str) -> Res<()> {
        // Part of the UGI specification, but not the UCI specification
        self.write_ugi(&format_args!("response {msg}"));
        Ok(())
    }

    fn status(&self) -> &ProgramStatus {
        &self.state.status
    }

    fn go_state_mut(&mut self) -> &mut dyn AbstractGoState {
        &mut self.state.go_state
    }

    fn load_go_state_pos(&mut self, name: &str, words: &mut Tokens) -> Res<()> {
        self.go_state_mut().load_pos(name, words, false)
    }

    fn handle_ugi(&mut self, proto: &str) -> Res<()> {
        self.state.protocol = Protocol::from_str(proto)?;
        let id_msg = self.id();
        self.write_ugi(&format_args!(
            "Starting {proto} mode. Type '{0}' or '{1}' for interactive mode.\n",
            "interactive".bold(),
            "i".bold()
        ));
        self.write_ugi_msg(&id_msg);
        self.write_ugi(&format_args!("{}", self.write_ugi_options().as_str()));
        self.write_ugi(&format_args!("{proto}ok"));
        self.output().set_pretty(self.state.protocol == Interactive);
        Ok(())
    }

    fn handle_uginewgame(&mut self) -> Res<()> {
        self.state.engine.send_forget()?;
        self.state.set_status(Run(NotStarted));
        Ok(())
    }

    fn handle_pos(&mut self, words: &mut Tokens) -> Res<()> {
        let check_game_over = self.state.debug_mode || self.is_interactive();
        let keep_hist = check_game_over;
        self.state.handle_position(words, false, self.strictness, check_game_over, keep_hist)?;
        if self.is_interactive() {
            //additional
            self.print_board(OutputOpts::default());
        }
        Ok(())
    }

    fn handle_go(&mut self, initial_search_type: SearchType, words: &mut Tokens) -> Res<()> {
        self.handle_go_impl(initial_search_type, words)
    }

    fn handle_stop(&mut self, suppress_best_move: bool) -> Res<()> {
        self.state.engine.send_stop(suppress_best_move);
        if let Some(tmp) = &mut self.state.temp_engine {
            tmp.send_stop(suppress_best_move);
        }
        Ok(())
    }

    fn handle_ponderhit(&mut self) -> Res<()> {
        let start_time = self.state.go_state.start_time();
        self.state.go_state = GoState::new(self, Normal, start_time);
        self.state.go_state.generic.limit = self
            .state
            .ponder_limit
            .ok_or_else(|| anyhow!("The engine received a '{}' command but wasn't pondering", "ponderhit".bold()))?;
        self.state.go_state.generic.limit.start_time = start_time;
        self.start_search()
    }

    fn handle_setoption(&mut self, words: &mut Tokens) -> Res<()> {
        self.handle_setoption_impl(words)
    }

    #[cold]
    fn handle_interactive(&mut self) -> Res<()> {
        self.state.protocol = Interactive;
        self.output().set_pretty(true);
        Ok(())
    }

    fn handle_debug(&mut self, words: &mut Tokens) -> Res<()> {
        self.handle_debug_impl(words)
    }

    fn handle_log(&mut self, words: &mut Tokens) -> Res<()> {
        self.handle_log_impl(words)
    }

    fn handle_output(&mut self, words: &mut Tokens) -> Res<()> {
        self.handle_output_impl(words)
    }

    fn handle_print(&mut self, words: &mut Tokens, opts: OutputOpts) -> Res<()> {
        self.handle_print_impl(words, opts)
    }

    #[cold]
    fn handle_engine_print(&mut self) -> Res<()> {
        let string = self.state.print_engine_state()?;
        self.write_ugi(&format_args!("{string}"));
        Ok(())
    }

    fn handle_move_eval(&mut self, words: &mut Tokens) -> Res<()> {
        let Some(mov) = words.next() else { bail!("Expected move after '{}' command", "move_eval".bold()) };
        let pos = self.state.pos();
        let mov = B::Move::from_text(mov, pos)?;
        ensure!(
            pos.is_pseudolegal_move_legal(mov),
            "The move '{0}' is pseudolegal but not legal in the current position ('{pos}')",
            mov.compact_formatter(pos).to_string().bold()
        );
        let string = self.state.print_engine_state_for_move(pos, mov)?;
        self.write_ugi(&format_args!("{string}"));
        Ok(())
    }

    fn handle_eval_or_tt(&mut self, eval: bool, words: &mut Tokens) -> Res<()> {
        self.handle_eval_or_tt_impl(eval, words)
    }

    fn handle_engine(&mut self, words: &mut Tokens) -> Res<()> {
        self.handle_engine_impl(words)
    }

    fn handle_set_eval(&mut self, words: &mut Tokens) -> Res<()> {
        self.handle_set_eval_impl(words)
    }

    #[cold]
    fn load_pgn(&mut self, words: &mut Tokens) -> Res<()> {
        let pgn_text = if words.peek().is_some() {
            fs::read_to_string(words.join(" "))?
        } else {
            inquire::Editor::new("Open the editor to enter a PGN, then press enter").prompt()?
        };
        let pgn_data = parse_pgn::<B>(&pgn_text, self.strictness, None)?;
        let keep_hist = self.is_interactive() || self.state.debug_mode;
        self.state.match_state.set_new_pos_state(pgn_data.game, keep_hist);
        self.print_board(OutputOpts::default());
        Ok(())
    }

    #[cold]
    fn handle_flip(&mut self) -> Res<()> {
        let new_board = self
            .state
            .pos()
            .clone()
            .make_nullmove()
            .ok_or(anyhow!("Could not flip the side to move (board: '{}'", self.state.board.as_fen().bold()))?;
        let state = UgiPosState::new(new_board);
        self.state.set_new_pos_state(state, true);
        self.print_board(OutputOpts::default());
        Ok(())
    }

    fn handle_query(&mut self, words: &mut Tokens) -> Res<()> {
        self.handle_query_impl(words)
    }

    fn handle_variant(&mut self, words: &mut Tokens) -> Res<()> {
        self.set_variant(words)?;
        self.print_board(OutputOpts::default());
        Ok(())
    }

    #[cold]
    fn handle_wait(&mut self, words: &mut Tokens) -> Res<()> {
        let mut max_duration = Duration::MAX;
        if let Some(token) = words.next() {
            max_duration = Duration::from_millis(parse_int_from_str(token, "duration in ms")?);
        }
        let start = Instant::now();
        while self.state.is_currently_searching() {
            sleep(Duration::from_millis(100));
            if start.elapsed() >= max_duration {
                break;
            }
        }
        Ok(())
    }

    fn handle_play(&mut self, words: &mut Tokens) -> Res<()> {
        handle_play_impl(self, words)
    }

    #[cold]
    fn handle_assist(&mut self, words: &mut Tokens) -> Res<()> {
        if let Some(next) = words.next() {
            let opt = EngineOptionNameForProto { name: RespondToMove, proto: self.protocol() };
            self.set_option(opt, next.to_string())
        } else {
            self.play_engine_move(None)
        }
    }

    #[cold]
    fn handle_undo(&mut self, words: &mut Tokens) -> Res<()> {
        let count = words.next().unwrap_or("1");
        let count = parse_int_from_str(count, "number of halfmoves to undo")?;
        let undone = self.state.undo_moves(count)?;
        self.print_board(OutputOpts::default());
        if undone < count {
            self.write_message(
                Warning,
                &format_args!("Reached initial position after undoing {undone} out of {count} halfmoves"),
            )
        }
        Ok(())
    }

    #[cold]
    fn handle_gb(&mut self, words: &mut Tokens) -> Res<()> {
        let count = words.next().unwrap_or("1");
        let count = parse_int_from_str(count, "number of positions to go back")?;
        let undone = self.state.go_back(count)?;
        if undone < count {
            self.write_message(
                Warning,
                &format_args!("There were only {undone} previous position commands, went back to the initial position"),
            );
        }
        self.print_board(OutputOpts::default());
        Ok(())
    }

    #[cold]
    fn handle_place_piece(&mut self, words: &mut Tokens) -> Res<()> {
        let pos = self.state.pos();
        let settings = pos.settings();
        let piece = ColPieceTypeOf::<B>::from_words(words, settings)?;
        let Some(coords) = words.next() else { bail!("Missing square from which to remove a piece") };
        let coords = B::Coordinates::from_str(coords)?;
        let piece = B::Piece::new(piece, coords);
        let pos = self.state.pos().clone().place_piece(piece)?;
        let pos = pos.verify(self.strictness)?;
        self.state.set_new_pos_state(UgiPosState::new(pos), true);
        self.print_board(OutputOpts::default());
        Ok(())
    }

    #[cold]
    fn handle_remove_piece(&mut self, words: &mut Tokens) -> Res<()> {
        let Some(coords) = words.next() else { bail!("Missing square from which to remove a piece") };
        let square = B::Coordinates::from_str(coords)?;
        let pos = self.state.pos().clone().remove_piece(square)?;
        let pos = pos.verify(self.strictness)?;
        self.state.set_new_pos_state(UgiPosState::new(pos), true);
        self.print_board(OutputOpts::default());
        Ok(())
    }

    #[cold]
    fn handle_move_piece(&mut self, words: &mut Tokens) -> Res<()> {
        let Some(src) = words.next() else { bail!("Missing square from which to remove the piece") };
        let src = B::Coordinates::from_str(src)?;
        let Some(dest) = words.next() else { bail!("Missing square on which to place the piece") };
        let dest = B::Coordinates::from_str(dest)?;
        let pos = self.state.pos().clone();
        let mut new_pos = pos.clone().remove_piece(src)?;
        let piece = pos.colored_piece_on(src); // call after `remove_piece` because that ensures coordinates are valid
        let piece = B::Piece::new(piece.colored_piece_type(), dest);
        new_pos.try_place_piece(piece)?;
        let pos = new_pos.verify(self.strictness)?;
        self.state.set_new_pos_state(UgiPosState::new(pos), true);
        self.print_board(OutputOpts::default());
        Ok(())
    }

    fn handle_randomize(&mut self, words: &mut Tokens) -> Res<()> {
        let mut symmetry = None;
        if let Some(&word) = words.peek() {
            let symmetries = Symmetry::iter().collect_vec();
            symmetry = Some(*select_name_static(word, symmetries.iter(), "symmetry", self.game_name(), NoDescription)?);
            _ = words.next();
        }
        let pos = B::random_pos(&mut rng(), self.strictness, symmetry)?;
        self.state.set_new_pos_state(UgiPosState::new(pos), true);
        self.print_board(OutputOpts::default());
        Ok(())
    }

    #[cold]
    fn print_help(&mut self) -> Res<()> {
        let engine_name = format!(
            "{0} ({1})",
            self.state.display_name.clone().bold(),
            self.state.engine.get_engine_info().long_name().bold()
        );
        print_help_impl(self, engine_name);
        Ok(())
    }

    fn write_is_player(&mut self, is_first: bool) -> Res<()> {
        self.write_response(&(self.state.board.active_player().is_first() == is_first).to_string())
    }

    fn respond_game(&mut self) -> Res<()> {
        let board = &self.state.board;
        self.write_ugi(&format_args!("{0}\n{1}", &board.long_name(), board.description().unwrap_or_default()));
        Ok(())
    }

    fn respond_engine(&mut self) -> Res<()> {
        let info = self.state.engine.get_engine_info();
        let name = info.long_name();
        let description = info.description().unwrap_or_default();
        drop(info);
        self.write_ugi(&format_args!("{name}\n{description}",));
        Ok(())
    }

    #[cold]
    fn handle_quit(&mut self, typ: Quitting) -> Res<()> {
        // Do this before sending `quit`: If that fails, we can still recognize that we wanted to quit,
        // so that continuing on errors won't prevent us from quitting the program.
        self.state.set_status(Quit(typ));
        self.state.engine.send_quit()?;
        Ok(())
    }
}

/// Trait to reduce code duplication. Unlike `AbstractEngineUgiState`, this is not implemented by the auto complete state.
trait AbstractEngineUgi: AbstractEngineUgiState {
    fn abstract_ugi_pos_state(&self) -> &dyn AbstractUgiPosState;

    fn all_commands(&self) -> &AllCommands;

    fn set_failed_cmd(&mut self, cmd: Option<String>);

    fn write_ugi_input(&self, words: Tokens);

    fn game_name(&self) -> &str;

    fn print_board(&mut self, opts: OutputOpts) {
        // TODO: Rework the output system
        _ = self.handle_print(&mut tokens(""), opts);
    }

    fn fuzzing_mode(&self) -> bool {
        cfg!(feature = "fuzzing")
    }

    fn protocol(&self) -> Protocol;

    fn is_interactive(&self) -> bool {
        self.protocol() == Interactive
    }

    fn debug_mode(&self) -> bool;

    fn handle_move_fen_or_pgn(&mut self, first_word: &str, rest: &mut Tokens) -> Res<bool>;
}

impl<B: Board> AbstractEngineUgi for EngineUGI<B> {
    fn abstract_ugi_pos_state(&self) -> &dyn AbstractUgiPosState {
        self.state.abstract_pos_state()
    }

    fn all_commands(&self) -> &AllCommands {
        &self.commands
    }

    fn set_failed_cmd(&mut self, cmd: Option<String>) {
        self.failed_cmd = cmd;
    }

    fn write_ugi_input(&self, words: Tokens) {
        self.output().write_ugi_input(words);
    }

    fn game_name(&self) -> &str {
        self.state.game_name()
    }

    fn protocol(&self) -> Protocol {
        self.state.protocol
    }

    fn debug_mode(&self) -> bool {
        self.state.debug_mode
    }

    #[cold]
    fn handle_move_fen_or_pgn(&mut self, first_word: &str, rest: &mut Tokens) -> Res<bool> {
        let original = rest.clone();
        let res = self.handle_move_input(first_word, rest);
        if let Ok(true) = res {
            return res;
        }
        let text = tokens_to_string(first_word, original);
        let mut tokens = tokens(&text);
        if let Ok(pgn_data) = parse_pgn::<B>(&text, self.strictness, Some(self.state.board.clone())) {
            let keep_hist = self.is_interactive() || self.state.debug_mode;
            self.state.match_state.set_new_pos_state(pgn_data.game, keep_hist);
            self.write_ugi(&format_args!(
                "{}",
                "Interpreting input as PGN (Not a valid command or variation)...".bold()
            ));
            self.print_board(OutputOpts::default());
            return Ok(true);
        } else if self.handle_pos(&mut tokens).is_ok() {
            if let Some(next) = tokens.peek() {
                self.write_message(
                    Warning,
                    &format_args!("Ignoring trailing input starting with '{}' after a valid position", next.red()),
                );
            }
            return Ok(true);
        }
        res
    }
}

#[cold]
fn print_game_over(ugi: &mut dyn AbstractEngineUgi, flip: bool) -> bool {
    ugi.print_board(OutputOpts { disable_flipping: true });
    let Some(res) = ugi.abstract_ugi_pos_state().player_result() else {
        return false;
    };
    let res = res.flip_if(flip);
    let text = match res {
        PlayerResult::Win => "V i c t o r y !",
        PlayerResult::Lose => "D e f e a t",
        PlayerResult::Draw => "D r a w .",
    };
    let text = print_as_ascii_art(text, 10);
    let text = match res {
        PlayerResult::Win => text.green(),
        PlayerResult::Lose => text.red(),
        PlayerResult::Draw => text.into(),
    };
    ugi.write_ugi(&format_args!("{text}"));
    true
}

#[cold]
fn print_help_impl(ugi: &dyn AbstractEngineUgi, engine_name: String) {
    let motors = "motors".bold();
    let game_name = ugi.game_name().bold();
    let mut text = format!(
        "{motors}: A work-in-progress collection of engines for various games, \
        currently playing {game_name}, using the engine {engine_name}.\
        \nSeveral commands are supported (see https://backscattering.de/chess/uci/ for a description of the UCI interface):\n\
        \n{}:\n",
        "UGI Commands".bold()
    );
    for cmd in ugi.all_commands().ugi.iter().filter(|c| c.standard() != Custom) {
        writeln!(&mut text, " {}", *cmd).unwrap();
    }
    write!(&mut text, "\n{}:\n", "Custom Commands".bold()).unwrap();
    for cmd in ugi.all_commands().ugi.iter().filter(|c| c.standard() == Custom) {
        writeln!(&mut text, " {}", *cmd).unwrap();
    }
    println!("{text}");
}

#[cold]
fn write_single_option(option: &EngineOption, res: &mut String) {
    writeln!(res, "{name}: {value}", name = option.name.to_string().bold(), value = option.value.value_to_str().bold())
        .unwrap();
}

#[cold]
fn write_options_impl(
    options: Vec<EngineOption>,
    engine_name: &str,
    game_name: &str,
    words: &mut Tokens,
) -> Res<String> {
    if words.peek().is_some_and(|next| next.eq_ignore_ascii_case("name")) {
        _ = words.next();
    }
    Ok(match words.join(" ").to_ascii_lowercase().as_str() {
        "" => {
            let mut res = format!("{engine_name} playing {game_name}\n");
            for o in options {
                write_single_option(&o, &mut res);
            }
            res
        }
        x => match options.iter().find(|o| o.name.to_string().eq_ignore_ascii_case(x)) {
            Some(opt) => {
                let mut res = String::new();
                write_single_option(opt, &mut res);
                res
            }
            None => {
                bail!("No option named '{0}' exists. Type '{1}' for a list of options.", x.red(), "ugi".bold())
            }
        },
    })
}

#[cold]
fn invalid_command_msg(interactive: bool, first_word: &str, rest: &mut Tokens, err_msg: &str) -> String {
    // The original UCI spec demands that unrecognized tokens should be ignored, whereas the
    // expositor UCI spec demands that an invalid token should cause the entire message to be ignored.
    let suggest_help = if interactive {
        format!("Type '{}' for a list of recognized commands", "help".bold())
    } else {
        format!(
            "If you are a human, consider typing '{0}' to see a list of recognized commands.\n\
            Also consider typing '{1}' or '{2}' to enable the interactive interface.",
            "help".bold(),
            "interactive".bold(),
            "i".bold(),
        )
    };
    let input = format!("{first_word} {}", rest.clone().join(" "));
    let first_len = first_word.chars().count();
    let error_msg = if input.len() > 150 || first_len > 50 {
        "Invalid first word of a long UGI command".to_string()
    } else if rest.peek().is_none() {
        format!("Invalid single-word UGI command '{}'", first_word.red())
    } else {
        format!("Invalid first word '{0}' of UGI command '{1}'", first_word.red(), input.trim().bold())
    };
    format!("{error_msg}, ignoring the entire command:\n{err_msg}\n{suggest_help}")
}

// take a BoardGameState instead of a board to correctly handle displaying the last move
#[cold]
fn format_tt_entry<B: Board>(state: MatchState<B>, entry: TTEntry<B>) -> String {
    let pos = state.board.clone();
    let pos2 = pos.clone();
    let formatter = pos.pretty_formatter(None, state.last_move(), OutputOpts::default());
    let mov = entry.mov(&pos);
    let mut formatter = AdaptFormatter {
        underlying: formatter,
        color_frame: Box::new(move |coords, color| {
            if let Some(mov) = mov {
                if Some(coords) == mov.src_square_in(&pos) || coords == mov.dest_square_in(&pos) {
                    return Some(Red);
                }
            };
            color
        }),
        display_piece: Box::new(move |coords, _, default| {
            if let Some(mov) = mov {
                if mov.src_square_in(&pos2) == Some(coords) {
                    return default.dimmed().to_string();
                } else if mov.dest_square_in(&pos2) == coords {
                    return default.bold().to_string();
                }
            }
            default
        }),
        horizontal_spacer_interval: None,
        vertical_spacer_interval: None,
        square_width: None,
    };
    let pos = &state.board;
    let mut res = pos.display_pretty(&mut formatter);
    let move_string =
        if let Some(mov) = mov { mov.to_extended_text(pos, Standard).bold().to_string() } else { "<none>".to_string() };
    let bound_str = entry.bound().comparison_str(false).bold().to_string();
    let score = Score::from_compact(entry.score);
    write!(
        &mut res,
        "\nHash: {6}\nScore: {bound_str}{0} ({1}), Raw Eval: {2}, Depth: {3}, Age Ctr: {4}, Best Move: {5}",
        pretty_score(score, None, None, &score_gradient(), true, false),
        entry.bound(),
        pretty_score(entry.raw_eval(), None, None, &score_gradient(), true, false),
        entry.depth.to_string().bold(),
        entry.age(),
        move_string,
        pos.hash_pos()
    )
    .unwrap();
    res
}

#[cold]
fn show_eval_pos<B: Board>(pos: &B, last: Option<B::Move>, eval: Box<dyn Eval<B>>) -> String {
    let eval = RefCell::new(eval);
    let formatter = pos.pretty_formatter(None, last, OutputOpts::default());
    let eval_pos = eval.borrow_mut().eval(pos, 0, pos.active_player());
    let p = pos.clone();
    let piece_width = ColPieceTypeOf::<B>::max_num_chars(pos.settings());
    let mut formatter = AdaptFormatter {
        underlying: formatter,
        color_frame: Box::new(|_, col| col),
        display_piece: Box::new(move |coords, _, default| {
            let piece = p.colored_piece_on(coords);
            let Some(color) = piece.color() else {
                return default;
            };
            let piece = format!(
                "{:piece_width$}:",
                piece
                    .colored_piece_type()
                    .str_formatter(p.settings(), Ascii, true)
                    .to_string()
                    .color(display_color(color))
            );
            let score = match p.clone().remove_piece(coords).unwrap().verify(Relaxed) {
                Ok(pos) => {
                    let diff = eval_pos - eval.borrow_mut().eval(&pos, 0, pos.active_player());
                    let (val, suffix) = suffix_for(diff.0 as isize, Some(10_000));
                    // reduce the scale by some scale because we expect pieces values to be much larger
                    // than eval values. The ideal scale depends on the game and eval,
                    let score_color = color_for_score(diff / eval.borrow().piece_scale(), &score_gradient());
                    format!("{:>5}", format!("{val:>3}{suffix}")).color(score_color).to_string()
                }
                Err(_) => " None".dimmed().to_string(),
            };
            format!("{piece}{score}")
        }),
        horizontal_spacer_interval: None,
        vertical_spacer_interval: None,
        square_width: Some(7),
    };
    pos.display_pretty(&mut formatter)
}

#[cold]
fn handle_play_impl(ugi: &mut dyn AbstractEngineUgi, words: &mut Tokens) -> Res<()> {
    let Some(game_name) = words.next() else { bail!("Missing game name after '{}'", "play".bold()) };
    let mut opts = match select_game(game_name) {
        Ok(game) => EngineOpts::for_game(game, ugi.debug_mode()),
        Err(err) => {
            let (game, variant) = game_name.split_once('-').unwrap_or(("fairy", game_name));
            let Ok(game) = select_game(game) else { return Err(err) };
            let mut opts = EngineOpts::for_game(game, ugi.debug_mode());
            opts.pos_name = Some(format!("{variant} startpos"));
            ugi.write_message(
                Warning,
                &format_args!("There is no implementation of '{game_name}', falling back to {}", "fairy".bold()),
            );
            opts
        }
    };
    if let Some(word) = words.next() {
        opts.engine = word.to_string();
    }
    if words.peek().is_some() {
        opts.pos_name = Some(words.join(" "));
    }
    let mut nested_match = create_match(opts)?;
    if nested_match.run() == QuitProgram {
        ugi.handle_quit(QuitProgram)?;
    } else {
        // print the current board again, now that the match is over
        ugi.print_board(OutputOpts::default());
    }
    Ok(())
}

#[cold]
fn compare_splitperft<B: Board>(ugi: &mut EngineUGI<B>, perft_res: SplitPerftRes<B>) -> Res<()> {
    let compare_text =
        inquire::Editor::new("Press 'e' to open an editor and enter your splitperft results, then press enter")
            .prompt()?;
    ugi.write_message(Info, &format_args!("Received input: '{compare_text}'"));
    let mut seen = HashSet::default();
    let mut errors = vec![];
    for line in compare_text.lines().filter(|l| !l.trim().is_empty()) {
        if let Err(err) = splitperft_line(line, &perft_res, &mut seen) {
            errors.push(err.to_string());
        }
    }
    for (unseen, nodes) in perft_res.children.iter().filter(|(m, _n)| !seen.contains(m)) {
        let err = anyhow!(
            "Missing move '{0}' ({1} nodes)",
            unseen.compact_formatter(&perft_res.pos).to_string().red(),
            nodes.to_string().bold()
        );
        errors.push(err.to_string());
    }
    if errors.is_empty() {
        ugi.write_ugi(&format_args!("Splitperft result matches ({} moves)!", perft_res.children.len()));
    } else {
        ugi.write_message(Warning, &format_args!("There were {0} errors: ", errors.len().to_string().bold()));
        for line in errors {
            ugi.write_ugi(&format_args!("{line}"));
        }
    }
    Ok(())
}

fn splitperft_line<B: Board>(line: &str, perft_res: &SplitPerftRes<B>, seen: &mut HashSet<B::Move>) -> Res<()> {
    let mut words = tokens(line).map(|w| w.to_ascii_lowercase());
    let mov = words.next().unwrap();
    let mov = mov.trim_end_matches(':');
    let mut numbers = words.filter_map(|w| w.trim_end_matches("nodes").parse::<u64>().ok());
    let Some(nodes) = numbers.next() else {
        bail!("Failed to find subtree nodes count in '{}'", line.red());
    };
    if numbers.next().is_some() {
        bail!("Line contains multiple numbers, can't decide which one is the splitperft nodes count")
    }
    let mov = match B::Move::from_text(mov, &perft_res.pos) {
        Ok(m) => m,
        Err(_) => {
            let mut matching = perft_res.children.iter().filter_map(|(m, _n)| {
                let strings = [
                    m.compact_formatter(&perft_res.pos).to_string(),
                    m.to_extended_text(&perft_res.pos, Standard),
                    m.to_extended_text(&perft_res.pos, Alternative),
                ];
                if strings.iter().any(|s| s.eq_ignore_ascii_case(mov)) { Some(*m) } else { None }
            });
            let Some(m) = matching.next() else {
                bail!("Invalid move '{0}' ({1} nodes)", mov.red(), nodes.to_string().bold())
            };
            if let Some(m2) = matching.next() {
                bail!(
                    "Move '{0}' can't be parsed directly and matches more than one textual move representation: '{1}' and '{2}'",
                    mov.red(),
                    m.compact_formatter(&perft_res.pos).to_string().bold(),
                    m2.compact_formatter(&perft_res.pos).to_string().bold()
                );
            }
            m
        }
    };
    if !seen.insert(mov) {
        bail!("Duplicate move '{0}'", mov.compact_formatter(&perft_res.pos).to_string().red());
    }
    let Some((mov, n)) = perft_res.children.iter().find(|(m, _n)| *m == mov) else {
        bail!(
            "Invalid move '{0}' (Internal error: Not a splitperft child but matches a legal move)",
            mov.compact_formatter(&perft_res.pos).to_string().red()
        );
    };
    if nodes != *n {
        bail!(
            "Incorrect splitperft number for move '{0}': Should be {1} but is {2}",
            mov.compact_formatter(&perft_res.pos).to_string().red(),
            n.to_string().red(),
            nodes.to_string().bold()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{list_chess_evals, list_chess_outputs, list_chess_searchers};
    use gears::cli::Game::Chess;
    use gears::create_selected_output_builders;
    use gears::games::chess::ChessColor::Black;
    use gears::games::chess::Chessboard;
    use gears::rand::prelude::SliceRandom;
    use gears::rand::rngs::StdRng;
    use gears::rand::{Rng, SeedableRng};

    fn create_chess_game() -> Box<EngineUGI<Chessboard>> {
        let outputs = list_chess_outputs();
        let searchers = list_chess_searchers();
        let evals = list_chess_evals();
        let opts = EngineOpts::for_game(Chess, true);
        Box::new(
            EngineUGI::create(
                opts.clone(),
                create_selected_output_builders(&opts.outputs, &outputs).unwrap(),
                outputs,
                searchers,
                evals,
            )
            .unwrap(),
        )
    }

    #[test]
    #[cfg(feature = "chess")]
    fn chess_test() {
        let mut ugi = create_chess_game();
        ugi.handle_input("idk").unwrap();
        ugi.handle_input("idk off").unwrap();
        let state = ugi.state.match_state.clone();
        assert_eq!(state.pos_before_moves, Chessboard::default());
        assert_eq!(state.pos().active_player(), Black);
        assert_eq!(state.mov_hist.len(), 1);
        assert_eq!(state.board_hist.len(), 1);
        assert_eq!(state.status, Run(NotStarted));
        ugi.handle_input("undo").unwrap();
        assert_eq!(*ugi.state.pos(), Chessboard::default());
        assert_eq!(ugi.state.mov_hist.len(), 0);
        ugi.handle_input("position startpos e2e4").unwrap();
        ugi.handle_input("randomize").unwrap();
        ugi.handle_input("gb").unwrap();
        assert_eq!(ugi.state.pos().active_player(), Black);
        // There are actual fuzz tests for the UGI interface, but they aren't run regularly.
        // So this is a very small part of what fuzz testing would do, but run regularly
        let mut cmds = vec![
            "e5",
            "show engine_state",
            "tt startpos moves e2e4",
            "query engine",
            "ugi",
            "flip",
            "gb",
            "place black knight a4",
            "remove a1",
            "move_piece b2 b3",
            "help",
            "eval",
        ];
        let seed = rng().random::<u64>();
        // let seed = 1880428284001215887;
        eprintln!("Seed: {seed}");
        let mut rng = StdRng::seed_from_u64(seed);
        cmds.shuffle(&mut rng);
        for c in cmds {
            eprintln!("<EXECUTING> {c}");
            ugi.handle_input(c).unwrap();
        }
        sleep(Duration::from_millis(5000));
        ugi.quit().unwrap(); // can't use handle_input("quit") because that gets ignored in fuzzing mode
        assert_eq!(ugi.state.status, Quit(QuitProgram));
    }
}
