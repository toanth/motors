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
use crate::games::fairy::attacks::AttackBitboardGen::{Leaping, Rider};
use crate::games::fairy::attacks::GenAttacksCondition::Always;
use crate::games::fairy::moves::Move;
use crate::games::fairy::piece_builder::Topology::Cylinder;
use crate::games::fairy::piece_builder::{RayDescription, RayDir, Topology};
use crate::games::fairy::pieces::{ColoredPieceId, GenPromoMoves};
use crate::games::fairy::rules::{CheckCount, CheckingAttack, SquareFilter};
use crate::games::fairy::{
    Bitboard, Board, CastlingMoveInfo, Color, FairyCastleInfo, Piece, RawBitboard, Side, Size, Square, UnverifiedBoard,
};
use crate::games::{
    AbstractPieceType, ColorTrait, ColoredPieceTrait, ColoredPieceTypeTrait, DimT, NUM_COLORS, SizeTrait, char_to_file,
};
use crate::general::bitboards::{BitboardTrait, ExtendedRawBitboard, RawBitboardTrait};
use crate::general::board::{BitboardBoard, BoardHelpers, BoardTrait, PieceTypeOf, Strictness, UnverifiedBoardTrait};
use crate::general::common::{Res, Tokens};
use crate::general::hq::{BitReverseSliderGenerator, hq_right_horizontal_cylinder};
use crate::general::move_list::MoveListTrait;
use crate::general::squares::{CompactSquare, RectangularCoordinates, RectangularSize};
use crate::{do_shift, shift_left};
use anyhow::{anyhow, bail, ensure};
use arbitrary::Arbitrary;
use arrayvec::ArrayVec;
use colored::Colorize;
use std::cmp::max;
use std::num::NonZero;
use std::str::FromStr;
use std::sync::Arc;

pub(super) type SliderGen<'a> = BitReverseSliderGenerator<'a, Square, Bitboard>;

// TODO: replace with `RayDescription`?
#[derive(Debug, Clone, Arbitrary)]
pub enum SliderDirections {
    // TODO: Add Backward and make it not depend on the color, instead about increasing / decreasing the rank
    // Or maybe remove altogether and use `Rider` instead
    Forward,
    Vertical,
    Rook,
    Bishop,
    Queen,
    // A ray bitboard for each square
    RiderRay { rays: Arc<[RawBitboard]> },
}

fn precompute_rider_attacks(sq_idx: usize, ray: RayDescription) -> RawBitboard {
    let width = ray.size.width.val() as isize;
    let max_steps = ray.limit_steps.unwrap_or(1000).min(max(width as usize, ray.size.height.val()));
    let this_piece = 1 << sq_idx;
    let file = sq_idx as isize % width;
    let mut attacks = 0;
    let mut dx = ray.dir.dx;
    let mut dy = ray.dir.dy;
    for _ in 0..max_steps {
        attacks |= do_shift!(dx, dy, width, file, this_piece, ray.topology == Cylinder, RawBitboard);
        if ray.with_reverse {
            attacks |= do_shift!(-dx, -dy, width, file, this_piece, ray.topology == Cylinder, RawBitboard);
        }

        dx += ray.dir.dx;
        dy += ray.dir.dy;
    }
    attacks
}

pub fn rider(ray: RayDescription) -> Arc<[RawBitboard]> {
    let mut res = vec![RawBitboard::default(); ray.size.num_squares()];
    for (idx, elem) in res.iter_mut().enumerate() {
        let bb = precompute_rider_attacks(idx, ray);
        debug_assert!(!bb.is_bit_set_at(idx));
        *elem = bb;
    }
    Arc::from(res)
}

fn precompute_single_leaper_attacks(offsets: &[RayDir], square: Square, size: Size, topology: Topology) -> RawBitboard {
    let mut res = RawBitboard::default();
    let width = size.width().val() as isize;
    let internal_width = size.internal_width() as isize;
    for RayDir { dx, dy } in offsets {
        let res_x = square.file() as isize + dx;
        let shift = if res_x >= 0 && res_x < width {
            dx + dy * internal_width
        } else if topology == Cylinder {
            let res_x = res_x.rem_euclid(width);
            (res_x - square.file() as isize) + dy * internal_width
        } else {
            continue;
        };
        let bb = Bitboard::single_piece_for(square, size);
        res |= shift_left!(bb.raw(), shift);
    }
    res
}

// TODO: Move the following to fairy piece_builder, clean up
#[derive(Debug, Clone, Arbitrary)]
/// Since some games like shogi have different pieces with the same attacks,
/// we deduplicate the attack bitboards as an optimization (TODO: Add a test that this actually works, by comparing pointers).
/// Also, this makes it cheap to clone, which allows us to store a different instance per player, which
/// reduces branches during movegen.
pub struct LeapingBitboards(Arc<[RawBitboard]>);

impl LeapingBitboards {
    pub(super) fn new(offsets: &[RayDir], size: Size, topology: Topology) -> Self {
        let mut res = vec![RawBitboard::default(); size.num_squares()];
        for (idx, elem) in res.iter_mut().enumerate() {
            let sq = size.idx_to_coordinates(idx as DimT);
            let bb = precompute_single_leaper_attacks(offsets, sq, size, topology);
            *elem = bb;
        }
        LeapingBitboards(Arc::from(res))
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub enum Modality {
    #[default]
    Both = 0b11,
    Capture = 0b10,
    NonCapture = 0b01,
}

impl Modality {
    pub fn betza_char(&self) -> char {
        match self {
            Modality::Both => ' ',
            Modality::Capture => 'c',
            Modality::NonCapture => 'm',
        }
    }

    pub fn should_generate_for(self, other: Self) -> bool {
        self as usize & other as usize != 0
    }
}

#[must_use]
#[derive(Debug, Clone, Arbitrary)]
pub enum AttackBitboardGen {
    /// Leaping pieces, like knights and kings, only care about blockers on the square they're leaping to.
    /// Implemented by precomputing a bitboard for every square
    Leaping(LeapingBitboards),
    /// Riders are generalized sliders. For every square, there is a ray bitboard which contains the squares this
    /// rider could move to on an empty board.
    Rider(SliderDirections),
    /// Dropping a piece, such as in m,n,k games or shogi.
    Drop,
    /// Castling moves are special enough that it makes sense to handle them separately.
    Castling(Side),
    /// Horizontal attacks on a cylinder, including riders that jump over squares, can't be handled the usual rider way with hq
    /// because square indices along a ray aren't monotonic.
    HorizontalCylinder { step_left: Option<NonZero<usize>>, step_right: Option<NonZero<usize>> },
}

impl AttackBitboardGen {
    pub fn rider(rays: Arc<[RawBitboard]>) -> Self {
        Rider(SliderDirections::RiderRay { rays })
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub enum RequiredForAttack {
    PieceOnBoard,
    // This is mostly the same as piece drop attacks, although there can be drops that also require a piece on the board,
    // such as ataxx cloning moves.
    PieceInHand,
}

/// Attacks are about bitboards for performance reasons, but there are also moves that aren't fully represented with bitboards.
/// This struct is only about attacks, so there are move types that are generated separately.
/// An instance of this struct represents "similar" attacks of a given piece, for example normal king moves without castling,
/// or all queen moves.
#[must_use]
#[derive(Debug, Clone, Arbitrary)]
pub struct AttackKind {
    // first, we distinguish between moving attacks (e.g. chess moves) and drops (e.g. mnk games)
    pub required: RequiredForAttack,
    // then, the condition is checked (e.g., pawns can only double push on their start rank)
    pub condition: GenAttacksCondition,
    // and the attack kind may also be disabled based on the modality, for example pawn pushes don't capture
    pub modality: Modality,
    // then, the bitboard of attacks is generated by taking the `bitor` of different attack generation primitives,
    // such as moves along a ray, or static leaper bitboards
    pub bb_gen: Vec<[AttackBitboardGen; NUM_COLORS]>,
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
    pub fn bb_filter(&self, us: Color, pos: &Board) -> RawBitboard {
        let mut res = pos.mask_bb();
        for filter in &self.bitboard_filter {
            res &= filter.bb(us, pos);
        }
        res.raw()
    }

    pub fn attacks_impl(
        &self,
        piece: Piece,
        filter_bb: RawBitboard,
        squares_mask: RawBitboard,
        pos: &Board,
        generator: &SliderGen,
    ) -> PieceAttackBB {
        let piece_id = piece.symbol;
        let sq = piece.coordinates;
        let size = pos.size();
        let mut res = 0;
        for bb_gen in &self.bb_gen {
            res |= match &bb_gen[piece_id.color().unwrap_or_default()] {
                Leaping(precomputed) => precomputed.0[size.internal_key(sq)],
                Rider(sliding) => {
                    let res = match sliding {
                        SliderDirections::Forward => {
                            generator.forward_attacks(sq, !piece_id.color().unwrap_or_default().is_first())
                        }
                        SliderDirections::Vertical => generator.vertical_attacks(sq),
                        SliderDirections::Rook => generator.rook_attacks(sq),
                        SliderDirections::Bishop => generator.bishop_attacks(sq),
                        SliderDirections::Queen => generator.queen_attacks(sq),
                        SliderDirections::RiderRay { rays } => {
                            let ray = Bitboard::new(rays[size.internal_key(sq)], size);
                            // TODO: Also use gen, remove the fallback
                            Bitboard::hyperbola_quintessence_fallback(
                                size.internal_key(sq),
                                pos.blocker_bb(),
                                Bitboard::reverse,
                                ray,
                            )
                        }
                    };
                    res.raw()
                }
                &AttackBitboardGen::Castling(side) => {
                    if let Some(sq) = pos.0.castling_info.player(piece_id.color().unwrap()).king_dest_sq(side) {
                        Bitboard::single_piece_for(sq, size).raw()
                    } else {
                        RawBitboard::default()
                    }
                }
                &AttackBitboardGen::Drop => pos.mask_bb,
                &AttackBitboardGen::HorizontalCylinder { step_left, step_right } => {
                    let file = sq.file() as usize;
                    let rank = sq.rank() as usize;
                    let w = size.width.val();
                    let all_blockers = pos.blocker_bb().raw();
                    let blockers = all_blockers >> (rank * w);
                    let mut res = 0;
                    if let Some(step) = step_right {
                        res = hq_right_horizontal_cylinder(file, w, step.get(), blockers);
                    }
                    if let Some(step) = step_left {
                        let rev_blockers = blockers.reverse_bits() >> (128 - w);
                        let left = hq_right_horizontal_cylinder(w - 1 - file, w, step.get(), rev_blockers)
                            .reverse_bits()
                            >> (128 - w);
                        res |= left;
                    }
                    debug_assert_eq!(res & !0 << w, 0);
                    res << (rank * w)
                }
            };
        }
        PieceAttackBB {
            all_attacks: res & squares_mask,
            kind: self.kind,
            piece: piece_id,
            filter_bb,
            capture_condition: self.capture_condition,
        }
    }

    fn check_conditions(&self, piece: Piece, pos: &Board, modality: Modality) -> bool {
        if !self.modality.should_generate_for(modality) {
            return false;
        }
        match self.condition {
            Always => true,
            GenAttacksCondition::Player(color) => piece.color().is_some_and(|c| c == color),
            GenAttacksCondition::CanCastle(side) => pos.0.castling_info.can_castle(pos.active_player(), side),
            GenAttacksCondition::OnRelativeRank(mut rank, color) => {
                if !color.is_first() {
                    rank = pos.size().height.get() - 1 - rank;
                }
                piece.color().is_some_and(|c| c == color) && piece.coordinates.rank() == rank
            }
        }
    }

    pub fn attacks(
        &self,
        piece: Piece,
        pos: &Board,
        modality: Modality,
        generator: &SliderGen,
    ) -> Option<PieceAttackBB> {
        if !self.check_conditions(piece, pos, modality) {
            return None;
        }
        let filter = self.bb_filter(piece.color().unwrap(), pos);
        Some(self.attacks_impl(piece, filter, pos.0.mask_bb, pos, generator))
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
    pub all_attacks: RawBitboard,
    pub filter_bb: RawBitboard,
    pub kind: GenAttackKind,
    pub piece: ColoredPieceId,
    pub capture_condition: CaptureCondition,
}

impl PieceAttackBB {
    pub fn bb(&self) -> RawBitboard {
        self.all_attacks & self.filter_bb
    }

    fn is_capture(&self, to: Square, pos: &Board) -> bool {
        match self.capture_condition {
            CaptureCondition::DestOccupied => pos.is_occupied(to),
            CaptureCondition::Always => true,
            CaptureCondition::Never => false,
        }
    }

    pub fn insert_moves<L: MoveListTrait<Board>>(&self, list: &mut L, pos: &Board, piece: Piece) {
        let bb = Bitboard::new(self.bb(), pos.size());
        let from = piece.coordinates; // can be invalid in case of a drop
        for to in bb.ones() {
            let mut move_kinds = ArrayVec::new();
            MoveKind::insert(&mut move_kinds, self, from, to, pos);
            for kind in move_kinds {
                debug_assert_eq!(
                    matches!(kind, MoveKind::Drop(_)),
                    from == Square::no_coordinates(),
                    "{pos} {kind:?} {from:?} {to:?} {bb:?}"
                );
                let is_capture = self.is_capture(to, pos);
                let mov = Move {
                    from: CompactSquare::new(from, pos.size()),
                    to: CompactSquare::new(to, pos.size()),
                    packed: Move::pack(kind, is_capture),
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
    Up(Color),
    Down(Color),
}

impl Dir {
    pub fn shift(self, bb: Bitboard) -> Bitboard {
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
#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
pub enum GenAttacksCondition {
    Always,
    Player(Color), // TODO: Remove? Only makes sense for asymmetric games anyway
    CanCastle(Side),
    OnRelativeRank(DimT, Color), // TODO: Use Bitboard instead?
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
        source: Square,
        target: Square,
        pos: &Board,
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
    // pub explosion_radius: usize, // TODO: Remove?
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

impl UnverifiedBoard {
    pub(super) fn zero_bitboard(&self) -> Bitboard {
        Bitboard::new(RawBitboard::default(), self.size())
    }

    pub(super) fn blocker_bb(&self) -> Bitboard {
        // TODO: Some games and piece types can modify this
        self.occupied_bb()
    }

    pub(super) fn piece_bb(&self, piece: PieceTypeOf<Board>) -> Bitboard {
        Bitboard::new(self.piece_bitboards[piece.val()], self.size())
    }

    pub(super) fn player_bb(&self, color: Color) -> Bitboard {
        Bitboard::new(self.color_bitboards[color], self.size())
    }

    pub(super) fn neutral_bb(&self) -> Bitboard {
        Bitboard::new(self.neutral_bb, self.size())
    }

    pub(super) fn mask_bb(&self) -> Bitboard {
        Bitboard::new(self.mask_bb, self.size())
    }

    pub fn royal_bb(&self) -> Bitboard {
        let mut res = self.zero_bitboard();
        for piece in self.rules().royals() {
            res |= self.piece_bb(piece)
        }
        res
    }

    pub fn royal_bb_for(&self, color: Color) -> Bitboard {
        self.royal_bb() & self.player_bb(color)
    }

    pub fn king_square(&self, color: Color) -> Option<Square> {
        self.royal_bb_for(color).to_square()
    }

    /// In normal chess, this is the king bitboard, but not the rook bitboard
    pub fn castling_bb(&self) -> Bitboard {
        let mut res = self.zero_bitboard();
        for piece in self.rules().castling() {
            res |= self.piece_bb(piece)
        }
        res
    }
    pub fn castling_bb_for(&self, color: Color) -> Bitboard {
        self.castling_bb() & self.player_bb(color)
    }
}

impl Board {
    /// Only includes capturing attacks, so no pawn pushes.
    /// All attack bitboards are based on pseudolegality, so they can't be used to determine if a move is legal,
    /// and (depending on the variant) also not easily for testing if a player is in check.
    /// This method is public mostly because it's often useful to have a rough approximation, e.g. for eval functions.
    pub fn capturing_attack_bb_of(&self, color: Color) -> Bitboard {
        self.capturing_attack_bb_of_if(color, |_, _, _| true)
    }

    pub fn capturing_attack_bb_of_if<F: FnMut(Piece, &PieceAttackBB, &Board) -> bool>(
        &self,
        color: Color,
        mut cond: F,
    ) -> Bitboard {
        let mut res = RawBitboard::default();
        let f = |piece: Piece, bb: &PieceAttackBB, pos: &Board| {
            if cond(piece, bb, pos) {
                res |= bb.all_attacks
            }
        };
        self.gen_attacks_impl(f, color, Modality::Capture);
        Bitboard::new(res, self.size())
    }

    pub(super) fn gen_pseudolegal_impl<T: MoveListTrait<Self>>(&self, moves: &mut T) {
        let f = |piece: Piece, bb: &PieceAttackBB, pos: &Board| {
            bb.insert_moves(moves, pos, piece);
        };
        self.gen_attacks_impl(f, self.active_player(), Modality::Both);
        self.rules().moves_filter.apply(moves, self);
    }

    fn gen_attacks_impl<F: FnMut(Piece, &PieceAttackBB, &Board)>(&self, mut f: F, color: Color, modality: Modality) {
        // TODO: Precomputed rays
        let generator = SliderGen::new(self.blocker_bb(), None);
        for (id, piece_type) in self.rules().pieces() {
            for attack_kind in &piece_type.attacks {
                match attack_kind.required {
                    RequiredForAttack::PieceOnBoard => {
                        let bb = self.col_piece_bb(color, id);
                        for start in bb.ones() {
                            let piece = Piece { symbol: ColoredPieceId::new(color, id), coordinates: start };
                            if let Some(bb) = attack_kind.attacks(piece, self, modality, &generator) {
                                f(piece, &bb, self);
                            }
                        }
                    }
                    RequiredForAttack::PieceInHand => {
                        if self.0.in_hand[color][id.val()] > 0 {
                            let piece =
                                Piece { symbol: ColoredPieceId::new(color, id), coordinates: Square::no_coordinates() };
                            if let Some(bb) = attack_kind.attacks(piece, self, modality, &generator) {
                                f(piece, &bb, self);
                            }
                        }
                    }
                }
            }
        }
    }

    pub(super) fn attack_of_impl(&self, sq: Square) -> ExtendedRawBitboard {
        let p = self.piece_type_on(sq);
        let Some(piece) = p.get(self.rules()) else {
            return 0;
        };
        let mut res = 0;
        for attack_kind in &piece.attacks {
            let Some(bb) = attack_kind.attacks(
                self.piece_on(sq),
                self,
                Modality::Capture,
                &SliderGen::new(self.blocker_bb(), None),
            ) else {
                continue;
            };
            res |= bb.bb();
        }
        res
    }

    /// Returns a bitboard of all royal pieces that are in check.
    // For most games, a superpiece method could work, but that's an optimization for later.
    // For now, just computing all the attacks is simpler,
    // more robust, and good enough
    pub fn in_check_bb(&self, color: Color) -> Bitboard {
        let them = color.other();
        let royals = self.royal_bb();
        let our_royals = royals & self.player_bb(color);
        let cond = self.rules().check_rules.attack_condition;
        let their_attacks = match cond {
            CheckingAttack::None => return self.zero_bitboard(),
            CheckingAttack::Capture => self.capturing_attack_bb_of(them),
            CheckingAttack::NoRoyalAdjacent => {
                let their_royals = royals & self.player_bb(them);
                if (their_royals & our_royals.moore_inclusive()).has_any() {
                    return self.zero_bitboard();
                }
                self.capturing_attack_bb_of(them)
            }
        };
        our_royals & their_attacks
    }

    pub(super) fn compute_is_in_check(&self, color: Color) -> bool {
        let rule = self.rules().check_rules;
        let in_check = self.in_check_bb(color);
        match rule.count {
            CheckCount::AllRoyals => in_check == self.royal_bb_for(color),
            CheckCount::AnyRoyal => in_check.has_any(),
        }
    }

    pub fn is_player_in_check(&self, color: Color) -> bool {
        self.in_check[color]
    }

    pub fn is_in_check(&self) -> bool {
        self.is_player_in_check(self.active_player())
    }

    pub fn gives_check_slow(&self, mov: Move) -> bool {
        debug_assert!(self.is_move_pseudolegal(mov));
        self.clone().make_move(mov).is_some_and(|new_pos| new_pos.is_in_check())
    }

    // precondition: there must be a piece of `color` on `sq`
    pub(super) fn k_in_row_at(&self, k: usize, sq: Square, color: Color) -> bool {
        debug_assert!(self.player_bb(color).is_bit_set_at(self.size().internal_key(sq)));
        let blockers = !self.player_bb(color);
        debug_assert!((blockers.raw() & RawBitboard::single_piece_at(self.size().internal_key(sq))).is_zero());

        let generator = SliderGen::new(blockers, None);

        (generator.horizontal_attacks(sq) & self.player_bb(color)).num_ones() >= k - 1
            || (generator.vertical_attacks(sq) & self.player_bb(color)).num_ones() >= k - 1
            || (generator.diagonal_attacks(sq) & self.player_bb(color)).num_ones() >= k - 1
            || (generator.anti_diagonal_attacks(sq) & self.player_bb(color)).num_ones() >= k - 1
    }
}

impl UnverifiedBoard {
    fn find_x_fen_rook_file(&self, side: char, color: Color, king_sq: Square) -> Res<DimT> {
        let has_rook = |file: DimT| {
            let sq = Square::from_rank_file(king_sq.rank(), file);
            let piece = self.piece_on(sq);
            // `contains` because e.g. 'rook (promoted)' should also match, and there aren't really any piece names that
            // "accidentally" contain 'rook'.
            piece.color() == Some(color) && piece.uncolored().get(self.rules()).unwrap().name.contains("rook")
        };
        if side == 'q' {
            for file in 0..king_sq.file() {
                if has_rook(file) {
                    return Ok(file);
                }
            }
        } else {
            for file in ((king_sq.file() + 1)..self.size().width.get()).rev() {
                if has_rook(file) {
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

            let color = if c.is_ascii_uppercase() { Color::first() } else { Color::second() };
            let king_bb = self.castling_bb_for(color);
            let Some(king_sq) = king_bb.to_square() else {
                bail!(
                    "Castling is only legal when there is a single royal piece, but the {0} player has {1}",
                    self.rules().colors[color].name,
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
            let entry = &mut info.players[color].sides[side as usize];
            ensure!(
                entry.is_none(),
                "Attempting to set the same castle right twice for player {0} and file '{1}' ({side})",
                color.name(self.settings()),
                b'a' + file
            );
            info.players[color].sides[side as usize] = Some(move_info);
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
                let ep = Square::from_str(ep_square)
                    .map_err(|err| anyhow!("Failed to read the ep square ('{}'): {err}", ep_square.red()))?;
                ensure!(self.is_empty(ep), "The en passant square ('{ep}') must be empty");
                Some(ep)
            };
        } else if words.peek().copied() == Some("-") {
            _ = words.next(); // Some GUIs always send castling and ep as '-' even if the variant doesn't support them
            if words.peek().copied() == Some("-") && !self.rules().has_castling {
                _ = words.next();
            }
        }
        Ok(())
    }
}
