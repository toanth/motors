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

use crate::PlayerResult;
use crate::PlayerResult::{Draw, Lose};
use crate::games::uttt::ColoredUtttPieceType::{OStone, XStone};
use crate::games::uttt::UtttColor::{O, X};
use crate::games::uttt::UtttPieceType::{Empty, Occupied};
use crate::games::uttt::uttt_square::{UtttSize, UtttSquare};
use crate::games::{
    AbstractPieceType, BoardHistory, CharType, Color, ColoredPiece, ColoredPieceType, GenericPiece, PieceType, PosHash,
    Settings, Size,
};
use crate::general::bitboards::{
    Bitboard, DynamicallySizedBitboard, ExtendedRawBitboard, KnownSizeBitboard, RawBitboard, RawStandardBitboard,
};
use crate::general::board::SelfChecks::*;
use crate::general::board::Strictness::Strict;
use crate::general::board::{
    Board, BoardHelpers, NameToPos, SelfChecks, Strictness, Symmetry, UnverifiedBoard, ply_counter_from_fullmove_nr,
    read_common_fen_part, simple_fen,
};
use crate::general::common::{EntityList, Res, StaticallyNamedEntity, Tokens, ith_one_u64, ith_one_u128, parse_int};
use crate::general::move_list::{InplaceMoveList, MoveList};
use crate::general::moves::Legality::Legal;
use crate::general::moves::{Legality, Move, UntrustedMove};
use crate::general::squares::{RectangularCoordinates, SmallGridSize, SmallGridSquare, SquareColor};
use crate::output::OutputOpts;
use crate::output::text_output::{
    AdaptFormatter, BoardFormatter, DefaultBoardFormatter, board_to_string, display_board_pretty, p1_color, p2_color,
};
use crate::search::DepthPly;
use anyhow::bail;
use arbitrary::Arbitrary;
use colored::Colorize;
use itertools::Itertools;
use rand::Rng;
use std::fmt::{Display, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::num::NonZeroUsize;
use std::ops::Not;
use std::str::FromStr;
use std::{fmt, iter};
use strum_macros::{EnumIter, FromRepr};

/// `Bitboard`s have the  semantic constraint of storing squares in a row-major fashion.
/// That's why the public API of this struct only exposes `RawBitboard`s, which don't have this constraint,
/// and `SubBitboard`s, which adhere to that.
pub type RawUtttBitboard = ExtendedRawBitboard;

pub type UtttSubSize = SmallGridSize<3, 3>;

pub type UtttSubSquare = SmallGridSquare<3, 3, 3>;

pub type UtttSubBitboard = DynamicallySizedBitboard<RawStandardBitboard, UtttSubSquare>;

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone)]
pub struct UtttSettings;

impl Settings for UtttSettings {}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, derive_more::Display, Arbitrary)]
#[must_use]
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

impl From<UtttColor> for usize {
    fn from(color: UtttColor) -> usize {
        color as usize
    }
}

impl Color for UtttColor {
    type Board = UtttBoard;

    fn second() -> Self {
        O
    }

    fn to_char(self, _settings: &UtttSettings) -> char {
        match self {
            X => 'x',
            O => 'o',
        }
    }

    fn name(self, _settings: &<Self::Board as Board>::Settings) -> &str {
        match self {
            X => "X",
            O => "O",
        }
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, EnumIter, FromRepr)]
#[must_use]
pub enum UtttPieceType {
    #[default]
    Empty,
    Occupied,
}

impl Display for UtttPieceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char(CharType::Ascii, &UtttSettings))
    }
}

impl AbstractPieceType<UtttBoard> for UtttPieceType {
    fn empty() -> Self {
        Empty
    }

    fn non_empty(_settings: &UtttSettings) -> impl Iterator<Item = Self> {
        iter::once(Occupied)
    }

    fn to_char(self, _typ: CharType, _settings: &UtttSettings) -> char {
        match self {
            Empty => '.',
            Occupied => 'x',
        }
    }

    fn from_char(c: char, _settings: &UtttSettings) -> Option<Self> {
        match c {
            '.' => Some(Empty),
            'o' | 'x' => Some(Occupied),
            _ => None,
        }
    }

    fn name(&self, _settings: &UtttSettings) -> impl AsRef<str> {
        match self {
            Empty => "empty",
            Occupied => "occupied",
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
#[must_use]
pub enum ColoredUtttPieceType {
    XStone,
    OStone,
    #[default]
    Empty, // last so that `XStone` and `OStone` have indices 0 and 1
}

impl Display for ColoredUtttPieceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char(CharType::Ascii, &UtttSettings))
    }
}

const UNICODE_X: char = '⨉'; // '⨉',
const UNICODE_O: char = '◯'; // '○'

impl AbstractPieceType<UtttBoard> for ColoredUtttPieceType {
    fn empty() -> Self {
        Self::Empty
    }

    fn non_empty(_settings: &UtttSettings) -> impl Iterator<Item = Self> {
        [XStone, OStone].into_iter()
    }

    fn to_char(self, typ: CharType, _settings: &UtttSettings) -> char {
        match typ {
            CharType::Ascii => match self {
                ColoredUtttPieceType::Empty => '.',
                XStone => 'x',
                OStone => 'o',
            },
            CharType::Unicode => match self {
                ColoredUtttPieceType::Empty => '.',
                XStone => UNICODE_X,
                OStone => UNICODE_O,
            },
        }
    }

    fn to_display_char(self, typ: CharType, settings: &UtttSettings) -> char {
        self.to_char(typ, settings).to_ascii_uppercase()
    }

    fn from_char(c: char, _settings: &UtttSettings) -> Option<Self> {
        match c {
            '.' => Some(ColoredUtttPieceType::Empty),
            'x' | 'X' | UNICODE_X => Some(XStone),
            'o' | 'O' | UNICODE_O => Some(OStone),
            _ => None,
        }
    }

    fn name(&self, _settings: &UtttSettings) -> impl AsRef<str> {
        match self {
            XStone => "x",
            OStone => "o",
            ColoredUtttPieceType::Empty => "empty",
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
#[must_use]
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

    fn dest_square(self) -> UtttSquare {
        self.0
    }
}

impl Display for UtttMove {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.is_null() { write!(f, "0000") } else { write!(f, "{}", self.dest_square()) }
    }
}

impl Move<UtttBoard> for UtttMove {
    type Underlying = u8;

    fn legality(_: &UtttSettings) -> Legality {
        Legal
    }

    fn src_square_in(self, _pos: &UtttBoard) -> Option<UtttSquare> {
        None
    }

    fn dest_square_in(self, _pos: &UtttBoard) -> UtttSquare {
        self.0
    }

    fn is_tactical(self, _board: &UtttBoard) -> bool {
        // TODO: Consider moves that win a sub-board tactical?
        false
    }

    fn description(self, board: &UtttBoard) -> String {
        let piece = board.active.to_string().bold();
        let to = self.0.to_string().bold();
        format!("Place a {piece} on {to}")
    }

    fn format_compact(self, f: &mut Formatter<'_>, _board: &UtttBoard) -> fmt::Result {
        write!(f, "{self}")
    }

    fn parse_compact_text<'a>(s: &'a str, board: &UtttBoard) -> Res<(&'a str, UtttMove)> {
        // TODO: This is not pseudolegal, so allowing it seems dangerous
        if let Some(rest) = s.strip_prefix("0000") {
            return Ok((rest, Self::NULL));
        }
        let Some(square_str) = s.get(..2) else {
            bail!("UTTT move '{}' doesn't start with a square consisting of two ASCII characters", s.red())
        };
        let square = UtttSquare::from_str(square_str)?;
        if !board.is_open(square) {
            bail!("Square {square} is not empty, so this move is invalid");
        } else if !board.is_move_pseudolegal(Self(square)) {
            bail!("Incorrect sub-board. The previous move determines the sub-board for this move")
        }
        Ok((&s[2..], Self(square)))
    }

    fn parse_extended_text<'a>(s: &'a str, board: &UtttBoard) -> Res<(&'a str, UtttMove)> {
        Self::parse_compact_text(s, board)
    }

    fn from_u64_unchecked(val: u64) -> UntrustedMove<UtttBoard> {
        UntrustedMove::from_move(Self(UtttSquare::unchecked(val as usize)))
    }

    fn to_underlying(self) -> Self::Underlying {
        self.0.to_u8()
    }
}

pub type UtttMoveList = InplaceMoveList<UtttBoard, 81>;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub struct UtttBoard {
    // contains not just the occupancy, but also won sub-boards in the higher bits.
    colors_internal: [RawUtttBitboard; 2],
    // all squares where a piece can still be placed (different from empty squares because subboards can be closed)
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
        "Ultimate Tic-Tac-Toe is a challenging variant of Tic-Tac-Toe, where every square is itself a Tic-Tac-Toe board"
            .to_string()
    }
}

// TODO: BitboardBoard trait with default implementations like empty_bb() = !occupied_bb(), active_player_bb(), etc
// (Need to make sure this works for bitboards where only some of the bits are used)
impl UtttBoard {
    const BOARD_BB: ExtendedRawBitboard = 0x1_ffff_ffff_ffff_ffff_ffff;
    const SUB_BOARD_MASK: RawStandardBitboard = 0x1ff;

    const NUM_SQUARES: usize = 81;

    fn board_bb(bb: ExtendedRawBitboard) -> RawUtttBitboard {
        bb & Self::BOARD_BB
    }

    // TODO: Should already be handled in board.rs? (same for some other methods)
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
        UtttSubBitboard::from_raw((bb >> Self::NUM_SQUARES) as u64)
    }

    fn get_sub_board(bb: RawUtttBitboard, sub_board: UtttSubSquare) -> UtttSubBitboard {
        let bb = (bb >> (sub_board.bb_idx() * 9)) as u64 & Self::SUB_BOARD_MASK;
        UtttSubBitboard::from_raw(bb)
    }

    pub fn sub_board(&self, color: UtttColor, sub_board: UtttSubSquare) -> UtttSubBitboard {
        Self::get_sub_board(self.colors_internal[color as usize], sub_board)
    }

    pub fn open_sub_board(&self, sub_board: UtttSubSquare) -> UtttSubBitboard {
        Self::get_sub_board(self.open, sub_board)
    }

    pub fn is_sub_board_won_at(sub_board: UtttSubBitboard, square: UtttSubSquare) -> bool {
        const ROW_BB: RawStandardBitboard = 0b111;
        const COLUMN_BB: RawStandardBitboard = 0b001_001_001;
        const DIAG_BB: RawStandardBitboard = 0b100_010_001;
        const ANTI_DIAG_BB: RawStandardBitboard = 0b001_010_100;
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
        debug_assert!(Self::calculate_sub_board_won(self.sub_board(color, sub_board)));
        debug_assert!(
            Self::is_sub_board_won_at(self.sub_board(color, sub_board), UtttSubSquare::unchecked(0))
                || Self::is_sub_board_won_at(self.sub_board(color, sub_board), UtttSubSquare::unchecked(4))
                || Self::is_sub_board_won_at(self.sub_board(color, sub_board), UtttSubSquare::unchecked(8))
        );
        self.colors_internal[color as usize] |=
            RawUtttBitboard::single_piece_at(Self::NUM_SQUARES + sub_board.bb_idx());
        self.open &= !((Self::SUB_BOARD_MASK as u128) << (sub_board.bb_idx() * 9));
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
    // But hopefully, the compiler will constant-fold this anyway
    fn won_masks() -> [UtttSubBitboard; 8] {
        [
            UtttSubBitboard::diagonal(UtttSubSquare::from_rank_file(1, 1))
                & UtttSubBitboard::from_raw(Self::SUB_BOARD_MASK),
            UtttSubBitboard::anti_diagonal(UtttSubSquare::from_rank_file(1, 1))
                & UtttSubBitboard::from_raw(Self::SUB_BOARD_MASK),
            UtttSubBitboard::file(0) & UtttSubBitboard::from_raw(Self::SUB_BOARD_MASK),
            UtttSubBitboard::file(1) & UtttSubBitboard::from_raw(Self::SUB_BOARD_MASK),
            UtttSubBitboard::file(2) & UtttSubBitboard::from_raw(Self::SUB_BOARD_MASK),
            UtttSubBitboard::rank(0) & UtttSubBitboard::from_raw(Self::SUB_BOARD_MASK),
            UtttSubBitboard::rank(1) & UtttSubBitboard::from_raw(Self::SUB_BOARD_MASK),
            UtttSubBitboard::rank(2) & UtttSubBitboard::from_raw(Self::SUB_BOARD_MASK),
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

    pub fn last_move_won_game(&self) -> bool {
        if self.last_move.is_null() {
            false
        } else {
            let sq = self.last_move.dest_square().sub_board();
            let bb = self.won_sub_boards(self.inactive_player());
            Self::is_sub_board_won_at(bb, sq)
        }
    }

    pub fn from_alternative_fen(fen: &str, strictness: Strictness) -> Res<Self> {
        if fen.len() != Self::NUM_SQUARES || fen.contains(|c: char| ![' ', 'x', 'o', 'X', 'O'].contains(&c)) {
            bail!(
                "Incorrect alternative UTTT FEN '{}', must consist of exactly 81 chars, all of which must be ' ', 'x', 'o', 'X', or 'O'",
                fen.red()
            );
        }
        let mut board = UnverifiedUtttBoard::new(Self::empty());
        for (idx, c) in fen.chars().enumerate() {
            if c == ' ' {
                continue;
            }
            let symbol = ColoredUtttPieceType::from_char(c, board.settings()).unwrap();
            let square = UtttSquare::from_bb_idx(idx);
            debug_assert!(board.check_coordinates(square).is_ok());
            board.place_piece(square, symbol);
            if c.is_uppercase() {
                board.0.active = UtttColor::from_char(c, &UtttSettings).unwrap().other();
                let mov = board.last_move_mut();
                if *mov != UtttMove::NULL {
                    bail!(
                        "Upper case pieces are used for the last move, but there is more than one upper case letter in '{}'",
                        fen.red()
                    );
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
            let mut c = self.colored_piece_on(square).to_char(CharType::Ascii, self.settings());
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
        res.push(self.active.to_char(&UtttSettings).to_ascii_uppercase());
        res.push(';');
        for sub_board in UtttSubSquare::iter() {
            let c = if sub_board == self.last_move.dest_square().sub_square() && self.is_sub_board_open(sub_board) {
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
                let c = self.colored_piece_on(sq).symbol.to_char(CharType::Ascii, self.settings()).to_ascii_uppercase();
                res.push(c);
            }
            res.push('/');
        }
        _ = res.pop();
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
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{0} {1}", simple_fen(self, false, true), self.last_move)
    }
}

impl Board for UtttBoard {
    type EmptyRes = UtttBoard;
    type Settings = UtttSettings;
    type SettingsRef = UtttSettings;
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

    fn name_to_pos_map() -> EntityList<NameToPos> {
        vec![
            NameToPos::strict("midgame", "o1o4x1/9/xox4x1/x2x5/4o4/o2x1o3/o2x1o3/1x1xo1ox1/oxx1o1ox1 o 14 h9"),
            NameToPos::strict(
                "mate_in_6",
                "oxoooxoxx/x2o2o1x/xox1x2x1/xxox2x1o/xox1oxo1o/o2x1o3/o2xoo3/1x1xo1ox1/oxx1o1ox1 o 25 g6",
            ),
        ]
    }

    fn bench_positions() -> Vec<Self> {
        Self::perft_test_positions()
            .iter()
            .map(|(fen, _res)| Self::from_alternative_fen(fen, Strict).unwrap())
            .collect_vec()
    }

    fn random_pos(rng: &mut impl Rng, strictness: Strictness, symmetry: Option<Symmetry>) -> Res<Self> {
        if symmetry.is_some() {
            bail!("The UTTT game doesn't support setting up a random symmetrical position")
        }
        loop {
            let mut pos = UnverifiedUtttBoard::new(UtttBoard::empty());
            let num_pieces = rng.random_range(0..42);
            for _ in 0..num_pieces {
                let empty = pos.0.all_empty_squares_bb();
                let sq = ith_one_u128(rng.random_range(0..empty.num_ones()), empty);
                let piece = if rng.random_bool(0.5) { O } else { X };
                let piece = ColoredUtttPieceType::new(piece, Occupied);
                pos.place_piece(UtttSquare::from_bb_idx(sq), piece)
            }
            if rng.random_bool(0.5) {
                pos.0.active = !pos.0.active;
            }
            if rng.random_bool(0.5) {
                let bb = pos.0.inactive_player_bb();
                if bb.has_set_bit() {
                    let piece = ith_one_u128(rng.random_range(0..bb.num_ones()), bb);
                    pos.0.last_move = UtttMove::new(UtttSquare::from_bb_idx(piece));
                }
            }
            if let Ok(pos) = pos.verify(strictness) {
                return Ok(pos);
            }
        }
    }

    fn settings(&self) -> &UtttSettings {
        &UtttSettings {}
    }

    fn settings_ref(&self) -> Self::SettingsRef {
        UtttSettings {}
    }

    fn active_player(&self) -> UtttColor {
        self.active
    }

    fn halfmove_ctr_since_start(&self) -> usize {
        self.ply_since_start
    }

    fn ply_draw_clock(&self) -> usize {
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

    fn default_perft_depth(&self) -> DepthPly {
        DepthPly::new(5)
    }

    fn cannot_call_movegen(&self) -> bool {
        self.last_move_won_game()
    }

    // TODO: Testcase that it's impossible to lead a FEN where a player won the game
    fn gen_pseudolegal<T: MoveList<Self>>(&self, moves: &mut T) {
        debug_assert!(!self.last_move_won_game(), "{self}");
        if self.last_move != UtttMove::NULL {
            let sub_board = self.last_move.dest_square().sub_square();
            if self.is_sub_board_open(sub_board) {
                debug_assert!(!self.is_sub_board_won(X, sub_board) && !self.is_sub_board_won(O, sub_board));
                let sub_bitboard = self.open_sub_board(sub_board);
                for idx in sub_bitboard.one_indices() {
                    let square = UtttSquare::new(sub_board, UtttSubSquare::from_bb_idx(idx));
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

    fn num_pseudolegal_moves(&self) -> usize {
        debug_assert!(!self.last_move_won_game());
        if self.last_move != UtttMove::NULL {
            let sub_board = self.last_move.dest_square().sub_square();
            if self.is_sub_board_open(sub_board) {
                debug_assert!(!self.is_sub_board_won(X, sub_board) && !self.is_sub_board_won(O, sub_board));
                return self.open_sub_board(sub_board).num_ones();
            }
        }
        self.open_bb().num_ones()
    }

    fn has_no_legal_moves(&self) -> bool {
        self.open_bb().is_zero()
    }

    fn random_legal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
        debug_assert!(!self.last_move_won_game());
        if self.last_move != UtttMove::NULL {
            let sub_board = self.last_move.dest_square().sub_square();
            if self.is_sub_board_open(sub_board) {
                debug_assert!(!self.is_sub_board_won(X, sub_board) && !self.is_sub_board_won(O, sub_board));
                let bb = self.open_sub_board(sub_board);
                let idx = rng.random_range(0..bb.num_ones());
                let idx = ith_one_u64(idx, bb.raw());
                let sq = UtttSquare::new(sub_board, UtttSubSquare::from_bb_idx(idx));
                return Some(UtttMove::new(sq));
            }
        }
        let bb = self.open_bb();
        let idx = rng.random_range(..bb.num_ones());
        let idx = ith_one_u128(idx, bb);
        Some(UtttMove::new(UtttSquare::from_bb_idx(idx)))
    }

    fn random_pseudolegal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
        self.random_legal_move(rng)
    }

    fn make_move(mut self, mov: Self::Move) -> Option<Self> {
        let color = self.active;
        let square = mov.dest_square();
        let bb = ExtendedRawBitboard::single_piece_at(square.bb_idx());
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
        if !self.size().coordinates_valid(mov.dest_square()) {
            return false;
        }
        if !self.last_move.is_null() {
            let sub_board = self.last_move.dest_square().sub_square();
            if self.is_sub_board_open(sub_board) && mov.dest_square().sub_board() != sub_board {
                return false;
            }
        }
        self.is_open(mov.dest_square())
    }

    fn player_result_no_movegen<H: BoardHistory>(&self, _history: &H) -> Option<PlayerResult> {
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

    fn player_result_slow<H: BoardHistory>(&self, history: &H) -> Option<PlayerResult> {
        self.player_result_no_movegen(history)
    }

    fn no_moves_result(&self) -> Option<PlayerResult> {
        debug_assert!(self.open_bb().is_zero());
        Some(Draw)
    }

    fn can_reasonably_win(&self, _player: Self::Color) -> bool {
        true
    }

    fn hash_pos(&self) -> PosHash {
        // TODO: Test using a better hash
        let mut hasher = DefaultHasher::default();
        // the bitboards and the last move must be part of the hash, but not the ply
        let members = (&self.colors_internal, self.open, self.active, self.last_move);
        members.hash(&mut hasher);
        PosHash(hasher.finish())
    }

    // TODO: Don't use a separate open bitboard, just set both players' bitboards to one for squares that are no longer
    // reachable because the sub board has been won, and update the piece_on function

    fn read_fen_and_advance_input_for(
        input: &mut Tokens,
        strictness: Strictness,
        _settings: UtttSettings,
    ) -> Res<Self> {
        let mut pos = Self::default().into();
        read_common_fen_part::<UtttBoard>(input, &mut pos)?;

        let mut fullmove_counter = parse_int(input, "fullmove counter")?;
        if fullmove_counter == 0 {
            if strictness == Strict {
                bail!("The fullmove counter is one-based and can't be zero")
            } else {
                fullmove_counter = 1;
            }
        }
        let fullmoves = NonZeroUsize::new(fullmove_counter).unwrap();
        pos.0.ply_since_start = ply_counter_from_fullmove_nr(fullmoves, pos.0.active_player().is_first());
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

    fn as_diagram(&self, typ: CharType, flip: bool, mark_active: bool) -> String {
        board_to_string(self, UtttPiece::to_char, typ, flip, mark_active)
    }

    fn display_pretty(&self, fmt: &mut dyn BoardFormatter<Self>) -> String {
        display_board_pretty(self, fmt)
    }

    fn pretty_formatter(
        &self,
        piece_to_char: Option<CharType>,
        last_move: Option<UtttMove>,
        opts: OutputOpts,
    ) -> Box<dyn BoardFormatter<Self>> {
        let l = if self.last_move.is_null() { None } else { Some(self.last_move) };
        let last_move = last_move.or(l);
        let pos = *self;
        let formatter = AdaptFormatter {
            underlying: Box::new(DefaultBoardFormatter::new(*self, piece_to_char, last_move, opts)),
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
            square_width: None,
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
                bail!(
                    "At least one square is occupied by both players, the bitboards are {0} and {1}",
                    this.colors_internal[0],
                    this.colors_internal[1]
                );
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
                        bail!(
                            "The square '{sq}', on which the last move has been played, is occupied by the {} player, \
                        which is not the player active in the previous ply",
                            this.active
                        );
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
                let won_sub_board = UtttBoard::calculate_sub_board_won(this.sub_board(color, sub_board));
                if won_sub_board {
                    this.mark_as_won(sub_board, color);
                }
            }
        }
        let mut won_by_both = this.won_sub_boards(O) & this.won_sub_boards(X);
        if won_by_both.has_set_bit() {
            bail!("Sub board {0} has been won by both players", UtttSubSquare::from_bb_idx(won_by_both.pop_lsb()));
        }
        for color in UtttColor::iter() {
            let won_sub_boards = this.won_sub_boards(color);
            if UtttBoard::calculate_sub_board_won(won_sub_boards) {
                let sq = this.last_move.dest_square();
                if !won_sub_boards.is_bit_set_at(sq.sub_board().bb_idx())
                    || !UtttBoard::is_sub_board_won_at(this.sub_board(color, sq.sub_board()), sq.sub_square())
                {
                    bail!("The game is won for player {color}, but their last move (at {sq}) didn't win the game");
                }
            }
        }
        if checks == Assertion {
            assert!((this.open & !this.all_empty_squares_bb()).is_zero());
            for sub_board in UtttSubSquare::iter() {
                let won = this.is_sub_board_won(X, sub_board) || this.is_sub_board_won(O, sub_board);
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

    fn settings(&self) -> &UtttSettings {
        self.0.settings()
    }

    fn size(&self) -> UtttSize {
        self.0.size()
    }

    fn place_piece(&mut self, square: UtttSquare, piece: ColoredUtttPieceType) {
        let color = piece.color().unwrap();
        let bb = ExtendedRawBitboard::single_piece_at(square.bb_idx());
        self.0.colors_internal[color as usize] |= bb;
    }

    fn remove_piece(&mut self, square: UtttSquare) {
        let bb = ExtendedRawBitboard::single_piece_at(square.bb_idx());
        self.0.colors_internal[0] &= !bb;
        self.0.colors_internal[1] &= !bb;
        self.0.last_move = UtttMove::NULL;
        self.0.ply_since_start = 0;
    }

    fn piece_on(&self, coords: UtttSquare) -> UtttPiece {
        self.0.colored_piece_on(coords)
    }

    fn is_empty(&self, square: UtttSquare) -> bool {
        self.0.is_empty(square)
    }

    fn active_player(&self) -> UtttColor {
        self.0.active
    }

    fn set_active_player(&mut self, player: UtttColor) {
        self.0.active = player;
    }

    fn set_ply_since_start(&mut self, ply: usize) -> Res<()> {
        self.0.ply_since_start = ply;
        Ok(())
    }

    fn set_halfmove_repetition_clock(&mut self, _ply: usize) -> Res<()> {
        // ignored
        Ok(())
    }
}

impl UnverifiedUtttBoard {
    pub fn last_move_mut(&mut self) -> &mut UtttMove {
        &mut self.0.last_move
    }
}
