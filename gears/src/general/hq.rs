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
use crate::general::bitboards::RayDirections::{AntiDiagonal, Diagonal, Horizontal, Vertical};
use crate::general::bitboards::{
    flip_lowest_byte, RawBitboard, RawStandardBitboard, ANTI_DIAGONALS_U64, DIAGONALS_U64, STEPS_U64,
};
use num::traits::WrappingSub;
use std::ops::{BitAnd, BitXor, Sub};

/// See <https://www.chessprogramming.org/SSSE3#SSSE3Version>, peshkov's optimization
#[inline]
pub(super) fn hq<B: RawBitboard>(
    square: WithRev<B>,
    ray: WithRev<B>,
    blockers: WithRev<B>,
    rev: impl FnOnce(B) -> B,
) -> B {
    let blockers = blockers & ray;
    let res = blockers.wrapping_sub(&square);
    (res & ray).finish(rev)
}

#[inline]
pub(super) fn hq_horizontal<B: RawBitboard>(
    square: WithRev<B>,
    blockers: WithRev<B>,
    ray: WithRev<B>,
    rev: impl FnOnce(B) -> B,
) -> B {
    // no need to `&` blockers and reversed blockers with the horizontal ray before the sub
    let res = blockers.wrapping_sub(&square);
    (res & ray).finish(rev)
}

// TODO: Make sure this gets optimized to SSSE3 instructions for RawStandardBitboard
#[derive(Debug, Default, Clone, Copy)]
pub struct WithRev<B: RawBitboard>([B; 2]);

impl<B: RawBitboard> WithRev<B> {
    pub const fn unreversed(bb: B) -> Self {
        Self([bb, bb])
    }
    pub const fn new(bb: B, reversed: B) -> Self {
        Self([bb, reversed])
    }
    pub fn bb(&self) -> B {
        self.0[0]
    }
    fn finish(self, rev: impl FnOnce(B) -> B) -> B {
        self.0[0] ^ rev(self.0[1])
    }
}

// const, unlike the bitxor operator
const fn xor(a: WithRev<u64>, b: WithRev<u64>) -> WithRev<u64> {
    WithRev([a.0[0] ^ b.0[0], a.0[1] ^ b.0[1]])
}

pub fn byte_swapped<B: RawBitboard>(bb: B) -> WithRev<B> {
    WithRev::new(bb, bb.swap_bytes())
}

impl<B: RawBitboard> Sub for WithRev<B> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self([self.0[0] - rhs.0[0], self.0[1] - rhs.0[1]])
    }
}

impl<B: RawBitboard> WrappingSub for WithRev<B> {
    fn wrapping_sub(&self, v: &Self) -> Self {
        Self([self.0[0].wrapping_sub(&v.0[0]), self.0[1].wrapping_sub(&v.0[1])])
    }
}

impl<B: RawBitboard> BitXor for WithRev<B> {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        WithRev([self.0[0] ^ rhs.0[0], self.0[1] ^ rhs.0[1]])
    }
}

impl<B: RawBitboard> BitAnd for WithRev<B> {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self([self.0[0] & rhs.0[0], self.0[1] & rhs.0[1]])
    }
}

#[derive(Debug, Default, Copy, Clone)]
#[repr(align(64))]
pub struct SliderData<B: RawBitboard> {
    square: WithRev<B>,
    // vertical, diagonal, anti_diagonal
    rays: [WithRev<B>; 3],
}

/// Because boards smaller than 8x8 still use an internal width of 8, this can be used as-is for those boards
/// `static` instead of `const` because it's large and used in multiple places
static PRECOMPUTED_HQ_DATA: [SliderData<u64>; 64] = {
    let zero = SliderData { square: WithRev::unreversed(0), rays: [WithRev::unreversed(0); 3] };
    let mut res = [zero; 64];
    const fn byteswapped(bb: RawStandardBitboard) -> WithRev<RawStandardBitboard> {
        WithRev::new(bb, bb.swap_bytes())
    }
    let mut i = 0;
    while i < 64 {
        let sq = byteswapped(1_u64 << i);
        res[i].square = sq;
        res[i].rays[Vertical as usize - 1] = xor(byteswapped(STEPS_U64[8] << (i % 8)), sq);
        res[i].rays[Diagonal as usize - 1] = xor(byteswapped(DIAGONALS_U64[8][i]), sq);
        res[i].rays[AntiDiagonal as usize - 1] = xor(byteswapped(ANTI_DIAGONALS_U64[8][i]), sq);
        i += 1;
    }
    res
};

const _: () = assert!(Horizontal as usize == 0);

// TODO: Construct the blockers once in movegen and reuse
#[inline]
pub(super) fn slider_attacks_u64_non_horizontal<const DIR: usize>(
    square_idx: usize,
    blockers: RawStandardBitboard,
) -> RawStandardBitboard {
    hq_u64_non_horizontal::<DIR>(square_idx, byte_swapped(blockers))
}

#[inline]
pub(super) fn hq_u64_non_horizontal<const DIR: usize>(
    square_idx: usize,
    blockers: WithRev<RawStandardBitboard>,
) -> RawStandardBitboard {
    debug_assert_ne!(DIR, Horizontal as usize);
    let precomputed = &PRECOMPUTED_HQ_DATA[square_idx];
    hq::<RawStandardBitboard>(precomputed.square, precomputed.rays[DIR - 1], blockers, |bb| bb.swap_bytes())
}

#[inline]
pub(super) fn slider_attacks_u64_horizontal(square_idx: usize, blockers: RawStandardBitboard) -> RawStandardBitboard {
    let sq = square_idx % 8;
    let blockers = (blockers >> (square_idx / 8 * 8)) & !(1 << sq);
    let flip = |bb| flip_lowest_byte(bb);
    let blockers = WithRev::new(blockers, flip(blockers));
    let ray = WithRev::unreversed(0xff);
    let sq = PRECOMPUTED_HQ_DATA[sq].square.0[0];
    let sq = WithRev::new(sq, flip(sq)); // TODO: flip(sq) can be precomputed
    hq_horizontal(sq, blockers, ray, flip) << (square_idx / 8 * 8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_test() {
        let bb = byte_swapped(0x100000100000001);
        let attacks = hq_u64_non_horizontal::<{ Vertical as usize }>(16, bb);
        assert_eq!(attacks, 0x101000101);
        let attacks = slider_attacks_u64_non_horizontal::<{ Vertical as usize }>(17, bb.bb());
        assert_eq!(attacks, 0x202020202000202);

        let bb = 0xc5;
        let attacks = slider_attacks_u64_horizontal(1, bb);
        assert_eq!(attacks, 0b101);

        let bb = bb << 16;
        let attacks = slider_attacks_u64_horizontal(1 + 16, bb);
        assert_eq!(attacks, 0b101 << 16);

        let bb = byte_swapped(0x240048005002005);
        let attacks = hq_u64_non_horizontal::<{ Diagonal as usize }>(27, bb);
        assert_eq!(attacks, 0x40201000040201);
    }
}
