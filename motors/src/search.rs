use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::time::{Duration, Instant};

use colored::Colorize;
use dyn_clone::{clone_box, DynClone};
use strum_macros::FromRepr;

use gears::games::{Board, ZobristHistoryBase, ZobristRepetition2Fold};
use gears::general::common::{EntityList, NamedEntity, Res, StaticallyNamedEntity};
use gears::search::{Depth, Nodes, Score, SearchInfo, SearchLimit, SearchResult, TimeControl};
use gears::ugi::{EngineOption, EngineOptionName};

use crate::search::multithreading::{EngineWrapper, SearchSender};
use crate::search::Searching::*;
use crate::search::tt::TT;

#[cfg(feature = "chess")]
pub mod chess;
pub mod generic;
pub mod human;
pub mod multithreading;
mod tt;

#[derive(Default, Debug, Clone)]
pub struct EngineInfo {
    pub name: String,
    pub version: String,
    pub default_bench_depth: Depth,
    pub options: Vec<EngineOption>,
    pub description: String, // TODO: Use
                             // TODO: NamedEntity?
}

#[derive(Debug)]
pub struct BenchResult {
    pub nodes: Nodes,
    pub time: Duration,
    pub depth: Depth,
}

impl Default for BenchResult {
    fn default() -> Self {
        Self {
            nodes: Nodes::MIN,
            time: Duration::default(),
            depth: Depth::MIN,
        }
    }
}

impl Display for BenchResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "depth {0}, time {2}ms, {1} nodes, {3} nps",
            self.depth.get(),
            self.nodes,
            self.time.as_millis(),
            ((self.nodes.get() as f64 / self.time.as_millis() as f64 * 1000.0).round())
                .to_string()
                .red()
        )
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
#[derive(Debug, Clone)]
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
        debug_assert!(ply + 1 < LIMIT);
        self.size = self.size.max(ply + 1);
        let len = self.pv_at_depth[ply + 1].length.max(ply + 1);
        debug_assert!(
            // no reductions on pv nodes (yet)
            self.pv_at_depth[ply + 1].length == 0 || self.pv_at_depth[ply + 1].length >= ply + 2
        );
        self.pv_at_depth[ply].length = len;
        self.pv_at_depth[ply + 1].length = 0;
        let (dest_arr, src_arr) = self.pv_at_depth.split_at_mut(ply + 1);
        let (dest_arr, src_arr) = (&mut dest_arr[ply], &mut src_arr[0]);
        dest_arr.list[ply] = mov;
        dest_arr.list[ply + 1..len].copy_from_slice(&src_arr.list[ply + 1..len]);
    }

    fn no_pv_move(&mut self, ply: usize) {
        self.pv_at_depth[ply].length = 0;
        self.pv_at_depth[ply + 1].length = 0;
    }

    fn reset(&mut self) {
        self.pv_at_depth[..self.size]
            .iter_mut()
            .for_each(|pv| pv.length = 0);
        self.size = 0;
    }

    // TODO: Fix pv table

    fn get_pv(&self) -> &[B::Move] {
        &self.pv_at_depth[0].list[..self.pv_at_depth[0].length]
    }
}

/// A trait because this type erases over the Engine being built.
pub trait AbstractEngineBuilder<B: Board>: NamedEntity + DynClone {
    fn build(&self, sender: SearchSender<B>, tt: TT) -> EngineWrapper<B>;

    fn build_for_bench(&self) -> Box<dyn Benchable<B>>;

    fn can_use_multiple_threads(&self) -> bool;
}

#[derive(Debug)]
pub struct EngineWrapperBuilder<B: Board> {
    builder: Box<dyn AbstractEngineBuilder<B>>,
    sender: SearchSender<B>,
}

impl<B: Board> Clone for EngineWrapperBuilder<B> {
    fn clone(&self) -> Self {
        Self {
            builder: clone_box(self.builder.deref()),
            sender: self.sender.clone(),
        }
    }
}

impl<B: Board> EngineWrapperBuilder<B> {
    pub fn new(builder: Box<dyn AbstractEngineBuilder<B>>, sender: SearchSender<B>) -> Self {
        Self { builder, sender }
    }

    pub fn build(&self) -> EngineWrapper<B> {
        let sender = self.sender.clone();
        self.builder.build(sender, TT::default())
    }
}

pub type EngineList<B> = EntityList<Box<dyn AbstractEngineBuilder<B>>>;

#[derive(Debug, Default)]
pub struct EngineBuilder<B: Board, E: Engine<B>> {
    state: E::State,
}

impl<B: Board, E: Engine<B>> Clone for EngineBuilder<B, E> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl<B: Board, E: Engine<B>> EngineBuilder<B, E> {
    pub fn new() -> Self {
        Self {
            state: E::State::default(),
        }
    }
}

impl<B: Board, E: Engine<B>> AbstractEngineBuilder<B> for EngineBuilder<B, E> {
    fn build(&self, sender: SearchSender<B>, tt: TT) -> EngineWrapper<B> {
        EngineWrapper::new_with_tt(E::new(self.state.clone()), sender, clone_box(self), tt)
    }

    fn build_for_bench(&self) -> Box<dyn Benchable<B>> {
        Box::new(E::new(self.state.clone()))
    }

    fn can_use_multiple_threads(&self) -> bool {
        E::can_use_multiple_threads()
    }
}

impl<B: Board, E: Engine<B>> StaticallyNamedEntity for EngineBuilder<B, E> {
    fn static_short_name() -> &'static str {
        E::static_short_name()
    }

    fn static_long_name() -> &'static str {
        E::static_long_name()
    }

    fn static_description() -> &'static str {
        E::static_description()
    }
}

// TODO: Necessary?
// pub trait EngineBase: StaticallyNamedEntity + Debug + Default + Send + 'static /*+ Send*/ {}

pub trait Benchable<B: Board>: StaticallyNamedEntity + Debug {
    fn bench(&mut self, position: B, depth: Depth) -> BenchResult;

    /// Returns information about this engine, such as the name, version and default bench depth.

    fn engine_info(&self) -> EngineInfo;

    /// Sets an option with the name 'option' to the value 'value'

    fn set_option(&mut self, option: EngineOptionName, value: String) -> Res<()> {
        Err(format!(
            "The engine '{name}' doesn't support setting custom options, including setting '{option}' to '{value}' (Note: 'Hash' and 'Threads' may still be supported)",
            name = self.long_name()
        ))
    }
}

pub trait Engine<B: Board>: Benchable<B> + Default + Send + 'static {
    type State: BasicSearchState<B>;

    fn new(state: Self::State) -> Self;

    fn set_tt(&mut self, tt: TT);

    /// The important function.

    fn search(
        &mut self,
        pos: B,
        limit: SearchLimit,
        history: ZobristHistoryBase,
        sender: &mut SearchSender<B>,
    ) -> Res<SearchResult<B>>;

    fn time_up(&self, tc: TimeControl, hard_limit: Duration, start_time: Instant) -> bool;

    // Sensible default values, but engines may choose to check more/less frequently than every 1024 nodes

    fn should_stop_impl(&self, limit: SearchLimit, sender: &SearchSender<B>) -> bool {
        let state = self.search_state();
        if state.nodes() >= limit.nodes {
            return true;
        }
        if state.nodes().get() % 1024 != 0 {
            return false;
        }
        self.time_up(limit.tc, limit.fixed_time, self.search_state().start_time())
            || sender.should_stop()
            || state.search_cancelled()
    }

    fn should_stop(&mut self, limit: SearchLimit, sender: &SearchSender<B>) -> bool {
        if self.should_stop_impl(limit, sender) {
            self.search_state_mut().set_searching(Stop);
            true
        } else {
            false
        }
    }

    fn search_state(&self) -> &Self::State;

    fn search_state_mut(&mut self) -> &mut Self::State;

    /// Stop the current search. Can be called from another thread while the search is running.

    fn stop(&mut self) {
        self.search_state_mut().set_searching(Stop);
    }

    fn quit(&mut self) {
        self.stop();
        self.search_state_mut().quit();
    }

    /// Returns a SearchInfo object with information about the search so far.
    /// Can be called during search, only returns the information regarding the current thread.

    fn search_info(&self) -> SearchInfo<B> {
        self.search_state().to_search_info()
    }

    /// Reset the engine into a fresh state, e.g. by clearing the TT and various heuristics.

    fn forget(&mut self) {
        self.search_state_mut().forget()
    }

    /// Returns the number of nodes looked at so far. Can be called during search.
    /// For smp, this may also return the number of nodes looked at at the current thread.

    fn nodes(&self) -> Nodes {
        self.search_state().nodes()
    }

    fn can_use_multiple_threads() -> bool
    where
        Self: Sized;
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Searching {
    Ongoing,
    Stop,
}

pub trait BasicSearchState<B: Board>: Debug + Default + Clone {
    fn nodes(&self) -> Nodes;
    fn searching(&self) -> Searching;
    fn set_searching(&mut self, searching: Searching);
    fn search_cancelled(&self) -> bool {
        self.searching() != Ongoing
    }
    fn should_stop(&self) -> bool;
    fn quit(&mut self);
    fn depth(&self) -> Depth;
    fn start_time(&self) -> Instant;
    fn score(&self) -> Score;
    fn forget(&mut self);
    fn new_search(&mut self, history: ZobristRepetition2Fold);
    fn to_search_info(&self) -> SearchInfo<B>;
}

#[derive(Debug, Clone)]
pub struct SimpleSearchState<B: Board> {
    board_history: ZobristRepetition2Fold,
    best_move: Option<B::Move>,
    nodes: u64,
    searching: Searching,
    should_stop: bool,
    depth: Depth,
    sel_depth: usize,
    start_time: Instant,
    score: Score,
}

impl<B: Board> Default for SimpleSearchState<B> {
    fn default() -> Self {
        let start_time = Instant::now();
        Self {
            board_history: ZobristRepetition2Fold::default(),
            start_time,
            score: Score(0),
            best_move: None,
            nodes: 0,
            searching: Stop,
            should_stop: false,
            depth: Depth::MIN,
            sel_depth: 0,
        }
    }
}

impl<B: Board> SimpleSearchState<B> {
    fn mov(&self) -> B::Move {
        self.best_move.unwrap_or_default()
    }

    fn pv(&self) -> Vec<B::Move> {
        vec![self.mov()]
    }

    fn hashfull(&self) -> Option<usize> {
        None
    }
    fn seldepth(&self) -> Option<usize> {
        if self.sel_depth == 0 {
            None
        } else {
            Some(self.sel_depth)
        }
    }
    fn additional(&self) -> Option<String> {
        None
    }

    fn to_bench_res(&self) -> BenchResult {
        BenchResult {
            nodes: self.nodes(),
            time: self.start_time().elapsed(),
            depth: self.depth(),
        }
    }
}

impl<B: Board> BasicSearchState<B> for SimpleSearchState<B> {
    fn nodes(&self) -> Nodes {
        Nodes::new(self.nodes).unwrap()
    }

    fn searching(&self) -> Searching {
        self.searching
    }

    fn set_searching(&mut self, searching: Searching) {
        self.searching = searching;
    }

    fn should_stop(&self) -> bool {
        self.should_stop
    }

    fn quit(&mut self) {
        self.should_stop = true;
    }

    fn depth(&self) -> Depth {
        self.depth
    }

    fn start_time(&self) -> Instant {
        self.start_time
    }

    fn score(&self) -> Score {
        self.score
    }

    fn forget(&mut self) {
        *self = SimpleSearchState::default();
    }

    fn new_search(&mut self, history: ZobristRepetition2Fold) {
        self.forget();
        self.board_history = history;
        self.searching = Ongoing;
    }

    fn to_search_info(&self) -> SearchInfo<B> {
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
}

// TODO: Remove, store PV table in search stack
#[derive(Default, Debug, Clone)]
pub struct SearchStateWithPv<B: Board, const PV_LIMIT: usize> {
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

    fn nodes(&self) -> Nodes {
        self.wrapped.nodes()
    }

    fn depth(&self) -> Depth {
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

    fn to_search_info(&self) -> SearchInfo<B> {
        let mut res = self.wrapped.to_search_info();
        // res.pv = self.pv(); // TODO: Re-enable PV output
        res
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, FromRepr)]
#[repr(u8)]
pub enum NodeType {
    #[default]
    Empty,
    LowerBound,
    // score greater than beta, cut-node
    Exact,
    // score between alpha and beta, PV node (important node!)
    UpperBound, // score less than alpha, all-node (relatively rare, but makes parent a cut-node)
}

pub fn run_bench<B: Board>(engine: &mut dyn Benchable<B>) -> BenchResult {
    let depth = engine.engine_info().default_bench_depth;
    run_bench_with_depth(engine, depth)
}

pub fn run_bench_with_depth<B: Board>(
    engine: &mut dyn Benchable<B>,
    mut depth: Depth,
) -> BenchResult {
    if depth.get() <= 0 {
        depth = engine.engine_info().default_bench_depth
    }
    let mut sum = BenchResult::default();
    for position in B::bench_positions() {
        let res = engine.bench(position, depth);
        sum.nodes = Nodes::new(sum.nodes.get() + res.nodes.get()).unwrap();
        sum.time += res.time;
    }
    sum.depth = depth;
    sum
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "caps")]
    fn simple_mate_test() {
        // let board = Chessboard::from_fen("4k3/8/4K3/8/8/8/8/6R1 w - - 0 1").unwrap();
        // let mut engine = Caps::<RandEval>::default();
        // let res = engine.search(board, SearchLimit::depth(Depth::new(2)), ZobristHistoryBase::default());
        // assert!(res.score.unwrap().is_game_won_score());
        // assert_eq!(res.score.unwrap().plies_until_game_won(), Some(1));
    }
}
