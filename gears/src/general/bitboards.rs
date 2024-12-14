extern crate num;

use arbitrary::{Arbitrary, Unstructured};
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::iter::FusedIterator;
use std::ops::{Deref, DerefMut};

use derive_more::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, ShlAssign, Shr,
    ShrAssign, Sub,
};
use num::traits::WrappingSub;
use num::{PrimInt, Unsigned};
use strum_macros::EnumIter;

use crate::games::{DimT, KnownSize, Size};
use crate::general::squares::{
    RectangularCoordinates, RectangularSize, SmallGridSize, SmallGridSquare,
};

macro_rules! remove_ones_above {
    ($bb: expr, $idx: expr, $typ:ty, $len: expr) => {
        if $idx < $len {
            $bb & (<$typ>::MAX >> ($len - 1 - $idx))
        } else {
            $bb
        }
    };
}

macro_rules! remove_ones_below {
    ($bb: expr, $idx: expr, $typ:ty) => {
        $bb & (<$typ>::MAX << $idx)
    };
}

macro_rules! step_bb_for {
    ($typ: ty, $num_bits:expr) => {{
        let mut res: [$typ; $num_bits] = [0; $num_bits];
        res[0] = 1;
        let mut step = 1;
        while step < $num_bits {
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
                    res[width][i] =
                        remove_ones_above!(res[width][i], (width + diag + 1) * width, $typ, $size);
                } else {
                    let diag = -diag as usize;
                    res[width][i] = remove_ones_below!($steps[width + 1] << diag, diag, $typ);
                    res[width][i] =
                        remove_ones_above!(res[width][i], (width - diag) * width, $typ, $size);
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
                res[width][i] = remove_ones_above!(
                    $steps[width - 1] << anti_diag,
                    anti_diag * width,
                    $typ,
                    $size
                );
                res[width][i] = remove_ones_below!(
                    res[width][i],
                    if anti_diag >= width {
                        (anti_diag - width + 2) * width - 1
                    } else {
                        0
                    },
                    $typ
                );
                i += 1;
            }
            width += 1;
        }
        res
    }};
}

// TODO: Store as array of structs? Could be a speed up

// allow width of at most 26 to prevent issues with square notation (26 letters in the alphabet)
// with one extra to make some boundary conditions go away
pub const MAX_WIDTH: usize = 27;

const STEPS_U64: [u64; 64] = step_bb_for!(u64, 64);

const DIAGONALS_U64: [[u64; 64]; MAX_WIDTH] = diagonal_bb_for!(u64, 64, STEPS_U64);

const ANTI_DIAGONALS_U64: [[u64; 64]; MAX_WIDTH] = anti_diagonal_bb_for!(u64, 64, STEPS_U64);

const STEPS_U128: [u128; 128] = step_bb_for!(u128, 128);

const DIAGONALS_U128: [[u128; 128]; MAX_WIDTH] = diagonal_bb_for!(u128, 128, STEPS_U128);

const ANTI_DIAGONALS_U128: [[u128; 128]; MAX_WIDTH] = anti_diagonal_bb_for!(u128, 128, STEPS_U128);

/// A [`RawBitboard`] is something like a `u64` or `u128` (but there's nothing that would prevent implementing it for a custom
/// 256 bit struct). Unlike a `[Bitboard]`, it has no notion of size, and therefore no notion of rows, columns, etc.
/// It's also not generally a unique type, so e.g. a `RawStandardBitboard` does not offer any type safety over using a `u64`.
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
    /// A `usize` may not be enough to hold all the bits, but sometimes that's fine.
    fn to_usize(self) -> usize;

    fn steps_bb(step_size: usize) -> Self;

    fn diagonal_bb(width: usize, diag: usize) -> Self;

    fn anti_diagonal_bb(width: usize, diag: usize) -> Self;

    /// Returns a bitboard where exactly the bits in the inclusive interval [low, high] are set.
    #[must_use]
    #[inline]
    fn squares_between(low: Self, high: Self) -> Self {
        debug_assert!(low.is_single_piece());
        debug_assert!(high.is_single_piece());
        debug_assert!(low.num_trailing_zeros() <= high.num_trailing_zeros());
        ((high - Self::single_piece(0)) ^ (low - Self::single_piece(0))) | high
    }

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
    fn single_piece(idx: usize) -> Self {
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
        if self.0.is_zero() {
            None
        } else {
            Some(self.0.pop_lsb())
        }
    }
}

impl<B: RawBitboard> FusedIterator for BitIterator<B> {}

pub type RawStandardBitboard = u64;

impl RawBitboard for RawStandardBitboard {
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

    fn is_single_piece(self) -> bool {
        self.is_power_of_two()
    }
}

pub type ExtendedRawBitboard = u128;

impl RawBitboard for ExtendedRawBitboard {
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

    fn is_single_piece(self) -> bool {
        self.is_power_of_two()
    }
}

#[derive(Debug, EnumIter)]
pub enum RayDirections {
    Horizontal,
    Vertical,
    Diagonal,
    AntiDiagonal,
}

/// A bitboard extends a [`RawBitboard`] (simply a set of 64 / 128 bits) with a size,
/// which allows talking about concepts like rows.
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

    #[inline]
    fn rank_0_for(size: C::Size) -> Self {
        Self::new((R::one() << size.internal_width()) - R::one(), size)
    }

    #[inline]
    fn file_0_for(size: C::Size) -> Self {
        Self::new(R::steps_bb(size.internal_width()), size)
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
    fn diag_for_sq(sq: C, size: C::Size) -> Self {
        debug_assert!(size.coordinates_valid(sq));
        Self::new(
            R::diagonal_bb(size.internal_width(), size.internal_key(sq)),
            size,
        )
    }

    #[inline]
    fn anti_diag_for_sq(sq: C, size: C::Size) -> Self {
        debug_assert!(size.coordinates_valid(sq));
        Self::new(
            R::anti_diagonal_bb(size.internal_width(), size.internal_key(sq)),
            size,
        )
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

    // TODO: The following two methods are likely very slow. Find something faster
    /// Flips the 0th rank of the bitboard horizontally and leaves the other bits in an unspecified state.
    // TODO: Test PrimInt::reverse_bits()
    fn flip_lowest_row(self) -> Self {
        let width = self.size().width().val();
        let mut bb = self;
        let file_mask = Self::file_0_for(self.size());
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
        if flip {
            self.flip_up_down()
        } else {
            self
        }
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

    /// Returns a bitboard where exactly the bits in the inclusive interval [low, high] are set,
    /// where `low_bb` is `Self::single_piece(low)` and `high_bb` is `Self::single_piece(high)`
    #[inline]
    fn square_between(low_bb: Self, high_bb: Self) -> Self {
        debug_assert!(low_bb.is_single_piece());
        debug_assert!(high_bb.is_single_piece());
        debug_assert_eq!(low_bb.size(), high_bb.size());
        let raw = R::squares_between(low_bb.raw(), high_bb.raw());
        Self::new(raw, low_bb.size())
    }

    #[inline]
    fn hyperbola_quintessence<F>(idx: usize, blockers: Self, reverse: F, ray: Self) -> Self
    where
        F: Fn(Self) -> Self,
    {
        let piece = Self::new(R::single_piece(idx), blockers.size());
        debug_assert!(!(piece & ray).is_zero());
        let blockers = blockers & ray;
        let reversed_blockers = reverse(blockers);
        let forward = blockers.wrapping_sub(&piece);
        let backward = reversed_blockers.wrapping_sub(&reverse(piece));
        let backward = reverse(backward);
        (forward ^ backward) & ray
    }

    #[inline]
    fn hyperbola_quintessence_non_horizontal(square: C, blockers: Self, ray: Self) -> Self {
        debug_assert_eq!(blockers.size(), ray.size());
        Self::hyperbola_quintessence(
            ray.size().internal_key(square),
            blockers,
            Self::flip_up_down,
            ray,
        )
    }

    #[inline]
    fn horizontal_attacks(square: C, blockers: Self) -> Self {
        let size = blockers.size();
        let rank_bb = Self::rank_0_for(size);
        let row = square.row() as usize;
        let square = C::from_row_column(0, square.column());
        let blockers = blockers >> (blockers.internal_width() * row);
        let lowest_row = Self::hyperbola_quintessence(
            size.internal_key(square),
            blockers,
            |x| x.flip_lowest_row(),
            rank_bb,
        );
        lowest_row << (lowest_row.internal_width() * row)
    }

    #[inline]
    fn vertical_attacks(square: C, blockers: Self) -> Self {
        let file = Self::file_for(square.column(), blockers.size());
        Self::hyperbola_quintessence_non_horizontal(square, blockers, file)
    }

    #[inline]
    fn diagonal_attacks(square: C, blockers: Self) -> Self {
        Self::hyperbola_quintessence_non_horizontal(
            square,
            blockers,
            Self::diag_for_sq(square, blockers.size()),
        )
    }

    #[inline]
    fn anti_diagonal_attacks(square: C, blockers: Self) -> Self {
        Self::hyperbola_quintessence_non_horizontal(
            square,
            blockers,
            Self::anti_diag_for_sq(square, blockers.size()),
        )
    }

    /// All slider attack functions, including `rook_attacks` and `bishop_attacks`, assume that the source square
    /// is empty, so if that's not the case, they should be called with `blockers ^ square_bitboard`.
    /// Despite the names, they are not only defined for chess bitboard, as the concept of rays is far more general.
    #[inline]
    fn slider_attacks(square: C, blockers: Self, dir: RayDirections) -> Self {
        match dir {
            RayDirections::Horizontal => Self::horizontal_attacks(square, blockers),
            RayDirections::Vertical => Self::vertical_attacks(square, blockers),
            RayDirections::Diagonal => Self::diagonal_attacks(square, blockers),
            RayDirections::AntiDiagonal => Self::anti_diagonal_attacks(square, blockers),
        }
    }

    #[inline]
    fn rook_attacks(square: C, blockers: Self) -> Self {
        Self::vertical_attacks(square, blockers) | Self::horizontal_attacks(square, blockers)
    }

    #[inline]
    fn bishop_attacks(square: C, blockers: Self) -> Self {
        Self::diagonal_attacks(square, blockers) | Self::anti_diagonal_attacks(square, blockers)
    }

    #[inline]
    fn queen_attacks(square: C, blockers: Self) -> Self {
        Self::rook_attacks(square, blockers) | Self::bishop_attacks(square, blockers)
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
        self.north().east()
    }

    // TODO: Optimize?
    #[inline]
    fn south_east(self) -> Self {
        self.south().east()
    }

    #[inline]
    fn south_west(self) -> Self {
        self.south().west()
    }

    #[inline]
    fn north_west(self) -> Self {
        self.north().west()
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
        self.one_indices()
            .map(move |i| self.size().to_coordinates_unchecked(i))
    }

    #[inline]
    fn to_square(self) -> Option<C> {
        if self.is_single_piece() {
            self.size()
                .check_coordinates(
                    self.size()
                        .to_coordinates_unchecked(self.num_trailing_zeros()),
                )
                .ok()
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
        }
        Ok(())
    }
}

pub trait KnownSizeBitboard<R: RawBitboard, C: RectangularCoordinates<Size: KnownSize<C>>>:
    Bitboard<R, C>
{
    #[inline]
    fn from_raw(raw: R) -> Self {
        Bitboard::new(raw, C::Size::default())
    }

    #[inline]
    fn single_piece(idx: usize) -> Self {
        Self::from_raw(RawBitboard::single_piece(idx))
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

fn flip_lowest_byte(bb: u64) -> u64 {
    const LOOKUP: [u8; 16] = [
        0x0, 0x8, 0x4, 0xc, 0x2, 0xa, 0x6, 0xe, 0x1, 0x9, 0x5, 0xd, 0x3, 0xb, 0x7, 0xf,
    ];
    (LOOKUP[((bb >> 4) & 0xf) as usize] | (LOOKUP[(bb & 0xf) as usize] << 4)) as u64
}

#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    Eq,
    PartialEq,
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

    #[inline]
    fn raw(self) -> RawStandardBitboard {
        self.0
    }

    #[inline]
    fn size(self) -> SmallGridSize<H, W> {
        SmallGridSize::default()
    }

    #[inline]
    fn flip_lowest_row(self) -> Self {
        let in_bounds = !SmallGridBitboard::files_too_high();
        let shift: usize = 8 - W;
        // let's hope the compiler evaluates those at compile time (they can't be `const`, unfortunately)
        Self::new(flip_lowest_byte(self.raw() << shift)) & in_bounds
    }

    #[inline]
    fn flip_up_down(self) -> Self {
        let in_bounds: Self = !Self::ranks_too_high();
        let shift: usize = 8 * (8 - H);
        Self::new((self.raw() << shift).swap_bytes()) & in_bounds
    }
}

impl<const H: usize, const W: usize>
    KnownSizeBitboard<RawStandardBitboard, SmallGridSquare<H, W, 8>> for SmallGridBitboard<H, W>
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
impl<'a, R: RawBitboard, C: RectangularCoordinates> Arbitrary<'a>
    for DynamicallySizedBitboard<R, C>
{
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

/// Unfortunately, we can't simply `#derive` the implementations for all the operators

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

impl<R: RawBitboard, C: RectangularCoordinates> ShlAssign<usize>
    for DynamicallySizedBitboard<R, C>
{
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

impl<R: RawBitboard, C: RectangularCoordinates> ShrAssign<usize>
    for DynamicallySizedBitboard<R, C>
{
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

    #[inline]
    fn raw(self) -> R {
        self.raw
    }

    #[inline]
    fn size(self) -> C::Size {
        self.size
    }
}

impl<R: RawBitboard, C: RectangularCoordinates<Size: KnownSize<C>>> KnownSizeBitboard<R, C>
    for DynamicallySizedBitboard<R, C>
{
}

// pub const A_FILE_8X8: RawStandardBitboard =
//     RawStandardBitboard::from_primitive(0x0101_0101_0101_0101);
// pub const FIRST_RANK_8X8: RawStandardBitboard = RawStandardBitboard::from_primitive(0xFF);

// can be called at compile time or when constructing fairy chess rules
// width is the internal width, so boards with a width less than 8, but height <= 8 can simply use 8 as the width
const fn precompute_single_leaper_attacks(
    square_idx: usize,
    diff_1: isize,
    diff_2: isize,
    width: isize,
) -> u64 {
    assert!(diff_1 <= diff_2); // an (a, b) leaper is the same as an (b, a) leaper
    let this_piece: u64 = 1 << square_idx;
    let file = square_idx as isize % width;
    let mut attacks: u64 = 0;
    if file >= diff_1 {
        attacks |=
            (this_piece << (diff_2 * width - diff_1)) | (this_piece >> (diff_2 * width + diff_1));
    }
    if file + diff_1 < width {
        attacks |=
            (this_piece >> (diff_2 * width - diff_1)) | (this_piece << (diff_2 * width + diff_1));
    }
    if file >= diff_2 {
        attacks |= this_piece >> (diff_1 * width + diff_2);
        if diff_1 != 0 {
            attacks |= this_piece << (diff_1 * width - diff_2);
        }
    }
    if file + diff_2 < width {
        attacks |= this_piece << (diff_1 * width + diff_2);
        if diff_1 != 0 {
            attacks |= this_piece >> (diff_1 * width - diff_2);
        }
    }
    attacks
}

/// 8x8 bitboards. Not necessarily only for chess, e.g. checkers would use the same bitboard.
/// Treated specially because some operations are much simpler and faster for 8x8 boards and boards
/// with smaller width or height can use such a bitboard internally.
pub mod chessboard {
    use super::*;
    use crate::games::chess::squares::NUM_SQUARES;
    use crate::games::chess::ChessColor::*;

    pub type ChessBitboard = SmallGridBitboard<8, 8>;

    pub const KNIGHTS: [ChessBitboard; 64] = {
        let mut res: [ChessBitboard; 64] = [ChessBitboard::new(0); 64];
        let mut i = 0;
        while i < 64 {
            res[i] = ChessBitboard::new(precompute_single_leaper_attacks(i, 1, 2, 8));
            i += 1;
        }
        res
    };

    pub const KINGS: [ChessBitboard; NUM_SQUARES] = {
        let mut res = [ChessBitboard::new(0); 64];
        let mut i = 0;
        while i < 64 {
            let bb = precompute_single_leaper_attacks(i, 1, 1, 8)
                | precompute_single_leaper_attacks(i, 0, 1, 8);
            res[i] = ChessBitboard::new(bb);
            i += 1;
        }
        res
    };

    // All squares with a sup distance of 2
    pub const ATAXX_LEAPERS: [ChessBitboard; NUM_SQUARES] = {
        let mut res = [ChessBitboard::new(0); 64];
        let mut i = 0;
        while i < 64 {
            let bb = precompute_single_leaper_attacks(i, 2, 2, 8)
                | precompute_single_leaper_attacks(i, 1, 2, 8)
                | precompute_single_leaper_attacks(i, 0, 2, 8);
            res[i] = ChessBitboard::new(bb);
            i += 1;
        }
        res
    };

    pub const fn white_squares() -> ChessBitboard {
        COLORED_SQUARES[White as usize]
    }
    pub const fn black_squares() -> ChessBitboard {
        COLORED_SQUARES[Black as usize]
    }

    pub const COLORED_SQUARES: [ChessBitboard; 2] = [
        ChessBitboard::new(0x55aa_55aa_55aa_55aa),
        ChessBitboard::new(0xaa55_aa55_aa55_aa55),
    ];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::mnk::MnkBitboard;
    use crate::games::{Height, Width};
    use crate::general::bitboards::chessboard::{ChessBitboard, ATAXX_LEAPERS, KINGS};
    use crate::general::squares::GridSize;

    #[test]
    fn precomputed_test() {
        for i in 0..64 {
            let bb = ChessBitboard::single_piece(i);
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
                MnkBitboard::hyperbola_quintessence(
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
            MnkBitboard::hyperbola_quintessence(
                3,
                MnkBitboard::new(0b_0100_0001, size),
                |x| MnkBitboard::new(x.reverse_bits(), size),
                MnkBitboard::new(0xff, size),
            ),
            MnkBitboard::new(0b_0111_0111, size),
        );

        assert_eq!(
            MnkBitboard::hyperbola_quintessence(
                28,
                MnkBitboard::new(0x1234_4000_0fed, size),
                |x| MnkBitboard::new(x.reverse_bits(), size),
                MnkBitboard::new(0xffff_ffff_ffff, size),
            ),
            MnkBitboard::new(0x0000_6fff_f800, size),
        );

        assert_eq!(
            MnkBitboard::hyperbola_quintessence(
                28,
                MnkBitboard::new(0x0110_0200_0001_1111, size),
                |x| MnkBitboard::new(x.reverse_bits(), size),
                MnkBitboard::new(0x1111_1111_1111_1111, size),
            ),
            MnkBitboard::new(0x0011_1111_0111_0000, size),
        );

        assert_eq!(
            MnkBitboard::hyperbola_quintessence(
                16,
                MnkBitboard::new(0xfffe_d002_a912, size),
                |x| MnkBitboard::new(x.swap_bytes(), size),
                MnkBitboard::new(0x0101_0101_0101, size),
            ),
            MnkBitboard::new(0x0101_0100_0100, size),
        );

        assert_eq!(
            MnkBitboard::hyperbola_quintessence(
                20,
                MnkBitboard::new(0xffff_ffef_ffff, size),
                |x| MnkBitboard::new(x.swap_bytes(), size),
                MnkBitboard::new(0x_ffff_ffff_ffff, size),
            ),
            MnkBitboard::new(0, size),
        );
    }
}
