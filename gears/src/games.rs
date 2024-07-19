use colored::Colorize;
use std::cmp::min;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;
use std::str::{FromStr, SplitWhitespace};

use derive_more::BitXorAssign;
use itertools::Itertools;
use num::PrimInt;
use rand::Rng;
use strum_macros::EnumIter;

use crate::games::PlayerResult::*;
use crate::general::common::Description::NoDescription;
use crate::general::common::{
    parse_int, select_name_static, EntityList, GenericSelect, IterIntersperse, Res,
    StaticallyNamedEntity,
};
use crate::general::move_list::MoveList;
use crate::general::squares::{RectangularCoordinates, RectangularSize};
use crate::output::OutputBuilder;
use crate::search::Depth;
use crate::{player_res_to_match_res, GameOver, GameOverReason, MatchResult, PlayerResult};

#[cfg(feature = "mnk")]
pub mod mnk;

#[cfg(feature = "ataxx")]
pub mod ataxx;
#[cfg(feature = "chess")]
pub mod chess;
#[cfg(test)]
mod generic_tests;

/// White is always the first player, Black is always the second. TODO: Change naming to redlect this.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Hash, EnumIter)]
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

    /// For chess, uncolored piece symbols are different from both white and black piece symbols, but
    /// used very rarely (and kind of ugly). So this maps to the much more common black piece version,
    /// which is useful for text-based outputs that color the pieces themselves.
    fn to_default_utf8_char(self) -> char {
        self.to_utf8_char()
    }

    fn from_ascii_char(c: char) -> Option<Self> {
        Self::from_utf8_char(c)
    }

    /// `from_utf8_char` should accept a (not necessarily strict) superset of `from_ascii_char`
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

pub fn file_to_char(file: DimT) -> char {
    debug_assert!(file < 26);
    (file + b'a') as char
}

pub fn char_to_file(file: char) -> DimT {
    debug_assert!(file >= 'a');
    debug_assert!(file <= 'z');
    file as DimT - b'a'
}

// Assume 2D grid for now.
pub trait Coordinates: Eq + Copy + Debug + Default + FromStr<Err = String> + Display {
    type Size: Size<Self>;

    /// mirrors the coordinates vertically
    fn flip_up_down(self, size: Self::Size) -> Self;

    /// mirrors the coordinates horizontally
    fn flip_left_right(self, size: Self::Size) -> Self;

    fn no_coordinates() -> Self;
}

pub type DimT = u8;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Default)]
pub struct Height(pub DimT);

impl Height {
    pub fn get(self) -> DimT {
        self.0
    }
    pub fn val(self) -> usize {
        self.0 as usize
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Default)]
pub struct Width(pub DimT);

impl Width {
    pub fn get(self) -> DimT {
        self.0
    }
    pub fn val(self) -> usize {
        self.0 as usize
    }
}

pub trait Size<C: Coordinates>: Eq + PartialEq + Copy + Clone + Display + Debug {
    fn num_squares(self) -> usize;

    /// Converts coordinates into an internal key. This function is injective, but **no further guarantees** are
    /// given. In particular, returned value do not have to be 0-based and do not have to be consecutive.
    /// E.g. for Ataxx, this returns the index of embedding the ataxx board into a 8x8 board.
    fn to_internal_key(self, coordinates: C) -> usize;

    /// Converts an internal key into coordinates, the inverse of `to_internal_key`.
    /// No further assumptions about which keys are valid should be made; in particular, there may be gaps in the set
    /// of valid keys (e.g. 4 and 12 might be valid, but 10 might not be). Although this function is safe in the rust
    /// sense, it doesn't guarantee any specified behavior for invalid keys.
    fn to_coordinates_unchecked(self, internal_key: usize) -> C;

    fn valid_coordinates(self) -> impl Iterator<Item = C>;

    fn coordinates_valid(self, coordinates: C) -> bool;

    fn check_coordinates(self, coordinates: C) -> Res<C> {
        match self.coordinates_valid(coordinates) {
            true => Ok(coordinates),
            false => Err(format!(
                "Coordinates {coordinates} lie outside of the board (size {self})"
            )),
        }
    }
}

pub trait MoveFlags: Eq + Copy + Debug + Default {}

#[derive(Eq, PartialEq, Debug, Copy, Clone, Default)]
pub struct NoMoveFlags {}

impl MoveFlags for NoMoveFlags {}

pub trait Move<B: Board>: Eq + Copy + Clone + Debug + Default + Display + Hash + Send {
    type Flags: MoveFlags;

    type Underlying: PrimInt + Into<usize>;

    /// From which square does the piece move?
    /// When this doesn't make sense, such as for m,n,k games, return some default value, such as `no_coordinates()`
    fn src_square(self) -> B::Coordinates;

    /// To which square does the piece move / get placed.
    fn dest_square(self) -> B::Coordinates;

    /// Move flags. Not all Move implementations have them, in which case `Flags` can be `NoMoveFlags`
    fn flags(self) -> Self::Flags;

    /// Tactical moves can drastically change the position and are often searched first, such as captures and queen or
    /// knight promotions in chess. Always returning `false` is a valid choice.
    fn is_tactical(self, board: &B) -> bool;

    /// Return a compact and easy to parse move representation, such as <from_square><to_square> as used by UCI
    fn to_compact_text(self) -> String;

    /// Parse a compact text representation emitted by `to_compact_text`, such as the one used by UCI
    fn from_compact_text(s: &str, board: &B) -> Res<B::Move>;

    /// Returns a longer representation of the move that may require the board, such as long algebraic notation
    fn to_extended_text(self, _board: &B) -> String {
        self.to_compact_text()
    }

    /// Parse a longer text representation emitted by `to_extended_text`, such as long algebraic notation.
    /// May optionally also parse additional notation, such as short algebraic notation.
    fn from_extended_text(s: &str, board: &B) -> Res<B::Move>;

    /// Parse a text representation of the move. This may be the same as `from_compact_text`
    /// or may use a different notation, such as standard algebraic notation in chess.
    /// This is supposed to be used whenever the move format is unknown, such as when the user enters a move, and therefore
    /// should handle as many different cases as possible, but always needs to handle the compact text representation.
    /// This function does not ensure that the move is actually pseudolegal in the current position.
    fn from_text(s: &str, board: &B) -> Res<B::Move> {
        match B::Move::from_extended_text(s, board) {
            Ok(m) => Ok(m),
            Err(e) => {
                if let Ok(m) = B::Move::from_compact_text(s, board) {
                    if board.is_move_pseudolegal(m) {
                        return Ok(m);
                    }
                }
                Err(e)
            }
        }
    }

    fn from_usize_unchecked(val: usize) -> Self;

    fn to_underlying(self) -> Self::Underlying;
}

pub type OutputList<B> = EntityList<Box<dyn OutputBuilder<B>>>;

#[derive(Copy, Clone, Eq, PartialEq, Default, Debug, derive_more::Display, BitXorAssign)]
pub struct ZobristHash(pub u64);

pub trait Settings: Eq + Copy + Debug + Default {}

pub trait BoardHistory<B: Board>: Default + Debug + Clone + 'static {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn is_repetition(&self, board: &B, plies_ago: usize) -> bool;
    fn push(&mut self, board: &B);
    fn pop(&mut self);
    fn clear(&mut self);
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct NoHistory {}

impl<B: Board> BoardHistory<B> for NoHistory {
    fn len(&self) -> usize {
        0
    }

    fn is_repetition(&self, _board: &B, _plies_ago: usize) -> bool {
        false
    }

    fn push(&mut self, _board: &B) {}

    fn pop(&mut self) {}

    fn clear(&mut self) {}
}

#[derive(Clone, Eq, PartialEq, Default, Debug)]
pub struct ZobristHistory<B: Board>(pub Vec<ZobristHash>, PhantomData<B>);

impl<B: Board> BoardHistory<B> for ZobristHistory<B> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn is_repetition(&self, pos: &B, plies_ago: usize) -> bool {
        pos.zobrist_hash() == self.0[self.0.len() - plies_ago]
    }

    fn push(&mut self, pos: &B) {
        self.0.push(pos.zobrist_hash());
    }

    fn pop(&mut self) {
        self.0
            .pop()
            .expect("ZobristHistory::pop() called on empty history");
    }
    fn clear(&mut self) {
        self.0.clear()
    }
}

/// Compares the actual board states as opposed to only comparing the hashes. This still isn't always entirely correct --
/// For example, the FIDE rule state that the set of legal moves must be identical, which is not the case
/// if the ep square is set but the pawn is pinned and can't actually take.
#[derive(Debug, Default, Clone)]
pub struct BoardCopyHistory<B: Board>(Vec<B>);

impl<B: Board> BoardHistory<B> for BoardCopyHistory<B> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn is_repetition(&self, board: &B, plies_ago: usize) -> bool {
        self.0[self.len() - plies_ago] == *board
    }

    fn push(&mut self, board: &B) {
        self.0.push(*board)
    }

    fn pop(&mut self) {
        self.0.pop();
    }

    fn clear(&mut self) {
        self.0.clear()
    }
}

pub fn n_fold_repetition<B: Board, H: BoardHistory<B>>(
    mut count: usize,
    history: &H,
    pos: &B,
    max_lookback: usize,
) -> bool {
    let stop = min(history.len(), max_lookback);
    if stop < 2 {
        // in many, but not all, games, we could increase this to 4
        return false;
    }
    for i in (2..=stop).step_by(2) {
        if history.is_repetition(pos, i) {
            count -= 1;
            if count <= 1 {
                return true;
            }
        }
    }
    false
}

type NameToPos<B> = GenericSelect<fn() -> B>;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum SelfChecks {
    CheckFen,
    Assertion,
}

/// Currently, a game is completely determined by the `Board` type:
/// The type implementing `Board` contains all the necessary information about the rules of the game.
/// However, a `Board` is assumed to be markovian and needs to satisfy `Copy` and `'static`.
/// When this is not desired, the `GameState` should be used instead, it contains a copy of the current board
/// and additional non-markovian information, such as the history of zobrist hashes for games that need this.
pub trait Board:
    Eq
    + PartialEq
    + Sized
    + Default
    + Debug
    + Display
    + Copy
    + Clone
    + Send
    + StaticallyNamedEntity
    + 'static
{
    type Settings: Settings;
    type Coordinates: Coordinates;
    type Piece: ColoredPiece;
    type Move: Move<Self>;
    type MoveList: MoveList<Self>;
    type LegalMoveList: MoveList<Self> + FromIterator<Self::Move>;

    /// Returns the name of the game, such as 'chess'.
    fn game_name() -> String {
        Self::static_short_name().to_string()
    }

    /// The position returned by this function does not have to be legal, e.g. in chess it would
    /// not include any kings. However, this is still useful to set up the board and is used
    /// in fen parsing, for example.
    fn empty_possibly_invalid(_settings: Self::Settings) -> Self {
        Self::default()
    }

    /// The starting position of the game.
    /// For games with random starting position, this function picks one randomly.
    fn startpos(settings: Self::Settings) -> Self;

    /// Constructs a specific, well-known position from its name, such as 'kiwipete' in chess.
    /// Not to be confused with `from_fen`, which can load arbitrary positions.
    fn from_name(name: &str) -> Res<Self> {
        select_name_static(
            name,
            Self::name_to_pos_map().iter(),
            "position",
            &Self::game_name(),
            NoDescription,
        )
        .map(|f| (f.val)())
    }

    /// Returns a Vec mapping well-known position names to their FEN, for example for kiwipete in chess.
    /// Can be implemented by a concrete `Board`, which will make `from_name` recognize the name and lets the
    /// GUI know about supported positions.
    /// "startpos" is handled automatically in `from_name` but can be overwritten here.
    fn name_to_pos_map() -> EntityList<NameToPos<Self>> {
        vec![NameToPos {
            name: "startpos",
            val: || Self::startpos(Self::Settings::default()),
        }]
    }

    fn bench_positions() -> Vec<Self> {
        Self::name_to_pos_map()
            .iter()
            .map(|f| (f.val)())
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

    /// Returns `true` iff there is no piece on the given square.
    fn is_empty(&self, coords: Self::Coordinates) -> bool;

    /// Returns `true` iff a pice of the given type and color exists on the given coordinates.
    /// Can sometimes be implemented more efficiently than by comparing `colored_piece_on`.
    fn is_piece_on(
        &self,
        coords: Self::Coordinates,
        piece: <Self::Piece as ColoredPiece>::ColoredPieceType,
    ) -> bool {
        self.colored_piece_on(coords).colored_piece_type() == piece
    }

    /// Returns the piece at the given coordinates.
    /// `uncolored_piece_on` can sometimes be implemented more efficiently, e.g. for chess,
    /// but both methods can be relatively slow. For example, a chess move already stores the moving piece;
    /// getting it from the chess move is more efficient than getting it from the board.
    fn colored_piece_on(&self, coords: Self::Coordinates) -> Self::Piece;

    /// Returns the uncolored piece type at the given coordinates.
    /// Can sometimes be implemented more efficiently than `colored_piece_on`
    fn uncolored_piece_on(
        &self,
        coords: Self::Coordinates,
    ) -> <<Self::Piece as ColoredPiece>::ColoredPieceType as ColoredPieceType>::Uncolored {
        self.colored_piece_on(coords).uncolored()
    }

    /// Returns the default depth that should be used for perft if not otherwise specified.
    /// Takes in a reference to self because some boards have a size determined at runtime,
    /// and the default perft depth can change depending on that (or even depending on the current position)
    fn default_perft_depth(&self) -> Depth {
        Depth::new(5)
    }

    /// This function is used for optimizations and may safely return `false`.
    fn are_all_pseudolegal_legal() -> bool {
        false
    }

    /// Returns a list of pseudo legal moves, that is, moves which can either be played using
    /// `make_move` or which will cause `make_move` to return `None`.
    fn pseudolegal_moves(&self) -> Self::MoveList;

    /// Returns a list of pseudo legal moves that are considered "tactical", such as captures and promotions in chess.
    fn tactical_pseudolegal(&self) -> Self::MoveList;

    /// Returns a list of legal moves, that is, moves that can be played using `make_move`
    /// and will not return `None`. TODO: Add trait for efficient legal moves implementation.
    fn legal_moves_slow(&self) -> Self::LegalMoveList {
        let pseudo_legal = self.pseudolegal_moves();
        if Self::are_all_pseudolegal_legal() {
            return pseudo_legal.into_iter().collect();
        }
        pseudo_legal
            .into_iter()
            .filter(|mov| self.is_pseudolegal_move_legal(*mov))
            .collect()
    }

    /// Returns a random legal move, that is, chooses a pseudorandom move from the set of legal moves.
    /// Can be implemented by generating all legal moves and randomly sampling one, so it's potentially
    /// a very inefficient function, random_pseudolegal_move should be prefered if possible
    fn random_legal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move>;

    /// Returns a random pseudolegal move
    fn random_pseudolegal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move>;

    /// Assumes pseudolegal movegen, returns None in case of an illegal pseudolegal move,
    /// like ignoring a check in chess. Not meant to return None on moves that never make sense,
    /// like moving to a square outside the board (in that case, the function should panic).
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

    fn player_result_no_movegen<H: BoardHistory<Self>>(&self, history: &H) -> Option<PlayerResult>;

    /// Returns the result (win/draw/loss), if any. Can be potentially slow because it can require movegen.
    /// If movegen is used anyway (such as in an ab search), it is usually better to call `game_result_no_movegen`
    /// and `no_moves_result` iff there were no legal moves.
    /// Despite the name, this method is not always slower than `game_result_no_movegen`, for some games both
    /// implementations are identical. But in a generic setting, this shouldn't be relied upon, hence the name.
    /// Note that many implementations never return `PlayerResult::Win` because the active player can't win the game,
    /// which is the case because the current player is flipped after the winning move.
    /// For example, being checkmated in chess is a loss for the current player.
    fn player_result_slow<H: BoardHistory<Self>>(&self, history: &H) -> Option<PlayerResult>;

    fn match_result_slow<H: BoardHistory<Self>>(&self, history: &H) -> Option<MatchResult> {
        let player_res = self.player_result_slow(history)?;
        let game_over = GameOver {
            result: player_res,
            reason: GameOverReason::Normal,
        };
        Some(player_res_to_match_res(game_over, self.active_player()))
    }

    /// Only called when there are no legal moves.
    /// In that case, the function returns the game state from the current player's perspective.
    /// Note that this doesn't check that there are indeed no legal moves to avoid paying the performance cost of that.
    /// This assumes that having no legal moves available automatically ends the game. If it is legal to pass,
    /// the movegen should generate a passing move.
    fn no_moves_result(&self) -> PlayerResult;

    /// Returns true iff the game is lost for the player who can now move, like being checkmated in chess.
    /// Using `game_result_no_movegen()` and `no_moves_result()` is often the faster option if movegen is needed anyway
    fn is_game_lost_slow(&self) -> bool {
        self.player_result_slow(&NoHistory::default())
            .is_some_and(|x| x == Lose)
    }

    /// Returns true iff the game is won for the current player after making the given move.
    /// This move has to be pseudolegal. If the move will likely be played anyway, it can be faster
    /// to play it and use `game_result()` or `game_result_no_movegen()` and `no_moves_result` instead.
    fn is_game_won_after_slow(&self, mov: Self::Move) -> bool {
        self.make_move(mov)
            .map_or(false, |new_pos| new_pos.is_game_lost_slow())
    }

    /// Returns `false` if it detects that `player` can not win the game except if the opponent runs out of time
    /// or makes "very dumb" mistakes.
    ///
    /// This is intended to be a comparatively cheap function and does not perform any kind of search.
    /// Typical cases where this returns false include chess positions where we only have our king left
    /// but the opponent still possesses enough material to mate (otherwise, the game would have ended in a draw).
    /// The result of this function on a position where [`game_result_slow`] returns a `Some` is unspecified.
    /// This is an approximation; always returning `true` would be a valid implementation of this method.
    /// The implementation of this method for chess technically violates the FIDE rules (as does the insufficient material
    /// draw condition), but that shouldn't be a problem in practice -- this rule is only meant ot be applied in human games anyway,
    /// and the FIDE rules are effectively uncheckable.
    fn can_reasonably_win(&self, player: Color) -> bool;

    fn zobrist_hash(&self) -> ZobristHash;

    /// Returns a compact textual description of the board that can be read in again with `from_fen`.
    fn as_fen(&self) -> String;

    /// Reads in a compact textual description of the board, such that `B::from_fen(board.as_fen()) == b` holds.
    fn from_fen(string: &str) -> Res<Self> {
        let mut words = string.split_whitespace();
        let res = Self::read_fen_and_advance_input(&mut words)
            .map_err(|err| format!("Failed to parse FEN {}: {err}", string.bold()))?;
        if words.next().is_some() {
            return Err(format!(
                "Input contained additional characters after FEN: {string}"
            ));
        }
        Ok(res)
    }

    /// Like `from_fen`, but changes the `input` argument to contain the reining input instead of panicking when there's
    /// any remaining input after reading the fen.
    fn read_fen_and_advance_input(input: &mut SplitWhitespace) -> Res<Self>;

    /// Returns an ASCII art representation of the board.
    /// This is not meant to return a FEN, but instead a diagram where the pieces
    /// are identified by their letters in algebraic notation.
    fn as_ascii_diagram(&self, flip: bool) -> String;

    /// Returns a UTF-8 representation of the board.
    /// This is not meant to return a FEN, but instead a diagram where the pieces
    /// are identified by their unicode symbols.
    fn as_unicode_diagram(&self, flip: bool) -> String;

    /// Verifies that the position is legal. This function is meant to be used in `assert!`s
    /// and for validating input, such as FENs, not to be used for filtering positions after a call to `make_move`
    /// (it should  already be ensured that the move results in a legal position or `None` through other means).
    /// If `checks` is `Assertion`, this performs internal validity checks, which is useful for asserting that there are no
    /// bugs in the implementation, but unnecessary if this function is only called to check the validity of a FEN.
    fn verify_position_legal(&self, checks: SelfChecks) -> Res<()>;
}

pub trait RectangularBoard: Board {
    fn height(&self) -> DimT;

    fn width(&self) -> DimT;

    fn idx_to_coordinates(&self, idx: DimT) -> Self::Coordinates;
}

impl<T: Board> RectangularBoard for T
where
    T::Coordinates: RectangularCoordinates,
    <T::Coordinates as Coordinates>::Size: RectangularSize<T::Coordinates>,
{
    fn height(&self) -> DimT {
        self.size().height().0
    }
    fn width(&self) -> DimT {
        self.size().width().0
    }

    fn idx_to_coordinates(&self, idx: DimT) -> Self::Coordinates {
        Self::Coordinates::from_row_column(idx / self.width(), idx % self.width())
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
            let piece = pos.colored_piece_on(T::Coordinates::from_row_column(y, x));
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

fn board_to_string<B: RectangularBoard, F: Fn(B::Piece) -> char>(
    pos: &B,
    piece_to_char: F,
    flip: bool,
) -> String {
    use std::fmt::Write;
    let mut squares = pos
        .size()
        .valid_coordinates()
        .map(|c| piece_to_char(pos.colored_piece_on(c)))
        .intersperse_(' ')
        .collect_vec();
    squares.push(' ');
    let mut rows = squares
        .chunks(pos.width() as usize * 2)
        .zip((1..).map(|x| x.to_string()))
        .map(|(row, nr)| format!("{} {nr}\n", row.iter().collect::<String>()))
        .collect_vec();
    if !flip {
        rows.reverse();
    }
    rows.push(
        ('A'..)
            .take(pos.width() as usize)
            .fold(String::default(), |mut s, c| -> String {
                write!(s, "{c} ").unwrap();
                s
            }),
    );
    rows.iter().flat_map(|x| x.chars()).collect::<String>() + "\n"
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
        debug_assert_eq!(
            square_before_line,
            line_num as usize * board.width() as usize
        );

        let handle_skipped = |digit_in_line, skipped_digits, idx: &mut usize| {
            if skipped_digits > 0 {
                let num_skipped = line[digit_in_line - skipped_digits..digit_in_line]
                    .parse::<usize>()
                    .unwrap();
                if num_skipped == 0 {
                    return Err("FEN position can't contain the number 0".to_string());
                }
                *idx += num_skipped;
            }
            Ok(())
        };

        for (i, c) in line.char_indices() {
            if c.is_numeric() {
                skipped_digits += 1;
                continue;
            }
            let symbol = <B::Piece as ColoredPiece>::ColoredPieceType::from_ascii_char(c)
                .ok_or_else(|| {
                    format!(
                        "Invalid character in {0} FEN position description (not a piece): {1}",
                        B::game_name(),
                        c.to_string().red()
                    )
                })?;
            handle_skipped(i, skipped_digits, &mut square)?;
            skipped_digits = 0;
            if square >= board.num_squares() {
                return Err(format!("FEN position contains at least {square} squares, but the board only has {0} squares", board.num_squares()));
            }

            // let player = symbol.color().ok_or_else(|| "Invalid format: Empty square can't appear as part of nnk fen (should be number of consecutive empty squares) ".to_string())?;
            board = place_piece(
                board,
                board
                    .idx_to_coordinates(square as DimT)
                    .flip_up_down(board.size()),
                symbol,
            )?;
            square += 1;
        }
        handle_skipped(line.len(), skipped_digits, &mut square)?;
        let line_len = square - square_before_line;
        if line_len != board.width() as usize {
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

#[cfg(test)]
mod tests {
    use crate::games::ataxx::AtaxxBoard;
    use crate::games::chess::Chessboard;
    use crate::games::generic_tests::GenericTests;
    use crate::games::mnk::MNKBoard;

    #[cfg(feature = "chess")]
    #[test]
    fn generic_chess_test() {
        GenericTests::<Chessboard>::all_tests()
    }

    #[cfg(feature = "mnk")]
    #[test]
    fn generic_mnk_test() {
        GenericTests::<MNKBoard>::all_tests()
    }

    #[cfg(feature = "ataxx")]
    #[test]
    fn generic_ataxx_test() {
        GenericTests::<AtaxxBoard>::all_tests()
    }
}
