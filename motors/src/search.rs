use crate::eval::Eval;
use crate::search::multithreading::SearchThreadType::*;
use crate::search::multithreading::SearchType::*;
use crate::search::multithreading::{AtomicSearchState, EngineReceives, EngineThread, SearchThreadType, Sender};
use crate::search::statistics::{Statistics, Summary};
use crate::search::tt::TT;
use crossbeam_channel::unbounded;
use derive_more::{Add, Neg, Sub};
use gears::arrayvec::ArrayVec;
use gears::colored::Color::Red;
use gears::colored::Colorize;
use gears::dyn_clone::DynClone;
use gears::games::ZobristHistory;
use gears::general::board::Board;
use gears::general::common::anyhow::bail;
use gears::general::common::{EntityList, Name, NamedEntity, Res, StaticallyNamedEntity};
use gears::general::move_list::MoveList;
use gears::general::moves::Move;
use gears::itertools::Itertools;
use gears::output::Message;
use gears::output::Message::Warning;
use gears::rand::prelude::StdRng;
use gears::rand::SeedableRng;
use gears::score::{Score, ScoreT, MAX_BETA, MIN_ALPHA, NO_SCORE_YET, SCORE_WON};
use gears::search::{Depth, NodeType, NodesLimit, SearchInfo, SearchLimit, SearchResult, TimeControl};
use gears::ugi::{EngineOption, EngineOptionName, EngineOptionType};
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::hint::spin_loop;
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::Ordering::Acquire;
use std::sync::Arc;
use std::thread::spawn;
use std::time::{Duration, Instant};

#[cfg(feature = "chess")]
pub mod chess;
pub mod generic;
mod move_picker;
pub mod multithreading;
pub(crate) mod spsa_param;
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
    options: HashMap<EngineOptionName, EngineOptionType>,
    max_threads: usize,
    pub internal_state_description: Option<String>,
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
            .map(|e| format!("\nEval: {}", e.clone().description.unwrap_or_else(|| e.long_name())))
            .unwrap_or_default();
        let desc = format!("Searcher: {0}{eval}", self.engine.description().unwrap_or(self.engine.long_name()),);
        Some(desc)
    }
}

impl EngineInfo {
    pub fn new<B: Board, E: Engine<B>>(
        engine: &E,
        eval: &dyn Eval<B>,
        version: &str,
        default_bench_depth: Depth,
        default_bench_nodes: NodesLimit,
        max_threads: Option<usize>,
        options: Vec<EngineOption>,
    ) -> Self {
        let num_cores = std::thread::available_parallelism().unwrap_or(NonZeroUsize::new(1024).unwrap());
        let max_threads = max_threads.unwrap_or(usize::MAX).min(num_cores.get());
        let options = HashMap::from_iter(options.into_iter().map(|o| (o.name, o.value)));
        Self {
            engine: Name::new(engine),
            eval: Some(Name::new(eval)),
            version: version.to_string(),
            default_bench_depth,
            default_bench_nodes,
            max_threads,
            options,
            internal_state_description: None,
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

    pub fn max_threads(&self) -> usize {
        self.max_threads
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn additional_options(&self) -> Vec<EngineOption> {
        self.options.iter().map(|(k, v)| EngineOption { name: k.clone(), value: v.clone() }).collect()
    }
}

#[derive(Debug)]
pub struct BenchResult {
    pub nodes: u64,
    pub time: Duration,
    pub max_depth: Depth,
    pub depth: Option<Depth>,
    pub pv_score_hash: u64,
}

impl Default for BenchResult {
    fn default() -> Self {
        Self { nodes: 0, time: Duration::default(), depth: None, max_depth: Depth::new(0), pv_score_hash: 0 }
    }
}

impl Display for BenchResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Uses colored instead of crossterm because that's necessary for OpenBench to parse the output
        let depth = if let Some(depth) = self.depth { format!("depth {depth}, ") } else { String::new() };
        writeln!(
            f,
            "{depth}max depth {0}, time {2} ms, {1} nodes, {3} nps, hash {4:X}",
            self.max_depth.get(),
            Colorize::bold(self.nodes.to_string().as_str()),
            self.time.as_millis().to_string().color(Red),
            (self.nodes as f64 / self.time.as_millis() as f64 * 1000.0).round().to_string().color(Red),
            self.pv_score_hash,
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Pv<B: Board, const LIMIT: usize> {
    list: ArrayVec<B::Move, LIMIT>,
}

impl<B: Board, const LIMIT: usize> Default for Pv<B, LIMIT> {
    fn default() -> Self {
        Self { list: ArrayVec::new() }
    }
}

impl<B: Board, const LIMIT: usize> Pv<B, LIMIT> {
    pub fn extend(&mut self, mov: B::Move, child_pv: &Pv<B, LIMIT>) {
        self.reset_to_move(mov);
        self.list.try_extend_from_slice(child_pv.list.as_slice()).unwrap();
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn clear(&mut self) {
        self.list.clear();
    }

    pub fn reset_to_move(&mut self, mov: B::Move) {
        self.list.clear();
        self.list.push(mov);
    }

    fn assign_from<const OTHER_LIMIT: usize>(&mut self, other: &Pv<B, OTHER_LIMIT>) {
        self.list.clear();
        self.list.try_extend_from_slice(other.list.as_slice()).unwrap();
    }

    fn get(&self, idx: usize) -> Option<B::Move> {
        self.list.get(idx).copied()
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
    fn build_in_new_thread(&self, eval: Box<dyn Eval<B>>) -> (Sender<EngineReceives<B>>, EngineInfo);

    fn build(&self, eval_builder: &dyn AbstractEvalBuilder<B>) -> Box<dyn Engine<B>>;
}

pub type SearcherList<B> = EntityList<Box<dyn AbstractSearcherBuilder<B>>>;

#[derive(Debug)]
pub struct SearcherBuilder<B: Board, E: Engine<B>> {
    _phantom_b: PhantomData<B>,
    _phantom_e: PhantomData<E>,
}

impl<B: Board, E: Engine<B>> Default for SearcherBuilder<B, E> {
    fn default() -> Self {
        Self { _phantom_b: PhantomData, _phantom_e: PhantomData }
    }
}

impl<B: Board, E: Engine<B>> Clone for SearcherBuilder<B, E> {
    fn clone(&self) -> Self {
        Self { _phantom_b: PhantomData, _phantom_e: PhantomData }
    }
}

impl<B: Board, E: Engine<B>> SearcherBuilder<B, E> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<B: Board, E: Engine<B>> AbstractSearcherBuilder<B> for SearcherBuilder<B, E> {
    fn build_in_new_thread(&self, eval: Box<dyn Eval<B>>) -> (Sender<EngineReceives<B>>, EngineInfo) {
        let engine = E::with_eval(eval);
        let info = engine.engine_info();
        let (sender, receiver) = unbounded();
        let mut thread = EngineThread::new(engine, receiver);
        _ = spawn(move || thread.main_loop());
        (sender, info)
    }

    fn build(&self, eval_builder: &dyn AbstractEvalBuilder<B>) -> Box<dyn Engine<B>> {
        Box::new(E::with_eval(eval_builder.build()))
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

pub trait Engine<B: Board>: StaticallyNamedEntity + Send + 'static {
    type SearchStackEntry: SearchStackEntry<B>
    where
        Self: Sized;
    type CustomInfo: CustomInfo<B>
    where
        Self: Sized;

    fn with_eval(eval: Box<dyn Eval<B>>) -> Self
    where
        Self: Sized;

    fn for_eval<E: Eval<B> + Default>() -> Self
    where
        Self: Sized,
    {
        Self::with_eval(Box::new(E::default()))
    }

    /// This should return the static eval (possibly with WDL normalization) without doing any kind of search.
    /// For engines like `RandomMover` where there is no static eval, this should return `Score(0)`.
    /// Most evals completely ignore the `ply` parameter, but it can be used e.g. to decide which color we are
    /// for asymmetric evals.
    fn static_eval(&mut self, pos: B, ply: usize) -> Score;

    fn clean_bench(&mut self, pos: B, limit: SearchLimit) -> BenchResult {
        self.forget();
        let _ = self.search_with_new_tt(pos, limit);
        self.search_state_dyn().to_bench_res()
    }

    fn bench(&mut self, pos: B, limit: SearchLimit, tt: TT) -> BenchResult {
        let _ = self.search_with_tt(pos, limit, tt);
        self.search_state_dyn().to_bench_res()
    }

    fn default_bench_nodes(&self) -> NodesLimit {
        self.engine_info().default_bench_nodes
    }

    fn default_bench_depth(&self) -> Depth {
        self.engine_info().default_bench_depth
    }

    fn max_bench_depth(&self) -> Depth;

    fn search_state_dyn(&self) -> &dyn AbstractSearchState<B>;

    fn search_state_mut_dyn(&mut self) -> &mut dyn AbstractSearchState<B>;

    fn search_state(&self) -> &SearchStateFor<B, Self>
    where
        Self: Sized;

    fn search_state_mut(&mut self) -> &mut SearchStateFor<B, Self>
    where
        Self: Sized;

    /// Reset the engine into a fresh state, e.g. by clearing the TT and various heuristics.
    fn forget(&mut self) {
        self.search_state_mut_dyn().forget(true);
    }
    /// Returns information about this engine, such as the name, version and default bench depth.
    fn engine_info(&self) -> EngineInfo;

    fn limit(&self) -> &SearchLimit
    where
        Self: Sized,
    {
        &self.search_state().search_params().limit
    }

    fn time_up(&self, tc: TimeControl, hard_limit: Duration, start_time: Instant) -> bool;

    // Sensible default values, but engines may choose to check more/less frequently than every 4096 nodes
    fn should_stop(&self) -> bool
    where
        Self: Sized,
    {
        let state = self.search_state();
        let limit = self.limit();
        // Do the less expensive checks first to avoid querying the time in each node
        // loads an atomic, so calling this function twice probably won't be optimized
        let nodes = state.uci_nodes();
        if nodes >= limit.nodes.get() || state.stop_flag() {
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

    fn should_not_start_negamax(&self, soft_limit: Duration, max_soft_depth: isize, mate_depth: Depth) -> bool
    where
        Self: Sized,
    {
        let state = self.search_state();
        state.start_time().elapsed() >= soft_limit
            || state.depth().get() as isize > max_soft_depth
            || state.best_score() >= Score(SCORE_WON.0 - mate_depth.get() as ScoreT)
    }

    /// Returns a [`SearchInfo`] object with information about the search so far.
    /// Can be called during search, only returns the information regarding the current thread.
    fn search_info(&self) -> SearchInfo<B> {
        self.search_state_dyn().to_search_info()
    }

    /// Sets an option with the name 'option' to the value 'value'.
    fn set_option(&mut self, option: EngineOptionName, old_value: &mut EngineOptionType, value: String) -> Res<()> {
        bail!(
            "The searcher '{name}' doesn't support setting custom options, including setting '{option}' to '{value}' \
            (Note: Some options, like 'Hash' and 'Threads', may still be supported but aren't handled by the searcher). \
            The current value of this option is '{old_value}.",
            name = self.long_name()
        )
    }

    fn print_spsa_params(&self) {
        /*do nothing*/
    }

    fn set_eval(&mut self, eval: Box<dyn Eval<B>>);

    /// The simplest version of the search function, ignores history-related rules like repetitions of positions that happened before
    /// starting the search.
    fn search_with_new_tt(&mut self, pos: B, limit: SearchLimit) -> SearchResult<B> {
        self.search_with_tt(pos, limit, TT::default())
    }

    fn search_with_tt(&mut self, pos: B, limit: SearchLimit, tt: TT) -> SearchResult<B> {
        self.search(SearchParams::new_unshared(pos, limit, ZobristHistory::default(), tt))
    }

    /// Start a new search and return the best move and score.
    /// 'parameters' contains information like the board history and allows the search to output intermediary results.
    fn search(&mut self, search_params: SearchParams<B>) -> SearchResult<B> {
        self.search_state_mut_dyn().new_search(search_params);
        let res = self.do_search();
        self.search_state_mut_dyn().end_search(&res);
        res
    }

    /// The important function.
    /// Should not be called directly (TODO: Rename to `search_impl`)
    fn do_search(&mut self) -> SearchResult<B>;
}

const DEFAULT_CHECK_TIME_INTERVAL: u64 = 2048;

#[allow(type_alias_bounds)]
pub type SearchStateFor<B: Board, E: Engine<B>> = SearchState<B, E::SearchStackEntry, E::CustomInfo>;

#[derive(Debug, Default, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Add, Sub, Neg)]
#[must_use]
pub struct MoveScore(pub i32);

impl MoveScore {
    const MAX: MoveScore = MoveScore(i32::MAX);
    const MIN: MoveScore = MoveScore(i32::MIN + 1);
}

pub trait MoveScorer<B: Board, E: Engine<B>>: Debug {
    fn score_move(&self, mov: B::Move, state: &SearchStateFor<B, E>) -> MoveScore;
}

/// A struct bundling parameters that modify the core search.
#[derive(Debug, Default)]
pub struct SearchParams<B: Board> {
    pub pos: B,
    pub limit: SearchLimit,
    pub atomic: Arc<AtomicSearchState<B>>,
    pub history: ZobristHistory,
    pub tt: TT,
    pub thread_type: SearchThreadType<B>,
    pub restrict_moves: Option<Vec<B::Move>>,
    // may be set to 0 if there are no legal moves
    pub num_multi_pv: usize,
}

impl<B: Board> SearchParams<B> {
    pub fn for_pos(pos: B, limit: SearchLimit) -> Self {
        Self::new_unshared(pos, limit, ZobristHistory::default(), TT::default())
    }

    pub fn new_unshared(pos: B, limit: SearchLimit, history: ZobristHistory, tt: TT) -> Self {
        Self::with_atomic_state(pos, limit, history, tt, Arc::new(AtomicSearchState::default()))
    }

    pub fn with_atomic_state(
        pos: B,
        limit: SearchLimit,
        history: ZobristHistory,
        tt: TT,
        atomic: Arc<AtomicSearchState<B>>,
    ) -> Self {
        Self::create(pos, limit, history, tt, None, 0, atomic, Auxiliary)
    }

    #[expect(clippy::too_many_arguments)]
    pub fn create(
        pos: B,
        limit: SearchLimit,
        history: ZobristHistory,
        tt: TT,
        restrict_moves: Option<Vec<B::Move>>,
        additional_pvs: usize,
        atomic: Arc<AtomicSearchState<B>>,
        thread_type: SearchThreadType<B>,
    ) -> Self {
        Self { pos, limit, atomic, history, tt, thread_type, restrict_moves, num_multi_pv: additional_pvs + 1 }
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
            pos: self.pos.clone(),
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

pub trait SearchStackEntry<B: Board>: Default + Clone + Debug {
    fn forget(&mut self) {
        *self = Self::default();
    }
    fn pv(&self) -> Option<&[B::Move]>;
    fn last_played_move(&self) -> Option<B::Move>;
}

#[derive(Copy, Clone, Default, Debug)]
pub struct EmptySearchStackEntry {}

impl<B: Board> SearchStackEntry<B> for EmptySearchStackEntry {
    fn pv(&self) -> Option<&[B::Move]> {
        None
    }

    fn last_played_move(&self) -> Option<B::Move> {
        None
    }
}

pub trait CustomInfo<B: Board>: Default + Clone + Debug {
    fn new_search(&mut self) {
        self.hard_forget_except_tt();
    }
    fn hard_forget_except_tt(&mut self);

    fn write_internal_info(&self) -> Option<String> {
        None
    }
}

#[derive(Default, Clone, Debug)]
pub struct NoCustomInfo {}

impl<B: Board> CustomInfo<B> for NoCustomInfo {
    fn hard_forget_except_tt(&mut self) {}
}

#[derive(Debug, Clone)]
struct PVData<B: Board> {
    alpha: Score,
    beta: Score,
    radius: Score,
    pv: Pv<B, 200>, // A PV of 200 plies should be more than enough for anybody (tm)
    score: Score,
    bound: Option<NodeType>,
}

impl<B: Board> Default for PVData<B> {
    fn default() -> Self {
        Self {
            alpha: MIN_ALPHA,
            beta: MAX_BETA,
            radius: Score(20),
            pv: Pv::default(),
            score: NO_SCORE_YET,
            bound: None,
        }
    }
}

pub trait AbstractSearchState<B: Board> {
    fn forget(&mut self, hard: bool);
    fn new_search(&mut self, params: SearchParams<B>);
    fn end_search(&mut self, res: &SearchResult<B>);
    fn search_params(&self) -> &SearchParams<B>;
    fn to_bench_res(&self) -> BenchResult;
    fn to_search_info(&self) -> SearchInfo<B>;
    fn aggregated_statistics(&self) -> &Statistics;
    fn send_search_info(&self);
    /// Engine-specific info, like the contents of history tables.
    fn write_internal_info(&self) -> Option<String>;
}

#[derive(Debug)]
pub struct SearchState<B: Board, E: SearchStackEntry<B>, C: CustomInfo<B>> {
    search_stack: Vec<E>,
    params: SearchParams<B>,
    custom: C,
    excluded_moves: Vec<B::Move>,
    multi_pvs: Vec<PVData<B>>,
    current_pv_num: usize,
    start_time: Instant,
    last_msg_time: Instant,
    statistics: Statistics,
    aggregated_statistics: Statistics, // statistics aggregated over all searches of the current match
}

impl<B: Board, E: SearchStackEntry<B>, C: CustomInfo<B>> Deref for SearchState<B, E, C> {
    type Target = C;
    fn deref(&self) -> &Self::Target {
        &self.custom
    }
}

impl<B: Board, E: SearchStackEntry<B>, C: CustomInfo<B>> DerefMut for SearchState<B, E, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.custom
    }
}

impl<B: Board, E: SearchStackEntry<B>, C: CustomInfo<B>> AbstractSearchState<B> for SearchState<B, E, C> {
    fn forget(&mut self, hard: bool) {
        self.start_time = Instant::now();
        self.last_msg_time = self.start_time;
        for e in &mut self.search_stack {
            e.forget();
        }
        if hard {
            self.custom.hard_forget_except_tt();
            self.params.atomic.reset(false);
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
        parameters.atomic.set_searching(true);
        self.forget(false);
        let moves = parameters.pos.legal_moves_slow();
        let num_moves = moves.num_moves();
        self.current_pv_num = 0;
        if let Some(search_moves) = &parameters.restrict_moves {
            // remove duplicates and invalid moves from the `restrict_move` parameter and invert the set because the usual case is
            // having no excluded moves
            self.excluded_moves = moves.into_iter().filter(|m| !search_moves.contains(m)).collect_vec();
        } else {
            self.excluded_moves = vec![];
        }
        let num_moves = num_moves - self.excluded_moves.len();
        // this can set num_multi_pv to 0
        parameters.num_multi_pv = parameters.num_multi_pv.min(num_moves);
        // it's possible that there are no legal moves to search; such as when the game is over or if restrict_moves
        // contains only invalid moves. Search must be able to deal with this, but we set still add an empty multipv entry
        self.multi_pvs.resize_with(parameters.num_multi_pv.max(1), PVData::default);
        // If only one move can be played, immediately return it without doing a real search to make the engine appear
        // smarter, and perform better on lichess when it's up against an opponent with pondering enabled.
        // However, don't do this if the engine is used for analysis.
        if num_moves == 1 && parameters.limit.is_only_time_based() {
            parameters.limit.depth = Depth::new(1);
        }
        self.params = parameters;
        // it's possible that a stop command has already been received and handled, which means the stop flag
        // can already be set
    }

    fn end_search(&mut self, res: &SearchResult<B>) {
        self.statistics_mut().end_search();
        self.send_statistics();
        self.aggregate_match_statistics();
        // might block, see method. Do this as the last step so that we're not using compute after sending
        // the search result.
        self.send_search_res(res);
        self.search_params_mut().atomic.set_searching(false);
    }

    fn search_params(&self) -> &SearchParams<B> {
        &self.params
    }

    fn to_bench_res(&self) -> BenchResult {
        let mut hasher = DefaultHasher::new();
        if self.params.num_multi_pv > 0 {
            for mov in self.current_mpv_pv() {
                mov.hash(&mut hasher);
            }
        }
        // the score can differ even if the pv is the same, so make sure to include that in the hash
        self.best_score().hash(&mut hasher);
        // The pv doesn't necessarily contain the best move for multipv searches. When run though cli `--bench`, the bench search doesn't do multipv,
        // but it's possible to input e.g. `bench mpv 2` to get a multipv bench. Additionally, bench is important for debugging, so to catch
        // bugs where the best move changes but not the PV, the best move and ponder move are included in the bench hash
        self.best_move().hash(&mut hasher);
        self.ponder_move().hash(&mut hasher);
        let hash = hasher.finish();
        BenchResult {
            nodes: self.uci_nodes(),
            time: self.start_time().elapsed(),
            max_depth: self.depth(),
            depth: None,
            pv_score_hash: hash,
        }
    }

    fn to_search_info(&self) -> SearchInfo<B> {
        SearchInfo {
            best_move_of_all_pvs: self.best_move(),
            depth: self.depth(),
            seldepth: self.seldepth(),
            time: self.start_time().elapsed(),
            nodes: NodesLimit::new(self.uci_nodes()).unwrap(),
            pv_num: self.current_pv_num,
            max_num_pvs: self.params.num_multi_pv,
            pv: self.current_mpv_pv(),
            score: self.cur_pv_data().score,
            hashfull: self.estimate_hashfull(),
            pos: self.params.pos.clone(),
            bound: self.cur_pv_data().bound,
            additional: Self::additional(),
        }
    }

    fn aggregated_statistics(&self) -> &Statistics {
        &self.aggregated_statistics
    }

    fn send_search_info(&self) {
        if let Some(mut output) = self.search_params().thread_type.output() {
            output.write_search_info(self.to_search_info());
        }
    }

    fn write_internal_info(&self) -> Option<String> {
        self.custom.write_internal_info()
    }
}

impl<B: Board, E: SearchStackEntry<B>, C: CustomInfo<B>> SearchState<B, E, C> {
    /// True if the engine has received a 'stop' command or if a search limit has been reached.
    fn stop_flag(&self) -> bool {
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
        SearchResult::new(self.best_move(), self.best_score(), self.ponder_move(), self.params.pos.clone())
    }

    fn stop_search(&self) {
        self.search_params().atomic.set_stop(true);
    }

    /// Returns the number of nodes looked at so far, including normal search and quiescent search.
    /// Can be called during search.
    /// For smp, this only returns the number of nodes looked at in the current thread.
    fn uci_nodes(&self) -> u64 {
        self.search_params().atomic.nodes()
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

    fn send_non_ugi(&mut self, typ: Message, message: &fmt::Arguments) {
        if let Some(mut output) = self.search_params().thread_type.output() {
            output.write_message(typ, message);
        }
    }

    fn send_currmove(&mut self, mov: B::Move, move_nr: usize, score: Score, alpha: Score, beta: Score) {
        if let Some(mut output) = self.params.thread_type.output() {
            output.write_currmove(&self.params.pos, mov, move_nr, score, alpha, beta);
            self.last_msg_time = Instant::now();
        }
    }

    /// Marked as cold since it's turned off by default in non-interactive mode, and will be called very rarely even if enabled.
    #[cold]
    fn send_currline(&mut self, ply: usize, eval: Score, alpha: Score, beta: Score) {
        if let Some(mut output) = self.params.thread_type.output() {
            if self.search_stack[0].last_played_move().is_none() {
                return;
            }
            let line = self.search_stack.iter().take(ply).map(|entry| entry.last_played_move().unwrap());
            output.write_currline(&self.params.pos, line, eval, alpha, beta);
            self.last_msg_time = Instant::now();
        }
    }

    /// this will block if
    /// a) this is a main thread (i.e., it actually outputs), and
    /// b) the search is an infinite search from `go infinite` but not `ponder`, and
    /// c) the search hasn't been cancelled yet. It will wait until the search has been cancelled.
    /// Auxiliary threads and ponder searches both return instantly from this function, without printing anything.
    /// If the search result has chosen a null move, this instead outputs a warning and a random legal move.
    fn send_search_res(&mut self, res: &SearchResult<B>) {
        let search_params = self.search_params();
        let Main(data) = &search_params.thread_type else {
            return;
        };
        if search_params.atomic.suppress_best_move.load(Acquire) {
            return;
        }
        if data.search_type == Infinite {
            while !self.search_params().atomic.stop_flag() {
                spin_loop();
            }
        }
        let pos = &self.search_params().pos;
        let mut output = data.output.lock().unwrap();
        if res.chosen_move == B::Move::default() {
            let mut rng = StdRng::seed_from_u64(42); // keep everything deterministic
            let chosen_move = pos.random_legal_move(&mut rng).unwrap_or_default();
            if chosen_move != B::Move::default() {
                debug_assert!(pos.is_move_legal(chosen_move), "{} {pos}", chosen_move.compact_formatter(pos));
                output.write_message(
                    Warning,
                    &format_args!("Engine did not return a best move, playing a random move instead"),
                );
                output.write_search_res(&SearchResult::<B>::move_only(chosen_move, pos.clone()));
                return;
            }
            output.write_message(Warning, &format_args!("search() called in a position with no legal moves"));
        }
        debug_assert!(res.chosen_move == B::Move::default() || pos.is_move_legal(res.chosen_move));

        output.write_search_res(res);
    }

    fn new(max_depth: Depth) -> Self {
        Self::new_with(vec![E::default(); max_depth.get() + 1], C::default())
    }

    fn new_with(search_stack: Vec<E>, custom: C) -> Self {
        let start_time = Instant::now();
        let params =
            SearchParams::new_unshared(B::default(), SearchLimit::infinite(), ZobristHistory::default(), TT::minimal());
        Self {
            search_stack,
            start_time,
            custom,
            statistics: Statistics::default(),
            aggregated_statistics: Statistics::default(),
            multi_pvs: vec![],
            params,
            excluded_moves: vec![],
            current_pv_num: 0,
            last_msg_time: start_time,
        }
    }

    fn cur_pv_data(&self) -> &PVData<B> {
        &self.multi_pvs[self.current_pv_num]
    }

    fn cur_pv_data_mut(&mut self) -> &mut PVData<B> {
        &mut self.multi_pvs[self.current_pv_num]
    }

    /// Each thread has its own copy, but the main thread can access the copies of the auxiliary threads
    fn atomic(&self) -> &AtomicSearchState<B> {
        &self.params.atomic
    }

    fn additional() -> Option<String> {
        None
    }

    fn search_params(&self) -> &SearchParams<B> {
        &self.params
    }

    fn search_params_mut(&mut self) -> &mut SearchParams<B> {
        &mut self.params
    }

    fn start_time(&self) -> Instant {
        self.start_time
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
        self.aggregated_statistics.aggregate_searches(&self.statistics);
    }

    fn send_statistics(&mut self) {
        // don't pay the performance penalty of aggregating statistics unless they are shown,
        // especially since the "statistics" feature is likely turned off
        if cfg!(feature = "statistics") {
            self.send_non_ugi(Message::Debug, &format_args!("{}", Summary::new(self.statistics())));
        }
    }

    fn current_mpv_pv(&self) -> &[B::Move] {
        // self.search_stack[0].pv doesn't have to be the same as `self.multi_pvs[self.current_pv_num].pv`
        // because it gets cleared when visiting the root,
        // and if the root never updates its PV (because it fails low or because the search is stopped), it will remain
        // empty. On the other hand, it can get updated during search; this only updates after each aw.
        self.search_stack
            .get(0)
            .and_then(|e| e.pv())
            .unwrap_or_else(|| &self.multi_pvs[self.current_pv_num].pv.list.as_slice())
    }
}

// TODO: Necessary?
pub fn run_bench<B: Board>(engine: &mut dyn Engine<B>, with_nodes: bool, positions: &[B]) -> BenchResult {
    let nodes = if with_nodes { Some(SearchLimit::nodes(engine.default_bench_nodes())) } else { None };
    let depth = SearchLimit::depth(engine.default_bench_depth());
    run_bench_with(engine, depth, nodes, positions, None)
}

pub fn run_bench_with<B: Board>(
    engine: &mut dyn Engine<B>,
    limit: SearchLimit,
    second_limit: Option<SearchLimit>,
    bench_positions: &[B],
    tt: Option<TT>,
) -> BenchResult {
    let mut hasher = DefaultHasher::new();
    let mut total = BenchResult::default();
    let tt = tt.unwrap_or_default();
    for position in bench_positions {
        // engine.forget();
        single_bench(position, engine, limit, tt.clone(), &mut total, &mut hasher);
        if let Some(limit) = second_limit {
            single_bench(position, engine, limit, tt.clone(), &mut total, &mut hasher);
        }
    }
    if limit.depth != SearchLimit::infinite().depth {
        total.depth = Some(limit.depth);
    }
    total.pv_score_hash = hasher.finish();
    if cfg!(feature = "statistics") {
        eprintln!("{}", Summary::new(engine.search_state_dyn().aggregated_statistics()));
    }
    total
}

fn single_bench<B: Board>(
    pos: &B,
    engine: &mut dyn Engine<B>,
    limit: SearchLimit,
    tt: TT,
    total: &mut BenchResult,
    hasher: &mut DefaultHasher,
) {
    #[cfg(feature = "fuzzing")]
    let limit = {
        let mut limit = limit;
        limit.fixed_time = limit.fixed_time.min(Duration::from_millis(20));
        limit
    };
    let res = engine.bench(pos.clone(), limit, tt);
    total.nodes += res.nodes;
    total.time += res.time;
    total.max_depth = total.max_depth.max(res.max_depth);
    res.pv_score_hash.hash(hasher);
}

#[cfg(test)]
mod tests {
    use super::*;
    use gears::general::board::BoardHelpers;
    use gears::general::moves::Move;

    // A testcase that any engine should pass
    pub fn generic_engine_test<B: Board, E: Engine<B>>(mut engine: E) {
        let tt = TT::default();
        for p in B::bench_positions() {
            let res = engine.bench(p.clone(), SearchLimit::nodes_(1), tt.clone());
            assert!(res.depth.is_none());
            assert!(res.max_depth.get() <= 1 + 1); // possible extensions
            assert!(res.nodes <= 100); // TODO: Assert exactly 1
            let params =
                SearchParams::new_unshared(p.clone(), SearchLimit::depth_(1), ZobristHistory::default(), tt.clone());
            let res = engine.search(params);
            let legal_moves = p.legal_moves_slow();
            if legal_moves.num_moves() > 0 {
                assert!(legal_moves.into_iter().contains(&res.chosen_move));
            } else {
                assert!(res.chosen_move.is_null());
            }
            // empty search moves, which is something the engine should handle
            let params =
                SearchParams::new_unshared(p.clone(), SearchLimit::depth_(2), ZobristHistory::default(), tt.clone())
                    .restrict_moves(vec![]);
            let res = engine.search(params);
            assert!(res.chosen_move.is_null());
            let mut search_moves = p.pseudolegal_moves().into_iter().collect_vec();
            search_moves.truncate(search_moves.len() / 2);
            search_moves.push(search_moves.first().copied().unwrap_or_default());
            search_moves.push(B::Move::default());
            let multi_pv = search_moves.len() + 3;
            let params =
                SearchParams::new_unshared(p, SearchLimit::nodes_(1_234), ZobristHistory::default(), tt.clone())
                    .additional_pvs(multi_pv - 1)
                    .restrict_moves(search_moves.clone());
            let res = engine.search(params);
            assert!(search_moves.contains(&res.chosen_move));
            // assert_eq!(engine.search_state().internal_node_count(), 1_234); // TODO: Assert exact match
        }
    }
}
