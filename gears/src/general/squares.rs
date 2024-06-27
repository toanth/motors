use std::cmp::max;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::games::{char_to_file, file_to_char, Color, Coordinates, DimT, Height, Size, Width};
use crate::general::bitboards::chess::ChessBitboard;
use crate::general::common::{parse_int, Res};

pub trait RectangularCoordinates: Coordinates {
    fn from_row_column(row: DimT, column: DimT) -> Self;
    fn row(self) -> DimT;
    fn column(self) -> DimT;
}

// Computes the L1 norm of a - b
pub fn manhattan_distance<C: RectangularCoordinates>(a: C, b: C) -> usize {
    a.row().abs_diff(b.row()) as usize + a.column().abs_diff(b.column()) as usize
}

// Compute the supremum norm of a - b
pub fn sup_distance<C: RectangularCoordinates>(a: C, b: C) -> usize {
    max(a.row().abs_diff(b.row()), a.column().abs_diff(b.column())) as usize
}

#[derive(Clone, Copy, Eq, PartialOrd, PartialEq, Debug, Default)]
pub struct GridCoordinates {
    pub row: DimT, // TODO: Store only one int
    pub column: DimT,
}

impl Coordinates for GridCoordinates {
    type Size = GridSize;

    fn flip_up_down(self, size: Self::Size) -> Self {
        GridCoordinates {
            row: size.height.0 - 1 - self.row,
            column: self.column,
        }
    }

    fn flip_left_right(self, size: Self::Size) -> Self {
        GridCoordinates {
            row: self.row,
            column: size.width.0 - 1 - self.column,
        }
    }

    fn no_coordinates() -> Self {
        GridCoordinates {
            row: DimT::MAX,
            column: DimT::MAX,
        }
    }
}

impl RectangularCoordinates for GridCoordinates {
    fn from_row_column(row: DimT, column: DimT) -> Self {
        GridCoordinates { row, column }
    }

    fn row(self) -> DimT {
        self.row
    }

    fn column(self) -> DimT {
        self.column
    }
}

impl FromStr for GridCoordinates {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = s.trim().chars();

        let file = s.next().ok_or("Empty input")?;
        let mut words = s.as_str().split_whitespace();
        let rank: usize = parse_int(&mut words, "rank (row)")?;
        if words.count() > 0 {
            return Err("too many words".to_string());
        }
        Self::algebraic_coordinate(file, rank)
    }
}

impl Display for GridCoordinates {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{0}{1}",
            file_to_char(self.column),
            self.row + 1 // output 1-indexed
        )
    }
}

impl GridCoordinates {
    pub fn algebraic_coordinate(file: char, rank: usize) -> Res<Self> {
        if !file.is_ascii_alphabetic() {
            return Err("file (column) must be a valid ascii letter".to_string());
        }
        let column = char_to_file(file.to_ascii_lowercase());
        let rank = DimT::try_from(rank).map_err(|err| err.to_string())?;
        Ok(GridCoordinates {
            column,
            row: rank.wrapping_sub(1),
        })
    }
}

pub trait RectangularSize<C: RectangularCoordinates>: Size<C> {
    fn height(self) -> Height;
    fn width(self) -> Width;
    fn internal_width(self) -> usize {
        self.width().val()
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct GridSize {
    pub height: Height,
    pub width: Width,
}

impl Display for GridSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{0}x{1}", self.height.0, self.width.0)
    }
}

impl GridSize {
    pub const fn new(height: Height, width: Width) -> Self {
        Self { height, width }
    }

    pub const fn chess() -> Self {
        Self::new(Height(8), Width(8))
    }

    pub fn ataxx() -> Self {
        Self::new(Height(7), Width(7))
    }

    pub const fn tictactoe() -> Self {
        Self::new(Height(3), Width(3))
    }

    pub const fn connect4() -> Self {
        Self::new(Height(6), Width(7))
    }
}

impl Size<GridCoordinates> for GridSize {
    fn num_squares(self) -> usize {
        self.height.val() * self.width.val()
    }

    fn to_internal_key(self, coordinates: GridCoordinates) -> usize {
        coordinates.row() as usize * self.width.val() + coordinates.column() as usize
    }

    fn to_coordinates_unchecked(self, internal_key: usize) -> GridCoordinates {
        GridCoordinates {
            // TODO: Handle overflows?
            row: (internal_key / self.width.val()) as DimT,
            column: (internal_key % self.width.val()) as DimT,
        }
    }

    fn valid_coordinates(self) -> impl Iterator<Item = GridCoordinates> {
        (0..self.num_squares()).map(move |i| self.to_coordinates_unchecked(i))
    }

    fn coordinates_valid(self, coordinates: GridCoordinates) -> bool {
        coordinates.row() < self.height().0 && coordinates.column() < self.width().0
    }
}

impl RectangularSize<GridCoordinates> for GridSize {
    fn height(self) -> Height {
        self.height
    }

    fn width(self) -> Width {
        self.width
    }
}

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone)]
pub struct SmallGridSize<const H: usize, const W: usize> {}

impl<const H: usize, const W: usize> Display for SmallGridSize<H, W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{H}x{W}")
    }
}

impl<const H: usize, const W: usize> Size<SmallGridSquare<H, W>> for SmallGridSize<H, W> {
    fn num_squares(self) -> usize {
        H * W
    }

    fn to_internal_key(self, coordinates: SmallGridSquare<H, W>) -> usize {
        coordinates.bb_idx()
    }

    fn to_coordinates_unchecked(self, internal_key: usize) -> SmallGridSquare<H, W> {
        SmallGridSquare::unchecked(internal_key)
    }

    fn valid_coordinates(self) -> impl Iterator<Item = SmallGridSquare<H, W>> {
        SmallGridSquare::iter()
    }

    fn coordinates_valid(self, coordinates: SmallGridSquare<H, W>) -> bool {
        (coordinates.idx as usize) < H * 8 && coordinates.file() < W as DimT
    }
}

impl<const H: usize, const W: usize> RectangularSize<SmallGridSquare<H, W>>
    for SmallGridSize<H, W>
{
    fn height(self) -> Height {
        Height(H as u8)
    }

    fn width(self) -> Width {
        Width(W as u8)
    }

    fn internal_width(self) -> usize {
        8
    }
}

pub enum SquareColor {
    White,
    Black,
}

#[derive(Default, Debug, Eq, PartialEq, Copy, Clone)]
pub struct SmallGridSquare<const H: usize, const W: usize> {
    idx: u8,
}

impl<const H: usize, const W: usize> FromStr for SmallGridSquare<H, W> {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        GridCoordinates::from_str(s)
            .and_then(|c| GridSize::chess().check_coordinates(c))
            .map(Self::from_coordinates)
    }
}

impl<const H: usize, const W: usize> SmallGridSquare<H, W> {
    pub const fn from_bb_index(idx: usize) -> Self {
        assert!(W <= 8);
        assert!(H <= 8);
        debug_assert!(idx % 8 < W);
        debug_assert!(idx / 8 < H);
        Self { idx: idx as u8 }
    }

    pub fn unchecked(idx: usize) -> Self {
        Self { idx: idx as u8 }
    }

    pub const fn from_coordinates(c: GridCoordinates) -> Self {
        Self::from_bb_index(c.row as usize * 8 + c.column as usize)
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
        .and_then(|c| GridSize::new(Height(H as DimT), Width(W as DimT)).check_coordinates(c))
        .map(Self::from_coordinates)
    }

    pub fn to_grid_coordinates(self) -> GridCoordinates {
        GridCoordinates {
            row: self.row(),
            column: self.column(),
        }
    }

    pub fn to_u8(self) -> u8 {
        self.idx
    }

    pub fn bb(self) -> ChessBitboard {
        ChessBitboard::single_piece(self.bb_idx())
    }

    /// Note that this isn't necessarily consecutive because the bitboard assumes a 8x8 board for efficiency reasons.
    pub fn bb_idx(self) -> usize {
        self.idx as usize
    }

    pub fn rank(self) -> DimT {
        self.row()
    }

    pub fn file(self) -> DimT {
        self.column()
    }

    pub fn flip(self) -> Self {
        self.flip_up_down(SmallGridSize::default())
    }

    pub fn flip_if(self, flip: bool) -> Self {
        match flip {
            true => self.flip(),
            false => self,
        }
    }

    pub fn north_unchecked(self) -> Self {
        debug_assert_ne!(self.rank() as usize, H - 1);
        Self::unchecked(self.bb_idx() + 8)
    }

    pub fn south_unchecked(self) -> Self {
        debug_assert_ne!(self.rank(), 0);
        Self::unchecked(self.bb_idx() - 8)
    }

    pub fn east_unchecked(self) -> Self {
        debug_assert_ne!(self.file() as usize, W - 1);
        Self::unchecked(self.bb_idx() + 1)
    }

    pub fn west_unchecked(self) -> Self {
        debug_assert_ne!(self.file(), 0);
        Self::unchecked(self.bb_idx() - 1)
    }

    pub fn pawn_advance_unchecked(self, color: Color) -> Self {
        match color {
            Color::White => self.north_unchecked(),
            Color::Black => self.south_unchecked(),
        }
    }

    pub fn is_backrank(self) -> bool {
        let rank = self.rank();
        rank == 0 || rank == H as DimT - 1
    }

    pub fn is_pawn_start_rank(self) -> bool {
        let rank = self.rank();
        rank == 1 || rank == H as DimT - 2
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        (0..H)
            .flat_map(|i| (8 * i)..(8 * i + W))
            .map(Self::from_bb_index)
    }
}

impl<const H: usize, const W: usize> Display for SmallGridSquare<H, W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.to_grid_coordinates().fmt(f)
    }
}

impl<const H: usize, const W: usize> Coordinates for SmallGridSquare<H, W> {
    type Size = SmallGridSize<H, W>;

    fn flip_up_down(self, _: Self::Size) -> Self {
        // hopefully, this `if` will be evaluated at compile time
        if H == 8 {
            Self {
                idx: self.idx ^ 0b111_000,
            }
        } else {
            Self::from_rank_file(H as DimT - 1 - self.rank(), self.file())
        }
    }

    fn flip_left_right(self, _: Self::Size) -> Self {
        if W == 8 {
            Self {
                idx: self.idx ^ 0b000_111,
            }
        } else {
            Self::from_rank_file(self.rank(), W as DimT - 1 - self.file())
        }
    }

    fn no_coordinates() -> Self {
        Self::unchecked(64)
    }
}

impl<const H: usize, const W: usize> RectangularCoordinates for SmallGridSquare<H, W> {
    fn from_row_column(row: DimT, column: DimT) -> Self {
        Self::from_rank_file(row, column)
    }

    fn row(self) -> DimT {
        self.idx / 8 as DimT
    }

    fn column(self) -> DimT {
        self.idx % 8 as DimT
    }
}
