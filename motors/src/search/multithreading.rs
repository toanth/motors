use colored::Colorize;
use std::hint::spin_loop;
use std::marker::PhantomData;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, AtomicU64};
use std::sync::{Arc, Mutex, MutexGuard};

use dyn_clone::clone_box;
use gears::games::ZobristHistory;
use gears::general::board::Board;
use portable_atomic::AtomicUsize;

use crate::eval::Eval;
use gears::general::common::{parse_int_from_str, Res};
use gears::general::moves::Move;
use gears::output::Message::*;
use gears::score::{Score, NO_SCORE_YET};
use gears::search::{Depth, SearchLimit};
use gears::ugi::EngineOptionName;
use gears::ugi::EngineOptionName::{Hash, Threads};

use crate::search::multithreading::EngineReceives::*;
use crate::search::multithreading::SearchThreadType::{Auxiliary, Main};
use crate::search::multithreading::SearchType::{Infinite, Normal, Ponder};
use crate::search::tt::TT;
use crate::search::{
    AbstractEvalBuilder, AbstractSearcherBuilder, Engine, EngineInfo, SearchParams, SearchState,
};
use crate::ugi_engine::UgiOutput;

pub type Sender<T> = crossbeam_channel::Sender<T>;
pub type Receiver<T> = crossbeam_channel::Receiver<T>;
pub type TryRecvError = crossbeam_channel::TryRecvError;

pub enum EngineReceives<B: Board> {
    // joins the thread
    Quit,
    Forget,
    SetOption(EngineOptionName, String, Arc<Mutex<EngineInfo>>),
    Search(SearchParams<B>),
    Bench(B, SearchLimit, Arc<Mutex<UgiOutput<B>>>),
    TTEntry(B, Arc<Mutex<UgiOutput<B>>>),
    EvalFor(B, Arc<Mutex<UgiOutput<B>>>),
    SetEval(Box<dyn Eval<B>>),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SearchType {
    Normal,
    Infinite,
    Ponder,
}

impl SearchType {
    pub fn new(ponder: bool, limit: &SearchLimit) -> Self {
        if ponder {
            Ponder
        } else if limit.is_infinite() {
            Infinite
        } else {
            Normal
        }
    }
}

/// The EngineWrapper stores one instance of this, which gets cloned and sent to the main thread on a search
#[derive(Debug, Clone)]
pub struct MainThreadData<B: Board> {
    atomic_search_data: Vec<Arc<AtomicSearchState<B>>>,
    pub output: Arc<Mutex<UgiOutput<B>>>,
    pub engine_info: Arc<Mutex<EngineInfo>>,
    // Not atomic because it doesn't need to be shared across threads: The main search thread sets it at the start
    // and checks if it is set when the search is finished
    pub search_type: SearchType,
}

impl<B: Board> MainThreadData<B> {
    pub fn new_search(&mut self, ponder: bool, limit: &SearchLimit) -> Res<()> {
        if self.atomic_search_data[0].currently_searching() {
            return Err(
                format!("Cannot start a new search with limit '{limit}' because the engine is already searching"),
            );
        }
        self.search_type = SearchType::new(ponder, limit);
        for data in &mut self.atomic_search_data {
            data.reset();
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub enum SearchThreadType<B: Board> {
    Main(MainThreadData<B>),
    #[default]
    /// The simple case of using the engine by itself, without the multithreading adapter, simply to find the best move,
    /// also uses the `Auxiliary` variant because there's no need to output anything.
    Auxiliary,
}

impl<B: Board> SearchThreadType<B> {
    pub fn output(&self) -> Option<MutexGuard<UgiOutput<B>>> {
        match self {
            Main(MainThreadData { output, .. }) => Some(output.lock().unwrap()),
            Auxiliary => None,
        }
    }
}

#[derive(Debug)]
#[repr(align(64))] // Prevent false sharing
pub struct AtomicSearchState<B: Board> {
    // All combinations of should_stop and currently_searching are (briefly) possible.
    // The default is both being false.
    // When it starts searching `searching` gets set to true.
    // When `stop` gets set the engine begins to stop.
    // When it has actually stopped it sets `currently_searching` to false.
    // If it has stopped without receiving a `stop` or reaching a limit
    // (i.e. infinite search has exceeded max depth), both are false.

    // This flag indicates that the engine should stop searching. It can be set by the UGI thread upon receiving a "stop"
    // command, or it can be set by the engine when a limiting stop condition is reached. It is not set upon exceeding the
    // max depth of an infinite search.
    should_stop: AtomicBool,
    // True if the engine is currently searching. Note that if an infinite search reaches its internal end condition but
    // hasn't yet been stopped, this is set to false; the thread may still spin until it receives a stop.
    currently_searching: AtomicBool,
    pub suppress_best_move: AtomicBool,
    edges: AtomicU64,
    depth: AtomicIsize,
    seldepth: AtomicUsize,
    best_move: AtomicUsize,
    ponder_move: AtomicUsize,
    score: AtomicI32,
    phantom_data: PhantomData<B>,
}

impl<B: Board> Default for AtomicSearchState<B> {
    fn default() -> Self {
        Self {
            should_stop: AtomicBool::new(false),
            currently_searching: AtomicBool::new(false),
            suppress_best_move: AtomicBool::new(false),
            edges: AtomicU64::new(0),
            depth: AtomicIsize::new(0),
            seldepth: AtomicUsize::new(0),
            best_move: AtomicUsize::new(B::Move::default().to_underlying().into()),
            ponder_move: AtomicUsize::new(B::Move::default().to_underlying().into()),
            score: AtomicI32::new(NO_SCORE_YET.0),
            phantom_data: PhantomData,
        }
    }
}

impl<B: Board> AtomicSearchState<B> {
    pub fn reset(&self) {
        // all stores can be Relaxed because we're overwriting all members
        self.set_score(NO_SCORE_YET);
        self.set_ponder_move(None);
        self.set_best_move(B::Move::default());
        self.update_seldepth(0);
        self.set_depth(0);
        self.edges.store(0, Relaxed);
        self.set_searching(false);
        self.suppress_best_move.store(false, Relaxed);
        self.should_stop.store(false, Relaxed);
    }

    pub fn stop_flag(&self) -> bool {
        self.should_stop.load(Acquire)
    }

    /// Intended to be used by the search thread, uses Relaxed ordering.
    /// Note that any other thread might want to load with Acquire semantic.
    pub(super) fn currently_searching(&self) -> bool {
        self.currently_searching.load(Relaxed)
    }

    /// Should only be used by the search thread, uses Relaxed ordering. Any other thread should never set this value.
    pub(super) fn set_searching(&self, val: bool) {
        self.currently_searching.store(val, Relaxed);
    }

    pub fn edges(&self) -> u64 {
        self.edges.load(Relaxed)
    }

    pub fn depth(&self) -> Depth {
        Depth::new(self.depth.load(Relaxed) as usize)
    }

    pub fn seldepth(&self) -> Depth {
        Depth::new(self.seldepth.load(Relaxed))
    }

    pub fn score(&self) -> Score {
        Score(self.score.load(Relaxed))
    }

    pub fn best_move(&self) -> B::Move {
        B::Move::from_usize_unchecked(self.best_move.load(Relaxed)).trust_unchecked()
    }

    pub fn ponder_move(&self) -> Option<B::Move> {
        let mov = B::Move::from_usize_unchecked(self.ponder_move.load(Relaxed)).trust_unchecked();
        if mov == B::Move::default() {
            None
        } else {
            Some(mov)
        }
    }

    pub fn set_stop(&self, val: bool) {
        self.should_stop.store(val, Release)
    }

    pub fn count_node(&self) {
        // TODO: Test if using a relaxed load, non-atomic add, and relaxed store is faster
        // (should compile to `add` instead of `lock add` on x86)
        self.edges.fetch_add(1, Relaxed);
    }

    pub fn set_depth(&self, depth: isize) {
        self.depth.store(depth, Relaxed);
    }

    pub fn update_seldepth(&self, current_seldepth: usize) {
        self.seldepth.fetch_max(current_seldepth, Relaxed);
    }

    pub fn set_score(&self, score: Score) {
        debug_assert!(score.verify_valid().is_some());
        self.score.store(score.0, Relaxed);
    }

    pub fn set_best_move(&self, best: B::Move) {
        self.best_move.store(best.to_underlying().into(), Relaxed);
    }

    pub fn set_ponder_move(&self, ponder_move: Option<B::Move>) {
        self.ponder_move.store(
            ponder_move.unwrap_or_default().to_underlying().into(),
            Relaxed,
        );
    }
}

// TODO: Maybe use a thread pool instead and get rid of this class and channels entirely?
// Would mean starting from a clean state for every search, or putting more search state in a struct that outlives the thread
pub struct EngineThread<B: Board, E: Engine<B>> {
    engine: E,
    receiver: Receiver<EngineReceives<B>>,
}

impl<B: Board, E: Engine<B>> EngineThread<B, E> {
    pub fn new(engine: E, receiver: Receiver<EngineReceives<B>>) -> Self {
        Self { engine, receiver }
    }

    fn start_search(&mut self, params: SearchParams<B>) {
        let _ = self.engine.search(params); // the engine takes care of sending the search result
    }

    fn bench_single_position(
        &mut self,
        pos: B,
        limit: SearchLimit,
        output: Arc<Mutex<UgiOutput<B>>>,
    ) {
        // self.engine.stop();
        let res = self.engine.clean_bench(pos, limit);
        output.lock().unwrap().write_ugi(&res.to_string());
    }

    fn get_tt_entry(&mut self, pos: B, output: Arc<Mutex<UgiOutput<B>>>) {
        if let Some(entry) = self
            .engine
            .search_state()
            .tt()
            .load::<B>(pos.zobrist_hash(), 0)
        {
            let msg = format!(
                "move {0} score {1} bound {2} depth {3}",
                entry.mov,
                entry.score,
                entry.bound(),
                entry.depth
            );
            output.lock().unwrap().write_ugi(&msg);
        } else {
            output.lock().unwrap().write_ugi("<none>");
        };
    }

    fn get_static_eval(&mut self, pos: B, output: Arc<Mutex<UgiOutput<B>>>) {
        let eval = self.engine.static_eval(pos);
        output
            .lock()
            .unwrap()
            .write_ugi(&format!("score cp {eval}"));
    }

    fn write_error(&mut self, msg: &str) {
        self.engine.search_state_mut().send_non_ugi(Error, msg);
        eprintln!("Engine thread encountered a fatal error: '{msg}'");
    }

    fn handle_input(&mut self, received: EngineReceives<B>) -> Res<bool> {
        match received {
            Quit => {
                return Ok(true);
            }
            Forget => {
                self.engine.forget();
            }
            SetOption(name, value, info) => match name {
                Threads => panic!("This should have already been handled by the engine owner"),
                _ => {
                    let mut guard = info.lock().unwrap();
                    let Some(val) = guard.options.get_mut(&name) else {
                        return Err(format!("The engine '{0}' doesn't provide the option '{1}', so it can't be set to value '{2}'", guard.engine.short.bold(), name.to_string().red(), value.bold()));
                    };
                    self.engine.set_option(name, val, value)?
                }
            },
            Search(params) => {
                self.start_search(params);
            }
            Bench(pos, limit, output) => self.bench_single_position(pos, limit, output),
            TTEntry(pos, output) => self.get_tt_entry(pos, output),
            EvalFor(pos, output) => self.get_static_eval(pos, output),
            SetEval(eval) => self.engine.set_eval(eval),
        };
        Ok(false)
    }

    pub fn try_handle_input(&mut self) -> Res<bool> {
        match self.receiver.recv() {
            Ok(msg) => self.handle_input(msg),
            Err(_err) => Ok(true),
        }
    }

    pub fn main_loop(&mut self) {
        // do this here so that it's run in the (main) search thread, which means we don't run into multithreading problems
        self.engine.print_spsa_params();
        loop {
            match self.try_handle_input() {
                Err(msg) => {
                    self.write_error(&msg);
                    break;
                }
                Ok(should_quit) => {
                    if should_quit {
                        break;
                    }
                }
            }
        }
        // Exit the main loop, cleaning up all allocated resources
    }
}

#[derive(Debug)]
#[must_use]
pub struct EngineWrapper<B: Board> {
    main: Sender<EngineReceives<B>>,
    auxiliary: Vec<Sender<EngineReceives<B>>>,
    searcher_builder: Box<dyn AbstractSearcherBuilder<B>>,
    eval_builder: Box<dyn AbstractEvalBuilder<B>>,
    main_thread_data: MainThreadData<B>,
    // If we receive a `setoption name Hash` while searching, we only apply that to the next search
    tt_for_next_search: TT,
}

impl<B: Board> Drop for EngineWrapper<B> {
    fn drop(&mut self) {
        _ = self.main.send(Quit);
    }
}

impl<B: Board> EngineWrapper<B> {
    pub fn new(
        tt: TT,
        output: Arc<Mutex<UgiOutput<B>>>,
        searcher_builder: Box<dyn AbstractSearcherBuilder<B>>,
        eval_builder: Box<dyn AbstractEvalBuilder<B>>,
    ) -> Self {
        let atomic = Arc::new(AtomicSearchState::default());
        let (main, info) = searcher_builder.build_in_new_thread(eval_builder.build());
        let main_thread_data = MainThreadData {
            atomic_search_data: vec![atomic],
            output,
            engine_info: Arc::new(Mutex::new(info)),
            search_type: Normal,
        };
        EngineWrapper {
            main,
            auxiliary: vec![],
            searcher_builder,
            eval_builder,
            main_thread_data,
            tt_for_next_search: tt,
        }
    }

    pub fn start_search(
        &mut self,
        pos: B,
        limit: SearchLimit,
        history: ZobristHistory<B>,
        search_moves: Option<Vec<B::Move>>,
        multi_pv: usize,
        ponder: bool,
    ) -> Res<()> {
        self.main_thread_data.new_search(ponder, &limit)?; // resets the atomic search state
        let thread_data = self.main_thread_data.clone();
        let params = SearchParams::create(
            pos,
            limit,
            history.clone(),
            self.tt_for_next_search.clone(),
            search_moves.clone(),
            multi_pv - 1,
            thread_data.atomic_search_data[0].clone(),
            Main(thread_data),
        );
        // reset `stop` first such that a finished ponder command won't print anything
        // self.search_sender().new_search(params.limit.is_infinite());
        self.start_search_with(params)
    }

    fn start_search_with(&mut self, params: SearchParams<B>) -> Res<()> {
        assert_eq!(
            self.main_thread_data.atomic_search_data.len(),
            self.auxiliary.len() + 1
        );
        for (i, o) in &mut self.auxiliary.iter_mut().enumerate() {
            Self::send_start_search(
                o,
                params.auxiliary(self.main_thread_data.atomic_search_data[i + 1].clone()),
            )?;
        }
        Self::send_start_search(&mut self.main, params)
    }

    fn send_start_search(
        sender: &mut Sender<EngineReceives<B>>,
        params: SearchParams<B>,
    ) -> Res<()> {
        debug_assert!(Arc::strong_count(&params.atomic) >= 2);
        sender.send(Search(params)).map_err(|err| err.to_string())
    }

    pub fn set_tt(&mut self, tt: TT) {
        // this sets the TT without overwriting any potential copy used by a search thread
        // (which would only exist when a search thread is currently searching)
        self.tt_for_next_search = tt;
    }

    pub fn next_tt(&self) -> TT {
        self.tt_for_next_search.clone()
    }

    pub fn set_option(&mut self, name: EngineOptionName, value: String) -> Res<()> {
        if name == Threads {
            let count: usize = parse_int_from_str(&value, "num threads")?;
            let max = self.engine_info().max_threads;
            if count == 0 || count > max {
                return Err(format!(
                    "Trying to set the number of threads to {count}. The maximum number of threads for this engine is {max}."
                ));
            }
            self.auxiliary.clear();
            self.auxiliary.resize_with(count - 1, || {
                self.searcher_builder
                    .build_in_new_thread(self.eval_builder.build())
                    .0
            });
            self.main_thread_data.atomic_search_data.truncate(1);
            self.main_thread_data
                .atomic_search_data
                .resize_with(count, || Arc::new(AtomicSearchState::default()));
            Ok(())
        } else if name == Hash {
            let value: usize = parse_int_from_str(&value, "hash size in mb")?;
            let size = value * 1_000_000;
            // first, give back the memory of the old TT to avoid spikes in memory usage
            self.set_tt(TT::minimal());
            self.set_tt(TT::new_with_bytes(size));
            Ok(())
        } else {
            for aux in &mut self.auxiliary {
                aux.send(SetOption(
                    name.clone(),
                    value.clone(),
                    self.main_thread_data.engine_info.clone(),
                ))
                .map_err(|err| err.to_string())?;
            }
            self.main
                .send(SetOption(
                    name,
                    value,
                    self.main_thread_data.engine_info.clone(),
                ))
                .map_err(|err| err.to_string())
        }
    }

    pub fn start_bench(&mut self, pos: B, limit: SearchLimit) -> Res<()> {
        self.main
            .send(Bench(pos, limit, self.main_thread_data.output.clone()))
            .map_err(|err| err.to_string())
    }

    pub fn static_eval(&mut self, pos: B) -> Res<()> {
        self.main
            .send(EvalFor(pos, self.main_thread_data.output.clone()))
            .map_err(|err| err.to_string())
    }

    pub fn tt_entry(&mut self, pos: B) -> Res<()> {
        self.main
            .send(TTEntry(pos, self.main_thread_data.output.clone()))
            .map_err(|err| err.to_string())
    }

    pub fn set_eval(&mut self, eval: Box<dyn Eval<B>>) -> Res<()> {
        for aux in &self.auxiliary {
            aux.send(SetEval(clone_box(eval.as_ref())))
                .map_err(|err| err.to_string())?;
        }
        self.main.send(SetEval(eval)).map_err(|err| err.to_string())
    }

    pub fn send_stop(&mut self, suppress_best_move: bool) {
        if suppress_best_move {
            self.main_thread_data.atomic_search_data[0]
                .suppress_best_move
                .store(true, Release);
        }
        for atomic in &self.main_thread_data.atomic_search_data {
            atomic.set_stop(true);
        }
        for atomic in &self.main_thread_data.atomic_search_data {
            while atomic.currently_searching.load(Acquire) {
                spin_loop(); // this should only take a short while
            }
        }
        if suppress_best_move {
            self.main_thread_data.atomic_search_data[0]
                .suppress_best_move
                .store(false, Release);
        }
    }

    pub fn send_quit(&mut self) -> Res<()> {
        self.send_stop(false);
        for o in &mut self.auxiliary {
            o.send(Quit).map_err(|err| err.to_string())?;
        }
        self.main.send(Quit).map_err(|err| err.to_string())
    }

    pub fn send_forget(&mut self) -> Res<()> {
        for o in &mut self.auxiliary {
            o.send(Forget).map_err(|err| err.to_string())?;
        }
        // tt_for_next_search references the same TT as the TT used during search unless it has been changed with `setoption`
        self.tt_for_next_search.forget();
        self.main.send(Forget).map_err(|err| err.to_string())
    }

    pub fn engine_info(&self) -> MutexGuard<EngineInfo> {
        self.main_thread_data.engine_info.lock().unwrap()
    }

    pub fn num_threads(&self) -> usize {
        self.auxiliary.len() + 1
    }
}
