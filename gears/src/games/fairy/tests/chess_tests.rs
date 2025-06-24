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

/// The following test cases are adapted from test cases for the [`Chessboard`] implementation
use crate::PlayerResult;
use crate::PlayerResult::{Draw, Lose};
use crate::games::chess::Chessboard;
use crate::games::chess::squares::{F_FILE_NUM, G_FILE_NUM};
use crate::games::fairy::Side::{Kingside, Queenside};
use crate::games::fairy::attacks::MoveKind;
use crate::games::fairy::moves::FairyMove;
use crate::games::fairy::rules::{Rules, RulesRef};
use crate::games::fairy::{FairyBitboard, FairyBoard, FairyColor, FairySquare, Side, UnverifiedFairyBoard};
use crate::games::{
    AbstractPieceType, BoardHistory, Color, ColoredPiece, Coordinates, NoHistory, Size, ZobristHistory, chess,
    n_fold_repetition,
};
use crate::general::bitboards::{Bitboard, RawBitboard};
use crate::general::board::Strictness::{Relaxed, Strict};
use crate::general::board::{Board, BoardHelpers, UnverifiedBoard};
use crate::general::moves::Move;
use crate::general::perft::perft;
use crate::general::squares::{GridCoordinates, GridSize, RectangularCoordinates};
use crate::search::Depth;
use itertools::Itertools;
use rand::rng;
use std::collections::HashSet;
use std::str::FromStr;
use strum::IntoEnumIterator;

fn chess_invariants(board: &UnverifiedFairyBoard) {
    assert_eq!(board.size.num_squares(), 64);
    assert_eq!(board.size, GridSize::chess());
    assert!(board.neutral_bb.is_zero());
    assert_eq!(board.mask_bb, 0xffff_ffff_ffff_ffff);
    let both = board.color_bitboards[0] & board.color_bitboards[1];
    assert!(both.is_zero());
    let rules = board.rules.0.clone();
    assert!(rules.has_ep);
    assert_eq!(rules.startpos_fen_part, chess::START_FEN);
}

#[test]
fn empty_test() {
    let board = FairyBoard::empty();
    let b2 = FairyBoard::empty_for_settings(RulesRef::new(Rules::chess()));
    assert_eq!(board, b2);
    chess_invariants(&board);
    assert_eq!(board.draw_counter, 0);
    assert_eq!(board.ply_since_start, 0);
    assert!(board.last_move.is_null());
    assert!(board.color_bitboards[0].is_zero());
    assert!(board.color_bitboards[1].is_zero());
    assert!(board.neutral_bb.is_zero());
    assert!(board.ep.is_none());
    for c in FairyColor::iter() {
        for side in Side::iter() {
            assert!(!board.castling_info.can_castle(c, side));
        }
    }
    assert_eq!(board.active, FairyColor::default());
    assert!(board.verify(Relaxed).is_err());
}

#[test]
fn startpos_test() {
    let board = FairyBoard::default();
    chess_invariants(&board);
    assert_eq!(board.fen_no_rules(), chess::START_FEN);
    assert_eq!(board.halfmove_ctr_since_start(), 0);
    assert_eq!(board.fullmove_ctr_1_based(), 1);
    assert_eq!(board.ply_since_start, 0);
    assert_eq!(board.draw_counter, 0);
    assert!(board.ep.is_none());
    assert_eq!(board.active_player(), FairyColor::from_name("white", &board.settings()).unwrap());
    for c in FairyColor::iter() {
        for side in Side::iter() {
            assert!(board.castling_info.can_castle(c, side));
        }
    }
    assert!(!board.is_in_check());
    assert!(!board.is_game_lost_slow(&NoHistory::default()));
    assert_eq!(board.player_bb(FairyColor::first()), FairyBitboard::new(0xffff, board.size()));
    assert_eq!(board.player_bb(FairyColor::second()), FairyBitboard::new(0xffff << 48, board.size()));
    assert_eq!(board.occupied_bb(), FairyBitboard::new(0xffff_0000_0000_ffff, board.size()));
    let white_king_bb = board.royal_bb_for(FairyColor::first());
    let king_bb = board.piece_bb(board.rules().pieces().find(|(_, p)| p.royal).unwrap().0);
    assert_eq!(white_king_bb, board.player_bb(FairyColor::first()) & king_bb);
    assert!(white_king_bb.is_single_piece());
    assert_eq!(white_king_bb.to_square().unwrap(), GridCoordinates::algebraic('e', 1).unwrap());
    let black_king_bb = board.royal_bb_for(FairyColor::second());
    assert_eq!(black_king_bb.to_square().unwrap(), GridCoordinates::algebraic('e', 8).unwrap());
    assert_eq!(white_king_bb | black_king_bb, king_bb);
    let square = FairySquare::from_rank_file(4, F_FILE_NUM);
    assert!(board.colored_piece_on(square).is_empty());
    let moves = board.pseudolegal_moves();
    assert!(moves.len() >= 20); // currently, castling moves are pseudolegal even if obstructed
    let legal_moves = board.legal_moves_slow();
    assert_eq!(legal_moves.len(), 20);
    assert!(legal_moves.into_iter().sorted().eq(moves.into_iter().filter(|&m| board.is_move_legal(m)).sorted()));
}

#[test]
fn invalid_fen_test() {
    // some of these FENs have been found through cargo fuzz for `Chessboard`
    let fens = &[
        "",
        "3Ss9999999999999999999999999999999",
        "½",
        "QQQQQQQQw`",
        "q0018446744073709551615",
        "QQQQKQQQ\nwV0 \n",
        "kQQQQQDDw-W0w",
        "2rr2k1/1p4bp/p1q1pqp1/4Pp1n/2PB4/1PN3P1/P3Q2P/2Rr2K1 w - f6 0 20",
        "7r/8/8/8/8/1k4P1/1K6/8 w - - 3 3",
    ];
    for fen in fens {
        let pos = FairyBoard::from_fen_for("chess", fen, Relaxed);
        assert!(pos.is_err(), "{fen}");
    }
}

#[test]
fn simple_fen_test() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w Qk - 0 1";
    let board = FairyBoard::from_fen_for("chess", fen, Strict).unwrap();
    assert!(!board.castling_info.can_castle(FairyColor::first(), Kingside));
    assert!(board.castling_info.can_castle(FairyColor::first(), Queenside));
    assert!(board.castling_info.can_castle(FairyColor::second(), Kingside));
    assert!(!board.castling_info.can_castle(FairyColor::second(), Queenside));
    let fens = [
        "8/8/8/3K4/8/8/5k2/8 w - - 0 1",
        "K7/R7/R7/R7/R7/R7/P7/k7 w - - 0 1",
        "QQKBnknn/8/8/8/8/8/8/8 w - - 0 1",
        "b5k1/b3Q3/3Q1Q2/5Q2/K1bQ1Qb1/2bbbbb1/6Q1/3QQ2b b - - 0 1",
        "rnbq1bn1/pppppp1p/8/K7/5k2/8/PPPP1PPP/RNBQ1BNR w - - 0 1",
        &FairyBoard::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w HhAa - 0 1", Strict)
            .unwrap()
            .fen_no_rules(),
        "rnbqkbnr/1ppppppp/p7/8/8/8/PPPPPPP1/RNBQKBN1 w Ah - 0 1",
        "rnbqkbnr/1ppppppp/p7/8/3pP3/8/PPPP1PP1/RNBQKBN1 b Ah e3 3 1",
        // chess960 fens (from webperft):
        "1rqbkrbn/1ppppp1p/1n6/p1N3p1/8/2P4P/PP1PPPP1/1RQBKRBN w FBfb - 0 9",
        "rbbqn1kr/pp2p1pp/6n1/2pp1p2/2P4P/P7/BP1PPPP1/R1BQNNKR w HAha - 1 42",
        "rqbbknr1/1ppp2pp/p5n1/4pp2/P7/1PP5/1Q1PPPPP/R1BBKNRN w GAga - 42 9",
    ];
    for fen in fens {
        let board = FairyBoard::from_fen(fen, Relaxed).unwrap();
        assert_eq!(fen, board.fen_no_rules());
        assert_eq!(board, FairyBoard::from_fen(&board.as_fen(), Relaxed).unwrap());
    }
}

#[test]
fn invalid_castle_right_test() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w AQk - 0 1";
    let board = FairyBoard::from_fen(fen, Relaxed);
    assert!(board.is_err());
}

#[test]
fn failed_chess_fuzz_test() {
    // the chess FEN used `KQkq` as castling flags, but currently disambiguated x-fens aren't supported by the fairy implementation
    // (mostly because that would require a way to specify what counts as a "rook" for the purpose of castling)
    let pos =
        FairyBoard::from_fen("r2k3r/ppp1pp1p/2nqb1Nn/3P4/4P3/2PP4/PR1NBPPP/R2NKRQ1 w FAha - 1 5", Relaxed).unwrap();
    let pos = pos.debug_verify_invariants(Relaxed).unwrap();
    for mov in pos.legal_moves_slow() {
        let new_pos = pos.clone().make_move(mov).unwrap_or(pos.clone());
        _ = new_pos.debug_verify_invariants(Relaxed).unwrap();
    }
    let mov = FairyMove::from_text("sB3x", &pos);
    assert!(mov.is_err());
}

#[test]
fn weird_fen_test() {
    // invalid ep square set, but 'Relaxed' should accept that (currently, 'Strict' does as well)
    let fen = "1nbqkbnr/ppp1pppp/8/r2pP3/6K1/8/PPPP1PPP/RNBQ1BNR w k d6 0 2";
    let board = FairyBoard::from_fen(fen, Relaxed).unwrap();
    assert_eq!(FairyBoard::from_fen(&board.as_fen(), Relaxed).unwrap(), board);
    assert_eq!(board.num_legal_moves(), Chessboard::from_fen(fen, Relaxed).unwrap().num_legal_moves());
    let fen = "1nbqkbnr/ppppppp1/6r1/6Pp/6K1/8/PPPP1PPP/RNBQ1BNR w k h6 0 2";
    let board = FairyBoard::from_fen(fen, Relaxed).unwrap();
    assert_eq!(board.num_legal_moves(), Chessboard::from_fen(fen, Relaxed).unwrap().num_legal_moves());
    // TODO: Currently, this gets accepted, but maybe that's fine? Annoying to have a mismatch to the chess implementation though
    // let fen = "1nbqkbnr/pppppppp/8/r5Pp/6K1/8/PPPP1PPP/RNBQ1BNR w k h6 0 2";
    // assert!(FairyBoard::from_fen(fen, Relaxed).is_err());
    let fen = "1nbqkbnr/ppppppp1/8/r5Pp/6K1/8/PPPP1PPP/RNBQ1BNR w k - 0 2";
    assert!(FairyBoard::from_fen(fen, Strict).is_ok());
}

#[test]
fn many_moves_test() {
    let fen = "QQQQQQBk/Q6B/Q6Q/Q6Q/Q6Q/Q6Q/Q6Q/KQQQQQQQ w - - 0 1";
    let board = FairyBoard::from_fen(fen, Relaxed).unwrap();
    let moves = board.pseudolegal_moves();
    assert_eq!(moves.len(), 265);
    let perft_res = perft(Depth::new(1), board, false);
    assert_eq!(perft_res.nodes, 265);
}

#[test]
fn simple_perft_test() {
    let endgame_fen = "6k1/8/6K1/8/3B1N2/8/8/7R w - - 0 1";
    let board = FairyBoard::from_fen(endgame_fen, Relaxed).unwrap();
    let perft_res = perft(Depth::new(1), board, false);
    assert_eq!(perft_res.depth, Depth::new(1));
    assert_eq!(perft_res.nodes, 5 + 7 + 13 + 14);
    let board = FairyBoard::default();
    let perft_res = perft(Depth::new(1), board.clone(), true);
    assert_eq!(perft_res.depth, Depth::new(1));
    assert_eq!(perft_res.nodes, 20);
    let perft_res = perft(Depth::new(2), board, false);
    assert_eq!(perft_res.depth, Depth::new(2));
    assert_eq!(perft_res.nodes, 20 * 20);

    let board = FairyBoard::from_fen("r1bqkbnr/1pppNppp/p1n5/8/8/8/PPPPPPPP/R1BQKBNR b KQkq - 0 3", Strict).unwrap();
    let perft_res = perft(Depth::new(1), board.clone(), true);
    assert_eq!(perft_res.nodes, 26);
    assert_eq!(perft(Depth::new(3), board, true).nodes, 16790);

    let board =
        FairyBoard::from_fen("rbbqQ1kr/1p2p1pp/6n1/p1pp1p2/2P4P/P7/BP1PPPP1/R1B1NNKR b KQkq - 0 10", Strict).unwrap();
    assert_eq!(board.num_legal_moves(), 2);
    let board =
        FairyBoard::from_fen("rbbqn1kr/pp2p1pp/6n1/2pp1p2/2P4P/P7/BP1PPPP1/R1BQNNKR w HAha - 0 9", Strict).unwrap();
    let perft_res = perft(Depth::new(4), board, false);
    assert_eq!(perft_res.nodes, 890_435);

    // DFRC
    let board = FairyBoard::from_fen("r1q1k1rn/1p1ppp1p/1npb2b1/p1N3p1/8/1BP4P/PP1PPPP1/1RQ1KRBN w BFag - 0 9", Strict)
        .unwrap();
    assert_eq!(perft(Depth::new(4), board, false).nodes, 1_187_103);
}

#[test]
fn mate_test() {
    let board = FairyBoard::from_fen("4k3/8/4K3/8/8/8/8/6R1 w - - 0 1", Strict).unwrap();
    assert!(!board.is_game_lost_slow(&NoHistory::default()));
    let moves = board.pseudolegal_moves();
    for mov in moves {
        if mov.src_square_in(&board) == board.king_square(FairyColor::first()) {
            assert_eq!(board.is_pseudolegal_move_legal(mov), mov.dest_square_in(&board).row() != 6);
        } else {
            assert!(board.is_pseudolegal_move_legal(mov));
        }
        if !board.is_pseudolegal_move_legal(mov) {
            continue;
        }
        let checkmates = mov.piece(&board).name(&board.settings()).as_ref() == "white rook"
            && mov.dest_square_in(&board) == FairySquare::from_rank_file(7, G_FILE_NUM);
        assert_eq!(board.is_game_won_after_slow(mov, NoHistory::default()), checkmates);
        let new_board = board.clone().make_move(mov).unwrap();
        assert_eq!(new_board.is_game_lost_slow(&NoHistory::default()), checkmates);
    }
}

#[test]
fn capture_only_test() {
    let board = FairyBoard::default();
    assert!(board.tactical_pseudolegal().is_empty());
    let board = FairyBoard::from_name("kiwipete").unwrap();
    assert_eq!(board.tactical_pseudolegal().len(), 8);
    let board = FairyBoard::from_fen("8/7r/8/K1k5/8/8/4p3/8 b - - 10 11", Strict).unwrap();
    let tactical = board.tactical_pseudolegal();
    assert_eq!(tactical.len(), 0); // for now, only captures count as tactical in fairy chess
    for m in tactical {
        assert!(matches!(m.kind(), MoveKind::ChangePiece(_)));
        assert!(!m.is_capture());
        assert_eq!(m.piece(&board).name(&board.settings()).as_ref(), "black pawn");
    }
}

#[test]
fn fifty_mr_test() {
    let board = FairyBoard::from_fen("1r2k3/P5R1/2P5/8/8/8/8/1R1K3R w BHb - 99 51", Strict).unwrap();
    let moves = board.legal_moves_slow();
    assert_eq!(moves.len(), 48);
    let mut mate_ctr = 0;
    let mut draw_ctr = 0;
    let resetting = ["c6c7", "a7a8q", "a7a8n", "a7a8b", "a7a8r", "a7b8n", "a7b8b", "a7b8r", "a7b8q", "b1b8"]
        .into_iter()
        .map(|str| FairyMove::from_text(str, &board).unwrap())
        .collect_vec();
    for m in moves {
        let new_pos = board.clone().make_move(m).unwrap();
        if resetting.contains(&m) {
            assert_eq!(new_pos.ply_draw_clock(), 0);
            if !["b1b8", "a7b8q", "a7b8r"].contains(&m.compact_formatter(&board).to_string().as_str()) {
                assert!(new_pos.player_result_slow(&NoHistory::default()).is_none(), "{m:?}");
            } else {
                assert!(new_pos.is_game_lost_slow(&NoHistory::default()));
                mate_ctr += 1;
            }
        } else {
            assert_eq!(new_pos.ply_draw_clock(), 100);
            let res = new_pos.player_result_slow(&NoHistory::default());
            if let Some(Lose) = res {
                mate_ctr += 1;
            } else {
                assert!(matches!(res, Some(Draw)));
                draw_ctr += 1;
            }
        }
    }
    assert_eq!(mate_ctr, 4);
    assert_eq!(draw_ctr, 37);
}

#[test]
fn repetition_test() {
    let mut board = FairyBoard::default();
    let new_hash = board.clone().make_nullmove().unwrap().hash_pos();
    let moves = ["g1f3", "g8f6", "f3g1", "f6g8", "g1f3", "g8f6", "f3g1", "f6g8", "e2e4"];
    let mut hist = ZobristHistory::default();
    assert_ne!(new_hash, board.hash_pos());
    for (i, mov) in moves.iter().enumerate() {
        let hash = board.hash_pos();
        assert_eq!(i > 3, n_fold_repetition(2, &hist, hash, board.ply_draw_clock()));
        assert_eq!(i > 7, n_fold_repetition(3, &hist, hash, board.ply_draw_clock()));
        assert_eq!(i == 8, board.player_result_no_movegen(&hist).is_some_and(|r| r == Draw));
        hist.push(hash);
        let mov = FairyMove::from_compact_text(mov, &board).unwrap();
        board = board.make_move(mov).unwrap();
        assert_eq!(n_fold_repetition(3, &hist, board.hash_pos(), board.ply_draw_clock()), board.is_draw_slow(&hist));
        assert_eq!(board.is_draw_slow(&hist), board.player_result_no_movegen(&hist).is_some());
    }
    let lucena = Chessboard::from_name("lucena").unwrap().as_fen();
    board = FairyBoard::from_fen_for("chess", &lucena, Strict).unwrap();
    assert_eq!(board.active_player(), FairyColor::first());
    let hash = board.hash_pos();
    let moves = ["c1b1", "a2c2", "b1e1", "c2a2", "e1c1"];
    for mov in moves {
        board = board.clone().make_move(FairyMove::from_compact_text(mov, &board).unwrap()).unwrap();
        assert_ne!(board.hash_pos(), hash);
        assert!(!n_fold_repetition(2, &hist, board.hash_pos(), 12345));
    }
    assert_eq!(board.active_player(), FairyColor::second());
    let kiwipete = Chessboard::from_name("kiwipete").unwrap().as_fen();
    let board = FairyBoard::from_fen_for("chess", &kiwipete, Strict).unwrap();
    let mut new_pos = board.clone();
    for mov in ["e1d1", "h8h7", "d1e1", "h7h8"] {
        new_pos = new_pos.make_move_from_str(mov).unwrap();
    }
    assert_ne!(new_pos.hash_pos(), board.hash_pos());
}

#[test]
fn checkmate_test() {
    let fen = "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3";
    let pos = FairyBoard::from_fen(fen, Strict).unwrap();
    assert_eq!(pos.active_player(), FairyColor::first());
    assert_eq!(pos.ply_since_start, 4);
    assert!(pos.clone().debug_verify_invariants(Strict).is_ok());
    assert!(pos.is_in_check());
    assert_eq!(pos.in_check_bb(FairyColor::first()).to_square(), pos.king_square(FairyColor::first()));
    let moves = pos.legal_moves_slow();
    assert!(moves.is_empty());
    assert!(pos.is_game_lost_slow(&NoHistory::default()));
    assert_eq!(pos.player_result_slow(&NoHistory::default()), Some(Lose));
    assert!(!pos.is_draw_slow(&NoHistory::default()));
    assert!(pos.make_nullmove().is_none());
    // this position can be claimed as a draw according to FIDE rules but it's also a mate in 1
    let pos = FairyBoard::from_fen("k7/p1P5/1PK5/8/8/8/8/8 w - - 99 51", Strict).unwrap();
    assert!(pos.match_result_slow(&NoHistory::default()).is_none());
    let mut draws = 0;
    let mut wins = 0;
    for mov in pos.legal_moves_slow() {
        let new_pos = pos.clone().make_move(mov).unwrap();
        if let Some(res) = new_pos.player_result_slow(&NoHistory::default()) {
            match res {
                PlayerResult::Win => {
                    unreachable!("The other player can't win through one of our moves")
                }
                Lose => {
                    wins += 1;
                }
                Draw => draws += 1,
            }
        }
    }
    assert_eq!(draws, 5);
    assert_eq!(wins, 3);
}

#[test]
fn weird_position_test() {
    // There's a similar test in `motors`
    // This fen is actually a legal chess position
    let fen = "q2k2q1/2nqn2b/1n1P1n1b/2rnr2Q/1NQ1QN1Q/3Q3B/2RQR2B/Q2K2Q1 w - - 0 1";
    let board = FairyBoard::from_fen(fen, Strict).unwrap();
    assert_eq!(board.active_player(), FairyColor::first());
    assert_eq!(perft(Depth::new(3), board, true).nodes, 568_299);
    // not a legal chess position, but the board should support this
    let fen = "RRRRRRRR/RRRRRRRR/BBBBBBBB/BBBBBBBB/QQQQQQQQ/QQQQQQQQ/QPPPPPPP/K6k b - - 0 1";
    let board = FairyBoard::from_fen(fen, Relaxed).unwrap();
    assert!(board.pseudolegal_moves().len() <= 3);
    let mut rng = rng();
    let mov = board.random_legal_move(&mut rng).unwrap();
    let board = board.make_move(mov).unwrap();
    assert_eq!(board.pseudolegal_moves().len(), 2);
    let fen = "B4Q1b/8/8/8/2K3P1/5k2/8/b4RNB b - - 0 1"; // far too many checks, but we still accept it
    let board = FairyBoard::from_fen(fen, Relaxed).unwrap();
    let num_pseudolegal = board.pseudolegal_moves().len();
    assert!(num_pseudolegal <= 20, "{num_pseudolegal}");
    assert_eq!(board.legal_moves_slow().len(), 3);
    // maximum number of legal moves in any position reachable from startpos
    let fen = "R6R/3Q4/1Q4Q1/4Q3/2Q4Q/Q4Q2/pp1Q4/kBNN1KB1 w - - 0 1";
    let board = FairyBoard::from_fen(fen, Strict).unwrap();
    assert_eq!(board.legal_moves_slow().len(), 218);
    assert!(board.clone().debug_verify_invariants(Strict).is_ok());
    let board = board.make_nullmove().unwrap();
    assert!(board.legal_moves_slow().is_empty());
    assert_eq!(board.player_result_slow(&NoHistory::default()), Some(Draw));
    // unlike the Chessboard, the Fairyboard currently doesn't support X-FEN, so those tests are missing here
    // An ep capture is pseudolegal but not legal
    let fen = "1nbqkbnr/ppp1pppp/8/r2pP2K/8/8/PPPP1PPP/RNBQ1BNR w k d6 0 2";
    let pos = FairyBoard::from_fen(fen, Relaxed).unwrap();
    assert_eq!(pos.num_legal_moves(), 31);
    // only legal move is to castle
    let fen = "8/8/8/8/4k3/7p/4q2P/6KR w K - 0 1";
    let pos = FairyBoard::from_fen(fen, Relaxed).unwrap();
    let moves = pos.legal_moves_slow();
    assert_eq!(moves.len(), 1);
    let after_move = pos.make_move(moves[0]).unwrap();
    assert_eq!(after_move.fen_no_rules(), "8/8/8/8/4k3/7p/4q2P/5RK1 b - - 1 1");
}

#[test]
fn chess960_startpos_test() {
    let mut fens = HashSet::new();
    let mut startpos_found = false;
    let mut same_fen = false;
    assert_eq!(FairyBoard::startpos(), FairyBoard::from_fen_for("chess", chess::START_FEN, Strict).unwrap());
    assert_eq!(FairyBoard::startpos().fen_no_rules(), chess::START_FEN);
    for i in 0..960 {
        let chessboard = Chessboard::chess_960_startpos(i).unwrap();
        let board = FairyBoard::from_fen_for("chess", &chessboard.as_fen(), Strict).unwrap();
        assert!(board.clone().debug_verify_invariants(Strict).is_ok());
        assert!(fens.insert(board.fen_no_rules()));
        let num_moves = board.num_legal_moves();
        assert!((18..=21).contains(&num_moves)); // 21 legal moves because castling can be legal
        for c in FairyColor::iter() {
            for s in Side::iter() {
                assert!(board.castling_info.can_castle(c, s));
            }
        }
        assert_eq!(
            board.king_square(FairyColor::first()).unwrap().flip_up_down(board.size()),
            board.king_square(FairyColor::second()).unwrap()
        );
        startpos_found |= board == FairyBoard::startpos();
        same_fen |= board.fen_no_rules() == chess::START_FEN;
        let chess_nodes = perft(Depth::new(3), chessboard, false).nodes;
        let fairy_nodes = perft(Depth::new(3), board, true).nodes;
        assert_eq!(chess_nodes, fairy_nodes);
    }
    assert!(!same_fen);
    assert!(startpos_found);
}

#[test]
fn ep_test() {
    let fen = "5k2/2p5/8/3P4/1pP5/8/P7/1K6 b - c3 0 1";
    let pos = FairyBoard::from_fen(fen, Relaxed).unwrap();
    assert_eq!(pos.ep, Some(FairySquare::from_str("c3").unwrap()));
    let pos = pos.debug_verify_invariants(Strict).unwrap();
    let new_pos = pos.clone().make_move_from_str("c7c5").unwrap();
    assert_eq!(new_pos.ep, Some(FairySquare::from_str("c6").unwrap()));
    let _ = new_pos.debug_verify_invariants(Strict).unwrap();
    let perft = perft(Depth::new(4), pos, true);
    assert_eq!(perft.nodes, 5020);
}

#[test]
fn insufficient_material_test() {
    let insufficient = [
        "8/4k3/8/8/8/8/8/2K5 w - - 0 1",
        "8/4k3/8/8/8/8/5N2/2K5 w - - 0 1",
        // fairy chess doesn't recognize that same-square colored bishops are unable to checkmate
    ];
    let sufficient = [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "8/8/4k3/8/8/1K6/8/7R w - - 0 1",
        "5r2/3R4/4k3/8/8/1K6/8/8 w - - 0 1",
        "8/8/4k3/8/8/1K6/8/6BB w - - 0 1",
        "8/8/4B3/8/8/7K/8/6bk w - - 0 1",
        "3B3B/2B5/1B1B4/B6k/3B4/4B3/1K3B2/1B6 w - - 0 1",
        "8/3k4/8/8/8/8/NNN5/1K6 w - - 0 1",
    ];
    let sufficient_but_unreasonable = [
        "6B1/8/8/6k1/8/2K5/8/6b1 w - - 0 1",
        "8/8/4B3/8/8/7K/8/6bk b - - 0 1",
        "8/8/4B3/7k/8/8/1K6/6b1 w - - 0 1",
        "8/3k4/8/8/8/8/1NN5/1K6 w - - 0 1",
        "8/2nk4/8/8/8/8/1NN5/1K6 w - - 0 1",
    ];
    for fen in insufficient {
        let board = FairyBoard::from_fen(fen, Strict).unwrap();
        assert!(board.is_draw_slow(&NoHistory::default()), "{fen}");
    }
    for fen in sufficient.iter().chain(sufficient_but_unreasonable.iter()) {
        let board = FairyBoard::from_fen(fen, Strict).unwrap();
        assert!(!board.is_draw_slow(&NoHistory::default()), "{fen}");
    }
}
