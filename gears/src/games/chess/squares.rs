use crate::games::DimT;
use crate::general::squares::{SmallGridSize, SmallGridSquare};
use std::str::FromStr;

pub const NUM_ROWS: usize = 8;
pub const NUM_COLUMNS: usize = 8;
pub const NUM_SQUARES: usize = NUM_ROWS * NUM_COLUMNS;
pub const A_FILE_NUM: DimT = 0;
pub const B_FILE_NUM: DimT = 1;
pub const C_FILE_NUM: DimT = 2;
pub const D_FILE_NUM: DimT = 3;
pub const E_FILE_NUM: DimT = 4;
pub const F_FILE_NUM: DimT = 5;
pub const G_FILE_NUM: DimT = 6;
pub const H_FILE_NUM: DimT = 7;

pub type ChessboardSize = SmallGridSize<8, 8>;

pub fn sq(text: &str) -> ChessSquare {
    ChessSquare::from_str(text).unwrap()
}

pub type ChessSquare = SmallGridSquare<8, 8, 8>;
