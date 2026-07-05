use crate::eval::Eval;
use crate::io::ugi_output::AbstractUgiOutput;
use crate::search::multithreading::SearchThreadType::*;
use crate::search::multithreading::{
    EngineReceives, EngineThread, Sender, SharedPerThreadState, SharedSearchState, ThreadData,
};
use crate::search::tt::{Age, TT};
use crossbeam_channel::unbounded;
use derive_more::{Add, Neg, Sub};
use gears::arrayvec::ArrayVec;
use gears::colored::Color::Red;
use gears::colored::Colorize;
use gears::dyn_clone::DynClone;
use gears::games::{BoardHistDyn, ZobristHistory};
use gears::general::board::Strictness::Relaxed;
use gears::general::board::{BoardHelpers, BoardTrait};
use gears::general::common::anyhow::bail;
use gears::general::common::{dbg_print, dbg_reset, EntityList, Name, NamedEntity, Res, StaticallyNamedEntity};
use gears::general::move_list::MoveListTrait;
use gears::itertools::Itertools;
use gears::output::Message;
use gears::score::{Score, ScoreT, MAX_BETA, MIN_ALPHA, NO_SCORE_YET, SCORE_LOST, SCORE_WON};
use gears::search::{Budget, DepthPly, NodeType, NodesLimit, SearchInfo, SearchLimit, SearchResult, TimeControl};
use gears::ugi::{EngineOption, EngineOptionNameForProtocol, EngineOptionType};
use std::collections::HashMap;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::Ordering::Release;
use std::sync::Arc;
use std::thread::spawn;
use std::time::{Duration, Instant};

#[cfg(feature = "chess")]
pub mod chess;
pub mod generic;
pub mod multithreading;
pub(crate) mod spsa_param;
pub(super) mod tt;

// only evaluate the debug message if debug mode is actually enabled
#[macro_export]
macro_rules! send_debug_msg {
    ($state: expr, $($args: tt)*) => {
        $crate::send_debug_msg_impl!($state.thread_data(), $($args)*)
    };
}

#[macro_export]
macro_rules! send_debug_msg_impl {
    ($thread_data: expr, $($args: tt)*) => {
        if let Some(mut output) = $thread_data.thread_type.output() && output.show_debug_output {
            output.write_message(gears::output::Message::Debug, &format_args!($($args)*));
        }
    };
}

#[derive(Debug, Clone)]
#[must_use]
pub struct EngineInfo {
    engine: Name,
    eval: Option<Name>,
    version: String,
    default_bench_depth: DepthPly,
    default_bench_nodes: NodesLimit,
    options: HashMap<EngineOptionNameForProtocol, EngineOptionType>,
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
    pub fn invalid() -> Self {
        Self {
            engine: Default::default(),
            eval: None,
            version: "".to_string(),
            default_bench_depth: Default::default(),
            default_bench_nodes: NodesLimit::MAX,
            options: Default::default(),
            max_threads: 0,
            internal_state_description: None,
        }
    }

    pub fn new<B: BoardTrait, E: Engine<B>>(
        engine: &E,
        eval: &dyn Eval<B>,
        version: &str,
        default_bench_depth: DepthPly,
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

    pub fn default_bench_depth(&self) -> DepthPly {
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
    pub max_iterations: DepthPly,
    pub depth: Option<DepthPly>,
    pub hash: u64,
}

impl Default for BenchResult {
    fn default() -> Self {
        Self { nodes: 0, time: Duration::default(), depth: None, max_iterations: DepthPly::new(0), hash: 0 }
    }
}

impl Display for BenchResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let depth = if let Some(depth) = self.depth { format!("depth {depth}, ") } else { String::new() };
        writeln!(
            f,
            "{depth}max depth {0}, time {2} ms, {1} nodes, {3} nps, hash {4:X}",
            self.max_iterations.get(),
            Colorize::bold(self.nodes.to_string().as_str()),
            self.time.as_millis().to_string().color(Red),
            (self.nodes as f64 / self.time.as_millis() as f64 * 1000.0).round().to_string().color(Red),
            self.hash,
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Pv<B: BoardTrait, const LIMIT: usize> {
    list: ArrayVec<B::Move, LIMIT>,
}

impl<B: BoardTrait, const LIMIT: usize> Default for Pv<B, LIMIT> {
    fn default() -> Self {
        Self { list: ArrayVec::new() }
    }
}

impl<B: BoardTrait, const LIMIT: usize> Pv<B, LIMIT> {
    pub fn extend(&mut self, mov: B::Move, child_pv: &Pv<B, LIMIT>) {
        self.reset_to_move(mov);
        self.list.try_extend_from_slice(child_pv.list.as_slice()).unwrap();
    }

    pub fn assert_valid(&self, mut pos: B) {
        for &m in &self.list {
            assert!(pos.is_move_legal(m));
            pos = pos.make_move(m).unwrap();
        }
        assert!(pos.debug_verify_invariants(Relaxed).is_ok());
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

pub trait AbstractEvalBuilder<B: BoardTrait>: NamedEntity + DynClone {
    fn build(&self) -> Box<dyn Eval<B>>;
}

#[derive(Debug, Default)]
pub struct EvalBuilder<B: BoardTrait, E: Eval<B> + Default> {
    _phantom_board: PhantomData<B>,
    _phantom_eval: PhantomData<E>,
}

impl<B: BoardTrait, E: Eval<B> + Default> Clone for EvalBuilder<B, E> {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl<B: BoardTrait, E: Eval<B> + Default> StaticallyNamedEntity for EvalBuilder<B, E> {
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

impl<B: BoardTrait, E: Eval<B> + Default> AbstractEvalBuilder<B> for EvalBuilder<B, E> {
    fn build(&self) -> Box<dyn Eval<B>> {
        Box::new(E::default())
    }
}

pub type EvalList<B> = EntityList<Box<dyn AbstractEvalBuilder<B>>>;

/// A trait because this type erases over the Engine being built.
/// There are two related concepts: `Engine` and `Searcher`.
/// A searcher is an algorithm like caps or gaps, an engine is a searcher plus an eval.
pub trait AbstractSearcherBuilder<B: BoardTrait>: NamedEntity + DynClone {
    fn build_in_new_thread(
        &self,
        thread_data: ThreadData<B>,
        eval: Box<dyn Eval<B>>,
    ) -> (Sender<EngineReceives<B>>, Arc<SharedSearchState<B>>) {
        let engine = self.build_for_eval(thread_data, eval);
        let info = engine.engine_info();
        let data = engine.search_state_dyn().thread_data();
        if let Main(data) = &data.thread_type {
            *data.engine_info.lock().unwrap() = info;
        }
        let data = data.shared.clone();
        let (sender, receiver) = unbounded();
        let mut thread = EngineThread::new(engine, receiver);
        _ = spawn(move || thread.main_loop());
        (sender, data)
    }

    fn build_in_this_thread(&self, eval_builder: &dyn AbstractEvalBuilder<B>) -> Box<dyn Engine<B>> {
        self.build_for_eval(ThreadData::single_and_no_output(), eval_builder.build())
    }

    fn build_for_eval(&self, thread_data: ThreadData<B>, eval: Box<dyn Eval<B>>) -> Box<dyn Engine<B>>;
}

pub type SearcherList<B> = EntityList<Box<dyn AbstractSearcherBuilder<B>>>;

#[derive(Debug)]
pub struct SearcherBuilder<B: BoardTrait, E: Engine<B>> {
    _phantom_b: PhantomData<B>,
    _phantom_e: PhantomData<E>,
}

impl<B: BoardTrait, E: Engine<B>> Default for SearcherBuilder<B, E> {
    fn default() -> Self {
        Self { _phantom_b: PhantomData, _phantom_e: PhantomData }
    }
}

impl<B: BoardTrait, E: Engine<B>> Clone for SearcherBuilder<B, E> {
    fn clone(&self) -> Self {
        Self { _phantom_b: PhantomData, _phantom_e: PhantomData }
    }
}

impl<B: BoardTrait, E: Engine<B>> SearcherBuilder<B, E> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<B: BoardTrait, E: Engine<B>> AbstractSearcherBuilder<B> for SearcherBuilder<B, E> {
    fn build_for_eval(&self, thread_data: ThreadData<B>, eval: Box<dyn Eval<B>>) -> Box<dyn Engine<B>> {
        Box::new(E::new(thread_data, eval))
    }
}

impl<B: BoardTrait, E: Engine<B>> StaticallyNamedEntity for SearcherBuilder<B, E> {
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

pub trait Engine<B: BoardTrait>: StaticallyNamedEntity + Send + 'static {
    type SearchStackEntry: SearchStackEntry<B>
    where
        Self: Sized;
    type CustomInfo: CustomInfo<B>
    where
        Self: Sized;

    fn new(thread_data: ThreadData<B>, eval: Box<dyn Eval<B>>) -> Self
    where
        Self: Sized;

    fn for_eval<E: Eval<B> + Default>() -> Self
    where
        Self: Sized,
    {
        Self::new(ThreadData::single_and_no_output(), Box::new(E::default()))
    }

    // fn thread_data(&self) -> &ThreadData<B> {
    //     self.search_state_dyn().thread_data()
    // }

    /// This should return the static eval (possibly with WDL normalization) without doing any kind of search.
    /// For engines like `RandomMover` where there is no static eval, this should return `Score(0)`.
    /// Most evals completely ignore the `ply` parameter, but it can be used e.g. to decide which color we are
    /// for asymmetric evals.
    fn static_eval(&mut self, pos: &B, ply: usize) -> Score;

    fn bench(&mut self, pos: B, limit: SearchLimit, tt: TT, additional_pvs: usize) -> BenchResult {
        let mut params = SearchParams::new_unshared(pos, limit, ZobristHistory::default(), tt);
        params.num_multi_pv = additional_pvs + 1;
        let _ = self.search(params);
        self.search_state_dyn().to_bench_res()
    }

    fn default_bench_nodes(&self) -> NodesLimit {
        self.engine_info().default_bench_nodes
    }

    fn default_bench_depth(&self) -> DepthPly {
        self.engine_info().default_bench_depth
    }

    fn max_bench_depth(&self) -> DepthPly;

    fn search_state_dyn(&self) -> &dyn AbstractSearchState<B>;

    fn search_state_mut_dyn(&mut self) -> &mut dyn AbstractSearchState<B>;

    /// Returns an optional description of what the engine thinks about this move, such as history values.
    fn eval_move(&self, _pos: &B, _mov: B::Move) -> Option<String> {
        None
    }

    /// Reset the engine into a fresh state, e.g. by clearing the TT and various heuristics.
    fn forget(&mut self) {
        let start = Instant::now();
        self.search_state_mut_dyn().forget(true);
        send_debug_msg!(
            self.search_state_dyn(),
            "Forgetting all search state took {} microseconds",
            start.elapsed().as_micros()
        );
    }
    /// Returns information about this engine, such as the name, version and default bench depth.
    fn engine_info(&self) -> EngineInfo;

    fn limit(&self) -> &SearchLimit
    where
        Self: Sized,
    {
        &self.search_state_dyn().search_params().limit
    }

    /// Returns a [`SearchInfo`] object with information about the search so far.
    /// Can be called during search, only returns the information regarding the current thread.
    fn search_info(&self, final_info: bool) -> SearchInfo<'_, B> {
        self.search_state_dyn().to_search_info(final_info)
    }

    /// Sets an option with the name 'option' to the value 'value'.
    fn set_option(
        &mut self,
        option: EngineOptionNameForProtocol,
        old_value: &mut EngineOptionType,
        value: String,
    ) -> Res<()> {
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

    fn get_eval(&mut self) -> Option<&dyn Eval<B>>;

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
        if !self.search_state_dyn().should_show_debug_msg() {
            self.search_state_mut_dyn().new_search(search_params);
        } else {
            let before = Instant::now();
            self.search_state_mut_dyn().new_search(search_params);
            let after_setup = Instant::now();
            let total_elapsed =
                after_setup.duration_since(self.search_state_dyn().search_params().limit.start_time).as_micros();
            send_debug_msg!(
                self.search_state_dyn(),
                "Preparing the search state for a new search took {0} microseconds, {1} have elapsed since the search request",
                after_setup.duration_since(before).as_micros(),
                total_elapsed
            );
        }
        let mut res = self.do_search();
        self.search_state_mut_dyn().end_search(&mut res);
        res
    }

    /// The important function.
    /// Should not be called directly
    fn do_search(&mut self) -> SearchResult<B>;
}

/// A proof number search isn't a normal engine, and neither is a random mover
pub trait NormalEngine<B: BoardTrait>: Engine<B> {
    fn search_state(&self) -> &SearchStateFor<B, Self>
    where
        Self: Sized;

    fn search_state_mut(&mut self) -> &mut SearchStateFor<B, Self>
    where
        Self: Sized;

    fn time_up(&self, tc: TimeControl, hard_limit: Duration, byoyomi: Duration, elapsed: Duration) -> bool {
        elapsed >= byoyomi + hard_limit.min(tc.remaining / 32 + tc.increment / 2)
    }

    // Sensible default values, but engines may choose to check more/less frequently than every n nodes
    fn count_node_and_test_stop(&mut self) -> bool
    where
        Self: Sized,
    {
        let nodes = self.search_state_mut().atomic().count_node();
        let state = self.search_state();
        let limit = self.limit();
        // Do the less expensive checks first to avoid querying the time in each node, but also
        // to ensure the game is reproducible
        if nodes >= limit.nodes.get()
            || (state.stop_flag() && (state.atomic().iterations().get() > 1 || !self.search_state().is_main_thread()))
        {
            self.search_state().thread_data.shared.set_stop();
            return true;
        }
        // Node limits are per-thread, but time limits are only checked on the main thread, which then
        // sets the stop flag for all threads.
        if nodes % DEFAULT_CHECK_TIME_INTERVAL != 0 || !self.search_state().is_main_thread() {
            return false;
        }
        let elapsed = self.search_state().start_time().elapsed();
        if self.time_up(limit.tc, limit.fixed_time, limit.byoyomi, elapsed) {
            self.search_state().thread_data.shared.set_stop();
            return true;
        }
        false
    }

    fn should_not_start_negamax(
        &self,
        elapsed: Duration,
        soft_limit: Duration,
        soft_nodes: u64,
        iter: isize,
        max_soft_iter: isize,
        mate_depth: isize,
    ) -> bool
    where
        Self: Sized,
    {
        let state = self.search_state();
        if iter <= 1 || !self.search_state().is_main_thread() {
            return false;
        }
        let score = state.best_score();
        (elapsed >= soft_limit
            // even in a multipv search, we stop as soon as a single mate is found
            || score >= Score(SCORE_WON.0 - mate_depth as ScoreT))
            || (score > SCORE_LOST && score <= Score(SCORE_LOST.0 - mate_depth as ScoreT))
            || state.uci_nodes() >= soft_nodes
            || iter > max_soft_iter
    }
}

const DEFAULT_CHECK_TIME_INTERVAL: u64 = 1024;

#[allow(type_alias_bounds)]
pub type SearchStateFor<B: BoardTrait, E: NormalEngine<B>> = SearchState<B, E::SearchStackEntry, E::CustomInfo>;

#[derive(Debug, Default, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Add, Sub, Neg)]
#[must_use]
pub struct MoveScore(pub i16);

impl MoveScore {
    const MAX: MoveScore = MoveScore(i16::MAX);
}

/// A struct bundling parameters that modify the core search.
#[derive(Debug, Default, Clone)]
pub struct SearchParams<B: BoardTrait> {
    pub pos: B,
    pub limit: SearchLimit,
    pub history: ZobristHistory,
    pub tt: TT,
    pub restrict_moves: Option<Vec<B::Move>>,
    // may be set to 0 if there are no legal moves
    pub num_multi_pv: usize,
    pub contempt: Score,
}

impl<B: BoardTrait> SearchParams<B> {
    pub fn for_pos(pos: B, limit: SearchLimit) -> Self {
        Self::new_unshared(pos, limit, ZobristHistory::default(), TT::default())
    }

    pub fn new_unshared(pos: B, limit: SearchLimit, history: ZobristHistory, tt: TT) -> Self {
        Self::create(pos, limit, history, tt, None, 0, Score(0))
    }

    #[expect(clippy::too_many_arguments)]
    fn create(
        pos: B,
        limit: SearchLimit,
        history: ZobristHistory,
        tt: TT,
        restrict_moves: Option<Vec<B::Move>>,
        additional_pvs: usize,
        contempt: Score,
    ) -> Self {
        Self { pos, limit, history, tt, restrict_moves, num_multi_pv: additional_pvs + 1, contempt }
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

    pub fn with_contempt(mut self, contempt: Score) -> Self {
        self.contempt = contempt;
        self
    }
}

pub trait SearchStackEntry<B: BoardTrait>: Default + Clone + Debug {
    fn forget(&mut self) {
        self.clone_from(&Self::default());
    }
    fn pv(&self) -> Option<&[B::Move]>;
    fn last_played_move(&self) -> Option<B::Move>;
    fn hash(&self, hasher: &mut impl Hasher);
}

#[derive(Copy, Clone, Default, Debug)]
pub struct EmptySearchStackEntry {}

impl<B: BoardTrait> SearchStackEntry<B> for EmptySearchStackEntry {
    fn pv(&self) -> Option<&[B::Move]> {
        None
    }

    fn last_played_move(&self) -> Option<B::Move> {
        None
    }

    fn hash(&self, _hasher: &mut impl Hasher) {}
}

pub trait CustomInfo<B: BoardTrait>: Default + Clone + Debug {
    fn new_search(&mut self) {
        self.hard_forget_except_tt();
    }
    fn hard_forget_except_tt(&mut self);

    fn write_internal_info(&self, _pos: &B) -> Option<String> {
        None
    }
}

#[derive(Default, Clone, Debug)]
pub struct NoCustomInfo {}

impl<B: BoardTrait> CustomInfo<B> for NoCustomInfo {
    fn hard_forget_except_tt(&mut self) {}
}

#[derive(Debug, Clone)]
pub struct PVData<B: BoardTrait> {
    alpha: Score,
    beta: Score,
    radius: Score,
    pub pv: Pv<B, 200>, // A PV of 200 plies should be more than enough for anybody (tm)
    pub score: Score,
    pub bound: Option<NodeType>,
}

impl<B: BoardTrait> Default for PVData<B> {
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

impl<B: BoardTrait> PVData<B> {
    pub fn reset(&mut self) {
        self.alpha = MIN_ALPHA;
        self.beta = MAX_BETA;
        self.radius = Score(20);
        self.pv.clear();
        self.score = NO_SCORE_YET;
        self.bound = None;
    }
}

pub trait AbstractSearchState<B: BoardTrait> {
    fn forget(&mut self, hard: bool);
    fn new_search(&mut self, params: SearchParams<B>);
    fn end_search(&mut self, res: &mut SearchResult<B>);
    fn thread_data(&self) -> &ThreadData<B>;
    fn search_params(&self) -> &SearchParams<B>;
    /// Returns the number of nodes looked at so far, including normal search and quiescent search.
    /// Can be called during search.
    /// For smp, this only returns the number of nodes looked at in the current thread.
    fn uci_nodes(&self) -> u64 {
        self.thread_data().this_thread().nodes()
    }
    fn pv_data(&self) -> &[PVData<B>];
    fn to_bench_res(&self) -> BenchResult;
    fn to_search_info(&self, final_info: bool) -> SearchInfo<'_, B>;
    fn send_search_info(&self, final_info: bool);
    fn should_show_debug_msg(&self) -> bool {
        self.thread_data().thread_type.output().is_some_and(|o| o.show_debug_output)
    }
    fn output_minimal(&self) -> bool {
        self.thread_data().thread_type.output().is_some_and(|o| o.minimal)
    }
    fn send_non_ugi(&mut self, typ: Message, message: &fmt::Arguments) {
        if let Some(mut output) = self.thread_data().thread_type.output() {
            output.write_message(typ, message);
        }
    }
    fn send_ugi(&mut self, message: &fmt::Arguments) {
        if let Some(mut output) = self.thread_data().thread_type.output() {
            output.write_ugi(message);
        }
    }
    fn age(&self) -> Age {
        self.search_params().tt.age
    }
    /// Engine-specific info, like the contents of history tables.
    fn write_internal_info(&self, pos: &B) -> Option<String>;

    fn is_main_thread(&self) -> bool {
        !matches!(self.thread_data().thread_type, Auxiliary)
    }
}

#[derive(Debug)]
pub struct SearchState<B: BoardTrait, E: SearchStackEntry<B>, C: CustomInfo<B>> {
    thread_data: ThreadData<B>,
    search_stack: Vec<E>,
    params: SearchParams<B>,
    custom: C,
    excluded_moves: Vec<B::Move>,
    multi_pvs: Vec<PVData<B>>,
    current_pv_num: usize,
    // The internal engine depth (if applicable) is represented as `Budget` and can be fractional.
    // This is different from the UCI "depth", expressed as `DepthPly`, which is the ID loop counter for a/b engines with ID.
    budget: Budget,
    execution_start_time: Instant,
    last_msg_time: Instant,
    age: Age,
}

impl<B: BoardTrait, E: SearchStackEntry<B>, C: CustomInfo<B>> Deref for SearchState<B, E, C> {
    type Target = C;
    fn deref(&self) -> &Self::Target {
        &self.custom
    }
}

impl<B: BoardTrait, E: SearchStackEntry<B>, C: CustomInfo<B>> DerefMut for SearchState<B, E, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.custom
    }
}

impl<B: BoardTrait, E: SearchStackEntry<B>, C: CustomInfo<B>> AbstractSearchState<B> for SearchState<B, E, C> {
    // `hard` is false when starting a new search and true when receiving `ucinewgame`
    fn forget(&mut self, hard: bool) {
        self.last_msg_time = Instant::now();
        self.execution_start_time = self.last_msg_time;
        // When starting a new search, the main thread must have already reset the stop flag.
        // Doing so here would introduce a race condition where we ignore a go command that comes immediately after this command.
        // However, we still need to reset the shared per-thread state, such as the node counter.
        self.atomic().reset();
        if hard {
            for e in &mut self.search_stack {
                e.forget();
            }
            self.custom.hard_forget_except_tt();
        } else {
            if let Some(e) = self.search_stack.get_mut(0) {
                e.forget();
            }
            self.custom.new_search();
        }
        self.params.history.clear(); // will get overwritten later
        for pv in &mut self.multi_pvs {
            pv.reset();
        }
    }

    fn new_search(&mut self, mut parameters: SearchParams<B>) {
        if matches!(self.thread_data.thread_type, SingleAndNoOutput) {
            self.thread_data.shared.set_searching(true);
            self.thread_data.shared.should_stop.store(false, Release);
        } else {
            debug_assert!(self.thread_data.shared.currently_searching());
        }
        self.forget(false);
        let moves = parameters.pos.legal_moves_slow();
        let num_moves = moves.num_moves();
        self.age = parameters.tt.age;
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
        // contains only invalid moves. Search must be able to deal with this, but we still add an empty multipv entry
        self.multi_pvs.resize_with(parameters.num_multi_pv.max(1), PVData::default);
        // If only one move can be played, immediately return it without doing a real search to make the engine appear
        // smarter, and perform better in cases like lichess when it's up against an opponent with pondering enabled.
        // However, don't do this if the engine is used for analysis.
        if num_moves == 1 && parameters.limit.is_only_time_based() {
            parameters.limit.depth = DepthPly::new(1);
        }
        if let Some(mut output) = self.thread_data.thread_type.output() {
            output.new_search();
        }
        parameters.limit.soft_nodes = parameters.limit.soft_nodes.min(parameters.limit.nodes);
        self.params = parameters;
        // it's possible that a stop command has already been received and handled, which means the stop flag
        // can already be set
    }

    fn end_search(&mut self, res: &mut SearchResult<B>) {
        send_debug_msg!(
            self,
            "Ending a search that took {0} microseconds ({1} microseconds since starting searching in this thread)",
            self.start_time().elapsed().as_micros(),
            self.execution_start_time.elapsed().as_micros()
        );
        // might block, see method. Do this as the last step so that we're not using compute after sending
        // the search result and so that we avoid race conditions.
        self.thread_data.end_and_send(res, self.start_time());
        send_debug_msg!(
            self,
            "Finished writing the search res {0} microseconds after getting a search command",
            self.start_time().elapsed().as_micros(),
        );
    }

    fn thread_data(&self) -> &ThreadData<B> {
        &self.thread_data
    }

    fn search_params(&self) -> &SearchParams<B> {
        &self.params
    }

    fn pv_data(&self) -> &[PVData<B>] {
        self.multi_pvs.as_slice()
    }

    fn to_bench_res(&self) -> BenchResult {
        let mut hasher = DefaultHasher::new();
        for pv_data in &self.multi_pvs {
            for mov in &pv_data.pv.list {
                mov.hash(&mut hasher);
            }
            pv_data.score.hash(&mut hasher);
            pv_data.alpha.hash(&mut hasher);
            pv_data.beta.hash(&mut hasher);
        }
        if let Some(pv) = self.search_stack.first().and_then(|e| e.pv()) {
            for mov in pv {
                mov.hash(&mut hasher);
            }
        }
        for entry in &self.search_stack {
            entry.hash(&mut hasher);
        }
        // the score can differ even if the pv is the same, so make sure to include that in the hash
        self.best_score().hash(&mut hasher);
        // The pv doesn't necessarily contain the best move for multipv searches. Additionally, bench is important for debugging, so to catch
        // bugs where the best move changes but not the PV, the best move and ponder move are included in the bench hash
        self.atomic().best_move().hash(&mut hasher);
        self.atomic().ponder_move().hash(&mut hasher);
        let hash = hasher.finish();
        BenchResult {
            nodes: self.uci_nodes(),
            time: self.execution_start_time().elapsed(),
            max_iterations: self.atomic().iterations(),
            depth: None,
            hash,
        }
    }

    fn to_search_info(&self, final_info: bool) -> SearchInfo<'_, B> {
        debug_assert!(!matches!(self.thread_data.thread_type, Auxiliary));

        let shared = self.thread_data.shared_atomic_state();
        let nodes = NodesLimit::new(shared.iter().map(|d| d.nodes()).sum()).unwrap();
        let seldepth = shared.iter().map(|d| d.seldepth()).max().unwrap();
        let num_threads = shared.len();
        SearchInfo {
            best_move_of_all_pvs: self.atomic().best_move(),
            iterations: self.atomic().iterations(),
            budget: self.budget,
            seldepth,
            time: self.execution_start_time().elapsed(),
            nodes,
            pv_num: self.current_pv_num,
            max_num_pvs: self.params.num_multi_pv,
            pv: self.current_mpv_pv(),
            score: self.cur_pv_data().score,
            hashfull: self.estimate_hashfull(self.age()),
            pos: self.params.pos.clone(),
            bound: self.cur_pv_data().bound,
            num_threads,
            additional: Self::additional(),
            final_info,
        }
    }

    fn send_search_info(&self, final_info: bool) {
        if let Some(mut output) = self.thread_data.thread_type.output() {
            output.write_search_info(self.to_search_info(final_info));
        }
    }

    fn age(&self) -> Age {
        self.age
    }

    fn write_internal_info(&self, pos: &B) -> Option<String> {
        self.custom.write_internal_info(pos)
    }
}

impl<B: BoardTrait, E: SearchStackEntry<B>, C: CustomInfo<B>> SearchState<B, E, C> {
    /// True if the engine has received a 'stop' command or if a search limit has been reached.
    fn stop_flag(&self) -> bool {
        self.thread_data.shared.stop_flag()
    }

    fn estimate_hashfull(&self, age: Age) -> usize {
        self.tt().estimate_hashfull::<B>(age)
    }

    fn best_score(&self) -> Score {
        self.atomic().score()
    }

    fn search_result(&self) -> SearchResult<B> {
        SearchResult::new(
            self.atomic().best_move(),
            self.best_score(),
            self.atomic().ponder_move(),
            self.params.pos.clone(),
        )
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

    // move_nr starts from 1, not 0
    fn send_currmove(&mut self, mov: B::Move, move_nr: usize, score: Score, alpha: Score, beta: Score) {
        if let Some(mut output) = self.thread_data.thread_type.output() {
            output.write_currmove(&self.params.pos, mov, move_nr, score, alpha, beta);
            self.last_msg_time = Instant::now();
        }
    }

    /// Marked as cold since it's turned off by default in non-interactive mode, and will be called very rarely even if enabled.
    #[cold]
    fn send_currline(&mut self, ply: usize, eval: Score, alpha: Score, beta: Score) {
        if let Some(mut output) = self.thread_data.thread_type.output() {
            if self.search_stack[0].last_played_move().is_none() {
                return;
            }
            let line = self.search_stack.iter().take(ply).map(|entry| entry.last_played_move().unwrap());
            output.write_currline(&self.params.pos, line, eval, alpha, beta);
            self.last_msg_time = Instant::now();
        }
    }

    /// Marked as cold for similar reasons to [`Self::send_currline`].
    #[cold]
    fn send_refutation(&mut self, root_move: B::Move, score: Score, move_num: usize) {
        if let Some(mut output) = self.thread_data.thread_type.output() {
            output.write_refutation(&self.params.pos, root_move, score, move_num);
            self.last_msg_time = Instant::now();
        }
    }

    fn new(thread_data: ThreadData<B>, max_ply: DepthPly) -> Self {
        Self::new_with(thread_data, vec![E::default(); max_ply.get() + 1], C::default())
    }

    fn new_with(thread_data: ThreadData<B>, search_stack: Vec<E>, custom: C) -> Self {
        let params =
            SearchParams::new_unshared(B::default(), SearchLimit::infinite(), ZobristHistory::default(), TT::minimal());
        let now = Instant::now();
        Self {
            thread_data,
            search_stack,
            custom,
            multi_pvs: vec![],
            params,
            excluded_moves: vec![],
            current_pv_num: 0,
            budget: Budget::new(0),
            execution_start_time: now,
            last_msg_time: now,
            age: Age::default(),
        }
    }

    fn cur_pv_data(&self) -> &PVData<B> {
        &self.multi_pvs[self.current_pv_num]
    }

    fn cur_pv_data_mut(&mut self) -> &mut PVData<B> {
        &mut self.multi_pvs[self.current_pv_num]
    }

    /// Each thread has its own copy
    fn atomic(&self) -> &SharedPerThreadState<B> {
        self.thread_data.this_thread()
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
        self.params.limit.start_time
    }

    fn execution_start_time(&self) -> Instant {
        self.execution_start_time
    }

    fn current_mpv_pv(&self) -> &[B::Move] {
        // self.search_stack[0].pv doesn't have to be the same as `self.multi_pvs[self.current_pv_num].pv`
        // because it gets cleared when visiting the root,
        // and if the root never updates its PV (because it fails low or because the search is stopped), it will remain
        // empty. On the other hand, it can get updated during search.
        let res = self.search_stack.first().and_then(|e| e.pv());
        if res.is_none_or(|pv| pv.is_empty()) {
            // if we didn't finish looking at the PV, use the PV from the last iteration
            self.multi_pvs[self.current_pv_num].pv.list.as_slice()
        } else {
            res.unwrap()
        }
    }
}

pub fn run_bench_with<B: BoardTrait>(
    engine: &mut dyn Engine<B>,
    limit: SearchLimit,
    second_limit: Option<SearchLimit>,
    bench_positions: &[B],
    tt: Option<TT>,
) -> BenchResult {
    dbg_reset();
    let mut hasher = DefaultHasher::new();
    let mut total = BenchResult::default();
    let mut tt = tt.unwrap_or_default();
    for position in bench_positions {
        // don't reset the engine state between searches to make `bench` reflect how aging etc. affect search.
        tt.age.increment();
        single_bench(position, engine, limit, tt.clone(), 0, &mut total, &mut hasher);
        if let Some(limit) = second_limit {
            tt.age.increment();
            single_bench(position, engine, limit, tt.clone(), 1, &mut total, &mut hasher);
        }
    }
    if limit.depth != SearchLimit::infinite().depth {
        total.depth = Some(limit.depth);
    }
    total.hash = hasher.finish();
    dbg_print();
    total
}

fn single_bench<B: BoardTrait>(
    pos: &B,
    engine: &mut dyn Engine<B>,
    limit: SearchLimit,
    tt: TT,
    additional_pvs: usize,
    total: &mut BenchResult,
    hasher: &mut DefaultHasher,
) {
    #[cfg(feature = "fuzzing")]
    let limit = {
        let mut limit = limit;
        limit.fixed_time = limit.fixed_time.min(Duration::from_millis(20));
        limit
    };
    let res = engine.bench(pos.clone(), limit, tt.clone(), additional_pvs);
    total.nodes += res.nodes;
    total.time += res.time;
    total.max_iterations = total.max_iterations.max(res.max_iterations);
    res.hash.hash(hasher);
    tt.hash_first_1k_entries(hasher);
}

/// The higher the value, the more we predict this node to be a cut node.
/// Large negative values mean we predict this node to be an all node.
// Idea from <https://github.com/all-c-a-p-s/Panda/pull/53>
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Neg)]
pub struct Temperature(pub i32);

pub const MAX_TEMP: i32 = 1 << 16;

impl Temperature {
    // Same gravity formula as histories. This maybe isn't ideal because it means the order in which bonuses get applied can affect
    // the result, but it ensures values are clamped
    pub fn update(&mut self, bonus: i32) {
        let bonus = bonus.clamp(-MAX_TEMP, MAX_TEMP);
        let bonus = bonus - self.0 * bonus.abs() / MAX_TEMP;
        self.0 += bonus;
    }
}

/// Runs bench with the given limit, then clears the engine state and runs everything again,
/// this time with the nodes of the first run as limit. Panics if anything doesn't match.
pub fn test_reproducible<B: BoardTrait>(
    positions: &[B],
    engine: &mut dyn Engine<B>,
    limit: SearchLimit,
    tt_size_mib: usize,
) {
    let old_tt = TT::new_with_mib(tt_size_mib);
    engine.forget();
    let mut results = Vec::with_capacity(positions.len());
    for p in positions {
        let res = engine.bench(p.clone(), limit, old_tt.clone(), 1);
        results.push(res);
    }

    engine.forget();
    let new_tt = TT::new_with_mib(tt_size_mib);
    for (pos, r) in positions.iter().zip_eq(results.iter()) {
        let limit = SearchLimit::nodes_(r.nodes);
        let res = engine.bench(pos.clone(), limit, new_tt.clone(), 1);
        assert_eq!(res.max_iterations, r.max_iterations, "{} {pos}", r.nodes);
        assert_eq!(res.depth, r.depth, "{} {pos}", r.nodes);
        assert_eq!(res.nodes, r.nodes, "{pos}");
        assert_eq!(res.hash, r.hash, "{} {pos}", r.nodes)
    }
    assert_eq!(old_tt.age, new_tt.age);
    let entries = old_tt.all_entries::<B>().zip_eq(new_tt.all_entries());
    for (i, (a, b)) in entries.enumerate() {
        assert_eq!(a, b, "{i}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gears::general::moves::MoveTrait;

    // A testcase that any engine should pass
    pub fn generic_engine_test<B: BoardTrait, E: Engine<B>>(mut engine: E) {
        let tt = TT::default();
        for p in B::bench_positions() {
            let res = engine.bench(p.clone(), SearchLimit::nodes_(1), tt.clone(), 0);
            assert!(res.depth.is_none(), "{res}");
            assert!(res.nodes <= 100, "{res}"); // TODO: Assert exactly 1
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
            let limit = SearchLimit::nodes_(1234).and(SearchLimit::soft_nodes_(1000));
            let params = SearchParams::new_unshared(p, limit, ZobristHistory::default(), tt.clone())
                .additional_pvs(multi_pv - 1)
                .restrict_moves(search_moves.clone());
            let res = engine.search(params);
            assert!(search_moves.contains(&res.chosen_move));
        }
        determinism_test(&mut engine);
    }

    fn determinism_test<B: BoardTrait>(engine: &mut dyn Engine<B>) {
        engine.forget();
        let positions = B::bench_positions().into_iter().collect_vec();
        for p in positions.iter().take(10) {
            engine.forget();
            let limit = SearchLimit::nodes_(1234);
            let params = SearchParams::for_pos(p.clone(), limit);
            let res = engine.search(params);
            engine.forget();
            let params = SearchParams::for_pos(p.clone(), limit);
            let res2 = engine.search(params);
            // make sure all info got reset on `forget()`
            assert_eq!(res, res2);
            // now, do the same test again, but the first search uses a time limit,
            // using the reported nodes for the second search
            engine.forget();
            let time_limit = SearchLimit::per_move(Duration::from_millis(12));
            let params = SearchParams::for_pos(p.clone(), time_limit);
            let res = engine.search(params);
            let nodes = engine.search_state_dyn().uci_nodes();
            let limit = SearchLimit::nodes_(nodes);
            engine.forget();
            let params = SearchParams::for_pos(p.clone(), limit);
            let res2 = engine.search(params);
            let nodes2 = engine.search_state_dyn().uci_nodes();
            assert_eq!(nodes, nodes2, "{p}");
            assert_eq!(res, res2, "{p}");
        }
        let limit = SearchLimit::soft_nodes_(1000).and(SearchLimit::nodes_(2000));
        test_reproducible(&positions, engine, limit, 1);
    }
}
