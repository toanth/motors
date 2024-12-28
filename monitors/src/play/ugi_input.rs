// TODO: Ensure the UCI implementation conforms to https://expositor.dev/uci/doc/uci-draft-1.pdf and works with Stockfish and Leela.

use std::fmt::{Display, Formatter};
use std::io::{BufRead, BufReader};
use std::ops::Add;
use std::process::ChildStdout;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::Instant;

use itertools::Itertools;

use crate::play::player::Protocol::{Uci, Ugi};
use crate::play::player::{EnginePlayer, Protocol};
use crate::play::ugi_client::{Client, PlayerId};
use crate::play::ugi_input::EngineStatus::*;
use crate::play::ugi_input::HandleBestMove::{Ignore, Play};
use gears::colored::Colorize;
use gears::general::board::Board;
use gears::general::common::anyhow::{anyhow, bail};
use gears::general::common::{parse_duration_ms, parse_int_from_str, tokens, Res, Tokens};
use gears::general::moves::Move;
use gears::output::Message::*;
use gears::score::{ScoreT, SCORE_LOST, SCORE_WON};
use gears::search::{Depth, NodesLimit, SearchInfo, SearchLimit};
use gears::ugi::EngineOptionType::*;
use gears::ugi::{EngineOption, EngineOptionName, UgiCheck, UgiCombo, UgiSpin, UgiString};
use gears::MatchStatus::Over;
use gears::{
    player_res_to_match_res, AdjudicationReason, GameOver, GameOverReason, MatchStatus,
    PlayerResult,
};
// TODO: Does not currently handle engines that simply don't terminate the search (unless the user inputs 'stop')
// (not receiving ugiok/uiok is handled, as is losing on time with a bestmove response,
// but non-responding engines currently require user intervention)

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum HandleBestMove {
    Ignore,
    Play(Instant),
}

#[derive(Debug, Copy, Clone)]
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
            ThinkingSince(time) | Ping(time) | Halt(Play(time)) => Some(*time),
            Idle | Sync | WaitingUgiOk | Halt(Ignore) => None,
        }
    }
}

#[derive(Debug)]
#[must_use]
pub struct CurrentMatch<B: Board> {
    pub search_info: Option<SearchInfo<B>>,
    pub limit: SearchLimit,
    pub original_limit: SearchLimit,
    // TODO: Maybe only store color as part of ThinkingSince? Ugi doesn't care if the color changes.
    /// On the other hand, there's no real need to support such a use case, and it would add some extra complexity.
    pub color: B::Color,
}

impl<B: Board> CurrentMatch<B> {
    pub fn new(limit: SearchLimit, color: B::Color) -> Self {
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
        .ok_or_else(|| anyhow!("The player tried to access a match which was already cancelled"))
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

        loop {
            if let Err(e) = engine_input_data.main_loop() {
                let keep_running = engine_input_data.deal_with_error(&e.to_string());
                if !keep_running {
                    return;
                }
            }
        }
    }

    fn deal_with_error(&mut self, error: &str) -> bool {
        let id = self.id;
        let name;
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
            .map_err(|err| anyhow!("Couldn't read input: {err}"))?
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
            bail!("The connection was closed. This probably means that the engine process crashed. Check the engine \
            logs for details ('{name}_stderr.log' and possibly 'debug_output_engine_{name}.log').")
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
        let mut words = tokens(ugi_str);
        // If the client doesn't exist anymore, this thread will join without printing an error message
        // But still return a useful error message to be on the safe side.
        let Some(client) = self.upgrade_client() else {
            bail!("Client no longer exists")
        };
        let mut client = client.lock().unwrap();
        let player = self.get_engine(&client);
        let engine_name = player.display_name.clone();
        for output in &mut client.outputs {
            output.write_ugi_input(words.clone(), Some(&engine_name));
        }
        let player = self.get_engine(&client);
        let status = player.status.clone();
        let c = player
            .current_match
            .as_ref()
            .map(|c| c.color)
            .ok_or_else(|| {
                anyhow!("The engine '{engine_name}' is not currently playing in a match")
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
                bail!("Invalid UGI message ('{ugi_str}') from engine '{engine_name}' while in state {status}: {err}",
                                   ugi_str = command.to_string().add(" ").add(&words.join(" ")).red())
            }
        }
        // Empty uci commands should be ignored, according to the spec
        Ok(client.match_state().status.clone())
    }

    fn handle_ugi_initial_state(
        command: &str,
        words: Tokens,
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
        words: Tokens,
        client: &mut MutexGuard<Client<B>>,
        engine: PlayerId,
    ) -> Res<()> {
        match command {
            "info" => Self::handle_info(words, client, engine),
            _ => bail!("Only 'info' is a valid engine message while in idle state"),
        }
    }

    fn handle_ugi_sync_state(
        command: &str,
        words: Tokens,
        client: &mut MutexGuard<Client<B>>,
        engine: PlayerId,
    ) -> Res<()> {
        match command {
            "info" => Self::handle_info(words, client, engine),
            "readyok" => Self::handle_readyok(words, client, engine),
            _ => bail!("Only 'info' or 'readyok' are valid responses after sending 'isready' in idle state")
        }
    }

    fn handle_ugi_active_state(
        command: &str,
        words: Tokens,
        client: &mut MutexGuard<Client<B>>,
        color: B::Color,
    ) -> Res<()> {
        match command {
            "info" => Self::handle_info(words, client, client.state.id(color)),
            "bestmove" => Self::handle_bestmove(words, client, color),
            _ => {
                bail!("Only 'info' or 'bestmove' are valid engine messages while running")
            }
        }
    }

    fn handle_ugi_ping_state(
        command: &str,
        words: Tokens,
        client: &mut MutexGuard<Client<B>>,
        color: B::Color,
    ) -> Res<()> {
        match command {
            "info" => Self::handle_info(words, client, client.state.id(color)),
            "readyok" => Self::handle_readyok(words, client, client.state.id(color)),
            "bestmove" => Self::handle_bestmove(words, client, color),
            _ => bail!("Only 'info', 'readyok' or 'bestmove' are valid engine responses after sending 'isready' while running")
        }
    }

    fn handle_ugi_halt_state(
        command: &str,
        words: Tokens,
        client: &mut MutexGuard<Client<B>>,
        color: B::Color,
        handle_best_move: HandleBestMove,
    ) -> Res<()> {
        match command {
            "info" => { /*ignore*/ }
            "bestmove" => {
                if matches!(handle_best_move, Play(_)) {
                    Self::handle_bestmove(words, client, color)?;
                }
                client.state.get_engine_mut(color).status = Idle;
            }
            _ => {
                bail!(
                    "Only 'info' or 'bestmove' are valid engine messages while in the 'halt' state"
                )
            }
        }
        Ok(())
    }

    fn handle_protocol(
        mut words: Tokens,
        client: &mut MutexGuard<Client<B>>,
        name: &str,
    ) -> Res<()> {
        let Some(version) = words.next() else {
            bail!("Expected a single word after 'protocol', but the engine message just ends there")
        };
        if let Some(next) = words.next() {
            bail!("Expected a single word after 'protocol' (which would have been '{version}'), but it's followed by '{next}'")
        }
        client.show_message(
            Debug,
            &format!("protocol version of engine '{name}': '{version}'"),
        );
        Ok(())
    }

    fn handle_id(mut id: Tokens, client: &mut MutexGuard<Client<B>>, engine: PlayerId) -> Res<()> {
        let first = id
            .next()
            .ok_or_else(|| anyhow!("Line ends after 'id'"))?
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
        mut words: Tokens,
        client: &mut MutexGuard<Client<B>>,
        engine: PlayerId,
        proto: Protocol,
    ) -> Res<()> {
        if let Some(next) = words.next() {
            // The spec demands that the message must be ignored in such a case, but showing an error (while continuing to interact)
            // seems like a more prudent course of action (from the engine's point of view, the client ignores the message)
            bail!("Additional word {next} after ugiok or uciok")
        }
        let engine = client.state.get_engine_from_id_mut(engine);
        engine.status = Idle;
        engine.proto = proto;
        Ok(())
    }

    fn handle_readyok(
        mut words: Tokens,
        client: &mut MutexGuard<Client<B>>,
        engine: PlayerId,
    ) -> Res<()> {
        let engine = client.state.get_engine_from_id_mut(engine);
        assert_eq!(engine.status, Sync); // Otherwise, this function wouldn't get called
        if let Some(next) = words.next() {
            bail!("Engine message doesn't end after 'readyok', the next word is {next}")
        }
        if engine.status == Sync {
            engine.status = Idle;
        }
        Ok(())
    }

    fn handle_bestmove(
        mut words: Tokens,
        client: &mut MutexGuard<Client<B>>,
        color: B::Color,
    ) -> Res<()> {
        // Stop the clock as soon as possible to minimize the affect client overhead has on the engine's time control.
        // If it turns out that the move was actually invalid, the engine lost anyway.
        client.stop_clock(color);
        let engine = client.state.get_engine_mut(color);
        engine.status = Idle;

        let Some(move_text) = words.next() else {
            bail!("missing move after 'bestmove'")
        };
        match B::Move::from_text(move_text, client.board()) {
            Err(err) => {
                let game_over = GameOver {
                    result: PlayerResult::Lose,
                    reason: GameOverReason::Adjudication(AdjudicationReason::InvalidMove),
                };
                client.game_over(player_res_to_match_res(game_over, color));
                return Err(err);
            }
            Ok(chosen_mov) => {
                if let Err(err) = client.play_move(chosen_mov) {
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

    fn handle_info(words: Tokens, client: &mut MutexGuard<Client<B>>, engine: PlayerId) -> Res<()> {
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
            let Some(value) = words.next() else {
                bail!("info line ends after '{key}', expected a value")
            };
            match key {
                "depth" => res.depth = Depth::try_new(parse_int_from_str(value, "depth")?)?,
                "seldepth" => {
                    res.seldepth = Depth::try_new(parse_int_from_str(value, "seldepth")?)?
                }
                "time" => {
                    res.time = parse_duration_ms(&mut tokens(value), "time")?;
                }
                "nodes" => {
                    res.nodes = NodesLimit::new(parse_int_from_str(value, "nodes")?).unwrap();
                }
                "pv" => {
                    match B::Move::from_compact_text(value, &board) {
                        Ok(mov) => pv_moves.push(mov),
                        Err(err) => {
                            bail!("'pv' needs to be followed by a valid move, but '{value}' isn't: {err}")
                        }
                    }
                    loop {
                        let undo_consume_word = words.clone();
                        let word = words.next();
                        if word.is_none() {
                            break;
                        }
                        if let Ok(mov) = B::Move::from_compact_text(word.unwrap(), &board) {
                            pv_moves.push(mov);
                        } else {
                            words = undo_consume_word;
                            break;
                        }
                    }
                }
                "score" => match value {
                    "cp" | "lowerbound" | "upperbound" => {
                        res.score.0 = parse_int_from_str(
                            words
                                .next()
                                .ok_or_else(|| anyhow!("missing score value after 'score cp'"))?,
                            "cp",
                        )?;
                    }
                    "mate" => {
                        let value: ScoreT = parse_int_from_str(
                            words
                                .next()
                                .ok_or_else(|| anyhow!("missing ply value after 'score mate'"))?,
                            "mate",
                        )?;
                        res.score = if value >= 0 {
                            SCORE_WON - value
                        } else {
                            SCORE_LOST + value
                        }
                    }
                    _ => bail!("Unrecognized `score` type"),
                },
                "nps" => _ = parse_int_from_str::<usize>(value, "nps")?,
                "string" => {
                    let msg = value.to_string().add(" ").add(&words.join(" "));
                    client.show_ugi_info_string(engine, &msg);
                }
                "error" => {
                    let message = value.to_string().add(" ").add(&words.join(" "));
                    bail!("The engine sent a UGI error: '{message}'")
                }
                // TODO: Handle multipv and some other info
                "refutation" | "currline" | "multipv" => { /*completely ignored*/ }
                "currmove" | "currmovenumber" | "hashfull" | "tbhits" | "sbhits" | "cpuload" => {}
                _ => {}
            }
            if !pv_moves.is_empty() {
                // handle multiple 'pv' in one response
                res.pv = pv_moves;
                pv_moves = Vec::default();
            }
        }
        client.update_info(engine, res)
    }

    fn handle_option(
        mut option: Tokens,
        client: &mut MutexGuard<Client<B>>,
        engine: PlayerId,
    ) -> Res<()> {
        let Some(word) = option.next() else {
            bail!("Line ended after 'option'")
        };
        if word != "name" {
            bail!("expected 'name' after 'option', got '{word}'")
        }
        let mut res = EngineOption::default();
        // TODO: Technically, the spec demands to accept multi-token names
        let Some(name) = option.next() else {
            bail!("Line ended after 'name', missing the option name")
        };
        res.name = EngineOptionName::from_str(name)?;
        let Some(typ) = option.next() else {
            bail!("Line ended after option name, missing 'type'")
        };
        if typ != "type" {
            bail!("Expected 'type' after option name, got {typ}")
        }
        let Some(typ) = option.next() else {
            bail!("Line ended after 'type', missing the option type")
        };
        match typ {
            "check" => res.value = Check(UgiCheck::default()),
            "spin" => res.value = Spin(UgiSpin::default()),
            "combo" => res.value = Combo(UgiCombo::default()),
            "button" => res.value = Button,
            "string" => res.value = UString(UgiString::default()),
            x => bail!("Unrecognized option type {x}"),
        }
        loop {
            let next = option.next();
            if next.is_none() {
                break;
            }
            let setting = next.unwrap();
            let Some(mut value) = option.next() else {
                bail!("Missing value after option {setting}")
            };
            match setting.to_lowercase().as_str() {
                "default" => match &mut res.value {
                    Check(c) => match value.to_lowercase().as_str() {
                        "true" | "on" => c.default = Some(true),
                        "false" | "off" => c.default = Some(false),
                        _ => bail!("Unrecognized check value '{value}', should be 'true' or 'false'"),
                    }
                    Spin(s) => s.default = Some(parse_int_from_str(value, &format!("{} default value", res.name))?),
                    Combo(c) => c.default = Some(value.to_string()),
                    Button => bail!("option {} has type 'Button' and can't have a default value", res.name),
                    UString(s) => {
                        if value == "<empty>" {
                            value = "";
                        }
                        s.default = Some(value.to_string());
                    }
                },
                "min" => match &mut res.value {
                    Spin(s) => s.min = Some(parse_int_from_str(value, &format!("{} min value", res.name))?),
                    _ => bail!("option {} has type '{}' and can't have a min value", res.name, res.value.type_to_str())
                },
                "max" => match &mut res.value {
                    Spin(s) => s.max = Some(parse_int_from_str(value, &format!("{} max value", res.name))?),
                    _ => bail!("option {} has type '{}' and can't have a max value", res.name, res.value.type_to_str())
                },
                "var" => match &mut res.value {
                    Combo(c) => c.options.push(value.to_string()),
                    _ => bail!("option {} has type '{}' and can't have a 'var' value (only 'combo' can have that)", res.name, res.value.type_to_str())
                },
                _ => bail!("Unrecognized parameter '{setting}' for option '{}'", res.name),
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
