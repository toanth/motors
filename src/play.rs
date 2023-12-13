use std::fmt::{Debug, Display, Formatter};

use colored::Colorize;
use itertools::Itertools;
use rand::rngs::StdRng;
use rand::{thread_rng, RngCore, SeedableRng};

use crate::games::chess::Chessboard;
use crate::games::mnk::MNKBoard;
use crate::games::PlayerResult::*;
use crate::games::{
    Board, Color, CreateEngine, EngineList, GraphicsList, Move, PlayerResult, RectangularBoard,
    RectangularCoordinates,
};
use crate::play::AdjudicationReason::*;
use crate::play::GameResult::Aborted;
use crate::play::MatchStatus::Over;
use crate::search::multithreading::{EngineOwner, EnginePlayer};
use crate::search::naive_slow_negamax::NaiveSlowNegamax;
use crate::search::random_mover::RandomMover;
use crate::search::{Engine, Searcher};
use crate::ui::text_ui::TextUI;
use crate::ui::{to_ui_handle, GraphicsHandle, UIHandle};

pub mod ugi;

pub mod run_match;

pub trait AbstractMatchManager: Debug {
    /// This does not only run a match, but also deals with options, such as handling UCI options
    /// Doesn't run asynchronously, so when this function returns the game has ended.
    fn run(&mut self) -> MatchResult;

    fn next_match(&mut self) -> Option<AnyMatch>;

    fn set_next_match(&mut self, next: Option<AnyMatch>);

    fn active_player(&self) -> Option<Color>;

    fn abort(&mut self) -> Result<MatchStatus, String>;

    fn match_status(&self) -> MatchStatus;

    fn game_name(&self) -> String;

    fn debug_mode(&self) -> bool;
}

pub trait MatchManager<B: Board>: AbstractMatchManager {
    fn board(&self) -> B;

    fn initial_pos(&self) -> B;

    fn move_history(&self) -> &[B::Move];

    fn last_move(&self) -> Option<B::Move> {
        self.move_history().last().copied()
    }

    fn graphics(&self) -> GraphicsHandle<B>;

    fn set_graphics(&mut self, graphics: GraphicsHandle<B>);

    // fn searcher(&self, _idx: usize) -> &dyn Searcher<B>;

    /// Should also set the engine's info callback
    fn set_engine(&mut self, idx: usize, engine: AnyEngine<B>);

    fn set_board(&mut self, board: B);
}

/// Is not object safe because it contains methods that don't start with `&mut self`, and a GAT.
pub trait CreatableMatchManager: AbstractMatchManager + 'static {
    type ForGame<C: Board>: MatchManager<C>;

    fn with_engine_and_ui<C: Board>(engine: AnyEngine<C>, ui: UIHandle<C>) -> Self::ForGame<C>;

    fn for_game<C: Board>() -> Self::ForGame<C> {
        Self::with_engine_and_ui(default_engine(), to_ui_handle(TextUI::default()))
    }
}

/// Result of a match from a player's perspective, together with the reason for this outcome
#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub struct GameOver {
    result: PlayerResult,
    reason: GameOverReason,
}

/// Status of a match from a MatchManager's perspective.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
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

/// Low-Level Result of a match from a MatchManager's perspective
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
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum AdjudicationReason {
    TimeUp,
    InvalidMove,
    AbortedByUser,
    EngineError,
}

impl Display for AdjudicationReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeUp => write!(f, "Time up"),
            InvalidMove => write!(f, "Invalid move"),
            AbortedByUser => write!(f, "Aborted by user"),
            EngineError => write!(f, "Engine error"),
        }
    }
}

/// Reason for why a match ended.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
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
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct MatchResult {
    pub result: GameResult,
    pub reason: GameOverReason,
}

pub type AnySearcher<B> = Box<dyn Searcher<B>>;

pub type AnyEngine<B> = Box<dyn EnginePlayer<B>>;

pub type AnyEngineRef<'a, B> = &'a dyn EnginePlayer<B>;

pub type AnyMutEngineRef<'a, B> = &'a mut dyn EnginePlayer<B>;

/// `AnyMatch` is a type-erased `MatchManager`, and almost the only thing that isn't generic over the Game.
/// Pretty much the entire program is spent inside the match manager.
pub type AnyMatch = Box<dyn AbstractMatchManager>;

struct MoveRes<B: Board> {
    mov: B::Move,
    board: B,
}

fn player_res_to_match_res(game_over: GameOver, is_p1: bool) -> MatchResult {
    let result = match game_over.result {
        Draw => GameResult::Draw,
        res => {
            if is_p1 == (res == Win) {
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

pub struct DefaultEngineList {}

pub fn generic_engines<B: Board>() -> Vec<(String, CreateEngine<B>)> {
    vec![
        ("random".to_string(), |_| {
            // there's probably a better way to seed the rng, but this should be good enough
            let owner =
                RandomMover::<B, StdRng>::with_rng(StdRng::seed_from_u64(thread_rng().next_u64()));
            Box::new(owner)
        }),
        ("naive_negamax".to_string(), |_| {
            let owner = EngineOwner::new::<NaiveSlowNegamax<B>>();
            Box::new(owner)
        }),
    ]
}

impl<B: Board> EngineList<B> for DefaultEngineList {
    fn list_engines() -> Vec<(String, CreateEngine<B>)> {
        generic_engines()
    }
}

pub fn select_from_name_with_err<T, F: Fn(&str, String) -> String>(
    name: &str,
    mut list: Vec<(String, T)>,
    err_msg: F,
) -> Result<T, String> {
    let idx = list.iter().find_position(|(key, _value)| key == name);
    if idx.is_none() {
        let list = Iterator::intersperse(
            list.iter().map(|(key, _val)| key.bold().to_string()),
            ", ".to_string(),
        )
        .collect::<String>();
        return Err(err_msg(name, list));
    }
    let idx = idx.unwrap().0;
    Ok(list.swap_remove(idx).1)
}

pub fn select_from_name<T>(
    name: &str,
    list: Vec<(String, T)>,
    typ: &str,
    game_name: &str,
) -> Result<T, String> {
    let err_func = |name: &str, list| {
        let game_name = game_name.bold();
        let name = name.red();
        format!(
            "Couldn't find {typ} '{name}' for the current game ({game_name}). Valid {typ} names are {0}.",
            list,
            )
    };
    select_from_name_with_err(name, list, err_func)
}

pub fn set_graphics_from_str<B: Board>(
    manager: &mut dyn MatchManager<B>,
    name: &str,
) -> Result<MatchStatus, String> {
    let create_graphics = select_from_name(
        name,
        B::GraphicsList::list_graphics(),
        "graphics",
        &B::game_name(),
    )?;
    let graphics = create_graphics("");
    manager.set_graphics(graphics);
    Ok(manager.match_status())
}

pub fn set_engine_from_str<B: Board>(
    manager: &mut dyn MatchManager<B>,
    name: &str,
) -> Result<MatchStatus, String> {
    let create_engine = select_from_name(
        name,
        B::EngineList::list_engines(),
        "engine",
        &B::game_name(),
    )?;
    manager.set_engine(0, create_engine(""));
    Ok(manager.match_status())
}

pub fn set_match_from_str<B: Board, M: MatchManager<B> + CreatableMatchManager>(
    manager: &mut M,
    name: &str,
) -> Result<MatchStatus, String> {
    let created_match = select_from_name(name, game_list::<M>(), "engine", &B::game_name())?;
    manager.set_next_match(Some(created_match));
    manager.abort()
}

pub fn set_position_from_str<B: Board, M: MatchManager<B>>(
    manager: &mut M,
    name: &str,
) -> Result<MatchStatus, String> {
    let fen = select_from_name(name, B::name_to_fen_map(), "position", &B::game_name())?;
    manager.set_board(B::from_fen(&fen)?);
    Ok(manager.match_status())
}

////// TODO: Refactor Player, also use in UCI mode: Split MatchManager trait into MatchManager struct and Player,
////// where a MatchManager controls the match like cutechess, but a Player controls only one single searcher.
////// Use Player struct in UCI mode as well, use adapters for different input format -- the extra additions like printing the board / game
////// should be handled by the Player. Very clear border between parsing UCI (input adapter of Player),
////// and sending commands to the player through normal functions.
////// Rough idea (to be iterated upon and eventually implemented in separate branch):
////// Player determines which commands are accepted and send to the searcher or answered directly (like printing the board),
////// decoupled from input parsing. A player contains an optional fallback that handles otherwise unrecognized inputs
////// MatchManager doesn't need to be a trait anymore because there's only one (generic) struct implementing it,
////// Player also has only one implementation, which is supposed to be minimal and fast, but optional extensions that enable stuff like remembering moves.
////// Output and Input are completely decoupled, the player doesn't know anything about either one (they always exist, can be different per player).
////// Input and output layer are stateless.
////// So in this model, each player has a UI, unlike the UCI model, where UI and match management are bundled together.
////// A player holds a MatchWriter and a list of graphics, there is also one MatchReader and a list of UIs,
////// but the player isn't aware of them, it simply receives commands from them.
////// There's a trait for Writer that is satisfied both by the MatchWriter and the UIs, and a trait for Reader
////// that is satisfied both by the MatchReader and (some of) the UIs.
////// The readers should all run in their own thread.
////// It probably makes sense to have a PlayManager struct that has a player and also holds the UI.
////// The MatchManager can be called Organizer.
////// The UCI model bundles the UI and the match manager together, but that doesn't always make sense
////// (the player should be able to do debug printing) and I need to implement the UI for games other than chess anyway,
////// so I can't rely on an external program like cutechess or similar to handle the UI part.

pub fn default_engine<B: Board>() -> AnyEngine<B> {
    let engine_list = B::EngineList::list_engines();
    let (_name, create) = engine_list.last().unwrap();
    create("")
}

pub fn game_list<M: CreatableMatchManager>() -> Vec<(String, AnyMatch)> {
    vec![
        ("mnk".to_string(), Box::new(M::for_game::<MNKBoard>())),
        ("chess".to_string(), Box::new(M::for_game::<Chessboard>())),
    ]
}
