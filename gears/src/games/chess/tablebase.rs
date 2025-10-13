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
use crate::games::chess::squares::{
    A_FILE_NUM, B_FILE_NUM, C_FILE_NUM, ChessSquare, ChessboardSize, D_FILE_NUM, NUM_SQUARES,
};
use crate::games::chess::{ChessColor, Chessboard, EDGE_SQUARES};
use crate::games::{Color, ColoredPieceType, Coordinates, NUM_COLORS};
use crate::general::bitboards::chessboard::{ChessBitboard, KINGS, KNIGHTS};
use crate::general::bitboards::{Bitboard, RawBitboard};
use crate::general::board::Strictness::Relaxed;
use crate::general::board::{Board, UnverifiedBoard};
use crate::general::hq::ChessSliderGenerator;
use crate::general::squares::RectangularCoordinates;

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

fn calc_tablebase(pieces: &[ColoredChessPieceType]) {
    let non_king = |p: &ColoredChessPieceType| !matches!(p.uncolor(), King | Empty);
    assert!(pieces.iter().all(non_king));
    todo!()
}

const NUM_XVX_ENTRIES: usize = NUM_SQUARES * NUM_SQUARES * NUM_SQUARES * NUM_KING_SYMMETRY_SQUARES * NUM_COLORS;

// use the maximum value so that "negamax" never chooses it if there's a legal position
const INVALID: i8 = 127;
const MATED: i8 = -100;
const DRAW: i8 = 0;

// TODO: For a white king on the main diagonal, we can exploit more symmetry by demanding on which side of the main diagonal
// the black king has to be. Also, if both pieces are the same we can exploit white/black symmetry
fn normalize(
    mut w_k: ChessSquare,
    mut b_k: ChessSquare,
    mut w_x: ChessSquare,
    mut b_x: ChessSquare,
) -> (ChessSquare, ChessSquare, ChessSquare, ChessSquare) {
    if w_k.file() >= 4 {
        w_k = w_k.flip_left_right(ChessboardSize::default());
        b_k = b_k.flip_left_right(ChessboardSize::default());
        w_x = w_x.flip_left_right(ChessboardSize::default());
        b_x = b_x.flip_left_right(ChessboardSize::default());
    }
    if w_k.rank() >= 4 {
        w_k = w_k.flip();
        b_k = b_k.flip();
        w_x = w_x.flip();
        b_x = b_x.flip();
    }
    if w_k.rank() > w_k.file() {
        w_k = w_k.flip_diagonally();
        b_k = b_k.flip_diagonally();
        w_x = w_x.flip_diagonally();
        b_x = b_x.flip_diagonally();
    }
    debug_assert!(w_k.file() < 4 && w_k.rank() < 4 && w_k.rank() <= w_k.file());
    (w_k, b_k, w_x, b_x)
}

fn idx(w_k: ChessSquare, b_k: ChessSquare, w_x: ChessSquare, b_x: ChessSquare, stm: ChessColor) -> usize {
    // (((w_k.bb_idx() * 64 + w_x.bb_idx()) * 64 + b_x.bb_idx()) * NUM_X_PIECES + wx_type as usize - 1) * NUM_X_PIECES + bx_type as usize - 1
    debug_assert!(w_k.file() < 4 && w_k.rank() < 4 && w_k.rank() <= w_k.file());
    let wk_idx = match w_k.rank() {
        0 => w_k.bb_idx(),
        1 => w_k.bb_idx() - 5,
        2 => w_k.bb_idx() - 5 - 6,
        3 => w_k.bb_idx() - 5 - 6 - 7,
        _ => unreachable!(),
    };
    // TODO: More cache-efficient indexing
    ((((w_x.bb_idx()) * 64 + b_x.bb_idx()) * 64 + b_k.bb_idx()) * NUM_KING_SYMMETRY_SQUARES + wk_idx) * NUM_COLORS
        + stm as usize
}

fn idx_normalized(w_k: ChessSquare, b_k: ChessSquare, w_x: ChessSquare, b_x: ChessSquare, stm: ChessColor) -> usize {
    let (w_k, b_k, w_x, b_x) = normalize(w_k, b_k, w_x, b_x);
    idx(w_k, b_k, w_x, b_x, stm)
}

fn attacks_for(wx_piece: ChessPieceType, w_x: ChessSquare, blockers: ChessBitboard) -> ChessBitboard {
    match wx_piece {
        Knight => KNIGHTS[w_x],
        Bishop => ChessSliderGenerator::new(blockers).bishop_attacks(w_x),
        Rook => ChessSliderGenerator::new(blockers).rook_attacks(w_x),
        Queen => ChessSliderGenerator::new(blockers).queen_attacks(w_x),
        _ => unreachable!(),
    }
}

// TODO: Test
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
            if (KINGS[their_king] & !KINGS[our_king] & our_piece.bb()).has_set_bit() {
                return false;
            }
            // stalemate
            if (their_king.bb() & EDGE_SQUARES).has_set_bit() && (KINGS[their_king] & KINGS[our_king]).has_set_bit() {
                let blockers = our_king.bb() | their_king.bb();
                let slider_gen = ChessSliderGenerator::new(blockers);
                let attacks = if piece == Rook {
                    slider_gen.rook_attacks(our_piece)
                } else {
                    slider_gen.queen_attacks(our_piece)
                };
                return (KINGS[their_king] & !(attacks | KINGS[our_king])).has_set_bit()
                    || attacks.is_bit_set(their_king);
            }
            true
        }
        King => unreachable!(),
        Empty => unreachable!(),
    }
}

fn calc_tablebase_x_vs_x(x_pieces: [ChessPieceType; NUM_COLORS]) -> Vec<i8> {
    assert!(x_pieces[White] as usize <= x_pieces[Black] as usize);
    // by default, assume that the position is a draw. This means that we don't need to handle the 50mr rule explicitly
    let mut res = vec![DRAW; NUM_XVX_ENTRIES];
    // Base Case: Fill out all positions that are checkmated, a stalemate or where a piece got captured.
    // The captured piece can only belong to the active player, as it must have gotten captured in the previous move.
    for w_king in KING_SQUARES_SYMMETRY {
        for b_king in ChessSquare::iter() {
            for w_x in ChessSquare::iter() {
                for b_x in ChessSquare::iter() {
                    for active in ChessColor::iter() {
                        let pieces = [w_x, b_x];
                        let kings = [w_king, b_king];
                        let i = idx(w_king, b_king, w_x, b_x, active);
                        if kings[active] == pieces[!active]
                            || w_king == w_x
                            || b_king == b_x
                            || (KINGS[w_king] | w_king.bb()).is_bit_set(b_king)
                        {
                            res[i] = INVALID;
                            continue;
                        }
                        let mut pos = Chessboard::empty();
                        if [kings[!active], pieces[!active]].contains(&pieces[active]) {
                            // the inactive player captured the now active player's piece in the previous turn
                            res[i] = if piece_vs_king_is_won(
                                x_pieces[!active],
                                pieces[!active],
                                kings[!active],
                                kings[active],
                                !active,
                            ) {
                                MATED
                            } else {
                                DRAW
                            };
                            continue;
                        } else {
                            pos.place_piece(pieces[active], ColoredChessPieceType::new(active, x_pieces[active]));
                        }
                        pos.place_piece(pieces[!active], ColoredChessPieceType::new(!active, x_pieces[!active]));
                        pos.place_piece(w_king, ColoredChessPieceType::new(White, King));
                        pos.place_piece(b_king, ColoredChessPieceType::new(Black, King));
                        pos.set_active_player(active);
                        let Ok(pos) = pos.verify(Relaxed) else {
                            res[i] = INVALID;
                            continue;
                        };
                        if pos.has_no_legal_moves() {
                            res[i] = if pos.is_in_check() { MATED } else { DRAW }
                        }
                    }
                }
            }
        }
    }
    // Fill out the remaining positions: For each possible position, look at all legal moves and choose the maximum possible result,
    // where the order is INVALID < LOST < DRAW < WON until nothing changes anymore.
    loop {
        let mut changed = false;
        for w_king in KING_SQUARES_SYMMETRY {
            for b_king in ChessSquare::iter() {
                for w_x in ChessSquare::iter() {
                    if [w_king, b_king].contains(&w_x) {
                        continue;
                    }
                    for b_x in ChessSquare::iter() {
                        if [w_king, b_king, w_x].contains(&b_x) {
                            continue;
                        }
                        for active in ChessColor::iter() {
                            let i = idx(w_king, b_king, w_x, b_x, active);
                            // because we're finding mates with monotonically increasing lengths, any result we've written
                            // will never change again
                            if res[i] != DRAW {
                                continue;
                            }
                            let pieces = [w_x, b_x];
                            let kings = [w_king, b_king];
                            let blockers = w_king.bb() | b_king.bb() | w_x.bb() | b_x.bb();
                            let mut r = INVALID;
                            // no need to test for legality: If the move results in an illegal position, the resulting entry is INVALID and
                            // will not influence the maximum. Therefore, we don't even need to construct a Chessboard,
                            // we can simply use the attacks of the individual pieces

                            for x_dest in attacks_for(x_pieces[active], pieces[active], blockers).ones() {
                                let i = if active == White {
                                    idx(w_king, b_king, x_dest, b_x, !active)
                                } else {
                                    idx(w_king, b_king, w_x, x_dest, !active)
                                };
                                r = r.min(res[i]);
                            }
                            for king_dest in KINGS[kings[active]].ones() {
                                let i = if active == White {
                                    idx_normalized(king_dest, b_king, w_x, b_x, !active)
                                } else {
                                    idx(w_king, king_dest, w_x, b_x, !active)
                                };
                                r = r.min(res[i]);
                            }
                            // if all moves lead to an invalid position, the game is a draw by stalemate
                            // (we can't be in check because then we'd already be MATED)
                            if r != INVALID && r != DRAW {
                                res[i] = if r < 0 { -r - 1 } else { -r + 1 };
                                changed = true;
                            }
                        }
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    res
}

mod tests {
    #[allow(unused)]
    use super::*;
    #[allow(unused)]
    use crate::games::chess::squares::sq;
    #[allow(unused)]
    use crate::general::bitboards::chessboard::ChessBitboard;
    #[allow(unused)]
    use std::sync::LazyLock;
    /// positions with one non-king non-pawn piece per player
    #[allow(unused)]
    static ROOK_VS_QUEEN: LazyLock<Vec<i8>> = LazyLock::new(|| calc_tablebase_x_vs_x([Rook, Queen]));

    #[test]
    #[ignore]
    fn immediate_game_over_test() {
        let pieces = [Knight, Knight];
        let table = calc_tablebase_x_vs_x(pieces);

        for w_king in ChessSquare::iter() {
            let i = idx_normalized(w_king, sq("b3"), sq("b1"), sq("c2"), White);
            if KING_SQUARES_SYMMETRY.contains(&w_king) {
                assert_eq!(i, idx(w_king, sq("b3"), sq("b1"), sq("c2"), White));
            }
            let res = table[i];
            if w_king == sq("a1") {
                assert_eq!(res, MATED);
                assert_eq!(table[idx_normalized(w_king, sq("b3"), sq("b1"), sq("c2"), Black)], INVALID);
            } else if ChessBitboard::new(0x7070702).is_bit_set(w_king) {
                assert_eq!(res, INVALID);
            } else {
                assert_eq!(res, DRAW);
            }
            let j = idx_normalized(sq("b3"), w_king, sq("c2"), sq("b1"), Black);
            assert_eq!(table[j], res, "{j} {i} {w_king}");
        }
        let table = calc_tablebase_x_vs_x([Bishop, Rook]);
        let i = idx_normalized(sq("h8"), sq("f8"), sq("h7"), sq("h6"), White);
        let res = table[i];
        dbg!(i, res);
        assert_eq!(res, DRAW);
    }

    #[test]
    #[ignore]
    fn game_over_in_one_test() {
        let table = calc_tablebase_x_vs_x([Knight, Knight]);
        let i = idx(sq("c2"), sq("a1"), sq("c5"), sq("a2"), White);
        assert_eq!(table[i], -MATED - 1, "{i}");
        let i = idx(sq("c2"), sq("h1"), sq("c5"), sq("a2"), White);
        dbg!(i);
        assert_eq!(table[i], DRAW);

        let table = &**ROOK_VS_QUEEN;
        let i = idx(sq("c1"), sq("b3"), sq("g1"), sq("c2"), White);
        dbg!(i);
        assert_eq!(table[i], MATED);
        let i = idx(sq("c1"), sq("b3"), sq("g1"), sq("e2"), Black);
        assert_eq!(table[i], -MATED - 1, "{i}");
        let i = idx(sq("c1"), sq("d3"), sq("g1"), sq("e2"), Black);
        assert_eq!(table[i], -MATED - 1, "{i}");
        let i = idx(sq("a1"), sq("c3"), sq("c1"), sq("c2"), Black);
        assert_eq!(table[i], -MATED - 1, "{i}");
        let i = idx_normalized(sq("a2"), sq("c3"), sq("c1"), sq("c1"), White);
        assert_eq!(table[i], DRAW, "{i}");
        let i = idx_normalized(sq("a2"), sq("c3"), sq("c1"), sq("d1"), Black);
        assert_eq!(table[i], DRAW, "{i}");
        let i = idx_normalized(sq("a2"), sq("c3"), sq("c1"), sq("d1"), White);
        assert_eq!(table[i], INVALID, "{i}");
        // not a mate, but another table, and we're using DTZ
        let i = idx(sq("d4"), sq("h1"), sq("g4"), sq("g4"), Black);
        assert_eq!(table[i], MATED, "{i}");
        let i = idx(sq("d4"), sq("h1"), sq("e4"), sq("g4"), White);
        let res = table[i];
        dbg!(i, res);
        assert_eq!(res, -MATED - 1, "{i}");

        let i = idx_normalized(sq("g1"), sq("f3"), sq("g2"), sq("g2"), White);
        assert_eq!(table[i], MATED, "{i}");
        let i = idx_normalized(sq("g1"), sq("f3"), sq("g2"), sq("g8"), Black);
        assert_eq!(table[i], -MATED - 1, "{i}");
    }

    #[test]
    #[ignore]
    fn normal_cases_test() {
        let table = &**ROOK_VS_QUEEN;
        let i = idx_normalized(sq("h1"), sq("f3"), sq("a2"), sq("a2"), White);
        assert_eq!(table[i], MATED, "{i}"); // DTM, not an actual mate
        let i = idx_normalized(sq("g1"), sq("f3"), sq("g2"), sq("g8"), Black);
        assert_eq!(table[i], -MATED - 1, "{i}");
        let i = idx_normalized(sq("h1"), sq("f3"), sq("a2"), sq("g8"), Black);
        assert_eq!(table[i], -MATED - 1, "{i}");
        let i = idx_normalized(sq("f1"), sq("f3"), sq("a2"), sq("g8"), Black);
        assert_eq!(table[i], -MATED - 1, "{i}");
        let i = idx_normalized(sq("h2"), sq("f3"), sq("a2"), sq("g8"), Black);
        assert_eq!(table[i], -MATED - 1, "{i}");
        let i = idx_normalized(sq("g1"), sq("f3"), sq("a2"), sq("g8"), White);
        assert_eq!(table[i], MATED + 2, "{i}");
        let i = idx_normalized(sq("g1"), sq("f3"), sq("a2"), sq("h8"), Black);
        assert_eq!(table[i], -MATED - 3, "{i}");
        let i = idx_normalized(sq("g1"), sq("f3"), sq("a8"), sq("h7"), Black);
        assert_eq!(table[i], -MATED - 3, "{i}"); // actual mate
    }
}
