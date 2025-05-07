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
use crate::games::Size;
#[cfg(feature = "chess")]
use crate::games::chess::squares::ChessSquare;
use crate::general::bitboards::RayDirections::{AntiDiagonal, Diagonal, Horizontal, Vertical};
#[cfg(feature = "chess")]
use crate::general::bitboards::chessboard::ChessBitboard;
use crate::general::bitboards::{
    ANTI_DIAGONALS_U64, ANTI_DIAGONALS_U128, Bitboard, DIAGONALS_U64, DIAGONALS_U128, ExtendedRawBitboard, MAX_WIDTH,
    RawBitboard, RawStandardBitboard, RayDirections, STEPS_U64, STEPS_U128,
};
use crate::general::squares::RectangularCoordinates;
use crate::general::squares::RectangularSize;
use num::traits::WrappingSub;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{BitAnd, BitOr, BitXor, Sub};

/// See <https://www.chessprogramming.org/SSSE3#SSSE3Version>, peshkov's optimization
/// The Hyperbola Quintessence function, used to compute slider (actually, rider) attacks except for horizontal rays in chess.
#[inline]
fn hq<W: WithRev>(square: W, ray: W, blockers: W) -> W {
    let blockers = blockers & ray;
    let res = blockers.wrapping_sub(&square);
    res & ray
}

#[derive(Debug, Copy, Clone)]
#[repr(align(64))]
pub struct HqDataByteswap<B: RawBitboard> {
    square: B::WithRev,
    // vertical, diagonal, anti_diagonal
    rays: [B::WithRev; 3],
}

#[derive(Debug, Copy, Clone)]
/// Unlike [`HqDataByteswap`], this is not 64-byte aligned
pub struct HqDataBitReverse<B: RawBitboard> {
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
pub struct ChessSliderGenerator {
    blockers: U64AndRev,
}

#[cfg(feature = "chess")]
impl ChessSliderGenerator {
    #[inline]
    pub fn new(blockers: ChessBitboard) -> Self {
        Self { blockers: U64AndRev::new(blockers.raw(), blockers.raw().swap_bytes()) }
    }

    #[inline]
    fn hq(&self, dir: RayDirections, data: &HqDataByteswap<RawStandardBitboard>) -> U64AndRev {
        assert_ne!(dir, Horizontal);
        hq(data.square, data.rays[dir as usize - 1], self.blockers)
    }

    #[inline]
    fn finish(bb: U64AndRev) -> ChessBitboard {
        ChessBitboard::new(bb.finish(|bb| bb.swap_bytes()))
    }

    #[inline]
    pub fn horizontal_attacks(&self, square: ChessSquare) -> ChessBitboard {
        let idx = square.bb_idx();
        let rank_shift = idx & 0b111_000;
        let shifted = (self.blockers.bb() >> rank_shift) & (2 * 63);
        let res = RANK_LOOKUP[(4 * shifted) as usize + (idx & 0b000_111)] as u64;
        ChessBitboard::new(res << rank_shift)
    }

    #[inline]
    pub fn vertical_attacks(&self, square: ChessSquare) -> ChessBitboard {
        let idx = square.bb_idx();
        let res = self.hq(Vertical, &CHESS_HQ_DATA[idx]);
        Self::finish(res)
    }

    #[inline]
    pub fn diagonal_attacks(&self, square: ChessSquare) -> ChessBitboard {
        let idx = square.bb_idx();
        let res = self.hq(Diagonal, &CHESS_HQ_DATA[idx]);
        Self::finish(res)
    }

    #[inline]
    pub fn anti_diagonal_attacks(&self, square: ChessSquare) -> ChessBitboard {
        let idx = square.bb_idx();
        let res = self.hq(AntiDiagonal, &CHESS_HQ_DATA[idx]);
        Self::finish(res)
    }

    pub fn rook_attacks(&self, square: ChessSquare) -> ChessBitboard {
        self.vertical_attacks(square) | self.horizontal_attacks(square)
    }

    pub fn bishop_attacks(&self, square: ChessSquare) -> ChessBitboard {
        let idx = square.bb_idx();
        let res = self.hq(Diagonal, &CHESS_HQ_DATA[idx]) | self.hq(AntiDiagonal, &CHESS_HQ_DATA[idx]);
        Self::finish(res)
    }

    pub fn queen_attacks(&self, square: ChessSquare) -> ChessBitboard {
        let idx = square.bb_idx();
        let non_horizontal = self.hq(Vertical, &CHESS_HQ_DATA[idx])
            | self.hq(Diagonal, &CHESS_HQ_DATA[idx])
            | self.hq(AntiDiagonal, &CHESS_HQ_DATA[idx]);
        let non_horizontal = Self::finish(non_horizontal);
        self.horizontal_attacks(square) | non_horizontal
    }

    // It might be worth investigating using Kogge-Stone, which should perform equally well no matter how many sliders there are.
    // However, in normal chess positions there are rarely more than 3 rook/bishop sliders per side, so Kogge-Stone is probably
    // still slower than this approach.
    pub fn all_bishop_attacks(&self, bishop_sliders: ChessBitboard) -> ChessBitboard {
        let mut res = ChessBitboard::default();
        for square in bishop_sliders.ones() {
            res |= self.bishop_attacks(square);
        }
        res
    }

    pub fn all_rook_attacks(&self, rook_sliders: ChessBitboard) -> ChessBitboard {
        let mut res = ChessBitboard::default();
        for square in rook_sliders.ones() {
            res |= self.rook_attacks(square);
        }
        res
    }
}

// Factoring out the similarities with `ChessSliderGenerator` into a trait doesn't really reduce the amount of boilerplate
pub struct BitReverseSliderGenerator<'a, C: RectangularCoordinates, B: Bitboard<ExtendedRawBitboard, C>> {
    blockers: U128AndRev,
    size: C::Size,
    custom_rays: Option<&'a [U128AndRev]>,
    _phantom: PhantomData<B>,
}

impl<'a, C: RectangularCoordinates, B: Bitboard<ExtendedRawBitboard, C>> BitReverseSliderGenerator<'a, C, B> {
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
    type RawBitboard: RawBitboard;

    fn new(bb: Self::RawBitboard, reversed_bb: Self::RawBitboard) -> Self;

    fn bb(&self) -> Self::RawBitboard;

    fn reversed_bb(&self) -> Self::RawBitboard;

    #[inline]
    fn finish(&self, rev: impl FnOnce(Self::RawBitboard) -> Self::RawBitboard) -> Self::RawBitboard {
        self.bb() ^ rev(self.reversed_bb())
    }
}

// Sometimes (e.g. bishop attack gen), this gets auto vectorized
#[derive(Debug, Default, Clone, Copy)]
#[repr(align(16))]
pub struct U64AndRev([RawStandardBitboard; 2]);

impl WithRev for U64AndRev {
    type RawBitboard = RawStandardBitboard;

    fn new(bb: RawStandardBitboard, reversed: RawStandardBitboard) -> Self {
        Self([bb, reversed])
    }

    #[inline]
    fn bb(&self) -> RawStandardBitboard {
        self.0[0]
    }

    #[inline]
    fn reversed_bb(&self) -> Self::RawBitboard {
        self.0[1]
    }
}

impl U64AndRev {
    pub const fn unreversed(bb: RawStandardBitboard) -> Self {
        Self([bb, bb])
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

#[derive(Debug, Default, Copy, Clone)]
pub struct U128AndRev([ExtendedRawBitboard; 2]);

impl U128AndRev {
    const fn bit_reversed(bb: ExtendedRawBitboard) -> Self {
        Self([bb, bb.reverse_bits()])
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

    fn new(bb: Self::RawBitboard, reversed_bb: Self::RawBitboard) -> Self {
        Self([bb, reversed_bb])
    }

    fn bb(&self) -> Self::RawBitboard {
        self.0[0]
    }

    fn reversed_bb(&self) -> Self::RawBitboard {
        self.0[1]
    }
}

/// Because boards smaller than 8x8 still use an internal width of 8, this can be used as-is for those boards
/// `static` instead of `const` because it's relatively large and used in multiple places
#[allow(unused)]
static CHESS_HQ_DATA: [HqDataByteswap<u64>; 64] = {
    let zero = HqDataByteswap { square: U64AndRev::unreversed(0), rays: [U64AndRev::unreversed(0); 3] };
    let mut res = [zero; 64];
    const fn byteswapped(bb: RawStandardBitboard) -> U64AndRev {
        U64AndRev([bb, bb.swap_bytes()])
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::general::bitboards::chessboard::white_squares;
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
        let blockers = white_squares();
        let generator = ChessSliderGenerator::new(blockers);
        let attacks_bishop_a1 = generator.bishop_attacks(ChessSquare::from_bb_idx(0));
        assert_eq!(attacks_bishop_a1, ChessBitboard::diagonal(ChessSquare::from_bb_idx(0)) & !ChessBitboard::new(1));
        let attacks_bishop_b1 = generator.bishop_attacks(ChessSquare::from_bb_idx(1));
        assert_eq!(attacks_bishop_b1.0, 0x500);
        let attacks_rook_a1 = generator.rook_attacks(ChessSquare::from_bb_idx(0));
        assert_eq!(attacks_rook_a1.0, 0x102);
        let attacks_rook_b1 = generator.rook_attacks(ChessSquare::from_bb_idx(1));
        assert_eq!(attacks_rook_b1.0, 0x2020d);
        let attacks_queen_a1 = generator.queen_attacks(ChessSquare::from_bb_idx(0));
        assert_eq!(attacks_queen_a1, attacks_rook_a1 | attacks_bishop_a1);
        let attacks_queen_b1 = generator.queen_attacks(ChessSquare::from_bb_idx(1));
        assert_eq!(attacks_queen_b1, attacks_rook_b1 | attacks_bishop_b1);
        let e4 = ChessSquare::from_str("e4").unwrap();
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
        let blockers = STEPS_U128[10] << 16;
        let blockers = DynamicallySizedBitboard::new(blockers, GridSize::connect4());
        let generator = BitReverseSliderGenerator::new(blockers, None);
        let attacks = generator.rook_attacks(GridCoordinates { row: 3, column: 2 });
        assert!(!attacks.is_bit_set_at(23));
        let expected = (1 << 16) | (1 << 21) | (1 << 22) | (1 << 24) | (1 << 25) | (1 << 26) | (1 << 30) | (1 << 37);
        assert_eq!(attacks.raw() & ((1 << 42) - 1), expected);
    }
}
