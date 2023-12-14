use std::fmt::Debug;
use std::marker::PhantomData;
use std::thread::spawn;

use crossbeam_channel::unbounded;

use crate::games::{Board, ZobristHistoryBase};
use crate::general::common::{parse_int_from_str, Res};
use crate::search::multithreading::EngineReceives::*;
use crate::search::multithreading::EngineSends::{BenchRes, EngineCopy, SearchRes};
use crate::search::EngineOptionName::Threads;
use crate::search::Searching::Ongoing;
use crate::search::{
    BasicSearchState, BenchResult, Engine, EngineInfo, EngineOptionName, EngineOptionType,
    SearchInfo, SearchLimit, SearchResult, Searcher, Searching,
};

pub type Sender<T> = crossbeam_channel::Sender<T>;
pub type Receiver<T> = crossbeam_channel::Receiver<T>;
pub type TryRecvError = crossbeam_channel::TryRecvError;

pub enum EngineReceives<B: Board> {
    Quit, // joins the thread
    Stop,
    Forget,
    SetOption(EngineOptionName, String),
    Search(B, SearchLimit, ZobristHistoryBase),
    Bench(B, usize),
}

pub enum EngineSends<B: Board> {
    // Nodes(u64),
    BenchRes(BenchResult),
    SearchRes(SearchResult<B>),
    EngineInformation(EngineInfo),
    Info(SearchInfo<B>),
    Message(String),
    Error(String),
    EngineCopy(EngineOwner<B>),
}

#[derive(Debug)]
pub struct EngineCommunicator<B: Board> {
    pub receiver: Receiver<EngineReceives<B>>,
    pub sender: Sender<EngineSends<B>>,
}

#[derive(Debug)]
pub struct EngineOwner<B: Board> {
    sender: Sender<EngineReceives<B>>,
    receiver: Receiver<EngineSends<B>>,
    engine_info: EngineInfo,
}

pub struct EngineThread<B: Board, E: Engine<B>> {
    _phantom: PhantomData<E>,
    phantom2: PhantomData<B>,
}

impl<B: Board, E: Engine<B>> EngineThread<B, E> {
    fn write_error(engine: &mut E, message: String) {
        // If sending the error fails, simply ignore the error and quit.
        // There may be an unread `Quit` command in the channel, so don't assume that this is an error.
        if engine.send(EngineSends::Error(message.clone())).is_err() {
            engine.quit();
        }
    }

    fn start_search(
        engine: &mut E,
        pos: B,
        limit: SearchLimit,
        history: ZobristHistoryBase,
    ) -> Res<()> {
        if !engine.search_state().search_cancelled() {
            return Err(format!(
                "Engine {} received a go command while still searching",
                engine.name()
            ));
        }
        engine.search_state_mut().set_searching(Ongoing);
        let search_res = engine.search(pos, limit, history)?;
        if !engine.search_state().search_cancelled() {
            engine.search_state_mut().set_searching(Searching::Stop);
        }
        engine.send(SearchRes(search_res))
    }

    fn bench(engine: &mut E, pos: B, depth: usize) -> Res<()> {
        engine.stop();
        engine.forget();
        let res = engine.bench(pos, depth)?;
        engine.send(BenchRes(res))
    }

    fn handle_input(engine: &mut E, received: EngineReceives<B>) -> Res<()> {
        Ok(match received {
            Quit => engine.quit(),
            Stop => engine.stop(),
            Forget => {
                if !engine.search_state().search_cancelled() {
                    return Err(format!("Engine '{}' received a 'forget' command (ucinewgame) while still searching", engine.name()));
                }
                engine.forget();
            }
            SetOption(name, val) => match name {
                Threads => {
                    let count = parse_int_from_str(&val, "thread count")?;
                    for _ in 0..count {
                        engine
                            .send(EngineCopy(engine.clone_for_multithreading()))
                            .unwrap();
                    }
                }
                _ => {
                    if let Some(res) = engine.set_option(name, &val)? {
                        engine.send(res).unwrap();
                    }
                }
            },
            Search(pos, limit, history) => Self::start_search(engine, pos, limit, history)?,
            Bench(pos, depth) => Self::bench(engine, pos, depth)?,
        })
    }

    pub fn try_handle_input(engine: &mut E) -> Res<()> {
        Ok(match engine.communicator().receiver.try_recv() {
            Err(err) => match err {
                TryRecvError::Empty => (),
                TryRecvError::Disconnected => {
                    Self::write_error(engine, err.to_string());
                    engine.quit(); // All is lost.
                }
            },
            Ok(r) => Self::handle_input(engine, r)?,
        })
    }

    pub fn check_if_aborted(engine: &mut E, limit: &SearchLimit) -> bool {
        if Self::try_handle_input(engine).is_err() {
            return true;
        }
        if engine.time_up(
            limit.tc,
            limit.fixed_time,
            engine.search_state().start_time(),
        ) {
            engine.search_state_mut().set_searching(Searching::Stop);
        }
        engine.search_state().search_cancelled()
    }

    pub fn main_loop(mut engine: E) {
        loop {
            match engine.communicator().receiver.recv() {
                Ok(r) => {
                    if let Err(err) = Self::handle_input(&mut engine, r) {
                        Self::write_error(&mut engine, err);
                    }
                }
                Err(err) => {
                    Self::write_error(&mut engine, err.to_string());
                    engine.quit()
                }
            }
            if engine.search_state().should_quit() {
                break;
            }
        }
        // Exit the main loop, cleaning up all allocated resources
    }
}

impl<B: Board> EngineOwner<B> {
    pub fn new<E: Engine<B>>() -> Self {
        Self::new_with(E::new)
    }

    pub fn new_with<E: Engine<B>, F>(f: F) -> Self
    where
        F: Fn(EngineCommunicator<B>) -> E,
    {
        let (engine_sender, owner_receiver) = unbounded();
        let (owner_sender, engine_receiver) = unbounded();
        let communicator = EngineCommunicator {
            receiver: engine_receiver,
            sender: engine_sender,
        };
        let engine = f(communicator);
        let info = engine.engine_info();
        spawn(move || EngineThread::main_loop(engine));
        EngineOwner {
            sender: owner_sender,
            receiver: owner_receiver,
            engine_info: info,
        }
    }
}

impl<B: Board> Drop for EngineOwner<B> {
    fn drop(&mut self) {
        _ = self.sender.send(Quit);
    }
}

pub trait EnginePlayer<B: Board>: Debug {
    // /// This alias would be unnecessary if Rust allowed to return `impl Iterator` from a trait method
    // type PollResultIter: Iterator<Item = EngineSends<B>>;
    // type WaitResultIter: Iterator<Item = EngineSends<B>>;
    // fn new<E: Engine<B>>(engine: E) -> Self;

    fn receiver(&self) -> &Receiver<EngineSends<B>>;

    fn start_search(&self, pos: B, limit: SearchLimit, history: ZobristHistoryBase) -> Res<()>;

    fn start_bench(&self, pos: B, depth: usize) -> Res<()>;

    fn set_option(&mut self, name: EngineOptionName, value: String) -> Res<()>;

    fn send_stop(&self) -> Res<()>;

    fn receive_engine(&mut self, engine: EngineOwner<B>);

    fn send_quit(&self) -> Res<()>;

    fn send_forget(&self) -> Res<()>;

    fn engine_info(&self) -> &EngineInfo;

    fn get_options(&self) -> &[EngineOptionType] {
        self.engine_info().options.as_slice()
    }
}

impl<B: Board> EnginePlayer<B> for EngineOwner<B> {
    fn receiver(&self) -> &Receiver<EngineSends<B>> {
        &self.receiver
    }

    fn start_search(&self, pos: B, limit: SearchLimit, history: ZobristHistoryBase) -> Res<()> {
        self.sender
            .send(Search(pos, limit, history))
            .map_err(|err| err.to_string())
    }

    fn start_bench(&self, pos: B, depth: usize) -> Res<()> {
        self.sender
            .send(Bench(pos, depth))
            .map_err(|err| err.to_string())
    }

    fn set_option(&mut self, name: EngineOptionName, value: String) -> Res<()> {
        self.sender
            .send(SetOption(name, value))
            .map_err(|err| err.to_string())
    }

    fn send_stop(&self) -> Res<()> {
        self.sender.send(Stop).map_err(|err| err.to_string())
    }

    fn receive_engine(&mut self, engine: EngineOwner<B>) {
        *self = engine;
    }

    fn send_quit(&self) -> Res<()> {
        self.sender.send(Quit).map_err(|err| err.to_string())
    }

    fn send_forget(&self) -> Res<()> {
        self.sender.send(Forget).map_err(|err| err.to_string())
    }

    fn engine_info(&self) -> &EngineInfo {
        &self.engine_info
    }
}

/// A `MultithreadedEngine` looks almost exactly like a normal engine from the outside and internally manages
/// multiple concurrently running engines. All outside code only interfaces with the main engine, which owns
/// the spawned concurrent engines. This type is general enough to support holding different engines, which
/// might be useful in the future for some less serious engines.
#[derive(Debug)]
pub struct MultithreadedEngine<B: Board> {
    main: EngineOwner<B>,
    owned: Vec<EngineOwner<B>>,
}

impl<B: Board> MultithreadedEngine<B> {
    pub fn new(engine: EngineOwner<B>) -> Self {
        Self {
            main: engine,
            owned: vec![],
        }
    }
}

impl<B: Board> EnginePlayer<B> for MultithreadedEngine<B> {
    fn receiver(&self) -> &Receiver<EngineSends<B>> {
        self.main.receiver()
    }

    fn start_search(&self, pos: B, limit: SearchLimit, history: ZobristHistoryBase) -> Res<()> {
        for o in self.owned.iter() {
            o.start_search(pos, limit, history.clone())?;
        }
        self.main.start_search(pos, limit, history)
    }

    fn start_bench(&self, pos: B, depth: usize) -> Res<()> {
        self.main.start_bench(pos, depth)
    }

    fn set_option(&mut self, name: EngineOptionName, value: String) -> Res<()> {
        if name != Threads {
            for o in self.owned.iter_mut() {
                o.set_option(name.clone(), value.clone())?;
            }
        }
        self.main.set_option(name, value)
    }

    fn send_stop(&self) -> Res<()> {
        for o in self.owned.iter() {
            o.send_stop()?;
        }
        self.main.send_stop()
    }

    fn receive_engine(&mut self, engine: EngineOwner<B>) {
        self.owned.push(engine);
    }

    fn send_quit(&self) -> Res<()> {
        for o in self.owned.iter() {
            o.send_quit()?;
        }
        self.main.send_quit()
    }

    fn send_forget(&self) -> Res<()> {
        for o in self.owned.iter() {
            o.send_forget()?
        }
        self.main.send_forget()
    }

    fn engine_info(&self) -> &EngineInfo {
        self.main.engine_info()
    }
}
