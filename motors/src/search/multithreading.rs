use std::fmt::Debug;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::thread::spawn;

use crossbeam_channel::unbounded;
use dyn_clone::{clone_box, DynClone};

use gears::games::{Board, ZobristHistoryBase};
use gears::general::common::{NamedEntity, parse_int_from_str, Res};
use gears::output::Message;
use gears::output::Message::Error;
use gears::output::text_output::TextWriter;
use gears::search::{Depth, SearchInfo, SearchLimit, SearchResult};
use gears::ugi::{EngineOption, EngineOptionName};
use gears::ugi::EngineOptionName::Threads;

use crate::search::{
    BasicSearchState, BenchResult, Engine, EngineInfo, EngineWrapperBuilder, Searching,
};
use crate::search::multithreading::EngineReceives::*;
use crate::search::Searching::Ongoing;
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
    Search(B, SearchLimit, ZobristHistoryBase),
    Bench(B, Depth),
}

/// A search sender is used for communication while the search is ongoing.
///. This is therefore necessarily a part of the engine's interface, unlike the Engine thread, which only
/// deals with starting searches and returning the results across threads, and is therefore unnecessary if
/// the engine is used as a library.
pub trait SearchSender<B: Board>: Debug + Send + DynClone {
    fn send_quit(&mut self);
    fn send_stop(&mut self);
    fn reset_quit_stop(&mut self);
    fn should_quit(&self) -> bool;
    fn should_stop(&self) -> bool;
    fn send_search_info(&mut self, info: SearchInfo<B>);
    fn send_search_res(&mut self, res: SearchResult<B>);
    fn send_bench_res(&mut self, res: BenchResult);
    fn send_message(&mut self, typ: Message, text: &str);
}

#[derive(Debug, Default)]
pub struct NoSender {
    writer: Option<TextWriter>,
}

impl Clone for NoSender {
    fn clone(&self) -> Self {
        Self { writer: None }
    }
}

impl<B: Board> SearchSender<B> for NoSender {
    fn send_quit(&mut self) {
        // do nothing
    }

    fn send_stop(&mut self) {
        // do nothing
    }

    fn reset_quit_stop(&mut self) {
        // do nothing
    }

    fn should_quit(&self) -> bool {
        false
    }

    fn should_stop(&self) -> bool {
        false
    }

    fn send_search_info(&mut self, _info: SearchInfo<B>) {
        // do nothing
    }

    fn send_search_res(&mut self, _res: SearchResult<B>) {
        // do nothing
    }

    fn send_bench_res(&mut self, _res: BenchResult) {
        // do nothing
    }

    fn send_message(&mut self, typ: Message, text: &str) {
        self.writer
            .as_mut()
            .map(|mut m| m.display_message(typ, text));
    }
}

pub static QUIT: AtomicBool = AtomicBool::new(false);
pub static STOP: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone)]
pub struct UgiSender<B: Board> {
    main_thread: bool,
    output: Arc<Mutex<UgiOutput<B>>>,
}

impl<B: Board> UgiSender<B> {
    pub fn new(output: Arc<Mutex<UgiOutput<B>>>) -> Self {
        Self {
            main_thread: true,
            output,
        }
    }
}

impl<B: Board> SearchSender<B> for UgiSender<B> {
    fn send_quit(&mut self) {
        STOP.store(true, SeqCst);
        QUIT.store(true, SeqCst);
    }

    fn send_stop(&mut self) {
        STOP.store(true, SeqCst);
    }

    fn reset_quit_stop(&mut self) {
        STOP.store(false, SeqCst);
        QUIT.store(false, SeqCst);
    }

    fn should_quit(&self) -> bool {
        QUIT.load(SeqCst)
    }

    fn should_stop(&self) -> bool {
        STOP.load(SeqCst)
    }

    fn send_search_info(&mut self, info: SearchInfo<B>) {
        if self.main_thread {
            self.output.lock().unwrap().show_search_info(info)
        }
    }

    fn send_search_res(&mut self, res: SearchResult<B>) {
        if self.main_thread {
            self.output.lock().unwrap().show_search_res(res)
        }
    }

    fn send_bench_res(&mut self, res: BenchResult) {
        if self.main_thread {
            self.output.lock().unwrap().show_bench(res)
        }
    }

    fn send_message(&mut self, typ: Message, text: &str) {
        self.output.lock().unwrap().write_message(typ, text)
    }
}

pub struct EngineThread<B: Board, E: Engine<B>> {
    engine: E,
    search_sender: Box<dyn SearchSender<B>>,
    receiver: Receiver<EngineReceives<B>>,
}

impl<B: Board, E: Engine<B>> EngineThread<B, E> {
    pub fn new(
        engine: E,
        search_sender: Box<dyn SearchSender<B>>,
        receiver: Receiver<EngineReceives<B>>,
    ) -> Self {
        Self {
            engine,
            search_sender,
            receiver,
        }
    }

    fn start_search(&mut self, pos: B, limit: SearchLimit, history: ZobristHistoryBase) -> Res<()> {
        if !self.engine.search_state().search_cancelled() {
            return Err(format!(
                "Engine {} received a go command while still searching",
                self.engine.long_name()
            ));
        }
        self.engine.search_state_mut().set_searching(Ongoing);
        let search_res = self
            .engine
            .search(pos, limit, history, self.search_sender.as_mut())?;
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

    fn handle_input(&mut self, received: EngineReceives<B>) -> Res<()> {
        match received {
            Quit => self.engine.quit(),
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
            Search(pos, limit, history) => self.start_search(pos, limit, history)?,
            Bench(pos, depth) => self.bench_single_position(pos, depth)?,
        };
        Ok(())
    }

    pub fn try_handle_input(&mut self) -> Res<()> {
        match self.receiver.recv() {
            Ok(msg) => self.handle_input(msg)?,
            Err(_err) => self.engine.quit(),
        };
        Ok(())
        //     Err(err) => {
        //         // If the channel has disconnected, this can either mean that the program has ended or that
        //         // something has gone terribly wrong. TODO: Check if this makes sense
        //         self.write_error(&err.to_string());
        //         self.engine.quit(); // All is lost.
        //     }
        //     Ok(r) => self.handle_input(r)?,
        // };
        // Ok(())
    }

    // TODO: Actually used?
    // Move into engine functionality

    pub fn check_if_aborted(&mut self, limit: &SearchLimit) -> bool {
        if self.try_handle_input().is_err() {
            return true;
        }
        if self.engine.time_up(
            limit.tc,
            limit.fixed_time,
            self.engine.search_state().start_time(),
        ) {
            self.engine
                .search_state_mut()
                .set_searching(Searching::Stop);
        }
        self.engine.search_state().search_cancelled()
    }

    pub fn main_loop(&mut self) {
        loop {
            if let Err(msg) = self.try_handle_input() {
                self.write_error(&msg);
                self.engine.quit();
            }
            if self.search_sender.should_quit() {
                break;
            }
        }
        // Exit the main loop, cleaning up all allocated resources
    }
}

#[derive(Debug)]
pub struct EngineOwner<B: Board> {
    sender: Sender<EngineReceives<B>>,
    search_sender: Box<dyn SearchSender<B>>,
    engine_info: EngineInfo,
}

impl<B: Board> EngineOwner<B> {
    pub fn new<E: Engine<B>>(engine: E, search_sender: Box<dyn SearchSender<B>>) -> Self {
        Self::new_with(|| engine, search_sender)
    }

    // TODO: Needed?

    pub fn new_with<E: Engine<B>, F>(f: F, search_sender: Box<dyn SearchSender<B>>) -> Self
    where
        F: FnOnce() -> E,
    {
        let (sender, receiver) = unbounded();
        let engine = f();
        let info = engine.engine_info();
        let search_sender_clone = clone_box(search_sender.deref());
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

    fn search_sender(&self) -> Box<dyn SearchSender<B>>;
}

impl<B: Board> EngineWrapper<B> for EngineOwner<B> {
    fn start_search(&mut self, pos: B, limit: SearchLimit, history: ZobristHistoryBase) -> Res<()> {
        self.search_sender.reset_quit_stop();
        self.sender
            .send(Search(pos, limit, history))
            .map_err(|err| err.to_string())
    }

    fn start_bench(&mut self, pos: B, depth: Depth) -> Res<()> {
        self.search_sender.reset_quit_stop();
        self.sender
            .send(Bench(pos, depth))
            .map_err(|err| err.to_string())
    }

    fn set_option(&mut self, name: EngineOptionName, value: String) -> Res<()> {
        self.sender
            .send(SetOption(name, value))
            .map_err(|err| err.to_string())
    }

    fn send_stop(&mut self) -> Res<()> {
        self.search_sender.send_stop();
        self.sender.send(Stop).map_err(|err| err.to_string())
    }

    fn send_quit(&mut self) -> Res<()> {
        self.search_sender.send_quit();
        self.sender.send(Quit).map_err(|err| err.to_string())
    }

    fn send_forget(&mut self) -> Res<()> {
        self.sender.send(Forget).map_err(|err| err.to_string())
    }

    fn engine_info(&self) -> &EngineInfo {
        &self.engine_info
    }

    fn search_sender(&self) -> Box<dyn SearchSender<B>> {
        clone_box(self.search_sender.deref())
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
            main: builder.single_threaded(),
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
                .resize_with(count - 1, || self.builder.multi_threaded());
        } else {
            for o in self.owned.iter_mut() {
                o.set_option(name.clone(), value.clone())?;
            }
        }
        self.main.set_option(name, value)
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

    fn search_sender(&self) -> Box<dyn SearchSender<B>> {
        self.main.search_sender()
    }
}
