use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::{Arc, Mutex};
use std::thread::spawn;

use crossbeam_channel::unbounded;
use dyn_clone::clone_box;

use crate::eval::Eval;
use gears::games::{Board, ZobristHistory};
use gears::general::common::{parse_int_from_str, NamedEntity, Res};
use gears::output::Message;
use gears::output::Message::{Debug, Error};
use gears::score::Score;
use gears::search::{Depth, SearchInfo, SearchLimit, SearchResult};
use gears::ugi::EngineOptionName::{Hash, Threads};
use gears::ugi::{EngineOption, EngineOptionName};

use crate::search::multithreading::EngineReceives::*;
use crate::search::statistics::{Statistics, Summary};
use crate::search::tt::TT;
use crate::search::{BenchResult, Engine, EngineBuilder, EngineInfo, SearchState};
use crate::ugi_engine::UgiOutput;

pub type Sender<T> = crossbeam_channel::Sender<T>;
pub type Receiver<T> = crossbeam_channel::Receiver<T>;
pub type TryRecvError = crossbeam_channel::TryRecvError;

pub enum EngineReceives<B: Board> {
    // joins the thread
    Quit,
    Forget,
    SetOption(EngineOptionName, String),
    Search(B, SearchLimit, ZobristHistory<B>, TT, Vec<B::Move>),
    Bench(B, Depth),
    EvalFor(B),
    SetEval(Box<dyn Eval<B>>),
}

#[derive(Debug, Default)]
struct SearchSenderState {
    searching: AtomicBool,
    infinite: AtomicBool,
    stop: AtomicBool,
    print_result: AtomicBool,
}

/// A search sender is used for communication while the search is ongoing.
/// This is therefore necessarily a part of the engine's interface, unlike the Engine thread, which only
/// deals with starting searches and returning the results across threads, and is therefore unnecessary if
/// the engine is used as a library.
#[derive(Debug, Clone)]
pub struct SearchSender<B: Board> {
    output: Option<Arc<Mutex<UgiOutput<B>>>>,
    sss: Arc<SearchSenderState>,
}

impl<B: Board> SearchSender<B> {
    pub fn new(output: Arc<Mutex<UgiOutput<B>>>) -> Self {
        Self {
            output: Some(output),
            sss: Arc::new(SearchSenderState::default()),
        }
    }

    pub fn no_sender() -> Self {
        Self {
            output: None,
            sss: Arc::new(SearchSenderState::default()),
        }
    }

    pub fn search_infinite(&mut self, infinite: bool) {
        self.sss.infinite.store(infinite, SeqCst);
        self.sss.print_result.store(!infinite, SeqCst);
    }

    pub fn send_stop(&mut self) {
        // Set `infinite` to `false` before stopping the search such that the engine will output a `bestmove`
        // as demanded by the spec, such as when it stops pondering:
        // It doesn't matter if the engine threads reads `infinite` before it is updated,
        // it will print the result in both cases.
        self.sss.print_result.store(true, SeqCst);
        self.sss.infinite.store(false, SeqCst);
        self.sss.stop.store(true, SeqCst);
        // wait until the search has finished to prevent race conditions
        while self.sss.searching.load(SeqCst) {}
    }

    pub fn set_searching(&mut self, value: bool) {
        self.sss.searching.store(value, SeqCst);
    }

    pub fn new_search(&mut self, infinite: bool) {
        // should be unnecessary but best to be certain
        self.sss.stop.store(true, SeqCst);
        // wait until any previous search has been stopped
        while self.sss.searching.load(SeqCst) {}
        self.sss.infinite.store(infinite, SeqCst);
        self.sss.print_result.store(true, SeqCst);
        self.sss.stop.store(false, SeqCst);
    }

    /// This function gets called both on a ponder hit and on a ponder miss; there is no distinction in how they
    /// are handled. Still, a ponder hit is the better outcome because the search can reuse the learned values.
    pub fn abort_pondering(&mut self) {
        // We simply abort the current search. Since the state is persistent, this still helps a lot.
        // This isn't the optimal implementation, but it's simple and ponder strength isn't a big concern.
        self.sss.print_result.store(false, SeqCst);
        self.sss.infinite.store(false, SeqCst);
        // can only stop after having made sure the result won't be printed
        self.sss.stop.store(true, SeqCst);
        // wait until the search has finished to avoid race conditions
        while self.sss.searching.load(SeqCst) {}
    }

    pub fn should_stop(&self) -> bool {
        self.sss.stop.load(SeqCst)
    }

    pub fn send_search_info(&mut self, info: SearchInfo<B>) {
        if let Some(output) = &self.output {
            output.lock().unwrap().show_search_info(info)
        }
    }

    pub fn send_search_res(&mut self, res: SearchResult<B>) {
        if let Some(output) = &self.output {
            // Spin until pondering has been disabled, such as through a `stop` command or through a ponderhit
            while self.sss.infinite.load(SeqCst) {}
            if self.sss.print_result.load(SeqCst) {
                output.lock().unwrap().show_search_res(res)
            }
        }
    }

    pub fn send_bench_res(&mut self, res: BenchResult) {
        if let Some(output) = &self.output {
            output.lock().unwrap().show_bench(res)
        }
    }

    pub fn send_static_eval(&mut self, eval: Score) {
        if let Some(output) = &self.output {
            output
                .lock()
                .unwrap()
                .write_ugi(&format!("score cp {}", eval.0))
        }
    }

    pub fn send_message(&mut self, typ: Message, text: &str) {
        if let Some(output) = &self.output {
            output.lock().unwrap().write_message(typ, text)
        }
    }

    pub fn send_statistics(&mut self, statistics: &Statistics) {
        // don't pay the performance penalty of aggregating statistics unless they are shown,
        // especially since the "statistics" feature is likely turned off
        if cfg!(feature = "statistics") && self.output.is_some() {
            self.send_message(Debug, &Summary::new(statistics).to_string());
        }
    }

    pub fn deactivate_output(&mut self) {
        self.output = None;
    }
}

pub struct EngineThread<B: Board, E: Engine<B>> {
    engine: E,
    search_sender: SearchSender<B>,
    receiver: Receiver<EngineReceives<B>>,
}

impl<B: Board, E: Engine<B>> EngineThread<B, E> {
    pub fn new(
        engine: E,
        search_sender: SearchSender<B>,
        receiver: Receiver<EngineReceives<B>>,
    ) -> Self {
        Self {
            engine,
            search_sender,
            receiver,
        }
    }

    fn start_search(
        &mut self,
        pos: B,
        limit: SearchLimit,
        history: ZobristHistory<B>,
        tt: TT,
        search_moves: Vec<B::Move>,
    ) -> Res<()> {
        if self.engine.is_currently_searching() {
            return Err(format!(
                "Engine {} received a go command while still searching",
                self.engine.long_name()
            ));
        }
        self.engine.set_tt(tt);
        let search_res = self.engine.search_moves(
            search_moves.into_iter(),
            pos,
            limit,
            history,
            self.search_sender.clone(),
        )?;

        self.search_sender.send_search_res(search_res);
        Ok(())
    }

    fn bench_single_position(&mut self, pos: B, depth: Depth) -> Res<()> {
        // self.engine.stop();
        self.engine.forget();
        let res = self.engine.bench(pos, depth);
        self.search_sender.send_bench_res(res);
        Ok(())
    }

    fn get_static_eval(&mut self, pos: B) {
        let eval = self.engine.static_eval(pos);
        self.search_sender.send_static_eval(eval);
    }

    fn write_error(&mut self, msg: &str) {
        self.search_sender.send_message(Error, msg);
    }

    fn handle_input(&mut self, received: EngineReceives<B>) -> Res<bool> {
        match received {
            Quit => {
                self.engine.quit();
                return Ok(true);
            }
            Forget => {
                if !self.engine.search_state().search_cancelled() {
                    return Err(format!("Engine '{}' received a 'forget' command (ucinewgame) while still searching", self.engine.long_name()));
                }
                self.engine.forget();
            }
            SetOption(name, value) => match name {
                Threads => panic!("This should have already been handled by the engine owner"),
                _ => self.engine.set_option(name, value)?, // TODO: Update info in UGI client
            },
            Search(pos, limit, history, tt, moves) => {
                self.start_search(pos, limit, history, tt, moves)?
            }
            Bench(pos, depth) => self.bench_single_position(pos, depth)?,
            EvalFor(pos) => self.get_static_eval(pos),
            SetEval(eval) => self.engine.set_eval(eval),
        };
        Ok(false)
    }

    pub fn try_handle_input(&mut self) -> Res<bool> {
        match self.receiver.recv() {
            Ok(msg) => self.handle_input(msg),
            Err(_err) => {
                self.engine.quit();
                Ok(true)
            }
        }
    }

    pub fn main_loop(&mut self) {
        loop {
            match self.try_handle_input() {
                Err(msg) => {
                    self.write_error(&msg);
                    self.engine.quit();
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

/// Implementations of this trait live in the UGI thread and deal with forwarding the UGI commands to
/// all engine threads and coordinating the different engine threads to arrive at only one chosen move.
#[derive(Debug)]
pub struct EngineWrapper<B: Board> {
    sender: Sender<EngineReceives<B>>,
    engine_info: EngineInfo,
    tt: TT,
    secondary: Vec<EngineWrapper<B>>,
    builder: EngineBuilder<B>,
}

impl<B: Board> Drop for EngineWrapper<B> {
    fn drop(&mut self) {
        _ = self.sender.send(Quit);
    }
}

impl<B: Board> EngineWrapper<B> {
    pub fn new_with_tt<E: Engine<B>>(engine: E, builder: EngineBuilder<B>, tt: TT) -> Self {
        let (sender, receiver) = unbounded();
        let info = engine.engine_info();
        let search_sender = builder.sender.clone();
        let mut thread = EngineThread {
            engine,
            search_sender,
            receiver,
        };
        spawn(move || thread.main_loop());
        EngineWrapper {
            sender,
            engine_info: info,
            tt,
            secondary: vec![],
            builder,
        }
    }

    fn search_sender(&mut self) -> &mut SearchSender<B> {
        &mut self.builder.sender
    }

    pub fn start_search(
        &mut self,
        pos: B,
        limit: SearchLimit,
        history: ZobristHistory<B>,
        ponder: bool,
        search_moves: Vec<B::Move>,
    ) -> Res<()> {
        if self.is_primary() {
            // reset `stop` first such that a finished ponder command won't print anything
            self.search_sender().new_search(limit.is_infinite());
        }
        for o in self.secondary.iter_mut() {
            o.start_search(pos, limit, history.clone(), ponder, search_moves.clone())?;
        }
        self.sender
            .send(Search(pos, limit, history, self.tt.clone(), search_moves))
            .map_err(|err| err.to_string())
    }

    pub fn start_bench(&mut self, pos: B, depth: Depth) -> Res<()> {
        self.search_sender().new_search(false);
        self.sender
            .send(Bench(pos, depth))
            .map_err(|err| err.to_string())
    }

    pub fn static_eval(&mut self, pos: B) -> Res<()> {
        self.sender
            .send(EvalFor(pos))
            .map_err(|err| err.to_string())
    }

    pub fn set_tt(&mut self, tt: TT) {
        for wrapper in self.secondary.iter_mut() {
            wrapper.set_tt(tt.clone());
        }
        self.tt = tt;
    }

    pub fn set_option(&mut self, name: EngineOptionName, value: String) -> Res<()> {
        if name == Threads {
            let count: usize = parse_int_from_str(&value, "num threads")?;
            if !self.builder.search_builder.can_use_multiple_threads() && count != 1 {
                return Err(format!(
                    "The engine {} only supports 1 thread",
                    self.engine_info.long_name()
                ));
            }
            self.secondary.clear();
            let mut sender = self.search_sender().clone();
            sender.deactivate_output();
            self.secondary
                .resize_with(count - 1, || self.builder.build_wrapper());
            Ok(())
        } else if name == Hash {
            let value: usize = parse_int_from_str(&value, "hash size in mb")?;
            let size = value * 1_000_000;
            self.set_tt(TT::new_with_bytes(size));
            Ok(())
        } else {
            for o in self.secondary.iter_mut() {
                o.set_option(name.clone(), value.clone())?;
            }
            self.sender
                .send(SetOption(name, value))
                .map_err(|err| err.to_string())
        }
    }

    pub fn set_eval(&mut self, eval: Box<dyn Eval<B>>) -> Res<()> {
        for o in self.secondary.iter_mut() {
            o.set_eval(clone_box(eval.as_ref()))?;
        }
        self.engine_info.set_eval(eval.as_ref());
        self.sender
            .send(SetEval(eval))
            .map_err(|err| err.to_string())
    }

    pub fn send_stop(&mut self) {
        for o in self.secondary.iter_mut() {
            o.send_stop();
        }
        self.search_sender().send_stop();
    }

    pub fn send_quit(&mut self) -> Res<()> {
        for o in self.secondary.iter_mut() {
            o.send_quit()?;
        }
        self.search_sender().send_stop();
        self.sender.send(Quit).map_err(|err| err.to_string())
    }

    pub fn send_forget(&mut self) -> Res<()> {
        for o in self.secondary.iter_mut() {
            o.send_forget()?
        }
        if self.is_primary() {
            self.tt.forget();
        }
        self.sender.send(Forget).map_err(|err| err.to_string())
    }

    pub fn engine_info(&self) -> &EngineInfo {
        &self.engine_info
    }

    pub fn get_options(&self) -> &[EngineOption] {
        self.engine_info().options.as_slice()
    }

    fn is_primary(&self) -> bool {
        self.builder.sender.output.is_some()
    }
}
