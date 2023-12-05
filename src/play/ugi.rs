use std::io::stdin;
use std::mem::discriminant;
use std::ops::{Deref, DerefMut};
use std::str::{FromStr, SplitWhitespace};
use std::time::Duration;

use colored::Colorize;
use itertools::Itertools;

use crate::games::Color::White;
use crate::games::{Board, BoardHistory, Color, Move, ZobristRepetition3Fold};
use crate::general::common::parse_int;
use crate::play::run_match::play;
use crate::play::ugi::SearchType::{Bench, Normal, Perft, SplitPerft};
use crate::play::MatchStatus::*;
use crate::play::{
    default_engine, set_engine_from_str, set_graphics_from_str, set_match_from_str,
    set_position_from_str, AbstractMatchManager, AdjudicationReason, AnyEngine, AnyMatch,
    CreatableMatchManager, GameOverReason, GameResult, MatchManager, MatchResult, MatchStatus,
};
use crate::search::perft::{perft, split_perft};
use crate::search::EngineOptionName::{Hash, Threads};
use crate::search::{
    run_bench_with_depth, Engine, EngineOptionName, EngineOptionType, EngineUciOptionType,
    InfoCallback, SearchInfo, SearchLimit, SearchResult, Searcher,
};
use crate::ui::no_graphic::NoGraphics;
use crate::ui::Message::Warning;
use crate::ui::{to_graphics_handle, GraphicsHandle, UIHandle};

// use itertools::Itertools;

fn format_search_result<B: Board>(res: SearchResult<B>) -> String {
    format!("bestmove {best}", best = res.chosen_move.to_compact_text())
}

pub trait AbstractUGI {
    fn ugi_loop(&mut self) -> MatchResult {
        loop {
            let res = self.read_input();
            if res.is_err() {
                eprintln!("An error occurred: {0}", res.err().unwrap());
                if self.continue_on_error() {
                    eprintln!("Continuing...");
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

    fn parse_input(&mut self, input: &str) -> Result<MatchStatus, String>;

    fn read_input(&mut self) -> Result<MatchStatus, String> {
        let mut input = String::default();
        if let Err(e) = stdin().read_line(&mut input) {
            return Err(format!("Failed to read input: {0}", e.to_string()));
        }
        self.parse_input(input.as_str())
    }

    fn write(&self, message: &str) {
        println!("{message}")
    }

    fn write_info(&self, info: &str) {
        println!("info {info}")
    }

    fn write_response(&self, response: String) {
        println!("response {response}")
    }
}

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

    fn abort(&mut self) -> Result<MatchStatus, String> {
        self.engine.stop();
        Ok(MatchStatus::aborted())
    }

    fn match_status(&self) -> MatchStatus {
        self.status
    }

    fn game_name(&self) -> String {
        B::game_name()
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

    fn format_info(&self, info: SearchInfo<B>) -> String {
        Self::format_info_impl(info)
    }

    fn graphics(&self) -> GraphicsHandle<B> {
        self.graphics.clone()
    }

    fn set_graphics(&mut self, graphics: GraphicsHandle<B>) {
        self.graphics = graphics;
        self.graphics.borrow_mut().show(self);
    }

    /// all of this will be refactored soon
    fn searcher(&self, _idx: usize) -> &dyn Searcher<B> {
        self.engine.as_ref()
    }

    fn set_engine(&mut self, _: usize, engine: AnyEngine<B>) {
        self.engine = engine;
        self.engine.set_info_callback(Self::info_callback());
    }

    fn set_board(&mut self, board: B) {
        self.board = board;
    }
}

impl<B: Board> AbstractUGI for UGI<B> {
    fn continue_on_error(&self) -> bool {
        self.debug_mode
    }

    fn parse_input(&mut self, input: &str) -> Result<MatchStatus, String> {
        let mut words = input.split_whitespace();
        let first_word = words.next().ok_or_else(|| "Empty input")?;
        match first_word {
            "ugi" => {
                let id_msg = self.id();
                self.write(id_msg.as_str());
                self.write(self.write_options().as_str());
                self.write("ugiok");
                Ok(self.status)
            }
            "uci" => {
                let id_msg = self.id();
                self.write(id_msg.as_str());
                self.write(self.write_options().as_str());
                self.write("uciok");
                Ok(self.status)
            }
            "isready" => {
                self.write("readyok");
                Ok(self.status)
            }
            "debug" => {
                match words.next().unwrap_or_default() {
                    "on" => self.debug_mode = true,
                    "off" => self.debug_mode = false,
                    x => return Err(format!("Invalid debug option '{x}'")),
                }
                Ok(self.status)
            }
            "setoption" => self.handle_setoption(words),
            "register" => Err("'register' isn't supported".to_string()),
            "ucinewgame" => {
                self.engine.forget();
                Ok(Ongoing)
            } // just ignore it
            "position" => self.handle_position(words),
            "go" => self.handle_go(words),
            "stop" => {
                self.engine.stop();
                Ok(Ongoing)
            }
            "ponderhit" => Ok(self.status), // ignore pondering
            "quit" => self.quit(),
            "query" => self.handle_query(words),
            "ui" => self.handle_ui(words),
            "print" => self.handle_print(words),
            "engine" => self.handle_engine(words),
            "game" => self.handle_game(words),
            "play" => {
                play(); // play a game locally without using UGI
                Ok(self.status)
            }
            x => {
                self.graphics.borrow_mut().display_message(
                    Warning,
                    &format!("Ignoring invalid token at start of UCI command '{x}'"),
                );
                let remaining = words.remainder().unwrap_or_default();
                if remaining.is_empty() {
                    return Err(format!("Invalid input '{x}'"));
                }
                self.parse_input(remaining)
                    .map_err(|msg| {
                        format!("Invalid input {x}, ignoring it led to '{remaining}' followed by error '{msg}'")
                    })
            }
        }
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
    pub fn new(mut engine: AnyEngine<B>, graphics: GraphicsHandle<B>) -> Self {
        engine.set_info_callback(Self::info_callback());
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
        }
    }

    fn format_info_impl(info: SearchInfo<B>) -> String {
        let score_str = if let Some(moves_until_over) = info.score.moves_until_game_won() {
            format!("mate {moves_until_over}")
        } else {
            format!("cp {0}", info.score.0) // TODO: WDL normalization
        };

        format!(
            "info depth {depth}{seldepth} score {score_str} time {time} nodes {nodes} nps {nps} pv {pv}{hashfull}{string}",
            depth = info.depth, time = info.time.as_millis(), nodes = info.nodes,
            seldepth = info.seldepth.map(|d| format!(" seldepth {d}")).unwrap_or_default(),
            nps=info.nps(),
            pv = info.pv.iter().map(|mv| mv.to_compact_text()).collect::<Vec<_>>().join(" "),
            hashfull = info.hashfull.map(|f| format!(" hashfull {f}")).unwrap_or_default(),
            string = info.additional.map(|s| format!(" string {s}")).unwrap_or_default()
        )
    }

    fn info_callback() -> InfoCallback<B> {
        InfoCallback {
            // TODO: Use self.write_info, if the borrow checker allows that
            func: |info| println!("{}", Self::format_info_impl(info).as_str()),
        }
    }

    fn quit(&mut self) -> Result<MatchStatus, String> {
        self.next_match = None;
        self.abort()
    }

    fn id(&self) -> String {
        format!(
            "id name Motors - {0} {1}\nid author ToTheAnd",
            self.engine.name(),
            self.engine.version()
        )
    }

    fn handle_setoption(&mut self, mut words: SplitWhitespace) -> Result<MatchStatus, String> {
        let name = words.next().unwrap_or_default();
        if name != "name" {
            return Err(format!(
                "Invalid option command: Expected 'name', got '{name};"
            ));
        }
        let mut name = String::default();
        loop {
            let next_word = words.next().unwrap_or_default();
            if next_word == "value" || next_word == "" {
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
        let value = value.trim();
        self.engine.set_option(name.clone(), value).or_else(|err| {
            if name == Threads && value == "1" {
                Ok(())
            } else {
                Err(err)
            }
        })?;
        Ok(Ongoing)
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
        let result_message = match search_type {
            Normal => format_search_result(self.engine.search(
                self.board,
                limit,
                self.board_hist.0.clone(),
            )),
            Perft => format!("{0}", perft(limit.depth, self.board)),
            SplitPerft => format!("{0}", split_perft(limit.depth, self.board)),
            Bench => {
                let depth = if limit.depth == usize::MAX {
                    self.engine.deref().default_bench_depth()
                } else {
                    limit.depth
                };
                run_bench_with_depth(self.engine.deref_mut(), depth)
            }
        };
        self.write(result_message.as_str());
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
        let mut opts = self.engine.get_options();
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
}
