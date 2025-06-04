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
use crate::games::NoHistory;
use crate::games::uttt::ColoredUtttPieceType::{OStone, XStone};
use crate::games::uttt::UtttColor::*;
use crate::games::uttt::uttt_square::UtttSquare;
use crate::games::uttt::{UnverifiedUtttBoard, UtttBoard, UtttMove, UtttSubSquare};
use crate::general::board::Strictness::Strict;
use crate::general::board::{Board, BoardHelpers, UnverifiedBoard};
use crate::general::perft::perft;
use crate::search::Depth;
use rand::SeedableRng;
use rand::rngs::StdRng;

#[test]
fn perft_tests() {
    let seed = 42;
    for (fen, perft_res) in UtttBoard::perft_test_positions() {
        let pos = UtttBoard::from_alternative_fen(fen, Strict).unwrap();
        println!("{pos}");
        let n = if cfg!(debug_assertions) { 7 } else { 100 };
        for (depth, nodes) in perft_res.iter().enumerate().take(n) {
            let res = perft(Depth::new(depth), pos, false);
            assert_eq!(res.nodes, *nodes, "{fen}, depth {depth}: {0} should be {1}", res.nodes, *nodes);
        }
        let mut rng = StdRng::seed_from_u64(seed);
        if pos.cannot_call_movegen() {
            continue;
        }
        let mov = pos.random_legal_move(&mut rng);
        if mov.is_none() {
            assert_eq!(perft_res[1], 0);
        } else {
            assert!(pos.is_move_legal(mov.unwrap()), "{pos} {}", mov.unwrap());
        }
    }
}

#[test]
fn alternative_fen_test() {
    for pos in UtttBoard::bench_positions() {
        // the ply isn't part of the alternative fen description
        let pos = pos.set_ply_since_start(0).unwrap().verify(Strict).unwrap();
        let roundtrip = UtttBoard::from_alternative_fen(&pos.to_alternative_fen(), Strict).unwrap();
        if !pos.cannot_call_movegen() {
            assert_eq!(roundtrip.legal_moves_slow(), pos.legal_moves_slow());
        }
        assert_eq!(roundtrip, pos);
        assert_eq!(roundtrip.hash_pos(), pos.hash_pos());
    }
}

#[test]
fn sub_board_won_test() {
    let mut pos = UnverifiedUtttBoard::new(UtttBoard::default());
    let sub_board = UtttSubSquare::from_bb_idx(0);
    pos.place_piece(UtttSquare::new(sub_board, UtttSubSquare::unchecked(0)), XStone);
    pos.place_piece(UtttSquare::new(sub_board, UtttSubSquare::unchecked(1)), OStone);
    pos.place_piece(UtttSquare::new(sub_board, UtttSubSquare::unchecked(3)), XStone);
    pos.place_piece(UtttSquare::new(sub_board, UtttSubSquare::unchecked(2)), OStone);
    assert!(!pos.verify(Strict).unwrap().is_sub_board_won(X, sub_board));
    assert!(!pos.verify(Strict).unwrap().is_sub_board_won(O, sub_board));
    pos.place_piece(UtttSquare::new(sub_board, UtttSubSquare::unchecked(6)), XStone);
    let pos = pos.verify(Strict).unwrap();
    assert!(pos.is_sub_board_won(X, sub_board));
    assert!(!pos.is_sub_board_won(O, sub_board));
    assert!(!pos.is_sub_board_open(sub_board));
    assert!(pos.is_sub_board_open(UtttSubSquare::unchecked(1)));
    assert!(!pos.is_game_lost_slow(&NoHistory::default()));
    assert_eq!(pos.active, X); // the active player doesn't get updated through place_piece
    assert_eq!(pos.last_move, UtttMove::NULL);
    let pos =
        pos.remove_piece(UtttSquare::new(sub_board, UtttSubSquare::unchecked(3))).unwrap().verify(Strict).unwrap();
    assert!(!pos.is_sub_board_won(X, sub_board));
    assert!(pos.is_sub_board_open(sub_board));
}
