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
use crate::PlayerResult;
use crate::PlayerResult::{Draw, Lose, Win};
use crate::games::chess::ChessColor::{Black, White};
use crate::games::chess::pieces::ChessPieceType::Pawn;
use crate::games::chess::squares::{ChessSquare, ChessboardSize, NUM_SQUARES};
use crate::games::chess::{Chessboard, PAWN_CAPTURES};
use crate::games::{Coordinates, NUM_COLORS};
use crate::general::bitboards::chessboard::{ChessBitboard, KINGS};
use crate::general::bitboards::{Bitboard, RawBitboard};
use crate::general::board::BitboardBoard;
use crate::general::squares::{RectangularCoordinates, sup_distance};
use std::sync::LazyLock;

const NUM_COMPLETE_BITBOARDS: usize = NUM_SQUARES * NUM_SQUARES / 2;

// Remove all bitboards with indices corresponding to a white pawns on a backrank
const NUM_RELEVANT_BITBOARDS: usize = NUM_COMPLETE_BITBOARDS - NUM_SQUARES * 8 / 2 * 2;

const OFFSET: usize = (NUM_COMPLETE_BITBOARDS - NUM_RELEVANT_BITBOARDS) / 2;

fn idx_full(w_pawn: ChessSquare, w_king: ChessSquare) -> usize {
    debug_assert!(w_pawn.file() < 4);
    (w_pawn.rank() as usize * 4 + w_pawn.file() as usize) * 64 + w_king.bb_idx()
}

fn idx_compact(w_pawn: ChessSquare, w_king: ChessSquare) -> usize {
    idx_full(w_pawn, w_king) - OFFSET
}

/// Indexed by the position of white's pawn king, gives the positions of black's king that are won for white, as a bitboard
type FullBitbase = [[ChessBitboard; NUM_COMPLETE_BITBOARDS]; NUM_COLORS];

pub type CompactBitbase = [[ChessBitboard; NUM_RELEVANT_BITBOARDS]; NUM_COLORS];

// based on <https://github.com/kervinck/pfkpk>

fn calc_pawn_vs_king_impl() -> FullBitbase {
    // bitboard of squares where black's king can't end up after a move. This doesn't include the white pawn if capturing the pawn is legal
    let mut invalid: [ChessBitboard; NUM_COMPLETE_BITBOARDS + 4 * NUM_SQUARES] =
        [ChessBitboard::new(0); NUM_COMPLETE_BITBOARDS + 4 * NUM_SQUARES];
    for w_pawn in ChessSquare::iter() {
        if w_pawn.file() >= 4 {
            continue;
        }
        for w_king in ChessSquare::iter() {
            invalid[idx_full(w_pawn, w_king)] = PAWN_CAPTURES[White][w_pawn] | KINGS[w_king] | w_king.bb();
        }
    }
    let mut res = [[ChessBitboard::default(); NUM_COMPLETE_BITBOARDS]; NUM_COLORS];
    // The base case is a pawn on the eighth rank; this is won unless black can immediately capture it.
    // It's never a stalemate because we could promote to a rook.
    for w_pawn in 64 - 8..64 - 4 {
        let w_pawn = ChessSquare::from_bb_idx(w_pawn);
        for w_king in ChessSquare::iter() {
            let our_king_dist = sup_distance(w_pawn, w_king);
            let promo_safe = if our_king_dist == 1 { !ChessBitboard::default() } else { !KINGS[w_pawn] };
            res[Black][idx_full(w_pawn, w_king)] = promo_safe & !KINGS[w_king] & !w_king.bb() & !w_pawn.bb();
        }
    }

    for w_pawn in ChessSquare::iter().rev() {
        if w_pawn.is_backrank() || w_pawn.file() >= 4 {
            continue;
        }
        let mut changed;
        loop {
            for w_king in ChessSquare::iter() {
                assert_eq!(res[Black][idx_full(w_pawn, w_king)] & w_pawn.bb(), ChessBitboard::default());
                let mut won = ChessBitboard::default();
                for to in (KINGS[w_king] & !w_pawn.bb()).ones() {
                    assert_eq!(res[Black][idx_full(w_pawn, to)] & w_pawn.bb(), ChessBitboard::default());
                    won |= res[Black][idx_full(w_pawn, to)] & !KINGS[to];
                }
                let pawn_push = w_pawn.north_unchecked();
                if pawn_push != w_king {
                    won |= res[Black][idx_full(pawn_push, w_king)];
                    if w_pawn.rank() == 1 && pawn_push.north_unchecked() != w_king {
                        let double_push = pawn_push.north_unchecked();
                        won |= res[Black][idx_full(double_push, w_king)] & !pawn_push.bb();
                    }
                }
                let i = idx_full(w_pawn, w_king);
                res[White][i] = won & !w_pawn.bb() & !invalid[i];
                debug_assert_eq!(res[White][i] & invalid[i], ChessBitboard::default());
            }
            changed = false;
            if w_pawn.is_backrank() || w_pawn.file() >= 4 {
                continue;
            }
            for w_king in ChessSquare::iter() {
                let i = idx_full(w_pawn, w_king);
                let no_draw_wtm = res[White][i] | invalid[i];
                let draw_btm = (!no_draw_wtm).moore_exclusive();
                let has_moves_btm = (!invalid[i]).moore_exclusive();
                let white_win_btm = has_moves_btm & !draw_btm & !w_pawn.bb();
                changed |= res[Black][i] != white_win_btm;
                res[Black][i] = white_win_btm;
            }
            if !changed {
                break;
            }
        }
    }
    res
}

pub fn calc_pawn_vs_king() -> CompactBitbase {
    let r = calc_pawn_vs_king_impl();
    let mut res = [[ChessBitboard::default(); NUM_RELEVANT_BITBOARDS]; NUM_COLORS];
    res[White].clone_from_slice(&r[White][OFFSET..NUM_RELEVANT_BITBOARDS + OFFSET]);
    res[Black].clone_from_slice(&r[Black][OFFSET..NUM_RELEVANT_BITBOARDS + OFFSET]);
    res
}

pub static PAWN_V_KING_TABLE: LazyLock<CompactBitbase> = LazyLock::new(calc_pawn_vs_king);

impl Chessboard {
    #[inline]
    pub fn query_bitbase(&self, table: &CompactBitbase) -> Option<PlayerResult> {
        if self.occupied_bb().num_ones() != 3 {
            return None;
        }
        let Some(pawn) = self.piece_bb(Pawn).to_square() else { return None };
        let flip = self.col_piece_bb(White, Pawn).is_zero();
        let (w_p, w_k, b_k) = if flip {
            (pawn.flip(), self.king_square(Black).flip(), self.king_square(White).flip())
        } else {
            (pawn, self.king_square(White), self.king_square(Black))
        };
        Some(query_pawn_v_king(table, w_p, w_k, b_k, flip != (self.active == Black)))
    }
}

fn query_pawn_v_king(
    table: &CompactBitbase,
    mut w_p: ChessSquare,
    mut w_k: ChessSquare,
    mut b_k: ChessSquare,
    is_black: bool,
) -> PlayerResult {
    if w_p.file() >= 4 {
        w_p = w_p.flip_left_right(ChessboardSize::default());
        w_k = w_k.flip_left_right(ChessboardSize::default());
        b_k = b_k.flip_left_right(ChessboardSize::default());
    }
    let i = idx_compact(w_p, w_k);
    if is_black {
        let res = table[Black][i].is_bit_set(b_k);
        if res { Lose } else { Draw }
    } else {
        let res = table[White][i].is_bit_set(b_k);
        if res { Win } else { Draw }
    }
}

#[cfg(test)]
mod tests {
    use crate::PlayerResult::{Draw, Lose, Win};
    use crate::games::chess::ChessColor::{Black, White};
    use crate::games::chess::bitbase::{
        PAWN_V_KING_TABLE, calc_pawn_vs_king, calc_pawn_vs_king_impl, idx_compact, idx_full, query_pawn_v_king,
    };
    use crate::games::chess::squares::{ChessSquare, sq};
    use crate::games::chess::{ChessBitboardTrait, ChessColor, Chessboard, PAWN_CAPTURES};
    use crate::general::bitboards::Bitboard;
    use crate::general::bitboards::chessboard::KINGS;
    use crate::general::board::Strictness::Strict;
    use crate::general::board::{Board, BoardHelpers};
    use crate::general::squares::{RectangularCoordinates, sup_distance};

    #[test]
    fn consistency_test() {
        let bitbase = calc_pawn_vs_king_impl();
        for w_pawn in ChessSquare::iter() {
            if w_pawn.is_backrank() || w_pawn.file() >= 4 {
                continue;
            }
            for w_king in ChessSquare::iter() {
                for b_king in ChessSquare::iter() {
                    if sup_distance(w_king, b_king) <= 1
                        || w_king == w_pawn
                        || b_king == w_pawn
                        || PAWN_CAPTURES[White][w_pawn].is_bit_set(b_king)
                    {
                        continue;
                    }
                    let mut expected = false;
                    let is_black_loss = |w_p: ChessSquare, w_k: ChessSquare, b_k: ChessSquare| {
                        bitbase[Black][idx_full(w_p, w_k)].is_bit_set(b_k)
                    };
                    for to in (KINGS[w_king] & !KINGS[b_king] & !w_pawn.bb()).ones() {
                        expected |= is_black_loss(w_pawn, to, b_king);
                    }
                    if w_pawn.north_unchecked() != w_king {
                        expected |= is_black_loss(w_pawn.north_unchecked(), w_king, b_king);
                    }
                    if w_pawn.rank() == 1
                        && w_pawn.north_unchecked() != w_king
                        && w_pawn.north_unchecked() != b_king
                        && w_pawn.north_unchecked().north_unchecked() != w_king
                    {
                        expected |= is_black_loss(w_pawn.north_unchecked().north_unchecked(), w_king, b_king);
                    }
                    let actual = bitbase[White][idx_full(w_pawn, w_king)].is_bit_set(b_king);
                    assert_eq!(expected, actual, "{w_pawn} {w_king} {b_king}");
                }
            }
        }
    }

    #[derive(Debug)]
    struct Testcase {
        side: ChessColor,
        w_pawn: ChessSquare,
        w_king: ChessSquare,
        b_king: ChessSquare,
        won: bool,
    }

    // from <https://github.com/kervinck/pfkpk/blob/master/pfkpk.c>
    #[test]
    fn simple_test() {
        let testcases = [
            Testcase { side: White, w_king: sq("a1"), w_pawn: sq("a2"), b_king: sq("a8"), won: false },
            Testcase { side: White, w_king: sq("a1"), w_pawn: sq("a2"), b_king: sq("h8"), won: true },
            Testcase { side: Black, w_king: sq("a1"), w_pawn: sq("a2"), b_king: sq("a8"), won: false },
            Testcase { side: Black, w_king: sq("a1"), w_pawn: sq("a2"), b_king: sq("h8"), won: true },
            Testcase { side: Black, w_king: sq("a1"), w_pawn: sq("a2"), b_king: sq("g2"), won: false },
            Testcase { side: Black, w_king: sq("a1"), w_pawn: sq("a2"), b_king: sq("g1"), won: true },
            Testcase { side: White, w_king: sq("a5"), w_pawn: sq("a4"), b_king: sq("d4"), won: true },
            Testcase { side: Black, w_king: sq("a5"), w_pawn: sq("a4"), b_king: sq("d4"), won: false },
            Testcase { side: White, w_king: sq("a1"), w_pawn: sq("f4"), b_king: sq("a3"), won: true },
            Testcase { side: Black, w_king: sq("a1"), w_pawn: sq("f4"), b_king: sq("a3"), won: false },
            Testcase { side: Black, w_king: sq("a3"), w_pawn: sq("a4"), b_king: sq("f3"), won: true },
            Testcase { side: White, w_king: sq("h6"), w_pawn: sq("g6"), b_king: sq("g8"), won: true },
            Testcase { side: White, w_king: sq("h3"), w_pawn: sq("h2"), b_king: sq("b7"), won: true },
            Testcase { side: Black, w_king: sq("a5"), w_pawn: sq("a4"), b_king: sq("e6"), won: false },
            Testcase { side: Black, w_king: sq("f8"), w_pawn: sq("g6"), b_king: sq("h8"), won: false },
            Testcase { side: White, w_king: sq("f6"), w_pawn: sq("g5"), b_king: sq("g8"), won: true },
            Testcase { side: White, w_king: sq("d1"), w_pawn: sq("c3"), b_king: sq("f8"), won: true },
            Testcase { side: White, w_king: sq("d4"), w_pawn: sq("c4"), b_king: sq("e6"), won: true },
            Testcase { side: White, w_king: sq("c6"), w_pawn: sq("d6"), b_king: sq("d8"), won: true },
            Testcase { side: Black, w_king: sq("d6"), w_pawn: sq("e6"), b_king: sq("d8"), won: true },
            Testcase { side: White, w_king: sq("g6"), w_pawn: sq("g5"), b_king: sq("h8"), won: true },
            Testcase { side: Black, w_king: sq("g6"), w_pawn: sq("g5"), b_king: sq("h8"), won: true },
            Testcase { side: White, w_king: sq("e4"), w_pawn: sq("e3"), b_king: sq("e6"), won: false },
            Testcase { side: Black, w_king: sq("e4"), w_pawn: sq("e3"), b_king: sq("e6"), won: true },
            Testcase { side: Black, w_king: sq("h3"), w_pawn: sq("b2"), b_king: sq("h5"), won: true },
            Testcase { side: White, w_king: sq("g2"), w_pawn: sq("b2"), b_king: sq("g5"), won: true },
        ];
        let table = &PAWN_V_KING_TABLE;
        for test in testcases {
            let res = query_pawn_v_king(table, test.w_pawn, test.w_king, test.b_king, test.side == Black);
            assert_eq!(res != Draw, test.won, "{test:?}");
        }
    }

    #[test]
    fn count_test() {
        // numbers from <https://github.com/kervinck/pfkpk/blob/master/kpk.c#L171>,
        // apparently given by Steven J. Edwards (1996):
        #[rustfmt::skip]
        let mut counts = [
            163328 / 2, 168024 / 2, // legal positions per side
            124960 / 2, 97604 / 2, // white winning per side
        ];
        let bitbase = calc_pawn_vs_king();
        for w_pawn in ChessSquare::iter() {
            if w_pawn.is_backrank() || w_pawn.file() >= 4 {
                continue;
            }
            for w_king in ChessSquare::iter() {
                if w_king == w_pawn {
                    continue;
                }
                let i = idx_compact(w_pawn, w_king);
                for b_king in ChessSquare::iter() {
                    if w_pawn == b_king || sup_distance(w_king, b_king) <= 1 {
                        continue;
                    }
                    let black_in_check = w_pawn.bb().pawn_attacks(White).is_bit_set(b_king);

                    if !black_in_check {
                        counts[0] -= 1
                    }
                    counts[1] -= 1;
                    if !black_in_check && bitbase[White][i].is_bit_set(b_king) {
                        counts[2] -= 1
                    }
                    if bitbase[Black][i].is_bit_set(b_king) {
                        counts[3] -= 1
                    }
                }
            }
        }
        assert_eq!(counts, [0, 0, 0, 0]);
    }

    #[test]
    fn chess_test() {
        let table = calc_pawn_vs_king();
        let pos = Chessboard::from_fen("1K1k4/1P6/8/8/8/8/8/8 b - - 0 1", Strict).unwrap();
        assert_eq!(pos.query_bitbase(&table), Some(Lose));
        let pos = pos.make_nullmove().unwrap();
        assert_eq!(pos.query_bitbase(&table), Some(Win));
        let pos = Chessboard::from_fen("3k4/1P6/8/8/8/1K6/8/8 b - - 0 1", Strict).unwrap();
        assert_eq!(pos.query_bitbase(&table), Some(Draw));
        let pos = Chessboard::from_fen("3k4/1P6/8/8/8/1K6/8/7R b - - 0 1", Strict).unwrap();
        assert_eq!(pos.query_bitbase(&table), None);

        let pos = Chessboard::from_fen("3k4/8/8/8/8/1K6/6p1/8 b - - 0 1", Strict).unwrap();
        assert_eq!(pos.query_bitbase(&table), Some(Win));
        let pos = pos.make_nullmove().unwrap();
        assert_eq!(pos.query_bitbase(&table), Some(Lose));
    }
}
