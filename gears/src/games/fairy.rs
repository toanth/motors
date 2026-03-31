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
mod algebraic_notation;
mod attacks;
mod config;
mod effects;
pub mod moves;
mod perft_tests;
mod piece_builder;
pub mod pieces;
mod rules;
#[cfg(test)]
mod tests;

use crate::PlayerResult;
use crate::games::CharType::Ascii;
use crate::games::fairy::moves::Move;
use crate::games::fairy::pieces::{ColoredPieceId, PieceId};
use crate::games::fairy::rules::{FenHandInfo, MoveNumFmt, RulesBuilder};
use crate::games::fairy::rules::{GameEndEager, NumRoyals, Rules, RulesRef};
use crate::games::{
    AbstractPieceType, BoardHistory, CharType, ColorTrait, ColoredPieceTrait, ColoredPieceTypeTrait, DimT,
    GenericPiece, NUM_COLORS, NoHistory, PosHash, SizeTrait,
};
use crate::general::bitboards::{BitboardTrait, DynamicallySizedBitboard, ExtendedRawBitboard, RawBitboardTrait};
use crate::general::board::SelfChecks::CheckFen;
use crate::general::board::Strictness::{Relaxed, Strict};
use crate::general::board::{
    AxesFormat, BBSelect, BitboardBoard, BoardHelpers, BoardSize, BoardTrait, ColPieceTypeOf, NameToPos, PieceTypeOf,
    SelfChecks, Strictness, Symmetry, UnverifiedBoardTrait, default_bitboards_from_name, position_fen_part,
    read_common_fen_part, read_halfmove_clock, read_move_number_in_ply, read_single_move_number,
};
use crate::general::common::Description::NoDescription;
use crate::general::common::{
    EntityList, GenericSelect, Res, SimpleSelect, StaticallyNamedEntity, Tokens, parse_int_from_str,
    select_name_static, tokens,
};
use crate::general::move_list::{MoveListTrait, SboMoveList};
use crate::general::moves::MoveTrait;
use crate::general::squares::{GridCoordinates, GridSize, RectangularCoordinates, RectangularSize, SquareColor};
use crate::output::OutputOpts;
use crate::output::text_output::{BoardFormatter, DefaultBoardFormatter, board_to_string, display_board_pretty};
use crate::search::DepthPly;
use crate::ugi::Protocol;
use anyhow::{anyhow, bail, ensure};
use arbitrary::Arbitrary;
use colored::Colorize;
use itertools::Itertools;
use rand::Rng;
use rand::prelude::IndexedRandom;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::{Deref, Index, IndexMut, Not};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, FromRepr};

// Using a 64 bit bitboard makes chess perft twice as fast, but obviously doesn't work for larger boards
type RawBitboard = ExtendedRawBitboard;
type Bitboard = DynamicallySizedBitboard<RawBitboard, Square>;

/// There can never be more than 32 piece types in a given game
/// (For chess, the number would be 6; for ataxx, 1).
/// Note that some effects can also be represented by one of these bitboards.
const MAX_NUM_PIECE_TYPES: usize = 16;

pub type Square = GridCoordinates;
pub type Size = GridSize;

/// If there are more moves than this, the move list allocates
const MAX_NO_ALLOC_MOVES: usize = 250; // should result in a SmallVec of size <= 1024

type MoveList = SboMoveList<Board, MAX_NO_ALLOC_MOVES>;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub struct Color(bool);

impl Color {
    pub fn from_idx(idx: usize) -> Self {
        Self(idx != 0)
    }
}

impl ColorTrait for Color {
    type Board = Board;

    fn second() -> Self {
        Self(true)
    }

    fn to_char(self, settings: &Rules) -> char {
        settings.colors[self].ascii_char
    }

    #[allow(refining_impl_trait)]
    fn name(self, settings: &Rules) -> &str {
        &settings.colors[self].name
    }
}

impl Not for Color {
    type Output = Self;
    fn not(self) -> Self {
        self.other()
    }
}

impl<T> Index<Color> for [T; 2] {
    type Output = T;

    fn index(&self, color: Color) -> &Self::Output {
        &self[color.0 as usize]
    }
}

impl<T> IndexMut<Color> for [T; 2] {
    fn index_mut(&mut self, color: Color) -> &mut Self::Output {
        &mut self[color.0 as usize]
    }
}

#[derive(Debug, Eq, PartialEq, Arbitrary)]
#[must_use]
struct ColorInfo {
    ascii_char: char,
    name: String,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, EnumIter, derive_more::Display, FromRepr, Arbitrary)]
#[must_use]
pub enum Side {
    Kingside,
    Queenside,
}

// TODO: Some of this can be moved to Rules
#[derive(Debug, Copy, Clone, Eq, Arbitrary)]
#[must_use]
struct CastlingMoveInfo {
    rook_file: DimT,
    king_dest_file: DimT,
    rook_dest_file: DimT,
    // standard FENs and chess960 FENs use different chars, and we want the FEN char to be preserved during a roundtrip
    // TODO: remove and replace with a single bool for the entire castling info
    fen_char: u8,
}

impl PartialEq for CastlingMoveInfo {
    fn eq(&self, other: &Self) -> bool {
        // don't compare fen_char
        self.rook_file == other.rook_file
            && self.king_dest_file == other.king_dest_file
            && self.rook_dest_file == other.rook_dest_file
    }
}

impl Hash for CastlingMoveInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.rook_file, self.rook_dest_file, self.king_dest_file).hash(state);
    }
}

impl ColoredFairyCastleInfo {
    fn king_dest_sq(self, side: Side) -> Option<Square> {
        self.sides[side as usize].map(|info| Square::from_rank_file(self.rank, info.king_dest_file))
    }
    fn rook_dest_sq(self, side: Side) -> Option<Square> {
        self.sides[side as usize].map(|info| Square::from_rank_file(self.rank, info.rook_dest_file))
    }
    fn rook_sq(self, side: Side) -> Option<Square> {
        self.sides[side as usize].map(|info| Square::from_rank_file(self.rank, info.rook_file))
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
struct ColoredFairyCastleInfo {
    sides: [Option<CastlingMoveInfo>; 2],
    rank: DimT,
}

// Stored inside the board.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
struct FairyCastleInfo {
    players: [ColoredFairyCastleInfo; NUM_COLORS],
}

impl Default for FairyCastleInfo {
    fn default() -> Self {
        Self {
            players: [
                ColoredFairyCastleInfo { sides: [None, None], rank: 0 },
                ColoredFairyCastleInfo { sides: [None, None], rank: 7 },
            ],
        }
    }
}

impl FairyCastleInfo {
    fn new(size: Size) -> Self {
        let mut res = Self::default();
        res.players[1].rank = size.height.0 - 1;
        res
    }
    fn player(&self, color: Color) -> &ColoredFairyCastleInfo {
        &self.players[color]
    }
    fn info(&self, color: Color, side: Side) -> Option<CastlingMoveInfo> {
        self.player(color).sides[side as usize]
    }
    pub fn can_castle(&self, color: Color, side: Side) -> bool {
        self.info(color, side).is_some()
    }
    pub fn unset(&mut self, color: Color, side: Side) {
        self.players[color].sides[side as usize] = None;
    }
    pub fn unset_both_sides(&mut self, color: Color) {
        self.unset(color, Side::Queenside);
        self.unset(color, Side::Kingside);
    }
    pub fn write_fen_part(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, " ")?;
        let mut can_castle = false;
        for color in Color::iter() {
            for side in Side::iter() {
                if let Some(info) = self.info(color, side) {
                    can_castle = true;
                    write!(f, "{}", info.fen_char as char)?;
                }
            }
        }
        if !can_castle {
            write!(f, "-")?;
        }
        Ok(())
    }
    fn king_dest_squares(&self, size: Size) -> Bitboard {
        let to_bb = |col: Color, side: Side| -> Bitboard {
            self.players[col]
                .king_dest_sq(side)
                .map(|sq| Bitboard::single_piece_for(sq, size))
                .unwrap_or(Bitboard::new(0, size))
        };
        to_bb(Color::first(), Side::Queenside)
            | to_bb(Color::first(), Side::Kingside)
            | to_bb(Color::second(), Side::Queenside)
            | to_bb(Color::second(), Side::Kingside)
    }
}

type AdditionalCtrT = i32;

/// A FairyBoard is a rectangular board for a chess-like variant.
#[derive(Debug, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub struct UnverifiedBoard {
    // unfortunately, ArrayVec isn't `Copy`
    piece_bitboards: [RawBitboard; MAX_NUM_PIECE_TYPES],
    color_bitboards: [RawBitboard; NUM_COLORS],
    neutral_bb: RawBitboard,
    // bb of all valid squares
    mask_bb: RawBitboard,
    // for each piece type, how many the player has available to drop
    in_hand: [[u8; MAX_NUM_PIECE_TYPES]; NUM_COLORS],
    ply_since_start: usize,
    // like the 50mr counter in chess TODO: Maybe make it count down?
    num_piece_bitboards: usize,
    draw_counter: usize,
    in_check: [bool; NUM_COLORS],
    // Their meaning depends on the variant. For example, this counts checks in 3check.
    additional_ctrs: [AdditionalCtrT; NUM_COLORS],
    active: Color,
    castling_info: FairyCastleInfo,
    ep: Option<Square>,
    last_move: Move,
    hash: PosHash,
    rules: RulesRef,
}

impl Default for UnverifiedBoard {
    fn default() -> Self {
        let rules = RulesRef::default();
        rules.empty_pos()
    }
}

impl UnverifiedBoard {
    fn occupied_bb(&self) -> Bitboard {
        Bitboard::new(self.color_bitboards[0] | self.color_bitboards[1] | self.neutral_bb, self.size())
    }

    fn either_player_bb(&self) -> Bitboard {
        Bitboard::new(self.color_bitboards[0] | self.color_bitboards[1], self.size())
    }

    fn rules(&self) -> &Rules {
        self.rules.get()
    }

    fn color_name(&self, color: Color) -> &str {
        &self.rules().colors[color].name
    }
}

impl From<Board> for UnverifiedBoard {
    fn from(value: Board) -> Self {
        value.0
    }
}

impl UnverifiedBoardTrait<Board> for UnverifiedBoard {
    fn verify_with_level(self, level: SelfChecks, strictness: Strictness) -> Res<Board> {
        let size = self.size();
        let rules = self.rules();
        if size != rules.size {
            bail!("Incorrect size: Is {size} and should be {}", rules.size)
        }
        if self.num_piece_bitboards != rules.pieces.len() {
            bail!(
                "The number of piece bitboard ({0}) does not match the number of pieces ({1})",
                self.num_piece_bitboards,
                rules.pieces.len()
            )
        }
        let mut pieces = RawBitboard::default();
        for (id, _piece) in rules.pieces.iter().enumerate() {
            let bb = self.piece_bitboards[id];
            if (bb & pieces).has_any() {
                bail!("Two pieces on the same square")
            }
            pieces |= bb;
        }
        ensure!(
            (self.color_bitboards[0] & self.color_bitboards[1]).is_zero(),
            "Both players have overlapping bitboards: {0} and {1}",
            self.color_bitboards[0],
            self.color_bitboards[1]
        );
        ensure!(
            ((self.color_bitboards[0] | self.color_bitboards[1]) & self.neutral_bb).is_zero(),
            "Player bitboards and neutral bitboard overlap: {0} and {1}",
            self.color_bitboards[0] | self.color_bitboards[1],
            self.neutral_bb
        );
        let colors = self.color_bitboards[0] | self.color_bitboards[1] | self.neutral_bb;
        ensure!(
            (pieces & !colors).is_zero(),
            "Internal bitboard mismatch: A piece doesn't have a color and isn't neutral: Pieces: {pieces}, colors and neutral: {colors}",
        );
        if strictness == Strict {
            let ctr = self.draw_counter.saturating_sub(1);
            if rules.game_end_eager.iter().any(|(cond, _)| {
                if let GameEndEager::DrawCounter(val) = cond { val.satisfied(ctr, &self) } else { false }
            }) {
                bail!("Invalid draw progress counter: '{0}' is too large", self.draw_counter);
            }
        }
        if self.ply_since_start >= usize::MAX / 2 {
            bail!("Ridiculously large ply counter ({})", self.ply_since_start)
        }

        for color in Color::iter() {
            for side in Side::iter() {
                if !self.castling_info.can_castle(color, side) {
                    continue;
                }
                let castling = self.castling_info.player(color);
                if let Some(rook_sq) = castling.rook_sq(side)
                    && self.is_empty(rook_sq)
                {
                    bail!(
                        "Color {0} can castle {side}, but there is no piece to castle with{1}",
                        self.color_name(color),
                        if level == CheckFen { " (invalid castling flag in FEN?)" } else { "" }
                    );
                }
            }
        }
        if self.ep.is_some() && !rules.has_ep {
            bail!("The ep square is set even though the rules don't mention en passant")
        }
        for color in Color::iter() {
            let royals = self.royal_bb_for(color);
            let num = royals.num_ones();
            match rules.num_royals[color] {
                NumRoyals::Exactly(n) => {
                    ensure!(
                        num == n,
                        "The {0} player must have exactly {n} royal pieces, but has {num}",
                        self.color_name(color)
                    )
                }
                NumRoyals::AtLeast(n) => {
                    ensure!(
                        num >= n,
                        "The {0} player must have at least {n} royal pieces, but has {num}",
                        self.color_name(color)
                    )
                }
                NumRoyals::BetweenInclusive(min, max) => {
                    ensure!(
                        (min..=max).contains(&num),
                        "The {0} player must have between {min} and {max} royal pieces, but has {num}",
                        self.color_name(color)
                    )
                }
            }
        }

        let mut res = Board(self);
        res.0.hash = res.compute_hash();
        for c in Color::iter() {
            res.0.in_check[c] = res.compute_is_in_check(c);
        }
        ensure!(
            res.rules().check_rules.active_check_ok.satisfied(&res, res.active_player()),
            "Player {} is in check, but that's not allowed in this position",
            res.rules().colors[res.active_player()].name
        );
        ensure!(
            res.rules().check_rules.inactive_check_ok.satisfied(&res, res.inactive_player()),
            "Player {} is in check, but that's not allowed in this position (it's not their turn to move)",
            res.rules().colors[res.inactive_player()].name
        );

        Ok(res)
    }

    fn settings(&self) -> &Rules {
        self.rules.get()
    }

    fn name(&self) -> String {
        self.settings().name.clone()
    }

    fn size(&self) -> BoardSize<Board> {
        self.rules().size
    }

    fn place_piece(&mut self, coords: Square, piece: ColPieceTypeOf<Board>) {
        let bb = self.single_piece(coords).raw();
        self.piece_bitboards[piece.to_uncolored_idx()] |= bb;
        if let Some(color) = piece.color() {
            self.color_bitboards[color] |= bb;
        } else {
            self.neutral_bb |= bb;
        }
    }

    fn remove_piece(&mut self, sq: Square) {
        let piece = self.piece_on(sq).symbol;
        self.remove_piece_impl(sq, piece);
        for col in Color::iter() {
            for side in Side::iter() {
                if Some(sq) == self.castling_info.players[col].rook_sq(side) {
                    self.castling_info.unset(col, side);
                }
            }
            if Some(sq) == self.king_square(col) {
                self.castling_info.unset_both_sides(col);
            }
        }
        // just always remove the ep square
        self.ep = None;
    }

    fn piece_on(&self, coords: Square) -> <Board as BoardTrait>::Piece {
        let idx = self.idx(coords);
        let piece = self
            .piece_bitboards
            .iter()
            .find_position(|bb| bb.is_bit_set_at(idx))
            .map(|(idx, _bb)| PieceId::new(idx))
            .unwrap_or(PieceId::empty());
        let color = self
            .color_bitboards
            .iter()
            .find_position(|bb| bb.is_bit_set_at(idx))
            .map(|(idx, _bb)| Color::from_idx(idx));

        GenericPiece::new(ColoredPieceId::create(piece, color), coords)
    }

    fn is_empty(&self, coords: Square) -> bool {
        !self.occupied_bb().is_bit_set_at(self.idx(coords))
    }

    fn active_player(&self) -> Color {
        self.active
    }

    fn set_active_player(&mut self, player: Color) {
        self.active = player;
    }

    fn set_ply_since_start(&mut self, ply: usize) -> Res<()> {
        self.ply_since_start = ply;
        Ok(())
    }

    fn set_halfmove_repetition_clock(&mut self, ply: usize) -> Res<()> {
        self.draw_counter = ply;
        Ok(())
    }

    fn fen_pos_part_contains_hand(&self) -> bool {
        self.rules().format_rules.hand == FenHandInfo::InBrackets
    }

    fn read_fen_hand_part(&mut self, input: &str) -> Res<()> {
        ensure!(
            self.rules().format_rules.hand != FenHandInfo::None,
            "The variant '{0}' does not contain hand info in the FEN, so '{1}' can't be parsed as hand info",
            self.rules().name,
            input.red()
        );
        if input == "-" {
            return Ok(());
        }
        let mut digit_start = None;
        for (i, c) in input.char_indices() {
            if c.is_ascii_digit() {
                digit_start = Some(digit_start.unwrap_or(i));
                continue;
            }
            let Some(piece) = ColoredPieceId::from_char(c, self.rules()) else {
                bail!(
                    "Expected a fairy piece, but '{0}' is not a valid FEN piece character for the current variant ({1})",
                    c.to_string().bold(),
                    self.rules().name.bold()
                )
            };
            let num = if let Some(digits) = digit_start { input[digits..i].parse::<u8>()? } else { 1 };
            digit_start = None;
            let val = &mut self.in_hand[piece.color().unwrap()][piece.to_uncolored_idx()];
            let Some(new_val) = val.checked_add(num) else {
                bail!(
                    "Too many pieces of type '{0}': A player can only have at most 255 pieces in hand, not {1}",
                    piece.name(self.rules.clone().get()).bold(),
                    (*val as usize + num as usize).to_string().red()
                )
            };
            *val = new_val;
        }
        if digit_start.is_some() {
            bail!("FEN hand description '{}' ends with a number", input.red())
        }
        Ok(())
    }
}

impl UnverifiedBoard {
    fn idx(&self, square: Square) -> usize {
        self.size().internal_key(square)
    }
    fn single_piece(&self, square: Square) -> Bitboard {
        Bitboard::new(RawBitboard::single_piece_at(self.idx(square)), self.size())
    }
    // doesn't affect the neutral bitboard (todo: change?)
    fn remove_piece_impl(&mut self, square: Square, piece: ColoredPieceId) {
        let bb = self.single_piece(square).raw();
        if let Some(col) = piece.color() {
            self.color_bitboards[col] ^= bb;
        }
        self.piece_bitboards[piece.uncolor().val()] ^= bb;
    }
    // adds or removes a given piece at a given square
    fn xor_given_piece_at(&mut self, square: Square, piece: PieceId, color: Color) {
        let idx = self.idx(square);
        let bb = RawBitboard::single_piece_at(idx);
        debug_assert_eq!(
            self.piece_bitboards[piece.val()].is_bit_set_at(idx),
            self.color_bitboards[color].is_bit_set_at(idx)
        );
        self.color_bitboards[color] ^= bb;
        self.piece_bitboards[piece.val()] ^= bb;
    }
    // doesn't affect the neutral bitboard
    fn remove_all_pieces(&mut self, bb: RawBitboard) {
        let mask = !bb;
        for bb in self.piece_bitboards.iter_mut() {
            *bb &= mask;
        }
        self.color_bitboards[0] &= mask;
        self.color_bitboards[1] &= mask;
    }
    fn compute_hash(&self) -> PosHash {
        let mut hasher = DefaultHasher::default();
        // unfortunately, this has the potential to get out of date when new fields are added
        let tuple = (
            &self.piece_bitboards,
            &self.color_bitboards,
            self.mask_bb,
            self.in_hand,
            self.num_piece_bitboards,
            self.active,
            self.castling_info,
            self.ep,
        );
        tuple.hash(&mut hasher);
        if self.rules().store_last_move {
            self.last_move.hash(&mut hasher);
        }
        PosHash(hasher.finish())
    }

    fn read_ctrs(&mut self, words: &mut Tokens, first: bool) -> Res<()> {
        let ctrs = words.peek().copied().unwrap_or_default();
        if self.rules().num_additional_ctrs() == 1 {
            let ctr = if ctrs == "-" { 0 } else { parse_int_from_str(ctrs, "additional counter")? };
            _ = words.next();
            self.additional_ctrs[0] = ctr;
            return Ok(());
        }
        let Some((c1, c2)) = ctrs.trim_start_matches('+').split_once('+') else {
            bail!("Expected two counters of the form '{0}', got '{1}'", "<number>+<number>".bold(), ctrs.red())
        };
        let mut ctr1 = parse_int_from_str(c1, "counter 1")?;
        let mut ctr2 = parse_int_from_str(c2, "counter 2")?;
        if first {
            ctr1 = self.rules().ctr_threshold[0].unwrap_or_default() - ctr1;
            ctr2 = self.rules().ctr_threshold[1].unwrap_or_default() - ctr2;
        }
        self.additional_ctrs[0] = ctr1;
        self.additional_ctrs[1] = ctr2;
        _ = words.next();
        Ok(())
    }

    fn write_fen_hand_part(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let rules = self.rules();
        if rules.format_rules.hand == FenHandInfo::None {
            return Ok(());
        }
        let mut empty = true;
        for color in Color::iter() {
            for (id, piece) in rules.pieces().rev() {
                let mut count = self.in_hand[color][id.val()];
                if count > 0 {
                    empty = false;
                }
                if !self.fen_pos_part_contains_hand() && count > 1 {
                    write!(f, "{count}{}", piece.player_symbol[color][Ascii])?;
                } else {
                    while count > 0 {
                        write!(f, "{}", piece.player_symbol[color][Ascii])?;
                        count -= 1;
                    }
                }
            }
        }
        if empty && rules.format_rules.hand == FenHandInfo::SeparateToken {
            write!(f, "-")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub struct Board(UnverifiedBoard);

impl Default for Board {
    fn default() -> Self {
        Self::startpos()
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.rules().format_rules.write_rules_part(f, &self.rules().name)?;
        write!(f, "{}", NoRulesFenFormatter(self))
    }
}

impl StaticallyNamedEntity for Board {
    fn static_short_name() -> impl Display
    where
        Self: Sized,
    {
        "fairy"
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        "Fairy Chess Variant".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "One of many variants of chess".to_string()
    }
}

type Piece = GenericPiece<Board, ColoredPieceId>;

impl BoardTrait for Board {
    type EmptyRes = Self::Unverified;
    type RawBitboard = RawBitboard;
    type Settings = Rules;
    type SettingsRef = RulesRef;
    type Coordinates = Square;
    type Color = Color;
    type Piece = Piece;
    type Move = Move;
    type MoveList = MoveList;
    type Unverified = UnverifiedBoard;

    fn empty_for_settings(settings: RulesRef) -> Self::Unverified {
        settings.empty_pos()
    }

    fn startpos_for_settings(settings: RulesRef) -> Self {
        let fen = settings.get().format_rules.startpos_fen.clone();
        Self::from_fen_for_rules(&fen, Strict, settings).unwrap()
    }

    fn name_to_pos_map() -> EntityList<NameToPos> {
        // TODO: add more named positions
        vec![
            NameToPos::desc(
                "kiwipete",
                "chess r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                "Classical chess test position",
            ),
            NameToPos::desc(
                "large_mnk",
                "mnk 11 11 4 11/11/11/11/11/11/11/11/11/11/11 x 1",
                "Starting position of a large m,n,k board",
            ),
            NameToPos::desc(
                "shogi_usi_startpos",
                "shogi lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1",
                "Shogi startpos, using USI notation instead of UGI notation",
            ),
        ]
    }

    fn bench_positions() -> impl IntoIterator<Item = Self> {
        let fens = &[
            "chess 4r1Q1/B2nr3/5b2/8/4p3/4KbNq/ppppppp1/RR3Nkn w - - 0 1",
            "3check r2qk2r/1bpp1ppp/ppnb1n2/1B2p3/2NPP3/2P2N1P/PP3PP1/R1BQR1K1 w kq - 0 11 +0+0",
            "kingofthehill r1b2b1r/4kppp/p1np1n2/1pp5/4PP2/2N1BN2/PPP2KPP/R4B1R b - - 1 11",
            "shogi 6snl/2+P1k1g2/p1+Bppp1p1/4l1p1p/1p3S1P1/3+b2Pl1/PP3PK1P/7R1/5+n1NL b 2GN2Prg2sp 1",
            "atomic rnb1kb1r/pp2pppp/1qp5/1B1p4/4P3/2N5/PPPP1PPP/R1B1K2R w KQkq - 3 7",
            "antichess rnbqk1nr/pppp1pQp/8/8/8/8/PbP1PPPP/RNB1KBNR w - - 0 5",
            "shatranj 8/p2k4/1pN4p/1Ppp1p2/5Rb1/PNPP2n1/5R2/1KB1r2r b - - 1 38",
            "makruk 7r/3k4/1nsm1pp1/n1p5/4RM1N/P1S5/1NK5/3R4 b - - 0 38",
            "ataxx x5o/1x4o/3-3/2---2/3-x2/1o3x1/o5x o - - 0 3",
        ];
        fens.iter()
            .map(|fen| Self::from_fen(fen, Strict).unwrap())
            .chain(Self::name_to_pos_map().into_iter().map(|n| Self::from_name(n.name).unwrap()))
    }

    // TODO: We could at least pass settings and do `startpos_for_setting()`, but ideally we'd also randomize the settings.
    // We could generate random positions but couln't control the probability of them being legal
    // unless we fell back to the starting position
    fn random_pos(_rng: &mut impl Rng, _strictness: Strictness, _symmetry: Option<Symmetry>) -> Res<Self> {
        bail!("Not currently implemented for Fairy")
    }

    fn settings(&self) -> &Self::Settings {
        self.rules()
    }

    fn settings_ref(&self) -> Self::SettingsRef {
        self.rules.clone()
    }

    fn variant_for(name: &str, rest: &mut Tokens, proto: Protocol) -> Res<Self> {
        if name.is_empty() {
            bail!("Missing name for fairy variant");
        };
        let mut variant = Self::variant_from_name(name, proto)?;
        let rest_copy = rest.clone();
        let res = variant.get().format_rules.read_rules_part(rest);
        if let Ok(Some(new)) = res {
            variant = new;
        } else if res.is_err() {
            *rest = rest_copy;
        }
        Ok(Self::startpos_for_settings(variant))
    }

    fn list_variants() -> Option<Vec<String>> {
        Some(Self::variants().iter().map(|v| v.name.to_string()).collect_vec())
    }

    fn active_player(&self) -> Color {
        self.0.active
    }

    fn halfmove_ctr_since_start(&self) -> usize {
        self.0.ply_since_start
    }

    fn ply_draw_clock(&self) -> usize {
        self.0.draw_counter
    }

    fn size(&self) -> Size {
        self.rules().size
    }

    fn is_empty(&self, coords: Self::Coordinates) -> bool {
        self.0.is_empty(coords)
    }

    fn is_piece_on(&self, coords: Self::Coordinates, piece: ColPieceTypeOf<Self>) -> bool {
        let idx = self.0.idx(coords);
        if let Some(color) = piece.color() {
            self.col_piece_bb(color, piece.uncolor()).is_bit_set_at(idx)
        } else {
            self.piece_bb(piece.uncolor()).is_bit_set_at(idx)
        }
    }

    fn colored_piece_on(&self, coords: Self::Coordinates) -> Self::Piece {
        self.0.piece_on(coords)
    }

    fn piece_type_on(&self, coords: Self::Coordinates) -> PieceTypeOf<Self> {
        let idx = self.0.idx(coords);
        if let Some((idx, _piece)) = self.0.piece_bitboards.iter().find_position(|p| p.is_bit_set_at(idx)) {
            PieceId::new(idx)
        } else {
            PieceId::empty()
        }
    }

    fn hand(&self, color: Self::Color) -> impl Iterator<Item = (usize, PieceId)> {
        let hand = self.rules().format_rules.hand != FenHandInfo::None;
        self.in_hand[color]
            .iter()
            .enumerate()
            .filter(move |_| hand)
            .map(|(idx, &count)| (count as usize, PieceId::new(idx)))
            .filter(|(count, _)| *count > 0)
    }

    fn default_perft_depth(&self) -> DepthPly {
        DepthPly::new(3)
    }

    fn cannot_call_movegen(&self) -> bool {
        for (cond, _) in &self.rules().game_end_eager {
            if self.precludes_movegen(cond) {
                return true;
            }
        }
        false
    }

    fn pseudolegal_moves(&self) -> MoveList {
        let mut moves = MoveList::new();
        self.gen_pseudolegal_impl(&mut moves);
        if moves.is_empty() && self.no_moves_result().is_none() {
            moves.push(Self::Move::default());
        }
        moves
    }

    fn tactical_pseudolegal(&self) -> MoveList {
        let mut moves = MoveList::new();
        self.gen_pseudolegal_impl(&mut moves);
        MoveListTrait::<Board>::filter_moves(&mut moves, |m: &mut Move| m.is_tactical(self));
        moves
    }

    fn num_pseudolegal_moves(&self) -> usize {
        let mut ctr = 0;
        self.gen_pseudolegal(|_| ctr += 1);
        if ctr == 0 && self.no_moves_result().is_none() {
            ctr += 1;
        }
        ctr
    }

    fn gen_pseudolegal(&self, mut callback: impl FnMut(Move)) {
        let mut moves = MoveList::new();
        self.gen_pseudolegal_impl(&mut moves);
        for m in moves {
            callback(m);
        }
    }

    // Implemented by simply filtering all pseudolegal moves
    fn gen_tactical_pseudolegal(&self, mut callback: impl FnMut(Move)) {
        let mut moves = MoveList::new();
        self.gen_pseudolegal_impl(&mut moves);
        for m in moves {
            if m.is_tactical(self) {
                callback(m);
            }
        }
    }

    fn random_legal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
        self.legal_moves_slow().choose(rng).copied()
    }

    fn random_pseudolegal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
        self.pseudolegal_moves().choose(rng).copied()
    }

    fn make_move(self, mov: Self::Move) -> Option<Self> {
        self.make_move_impl(mov)
    }

    fn make_nullmove(mut self) -> Option<Self> {
        self.0.last_move = Move::default();
        self.end_move(Move::default())
    }

    fn is_move_pseudolegal(&self, mov: Self::Move) -> bool {
        let moves = self.pseudolegal_moves();
        moves.contains(&mov)
    }

    // TODO: Maybe we can use the fact that some games have an easy way to check for pseudolegal moves
    fn is_pseudolegal_move_legal(&self, mov: Self::Move) -> bool {
        self.clone().make_move(mov).is_some()
    }

    fn player_result_no_movegen<H: BoardHistory>(&self, history: &H) -> Option<PlayerResult> {
        for (cond, outcome) in &self.rules().game_end_eager {
            if cond.satisfied(self, history) {
                return Some(outcome.to_res(self, history));
            }
        }
        None
    }
    /// When loading a position where the side to move has won and there is no legal previous move for the other player,
    /// like a position where the current player has the king in the center in king of the hill,
    /// [`Self::player_result_slow`] can return a win for an incorrect player, but this can never happen in a real game.
    fn player_result_slow<H: BoardHistory>(&self, history: &H) -> Option<PlayerResult> {
        if let Some(res) = self.player_result_no_movegen(history) {
            return Some(res);
        }
        if self.legal_moves_slow().is_empty() {
            return self.no_moves_result();
        }
        None
    }

    fn no_moves_result(&self) -> Option<PlayerResult> {
        for (cond, outcome) in &self.rules().game_end_no_moves {
            if cond.satisfied(self) {
                // TODO: Pass the actual history? Currently not needed, but this not working is surprising
                return Some(outcome.to_res(self, &NoHistory::default()));
            }
        }
        None
    }

    fn can_reasonably_win(&self, _player: Color) -> bool {
        true
    }

    fn hash_pos(&self) -> PosHash {
        self.hash
    }

    fn read_fen_and_advance_input_for(input: &mut Tokens, strictness: Strictness, mut rules: RulesRef) -> Res<Self> {
        let variant_name = input.peek().copied().unwrap_or_default();
        // Different rules can have the same name, for example shogi in ugi and usi mode
        if variant_name == rules.get().name {
            _ = input.next();
        } else {
            // TODO: support custom matching, e.g. <n>check for any reasonable `n`
            let variants = Self::variants();
            if let Some(v) = variants.iter().find(|v| v.name.eq_ignore_ascii_case(variant_name)) {
                _ = input.next();
                rules = (v.val)();
            }
        }
        let input_copy = input.clone();
        match rules.get().format_rules.read_rules_part(input) {
            Ok(Some(fen_rules)) => rules = fen_rules,
            Ok(None) => {}
            Err(_) => {
                *input = input_copy;
            }
        }
        let board = Board::empty_for_settings(rules.clone());
        let input_copy = input.clone();
        match board.read_fen_without_rules(input, strictness) {
            Ok(res) => Ok(res),
            Err(err) => {
                *input = input_copy;
                // We support loading both cutechess FENs and SFENs for shogi (and variants) without explicitly setting
                // the protocol through a UGI option. Output will be based on input format, so if we receive an SFEN we
                // will output FENs and moves in SFEN and USI format.
                if let Some(rules) = rules.get().fallback.clone() {
                    let rules = RulesRef::from_arc(rules);
                    if let Ok(res) = Self::empty_for_settings(rules).read_fen_without_rules(input, strictness) {
                        return Ok(res);
                    }
                }
                Err(err)
            }
        }
    }

    fn axes_format(&self) -> AxesFormat {
        self.rules().format_rules.axes_format
    }

    fn as_diagram(&self, typ: CharType, flip: bool, mark_active: bool) -> String {
        board_to_string(self, GenericPiece::to_char, typ, flip, mark_active)
    }

    fn display_pretty(&self, formatter: &mut dyn BoardFormatter<Self>) -> String {
        display_board_pretty(self, formatter)
    }

    fn pretty_formatter(
        &self,
        piece: Option<CharType>,
        last_move: Option<Self::Move>,
        opts: OutputOpts,
    ) -> Box<dyn BoardFormatter<Self>> {
        Box::new(DefaultBoardFormatter::new(self.clone(), piece, last_move, opts))
    }

    fn background_color(&self, square: Square) -> SquareColor {
        // TODO: Maybe have a member in settings for turning that on
        square.square_color()
    }

    fn as_alternative_fen(&self) -> Option<String> {
        let alternative = self.settings().fallback.clone()?;
        let pos = UnverifiedBoard { rules: RulesRef::from_arc(alternative), ..self.0 };
        Some(pos.verify(Relaxed).ok()?.to_string())
    }

    fn bitboard_from_name(&self) -> BBSelect<Self> {
        let mut res = default_bitboards_from_name(self);
        res.push(GenericSelect::full(
            "royals",
            None,
            "Bitboard of royal pieces, such as kings in chess",
            self.royal_bb().raw(),
        ));
        res.push(GenericSelect::full(
            "neutral",
            None,
            "Bitboard of squares that aren't empty and don't have a piece belonging to either player",
            self.neutral_bb,
        ));
        if self.rules().has_ep {
            res.push(GenericSelect::full(
                "e_p_square",
                None,
                "Bitboard of squares where an ep capture is possible",
                self.ep.map(|sq| self.single_piece(sq)).unwrap_or(self.zero_bitboard()).raw(),
            ));
        }
        if self.rules().has_castling {
            res.push(GenericSelect::full(
                "castling_dest_squares",
                None,
                "Bitboard of squares where the king ends up after castling",
                self.castling_info.king_dest_squares(self.size()).raw(),
            ));
        }
        res
    }

    fn valid_squares_bb(&self) -> Self::RawBitboard {
        self.mask_bb
    }

    fn attacks_of(&self, sq: Square) -> ExtendedRawBitboard {
        self.attack_of_impl(sq)
    }
}

impl BitboardBoard for Board {
    type Bitboard = Bitboard;

    fn piece_bb(&self, piece: PieceTypeOf<Self>) -> Bitboard {
        self.0.piece_bb(piece)
    }

    fn player_bb(&self, color: Color) -> Bitboard {
        self.0.player_bb(color)
    }

    fn neutral_bb(&self) -> Self::Bitboard {
        self.0.neutral_bb()
    }

    fn mask_bb(&self) -> Self::Bitboard {
        self.0.mask_bb()
    }
}

type NameToVariant = SimpleSelect<Box<dyn Fn() -> RulesRef>>;

impl Deref for Board {
    type Target = UnverifiedBoard;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Board {
    fn variants_for(protocol: Protocol) -> EntityList<NameToVariant> {
        vec![
            SimpleSelect { name: "chess", val: Box::new(|| RulesRef::new(RulesBuilder::chess().build())) },
            SimpleSelect {
                name: "cylinder_chess",
                val: Box::new(|| RulesRef::new(RulesBuilder::cylinder_chess().build())),
            },
            SimpleSelect { name: "atomic", val: Box::new(|| RulesRef::new(RulesBuilder::atomic().build())) },
            SimpleSelect {
                name: "kingofthehill",
                val: Box::new(|| RulesRef::new(RulesBuilder::king_of_the_hill().build())),
            },
            SimpleSelect { name: "horde", val: Box::new(|| RulesRef::new(RulesBuilder::horde().build())) },
            SimpleSelect {
                name: "racingkings",
                val: Box::new(|| RulesRef::new(RulesBuilder::racing_kings(Size::chess()).build())),
            },
            SimpleSelect { name: "crazyhouse", val: Box::new(|| RulesRef::new(RulesBuilder::crazyhouse().build())) },
            SimpleSelect { name: "3check", val: Box::new(|| RulesRef::new(RulesBuilder::n_check(3).build())) },
            SimpleSelect { name: "5check", val: Box::new(|| RulesRef::new(RulesBuilder::n_check(5).build())) },
            SimpleSelect { name: "antichess", val: Box::new(|| RulesRef::new(RulesBuilder::antichess().build())) },
            SimpleSelect { name: "shatranj", val: Box::new(|| RulesRef::new(RulesBuilder::shatranj().build())) },
            SimpleSelect { name: "makruk", val: Box::new(|| RulesRef::new(RulesBuilder::makruk().build())) },
            SimpleSelect {
                name: "shogi",
                val: Box::new(move || RulesRef::new(RulesBuilder::shogi(protocol == Protocol::USI).build())),
            },
            SimpleSelect { name: "ataxx", val: Box::new(|| RulesRef::new(RulesBuilder::ataxx().build())) },
            SimpleSelect {
                name: "droptaxx",
                val: Box::new(|| RulesRef::new(RulesBuilder::droptaxx(Size::ataxx()).build())),
            },
            SimpleSelect { name: "tictactoe", val: Box::new(|| RulesRef::new(RulesBuilder::tictactoe().build())) },
            SimpleSelect {
                name: "mnk",
                val: Box::new(|| RulesRef::new(RulesBuilder::mnk(GridSize::connect4(), 4).build())),
            },
            SimpleSelect {
                name: "cfour",
                val: Box::new(|| RulesRef::new(RulesBuilder::cfour(GridSize::connect4(), 4).build())),
            },
        ]
    }

    fn variants() -> EntityList<NameToVariant> {
        Self::variants_for(Protocol::Interactive)
    }

    fn variant_from_name(name: &str, proto: Protocol) -> Res<RulesRef> {
        Ok((select_name_static(name, Self::variants_for(proto).iter(), "variant", "fairy", NoDescription)?.val)())
    }

    /// Returns the startpos for the given variant
    pub fn variant_simple(name: &str) -> Res<Self> {
        Self::variant_for(name, &mut tokens(""), Protocol::Interactive)
    }

    pub fn from_fen_for(variant: &str, fen: &str, strictness: Strictness) -> Res<Self> {
        let variant = Self::variant_from_name(variant, Protocol::Interactive)?;
        Self::from_fen_for_rules(fen, strictness, variant)
    }

    pub fn from_fen_for_rules(fen: &str, strictness: Strictness, rules: RulesRef) -> Res<Self> {
        let mut words = tokens(fen);
        let res = Self::read_fen_and_advance_input_for(&mut words, strictness, rules)
            .map_err(|err| anyhow!("Failed to parse FEN '{}': {err}", fen.bold()))?;
        if let Some(next) = words.next() {
            return Err(anyhow!(
                "Input `{0}' contained additional characters after FEN, starting with '{1}'",
                fen.bold(),
                next.red()
            ));
        }
        Ok(res)
    }

    pub fn fen_no_rules(&self) -> String {
        NoRulesFenFormatter(self).to_string()
    }

    fn precludes_movegen(&self, cond: &GameEndEager) -> bool {
        match cond {
            GameEndEager::NoPiece(_, _)
            | GameEndEager::InRowAtLeast(_, _)
            | GameEndEager::PieceIn(_, _, _)
            | GameEndEager::AdditionalCounter
            | GameEndEager::InCheck(_)
            | GameEndEager::CanAchieve(_) => cond.satisfied(self, &NoHistory::default()),
            // These conditions are ignored in perft
            GameEndEager::DrawCounter(_) | GameEndEager::Repetition(_) | GameEndEager::InsufficientMaterial(_, _) => {
                false
            }

            GameEndEager::And(both) => self.precludes_movegen(&both[0]) && self.precludes_movegen(&both[1]),
            GameEndEager::Not(c) => match Box::deref(c) {
                GameEndEager::DrawCounter(_)
                | GameEndEager::Repetition(_)
                | GameEndEager::InsufficientMaterial(_, _) => false,
                c => !self.precludes_movegen(c),
            },
        }
    }
}

pub struct NoRulesFenFormatter<'a>(&'a Board);

impl Display for NoRulesFenFormatter<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let pos = self.0;
        position_fen_part(f, pos)?;
        match pos.rules().format_rules.hand {
            FenHandInfo::None => {
                write!(f, " {}", pos.active_player().to_char(pos.settings()))?;
            }
            FenHandInfo::InBrackets => {
                write!(f, "[")?;
                pos.write_fen_hand_part(f)?;
                write!(f, "]")?;
                write!(f, " {}", pos.active_player().to_char(pos.settings()))?;
            }
            FenHandInfo::SeparateToken => {
                write!(f, " {} ", pos.active_player().to_char(pos.settings()))?;
                pos.write_fen_hand_part(f)?;
            }
        }
        if pos.rules().has_castling {
            pos.0.castling_info.write_fen_part(f)?;
        }
        if pos.rules().has_ep {
            if let Some(sq) = pos.0.ep {
                write!(f, " {sq}")?;
            } else {
                write!(f, " -")?;
            }
        }
        let fmt_ctr = |f: &mut Formatter, ctr, threshold| {
            let ctr = if threshold == 0 {
                if ctr == 0 {
                    return write!(f, "-");
                }
                ctr
            } else {
                threshold - ctr
            };
            write!(f, "{ctr}")
        };
        if pos.rules().num_additional_ctrs() > 0 {
            write!(f, " ")?;
            if let Some(threshold) = pos.rules().ctr_threshold[0] {
                fmt_ctr(f, pos.additional_ctrs[0], threshold)?;
            }
            if pos.rules().num_additional_ctrs() > 1 {
                write!(f, "+")?;
            }
            if let Some(threshold) = pos.rules().ctr_threshold[1] {
                fmt_ctr(f, pos.additional_ctrs[1], threshold)?;
            }
        }
        if pos.rules().format_rules.has_ply_clock {
            write!(f, " {}", pos.ply_draw_clock())?;
        }
        let ctr = match pos.rules().format_rules.move_num_fmt {
            MoveNumFmt::Fullmove => pos.fullmove_ctr_1_based(),
            MoveNumFmt::Halfmove => pos.halfmove_ctr_since_start() + 1,
        };
        write!(f, " {ctr}")
    }
}

impl UnverifiedBoard {
    fn read_fen_without_rules(mut self, input: &mut Tokens, strictness: Strictness) -> Res<Board> {
        read_common_fen_part::<Board>(input, &mut self)?;
        if self.rules().format_rules.hand == FenHandInfo::SeparateToken {
            let Some(hand) = input.next() else {
                bail!("Missing hand part");
            };
            self.read_fen_hand_part(hand)?;
        }
        // does nothing if the rules don't require these parts
        self.read_castling_and_ep_fen_parts(input, strictness)?;
        let trailing_counters = self.rules().num_additional_ctrs() > 0 && self.read_ctrs(input, true).is_err();
        if self.rules().format_rules.has_ply_clock {
            read_halfmove_clock::<Board>(input, &mut self)?;
        }
        match self.rules().format_rules.move_num_fmt {
            MoveNumFmt::Fullmove => read_single_move_number::<Board>(input, &mut self, strictness, None)?,
            MoveNumFmt::Halfmove => read_move_number_in_ply::<Board>(input, &mut self, strictness)?,
        }
        if trailing_counters {
            self.read_ctrs(input, false)?;
        }
        self.verify_with_level(CheckFen, strictness)
    }

    fn square_formatter(&self, sq: Square) -> SquareFormatter {
        SquareFormatter { sq, axes_format: self.rules().format_rules.axes_format, size: self.size() }
    }
}

struct SquareFormatter {
    sq: Square,
    axes_format: AxesFormat,
    size: Size,
}

impl Display for SquareFormatter {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let file = self.axes_format.ith_x_axis_entry(self.sq.file(), self.size.width().get(), None, false);
        let rank = self.axes_format.ith_y_axis_entry(self.sq.rank(), self.size.height().get(), None, false);
        write!(f, "{file}{rank}")
    }
}
