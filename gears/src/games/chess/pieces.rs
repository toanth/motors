use arbitrary::Arbitrary;
use itertools::Itertools;
use std::fmt::{Display, Formatter};
use std::ops::{Index, IndexMut};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, FromRepr};

use crate::games::chess::pieces::PieceType::*;

use crate::games::chess::Color::*;
use crate::games::chess::{Board, Color, Settings};
use crate::games::{AbstractPieceType, CharType, ColoredPieceTypeTrait, GenericPiece, PieceTypeTrait};

pub const NUM_CHESS_PIECES: usize = 6;
pub const BLACK_OFFSET: usize = 8;

// These symbols were introduced in Unicode 12 and aren't widely supported yet
// They also don't look that great, so while we accept them, we don't emit them
pub const UNICODE_NEUTRAL_PAWN: char = '🨅';
pub const UNICODE_NEUTRAL_KNIGHT: char = '🨄';
pub const UNICODE_NEUTRAL_BISHOP: char = '🨃';
pub const UNICODE_NEUTRAL_ROOK: char = '🨂';
pub const UNICODE_NEUTRAL_QUEEN: char = '🨁';
pub const UNICODE_NEUTRAL_KING: char = '🨀';

// normal unicode symbols
pub const UNICODE_WHITE_PAWN: char = '♙';
pub const UNICODE_WHITE_KNIGHT: char = '♘';
pub const UNICODE_WHITE_BISHOP: char = '♗';
pub const UNICODE_WHITE_ROOK: char = '♖';
pub const UNICODE_WHITE_QUEEN: char = '♕';
pub const UNICODE_WHITE_KING: char = '♔';

// The black pieces are a lot easier to look at, so they're used for the uncolored versions
pub const UNICODE_BLACK_PAWN: char = '\u{265F}'; // the '♟︎' character seems to give RustRover trouble
pub const UNICODE_BLACK_KNIGHT: char = '♞';
pub const UNICODE_BLACK_BISHOP: char = '♝';
pub const UNICODE_BLACK_ROOK: char = '♜';
pub const UNICODE_BLACK_QUEEN: char = '♛';
pub const UNICODE_BLACK_KING: char = '♚';

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Default, Hash, EnumIter, FromRepr, Arbitrary)]
#[must_use]
pub enum PieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
    #[default]
    Empty,
}

impl PieceType {
    pub fn pieces() -> impl Iterator<Item = PieceType> {
        Self::iter().dropping_back(1)
    }

    pub fn non_king_pieces() -> impl Iterator<Item = PieceType> {
        Self::iter().dropping_back(2)
    }

    pub fn non_pawn_pieces() -> impl Iterator<Item = PieceType> {
        Self::pieces().dropping(1)
    }

    pub fn to_name(self) -> &'static str {
        match self {
            Pawn => "pawn",
            Knight => "knight",
            Bishop => "bishop",
            Rook => "rook",
            Queen => "queen",
            King => "king",
            Empty => "empty",
        }
    }
    pub fn parse_from_char(c: char) -> Option<Self> {
        match c.to_ascii_lowercase() {
            ' ' => Some(Empty),
            // it's normal to use white symbols as colorless symbols, so also support that
            // And since we output the black pieces, we should definitely parse them, too
            'p' | UNICODE_NEUTRAL_PAWN | UNICODE_WHITE_PAWN | UNICODE_BLACK_PAWN => Some(Pawn),
            'n' | 's' | UNICODE_NEUTRAL_KNIGHT | UNICODE_WHITE_KNIGHT | UNICODE_BLACK_KNIGHT => Some(Knight),
            'b' | 'l' | UNICODE_NEUTRAL_BISHOP | UNICODE_WHITE_BISHOP | UNICODE_BLACK_BISHOP => Some(Bishop),
            'r' | 't' | UNICODE_NEUTRAL_ROOK | UNICODE_WHITE_ROOK | UNICODE_BLACK_ROOK => Some(Rook),
            'q' | 'd' | UNICODE_NEUTRAL_QUEEN | UNICODE_WHITE_QUEEN | UNICODE_BLACK_QUEEN => Some(Queen),
            'k' | UNICODE_NEUTRAL_KING | UNICODE_WHITE_KING | UNICODE_BLACK_KING => Some(King),
            _ => None,
        }
    }
}

impl Display for PieceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_name())
    }
}

impl AbstractPieceType<Board> for PieceType {
    fn empty() -> Self {
        Empty
    }

    fn non_empty(_settings: &Settings) -> impl Iterator<Item = Self> {
        Self::pieces()
    }

    fn to_char(self, typ: CharType, _settings: &Settings) -> char {
        match typ {
            CharType::Ascii => match self {
                Empty => '.',
                Pawn => 'p',
                Knight => 'N',
                Bishop => 'B',
                Rook => 'R',
                Queen => 'Q',
                King => 'K',
            },
            // The black pieces are often the prettiest and the easiest to recognize, though this depends very much on the font
            CharType::Unicode => match self {
                Empty => '.',
                // Some fonts have problems with the black pawn and use the emoji instead, so use the white version to circumvent that
                Pawn => UNICODE_WHITE_PAWN,
                Knight => UNICODE_BLACK_KNIGHT,
                Bishop => UNICODE_BLACK_BISHOP,
                Rook => UNICODE_BLACK_ROOK,
                Queen => UNICODE_BLACK_QUEEN,
                King => UNICODE_BLACK_KING,
            },
        }
    }

    fn to_display_char(self, typ: CharType, settings: &Settings) -> char {
        ColoredPieceType::new(White, self).to_display_char(typ, settings)
    }

    /// Also parses German notation.
    fn from_char(c: char, _settings: &Settings) -> Option<Self> {
        Self::parse_from_char(c)
    }

    fn name(&self, _settings: &Settings) -> impl AsRef<str> {
        match self {
            Pawn => "pawn",
            Knight => "knight",
            Bishop => "bishop",
            Rook => "rook",
            Queen => "queen",
            King => "king",
            Empty => "empty",
        }
    }

    fn to_uncolored_idx(self) -> usize {
        self as usize
    }
}

impl PieceTypeTrait<Board> for PieceType {
    type Colored = ColoredPieceType;

    fn from_idx(idx: usize) -> Self {
        Self::from_repr(idx).unwrap()
    }
}

impl<T> Index<PieceType> for [T; 6] {
    type Output = T;

    fn index(&self, index: PieceType) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T> IndexMut<PieceType> for [T; 6] {
    fn index_mut(&mut self, index: PieceType) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone, EnumIter, FromRepr)]
#[repr(usize)]
#[must_use]
pub enum ColoredPieceType {
    WhitePawn,
    WhiteKnight,
    WhiteBishop,
    WhiteRook,
    WhiteQueen,
    WhiteKing,
    #[default]
    Empty,
    BlackPawn = BLACK_OFFSET,
    BlackKnight,
    BlackBishop,
    BlackRook,
    BlackQueen,
    BlackKing,
}

impl ColoredPieceType {
    pub fn pieces() -> impl Iterator<Item = ColoredPieceType> {
        Self::iter().filter(|p| *p != ColoredPieceType::Empty)
    }

    pub fn non_pawns() -> impl Iterator<Item = ColoredPieceType> {
        Self::iter().filter(|p| {
            ![ColoredPieceType::Empty, ColoredPieceType::BlackPawn, ColoredPieceType::WhitePawn].contains(p)
        })
    }

    pub fn name(self) -> String {
        format!(
            "{0}{1}",
            self.color()
                .map(|c| {
                    let mut s = c.to_string();
                    s.push(' ');
                    s
                })
                .unwrap_or_default(),
            self.uncolor().to_name()
        )
    }

    pub fn parse_from_char(c: char) -> Option<Self> {
        match c {
            ' ' => Some(ColoredPieceType::Empty),
            'P' | UNICODE_WHITE_PAWN => Some(ColoredPieceType::WhitePawn),
            'N' | 'S' | UNICODE_WHITE_KNIGHT => Some(ColoredPieceType::WhiteKnight),
            'B' | 'L' | UNICODE_WHITE_BISHOP => Some(ColoredPieceType::WhiteBishop),
            'R' | 'T' | UNICODE_WHITE_ROOK => Some(ColoredPieceType::WhiteRook),
            'Q' | 'D' | UNICODE_WHITE_QUEEN => Some(ColoredPieceType::WhiteQueen),
            'K' | UNICODE_WHITE_KING => Some(ColoredPieceType::WhiteKing),
            'p' | UNICODE_BLACK_PAWN => Some(ColoredPieceType::BlackPawn),
            'n' | 's' | UNICODE_BLACK_KNIGHT => Some(ColoredPieceType::BlackKnight),
            'b' | 'l' | UNICODE_BLACK_BISHOP => Some(ColoredPieceType::BlackBishop),
            'r' | 't' | UNICODE_BLACK_ROOK => Some(ColoredPieceType::BlackRook),
            'q' | 'd' | UNICODE_BLACK_QUEEN => Some(ColoredPieceType::BlackQueen),
            'k' | UNICODE_BLACK_KING => Some(ColoredPieceType::BlackKing),
            _ => None,
        }
    }
}

impl Display for ColoredPieceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name(&Settings::default()).as_ref())
    }
}

impl AbstractPieceType<Board> for ColoredPieceType {
    fn empty() -> Self {
        Self::Empty
    }

    fn non_empty(_settings: &Settings) -> impl Iterator<Item = Self> {
        Self::pieces()
    }

    fn to_char(self, typ: CharType, _settings: &Settings) -> char {
        match typ {
            CharType::Ascii => match self {
                ColoredPieceType::Empty => '.',
                ColoredPieceType::WhitePawn => 'P',
                ColoredPieceType::WhiteKnight => 'N',
                ColoredPieceType::WhiteBishop => 'B',
                ColoredPieceType::WhiteRook => 'R',
                ColoredPieceType::WhiteQueen => 'Q',
                ColoredPieceType::WhiteKing => 'K',
                ColoredPieceType::BlackPawn => 'p',
                ColoredPieceType::BlackKnight => 'n',
                ColoredPieceType::BlackBishop => 'b',
                ColoredPieceType::BlackRook => 'r',
                ColoredPieceType::BlackQueen => 'q',
                ColoredPieceType::BlackKing => 'k',
            },
            CharType::Unicode => match self {
                ColoredPieceType::Empty => '.',
                ColoredPieceType::WhitePawn => UNICODE_WHITE_PAWN,
                ColoredPieceType::WhiteKnight => UNICODE_WHITE_KNIGHT,
                ColoredPieceType::WhiteBishop => UNICODE_WHITE_BISHOP,
                ColoredPieceType::WhiteRook => UNICODE_WHITE_ROOK,
                ColoredPieceType::WhiteQueen => UNICODE_WHITE_QUEEN,
                ColoredPieceType::WhiteKing => UNICODE_WHITE_KING,
                ColoredPieceType::BlackPawn => UNICODE_BLACK_PAWN,
                ColoredPieceType::BlackKnight => UNICODE_BLACK_KNIGHT,
                ColoredPieceType::BlackBishop => UNICODE_BLACK_BISHOP,
                ColoredPieceType::BlackRook => UNICODE_BLACK_ROOK,
                ColoredPieceType::BlackQueen => UNICODE_BLACK_QUEEN,
                ColoredPieceType::BlackKing => UNICODE_BLACK_KING,
            },
        }
    }

    fn to_display_char(self, typ: CharType, settings: &Settings) -> char {
        if self == ColoredPieceType::Empty {
            self.to_char(typ, settings)
        } else if typ == CharType::Unicode {
            ColoredPieceType::new(White, self.uncolor()).to_char(typ, settings)
        } else {
            self.to_char(typ, settings)
        }
    }

    /// Also parses German notation (pawns are still represented as 'p' to avoid ambiguity with bishops).
    fn from_char(c: char, _settings: &Settings) -> Option<Self> {
        Self::parse_from_char(c)
    }

    fn name(&self, _settings: &Settings) -> impl AsRef<str> {
        match self {
            ColoredPieceType::WhitePawn => "white pawn",
            ColoredPieceType::WhiteKnight => "white knight",
            ColoredPieceType::WhiteBishop => "white bishop",
            ColoredPieceType::WhiteRook => "white rook",
            ColoredPieceType::WhiteQueen => "white queen",
            ColoredPieceType::WhiteKing => "white king",
            ColoredPieceType::Empty => "empty",
            ColoredPieceType::BlackPawn => "black pawn",
            ColoredPieceType::BlackKnight => "black knight",
            ColoredPieceType::BlackBishop => "black bishop",
            ColoredPieceType::BlackRook => "black rook",
            ColoredPieceType::BlackQueen => "black queen",
            ColoredPieceType::BlackKing => "black king",
        }
    }

    fn to_uncolored_idx(self) -> usize {
        self.to_colored_idx() % BLACK_OFFSET
    }
}

impl ColoredPieceTypeTrait<Board> for ColoredPieceType {
    type Uncolored = PieceType;

    fn color(self) -> Option<Color> {
        if self == ColoredPieceType::Empty { None } else { Color::from_repr(self as usize / BLACK_OFFSET) }
    }

    fn to_colored_idx(self) -> usize {
        self as usize
    }

    fn new(color: Color, uncolored: Self::Uncolored) -> Self {
        Self::from_repr((uncolored as usize) + (color as usize) * BLACK_OFFSET).unwrap()
    }
}

pub type Piece = GenericPiece<Board, ColoredPieceType>;
