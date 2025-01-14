use std::fmt::Debug;
use std::mem::swap;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossbeam_utils::sync::{Parker, Unparker};

use crate::cli::CommandLineArgs;
use crate::play::adjudication::{Adjudication, Adjudicator};
use crate::play::player::Player::{Engine, Human};
use crate::play::player::{
    limit_to_ugi, EnginePlayer, HumanPlayerStatus, Player, PlayerBuilder, Protocol,
};
use crate::play::ugi_input::BestMoveAction;
use crate::play::ugi_input::BestMoveAction::Ignore;
use crate::play::ugi_input::EngineStatus::*;
use crate::ui::Input;
use gears::colored::Colorize;
use gears::games::{BoardHistory, Color, ZobristHistory};
use gears::general::board::Strictness::Relaxed;
use gears::general::board::{Board, BoardHelpers};
use gears::general::common::anyhow::bail;
use gears::general::common::Res;
use gears::general::moves::Move;
use gears::output::Message::*;
use gears::output::{Message, OutputBox, OutputBuilder, OutputOpts};
use gears::search::{SearchInfo, TimeControl};
use gears::MatchStatus::*;
use gears::Quitting::*;
use gears::{
    output_builder_from_str, player_res_to_match_res, AbstractRun, AdjudicationReason, GameOver,
    GameOverReason, GameResult, GameState, MatchResult, MatchStatus, PlayerResult, Quitting,
};
// TODO: Use tokio? Probably more efficient and it has non-blocking reads.

pub type PlayerId = usize;

const NO_PLAYER: PlayerId = PlayerId::MAX;

// A struct that gets manipulated by the UI and the players
// TODO: Use 'Match' struct
#[derive(Debug, Clone)]
pub struct UgiMatchState<B: Board> {
    pub status: MatchStatus,
    /// Current board state (does not include history), should be cheap to copy
    pub board: B,
    /// Needed for repetition detection
    pub board_history: ZobristHistory<B>,
    /// Needed to reconstruct the match, such as for the PGN export.
    pub move_history: Vec<B::Move>,
    /// useful for gui matches to allow a "restart" option
    // TODO: In the future, maybe it makes sense to allow having an initial history?
    // This would allow correct handling of 3 fold repetition and resetting to initial position.
    pub initial_pos: B,
    /// The "event" field in the PGN
    pub event: String,
    /// The "site" field in the PGH
    pub site: String,
    pub p1: PlayerId,
    pub p2: PlayerId,
}

impl<B: Board> UgiMatchState<B> {
    fn new(initial_pos: B, event: String, site: String) -> Self {
        Self {
            status: NotStarted,
            board: initial_pos.clone(),
            board_history: ZobristHistory::default(),
            move_history: vec![],
            initial_pos,
            event,
            site,
            p1: NO_PLAYER,
            p2: NO_PLAYER,
        }
    }

    fn reset(&mut self) {
        self.board = self.initial_pos.clone();
        self.move_history.clear();
        self.board_history.clear();
        self.board_history.push(&self.board);
        self.status = NotStarted;
    }

    fn player_mut(&mut self, color: B::Color) -> &mut PlayerId {
        if color.is_first() {
            &mut self.p1
        } else {
            &mut self.p2
        }
    }
}

/// This struct exists (instead of simply using `Client`) because the all-mighty borrow checker demands it.
#[derive(Debug)]
pub struct ClientState<B: Board> {
    pub the_match: UgiMatchState<B>,
    game_name: String,
    pub players: Vec<Player<B>>,
    /// The ith entry is the builder used to construct the ith player.
    pub player_builders: Vec<PlayerBuilder>,
    /// Wait for this duration after a match, which is useful for the GUI to actually show the final position
    pub wait_after_match: Duration,
    /// Restart crashed engines (this still counts as a loss) instead of exiting the program
    pub recover: bool,
    /// In debug mode, everything gets logged.
    pub debug: bool,
}

impl<B: Board> ClientState<B> {
    // Getters should be implemented as methods of the UgiMatchState, but all other functions, especially those that modify
    // the state, are better suited in the Client, because only the client can access outputs.
    pub fn id(&self, color: B::Color) -> PlayerId {
        if color.is_first() {
            self.the_match.p1
        } else {
            self.the_match.p2
        }
    }

    pub fn get_player_from_id_mut(&mut self, id: PlayerId) -> &mut Player<B> {
        &mut self.players[id]
    }

    pub fn get_player_from_id(&self, id: PlayerId) -> &Player<B> {
        &self.players[id]
    }

    pub fn get_player(&self, color: B::Color) -> &Player<B> {
        self.get_player_from_id(self.id(color))
    }

    pub fn get_player_mut(&mut self, color: B::Color) -> &mut Player<B> {
        self.get_player_from_id_mut(self.id(color))
    }

    pub(super) fn get_engine_from_id_mut(&mut self, id: PlayerId) -> &mut EnginePlayer<B> {
        match self.get_player_from_id_mut(id) {
            Engine(e) => e,
            Human(_) => {
                panic!("Internal error: Expected player {id} to be an engine, but it was a human")
            }
        }
    }

    pub fn get_engine_from_id(&self, id: PlayerId) -> &EnginePlayer<B> {
        match self.get_player_from_id(id) {
            Engine(e) => e,
            Human(_) => {
                panic!("Internal error: Expected player {id} to be an engine, but it was a human")
            }
        }
    }

    pub fn get_engine(&self, color: B::Color) -> &EnginePlayer<B> {
        self.get_engine_from_id(self.id(color))
    }

    pub(super) fn get_engine_mut(&mut self, color: B::Color) -> &mut EnginePlayer<B> {
        self.get_engine_from_id_mut(self.id(color))
    }

    pub fn contains_human(&self) -> bool {
        !(self.get_player(B::Color::first()).is_engine()
            && self.get_player(B::Color::second()).is_engine())
    }

    pub fn num_players(&self) -> usize {
        self.players.len()
    }
}

impl<B: Board> GameState<B> for ClientState<B> {
    fn initial_pos(&self) -> &B {
        &self.the_match.initial_pos
    }

    fn get_board(&self) -> &B {
        &self.the_match.board
    }

    fn game_name(&self) -> &str {
        &self.game_name
    }

    fn move_history(&self) -> &[B::Move] {
        self.the_match.move_history.as_slice()
    }

    fn match_status(&self) -> MatchStatus {
        self.the_match.status.clone()
    }

    fn name(&self) -> &str {
        "GUI"
    }

    fn event(&self) -> String {
        self.the_match.event.clone()
    }

    fn site(&self) -> &str {
        &self.the_match.site
    }

    fn player_name(&self, color: B::Color) -> Option<String> {
        Some(self.get_player(color).get_name().to_string())
    }

    fn time(&self, color: B::Color) -> Option<TimeControl> {
        self.get_player(color).get_time()
    }

    fn thinking_since(&self, color: B::Color) -> Option<Instant> {
        self.get_player(color).thinking_since()
    }

    fn engine_state(&self) -> Res<String> {
        Ok("Getting the internal engine state is not supported in the match manager".to_string())
    }
}

/// The word `Client` is used instead of the more common term `Ugi GUI` to avoid confusion with regard to the actual GUI,
/// and in keeping with the (unofficial, but vastly more detailed) UCI specification at <https://expositor.dev/uci/doc/uci-draft-1.pdf>
#[derive(Debug)]
pub struct Client<B: Board> {
    /// The state sub-object is necessary to appease the borrow checker, because output functions take
    /// both themselves as mut reference and the state as non-mut reference
    pub state: ClientState<B>,
    pub outputs: Vec<OutputBox<B>>,
    pub all_outputs: Vec<Box<dyn OutputBuilder<B>>>,
    /// Match-specific draw / resign adjudication on top of the game rules
    /// (i.e. 50mr and insufficient material are *not* handled by this, but by the `board`)
    pub adjudicator: Adjudicator,
    ugi_output: OutputBox<B>,
    // quit the entire program (not just a single match)
    send_quit: Unparker,
    will_quit: bool,
}

impl<B: Board> Client<B> {
    fn create(
        send_quit: Unparker,
        all_outputs: Vec<Box<dyn OutputBuilder<B>>>,
        args: &CommandLineArgs,
    ) -> Res<Arc<Mutex<Self>>> {
        let initial_pos = match &args.start_pos {
            None => B::default(),
            Some(fen) => B::from_fen(fen, Relaxed)?,
        };
        let event = args
            .event
            .clone()
            .unwrap_or_else(|| format!("Monitors - {} match", B::game_name()));
        let site = args
            .site
            .clone()
            .unwrap_or_else(|| "github.com/toanth/motors".to_string());

        let adjudicator = Adjudicator::new(
            args.resign_adjudication,
            args.draw_adjudication,
            args.max_moves.unwrap_or(NonZeroUsize::MAX).get(),
        );

        let match_state = UgiMatchState::new(initial_pos, event, site);
        let state = ClientState {
            the_match: match_state,
            game_name: B::game_name(),
            players: vec![],
            player_builders: vec![],
            wait_after_match: args.wait_after_match,
            recover: args.recover,
            debug: false,
        };
        let ugi_output = output_builder_from_str("ugi", &all_outputs)
            .expect("Couldn't create 'ugi' output")
            .for_client(&state)?;
        Ok(Arc::new(Mutex::new(Self {
            state,
            outputs: vec![],
            all_outputs,
            adjudicator,
            ugi_output,
            send_quit,
            will_quit: false,
        })))
    }

    pub fn match_state(&mut self) -> &mut UgiMatchState<B> {
        &mut self.state.the_match
    }

    pub fn board(&mut self) -> &mut B {
        &mut self.match_state().board
    }

    pub fn add_output(&mut self, mut output: Box<dyn OutputBuilder<B>>) -> Res<()> {
        let output = output.for_client(&self.state)?;
        self.outputs.push(output);
        Ok(())
    }

    pub fn add_player(&mut self, player: Player<B>, builder: PlayerBuilder) -> PlayerId {
        self.state.players.push(player);
        self.state.player_builders.push(builder);
        self.state.num_players() - 1
    }

    pub fn get_color(&self, player: PlayerId) -> Option<B::Color> {
        if self.state.id(B::Color::first()) == player {
            Some(B::Color::first())
        } else if self.state.id(B::Color::second()) == player {
            Some(B::Color::second())
        } else {
            None
        }
    }

    /// Gets called upon an engine crash (in which case the engine also loses the current game, if there is any).
    pub fn hard_reset_player(this: Arc<Mutex<Self>>, id: PlayerId) -> Res<()> {
        let mut client = this.lock().unwrap();
        // The player isn't participating in an ongoing match (other players might), but it could have been
        // before the crash
        let color = client.get_color(id);
        let player = client.state.get_player_from_id_mut(id);
        player.reset();
        let builder = this.lock().unwrap().state.player_builders[id].clone();
        builder.replace(this.clone(), Some(id))?;
        let player = client.state.get_player_from_id_mut(id);
        if let Some(color) = color {
            // reset the state to before the crash
            player.assign_to_match(color);
        }
        Ok(())
    }

    /// Resets the current match to the state before any moves were played.
    pub fn reset(&mut self) {
        // A newly constructed UgiMatchState does not contain any players
        if self.match_state().p1 != NO_PLAYER {
            for color in B::Color::iter() {
                self.cancel_thinking(color);
                self.state.get_player_mut(color).reset();
            }
        }
        self.match_state().reset();
    }

    pub fn abort_match(&mut self) {
        self.cancel_thinking(B::Color::first());
        self.cancel_thinking(B::Color::second());
        self.game_over(MatchResult {
            result: GameResult::Aborted,
            reason: GameOverReason::Adjudication(AdjudicationReason::AbortedByUser),
        });
    }

    /// This does not only cancel the match (like `abort_match`, `lose_on_time` or `game_over`), but also exits the client completely.
    pub fn quit_program(&mut self) {
        if self.match_state().status == Ongoing {
            self.abort_match();
        }
        self.will_quit = true;
        self.send_quit.unpark();
    }

    pub fn will_quit(&self) -> bool {
        self.will_quit
    }

    pub fn lose_on_time(&mut self, color: B::Color) {
        let time = self.state.get_player_mut(color).get_original_tc();
        self.show_message(
            Warning,
            &format!(
                "The {} player ran out of time (the time control was {start}ms + {inc}ms)",
                color.name(&self.state.the_match.board.settings()),
                start = time.remaining.as_millis(),
                inc = time.increment.as_millis()
            ),
        );
        if self.match_state().status == Ongoing {
            // Draw by adjudication when the opponent has insufficient mating material, only applied to
            // games with a human player
            let result = if (!self.state.get_player(B::Color::first()).is_engine()
                || !self.state.get_player(B::Color::second()).is_engine())
                && !self.board().can_reasonably_win(color.other())
            {
                PlayerResult::Draw
            } else {
                PlayerResult::Lose
            };
            let res = GameOver {
                result,
                reason: GameOverReason::Adjudication(AdjudicationReason::TimeUp),
            };
            self.game_over(player_res_to_match_res(res, color));
        }
    }

    pub fn game_over(&mut self, result: MatchResult) {
        self.match_state().status = Over(result);
        for output in &mut self.outputs {
            output.inform_game_over(&self.state);
        }
    }

    pub fn update_info(&mut self, id: PlayerId, info: SearchInfo<B>) -> Res<()> {
        let engine = self.state.get_engine_from_id_mut(id);
        for output in &mut self.outputs {
            output.update_engine_info(&engine.display_name, &info);
        }
        let Some(current_match) = engine.current_match.as_mut() else {
            bail!("The engine sent info ('{info}') while it wasn't playing in match")
        };
        current_match.search_info = Some(info);
        Ok(())
    }

    pub fn show_ugi_info_string(&mut self, id: PlayerId, info: &str) {
        self.show_message(
            Info,
            &format!(
                "Engine {}: '{info}'",
                self.state.get_engine_from_id(id).display_name
            ),
        );
    }

    /// Plays a move without assuming that it comes from the current player, so this can be used to set up a position.
    /// Therefore, it does not show the current board after each move, does not test for the end of the match,
    /// and does not transfer control to the other player.
    pub fn play_move_internal(&mut self, mov: B::Move) -> Res<()> {
        if !self.board().is_move_pseudolegal(mov) {
            // can't use to_extended_text because that assumes pseudolegality internally
            bail!(
                "The move '{}' is not pseudolegal in the current position",
                mov.compact_formatter(self.board()).to_string().red()
            )
        }
        let Some(board) = self.board().clone().make_move(mov) else {
            let player_res = GameOver {
                result: PlayerResult::Lose,
                reason: GameOverReason::Adjudication(AdjudicationReason::InvalidMove),
            };
            self.game_over(player_res_to_match_res(
                player_res,
                self.active_player().unwrap(),
            ));
            let pos = self.board();
            bail!(
                "Invalid move '{0}' in position {pos}",
                mov.compact_formatter(pos).to_string().red()
            )
        };

        *self.board() = board.clone();
        self.match_state().board_history.push(&board);
        self.match_state().move_history.push(mov);
        Ok(())
    }

    pub fn play_move(&mut self, mov: B::Move) -> Res<()> {
        let color = self.board().active_player();
        self.play_move_internal(mov)?;
        // Make sure to stop the engine if the user entered a legal move while it was thinking
        // This will send a `stop` command if an engine was thinking about this move,
        // which won't be handled until the mutex is released, at which point it will just be ignored.
        self.stop_thinking(color, Ignore);
        self.show();
        match self.compute_match_result() {
            // if a human player makes the first move, this starts the game (and the clock)
            None => {
                self.match_state().status = Ongoing;
                self.start_thinking(color.other());
            }
            Some(result) => self.game_over(result),
        };
        Ok(())
    }

    fn compute_match_result(&mut self) -> Option<MatchResult> {
        let state = self.match_state();
        if let Some(res) = state.board.match_result_slow(&state.board_history) {
            return Some(res);
        }
        self.adjudicator.adjudicate(&self.state)
    }

    pub fn show(&mut self) {
        for output in &mut self.outputs {
            output.show(&self.state, OutputOpts::default());
        }
    }

    pub fn show_error(&mut self, message: &str) {
        self.show_message(Error, message);
    }

    pub fn show_message(&mut self, typ: Message, message: &str) {
        for output in &mut self.outputs {
            output.display_message_with_state(&self.state, typ, message);
        }
    }

    /// The player doesn't necessarily have to be playing a match right now.
    /// This can happen when initializing more than 2 players in tournament mode.
    pub fn send_ugi_message_to(&mut self, engine: PlayerId, message: &str) {
        let engine = self.state.get_engine_from_id_mut(engine);
        let name = engine.display_name.clone();
        for output in &mut self.outputs {
            output.write_ugi_output(&format_args!("{message}"), Some(&name));
        }

        if let Err(err) = engine.write_ugi_impl(message) {
            self.show_error(&format!(
                "Couldn't send message '{message}' to engine '{name}': {err}"
            ));
        }
    }

    pub fn send_ugi_message(&mut self, color: B::Color, message: &str) {
        self.send_ugi_message_to(self.state.id(color), message);
    }

    fn send_uginewgame(&mut self, color: B::Color) {
        let msg = match self.state.get_engine(color).proto {
            Protocol::Uci => "ucinewgame",
            Protocol::Ugi => "uginewgame",
        };
        self.send_ugi_message(color, msg);
    }

    // TODO: Not used
    // correctly using isready is difficult with the current design, so this will have to wait until the refactoring
    #[allow(unused)]
    fn send_isready(&mut self, color: B::Color) {
        self.send_ugi_message(color, "isready");
        let engine = self.state.get_engine_mut(color);
        match engine.status {
            ThinkingSince(start) => engine.status = Ping(start),
            _ => engine.status = Sync,
        }
    }

    fn send_position(&mut self, color: B::Color) {
        self.send_ugi_message(
            color,
            &self
                .ugi_output
                .as_string(&self.state, OutputOpts::default()),
        );
    }

    /// This function does no validation at all. This allows for greater flexibility when the user knows that
    /// an engine supports options or even option types that the client isn't aware of.
    /// However, this does mean that invalid user input can lead to engine crashes (but that's already the case
    /// anyway, and not something the client can handle in general)
    pub fn send_setoption(&mut self, engine: PlayerId, name: &str, value: &str) {
        let msg = format!("setoption name {name} value {value}");
        self.send_ugi_message_to(engine, &msg);
    }

    fn send_go(&mut self, color: B::Color) {
        let p1_time = self.state.get_player(B::Color::first()).get_time().unwrap();
        let p2_time = self
            .state
            .get_player(B::Color::second())
            .get_time()
            .unwrap();
        let engine = self.state.get_engine_mut(color);
        let msg = limit_to_ugi(engine.current_match().limit, p1_time, p2_time).unwrap();
        self.send_ugi_message(color, &msg);
    }

    pub fn start_thinking(&mut self, color: B::Color) {
        if self.state.get_player_mut(color).is_engine() {
            assert!(matches!(self.state.get_engine_mut(color).status, Idle)); // TODO: Could this also be Halt?
            debug_assert!(self.state.get_engine(color).current_match.is_some());
            self.send_position(color);
            self.send_go(color);
        }
        self.state.get_player_mut(color).start_clock();
    }

    pub fn stop_clock(&mut self, color: B::Color) {
        let player = self.state.get_player_mut(color);
        if player.update_clock_and_check_for_time_loss() {
            self.lose_on_time(color);
        }
    }

    /// Sends a UGI 'stop' message and ignores the resulting 'bestmove' message.
    pub fn cancel_thinking(&mut self, color: B::Color) {
        self.stop_thinking(color, Ignore);
    }

    /// Sends a UGI 'stop' message and either ignores the resulting 'bestmove' message or plays it,
    /// in the case of an engine (for a human player, assert that `handle_best_move` is `Ignore`)
    pub fn stop_thinking(&mut self, color: B::Color, bestmove_action: BestMoveAction) {
        match self.state.get_player_mut(color) {
            Engine(engine) => {
                if matches!(engine.status, ThinkingSince(_)) {
                    if matches!(bestmove_action, Ignore) {
                        // Since we're ignoring the bestmove message anyway, we should stop the clock now.
                        // This case can happen when the user inputs a move for a currently thinking engine.
                        self.stop_clock(color);
                    }
                    self.send_ugi_message(color, "stop");
                    let engine = self.state.get_engine_mut(color); // the borrow checker demands another sacrifice
                    engine.halt(bestmove_action);
                }
            }
            Human(human) => {
                assert!(matches!(bestmove_action, Ignore));
                human.status = HumanPlayerStatus::Idle;
            }
        }
    }

    // TODO: Test that all of the commands that interfere with a player also work when used while an engine is thinking,
    // ideally also while the GUI is waiting for readyok. Testcases (with mocking) would be really helpful.

    pub fn undo_halfmoves(&mut self, num_plies_to_undo: usize) -> Res<()> {
        let plies_played = self.match_state().move_history.len();
        if plies_played < num_plies_to_undo {
            bail!("Couldn't undo the last {num_plies_to_undo} half moves because only {plies_played} half moves \
                have been played so far (this is the initial position: '{}')", self.board().as_fen())
        }
        let prev_ply = plies_played - num_plies_to_undo;
        self.rewind_to_ply(prev_ply)
    }

    pub fn rewind_to_ply(&mut self, ply: usize) -> Res<()> {
        assert!(ply <= self.match_state().move_history.len());
        debug_assert!(
            self.match_state().board_history.len() == self.match_state().move_history.len() + 1
        );
        let initial_pos = self.match_state().initial_pos.clone();
        let mut moves = self.match_state().move_history.clone();
        moves.truncate(ply);
        self.change_position_to(initial_pos, &moves)?;
        self.start_match();
        Ok(())
    }

    /// This function changes the current position without resetting the player's timers.
    /// It's used to implement `undoMove`. After this function, the game is in the `NotStarted` state.
    pub fn change_position_to(&mut self, initial_pos: B, moves: &[B::Move]) -> Res<()> {
        let player = self.board().active_player();
        let current_match_state = self.state.the_match.clone();
        self.cancel_thinking(player); // this also works if there is no current player
        self.match_state().initial_pos = initial_pos;
        self.match_state().reset();
        let mut change_pos = || -> Res<()> {
            for mov in moves {
                self.play_move_internal(*mov)?;
            }
            Ok(())
        };
        if let Err(err) = change_pos() {
            self.state.the_match = current_match_state;
            return Err(err);
        }
        self.match_state().status = NotStarted;
        Ok(())
    }

    /// Makes the players change sides without stopping the current game.
    /// Unlike manually calling `set_player` multiple times, this keeps the time controls of the players constant
    /// instead of keeping the time controls of the sides constant. (So the white side will have the same time control
    /// as the black side used to have, because they are the same player)
    pub fn flip_players(&mut self) {
        let active = self.active_player();
        if let Some(color) = active {
            self.cancel_thinking(color);
        }
        swap(&mut self.state.the_match.p1, &mut self.state.the_match.p2);
        for color in B::Color::iter() {
            if let Engine(engine) = self.state.get_player_mut(color) {
                if let Some(m) = engine.current_match.as_mut() {
                    m.color = color;
                }
            }
        }
        if let Some(color) = active {
            self.start_thinking(color);
        }
    }

    pub fn set_player(&mut self, color: B::Color, player: PlayerId) {
        let active_player = self.active_player();
        let old_player = self.state.get_player(color);
        let limit = old_player.get_limit();
        let original = old_player.get_original_tc();
        self.cancel_thinking(color);
        self.state.get_player_mut(color).reset();
        *self.match_state().player_mut(color) = player;
        let new_player = self.state.get_player_from_id_mut(player);
        new_player.assign_to_match(color);
        if let Some(limit) = limit {
            new_player.transfer_time(limit, original);
        }
        if active_player == Some(color) {
            self.start_thinking(color);
        }
    }

    /// This function resets the player's timers. The board and the internal initial position
    pub fn reset_to_new_start_position(&mut self, initial_pos: B) {
        self.match_state().initial_pos = initial_pos;
        self.reset();
    }

    /// Start a new match from the original starting position, with the given players.
    pub fn new_match(&mut self, p1: PlayerId, p2: PlayerId) {
        assert!(p1 < self.state.num_players());
        assert!(p2 < self.state.num_players());
        self.reset();
        self.match_state().p1 = p1;
        self.match_state().p2 = p2;
        for color in B::Color::iter() {
            self.state.get_player_mut(color).assign_to_match(color);
        }
        self.start_match();
    }

    pub fn restart(&mut self) {
        self.new_match(self.state.the_match.p1, self.state.the_match.p2);
    }

    pub fn restart_flipped_colors(&mut self) {
        self.new_match(self.state.the_match.p2, self.state.the_match.p1);
    }

    /// Start a match from any starting position with the current players.
    pub fn start_match(&mut self) {
        assert_eq!(self.match_state().status, NotStarted);
        for color in B::Color::iter() {
            if self.state.get_player(color).is_engine() {
                self.send_uginewgame(color);
            }
        }
        self.show();
        self.match_state().status = Ongoing;
        let player = self.board().active_player();
        self.start_thinking(player);
    }

    pub fn active_player(&self) -> Option<B::Color> {
        if self.state.the_match.status == Ongoing {
            Some(self.state.the_match.board.active_player())
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct RunClient<B: Board> {
    pub client: Arc<Mutex<Client<B>>>,
    pub should_quit: Parker,
    /// This is what actually takes control of the program. It could be e.g. a GUI or a SPRT runner.
    pub input: Box<dyn Input<B>>,
}

impl<B: Board> RunClient<B> {
    pub fn create(
        input: Box<dyn Input<B>>,
        all_outputs: Vec<Box<dyn OutputBuilder<B>>>,
        args: &CommandLineArgs,
    ) -> Res<Self> {
        let should_quit = Parker::new();
        let client = Client::create(should_quit.unparker().clone(), all_outputs, args)?;
        Ok(Self {
            client,
            should_quit,
            input,
        })
    }
}

impl<B: Board> AbstractRun for RunClient<B> {
    fn run(&mut self) -> Quitting {
        {
            let mut guard = self.client.lock().unwrap();
            guard.new_match(0, 1);
        }
        self.input.assume_control(self.client.clone());
        self.should_quit.park();
        // The program has been closed.
        // Calling the `drop` implementation of an engine player ensures that its child process is terminated.
        // However, this only happens if no other thread is still holding an `Arc` to the client when this thread
        // exits. Therefore, we wait for the input thread(s) to terminate before exiting the main thread.
        self.input.join_threads();
        assert_eq!(Arc::strong_count(&self.client), 1);
        QuitMatch
    }
}
