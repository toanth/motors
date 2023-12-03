use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut, Div, Mul};
use std::str::FromStr;
use std::time::{Duration, Instant};

use colored::Colorize;
use derive_more::{Add, AddAssign, Neg, Sub};

use crate::games::{Board, PlayerResult, ZobristHistoryBase, ZobristRepetition2Fold};
use crate::play::AnyMutEngineRef;
use crate::search::tt::TTScoreType::*;
use crate::search::tt::{TTScoreType, TT};

pub mod chess;
pub mod generic_negamax;
pub mod human;
pub mod naive_slow_negamax;
pub mod perft;
pub mod random_mover;
mod tt;

#[derive(Default, Debug)]
pub struct BenchResult {
    pub nodes: u64,
    pub time: Duration,
    pub depth: usize,
}

// TODO: Turn this into an enum that can also represent a win in n plies (and maybe a draw?)
#[derive(Default, Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Add, Sub, Neg, AddAssign)]
pub struct Score(pub i32);

impl Add<i32> for Score {
    type Output = Score;

    fn add(self, rhs: i32) -> Self::Output {
        Score(self.0 + rhs)
    }
}

impl Sub<i32> for Score {
    type Output = Score;

    fn sub(self, rhs: i32) -> Self::Output {
        Score(self.0 - rhs)
    }
}

impl Mul<i32> for Score {
    type Output = Score;

    fn mul(self, rhs: i32) -> Self::Output {
        Score(self.0 * rhs)
    }
}

impl Div<i32> for Score {
    type Output = Score;

    fn div(self, rhs: i32) -> Self::Output {
        Score(self.0 / rhs)
    }
}

impl Score {
    pub fn is_game_won_score(self) -> bool {
        self >= MIN_SCORE_WON
    }
    pub fn is_game_lost_score(self) -> bool {
        self <= MAX_SCORE_LOST
    }
    /// Returns a negative number of plies if the game is lost
    pub fn plies_until_game_won(self) -> Option<isize> {
        if self.is_game_won_score() {
            Some((SCORE_WON - self).0 as isize)
        } else if self.is_game_lost_score() {
            Some((SCORE_LOST - self).0 as isize)
        } else {
            None
        }
    }
    /// Returns a negative number if the game is lost
    pub fn moves_until_game_won(self) -> Option<isize> {
        self.plies_until_game_won()
            .map(|n| (n as f32 / 2f32).ceil() as isize)
    }

    fn bound(self, original_alpha: Score, beta: Score) -> TTScoreType {
        if self <= original_alpha {
            UpperBound
        } else if self >= beta {
            LowerBound
        } else {
            Exact
        }
    }
}

pub const SCORE_LOST: Score = Score(-31_000);
pub const SCORE_WON: Score = Score(31_000);
pub const SCORE_TIME_UP: Score = Score(SCORE_WON.0 + 1); // can't use + directly because derive_more's + isn't `const`
pub const MIN_SCORE_WON: Score = Score(SCORE_WON.0 - 1000);
pub const MAX_SCORE_LOST: Score = Score(SCORE_LOST.0 + 1000);

pub const MAX_DEPTH: usize = 10_000;

pub fn game_result_to_score(res: PlayerResult, ply: usize) -> Score {
    match res {
        PlayerResult::Win => SCORE_WON - ply as i32,
        PlayerResult::Lose => SCORE_LOST + ply as i32,
        PlayerResult::Draw => Score(0),
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct Pv<B: Board, const LIMIT: usize> {
    list: [B::Move; LIMIT],
    length: usize,
}

impl<B: Board, const LIMIT: usize> Default for Pv<B, LIMIT> {
    fn default() -> Self {
        Self {
            list: [B::Move::default(); LIMIT],
            length: 0,
        }
    }
}

/// Implements a triangular pv table, except that it's actually quadratic
/// (the doubled memory requirements should be inconsequential, and this is much easier to implement
/// and potentially faster)
#[derive(Debug)]
pub struct GenericPVTable<B: Board, const LIMIT: usize> {
    pv_at_depth: [Pv<B, LIMIT>; LIMIT],
    size: usize,
}

impl<B: Board, const LIMIT: usize> Default for GenericPVTable<B, LIMIT> {
    fn default() -> Self {
        Self {
            pv_at_depth: [Default::default(); LIMIT],
            size: 0,
        }
    }
}

impl<B: Board, const LIMIT: usize> GenericPVTable<B, LIMIT> {
    fn new_pv_move(&mut self, ply: usize, mov: B::Move) {
        debug_assert!(ply < LIMIT);
        self.size = self.size.max(ply + 1);
        let len = self.pv_at_depth[ply + 1].length.max(ply + 1);
        self.pv_at_depth[ply].length = len;
        self.pv_at_depth[ply + 1].length = 0;
        let (dest_arr, src_arr) = self.pv_at_depth.split_at_mut(ply + 1);
        let (dest_arr, src_arr) = (&mut dest_arr[ply], &mut src_arr[0]);
        dest_arr.list[ply] = mov;
        dest_arr.list[ply + 1..len].copy_from_slice(&src_arr.list[ply + 1..len]);
    }

    fn reset(&mut self) {
        self.pv_at_depth[..self.size]
            .iter_mut()
            .for_each(|pv| pv.length = 0);
        self.size = 0;
    }

    fn get_pv(&self) -> &[B::Move] {
        &self.pv_at_depth[0].list[..self.pv_at_depth[0].length]
    }
}

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
            increment: Duration::from_millis(0),
            moves_to_go: 30,
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

    pub fn depth(depth: usize) -> Self {
        let mut res = Self::infinite();
        res.depth = depth;
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
pub enum EngineOptionName {
    Hash,
    Threads,
    Ponder,
    MultiPv,
    Other(String),
}

impl Display for EngineOptionName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EngineOptionName::Hash => "Hash",
            EngineOptionName::Threads => "Threads",
            EngineOptionName::Ponder => "Ponder",
            EngineOptionName::MultiPv => "MultiPV",
            EngineOptionName::Other(x) => &x,
        };
        write!(f, "{s}")
    }
}

impl FromStr for EngineOptionName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "hash" => EngineOptionName::Hash,
            "threads" => EngineOptionName::Threads,
            "ponder" => EngineOptionName::Ponder,
            "multipv" => EngineOptionName::MultiPv,
            _ => EngineOptionName::Other(s.to_string()),
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct EngineOptionType {
    pub name: EngineOptionName,
    pub typ: EngineUciOptionType,
    pub default: Option<String>,
    pub min: Option<String>,
    pub max: Option<String>,
    pub vars: Vec<String>,
}

pub trait Searcher<B: Board>: Debug + 'static {
    /// The important function.
    fn search(
        &mut self,
        pos: B,
        limit: SearchLimit,
        history: ZobristHistoryBase,
    ) -> SearchResult<B>;

    fn time_up(&self, tc: TimeControl, hard_limit: Duration, start_time: Instant) -> bool;

    /// Returns the name of this searcher, such as `"Random Mover"` or `"Human"`.
    /// Takes &self as parameter so that Searcher can be made into an object.
    fn name(&self) -> &'static str;
}

pub trait Engine<B: Board>: Searcher<B> {
    fn bench(&mut self, position: B, depth: usize) -> BenchResult;

    fn default_bench_depth(&self) -> usize;

    /// Stop the current search. Can be called from another thread while the search is running.
    fn stop(&mut self);

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
    fn set_option(&mut self, option: EngineOptionName, value: &str) -> Result<(), String> {
        Err(format!(
            "The engine '{name}' doesn't support setting options, including setting '{option}' to '{value}'",
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

#[derive(Debug)]
struct SimpleSearchState<B: Board> {
    tt: TT<B>,
    board_history: ZobristRepetition2Fold,
    best_move: Option<B::Move>,
    nodes: u64,
    search_cancelled: bool,
    info_callback: InfoCallback<B>,
    depth: usize,
    start_time: Instant,
    score: Score,
}

impl<B: Board> Default for SimpleSearchState<B> {
    fn default() -> Self {
        let start_time = Instant::now();
        Self {
            tt: Default::default(),
            board_history: ZobristRepetition2Fold::default(),
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

impl<B: Board> SimpleSearchState<B> {
    fn new_search(&mut self, history: ZobristRepetition2Fold) {
        self.board_history = history;
        self.start_time = Instant::now();
        self.score = Score(0);
        self.best_move = None;
        self.nodes = 0;
        self.search_cancelled = false;
        self.depth = 0;
        // Don't reset the TT or the info callback
    }

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

    fn pv(&self) -> Vec<B::Move> {
        vec![self.mov()]
    }

    fn info_callback(&self) -> InfoCallback<B> {
        self.info_callback
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

#[derive(Default, Debug)]
struct SearchStateWithPv<B: Board, const PV_LIMIT: usize> {
    wrapped: SimpleSearchState<B>,
    pv_table: GenericPVTable<B, PV_LIMIT>,
}

impl<B: Board, const PV_LIMIT: usize> Deref for SearchStateWithPv<B, PV_LIMIT> {
    type Target = SimpleSearchState<B>;

    fn deref(&self) -> &Self::Target {
        &self.wrapped
    }
}

impl<B: Board, const PV_LIMIT: usize> DerefMut for SearchStateWithPv<B, PV_LIMIT> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.wrapped
    }
}

impl<B: Board, const PV_LIMIT: usize> SearchStateWithPv<B, PV_LIMIT> {
    fn new_search(&mut self, history: ZobristRepetition2Fold) {
        self.wrapped.new_search(history);
        self.pv_table.reset();
    }

    fn forget(&mut self) {
        self.wrapped.forget();
        self.pv_table.reset();
    }

    fn nodes(&self) -> u64 {
        self.wrapped.nodes()
    }

    fn depth(&self) -> usize {
        self.wrapped.depth()
    }

    fn start_time(&self) -> Instant {
        self.wrapped.start_time()
    }

    fn mov(&self) -> B::Move {
        self.wrapped.mov()
    }

    fn score(&self) -> Score {
        self.wrapped.score()
    }

    fn pv(&self) -> Vec<B::Move> {
        self.pv_table.get_pv().to_vec()
    }

    fn info_callback(&self) -> InfoCallback<B> {
        self.wrapped.info_callback()
    }
}

pub fn run_bench<B: Board>(engine: AnyMutEngineRef<B>) -> String {
    let depth = engine.default_bench_depth();
    run_bench_with_depth(engine, depth)
}
pub fn run_bench_with_depth<B: Board>(engine: AnyMutEngineRef<B>, mut depth: usize) -> String {
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
        "depth {0}, time {2}ms, {1} nodes, {3} nps",
        sum.depth,
        sum.nodes,
        sum.time.as_millis(),
        ((sum.nodes as f64 / sum.time.as_millis() as f64 * 1000.0).round())
            .to_string()
            .red()
    )
}
