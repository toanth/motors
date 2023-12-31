use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::str::FromStr;

use derive_more::{BitOrAssign, BitXorAssign};
use itertools::Itertools;
use num::iter;
use rand::Rng;
use strum_macros::EnumIter;

use crate::games::PlayerResult::{Draw, Lose};
use crate::general::common::parse_int;
use crate::general::move_list::MoveList;
use crate::play::AnyEngine;
use crate::ui::GraphicsHandle;

pub mod mnk;

pub mod chess;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, EnumIter)]
pub enum Color {
    #[default]
    White = 0,
    Black = 1,
}

impl Color {
    pub fn other(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Color::White => write!(f, "white"),
            Color::Black => write!(f, "black"),
        }
    }
}

pub trait AbstractPieceType: Eq + Copy + Debug + Default + Display {
    fn empty() -> Self;

    fn to_ascii_char(self) -> char;

    fn to_utf8_char(self) -> char {
        self.to_ascii_char()
    }

    fn from_ascii_char(c: char) -> Option<Self> {
        Self::from_utf8_char(c)
    }

    fn from_utf8_char(c: char) -> Option<Self>;

    fn to_uncolored_idx(self) -> usize;
}

pub trait UncoloredPieceType: AbstractPieceType {
    type Colored: ColoredPieceType;

    fn from_uncolored_idx(idx: usize) -> Self;
}

pub trait ColoredPieceType: AbstractPieceType {
    type Uncolored: UncoloredPieceType;

    fn color(self) -> Option<Color>;

    fn uncolor(self) -> Self::Uncolored {
        Self::Uncolored::from_uncolored_idx(self.to_uncolored_idx())
    }

    fn to_colored_idx(self) -> usize;

    fn new(color: Color, uncolored: Self::Uncolored) -> Self;
}

// pub trait UncoloredPiece: Eq + Copy + Debug + Default {
//     type Coordinates: Coordinates;
//     type UncoloredPieceType: UncoloredPieceType;
//     fn coordinates(self) -> Self::Coordinates;
//
//     fn uncolored_piece_type(self) -> Self::UncoloredPieceType;
//
//     fn to_utf8_char(self) -> char {
//         self.to_ascii_char()
//     }
//
//     fn to_ascii_char(self) -> char;
//
//     fn is_empty(self) -> bool {
//         self.uncolored_piece_type() == Self::UncoloredPieceType::empty()
//     }
// }

pub trait ColoredPiece: Eq + Copy + Debug + Default {
    type ColoredPieceType: ColoredPieceType;
    type Coordinates: Coordinates;
    fn coordinates(self) -> Self::Coordinates;

    fn uncolored(self) -> <Self::ColoredPieceType as ColoredPieceType>::Uncolored {
        self.colored_piece_type().uncolor()
    }

    fn to_utf8_char(self) -> char {
        self.colored_piece_type().to_utf8_char()
    }

    fn to_ascii_char(self) -> char {
        self.colored_piece_type().to_ascii_char()
    }

    fn is_empty(self) -> bool {
        self.colored_piece_type() == Self::ColoredPieceType::empty()
    }

    fn colored_piece_type(self) -> Self::ColoredPieceType;

    fn color(self) -> Option<Color> {
        self.colored_piece_type().color()
    }
}

#[derive(Eq, PartialEq, Default, Debug, Copy, Clone)]
pub struct GenericPiece<C: Coordinates, T: ColoredPieceType> {
    symbol: T,
    coordinates: C,
}

impl<C: Coordinates, T: ColoredPieceType> ColoredPiece for GenericPiece<C, T> {
    type ColoredPieceType = T;
    type Coordinates = C;

    fn coordinates(self) -> Self::Coordinates {
        self.coordinates
    }

    fn colored_piece_type(self) -> Self::ColoredPieceType {
        self.symbol
    }
}

impl<C: Coordinates, T: ColoredPieceType> Display for GenericPiece<C, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.symbol, f)
    }
}

// pub enum TextRepresentation {
//     Fen,
//     AsciiDiagram,
//     Utf8Diagram,
// }

// Assume 2D grid for now.
pub trait Coordinates: Eq + Copy + Debug + Default + FromStr<Err = String> + Display {
    type Size: Size<Self>;
    // fn new(_: usize, _: usize) -> Self;
    //
    // fn row(self) -> usize;
    //
    // fn column(self) -> usize;

    /// mirrors the coordinates vertically
    fn flip_up_down(self, size: Self::Size) -> Self;

    /// mirrors the coordinates horizontally
    fn flip_left_right(self, size: Self::Size) -> Self;

    fn no_coordinates() -> Self;
}

pub trait RectangularCoordinates: Coordinates {
    fn from_row_column(row: usize, column: usize) -> Self;
    fn row(self) -> usize;
    fn column(self) -> usize;
}

// Computes the L1 norm of a - b
pub fn manhattan_distance<C: RectangularCoordinates>(a: C, b: C) -> usize {
    a.row().abs_diff(b.row()) + a.column().abs_diff(b.column())
}

// Compute the supremum norm of a - b
pub fn sup_distance<C: RectangularCoordinates>(a: C, b: C) -> usize {
    a.row()
        .abs_diff(b.row())
        .max(a.column().abs_diff(b.column()))
}

#[derive(Clone, Copy, Eq, PartialOrd, PartialEq, Debug, Default)]
pub struct GridCoordinates {
    pub row: usize,
    pub column: usize,
}

impl Coordinates for GridCoordinates {
    type Size = GridSize;

    fn flip_up_down(self, size: Self::Size) -> Self {
        GridCoordinates {
            row: size.height.0 - 1 - self.row,
            column: self.column,
        }
    }

    fn flip_left_right(self, size: Self::Size) -> Self {
        GridCoordinates {
            row: self.row,
            column: size.width.0 - 1 - self.column,
        }
    }

    fn no_coordinates() -> Self {
        GridCoordinates {
            row: u32::MAX as usize,
            column: u32::MAX as usize,
        }
    }
}

impl RectangularCoordinates for GridCoordinates {
    fn from_row_column(row: usize, column: usize) -> Self {
        GridCoordinates { row, column }
    }

    fn row(self) -> usize {
        self.row
    }

    fn column(self) -> usize {
        self.column
    }
}

impl FromStr for GridCoordinates {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = s.trim().chars();

        let column = s
            .next()
            .and_then(|c| {
                if c.is_ascii_alphabetic() {
                    Some(c.to_ascii_lowercase() as usize - 'a' as usize)
                } else {
                    None
                }
            })
            .ok_or("file (column) must be a valid ascii letter")?;
        let mut words = s.as_str().split_whitespace();
        let row: usize = parse_int(&mut words, "rank (row)")?;
        if words.count() > 0 {
            return Err("too many words".to_string());
        }
        Ok(GridCoordinates {
            column,
            row: row.wrapping_sub(1),
        })
    }
}

impl Display for GridCoordinates {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{0}{1}",
            (self.column + 'a' as usize) as u8 as char,
            self.row + 1 // output 1-indexed
        )
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Default)]
pub struct Height(pub usize);

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Default)]
pub struct Width(pub usize);

pub trait Size<C: Coordinates>: Eq + PartialEq + Copy + Clone + Debug {
    fn num_squares(self) -> usize;

    fn to_idx(self, coordinates: C) -> usize;

    fn to_coordinates(self, idx: usize) -> C;

    fn valid_coordinates(self, coordinates: C) -> bool;
}

pub trait RectangularSize<C: RectangularCoordinates>: Size<C> {
    fn height(self) -> Height;
    fn width(self) -> Width;
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct GridSize {
    pub height: Height,
    pub width: Width,
}

impl GridSize {
    pub const fn new(height: Height, width: Width) -> Self {
        Self { height, width }
    }

    pub const fn chess() -> Self {
        Self::new(Height(8), Width(8))
    }

    pub const fn tictactoe() -> Self {
        Self::new(Height(3), Width(3))
    }

    pub const fn connect4() -> Self {
        Self::new(Height(6), Width(7))
    }
}

impl Size<GridCoordinates> for GridSize {
    fn num_squares(self) -> usize {
        self.height.0 * self.width.0
    }

    // fn to_coordinates(self, idx: usize) -> GridCoordinates {
    //     GridCoordinates {
    //         row: idx / self.width().0,
    //         column: idx % self.width().0,
    //     }
    // }

    fn to_idx(self, coordinates: GridCoordinates) -> usize {
        coordinates.row() * self.width.0 + coordinates.column()
    }

    fn to_coordinates(self, idx: usize) -> GridCoordinates {
        GridCoordinates {
            row: idx / self.width.0,
            column: idx % self.width.0,
        }
    }

    fn valid_coordinates(self, coordinates: GridCoordinates) -> bool {
        coordinates.row() < self.height().0 && coordinates.column() < self.width().0
    }
}

impl RectangularSize<GridCoordinates> for GridSize {
    fn height(self) -> Height {
        self.height
    }

    fn width(self) -> Width {
        self.width
    }
}

pub trait MoveFlags: Eq + Copy + Debug + Default {}

#[derive(Eq, PartialEq, Debug, Copy, Clone, Default)]
pub struct NoMoveFlags {}

impl MoveFlags for NoMoveFlags {}

pub trait Move<B: Board>: Eq + Copy + Clone + Debug + Default + Display {
    type Flags: MoveFlags;

    /// From which square does the piece move?
    /// When this doesn't make sense, such as for m,n,k games, return some default value, such as `no_coordinates()`
    fn from_square(self) -> B::Coordinates;

    /// To which square does the piece move / get placed.
    fn to_square(self) -> B::Coordinates;

    /// Move flags. Not all Move implementations have them, in which case `Flags` can be `NoMoveFlags`
    fn flags(self) -> Self::Flags;

    /// Return a compact and easy to parse move representation, such as <from_square><to_square> as used by UCI
    fn to_compact_text(self) -> String;

    /// Parse a compact text representation emitted by `to_compact_text`, such as the one used by UCI
    fn from_compact_text(s: &str, board: &B) -> Result<Self, String>;

    /// Returns a longer representation of the move that may require the board, such as long algebraic notation
    fn to_extended_text(self, _board: &B) -> String {
        self.to_compact_text()
    }

    /// Parse a text representation of the move. This may be the same as `from_compact_text`
    /// or may use a different notation, such as standard algebraic notation in chess (TODO: Support both uci and long/short algebraic notation for chess)
    fn from_text(s: &str, board: &B) -> Result<Self, String> {
        Self::from_compact_text(s, board)
    }
}

pub type CreateGraphics<B> = fn(&str) -> GraphicsHandle<B>;

pub type CreateEngine<B> = fn(&str) -> AnyEngine<B>;

/// It's very inelegant to have the Board define what graphics / engines support it, but
/// unfortunately this is the only way I found to do that without using the unstable `specialization` feature.
/// I really don't like this code, but I don't know a better way to write this in Rust :/
pub trait GraphicsList<B: Board> {
    fn list_graphics() -> Vec<(String, CreateGraphics<B>)>;
}

/// Lists all the engines that support this game.
/// The last entry in the list is the default engine, which will be chosen
/// if no other engine is specified.
pub trait EngineList<B: Board> {
    fn list_engines() -> Vec<(String, CreateEngine<B>)>;
}

#[derive(Copy, Clone, Eq, PartialEq, Default, Debug, derive_more::Display, BitXorAssign)]
pub struct ZobristHash(pub u64);

pub trait Settings: Eq + Copy + Debug + Default {}

/// Result of a match from a player's perspective.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PlayerResult {
    Win,
    Lose,
    Draw,
}

pub trait BoardHistory<B: Board>: Default + Debug + Clone + 'static {
    fn game_result(&self, board: &B) -> Option<PlayerResult>;
    fn is_repetition(&self, board: &B) -> bool {
        self.game_result(board).is_some()
    }
    fn push(&mut self, board: &B);
    fn pop(&mut self, _board: &B);
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct NoHistory {}

impl<B: Board> BoardHistory<B> for NoHistory {
    fn game_result(&self, board: &B) -> Option<PlayerResult> {
        None
    }

    fn push(&mut self, _board: &B) {}

    fn pop(&mut self, _board: &B) {}
}

#[derive(Clone, Eq, PartialEq, Default, Debug)]
pub struct ZobristHistoryBase(pub Vec<ZobristHash>);

impl ZobristHistoryBase {
    fn push(&mut self, hash: ZobristHash) {
        self.0.push(hash);
    }

    fn pop(&mut self) {
        self.0
            .pop()
            .expect("ZobristHistory::pop() called on empty history");
    }
}

fn find_repetition(
    hash: ZobristHash,
    max_history: usize,
    history: &ZobristHistoryBase,
    mut count: usize,
) -> bool {
    let stop = 0.max(history.0.len() as isize - max_history as isize);
    for i in iter::range_step_inclusive(history.0.len() as isize - 4, stop, -2) {
        if history.0[i as usize] == hash {
            count -= 1;
            if count <= 1 {
                return true;
            }
        }
    }
    false
}

#[derive(Clone, Eq, PartialEq, Default, Debug)]
pub struct ZobristRepetition2Fold(pub ZobristHistoryBase);

impl<B: Board> BoardHistory<B> for ZobristRepetition2Fold {
    fn game_result(&self, board: &B) -> Option<PlayerResult> {
        match find_repetition(
            board.zobrist_hash(),
            board.halfmove_repetition_clock(),
            &self.0,
            2,
        ) {
            true => Some(Draw),
            false => None,
        }
    }

    fn push(&mut self, board: &B) {
        self.0.push(board.zobrist_hash())
    }

    // the _board parameter is only there to deduce the trait
    fn pop(&mut self, _board: &B) {
        self.0.pop();
    }
}

#[derive(Default, Debug, Eq, PartialEq, Clone)]
pub struct ZobristRepetition3Fold(pub ZobristHistoryBase);

/// The threefold repetition rule is pretty common in games where repetitions regularly occur.
/// Only relies on the hashes, which isn't always entirely correct --
/// For example, the FIDE rule state that the set of legal moves must be identical, which is not the case
/// if the ep square is set (and therefore part of the hash) but the pawn is pinned and can't actually take.
/// TODO: Write another implementation that actually checks for that, to be used by the game manager.
impl<B: Board> BoardHistory<B> for ZobristRepetition3Fold {
    fn game_result(&self, board: &B) -> Option<PlayerResult> {
        match find_repetition(
            board.zobrist_hash(),
            board.halfmove_repetition_clock(),
            &self.0,
            3,
        ) {
            true => Some(Draw),
            false => None,
        }
    }

    fn push(&mut self, board: &B) {
        self.0.push(board.zobrist_hash())
    }

    fn pop(&mut self, _board: &B) {
        self.0.pop()
    }
}

/// Currently, a game is completely determined by the `Board` type:
/// The type implementing `Board` contains all the necessary information about the rules of the game.
/// However, a `Board` is assumed to be markovian and needs to satisfy `Copy` and `'static`.
/// When this is not desired, the `GameState` should be used instead, it contains a copy of the current board
/// and additional non-markovian information, such as the history of zobrist hashes for games that need this.
/// TODO: Use a struct containing the current board and the history, switch to make/unmake
pub trait Board:
    Eq + PartialEq + Sized + Default + Debug + Display + Copy + Clone + 'static
{
    type Settings: Settings;
    type Coordinates: Coordinates;
    // type Size: Size;
    type Piece: ColoredPiece;
    type Move: Move<Self>;
    type MoveList: MoveList<Self>;
    type LegalMoveList: MoveList<Self> + FromIterator<Self::Move>;
    type EngineList: EngineList<Self>;
    type GraphicsList: GraphicsList<Self>;

    /// Returns the name of the game, such as 'chess'
    fn game_name() -> String;

    /// An empty board. This does not have to be a valid position.
    fn empty(_: Self::Settings) -> Self {
        Default::default()
    }

    /// The starting position of the game.
    /// For games with random starting position, this function picks one randomly.
    fn startpos(settings: Self::Settings) -> Self;

    /// Constructs a specific, well-known position from its name, such as 'kiwipete' in chess.
    /// Not to be confused with `from_fen`, which can load arbitrary positions.
    fn from_name(name: &str) -> Option<Self> {
        Self::name_to_fen_map()
            .iter()
            .find(|(nam, _fen)| nam == name)
            .map(|(_name, fen)| Self::from_fen(fen.as_str()).unwrap())
            .or_else(|| {
                if name == "startpos" {
                    Some(Self::startpos(Self::Settings::default()))
                } else {
                    None
                }
            })
    }

    /// Returns a Vec mapping well-known position names to their FEN, for example for kiwipete in chess.
    /// Can be implemented by a concrete `Board`, which will make `from_name` recognize the name and lets the
    /// GUI know about supported positions.
    /// "startpos" is handled automatically in `from_name` but can be overwritten here.
    fn name_to_fen_map() -> Vec<(String, String)> {
        Vec::new()
    }

    fn bench_positions() -> Vec<Self> {
        let named_positions = Self::name_to_fen_map();
        named_positions
            .iter()
            .map(|(_name, fen)| Self::from_fen(fen).unwrap())
            .collect_vec()
    }

    fn settings(&self) -> Self::Settings;

    /// The player who can now move.
    fn active_player(&self) -> Color;

    /// The number of moves (turns) since the start of the game.
    fn fullmove_ctr(&self) -> usize {
        self.halfmove_ctr_since_start() / 2
    }

    /// The number of half moves (plies) since the start of the game.
    fn halfmove_ctr_since_start(&self) -> usize;

    /// An upper bound on the number of past plies that need to be considered for repetitions.
    /// This can be the same as `halfmove_ctr_since_start` or always zero if repetitions aren't possible.   
    fn halfmove_repetition_clock(&self) -> usize;

    /// The size of the board expressed as coordinates.
    /// The value returned from this function does not correspond to a valid square.
    fn size(&self) -> <Self::Coordinates as Coordinates>::Size;

    /// The number of squares of the board.
    fn num_squares(&self) -> usize {
        self.size().num_squares()
    }

    /// Converts coordinates into an internal index.
    fn to_idx(&self, pos: Self::Coordinates) -> usize {
        self.size().to_idx(pos)
    }

    /// Converts an index into coordinates, the reveres of `to_idx`
    fn to_coordinates(&self, idx: usize) -> Self::Coordinates {
        self.size().to_coordinates(idx)
    }

    /// Returns the piece at the given coordinates.
    /// Should return the same as `piece_on_idx(self.to_idx(pos))`.
    fn piece_on(&self, pos: Self::Coordinates) -> Self::Piece {
        self.piece_on_idx(self.to_idx(pos))
    }

    /// Returns the piece at the given index.
    fn piece_on_idx(&self, pos: usize) -> Self::Piece;

    fn are_all_pseudolegal_legal() -> bool {
        false
    }

    /// Returns a list of pseudo legal moves, that is, moves which can either be played using
    /// `make_move` or which will cause `make_move` to return `None`.
    fn pseudolegal_moves(&self) -> Self::MoveList;

    /// Returns a list of pseudo legal moves that are considered "noisy", such as captures and promotions in chess.
    fn noisy_pseudolegal(&self) -> Self::MoveList;

    /// Returns a random legal move, that is, chooses a pseudorandom move from the set of legal moves.
    /// Can be implemented by generating all legal moves and randomly sampling one, so it's potentially
    /// a very inefficient function, random_pseudolegal_move should be prefered if possible
    fn random_legal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move>;

    /// Returns a random pseudolegal move
    fn random_pseudolegal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move>;

    /// Assumes pseudolegal movegen, returns None in case of an illegal pseudolegal move,
    /// like ignoring a check in chess. Not meant to return None on moves that never make sense,
    /// like moving to a square outside of the board (in that case, the function should panic).
    /// In other words, this function only gracefully checks legality assuming that the move is pseudolegal.
    fn make_move(self, mov: Self::Move) -> Option<Self>;

    /// Makes a nullmove, i.e. flips the active player. While this action isn't strictly legal in most games,
    /// it's still very useful and necessary for null move pruning.
    /// Just like make_move, this function may fail, such as when trying to do a nullmove while in check.
    fn make_nullmove(self) -> Option<Self>;

    /// Returns true iff the move is pseudolegal, that is, it can be played with `make_move` without
    /// causing a panic.
    fn is_move_pseudolegal(&self, mov: Self::Move) -> bool;

    /// Returns true iff the move is legal, that is, if it is pseudolegal and playing it with `make_move`
    /// would return Some result. `is_move_pseudolegal` can be much faster.
    fn is_move_legal(&self, mov: Self::Move) -> bool {
        self.is_move_pseudolegal(mov)
            && (Self::are_all_pseudolegal_legal() || self.is_pseudolegal_move_legal(mov))
    }

    /// Expects a pseudolegal move and returns if this move is also legal, which means that playing it with
    /// `make_move` returns `Some(new_board)`
    fn is_pseudolegal_move_legal(&self, mov: Self::Move) -> bool {
        self.make_move(mov).is_some()
    }

    fn game_result_no_movegen(&self) -> Option<PlayerResult>;

    /// Returns the result (win/draw/loss), if any. Can be potentially slow because it can require movegen.
    /// If movegen is used anyway (such as in an ab search), it is usually better to call `game_result_no_movegen`
    /// and `no_moves_result` iff there were no legal moves.
    /// Despite the name, this method is not always slower than `game_result_no_movegen`, for some games both
    /// implementations are identical. But in a generic setting, this shouldn't be relied upon, hence the name.
    /// Note that many implementations never return `PlayerResult::Win` because the active player can't win the game,
    /// which is the case because the current player is flipped after the winning move.
    /// For example, being checkmated in chess is a loss for the current player.
    fn game_result_slow(&self) -> Option<PlayerResult>;

    /// Only called when there are no legal moves.
    /// In that case, the function returns the game state from the current player's perspective.
    /// Note that this doesn't check that there are indeed no legal moves to avoid paying the performance cost of that.
    fn no_moves_result(&self) -> PlayerResult;

    /// Returns true iff the game is lost for player who can now move, like being checkmated in chess.
    /// Using `game_result_no_movegen()` and `no_moves_result()` is often the faster option if movegen is needed anyway
    fn is_game_lost_slow(&self) -> bool {
        self.game_result_slow().is_some_and(|x| x == Lose)
    }

    /// Returns true iff the game is won for the current player after making the given move.
    /// This move has to be pseudolegal. If the move will likely be played anyway, it can be faster
    /// to play it and use `game_result()` or `game_result_no_movegen()` and `no_moves_result` instead.
    fn is_game_won_after_slow(&self, mov: Self::Move) -> bool {
        self.make_move(mov)
            .map_or(false, |new_pos| new_pos.is_game_lost_slow())
    }

    /// Returns true iff the game is a draw. This function covers all possibilities of a draw occurring,
    /// like stalemate, insufficient material, threefold repetition and 50 move rule in chess.
    /// Of course, explicitly testing for no legal moves is also possible in many games, and may be
    /// significantly faster, which is why the `is_draw_no_movegen` method exists.
    // fn is_draw(&self) -> bool;

    fn zobrist_hash(&self) -> ZobristHash;

    /// Returns a compact textual description of the board that can be read in again with `from_fen`.
    fn as_fen(&self) -> String;

    /// Reads in a compact textual description of the board, such that `B::from_fen(board.as_fen()) == b` holds.
    fn from_fen(mut string: &str) -> Result<Self, String> {
        let res = Self::read_fen_and_advance_input(&mut string)?;
        if !string.trim().is_empty() {
            return Err(format!(
                "Input contained additional characters after fen: {string}"
            ));
        }
        return Ok(res);
    }

    fn read_fen_and_advance_input(string: &mut &str) -> Result<Self, String>;

    /// Returns an ASCII art representation of the board.
    /// For chess, this is not meant to return a FEN, but instead a diagram where the pieces
    /// are identified by their letters in algebraic notation.
    fn as_ascii_diagram(&self) -> String;

    /// Returns an UTF-8 representation of the board.
    /// For chess, this is not meant to return a FEN, but instead a diagram where the pieces
    /// are identified by their unicode symbols.
    fn as_unicode_diagram(&self) -> String;

    /// Verifies that the position is legal. This function is meant to be used in `assert!`s
    /// and for validating input, such as FENs, not to be used for filtering positions after a call to `make_move`
    /// (it should  already be ensured that the move results in a legal position or `None` through other means).
    fn verify_position_legal(&self) -> Result<(), String>;
}

pub fn game_result_slow<B: Board, H: BoardHistory<B>>(
    board: &B,
    history: &H,
) -> Option<PlayerResult> {
    board
        .game_result_slow()
        .or_else(|| history.game_result(&board))
}

pub fn game_result_no_movegen<B: Board, H: BoardHistory<B>>(
    board: &B,
    history: &H,
) -> Option<PlayerResult> {
    board
        .game_result_no_movegen()
        .or_else(|| history.game_result(&board))
}

pub trait RectangularBoard: Board {
    fn height(&self) -> usize;

    fn width(&self) -> usize;
}

impl<T: Board> RectangularBoard for T
where
    T::Coordinates: RectangularCoordinates,
    <T::Coordinates as Coordinates>::Size: RectangularSize<T::Coordinates>,
{
    fn height(&self) -> usize {
        self.size().height().0
    }
    fn width(&self) -> usize {
        self.size().width().0
    }
}

pub fn position_fen_part<T: RectangularBoard>(pos: &T) -> String
where
    T::Coordinates: RectangularCoordinates,
{
    let mut res: String = Default::default();
    for y in (0..pos.height()).rev() {
        let mut empty_ctr = 0;
        for x in 0..pos.width() {
            let piece = pos.piece_on(T::Coordinates::from_row_column(y, x));
            if piece.is_empty() {
                empty_ctr += 1;
            } else {
                if empty_ctr > 0 {
                    res += &empty_ctr.to_string();
                }
                empty_ctr = 0;
                res.push(piece.to_ascii_char());
            }
        }
        if empty_ctr > 0 {
            res += &empty_ctr.to_string();
        }
        if y > 0 {
            res.push('/');
        }
    }
    res
}

pub fn legal_moves_slow<T: Board>(pos: &T) -> T::LegalMoveList {
    let pseudo_legal = pos.pseudolegal_moves();
    if T::are_all_pseudolegal_legal() {
        return pseudo_legal.collect();
    }
    return pseudo_legal
        .filter(|mov| pos.is_pseudolegal_move_legal(*mov))
        .collect();
}

fn board_to_string<B: RectangularBoard, F: Fn(B::Piece) -> char>(
    pos: &B,
    piece_to_char: F,
) -> String {
    Iterator::intersperse(
        (0..pos.num_squares())
            .map(|i| piece_to_char(pos.piece_on_idx(i)))
            .collect::<Vec<_>>()
            .chunks(pos.width())
            .rev(),
        &['\n'],
    )
    .flatten()
    .collect::<String>()
        + "\n"
}

fn read_position_fen<B: RectangularBoard, F>(
    position: &str,
    mut board: B,
    place_piece: F,
) -> Result<B, String>
where
    F: Fn(B, B::Coordinates, <B::Piece as ColoredPiece>::ColoredPieceType) -> Result<B, String>,
{
    let lines = position.split('/');
    debug_assert!(lines.clone().count() > 0);

    let mut square = 0;
    for (line, line_num) in lines.zip(0..) {
        let mut skipped_digits = 0;
        let square_before_line = square;
        debug_assert_eq!(square_before_line, line_num * board.width());

        let handle_skipped = |digit_in_line, skipped_digits, idx: &mut usize| {
            if skipped_digits > 0 {
                let num_skipped = line[digit_in_line - skipped_digits..digit_in_line]
                    .parse::<usize>()
                    .unwrap();
                if num_skipped == 0 {
                    return Err("fen position can't contain the number 0".to_string());
                }
                *idx += num_skipped;
            }
            return Ok(());
        };

        for (i, c) in line.char_indices() {
            if c.is_numeric() {
                skipped_digits += 1;
                continue;
            }
            let symbol = <B::Piece as ColoredPiece>::ColoredPieceType::from_ascii_char(c)
                .ok_or_else(|| format!("Invalid character: {c}"))?;
            handle_skipped(i, skipped_digits, &mut square)?;
            skipped_digits = 0;
            if square >= board.num_squares() {
                return Err(format!("fen position contains at least {square} squares, but the board only has {0} squares", board.num_squares()));
            }

            // let player = symbol.color().ok_or_else(|| "Invalid format: Empty square can't appear as part of nnk fen (should be number of consecutive empty squares) ".to_string())?;
            board = place_piece(
                board,
                board.to_coordinates(square).flip_up_down(board.size()),
                symbol,
            )?;
            square += 1;
        }
        handle_skipped(line.len(), skipped_digits, &mut square)?;
        let line_len = square - square_before_line;
        if line_len != board.width() {
            return Err(format!(
                "Line '{line}' has incorrect width: {line_len}, should be {0}",
                board.width()
            ));
        }
    }
    Ok(board)
}
// impl<B: Board> Display for B {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         write!(f, "{0}", self.as_utf8_diagram())
//     }
// }
