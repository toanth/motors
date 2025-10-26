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
use crate::games::chess::pieces::{ChessPieceType, ColoredChessPieceType};
use crate::games::chess::squares::{A_FILE_NUM, B_FILE_NUM, C_FILE_NUM, ChessSquare, D_FILE_NUM};
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
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::atomic::AtomicI8;
use std::sync::atomic::Ordering::Relaxed;
use std::time::Instant;

type Entry = AtomicI8;

// generate up to 6 man tablebases
const MAX_TB_MAN: usize = 6;
const MAX_NON_K_PIECES: usize = MAX_TB_MAN - 2;

const KING_SQUARES_SYMMETRY: [ChessSquare; 10] = [
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

const KING_SQUARES_PAWN: [ChessSquare; 32] = {
    let mut res = [ChessSquare::from_bb_idx(0); 32];
    let mut i = 0;
    while i < 32 {
        let idx = (i / 4) * 8 + i % 4;
        res[i] = ChessSquare::from_bb_idx(idx);
        i += 1;
    }
    res
};

const NUM_KING_SYMMETRY_SQUARES: usize = 10;

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

pub(super) fn piece_v_king_is_won(
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct PosIdx<const N_W: usize, const N_B: usize, const PAWN: bool> {
    wk_idx: usize,
    b_king: ChessSquare,
    w_nk: [ChessSquare; N_W],
    b_nk: [ChessSquare; N_B],
    pawns: [usize; NUM_COLORS],
    active: ChessColor,
}

impl<const N_W: usize, const N_B: usize, const PAWN: bool> PosIdx<N_W, N_B, PAWN> {
    // TODO: For a white king on the main diagonal, we can exploit more symmetry by fixing on which side of the main diagonal
    // the black king has to be. Also, if both sides have the same piece types we can exploit white/black symmetry,
    // and if a side has the same piece type multiple times the order can also be ignored

    fn from_chessboard(pos: &Chessboard) -> Self {
        let mut w_pieces = [SmallGridSquare::default(); N_W];
        let mut b_pieces = [SmallGridSquare::default(); N_B];
        let mut w_i = 0;
        let mut b_i = 0;
        debug_assert_eq!(N_W, pos.player_bb(White).num_ones() - 1);
        debug_assert_eq!(N_B, pos.player_bb(Black).num_ones() - 1);
        debug_assert_ne!(PAWN, pos.piece_bb(Pawn).is_zero());
        for p in ChessPieceType::non_king_pieces() {
            for sq in pos.col_piece_bb(White, p) {
                w_pieces[w_i] = sq;
                w_i += 1;
            }
            for sq in pos.col_piece_bb(Black, p) {
                b_pieces[b_i] = sq;
                b_i += 1;
            }
        }
        let w_pawns = pos.col_piece_bb(White, Pawn).num_ones();
        let b_pawns = pos.col_piece_bb(Black, Pawn).num_ones();
        Self::normalized(pos.king_sq(White), pos.king_sq(Black), w_pieces, b_pieces, pos.active, w_pawns, b_pawns)
    }

    fn normalized(
        wk: ChessSquare,
        bk: ChessSquare,
        w_nk: [ChessSquare; N_W],
        b_nk: [ChessSquare; N_B],
        active: ChessColor,
        w_pawns: usize,
        b_pawns: usize,
    ) -> Self {
        let res = Self { wk_idx: 0, b_king: bk, w_nk, b_nk, active, pawns: [w_pawns, b_pawns] };
        res.normalize(wk)
    }

    const fn has_pawn() -> bool {
        PAWN
    }

    const fn num_wk_squares() -> usize {
        if Self::has_pawn() { 64 / 2 } else { NUM_KING_SYMMETRY_SQUARES }
    }

    fn w_king(&self) -> ChessSquare {
        if Self::has_pawn() {
            return KING_SQUARES_PAWN[self.wk_idx];
        }
        KING_SQUARES_SYMMETRY[self.wk_idx]
    }

    fn player_bbs(&self) -> [ChessBitboard; 2] {
        let mut res = [ChessBitboard::default(); 2];
        for w_x in self.w_nk {
            res[White] |= w_x.bb();
        }
        for b_x in self.b_nk {
            res[Black] |= b_x.bb();
        }
        res[White] |= self.w_king().bb();
        res[Black] |= self.b_king.bb();
        res
    }

    fn idx(&self) -> usize {
        // the active player has to be the outermost loop so that we don't try to write an entry before we've seen
        // all the positions that can reach this entry
        debug_assert!(self.wk_idx < Self::num_wk_squares());
        let mut res = 0;
        // hopefully, the compiler can unroll the following loops if PAWN is false.
        let w_pawns = if Self::has_pawn() { self.pawns[White] } else { 0 };
        let b_pawns = if Self::has_pawn() { self.pawns[Black] } else { 0 };
        // TODO: in the final compressed table, we can use 6 here, but for generating tables it's a lot more convenient to
        // have base case entries with a pawn on the opponent's back rank
        const PAWN_RANKS: usize = 8;
        const NUM_FILES: usize = 8;
        for wp in &self.w_nk[..w_pawns] {
            res *= PAWN_RANKS;
            res += PAWN_RANKS - 1 - wp.rank() as usize;
        }
        for bp in &self.b_nk[..b_pawns] {
            res *= PAWN_RANKS;
            res += bp.rank() as usize;
        }
        res *= NUM_COLORS;
        res += self.active as usize;
        for wp in &self.w_nk[..w_pawns] {
            res *= NUM_FILES;
            res += wp.file() as usize;
        }
        for bp in &self.b_nk[..b_pawns] {
            res *= NUM_FILES;
            res += bp.file() as usize;
        }
        res = (res * Self::num_wk_squares() + self.wk_idx) * 64 + self.b_king.bb_idx();
        for w_x in &self.w_nk[w_pawns..] {
            res *= 64;
            res += w_x.bb_idx();
        }
        for b_x in &self.b_nk[b_pawns..] {
            res *= 64;
            res += b_x.bb_idx();
        }
        debug_assert_eq!(*self, Self::from_usize(res, [w_pawns, b_pawns]), "{res} {w_pawns} {b_pawns}");
        res
    }

    fn normalize(mut self, mut w_king: ChessSquare) -> Self {
        assert!(N_W >= N_B);
        // flipping horizontally is an `xor constant`, as is flipping vertically. So we can combine that into a single xor.
        let mut xor = 0;
        if w_king.file() >= 4 {
            xor = 0b111;
        }
        if !Self::has_pawn() && w_king.rank() >= 4 {
            xor ^= 0b111_000;
        }
        let xor = ChessSquare::from_bb_idx(xor);
        self.b_king ^= xor;
        w_king ^= xor;
        for w_x in &mut self.w_nk {
            *w_x ^= xor;
        }
        for b_x in &mut self.b_nk {
            *b_x ^= xor;
        }
        if !Self::has_pawn() && w_king.rank() > w_king.file() {
            // if [w_king.rank(), self.b_king.rank()] > [w_king.file(), self.b_king.file()] {
            w_king = w_king.flip_diagonally();
            self.b_king = self.b_king.flip_diagonally();
            for w_x in &mut self.w_nk {
                *w_x = w_x.flip_diagonally();
            }
            for b_x in &mut self.b_nk {
                *b_x = b_x.flip_diagonally();
            }
        }
        self.wk_idx = if Self::has_pawn() {
            debug_assert!(w_king.bb_idx() % 8 < 4);
            (w_king.bb_idx() / 8) * 4 + w_king.bb_idx() % 8
        } else {
            KING_SQUARES_SYMMETRY.iter().position(|&sq| sq == w_king).unwrap()
        };
        self
    }

    fn idx_normalized(self, w_king: ChessSquare) -> usize {
        self.normalize(w_king).idx()
    }

    // TODO: If there are multiple pieces of the same type and color, this assigns different numbers based on which piece is where,
    // instead of treating them as indistinguishable
    fn from_usize(mut idx: usize, pawns: [usize; NUM_COLORS]) -> Self {
        assert!(N_W >= N_B);
        debug_assert_ne!(pawns[White] == 0 && pawns[Black] == 0, Self::has_pawn());
        // hopefully, this allows the compiler to optimize better
        let w_pawns = if Self::has_pawn() { pawns[White] } else { 0 };
        let b_pawns = if Self::has_pawn() { pawns[Black] } else { 0 };
        let mut res = Self::default();
        for b_x in res.b_nk[b_pawns..].iter_mut().rev() {
            *b_x = ChessSquare::from_bb_idx(idx % 64);
            idx /= 64;
        }
        for w_x in res.w_nk[w_pawns..].iter_mut().rev() {
            *w_x = ChessSquare::from_bb_idx(idx % 64);
            idx /= 64;
        }
        res.b_king = ChessSquare::from_bb_idx(idx % 64);
        idx /= 64;
        res.wk_idx = idx % Self::num_wk_squares();
        idx /= Self::num_wk_squares();
        if !Self::has_pawn() {
            res.active = ChessColor::from_repr(idx).unwrap();
        } else {
            // in all positions withing a single iteration of the outer loop, all pawns are on the same rank.
            // This is because the results for positions after pawn pushes need to be known for computing the current position.
            // We have to iterate over pawn ranks instead of over pawn squares because normalizing the position
            // can mirror the board horizontally, which changes pawn squares but not pawn ranks.
            const PAWN_ROWS: usize = 8; // TODO: Eventually, can set this to 7
            const NUM_FILES: usize = 8;
            for b_x in res.b_nk[..b_pawns].iter_mut().rev() {
                *b_x = ChessSquare::from_bb_idx(idx % NUM_FILES);
                idx /= NUM_FILES;
            }
            for w_x in res.w_nk[..w_pawns].iter_mut().rev() {
                *w_x = ChessSquare::from_bb_idx(idx % NUM_FILES);
                idx /= NUM_FILES;
            }
            res.active = ChessColor::from_repr(idx % 2).unwrap();
            idx /= 2;
            for b_x in res.b_nk[..b_pawns].iter_mut().rev() {
                *b_x = ChessSquare::from_bb_idx(b_x.bb_idx() + (idx % PAWN_ROWS) * NUM_FILES);
                idx /= PAWN_ROWS;
            }
            for w_x in res.w_nk[..w_pawns].iter_mut().rev() {
                *w_x = ChessSquare::from_bb_idx(w_x.bb_idx() + (PAWN_ROWS - 1 - idx % PAWN_ROWS) * NUM_FILES);
                idx /= PAWN_ROWS;
            }
        }
        res.pawns = [w_pawns, b_pawns];
        res
    }

    fn size() -> usize {
        (1_usize << (6 * (N_W + N_B + 1) + 1)) * Self::num_wk_squares()
    }

    fn outer_stepsize(pawns: [usize; NUM_COLORS]) -> usize {
        if PAWN { Self::size() / (1 << 3 * (pawns[White] + pawns[Black])) } else { Self::size() }
    }

    fn outer_iter(
        pawns: [usize; NUM_COLORS],
    ) -> impl Iterator<
        Item = (
            impl ParallelIterator<Item = (usize, Self)> + Clone,
            impl ParallelIterator<Item = (usize, Self)> + Clone,
        ),
    > {
        let max = Self::size();
        let step = Self::outer_stepsize(pawns);
        let inner_step = step / NUM_COLORS;
        (0..max).step_by(step).map(move |i| (Self::inner_iter(i, pawns), Self::inner_iter(i + inner_step, pawns)))
    }

    fn inner_iter(n: usize, pawns: [usize; NUM_COLORS]) -> impl ParallelIterator<Item = (usize, Self)> + Clone {
        let step = Self::outer_stepsize(pawns) / NUM_COLORS;
        (n..n + step).into_par_iter().map(move |i| (i, Self::from_usize(i, pawns)))
    }
}

impl<const N_W: usize, const N_B: usize, const PAWNS: bool> Default for PosIdx<N_W, N_B, PAWNS> {
    fn default() -> Self {
        Self {
            wk_idx: 0,
            b_king: ChessSquare::default(),
            w_nk: [ChessSquare::default(); N_W],
            b_nk: [ChessSquare::default(); N_B],
            active: ChessColor::default(),
            pawns: [0, 0],
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

fn base_case_iter<const N_W: usize, const N_B: usize, const PAWNS: bool>(
    p: PosIdx<N_W, N_B, PAWNS>,
    nk_pieces: [&[ChessPieceType]; NUM_COLORS],
    active: ChessColor,
) -> i8 {
    assert_eq!(active, p.active);
    let kings = [p.w_king(), p.b_king];
    let bbs = p.player_bbs();
    let captured: ChessBitboard = bbs[active] & bbs[!active];
    if bbs[White].num_ones() != N_W + 1
        || bbs[Black].num_ones() != N_B + 1
        || captured.more_than_one_bit_set()
        || bbs[!active].has(kings[active])
        || (KINGS[p.w_king()] | p.w_king().bb()).has(p.b_king)
    {
        return INVALID;
    }
    let mut pos = Chessboard::empty();
    pos.set_active_player(active);
    pos.place_piece(p.w_king(), ColoredChessPieceType::new(White, King));
    pos.place_piece(p.b_king, ColoredChessPieceType::new(Black, King));
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
fn base_case<const N_W: usize, const N_B: usize, const PAWNS: bool>(
    nk_pieces: [&[ChessPieceType]; NUM_COLORS],
    table: &[Entry],
    pawns: [usize; NUM_COLORS],
) {
    for (w_iter, b_iter) in PosIdx::<N_W, N_B, PAWNS>::outer_iter(pawns) {
        w_iter.for_each(|(i, p)| table[i].store(base_case_iter(p, nk_pieces, White), Relaxed));
        b_iter.for_each(|(i, p)| table[i].store(base_case_iter(p, nk_pieces, Black), Relaxed));
    }
}

fn value_after<const N_W: usize, const N_B: usize, const PAWN: bool>(
    p: PosIdx<N_W, N_B, PAWN>,
    piece_i: usize,
    dest: ChessSquare,
    table: &[Entry],
    pawns: [usize; NUM_COLORS],
    nk_pieces: [&[ChessPieceType]; NUM_COLORS],
) -> i8 {
    let mut new_p = PosIdx { active: !p.active, ..p };
    if p.active == White {
        new_p.w_nk[piece_i] = dest;
    } else {
        new_p.b_nk[piece_i] = dest;
    };
    let idx = new_p.idx();
    debug_assert_eq!(PosIdx::<N_W, N_B, PAWN>::from_usize(idx, pawns), new_p);
    if nk_pieces[p.active][piece_i] == Pawn {
        match table[idx].load(Relaxed).cmp(&DRAW) {
            Ordering::Less => MATED,
            Ordering::Equal => DRAW,
            Ordering::Greater => -MATED,
        }
    } else {
        table[idx].load(Relaxed)
    }
}

fn step<const N_W: usize, const N_B: usize, const PAWN: bool>(
    (p_i, p): (usize, PosIdx<N_W, N_B, PAWN>),
    nk_pieces: [&[ChessPieceType]; NUM_COLORS],
    table: &[Entry],
    pawns: [usize; NUM_COLORS],
    active: ChessColor,
    iteration: isize,
) -> bool {
    assert!(p_i < PosIdx::<N_W, N_B, PAWN>::size());
    assert_eq!(active, p.active);
    // if there are two pieces on the same square, the position has been handled in a base case
    let mut sides = [p.w_king().bb(), p.b_king.bb()];
    for w_x in p.w_nk {
        sides[White] |= w_x.bb();
    }
    for b_x in p.b_nk {
        sides[Black] |= b_x.bb();
    }
    let blockers = sides[White] | sides[Black];
    if blockers.num_ones() != N_W + N_B + 2 {
        return false;
    }
    // because we're writing positions with monotonically increasing DTZ, any result we've written
    // will never change again
    if table[p_i].load(Relaxed) != DRAW {
        return false;
    }
    let pieces: [&[ChessSquare]; 2] = [&p.w_nk, &p.b_nk];
    let kings = [p.w_king(), p.b_king];
    let promo = (0..pawns[!active]).any(|i| pieces[!active][i].is_backrank());
    if promo {
        return false;
    }

    let mut r = INVALID;
    // no need to test for legality: If the move results in an illegal position, the resulting entry is INVALID and
    // will not influence the minimum. Therefore, we don't even need to construct a `Chessboard`,
    // we can simply use the attacks of the individual pieces

    for (i, &x_piece) in pieces[active].iter().enumerate() {
        let attacks = attacks_for(nk_pieces[active][i], x_piece, blockers, p.active) & !sides[active];
        for x_dest in attacks {
            let mut res = value_after(p, i, x_dest, table, pawns, nk_pieces);
            // handle ep, no need to test for legality.
            // fortunately, positions with an ep capture can't be base case positions
            if nk_pieces[active][i] == Pawn && x_dest.rank().abs_diff(x_piece.rank()) == 2 {
                let mut pawn_bb = ChessBitboard::default();
                for i in 0..pawns[!active] {
                    pawn_bb |= pieces[!active][i].bb();
                }
                let possible_ep_pawns: ChessBitboard = (x_dest.bb().west() | x_dest.bb().east()) & pawn_bb;
                for pawn in possible_ep_pawns {
                    // the position after our opponent captures en passant
                    let mut new_p = PosIdx { active: !active, ..p };
                    let mut ps: [&mut [ChessSquare]; 2] = [&mut new_p.w_nk, &mut new_p.b_nk];
                    let dest = x_piece.pawn_advance_unchecked(active);
                    ps[active][i] = dest;
                    let i = ps[!active].iter().position(|p| *p == pawn).unwrap();
                    let ep_res = value_after(new_p, i, dest, table, pawns, nk_pieces);
                    // `res` is from the active player's pov instead of the inactive player's
                    let ep_res = match ep_res.cmp(&DRAW) {
                        Ordering::Less => -MATED,
                        Ordering::Equal => DRAW,
                        Ordering::Greater => MATED,
                    };
                    res = res.max(ep_res);
                }
            }
            r = r.min(res);
        }
    }
    for king_dest in KINGS[kings[active]] {
        let mut p = PosIdx { active: !active, ..p };
        let i = if active == White {
            p.idx_normalized(king_dest)
        } else {
            p.b_king = king_dest;
            p.idx_normalized(p.w_king())
        };
        debug_assert_eq!(PosIdx::<N_W, N_B, PAWN>::from_usize(i, pawns).active, !active);
        r = r.min(table[i].load(Relaxed));
    }
    // if all moves lead to an invalid position, the game is a draw by stalemate
    // (we can't be in check because then we'd already be MATED)
    if r == INVALID || r == DRAW {
        return false;
    }
    let res = if r < 0 { -r - 1 } else { -r + 1 };
    table[p_i].store(res, Relaxed);
    debug_assert!(
        ((-MATED as isize - iteration * 2)..=(-MATED as isize - (iteration - 2) * 2)).contains(&(res.abs() as isize)),
        "{iteration} {p_i} {res} {p:?}"
    );
    res != DRAW
}

// Fill out the remaining positions: For each possible position, look at all legal moves and choose the maximum possible result,
// where the order is INVALID < LOST < DRAW < WON until nothing changes anymore.
fn fixed_point_iteration<const N_W: usize, const N_B: usize, const PAWN: bool>(
    nk_pieces: [&[ChessPieceType]; NUM_COLORS],
    table: &[Entry],
    pawns: [usize; NUM_COLORS],
) {
    assert_ne!(PAWN, pawns == [0, 0]);

    for (w_iter, b_iter) in PosIdx::<N_W, N_B, PAWN>::outer_iter(pawns) {
        let mut iteration = 0;
        loop {
            iteration += 1;
            let mut changed = w_iter
                .clone()
                .fold(|| false, |changed, item| step(item, nk_pieces, table, pawns, White, iteration) || changed)
                .reduce(|| false, |a, b| a || b);
            changed |= b_iter
                .clone()
                .fold(|| false, |changed, item| step(item, nk_pieces, table, pawns, Black, iteration) || changed)
                .reduce(|| false, |a, b| a || b);
            if !changed {
                break;
            }
        }
    }
}

fn calc_table<const N_W: usize, const N_B: usize, const PAWNS: bool>(
    nk_pieces: [&[ChessPieceType]; NUM_COLORS],
    pawns: [usize; NUM_COLORS],
) -> Vec<Entry> {
    let start = Instant::now();
    assert!(nk_pieces.into_iter().flatten().all(|p| !matches!(p, King | Empty)));
    assert!(nk_pieces[White].len() == N_W && nk_pieces[Black].len() == N_B);
    assert!(piece_type_idx(nk_pieces[Black]) <= piece_type_idx(nk_pieces[White]));
    // By default, assume that the position is a draw. This means that we don't need to handle the 50mr rule explicitly
    let mut table = vec![];
    table.resize_with(PosIdx::<N_W, N_B, PAWNS>::size(), || Entry::new(DRAW));

    base_case::<N_W, N_B, PAWNS>(nk_pieces, &table, pawns);
    println!("Base case took {0:.3} seconds for {nk_pieces:?}", start.elapsed().as_secs_f64());
    fixed_point_iteration::<N_W, N_B, PAWNS>(nk_pieces, &table, pawns);
    println!("Finished after {0:.3} seconds for {nk_pieces:?}", start.elapsed().as_secs_f64());
    table
}

// for each of the 10 colored non-king piece types [P,N,B,R,Q,p,n,b,r,q], counts how often it appears
type PieceList = [[usize; 5]; 2];

fn calc_tablebase(pieces: PieceList) -> Vec<Entry> {
    if pieces[White][0] > 0 || pieces[Black][0] > 0 {
        calc_tablebase_impl::<true>(pieces)
    } else {
        calc_tablebase_impl::<false>(pieces)
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

fn calc_tablebase_impl<const PAWNS: bool>(pieces: PieceList) -> Vec<Entry> {
    let nk_pieces = piece_list_to_nk_pieces(pieces);
    let pawns = [pieces[White][Pawn as usize], pieces[Black][Pawn as usize]];
    let nk_pieces = [nk_pieces[White].as_slice(), nk_pieces[Black].as_slice()];
    match nk_pieces[Black].len() {
        0 => match nk_pieces[White].len() {
            0 => unreachable!("Only kings left; already a draw"),
            1 => calc_table::<1, 0, PAWNS>(nk_pieces, pawns),
            2 => calc_table::<2, 0, PAWNS>(nk_pieces, pawns),
            3 => calc_table::<3, 0, PAWNS>(nk_pieces, pawns),
            4 => calc_table::<4, 0, PAWNS>(nk_pieces, pawns),
            _ => unreachable!("Too many pieces"),
        },
        1 => match nk_pieces[White].len() {
            0 => unreachable!("#black pieces must be <= #white pieces"),
            1 => calc_table::<1, 1, PAWNS>(nk_pieces, pawns),
            2 => calc_table::<2, 1, PAWNS>(nk_pieces, pawns),
            3 => calc_table::<3, 1, PAWNS>(nk_pieces, pawns),
            _ => unreachable!("Too many pieces"),
        },
        2 => match nk_pieces[White].len() {
            0 | 1 => unreachable!("#black pieces must be <= #white pieces"),
            2 => calc_table::<2, 2, PAWNS>(nk_pieces, pawns),
            _ => unreachable!("Too many pieces"),
        },
        _ => unreachable!("Too many pieces"),
    }
}

fn idx_of(pos: &Chessboard) -> usize {
    if pos.piece_bb(Pawn).has_any() { idx_of_impl::<true>(pos) } else { idx_of_impl::<false>(pos) }
}

fn idx_of_impl<const PAWNS: bool>(pos: &Chessboard) -> usize {
    let num_white = pos.player_bb(White).num_ones() - 1;
    let num_black = pos.player_bb(Black).num_ones() - 1;
    match num_black {
        0 => match num_white {
            0 => unreachable!("Only kings left; already a draw"),
            1 => PosIdx::<1, 0, PAWNS>::from_chessboard(pos).idx(),
            2 => PosIdx::<2, 0, PAWNS>::from_chessboard(pos).idx(),
            3 => PosIdx::<3, 0, PAWNS>::from_chessboard(pos).idx(),
            4 => PosIdx::<4, 0, PAWNS>::from_chessboard(pos).idx(),
            _ => unreachable!("Too many pieces"),
        },
        1 => match num_white {
            0 => unreachable!("#black pieces must be <= #white pieces"),
            1 => PosIdx::<1, 1, PAWNS>::from_chessboard(pos).idx(),
            2 => PosIdx::<2, 1, PAWNS>::from_chessboard(pos).idx(),
            3 => PosIdx::<3, 1, PAWNS>::from_chessboard(pos).idx(),
            _ => unreachable!("Too many pieces"),
        },
        2 => match num_white {
            0 | 1 => unreachable!("#black pieces must be <= #white pieces"),
            2 => PosIdx::<2, 2, PAWNS>::from_chessboard(pos).idx(),
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
            res[c][p as usize] = n;
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
            if list[White].iter().sum::<usize>() >= list[Black].iter().sum::<usize>() {
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
    &LazyLock::force(res)
}

fn probe_dtz(mut pos: Chessboard) -> i8 {
    if let Some(sq) = pos.ep_square {
        pos.ep_square = None;
        let pawn = sq.pawn_advance_unchecked(!pos.active);
        debug_assert!(pos.is_piece_on(pawn, ColoredChessPieceType::new(!pos.active, Pawn)));
        let ep_pawns = (pawn.bb().east() | pawn.bb().west()) & pos.col_piece_bb(pos.active, Pawn);
        let mut res = i8::MIN;
        for p in ep_pawns {
            let mut pos = pos.clone();
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

mod tests {
    #[allow(unused)]
    use super::*;
    use crate::games::chess::pieces::ChessPiece;
    use crate::games::chess::pieces::ColoredChessPieceType::{BlackKing, WhiteKing};
    use crate::games::chess::squares::ChessboardSize;
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

    /// positions with one non-king non-pawn piece per player
    #[allow(unused)]
    static QUEEN_VS_ROOK: LazyLock<&'static [Entry]> =
        LazyLock::new(|| TB.get(&[[0, 0, 0, 0, 1], [0, 0, 0, 1, 0]]).unwrap());

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

    #[test]
    #[ignore]
    fn immediate_game_over_test() {
        let table = calc_table::<1, 1, false>([&[Knight], &[Knight]], [0, 0]);

        for w_king in ChessSquare::iter() {
            let p = PosIdx::<1, 1, false>::normalized(w_king, sq("b3"), [sq("b1")], [sq("c2")], White, 0, 0);
            let i = p.idx();
            let res = table[i].load(Relaxed);
            if w_king == sq("a1") {
                assert_eq!(res, MATED);
                let p2 = PosIdx { active: Black, ..p };
                assert_eq!(table[p2.idx_normalized(w_king)].load(Relaxed), INVALID);
            } else if ChessBitboard::new(0x7070702).has(w_king) {
                assert_eq!(res, INVALID);
            } else {
                assert_eq!(res, DRAW);
            }
            let p2 = PosIdx::<1, 1, false> {
                wk_idx: 0,
                b_king: w_king,
                w_nk: [sq("c2")],
                b_nk: [sq("b1")],
                active: Black,
                pawns: [0, 0],
            };
            let j = p2.idx_normalized(sq("b3"));
            assert_eq!(table[j].load(Relaxed), res, "{j} {i} {w_king}");
        }
        let table = calc_table::<1, 1, false>([&[Rook], &[Bishop]], [0, 0]);
        let p2 = PosIdx::<1, 1, false> {
            w_nk: [sq("h7")],
            b_nk: [sq("h6")],
            wk_idx: 0,
            b_king: sq("f8"),
            active: Black,
            pawns: [0, 0],
        };
        let i = p2.idx_normalized(sq("h8"));
        let res = table[i].load(Relaxed);
        assert_eq!(res, DRAW);
    }

    #[test]
    #[ignore]
    fn game_over_in_one_test() {
        let table = calc_table::<1, 1, false>([&[Knight], &[Knight]], [0, 0]);
        let p = PosIdx::<1, 1, false>::normalized(sq("c2"), sq("a1"), [sq("c5")], [sq("a2")], White, 0, 0);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
        let p = PosIdx { b_king: sq("h1"), ..p };
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), DRAW, "{i}");

        let table = &**QUEEN_VS_ROOK;
        let p = PosIdx::<1, 1, false>::normalized(sq("c1"), sq("b3"), [sq("g1")], [sq("c2")], White, 0, 0);
        let i = p.idx();
        assert_ne!(table[i].load(Relaxed), MATED, "{i}");
        let p = PosIdx::<1, 1, false>::normalized(sq("b3"), sq("c1"), [sq("c2")], [sq("g1")], Black, 0, 0);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), MATED, "{i}");
        let p = PosIdx::<1, 1, false> {
            wk_idx: 0,
            b_king: sq("c3"),
            b_nk: [sq("c2")],
            w_nk: [sq("c1")],
            active: Black,
            pawns: [0, 0],
        };
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
        let p = PosIdx::<1, 1, false>::normalized(sq("c3"), sq("a2"), [sq("c1")], [sq("c1")], Black, 0, 0);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), DRAW, "{i}");
        let p = PosIdx::<1, 1, false>::normalized(sq("c3"), sq("a2"), [sq("d1")], [sq("c1")], White, 0, 0);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), DRAW, "{i}");
        let p = PosIdx { active: Black, ..p };
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), INVALID, "{i}");

        let p = PosIdx::<1, 1, false>::normalized(sq("d4"), sq("h1"), [sq("g4")], [sq("g4")], Black, 0, 0);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), MATED, "{i}");

        let p = PosIdx::<1, 1, false>::normalized(sq("g1"), sq("f3"), [sq("g2")], [sq("g8")], Black, 0, 0);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
        let p = PosIdx::<1, 1, false>::normalized(sq("f3"), sq("g1"), [sq("g8")], [sq("g2")], White, 0, 0);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
    }

    #[test]
    #[ignore]
    fn piece_vs_piece_test() {
        let table = &**QUEEN_VS_ROOK;
        let p = PosIdx::<1, 1, false>::normalized(sq("f3"), sq("h1"), [sq("a2")], [sq("a2")], Black, 0, 0);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), MATED, "{i}"); // DTM, not an actual mate
        let p = PosIdx::<1, 1, false>::normalized(sq("f3"), sq("g1"), [sq("g8")], [sq("g2")], White, 0, 0);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
        let p = PosIdx::<1, 1, false>::normalized(sq("f3"), sq("h1"), [sq("g8")], [sq("a2")], White, 0, 0);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
        let p = PosIdx::<1, 1, false>::normalized(sq("f3"), sq("f1"), [sq("g8")], [sq("a2")], White, 0, 0);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
        let p = PosIdx::<1, 1, false>::normalized(sq("f3"), sq("h2"), [sq("g8")], [sq("a2")], White, 0, 0);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
        let p = PosIdx::<1, 1, false>::normalized(sq("f3"), sq("g1"), [sq("g8")], [sq("a2")], Black, 0, 0);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), MATED + 2, "{i}");
        let p = PosIdx::<1, 1, false>::normalized(sq("f3"), sq("g1"), [sq("h8")], [sq("a2")], White, 0, 0);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 3, "{i}");
        let p = PosIdx::<1, 1, false>::normalized(sq("f3"), sq("g1"), [sq("h7")], [sq("a8")], White, 0, 0);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 3, "{i}"); // actual mate
    }

    fn test_consistency<const N_W: usize, const N_B: usize, const PAWN: bool>(
        table: &[Entry],
        pieces: [&[ChessPieceType]; NUM_COLORS],
    ) {
        let w_pawns = pieces[White].iter().filter(|&&p| p == Pawn).count();
        let b_pawns = pieces[Black].iter().filter(|&&p| p == Pawn).count();
        assert!(piece_type_idx(pieces[White]) >= piece_type_idx(pieces[Black]));
        let seed = 42;
        let mut rng = StdRng::seed_from_u64(seed);
        let dist = Uniform::new(0, table.len()).unwrap();
        for _ in 0..1_000_000 {
            let idx = dist.sample(&mut rng);
            let res = table[idx].load(Relaxed);
            if res == INVALID {
                continue;
            }
            let p = PosIdx::<N_W, N_B, PAWN>::from_usize(idx, [w_pawns, b_pawns]);
            let bbs = p.player_bbs();
            if bbs[White].intersects(bbs[Black]) {
                assert!((bbs[White] & bbs[Black]).is_single_piece());
                continue;
            }
            let mut pos = Chessboard::empty();
            pos.set_active_player(p.active);
            pos.place_piece(p.w_king(), WhiteKing);
            pos.place_piece(p.b_king, BlackKing);
            for (i, &pcs) in pieces[White].iter().enumerate() {
                pos.place_piece(p.w_nk[i], ColoredChessPieceType::new(White, pcs));
            }
            for (i, &pcs) in pieces[Black].iter().enumerate() {
                pos.place_piece(p.b_nk[i], ColoredChessPieceType::new(Black, pcs));
            }
            if pos.0.piece_bbs[Pawn].intersects(ChessBitboard::backranks_for(ChessboardSize::default())) {
                continue;
            }
            let pos = pos.verify(Strict).unwrap();
            assert_eq!(idx_of(&pos), idx, "{idx} {pos}");
            assert_eq!(probe_dtz(pos), res, "{res} {pos}");
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
        let table = &**QUEEN_VS_ROOK;
        test_consistency::<1, 1, false>(table, [&[Queen], &[Rook]]);
    }

    #[test]
    #[ignore]
    fn piece_vs_2pieces_test() {
        let list: PieceList = [[0, 1, 1, 0, 0], [0, 0, 0, 1, 0]];
        let table = TB.get(&list).unwrap().as_slice();
        let mut p =
            PosIdx::<2, 1, false>::normalized(sq("b3"), sq("b1"), [sq("c3"), sq("b2")], [sq("h7")], Black, 0, 0);
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), MATED, "{i}");
        p.active = White;
        p.w_nk[0] = sq("e2");
        let i = p.idx();
        assert_eq!(table[i].load(Relaxed), -MATED - 1, "{i}");
        let pos = Chessboard::from_fen("8/8/8/N7/8/r7/B7/k1K5 b - - 0 1", Strict).unwrap();
        assert_eq!(table[idx_of(&pos)].load(Relaxed), -MATED - 1);

        test_consistency::<2, 1, false>(&table, [&[Knight, Bishop], &[Rook]]);
    }

    #[test]
    fn pawn_vs_king_test() {
        let list: PieceList = [[1, 0, 0, 0, 0], [0, 0, 0, 0, 0]];
        let table = TB.get(&list).unwrap().as_slice();
        let p = PosIdx::<1, 0, true>::normalized(sq("a1"), sq("c1"), [sq("a7")], [], White, 1, 0);
        assert_eq!(table[p.idx()].load(Relaxed), -MATED - 1, "{0} {1} {p:?}", p.idx(), table[p.idx()].load(Relaxed));
        for (i, e) in table.iter().enumerate().rev() {
            let e = e.load(Relaxed);
            let p = PosIdx::<1, 0, true>::from_usize(i, [1, 0]);
            if p.w_nk[0] == p.b_king {
                assert!([DRAW, INVALID].contains(&e));
                continue;
            }
            if p.w_nk[0].rank() == 0 {
                assert_eq!(e, INVALID);
            }
            if e == INVALID || p.w_nk[0].is_backrank() {
                continue;
            }
            let won = piece_v_king_is_won(Pawn, p.w_nk[0], p.w_king(), p.b_king, p.active);
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
        let table = TB.get(&list).unwrap().as_slice();
        let pos = Chessboard::from_fen("8/8/8/8/8/1N6/p1K5/k7 b - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, MATED);
        let pos = Chessboard::from_fen("8/8/8/8/8/p7/2K5/k1N5 b - - 0 1", Strict).unwrap();
        let res = probe_dtz(pos);
        assert_eq!(res, MATED + 1, "{0} {1:?}", idx_of(&pos), PosIdx::<1, 1, true>::from_chessboard(&pos));
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
        test_consistency::<1, 1, true>(table, [&[Knight], &[Pawn]]);
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
        test_consistency::<1, 1, true>(table, [&[Pawn], &[Pawn]]);
    }
}
