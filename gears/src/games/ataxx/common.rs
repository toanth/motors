use crate::games::ataxx::common::AtaxxMoveType::{Cloning, Leaping};
use crate::games::ataxx::common::AtaxxPieceType::{Blocked, Empty, Occupied};
use crate::games::ataxx::AtaxxColor::{O, X};
use crate::games::ataxx::{AtaxxBoard, AtaxxColor, AtaxxSquare};
use crate::games::{AbstractPieceType, CharType, ColoredPieceType, DimT, PieceType};
use crate::general::board::{Board, BoardHelpers};
use crate::general::common::Res;
use crate::general::moves::Legality::Legal;
use crate::general::moves::{Legality, Move, UntrustedMove};
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
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char(CharType::Unicode))
    }
}

impl AbstractPieceType for AtaxxPieceType {
    fn empty() -> Self {
        Empty
    }

    fn to_char(self, _typ: CharType) -> char {
        match self {
            Empty => '.',
            Blocked => '-',
            Occupied => 'x',
        }
    }

    fn from_char(c: char) -> Option<Self> {
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
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_char(CharType::Unicode))
    }
}

impl AbstractPieceType for ColoredAtaxxPieceType {
    fn empty() -> Self {
        Self::Empty
    }

    fn to_char(self, _typ: CharType) -> char {
        match self {
            ColoredAtaxxPieceType::Empty => '.',
            ColoredAtaxxPieceType::Blocked => '-',
            XPiece => 'x',
            OPiece => 'o',
        }
    }

    fn from_char(c: char) -> Option<Self> {
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
    pub(super) source: u8,
    pub(super) target: AtaxxSquare,
}

impl Display for AtaxxMove {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.format_compact(f)
    }
}

impl Move<AtaxxBoard> for AtaxxMove {
    type Underlying = u16;

    fn legality() -> Legality {
        Legal
    }

    fn src_square_in(self, _pos: &AtaxxBoard) -> Option<AtaxxSquare> {
        if self.source == u8::MAX {
            None
        } else {
            Some(AtaxxSquare::unchecked(self.source as usize))
        }
    }

    fn dest_square_in(self, _pos: &AtaxxBoard) -> AtaxxSquare {
        self.target
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
        let (remaining, from, to_square) = if let Some(sq) = second_square {
            (&s[4..], first_square.to_u8(), sq)
        } else {
            (&s[2..], u8::MAX, first_square)
        };

        let res = Self {
            source: from,
            target: to_square,
        };
        if !board.is_move_pseudolegal(res) {
            if board.is_occupied(to_square) {
                bail!("The square {} is not empty", to_square.to_string().bold())
            } else if let Some(from_square) = res.src_square_in(&board) {
                bail!("There is no legal move from {from_square} to {to_square}");
            }
            bail!(
                "No piece can create a clone of itself on {}",
                to_square.to_string().red()
            )
        }

        Ok((remaining, res))
    }

    fn parse_extended_text<'a>(s: &'a str, board: &AtaxxBoard) -> Res<(&'a str, AtaxxMove)> {
        Self::parse_compact_text(s, board)
    }

    fn from_u64_unchecked(val: u64) -> UntrustedMove<AtaxxBoard> {
        let source = (val >> 8) as u8;
        let target = AtaxxSquare::unchecked(val as usize & 0xff);
        UntrustedMove::from_move(Self { source, target })
    }

    fn to_underlying(self) -> Self::Underlying {
        (u16::from(self.source) << 8) | u16::from(self.target.to_u8())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum AtaxxMoveType {
    Leaping,
    Cloning,
}

impl AtaxxMove {
    pub fn dest_square(self) -> AtaxxSquare {
        self.target
    }

    pub fn cloning(square: AtaxxSquare) -> Self {
        Self {
            target: square,
            source: u8::MAX,
        }
    }

    pub fn leaping(source: AtaxxSquare, target: AtaxxSquare) -> Self {
        Self {
            source: source.to_u8(),
            target,
        }
    }

    pub fn typ(self) -> AtaxxMoveType {
        if self.source == u8::MAX {
            Cloning
        } else {
            Leaping
        }
    }
}
