use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::games::{
    Coordinates, GridCoordinates, Height, RectangularCoordinates, RectangularSize, Size, Width,
};

pub(super) const NUM_ROWS: usize = 8;
pub(super) const NUM_COLUMNS: usize = 8;
pub(super) const NUM_SQUARES: usize = NUM_ROWS * NUM_COLUMNS;
pub(super) const A_FILE_NO: usize = 0;
pub(super) const B_FILE_NO: usize = 1;
pub(super) const C_FILE_NO: usize = 2;
pub(super) const D_FILE_NO: usize = 3;
pub(super) const E_FILE_NO: usize = 4;
pub(super) const F_FILE_NO: usize = 5;
pub(super) const G_FILE_NO: usize = 6;
pub(super) const H_FILE_NO: usize = 7;

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone)]
pub struct ChessboardSize {}

impl Size<ChessSquare> for ChessboardSize {
    fn num_squares(self) -> usize {
        NUM_SQUARES
    }

    fn to_idx(self, coordinates: ChessSquare) -> usize {
        coordinates.index()
    }

    fn to_coordinates(self, idx: usize) -> ChessSquare {
        ChessSquare { idx: idx as u8 }
    }

    fn valid_coordinates(self, coordinates: ChessSquare) -> bool {
        coordinates.idx < NUM_SQUARES as u8
    }
}

impl RectangularSize<ChessSquare> for ChessboardSize {
    fn height(self) -> Height {
        Height(8)
    }

    fn width(self) -> Width {
        Width(8)
    }
}

pub enum SquareColor {
    White,
    Black,
}

#[derive(Default, Debug, Eq, PartialEq, Copy, Clone)]
pub struct ChessSquare {
    idx: u8,
}

impl ChessSquare {
    pub fn to_grid_coordinates(self) -> GridCoordinates {
        GridCoordinates {
            row: self.row(),
            column: self.column(),
        }
    }

    pub fn index(self) -> usize {
        self.idx as usize
    }

    pub const fn new(idx: usize) -> Self {
        debug_assert!(idx < NUM_SQUARES);
        Self { idx: idx as u8 }
    }

    pub fn unchecked(idx: usize) -> Self {
        Self { idx: idx as u8 }
    }

    pub const fn from_coordinates(c: GridCoordinates) -> Self {
        Self::new(c.row * NUM_COLUMNS + c.column)
    }

    pub const fn from_rank_file(rank: usize, file: usize) -> Self {
        Self::from_coordinates(GridCoordinates {
            row: rank,
            column: file,
        })
    }

    pub fn rank(self) -> usize {
        self.row()
    }

    pub fn file(self) -> usize {
        self.column()
    }

    pub fn square_color(self) -> SquareColor {
        if self.idx % 2 == 0 {
            SquareColor::Black
        } else {
            SquareColor::White
        }
    }

    pub fn north(self) -> ChessSquare {
        debug_assert_ne!(self.rank(), 7);
        Self::new(self.index() + 8)
    }

    pub fn south(self) -> ChessSquare {
        debug_assert_ne!(self.rank(), 0);
        Self::new(self.index() - 8)
    }

    pub fn east(self) -> ChessSquare {
        debug_assert_ne!(self.file(), H_FILE_NO);
        Self::new(self.index() + 1)
    }

    pub fn west(self) -> ChessSquare {
        debug_assert_ne!(self.file(), A_FILE_NO);
        Self::new(self.index() - 1)
    }

    pub fn pawn_move_to_center(self) -> ChessSquare {
        debug_assert_ne!(self.rank() % 7, 0);
        if self.rank() < 4 {
            self.north()
        } else {
            self.south()
        }
    }

    // pub fn invalid() -> Self {
    //     Self {idx: NUM_SQUARES as u8 }
    // }
}

impl FromStr for ChessSquare {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        GridCoordinates::from_str(s).and_then(|c| {
            if c.row >= NUM_ROWS || c.column >= NUM_COLUMNS {
                Err(format!("'{s}' lies outside of a chess board"))
            } else {
                Ok(Self {
                    idx: (c.row * 8 + c.column) as u8,
                })
            }
        })
    }
}

impl Display for ChessSquare {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.to_grid_coordinates().fmt(f)
    }
}

impl Coordinates for ChessSquare {
    type Size = ChessboardSize;

    fn flip_up_down(self, _: Self::Size) -> Self {
        Self {
            idx: self.idx ^ 0b111_000,
        }
    }

    fn flip_left_right(self, _: Self::Size) -> Self {
        Self {
            idx: self.idx ^ 0b000_111,
        }
    }

    fn no_coordinates() -> Self {
        Self::unchecked(64)
    }
}

impl RectangularCoordinates for ChessSquare {
    fn from_row_column(row: usize, column: usize) -> Self {
        Self::from_rank_file(row, column)
    }

    fn row(self) -> usize {
        (self.idx / 8) as usize
    }

    fn column(self) -> usize {
        (self.idx % 8) as usize
    }
}
