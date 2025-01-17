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

use crate::games::fairy::attacks::AttackBitboardFilter::{EmptySquares, NotUs};
use crate::games::fairy::attacks::AttackKind::Normal;
use crate::games::fairy::attacks::AttackTypes::{Leaping, Rider};
use crate::games::fairy::attacks::GenAttacksCondition::Always;
use crate::games::fairy::moves::FairyMove;
use crate::games::fairy::pieces::ColoredPieceId;
use crate::games::fairy::rules::CheckRules;
use crate::games::fairy::Side::{Kingside, Queenside};
use crate::games::fairy::{
    CastlingMoveInfo, FairyBitboard, FairyBoard, FairyCastleInfo, FairyColor, FairyPiece, FairySize, FairySquare,
    RawFairyBitboard, Side, UnverifiedFairyBoard,
};
use crate::games::{char_to_file, Color, ColoredPiece, ColoredPieceType, DimT, Size};
use crate::general::bitboards::{Bitboard, RawBitboard, RayDirections};
use crate::general::board::{BitboardBoard, Board, BoardHelpers, PieceTypeOf, Strictness, UnverifiedBoard};
use crate::general::common::{Res, Tokens};
use crate::general::move_list::MoveList;
use crate::general::squares::{CompactSquare, RectangularCoordinates, RectangularSize};
use crate::{precompute_leaper_attacks, shift_left};
use anyhow::{bail, ensure};
use arbitrary::Arbitrary;
use arrayvec::ArrayVec;
use crossterm::style::Stylize;
use std::str::FromStr;
use strum::IntoEnumIterator;

///
/// The general organization of movegen is that of a pipeline, where a stage communicates with the next through enums,
/// which usually need the global `rules()` to be interpreted correctly
#[derive(Debug, Clone, Arbitrary)]
pub enum SliderDirections {
    Vertical,
    Rook,
    Bishop,
    Queen,
    Rider { precomputed: Box<[RawFairyBitboard]> },
}

// not const, which allows using ranges and for loops
pub fn leaper_attack_range<Iter1: Iterator<Item = isize>, Iter2: Iterator<Item = isize> + Clone>(
    horizontal_range: Iter1,
    vertical_range: Iter2,
    square: FairySquare,
    size: FairySize,
) -> RawFairyBitboard {
    let mut res = RawFairyBitboard::default();
    let width = size.width().val() as isize;
    let internal_width = size.internal_width() as isize;
    for dx in horizontal_range {
        for dy in vertical_range.clone() {
            let shift = dx + dy * internal_width;
            let bb = FairyBitboard::single_piece_for(square, size);
            if square.file() as isize >= -dx && square.file() as isize + dx < width {
                res |= shift_left!(bb.raw(), shift);
            }
        }
    }
    res
}

#[derive(Debug, Arbitrary)]
pub struct LeapingBitboards(Box<[RawFairyBitboard]>);

fn leaper(n: usize, m: usize, rider: bool, size: FairySize) -> Box<[RawFairyBitboard]> {
    let mut res = vec![RawFairyBitboard::default(); size.num_squares()].into_boxed_slice();
    let (n, m) = (n.min(m), n.max(m));
    for idx in 0..size.num_squares() {
        let bb = precompute_leaper_attacks!(idx, n, m, rider, size.width.val(), u128);
        res[idx] = bb;
    }
    res
}

impl LeapingBitboards {
    pub(super) fn fixed(n: usize, m: usize, size: FairySize) -> Self {
        Self(leaper(n, m, false, size))
    }

    pub(super) fn range<Iter1: Iterator<Item = isize> + Clone, Iter2: Iterator<Item = isize> + Clone>(
        horizontal_range: Iter1,
        vertical_range: Iter2,
        size: FairySize,
    ) -> Self {
        let mut res = vec![RawFairyBitboard::default(); size.num_squares()].into_boxed_slice();
        for idx in 0..size.num_squares() {
            let sq = size.idx_to_coordinates(idx as DimT);
            let bb = leaper_attack_range(horizontal_range.clone(), vertical_range.clone(), sq, size);
            res[idx] = bb;
        }
        LeapingBitboards(res)
    }

    pub(super) fn combine(mut self, other: LeapingBitboards) -> Self {
        assert_eq!(self.0.len(), other.0.len());
        self.0.iter_mut().zip(other.0.iter()).for_each(|(a, b)| *a |= b);
        self
    }
    pub(super) fn remove(mut self, other: LeapingBitboards) -> Self {
        assert_eq!(self.0.len(), other.0.len());
        self.0.iter_mut().zip(other.0.iter()).for_each(|(a, b)| *a &= !b);
        self
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
pub enum AttackMode {
    All,
    Captures,
    NoCaptures,
}

impl AttackMode {
    pub fn generate_for_mode(self, other: Self) -> bool {
        self == other || self == AttackMode::All || other == AttackMode::All
    }
}

#[must_use]
#[derive(Debug, Arbitrary)]
pub enum AttackTypes {
    // Leaping pieces, like knights and kings, only care about blockers on the square they're leaping to
    Leaping(LeapingBitboards),
    // Riders are generalized sliders
    Rider(SliderDirections),
    // Castling moves are special enough that it makes sense to handle them separately
    Castling(Side),
    Drop,
    // HardCoded {
    //     source: FairySquare,
    //     target: FairySquare,
    // },
}

impl AttackTypes {
    pub fn leaping(n: usize, m: usize, size: FairySize) -> Self {
        Leaping(LeapingBitboards::fixed(n, m, size))
    }
    pub fn rider(n: usize, m: usize, size: FairySize) -> Self {
        let bbs = leaper(n, m, true, size);
        Rider(SliderDirections::Rider { precomputed: bbs })
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub enum AttackKind {
    #[default]
    Normal,
    DoublePawnPush,
    Castle(Side),
    Drop,
}

impl AttackKind {
    pub fn to_move_kind(self, piece: ColoredPieceId) -> MoveKind {
        match self {
            Self::Normal => MoveKind::Normal,
            Self::DoublePawnPush => MoveKind::DoublePawnPush,
            Self::Castle(side) => MoveKind::Castle(side),
            AttackKind::Drop => MoveKind::Drop(piece.as_u8()),
        }
    }
}

#[derive(Debug, Copy, Clone, Arbitrary)]
#[must_use]
pub enum RequiredForAttack {
    PieceOnBoard,
    // This is mostly the same as piece drop attacks, although there can be drops that also require a piece on the board,
    // such as ataxx cloning moves.
    PieceInHand,
}

/// Attacks are about bitboards for performance reasons, but there are also moves that aren't fully represented with bitboards.
/// This struct is only about attacks, so there are move types that are generated separately
#[must_use]
#[derive(Debug, Arbitrary)]
pub struct GenPieceAttackKind {
    // first, we distinguish between moving attacks (e.g. chess moves) and drops (e.g. mnk games)
    pub required: RequiredForAttack,
    // then, the condition is checked (e.g., pawns can only double push on their start rank)
    pub condition: GenAttacksCondition,
    // and the attack kind may also be disabled based on the mode, for example pawn pushes don't capture
    pub attack_mode: AttackMode,
    // then, the bitboard of attacks is generated
    pub typ: AttackTypes,
    // it is then filtered (e.g., that's how pawn captures of opponent pieces and ep squares are done,
    // and also how double pawn pushes are generated: They're vertical sliders).
    // A square needs to pass all the filters, that is, the filters get combined using a `bitwise and`.
    pub bitboard_filter: Vec<AttackBitboardFilter>,
    // this is annotated with the move kind (e.g. castling)
    pub kind: AttackKind,
    // this is used to decide if a move is a capture
    pub capture_condition: CaptureCondition,
}

impl GenPieceAttackKind {
    pub fn simple(typ: AttackTypes) -> Self {
        Self {
            required: RequiredForAttack::PieceOnBoard,
            typ,
            condition: Always,
            bitboard_filter: vec![NotUs],
            kind: Normal,
            attack_mode: AttackMode::All,
            capture_condition: CaptureCondition::DestOccupied,
        }
    }
    pub fn pawn_noncapture(typ: AttackTypes, condition: GenAttacksCondition) -> Self {
        Self {
            required: RequiredForAttack::PieceOnBoard,
            typ,
            condition,
            bitboard_filter: vec![EmptySquares],
            kind: Normal,
            attack_mode: AttackMode::NoCaptures,
            capture_condition: CaptureCondition::Never,
        }
    }
    pub fn pawn_capture(typ: AttackTypes, condition: GenAttacksCondition, bb_filter: AttackBitboardFilter) -> Self {
        Self {
            required: RequiredForAttack::PieceOnBoard,
            typ,
            condition,
            bitboard_filter: vec![bb_filter],
            kind: Normal,
            attack_mode: AttackMode::Captures,
            capture_condition: CaptureCondition::Always,
        }
    }
    pub fn piece_drop(filter: AttackBitboardFilter) -> Self {
        Self {
            required: RequiredForAttack::PieceInHand,
            condition: Always,
            attack_mode: AttackMode::NoCaptures,
            typ: AttackTypes::Drop,
            bitboard_filter: vec![filter],
            kind: AttackKind::Drop,
            capture_condition: CaptureCondition::Never,
        }
    }

    pub fn bb_filter(&self, us: FairyColor, pos: &FairyBoard) -> RawFairyBitboard {
        let mut res = !RawFairyBitboard::default();
        for filter in &self.bitboard_filter {
            res &= filter.bb(us, pos);
        }
        res
    }

    pub fn attacks_for_blockers(
        &self,
        piece: FairyPiece,
        blockers: FairyBitboard,
        filter_bb: RawFairyBitboard,
        squares_mask: RawFairyBitboard,
        pos: &FairyBoard,
    ) -> PieceAttackBB {
        let piece_id = piece.symbol;
        let piece = piece.coordinates;
        let size = blockers.size();
        let res = match &self.typ {
            Leaping(precomputed) => precomputed.0[size.internal_key(piece)],
            Rider(sliding) => {
                let blockers = FairyBitboard::new(
                    blockers.raw() & !RawFairyBitboard::single_piece_at(size.internal_key(piece)),
                    size,
                );
                let res = match sliding {
                    SliderDirections::Vertical => FairyBitboard::vertical_attacks(piece, blockers),
                    SliderDirections::Rook => FairyBitboard::rook_attacks(piece, blockers),
                    SliderDirections::Bishop => FairyBitboard::bishop_attacks(piece, blockers),
                    SliderDirections::Queen => FairyBitboard::queen_attacks(piece, blockers),
                    SliderDirections::Rider { precomputed } => {
                        let ray = FairyBitboard::new(precomputed[size.internal_key(piece)], size);
                        // TODO: Allow horizontal
                        FairyBitboard::hyperbola_quintessence_non_horizontal(piece, blockers, ray)
                    }
                };
                res.raw()
            }
            // &AttackTypes::HardCoded { source, target } => {
            //     if source == piece {
            //         FairyBitboard::single_piece_for(target, size).raw()
            //     } else {
            //         RawFairyBitboard::default()
            //     }
            // }
            &AttackTypes::Castling(side) => {
                if let Some(sq) = pos.0.castling_info.player(piece_id.color().unwrap()).king_dest_sq(side) {
                    FairyBitboard::single_piece_for(sq, size).raw()
                } else {
                    RawFairyBitboard::default()
                }
            }
            &AttackTypes::Drop => pos.empty_bb().raw(),
        };
        PieceAttackBB {
            all_attacks: res & squares_mask,
            kind: self.kind,
            piece: piece_id,
            filter_bb,
            capture_condition: self.capture_condition,
        }
    }

    fn check_conditions(&self, piece: FairyPiece, pos: &FairyBoard, mode: AttackMode) -> bool {
        if !self.attack_mode.generate_for_mode(mode) {
            return false;
        }
        match self.condition {
            Always => true,
            GenAttacksCondition::Player(color) => piece.color().is_some_and(|c| c == color),
            GenAttacksCondition::CanCastle(side) => pos.0.castling_info.can_castle(pos.active_player(), side),
            GenAttacksCondition::OnRank(rank, color) => {
                piece.color().is_some_and(|c| c == color) && piece.coordinates.rank() == rank
            }
        }
    }

    pub fn attacks(&self, piece: FairyPiece, pos: &FairyBoard, mode: AttackMode) -> Option<PieceAttackBB> {
        if !self.check_conditions(piece, pos, mode) {
            return None;
        }
        let filter = self.bb_filter(piece.color().unwrap(), pos);
        let blockers = pos.blocker_bb();
        Some(self.attacks_for_blockers(piece, blockers, filter, pos.0.mask_bb, pos))
    }
}

/// When is a given move a capture?
#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
pub enum CaptureCondition {
    DestOccupied,
    Always,
    Never,
}

#[must_use]
pub struct PieceAttackBB {
    pub all_attacks: RawFairyBitboard,
    pub filter_bb: RawFairyBitboard,
    pub kind: AttackKind,
    pub piece: ColoredPieceId,
    pub capture_condition: CaptureCondition,
}

impl PieceAttackBB {
    pub fn bb(&self) -> RawFairyBitboard {
        self.all_attacks & self.filter_bb
    }
    fn is_capture(&self, to: FairySquare, pos: &FairyBoard) -> bool {
        match self.capture_condition {
            CaptureCondition::DestOccupied => pos.is_occupied(to),
            CaptureCondition::Always => true,
            CaptureCondition::Never => false,
        }
    }
    pub fn insert_moves<L: MoveList<FairyBoard>>(&self, list: &mut L, pos: &FairyBoard, piece: FairyPiece) {
        let bb = FairyBitboard::new(self.bb(), pos.size());
        let from = piece.coordinates;
        for to in bb.ones() {
            let mut move_kinds = ArrayVec::new();
            MoveKind::insert(&mut move_kinds, self, to, pos);
            for kind in move_kinds {
                let is_capture = self.is_capture(to, pos);
                let mov = FairyMove {
                    from: CompactSquare::new(from, pos.size()),
                    to: CompactSquare::new(to, pos.size()),
                    packed: FairyMove::pack(kind, is_capture),
                };
                list.add_move(mov);
            }
        }
    }
}

// no pont in making this a trait as I don't want it to be extensible like at compile time
// restriction: don't use traits or Box<dyn Fn> for "extensibility", just use enums.
/// Bitand the generated attack bitboard with a bitboard given by this enum
#[derive(Debug, Copy, Clone, Arbitrary)]
#[must_use]
pub enum AttackBitboardFilter {
    EmptySquares,
    Them,
    // Us,
    NotUs,
    // NotThem,
    Rank(DimT),
    // File(DimT),
    PawnCapture, // Them | {ep_square}
                 // Custom(RawFairyBitboard),
}

impl AttackBitboardFilter {
    pub fn bb(self, us: FairyColor, pos: &FairyBoard) -> RawFairyBitboard {
        let bb = match self {
            AttackBitboardFilter::EmptySquares => pos.empty_bb(),
            AttackBitboardFilter::Them => pos.player_bb(!us),
            // AttackBitboardFilter::Us => pos.player_bb(us),
            NotUs => !pos.player_bb(us),
            // AttackBitboardFilter::NotThem => !pos.player_bb(!us),
            AttackBitboardFilter::Rank(rank) => FairyBitboard::rank_for(rank, pos.size()),
            // AttackBitboardFilter::File(file) => FairyBitboard::file_for(file, pos.size()),
            AttackBitboardFilter::PawnCapture => {
                let ep_bb =
                    pos.0.ep.map(|sq| FairyBitboard::single_piece_for(sq, pos.size()).raw()).unwrap_or_default();
                return ep_bb | pos.player_bb(!us).raw();
            } // AttackBitboardFilter::Custom(bb) => return bb,
        };
        bb.raw()
    }
}

#[must_use]
#[derive(Debug, Arbitrary)]
pub enum GenAttacksCondition {
    Always,
    Player(FairyColor),
    CanCastle(Side),
    OnRank(DimT, FairyColor),
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub enum MoveKind {
    #[default]
    Normal,
    DoublePawnPush,
    // the given piece appears at the target square, like in m,n,k games or ataxx clones
    Drop(u8),
    // like Drop, but the piece on the source square disappears. Used for chess promotions
    ChangePiece(u8),
    Castle(Side),
    Conversion,
}

// this is also an upper bound of the number of pieces a pawn can promote to
pub const MAX_MOVE_KINDS_PER_ATTACK: usize = 32;

impl MoveKind {
    fn insert(
        list: &mut ArrayVec<MoveKind, MAX_MOVE_KINDS_PER_ATTACK>,
        attack: &PieceAttackBB,
        target: FairySquare,
        pos: &FairyBoard,
    ) {
        let promos = &pos.rules().pieces[attack.piece.uncolor().val()].promotions;
        if !promos.squares.is_bit_set_at(pos.size().internal_key(target)) {
            list.push(attack.kind.to_move_kind(attack.piece));
        } else {
            for &piece in &promos.pieces {
                let id = ColoredPieceId::new(pos.active_player(), piece);
                list.push(MoveKind::ChangePiece(id.as_u8()));
            }
        }
    }
}

/// Effect rules are stored in the rules and are used to determine the effect of each move.
#[derive(Debug, Copy, Clone, Arbitrary)]
pub struct EffectRules {
    pub reset_draw_counter_on_capture: bool,
    pub conversion_radius: usize,
    // pub explosion_radius: usize, // TODO: Atomic chess
}

impl Default for EffectRules {
    fn default() -> Self {
        Self {
            reset_draw_counter_on_capture: true,
            conversion_radius: 0,
            // explosion_radius: 0,
        }
    }
}

impl UnverifiedFairyBoard {
    pub(super) fn zero_bitboard(&self) -> FairyBitboard {
        FairyBitboard::new(RawFairyBitboard::default(), self.size)
    }

    pub(super) fn blocker_bb(&self) -> FairyBitboard {
        // TODO: Some games and piece types can modify this
        self.occupied_bb()
    }

    pub(super) fn piece_bb(&self, piece: PieceTypeOf<FairyBoard>) -> FairyBitboard {
        FairyBitboard::new(self.piece_bitboards[piece.val()], self.size)
    }

    pub(super) fn player_bb(&self, color: FairyColor) -> FairyBitboard {
        FairyBitboard::new(self.color_bitboards[color.idx()], self.size())
    }

    pub(super) fn mask_bb(&self) -> FairyBitboard {
        FairyBitboard::new(self.mask_bb, self.size())
    }

    pub fn royal_bb(&self) -> FairyBitboard {
        let mut res = self.zero_bitboard();
        for piece in self.rules().royals() {
            res |= self.piece_bb(piece)
        }
        res
    }

    pub fn royal_bb_for(&self, color: FairyColor) -> FairyBitboard {
        self.royal_bb() & self.player_bb(color)
    }

    /// In normal chess, this is the king bitboard, but not the rook bitboard
    pub fn castling_bb(&self) -> FairyBitboard {
        let mut res = self.zero_bitboard();
        for piece in self.rules().castling() {
            res |= self.piece_bb(piece)
        }
        res
    }
    pub fn castling_bb_for(&self, color: FairyColor) -> FairyBitboard {
        self.castling_bb() & self.player_bb(color)
    }
}

impl FairyBoard {
    // only includes capturing attacks, so no pawn pushes
    pub fn capturing_attack_bb_of(&self, color: FairyColor) -> FairyBitboard {
        let mut res = RawFairyBitboard::default();
        let f = |_piece: FairyPiece, bb: &PieceAttackBB| res |= bb.all_attacks;
        self.gen_attacks_impl(f, color, AttackMode::Captures);
        FairyBitboard::new(res, self.size())
    }

    pub(super) fn gen_pseudolegal_impl<T: MoveList<Self>>(&self, moves: &mut T) {
        let f = |piece: FairyPiece, bb: &PieceAttackBB| {
            bb.insert_moves(moves, self, piece);
        };
        self.gen_attacks_impl(f, self.active_player(), AttackMode::All);
    }

    fn gen_attacks_impl<F: FnMut(FairyPiece, &PieceAttackBB)>(&self, mut f: F, color: FairyColor, mode: AttackMode) {
        for (id, piece_type) in self.rules().pieces() {
            for attack_kind in &piece_type.attacks {
                match attack_kind.required {
                    RequiredForAttack::PieceOnBoard => {
                        let bb = self.colored_piece_bb(color, id);
                        for start in bb.ones() {
                            let piece = FairyPiece { symbol: ColoredPieceId::new(color, id), coordinates: start };
                            if let Some(bb) = attack_kind.attacks(piece, self, mode) {
                                f(piece, &bb);
                            }
                        }
                    }
                    RequiredForAttack::PieceInHand => {
                        if self.0.in_hand[id.val()] > 0 {
                            let piece = FairyPiece {
                                symbol: ColoredPieceId::new(color, id),
                                coordinates: FairySquare::no_coordinates(),
                            };
                            if let Some(bb) = attack_kind.attacks(piece, self, mode) {
                                f(piece, &bb);
                            }
                        }
                    }
                }
            }
        }
    }

    // Returns a bitboard of all royal pieces that are in check.
    // Technically, we should generate all moves and see if one of them removes a royal piece from the board.
    // However, games where attack bitboards aren't sufficient, like atomic chess, generally don't have forced check-evasion rules
    // that depend on those effects.
    // For most games, a superpiece method could work, but that's an optimization for later. For now, just computing all the attacks is simpler,
    // more robust, and good enough
    pub fn in_check_bb(&self, color: FairyColor) -> FairyBitboard {
        let royals = self.royal_bb_for(color);
        let their_attacks = self.capturing_attack_bb_of(color.other());
        royals & their_attacks
    }

    pub fn is_player_in_check(&self, color: FairyColor) -> bool {
        let rule = self.rules().check_rules;
        let in_check = self.in_check_bb(color);
        match rule {
            CheckRules::AllRoyals => in_check == self.royal_bb_for(color),
            CheckRules::AnyRoyal => in_check.has_set_bit(),
        }
    }

    pub fn is_in_check(&self) -> bool {
        self.is_player_in_check(self.active_player())
    }

    pub(super) fn k_in_row_at(&self, k: usize, sq: FairySquare, color: FairyColor) -> bool {
        let blockers = !self.player_bb(color);
        debug_assert!((blockers.raw() & RawFairyBitboard::single_piece_at(self.size().internal_key(sq))).is_zero());

        for dir in RayDirections::iter() {
            if (FairyBitboard::slider_attacks(sq, blockers, dir) & self.player_bb(color)).num_ones() >= k - 1 {
                return true;
            }
        }
        false
    }
}

impl UnverifiedFairyBoard {
    fn parse_castling_info(&self, castling_word: &str) -> Res<FairyCastleInfo> {
        let size = self.size;
        let mut info = FairyCastleInfo::new(size);

        if castling_word == "-" {
            return Ok(info);
        }

        for c in castling_word.chars() {
            ensure!(
                c.is_ascii_alphabetic(),
                "Unrecognized character '{0}' in castling descriptor '{1}'",
                c.to_string().red(),
                castling_word.red()
            );

            let color = if c.is_ascii_uppercase() { FairyColor::first() } else { FairyColor::second() };
            let king_bb = self.castling_bb_for(color);
            ensure!(
                king_bb.is_single_piece(),
                "Castling is only legal when there is a single royal piece, but the {0} player has {1}",
                self.rules().colors[color.idx()].name,
                king_bb.num_ones()
            );

            let lowercase_c = c.to_ascii_lowercase();
            let file = if (lowercase_c == 'k' || lowercase_c == 'q') && size.width.val() == 8 {
                if lowercase_c == 'k' {
                    7
                } else {
                    0
                }
            } else {
                char_to_file(lowercase_c)
            };
            let king_sq = king_bb.ones().next().unwrap();
            let side = if file > king_sq.file() { Kingside } else { Queenside };
            let king_dest_file = if side == Kingside { b'g' - b'a' } else { b'c' - b'a' };
            let rook_dest_file = if side == Kingside { king_dest_file - 1 } else { king_dest_file + 1 };
            let move_info = CastlingMoveInfo { rook_file: file, king_dest_file, rook_dest_file, fen_char: c as u8 };
            info.players[color.idx()].sides[side as usize] = Some(move_info);
        }
        Ok(info)
    }

    pub(super) fn read_castling_and_ep_fen_parts(mut self, words: &mut Tokens, _strictness: Strictness) -> Res<Self> {
        if self.rules().has_castling {
            let Some(castling_word) = words.next() else {
                bail!("FEN ends after color to move, missing castling rights")
            };
            self.castling_info = self.parse_castling_info(castling_word)?;
        }
        if self.rules().has_ep {
            let Some(ep_square) = words.next() else { bail!("FEN ends before en passant square") };
            self.ep = if ep_square == "-" { None } else { Some(FairySquare::from_str(ep_square)?) };
        }
        Ok(self)
    }
}
