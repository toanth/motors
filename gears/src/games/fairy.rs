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

use crate::games::chess::castling::{CastleRight, CastlingFlags};
use crate::games::chess::pieces::{
    NUM_COLORS, UNICODE_BLACK_BISHOP, UNICODE_BLACK_KING, UNICODE_BLACK_KNIGHT, UNICODE_BLACK_PAWN,
    UNICODE_BLACK_QUEEN, UNICODE_BLACK_ROOK, UNICODE_NEUTRAL_BISHOP, UNICODE_NEUTRAL_KING,
    UNICODE_NEUTRAL_KNIGHT, UNICODE_NEUTRAL_PAWN, UNICODE_NEUTRAL_QUEEN, UNICODE_NEUTRAL_ROOK,
    UNICODE_WHITE_BISHOP, UNICODE_WHITE_KING, UNICODE_WHITE_KNIGHT, UNICODE_WHITE_PAWN,
    UNICODE_WHITE_QUEEN, UNICODE_WHITE_ROOK,
};
use crate::games::chess::ChessColor;
use crate::games::fairy::attacks::AttackKind::Normal;
use crate::games::fairy::attacks::AttackTypes::{Leaping, Sliding};
use crate::games::fairy::attacks::GenAttacksCondition::{Always, OnRank, Side};
use crate::games::fairy::attacks::{
    AttackBitboardFilter, AttackKind, AttackTypes, GenPieceAttackKind, LeapingBitboards,
    MoveEffect, MoveKind, SliderDirections,
};
use crate::games::{
    AbstractPieceType, BoardHistory, CharType, Color, ColoredPiece, ColoredPieceType, Coordinates,
    DimT, GenericPiece, PieceType, Settings, Size, ZobristHash,
};
use crate::general::bitboards::Axis::Vertical;
use crate::general::bitboards::{
    precompute_single_leaper_attacks, Bitboard, DynamicallySizedBitboard, ExtendedRawBitboard,
    RawBitboard, RawStandardBitboard,
};
use crate::general::board::{
    BitboardBoard, Board, BoardSize, ColPieceType, NameToPos, PieceTypeOf, SelfChecks, Strictness,
    UnverifiedBoard,
};
use crate::general::common::{EntityList, Res, StaticallyNamedEntity, Tokens};
use crate::general::move_list::{EagerNonAllocMoveList, MoveList};
use crate::general::moves::Move;
use crate::general::moves::{
    ExtendedFormat, ExtendedFormatter, Legality, MoveFlags, UntrustedMove,
};
use crate::general::squares::{
    GridCoordinates, GridSize, RectangularCoordinates, RectangularSize, SquareColor,
};
use crate::output::pgn::RoundNumber::Custom;
use crate::output::text_output::{board_to_string, BoardFormatter};
use crate::search::Depth;
use crate::{shift_left, PlayerResult};
use anyhow::bail;
use arbitrary::Arbitrary;
use arrayvec::ArrayVec;
use derive_more::{IntoIterator, Not};
use itertools::Itertools;
use rand::Rng;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::iter::once;
use std::ops::RangeInclusive;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use thread_local::ThreadLocal;

type RawFairyBitboard = ExtendedRawBitboard;
type FairyBitboard = DynamicallySizedBitboard<RawFairyBitboard, FairySquare>;

/// There can never be more than 32 piece types in a given game
/// (For chess, the number would be 6; for ataxx, 1).
/// Note that some effects can also be represented by one of these bitboards.
const MAX_NUM_PIECE_TYPES: usize = 16;

pub type FairySquare = GridCoordinates;
pub type FairySize = GridSize;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub struct PieceId(usize);

impl PieceId {
    pub fn piece(&self) -> &Piece {
        &rules().pieces[self.0]
    }
}

impl Display for PieceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char(CharType::Ascii))
    }
}

impl AbstractPieceType for PieceId {
    fn empty() -> Self {
        Self(usize::MAX)
    }

    fn to_char(self, typ: CharType) -> char {
        self.piece().uncolored_symbol[typ as usize]
    }

    fn from_char(c: char) -> Option<Self> {
        FAIRY_RULES
            .get()
            .unwrap()
            .pieces
            .iter()
            .find(|&p| p.uncolored_symbol.contains(&c))
            .map(|p| p.id)
    }

    fn to_uncolored_idx(self) -> usize {
        self.0
    }
}

impl PieceType<FairyBoard> for PieceId {
    type Colored = ColoredPieceId;

    fn from_idx(idx: usize) -> Self {
        Self(idx)
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
#[must_use]
pub struct ColoredPieceId {
    id: PieceId,
    color: Option<FairyColor>,
}

impl ColoredPieceId {
    pub fn piece(&self) -> &Piece {
        &rules().pieces[self.id.0]
    }
}

impl Display for ColoredPieceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let color = self
            .color
            .map(|c| rules().colors[c as usize].name.clone())
            .unwrap_or_default();
        write!(
            f,
            "{color} {0}",
            rules().pieces[self.id.0].uncolored_symbol[CharType::Ascii as usize],
        )
    }
}

impl AbstractPieceType for ColoredPieceId {
    fn empty() -> Self {
        Self {
            id: PieceId::empty(),
            color: None,
        }
    }

    fn to_char(self, typ: CharType) -> char {
        if let Some(color) = self.color {
            self.piece().player_symbol[color as usize][typ as usize]
        } else {
            self.piece().uncolored_symbol[typ as usize]
        }
    }

    fn from_char(c: char) -> Option<Self> {
        if let Some(p) = rules()
            .pieces
            .iter()
            .find(|&p| p.player_symbol.iter().any(|s| s.contains(&c)))
        {
            if p.player_symbol[0].contains(&c) {
                Some(Self {
                    id: p.id,
                    color: Some(FairyColor::First),
                })
            } else {
                Some(Self {
                    id: p.id,
                    color: Some(FairyColor::Second),
                })
            }
        } else if let Some(p) = rules()
            .pieces
            .iter()
            .find(|&p| p.uncolored_symbol.contains(&c))
        {
            Some(Self {
                id: p.id,
                color: None,
            })
        } else {
            None
        }
    }

    fn to_uncolored_idx(self) -> usize {
        self.id.0
    }
}

impl ColoredPieceType<FairyBoard> for ColoredPieceId {
    type Uncolored = PieceId;

    fn color(self) -> Option<FairyColor> {
        self.color
    }

    fn uncolor(self) -> Self::Uncolored {
        self.id
    }

    fn to_colored_idx(self) -> usize {
        self.id.0
    }

    fn new(color: FairyColor, uncolored: Self::Uncolored) -> Self {
        Self {
            id: uncolored,
            color: Some(color),
        }
    }
}

#[derive(Debug, Default)]
#[must_use]
struct Promo {
    pieces: Vec<PieceId>,
    squares: FairyBitboard,
}

/// This struct defines the rules for a single piece.
pub struct Piece {
    pub(super) name: String,
    pub(super) uncolored_symbol: [char; 2],
    pub(super) player_symbol: [[char; 2]; NUM_COLORS],
    pub(super) id: PieceId,
    // most of the attack data is represented with a bitboard. attacks are represented with bitboards and form the vast majority of moves.
    // For example, pawn promotions are normal pawn moves with a special effect applied to them.
    // However, some moves, like castling, are better represented
    pub(super) attacks: Vec<GenPieceAttackKind>,
    /// Promotions change the piece type and can differentiate moves with otherwise identical information.
    /// However, they are not the only way to change piece types; this can also done through move effects based on the move kind.
    pub(super) promotions: Promo,
    pub(super) royal: bool,
}

impl Piece {
    // TODO: Add cargo feature for fairy, requires chess
    pub fn king(id: PieceId, size: FairySize) -> Self {
        Self {
            name: "king".to_string(),
            uncolored_symbol: ['K', UNICODE_NEUTRAL_KING],
            player_symbol: [['k', UNICODE_BLACK_KING], ['K', UNICODE_WHITE_KING]],
            id,
            attacks: vec![GenPieceAttackKind::simple(Leaping(
                LeapingBitboards::fixed(1, 1, size).combine(LeapingBitboards::fixed(1, 0, size)),
            ))],
            promotions: Promo::default(),
            royal: true,
        }
    }
    pub fn queen(id: PieceId) -> Self {
        Self {
            name: "queen".to_string(),
            uncolored_symbol: ['Q', UNICODE_NEUTRAL_QUEEN],
            player_symbol: [['q', UNICODE_BLACK_QUEEN], ['Q', UNICODE_WHITE_QUEEN]],
            id,
            attacks: vec![GenPieceAttackKind::simple(Sliding(SliderDirections::Queen))],
            promotions: Promo::default(),
            royal: false,
        }
    }
    pub fn rook(id: PieceId) -> Self {
        Self {
            name: "rook".to_string(),
            uncolored_symbol: ['R', UNICODE_NEUTRAL_ROOK],
            player_symbol: [['r', UNICODE_BLACK_ROOK], ['R', UNICODE_WHITE_ROOK]],
            id,
            attacks: vec![GenPieceAttackKind::simple(Sliding(SliderDirections::Rook))],
            promotions: Promo::default(),
            royal: false,
        }
    }
    pub fn bishop(id: PieceId) -> Self {
        Self {
            name: "bishop".to_string(),
            uncolored_symbol: ['B', UNICODE_NEUTRAL_BISHOP],
            player_symbol: [['b', UNICODE_BLACK_BISHOP], ['B', UNICODE_WHITE_BISHOP]],
            id,
            attacks: vec![GenPieceAttackKind::simple(Sliding(
                SliderDirections::Bishop,
            ))],
            promotions: Promo::default(),
            royal: false,
        }
    }
    pub fn knight(id: PieceId, size: FairySize) -> Self {
        Self {
            name: "knight".to_string(),
            uncolored_symbol: ['N', UNICODE_NEUTRAL_KNIGHT],
            player_symbol: [['n', UNICODE_BLACK_KNIGHT], ['N', UNICODE_WHITE_KNIGHT]],
            id,
            attacks: vec![GenPieceAttackKind::simple(AttackTypes::leaping(1, 2, size))],
            promotions: Promo::default(),
            royal: false,
        }
    }
    pub fn pawn(id: PieceId, size: FairySize) -> Self {
        let normal_white = GenPieceAttackKind {
            typ: Leaping(LeapingBitboards::range(once(0), once(1), size)),
            condition: Side(FairyColor::Second),
            bitboard_filter: AttackBitboardFilter::EmptySquares,
            kind: Normal,
        };
        let normal_black = GenPieceAttackKind {
            typ: Leaping(LeapingBitboards::range(once(0), once(-1), size)),
            condition: Side(FairyColor::Second),
            bitboard_filter: AttackBitboardFilter::EmptySquares,
            kind: Normal,
        };
        let white_capture = GenPieceAttackKind {
            typ: Leaping(LeapingBitboards::range([-1, 1].into_iter(), once(1), size)),
            condition: Side(FairyColor::First),
            bitboard_filter: AttackBitboardFilter::PawnCapture,
            kind: Normal,
        };
        let black_capture = GenPieceAttackKind {
            typ: Leaping(LeapingBitboards::range([-1, 1].into_iter(), once(-1), size)),
            condition: Side(FairyColor::Second),
            bitboard_filter: AttackBitboardFilter::PawnCapture,
            kind: Normal,
        };
        // promotions are handled as effects instead of duplicating all normal and capture moves
        let white_double = GenPieceAttackKind {
            typ: Leaping(LeapingBitboards::range(once(0), once(2), size)),
            condition: OnRank(1, FairyColor::First),
            bitboard_filter: AttackBitboardFilter::EmptySquares,
            kind: Normal,
        };
        let black_double = GenPieceAttackKind {
            typ: Leaping(LeapingBitboards::range(once(0), once(-2), size)),
            condition: OnRank(size.height().val() - 1, FairyColor::Second),
            bitboard_filter: AttackBitboardFilter::EmptySquares,
            kind: Normal,
        };
        Self {
            name: "pawn".to_string(),
            uncolored_symbol: ['p', UNICODE_NEUTRAL_PAWN],
            player_symbol: [['p', UNICODE_BLACK_PAWN], ['P', UNICODE_WHITE_PAWN]],
            id,

            attacks: vec![
                normal_white,
                normal_black,
                white_double,
                black_double,
                white_capture,
                black_capture,
            ],
            // the promotion piecesa are set later, once it's known which pieces are available
            promotions: Promo {
                pieces: vec![],
                squares: FairyBitboard::backranks_for(size),
            },
            royal: false,
        }
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub struct FairyMove {
    from: FairySquare,
    to: FairySquare,
    piece: PieceId,
    kind: MoveKind,
}

impl Display for FairyMove {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.format_compact(f)
    }
}

impl Move<FairyBoard> for FairyMove {
    type Flags = MoveKind;
    type Underlying = usize; // TODO: u64

    fn is_null(self) -> bool {
        todo!()
    }

    fn legality() -> Legality {
        rules().legality
    }

    fn src_square(self) -> FairySquare {
        self.from
    }

    fn dest_square(self) -> FairySquare {
        self.to
    }

    fn flags(self) -> Self::Flags {
        self.kind
    }

    fn is_tactical(self, board: &FairyBoard) -> bool {
        todo!()
    }

    fn format_compact(self, f: &mut Formatter<'_>) -> fmt::Result {
        todo!()
    }

    fn format_extended(
        self,
        f: &mut Formatter<'_>,
        _board: &FairyBoard,
        _format: ExtendedFormat,
    ) -> fmt::Result {
        todo!()
    }

    fn extended_formatter(
        self,
        pos: FairyBoard,
        format: ExtendedFormat,
    ) -> ExtendedFormatter<FairyBoard> {
        todo!()
    }

    fn to_extended_text(self, board: &FairyBoard, format: ExtendedFormat) -> String {
        todo!()
    }

    fn parse_compact_text<'a>(s: &'a str, board: &FairyBoard) -> Res<(&'a str, FairyMove)> {
        todo!()
    }

    fn from_compact_text(s: &str, board: &FairyBoard) -> Res<FairyMove> {
        todo!()
    }

    fn parse_extended_text<'a>(s: &'a str, board: &FairyBoard) -> Res<(&'a str, FairyMove)> {
        todo!()
    }

    fn from_extended_text(s: &str, board: &FairyBoard) -> Res<FairyMove> {
        todo!()
    }

    fn from_text(s: &str, board: &FairyBoard) -> Res<FairyMove> {
        todo!()
    }

    // TODO: Doesn't really make sense, at least use u64
    fn from_usize_unchecked(val: usize) -> UntrustedMove<FairyBoard> {
        todo!()
    }

    fn to_underlying(self) -> Self::Underlying {
        todo!()
    }
}

/// Maximum number of pseudolegal moves in a position
const MAX_MOVES: usize = 1024;

type FairyMoveList = EagerNonAllocMoveList<FairyBoard, MAX_MOVES>;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum GameLoss {
    Checkmate,
    NoRoyals,
    NoPieces,
    NoMoves,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Draw {
    NoMoves,
    Counter,
    Repetition(usize),
}

#[derive(
    Debug, Default, Copy, Clone, Eq, PartialEq, derive_more::Display, Hash, EnumIter, Arbitrary,
)]
#[must_use]
pub enum FairyColor {
    #[default]
    First,
    Second,
}

impl Not for FairyColor {
    type Output = Self;

    fn not(self) -> Self::Output {
        self.other()
    }
}

impl Color for FairyColor {
    fn other(self) -> Self {
        match self {
            FairyColor::First => FairyColor::Second,
            FairyColor::Second => FairyColor::First,
        }
    }

    fn color_char(self, _typ: CharType) -> char {
        rules().colors[self as usize].ascii_char
    }
}

impl FairyColor {
    fn to_chess_color(self) -> ChessColor {
        match self {
            FairyColor::First => ChessColor::White,
            FairyColor::Second => ChessColor::Black,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct ColorInfo {
    ascii_char: char,
    name: String,
}

/// This struct defined the rules for each piece.
/// Since the rules don't change during a game, but are expensive to copy and the board uses copy-make,
/// the board contains an `Rc` to the rules
#[must_use]
struct Rules {
    pieces: ArrayVec<Piece, MAX_NUM_PIECE_TYPES>,
    colors: [ColorInfo; NUM_COLORS],
    counter: usize,
    move_number: usize,
    game_loss: GameLoss,
    draw: Draw,
    startpos_fen: String,
    legality: Legality,
    size: GridSize,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct RulesRef {}

impl Settings for RulesRef {
    fn text(&self) -> Option<String> {
        todo!()
    }
}

/// The least bad option to implement rules.
/// In the future, it might make sense to explore an implementation where each piece, move, etc, contains
/// a reference / Rc to the rules
static FAIRY_RULES: ThreadLocal<Rules> = ThreadLocal::new();

// this function is a lot slower than just reading a variable, but speed isn't the largest concern for fairy chess anyway.
// TODO: Still, it might be worth to test if caching the rules improves elo. The major drawback would be the possibility of panics
// if a cached entry still exists when the rules are getting changed
fn rules() -> &'static Rules {
    FAIRY_RULES.get().unwrap()
}

/// A FairyBoard is a retangular board for a chess-like variant.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub struct UnverifiedFairyBoard {
    // unfortunately, ArrayVec isn't `Copy`
    piece_bitboards: [FairyBitboard; MAX_NUM_PIECE_TYPES],
    color_bitboards: [FairyBitboard; NUM_COLORS],
    ply_since_start: usize,
    // like the 50mr counter in chess TODO: Maybe make it count down?
    num_piece_bitboards: usize,
    draw_counter: usize,
    active: FairyColor,
    // store info for each supported special move
    castling_flags: CastlingFlags,
    ep: Option<FairySquare>,
}

impl UnverifiedFairyBoard {
    fn empty_bb(&self) -> FairyBitboard {
        !(self.color_bitboards[0] | self.color_bitboards[1])
    }
}

impl From<FairyBoard> for UnverifiedFairyBoard {
    fn from(value: FairyBoard) -> Self {
        value.0
    }
}

impl UnverifiedBoard<FairyBoard> for UnverifiedFairyBoard {
    fn verify_with_level(self, level: SelfChecks, strictness: Strictness) -> Res<FairyBoard> {
        todo!()
    }

    fn size(&self) -> BoardSize<FairyBoard> {
        rules().size
    }

    fn place_piece(mut self, coords: FairySquare, piece: ColPieceType<FairyBoard>) -> Self {
        let bb = self.single_piece(coords);
        self.piece_bitboards[piece.id.0] |= bb;
        if let Some(color) = piece.color() {
            self.color_bitboards[color as usize] |= bb;
        }
        self
    }

    fn remove_piece(mut self, coords: FairySquare) -> Self {
        let idx = self.idx(coords);
        let bb = self.single_piece(coords);
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
        self
    }

    fn piece_on(&self, coords: FairySquare) -> <FairyBoard as Board>::Piece {
        let idx = self.idx(coords);
        let piece = self
            .piece_bitboards
            .iter()
            .find_position(|bb| bb.is_bit_set_at(idx))
            .map(|(idx, _bb)| PieceId(idx))
            .unwrap_or(PieceId::empty());
        let color = self
            .color_bitboards
            .iter()
            .find_position(|bb| bb.is_bit_set_at(idx))
            .map(|(idx, _bb)| FairyColor::iter().nth(idx).unwrap());

        GenericPiece::new(ColoredPieceId { id: piece, color }, coords)
    }

    fn is_empty(&self, coords: FairySquare) -> bool {
        self.empty_bb().is_bit_set_at(self.idx(coords))
    }

    fn set_active_player(mut self, player: FairyColor) -> Self {
        self.active = player;
        self
    }

    fn set_ply_since_start(mut self, ply: usize) -> Res<Self> {
        self.ply_since_start = ply;
        Ok(self)
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub struct FairyBoard(UnverifiedFairyBoard);

impl Default for FairyBoard {
    fn default() -> Self {
        todo!()
    }
}

impl Display for FairyBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_fen())
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
        todo!()
    }

    fn startpos_for_settings(settings: Self::Settings) -> Self {
        todo!()
    }

    fn startpos_with_current_settings(self) -> Self {
        todo!()
    }

    fn startpos() -> Self {
        todo!()
    }

    fn from_name(name: &str) -> Res<Self> {
        todo!()
    }

    fn name_to_pos_map() -> EntityList<NameToPos<Self>> {
        todo!()
    }

    fn bench_positions() -> Vec<Self> {
        todo!()
    }

    fn settings(&self) -> Self::Settings {
        RulesRef::default()
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
        if let Some(color) = piece.color {
            self.colored_piece_bb(color, piece.id).is_bit_set_at(idx)
        } else {
            self.piece_bb(piece.id).is_bit_set_at(idx)
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
            PieceId(idx)
        } else {
            PieceId::empty()
        }
    }

    fn default_perft_depth(&self) -> Depth {
        Depth::new(3)
    }

    fn gen_pseudolegal<T: MoveList<Self>>(&self, moves: &mut T) {
        let color = self.active_player();
        for piece in &rules().pieces {
            let bb = self.colored_piece_bb(self.active_player(), piece.id);
            for start in bb.ones() {
                for attack_kind in &piece.attacks {
                    let piece = FairyPiece {
                        symbol: ColoredPieceId::new(color, piece.id),
                        coordinates: start,
                    };
                    let bb = attack_kind.attacks(piece, self);
                    bb.insert_moves(moves, self, piece);
                }
            }
        }
    }

    fn gen_tactical_pseudolegal<T: MoveList<Self>>(&self, moves: &mut T) {
        todo!()
    }

    fn legal_moves_slow(&self) -> Self::MoveList {
        todo!()
    }

    fn random_legal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
        todo!()
    }

    fn random_pseudolegal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
        todo!()
    }

    fn make_move(self, mov: Self::Move) -> Option<Self> {
        todo!()
    }

    fn make_nullmove(self) -> Option<Self> {
        todo!()
    }

    fn is_move_pseudolegal(&self, mov: Self::Move) -> bool {
        todo!()
    }

    fn is_move_legal(&self, mov: Self::Move) -> bool {
        todo!()
    }

    fn is_pseudolegal_move_legal(&self, mov: Self::Move) -> bool {
        todo!()
    }

    fn player_result_no_movegen<H: BoardHistory<Self>>(&self, history: &H) -> Option<PlayerResult> {
        todo!()
    }

    fn player_result_slow<H: BoardHistory<Self>>(&self, history: &H) -> Option<PlayerResult> {
        todo!()
    }

    fn no_moves_result(&self) -> PlayerResult {
        todo!()
    }

    fn is_game_lost_slow(&self) -> bool {
        todo!()
    }

    fn is_game_won_after_slow(&self, mov: Self::Move) -> bool {
        todo!()
    }

    fn can_reasonably_win(&self, player: Self::Color) -> bool {
        true
    }

    fn zobrist_hash(&self) -> ZobristHash {
        todo!()
    }

    fn as_fen(&self) -> String {
        todo!()
    }

    fn read_fen_and_advance_input(input: &mut Tokens, strictness: Strictness) -> Res<Self> {
        todo!()
    }

    fn should_flip_visually() -> bool {
        true
    }

    fn as_diagram(&self, typ: CharType, flip: bool) -> String {
        board_to_string(self, GenericPiece::to_char, typ, flip)
    }

    fn display_pretty(&self, formatter: &mut dyn BoardFormatter<Self>) -> String {
        todo!()
    }

    fn pretty_formatter(
        &self,
        piece: Option<CharType>,
        last_move: Option<Self::Move>,
    ) -> Box<dyn BoardFormatter<Self>> {
        todo!()
    }

    fn background_color(&self, coords: Self::Coordinates) -> SquareColor {
        todo!()
    }
}

impl BitboardBoard for FairyBoard {
    type RawBitboard = RawFairyBitboard;
    type Bitboard = FairyBitboard;

    fn piece_bb(&self, piece: PieceTypeOf<Self>) -> Self::Bitboard {
        self.0.piece_bitboards[piece.0]
    }

    fn player_bb(&self, color: Self::Color) -> Self::Bitboard {
        self.0.color_bitboards[color as usize]
    }
}
