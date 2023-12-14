use std::cell::RefCell;
use std::fmt::Debug;
use std::io::{stdin, Write};
use std::mem::discriminant;
use std::rc::Rc;
use std::str::{FromStr, SplitWhitespace};
use std::thread::spawn;
use std::time::Duration;

use colored::Colorize;
use crossbeam_channel::{select, unbounded};
use itertools::Itertools;

use crate::games::Color::White;
use crate::games::{Board, BoardHistory, Color, Move, ZobristRepetition3Fold};
use crate::general::common::{parse_int, Res};
use crate::play::run_match::play;
use crate::play::ugi::SearchType::{Bench, Normal, Perft, SplitPerft};
use crate::play::MatchStatus::*;
use crate::play::{
    default_engine, set_engine_from_str, set_graphics_from_str, set_match_from_str,
    set_position_from_str, AbstractMatchManager, AdjudicationReason, AnyEngine, AnyMatch,
    CreatableMatchManager, GameOverReason, GameResult, MatchManager, MatchResult, MatchStatus,
};
use crate::search::multithreading::{EngineSends, Receiver, Sender};
use crate::search::perft::{perft, split_perft};
use crate::search::EngineOptionName::{Hash, Threads};
use crate::search::{
    BenchResult, EngineOptionName, EngineOptionType, EngineUciOptionType, SearchInfo, SearchLimit,
    SearchResult, Searcher,
};
use crate::ui::logger::{LogStream, Logger};
use crate::ui::no_graphic::NoGraphics;
use crate::ui::Message::*;
use crate::ui::{to_graphics_handle, Graphics, GraphicsHandle, Message, UIHandle};

// use itertools::Itertools;

fn write_ugi<B: Board>(logger: &mut Logger<B>, message: &str) {
    // UGI is always done through stdin and stdout, no matter what the UI is.
    println!("{message}");
    logger.stream.write("<", message);
}

fn ugi_input_thread(sender: Sender<Res<String>>) {
    loop {
        let mut input = String::default();
        match stdin().read_line(&mut input) {
            Ok(_) => {
                if sender.send(Ok(input)).is_err() {
                    break;
                }
            }
            Err(e) => {
                _ = sender.send(Err(format!("Failed to read input: {0}", e.to_string())));
                break;
            }
        }
    }
}

pub trait AbstractUGI {
    fn ugi_loop(&mut self) -> MatchResult {
        self.write_message(Debug, "Starting UGI loop");
        let (sender, receiver) = unbounded();
        spawn(move || {
            ugi_input_thread(sender);
        });
        loop {
            let res = self.handle_ugi(receiver.clone());
            if res.is_err() {
                self.write_message(Error, res.err().unwrap().as_str());
                if self.continue_on_error() {
                    self.write_message(Debug, "Continuing... ('debug' is 'on')");
                    continue;
                }
                return MatchResult {
                    result: GameResult::P2Win,
                    reason: GameOverReason::Adjudication(AdjudicationReason::EngineError),
                };
            }
            if let Over(res) = res.unwrap() {
                return res;
            }
        }
    }

    fn continue_on_error(&self) -> bool;

    fn handle_ugi(&mut self, receiver: Receiver<Res<String>>) -> Result<MatchStatus, String>;

    // TODO: This should be a method of GameManager instead
    fn write_message(&mut self, typ: Message, message: &str);

    fn write_ugi(&self, message: &str);

    fn write_response(&mut self, response: String) {
        self.write_ugi(&format!("response {response}"))
    }
}

// impl<B: Board> InfoCallback<B> for UgiInfoCallback<B> {
//     fn print_info(&self, info: SearchInfo<B>) {
//         let score_str = if let Some(moves_until_over) = info.score.moves_until_game_won() {
//             format!("mate {moves_until_over}")
//         } else {
//             format!("cp {0}", info.score.0) // TODO: WDL normalization
//         };
//
//         let info_str = format!(
//         "info depth {depth}{seldepth} score {score_str} time {time} nodes {nodes} nps {nps} pv {pv}{hashfull}{string}",
//         depth = info.depth, time = info.time.as_millis(), nodes = info.nodes,
//         seldepth = info.seldepth.map(|d| format!(" seldepth {d}")).unwrap_or_default(),
//         nps=info.nps(),
//         pv = info.pv.iter().map(|mv| mv.to_compact_text()).collect::<Vec<_>>().join(" "),
//         hashfull = info.hashfull.map(|f| format!(" hashfull {f}")).unwrap_or_default(),
//         string = info.additional.map(|s| format!(" string {s}")).unwrap_or_default()
//     );
//         println!("{info_str}");
//         self.logger
//             .borrow_mut()
//             .display_message_simple(Info, &info_str);
//     }
// }

enum SearchType {
    Normal,
    Perft,
    SplitPerft,
    Bench,
}

// Implement both UGI and UCI
#[derive(Debug)]
pub struct UGI<B: Board> {
    engine: AnyEngine<B>,
    board: B,
    debug_mode: bool,
    status: MatchStatus,
    graphics: GraphicsHandle<B>,
    mov_hist: Vec<B::Move>,
    board_hist: ZobristRepetition3Fold,
    initial_pos: B,
    next_match: Option<AnyMatch>,
    logger: Rc<RefCell<Logger<B>>>,
}

impl<B: Board> AbstractMatchManager for UGI<B> {
    fn run(&mut self) -> MatchResult {
        self.ugi_loop()
    }

    fn next_match(&mut self) -> Option<AnyMatch> {
        self.next_match.take()
    }

    fn set_next_match(&mut self, next: Option<AnyMatch>) {
        self.next_match = next;
    }

    fn active_player(&self) -> Option<Color> {
        match self.status {
            Ongoing => Some(self.board.active_player()),
            _ => None,
        }
    }

    fn abort(&mut self) -> Res<MatchStatus> {
        self.engine.send_stop()?;
        self.status = MatchStatus::aborted();
        Ok(self.status)
    }

    fn match_status(&self) -> MatchStatus {
        self.status
    }

    fn game_name(&self) -> String {
        B::game_name()
    }

    fn debug_mode(&self) -> bool {
        self.debug_mode
    }
}

impl<B: Board> MatchManager<B> for UGI<B> {
    fn board(&self) -> B {
        self.board
    }

    fn initial_pos(&self) -> B {
        self.initial_pos
    }

    fn move_history(&self) -> &[B::Move] {
        self.mov_hist.as_slice()
    }

    fn graphics(&self) -> GraphicsHandle<B> {
        self.graphics.clone()
    }

    fn set_graphics(&mut self, graphics: GraphicsHandle<B>) {
        self.graphics = graphics;
        self.graphics.borrow_mut().show(self);
    }

    // /// all of this will be refactored soon
    // fn searcher(&self, _idx: usize) -> &dyn Searcher<B> {
    //     self.engine.as_ref()
    // }

    fn set_engine(&mut self, _: usize, engine: AnyEngine<B>) {
        self.engine = engine;
    }

    fn set_board(&mut self, board: B) {
        self.board = board;
    }
}

impl<B: Board> AbstractUGI for UGI<B> {
    fn handle_ugi(&mut self, stdin_receiver: Receiver<Res<String>>) -> Result<MatchStatus, String> {
        return select! {
            recv(stdin_receiver) -> input =>
                self.parse_input(&input.map_err(|err| err.to_string())??),
            recv(self.engine.receiver()) -> msg => self.handle_engine_response(msg.map_err(|err| err.to_string())?),
        };
    }

    fn write_ugi(&self, message: &str) {
        write_ugi(&mut self.logger.borrow_mut(), message);
    }

    fn write_message(&mut self, typ: Message, msg: &str) {
        self.graphics.borrow_mut().display_message(self, typ, msg);
        self.logger.borrow_mut().display_message(self, typ, msg);
    }

    fn continue_on_error(&self) -> bool {
        self.debug_mode
    }
}

impl<B: Board> CreatableMatchManager for UGI<B> {
    type ForGame<C: Board> = UGI<C>;

    fn with_engine_and_ui<C: Board>(engine: AnyEngine<C>, ui: UIHandle<C>) -> Self::ForGame<C> {
        <Self::ForGame<C>>::new(engine, ui)
    }
}

impl<B: Board> Default for UGI<B> {
    fn default() -> Self {
        Self::new(default_engine(), to_graphics_handle(NoGraphics::default()))
    }
}

impl<B: Board> UGI<B> {
    pub fn new(engine: AnyEngine<B>, graphics: GraphicsHandle<B>) -> Self {
        let board = B::default();
        Self {
            engine,
            board,
            mov_hist: vec![],
            board_hist: ZobristRepetition3Fold::default(),
            debug_mode: false,
            status: NotStarted,
            graphics,
            next_match: None,
            initial_pos: B::default(),
            logger: Rc::new(RefCell::new(Logger::new(LogStream::None))),
        }
    }

    fn handle_engine_response(&mut self, response: EngineSends<B>) -> Res<MatchStatus> {
        match response {
            // EngineSends::Nodes(n) => self.
            EngineSends::BenchRes(res) => self.show_bench(res),
            EngineSends::SearchRes(res) => self.show_search_res(res),
            EngineSends::EngineInformation(info) => { /*not handled here*/ }
            EngineSends::Info(info) => self.show_search_info(info),
            EngineSends::Message(msg) => self.write_message(Info, &msg),
            EngineSends::Error(msg) => {
                self.write_message(Error, &msg);
                return Err(msg);
            }
            EngineSends::EngineCopy(engine) => self.engine.receive_engine(engine),
        }
        return Ok(self.status);
    }

    fn parse_input(&mut self, mut input: &str) -> Result<MatchStatus, String> {
        input = input.trim();
        self.logger.borrow_mut().stream.write(">", input);
        let mut words = input.split_whitespace();
        let first_word = words.next().ok_or_else(|| "Empty input")?;
        match first_word {
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
                match words.next().unwrap_or_default() {
                    "on" => {
                        self.debug_mode = true;
                        // don't change the log stream if it's already set
                        if !self.logger.borrow().is_active() {
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
                        self.debug_mode = false;
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
                self.engine.send_forget()?;
                self.status = NotStarted;
            } // just ignore it
            "position" => {
                self.handle_position(words)?;
            }
            "go" => {
                self.handle_go(words)?;
            }
            "stop" => {
                self.engine.send_stop()?;
            }
            "ponderhit" => {} // ignore pondering
            "quit" => {
                self.quit()?;
            }
            "query" => {
                self.handle_query(words)?;
            }
            "ui" => {
                self.handle_ui(words)?;
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
            "game" => {
                self.handle_game(words)?;
            }
            "play" => {
                play(); // play a game locally without using UGI
            }
            x => {
                // An invalid token at the start of the input should be ignored according to the UCI spec.
                // The same is true recursively for the remaining input.
                let remaining = words.remainder().unwrap_or_default().trim();
                self.graphics.borrow_mut().display_message(
                    self,
                    Warning,
                    &format!("Ignoring invalid token at start of UCI command '{x}'"),
                );
                self.write_message(
                    Debug,
                    &format!("Invalid input '{x}' followed by '{remaining}'"),
                );
                if remaining.is_empty() {
                    return Ok(self.status);
                }
                self.parse_input(remaining)?;
            }
        }
        Ok(self.status)
    }

    fn quit(&mut self) -> Result<MatchStatus, String> {
        self.next_match = None;
        self.abort()
    }

    fn id(&self) -> String {
        let info = self.engine.engine_info();
        format!(
            "id name Motors - {0} {1}\nid author ToTheAnd",
            info.name, info.version
        )
    }

    fn handle_setoption(&mut self, mut words: SplitWhitespace) -> Result<MatchStatus, String> {
        let mut name = words.next().unwrap_or_default().to_ascii_lowercase();
        if name != "name" {
            return Err(format!(
                "Invalid option command: Expected 'name', got '{name};"
            ));
        }
        name = String::default();
        loop {
            let next_word = words.next().unwrap_or_default();
            if next_word.to_ascii_lowercase() == "value" || next_word == "" {
                break;
            }
            name = name + " " + next_word;
        }
        let mut value = words.next().unwrap_or_default().to_string();
        loop {
            let next_word = words.next().unwrap_or_default();
            if next_word == "" {
                break;
            }
            value = value + " " + next_word;
        }
        let name = EngineOptionName::from_str(&name.trim()).unwrap();
        let value = value.trim().to_string();
        self.engine
            .set_option(name.clone(), value.clone())
            .or_else(|err| {
                if name == Threads && value == "1" {
                    Ok(())
                } else {
                    Err(err)
                }
            })?;
        Ok(self.status)
    }

    fn handle_go(&mut self, mut words: SplitWhitespace) -> Result<MatchStatus, String> {
        let mut limit = SearchLimit::infinite();
        let is_white = self.board.active_player() == White;
        let mut search_type = Normal;
        while let Some(next_word) = words.next() {
            match next_word {
                "searchmoves" => {
                    return Err("The 'go searchmoves' option is not implemented".to_string())
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
                "movestogo" => limit.tc.moves_to_go = parse_int(&mut words, "'movestogo' number")?,
                "depth" => limit.depth = parse_int(&mut words, "depth number")?,
                "nodes" => limit.nodes = parse_int(&mut words, "node count")?,
                "mate" => {
                    limit.depth = parse_int(&mut words, "mate move count")?;
                    limit.depth *= 2 // 'mate' is given in moves instead of plies
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

    fn start_search(
        &mut self,
        search_type: SearchType,
        limit: SearchLimit,
    ) -> Result<MatchStatus, String> {
        self.status = Ongoing;
        // TODO: Do this asynchronously to be able to handle stop commands
        match search_type {
            Normal => self
                .engine
                .start_search(self.board, limit, self.board_hist.0.clone())?,
            Perft => {
                let msg = format!("{0}", perft(limit.depth, self.board));
                self.write_ugi(&msg);
            }
            SplitPerft => {
                let msg = format!("{0}", split_perft(limit.depth, self.board));
                self.write_ugi(&msg);
            }
            Bench => {
                let depth = if limit.depth == usize::MAX {
                    self.engine.engine_info().default_bench_depth
                } else {
                    limit.depth
                };
                self.engine
                    .start_bench(self.board, depth)
                    .expect("bench panic");
            }
        };
        Ok(Ongoing)
    }

    fn handle_position(&mut self, mut words: SplitWhitespace) -> Result<MatchStatus, String> {
        let input = words.remainder().unwrap_or_default().trim();
        let position_word = words
            .next()
            .ok_or_else(|| "Missing position after 'position' command".to_string())?;
        match position_word {
            "fen" => {
                let mut s = words.remainder().unwrap_or_default();
                self.board = B::read_fen_and_advance_input(&mut s)?;
                words = s.split_whitespace();
            }
            "startpos" => self.board = B::startpos(self.board.settings()),
            name => {
                set_position_from_str(self, name)?;
            }
        };
        self.status = NotStarted;
        self.initial_pos = self.board;
        self.mov_hist.clear();
        if let Some(word) = words.next() {
            if word != "moves" {
                return Err(format!("Unrecognized word '{word}' after position command, expected either 'moves' or nothing"));
            }
            for mov in words {
                let mov = Move::from_compact_text(mov, &self.board)
                    .map_err(|err| format!("Couldn't parse move: {err}"))?;
                if let Over(result) = self.make_move(mov)? {
                    return Err(format!(
                        "Game is already over after move '{mov}': {0}, {1}. The position was '{input}'",
                        result.result, result.reason,
                    ));
                }
            }
        }
        // TODO: This isn't really necessary, but maybe still a good idea?
        // if self.board.is_game_lost() || self.board.is_draw() {
        //     let result = if self.board.is_game_lost() {
        //         if self.board.active_player() == White {
        //             GameResult::P2Win
        //         } else {
        //             GameResult::P1Win
        //         }
        //     } else {
        //         GameResult::Draw
        //     };
        //     self.status = Over(MatchResult {
        //         result,
        //         reason: GameOverReason::Normal,
        //     });
        // }
        // self.graphics.borrow_mut().show(self);
        Ok(self.status)
    }

    fn handle_query(&mut self, mut words: SplitWhitespace) -> Result<MatchStatus, String> {
        match words.next().ok_or_else(|| "Missing argument to 'query'")? {
            "gameover" => self
                .write_response((discriminant(&self.status) == discriminant(&Ongoing)).to_string()),
            "p1turn" => self.write_response((self.board.active_player() == White).to_string()),
            "result" => {
                let response = match self.status {
                    Over(res) => match res.result {
                        GameResult::P1Win => "p1win",
                        GameResult::P2Win => "p2win",
                        GameResult::Draw => "draw",
                        GameResult::Aborted => "aborted",
                    },
                    _ => "none",
                };
                self.write_response(response.to_string());
            }
            s => return Err(format!("unrecognized option {s}")),
        }
        Ok(self.status)
    }

    fn handle_print(&mut self, words: SplitWhitespace) -> Result<MatchStatus, String> {
        let old_graphics = self.graphics.clone();
        let res = self.handle_ui(words);
        self.graphics = old_graphics;
        res
    }

    fn handle_ui(&mut self, mut words: SplitWhitespace) -> Result<MatchStatus, String> {
        set_graphics_from_str(self, words.next().unwrap_or_default())
    }

    // TODO: Move this function, and others throughout the project,
    // to the base trait so they don't depend on the type of `Board` to reduce code bloat.
    fn handle_log(&mut self, words: SplitWhitespace) -> Result<MatchStatus, String> {
        let remaining = words.remainder(); // Support whitespaces in file name (but not at the beginning)
        self.logger = Rc::new(RefCell::new(Logger::from_str(
            remaining.unwrap_or_default(),
        )?));
        Ok(self.status)
    }

    fn handle_engine(&mut self, mut words: SplitWhitespace) -> Result<MatchStatus, String> {
        set_engine_from_str(self, words.next().unwrap_or_default())
    }

    fn handle_game(&mut self, mut words: SplitWhitespace) -> Result<MatchStatus, String> {
        set_match_from_str(self, words.next().unwrap_or_default())
    }

    fn write_options(&self) -> String {
        let opt_to_string = |opt: &EngineOptionType| -> String {
            let default = opt
                .default
                .as_ref()
                .map(|x| format!(" default {x}"))
                .unwrap_or_default();
            let min = opt
                .min
                .as_ref()
                .map(|x| format!(" min {x}"))
                .unwrap_or_default();
            let max = opt
                .max
                .as_ref()
                .map(|x| format!(" max {x}"))
                .unwrap_or_default();
            let vars = opt
                .vars
                .iter()
                .map(|x| format!(" var {x}"))
                .collect::<Vec<String>>()
                .join("");
            format!(
                "option name {name} type {typ}{default}{min}{max}{vars}",
                name = opt.name,
                typ = opt.typ.to_str()
            )
        };
        let mut opts = self.engine.get_options().iter().cloned().collect_vec();
        if opts.iter().find(|opt| opt.name == Hash).is_none() {
            opts.push(EngineOptionType {
                name: Hash,
                typ: EngineUciOptionType::Spin,
                default: Some("1".to_string()),
                min: Some("0".to_string()),
                max: Some("1000".to_string()),
                vars: vec![],
            })
        }
        if opts.iter().find(|opt| opt.name == Threads).is_none() {
            opts.push(EngineOptionType {
                name: Threads,
                typ: EngineUciOptionType::Spin,
                default: Some("1".to_string()),
                min: Some("1".to_string()),
                max: Some("1".to_string()),
                vars: vec![],
            })
        }
        opts.iter()
            .map(|opt| opt_to_string(opt))
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn make_move(&mut self, mov: B::Move) -> Result<MatchStatus, String> {
        if !self.board.is_move_pseudolegal(mov) {
            return Err(format!("Illegal move {mov} (not pseudolegal)"));
        }
        self.board_hist.push(&self.board);
        self.mov_hist.push(mov);
        self.board = self
            .board
            .make_move(mov)
            .ok_or_else(|| format!("Illegal move {mov} (pseudolegal)"))?;
        // if self.board.is_game_lost() || self.board.is_draw() {
        //     let result = if self.board.is_game_lost() {
        //         if self.board.active_player() == White {
        //             GameResult::P2Win
        //         } else {
        //             GameResult::P1Win
        //         }
        //     } else {
        //         GameResult::Draw
        //     };
        //     self.status = Over(MatchResult {
        //         result,
        //         reason: GameOverReason::Normal,
        //     });
        // }
        Ok(self.status)
    }

    fn show_bench(&mut self, bench_result: BenchResult) {
        self.write_ugi(&format!(
            "depth {0}, time {2}ms, {1} nodes, {3} nps",
            bench_result.depth,
            bench_result.nodes,
            bench_result.time.as_millis(),
            ((bench_result.nodes as f64 / bench_result.time.as_millis() as f64 * 1000.0).round())
                .to_string()
                .red()
        ));
    }

    fn show_search_res(&mut self, search_result: SearchResult<B>) {
        self.write_ugi(&format!(
            "bestmove {best}",
            best = search_result.chosen_move.to_compact_text()
        ));
    }

    fn show_search_info(&mut self, info: SearchInfo<B>) {
        let score_str = if let Some(moves_until_over) = info.score.moves_until_game_won() {
            format!("mate {moves_until_over}")
        } else {
            format!("cp {0}", info.score.0) // TODO: WDL normalization
        };

        let info_str = format!(
            "info depth {depth}{seldepth} score {score_str} time {time} nodes {nodes} nps {nps} pv {pv}{hashfull}{string}",
            depth = info.depth, time = info.time.as_millis(), nodes = info.nodes,
            seldepth = info.seldepth.map(|d| format!(" seldepth {d}")).unwrap_or_default(),
            nps = info.nps(),
            pv = info.pv.iter().map(|mv| mv.to_compact_text()).collect::<Vec<_>>().join(" "),
            hashfull = info.hashfull.map(|f| format!(" hashfull {f}")).unwrap_or_default(),
            string = info.additional.map(|s| format!(" string {s}")).unwrap_or_default()
        );
        self.write_ugi(&info_str);
    }
}
