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
mod attacks;
mod moves;
mod perft_tests;
mod pieces;
mod rules;

use crate::games::chess::pieces::NUM_COLORS;
use crate::games::fairy::moves::FairyMove;
use crate::games::fairy::pieces::{ColoredPieceId, PieceId};
use crate::games::fairy::rules::{Draw, GameLoss, Rules, RulesRef};
use crate::games::{
    n_fold_repetition, AbstractPieceType, BoardHistory, CharType, Color, ColoredPiece,
    ColoredPieceType, Coordinates, DimT, GenericPiece, NoHistory, PosHash, Size,
};
use crate::general::bitboards::{
    Bitboard, DynamicallySizedBitboard, ExtendedRawBitboard, RawBitboard,
};
use crate::general::board::SelfChecks::CheckFen;
use crate::general::board::Strictness::Strict;
use crate::general::board::{
    position_fen_part, read_common_fen_part, read_single_move_number, read_two_move_numbers,
    BitboardBoard, Board, BoardHelpers, BoardSize, ColPieceType, NameToPos, PieceTypeOf,
    SelfChecks, Strictness, UnverifiedBoard,
};
use crate::general::common::Description::NoDescription;
use crate::general::common::{
    select_name_static, tokens, EntityList, GenericSelect, Res, StaticallyNamedEntity, Tokens,
};
use crate::general::move_list::{EagerNonAllocMoveList, MoveList};
use crate::general::moves::Move;
use crate::general::squares::{GridCoordinates, GridSize, RectangularCoordinates, SquareColor};
use crate::output::text_output::{
    board_to_string, display_board_pretty, BoardFormatter, DefaultBoardFormatter,
};
use crate::output::OutputOpts;
use crate::search::Depth;
use crate::PlayerResult;
use anyhow::bail;
use arbitrary::Arbitrary;
use itertools::Itertools;
use rand::prelude::IndexedRandom;
use rand::Rng;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::{Deref, Not};
use std::sync::Arc;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, FromRepr};

// Using a 64 bit bitboard makes chess perft twice as fast, but obviously doesn't work for larger boards
type RawFairyBitboard = ExtendedRawBitboard;
type FairyBitboard = DynamicallySizedBitboard<RawFairyBitboard, FairySquare>;

/// There can never be more than 32 piece types in a given game
/// (For chess, the number would be 6; for ataxx, 1).
/// Note that some effects can also be represented by one of these bitboards.
const MAX_NUM_PIECE_TYPES: usize = 16;

pub type FairySquare = GridCoordinates;
pub type FairySize = GridSize;

/// Maximum number of pseudolegal moves in a position
const MAX_MOVES: usize = 1024;

type FairyMoveList = EagerNonAllocMoveList<FairyBoard, MAX_MOVES>;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub struct FairyColor(bool);

impl FairyColor {
    pub fn idx(&self) -> usize {
        self.0 as usize
    }
    pub fn from_idx(idx: usize) -> Self {
        Self(idx != 0)
    }
}

impl Color for FairyColor {
    type Board = FairyBoard;

    fn second() -> Self {
        Self(true)
    }

    fn to_char(self, settings: &RulesRef) -> char {
        settings.0.colors[self.idx()].ascii_char
    }

    fn name(self, settings: &<Self::Board as Board>::Settings) -> impl Display {
        settings.0.colors[self.idx()].name.clone()
    }
}

impl Not for FairyColor {
    type Output = Self;
    fn not(self) -> Self {
        self.other()
    }
}

#[derive(Debug, Eq, PartialEq, Arbitrary)]
#[must_use]
struct ColorInfo {
    ascii_char: char,
    name: String,
}

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Hash, EnumIter, derive_more::Display, FromRepr, Arbitrary,
)]
#[must_use]
pub enum Side {
    Kingside,
    Queenside,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
struct CastlingMoveInfo {
    rook_file: DimT,
    king_dest_file: DimT,
    rook_dest_file: DimT,
    fen_char: u8,
}

impl ColoredFairyCastleInfo {
    fn king_dest_sq(self, side: Side) -> Option<FairySquare> {
        self.sides[side as usize]
            .map(|info| FairySquare::from_rank_file(self.rank, info.king_dest_file))
    }
    fn rook_dest_sq(self, side: Side) -> Option<FairySquare> {
        self.sides[side as usize]
            .map(|info| FairySquare::from_rank_file(self.rank, info.rook_dest_file))
    }
    fn rook_sq(self, side: Side) -> Option<FairySquare> {
        self.sides[side as usize].map(|info| FairySquare::from_rank_file(self.rank, info.rook_file))
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
            players: [ColoredFairyCastleInfo {
                sides: [None, None],
                rank: 0,
            }; 2],
        }
    }
}

impl FairyCastleInfo {
    fn new(size: FairySize) -> Self {
        let mut res = Self::default();
        res.players[1].rank = size.height.0 - 1;
        res
    }
    fn player(&self, color: FairyColor) -> &ColoredFairyCastleInfo {
        &self.players[color.idx()]
    }
    fn info(&self, color: FairyColor, side: Side) -> Option<CastlingMoveInfo> {
        self.player(color).sides[side as usize]
    }
    pub fn can_castle(&self, color: FairyColor, side: Side) -> bool {
        self.info(color, side).is_some()
    }
    pub fn unset(&mut self, color: FairyColor, side: Side) {
        self.players[color.idx()].sides[side as usize] = None;
    }
    pub fn write_fen_part(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut can_castle = false;
        for color in FairyColor::iter() {
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
        write!(f, " ")
    }
}

/// A FairyBoard is a rectangular board for a chess-like variant.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub struct UnverifiedFairyBoard {
    // unfortunately, ArrayVec isn't `Copy`
    piece_bitboards: [RawFairyBitboard; MAX_NUM_PIECE_TYPES],
    color_bitboards: [RawFairyBitboard; NUM_COLORS],
    // bb of all valid squares
    mask_bb: RawFairyBitboard,
    // for each piece type, how many the player has available to drop
    in_hand: [u8; MAX_NUM_PIECE_TYPES],
    ply_since_start: usize,
    // like the 50mr counter in chess TODO: Maybe make it count down?
    num_piece_bitboards: usize,
    draw_counter: usize,
    active: FairyColor,
    castling_info: FairyCastleInfo,
    size: GridSize,
    ep: Option<FairySquare>,
    last_move: FairyMove,
    rules: RulesRef,
}

impl Default for UnverifiedFairyBoard {
    fn default() -> Self {
        let rules = RulesRef::default();
        rules.empty_pos()
    }
}

impl UnverifiedFairyBoard {
    fn occupied_bb(&self) -> FairyBitboard {
        FairyBitboard::new(
            self.color_bitboards[0] | self.color_bitboards[1],
            self.size(),
        )
    }

    // TODO: Rename to `rules()` and remove free `rules()`, also don't clone the arc each time
    fn get_rules(&self) -> Arc<Rules> {
        self.rules.0.clone()
    }
}

impl From<FairyBoard> for UnverifiedFairyBoard {
    fn from(value: FairyBoard) -> Self {
        value.0
    }
}

impl UnverifiedBoard<FairyBoard> for UnverifiedFairyBoard {
    fn verify_with_level(self, level: SelfChecks, strictness: Strictness) -> Res<FairyBoard> {
        let size = self.size();
        let rules = self.get_rules();
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
        let mut pieces = RawFairyBitboard::default();
        for (id, _piece) in rules.pieces.iter().enumerate() {
            let bb = self.piece_bitboards[id];
            if (bb & pieces).has_set_bit() {
                bail!("Two pieces on the same square")
            }
            pieces |= bb;
        }
        if strictness == Strict {
            let draw = rules
                .draw
                .iter()
                .find_map(|d| {
                    if let Draw::Counter(val) = d {
                        Some(*val)
                    } else {
                        None
                    }
                })
                .unwrap_or_default();
            if self.draw_counter > draw {
                bail!(
                    "Progress counter too large: {0} is larger than {draw}",
                    self.draw_counter
                );
            }
        }
        if self.ply_since_start >= usize::MAX / 2 {
            bail!("Ridiculously large ply counter ({})", self.ply_since_start)
        }

        for color in FairyColor::iter() {
            for side in Side::iter() {
                if !self.castling_info.can_castle(color, side) {
                    continue;
                }
                if let Some(rook_sq) = self.castling_info.player(color).rook_sq(side) {
                    if self.is_empty(rook_sq) {
                        bail!(
                            "Color {0} can castle {side}, but there is no piece to castle with{1}",
                            self.get_rules().colors[color.idx()].name,
                            if level == CheckFen {
                                " (invalid castling flag in FEN?)"
                            } else {
                                ""
                            }
                        );
                    }
                }
            }
        }
        if self.ep.is_some() && !rules.has_ep {
            bail!("The ep square is set even though the rules don't mention en passant")
        }

        let res = FairyBoard(self);
        if res.is_player_in_check(res.inactive_player()) {
            bail!(
                "Player {} is in check, but it's not their turn to move",
                res.get_rules().colors[res.inactive_player().idx()].name
            );
        }

        Ok(res)
    }

    fn settings(&self) -> RulesRef {
        RulesRef(self.get_rules())
    }

    fn size(&self) -> BoardSize<FairyBoard> {
        self.size
    }

    fn place_piece(&mut self, coords: FairySquare, piece: ColPieceType<FairyBoard>) {
        let bb = self.single_piece(coords).raw();
        self.piece_bitboards[piece.to_uncolored_idx()] |= bb;
        if let Some(color) = piece.color() {
            self.color_bitboards[color.idx()] |= bb;
        }
    }

    fn remove_piece(&mut self, coords: FairySquare) {
        let idx = self.idx(coords);
        let bb = self.single_piece(coords).raw();
        if let Some(col_bb) = self
            .color_bitboards
            .iter_mut()
            .find(|bb| bb.is_bit_set_at(idx))
        {
            *col_bb ^= bb;
        }
        if let Some(piece_bb) = self
            .piece_bitboards
            .iter_mut()
            .find(|bb| bb.is_bit_set_at(idx))
        {
            *piece_bb ^= bb;
        }
    }

    fn piece_on(&self, coords: FairySquare) -> <FairyBoard as Board>::Piece {
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
            .map(|(idx, _bb)| FairyColor::from_idx(idx));

        GenericPiece::new(ColoredPieceId::create(piece, color), coords)
    }

    fn is_empty(&self, coords: FairySquare) -> bool {
        !self.occupied_bb().is_bit_set_at(self.idx(coords))
    }

    fn active_player(&self) -> FairyColor {
        self.active
    }

    fn set_active_player(&mut self, player: FairyColor) {
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
}

impl UnverifiedFairyBoard {
    fn idx(&self, square: FairySquare) -> usize {
        self.size().internal_key(square)
    }
    fn single_piece(&self, square: FairySquare) -> FairyBitboard {
        FairyBitboard::new(
            RawFairyBitboard::single_piece_at(self.idx(square)),
            self.size(),
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub struct FairyBoard(UnverifiedFairyBoard);

impl Default for FairyBoard {
    fn default() -> Self {
        Self::startpos()
    }
}

impl Display for FairyBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.get_rules().rules_fen_part(f)?;
        write!(f, "{}", NoRulesFenFormatter(self))
    }
}

impl StaticallyNamedEntity for FairyBoard {
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

type FairyPiece = GenericPiece<FairyBoard, ColoredPieceId>;

impl Board for FairyBoard {
    type EmptyRes = Self::Unverified;
    type Settings = RulesRef;
    type Coordinates = FairySquare;
    type Color = FairyColor;
    type Piece = FairyPiece;
    type Move = FairyMove;
    type MoveList = FairyMoveList;
    type Unverified = UnverifiedFairyBoard;

    fn empty_for_settings(settings: Self::Settings) -> Self::Unverified {
        settings.empty_pos()
    }

    fn startpos_for_settings(settings: Self::Settings) -> Self {
        Self::from_fen(&settings.0.startpos_fen, Strict).unwrap()
    }

    fn name_to_pos_map() -> EntityList<NameToPos<Self>> {
        // TODO: add more named positions
        vec![
            GenericSelect {
                name: "kiwipete",
                val: || {
                    Self::from_fen(
                    "chess r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                    Strict,
                )
                .unwrap()
                },
            },
            GenericSelect {
                name: "large_mnk",
                val: || {
                    Self::from_fen("mnk 11 11 4 11/11/11/11/11/11/11/11/11/11/11 x 1", Strict)
                        .unwrap()
                },
            },
        ]
    }

    fn bench_positions() -> Vec<Self> {
        // TODO: More positions covering a wide variety of rules
        vec![Self::startpos()]
    }

    fn settings(&self) -> Self::Settings {
        self.rules.clone()
    }

    fn variant(first: &str, rest: &mut Tokens) -> Res<FairyBoard> {
        if first.is_empty() {
            bail!("Missing name for fairy variant");
        };
        let mut variant = (select_name_static(
            first,
            Self::variants().iter(),
            "variant",
            "fairy",
            NoDescription,
        )?
        .val)();
        let rest_copy = rest.clone();
        let res = variant.0.read_rules_fen_part(rest);
        if let Ok(Some(new)) = res {
            variant = new;
        } else if let Err(_) = res {
            *rest = rest_copy;
        }
        Ok(Self::startpos_for_settings(variant))
    }

    fn list_variants() -> Option<Vec<String>> {
        Some(
            Self::variants()
                .iter()
                .map(|v| v.name.to_string())
                .collect_vec(),
        )
    }

    fn active_player(&self) -> FairyColor {
        self.0.active
    }

    fn halfmove_ctr_since_start(&self) -> usize {
        self.0.ply_since_start
    }

    fn halfmove_repetition_clock(&self) -> usize {
        self.0.draw_counter
    }

    fn size(&self) -> <Self::Coordinates as Coordinates>::Size {
        self.0.size()
    }

    fn is_empty(&self, coords: Self::Coordinates) -> bool {
        self.0.is_empty(coords)
    }

    fn is_piece_on(&self, coords: Self::Coordinates, piece: ColPieceType<Self>) -> bool {
        let idx = self.0.idx(coords);
        if let Some(color) = piece.color() {
            self.colored_piece_bb(color, piece.uncolor())
                .is_bit_set_at(idx)
        } else {
            self.piece_bb(piece.uncolor()).is_bit_set_at(idx)
        }
    }

    fn colored_piece_on(&self, coords: Self::Coordinates) -> Self::Piece {
        self.0.piece_on(coords)
    }

    fn piece_type_on(&self, coords: Self::Coordinates) -> PieceTypeOf<Self> {
        let idx = self.0.idx(coords);
        if let Some((idx, _piece)) = self
            .0
            .piece_bitboards
            .iter()
            .find_position(|p| p.is_bit_set_at(idx))
        {
            PieceId::new(idx)
        } else {
            PieceId::empty()
        }
    }

    fn default_perft_depth(&self) -> Depth {
        Depth::new(3)
    }

    fn gen_pseudolegal<T: MoveList<Self>>(&self, moves: &mut T) {
        self.gen_pseudolegal_impl(moves);
    }

    fn gen_tactical_pseudolegal<T: MoveList<Self>>(&self, _moves: &mut T) {
        // do nothing for now
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
        self.0.last_move = FairyMove::default();
        self.end_move()
    }

    fn is_move_pseudolegal(&self, mov: Self::Move) -> bool {
        self.pseudolegal_moves().contains(&mov)
    }

    fn is_pseudolegal_move_legal(&self, mov: Self::Move) -> bool {
        self.clone().make_move(mov).is_some()
    }

    fn player_result_no_movegen<H: BoardHistory<Self>>(&self, history: &H) -> Option<PlayerResult> {
        let us = self.active_player();
        for condition in &self.get_rules().game_loss {
            let loss = match condition {
                GameLoss::NoRoyals => self.royal_bb_for(us).is_zero(),
                GameLoss::NoPieces => self.active_player_bb().is_zero(),
                GameLoss::Checkmate => false,
                GameLoss::NoNonRoyals => (self.active_player_bb() & !self.royal_bb()).is_zero(),
                GameLoss::NoNonRoyalsExceptRecapture => {
                    let has_nonroyals = (self.active_player_bb() & !self.royal_bb()).has_set_bit();
                    if has_nonroyals {
                        false
                    } else {
                        let their_nonroyals = self.inactive_player_bb() & !self.royal_bb();
                        if their_nonroyals.num_ones() > 1 {
                            true
                        } else {
                            let capturable = their_nonroyals & !self.capturing_attack_bb_of(us);
                            return if capturable.has_set_bit() {
                                Some(PlayerResult::Lose)
                            } else {
                                Some(PlayerResult::Draw)
                            };
                        }
                    }
                }
                GameLoss::NoMoves => false, // will be dealt with in `no_moves_result()`
                &GameLoss::InRowAtLeast(k) => {
                    if self.0.last_move.is_null() {
                        false
                    } else {
                        let sq = self.0.last_move.dest_square_in(self);
                        self.k_in_row_at(k, sq, !us)
                    }
                }
            };
            if loss {
                return Some(PlayerResult::Lose);
            }
        }
        for rule in &self.get_rules().draw {
            if match rule {
                &Draw::Counter(max) => self.0.draw_counter >= max,
                &Draw::Repetition(max) => n_fold_repetition(max, history, self, usize::MAX),
                _ => false,
            } {
                return Some(PlayerResult::Draw);
            }
        }
        None
    }

    fn player_result_slow<H: BoardHistory<Self>>(&self, history: &H) -> Option<PlayerResult> {
        if let Some(res) = self.player_result_no_movegen(history) {
            return Some(res);
        }
        if self.legal_moves_slow().is_empty() {
            return Some(self.no_moves_result());
        }
        None
    }

    fn no_moves_result(&self) -> PlayerResult {
        if self
            .get_rules()
            .game_loss
            .iter()
            .any(|rule| [GameLoss::Checkmate, GameLoss::NoMoves].contains(rule))
        {
            return PlayerResult::Lose;
        }
        if self.get_rules().draw.contains(&Draw::NoMoves) {
            return PlayerResult::Draw;
        }
        unreachable!("The game rules must specify what happens when there are no legal moves")
    }

    fn is_game_lost_slow(&self) -> bool {
        let us = self.active_player();
        for rule in &self.get_rules().game_loss {
            let res = match rule {
                GameLoss::Checkmate => self.is_in_check() && self.legal_moves_slow().is_empty(),
                GameLoss::NoRoyals => self.royal_bb_for(us).is_zero(),
                GameLoss::NoPieces => self.player_bb(us).is_zero(),
                GameLoss::NoMoves => self.legal_moves_slow().is_empty(), // TODO: Special function?
                GameLoss::NoNonRoyals
                | GameLoss::NoNonRoyalsExceptRecapture
                | GameLoss::InRowAtLeast(_) => {
                    self.player_result_no_movegen(&NoHistory::default()) == Some(PlayerResult::Lose)
                }
            };
            if res {
                return true;
            }
        }
        false
    }

    fn can_reasonably_win(&self, _player: FairyColor) -> bool {
        true
    }

    fn hash_pos(&self) -> PosHash {
        let mut hasher = DefaultHasher::default();
        self.0.hash(&mut hasher);
        PosHash(hasher.finish())
    }

    // Eventually, FEN parsing should work like this: If the first token of the FEN is a recognized game name, like `chess`,
    // that sets the rules() and parses the FEN according to those rules. Otherwise, the rules are inferred from the FEN.
    fn read_fen_and_advance_input(input: &mut Tokens, strictness: Strictness) -> Res<Self> {
        let variants = Self::variants();
        let mut board;
        if let Some(v) = variants.iter().find(|v| {
            v.name
                .eq_ignore_ascii_case(input.peek().copied().unwrap_or_default())
        }) {
            let rules = (v.val)();
            board = Self::empty_for_settings(rules);
            input.next();
        } else {
            // TODO: This always constructs a chess board, which means you can't leave out the variant name in the fen.
            board = Self::empty();
        };
        if let Some(rules) = board.rules.0.read_rules_fen_part(input)? {
            board = Self::empty_for_settings(rules);
        }
        board = read_common_fen_part::<Self>(input, board)?;
        board = board.read_castling_and_ep_fen_parts(input, strictness)?;
        if board.get_rules().has_halfmove_repetition_clock() {
            board = read_two_move_numbers::<Self>(input, board, strictness)?;
        } else {
            board = read_single_move_number::<Self>(input, board, strictness)?;
        }
        board.verify_with_level(CheckFen, strictness)
    }

    fn should_flip_visually() -> bool {
        true
    }

    fn as_diagram(&self, typ: CharType, flip: bool) -> String {
        board_to_string(self, GenericPiece::to_char, typ, flip)
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
        Box::new(DefaultBoardFormatter::new(
            self.clone(),
            piece,
            last_move,
            opts,
        ))
    }

    fn background_color(&self, square: FairySquare) -> SquareColor {
        // TODO: Maybe have a member in settings for turning that on
        square.square_color()
    }
}

impl BitboardBoard for FairyBoard {
    type RawBitboard = RawFairyBitboard;
    type Bitboard = FairyBitboard;

    fn piece_bb(&self, piece: PieceTypeOf<Self>) -> FairyBitboard {
        self.0.piece_bb(piece)
    }

    fn player_bb(&self, color: FairyColor) -> FairyBitboard {
        self.0.player_bb(color)
    }

    fn mask_bb(&self) -> Self::Bitboard {
        self.0.mask_bb()
    }
}

type NameToVariant = GenericSelect<fn() -> RulesRef>;

impl Deref for FairyBoard {
    type Target = UnverifiedFairyBoard;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FairyBoard {
    fn get_rules(&self) -> Arc<Rules> {
        self.0.get_rules()
    }
    fn variants() -> EntityList<NameToVariant> {
        vec![
            GenericSelect {
                name: "chess",
                val: || RulesRef::new(Rules::chess()),
            },
            GenericSelect {
                name: "shatranj",
                val: || RulesRef::new(Rules::shatranj()),
            },
            GenericSelect {
                name: "tictactoe",
                val: || RulesRef::new(Rules::tictactoe()),
            },
            GenericSelect {
                name: "mnk",
                val: || RulesRef::new(Rules::mnk(GridSize::connect4(), 4)),
            },
        ]
    }

    pub fn variant_simple(name: &str) -> Res<Self> {
        Self::variant(name, &mut tokens(""))
    }

    pub fn from_fen_for(variant: &str, fen: &str, strictness: Strictness) -> Res<Self> {
        if fen.starts_with(variant) {
            Self::from_fen(fen, strictness)
        } else {
            Self::from_fen(&(variant.to_string() + " " + &fen), strictness)
        }
    }

    pub fn fen_no_rules(&self) -> String {
        NoRulesFenFormatter(self).to_string()
    }
}

pub struct NoRulesFenFormatter<'a>(&'a FairyBoard);

impl<'a> Display for NoRulesFenFormatter<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let pos = self.0;
        position_fen_part(f, pos)?;
        write!(f, " {} ", pos.active_player().to_char(&pos.settings()))?;
        if pos.get_rules().has_castling {
            pos.0.castling_info.write_fen_part(f)?;
        }
        if pos.get_rules().has_ep {
            if let Some(sq) = pos.0.ep {
                write!(f, "{sq} ")?;
            } else {
                write!(f, "- ")?;
            }
        }
        if pos.get_rules().has_halfmove_repetition_clock() {
            write!(f, "{} ", pos.halfmove_repetition_clock())?;
        }
        write!(f, "{}", pos.fullmove_ctr_1_based())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::chess::Chessboard;
    use crate::games::fairy::attacks::MoveKind;
    use crate::games::fairy::moves::FairyMove;
    use crate::games::mnk::MNKBoard;
    use crate::games::{chess, Height, Width};
    use crate::general::board::Strictness::Strict;
    use crate::general::perft::perft;
    use std::str::FromStr;

    #[test]
    fn simple_chess_startpos_test() {
        let fen = chess::START_FEN;
        let pos = FairyBoard::from_fen(fen, Strict).unwrap();
        let as_fen = pos.as_fen();
        assert_eq!("chess ".to_string() + fen, as_fen);
        let size = pos.size();
        assert_eq!(size, GridSize::new(Height(8), Width(8)));
        assert_eq!(pos.royal_bb().num_ones(), 2);
        assert_eq!(pos.active_player(), FairyColor::first());
        assert_eq!(pos.occupied_bb().num_ones(), 32);
        assert_eq!(pos.empty_bb().num_ones(), 32);
        assert_eq!(pos.player_bb(FairyColor::first()).raw(), 0xffff);
        let capture_bb = pos.capturing_attack_bb_of(FairyColor::first());
        assert_eq!(capture_bb.raw(), 0xff_ff_ff - 0x81);
        assert_eq!(22, capture_bb.num_ones());
        assert_eq!(
            22,
            pos.capturing_attack_bb_of(FairyColor::second()).num_ones()
        );
        assert_eq!(pos.legal_moves_slow().len(), 20);
    }

    #[test]
    fn chess_makemove_test() {
        let chesspos = Chessboard::from_name("kiwipete").unwrap();
        let fen = chesspos.as_fen();
        let pos = FairyBoard::from_fen(&fen, Strict).unwrap();
        assert_eq!(pos.as_fen(), "chess ".to_string() + &fen);
        let moves = pos.legal_moves_slow();
        let chessmoves = chesspos.legal_moves_slow().into_iter().collect_vec();
        let num_castling = moves
            .iter()
            .filter(|m| matches!(m.kind(), MoveKind::Castle(_)))
            .count();
        assert_eq!(num_castling, 2);
        assert_eq!(moves.len(), chessmoves.len());
        for mov in moves {
            let new_pos = pos.clone().make_move(mov).unwrap();
            println!("{new_pos} | {}", mov.compact_formatter(&pos));
            let chess_pos = chessmoves
                .iter()
                .map(|&m| chesspos.make_move(m).unwrap())
                .find(|p| p.as_fen() == new_pos.fen_no_rules())
                .unwrap();
            assert_eq!(
                new_pos,
                FairyBoard::from_fen(&new_pos.as_fen(), Strict).unwrap()
            );
            // for m in new_pos.legal_moves_slow() {
            //     println!("{m} 1");
            // }
            assert_eq!(chess_pos.num_legal_moves(), new_pos.num_legal_moves());
        }
    }

    #[test]
    fn simple_ep_test() {
        let pos = FairyBoard::from_fen(
            "r3k2r/p2pqpb1/bn2pnp1/2pPN3/1pB1P3/2N2Q1p/PPPB1PPP/R3K2R w HAha c6 0 2",
            Strict,
        )
        .unwrap();
        let moves = pos.legal_moves_slow();
        let mov = FairyMove::from_compact_text("d5c6", &pos).unwrap();
        assert!(moves.into_iter().contains(&mov));
        let new_pos = pos.make_move(mov).unwrap();
        assert!(new_pos.0.ep.is_none());
        assert!(new_pos.is_empty(FairySquare::from_str("c5").unwrap()));
        let moves = new_pos.legal_moves_slow();
        let mov = FairyMove::from_compact_text("e7c5", &new_pos).unwrap();
        assert!(moves.contains(&mov));
    }

    #[test]
    fn simple_chess_perft_test() {
        for chess_pos in Chessboard::bench_positions() {
            let fairy_pos = FairyBoard::from_fen(&chess_pos.as_fen(), Strict).unwrap();
            println!("{chess_pos}");
            let max = if cfg!(debug_assertions) { 3 } else { 5 };
            for i in 1..max {
                let depth = Depth::new(i);
                let chess_perft = perft(depth, chess_pos, false);
                let fairy_perft = perft(depth, fairy_pos.clone(), false);
                assert_eq!(chess_perft.depth, fairy_perft.depth);
                assert_eq!(chess_perft.nodes, fairy_perft.nodes);
                assert!(chess_perft.time.as_millis() * 100 + 1000 > fairy_perft.time.as_millis());
            }
        }
    }

    #[test]
    fn simple_chess960_test() {
        let fen = "1rqbkrbn/1ppppp1p/1n6/2N3p1/p7/2P4P/PP1PPPPB/1RQBKR1N w FBfb - 0 10";
        let pos = FairyBoard::from_fen(fen, Strict).unwrap();
        let chess_pos = Chessboard::from_fen(fen, Strict).unwrap();
        assert_eq!(pos.as_fen(), "chess ".to_string() + &fen);
        let moves = pos.legal_moves_slow();
        let mov = FairyMove::from_compact_text("e1f1", &pos).unwrap();
        assert!(moves.contains(&mov));
        assert_eq!(moves.len(), chess_pos.legal_moves_slow().len());
        let fen = "rbbqn1kr/pp2p1pp/6n1/2pp1p2/2P4P/P7/BP1PPPP1/1RBQNNKR b Hha - 1 9";
        let pos = FairyBoard::from_fen(fen, Strict).unwrap();
        let mov = FairyMove::from_compact_text("g8h8", &pos).unwrap();
        let moves = pos.legal_moves_slow();
        assert!(moves.contains(&mov));
    }

    #[test]
    fn simple_shatranj_startpos_test() {
        let pos = FairyBoard::variant_simple("shatranj").unwrap();
        let as_fen = pos.as_fen();
        assert_eq!(as_fen, pos.get_rules().startpos_fen);
        let size = pos.size();
        assert_eq!(size, GridSize::new(Height(8), Width(8)));
        assert_eq!(pos.royal_bb().num_ones(), 2);
        assert_eq!(pos.active_player(), FairyColor::first());
        assert_eq!(pos.occupied_bb().num_ones(), 32);
        assert_eq!(pos.empty_bb().num_ones(), 32);
        assert_eq!(pos.player_bb(FairyColor::first()).raw(), 0xffff);
        let capture_bb = pos.capturing_attack_bb_of(FairyColor::first());
        assert_eq!(capture_bb.raw(), 16760150);
        assert_eq!(18, capture_bb.num_ones());
        assert_eq!(
            18,
            pos.capturing_attack_bb_of(FairyColor::second()).num_ones()
        );
        assert_eq!(pos.legal_moves_slow().len(), 8 + 2 * 2 + 2 * 2);
    }

    #[test]
    fn simple_mnk_test() {
        let pos = FairyBoard::from_fen("tictactoe 3 3 3 3/3/3 x 1", Strict).unwrap();
        assert_eq!(pos.size(), GridSize::tictactoe());
        assert_eq!(
            pos.active_player(),
            FairyColor::from_char('x', &pos.settings()).unwrap()
        );
        assert!(pos.royal_bb().is_zero());
        assert_eq!(pos.empty_bb().num_ones(), 9);
        assert_eq!(pos.num_legal_moves(), 9);
        let mov = FairyMove::from_compact_text("a1", &pos).unwrap();
        let pos = pos.make_move(mov).unwrap();
        assert_eq!(pos.empty_bb().num_ones(), 8);
        assert_eq!(pos.num_legal_moves(), 8);
        assert_eq!(pos.as_fen(), "mnk 3 3 3 3/3/X2 o 1");
        let mov = FairyMove::from_compact_text("c2", &pos).unwrap();
        let pos = pos.make_move(mov).unwrap();
        assert_eq!(pos.num_legal_moves(), 7);
        assert_eq!(pos.as_fen(), "mnk 3 3 3 3/2O/X2 x 2");
    }

    #[test]
    fn simple_mnk_perft_test() {
        for mnk_pos in MNKBoard::bench_positions() {
            let fairy_pos = FairyBoard::from_fen_for("mnk", &mnk_pos.as_fen(), Strict).unwrap();
            println!("{mnk_pos}");
            let max = if cfg!(debug_assertions) { 4 } else { 6 };
            for i in 1..max {
                let depth = Depth::new(i);
                let chess_perft = perft(depth, mnk_pos, false);
                let fairy_perft = perft(depth, fairy_pos.clone(), false);
                assert_eq!(chess_perft.depth, fairy_perft.depth);
                assert_eq!(chess_perft.nodes, fairy_perft.nodes);
                let chess_time = chess_perft.time.as_millis();
                let fairy_time = fairy_perft.time.as_millis();
                assert!(
                    chess_time * 100 + 1000 > fairy_time,
                    "{chess_time} {fairy_time} {i} {fairy_pos}"
                );
            }
        }
    }
}
