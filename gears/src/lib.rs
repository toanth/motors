//! [`gears`](crate) is a board game library. It deals with board representation, move generation, FEN parsing, etc.
//! It is designed to be easily extensible to new games. [`gears`](crate) forms the foundation of the `motors`, `monitors`
//! and `pliers` crates, which deal with engines, UI, and tuning, respectively.

use std::fmt::{Debug, Display, Formatter};
use std::time::Instant;

use crate::games::Color::White;
use crate::games::{Board, Color};
use crate::general::common::Description::WithDescription;
use crate::general::common::{select_name_dyn, Res};
use crate::output::OutputBuilder;
use crate::search::TimeControl;
use crate::AdjudicationReason::*;
use crate::GameResult::Aborted;
use crate::MatchStatus::Over;
use crate::PlayerResult::Win;

/// A few helpers for interacting with the command line.
pub mod cli;
/// Anything related to the specific games, organized in submodules like "chess".
pub mod games;
/// Anything that doesn't fit into the other modules, such as low-level helper functions
pub mod general;
/// Anything related to printing the game. A part of this library instead of the `monitors` crate
/// because it's very helpful to allow an engine to do debug printing and logging.
/// Still, the monitors crate contains more advanced UIs, such as a GUI.
pub mod output;
/// Score and packed score
pub mod score;
/// Basic search helper types and functions that are used by `motors` and `monitors`
pub mod search;
/// Ugi helpers used both by `motors` and `monitors`
pub mod ugi;

// *** Match status information ***

/// Result of a match from a player's perspective.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PlayerResult {
    Win,
    Lose,
    Draw,
}

/// Result of a match from a player's perspective, together with the reason for this outcome
#[derive(Eq, PartialEq, Clone, Debug)]
pub struct GameOver {
    pub result: PlayerResult,
    pub reason: GameOverReason,
}

/// Status of a match from a MatchManager's perspective.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub enum MatchStatus {
    #[default]
    NotStarted,
    Ongoing,
    Over(MatchResult),
}

impl MatchStatus {
    pub fn aborted() -> Self {
        Over(MatchResult {
            result: Aborted,
            reason: GameOverReason::Adjudication(AbortedByUser),
        })
    }
}

/// Low-level result of a match from a MatchManager's perspective
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum GameResult {
    P1Win,
    P2Win,
    Draw,
    Aborted,
}

impl Display for GameResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GameResult::P1Win => write!(f, "Player 1 won"),
            GameResult::P2Win => write!(f, "Player 2 won"),
            GameResult::Draw => write!(f, "The game ended in a draw"),
            Aborted => write!(f, "The game was aborted"),
        }
    }
}

/// Reason for why the match manager adjudicated a match
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum AdjudicationReason {
    TimeUp,
    InvalidMove,
    AbortedByUser,
    EngineError,
    Adjudicator(String), // e.g. both engines displayed a winning score for one player for many consecutive moves
}

impl Display for AdjudicationReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeUp => write!(f, "Time up"),
            InvalidMove => write!(f, "Invalid move"),
            AbortedByUser => write!(f, "Aborted by user"),
            EngineError => write!(f, "Engine error"),
            Adjudicator(reason) => write!(f, "Matchmaker adjudication: {reason}"),
        }
    }
}

/// Reason for why a match ended.
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum GameOverReason {
    Normal,
    Adjudication(AdjudicationReason),
}

impl Display for GameOverReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GameOverReason::Normal => write!(f, "The game ended normally"),
            GameOverReason::Adjudication(a) => write!(f, "{a}"),
        }
    }
}

/// Result of a match from a MatchManager's perspective, with the reason for why it ended.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MatchResult {
    pub result: GameResult,
    pub reason: GameOverReason,
}

pub fn player_res_to_match_res(game_over: GameOver, color: Color) -> MatchResult {
    let result = match game_over.result {
        PlayerResult::Draw => GameResult::Draw,
        res => {
            if (color == White) == (res == Win) {
                GameResult::P1Win
            } else {
                GameResult::P2Win
            }
        }
    };
    MatchResult {
        result,
        reason: game_over.reason,
    }
}

#[derive(Debug, Clone)]
pub struct OutputArgs {
    pub name: String,
    pub opts: Vec<String>,
}

impl OutputArgs {
    pub fn new(name: String) -> Self {
        Self { name, opts: vec![] }
    }
}

/// The user can decide to quit either the current match or the entire program.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Quitting {
    QuitProgram,
    QuitMatch,
}

/// Base trait for the different modes in which the user can run the program.
/// It only contains one method: [`run`].
pub trait AbstractRun: Debug {
    fn run(&mut self) -> Quitting;
}

/// `AnyRunnable` is a type-erased [`AbstractRun`], and almost the only thing that isn't generic over the Game.
/// Pretty much the entire program is spent inside the match manager.
pub type AnyRunnable = Box<dyn AbstractRun>;

/// The current state of the match.
pub trait GameState<B: Board> {
    fn initial_pos(&self) -> B;
    fn get_board(&self) -> B;
    fn game_name(&self) -> &'static str {
        B::game_name()
    }
    fn move_history(&self) -> &[B::Move];
    fn active_player(&self) -> Color {
        self.get_board().active_player()
    }
    fn last_move(&self) -> Option<B::Move> {
        self.move_history().last().copied()
    }
    fn ply_count(&self) -> usize {
        self.move_history().len()
    }
    fn match_status(&self) -> MatchStatus;
    // fn clear_state(&mut self);
    /// For the UGI client, this returns "gui". For an engine, it returns the name of the engine.
    fn name(&self) -> &str;
    fn event(&self) -> String;
    fn site(&self) -> &str;
    /// The name of the player, if known (i.e. display_name for the GUI and None for the other player of an engine)
    fn player_name(&self, color: Color) -> Option<&str>;
    fn time(&self, color: Color) -> Option<TimeControl>;
    fn thinking_since(&self, color: Color) -> Option<Instant>;
}

pub fn output_builder_from_str<B: Board>(
    name: &str,
    list: &[Box<dyn OutputBuilder<B>>],
) -> Res<Box<dyn OutputBuilder<B>>> {
    Ok(dyn_clone::clone_box(select_name_dyn(
        name,
        list,
        "output",
        B::game_name(),
        WithDescription,
    )?))
}

pub fn create_selected_output_builders<B: Board>(
    outputs: &[OutputArgs],
    list: &[Box<dyn OutputBuilder<B>>],
) -> Res<Vec<Box<dyn OutputBuilder<B>>>> {
    outputs
        .iter()
        .map(|o| output_builder_from_str(&o.name, list))
        .collect()
}
