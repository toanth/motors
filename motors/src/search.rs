use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use std::mem::take;
use std::ops::Deref;
use std::time::{Duration, Instant};

use colored::Colorize;
use dyn_clone::{clone_box, DynClone};
use strum_macros::FromRepr;

use gears::games::{Board, ZobristHistoryBase, ZobristRepetition2Fold};
use gears::general::common::{EntityList, NamedEntity, Res, StaticallyNamedEntity};
use gears::search::{
    Depth, NodesLimit, Score, SearchInfo, SearchLimit, SearchResult, TimeControl, SCORE_WON,
};
use gears::ugi::{EngineOption, EngineOptionName};

use crate::search::multithreading::{EngineWrapper, SearchSender};
use crate::search::statistics::SearchType::{MainSearch, Qsearch};
use crate::search::statistics::{SearchType, Statistics};
use crate::search::tt::TT;
use crate::search::Searching::*;

#[cfg(feature = "chess")]
pub mod chess;
pub mod generic;
mod move_picker;
pub mod multithreading;
pub mod statistics;
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
    pub nodes: NodesLimit,
    pub time: Duration,
    pub depth: Depth,
}

impl Default for BenchResult {
    fn default() -> Self {
        Self {
            nodes: NodesLimit::MIN,
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
pub struct Pv<B: Board, const LIMIT: usize> {
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

impl<B: Board, const LIMIT: usize> Pv<B, LIMIT> {
    pub fn push(&mut self, ply: usize, mov: B::Move, child_pv: &Pv<B, LIMIT>) {
        self.list[ply] = mov;
        for i in ply + 1..child_pv.length {
            self.list[i] = child_pv.list[i];
        }
        self.length = (ply + 1).max(child_pv.length);
    }

    pub fn clear(&mut self) {
        self.length = 0;
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
    _phantom_b: PhantomData<B>,
    _phantom_e: PhantomData<E>,
}

impl<B: Board, E: Engine<B>> Clone for EngineBuilder<B, E> {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl<B: Board, E: Engine<B>> EngineBuilder<B, E> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<B: Board, E: Engine<B>> AbstractEngineBuilder<B> for EngineBuilder<B, E> {
    fn build(&self, sender: SearchSender<B>, tt: TT) -> EngineWrapper<B> {
        EngineWrapper::new_with_tt(E::default(), sender, clone_box(self), tt)
    }

    fn build_for_bench(&self) -> Box<dyn Benchable<B>> {
        Box::new(E::default())
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
    fn set_tt(&mut self, tt: TT);

    fn search_from_pos(&mut self, pos: B, limit: SearchLimit) -> Res<SearchResult<B>> {
        self.search(
            pos,
            limit,
            ZobristHistoryBase::default(),
            &mut SearchSender::no_sender(),
        )
    }

    fn search(
        &mut self,
        pos: B,
        limit: SearchLimit,
        history: ZobristHistoryBase,
        sender: &mut SearchSender<B>,
    ) -> Res<SearchResult<B>> {
        self.search_state_mut()
            .new_search(ZobristRepetition2Fold(history));
        let res = self.do_search(pos, limit, sender);
        self.search_state_mut().end_search();
        sender.send_statistics(self.search_state().statistics());
        res
    }

    /// The important function.
    fn do_search(
        &mut self,
        pos: B,
        limit: SearchLimit,
        sender: &mut SearchSender<B>,
    ) -> Res<SearchResult<B>>;

    fn time_up(&self, tc: TimeControl, hard_limit: Duration, start_time: Instant) -> bool;

    // Sensible default values, but engines may choose to check more/less frequently than every 1024 nodes
    fn should_stop(&self, limit: SearchLimit, sender: &SearchSender<B>) -> bool {
        let state = self.search_state();
        // Do the less expensive checks first to avoid querying the time in each node
        if state.main_search_nodes() >= limit.nodes.get() {
            return true;
        }
        if state.main_search_nodes() % 1024 != 0 {
            return false;
        }
        self.time_up(limit.tc, limit.fixed_time, self.search_state().start_time())
            || sender.should_stop()
            || state.search_cancelled()
    }

    fn should_not_start_next_iteration(
        &self,
        soft_limit: Duration,
        max_depth: isize,
        mate_depth: Depth,
    ) -> bool {
        let state = self.search_state();
        state.start_time().elapsed() >= soft_limit
            || state.depth().get() as isize > max_depth
            || state.score() >= Score(SCORE_WON.0 - mate_depth.get() as i32)
    }

    fn quit(&mut self) {
        self.search_state_mut().quit();
    }

    fn search_state(&self) -> &impl SearchState<B>;

    fn search_state_mut(&mut self) -> &mut impl SearchState<B>;

    /// Returns a SearchInfo object with information about the search so far.
    /// Can be called during search, only returns the information regarding the current thread.
    fn search_info(&self) -> SearchInfo<B> {
        self.search_state().to_search_info()
    }

    /// This should return the static eval (possibly with WDL normalization) without doing any kind of search.
    /// For engines like `RandomMover` where there is no static eval, this should return `Score(0)`.
    fn get_static_eval(&mut self, pos: B) -> Score;

    /// Reset the engine into a fresh state, e.g. by clearing the TT and various heuristics.
    fn forget(&mut self) {
        self.search_state_mut().forget(true);
    }

    fn is_currently_searching(&self) -> bool {
        !self.search_state().search_cancelled()
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

pub trait SearchState<B: Board>: Debug + Clone {
    /// Returns the number of nodes looked at so far, excluding quiescent search, SEE and similar. Can be called during search.
    /// For smp, this only returns the number of nodes looked at in the current thread.
    #[inline(always)]
    fn main_search_nodes(&self) -> u64 {
        self.statistics().main_search_nodes()
    }
    fn uci_nodes(&self) -> u64 {
        self.statistics().uci_nodes()
    }
    fn searching(&self) -> Searching;
    fn search_cancelled(&self) -> bool {
        self.searching() != Ongoing
    }
    fn should_stop(&self) -> bool;
    fn quit(&mut self);
    #[inline(always)]
    fn depth(&self) -> Depth {
        Depth::new(self.statistics().depth())
    }
    fn start_time(&self) -> Instant;
    fn score(&self) -> Score;
    fn forget(&mut self, hard: bool);
    fn new_search(&mut self, history: ZobristRepetition2Fold);
    fn end_search(&mut self);
    fn to_search_info(&self) -> SearchInfo<B>;
    fn statistics(&self) -> &Statistics;
}

pub trait SearchStackEntry<B: Board>: Default + Clone + Debug {
    fn forget(&mut self) {
        *self = Self::default();
    }
    fn pv(&self) -> Option<&[B::Move]>;
}

#[derive(Copy, Clone, Default, Debug)]
struct EmptySearchStackEntry {}

impl<B: Board> SearchStackEntry<B> for EmptySearchStackEntry {
    fn pv(&self) -> Option<&[B::Move]> {
        None
    }
}

trait CustomInfo: Default + Clone + Debug {
    fn tt(&self) -> Option<&TT> {
        None
    }
    fn new_search(&mut self) {
        self.forget()
    }
    fn forget(&mut self) {
        // do nothing
    }
}

#[derive(Default, Clone, Debug)]
struct NoCustomInfo {}

impl CustomInfo for NoCustomInfo {}

#[derive(Debug, Clone)]
pub struct ABSearchState<B: Board, E: SearchStackEntry<B>, C: CustomInfo> {
    search_stack: Vec<E>,
    board_history: ZobristRepetition2Fold,
    custom: C,
    best_move: Option<B::Move>,
    searching: Searching,
    should_stop: bool,
    start_time: Instant,
    score: Score,
    statistics: Statistics,
}

impl<B: Board, E: SearchStackEntry<B>, C: CustomInfo> ABSearchState<B, E, C> {
    fn new(max_depth: Depth) -> Self {
        Self::new_with(vec![E::default(); max_depth.get()], C::default())
    }

    fn new_with(search_stack: Vec<E>, custom: C) -> Self {
        let start_time = Instant::now();
        Self {
            search_stack,
            board_history: ZobristRepetition2Fold::default(),
            start_time,
            score: Score(0),
            best_move: None,
            searching: Stop,
            should_stop: false,
            custom,
            statistics: Statistics::default(),
        }
    }

    fn mov(&self) -> B::Move {
        self.best_move.unwrap_or_default()
    }

    fn pv(&self) -> Vec<B::Move> {
        if let Some(pv) = self.search_stack[0].pv() {
            assert!(!pv.is_empty());
            assert_eq!(pv[0], self.mov());
            Vec::from(pv)
        } else {
            vec![self.mov()]
        }
    }

    fn hashfull(&self) -> Option<usize> {
        self.custom.tt().map(|tt| tt.estimate_hashfull::<B>())
    }
    fn seldepth(&self) -> Option<usize> {
        let res = self.statistics.sel_depth();
        if res == 0 {
            None
        } else {
            Some(res)
        }
    }
    fn additional(&self) -> Option<String> {
        None
    }

    fn to_bench_res(&self) -> BenchResult {
        BenchResult {
            nodes: NodesLimit::new(self.uci_nodes()).unwrap(),
            time: self.start_time().elapsed(),
            depth: self.depth(),
        }
    }
}

impl<B: Board, E: SearchStackEntry<B>, C: CustomInfo> SearchState<B> for ABSearchState<B, E, C> {
    fn searching(&self) -> Searching {
        self.searching
    }

    fn should_stop(&self) -> bool {
        self.should_stop
    }

    fn quit(&mut self) {
        self.should_stop = true;
    }

    fn start_time(&self) -> Instant {
        self.start_time
    }

    fn score(&self) -> Score {
        self.score
    }

    fn forget(&mut self, hard: bool) {
        for e in self.search_stack.iter_mut() {
            e.forget();
        }
        if hard {
            self.custom.forget();
        } else {
            self.custom.new_search();
        }
        self.start_time = Instant::now();
        self.board_history = ZobristRepetition2Fold::default(); // will get overwritten later
        self.score = Score(0);
        self.best_move = None;
        self.searching = Stop;
        self.should_stop = false;
        self.statistics = Statistics::default();
    }

    fn new_search(&mut self, history: ZobristRepetition2Fold) {
        self.forget(false);
        self.board_history = history;
        self.searching = Ongoing;
    }

    fn end_search(&mut self) {
        self.statistics.end_search();
        self.searching = Stop;
    }

    fn to_search_info(&self) -> SearchInfo<B> {
        SearchInfo {
            best_move: self.mov(),
            depth: self.depth(),
            seldepth: self.seldepth(),
            time: self.start_time().elapsed(),
            nodes: NodesLimit::new(self.uci_nodes()).unwrap(),
            pv: self.pv(),
            score: self.score(),
            hashfull: self.hashfull(),
            additional: self.additional(),
        }
    }

    /// If the 'statistics' feature is enabled, this collects additional statistics.
    /// If not, this still keeps track of nodes, depth and seldepth, which is used for UCI output.
    #[inline(always)]
    fn statistics(&self) -> &Statistics {
        &self.statistics
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
        sum.nodes = NodesLimit::new(sum.nodes.get() + res.nodes.get()).unwrap();
        sum.time += res.time;
    }
    sum.depth = depth;
    sum
}
