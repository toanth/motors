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
use crate::games::chess::pieces::{
    NUM_COLORS, UNICODE_BLACK_BISHOP, UNICODE_BLACK_KING, UNICODE_BLACK_KNIGHT, UNICODE_BLACK_PAWN,
    UNICODE_BLACK_QUEEN, UNICODE_BLACK_ROOK, UNICODE_NEUTRAL_BISHOP, UNICODE_NEUTRAL_KING, UNICODE_NEUTRAL_KNIGHT,
    UNICODE_NEUTRAL_PAWN, UNICODE_NEUTRAL_QUEEN, UNICODE_NEUTRAL_ROOK, UNICODE_WHITE_BISHOP, UNICODE_WHITE_KING,
    UNICODE_WHITE_KNIGHT, UNICODE_WHITE_PAWN, UNICODE_WHITE_QUEEN, UNICODE_WHITE_ROOK,
};
use crate::games::fairy::attacks::AttackKind::*;
use crate::games::fairy::attacks::AttackTypes::*;
use crate::games::fairy::attacks::GenAttacksCondition::*;
use crate::games::fairy::attacks::{
    AttackBitboardFilter, AttackMode, AttackTypes, CaptureCondition, GenPieceAttackKind, LeapingBitboards,
    RequiredForAttack, SliderDirections,
};
use crate::games::fairy::rules::RulesRef;
use crate::games::fairy::Side::*;
use crate::games::fairy::{FairyBitboard, FairyBoard, FairyColor, FairySize, RawFairyBitboard};
use crate::games::{AbstractPieceType, CharType, Color, ColoredPieceType, Height, PieceType, Width};
use crate::general::bitboards::Bitboard;
use crate::general::squares::RectangularSize;
use arbitrary::Arbitrary;
use itertools::Itertools;
use std::cmp::max;
use std::collections::HashMap;
use std::iter::once;

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
}

impl AbstractPieceType<FairyBoard> for PieceId {
    fn empty() -> Self {
        Self(u8::MAX)
    }

    fn to_char(self, typ: CharType, rules: &RulesRef) -> char {
        rules.0.pieces[self.val()].uncolored_symbol[typ as usize]
    }

    fn from_char(c: char, rules: &RulesRef) -> Option<Self> {
        rules.0.matching_piece_ids(|p| p.uncolored_symbol.contains(&c)).next()
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
//
// impl Display for ColoredPieceId {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         let color = self
//             .color
//             .map(|c| rules().colors[c.idx()].name.clone())
//             .unwrap_or_default();
//         write!(
//             f,
//             "{color} {0}",
//             rules().pieces[self.id.val()].uncolored_symbol[CharType::Ascii as usize],
//         )
//     }
// }

impl AbstractPieceType<FairyBoard> for ColoredPieceId {
    fn empty() -> Self {
        Self { id: PieceId::empty(), color: None }
    }

    fn to_char(self, typ: CharType, rules: &RulesRef) -> char {
        if let Some(color) = self.color {
            rules.0.pieces[self.id.val()].player_symbol[color.idx()][typ as usize]
        } else {
            rules.0.pieces[self.id.val()].uncolored_symbol[typ as usize]
        }
    }

    fn from_char(c: char, rules: &RulesRef) -> Option<Self> {
        let found = rules.0.pieces().find(|(_id, p)| p.player_symbol.iter().any(|s| s.contains(&c)));
        if let Some((id, p)) = found {
            if p.player_symbol[0].contains(&c) {
                Some(Self { id, color: Some(FairyColor::first()) })
            } else {
                Some(Self { id, color: Some(FairyColor::second()) })
            }
        } else {
            rules.0.matching_piece_ids(|p| p.uncolored_symbol.contains(&c)).next().map(|id| Self { id, color: None })
        }
    }

    fn to_uncolored_idx(self) -> usize {
        self.id.val()
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
        self.id.val()
    }

    fn new(color: FairyColor, uncolored: Self::Uncolored) -> Self {
        Self { id: uncolored, color: Some(color) }
    }
}

#[derive(Debug, Default, Arbitrary)]
#[must_use]
pub(super) struct Promo {
    pub pieces: Vec<PieceId>,
    pub squares: RawFairyBitboard,
}

impl Promo {
    pub fn none() -> Self {
        Self::default()
    }
}

/// This struct defines the rules for a single piece.
#[derive(Debug, Arbitrary)]
pub struct Piece {
    pub(super) name: String,
    pub(super) uncolored_symbol: [char; 2],
    pub(super) player_symbol: [[char; 2]; NUM_COLORS],
    // Most of the attack data is represented with a bitboard.
    // To distinguish between different special moves, the `GenPieceAttackKind` struct has an `AttackKind` field.
    pub(super) attacks: Vec<GenPieceAttackKind>,
    /// Promotions change the piece type and can differentiate moves with otherwise identical information.
    /// However, they are not the only way to change piece types; this can also be done through move effects based on the move kind.
    pub(super) promotions: Promo,
    pub(super) can_ep_capture: bool,
    pub(super) reset_draw_counter: bool,
    pub(super) royal: bool,
    // true for kings but not for rooks
    pub(super) can_castle: bool,
}

impl Piece {
    pub fn new(name: &str, attacks: Vec<AttackTypes>, ascii_char: char, unicode_chars: Option<[char; 3]>) -> Self {
        let lowercase_ascii = ascii_char.to_ascii_lowercase();
        let uppercase_ascii = ascii_char.to_ascii_uppercase();
        let [u_white, u_black, u_uncolored] = if let Some(unicode) = unicode_chars {
            unicode
        } else {
            [uppercase_ascii, lowercase_ascii, uppercase_ascii]
        };
        let attacks = attacks.into_iter().map(GenPieceAttackKind::simple).collect_vec();
        Self {
            name: name.to_string(),
            uncolored_symbol: [uppercase_ascii, u_uncolored],
            player_symbol: [[uppercase_ascii, u_white], [lowercase_ascii, u_black]],
            attacks,
            promotions: Promo::none(),
            can_ep_capture: false,
            reset_draw_counter: false,
            royal: false,
            can_castle: false,
        }
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
    pub fn pieces(size: FairySize) -> Vec<Self> {
        // order of leapers matters
        let mut leapers = vec![
            Self::leaper("wazir", 0, 1, size, None, Some(['🨠', '🨦', '🨬'])),
            Self::leaper("ferz", 1, 1, size, None, Some(['\u{1FA54}', '\u{1FA56}', '\u{1FA55}'])),
            Self::leaper("dabbaba", 0, 2, size, None, None),
            Self::leaper(
                "knight",
                1,
                2,
                size,
                Some('n'),
                Some([UNICODE_WHITE_KNIGHT, UNICODE_BLACK_KNIGHT, UNICODE_NEUTRAL_KNIGHT]),
            ),
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
                let castle_king_side = GenPieceAttackKind {
                    required: RequiredForAttack::PieceOnBoard,
                    condition: CanCastle(Kingside),
                    attack_mode: AttackMode::NoCaptures,
                    typ: Castling(Kingside),
                    bitboard_filter: vec![],
                    kind: Castle(Kingside),
                    capture_condition: CaptureCondition::Never,
                };
                let castle_queen_side = GenPieceAttackKind {
                    required: RequiredForAttack::PieceOnBoard,
                    condition: CanCastle(Queenside),
                    attack_mode: AttackMode::NoCaptures,
                    typ: Castling(Queenside),
                    bitboard_filter: vec![],
                    kind: Castle(Queenside),
                    capture_condition: CaptureCondition::Never,
                };
                Self {
                    name: "king".to_string(),
                    uncolored_symbol: ['K', UNICODE_NEUTRAL_KING],
                    player_symbol: [['K', UNICODE_WHITE_KING], ['k', UNICODE_BLACK_KING]],
                    attacks: vec![
                        GenPieceAttackKind::simple(Leaping(
                            LeapingBitboards::fixed(1, 1, size).combine(LeapingBitboards::fixed(1, 0, size)),
                        )),
                        castle_king_side,
                        castle_queen_side,
                    ],
                    promotions: Promo::none(),
                    can_ep_capture: false,
                    reset_draw_counter: false,
                    royal: true,
                    can_castle: true,
                }
            },
            Self::new(
                "queen",
                vec![Rider(SliderDirections::Queen)],
                'q',
                Some([UNICODE_WHITE_QUEEN, UNICODE_BLACK_QUEEN, UNICODE_NEUTRAL_QUEEN]),
            ),
            Self::new(
                "rook",
                vec![Rider(SliderDirections::Rook)],
                'r',
                Some([UNICODE_WHITE_ROOK, UNICODE_BLACK_ROOK, UNICODE_NEUTRAL_ROOK]),
            ),
            Self::new(
                "bishop",
                vec![Rider(SliderDirections::Bishop)],
                'b',
                Some([UNICODE_WHITE_BISHOP, UNICODE_BLACK_BISHOP, UNICODE_NEUTRAL_BISHOP]),
            ),
            {
                let normal_white = GenPieceAttackKind::pawn_noncapture(
                    Leaping(LeapingBitboards::range(once(0), once(1), size)),
                    Player(FairyColor::first()),
                );
                let normal_black = GenPieceAttackKind::pawn_noncapture(
                    Leaping(LeapingBitboards::range(once(0), once(-1), size)),
                    Player(FairyColor::second()),
                );
                let white_capture = GenPieceAttackKind::pawn_capture(
                    Leaping(LeapingBitboards::range([-1, 1].into_iter(), once(1), size)),
                    Player(FairyColor::first()),
                    AttackBitboardFilter::PawnCapture,
                );
                let black_capture = GenPieceAttackKind::pawn_capture(
                    Leaping(LeapingBitboards::range([-1, 1].into_iter(), once(-1), size)),
                    Player(FairyColor::second()),
                    AttackBitboardFilter::PawnCapture,
                );
                // promotions are handled as effects instead of duplicating all normal and capture moves
                let white_double = GenPieceAttackKind {
                    required: RequiredForAttack::PieceOnBoard,
                    typ: Rider(SliderDirections::Vertical),
                    condition: OnRank(1, FairyColor::first()),
                    bitboard_filter: vec![AttackBitboardFilter::EmptySquares, AttackBitboardFilter::Rank(3)],
                    kind: DoublePawnPush,
                    attack_mode: AttackMode::NoCaptures,
                    capture_condition: CaptureCondition::Never,
                };
                let black_double = GenPieceAttackKind {
                    required: RequiredForAttack::PieceOnBoard,
                    typ: Rider(SliderDirections::Vertical),
                    condition: OnRank(size.height().get().saturating_sub(2), FairyColor::second()),
                    bitboard_filter: vec![
                        AttackBitboardFilter::EmptySquares,
                        AttackBitboardFilter::Rank(size.height().get().saturating_sub(4)),
                    ],
                    kind: DoublePawnPush,
                    attack_mode: AttackMode::NoCaptures,
                    capture_condition: CaptureCondition::Never,
                };
                Self {
                    name: "pawn".to_string(),
                    uncolored_symbol: ['p', UNICODE_NEUTRAL_PAWN],
                    player_symbol: [['P', UNICODE_WHITE_PAWN], ['p', UNICODE_BLACK_PAWN]],

                    attacks: vec![normal_white, normal_black, white_double, black_double, white_capture, black_capture],
                    // the promotion pieces are set later, once it's known which pieces are available
                    promotions: Promo { pieces: vec![], squares: FairyBitboard::backranks_for(size).raw() },
                    can_ep_capture: true,
                    reset_draw_counter: true,
                    royal: false,
                    can_castle: false,
                }
            },
            {
                let mut res = Self::new(
                    "king (shatranj)",
                    vec![Leaping(LeapingBitboards::fixed(1, 1, size).combine(LeapingBitboards::fixed(0, 1, size)))],
                    'k',
                    Some([UNICODE_WHITE_KING, UNICODE_BLACK_KING, UNICODE_NEUTRAL_KING]),
                );
                res.royal = true;
                res
            },
            // like the chess pawn, but without double pawn push and ep
            {
                let normal_white = GenPieceAttackKind::pawn_noncapture(
                    Leaping(LeapingBitboards::range(once(0), once(1), size)),
                    Player(FairyColor::first()),
                );
                let normal_black = GenPieceAttackKind::pawn_noncapture(
                    Leaping(LeapingBitboards::range(once(0), once(-1), size)),
                    Player(FairyColor::second()),
                );
                let white_capture = GenPieceAttackKind::pawn_capture(
                    Leaping(LeapingBitboards::range([-1, 1].into_iter(), once(1), size)),
                    Player(FairyColor::first()),
                    AttackBitboardFilter::Them,
                );
                let black_capture = GenPieceAttackKind::pawn_capture(
                    Leaping(LeapingBitboards::range([-1, 1].into_iter(), once(-1), size)),
                    Player(FairyColor::second()),
                    AttackBitboardFilter::Them,
                );
                Self {
                    name: "pawn (shatranj)".to_string(),
                    uncolored_symbol: ['p', UNICODE_NEUTRAL_PAWN],
                    player_symbol: [['P', UNICODE_WHITE_PAWN], ['p', UNICODE_BLACK_PAWN]],

                    attacks: vec![normal_white, normal_black, white_capture, black_capture],
                    // the promotion pieces are set later, once it's known which pieces are available
                    promotions: Promo { pieces: vec![], squares: FairyBitboard::backranks_for(size).raw() },
                    can_ep_capture: false,
                    reset_draw_counter: true,
                    royal: false,
                    can_castle: false,
                }
            },
            Self::new(
                "pawn (shogi)",
                vec![Leaping(LeapingBitboards::range(once(0), once(1), size))],
                'p',
                Some(['歩', '歩', '歩']),
            ),
            Self::new(
                "gold general",
                vec![Leaping(
                    LeapingBitboards::fixed(0, 1, size)
                        .combine(LeapingBitboards::fixed(1, 1, size))
                        .remove(LeapingBitboards::range(once(-1), [-1, 1].into_iter(), size)),
                )],
                'g',
                Some(['金', '金', '金']),
            ),
            Self::new(
                "silver general",
                vec![Leaping(LeapingBitboards::range(once(1), -1..=1, size).combine(LeapingBitboards::range(
                    once(-1),
                    [-1, 1].into_iter(),
                    size,
                )))],
                's',
                Some(['銀', '銀', '銀']),
            ),
            Self::new(
                "knight (shogi)",
                vec![Leaping(LeapingBitboards::range(once(2), -1..=1, size))],
                'n',
                Some(['桂', '桂', '桂']),
            ),
            Self::new("lance", vec![Rider(SliderDirections::Vertical)], 'l', Some(['香', '香', '香'])),
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
                vec![Leaping(LeapingBitboards::range(once(0), [-1, 1].into_iter(), size))],
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
            Self::new("gnu", vec![AttackTypes::leaping(1, 2, size), AttackTypes::leaping(1, 3, size)], 'f', None),
            {
                const UNICODE_X: char = '⨉'; // '⨉',
                const UNICODE_O: char = '◯'; // '○'
                Self {
                    name: "mnk".to_string(),
                    uncolored_symbol: ['x', UNICODE_X],
                    player_symbol: [['X', UNICODE_X], ['O', UNICODE_O]],
                    attacks: vec![GenPieceAttackKind::piece_drop(AttackBitboardFilter::EmptySquares)],
                    promotions: Default::default(),
                    can_ep_capture: false,
                    reset_draw_counter: false,
                    royal: false,
                    can_castle: false,
                }
            },
        ];
        rest.append(&mut leapers);
        rest.append(&mut riders);
        rest
    }

    pub fn chess_pieces() -> Vec<Piece> {
        let size = FairySize::new(Height::new(8), Width::new(8));
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
        let size = FairySize::new(Height::new(8), Width::new(8));
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

    pub fn complete_piece_map(size: FairySize) -> HashMap<String, Self> {
        let mut res = HashMap::new();
        for piece in Self::pieces(size) {
            // insertion can fail because some pieces get inserted twice
            _ = res.insert(piece.name.clone(), piece);
        }
        res
    }
}
