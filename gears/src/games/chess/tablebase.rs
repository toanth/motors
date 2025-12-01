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
use crate::PlayerResult::Draw;
use crate::games::chess::Color::{Black, White};
use crate::games::chess::bitbase::{PAWN_V_KING_TABLE, query_pawn_v_king};
use crate::games::chess::pieces::ColoredPieceType::{BlackKing, BlackPawn, WhiteKing, WhitePawn};
use crate::games::chess::pieces::PieceType::{Bishop, Empty, King, Knight, Pawn, Queen, Rook};
use crate::games::chess::pieces::{ColoredPieceType, NUM_CHESS_PIECES, PieceType};
use crate::games::chess::squares::{
    A_FILE_NUM, B_FILE_NUM, C_FILE_NUM, ChessboardSize, D_FILE_NUM, NUM_SQUARES, Square,
};
use crate::games::chess::unverified::UnverifiedBoard;
use crate::games::chess::{Board, ChessBitboardTrait, Color, EDGE_SQUARES, PAWN_CAPTURES};
use crate::games::{ColorTrait, ColoredPieceTypeTrait, CoordinatesTrait, DimT, NUM_COLORS};
use crate::general::bitboards::chessboard::{Bitboard, KINGS, KNIGHTS};
use crate::general::bitboards::{BitboardTrait, KnownSizeBitboard, RawBitboardTrait};
use crate::general::board::Strictness::Strict;
use crate::general::board::{BitboardBoard, BoardTrait, SelfChecks, Strictness, UnverifiedBoardTrait};
use crate::general::hq::ChessSliderGenerator;
use crate::general::squares::RectangularCoordinates;
use itertools::Itertools;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use std::cmp::{Ordering, max};
use std::collections::HashMap;
use std::io;
use std::io::Write;
use std::mem::swap;
use std::sync::LazyLock;
use std::sync::atomic::Ordering::{AcqRel, Relaxed};
use std::sync::atomic::{AtomicI8, AtomicUsize, fence};
use std::time::Instant;

type Entry = AtomicI8;

// Generate up to 6 man tablebases; assumes that usize is at least 64 bits large
pub const MAX_TB_MAN: usize = 6;
pub const MAX_NON_K_PIECES: usize = MAX_TB_MAN - 2;

const INVALID_KING_IDX: u16 = u16::MAX;

mod no_pawns {
    use super::*;

    const NUM_KING_SYMMETRY_SQUARES: usize = 10;

    // Using the upper instead of lower triangle in the lower left corner means we need almost no
    // horizontal flips and instead vertical flips, which are cheaper for bitboards.
    static KING_SQUARES_SYMMETRY: [Square; NUM_KING_SYMMETRY_SQUARES] = [
        Square::from_bb_idx(A_FILE_NUM as usize),
        Square::from_bb_idx(A_FILE_NUM as usize + 8),
        Square::from_bb_idx(B_FILE_NUM as usize + 8),
        Square::from_bb_idx(A_FILE_NUM as usize + 16),
        Square::from_bb_idx(B_FILE_NUM as usize + 16),
        Square::from_bb_idx(C_FILE_NUM as usize + 16),
        Square::from_bb_idx(A_FILE_NUM as usize + 24),
        Square::from_bb_idx(B_FILE_NUM as usize + 24),
        Square::from_bb_idx(C_FILE_NUM as usize + 24),
        Square::from_bb_idx(D_FILE_NUM as usize + 24),
    ];

    static KING_INDICES_SYMMETRY: [u8; 64] = {
        let mut res: [u8; 64] = [NUM_KING_SYMMETRY_SQUARES as u8; 64];
        let mut i = 0;
        while i < 64 {
            let mut j = 0;
            while j < NUM_KING_SYMMETRY_SQUARES {
                if KING_SQUARES_SYMMETRY[j].bb_idx() == i {
                    res[i] = j as u8;
                }
                j = j + 1;
            }
            i = i + 1;
        }
        res
    };

    pub const NUM_KING_SQUARES: usize = 462;

    pub static KING_SQUARES: LazyLock<[[Square; NUM_COLORS]; NUM_KING_SQUARES]> = LazyLock::new(|| {
        let mut res = [[Square::from_bb_idx(0), Square::from_bb_idx(0)]; NUM_KING_SQUARES];
        let mut i = 0;
        let mut w_king_table_idx: usize = 0;
        while w_king_table_idx < NUM_KING_SYMMETRY_SQUARES {
            let w_king = KING_SQUARES_SYMMETRY[w_king_table_idx];
            let w_king_idx = w_king.bb_idx();
            let forbidden = KINGS[w_king_idx].0 | (1 << w_king_idx);
            let mut b_king_idx: usize = 0;
            while b_king_idx < 64 {
                let b_king = Square::from_bb_idx(b_king_idx);
                if (forbidden & (1 << b_king_idx)) != 0
                    || (w_king_idx / 8 == w_king_idx % 8 && b_king_idx / 8 < b_king_idx % 8)
                {
                    b_king_idx += 1;
                    continue;
                }
                res[i] = [w_king, b_king];
                i += 1;
                b_king_idx += 1;
            }
            w_king_table_idx += 1;
        }
        assert!(i == NUM_KING_SQUARES);
        res
    });

    pub static KING_INDICES: LazyLock<[[u16; 64]; NUM_KING_SYMMETRY_SQUARES + 1]> = LazyLock::new(|| {
        let mut res = [[INVALID_KING_IDX; 64]; NUM_KING_SYMMETRY_SQUARES + 1];
        let mut i = 0;
        while i < NUM_KING_SQUARES {
            let [w, b] = KING_SQUARES[i];
            let w = KING_INDICES_SYMMETRY[w.bb_idx()] as usize;
            res[w][b.bb_idx()] = i as u16;
            i = i + 1;
        }
        res
    });

    static CHECKING_SQUARES: LazyLock<[[Bitboard; NUM_KING_SQUARES]; 6]> = LazyLock::new(|| {
        let mut res = [[Bitboard::default(); NUM_KING_SQUARES]; 6];
        for piece_idx in 0..6 {
            let piece = PieceType::from_repr(piece_idx % 5).unwrap();
            let color = Color::from_repr(piece_idx / 5).unwrap();
            let table = &mut res[piece as usize];
            for (i, entry) in table.iter_mut().enumerate() {
                let kings = KING_SQUARES[i];
                *entry = attacks_for(piece, kings[0], kings[1].bb(), color) & !kings[1].bb();
                if piece != Knight {
                    *entry &= KINGS[kings[0]];
                }
            }
        }
        res
    });

    // For the compact index, the restricted king is the nstm.
    // Otherwise, it's White
    pub fn kings_idx(kings: [Square; NUM_COLORS]) -> usize {
        let restricted = KING_INDICES_SYMMETRY[kings[0]] as usize;
        KING_INDICES[restricted][kings[1]] as usize
    }

    pub fn checking(kings: [Square; NUM_COLORS], piece: PieceType, active: Color) -> Bitboard {
        let i = kings_idx(kings);
        if piece == Pawn && active == Black { CHECKING_SQUARES[5][i] } else { CHECKING_SQUARES[piece][i] }
    }
}

mod pawns {
    use super::*;
    use crate::games::chess::PAWN_CAPTURES;

    pub const NUM_KING_SQUARES: usize = 3612;

    pub static KING_SQUARES: LazyLock<[[Square; NUM_COLORS]; NUM_KING_SQUARES]> = LazyLock::new(|| {
        let mut res = [[Square::default(), Square::default()]; NUM_KING_SQUARES];
        let mut i = 0;
        for w_king_idx in 0..64 {
            // the second half of the array is unnecessary for compact indices
            let w_king = Square::from_rank_file((w_king_idx % 32) / 4, (w_king_idx / 32) * 4 + w_king_idx % 4);
            for b_king in Square::iter() {
                if (KINGS[b_king] | b_king.bb()).has(w_king) {
                    continue;
                }
                res[i] = [w_king, b_king];
                i += 1;
            }
        }
        assert!(i == NUM_KING_SQUARES);
        res
    });

    pub static KING_INDICES: LazyLock<[[u16; 64]; 64]> = LazyLock::new(|| {
        let mut res = [[INVALID_KING_IDX; 64]; 64];
        for i in 0..NUM_KING_SQUARES {
            let [w, b] = KING_SQUARES[i];
            res[w.bb_idx()][b.bb_idx()] = i as u16;
        }
        res
    });

    static CHECKING_SQUARES: LazyLock<[[Bitboard; NUM_KING_SQUARES]; 6]> = LazyLock::new(|| {
        let mut res = [[Bitboard::default(); NUM_KING_SQUARES]; 6];
        for piece_idx in 0..6 {
            let piece = PieceType::from_repr(piece_idx % 5).unwrap();
            let color = Color::from_repr(piece_idx / 5).unwrap();
            let table = &mut res[piece as usize];
            for (i, entry) in table.iter_mut().enumerate() {
                let kings = KING_SQUARES[i];
                *entry = if piece == Pawn {
                    PAWN_CAPTURES[!color][kings[0]]
                } else {
                    attacks_for(piece, kings[0], kings[1].bb(), color) & !kings[1].bb()
                };
                if piece != Knight {
                    *entry &= KINGS[kings[0]];
                }
            }
        }
        res
    });

    pub fn kings_idx(kings: [Square; NUM_COLORS]) -> usize {
        KING_INDICES[kings[0].bb_idx()][kings[1].bb_idx()] as usize
    }

    pub fn checking(kings: [Square; NUM_COLORS], piece: PieceType, active: Color) -> Bitboard {
        let i = kings_idx(kings);
        if piece == Pawn && active == Black { CHECKING_SQUARES[5][i] } else { CHECKING_SQUARES[piece][i] }
    }
}

// use the maximum value so that "negamax" never chooses it if there's a legal position
const INVALID: i8 = 127;
// resetting the halfmove clock on a move that would otherwise give a value of 100 is fine, so there are 100 valid non-draw
// values for each side (1 to 100 and -1 to -100). MATED can only appear in positions that are an actual checkmate.
const MATED: i8 = -101;
const DRAW: i8 = 0;

fn attacks_for(wx_piece: PieceType, w_x: Square, blockers: Bitboard, us: Color) -> Bitboard {
    match wx_piece {
        Pawn => {
            let bb = w_x.bb();
            let push = bb.pawn_advance(us) & !blockers;
            let d_push = (push & Bitboard::pawn_ranks().pawn_advance(us)).pawn_advance(us) & !blockers;
            push | d_push | (PAWN_CAPTURES[us][w_x] & blockers)
        }
        Knight => KNIGHTS[w_x],
        Bishop => ChessSliderGenerator::new(blockers).bishop_attacks(w_x),
        Rook => ChessSliderGenerator::new(blockers).rook_attacks(w_x),
        Queen => ChessSliderGenerator::new(blockers).queen_attacks(w_x),
        _ => unreachable!(),
    }
}

fn colors_ordered(counts: PieceCounts) -> bool {
    let mut res = 0;
    for c in Color::iter() {
        for (i, &cnt) in counts[c].iter().enumerate() {
            res += (cnt as isize) << (3 * i);
        }
        res = -res;
    }
    res >= 0
}

// the values of `n choose k`, giving the number of ways `k` indistinguishable pieces can be placed on `n` squares.
static COMBINATIONS: [[usize; MAX_NON_K_PIECES + 1]; NUM_SQUARES + 1] = {
    let mut res = [[0; MAX_NON_K_PIECES + 1]; NUM_SQUARES + 1];
    let mut n = 1;
    res[0][0] = 1;
    while n <= NUM_SQUARES {
        let mut k = 1;
        res[n][0] = 1;
        if n <= MAX_NON_K_PIECES {
            res[n][n] = 1;
        }
        while k < n && k <= MAX_NON_K_PIECES {
            res[n][k] = res[n - 1][k - 1] + res[n - 1][k];
            k += 1;
        }
        n += 1;
    }
    res
};

fn decode(mut n: usize, mut k: usize) -> Bitboard {
    debug_assert!(k >= 1);
    debug_assert!(n < COMBINATIONS[64][k]);
    if k == 1 {
        return Bitboard::from_raw(1 << n);
    }
    let mut res = Bitboard::default();
    while k > 2 {
        let idx = ((k - 1)..65).find(|&i| COMBINATIONS[i][k] > n).unwrap() - 1;
        n -= COMBINATIONS[idx][k];
        k -= 1;
        res |= Bitboard::from_raw(1 << idx);
    }
    // the same computation as the loop above, but faster.
    // see <https://stackoverflow.com/questions/27086195/linear-index-upper-triangular-matrix>;
    // we extend triangular roots (<https://en.wikipedia.org/wiki/Triangular_number#Triangular_roots_and_tests_for_triangular_numbers>)
    // to get the following pattern:
    //  0  a0  a1  a3  a6
    //  0   0  a2  a4  a7
    //  0   0   0  a5  a8
    //  0   0   0   0  a9
    //  0   0   0   0   0
    debug_assert!(n < 1 << 11);
    let i = ((8 * n as u16 + 1).isqrt() + 1) as usize / 2;
    let j = n - i * (i - 1) / 2;
    debug_assert!(j < i);
    res |= Bitboard::from_raw(1 << i);
    res |= Bitboard::from_raw(1 << j);
    res
}

#[inline]
fn encode(bb: Bitboard, num_pieces: usize) -> usize {
    debug_assert_eq!(num_pieces, bb.num_ones());
    let mut r = 0;
    for (i, sq) in bb.into_iter().enumerate() {
        r += COMBINATIONS[sq.bb_idx()][i + 1];
    }
    debug_assert!(r < COMBINATIONS[64][num_pieces]);
    r
}

/// The next value in the same lexicographical order as `decode`.
#[inline]
fn next_combination(n: u64) -> Bitboard {
    // See `https://graphics.stanford.edu/~seander/bithacks.html#NextBitPermutation`
    let t = n | (n - 1);
    Bitboard::new((t + 1) | (((!t & (!t).wrapping_neg()) - 1) >> (n.trailing_zeros() + 1)))
    // TODO: Benchmark against the following (potentially using precomputed `fastdiv`):
    // let c = n & n.wrapping_neg();
    // let r = n + c;
    // ChessBitboard::new((((r ^ n) >> 2) / c) | r)
}
// TODO: Move to its own crate, probably called `rulers` (rename / rework existing rulers crate)

// todo: Profile and use <https://docs.rs/fastdiv/latest/fastdiv/>

// TODO: Only store pawn positions with the king on the left half, requires treatng pawns specially

// This can represent positions where multiple pieces are on the same square.
// This is used to represent captures within the same table during table construction.
// A representation of piece bbs + color bbs wouldn't be able to represent that.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
struct PosIdx<const PAWNS: bool> {
    king_idx: u32,
    bbs: [Bitboard; MAX_TB_MAN],
    active: Color,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct TableData<const PAWNS: bool> {
    piece_types: [ColoredPieceType; MAX_TB_MAN],
    num_pieces: [usize; MAX_TB_MAN],
    pieces_per_player: [usize; NUM_COLORS],
    num_bbs: usize,
    non_pawn_start: usize,
    piece_counts: PieceCounts,
    multipliers: [isize; MAX_TB_MAN - 2],
    kings_multiplier: isize,
}

impl<const PAWNS: bool> TableData<PAWNS> {
    fn new(piece_counts: PieceCounts) -> Self {
        assert!(colors_ordered(piece_counts));
        let mut piece_types = [ColoredPieceType::default(); MAX_TB_MAN];
        let mut num_pieces = [0; MAX_TB_MAN];
        let mut pieces_per_player = [0; NUM_COLORS];
        let mut i = 0;
        for p in PieceType::non_king_pieces() {
            for c in Color::iter() {
                let count = piece_counts[c][p as usize];
                if count == 0 {
                    continue;
                }
                piece_types[i] = ColoredPieceType::new(c, p);
                num_pieces[i] = count as usize;
                pieces_per_player[c] += count as usize;
                i += 1;
            }
        }
        let non_pawn_start = usize::from(piece_counts[White][0] != 0) + usize::from(piece_counts[Black][0] != 0);
        let mut current_multiplier = 1;
        let mut multipliers = [0; MAX_TB_MAN - 2];
        for i in (non_pawn_start..i).rev() {
            multipliers[i] = current_multiplier;
            current_multiplier *= COMBINATIONS[64][num_pieces[i]] as isize;
        }
        let kings_multiplier = current_multiplier;
        current_multiplier *= (NUM_COLORS * Self::num_king_squares()) as isize;
        for i in (0..non_pawn_start).rev() {
            multipliers[i] = current_multiplier;
            current_multiplier *= COMBINATIONS[Self::num_pawn_squares()][num_pieces[i]] as isize;
        }
        piece_types[i] = WhiteKing;
        num_pieces[i] = 1;
        pieces_per_player[White] += 1;
        piece_types[i + 1] = BlackKing;
        num_pieces[i + 1] = 1;
        pieces_per_player[Black] += 1;
        let num_bbs = i + 2;
        let res = Self {
            piece_types,
            num_pieces,
            pieces_per_player,
            num_bbs,
            non_pawn_start,
            piece_counts,
            multipliers,
            kings_multiplier,
        };
        debug_assert_eq!(current_multiplier as usize, res.size());
        debug_assert_eq!(res.inner_size() / NUM_COLORS, res.kings_multiplier as usize * Self::num_king_squares());
        res
    }

    const fn has_pawn() -> bool {
        PAWNS
    }

    fn w_pawns(&self) -> usize {
        if Self::has_pawn() { self.piece_counts[White][0] as usize } else { 0 }
    }

    fn b_pawns(&self) -> usize {
        if Self::has_pawn() { self.piece_counts[Black][0] as usize } else { 0 }
    }

    fn num_pawn_bbs(&self) -> usize {
        if Self::has_pawn() { self.non_pawn_start } else { 0 }
    }

    fn num_all_pieces(&self) -> usize {
        self.pieces_per_player[White] + self.pieces_per_player[Black]
    }

    const fn num_king_squares() -> usize {
        match Self::has_pawn() {
            false => no_pawns::NUM_KING_SQUARES,
            true => pawns::NUM_KING_SQUARES,
        }
    }

    fn kings_index(kings: [Square; 2]) -> usize {
        match Self::has_pawn() {
            false => no_pawns::kings_idx(kings),
            true => pawns::kings_idx(kings),
        }
    }

    fn color_change_delta(&self, current_color: Color) -> isize {
        let res = self.kings_multiplier * Self::num_king_squares() as isize;
        debug_assert_eq!(res as usize, self.inner_size() / NUM_COLORS);
        match current_color {
            White => res,
            Black => -res,
        }
    }

    const fn num_pawn_squares() -> usize {
        match Self::has_pawn() {
            false => 0,
            // we don't remove promoted pawns to make the fixed point iteration simpler
            true => 64 - 8,
        }
    }

    fn size(&self) -> usize {
        self.inner_size() * self.outer_size()
    }

    fn outer_size(&self) -> usize {
        COMBINATIONS[Self::num_pawn_squares()][self.piece_counts[White][Pawn as usize] as usize]
            * COMBINATIONS[Self::num_pawn_squares()][self.piece_counts[Black][Pawn as usize] as usize]
    }

    fn inner_size(&self) -> usize {
        let mut res = 1;
        for &count in &self.num_pieces[self.num_pawn_bbs()..self.num_bbs - 2] {
            let k = COMBINATIONS[64][count];
            res *= k;
        }
        res * Self::num_king_squares() * NUM_COLORS
    }

    fn matches(&self, pieces: &[Bitboard; NUM_CHESS_PIECES], colors: [Bitboard; NUM_COLORS]) -> bool {
        for i in 0..self.num_bbs {
            let t = self.piece_types[i];
            if self.num_pieces[i] != (pieces[t.uncolor()] & colors[t.color().unwrap()]).num_ones() {
                return false;
            }
        }
        true
    }
}

impl<const PAWNS: bool> PosIdx<PAWNS> {
    fn from_chessboard(pos: &Board, t: &TableData<PAWNS>) -> Self {
        Self::from_bitboards(pos.bbs.pieces, pos.bbs.colors, pos.active, t, false, false)
    }

    /// `same_material` only matters when `compact` is true
    fn from_bitboards(
        mut pieces: [Bitboard; NUM_CHESS_PIECES],
        mut colors: [Bitboard; NUM_COLORS],
        mut active: Color,
        t: &TableData<PAWNS>,
        compact: bool,
        same_material: bool,
    ) -> Self {
        debug_assert_ne!(PAWNS, pieces[Pawn].is_zero());
        debug_assert_eq!(colors[0] | colors[1], pieces.iter().fold(Bitboard::new(0), |a, &b| a | b));
        debug_assert!(!colors[0].intersects(colors[1]));
        if (compact && same_material && active == Black) || !t.matches(&pieces, colors) {
            active = !active;
            colors.swap(0, 1);
            if PAWNS {
                for bb in &mut pieces {
                    *bb = bb.flip_up_down();
                }
                colors[0] = colors[0].flip_up_down();
                colors[1] = colors[1].flip_up_down();
            }
        }
        debug_assert!(t.matches(&pieces, colors));

        let mut bbs = [Bitboard::default(); MAX_TB_MAN];
        let mut piece_types = [ColoredPieceType::Empty; MAX_TB_MAN];
        let mut num_pieces = [0; MAX_TB_MAN];
        let mut i = 0;
        for p in PieceType::pieces() {
            for c in Color::iter() {
                let bb = pieces[p] & colors[c];
                if bb.is_zero() {
                    continue;
                }
                bbs[i] = bb;
                piece_types[i] = ColoredPieceType::new(c, p);
                num_pieces[i] = bb.num_ones() as u8;
                i += 1;
            }
        }
        let w_king = bbs[i - 2].to_square().unwrap();
        let b_king = bbs[i - 1].to_square().unwrap();
        let mut res = Self { king_idx: 0, bbs, active };
        res.normalize(t, [w_king, b_king], compact);
        res
    }

    fn pawns(&self, t: &TableData<PAWNS>) -> Bitboard {
        if PAWNS {
            debug_assert!([1, 2].contains(&t.non_pawn_start));
            if t.non_pawn_start == 1 { self.bbs[0] } else { self.bbs[0] | self.bbs[1] }
        } else {
            Bitboard::default()
        }
    }

    fn king_idx(&self) -> usize {
        self.king_idx as usize
    }

    fn kings(&self) -> [Square; NUM_COLORS] {
        match PAWNS {
            false => no_pawns::KING_SQUARES[self.king_idx()],
            true => pawns::KING_SQUARES[self.king_idx()],
        }
    }

    fn player_bbs(&self, t: &TableData<PAWNS>) -> [Bitboard; NUM_COLORS] {
        let mut res = [Bitboard::default(); NUM_COLORS];
        for i in 0..t.num_bbs {
            let c = t.piece_types[i].color().unwrap();
            res[c] |= self.bbs[i];
        }
        res
    }

    fn idx(&self, t: &TableData<PAWNS>) -> usize {
        debug_assert!(self.king_idx() < TableData::<PAWNS>::num_king_squares());
        let mut res = 0;
        // pawns are encoded separately because a) they have to be the outermost loop and b) white pawns must
        // be encoded in reverse order so that pawn pushes lead to positions that have already been computed
        // TODO: Test if using t.multipliers is faster than repeatedly multiplying res; should be easier to autovec
        if PAWNS {
            // we don't need to shift pawn bitboards because no bit with index >= 56 is set on the black or flipped white pawn bitboard
            debug_assert!(t.non_pawn_start > 0);
            let mut i = 0;
            if t.piece_types[0] == WhitePawn {
                res = encode(self.bbs[0].flip_up_down(), t.num_pieces[0]);
                i += 1;
            }
            if t.piece_types[i] == BlackPawn {
                res *= COMBINATIONS[TableData::<PAWNS>::num_pawn_squares()][t.num_pieces[i]];
                res += encode(self.bbs[i], t.num_pieces[i]);
            }
        }

        // the active player has to be the outermost part of the inner loop so that we don't try to write an entry
        // before we've seen all the positions that can reach this entry
        res *= NUM_COLORS;
        res += self.active as usize;
        res *= TableData::<PAWNS>::num_king_squares();
        res += self.king_idx();

        for (i, &bb) in self.bbs[t.num_pawn_bbs()..t.num_bbs - 2].iter().enumerate() {
            debug_assert!(bb.has_any());
            let count = t.num_pieces[i + t.num_pawn_bbs()];
            res *= COMBINATIONS[64][count];
            res += encode(bb, count);
        }
        res
    }

    /// For iterating, `next()` should be preferred
    fn from_idx(mut idx: usize, t: &TableData<PAWNS>) -> Self {
        let original_idx = idx;
        debug_assert_ne!(t.piece_counts[White][0] == 0 && t.piece_counts[Black][0] == 0, PAWNS);
        // hopefully, this allows the compiler to optimize better

        let mut res = Self::default();
        // bijection between an index and two squares with sq.0 < sq.1, used for two pieces of the same colored piece type
        // see <https://en.wikipedia.org/wiki/Combinatorial_number_system>
        for i in (t.num_pawn_bbs()..t.num_bbs - 2).rev() {
            let count = t.num_pieces[i];
            debug_assert_ne!(count, 0);
            let max = COMBINATIONS[64][count];
            res.bbs[i] = decode(idx % max, count);
            debug_assert_eq!(count, res.bbs[i].num_ones());
            idx /= max;
        }
        res.king_idx = (idx % TableData::<PAWNS>::num_king_squares()) as u32;
        idx /= TableData::<PAWNS>::num_king_squares();
        let king_bbs = res.kings().map(|sq| sq.bb());
        res.bbs[t.num_bbs - 2] = king_bbs[0];
        debug_assert_eq!(t.num_pieces[t.num_bbs - 2], 1);
        debug_assert_eq!(t.piece_types[t.num_bbs - 2], WhiteKing);
        res.bbs[t.num_bbs - 1] = king_bbs[1];
        debug_assert_eq!(t.num_pieces[t.num_bbs - 1], 1);
        debug_assert_eq!(t.piece_types[t.num_bbs - 1], BlackKing);
        res.active = Color::from_repr(idx % 2).unwrap();
        idx /= 2;
        // pawns give the index in the outer iteration
        for c in Color::iter().rev() {
            match if c == White { t.w_pawns() } else { t.b_pawns() } {
                0 => {}
                count => {
                    let max = COMBINATIONS[TableData::<PAWNS>::num_pawn_squares()][count];
                    let bb = decode(idx % max, count);
                    if c == White {
                        res.bbs[0] = bb.flip_up_down();
                    } else {
                        res.bbs[t.num_pawn_bbs() - 1] = bb;
                    }
                    idx /= max;
                }
            }
        }
        debug_assert_eq!(idx, 0, "{original_idx}");
        debug_assert!(original_idx == res.idx(t), "{res:?}");
        res
    }

    fn normalize(&mut self, t: &TableData<PAWNS>, kings: [Square; NUM_COLORS], compact: bool) {
        // during construction, the white king is the primary king. In the compacted table, the active king is the primary king.
        let [mut primary_king, mut secondary_king] = kings;
        self.bbs[t.num_bbs - 1] = kings[1].bb();
        self.bbs[t.num_bbs - 2] = kings[0].bb();
        let flip = compact && self.active == White;
        if flip {
            // the inactive king is the primary king
            swap(&mut primary_king, &mut secondary_king);
        }
        if !PAWNS || compact {
            if primary_king.file() >= 4 {
                primary_king = primary_king.flip_left_right(ChessboardSize::default());
                secondary_king = secondary_king.flip_left_right(ChessboardSize::default());
                for bb in &mut self.bbs[0..t.num_bbs] {
                    *bb = bb.flip_left_right();
                }
            }
        }
        if !PAWNS && primary_king.rank() >= 4 {
            primary_king = primary_king.flip();
            secondary_king = secondary_king.flip();
            for bb in &mut self.bbs[0..t.num_bbs] {
                *bb = bb.flip_up_down();
            }
        }
        if !PAWNS && (primary_king.rank(), secondary_king.rank()) < (primary_king.file(), secondary_king.file()) {
            primary_king = primary_king.flip_diagonally();
            secondary_king = secondary_king.flip_diagonally();
            for bb in &mut self.bbs[0..t.num_bbs] {
                *bb = bb.flip_diagonally();
            }
        }

        self.king_idx = (TableData::<PAWNS>::kings_index([primary_king, secondary_king])) as u32;
        if flip {
            debug_assert_eq!([self.bbs[t.num_bbs - 1], self.bbs[t.num_bbs - 2]], self.kings().map(|sq| sq.bb()))
        } else {
            debug_assert_eq!([self.bbs[t.num_bbs - 2], self.bbs[t.num_bbs - 1]], self.kings().map(|sq| sq.bb()));
        }
        debug_assert_eq!(t.piece_types[t.num_bbs - 2..t.num_bbs], [WhiteKing, BlackKing]);
    }

    fn outer_iter(
        t: &TableData<PAWNS>,
    ) -> impl Iterator<
        Item = (
            impl ParallelIterator<Item = PosIdxIter<'_, PAWNS>> + Clone,
            impl ParallelIterator<Item = PosIdxIter<'_, PAWNS>> + Clone,
        ),
    > {
        let max = t.size();
        let step = t.inner_size();
        let inner_step = step / NUM_COLORS;
        (0..max).step_by(step).map(move |i| (Self::inner_iter(i, t), Self::inner_iter(i + inner_step, t)))
    }

    fn inner_iter(n: usize, t: &TableData<PAWNS>) -> impl ParallelIterator<Item = PosIdxIter<'_, PAWNS>> + Clone {
        let step = t.inner_size() / NUM_COLORS;
        let num_chunks = rayon::current_num_threads() * 128;
        let chunk_size = step.div_ceil(num_chunks);
        (0..num_chunks)
            .into_par_iter()
            .map(move |i| Self::chunk_iter(t, n + i * chunk_size, n + ((i + 1) * chunk_size).min(step)))
    }

    fn chunk_iter(t: &TableData<PAWNS>, lower: usize, upper: usize) -> PosIdxIter<'_, PAWNS> {
        PosIdxIter { pos_idx: Self::from_idx(lower.min(upper - 1), t), t, first: true, current: lower, end: upper }
    }
}

/// Not an actual iterator because that would force retuning `PosIdx` by value, but it's cheaper to return a reference.
struct PosIdxIter<'a, const PAWNS: bool> {
    pos_idx: PosIdx<PAWNS>,
    t: &'a TableData<PAWNS>,
    first: bool,
    current: usize,
    end: usize,
}

impl<const PAWNS: bool> PosIdxIter<'_, PAWNS> {
    fn next(&mut self) -> Option<(usize, &PosIdx<PAWNS>)> {
        if self.current + usize::from(!self.first) >= self.end {
            return None;
        } else if self.first {
            self.first = false;
            return Some((self.current, &self.pos_idx));
        }
        self.current += 1;
        let p = &mut self.pos_idx;
        let non_pawns = self.t.num_pawn_bbs();
        for bb in p.bbs[non_pawns..self.t.num_bbs - 2].iter_mut().rev() {
            let n = bb.raw();
            if n | (n - 1) != u64::MAX {
                *bb = next_combination(n);
                return Some((self.current, &self.pos_idx));
            }
            *bb = Bitboard::new((1 << bb.num_ones()) - 1);
        }
        p.king_idx += 1;
        if p.king_idx() < TableData::<PAWNS>::num_king_squares() {
            let kings = p.kings();
            p.bbs[self.t.num_bbs - 2] = kings[0].bb();
            p.bbs[self.t.num_bbs - 1] = kings[1].bb();
        }

        Some((self.current, &self.pos_idx))
    }
}

fn set_base_case(pos: UnverifiedBoard, captured_or_promo: Bitboard) -> i8 {
    let Ok(pos) = pos.verify_with_level(SelfChecks::Tablebase, Strictness::Relaxed) else {
        return INVALID;
    };
    if captured_or_promo.has_any() {
        if pos.occupied_bb().num_ones() == 2 {
            return DRAW; // only kings left, insufficient material
        }
        let dtz = probe_dtz(pos);
        return match dtz.cmp(&DRAW) {
            Ordering::Less => MATED,
            Ordering::Equal => DRAW,
            Ordering::Greater => -MATED,
        };
    }
    if pos.is_checkmate_slow() {
        return MATED;
    }
    // no need to handle stalemate as the default is already DRAW
    DRAW
}

fn base_case_iter<const PAWN: bool>(p: &PosIdx<PAWN>, t: &TableData<PAWN>, active: Color) -> i8 {
    assert_eq!(active, p.active, "{0} {t:?} {p:?}", p.idx(t));
    let mut piece_counts = t.piece_counts;
    let bbs = p.player_bbs(t);
    let captured: Bitboard = bbs[active] & bbs[!active];
    let kings = p.kings();
    if bbs[White].num_ones() != t.pieces_per_player[White]
        || bbs[Black].num_ones() != t.pieces_per_player[Black]
        || captured.more_than_one_bit_set()
        || bbs[!active].has(kings[active])
        || (KINGS[kings[White]] | kings[White].bb()).has(kings[Black])
    {
        return INVALID;
    }
    let mut pos = Board::empty();
    pos.set_active_player(active);
    for i in 0..t.num_bbs - 2 {
        let pt = t.piece_types[i];
        for sq in p.bbs[i] & !captured {
            pos.place_piece(sq, pt);
            piece_counts[pt.color().unwrap()][pt.uncolor() as usize] -= 1;
        }
    }
    pos.place_piece(kings[0], WhiteKing);
    pos.place_piece(kings[1], BlackKing);
    if let Some(captured_sq) = captured.to_square() {
        if let Some(piece) =
            piece_counts[!active].iter().find_position(|&&cnt| cnt > 0).map(|x| PieceType::from_repr(x.0).unwrap())
        {
            pos.place_piece(captured_sq, ColoredPieceType::new(!active, piece));
        }
    }
    let promoted = pos.0.col_piece_bb(!active, Pawn) & Bitboard::rank(7 * (active as DimT));
    if promoted.more_than_one_bit_set() {
        return INVALID;
    }
    if let Some(sq) = promoted.to_square() {
        if (captured & !promoted).has_any() {
            return INVALID;
        }
        let mut r = INVALID;
        for promo in [Queen, Knight, Rook, Bishop] {
            pos.remove_piece(sq);
            pos.place_piece(sq, ColoredPieceType::new(!active, promo));
            r = r.min(set_base_case(pos, promoted));
            if r == MATED {
                break;
            }
        }
        return r;
    }
    set_base_case(pos, captured)
}

// Base Case: Fill out all positions that are checkmated, a stalemate or where a piece got captured.
// The captured piece can only belong to the active player, as it must have gotten captured in the previous move.
fn base_case<const PAWNS: bool>(t: &TableData<PAWNS>, table: &[Entry]) {
    for (w_iter, b_iter) in PosIdx::<PAWNS>::outer_iter(t) {
        w_iter.for_each(|mut iter| {
            while let Some((i, p)) = iter.next() {
                table[i].store(base_case_iter(p, t, White), Relaxed)
            }
        });
        b_iter.for_each(|mut iter| {
            while let Some((i, p)) = iter.next() {
                table[i].store(base_case_iter(p, t, Black), Relaxed)
            }
        });
    }
}

fn value_after<const PAWNS: bool>(
    p: &PosIdx<PAWNS>,
    t: &TableData<PAWNS>,
    idx: usize,
    i: usize,
    src: Square,
    dest: Square,
    table: &[Entry],
) -> i8 {
    debug_assert!(p.bbs[i].has(src));
    let mut old_bb = p.bbs[i];
    let unscaled_delta = if t.num_pieces[i] == 1 {
        let res = dest.bb_idx() as isize - src.bb_idx() as isize;
        if t.piece_types[i] == WhitePawn { -res } else { res }
    } else {
        let mut new_bb = old_bb ^ src.bb() ^ dest.bb();
        if t.piece_types[i] == WhitePawn {
            old_bb = old_bb.flip_up_down();
            new_bb = new_bb.flip_up_down();
        }
        let k = t.num_pieces[i];
        encode(new_bb, k) as isize - encode(old_bb, k) as isize
    };
    let delta = unscaled_delta * t.multipliers[i] + t.color_change_delta(p.active);
    let new_idx = (idx as isize + delta) as usize;
    if cfg!(debug_assertions) {
        let mut new_p = PosIdx { active: !p.active, ..*p };
        let new_bb = p.bbs[i] ^ src.bb() ^ dest.bb();
        new_p.bbs[i] = new_bb;
        let expected = new_p.idx(t);
        debug_assert_eq!(
            new_idx,
            expected,
            "{delta} {unscaled_delta} {0} {src}{dest} {old_bb} {new_bb} {new_p:?}",
            expected - idx
        );
    }
    let val = table[new_idx].load(Relaxed);
    if t.piece_types[i].uncolor() == Pawn {
        match val.cmp(&DRAW) {
            Ordering::Less => MATED,
            Ordering::Equal => DRAW,
            Ordering::Greater => -MATED,
        }
    } else {
        val
    }
}

fn step<const PAWNS: bool>(
    (p_i, p): (usize, &PosIdx<PAWNS>),
    t: &TableData<PAWNS>,
    table: &[Entry],
    active: Color,
    iteration: isize,
) -> Option<i8> {
    debug_assert_eq!(t.size(), table.len());
    assert_eq!(active, p.active, "{p_i} {iteration} {p:?}",);
    // if there are two pieces on the same square, the position has been handled in a base case
    let kings = p.kings();
    let sides = p.player_bbs(t);
    let blockers = sides[White] | sides[Black];
    // Because we're writing positions with monotonically increasing DTZ, any result we've written
    // will never change again.
    if blockers.num_ones() != t.num_all_pieces() || p.pawns(t).intersects(Bitboard::backranks()) {
        return None;
    }
    if table[p_i].load(Relaxed) != DRAW {
        return None;
    }
    // the best possible outcome, no point in searching additional moves if we reach this
    let best = MATED.max(MATED + iteration as i8 * 2 + active as i8 - 1);

    // no need to test for legality: If the move results in an illegal position, the resulting entry is INVALID and
    // will not influence the minimum. Therefore, we don't even need to construct a `Chessboard`,
    // we can simply use the attacks of the individual pieces
    let test_nonking_move = |i: usize, src: Square, dest: Square| {
        let mut res = value_after(&p, t, p_i, i, src, dest, table);
        // handle ep, no need to test for legality.
        // fortunately, positions with an ep capture can't be base case positions (because ep being set implies legal moves)
        if i < t.num_pawn_bbs() && dest.rank().abs_diff(src.rank()) == 2 {
            let pawn_bb = p.pawns(t) ^ p.bbs[i];
            let possible_ep_pawns: Bitboard = (dest.bb().west() | dest.bb().east()) & pawn_bb;
            for pawn in possible_ep_pawns {
                // the position after our opponent captures en passant
                let mut new_p = PosIdx { active: !active, ..*p };
                let dest = src.pawn_advance_unchecked(active);
                let delta = src.bb() | dest.bb();
                new_p.bbs[i] ^= delta;
                let other_bb_i = 1 - i;
                let ep_res = value_after(&new_p, t, new_p.idx(t), other_bb_i, pawn, dest, table);
                // `res` is from the active player's pov instead of the inactive player's
                let ep_res = match ep_res.cmp(&DRAW) {
                    Ordering::Less => -MATED,
                    Ordering::Equal => DRAW,
                    Ordering::Greater => MATED,
                };
                res = res.max(ep_res);
            }
        }
        debug_assert!(
            res >= best || best > DRAW,
            "{res} {best} {iteration} {i} {src}{dest} {active} {p_i} {p:?} {kings:?}"
        );
        debug_assert!(
            res >= DRAW || res < best + 2,
            "{res} {best} {iteration} {i} {src}{dest} {active} {p_i} {p:?} {kings:?}"
        );
        res
    };

    let test_king_move = |king_dest: Square| {
        let kings = match active {
            White => [king_dest, kings[Black]],
            Black => [kings[White], king_dest],
        };
        let new_k_idx = TableData::<PAWNS>::kings_index(kings);
        let new_idx = if new_k_idx != INVALID_KING_IDX as usize {
            // no need to normalize; we can compute the index delta without creating a new `PosIdx`
            let unscaled_delta = new_k_idx as isize - p.king_idx as isize;
            let delta = unscaled_delta * t.kings_multiplier + t.color_change_delta(p.active);
            let res = (p_i as isize + delta) as usize;
            if cfg!(debug_assertions) {
                let mut p = PosIdx { active: !p.active, ..*p };
                p.normalize(t, kings, false);
                debug_assert_eq!(p.idx(t), res);
            }
            res
        } else {
            let mut p = *p; // `PosIdx { active: !p.active, ..p };` causes an ICE
            p.active = !p.active;
            p.normalize(t, kings, false);
            p.idx(t)
        };
        debug_assert_eq!(PosIdx::<PAWNS>::from_idx(new_idx, t).active, !active);
        table[new_idx].load(Relaxed)
    };
    let filter = if iteration == 0 { !sides[active] } else { !blockers };
    let mut res = INVALID;

    // If a pawn move or capture wins, it's an immediate win that gets dealt with in iteration 0.
    // So in all later iterations, it makes sense to test them last, and only if the best result is worse than a draw
    for i in 0..t.num_bbs - 2 {
        if t.piece_types[i].color() != Some(active) || (iteration > 0 && i < t.num_pawn_bbs()) {
            continue;
        }
        for sq in p.bbs[i] {
            let attacks = attacks_for(t.piece_types[i].uncolor(), sq, blockers, p.active) & filter;
            for dest in attacks {
                let r = test_nonking_move(i, sq, dest);
                if r <= best {
                    return Some(DRAW.max(-r - 1));
                }
                res = res.min(r);
            }
        }
    }
    for king_dest in KINGS[kings[active]] & !KINGS[kings[!active]] & filter {
        res = res.min(test_king_move(king_dest));
        if res <= best {
            return Some(DRAW.max(-res - 1));
        }
    }
    if res < DRAW {
        return Some(-res - 1); // won't find a shorter winning move
    } else if res == DRAW {
        return None; // nothing's changed
    } else if iteration == 0 {
        // we've already looked at all moves, they're all losing
        return if res == INVALID { None } else { Some(-res + 1) };
    }
    // if we're here, all quiet moves are losing, but maybe a capture or pawn move achieves a draw
    for i in 0..t.num_bbs - 2 {
        if t.piece_types[i].color() != Some(active) {
            continue;
        }
        let filter = if i < t.num_pawn_bbs() { !sides[active] } else { sides[!active] };
        for sq in p.bbs[i] {
            let attacks = attacks_for(t.piece_types[i].uncolor(), sq, blockers, active) & filter;
            for dest in attacks {
                let r = test_nonking_move(i, sq, dest);
                if r == DRAW {
                    return None;
                }
                res = res.min(r);
            }
        }
    }
    for king_dest in KINGS[kings[active]] & !KINGS[kings[!active]] & sides[!active] {
        res = res.min(test_king_move(king_dest));
        if res == DRAW {
            return None;
        }
    }
    debug_assert!(res > DRAW);
    // if all moves lead to an invalid position, the game is a draw by stalemate
    // (we can't be in check because then we'd already be MATED)
    if res == INVALID {
        return None;
    }
    // TODO: Separate draw and uninit values so that we don't try to compute attacks and look up children
    // for positions that are already known to be draws
    table[p_i].store(res, Relaxed);

    // mate(d) in (w/b): | iteration
    //    [1]    [1, 2]     0       [100]    [99, 100]
    //    [2, 3] [3, 4]     1       [98, 99] [97, 98]
    //    [4, 5] [5, 6]     2       [96, 97] [95, 96]
    //    [6, 7] [7, 8]     3
    debug_assert!(
        [0, 1].contains(&(-MATED as isize - res as isize - iteration * 2 - active as isize + 1)),
        "{iteration} {p_i} {res} {active} {0} {p:?}",
        -MATED as isize - res as isize - iteration * 2 - 1
    );
    Some(-res + 1)
}

/// Fill out the remaining positions: For each possible position, look at all legal moves and choose the maximum possible result,
/// where the order is INVALID < LOST < DRAW < WON until nothing changes anymore.
fn fixed_point_iteration<const PAWNS: bool>(t: &TableData<PAWNS>, table: &[Entry]) {
    let start = Instant::now();

    let mut last_print = Instant::now();
    for (outer_i, (w_iter, b_iter)) in PosIdx::<PAWNS>::outer_iter(t).enumerate() {
        let mut iteration = 0;
        loop {
            let fold_op = |color: Color| {
                move |mut changed, mut iter: PosIdxIter<PAWNS>| {
                    while let Some(item) = iter.next() {
                        let res = step(item, t, table, color, iteration);
                        match res {
                            None => continue,
                            Some(val) => {
                                table[item.0].store(val, Relaxed);
                                changed |= val != DRAW;
                            }
                        }
                    }
                    changed
                }
            };
            // make sure the next call to `step` sees the updated entries (probably unnecessary in practice, but technically necessary)
            fence(AcqRel);
            let mut changed = w_iter.clone().fold(|| false, fold_op(White)).reduce(|| false, |a, b| a || b);
            fence(AcqRel);
            changed |= b_iter.clone().fold(|| false, fold_op(Black)).reduce(|| false, |a, b| a || b);
            if !changed {
                break;
            }
            if start.elapsed().as_millis() >= 10_000 {
                print!("\t{}%", iteration * 2);
                _ = io::stdout().flush();
            }
            iteration += 1;
        }
        if PAWNS && start.elapsed().as_millis() >= 10_000 && last_print.elapsed().as_millis() >= 2_000 {
            last_print = Instant::now();
            println!("\nPawn index {outer_i} of {0}", t.outer_size());
        }
    }
}

/// Assumes there is no en passant square, assumes the position is already normalized
fn compact_idx<const PAWNS: bool>(p: PosIdx<PAWNS>, t: &TableData<PAWNS>, same_material: bool) -> usize {
    let us = p.active;
    let kings = p.kings();
    assert_eq!(p.bbs[t.num_bbs - 2 + usize::from(us.is_first())].to_square().unwrap(), kings[0]);
    const NUM_PAWN_SQUARES: usize = 64 - 2 * 8;
    // place the two kings first because we always have to assume there are 10 squares for the white king
    // todo: We can even use a knight of the active player to enumerate all 3 piece combinations
    // (doesn't work for sliders though because adding pieces can change which squares are attacked)
    // TODO: We can also store for each (king,king) combination and piece type the number of squares where we can place a piece such that
    // we give check to the nstm king, can encode that different pieces don't interact like this except for queens with rooks/bishops
    // TODO: In positions with equal material, we can demand that it's always White's turn to move
    // TODO: We can assume that when placing the nstm king last, there are significantly fewer squares where
    // it can be placed without being in check. So just pick a number for this (e.g. 5 instead of 10 for white pawnless kingn),
    // and if a piece configurations admits more squares, store the excess positions sparsely in a separate (sorted/hashed) list.
    let mut res = p.king_idx();
    let mut occupied = p.bbs[t.num_bbs - 1] | p.bbs[t.num_bbs - 2];
    let mut num_free = NUM_PAWN_SQUARES;
    let mut encode_bb = |bb: Bitboard, mut mask_bb: Bitboard, piece: PieceType, color: Color, num_free: &mut usize| {
        mask_bb &= !occupied;
        let mut r = 0;
        for (i, sq) in bb.ones().enumerate() {
            let mut idx = sq.bb_idx();
            let below = Bitboard::new((1 << idx) - 1);
            let occupied_below = (below & !mask_bb).num_ones();
            let invalid = if color != us {
                Bitboard::default()
            } else if !PAWNS {
                no_pawns::checking(kings, piece, !us)
            } else {
                pawns::checking(kings, piece, !us)
            };
            // would imply the value is INVALID
            debug_assert!(!invalid.has(sq), "{sq} {invalid} {piece} {p:?} {us} {kings:?}");
            let invalid_below = (below & invalid).num_ones().saturating_sub(occupied.num_ones() - 2);
            // TODO: Also reduce size
            idx -= max(occupied_below, invalid_below);
            r += COMBINATIONS[idx][i + 1];
        }
        debug_assert!(mask_bb.contains(bb));
        let k = bb.num_ones();
        res *= COMBINATIONS[*num_free][k];
        res += r;
        *num_free -= k;
        occupied |= bb;
    };
    let pawn_mask = !Bitboard::backranks();
    // place the pawns now because we will always have to assume there are 48 free squares to choose from,
    // even if we placed other pieces first
    let mut pawn_idx = 0;
    if PAWNS && t.piece_types[0] == WhitePawn {
        encode_bb(p.bbs[0], pawn_mask, Pawn, White, &mut num_free);
        pawn_idx += 1;
    }
    if PAWNS && t.piece_types[pawn_idx] == BlackPawn {
        encode_bb(p.bbs[pawn_idx], pawn_mask, Pawn, Black, &mut num_free);
        pawn_idx += 1;
    }
    num_free += 16; // non-pawn pieces can also be placed on backranks
    // place all other pieces
    for i in pawn_idx..t.num_bbs - 2 {
        let piece = t.piece_types[i];
        // todo: iterate over the active pieces first
        encode_bb(p.bbs[i], Bitboard::new(!0), piece.uncolor(), piece.color().unwrap(), &mut num_free);
    }
    debug_assert_eq!(occupied, p.player_bbs(t)[0] | p.player_bbs(t)[1]);
    if !same_material {
        res *= 2;
        res += us as usize;
    }
    res
}

/// Computes the size of a compact table, i.e. a value > the maximum return value of compact_idx
fn compact_size(piece_counts: PieceCounts) -> usize {
    const NUM_PAWN_SQUARES: usize = 64 - 2 * 8;
    let no_pawns = piece_counts[White][Pawn as usize] + piece_counts[Black][Pawn as usize] == 0;
    let num_king_combos = if no_pawns { no_pawns::NUM_KING_SQUARES } else { pawns::NUM_KING_SQUARES / 2 };
    let mut res = num_king_combos;
    let mut num_free = NUM_PAWN_SQUARES;
    let mut encode_pieces = |k: u8, num_free: &mut usize| {
        res *= COMBINATIONS[*num_free][k as usize];
        *num_free -= k as usize;
    };
    // place the pawns now because we will always have to assume there are 48 free squares to choose from,
    // even if we placed other pieces first
    encode_pieces(piece_counts[White][Pawn as usize], &mut num_free);
    encode_pieces(piece_counts[Black][Pawn as usize], &mut num_free);
    num_free += 16; // non-pawn pieces can also be placed on backranks
    // place all other pieces
    for c in Color::iter() {
        for p_idx in 1..5 {
            encode_pieces(piece_counts[c][p_idx], &mut num_free);
        }
    }
    // when both sides have the same material, we demand that it's white's turn to move
    if piece_counts[White] != piece_counts[Black] {
        res *= 2;
    }
    res
}

fn normalize<const PAWNS: bool>(mut p: PosIdx<PAWNS>, t: &TableData<PAWNS>, same_material: bool) -> PosIdx<PAWNS> {
    let mut t = *t;
    if same_material && p.active == Black {
        for i in 0..t.num_bbs {
            t.piece_types[i] = t.piece_types[i].flip_color();
            if PAWNS {
                p.bbs[i] = p.bbs[i].flip_up_down();
            }
        }
        for i in 0..t.num_bbs - 1 {
            if t.piece_types[i].flip_color() == t.piece_types[i + 1] {
                p.bbs.swap(i, i + 1);
                t.piece_types.swap(i, i + 1);
            }
        }
        p.active = White;
    }
    let kings = [p.bbs[t.num_bbs - 2], p.bbs[t.num_bbs - 1]];
    let kings = kings.map(|bb| bb.to_square().unwrap());
    p.normalize(&t, kings, true);
    p
}

fn postprocess<const PAWNS: bool>(table: &[Entry], t: &TableData<PAWNS>) -> Vec<Entry> {
    let draws = AtomicUsize::new(0);
    let wins = AtomicUsize::new(0);
    let losses = AtomicUsize::new(0);
    let same_material = t.piece_counts[White] == t.piece_counts[Black];
    let mut compressed = vec![];
    compressed.resize_with(compact_size(t.piece_counts), || Entry::new(INVALID)); // TODO: Smaller size

    let lambda = |i: usize, p2: &PosIdx<PAWNS>| {
        let colors = p2.player_bbs(t);
        if colors[White].intersects(colors[Black])
            || p2.pawns(t).intersects(Bitboard::backranks())
            || p2.kings()[White].file() >= 4
        {
            return;
        }
        let val = table[i].load(Relaxed);
        if val == INVALID {
            return;
        } else if val == DRAW {
            _ = draws.fetch_add(1, Relaxed);
        } else if (val > DRAW) == (p2.active == White) {
            _ = wins.fetch_add(1, Relaxed);
        } else {
            _ = losses.fetch_add(1, Relaxed);
        }
        let p = normalize(*p2, t, same_material);
        let idx = compact_idx(p, t, same_material);
        if !same_material {
            debug_assert_eq!(compressed[idx].load(Relaxed), INVALID, "{idx} {i} {val} {p:?}");
        } else {
            debug_assert!(
                [INVALID, val].contains(&compressed[idx].load(Relaxed)),
                "{idx} {i} {val} {p:?} {0}",
                compressed[idx].load(Relaxed)
            );
        }
        compressed[idx].store(val, Relaxed);
    };
    let handle_batch = |mut iter: PosIdxIter<PAWNS>| {
        while let Some((i, p)) = iter.next() {
            lambda(i, p);
        }
    };
    for (w_iter, b_iter) in PosIdx::<PAWNS>::outer_iter(t) {
        w_iter.for_each(|iter| handle_batch(iter));
        b_iter.for_each(|iter| handle_batch(iter));
    }
    // these values are after symmetry reduction, so they are somewhat arbitrary
    let all = compressed.len();
    let factor = if same_material { 2 } else { 1 };
    let wins = wins.load(Relaxed) / factor;
    let losses = losses.load(Relaxed) / factor;
    let draws = draws.load(Relaxed) / factor;
    let invalid = all - draws - wins - losses;
    println!(
        "White wins:{wins:8}   Draws:{draws:8}   Black wins:{losses:8}   Invalid:{invalid:8}   Percent invalid:{0:.5}   Percent win/loss:{1:.5}",
        invalid as f64 * 100.0 / all as f64,
        (wins + losses) as f64 * 100.0 / all as f64
    );
    compressed
}

fn calc_table<const PAWNS: bool>(t: &TableData<PAWNS>) -> Vec<Entry> {
    let start = Instant::now();
    // By default, assume that the position is a draw. This means that we don't need to handle the 50mr rule explicitly
    let mut table = vec![];
    let n = t.size();
    table.resize_with(n, || Entry::new(DRAW));

    // TODO: Consider encoding some tables in a sparse representation:
    // If most positions are draws, we can instead store a list of idx,outcome pairs, which should usually fit into
    // around 6 bytes each, so it's useful if less than 1/6th of positions aren't draws. This doesn't have to be done at
    // table granularity, it can also be done for both sides, or pawn positions, so basically the outermost k loops.
    // Initially, entries can be stored sorted and looked up with binary search (eytzinger layout?), but a better option
    // would be a (perfect?) hash function.
    base_case::<PAWNS>(t, &table);
    println!("Base case took {0:.3} seconds for {1:?}, size {n}", start.elapsed().as_secs_f64(), t.piece_counts);
    fixed_point_iteration::<PAWNS>(t, &table);
    println!("Iterations finished after {0:.3} seconds for {1:?}", start.elapsed().as_secs_f64(), t.piece_counts);
    let res = postprocess::<PAWNS>(&table, t);
    println!("Compacted after {0:.3} seconds, compact size {1}", start.elapsed().as_secs_f64(), res.len());
    // res
    table // TODO: Return `res` instead. Or actually, write `res` to a file and return table?
}

// for each of the 10 colored non-king piece types [P,N,B,R,Q,p,n,b,r,q], counts how often it appears
type PieceCounts = [[u8; 5]; NUM_COLORS];

fn calc_tablebase(piece_counts: PieceCounts) -> Vec<Entry> {
    if piece_counts[White][0] > 0 || piece_counts[Black][0] > 0 {
        let t = TableData::<true>::new(piece_counts);
        calc_table::<true>(&t)
    } else {
        let t = TableData::<false>::new(piece_counts);
        calc_table::<false>(&t)
    }
}

fn idx_of(pos: &Board, piece_counts: PieceCounts) -> usize {
    // todo: support querying the compact table
    if pos.piece_bb(Pawn).has_any() {
        let t = &TableData::<true>::new(piece_counts);
        PosIdx::<true>::from_chessboard(pos, t).idx(t)
    } else {
        let t = &TableData::<false>::new(piece_counts);
        PosIdx::<false>::from_chessboard(pos, t).idx(t)
    }
}

fn to_piece_list(pos: &Board) -> (PieceCounts, bool) {
    let mut res = PieceCounts::default();
    for c in Color::iter() {
        for p in PieceType::non_king_pieces() {
            let n = pos.col_piece_bb(c, p).0.num_ones();
            res[c][p as usize] = n as u8;
        }
    }
    if colors_ordered(res) {
        (res, false)
    } else {
        res.swap(0, 1);
        (res, true)
    }
}

type Tablebase = HashMap<PieceCounts, LazyLock<Vec<Entry>, Box<dyn Fn() -> Vec<Entry> + Send>>>;

// This also inserts invalid piece lists, but since they're LazyLocks that's fine - we won't attempt to access them
fn gen_piece_list(res: &mut Tablebase, list: PieceCounts, depth: usize) {
    if depth == 0 {
        return;
    }
    for c in Color::iter() {
        for i in 0..5 {
            let mut list = list;
            list[c][i] += 1;
            if colors_ordered(list) {
                _ = res.insert(list, LazyLock::new(Box::new(move || calc_tablebase(list))));
                gen_piece_list(res, list, depth - 1);
            }
        }
    }
}

static TB: LazyLock<Tablebase> = LazyLock::new(|| {
    let mut res = Tablebase::default();
    for i in 0..5 {
        let mut list = PieceCounts::default();
        list[White][i] = 1;
        _ = res.insert(list, LazyLock::new(Box::new(move || calc_tablebase(list))));
        gen_piece_list(&mut res, list, MAX_TB_MAN - 2 - 1);
    }
    res
});

/// Ensures this list and all lists required to compute it exist.
/// Computes them in the right order such that for any table t, all tables required for t have been computed before t is computed.
/// This ensures all tables can be computed with the maximum parallelism and no thread has to wait for another thread
/// to finish computing a required table.
fn force_dtz_table(mut pieces: PieceCounts) -> &'static [Entry] {
    if !colors_ordered(pieces) {
        pieces.swap(0, 1);
    }
    if pieces[White] == [0, 0, 0, 0, 0] {
        return &[];
    }
    for c in Color::iter() {
        for (i, &n) in pieces[c].iter().enumerate() {
            if n == 0 {
                continue;
            }
            let mut pieces = pieces;
            pieces[c][i] -= 1;
            _ = force_dtz_table(pieces);
            if i == Pawn as usize {
                for p in [Knight, Bishop, Rook, Queen] {
                    let mut pieces = pieces;
                    pieces[c][p as usize] += 1;
                    _ = force_dtz_table(pieces);
                }
            }
        }
    }
    let Some(res) = TB.get(&pieces) else { panic!("piece list not in TB (too many pieces?): {pieces:?}") };
    LazyLock::force(res).as_ref()
}

// TODO: Use compact table insteda of the intermediate tables created during construction
fn probe_dtz(mut pos: Board) -> i8 {
    if let Some(sq) = pos.ep_square {
        pos.ep_square = None;
        let pawn = sq.pawn_advance_unchecked(!pos.active);
        debug_assert!(pos.is_piece_on(pawn, ColoredPieceType::new(!pos.active, Pawn)));
        let ep_pawns = (pawn.bb().east() | pawn.bb().west()) & pos.col_piece_bb(pos.active, Pawn);
        let mut res = i8::MIN;
        for p in ep_pawns {
            let mut pos = pos;
            pos.bbs.pieces[Pawn] ^= sq.bb() | p.bb() | pawn.bb();
            pos.bbs.colors[pos.active] ^= sq.bb() | p.bb();
            pos.bbs.colors[!pos.active] ^= pawn.bb();
            pos.active = !pos.active;
            let r = probe_dtz(pos);
            let r = match r.cmp(&DRAW) {
                Ordering::Less => -MATED - 1,
                Ordering::Equal => DRAW,
                Ordering::Greater => MATED + 1,
            };
            res = res.max(r);
        }
        let normal_res = probe_dtz(pos);
        return res.max(normal_res);
    }
    let (list, flipped) = to_piece_list(&pos);
    if flipped {
        // don't call pos.flip_side_to_move because that does unnecessary work like computing threats;
        // we don't actually require a consistent position
        pos.bbs.colors.swap(0, 1);
        pos.active = !pos.active;
        for bb in &mut pos.bbs.pieces {
            *bb = bb.flip_up_down();
        }
        for bb in &mut pos.bbs.colors {
            *bb = bb.flip_up_down();
        }
    }
    let Some(table) = TB.get(&list) else {
        panic!("No table for pos; too many pieces? ({list:?}) {0:?}", pos.bbs);
    };
    let idx = idx_of(&pos, list);
    let res = table[idx].load(Relaxed);
    debug_assert_ne!(res, INVALID, "{idx} {list:?} {flipped} {0:?} -- {1:?}", pos.bbs.colors, pos.bbs.pieces);
    res
}

#[allow(unused)]
mod tests {
    use super::*;
    use crate::games::chess::BitboardRepr;
    use crate::games::chess::pieces::ColoredPieceType::{BlackKing, WhiteKing};
    use crate::games::chess::pieces::Piece;
    use crate::games::chess::squares::sq;
    use crate::general::bitboards::chessboard::Bitboard;
    use crate::general::board::BoardHelpers;
    use rand::SeedableRng;
    use rand::distr::{Distribution, Uniform};
    use rand::rngs::StdRng;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn combinations_test() {
        assert_eq!(COMBINATIONS[4][3], 4);
        assert_eq!(COMBINATIONS[5][3], 10);
        assert_eq!(COMBINATIONS[42][1], 42);
        assert_eq!(COMBINATIONS[42][0], 1);
        assert_eq!(COMBINATIONS[5][4], 5);
        assert_eq!(COMBINATIONS[4][4], 1);
        assert_eq!(COMBINATIONS[8][4], 70);
    }

    #[test]
    fn decode_test() {
        const MAX: usize = 5;
        let mut arr = [0; MAX];
        let bb = decode(100, 2);
        assert_eq!(bb.raw(), (1 << 9) | (1 << 14));
        for k in 1..MAX {
            for n in 0..COMBINATIONS[64][k] {
                let bb = decode(n, k);
                let res = encode(bb, bb.num_ones());
                assert_eq!(res, n, "{k} {arr:?}");
            }
        }
    }

    #[test]
    fn only_kings_test() {
        let piece_counts = PieceCounts::default();
        let t = &TableData::<false>::new(piece_counts);
        for i in 0..no_pawns::NUM_KING_SQUARES {
            let p = PosIdx::<NO_PAWNS>::from_idx(i, t);
            let kings = p.kings();
            assert!(kings[0].bb_idx() < 32);
            assert_eq!(kings, no_pawns::KING_SQUARES[i]);
            assert_eq!(no_pawns::kings_idx(kings), i);
            let idx = p.idx(t);
            assert_eq!(i, idx);
        }
    }

    fn piece_v_king_is_won(
        piece: PieceType,
        our_piece: Square,
        our_king: Square,
        their_king: Square,
        stm: Color,
    ) -> bool {
        match piece {
            Pawn => query_pawn_v_king(&PAWN_V_KING_TABLE, our_piece, our_king, their_king, !stm.is_first()) != Draw,
            Knight | Bishop => false,
            Rook | Queen => {
                if stm == White {
                    return true;
                }
                // piece can be captured
                if (KINGS[their_king] & !KINGS[our_king] & our_piece.bb()).has_any() {
                    return false;
                }
                // stalemate
                if EDGE_SQUARES.has(their_king) && KINGS[their_king].intersects(KINGS[our_king]) {
                    let blockers = our_king.bb() | their_king.bb();
                    let slider_gen = ChessSliderGenerator::new(blockers);
                    let attacks = if piece == Rook {
                        slider_gen.rook_attacks(our_piece)
                    } else {
                        slider_gen.queen_attacks(our_piece)
                    };
                    return KINGS[their_king].intersects(!(attacks | KINGS[our_king])) || attacks.has(their_king);
                }
                true
            }
            King | Empty => unreachable!(),
        }
    }

    #[test]
    fn rook_vs_king_test() {
        let pieces = [[0, 0, 0, 1, 0], [0, 0, 0, 0, 0]];
        let t = TableData::new(pieces);
        let table = force_dtz_table(pieces);
        let pos = Board::from_fen("8/8/8/8/8/K1k5/8/2r5 b - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, -MATED - 1);
        let pos = Board::from_fen("8/8/8/8/8/2k5/K7/2r5 w - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, MATED + 2);
        let pos = Board::from_fen("8/8/8/8/8/2k5/K7/3r4 b - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, -MATED - 3);

        let mut j = 0;
        for c in Color::iter() {
            let n = table.len() / 2;
            let current = c as usize * n;
            let mut iter = PosIdxIter::<false> {
                pos_idx: PosIdx::<false>::from_idx(current, &t),
                t: &t,
                first: true,
                current,
                end: current + n,
            };
            while let Some((i, p)) = iter.next() {
                assert_eq!(i, j);
                assert_eq!(*p, PosIdx::<false>::from_idx(i, &t));
                j += 1;
            }
        }
    }

    #[test]
    fn single_piece_test() {
        for gen_table in [false, true] {
            for p in [Knight, Bishop, Rook, Queen, Pawn] {
                let mut piece_counts = PieceCounts::default();
                piece_counts[White][p as usize] = 1;
                if gen_table {
                    _ = force_dtz_table(piece_counts);
                }
                for w_k in Square::iter() {
                    for b_k in Square::iter() {
                        for w_p in Square::iter() {
                            let piece = Piece::new(ColoredPieceType::new(White, p), w_p);
                            let mut pos = Board::empty();
                            pos.place_piece(w_k, WhiteKing);
                            let Ok(()) = pos.try_place_piece(Piece::new(BlackKing, b_k)) else { continue };
                            let Ok(()) = pos.try_place_piece(piece) else { continue };
                            let Ok(pos) = pos.verify(Strict) else { continue };
                            if p == Pawn {
                                let t = &TableData::<true>::new(piece_counts);
                                let p_idx = PosIdx::<PAWNS>::from_chessboard(&pos, t);
                                let idx = p_idx.idx(t);
                                assert_eq!(p_idx, PosIdx::from_idx(idx, t));
                            } else {
                                let t = &TableData::<false>::new(piece_counts);
                                let p_idx = PosIdx::<NO_PAWNS>::from_chessboard(&pos, t);
                                let idx = p_idx.idx(t);
                                assert_eq!(p_idx, PosIdx::from_idx(idx, t));
                            };
                            if !gen_table {
                                continue;
                            }
                            let dtz = probe_dtz(pos);
                            let won = piece_v_king_is_won(p, w_p, w_k, b_k, White);
                            assert!(dtz >= 0, "{dtz} {p} {pos}");
                            assert_eq!(dtz > 0, won, "{dtz} {p} {pos}");
                            assert!(dtz <= 100, "{dtz} {p} {pos}");
                        }
                    }
                }
            }
        }
    }

    const PAWNS: bool = true;
    const NO_PAWNS: bool = false;

    #[test]
    #[ignore]
    fn immediate_game_over_test() {
        let pieces = [[0, 1, 0, 0, 0], [0, 1, 0, 0, 0]];
        let t = &TableData::<NO_PAWNS>::new(pieces);
        let table = force_dtz_table(pieces);

        let b_king = sq("b3");
        for w_king in Square::iter() {
            if (KINGS[b_king] | b_king.bb() | sq("b1").bb() | sq("c2").bb()).has(w_king) {
                continue;
            }
            let mut bbs = BitboardRepr::default();
            bbs.place_piece(w_king, White, King);
            bbs.place_piece(b_king, Black, King);
            bbs.place_piece(sq("b1"), White, Knight);
            bbs.place_piece(sq("c2"), Black, Knight);
            let p = PosIdx::<NO_PAWNS>::from_bitboards(bbs.pieces, bbs.colors, White, t, false, true);
            let i = p.idx(t);
            let res = table[i].load(Relaxed);
            if w_king == sq("a1") {
                assert_eq!(res, MATED);
                let mut p2 = PosIdx { active: Black, ..p };
                p2.normalize(t, [w_king, p.kings()[Black]], false);
                let i = p2.idx(t);
                assert_eq!(table[i].load(Relaxed), INVALID);
            } else {
                assert_eq!(res, DRAW);
            }
        }
        let pieces = [[0, 0, 0, 1, 0], [0, 0, 1, 0, 0]];
        let t = &TableData::<NO_PAWNS>::new(pieces);
        let table = force_dtz_table(pieces);
        let pos = Board::from_fen("5Rbk/8/7K/8/8/8/8/8 b - - 0 1", Strict).unwrap();
        let i = PosIdx::<NO_PAWNS>::from_chessboard(&pos, t).idx(t);
        let res = table[i].load(Relaxed);
        assert_eq!(res, DRAW);
    }

    #[test]
    #[ignore]
    fn idx_test() {
        let pieces = [[1, 0, 0, 0, 1], [1, 0, 0, 0, 0]];
        let t = &TableData::<PAWNS>::new(pieces);
        let mut table = vec![];
        table.resize_with(t.size(), || AtomicBool::new(false));
        for (it1, it2) in PosIdx::<PAWNS>::outer_iter(t) {
            it1.for_each(|mut it| {
                while let Some((i, _p)) = it.next() {
                    assert!(!table[i].load(Relaxed));
                    table[i].store(true, Relaxed);
                }
            });
            it2.for_each(|mut it| {
                while let Some((i, _p)) = it.next() {
                    assert!(!table[i].load(Relaxed));
                    table[i].store(true, Relaxed);
                }
            })
        }
    }

    #[test]
    #[ignore]
    fn game_over_in_one_test() {
        let pieces = [[0, 1, 0, 0, 0], [0, 1, 0, 0, 0]];
        let t = &TableData::<NO_PAWNS>::new(pieces);
        let mut pos = Board::from_fen("8/8/8/2N5/8/8/n1K5/k7 w - - 0 1", Strict).unwrap();
        let p = PosIdx::<NO_PAWNS>::from_chessboard(&pos, t);
        assert_eq!(p.kings(), [sq("b3"), sq("a1")]);
        let i = p.idx(t);
        let table = force_dtz_table(pieces);
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
        pos.bbs.move_piece(sq("a1"), sq("h1"), Black, King);
        let p = PosIdx::<NO_PAWNS>::from_chessboard(&pos, t);
        let i = p.idx(t);
        assert_eq!(table[i].load(Relaxed), DRAW, "{i}");

        let pieces = [[0, 0, 0, 0, 1], [0, 0, 0, 1, 0]];
        let t = &TableData::<NO_PAWNS>::new(pieces);
        let table = force_dtz_table(pieces);
        let test_fen = |fen: &str, outcome: i8| {
            let pos = Board::from_fen(fen, Strict).unwrap();
            let p = PosIdx::<NO_PAWNS>::from_chessboard(&pos, t);
            let i = p.idx(t);
            assert_eq!(table[i].load(Relaxed), outcome, "{i} '{fen}'");
        };
        test_fen("8/8/8/8/8/1K6/2Q5/2k3r1 b - - 0 1", MATED);
        test_fen("8/8/8/8/8/2k5/2r5/K1Q5 b - - 0 1", -MATED - 1);
        test_fen("6r1/8/8/8/8/5k2/6Q1/6K1 b - - 0 1", -MATED - 1);
        test_fen("6Q1/8/8/8/8/5K2/6r1/6k1 w - - 0 1", -MATED - 1);

        let mut pos = Board::from_fen("8/8/8/8/8/2K5/k7/2rQ4 w - - 0 1", Strict).unwrap();
        let p = PosIdx::<NO_PAWNS>::from_chessboard(&pos, t);
        let res = value_after(&p, t, p.idx(t), 1, p.bbs[1].to_square().unwrap(), p.bbs[0].to_square().unwrap(), table);
    }

    #[test]
    #[ignore]
    fn queen_vs_rook_test() {
        let pieces = [[0, 0, 0, 0, 1], [0, 0, 0, 1, 0]];
        let table = force_dtz_table(pieces);
        let t = &TableData::<NO_PAWNS>::new(pieces);

        let test_fen = |fen: &str, outcome: i8| {
            let pos = Board::from_fen(fen, Strict).unwrap();
            let p = PosIdx::<NO_PAWNS>::from_chessboard(&pos, t);
            let i = p.idx(t);
            assert_eq!(table[i].load(Relaxed), outcome, "{i} '{fen}'");
        };
        test_fen("6Q1/8/8/8/8/5K2/6r1/6k1 w - - 0 1", -MATED - 1);
        test_fen("6Q1/8/8/8/8/5K2/r7/7k w - - 0 1", -MATED - 1);
        test_fen("6Q1/8/8/8/8/5K2/r7/5k2 w - - 0 1", -MATED - 1);
        test_fen("6Q1/8/8/8/8/5K2/r6k/8 w - - 0 1", -MATED - 1);
        test_fen("6Q1/8/8/8/8/5K2/r7/6k1 b - - 0 1", MATED + 2);
        test_fen("7Q/8/8/8/8/5K2/r7/6k1 w - - 0 1", -MATED - 3);
        test_fen("r7/7Q/8/8/8/5K2/8/6k1 w - - 0 1", -MATED - 3);
        test_fen("8/8/2k5/1r6/8/8/8/2KQ4 b - - 0 1", MATED + 62);
        test_consistency::<NO_PAWNS>(t, table);
    }

    fn test_consistency<const PAWNS: bool>(t: &TableData<PAWNS>, table: &[Entry]) {
        let seed = 42;
        let mut rng = StdRng::seed_from_u64(seed);
        let dist = Uniform::new(0, table.len()).unwrap();
        for _ in 0..5_000_000 {
            let idx = dist.sample(&mut rng);
            let res = table[idx].load(Relaxed);
            if res == INVALID {
                continue;
            }
            let p = PosIdx::<PAWNS>::from_idx(idx, t);
            let bbs = p.player_bbs(t);
            if bbs[White].intersects(bbs[Black]) || p.pawns(t).intersects(Bitboard::backranks()) {
                continue;
            }
            let mut pos = Board::empty();
            for i in 0..t.num_bbs {
                for sq in p.bbs[i] {
                    pos.place_piece(sq, t.piece_types[i]);
                }
            }
            pos.set_active_player(p.active);
            let pos = pos.verify(Strict).unwrap();
            // assert_eq!(idx_of(&pos), idx, "{idx} {pos}");
            assert_eq!(
                probe_dtz(pos),
                res,
                "{res} {idx} {0} {pos} {1:?}",
                idx_of(&pos, t.piece_counts),
                PosIdx::<PAWNS>::from_idx(idx_of(&pos, t.piece_counts), t)
            );
            let mut max = -120;
            let mut best = pos;
            for child in pos.children() {
                let mut child_res = probe_dtz(child);
                assert_ne!(child_res, INVALID, "{idx} {0} '{pos}' '{child}'", idx_of(&child, t.piece_counts));
                let pawn_move = child.piece_bb(Pawn) != pos.piece_bb(Pawn);
                let capture = child.occupied_bb().num_ones() != pos.occupied_bb().num_ones();
                if pawn_move || capture {
                    child_res = match child_res.cmp(&DRAW) {
                        Ordering::Less => MATED,
                        Ordering::Equal => DRAW,
                        Ordering::Greater => -MATED,
                    };
                }
                if -child_res > max {
                    max = -child_res;
                    best = child;
                }
            }
            if max == -120 {
                assert!(res == 0 || res == MATED);
                continue;
            }
            let recomputed = if max == 0 {
                max
            } else if max < 0 {
                max + 1
            } else {
                max - 1
            };
            assert_eq!(recomputed, res, "{idx}: {max} {res} [{pos}] -- {best}");
        }
    }

    #[test]
    #[ignore]
    fn piece_vs_2pieces_test() {
        let t = TableData::<false>::new([[0, 1, 1, 0, 0], [0, 0, 0, 1, 0]]);
        let table = force_dtz_table(t.piece_counts);
        let mut pos = Board::from_fen("8/7r/8/8/8/1KN5/1B6/1k6 b - - 0 1", Strict).unwrap();
        let p = PosIdx::<NO_PAWNS>::from_chessboard(&pos, &t);
        let i = p.idx(&t);
        assert_eq!(table[i].load(Relaxed), MATED, "{i}");
        let pos = Board::from_fen("8/7r/8/8/8/1K6/1B2N3/1k6 w - - 0 1", Strict).unwrap();
        let p = PosIdx::<NO_PAWNS>::from_chessboard(&pos, &t);
        let i = p.idx(&t);
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
        let pos = Board::from_fen("8/8/8/N7/8/r7/B7/k1K5 b - - 0 1", Strict).unwrap();
        assert_eq!(table[idx_of(&pos, t.piece_counts)].load(Relaxed), -MATED - 1);

        test_consistency::<NO_PAWNS>(&t, table);
    }

    // todo: Also support querying the compact pawn table
    #[test]
    fn pawn_vs_king_test() {
        let list: PieceCounts = [[1, 0, 0, 0, 0], [0, 0, 0, 0, 0]];
        let t = &TableData::<PAWNS>::new(list);
        let table = force_dtz_table(list);
        let pos = Board::from_fen("8/P7/8/8/8/8/8/K1k5 w - - 0 1", Strict).unwrap();
        let p = PosIdx::<PAWNS>::from_chessboard(&pos, t);
        println!("{}", p.idx(t));
        assert_eq!(table[p.idx(t)].load(Relaxed), -MATED - 1, "{0} {1} {p:?}", p.idx(t), table[p.idx(t)].load(Relaxed));
        for (i, e) in table.iter().enumerate().rev() {
            let e = e.load(Relaxed);
            let p = PosIdx::<PAWNS>::from_idx(i, t);
            assert_eq!(t.piece_types[0..t.num_bbs], [WhitePawn, WhiteKing, BlackKing]);
            if p.bbs[2] == p.bbs[0] {
                assert!([DRAW, INVALID].contains(&e));
                continue;
            }
            let pawn = p.bbs[0].to_square().unwrap();
            if pawn.rank() == 0 {
                assert_eq!(e, INVALID);
            }
            if e == INVALID || pawn.is_backrank() {
                continue;
            }
            let won = piece_v_king_is_won(Pawn, pawn, p.kings()[White], p.kings()[Black], p.active);
            assert_eq!(e != 0, won, "{i} {e} {won} {p:?}");
            if e != 0 {
                assert_eq!(e > 0, p.active == White, "{i} {e} {won} {p:?}");
            }
        }
    }

    #[test]
    #[ignore]
    fn piece_vs_pawn_test() {
        let list: PieceCounts = [[0, 1, 0, 0, 0], [1, 0, 0, 0, 0]];
        let t = &TableData::<PAWNS>::new(list);
        let table = force_dtz_table(list);
        let pos = Board::from_fen("8/8/8/8/8/1N6/p1K5/k7 b - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, MATED);
        let pos = Board::from_fen("8/8/8/8/8/p7/2K5/k1N5 b - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, MATED + 1, "{0} {1:?}", idx_of(&pos, list), PosIdx::<PAWNS>::from_chessboard(&pos, t));
        let pos = Board::from_fen("k4N2/8/3K4/8/8/8/p7/8 w - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, MATED + 2);
        let pos = Board::from_fen("k4N2/8/3K4/8/8/p7/8/8 b - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, -MATED - 1);
        let pos = Board::from_fen("8/1K6/8/8/5k2/8/6p1/3N4 w - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, MATED + 8);
        let pos = Board::from_fen("8/1K6/8/8/5k2/6p1/8/3N4 b - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, -MATED - 1);
        test_consistency::<PAWNS>(t, table);
    }

    #[test]
    #[ignore]
    fn pawn_vs_pawn_test() {
        let list: PieceCounts = [[1, 0, 0, 0, 0], [1, 0, 0, 0, 0]];
        let t = &TableData::<PAWNS>::new(list);
        let table = force_dtz_table(list);
        let pos = Board::from_fen("8/8/8/8/5p2/8/6P1/5K1k w - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, DRAW); // would be won without ep
        let pos = Board::from_fen("8/8/8/8/5pP1/8/8/5K1k b - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, MATED + 2); // no ep
        let pos = Board::from_fen("8/8/8/8/5pP1/8/8/5K1k b - g3 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, -MATED - 1); // ep
        let pos = Board::from_fen("8/8/8/8/K6p/8/6P1/7k w - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, MATED + 2, "{0}", idx_of(&pos, list));
        test_consistency::<PAWNS>(t, table);
    }

    #[test]
    #[ignore]
    fn two_same_pieces_test() {
        let list: PieceCounts = [[0, 0, 2, 0, 0], [0, 0, 0, 0, 0]];
        let t = &TableData::<NO_PAWNS>::new(list);
        let table = force_dtz_table(list);
        test_consistency::<NO_PAWNS>(t, table);
        let list: PieceCounts = [[2, 0, 0, 0, 0], [0, 0, 0, 0, 0]];
        let t = &TableData::<PAWNS>::new(list);
        let table = force_dtz_table(list);
        let pos = Board::from_fen("8/1k2P3/8/8/8/8/1P6/1K6 w - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, -MATED - 1, "{0}", idx_of(&pos, t.piece_counts));
        test_consistency::<PAWNS>(t, table);
    }

    #[test]
    #[ignore]
    fn two_knights_vs_pawn_test() {
        let list = [[0, 2, 0, 0, 0], [1, 0, 0, 0, 0]];
        let t = &TableData::<PAWNS>::new(list);
        let table = force_dtz_table(list);
        let pos = Board::from_fen("8/8/8/8/2N5/4N3/2K4p/k7 w - - 0 1", Strict).unwrap();
        let i = PosIdx::<PAWNS>::from_chessboard(&pos, t).idx(t);
        assert_eq!(table[i].load(Relaxed), DRAW);
        let pos = Board::from_fen("8/8/8/8/2p1K3/2k5/5NN1/8 b - - 0 1", Strict).unwrap();
        let p = PosIdx::<PAWNS>::from_chessboard(&pos, t);
        let i = p.idx(t);
        let res = table[i].load(Relaxed);
        assert_eq!(res, DRAW);
        for p in pos.children() {
            let i = PosIdx::<PAWNS>::from_chessboard(&p, t).idx(t);
            assert_eq!(table[i].load(Relaxed), DRAW, "{i} {p:?}");
        }
        test_consistency::<PAWNS>(t, table);
    }

    #[test]
    #[ignore]
    fn long_mate_test() {
        let list = [[0, 0, 0, 0, 1], [0, 0, 2, 0, 0]];
        let table = force_dtz_table(list);
        let t = &TableData::<NO_PAWNS>::new(list);
        let test_fen = |fen: &str, outcome: i8| {
            let pos = Board::from_fen(fen, Strict).unwrap();
            let p = PosIdx::<NO_PAWNS>::from_chessboard(&pos, t);
            let i = p.idx(t);
            assert_eq!(table[i].load(Relaxed), outcome, "{i} {p:?} '{fen}'");
        };
        // DTZ 142, the longest DTZ in this table, which is too large
        test_fen("8/1q6/8/8/2B5/8/3K4/k5B1 w - - 0 1", DRAW);
        test_fen("8/8/8/5q2/5B2/5K2/2k1B3/8 b - - 0 1", DRAW); // DTZ 101, also too large
        test_fen("8/8/8/5q2/5B2/2k2K2/4B3/8 w - - 0 1", -1); // DTZ 100
        test_fen("8/8/8/5q2/5B2/2k3K1/4B3/8 b - - 0 1", 2); // DTZ 99
        test_fen("8/8/8/5q2/3k1B2/6K1/4B3/8 w - - 0 1", -3); // DTZ 98
        test_fen("8/8/8/5q2/3k1B2/5BK1/8/8 b - - 0 1", 4); // DTZ 97
        test_consistency::<NO_PAWNS>(t, table);
    }
}
