use itertools::Itertools;
use std::fmt::{Display, Formatter};
use std::ops::{Index, IndexMut};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, FromRepr};

use crate::games::chess::pieces::ChessPieceType::*;

use crate::games::chess::ChessColor::*;
use crate::games::chess::{ChessColor, ChessSettings, Chessboard};
use crate::games::{AbstractPieceType, CharType, Color, ColoredPieceType, GenericPiece, PieceType};

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

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, EnumIter, FromRepr)]
#[must_use]
pub enum ChessPieceType {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
    #[default]
    Empty,
}

impl ChessPieceType {
    pub fn pieces() -> impl Iterator<Item = ChessPieceType> {
        Self::iter().dropping_back(1)
    }

    pub fn non_king_pieces() -> impl Iterator<Item = ChessPieceType> {
        Self::iter().dropping_back(2)
    }

    pub fn non_pawn_pieces() -> impl Iterator<Item = ChessPieceType> {
        Self::pieces().dropping(1)
    }

    pub fn is_knb(self) -> bool {
        [King, Knight, Bishop].contains(&self)
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

impl Display for ChessPieceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_name())
    }
}

impl AbstractPieceType<Chessboard> for ChessPieceType {
    fn empty() -> Self {
        Empty
    }

    fn non_empty(_settings: &ChessSettings) -> impl Iterator<Item = Self> {
        Self::pieces()
    }

    fn to_char(self, typ: CharType, _settings: &ChessSettings) -> char {
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

    fn to_display_char(self, typ: CharType, settings: &ChessSettings) -> char {
        ColoredChessPieceType::new(White, self).to_display_char(typ, settings)
    }

    /// Also parses German notation.
    fn from_char(c: char, _settings: &ChessSettings) -> Option<Self> {
        Self::parse_from_char(c)
    }

    fn name(&self, _settings: &ChessSettings) -> impl AsRef<str> {
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

impl PieceType<Chessboard> for ChessPieceType {
    type Colored = ColoredChessPieceType;

    fn from_idx(idx: usize) -> Self {
        Self::from_repr(idx).unwrap()
    }
}

impl<T> Index<ChessPieceType> for [T; 6] {
    type Output = T;

    fn index(&self, index: ChessPieceType) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T> IndexMut<ChessPieceType> for [T; 6] {
    fn index_mut(&mut self, index: ChessPieceType) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone, EnumIter, FromRepr)]
#[repr(usize)]
#[must_use]
pub enum ColoredChessPieceType {
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

impl ColoredChessPieceType {
    pub fn pieces() -> impl Iterator<Item = ColoredChessPieceType> {
        Self::iter().filter(|p| *p != ColoredChessPieceType::Empty)
    }

    pub fn non_pawns() -> impl Iterator<Item = ColoredChessPieceType> {
        Self::iter().filter(|p| {
            ![ColoredChessPieceType::Empty, ColoredChessPieceType::BlackPawn, ColoredChessPieceType::WhitePawn]
                .contains(p)
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
            ' ' => Some(ColoredChessPieceType::Empty),
            'P' | UNICODE_WHITE_PAWN => Some(ColoredChessPieceType::WhitePawn),
            'N' | 'S' | UNICODE_WHITE_KNIGHT => Some(ColoredChessPieceType::WhiteKnight),
            'B' | 'L' | UNICODE_WHITE_BISHOP => Some(ColoredChessPieceType::WhiteBishop),
            'R' | 'T' | UNICODE_WHITE_ROOK => Some(ColoredChessPieceType::WhiteRook),
            'Q' | 'D' | UNICODE_WHITE_QUEEN => Some(ColoredChessPieceType::WhiteQueen),
            'K' | UNICODE_WHITE_KING => Some(ColoredChessPieceType::WhiteKing),
            'p' | UNICODE_BLACK_PAWN => Some(ColoredChessPieceType::BlackPawn),
            'n' | 's' | UNICODE_BLACK_KNIGHT => Some(ColoredChessPieceType::BlackKnight),
            'b' | 'l' | UNICODE_BLACK_BISHOP => Some(ColoredChessPieceType::BlackBishop),
            'r' | 't' | UNICODE_BLACK_ROOK => Some(ColoredChessPieceType::BlackRook),
            'q' | 'd' | UNICODE_BLACK_QUEEN => Some(ColoredChessPieceType::BlackQueen),
            'k' | UNICODE_BLACK_KING => Some(ColoredChessPieceType::BlackKing),
            _ => None,
        }
    }
}

impl Display for ColoredChessPieceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name(&ChessSettings::default()).as_ref())
    }
}

impl AbstractPieceType<Chessboard> for ColoredChessPieceType {
    fn empty() -> Self {
        Self::Empty
    }

    fn non_empty(_settings: &ChessSettings) -> impl Iterator<Item = Self> {
        Self::pieces()
    }

    fn to_char(self, typ: CharType, _settings: &ChessSettings) -> char {
        match typ {
            CharType::Ascii => match self {
                ColoredChessPieceType::Empty => '.',
                ColoredChessPieceType::WhitePawn => 'P',
                ColoredChessPieceType::WhiteKnight => 'N',
                ColoredChessPieceType::WhiteBishop => 'B',
                ColoredChessPieceType::WhiteRook => 'R',
                ColoredChessPieceType::WhiteQueen => 'Q',
                ColoredChessPieceType::WhiteKing => 'K',
                ColoredChessPieceType::BlackPawn => 'p',
                ColoredChessPieceType::BlackKnight => 'n',
                ColoredChessPieceType::BlackBishop => 'b',
                ColoredChessPieceType::BlackRook => 'r',
                ColoredChessPieceType::BlackQueen => 'q',
                ColoredChessPieceType::BlackKing => 'k',
            },
            CharType::Unicode => match self {
                ColoredChessPieceType::Empty => '.',
                ColoredChessPieceType::WhitePawn => UNICODE_WHITE_PAWN,
                ColoredChessPieceType::WhiteKnight => UNICODE_WHITE_KNIGHT,
                ColoredChessPieceType::WhiteBishop => UNICODE_WHITE_BISHOP,
                ColoredChessPieceType::WhiteRook => UNICODE_WHITE_ROOK,
                ColoredChessPieceType::WhiteQueen => UNICODE_WHITE_QUEEN,
                ColoredChessPieceType::WhiteKing => UNICODE_WHITE_KING,
                ColoredChessPieceType::BlackPawn => UNICODE_BLACK_PAWN,
                ColoredChessPieceType::BlackKnight => UNICODE_BLACK_KNIGHT,
                ColoredChessPieceType::BlackBishop => UNICODE_BLACK_BISHOP,
                ColoredChessPieceType::BlackRook => UNICODE_BLACK_ROOK,
                ColoredChessPieceType::BlackQueen => UNICODE_BLACK_QUEEN,
                ColoredChessPieceType::BlackKing => UNICODE_BLACK_KING,
            },
        }
    }

    fn to_display_char(self, typ: CharType, settings: &ChessSettings) -> char {
        if self == ColoredChessPieceType::Empty {
            self.to_char(typ, settings)
        } else if typ == CharType::Unicode {
            ColoredChessPieceType::new(White, self.uncolor()).to_char(typ, settings)
        } else {
            self.to_char(typ, settings)
        }
    }

    /// Also parses German notation (pawns are still represented as 'p' to avoid ambiguity with bishops).
    fn from_char(c: char, _settings: &ChessSettings) -> Option<Self> {
        Self::parse_from_char(c)
    }

    fn name(&self, _settings: &ChessSettings) -> impl AsRef<str> {
        match self {
            ColoredChessPieceType::WhitePawn => "white pawn",
            ColoredChessPieceType::WhiteKnight => "white knight",
            ColoredChessPieceType::WhiteBishop => "white bishop",
            ColoredChessPieceType::WhiteRook => "white rook",
            ColoredChessPieceType::WhiteQueen => "white queen",
            ColoredChessPieceType::WhiteKing => "white king",
            ColoredChessPieceType::Empty => "empty",
            ColoredChessPieceType::BlackPawn => "black pawn",
            ColoredChessPieceType::BlackKnight => "black knight",
            ColoredChessPieceType::BlackBishop => "black bishop",
            ColoredChessPieceType::BlackRook => "black rook",
            ColoredChessPieceType::BlackQueen => "black queen",
            ColoredChessPieceType::BlackKing => "black king",
        }
    }

    fn to_uncolored_idx(self) -> usize {
        self.to_colored_idx() % BLACK_OFFSET
    }
}

impl ColoredPieceType<Chessboard> for ColoredChessPieceType {
    type Uncolored = ChessPieceType;

    fn color(self) -> Option<ChessColor> {
        match self {
            ColoredChessPieceType::Empty => None,
            x => ChessColor::iter().nth((x as u8 / BLACK_OFFSET as u8) as usize),
        }
    }

    fn to_colored_idx(self) -> usize {
        self as usize
    }

    fn new(color: ChessColor, uncolored: Self::Uncolored) -> Self {
        Self::from_repr((uncolored as usize) + (color as usize) * BLACK_OFFSET).unwrap()
    }
}

pub type ChessPiece = GenericPiece<Chessboard, ColoredChessPieceType>;
