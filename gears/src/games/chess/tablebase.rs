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
use crate::games::chess::ChessColor::{Black, White};
use crate::games::chess::bitbase::{PAWN_V_KING_TABLE, query_pawn_v_king};
use crate::games::chess::pieces::ChessPieceType::{Bishop, Empty, King, Knight, Pawn, Queen, Rook};
use crate::games::chess::pieces::{ChessPieceType, ColoredChessPieceType, NUM_CHESS_PIECES};
use crate::games::chess::squares::{A_FILE_NUM, B_FILE_NUM, C_FILE_NUM, ChessSquare, D_FILE_NUM, NUM_SQUARES};
use crate::games::chess::tablebase::KingSymmetry::{CompactPawnTable, GeneratePawnTable, NoPawns};
use crate::games::chess::unverified::UnverifiedChessboard;
use crate::games::chess::{ChessBitboardTrait, ChessColor, Chessboard, EDGE_SQUARES};
use crate::games::{Color, ColoredPieceType, DimT, NUM_COLORS, PieceType};
use crate::general::bitboards::chessboard::{ChessBitboard, KINGS, KNIGHTS};
use crate::general::bitboards::{Bitboard, KnownSizeBitboard, RawBitboard};
use crate::general::board::Strictness::Strict;
use crate::general::board::{BitboardBoard, Board, UnverifiedBoard};
use crate::general::hq::ChessSliderGenerator;
use crate::general::squares::{RectangularCoordinates, SmallGridSquare};
use arrayvec::ArrayVec;
use number_encoding::combinadics;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::atomic::Ordering::{AcqRel, Relaxed};
use std::sync::atomic::{AtomicI8, AtomicUsize, fence};
use std::time::Instant;
use strum_macros::FromRepr;

type Entry = AtomicI8;

// Generate up to 6 man tablebases; assumes that usize is at least 64 bits large
const MAX_TB_MAN: usize = 6;
const MAX_NON_K_PIECES: usize = MAX_TB_MAN - 2;

mod no_pawns {
    use super::*;

    const NUM_KING_SYMMETRY_SQUARES: usize = 10;

    static KING_SQUARES_SYMMETRY: [ChessSquare; NUM_KING_SYMMETRY_SQUARES] = [
        ChessSquare::from_bb_idx(A_FILE_NUM as usize),
        ChessSquare::from_bb_idx(B_FILE_NUM as usize),
        ChessSquare::from_bb_idx(C_FILE_NUM as usize),
        ChessSquare::from_bb_idx(D_FILE_NUM as usize),
        ChessSquare::from_bb_idx(B_FILE_NUM as usize + 8),
        ChessSquare::from_bb_idx(C_FILE_NUM as usize + 8),
        ChessSquare::from_bb_idx(D_FILE_NUM as usize + 8),
        ChessSquare::from_bb_idx(C_FILE_NUM as usize + 16),
        ChessSquare::from_bb_idx(D_FILE_NUM as usize + 16),
        ChessSquare::from_bb_idx(D_FILE_NUM as usize + 24),
    ];

    static KING_INDICES_SYMMETRY: [usize; 64] = {
        let mut res: [usize; 64] = [usize::MAX; 64];
        let mut i = 0;
        while i < 64 {
            let mut j = 0;
            while j < NUM_KING_SYMMETRY_SQUARES {
                if KING_SQUARES_SYMMETRY[j].bb_idx() == i {
                    res[i] = j;
                }
                j = j + 1;
            }
            i = i + 1;
        }
        res
    };

    pub const NUM_KING_SQUARES: usize = 462;

    pub static KING_SQUARES: [[ChessSquare; NUM_COLORS]; NUM_KING_SQUARES] = {
        let mut res = [[ChessSquare::from_bb_idx(0), ChessSquare::from_bb_idx(0)]; NUM_KING_SQUARES];
        let mut i = 0;
        let mut w_king_table_idx: usize = 0;
        while w_king_table_idx < NUM_KING_SYMMETRY_SQUARES {
            let w_king = KING_SQUARES_SYMMETRY[w_king_table_idx];
            let w_king_idx = w_king.bb_idx();
            let forbidden = KINGS[w_king_idx].0 | (1 << w_king_idx);
            let mut b_king_idx: usize = 0;
            while b_king_idx < 64 {
                let b_king = ChessSquare::from_bb_idx(b_king_idx);
                if (forbidden & (1 << b_king_idx)) != 0
                    || (w_king_idx / 8 == w_king_idx % 8 && b_king_idx / 8 > b_king_idx % 8)
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
    };

    pub static KING_INDICES: [[u16; 64]; NUM_KING_SYMMETRY_SQUARES] = {
        let mut res = [[u16::MAX; 64]; NUM_KING_SYMMETRY_SQUARES];
        let mut i = 0;
        while i < NUM_KING_SQUARES {
            let [w, b] = KING_SQUARES[i];
            let w = KING_INDICES_SYMMETRY[w.bb_idx()];
            res[w][b.bb_idx()] = i as u16;
            i = i + 1;
        }
        res
    };

    pub fn kings_idx(kings: [ChessSquare; NUM_COLORS]) -> usize {
        let w = KING_INDICES_SYMMETRY[kings[White]];
        KING_INDICES[w][kings[Black]] as usize
    }
}

mod pawns {
    use super::*;

    pub const NUM_KING_SQUARES: usize = 3612;

    pub static KING_SQUARES: [[ChessSquare; NUM_COLORS]; NUM_KING_SQUARES] = {
        let mut res = [[ChessSquare::from_bb_idx(0), ChessSquare::from_bb_idx(0)]; NUM_KING_SQUARES];
        let mut i = 0;
        let mut w_king_idx: usize = 0;
        while w_king_idx < 64 {
            let w_king = ChessSquare::from_bb_idx(w_king_idx);
            let mut b_king_idx: usize = 0;
            while b_king_idx < 64 {
                let b_king = ChessSquare::from_bb_idx(b_king_idx);
                if ((KINGS[b_king_idx].0 | (1 << b_king_idx)) & (1 << w_king.bb_idx())) != 0 {
                    b_king_idx += 1;
                    continue;
                }
                res[i] = [w_king, b_king];
                i += 1;
                b_king_idx += 1;
            }
            w_king_idx += 1;
        }
        assert!(i == NUM_KING_SQUARES);
        res
    };

    pub static KING_INDICES: [[u16; 64]; 64] = {
        let mut res = [[u16::MAX; 64]; 64];
        let mut i = 0;
        while i < NUM_KING_SQUARES {
            let [w, b] = KING_SQUARES[i];
            res[w.bb_idx()][b.bb_idx()] = i as u16;
            i = i + 1;
        }
        res
    };
}

// use the maximum value so that "negamax" never chooses it if there's a legal position
const INVALID: i8 = 127;
const MATED: i8 = -100;
const DRAW: i8 = 0;

fn attacks_for(wx_piece: ChessPieceType, w_x: ChessSquare, blockers: ChessBitboard, us: ChessColor) -> ChessBitboard {
    match wx_piece {
        Pawn => {
            let bb = w_x.bb();
            let push = bb.pawn_advance(us) & !blockers;
            let d_push = (push & ChessBitboard::pawn_ranks().pawn_advance(us)).pawn_advance(us) & !blockers;
            push | d_push | (bb.pawn_attacks(us) & blockers)
        }
        Knight => KNIGHTS[w_x],
        Bishop => ChessSliderGenerator::new(blockers).bishop_attacks(w_x),
        Rook => ChessSliderGenerator::new(blockers).rook_attacks(w_x),
        Queen => ChessSliderGenerator::new(blockers).queen_attacks(w_x),
        _ => unreachable!(),
    }
}

fn piece_type_idx(pieces: &[ChessPieceType]) -> usize {
    let mut res = 0;
    debug_assert!(pieces.is_sorted());
    for &p in pieces {
        debug_assert!(p < King);
        res *= 6; // PAWN, KNIGHT, BISHOP, ROOK, QUEEN
        res += p as usize + 1;
    }
    res
}

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

#[derive(Debug, Copy, Clone, Eq, PartialEq, FromRepr)]
enum KingSymmetry {
    // The board is mirrored horizontally, vertically and along the main diagonal so that the white king is always
    // in one of the KING_SQUARES_SYMMETRY squares
    NoPawns,
    // The board is not mirrored at all. Mirroring the board horizontally would cause pawns to switch places,
    // which would mean we can't iterate over pawn squares in the outer iteration. So this means the table used
    // during construction is twice as large as it could be, but pawn positions are fixed during fixed-point iteration.
    // This is effectively the same as having a separate table per pawn bitboard, reducing the working set size.
    GeneratePawnTable,
    // There are no invalid positions.
    // The board is mirrored horizontally so that the white king is always on the left side of the board.
    // Pawns can't be on the backrank, no two pieces can be on the same square, and the sntm can't be in check.
    CompactPawnTable,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct PosIdx<const N_W: usize, const N_B: usize, const SYMMETRY: usize> {
    king_idx: usize,
    w_nk: [ChessSquare; N_W],
    b_nk: [ChessSquare; N_B],
    pieces: PieceList,
    active: ChessColor,
}

impl<const N_W: usize, const N_B: usize, const SYMMETRY: usize> PosIdx<N_W, N_B, SYMMETRY> {
    // TODO: For a white king on the main diagonal, we can exploit more symmetry by fixing on which side of the main diagonal
    // the black king has to be. Also, if both sides have the same piece types we can exploit white/black symmetry,
    // and if a side has the same piece type multiple times the order can also be ignored

    const fn symmetry() -> KingSymmetry {
        KingSymmetry::from_repr(SYMMETRY).unwrap()
    }

    fn from_chessboard(pos: &Chessboard) -> Self {
        let mut w_pieces = [SmallGridSquare::default(); N_W];
        let mut b_pieces = [SmallGridSquare::default(); N_B];
        let mut w_i = 0;
        let mut b_i = 0;
        let mut pieces = PieceList::default();
        debug_assert_eq!(N_W, pos.player_bb(White).num_ones() - 1);
        debug_assert_eq!(N_B, pos.player_bb(Black).num_ones() - 1);
        debug_assert_eq!(Self::symmetry() == NoPawns, pos.piece_bb(Pawn).is_zero());
        for p in ChessPieceType::non_king_pieces() {
            for sq in pos.col_piece_bb(White, p) {
                w_pieces[w_i] = sq;
                pieces[White][p as usize] += 1;
                w_i += 1;
            }
            for sq in pos.col_piece_bb(Black, p) {
                b_pieces[b_i] = sq;
                pieces[Black][p as usize] += 1;
                b_i += 1;
            }
        }
        Self::normalized(pos.king_sq(White), pos.king_sq(Black), w_pieces, b_pieces, pos.active, pieces)
    }

    fn normalized(
        wk: ChessSquare,
        bk: ChessSquare,
        w_nk: [ChessSquare; N_W],
        b_nk: [ChessSquare; N_B],
        active: ChessColor,
        pieces: PieceList,
    ) -> Self {
        let res = Self { king_idx: 0, w_nk, b_nk, active, pieces };
        res.normalize([wk, bk])
    }

    const fn has_pawn() -> bool {
        SYMMETRY != NoPawns as usize
    }

    const fn num_king_squares() -> usize {
        match Self::symmetry() {
            NoPawns => no_pawns::NUM_KING_SQUARES,
            GeneratePawnTable => pawns::NUM_KING_SQUARES,
            CompactPawnTable => pawns::NUM_KING_SQUARES / 2,
        }
    }

    fn kings(&self) -> [ChessSquare; NUM_COLORS] {
        match Self::symmetry() {
            NoPawns => no_pawns::KING_SQUARES[self.king_idx],
            GeneratePawnTable => pawns::KING_SQUARES[self.king_idx],
            CompactPawnTable => todo!(),
        }
    }

    fn kings_index(kings: [ChessSquare; 2]) -> usize {
        match Self::symmetry() {
            NoPawns => no_pawns::kings_idx(kings),
            GeneratePawnTable => pawns::KING_INDICES[kings[White].bb_idx()][kings[Black].bb_idx()] as usize,
            CompactPawnTable => todo!(),
        }
    }

    fn player_bbs(&self) -> [ChessBitboard; 2] {
        let mut res = [ChessBitboard::default(); 2];
        for w_x in self.w_nk {
            res[White] |= w_x.bb();
        }
        for b_x in self.b_nk {
            res[Black] |= b_x.bb();
        }
        res[White] |= self.kings()[White].bb();
        res[Black] |= self.kings()[Black].bb();
        res
    }

    const fn num_pawn_squares() -> usize {
        match Self::symmetry() {
            NoPawns => 0,
            GeneratePawnTable => 64 - 8,
            CompactPawnTable => 64 - 16,
        }
    }

    #[inline]
    fn encode(pieces: &[ChessSquare], flip: bool) -> usize {
        debug_assert!(pieces.windows(2).all(|w| w[0].bb_idx() < w[1].bb_idx()));
        let mut r = 0;
        if flip {
            for (i, &sq) in pieces.iter().enumerate() {
                let idx = Self::num_pawn_squares() - 1 - (sq.bb_idx() - 8);
                r += COMBINATIONS[idx][pieces.len() - i];
            }
        } else {
            for (i, &sq) in pieces.iter().enumerate() {
                r += COMBINATIONS[sq.bb_idx()][i + 1];
            }
        }
        r
    }

    fn idx(&self) -> usize {
        debug_assert!(self.king_idx < Self::num_king_squares());
        let mut res = 0;
        // hopefully, the compiler can unroll the following loops if PAWN is false.
        let w_pawns = if Self::has_pawn() { self.pieces[White][0] as usize } else { 0 };
        let b_pawns = if Self::has_pawn() { self.pieces[Black][0] as usize } else { 0 };

        // pawns are encoded separately because a) they have to be the outermost loop and b) white pawns must
        // be encoded in reverse order so that pawn pushes lead to positions that have already been computed
        res *= COMBINATIONS[Self::num_pawn_squares()][w_pawns];
        res += Self::encode(&self.w_nk[0..w_pawns], true);
        res *= COMBINATIONS[Self::num_pawn_squares()][b_pawns];
        res += Self::encode(&self.b_nk[0..b_pawns], false);

        // the active player has to be the outermost part of the inner loop so that we don't try to write an entry
        // before we've seen all the positions that can reach this entry
        res *= NUM_COLORS;
        res += self.active as usize;
        res *= Self::num_king_squares();
        res += self.king_idx;

        let mut i = w_pawns;
        for c in ChessColor::iter() {
            let pieces: &[ChessSquare] = if c == White { &self.w_nk } else { &self.b_nk };
            for &count in self.pieces[c][1..].iter() {
                let count = count as usize;
                res *= COMBINATIONS[64][count];
                let n = Self::encode(&pieces[i..i + count], false);
                res += n;
                i += count;
            }
            debug_assert_eq!(i, pieces.len());
            i = b_pawns;
        }
        debug_assert_eq!(*self, Self::from_idx(res, self.pieces), "{res} {w_pawns} {b_pawns}");
        res
    }

    fn idx_normalized(self, kings: [ChessSquare; NUM_COLORS]) -> usize {
        self.normalize(kings).idx()
    }

    fn from_idx(mut idx: usize, pieces: PieceList) -> Self {
        assert!(N_W >= N_B);
        debug_assert_ne!(pieces[White][0] == 0 && pieces[Black][0] == 0, Self::has_pawn());
        // hopefully, this allows the compiler to optimize better
        let w_pawns = if Self::has_pawn() { pieces[White][0] as usize } else { 0 };
        let b_pawns = if Self::has_pawn() { pieces[Black][0] as usize } else { 0 };
        let mut res = Self::default();

        let mut i = N_B;
        let mut arr = [0; MAX_NON_K_PIECES];
        // bijection between an index and two squares with sq.0 < sq.1, used for two pieces of the same colored piece type
        // see <https://en.wikipedia.org/wiki/Combinatorial_number_system>
        for c in ChessColor::iter().rev() {
            for &count in pieces[c][1..].iter().rev() {
                let count = count as usize;
                let k = match count {
                    0 => continue,
                    1 => {
                        arr[0] = idx % 64;
                        64
                    }
                    count => {
                        let k = COMBINATIONS[64][count];
                        combinadics::decode_mut(idx % k, count, &mut arr[0..count]);
                        k
                    }
                };
                idx /= k;
                let pieces: &mut [ChessSquare] = if c == Black { &mut res.b_nk } else { &mut res.w_nk };
                i -= count;
                for j in 0..count {
                    let n = arr[j]; // ith_one_u64(arr[j], !occupied.raw()); // only works for querying, not when computing tables
                    pieces[i + j] = ChessSquare::from_bb_idx(n);
                }
            }
            debug_assert_eq!(i, pieces[c][0] as usize);
            i = N_W;
        }
        res.king_idx = idx % Self::num_king_squares();
        idx /= Self::num_king_squares();
        res.active = ChessColor::from_repr(idx % 2).unwrap();
        idx /= 2;
        // pawns give the index in the outer iteration
        for c in ChessColor::iter().rev() {
            let pawns: &mut [ChessSquare] = if c == Black { &mut res.b_nk } else { &mut res.w_nk };
            match if c == White { w_pawns } else { b_pawns } {
                0 => {}
                1 => {
                    let sq = idx % Self::num_pawn_squares();
                    let sq = if c == White { Self::num_pawn_squares() - 1 - sq + 8 } else { sq };
                    pawns[0] = ChessSquare::from_bb_idx(sq);
                    idx /= Self::num_pawn_squares();
                }
                count => {
                    let k = COMBINATIONS[Self::num_pawn_squares()][count];
                    combinadics::decode_mut(idx % k, count, &mut arr[0..count]);
                    if c == White {
                        for i in 0..count {
                            let bb_idx = Self::num_pawn_squares() - 1 - arr[i] + 8;
                            pawns[count - 1 - i] = ChessSquare::from_bb_idx(bb_idx);
                        }
                    } else {
                        for i in 0..count {
                            pawns[i] = ChessSquare::from_bb_idx(arr[i]);
                        }
                    }
                    idx /= k;
                }
            }
        }
        debug_assert_eq!(idx, 0);
        res.pieces = pieces;
        res
    }

    fn normalize(mut self, kings: [ChessSquare; NUM_COLORS]) -> Self {
        assert!(N_W >= N_B);
        let [mut w_king, mut b_king] = kings;
        if Self::symmetry() != GeneratePawnTable {
            // flipping horizontally is an `xor constant`, as is flipping vertically. So we can combine that into a single xor.
            let mut xor = 0;
            if w_king.file() >= 4 {
                xor = 0b111;
            }
            if !Self::has_pawn() && w_king.rank() >= 4 {
                xor ^= 0b111_000;
            }
            let xor = ChessSquare::from_bb_idx(xor);
            b_king ^= xor;
            w_king ^= xor;
            for w_x in &mut self.w_nk {
                *w_x ^= xor;
            }
            for b_x in &mut self.b_nk {
                *b_x ^= xor;
            }
            if !Self::has_pawn() && (w_king.rank(), b_king.rank()) > (w_king.file(), b_king.file()) {
                w_king = w_king.flip_diagonally();
                b_king = b_king.flip_diagonally();
                for w_x in &mut self.w_nk {
                    *w_x = w_x.flip_diagonally();
                }
                for b_x in &mut self.b_nk {
                    *b_x = b_x.flip_diagonally();
                }
            }
        }
        for c in ChessColor::iter() {
            let mut i = 0;
            let squares: &mut [ChessSquare] = if c == White { &mut self.w_nk } else { &mut self.b_nk };
            for count in self.pieces[c] {
                squares[i..i + count as usize].sort_by_key(|sq| sq.bb_idx());
                i += count as usize;
            }
        }
        self.king_idx = Self::kings_index([w_king, b_king]);
        self
    }

    fn size(pieces: PieceList) -> usize {
        Self::inner_size(pieces) * Self::outer_size(pieces)
    }

    fn outer_size(pieces: PieceList) -> usize {
        COMBINATIONS[Self::num_pawn_squares()][pieces[White][Pawn as usize] as usize]
            * COMBINATIONS[Self::num_pawn_squares()][pieces[Black][Pawn as usize] as usize]
    }

    fn inner_size(pieces: PieceList) -> usize {
        let mut res = 1;
        for c in ChessColor::iter().rev() {
            for &count in &pieces[c][1..] {
                let k = COMBINATIONS[64][count as usize];
                res *= k;
            }
        }
        res * Self::num_king_squares() * NUM_COLORS
    }

    fn outer_iter(
        pieces: PieceList,
    ) -> impl Iterator<
        Item = (
            impl ParallelIterator<Item = (usize, Self)> + Clone,
            impl ParallelIterator<Item = (usize, Self)> + Clone,
        ),
    > {
        let max = Self::size(pieces);
        let step = Self::inner_size(pieces);
        let inner_step = step / NUM_COLORS;
        (0..max).step_by(step).map(move |i| (Self::inner_iter(i, pieces), Self::inner_iter(i + inner_step, pieces)))
    }

    fn inner_iter(n: usize, pieces: PieceList) -> impl ParallelIterator<Item = (usize, Self)> + Clone {
        let step = Self::inner_size(pieces) / NUM_COLORS;
        (n..n + step).into_par_iter().map(move |i| (i, Self::from_idx(i, pieces)))
    }
}

impl<const N_W: usize, const N_B: usize, const SYMMETRY: usize> Default for PosIdx<N_W, N_B, SYMMETRY> {
    fn default() -> Self {
        Self {
            king_idx: 0,
            w_nk: [ChessSquare::default(); N_W],
            b_nk: [ChessSquare::default(); N_B],
            active: ChessColor::default(),
            pieces: PieceList::default(),
        }
    }
}

fn set_base_case(pos: UnverifiedChessboard, captured_or_promo: ChessBitboard) -> i8 {
    let Ok(pos) = pos.verify(Strict) else {
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

fn base_case_iter<const N_W: usize, const N_B: usize, const SYMMETRY: usize>(
    p: PosIdx<N_W, N_B, SYMMETRY>,
    nk_pieces: [&[ChessPieceType]; NUM_COLORS],
    active: ChessColor,
) -> i8 {
    assert_eq!(active, p.active, "{nk_pieces:?} {p:?}");
    let bbs = p.player_bbs();
    let captured: ChessBitboard = bbs[active] & bbs[!active];
    let kings = p.kings();
    if bbs[White].num_ones() != N_W + 1
        || bbs[Black].num_ones() != N_B + 1
        || captured.more_than_one_bit_set()
        || bbs[!active].has(kings[active])
        || (KINGS[kings[White]] | kings[White].bb()).has(kings[Black])
    {
        return INVALID;
    }
    let mut pos = Chessboard::empty();
    pos.set_active_player(active);
    pos.place_piece(kings[White], ColoredChessPieceType::new(White, King));
    pos.place_piece(kings[Black], ColoredChessPieceType::new(Black, King));
    for (i, &w_x) in p.w_nk.iter().enumerate() {
        if !(active == White && w_x.bb() == captured) {
            pos.place_piece(w_x, ColoredChessPieceType::new(White, nk_pieces[White][i]));
        }
    }
    for (i, &b_x) in p.b_nk.iter().enumerate() {
        if !(active == Black && b_x.bb() == captured) {
            pos.place_piece(b_x, ColoredChessPieceType::new(Black, nk_pieces[Black][i]));
        }
    }
    let promoted = pos.0.col_piece_bb(!active, Pawn) & ChessBitboard::rank(7 * (active as DimT));
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
            pos.place_piece(sq, ColoredChessPieceType::new(!active, promo));
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
fn base_case<const N_W: usize, const N_B: usize, const SYMMETRY: usize>(
    nk_pieces: [&[ChessPieceType]; NUM_COLORS],
    pieces: PieceList,
    table: &[Entry],
) {
    for (w_iter, b_iter) in PosIdx::<N_W, N_B, SYMMETRY>::outer_iter(pieces) {
        w_iter.for_each(|(i, p)| table[i].store(base_case_iter(p, nk_pieces, White), Relaxed));
        b_iter.for_each(|(i, p)| table[i].store(base_case_iter(p, nk_pieces, Black), Relaxed));
    }
}

fn value_after<const N_W: usize, const N_B: usize, const SYMMETRY: usize>(
    p: PosIdx<N_W, N_B, SYMMETRY>,
    piece_i: usize,
    dest: ChessSquare,
    table: &[Entry],
    nk_pieces: [&[ChessPieceType]; NUM_COLORS],
) -> i8 {
    let nk_pieces = nk_pieces[p.active];
    let mut new_p = PosIdx { active: !p.active, ..p };
    let pieces: &mut [ChessSquare] = if p.active == White { &mut new_p.w_nk } else { &mut new_p.b_nk };
    pieces[piece_i] = dest;
    // ensure piece squares are sorted in ascending order for the same colored piece
    let mut i = piece_i;
    while i > 0 && nk_pieces[i - 1] == nk_pieces[i] && pieces[i].bb_idx() < pieces[i - 1].bb_idx() {
        pieces.swap(i, i - 1);
        i -= 1;
    }
    let mut i = piece_i;
    while i + 1 < nk_pieces.len() && nk_pieces[i + 1] == nk_pieces[i] && pieces[i].bb_idx() > pieces[i + 1].bb_idx() {
        pieces.swap(i, i + 1);
        i += 1;
    }
    let idx = new_p.idx();
    debug_assert_eq!(PosIdx::<N_W, N_B, SYMMETRY>::from_idx(idx, p.pieces), new_p);
    if nk_pieces[piece_i] == Pawn {
        match table[idx].load(Relaxed).cmp(&DRAW) {
            Ordering::Less => MATED,
            Ordering::Equal => DRAW,
            Ordering::Greater => -MATED,
        }
    } else {
        table[idx].load(Relaxed)
    }
}

fn step<const N_W: usize, const N_B: usize, const SYMMETRY: usize>(
    (p_i, p): (usize, PosIdx<N_W, N_B, SYMMETRY>),
    nk_pieces: [&[ChessPieceType]; NUM_COLORS],
    table: &[Entry],
    pcs: PieceList,
    active: ChessColor,
    iteration: isize,
) -> Option<i8> {
    assert!(p_i < PosIdx::<N_W, N_B, SYMMETRY>::size(pcs));
    assert_eq!(active, p.active, "{p_i} {iteration} {nk_pieces:?} {p:?}",);
    // if there are two pieces on the same square, the position has been handled in a base case
    let kings = p.kings();
    let mut sides = kings.map(|sq| sq.bb());
    for w_x in p.w_nk {
        sides[White] |= w_x.bb();
    }
    for b_x in p.b_nk {
        sides[Black] |= b_x.bb();
    }
    let blockers = sides[White] | sides[Black];
    if blockers.num_ones() != N_W + N_B + 2 {
        return None;
    }
    // because we're writing positions with monotonically increasing DTZ, any result we've written
    // will never change again
    if table[p_i].load(Relaxed) != DRAW {
        return None;
    }
    let pieces: [&[ChessSquare]; 2] = [&p.w_nk, &p.b_nk];
    let promo = (0..pcs[!active][0] as usize).any(|i| pieces[!active][i].is_backrank());
    if promo {
        return None;
    }

    // the best possible outcome, no point in searching additional moves if we reach this
    let best = MATED.max(MATED + iteration as i8 * 2 + active as i8 - 1);
    // no need to test for legality: If the move results in an illegal position, the resulting entry is INVALID and
    // will not influence the minimum. Therefore, we don't even need to construct a `Chessboard`,
    // we can simply use the attacks of the individual pieces

    let test_nonking_move = |piece_i: usize, x_piece: ChessSquare, x_dest: ChessSquare| {
        let mut res = value_after(p, piece_i, x_dest, table, nk_pieces);
        // handle ep, no need to test for legality.
        // fortunately, positions with an ep capture can't be base case positions
        if nk_pieces[active][piece_i] == Pawn && x_dest.rank().abs_diff(x_piece.rank()) == 2 {
            let mut pawn_bb = ChessBitboard::default();
            for i in 0..pcs[!active][0] as usize {
                pawn_bb |= pieces[!active][i].bb();
            }
            let possible_ep_pawns: ChessBitboard = (x_dest.bb().west() | x_dest.bb().east()) & pawn_bb;
            for pawn in possible_ep_pawns {
                // the position after our opponent captures en passant
                let mut new_p = PosIdx { active: !active, ..p };
                let mut ps: [&mut [ChessSquare]; 2] = [&mut new_p.w_nk, &mut new_p.b_nk];
                let dest = x_piece.pawn_advance_unchecked(active);
                ps[active][piece_i] = dest;
                let i = ps[!active].iter().position(|p| *p == pawn).unwrap();
                let ep_res = value_after(new_p, i, dest, table, nk_pieces);
                // `res` is from the active player's pov instead of the inactive player's
                let ep_res = match ep_res.cmp(&DRAW) {
                    Ordering::Less => -MATED,
                    Ordering::Equal => DRAW,
                    Ordering::Greater => MATED,
                };
                res = res.max(ep_res);
            }
        }
        debug_assert!(res >= best, "{res} {best} {iteration} {x_piece}{x_dest} {active} {p_i} {p:?} {kings:?}");
        debug_assert!(
            res >= DRAW || res < best + 2,
            "{res} {best} {iteration} {x_piece}{x_dest} {active} {p_i} {p:?} {kings:?}"
        );
        res
    };

    let test_king_move = |king_dest: ChessSquare| {
        // `let p = PosIdx { active: !active, ..p };` causes an internal compiler error
        let mut p = p;
        p.active = !p.active;
        let i = match active {
            White => p.idx_normalized([king_dest, kings[Black]]),
            Black => p.idx_normalized([kings[White], king_dest]),
        };
        debug_assert_eq!(PosIdx::<N_W, N_B, SYMMETRY>::from_idx(i, pcs).active, !active);
        table[i].load(Relaxed)
    };
    let filter = if iteration == 0 { !sides[active] } else { !blockers };
    let mut res = INVALID;

    // If a pawn move or capture wins, it's an immediate win that gets dealt with in iteration 0.
    // So in all later iterations, it makes sense to test them last, and only if the best result is worse than a draw
    for (piece_i, &x_piece) in pieces[active].iter().enumerate() {
        if nk_pieces[active][piece_i] == Pawn && iteration > 0 {
            continue;
        }
        let attacks = attacks_for(nk_pieces[active][piece_i], x_piece, blockers, p.active) & filter;
        for x_dest in attacks {
            let r = test_nonking_move(piece_i, x_piece, x_dest);
            if r <= best {
                return Some(-r - 1);
            }
            res = res.min(r);
        }
    }
    for king_dest in KINGS[kings[active]] & !KINGS[kings[!active]] & filter {
        res = res.min(test_king_move(king_dest));
        if res <= best {
            return Some(-res - 1);
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
    for (piece_i, &x_piece) in pieces[active].iter().enumerate() {
        let filter = if nk_pieces[active][piece_i] == Pawn { !sides[active] } else { sides[!active] };
        let attacks = attacks_for(nk_pieces[active][piece_i], x_piece, blockers, p.active) & filter;
        for x_dest in attacks {
            let r = test_nonking_move(piece_i, x_piece, x_dest);
            if r == DRAW {
                return None;
            }
            res = res.min(r);
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

// Fill out the remaining positions: For each possible position, look at all legal moves and choose the maximum possible result,
// where the order is INVALID < LOST < DRAW < WON until nothing changes anymore.
fn fixed_point_iteration<const N_W: usize, const N_B: usize, const SYMMETRY: usize>(
    nk_pieces: [&[ChessPieceType]; NUM_COLORS],
    pieces: PieceList,
    table: &[Entry],
) {
    assert_eq!(SYMMETRY == NoPawns as usize, [pieces[White][0], pieces[Black][0]] == [0, 0]);

    for (w_iter, b_iter) in PosIdx::<N_W, N_B, SYMMETRY>::outer_iter(pieces) {
        let mut iteration = 0;
        loop {
            let fold_op = |color: ChessColor| {
                move |changed, item: (usize, PosIdx<N_W, N_B, SYMMETRY>)| match step(
                    item, nk_pieces, table, pieces, color, iteration,
                ) {
                    None => changed,
                    Some(val) => {
                        table[item.0].store(val, Relaxed);
                        true
                    }
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
            iteration += 1;
        }
    }
}

/// Assumes there is no en passant square, assumes the position is already normalized
fn compact_idx(
    pieces: [ChessBitboard; NUM_CHESS_PIECES],
    colors: [ChessBitboard; NUM_COLORS],
    us: ChessColor,
) -> usize {
    const NUM_PAWN_SQUARES: usize = 64 - 2 * 8;
    // place the two kings first because we always have to assume there are 10 squares for the white king
    let kings = [pieces[King] & colors[White], pieces[King] & colors[Black]];
    let kings = kings.map(|bb| bb.to_square().unwrap());
    // todo: We can even use a knight of the active player to enumerate all 3 piece combinations
    // (doesn't work for sliders though because adding pieces can change which squares are attacked)
    let mut res = if pieces[Pawn].is_zero() {
        no_pawns::kings_idx(kings)
    } else {
        pawns::KING_INDICES[kings[White]][kings[Black]] as usize
    };
    let mut occupied = pieces[King];
    let mut num_free = NUM_PAWN_SQUARES;
    let mut encode_bb = |bb: ChessBitboard, mut mask_bb: ChessBitboard, num_free: &mut usize| {
        mask_bb &= !occupied;
        let mut r = 0;
        for (i, sq) in bb.ones().enumerate() {
            let mut idx = sq.bb_idx();
            let below = ChessBitboard::new((1 << idx) - 1);
            idx -= (below & !mask_bb).num_ones();
            r += COMBINATIONS[idx][i + 1];
        }
        debug_assert!(mask_bb.contains(bb));
        let k = bb.num_ones();
        res *= COMBINATIONS[*num_free][k];
        res += r;
        *num_free -= k;
        occupied |= bb;
    };
    let pawn_mask = !ChessBitboard::backranks();
    // place the pawns now because we will always have to assume there are 48 free squares to choose from,
    // even if we placed other pieces first
    encode_bb(pieces[Pawn] & colors[White], pawn_mask, &mut num_free);
    encode_bb(pieces[Pawn] & colors[Black], pawn_mask, &mut num_free);
    num_free += 16; // non-pawn pieces can also be placed on backranks
    // place all other pieces
    for c in ChessColor::iter() {
        for p_idx in 1..5 {
            encode_bb(pieces[p_idx] & colors[c], ChessBitboard::new(!0), &mut num_free);
        }
    }
    debug_assert_eq!(occupied, (colors[White] | colors[Black]));
    res *= 2;
    res += us as usize;
    res
}

/// Computes the size of a compact table, i.e. a value > the maximum return value of compact_idx
fn compact_size(pieces: PieceList) -> usize {
    const NUM_PAWN_SQUARES: usize = 64 - 2 * 8;
    let mut res = if pieces[White][Pawn as usize] + pieces[Black][Pawn as usize] == 0 {
        no_pawns::NUM_KING_SQUARES
    } else {
        pawns::NUM_KING_SQUARES
    };
    let mut num_free = NUM_PAWN_SQUARES;
    let mut encode_pieces = |k: u8, num_free: &mut usize| {
        res *= COMBINATIONS[*num_free][k as usize];
        *num_free -= k as usize;
    };
    // place the pawns now because we will always have to assume there are 48 free squares to choose from,
    // even if we placed other pieces first
    encode_pieces(pieces[White][Pawn as usize], &mut num_free);
    encode_pieces(pieces[Black][Pawn as usize], &mut num_free);
    num_free += 16; // non-pawn pieces can also be placed on backranks
    // place all other pieces
    for c in ChessColor::iter() {
        for p_idx in 1..5 {
            encode_pieces(pieces[c][p_idx], &mut num_free);
        }
    }
    res *= 2;
    res
}

fn postprocess<const N_W: usize, const N_B: usize, const SYMMETRY: usize>(
    table: &[Entry],
    pieces: PieceList,
) -> Vec<Entry> {
    let draws = AtomicUsize::new(0);
    let wins = AtomicUsize::new(0);
    let losses = AtomicUsize::new(0);
    let mut compressed = vec![];
    compressed.resize_with(compact_size(pieces), || Entry::new(INVALID)); // TODO: Smaller size
    for (w_iter, b_iter) in PosIdx::<N_W, N_B, SYMMETRY>::outer_iter(pieces) {
        let lambda = |i: usize, p: PosIdx<N_W, N_B, SYMMETRY>| {
            let val = table[i].load(Relaxed);
            let mut pieces = [ChessBitboard::default(); NUM_CHESS_PIECES];
            let mut colors = [ChessBitboard::default(); NUM_COLORS];
            for c in ChessColor::iter() {
                let non_king: &[ChessSquare] = if c == White { &p.w_nk } else { &p.b_nk };
                let mut i = 0;
                for piece in ChessPieceType::non_king_pieces() {
                    for _ in 0..p.pieces[c][piece as usize] {
                        let bb = non_king[i].bb();
                        pieces[piece] |= bb;
                        colors[c] |= bb;
                        i += 1;
                    }
                }
                debug_assert_eq!(i, non_king.len());
            }
            let kings = p.kings();
            pieces[King] |= kings[White].bb() | kings[Black].bb();
            colors[White] |= kings[White].bb();
            colors[Black] |= kings[Black].bb();
            if colors[White].intersects(colors[Black]) || pieces[Pawn].intersects(ChessBitboard::backranks()) {
                return;
            }
            if val == INVALID {
                return;
            } else if val == DRAW {
                _ = draws.fetch_add(1, Relaxed);
            } else if (val > DRAW) == (p.active == White) {
                _ = wins.fetch_add(1, Relaxed);
            } else {
                _ = losses.fetch_add(1, Relaxed);
            }
            let idx = compact_idx(pieces, colors, p.active);
            debug_assert_eq!(compressed[idx].load(Relaxed), INVALID, "{idx} {i} {p:?} {kings:?}");
            compressed[idx].store(val, Relaxed);
        };
        w_iter.for_each(|(i, p)| lambda(i, p));
        b_iter.for_each(|(i, p)| lambda(i, p));
    }
    // these values are after symmetry reduction, so they are somewhat arbitrary
    let wins = wins.load(Relaxed);
    let losses = losses.load(Relaxed);
    let draws = draws.load(Relaxed);
    let invalid = compressed.len() - draws - wins - losses;
    println!("White wins: {wins}\tDraws: {draws}\tBlack wins: {losses}\tInvalid: {invalid}");
    compressed
}

fn calc_table<const N_W: usize, const N_B: usize, const SYMMETRY: usize>(
    nk_pieces: [&[ChessPieceType]; NUM_COLORS],
    pieces: PieceList,
) -> Vec<Entry> {
    let start = Instant::now();
    assert!(nk_pieces.into_iter().flatten().all(|p| !matches!(p, King | Empty)));
    assert!(nk_pieces[White].len() == N_W && nk_pieces[Black].len() == N_B);
    assert!(piece_type_idx(nk_pieces[Black]) <= piece_type_idx(nk_pieces[White]));
    // By default, assume that the position is a draw. This means that we don't need to handle the 50mr rule explicitly
    let mut table = vec![];
    let n = PosIdx::<N_W, N_B, SYMMETRY>::size(pieces);
    table.resize_with(n, || Entry::new(DRAW));

    // TODO: Consider encoding some tables in a sparse representation:
    // If most positions are draws, we can instead store a list of idx,outcome pairs, which should usually fit into
    // around 6 bytes each, so it's useful if less than 1/6th of positions aren't draws. This doesn't have to be done at
    // table granularity, it can also be done for both sides, or pawn positions, so basically the outermost k loops.
    // Initially, entries can be stored sorted and looked up with binary search (eytzinger layout?), but a better option
    // would be a (perfect?) hash function.

    base_case::<N_W, N_B, SYMMETRY>(nk_pieces, pieces, &table);
    println!("Base case took {0:.3} seconds for {nk_pieces:?}, size {n}", start.elapsed().as_secs_f64());
    fixed_point_iteration::<N_W, N_B, SYMMETRY>(nk_pieces, pieces, &table);
    println!("Iterations finished after {0:.3} seconds for {nk_pieces:?}", start.elapsed().as_secs_f64());
    let res = postprocess::<N_W, N_B, SYMMETRY>(&table, pieces);
    println!("Compacted after {0:.3} seconds, compact size {1}", start.elapsed().as_secs_f64(), res.len());
    // res
    table // TODO: Return `res` instead. Or actually, write `res` to a file and return table?
}

// for each of the 10 colored non-king piece types [P,N,B,R,Q,p,n,b,r,q], counts how often it appears
type PieceList = [[u8; 5]; NUM_COLORS];

fn calc_tablebase(pieces: PieceList) -> Vec<Entry> {
    if pieces[White][0] > 0 || pieces[Black][0] > 0 {
        calc_tablebase_impl::<{ GeneratePawnTable as usize }>(pieces)
    } else {
        calc_tablebase_impl::<{ NoPawns as usize }>(pieces)
    }
}

// nk = non-king
fn piece_list_to_nk_pieces(list: PieceList) -> [ArrayVec<ChessPieceType, MAX_NON_K_PIECES>; NUM_COLORS] {
    const N: usize = MAX_NON_K_PIECES;
    let mut nk_pieces = [ArrayVec::<ChessPieceType, N>::new(), ArrayVec::<ChessPieceType, N>::new()];
    for c in ChessColor::iter() {
        for (i, &cnt) in list[c].iter().enumerate() {
            for _ in 0..cnt {
                nk_pieces[c].push(ChessPieceType::from_idx(i));
            }
        }
    }
    nk_pieces
}

fn calc_tablebase_impl<const SYMMETRY: usize>(pieces: PieceList) -> Vec<Entry> {
    assert!(SYMMETRY != CompactPawnTable as usize);
    let nk_pieces = piece_list_to_nk_pieces(pieces);
    let nk_pieces = [nk_pieces[White].as_slice(), nk_pieces[Black].as_slice()];
    match nk_pieces[Black].len() {
        0 => match nk_pieces[White].len() {
            0 => unreachable!("Only kings left; already a draw"),
            1 => calc_table::<1, 0, SYMMETRY>(nk_pieces, pieces),
            2 => calc_table::<2, 0, SYMMETRY>(nk_pieces, pieces),
            3 => calc_table::<3, 0, SYMMETRY>(nk_pieces, pieces),
            4 => calc_table::<4, 0, SYMMETRY>(nk_pieces, pieces),
            _ => unreachable!("Too many pieces"),
        },
        1 => match nk_pieces[White].len() {
            0 => unreachable!("#black pieces must be <= #white pieces"),
            1 => calc_table::<1, 1, SYMMETRY>(nk_pieces, pieces),
            2 => calc_table::<2, 1, SYMMETRY>(nk_pieces, pieces),
            3 => calc_table::<3, 1, SYMMETRY>(nk_pieces, pieces),
            _ => unreachable!("Too many pieces"),
        },
        2 => match nk_pieces[White].len() {
            0 | 1 => unreachable!("#black pieces must be <= #white pieces"),
            2 => calc_table::<2, 2, SYMMETRY>(nk_pieces, pieces),
            _ => unreachable!("Too many pieces"),
        },
        _ => unreachable!("Too many pieces"),
    }
}

fn idx_of(pos: &Chessboard) -> usize {
    // todo: support querying the compact table
    if pos.piece_bb(Pawn).has_any() {
        idx_of_impl::<{ GeneratePawnTable as usize }>(pos)
    } else {
        idx_of_impl::<{ NoPawns as usize }>(pos)
    }
}

fn idx_of_impl<const SYMMETRY: usize>(pos: &Chessboard) -> usize {
    assert!(SYMMETRY != CompactPawnTable as usize); // todo: support
    let num_white = pos.player_bb(White).num_ones() - 1;
    let num_black = pos.player_bb(Black).num_ones() - 1;
    match num_black {
        0 => match num_white {
            0 => unreachable!("Only kings left; already a draw"),
            1 => PosIdx::<1, 0, SYMMETRY>::from_chessboard(pos).idx(),
            2 => PosIdx::<2, 0, SYMMETRY>::from_chessboard(pos).idx(),
            3 => PosIdx::<3, 0, SYMMETRY>::from_chessboard(pos).idx(),
            4 => PosIdx::<4, 0, SYMMETRY>::from_chessboard(pos).idx(),
            _ => unreachable!("Too many pieces"),
        },
        1 => match num_white {
            0 => unreachable!("#black pieces must be <= #white pieces"),
            1 => PosIdx::<1, 1, SYMMETRY>::from_chessboard(pos).idx(),
            2 => PosIdx::<2, 1, SYMMETRY>::from_chessboard(pos).idx(),
            3 => PosIdx::<3, 1, SYMMETRY>::from_chessboard(pos).idx(),
            _ => unreachable!("Too many pieces"),
        },
        2 => match num_white {
            0 | 1 => unreachable!("#black pieces must be <= #white pieces"),
            2 => PosIdx::<2, 2, SYMMETRY>::from_chessboard(pos).idx(),
            _ => unreachable!("Too many pieces"),
        },
        _ => unreachable!("Too many pieces"),
    }
}

fn to_piece_list(pos: &Chessboard) -> (PieceList, bool) {
    let mut res = PieceList::default();
    let mut idx = [0, 0];
    for c in ChessColor::iter() {
        for p in ChessPieceType::non_king_pieces() {
            let n = pos.col_piece_bb(c, p).0.num_ones();
            res[c][p as usize] = n as u8;
            for _ in 0..n {
                idx[c] *= 6;
                idx[c] += p as usize + 1;
            }
        }
    }
    if idx[White] < idx[Black] {
        res.swap(0, 1);
        (res, true)
    } else {
        (res, false)
    }
}

type Tablebase = HashMap<PieceList, LazyLock<Vec<Entry>, Box<dyn Fn() -> Vec<Entry> + Send>>>;

// This also inserts invalid piece lists, but since they're LazyLocks that's fine - we won't attempt to access them
fn gen_piece_list(res: &mut Tablebase, list: PieceList, depth: usize) {
    if depth == 0 {
        return;
    }
    for c in ChessColor::iter() {
        for i in 0..5 {
            let mut list = list;
            list[c][i] += 1;
            let l = piece_list_to_nk_pieces(list);
            if piece_type_idx(&l[White]) >= piece_type_idx(&l[Black]) {
                _ = res.insert(list, LazyLock::new(Box::new(move || calc_tablebase(list))));
                gen_piece_list(res, list, depth - 1);
            }
        }
    }
}

static TB: LazyLock<Tablebase> = LazyLock::new(|| {
    let mut res = Tablebase::default();
    for i in 0..5 {
        let mut list = PieceList::default();
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
fn force_dtz_table(mut pieces: PieceList) -> &'static [Entry] {
    let p = piece_list_to_nk_pieces(pieces);
    let w_idx = piece_type_idx(&p[White]);
    let b_idx = piece_type_idx(&p[Black]);
    if w_idx < b_idx {
        pieces.swap(0, 1);
    }
    if w_idx == 0 {
        return &[];
    }
    for c in ChessColor::iter() {
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

fn probe_dtz(mut pos: Chessboard) -> i8 {
    if let Some(sq) = pos.ep_square {
        pos.ep_square = None;
        let pawn = sq.pawn_advance_unchecked(!pos.active);
        debug_assert!(pos.is_piece_on(pawn, ColoredChessPieceType::new(!pos.active, Pawn)));
        let ep_pawns = (pawn.bb().east() | pawn.bb().west()) & pos.col_piece_bb(pos.active, Pawn);
        let mut res = i8::MIN;
        for p in ep_pawns {
            let mut pos = pos;
            pos.piece_bbs[Pawn] ^= sq.bb() | p.bb() | pawn.bb();
            pos.color_bbs[pos.active] ^= sq.bb() | p.bb();
            pos.color_bbs[!pos.active] ^= pawn.bb();
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
        pos.color_bbs.swap(0, 1);
        pos.active = !pos.active;
        for bb in &mut pos.piece_bbs {
            *bb = bb.flip_up_down();
        }
        for bb in &mut pos.color_bbs {
            *bb = bb.flip_up_down();
        }
    }
    let Some(table) = TB.get(&list) else {
        panic!("No table for {pos}; too many pieces ({list:?})?");
    };
    let idx = idx_of(&pos);
    let res = table[idx].load(Relaxed);
    debug_assert_ne!(res, INVALID, "{idx} {list:?} {flipped} {0:?} -- {1:?}", pos.color_bbs, pos.piece_bbs);
    res
}

#[allow(unused)]
mod tests {
    #[allow(unused)]
    use super::*;
    use crate::games::chess::pieces::ChessPiece;
    use crate::games::chess::pieces::ColoredChessPieceType::{BlackKing, WhiteKing};
    #[allow(unused)]
    use crate::games::chess::squares::sq;
    #[allow(unused)]
    use crate::general::bitboards::chessboard::ChessBitboard;
    use crate::general::board::BoardHelpers;
    use rand::SeedableRng;
    use rand::distr::{Distribution, Uniform};
    use rand::rngs::StdRng;
    #[allow(unused)]
    use std::sync::LazyLock;

    fn piece_v_king_is_won(
        piece: ChessPieceType,
        our_piece: ChessSquare,
        our_king: ChessSquare,
        their_king: ChessSquare,
        stm: ChessColor,
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
    fn single_piece_test() {
        for w_k in ChessSquare::iter() {
            for b_k in ChessSquare::iter() {
                for p in ChessPieceType::non_king_pieces() {
                    for w_p in ChessSquare::iter() {
                        let mut pos = Chessboard::empty();
                        pos.place_piece(w_k, WhiteKing);
                        let Ok(_) = pos.try_place_piece(ChessPiece::new(BlackKing, b_k)) else { continue };
                        let Ok(_) = pos.try_place_piece(ChessPiece::new(ColoredChessPieceType::new(White, p), w_p))
                        else {
                            continue;
                        };
                        let Ok(pos) = pos.verify(Strict) else { continue };
                        _ = force_dtz_table(to_piece_list(&pos).0);
                        let dtz = probe_dtz(pos);
                        let won = piece_v_king_is_won(p, w_p, w_k, b_k, White);
                        assert!(dtz >= 0, "{dtz} {pos}, {0:?}", to_piece_list(&pos),);
                        assert_eq!(dtz > 0, won, "{dtz} {pos}");
                        assert!(dtz <= 100, "{dtz} {pos}");
                    }
                }
            }
        }
    }

    const NO_PAWNS: usize = NoPawns as usize;

    #[test]
    #[ignore]
    fn immediate_game_over_test() {
        let pieces = [[0, 1, 0, 0, 0], [0, 1, 0, 0, 0]];
        let table = force_dtz_table(pieces);

        for w_king in ChessSquare::iter() {
            if (KINGS[w_king] | w_king.bb()).has(sq("b3")) {
                continue;
            }
            let p = PosIdx::<1, 1, NO_PAWNS>::normalized(w_king, sq("b3"), [sq("b1")], [sq("c2")], White, pieces);
            let i = p.idx();
            let res = table[i].load(Relaxed);
            if w_king == sq("a1") {
                assert_eq!(res, MATED);
                let p2 = PosIdx { active: Black, ..p };
                assert_eq!(table[p2.idx_normalized([w_king, p.kings()[Black]])].load(Relaxed), INVALID);
            } else if ChessBitboard::new(0x7070702).has(w_king) {
                assert_eq!(res, INVALID);
            } else {
                assert_eq!(res, DRAW);
            }
        }
        let pieces = [[0, 0, 0, 1, 0], [0, 0, 1, 0, 0]];
        let table = force_dtz_table(pieces);
        let p2 = PosIdx::<1, 1, NO_PAWNS> {
            king_idx: PosIdx::<1, 1, NO_PAWNS>::kings_index([sq("a1"), sq("h6")]),
            w_nk: [sq("g8")],
            b_nk: [sq("f8")],
            active: Black,
            pieces,
        };
        let i = p2.idx_normalized([sq("h8"), p2.kings()[Black]]);
        let res = table[i].load(Relaxed);
        assert_eq!(res, DRAW);
    }

    #[test]
    #[ignore]
    fn game_over_in_one_test() {
        let pieces = [[0, 1, 0, 0, 0], [0, 1, 0, 0, 0]];
        let table = force_dtz_table(pieces);
        let p = PosIdx::<1, 1, NO_PAWNS>::normalized(sq("c2"), sq("a1"), [sq("c5")], [sq("a2")], White, pieces);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
        let p = PosIdx { king_idx: PosIdx::<1, 1, NO_PAWNS>::kings_index([p.kings()[White], sq("h1")]), ..p };
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), DRAW, "{i}");

        let pieces = [[0, 0, 0, 0, 1], [0, 0, 0, 1, 0]];
        let table = force_dtz_table(pieces);
        let p = PosIdx::<1, 1, NO_PAWNS>::normalized(sq("c1"), sq("b3"), [sq("g1")], [sq("c2")], White, pieces);
        let i = p.idx();
        assert_ne!(table[i].load(Relaxed), MATED, "{i}");
        let p = PosIdx::<1, 1, NO_PAWNS>::normalized(sq("b3"), sq("c1"), [sq("c2")], [sq("g1")], Black, pieces);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), MATED, "{i}");
        let p = PosIdx::<1, 1, NO_PAWNS> {
            king_idx: PosIdx::<1, 1, NO_PAWNS>::kings_index([sq("a1"), sq("c3")]),
            b_nk: [sq("c2")],
            w_nk: [sq("c1")],
            active: Black,
            pieces,
        };
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
        let p = PosIdx::<1, 1, NO_PAWNS>::normalized(sq("c3"), sq("a2"), [sq("c1")], [sq("c1")], Black, pieces);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), DRAW, "{i}");
        let p = PosIdx::<1, 1, NO_PAWNS>::normalized(sq("c3"), sq("a2"), [sq("d1")], [sq("c1")], White, pieces);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), DRAW, "{i}");
        let p = PosIdx { active: Black, ..p };
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), INVALID, "{i}");

        let p = PosIdx::<1, 1, NO_PAWNS>::normalized(sq("d4"), sq("h1"), [sq("g4")], [sq("g4")], Black, pieces);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), MATED, "{i}");

        let p = PosIdx::<1, 1, NO_PAWNS>::normalized(sq("g1"), sq("f3"), [sq("g2")], [sq("g8")], Black, pieces);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
        let p = PosIdx::<1, 1, NO_PAWNS>::normalized(sq("f3"), sq("g1"), [sq("g8")], [sq("g2")], White, pieces);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
    }

    #[test]
    #[ignore]
    fn queen_vs_rook_test() {
        let pieces = [[0, 0, 0, 0, 1], [0, 0, 0, 1, 0]];
        let table = force_dtz_table(pieces);
        let p = PosIdx::<1, 1, NO_PAWNS>::normalized(sq("f3"), sq("h1"), [sq("a2")], [sq("a2")], Black, pieces);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), MATED, "{i}"); // DTM, not an actual mate
        let p = PosIdx::<1, 1, NO_PAWNS>::normalized(sq("f3"), sq("g1"), [sq("g8")], [sq("g2")], White, pieces);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
        let p = PosIdx::<1, 1, NO_PAWNS>::normalized(sq("f3"), sq("h1"), [sq("g8")], [sq("a2")], White, pieces);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
        let p = PosIdx::<1, 1, NO_PAWNS>::normalized(sq("f3"), sq("f1"), [sq("g8")], [sq("a2")], White, pieces);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
        let p = PosIdx::<1, 1, NO_PAWNS>::normalized(sq("f3"), sq("h2"), [sq("g8")], [sq("a2")], White, pieces);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
        let p = PosIdx::<1, 1, NO_PAWNS>::normalized(sq("f3"), sq("g1"), [sq("g8")], [sq("a2")], Black, pieces);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), MATED + 2, "{i}");
        let p = PosIdx::<1, 1, NO_PAWNS>::normalized(sq("f3"), sq("g1"), [sq("h8")], [sq("a2")], White, pieces);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 3, "{i}");
        let p = PosIdx::<1, 1, NO_PAWNS>::normalized(sq("f3"), sq("g1"), [sq("h7")], [sq("a8")], White, pieces);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 3, "{i}"); // actual mate
    }

    fn test_consistency<const N_W: usize, const N_B: usize, const SYMMETRY: usize>(
        table: &[Entry],
        pieces: [&[ChessPieceType]; NUM_COLORS],
        list: PieceList,
    ) {
        assert!(piece_type_idx(pieces[White]) >= piece_type_idx(pieces[Black]));
        let seed = 42;
        let mut rng = StdRng::seed_from_u64(seed);
        let dist = Uniform::new(0, table.len()).unwrap();
        for _ in 0..5_000_000 {
            let idx = dist.sample(&mut rng);
            let res = table[idx].load(Relaxed);
            if res == INVALID {
                continue;
            }
            let p = PosIdx::<N_W, N_B, SYMMETRY>::from_idx(idx, list);
            let bbs = p.player_bbs();
            if bbs[White].intersects(bbs[Black]) {
                assert!((bbs[White] & bbs[Black]).is_single_piece());
                continue;
            }
            let mut pos = Chessboard::empty();
            pos.set_active_player(p.active);
            pos.place_piece(p.kings()[White], WhiteKing);
            pos.place_piece(p.kings()[Black], BlackKing);
            for (i, &pcs) in pieces[White].iter().enumerate() {
                pos.place_piece(p.w_nk[i], ColoredChessPieceType::new(White, pcs));
            }
            for (i, &pcs) in pieces[Black].iter().enumerate() {
                pos.place_piece(p.b_nk[i], ColoredChessPieceType::new(Black, pcs));
            }
            if pos.0.piece_bbs[Pawn].intersects(ChessBitboard::backranks()) {
                continue;
            }
            let pos = pos.verify(Strict).unwrap();
            // assert_eq!(idx_of(&pos), idx, "{idx} {pos}");
            assert_eq!(
                probe_dtz(pos),
                res,
                "{res} {idx} {0} {pos} {1:?}",
                idx_of(&pos),
                PosIdx::<N_W, N_B, SYMMETRY>::from_idx(idx_of(&pos), list)
            );
            let mut max = -120;
            let mut best = pos;
            for child in pos.children() {
                let mut child_res = probe_dtz(child);
                assert_ne!(child_res, INVALID, "{idx} {0} '{pos}' '{child}'", idx_of(&child));
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
    fn consistency_test() {
        let pieces = [[0, 0, 0, 0, 1], [0, 0, 0, 1, 0]];
        let table = force_dtz_table(pieces);
        test_consistency::<1, 1, NO_PAWNS>(table, [&[Queen], &[Rook]], pieces);
    }

    #[test]
    #[ignore]
    fn piece_vs_2pieces_test() {
        let list: PieceList = [[0, 1, 1, 0, 0], [0, 0, 0, 1, 0]];
        let table = force_dtz_table(list);
        let mut p =
            PosIdx::<2, 1, NO_PAWNS>::normalized(sq("b3"), sq("b1"), [sq("c3"), sq("b2")], [sq("h7")], Black, list);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), MATED, "{i}");
        p.active = White;
        p.w_nk[0] = sq("e2");
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
        let pos = Chessboard::from_fen("8/8/8/N7/8/r7/B7/k1K5 b - - 0 1", Strict).unwrap();
        assert_eq!(table[idx_of(&pos)].load(Relaxed), -MATED - 1);

        test_consistency::<2, 1, NO_PAWNS>(table, [&[Knight, Bishop], &[Rook]], list);
    }

    // todo: Also support querying the compact pawn table
    const PAWNS: usize = GeneratePawnTable as usize;
    #[test]
    fn pawn_vs_king_test() {
        let list: PieceList = [[1, 0, 0, 0, 0], [0, 0, 0, 0, 0]];
        let table = force_dtz_table(list);
        let p = PosIdx::<1, 0, PAWNS>::normalized(sq("a1"), sq("c1"), [sq("a7")], [], White, list);
        assert_eq!(table[p.idx()].load(Relaxed), -MATED - 1, "{0} {1} {p:?}", p.idx(), table[p.idx()].load(Relaxed));
        for (i, e) in table.iter().enumerate().rev() {
            let e = e.load(Relaxed);
            let p = PosIdx::<1, 0, PAWNS>::from_idx(i, list);
            if p.w_nk[0] == p.kings()[Black] {
                assert!([DRAW, INVALID].contains(&e));
                continue;
            }
            if p.w_nk[0].rank() == 0 {
                assert_eq!(e, INVALID);
            }
            if e == INVALID || p.w_nk[0].is_backrank() {
                continue;
            }
            let won = piece_v_king_is_won(Pawn, p.w_nk[0], p.kings()[White], p.kings()[Black], p.active);
            assert_eq!(e != 0, won, "{i} {e} {won} {p:?}");
            if e != 0 {
                assert_eq!(e > 0, p.active == White, "{i} {e} {won} {p:?}");
            }
        }
    }

    #[test]
    #[ignore]
    fn piece_vs_pawn_test() {
        let list: PieceList = [[0, 1, 0, 0, 0], [1, 0, 0, 0, 0]];
        let table = force_dtz_table(list);
        let pos = Chessboard::from_fen("8/8/8/8/8/1N6/p1K5/k7 b - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, MATED);
        let pos = Chessboard::from_fen("8/8/8/8/8/p7/2K5/k1N5 b - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, MATED + 1, "{0} {1:?}", idx_of(&pos), PosIdx::<1, 1, PAWNS>::from_chessboard(&pos));
        let pos = Chessboard::from_fen("k4N2/8/3K4/8/8/8/p7/8 w - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, MATED + 2);
        let pos = Chessboard::from_fen("k4N2/8/3K4/8/8/p7/8/8 b - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, -MATED - 1);
        let pos = Chessboard::from_fen("8/1K6/8/8/5k2/8/6p1/3N4 w - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, MATED + 8);
        let pos = Chessboard::from_fen("8/1K6/8/8/5k2/6p1/8/3N4 b - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, -MATED - 1);
        test_consistency::<1, 1, PAWNS>(table, [&[Knight], &[Pawn]], list);
    }

    #[test]
    #[ignore]
    fn pawn_vs_pawn_test() {
        let list: PieceList = [[1, 0, 0, 0, 0], [1, 0, 0, 0, 0]];
        let table = force_dtz_table(list);
        let pos = Chessboard::from_fen("8/8/8/8/5p2/8/6P1/5K1k w - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, DRAW); // would be won without ep
        let pos = Chessboard::from_fen("8/8/8/8/5pP1/8/8/5K1k b - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, MATED + 2); // no ep
        let pos = Chessboard::from_fen("8/8/8/8/5pP1/8/8/5K1k b - g3 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, -MATED - 1); // ep
        let pos = Chessboard::from_fen("8/8/8/8/K6p/8/6P1/7k w - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, MATED + 2, "{0}", idx_of(&pos));
        test_consistency::<1, 1, PAWNS>(table, [&[Pawn], &[Pawn]], list);
    }

    #[test]
    #[ignore]
    fn two_same_pieces_test() {
        let list: PieceList = [[0, 0, 2, 0, 0], [0, 0, 0, 0, 0]];
        let table = force_dtz_table(list);
        test_consistency::<2, 0, NO_PAWNS>(table, [&[Bishop, Bishop], &[]], list);
        let list: PieceList = [[2, 0, 0, 0, 0], [0, 0, 0, 0, 0]];
        let table = force_dtz_table(list);
        let pos = Chessboard::from_fen("8/1k2P3/8/8/8/8/1P6/1K6 w - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, -MATED - 1, "{0}", idx_of(&pos));
        test_consistency::<2, 0, PAWNS>(table, [&[Pawn, Pawn], &[]], list);
    }
}
