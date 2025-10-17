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
use crate::games::chess::{ChessColor, Chessboard, EDGE_SQUARES};
use crate::games::{Color, ColoredPieceType, NUM_COLORS, PieceType};
use crate::general::bitboards::chessboard::{ChessBitboard, KINGS, KNIGHTS};
use crate::general::bitboards::{Bitboard, RawBitboard};
use crate::general::board::Strictness::Relaxed;
use crate::general::board::{BitboardBoard, Board, UnverifiedBoard};
use crate::general::hq::ChessSliderGenerator;
use crate::general::squares::{RectangularCoordinates, SmallGridSquare};
use arrayvec::ArrayVec;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::LazyLock;
use std::time::Instant;

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

const NUM_KING_SYMMETRY_SQUARES: usize = 10;

// use the maximum value so that "negamax" never chooses it if there's a legal position
const INVALID: i8 = 127;
const MATED: i8 = -100;
const DRAW: i8 = 0;

fn attacks_for(wx_piece: ChessPieceType, w_x: ChessSquare, blockers: ChessBitboard) -> ChessBitboard {
    match wx_piece {
        Knight => KNIGHTS[w_x],
        Bishop => ChessSliderGenerator::new(blockers).bishop_attacks(w_x),
        Rook => ChessSliderGenerator::new(blockers).rook_attacks(w_x),
        Queen => ChessSliderGenerator::new(blockers).queen_attacks(w_x),
        _ => unreachable!(),
    }
}

pub(super) fn piece_vs_king_is_won(
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
        King => unreachable!(),
        Empty => unreachable!(),
    }
}

fn piece_type_idx(pieces: &[ChessPieceType]) -> usize {
    let mut res = 0;
    debug_assert!(pieces.is_sorted());
    for &p in pieces {
        debug_assert!(p < King);
        res *= 5; // PAWN, KNIGHT, BISHOP, ROOK, QUEEN
        res += p as usize;
    }
    res
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct PosIdx<const N_W: usize, const N_B: usize> {
    wk_idx: usize,
    b_king: ChessSquare,
    w_nk: [ChessSquare; N_W],
    b_nk: [ChessSquare; N_B],
    active: ChessColor,
}

impl<const N_W: usize, const N_B: usize> PosIdx<N_W, N_B> {
    // TODO: For a white king on the main diagonal, we can exploit more symmetry by fixing on which side of the main diagonal
    // the black king has to be. Also, if both pieces are the same we can exploit white/black symmetry

    fn from_chessboard(pos: &Chessboard) -> Self {
        let mut w_pieces = [SmallGridSquare::default(); N_W];
        let mut b_pieces = [SmallGridSquare::default(); N_B];
        let mut w_i = 0;
        let mut b_i = 0;
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
        Self::normalized(pos.king_sq(White), pos.king_sq(Black), w_pieces, b_pieces, pos.active)
    }

    fn normalized(
        wk: ChessSquare,
        bk: ChessSquare,
        w_xs: [ChessSquare; N_W],
        b_xs: [ChessSquare; N_B],
        active: ChessColor,
    ) -> Self {
        let res = Self { wk_idx: 0, b_king: bk, w_nk: w_xs, b_nk: b_xs, active };
        res.normalize(wk)
    }

    fn w_king(&self) -> ChessSquare {
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
        let mut res = self.active as usize;
        res = (res * NUM_KING_SYMMETRY_SQUARES + self.wk_idx) * 64 + self.b_king.bb_idx();
        for w_x in self.w_nk {
            res *= 64;
            res += w_x.bb_idx();
        }
        for b_x in self.b_nk {
            res *= 64;
            res += b_x.bb_idx();
        }
        debug_assert_eq!(*self, Self::from_usize(res));
        res
    }

    fn is_normalized(&self, w_king: ChessSquare) -> bool {
        w_king == KING_SQUARES_SYMMETRY[self.wk_idx]
            && w_king.file() < 4
            && w_king.rank() < 4
            && w_king.rank() <= w_king.file()
    }

    fn normalize(mut self, mut w_king: ChessSquare) -> Self {
        // flipping horizontally is an `xor constant`, as is flipping vertically. So we can combine that into a single xor.
        let mut xor = 0;
        if w_king.file() >= 4 {
            xor = 0b111;
        }
        if w_king.rank() >= 4 {
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
        if w_king.rank() > w_king.file() {
            w_king = w_king.flip_diagonally();
            self.b_king = self.b_king.flip_diagonally();
            for w_x in &mut self.w_nk {
                *w_x = w_x.flip_diagonally();
            }
            for b_x in &mut self.b_nk {
                *b_x = b_x.flip_diagonally();
            }
        }
        self.wk_idx = KING_SQUARES_SYMMETRY.iter().position(|&sq| sq == w_king).unwrap();
        debug_assert!(self.is_normalized(w_king));
        self
    }

    fn idx_normalized(self, w_king: ChessSquare) -> usize {
        self.normalize(w_king).idx()
    }

    fn from_usize(mut idx: usize) -> Self {
        let mut res = Self::default();
        for b_x in res.b_nk.iter_mut().rev() {
            *b_x = ChessSquare::from_bb_idx(idx % 64);
            idx /= 64;
        }
        for w_x in res.w_nk.iter_mut().rev() {
            *w_x = ChessSquare::from_bb_idx(idx % 64);
            idx /= 64;
        }
        res.b_king = ChessSquare::from_bb_idx(idx % 64);
        idx /= 64;
        res.wk_idx = idx % NUM_KING_SYMMETRY_SQUARES;
        idx /= NUM_KING_SYMMETRY_SQUARES;
        res.active = ChessColor::from_repr(idx).unwrap();
        res
    }

    fn size() -> usize {
        (1_usize << (6 * (N_W + N_B + 1) + 1)) * NUM_KING_SYMMETRY_SQUARES
    }

    fn iter() -> impl Iterator<Item = (usize, Self)> {
        let max = Self::size();
        (0..max).map(|i| (i, Self::from_usize(i)))
    }
}

impl<const N_W: usize, const N_B: usize> Default for PosIdx<N_W, N_B> {
    fn default() -> Self {
        Self {
            wk_idx: 0,
            b_king: ChessSquare::default(),
            w_nk: [ChessSquare::default(); N_W],
            b_nk: [ChessSquare::default(); N_B],
            active: ChessColor::default(),
        }
    }
}

// Base Case: Fill out all positions that are checkmated, a stalemate or where a piece got captured.
// The captured piece can only belong to the active player, as it must have gotten captured in the previous move.
fn base_case<const N_W: usize, const N_B: usize>(nk_pieces: [&[ChessPieceType]; NUM_COLORS], res: &mut [i8]) {
    for (i, p) in PosIdx::<N_W, N_B>::iter() {
        let kings = [p.w_king(), p.b_king];
        let active = p.active;
        let bbs = p.player_bbs();
        let captured: ChessBitboard = bbs[active] & bbs[!active];
        if bbs[White].num_ones() != N_W + 1
            || bbs[Black].num_ones() != N_B + 1
            || captured.more_than_one_bit_set()
            || bbs[!active].has(kings[active])
            || (KINGS[p.w_king()] | p.w_king().bb()).has(p.b_king)
        {
            res[i] = INVALID;
            continue;
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
        let Ok(pos) = pos.verify(Relaxed) else {
            res[i] = INVALID;
            continue;
        };
        if captured.has_any() {
            assert!(pos.occupied_bb().num_ones() == N_W + N_B + 1);
            if pos.occupied_bb().num_ones() == 2 {
                continue; // only kings left, insufficient material
            }
            let dtz = probe_dtz(pos);
            res[i] = match dtz.cmp(&0) {
                Ordering::Less => MATED,
                Ordering::Equal => DRAW,
                Ordering::Greater => -MATED,
            };
            continue;
        }
        if pos.is_checkmate_slow() {
            res[i] = MATED;
        }
        // no need to handle stalemate as the default is already DRAW
    }
}

// Fill out the remaining positions: For each possible position, look at all legal moves and choose the maximum possible result,
// where the order is INVALID < LOST < DRAW < WON until nothing changes anymore.
fn fixed_point_iteration<const N_W: usize, const N_B: usize>(
    nk_pieces: [&[ChessPieceType]; NUM_COLORS],
    res: &mut [i8],
) {
    // Whenever an entry changes, only a few, similar, positions are directly affected.
    // In particular, the opponent's kings can have come from at most one square away. So we only check these positions.
    // This table is indexed by the white king, and each bitboard is indexed by the black king
    // TODO: Can already initialize this in the base case loop
    // TODO: One such table per side to move
    let mut to_check = [!ChessBitboard::default(); NUM_KING_SYMMETRY_SQUARES];
    let mut iteration = 0;
    loop {
        iteration += 1;
        let mut check_next_round = [ChessBitboard::default(); NUM_KING_SYMMETRY_SQUARES];
        let mut i = 0;
        let size = PosIdx::<N_W, N_B>::size();
        while i + 1 < size {
            i += 1;
            let p = PosIdx::<N_W, N_B>::from_usize(i);
            if !to_check[p.wk_idx].has(p.b_king) {
                i += size / (NUM_KING_SYMMETRY_SQUARES * 64 * NUM_COLORS) - 1;
                continue;
            }
            // if there are two pieces on the same square, the position has been handled in a base case
            let mut blockers = p.w_king().bb() | p.b_king.bb();
            for w_x in p.w_nk {
                blockers |= w_x.bb();
            }
            for b_x in p.b_nk {
                blockers |= b_x.bb();
            }
            if blockers.num_ones() != N_W + N_B + 2 {
                continue;
            }
            let active = p.active;
            // because we're finding mates with monotonically increasing lengths, any result we've written
            // will never change again
            if res[i] != DRAW {
                continue;
            }
            let pieces: [&[ChessSquare]; 2] = [&p.w_nk, &p.b_nk];
            let kings = [p.w_king(), p.b_king];
            let mut r = INVALID;
            // no need to test for legality: If the move results in an illegal position, the resulting entry is INVALID and
            // will not influence the maximum. Therefore, we don't even need to construct a Chessboard,
            // we can simply use the attacks of the individual pieces

            for (i, &x_piece) in pieces[active].iter().enumerate() {
                for x_dest in attacks_for(nk_pieces[active][i], x_piece, blockers) {
                    let mut p = PosIdx { active: !active, ..p };
                    if active == White {
                        p.w_nk[i] = x_dest;
                    } else {
                        p.b_nk[i] = x_dest;
                    };
                    let i = p.idx();
                    debug_assert_eq!(PosIdx::<N_W, N_B>::from_usize(i).active, !active);
                    r = r.min(res[i]);
                }
            }
            for king_dest in KINGS[kings[active]] {
                let mut p = PosIdx { active: !active, ..p };
                let i = if active == White {
                    p.idx_normalized(king_dest)
                } else {
                    p.b_king = king_dest;
                    p.idx()
                };
                debug_assert_eq!(PosIdx::<N_W, N_B>::from_usize(i).active, !active);
                r = r.min(res[i]);
            }
            // if all moves lead to an invalid position, the game is a draw by stalemate
            // (we can't be in check because then we'd already be MATED)
            if r == INVALID || r == DRAW {
                continue;
            }
            res[i] = if r < 0 { -r - 1 } else { -r + 1 };
            debug_assert!(
                ((-MATED - iteration * 2)..(-MATED - iteration + 1)).contains(&res[i].abs()),
                "{iteration} {i} {0}",
                res[i]
            );
            // conservative approximation of positions that could lead to this position
            // TODO: Retest if this actually improves performance over a simple `changed: bool`
            if active == White {
                check_next_round[p.wk_idx] |= (KINGS[p.b_king] & !blockers) | p.b_king.bb();
            } else {
                check_next_round[p.wk_idx] |= p.b_king.bb();
                for wk in KINGS[p.w_king()] & !blockers {
                    let p = PosIdx::normalized(wk, p.b_king, p.w_nk, p.b_nk, active);
                    check_next_round[p.wk_idx] |= p.b_king.bb();
                }
            }
        }
        if check_next_round.iter().all(|bb| bb.is_zero()) {
            break;
        }
        to_check = check_next_round;
    }
}

fn calc_tablebase_no_pawns<const N_W: usize, const N_B: usize>(nk_pieces: [&[ChessPieceType]; NUM_COLORS]) -> Vec<i8> {
    let start = Instant::now();
    assert!(nk_pieces.into_iter().flatten().all(|p| !matches!(p, King | Empty)));
    assert!(nk_pieces[White].len() == N_W && nk_pieces[Black].len() == N_B);
    assert!(piece_type_idx(nk_pieces[White]) <= piece_type_idx(nk_pieces[Black]));
    // By default, assume that the position is a draw. This means that we don't need to handle the 50mr rule explicitly
    let mut res = vec![DRAW; PosIdx::<N_W, N_B>::size()];

    base_case::<N_W, N_B>(nk_pieces, &mut res);
    println!("Base case took {0:.3} seconds", start.elapsed().as_secs_f64());
    fixed_point_iteration::<N_W, N_B>(nk_pieces, &mut res);
    println!("Finished after {0:.3} seconds", start.elapsed().as_secs_f64());
    res
}

// for each of the 10 colored non-king piece types [P,N,B,R,Q,p,n,b,r,q], counts how often it appears
type PieceList = [[usize; 5]; 2];

fn calc_tablebase(pieces: PieceList) -> Vec<i8> {
    // nk = non-king
    const N: usize = MAX_NON_K_PIECES;
    let mut nk_pieces = [ArrayVec::<ChessPieceType, N>::new(), ArrayVec::<ChessPieceType, N>::new()];
    for c in ChessColor::iter() {
        for (i, &cnt) in pieces[c].iter().enumerate() {
            for _ in 0..cnt {
                nk_pieces[c].push(ChessPieceType::from_idx(i));
            }
        }
    }
    let nk_pieces = [nk_pieces[White].as_slice(), nk_pieces[Black].as_slice()];
    match nk_pieces[White].len() {
        0 => match nk_pieces[Black].len() {
            0 => unreachable!("Only kings left; already a draw"),
            1 => calc_tablebase_no_pawns::<0, 1>(nk_pieces),
            2 => calc_tablebase_no_pawns::<0, 2>(nk_pieces),
            3 => calc_tablebase_no_pawns::<0, 3>(nk_pieces),
            4 => calc_tablebase_no_pawns::<0, 4>(nk_pieces),
            _ => unreachable!("Too many pieces"),
        },
        1 => match nk_pieces[Black].len() {
            0 => unreachable!("#white pieces must be <= #black pieces"),
            1 => calc_tablebase_no_pawns::<1, 1>(nk_pieces),
            2 => calc_tablebase_no_pawns::<1, 2>(nk_pieces),
            3 => calc_tablebase_no_pawns::<1, 3>(nk_pieces),
            _ => unreachable!("Too many pieces"),
        },
        2 => match nk_pieces[Black].len() {
            0 | 1 => unreachable!("#white pieces must be <= #black pieces"),
            2 => calc_tablebase_no_pawns::<2, 2>(nk_pieces),
            _ => unreachable!("Too many pieces"),
        },
        _ => unreachable!("Too many pieces"),
    }
}

fn idx_of(pos: &Chessboard) -> usize {
    let num_white = pos.player_bb(White).num_ones() - 1;
    let num_black = pos.player_bb(Black).num_ones() - 1;
    match num_white {
        0 => match num_black {
            0 => unreachable!("Only kings left; already a draw"),
            1 => PosIdx::<0, 1>::from_chessboard(pos).idx(),
            2 => PosIdx::<0, 2>::from_chessboard(pos).idx(),
            3 => PosIdx::<0, 3>::from_chessboard(pos).idx(),
            4 => PosIdx::<0, 4>::from_chessboard(pos).idx(),
            _ => unreachable!("Too many pieces"),
        },
        1 => match num_black {
            0 => unreachable!("#white pieces must be <= #black pieces"),
            1 => PosIdx::<1, 1>::from_chessboard(pos).idx(),
            2 => PosIdx::<1, 2>::from_chessboard(pos).idx(),
            3 => PosIdx::<1, 3>::from_chessboard(pos).idx(),
            _ => unreachable!("Too many pieces"),
        },
        2 => match num_black {
            0 | 1 => unreachable!("#white pieces must be <= #black pieces"),
            2 => PosIdx::<2, 2>::from_chessboard(pos).idx(),
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
                idx[c] *= 5;
                idx[c] += p as usize;
            }
        }
    }
    if idx[White] > idx[Black] {
        res.swap(0, 1);
        (res, true)
    } else {
        (res, false)
    }
}

type Tablebase = HashMap<PieceList, LazyLock<Vec<i8>, Box<dyn Fn() -> Vec<i8> + Send>>>;

// This also inserts invalid piece lists, but since they're LazyLocks that's fine - we won't attempt to access them
fn gen_piece_list(res: &mut Tablebase, list: PieceList, depth: usize) {
    if depth == 0 {
        return;
    }
    for c in ChessColor::iter() {
        for i in 0..5 {
            let mut list = list;
            list[c][i] += 1;
            if list[0].iter().sum::<usize>() <= list[1].iter().sum::<usize>() {
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
        list[Black][i] = 1;
        _ = res.insert(list, LazyLock::new(Box::new(move || calc_tablebase(list))));
        gen_piece_list(&mut res, list, MAX_TB_MAN - 2 - 1);
    }
    res
});

fn probe_dtz(mut pos: Chessboard) -> i8 {
    let (list, flipped) = to_piece_list(&pos);
    if flipped {
        pos.color_bbs.swap(0, 1);
        pos.active = !pos.active;
        // don't call pos.flip_side_to_move because that does unnecessary work like computing threats;
        // we don't actually require a consistent position
    }
    let table = TB.get(&list).unwrap().deref();
    let idx = idx_of(&pos);
    table[idx]
}

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
    #[allow(unused)]
    use std::sync::LazyLock;

    /// positions with one non-king non-pawn piece per player
    #[allow(unused)]
    static ROOK_VS_QUEEN: LazyLock<&'static [i8]> =
        LazyLock::new(|| TB.get(&[[0, 0, 0, 1, 0], [0, 0, 0, 0, 1]]).unwrap());

    #[test]
    fn single_piece_test() {
        for w_k in ChessSquare::iter() {
            for b_k in ChessSquare::iter() {
                for p in ChessPieceType::non_king_pieces() {
                    if p == Pawn {
                        continue;
                    }; // TODO: Also support pawns
                    for w_p in ChessSquare::iter() {
                        let mut pos = Chessboard::empty();
                        pos.place_piece(w_k, WhiteKing);
                        let Ok(_) = pos.try_place_piece(ChessPiece::new(BlackKing, b_k)) else { continue };
                        let Ok(_) = pos.try_place_piece(ChessPiece::new(ColoredChessPieceType::new(White, p), w_p))
                        else {
                            continue;
                        };
                        let Ok(pos) = pos.verify(Relaxed) else { continue };
                        let dtz = probe_dtz(pos);
                        let won = piece_vs_king_is_won(p, w_p, w_k, b_k, White);
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
        let table = calc_tablebase_no_pawns::<1, 1>([&[Knight], &[Knight]]);

        for w_king in ChessSquare::iter() {
            let p = PosIdx::<1, 1>::normalized(w_king, sq("b3"), [sq("b1")], [sq("c2")], White);
            let i = p.idx();
            let res = table[i];
            if w_king == sq("a1") {
                assert_eq!(res, MATED);
                let p2 = PosIdx { active: Black, ..p };
                assert_eq!(table[p2.idx_normalized(w_king)], INVALID);
            } else if ChessBitboard::new(0x7070702).has(w_king) {
                assert_eq!(res, INVALID);
            } else {
                assert_eq!(res, DRAW);
            }
            let p2 = PosIdx { wk_idx: 0, b_king: w_king, w_nk: [sq("c2")], b_nk: [sq("b1")], active: Black };
            let j = p2.idx_normalized(sq("b3"));
            assert_eq!(table[j], res, "{j} {i} {w_king}");
        }
        let table = calc_tablebase_no_pawns::<1, 1>([&[Bishop], &[Rook]]);
        let p2 = PosIdx { w_nk: [sq("h7")], b_nk: [sq("h6")], wk_idx: 0, b_king: sq("f8"), active: White };
        let i = p2.idx_normalized(sq("h8"));
        let res = table[i];
        assert_eq!(res, DRAW);
    }

    #[test]
    #[ignore]
    fn game_over_in_one_test() {
        let table = calc_tablebase_no_pawns::<1, 1>([&[Knight], &[Knight]]);
        let p = PosIdx::normalized(sq("c2"), sq("a1"), [sq("c5")], [sq("a2")], White);
        let i = p.idx();
        assert_eq!(table[i], -MATED - 1, "{i}");
        let p = PosIdx { b_king: sq("h1"), ..p };
        let i = p.idx();
        assert_eq!(table[i], DRAW, "{i}");

        let table = &**ROOK_VS_QUEEN;
        let p = PosIdx::normalized(sq("c1"), sq("b3"), [sq("g1")], [sq("c2")], White);
        let i = p.idx();
        assert_eq!(table[i], MATED, "{i}");
        let p2 = PosIdx { active: Black, b_nk: [sq("e2")], ..p };
        let i = p2.idx();
        assert_eq!(table[i], -MATED - 1, "{i}");
        let p2 = PosIdx { b_king: sq("d3"), ..p2 };
        let i = p2.idx();
        assert_eq!(table[i], -MATED - 1, "{i}");
        let p = PosIdx { wk_idx: 0, b_king: sq("c3"), w_nk: [sq("c1")], b_nk: [sq("c2")], active: Black };
        let i = p.idx();
        assert_eq!(table[i], -MATED - 1, "{i}");
        let p = PosIdx::normalized(sq("a2"), sq("c3"), [sq("c1")], [sq("c1")], White);
        let i = p.idx();
        assert_eq!(table[i], DRAW, "{i}");
        let p = PosIdx::normalized(sq("a2"), sq("c3"), [sq("c1")], [sq("d1")], Black);
        let i = p.idx();
        assert_eq!(table[i], DRAW, "{i}");
        let p = PosIdx { active: White, ..p };
        let i = p.idx();
        assert_eq!(table[i], INVALID, "{i}");
        // not a mate, but another table, and we're using DTZ
        let p = PosIdx::normalized(sq("d4"), sq("h1"), [sq("g4")], [sq("g4")], Black);
        let i = p.idx();
        assert_eq!(table[i], MATED, "{i}");
        let p = PosIdx::normalized(sq("d4"), sq("h1"), [sq("e4")], [sq("g4")], White);
        let i = p.idx();
        let res = table[i];
        assert_eq!(res, -MATED - 1, "{i}");

        let p = PosIdx::normalized(sq("g1"), sq("f3"), [sq("g2")], [sq("g2")], White);
        let i = p.idx();
        assert_eq!(table[i], MATED, "{i}");
        let p = PosIdx::normalized(sq("g1"), sq("f3"), [sq("g2")], [sq("g8")], Black);
        let i = p.idx();
        assert_eq!(table[i], -MATED - 1, "{i}");
    }

    #[test]
    #[ignore]
    fn piece_vs_piece_test() {
        let table = &**ROOK_VS_QUEEN;
        let p = PosIdx::normalized(sq("h1"), sq("f3"), [sq("a2")], [sq("a2")], White);
        let i = p.idx();
        assert_eq!(table[i], MATED, "{i}"); // DTM, not an actual mate
        let p = PosIdx::normalized(sq("g1"), sq("f3"), [sq("g2")], [sq("g8")], Black);
        let i = p.idx();
        assert_eq!(table[i], -MATED - 1, "{i}");
        let p = PosIdx::normalized(sq("h1"), sq("f3"), [sq("a2")], [sq("g8")], Black);
        let i = p.idx();
        assert_eq!(table[i], -MATED - 1, "{i}");
        let p = PosIdx::normalized(sq("f1"), sq("f3"), [sq("a2")], [sq("g8")], Black);
        let i = p.idx();
        assert_eq!(table[i], -MATED - 1, "{i}");
        let p = PosIdx::normalized(sq("h2"), sq("f3"), [sq("a2")], [sq("g8")], Black);
        let i = p.idx();
        assert_eq!(table[i], -MATED - 1, "{i}");
        let p = PosIdx::normalized(sq("g1"), sq("f3"), [sq("a2")], [sq("g8")], White);
        let i = p.idx();
        assert_eq!(table[i], MATED + 2, "{i}");
        let p = PosIdx::normalized(sq("g1"), sq("f3"), [sq("a2")], [sq("h8")], Black);
        let i = p.idx();
        assert_eq!(table[i], -MATED - 3, "{i}");
        let p = PosIdx::normalized(sq("g1"), sq("f3"), [sq("a8")], [sq("h7")], Black);
        let i = p.idx();
        assert_eq!(table[i], -MATED - 3, "{i}"); // actual mate
    }

    fn test_consistency<const N_W: usize, const N_B: usize>(table: &[i8], pieces: [&[ChessPieceType]; 2]) {
        for idx in 0..table.len() {
            let res = table[idx];
            if res == INVALID {
                continue;
            }
            let p = PosIdx::<N_W, N_B>::from_usize(idx);
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
            let pos = pos.verify(Relaxed).unwrap();
            assert_eq!(idx_of(&pos), idx, "{idx} {pos}");
            assert_eq!(probe_dtz(pos), res, "{res} {pos}");
            let mut max = -120;
            for child in pos.children() {
                let mut child_res = probe_dtz(child);
                assert_ne!(child_res, INVALID);
                if child.occupied_bb().num_ones() != pos.occupied_bb().num_ones() {
                    child_res = match child_res.cmp(&0) {
                        Ordering::Less => MATED,
                        Ordering::Equal => 0,
                        Ordering::Greater => -MATED,
                    };
                }
                max = max.max(-child_res);
            }
            if max == -120 {
                assert!(res == 0 || res == MATED);
                continue;
            }
            if res == 0 {
                assert_eq!(max, 0, "{idx}: {max} {res} [{pos}]");
            } else if res < 0 {
                assert_eq!(max + 1, res, "{idx}: {max} {res} [{pos}]");
            } else {
                assert_eq!(max - 1, res, "{idx}: {max} {res} [{pos}]");
            }
        }
    }

    #[test]
    #[ignore]
    fn consistency_test() {
        let table = &**ROOK_VS_QUEEN;
        test_consistency::<1, 1>(table, [&[Rook], &[Queen]]);
    }

    #[test]
    #[ignore]
    fn piece_vs_2pieces_test() {
        let list: PieceList = [[0, 0, 0, 1, 0], [0, 1, 1, 0, 0]];
        let table = TB.get(&list).unwrap().as_slice();
        let mut p = PosIdx::normalized(sq("b1"), sq("b3"), [sq("h7")], [sq("c3"), sq("b2")], White);
        let i = p.idx();
        assert_eq!(table[i], MATED, "{i}");
        p.active = Black;
        p.b_nk[0] = sq("e2");
        let i = p.idx();
        assert_eq!(table[i], -MATED - 1, "{i}");
        let pos = Chessboard::from_fen("8/8/8/n7/8/R7/b7/K1k5 w - - 0 1", Relaxed).unwrap();
        assert_eq!(table[idx_of(&pos)], -MATED - 1);

        test_consistency::<1, 2>(&table, [&[Rook], &[Knight, Bishop]]);
    }
}
