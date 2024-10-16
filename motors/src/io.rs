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
pub mod cli;
mod command;
mod input;
pub mod ugi_output;

use itertools::Itertools;
use std::cell::RefCell;
use std::fmt::{Debug, Display, Formatter, Write};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::eval::Eval;
use crate::io::ascii_art::print_as_ascii_art;
use crate::io::cli::EngineOpts;
use crate::io::command::Standard::Custom;
use crate::io::command::{
    accept_depth, go_options, query_options, ugi_commands, CommandList, GoState,
};
use crate::io::input::Input;
use crate::io::ugi_output::{color_for_score, pretty_score, score_gradient, suffix_for, UgiOutput};
use crate::io::ProgramStatus::{Quit, Run};
use crate::io::Protocol::{Interactive, UGI};
use crate::io::SearchType::*;
use crate::search::multithreading::EngineWrapper;
use crate::search::tt::{TTEntry, DEFAULT_HASH_SIZE_MB, TT};
use crate::search::{run_bench_with, EvalList, SearchParams, SearcherList};
use crate::{
    create_engine_box_from_str, create_engine_from_str, create_eval_from_str, create_match,
};
use gears::cli::select_game;
use gears::crossterm::style;
use gears::crossterm::style::Stylize;
use gears::games::{BoardHistory, ColoredPiece, OutputList, ZobristHistory};
use gears::general::board::Strictness::{Relaxed, Strict};
use gears::general::board::{Board, Strictness, UnverifiedBoard};
use gears::general::common::anyhow::{anyhow, bail};
use gears::general::common::Description::{NoDescription, WithDescription};
use gears::general::common::{
    parse_bool_from_str, parse_duration_ms, parse_int_from_str, select_name_dyn, tokens, ColorMsg,
    NamedEntity,
};
use gears::general::common::{Res, Tokens};
use gears::general::moves::ExtendedFormat::{Alternative, Standard};
use gears::general::moves::Move;
use gears::general::perft::{perft_for, split_perft};
use gears::output::logger::LoggerBuilder;
use gears::output::text_output::{display_color, AdaptFormatter};
use gears::output::Message::*;
use gears::output::{Message, OutputBox, OutputBuilder};
use gears::search::{Depth, SearchLimit, TimeControl};
use gears::ugi::EngineOptionName::*;
use gears::ugi::EngineOptionType::*;
use gears::ugi::{
    parse_ugi_position_and_moves, EngineOption, EngineOptionName, UgiCheck, UgiCombo, UgiSpin,
    UgiString,
};
use gears::MatchStatus::*;
use gears::Quitting::QuitProgram;
use gears::{output_builder_from_str, AbstractRun, GameState, MatchStatus, PlayerResult, Quitting};

const DEFAULT_MOVE_OVERHEAD_MS: u64 = 50;

// TODO: Ensure this conforms to <https://expositor.dev/uci/doc/uci-draft-1.pdf>

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[must_use]
enum SearchType {
    Normal,
    Ponder,
    Bench,
    Perft,
    SplitPerft,
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
            }
        )
    }
}

#[derive(Debug, Clone)]
pub enum ProgramStatus {
    Run(MatchStatus),
    Quit(Quitting),
}

#[derive(
    Debug, Default, Copy, Clone, Eq, PartialEq, derive_more::Display, derive_more::FromStr,
)]
pub enum Protocol {
    #[default]
    Interactive,
    UGI,
    UCI,
    UAI,
}

#[derive(Debug, Clone)]
struct BoardGameState<B: Board> {
    board: B,
    debug_mode: bool,
    status: ProgramStatus,
    mov_hist: Vec<B::Move>,
    board_hist: ZobristHistory<B>,
    initial_pos: B,
    last_played_color: B::Color,
    ponder_limit: Option<SearchLimit>,
}

impl<B: Board> BoardGameState<B> {
    fn last_move(&self) -> Option<B::Move> {
        self.mov_hist.last().copied()
    }

    fn make_move(&mut self, mov: B::Move) -> Res<B> {
        debug_assert!(self.board.is_move_pseudolegal(mov));
        if let Run(Over(result)) = &self.status {
            bail!(
                "Cannot play move '{mov}' because the game is already over: {0} ({1}). The position is '{2}'",
                result.result, result.reason, self.board
            )
        }
        self.board_hist.push(&self.board);
        self.mov_hist.push(mov);
        self.board = self.board.make_move(mov).ok_or_else(|| {
            anyhow!(
                "Illegal move {mov} (pseudolegal but not legal) in position {}",
                self.board
            )
        })?;
        Ok(self.board)
    }

    fn clear_state(&mut self) {
        self.board = self.initial_pos;
        self.mov_hist.clear();
        self.board_hist.clear();
        self.status = Run(NotStarted);
    }

    fn handle_position(
        &mut self,
        words: &mut Tokens,
        allow_pos_word: bool,
        strictness: Strictness,
    ) -> Res<()> {
        let pos = self.board;
        let Some(next_word) = words.next() else {
            bail!(
                "Missing position after '{}' command",
                "position".important()
            )
        };
        parse_ugi_position_and_moves(
            next_word,
            words,
            allow_pos_word,
            strictness,
            &pos,
            self,
            |this, mov| this.make_move(mov).map(|_| ()),
            |this| {
                this.initial_pos = this.board;
                this.clear_state()
            },
            |state| &mut state.board,
        )?;
        self.last_played_color = self.board.active_player();
        Ok(())
    }
}

#[derive(Debug)]
struct EngineGameState<B: Board> {
    board_state: BoardGameState<B>,
    game_name: String,
    protocol: Protocol,
    engine: EngineWrapper<B>,
    /// This doesn't have to be the UGI engine name. It often isn't, especially when two engines with
    /// the same name play against each other, such as in a SPRT. It should be unique, however
    /// (the `monitors` client ensures that, but another GUI might not).
    display_name: String,
    opponent_name: Option<String>,
}

impl<B: Board> Deref for EngineGameState<B> {
    type Target = BoardGameState<B>;

    fn deref(&self) -> &Self::Target {
        &self.board_state
    }
}

impl<B: Board> DerefMut for EngineGameState<B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.board_state
    }
}

#[derive(Debug)]
struct AllCommands<B: Board> {
    ugi: CommandList<EngineUGI<B>>,
    go: CommandList<GoState<B>>,
    query: CommandList<EngineUGI<B>>,
}

/// Implements both UGI and UCI.
#[derive(Debug)]
pub struct EngineUGI<B: Board> {
    state: EngineGameState<B>,
    commands: AllCommands<B>,
    output: Arc<Mutex<UgiOutput<B>>>,
    output_factories: Rc<OutputList<B>>,
    searcher_factories: Rc<SearcherList<B>>,
    eval_factories: Rc<EvalList<B>>,
    move_overhead: Duration,
    strictness: Strictness,
    multi_pv: usize,
    allow_ponder: bool,
}

impl<B: Board> AbstractRun for EngineUGI<B> {
    fn run(&mut self) -> Quitting {
        self.ugi_loop()
    }

    fn handle_input(&mut self, input: &str) -> Res<()> {
        self.handle_ugi_input(tokens(input))
    }
    fn quit(&mut self) -> Res<()> {
        self.handle_quit(QuitProgram)
    }
}

impl<B: Board> GameState<B> for EngineGameState<B> {
    fn initial_pos(&self) -> B {
        self.initial_pos
    }

    fn get_board(&self) -> B {
        self.board
    }

    fn game_name(&self) -> &str {
        &self.game_name
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
        format!("{0} {1} match", self.name(), B::game_name())
    }

    fn site(&self) -> &str {
        "??"
    }

    fn player_name(&self, color: B::Color) -> Option<String> {
        if color == self.last_played_color {
            Some(self.name().to_string())
        } else {
            self.opponent_name.clone()
        }
    }

    fn time(&self, _color: B::Color) -> Option<TimeControl> {
        // Technically, we get the time with 'go', but we can't trust it for the other player,
        // and we don't really need this for ourselves while we're thinking
        None
    }

    fn thinking_since(&self, _color: B::Color) -> Option<Instant> {
        None
    }
}

impl<B: Board> EngineUGI<B> {
    pub fn create(
        opts: EngineOpts,
        mut selected_output_builders: OutputList<B>,
        all_output_builders: OutputList<B>,
        all_searchers: SearcherList<B>,
        all_evals: EvalList<B>,
    ) -> Res<Self> {
        let output = Arc::new(Mutex::new(UgiOutput::new(opts.interactive)));
        let board = match opts.pos_name {
            None => B::default(),
            Some(name) => B::from_name(&name)?,
        };
        let engine = create_engine_from_str(
            &opts.engine,
            &all_searchers,
            &all_evals,
            output.clone(),
            TT::default(),
        )?;
        let display_name = engine.get_engine_info().short_name();
        let board_state = BoardGameState {
            board,
            debug_mode: opts.debug,
            status: Run(NotStarted),
            mov_hist: vec![],
            board_hist: ZobristHistory::default(),
            initial_pos: B::default(),
            last_played_color: B::Color::default(),
            ponder_limit: None,
        };
        let protocol = if opts.interactive {
            Interactive
        } else {
            Protocol::UGI
        };
        let state = EngineGameState {
            board_state,
            game_name: B::game_name(),
            protocol,
            engine,
            display_name,
            opponent_name: None,
        };
        let err_msg_builder = output_builder_from_str("error", &all_output_builders)?;
        selected_output_builders.push(err_msg_builder);
        for builder in &mut selected_output_builders {
            output
                .lock()
                .unwrap()
                .additional_outputs
                .push(builder.for_engine(&state)?);
        }
        Ok(Self {
            state,
            commands: AllCommands {
                ugi: ugi_commands(),
                go: go_options(None),
                query: query_options(),
            },
            output,
            output_factories: Rc::new(all_output_builders),
            searcher_factories: Rc::new(all_searchers),
            eval_factories: Rc::new(all_evals),
            move_overhead: Duration::from_millis(DEFAULT_MOVE_OVERHEAD_MS),
            strictness: Relaxed,
            multi_pv: 1,
            allow_ponder: false,
        })
    }

    pub fn fuzzing_mode(&self) -> bool {
        cfg!(feature = "fuzzing")
    }

    fn is_interactive(&self) -> bool {
        self.state.protocol == Interactive
    }

    fn output(&self) -> MutexGuard<UgiOutput<B>> {
        self.output.lock().unwrap()
    }

    fn ugi_loop(&mut self) -> Quitting {
        self.write_message(
            Debug,
            &format!("Starting UGI loop (playing {})", B::game_name()),
        );
        let text = format!("Motors: {}", self.state.game_name());
        let text = print_as_ascii_art(&text, 2);
        self.write_ugi(&text.dimmed().to_string());
        self.write_engine_ascii_art();
        self.write_ugi(&format!(
            "[Type '{}' to change how the game state is displayed{}]",
            "output".important(),
            ", e.g., 'output pretty' or 'output chess'".dimmed()
        ));
        if self.fuzzing_mode() {
            self.write_message(Warning, &"Fuzzing Mode Enabled!".important().to_string());
        }

        let (mut input, interactive) = Input::new(self.state.protocol == Interactive, self);
        if self.state.protocol == Interactive && !interactive {
            self.state.protocol = UGI; // Will be overwritten shortly, and isn't really used much anyway
        }
        loop {
            input.set_interactive(self.state.protocol == Interactive, self);
            let input = match input.get_line(self) {
                Ok(input) => input,
                Err(err) => {
                    self.write_message(Error, &err.to_string());
                    break;
                }
            };

            let res = self.handle_ugi_input(tokens(&input));
            match res {
                Err(err) => {
                    self.write_message(Error, &err.to_string());
                    if !self.continue_on_error() {
                        self.write_ugi(&format!("info error {err}"));
                    }
                    // explicitly check this here so that continuing on error doesn't prevent us from quitting.
                    if let Quit(quitting) = self.state.status {
                        return quitting;
                    }
                    if self.continue_on_error() {
                        let interactive = if self.is_interactive() { "on" } else { "off" };
                        self.write_message(
                            Debug,
                            &format!("Continuing... (interactive mode is {interactive})"),
                        );
                        continue;
                    }
                    return QuitProgram;
                }
                Ok(()) => {
                    if let Quit(quitting) = &self.state.status {
                        return *quitting;
                    }
                }
            }
        }
        QuitProgram
    }

    fn write_ugi(&mut self, message: &str) {
        self.output().write_ugi(message);
    }

    fn write_message(&mut self, typ: Message, msg: &str) {
        self.output().write_message(typ, msg);
    }

    fn continue_on_error(&self) -> bool {
        self.state.debug_mode || self.state.protocol == Interactive
    }

    pub fn handle_ugi_input(&mut self, mut words: Tokens) -> Res<()> {
        self.output().write_ugi_input(words.clone());
        if self.fuzzing_mode() {
            self.output()
                .write_ugi(&format!("Fuzzing input: [{}]", words.clone().join(" ")));
        }
        let words = &mut words;
        let Some(first_word) = words.next() else {
            return Ok(()); // ignore empty input
        };
        let Ok(cmd) = select_name_dyn(
            first_word,
            &self.commands.ugi,
            "command",
            self.state.game_name(),
            NoDescription,
        ) else {
            if self.handle_move_input(first_word, words)? {
                return Ok(());
            } else if first_word.eq_ignore_ascii_case("barbecue") {
                self.write_message(Error, "lol");
            }
            self.invalid_command(first_word, words);
            return Ok(());
        };

        // this does all the actual work of executing the command
        () = cmd.func()(self, words, first_word)?;

        if let Some(remaining) = words.next() {
            // can't reuse cmd because the borrow checker complains
            let cmd = select_name_dyn(
                first_word,
                &self.commands.ugi,
                "command",
                self.state.game_name(),
                NoDescription,
            )?;
            self.write_message(
                Warning,
                &format!(
                    "Ignoring trailing input starting with '{0}' after a valid '{1}' command",
                    remaining.important().error(),
                    cmd.short_name().important()
                )
                .to_string(),
            );
        }
        Ok(())
    }

    fn invalid_command(&mut self, first_word: &str, rest: &mut Tokens) {
        // The original UCI spec demands that unrecognized tokens should be ignored, whereas the
        // expositor UCI spec demands that an invalid token should cause the entire message to be ignored.
        let suggest_help = if self.is_interactive() {
            format!(
                "Type '{}' for a list of recognized commands",
                "help".important()
            )
        } else {
            format!(
                "If you are a human, consider typing '{}' to see a list of recognized commands.",
                "help".important()
            )
        };
        let input = format!("{first_word} {}", rest.clone().join(" "));
        let first_len = first_word.chars().count();
        let error_msg = if input.len() > 200 || first_len > 25 {
            let first_word = if first_len > 50 {
                format!(
                    "{0}{1}",
                    first_word.chars().take(50).collect::<String>().error(),
                    "...(rest omitted for brevity)".dimmed()
                )
            } else {
                first_word.error().to_string()
            };
            format!("Invalid first word '{first_word}' of a long UGI command")
        } else if rest.peek().is_none() {
            format!("Invalid single-word UGI command '{}'", first_word.error())
        } else {
            format!(
                "Invalid first word '{0}' of UGI command '{1}'",
                first_word.error(),
                input.trim().important()
            )
        };
        self.write_message(
            Warning,
            &format!("{error_msg}, ignoring the entire command.\n{suggest_help}"),
        );
    }

    fn print_game_over(&mut self, flip: bool) -> bool {
        self.print_board();
        let Some(res) = self.state.board.player_result_slow(&self.state.board_hist) else {
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
            PlayerResult::Win => text.dark_green(),
            PlayerResult::Lose => text.dark_red(),
            PlayerResult::Draw => text.stylize(),
        };
        self.write_ugi(&text.to_string());
        true
    }

    fn handle_move_input(&mut self, first_word: &str, rest: &mut Tokens) -> Res<bool> {
        let Ok(mov) = B::Move::from_text(first_word, &self.state.board) else {
            return Ok(false);
        };
        let mut state = self.state.clone();
        state.make_move(mov)?;
        for word in rest {
            let mov = B::Move::from_text(word, &state.board)?;
            state.make_move(mov)?;
        }
        self.state.board_state = state;
        if self.print_game_over(true) {
            return Ok(true);
        }
        self.write_ugi("Searching...");
        let engine = self.state.engine.get_engine_info().short_name();
        let mut engine =
            create_engine_box_from_str(&engine, &self.searcher_factories, &self.eval_factories)?;
        let limit = SearchLimit::per_move(Duration::from_millis(1_000));
        let params = SearchParams::new_unshared(
            self.state.board,
            limit,
            self.state.board_hist.clone(),
            TT::default(),
        );
        let res = engine.search(params);
        self.write_ugi(
            &res.chosen_move
                .to_extended_text(&self.state.board, Alternative),
        );
        self.state.make_move(res.chosen_move)?;
        _ = self.print_game_over(false);
        Ok(true)
    }

    pub fn handle_quit(&mut self, typ: Quitting) -> Res<()> {
        // Do this before sending `quit`: If that fails, we can still recognize that we wanted to quit,
        // so that continuing on errors won't prevent us from quitting the program.
        self.state.status = Quit(typ);
        self.state.engine.send_quit()?;
        Ok(())
    }

    fn handle_ugi(&mut self, proto: &str) -> Res<()> {
        let id_msg = self.id();
        self.write_ugi(id_msg.as_str());
        self.write_ugi(self.write_ugi_options().as_str());
        self.write_ugi(&format!("{proto}ok"));
        self.state.protocol = Protocol::from_str(proto).unwrap();
        self.output().pretty = self.state.protocol == Interactive;
        Ok(())
    }

    fn id(&self) -> String {
        let info = self.state.engine.get_engine_info();
        format!(
            "id name Motors -- Game {0} -- Engine {1}\nid author ToTheAnd",
            B::game_name(),
            info.long_name(),
        )
    }

    fn handle_setoption(&mut self, words: &mut Tokens) -> Res<()> {
        if words.peek().is_some_and(|w| w.eq_ignore_ascii_case("name")) {
            _ = words.next();
        }
        let mut name = String::default();
        loop {
            let next_word = words.next().unwrap_or_default();
            if next_word.eq_ignore_ascii_case("value") || next_word.is_empty() {
                break;
            }
            name = name + " " + next_word;
        }
        let mut value = words.next().unwrap_or_default().to_string();
        loop {
            let next_word = words.next().unwrap_or_default();
            if next_word.is_empty() {
                break;
            }
            value = value + " " + next_word;
        }
        let name = EngineOptionName::from_str(name.trim()).unwrap();
        match name {
            EngineOptionName::Ponder => {
                self.allow_ponder = parse_bool_from_str(&value, "ponder")?;
            }
            MoveOverhead => {
                self.move_overhead = parse_duration_ms(&mut tokens(&value), "move overhead")?;
            }
            MultiPv => {
                self.multi_pv = parse_int_from_str(&value, "multipv")?;
            }
            UCIOpponent => {
                let mut words = value.split_whitespace();
                loop {
                    match words.next() {
                        None => {
                            break;
                        }
                        Some(word)
                            if word.eq_ignore_ascii_case("computer")
                                || word.eq_ignore_ascii_case("human") =>
                        {
                            self.state.opponent_name = Some(words.join(" "));
                            break;
                        }
                        _ => continue,
                    }
                }
            }
            Strictness => {
                self.strictness = if parse_bool_from_str(&value, "strictness")? {
                    Strict
                } else {
                    Relaxed
                };
            }
            SetEngine => {
                self.handle_engine(&mut tokens(&value))?;
            }
            SetEval => {
                self.handle_set_eval(&mut tokens(&value))?;
            }
            _ => {
                let value = value.trim().to_string();
                self.state
                    .engine
                    .set_option(name.clone(), value.clone())
                    .or_else(|err| {
                        if name == Threads && value == "1" {
                            Ok(())
                        } else {
                            Err(err)
                        }
                    })?;
            }
        }
        Ok(())
    }

    fn print_board(&mut self) {
        // TODO: Rework the output system
        _ = self.handle_print(&mut tokens(""));
    }

    fn handle_position(&mut self, words: &mut Tokens) -> Res<()> {
        self.state.handle_position(words, false, self.strictness)?;
        if self.is_interactive() {
            self.print_board();
        }
        Ok(())
    }

    fn handle_go(&mut self, initial_search_type: SearchType, words: &mut Tokens) -> Res<()> {
        let mut opts = GoState::new(self, initial_search_type, self.move_overhead);

        if matches!(initial_search_type, Perft | SplitPerft | Bench) {
            accept_depth(&mut opts.limit, words)?;
        }
        while let Some(option) = words.next() {
            opts.cont = false;
            let cmd = select_name_dyn(
                option,
                &self.commands.go,
                "go option",
                &self.state.game_name,
                WithDescription,
            );
            match cmd {
                Ok(cmd) => cmd.func()(&mut opts, words, option)?,
                // TODO: Handle as command, no need for reading_moves
                Err(err) => {
                    if opts.reading_moves {
                        let mov =
                            B::Move::from_compact_text(option, &opts.board).map_err(|err| {
                                anyhow!(
                                    "{err}. '{}' is not a valid 'go' option.",
                                    option.important()
                                )
                            })?;
                        opts.search_moves.as_mut().unwrap().push(mov);
                        continue;
                    }
                    bail!(err)
                }
            }
            if !opts.cont {
                opts.reading_moves = false;
            }
        }
        opts.limit.tc.remaining = opts
            .limit
            .tc
            .remaining
            .saturating_sub(opts.move_overhead)
            .max(Duration::from_millis(1));

        if cfg!(feature = "fuzzing") {
            opts.limit.fixed_time = opts.limit.fixed_time.max(Duration::from_secs(1));
            if opts.complete {
                opts.limit.fixed_time = Duration::from_millis(10);
            }
            if matches!(opts.search_type, Perft | SplitPerft) {
                let depth = if opts.complete { 2 } else { 3 };
                opts.limit.depth = opts.limit.depth.min(Depth::new_unchecked(depth));
            }
        }

        if opts.complete && !matches!(opts.search_type, Bench | Perft) {
            bail!(
                "The '{0}' options can only be used for '{1}' and '{2}' searches",
                "complete".important(),
                "bench".important(),
                "perft".important()
            )
        }

        match opts.search_type {
            Bench => {
                let bench_positions = if opts.complete {
                    B::bench_positions()
                } else {
                    vec![self.state.board]
                };
                return self.bench(opts.limit, &bench_positions);
            }
            Perft => {
                let positions = if opts.complete {
                    B::bench_positions()
                } else {
                    vec![self.state.board]
                };
                self.output()
                    .write_ugi(&perft_for(opts.limit.depth, &positions).to_string())
            }
            SplitPerft => {
                if opts.limit.depth.get() == 0 {
                    bail!(
                        "{} requires a depth of at least 1",
                        "splitperft".important()
                    )
                }
                self.write_ugi(&split_perft(opts.limit.depth, self.state.board).to_string());
            }
            _ => return self.start_search(opts),
        }
        Ok(())
    }

    fn start_search(&mut self, opts: GoState<B>) -> Res<()> {
        self.write_message(
            Debug,
            &format!(
                "Starting {0} search with limit {1}",
                opts.search_type, opts.limit
            ),
        );
        if let Some(res) = opts.board.match_result_slow(&self.state.board_hist) {
            self.write_message(Warning, &format!("Starting a {3} search in position '{2}', but the game is already over. {0}, reason: {1}.",
                                        res.result, res.reason, self.state.board.as_fen().important(), opts.search_type));
        }
        self.state.status = Run(Ongoing);
        match opts.search_type {
            // this keeps the current history even if we're searching a different position, but that's probably not a problem
            // and doing a normal search from a custom position isn't even implemented at the moment -- TODO: implement?
            Normal => {
                // It doesn't matter if we got a ponderhit or a miss, we simply abort the ponder search and start a new search.
                if self.state.ponder_limit.is_some() {
                    self.state.ponder_limit = None;
                    // TODO: Maybe do this all the time to make sure two `go` commands after another work -- write testcase for that
                    self.state.engine.send_stop(true); // aborts the pondering without printing a search result
                }
                self.state.engine.start_search(
                    opts.board,
                    opts.limit,
                    opts.board_hist,
                    opts.search_moves,
                    opts.multi_pv,
                    false,
                    opts.threads,
                )?;
            }
            SearchType::Ponder => {
                self.state.ponder_limit = Some(opts.limit);
                self.state.engine.start_search(
                    opts.board,
                    SearchLimit::infinite(), //always allocate infinite time for pondering
                    opts.board_hist,
                    opts.search_moves,
                    opts.multi_pv, // don't ignore multi_pv in pondering mode
                    true,
                    opts.threads,
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
            limit.depth = Depth::MAX;
            limit.nodes = self.state.engine.get_engine_info().default_bench_nodes();
            Some(limit)
        };
        let res = run_bench_with(engine.as_mut(), limit, second_limit, positions);
        self.output().write_ugi(&res.to_string());
        Ok(())
    }

    fn handle_eval_or_tt(&mut self, eval: bool, words: &mut Tokens) -> Res<()> {
        let mut state = self.state.clone();
        if words.peek().is_some() {
            state.handle_position(words, true, Relaxed)?;
        }
        let text = if eval {
            let info = self.state.engine.get_engine_info();
            if let Some(eval_name) = info.eval() {
                let mut eval =
                    create_eval_from_str(&eval_name.short_name(), &self.eval_factories)?.build();
                let eval_score = eval.eval(&state.board);
                let diagram = show_eval_pos(state.board, state.last_move(), eval);
                diagram
                    + &format!(
                        "Eval Score: {}\n",
                        pretty_score(eval_score, None, &score_gradient(), true, false)
                    )
            } else {
                format!(
                    "The engine '{}' doesn't have an eval function",
                    info.short_name().important()
                )
            }
        } else if let Some(entry) = self.state.engine.tt_entry(&state.board) {
            format_tt_entry(state, entry)
        } else {
            "There is no TT entry for this position"
                .important()
                .to_string()
        };
        self.write_ugi(&text);
        Ok(())
    }

    fn handle_query(&mut self, words: &mut Tokens) -> Res<()> {
        let query = *words
            .peek()
            .ok_or(anyhow!("Missing argument to '{}'", "query".important()))?;
        match select_name_dyn(
            query,
            &self.commands.query,
            "query option",
            self.state.game_name(),
            WithDescription,
        ) {
            Ok(cmd) => {
                _ = words.next();
                cmd.func()(self, words, query)
            }
            Err(err) => {
                if let Ok(opt) = self.write_option(words) {
                    self.write_ugi(&opt);
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
                if self
                    .output()
                    .additional_outputs
                    .iter()
                    .any(|o| o.prints_board() && !o.is_logger())
                {
                    Ok(None)
                } else {
                    // Even though "pretty" can look better than "prettyascii", it's also significantly more risky
                    // because how it looks very much depends on the terminal.
                    self.select_output(&mut tokens("prettyascii"))
                }
            }
        }
    }

    fn handle_print(&mut self, words: &mut Tokens) -> Res<()> {
        let output = self.select_output(words)?;
        let print = |this: &Self, output: Option<OutputBox<B>>, state| match output {
            None => {
                this.output().show(state);
            }
            Some(mut output) => {
                output.show(state);
            }
        };
        if words.peek().is_some() {
            let old_state = self.state.board_state.clone();
            if let Err(err) = self.state.handle_position(words, true, self.strictness) {
                self.state.board_state = old_state;
                return Err(err);
            }
            print(self, output, &self.state);
            self.state.board_state = old_state;
        } else {
            print(self, output, &self.state);
        }
        Ok(())
    }

    fn handle_output(&mut self, words: &mut Tokens) -> Res<()> {
        let mut next = words.next().unwrap_or_default();
        let output_ptr = self.output.clone();
        let mut output = output_ptr.lock().unwrap();
        if next.eq_ignore_ascii_case("remove") || next.eq_ignore_ascii_case("clear") {
            let next = words.next().unwrap_or("all");
            if next.eq_ignore_ascii_case("all") {
                output.additional_outputs.retain(|o| !o.prints_board());
            } else {
                output
                    .additional_outputs
                    .retain(|o| !o.short_name().eq_ignore_ascii_case(next));
            }
        } else if !output
            .additional_outputs
            .iter()
            .any(|o| o.short_name().eq_ignore_ascii_case(next))
        {
            if next.eq_ignore_ascii_case("add") {
                next = words
                    .next()
                    .ok_or_else(|| anyhow!("Missing output name after 'add'"))?;
            } else {
                output.additional_outputs.retain(|o| !o.prints_board())
            }
            output.additional_outputs.push(
                output_builder_from_str(next, &self.output_factories)
                    .map_err(|err| {
                        anyhow!(
                            "{err}\nSpecial commands are '{0}' and '{1}'.",
                            "remove".important(),
                            "add".important()
                        )
                    })?
                    .for_engine(&self.state)?,
            );
            if self.is_interactive() {
                drop(output);
                self.print_board();
            }
        }
        Ok(())
    }

    fn handle_debug(&mut self, words: &mut Tokens) -> Res<()> {
        match words.next().unwrap_or("on") {
            "on" => {
                self.state.debug_mode = true;
                // make sure to print all the messages that can be sent (adding an existing output is a no-op)
                self.handle_output(&mut tokens("error"))?;
                self.handle_output(&mut tokens("debug"))?;
                self.handle_output(&mut tokens("info"))?;
                self.write_message(Debug, "Debug mode enabled");
                // don't change the log stream if it's already set
                if self
                    .output()
                    .additional_outputs
                    .iter()
                    .any(|o| o.is_logger())
                {
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
                self.write_message(Debug, "Debug mode disabled");
                // don't remove the error output, as there is basically no reason to do so
                self.handle_log(&mut tokens("none"))
            }
            x => bail!("Invalid debug option '{x}'"),
        }
    }

    // This function doesn't depend on the generic parameter, and luckily the rust compiler is smart enough to
    // polymorphize the monomorphed functions again,i.e. it will only generate this function once. So no need to
    // manually move it into a context where it doesn't depend on `B`.
    fn handle_log(&mut self, words: &mut Tokens) -> Res<()> {
        self.output().additional_outputs.retain(|o| !o.is_logger());
        let next = words.peek().copied().unwrap_or_default();
        if next != "off" && next != "none" {
            let logger = LoggerBuilder::from_words(words).for_engine(&self.state)?;
            self.output().additional_outputs.push(logger);
        }
        // write the debug message after setting the logger so that it also gets logged.
        self.write_message(
            Debug,
            &format!(
                "Set the debug logfile to '{}'",
                self.output()
                    .additional_outputs
                    .last()
                    .unwrap()
                    .output_name()
            ),
        );
        Ok(())
    }

    fn print_help(&self) {
        let engine_name = format!(
            "{0} ({1})",
            self.state.display_name.clone().important(),
            self.state.engine.get_engine_info().long_name().important()
        );
        let motors = "motors".important();
        let game_name = B::game_name().important();
        let mut text = format!("{motors}: A work-in-progress collection of engines for various games, \
            currently playing {game_name}, using the engine {engine_name}.\
            \nSeveral commands are supported (see https://backscattering.de/chess/uci/ for a description of the UCI interface):\n\
            \n{}:\n", "UGI Commands".important());
        for cmd in self.commands.ugi.iter().filter(|c| c.standard() != Custom) {
            writeln!(&mut text, " {}", *cmd).unwrap();
        }
        write!(&mut text, "\n{}:\n", "Custom Commands".important()).unwrap();
        for cmd in self.commands.ugi.iter().filter(|c| c.standard() == Custom) {
            writeln!(&mut text, " {}", *cmd).unwrap();
        }
        println!("{text}");
    }

    fn handle_engine(&mut self, words: &mut Tokens) -> Res<()> {
        let Some(engine) = words.next() else {
            let info = self.state.engine.get_engine_info();
            let short = info.short_name();
            let long = info.long_name();
            let description = info.description().unwrap_or_default();
            drop(info);
            self.write_ugi(&format!(
                "\n{alias}: {short}\n{engine}: {long}\n{descr}: {description}",
                alias = "Alias".important(),
                engine = "Engine".important(),
                descr = "Description".important(),
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
            TT::new_with_bytes(self.state.engine.next_tt().size_in_bytes()),
        )?;
        let hash = self.state.engine.next_tt().size_in_mb();
        let threads = self.state.engine.num_threads();
        self.state.engine.send_quit()?;
        // This resets some engine options, but that's probably for the better since the new engine might not support those.
        // However, we make an exception for threads and hash
        self.state.engine = engine;
        // We set those options after changing the engine, so if we get an error this doesn't prevent us from using the new engine.
        self.state.engine.set_option(Hash, hash.to_string())?;
        self.state.engine.set_option(Threads, threads.to_string())?;
        self.write_engine_ascii_art();
        Ok(())
    }

    fn handle_set_eval(&mut self, words: &mut Tokens) -> Res<()> {
        let Some(name) = words.next() else {
            let name = self
                .state
                .engine
                .get_engine_info()
                .eval()
                .clone()
                .map_or_else(|| "<eval unused>".to_string(), |e| e.short_name());
            self.write_ugi(&format!("Current eval: {name}   "));
            return Ok(());
        };
        let eval = create_eval_from_str(name, &self.eval_factories)?.build();
        self.state.engine.set_eval(eval)?;
        self.write_engine_ascii_art();
        Ok(())
    }

    fn handle_play(&mut self, words: &mut Tokens) -> Res<()> {
        let default = self.state.game_name();
        let game_name = words.next().unwrap_or(default);
        let game = select_game(game_name)?;
        let mut opts = EngineOpts::for_game(game, self.state.debug_mode);
        if let Some(word) = words.next() {
            opts.engine = word.to_string();
        }
        if words.peek().is_some() {
            opts.pos_name = Some(words.join(" "));
        }
        let mut nested_match = create_match(opts)?;
        if nested_match.run() == QuitProgram {
            self.handle_quit(QuitProgram)?;
        } else {
            // print the current board again, now that the match is over
            self.print_board();
        }
        Ok(())
    }

    fn write_single_option(option: &EngineOption, res: &mut String) {
        writeln!(
            res,
            "{name}: {value}",
            name = option.name.to_string().important(),
            value = option.value.value_to_str().important()
        )
        .unwrap();
    }

    fn write_option(&self, words: &mut Tokens) -> Res<String> {
        let options = self.get_options();
        if words
            .peek()
            .is_some_and(|next| next.eq_ignore_ascii_case("name"))
        {
            _ = words.next();
        }
        Ok(match words.join(" ").to_ascii_lowercase().as_str() {
            "" => {
                let mut res = format!(
                    "{0} playing {1}\n",
                    self.state.engine.get_engine_info().short_name(),
                    self.state.game_name()
                );
                for o in options {
                    Self::write_single_option(&o, &mut res);
                }
                res
            }
            x => {
                match options
                    .iter()
                    .find(|o| o.name.to_string().eq_ignore_ascii_case(x))
                {
                    Some(opt) => {
                        let mut res = String::new();
                        Self::write_single_option(opt, &mut res);
                        res
                    }
                    None => {
                        bail!(
                            "No option named '{0}' exists. Type '{1}' for a list of options.",
                            x.error(),
                            "ugi".important()
                        )
                    }
                }
            }
        })
    }

    fn write_ugi_options(&self) -> String {
        self.get_options()
            .iter()
            .map(|opt| format!("option {opt}"))
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn get_options(&self) -> Vec<EngineOption> {
        let engine_info = self.state.engine.get_engine_info();
        let engine = engine_info.engine().clone();
        let eval_name = engine_info
            .eval()
            .as_ref()
            .map(|i| i.long_name())
            .unwrap_or("<none>".to_string());
        let max_threads = engine_info.max_threads();
        drop(engine_info);
        let mut res = vec![
            EngineOption {
                name: MoveOverhead,
                value: Spin(UgiSpin {
                    val: self.move_overhead.as_millis() as i64,
                    default: Some(DEFAULT_MOVE_OVERHEAD_MS as i64),
                    min: Some(0),
                    max: Some(10_000),
                }),
            },
            EngineOption {
                name: EngineOptionName::Ponder,
                value: Check(UgiCheck {
                    val: self.allow_ponder,
                    default: Some(false),
                }),
            },
            EngineOption {
                name: MultiPv,
                value: Spin(UgiSpin {
                    val: 1,
                    default: Some(1),
                    min: Some(1),
                    max: Some(256),
                }),
            },
            EngineOption {
                name: UCIOpponent,
                value: UString(UgiString {
                    default: None,
                    val: self.state.opponent_name.clone().unwrap_or_default(),
                }),
            },
            EngineOption {
                name: UCIEngineAbout,
                value: UString(UgiString {
                    val: String::new(),
                    default: Some(format!(
                        "Motors by ToTheAnd. Game: {2}. Engine: {0}. Eval: {1}  ",
                        engine.long,
                        eval_name.clone(),
                        B::game_name()
                    )),
                }),
            },
            EngineOption {
                name: Threads,
                value: Spin(UgiSpin {
                    val: self.state.engine.num_threads() as i64,
                    default: Some(1),
                    min: Some(1),
                    max: Some(max_threads as i64),
                }),
            },
            EngineOption {
                name: Hash,
                value: Spin(UgiSpin {
                    val: self.state.engine.next_tt().size_in_mb() as i64,
                    default: Some(DEFAULT_HASH_SIZE_MB as i64),
                    min: Some(0),
                    max: Some(10_000_000), // use at most 10 terabytes (should be enough for anybody™)
                }),
            },
            EngineOption {
                name: Strictness,
                value: Check(UgiCheck {
                    val: self.strictness == Strict,
                    default: Some(false),
                }),
            },
            EngineOption {
                name: SetEngine,
                value: Combo(UgiCombo {
                    val: engine.short_name(),
                    default: Some(engine.long_name()),
                    options: self
                        .searcher_factories
                        .iter()
                        .map(|s| s.long_name())
                        .collect_vec(),
                }),
            },
            EngineOption {
                name: SetEval,
                value: Combo(UgiCombo {
                    val: eval_name.clone(),
                    default: Some(eval_name),
                    options: self
                        .eval_factories
                        .iter()
                        .map(|e| e.long_name())
                        .collect_vec(),
                }),
            },
        ];
        res.extend(self.state.engine.get_engine_info().additional_options());
        res
    }

    fn write_engine_ascii_art(&mut self) {
        let text = self.state.engine.get_engine_info().short_name();
        let text = print_as_ascii_art(&text, 2).dark_cyan().to_string();
        self.write_ugi(&text);
    }
}

// take a BoardGameState instead of a board to correctly handle displaying the last move
fn format_tt_entry<B: Board>(state: BoardGameState<B>, entry: TTEntry<B>) -> String {
    let pos = state.board;
    let formatter = pos.pretty_formatter(None, state.last_move());
    let mov = entry.mov.check_legal(&pos);
    let mut formatter = AdaptFormatter {
        underlying: formatter,
        color_frame: Box::new(move |coords, color| {
            if let Some(mov) = mov {
                if coords == mov.src_square() || coords == mov.dest_square() {
                    return Some(style::Color::DarkRed);
                }
            };
            color
        }),
        display_piece: Box::new(move |coords, _, default| {
            if let Some(mov) = mov {
                if mov.src_square() == coords {
                    return default.dimmed().to_string();
                } else if mov.dest_square() == coords {
                    return default.important().to_string();
                }
            }
            default
        }),
        horizontal_spacer_interval: None,
        vertical_spacer_interval: None,
        square_width: None,
    };
    let mut res = pos.display_pretty(&mut formatter);
    let move_string = if let Some(mov) = mov {
        mov.to_extended_text(&pos, Standard).important().to_string()
    } else {
        "<none>".to_string()
    };
    let bound_str = entry.bound().comparison_str().important().to_string();
    write!(
        &mut res,
        "\nScore: {bound_str}{0} ({1}), Depth: {2}, Best Move: {3}",
        pretty_score(entry.score, None, &score_gradient(), true, false),
        entry.bound(),
        entry.depth.to_string().important(),
        move_string,
    )
    .unwrap();
    res
}

fn show_eval_pos<B: Board>(pos: B, last: Option<B::Move>, eval: Box<dyn Eval<B>>) -> String {
    let eval = RefCell::new(eval);
    let formatter = pos.pretty_formatter(None, last);
    let eval_pos = eval.borrow_mut().eval(&pos);
    let mut formatter = AdaptFormatter {
        underlying: formatter,
        color_frame: Box::new(|_, col| col),
        display_piece: Box::new(move |coords, _, default| {
            let piece = pos.colored_piece_on(coords);
            let Some(color) = piece.color() else {
                return default;
            };
            let piece = format!(
                "{}:",
                piece.to_ascii_char().to_string().with(display_color(color))
            );
            let score = match pos.remove_piece(coords).unwrap().verify(Relaxed) {
                Ok(pos) => {
                    let diff = eval_pos - eval.borrow_mut().eval(&pos);
                    let (val, suffix) = suffix_for(diff.0 as isize, Some(10_000));
                    // reduce the scale by some scale because we expect pieces values to be much larger
                    // than eval values. The ideal scale depends on the game and eval,
                    let score_color =
                        color_for_score(diff / eval.borrow().piece_scale(), &score_gradient());
                    format!("{:>5}", format!("{val:>3}{suffix}"))
                        .with(score_color)
                        .to_string()
                }
                Err(_) => " None".bold().to_string(),
            };
            format!("{0}{1}", piece, score)
        }),
        horizontal_spacer_interval: None,
        vertical_spacer_interval: None,
        square_width: Some(7),
    };
    pos.display_pretty(&mut formatter)
}
