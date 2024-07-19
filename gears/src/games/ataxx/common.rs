use crate::games::ataxx::common::AtaxxMoveType::{Cloning, Leaping};
use crate::games::ataxx::common::AtaxxPieceType::{Blocked, Empty, Occupied};
use crate::games::ataxx::{AtaxxBoard, AtaxxSquare};
use crate::games::Color::{Black, White};
use crate::games::{
    AbstractPieceType, Color, ColoredPieceType, Coordinates, DimT, Move, NoMoveFlags,
    UncoloredPieceType,
};
use crate::general::common::Res;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use ColoredAtaxxPieceType::*;

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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_utf8_char())
    }
}

impl AbstractPieceType for AtaxxPieceType {
    fn empty() -> Self {
        Empty
    }

    fn to_ascii_char(self) -> char {
        match self {
            Empty => '.',
            Blocked => '-',
            Occupied => 'x',
        }
    }

    fn from_utf8_char(c: char) -> Option<Self> {
        match c {
            '.' => Some(Empty),
            '-' => Some(Blocked),
            'o' | 'x' => Some(Occupied),
            _ => None,
        }
    }

    fn to_uncolored_idx(self) -> usize {
        self as usize
    }
}

impl UncoloredPieceType for AtaxxPieceType {
    type Colored = ColoredAtaxxPieceType;

    fn from_uncolored_idx(idx: usize) -> Self {
        Self::iter().nth(idx).unwrap()
    }
}

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone, EnumIter)]
pub enum ColoredAtaxxPieceType {
    #[default]
    Empty,
    Blocked,
    WhitePiece,
    BlackPiece,
}

impl Display for ColoredAtaxxPieceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_utf8_char())
    }
}

impl AbstractPieceType for ColoredAtaxxPieceType {
    fn empty() -> Self {
        Self::Empty
    }

    fn to_ascii_char(self) -> char {
        match self {
            ColoredAtaxxPieceType::Empty => '.',
            ColoredAtaxxPieceType::Blocked => '-',
            WhitePiece => 'x',
            BlackPiece => 'o',
        }
    }

    fn from_utf8_char(c: char) -> Option<Self> {
        match c {
            '.' => Some(Self::Empty),
            '-' => Some(Self::Blocked),
            'x' => Some(WhitePiece),
            'o' => Some(BlackPiece),
            _ => None,
        }
    }

    fn to_uncolored_idx(self) -> usize {
        (self as usize).min(Occupied as usize)
    }
}

impl ColoredPieceType for ColoredAtaxxPieceType {
    type Uncolored = AtaxxPieceType;

    fn color(self) -> Option<Color> {
        match self {
            WhitePiece => Some(White),
            BlackPiece => Some(Black),
            _ => None,
        }
    }

    fn to_colored_idx(self) -> usize {
        (self as usize).min(Occupied as usize)
    }

    fn new(color: Color, uncolored: Self::Uncolored) -> Self {
        match uncolored {
            Occupied => match color {
                White => WhitePiece,
                Black => BlackPiece,
            },
            Empty => Self::Empty,
            Blocked => Self::Blocked,
        }
    }
}

pub const MAX_ATAXX_MOVES_IN_POS: usize =
    NUM_SQUARES + 2 * ((NUM_ROWS - 2) * (NUM_COLUMNS - 2) * 2 + (NUM_ROWS - 2 + NUM_COLUMNS - 2));

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct AtaxxMove {
    source: AtaxxSquare,
    target: AtaxxSquare,
}

impl Display for AtaxxMove {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{0}", self.to_compact_text())
    }
}

impl Move<AtaxxBoard> for AtaxxMove {
    type Flags = NoMoveFlags;
    type Underlying = u16;

    fn src_square(self) -> AtaxxSquare {
        self.source
    }

    fn dest_square(self) -> AtaxxSquare {
        self.target
    }

    fn flags(self) -> Self::Flags {
        NoMoveFlags::default()
    }

    fn is_tactical(self, _board: &AtaxxBoard) -> bool {
        false
    }

    fn to_compact_text(self) -> String {
        match self.typ() {
            Leaping => format!("{0}{1}", self.source, self.target),
            Cloning => format!("{}", self.target),
        }
    }

    fn from_compact_text(s: &str, _board: &AtaxxBoard) -> Res<AtaxxMove> {
        let s = s.trim();
        if s.is_empty() {
            return Err("Empty input".to_string());
        }
        if s == "0000" {
            return Ok(Self::default());
        }
        if s.len() != 2 && s.len() != 4 {
            return Err(format!(
                "Incorrect move length: '{s}'. Must contain exactly one or two squares"
            ));
        }
        let mut from_square = AtaxxSquare::no_coordinates();
        let mut to_square = AtaxxSquare::from_str(&s[0..2])?;
        if s.len() == 4 {
            from_square = to_square;
            to_square = AtaxxSquare::from_str(&s[2..4])?;
        }

        Ok(Self {
            source: from_square,
            target: to_square,
        })
    }

    fn from_extended_text(s: &str, board: &AtaxxBoard) -> Res<AtaxxMove> {
        Self::from_compact_text(s, board)
    }

    fn from_usize_unchecked(val: usize) -> Self {
        let source = AtaxxSquare::from_bb_index((val >> 8) & 0xff);
        let target = AtaxxSquare::from_bb_index(val & 0xff);
        Self { target, source }
    }

    fn to_underlying(self) -> Self::Underlying {
        ((self.source.to_u8() as u16) << 8) | (self.target.to_u8() as u16)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum AtaxxMoveType {
    Leaping,
    Cloning,
}

impl AtaxxMove {
    pub fn cloning(square: AtaxxSquare) -> Self {
        Self {
            target: square,
            source: AtaxxSquare::no_coordinates(),
        }
    }

    pub fn leaping(source: AtaxxSquare, target: AtaxxSquare) -> Self {
        Self { target, source }
    }

    pub fn typ(self) -> AtaxxMoveType {
        if self.source == AtaxxSquare::no_coordinates() {
            Cloning
        } else {
            Leaping
        }
    }
}
