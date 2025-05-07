use std::fmt::{Debug, Display, Formatter};
use std::io::stdin;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::thread::{Builder, JoinHandle};

use itertools::Itertools;
use rand::rng;

use crate::cli::PlayerArgs::{Engine, Human};
use crate::cli::{HumanArgs, PlayerArgs, parse_engine, parse_human};
use crate::play::player::{Player, PlayerBuilder};
use crate::play::ugi_client::Client;
use crate::play::ugi_input::BestMoveAction::Play;
use crate::ui::text_input::DefaultPlayer::{Active, Inactive, NoPlayer};
use crate::ui::{Input, InputBuilder};
use gears::MatchStatus::{Ongoing, Over};
use gears::colored::Colorize;
use gears::games::Color;
use gears::general::board::Strictness::Relaxed;
use gears::general::board::{Board, BoardHelpers};
use gears::general::common::Description::{NoDescription, WithDescription};
use gears::general::common::anyhow::{anyhow, bail};
use gears::general::common::{
    NamedEntity, Res, StaticallyNamedEntity, Tokens, parse_int_from_str, select_name_static,
    to_name_and_optional_description, tokens,
};
use gears::general::moves::ExtendedFormat::Alternative;
use gears::general::moves::Move;
use gears::output::Message::{Info, Warning};
use gears::output::OutputOpts;
use gears::search::TimeControl;
use gears::ugi::{EngineOption, parse_ugi_position_part};
use gears::{GameState, output_builder_from_str};

// TODO: Unify with motors `Command`, probably move to gears
struct TextSelection<F> {
    names: Vec<&'static str>,
    func: F,
    description: Option<&'static str>,
}

impl<F> Debug for TextSelection<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.short_name())
    }
}

impl<F> NamedEntity for TextSelection<F> {
    fn short_name(&self) -> String {
        (*self.names.first().unwrap()).to_string()
    }

    fn long_name(&self) -> String {
        self.short_name()
    }

    fn description(&self) -> Option<String> {
        self.description.map(ToString::to_string)
    }

    fn matches(&self, name: &str) -> bool {
        self.names.iter().any(|n| n.eq_ignore_ascii_case(name))
    }
}

fn sel_descr<F>(names: Vec<&'static str>, func: F, description: &'static str) -> TextSelection<F> {
    TextSelection { names, func, description: Some(description) }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
enum DefaultPlayer {
    Active,
    Inactive,
    NoPlayer,
}

type Command<B> = TextSelection<for<'a> fn(MutexGuard<Client<B>>, &'a mut Tokens) -> Res<()>>;

pub(super) struct TextInputThread<B: Board> {
    commands: Vec<Command<B>>,
}

impl<B: Board> TextInputThread<B> {
    pub fn input_loop(ugi_client: Weak<Mutex<Client<B>>>) {
        let input_thread = Self {
            // created here so that this isn't done each time the user inputs something (which probably wouldn't matter
            // either, but I like this more)
            commands: vec![
                sel_descr(
                    vec!["quit"],
                    |mut client, _| {
                        client.quit_program();
                        Ok(())
                    },
                    "Exits the entire program",
                ),
                sel_descr(
                    vec!["abort", "resign"],
                    |mut client, _| {
                        client.abort_match();
                        Ok(())
                    },
                    "Aborts the current match",
                ),
                sel_descr(
                    vec!["stop"],
                    |client, words| Self::handle_stop(client, words),
                    "If an engine is currently thinking, tell it to stop and play the move it thinks is best",
                ),
                sel_descr(
                    vec!["restart"],
                    |mut client, _| {
                        client.restart();
                        Ok(())
                    },
                    "Restart the current match",
                ),
                sel_descr(
                    vec!["flip"],
                    |mut client, _| {
                        client.flip_players();
                        Ok(())
                    },
                    "Makes the players switch sides",
                ),
                sel_descr(
                    vec!["moves"],
                    |client, _| {
                        Self::list_moves(client);
                        Ok(())
                    },
                    "Lists all legal moves in the current position",
                ),
                sel_descr(
                    vec!["random"],
                    |client, _| Self::random_move(client),
                    "Plays a random legal move in the current position",
                ),
                sel_descr(
                    vec!["undo", "takeback"],
                    |client, words| Self::handle_undo(client, words),
                    "Takes back the last n half moves (default: n = 1), e.g. 'undo 2'",
                ),
                sel_descr(
                    vec!["ui", "output"],
                    |client, words| Self::handle_ui(client, words),
                    "Set or add outputs, e.g. 'ui ascii' or 'ui add fen'",
                ),
                sel_descr(
                    vec!["next_match"],
                    |mut client, _| {
                        client.restart_flipped_colors();
                        Ok(())
                    },
                    "Restarts the match with flipped colors",
                ),
                sel_descr(
                    vec!["print", "show"],
                    |client, words| Self::handle_print(client, words),
                    "Shows the current state of the match, using the specified output, e.g. 'show' or 'print pgn'",
                ),
                sel_descr(
                    vec!["info"],
                    |client, words| Self::handle_info(client, words),
                    "Print general information about the given player, e.g. 'info' or 'info black'",
                ),
                sel_descr(
                    vec!["set_player"],
                    |client, words| Self::handle_set_player(client, words),
                    "Set a player, e.g. 'set_player white human'.",
                ),
                sel_descr(
                    vec!["load_player"],
                    |_, _| panic!("This should've been handled manually'"),
                    "Load a new player, which will then be available to play, such as by using 'set_player'",
                ),
                sel_descr(
                    vec!["position"],
                    |client, words| Self::handle_position(client, words),
                    "Set the current position, e.g. 'position fen <fen>'",
                ),
                sel_descr(
                    vec!["tc", "time"],
                    |client, words| Self::handle_tc(client, words),
                    "Set the time control of a player, given in seconds, e.g. 'tc white 300+3' or 'tc black 8+0.08'",
                ),
                sel_descr(
                    vec!["ugi", "uci", "send_ugi", "send_uci"],
                    |client, words| Self::handle_send_ugi(client, words),
                    "Manually send a UGI command to an engine, e.g 'ugi white go depth 3'. Note that this can very easily crash the engine and is only intended as a developer tool.",
                ),
                sel_descr(vec![""], |_, _| Ok(()), "Empty commands are ignored"),
            ],
        };
        loop {
            let mut input = String::new();
            // At this point, we don't hold an Arc, so when the main thread terminates, we're not preventing dropping
            // the client.
            if let Err(err) = stdin().read_line(&mut input) {
                if let Some(ugi_client) = ugi_client.upgrade() {
                    // Else, the match has ended, so don't bother printing an error message.
                    ugi_client.lock().unwrap().show_error(&format_args!("Couldn't get input: {err}"));
                };
                break;
            }
            let input = input.as_str().trim();
            let Some(client) = ugi_client.upgrade() else {
                // The program has been terminated
                break;
            };
            match input_thread.handle_input(client, tokens(input), input) {
                Ok(continue_running) => {
                    if !continue_running {
                        break;
                    }
                }
                Err(e) => {
                    ugi_client.upgrade().inspect(|client| {
                        client.lock().unwrap().show_message(Warning, &format_args!("Ignoring input. {e}"));
                    });
                }
            }
        }
    }

    fn handle_input(&self, ugi_client: Arc<Mutex<Client<B>>>, mut words: Tokens, input: &str) -> Res<bool> {
        let command = words.next().unwrap_or_default();
        if command.eq_ignore_ascii_case("help") {
            // Can't be a part of the `commands` vec because `print_help` needs a reference to the commands vec
            Self::print_help(&self.commands, &mut words)?;
        } else if command.eq_ignore_ascii_case("load_player") {
            // Shouldn't be a part of the `commands` vec because it has a different signature (takes an `Arc<Mutex<Client>>`
            // instead of a `MutexGuard<Client>`).
            Self::handle_load_player(ugi_client.clone(), &mut words)?;
        } else {
            let mut client = ugi_client.lock().unwrap();
            match B::Move::from_text(input, client.board()) {
                Ok(mov) => {
                    let Some(active_player) = client.active_player() else {
                        bail!("Ignoring move because the game is over".to_string())
                    };
                    client.play_move(mov).map_err(|err| anyhow!("Ignoring input: {err}"))?;
                    // `play_move` will have stopped the clock by now.
                    assert!(client.state.get_player(active_player).thinking_since().is_none());
                    return Ok(true);
                }
                Err(err) => {
                    let func = select_name_static(command, self.commands.iter(), "command", &B::game_name(), NoDescription)
                        .map_err(|msg| anyhow!("'{command}' is not a legal move: {err}.\nIt's also not a command: {msg}\nType 'help' for more information."))?
                        .func;
                    func(client, &mut words)?;
                }
            }
        }
        if let Some(w) = words.next() {
            ugi_client.lock().unwrap().show_message(
                Warning,
                &format_args!("Ignoring extra input starting with '{w}' after the '{command}' command"),
            );
        }
        if ugi_client.lock().unwrap().will_quit() {
            // The program has just decided to terminate, but to do so it needs to join all threads.
            // Return so that that can happen.
            return Ok(false);
        }
        Ok(true)
    }

    fn print_help(commands: &[Command<B>], words: &mut Tokens) -> Res<()> {
        if let Some(name) = words.next() {
            let desc = select_name_static(name, commands.iter(), "command", &B::game_name(), NoDescription)?
                .description
                .unwrap_or("No description available");
            println!("{desc}");
        } else {
            println!(
                "Input either a move (most formats based on algebraic notation are recognized) or a command. Valid commands are:"
            );
            for cmd in commands {
                println!(
                    "{:25}  {description}",
                    cmd.names.iter().map(|c| format!("'{}'", c.bold())).join(", ") + ":",
                    description = cmd.description.unwrap_or("<No description>")
                );
            }
        }
        Ok(())
    }

    fn get_side(client: &MutexGuard<Client<B>>, words: &mut Tokens, default_player: DefaultPlayer) -> Res<B::Color> {
        match words.next().unwrap_or_default().to_ascii_lowercase().as_str() {
            "white" | "p1" => Ok(B::Color::first()),
            "black" | "p2" => Ok(B::Color::second()),
            x => {
                let player = if x == "current" || x == "active" {
                    Active
                } else if x == "other" || x == "inactive" {
                    Inactive
                } else {
                    default_player
                };
                if player == Active {
                    Ok(client.active_player().ok_or_else(|| {
                        anyhow!("No color given and there is no active player (the match isn't running)")
                    })?)
                } else if player == Inactive {
                    Ok(client
                        .active_player()
                        .ok_or_else(|| {
                            anyhow!("No color given and there is no inactive player (the match isn't running)")
                        })?
                        .other())
                } else {
                    bail!("Missing the side. Valid values are 'white', 'p1', 'black', 'p2', 'active' and 'inactive'")
                }
            }
        }
    }

    fn list_moves(mut client: MutexGuard<Client<B>>) {
        let board = &client.match_state().board;
        println!("{}", board.legal_moves_slow().into_iter().map(|m| m.to_extended_text(board, Alternative)).join(", "));
    }

    fn random_move(mut client: MutexGuard<Client<B>>) -> Res<()> {
        let board = client.state.the_match.board.clone();
        let mut rng = rng();
        let over = matches!(client.match_state().status, Over(_));
        let Some(mov) = board.random_legal_move(&mut rng) else {
            bail!(
                "There are no legal moves in the current position ({}){reason}",
                board.as_fen(),
                reason = if over { ". The game is over" } else { "" }
            )
        };
        client.play_move(mov)
    }

    fn handle_stop(mut client: MutexGuard<Client<B>>, words: &mut Tokens) -> Res<()> {
        let side = Self::get_side(&client, words, Active)?;
        if !client.state.get_player(side).is_engine() {
            bail!(
                "The {} player is a human and not an engine, so they can't be stopped",
                side.name(&client.match_state().board.settings()).as_ref()
            )
        }
        match client.active_player() {
            None => bail!("The match isn't running"),
            Some(p) => {
                if p == side {
                    client.stop_thinking(side, Play);
                    Ok(())
                } else {
                    bail!(
                        "The {} player is not currently thinking",
                        p.name(&client.match_state().board.settings()).as_ref()
                    )
                }
            }
        }
    }

    fn handle_position(mut client: MutexGuard<Client<B>>, words: &mut Tokens) -> Res<()> {
        let old_board = client.board().clone();
        // TODO: Use parse_ugi_position
        client.reset_to_new_start_position(parse_ugi_position_part("position", words, false, &old_board, Relaxed)?);
        let Some(word) = words.next() else {
            return Ok(());
        };
        if word != "moves" {
            bail!("Unrecognized word '{word}' after position command, expected either 'moves' or nothing")
        }
        for mov in words {
            let mov =
                B::Move::from_compact_text(mov, client.board()).map_err(|err| anyhow!("Couldn't parse move: {err}"))?;
            client.play_move_internal(mov)?;
        }
        Ok(())
    }

    fn handle_undo(mut client: MutexGuard<Client<B>>, words: &mut Tokens) -> Res<()> {
        let num = parse_int_from_str(words.next().unwrap_or("1"), "num moves")?;
        client.undo_halfmoves(num)
    }

    fn handle_set_player(mut client: MutexGuard<Client<B>>, words: &mut Tokens) -> Res<()> {
        let side = Self::get_side(&client, words, NoPlayer)?;
        let Some(name) = words.next() else {
            bail!(
                "Missing the name of the new player (e.g. 'human'). Loaded players are {}",
                client.state.players.iter().map(Player::get_name).join(", ")
            )
        };
        let (p1, p2) = (client.state.id(B::Color::first()), client.state.id(B::Color::second()));
        let mut found = false;
        for (idx, player) in client.state.players.iter().enumerate() {
            if player.get_name().to_lowercase() == name.to_lowercase() {
                if idx != p1 && idx != p2 {
                    // TODO: Remove this restriction, simply build a new one
                    client.set_player(side, idx);
                    client.show();
                    return Ok(());
                }
                found = true;
            }
        }
        if name.eq_ignore_ascii_case("human") {
            let builder = PlayerBuilder::new(PlayerArgs::Human(HumanArgs::default()));
            let id = builder.build_human(&mut client, None)?;
            client.set_player(side, id);
            client.show();
            return Ok(());
        }
        if found {
            bail!("The player '{name}' is already playing in this match")
        } else {
            bail!(
                "No player with the given name '{name}' found. Maybe you need to load this player first (type 'load_player <options>')"
            )
        }
    }

    fn handle_tc(mut client: MutexGuard<Client<B>>, words: &mut Tokens) -> Res<()> {
        let color = Self::get_side(&client, words, Active)?;
        let tc = TimeControl::from_str(words.next().unwrap_or_default())?;
        client.state.get_player_mut(color).set_time(tc)?;
        client.show();
        Ok(())
    }

    fn handle_load_player(ugi_client: Arc<Mutex<Client<B>>>, words: &mut Tokens) -> Res<()> {
        let mut words = words.map(ToString::to_string).peekable();
        let args = if words.peek().is_some_and(|w| w.eq_ignore_ascii_case("human")) {
            words.next();
            Human(parse_human(&mut words)?)
        } else {
            Engine(parse_engine(&mut words)?)
        };
        let builder = PlayerBuilder::new(args);
        _ = builder.build(ugi_client)?;
        Ok(())
    }

    fn handle_ui(mut client: MutexGuard<Client<B>>, words: &mut Tokens) -> Res<()> {
        match words.next() {
            None => {
                let infos = client
                    .outputs
                    .iter()
                    .map(|o| to_name_and_optional_description(o.as_ref(), WithDescription))
                    .join(",");
                client.show_message(Info, &format_args!("{}", infos));
            }
            Some(mut name) => {
                let mut replace = true;
                if name == "add" {
                    name = words.next().ok_or_else(|| anyhow!("Expected an output name after 'add'"))?;
                    replace = false;
                } else if name == "remove" {
                    name = words.next().ok_or_else(|| anyhow!("Expected an output name after 'remove'"))?;
                    match client.outputs.iter().position(|o| o.matches(name)) {
                        None => {
                            bail!("There is no output with name '{name}' currently in use")
                        }
                        Some(idx) => {
                            client.outputs.remove(idx);
                        }
                    }
                    return Ok(());
                }
                let output = output_builder_from_str(name, &client.all_outputs)?.for_client(&client.state)?;
                if replace && !client.outputs.is_empty() {
                    client.outputs[0] = output;
                } else {
                    client.outputs.push(output);
                }
            }
        }
        client.show();
        Ok(())
    }

    fn handle_print(mut client: MutexGuard<Client<B>>, words: &mut Tokens) -> Res<()> {
        match words.next().unwrap_or_default() {
            "" => client.show(),
            x => {
                output_builder_from_str(x, &client.all_outputs)?
                    .for_client(&client.state)?
                    .show(&client.state, OutputOpts::default());
            }
        }
        Ok(())
    }

    fn handle_info(mut client: MutexGuard<Client<B>>, words: &mut Tokens) -> Res<()> {
        let color = Self::get_side(&client, words, Active)?;
        match client.state.get_player_mut(color) {
            Player::Engine(engine) => {
                println!(
                    "Name: {name} ({ugi_name}), author: {author}",
                    name = engine.display_name,
                    ugi_name = engine.ugi_name,
                    author = engine.author
                );
                println!("Options:\n{}", engine.options.iter().map(EngineOption::to_string).join("\n"));
            }
            Player::Human(human) => {
                println!("{} (human player)", human.name);
            }
        }
        if client.match_state().status == Ongoing {
            println!(
                "Remaining time: {}",
                client
                    .state
                    .get_player(color)
                    .get_time()
                    .unwrap()
                    .remaining_to_string(client.state.thinking_since(color))
            );
        }
        Ok(())
    }

    fn handle_send_ugi(mut client: MutexGuard<Client<B>>, words: &mut Tokens) -> Res<()> {
        let player = Self::get_side(&client, words, Active)?;
        if !client.state.get_player_mut(player).is_engine() {
            bail!(
                "The {} player is not an engine and can't receive UGI commands",
                player.name(&client.match_state().board.settings()).as_ref()
            )
        }
        client.send_ugi_message(player, words.join(" ").as_str().trim());
        Ok(())
    }
}

#[derive(Debug, Default)]
pub(super) struct TextInput {
    handle: Option<JoinHandle<()>>,
}

impl StaticallyNamedEntity for TextInput {
    fn static_short_name() -> impl Display {
        "text"
    }

    fn static_long_name() -> String {
        "Text-based input".to_string()
    }

    fn static_description() -> String {
        "Use the console to change the match state".to_string()
    }
}

impl<B: Board> Input<B> for TextInput {
    fn assume_control(&mut self, ugi_client: Arc<Mutex<Client<B>>>) {
        self.handle = Some(
            Builder::new()
                .name("Text input thread".to_string())
                .spawn(move || TextInputThread::input_loop(Arc::downgrade(&ugi_client)))
                .unwrap(),
        );
    }

    fn join_threads(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.join().expect("The input thread panicked");
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct TextInputBuilder {}

impl NamedEntity for TextInputBuilder {
    fn short_name(&self) -> String {
        TextInput::static_short_name().to_string()
    }

    fn long_name(&self) -> String {
        TextInput::static_long_name().to_string()
    }

    fn description(&self) -> Option<String> {
        Some(TextInput::static_description())
    }
}

impl<B: Board> InputBuilder<B> for TextInputBuilder {
    fn build(&self) -> Box<dyn Input<B>> {
        Box::new(TextInput::default())
    }
}
