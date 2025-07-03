use std::any::TypeId;
use std::collections::HashMap;
use std::env::current_exe;
use std::fmt::Debug;
use std::fmt::Write;
use std::fs::File;
use std::io::BufReader;
use std::ops::AddAssign;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{Builder, sleep};
use std::time::{Duration, Instant};

use gears::games::chess::Chessboard;
use gears::general::board::{Board, BoardHelpers};
use gears::general::common::Res;
use gears::general::common::anyhow::bail;
use gears::output::Message::*;
use gears::search::{DepthPly, MAX_DEPTH, NodesLimit, SearchLimit, TimeControl};
use gears::ugi::EngineOption;
use lazy_static::lazy_static;
use whoami::realname;

use crate::cli::{ClientEngineCliArgs, PlayerArgs};
use crate::play::player::EngineStatus::*;
use crate::play::player::Player::{Engine, Human};
use crate::play::player::Protocol::{Uci, Ugi};
use crate::play::ugi_client::{Client, PlayerId};
use crate::play::ugi_input::HandleBestMove::Play;
use crate::play::ugi_input::{BestMoveAction, CurrentMatch, EngineStatus, InputThread};

#[derive(Default, Debug)]
/// Ensures that there are no two engines with the same name after ignoring case
pub struct NameSet {}

lazy_static! {
    static ref PLAYER_NAMES: Mutex<HashMap<String, usize>> = Mutex::new(HashMap::default());
}

impl NameSet {
    #[must_use]
    pub fn make_name_unique(name: String) -> String {
        let mut guard = PLAYER_NAMES.lock().unwrap();
        let lowercase_name = name.to_lowercase();
        if let Some(mut n) = guard.get(&lowercase_name).copied() {
            // handle (admittedly paranoid) case where there's an engine named `stockfish_2` or similar
            while guard.contains_key(&format!("{lowercase_name}_{n}")) {
                n += 1;
            }
            let new_name = format!("{name}_{n}");
            n += 1;
            let Some(_) = guard.insert(lowercase_name, n) else { panic!("Internal error") };
            new_name
        } else {
            guard.insert(lowercase_name, 2);
            name
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct TimeMargin(pub Duration);

impl Default for TimeMargin {
    fn default() -> Self {
        Self(Duration::from_millis(10))
    }
}

#[derive(Debug, Default, Copy, Clone)]
#[must_use]
pub enum Protocol {
    #[default] // will be set depending on the game
    Uci,
    Ugi,
}

// The EnginePlayer is a member of the UgiMatchState and owns the child process running the engine as well as the
// thread listening for input from the child's stdout
#[derive(Debug)]
#[must_use]
pub struct EnginePlayer<B: Board> {
    /// Only used by the `write_ugi_impl` method
    child_stdin: ChildStdin,
    /// Only used by `drop`
    child: Child,
    pub status: EngineStatus,
    pub ugi_name: String,
    pub display_name: String,
    pub author: String,
    pub options: Vec<EngineOption>,
    /// If set to false, the GUI will use UCI instead of UGI (so `uci` and `ucinewgame` instead of `ugi` and `uginewgame`)
    pub proto: Protocol,
    pub white_pov: bool,
    pub time_margin: TimeMargin,
    pub default_limit: SearchLimit,
    pub current_match: Option<CurrentMatch<B>>,
}

impl<B: Board> Drop for EnginePlayer<B> {
    fn drop(&mut self) {
        _ = self.write_ugi_impl("quit");
        let start = Instant::now();
        // The spec demands a grace period of at least 5 seconds
        while start.elapsed() < Duration::from_millis(5000) {
            sleep(Duration::from_millis(50));
            if let Ok(Some(_)) = self.child.try_wait() {
                return;
            }
        }
        _ = self.child.kill();
        // make sure that the child has actually stopped running and isn't a zombie
        self.child.wait().unwrap();
    }
}

impl<B: Board> EnginePlayer<B> {
    pub fn new(
        proto: Protocol,
        default_limit: SearchLimit,
        time_margin: TimeMargin,
        white_pov: bool,
        display_name: String,
        mut child: Child,
    ) -> Self {
        let child_stdin = child.stdin.take().unwrap();
        Self {
            child_stdin,
            child,
            status: WaitingUgiOk,
            ugi_name: "<unknown>".to_string(),
            display_name,
            author: "<unknown>".to_string(),
            options: vec![],
            proto,
            white_pov,
            time_margin,
            default_limit,
            current_match: None,
        }
    }

    pub fn current_match(&mut self) -> &mut CurrentMatch<B> {
        self.current_match
            .as_mut()
            .ok_or_else(|| format!("Internal error: Engine '{}' isn't currently playing a match", self.display_name))
            .unwrap()
    }

    /// Outside code should use the `send_ugi_message\[_to\]` method of client because those also log communication
    pub fn write_ugi_impl(&mut self, msg: &str) -> Res<()> {
        use std::io::Write;
        writeln!(self.child_stdin, "{msg}")?;
        Ok(())
    }

    pub fn halt(&mut self, bestmove_action: BestMoveAction) {
        if matches!(self.status, ThinkingSince(_)) {
            self.status.halt(bestmove_action);
        }
    }
}

fn send_initial_ugi_impl<B: Board>(client: Arc<Mutex<Client<B>>>, id: PlayerId, retry_on_failure: bool) -> Res<()> {
    let proto = client.lock().unwrap().state.get_engine_from_id(id).proto;
    let msg = match proto {
        Ugi => "ugi",
        Uci => "uci",
    };
    // Do this now and through the UgiGui so that it gets logged in debug mode.
    client.lock().unwrap().send_ugi_message_to(id, msg);

    let start = Instant::now();
    while matches!(client.lock().unwrap().state.get_engine_from_id(id).status, WaitingUgiOk) {
        sleep(Duration::from_millis(5));
        if start.elapsed() > Duration::from_millis(5100) {
            // the spec demands a grace period of at least 5 seconds
            if retry_on_failure {
                client.lock().unwrap().state.get_engine_from_id_mut(id).proto = match proto {
                    Uci => Ugi,
                    Ugi => Uci,
                };
                return send_initial_ugi_impl(client, id, false);
            }
            let name = client.lock().unwrap().state.get_engine_from_id(id).display_name.clone();
            client.lock().unwrap().quit_program();
            bail!(
                "Couldn't initialize engine '{name}'. Didn't receive 'ugiok' or 'uciok' after the timeout was reached."
            )
        }
    }
    Ok(())
}

fn send_initial_ugi<B: Board>(
    client: Arc<Mutex<Client<B>>>,
    id: PlayerId,
    init_string: Option<String>,
    custom_options: &HashMap<String, String>,
) -> Res<()> {
    // TODO: initialized shouldn't be a member and instead be passed as parameter
    if let Some(init_string) = init_string {
        client.lock().unwrap().send_ugi_message_to(id, &init_string);
    }

    send_initial_ugi_impl(client.clone(), id, true)?;

    for (name, value) in custom_options {
        client.lock().unwrap().send_setoption(id, name, value);
    }
    Ok(())
}

pub fn limit_to_ugi(limit: SearchLimit, wtime: TimeControl, btime: TimeControl) -> Result<String, std::fmt::Error> {
    let mut res = String::new();
    write!(res, "go ")?;
    if wtime.remaining != Duration::MAX {
        write!(res, "wtime {} ", wtime.remaining.as_millis())?;
    }
    if !wtime.increment.is_zero() {
        write!(res, "winc {} ", wtime.increment.as_millis())?;
    }
    if btime.remaining != Duration::MAX {
        write!(res, "btime {} ", btime.remaining.as_millis())?;
    }
    if !btime.increment.is_zero() {
        write!(res, "binc {} ", btime.increment.as_millis())?;
    }
    if limit.nodes != NodesLimit::MAX {
        write!(res, "nodes {} ", limit.nodes)?;
    }
    if limit.depth < MAX_DEPTH {
        write!(res, "depth {} ", limit.depth.get())?;
    }
    if limit.fixed_time != Duration::MAX {
        write!(res, "movetime {} ", limit.fixed_time.as_millis())?;
    }
    let mut res = res.trim_end().to_string();
    if res == "go" {
        res.add_assign(" infinite");
    }
    Ok(res.trim_end().to_string())
}

#[derive(Debug, Default)]
#[must_use]
pub enum HumanPlayerStatus {
    #[default]
    Idle,
    ThinkingSince(Instant),
}

#[derive(Debug, Default)]
#[must_use]
pub struct HumanPlayer {
    tc: TimeControl,
    original_tc: TimeControl,
    pub name: String,
    pub status: HumanPlayerStatus,
}

#[derive(Debug, Clone)]
#[must_use]
pub struct PlayerBuilder {
    args: PlayerArgs,
}

impl PlayerBuilder {
    pub fn new(args: PlayerArgs) -> Self {
        Self { args }
    }

    pub fn build<B: Board>(self, client: Arc<Mutex<Client<B>>>) -> Res<PlayerId> {
        self.replace(client, None)
    }

    pub fn replace<B: Board>(self, client: Arc<Mutex<Client<B>>>, player: Option<PlayerId>) -> Res<PlayerId> {
        match self.args {
            PlayerArgs::Engine(ref args) => self.clone().build_engine(args.clone(), client, player),
            PlayerArgs::Human(_) => self.clone().build_human(&mut client.lock().unwrap(), player),
        }
    }

    pub fn build_human<B: Board>(
        self,
        ugi_client: &mut MutexGuard<Client<B>>,
        replace: Option<PlayerId>,
    ) -> Res<PlayerId> {
        assert!(matches!(self.args, PlayerArgs::Human(_)));
        let PlayerArgs::Human(ref args) = self.args else { panic!() };
        let tc = args.tc.unwrap_or(TimeControl::infinite());
        let human = Human(HumanPlayer {
            tc,
            original_tc: tc,
            name: args.name.clone().unwrap_or_else(|| realname().unwrap_or("Human".to_string())),
            status: HumanPlayerStatus::Idle,
        });
        let res = match replace {
            None => ugi_client.add_player(human, self.clone()),
            Some(player) => {
                ugi_client.state.players[player] = human;
                player
            }
        };
        Ok(res)
        // No separate input thread as that is handled by the UI
    }

    fn get_engine_path_and_set_args(args: &mut ClientEngineCliArgs, game_name: &str) -> Res<PathBuf> {
        if cfg!(feature = "motors") && args.path.is_none() {
            if args.cmd.is_empty() {
                // the 'motors-' prefix is used to denote an engine that's build in into monitors;
                // so this becomes 'monitors engine default'
                if let Some(name) = &args.display_name {
                    bail!(
                        "The engine name is set ('{name}') but the command isn't. Please specify a command (use 'motors-<name>' to use the built-in engine <name>)"
                    )
                }
                args.cmd = "motors-default".to_string();
            }
            if let Some(engine) = args.cmd.strip_prefix("motors-") {
                args.engine_args.push("motors".to_string());
                args.engine_args.push("--engine".to_string());
                args.engine_args.push(engine.to_string());
                args.engine_args.push("--game".to_string());
                args.engine_args.push(game_name.to_string());
                args.display_name = Some(args.display_name.clone().unwrap_or(engine.to_string()));
                if args.add_debug_flag && !args.engine_args.contains(&"--debug".to_string()) {
                    args.engine_args.push("--debug".to_string()); // non-built-in engines might not support --debug
                }
                return Ok(current_exe().unwrap_or(PathBuf::from_str("monitors").unwrap()));
            }
        }
        let mut path = args.path.clone().unwrap_or_default();
        path.set_file_name(args.cmd.clone());
        Ok(path)
    }

    fn build_engine<B: Board>(
        self,
        mut args: ClientEngineCliArgs,
        client: Arc<Mutex<Client<B>>>,
        replace: Option<PlayerId>,
    ) -> Res<PlayerId> {
        let copy = self;
        let path = Self::get_engine_path_and_set_args(&mut args, &B::game_name())?;
        if !path.is_file() {
            bail!(
                "The specified engine path '{}' does not point to a file (make sure the path and command are set correctly)",
                path.as_os_str().to_str().unwrap_or("<invalid>")
            )
        }
        let display_name = NameSet::make_name_unique(args.display_name.unwrap_or(args.cmd));
        let stderr_output = match args.stderr {
            None => PathBuf::from_str(&format!("{display_name}_stderr.log"))?,
            Some(path) => path,
        };

        client.lock().unwrap().show_message(
            Debug,
            &format_args!(
                "Initializing engine '{display_name}' with command '{cmd}' and options {:?}",
                args.engine_args,
                cmd = path.to_str().unwrap_or("<unknown>")
            ),
        );
        let mut child = Command::new(path.as_path())
            .args(args.engine_args.clone())
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(File::create(stderr_output)?)
            .spawn()?;
        let stdout = child.stdout.take().unwrap();
        let tc = args.tc.unwrap_or_else(|| TimeControl {
            remaining: Duration::from_millis(2000),
            increment: Duration::from_millis(400),
            moves_to_go: None,
        });
        let fixed_time = args.move_time.unwrap_or(Duration::MAX);
        let depth = args.depth.unwrap_or(DepthPly::MAX);
        let mate = args.mate.unwrap_or(DepthPly::MAX);
        let nodes = args.nodes.unwrap_or(NodesLimit::MAX);
        let soft_nodes = NodesLimit::MAX;
        let default_limit = SearchLimit { tc, fixed_time, depth, nodes, soft_nodes, mate, start_time: Instant::now() };

        // try to set uci/ugi mode based on the game, but possibly change that according to how the engine responds
        let proto =
            args.proto.unwrap_or_else(|| if TypeId::of::<B>() == TypeId::of::<Chessboard>() { Uci } else { Ugi });

        let engine = EnginePlayer::new(
            proto,
            default_limit,
            args.time_margin.unwrap_or_default(),
            args.white_pov,
            display_name.clone(),
            child,
        );

        let id = match replace {
            None => client.lock().unwrap().add_player(Engine(engine), copy),
            Some(id) => {
                client.lock().unwrap().state.players[id] = Engine(engine);
                id
            }
        };

        let weak = Arc::downgrade(&client);
        Builder::new()
            .name(format!("UGI input from engine {display_name}"))
            .spawn(move || {
                InputThread::run_ugi_player_input_thread(id, weak, BufReader::new(stdout));
            })
            .unwrap();

        send_initial_ugi(client, id, args.init_string, &args.custom_options)?;

        Ok(id)
    }
}

pub enum PlayerLimit {
    Human(TimeControl),
    Engine(SearchLimit),
}

#[derive(Debug)]
#[expect(clippy::large_enum_variant)]
pub enum Player<B: Board> {
    /// The usize is te index into the vec of engines stored in the match.
    Engine(EnginePlayer<B>),
    Human(HumanPlayer),
}

impl<B: Board> Player<B> {
    pub fn reset(&mut self) {
        match self {
            Engine(e) => {
                e.current_match = None;
                if !matches!(e.status, WaitingUgiOk) {
                    e.status = Idle;
                }
                // TODO: Iirc cutechess has an option to restart the entire engine process, which should also be implemented here
            }
            Human(h) => h.tc = h.original_tc,
        }
    }

    pub fn assign_to_match(&mut self, color: B::Color) {
        match self {
            Engine(engine) => {
                engine.current_match = Some(CurrentMatch::new(engine.default_limit, color));
            }
            Human(human) => human.tc = human.original_tc,
        }
    }

    pub fn get_name(&self) -> &str {
        match self {
            Engine(engine) => &engine.display_name,
            Human(human) => &human.name,
        }
    }

    pub fn set_time(&mut self, new_tc: TimeControl) -> Res<()> {
        match self {
            Engine(engine) => {
                let Some(m) = engine.current_match.as_mut() else {
                    bail!("Engine {} isn't currently playing a match", engine.display_name)
                };
                m.limit.tc = new_tc;
            }
            Human(human) => human.tc = new_tc,
        }
        Ok(())
    }

    pub fn get_time(&self) -> Option<TimeControl> {
        match self {
            Engine(engine) => engine.current_match.as_ref().map(|m| m.limit.tc),
            Human(human) => Some(human.tc),
        }
    }

    pub fn get_limit(&self) -> Option<PlayerLimit> {
        match self {
            Engine(engine) => engine.current_match.as_ref().map(|m| PlayerLimit::Engine(m.limit)),
            Human(human) => Some(PlayerLimit::Human(human.tc)),
        }
    }

    pub fn get_original_tc(&self) -> TimeControl {
        match self {
            Engine(engine) => engine.current_match.as_ref().unwrap().original_limit.tc,
            Human(human) => human.original_tc,
        }
    }

    pub fn start_clock(&mut self) {
        let start = Instant::now();
        match self {
            Engine(engine) => engine.status = ThinkingSince(start),
            Human(human) => human.status = HumanPlayerStatus::ThinkingSince(start),
        }
    }

    pub fn is_engine(&self) -> bool {
        match self {
            Engine(_) => true,
            Human(_) => false,
        }
    }

    pub fn thinking_since(&self) -> Option<Instant> {
        match self {
            Engine(engine) => match engine.status {
                ThinkingSince(start) | Halt(Play(start)) | Ping(start) => Some(start),
                _ => None,
            },
            Human(human) => match human.status {
                HumanPlayerStatus::Idle => None,
                HumanPlayerStatus::ThinkingSince(start) => Some(start),
            },
        }
    }

    pub fn update_clock_and_check_for_time_loss(&mut self) -> bool {
        let elapsed = self.thinking_since().expect("Tried to stop the clock of a player who wasn't thinking").elapsed();
        match self {
            // An engine needs to check both the fixed move time and the TimeControl, but a human only has a TimeControl
            Engine(engine) => {
                let limit = engine.current_match.as_ref().unwrap().limit;
                if elapsed > limit.max_move_time().saturating_add(engine.time_margin.0) {
                    return true;
                }
                engine.current_match().limit.tc.update(elapsed);
            }
            Human(human) => {
                if elapsed > human.tc.remaining {
                    return true;
                }
                human.tc.update(elapsed);
            }
        }
        false
    }

    pub fn transfer_time(&mut self, limit: PlayerLimit, original_tc: TimeControl) {
        match self {
            Engine(engine) => {
                engine.current_match().original_limit.tc = original_tc;
                match limit {
                    PlayerLimit::Human(tc) => engine.current_match().limit.tc = tc,
                    PlayerLimit::Engine(limit) => engine.current_match().limit = limit,
                };
            }
            Human(human) => {
                human.original_tc = original_tc;
                human.tc = match limit {
                    PlayerLimit::Human(tc) => tc,
                    PlayerLimit::Engine(limit) => limit.tc,
                };
            }
        }
    }
}
