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

pub mod uttt_square;
#[cfg(test)]
mod uttt_tests;

use crate::games::uttt::uttt_square::{UtttSize, UtttSquare};
use crate::games::uttt::ColoredUtttPieceType::{OStone, XStone};
use crate::games::uttt::UtttColor::{O, X};
use crate::games::uttt::UtttPieceType::{Empty, Occupied};
use crate::games::{
    AbstractPieceType, BoardHistory, Color, ColoredPiece, ColoredPieceType, Coordinates,
    GenericPiece, NoHistory, PieceType, Settings, ZobristHash,
};
use crate::general::bitboards::{
    Bitboard, DefaultBitboard, ExtendedRawBitboard, RawBitboard, RawStandardBitboard,
};
use crate::general::board::SelfChecks::*;
use crate::general::board::Strictness::Strict;
use crate::general::board::{
    board_from_name, common_fen_part, ply_counter_from_fullmove_nr, read_common_fen_part, Board,
    SelfChecks, Strictness, UnverifiedBoard,
};
use crate::general::common::{ith_one_u128, parse_int, Res, StaticallyNamedEntity, Tokens};
use crate::general::move_list::{EagerNonAllocMoveList, MoveList};
use crate::general::moves::Legality::Legal;
use crate::general::moves::{Legality, Move, NoMoveFlags, UntrustedMove};
use crate::general::squares::{
    RectangularCoordinates, SmallGridSize, SmallGridSquare, SquareColor,
};
use crate::output::text_output::{
    board_to_string, display_board_pretty, p1_color, p2_color, AdaptFormatter, BoardFormatter,
    DefaultBoardFormatter, PieceToChar,
};
use crate::PlayerResult;
use crate::PlayerResult::{Draw, Lose};
use anyhow::bail;
use arbitrary::Arbitrary;
use crossterm::style::Stylize;
use itertools::Itertools;
use rand::Rng;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::num::NonZeroUsize;
use std::ops::Not;
use std::str::FromStr;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, FromRepr};

/// `Bitboard`s have the  semantic constraint of storing squares in a row-major fashion.
/// That's why the public API of this struct only exposes `RawBitboard`s, which don't have this constraint,
/// and `SubBitboard`s, which adhere to that.
pub type RawUtttBitboard = ExtendedRawBitboard;

pub type UtttSubSize = SmallGridSize<3, 3>;

pub type UtttSubSquare = SmallGridSquare<3, 3, 3>;

pub type UtttSubBitboard = DefaultBitboard<RawStandardBitboard, UtttSubSquare>;

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone)]
pub struct UtttSettings {}

impl Settings for UtttSettings {}

#[derive(
    Debug, Default, Copy, Clone, Eq, PartialEq, Hash, derive_more::Display, EnumIter, Arbitrary,
)]
pub enum UtttColor {
    #[default]
    X = 0,
    O = 1,
}

impl Not for UtttColor {
    type Output = Self;

    fn not(self) -> Self::Output {
        self.other()
    }
}

impl Color for UtttColor {
    fn other(self) -> Self {
        match self {
            X => O,
            O => X,
        }
    }

    fn ascii_color_char(self) -> char {
        match self {
            X => 'x',
            O => 'o',
        }
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, EnumIter, FromRepr)]
pub enum UtttPieceType {
    #[default]
    Empty,
    Occupied,
}

impl Display for UtttPieceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_ascii_char())
    }
}

impl AbstractPieceType for UtttPieceType {
    fn empty() -> Self {
        Empty
    }

    fn to_ascii_char(self) -> char {
        match self {
            Empty => '.',
            Occupied => 'x',
        }
    }

    fn from_utf8_char(c: char) -> Option<Self> {
        match c {
            '.' => Some(Empty),
            'o' | 'x' => Some(Occupied),
            _ => None,
        }
    }

    fn to_uncolored_idx(self) -> usize {
        self as usize
    }
}

impl PieceType<UtttBoard> for UtttPieceType {
    type Colored = ColoredUtttPieceType;

    fn from_idx(idx: usize) -> Self {
        Self::from_repr(idx).unwrap()
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, FromRepr)]
pub enum ColoredUtttPieceType {
    XStone,
    OStone,
    #[default]
    Empty, // last so that `XStone` and `OStone` have indices 0 and 1
}

impl Display for ColoredUtttPieceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_ascii_char())
    }
}

const UNICODE_X: char = '⨉'; // '⨉',
const UNICODE_O: char = '◯'; // '○'

impl AbstractPieceType for ColoredUtttPieceType {
    fn empty() -> Self {
        Self::Empty
    }

    fn to_ascii_char(self) -> char {
        match self {
            ColoredUtttPieceType::Empty => '.',
            XStone => 'x',
            OStone => 'o',
        }
    }

    fn to_utf8_char(self) -> char {
        match self {
            ColoredUtttPieceType::Empty => '.',
            XStone => UNICODE_X,
            OStone => UNICODE_O,
        }
    }

    fn from_utf8_char(c: char) -> Option<Self> {
        match c {
            '.' => Some(ColoredUtttPieceType::Empty),
            'x' | 'X' | UNICODE_X => Some(XStone),
            'o' | 'O' | UNICODE_O => Some(OStone),
            _ => None,
        }
    }

    fn to_uncolored_idx(self) -> usize {
        match self {
            ColoredUtttPieceType::Empty => 0,
            _ => 1,
        }
    }
}

impl ColoredPieceType<UtttBoard> for ColoredUtttPieceType {
    type Uncolored = UtttPieceType;

    fn color(self) -> Option<UtttColor> {
        match self {
            ColoredUtttPieceType::Empty => None,
            XStone => Some(X),
            OStone => Some(O),
        }
    }

    fn to_colored_idx(self) -> usize {
        self as usize
    }

    fn new(color: UtttColor, uncolored: Self::Uncolored) -> Self {
        if uncolored == Empty {
            ColoredUtttPieceType::Empty
        } else {
            match color {
                X => XStone,
                O => OStone,
            }
        }
    }
}

pub type UtttPiece = GenericPiece<UtttBoard, ColoredUtttPieceType>;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub struct UtttMove(UtttSquare);

impl Default for UtttMove {
    fn default() -> Self {
        Self::NULL
    }
}

impl UtttMove {
    pub const fn new(square: UtttSquare) -> Self {
        Self(square)
    }
    pub const NULL: Self = Self::new(UtttSquare::no_coordinates_const());
}

impl Display for UtttMove {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.format_compact(f)
    }
}

impl Move<UtttBoard> for UtttMove {
    type Flags = NoMoveFlags;
    type Underlying = u8;

    fn legality() -> Legality {
        Legal
    }

    fn src_square(self) -> UtttSquare {
        UtttSquare::no_coordinates()
    }

    fn dest_square(self) -> UtttSquare {
        self.0
    }

    fn flags(self) -> Self::Flags {
        NoMoveFlags {}
    }

    fn is_tactical(self, _board: &UtttBoard) -> bool {
        // TODO: Consider moves that win a sub-board tactical?
        false
    }

    fn format_compact(self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self == Self::NULL {
            write!(f, "0000")
        } else {
            write!(f, "{}", self.dest_square())
        }
    }

    fn from_compact_text(s: &str, board: &UtttBoard) -> Res<Self> {
        // TODO: This is not pseudolegal, so allowing it seems dangerous
        if s == "0000" {
            return Ok(Self::NULL);
        }
        let square = UtttSquare::from_str(s)?;
        if !board.is_open(square) {
            bail!("Square {square} is not empty, so this move is invalid");
        } else if !board.is_move_pseudolegal(Self(square)) {
            bail!("Incorrect sub-board. The previous move determines the sub-board for this move")
        }
        Ok(Self(square))
    }

    fn from_extended_text(s: &str, board: &UtttBoard) -> Res<Self> {
        Self::from_compact_text(s, board)
    }

    fn from_usize_unchecked(val: usize) -> UntrustedMove<UtttBoard> {
        UntrustedMove::from_move(Self(UtttSquare::unchecked(val)))
    }

    fn to_underlying(self) -> Self::Underlying {
        self.0.to_u8()
    }
}

pub type UtttMoveList = EagerNonAllocMoveList<UtttBoard, 81>;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub struct UtttBoard {
    // contains not just the occupancy, but also won sub-boards in the higher bits.
    colors_internal: [RawUtttBitboard; 2],
    open: RawUtttBitboard,
    active: UtttColor,
    ply_since_start: usize,
    last_move: UtttMove,
}

impl Default for UtttBoard {
    fn default() -> Self {
        Self {
            colors_internal: [RawUtttBitboard::default(); 2],
            open: Self::board_bb(!RawUtttBitboard::default()),
            active: UtttColor::default(),
            ply_since_start: 0,
            last_move: UtttMove::NULL,
        }
    }
}

impl StaticallyNamedEntity for UtttBoard {
    fn static_short_name() -> impl Display
    where
        Self: Sized,
    {
        "UTTT"
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        "Ultimate Tic-Tac-Toe".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "Ultimate Tic-Tac-Toe is a challenging variant of Tic-Tac-Toe, where every square is itself a Tic-Tac-Toe board".to_string()
    }
}

// TODO: BitboardBoard trait with default implementations like empty_bb() = !occupied_bb(), active_player_bb(), etc
// (Need to make sure this works for bitboards where only some of the bits are used)
impl UtttBoard {
    const BOARD_BB: ExtendedRawBitboard = ExtendedRawBitboard(0x1_ffff_ffff_ffff_ffff_ffff);
    const SUB_BOARD_MASK: RawStandardBitboard = RawStandardBitboard(0x1ff);

    const NUM_SQUARES: usize = 81;

    fn board_bb(bb: ExtendedRawBitboard) -> RawUtttBitboard {
        bb & Self::BOARD_BB
    }

    pub fn occupied_bb(&self) -> ExtendedRawBitboard {
        Self::board_bb(self.colors_internal[0] | self.colors_internal[1])
    }

    /// Returns all empty squares, even those in closed sub boards
    pub fn all_empty_squares_bb(&self) -> RawUtttBitboard {
        Self::board_bb(!(self.colors_internal[0] | self.colors_internal[1]))
    }

    pub fn open_bb(&self) -> RawUtttBitboard {
        self.open
    }

    pub fn player_bb(&self, color: UtttColor) -> RawUtttBitboard {
        Self::board_bb(self.colors_internal[color as usize])
    }

    pub fn active_player_bb(&self) -> RawUtttBitboard {
        self.player_bb(self.active)
    }

    pub fn inactive_player_bb(&self) -> RawUtttBitboard {
        self.player_bb(!self.active)
    }

    pub fn won_sub_boards(&self, color: UtttColor) -> UtttSubBitboard {
        let bb = self.colors_internal[color as usize];
        UtttSubBitboard::new(RawStandardBitboard((bb >> Self::NUM_SQUARES).0 as u64))
    }

    fn get_sub_board(bb: RawUtttBitboard, sub_board: UtttSubSquare) -> UtttSubBitboard {
        let bb =
            RawStandardBitboard((bb >> (sub_board.bb_idx() * 9)).0 as u64) & Self::SUB_BOARD_MASK;
        UtttSubBitboard::new(bb)
    }

    pub fn sub_board(&self, color: UtttColor, sub_board: UtttSubSquare) -> UtttSubBitboard {
        Self::get_sub_board(self.colors_internal[color as usize], sub_board)
    }

    pub fn open_sub_board(&self, sub_board: UtttSubSquare) -> UtttSubBitboard {
        Self::get_sub_board(self.open, sub_board)
    }

    pub fn is_sub_board_won_at(sub_board: UtttSubBitboard, square: UtttSubSquare) -> bool {
        const ROW_BB: RawStandardBitboard = RawStandardBitboard(0b111);
        const COLUMN_BB: RawStandardBitboard = RawStandardBitboard(0b001_001_001);
        const DIAG_BB: RawStandardBitboard = RawStandardBitboard(0b100_010_001);
        const ANTI_DIAG_BB: RawStandardBitboard = RawStandardBitboard(0b001_010_100);
        let bb = sub_board.raw();
        let row_bb = ROW_BB << (3 * square.row());
        let column_bb = COLUMN_BB << square.column();
        if bb & row_bb == row_bb || bb & column_bb == column_bb {
            return true;
        }
        // technically, this can also be true if the sub-board isn't won at this square, but that's fine
        if bb & DIAG_BB == DIAG_BB || bb & ANTI_DIAG_BB == ANTI_DIAG_BB {
            return true;
        }
        false
    }

    fn mark_as_won(&mut self, sub_board: UtttSubSquare, color: UtttColor) {
        debug_assert!(Self::calculate_sub_board_won(
            self.sub_board(color, sub_board)
        ));
        debug_assert!(
            Self::is_sub_board_won_at(
                self.sub_board(color, sub_board),
                UtttSubSquare::unchecked(0)
            ) || Self::is_sub_board_won_at(
                self.sub_board(color, sub_board),
                UtttSubSquare::unchecked(4)
            ) || Self::is_sub_board_won_at(
                self.sub_board(color, sub_board),
                UtttSubSquare::unchecked(8)
            )
        );
        self.colors_internal[color as usize] |=
            RawUtttBitboard::single_piece(Self::NUM_SQUARES + sub_board.bb_idx());
        self.open &= !ExtendedRawBitboard::from_u128(
            (Self::SUB_BOARD_MASK.to_primitive() as u128) << (sub_board.bb_idx() * 9),
        );
        debug_assert!(self.is_sub_board_won(color, sub_board));
        debug_assert!(!self.is_sub_board_open(sub_board));
    }

    fn update_won_bb(&mut self, square: UtttSquare, color: UtttColor) {
        // this looks at the metadata, which should not have this sub board marked as being won
        debug_assert!(!self.is_sub_board_won(color, square.sub_board()));

        let sub_board = self.sub_board(color, square.sub_board());
        if Self::is_sub_board_won_at(sub_board, square.sub_square()) {
            debug_assert!(Self::calculate_sub_board_won(sub_board));
            self.mark_as_won(square.sub_board(), color);
        } else {
            debug_assert!(!Self::calculate_sub_board_won(sub_board));
        }
    }

    // Ideally, this would be an associated const, but Rust's restrictions around const fns would make that ugly.
    // But hopefully, the compiler will constant fold this anyway
    fn won_masks() -> [UtttSubBitboard; 8] {
        [
            UtttSubBitboard::diagonal(UtttSubSquare::from_rank_file(1, 1))
                & UtttSubBitboard::new(Self::SUB_BOARD_MASK),
            UtttSubBitboard::anti_diagonal(UtttSubSquare::from_rank_file(1, 1))
                & UtttSubBitboard::new(Self::SUB_BOARD_MASK),
            UtttSubBitboard::file_no(0) & UtttSubBitboard::new(Self::SUB_BOARD_MASK),
            UtttSubBitboard::file_no(1) & UtttSubBitboard::new(Self::SUB_BOARD_MASK),
            UtttSubBitboard::file_no(2) & UtttSubBitboard::new(Self::SUB_BOARD_MASK),
            UtttSubBitboard::rank_no(0) & UtttSubBitboard::new(Self::SUB_BOARD_MASK),
            UtttSubBitboard::rank_no(1) & UtttSubBitboard::new(Self::SUB_BOARD_MASK),
            UtttSubBitboard::rank_no(2) & UtttSubBitboard::new(Self::SUB_BOARD_MASK),
        ]
    }

    fn calculate_sub_board_won(bb: UtttSubBitboard) -> bool {
        for mask in Self::won_masks() {
            if mask & bb == mask {
                return true;
            }
        }
        false
    }

    pub fn is_sub_board_won(self, color: UtttColor, sub_board: UtttSubSquare) -> bool {
        self.colors_internal[color as usize].is_bit_set_at(Self::NUM_SQUARES + sub_board.bb_idx())
    }

    pub fn is_sub_board_open(self, sub_board: UtttSubSquare) -> bool {
        self.open_sub_board(sub_board).has_set_bit()
    }

    pub fn is_open(self, square: UtttSquare) -> bool {
        self.open.is_bit_set_at(square.bb_idx())
    }

    pub fn from_alternative_fen(fen: &str, strictness: Strictness) -> Res<Self> {
        if fen.len() != Self::NUM_SQUARES
            || fen.contains(|c: char| ![' ', 'x', 'o', 'X', 'O'].contains(&c))
        {
            bail!(
                "Incorrect alternative UTTT FEN '{}', must consist of exactly 81 chars, all of which must be ' ', 'x', 'o', 'X', or 'O'",fen.red()
            );
        }
        let mut board = UnverifiedUtttBoard::new(Self::empty());
        for (idx, c) in fen.chars().enumerate() {
            if c == ' ' {
                continue;
            }
            let symbol = ColoredUtttPieceType::from_ascii_char(c).unwrap();
            let square = UtttSquare::from_bb_idx(idx);
            debug_assert!(board.check_coordinates(square).is_ok());
            board = board.place_piece_unchecked(square, symbol);
            if c.is_uppercase() {
                board.0.active = UtttColor::from_char(c).unwrap().other();
                let mov = board.last_move_mut();
                if *mov != UtttMove::NULL {
                    bail!("Upper case pieces are used for the last move, but there is more than one upper case letter in '{}'", fen.red());
                }
                *mov = UtttMove::new(square);
            }
        }
        board.verify_with_level(CheckFen, strictness)
    }

    pub fn to_alternative_fen(&self) -> String {
        let mut res = String::with_capacity(Self::NUM_SQUARES);
        for i in 0..Self::NUM_SQUARES {
            let square = UtttSquare::from_bb_idx(i);
            let mut c = self.colored_piece_on(square).to_ascii_char();
            if c == '.' {
                c = ' ';
            }
            if self.last_move != UtttMove::NULL && i == self.last_move.dest_square().bb_idx() {
                c = c.to_ascii_uppercase();
            }
            res.push(c);
        }
        res
    }

    #[must_use]
    pub fn yet_another_fen_format(&self) -> String {
        let mut res = String::new();
        res.push(self.active.ascii_color_char().to_ascii_uppercase());
        res.push(';');
        for sub_board in UtttSubSquare::iter() {
            let c = if sub_board == self.last_move.dest_square().sub_square()
                && self.is_sub_board_open(sub_board)
            {
                '@'
            } else if self.is_sub_board_won(X, sub_board) {
                'X'
            } else if self.is_sub_board_won(O, sub_board) {
                'O'
            } else {
                '.'
            };
            res.push(c);
        }
        res.push(';');
        for sub_board in UtttSubSquare::iter() {
            for sub_square in UtttSubSquare::iter() {
                let sq = UtttSquare::new(sub_board, sub_square);
                let c = self
                    .colored_piece_on(sq)
                    .symbol
                    .to_ascii_char()
                    .to_ascii_uppercase();
                res.push(c);
            }
            res.push('/');
        }
        res.pop();
        res
    }

    #[expect(clippy::unreadable_literal)]
    fn perft_test_positions() -> &'static [(&'static str, &'static [u64])] {
        // FENs from Serdra, perft numbers slightly modified to not count moves when the game is over
        #[rustfmt::skip]
        let res: &'static [(&'static str, &'static [u64])] = &[
        ("                                                                                 ", &[1, 81, 720, 6336, 55080, 473256, 4020960, 33782544, 281067408]),
        ("ox  x      o  x o  x     x     o    x o    x        O          oo x              ", &[1, 6, 44, 320, 2278, 16233, 116635, 849418, 6381392]),
        ("xo  o  xox x       o  o    o        x  x                              Oo  x    x ", &[1, 7, 51, 370, 2827, 21383, 179029, 1487259, 13593237]),
        (" ox     x    x x  x     o x          o      o         o      x o          oOx    ", &[1, 9, 64, 454, 3185, 23060, 166468, 1260336, 9736622]),
        (" x xx          o  o        o      x o    o  o    x           x   O     o  x x    ", &[1, 8, 58, 463, 3479, 29053, 241143, 2173280, 19748086]),
        ("o    x xx x   O oo  oo x    x  x   o  x oxo oo  oo   o x       o  xx    x x xx o ", &[1, 44, 391, 3436, 31662, 289755, 2792347, 26647358, 264767468]/*[1, 44, 391, 3436, 31662, 289755, 2792347, 26670866, 265068991,]*/),
        ("o    oox xox    o  xo  oox   x        O      xxo  oxxoxox  x      o xo o    xx   ", &[1, 4, 28, 239, 2212, 21384, 196693, 1923003, 18155997]),
        ("xxO o oxx x  o  o      o   o    x   o     xx x   x  o x    oo x o x  xoxo xo    o", &[1, 8, 86, 694, 5205, 40777, 319881, 2664061, 22872400]/*[1, 8, 86, 694, 5205, 40777, 319881, 2665475, 22892073,]*/),
        ("  o xox x      xo   xo   o x x  O oxo  o    o x  x    o  o    ox  xx x  oo x o  x", &[1, 7, 67, 840, 9609, 115330, 1283277, 14818322, 158683651]),
        ("   ooxx     o xx  x        o    x xx x   ox x  o oxOo o  x  oooxo    x   o ox    ", &[1, 41, 440, 4759, 48816, 496752, 4825482, 47240207, 442983131]),
        (" o    o ox    x  x      oo xoxxxx    o   o x    o x oo   o  xOxx  ox      xoo x  ", &[1, 6, 33, 298, 2978, 27462, 251373, 2277374, 20505230]),
        ("xxox    ooooo    o  oxxx x o oxxxo   xo  o xO xxoooxo   xo x o xx   ox oox xox  x", &[1, 3, 22, 170, 1292, 7611, 42488, 178604, 683640]/*[1, 3, 22, 190, 1428, 9417, 51891, 246026, 928623,]*/),
        ("oo o xxo      oooo       ox ox o  o x   ox oxxx xxoooo x xxxo  xx xox xxxx xOoo o", &[1, 4, 58, 519, 4456, 33205, 232391, 1384237, 7568559]/*[1, 4, 58, 547, 4704, 36991, 263264, 1660876, 9373948,]*/),
        ("   x  o x   xxox o oxxxx    oo  xoo o  x oo   oo oxo ooxo xx xxx o  xxoxO xo xxoo", &[1, 6, 63, 414, 2614, 17476, 108288, 680618, 3769073]/*[1, 6, 63, 414, 2614, 17476, 113003, 716714, 4215813,]*/),
        (" x  ox x    ox x oooxo    oxo xxx ox  ooxooxx  xoo   o   xo x oo x  O oxoxxoxx xo", &[1, 5, 23, 171, 1094, 7508, 47807, 322940, 2032799]),
        ("xx oxx x o o  xx xoo   ooxx x  oox    xxoxo    x oxxoxooxx o xoO o  ox xo  o  oo ", &[1, 22, 163, 1457, 10431, 82349, 519427, 3451682, 17775153]/*[1, 22, 163, 1457, 10431, 82494, 525556, 3507096, 18503464,]*/),
        ("  o   xxxoox oxoxxxx o ox oxo oxoxoxo   ooxxo xx xox ooxoxxoooxxxoxOxo oooxxxooox", &[1, 3, 4, 1, 0, 0, 0, 0, 0]),
        ("x ox ox  ooxxoxoo  oxox xo  xoooxxxooxxxoxx xoooox   oxxoooxxoo  xxoxoxoxxo o Oxx", &[1, 3, 6, 5, 0, 0, 0, 0, 0]),
        ("xx xooOxoooxooxxxoxo o xo o ox x x   xooxoxx ox xxoxoxxoxoox ooxx o oo xoxx xooox", &[1, 9, 35, 123, 327, 695, 1090, 1359, 896]/*[1, 9, 38, 135, 390, 882, 1563, 2019, 1572,]*/),
        ("xx oOxooxx  xooooooxo xxox oxooxoxoxxoxooooo oxoxx o x xxxx oxxxxoxxo  xo   ox  o", &[1, 3, 4, 2, 0, 0, 0, 0, 0]),
        ("o   o xxxooooxxxo oxxxoxooxooxx ooxxxxxxoo   xoooOxxooo  xo  ooxxx x x oxxoo oxox", &[1, 2, 1, 0, 0, 0, 0, 0, 0]/*[1, 2, 2, 0, 0, 0, 0, 0, 0,]*/),
        ("xoo x oxooxoxoxxxoxo o xxxooxxx  xooOo o    xxxoo o  x xxo ooxxoox xxooxxooxxooox", &[1, 2, 5, 29, 118, 451, 1452, 4785, 12074]/*[1, 2, 5, 29, 130, 507, 1888, 6357, 19631,]*/),
        ("oxoxoxo  xo x oOoo xxoo oxoxxx xxxooxxoooxx xxooo xxooo  ooox xo xoxo xoxxxo   xx", &[1, 6, 16, 26, 41, 19, 6, 0, 0]/*[1, 6, 16, 32, 47, 23, 6, 0, 0,]*/),
        ("  oo xoox  xx oxxoxoo  oxooooxxoo xxx xo xoxooxxoxxx ox xx ooooxoOxxooxo x oxoxx ", &[1, 9, 26, 71, 140, 284, 357, 338, 194]/*[1, 9, 26, 71, 156, 320, 468, 443, 297,]*/),
        ("ox  oxx    o  xo o o x  o xox  x  o x   xooooox ooxx ox oxx oox  x  x  xo xoXoxox", &[1, 0, 0, 0, 0, 0, 0, 0]),
        ];
        res
    }
}

impl Display for UtttBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_fen())
    }
}

impl Board for UtttBoard {
    type EmptyRes = UtttBoard;
    type Settings = UtttSettings;
    type Coordinates = UtttSquare;
    type Color = UtttColor;
    type Piece = UtttPiece;
    type Move = UtttMove;
    type MoveList = UtttMoveList;
    type Unverified = UnverifiedUtttBoard;

    fn empty_for_settings(_settings: UtttSettings) -> Self {
        Self::default()
    }

    fn startpos_for_settings(_settings: UtttSettings) -> Self {
        Self::default()
    }

    fn from_name(name: &str) -> Res<Self> {
        board_from_name(name)
    }

    fn bench_positions() -> Vec<Self> {
        Self::perft_test_positions()
            .iter()
            .map(|(fen, _res)| Self::from_alternative_fen(fen, Strict).unwrap())
            .collect_vec()
    }

    fn settings(&self) -> UtttSettings {
        UtttSettings {}
    }

    fn active_player(&self) -> UtttColor {
        self.active
    }

    fn halfmove_ctr_since_start(&self) -> usize {
        self.ply_since_start
    }

    fn halfmove_repetition_clock(&self) -> usize {
        0
    }

    fn size(&self) -> UtttSize {
        UtttSize::default()
    }

    /// Only checks if the square is empty, which is not the same as checking if the square is open:
    /// When a sub-board has been won, it's illegal to place a piece on an empty square in it
    fn is_empty(&self, square: UtttSquare) -> bool {
        !(self.colors_internal[0] | self.colors_internal[1]).is_bit_set_at(square.bb_idx())
    }

    fn colored_piece_on(&self, square: UtttSquare) -> Self::Piece {
        if self.colors_internal[XStone as usize].is_bit_set_at(square.bb_idx()) {
            UtttPiece::new(XStone, square)
        } else if self.colors_internal[OStone as usize].is_bit_set_at(square.bb_idx()) {
            UtttPiece::new(OStone, square)
        } else {
            UtttPiece::new(ColoredUtttPieceType::Empty, square)
        }
    }

    fn gen_pseudolegal<T: MoveList<Self>>(&self, moves: &mut T) {
        // don't assume that the board is empty in startpos to support different starting positions
        if self.player_result_no_movegen(&NoHistory::default()) == Some(Lose) {
            return;
        }
        if self.last_move != UtttMove::NULL {
            let sub_board = self.last_move.dest_square().sub_square();
            if self.is_sub_board_open(sub_board) {
                debug_assert!(
                    !self.is_sub_board_won(X, sub_board) && !self.is_sub_board_won(O, sub_board)
                );
                let sub_bitboard = self.open_sub_board(sub_board);
                for idx in sub_bitboard.one_indices() {
                    let square = UtttSquare::new(sub_board, UtttSubSquare::from_bb_index(idx));
                    moves.add_move(UtttMove::new(square));
                }
                return;
            }
        }

        for sq in self.open_bb().one_indices() {
            moves.add_move(UtttMove::new(UtttSquare::from_bb_idx(sq)));
        }
    }

    fn gen_tactical_pseudolegal<T: MoveList<Self>>(&self, _moves: &mut T) {
        // TODO: Test considering moves that win a sub-board as tactical
        // currently, no moves are considered tactical
    }

    fn random_legal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
        let open = self.open_bb();
        if open.is_zero() {
            return None;
        }
        let idx = rng.gen_range(0..open.num_ones());
        let idx = ith_one_u128(idx, open.0);
        Some(UtttMove(UtttSquare::from_bb_idx(idx)))
    }

    fn random_pseudolegal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
        self.random_legal_move(rng)
    }

    fn make_move(mut self, mov: Self::Move) -> Option<Self> {
        let color = self.active;
        let square = mov.dest_square();
        let bb = ExtendedRawBitboard::single_piece(square.bb_idx());
        self.colors_internal[color as usize] |= bb;
        self.open &= !bb;
        self.update_won_bb(square, color);
        self.active = !self.active;
        self.last_move = mov;
        self.ply_since_start += 1;
        Some(self)
    }

    fn make_nullmove(mut self) -> Option<Self> {
        self.active = !self.active;
        self.last_move = UtttMove::NULL;
        self.ply_since_start += 1;
        Some(self)
    }

    fn is_move_pseudolegal(&self, mov: Self::Move) -> bool {
        if !self.last_move.is_null() {
            let sub_board = self.last_move.dest_square().sub_square();
            if self.is_sub_board_open(sub_board) && mov.dest_square().sub_board() != sub_board {
                return false;
            }
        }
        self.is_open(mov.dest_square())
    }

    fn player_result_no_movegen<H: BoardHistory<Self>>(
        &self,
        _history: &H,
    ) -> Option<PlayerResult> {
        if self.last_move == UtttMove::NULL {
            return None;
        }
        let sq = self.last_move.dest_square().sub_board();
        let bb = self.won_sub_boards(self.inactive_player());
        if Self::is_sub_board_won_at(bb, sq) {
            Some(Lose)
        } else if self.open_bb().is_zero() {
            Some(Draw) // technically, this doesn't need to be checked here, but it's cheap, so we might as well
        } else {
            None
        }
    }

    fn player_result_slow<H: BoardHistory<Self>>(&self, history: &H) -> Option<PlayerResult> {
        self.player_result_no_movegen(history)
    }

    fn no_moves_result(&self) -> PlayerResult {
        debug_assert!(self.open_bb().is_zero());
        Draw
    }

    fn can_reasonably_win(&self, _player: Self::Color) -> bool {
        true
    }

    fn zobrist_hash(&self) -> ZobristHash {
        // TODO: Test using a better hash. Also, `ZobristHash` is a bad name in general
        let mut hasher = DefaultHasher::default();
        // the bitboards and the last move must be part of the hash
        self.hash(&mut hasher);
        ZobristHash(hasher.finish())
    }

    fn as_fen(&self) -> String {
        use fmt::Write;
        let mut res = common_fen_part(self, false, true);
        write!(&mut res, " {}", self.last_move).unwrap();
        res
    }

    // TODO: Don't use a separate open bitboard, just set both players' bitboards to one for squares that are no longer
    // reachable because the sub board has been won, and update the piece_on function

    fn read_fen_and_advance_input(input: &mut Tokens, strictness: Strictness) -> Res<Self> {
        let pos = Self::default();
        let mut pos = read_common_fen_part::<UtttBoard>(input, pos.into())?;

        let mut fullmove_counter = parse_int(input, "fullmove counter")?;
        if fullmove_counter == 0 {
            if strictness == Strict {
                bail!("The fullmove counter is one-based and can't be zero")
            } else {
                fullmove_counter = 1;
            }
        }
        let fullmoves = NonZeroUsize::new(fullmove_counter).unwrap();
        pos.0.ply_since_start =
            ply_counter_from_fullmove_nr::<UtttBoard>(fullmoves, pos.0.active_player());
        let Some(last_move) = input.next() else {
            bail!("Ultimate Tic-Tac-Toe FEN ends after ply counter, missing the last move")
        };
        // Use an empty board for parsing the last move instead of the current board because that would complain about the last
        // move being invalid because the square is already occupied.
        let last_move = UtttMove::from_compact_text(last_move, &Self::default())?;
        pos.0.last_move = last_move;
        // The won sub boards bitboard is set in the verify method
        pos.verify_with_level(CheckFen, strictness)
    }

    fn should_flip_visually() -> bool {
        false
    }

    fn as_ascii_diagram(&self, flip: bool) -> String {
        board_to_string(self, UtttPiece::to_ascii_char, flip)
    }

    fn as_unicode_diagram(&self, flip: bool) -> String {
        board_to_string(self, UtttPiece::to_utf8_char, flip)
    }

    fn display_pretty(&self, fmt: &mut dyn BoardFormatter<Self>) -> String {
        display_board_pretty(self, fmt)
    }

    fn pretty_formatter(
        &self,
        piece_to_char: PieceToChar,
        last_move: Option<UtttMove>,
    ) -> Box<dyn BoardFormatter<Self>> {
        let pos = *self;
        let formatter = AdaptFormatter {
            underlying: Box::new(DefaultBoardFormatter::new(*self, piece_to_char, last_move)),
            color_frame: Box::new(move |sq, col| {
                if col.is_some() {
                    return col;
                }
                if pos.is_sub_board_won(UtttColor::first(), sq.sub_board()) {
                    Some(p1_color())
                } else if pos.is_sub_board_won(UtttColor::second(), sq.sub_board()) {
                    Some(p2_color())
                } else {
                    None
                }
            }),
            display_piece: Box::new(|_, _, default| default),
            horizontal_spacer_interval: Some(3),
            vertical_spacer_interval: Some(3),
        };
        Box::new(formatter)
    }

    fn background_color(&self, square: UtttSquare) -> SquareColor {
        square.sub_square().square_color()
    }
}

#[derive(Debug, Copy, Clone)]
#[must_use]
pub struct UnverifiedUtttBoard(UtttBoard);

impl From<UtttBoard> for UnverifiedUtttBoard {
    fn from(board: UtttBoard) -> Self {
        Self(board)
    }
}

impl UnverifiedBoard<UtttBoard> for UnverifiedUtttBoard {
    fn verify_with_level(self, checks: SelfChecks, _strictness: Strictness) -> Res<UtttBoard> {
        let mut this = self.0;
        if checks != CheckFen {
            for color in UtttColor::iter() {
                let bb = this.colors_internal[color as usize];
                if (bb >> (81 + 9)).has_set_bit() {
                    bail!("The {color} bitboard contains a set bit above the range of used bits, the bitboard is {bb}");
                }
            }
            if (this.colors_internal[0] & this.colors_internal[1]).has_set_bit() {
                bail!("At least one square is occupied by both players, the bitboards are {0} and {1}", this.colors_internal[0], this.colors_internal[1]);
            }
        }
        if this.last_move != UtttMove::NULL {
            let sq = this.last_move.dest_square();
            match this.colored_piece_on(sq).color() {
                None => {
                    bail!("The square '{sq}', on which the last move has been played, is empty")
                }
                Some(col) => {
                    if col == this.active {
                        bail!("The square '{sq}', on which the last move has been played, is occupied by the {} player, \
                        which is not the player active in the previous ply", this.active);
                    }
                }
            }
        }
        // Allow starting positions with squares already filled out, so the ply and the number of nonempty squares don't have to match.
        // But the ply number still has to be at most the number of nonempty squares
        if this.ply_since_start > this.occupied_bb().num_ones() {
            bail!(
                "The ply number is '{0}', but only {1} pieces have been placed so far",
                this.ply_since_start,
                this.occupied_bb().num_ones()
            );
        } else if this.ply_since_start == 0 && !this.active_player().is_first() {
            this.ply_since_start = 1; // just quietly fix this, even in strict mode.
        }
        this.open = UtttBoard::board_bb(!this.occupied_bb());
        for color in UtttColor::iter() {
            // reset the metadata because it's out of date
            this.colors_internal[color as usize] &= UtttBoard::BOARD_BB;
            for sub_board in UtttSubSquare::iter() {
                let won_sub_board =
                    UtttBoard::calculate_sub_board_won(this.sub_board(color, sub_board));
                if won_sub_board {
                    this.mark_as_won(sub_board, color);
                }
            }
        }
        let mut won_by_both = this.won_sub_boards(O) & this.won_sub_boards(X);
        if won_by_both.has_set_bit() {
            bail!(
                "Sub board {0} has been won by both players",
                UtttSubSquare::from_bb_index(won_by_both.pop_lsb())
            );
        }
        for color in UtttColor::iter() {
            let won_sub_boards = this.won_sub_boards(color);
            if UtttBoard::calculate_sub_board_won(won_sub_boards) {
                let sq = this.last_move.dest_square();
                if !won_sub_boards.is_bit_set_at(sq.sub_board().bb_idx())
                    || !UtttBoard::is_sub_board_won_at(
                        this.sub_board(color, sq.sub_board()),
                        sq.sub_square(),
                    )
                {
                    bail!("The game is won for player {color}, but their last move (at {sq}) didn't win the game");
                }
            }
        }
        if checks == Assertion {
            assert!((this.open & !this.all_empty_squares_bb()).is_zero());
            for sub_board in UtttSubSquare::iter() {
                let won =
                    this.is_sub_board_won(X, sub_board) || this.is_sub_board_won(O, sub_board);
                let bb = UtttBoard::get_sub_board(this.occupied_bb(), sub_board);
                if bb.raw() == UtttBoard::SUB_BOARD_MASK {
                    assert!(!this.is_sub_board_open(sub_board));
                } else {
                    assert_ne!(this.is_sub_board_open(sub_board), won);
                }
            }
        }
        Ok(this)
    }

    fn size(&self) -> UtttSize {
        self.0.size()
    }

    fn place_piece_unchecked(mut self, square: UtttSquare, piece: ColoredUtttPieceType) -> Self {
        let color = piece.color().unwrap();
        let bb = ExtendedRawBitboard::single_piece(square.bb_idx());
        self.0.colors_internal[color as usize] |= bb;
        self
    }

    fn remove_piece_unchecked(mut self, square: UtttSquare) -> Self {
        let bb = ExtendedRawBitboard::single_piece(square.bb_idx());
        self.0.colors_internal[0] &= !bb;
        self.0.colors_internal[1] &= !bb;
        self
    }

    fn piece_on(&self, coords: UtttSquare) -> Res<UtttPiece> {
        Ok(self.0.colored_piece_on(self.check_coordinates(coords)?))
    }

    fn set_active_player(mut self, player: UtttColor) -> Self {
        self.0.active = player;
        self
    }

    fn set_ply_since_start(mut self, ply: usize) -> Res<Self> {
        self.0.ply_since_start = ply;
        Ok(self)
    }
}

impl UnverifiedUtttBoard {
    pub fn last_move_mut(&mut self) -> &mut UtttMove {
        &mut self.0.last_move
    }
}
