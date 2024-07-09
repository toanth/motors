use std::fmt::{Debug, Display, Formatter, Write};
use std::io::stdin;
use std::ops::{Deref, DerefMut};
use std::str::{FromStr, SplitWhitespace};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use colored::Colorize;

use gears::cli::{select_game, Game};
use gears::games::Color::White;
use gears::games::{Board, BoardHistory, Color, Move, OutputList, ZobristHistory};
use gears::general::common::Description::WithDescription;
use gears::general::common::{
    parse_bool_from_str, parse_duration_ms, parse_int, parse_int_from_str,
    to_name_and_optional_description, NamedEntity,
};
use gears::general::common::{IterIntersperse, Res};
use gears::general::perft::{perft, perft_for, split_perft};
use gears::output::logger::LoggerBuilder;
use gears::output::Message::*;
use gears::output::{Message, OutputBox, OutputBuilder};
use gears::search::{Depth, NodesLimit, SearchInfo, SearchLimit, SearchResult, TimeControl};
use gears::ugi::EngineOptionName::{MoveOverhead, Threads};
use gears::ugi::EngineOptionType::Spin;
use gears::ugi::{
    parse_ugi_position, EngineOption, EngineOptionName, EngineOptionType, UgiCheck, UgiSpin,
};
use gears::MatchStatus::*;
use gears::Quitting::{QuitMatch, QuitProgram};
use gears::{output_builder_from_str, AbstractRun, GameResult, GameState, MatchStatus, Quitting};

use crate::cli::EngineOpts;
use crate::search::multithreading::{EngineWrapper, SearchSender};
use crate::search::{
    run_bench_with_depth_and_nodes, BenchLimit, BenchResult, EvalList, SearcherList,
};
use crate::ugi_engine::ProgramStatus::{Quit, Run};
use crate::ugi_engine::SearchType::*;
use crate::{
    create_engine_bench_from_str, create_engine_from_str, create_eval_from_str, create_match,
};

const DEFAULT_MOVE_OVERHEAD_MS: u64 = 50;

// TODO: Ensure this conforms to <https://expositor.dev/uci/doc/uci-draft-1.pdf>

enum SearchType {
    Normal,
    Ponder,
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
                Ponder => "ponder",
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
    Quit(Quitting),
}

#[derive(Debug, Clone)]
struct BoardGameState<B: Board> {
    board: B,
    debug_mode: bool,
    status: ProgramStatus,
    mov_hist: Vec<B::Move>,
    board_hist: ZobristHistory<B>,
    initial_pos: B,
    last_played_color: Color,
    ponder_limit: Option<SearchLimit>,
}

impl<B: Board> BoardGameState<B> {
    fn make_move(&mut self, mov: B::Move) -> Res<()> {
        if !self.board.is_move_pseudolegal(mov) {
            return Err(format!("Illegal move {mov} (not pseudolegal)"));
        }
        if let Run(Over(result)) = &self.status {
            return Err(format!(
                "Cannot play move '{mov}' because the game is already over: {0} ({1}). The position is '{2}'",
                result.result, result.reason, self.board.as_fen()
            ));
        }
        self.board_hist.push(&self.board);
        self.mov_hist.push(mov);
        self.board = self
            .board
            .make_move(mov)
            .ok_or_else(|| format!("Illegal move {mov} (pseudolegal)"))?;
        if self.debug_mode {
            if let Some(res) = self.board.match_result_slow(&self.board_hist) {
                return Err(format!("The game is over ({0}, reason: {1}) after move {mov}, which results in the following position: {2}", res.result, res.reason, self.board.as_fen()));
            }
        }
        Ok(())
    }

    fn clear_state(&mut self) {
        self.board = self.initial_pos;
        self.mov_hist.clear();
        self.board_hist.clear();
        self.status = Run(NotStarted);
    }

    fn handle_position(&mut self, words: &mut SplitWhitespace) -> Res<()> {
        self.initial_pos = parse_ugi_position(words, &self.board)?;
        self.clear_state();

        let Some(word) = words.next() else {
            return Ok(());
        };
        if word != "moves" && word != "m" {
            return Err(format!("Unrecognized word '{word}' after position command, expected either 'moves', 'm', or nothing"));
        }
        for mov in words {
            let mov = B::Move::from_compact_text(mov, &self.board)
                .map_err(|err| format!("Couldn't parse move: {err}"))?;
            self.make_move(mov)?;
        }
        self.last_played_color = self.board.active_player();
        Ok(())
    }
}

#[derive(Debug)]
struct EngineGameState<B: Board> {
    board_state: BoardGameState<B>,
    engine: EngineWrapper<B>,
    /// This doesn't have to be the UGI engine name. It often isn't, especially when two engines with
    /// the same name play against each other, such as in a SPRT. It should be unique, however
    /// (the `monitors` client ensures that, but another GUI might not).
    display_name: String,
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
            output.display_message(typ, msg);
        }
    }

    pub fn show(&mut self, m: &dyn GameState<B>) -> bool {
        for output in self.additional_outputs.iter_mut() {
            output.show(m)
        }
        self.additional_outputs
            .iter()
            .any(|o| !o.is_logger() && o.prints_board())
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
        let best = search_result.chosen_move.to_compact_text();
        if let Some(ponder) = search_result.ponder_move() {
            let ponder = ponder.to_compact_text();
            self.write_ugi(&format!("bestmove {best} ponder {ponder}"));
        } else {
            self.write_ugi(&format!("bestmove {best}"));
        }
    }

    pub fn show_search_info(&mut self, info: SearchInfo<B>) {
        self.write_ugi(&info.to_string());
    }
}

/// Implements both UGI and UCI.
#[derive(Debug)]
pub struct EngineUGI<B: Board> {
    state: EngineGameState<B>,
    output: Arc<Mutex<UgiOutput<B>>>,
    output_factories: OutputList<B>,
    searcher_factories: SearcherList<B>,
    eval_factories: EvalList<B>,
    search_sender: SearchSender<B>,
    move_overhead: Duration,
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
        mut selected_output_builders: OutputList<B>,
        all_output_builders: OutputList<B>,
        all_searchers: SearcherList<B>,
        all_evals: EvalList<B>,
    ) -> Res<Self> {
        let output = Arc::new(Mutex::new(UgiOutput::default()));
        let sender = SearchSender::new(output.clone());
        let board = B::default();
        let engine =
            create_engine_from_str(&opts.engine, &all_searchers, &all_evals, sender.clone())?;
        let board_state = BoardGameState {
            board,
            debug_mode: opts.debug,
            status: Run(NotStarted),
            mov_hist: vec![],
            board_hist: ZobristHistory::default(),
            initial_pos: B::default(),
            last_played_color: Default::default(),
            ponder_limit: None,
        };
        let state = EngineGameState {
            board_state,
            engine,
            display_name: opts.engine,
        };
        let err_msg_builder = output_builder_from_str("error", &all_output_builders)?;
        selected_output_builders.push(err_msg_builder);
        for builder in selected_output_builders.iter_mut() {
            output
                .lock()
                .unwrap()
                .additional_outputs
                .push(builder.for_engine(&state)?);
        }
        Ok(Self {
            state,
            output,
            output_factories: all_output_builders,
            searcher_factories: all_searchers,
            eval_factories: all_evals,
            search_sender: sender,
            move_overhead: Duration::from_millis(DEFAULT_MOVE_OVERHEAD_MS),
            allow_ponder: false,
        })
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

            let res = self.parse_input(input.split_whitespace());
            match res {
                Err(err) => {
                    self.write_message(Error, err.as_str());
                    // explicitly check this here so that continuing on error doesn't prevent us from quitting.
                    if let Quit(quitting) = self.state.status {
                        return quitting;
                    }
                    if self.continue_on_error() {
                        self.write_message(Debug, "Continuing... ('debug' is 'on')");
                        continue;
                    }
                    return QuitProgram;
                }
                Ok(status) => {
                    if let Quit(quitting) = status {
                        return quitting;
                    }
                }
            }
        }
        QuitProgram
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
        let words = &mut words;
        let first_word = words.next().ok_or("Empty input")?;
        match first_word {
            // put time-critical commands at the top
            "go" | "g" => {
                self.handle_go(words)?;
            }
            "position" | "pos" | "p" => {
                self.state.handle_position(words)?;
            }
            "stop" => {
                self.state.engine.send_stop();
            }
            "ugi" => {
                let id_msg = self.id();
                self.write_ugi(id_msg.as_str());
                self.write_ugi(self.write_ugi_options().as_str());
                self.write_ugi("ugiok");
            }
            "uci" => {
                let id_msg = self.id();
                self.write_ugi(id_msg.as_str());
                self.write_ugi(self.write_ugi_options().as_str());
                self.write_ugi("uciok");
            }
            "ponderhit" => self.start_search(
                Normal,
                self.state.ponder_limit.ok_or_else(|| {
                    format!(
                        "The engine received a '{}' command but wasn't pondering",
                        first_word.bold()
                    )
                })?,
                self.state.board,
                vec![],
            )?,
            "isready" => {
                self.write_ugi("readyok");
            }
            "debug" | "d" => {
                self.handle_debug(words)?;
            }
            "setoption" => {
                self.handle_setoption(words)?;
            }
            "ucinewgame" | "uginewgame" | "clear" => {
                self.state.engine.send_forget()?;
                self.state.status = Run(NotStarted);
            }
            "register" => return Err("'register' isn't supported".to_string()),
            "flip" => {
                self.state.board = self.state.board.make_nullmove().ok_or(format!(
                    "Could not flip the side to move (board: '{}'",
                    self.state.board.as_fen().bold()
                ))?;
            }
            "quit" => {
                self.quit(QuitProgram)?;
            }
            "quit_match" | "end_game" | "qm" => {
                self.quit(QuitMatch)?;
            }
            "query" => {
                self.handle_query(words)?;
            }
            "option" => {
                self.write_ugi(&self.print_option(words)?);
            }
            "output" | "o" => {
                self.handle_output(words)?;
            }
            "print" | "show" | "s" | "display" => {
                self.handle_print(words)?;
            }
            "log" => {
                self.handle_log(words)?;
            }
            "engine" => {
                self.handle_engine(words)?;
            }
            "set-eval" => {
                self.handle_set_eval(words)?;
            }
            "play" => {
                self.handle_play(words)?;
            }
            "perft" => {
                self.handle_perft_or_bench(Perft, words)?;
            }
            "splitperft" | "sp" => {
                self.handle_perft_or_bench(SplitPerft, words)?;
            }
            "bench" => {
                self.handle_perft_or_bench(Bench, words)?;
            }
            "eval" | "e" => self.handle_eval(words)?,
            "help" => self.print_help(),
            x => {
                // The original UCI spec demands that unrecognized tokens should be ignored, whereas the
                // expositor UCI spec demands that an invalid token should cause the entire message to be ignored.
                self.write_message(
                    Warning,
                    &format!(
                        "Invalid token at start of UCI command '{0}', ignoring the entire command. \
                        If you are a human, consider typing {1} to see a list of recognized commands.", x.red(), "help".bold()
                    ),
                );
            }
        }
        if let Some(remaining) = words.next() {
            self.write_message(
                Warning,
                &format!(
                    "Ignoring trailing input starting with '{}'",
                    remaining.bold()
                ),
            );
        }
        Ok(self.state.status.clone())
    }

    fn quit(&mut self, typ: Quitting) -> Res<()> {
        // Do this before sending `quit`: If that fails, we can still recognize that we wanted to quit,
        // so that continuing on errors won't prevent us from quitting the program.
        self.state.status = Quit(typ);
        self.state.engine.send_quit()?;
        Ok(())
    }

    fn id(&self) -> String {
        let info = self.state.engine.engine_info();
        format!(
            "id name Motors -- Engine {0}\nid author ToTheAnd",
            info.long_name(),
        )
    }

    fn handle_setoption(&mut self, words: &mut SplitWhitespace) -> Res<()> {
        let mut name = words.next().unwrap_or_default().to_ascii_lowercase();
        if name != "name" {
            return Err(format!(
                "Invalid 'setoption' command: Expected 'name', got '{};",
                name.red()
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
        match name {
            EngineOptionName::Ponder => {
                self.allow_ponder = parse_bool_from_str(&value, "ponder")?;
            }
            MoveOverhead => {
                self.move_overhead =
                    parse_duration_ms(&mut value.split_whitespace(), "move overhead")?;
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

    fn handle_go(&mut self, words: &mut SplitWhitespace) -> Res<()> {
        let mut limit = SearchLimit::infinite();
        let is_white = self.state.board.active_player() == White;
        let mut search_type = Normal;
        let mut search_moves = vec![];
        let mut reading_moves = false;
        while let Some(next_word) = words.next() {
            match next_word {
                "searchmoves" => {
                    reading_moves = true;
                    continue;
                }
                "ponder" => search_type = Ponder, // setting different search types uses the last one specified
                "wtime" | "p1time" | "wt" | "p1t" => {
                    let time = parse_duration_ms(words, "wtime")?;
                    // always parse the duration, even if it isn't relevant
                    if is_white {
                        limit.tc.remaining = time;
                    }
                }
                "btime" | "p2time" | "bt" | "p2t" => {
                    let time = parse_duration_ms(words, "btime")?;
                    if !is_white {
                        limit.tc.remaining = time;
                    }
                }
                "winc" | "p1inc" | "wi" => {
                    let increment = parse_duration_ms(words, "winc")?;
                    if is_white {
                        limit.tc.increment = increment;
                    }
                }
                "binc" | "p2inc" | "bi" => {
                    let increment = parse_duration_ms(words, "binc")?;
                    if !is_white {
                        limit.tc.increment = increment;
                    }
                }
                "movestogo" => limit.tc.moves_to_go = Some(parse_int(words, "'movestogo' number")?),
                "depth" | "d" => limit.depth = Depth::new(parse_int(words, "depth number")?),
                "nodes" | "n" => {
                    limit.nodes = NodesLimit::new(parse_int(words, "node count")?)
                        .ok_or_else(|| "node count can't be zero".to_string())?
                }
                "mate" => {
                    let depth: usize = parse_int(words, "mate move count")?;
                    limit.mate = Depth::new(depth * 2) // 'mate' is given in moves instead of plies
                }
                "movetime" => {
                    limit.fixed_time = parse_duration_ms(words, "time per move in milliseconds")?;
                    limit.fixed_time = limit
                        .fixed_time
                        .saturating_sub(self.move_overhead)
                        .max(Duration::from_millis(1));
                }
                "infinite" | "inf" => (), // "infinite" is the identity element of the bounded semilattice of `go` options
                "perft" | "p" => search_type = Perft,
                "splitperft" | "sp" => search_type = SplitPerft,
                "bench" => search_type = Bench,
                _ => {
                    if reading_moves {
                        let mov = B::Move::from_compact_text(next_word, &self.state.board)
                            .map_err(|err| {
                                format!("{err}. '{}' is not a valid 'go' option.", next_word.bold())
                            })?;
                        search_moves.push(mov);
                        continue;
                    } else {
                        return Err(format!("Unrecognized 'go' option: '{next_word}'"));
                    }
                }
            }
            reading_moves = false;
        }
        limit.tc.remaining = limit
            .tc
            .remaining
            .saturating_sub(self.move_overhead)
            .max(Duration::from_millis(1));
        self.start_search(search_type, limit, self.state.board, search_moves)
    }

    fn start_search(
        &mut self,
        search_type: SearchType,
        mut limit: SearchLimit,
        pos: B,
        moves: Vec<B::Move>,
    ) -> Res<()> {
        self.write_message(
            Debug,
            &format!("Starting {search_type} search with tc {}", limit.tc),
        );
        self.state.status = Run(Ongoing);
        let default_depth = match search_type {
            Perft | SplitPerft => pos.default_perft_depth(),
            Bench => self.state.engine.engine_info().default_bench_depth(),
            _ => limit.depth,
        };
        if limit.depth == Depth::MAX {
            limit.depth = default_depth;
        }
        match search_type {
            // this keeps the current history even if we're searching a different position, but that's probably not a problem
            // and doing a normal search from a custom position isn't even implemented at the moment -- TODO: implement?
            Normal => {
                // It doesn't matter if we got a ponderhit or a miss, we simply abort the ponder search and start a new search.
                if self.state.ponder_limit.is_some() {
                    self.state.ponder_limit = None;
                    self.search_sender.abort_pondering();
                }
                self.state.engine.start_search(
                    pos,
                    limit,
                    self.state.board_hist.clone(),
                    false,
                    moves,
                )?
            }
            Ponder => {
                self.state.ponder_limit = Some(limit.clone());
                self.state.engine.start_search(
                    pos,
                    SearchLimit::infinite(), //always allocate infinite time for pondering
                    self.state.board_hist.clone(),
                    true,
                    moves,
                )?;
            }
            Perft => {
                let msg = format!("{0}", perft(limit.depth, pos));
                self.write_ugi(&msg);
            }
            SplitPerft => {
                let msg = format!("{0}", split_perft(limit.depth, pos));
                self.write_ugi(&msg);
            }
            Bench => {
                self.state
                    .engine
                    .start_bench(pos, BenchLimit::Depth(limit.depth))
                    .expect("bench panic");
            }
        };
        Ok(())
    }

    fn load_position_into_copy(&self, words: &mut SplitWhitespace) -> Res<B> {
        let mut board_state_clone = self.state.board_state.clone();
        board_state_clone.handle_position(words)?;
        Ok(board_state_clone.board)
    }

    fn handle_perft_or_bench(&mut self, typ: SearchType, words: &mut SplitWhitespace) -> Res<()> {
        let mut board = self.state.board;
        let mut limit = SearchLimit::infinite();
        let mut complete = false;
        while let Some(word) = words.next() {
            match word {
                "position" | "pos" | "p" => board = self.load_position_into_copy(words)?,
                "depth" | "d" => {
                    limit.depth = Depth::new(parse_int(words, "depth number")?);
                }
                "nodes" | "n" => {
                    limit.nodes =
                        NodesLimit::new(parse_int(words, "node count")?).unwrap_or(NodesLimit::MAX);
                }
                "complete" => complete = true,
                x => {
                    if let Ok(depth) = parse_int_from_str(x, "depth") {
                        limit.depth = Depth::new(depth);
                    } else {
                        return Err(format!(
                            "unrecognized bench/perft argument '{}', expected 'position', 'complete', 'nodes', 'depth' or the depth value",
                            x.red()
                        ));
                    }
                }
            }
        }
        if complete {
            let res = match typ {
                Perft => perft_for(limit.depth, B::bench_positions()).to_string(),
                Bench => {
                    let mut engine = create_engine_bench_from_str(
                        &self.state.engine.engine_info().short_name(),
                        &self.searcher_factories,
                        &self.eval_factories,
                    )?;
                    run_bench_with_depth_and_nodes(engine.as_mut(), limit.depth, limit.nodes).to_string()
                },
                _ => return Err(format!("Can only use the '{}' option with bench or perft, not splitperft or normal runs", "complete".bold()))
            };
            self.write_ugi(&res);
            Ok(())
        } else {
            self.start_search(typ, limit, board, vec![])
        }
    }

    fn handle_eval(&mut self, words: &mut SplitWhitespace) -> Res<()> {
        if words.clone().next().is_some() {
            let mut board_state_clone = self.state.board_state.clone();
            board_state_clone.handle_position(words)?;
            self.state.engine.static_eval(board_state_clone.board)
        } else {
            self.state.engine.static_eval(self.state.board)
        }
    }

    fn handle_query(&mut self, words: &mut SplitWhitespace) -> Res<()> {
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

    fn handle_print(&mut self, words: &mut SplitWhitespace) -> Res<()> {
        match words.next() {
            None => {
                if !self.output().show(&self.state) {
                    return self.handle_print(&mut "unicode".split_whitespace());
                }
            }
            Some(name) => {
                // This is definitely not the fastest way to print something, but performance isn't a huge concern here
                let mut output = output_builder_from_str(name, &self.output_factories)?
                    .for_engine(&self.state)?;
                output.show(&self.state);
            }
        }
        Ok(())
    }

    fn handle_output(&mut self, words: &mut SplitWhitespace) -> Res<()> {
        let next = words.next().unwrap_or_default();
        let output_ptr = self.output.clone();
        let mut output = output_ptr.lock().unwrap();
        if next.eq_ignore_ascii_case("remove") {
            let next = words.next().ok_or(
                "No output to remove specified. Use 'all' to remove all outputs".to_string(),
            )?;
            if next.eq_ignore_ascii_case("all") {
                output.additional_outputs.clear();
            } else {
                output
                    .additional_outputs
                    .retain(|o| !o.short_name().eq_ignore_ascii_case(next))
            }
        } else if next.eq_ignore_ascii_case("list") {
            for o in output.additional_outputs.iter() {
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
                        format!(
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

    fn handle_debug(&mut self, words: &mut SplitWhitespace) -> Res<()> {
        match words.next().unwrap_or("on") {
            "on" => {
                self.state.debug_mode = true;
                // make sure to print all the messages that can be sent (adding an existing output is a no-op)
                self.handle_output(&mut "error".split_whitespace())?;
                self.handle_output(&mut "debug".split_whitespace())?;
                self.handle_output(&mut "info".split_whitespace())?;
                self.write_message(Debug, "Debug mode enabled");
                // don't change the log stream if it's already set
                if !self
                    .output()
                    .additional_outputs
                    .iter()
                    .any(|o| o.is_logger())
                {
                    // In case of an error here, still keep the debug mode set.
                    self.handle_log(&mut "".split_whitespace())
                        .map_err(|err| format!("Couldn't set the debug log file: '{err}'"))?;
                    Ok(())
                } else {
                    Ok(())
                }
            }
            "off" => {
                self.state.debug_mode = false;
                _ = self.handle_output(&mut "remove debug".split_whitespace());
                _ = self.handle_output(&mut "remove info".split_whitespace());
                self.write_message(Debug, "Debug mode disabled");
                // don't remove the error output, as there is basically no reason to do so
                self.handle_log(&mut "none".split_whitespace())
            }
            x => Err(format!("Invalid debug option '{x}'")),
        }
    }

    // TODO: Move this function, and others throughout the project,
    // somewhere else so they don't depend on the type of `Board` to reduce code bloat.

    fn handle_log(&mut self, words: &mut SplitWhitespace) -> Res<()> {
        self.output().additional_outputs.retain(|o| !o.is_logger());
        let next = words.clone().next().unwrap_or_default();
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
        let str = format!("{motors}: A work-in-progress collection of engines for various games, \
        currently playing {game_name}, using the engine {engine_name}.\
        \nThe behavior of normal UCI / UGI commands can be found here: https://backscattering.de/chess/uci/ \
        \nSeveral additional commands are supported:\
        \n {debug}: Turns debug logging on or off. `debug <logfile>` sets logging as if by calling `log <logfile>`, \
        enables additional debug output, and also enables error recovery mode: \
        For incorrect input, the program will now print an error message and continue instead of terminating.\
        \n {output}: Adds additional outputs. An 'output' prints information about the current state of the game and can handle messages.\
        Type `output` to see a list of outputs and a short explanation of what they do.\
        \n {print}: `print <output>` prints the game using the specified output, or all of the current outputs if none is given, \
        or `unicode` if no outputs are being used.\
        \n {log}: `log <logfile> starts logging to <logfile>; use `none` or `off` to turn off logging and `stdout` or `stderr` to print to those streams.\
        \n {engine}: Loads another engine for the same game. Use 'play' to change the game.\
        \n {perft}: Equivalent to `go perft`, but allows setting the position as last argument, e.g. `perft depth 3 position startpos` \
        or simply `perft` to use the current position and game-specific default depth.\
        \n {bench}: See `perft`, but replace 'perft' with 'bench'. The default depth is engine-specific.\
        \n {eval}: Prints the static eval of the current position, without doing any actual searching.\
        \n {set_eval}: Loads another evaluation function for the same engine.\
        \n {option}: Prints the current value of the specified UGI option, or of all UGI options if no name is specified.\
        \n {play}: Pause the current match and start a new match of the given game, e.g. 'play chess'. Once that receives \
        '{quit_match}', exit the match and resume the current match.\
        \n {help}: Prints this help message. \
        \nThis command line interface is mainly intended for internal use, if you want to play against this engine or use it for analysis,\
        you should probably use a GUI, such as the WIP {monitors} project.",
            game_name = B::game_name().bold(),
            motors = "motors".bold(),
            debug = "debug | d".bold(),
            output = "output | o".bold(),
            print = "print | show | s | display".bold(),
            log = "log".bold(),
            engine = "engine".bold(),
            perft = "perft".bold(),
            bench = "bench".bold(),
            eval = "eval | e".bold(),
            set_eval = "set-eval".bold(),
            option = "option".bold(),
            play = "play".bold(),
            quit_match = "quit_match".bold(),
            help = "help".bold(),
            monitors = "monitors".italic(),
        );
        println!("{str}");
    }

    fn handle_engine(&mut self, words: &mut SplitWhitespace) -> Res<()> {
        let Some(name) = words.next() else {
            let info = self.state.engine.engine_info();
            self.write_ugi(&format!(
                "\n{alias}: {0}\n{engine}: {1}\n{description}: {2}",
                info.short_name(),
                info.long_name(),
                info.description().unwrap_or_default(),
                alias = "Alias".bold(),
                engine = "Engine".bold(),
                description = "Description".bold(),
            ));
            return Ok(());
        };
        // catch invalid names before committing to shutting down the current engine
        let engine = create_engine_from_str(
            name,
            &self.searcher_factories,
            &self.eval_factories,
            self.search_sender.clone(),
        )?;
        self.state.engine.send_quit()?;
        self.state.engine = engine;
        Ok(())
    }

    fn handle_set_eval(&mut self, words: &mut SplitWhitespace) -> Res<()> {
        let Some(name) = words.next() else {
            let name = self
                .state
                .engine
                .engine_info()
                .eval()
                .clone()
                .map(|e| e.short_name())
                .unwrap_or("<eval unused>".to_string());
            self.write_ugi(&format!("Current eval: {name}"));
            return Ok(());
        };
        let eval = create_eval_from_str(name, &self.eval_factories)?.build();
        self.state.engine.set_eval(eval)
    }

    fn handle_play(&mut self, words: &mut SplitWhitespace) -> Res<()> {
        let default = Game::default().to_string();
        let game_name = words.next().unwrap_or(&default);
        let game = select_game(game_name)?;
        let opts = EngineOpts::for_game(game, self.state.debug_mode);
        let mut nested_match = create_match(opts)?;
        if nested_match.run() == QuitProgram {
            self.quit(QuitProgram)?;
        }
        Ok(())
    }

    fn print_option(&self, words: &mut SplitWhitespace) -> Res<String> {
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
                        writeln!(
                            res,
                            "{name}: {value}",
                            name = o.name,
                            value = o.value.value_to_str()
                        )
                        .unwrap();
                    }
                    res
                }
                x => {
                    match options
                        .iter()
                        .find(|o| o.name.to_string().eq_ignore_ascii_case(x))
                    {
                        Some(opt) => {
                            format!("{0}: {1}", opt.name, opt.value.value_to_str())
                        }
                        None => {
                            return Err(format!(
                                "No option named '{0}' exists. Type '{1}' for a list of options.",
                                x.red(),
                                "ugi".bold()
                            ))
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
                value: EngineOptionType::Check(UgiCheck {
                    val: self.allow_ponder,
                    default: Some(false),
                }),
            },
        ];
        res.extend(self.state.engine.get_options().iter().cloned());
        res
    }
}
