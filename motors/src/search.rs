use std::fmt::{Debug, Display, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;
use std::ops::Deref;
use std::time::{Duration, Instant};

use colored::Colorize;
use derive_more::{Add, Neg, Sub};
use dyn_clone::{clone_box, DynClone};
use strum_macros::FromRepr;

use crate::eval::rand_eval::RandEval;
use crate::eval::Eval;
use gears::games::{Board, ZobristHistory};
use gears::general::common::{EntityList, Name, NamedEntity, Res, StaticallyNamedEntity};
use gears::score::{Score, ScoreT, SCORE_WON};
use gears::search::{
    Depth, NodesLimit, SearchInfo, SearchLimit, SearchResult, TimeControl, MAX_DEPTH,
};
use gears::ugi::{EngineOption, EngineOptionName};

use crate::search::multithreading::{EngineWrapper, SearchSender};
use crate::search::statistics::Statistics;
use crate::search::tt::TT;
use crate::search::NodeType::{Exact, FailHigh, FailLow};
use crate::search::Searching::*;

#[cfg(feature = "chess")]
pub mod chess;
pub mod generic;
mod move_picker;
pub mod multithreading;
pub mod statistics;
mod tt;

#[derive(Debug, Clone)]
pub struct EngineInfo {
    engine: Name,
    eval: Option<Name>,
    version: String,
    default_bench_depth: Depth,
    default_bench_nodes: NodesLimit,
    options: Vec<EngineOption>,
}

impl NamedEntity for EngineInfo {
    fn short_name(&self) -> String {
        if let Some(eval) = self.eval.clone() {
            format!("{0}-{1}", self.engine.short_name(), eval.short_name())
        } else {
            self.engine.short_name()
        }
    }

    fn long_name(&self) -> String {
        if let Some(eval) = self.eval.clone() {
            format!(
                "{0} -- Version {1}. Eval {2}",
                self.engine.long_name(),
                self.version,
                eval.long_name()
            )
        } else {
            format!("{} -- Version {1}", self.engine.long_name(), self.version)
        }
    }

    fn description(&self) -> Option<String> {
        let eval = self
            .eval
            .clone()
            .map(|e| {
                format!(
                    "\nEval: {}",
                    e.clone().description.unwrap_or_else(|| e.long_name())
                )
            })
            .unwrap_or_default();
        let desc = format!(
            "Searcher: {0}{eval}",
            self.engine.description().unwrap_or(self.engine.long_name()),
        );
        Some(desc)
    }
}

impl EngineInfo {
    pub fn new_without_eval<B: Board, E: Engine<B>>(
        engine: &E,
        version: &str,
        default_bench_depth: Depth,
        default_bench_nodes: NodesLimit,
        options: Vec<EngineOption>,
    ) -> Self {
        let mut res = Self::new(
            engine,
            &RandEval::default(),
            version,
            default_bench_depth,
            default_bench_nodes,
            options,
        );
        res.eval = None;
        res
    }

    pub fn new<B: Board, E: Engine<B>>(
        engine: &E,
        eval: &dyn Eval<B>,
        version: &str,
        default_bench_depth: Depth,
        default_bench_nodes: NodesLimit,
        options: Vec<EngineOption>,
    ) -> Self {
        Self {
            engine: Name::new(engine),
            eval: Some(Name::new(eval)),
            version: version.to_string(),
            default_bench_depth,
            default_bench_nodes,
            options,
        }
    }

    pub fn engine(&self) -> &Name {
        &self.engine
    }

    pub fn eval(&self) -> &Option<Name> {
        &self.eval
    }

    pub fn default_bench_depth(&self) -> Depth {
        self.default_bench_depth
    }

    pub fn default_bench_nodes(&self) -> NodesLimit {
        self.default_bench_nodes
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    fn set_eval<B: Board>(&mut self, eval: &dyn Eval<B>) {
        self.eval = Some(Name::new(eval))
    }
}

#[derive(Debug)]
pub struct BenchResult {
    pub nodes: NodesLimit,
    pub time: Duration,
    pub depth: Depth,
    pub moves_hash: u64,
}

impl Default for BenchResult {
    fn default() -> Self {
        Self {
            nodes: NodesLimit::MIN,
            time: Duration::default(),
            depth: Depth::MIN,
            moves_hash: 0,
        }
    }
}

impl Display for BenchResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "depth {0}, time {2} ms, {1} nodes, {3} nps, hash {4:X}",
            self.depth.get().to_string().bold(),
            self.nodes.to_string().bold(),
            self.time.as_millis().to_string().red(),
            (self.nodes.get() as f64 / self.time.as_millis() as f64 * 1000.0)
                .round()
                .to_string()
                .red(),
            self.moves_hash,
        )
    }
}

// TODO: Use ArrayVec
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
    pub fn extend(&mut self, ply: usize, mov: B::Move, child_pv: &Pv<B, LIMIT>) {
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

pub trait AbstractEvalBuilder<B: Board>: NamedEntity + DynClone {
    fn build(&self) -> Box<dyn Eval<B>>;
}

#[derive(Debug, Default)]
pub struct EvalBuilder<B: Board, E: Eval<B> + Default> {
    _phantom_board: PhantomData<B>,
    _phantom_eval: PhantomData<E>,
}

impl<B: Board, E: Eval<B> + Default> Clone for EvalBuilder<B, E> {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl<B: Board, E: Eval<B> + Default> StaticallyNamedEntity for EvalBuilder<B, E> {
    fn static_short_name() -> impl Display
    where
        Self: Sized,
    {
        E::static_short_name()
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        E::static_long_name()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        E::static_description()
    }
}

impl<B: Board, E: Eval<B> + Default> AbstractEvalBuilder<B> for EvalBuilder<B, E> {
    fn build(&self) -> Box<dyn Eval<B>> {
        Box::new(E::default())
    }
}

pub type EvalList<B> = EntityList<Box<dyn AbstractEvalBuilder<B>>>;

/// A trait because this type erases over the Engine being built.
/// There are two related concepts: `Engine` and `Searcher`.
/// A searcher is an algorithm like caps or gaps, an engine is a searcher plus an eval.
pub trait AbstractSearcherBuilder<B: Board>: NamedEntity + DynClone {
    fn build(&self, engine_builder: EngineBuilder<B>, tt: TT) -> EngineWrapper<B>;

    fn build_for_bench(&self, eval_builder: &dyn AbstractEvalBuilder<B>) -> Box<dyn Benchable<B>>;

    fn can_use_multiple_threads(&self) -> bool;
}

#[derive(Debug)]
pub struct EngineBuilder<B: Board> {
    search_builder: Box<dyn AbstractSearcherBuilder<B>>,
    eval_builder: Box<dyn AbstractEvalBuilder<B>>,
    sender: SearchSender<B>,
}

impl<B: Board> Clone for EngineBuilder<B> {
    fn clone(&self) -> Self {
        Self {
            search_builder: clone_box(self.search_builder.deref()),
            eval_builder: clone_box(self.eval_builder.deref()),
            sender: self.sender.clone(),
        }
    }
}

impl<B: Board> EngineBuilder<B> {
    pub fn new(
        search_builder: Box<dyn AbstractSearcherBuilder<B>>,
        eval_builder: Box<dyn AbstractEvalBuilder<B>>,
        sender: SearchSender<B>,
    ) -> Self {
        Self {
            search_builder,
            eval_builder,
            sender,
        }
    }

    pub fn build_wrapper(&self) -> EngineWrapper<B> {
        let cloned = self.clone();
        self.search_builder.build(cloned, TT::default())
    }
}

pub type SearcherList<B> = EntityList<Box<dyn AbstractSearcherBuilder<B>>>;

#[derive(Debug, Default)]
pub struct SearcherBuilder<B: Board, E: Engine<B> + Default> {
    _phantom_b: PhantomData<B>,
    _phantom_e: PhantomData<E>,
}

impl<B: Board, E: Engine<B>> Clone for SearcherBuilder<B, E> {
    fn clone(&self) -> Self {
        Self {
            _phantom_b: PhantomData::default(),
            _phantom_e: PhantomData::default(),
        }
    }
}

impl<B: Board, E: Engine<B>> SearcherBuilder<B, E> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<B: Board, E: Engine<B>> AbstractSearcherBuilder<B> for SearcherBuilder<B, E> {
    fn build(&self, engine_builder: EngineBuilder<B>, tt: TT) -> EngineWrapper<B> {
        EngineWrapper::new_with_tt(
            E::with_eval(engine_builder.eval_builder.build()),
            engine_builder,
            tt,
        )
    }

    fn build_for_bench(&self, eval_builder: &dyn AbstractEvalBuilder<B>) -> Box<dyn Benchable<B>> {
        Box::new(E::with_eval(eval_builder.build()))
    }

    fn can_use_multiple_threads(&self) -> bool {
        E::can_use_multiple_threads()
    }
}

impl<B: Board, E: Engine<B>> StaticallyNamedEntity for SearcherBuilder<B, E> {
    fn static_short_name() -> impl Display {
        E::static_short_name()
    }

    fn static_long_name() -> String {
        E::static_long_name()
    }

    fn static_description() -> String {
        E::static_description()
    }
}

pub enum BenchLimit {
    Depth(Depth),
    Nodes(NodesLimit),
}

impl BenchLimit {
    pub fn to_search_limit(self, depth_limit: Depth) -> SearchLimit {
        let mut res = SearchLimit::infinite();
        match self {
            BenchLimit::Depth(depth) => res.depth = depth.min(depth_limit),
            BenchLimit::Nodes(nodes) => res.nodes = nodes,
        }
        return res;
    }
}

pub trait Benchable<B: Board>: StaticallyNamedEntity + Debug {
    // TODO: Default implementation that calls serch_from_pos
    fn bench(&mut self, position: B, limit: BenchLimit) -> BenchResult;

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

const DEFAULT_CHECK_TIME_INTERVAL: u64 = 2048;

pub trait Engine<B: Board>: Benchable<B> + Default + Send + 'static {
    fn set_tt(&mut self, tt: TT);

    fn set_eval(&mut self, eval: Box<dyn Eval<B>>);

    fn search_from_pos(&mut self, pos: B, limit: SearchLimit) -> Res<SearchResult<B>> {
        self.search(
            pos,
            limit,
            ZobristHistory::default(),
            SearchSender::no_sender(),
        )
    }

    fn search_moves<I: ExactSizeIterator<Item = B::Move>>(
        &mut self,
        moves: I,
        pos: B,
        limit: SearchLimit,
        history: ZobristHistory<B>,
        sender: SearchSender<B>,
    ) -> Res<SearchResult<B>> {
        self.search_state_mut().new_search(history, sender);
        let res = self.do_search(pos, moves, limit);
        let search_state = self.search_state_mut();
        search_state.end_search();
        search_state.send_statistics();
        search_state.aggregate_match_statistics();
        res
    }

    fn search(
        &mut self,
        pos: B,
        limit: SearchLimit,
        history: ZobristHistory<B>,
        sender: SearchSender<B>,
    ) -> Res<SearchResult<B>> {
        self.search_moves([].into_iter(), pos, limit, history, sender)
    }

    /// The important function.
    fn do_search<I: ExactSizeIterator<Item = B::Move>>(
        &mut self,
        pos: B,
        moves: I,
        limit: SearchLimit,
    ) -> Res<SearchResult<B>>;

    fn time_up(&self, tc: TimeControl, hard_limit: Duration, start_time: Instant) -> bool;

    // Sensible default values, but engines may choose to check more/less frequently than every 4096 nodes
    fn should_stop_impl(&self, limit: SearchLimit) -> bool {
        let state = self.search_state();
        // Do the less expensive checks first to avoid querying the time in each node
        if state.main_search_nodes() >= limit.nodes.get() || state.search_cancelled() {
            return true;
        }
        if state.main_search_nodes() % DEFAULT_CHECK_TIME_INTERVAL != 0 {
            return false;
        }
        self.time_up(limit.tc, limit.fixed_time, self.search_state().start_time())
            || self.search_state().search_sender().should_stop()
    }

    #[inline(always)]
    fn should_stop(&mut self, limit: SearchLimit) -> bool {
        if self.should_stop_impl(limit) {
            self.search_state_mut().mark_search_should_end();
            true
        } else {
            false
        }
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
            || state.score() >= Score(SCORE_WON.0 - mate_depth.get() as ScoreT)
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

    fn with_eval(eval: Box<dyn Eval<B>>) -> Self;

    fn for_eval<E: Eval<B> + Default>() -> Self {
        Self::with_eval(Box::new(E::default()))
    }

    /// This should return the static eval (possibly with WDL normalization) without doing any kind of search.
    /// For engines like `RandomMover` where there is no static eval, this should return `Score(0)`.
    fn static_eval(&mut self, pos: B) -> Score;
}

#[derive(Debug, Default, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Add, Sub, Neg)]
pub struct MoveScore(pub i32);

impl MoveScore {
    const MAX: MoveScore = MoveScore(i32::MAX);
    const MIN: MoveScore = MoveScore(i32::MIN + 1);
    const IGNORE_MOVE: MoveScore = MoveScore(i32::MIN);
}

pub trait MoveScorer<B: Board> {
    type State: SearchState<B>;
    fn score_move(&self, mov: B::Move, state: &Self::State) -> MoveScore;
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
    fn new_search(&mut self, history: ZobristHistory<B>, sender: SearchSender<B>);
    fn end_search(&mut self) {
        self.mark_search_should_end();
        self.statistics_mut().end_search();
    }
    fn mark_search_should_end(&mut self);
    fn to_search_info(&self) -> SearchInfo<B>;
    fn statistics(&self) -> &Statistics;
    fn statistics_mut(&mut self) -> &mut Statistics;
    fn aggregate_match_statistics(&mut self);
    fn search_sender(&self) -> &SearchSender<B>;
    fn search_sender_mut(&mut self) -> &mut SearchSender<B>;
    fn send_statistics(&mut self);
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

pub trait CustomInfo: Default + Clone + Debug {
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
pub struct BestMoveCustomInfo<B: Board> {
    pub chosen_move: Option<B::Move>,
}

impl<B: Board> CustomInfo for BestMoveCustomInfo<B> {}

#[derive(Debug, Clone)]
pub struct ABSearchState<B: Board, E: SearchStackEntry<B>, C: CustomInfo> {
    search_stack: Vec<E>,
    board_history: ZobristHistory<B>,
    search_moves: Vec<B::Move>,
    custom: C,
    searching: Searching,
    should_stop: bool,
    start_time: Instant,
    score: Score,
    statistics: Statistics,
    aggregated_statistics: Statistics, // statistics aggregated over all searches of the current match
    sender: SearchSender<B>,
}

impl<B: Board, E: SearchStackEntry<B>, C: CustomInfo> ABSearchState<B, E, C> {
    fn new(max_depth: Depth) -> Self {
        Self::new_with(vec![E::default(); max_depth.get()], C::default())
    }

    fn new_with(search_stack: Vec<E>, custom: C) -> Self {
        let start_time = Instant::now();
        Self {
            search_stack,
            board_history: ZobristHistory::default(),
            start_time,
            score: Score(0),
            searching: Stop,
            should_stop: false,
            custom,
            statistics: Statistics::default(),
            aggregated_statistics: Default::default(),
            sender: SearchSender::no_sender(),
            search_moves: vec![],
        }
    }

    fn mov(&self) -> B::Move {
        self.search_stack
            .first()
            .and_then(|e| e.pv())
            .and_then(|pv| pv.first())
            .copied()
            .unwrap_or_default()
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
        let mut hasher = DefaultHasher::new();
        self.mov().hash(&mut hasher);
        let hash = hasher.finish();
        BenchResult {
            nodes: NodesLimit::new(self.uci_nodes()).unwrap(),
            time: self.start_time().elapsed(),
            depth: self.depth(),
            moves_hash: hash,
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
        self.sender = SearchSender::no_sender();
        self.start_time = Instant::now();
        self.board_history = ZobristHistory::default(); // will get overwritten later
        self.score = Score(0);
        self.searching = Stop;
        self.should_stop = false;
        self.statistics = Statistics::default();
    }

    fn new_search(&mut self, history: ZobristHistory<B>, sender: SearchSender<B>) {
        self.forget(false);
        self.board_history = history;
        self.sender = sender;
        self.searching = Ongoing;
    }

    fn mark_search_should_end(&mut self) {
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

    #[inline(always)]
    fn statistics_mut(&mut self) -> &mut Statistics {
        &mut self.statistics
    }

    #[inline(always)]
    fn aggregate_match_statistics(&mut self) {
        self.aggregated_statistics
            .aggregate_searches(&self.statistics);
    }

    fn search_sender(&self) -> &SearchSender<B> {
        &self.sender
    }

    fn search_sender_mut(&mut self) -> &mut SearchSender<B> {
        &mut self.sender
    }

    fn send_statistics(&mut self) {
        self.sender.send_statistics(&self.statistics);
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, FromRepr)]
#[repr(u8)]
pub enum NodeType {
    /// Don't use 0 because that's used to represent the empty node type for the internal TT representation
    /// score is a lower bound >= beta, cut-node (the most common node type)
    FailHigh = 1,
    /// score known exactly in `(alpha, beta)`, PV node (very rare, but those are the most important nodes)
    Exact = 2,
    /// score between alpha and beta, PV node (important node!)
    FailLow = 3, // score is an upper bound <= alpha, all-node (relatively rare, but makes parent a cut-node)
}

impl NodeType {
    pub fn inverse(self) -> Self {
        // Could maybe try some bit twiddling tricks in case the compiler doesn't already do that
        match self {
            FailHigh => FailLow,
            Exact => Exact,
            FailLow => FailHigh,
        }
    }
}

pub fn run_bench<B: Board>(engine: &mut dyn Benchable<B>) -> BenchResult {
    let depth = engine.engine_info().default_bench_depth;
    let nodes = engine.engine_info().default_bench_nodes;
    run_bench_with_depth_and_nodes(engine, depth, nodes)
}

pub fn run_bench_with_depth_and_nodes<B: Board>(
    engine: &mut dyn Benchable<B>,
    mut depth: Depth,
    mut nodes: NodesLimit,
) -> BenchResult {
    if depth.get() == 0 || depth == MAX_DEPTH {
        depth = engine.engine_info().default_bench_depth
    }
    if nodes == NodesLimit::MAX {
        nodes = engine.engine_info().default_bench_nodes;
        assert!(nodes <= NodesLimit::new(10_000_000).unwrap());
    }
    let mut hasher = DefaultHasher::new();
    let mut sum = BenchResult::default();
    for position in B::bench_positions() {
        let res = engine.bench(position, BenchLimit::Depth(depth));
        sum.nodes = NodesLimit::new(sum.nodes.get() + res.nodes.get()).unwrap();
        sum.time += res.time;
        res.moves_hash.hash(&mut hasher);
        let res = engine.bench(position, BenchLimit::Nodes(nodes));
        sum.nodes = NodesLimit::new(sum.nodes.get() + res.nodes.get()).unwrap();
        sum.time += res.time;
        res.moves_hash.hash(&mut hasher);
    }
    sum.depth = depth;
    sum.moves_hash = hasher.finish();
    sum
}
