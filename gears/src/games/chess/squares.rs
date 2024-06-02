use std::fmt::Display;

use crate::games::DimT;
use crate::general::squares::{SmallGridSize, SmallGridSquare};

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

pub type ChessboardSize = SmallGridSize<8, 8>;

pub enum SquareColor {
    White,
    Black,
}

pub type ChessSquare = SmallGridSquare<8, 8>;
