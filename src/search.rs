use std::collections::HashMap;
use std::fmt::Debug;
use std::time::{Duration, Instant};

use colored::Colorize;
use rand::thread_rng;

use crate::games::Board;
use crate::play::AnyMutEngineRef;

pub mod generic_negamax;
pub mod human;
pub mod naive_slow_negamax;
pub mod perft;
pub mod random_mover;

#[derive(Default, Debug)]
pub struct BenchResult {
    pub nodes: u64,
    pub time: Duration,
    pub depth: usize,
}

// TODO: Turn this into an enum that can also represent a win in n plies (and maybe a draw?)
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct Score(pub i32);

impl Score {
    pub fn is_game_won_score(self) -> bool {
        self.0 >= MIN_SCORE_WON
    }
    pub fn is_game_lost_score(self) -> bool {
        self.0 <= MAX_SCORE_LOST
    }
    /// Returns a negative number of if the game is lost
    pub fn plies_until_game_won(self) -> Option<isize> {
        if self.is_game_won_score() {
            Some((SCORE_WON - self.0) as isize)
        } else if self.is_game_lost_score() {
            Some((SCORE_LOST - self.0) as isize)
        } else {
            None
        }
    }
    /// Returns a negative number if the game is lost
    pub fn moves_until_game_won(self) -> Option<isize> {
        self.plies_until_game_won()
            .map(|n| (n as f32 / 2f32).ceil() as isize)
    }
}

pub const SCORE_LOST: i32 = -31_000;
pub const SCORE_WON: i32 = 31_000;
pub const SCORE_TIME_UP: i32 = SCORE_WON + 1;
pub const MIN_SCORE_WON: i32 = SCORE_WON - 1000;
pub const MAX_SCORE_LOST: i32 = SCORE_LOST + 1000;

pub const MAX_DEPTH: usize = 10_000;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct TimeControl {
    pub remaining: Duration,
    pub increment: Duration,
    pub moves_to_go: usize,
}

impl Default for TimeControl {
    fn default() -> Self {
        TimeControl {
            remaining: Duration::MAX,
            increment: Duration::MAX,
            moves_to_go: 0,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SearchLimit {
    pub tc: TimeControl,
    pub fixed_time: Duration,
    pub depth: usize,
    pub nodes: u64,
}

impl Default for SearchLimit {
    fn default() -> Self {
        SearchLimit {
            tc: TimeControl::default(),
            fixed_time: Duration::MAX,
            depth: MAX_DEPTH,
            nodes: u64::MAX,
        }
    }
}

impl SearchLimit {
    pub fn infinite() -> Self {
        Self::default()
    }

    pub fn tc(tc: TimeControl) -> Self {
        let mut res = Self::infinite();
        res.tc = tc;
        res
    }

    pub fn per_move(time: Duration) -> Self {
        let mut res = Self::infinite();
        res.fixed_time = time;
        res
    }
}

/// An Engine can use two different limits to implement soft/hard time/node management
fn should_stop<B: Board, E: Engine<B>>(
    limit: &SearchLimit,
    engine: &E,
    start_time: Instant,
) -> bool {
    engine.time_up(limit.tc, limit.fixed_time, start_time) || engine.nodes() >= limit.nodes
}

#[derive(Eq, PartialEq, Debug, Default)]
pub struct SearchResult<B: Board> {
    pub chosen_move: B::Move,
    pub score: Option<Score>,
    pub pv: Option<Vec<B::Move>>,
    pub additional: HashMap<String, String>,
}

impl<B: Board> SearchResult<B> {
    fn move_only(chosen_move: B::Move) -> Self {
        Self {
            chosen_move,
            ..Default::default()
        }
    }

    fn move_and_score(chosen_move: B::Move, score: Score) -> Self {
        Self {
            chosen_move,
            score: Some(score),
            ..Default::default()
        }
    }
}

pub struct SearchInfo<B: Board> {
    pub best_move: B::Move,
    pub depth: usize,
    pub seldepth: Option<usize>,
    pub time: Duration,
    pub nodes: u64,
    pub pv: Vec<B::Move>,
    pub score: Score,
    pub hashfull: Option<usize>,
    pub additional: Option<String>,
}

impl<B: Board> SearchInfo<B> {
    pub fn nps(&self) -> usize {
        let micros = self.time.as_micros() as f64;
        if micros == 0.0 {
            0
        } else {
            ((self.nodes as f64 * 1_000_000.0) / micros) as usize
        }
    }

    /// This function is the default for the info callback function.
    pub fn ignore(self) {
        // do nothing.
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum EngineUciOptionType {
    Check,
    Spin,
    Combo,
    Button,
    String,
}

impl EngineUciOptionType {
    pub fn to_str(self) -> &'static str {
        match self {
            EngineUciOptionType::Check => "check",
            EngineUciOptionType::Spin => "spin",
            EngineUciOptionType::Combo => "combo",
            EngineUciOptionType::Button => "button",
            EngineUciOptionType::String => "string",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct EngineOptionType {
    pub name: String,
    pub typ: EngineUciOptionType,
    pub default: Option<String>,
    pub min: Option<String>,
    pub max: Option<String>,
    pub vars: Vec<String>,
}

pub trait Searcher<B: Board>: Debug + 'static {
    /// The important function.
    fn search(&mut self, pos: B, limit: SearchLimit) -> SearchResult<B>;

    fn time_up(&self, tc: TimeControl, hard_limit: Duration, start_time: Instant) -> bool;

    /// Returns the name of this searcher, such as `"Random Mover"` or `"Human"`.
    /// Takes &self as parameter so that Searcher can be made into an object.
    fn name(&self) -> &'static str;
}

pub trait Engine<B: Board>: Searcher<B> {
    fn bench(&mut self, position: B, depth: usize) -> BenchResult;

    /// Stop the current search. Can be called from another thread while the search is running.
    fn stop(&mut self) -> Result<SearchResult<B>, String>;
    // {
    //     Err(format!("The engine '{0}' can't be stopped", self.name()))
    // }

    fn set_info_callback(&mut self, f: InfoCallback<B>);

    /// Returns a SearchInfo object with information about the search so far.
    /// Can be called during search, only returns the information regarding the current thread.
    fn search_info(&self) -> SearchInfo<B>;

    /// Reset the engine into a fresh state, e.g. by clearing the TT and various heuristics.
    fn forget(&mut self);

    /// Returns the number of nodes looked at so far. Can be called during search.
    /// For smp, this may also return the number of nodes looked at at the current thread.
    fn nodes(&self) -> u64;

    /// Returns the version of this searcher, such as `"0.1.0"`.
    /// Takes &self as parameter so that Searcher can be made into an object.
    fn version(&self) -> &'static str {
        "0.0.0"
    }

    /// Sets an option with the name 'option' to the value 'value'
    fn set_option(&mut self, option: &str, value: &str) -> Result<(), String> {
        Err(format!(
            "The searcher '{name}' doesn't support setting options, including setting '{option}' to '{value}'",
            name=self.name()
        ))
    }

    /// Returns a list of textual representations of all options of this searcher
    fn get_options(&self) -> Vec<EngineOptionType> {
        Vec::default()
    }
}

#[derive(Copy, Clone, Debug)]
pub struct InfoCallback<B: Board> {
    pub func: fn(SearchInfo<B>) -> (),
}

impl<B: Board> InfoCallback<B> {
    fn call(self, info: SearchInfo<B>) {
        (self.func)(info)
    }
}

impl<B: Board> Default for InfoCallback<B> {
    fn default() -> Self {
        Self {
            func: SearchInfo::ignore,
        }
    }
}

pub trait EngineState<B: Board> {
    fn forget(&mut self);
    fn nodes(&self) -> u64;
    fn depth(&self) -> usize;
    fn start_time(&self) -> Instant;
    fn mov(&self) -> B::Move;
    fn score(&self) -> Score;
    fn pv(&self) -> Vec<B::Move> {
        vec![self.mov()]
    }
    fn hashfull(&self) -> Option<usize> {
        None
    }
    fn seldepth(&self) -> Option<usize> {
        None
    }
    fn additional(&self) -> Option<String> {
        None
    }
    fn to_info(&self) -> SearchInfo<B> {
        SearchInfo {
            best_move: self.mov(),
            depth: self.depth(),
            seldepth: self.seldepth(),
            time: self.start_time().elapsed(),
            nodes: self.nodes(),
            pv: self.pv(),
            score: self.score(),
            hashfull: self.hashfull(),
            additional: self.additional(),
        }
    }
    fn info_callback(&self) -> InfoCallback<B>;
    fn send_new_info(&self) {
        self.info_callback().call(self.to_info());
    }

    fn to_bench_res(&self) -> BenchResult {
        BenchResult {
            nodes: self.nodes(),
            time: self.start_time().elapsed(),
            depth: self.depth(),
        }
    }
}

#[derive(Debug)]
struct SimpleSearchState<B: Board> {
    initial_pos: B,
    best_move: Option<B::Move>,
    nodes: u64,
    search_cancelled: bool,
    info_callback: InfoCallback<B>,
    depth: usize,
    start_time: Instant,
    score: Score,
}

impl<B: Board> SimpleSearchState<B> {
    fn initial_state(initial_pos: B, info_callback: InfoCallback<B>) -> Self {
        Self {
            initial_pos,
            info_callback,
            ..Default::default()
        }
    }
}

impl<B: Board> Default for SimpleSearchState<B> {
    fn default() -> Self {
        let start_time = Instant::now();
        Self {
            initial_pos: B::default(),
            start_time,
            score: Score(0),
            best_move: None,
            nodes: 0,
            search_cancelled: false,
            info_callback: InfoCallback::default(),
            depth: 0,
        }
    }
}

impl<B: Board> EngineState<B> for SimpleSearchState<B> {
    fn forget(&mut self) {
        let default_val = SimpleSearchState::default();
        *self = default_val;
    }

    fn nodes(&self) -> u64 {
        self.nodes
    }

    fn depth(&self) -> usize {
        self.depth
    }

    fn start_time(&self) -> Instant {
        self.start_time
    }

    fn mov(&self) -> B::Move {
        self.best_move.unwrap_or_default()
    }

    fn score(&self) -> Score {
        self.score
    }

    fn info_callback(&self) -> InfoCallback<B> {
        self.info_callback
    }
}

fn stop_engine<B: Board>(
    initial_pos: &B,
    chosen_move: Option<B::Move>,
    score: Score,
) -> Result<SearchResult<B>, String> {
    Ok(SearchResult::move_and_score(
        chosen_move.unwrap_or_else(|| {
            let mut rng = thread_rng();
            initial_pos
                .random_legal_move(&mut rng)
                .expect("search and stop() called in a position with no legal moves")
        }),
        score,
    ))
}

pub fn run_bench<B: Board>(engine: AnyMutEngineRef<B>, mut depth: usize) -> String {
    if depth == MAX_DEPTH {
        depth = 5; // Default value
    }
    let mut sum = BenchResult::default();
    for position in B::bench_positions() {
        engine.forget();
        let res = engine.bench(position, depth);
        sum.nodes += res.nodes;
        sum.time += res.time;
        sum.depth = sum.depth.max(res.depth);
    }
    format!(
        "depth {0}, nodes {1}, time {2}ms, nps {3}k",
        sum.depth,
        sum.nodes,
        sum.time.as_millis(),
        ((sum.nodes as f64 / sum.time.as_millis() as f64).round())
            .to_string()
            .red()
    )
}
