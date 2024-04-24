use std::fmt::Debug;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::{Arc, Mutex};
use std::thread::spawn;

use crossbeam_channel::unbounded;

use gears::games::{Board, ZobristHistoryBase};
use gears::general::common::{parse_int_from_str, NamedEntity, Res};
use gears::output::Message;
use gears::output::Message::Error;
use gears::search::{Depth, Score, SearchInfo, SearchLimit, SearchResult};
use gears::ugi::EngineOptionName::{Hash, Threads};
use gears::ugi::{EngineOption, EngineOptionName};

use crate::search::multithreading::EngineReceives::*;
use crate::search::tt::TT;
use crate::search::{AbstractEngineBuilder, BenchResult, Engine, EngineInfo, SearchState};
use crate::ugi_engine::UgiOutput;

pub type Sender<T> = crossbeam_channel::Sender<T>;
pub type Receiver<T> = crossbeam_channel::Receiver<T>;
pub type TryRecvError = crossbeam_channel::TryRecvError;

pub enum EngineReceives<B: Board> {
    // joins the thread
    Quit,
    Forget,
    SetOption(EngineOptionName, String),
    Search(B, SearchLimit, ZobristHistoryBase, TT),
    Bench(B, Depth),
    Eval(B),
}

/// A search sender is used for communication while the search is ongoing.
/// This is therefore necessarily a part of the engine's interface, unlike the Engine thread, which only
/// deals with starting searches and returning the results across threads, and is therefore unnecessary if
/// the engine is used as a library.
#[derive(Debug, Clone)]
pub struct SearchSender<B: Board> {
    output: Option<Arc<Mutex<UgiOutput<B>>>>,
}

impl<B: Board> SearchSender<B> {
    pub fn new(output: Arc<Mutex<UgiOutput<B>>>) -> Self {
        Self {
            output: Some(output),
        }
    }

    pub fn no_sender() -> Self {
        Self { output: None }
    }

    pub fn send_stop(&mut self) {
        STOP.store(true, SeqCst);
    }

    pub fn reset_stop(&mut self) {
        STOP.store(false, SeqCst);
    }

    pub fn should_stop(&self) -> bool {
        STOP.load(SeqCst)
    }

    pub fn send_search_info(&mut self, info: SearchInfo<B>) {
        if let Some(output) = &self.output {
            output.lock().unwrap().show_search_info(info)
        }
    }

    pub fn send_search_res(&mut self, res: SearchResult<B>) {
        if let Some(output) = &self.output {
            output.lock().unwrap().show_search_res(res)
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

    pub fn deactivate_output(&mut self) {
        self.output = None;
    }
}

pub static STOP: AtomicBool = AtomicBool::new(false);

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
        history: ZobristHistoryBase,
        tt: TT,
    ) -> Res<()> {
        if self.engine.is_currently_searching() {
            return Err(format!(
                "Engine {} received a go command while still searching",
                self.engine.long_name()
            ));
        }
        self.engine.set_tt(tt);
        let search_res = self
            .engine
            .search(pos, limit, history, &mut self.search_sender)?;

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
        let eval = self.engine.get_static_eval(pos);
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
            Search(pos, limit, history, tt) => self.start_search(pos, limit, history, tt)?,
            Bench(pos, depth) => self.bench_single_position(pos, depth)?,
            Eval(pos) => self.get_static_eval(pos),
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
    search_sender: SearchSender<B>,
    engine_info: EngineInfo,
    tt: TT,
    secondary: Vec<EngineWrapper<B>>,
    builder: Box<dyn AbstractEngineBuilder<B>>,
}

impl<B: Board> Drop for EngineWrapper<B> {
    fn drop(&mut self) {
        _ = self.sender.send(Quit);
    }
}

impl<B: Board> EngineWrapper<B> {
    pub fn new_with_tt<E: Engine<B>>(
        engine: E,
        search_sender: SearchSender<B>,
        builder: Box<dyn AbstractEngineBuilder<B>>,
        tt: TT,
    ) -> Self {
        let (sender, receiver) = unbounded();
        let info = engine.engine_info();
        let search_sender_clone = search_sender.clone();
        let mut thread = EngineThread {
            engine,
            search_sender,
            receiver,
        };
        spawn(move || thread.main_loop());
        EngineWrapper {
            sender,
            search_sender: search_sender_clone,
            engine_info: info,
            tt,
            secondary: vec![],
            builder,
        }
    }

    pub fn start_search(
        &mut self,
        pos: B,
        limit: SearchLimit,
        history: ZobristHistoryBase,
    ) -> Res<()> {
        if self.is_primary() {
            self.search_sender.reset_stop();
        }
        for o in self.secondary.iter_mut() {
            o.start_search(pos, limit, history.clone())?;
        }
        self.sender
            .send(Search(pos, limit, history, self.tt.clone()))
            .map_err(|err| err.to_string())
    }

    pub fn start_bench(&mut self, pos: B, depth: Depth) -> Res<()> {
        self.search_sender.reset_stop();
        self.sender
            .send(Bench(pos, depth))
            .map_err(|err| err.to_string())
    }

    pub fn static_eval(&mut self, pos: B) -> Res<()> {
        self.sender.send(Eval(pos)).map_err(|err| err.to_string())
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
            if !self.builder.can_use_multiple_threads() && count != 1 {
                return Err(format!(
                    "The engine {} only supports 1 thread",
                    self.engine_info.name
                ));
            }
            self.secondary.clear();
            let mut sender = self.search_sender.clone();
            sender.deactivate_output();
            self.secondary.resize_with(count - 1, || {
                self.builder.build(sender.clone(), self.tt.clone())
            });
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

    pub fn send_stop(&mut self) {
        for o in self.secondary.iter_mut() {
            o.send_stop();
        }
        self.search_sender.send_stop();
    }

    pub fn send_quit(&mut self) -> Res<()> {
        for o in self.secondary.iter_mut() {
            o.send_quit()?;
        }
        self.search_sender.send_stop();
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
        self.search_sender.output.is_some()
    }
}
