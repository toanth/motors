extern crate num;

use std::fmt::{Debug, Display, Formatter};
use std::num::Wrapping;
use std::ops::{Deref, DerefMut};

use derive_more::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, ShlAssign, Shr,
    ShrAssign, Sub,
};
use num::{One, PrimInt, Unsigned, Zero};
use strum_macros::EnumIter;

#[cfg(feature = "chess")]
use crate::games::chess::squares::ChessSquare;
#[cfg(feature = "chess")]
use crate::games::chess::squares::ChessboardSize;
use crate::games::{DimT, Size};
use crate::general::common::{pop_lsb128, pop_lsb64};
use crate::general::squares::{RectangularCoordinates, RectangularSize};

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Direction {
    Horizontal,
    Vertical,
    Diagonal,
    AntiDiagonal,
}

const fn compute_step_bbs() -> [u128; 128] {
    let mut res = [0; 128];
    res[0] = 1;
    let mut step = 1;
    while step < 128 {
        let mut i = 0;
        while i < 128 {
            res[step] |= 1 << i;
            i += step;
        }
        step += 1;
    }
    res
}

const fn compute_diagonal_bbs() -> [[u128; 128]; MAX_WIDTH] {
    let mut res: [[u128; 128]; MAX_WIDTH] = [[0; 128]; MAX_WIDTH];
    let mut width = 1;
    while width < MAX_WIDTH {
        // can't use for loops in const functions
        let mut i: usize = 0;
        while i < 128 {
            let diag = (i / width) as i32 - (i % width) as i32;
            if diag > 0 {
                let diag = diag as usize;
                res[width][i] = STEPS[width + 1] << (diag * width);
                // let diag = width as i32 - diag;
                // let diag = if diag >= 0 { diag as usize } else { 0 }; // max isn't const
                res[width][i] = remove_ones_above(res[width][i], (width + diag + 1) * width);
            } else {
                let diag = -diag as usize;
                res[width][i] = remove_ones_below(STEPS[width + 1] << diag, diag);
                res[width][i] = remove_ones_above(res[width][i], (width - diag) * width);
            }
            i += 1;
        }
        width += 1;
    }
    res
}

const fn compute_anti_diagonal_bbs() -> [[u128; 128]; MAX_WIDTH] {
    let mut res: [[u128; 128]; MAX_WIDTH] = [[0; 128]; MAX_WIDTH];
    let mut width = 1;
    while width < MAX_WIDTH {
        let mut i = 0;
        while i < 128 {
            let anti_diag = i / width + i % width;
            res[width][i] = remove_ones_above(STEPS[width - 1] << anti_diag, anti_diag * width);
            res[width][i] = remove_ones_below(
                res[width][i],
                if anti_diag >= width {
                    (anti_diag - width + 2) * width - 1
                } else {
                    0
                },
            );
            i += 1;
        }
        width += 1;
    }
    res
}
//
// const RANKS: [u128; 12] = compute_rank_bbs();
//
// const DIAGONALS: [[[u128; 23]; 12]; 12] = compute_all_diag_bbs(false);
//
// const ANTI_DIAGONALS: [[[u128; 23]; 12]; 12] = compute_all_diag_bbs(true);

// TODO: Store as array of structs? Probably best to change that when there is a working search
// to run a sprt, only increasing bench nps might not be worth it

const MAX_WIDTH: usize = 12;

const STEPS: [u128; 128] = compute_step_bbs();

const DIAGONALS: [[u128; 128]; MAX_WIDTH] = compute_diagonal_bbs();

const ANTI_DIAGONALS: [[u128; 128]; MAX_WIDTH] = compute_anti_diagonal_bbs();

// This seems like a lot of boilerplate code.
// Maybe there's a better way?
pub trait RawBitboard:
    Copy
    + Clone
    + Debug
    + Eq
    + PartialEq
    + Sub<Output = Self>
    + Not<Output = Self>
    + BitAnd<Output = Self>
    + BitAnd<usize>
    + BitAndAssign
    + BitOr<Output = Self>
    + BitOrAssign
    + BitXor<Output = Self>
    + BitXorAssign
    + Shl<usize, Output = Self>
    + ShlAssign<usize>
    + Shr<usize, Output = Self>
    + ShrAssign<usize>
{
    type Primitive: Unsigned + PrimInt;

    fn from_u128(val: u128) -> Self;

    fn from_primitive(val: Self::Primitive) -> Self;

    fn to_primitive(self) -> Self::Primitive;

    fn to_wrapped(self) -> Wrapping<Self::Primitive> {
        Wrapping(self.to_primitive())
    }

    /// Returns a bitboard where exactly the bits in the inclusive interval [low, high] are set.
    fn squares_between(low: Self, high: Self) -> Self {
        debug_assert!(low.is_single_piece());
        debug_assert!(high.is_single_piece());
        debug_assert!(low.trailing_zeros() <= high.trailing_zeros());
        ((high - Self::single_piece(0)) ^ (low - Self::single_piece(0))) | high
    }

    fn is_zero(self) -> bool {
        self.to_primitive() == Self::Primitive::zero()
    }

    // TODO: BitIter that returns indices of set bits.
    fn has_set_bit(self) -> bool {
        !self.is_zero()
    }

    fn pop_lsb(&mut self) -> usize;

    fn single_piece(idx: usize) -> Self {
        Self::from_primitive(Self::Primitive::one() << idx)
    }

    fn is_single_piece(self) -> bool;
    // apparently, the num crate doesn't provide a is_power_of_two() method

    fn is_bit_set_at(self, idx: usize) -> bool {
        ((self.to_primitive() >> idx) & Self::Primitive::one()) == Self::Primitive::one()
    }

    fn trailing_zeros(self) -> usize {
        self.to_primitive().trailing_zeros() as usize
    }

    fn num_ones(self) -> usize {
        self.to_primitive().count_ones() as usize
    }

    fn ones(self) -> BitIterator<Self> {
        BitIterator(self)
    }
}

pub struct BitIterator<B: RawBitboard>(B);

impl<B: RawBitboard> Iterator for BitIterator<B> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_zero() {
            None
        } else {
            Some(self.0.pop_lsb())
        }
    }
}

#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Default,
    Hash,
    Not,
    BitOr,
    BitOrAssign,
    BitAnd,
    BitAndAssign,
    BitXor,
    BitXorAssign,
    Shl,
    ShlAssign,
    Shr,
    ShrAssign,
)]
pub struct RawStandardBitboard(pub u64);

// TODO: Why are these methods not derived?

impl Sub for RawStandardBitboard {
    type Output = RawStandardBitboard;

    fn sub(self, rhs: Self) -> Self::Output {
        RawStandardBitboard(self.0.wrapping_sub(rhs.0))
    }
}

impl BitAnd<usize> for RawStandardBitboard {
    type Output = usize;

    fn bitand(self, rhs: usize) -> usize {
        (self.0 as usize).bitand(rhs)
    }
}

impl Debug for RawStandardBitboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Chess Bitboard {:#x}", self.0)
    }
}

impl RawBitboard for RawStandardBitboard {
    type Primitive = u64;

    fn from_u128(val: u128) -> Self {
        // we rely on the truncating behavior of `as`in several places
        // debug_assert!(val <= u64::MAX as u128);
        Self(val as u64)
    }

    fn from_primitive(val: Self::Primitive) -> Self {
        Self(val)
    }

    fn to_primitive(self) -> Self::Primitive {
        self.0
    }

    fn pop_lsb(&mut self) -> usize {
        pop_lsb64(&mut self.0) as usize
    }

    fn is_single_piece(self) -> bool {
        self.0.is_power_of_two()
    }
}

#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Default,
    Not,
    BitOr,
    BitOrAssign,
    BitAnd,
    BitAndAssign,
    BitXor,
    BitXorAssign,
    Shl,
    ShlAssign,
    Shr,
    ShrAssign,
)]
pub struct ExtendedRawBitboard(pub u128);

impl Debug for ExtendedRawBitboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Extended Bitboard {:#x}", self.0)
    }
}

impl Sub for ExtendedRawBitboard {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_sub(rhs.0))
    }
}

impl BitAnd<usize> for ExtendedRawBitboard {
    type Output = usize;

    fn bitand(self, rhs: usize) -> Self::Output {
        (self.0 as usize).bitand(rhs)
    }
}

impl RawBitboard for ExtendedRawBitboard {
    type Primitive = u128;

    fn from_u128(val: u128) -> Self {
        Self(val)
    }

    fn from_primitive(val: Self::Primitive) -> Self {
        Self(val)
    }

    fn to_primitive(self) -> Self::Primitive {
        self.0
    }

    fn pop_lsb(&mut self) -> usize {
        pop_lsb128(&mut self.0) as usize
    }

    fn is_single_piece(self) -> bool {
        self.0.is_power_of_two()
    }
}

#[derive(Debug, EnumIter)]
pub enum RayDirections {
    Horizontal,
    Vertical,
    Diagonal,
    AntiDiagonal,
}

pub trait Bitboard<R: RawBitboard, C: RectangularCoordinates>:
    Copy
    + Clone
    + Debug
    + Eq
    + PartialEq
    + Sub<Output = Self>
    + Not<Output = Self>
    + BitAnd<Output = Self>
    // + BitAnd<usize>
    + BitAndAssign
    + BitOr<Output = Self>
    + BitOrAssign
    + BitXor<Output = Self>
    + BitXorAssign
    + Shl<usize, Output = Self>
    + ShlAssign<usize>
    + Shr<usize, Output = Self>
    + ShrAssign<usize>
    + Deref<Target = R>
    + DerefMut
where
    C::Size: RectangularSize<C>,
{
    fn from_raw(raw: R, size: C::Size) -> Self;

    fn from_uint(bb: R::Primitive, size: C::Size) -> Self {
        Self::from_raw(R::from_primitive(bb), size)
    }

    fn from_u128(bb: u128, size: C::Size) -> Self {
        Self::from_raw(R::from_u128(bb), size)
    }

    fn rank_0(size: C::Size) -> Self {
        Self::from_raw(
            R::from_primitive((R::Primitive::one() << size.width().val()) - R::Primitive::one()),
            size,
        )
    }

    fn file_0(size: C::Size) -> Self {
        Self::from_raw(R::from_u128(STEPS[size.width().val()]), size)
    }

    fn rank(idx: DimT, size: C::Size) -> Self {
        debug_assert!(idx < size.height().0);
        Self::rank_0(size) << (idx as usize * size.width().val())
    }

    fn file(idx: DimT, size: C::Size) -> Self {
        debug_assert!(idx < size.height().0);
        Self::file_0(size) << idx as usize
    }

    fn diag_for_sq(sq: C, size: C::Size) -> Self {
        debug_assert!(size.coordinates_valid(sq));
        Self::from_u128(DIAGONALS[size.width().val()][size.to_idx(sq)], size)
    }

    fn anti_diag_for_sq(sq: C, size: C::Size) -> Self {
        debug_assert!(size.coordinates_valid(sq));
        Self::from_u128(ANTI_DIAGONALS[size.width().val()][size.to_idx(sq)], size)
    }

    fn raw(self) -> R;

    fn size(self) -> C::Size;

    fn width(self) -> usize {
        self.size().width().val()
    }

    fn height(self) -> usize {
        self.size().height().val()
    }

    fn piece_coordinates(self) -> C {
        debug_assert!(self.is_single_piece());
        let idx = self.trailing_zeros();
        self.size().to_coordinates(idx)
    }

    // TODO: The following two methods are likely very slow. Find something faster
    /// Flips the `_rank`th rank of the bitboard horizontally and leaves the other bits in an unspecified state.
    fn flip_left_right(self, _rank: usize) -> Self {
        let width = self.size().width().val();
        let mut bb = self;
        let file_mask = Self::file_0(self.size());
        // flip files linearly
        for i in 0..width / 2 {
            let left_shift = i;
            let right_shift = width - 1 - i;
            let left_file = (bb >> left_shift) & file_mask;
            let right_file = (bb >> right_shift) & file_mask;
            let xor = left_file ^ right_file;
            bb ^= xor << left_shift;
            bb ^= xor << right_shift;
        }
        bb
    }

    fn flip_up_down(self) -> Self {
        let size = self.size();
        let mut bb = self;
        let rank_mask = Self::rank_0(size);
        // flip ranks linearly
        for i in 0..size.height().val() / 2 {
            let lower_shift = i * size.width().val();
            let upper_shift = (size.height().val() - 1 - i) * size.width().val();
            let lower_rank = (bb >> lower_shift) & rank_mask;
            let upper_rank = (bb >> upper_shift) & rank_mask;
            let xor = lower_rank ^ upper_rank;
            bb ^= xor << lower_shift;
            bb ^= xor << upper_shift;
        }
        bb
    }

    fn get_piece_file(self) -> usize {
        debug_assert!(self.is_single_piece());
        self.trailing_zeros() % self.size().width().val()
    }

    fn get_piece_rank(self) -> usize {
        debug_assert!(self.is_single_piece());
        self.trailing_zeros() / self.size().width().val()
    }

    /// Returns a bitboard where exactly the bits in the inclusive interval [low, high] are set,
    /// where `low_bb` is `Self::single_piece(low)` and `high_bb` is `Self::single_piece(high)`
    fn square_between(low_bb: Self, high_bb: Self) -> Self {
        debug_assert!(low_bb.is_single_piece());
        debug_assert!(high_bb.is_single_piece());
        debug_assert_eq!(low_bb.size(), high_bb.size());
        let raw = R::squares_between(low_bb.raw(), high_bb.raw());
        Self::from_raw(raw, low_bb.size())
    }

    fn hyperbola_quintessence<F>(idx: usize, blockers: Self, reverse: F, ray: Self) -> Self
    where
        F: Fn(Self) -> Self,
    {
        let piece = Self::from_raw(R::single_piece(idx), blockers.size());
        debug_assert!(!(piece & ray).is_zero());
        let blockers = blockers & ray;
        let reversed_blockers = reverse(blockers);
        let forward = blockers - piece;
        let backward = reversed_blockers - reverse(piece);
        let backward = reverse(backward);
        (forward ^ backward) & ray
    }

    fn hyperbola_quintessence_non_horizontal(square: C, blockers: Self, ray: Self) -> Self {
        debug_assert_eq!(blockers.size(), ray.size());
        Self::hyperbola_quintessence(
            ray.size().to_idx(square),
            blockers,
            |x| x.flip_up_down(),
            ray,
        )
    }

    fn horizontal_attacks(square: C, blockers: Self) -> Self {
        let size = blockers.size();
        let rank = square.row();
        let rank_bb = Self::rank(rank, size);
        Self::hyperbola_quintessence(size.to_idx(square), blockers, |x| x.flip_left_right(rank as usize), rank_bb)
    }

    fn vertical_attacks(square: C, blockers: Self) -> Self {
        let file = Self::file(square.column(), blockers.size());
        Self::hyperbola_quintessence_non_horizontal(square, blockers, file)
    }

    fn diagonal_attacks(square: C, blockers: Self) -> Self {
        Self::hyperbola_quintessence_non_horizontal(
            square,
            blockers,
            Self::diag_for_sq(square, blockers.size()),
        )
    }

    fn anti_diagonal_attacks(square: C, blockers: Self) -> Self {
        Self::hyperbola_quintessence_non_horizontal(
            square,
            blockers,
            Self::anti_diag_for_sq(square, blockers.size()),
        )
    }

    /// All slider attack functions, including `rook_attacks` and `bishop_attacks`, assume that the source square
    /// is empty, so if that's not the case, they should be called with `blockers ^ square_bitboard`.
    fn slider_attacks(square: C, blockers: Self, dir: RayDirections) -> Self {
        match dir {
            RayDirections::Horizontal => Self::horizontal_attacks(square, blockers),
            RayDirections::Vertical => Self::vertical_attacks(square, blockers),
            RayDirections::Diagonal => Self::diagonal_attacks(square, blockers),
            RayDirections::AntiDiagonal => Self::anti_diagonal_attacks(square, blockers),
        }
    }

    fn rook_attacks(square: C, blockers: Self) -> Self {
        Self::vertical_attacks(square, blockers) | Self::horizontal_attacks(square, blockers)
    }

    fn bishop_attacks(square: C, blockers: Self) -> Self {
        Self::diagonal_attacks(square, blockers) | Self::anti_diagonal_attacks(square, blockers)
    }

    fn queen_attacks(square: C, blockers: Self) -> Self {
        Self::rook_attacks(square, blockers) | Self::bishop_attacks(square, blockers)
    }

    fn attacks(square: C, blockers: Self, direction: Direction) -> Self {
        match direction {
            Direction::Horizontal => Self::horizontal_attacks(square, blockers),
            Direction::Vertical => Self::vertical_attacks(square, blockers),
            Direction::Diagonal => Self::diagonal_attacks(square, blockers),
            Direction::AntiDiagonal => Self::anti_diagonal_attacks(square, blockers),
        }
    }

    fn north(self) -> Self {
        self << self.width()
    }

    fn south(self) -> Self {
        self >> self.width()
    }

    fn east(self) -> Self {
        (self & !Self::file(self.size().width().0 - 1, self.size())) << 1
    }

    fn west(self) -> Self {
        (self & !Self::file(0, self.size())) >> 1
    }

    fn north_east(self) -> Self {
        self.north().east()
    }

    fn south_east(self) -> Self {
        self.south().east()
    }

    fn south_west(self) -> Self {
        self.south().west()
    }

    fn north_west(self) -> Self {
        self.north().west()
    }

    fn moore_neighbors(self) -> Self {
        let line = self | self.south() | self.north();
        line | line.west() | line.east()
    }

    fn extended_moore_neighbors(self, radius: usize) -> Self {
        let mut res = self;
        for _ in 0..radius {
            res = res.moore_neighbors();
        }
        res
    }
}

// Deriving Eq and Partial Eq means that irrelevant bits are also getting compared.
// This makes comparisons fast but shifts responsibility to the user to properly zero out those,
// which can be confusing. TODO: Change?
#[derive(Default, Copy, Clone, PartialEq, Eq, Debug)]
pub struct DefaultBitboard<R: RawBitboard, C: RectangularCoordinates> {
    raw: R,
    size: C::Size,
}

// TODO: Bitboard overloy for board text output?
impl<R: RawBitboard, C: RectangularCoordinates> Display for DefaultBitboard<R, C>
where
    C::Size: RectangularSize<C>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for row in (0..self.size().height().0).rev() {
            for column in 0..self.size().width().0 {
                let idx = row * self.size().width().0 + column;
                write!(
                    f,
                    "{}",
                    if self.is_bit_set_at(idx as usize) {
                        '1'
                    } else {
                        '0'
                    }
                )?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> Deref for DefaultBitboard<R, C> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> DerefMut for DefaultBitboard<R, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.raw
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> Sub for DefaultBitboard<R, C>
where
    C::Size: RectangularSize<C>,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        debug_assert_eq!(self.size(), rhs.size());
        Self::from_raw(self.raw - rhs.raw, self.size())
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> Not for DefaultBitboard<R, C>
where
    C::Size: RectangularSize<C>,
{
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::from_raw(!self.raw, self.size())
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> BitOr for DefaultBitboard<R, C>
where
    C::Size: RectangularSize<C>,
{
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        debug_assert_eq!(self.size(), rhs.size());
        Self::from_raw(self.raw | rhs.raw, self.size())
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> BitOrAssign for DefaultBitboard<R, C>
where
    C::Size: RectangularSize<C>,
{
    fn bitor_assign(&mut self, rhs: Self) {
        debug_assert_eq!(self.size(), rhs.size());
        self.raw |= rhs.raw
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> BitAnd for DefaultBitboard<R, C>
where
    C::Size: RectangularSize<C>,
{
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        debug_assert_eq!(self.size(), rhs.size());
        Self::from_raw(self.raw() & rhs.raw(), self.size())
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> BitAndAssign for DefaultBitboard<R, C>
where
    C::Size: RectangularSize<C>,
{
    fn bitand_assign(&mut self, rhs: Self) {
        debug_assert_eq!(self.size(), rhs.size());
        self.raw &= rhs.raw
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> BitXor for DefaultBitboard<R, C>
where
    C::Size: RectangularSize<C>,
{
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self {
        debug_assert_eq!(self.size(), rhs.size());
        Self::from_raw(self.raw ^ rhs.raw, self.size())
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> BitXorAssign for DefaultBitboard<R, C>
where
    C::Size: RectangularSize<C>,
{
    fn bitxor_assign(&mut self, rhs: Self) {
        debug_assert_eq!(self.size(), rhs.size());
        self.raw ^= rhs.raw
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> Shl<usize> for DefaultBitboard<R, C>
where
    C::Size: RectangularSize<C>,
{
    type Output = Self;

    fn shl(self, rhs: usize) -> Self::Output {
        Self::from_raw(self.raw << rhs, self.size())
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> ShlAssign<usize> for DefaultBitboard<R, C>
where
    C::Size: RectangularSize<C>,
{
    fn shl_assign(&mut self, rhs: usize) {
        self.raw <<= rhs
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> Shr<usize> for DefaultBitboard<R, C>
where
    C::Size: RectangularSize<C>,
{
    type Output = Self;

    fn shr(self, rhs: usize) -> Self::Output {
        Self::from_raw(self.raw >> rhs, self.size())
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> ShrAssign<usize> for DefaultBitboard<R, C>
where
    C::Size: RectangularSize<C>,
{
    fn shr_assign(&mut self, rhs: usize) {
        self.raw >>= rhs
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> Bitboard<R, C> for DefaultBitboard<R, C>
where
    C::Size: RectangularSize<C>,
{
    fn from_raw(raw: R, size: C::Size) -> Self {
        Self { raw, size }
    }

    fn size(self) -> C::Size {
        self.size
    }

    fn raw(self) -> R {
        self.raw
    }
}

/// Bitboards for Chessboards. (Not necessarily only for chess, e.g. checkers would use the same bitboards).
/// Treated specially because some operations are much simpler and faster for 8x8 boards.
#[cfg(feature = "chess")]
pub mod chess {
    use crate::games::Color;
    use crate::games::Color::*;
    use crate::general::squares::{GridCoordinates, GridSize};
    use derive_more::Display;

    use super::*;

    /// Some of the (automatically derived) methods of ChessBitbiard aren't `const`,
    /// so use `u64` for all `const fn`s.
    const CHESS_DIAGONALS: [ChessBitboard; 64] = {
        let mut res = [ChessBitboard::from_u64(0); 64];
        let mut i = 0;
        while i < 64 {
            res[i] = ChessBitboard::from_u64(DIAGONALS[8][i] as u64);
            i += 1;
        }
        res
    };

    const CHESS_ANTI_DIAGONALS: [ChessBitboard; 64] = {
        let mut res = [ChessBitboard::from_u64(0); 64];
        let mut i = 0;
        while i < 64 {
            res[i] = ChessBitboard::from_u64(ANTI_DIAGONALS[8][i] as u64);
            i += 1;
        }
        res
    };

    const fn precompute_single_knight_attacks(square_idx: usize) -> u64 {
        let this_knight: u64 = 1 << square_idx;
        let a_file: u64 = A_FILE.raw.0;
        let knight_not_a_file = this_knight & !a_file;
        let mut attacks = (knight_not_a_file << 15) | (knight_not_a_file >> 17);
        let knight_not_h_file = this_knight & !(a_file << 7);
        attacks |= (knight_not_h_file >> 15) | (knight_not_h_file << 17);
        let knight_not_ab_file = knight_not_a_file & !(a_file << 1);
        attacks |= (knight_not_ab_file << 6) | (knight_not_ab_file >> 10);
        let knight_not_gh_file = knight_not_h_file & !(a_file << 6);
        attacks |= (knight_not_gh_file >> 6) | (knight_not_gh_file << 10);
        attacks
    }

    const fn precompute_single_king_attacks(square_idx: usize) -> u64 {
        let king = 1 << square_idx;
        let a_file = A_FILE.raw.0;
        let king_not_a_file = king & !a_file;
        let king_not_h_file = king & !(a_file << 7);
        (king << 8)
            | (king >> 8)
            | (king_not_a_file >> 1)
            | (king_not_a_file << 7)
            | (king_not_a_file >> 9)
            | (king_not_h_file << 1)
            | (king_not_h_file >> 7)
            | (king_not_h_file << 9)
    }

    const fn precompute_single_pawn_capture(color: Color, square_idx: usize) -> u64 {
        let pawn = 1 << square_idx;
        let pawn_not_a_file = pawn & !A_FILE.raw.0;
        let pawn_not_h_file = pawn & !(A_FILE.raw.0 << 7);
        match color {
            White => (pawn_not_a_file << 7) | (pawn_not_h_file << 9),
            Black => (pawn_not_a_file >> 9) | (pawn_not_h_file >> 7),
        }
    }

    pub const KNIGHTS: [ChessBitboard; 64] = {
        let mut res: [ChessBitboard; 64] = [ChessBitboard::from_u64(0); 64];
        let mut i = 0;
        while i < 64 {
            res[i] = ChessBitboard::from_u64(precompute_single_knight_attacks(i));
            i += 1;
        }
        res
    };

    pub const KINGS: [ChessBitboard; 64] = {
        let mut res = [ChessBitboard::from_u64(0); 64];
        let mut i = 0;
        while i < 64 {
            res[i] = ChessBitboard::from_u64(precompute_single_king_attacks(i));
            i += 1;
        }
        res
    };

    pub const PAWN_CAPTURES: [[ChessBitboard; 64]; 2] = {
        let mut res = [[ChessBitboard::from_u64(0); 64]; 2];
        let mut i = 0;
        while i < 64 {
            res[White as usize][i] =
                ChessBitboard::from_u64(precompute_single_pawn_capture(White, i));
            res[Black as usize][i] =
                ChessBitboard::from_u64(precompute_single_pawn_capture(Black, i));
            i += 1;
        }
        res
    };

    pub const WHITE_SQUARES: ChessBitboard = ChessBitboard::from_u64(0xaaaa_aaaa_aaaa_aaaa);
    pub const BLACK_SQUARES: ChessBitboard = ChessBitboard::from_u64(0x5555_5555_5555_5555);

    pub const A_FILE: ChessBitboard = ChessBitboard::from_u64(0x0101_0101_0101_0101);
    pub const FIRST_RANK: ChessBitboard = ChessBitboard::from_u64(0xFF);

    #[derive(
        Copy,
        Clone,
        Eq,
        PartialEq,
        Default,
        Hash,
        Not,
        BitOr,
        BitOrAssign,
        BitAnd,
        BitAndAssign,
        BitXor,
        BitXorAssign,
        Shl,
        ShlAssign,
        Shr,
        ShrAssign,
    )]
    #[cfg(feature = "chess")]
    pub struct ChessBitboard {
        raw: RawStandardBitboard,
    }

    impl ChessBitboard {
        pub const fn new(raw: RawStandardBitboard) -> Self {
            Self { raw }
        }

        pub const fn from_u64(bb: u64) -> Self {
            Self::new(RawStandardBitboard(bb))
        }

        pub fn single_piece(idx: usize) -> Self {
            Self::new(RawStandardBitboard::single_piece(idx))
        }

        pub fn rank_no(idx: DimT) -> Self {
            Self::rank(idx, ChessboardSize::default())
        }

        pub fn file_no(idx: DimT) -> Self {
            Self::file(idx, ChessboardSize::default())
        }

        pub fn pawn_ranks() -> Self {
            Self::from_u64(0x00ff_0000_0000_ff00)
        }

        pub fn pawn_advance(self, color: Color) -> Self {
            match color {
                White => self.north(),
                Black => self.south(),
            }
        }

        pub const fn to_u64(self) -> u64 {
            self.raw.0
        }
    }

    impl Debug for ChessBitboard {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "Chess Bitboard {:#x}", self.0)
        }
    }

    impl Display for ChessBitboard {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            Display::fmt(
                &DefaultBitboard::<RawStandardBitboard, GridCoordinates>::from_raw(
                    self.raw,
                    GridSize::chess(),
                ),
                f,
            )
        }
    }

    impl Deref for ChessBitboard {
        type Target = RawStandardBitboard;

        fn deref(&self) -> &Self::Target {
            &self.raw
        }
    }

    impl DerefMut for ChessBitboard {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.raw
        }
    }

    impl Bitboard<RawStandardBitboard, ChessSquare> for ChessBitboard {
        fn from_raw(raw: RawStandardBitboard, _size: ChessboardSize) -> Self {
            Self { raw }
        }

        fn size(self) -> ChessboardSize {
            ChessboardSize::default()
        }

        fn raw(self) -> RawStandardBitboard {
            self.raw
        }

        // idea from here: https://stackoverflow.com/questions/2602823/in-c-c-whats-the-simplest-way-to-reverse-the-order-of-bits-in-a-byte/2603254#2603254
        fn flip_left_right(self, rank: usize) -> ChessBitboard {
            const LOOKUP: [u8; 16] = [
                0x0, 0x8, 0x4, 0xc, 0x2, 0xa, 0x6, 0xe, 0x1, 0x9, 0x5, 0xd, 0x3, 0xb, 0x7, 0xf,
            ];
            let bb = self.0 >> (8 * rank);
            Self::from_u64(
                (LOOKUP[((bb >> 4) & 0xf) as usize] | (LOOKUP[(bb & 0xf) as usize] << 4)) as u64,
            ) << (8 * rank)
        }

        fn flip_up_down(self) -> Self {
            Self::from_u64(self.0.swap_bytes())
        }

        fn get_piece_file(self) -> usize {
            debug_assert!(self.raw().0.is_power_of_two());
            self.0.trailing_zeros() as usize % 8
        }

        fn get_piece_rank(self) -> usize {
            debug_assert!(self.raw().0.is_power_of_two());
            self.0.trailing_zeros() as usize / 8
        }

        fn file_0(_size: ChessboardSize) -> Self {
            Self::from_u64(0x0101_0101_0101_0101)
        }

        // specialization of the generic trait method for performance
        fn diag_for_sq(sq: ChessSquare, _size: ChessboardSize) -> Self {
            CHESS_DIAGONALS[sq.idx()]
        }

        fn anti_diag_for_sq(sq: ChessSquare, _size: ChessboardSize) -> Self {
            CHESS_ANTI_DIAGONALS[sq.idx()]
        }
    }

    impl Sub for ChessBitboard {
        type Output = ChessBitboard;

        fn sub(self, rhs: Self) -> Self::Output {
            ChessBitboard::new(self.raw - rhs.raw)
        }
    }
}

// Ideally, this would be generic over the bitboard type, but then it couldn't be const.
pub const fn remove_ones_above(bb: u128, idx: usize) -> u128 {
    if idx < 128 {
        bb & (u128::MAX >> (127 - idx))
    } else {
        bb
    }
}

pub const fn remove_ones_below(bb: u128, idx: usize) -> u128 {
    bb & (u128::MAX << idx)
}

#[cfg(test)]
mod tests {
    use crate::games::mnk::MnkBitboard;
    use crate::games::{Height, Width};
    use crate::general::bitboards::{remove_ones_above, remove_ones_below, Bitboard};
    use crate::general::squares::GridSize;

    #[test]
    fn remove_ones_above_test() {
        assert_eq!(remove_ones_above(0xffff_ffff, 15), 0xffff);
        assert_eq!(remove_ones_above(0x00ab_cdef, 7), 0xef);
        assert_eq!(remove_ones_above(0x1248, 6), 0x48);
        assert_eq!(remove_ones_above(0x1148, 4), 0x8);
        assert_eq!(remove_ones_above(0x12345, 0), 1);
        assert_eq!(remove_ones_above(0x12345, 127), 0x12345);
    }

    #[test]
    fn remove_ones_below_test() {
        assert_eq!(remove_ones_below(0xffff_ffff, 16), 0xffff_0000);
        assert_eq!(remove_ones_below(0x00ab_cdef, 8), 0x00ab_cd00);
        assert_eq!(remove_ones_below(0x1248, 8), 0x1200);
        assert_eq!(remove_ones_below(0x1148, 5), 0x1140);
        assert_eq!(remove_ones_below(0x12345, 0), 0x12345);
        assert_eq!(remove_ones_below(0x12345, 1), 0x12344);
        assert_eq!(remove_ones_below(0x12345, 127), 0);
    }

    #[test]
    fn hyperbola_quintessence_test() {
        let size = GridSize::new(Height(1), Width(2));
        for i in 0..64 {
            let row = i / 8;
            let expected = (0xff_u128 - (1 << (i % 8))) << (row * 8);
            assert_eq!(
                MnkBitboard::hyperbola_quintessence(
                    i,
                    MnkBitboard::from_uint(0, size),
                    |x| MnkBitboard::from_uint(x.0.reverse_bits(), size),
                    MnkBitboard::from_uint(0xff, size) << (row * 8)
                ),
                MnkBitboard::from_uint(expected, size),
                "{i}"
            );
        }

        assert_eq!(
            MnkBitboard::hyperbola_quintessence(
                3,
                MnkBitboard::from_uint(0b_0100_0001, size),
                |x| MnkBitboard::from_uint(x.0.reverse_bits(), size),
                MnkBitboard::from_uint(0xff, size),
            ),
            MnkBitboard::from_uint(0b_0111_0111, size),
        );

        assert_eq!(
            MnkBitboard::hyperbola_quintessence(
                28,
                MnkBitboard::from_uint(0x1234_4000_0fed, size),
                |x| MnkBitboard::from_uint(x.0.reverse_bits(), size),
                MnkBitboard::from_uint(0xffff_ffff_ffff, size),
            ),
            MnkBitboard::from_uint(0x0000_6fff_f800, size),
        );

        assert_eq!(
            MnkBitboard::hyperbola_quintessence(
                28,
                MnkBitboard::from_uint(0x0110_0200_0001_1111, size),
                |x| MnkBitboard::from_uint(x.0.reverse_bits(), size),
                MnkBitboard::from_uint(0x1111_1111_1111_1111, size),
            ),
            MnkBitboard::from_uint(0x0011_1111_0111_0000, size),
        );

        assert_eq!(
            MnkBitboard::hyperbola_quintessence(
                16,
                MnkBitboard::from_uint(0xfffe_d002_a912, size),
                |x| MnkBitboard::from_uint(x.0.swap_bytes(), size),
                MnkBitboard::from_uint(0x0101_0101_0101, size),
            ),
            MnkBitboard::from_uint(0x0101_0100_0100, size),
        );

        assert_eq!(
            MnkBitboard::hyperbola_quintessence(
                20,
                MnkBitboard::from_uint(0xffff_ffef_ffff, size),
                |x| MnkBitboard::from_uint(x.0.swap_bytes(), size),
                MnkBitboard::from_uint(0x_ffff_ffff_ffff, size),
            ),
            MnkBitboard::from_uint(0, size),
        );
    }
}
