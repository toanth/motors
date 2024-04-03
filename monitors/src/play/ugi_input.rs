// TODO: Ensure the UCI implementation conforms to https://expositor.dev/uci/doc/uci-draft-1.pdf and works with Stockfish and Leela.

use std::fmt::{Display, Formatter};
use std::io::{BufRead, BufReader};
use std::ops::Add;
use std::process::ChildStdout;
use std::str::{FromStr, SplitWhitespace};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::{Duration, Instant};

use colored::Colorize;
use itertools::Itertools;

use gears::{
    AdjudicationReason, GameOver, GameOverReason, MatchStatus, player_res_to_match_res,
    PlayerResult,
};
use gears::games::{Board, Color, Move};
use gears::general::common::{parse_int_from_str, Res};
use gears::MatchStatus::Over;
use gears::output::Message::Debug;
use gears::search::{Depth, Nodes, SCORE_LOST, SCORE_WON, SearchInfo, SearchLimit};
use gears::ugi::{EngineOption, EngineOptionName, UgiCheck, UgiCombo, UgiSpin, UgiString};
use gears::ugi::EngineOptionType::*;

use crate::play::player::{EnginePlayer, Protocol};
use crate::play::player::Protocol::{Uci, Ugi};
use crate::play::ugi_client::{Client, PlayerId};
use crate::play::ugi_input::EngineStatus::*;
use crate::play::ugi_input::HandleBestMove::{Ignore, Play};

// TODO: Does not currently handle engines that simply don't terminate the search (unless the user inputs 'stop')
// (not receiving ugiok/uiok is handled, as is losing on time with a bestmove response,
// but non-responding engines currently require user intervention)

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum HandleBestMove {
    Ignore,
    Play(Instant),
}

pub enum BestMoveAction {
    /// Ignore the bestmove uci message sent by the engine upon receiving 'stop'.
    /// This is called in some failure cases to allow shutting down the engine
    /// and should therefore work with as few preconditions as possible, but it's also called in some
    /// other cases, such as a user inputting a valid move while an engine is thinking.
    Ignore,
    /// Play the bestmove returned from the engine. This is used if the user manually stops the engine
    /// because they're impatient (or because they inadvertently ran it with infinite time)
    Play,
}

#[derive(Debug, Default, PartialEq, Clone)]
pub enum EngineStatus {
    ThinkingSince(Instant),
    Ping(Instant),
    Idle,
    Sync,
    #[default]
    WaitingUgiOk,
    Halt(HandleBestMove),
}

impl Display for EngineStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            ThinkingSince(start) => {
                format!("thinking (since {} ms ago)", start.elapsed().as_millis())
            }
            Idle => "idle".to_string(),
            WaitingUgiOk => "initializing, waiting for 'ugiok'".to_string(),
            Halt(_) => "quit".to_string(),
            Sync => "waiting for 'readyok')".to_string(),
            Ping(start) => format!("thinking (since {} ms ago), waiting for the engine to answer 'isready' with 'readyok'", start.elapsed().as_millis())
        };
        write!(f, "{str}")
    }
}

impl EngineStatus {
    pub fn halt(&mut self, action: BestMoveAction) {
        match action {
            BestMoveAction::Ignore => *self = Halt(Ignore),
            BestMoveAction::Play => *self = Halt(Play(self.thinking_since().expect(
                "Called 'halt' without ignoring the best move on an engine that wasn't thinking",
            ))),
        }
    }

    pub fn thinking_since(&mut self) -> Option<Instant> {
        match self {
            ThinkingSince(time) => Some(time.clone()),
            Ping(time) => Some(time.clone()),
            Idle => None,
            Sync => None,
            WaitingUgiOk => None,
            Halt(Play(time)) => Some(time.clone()),
            Halt(Ignore) => None,
        }
    }
}

#[derive(Debug)]
pub struct CurrentMatch<B: Board> {
    pub search_info: Option<SearchInfo<B>>,
    pub limit: SearchLimit,
    pub original_limit: SearchLimit,
    // TODO: Maybe only store color as part of ThinkingSince? Ugi doesn't care if the color changes.
    /// On the other hand, there's no real need to support such a use case, and it would add some extra complexity.
    pub color: Color,
}

impl<B: Board> CurrentMatch<B> {
    pub fn new(limit: SearchLimit, color: Color) -> Self {
        Self {
            search_info: None,
            limit,
            original_limit: limit,
            color,
        }
    }
}

pub fn access_client<B: Board>(client: Weak<Mutex<Client<B>>>) -> Res<Arc<Mutex<Client<B>>>> {
    client
        .upgrade()
        .ok_or_else(|| "The player tried to access a match which was already cancelled".to_string())
}

pub struct InputThread<B: Board> {
    id: usize,
    client: Weak<Mutex<Client<B>>>,
    child_stdout: BufReader<ChildStdout>,
}

impl<B: Board> InputThread<B> {
    pub fn run_ugi_player_input_thread(
        id: usize,
        client: Weak<Mutex<Client<B>>>,
        child_stdout: BufReader<ChildStdout>,
    ) {
        let mut engine_input_data = Self {
            id,
            client,
            child_stdout,
        };

        if let Err(e) = engine_input_data.main_loop() {
            let keep_running = engine_input_data.deal_with_error(&e);
            if !keep_running {
                return;
            }
        }
    }

    fn deal_with_error(&mut self, error: &str) -> bool {
        let id = self.id;
        let mut name = String::default();
        match self.upgrade_client() {
            // If we can't access the client, this means there's no match running right now, so assume the program has been terminated
            None => return false,
            Some(client) => {
                {
                    let mut client = client.lock().unwrap();
                    name = client.state.get_engine_from_id_mut(id).display_name.clone();
                    client.show_error(&format!(
                        "The engine '{name}' encountered an error: {error}"
                    ));
                    if !client.state.recover {
                        // Only try to restart crashed engines if the user has explicitly enabled this (TODO: enable by default for the GUI)
                        return false;
                    }
                }
                if let Err(err) = Client::hard_reset_player(client.clone(), self.id) {
                    client.lock().unwrap().show_error(&format!("Error: Could not restart engine '{name}' after it encountered an error: {err}"));
                    return false; // All hope is lost.
                }
                let mut client = client.lock().unwrap();
                let player = client.state.get_engine_from_id(id); // make the borrow checker happy by getting the player again
                let res = GameOver {
                    result: PlayerResult::Lose,
                    reason: GameOverReason::Adjudication(AdjudicationReason::EngineError),
                };
                if let Some(current_match) = player.current_match.as_ref() {
                    let color = current_match.color;
                    client.game_over(player_res_to_match_res(res, color));
                }
            }
        }
        true
    }

    fn get_input(&mut self) -> Res<String> {
        let mut str = String::default();
        if self
            .child_stdout
            .read_line(&mut str)
            .map_err(|err| format!("Couldn't read input: {}", err.to_string()))?
            == 0
        {
            let name = self
                .upgrade_client()
                .map(|c| {
                    c.lock()
                        .unwrap()
                        .state
                        .get_engine_from_id(self.id)
                        .display_name
                        .clone()
                })
                .unwrap_or_default();
            return Err(format!("The connection was closed. This probably means that the engine process crashed. Check the engine logs for details ('{name}_stderr.log' and possibly 'debug_output_engine_{name}.log')."));
        }
        Ok(str)
    }

    fn upgrade_client(&mut self) -> Option<Arc<Mutex<Client<B>>>> {
        self.client.upgrade()
    }

    fn get_engine<'a>(&self, client: &'a MutexGuard<Client<B>>) -> &'a EnginePlayer<B> {
        client.state.get_engine_from_id(self.id)
    }

    fn main_loop(&mut self) -> Res<()> {
        loop {
            let input = self.get_input()?;
            // Now that we have input, lock the client until the input has been handled.
            let status = self.handle_ugi(input.trim())?;
            if let Over(_) = status {
                break;
            }
        }
        Ok(())
    }

    fn handle_ugi(&mut self, ugi_str: &str) -> Res<MatchStatus> {
        let mut words = ugi_str.split_whitespace();
        // If the client doesn't exist anymore, this thread will join without printing an error message
        let client = self.upgrade_client().ok_or_else(|| String::default())?;
        let mut client = client.lock().unwrap();
        let player = self.get_engine(&client);
        let engine_name = player.display_name.clone();
        for output in client.outputs.iter_mut() {
            output.write_ugi_input(words.clone(), Some(&engine_name));
        }
        let player = self.get_engine(&client);
        let status = player.status.clone();
        let c = player
            .current_match
            .as_ref()
            .map(|c| c.color)
            .ok_or_else(|| {
                format!("The engine '{engine_name}' is not currently playing in a match")
            });
        if let Some(command) = words.next() {
            if let Err(err) = match status {
                ThinkingSince(_) => {
                    Self::handle_ugi_active_state(command, words.clone(), &mut client, c?)
                }
                // In the idle or sync state, it's possible that the engine isn't participating in a match, so don't use the color to refer to it.
                Idle => Self::handle_ugi_idle_state(command, words.clone(), &mut client, self.id),
                Sync => Self::handle_ugi_sync_state(command, words.clone(), &mut client, self.id),
                Ping(_) => Self::handle_ugi_ping_state(command, words.clone(), &mut client, c?),
                // In the initial state, the engine is not participating in a match, so it can't be accessed using its color
                WaitingUgiOk => {
                    Self::handle_ugi_initial_state(command, words.clone(), &mut client, self.id)
                }
                Halt(handle_bestmove) => Self::handle_ugi_halt_state(
                    command,
                    words.clone(),
                    &mut client,
                    c?,
                    handle_bestmove,
                ),
            } {
                return Err(format!("Invalid UGI message ('{ugi_str}') from engine '{engine_name}' while in state {status}: {err}",
                                   ugi_str = command.to_string().add(" ").add(&words.join(" ")).red()));
            }
        }
        // Empty uci commands should be ignored, according to the spec
        Ok(client.match_state().status.clone())
    }

    fn handle_ugi_initial_state(
        command: &str,
        words: SplitWhitespace,
        client: &mut MutexGuard<Client<B>>,
        engine: PlayerId,
    ) -> Res<()> {
        match command {
            "id" => Self::handle_id(words, client, engine),
            "option" => Self::handle_option(words, client, engine),
            "protocol" => Self::handle_protocol(
                words,
                client,
                &client.state.get_engine_from_id(engine).display_name.clone(),
            ),
            "info" => Self::handle_info(words, client, engine),
            "uciok" => Self::handle_ugiok(words, client, engine, Uci),
            "ugiok" => Self::handle_ugiok(words, client, engine, Ugi),
            _ => {
                /*ignore the message completely, without showing any warning or error messages*/
                Ok(())
            }
        }
    }

    fn handle_ugi_idle_state(
        command: &str,
        words: SplitWhitespace,
        client: &mut MutexGuard<Client<B>>,
        engine: PlayerId,
    ) -> Res<()> {
        match command {
            "info" => Self::handle_info(words, client, engine),
            _ => Err("Only 'info' is a valid engine message while in idle state".to_string()),
        }
    }

    fn handle_ugi_sync_state(
        command: &str,
        words: SplitWhitespace,
        client: &mut MutexGuard<Client<B>>,
        engine: PlayerId,
    ) -> Res<()> {
        match command {
            "info" => Self::handle_info(words, client, engine),
            "readyok" => Self::handle_readyok(words, client, engine),
            _ => Err("Only 'info' or 'readyok' are valid responses after sending 'isready' in idle state".to_string())
        }
    }

    fn handle_ugi_active_state(
        command: &str,
        words: SplitWhitespace,
        client: &mut MutexGuard<Client<B>>,
        color: Color,
    ) -> Res<()> {
        match command {
            "info" => Self::handle_info(words, client, client.state.id(color)),
            "bestmove" => Self::handle_bestmove(words, client, color),
            _ => {
                Err("Only 'info' or 'bestmove' are valid engine messages while running".to_string())
            }
        }
    }

    fn handle_ugi_ping_state(
        command: &str,
        words: SplitWhitespace,
        client: &mut MutexGuard<Client<B>>,
        color: Color,
    ) -> Res<()> {
        match command {
            "info" => Self::handle_info(words, client, client.state.id(color)),
            "readyok" => Self::handle_readyok(words, client, client.state.id(color)),
            "bestmove" => Self::handle_bestmove(words, client, color),
            _ => Err("Only 'info', 'readyok' or 'bestmove' are valid engine responses after sending 'isready' while running".to_string())
        }
    }

    fn handle_ugi_halt_state(
        command: &str,
        words: SplitWhitespace,
        client: &mut MutexGuard<Client<B>>,
        color: Color,
        handle_best_move: HandleBestMove,
    ) -> Res<()> {
        match command {
            "info" => { /*ignore*/ }
            "bestmove" => {
                if matches!(handle_best_move, Play(_)) {
                    Self::handle_bestmove(words, client, color)?
                }
                client.state.get_engine_mut(color).status = Idle;
            }
            _ => {
                return Err(
                    "Only 'info' or 'bestmove' are valid engine messages while in the 'halt' state"
                        .to_string(),
                )
            }
        }
        Ok(())
    }

    fn handle_protocol(
        mut words: SplitWhitespace,
        client: &mut MutexGuard<Client<B>>,
        name: &str,
    ) -> Res<()> {
        let Some(version) = words.next() else {
            return Err(
                "Expected a single word after 'protocol', but the engine message just ends there"
                    .to_string(),
            );
        };
        if let Some(next) = words.next() {
            return Err(format!("Expected a single word after 'protocol' (which would have been '{version}'), but it's followed by '{next}'"));
        }
        client.show_message(
            Debug,
            &format!("protocol version of engine '{}': '{version}'", name),
        );
        Ok(())
    }

    fn handle_id(
        mut id: SplitWhitespace,
        client: &mut MutexGuard<Client<B>>,
        engine: PlayerId,
    ) -> Res<()> {
        let first = id
            .next()
            .ok_or_else(|| "Line ends after 'id'".to_string())?
            .trim();
        let rest = id.join(" ").trim().to_string();
        let engine = client.state.get_engine_from_id_mut(engine);
        match first {
            "name" => engine.ugi_name = rest,
            "author" => engine.author = rest,
            _ => { /* ignore unrecognized keys */ }
        }
        Ok(())
    }

    fn handle_ugiok(
        mut words: SplitWhitespace,
        client: &mut MutexGuard<Client<B>>,
        engine: PlayerId,
        proto: Protocol,
    ) -> Res<()> {
        if let Some(next) = words.next() {
            // The spec demands that the message must be ignored in such a case, but showing an error (while continuing to interact)
            // seems like a more prudent course of action (from the engine's point of view, the client ignores the message)
            return Err(format!("Additional word {next} after ugiok or uciok"));
        }
        let engine = client.state.get_engine_from_id_mut(engine);
        engine.status = Idle;
        engine.proto = proto;
        Ok(())
    }

    fn handle_readyok(
        mut words: SplitWhitespace,
        client: &mut MutexGuard<Client<B>>,
        engine: PlayerId,
    ) -> Res<()> {
        let engine = client.state.get_engine_from_id_mut(engine);
        assert_eq!(engine.status, Sync); // Otherwise, this function wouldn't get called
        if let Some(next) = words.next() {
            return Err(format!(
                "Engine message doesn't end after 'readyok', the next word is {next}"
            ));
        }
        match &engine.status {
            Sync => engine.status = Idle,
            _ => {}
        }
        Ok(())
    }

    fn handle_bestmove(
        mut words: SplitWhitespace,
        client: &mut MutexGuard<Client<B>>,
        color: Color,
    ) -> Res<()> {
        // Stop the clock as soon as possible to minimize the affect client overhead has on the engine's time control.
        // If it turns out that the move was actually invalid, the engine lost anyway.
        client.stop_clock(color);
        let engine = client.state.get_engine_mut(color);
        engine.status = Idle;

        let mov = words
            .next()
            .ok_or_else(|| "missing move after 'bestmove'")?;
        match B::Move::from_text(mov, &client.board()) {
            Err(err) => {
                let game_over = GameOver {
                    result: PlayerResult::Lose,
                    reason: GameOverReason::Adjudication(AdjudicationReason::InvalidMove),
                };
                client.game_over(player_res_to_match_res(game_over, color));
                return Err(err);
            }
            Ok(mov) => {
                if let Err(err) = client.play_move(mov) {
                    let game_over = GameOver {
                        result: PlayerResult::Lose,
                        reason: GameOverReason::Adjudication(AdjudicationReason::InvalidMove),
                    };
                    client.game_over(player_res_to_match_res(game_over, color));
                    return Err(err);
                }
            }
        }
        Ok(())
    }

    fn handle_info(
        mut words: SplitWhitespace,
        client: &mut MutexGuard<Client<B>>,
        engine: PlayerId,
    ) -> Res<()> {
        let mut res = SearchInfo::default();
        let mut pv_moves = vec![];
        let board = *client.board();
        let mut words = words.peekable();
        if words.peek().is_some_and(|opt| *opt == "string") {
            words.next();
            let msg = words.join(" ");
            client.show_ugi_info_string(engine, &msg);
            return Ok(());
        }
        loop {
            let key = words.next();
            if key.is_none() {
                break;
            }
            let key = key.unwrap();
            let value = words
                .next()
                .ok_or_else(|| format!("info line ends after '{key}', expected a value"))?;
            match key {
                "depth" => res.depth = Depth::new(parse_int_from_str(value, "depth")?),
                "seldepth" => res.seldepth = Some(parse_int_from_str(value, "seldepth")?),
                "time" => res.time = Duration::from_millis(parse_int_from_str(value, "time")?),
                "nodes" => res.nodes = Nodes::new(parse_int_from_str(value, "nodes")?).unwrap(),
                "pv" => {
                    match B::Move::from_compact_text(value, &board) {
                        Ok(mov) => pv_moves.push(mov),
                        Err(err) => return Err(format!(
                            "'pv' needs to be followed by a valid move, but '{value}' isn't: {err}"
                        )),
                    }
                    loop {
                        let undo_consume_word = words.clone();
                        let word = words.next();
                        if word.is_none() {
                            break;
                        }
                        match B::Move::from_compact_text(word.unwrap(), &board) {
                            Ok(mov) => pv_moves.push(mov),
                            Err(_) => {
                                words = undo_consume_word;
                                break;
                            }
                        }
                    }
                }
                "score" => match value {
                    "cp" | "lowerbound" | "upperbound" => {
                        res.score.0 = parse_int_from_str(
                            words
                                .next()
                                .ok_or_else(|| "missing score value after 'score cp'")?,
                            "cp",
                        )?
                    }
                    "mate" => {
                        let value: i32 = parse_int_from_str(
                            words
                                .next()
                                .ok_or_else(|| "missing ply value after 'score mate'")?,
                            "mate",
                        )?;
                        res.score = if value >= 0 {
                            SCORE_WON - value
                        } else {
                            SCORE_LOST + value
                        }
                    }
                    _ => return Err("Unrecognized `score` type".to_string()),
                },
                "nps" => _ = parse_int_from_str::<usize>(value, "nps")?,
                "string" => {
                    let msg = value.to_string().add(" ").add(&words.join(" "));
                    client.show_ugi_info_string(engine, &msg);
                }
                "error" => {
                    let message = value.to_string().add(" ").add(&words.join(" "));
                    return Err(format!("The engine sent a UGI error: '{message}'"));
                }
                "refutation" | "currline" | "multipv" => { /*completely ignored*/ }
                "currmove" | "currmovenumber" | "hashfull" | "tbhits" | "sbhits" | "cpuload" => {}
                _ => {}
            }
            if !pv_moves.is_empty() {
                // handle multiple 'pv' in one response
                res.pv = pv_moves;
                pv_moves = Default::default();
            }
        }
        client.update_info(engine, res)
    }

    fn handle_option(
        mut option: SplitWhitespace,
        client: &mut MutexGuard<Client<B>>,
        engine: PlayerId,
    ) -> Res<()> {
        let word = option
            .next()
            .ok_or_else(|| "Line ended after 'option'".to_string())?;
        if word != "name" {
            return Err("expected 'name' after 'option', got '{word}'".to_string());
        }
        let mut res = EngineOption::default();
        // TODO: Technically, the spec demands to accept multi-token names
        let name = option
            .next()
            .ok_or_else(|| "Line ended after 'name', missing the option name".to_string())?;
        res.name = EngineOptionName::from_str(name)?;
        let typ = option
            .next()
            .ok_or_else(|| "Line ended after option name, missing 'type'".to_string())?;
        if typ != "type" {
            return Err("Expected 'type' after option name, got {typ}".to_string());
        }
        let typ = option
            .next()
            .ok_or_else(|| "Line ended after 'type', missing the option type".to_string())?;
        match typ {
            "check" => res.value = Check(UgiCheck::default()),
            "spin" => res.value = Spin(UgiSpin::default()),
            "combo" => res.value = Combo(UgiCombo::default()),
            "button" => res.value = Button,
            "string" => res.value = UString(UgiString::default()),
            x => return Err(format!("Unrecognized option type {x}")),
        }
        loop {
            let next = option.next();
            if next.is_none() {
                break;
            }
            let setting = next.unwrap();
            let mut value = option
                .next()
                .ok_or_else(|| "Missing value after option {setting}")?;
            match setting.to_lowercase().as_str() {
                "default" => match &mut res.value {
                    Check(c) => match value.to_lowercase().as_str() {
                        "true" | "on" => c.default = Some(true),
                        "false" | "off" => c.default = Some(false),
                        _ => return Err(format!("Unrecognized check value '{value}', should be 'true' or 'false'")),
                    }
                    Spin(s) => s.default = Some(parse_int_from_str(value, &format!("{} default value", res.name))?),
                    Combo(c) => c.default = Some(value.to_string()),
                    Button => return Err(format!("option {} has type 'Button' and can't have a default value", res.name)),
                    UString(s) => {
                        if value == "<empty>" {
                            value = "";
                        }
                        s.default = Some(value.to_string());
                    }
                },
                "min" => match &mut res.value {
                    Spin(s) => s.min = Some(parse_int_from_str(value, &format!("{} min value", res.name))?),
                    _ => return Err(format!("option {} has type '{}' and can't have a min value", res.name, res.value.type_to_str()))
                },
                "max" => match &mut res.value {
                    Spin(s) => s.max = Some(parse_int_from_str(value, &format!("{} max value", res.name))?),
                    _ => return Err(format!("option {} has type '{}' and can't have a max value", res.name, res.value.type_to_str()))
                },
                "var" => match &mut res.value {
                    Combo(c) => c.options.push(value.to_string()),
                    _ => return Err(format!("option {} has type '{}' and can't have a 'var' value (only 'combo' can have that)", res.name, res.value.type_to_str()))
                },
                _ => return Err(format!("Unrecognized parameter '{setting}' for option '{}'", res.name)),
            }
        }
        client
            .state
            .get_engine_from_id_mut(engine)
            .options
            .push(res);
        Ok(())
    }
}
