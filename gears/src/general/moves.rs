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
use anyhow::bail;
use arbitrary::Arbitrary;
use num::PrimInt;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;

/// Statically known properties of a move.
/// Many games don't have a distinction between legal and pseudolegal moves, so those moves are always `Legal`.
/// In some contexts, such as when loading a move from the TT, it's unknown whether this is actually a pseudolegal move
/// for the given position, which is why such a move is represented as a `Untrusted<Move>`.
/// Note that legality depends on the position and can't be statically enforced; incorrectly assuming (pseudo)legality
/// usually results in a panic when playing the move, although *there is no guarantee given; the behavior is unspecified*.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
pub enum Legality {
    PseudoLegal,
    Legal,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum ExtendedFormat {
    Standard,
    Alternative,
}

/// A `Move` implementation uniquely describes a (pseudolegal) move in a given position. It may not store not enough
/// information to reconstruct the move without the position.
/// All `Move` functions that take a `Board` parameter assume that the move is pseudolegal for the given board
/// unless otherwise noted. [`UntrustedMove`] should be used when it's not clear that a move is pseudolegal.
pub trait Move<B: Board>:
    Eq + Copy + Clone + Debug + Default + Hash + Send + Sync + for<'a> Arbitrary<'a>
where
    B: Board<Move = Self>,
{
    type Underlying: PrimInt + Into<u64>;

    fn is_null(self) -> bool {
        self == Self::default()
    }

    /// For games with legal movegen, this should return `Legal`, for games with pseudo-legal movegen this should return
    /// `PseudoLegal`. Note that legality depends on the move and the position, which means the result of this function
    /// is not a statically guaranteed property and instead a promise that depends on correct usage.
    /// If pseudolegality can't be expected, [`UntrustedMove`] should be used to wrap the move.
    fn legality() -> Legality;

    /// From which square does the piece move?
    /// When this doesn't make sense, such as for m,n,k games, return some default value, such as `no_coordinates()`
    fn src_square_in(self, pos: &B) -> Option<B::Coordinates>;

    /// To which square does the piece move / get placed.
    fn dest_square_in(self, pos: &B) -> B::Coordinates;

    /// Tactical moves can drastically change the position and are often searched first, such as captures and queen or
    /// knight promotions in chess. Always returning `false` is a valid choice.
    fn is_tactical(self, board: &B) -> bool;

    /// Compact text representation is used by UGI, e.g. for chess it's `<to><from><promo_piece_if_present>`.
    /// Takes a [`Board`] parameter because some move types may not store enough information to be printed in a human-readable
    /// way without that.
    /// Similarly, the compact text representation may not store enough information to reconstruct a `Move`
    /// without using a `Board`.
    /// This method **must not panic** for illegal moves.
    /// Moves that don't need a board to be printed should implement the `Display` trait.
    fn format_compact(self, f: &mut Formatter<'_>, _board: &B) -> fmt::Result;

    /// Returns a longer representation of the move that may require the board, such as long algebraic notation
    /// Implementations of this trait *may* choose to ignore the board and to not require pseudolegality.
    fn format_extended(
        self,
        f: &mut Formatter<'_>,
        board: &B,
        _format: ExtendedFormat,
    ) -> fmt::Result {
        self.format_compact(f, board)
    }

    /// Returns a formatter object that implements `Display` such that it prints the result of `to_compact_text`.
    fn compact_formatter(self, pos: &B) -> CompactFormatter<B> {
        CompactFormatter { pos, mov: self }
    }

    /// Returns a formatter object that implements `Display` such that it prints the result of `to_extended_text`.
    /// Like [`self.format_extended`], an implementation *may* choose to not require pseudolegality.
    fn extended_formatter(self, pos: &B, format: ExtendedFormat) -> ExtendedFormatter<B> {
        ExtendedFormatter {
            pos,
            mov: self,
            format,
        }
    }

    /// A convenience method based on `format_extended` that returns a `String`.
    fn to_extended_text(self, board: &B, format: ExtendedFormat) -> String {
        self.extended_formatter(board, format).to_string()
    }

    /// Parse a compact text representation emitted by `to_compact_text`, such as the one used by UCI.
    /// Returns the remaining input.
    /// Needs to ensure that the move is at least pseudolegal.
    fn parse_compact_text<'a>(s: &'a str, board: &B) -> Res<(&'a str, B::Move)>;

    /// Parse a compact text representation emitted by `to_compact_text`, such as the one used by UCI.
    /// Returns an error unless the entire input has been consumed.
    /// Needs to ensure that the move is at least pseudolegal.
    fn from_compact_text(s: &str, board: &B) -> Res<B::Move> {
        let (remaining, parsed) = Self::parse_compact_text(s, board)?;
        if !remaining.is_empty() {
            bail!(
                "Additional input after move {0}: '{1}'",
                parsed.compact_formatter(board),
                remaining
            );
        }
        Ok(parsed)
    }

    /// Parse a longer text representation emitted by `format_extended`, such as long algebraic notation.
    /// May optionally also parse additional notation, such as short algebraic notation.
    /// Needs to ensure that the move is at least pseudolegal. Returns the remaining input.
    fn parse_extended_text<'a>(s: &'a str, board: &B) -> Res<(&'a str, B::Move)>;

    /// Parse a longer text representation emitted by `format_extended`, such as long algebraic notation.
    /// May optionally also parse additional notation, such as short algebraic notation.
    /// Needs to ensure that the move is at least pseudolegal.
    /// Returns an error unless the entire input has been consumed.
    fn from_extended_text(s: &str, board: &B) -> Res<B::Move> {
        let (remaining, parsed) = Self::parse_extended_text(s, board)?;
        if !remaining.is_empty() {
            bail!(
                "Additional input after move {0}: '{1}'",
                parsed.compact_formatter(board),
                remaining
            );
        }
        Ok(parsed)
    }

    /// Parse a text representation of the move. This may be the same as `from_compact_text`
    /// or may use a different notation, such as standard algebraic notation in chess.
    /// This is supposed to be used whenever the move format is unknown, such as when the user enters a move, and therefore
    /// should handle as many different cases as possible, but always needs to handle the compact text representation.
    /// Like all move parsing functions, this function needs to ensure that the move is pseudolegal in the current position.
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

    /// Load the move from its raw underlying integer representation, the inverse of `to_underlying`.
    /// Does not take a `Board` and therefore does not ensure pseudolegality.
    fn from_u64_unchecked(val: u64) -> UntrustedMove<B>;

    /// Serialize this move into an internal integer representation.
    /// Typically, this function behaves like a `transmute`, i.e.,
    /// it simply returns the internal representation as an appropriately-sized integer,
    /// but this is not a strict requirement.
    fn to_underlying(self) -> Self::Underlying;
}

#[derive(Debug, Copy, Clone)]
pub struct CompactFormatter<'a, B: Board> {
    pos: &'a B,
    mov: B::Move,
}

impl<'a, B: Board> Display for CompactFormatter<'a, B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.mov.format_compact(f, self.pos)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ExtendedFormatter<'a, B: Board> {
    pos: &'a B,
    mov: B::Move,
    format: ExtendedFormat,
}

impl<B: Board> Display for ExtendedFormatter<'_, B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.mov == B::Move::default() {
            write!(f, "0000")
        } else {
            self.mov.format_extended(f, &self.pos, self.format)
        }
    }
}

/// A wrapper type that statically denotes that the wrapped move is not trusted to be (pseudo)legal in the context
/// where it is expected to be used. For example, moves generated through normal movegen functions should always be at least
/// pseudolegal for the given position, but a move loaded from the TT may not be pseudolegal, which is why it's wrapped
/// in this struct.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
#[must_use]
#[repr(transparent)]
pub struct UntrustedMove<B: Board>(B::Move);

impl<B: Board> Display for UntrustedMove<B>
where
    B::Move: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        <B::Move as Display>::fmt(&self.0, f)
    }
}

impl<B: Board> UntrustedMove<B> {
    pub fn from_move(mov: B::Move) -> Self {
        Self(mov)
    }

    pub fn check_pseudolegal(self, pos: &B) -> Option<B::Move> {
        if pos.is_move_pseudolegal(self.0) {
            Some(self.0)
        } else {
            None
        }
    }

    pub fn check_legal(&self, pos: &B) -> Option<B::Move> {
        if pos.is_move_legal(self.0) {
            Some(self.0)
        } else {
            None
        }
    }

    pub fn trust_unchecked(self) -> B::Move {
        self.0
    }

    pub fn to_underlying(self) -> <B::Move as Move<B>>::Underlying {
        self.0.to_underlying()
    }
}
