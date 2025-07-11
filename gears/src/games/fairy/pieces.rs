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
use crate::games::CharType::Unicode;
use crate::games::chess::pieces::{
    UNICODE_BLACK_BISHOP, UNICODE_BLACK_KING, UNICODE_BLACK_KNIGHT, UNICODE_BLACK_PAWN, UNICODE_BLACK_QUEEN,
    UNICODE_BLACK_ROOK, UNICODE_NEUTRAL_BISHOP, UNICODE_NEUTRAL_KING, UNICODE_NEUTRAL_KNIGHT, UNICODE_NEUTRAL_PAWN,
    UNICODE_NEUTRAL_QUEEN, UNICODE_NEUTRAL_ROOK, UNICODE_WHITE_BISHOP, UNICODE_WHITE_KING, UNICODE_WHITE_KNIGHT,
    UNICODE_WHITE_PAWN, UNICODE_WHITE_QUEEN, UNICODE_WHITE_ROOK,
};
use crate::games::fairy::Side::*;
use crate::games::fairy::attacks::AttackTypes::*;
use crate::games::fairy::attacks::GenAttackKind::*;
use crate::games::fairy::attacks::GenAttacksCondition::*;
use crate::games::fairy::attacks::{
    AttackKind, AttackMode, AttackTypes, CaptureCondition, Dir, LeapingBitboards, MoveKind, RequiredForAttack,
    SliderDirections,
};
use crate::games::fairy::moves::FairyMove;
use crate::games::fairy::rules::SquareFilter::{EmptySquares, InDirectionOf, NoSquares, Not, SideRelativeBitboard};
use crate::games::fairy::rules::{PieceCond, PlayerCond, PromoFenModifier, Rules, SquareFilter};
use crate::games::fairy::{FairyBitboard, FairyBoard, FairyColor, FairySize, FairySquare};
use crate::games::{AbstractPieceType, CharType, Color, ColoredPieceType, DimT, NUM_CHAR_TYPES, NUM_COLORS, PieceType};
use crate::general::bitboards::Bitboard;
use crate::general::board::Board;
use crate::general::common::Res;
use crate::general::squares::RectangularSize;
use anyhow::bail;
use arbitrary::Arbitrary;
use colored::Colorize;
use itertools::Itertools;
use std::cmp::max;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use std::iter::once;

const UNICODE_X: char = '⨉'; // '⨉',
const UNICODE_O: char = '◯'; // '○'

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub struct PieceId(u8);

impl PieceId {
    pub fn new(val: usize) -> PieceId {
        Self(val as u8)
    }
    pub fn val(self) -> usize {
        self.0 as usize
    }
    pub fn as_u8(self) -> u8 {
        self.0
    }
    pub fn get(self, rules: &Rules) -> &Piece {
        &rules.pieces[self.val()]
    }
}

impl AbstractPieceType<FairyBoard> for PieceId {
    fn empty() -> Self {
        Self(u8::MAX)
    }

    fn non_empty(settings: &Rules) -> impl Iterator<Item = Self> {
        (0..settings.pieces.len()).map(Self::new)
    }

    fn to_char(self, typ: CharType, rules: &Rules) -> char {
        self.get(rules).uncolored_symbol[typ as usize]
    }

    fn from_char(c: char, rules: &Rules) -> Option<Self> {
        rules.matching_piece_ids(|p| p.uncolored_symbol.contains(&c)).next()
    }

    #[allow(refining_impl_trait)]
    fn name(&self, settings: &Rules) -> String {
        self.get(settings).name.clone()
    }

    fn max_num_chars(settings: &Rules) -> usize {
        if settings.pieces.iter().any(|p| p.promotions.promoted_from.is_some()) { 2 } else { 1 }
    }

    fn to_uncolored_idx(self) -> usize {
        self.val()
    }
}

impl PieceType<FairyBoard> for PieceId {
    type Colored = ColoredPieceId;

    fn from_idx(idx: usize) -> Self {
        Self::new(idx)
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub struct ColoredPieceId {
    id: PieceId,
    color: Option<FairyColor>,
}

impl ColoredPieceId {
    pub fn as_u8(&self) -> u8 {
        self.id.0 * 3 + self.color.map_or(0, |c| c.idx() as u8 + 1)
    }
    pub fn val(self) -> usize {
        self.as_u8() as usize
    }
    pub fn from_u8(val: u8) -> Self {
        let id = PieceId(val / 3);
        let color = match val % 3 {
            0 => None,
            c => Some(FairyColor::from_idx(c as usize - 1)),
        };
        ColoredPieceId { id, color }
    }
    pub fn create(piece: PieceId, color: Option<FairyColor>) -> Self {
        ColoredPieceId { id: piece, color }
    }
}

impl AbstractPieceType<FairyBoard> for ColoredPieceId {
    fn empty() -> Self {
        Self { id: PieceId::empty(), color: None }
    }

    fn non_empty(settings: &Rules) -> impl Iterator<Item = Self> {
        settings
            .pieces
            .iter()
            .enumerate()
            .flat_map(|(idx, p)| {
                // Mapping to options is ugly but makes the compiler happy
                if p.uncolored {
                    [Some(Self { id: PieceId::new(idx), color: None }), None].into_iter()
                } else {
                    [
                        Some(Self { id: PieceId::new(idx), color: Some(FairyColor::first()) }),
                        Some(Self { id: PieceId::new(idx), color: Some(FairyColor::second()) }),
                    ]
                    .into_iter()
                }
            })
            .flatten()
    }

    fn to_char(self, typ: CharType, rules: &Rules) -> char {
        if let Some(color) = self.color {
            self.id.get(rules).player_symbol[color.idx()][typ as usize]
        } else if self == Self::empty() {
            '.'
        } else {
            self.id.get(rules).uncolored_symbol[typ as usize]
        }
    }

    fn from_char(c: char, rules: &Rules) -> Option<Self> {
        let found = rules.pieces().find(|(_id, p)| p.player_symbol.iter().any(|s| s.contains(&c)));
        if let Some((id, p)) = found {
            if p.player_symbol[CharType::Ascii].contains(&c) {
                Some(Self { id, color: Some(FairyColor::first()) })
            } else {
                Some(Self { id, color: Some(FairyColor::second()) })
            }
        } else {
            rules.matching_piece_ids(|p| p.uncolored_symbol.contains(&c)).next().map(|id| Self { id, color: None })
        }
    }

    fn name(&self, settings: &Rules) -> impl AsRef<str> {
        if let Some(color) = self.color {
            format!("{0} {1}", color.name(settings), self.id.name(settings))
        } else {
            self.id.name(settings)
        }
    }

    fn max_num_chars(settings: &Rules) -> usize {
        PieceId::max_num_chars(settings)
    }

    fn to_uncolored_idx(self) -> usize {
        self.id.val()
    }
}

impl ColoredPieceType<FairyBoard> for ColoredPieceId {
    type Uncolored = PieceId;

    fn new(color: FairyColor, uncolored: Self::Uncolored) -> Self {
        Self { id: uncolored, color: Some(color) }
    }

    fn color(self) -> Option<FairyColor> {
        self.color
    }

    fn uncolor(self) -> Self::Uncolored {
        self.id
    }

    fn to_colored_idx(self) -> usize {
        self.id.val()
    }

    fn write_as_str(
        mut self,
        rules: &Rules,
        char_type: CharType,
        display_pretty: bool,
        f: &mut Formatter<'_>,
    ) -> fmt::Result {
        let to_c = |p: Self| {
            if display_pretty { p.to_char(char_type, rules) } else { p.to_display_char(char_type, rules) }
        };
        if let Some(unpromoted) = self.promoted_from(rules) {
            match rules.format_rules.promo_fen_modifier {
                PromoFenModifier::Crazyhouse => write!(f, "{}~", to_c(self)),
                PromoFenModifier::Shogi => {
                    self.id = unpromoted;
                    write!(f, "+{}", to_c(self))
                }
            }
        } else {
            write!(f, "{}", to_c(self))
        }
    }

    fn make_promoted(&mut self, rules: &Rules) -> Res<()> {
        let Some(promoted) = self.id.get(rules).promotions.promoted_version else {
            bail!(
                "The piece '{0}' can't be marked as having been promoted. Current variant: {1}",
                self.name(rules).as_ref().bold(),
                rules.name.bold()
            )
        };
        self.id = promoted;
        Ok(())
    }

    fn promoted_from(&self, rules: &Rules) -> Option<PieceId> {
        self.id.get(rules).promotions.promoted_from
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub enum GenPromoMoves {
    NoPromo,
    ForcedPromo,
    OptionalPromo,
}

#[derive(Debug, Default, Copy, Clone, Arbitrary)]
#[must_use]
pub enum PromoCondition {
    #[default]
    Never,
    TargetSquare,
    SourceOrTargetNoDrop, // used in shogi
}

#[derive(Debug, Clone, Arbitrary)]
#[must_use]
pub(super) struct Promo {
    pub pieces: Vec<PieceId>,
    pub optional_promo_zone: SquareFilter,
    pub forced_promo_zone: SquareFilter,
    pub condition: PromoCondition,
    // Only set in variants where this matters, like crazyhouse and shogi, but not in e.g. chess.
    // In crazyhouse, this is always set to pawn, and used to add the unpromoted version to the hand.
    pub promoted_from: Option<PieceId>,
    // when reading a fen, this is what the promotion modifier turns the piece into.
    pub promoted_version: Option<PieceId>,
}

impl Promo {
    pub fn none() -> Self {
        Self {
            pieces: vec![],
            optional_promo_zone: NoSquares,
            forced_promo_zone: NoSquares,
            condition: PromoCondition::Never,
            promoted_from: None,
            promoted_version: None,
        }
    }

    fn gen_promo_impl<F: Fn(FairyBitboard) -> bool>(&self, contained: F, pos: &FairyBoard) -> GenPromoMoves {
        if contained(self.forced_promo_zone.bb(pos.active_player(), pos)) {
            GenPromoMoves::ForcedPromo
        } else if contained(self.optional_promo_zone.bb(pos.active_player(), pos)) {
            GenPromoMoves::OptionalPromo
        } else {
            GenPromoMoves::NoPromo
        }
    }

    pub fn gen_promo(&self, source: FairySquare, dest: FairySquare, pos: &FairyBoard) -> GenPromoMoves {
        match self.condition {
            PromoCondition::Never => GenPromoMoves::NoPromo,
            PromoCondition::TargetSquare => self.gen_promo_impl(|bb| bb.is_bit_set(dest), pos),
            PromoCondition::SourceOrTargetNoDrop => {
                let promo = |bb: FairyBitboard| {
                    source != FairySquare::no_coordinates() && (bb.is_bit_set(dest) || bb.is_bit_set(source))
                };
                self.gen_promo_impl(promo, pos)
            }
        }
    }
}

#[derive(Debug, Clone, Arbitrary)]
pub enum DrawCtrReset {
    Always,
    Never,
    MoveKind(Vec<MoveKind>),
}

impl DrawCtrReset {
    pub fn reset(&self, mov: FairyMove) -> bool {
        match self {
            DrawCtrReset::Always => true,
            DrawCtrReset::Never => false,
            DrawCtrReset::MoveKind(vec) => vec.contains(&mov.kind()),
        }
    }
}

pub(super) const PAWN_IDX: usize = 0;
#[allow(unused)]
pub(super) const CHESS_KNIGHT_IDX: usize = 1;
#[allow(unused)]
pub(super) const CHESS_BISHOP_IDX: usize = 2;
#[allow(unused)]
pub(super) const CHESS_ROOK_IDX: usize = 3;
#[allow(unused)]
pub(super) const CHESS_QUEEN_IDX: usize = 4;
#[allow(unused)]
pub(super) const CHESS_KING_IDX: usize = 5;

/// This struct defines the rules for a single piece.
// Cloning a piece uses copy-on-write semantics for attack bitboards
#[derive(Debug, Clone, Arbitrary)]
#[must_use]
pub struct Piece {
    pub(super) name: String,
    // Some "pieces" don't belong to a player, such as gaps/blocked squares, environmental effects, or
    // (not currently used) actual neutral pieces. If a piece can be both colored and neutral, this currently has to be simulated
    // using two different pieces.
    pub(super) uncolored: bool,
    pub(super) uncolored_symbol: [char; NUM_CHAR_TYPES],
    pub(super) player_symbol: [[char; NUM_CHAR_TYPES]; NUM_COLORS],
    /// Most of the attack data is represented with a bitboard.
    /// To distinguish between different special moves, the [`AttackKind`] struct has a [`GenAttackKind`] field.
    pub(super) attacks: Vec<AttackKind>,
    /// Promotions change the piece type and can differentiate moves with otherwise identical information.
    /// However, they are not the only way to change piece types; this can also be done through move effects based on the move kind.
    pub(super) promotions: Promo,
    pub(super) can_ep_capture: bool,
    pub(super) resets_draw_counter: DrawCtrReset,
    pub(super) royal: bool,
    // true for kings but not for rooks
    pub(super) can_castle: bool,
}

impl Piece {
    pub fn set_unicode_symbol(&mut self, symbol: char) {
        self.uncolored_symbol[Unicode] = symbol;
        self.player_symbol[FairyColor::first()][Unicode] = symbol;
        self.player_symbol[FairyColor::second()][Unicode] = symbol;
    }

    pub fn add_attack(mut self, attack: AttackKind) -> Self {
        self.attacks.push(attack);
        self
    }

    pub fn new_for(name: &str, attacks: Vec<AttackKind>, ascii_char: char, unicode_chars: Option<[char; 3]>) -> Self {
        let lowercase_ascii = ascii_char.to_ascii_lowercase();
        let uppercase_ascii = ascii_char.to_ascii_uppercase();
        let [u_white, u_black, u_uncolored] = if let Some(unicode) = unicode_chars {
            unicode
        } else {
            [uppercase_ascii, lowercase_ascii, uppercase_ascii]
        };
        Self {
            name: name.to_string(),
            uncolored: false,
            uncolored_symbol: [uppercase_ascii, u_uncolored],
            player_symbol: [[uppercase_ascii, u_white], [lowercase_ascii, u_black]],
            attacks,
            promotions: Promo::none(),
            can_ep_capture: false,
            resets_draw_counter: DrawCtrReset::Never,
            royal: false,
            can_castle: false,
        }
    }

    pub fn new(name: &str, attacks: Vec<AttackTypes>, ascii_char: char, unicode_chars: Option<[char; 3]>) -> Self {
        let attacks = attacks.into_iter().map(AttackKind::simple).collect_vec();
        Self::new_for(name, attacks, ascii_char, unicode_chars)
    }

    pub fn leaper(
        name: &str,
        n: usize,
        m: usize,
        size: FairySize,
        ascii_char: Option<char>,
        unicode: Option<[char; 3]>,
    ) -> Self {
        let ascii = ascii_char.unwrap_or(name.chars().next().unwrap());
        let attacks = vec![AttackTypes::leaping(n, m, size)];
        Self::new(name, attacks, ascii, unicode)
    }

    fn chess_pawn_no_promo(size: FairySize) -> Self {
        let normal_white = AttackKind::pawn_noncapture(
            Leaping(LeapingBitboards::range_hv(once(0), once(1), size)),
            Player(FairyColor::first()),
        );
        let normal_black = AttackKind::pawn_noncapture(
            Leaping(LeapingBitboards::range_hv(once(0), once(-1), size)),
            Player(FairyColor::second()),
        );
        let white_capture = AttackKind::pawn_capture(
            Leaping(LeapingBitboards::range_hv([-1, 1].into_iter(), once(1), size)),
            Player(FairyColor::first()),
            SquareFilter::PawnCapture,
        );
        let black_capture = AttackKind::pawn_capture(
            Leaping(LeapingBitboards::range_hv([-1, 1].into_iter(), once(-1), size)),
            Player(FairyColor::second()),
            SquareFilter::PawnCapture,
        );
        // promotions are handled as effects instead of duplicating all normal and capture moves
        let white_double = AttackKind {
            required: RequiredForAttack::PieceOnBoard,
            typ: Rider(SliderDirections::Vertical),
            condition: OnRank(1, FairyColor::first()),
            bitboard_filter: vec![EmptySquares, SquareFilter::Rank(3)],
            kind: DoublePawnPush,
            attack_mode: AttackMode::NoCaptures,
            capture_condition: CaptureCondition::Never,
        };
        let black_double = AttackKind {
            required: RequiredForAttack::PieceOnBoard,
            typ: Rider(SliderDirections::Vertical),
            condition: OnRank(size.height().get().saturating_sub(2), FairyColor::second()),
            bitboard_filter: vec![EmptySquares, SquareFilter::Rank(size.height().get().saturating_sub(4))],
            kind: DoublePawnPush,
            attack_mode: AttackMode::NoCaptures,
            capture_condition: CaptureCondition::Never,
        };
        let mut res = Self::pawn_shatranj_no_promo(size);
        res.name = "pawn".to_string();
        res.attacks = vec![normal_white, normal_black, white_capture, black_capture, white_double, black_double];
        res.can_ep_capture = true;
        res
    }

    // like the chess pawn, but without double pawn push and ep
    fn pawn_shatranj_no_promo(size: FairySize) -> Self {
        let normal_white = AttackKind::pawn_noncapture(
            Leaping(LeapingBitboards::range_hv(once(0), once(1), size)),
            Player(FairyColor::first()),
        );
        let normal_black = AttackKind::pawn_noncapture(
            Leaping(LeapingBitboards::range_hv(once(0), once(-1), size)),
            Player(FairyColor::second()),
        );
        let white_capture = AttackKind::pawn_capture(
            Leaping(LeapingBitboards::range_hv([-1, 1].into_iter(), once(1), size)),
            Player(FairyColor::first()),
            SquareFilter::Them,
        );
        let black_capture = AttackKind::pawn_capture(
            Leaping(LeapingBitboards::range_hv([-1, 1].into_iter(), once(-1), size)),
            Player(FairyColor::second()),
            SquareFilter::Them,
        );
        Self {
            name: "pawn (shatranj)".to_string(),
            uncolored: false,
            uncolored_symbol: ['P', UNICODE_NEUTRAL_PAWN],
            player_symbol: [['P', UNICODE_WHITE_PAWN], ['p', UNICODE_BLACK_PAWN]],

            attacks: vec![normal_white, normal_black, white_capture, black_capture],
            // the promotion pieces are set later, once it's known which pieces are available
            promotions: Promo {
                pieces: vec![],
                condition: PromoCondition::TargetSquare,
                forced_promo_zone: SquareFilter::Bitboard(FairyBitboard::backranks_for(size).raw()),
                optional_promo_zone: NoSquares,
                promoted_from: None,
                promoted_version: None,
            },
            can_ep_capture: false,
            resets_draw_counter: DrawCtrReset::Always,
            royal: false,
            can_castle: false,
        }
    }

    fn ferz(size: FairySize) -> Self {
        Self::leaper("ferz", 1, 1, size, None, Some(['\u{1FA54}', '\u{1FA56}', '\u{1FA55}']))
    }

    fn knight(size: FairySize) -> Self {
        Self::leaper(
            "knight",
            1,
            2,
            size,
            Some('n'),
            Some([UNICODE_WHITE_KNIGHT, UNICODE_BLACK_KNIGHT, UNICODE_NEUTRAL_KNIGHT]),
        )
    }

    fn silver_no_drop(size: FairySize) -> Self {
        Self::new_for(
            "silver general",
            AttackKind::simple_side_relative(
                LeapingBitboards::range_hv(-1..=1, once(1), size).combine(LeapingBitboards::range_hv(
                    [-1, 1].into_iter(),
                    once(-1),
                    size,
                )),
                size,
            ),
            's',
            Some(['銀', '銀', '銀']),
        )
    }

    fn bishop() -> Self {
        Self::new(
            "bishop",
            vec![Rider(SliderDirections::Bishop)],
            'b',
            Some([UNICODE_WHITE_BISHOP, UNICODE_BLACK_BISHOP, UNICODE_NEUTRAL_BISHOP]),
        )
    }

    fn rook() -> Self {
        Self::new(
            "rook",
            vec![Rider(SliderDirections::Rook)],
            'r',
            Some([UNICODE_WHITE_ROOK, UNICODE_BLACK_ROOK, UNICODE_NEUTRAL_ROOK]),
        )
    }

    fn queen() -> Self {
        Self::new(
            "queen",
            vec![Rider(SliderDirections::Queen)],
            'q',
            Some([UNICODE_WHITE_QUEEN, UNICODE_BLACK_QUEEN, UNICODE_NEUTRAL_QUEEN]),
        )
    }

    fn king_shatranj(size: FairySize) -> Self {
        let mut res = Self::new(
            "king (shatranj)",
            vec![Leaping(LeapingBitboards::fixed(1, 1, size).combine(LeapingBitboards::fixed(0, 1, size)))],
            'k',
            Some([UNICODE_WHITE_KING, UNICODE_BLACK_KING, UNICODE_NEUTRAL_KING]),
        );
        res.royal = true;
        res
    }

    pub fn pieces(size: FairySize) -> Vec<Self> {
        let not_their_rank =
            move |rank: DimT| (!FairyBitboard::rank_for(size.height.get().saturating_sub(1 + rank), size)).raw();
        // order of leapers matters
        let mut leapers = vec![
            Self::leaper("wazir", 0, 1, size, None, Some(['🨠', '🨦', '🨬'])),
            Self::ferz(size),
            Self::leaper("dabbaba", 0, 2, size, None, None),
            Self::knight(size),
            Self::leaper("alfil", 2, 2, size, None, Some(['\u{1FA55}', '\u{1FA57}', '\u{1FA55}'])),
            Self::leaper("threeleaper", 0, 3, size, Some('h'), None),
            Self::leaper("camel", 1, 3, size, None, Some(['🨢', '🨨', '🨮'])),
            Self::leaper("zebra", 2, 3, size, None, None),
            Self::leaper("tripper", 3, 3, size, Some('g'), None),
            Self::leaper("fourleaper", 0, 4, size, None, None),
            Self::leaper("giraffe", 1, 4, size, None, None),
            Self::leaper("stag", 2, 4, size, None, None),
            Self::leaper("antelope", 3, 4, size, None, None),
            Self::leaper("commuter", 4, 4, size, None, None),
        ];
        let mut riders = vec![];
        for (idx, leaper) in leapers.iter().enumerate() {
            // see <https://stackoverflow.com/questions/40950460/how-to-convert-triangular-matrix-indexes-in-to-row-column-coordinates/40954159#40954159>
            let n = (((2 * idx + 2) as f64 + 0.25).sqrt() - 0.5).ceil() as usize;
            let m = idx + 1 - (n - 1) * n / 2;
            if max(n, m) == 1 {
                continue; // already a normal chess piece (rook or bishop)
            }
            let attacks = vec![AttackTypes::rider(n, m, size)];
            let name = leaper.name.clone() + "rider";
            let rider = Self::new(&name, attacks, name.chars().next().unwrap(), None);
            riders.push(rider);
        }
        riders[3].name = "nightrider".to_string();
        let mut rest = vec![
            {
                let castle_king_side = AttackKind {
                    required: RequiredForAttack::PieceOnBoard,
                    condition: CanCastle(Kingside),
                    attack_mode: AttackMode::NoCaptures,
                    typ: Castling(Kingside),
                    bitboard_filter: vec![],
                    kind: Castle(Kingside),
                    capture_condition: CaptureCondition::Never,
                };
                let castle_queen_side = AttackKind {
                    required: RequiredForAttack::PieceOnBoard,
                    condition: CanCastle(Queenside),
                    attack_mode: AttackMode::NoCaptures,
                    typ: Castling(Queenside),
                    bitboard_filter: vec![],
                    kind: Castle(Queenside),
                    capture_condition: CaptureCondition::Never,
                };
                let mut res = Self::king_shatranj(size);
                res.name = "king".to_string();
                res.attacks.push(castle_king_side);
                res.attacks.push(castle_queen_side);
                res.can_castle = true;
                res
            },
            Self::bishop(),
            Self::rook(),
            Self::queen(),
            Self::chess_pawn_no_promo(size),
            {
                let mut res = Self::chess_pawn_no_promo(size);
                res.name = "pawn (horde)".to_string();
                // double pushes from the backrank don't set the ep square, so their `kind` is `Normal` instead of `DoublePawnPush`
                res.attacks.push(AttackKind {
                    required: RequiredForAttack::PieceOnBoard,
                    typ: Rider(SliderDirections::Vertical),
                    condition: OnRank(0, FairyColor::first()),
                    bitboard_filter: vec![EmptySquares, SquareFilter::Rank(2)],
                    kind: Normal,
                    attack_mode: AttackMode::NoCaptures,
                    capture_condition: CaptureCondition::Never,
                });
                res.attacks.push(AttackKind {
                    required: RequiredForAttack::PieceOnBoard,
                    typ: Rider(SliderDirections::Vertical),
                    condition: OnRank(size.height().get().saturating_sub(1), FairyColor::second()),
                    bitboard_filter: vec![EmptySquares, SquareFilter::Rank(size.height().get().saturating_sub(3))],
                    kind: Normal,
                    attack_mode: AttackMode::NoCaptures,
                    capture_condition: CaptureCondition::Never,
                });
                res
            },
            Self::king_shatranj(size),
            Self::pawn_shatranj_no_promo(size),
            {
                let mut cowrie = Self::pawn_shatranj_no_promo(size);
                cowrie.name = "cowrie".to_string();
                cowrie.uncolored_symbol[Unicode] = 'บ';
                cowrie.resets_draw_counter = DrawCtrReset::Never;
                let h = size.height.get();
                let rank_3_and_6 =
                    FairyBitboard::rank_for(h.saturating_sub(3), size) | FairyBitboard::rank_for(2.min(h - 1), size);
                cowrie.promotions.forced_promo_zone = SquareFilter::Bitboard(rank_3_and_6.raw());
                cowrie
            },
            {
                let mut promo_cowrie = Self::ferz(size);
                promo_cowrie.name = "promoted cowrie".to_string();
                promo_cowrie.uncolored_symbol = ['M', 'M'];
                promo_cowrie.player_symbol = [['M', 'M'], ['m', 'm']];
                promo_cowrie
            },
            {
                let mut khon = Self::ferz(size);
                khon.name = "khon".to_string();
                khon.uncolored_symbol = ['S', 'S'];
                khon.player_symbol = [['S', 'S'], ['s', 's']];
                khon
            },
            {
                let mut met = Self::silver_no_drop(size);
                met.name = "met".to_string();
                met.uncolored_symbol = ['M', 'ค'];
                met.player_symbol = [['M', 'M'], ['m', 'm']];
                met
            },
            {
                let mut ma = Self::knight(size);
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
                let mut khun = Self::king_shatranj(size);
                khun.name = "khun".to_string();
                khun.uncolored_symbol[Unicode] = 'ข';
                khun
            },
            Self::new_for(
                "pawn (shogi)",
                AttackKind::simple_side_relative(LeapingBitboards::range_hv(once(0), once(1), size), size),
                'p',
                Some(['歩', '歩', '歩']),
            )
            .add_attack(AttackKind::drop(vec![
                EmptySquares,
                SideRelativeBitboard(not_their_rank(0)),
                Not(Box::new(SquareFilter::SameFile(Box::new(SquareFilter::Has(
                    PieceCond::Only(PieceId(0)),
                    PlayerCond::Active,
                ))))),
                // the no-checkmate-after-drop condition is checked pseudolegaly
            ])),
            Self::new_for(
                "gold general",
                AttackKind::simple_side_relative(
                    LeapingBitboards::fixed(0, 1, size)
                        .combine(LeapingBitboards::fixed(1, 1, size))
                        .remove(LeapingBitboards::range_hv([-1, 1].into_iter(), once(-1), size)),
                    size,
                ),
                'g',
                Some(['金', '金', '金']),
            )
            .add_attack(AttackKind::drop(vec![EmptySquares])),
            Self::silver_no_drop(size).add_attack(AttackKind::drop(vec![EmptySquares])),
            Self::new_for(
                "knight (shogi)",
                AttackKind::simple_side_relative(LeapingBitboards::range_hv([-1, 1].into_iter(), once(2), size), size),
                'n',
                Some(['桂', '桂', '桂']),
            )
            .add_attack(AttackKind::drop(vec![
                EmptySquares,
                SideRelativeBitboard(not_their_rank(0) & not_their_rank(1)),
            ])),
            Self::new("lance", vec![Rider(SliderDirections::Forward)], 'l', Some(['香', '香', '香']))
                .add_attack(AttackKind::drop(vec![EmptySquares, SideRelativeBitboard(not_their_rank(0))])),
            Self::new(
                "dragon king",
                vec![Rider(SliderDirections::Rook), Leaping(LeapingBitboards::fixed(1, 1, size))],
                'd',
                Some(['龍', '龍', '龍']),
            ),
            Self::new(
                "dragon horse",
                vec![Rider(SliderDirections::Bishop), Leaping(LeapingBitboards::fixed(0, 1, size))],
                'h',
                Some(['馬', '馬', '馬']),
            ),
            Self::new(
                "go-between",
                vec![Leaping(LeapingBitboards::range_hv(once(0), [-1, 1].into_iter(), size))],
                'g',
                None,
            ),
            // compound pieces
            Self::new(
                "archbishop",
                vec![AttackTypes::leaping(1, 2, size), Rider(SliderDirections::Bishop)],
                'a',
                Some(['🩐', '🩓', '🩐']),
            ),
            Self::new(
                "chancellor",
                vec![AttackTypes::leaping(1, 2, size), Rider(SliderDirections::Rook)],
                'c',
                Some(['🩏', '🩒', '🩏']),
            ),
            Self::new(
                "amazon",
                vec![AttackTypes::leaping(1, 2, size), Rider(SliderDirections::Queen)],
                'a',
                Some(['🩎', '🩑', '🩎']),
            ),
            Self::new("kirin", vec![AttackTypes::leaping(1, 1, size), AttackTypes::leaping(0, 2, size)], 'f', None),
            Self::new("frog", vec![AttackTypes::leaping(1, 1, size), AttackTypes::leaping(0, 3, size)], 'f', None),
            Self::new("gnu", vec![AttackTypes::leaping(1, 2, size), AttackTypes::leaping(1, 3, size)], 'g', None),
            Self {
                name: "mnk".to_string(),
                uncolored: false,
                uncolored_symbol: ['x', UNICODE_X],
                player_symbol: [['X', UNICODE_X], ['O', UNICODE_O]],
                attacks: vec![AttackKind::drop(vec![EmptySquares])],
                promotions: Promo::none(),
                can_ep_capture: false,
                resets_draw_counter: DrawCtrReset::Never,
                royal: false,
                can_castle: false,
            },
            Self {
                name: "cfour".to_string(),
                uncolored: false,
                uncolored_symbol: ['x', UNICODE_X],
                player_symbol: [['X', UNICODE_X], ['O', UNICODE_O]],
                attacks: vec![AttackKind::drop(vec![
                    EmptySquares,
                    Not(Box::new(InDirectionOf(Box::new(EmptySquares), Dir::North))),
                ])],
                promotions: Promo::none(),
                can_ep_capture: false,
                resets_draw_counter: DrawCtrReset::Never,
                royal: false,
                can_castle: false,
            },
            Self {
                name: "ataxx".to_string(),
                uncolored: false,
                uncolored_symbol: ['x', UNICODE_X],
                player_symbol: [['x', 'X'], ['o', 'O']],
                attacks: vec![
                    AttackKind::drop(vec![EmptySquares, SquareFilter::Neighbor(Box::new(SquareFilter::Us))]),
                    AttackKind {
                        required: RequiredForAttack::PieceOnBoard,
                        condition: Always,
                        attack_mode: AttackMode::All,
                        typ: Leaping(
                            LeapingBitboards::fixed(0, 2, size).combine(
                                LeapingBitboards::fixed(1, 2, size).combine(LeapingBitboards::fixed(2, 2, size)),
                            ),
                        ),
                        bitboard_filter: vec![EmptySquares],
                        kind: Normal,
                        capture_condition: CaptureCondition::Never,
                    },
                ],
                promotions: Promo::none(),
                can_ep_capture: false,
                resets_draw_counter: DrawCtrReset::MoveKind(vec![MoveKind::Drop(0)]),
                royal: false,
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
                royal: false,
                can_castle: false,
            },
        ];
        rest.append(&mut leapers);
        rest.append(&mut riders);
        rest
    }

    pub fn chess_pieces() -> Vec<Piece> {
        let size = FairySize::chess();
        let mut pieces = Self::complete_piece_map(size);
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

    pub fn shatranj_pieces() -> Vec<Piece> {
        let size = FairySize::chess();
        let mut pieces = Self::complete_piece_map(size);
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

    pub fn makruk_pieces() -> Vec<Piece> {
        let size = FairySize::chess();
        let mut pieces = Self::complete_piece_map(size);
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

    pub fn shogi_pieces() -> Vec<Piece> {
        let size = FairySize::shogi();
        let mut pieces = Self::complete_piece_map(size);
        let gold = pieces.remove("gold general").unwrap();
        // cloning precomputed piece attack bitboards uses copy-on-write semantics,
        // so gold general attack bitboards aren't duplicated
        let pawn = pieces.remove("pawn (shogi)").unwrap();
        let mut tokin = pawn.clone();
        tokin.name = "tokin".to_string();
        tokin.set_unicode_symbol('と');
        tokin.attacks = gold.attacks.clone();
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
        let silver = pieces.remove("silver general").unwrap();
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
        bishop = bishop.add_attack(AttackKind::drop(vec![EmptySquares]));
        let mut rook = pieces.remove("rook").unwrap();
        rook.set_unicode_symbol('飛');
        rook = rook.add_attack(AttackKind::drop(vec![EmptySquares]));
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
            pieces.remove("dragon horse").unwrap(),
            pieces.remove("dragon king").unwrap(),
        ];
        const PROMO: usize = 8;
        assert_eq!(res[PROMO].name, "tokin");
        let back_rank = FairyBitboard::rank_for(size.height().get() - 1, size);
        let mut promo_zone = back_rank | back_rank.south();
        promo_zone |= promo_zone.south();
        for i in 0..PROMO - 2 {
            res[i].promotions.pieces = vec![PieceId::new(PROMO + i)];
            res[i].promotions.promoted_version = Some(PieceId::new(PROMO + i));
            res[PROMO + i].promotions.promoted_from = Some(PieceId::new(i));
            res[i].promotions.condition = PromoCondition::SourceOrTargetNoDrop;
            res[i].promotions.optional_promo_zone = SideRelativeBitboard(promo_zone.raw());
            if i == 0 || i == 1 {
                res[i].promotions.forced_promo_zone = SideRelativeBitboard(back_rank.raw());
            } else if i == 2 {
                res[i].promotions.forced_promo_zone = SideRelativeBitboard((back_rank | back_rank.south()).raw());
            }
        }
        res
    }

    pub fn complete_piece_map(size: FairySize) -> HashMap<String, Self> {
        let mut res = HashMap::new();
        for piece in Self::pieces(size) {
            // insertion can fail because some pieces get inserted twice
            _ = res.insert(piece.name.clone(), piece);
        }
        res
    }

    pub fn create_piece_by_name(name: &str, size: FairySize) -> Option<Piece> {
        Self::pieces(size).into_iter().find(|p| p.name == name)
    }
}
