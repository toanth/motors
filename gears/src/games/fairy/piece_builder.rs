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
use crate::games::fairy::attacks::SliderDirections::{Bishop, Forward, Queen, Rook, Vertical};
use crate::games::fairy::attacks::{
    AttackKind, AttackMode, AttackTypes, CaptureCondition, Dir, GenAttackKind, GenAttacksCondition, LeapingBitboards,
    MoveKind, RequiredForAttack, SliderDirections,
};
use crate::games::fairy::piece_builder::AttackBuilder::{Castling, Fixed, Leaping, Rider, Slider};
use crate::games::fairy::pieces::{DrawCtrReset, Piece, PieceId, Promo, PromoCondition};
use crate::games::fairy::rules::PlayerCond::Inactive;
use crate::games::fairy::rules::SquareFilter::{EmptySquares, InDirectionOf, NoSquares, Not, NotUs, RanksRelative};
use crate::games::fairy::rules::{PieceCond, PlayerCond, SquareFilter};
use crate::games::fairy::{FairyColor, FairySize, Side};
use crate::games::{Color, NUM_CHAR_TYPES, NUM_COLORS};
use arbitrary::Arbitrary;
use itertools::Itertools;
use std::cmp::max;
use std::collections::HashMap;

const UNICODE_X: char = '⨉'; // '⨉',
const UNICODE_O: char = '◯'; // '○'

// TODO: If several pieces have the same attacks, only build them once and share the generated bitboards
#[derive(Debug, Clone, Arbitrary)]
pub(super) enum AttackBuilder {
    Leaping { n: usize, m: usize },
    Rider { n: usize, m: usize },
    Slider(SliderDirections),
    Fixed { offsets: Vec<(isize, isize)> },
    Castling(Side),
    Drop,
}

impl AttackBuilder {
    fn range_hv(hor_range: &[isize], vert_range: &[isize]) -> Self {
        Fixed { offsets: hor_range.iter().copied().cartesian_product(vert_range.iter().copied()).collect() }
    }

    fn radius_exact(n: isize) -> Self {
        let v = (-n..=n)
            .into_iter()
            .cartesian_product([-n, n])
            .chain([-n, n].into_iter().cartesian_product(-n + 1..n))
            .collect_vec();
        Fixed { offsets: v }
    }

    #[allow(unused)]
    fn radius_up_to(n: isize) -> Self {
        let offsets = (-n..=n).into_iter().cartesian_product(-n..=n).filter(|&x| x != (0, 0));
        Fixed { offsets: offsets.collect_vec() }
    }
}

#[derive(Debug, Clone, Arbitrary)]
pub(super) struct AttackKindBuilder {
    pub(super) cylinder: bool,
    pub(super) build_col_relative: bool,
    pub(super) typ: AttackBuilder,
    pub(super) required: RequiredForAttack,
    pub(super) condition: GenAttacksCondition,
    pub(super) attack_mode: AttackMode,
    pub(super) bitboard_filter: Vec<SquareFilter>,
    pub(super) kind: GenAttackKind,
    pub(super) capture_condition: CaptureCondition,
}

impl AttackKindBuilder {
    pub fn build(&self, size: FairySize) -> AttackKind {
        let typ = match &self.typ {
            &Leaping { n, m } => AttackTypes::leaping_cylinder(n, m, size, self.cylinder),
            &Rider { n, m } => AttackTypes::rider(n, m, size, self.cylinder),
            Slider(direction) => AttackTypes::Rider(direction.clone()),
            Fixed { offsets } => {
                AttackTypes::Leaping(LeapingBitboards::range(offsets.iter().copied(), size, self.cylinder))
            }
            &Castling(side) => AttackTypes::Castling(side),
            AttackBuilder::Drop => AttackTypes::Drop,
        };
        let flipped = if let Slider(dir) = &self.typ
            && let SliderDirections::Rider { .. } = dir
        {
            panic!("Rider builders should use the 'Rider' variant instead")
        } else if self.build_col_relative
            && let Fixed { offsets } = &self.typ
        {
            let is_symmetrical = offsets.iter().all(|&(d_file, d_rank)| offsets.contains(&(d_file, -d_rank)));
            if is_symmetrical {
                typ.clone() // only use a single copy of the precomputed bitboards
            } else {
                let flipped_offsets = offsets.iter().map(|&(d_file, d_rank)| (d_file, -d_rank)).collect_vec();
                AttackTypes::Leaping(LeapingBitboards::range(flipped_offsets.iter().copied(), size, self.cylinder))
            }
        } else {
            typ.clone()
        };
        let typ = [typ, flipped];
        AttackKind {
            required: self.required,
            condition: self.condition,
            attack_mode: self.attack_mode,
            typ,
            bitboard_filter: self.bitboard_filter.clone(),
            kind: self.kind,
            capture_condition: self.capture_condition,
        }
    }

    fn simple(typ: AttackBuilder) -> Self {
        Self {
            cylinder: false,
            required: RequiredForAttack::PieceOnBoard,
            typ,
            condition: Always,
            bitboard_filter: vec![NotUs],
            kind: Normal,
            attack_mode: AttackMode::All,
            capture_condition: CaptureCondition::DestOccupied,
            build_col_relative: true,
        }
    }

    pub fn pawn_noncapture(typ: AttackBuilder) -> Self {
        Self {
            cylinder: false,
            build_col_relative: true,
            required: RequiredForAttack::PieceOnBoard,
            typ,
            condition: Always,
            bitboard_filter: vec![EmptySquares],
            kind: Normal,
            attack_mode: AttackMode::NoCaptures,
            capture_condition: CaptureCondition::Never,
        }
    }

    pub fn pawn_capture(typ: AttackBuilder, bb_filter: SquareFilter) -> Self {
        Self {
            cylinder: false,
            build_col_relative: true,
            required: RequiredForAttack::PieceOnBoard,
            typ,
            condition: Always,
            bitboard_filter: vec![bb_filter],
            kind: Normal,
            attack_mode: AttackMode::Captures,
            capture_condition: CaptureCondition::Always,
        }
    }
    pub fn drop(filter: Vec<SquareFilter>) -> Self {
        Self {
            cylinder: false,
            build_col_relative: false,
            required: RequiredForAttack::PieceInHand,
            condition: Always,
            attack_mode: AttackMode::NoCaptures,
            typ: AttackBuilder::Drop,
            bitboard_filter: filter,
            kind: GenAttackKind::Drop,
            capture_condition: CaptureCondition::Never,
        }
    }

    pub fn side_relative(mut self) -> Self {
        self.build_col_relative = true;
        self
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
    pub(super) royal: bool,
    /// The move output (compact and SAN) can omit the piece type. This is true for generalized pawns, but also mnk pieces.
    pub(super) output_omit_piece: bool,
    /// true for kings but not for rooks
    pub(super) can_castle: bool,
}

// TODO: Remove?
// pub(super) const PAWN_IDX: usize = 0;
// #[allow(unused)]
// pub(super) const CHESS_KNIGHT_IDX: usize = 1;
// #[allow(unused)]
// pub(super) const CHESS_BISHOP_IDX: usize = 2;
// #[allow(unused)]
// pub(super) const CHESS_ROOK_IDX: usize = 3;
// #[allow(unused)]
// pub(super) const CHESS_QUEEN_IDX: usize = 4;
// #[allow(unused)]
// pub(super) const CHESS_KING_IDX: usize = 5;

impl PieceBuilder {
    pub fn build(&self, size: FairySize) -> Piece {
        let attacks = self.attacks.iter().map(|a| a.build(size)).collect_vec();
        Piece {
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
        self.player_symbol[FairyColor::first()][Unicode] = symbol;
        self.player_symbol[FairyColor::second()][Unicode] = symbol;
    }

    pub fn set_ascii_symbol(&mut self, symbol: char) {
        self.uncolored_symbol[Ascii] = symbol;
        self.player_symbol[FairyColor::first()][Ascii] = symbol.to_ascii_uppercase();
        self.player_symbol[FairyColor::second()][Ascii] = symbol.to_ascii_lowercase();
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
            royal: false,
            output_omit_piece: false,
            can_castle: false,
        }
    }

    pub fn new(name: &str, attacks: Vec<AttackBuilder>, ascii_char: char, unicode_chars: Option<[char; 3]>) -> Self {
        let attacks = attacks.into_iter().map(AttackKindBuilder::simple).collect_vec();
        Self::new_for(name, attacks, ascii_char, unicode_chars)
    }

    pub fn leaper(name: &str, n: usize, m: usize, ascii_char: Option<char>, unicode: Option<[char; 3]>) -> Self {
        let ascii = ascii_char.unwrap_or(name.chars().next().unwrap());
        Self::new(name, vec![Leaping { n, m }], ascii, unicode)
    }

    fn chess_pawn_no_promo() -> Self {
        let single_push = AttackKindBuilder::pawn_noncapture(AttackBuilder::range_hv(&[0], &[1]));
        // let normal_black =
        //     AttackKindBuilder::pawn_noncapture(AttackBuilder::range_hv(&[0], &[-1]), Player(FairyColor::second()));
        let capture =
            AttackKindBuilder::pawn_capture(AttackBuilder::range_hv(&[-1, 1], &[1]), SquareFilter::PawnCapture);
        // let black_capture = AttackKindBuilder::pawn_capture(
        //     AttackBuilder::range_hv(&[-1, 1], &[-1]),
        //     Player(FairyColor::second()),
        //     SquareFilter::PawnCapture,
        // );
        // promotions are handled as effects instead of duplicating all normal and capture moves
        let white_double = AttackKindBuilder {
            cylinder: false,
            build_col_relative: false,
            required: RequiredForAttack::PieceOnBoard,
            typ: Slider(Vertical),
            condition: OnRelativeRank(1, FairyColor::first()),
            bitboard_filter: vec![EmptySquares, SquareFilter::Rank(3)],
            kind: DoublePawnPush,
            attack_mode: AttackMode::NoCaptures,
            capture_condition: CaptureCondition::Never,
        };
        let black_double = AttackKindBuilder {
            cylinder: false,
            build_col_relative: false,
            required: RequiredForAttack::PieceOnBoard,
            typ: Slider(Vertical),
            condition: OnRelativeRank(1, FairyColor::second()),
            bitboard_filter: vec![EmptySquares, RanksRelative(vec![3], PlayerCond::Second)],
            kind: DoublePawnPush,
            attack_mode: AttackMode::NoCaptures,
            capture_condition: CaptureCondition::Never,
        };
        let mut res = Self::pawn_shatranj_no_promo();
        res.name = "pawn".to_string();
        res.attacks = vec![single_push, capture, white_double, black_double];
        res.can_ep_capture = true;
        res
    }

    // like the chess pawn, but without double pawn push and ep
    fn pawn_shatranj_no_promo() -> Self {
        let normal_white = AttackKindBuilder::pawn_noncapture(AttackBuilder::range_hv(&[0], &[1]));
        // let normal_black =
        //     AttackKindBuilder::pawn_noncapture(AttackBuilder::range_hv(&[0], &[-1]));
        let white_capture =
            AttackKindBuilder::pawn_capture(AttackBuilder::range_hv(&[-1, 1], &[1]), SquareFilter::Them);
        // let black_capture = AttackKindBuilder::pawn_capture(
        //     AttackBuilder::range_hv(&[-1, 1], &[-1]),
        //     Player(FairyColor::second()),
        //     SquareFilter::Them,
        // );
        Self {
            name: "pawn (shatranj)".to_string(),
            uncolored: false,
            uncolored_symbol: ['P', UNICODE_NEUTRAL_PAWN],
            player_symbol: [['P', UNICODE_WHITE_PAWN], ['p', UNICODE_BLACK_PAWN]],

            attacks: vec![normal_white, white_capture],
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
        let mut offsets = [-1, 0, 1].into_iter().cartesian_product([1]).collect_vec();
        offsets.append(&mut ([-1, 1].into_iter().cartesian_product([-1]).collect_vec()));
        Self::new("silver general", vec![(Fixed { offsets })], 's', Some(['銀', '銀', '銀']))
    }

    fn gold_no_drop() -> Self {
        let offsets = vec![(0, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];
        Self::new("gold general", vec![(Fixed { offsets })], 'g', Some(['金', '金', '金']))
    }

    fn bishop() -> Self {
        Self::new(
            "bishop",
            vec![Slider(Bishop)],
            'b',
            Some([UNICODE_WHITE_BISHOP, UNICODE_BLACK_BISHOP, UNICODE_NEUTRAL_BISHOP]),
        )
    }

    fn rook() -> Self {
        Self::new("rook", vec![Slider(Rook)], 'r', Some([UNICODE_WHITE_ROOK, UNICODE_BLACK_ROOK, UNICODE_NEUTRAL_ROOK]))
    }

    fn queen() -> Self {
        Self::new(
            "queen",
            vec![Slider(Queen)],
            'q',
            Some([UNICODE_WHITE_QUEEN, UNICODE_BLACK_QUEEN, UNICODE_NEUTRAL_QUEEN]),
        )
    }

    fn king_shatranj() -> Self {
        let mut res = Self::new(
            "king (shatranj)",
            vec![AttackBuilder::radius_exact(1)],
            'k',
            Some([UNICODE_WHITE_KING, UNICODE_BLACK_KING, UNICODE_NEUTRAL_KING]),
        );
        res.royal = true;
        res
    }

    pub fn pieces() -> Vec<Self> {
        // let not_their_rank =
        //     move |rank: DimT| (!FairyBitboard::rank_for(size.height.get().saturating_sub(1 + rank), size)).raw();
        // order of leapers matters
        let mut leapers = vec![
            Self::leaper("wazir", 0, 1, None, Some(['🨠', '🨦', '🨬'])),
            Self::ferz(),
            Self::leaper("dabbaba", 0, 2, None, None),
            Self::knight(),
            Self::leaper("alfil", 2, 2, None, Some(['\u{1FA55}', '\u{1FA57}', '\u{1FA55}'])),
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
            let attacks = vec![AttackBuilder::Rider { n, m }];
            let name = leaper.name.clone() + "rider";
            let rider = Self::new(&name, attacks, name.chars().next().unwrap(), None);
            riders.push(rider);
        }
        riders[3].name = "nightrider".to_string();
        let mut rest = vec![
            {
                let castle_king_side = AttackKindBuilder {
                    cylinder: false,
                    build_col_relative: false,
                    required: RequiredForAttack::PieceOnBoard,
                    condition: CanCastle(Kingside),
                    attack_mode: AttackMode::NoCaptures,
                    typ: Castling(Kingside),
                    bitboard_filter: vec![],
                    kind: Castle(Kingside),
                    capture_condition: CaptureCondition::Never,
                };
                let castle_queen_side = AttackKindBuilder {
                    cylinder: false,
                    build_col_relative: false,
                    required: RequiredForAttack::PieceOnBoard,
                    condition: CanCastle(Queenside),
                    attack_mode: AttackMode::NoCaptures,
                    typ: Castling(Queenside),
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
                    cylinder: false,
                    build_col_relative: false,
                    required: RequiredForAttack::PieceOnBoard,
                    typ: Slider(Vertical),
                    condition: OnRelativeRank(0, FairyColor::first()),
                    bitboard_filter: vec![EmptySquares, SquareFilter::Rank(2)],
                    kind: Normal,
                    attack_mode: AttackMode::NoCaptures,
                    capture_condition: CaptureCondition::Never,
                });
                res.attacks.push(AttackKindBuilder {
                    cylinder: false,
                    build_col_relative: false,
                    required: RequiredForAttack::PieceOnBoard,
                    typ: Slider(Vertical),
                    condition: OnRelativeRank(0, FairyColor::second()),
                    bitboard_filter: vec![EmptySquares, RanksRelative(vec![2], PlayerCond::Second)],
                    kind: Normal,
                    attack_mode: AttackMode::NoCaptures,
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
                let mut khon = Self::ferz();
                khon.name = "khon".to_string();
                khon.uncolored_symbol = ['S', 'S'];
                khon.player_symbol = [['S', 'S'], ['s', 's']];
                khon
            },
            {
                let mut met = Self::silver_no_drop();
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
                shogi_pawn.name = "pawn (shogi)".to_string();
                shogi_pawn.uncolored_symbol[Unicode] = '歩';
                shogi_pawn.player_symbol[FairyColor::first()][Unicode] = '歩';
                shogi_pawn.player_symbol[FairyColor::second()][Unicode] = '歩';
                let white_attacks = AttackBuilder::range_hv(&[0], &[1]);
                // let black_attacks = AttackBuilder::range_hv(&[0], &[-1]);
                shogi_pawn.attacks = vec![
                    AttackKindBuilder::simple(white_attacks),
                    // AttackKindBuilder::simple(black_attacks).with_cond(Player(FairyColor::second())),
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
                    AttackKindBuilder::simple(AttackBuilder::range_hv(&[-1, 1], &[2])).side_relative(),
                    AttackKindBuilder::drop(vec![EmptySquares, Not(Box::new(RanksRelative(vec![0, 1], Inactive)))]),
                ],
                'n',
                Some(['桂', '桂', '桂']),
            ),
            Self::new_for(
                "lance",
                vec![
                    AttackKindBuilder::simple(Slider(Forward)),
                    AttackKindBuilder::drop(vec![EmptySquares, Not(Box::new(RanksRelative(vec![0], Inactive)))]),
                ],
                'l',
                Some(['香', '香', '香']),
            ),
            Self::new(
                "dragon king",
                vec![Slider(Rook), AttackBuilder::range_hv(&[-1, 1], &[-1, 1])],
                'd',
                Some(['龍', '龍', '龍']),
            ),
            Self::new("dragon horse", vec![Slider(Bishop), Leaping { n: 0, m: 1 }], 'h', Some(['馬', '馬', '馬'])),
            Self::new("go-between", vec![Fixed { offsets: vec![(0, -1), (0, 1)] }], 'g', None),
            // compound pieces
            Self::new("archbishop", vec![Leaping { n: 1, m: 2 }, Slider(Bishop)], 'a', Some(['🩐', '🩓', '🩐'])),
            Self::new("chancellor", vec![Leaping { n: 1, m: 2 }, Slider(Rook)], 'c', Some(['🩏', '🩒', '🩏'])),
            Self::new("amazon", vec![Leaping { n: 1, m: 2 }, Slider(Queen)], 'a', Some(['🩎', '🩑', '🩎'])),
            Self::new("kirin", vec![Leaping { n: 1, m: 1 }, Leaping { n: 0, m: 2 }], 'f', None),
            Self::new("frog", vec![Leaping { n: 1, m: 1 }, Leaping { n: 0, m: 3 }], 'f', None),
            Self::new("gnu", vec![Leaping { n: 1, m: 2 }, Leaping { n: 1, m: 3 }], 'g', None),
            Self {
                name: "mnk".to_string(),
                uncolored: false,
                uncolored_symbol: ['x', UNICODE_X],
                player_symbol: [['X', UNICODE_X], ['O', UNICODE_O]],
                attacks: vec![AttackKindBuilder::drop(vec![EmptySquares])],
                promotions: Promo::none(),
                can_ep_capture: false,
                resets_draw_counter: DrawCtrReset::Never,
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
                        cylinder: false,
                        required: RequiredForAttack::PieceOnBoard,
                        condition: Always,
                        attack_mode: AttackMode::All,
                        typ: AttackBuilder::radius_exact(2),
                        bitboard_filter: vec![EmptySquares],
                        kind: Normal,
                        capture_condition: CaptureCondition::Never,
                        build_col_relative: false,
                    },
                ],
                promotions: Promo::none(),
                can_ep_capture: false,
                resets_draw_counter: DrawCtrReset::MoveKind(vec![MoveKind::Drop(0)]),
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
        let gold = pieces.remove("gold general").unwrap();
        // cloning precomputed piece attack bitboards uses copy-on-write semantics,
        // so gold general attack bitboards aren't duplicated
        let pawn = pieces.remove("pawn (shogi)").unwrap();
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
            pieces.remove("dragon horse").unwrap(),
            pieces.remove("dragon king").unwrap(),
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
