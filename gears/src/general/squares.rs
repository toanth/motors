use anyhow::{anyhow, bail};
use arbitrary::Arbitrary;
use colored::Colorize;
use std::cmp::max;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use crate::games::chess::ChessColor;
use crate::games::{char_to_file, file_to_char, Coordinates, DimT, Height, Size, Width};
use crate::general::bitboards::chess::ChessBitboard;
use crate::general::common::{parse_int, Res};

pub trait RectangularCoordinates: Coordinates<Size: RectangularSize<Self>> {
    fn from_row_column(row: DimT, column: DimT) -> Self;
    fn row(self) -> DimT;
    fn column(self) -> DimT;
    fn rank(self) -> DimT {
        self.row()
    }
    fn file(self) -> DimT {
        self.column()
    }
}

// Computes the L1 norm of a - b
pub fn manhattan_distance<C: RectangularCoordinates>(a: C, b: C) -> usize {
    a.row().abs_diff(b.row()) as usize + a.column().abs_diff(b.column()) as usize
}

// Compute the supremum norm of a - b
pub fn sup_distance<C: RectangularCoordinates>(a: C, b: C) -> usize {
    max(a.row().abs_diff(b.row()), a.column().abs_diff(b.column())) as usize
}

#[derive(Clone, Copy, Eq, PartialOrd, PartialEq, Debug, Default, Hash, Arbitrary)]
#[must_use]
pub struct GridCoordinates {
    pub row: DimT,
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
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = s.trim().chars();

        let Some(file) = s.next() else {
            bail!("Empty input")
        };
        let mut words = s.as_str().split_whitespace().peekable();
        let rank: usize = parse_int(&mut words, "rank (row)")?;
        if words.count() > 0 {
            bail!("too many words".to_string());
        }
        Self::algebraic_coordinate(file, rank)
    }
}

impl Display for GridCoordinates {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if *self == Self::no_coordinates() {
            write!(f, "<invalid>")
        } else {
            let file = if self.column < 26 {
                file_to_char(self.column)
            } else {
                '?'
            };
            write!(
                f,
                "{0}{1}",
                file,
                // output 1-indexed, convert to usize to prevent overflow (this function can be called on invalid coordinates)
                self.row as usize + 1
            )
        }
    }
}

impl GridCoordinates {
    pub fn algebraic_coordinate(file: char, rank: usize) -> Res<Self> {
        if !file.is_ascii_alphabetic() {
            bail!(
                "file (column) '{}' must be a valid ascii letter",
                file.to_string().red()
            );
        }
        let column = char_to_file(file.to_ascii_lowercase());
        let rank = DimT::try_from(rank)?;
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

    fn idx_to_coordinates(&self, idx: DimT) -> C {
        C::from_row_column(idx / self.width().0, idx % self.width().0)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Arbitrary)]
#[must_use]
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

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone, Arbitrary)]
#[must_use]
pub struct SmallGridSize<const H: usize, const W: usize> {}

impl<const H: usize, const W: usize> Display for SmallGridSize<H, W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{H}x{W}")
    }
}

impl<const H: usize, const W: usize, const INTERNAL_WIDTH: usize>
    Size<SmallGridSquare<H, W, INTERNAL_WIDTH>> for SmallGridSize<H, W>
{
    fn num_squares(self) -> usize {
        H * W
    }

    fn to_internal_key(self, coordinates: SmallGridSquare<H, W, INTERNAL_WIDTH>) -> usize {
        coordinates.bb_idx()
    }

    fn to_coordinates_unchecked(
        self,
        internal_key: usize,
    ) -> SmallGridSquare<H, W, INTERNAL_WIDTH> {
        SmallGridSquare::unchecked(internal_key)
    }

    fn valid_coordinates(self) -> impl Iterator<Item = SmallGridSquare<H, W, INTERNAL_WIDTH>> {
        SmallGridSquare::iter()
    }

    fn coordinates_valid(self, coordinates: SmallGridSquare<H, W, INTERNAL_WIDTH>) -> bool {
        (coordinates.idx as usize) < H * INTERNAL_WIDTH && coordinates.file() < W as DimT
    }
}

impl<const H: usize, const W: usize, const INTERNAL_WIDTH: usize>
    RectangularSize<SmallGridSquare<H, W, INTERNAL_WIDTH>> for SmallGridSize<H, W>
{
    fn height(self) -> Height {
        Height(H as u8)
    }

    fn width(self) -> Width {
        Width(W as u8)
    }

    fn internal_width(self) -> usize {
        INTERNAL_WIDTH
    }
}

#[derive(Debug, Copy, Clone)]
pub enum SquareColor {
    White,
    Black,
}

// Ideally, there would be an alias setting `INTERNAL_WIDTH` or a default parameter for `INTERNAL_WIDTH` to `max(8, W)`,
// but both of those things aren't possible in stale Rust.
#[derive(Default, Debug, Eq, PartialEq, Copy, Clone, Hash, Arbitrary)]
#[must_use]
pub struct SmallGridSquare<const H: usize, const W: usize, const INTERNAL_WIDTH: usize> {
    idx: u8,
}

impl<const H: usize, const W: usize, const INTERNAL_WIDTH: usize> FromStr
    for SmallGridSquare<H, W, INTERNAL_WIDTH>
{
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        GridCoordinates::from_str(s)
            .and_then(|c| GridSize::new(Height::new(H), Width::new(H)).check_coordinates(c))
            .map(Self::from_coordinates)
    }
}

impl<const H: usize, const W: usize, const INTERNAL_WIDTH: usize>
    SmallGridSquare<H, W, INTERNAL_WIDTH>
{
    // In the future, it might make sense to relax those constraints
    const MAX_W: usize = 25; // there are 26 letters in the alphabet, so this ensures a simple 1-based textual representation
    const MAX_H: usize = 35; // the maximum radix for from_char is 36, so this ensures valid heights are between 1 and 36

    const UP_DOWN_MASK: DimT = ((1 << H.ilog2()) - 1) << INTERNAL_WIDTH.ilog2();
    const LEFT_RIGHT_MASK: DimT = (1 << W.ilog2()) - 1;

    pub const fn from_bb_index(idx: usize) -> Self {
        assert!(H <= Self::MAX_H);
        assert!(W <= Self::MAX_W);
        assert!(H * W <= DimT::MAX as usize); // `<=` because invalid coordinates have to be representable
        debug_assert!(idx % INTERNAL_WIDTH < W);
        debug_assert!(idx / INTERNAL_WIDTH < H);
        Self { idx: idx as u8 }
    }

    pub const fn unchecked(idx: usize) -> Self {
        Self { idx: idx as u8 }
    }

    pub const fn from_coordinates(c: GridCoordinates) -> Self {
        Self::from_bb_index(c.row as usize * INTERNAL_WIDTH + c.column as usize)
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
            // + 1 because the rank number uses 1-based indices
            rank.to_digit(H as u32 + 1).ok_or_else(|| {
                anyhow!("the rank is '{rank}', which does not represent a number between 1 and {H} (the height)")
            })? as usize,
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

    // TODO: Don't return a ChessBitboard
    pub fn bb(self) -> ChessBitboard {
        ChessBitboard::single_piece(self.bb_idx())
    }

    /// Note that this isn't necessarily consecutive because the bitboard assumes a width of at least 8 for efficiency reasons.
    pub fn bb_idx(self) -> usize {
        self.idx as usize
    }

    pub fn flip(self) -> Self {
        self.flip_up_down(SmallGridSize::default())
    }

    pub fn flip_if(self, flip: bool) -> Self {
        if flip {
            self.flip()
        } else {
            self
        }
    }

    pub fn north_unchecked(self) -> Self {
        debug_assert_ne!(self.rank() as usize, H - 1);
        Self::unchecked(self.bb_idx() + INTERNAL_WIDTH)
    }

    pub fn south_unchecked(self) -> Self {
        debug_assert_ne!(self.rank(), 0);
        Self::unchecked(self.bb_idx() - INTERNAL_WIDTH)
    }

    pub fn east_unchecked(self) -> Self {
        debug_assert_ne!(self.file() as usize, W - 1);
        Self::unchecked(self.bb_idx() + 1)
    }

    pub fn west_unchecked(self) -> Self {
        debug_assert_ne!(self.file(), 0);
        Self::unchecked(self.bb_idx() - 1)
    }

    pub fn pawn_advance_unchecked(self, color: ChessColor) -> Self {
        match color {
            ChessColor::White => self.north_unchecked(),
            ChessColor::Black => self.south_unchecked(),
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
            .flat_map(|i| (INTERNAL_WIDTH * i)..(INTERNAL_WIDTH * i + W))
            .map(Self::from_bb_index)
    }

    pub const fn no_coordinates_const() -> Self {
        Self::unchecked(H * INTERNAL_WIDTH)
    }
}

impl<const H: usize, const W: usize, const INTERNAL_WIDTH: usize> Display
    for SmallGridSquare<H, W, INTERNAL_WIDTH>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.to_grid_coordinates().fmt(f)
    }
}

impl<const H: usize, const W: usize, const INTERNAL_WIDTH: usize> Coordinates
    for SmallGridSquare<H, W, INTERNAL_WIDTH>
{
    type Size = SmallGridSize<H, W>;

    fn flip_up_down(self, _: Self::Size) -> Self {
        // hopefully, this `if` and the constant will be evaluated at compile time
        if H.is_power_of_two() && INTERNAL_WIDTH.is_power_of_two() {
            Self {
                idx: self.idx ^ Self::UP_DOWN_MASK,
            }
        } else {
            Self::from_rank_file(H as DimT - 1 - self.rank(), self.file())
        }
    }

    fn flip_left_right(self, _: Self::Size) -> Self {
        if W.is_power_of_two() {
            Self {
                idx: self.idx ^ Self::LEFT_RIGHT_MASK,
            }
        } else {
            Self::from_rank_file(self.rank(), W as DimT - 1 - self.file())
        }
    }

    fn no_coordinates() -> Self {
        Self::unchecked(H * INTERNAL_WIDTH)
    }
}

impl<const H: usize, const W: usize, const INTERNAL_WIDTH: usize> RectangularCoordinates
    for SmallGridSquare<H, W, INTERNAL_WIDTH>
{
    fn from_row_column(row: DimT, column: DimT) -> Self {
        Self::from_rank_file(row, column)
    }

    fn row(self) -> DimT {
        self.idx / INTERNAL_WIDTH as DimT
    }

    fn column(self) -> DimT {
        self.idx % INTERNAL_WIDTH as DimT
    }
}
