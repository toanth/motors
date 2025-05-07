use crate::eval::Eval;
use crate::io::ugi_output::UgiOutput;
use crate::search::multithreading::EngineReceives::*;
use crate::search::multithreading::SearchThreadType::{Auxiliary, Main};
use crate::search::multithreading::SearchType::{Infinite, Normal, Ponder};
use crate::search::tt::{TT, TTEntry};
use crate::search::{AbstractEvalBuilder, AbstractSearcherBuilder, Engine, EngineInfo, SearchParams};
use gears::colored::Colorize;
use gears::dyn_clone::clone_box;
use gears::games::ZobristHistory;
use gears::general::board::Board;
use gears::general::common::anyhow::{anyhow, bail, ensure};
use gears::general::common::{Name, NamedEntity, Res, parse_int_from_str};
use gears::general::moves::Move;
use gears::output::Message::*;
use gears::score::{NO_SCORE_YET, Score};
use gears::search::{Depth, SearchLimit};
use gears::ugi::EngineOptionName;
use gears::ugi::EngineOptionName::{Hash, Threads};
use std::fmt;
use std::hint::spin_loop;
use std::marker::PhantomData;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicIsize, AtomicU64, AtomicUsize};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub type Sender<T> = crossbeam_channel::Sender<T>;
pub type Receiver<T> = crossbeam_channel::Receiver<T>;
pub type TryRecvError = crossbeam_channel::TryRecvError;

pub enum EngineReceives<B: Board> {
    // joins the thread
    Quit,
    Forget,
    SetOption(EngineOptionName, String, Arc<Mutex<EngineInfo>>),
    Search(SearchParams<B>),
    SetEval(Box<dyn Eval<B>>),
    Print(Arc<Mutex<EngineInfo>>, B),
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
            bail!("Cannot start a new search with limit '{limit}' because the engine is already searching");
        }
        self.search_type = SearchType::new(ponder, limit);
        for data in &mut self.atomic_search_data {
            data.reset(true);
        }
        Ok(())
    }

    pub(super) fn shared_atomic_state(&self) -> &[Arc<AtomicSearchState<B>>] {
        self.atomic_search_data.as_slice()
    }
}

fn set_num_threads<B: Board>(count: usize, max_threads: usize, output: &Arc<Mutex<UgiOutput<B>>>) -> Res<usize> {
    ensure!(count > 0, "The number of threads should be between 1 and {max_threads}, not zero");
    if count > max_threads {
        output.lock().unwrap().write_message(Warning, &format_args!(
            "Setting the number of threads to {count} even though this engine on this machine can only make use of {} parallel thread(s)",
            max_threads
        ));
    }
    Ok(count.min(1 << 20))
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
    pub(super) fn new_single_thread(
        output: Arc<Mutex<UgiOutput<B>>>,
        engine_info: Arc<Mutex<EngineInfo>>,
        atomic: Arc<AtomicSearchState<B>>,
    ) -> Self {
        let data = MainThreadData { atomic_search_data: vec![atomic], output, engine_info, search_type: Normal };
        Main(data)
    }

    pub fn num_threads(&self) -> Option<usize> {
        match self {
            Main(data) => Some(data.atomic_search_data.len()),
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
    pub(super) currently_searching: AtomicBool,
    pub suppress_best_move: AtomicBool,
    nodes: AtomicU64,
    depth: AtomicIsize,
    seldepth: AtomicUsize,
    best_move: AtomicU64,
    ponder_move: AtomicU64,
    score: AtomicI32,
    phantom_data: PhantomData<B>,
}

impl<B: Board> Default for AtomicSearchState<B> {
    fn default() -> Self {
        Self {
            should_stop: AtomicBool::new(false),
            currently_searching: AtomicBool::new(false),
            suppress_best_move: AtomicBool::new(false),
            nodes: AtomicU64::new(0),
            depth: AtomicIsize::new(0),
            seldepth: AtomicUsize::new(0),
            best_move: AtomicU64::new(B::Move::default().to_underlying().into()),
            ponder_move: AtomicU64::new(B::Move::default().to_underlying().into()),
            score: AtomicI32::new(NO_SCORE_YET.0),
            phantom_data: PhantomData,
        }
    }
}

impl<B: Board> AtomicSearchState<B> {
    // called on 'ucinewgame' and on starting a new search
    pub fn reset(&self, starting_search: bool) {
        // all stores can be Relaxed because we're overwriting all members
        self.set_score(NO_SCORE_YET);
        self.set_ponder_move(None);
        self.set_best_move(B::Move::default());
        self.seldepth.store(0, Relaxed); // don't use `update_seldepth` as that uses `fetch_max`.
        self.set_depth(0);
        self.nodes.store(0, Relaxed);
        self.set_searching(starting_search);
        self.suppress_best_move.store(false, Relaxed);
        self.should_stop.store(false, Relaxed);
    }

    pub fn stop_flag(&self) -> bool {
        self.should_stop.load(Acquire)
    }

    /// Intended to be used by the search thread, uses Relaxed ordering.
    /// Note that any other thread might want to load with Acquire semantic.
    pub fn currently_searching(&self) -> bool {
        self.currently_searching.load(Relaxed)
    }

    /// Should only be used by the search thread, uses Relaxed ordering. Any other thread should never set this value
    /// (but could read it).
    pub fn set_searching(&self, val: bool) {
        self.currently_searching.store(val, Relaxed);
    }

    pub fn nodes(&self) -> u64 {
        self.nodes.load(Relaxed)
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

    pub(super) fn get_score_t(&self) -> &AtomicI32 {
        &self.score
    }

    pub fn best_move(&self) -> B::Move {
        B::Move::from_u64_unchecked(self.best_move.load(Relaxed)).trust_unchecked()
    }

    pub fn ponder_move(&self) -> Option<B::Move> {
        let mov = B::Move::from_u64_unchecked(self.ponder_move.load(Relaxed)).trust_unchecked();
        if mov == B::Move::default() { None } else { Some(mov) }
    }

    pub(super) fn set_stop(&self, val: bool) {
        self.should_stop.store(val, Release)
    }

    pub(super) fn count_node(&self) -> u64 {
        // TODO: Test if using a relaxed load, non-atomic add, and relaxed store is faster
        // (should compile to `add` instead of `lock add` on x86)
        self.nodes.fetch_add(1, Relaxed) + 1
    }

    pub(super) fn set_depth(&self, depth: isize) {
        self.depth.store(depth, Relaxed);
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

pub struct EngineThread<B: Board> {
    engine: Box<dyn Engine<B>>,
    receiver: Receiver<EngineReceives<B>>,
}

impl<B: Board> EngineThread<B> {
    pub fn new(engine: Box<dyn Engine<B>>, receiver: Receiver<EngineReceives<B>>) -> Self {
        Self { engine, receiver }
    }

    fn search(&mut self, params: SearchParams<B>) {
        let _ = self.engine.search(params); // the engine takes care of sending the search result
    }

    fn write_error(&mut self, msg: &fmt::Arguments) {
        self.engine.search_state_mut_dyn().send_non_ugi(Error, msg);
        eprintln!("Engine thread encountered an error: '{msg}'");
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
                        bail!(
                            "The engine '{0}' doesn't provide the option '{1}', so it can't be set to value '{2}'",
                            guard.engine.short_name().bold(),
                            name.to_string().red(),
                            value.bold()
                        );
                    };
                    self.engine.set_option(name, val, value)?
                }
            },
            Search(params) => {
                self.search(params);
            }
            SetEval(eval) => self.engine.set_eval(eval),
            Print(engine_info, pos) => {
                let state_info = self.engine.search_state_dyn().write_internal_info(&pos);
                let info = state_info.unwrap_or_else(|| {
                    format!(
                        "The engine {} doesn't support printing internal engine information.",
                        self.engine.short_name()
                    )
                });
                engine_info.lock().unwrap().internal_state_description = Some(info);
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
pub struct EngineWrapper<B: Board> {
    main: Sender<EngineReceives<B>>,
    auxiliary: Vec<Sender<EngineReceives<B>>>,
    searcher_builder: Box<dyn AbstractSearcherBuilder<B>>,
    eval_builder: Box<dyn AbstractEvalBuilder<B>>,
    main_thread_data: MainThreadData<B>,
    // If we receive a `setoption name Hash` while searching, we only apply that to the next search
    tt_for_next_search: TT,
    // It's possible to temporarily add or remove threads
    overwrite_num_threads: Option<usize>,
}

impl<B: Board> Drop for EngineWrapper<B> {
    fn drop(&mut self) {
        self.main_atomic_search_data().set_stop(true);
        _ = self.main.send(Quit);
        let start_time = Instant::now();
        while self.main_atomic_search_data().currently_searching() {
            spin_loop();
            if start_time.elapsed() > Duration::from_millis(500) {
                eprintln!("Warning: Engine hasn't stopped 500ms after being told to quit");
                break;
            }
        }
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
            overwrite_num_threads: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn start_search(
        &mut self,
        pos: B,
        limit: SearchLimit,
        history: ZobristHistory,
        search_moves: Option<Vec<B::Move>>,
        multi_pv: usize,
        ponder: bool,
        threads: Option<usize>,
        tt: Option<TT>,
    ) -> Res<()> {
        self.resize_threads(self.num_threads());
        self.overwrite_num_threads = None;
        let threads = match threads {
            None => self.num_threads(),
            Some(t) => {
                let max_threads = self.get_engine_info().max_threads();
                let t = set_num_threads(t, max_threads, &self.main_thread_data.output)?;
                let current = self.num_threads();
                self.overwrite_num_threads = Some(current);
                if t > current {
                    self.resize_threads(t);
                }
                t
            }
        };
        self.main_thread_data.new_search(ponder, &limit)?; // resets the atomic search state
        let thread_data = self.main_thread_data.clone();
        self.tt_for_next_search.age.increment();
        let tt = tt.unwrap_or(self.tt_for_next_search.clone());
        let params = SearchParams::create(
            pos,
            limit,
            history,
            tt,
            search_moves.clone(),
            multi_pv.saturating_sub(1),
            thread_data.atomic_search_data[0].clone(),
            Main(thread_data),
        );
        // reset `stop` first such that a finished ponder command won't print anything
        // self.search_sender().new_search(params.limit.is_infinite());
        self.start_search_with(params, threads)
    }

    fn start_search_with(&mut self, params: SearchParams<B>, threads: usize) -> Res<()> {
        assert_eq!(self.main_thread_data.atomic_search_data.len(), self.auxiliary.len() + 1);
        for (i, o) in &mut self.auxiliary.iter_mut().enumerate().take(threads - 1) {
            Self::send_start_search(o, params.auxiliary(self.main_thread_data.atomic_search_data[i + 1].clone()))?;
        }
        Self::send_start_search(&mut self.main, params)
    }

    fn send_start_search(sender: &mut Sender<EngineReceives<B>>, params: SearchParams<B>) -> Res<()> {
        debug_assert!(Arc::strong_count(&params.atomic) >= 2);
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

    pub fn resize_threads(&mut self, count: usize) {
        self.auxiliary
            .resize_with(count - 1, || self.searcher_builder.build_in_new_thread(self.eval_builder.build()).0);
        self.main_thread_data.atomic_search_data.resize_with(count, || Arc::new(AtomicSearchState::default()));
    }

    pub fn set_option(&mut self, name: EngineOptionName, value: String) -> Res<()> {
        if name == Threads {
            let count: usize = parse_int_from_str(&value, "num threads")?;
            let max = self.get_engine_info().max_threads;
            let count = set_num_threads(count, max, &self.main_thread_data.output)?;
            self.overwrite_num_threads = None;
            self.resize_threads(count);
            Ok(())
        } else if name == Hash {
            let size: usize = parse_int_from_str(&value, "hash size in MiB")?;
            // first, give back the memory of the old TT to avoid spikes in memory usage
            self.set_tt(TT::minimal());
            self.set_tt(TT::new_with_mib(size));
            Ok(())
        } else {
            for aux in &mut self.auxiliary {
                aux.send(SetOption(name.clone(), value.clone(), self.main_thread_data.engine_info.clone()))
                    .map_err(|err| anyhow!(err.to_string()))?;
            }
            self.main
                .send(SetOption(name, value, self.main_thread_data.engine_info.clone()))
                .map_err(|err| anyhow!(err.to_string()))
        }
    }

    pub fn tt_entry(&mut self, pos: &B) -> Option<TTEntry<B>> {
        self.tt_for_next_search.load(pos.hash_pos(), 0)
    }

    pub fn set_eval(&mut self, eval: Box<dyn Eval<B>>) -> Res<()> {
        for aux in &self.auxiliary {
            aux.send(SetEval(clone_box(eval.as_ref()))).map_err(|err| anyhow!(err.to_string()))?;
        }
        self.get_engine_info().eval = Some(Name::new(eval.as_ref()));
        self.main.send(SetEval(eval)).map_err(|err| anyhow!(err.to_string()))
    }

    pub fn send_print(&self, pos: B) -> Res<()> {
        self.main.send(Print(self.get_engine_info_arc(), pos)).map_err(|err| anyhow!(err.to_string()))
    }

    pub fn send_stop(&mut self, suppress_best_move: bool) {
        if suppress_best_move {
            self.main_thread_data.atomic_search_data[0].suppress_best_move.store(true, Release);
        }
        for atomic in &self.main_thread_data.atomic_search_data {
            atomic.set_stop(true);
        }
        for atomic in &self.main_thread_data.atomic_search_data {
            while atomic.currently_searching.load(Acquire) {
                spin_loop(); // this should only take a short while
            }
        }
        // At this point, the engine threat has already read this flag and decided not to print the best move
        if suppress_best_move {
            self.main_thread_data.atomic_search_data[0].suppress_best_move.store(false, Release);
        }
        // it's possible that the current search had been done with a different number of threads, so remove superfluous entries
        self.resize_threads(self.num_threads());
        self.overwrite_num_threads = None;
    }

    pub fn send_quit(&mut self) -> Res<()> {
        self.send_stop(false);
        for o in &mut self.auxiliary {
            o.send(Quit).map_err(|err| anyhow!(err.to_string()))?;
        }
        self.main.send(Quit).map_err(|err| anyhow!(err.to_string()))
    }

    pub fn send_forget(&mut self) -> Res<()> {
        for o in &mut self.auxiliary {
            o.send(Forget).map_err(|err| anyhow!(err.to_string()))?;
        }
        // tt_for_next_search references the same TT as the TT used during search unless it has been changed with `setoption`
        self.tt_for_next_search.forget();
        self.main.send(Forget).map_err(|err| anyhow!(err.to_string()))
    }

    pub fn get_engine_info(&self) -> MutexGuard<EngineInfo> {
        self.main_thread_data.engine_info.lock().unwrap()
    }

    pub fn get_engine_info_arc(&self) -> Arc<Mutex<EngineInfo>> {
        self.main_thread_data.engine_info.clone()
    }

    pub fn num_threads(&self) -> usize {
        if let Some(num) = self.overwrite_num_threads { num } else { self.auxiliary.len() + 1 }
    }

    pub fn main_atomic_search_data(&self) -> Arc<AtomicSearchState<B>> {
        self.main_thread_data.atomic_search_data[0].clone()
    }
}

#[cfg(test)]
mod tests {
    use crate::create_match;
    use crate::io::cli::EngineOpts;
    use gears::cli::Game;
    use gears::cli::Game::Chess;

    #[test]
    fn start_search_test() {
        let opts = EngineOpts::for_game(Game::default(), false);
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
        ugi.quit().unwrap();
    }
}
