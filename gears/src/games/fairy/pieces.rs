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
use crate::games::fairy::attacks::{AttackKind, MoveKind};
use crate::games::fairy::moves::Move;
use crate::games::fairy::rules::SquareFilter::NoSquares;
use crate::games::fairy::rules::{PromoFenModifier, Rules, SquareFilter};
use crate::games::fairy::{Bitboard, Board, Color, Square};
use crate::games::{
    AbstractPieceType, CharType, ColorTrait, ColoredPieceTypeTrait, NUM_CHAR_TYPES, NUM_COLORS, PieceTypeTrait,
};
use crate::general::bitboards::BitboardTrait;
use crate::general::board::BoardTrait;
use crate::general::common::Res;
use anyhow::bail;
use arbitrary::Arbitrary;
use colored::Colorize;
use std::fmt;
use std::fmt::Formatter;

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
    pub fn get(self, rules: &Rules) -> Option<&Piece> {
        rules.pieces.get(self.val())
    }
}

impl AbstractPieceType<Board> for PieceId {
    fn empty() -> Self {
        Self(u8::MAX)
    }

    fn non_empty(settings: &Rules) -> impl Iterator<Item = Self> {
        (0..settings.pieces.len()).map(Self::new)
    }

    fn to_char(self, typ: CharType, rules: &Rules) -> char {
        if let Some(p) = self.get(rules) { p.uncolored_symbol[typ as usize] } else { ' ' }
    }

    fn from_char(c: char, rules: &Rules) -> Option<Self> {
        rules.matching_piece_ids(|p| p.uncolored_symbol.contains(&c)).next()
    }

    #[allow(refining_impl_trait)]
    fn name(&self, settings: &Rules) -> String {
        if *self == Self::empty() {
            return "<No piece>".to_string();
        }
        self.get(settings).unwrap().name.clone()
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
                    self = unpromoted;
                    write!(f, "+{}", to_c(self))
                }
            }
        } else {
            write!(f, "{}", to_c(self))
        }
    }

    fn max_num_chars(settings: &Rules) -> usize {
        if settings.pieces.iter().any(|p| p.promotions.promoted_from.is_some()) { 2 } else { 1 }
    }

    fn to_uncolored_idx(self) -> usize {
        self.val()
    }

    fn make_promoted(&mut self, rules: &Rules) -> Res<()> {
        let Some(promoted) = self.get(rules).expect("can't promote empty piece").promotions.promoted_version else {
            bail!(
                "The piece '{0}' can't be marked as having been promoted. Current variant: {1}",
                self.name(rules).bold(),
                rules.name.bold()
            )
        };
        *self = promoted;
        Ok(())
    }
}

impl PieceTypeTrait<Board> for PieceId {
    type Colored = ColoredPieceId;

    fn from_idx(idx: usize) -> Self {
        Self::new(idx)
    }

    fn promoted_from(&self, rules: &Rules) -> Option<PieceId> {
        self.get(rules).expect("empty piece isn't promoted").promotions.promoted_from
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub struct ColoredPieceId {
    id: PieceId,
    color: Option<Color>,
}

impl ColoredPieceId {
    pub fn as_u8(&self) -> u8 {
        self.id.0 * 3 + self.color.map_or(0, |c| c.0 as u8 + 1)
    }
    pub fn val(self) -> usize {
        self.as_u8() as usize
    }
    pub fn from_u8(val: u8) -> Self {
        let id = PieceId(val / 3);
        let color = match val % 3 {
            0 => None,
            c => Some(Color::from_idx(c as usize - 1)),
        };
        ColoredPieceId { id, color }
    }
    pub fn create(piece: PieceId, color: Option<Color>) -> Self {
        ColoredPieceId { id: piece, color }
    }
}

impl AbstractPieceType<Board> for ColoredPieceId {
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
                        Some(Self { id: PieceId::new(idx), color: Some(Color::first()) }),
                        Some(Self { id: PieceId::new(idx), color: Some(Color::second()) }),
                    ]
                    .into_iter()
                }
            })
            .flatten()
    }

    fn to_char(self, typ: CharType, rules: &Rules) -> char {
        let Some(piece) = self.id.get(rules) else { return '.' };
        if let Some(color) = self.color {
            piece.player_symbol[color][typ as usize]
        } else {
            piece.uncolored_symbol[typ as usize]
        }
    }

    fn from_char(c: char, rules: &Rules) -> Option<Self> {
        let found = rules.pieces().find(|(_id, p)| p.player_symbol.iter().any(|s| s.contains(&c)));
        if let Some((id, p)) = found {
            if p.player_symbol[CharType::Ascii].contains(&c) {
                Some(Self { id, color: Some(Color::first()) })
            } else {
                Some(Self { id, color: Some(Color::second()) })
            }
        } else {
            rules.matching_piece_ids(|p| p.uncolored_symbol.contains(&c)).next().map(|id| Self { id, color: None })
        }
    }

    fn max_num_chars(settings: &Rules) -> usize {
        PieceId::max_num_chars(settings)
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
        if let Some(unpromoted) = self.id.promoted_from(rules) {
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

    fn name(&self, settings: &Rules) -> impl AsRef<str> {
        if let Some(color) = self.color {
            format!("{0} {1}", color.name(settings), self.id.name(settings))
        } else {
            self.id.name(settings)
        }
    }

    fn to_uncolored_idx(self) -> usize {
        self.id.val()
    }

    fn make_promoted(&mut self, rules: &Rules) -> Res<()> {
        self.id.make_promoted(rules)
    }
}

impl ColoredPieceTypeTrait<Board> for ColoredPieceId {
    type Uncolored = PieceId;

    fn new(color: Color, uncolored: Self::Uncolored) -> Self {
        Self { id: uncolored, color: Some(color) }
    }

    fn color(self) -> Option<Color> {
        self.color
    }

    fn uncolor(self) -> Self::Uncolored {
        self.id
    }

    fn to_colored_idx(self) -> usize {
        self.id.val()
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

    fn gen_promo_impl<F: Fn(Bitboard) -> bool>(&self, contained: F, pos: &Board) -> GenPromoMoves {
        if contained(self.forced_promo_zone.bb(pos.active_player(), pos)) {
            GenPromoMoves::ForcedPromo
        } else if contained(self.optional_promo_zone.bb(pos.active_player(), pos)) {
            GenPromoMoves::OptionalPromo
        } else {
            GenPromoMoves::NoPromo
        }
    }

    pub fn gen_promo(&self, source: Square, dest: Square, pos: &Board) -> GenPromoMoves {
        match self.condition {
            PromoCondition::Never => GenPromoMoves::NoPromo,
            PromoCondition::TargetSquare => self.gen_promo_impl(|bb| bb.has(dest), pos),
            PromoCondition::SourceOrTargetNoDrop => {
                let promo = |bb: Bitboard| source != Square::no_coordinates() && (bb.has(dest) || bb.has(source));
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
    pub fn reset(&self, mov: Move) -> bool {
        match self {
            DrawCtrReset::Always => true,
            DrawCtrReset::Never => false,
            DrawCtrReset::MoveKind(vec) => vec.contains(&mov.kind()),
        }
    }
}

pub(super) const PAWN_IDX: usize = 0;
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
    // The move output (compact and SAN) can omit the piece type. This is true for generalized pawns, but also mnk pieces.
    pub(super) output_omit_piece: bool,
    // true for kings but not for rooks
    pub(super) can_castle: bool,
}

impl Piece {
    pub fn new_for(name: &str, attacks: Vec<AttackKind>, ascii_char: char, unicode_chars: Option<[char; 3]>) -> Self {
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
            royal: false,
            output_omit_piece: false,
            can_castle: false,
        }
    }
}
