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
use crate::games::CharType::{Ascii, Unicode};
use crate::games::chess::pieces::{
    UNICODE_BLACK_BISHOP, UNICODE_BLACK_KING, UNICODE_BLACK_KNIGHT, UNICODE_BLACK_PAWN, UNICODE_BLACK_QUEEN,
    UNICODE_BLACK_ROOK, UNICODE_NEUTRAL_BISHOP, UNICODE_NEUTRAL_KING, UNICODE_NEUTRAL_KNIGHT, UNICODE_NEUTRAL_PAWN,
    UNICODE_NEUTRAL_QUEEN, UNICODE_NEUTRAL_ROOK, UNICODE_WHITE_BISHOP, UNICODE_WHITE_KING, UNICODE_WHITE_KNIGHT,
    UNICODE_WHITE_PAWN, UNICODE_WHITE_QUEEN, UNICODE_WHITE_ROOK,
};
use crate::games::fairy::Side::*;
use crate::games::fairy::attacks::GenAttackKind::*;
use crate::games::fairy::attacks::GenAttacksCondition::*;
use crate::games::fairy::attacks::SliderDirections::{Bishop, Queen, Rook};
use crate::games::fairy::attacks::{
    AttackBitboardGen, AttackKind, CaptureCondition, Dir, GenAttackKind, GenAttacksCondition, LeapingBitboards,
    Modality, MoveKind, RequiredForAttack, rider,
};
use crate::games::fairy::config::n_m_to_ray_dirs;
use crate::games::fairy::piece_builder::AttackBBGenBuilder::{Leaper, PlaneBishop, PlaneQueen, PlaneRook, Rider};
use crate::games::fairy::piece_builder::Topology::*;
use crate::games::fairy::pieces::{DrawCtrReset, Piece, PieceId, Promo, PromoCondition};
use crate::games::fairy::rules::PlayerCond::Inactive;
use crate::games::fairy::rules::SquareFilter::{EmptySquares, InDirectionOf, NoSquares, Not, NotUs, RanksRelative};
use crate::games::fairy::rules::{PieceCond, PlayerCond, SquareFilter};
use crate::games::fairy::{Color, RawBitboard, Side, Size};
use crate::games::{ColorTrait, NUM_CHAR_TYPES, NUM_COLORS};
use crate::general::squares::RectangularSize;
use arbitrary::Arbitrary;
use itertools::Itertools;
use std::cmp::max;
use std::collections::HashMap;
use std::num::NonZero;
use std::sync::Arc;

const UNICODE_X: char = '⨉'; // '⨉',
const UNICODE_O: char = '◯'; // '○'

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub enum Topology {
    #[default]
    Plane,
    Cylinder,
    // TODO: Bouncing (subset of 4 board edges)
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Arbitrary)]
pub struct RayDir {
    pub dx: isize,
    pub dy: isize,
}

impl RayDir {
    // Invert the y direction of the ray, i.e. change the perspective to the other player
    pub fn on_flipped(mut self) -> Self {
        self.dy *= -1;
        self
    }
    // The opposite direction of the ray
    pub fn inverse(mut self) -> Self {
        self.dx *= -1;
        self.dy *= -1;
        self
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct RayDescription {
    pub dir: RayDir,
    pub with_reverse: bool,
    pub limit_steps: Option<usize>,
    pub size: Size,
    pub topology: Topology,
}

impl RayDescription {
    pub fn new(
        mut dir: RayDir,
        with_reverse: bool,
        mut limit_steps: Option<usize>,
        size: Size,
        topology: Topology,
    ) -> Self {
        if with_reverse && (dir.dx, dir.dy) < (0, 0) {
            dir.dx *= -1;
            dir.dy *= -1;
        }
        dir.dx %= size.width.val() as isize;
        if let Some(limit) = limit_steps
            && topology == Plane
        {
            if limit >= max(size.width().val(), size.height().val()) {
                limit_steps = None;
            }
        }
        Self { dir, with_reverse, limit_steps, topology, size }
    }
}

// Often, a lot of attack data is shared between different pieces in the same game. For example, there might be several
// pieces that can move along the same ray. This struct ensures this shared information is only computed and stored once
#[derive(Debug, Default)]
pub(super) struct PieceBuilderCache {
    pub(super) ray_cache: HashMap<RayDescription, Arc<[RawBitboard]>>,
    pub(super) leaper_cache: HashMap<LeaperBBBuilder, LeapingBitboards>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub struct LeaperBBBuilder {
    pub offsets: Vec<RayDir>,
    pub topology: Topology,
    pub modality: Modality,
}

impl LeaperBBBuilder {
    fn simple_n_m(n: usize, m: usize) -> Self {
        let offsets = n_m_to_ray_dirs(n, m);
        LeaperBBBuilder { offsets, topology: Default::default(), modality: Default::default() }
    }
    fn simple(dirs: Vec<RayDir>) -> Self {
        LeaperBBBuilder { offsets: dirs, topology: Default::default(), modality: Default::default() }
    }
    fn radius_up_to(n: isize) -> Self {
        let offsets =
            (-n..=n).cartesian_product(-n..=n).filter(|&x| x != (0, 0)).map(|(dx, dy)| RayDir { dx, dy }).collect_vec();
        LeaperBBBuilder { offsets, topology: Default::default(), modality: Default::default() }
    }
    fn radius_exact(n: isize) -> Self {
        let offsets = (-n..=n)
            .cartesian_product([-n, n])
            .chain([-n, n].into_iter().cartesian_product(-n + 1..n))
            .map(|(dx, dy)| RayDir { dx, dy })
            .collect_vec();
        LeaperBBBuilder { offsets, topology: Default::default(), modality: Default::default() }
    }

    fn build_impl(&self, size: Size, cache: &mut PieceBuilderCache) -> LeapingBitboards {
        if let Some(res) = cache.leaper_cache.get(self) {
            return res.clone();
        }
        let res = LeapingBitboards::new(&self.offsets, size, self.topology);
        _ = cache.leaper_cache.insert(self.clone(), res.clone());
        res
    }

    fn build(
        &self,
        size: Size,
        col_relative: bool,
        cache: &mut PieceBuilderCache,
    ) -> Vec<[AttackBitboardGen; NUM_COLORS]> {
        let first_player = AttackBitboardGen::Leaping(self.build_impl(size, cache));
        let is_symmetrical = self.offsets.iter().all(|r| self.offsets.contains(&r.on_flipped()));
        let second_player = if !col_relative || is_symmetrical {
            first_player.clone()
        } else {
            let flipped_offsets = self.offsets.iter().map(|r| r.on_flipped()).collect_vec();
            let flipped = Self { offsets: flipped_offsets, ..self.clone() };
            AttackBitboardGen::Leaping(flipped.build_impl(size, cache))
        };
        vec![[first_player, second_player]]
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Arbitrary)]
pub struct RayBBBuilder {
    pub ray_steps: Vec<RayDir>,
    pub limit: Option<usize>,
    pub topology: Topology,
    pub modality: Modality,
}

impl RayBBBuilder {
    fn simple_n_m(n: usize, m: usize) -> Self {
        let ray_steps = n_m_to_ray_dirs(n, m);
        Self { ray_steps, limit: None, topology: Default::default(), modality: Default::default() }
    }
    fn simple(ray_steps: Vec<RayDir>) -> Self {
        Self { ray_steps, limit: None, topology: Default::default(), modality: Default::default() }
    }

    fn build(
        &self,
        size: Size,
        col_relative: bool,
        cache: &mut PieceBuilderCache,
    ) -> Vec<[AttackBitboardGen; NUM_COLORS]> {
        // TODO: Make direction be not color relative, so instead of `.clone()` we need to flip here
        let mut rays = vec![];
        for &r in &self.ray_steps {
            assert!(!matches!(r, RayDir { dx: 0, dy: 0 })); // TODO: Testcase
            if rays.iter().any(|d: &RayDescription| d.dir == r.inverse()) {
                continue;
            }
            let with_reverse = self.ray_steps.contains(&r.inverse());
            rays.push(RayDescription::new(r, with_reverse, self.limit, size, self.topology));
        }
        let mut res = vec![];
        for r in rays {
            if r.dir.dy == 0 && r.topology == Cylinder {
                let dx = NonZero::new(r.dir.dx.abs() as usize).unwrap();
                let a = if r.with_reverse {
                    AttackBitboardGen::HorizontalCylinder { step_left: Some(dx), step_right: Some(dx) }
                } else if r.dir.dx > 0 {
                    AttackBitboardGen::HorizontalCylinder { step_left: None, step_right: Some(dx) }
                } else {
                    AttackBitboardGen::HorizontalCylinder { step_left: Some(dx), step_right: None }
                };
                res.push([a.clone(), a])
            } else {
                res.push(build_rider_pair(r, col_relative, cache));
            }
        }
        res
    }
}

/// A struct that holds all the information necessary to, given a size, build an instance of `AttackBBGen`, which is
/// then used as the core part of movegen, to calculate attack bitboards.
#[derive(Debug, Clone, Eq, PartialEq, Arbitrary)]
pub enum AttackBBGenBuilder {
    Leaper(LeaperBBBuilder),
    Rider(RayBBBuilder),
    // special cased because there is a faster implementation for these very common cases.
    // They don't only apply to the 3 chess pieces, but all pieces that attack in these directions on a plane board
    PlaneBishop,
    PlaneRook,
    PlaneQueen,
    Castle(Side), // TODO: More info, orient on betza
    Drop,
}

impl AttackBBGenBuilder {
    fn simple_n_m_leaper(n: usize, m: usize) -> Self {
        Leaper(LeaperBBBuilder::simple_n_m(n, m))
    }
    fn simple_leaper(offsets: Vec<RayDir>) -> Self {
        Leaper(LeaperBBBuilder::simple(offsets))
    }

    fn simple_n_m_rider(n: usize, m: usize) -> Self {
        Self::Rider(RayBBBuilder::simple_n_m(n, m))
    }

    fn build(&self, size: Size, cache: &mut PieceBuilderCache) -> Vec<[AttackBitboardGen; NUM_COLORS]> {
        // todo: Allow building non-color relative direction (where e.g. forward always means increasing y direction)
        match self {
            Leaper(leaper_builder) => leaper_builder.build(size, true, cache),
            Rider(rider_builder) => rider_builder.build(size, true, cache),
            PlaneBishop => {
                vec![[AttackBitboardGen::Rider(Bishop), AttackBitboardGen::Rider(Bishop)]]
            }
            PlaneRook => vec![[AttackBitboardGen::Rider(Rook), AttackBitboardGen::Rider(Rook)]],
            PlaneQueen => vec![[AttackBitboardGen::Rider(Queen), AttackBitboardGen::Rider(Queen)]],
            &AttackBBGenBuilder::Castle(side) => {
                vec![[AttackBitboardGen::Castling(side), AttackBitboardGen::Castling(side)]]
            }
            AttackBBGenBuilder::Drop => vec![[AttackBitboardGen::Drop, AttackBitboardGen::Drop]],
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Arbitrary)]
pub(super) struct AttackKindBuilder {
    pub(super) build_col_relative: bool,
    pub(super) attack_bb_gen: AttackBBGenBuilder,
    pub(super) required: RequiredForAttack,
    pub(super) condition: GenAttacksCondition,
    pub(super) modality: Modality, // TODO: Remove
    pub(super) bitboard_filter: Vec<SquareFilter>,
    pub(super) kind: GenAttackKind,
    pub(super) capture_condition: CaptureCondition,
}

fn build_rider_ray(ray: RayDescription, cache: &mut PieceBuilderCache) -> Arc<[RawBitboard]> {
    let entry = cache.ray_cache.entry(ray);
    entry.or_insert_with(|| rider(ray)).clone()
}

fn build_rider_pair(
    mut ray: RayDescription,
    side_relative: bool,
    cache: &mut PieceBuilderCache,
) -> [AttackBitboardGen; NUM_COLORS] {
    ray.dir.dx %= ray.size.width.val() as isize;
    let first_player = build_rider_ray(ray, cache);
    if side_relative {
        ray.dir.dy = -ray.dir.dy;
    }
    let second_player = build_rider_ray(ray, cache);
    [AttackBitboardGen::rider(first_player), AttackBitboardGen::rider(second_player)]
}

impl AttackKindBuilder {
    pub fn build(&self, size: Size, cache: &mut PieceBuilderCache) -> AttackKind {
        let bb_gen = self.attack_bb_gen.build(size, cache);
        AttackKind {
            required: self.required,
            condition: self.condition,
            modality: self.modality,
            bb_gen,
            bitboard_filter: self.bitboard_filter.clone(),
            kind: self.kind,
            capture_condition: self.capture_condition,
        }
    }

    fn simple(attack_bb_gen: AttackBBGenBuilder) -> Self {
        Self {
            required: RequiredForAttack::PieceOnBoard,
            attack_bb_gen,
            condition: Always,
            bitboard_filter: vec![NotUs],
            kind: Normal,
            modality: Modality::Both,
            capture_condition: CaptureCondition::DestOccupied,
            build_col_relative: true,
        }
    }

    pub fn pawn_noncapture(attack_bb_gen: AttackBBGenBuilder) -> Self {
        Self {
            build_col_relative: true,
            required: RequiredForAttack::PieceOnBoard,
            attack_bb_gen,
            condition: Always,
            bitboard_filter: vec![EmptySquares],
            kind: Normal,
            modality: Modality::NonCapture,
            capture_condition: CaptureCondition::Never,
        }
    }

    pub fn pawn_double(col: Color) -> Self {
        // a double move is implemented as a forward slider that then gets filtered to only the 4th/5th rank
        if col.is_first() {
            AttackKindBuilder {
                build_col_relative: false,
                required: RequiredForAttack::PieceOnBoard,
                attack_bb_gen: Rider(RayBBBuilder::simple(vec![RayDir { dx: 0, dy: 1 }])),
                condition: OnRelativeRank(1, Color::first()),
                bitboard_filter: vec![EmptySquares, SquareFilter::Rank(3)],
                kind: DoublePawnPush,
                modality: Modality::NonCapture,
                capture_condition: CaptureCondition::Never,
            }
        } else {
            AttackKindBuilder {
                build_col_relative: false,
                required: RequiredForAttack::PieceOnBoard,
                // 1 because currently this is always build player-relative. TODO: Combine black_double and white_double
                attack_bb_gen: Rider(RayBBBuilder::simple(vec![RayDir { dx: 0, dy: 1 }])),
                condition: OnRelativeRank(1, Color::second()),
                bitboard_filter: vec![EmptySquares, RanksRelative(vec![3], PlayerCond::Second)],
                kind: DoublePawnPush,
                modality: Modality::NonCapture,
                capture_condition: CaptureCondition::Never,
            }
        }
    }

    pub fn only_capture(attack_bb_gen: AttackBBGenBuilder, bb_filter: SquareFilter) -> Self {
        Self {
            build_col_relative: true,
            required: RequiredForAttack::PieceOnBoard,
            attack_bb_gen,
            condition: Always,
            bitboard_filter: vec![bb_filter],
            kind: Normal,
            modality: Modality::Capture,
            capture_condition: CaptureCondition::Always,
        }
    }
    pub fn drop(filter: Vec<SquareFilter>) -> Self {
        Self {
            build_col_relative: false,
            required: RequiredForAttack::PieceInHand,
            condition: Always,
            modality: Modality::NonCapture,
            attack_bb_gen: AttackBBGenBuilder::Drop,
            bitboard_filter: filter,
            kind: Drop,
            capture_condition: CaptureCondition::Never,
        }
    }
}

#[derive(Debug, Copy, Clone, Arbitrary)]
pub(super) struct DropInfo {
    // Disallow dropping two pieces of the same type on the same file
    pub drop_no_double: bool,
}

impl Default for DropInfo {
    fn default() -> Self {
        Self { drop_no_double: false }
    }
}

#[derive(Debug, Clone, Arbitrary)]
pub(super) struct PieceBuilder {
    pub(super) name: String,
    /// Some "pieces" don't belong to a player, such as gaps/blocked squares, environmental effects, or
    /// (not currently used) actual neutral pieces. If a piece can be both colored and neutral, this currently has to be simulated
    /// using two different pieces.
    pub(super) uncolored: bool,
    pub(super) uncolored_symbol: [char; NUM_CHAR_TYPES],
    pub(super) player_symbol: [[char; NUM_CHAR_TYPES]; NUM_COLORS],
    /// Most of the attack data is represented with a bitboard.
    /// To distinguish between different special moves, the [`AttackKind`] struct has a [`GenAttackKind`] field.
    pub(super) attacks: Vec<AttackKindBuilder>,
    /// Promotions change the piece type and can differentiate moves with otherwise identical information.
    /// However, they are not the only way to change piece types; this can also be done through move effects based on the move kind.
    pub(super) promotions: Promo,
    pub(super) can_ep_capture: bool,
    pub(super) resets_draw_counter: DrawCtrReset,
    pub(super) drop_info: Option<DropInfo>,
    pub(super) royal: bool,
    /// The move output (compact and SAN) can omit the piece type. This is true for generalized pawns, but also mnk pieces.
    pub(super) output_omit_piece: bool,
    /// true for kings but not for rooks
    pub(super) can_castle: bool,
}

impl PieceBuilder {
    pub fn build(&self, size: Size, idx: usize, cache: &mut PieceBuilderCache) -> Piece {
        let mut attacks = self.attacks.clone();
        if let Some(drop_info) = self.drop_info {
            if !attacks.iter().any(|a| a.kind == Drop) {
                attacks.push(AttackKindBuilder::drop(vec![EmptySquares]));
            }
            for a in &mut attacks {
                if a.kind == Drop {
                    if drop_info.drop_no_double {
                        a.bitboard_filter.push(Not(Box::new(SquareFilter::SameFile(Box::new(SquareFilter::Has(
                            PieceCond::Only(PieceId::new(idx)),
                            PlayerCond::Active,
                        ))))));
                    }
                }
            }
        }
        let attacks = self.attacks.iter().map(|a| a.build(size, cache)).collect_vec();
        Piece {
            idx,
            name: self.name.clone(),
            uncolored: self.uncolored,
            uncolored_symbol: self.uncolored_symbol,
            player_symbol: self.player_symbol,
            attacks,
            promotions: self.promotions.clone(),
            can_ep_capture: self.can_ep_capture,
            resets_draw_counter: self.resets_draw_counter.clone(),
            royal: self.royal,
            output_omit_piece: self.output_omit_piece,
            can_castle: self.can_castle,
        }
    }

    pub fn set_unicode_symbol(&mut self, symbol: char) {
        self.uncolored_symbol[Unicode] = symbol;
        self.player_symbol[Color::first()][Unicode] = symbol;
        self.player_symbol[Color::second()][Unicode] = symbol;
    }

    pub fn set_ascii_symbol(&mut self, symbol: char) {
        self.uncolored_symbol[Ascii] = symbol;
        self.player_symbol[Color::first()][Ascii] = symbol.to_ascii_uppercase();
        self.player_symbol[Color::second()][Ascii] = symbol.to_ascii_lowercase();
    }

    pub fn matches_char(&self, c: char) -> bool {
        self.uncolored_symbol.contains(&c) || self.player_symbol[0].contains(&c) || self.player_symbol[1].contains(&c)
    }

    pub fn add_attack(mut self, attack: AttackKindBuilder) -> Self {
        self.attacks.push(attack);
        self
    }

    pub fn new_for(
        name: &str,
        attacks: Vec<AttackKindBuilder>,
        ascii_char: char,
        unicode_chars: Option<[char; 3]>,
    ) -> Self {
        let lowercase_ascii = ascii_char.to_ascii_lowercase();
        let uppercase_ascii = ascii_char.to_ascii_uppercase();
        let [white_uni, black_uni, uncolored_uni] = if let Some(unicode) = unicode_chars {
            unicode
        } else {
            [uppercase_ascii, lowercase_ascii, uppercase_ascii]
        };
        Self {
            name: name.to_string(),
            uncolored: false,
            uncolored_symbol: [uppercase_ascii, uncolored_uni],
            player_symbol: [[uppercase_ascii, white_uni], [lowercase_ascii, black_uni]],
            attacks,
            promotions: Promo::none(),
            can_ep_capture: false,
            resets_draw_counter: DrawCtrReset::Never,
            drop_info: None,
            royal: false,
            output_omit_piece: false,
            can_castle: false,
        }
    }

    pub fn new(
        name: &str,
        attacks: Vec<AttackBBGenBuilder>,
        ascii_char: char,
        unicode_chars: Option<[char; 3]>,
    ) -> Self {
        let attacks = attacks.into_iter().map(AttackKindBuilder::simple).collect_vec();
        Self::new_for(name, attacks, ascii_char, unicode_chars)
    }

    pub fn leaper(name: &str, n: usize, m: usize, ascii_char: Option<char>, unicode: Option<[char; 3]>) -> Self {
        let ascii = ascii_char.unwrap_or(name.chars().next().unwrap());
        Self::new(name, vec![AttackBBGenBuilder::Leaper(LeaperBBBuilder::simple_n_m(n, m))], ascii, unicode)
    }

    fn chess_pawn_no_promo() -> Self {
        let single_push = Leaper(LeaperBBBuilder::simple(vec![RayDir { dx: 0, dy: 1 }]));
        let single_push = AttackKindBuilder::pawn_noncapture(single_push);
        let capture = Leaper(LeaperBBBuilder::simple(vec![RayDir { dx: 1, dy: 1 }, RayDir { dx: -1, dy: 1 }]));
        let capture = AttackKindBuilder::only_capture(capture, SquareFilter::PawnCapture);
        // promotions are handled as effects instead of duplicating all normal and capture moves
        // a double move is implemented as a forward slider that then gets filtered to only the 4th/5th rank
        let white_double = AttackKindBuilder::pawn_double(Color::first());
        let black_double = AttackKindBuilder::pawn_double(Color::second());
        let mut res = Self::pawn_shatranj_no_promo();
        res.name = "pawn".to_string();
        res.attacks = vec![single_push, capture, white_double, black_double];
        res.can_ep_capture = true;
        res
    }

    // like the chess pawn, but without double pawn push and ep
    fn pawn_shatranj_no_promo() -> Self {
        let push = Leaper(LeaperBBBuilder::simple(vec![RayDir { dx: 0, dy: 1 }]));
        let push = AttackKindBuilder::pawn_noncapture(push);
        let capture = Leaper(LeaperBBBuilder::simple(vec![RayDir { dx: 1, dy: 1 }, RayDir { dx: -1, dy: 1 }]));
        let capture = AttackKindBuilder::only_capture(capture, SquareFilter::PawnCapture);
        Self {
            name: "pawn (shatranj)".to_string(),
            uncolored: false,
            uncolored_symbol: ['P', UNICODE_NEUTRAL_PAWN],
            player_symbol: [['P', UNICODE_WHITE_PAWN], ['p', UNICODE_BLACK_PAWN]],

            attacks: vec![push, capture],
            // the promotion pieces are set later, once it's known which pieces are available
            promotions: Promo {
                pieces: vec![],
                condition: PromoCondition::TargetSquare,
                forced_promo_zone: RanksRelative(vec![0], PlayerCond::All),
                optional_promo_zone: NoSquares,
                promoted_from: None,
                promoted_version: None,
            },
            can_ep_capture: false,
            resets_draw_counter: DrawCtrReset::Always,
            drop_info: Some(DropInfo::default()),
            royal: false,
            output_omit_piece: true,
            can_castle: false,
        }
    }

    fn ferz() -> Self {
        Self::leaper("ferz", 1, 1, None, Some(['\u{1FA54}', '\u{1FA56}', '\u{1FA55}']))
    }

    fn knight() -> Self {
        Self::leaper(
            "knight",
            1,
            2,
            Some('n'),
            Some([UNICODE_WHITE_KNIGHT, UNICODE_BLACK_KNIGHT, UNICODE_NEUTRAL_KNIGHT]),
        )
    }

    fn silver_no_drop() -> Self {
        let mut offsets = [-1, 0, 1].into_iter().cartesian_product([1]).map(|(dx, dy)| RayDir { dx, dy }).collect_vec();
        offsets
            .append(&mut ([-1, 1].into_iter().cartesian_product([-1]).map(|(dx, dy)| RayDir { dx, dy }).collect_vec()));
        let attacks = AttackBBGenBuilder::simple_leaper(offsets);
        Self::new("silver", vec![attacks], 's', Some(['銀', '銀', '銀']))
    }

    fn gold_no_drop() -> Self {
        let offsets = vec![(0, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)]
            .into_iter()
            .map(|(dx, dy)| RayDir { dx, dy })
            .collect_vec();
        let attacks = AttackBBGenBuilder::simple_leaper(offsets);
        Self::new("gold", vec![attacks], 'g', Some(['金', '金', '金']))
    }

    fn bishop() -> Self {
        Self::new(
            "bishop",
            vec![PlaneBishop],
            'b',
            Some([UNICODE_WHITE_BISHOP, UNICODE_BLACK_BISHOP, UNICODE_NEUTRAL_BISHOP]),
        )
    }

    fn rook() -> Self {
        Self::new("rook", vec![PlaneRook], 'r', Some([UNICODE_WHITE_ROOK, UNICODE_BLACK_ROOK, UNICODE_NEUTRAL_ROOK]))
    }

    fn queen() -> Self {
        Self::new(
            "queen",
            vec![PlaneQueen],
            'q',
            Some([UNICODE_WHITE_QUEEN, UNICODE_BLACK_QUEEN, UNICODE_NEUTRAL_QUEEN]),
        )
    }

    fn king_shatranj() -> Self {
        let mut res = Self::new(
            "king (shatranj)",
            vec![Leaper(LeaperBBBuilder::radius_up_to(1))],
            'k',
            Some([UNICODE_WHITE_KING, UNICODE_BLACK_KING, UNICODE_NEUTRAL_KING]),
        );
        res.royal = true;
        res
    }

    pub fn pieces() -> Vec<Self> {
        // order of leapers matters
        let mut leapers = vec![
            Self::leaper("wazir", 0, 1, None, Some(['🨠', '🨦', '🨬'])),
            Self::ferz(),
            Self::leaper("dabbaba", 0, 2, None, None),
            Self::knight(),
            Self::leaper("alfil", 2, 2, Some('b'), Some(['\u{1FA55}', '\u{1FA57}', '\u{1FA55}'])),
            Self::leaper("threeleaper", 0, 3, Some('h'), None),
            Self::leaper("camel", 1, 3, None, Some(['🨢', '🨨', '🨮'])),
            Self::leaper("zebra", 2, 3, None, None),
            Self::leaper("tripper", 3, 3, Some('g'), None),
            Self::leaper("fourleaper", 0, 4, None, None),
            Self::leaper("giraffe", 1, 4, None, None),
            Self::leaper("stag", 2, 4, None, None),
            Self::leaper("antelope", 3, 4, None, None),
            Self::leaper("commuter", 4, 4, None, None),
        ];
        let mut riders = vec![];
        for (idx, leaper) in leapers.iter().enumerate() {
            // see <https://stackoverflow.com/questions/40950460/how-to-convert-triangular-matrix-indexes-in-to-row-column-coordinates/40954159#40954159>
            let n = (((2 * idx + 2) as f64 + 0.25).sqrt() - 0.5).ceil() as usize;
            let m = idx + 1 - (n - 1) * n / 2;
            if max(n, m) == 1 {
                continue; // already a normal chess piece (rook or bishop)
            }
            let attacks = vec![AttackBBGenBuilder::simple_n_m_rider(n, m)];
            let name = leaper.name.clone() + "rider";
            let rider = Self::new(&name, attacks, name.chars().next().unwrap(), None);
            riders.push(rider);
        }
        riders[3].name = "nightrider".to_string();
        let mut rest = vec![
            {
                let castle_king_side = AttackKindBuilder {
                    build_col_relative: false,
                    required: RequiredForAttack::PieceOnBoard,
                    condition: CanCastle(Kingside),
                    modality: Modality::NonCapture,
                    attack_bb_gen: AttackBBGenBuilder::Castle(Kingside),
                    bitboard_filter: vec![],
                    kind: Castle(Kingside),
                    capture_condition: CaptureCondition::Never,
                };
                let castle_queen_side = AttackKindBuilder {
                    build_col_relative: false,
                    required: RequiredForAttack::PieceOnBoard,
                    condition: CanCastle(Queenside),
                    modality: Modality::NonCapture,
                    attack_bb_gen: AttackBBGenBuilder::Castle(Queenside),
                    bitboard_filter: vec![],
                    kind: Castle(Queenside),
                    capture_condition: CaptureCondition::Never,
                };
                let mut res = Self::king_shatranj();
                res.name = "king".to_string();
                res.attacks.push(castle_king_side);
                res.attacks.push(castle_queen_side);
                res.can_castle = true;
                res
            },
            Self::bishop(),
            Self::rook(),
            Self::queen(),
            Self::chess_pawn_no_promo(),
            {
                let mut res = Self::chess_pawn_no_promo();
                res.name = "pawn (horde)".to_string();
                // double pushes from the backrank don't set the ep square, so their `kind` is `Normal` instead of `DoublePawnPush`
                res.attacks.push(AttackKindBuilder {
                    build_col_relative: false,
                    required: RequiredForAttack::PieceOnBoard,
                    attack_bb_gen: Rider(RayBBBuilder::simple(vec![RayDir { dx: 0, dy: 1 }])),
                    condition: OnRelativeRank(0, Color::first()),
                    bitboard_filter: vec![EmptySquares, SquareFilter::Rank(2)],
                    kind: Normal,
                    modality: Modality::NonCapture,
                    capture_condition: CaptureCondition::Never,
                });
                res.attacks.push(AttackKindBuilder {
                    build_col_relative: false,
                    required: RequiredForAttack::PieceOnBoard,
                    attack_bb_gen: Rider(RayBBBuilder::simple(vec![RayDir { dx: 0, dy: 1 }])),
                    condition: OnRelativeRank(0, Color::second()),
                    bitboard_filter: vec![EmptySquares, RanksRelative(vec![2], PlayerCond::Second)],
                    kind: Normal,
                    modality: Modality::NonCapture,
                    capture_condition: CaptureCondition::Never,
                });
                res
            },
            Self::king_shatranj(),
            Self::pawn_shatranj_no_promo(),
            {
                let mut cowrie = Self::pawn_shatranj_no_promo();
                cowrie.name = "cowrie".to_string();
                cowrie.uncolored_symbol[Unicode] = 'บ';
                cowrie.resets_draw_counter = DrawCtrReset::Never;
                cowrie.promotions.forced_promo_zone = RanksRelative(vec![2], PlayerCond::All);
                cowrie
            },
            {
                let mut promo_cowrie = Self::ferz();
                promo_cowrie.name = "promoted cowrie".to_string();
                promo_cowrie.uncolored_symbol = ['M', 'M'];
                promo_cowrie.player_symbol = [['M', 'M'], ['m', 'm']];
                promo_cowrie
            },
            {
                let mut khon = Self::silver_no_drop();
                khon.name = "khon".to_string();
                khon.uncolored_symbol = ['S', 'S'];
                khon.player_symbol = [['S', 'S'], ['s', 's']];
                khon
            },
            {
                let mut met = Self::ferz();
                met.name = "met".to_string();
                met.uncolored_symbol = ['M', 'ค'];
                met.player_symbol = [['M', 'M'], ['m', 'm']];
                met
            },
            {
                let mut ma = Self::knight();
                ma.name = "ma".to_string();
                ma.uncolored_symbol[Unicode] = 'ม';
                ma
            },
            {
                let mut ruea = Self::rook();
                ruea.name = "ruea".to_string();
                ruea.uncolored_symbol[Unicode] = 'ร';
                ruea
            },
            {
                let mut khun = Self::king_shatranj();
                khun.name = "khun".to_string();
                khun.uncolored_symbol[Unicode] = 'ข';
                khun
            },
            {
                let mut shogi_pawn = Self::pawn_shatranj_no_promo();
                shogi_pawn.name = "shogiPawn".to_string();
                shogi_pawn.uncolored_symbol[Unicode] = '歩';
                shogi_pawn.player_symbol[Color::first()][Unicode] = '歩';
                shogi_pawn.player_symbol[Color::second()][Unicode] = '歩';
                let white_attacks = AttackBBGenBuilder::simple_leaper(vec![RayDir { dx: 0, dy: 1 }]);
                // let black_attacks = AttackBuilder::range_hv(&[0], &[-1]);
                shogi_pawn.attacks = vec![
                    AttackKindBuilder::simple(white_attacks),
                    // AttackKindBuilder::simple(black_attacks).with_cond(Player(Color::second())),
                    AttackKindBuilder::drop(vec![
                        EmptySquares,
                        Not(Box::new(RanksRelative(vec![0], Inactive))), // TODO: This can be converted into a bitboard when building the board
                        Not(Box::new(SquareFilter::SameFile(Box::new(SquareFilter::Has(
                            PieceCond::Only(PieceId::new(0)),
                            PlayerCond::Active,
                        ))))),
                        // the no-checkmate-after-drop condition is checked pseudolegaly
                    ]),
                ];
                shogi_pawn
            },
            Self::gold_no_drop().add_attack(AttackKindBuilder::drop(vec![EmptySquares])),
            Self::silver_no_drop().add_attack(AttackKindBuilder::drop(vec![EmptySquares])),
            Self::new_for(
                "knight (shogi)",
                vec![
                    AttackKindBuilder::simple(AttackBBGenBuilder::simple_leaper(vec![
                        RayDir { dx: -1, dy: 2 },
                        RayDir { dx: 1, dy: 2 },
                    ])),
                    AttackKindBuilder::drop(vec![EmptySquares, Not(Box::new(RanksRelative(vec![0, 1], Inactive)))]),
                ],
                'n',
                Some(['桂', '桂', '桂']),
            ),
            Self::new_for(
                "lance",
                vec![
                    AttackKindBuilder::simple(Rider(RayBBBuilder::simple(vec![RayDir { dx: 0, dy: 1 }]))),
                    AttackKindBuilder::drop(vec![EmptySquares, Not(Box::new(RanksRelative(vec![0], Inactive)))]),
                ],
                'l',
                Some(['香', '香', '香']),
            ),
            Self::new(
                "bers", // wikipedia calls it a dragon or dragon king, but fairy-sf uses bers
                vec![PlaneRook, AttackBBGenBuilder::simple_n_m_leaper(1, 1)],
                'd',
                Some(['龍', '龍', '龍']),
            ),
            Self::new(
                "dragonHorse",
                vec![PlaneBishop, AttackBBGenBuilder::simple_n_m_leaper(1, 0)],
                'h',
                Some(['馬', '馬', '馬']),
            ),
            Self::new(
                "go-between",
                vec![AttackBBGenBuilder::simple_leaper(vec![RayDir { dx: 0, dy: -1 }, RayDir { dx: 0, dy: 1 }])],
                'g',
                None,
            ),
            // compound pieces
            Self::new(
                "archbishop",
                vec![AttackBBGenBuilder::simple_n_m_leaper(2, 1), PlaneBishop],
                'a',
                Some(['🩐', '🩓', '🩐']),
            ),
            Self::new(
                "chancellor",
                vec![AttackBBGenBuilder::simple_n_m_leaper(2, 1), PlaneRook],
                'c',
                Some(['🩏', '🩒', '🩏']),
            ),
            Self::new(
                "amazon",
                vec![AttackBBGenBuilder::simple_n_m_leaper(2, 1), PlaneQueen],
                'a',
                Some(['🩎', '🩑', '🩎']),
            ),
            Self::new(
                "kirin",
                vec![AttackBBGenBuilder::simple_n_m_leaper(1, 1), AttackBBGenBuilder::simple_n_m_leaper(2, 0)],
                'f',
                None,
            ),
            Self::new(
                "frog",
                vec![AttackBBGenBuilder::simple_n_m_leaper(1, 1), AttackBBGenBuilder::simple_n_m_leaper(3, 0)],
                'f',
                None,
            ),
            Self::new(
                "gnu",
                vec![AttackBBGenBuilder::simple_n_m_leaper(2, 1), AttackBBGenBuilder::simple_n_m_leaper(3, 1)],
                'g',
                None,
            ),
            Self {
                name: "mnk".to_string(),
                uncolored: false,
                uncolored_symbol: ['x', UNICODE_X],
                player_symbol: [['X', UNICODE_X], ['O', UNICODE_O]],
                attacks: vec![AttackKindBuilder::drop(vec![EmptySquares])],
                promotions: Promo::none(),
                can_ep_capture: false,
                resets_draw_counter: DrawCtrReset::Never,
                drop_info: Some(DropInfo::default()),
                royal: false,
                // we set `output_as_pawn` to true because we don't want to print the piece type
                output_omit_piece: true,
                can_castle: false,
            },
            Self {
                name: "cfour".to_string(),
                uncolored: false,
                uncolored_symbol: ['x', UNICODE_X],
                player_symbol: [['X', UNICODE_X], ['O', UNICODE_O]],
                attacks: vec![AttackKindBuilder::drop(vec![
                    EmptySquares,
                    Not(Box::new(InDirectionOf(Box::new(EmptySquares), Dir::North))),
                ])],
                promotions: Promo::none(),
                can_ep_capture: false,
                resets_draw_counter: DrawCtrReset::Never,
                drop_info: Some(DropInfo::default()),
                royal: false,
                output_omit_piece: true,
                can_castle: false,
            },
            Self {
                name: "ataxx".to_string(),
                uncolored: false,
                uncolored_symbol: ['x', UNICODE_X],
                player_symbol: [['x', 'X'], ['o', 'O']],
                attacks: vec![
                    AttackKindBuilder::drop(vec![EmptySquares, SquareFilter::Neighbor(Box::new(SquareFilter::Us))]),
                    AttackKindBuilder {
                        required: RequiredForAttack::PieceOnBoard,
                        condition: Always,
                        modality: Modality::Both,
                        attack_bb_gen: Leaper(LeaperBBBuilder::radius_exact(2)),
                        bitboard_filter: vec![EmptySquares],
                        kind: Normal,
                        capture_condition: CaptureCondition::Never,
                        build_col_relative: false,
                    },
                ],
                promotions: Promo::none(),
                can_ep_capture: false,
                resets_draw_counter: DrawCtrReset::MoveKind(vec![MoveKind::Drop(0)]),
                drop_info: Some(DropInfo::default()),
                royal: false,
                output_omit_piece: true,
                can_castle: false,
            },
            Self {
                name: "gap".to_string(),
                uncolored: true,
                uncolored_symbol: ['-', '-'],
                player_symbol: [[' ', ' '], [' ', ' ']],
                attacks: vec![],
                promotions: Promo::none(),
                can_ep_capture: false,
                resets_draw_counter: DrawCtrReset::Never,
                drop_info: None,
                royal: false,
                output_omit_piece: true,
                can_castle: false,
            },
        ];
        rest.append(&mut leapers);
        rest.append(&mut riders);
        rest
    }

    pub fn chess_pieces() -> Vec<Self> {
        let mut pieces = Self::complete_piece_map();
        let mut res = vec![
            pieces.remove("pawn").unwrap(),
            pieces.remove("knight").unwrap(),
            pieces.remove("bishop").unwrap(),
            pieces.remove("rook").unwrap(),
            pieces.remove("queen").unwrap(),
            pieces.remove("king").unwrap(),
        ];
        for p in 1..5 {
            res[0].promotions.pieces.push(PieceId::new(p));
        }
        res
    }

    pub fn shatranj_pieces() -> Vec<Self> {
        let mut pieces = Self::complete_piece_map();
        let mut res = vec![
            pieces.remove("pawn (shatranj)").unwrap(),
            pieces.remove("knight").unwrap(),
            pieces.remove("alfil").unwrap(),
            pieces.remove("rook").unwrap(),
            pieces.remove("ferz").unwrap(),
            pieces.remove("king (shatranj)").unwrap(),
        ];
        res[0].promotions.pieces.push(PieceId::new(4));
        res
    }

    pub fn makruk_pieces() -> Vec<Self> {
        let mut pieces = Self::complete_piece_map();
        let mut res = vec![
            pieces.remove("cowrie").unwrap(),
            pieces.remove("ma").unwrap(),
            pieces.remove("khon").unwrap(),
            pieces.remove("met").unwrap(),
            pieces.remove("ruea").unwrap(),
            pieces.remove("khun").unwrap(),
            pieces.remove("promoted cowrie").unwrap(),
        ];
        let promo_id = PieceId::new(res.len() - 1);
        res[0].promotions.pieces.push(promo_id);
        res
    }

    pub fn shogi_pieces() -> Vec<Self> {
        let mut pieces = Self::complete_piece_map();
        let gold = pieces.remove("gold").unwrap();
        // cloning precomputed piece attack bitboards uses copy-on-write semantics,
        // so gold general attack bitboards aren't duplicated
        let pawn = pieces.remove("shogiPawn").unwrap();
        let mut tokin = gold.clone();
        tokin.name = "tokin".to_string();
        tokin.set_unicode_symbol('と');
        tokin.set_ascii_symbol('p');
        let lance = pieces.remove("lance").unwrap();
        let mut promoted_lance = lance.clone();
        promoted_lance.name = "promoted lance".to_string();
        promoted_lance.set_unicode_symbol('杏');
        promoted_lance.attacks = gold.attacks.clone();
        let knight = pieces.remove("knight (shogi)").unwrap();
        let mut promoted_knight = knight.clone();
        promoted_knight.name = "promoted knight".to_string();
        promoted_knight.set_unicode_symbol('圭');
        promoted_knight.attacks = gold.attacks.clone();
        let silver = pieces.remove("silver").unwrap();
        let mut promoted_silver = silver.clone();
        promoted_silver.name = "promoted silver".to_string();
        promoted_silver.set_unicode_symbol('全');
        promoted_silver.attacks = gold.attacks.clone();
        // like in shogi, there are no castling moves in shatranj
        let mut king = pieces.remove("king (shatranj)").unwrap();
        king.name = "king (shogi)".to_string();
        king.set_unicode_symbol('玉');
        let mut bishop = pieces.remove("bishop").unwrap();
        bishop.set_unicode_symbol('角');
        bishop = bishop.add_attack(AttackKindBuilder::drop(vec![EmptySquares]));
        let mut rook = pieces.remove("rook").unwrap();
        rook.set_unicode_symbol('飛');
        rook = rook.add_attack(AttackKindBuilder::drop(vec![EmptySquares]));
        let mut res = vec![
            pawn,
            lance,
            knight,
            silver,
            bishop,
            rook,
            gold,
            king,
            tokin,
            promoted_lance,
            promoted_knight,
            promoted_silver,
            pieces.remove("dragonHorse").unwrap(),
            pieces.remove("bers").unwrap(),
        ];
        const PROMO: usize = 8;
        assert_eq!(res[PROMO].name, "tokin");
        for i in 0..PROMO - 2 {
            res[i].promotions.pieces = vec![PieceId::new(PROMO + i)];
            res[i].promotions.promoted_version = Some(PieceId::new(PROMO + i));
            res[PROMO + i].promotions.promoted_from = Some(PieceId::new(i));
            res[i].promotions.condition = PromoCondition::SourceOrTargetNoDrop;
            res[i].promotions.optional_promo_zone = RanksRelative(vec![0, 1, 2], Inactive);
            if i == 0 || i == 1 {
                res[i].promotions.forced_promo_zone = RanksRelative(vec![0], Inactive);
            } else if i == 2 {
                res[i].promotions.forced_promo_zone = RanksRelative(vec![0, 1], Inactive);
            }
        }
        res
    }

    pub fn complete_piece_map() -> HashMap<String, Self> {
        let mut res = HashMap::new();
        for piece in Self::pieces() {
            // insertion can fail because some pieces get inserted twice
            _ = res.insert(piece.name.clone(), piece);
        }
        res
    }

    pub fn find_piece_by_name(name: &str) -> Option<Self> {
        Self::pieces().into_iter().find(|p| p.name == name)
    }
}
