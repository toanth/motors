use crate::games::ataxx::common::AtaxxMoveType::{Cloning, Leaping};
use crate::games::ataxx::common::AtaxxPieceType::{Blocked, Empty, Occupied};
use crate::games::ataxx::AtaxxColor::{O, X};
use crate::games::ataxx::{AtaxxBoard, AtaxxColor, AtaxxSquare};
use crate::games::{AbstractPieceType, ColoredPieceType, Coordinates, DimT, PieceType};
use crate::general::board::Board;
use crate::general::common::Res;
use crate::general::moves::Legality::Legal;
use crate::general::moves::{Legality, Move, NoMoveFlags, UntrustedMove};
use anyhow::bail;
use arbitrary::Arbitrary;
use colored::Colorize;
use std::fmt;
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

impl PieceType<AtaxxBoard> for AtaxxPieceType {
    type Colored = ColoredAtaxxPieceType;

    fn from_idx(idx: usize) -> Self {
        Self::iter().nth(idx).unwrap()
    }
}

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone, EnumIter)]
pub enum ColoredAtaxxPieceType {
    #[default]
    Empty,
    Blocked,
    XPiece,
    OPiece,
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
            XPiece => 'x',
            OPiece => 'o',
        }
    }

    fn from_utf8_char(c: char) -> Option<Self> {
        match c {
            '.' => Some(Self::Empty),
            '-' => Some(Self::Blocked),
            'x' => Some(XPiece),
            'o' => Some(OPiece),
            _ => None,
        }
    }

    fn to_uncolored_idx(self) -> usize {
        (self as usize).min(Occupied as usize)
    }
}

impl ColoredPieceType<AtaxxBoard> for ColoredAtaxxPieceType {
    type Uncolored = AtaxxPieceType;

    fn color(self) -> Option<AtaxxColor> {
        match self {
            OPiece => Some(O),
            XPiece => Some(X),
            _ => None,
        }
    }

    fn to_colored_idx(self) -> usize {
        (self as usize).min(Occupied as usize)
    }

    fn new(color: AtaxxColor, uncolored: Self::Uncolored) -> Self {
        match uncolored {
            Occupied => match color {
                O => XPiece,
                X => OPiece,
            },
            Empty => Self::Empty,
            Blocked => Self::Blocked,
        }
    }
}

pub const MAX_ATAXX_MOVES_IN_POS: usize =
    NUM_SQUARES + 2 * ((NUM_ROWS - 2) * (NUM_COLUMNS - 2) * 2 + (NUM_ROWS - 2 + NUM_COLUMNS - 2));

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[repr(C)]
pub struct AtaxxMove {
    source: AtaxxSquare,
    target: AtaxxSquare,
}

impl Display for AtaxxMove {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.format_compact(f)
    }
}

impl Move<AtaxxBoard> for AtaxxMove {
    type Flags = NoMoveFlags;
    type Underlying = u16;

    fn legality() -> Legality {
        Legal
    }

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

    fn format_compact(self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.typ() {
            Leaping => write!(f, "{0}{1}", self.source, self.target),
            Cloning => write!(f, "{}", self.target),
        }
    }

    fn parse_compact_text<'a>(s: &'a str, board: &AtaxxBoard) -> Res<(&'a str, AtaxxMove)> {
        let s = s.trim();
        if s.is_empty() {
            bail!("Empty input");
        }
        if let Some(rest) = s.strip_prefix("0000") {
            return Ok((rest, Self::default()));
        }
        let Some(first_square) = s.get(..2) else {
            bail!("Move '{}' doesn't start with 2 ascii characters", s.red());
        };
        let first_square = AtaxxSquare::from_str(first_square)?;
        let second_square = s.get(2..4).and_then(|s| AtaxxSquare::from_str(s).ok());
        let (remaining, from_square, to_square) = if let Some(sq) = second_square {
            (&s[4..], first_square, sq)
        } else {
            (&s[2..], AtaxxSquare::no_coordinates(), first_square)
        };

        let res = Self {
            source: from_square,
            target: to_square,
        };
        if !board.is_move_pseudolegal(res) {
            if !board.is_empty(to_square) {
                bail!("The square {} is not empty", to_square.to_string().bold())
            } else if from_square != AtaxxSquare::no_coordinates() {
                bail!("The")
            }
            bail!("No piece can move to {}", to_square.to_string().red())
        }

        Ok((remaining, res))
    }

    fn parse_extended_text<'a>(s: &'a str, board: &AtaxxBoard) -> Res<(&'a str, AtaxxMove)> {
        Self::parse_compact_text(s, board)
    }

    fn from_usize_unchecked(val: usize) -> UntrustedMove<AtaxxBoard> {
        let source = AtaxxSquare::unchecked((val >> 8) & 0xff);
        let target = AtaxxSquare::unchecked(val & 0xff);
        UntrustedMove::from_move(Self { source, target })
    }

    fn to_underlying(self) -> Self::Underlying {
        (u16::from(self.source.to_u8()) << 8) | u16::from(self.target.to_u8())
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
        Self { source, target }
    }

    pub fn typ(self) -> AtaxxMoveType {
        if self.source == AtaxxSquare::no_coordinates() {
            Cloning
        } else {
            Leaping
        }
    }
}
