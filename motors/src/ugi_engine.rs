use std::fmt::{Debug, Display, Formatter};
use std::io::stdin;
use std::str::{FromStr, SplitWhitespace};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use colored::Colorize;
use crossbeam_channel::select;
use itertools::Itertools;

use gears::games::Color::White;
use gears::games::{Board, BoardHistory, Color, Move, OutputList, ZobristRepetition3Fold};
use gears::general::common::parse_int;
use gears::general::common::Res;
use gears::general::perft::{perft, split_perft};
use gears::output::logger::LoggerBuilder;
use gears::output::Message::*;
use gears::output::{Message, OutputBox, OutputBuilder};
use gears::search::{Depth, Nodes, SearchInfo, SearchLimit, SearchResult, TimeControl};
use gears::ugi::EngineOptionName::Threads;
use gears::ugi::{parse_ugi_position, EngineOptionName};
use gears::MatchStatus::*;
use gears::{output_builder_from_str, AbstractRun, AnyMatch, GameResult, GameState, MatchStatus};

use crate::cli::EngineOpts;
use crate::create_engine_from_str;
use crate::search::multithreading::{EngineWrapper, Receiver, SearchSender, Sender};
use crate::search::{BenchResult, EngineList};
use crate::ugi_engine::ProgramStatus::{Quit, Run};
use crate::ugi_engine::SearchType::*;

// TODO: Ensure this conforms to https://expositor.dev/uci/doc/uci-draft-1.pdf

fn ugi_input_thread(sender: Sender<Res<String>>) {
    loop {
        let mut input = String::default();
        match stdin().read_line(&mut input) {
            Ok(count) => {
                if count == 0 {
                    break;
                }
                if sender.send(Ok(input)).is_err() {
                    break;
                }
            }
            Err(e) => {
                _ = sender.send(Err(format!("Failed to read input: {e}")));
                break;
            }
        }
    }
}

enum SearchType {
    Normal,
    Perft,
    SplitPerft,
    Bench,
}

impl Display for SearchType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Normal => "normal",
                Perft => "perft",
                SplitPerft => "split perft",
                Bench => "bench",
            }
        )
    }
}

#[derive(Debug, Clone)]
enum ProgramStatus {
    Run(MatchStatus),
    Quit,
}

#[derive(Debug)]
struct EngineGameState<B: Board> {
    engine: EngineWrapper<B>,
    board: B,
    debug_mode: bool,
    status: ProgramStatus,
    mov_hist: Vec<B::Move>,
    board_hist: ZobristRepetition3Fold,
    initial_pos: B,
    /// This doesn't have to be the UGI engine name. It often isn't, especially when two engines with
    /// the same name play against each other, such as in a SPRT. It should be unique, however
    /// (the `monitors` client ensures that, but another GUI might not).
    display_name: String,
    last_played_color: Color,
}

// TODO: Keep this is a global object instead? Would make it easier to print warnings from anywhere, simplify search sender design

#[derive(Debug, Default)]
/// All UGI communication is done through stdout, but there can be additional outputs,
/// such as a logger, or human-readable printing to stderr
pub struct UgiOutput<B: Board> {
    additional_outputs: Vec<OutputBox<B>>,
}

impl<B: Board> UgiOutput<B> {
    /// Part of the UGI specification, but not the UCI specification

    fn write_response(&mut self, response: String) {
        self.write_ugi(&format!("response {response}"))
    }

    pub fn write_ugi(&mut self, message: &str) {
        // UGI is always done through stdin and stdout, no matter what the UI is.
        // TODO: Keep stdout mutex? Might make printing slightly faster and prevents everyone else from
        // accessing stdout, which is probably a good thing because it prevents sending invalid UCI commands
        println!("{message}");
        for output in self.additional_outputs.iter_mut() {
            output.write_ugi_output(message, None);
        }
    }

    fn write_ugi_input(&mut self, msg: SplitWhitespace) {
        for output in self.additional_outputs.iter_mut() {
            output.write_ugi_input(msg.clone(), None)
        }
    }

    pub fn write_message(&mut self, typ: Message, msg: &str) {
        for output in self.additional_outputs.iter_mut() {
            output.display_message_simple(typ, msg);
        }
    }

    pub fn show_bench(&mut self, bench_result: BenchResult) {
        self.write_ugi(&format!(
            "depth {0}, time {2}ms, {1} nodes, {3} nps",
            bench_result.depth.get(),
            bench_result.nodes,
            bench_result.time.as_millis(),
            ((bench_result.nodes.get() as f64 / bench_result.time.as_millis() as f64 * 1000.0)
                .round())
            .to_string()
            .red()
        ));
    }

    pub fn show_search_res(&mut self, search_result: SearchResult<B>) {
        self.write_ugi(&format!(
            "bestmove {best}",
            best = search_result.chosen_move.to_compact_text()
        ));
    }

    pub fn show_search_info(&mut self, info: SearchInfo<B>) {
        self.write_ugi(&info.to_string());
    }
}

// Implement both UGI and UCI
#[derive(Debug)]
pub struct EngineUGI<B: Board> {
    state: EngineGameState<B>,
    output: Arc<Mutex<UgiOutput<B>>>,
    next_match: Option<AnyMatch>,
    output_factories: OutputList<B>,
}

impl<B: Board> AbstractRun for EngineUGI<B> {
    fn run(&mut self) {
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

    fn move_history(&self) -> &[B::Move] {
        &self.mov_hist
    }

    fn match_status(&self) -> MatchStatus {
        match self.status.clone() {
            Run(status) => status,
            Quit => {
                panic!("It shouldn't be possible to call match_status when quitting the engine.")
            }
        }
    }

    fn debug_info_enabled(&self) -> bool {
        self.debug_mode
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

    fn player_name(&self, color: Color) -> Option<&str> {
        if color == self.last_played_color {
            Some(self.name())
        } else {
            None // TODO: Get the opponent's name from UGI? There's probably a way because that's required for contempt
        }
    }

    fn time(&self, _color: Color) -> Option<TimeControl> {
        // Technically, we get the time with 'go', but we can't trust it for the other player,
        // and we don't really need this for ourselves while we're thinking
        None
    }

    fn thinking_since(&self, _color: Color) -> Option<Instant> {
        None
    }
}

impl<B: Board> EngineUGI<B> {
    pub fn create(
        opts: EngineOpts,
        selected_output_builders: OutputList<B>,
        all_output_builders: OutputList<B>,
        all_engines: EngineList<B>,
    ) -> Res<Self> {
        let output = Arc::new(Mutex::new(UgiOutput::default()));
        let sender = SearchSender::new(output.clone());
        let board = B::default();
        let engine = create_engine_from_str(&opts.engine, &all_engines, sender)?;
        let state = EngineGameState {
            engine,
            board,
            debug_mode: opts.debug,
            status: Run(NotStarted),
            mov_hist: vec![],
            board_hist: ZobristRepetition3Fold::default(),
            initial_pos: B::default(),
            display_name: opts.engine,
            last_played_color: Default::default(),
        };
        for builder in selected_output_builders {
            output
                .lock()
                .unwrap()
                .additional_outputs
                .push(builder.for_engine(&state)?);
        }
        Ok(Self {
            state,
            output,
            next_match: None,
            output_factories: all_output_builders,
        })
    }

    fn output(&self) -> MutexGuard<UgiOutput<B>> {
        self.output.lock().unwrap()
    }

    fn clear_state(&mut self) {
        self.state.board = self.state.initial_pos;
        self.state.mov_hist.clear();
        <ZobristRepetition3Fold as BoardHistory<B>>::clear(&mut self.state.board_hist);
        self.state.status = Run(NotStarted);
    }

    fn handle_ugi(&mut self, stdin_receiver: Receiver<Res<String>>) -> Res<ProgramStatus> {
        select! {
            recv(stdin_receiver) -> input =>
                self.parse_input(input.map_err(|err| err.to_string())??.split_whitespace()),
        }
    }

    fn ugi_loop(&mut self) {
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

            let res = self.parse_input(input.split_whitespace());
            if let Err(err) = res {
                self.write_message(Error, err.as_str());
                if self.continue_on_error() {
                    self.write_message(Debug, "Continuing... ('debug' is 'on')");
                    continue;
                }
                return;
            }
            if let Ok(Quit) = res {
                return;
            }
        }
    }

    fn write_ugi(&mut self, message: &str) {
        self.output().write_ugi(message)
    }

    fn write_message(&mut self, typ: Message, msg: &str) {
        self.output().write_message(typ, msg)
    }

    fn continue_on_error(&self) -> bool {
        self.state.debug_mode
    }

    fn parse_input(&mut self, mut words: SplitWhitespace) -> Res<ProgramStatus> {
        self.output().write_ugi_input(words.clone());
        let first_word = words.next().ok_or("Empty input")?;
        match first_word {
            "go" => {
                self.handle_go(words)?;
            }
            "position" => {
                self.handle_position(words)?;
            }
            "ugi" => {
                let id_msg = self.id();
                self.write_ugi(id_msg.as_str());
                self.write_ugi(self.write_options().as_str());
                self.write_ugi("ugiok");
            }
            "uci" => {
                let id_msg = self.id();
                self.write_ugi(id_msg.as_str());
                self.write_ugi(self.write_options().as_str());
                self.write_ugi("uciok");
            }
            "isready" => {
                self.write_ugi("readyok");
            }
            "debug" => {
                match words.next().unwrap_or("on") {
                    "on" => {
                        self.state.debug_mode = true;
                        // don't change the log stream if it's already set
                        if !self
                            .output()
                            .additional_outputs
                            .iter()
                            .any(|o| o.is_logger())
                        {
                            if let Err(msg) = self.handle_log("".split_whitespace()) {
                                // Don't return an error, instead simply print a message and continue.
                                self.write_message(
                                    Error,
                                    &format!("Couldn't set the debug log file: '{msg}'"),
                                );
                            };
                        }
                    }
                    "off" => {
                        self.state.debug_mode = false;
                        self.handle_log("none".split_whitespace())?;
                    }
                    x => return Err(format!("Invalid debug option '{x}'")),
                }
            }
            "setoption" => {
                self.handle_setoption(words)?;
            }
            "register" => return Err("'register' isn't supported".to_string()),
            "ucinewgame" => {
                self.state.engine.send_forget()?;
                self.state.status = Run(NotStarted);
            }
            "stop" => {
                self.state.engine.send_stop();
            }
            "ponderhit" => {} // ignore pondering
            "quit" => {
                self.quit()?;
            }
            "query" => {
                self.handle_query(words)?;
            }
            "output" => {
                self.handle_ui(words)?;
            }
            "remove_uis" => {
                self.handle_remove_uis()?;
            }
            "print" => {
                self.handle_print(words)?;
            }
            "log" => {
                self.handle_log(words)?;
            }
            "engine" => {
                self.handle_engine(words)?;
            }
            "play" => {
                todo!("remove?");
            }
            x => {
                // The original UCI spec demands that unrecognized tokens should be ignored, whereas the
                // expositor UCI spec demands that an invalid token should cause the entire message to be ignored.
                self.write_message(
                    Warning,
                    &format!(
                        "Invalid token at start of UCI command '{x}', ignoring the entire command"
                    ),
                );
            }
        }
        Ok(self.state.status.clone())
    }

    fn quit(&mut self) -> Res<()> {
        self.next_match = None;
        self.state.engine.send_quit()?;
        self.state.status = Quit;
        Ok(())
    }

    fn id(&self) -> String {
        let info = self.state.engine.engine_info();
        format!(
            "id name Motors - {0} {1}\nid author ToTheAnd",
            info.name, info.version
        )
    }

    fn handle_setoption(&mut self, mut words: SplitWhitespace) -> Res<()> {
        let mut name = words.next().unwrap_or_default().to_ascii_lowercase();
        if name != "name" {
            return Err(format!(
                "Invalid 'setoption' command: Expected 'name', got '{name};"
            ));
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
        Ok(())
    }

    fn handle_go(&mut self, mut words: SplitWhitespace) -> Res<()> {
        let mut limit = SearchLimit::infinite();
        let is_white = self.state.board.active_player() == White;
        let mut search_type = Normal;
        while let Some(next_word) = words.next() {
            match next_word {
                // TODO: Add "eval" UCI command to print the static eval (probably requires adapting the `Engine` trait)
                "searchmoves" => {
                    return Err("The 'go searchmoves' option is not implemented".to_string());
                }
                "ponder" => return Err("Pondering is not (yet?) implemented".to_string()),
                "wtime" | "p1time" => {
                    let time =
                        Duration::from_millis(parse_int(&mut words, "'wtime' milliseconds")?);
                    if is_white {
                        // always parse the int, even if it isn't relevant
                        limit.tc.remaining = time;
                    }
                }
                "btime" | "p2time" => {
                    let time =
                        Duration::from_millis(parse_int(&mut words, "'btime' milliseconds")?);
                    if !is_white {
                        limit.tc.remaining = time;
                    }
                }
                "winc" | "p1inc" => {
                    let increment =
                        Duration::from_millis(parse_int(&mut words, "'winc' milliseconds")?);
                    if is_white {
                        limit.tc.increment = increment;
                    }
                }
                "binc" | "p2inc" => {
                    let increment =
                        Duration::from_millis(parse_int(&mut words, "'binc' milliseconds")?);
                    if !is_white {
                        limit.tc.increment = increment;
                    }
                }
                "movestogo" => {
                    limit.tc.moves_to_go = Some(parse_int(&mut words, "'movestogo' number")?)
                }
                "depth" => limit.depth = Depth::new(parse_int(&mut words, "depth number")?),
                "nodes" => {
                    limit.nodes = Nodes::new(parse_int(&mut words, "node count")?)
                        .ok_or_else(|| "node count can't be zero".to_string())?
                }
                "mate" => {
                    let depth: usize = parse_int(&mut words, "mate move count")?;
                    limit.depth = Depth::new(depth * 2) // 'mate' is given in moves instead of plies
                }
                "movetime" => {
                    limit.fixed_time = Duration::from_millis(parse_int(
                        &mut words,
                        "time per move in milliseconds",
                    )?);
                    limit.fixed_time =
                        (limit.fixed_time - Duration::from_millis(2)).max(Duration::from_millis(1));
                }
                "infinite" => (), // "infinite" is the identity element of the bounded semilattice of `go` options
                "perft" => search_type = Perft,
                "splitperft" => search_type = SplitPerft,
                "bench" => search_type = Bench,
                _ => return Err(format!("Unrecognized 'go' option: '{next_word}'")),
            }
        }
        self.start_search(search_type, limit)
    }

    fn start_search(&mut self, search_type: SearchType, limit: SearchLimit) -> Res<()> {
        self.write_message(
            Debug,
            &format!("Starting {search_type} search with tc {}", limit.tc),
        );
        self.state.status = Run(Ongoing);
        // TODO: Do this asynchronously to be able to handle stop commands
        match search_type {
            Normal => self.state.engine.start_search(
                self.state.board,
                limit,
                self.state.board_hist.0.clone(),
            )?,
            Perft => {
                let msg = format!("{0}", perft(limit.depth, self.state.board));
                self.write_ugi(&msg);
            }
            SplitPerft => {
                let msg = format!("{0}", split_perft(limit.depth, self.state.board));
                self.write_ugi(&msg);
            }
            Bench => {
                let depth = if limit.depth == Depth::MAX {
                    self.state.engine.engine_info().default_bench_depth
                } else {
                    limit.depth
                };
                self.state
                    .engine
                    .start_bench(self.state.board, depth)
                    .expect("bench panic");
            }
        };
        Ok(())
    }

    fn handle_position(&mut self, mut words: SplitWhitespace) -> Res<()> {
        self.state.initial_pos = parse_ugi_position(&mut words, self.state.board.settings())?;
        self.clear_state();

        let Some(word) = words.next() else {
            return Ok(());
        };
        if word != "moves" {
            return Err(format!("Unrecognized word '{word}' after position command, expected either 'moves' or nothing"));
        }
        for mov in words {
            let mov = B::Move::from_compact_text(mov, &self.state.board)
                .map_err(|err| format!("Couldn't parse move: {err}"))?;
            self.make_move(mov)?;
        }
        self.state.last_played_color = self.state.board.active_player();
        Ok(())
    }

    fn handle_query(&mut self, mut words: SplitWhitespace) -> Res<()> {
        match words.next().ok_or("Missing argument to 'query'")? {
            "gameover" => self
                .output()
                .write_response(matches!(self.state.status, Run(Ongoing)).to_string()),
            "p1turn" => self
                .output()
                .write_response((self.state.board.active_player() == White).to_string()),
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
                self.output().write_response(response.to_string());
            }
            s => return Err(format!("unrecognized option {s}")),
        }
        Ok(())
    }

    fn handle_print(&mut self, mut words: SplitWhitespace) -> Res<()> {
        // This is definitely not the fastest way to print something, but performance isn't a huge concern here
        let mut output =
            output_builder_from_str(words.next().unwrap_or_default(), &self.output_factories)?
                .for_engine(&self.state)?;
        output.show(&self.state);
        Ok(())
    }

    fn handle_ui(&mut self, mut words: SplitWhitespace) -> Res<()> {
        self.output().additional_outputs.push(
            output_builder_from_str(words.next().unwrap_or_default(), &self.output_factories)?
                .for_engine(&self.state)?,
        );
        Ok(())
    }

    fn handle_remove_uis(&mut self) -> Res<()> {
        self.write_message(
            Debug,
            &format!("Removed all {} UIs", self.output().additional_outputs.len()),
        );
        self.output().additional_outputs.clear();
        Ok(())
    }

    // TODO: Move this function, and others throughout the project,
    // somewhere else so they don't depend on the type of `Board` to reduce code bloat.

    fn handle_log(&mut self, words: SplitWhitespace) -> Res<()> {
        self.output().additional_outputs.retain(|o| !o.is_logger());
        let next = words.clone().next().unwrap_or_default();
        if next != "off" && next != "none" {
            self.output()
                .additional_outputs
                .push(LoggerBuilder::from_words(words).for_engine(&self.state)?);
        }
        Ok(())
    }

    fn handle_engine(&mut self, mut words: SplitWhitespace) -> Res<()> {
        todo!("Currently, this is no longer implemented (and maybe not a good idea in general?)")
        // self.state.engine = create_engine_from_str(words.next().unwrap_or_default(), "")?;
        // Ok(())
    }

    fn write_options(&self) -> String {
        let mut opts = self
            .state
            .engine
            .get_options()
            .iter()
            .cloned()
            .collect_vec();
        opts.iter()
            .map(|opt| format!("option {opt}"))
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn make_move(&mut self, mov: B::Move) -> Res<()> {
        if !self.state.board.is_move_pseudolegal(mov) {
            return Err(format!("Illegal move {mov} (not pseudolegal)"));
        }
        if let Run(Over(result)) = &self.state.status {
            return Err(format!(
                "Cannot play move '{mov}' because the game is already over: {0} ({1}). The position is '{2}'",
                result.result, result.reason, self.state.board.as_fen()
            ));
        }
        self.state.board_hist.push(&self.state.board);
        self.state.mov_hist.push(mov);
        self.state.board = self
            .state
            .board
            .make_move(mov)
            .ok_or_else(|| format!("Illegal move {mov} (pseudolegal)"))?;
        if self.state.debug_mode {
            if let Some(res) = self.state.board.match_result_slow() {
                return Err(format!("The game is over ({0}, reason: {1}) after move {mov}, which results in the following position: {2}", res.result, res.reason, self.state.board.as_fen()));
            }
        }
        Ok(())
    }
}
