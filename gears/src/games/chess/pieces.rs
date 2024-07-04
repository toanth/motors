use std::fmt::{Display, Formatter};

use itertools::Itertools;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, FromRepr};

use crate::games::chess::pieces::ColoredChessPiece::BlackPawn;
use crate::games::chess::pieces::UncoloredChessPiece::*;
use crate::games::chess::squares::ChessSquare;

use crate::games::Color::Black;
use crate::games::{AbstractPieceType, Color, ColoredPieceType, GenericPiece, UncoloredPieceType};

pub const NUM_CHESS_PIECES: usize = 6;
pub const NUM_COLORS: usize = 2;
pub const BLACK_OFFSET: usize = 8;

// These symbols were introduced in unicode 12 and aren't widely supported yet
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

pub const UNICODE_BLACK_PAWN: char = '\u{265F}'; // the '♟︎' character seems to give RustRover trouble
pub const UNICODE_BLACK_KNIGHT: char = '♞';
pub const UNICODE_BLACK_BISHOP: char = '♝';
pub const UNICODE_BLACK_ROOK: char = '♜';
pub const UNICODE_BLACK_QUEEN: char = '♛';
pub const UNICODE_BLACK_KING: char = '♚';

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, EnumIter, FromRepr)]
pub enum UncoloredChessPiece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
    #[default]
    Empty,
}

impl UncoloredChessPiece {
    pub fn pieces() -> UncoloredChessPieceIter {
        Self::iter().dropping_back(1)
    }

    pub fn non_king_pieces() -> UncoloredChessPieceIter {
        Self::iter().dropping_back(2)
    }

    pub fn non_pawn_pieces() -> UncoloredChessPieceIter {
        Self::pieces().dropping(1)
    }

    pub fn name(self) -> &'static str {
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
}

impl Display for UncoloredChessPiece {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_utf8_char())
    }
}

impl AbstractPieceType for UncoloredChessPiece {
    fn empty() -> Self {
        Empty
    }

    fn to_ascii_char(self) -> char {
        match self {
            Empty => '.',
            Pawn => 'p',
            Knight => 'N',
            Bishop => 'B',
            Rook => 'R',
            Queen => 'Q',
            King => 'K',
        }
    }

    fn to_utf8_char(self) -> char {
        match self {
            Empty => '.',
            Pawn => UNICODE_NEUTRAL_PAWN,
            Knight => UNICODE_NEUTRAL_KNIGHT,
            Bishop => UNICODE_NEUTRAL_BISHOP,
            Rook => UNICODE_NEUTRAL_ROOK,
            Queen => UNICODE_NEUTRAL_QUEEN,
            King => UNICODE_NEUTRAL_KING,
        }
    }

    fn to_default_utf8_char(self) -> char {
        ColoredChessPiece::new(Black, self).to_default_utf8_char()
    }

    /// Also parses German notation.
    fn from_ascii_char(c: char) -> Option<Self> {
        match c {
            ' ' => Some(Pawn),
            'N' | 'S' => Some(Knight),
            'B' | 'L' => Some(Bishop),
            'R' | 'T' => Some(Rook),
            'Q' | 'D' => Some(Queen),
            'K' => Some(King),
            _ => None,
        }
    }

    fn from_utf8_char(c: char) -> Option<Self> {
        match c {
            ' ' => Some(Empty),
            UNICODE_NEUTRAL_PAWN => Some(Pawn),
            UNICODE_NEUTRAL_KNIGHT => Some(Knight),
            UNICODE_NEUTRAL_BISHOP => Some(Bishop),
            UNICODE_NEUTRAL_ROOK => Some(Rook),
            UNICODE_NEUTRAL_QUEEN => Some(Queen),
            UNICODE_NEUTRAL_KING => Some(King),
            // it's normal to use white symbols as colorless symbols, so also support that
            UNICODE_WHITE_PAWN => Some(Pawn),
            UNICODE_WHITE_KNIGHT => Some(Knight),
            UNICODE_WHITE_BISHOP => Some(Bishop),
            UNICODE_WHITE_ROOK => Some(Rook),
            UNICODE_WHITE_QUEEN => Some(Queen),
            UNICODE_WHITE_KING => Some(King),
            _ => Self::from_ascii_char(c),
        }
    }

    fn to_uncolored_idx(self) -> usize {
        self as usize
    }
}

impl UncoloredPieceType for UncoloredChessPiece {
    type Colored = ColoredChessPiece;

    fn from_uncolored_idx(idx: usize) -> Self {
        // TODO: Might be unnecessarily slow? Test using a match instead.
        Self::iter().nth(idx).unwrap()
    }
}

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone, EnumIter, FromRepr)]
#[repr(usize)]
pub enum ColoredChessPiece {
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

impl ColoredChessPiece {
    pub fn pieces() -> impl Iterator<Item = ColoredChessPiece> {
        Self::iter().filter(|p| *p != ColoredChessPiece::Empty)
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
            self.uncolor().name()
        )
    }
}

impl Display for ColoredChessPiece {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_utf8_char())
    }
}

impl AbstractPieceType for ColoredChessPiece {
    fn empty() -> Self {
        Self::Empty
    }

    fn to_ascii_char(self) -> char {
        match self {
            ColoredChessPiece::Empty => '.',
            ColoredChessPiece::WhitePawn => 'P',
            ColoredChessPiece::WhiteKnight => 'N',
            ColoredChessPiece::WhiteBishop => 'B',
            ColoredChessPiece::WhiteRook => 'R',
            ColoredChessPiece::WhiteQueen => 'Q',
            ColoredChessPiece::WhiteKing => 'K',
            ColoredChessPiece::BlackPawn => 'p',
            ColoredChessPiece::BlackKnight => 'n',
            ColoredChessPiece::BlackBishop => 'b',
            ColoredChessPiece::BlackRook => 'r',
            ColoredChessPiece::BlackQueen => 'q',
            ColoredChessPiece::BlackKing => 'k',
        }
    }

    fn to_utf8_char(self) -> char {
        match self {
            ColoredChessPiece::Empty => '.',
            ColoredChessPiece::WhitePawn => UNICODE_WHITE_PAWN,
            ColoredChessPiece::WhiteKnight => UNICODE_WHITE_KNIGHT,
            ColoredChessPiece::WhiteBishop => UNICODE_WHITE_BISHOP,
            ColoredChessPiece::WhiteRook => UNICODE_WHITE_ROOK,
            ColoredChessPiece::WhiteQueen => UNICODE_WHITE_QUEEN,
            ColoredChessPiece::WhiteKing => UNICODE_WHITE_KING,
            ColoredChessPiece::BlackPawn => UNICODE_BLACK_PAWN,
            ColoredChessPiece::BlackKnight => UNICODE_BLACK_KNIGHT,
            ColoredChessPiece::BlackBishop => UNICODE_BLACK_BISHOP,
            ColoredChessPiece::BlackRook => UNICODE_BLACK_ROOK,
            ColoredChessPiece::BlackQueen => UNICODE_BLACK_QUEEN,
            ColoredChessPiece::BlackKing => UNICODE_BLACK_KING,
        }
    }

    fn to_default_utf8_char(self) -> char {
        if self == ColoredChessPiece::Empty {
            self.to_utf8_char()
        } else {
            ColoredChessPiece::new(Black, self.uncolor()).to_utf8_char()
        }
    }

    /// Also parses German notation (pawns are still represented as 'p' to avoid ambiguity with bishops).
    fn from_ascii_char(c: char) -> Option<Self> {
        match c {
            ' ' => Some(ColoredChessPiece::Empty),
            'P' => Some(ColoredChessPiece::WhitePawn),
            'N' | 'S' => Some(ColoredChessPiece::WhiteKnight),
            'B' | 'L' => Some(ColoredChessPiece::WhiteBishop),
            'R' | 'T' => Some(ColoredChessPiece::WhiteRook),
            'Q' | 'D' => Some(ColoredChessPiece::WhiteQueen),
            'K' => Some(ColoredChessPiece::WhiteKing),
            'p' => Some(ColoredChessPiece::BlackPawn),
            'n' | 's' => Some(ColoredChessPiece::BlackKnight),
            'b' | 'l' => Some(ColoredChessPiece::BlackBishop),
            'r' | 't' => Some(ColoredChessPiece::BlackRook),
            'q' | 'd' => Some(ColoredChessPiece::BlackQueen),
            'k' => Some(ColoredChessPiece::BlackKing),
            _ => None,
        }
    }

    fn from_utf8_char(c: char) -> Option<Self> {
        match c {
            ' ' => Some(ColoredChessPiece::Empty),
            UNICODE_WHITE_PAWN => Some(ColoredChessPiece::WhitePawn),
            UNICODE_WHITE_KNIGHT => Some(ColoredChessPiece::WhiteKnight),
            UNICODE_WHITE_BISHOP => Some(ColoredChessPiece::WhiteBishop),
            UNICODE_WHITE_ROOK => Some(ColoredChessPiece::WhiteRook),
            UNICODE_WHITE_QUEEN => Some(ColoredChessPiece::WhiteQueen),
            UNICODE_WHITE_KING => Some(ColoredChessPiece::WhiteKing),
            UNICODE_BLACK_PAWN => Some(ColoredChessPiece::BlackPawn),
            UNICODE_BLACK_KNIGHT => Some(ColoredChessPiece::BlackKnight),
            UNICODE_BLACK_BISHOP => Some(ColoredChessPiece::BlackBishop),
            UNICODE_BLACK_ROOK => Some(ColoredChessPiece::BlackRook),
            UNICODE_BLACK_QUEEN => Some(ColoredChessPiece::BlackQueen),
            UNICODE_BLACK_KING => Some(ColoredChessPiece::BlackKing),
            _ => Self::from_ascii_char(c),
        }
    }

    fn to_uncolored_idx(self) -> usize {
        self.to_colored_idx() % BLACK_OFFSET
    }
}

impl ColoredPieceType for ColoredChessPiece {
    type Uncolored = UncoloredChessPiece;

    fn color(self) -> Option<Color> {
        match self {
            ColoredChessPiece::Empty => None,
            x => Color::iter().nth((x as u8 / BlackPawn as u8) as usize),
        }
    }

    fn to_colored_idx(self) -> usize {
        self as usize
    }

    fn new(color: Color, uncolored: Self::Uncolored) -> Self {
        Self::from_repr((uncolored as usize) + (color as usize) * BLACK_OFFSET).unwrap()
    }
}

pub type ChessPiece = GenericPiece<ChessSquare, ColoredChessPiece>;
