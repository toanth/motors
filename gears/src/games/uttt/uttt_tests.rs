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
use crate::games::uttt::uttt_square::UtttSquare;
use crate::games::uttt::ColoredUtttPieceType::{OStone, XStone};
use crate::games::uttt::UtttColor::*;
use crate::games::uttt::{UtttBoard, UtttMove, UtttPiece, UtttSubSquare};
use crate::general::board::Strictness::Strict;
use crate::general::board::{Board, UnverifiedBoard};
use crate::general::perft::perft;
use crate::search::Depth;

#[test]
fn perft_tests() {
    for (fen, perft_res) in UtttBoard::perft_test_positions() {
        let pos = UtttBoard::from_alternative_fen(fen, Strict).unwrap();
        println!("{pos}");
        let n = if cfg!(debug_assertions) { 7 } else { 100 };
        for (depth, nodes) in perft_res.iter().enumerate().take(n) {
            let res = perft(Depth::new_unchecked(depth), pos);
            assert_eq!(
                res.nodes, *nodes,
                "{fen}, depth {depth}: {0} should be {1}",
                res.nodes, *nodes
            );
        }
    }
}

#[test]
fn alternative_fen_test() {
    for pos in UtttBoard::bench_positions() {
        // the ply isn't part of the alternative fen description
        let pos = pos.set_ply_since_start(0).unwrap().verify(Strict).unwrap();
        let roundtrip = UtttBoard::from_alternative_fen(&pos.to_alternative_fen(), Strict).unwrap();
        assert_eq!(roundtrip.legal_moves_slow(), pos.legal_moves_slow());
        assert_eq!(roundtrip, pos);
        assert_eq!(roundtrip.zobrist_hash(), pos.zobrist_hash());
    }
}

#[test]
fn sub_board_won_test() {
    let pos = UtttBoard::default();
    let sub_board = UtttSubSquare::from_bb_index(0);
    let pos = pos
        .place_piece(UtttPiece::new(
            XStone,
            UtttSquare::new(sub_board, UtttSubSquare::unchecked(0)),
        ))
        .unwrap();
    let pos = pos.place_piece_unchecked(
        UtttSquare::new(sub_board, UtttSubSquare::unchecked(1)),
        OStone,
    );
    let pos = pos.place_piece_unchecked(
        UtttSquare::new(sub_board, UtttSubSquare::unchecked(3)),
        XStone,
    );
    let pos = pos.place_piece_unchecked(
        UtttSquare::new(sub_board, UtttSubSquare::unchecked(2)),
        OStone,
    );
    assert!(!pos.verify(Strict).unwrap().is_sub_board_won(X, sub_board));
    assert!(!pos.verify(Strict).unwrap().is_sub_board_won(O, sub_board));
    let pos = pos.place_piece_unchecked(
        UtttSquare::new(sub_board, UtttSubSquare::unchecked(6)),
        XStone,
    );
    let pos = pos.verify(Strict).unwrap();
    assert!(pos.is_sub_board_won(X, sub_board));
    assert!(!pos.is_sub_board_won(O, sub_board));
    assert!(!pos.is_sub_board_open(sub_board));
    assert!(pos.is_sub_board_open(UtttSubSquare::unchecked(1)));
    assert!(!pos.is_game_lost_slow());
    assert_eq!(pos.active, X); // the active player doesn't get updated through place_piece
    assert_eq!(pos.last_move, UtttMove::NULL);
    let pos = pos
        .remove_piece(UtttSquare::new(sub_board, UtttSubSquare::unchecked(3)))
        .unwrap()
        .verify(Strict)
        .unwrap();
    assert!(!pos.is_sub_board_won(X, sub_board));
    assert!(pos.is_sub_board_open(sub_board));
}
