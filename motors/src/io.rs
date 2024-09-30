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
pub mod cli;
mod command;
pub mod ugi_output;

use std::fmt::{Debug, Display, Formatter, Write};
use std::io::stdin;
use std::iter::Peekable;
use std::ops::{Deref, DerefMut};
use std::str::{FromStr, SplitWhitespace};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use colored::Colorize;
use itertools::Itertools;

use gears::cli::{select_game, Game};
use gears::games::{BoardHistory, Color, OutputList, ZobristHistory};
use gears::general::board::Board;
use gears::general::common::anyhow::{anyhow, bail};
use gears::general::common::Description::{NoDescription, WithDescription};
use gears::general::common::{
    parse_bool_from_str, parse_duration_ms, parse_int_from_str, select_name_dyn,
    to_name_and_optional_description, NamedEntity,
};
use gears::general::common::{IterIntersperse, Res};
use gears::general::moves::Move;
use gears::general::perft::{perft, perft_for, split_perft};
use gears::output::logger::LoggerBuilder;
use gears::output::Message::*;
use gears::output::{Message, OutputBuilder};
use gears::search::{Depth, SearchLimit, TimeControl};
use gears::ugi::EngineOptionName::*;
use gears::ugi::EngineOptionType::*;
use gears::ugi::{
    load_ugi_position, parse_ugi_position_part, EngineOption, EngineOptionName, UgiCheck, UgiSpin,
    UgiString,
};
use gears::MatchStatus::*;
use gears::Quitting::QuitProgram;
use gears::{output_builder_from_str, AbstractRun, GameResult, GameState, MatchStatus, Quitting};

use crate::io::cli::EngineOpts;
use crate::io::command::Standard::Custom;
use crate::io::command::{go_commands, ugi_commands, CommandTrait, GoState};
use crate::io::ugi_output::UgiOutput;
use crate::io::ProgramStatus::{Quit, Run};
use crate::io::Protocol::Interactive;
use crate::io::SearchType::*;
use crate::search::multithreading::EngineWrapper;
use crate::search::tt::{DEFAULT_HASH_SIZE_MB, TT};
use crate::search::{run_bench_with, EvalList, SearcherList};
use crate::{
    create_engine_bench_from_str, create_engine_from_str, create_eval_from_str, create_match,
};

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
    fn make_move(&mut self, mov: B::Move) -> Res<()> {
        if !self.board.is_move_pseudolegal(mov) {
            bail!("Illegal move {mov} (not pseudolegal)")
        }
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
        Ok(())
    }

    fn clear_state(&mut self) {
        self.board = self.initial_pos;
        self.mov_hist.clear();
        self.board_hist.clear();
        self.status = Run(NotStarted);
    }

    fn handle_position(&mut self, words: &mut Peekable<SplitWhitespace>) -> Res<()> {
        self.initial_pos = parse_ugi_position_part(words, &self.board)?;
        self.clear_state();

        let Some(word) = words.next() else {
            return Ok(());
        };
        if word != "moves" && word != "m" {
            bail!("Unrecognized word '{word}' after position command, expected either 'moves', 'm', or nothing")
        }
        for mov in words {
            let mov = B::Move::from_compact_text(mov, &self.board)
                .map_err(|err| anyhow!("Couldn't parse move '{}': {err}", mov.red()))?;
            self.make_move(mov)?;
        }
        // TODO: Handle flip / nullmove?
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

/// Implements both UGI and UCI.
#[derive(Debug)]
pub struct EngineUGI<B: Board> {
    state: EngineGameState<B>,
    commands: Vec<Box<dyn CommandTrait<Self>>>,
    output: Arc<Mutex<UgiOutput<B>>>,
    output_factories: OutputList<B>,
    searcher_factories: SearcherList<B>,
    eval_factories: EvalList<B>,
    move_overhead: Duration,
    multi_pv: usize,
    allow_ponder: bool,
}

impl<B: Board> AbstractRun for EngineUGI<B> {
    fn run(&mut self) -> Quitting {
        self.ugi_loop()
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
            Quit(_) => {
                panic!("It shouldn't be possible to call match_status when quitting the engine.")
            }
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
        let output = Arc::new(Mutex::new(UgiOutput::default()));
        let board = B::default();
        let engine = create_engine_from_str(
            &opts.engine,
            &all_searchers,
            &all_evals,
            output.clone(),
            TT::default(),
        )?;
        let display_name = engine.engine_info().short_name();
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
        let state = EngineGameState {
            board_state,
            game_name: B::game_name(),
            protocol: Protocol::default(),
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
            commands: ugi_commands(),
            output,
            output_factories: all_output_builders,
            searcher_factories: all_searchers,
            eval_factories: all_evals,
            move_overhead: Duration::from_millis(DEFAULT_MOVE_OVERHEAD_MS),
            multi_pv: 1,
            allow_ponder: false,
        })
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
        loop {
            let mut input = String::default();
            // If reading the input failed, always terminate. This probably means that the pipe is broken or similar,
            // so there's no point in continuing.
            match stdin().read_line(&mut input) {
                Ok(count) => {
                    if count == 0 {
                        self.write_message(Debug, "Read 0 bytes. Terminating the program.");
                        break;
                    }
                }
                Err(e) => {
                    self.write_message(Error, &format!("Failed to read input: {e}"));
                    break;
                }
            }

            let res = self.handle_input(input.split_whitespace().peekable());
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

    fn handle_input(&mut self, mut words: Peekable<SplitWhitespace>) -> Res<()> {
        self.output().write_ugi_input(words.clone());
        let words = &mut words;
        let Some(first_word) = words.next() else {
            return Ok(()); // ignore empty input
        };
        let Ok(cmd) = select_name_dyn(
            first_word,
            &self.commands,
            "command",
            &self.state.game_name(),
            NoDescription,
        ) else {
            if first_word.eq_ignore_ascii_case("barbecue") {
                self.write_message(Error, "lol");
            }
            // The original UCI spec demands that unrecognized tokens should be ignored, whereas the
            // expositor UCI spec demands that an invalid token should cause the entire message to be ignored.
            self.write_message(
                Warning,
                &format!(
                    "Invalid token at start of UCI command '{0}', ignoring the entire command. \
                    If you are a human, consider typing {1} to see a list of recognized commands.",
                    first_word.red(),
                    "help".bold()
                ),
            );
            return Ok(());
        };
        () = cmd.func()(self, words, first_word)?;
        if let Some(remaining) = words.next() {
            self.write_message(
                Warning,
                &format!(
                    "Ignoring trailing input starting with '{}'",
                    remaining.bold()
                ),
            );
        }
        Ok(())
    }

    fn quit(&mut self, typ: Quitting) -> Res<()> {
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
        Ok(())
    }

    fn id(&self) -> String {
        let info = self.state.engine.engine_info();
        format!(
            "id name Motors -- Game {0} -- Engine {1}\nid author ToTheAnd",
            B::game_name(),
            info.long_name(),
        )
    }

    fn handle_list(&mut self, words: &mut Peekable<SplitWhitespace>) -> Res<()> {
        // TODO: Use thiserror and anyhow, detect errors from non-matching name and print them directly instead of
        // actually returning an error. Detect invalid commands after 'list'.
        // long term, create a `Command` struct and replace the match with a select_name call?
        let Some(next) = words.next() else {
            bail!("Expected a word after '{}'", "list".bold())
        };
        if next.eq_ignore_ascii_case("list") {
            bail!(
                "'{}' is not a valid command; write something other than 'list' after 'list'",
                "list list".red()
            )
        }
        let mut rearranged = next.to_string() + " list ";
        rearranged += &words.intersperse_(" ").collect::<String>();
        _ = self.handle_input(rearranged.split_whitespace().peekable());
        Ok(())
    }

    fn handle_setoption(&mut self, words: &mut Peekable<SplitWhitespace>) -> Res<()> {
        let mut name = words.next().unwrap_or_default().to_ascii_lowercase();
        if name != "name" {
            bail!(
                "Invalid 'setoption' command: Expected 'name', got '{};",
                name.red()
            )
        }
        name = String::default();
        loop {
            let next_word = words.next().unwrap_or_default();
            if next_word.to_ascii_lowercase() == "value" || next_word.is_empty() {
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
                self.move_overhead =
                    parse_duration_ms(&mut value.split_whitespace().peekable(), "move overhead")?;
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

    fn accept_depth(limit: &mut SearchLimit, words: &mut Peekable<SplitWhitespace>) {
        if let Some(word) = words.peek() {
            if let Ok(number) = parse_int_from_str(word, "depth") {
                limit.depth = Depth::new(number);
                _ = words.next();
            }
        }
    }

    fn handle_go(
        &mut self,
        search_type: SearchType,
        words: &mut Peekable<SplitWhitespace>,
    ) -> Res<()> {
        // "infinite" is the identity element of the bounded semilattice of `go` options
        // let mut limit = SearchLimit::infinite();
        // let is_first = self.state.board.active_player().is_first();
        // let mut multi_pv = self.multi_pv;
        // let mut search_moves = None;
        // let mut reading_moves = false;
        // let mut complete = false;
        // let mut board = self.state.board;

        let mut state = GoState::new(self, search_type, self.move_overhead);

        if matches!(search_type, Perft | SplitPerft | Bench) {
            Self::accept_depth(&mut state.limit, words);
        }
        // TODO: Don't recreate each time

        let cmds = go_commands::<B>();
        while let Some(option) = words.next() {
            state.cont = false;
            let cmd = select_name_dyn(
                option,
                &cmds,
                "go option",
                &self.state.game_name,
                WithDescription,
            );
            match cmd {
                Ok(cmd) => cmd.func()(&mut state, words, option)?,
                Err(err) => {
                    if state.reading_moves {
                        let mov =
                            B::Move::from_compact_text(option, &state.board).map_err(|err| {
                                anyhow!("{err}. '{}' is not a valid 'go' option.", option.bold())
                            })?;    
                        state.search_moves.as_mut().unwrap().push(mov);
                        continue;
                    }
                    bail!(err)
                }
            }
            if !state.cont {
                state.reading_moves = false;
            }
        }
        state.limit.tc.remaining = state
            .limit
            .tc
            .remaining
            .saturating_sub(self.move_overhead)
            .max(Duration::from_millis(1));

        if (search_type == Perft || search_type == SplitPerft) && state.limit.depth == Depth::MAX {
            state.limit.depth = state.board.default_perft_depth();
        }

        if state.complete {
            match search_type {
                Bench => {
                    let mut engine = create_engine_bench_from_str(
                        &self.state.engine.engine_info().short_name(),
                        &self.searcher_factories,
                        &self.eval_factories,
                    )?;
                    let res = run_bench_with(
                        engine.as_mut(),
                        state.limit,
                        Some(SearchLimit::nodes(
                            self.state.engine.engine_info().default_bench_nodes(),
                        )),
                    );
                    self.output().write_ugi(&res.to_string())
                }
                Perft => self
                    .output()
                    .write_ugi(&perft_for(state.limit.depth, B::bench_positions()).to_string()),
                _ => {
                    bail!(
                        "Can only use the '{}' option with 'bench' or 'perft'",
                        "complete".bold()
                    )
                }
            }
        }
        self.start_search(
            search_type,
            state.limit,
            state.board,
            state.search_moves,
            state.multi_pv,
        )
    }

    fn start_search(
        &mut self,
        search_type: SearchType,
        mut limit: SearchLimit,
        pos: B,
        moves: Option<Vec<B::Move>>,
        multi_pv: usize,
    ) -> Res<()> {
        self.write_message(
            Debug,
            &format!("Starting {search_type} search with limit {limit}"),
        );
        if let Some(res) = pos.match_result_slow(&self.state.board_hist) {
            self.write_message(Warning, &format!("Starting a {search_type} search in position '{2}', but the game is already over. {0}, reason: {1}.",
                                                 res.result, res.reason, self.state.board.as_fen().bold()));
        }
        if cfg!(feature = "fuzzing") {
            limit.fixed_time = limit.fixed_time.max(Duration::from_secs(2));
            if matches!(search_type, Perft | SplitPerft) {
                limit.depth = limit.depth.min(Depth::new(3));
            }
        }
        self.state.status = Run(Ongoing);
        match search_type {
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
                    pos,
                    limit,
                    self.state.board_hist.clone(),
                    moves,
                    multi_pv,
                    false,
                )?;
            }
            SearchType::Ponder => {
                self.state.ponder_limit = Some(limit);
                self.state.engine.start_search(
                    pos,
                    SearchLimit::infinite(), //always allocate infinite time for pondering
                    self.state.board_hist.clone(),
                    moves,
                    multi_pv, // don't ignore multi_pv in pondering mode
                    true,
                )?;
            }
            Perft => {
                let msg = format!("{0}", perft(limit.depth, pos));
                self.write_ugi(&msg);
            }
            SplitPerft => {
                if limit.depth.get() == 0 {
                    bail!("{} requires a depth of at least 1", "splitperft".bold())
                }
                let msg = format!("{0}", split_perft(limit.depth, pos));
                self.write_ugi(&msg);
            }
            Bench => {
                self.state
                    .engine
                    .start_bench(pos, limit)
                    .expect("bench panic");
            }
        };
        Ok(())
    }

    fn handle_eval_or_tt(&mut self, eval: bool, words: &mut Peekable<SplitWhitespace>) -> Res<()> {
        let board = if words.peek().copied().is_some() {
            if matches!(*words.peek().unwrap(), "position" | "pos" | "p") {
                words.next();
            }
            let mut board_state_clone = self.state.board_state.clone();
            board_state_clone.handle_position(words)?;
            board_state_clone.board
        } else {
            self.state.board
        };
        if eval {
            self.state.engine.static_eval(board)
        } else {
            self.state.engine.tt_entry(board)
        }
    }

    fn handle_query(&mut self, words: &mut Peekable<SplitWhitespace>) -> Res<()> {
        match *words
            .peek()
            .ok_or(anyhow!("Missing argument to '{}'", "query".bold()))?
        {
            "gameover" => self
                .output()
                .write_response(&matches!(self.state.status, Run(Ongoing)).to_string()),
            "p1turn" => self
                .output()
                .write_response(&(self.state.board.active_player().is_first()).to_string()),
            "p2turn" => self
                .output()
                .write_response(&(!self.state.board.active_player().is_first()).to_string()),
            "result" => {
                let response = match &self.state.status {
                    Run(Over(res)) => match res.result {
                        GameResult::P1Win => "p1win",
                        GameResult::P2Win => "p2win",
                        GameResult::Draw => "draw",
                        GameResult::Aborted => "aborted",
                    },
                    _ => "none",
                };
                self.output().write_response(response);
            }
            s => {
                let Ok(opt) = self.write_option(words) else {
                    bail!("unrecognized query option {s}")
                };
                self.write_ugi(&opt);
            }
        }
        Ok(())
    }

    fn handle_print(&mut self, words: &mut Peekable<SplitWhitespace>) -> Res<()> {
        match words.next() {
            None => {
                if !self.output().show(&self.state) {
                    return self.handle_print(&mut "unicode".split_whitespace().peekable());
                }
            }
            Some(name) => {
                // This is definitely not the fastest way to print something, but performance isn't a huge concern here
                // TODO: Allow `print (pos) kiwipete` without specifying the output
                let mut output = output_builder_from_str(name, &self.output_factories)?
                    .for_engine(&self.state)?;
                let old_board = self.state.board;
                match words.next() {
                    None => {}
                    Some("position" | "pos" | "p") => {
                        self.state.board = load_ugi_position(words, &self.state.board)?;
                    }
                    Some(x) => {
                        let Ok(new_board) = load_ugi_position(words, &self.state.board) else {
                            bail!(
                                "Unrecognized input '{x}' after valid print command, should be either nothing or a valid 'position' command"
                            )
                        };
                        self.state.board = new_board;
                    }
                }
                output.show(&self.state);
                self.state.board = old_board;
            }
        }
        Ok(())
    }

    fn handle_output(&mut self, words: &mut Peekable<SplitWhitespace>) -> Res<()> {
        let next = words.next().unwrap_or_default();
        let output_ptr = self.output.clone();
        let mut output = output_ptr.lock().unwrap();
        if next.eq_ignore_ascii_case("remove") {
            let Some(next) = words.next() else {
                bail!("No output to remove specified. Use 'all' to remove all outputs")
            };
            if next.eq_ignore_ascii_case("all") {
                output.additional_outputs.clear();
            } else {
                output
                    .additional_outputs
                    .retain(|o| !o.short_name().eq_ignore_ascii_case(next));
            }
        } else if next.eq_ignore_ascii_case("list") {
            // TODO: Remove, use the general list command
            for o in &output.additional_outputs {
                print!(
                    "{}",
                    to_name_and_optional_description(o.as_ref(), WithDescription)
                );
            }
            println!();
        } else if !output
            .additional_outputs
            .iter()
            .any(|o| o.short_name().eq_ignore_ascii_case(next))
        {
            output.additional_outputs.push(
                output_builder_from_str(next, &self.output_factories)
                    .map_err(|err| {
                        anyhow!(
                            "{err}\nSpecial commands are '{0}' and '{1}'.",
                            "remove".bold(),
                            "list".bold()
                        )
                    })?
                    .for_engine(&self.state)?,
            );
        }
        Ok(())
    }

    fn handle_debug(&mut self, words: &mut Peekable<SplitWhitespace>) -> Res<()> {
        match words.next().unwrap_or("on") {
            "on" => {
                self.state.debug_mode = true;
                // make sure to print all the messages that can be sent (adding an existing output is a no-op)
                self.handle_output(&mut "error".split_whitespace().peekable())?;
                self.handle_output(&mut "debug".split_whitespace().peekable())?;
                self.handle_output(&mut "info".split_whitespace().peekable())?;
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
                    self.handle_log(&mut "".split_whitespace().peekable())
                        .map_err(|err| anyhow!("Couldn't set the debug log file: '{err}'"))?;
                    Ok(())
                }
            }
            "off" => {
                self.state.debug_mode = false;
                _ = self.handle_output(&mut "remove debug".split_whitespace().peekable());
                _ = self.handle_output(&mut "remove info".split_whitespace().peekable());
                self.write_message(Debug, "Debug mode disabled");
                // don't remove the error output, as there is basically no reason to do so
                self.handle_log(&mut "none".split_whitespace().peekable())
            }
            x => bail!("Invalid debug option '{x}'"),
        }
    }

    // This function doesn't depend on the generic parameter, and luckily the rust compiler is smart enough to
    // polymorphize the monomorphed functions again,i.e. it will only generate this function once. So no need to
    // manually move it into a context where it doesn't depend on `B`.
    fn handle_log(&mut self, words: &mut Peekable<SplitWhitespace>) -> Res<()> {
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
            self.state.display_name.bold(),
            self.state.engine.engine_info().long_name().bold()
        );
        let motors = "motors".bold();
        let game_name = B::game_name().bold();
        let mut text = format!("{motors}: A work-in-progress collection of engines for various games, \
            currently playing {game_name}, using the engine {engine_name}.\
            \nSeveral commands are supported (see https://backscattering.de/chess/uci/ for a description of the UCI interface):\n\
            \n{}:\n", "UGI Commands".bold());
        for cmd in self.commands.iter().filter(|c| c.standard() != Custom) {
            writeln!(&mut text, " {}", *cmd).unwrap();
        }
        write!(&mut text, "\n{}:\n", "Custom Commands".bold()).unwrap();
        for cmd in self.commands.iter().filter(|c| c.standard() == Custom) {
            writeln!(&mut text, " {}", *cmd).unwrap();
        }
        println!("{text}");
    }

    fn handle_engine(&mut self, words: &mut Peekable<SplitWhitespace>) -> Res<()> {
        let Some(engine) = words.next() else {
            let info = self.state.engine.engine_info();
            let short = info.short_name();
            let long = info.long_name();
            let description = info.description().unwrap_or_default();
            drop(info);
            self.write_ugi(&format!(
                "\n{alias}: {short}\n{engine}: {long}\n{descr}: {description}",
                alias = "Alias".bold(),
                engine = "Engine".bold(),
                descr = "Description".bold(),
            ));
            return Ok(());
        };
        if let Some(game) = words.next() {
            return self.handle_play(&mut format!("{game} {engine}").split_whitespace().peekable());
        }
        // catch invalid names before committing to shutting down the current engine
        let engine = create_engine_from_str(
            engine,
            &self.searcher_factories,
            &self.eval_factories,
            self.output.clone(),
            TT::new_with_bytes(self.state.engine.next_tt().size_in_bytes()),
        )?;
        self.state.engine.send_quit()?;
        self.state.engine = engine;
        Ok(())
    }

    fn handle_set_eval(&mut self, words: &mut Peekable<SplitWhitespace>) -> Res<()> {
        let Some(name) = words.next() else {
            let name = self
                .state
                .engine
                .engine_info()
                .eval()
                .clone()
                .map_or_else(|| "<eval unused>".to_string(), |e| e.short_name());
            self.write_ugi(&format!("Current eval: {name}"));
            return Ok(());
        };
        let eval = create_eval_from_str(name, &self.eval_factories)?.build();
        self.state.engine.set_eval(eval)
    }

    fn handle_play(&mut self, words: &mut Peekable<SplitWhitespace>) -> Res<()> {
        let default = Game::default().to_string();
        let game_name = words.next().unwrap_or(&default);
        let game = select_game(game_name)?;
        let mut opts = EngineOpts::for_game(game, self.state.debug_mode);
        if let Some(word) = words.next() {
            opts.engine = word.to_string();
        }
        let mut nested_match = create_match(opts)?;
        if cfg!(feature = "fuzzing") {
            return Ok(()); // TODO: Allow fuzzing this as well
        }
        if nested_match.run() == QuitProgram {
            self.quit(QuitProgram)?;
        }
        Ok(())
    }

    fn write_single_option(option: &EngineOption, res: &mut String) {
        writeln!(
            res,
            "{name}: {value}",
            name = option.name.to_string().bold(),
            value = option.value.value_to_str().bold()
        )
        .unwrap();
    }

    fn write_option(&self, words: &mut Peekable<SplitWhitespace>) -> Res<String> {
        let options = self.get_options();
        Ok(
            match words
                .intersperse_(" ")
                .collect::<String>()
                .to_ascii_lowercase()
                .as_str()
            {
                "" => {
                    let mut res = String::default();
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
                            Self::write_single_option(&opt, &mut res);
                            res
                        }
                        None => {
                            bail!(
                                "No option named '{0}' exists. Type '{1}' for a list of options.",
                                x.red(),
                                "ugi".bold()
                            )
                        }
                    }
                }
            },
        )
    }

    fn write_ugi_options(&self) -> String {
        self.get_options()
            .iter()
            .map(|opt| format!("option {opt}"))
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn get_options(&self) -> Vec<EngineOption> {
        let engine_info = self.state.engine.engine_info();
        let engine = engine_info.engine().clone();
        let eval_info = engine_info.eval().clone();
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
                        eval_info.map_or_else(|| "<none>".to_string(), |i| i.long),
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
        ];
        res.extend(self.state.engine.engine_info().additional_options());
        res
    }
}
