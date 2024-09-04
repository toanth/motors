use std::fmt::{Debug, Display, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::hint::spin_loop;
use std::marker::PhantomData;
use std::sync::{Arc, MutexGuard};
use std::thread::spawn;
use std::time::{Duration, Instant};

use colored::Colorize;
use crossbeam_channel::unbounded;
use derive_more::{Add, Neg, Sub};
use dyn_clone::DynClone;
use gears::games::ZobristHistory;
use gears::general::board::Board;
use itertools::Itertools;
use rand::prelude::StdRng;
use rand::SeedableRng;
use strum_macros::FromRepr;

use crate::eval::rand_eval::RandEval;
use crate::eval::Eval;
use gears::general::common::{EntityList, Name, NamedEntity, Res, StaticallyNamedEntity};
use gears::general::move_list::MoveList;
use gears::output::Message;
use gears::output::Message::Warning;
use gears::score::{Score, ScoreT, MAX_BETA, MIN_ALPHA, NO_SCORE_YET, SCORE_WON};
use gears::search::{Depth, NodesLimit, SearchInfo, SearchLimit, SearchResult, TimeControl};
use gears::ugi::{EngineOption, EngineOptionName};

use crate::search::multithreading::SearchThreadType::*;
use crate::search::multithreading::SearchType::*;
use crate::search::multithreading::{
    AtomicSearchState, EngineReceives, EngineThread, SearchThreadType, Sender,
};
use crate::search::statistics::{Statistics, Summary};
use crate::search::tt::TT;
use crate::search::NodeType::{Exact, FailHigh, FailLow};
use crate::ugi_engine::UgiOutput;

#[cfg(feature = "chess")]
pub mod chess;
pub mod generic;
mod move_picker;
pub mod multithreading;
pub mod statistics;
pub(super) mod tt;

#[derive(Debug, Clone)]
#[must_use]
pub struct EngineInfo {
    engine: Name,
    eval: Option<Name>,
    version: String,
    default_bench_depth: Depth,
    default_bench_nodes: NodesLimit,
    options: Vec<EngineOption>,
    can_use_multiple_threads: bool,
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
            format!("{0}. Eval {1}", self.engine.long_name(), eval.long_name())
        } else {
            self.engine.long_name().to_string()
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
            false,
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
        can_use_multiple_threads: bool,
        options: Vec<EngineOption>,
    ) -> Self {
        Self {
            engine: Name::new(engine),
            eval: Some(Name::new(eval)),
            version: version.to_string(),
            default_bench_depth,
            default_bench_nodes,
            options,
            can_use_multiple_threads,
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

    pub fn reset_to_move(&mut self, mov: B::Move) {
        self.list[0] = mov;
        self.length = 1;
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
    fn build_in_new_thread(
        &self,
        eval: Box<dyn Eval<B>>,
    ) -> (Sender<EngineReceives<B>>, EngineInfo);

    fn build_for_bench(&self, eval_builder: &dyn AbstractEvalBuilder<B>) -> Box<dyn Benchable<B>>;

    fn can_use_multiple_threads(&self) -> bool;
}

pub type SearcherList<B> = EntityList<Box<dyn AbstractSearcherBuilder<B>>>;

#[derive(Debug)]
pub struct SearcherBuilder<B: Board, E: Engine<B>> {
    _phantom_b: PhantomData<B>,
    _phantom_e: PhantomData<E>,
}

impl<B: Board, E: Engine<B>> Default for SearcherBuilder<B, E> {
    fn default() -> Self {
        Self {
            _phantom_b: PhantomData,
            _phantom_e: PhantomData,
        }
    }
}

impl<B: Board, E: Engine<B>> Clone for SearcherBuilder<B, E> {
    fn clone(&self) -> Self {
        Self {
            _phantom_b: PhantomData,
            _phantom_e: PhantomData,
        }
    }
}

impl<B: Board, E: Engine<B>> SearcherBuilder<B, E> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<B: Board, E: Engine<B>> AbstractSearcherBuilder<B> for SearcherBuilder<B, E> {
    fn build_in_new_thread(
        &self,
        eval: Box<dyn Eval<B>>,
    ) -> (Sender<EngineReceives<B>>, EngineInfo) {
        let engine = E::with_eval(eval);
        let info = engine.engine_info();
        let (sender, receiver) = unbounded();
        let mut thread = EngineThread::new(engine, receiver);
        spawn(move || thread.main_loop());
        (sender, info)
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

pub trait Benchable<B: Board>: Debug {
    fn clean_bench(&mut self, pos: B, limit: SearchLimit) -> BenchResult;

    fn bench(&mut self, pos: B, limit: SearchLimit, tt: TT) -> BenchResult;

    fn default_bench_nodes(&self) -> NodesLimit;
    fn default_bench_depth(&self) -> Depth;

    /// Reset the engine into a fresh state, e.g. by clearing the TT and various heuristics.
    fn forget(&mut self);
}

pub trait AbstractEngine<B: Board>: StaticallyNamedEntity + Benchable<B> {
    fn max_bench_depth(&self) -> Depth;

    /// Returns information about this engine, such as the name, version and default bench depth.
    fn engine_info(&self) -> EngineInfo;

    /// Sets an option with the name 'option' to the value 'value'.
    fn set_option(&mut self, option: EngineOptionName, value: String) -> Res<()> {
        Err(format!(
            "The engine '{name}' doesn't support setting custom options, including setting '{option}' to '{value}' (Note: 'Hash' and 'Threads' may still be supported)",
            name = self.long_name()
        ))
    }
}

impl<B: Board, E: Engine<B>> Benchable<B> for E {
    fn clean_bench(&mut self, pos: B, limit: SearchLimit) -> BenchResult {
        self.forget();
        let _ = self.search_with_new_tt(pos, limit);
        self.search_state().to_bench_res()
    }

    fn bench(&mut self, pos: B, limit: SearchLimit, tt: TT) -> BenchResult {
        let _ = self.search_with_tt(pos, limit, tt);
        self.search_state().to_bench_res()
    }

    fn default_bench_nodes(&self) -> NodesLimit {
        self.engine_info().default_bench_nodes
    }

    fn default_bench_depth(&self) -> Depth {
        self.engine_info().default_bench_depth
    }

    fn forget(&mut self) {
        self.search_state_mut().forget(true);
    }
}

const DEFAULT_CHECK_TIME_INTERVAL: u64 = 2048;

pub trait Engine<B: Board>: AbstractEngine<B> + Send + 'static {
    fn set_eval(&mut self, eval: Box<dyn Eval<B>>);

    /// The simplest version of the search function, ignores history-related rules like repetitions.
    fn search_with_new_tt(&mut self, pos: B, limit: SearchLimit) -> SearchResult<B> {
        self.search_with_tt(pos, limit, TT::default())
    }

    fn search_with_tt(&mut self, pos: B, limit: SearchLimit, tt: TT) -> SearchResult<B> {
        self.search(SearchParams::new_simple(pos, limit, tt))
    }

    /// Start a new search and return the best move and score.
    /// 'parameters' contains information like the board history and allows the search to output intermediary results.
    fn search(&mut self, search_params: SearchParams<B>) -> SearchResult<B> {
        self.search_state_mut().new_search(search_params);
        let res = self.do_search();
        let search_state = self.search_state_mut();
        search_state.end_search();
        search_state.send_statistics();
        search_state.aggregate_match_statistics();
        // might block, see method. Do this as the last step so that we're not using compute after sending
        // the search result.
        self.search_state_mut().send_search_res(res);
        res
    }

    /// The important function.
    /// Should not be called directly (TODO: Rename to `search_impl`)
    fn do_search(&mut self) -> SearchResult<B>;

    fn limit(&self) -> &SearchLimit {
        &self.search_state().search_params().limit
    }

    fn time_up(&self, tc: TimeControl, hard_limit: Duration, start_time: Instant) -> bool;

    // Sensible default values, but engines may choose to check more/less frequently than every 4096 nodes
    fn should_stop(&self) -> bool {
        let state = self.search_state();
        let limit = self.limit();
        // Do the less expensive checks first to avoid querying the time in each node
        // TODO: Call `search.stopped()` below, not every node? It's an Acquire load, so potentially expensive
        // loads an atomic, so calling this function twice probably won't be optimized
        let nodes = state.internal_node_count();
        if nodes >= limit.nodes.get()
            || !state.currently_searching()
            || state.stop_command_received()
        {
            self.search_state().stop_search();
            return true;
        }
        if nodes % DEFAULT_CHECK_TIME_INTERVAL != 0 {
            return false;
        }
        if self.time_up(limit.tc, limit.fixed_time, self.search_state().start_time()) {
            self.search_state().stop_search();
            return true;
        }
        false
    }

    fn should_not_start_iteration(
        &self,
        soft_limit: Duration,
        max_depth: isize,
        mate_depth: Depth,
    ) -> bool {
        let state = self.search_state();
        state.start_time().elapsed() >= soft_limit
            || state.depth().get() as isize > max_depth
            || state.best_score() >= Score(SCORE_WON.0 - mate_depth.get() as ScoreT)
    }

    fn search_state(&self) -> &impl SearchState<B>;

    fn search_state_mut(&mut self) -> &mut impl SearchState<B>;

    /// Returns a [`SearchInfo`] object with information about the search so far.
    /// Can be called during search, only returns the information regarding the current thread.
    fn search_info(&self) -> SearchInfo<B> {
        self.search_state().to_search_info()
    }

    fn send_search_info(&mut self) {
        let msg = self.search_info().to_string();
        self.search_state_mut().send_ugi(&msg)
    }

    fn is_currently_searching(&self) -> bool {
        self.search_state().currently_searching()
    }

    fn can_use_multiple_threads() -> bool;

    fn with_eval(eval: Box<dyn Eval<B>>) -> Self;

    fn for_eval<E: Eval<B> + Default>() -> Self
    where
        Self: Sized,
    {
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
}

pub trait MoveScorer<B: Board> {
    type State: SearchState<B>;
    fn score_move(&self, mov: B::Move, state: &Self::State) -> MoveScore;
}

/// A struct bundling parameters that modify the core search.
#[derive(Debug, Default)]
pub struct SearchParams<B: Board> {
    pub pos: B,
    pub limit: SearchLimit,
    pub atomic: Arc<AtomicSearchState<B>>,
    pub history: ZobristHistory<B>,
    pub tt: TT,
    pub thread_type: SearchThreadType<B>,
    pub restrict_moves: Option<Vec<B::Move>>,
    // may be set to 0 if there are no legal moves
    pub num_multi_pv: usize,
}

impl<B: Board> SearchParams<B> {
    pub fn new_simple(pos: B, limit: SearchLimit, tt: TT) -> Self {
        Self::with_atomic_state(pos, limit, tt, Arc::new(AtomicSearchState::default()))
    }

    pub fn with_atomic_state(
        pos: B,
        limit: SearchLimit,
        tt: TT,
        atomic: Arc<AtomicSearchState<B>>,
    ) -> Self {
        Self::create(
            pos,
            limit,
            ZobristHistory::default(),
            tt,
            None,
            0,
            atomic,
            Auxiliary,
        )
    }

    pub fn create(
        pos: B,
        limit: SearchLimit,
        history: ZobristHistory<B>,
        tt: TT,
        restrict_moves: Option<Vec<B::Move>>,
        additional_pvs: usize,
        atomic: Arc<AtomicSearchState<B>>,
        thread_type: SearchThreadType<B>,
    ) -> Self {
        Self {
            pos,
            limit,
            atomic,
            history,
            tt,
            thread_type,
            restrict_moves,
            num_multi_pv: additional_pvs + 1,
        }
    }

    pub fn restrict_moves(mut self, moves: Vec<B::Move>) -> Self {
        self.restrict_moves = Some(moves);
        self
    }

    pub fn additional_pvs(mut self, num_additional: usize) -> Self {
        self.num_multi_pv = num_additional + 1;
        self
    }

    pub fn set_tt(mut self, tt: TT) -> Self {
        self.tt = tt;
        self
    }

    pub fn auxiliary(&self, atomic: Arc<AtomicSearchState<B>>) -> Self {
        // allow calling this on an auxiliary thread as well
        //assert!(matches!(self.thread_type, Main(_)));
        Self {
            pos: self.pos,
            limit: self.limit,
            atomic,
            history: self.history.clone(),
            tt: self.tt.clone(),
            thread_type: Auxiliary,
            restrict_moves: self.restrict_moves.clone(),
            num_multi_pv: self.num_multi_pv,
        }
    }
}

pub trait SearchState<B: Board>: Debug {
    fn search_params(&self) -> &SearchParams<B>;

    fn search_params_mut(&mut self) -> &mut SearchParams<B>;

    fn stop_command_received(&self) -> bool {
        self.search_params().atomic.stop_flag()
    }

    fn estimate_hashfull(&self) -> usize {
        self.tt().estimate_hashfull::<B>()
    }

    fn depth(&self) -> Depth {
        self.search_params().atomic.depth()
    }

    fn seldepth(&self) -> Depth {
        self.search_params().atomic.seldepth()
    }

    fn best_score(&self) -> Score {
        self.search_params().atomic.score()
    }

    fn best_move(&self) -> B::Move {
        self.search_params().atomic.best_move()
    }

    fn ponder_move(&self) -> Option<B::Move> {
        self.search_params().atomic.ponder_move()
    }

    fn search_result(&self) -> SearchResult<B> {
        SearchResult::new(self.best_move(), self.best_score(), self.ponder_move())
    }

    fn internal_node_count(&self) -> u64 {
        self.search_params().atomic.edges()
    }

    fn currently_searching(&self) -> bool {
        self.search_params().atomic.currently_searching()
    }

    fn stop_search(&self) {
        self.search_params().atomic.set_searching(false)
    }

    /// Returns the number of nodes looked at so far, including normal search and quiescent search.
    /// Can be called during search.
    /// For smp, this only returns the number of nodes looked at in the current thread.
    fn uci_nodes(&self) -> u64 {
        // + 1 because we only count make_move calls, which ignores the root
        self.search_params().atomic.edges() + 1
    }

    fn count_node(&self) {
        self.search_params().atomic.count_node();
    }

    fn tt(&self) -> &TT {
        &self.search_params().tt
    }

    fn tt_mut(&mut self) -> &mut TT {
        &mut self.search_params_mut().tt
    }

    fn multi_pv(&self) -> usize {
        self.search_params().num_multi_pv
    }

    fn is_main(&self) -> bool {
        matches!(self.search_params().thread_type, Main(_))
    }

    fn send_ugi(&mut self, ugi_str: &str) {
        if let Some(mut output) = self.search_params().thread_type.output() {
            output.write_ugi(ugi_str);
        }
    }

    fn send_non_ugi(&mut self, typ: Message, message: &str) {
        if let Some(mut output) = self.search_params().thread_type.output() {
            output.write_message(typ, message);
        }
    }

    /// this will block if
    /// a) this is a main thread (i.e., it actually outputs), and
    /// b) the search is an infinite search from `go infinite` but not `ponder`, and
    /// c) the search hasn't been cancelled yet. It will wait until the search has been cancelled.
    /// Auxiliary threads and ponder searches both return instantly from this function, without printing anything.
    /// If the search result has chosen a null move, this instead outputs a warning and a random legal move.
    fn send_search_res(&mut self, res: SearchResult<B>) {
        let Main(data) = &self.search_params().thread_type else {
            return;
        };
        if data.search_type == Ponder {
            return;
        }
        if data.search_type == Infinite {
            while !self.search_params().atomic.stop_flag() {
                spin_loop();
            }
        }
        let pos = self.search_params().pos;
        let mut output = data.output.lock().unwrap();
        if res.chosen_move == B::Move::default() {
            let mut rng = StdRng::seed_from_u64(42); // keep everything deterministic
            let chosen_move = pos.random_legal_move(&mut rng).unwrap_or_default();
            if chosen_move != B::Move::default() {
                debug_assert!(pos.is_move_legal(chosen_move));
                output.write_message(Warning, "Not even a single iteration finished");
                output.write_ugi(&SearchResult::<B>::move_only(chosen_move).to_string());
                return;
            }
            output.write_message(Warning, "search() called in a position with no legal moves");
        }
        debug_assert!(pos.is_move_legal(res.chosen_move) || res.chosen_move == B::Move::default());

        output.write_ugi(&res.to_string());
    }

    fn start_time(&self) -> Instant;

    fn forget(&mut self, hard: bool);

    fn new_search(&mut self, opts: SearchParams<B>);

    fn end_search(&mut self) {
        self.statistics_mut().end_search();
        self.search_params_mut().atomic.set_searching(false);
    }

    fn to_search_info(&self) -> SearchInfo<B>;

    fn statistics(&self) -> &Statistics;

    fn statistics_mut(&mut self) -> &mut Statistics;

    fn aggregate_match_statistics(&mut self);

    fn output_mut(&mut self) -> Option<MutexGuard<UgiOutput<B>>> {
        self.search_params().thread_type.output()
    }

    fn send_statistics(&mut self);

    fn pv(&self) -> Option<&[B::Move]>;

    fn to_bench_res(&self) -> BenchResult {
        let mut hasher = DefaultHasher::new();
        if let Some(pv) = self.pv() {
            for mov in pv {
                mov.hash(&mut hasher);
            }
        } else {
            self.best_move().hash(&mut hasher);
        }
        let hash = hasher.finish();
        BenchResult {
            nodes: NodesLimit::new(self.uci_nodes()).unwrap(),
            time: self.start_time().elapsed(),
            depth: self.depth(),
            moves_hash: hash,
        }
    }
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

pub trait CustomInfo<B: Board>: Default + Clone + Debug {
    fn new_search(&mut self) {
        self.hard_forget_except_tt();
    }
    fn hard_forget_except_tt(&mut self);
}

#[derive(Default, Clone, Debug)]
pub struct NoCustomInfo {}

impl<B: Board> CustomInfo<B> for NoCustomInfo {
    fn hard_forget_except_tt(&mut self) {}
}

#[derive(Debug, Copy, Clone)]
struct PVData<B: Board> {
    alpha: Score,
    beta: Score,
    radius: Score,
    best_move: B::Move,
    score: Score,
}

impl<B: Board> Default for PVData<B> {
    fn default() -> Self {
        Self {
            alpha: MIN_ALPHA,
            beta: MAX_BETA,
            radius: Score(20),
            best_move: B::Move::default(),
            score: NO_SCORE_YET,
        }
    }
}

#[derive(Debug)]
pub struct ABSearchState<B: Board, E: SearchStackEntry<B>, C: CustomInfo<B>> {
    search_stack: Vec<E>,
    params: SearchParams<B>,
    custom: C,
    excluded_moves: Vec<B::Move>,
    multi_pvs: Vec<PVData<B>>,
    current_pv_num: usize,
    start_time: Instant,
    statistics: Statistics,
    aggregated_statistics: Statistics, // statistics aggregated over all searches of the current match
}

impl<B: Board, E: SearchStackEntry<B>, C: CustomInfo<B>> ABSearchState<B, E, C> {
    fn new(max_depth: Depth) -> Self {
        Self::new_with(vec![E::default(); max_depth.get()], C::default())
    }

    fn new_with(search_stack: Vec<E>, custom: C) -> Self {
        let start_time = Instant::now();
        Self {
            search_stack,
            start_time,
            custom,
            statistics: Statistics::default(),
            aggregated_statistics: Statistics::default(),
            multi_pvs: vec![],
            params: SearchParams::new_simple(B::default(), SearchLimit::infinite(), TT::minimal()),
            excluded_moves: vec![],
            current_pv_num: 0,
        }
    }

    fn current_pv_data(&mut self) -> &mut PVData<B> {
        &mut self.multi_pvs[self.current_pv_num]
    }

    fn current_mpv_pv(&self) -> Vec<B::Move> {
        if let Some(pv) = self.search_stack[0].pv() {
            if !pv.is_empty() {
                // note that pv[0] doesn't have to be the same as self.best_move(), because the current pv may be an
                // additional multi pv
                assert_eq!(pv[0], self.multi_pvs[self.current_pv_num].best_move);
                return Vec::from(pv);
            }
        }
        vec![self.multi_pvs[self.current_pv_num].best_move]
    }

    /// Each thread has its own copy, but the main thread can access the copies of the auxiliary threads
    fn atomic(&self) -> &AtomicSearchState<B> {
        &self.params.atomic
    }

    fn additional() -> Option<String> {
        None
    }
}

impl<B: Board, E: SearchStackEntry<B>, C: CustomInfo<B>> SearchState<B> for ABSearchState<B, E, C> {
    fn search_params(&self) -> &SearchParams<B> {
        &self.params
    }

    fn search_params_mut(&mut self) -> &mut SearchParams<B> {
        &mut self.params
    }

    fn start_time(&self) -> Instant {
        self.start_time
    }

    fn forget(&mut self, hard: bool) {
        self.start_time = Instant::now();
        for e in &mut self.search_stack {
            e.forget();
        }
        if hard {
            self.custom.hard_forget_except_tt();
        } else {
            self.custom.new_search();
        }
        self.params.history = ZobristHistory::default(); // will get overwritten later
        self.statistics = Statistics::default();
        for pv in &mut self.multi_pvs {
            *pv = PVData::default();
        }
    }

    fn new_search(&mut self, mut parameters: SearchParams<B>) {
        self.forget(false);
        let moves = parameters.pos.legal_moves_slow();
        let num_moves = moves.num_moves();
        self.current_pv_num = 0;
        if let Some(search_moves) = &parameters.restrict_moves {
            // remove duplicates and invalid moves from the `restrict_move` parameter and invert the set because the usual case is
            // having no excluded moves
            self.excluded_moves = moves
                .into_iter()
                .filter(|m| !search_moves.contains(m))
                .collect_vec();
        } else {
            self.excluded_moves = vec![];
        }
        // this can set num_multi_pv to 0
        parameters.num_multi_pv = parameters
            .num_multi_pv
            .min(num_moves - self.excluded_moves.len());
        self.multi_pvs
            .resize_with(parameters.num_multi_pv, PVData::default);
        // it's possible that there are no legal moves to search; such as when the game is over or if restrict_moves
        // contains only invalid moves. Search must be able to deal with this
        debug_assert!(self.excluded_moves.len() + parameters.num_multi_pv <= num_moves);
        self.params = parameters;
        debug_assert!(self.currently_searching() && !self.stop_command_received());
    }

    fn to_search_info(&self) -> SearchInfo<B> {
        SearchInfo {
            best_move: self.best_move(),
            depth: self.depth(),
            seldepth: self.seldepth(),
            time: self.start_time().elapsed(),
            nodes: NodesLimit::new(self.uci_nodes()).unwrap(),
            pv_num: self.current_pv_num,
            pv: self.current_mpv_pv(),
            score: self.best_score(),
            hashfull: self.estimate_hashfull(),
            additional: Self::additional(),
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

    fn aggregate_match_statistics(&mut self) {
        self.aggregated_statistics
            .aggregate_searches(&self.statistics);
    }

    fn send_statistics(&mut self) {
        // don't pay the performance penalty of aggregating statistics unless they are shown,
        // especially since the "statistics" feature is likely turned off
        if cfg!(feature = "statistics") {
            self.send_non_ugi(Message::Debug, &Summary::new(self.statistics()).to_string());
        }
    }

    fn pv(&self) -> Option<&[B::Move]> {
        self.search_stack.first().and_then(|e| e.pv())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, FromRepr)]
#[repr(u8)]
#[must_use]
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

pub fn run_bench<B: Board>(engine: &mut dyn Benchable<B>, with_nodes: bool) -> BenchResult {
    let nodes = if with_nodes {
        Some(SearchLimit::nodes(engine.default_bench_nodes()))
    } else {
        None
    };
    let depth = SearchLimit::depth(engine.default_bench_depth());
    run_bench_with(engine, depth, nodes)
}

pub fn run_bench_with<B: Board>(
    engine: &mut dyn Benchable<B>,
    limit: SearchLimit,
    second_limit: Option<SearchLimit>,
) -> BenchResult {
    let mut hasher = DefaultHasher::new();
    let mut sum = BenchResult::default();
    let tt = TT::default();
    for position in B::bench_positions() {
        // engine.forget();
        single_bench(position, engine, limit, tt.clone(), &mut sum, &mut hasher);
        if let Some(limit) = second_limit {
            single_bench(position, engine, limit, tt.clone(), &mut sum, &mut hasher);
        }
    }
    sum.moves_hash = hasher.finish();
    sum
}

fn single_bench<B: Board>(
    pos: B,
    engine: &mut dyn Benchable<B>,
    limit: SearchLimit,
    tt: TT,
    sum: &mut BenchResult,
    hasher: &mut DefaultHasher,
) {
    let res = engine.bench(pos, limit, tt);
    sum.nodes = NodesLimit::new(sum.nodes.get() + res.nodes.get()).unwrap();
    sum.time += res.time;
    res.moves_hash.hash(hasher);
    sum.depth = sum.depth.max(sum.depth);
}

#[cfg(test)]
mod tests {
    use super::*;
    use gears::general::moves::Move;

    // A testcase that any engine should pass
    pub fn generic_engine_test<B: Board, E: Engine<B>>(mut engine: E) {
        let tt = TT::default();
        for p in B::bench_positions() {
            let res = engine.bench(p, SearchLimit::nodes_(1), tt.clone());
            assert!(res.depth.get() <= 1);
            assert!(res.nodes.get() <= 100); // TODO: Assert exactly 1
            let res = engine.search_with_new_tt(p, SearchLimit::depth_(1));
            assert!(p.legal_moves_slow().into_iter().contains(&res.chosen_move));
            // empty search moves, which is something the engine should handle
            let res = engine
                .search(SearchParams::for_pos(p, SearchLimit::depth_(2)).restrict_moves(vec![]));
            assert!(res.chosen_move.is_null());
            let mut search_moves = p.pseudolegal_moves().into_iter().collect_vec();
            search_moves.truncate(search_moves.len() / 2);
            search_moves.push(search_moves.first().copied().unwrap_or_default());
            search_moves.push(B::Move::default());
            let multi_pv = search_moves.len() + 3;
            let params = SearchParams::for_pos(p, SearchLimit::nodes_(1_234))
                .additional_pvs(multi_pv - 1)
                .restrict_moves(search_moves.clone());
            let res = engine.search(params);
            assert!(search_moves.contains(&res.chosen_move));
            // assert_eq!(engine.search_state().internal_node_count(), 1_234); // TODO: Assert exact match
        }
    }
}
