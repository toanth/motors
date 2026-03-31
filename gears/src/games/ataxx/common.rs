use crate::games::ataxx::Color::{O, X};
use crate::games::ataxx::common::AtaxxMoveType::{Cloning, Leaping};
use crate::games::ataxx::common::AtaxxPieceType::{Blocked, Empty, Occupied};
use crate::games::ataxx::{Board, Color, Settings, Square};
use crate::games::{AbstractPieceType, CharType, ColorTrait, ColoredPieceTypeTrait, DimT, PieceTypeTrait};
use crate::general::board::{BoardHelpers, BoardTrait};
use crate::general::common::Res;
use crate::general::moves::Legality::Legal;
use crate::general::moves::{Legality, MoveTrait, UntrustedMove};
use ColoredPieceType::*;
use anyhow::{bail, ensure};
use arbitrary::Arbitrary;
use colored::Colorize;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

pub const NUM_ROWS: usize = 7;
pub const NUM_COLUMNS: usize = 7;
pub const NUM_SQUARES: usize = NUM_ROWS * NUM_COLUMNS;
#[allow(unused)]
pub const A_FILE_NO: DimT = 0;
#[allow(unused)]
pub const B_FILE_NO: DimT = 1;
#[allow(unused)]
pub const C_FILE_NO: DimT = 2;
#[allow(unused)]
pub const D_FILE_NO: DimT = 3;
#[allow(unused)]
pub const E_FILE_NO: DimT = 4;
#[allow(unused)]
pub const F_FILE_NO: DimT = 5;
#[allow(unused)]
pub const G_FILE_NO: DimT = 6;

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone, EnumIter)]
pub enum AtaxxPieceType {
    #[default]
    Empty,
    Blocked,
    Occupied,
}

impl Display for AtaxxPieceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char(CharType::Unicode, &Settings))
    }
}

impl AbstractPieceType<Board> for AtaxxPieceType {
    fn empty() -> Self {
        Empty
    }

    fn non_empty(_settings: &Settings) -> impl Iterator<Item = Self> {
        [Blocked, Occupied].into_iter()
    }

    fn to_char(self, _typ: CharType, _setting: &Settings) -> char {
        match self {
            Empty => '.',
            Blocked => '-',
            Occupied => 'x',
        }
    }

    fn from_char(c: char, _pos: &Settings) -> Option<Self> {
        match c {
            '.' => Some(Empty),
            '-' => Some(Blocked),
            'o' | 'x' => Some(Occupied),
            _ => None,
        }
    }

    fn name(&self, _settings: &Settings) -> impl AsRef<str> + ToString {
        match self {
            Empty => "empty",
            Blocked => "gap",
            Occupied => "stone",
        }
    }

    fn to_uncolored_idx(self) -> usize {
        self as usize
    }
}

impl PieceTypeTrait<Board> for AtaxxPieceType {
    type Colored = ColoredPieceType;

    fn from_idx(idx: usize) -> Self {
        Self::iter().nth(idx).unwrap()
    }
}

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone, EnumIter)]
pub enum ColoredPieceType {
    #[default]
    Empty,
    Blocked,
    XPiece,
    OPiece,
}

impl Display for ColoredPieceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char(CharType::Unicode, &Settings))
    }
}

impl AbstractPieceType<Board> for ColoredPieceType {
    fn empty() -> Self {
        Self::Empty
    }

    fn non_empty(_settings: &Settings) -> impl Iterator<Item = Self> {
        [Self::Blocked, XPiece, OPiece].into_iter()
    }

    fn to_char(self, _typ: CharType, _settings: &Settings) -> char {
        match self {
            ColoredPieceType::Empty => '.',
            ColoredPieceType::Blocked => '-',
            XPiece => 'x',
            OPiece => 'o',
        }
    }

    fn to_display_char(self, typ: CharType, settings: &Settings) -> char {
        self.to_char(typ, settings).to_ascii_uppercase()
    }

    fn from_char(c: char, _settings: &Settings) -> Option<Self> {
        match c {
            '.' => Some(Self::Empty),
            '-' => Some(Self::Blocked),
            'x' => Some(XPiece),
            'o' => Some(OPiece),
            _ => None,
        }
    }

    fn name(&self, _settings: &Settings) -> &'static str {
        match self {
            ColoredPieceType::Empty => "empty",
            ColoredPieceType::Blocked => "gap",
            XPiece => "x",
            OPiece => "o",
        }
    }

    fn to_uncolored_idx(self) -> usize {
        (self as usize).min(Occupied as usize)
    }
}

impl ColoredPieceTypeTrait<Board> for ColoredPieceType {
    type Uncolored = AtaxxPieceType;

    fn new(color: Color, uncolored: Self::Uncolored) -> Self {
        match uncolored {
            Occupied => match color {
                O => XPiece,
                X => OPiece,
            },
            Empty => Self::Empty,
            Blocked => Self::Blocked,
        }
    }

    fn color(self) -> Option<Color> {
        match self {
            OPiece => Some(O),
            XPiece => Some(X),
            _ => None,
        }
    }

    fn to_colored_idx(self) -> usize {
        (self as usize).min(Occupied as usize)
    }
}

pub const MAX_MOVES_IN_POS: usize =
    NUM_SQUARES + 2 * ((NUM_ROWS - 2) * (NUM_COLUMNS - 2) * 2 + (NUM_ROWS - 2 + NUM_COLUMNS - 2));

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[repr(C)]
pub struct Move {
    pub(super) source: u8,
    pub(super) target: Square,
}

impl Display for Move {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.typ() {
            Leaping => write!(f, "{0}{1}", Square::unchecked(self.source as usize), self.target),
            Cloning => write!(f, "{}", self.target),
        }
    }
}

impl MoveTrait<Board> for Move {
    type Underlying = u16;

    fn legality(_: &Settings) -> Legality {
        Legal
    }

    fn src_square_in(self, _pos: &Board) -> Option<Square> {
        if self.source == u8::MAX { None } else { Some(Square::unchecked(self.source as usize)) }
    }

    fn dest_square_in(self, _pos: &Board) -> Square {
        self.target
    }

    fn is_tactical(self, _board: &Board) -> bool {
        false
    }

    fn description(self, board: &Board) -> String {
        let piece = board.active_player.name(board.settings()).bold();
        match self.typ() {
            Leaping => format!(
                "The {piece} leaps from {0} to {1}",
                self.source.to_string().bold(),
                self.target.to_string().bold()
            ),
            Cloning => format!("Clone a {piece} onto {0}", self.target.to_string().bold()),
        }
    }

    fn format_compact(self, f: &mut Formatter<'_>, _board: &Board) -> fmt::Result {
        write!(f, "{self}")
    }

    fn parse_compact_text<'a>(s: &'a str, board: &Board) -> Res<(&'a str, Move)> {
        let s = s.trim();
        ensure!(!s.is_empty(), "Empty input");
        if let Some(rest) = s.strip_prefix("0000") {
            return Ok((rest, Self::default()));
        }
        let Some(first_square) = s.get(..2) else {
            bail!("Move '{}' doesn't start with 2 ascii characters", s.red());
        };
        let first_square = Square::from_str(first_square)?;
        let second_square = s.get(2..4).and_then(|s| Square::from_str(s).ok());
        let (remaining, from, to_square) = if let Some(sq) = second_square {
            (&s[4..], first_square.as_u8(), sq)
        } else {
            (&s[2..], u8::MAX, first_square)
        };

        let res = Self { source: from, target: to_square };
        if !board.is_move_pseudolegal(res) {
            if board.is_occupied(to_square) {
                bail!("The square {} is not empty", to_square.to_string().bold())
            } else if let Some(from_square) = res.src_square_in(board) {
                bail!("There is no legal move from {from_square} to {to_square}");
            }
            bail!("No piece can create a clone of itself on {}", to_square.to_string().red())
        }

        Ok((remaining, res))
    }

    fn parse_extended_text<'a>(s: &'a str, board: &Board) -> Res<(&'a str, Move)> {
        Self::parse_compact_text(s, board)
    }

    fn from_u64_unchecked(val: u64) -> UntrustedMove<Board> {
        let source = (val >> 8) as u8;
        let target = Square::unchecked(val as usize & 0xff);
        UntrustedMove::from_move(Self { source, target })
    }

    fn to_underlying(self) -> Self::Underlying {
        (u16::from(self.source) << 8) | u16::from(self.target.as_u8())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum AtaxxMoveType {
    Leaping,
    Cloning,
}

impl Move {
    pub fn dest_square(self) -> Square {
        self.target
    }

    pub fn cloning(square: Square) -> Self {
        Self { target: square, source: u8::MAX }
    }

    pub fn leaping(source: Square, target: Square) -> Self {
        Self { source: source.as_u8(), target }
    }

    pub fn typ(self) -> AtaxxMoveType {
        if self.source == u8::MAX { Cloning } else { Leaping }
    }
}
