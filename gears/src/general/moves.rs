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

use crate::general::board::Board;
use crate::general::common::Res;
use num::PrimInt;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;

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

    /// Compact text representation is used by UGI, e.g. for chess it's `<to><from><promo_piece_if_present>`.
    fn format_compact(self, f: &mut Formatter<'_>) -> fmt::Result;

    /// Parse a compact text representation emitted by `to_compact_text`, such as the one used by UCI
    fn from_compact_text(s: &str, board: &B) -> Res<B::Move>;

    /// Returns a longer representation of the move that may require the board, such as long algebraic notation
    fn format_extended(self, f: &mut Formatter<'_>, _board: &B) -> fmt::Result {
        self.format_compact(f)
    }

    /// Returns a formatter object that implements `Display` such that it prints the result of `to_extended_text`.
    fn extended_formatter(self, pos: B) -> ExtendedFormatter<B, Self> {
        ExtendedFormatter { pos, mov: self }
    }

    /// A convenience method based on `format_extended` that returns a `String`.
    fn to_extended_text(self, board: &B) -> String {
        self.extended_formatter(*board).to_string()
    }

    /// Parse a longer text representation emitted by `format_extended`, such as long algebraic notation.
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

#[derive(Debug, Copy, Clone)]
pub struct ExtendedFormatter<B: Board, M: Move<B>> {
    pos: B,
    mov: M,
}

impl<B: Board, M: Move<B>> Display for ExtendedFormatter<B, M> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.mov.format_extended(f, &self.pos)
    }
}
