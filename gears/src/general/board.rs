/*
 *  Gears, a collection of board games.
 *  Copyright (C) 2024 ToTheAnd
 *
 *  Gears is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Gears is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Gears. If not, see <https://www.gnu.org/licenses/>.
 */

use crate::games::{
    AbstractPieceType, BoardHistory, Color, ColoredPiece, ColoredPieceType, Coordinates, DimT,
    NoHistory, Settings, Size, ZobristHash,
};
use crate::general::board::SelfChecks::{Assertion, Verify};
use crate::general::common::Description::NoDescription;
use crate::general::common::{
    select_name_static, EntityList, GenericSelect, IterIntersperse, Res, StaticallyNamedEntity,
};
use crate::general::move_list::MoveList;
use crate::general::moves::Legality::Legal;
use crate::general::moves::Move;
use crate::general::squares::{RectangularCoordinates, RectangularSize};
use crate::search::Depth;
use crate::PlayerResult::Lose;
use crate::{player_res_to_match_res, GameOver, GameOverReason, MatchResult, PlayerResult};
use colored::Colorize;
use itertools::Itertools;
use rand::Rng;
use std::fmt::{Debug, Display};
use std::str::SplitWhitespace;

pub(crate) type NameToPos<B> = GenericSelect<fn() -> B>;

// Enum variants are listed in order; later checks include earlier checks.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum SelfChecks {
    CheckFen,
    Verify,
    Assertion,
}

pub trait UnverifiedBoard<B: Board>: Debug + Copy + Clone + From<B>
where
    B: Board<Unverified = Self>,
{
    fn new(board: B) -> Self {
        Self::from(board)
    }

    /// Same as `verify_with_level(Verify) for release builds
    /// and `verify_with_level(Assertion) in debug builds
    fn verify(self) -> Res<B> {
        if cfg!(debug_assertions) {
            self.verify_with_level(Assertion)
        } else {
            self.verify_with_level(Verify)
        }
    }

    /// Verifies that the position is legal. This function is meant to be used in `assert!`s,
    /// for validating input, such as FENs, and for allowing a user to programmatically set up custom positions and then
    /// verify that they are legal, not to be used for filtering positions after a call to `make_move`
    /// (it should already be ensured through other means that the move results in a legal position or `None`).
    /// If `checks` is `Assertion`, this performs internal validity checks, which is useful for asserting that there are no
    /// bugs in the implementation, but unnecessary if this function is only called to check the validity of a FEN.
    /// `CheckFen` sometimes needs to do less work than `Verify`.
    fn verify_with_level(self, level: SelfChecks) -> Res<B>;

    // TODO: Refactor such that debug_verify_invariants actually checks invariants that should not be broken in a Board
    // but are getting corrected in the verify method of an UnverifiedBoard

    /// Returns the size of the board.
    fn size(&self) -> BoardSize<B>;

    /// Checks if the given coordinates are valid.
    fn check_coordinates(&self, coords: B::Coordinates) -> Res<B::Coordinates> {
        self.size().check_coordinates(coords)
    }

    /// Place a piece of the given type and color on the given square. Like all functions that return an `UnverifiedBoard`,
    /// this doesn't check that the resulting position is legal.
    /// However, this function can still fail if the piece can't be placed because the coordinates.
    /// If there is a piece already on the square, it is implementation-defined what will happen; possible options include
    /// replacing the piece, returning an `Err`, or silently going into a bad state that will return an `Err` on `verify`.
    ///  Not intended to do any expensive checks.
    fn place_piece(self, piece: B::Piece) -> Res<Self> {
        let square = self.check_coordinates(piece.coordinates())?;
        // TODO: PieceType should not include the empty square; use a different, generic, struct for that
        Ok(self.place_piece_unchecked(square, piece.colored_piece_type()))
    }

    /// Like `place_piece`, but does not check that the coordinates are valid.
    fn place_piece_unchecked(self, coords: B::Coordinates, piece: ColPieceType<B>) -> Self;

    /// Remove the piece at the given coordinates. If there is no piece there, nothing happens.
    /// If the coordinates are invalid, an `Err` is returned.
    /// Some `UnverifiedBoard`s can represent multiple pieces at the same coordinates; it is implementation-defined
    /// what this method does in that case.
    fn remove_piece(self, coords: B::Coordinates) -> Res<Self> {
        let coords = self.check_coordinates(coords)?;
        if self.piece_on(coords).unwrap().is_empty() {
            Ok(self)
        } else {
            Ok(self.remove_piece_unchecked(coords))
        }
    }

    /// Like `remove_piece`, but does not check that the coordinates are valid.
    fn remove_piece_unchecked(self, coords: B::Coordinates) -> Self;

    /// Returns the piece on the given coordinates, or `None` if the coordinates aren't valid.
    /// Some `UnverifiedBoard`s can represent multiple pieces at the same coordinates; it is implementation-defined
    /// what this method does in that case (but it should never return empty coordinates in that case).
    fn piece_on(&self, coords: B::Coordinates) -> Res<B::Piece>;

    /// Set the active player. Like all of these functions, it does not guarantee or check that the resulting position
    /// is legal. For example, in chess, the side not to move might be in check, so that it would be possible to capture the king.
    fn set_active_player(self, player: B::Color) -> Self;

    /// Set the ply counter since the start of the game. Does not check that the resulting positions is legal, e.g. if
    /// the ply counter is larger than the number of placed pieces in games like m,n,k games or Ultimate Tic-Tac-Toe.
    /// Can fail if the ply number is not representable in the internal representation.
    fn set_ply_since_start(self, ply: usize) -> Res<Self>;

    // TODO: Also put more methods, like `as_fen`, in this trait?
    // Might be useful to print such boards, but the implementation might be annoying
}

// Rustc warns that the `Board` bounds are not enforced but removing them makes the program fail to compile
#[allow(type_alias_bounds)]
pub type ColPieceType<B: Board> = <B::Piece as ColoredPiece<B>>::ColoredPieceType;

#[allow(type_alias_bounds)]
pub type PieceType<B: Board> = <ColPieceType<B> as ColoredPieceType<B>>::Uncolored;

#[allow(type_alias_bounds)]
pub type BoardSize<B: Board> = <B::Coordinates as Coordinates>::Size;

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
    + Sync
    + StaticallyNamedEntity
    + 'static
{
    /// Should be either `Self::Unverified` or `Self`
    type EmptyRes: Into<Self::Unverified>;
    type Settings: Settings;
    type Coordinates: Coordinates;
    type Color: Color;
    type Piece: ColoredPiece<Self>;
    type Move: Move<Self>;
    type MoveList: MoveList<Self>;
    type LegalMoveList: MoveList<Self> + FromIterator<Self::Move>; // TODO: Remove use MoveList
    type Unverified: UnverifiedBoard<Self>;

    /// Returns the name of the game, such as 'chess'.
    #[must_use]
    fn game_name() -> String {
        Self::static_short_name().to_string()
    }

    /// The position returned by this function does not have to be legal, e.g. in chess it would
    /// not include any kings. However, this is still useful to set up the board and is used
    /// in fen parsing, for example.
    fn empty_for_settings(settings: Self::Settings) -> Self::EmptyRes;

    /// Like `empty_for_setting`, but uses a default settings objects.
    /// Most games have empty setting objects, so explicitly passing in settings is unnecessary.
    fn empty() -> Self::EmptyRes {
        Self::empty_for_settings(Self::Settings::default())
    }

    /// The starting position of the game.
    /// This always returns the same position, even when there are technically multiple starting positions.
    /// For example, the `Chessboard` implementation supports (D)FRC, but `startpos()` still only returns
    /// the standard chess start pos
    fn startpos_for_settings(settings: Self::Settings) -> Self;

    /// Like `startpos_for_settings()` with default settings.
    /// Most boards have empty settings, so explicitly passing in settings is unnecessary.
    /// Usually, `startpos()` returns the same as `default()`, but this isn't enforced.
    fn startpos() -> Self {
        Self::startpos_for_settings(Self::Settings::default())
    }

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
    #[must_use]
    fn name_to_pos_map() -> EntityList<NameToPos<Self>> {
        vec![NameToPos {
            name: "startpos",
            val: || Self::startpos_for_settings(Self::Settings::default()),
        }]
    }

    #[must_use]
    fn bench_positions() -> Vec<Self> {
        Self::name_to_pos_map()
            .iter()
            .map(|f| (f.val)())
            .collect_vec()
    }

    fn settings(&self) -> Self::Settings;

    /// The player who can now move.
    fn active_player(&self) -> Self::Color;

    fn inactive_player(&self) -> Self::Color {
        self.active_player().other()
    }

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
    fn is_piece_on(&self, coords: Self::Coordinates, piece: ColPieceType<Self>) -> bool {
        self.colored_piece_on(coords).colored_piece_type() == piece
    }

    /// Returns the piece at the given coordinates.
    /// `uncolored_piece_on` can sometimes be implemented more efficiently, e.g. for chess,
    /// but both methods can be relatively slow. For example, a chess move already stores the moving piece;
    /// getting it from the chess move is more efficient than getting it from the board.
    fn colored_piece_on(&self, coords: Self::Coordinates) -> Self::Piece;

    /// Returns the uncolored piece type at the given coordinates.
    /// Can sometimes be implemented more efficiently than `colored_piece_on`
    fn piece_type_on(&self, coords: Self::Coordinates) -> PieceType<Self> {
        self.colored_piece_on(coords).uncolored()
    }

    /// Returns the default depth that should be used for perft if not otherwise specified.
    /// Takes in a reference to self because some boards have a size determined at runtime,
    /// and the default perft depth can change depending on that (or even depending on the current position)
    fn default_perft_depth(&self) -> Depth {
        Depth::new(5)
    }

    /// Returns a list of pseudo legal moves, that is, moves which can either be played using
    /// `make_move` or which will cause `make_move` to return `None`.
    fn pseudolegal_moves(&self) -> Self::MoveList;

    /// Returns a list of pseudo legal moves that are considered "tactical", such as captures and promotions in chess.
    fn tactical_pseudolegal(&self) -> Self::MoveList;

    /// Returns a list of legal moves, that is, moves that can be played using `make_move`
    /// and will not return `None`.
    fn legal_moves_slow(&self) -> Self::LegalMoveList {
        let pseudo_legal = self.pseudolegal_moves();
        if Self::Move::legality() == Legal {
            return pseudo_legal.into_iter().collect();
        }
        pseudo_legal
            .into_iter()
            .filter(|mov| self.is_pseudolegal_move_legal(*mov))
            .collect()
    }

    /// Returns a random legal move, that is, chooses a pseudorandom move from the set of legal moves.
    /// Can be implemented by generating all legal moves and randomly sampling one, so it's potentially
    /// `random_pseudolegal_move`
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
    /// `make_move`
    fn make_nullmove(self) -> Option<Self>;

    /// Returns true iff the move is pseudolegal, that is, it can be played with `make_move` without
    /// causing a panic. When it is not certain that a move is definitely (pseudo)legal, `Untrusted<Move>`
    /// should be used.
    fn is_move_pseudolegal(&self, mov: Self::Move) -> bool;

    /// Returns true iff the move is legal, that is, if it is pseudolegal and playing it with `make_move`
    /// would return Some result. `is_move_pseudolegal` can be much faster.
    fn is_move_legal(&self, mov: Self::Move) -> bool {
        // the call to `is_pseudolegal_move_legal` should get inlined, after which it should evaluate to `true` for
        // boards with legal movegen
        self.is_move_pseudolegal(mov) && self.is_pseudolegal_move_legal(mov)
    }

    /// Expects a pseudolegal move and returns if this move is also legal, which means that playing it with
    /// `make_move` returns `Some(new_board)`
    fn is_pseudolegal_move_legal(&self, mov: Self::Move) -> bool {
        Self::Move::legality() == Legal || self.make_move(mov).is_some()
    }

    /// Returns the result (win/draw/loss), if any, but doesn't necessarily catch all game-ending conditions.
    /// That is, this function might return `None` if the game has actually ended,
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
    fn can_reasonably_win(&self, player: Self::Color) -> bool;

    fn zobrist_hash(&self) -> ZobristHash;

    /// Returns a compact textual description of the board that can be read in again with `from_fen`.
    fn as_fen(&self) -> String;

    /// Reads in a compact textual description of the board, such that `B::from_fen(board.as_fen()) == b` holds.
    /// Assumes that the entire string represents the FEN, without any trailing tokens.
    /// Use the lower-level `read_fen_and_advance_input` if this assumption doesn't have to hold.
    fn from_fen(string: &str) -> Res<Self> {
        let mut words = string.split_whitespace();
        let res = Self::read_fen_and_advance_input(&mut words)
            .map_err(|err| format!("Failed to parse FEN {}: {err}", string.bold()))?;
        if let Some(next) = words.next() {
            return Err(format!(
                "Input `{0}' contained additional characters after FEN, starting with '{1}'",
                string.bold(),
                next.red()
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

    /// Verifies that all invariants of this board are satisfied. It should never be possible for this function to
    /// fail for a bug-free program; failure most likely means the `Board` implementation is bugged.
    fn debug_verify_invariants(self) -> Res<Self> {
        Self::Unverified::new(self).verify_with_level(Assertion)
    }

    /// Place a piece of the given type and color on the given square. Doesn't check that the resulting position is
    /// legal (hence the `Unverified` return type), but can still fail if the piece can't be placed because e.g. there
    /// is already a piece on that square. See `[UnverifiedBoard::place_piece]`.
    fn place_piece(self, piece: Self::Piece) -> Res<Self::Unverified> {
        Self::Unverified::new(self).place_piece(piece)
    }

    /// Remove a piece from the given square. See `[UnverifiedBoard::remove_piece]`.
    fn remove_piece(self, square: Self::Coordinates) -> Res<Self::Unverified> {
        Self::Unverified::new(self).remove_piece(square)
    }

    /// Set the active player. See `[UnverifiedBoard::set_active_player]`.
    fn set_active_player(self, new_active: Self::Color) -> Self::Unverified {
        Self::Unverified::new(self).set_active_player(new_active)
    }

    /// Set the ply counter since the start of the game. See `[UnverifiedBoard::set_ply_since_start]`
    fn set_ply_since_start(self, ply: usize) -> Res<Self::Unverified> {
        Self::Unverified::new(self).set_ply_since_start(ply)
    }
}

pub trait RectangularBoard: Board<Coordinates: RectangularCoordinates> {
    fn height(&self) -> DimT;

    fn width(&self) -> DimT;

    fn idx_to_coordinates(&self, idx: DimT) -> Self::Coordinates;
}

impl<B: Board> RectangularBoard for B
where
    B::Coordinates: RectangularCoordinates,
{
    fn height(&self) -> DimT {
        self.size().height().0
    }
    fn width(&self) -> DimT {
        self.size().width().0
    }

    fn idx_to_coordinates(&self, idx: DimT) -> Self::Coordinates {
        self.size().idx_to_coordinates(idx)
    }
}

pub fn position_fen_part<B: RectangularBoard>(pos: &B) -> String {
    let mut res: String = String::default();
    for y in (0..pos.height()).rev() {
        let mut empty_ctr = 0;
        for x in 0..pos.width() {
            let piece = pos.colored_piece_on(B::Coordinates::from_row_column(y, x));
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

pub fn common_fen_part<T: RectangularBoard>(pos: &T) -> String {
    let stm = pos.active_player();
    let halfmove_ctr = pos.halfmove_repetition_clock();
    format!("{} {stm} {halfmove_ctr}", position_fen_part(pos))
}

pub fn board_to_string<B: RectangularBoard, F: Fn(B::Piece) -> char>(
    pos: &B,
    piece_to_char: F,
    flip: bool,
) -> String {
    use std::fmt::Write;
    let mut squares = (0..pos.height())
        .cartesian_product(0..pos.width())
        .map(|(row, column)| {
            piece_to_char(pos.colored_piece_on(B::Coordinates::from_row_column(row, column)))
        })
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

pub(crate) fn read_position_fen<B: RectangularBoard>(
    position: &str,
    mut board: B::Unverified,
) -> Result<B::Unverified, String> {
    let lines = position.split('/');
    debug_assert!(lines.clone().count() > 0);

    let mut square = 0;
    for (line, line_num) in lines.zip(0_usize..) {
        let mut skipped_digits = 0;
        let square_before_line = square;
        debug_assert_eq!(square_before_line, line_num * board.size().width().val());

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
            let symbol = ColPieceType::<B>::from_ascii_char(c).ok_or_else(|| {
                format!(
                    "Invalid character in {0} FEN position description (not a piece): {1}",
                    B::game_name(),
                    c.to_string().red()
                )
            })?;
            handle_skipped(i, skipped_digits, &mut square)?;
            skipped_digits = 0;
            if square >= board.size().num_squares() {
                return Err(format!("FEN position contains at least {square} squares, but the board only has {0} squares", board.size().num_squares()));
            }

            board = board.place_piece_unchecked(
                board
                    .size()
                    .idx_to_coordinates(square as DimT)
                    .flip_up_down(board.size()),
                symbol,
            );
            square += 1;
        }
        handle_skipped(line.len(), skipped_digits, &mut square)?;
        let line_len = square - square_before_line;
        if line_len != board.size().width().val() {
            return Err(format!(
                "Line '{line}' has incorrect width: {line_len}, should be {0}",
                board.size().width().val()
            ));
        }
    }
    Ok(board)
}

pub(crate) fn read_common_fen_part<B: RectangularBoard>(
    words: &mut SplitWhitespace,
    board: B::Unverified,
) -> Result<B::Unverified, String> {
    let position_part = words
        .next()
        .ok_or_else(|| format!("Empty {0} FEN string", B::game_name()))?;
    let mut board = read_position_fen::<B>(position_part, board)?;

    let active = words.next().ok_or_else(|| {
        format!(
            "{0} FEN ends after the position description and doesn't include the active player",
            B::game_name()
        )
    })?;
    let correct_chars = [
        B::Color::first().ascii_color_char(),
        B::Color::second().ascii_color_char(),
    ];
    if active.chars().count() != 1 {
        return Err(format!(
            "Expected a single char to describe the active player ('{0}' or '{1}'), got '{2}'",
            correct_chars[0].to_string().bold(),
            correct_chars[1].to_string().bold(),
            active.red()
        ));
    }
    let active = B::Color::from_char(active.chars().next().unwrap()).ok_or_else(|| {
        format!(
            "Expected '{0}' or '{1}' for the color, not '{2}'",
            correct_chars[0].to_string().bold(),
            correct_chars[1].to_string().bold(),
            active.red()
        )
    })?;
    board = board.set_active_player(active);
    Ok(board)
}
