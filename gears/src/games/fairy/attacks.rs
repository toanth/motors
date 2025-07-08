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

use crate::games::fairy::Side::{Kingside, Queenside};
use crate::games::fairy::attacks::AttackTypes::{Leaping, Rider};
use crate::games::fairy::attacks::GenAttackKind::Normal;
use crate::games::fairy::attacks::GenAttacksCondition::Always;
use crate::games::fairy::moves::FairyMove;
use crate::games::fairy::pieces::{ColoredPieceId, GenPromoMoves};
use crate::games::fairy::rules::SquareFilter::{EmptySquares, NotUs};
use crate::games::fairy::rules::{CheckCount, CheckingAttack, SquareFilter};
use crate::games::fairy::{
    CastlingMoveInfo, FairyBitboard, FairyBoard, FairyCastleInfo, FairyColor, FairyPiece, FairySize, FairySquare,
    RawFairyBitboard, Side, UnverifiedFairyBoard,
};
use crate::games::{AbstractPieceType, Color, ColoredPiece, ColoredPieceType, Coordinates, DimT, Size, char_to_file};
use crate::general::bitboards::{Bitboard, RawBitboard};
use crate::general::board::{BitboardBoard, Board, BoardHelpers, PieceTypeOf, Strictness, UnverifiedBoard};
use crate::general::common::{Res, Tokens};
use crate::general::hq::BitReverseSliderGenerator;
use crate::general::move_list::MoveList;
use crate::general::squares::{CompactSquare, RectangularCoordinates, RectangularSize};
use crate::{precompute_leaper_attacks, shift_left};
use anyhow::{anyhow, bail, ensure};
use arbitrary::Arbitrary;
use arrayvec::ArrayVec;
use colored::Colorize;
use std::str::FromStr;
use std::sync::Arc;

type SliderGen<'a> = BitReverseSliderGenerator<'a, FairySquare, FairyBitboard>;

/// The general organization of movegen is that of a pipeline, where a stage communicates with the next through enums,
/// which usually need the `rules()` to be interpreted correctly
#[derive(Debug, Clone, Arbitrary)]
pub enum SliderDirections {
    Forward,
    Vertical,
    Rook,
    Bishop,
    Queen,
    Rider { precomputed: Arc<[RawFairyBitboard]> },
}

// not `const`, which allows using ranges and for loops
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

#[derive(Debug, Clone, Arbitrary)]
/// Logically, this type should store a `Box`, but since some games like shogi have different pieces with the same attacks,
/// we deduplicate the attack bitboards as an optimization.
pub struct LeapingBitboards(Arc<[RawFairyBitboard]>);

fn leaper(n: usize, m: usize, rider: bool, size: FairySize) -> Arc<[RawFairyBitboard]> {
    let mut res = vec![RawFairyBitboard::default(); size.num_squares()];
    let (n, m) = (n.min(m), n.max(m));
    for (idx, elem) in res.iter_mut().enumerate() {
        let bb = precompute_leaper_attacks!(idx, n, m, rider, size.width.val(), u128);
        *elem = bb;
    }
    Arc::from(res)
}

impl LeapingBitboards {
    pub(super) fn fixed(n: usize, m: usize, size: FairySize) -> Self {
        Self(leaper(n, m, false, size))
    }

    pub(super) fn range_hv<Iter1: Iterator<Item = isize> + Clone, Iter2: Iterator<Item = isize> + Clone>(
        horizontal_range: Iter1,
        vertical_range: Iter2,
        size: FairySize,
    ) -> Self {
        let mut res = vec![RawFairyBitboard::default(); size.num_squares()];
        for (idx, elem) in res.iter_mut().enumerate() {
            let sq = size.idx_to_coordinates(idx as DimT);
            let bb = leaper_attack_range(horizontal_range.clone(), vertical_range.clone(), sq, size);
            *elem = bb;
        }
        LeapingBitboards(Arc::from(res))
    }

    pub(super) fn combine(mut self, other: LeapingBitboards) -> Self {
        assert_eq!(self.0.len(), other.0.len());
        Arc::make_mut(&mut self.0).iter_mut().zip(other.0.iter()).for_each(|(a, b)| *a |= b);
        self
    }

    pub(super) fn remove(mut self, other: LeapingBitboards) -> Self {
        assert_eq!(self.0.len(), other.0.len());
        Arc::make_mut(&mut self.0).iter_mut().zip(other.0.iter()).for_each(|(a, b)| *a &= !b);
        self
    }

    pub(super) fn flip(&self, size: FairySize) -> Self {
        let mut res = self.clone();
        let res_mut = Arc::make_mut(&mut res.0);
        for i in 0..size.num_squares() {
            let sq = size.idx_to_coordinates(i as DimT);
            let flipped_i = size.internal_key(sq.flip_up_down(size));
            res_mut[flipped_i] = FairyBitboard::new(self.0[i], size).flip_up_down().raw();
        }
        res
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
#[derive(Debug, Clone, Arbitrary)]
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
pub enum GenAttackKind {
    #[default]
    Normal,
    DoublePawnPush,
    Castle(Side),
    Drop,
}

impl GenAttackKind {
    pub fn to_move_kind(self, piece: ColoredPieceId) -> MoveKind {
        match self {
            Self::Normal => MoveKind::Normal,
            Self::DoublePawnPush => MoveKind::DoublePawnPush,
            Self::Castle(side) => MoveKind::Castle(side),
            GenAttackKind::Drop => MoveKind::Drop(piece.to_uncolored_idx() as u8),
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
#[derive(Debug, Clone, Arbitrary)]
pub struct AttackKind {
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
    pub bitboard_filter: Vec<SquareFilter>,
    // this is annotated with the move kind (e.g. castling)
    pub kind: GenAttackKind,
    // this is used to decide if a move is a capture
    pub capture_condition: CaptureCondition,
}

impl AttackKind {
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

    pub fn simple_side_relative(leaping: LeapingBitboards, size: FairySize) -> Vec<Self> {
        let flipped_leaper = leaping.flip(size);
        let leaping = Self {
            required: RequiredForAttack::PieceOnBoard,
            typ: Leaping(leaping),
            condition: GenAttacksCondition::Player(FairyColor::first()),
            bitboard_filter: vec![NotUs],
            kind: Normal,
            attack_mode: AttackMode::All,
            capture_condition: CaptureCondition::DestOccupied,
        };
        let mut flipped = leaping.clone();
        flipped.typ = Leaping(flipped_leaper);
        flipped.condition = GenAttacksCondition::Player(FairyColor::second());
        vec![leaping, flipped]
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
    pub fn pawn_capture(typ: AttackTypes, condition: GenAttacksCondition, bb_filter: SquareFilter) -> Self {
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
    pub fn drop(filter: Vec<SquareFilter>) -> Self {
        Self {
            required: RequiredForAttack::PieceInHand,
            condition: Always,
            attack_mode: AttackMode::NoCaptures,
            typ: AttackTypes::Drop,
            bitboard_filter: filter,
            kind: GenAttackKind::Drop,
            capture_condition: CaptureCondition::Never,
        }
    }

    pub fn bb_filter(&self, us: FairyColor, pos: &FairyBoard) -> RawFairyBitboard {
        let mut res = pos.mask_bb();
        for filter in &self.bitboard_filter {
            res &= filter.bb(us, pos);
        }
        res.raw()
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
                // let blockers = FairyBitboard::new(
                //     // TODO: Remove the &! after switching to `WithRev` impl
                //     blockers.raw() & !RawFairyBitboard::single_piece_at(size.internal_key(piece)),
                //     size,
                // );
                // TODO: Keep `gen` alive across calls by making it a parameter
                let generator = SliderGen::new(blockers, None);
                let res = match sliding {
                    SliderDirections::Forward => {
                        generator.forward_attacks(piece, !piece_id.color().unwrap_or_default().is_first())
                    }
                    SliderDirections::Vertical => generator.vertical_attacks(piece),
                    SliderDirections::Rook => generator.rook_attacks(piece),
                    SliderDirections::Bishop => generator.bishop_attacks(piece),
                    SliderDirections::Queen => generator.queen_attacks(piece),
                    SliderDirections::Rider { precomputed } => {
                        let ray = FairyBitboard::new(precomputed[size.internal_key(piece)], size);
                        // TODO: Also use gen, remove the fallback
                        FairyBitboard::hyperbola_quintessence_fallback(
                            size.internal_key(piece),
                            blockers,
                            FairyBitboard::flip_up_down,
                            ray,
                        )
                        // FairyBitboard::hyperbola_quintessence_non_horizontal(piece, blockers, ray)
                    }
                };
                res.raw()
            }
            &AttackTypes::Castling(side) => {
                if let Some(sq) = pos.0.castling_info.player(piece_id.color().unwrap()).king_dest_sq(side) {
                    FairyBitboard::single_piece_for(sq, size).raw()
                } else {
                    RawFairyBitboard::default()
                }
            }
            &AttackTypes::Drop => pos.mask_bb,
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
    pub kind: GenAttackKind,
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
        let from = piece.coordinates; // can be invalid in case of a drop
        for to in bb.ones() {
            let mut move_kinds = ArrayVec::new();
            MoveKind::insert(&mut move_kinds, self, from, to, pos);
            for kind in move_kinds {
                debug_assert_eq!(
                    matches!(kind, MoveKind::Drop(_)),
                    from == FairySquare::no_coordinates(),
                    "{pos} {kind:?} {from:?} {to:?} {bb:?}"
                );
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
pub enum Dir {
    North,
    South,
    East,
    West,
    Horizontal,
    Vertical,
    Diagonal,
    AntiDiagonal,
    Up(FairyColor),
    Down(FairyColor),
}

impl Dir {
    pub fn shift(self, bb: FairyBitboard) -> FairyBitboard {
        match self {
            Dir::North => bb.north(),
            Dir::South => bb.south(),
            Dir::East => bb.east(),
            Dir::West => bb.west(),
            Dir::Horizontal => bb.east() | bb.west(),
            Dir::Vertical => bb.north() | bb.south(),
            Dir::Diagonal => bb.north_east() | bb.south_west(),
            Dir::AntiDiagonal => bb.south_east() | bb.north_west(),
            Dir::Up(color) => {
                if color.is_first() {
                    bb.north()
                } else {
                    bb.south()
                }
            }
            Dir::Down(color) => {
                if color.is_first() {
                    bb.south()
                } else {
                    bb.north()
                }
            }
        }
    }
}

#[must_use]
#[derive(Debug, Clone, Arbitrary)]
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
    // special because it sets the ep square
    DoublePawnPush,
    // the given piece appears at the target square, like in m,n,k games or ataxx clones
    Drop(u8),
    // like Drop, but the piece on the source square disappears
    Promotion(u8),
    Castle(Side),
}

// this is also an upper bound of the number of pieces a pawn can promote to
pub const MAX_MOVE_KINDS_PER_ATTACK: usize = 32;

impl MoveKind {
    fn insert(
        list: &mut ArrayVec<MoveKind, MAX_MOVE_KINDS_PER_ATTACK>,
        attack: &PieceAttackBB,
        source: FairySquare,
        target: FairySquare,
        pos: &FairyBoard,
    ) {
        let promo = &pos.rules().pieces[attack.piece.uncolor().val()].promotions;
        let gen_promo = promo.gen_promo(source, target, pos);
        if gen_promo != GenPromoMoves::ForcedPromo {
            list.push(attack.kind.to_move_kind(attack.piece));
        }
        if gen_promo != GenPromoMoves::NoPromo {
            for &piece in &promo.pieces {
                let id = ColoredPieceId::new(pos.active_player(), piece);
                list.push(MoveKind::Promotion(id.as_u8()));
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
        FairyBitboard::new(RawFairyBitboard::default(), self.size())
    }

    pub(super) fn blocker_bb(&self) -> FairyBitboard {
        // TODO: Some games and piece types can modify this
        self.occupied_bb()
    }

    pub(super) fn piece_bb(&self, piece: PieceTypeOf<FairyBoard>) -> FairyBitboard {
        FairyBitboard::new(self.piece_bitboards[piece.val()], self.size())
    }

    pub(super) fn player_bb(&self, color: FairyColor) -> FairyBitboard {
        FairyBitboard::new(self.color_bitboards[color.idx()], self.size())
    }

    pub(super) fn neutral_bb(&self) -> FairyBitboard {
        FairyBitboard::new(self.neutral_bb, self.size())
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

    pub fn king_square(&self, color: FairyColor) -> Option<FairySquare> {
        self.royal_bb_for(color).to_square()
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
    /// Only includes capturing attacks, so no pawn pushes.
    /// All attack bitboards are based on pseudolegality, so they can't be used to determine if a move is legal,
    /// and (depending on the variant) also not easily for testing if a player is in check.
    /// This method is public mostly because it's often useful to have a rough approximation, e.g. for eval functions.
    pub fn capturing_attack_bb_of(&self, color: FairyColor) -> FairyBitboard {
        self.capturing_attack_bb_of_if(color, |_, _, _| true)
    }

    pub fn capturing_attack_bb_of_if<F: FnMut(FairyPiece, &PieceAttackBB, &FairyBoard) -> bool>(
        &self,
        color: FairyColor,
        mut cond: F,
    ) -> FairyBitboard {
        let mut res = RawFairyBitboard::default();
        let f = |piece: FairyPiece, bb: &PieceAttackBB, pos: &FairyBoard| {
            if cond(piece, bb, pos) {
                res |= bb.all_attacks
            }
        };
        self.gen_attacks_impl(f, color, AttackMode::Captures);
        FairyBitboard::new(res, self.size())
    }

    pub(super) fn gen_pseudolegal_impl<T: MoveList<Self>>(&self, moves: &mut T) {
        let f = |piece: FairyPiece, bb: &PieceAttackBB, pos: &FairyBoard| {
            bb.insert_moves(moves, pos, piece);
        };
        self.gen_attacks_impl(f, self.active_player(), AttackMode::All);
        self.rules().moves_filter.apply(moves, self);
    }

    fn gen_attacks_impl<F: FnMut(FairyPiece, &PieceAttackBB, &FairyBoard)>(
        &self,
        mut f: F,
        color: FairyColor,
        mode: AttackMode,
    ) {
        for (id, piece_type) in self.rules().pieces() {
            for attack_kind in &piece_type.attacks {
                match attack_kind.required {
                    RequiredForAttack::PieceOnBoard => {
                        let bb = self.col_piece_bb(color, id);
                        for start in bb.ones() {
                            let piece = FairyPiece { symbol: ColoredPieceId::new(color, id), coordinates: start };
                            if let Some(bb) = attack_kind.attacks(piece, self, mode) {
                                f(piece, &bb, self);
                            }
                        }
                    }
                    RequiredForAttack::PieceInHand => {
                        if self.0.in_hand[color][id.val()] > 0 {
                            let piece = FairyPiece {
                                symbol: ColoredPieceId::new(color, id),
                                coordinates: FairySquare::no_coordinates(),
                            };
                            if let Some(bb) = attack_kind.attacks(piece, self, mode) {
                                f(piece, &bb, self);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Returns a bitboard of all royal pieces that are in check.
    // For most games, a superpiece method could work, but that's an optimization for later.
    // For now, just computing all the attacks is simpler,
    // more robust, and good enough
    pub fn in_check_bb(&self, color: FairyColor) -> FairyBitboard {
        let them = color.other();
        let royals = self.royal_bb();
        let our_royals = royals & self.player_bb(color);
        let cond = self.rules().check_rules.attack_condition;
        let their_attacks = match cond {
            CheckingAttack::None => return self.zero_bitboard(),
            CheckingAttack::Capture => self.capturing_attack_bb_of(them),
            CheckingAttack::NoRoyalAdjacent => {
                let their_royals = royals & self.player_bb(them);
                if (their_royals & our_royals.moore_neighbors()).has_set_bit() {
                    return self.zero_bitboard();
                }
                self.capturing_attack_bb_of(them)
            }
        };
        our_royals & their_attacks
    }

    pub(super) fn compute_is_in_check(&self, color: FairyColor) -> bool {
        let rule = self.rules().check_rules;
        let in_check = self.in_check_bb(color);
        match rule.count {
            CheckCount::AllRoyals => in_check == self.royal_bb_for(color),
            CheckCount::AnyRoyal => in_check.has_set_bit(),
        }
    }

    pub fn is_player_in_check(&self, color: FairyColor) -> bool {
        self.in_check[color]
    }

    pub fn is_in_check(&self) -> bool {
        self.is_player_in_check(self.active_player())
    }

    // precondition: there must be a piece of `color` on `sq`
    pub(super) fn k_in_row_at(&self, k: usize, sq: FairySquare, color: FairyColor) -> bool {
        debug_assert!(self.player_bb(color).is_bit_set_at(self.size().internal_key(sq)));
        let blockers = !self.player_bb(color);
        debug_assert!((blockers.raw() & RawFairyBitboard::single_piece_at(self.size().internal_key(sq))).is_zero());

        let generator = SliderGen::new(blockers, None);

        (generator.horizontal_attacks(sq) & self.player_bb(color)).num_ones() >= k - 1
            || (generator.vertical_attacks(sq) & self.player_bb(color)).num_ones() >= k - 1
            || (generator.diagonal_attacks(sq) & self.player_bb(color)).num_ones() >= k - 1
            || (generator.anti_diagonal_attacks(sq) & self.player_bb(color)).num_ones() >= k - 1
    }
}

impl UnverifiedFairyBoard {
    fn find_x_fen_rook_file(&self, side: char, color: FairyColor, king_sq: FairySquare) -> Res<DimT> {
        if side == 'q' {
            for file in 0..king_sq.file() {
                let sq = FairySquare::from_rank_file(king_sq.rank(), file);
                let piece = self.piece_on(sq);
                // `contains` because e.g. 'rook (promoted)' should also match, and there aren't really any piece names that
                // "accidentally" contain 'rook'.
                if piece.color() == Some(color) && piece.uncolored().get(self.rules()).name.contains("rook") {
                    return Ok(file);
                }
            }
        } else {
            for file in ((king_sq.file() + 1)..self.size().width.get()).rev() {
                let sq = FairySquare::from_rank_file(king_sq.rank(), file);
                let piece = self.piece_on(sq);
                if piece.color() == Some(color) && piece.uncolored().get(self.rules()).name.contains("rook") {
                    return Ok(file);
                }
            }
        }
        let side = if side == 'q' { "queen" } else { "king" };
        bail!(
            "No rook found for {0} to castle {1}side: When using X-FEN castling rights (i.e., 'kqKQ' letters), \
            the rook piece must be named exactly 'rook'. Use the file letter instead to allow castling with other pieces",
            color.name(self.rules()).bold(),
            side.bold()
        )
    }

    fn parse_castling_info(&self, castling_word: &str) -> Res<FairyCastleInfo> {
        let mut info = FairyCastleInfo::new(self.size());

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
            let Some(king_sq) = king_bb.to_square() else {
                bail!(
                    "Castling is only legal when there is a single royal piece, but the {0} player has {1}",
                    self.rules().colors[color.idx()].name,
                    king_bb.num_ones()
                )
            };

            let lowercase_c = c.to_ascii_lowercase();
            // X-FEN requires finding a rook, which we test for by literally searching for "rook" in the piece name.
            // For Shredder FEN, we instead use the given square, which enables castling with other pieces.
            let file = if lowercase_c == 'k' || lowercase_c == 'q' {
                self.find_x_fen_rook_file(lowercase_c, color, king_sq)?
            } else {
                char_to_file(lowercase_c)
            };
            let side = if file > king_sq.file() { Kingside } else { Queenside };
            let king_dest_file = if side == Kingside { b'g' - b'a' } else { b'c' - b'a' };
            let rook_dest_file = if side == Kingside { king_dest_file - 1 } else { king_dest_file + 1 };
            let move_info = CastlingMoveInfo { rook_file: file, king_dest_file, rook_dest_file, fen_char: c as u8 };
            let entry = &mut info.players[color.idx()].sides[side as usize];
            ensure!(
                entry.is_none(),
                "Attempting to set the same castle right twice for player {0} and file '{1}' ({side})",
                color.name(self.settings()),
                b'a' + file
            );
            info.players[color.idx()].sides[side as usize] = Some(move_info);
        }
        Ok(info)
    }

    pub(super) fn read_castling_and_ep_fen_parts(&mut self, words: &mut Tokens, _strictness: Strictness) -> Res<()> {
        if self.rules().has_castling {
            let Some(castling_word) = words.next() else {
                bail!("FEN ends after color to move, missing castling rights")
            };
            self.castling_info = self.parse_castling_info(castling_word)?;
        }
        if self.rules().has_ep {
            let Some(ep_square) = words.next() else { bail!("FEN ends before en passant square") };
            self.ep = if ep_square == "-" {
                None
            } else {
                let ep = FairySquare::from_str(ep_square)
                    .map_err(|err| anyhow!("Failed to read the ep square ('{}'): {err}", ep_square.red()))?;
                ensure!(self.is_empty(ep), "The en passant square ('{ep}') must be empty");
                Some(ep)
            };
        }
        Ok(())
    }
}
