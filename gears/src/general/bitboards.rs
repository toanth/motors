extern crate num;

use std::fmt::{Debug, Formatter};
use std::num::Wrapping;

use derive_more::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, ShlAssign, Shr,
    ShrAssign, Sub,
};
use num::{One, PrimInt, Unsigned, Zero};
use strum_macros::EnumIter;

use crate::games::{DimT, GridCoordinates, RectangularCoordinates, RectangularSize, Size};
#[cfg(feature = "chess")]
use crate::games::chess::squares::{ChessboardSize, ChessSquare};
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

const fn gen_single_knight_attacks(square_idx: usize) -> u64 {
    let this_knight: u64 = 1 << square_idx;
    let a_file = 0x0101_0101_0101_0101;
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
const fn gen_knights() -> [u64; 64] {
    let mut res: [u64; 64] = [0; 64];
    let mut i = 0;
    while i < 64 {
        res[i] = gen_single_knight_attacks(i);
        i += 1;
    }
    res
}

pub const KNIGHTS: [u64; 64] = gen_knights();

pub const WHITE_SQUARES: ChessBitboard = ChessBitboard(0xaaaa_aaaa_aaaa_aaaa);
pub const BLACK_SQUARES: ChessBitboard = ChessBitboard(0x5555_5555_5555_5555);

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
#[cfg(feature = "chess")]
pub struct ChessBitboard(pub u64);

#[cfg(feature = "chess")]
impl Sub for ChessBitboard {
    type Output = ChessBitboard;

    fn sub(self, rhs: Self) -> Self::Output {
        ChessBitboard(self.0.wrapping_sub(rhs.0))
    }
}

#[cfg(feature = "chess")]
impl Debug for ChessBitboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Chess Bitboard {:#x}", self.0)
    }
}

impl ChessBitboard {
    pub fn file_no(idx: DimT) -> Self {
        Self::file(idx, ChessboardSize::default())
    }
    pub fn rank_no(idx: DimT) -> Self {
        Self::rank(idx, ChessboardSize::default())
    }

    pub fn north(self) -> Self {
        self << 8
    }

    pub fn south(self) -> Self {
        self >> 8
    }

    pub fn east(self) -> Self {
        (self & !Self::file_no(7)) << 1
    }

    pub fn west(self) -> Self {
        (self & !Self::file_no(0)) >> 1
    }

    pub fn north_east(self) -> Self {
        self.north().east()
    }

    pub fn south_east(self) -> Self {
        self.south().east()
    }

    pub fn south_west(self) -> Self {
        self.south().west()
    }

    pub fn north_west(self) -> Self {
        self.north().west()
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

#[cfg(feature = "chess")]
pub const A_FILE: ChessBitboard = ChessBitboard(0x0101_0101_0101_0101);
#[cfg(feature = "chess")]
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
pub trait Bitboard<C: RectangularCoordinates>:
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
where
    C::Size: RectangularSize<C>,
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

    fn rank_0(size: C::Size) -> Self {
        Self::from_primitive(
            (Self::Primitive::one() << size.width().val()) - Self::Primitive::one(),
        )
    }

    fn file_0(size: C::Size) -> Self {
        Self::from_u128(STEPS[size.width().val()])
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
        Self::from_u128(DIAGONALS[size.width().val()][size.to_idx(sq)])
    }

    fn anti_diag_for_sq(sq: C, size: C::Size) -> Self {
        debug_assert!(size.coordinates_valid(sq));
        Self::from_u128(ANTI_DIAGONALS[size.width().val()][size.to_idx(sq)])
    }

    fn piece_coordinates(self, size: C::Size) -> C {
        debug_assert!(self.is_single_piece());
        let idx = self.trailing_zeros();
        size.to_coordinates(idx)
    }

    /// TODO: The following two methods are likely very slow. Find something faster
    fn flip_left_right(self, size: C::Size) -> Self {
        let width = size.width().val();
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
    fn flip_up_down(self, size: C::Size) -> Self {
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

    fn get_piece_file(self, size: C::Size) -> usize {
        debug_assert!(self.is_single_piece());
        self.trailing_zeros() % size.width().val()
    }

    fn get_piece_rank(self, size: C::Size) -> usize {
        debug_assert!(self.is_single_piece());
        self.trailing_zeros() / size.width().val()
    }

    fn hyperbola_quintessence<F>(idx: usize, blockers: Self, reverse: F, ray: Self) -> Self
    where
        F: Fn(Self) -> Self,
    {
        let piece = Self::single_piece(idx);
        debug_assert!(!(piece & ray).is_zero());
        let blockers = blockers & ray;
        let reversed_blockers = reverse(blockers);
        let forward = blockers - piece;
        let backward = reversed_blockers - reverse(piece);
        let backward = reverse(backward);
        (forward ^ backward) & ray
    }

    fn hyperbola_quintessence_non_horizontal(
        square: C,
        blockers: Self,
        ray: Self,
        size: C::Size,
    ) -> Self {
        Self::hyperbola_quintessence(size.to_idx(square), blockers, |x| x.flip_up_down(size), ray)
    }

    fn horizontal_attacks(square: C, blockers: Self, size: C::Size) -> Self {
        let rank = Self::rank(square.row(), size);
        Self::hyperbola_quintessence(
            size.to_idx(square),
            blockers,
            |x| x.flip_left_right(size),
            rank,
        )
    }

    fn vertical_attacks(square: C, blockers: Self, size: C::Size) -> Self {
        let file = Self::file(square.column(), size);
        Self::hyperbola_quintessence_non_horizontal(square, blockers, file, size)
    }

    fn diagonal_attacks(square: C, blockers: Self, size: C::Size) -> Self {
        Self::hyperbola_quintessence_non_horizontal(
            square,
            blockers,
            Self::diag_for_sq(square, size),
            size,
        )
    }

    fn anti_diagonal_attacks(square: C, blockers: Self, size: C::Size) -> Self {
        Self::hyperbola_quintessence_non_horizontal(
            square,
            blockers,
            Self::anti_diag_for_sq(square, size),
            size,
        )
    }

    fn slider_attacks(square: C, blockers: Self, size: C::Size, dir: SliderAttacks) -> Self {
        match dir {
            SliderAttacks::Horizontal => Self::horizontal_attacks(square, blockers, size),
            SliderAttacks::Vertical => Self::vertical_attacks(square, blockers, size),
            SliderAttacks::Diagonal => Self::diagonal_attacks(square, blockers, size),
            SliderAttacks::AntiDiagonal => Self::anti_diagonal_attacks(square, blockers, size),
        }
    }

    fn rook_attacks(square: C, blockers: Self, size: C::Size) -> Self {
        Self::vertical_attacks(square, blockers, size)
            | Self::horizontal_attacks(square, blockers, size)
    }

    fn bishop_attacks(square: C, blockers: Self, size: C::Size) -> Self {
        Self::diagonal_attacks(square, blockers, size)
            | Self::anti_diagonal_attacks(square, blockers, size)
    }

    fn queen_attacks(square: C, blockers: Self, size: C::Size) -> Self {
        Self::rook_attacks(square, blockers, size) | Self::bishop_attacks(square, blockers, size)
    }

    fn attacks(square: C, blockers: Self, size: C::Size, direction: Direction) -> Self {
        match direction {
            Direction::Horizontal => Self::horizontal_attacks(square, blockers, size),
            Direction::Vertical => Self::vertical_attacks(square, blockers, size),
            Direction::Diagonal => Self::diagonal_attacks(square, blockers, size),
            Direction::AntiDiagonal => Self::anti_diagonal_attacks(square, blockers, size),
        }
    }
}

#[cfg(feature = "chess")]
impl BitAnd<usize> for ChessBitboard {
    type Output = usize;

    fn bitand(self, rhs: usize) -> usize {
        (self.0 as usize).bitand(rhs)
    }
}

#[cfg(feature = "chess")]
impl Bitboard<ChessSquare> for ChessBitboard {
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

    fn rank_0(_: ChessboardSize) -> Self {
        FIRST_RANK
    }

    fn file_0(_: ChessboardSize) -> Self {
        A_FILE
    }

    fn flip_left_right(self, _: ChessboardSize) -> ChessBitboard {
        const FLIP_DUOS: ChessBitboard = ChessBitboard(0x5555_5555_5555_5555);
        const FLIP_NIBBLES: ChessBitboard = ChessBitboard(0x3333_3333_3333_3333);
        const FLIP_BYTES: ChessBitboard = ChessBitboard(0x0f0f_0f0f_0f0f_0f0f);
        let bb = self;
        // SWAR flip
        let bb = ((bb >> 1) & FLIP_DUOS) | ((bb & FLIP_DUOS) << 1);
        let bb = ((bb >> 2) & FLIP_NIBBLES) | ((bb & FLIP_NIBBLES) << 2);

        ((bb >> 4) & FLIP_BYTES) | ((bb & FLIP_BYTES) << 4)
    }

    fn flip_up_down(self, _: ChessboardSize) -> Self {
        Self(self.0.swap_bytes())
    }

    fn get_piece_file(self, _: ChessboardSize) -> usize {
        debug_assert!(self.0.is_power_of_two());
        self.0.trailing_zeros() as usize % 8
    }

    fn get_piece_rank(self, _: ChessboardSize) -> usize {
        debug_assert!(self.0.is_power_of_two());
        self.0.trailing_zeros() as usize / 8
    }
}

impl BitAnd<usize> for ExtendedBitboard {
    type Output = usize;

    fn bitand(self, rhs: usize) -> Self::Output {
        (self.0 as usize).bitand(rhs)
    }
}

impl Bitboard<GridCoordinates> for ExtendedBitboard {
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
        Bitboard, ExtendedBitboard, remove_ones_above, remove_ones_below,
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
                ExtendedBitboard::hyperbola_quintessence(
                    i,
                    ExtendedBitboard(0),
                    |x| ExtendedBitboard(x.0.reverse_bits()),
                    ExtendedBitboard(0xff) << (row * 8)
                ),
                ExtendedBitboard(expected),
                "{i}"
            );
        }

        assert_eq!(
            ExtendedBitboard::hyperbola_quintessence(
                3,
                ExtendedBitboard(0b_0100_0001),
                |x| ExtendedBitboard(x.0.reverse_bits()),
                ExtendedBitboard(0xff),
            ),
            ExtendedBitboard(0b_0111_0111)
        );

        assert_eq!(
            ExtendedBitboard::hyperbola_quintessence(
                28,
                ExtendedBitboard(0x1234_4000_0fed),
                |x| ExtendedBitboard(x.0.reverse_bits()),
                ExtendedBitboard(0xffff_ffff_ffff),
            ),
            ExtendedBitboard(0x0000_6fff_f800)
        );

        assert_eq!(
            ExtendedBitboard::hyperbola_quintessence(
                28,
                ExtendedBitboard(0x0110_0200_0001_1111),
                |x| ExtendedBitboard(x.0.reverse_bits()),
                ExtendedBitboard(0x1111_1111_1111_1111),
            ),
            ExtendedBitboard(0x0011_1111_0111_0000)
        );

        assert_eq!(
            ExtendedBitboard::hyperbola_quintessence(
                16,
                ExtendedBitboard(0xfffe_d002_a912),
                |x| ExtendedBitboard(x.0.swap_bytes()),
                ExtendedBitboard(0x0101_0101_0101),
            ),
            ExtendedBitboard(0x0101_0100_0100)
        );

        assert_eq!(
            ExtendedBitboard::hyperbola_quintessence(
                20,
                ExtendedBitboard(0xffff_ffef_ffff),
                |x| ExtendedBitboard(x.0.swap_bytes()),
                ExtendedBitboard(0x_ffff_ffff_ffff),
            ),
            ExtendedBitboard(0)
        );
    }
}
