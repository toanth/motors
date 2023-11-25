use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use std::str::SplitWhitespace;
use std::time::{Duration, Instant};

use colored::Colorize;
use itertools::Itertools;
use rand::prelude::ThreadRng;

use crate::games::chess::Chessboard;
use crate::games::mnk::MNKBoard;
use crate::games::{
    Board, Color, CreateEngine, EngineList, GraphicsList, Move, RectangularBoard,
    RectangularCoordinates,
};
use crate::play::AdjudicationReason::*;
use crate::play::GameResult::Aborted;
use crate::play::MatchStatus::Over;
use crate::play::PlayerResult::{Draw, Lose, Win};
use crate::search::human::Human;
use crate::search::naive_slow_negamax::NaiveSlowNegamax;
use crate::search::random_mover::RandomMover;
use crate::search::{Engine, SearchInfo, SearchLimit, SearchResult, Searcher};
use crate::ui::text_ui::TextUI;
use crate::ui::Message::{Info, Warning};
use crate::ui::{to_ui_handle, GraphicsHandle, UIHandle};

pub mod ugi;

pub mod run_match;

pub trait AbstractMatchManager: Debug {
    fn active_player(&self) -> Option<Color>;

    /// This does not only run a match, but also deals with options, such as handling UCI options
    /// Doesn't run asynchronously, so when this function returns the game has ended.
    fn run(&mut self) -> MatchResult;

    fn abort(&mut self) -> Result<MatchStatus, String>;

    fn match_status(&self) -> MatchStatus;

    fn game_name(&self) -> String;

    fn next_match(&mut self) -> Option<AnyMatch>;

    fn set_next_match(&mut self, next: Option<AnyMatch>);
}

pub trait MatchManager<B: Board>: AbstractMatchManager {
    fn board(&self) -> B;

    fn initial_pos(&self) -> B;

    fn move_history(&self) -> &[B::Move];

    fn last_move(&self) -> Option<B::Move> {
        self.move_hist().last().copied()
    }

    fn format_info(&self, info: SearchInfo<B>) -> String;

    fn move_hist(&self) -> &[B::Move];

    fn graphics(&self) -> GraphicsHandle<B>;
}

/// Is not object safe because it contains methods that don't start with `&mut self`, and a GAT.
pub trait CreatableMatchManager: AbstractMatchManager + 'static {
    type ForGame<C: Board>: MatchManager<C>;

    fn with_engine_and_ui<C: Board>(engine: AnyEngine<C>, ui: UIHandle<C>) -> Self::ForGame<C>;

    fn for_game<C: Board>() -> Self::ForGame<C> {
        Self::with_engine_and_ui(default_engine(), to_ui_handle(TextUI::default()))
    }
}

/// Result of a match from a player's perspective.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum PlayerResult {
    Win,
    Lose,
    Draw,
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

pub type AnyEngine<B> = Box<dyn Engine<B>>;

/// `AnyMatch` is a type-erased `MatchManager`, and almost the only thing that isn't generic over the Game.
/// Pretty much the entire program is spent inside the match manager.
pub type AnyMatch = Box<dyn AbstractMatchManager>;

/// A player for the built-in match manager. TODO: Refactor
#[derive(Debug)]
pub struct Player<B: Board> {
    pub searcher: AnySearcher<B>,
    pub limit: SearchLimit,
    pub original_limit: SearchLimit,
    pub retry_on_invalid_move: bool,
    phantom: PhantomData<B>,
}

impl<B: Board> Player<B> {
    pub fn make_move(&mut self, pos: B) -> SearchResult<B> {
        self.searcher.search(pos, self.limit)
    }

    pub fn update_time(&mut self, time_spent_last_move: Duration) -> MatchStatus {
        let max_time = self.limit.tc.remaining.max(self.limit.fixed_time);
        if time_spent_last_move > max_time {
            return Over(MatchResult {
                result: Aborted,
                reason: GameOverReason::Adjudication(TimeUp),
            });
        }
        self.limit.tc.remaining -= time_spent_last_move;
        if self.limit.tc.moves_to_go == 1 {
            self.limit.tc.moves_to_go = self.original_limit.tc.moves_to_go;
            self.limit.tc.remaining += self.original_limit.tc.remaining;
        } else if self.limit.tc.moves_to_go != 0 {
            self.limit.tc.moves_to_go -= 1;
        }
        MatchStatus::Ongoing
    }

    pub fn new_for_searcher<S: Searcher<B>>(searcher: S, limit: SearchLimit) -> Self {
        Self::new(Box::new(searcher), limit)
    }

    pub fn new(searcher: AnySearcher<B>, limit: SearchLimit) -> Self {
        Self {
            searcher,
            limit,
            original_limit: limit,
            retry_on_invalid_move: false,
            phantom: Default::default(),
        }
    }

    pub fn human(ui: UIHandle<B>) -> Self {
        Player {
            searcher: Box::new(Human::new(ui)),
            limit: SearchLimit::infinite(),
            original_limit: SearchLimit::infinite(),
            retry_on_invalid_move: true,
            phantom: Default::default(),
        }
    }
}

struct MoveRes<B: Board> {
    mov: B::Move,
    board: B,
}

fn make_move<B: Board>(
    pos: B,
    player: &mut Player<B>,
    graphics: GraphicsHandle<B>,
    move_hist: &mut Vec<B::Move>,
) -> Result<MoveRes<B>, GameOver> {
    if pos.is_draw() {
        return Err(GameOver {
            result: Draw,
            reason: GameOverReason::Normal,
        });
    }
    if pos.is_game_lost() {
        return Err(GameOver {
            result: Lose,
            reason: GameOverReason::Normal,
        });
    }

    graphics.borrow_mut().display_message(
        Info,
        format!("Player: {0}", player.searcher.name()).as_str(),
    );

    let new_pos;
    let mut response;

    loop {
        let start_time = Instant::now();
        response = player.make_move(pos);
        let duration = start_time.elapsed();

        if let MatchStatus::Over(res) = player.update_time(duration) {
            assert_eq!(res.result, Aborted);
            return Err(GameOver {
                result: Lose,
                reason: res.reason,
            });
        }

        let mov = response.chosen_move;
        if pos.is_move_legal(mov) {
            new_pos = pos.make_move(mov).unwrap();
            break;
        }

        if player.retry_on_invalid_move {
            graphics
                .borrow_mut()
                .display_message(Warning, "Invalid move. Try again:");
            continue;
        }
        move_hist.push(mov);
        return Err(GameOver {
            result: Lose,
            reason: GameOverReason::Adjudication(InvalidMove),
        });
    }

    graphics.borrow_mut().display_message(
        Info,
        format!(
            "Eval: {0}",
            response
                .score
                .map_or("no score".to_string(), |s| s.0.to_string())
        )
        .as_str(),
    );

    Ok(MoveRes {
        mov: response.chosen_move,
        board: new_pos,
    })
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

// TODO: Rework the built-in match manager, use UGI exclusively to communicate with players to support external engines
#[derive(Debug)]
pub struct BuiltInMatch<B: Board> {
    board: B,
    move_hist: Vec<B::Move>,
    status: MatchStatus,
    p1: Player<B>,
    p2: Player<B>,
    graphics: GraphicsHandle<B>,
    next_match: Option<AnyMatch>,
}

impl<B: Board> AbstractMatchManager for BuiltInMatch<B> {
    fn active_player(&self) -> Option<Color> {
        if let MatchStatus::Ongoing = self.status {
            Some(self.board.active_player())
        } else {
            None
        }
    }

    fn match_status(&self) -> MatchStatus {
        self.status
    }

    /// Runs the entire game in this function.
    /// TODO: Another implementation would be to run this asynchronously, but I don't want to deal with multithreading right now
    fn run(&mut self) -> MatchResult {
        self.graphics.borrow_mut().show(self);
        loop {
            let res = make_move(
                self.board,
                &mut self.p1,
                self.graphics.clone(),
                &mut self.move_hist,
            );
            if res.is_err() {
                return player_res_to_match_res(res.err().unwrap(), true);
            }
            self.make_move(res.unwrap());

            let res = make_move(
                self.board,
                &mut self.p2,
                self.graphics.clone(),
                &mut self.move_hist,
            );
            if res.is_err() {
                return player_res_to_match_res(res.err().unwrap(), false);
            }
            self.make_move(res.unwrap());
        }
    }

    fn abort(&mut self) -> Result<MatchStatus, String> {
        self.status = MatchStatus::aborted();
        Ok(self.status)
    }

    fn game_name(&self) -> String {
        B::game_name()
    }

    fn next_match(&mut self) -> Option<AnyMatch> {
        self.next_match.take()
    }

    fn set_next_match(&mut self, next: Option<AnyMatch>) {
        self.next_match = next;
    }
}

impl<B: Board> MatchManager<B> for BuiltInMatch<B> {
    fn board(&self) -> B {
        self.board
    }

    fn graphics(&self) -> GraphicsHandle<B> {
        self.graphics.clone()
    }

    fn move_hist(&self) -> &[B::Move] {
        self.move_hist.as_slice()
    }

    fn format_info(&self, info: SearchInfo<B>) -> String {
        format!(
            "After {0} milliseconds: Move {1}, score {2}",
            info.time.as_millis(),
            info.best_move,
            info.score.0
        )
    }

    fn initial_pos(&self) -> B {
        B::startpos(self.board.settings())
    }

    fn move_history(&self) -> &[B::Move] {
        self.move_hist.as_slice()
    }
}

impl<B: Board> CreatableMatchManager for BuiltInMatch<B> {
    type ForGame<C: Board> = BuiltInMatch<C>;

    fn with_engine_and_ui<C: Board>(engine: AnyEngine<C>, ui: UIHandle<C>) -> Self::ForGame<C> {
        let player_1 = Player::human(ui.clone());
        let limit = SearchLimit::per_move(Duration::from_millis(1_000));
        let player_2 = Player::new(engine, limit);
        <Self::ForGame<C>>::new(C::Settings::default(), player_1, player_2, ui)
    }
}

impl<B: Board> Default for BuiltInMatch<B> {
    fn default() -> Self {
        Self::with_engine_and_ui(default_engine(), to_ui_handle(TextUI::default()))
    }
}

impl<B: Board> BuiltInMatch<B> {
    pub fn new(
        game_settings: B::Settings,
        player_1: Player<B>,
        player_2: Player<B>,
        graphics: GraphicsHandle<B>,
    ) -> Self {
        Self::from_position(B::startpos(game_settings), player_1, player_2, graphics)
    }

    pub fn from_position(
        pos: B,
        p1: Player<B>,
        p2: Player<B>,
        graphics: GraphicsHandle<B>,
    ) -> Self {
        BuiltInMatch {
            board: pos,
            move_hist: vec![],
            status: MatchStatus::NotStarted,
            p1,
            p2,
            graphics,
            next_match: None,
        }
    }

    fn make_move(&mut self, move_res: MoveRes<B>) {
        self.board = move_res.board;
        self.move_hist.push(move_res.mov);
        self.graphics.borrow_mut().show(self);
    }
}

pub struct DefaultEngineList {}

pub fn generic_engines<B: Board>() -> Vec<(String, CreateEngine<B>)> {
    vec![
        ("random".to_string(), |_| {
            Box::new(RandomMover::<B, ThreadRng>::default())
        }),
        ("naive_negamax".to_string(), |_| {
            Box::new(NaiveSlowNegamax::default())
        }),
    ]
}

impl<B: Board> EngineList<B> for DefaultEngineList {
    fn list_engines() -> Vec<(String, CreateEngine<B>)> {
        generic_engines()
    }
}

struct CompletionList<'a, F, T, U, R>
where
    F: Fn(&mut T, U) -> Result<R, String>,
    T: AbstractMatchManager,
{
    typ: &'static str,
    list: Vec<(String, U)>,
    set_elem: F,
    manager: &'a mut T,
}

impl<'a, F, T, U, R> CompletionList<'a, F, T, U, R>
where
    F: Fn(&mut T, U) -> Result<R, String>,
    T: AbstractMatchManager,
{
    fn handle_input(self, mut words: SplitWhitespace) -> Result<R, String> {
        let name = words
            .next()
            .ok_or_else(|| format!("Missing {} name", self.typ))?;
        let rest = words.remainder().unwrap_or_default();
        if !rest.trim().is_empty() {
            return Err(format!("Additional input after {0}: '{rest}'", self.typ));
        }
        self.handle_name(name)
    }

    fn handle_name(mut self, name: &str) -> Result<R, String> {
        let idx = self.list.iter().find_position(|(key, _value)| key == name);
        if idx.is_none() {
            let list = Iterator::intersperse(
                self.list.iter().map(|(key, _val)| key.bold().to_string()),
                ", ".to_string(),
            )
            .collect::<String>();
            let game_name = self.manager.game_name().bold();
            let name = name.red();
            return Err(format!(
                    "Couldn't find {typ} '{name}' for the current game ({game_name}). Valid {typ} names are {0}.",
                    list,
                    typ = self.typ,)
            );
        }
        let set_elem = self.set_elem;
        let idx = idx.unwrap().0;
        return set_elem(self.manager, self.list.swap_remove(idx).1);
    }
}

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
