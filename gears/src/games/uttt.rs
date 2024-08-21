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
    GenericPiece, PieceType, Settings, ZobristHash,
};
use crate::general::bitboards::{
    Bitboard, DefaultBitboard, ExtendedRawBitboard, RawBitboard, RawStandardBitboard,
};
use crate::general::board::SelfChecks::*;
use crate::general::board::{
    board_to_string, common_fen_part, read_common_fen_part, Board, SelfChecks, UnverifiedBoard,
};
use crate::general::common::{ith_one_u128, parse_int, Res, StaticallyNamedEntity};
use crate::general::move_list::EagerNonAllocMoveList;
use crate::general::moves::Legality::Legal;
use crate::general::moves::{Legality, Move, NoMoveFlags, UntrustedMove};
use crate::general::squares::{RectangularCoordinates, SmallGridSize, SmallGridSquare};
use crate::PlayerResult;
use crate::PlayerResult::{Draw, Lose};
use colored::Colorize;
use rand::Rng;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Not;
use std::str::{FromStr, SplitWhitespace};
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

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, derive_more::Display, EnumIter)]
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
        self.uncolor().to_uncolored_idx()
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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
        if s == "0000" {
            return Ok(Self::NULL);
        }
        let square = UtttSquare::from_str(s)?;
        if !board.is_empty(square) {
            return Err(format!(
                "Square {square} is not empty, so this move is invalid"
            ));
        }
        Ok(Self(square))
    }

    fn from_extended_text(s: &str, board: &UtttBoard) -> Res<Self> {
        Self::from_compact_text(s, board)
    }

    fn from_usize_unchecked(val: usize) -> UntrustedMove<UtttBoard> {
        UntrustedMove::from_move(Self(UtttSquare::from_bb_idx(val)))
    }

    fn to_underlying(self) -> Self::Underlying {
        self.0.to_u8()
    }
}

pub type UtttMoveList = EagerNonAllocMoveList<UtttBoard, 81>;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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

    fn won_sub_board_bb(bb: ExtendedRawBitboard) -> UtttSubBitboard {
        UtttSubBitboard::new(RawStandardBitboard((bb >> 81).0 as u64))
    }

    pub fn won_sub_boards(&self, color: UtttColor) -> UtttSubBitboard {
        Self::won_sub_board_bb(self.colors_internal[color as usize])
    }

    // TODO: take this parameter and color, so that we don't mask twice: with BOARD_BB and SUB_BOARD_MASK
    pub fn sub_board(bb: RawUtttBitboard, sub_board: UtttSubSquare) -> UtttSubBitboard {
        let bb =
            RawStandardBitboard((bb >> (sub_board.bb_idx() * 9)).0 as u64) & Self::SUB_BOARD_MASK;
        UtttSubBitboard::new(bb)
    }

    pub fn sub_board_iter(self, color: UtttColor) -> impl Iterator<Item = UtttSubBitboard> {
        let bb = self.player_bb(color);
        (0..9).map(move |idx| Self::sub_board(bb, UtttSubSquare::from_bb_index(idx)))
    }

    pub fn is_sub_board_won_at(sub_board: UtttSubBitboard, square: UtttSubSquare) -> bool {
        let bb = sub_board.raw();
        const ROW_BB: RawStandardBitboard = RawStandardBitboard(0b111);
        const COLUMN_BB: RawStandardBitboard = RawStandardBitboard(0b100_100_100);
        const DIAG_BB: RawStandardBitboard = RawStandardBitboard(0b100_010_001);
        const ANTI_DIAG_BB: RawStandardBitboard = RawStandardBitboard(0b001_010_100);
        let row = square.row();
        let column = square.column();
        let row_bb = ROW_BB << (3 * square.row());
        let column_bb = COLUMN_BB << square.column();
        if bb & row_bb == row_bb {
            return true;
        }
        if bb & column_bb == column_bb {
            return true;
        }
        if row == column && bb & DIAG_BB == DIAG_BB {
            return true;
        }
        if row == 2 - column && bb & ANTI_DIAG_BB == ANTI_DIAG_BB {
            return true;
        }
        false
    }

    fn mark_as_won(&mut self, sub_board: UtttSubSquare, color: UtttColor) {
        self.colors_internal[color as usize] |=
            RawUtttBitboard::single_piece(81 + sub_board.bb_idx());
        self.open &= !ExtendedRawBitboard::from_primitive(
            (Self::SUB_BOARD_MASK.to_primitive() as u128) << sub_board.bb_idx() * 9,
        );
    }

    fn update_won_bb(&mut self, square: UtttSquare, color: UtttColor) {
        let sub_board = Self::sub_board(self.colors_internal[color as usize], square.sub_board());
        if Self::is_sub_board_won_at(sub_board, square.sub_square()) {
            self.mark_as_won(square.sub_board(), color);
        }
    }

    // Ideally, this would be an associated const, but Rust's restrictions around const fns would make that ugly.
    // But hopefully, the compiler will constant fold this anyway
    fn won_masks() -> [UtttSubBitboard; 8] {
        [
            UtttSubBitboard::file_no(0) & UtttSubBitboard::new(Self::SUB_BOARD_MASK),
            UtttSubBitboard::file_no(1) & UtttSubBitboard::new(Self::SUB_BOARD_MASK),
            UtttSubBitboard::file_no(2) & UtttSubBitboard::new(Self::SUB_BOARD_MASK),
            UtttSubBitboard::rank_no(0) & UtttSubBitboard::new(Self::SUB_BOARD_MASK),
            UtttSubBitboard::rank_no(1) & UtttSubBitboard::new(Self::SUB_BOARD_MASK),
            UtttSubBitboard::rank_no(2) & UtttSubBitboard::new(Self::SUB_BOARD_MASK),
            UtttSubBitboard::diagonal(UtttSubSquare::from_rank_file(1, 1))
                & UtttSubBitboard::new(Self::SUB_BOARD_MASK),
            UtttSubBitboard::anti_diagonal(UtttSubSquare::from_rank_file(1, 1))
                & UtttSubBitboard::new(Self::SUB_BOARD_MASK),
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
        Self::won_sub_board_bb(self.player_bb(color)).is_bit_set_at(sub_board.bb_idx())
    }

    pub fn is_sub_board_open(self, sub_board: UtttSubSquare) -> bool {
        Self::sub_board(self.open, sub_board).has_set_bit()
    }

    pub fn read_alternative_fen(fen: &str) -> Res<Self> {
        if fen.len() != 81 || fen.contains(|c: char| ![' ', 'x', 'o', 'X', 'O'].contains(&c)) {
            return Err(format!(
                "Incorrect alternative UTTT FEN '{}', must consist of exactly 81 chars, all of which must be ' ', 'x', 'o', 'X', or 'O'",fen.red()
            ));
        }
        let mut board = UnverifiedUtttBoard::new(Self::empty());
        for (idx, c) in fen.chars().enumerate() {
            if c == ' ' {
                continue;
            }
            let symbol = ColoredUtttPieceType::from_ascii_char(c).unwrap();
            let square = UtttSquare::from_bb_idx(idx).flip_up_down(UtttSize::default());
            debug_assert!(board.check_coordinates(square).is_ok());
            board = board.place_piece_unchecked(square, symbol);
            if c.is_uppercase() {
                let mov = board.last_move_mut();
                if *mov != UtttMove::NULL {
                    return Err(format!("Upper case pieces are used for the last move, but there is more than one upper case letter in '{}'", fen.red()));
                }
                *mov = UtttMove::new(square);
            }
        }
        println!("{}", board.0);
        println!("{}", board.0.as_ascii_diagram(false));
        board.verify_with_level(CheckFen)
    }
}

impl Display for UtttBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_fen())
    }
}

// TODO: Bench positions

impl Board for UtttBoard {
    type EmptyRes = UtttBoard;
    type Settings = UtttSettings;
    type Coordinates = UtttSquare;
    type Color = UtttColor;
    type Piece = UtttPiece;
    type Move = UtttMove;
    type MoveList = UtttMoveList;
    type LegalMoveList = Self::MoveList;
    type Unverified = UnverifiedUtttBoard;

    fn empty_for_settings(_settings: UtttSettings) -> Self {
        Self::default()
    }

    fn startpos_for_settings(_settings: UtttSettings) -> Self {
        Self::default()
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
        self.halfmove_ctr_since_start()
    }

    fn size(&self) -> UtttSize {
        UtttSize::default()
    }

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

    fn pseudolegal_moves(&self) -> Self::MoveList {
        // don't assume that the board is empty in startpos to support different starting positions
        let mut res = UtttMoveList::default();
        if self.last_move != UtttMove::NULL {
            let sub_board = self.last_move.dest_square().sub_square();
            let sub_bitboard = Self::sub_board(self.all_empty_squares_bb(), sub_board);
            println!("{}", sub_board);
            println!("{}", sub_bitboard);
            println!(
                "{}",
                Self::sub_board(self.all_empty_squares_bb(), sub_board)
            );
            println!("{}", Self::sub_board(self.colors_internal[0], sub_board));
            println!("{}", Self::sub_board(self.colors_internal[1], sub_board));
            println!("{}", self.as_ascii_diagram(false));

            if self.is_sub_board_open(sub_board) {
                let sub_bitboard = Self::sub_board(self.open_bb(), sub_board);
                for idx in sub_bitboard.one_indices() {
                    let square = UtttSquare::new(sub_board, UtttSubSquare::from_bb_index(idx));
                    res.push(UtttMove::new(square));
                }
                return res;
            }
        }
        for sq in self.open_bb().one_indices() {
            res.push(UtttMove::new(UtttSquare::from_bb_idx(sq)));
        }
        res
    }

    fn tactical_pseudolegal(&self) -> Self::MoveList {
        // TODO: Test considering moves that win a sub-board as tactical
        UtttMoveList::default()
    }

    fn random_legal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
        let empty = self.all_empty_squares_bb();
        if empty.is_zero() {
            return None;
        }
        let idx = rng.gen_range(0..empty.num_ones());
        let idx = ith_one_u128(idx, empty.0);
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
        self.is_empty(mov.dest_square())
    }

    fn player_result_no_movegen<H: BoardHistory<Self>>(
        &self,
        _history: &H,
    ) -> Option<PlayerResult> {
        if self.last_move == UtttMove::NULL {
            return None;
        }
        let sq = self.last_move.dest_square().sub_board();
        let bb = Self::won_sub_board_bb(self.player_bb(!self.active_player()));
        if Self::is_sub_board_won_at(bb, sq) {
            Some(Lose)
        } else if self.all_empty_squares_bb().is_zero() {
            Some(Draw) // technically, this doesn't need to be checked here, but it's cheap, so we might as well
        } else {
            None
        }
    }

    fn player_result_slow<H: BoardHistory<Self>>(&self, history: &H) -> Option<PlayerResult> {
        self.player_result_no_movegen(history)
    }

    fn no_moves_result(&self) -> PlayerResult {
        debug_assert!(self.all_empty_squares_bb().is_zero());
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
        let mut res = common_fen_part(self);
        write!(&mut res, " {}", self.last_move).unwrap();
        res
    }

    fn read_fen_and_advance_input(input: &mut SplitWhitespace) -> Res<Self> {
        let pos = Self::default();
        let mut pos = read_common_fen_part::<UtttBoard>(input, pos.into())?;

        pos.0.ply_since_start = parse_int(input, "ply number")?;
        let last_move = input.next().ok_or_else(|| {
            "Ultimate Tic-Tac-Toe FEN ends after ply counter, missing the last move".to_string()
        })?;
        // Use an empty board for parsing the last move instead of the current board because that would complain about the last
        // move being invalid because the square is already occupied.
        let last_move = UtttMove::from_compact_text(last_move, &Self::default())?;
        pos.0.last_move = last_move;
        // The won sub boards bitboard is set in the verify method
        pos.verify_with_level(CheckFen)
    }

    fn as_ascii_diagram(&self, flip: bool) -> String {
        board_to_string(self, UtttPiece::to_ascii_char, flip)
    }

    fn as_unicode_diagram(&self, flip: bool) -> String {
        board_to_string(self, UtttPiece::to_utf8_char, flip)
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
    fn verify_with_level(self, checks: SelfChecks) -> Res<UtttBoard> {
        let mut this = self.0;
        if checks != CheckFen {
            for color in UtttColor::iter() {
                let bb = this.colors_internal[color as usize];
                if (bb >> (81 + 9)).has_set_bit() {
                    return Err(format!("The {color} bitboard contains a set bit above the range of used bits, the bitboard is {bb}"));
                }
            }
            if (this.colors_internal[0] & this.colors_internal[1]).has_set_bit() {
                return Err(format!("At least one square is occupied by both players, the bitboards are {0} and {1}", this.colors_internal[0], this.colors_internal[1]));
            }
        }
        if this.last_move != UtttMove::NULL {
            let sq = this.last_move.dest_square();
            match this.colored_piece_on(sq).color() {
                None => {
                    return Err(format!(
                        "The square '{sq}', on which the last move has been played, is empty"
                    ))
                }
                Some(col) => {
                    if col == this.active {
                        return Err(format!("The square '{sq}', on which the last move has been played, is occupied by the {} player, \
                        which is not the player active in the previous ply", this.active));
                    }
                }
            }
        }
        // Allow starting positions with squares already filled out, so the ply and the number of nonempty squares don't have to match.
        // But the ply number still has to be at most the number of nonempty squares
        if this.ply_since_start > this.occupied_bb().num_ones() {
            return Err(format!(
                "The ply number is '{0}', but only {1} pieces have been placed so far",
                this.ply_since_start,
                this.occupied_bb().num_ones()
            ));
        }
        for color in UtttColor::iter() {
            // reset the metadata because it's out of date
            this.colors_internal[color as usize] &= UtttBoard::BOARD_BB;
            for sub_board in UtttSubSquare::iter() {
                let won_sub_board = UtttBoard::calculate_sub_board_won(UtttBoard::sub_board(
                    this.player_bb(color),
                    sub_board,
                ));
                if won_sub_board {
                    this.mark_as_won(sub_board, color);
                }
            }
        }
        let mut won_by_both = this.won_sub_boards(O) & this.won_sub_boards(X);
        if won_by_both.has_set_bit() {
            return Err(format!(
                "Sub board {0} has been won by both players",
                UtttSubSquare::from_bb_index(won_by_both.pop_lsb())
            ));
        }
        for color in UtttColor::iter() {
            let won_sub_boards = this.won_sub_boards(color);
            if UtttBoard::calculate_sub_board_won(won_sub_boards) {
                let sq = this.last_move.dest_square();
                if !won_sub_boards.is_bit_set_at(sq.sub_board().bb_idx())
                    || !UtttBoard::is_sub_board_won_at(
                        UtttBoard::sub_board(this.player_bb(color), sq.sub_board()),
                        sq.sub_square(),
                    )
                {
                    return Err(format!("The game is won for player {color}, but their last move (at {sq}) didn't win the game"));
                }
            }
        }
        if checks == Assertion {
            for sub_board in UtttSubSquare::iter() {
                let won = this.is_sub_board_won(X, sub_board) ^ this.is_sub_board_won(O, sub_board);
                if UtttBoard::sub_board(this.occupied_bb(), sub_board).raw()
                    == UtttBoard::SUB_BOARD_MASK
                {
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
        self.0.open &= !bb;
        self.0.update_won_bb(square, color);
        self
    }

    fn remove_piece_unchecked(mut self, square: UtttSquare) -> Self {
        let bb = ExtendedRawBitboard::single_piece(square.bb_idx());
        self.0.colors_internal[0] &= !bb;
        self.0.colors_internal[1] &= !bb;
        self.0.open |= bb;
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
