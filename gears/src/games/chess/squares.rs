use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::games::{
    Coordinates, DimT, GridCoordinates, GridSize, Height, RectangularCoordinates, RectangularSize,
    Size, Width,
};
use crate::general::bitboards::chess::ChessBitboard;
use crate::general::common::Res;

pub const NUM_ROWS: usize = 8;
pub const NUM_COLUMNS: usize = 8;
pub const NUM_SQUARES: usize = NUM_ROWS * NUM_COLUMNS;
pub const A_FILE_NO: DimT = 0;
pub const B_FILE_NO: DimT = 1;
pub const C_FILE_NO: DimT = 2;
pub const D_FILE_NO: DimT = 3;
pub const E_FILE_NO: DimT = 4;
pub const F_FILE_NO: DimT = 5;
pub const G_FILE_NO: DimT = 6;
pub const H_FILE_NO: DimT = 7;

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone)]
pub struct ChessboardSize {}

impl Display for ChessboardSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "8x8")
    }
}

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

    fn coordinates_valid(self, coordinates: ChessSquare) -> bool {
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
    pub const fn new(idx: usize) -> Self {
        debug_assert!(idx < NUM_SQUARES);
        Self { idx: idx as u8 }
    }

    pub fn unchecked(idx: usize) -> Self {
        Self { idx: idx as u8 }
    }

    pub const fn from_coordinates(c: GridCoordinates) -> Self {
        Self::new(c.row as usize * NUM_COLUMNS + c.column as usize)
    }

    pub const fn from_rank_file(rank: DimT, file: DimT) -> Self {
        Self::from_coordinates(GridCoordinates {
            row: rank,
            column: file,
        })
    }

    pub fn from_chars(file: char, rank: char) -> Res<Self> {
        GridCoordinates::algebraic_coordinate(
            file,
            rank.to_digit(10)
                .ok_or_else(|| format!("the rank is '{rank}', which is not a digit"))?
                as usize,
        )
        .and_then(|c| GridSize::chess().check_coordinates(c))
        .map(Self::from_coordinates)
    }

    pub fn to_grid_coordinates(self) -> GridCoordinates {
        GridCoordinates {
            row: self.row(),
            column: self.column(),
        }
    }

    pub fn bb(self) -> ChessBitboard {
        ChessBitboard::single_piece(self.index())
    }

    pub fn index(self) -> usize {
        self.idx as usize
    }

    pub fn rank(self) -> DimT {
        self.row()
    }

    pub fn file(self) -> DimT {
        self.column()
    }

    pub fn flip(self) -> Self {
        self.flip_up_down(ChessboardSize::default())
    }

    pub fn flip_if(self, flip: bool) -> Self {
        match flip {
            true => self.flip(),
            false => self,
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

    pub fn is_backrank(self) -> bool {
        let rank = self.rank();
        rank == 0 || rank == 7
    }
}

impl FromStr for ChessSquare {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        GridCoordinates::from_str(s)
            .and_then(|c| GridSize::chess().check_coordinates(c))
            .map(Self::from_coordinates)
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
    fn from_row_column(row: DimT, column: DimT) -> Self {
        Self::from_rank_file(row, column)
    }

    fn row(self) -> DimT {
        self.idx / 8
    }

    fn column(self) -> DimT {
        self.idx % 8
    }
}
