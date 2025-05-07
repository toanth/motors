extern crate num;

use arbitrary::{Arbitrary, Unstructured};
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::iter::FusedIterator;
use std::ops::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Deref, DerefMut, Not, Shl, ShlAssign, Shr,
    ShrAssign, Sub,
};

use derive_more::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, ShlAssign, Shr, ShrAssign, Sub,
};
use num::traits::WrappingSub;
use num::{PrimInt, Unsigned};
use strum_macros::EnumIter;

use crate::games::{DimT, KnownSize, Size};
use crate::general::bitboards::chessboard::{RAYS_EXCLUSIVE, RAYS_INCLUSIVE};
use crate::general::hq::{U64AndRev, U128AndRev, WithRev};
use crate::general::squares::{RectangularCoordinates, RectangularSize, SmallGridSize, SmallGridSquare};

/// Remove all `1` bits in `bb` strictly above the given `idx`, where `bb` is the type, like `u64`,
/// and `len` is the number of bits in the type, like `64`.
macro_rules! remove_ones_above {
    ($bb: expr, $idx: expr, $typ:ty, $len: expr) => {
        if $idx < $len { $bb & (<$typ>::MAX >> ($len - 1 - $idx)) } else { $bb }
    };
}

macro_rules! remove_ones_below {
    ($bb: expr, $idx: expr, $typ:ty) => {
        $bb & (<$typ>::MAX << $idx)
    };
}

const MAX_STEP_SIZE: usize = 30;

macro_rules! step_bb_for {
    ($typ: ty, $num_bits:expr) => {{
        const LEN: usize = if $num_bits < MAX_STEP_SIZE { $num_bits } else { MAX_STEP_SIZE };
        let mut res: [$typ; LEN] = [0; LEN];
        res[0] = 1;
        let mut step = 1;
        while step < LEN {
            let mut i = 0;
            while i < $num_bits {
                res[step] |= 1 << i;
                i += step;
            }
            step += 1;
        }
        res
    }};
}

macro_rules! diagonal_bb_for {
    ($typ: ty, $size: expr, $steps: ident) => {{
        let mut res: [[$typ; $size]; MAX_WIDTH] = [[0; $size]; MAX_WIDTH];
        let mut width = 1;
        while width < MAX_WIDTH {
            // can't use for loops in const functions
            let mut i: usize = 0;
            while i < $size {
                let diag = (i / width) as isize - (i % width) as isize;
                if diag > 0 {
                    let diag = diag as usize;
                    res[width][i] = $steps[width + 1] << (diag * width);
                    res[width][i] = remove_ones_above!(res[width][i], (width + diag + 1) * width, $typ, $size);
                } else {
                    let diag = -diag as usize;
                    res[width][i] = remove_ones_below!($steps[width + 1] << diag, diag, $typ);
                    res[width][i] = remove_ones_above!(res[width][i], (width - diag) * width, $typ, $size);
                }
                i += 1;
            }
            width += 1;
        }
        res
    }};
}

macro_rules! anti_diagonal_bb_for {
    ($typ: ty, $size: expr, $steps: ident) => {{
        let mut res: [[$typ; $size]; MAX_WIDTH] = [[0; $size]; MAX_WIDTH];
        let mut width = 1;
        while width < MAX_WIDTH {
            let mut i = 0;
            while i < $size {
                let anti_diag = i / width + i % width;
                res[width][i] = remove_ones_above!($steps[width - 1] << anti_diag, anti_diag * width, $typ, $size);
                #[allow(clippy::assign_op_pattern)]
                if anti_diag >= width {
                    res[width][i] = remove_ones_below!(res[width][i], (anti_diag - width + 2) * width - 1, $typ);
                }
                i += 1;
            }
            width += 1;
        }
        res
    }};
}

macro_rules! ray_between_exclusive {
    ($a: expr, $b: expr, $ray: expr, $one: expr, $typ: ty) => {{
        let a: $typ = $one << $a;
        let b: $typ = $one << $b;
        let max = if a > b { a } else { b };
        let min = if a < b { a } else { b };
        (max - min) & $ray & !min
    }};
}

macro_rules! ray_between_inclusive {
    ($a: expr, $b: expr, $ray: expr, $one: expr, $typ: ty) => {{
        let a: $typ = $one << $a;
        let b: $typ = $one << $b;
        let max = if a > b { a } else { b };
        let min = if a < b { a } else { b };
        ((max - min) | max) & $ray
    }};
}

// TODO: Store as array of structs? Could be a speed up

// TODO: Remove the (anti)diagonal bitboards as they are more or less stored in the hq rays
// (except that the square itself doesn't have a set bit) and in the RAY_INCLUSIVE arrays?

// allow width of at most 26 to prevent issues with square notation (26 letters in the alphabet)
// with one extra to make some boundary conditions go away
pub const MAX_WIDTH: usize = 27;

pub(super) const STEPS_U64: [u64; MAX_STEP_SIZE] = step_bb_for!(u64, 64);

pub(super) const DIAGONALS_U64: [[u64; 64]; MAX_WIDTH] = diagonal_bb_for!(u64, 64, STEPS_U64);

pub(super) const ANTI_DIAGONALS_U64: [[u64; 64]; MAX_WIDTH] = anti_diagonal_bb_for!(u64, 64, STEPS_U64);

// These arrays are `static` instead of `const` because they are pretty large

pub(super) static STEPS_U128: [u128; MAX_STEP_SIZE] = step_bb_for!(u128, 128);

// TODO: Remove
pub(super) static DIAGONALS_U128: [[u128; 128]; MAX_WIDTH] = diagonal_bb_for!(u128, 128, STEPS_U128);

pub(super) static ANTI_DIAGONALS_U128: [[u128; 128]; MAX_WIDTH] = anti_diagonal_bb_for!(u128, 128, STEPS_U128);

/// A [`RawBitboard`] is something like a `u64` or `u128` (but there's nothing that would prevent implementing it for a custom
/// 256 bit struct). Unlike a `[Bitboard]`, it has no notion of size, and therefore no notion of rows, columns, etc.
/// It's also not generally a unique type, so e.g. a [`RawStandardBitboard`] does not offer any type safety over using a `u64`.
#[must_use]
pub trait RawBitboard:
    for<'a> Arbitrary<'a>
    + PrimInt
    + From<u8>
    + WrappingSub
    + BitAndAssign
    + BitOrAssign
    + BitXorAssign
    + ShlAssign<usize>
    + ShrAssign<usize>
    + Unsigned
    + Display
    + Debug
{
    type WithRev: WithRev<RawBitboard = Self>;

    /// A `usize` may not be enough to hold all the bits, but sometimes that's fine.
    fn to_usize(self) -> usize;

    fn steps_bb(step_size: usize) -> Self;

    fn diagonal_bb(width: usize, diag: usize) -> Self;

    fn anti_diagonal_bb(width: usize, diag: usize) -> Self;

    fn remove_ones_above(self, idx: usize) -> Self;

    #[inline]
    fn is_zero(self) -> bool {
        // it might make sense to override this for custom types that implement large bitboards
        self == Self::zero()
    }

    #[inline]
    fn has_set_bit(self) -> bool {
        !self.is_zero()
    }

    #[inline]
    fn pop_lsb(&mut self) -> usize {
        let shift = self.num_trailing_zeros();
        *self &= *self - Self::one();
        shift
    }

    #[must_use]
    #[inline]
    fn single_piece_at(idx: usize) -> Self {
        Self::one() << idx
    }

    // apparently, the num crate doesn't provide a is_power_of_two() method
    fn is_single_piece(self) -> bool;

    #[inline]
    fn more_than_one_bit_set(self) -> bool {
        (self & (self.wrapping_sub(&Self::one()))).has_set_bit()
    }

    #[inline]
    fn is_bit_set_at(self, idx: usize) -> bool {
        (self >> idx) & Self::one() != Self::zero()
    }

    #[inline]
    fn num_trailing_zeros(self) -> usize {
        self.trailing_zeros() as usize
    }

    #[inline]
    fn num_ones(self) -> usize {
        self.count_ones() as usize
    }

    #[inline]
    fn one_indices(self) -> BitIterator<Self> {
        BitIterator(self)
    }
}

#[must_use]
pub struct BitIterator<B: RawBitboard>(B);

impl<B: RawBitboard> Iterator for BitIterator<B> {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_zero() { None } else { Some(self.0.pop_lsb()) }
    }
}

impl<B: RawBitboard> FusedIterator for BitIterator<B> {}

pub type RawStandardBitboard = u64;

impl RawBitboard for RawStandardBitboard {
    type WithRev = U64AndRev;

    fn to_usize(self) -> usize {
        self as usize
    }

    #[inline]
    fn steps_bb(step_size: usize) -> Self {
        STEPS_U64[step_size]
    }

    #[inline]
    fn diagonal_bb(width: usize, sq: usize) -> Self {
        DIAGONALS_U64[width][sq]
    }

    #[inline]
    fn anti_diagonal_bb(width: usize, sq: usize) -> Self {
        ANTI_DIAGONALS_U64[width][sq]
    }

    fn remove_ones_above(self, idx: usize) -> Self {
        remove_ones_above!(self, idx, u64, 64)
    }

    fn is_single_piece(self) -> bool {
        self.is_power_of_two()
    }
}

pub type ExtendedRawBitboard = u128;

impl RawBitboard for ExtendedRawBitboard {
    type WithRev = U128AndRev;

    fn to_usize(self) -> usize {
        self as usize
    }

    #[inline]
    fn steps_bb(step_size: usize) -> Self {
        STEPS_U128[step_size]
    }

    // TODO: Make table smaller by only storing #diagonal entries, not #square entries
    #[inline]
    fn diagonal_bb(width: usize, sq: usize) -> Self {
        DIAGONALS_U128[width][sq]
    }

    #[inline]
    fn anti_diagonal_bb(width: usize, sq: usize) -> Self {
        ANTI_DIAGONALS_U128[width][sq]
    }

    fn remove_ones_above(self, idx: usize) -> Self {
        remove_ones_above!(self, idx, u128, 128)
    }

    fn is_single_piece(self) -> bool {
        self.is_power_of_two()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, EnumIter)]
pub enum RayDirections {
    Horizontal,
    Vertical,
    Diagonal,
    AntiDiagonal,
}

/// A bitboard extends a [`RawBitboard`] (simply a set of 64 / 128 / ... bits) with a size,
/// which allows talking about concepts like rows.
// TODO: Redesign: Use two traits: Sized bitboard and unsized bitboard.
// Bitboards that aren't sized, like Fairy or Mnk bitboards, don't need to be made sized.
#[must_use]
pub trait Bitboard<R: RawBitboard, C: RectangularCoordinates>:
    Debug
    + Display
    + Copy
    + Clone
    + Eq
    + PartialEq
    + WrappingSub
    + Not<Output = Self>
    + BitAnd<Output = Self>
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
{
    fn new(raw: R, size: C::Size) -> Self;

    fn single_piece_for(square: C, size: C::Size) -> Self {
        Self::new(R::single_piece_at(size.internal_key(square)), size)
    }

    #[inline]
    fn rank_0_for(size: C::Size) -> Self {
        Self::new((R::one() << size.width().val()) - R::one(), size)
    }

    #[inline]
    fn file_0_for(size: C::Size) -> Self {
        let steps = R::steps_bb(size.internal_width());
        // in theory, the compiler should be smart enough to realize that it can remove this for chess bitboards
        let bb = steps.remove_ones_above(size.num_squares() - 1);
        Self::new(bb, size)
    }

    #[inline]
    fn rank_for(idx: DimT, size: C::Size) -> Self {
        debug_assert!(idx < size.height().0);
        Self::rank_0_for(size) << (idx as usize * size.internal_width())
    }

    #[inline]
    fn file_for(idx: DimT, size: C::Size) -> Self {
        debug_assert!(idx < size.width().0);
        Self::file_0_for(size) << idx as usize
    }

    #[inline]
    fn backranks_for(size: C::Size) -> Self {
        Self::rank_0_for(size) | Self::rank_for(size.height().get() - 1, size)
    }

    fn ray_exclusive(a: C, b: C, size: C::Size) -> Self;

    fn ray_inclusive(a: C, b: C, size: C::Size) -> Self;

    #[inline]
    fn diag_for_sq(sq: C, size: C::Size) -> Self {
        debug_assert!(size.coordinates_valid(sq));
        Self::new(R::diagonal_bb(size.internal_width(), size.internal_key(sq)), size)
    }

    #[inline]
    fn anti_diag_for_sq(sq: C, size: C::Size) -> Self {
        debug_assert!(size.coordinates_valid(sq));
        Self::new(R::anti_diagonal_bb(size.internal_width(), size.internal_key(sq)), size)
    }

    /// Not especially fast; meant to be called once to precompute this bitboard
    fn valid_squares_for_size(size: C::Size) -> Self {
        let rank_0 = Self::rank_0_for(size);
        let mut ranks = rank_0;
        for n in 1..size.height().val() {
            ranks |= rank_0 << (size.internal_width() * n);
        }
        ranks
    }

    fn raw(self) -> R;

    fn size(self) -> C::Size;

    #[inline]
    fn width(self) -> usize {
        self.size().width().val()
    }

    #[inline]
    fn internal_width(self) -> usize {
        self.size().internal_width()
    }

    #[inline]
    fn height(self) -> usize {
        self.size().height().val()
    }

    fn flip_up_down(self) -> Self {
        let size = self.size();
        let mut bb = self;
        let rank_mask = Self::rank_0_for(size);
        // flip ranks linearly
        for i in 0..size.height().val() / 2 {
            let lower_shift = i * self.internal_width();
            let upper_shift = (size.height().val() - 1 - i) * self.internal_width();
            let lower_rank = (bb >> lower_shift) & rank_mask;
            let upper_rank = (bb >> upper_shift) & rank_mask;
            let xor = lower_rank ^ upper_rank;
            bb ^= xor << lower_shift;
            bb ^= xor << upper_shift;
        }
        bb
    }

    #[inline]
    fn flip_if(self, flip: bool) -> Self {
        if flip { self.flip_up_down() } else { self }
    }

    #[inline]
    fn get_piece_file(self) -> usize {
        debug_assert!(self.is_single_piece());
        self.num_trailing_zeros() % self.internal_width()
    }

    #[inline]
    fn get_piece_rank(self) -> usize {
        debug_assert!(self.is_single_piece());
        self.num_trailing_zeros() / self.internal_width()
    }

    #[inline]
    /// This function is very general and can deal with arbitrary rays (e.g. riders in fairy chess),
    /// but at the cost of calling `reverse` multiple times.
    fn hyperbola_quintessence_fallback<F>(idx: usize, blockers: Self, reverse: F, ray: Self) -> Self
    where
        F: Fn(Self) -> Self,
    {
        let piece = Self::new(R::single_piece_at(idx), blockers.size());
        debug_assert!(!(piece & ray).is_zero());
        let blockers = blockers & ray;
        let reversed_blockers = reverse(blockers);
        let forward = blockers.wrapping_sub(&piece);
        let backward = reversed_blockers.wrapping_sub(&reverse(piece));
        let backward = reverse(backward);
        (forward ^ backward) & ray
    }

    #[inline]
    fn north(self) -> Self {
        self << self.internal_width()
    }

    #[inline]
    fn south(self) -> Self {
        self >> self.internal_width()
    }

    #[inline]
    fn east(self) -> Self {
        (self & !Self::file_for(self.size().width().0 - 1, self.size())) << 1
    }

    #[inline]
    fn west(self) -> Self {
        (self & !Self::file_0_for(self.size())) >> 1
    }

    #[inline]
    fn north_east(self) -> Self {
        self.east().north()
    }

    #[inline]
    fn south_east(self) -> Self {
        self.east().south()
    }

    #[inline]
    fn south_west(self) -> Self {
        self.west().south()
    }

    #[inline]
    fn north_west(self) -> Self {
        self.west().north()
    }

    /// Includes the bitboard itself
    #[inline]
    fn moore_neighbors(self) -> Self {
        let line = self | self.south() | self.north();
        line | line.west() | line.east()
    }

    /// Include the bitboard itself
    #[inline]
    fn extended_moore_neighbors(self, radius: usize) -> Self {
        let mut res = self;
        for _ in 0..radius {
            res = res.moore_neighbors();
        }
        res
    }

    #[inline]
    fn ones(self) -> impl Iterator<Item = C> {
        self.one_indices().map(move |i| self.size().to_coordinates_unchecked(i))
    }

    #[inline]
    fn to_square(self) -> Option<C> {
        if self.is_single_piece() {
            self.size().check_coordinates(self.size().to_coordinates_unchecked(self.num_trailing_zeros())).ok()
        } else {
            None
        }
    }

    fn format(self, f: &mut Formatter<'_>) -> fmt::Result {
        for row in (0..self.size().height().val()).rev() {
            for column in 0..self.size().width().val() {
                let idx = row * self.size().internal_width() + column;
                write!(f, "{}", if self.is_bit_set_at(idx) { '1' } else { '0' })?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

pub trait KnownSizeBitboard<R: RawBitboard, C: RectangularCoordinates<Size: KnownSize<C>>>: Bitboard<R, C> {
    #[inline]
    fn from_raw(raw: R) -> Self {
        Bitboard::new(raw, C::Size::default())
    }

    // May panic if those coordinates aren't valid, which can happen for runtime sizes
    // (but should be able to be statically checked for [`KnownSizeBitboard`]s).
    // The same goes for similar methods using this function.
    #[inline]
    fn idx_of(c: C) -> usize {
        C::Size::default().internal_key(c)
    }

    #[inline]
    fn is_bit_set(self, c: C) -> bool {
        self.is_bit_set_at(Self::idx_of(c))
    }

    #[inline]
    fn single_piece(c: C) -> Self {
        Self::from_raw(RawBitboard::single_piece_at(Self::idx_of(c)))
    }

    #[inline]
    fn rank_0() -> Self {
        Bitboard::rank_0_for(C::Size::default())
    }

    #[inline]
    fn file_0() -> Self {
        Bitboard::file_0_for(C::Size::default())
    }

    #[inline]
    fn rank(idx: DimT) -> Self {
        Bitboard::rank_for(idx, C::Size::default())
    }

    #[inline]
    fn file(idx: DimT) -> Self {
        Bitboard::file_for(idx, C::Size::default())
    }

    #[inline]
    fn diagonal(sq: C) -> Self {
        Bitboard::diag_for_sq(sq, C::Size::default())
    }

    #[inline]
    fn anti_diagonal(sq: C) -> Self {
        Bitboard::anti_diag_for_sq(sq, C::Size::default())
    }
}

#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Hash,
    Arbitrary,
    derive_more::Deref,
    derive_more::DerefMut,
    Sub,
    BitAnd,
    BitAndAssign,
    BitOr,
    BitOrAssign,
    BitXor,
    BitXorAssign,
    Not,
    Shl,
    ShlAssign,
    Shr,
    ShrAssign,
)]
pub struct SmallGridBitboard<const H: usize, const W: usize>(pub RawStandardBitboard);

impl<const H: usize, const W: usize> SmallGridBitboard<H, W> {
    pub const A_FILE: Self = Self::new(0x0101_0101_0101_0101);
    pub const FIRST_RANK: Self = Self::new(0xff);

    pub const INVALID_EDGE_MASK: Self = Self(Self::files_too_high().0 | Self::ranks_too_high().0);

    #[inline]
    pub const fn new(raw: RawStandardBitboard) -> Self {
        Self(raw)
    }

    pub const fn files_too_high() -> Self {
        let mut file = Self::A_FILE.0 << 7;
        let mut res = 0;
        let mut n = 0;
        while n < 8 - W {
            res |= file;
            n += 1;
            file >>= 1;
        }
        Self(res)
    }

    pub const fn ranks_too_high() -> Self {
        let mut rank = Self::FIRST_RANK.0 << (7 * 8);
        let mut res = 0;
        let mut n = 0;
        while n < 8 - H {
            res |= rank;
            n += 1;
            rank >>= 8;
        }
        Self(res)
    }
}

impl<const H: usize, const W: usize> WrappingSub for SmallGridBitboard<H, W> {
    #[inline]
    fn wrapping_sub(&self, v: &Self) -> Self {
        Self(self.0.wrapping_sub(v.0))
    }
}

impl<const H: usize, const W: usize> Display for SmallGridBitboard<H, W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.format(f)
    }
}

impl<const H: usize, const W: usize> Bitboard<RawStandardBitboard, SmallGridSquare<H, W, 8>>
    for SmallGridBitboard<H, W>
{
    #[inline]
    fn new(raw: RawStandardBitboard, _size: SmallGridSize<H, W>) -> Self {
        Self(raw)
    }

    fn ray_exclusive(a: SmallGridSquare<H, W, 8>, b: SmallGridSquare<H, W, 8>, _size: SmallGridSize<H, W>) -> Self {
        Self::new(RAYS_EXCLUSIVE[a.bb_idx()][b.bb_idx()])
    }

    fn ray_inclusive(a: SmallGridSquare<H, W, 8>, b: SmallGridSquare<H, W, 8>, _size: SmallGridSize<H, W>) -> Self {
        Self::new(RAYS_INCLUSIVE[a.bb_idx()][b.bb_idx()])
    }

    #[inline]
    fn raw(self) -> RawStandardBitboard {
        self.0
    }

    #[inline]
    fn size(self) -> SmallGridSize<H, W> {
        SmallGridSize::default()
    }

    #[inline]
    fn flip_up_down(self) -> Self {
        let in_bounds: Self = !Self::ranks_too_high();
        let shift: usize = 8 * (8 - H);
        Self::new((self.raw() << shift).swap_bytes()) & in_bounds
    }
}

impl<const H: usize, const W: usize> KnownSizeBitboard<RawStandardBitboard, SmallGridSquare<H, W, 8>>
    for SmallGridBitboard<H, W>
{
}

// Deriving Eq and Partial Eq means that irrelevant bits are also getting compared.
// This makes comparisons fast but shifts responsibility to the user to properly zero out those,
// which can be confusing. TODO: Change?
/// Despite the name, it's technically possible to instantiate this generic struct with a `KnownSize`.
#[derive(Default, Copy, Clone, PartialEq, Eq, Debug)]
#[must_use]
pub struct DynamicallySizedBitboard<R: RawBitboard, C: RectangularCoordinates> {
    raw: R,
    size: C::Size,
}

// for some reason, automatically deriving `Arbitrary` doesn't work here
impl<'a, R: RawBitboard, C: RectangularCoordinates> Arbitrary<'a> for DynamicallySizedBitboard<R, C> {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let (raw, size) = u.arbitrary::<(R, C::Size)>()?;
        Ok(Self { raw, size })
    }
}

// TODO: Bitboard overlay for board text output?
impl<R: RawBitboard, C: RectangularCoordinates> Display for DynamicallySizedBitboard<R, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.format(f)
    }
}

// Unfortunately, we can't simply `#derive` the implementations for all the operators

impl<R: RawBitboard, C: RectangularCoordinates> Deref for DynamicallySizedBitboard<R, C> {
    type Target = R;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> DerefMut for DynamicallySizedBitboard<R, C> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.raw
    }
}

/// Necessary for WrappingSub (even though we don't want `Sub` itself).
impl<R: RawBitboard, C: RectangularCoordinates> Sub for DynamicallySizedBitboard<R, C> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        debug_assert_eq!(self.size(), rhs.size());
        Self::new(self.raw - rhs.raw, self.size())
    }
}

/// Necessary for hyperbola quintessence.
impl<R: RawBitboard, C: RectangularCoordinates> WrappingSub for DynamicallySizedBitboard<R, C> {
    #[inline]
    fn wrapping_sub(&self, v: &Self) -> Self {
        Self::new(self.raw.wrapping_sub(&v.raw), self.size)
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> Not for DynamicallySizedBitboard<R, C> {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self::new(!self.raw, self.size())
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> BitOr for DynamicallySizedBitboard<R, C> {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        debug_assert_eq!(self.size(), rhs.size());
        Self::new(self.raw | rhs.raw, self.size())
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> BitOrAssign for DynamicallySizedBitboard<R, C> {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        debug_assert_eq!(self.size(), rhs.size());
        self.raw |= rhs.raw;
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> BitAnd for DynamicallySizedBitboard<R, C> {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        debug_assert_eq!(self.size(), rhs.size());
        Self::new(self.raw() & rhs.raw(), self.size())
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> BitAndAssign for DynamicallySizedBitboard<R, C> {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        debug_assert_eq!(self.size(), rhs.size());
        self.raw &= rhs.raw;
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> BitXor for DynamicallySizedBitboard<R, C> {
    type Output = Self;

    #[inline]
    fn bitxor(self, rhs: Self) -> Self {
        debug_assert_eq!(self.size(), rhs.size());
        Self::new(self.raw ^ rhs.raw, self.size())
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> BitXorAssign for DynamicallySizedBitboard<R, C> {
    #[inline]
    fn bitxor_assign(&mut self, rhs: Self) {
        debug_assert_eq!(self.size(), rhs.size());
        self.raw ^= rhs.raw;
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> Shl<usize> for DynamicallySizedBitboard<R, C> {
    type Output = Self;

    #[inline]
    fn shl(self, rhs: usize) -> Self::Output {
        Self::new(self.raw << rhs, self.size())
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> ShlAssign<usize> for DynamicallySizedBitboard<R, C> {
    #[inline]
    fn shl_assign(&mut self, rhs: usize) {
        self.raw <<= rhs;
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> Shr<usize> for DynamicallySizedBitboard<R, C> {
    type Output = Self;

    #[inline]
    fn shr(self, rhs: usize) -> Self::Output {
        Self::new(self.raw >> rhs, self.size())
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> ShrAssign<usize> for DynamicallySizedBitboard<R, C> {
    #[inline]
    fn shr_assign(&mut self, rhs: usize) {
        self.raw >>= rhs;
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> Bitboard<R, C> for DynamicallySizedBitboard<R, C> {
    #[inline]
    fn new(raw: R, size: C::Size) -> Self {
        Self { raw, size }
    }

    fn ray_exclusive(a: C, b: C, size: C::Size) -> Self {
        let ray = Self::ray(a, b, size).raw;
        let underlying = ray_between_exclusive!(size.internal_key(a), size.internal_key(b), ray, R::one(), R);
        Self::new(underlying, size)
    }

    fn ray_inclusive(a: C, b: C, size: C::Size) -> Self {
        let ray = Self::ray(a, b, size).raw;
        let underlying = ray_between_inclusive!(size.internal_key(a), size.internal_key(b), ray, R::one(), R);
        Self::new(underlying, size)
    }

    #[inline]
    fn raw(self) -> R {
        self.raw
    }

    #[inline]
    fn size(self) -> C::Size {
        self.size
    }
}

impl<R: RawBitboard, C: RectangularCoordinates> DynamicallySizedBitboard<R, C> {
    fn ray(a: C, b: C, size: C::Size) -> Self {
        if a.row() == b.row() {
            Self::rank_for(a.row(), size)
        } else if a.file() == b.file() {
            Self::file_for(a.file(), size)
        } else if a.file().wrapping_sub(a.rank()) == b.file().wrapping_sub(b.rank()) {
            Self::diag_for_sq(a, size)
        } else if a.file().wrapping_sub(a.rank()) == b.rank().wrapping_sub(b.file()) {
            Self::anti_diag_for_sq(a, size)
        } else {
            return Self::new(R::zero(), size);
        }
    }
}

impl<R: RawBitboard, C: RectangularCoordinates<Size: KnownSize<C>>> KnownSizeBitboard<R, C>
    for DynamicallySizedBitboard<R, C>
{
}

#[macro_export]
macro_rules! shift_left {
    ($bb: expr, $amount: expr) => {
        if $amount >= 0 { $bb << $amount } else { $bb >> -$amount }
    };
}

// can't be a lambda because rust doesn't support that in const fns
#[macro_export]
macro_rules! do_shift {
    ($horizontal_shift: expr,
    $vertical_shift: expr,
    $width: expr,
    $file: expr,
    $bb: expr,
    $typ: ty) => {{
        let shift = $horizontal_shift + $vertical_shift * $width;
        if $file >= -$horizontal_shift
            && $file + $horizontal_shift < $width
            && shift < <$typ>::BITS as isize
            && -shift < <$typ>::BITS as isize
        {
            shift_left!($bb, shift)
        } else {
            0
        }
    }};
}

// can be called at compile time or when constructing fairy chess rules
// width is the internal width, so boards with a width less than 8, but height <= 8 can simply use 8 as the width
#[macro_export]
macro_rules! precompute_leaper_attacks {
    ($square_idx: expr, $diff_1: expr, $diff_2: expr, $repeat: expr, $width: expr, $typ: ty) => {{
        let diff_1 = $diff_1 as isize;
        let diff_2 = $diff_2 as isize;
        let width = $width as isize;
        assert!(diff_1 <= diff_2); // an (a, b) leaper is the same as an (b, a) leaper
        let this_piece: $typ = 1 << $square_idx;
        let file = $square_idx as isize % width;
        let mut attacks: $typ = 0;
        let mut i = 0;
        while i < 4 {
            // for horizontal_dir in [-1, 1], for vertical_dir in [-1, 1]
            let horizontal_dir = i / 2 * 2 - 1;
            let vertical_dir = i % 2 * 2 - 1;
            let mut horizontal_offset = horizontal_dir;
            let mut vertical_offset = vertical_dir;
            let mut repetition = 0;
            loop {
                attacks |= $crate::do_shift!(
                    diff_2 * horizontal_offset,
                    diff_1 * vertical_offset,
                    width,
                    file,
                    this_piece,
                    $typ
                );
                attacks |= $crate::do_shift!(
                    diff_1 * horizontal_offset,
                    diff_2 * vertical_offset,
                    width,
                    file,
                    this_piece,
                    $typ
                );
                if !$repeat || repetition > $width {
                    // TODO: max(width, height)
                    break;
                }
                horizontal_offset += horizontal_dir;
                vertical_offset += vertical_dir;
                repetition += 1;
            }
            i += 1;
        }
        attacks
    }};
}

/// 8x8 bitboards. Not necessarily only for chess, e.g. checkers would use the same bitboard.
/// Treated specially because some operations are much simpler and faster for 8x8 boards and boards
/// with smaller width or height can use such a bitboard internally.
// TODO: Use Raw bitboards
pub mod chessboard {
    use super::*;

    pub type ChessBitboard = SmallGridBitboard<8, 8>;

    pub const KNIGHTS: [ChessBitboard; 64] = {
        let mut res: [ChessBitboard; 64] = [ChessBitboard::new(0); 64];
        let mut i = 0;
        while i < 64 {
            res[i] = ChessBitboard::new(precompute_leaper_attacks!(i, 1, 2, false, 8, u64));
            i += 1;
        }
        res
    };

    pub const KINGS: [ChessBitboard; 64] = {
        let mut res = [ChessBitboard::new(0); 64];
        let mut i = 0;
        while i < 64 {
            let bb =
                precompute_leaper_attacks!(i, 1, 1, false, 8, u64) | precompute_leaper_attacks!(i, 0, 1, false, 8, u64);
            res[i] = ChessBitboard::new(bb);
            i += 1;
        }
        res
    };

    // All squares with a sup distance of 2
    pub const ATAXX_LEAPERS: [ChessBitboard; 64] = {
        let mut res = [ChessBitboard::new(0); 64];
        let mut i = 0;
        while i < 64 {
            let bb = precompute_leaper_attacks!(i, 2, 2, false, 8, u64)
                | precompute_leaper_attacks!(i, 1, 2, false, 8, u64)
                | precompute_leaper_attacks!(i, 0, 2, false, 8, u64);
            res[i] = ChessBitboard::new(bb);
            i += 1;
        }
        res
    };

    // `static` instead of `const` because it's pretty large
    pub static RAYS_EXCLUSIVE: [[RawStandardBitboard; 64]; 64] = {
        let mut res = [[0; 64]; 64];
        let mut start = 0;
        while start < 64 {
            let file = start % 8;
            let rank = start / 8;
            let mut i = 0;
            while i < 8 {
                let sq = rank * 8 + i;
                res[start][sq] = ray_between_exclusive!(start, sq, 0xff << (8 * rank), 1_u64, u64);
                let sq = 8 * i + file;
                res[start][sq] = ray_between_exclusive!(start, sq, ChessBitboard::A_FILE.0 << file, 1_u64, u64);
                i += 1;
            }
            let mut diag = DIAGONALS_U64[8][start];
            while diag != 0 {
                let sq = diag.trailing_zeros() as usize;
                res[start][sq] = ray_between_exclusive!(start, sq, DIAGONALS_U64[8][start], 1_u64, u64);
                diag &= diag - 1;
            }
            let mut anti_diag = ANTI_DIAGONALS_U64[8][start];
            while anti_diag != 0 {
                let sq = anti_diag.trailing_zeros() as usize;
                res[start][sq] = ray_between_exclusive!(start, sq, ANTI_DIAGONALS_U64[8][start], 1_u64, u64);
                anti_diag &= anti_diag - 1;
            }
            start += 1;
        }
        res
    };

    /// If the squares are not on a ray, this still includes both squares
    pub static RAYS_INCLUSIVE: [[RawStandardBitboard; 64]; 64] = {
        let mut res = [[0; 64]; 64];
        let mut a = 0;
        while a < 64 {
            let mut b = 0;
            while b < 64 {
                res[a][b] = RAYS_EXCLUSIVE[a][b] | (1 << a) | (1 << b);
                b += 1;
            }
            a += 1;
        }
        res
    };

    pub const fn white_squares() -> ChessBitboard {
        COLORED_SQUARES[0]
    }
    pub const fn black_squares() -> ChessBitboard {
        COLORED_SQUARES[1]
    }

    pub const COLORED_SQUARES: [ChessBitboard; 2] =
        [ChessBitboard::new(0x55aa_55aa_55aa_55aa), ChessBitboard::new(0xaa55_aa55_aa55_aa55)];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::mnk::MnkBitboard;
    use crate::games::{Height, Width};
    use crate::general::bitboards::chessboard::{ATAXX_LEAPERS, ChessBitboard, KINGS};
    use crate::general::squares::GridSize;

    #[test]
    fn precomputed_test() {
        for i in 0..64 {
            // equivalent to `ChessSquare::from_bb_index(i).bb()`
            let bb = ChessBitboard::new(RawStandardBitboard::single_piece_at(i));
            let king = bb.west() | bb.east() | bb;
            let king = king.south() | king.north() | king;
            let leaping = king.west() | king.east() | king;
            let leaping = leaping.south() | leaping.north() | leaping;
            assert_eq!(KINGS[i], king ^ bb, "{i}");
            assert_eq!(ATAXX_LEAPERS[i].raw(), leaping.raw() & !king.raw());
        }
    }

    #[test]
    fn remove_ones_above_test() {
        assert_eq!(remove_ones_above!(0xffff_ffff, 15, u64, 64), 0xffff);
        assert_eq!(remove_ones_above!(0x00ab_cdef, 7, u128, 128), 0xef);
        assert_eq!(remove_ones_above!(0x1248, 6, u32, 32), 0x48);
        assert_eq!(remove_ones_above!(0x1148, 4, u16, 16), 0x8);
        assert_eq!(remove_ones_above!(0x12345, 0, u64, 64), 1);
        assert_eq!(remove_ones_above!(0x12345, 127, u128, 128), 0x12345);
    }

    #[test]
    fn remove_ones_below_test() {
        assert_eq!(remove_ones_below!(0xffff_ffff, 16, u128), 0xffff_0000);
        assert_eq!(remove_ones_below!(0x00ab_cdef, 8, u64), 0x00ab_cd00);
        assert_eq!(remove_ones_below!(0x1248, 8, u32), 0x1200);
        assert_eq!(remove_ones_below!(0x1148, 5, u16), 0x1140);
        assert_eq!(remove_ones_below!(0x12345, 0, u32), 0x12345);
        assert_eq!(remove_ones_below!(0x12345, 1, u128), 0x12344);
        // assert_eq!(remove_ones_below!(0x12345, 127, u64), 0);
    }

    #[test]
    fn hyperbola_quintessence_test() {
        let size = GridSize::new(Height(1), Width(2));
        for i in 0..64 {
            let row = i / 8;
            let expected = (0xff_u128 - (1 << (i % 8))) << (row * 8);
            assert_eq!(
                MnkBitboard::hyperbola_quintessence_fallback(
                    i,
                    MnkBitboard::new(0, size),
                    |x| MnkBitboard::new(x.reverse_bits(), size),
                    MnkBitboard::new(0xff, size) << (row * 8)
                ),
                MnkBitboard::new(expected, size),
                "{i}"
            );
        }

        assert_eq!(
            MnkBitboard::hyperbola_quintessence_fallback(
                3,
                MnkBitboard::new(0b_0100_0001, size),
                |x| MnkBitboard::new(x.reverse_bits(), size),
                MnkBitboard::new(0xff, size),
            ),
            MnkBitboard::new(0b_0111_0111, size),
        );

        assert_eq!(
            MnkBitboard::hyperbola_quintessence_fallback(
                28,
                MnkBitboard::new(0x1234_4000_0fed, size),
                |x| MnkBitboard::new(x.reverse_bits(), size),
                MnkBitboard::new(0xffff_ffff_ffff, size),
            ),
            MnkBitboard::new(0x0000_6fff_f800, size),
        );

        assert_eq!(
            MnkBitboard::hyperbola_quintessence_fallback(
                28,
                MnkBitboard::new(0x0110_0200_0001_1111, size),
                |x| MnkBitboard::new(x.reverse_bits(), size),
                MnkBitboard::new(0x1111_1111_1111_1111, size),
            ),
            MnkBitboard::new(0x0011_1111_0111_0000, size),
        );

        assert_eq!(
            MnkBitboard::hyperbola_quintessence_fallback(
                16,
                MnkBitboard::new(0xfffe_d002_a912, size),
                |x| MnkBitboard::new(x.swap_bytes(), size),
                MnkBitboard::new(0x0101_0101_0101, size),
            ),
            MnkBitboard::new(0x0101_0100_0100, size),
        );

        assert_eq!(
            MnkBitboard::hyperbola_quintessence_fallback(
                20,
                MnkBitboard::new(0xffff_ffef_ffff, size),
                |x| MnkBitboard::new(x.swap_bytes(), size),
                MnkBitboard::new(0x_ffff_ffff_ffff, size),
            ),
            MnkBitboard::new(0, size),
        );
    }
}
