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
use crate::games::SizeTrait;
#[cfg(feature = "chess")]
use crate::games::chess::squares::Square;
use crate::general::bitboards::RayDirections::{AntiDiagonal, Diagonal, Horizontal, Vertical};
#[cfg(feature = "chess")]
use crate::general::bitboards::chessboard::Bitboard;
use crate::general::bitboards::{
    ANTI_DIAGONALS_U64, ANTI_DIAGONALS_U128, BitboardTrait, DIAGONALS_U64, DIAGONALS_U128, ExtendedRawBitboard,
    KnownSizeBitboard, MAX_WIDTH, RawBitboardTrait, RawStandardBitboard, RayDirections, STEPS_U64, STEPS_U128,
};
use crate::general::squares::RectangularCoordinates;
use crate::general::squares::RectangularSize;
use num::traits::WrappingSub;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{BitAnd, BitOr, BitXor, Sub};

/// See <https://www.chessprogramming.org/SSSE3#SSSE3Version>, peshkov's optimization
/// The Hyperbola Quintessence function, used to compute slider and rider attacks except for horizontal rays on u64 bitboards.
#[inline]
fn hq<W: WithRev>(square: W, ray: W, blockers: W) -> W {
    let blockers = blockers & ray;
    let res = blockers.wrapping_sub(&square);
    res & ray
}

#[derive(Debug, Copy, Clone)]
#[repr(align(64))]
pub struct HqDataByteswap<B: RawBitboardTrait> {
    square: B::WithRev,
    // vertical, diagonal, anti_diagonal
    rays: [B::WithRev; 3],
}

#[derive(Debug, Copy, Clone)]
/// Unlike [`HqDataByteswap`], this is not 64-byte aligned
pub struct HqDataBitReverse<B: RawBitboardTrait> {
    square: B::WithRev,
    rays: [B::WithRev; 4],
}
type U128BitReverseHq = HqDataBitReverse<ExtendedRawBitboard>;

impl U128BitReverseHq {
    const fn zeroed() -> Self {
        Self { square: U128AndRev::bit_reversed(0), rays: [U128AndRev::bit_reversed(0); 4] }
    }
}

#[cfg(feature = "chess")]
#[derive(Debug, Copy, Clone)]
pub struct ChessSliderGenerator {
    blockers: U64AndRev,
}

#[cfg(feature = "chess")]
impl ChessSliderGenerator {
    #[inline]
    pub fn new(blockers: Bitboard) -> Self {
        Self { blockers: U64AndRev::new(blockers.raw(), blockers.raw().swap_bytes()) }
    }

    #[inline]
    fn hq(&self, dir: RayDirections, data: &HqDataByteswap<RawStandardBitboard>) -> U64AndRev {
        assert_ne!(dir, Horizontal);
        hq(data.square, data.rays[dir as usize - 1], self.blockers)
    }

    #[inline]
    fn finish(bb: U64AndRev) -> Bitboard {
        Bitboard::new(bb.finish(|bb| bb.swap_bytes()))
    }

    #[inline]
    pub fn horizontal_attacks(&self, square: Square) -> Bitboard {
        let idx = square.bb_idx();
        let rank_shift = idx & 0b111_000;
        // TODO: Probably better to not use an sse instruction for this; keep a separate blocker bb?
        let shifted = (self.blockers.bb() >> rank_shift) & 0b0111_1110;
        let res = RANK_LOOKUP[(4 * shifted) as usize + (idx & 0b000_111)] as u64;
        Bitboard::new(res << rank_shift)
    }

    #[inline]
    pub fn vertical_attacks(&self, square: Square) -> Bitboard {
        let idx = square.bb_idx();
        let res = self.hq(Vertical, &CHESS_HQ_DATA[idx]);
        Self::finish(res)
    }

    #[inline]
    pub fn diagonal_attacks(&self, square: Square) -> Bitboard {
        let idx = square.bb_idx();
        let res = self.hq(Diagonal, &CHESS_HQ_DATA[idx]);
        Self::finish(res)
    }

    #[inline]
    pub fn anti_diagonal_attacks(&self, square: Square) -> Bitboard {
        let idx = square.bb_idx();
        let res = self.hq(AntiDiagonal, &CHESS_HQ_DATA[idx]);
        Self::finish(res)
    }

    pub fn rook_attacks(&self, square: Square) -> Bitboard {
        self.vertical_attacks(square) | self.horizontal_attacks(square)
    }

    // TODO: Can probably use 256 bit SIMD to calculate diagonals and anti diagonals in parallel
    pub fn bishop_attacks(&self, square: Square) -> Bitboard {
        let idx = square.bb_idx();
        let res = self.hq(Diagonal, &CHESS_HQ_DATA[idx]) | self.hq(AntiDiagonal, &CHESS_HQ_DATA[idx]);
        Self::finish(res)
    }

    pub fn queen_attacks(&self, square: Square) -> Bitboard {
        let idx = square.bb_idx();
        let non_horizontal = self.hq(Vertical, &CHESS_HQ_DATA[idx])
            | self.hq(Diagonal, &CHESS_HQ_DATA[idx])
            | self.hq(AntiDiagonal, &CHESS_HQ_DATA[idx]);
        let non_horizontal = Self::finish(non_horizontal);
        self.horizontal_attacks(square) | non_horizontal
    }
}

/// See <https://www.chessprogramming.org/Kogge-Stone_Algorithm>
pub fn kogge_stone<const DIR: usize>(
    sliders: Bitboard,
    empty: Bitboard,
    fwd_filter: Bitboard,
    bwd_filter: Bitboard,
) -> Bitboard {
    let mut forward = sliders;
    let mut fwd_allowed = empty & fwd_filter;
    let mut backward = sliders;
    let mut bwd_allowed = empty & bwd_filter;
    forward |= fwd_allowed & (forward << DIR);
    backward |= bwd_allowed & (backward >> DIR);
    fwd_allowed &= fwd_allowed << DIR;
    bwd_allowed &= bwd_allowed >> DIR;
    forward |= fwd_allowed & (forward << 2 * DIR);
    backward |= bwd_allowed & (backward >> 2 * DIR);
    fwd_allowed &= fwd_allowed << 2 * DIR;
    bwd_allowed &= bwd_allowed >> 2 * DIR;
    forward |= fwd_allowed & (forward << 4 * DIR);
    backward |= bwd_allowed & (backward >> 4 * DIR);
    ((forward << DIR) & fwd_filter) | ((backward >> DIR) & bwd_filter)
}

pub fn all_rook_attacks(sliders: Bitboard, empty: Bitboard) -> Bitboard {
    let all = Bitboard::new(!0);
    let vertical = kogge_stone::<8>(sliders, empty, all, all);
    let horizontal = kogge_stone::<1>(sliders, empty, !Bitboard::file_0(), !Bitboard::file(7));
    horizontal | vertical
}

pub fn all_bishop_attacks(sliders: Bitboard, empty: Bitboard) -> Bitboard {
    let not_a_file = !Bitboard::file_0();
    let not_h_file = !Bitboard::file(7);
    let diagonal = kogge_stone::<9>(sliders, empty, not_a_file, not_h_file);
    let anti_diagonal = kogge_stone::<7>(sliders, empty, not_h_file, not_a_file);
    diagonal | anti_diagonal
}

// Factoring out the similarities with `ChessSliderGenerator` into a trait doesn't really reduce the amount of boilerplate
pub struct BitReverseSliderGenerator<'a, C: RectangularCoordinates, B: BitboardTrait<ExtendedRawBitboard, C>> {
    blockers: U128AndRev,
    size: C::Size,
    custom_rays: Option<&'a [U128AndRev]>,
    _phantom: PhantomData<B>,
}

impl<'a, C: RectangularCoordinates, B: BitboardTrait<ExtendedRawBitboard, C>> BitReverseSliderGenerator<'a, C, B> {
    pub fn new(blockers: B, rays: Option<&'a [U128AndRev]>) -> Self {
        Self {
            blockers: U128AndRev::new(blockers.raw(), blockers.raw().reverse_bits()),
            size: blockers.size(),
            custom_rays: rays,
            _phantom: PhantomData,
        }
    }

    #[inline]
    fn data(&self) -> &[U128BitReverseHq; 128] {
        &BIT_REVERSE_HQ_DATA[self.size.width().val()]
    }

    #[inline]
    fn hq(&self, dir: RayDirections, data: &HqDataBitReverse<ExtendedRawBitboard>) -> U128AndRev {
        hq(data.square, data.rays[dir as usize], self.blockers)
    }

    #[inline]
    fn finish(&self, bb: U128AndRev) -> B {
        B::new(bb.finish(|bb| bb.reverse_bits()), self.size)
    }

    #[inline]
    pub fn forward_attacks(&self, square: C, flip: bool) -> B {
        let idx = self.size.internal_key(square);
        let sq = self.data()[idx].square;
        // this is also known as the `o ^ (o - 2r)` trick, <https://www.chessprogramming.org/Subtracting_a_Rook_from_a_Blocking_Piece>
        let res = if !flip {
            let ray = self.data()[idx].rays[Vertical as usize].bb();
            let blockers = self.blockers.bb() & ray;
            ((blockers.wrapping_sub(sq.bb() * 2)) ^ blockers) & ray
        } else {
            let ray = self.data()[idx].rays[Vertical as usize].rev_bb();
            let blockers = self.blockers.rev_bb() & ray;
            (((blockers.wrapping_sub(sq.rev_bb() * 2)) ^ blockers) & ray).reverse_bits()
        };
        B::new(res, self.size)
    }

    #[inline]
    pub fn horizontal_attacks(&self, square: C) -> B {
        let idx = self.size.internal_key(square);
        let res = self.hq(Horizontal, &self.data()[idx]);
        self.finish(res)
    }

    #[inline]
    pub fn vertical_attacks(&self, square: C) -> B {
        let idx = self.size.internal_key(square);
        let res = self.hq(Vertical, &self.data()[idx]);
        self.finish(res)
    }

    #[inline]
    pub fn diagonal_attacks(&self, square: C) -> B {
        let idx = self.size.internal_key(square);
        let res = self.hq(Diagonal, &self.data()[idx]);
        self.finish(res)
    }

    #[inline]
    pub fn anti_diagonal_attacks(&self, square: C) -> B {
        let idx = self.size.internal_key(square);
        let res = self.hq(AntiDiagonal, &self.data()[idx]);
        self.finish(res)
    }

    #[inline]
    pub fn rook_attacks(&self, square: C) -> B {
        let idx = self.size.internal_key(square);
        let res = self.hq(Horizontal, &self.data()[idx]) | self.hq(Vertical, &self.data()[idx]);
        self.finish(res)
    }

    #[inline]
    pub fn bishop_attacks(&self, square: C) -> B {
        let idx = self.size.internal_key(square);
        let res = self.hq(Diagonal, &self.data()[idx]) | self.hq(AntiDiagonal, &self.data()[idx]);
        self.finish(res)
    }

    #[inline]
    pub fn queen_attacks(&self, square: C) -> B {
        let idx = self.size.internal_key(square);
        let res = self.hq(Diagonal, &self.data()[idx])
            | self.hq(AntiDiagonal, &self.data()[idx])
            | self.hq(Vertical, &self.data()[idx])
            | self.hq(Horizontal, &self.data()[idx]);
        self.finish(res)
    }

    pub fn custom_attacks(&self, square: C, ray_indices_bitset: u64) -> B {
        let rays = self.custom_rays.expect("Must have custom rays");
        let mut res = U128AndRev::new(0, 0);
        let idx = self.size.internal_key(square);
        let sq = self.data()[idx].square;
        for index in ray_indices_bitset.one_indices() {
            res = res | hq(sq, rays[index], self.blockers);
        }
        self.finish(res)
    }
}

pub trait WithRev: Debug + Copy + Clone + BitAnd<Output = Self> + BitOr<Output = Self> + WrappingSub {
    type RawBitboard: RawBitboardTrait;

    fn bb(&self) -> Self::RawBitboard;

    fn rev_bb(&self) -> Self::RawBitboard;

    #[inline]
    fn finish(&self, rev: impl FnOnce(Self::RawBitboard) -> Self::RawBitboard) -> Self::RawBitboard {
        self.bb() ^ rev(self.rev_bb())
    }
}

#[cfg(all(feature = "unsafe", target_arch = "x86_64", target_feature = "sse2"))]
mod sse2 {
    use super::*;
    use std::arch::x86_64::__m128i;
    #[allow(unused_imports)]
    use std::arch::x86_64::{
        _mm_and_si128, _mm_cvtsi128_si64, _mm_or_si128, _mm_set_epi64x, _mm_shuffle_epi8, _mm_sub_epi64, _mm_xor_si128,
    };
    use std::mem::transmute;

    #[derive(Debug, Clone, Copy)]
    pub struct U64AndRev(__m128i);

    impl WithRev for U64AndRev {
        type RawBitboard = RawStandardBitboard;

        #[inline]
        fn bb(&self) -> RawStandardBitboard {
            unsafe { _mm_cvtsi128_si64(self.0) as RawStandardBitboard }
        }

        #[inline]
        fn rev_bb(&self) -> Self::RawBitboard {
            unreachable!("Should not need to be called for this implementation")
        }

        #[cfg(target_feature = "ssse3")]
        fn finish(&self, _rev: impl FnOnce(Self::RawBitboard) -> Self::RawBitboard) -> Self::RawBitboard {
            unsafe {
                let byteswap_reverse = _mm_set_epi64x(i64::MAX, 0x08_09_0a_0b_0c_0d_0e_0f);
                let reverse = _mm_shuffle_epi8(self.0, byteswap_reverse);
                let r = _mm_xor_si128(self.0, reverse);
                _mm_cvtsi128_si64(r) as RawStandardBitboard
            }
        }

        #[cfg(not(target_feature = "ssse3"))]
        fn finish(&self, rev: impl FnOnce(Self::RawBitboard) -> Self::RawBitboard) -> Self::RawBitboard {
            // SAFETY: alignment isn't a concern for transmute
            unsafe {
                let [bb, reversed] = transmute(self.0);
                bb ^ rev(reversed)
            }
        }
    }

    impl U64AndRev {
        pub const fn new(bb: RawStandardBitboard, reversed: RawStandardBitboard) -> Self {
            // unsafe {
            //     let bb = _mm_cvtsi64_si128(bb as i64);
            //     let reversed = _mm_cvtsi64_si128(reversed as i64);
            //     Self(x86_64::_mm_unpacklo_epi64(bb, reversed))
            // }

            // intrinsics aren't const yet in stable rust
            // SAFETY: __m128i is simply a "bag of bits" and alignment doesn't matter for transmute
            unsafe { Self(transmute([bb, reversed])) }
        }
    }

    impl Sub for U64AndRev {
        type Output = Self;

        #[inline]
        fn sub(self, rhs: Self) -> Self::Output {
            unsafe { Self(_mm_sub_epi64(self.0, rhs.0)) }
        }
    }

    impl WrappingSub for U64AndRev {
        #[inline]
        fn wrapping_sub(&self, rhs: &Self) -> Self {
            unsafe { Self(_mm_sub_epi64(self.0, rhs.0)) }
        }
    }

    impl BitXor for U64AndRev {
        type Output = Self;

        #[inline]
        fn bitxor(self, rhs: Self) -> Self::Output {
            unsafe { Self(_mm_xor_si128(self.0, rhs.0)) }
        }
    }

    impl BitAnd for U64AndRev {
        type Output = Self;

        #[inline]
        fn bitand(self, rhs: Self) -> Self::Output {
            unsafe { Self(_mm_and_si128(self.0, rhs.0)) }
        }
    }

    impl BitOr for U64AndRev {
        type Output = Self;

        #[inline]
        fn bitor(self, rhs: Self) -> Self::Output {
            unsafe { Self(_mm_or_si128(self.0, rhs.0)) }
        }
    }
}

mod fallback {
    use super::*;

    #[derive(Debug, Default, Clone, Copy)]
    #[repr(align(16))]
    #[allow(unused)]
    pub struct U64AndRev([RawStandardBitboard; 2]);

    impl WithRev for U64AndRev {
        type RawBitboard = RawStandardBitboard;

        #[inline]
        fn bb(&self) -> RawStandardBitboard {
            self.0[0]
        }

        #[inline]
        fn rev_bb(&self) -> Self::RawBitboard {
            self.0[1]
        }
    }

    impl U64AndRev {
        #[allow(unused)]
        pub const fn new(bb: RawStandardBitboard, reversed: RawStandardBitboard) -> Self {
            Self([bb, reversed])
        }
    }

    impl Sub for U64AndRev {
        type Output = Self;

        #[inline]
        fn sub(self, rhs: Self) -> Self::Output {
            Self([self.0[0] - rhs.0[0], self.0[1] - rhs.0[1]])
        }
    }

    impl WrappingSub for U64AndRev {
        #[inline]
        fn wrapping_sub(&self, v: &Self) -> Self {
            Self([self.0[0].wrapping_sub(v.0[0]), self.0[1].wrapping_sub(v.0[1])])
        }
    }

    impl BitXor for U64AndRev {
        type Output = Self;

        #[inline]
        fn bitxor(self, rhs: Self) -> Self::Output {
            Self([self.0[0] ^ rhs.0[0], self.0[1] ^ rhs.0[1]])
        }
    }

    impl BitAnd for U64AndRev {
        type Output = Self;

        #[inline]
        fn bitand(self, rhs: Self) -> Self::Output {
            Self([self.0[0] & rhs.0[0], self.0[1] & rhs.0[1]])
        }
    }

    impl BitOr for U64AndRev {
        type Output = Self;

        #[inline]
        fn bitor(self, rhs: Self) -> Self::Output {
            Self([self.0[0] | rhs.0[0], self.0[1] | rhs.0[1]])
        }
    }
}

#[cfg(not(all(feature = "unsafe", target_arch = "x86_64", target_feature = "sse2")))]
pub type U64AndRev = fallback::U64AndRev;

#[cfg(all(feature = "unsafe", target_arch = "x86_64", target_feature = "sse2"))]
pub type U64AndRev = sse2::U64AndRev;

#[derive(Debug, Default, Copy, Clone)]
pub struct U128AndRev([ExtendedRawBitboard; 2]);

impl U128AndRev {
    const fn bit_reversed(bb: ExtendedRawBitboard) -> Self {
        Self([bb, bb.reverse_bits()])
    }

    const fn new(bb: ExtendedRawBitboard, reversed_bb: ExtendedRawBitboard) -> Self {
        Self([bb, reversed_bb])
    }
}

impl BitAnd for U128AndRev {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self([self.0[0] & rhs.0[0], self.0[1] & rhs.0[1]])
    }
}

impl BitOr for U128AndRev {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self([self.0[0] | rhs.0[0], self.0[1] | rhs.0[1]])
    }
}

impl WrappingSub for U128AndRev {
    fn wrapping_sub(&self, v: &Self) -> Self {
        Self([self.0[0].wrapping_sub(v.0[0]), self.0[1].wrapping_sub(v.0[1])])
    }
}

impl Sub<Self> for U128AndRev {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self([self.0[0] - rhs.0[0], self.0[1] - rhs.0[1]])
    }
}

impl WithRev for U128AndRev {
    type RawBitboard = ExtendedRawBitboard;

    fn bb(&self) -> Self::RawBitboard {
        self.0[0]
    }

    fn rev_bb(&self) -> Self::RawBitboard {
        self.0[1]
    }
}

/// Because boards smaller than 8x8 still use an internal width of 8, this can be used as-is for those boards
/// `static` instead of `const` because it's relatively large and used in multiple places
#[allow(unused)]
static CHESS_HQ_DATA: [HqDataByteswap<u64>; 64] = {
    let zero = HqDataByteswap { square: U64AndRev::new(0, 0), rays: [U64AndRev::new(0, 0); 3] };
    let mut res = [zero; 64];
    const fn byteswapped(bb: RawStandardBitboard) -> U64AndRev {
        U64AndRev::new(bb, bb.swap_bytes())
    }
    let mut i = 0;
    while i < 64 {
        let sq = 1_u64 << i;
        res[i].square = byteswapped(sq);
        res[i].rays[Vertical as usize - 1] = byteswapped((STEPS_U64[8] << (i % 8)) ^ sq);
        res[i].rays[Diagonal as usize - 1] = byteswapped((DIAGONALS_U64[8][i]) ^ sq);
        res[i].rays[AntiDiagonal as usize - 1] = byteswapped((ANTI_DIAGONALS_U64[8][i]) ^ sq);
        i += 1;
    }
    res
};

#[allow(unused)]
static RANK_LOOKUP: [u8; 8 * (1 << (8 - 2))] = {
    let mut res = [0; 64 * 8];
    let mut i = 0;
    while i < 64 {
        let mut sq = 0;
        while sq < 8 {
            let sq_bb = 1 << sq;
            let blockers: u8 = (i << 1) & !sq_bb;
            let attacks = blockers.wrapping_sub(sq_bb);
            let rev = blockers.reverse_bits().wrapping_sub(sq_bb.reverse_bits());
            let attacks = rev.reverse_bits() ^ attacks;
            res[i as usize * 8 + sq as usize] = attacks;
            sq += 1;
        }
        i += 1;
    }
    res
};

const _: () = assert!(Horizontal as usize == 0);

// This only depends on the width, not the height, which means that bits above the border of the board can be set.
// However, these bitboards are always eventually combined with a blocker bitboard using `&`, which clears those bits
// This needs 16 * 2 * 5 * 128 * 27 = 540 KiB
// TODO: Generate on demand for the required width instead of precomputing?
static BIT_REVERSE_HQ_DATA: [[U128BitReverseHq; 128]; MAX_WIDTH] = {
    let mut res = [[U128BitReverseHq::zeroed(); 128]; MAX_WIDTH];
    let mut width = 1;
    while width < MAX_WIDTH {
        let mut sq = 0;
        while sq < 128 {
            let entry = &mut res[width][sq];
            let sq_bb = 1 << sq;
            entry.square = U128AndRev::bit_reversed(sq_bb);
            entry.rays[Horizontal as usize] =
                U128AndRev::bit_reversed((((1 << width) - 1) << ((sq / width) * width)) ^ sq_bb);
            entry.rays[Vertical as usize] = U128AndRev::bit_reversed((STEPS_U128[width] << (sq % width)) ^ sq_bb);
            entry.rays[Diagonal as usize] = U128AndRev::bit_reversed(DIAGONALS_U128[width][sq] ^ sq_bb);
            entry.rays[AntiDiagonal as usize] = U128AndRev::bit_reversed(ANTI_DIAGONALS_U128[width][sq] ^ sq_bb);
            sq += 1;
        }
        width += 1;
    }
    res
};

// Attacks that go to the right on a cylindrical board.
// Attacks that go to the left can be calculated by calling `.reverse_bits` before and after.
// This special case function is necessary because attacks that wrap around break many assumptions of regular hq, such as indices within
// a ray being monotonic.
pub fn hq_right_horizontal_cylinder(
    file: usize,
    w: usize,
    step: usize,
    all_blockers: ExtendedRawBitboard,
) -> ExtendedRawBitboard {
    debug_assert!(step > 0 && step <= w, "{step} {w}");
    let mut res = ExtendedRawBitboard::default();
    let piece = ExtendedRawBitboard::single_piece_at(file);
    let mut start = piece;
    loop {
        debug_assert!(start.is_single_piece());
        let ray = STEPS_U128[step] << (start.trailing_zeros() as usize + step);
        let blockers = all_blockers & ray;
        let forward = (blockers.wrapping_sub(start) ^ blockers) & ray;
        let wrap_around = forward & (!0 << w);
        res |= forward & !wrap_around;
        let wrap_around = (wrap_around >> w) & ((1 << w) - 1) & !res;
        start = wrap_around.lsb();
        if start.is_zero() {
            break;
        }
        res |= start;
        if all_blockers & start != 0 {
            break;
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::general::bitboards::chessboard::light_squares;
    use crate::general::bitboards::{DynamicallySizedBitboard, KnownSizeBitboard};
    use crate::general::squares::{GridCoordinates, GridSize};
    use std::str::FromStr;

    #[test]
    fn rank_lookup_test() {
        assert_eq!(RANK_LOOKUP[0], !1_u8);
        assert_eq!(RANK_LOOKUP[1], !2_u8);
        assert_eq!(RANK_LOOKUP[2], !4_u8);
        assert_eq!(RANK_LOOKUP[7], !128_u8);
        assert_eq!(RANK_LOOKUP[8], 2_u8);
        assert_eq!(RANK_LOOKUP[9], !2_u8);
        assert_eq!(RANK_LOOKUP[10], !5_u8);
        assert_eq!(RANK_LOOKUP[17], 5);
        assert_eq!(RANK_LOOKUP[0b010_101 * 8 + 3], 0b0011_0110);
    }

    #[test]
    fn chess_test() {
        let blockers = light_squares();
        let generator = ChessSliderGenerator::new(blockers);
        let attacks_bishop_a1 = generator.bishop_attacks(Square::from_bb_idx(0));
        assert_eq!(attacks_bishop_a1, Bitboard::diagonal(Square::from_bb_idx(0)) & !Bitboard::new(1));
        let attacks_bishop_b1 = generator.bishop_attacks(Square::from_bb_idx(1));
        assert_eq!(attacks_bishop_b1.0, 0x500);
        let attacks_rook_a1 = generator.rook_attacks(Square::from_bb_idx(0));
        assert_eq!(attacks_rook_a1.0, 0x102);
        let attacks_rook_b1 = generator.rook_attacks(Square::from_bb_idx(1));
        assert_eq!(attacks_rook_b1.0, 0x2020d);
        let attacks_queen_a1 = generator.queen_attacks(Square::from_bb_idx(0));
        assert_eq!(attacks_queen_a1, attacks_rook_a1 | attacks_bishop_a1);
        let attacks_queen_b1 = generator.queen_attacks(Square::from_bb_idx(1));
        assert_eq!(attacks_queen_b1, attacks_rook_b1 | attacks_bishop_b1);
        let e4 = Square::from_str("e4").unwrap();
        assert!(blockers.is_bit_set_at(e4.bb_idx()));
        let attacks_rook_e4 = generator.rook_attacks(e4);
        let e4_bb = e4.bb();
        let mut vertical = e4_bb.north() | e4_bb.south();
        vertical |= vertical.north() | vertical.south();
        let mut horizontal = e4_bb.west() | e4_bb.east();
        horizontal |= horizontal.west() | horizontal.east();
        let expected = vertical ^ horizontal;
        assert_eq!(attacks_rook_e4, expected);
    }

    #[test]
    fn extended_test() {
        // . x . . . . .
        // . . . . . . .
        // . . . . . x .
        // . . x . . . .
        // . . . . . . .
        // . . . . . . .
        let blockers = STEPS_U128[10] << 16;
        let blockers = DynamicallySizedBitboard::new(blockers, GridSize::connect4());
        let generator = BitReverseSliderGenerator::new(blockers, None);
        let attacks = generator.rook_attacks(GridCoordinates { row: 3, column: 2 });
        assert!(!attacks.is_bit_set_at(23));
        let expected = (1 << 16) | (1 << 21) | (1 << 22) | (1 << 24) | (1 << 25) | (1 << 26) | (1 << 30) | (1 << 37);
        assert_eq!(attacks.raw() & ((1 << 42) - 1), expected);

        let attacks = generator.forward_attacks(GridCoordinates { row: 0, column: 1 }, false);
        assert_eq!(attacks.raw(), (STEPS_U128[7] << 8).remove_ones_above(42), "{attacks}");
        let attacks = generator.forward_attacks(GridCoordinates { row: 0, column: 1 }, true);
        assert!(attacks.is_zero(), "{attacks}");
        let attacks = generator.forward_attacks(GridCoordinates { row: 1, column: 5 }, false);
        assert_eq!(attacks.raw(), ((1 << 14) | (1 << 21)) << 5, "{attacks}");
        let attacks = generator.forward_attacks(GridCoordinates { row: 1, column: 5 }, true);
        assert_eq!(attacks.raw(), 1 << 5, "{attacks}");
        let attacks = generator.forward_attacks(GridCoordinates { row: 4, column: 2 }, false);
        assert_eq!(attacks.raw().remove_ones_above(42), 1 << 37, "{attacks}");
        let attacks = generator.forward_attacks(GridCoordinates { row: 4, column: 2 }, true);
        assert_eq!(attacks.raw(), (1 << 16) | (1 << 23), "{attacks}");
    }

    #[test]
    fn test_all_rook_attacks() {
        let rooks = Bitboard::new(0x8000100000028000);
        let blockers = Bitboard::new(0x8002101801228080);
        let attacks = all_rook_attacks(rooks, !blockers);
        let expected = Bitboard::new(0xff92ef9282bdff82);
        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_all_bishop_attacks() {
        let bishops = Bitboard::new(0x8000100000021008);
        let blockers = Bitboard::new(0x8000100004821208);
        let attacks = all_bishop_attacks(bishops, !blockers);
        let expected = Bitboard::new(0x446820b84dae1728);
        assert_eq!(attacks, expected);
    }

    #[test]
    fn test_hq_right_horizontal_cylinder() {
        let blockers = !0;
        let attacks = hq_right_horizontal_cylinder(0, 8, 1, blockers);
        debug_assert_eq!(attacks, 2);
        let blockers = 17;
        let attacks = hq_right_horizontal_cylinder(1, 9, 1, blockers);
        debug_assert_eq!(attacks, 0b11100);
        let blockers = 0;
        let attacks = hq_right_horizontal_cylinder(2, 10, 1, blockers);
        debug_assert_eq!(attacks, 0b11_1111_1111);
        let blockers = 2;
        let attacks = hq_right_horizontal_cylinder(4, 6, 1, blockers);
        debug_assert_eq!(attacks, 0b10_0011);
        for w in 2..=26 {
            for file in 0..w {
                for step in 1..(w + 1) {
                    for blocker_step in 1..(w + 3) {
                        for blocker_start in 0..(w + 3) {
                            let blockers = STEPS_U128[blocker_step] << blocker_start;
                            let attacks = hq_right_horizontal_cylinder(file, w, step, blockers);
                            let mut expected = 0;
                            let mut i = (file + step) % w;
                            loop {
                                expected |= 1 << i;
                                if blockers.is_bit_set_at(i) || i == file {
                                    break;
                                }
                                i = (i + step) % w;
                            }
                            assert_eq!(
                                attacks, expected,
                                "w {w}, file {file}, step {step}, blocker_step {blocker_step}, blocker_start {blocker_start}, blockers {blockers:x}"
                            );
                        }
                    }
                }
            }
        }
    }
}
