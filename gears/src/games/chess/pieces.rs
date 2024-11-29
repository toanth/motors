use std::fmt::{Display, Formatter};

use itertools::Itertools;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, FromRepr};

use crate::games::chess::pieces::ChessPieceType::*;
use crate::games::chess::pieces::ColoredChessPieceType::BlackPawn;

use crate::games::chess::ChessColor::*;
use crate::games::chess::{ChessColor, Chessboard};
use crate::games::{AbstractPieceType, ColoredPieceType, GenericPiece, PieceType};

pub const NUM_CHESS_PIECES: usize = 6;
pub const NUM_COLORS: usize = 2;
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
    pub fn pieces() -> impl DoubleEndedIterator<Item = ChessPieceType> {
        Self::iter().dropping_back(1)
    }

    pub fn non_king_pieces() -> impl DoubleEndedIterator<Item = ChessPieceType> {
        Self::iter().dropping_back(2)
    }

    pub fn non_pawn_pieces() -> impl DoubleEndedIterator<Item = ChessPieceType> {
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

impl Display for ChessPieceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_utf8_char())
    }
}

impl AbstractPieceType for ChessPieceType {
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
        // The black pieces are often the prettiest and the easiest to recognize, though this depends very much on the font
        match self {
            Empty => '.',
            // Some fonts have problems with the black pawn for some reason, so use the white version to circumvent that
            Pawn => UNICODE_WHITE_PAWN,
            Knight => UNICODE_BLACK_KNIGHT,
            Bishop => UNICODE_BLACK_BISHOP,
            Rook => UNICODE_BLACK_ROOK,
            Queen => UNICODE_BLACK_QUEEN,
            King => UNICODE_BLACK_KING,
        }
    }

    fn to_default_utf8_char(self) -> char {
        ColoredChessPieceType::new(Black, self).to_default_utf8_char()
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
            // it's normal to use white symbols as colorless symbols, so also support that
            // And since we output the black pieces, we should definitely parse them, too
            UNICODE_NEUTRAL_PAWN | UNICODE_WHITE_PAWN | UNICODE_BLACK_PAWN => Some(Pawn),
            UNICODE_NEUTRAL_KNIGHT | UNICODE_WHITE_KNIGHT | UNICODE_BLACK_KNIGHT => Some(Knight),
            UNICODE_NEUTRAL_BISHOP | UNICODE_WHITE_BISHOP | UNICODE_BLACK_BISHOP => Some(Bishop),
            UNICODE_NEUTRAL_ROOK | UNICODE_WHITE_ROOK | UNICODE_BLACK_ROOK => Some(Rook),
            UNICODE_NEUTRAL_QUEEN | UNICODE_WHITE_QUEEN | UNICODE_BLACK_QUEEN => Some(Queen),
            UNICODE_NEUTRAL_KING | UNICODE_WHITE_KING | UNICODE_BLACK_KING => Some(King),
            _ => Self::from_ascii_char(c),
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

impl Display for ColoredChessPieceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_utf8_char())
    }
}

impl AbstractPieceType for ColoredChessPieceType {
    fn empty() -> Self {
        Self::Empty
    }

    fn to_ascii_char(self) -> char {
        match self {
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
        }
    }

    fn to_utf8_char(self) -> char {
        match self {
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
        }
    }

    fn to_default_utf8_char(self) -> char {
        if self == ColoredChessPieceType::Empty {
            self.to_utf8_char()
        } else {
            ColoredChessPieceType::new(Black, self.uncolor()).to_utf8_char()
        }
    }

    /// Also parses German notation (pawns are still represented as 'p' to avoid ambiguity with bishops).
    fn from_ascii_char(c: char) -> Option<Self> {
        match c {
            ' ' => Some(ColoredChessPieceType::Empty),
            'P' => Some(ColoredChessPieceType::WhitePawn),
            'N' | 'S' => Some(ColoredChessPieceType::WhiteKnight),
            'B' | 'L' => Some(ColoredChessPieceType::WhiteBishop),
            'R' | 'T' => Some(ColoredChessPieceType::WhiteRook),
            'Q' | 'D' => Some(ColoredChessPieceType::WhiteQueen),
            'K' => Some(ColoredChessPieceType::WhiteKing),
            'p' => Some(ColoredChessPieceType::BlackPawn),
            'n' | 's' => Some(ColoredChessPieceType::BlackKnight),
            'b' | 'l' => Some(ColoredChessPieceType::BlackBishop),
            'r' | 't' => Some(ColoredChessPieceType::BlackRook),
            'q' | 'd' => Some(ColoredChessPieceType::BlackQueen),
            'k' => Some(ColoredChessPieceType::BlackKing),
            _ => None,
        }
    }

    fn from_utf8_char(c: char) -> Option<Self> {
        match c {
            ' ' => Some(ColoredChessPieceType::Empty),
            UNICODE_WHITE_PAWN => Some(ColoredChessPieceType::WhitePawn),
            UNICODE_WHITE_KNIGHT => Some(ColoredChessPieceType::WhiteKnight),
            UNICODE_WHITE_BISHOP => Some(ColoredChessPieceType::WhiteBishop),
            UNICODE_WHITE_ROOK => Some(ColoredChessPieceType::WhiteRook),
            UNICODE_WHITE_QUEEN => Some(ColoredChessPieceType::WhiteQueen),
            UNICODE_WHITE_KING => Some(ColoredChessPieceType::WhiteKing),
            UNICODE_BLACK_PAWN => Some(ColoredChessPieceType::BlackPawn),
            UNICODE_BLACK_KNIGHT => Some(ColoredChessPieceType::BlackKnight),
            UNICODE_BLACK_BISHOP => Some(ColoredChessPieceType::BlackBishop),
            UNICODE_BLACK_ROOK => Some(ColoredChessPieceType::BlackRook),
            UNICODE_BLACK_QUEEN => Some(ColoredChessPieceType::BlackQueen),
            UNICODE_BLACK_KING => Some(ColoredChessPieceType::BlackKing),
            _ => Self::from_ascii_char(c),
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
            x => ChessColor::iter().nth((x as u8 / BlackPawn as u8) as usize),
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
