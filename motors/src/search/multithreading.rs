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
use gears::search::{Depth, SearchInfo, SearchLimit, SearchResult};
use gears::ugi::EngineOptionName::{Hash, Threads};
use gears::ugi::{EngineOption, EngineOptionName};

use crate::search::multithreading::EngineReceives::*;
use crate::search::tt::TT;
use crate::search::Searching::Ongoing;
use crate::search::{
    BasicSearchState, BenchResult, Engine, EngineInfo, EngineWrapperBuilder, Searching,
};
use crate::ugi_engine::UgiOutput;

pub type Sender<T> = crossbeam_channel::Sender<T>;
pub type Receiver<T> = crossbeam_channel::Receiver<T>;
pub type TryRecvError = crossbeam_channel::TryRecvError;

pub enum EngineReceives<B: Board> {
    Quit,
    // joins the thread
    Stop,
    Forget,
    SetOption(EngineOptionName, String),
    Search(B, SearchLimit, ZobristHistoryBase, TT),
    Bench(B, Depth),
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
        if !self.engine.search_state().search_cancelled() {
            return Err(format!(
                "Engine {} received a go command while still searching",
                self.engine.long_name()
            ));
        }
        self.engine.search_state_mut().set_searching(Ongoing);
        let search_res = self
            .engine
            .search(pos, limit, history, &mut self.search_sender)?;
        if !self.engine.search_state().search_cancelled() {
            self.engine
                .search_state_mut()
                .set_searching(Searching::Stop);
        }

        self.search_sender.send_search_res(search_res);
        Ok(())
    }

    fn bench_single_position(&mut self, pos: B, depth: Depth) -> Res<()> {
        self.engine.stop();
        self.engine.forget();
        let res = self.engine.bench(pos, depth);
        self.search_sender.send_bench_res(res);
        Ok(())
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
            Stop => self.engine.stop(), // TODO: This can probably get called twice (also from the sender), so make sure it's idempotent
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

#[derive(Debug)]
pub struct EngineOwner<B: Board> {
    sender: Sender<EngineReceives<B>>,
    search_sender: SearchSender<B>,
    engine_info: EngineInfo,
    tt: TT,
}

impl<B: Board> EngineOwner<B> {
    pub fn new<E: Engine<B>>(engine: E, search_sender: SearchSender<B>) -> Self {
        Self::new_with(|| engine, search_sender)
    }

    // TODO: Needed?

    pub fn new_with<E: Engine<B>, F>(f: F, search_sender: SearchSender<B>) -> Self
    where
        F: FnOnce() -> E,
    {
        let (sender, receiver) = unbounded();
        let engine = f();
        let info = engine.engine_info();
        let search_sender_clone = search_sender.clone();
        let mut thread = EngineThread {
            engine,
            search_sender,
            receiver,
        };
        spawn(move || thread.main_loop());
        EngineOwner {
            sender,
            search_sender: search_sender_clone,
            engine_info: info,
            tt: TT::default(),
        }
    }
}

impl<B: Board> Drop for EngineOwner<B> {
    fn drop(&mut self) {
        _ = self.sender.send(Quit);
    }
}

/// Implementations of this trait live in the UGI thread and deal with forwarding the UGI commands to
/// all engine threads and coordinating the different engine threads to arrive at only one chosen move.
pub trait EngineWrapper<B: Board>: Debug {
    // /// This alias would be unnecessary if Rust allowed to return `impl Iterator` from a trait method
    // type PollResultIter: Iterator<Item = EngineSends<B>>;
    // type WaitResultIter: Iterator<Item = EngineSends<B>>;
    // fn new<E: Engine<B>>(engine: E) -> Self;

    fn start_search(&mut self, pos: B, limit: SearchLimit, history: ZobristHistoryBase) -> Res<()>;

    fn start_bench(&mut self, pos: B, depth: Depth) -> Res<()>;

    fn set_option(&mut self, name: EngineOptionName, value: String) -> Res<()>;

    fn send_stop(&mut self) -> Res<()>;

    fn send_quit(&mut self) -> Res<()>;

    fn send_forget(&mut self) -> Res<()>;

    fn engine_info(&self) -> &EngineInfo;

    fn get_options(&self) -> &[EngineOption] {
        self.engine_info().options.as_slice()
    }

    fn search_sender(&mut self) -> &mut SearchSender<B>;
}

impl<B: Board> EngineWrapper<B> for EngineOwner<B> {
    fn start_search(&mut self, pos: B, limit: SearchLimit, history: ZobristHistoryBase) -> Res<()> {
        self.search_sender.reset_stop();
        self.sender
            .send(Search(pos, limit, history, self.tt.clone()))
            .map_err(|err| err.to_string())
    }

    fn start_bench(&mut self, pos: B, depth: Depth) -> Res<()> {
        self.search_sender.reset_stop();
        self.sender
            .send(Bench(pos, depth))
            .map_err(|err| err.to_string())
    }

    fn set_option(&mut self, name: EngineOptionName, value: String) -> Res<()> {
        if name == Threads {
            if value.trim() != "1" {
                return Err(format!(
                    "The engine '{}' only supports running on a single thread",
                    self.engine_info.name
                ));
            }
            Ok(())
        } else if name == Hash {
            let value: usize = parse_int_from_str(&value, "hash size in mb")?;
            let size = value * 1_000_000;
            self.tt = TT::new_with_bytes(size);
            Ok(())
        } else {
            self.sender
                .send(SetOption(name, value))
                .map_err(|err| err.to_string())
        }
    }

    fn send_stop(&mut self) -> Res<()> {
        self.search_sender.send_stop();
        self.sender.send(Stop).map_err(|err| err.to_string())
    }

    fn send_quit(&mut self) -> Res<()> {
        self.search_sender.send_stop();
        self.sender.send(Quit).map_err(|err| err.to_string())
    }

    fn send_forget(&mut self) -> Res<()> {
        self.tt.forget();
        self.sender.send(Forget).map_err(|err| err.to_string())
    }

    fn engine_info(&self) -> &EngineInfo {
        &self.engine_info
    }

    fn search_sender(&mut self) -> &mut SearchSender<B> {
        &mut self.search_sender
    }
}

/// A `MultithreadedEngine` looks almost exactly like a normal engine from the outside and internally manages
/// multiple concurrently running engines. All outside code only interfaces with the main engine, which owns
/// the spawned concurrent engines. This type is general enough to support holding different engines, which
/// might be useful in the future for some less serious engines.
#[derive(Debug)]
pub struct MultithreadedEngine<B: Board> {
    main: Box<dyn EngineWrapper<B>>,
    owned: Vec<Box<dyn EngineWrapper<B>>>,
    builder: EngineWrapperBuilder<B>,
}

impl<B: Board> MultithreadedEngine<B> {
    pub fn new(builder: EngineWrapperBuilder<B>) -> Self {
        Self {
            main: builder.single_threaded(true),
            owned: vec![],
            builder,
        }
    }
}

impl<B: Board> EngineWrapper<B> for MultithreadedEngine<B> {
    fn start_search(&mut self, pos: B, limit: SearchLimit, history: ZobristHistoryBase) -> Res<()> {
        for o in self.owned.iter_mut() {
            o.start_search(pos, limit, history.clone())?;
        }
        self.main.start_search(pos, limit, history)
    }

    fn start_bench(&mut self, pos: B, depth: Depth) -> Res<()> {
        self.main.start_bench(pos, depth)
    }

    fn set_option(&mut self, name: EngineOptionName, value: String) -> Res<()> {
        if name == Threads {
            let count: usize = parse_int_from_str(&value, "num threads")?;
            self.owned.clear();
            self.owned
                .resize_with(count - 1, || self.builder.single_threaded(false));
            Ok(())
        } else if name == Hash {
            self.main.set_option(name, value)
        } else {
            for o in self.owned.iter_mut() {
                o.set_option(name.clone(), value.clone())?;
            }
            self.main.set_option(name, value)
        }
    }

    fn send_stop(&mut self) -> Res<()> {
        for o in self.owned.iter_mut() {
            o.send_stop()?;
        }
        self.main.send_stop()
    }

    fn send_quit(&mut self) -> Res<()> {
        for o in self.owned.iter_mut() {
            o.send_quit()?;
        }
        self.main.send_quit()
    }

    fn send_forget(&mut self) -> Res<()> {
        for o in self.owned.iter_mut() {
            o.send_forget()?
        }
        self.main.send_forget()
    }

    fn engine_info(&self) -> &EngineInfo {
        self.main.engine_info()
    }

    fn search_sender(&mut self) -> &mut SearchSender<B> {
        self.main.search_sender()
    }
}
