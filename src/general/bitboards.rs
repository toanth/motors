extern crate num;

use std::fmt::{Debug, Formatter};
use std::num::Wrapping;

use derive_more::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, ShlAssign, Shr,
    ShrAssign, Sub,
};
use num::{One, PrimInt, Unsigned, Zero};
use strum_macros::EnumIter;

use crate::games::{GridCoordinates, GridSize, RectangularSize, Size};
use crate::general::common::{pop_lsb128, pop_lsb64};

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
// to run a sprt, only increasing bench might not be worth it

const MAX_WIDTH: usize = 12;

const STEPS: [u128; 128] = compute_step_bbs();

const DIAGONALS: [[u128; 128]; MAX_WIDTH] = compute_diagonal_bbs();

const ANTI_DIAGONALS: [[u128; 128]; MAX_WIDTH] = compute_anti_diagonal_bbs();

//
// const fn compute_diag_bb_chessboard(square: GridCoordinates) -> ChessBitboard {
//     let mut row = square.row();
//     let mut column = square.column();
//     let offset = min(row, column);
//     row -= offset;
//     column -= offset;
//     let mut res = 0;
//     while max(row, column) < 8 && min(row, column) >= 0 {
//         res |= 1 << (8 * row + column);
//         row += 1;
//         column += 1;
//     }
//     res
// }
//
// fn compute_anti_diag_bb(square: GridCoordinates) -> ChessBitboard {
//     let square = GridCoordinates::new(7 - square.row(), square.column());
//     flip_vertical(compute_diag_bb(square))
// }

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

// + BitAnd<usize>
// + BitOrAssign
// + BitXor<Output = Self>
// + BitXorAssign
// + Shl<usize, Output = Self>
// + ShlAssign<usize>
// + Shr<usize, Output = Self>
// + ShrAssign<usize>
pub struct ChessBitboard(pub u64);

impl Sub for ChessBitboard {
    type Output = ChessBitboard;

    fn sub(self, rhs: Self) -> Self::Output {
        ChessBitboard(self.0.wrapping_sub(rhs.0))
    }
}

impl Debug for ChessBitboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Chess Bitboard {:#x}", self.0)
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
pub struct ExtendedBitboard(pub u128);

impl Debug for ExtendedBitboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Extended Bitboard {:#x}", self.0)
    }
}

impl Sub for ExtendedBitboard {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_sub(rhs.0))
    }
}

pub const A_FILE: ChessBitboard = ChessBitboard(0x0101_0101_0101_0101);
pub const FIRST_RANK: ChessBitboard = ChessBitboard(0xFF);

#[derive(Debug, EnumIter)]
pub enum SliderAttacks {
    Horizontal,
    Vertical,
    Diagonal,
    AntiDiagonal,
}

// This seems like a lot of boilerplate code.
// Maybe there's a better way?
pub trait Bitboard:
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

    fn is_zero(self) -> bool {
        self.to_primitive() == Self::Primitive::zero()
    }

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

    fn rank_0(size: GridSize) -> Self {
        Self::from_primitive((Self::Primitive::one() << size.width().0) - Self::Primitive::one())
    }

    fn file_0(size: GridSize) -> Self {
        Self::from_u128(STEPS[size.width().0])
    }

    fn rank(i: usize, size: GridSize) -> Self {
        debug_assert!(i < size.height().0);
        Self::rank_0(size) << (i * size.width().0)
    }

    fn file(i: usize, size: GridSize) -> Self {
        debug_assert!(i < size.width().0);
        Self::file_0(size) << i
    }

    fn diag_for_sq(sq: usize, size: GridSize) -> Self {
        debug_assert!(sq < size.num_squares());
        Self::from_u128(DIAGONALS[size.width().0][sq])
    }

    fn anti_diag_for_sq(sq: usize, size: GridSize) -> Self {
        debug_assert!(sq < size.num_squares());
        Self::from_u128(ANTI_DIAGONALS[size.width().0][sq])
    }

    fn piece_coordinates(self, size: GridSize) -> GridCoordinates {
        debug_assert!(self.is_single_piece());
        let idx = self.trailing_zeros();
        size.to_coordinates(idx)
    }

    /// TODO: The following two methods are likely very slow. Find something faster
    fn flip_left_right(self, size: GridSize) -> Self {
        let width = size.width().0;
        let mut bb = self;
        let file_mask = Self::file_0(size);
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
    fn flip_up_down(self, size: GridSize) -> Self {
        let mut bb = self;
        let rank_mask = Self::rank_0(size);
        // flip ranks linearly
        for i in 0..size.height().0 / 2 {
            let lower_shift = i * size.width().0;
            let upper_shift = (size.height().0 - 1 - i) * size.width().0;
            let lower_rank = (bb >> lower_shift) & rank_mask;
            let upper_rank = (bb >> upper_shift) & rank_mask;
            let xor = lower_rank ^ upper_rank;
            bb ^= xor << lower_shift;
            bb ^= xor << upper_shift;
        }
        bb
    }

    fn get_piece_file(self, size: GridSize) -> usize {
        debug_assert!(self.is_single_piece());
        self.trailing_zeros() % size.width().0
    }

    fn get_piece_rank(self, size: GridSize) -> usize {
        debug_assert!(self.is_single_piece());
        self.trailing_zeros() / size.width().0
    }

    fn hyperbola_quintessence<F>(self, blockers: Self, reverse: F, ray: Self) -> Self
    where
        F: Fn(Self) -> Self,
    {
        // alternative implementation without reverse:
        // let forward = (blocker & ray) - (self << 1);
        // let forward = blockers - forward;
        // let mask = forward - 1;
        debug_assert!(self.is_single_piece());
        debug_assert!(!(self & ray).is_zero());
        let blockers = blockers & ray;
        let reversed_blockers = reverse(blockers);
        let forward = blockers - self;
        let backward = reversed_blockers - reverse(self);
        let backward = reverse(backward);
        (forward ^ backward) & ray
    }

    fn hyperbola_quintessence_non_horizontal(
        self,
        blockers: Self,
        ray: Self,
        size: GridSize,
    ) -> Self {
        self.hyperbola_quintessence(blockers, |x| x.flip_up_down(size), ray)
    }

    fn horizontal_attacks(self, blockers: Self, size: GridSize) -> Self {
        let rank = Self::rank_0(size) << (size.width.0 * self.get_piece_rank(size));
        self.hyperbola_quintessence(blockers, |x| x.flip_left_right(size), rank)
    }

    fn vertical_attacks(self, blockers: Self, size: GridSize) -> Self {
        let file = Self::file_0(size) << self.get_piece_file(size);
        self.hyperbola_quintessence_non_horizontal(blockers, file, size)
    }

    fn diagonal_attacks(self, blockers: Self, size: GridSize) -> Self {
        self.hyperbola_quintessence_non_horizontal(
            blockers,
            Self::diag_for_sq(self.trailing_zeros(), size),
            size,
        )
    }

    fn anti_diagonal_attacks(self, blockers: Self, size: GridSize) -> Self {
        self.hyperbola_quintessence_non_horizontal(
            blockers,
            Self::anti_diag_for_sq(self.trailing_zeros(), size),
            size,
        )
    }

    fn slider_attacks(self, blockers: Self, size: GridSize, dir: SliderAttacks) -> Self {
        match dir {
            SliderAttacks::Horizontal => self.horizontal_attacks(blockers, size),
            SliderAttacks::Vertical => self.vertical_attacks(blockers, size),
            SliderAttacks::Diagonal => self.diagonal_attacks(blockers, size),
            SliderAttacks::AntiDiagonal => self.anti_diagonal_attacks(blockers, size),
        }
    }

    fn rook_attacks(self, blockers: Self, size: GridSize) -> Self {
        self.vertical_attacks(blockers, size) | self.horizontal_attacks(blockers, size)
    }

    fn bishop_attacks(self, blockers: Self, size: GridSize) -> Self {
        self.diagonal_attacks(blockers, size) | self.anti_diagonal_attacks(blockers, size)
    }

    fn queen_attacks(self, blockers: Self, size: GridSize) -> Self {
        self.rook_attacks(blockers, size) | self.bishop_attacks(blockers, size)
    }

    fn attacks(self, blockers: Self, size: GridSize, direction: Direction) -> Self {
        match direction {
            Direction::Horizontal => self.horizontal_attacks(blockers, size),
            Direction::Vertical => self.vertical_attacks(blockers, size),
            Direction::Diagonal => self.diagonal_attacks(blockers, size),
            Direction::AntiDiagonal => self.anti_diagonal_attacks(blockers, size),
        }
    }
}

impl BitAnd<usize> for ChessBitboard {
    type Output = usize;

    fn bitand(self, rhs: usize) -> usize {
        (self.0 as usize).bitand(rhs)
    }
}

impl Bitboard for ChessBitboard {
    type Primitive = u64;

    fn from_u128(value: u128) -> Self {
        ChessBitboard(value as u64)
    }

    fn from_primitive(val: Self::Primitive) -> Self {
        ChessBitboard(val)
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

    fn rank_0(_: GridSize) -> Self {
        FIRST_RANK
    }

    fn file_0(_: GridSize) -> Self {
        A_FILE
    }

    fn flip_left_right(self, _: GridSize) -> ChessBitboard {
        const FLIP_DUOS: ChessBitboard = ChessBitboard(0x5555_5555_5555_5555);
        const FLIP_NIBBLES: ChessBitboard = ChessBitboard(0x3333_3333_3333_3333);
        const FLIP_BYTES: ChessBitboard = ChessBitboard(0x0f0f_0f0f_0f0f_0f0f);
        let bb = self;
        // SWAR flip
        let bb = ((bb >> 1) & FLIP_DUOS) | ((bb & FLIP_DUOS) << 1);
        let bb = ((bb >> 2) & FLIP_NIBBLES) | ((bb & FLIP_NIBBLES) << 2);

        ((bb >> 4) & FLIP_BYTES) | ((bb & FLIP_BYTES) << 4)
    }

    fn flip_up_down(self, _: GridSize) -> Self {
        Self(self.0.swap_bytes())
    }

    fn get_piece_file(self, _: GridSize) -> usize {
        debug_assert!(self.0.is_power_of_two());
        self.0.trailing_zeros() as usize % 8
    }

    fn get_piece_rank(self, _: GridSize) -> usize {
        debug_assert!(self.0.is_power_of_two());
        self.0.trailing_zeros() as usize / 8
    }
}

pub fn flip_up_down_chessboard(bb: ChessBitboard) -> ChessBitboard {
    bb.flip_up_down(Default::default())
}

pub fn flip_left_right_chessboard(bb: ChessBitboard) -> ChessBitboard {
    bb.flip_left_right(Default::default())
}

pub fn get_file_chessboard(piece: ChessBitboard) -> usize {
    piece.get_piece_file(Default::default())
}

pub fn get_rank_chessboard(piece: ChessBitboard) -> usize {
    piece.get_piece_rank(Default::default())
}

impl BitAnd<usize> for ExtendedBitboard {
    type Output = usize;

    fn bitand(self, rhs: usize) -> Self::Output {
        (self.0 as usize).bitand(rhs)
    }
}

impl Bitboard for ExtendedBitboard {
    type Primitive = u128;

    fn from_u128(val: u128) -> Self {
        ExtendedBitboard(val)
    }

    fn from_primitive(val: Self::Primitive) -> Self {
        ExtendedBitboard(val)
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

    // fn flip_up_down(self, size: RectangularSize) -> Self {
    //     Self::from_primitive(self.to_primitive().reverse_bits()) // TODO: Remove?
    // }
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
    use crate::general::bitboards::{
        remove_ones_above, remove_ones_below, Bitboard, ExtendedBitboard,
    };

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
        for i in 0..64 {
            let row = i / 8;
            let expected = (0xff_u128 - (1 << (i % 8))) << (row * 8);
            assert_eq!(
                ExtendedBitboard(1 << i).hyperbola_quintessence(
                    ExtendedBitboard(0),
                    |x| ExtendedBitboard(x.0.reverse_bits()),
                    ExtendedBitboard(0xff) << (row * 8)
                ),
                ExtendedBitboard(expected),
                "{i}"
            );
        }

        assert_eq!(
            ExtendedBitboard(0b_0000_1000).hyperbola_quintessence(
                ExtendedBitboard(0b_0100_0001),
                |x| ExtendedBitboard(x.0.reverse_bits()),
                ExtendedBitboard(0xff)
            ),
            ExtendedBitboard(0b_0111_0111)
        );

        assert_eq!(
            ExtendedBitboard(0x0000_1000_0000).hyperbola_quintessence(
                ExtendedBitboard(0x1234_4000_0fed),
                |x| ExtendedBitboard(x.0.reverse_bits()),
                ExtendedBitboard(0xffff_ffff_ffff)
            ),
            ExtendedBitboard(0x0000_6fff_f800)
        );

        assert_eq!(
            ExtendedBitboard(0x0000_0000_1000_0000).hyperbola_quintessence(
                ExtendedBitboard(0x0110_0200_0001_1111),
                |x| ExtendedBitboard(x.0.reverse_bits()),
                ExtendedBitboard(0x1111_1111_1111_1111)
            ),
            ExtendedBitboard(0x0011_1111_0111_0000)
        );

        assert_eq!(
            ExtendedBitboard(0x0000_0001_0000).hyperbola_quintessence(
                ExtendedBitboard(0xfffe_d002_a912),
                |x| ExtendedBitboard(x.0.swap_bytes()),
                ExtendedBitboard(0x0101_0101_0101)
            ),
            ExtendedBitboard(0x0101_0100_0100)
        );

        assert_eq!(
            ExtendedBitboard(0x0000_0010_0000).hyperbola_quintessence(
                ExtendedBitboard(0xffff_ffef_ffff),
                |x| ExtendedBitboard(x.0.swap_bytes()),
                ExtendedBitboard(0x_ffff_ffff_ffff)
            ),
            ExtendedBitboard(0)
        );
    }
}
