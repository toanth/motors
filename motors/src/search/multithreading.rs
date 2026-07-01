use crate::eval::Eval;
use crate::io::ugi_output::{pretty_score, score_gradient, UgiOutput};
use crate::search::multithreading::EngineReceives::*;
use crate::search::multithreading::SearchThreadType::{Auxiliary, Main, SingleAndNoOutput};
use crate::search::multithreading::SearchType::{Infinite, Normal, Ponder};
use crate::search::tt::{TTEntry, TT};
use crate::search::{AbstractEvalBuilder, AbstractSearcherBuilder, Engine, EngineInfo, SearchParams};
use crate::send_debug_msg_impl;
use gears::colored::Colorize;
use gears::dyn_clone::clone_box;
use gears::games::ZobristHistory;
use gears::general::board::BoardTrait;
use gears::general::common::anyhow::{anyhow, bail, ensure};
use gears::general::common::{dbg_end_search, dbg_print, dbg_reset, parse_int_from_str, Name, NamedEntity, Res};
use gears::general::moves::ExtendedFormat::Standard;
use gears::general::moves::MoveTrait;
use gears::itertools::Itertools;
use gears::output::Message::*;
use gears::rand::prelude::SmallRng;
use gears::rand::SeedableRng;
use gears::score::{Score, NO_SCORE_YET};
use gears::search::{DepthPly, SearchLimit, SearchResult};
use gears::ugi::EngineOptionName::{Hash, Threads};
use gears::ugi::EngineOptionNameForProtocol;
use std::hint::spin_loop;
use std::marker::PhantomData;
use std::sync::atomic::Ordering::{AcqRel, Acquire, Relaxed, Release, SeqCst};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize};
use std::sync::{Arc, Barrier, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use std::{fmt, iter, thread};

pub type Sender<T> = crossbeam_channel::Sender<T>;
pub type Receiver<T> = crossbeam_channel::Receiver<T>;
pub type TryRecvError = crossbeam_channel::TryRecvError;

pub enum EngineReceives<B: BoardTrait> {
    // joins the thread
    Quit,
    Forget,
    SetOption(EngineOptionNameForProtocol, String, Arc<Mutex<EngineInfo>>),
    Search(SearchParams<B>),
    SetEval(Box<dyn Eval<B>>),
    Print(Arc<Mutex<EngineInfo>>, B),
    PrintMove(Arc<Mutex<EngineInfo>>, B, B::Move),
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
#[derive(Debug)]
pub struct MainThreadData<B: BoardTrait> {
    pub output: Arc<Mutex<UgiOutput<B>>>,
    pub engine_info: Arc<Mutex<EngineInfo>>,
    pub search_type: Mutex<SearchType>,
}

fn new_num_threads_value<B: BoardTrait>(
    count: usize,
    max_threads: usize,
    output: &Arc<Mutex<UgiOutput<B>>>,
) -> Res<usize> {
    ensure!(count > 0, "The number of threads should be between 1 and {max_threads}, not zero");
    if count > max_threads {
        output.lock().unwrap().write_message(Warning, &format_args!(
            "Setting the number of threads to {count} even though this searcher on this machine can only make use of {max_threads} parallel thread(s)"
        ));
    }
    let clamp = if cfg!(feature = "fuzzing") { max_threads * 3 } else { 1 << 20 };
    Ok(count.min(clamp))
}

#[derive(Debug, Default)]
pub enum SearchThreadType<B: BoardTrait> {
    #[default]
    /// The simple case of using the engine by itself, without the multithreading adapter, simply to find the best move
    SingleAndNoOutput,
    /// The engine is called from the UCI interface and runs in a separate thread. There might be auxiliary threads.
    Main(Arc<MainThreadData<B>>),
    /// This is an auxiliary thread, which never needs to output anything and doesn't even have to return a move.
    /// It's only used to write to shared state like the TT or the atomic search state.
    Auxiliary,
}

impl<B: BoardTrait> SearchThreadType<B> {
    pub fn output(&self) -> Option<MutexGuard<'_, UgiOutput<B>>> {
        match self {
            Main(main) => Some(main.output.lock().unwrap()),
            Auxiliary | SingleAndNoOutput => None,
        }
    }
}

#[derive(Debug)]
#[repr(align(64))] // Prevent false sharing
pub struct SharedPerThreadState<B: BoardTrait> {
    nodes: AtomicU64,
    iteration: AtomicUsize,
    seldepth: AtomicUsize,
    best_move: AtomicU64,
    ponder_move: AtomicU64,
    score: AtomicI32,
    phantom_data: PhantomData<B>,
}

impl<B: BoardTrait> Default for SharedPerThreadState<B> {
    fn default() -> Self {
        Self {
            nodes: AtomicU64::new(0),
            iteration: AtomicUsize::new(0),
            seldepth: AtomicUsize::new(0),
            best_move: AtomicU64::new(B::Move::default().to_underlying().into()),
            ponder_move: AtomicU64::new(B::Move::default().to_underlying().into()),
            score: AtomicI32::new(NO_SCORE_YET.0),
            phantom_data: PhantomData,
        }
    }
}

impl<B: BoardTrait> SharedPerThreadState<B> {
    pub fn reset(&self) {
        // all stores can be Relaxed because we're overwriting all members
        self.set_score(NO_SCORE_YET);
        self.set_ponder_move(None);
        self.set_best_move(B::Move::default());
        self.seldepth.store(0, Relaxed);
        self.set_iteration(0);
        self.nodes.store(0, Relaxed);
    }

    pub fn nodes(&self) -> u64 {
        self.nodes.load(Relaxed)
    }

    pub fn iterations(&self) -> DepthPly {
        DepthPly::new(self.iteration.load(Relaxed))
    }

    pub fn seldepth(&self) -> DepthPly {
        DepthPly::new(self.seldepth.load(Relaxed))
    }

    pub fn reset_seldepth(&self) {
        self.seldepth.store(0, Relaxed);
    }

    pub fn score(&self) -> Score {
        Score(self.score.load(Relaxed))
    }

    pub(super) fn get_score(&self) -> &AtomicI32 {
        &self.score
    }

    pub fn best_move(&self) -> B::Move {
        B::Move::from_u64_unchecked(self.best_move.load(Relaxed)).trust_unchecked()
    }

    pub fn ponder_move(&self) -> Option<B::Move> {
        let mov = B::Move::from_u64_unchecked(self.ponder_move.load(Relaxed)).trust_unchecked();
        if mov == B::Move::default() { None } else { Some(mov) }
    }

    pub(super) fn count_node(&self) -> u64 {
        // Using a relaxed load + store instead of `.fetch_add` is still correct: Only the search thread ever changes its nodes.
        // This compiles to `inc` on x86, unlike `fetch_add`, which compiles to `lock inc`
        let n = self.nodes.load(Relaxed) + 1;
        self.nodes.store(n, Relaxed);
        n
    }

    pub(super) fn set_iteration(&self, iteration: usize) {
        self.iteration.store(iteration, Relaxed);
    }

    pub(super) fn update_seldepth(&self, current_seldepth: usize) {
        _ = self.seldepth.fetch_max(current_seldepth, Relaxed);
    }

    pub fn set_score(&self, score: Score) {
        debug_assert!(score.is_valid());
        self.score.store(score.0, Relaxed);
    }

    pub(super) fn set_best_move(&self, best: B::Move) {
        self.best_move.store(best.to_underlying().into(), Relaxed);
    }

    pub(super) fn set_ponder_move(&self, ponder_move: Option<B::Move>) {
        self.ponder_move.store(ponder_move.unwrap_or_default().to_underlying().into(), Relaxed);
    }
}

#[derive(Debug)]
pub struct SharedSearchState<B: BoardTrait> {
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
    pub(super) should_stop: AtomicBool,
    // True if the engine is currently searching. Note that if an infinite search reaches its internal end condition but
    // hasn't yet been stopped, this is set to false; the thread may still spin until it receives a stop.
    pub(super) currently_searching: AtomicBool,
    end_search_barrier: Barrier,
    pub suppress_best_move: AtomicBool,
    // the main thread is at index 0
    shared_per_thread_data: Vec<SharedPerThreadState<B>>,
}

impl<B: BoardTrait> SharedSearchState<B> {
    pub fn new(num_threads: usize) -> Arc<Self> {
        assert!(num_threads > 0);
        let shared_per_thread_data = iter::repeat_with(SharedPerThreadState::default).take(num_threads).collect_vec();
        Arc::new(Self {
            should_stop: AtomicBool::new(false),
            currently_searching: AtomicBool::new(false),
            end_search_barrier: Barrier::new(num_threads),
            suppress_best_move: AtomicBool::new(false),
            shared_per_thread_data,
        })
    }

    pub fn stop_flag(&self) -> bool {
        self.should_stop.load(Acquire)
    }

    pub(super) fn set_stop(&self) {
        self.should_stop.store(true, Release)
    }

    pub fn currently_searching(&self) -> bool {
        self.currently_searching.load(Relaxed)
    }

    pub fn set_searching(&self, val: bool) {
        self.currently_searching.store(val, Relaxed);
    }

    pub fn num_threads(&self) -> usize {
        self.shared_per_thread_data.len()
    }
}

// TODO: add `thread_data()` function to `Engine` trait
#[derive(Debug)]
pub struct ThreadData<B: BoardTrait> {
    pub thread_idx: usize,
    pub thread_type: SearchThreadType<B>,
    pub shared: Arc<SharedSearchState<B>>,
}

impl<B: BoardTrait> ThreadData<B> {
    pub fn single_and_no_output() -> Self {
        let shared = SharedSearchState::new(1);
        Self { thread_idx: 0, thread_type: SingleAndNoOutput, shared }
    }

    pub fn single_threaded(main_thread_data: Arc<MainThreadData<B>>) -> Self {
        let shared = SharedSearchState::new(1);
        Self { thread_idx: 0, thread_type: Main(main_thread_data), shared }
    }

    pub fn this_thread(&self) -> &SharedPerThreadState<B> {
        debug_assert_eq!(self.thread_idx != 0, matches!(self.thread_type, Auxiliary));
        &self.shared.shared_per_thread_data[self.thread_idx]
    }

    pub(super) fn shared_atomic_state(&self) -> &[SharedPerThreadState<B>] {
        self.shared.shared_per_thread_data.as_slice()
    }

    /// this will block if
    /// a) this is a main thread (i.e., it actually outputs), and
    /// b) the search is an infinite search from `go infinite` but not `ponder`, and
    /// c) the search hasn't been canceled yet. It will wait until the search has been canceled.
    /// Auxiliary threads and ponder searches both return instantly from this function, without printing anything.
    /// If the search result has chosen a null move, this instead outputs a warning and a random legal move.
    pub fn end_and_send(&self, res: &mut SearchResult<B>, start_time: Instant) {
        dbg_end_search(self.this_thread().nodes());
        let Main(data) = &self.thread_type else {
            _ = self.shared.end_search_barrier.wait();
            if let SingleAndNoOutput = self.thread_type {
                self.shared.set_searching(false);
            }
            return;
        };
        if *data.search_type.lock().unwrap() == Normal {
            // make sure all auxiliary threads stop as well, no matter which condition made the main thread
            // stop. However, this means that `go nodes n` only applies to the main thread; auxiliary
            // threads may search fewer (but not more) nodes. Depending on the stop condition, it's possible that
            // auxiliary threads have already stopped.
            self.shared.set_stop();
        } else {
            while !self.shared.stop_flag() {
                thread::yield_now();
            }
        }
        send_debug_msg_impl!(
            self,
            "Reached the end search barrier after {} microseconds, waiting for other threads...",
            start_time.elapsed().as_micros()
        );
        _ = self.shared.end_search_barrier.wait();
        send_debug_msg_impl!(
            self,
            "Passed the end search barrier (all threads have finished) after {} microseconds",
            start_time.elapsed().as_micros()
        );
        // on a `ponderhit`, we don't want to print a best move
        if self.shared.suppress_best_move.load(Acquire) {
            self.shared.currently_searching.store(false, SeqCst);
            return;
        }
        let mut output = data.output.lock().unwrap();
        if res.chosen_move == B::Move::default() {
            let mut rng = SmallRng::seed_from_u64(42); // keep everything deterministic
            match res.pos.random_legal_move(&mut rng) {
                None => {
                    output.write_message(Warning, &format_args!("search() called in a position with no legal moves"))
                }
                Some(chosen_move) => {
                    debug_assert!(
                        res.pos.is_move_legal(chosen_move),
                        "{0} {1}",
                        chosen_move.compact_formatter(&res.pos),
                        res.pos
                    );
                    output.write_message(
                        Warning,
                        &format_args!("Engine did not return a best move, playing a random move instead"),
                    );
                    *res = SearchResult::<B>::move_only(chosen_move, res.pos.clone());
                }
            }
        }
        // Do this before setting the searching flag so that we have a result when we claim to have finished.
        // This is useful for being able to unit test search commands through the UCI interface.
        output.previous_search_res = Some(res.clone());
        // Do this before sending 'bestmove' to avoid a race condition:
        // We send bestmove, the GUI sends a new 'go', the uci thread tries to start a new search,
        // but the searching flag is still not unset, so it fails.
        // We use a combined load and store with `AcqRel` to guarantee(?) the correct order.
        _ = self.shared.currently_searching.swap(false, AcqRel);
        debug_assert!(res.chosen_move == B::Move::default() || res.pos.is_move_legal(res.chosen_move));

        output.write_search_res(res);
    }
}

pub struct EngineThread<B: BoardTrait> {
    engine: Box<dyn Engine<B>>,
    receiver: Receiver<EngineReceives<B>>,
}

impl<B: BoardTrait> EngineThread<B> {
    pub fn new(engine: Box<dyn Engine<B>>, receiver: Receiver<EngineReceives<B>>) -> Self {
        Self { engine, receiver }
    }

    fn search(&mut self, params: SearchParams<B>) {
        let _ = self.engine.search(params); // the engine takes care of sending the search result
    }

    fn write_error(&mut self, msg: &fmt::Arguments) {
        self.engine.search_state_mut_dyn().send_non_ugi(Error, msg);
        // the above only prints if this is the main search thread, but this prints always (but doesn't get logged)
        eprintln!("Engine thread encountered an error: '{msg}'");
    }

    fn handle_input(&mut self, received: EngineReceives<B>) -> Res<bool> {
        match received {
            Search(params) => {
                dbg_reset();
                self.search(params);
                dbg_print();
            }
            Quit => {
                return Ok(true);
            }
            Forget => {
                self.engine.forget();
            }
            SetOption(opt, value, info) => match opt.name {
                Threads => panic!("This should have already been handled by the engine owner"),
                _ => {
                    let mut guard = info.lock().unwrap();
                    let Some(val) = guard.options.get_mut(&opt) else {
                        bail!(
                            "The engine '{0}' doesn't provide the option '{1}', so it can't be set to value '{2}'",
                            guard.engine.short_name().bold(),
                            opt.to_string().red(),
                            value.bold()
                        );
                    };
                    self.engine.set_option(opt, val, value)?
                }
            },
            SetEval(eval) => self.engine.set_eval(eval),
            Print(engine_info, pos) => {
                let state_info = self.engine.search_state_dyn().write_internal_info(&pos);
                let info = state_info.unwrap_or_else(|| {
                    format!(
                        "The engine '{}' doesn't support printing internal engine information.",
                        self.engine.short_name()
                    )
                });
                engine_info.lock().unwrap().internal_state_description = Some(info);
            }
            PrintMove(engine_info, pos, mov) => {
                let name = self.engine.long_name();
                let Some(eval) = self.engine.get_eval() else {
                    self.engine
                        .search_state_mut_dyn()
                        .send_ugi(&format_args!("The engine {name} does not support evaluating moves",));
                    return Ok(false);
                };
                let mut eval = clone_box(eval);
                let us = pos.active_player();
                let new_pos = pos.clone().make_move(mov).expect("Pseudolegal but not a legal move");
                let new_score = eval.eval(&new_pos, 0, us);
                let old_score = eval.eval(&pos, 0, us);
                let gradient = score_gradient();
                let old_score_str = pretty_score(old_score, None, None, &gradient, true, false);
                let new_score_str = pretty_score(new_score, None, Some(old_score), &gradient, true, false);
                let move_str = mov.extended_formatter(&pos, Standard, None);
                let mut res = format!(
                    "Static Eval before '{move_str}': {old_score_str}\nStatic Eval after '{move_str}': {new_score_str}"
                );
                if let Some(str) = self.engine.eval_move(&pos, mov) {
                    res += "\n";
                    res += &str;
                }
                engine_info.lock().unwrap().internal_state_description = Some(res);
            }
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
                    self.write_error(&format_args!("{msg}"));
                    // continue as normal
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
pub struct EngineWrapper<B: BoardTrait> {
    senders: Vec<Sender<EngineReceives<B>>>,
    searcher_builder: Box<dyn AbstractSearcherBuilder<B>>,
    eval_builder: Box<dyn AbstractEvalBuilder<B>>,
    pub shared_state: Arc<SharedSearchState<B>>,
    main_thread_data: Arc<MainThreadData<B>>,
    // If we receive a `setoption name Hash` while searching, we only apply that to the next search
    tt_for_next_search: TT,
    // It's possible to temporarily add or remove threads. Then this value is used to remember the old number of threads,
    // so that on a new search we can reset the number of threads
    overwrite_num_threads: Option<usize>,
}

impl<B: BoardTrait> Drop for EngineWrapper<B> {
    fn drop(&mut self) {
        self.tear_down_threads();
    }
}

impl<B: BoardTrait> EngineWrapper<B> {
    pub fn new(
        tt: TT,
        output: Arc<Mutex<UgiOutput<B>>>,
        searcher_builder: Box<dyn AbstractSearcherBuilder<B>>,
        eval_builder: Box<dyn AbstractEvalBuilder<B>>,
    ) -> Self {
        let main_thread_data = Arc::new(MainThreadData {
            output,
            engine_info: Arc::new(Mutex::new(EngineInfo::invalid())),
            search_type: Mutex::new(Normal),
        });
        let thread_data = ThreadData::single_threaded(main_thread_data.clone());
        let (sender, shared_state) = searcher_builder.build_in_new_thread(thread_data, eval_builder.build());
        let senders = vec![sender];
        Self {
            senders,
            searcher_builder,
            eval_builder,
            shared_state,
            main_thread_data,
            tt_for_next_search: tt,
            overwrite_num_threads: None,
        }
    }

    pub fn resize_threads(&mut self, count: usize) {
        assert!(count > 0);
        assert_eq!(self.senders.len(), self.shared_state.num_threads());
        if count == self.senders.len() {
            return;
        }
        self.tear_down_threads();
        self.senders.clear();
        let main_thread_data = Arc::new(MainThreadData {
            output: self.main_thread_data.output.clone(),
            engine_info: self.get_engine_info_arc(),
            search_type: Mutex::new(Normal),
        });
        let shared = SharedSearchState::new(count);
        let thread_data_main_thread =
            ThreadData { thread_idx: 0, thread_type: Main(main_thread_data), shared: shared.clone() };
        let eval = self.eval_builder.build();
        let main_sender = self.searcher_builder.build_in_new_thread(thread_data_main_thread, eval).0;
        self.senders.push(main_sender);
        for i in 1..count {
            let eval = self.eval_builder.build();
            let thread_data = ThreadData { thread_idx: i, thread_type: Auxiliary, shared: shared.clone() };
            let sender = self.searcher_builder.build_in_new_thread(thread_data, eval).0;
            self.senders.push(sender);
        }
        self.shared_state = shared;
        debug_assert_eq!(self.senders.len(), self.shared_state.num_threads());
    }

    fn tear_down_threads(&mut self) {
        let start_time = Instant::now();
        self.shared_state.set_stop();
        for sender in &mut self.senders {
            _ = sender.send(Quit);
        }
        while self.shared_state.currently_searching() {
            spin_loop();
            if start_time.elapsed() > Duration::from_millis(500) {
                eprintln!("Warning: Engine hasn't stopped 500ms after being told to quit");
                break;
            }
        }
    }

    fn main_thread_data(&self) -> &MainThreadData<B> {
        &self.main_thread_data
    }

    #[allow(clippy::too_many_arguments)]
    pub fn start_search(
        &mut self,
        pos: B,
        limit: SearchLimit,
        history: ZobristHistory,
        search_moves: Option<Vec<B::Move>>,
        multi_pv: usize,
        is_ponder: bool,
        threads: Option<usize>,
        tt: Option<TT>,
        contempt: Score,
    ) -> Res<()> {
        if let Some(n) = self.overwrite_num_threads {
            self.resize_threads(n);
            self.overwrite_num_threads = None;
        }
        let threads = match threads {
            None => self.num_threads(),
            Some(t) => {
                let max_threads = self.get_engine_info().max_threads();
                let t = new_num_threads_value(t, max_threads, &self.main_thread_data.output)?;
                let current = self.num_threads();
                self.overwrite_num_threads = Some(current);
                self.resize_threads(t);
                t
            }
        };

        if self.shared_state.currently_searching() {
            bail!("Cannot start a new search with limit '{limit}' because the engine is already searching");
        }
        self.shared_state.should_stop.store(false, Release);
        *self.main_thread_data.search_type.lock().unwrap() = SearchType::new(is_ponder, &limit);
        self.tt_for_next_search.age.increment();
        let tt = tt.unwrap_or(self.tt_for_next_search.clone());
        let params =
            SearchParams::create(pos, limit, history, tt, search_moves.clone(), multi_pv.saturating_sub(1), contempt);
        debug_assert_eq!(self.senders.len(), threads);
        self.start_search_with(params)
    }

    fn start_search_with(&mut self, params: SearchParams<B>) -> Res<()> {
        assert_eq!(self.shared_state.shared_per_thread_data.len(), self.senders.len());
        // Make sure that a `wait` command immediately following this `go` command will block until the search has finished.
        self.shared_state.set_searching(true);
        for o in &mut self.senders {
            debug_assert!(Arc::strong_count(&self.shared_state) >= 2);
            Self::send_start_search(o, params.clone())?;
        }
        Ok(())
    }

    fn send_start_search(sender: &mut Sender<EngineReceives<B>>, params: SearchParams<B>) -> Res<()> {
        sender.send(Search(params)).map_err(|err| anyhow!(err.to_string()))
    }

    pub fn set_tt(&mut self, tt: TT) {
        // this sets the TT without overwriting any potential copy used by a search thread
        // (which would only exist when a search thread is currently searching)
        self.tt_for_next_search = tt;
    }

    pub fn next_tt(&self) -> TT {
        self.tt_for_next_search.clone()
    }

    pub fn set_option(&mut self, opt: EngineOptionNameForProtocol, value: String) -> Res<()> {
        if opt.name == Threads {
            let count: usize = parse_int_from_str(&value, "num threads")?;
            let max = self.get_engine_info().max_threads;
            let count = new_num_threads_value(count, max, &self.main_thread_data.output)?;
            self.overwrite_num_threads = None;
            self.resize_threads(count);
        } else if opt.name == Hash {
            let size: usize = parse_int_from_str(&value, "hash size in MiB")?;
            // first, give back the memory of the old TT to avoid spikes in memory usage
            self.set_tt(TT::minimal());
            self.set_tt(TT::new_with_mib(size));
        } else {
            for aux in &mut self.senders {
                aux.send(SetOption(opt.clone(), value.clone(), self.main_thread_data.engine_info.clone()))
                    .map_err(|err| anyhow!(err.to_string()))?;
            }
        }
        Ok(())
    }

    pub fn tt_entry(&mut self, pos: &B) -> Option<TTEntry<B>> {
        self.tt_for_next_search.load(pos, 0)
    }

    pub fn tt(&mut self) -> TT {
        self.tt_for_next_search.clone()
    }

    pub fn set_eval(&mut self, eval: Box<dyn Eval<B>>) -> Res<()> {
        for aux in &self.senders {
            aux.send(SetEval(clone_box(eval.as_ref()))).map_err(|err| anyhow!(err.to_string()))?;
        }
        self.get_engine_info().eval = Some(Name::new(eval.as_ref()));
        Ok(())
    }

    pub fn send_print(&self, pos: B) -> Res<()> {
        self.senders[0].send(Print(self.get_engine_info_arc(), pos)).map_err(|err| anyhow!(err.to_string()))
    }

    pub fn send_print_move(&self, pos: B, mov: B::Move) -> Res<()> {
        self.senders[0].send(PrintMove(self.get_engine_info_arc(), pos, mov)).map_err(|err| anyhow!(err.to_string()))
    }

    pub fn send_stop(&mut self, suppress_best_move: bool) {
        if suppress_best_move {
            self.shared_state.suppress_best_move.store(true, Release);
        }
        self.shared_state.set_stop();
        // TODO: Use barrier for making sure all threads finish searching
        while self.shared_state.currently_searching.load(Acquire) {
            debug_assert!(self.shared_state.stop_flag());
            thread::yield_now(); // this should only take a short while
        }
        // At this point, the engine threat has already read this flag and decided not to print the best move
        if suppress_best_move {
            self.shared_state.suppress_best_move.store(false, Release);
        }
        // it's possible that the current search had been done with a different number of threads, so remove superfluous entries
        self.resize_threads(self.num_threads());
        self.overwrite_num_threads = None;
    }

    pub fn send_quit(&mut self) -> Res<()> {
        self.send_stop(false);
        for o in &mut self.senders {
            o.send(Quit).map_err(|err| anyhow!(err.to_string()))?;
        }
        Ok(())
    }

    pub fn send_forget(&mut self) -> Res<()> {
        // tt_for_next_search references the same TT as the TT used during search unless it has been changed with `setoption`
        self.tt_for_next_search.forget();
        for o in &mut self.senders {
            o.send(Forget).map_err(|err| anyhow!(err.to_string()))?;
        }
        Ok(())
    }

    pub fn get_engine_info(&self) -> MutexGuard<'_, EngineInfo> {
        self.main_thread_data.engine_info.lock().unwrap()
    }

    pub fn get_engine_info_arc(&self) -> Arc<Mutex<EngineInfo>> {
        self.main_thread_data().engine_info.clone()
    }

    pub fn num_threads(&self) -> usize {
        if let Some(num) = self.overwrite_num_threads { num } else { self.senders.len() }
    }

    // pub fn main_atomic_search_data(&self) -> Arc<AtomicSearchState<B>> {
    //     self.main_thread_data.atomic_search_data[0].clone()
    // }
}

#[cfg(test)]
mod tests {
    use crate::create_match;
    use crate::io::cli::EngineOpts;
    use gears::cli::Game;
    use gears::cli::Game::Chess;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn start_search_test() {
        let opts = EngineOpts::for_game(Game::default(), false);
        let mut ugi = create_match(opts).unwrap();
        _ = ugi.handle_input("go mate 9999999").unwrap_err();
        ugi.handle_input("go").unwrap();
        ugi.handle_input("random_pos").unwrap();
        ugi.handle_input("stop").unwrap();
        ugi.handle_input("go mate 789").unwrap();
        let res = ugi.handle_input("go");
        assert!(res.is_err());
        ugi.handle_input("stop").unwrap();
        ugi.handle_input("go bench 1").unwrap();
        ugi.handle_input("wait").unwrap();
        ugi.handle_input("go wtime 1 btime 1").unwrap();
        ugi.handle_input("stop").unwrap();
        ugi.quit().unwrap();
    }

    #[test]
    #[cfg(feature = "chess")]
    fn immediate_response_test() {
        let opts = EngineOpts::for_game(Chess, true);
        let mut ugi = create_match(opts).unwrap();
        ugi.handle_input("go").unwrap();
        ugi.handle_input("random_pos").unwrap();
        ugi.handle_input("stop").unwrap();
        ugi.handle_input("go").unwrap();
        let res = ugi.handle_input("go");
        assert!(res.is_err());
        ugi.handle_input("stop").unwrap();
        ugi.handle_input("go bench 1").unwrap();
        ugi.handle_input("wait").unwrap();
        ugi.handle_input("go wtime 1 btime 1").unwrap();
        ugi.handle_input("stop").unwrap();
        ugi.quit().unwrap();
    }

    #[test]
    #[cfg(feature = "chess")]
    fn set_options_during_match() {
        let opts = EngineOpts::for_game(Chess, true);
        let mut ugi = create_match(opts).unwrap();
        ugi.handle_input("go").unwrap();
        ugi.handle_input("random_pos").unwrap();
        ugi.handle_input("setoption name Hash value 1").unwrap();
        ugi.handle_input("setoption uci_chEss960 on").unwrap();
        ugi.handle_input("position startpos moves e2e4").unwrap();
        ugi.handle_input("setoption name Engine value random").unwrap();
        ugi.handle_input("stop").unwrap();
        ugi.handle_input("go").unwrap();
        ugi.handle_input("stop").unwrap();
    }

    #[test]
    fn ponder_test() {
        let opts = EngineOpts::for_game(Game::default(), false);
        let mut ugi = create_match(opts).unwrap();
        ugi.handle_input("go ponder").unwrap();
        ugi.handle_input("stop").unwrap();
        ugi.handle_input("go ponder").unwrap();
        let res = ugi.handle_input("go ponder");
        assert!(res.is_err());
        ugi.handle_input("ponderhit").unwrap();
        ugi.handle_input("stop").unwrap();
        ugi.handle_input("go ponder nodes 100").unwrap();
        ugi.handle_input("ponderhit").unwrap();
        ugi.handle_input("wait").unwrap();
        let res = ugi.handle_input("ponderhit");
        assert!(res.is_err());
        ugi.quit().unwrap();
    }

    #[test]
    fn multithreaded_search_test() {
        let opts = EngineOpts::for_game(Game::default(), false);
        let mut ugi = create_match(opts).unwrap();
        ugi.handle_input("go t 2").unwrap();
        ugi.handle_input("stop").unwrap();
        ugi.handle_input("so Threads 3").unwrap();
        ugi.handle_input("go").unwrap();
        let res = ugi.handle_input("go");
        assert!(res.is_err());
        ugi.handle_input("stop").unwrap();
        ugi.handle_input("stop").unwrap();
        ugi.handle_input("g t 1").unwrap();
        thread::sleep(Duration::from_millis(200));
        ugi.quit().unwrap();
    }
}
