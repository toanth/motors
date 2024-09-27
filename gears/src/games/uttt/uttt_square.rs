/*
 *  Gears, a collection of board games.
 *  Copyright (C) 2024 ToTheAnd
 *
 *  Gears is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Gears is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Gears. If not, see <https://www.gnu.org/licenses/>.
 */

use crate::games::uttt::UtttSubSquare;
use crate::games::{Coordinates, DimT, Height, Size, Width};
use crate::general::squares::{
    RectangularCoordinates, RectangularSize, SmallGridSize, SmallGridSquare,
};
use arbitrary::Arbitrary;
use itertools::Itertools;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

/// The board isn't represented as a 9x9 grid but instead as a 3x3 grid of 3x3 grids, so we can't use
/// the more generic `SmallGridSize` type.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub struct UtttSize {}

impl Display for UtttSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        SmallGridSize::<9, 9>::default().fmt(f)
    }
}

impl Size<UtttSquare> for UtttSize {
    fn num_squares(self) -> usize {
        9 * 9
    }

    fn to_internal_key(self, coordinates: UtttSquare) -> usize {
        coordinates.bb_idx()
    }

    fn to_coordinates_unchecked(self, internal_key: usize) -> UtttSquare {
        UtttSquare::from_bb_idx(internal_key)
    }

    fn valid_coordinates(self) -> impl Iterator<Item = UtttSquare> {
        UtttSquare::iter()
    }

    fn coordinates_valid(self, coordinates: UtttSquare) -> bool {
        let size = SmallGridSize::<3, 3>::default();
        size.coordinates_valid(coordinates.sub_board)
            && size.coordinates_valid(coordinates.sub_square)
    }
}

impl RectangularSize<UtttSquare> for UtttSize {
    fn height(self) -> Height {
        Height(9)
    }

    fn width(self) -> Width {
        Width(9)
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub struct UtttSquare {
    sub_board: UtttSubSquare,
    sub_square: UtttSubSquare,
}

impl Display for UtttSquare {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // use `unchecked` because this function can be called to print invalid coordinates
        SmallGridSquare::<9, 9, 9>::unchecked((self.rank() * 9 + self.file()) as usize).fmt(f)
    }
}

impl FromStr for UtttSquare {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        SmallGridSquare::<9, 9, 9>::from_str(s).map(|c| Self::from_row_column(c.row(), c.column()))
    }
}

impl Coordinates for UtttSquare {
    type Size = UtttSize;

    fn flip_up_down(self, _size: Self::Size) -> Self {
        let size = SmallGridSize::default();
        Self {
            sub_board: self.sub_board.flip_up_down(size),
            sub_square: self.sub_square.flip_up_down(size),
        }
    }

    fn flip_left_right(self, _size: Self::Size) -> Self {
        let size = SmallGridSize::default();
        Self {
            sub_board: self.sub_board.flip_left_right(size),
            sub_square: self.sub_square.flip_left_right(size),
        }
    }

    fn no_coordinates() -> Self {
        Self {
            sub_board: SmallGridSquare::no_coordinates_const(),
            sub_square: SmallGridSquare::no_coordinates_const(),
        }
    }
}

impl RectangularCoordinates for UtttSquare {
    fn from_row_column(row: DimT, column: DimT) -> Self {
        let sub_board = SmallGridSquare::from_row_column(row / 3, column / 3);
        let sub_square = SmallGridSquare::from_row_column(row % 3, column % 3);
        Self {
            sub_board,
            sub_square,
        }
    }

    fn row(self) -> DimT {
        self.sub_square.row() + self.sub_board.row() * 3
    }

    fn column(self) -> DimT {
        self.sub_square.column() + self.sub_board.column() * 3
    }
}

impl UtttSquare {
    pub fn new(sub_board: UtttSubSquare, sub_square: UtttSubSquare) -> Self {
        Self {
            sub_board,
            sub_square,
        }
    }

    #[must_use]
    pub fn bb_idx(self) -> usize {
        let sub_square = self.sub_square.bb_idx();
        let sub_board = self.sub_board.bb_idx();
        debug_assert!(sub_square < 9);
        debug_assert!(sub_board < 9);
        sub_board * 9 + sub_square
    }

    pub fn to_u8(self) -> u8 {
        self.sub_square.to_u8() + self.sub_board.to_u8() * 9
    }

    pub fn from_bb_idx(idx: usize) -> Self {
        let sub_board_idx = idx / 9;
        let sub_square_idx = idx % 9;
        Self {
            sub_board: SmallGridSquare::from_bb_index(sub_board_idx),
            sub_square: SmallGridSquare::from_bb_index(sub_square_idx),
        }
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        (0..9).cartesian_product(0..9).map(|(a, b)| Self {
            sub_board: SmallGridSquare::from_bb_index(a),
            sub_square: SmallGridSquare::from_bb_index(b),
        })
    }

    pub fn sub_board(self) -> UtttSubSquare {
        self.sub_board
    }

    pub fn sub_square(self) -> UtttSubSquare {
        self.sub_square
    }

    pub const fn no_coordinates_const() -> Self {
        Self {
            sub_board: SmallGridSquare::no_coordinates_const(),
            sub_square: SmallGridSquare::no_coordinates_const(),
        }
    }
}
