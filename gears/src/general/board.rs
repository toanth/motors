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
use crate::general::common::Description::NoDescription;
use crate::general::common::{
    select_name_static, EntityList, GenericSelect, IterIntersperse, Res, StaticallyNamedEntity,
};
use crate::general::move_list::MoveList;
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
    type Color: Color;
    type Piece: ColoredPiece<Self::Color>;
    type Move: Move<Self>;
    type MoveList: MoveList<Self>;
    type LegalMoveList: MoveList<Self> + FromIterator<Self::Move>;

    /// Returns the name of the game, such as 'chess'.
    #[must_use]
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
    #[must_use]
    fn name_to_pos_map() -> EntityList<NameToPos<Self>> {
        vec![NameToPos {
            name: "startpos",
            val: || Self::startpos(Self::Settings::default()),
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
        piece: <Self::Piece as ColoredPiece<Self::Color>>::ColoredPieceType,
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
    ) -> <<Self::Piece as ColoredPiece<Self::Color>>::ColoredPieceType as ColoredPieceType<
        Self::Color,
    >>::Uncolored {
        self.colored_piece_on(coords).uncolored()
    }

    /// Returns the default depth that should be used for perft if not otherwise specified.
    /// Takes in a reference to self because some boards have a size determined at runtime,
    /// and the default perft depth can change depending on that (or even depending on the current position)
    fn default_perft_depth(&self) -> Depth {
        Depth::new(5)
    }

    /// This function is used for optimizations and may safely return `false`.
    #[must_use]
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
    fn can_reasonably_win(&self, player: Self::Color) -> bool;

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
    let mut res: String = String::default();
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

pub fn board_to_string<B: RectangularBoard, F: Fn(B::Piece) -> char>(
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

pub(crate) fn read_position_fen<B: RectangularBoard, F>(
    position: &str,
    mut board: B,
    place_piece: F,
) -> Result<B, String>
where
    F: Fn(
        B,
        B::Coordinates,
        <B::Piece as ColoredPiece<B::Color>>::ColoredPieceType,
    ) -> Result<B, String>,
{
    let lines = position.split('/');
    debug_assert!(lines.clone().count() > 0);

    let mut square = 0;
    for (line, line_num) in lines.zip(0_usize..) {
        let mut skipped_digits = 0;
        let square_before_line = square;
        debug_assert_eq!(square_before_line, line_num * board.width() as usize);

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
            let symbol = <B::Piece as ColoredPiece<B::Color>>::ColoredPieceType::from_ascii_char(c)
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
