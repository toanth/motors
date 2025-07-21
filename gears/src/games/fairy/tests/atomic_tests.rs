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
use crate::games::fairy::Side::Kingside;
use crate::games::fairy::moves::FairyMove;
use crate::games::fairy::{FairyBoard, FairyCastleInfo, FairyColor, FairySquare};
use crate::games::{Color, NoHistory, ZobristHistory, chess, BoardHistDyn};
use crate::general::bitboards::RawBitboard;
use crate::general::board::Strictness::{Relaxed, Strict};
use crate::general::board::{Board, BoardHelpers};
use crate::general::moves::Move;
use crate::{GameResult, PlayerResult};
// See <https://lichess.org/study/uf9GpQyI> for a description of rules and interesting positions
// on which these test cases are based

#[test]
fn startpos_test() {
    let pos = FairyBoard::variant_simple("atomic").unwrap();
    assert_eq!(pos.fen_no_rules(), chess::START_FEN);
    assert_eq!(pos.num_legal_moves(), 20);
    assert!(pos.debug_verify_invariants(Strict).is_ok());
}

#[test]
fn capture_test() {
    let pos =
        FairyBoard::from_fen_for("atomic", "r1bqkbnr/pp1p3p/1Qn1ppp1/8/8/4P3/PPPP1PPP/RNB1KB1R b KQkq - 0 1", Strict)
            .unwrap();
    assert_eq!(pos.active_player(), FairyColor::second());
    let mov = FairyMove::from_text("a7b6", &pos).unwrap();
    let new_pos = pos.make_move(mov).unwrap();
    assert_eq!(new_pos.as_fen(), "atomic r1bqkbnr/1p1p3p/4ppp1/8/8/4P3/PPPP1PPP/RNB1KB1R w KQkq - 0 2");
}

#[test]
fn explosion_win_test() {
    let pos =
        FairyBoard::from_fen_for("atomic", "rnbqkbnr/pp2pppp/3p4/2p1N3/8/8/PPPPPPPP/RNBQKB1R w KQkq - 0 2", Strict)
            .unwrap();
    let new_pos = pos.make_move_from_str("e5f7").unwrap();
    assert_eq!(new_pos.match_result_slow(&NoHistory::default()).unwrap().result, GameResult::P1Win);
    assert_eq!(new_pos.fen_no_rules(), "rnbq3r/pp2p1pp/3p4/2p5/8/8/PPPPPPPP/RNBQKB1R b KQ - 0 2");
    assert!(!new_pos.is_in_check());
}

#[test]
fn ep_test() {
    let pos = FairyBoard::from_fen_for("atomic", "8/2bpk3/8/4P3/8/8/3K3B/8 b - - 0 1", Strict).unwrap();
    let before_ep = pos.make_move_from_str("d7d5").unwrap();
    assert_eq!(before_ep.fen_no_rules(), "8/2b1k3/8/3pP3/8/8/3K3B/8 w - d6 0 2");
    assert_eq!(before_ep.ep, Some(FairySquare::algebraic('d', 6).unwrap()));
    assert_eq!(before_ep, FairyBoard::from_fen_for("atomic", &before_ep.fen_no_rules(), Relaxed).unwrap());
    let ep = FairyMove::from_text("e5d6", &before_ep).unwrap();
    let after_ep = before_ep.make_move(ep).unwrap();
    assert!(after_ep.royal_bb_for(FairyColor::second()).is_zero());
    assert_eq!(after_ep.fen_no_rules(), "8/8/8/8/8/8/3K3B/8 b - - 0 2");
    assert!(after_ep.is_game_lost_slow(&NoHistory::default()));
    assert_eq!(after_ep.match_result_slow(&NoHistory::default()).unwrap().result, GameResult::P1Win);
}

#[test]
fn no_suicide_test() {
    let pos = FairyBoard::from_fen_for("atomic", "2K2k2/3p4/8/8/8/7B/8/8 w - - 0 1", Strict).unwrap();
    assert!(pos.debug_verify_invariants(Strict).is_ok());
    assert!(pos.clone().make_move_from_str("h3d7").is_err());
    assert!(pos.make_move_from_str("c8d7").is_err());
    let pos = FairyBoard::from_fen_for("atomic", "8/8/8/5Q2/8/2K5/2k5/8 b - - 0 1", Strict).unwrap();
    assert!(pos.debug_verify_invariants(Strict).is_ok());
    assert!(pos.make_move_from_str("f5c2").is_err());
    let pos = FairyBoard::from_fen_for("atomic", "5R2/8/8/8/4k3/5pK1/8/8 w - - 0 1", Strict).unwrap();
    // pseudolegal, but currently fairy only accepts legal moves
    let new_pos = pos.clone().make_move_from_str("f8f3");
    assert!(new_pos.is_err());
    for pos in pos.children() {
        assert!(pos.match_result_slow(&NoHistory::default()).is_none());
    }

    let pos = FairyBoard::from_fen_for("atomic", "2Kk4/3p4/8/8/8/7B/8/8 w - - 0 1", Strict).unwrap();
    for mov in ["h3d7", "c8d7", "c8d8"] {
        let new_pos = pos.clone().make_move_from_str(mov);
        assert!(new_pos.is_err());
    }
}

#[test]
fn check_test() {
    let pos =
        FairyBoard::from_fen_for("atomic", "rnbqk1r1/1p2p2p/p1pp1pp1/q1N5/3PP3/8/PPP2PPP/R2QKB1R w KQq - 0 1", Strict)
            .unwrap();
    assert!(pos.is_in_check());
    let legal = ["b2b4", "c2c3", "d1d2", "e1e2", "f1a6", "c5a6"];
    let mut num_seen = 0;
    for m in pos.pseudolegal_moves() {
        let contained = legal.contains(&m.compact_formatter(&pos).to_string().as_str());
        if pos.is_pseudolegal_move_legal(m) {
            assert!(contained);
            num_seen += 1;
            let new_pos = pos.clone().make_move(m).unwrap();
            assert!(!new_pos.is_in_check());
            assert!(new_pos.player_result_slow(&NoHistory::default()).is_none());
        } else {
            assert!(!contained)
        }
    }
    assert_eq!(num_seen, legal.len());

    let pos = FairyBoard::from_fen_for("atomic", "8/8/8/3k3b/8/8/4N1P1/3K4 w - - 0 1", Strict).unwrap();
    assert!(!pos.is_in_check());
    let pos = pos.make_nullmove().unwrap();
    let pos = pos.make_move_from_str("h5e2").unwrap();
    assert_eq!(pos.match_result_slow(&NoHistory::default()).unwrap().result, GameResult::P2Win);

    let pos = FairyBoard::from_fen_for("atomic", "8/8/8/7b/8/7N/6P1/2kK4 w - - 0 1", Strict).unwrap();
    assert!(!pos.is_in_check());
    let pos = pos.make_nullmove().unwrap();
    assert!(pos.make_move_from_str("h5d1").is_err());
}

#[test]
fn atomic_check_test() {
    let pos = FairyBoard::from_fen_for("atomic", "8/8/8/7b/5nn1/7N/6P1/1k1K4 w - - 0 1", Strict).unwrap();
    let new_pos = pos.make_move_from_str("h3f4");
    assert!(new_pos.is_err());

    let pos = FairyBoard::from_fen_for(
        "atomic",
        "r1b1k1nr/pp1pQ1pp/n1p2p2/1B2p3/1P5q/4P3/P1PP1PPP/RN2K2R b KQkq - 0 1",
        Strict,
    )
    .unwrap();
    assert!(pos.is_in_check());
    let moves = pos.legal_moves_slow();
    assert_eq!(moves.len(), 1);
    assert_eq!(moves[0].compact_formatter(&pos).to_string(), "h4f2");
    let new_pos = pos.make_move_from_str("h4f2").unwrap();
    assert!(!new_pos.is_in_check());
    assert!(new_pos.in_check_bb(FairyColor::second()).has_set_bit());
    assert_eq!(new_pos.player_result_slow(&NoHistory::default()), Some(PlayerResult::Lose));
}

#[test]
fn checkmate_stalemate_test() {
    let pos =
        FairyBoard::from_fen_for("atomic", "rn1qkb1r/pppBpppp/5n2/3p4/6b1/4PQ2/PPPP1PPP/RNB1K1NR b KQkq - 0 1", Strict)
            .unwrap();
    assert!(pos.is_in_check());
    assert_eq!(pos.num_legal_moves(), 0);
    assert_eq!(pos.player_result_slow(&NoHistory::default()), Some(PlayerResult::Lose));

    let pos = FairyBoard::from_fen_for("atomic", "8/8/8/8/8/8/6QQ/6kK b - - 0 1", Strict).unwrap();
    assert!(!pos.is_in_check());
    assert!(pos.legal_moves_slow().is_empty());
    assert_eq!(pos.player_result_slow(&NoHistory::default()), Some(PlayerResult::Draw));
}

#[test]
fn castling_test() {
    let pos = FairyBoard::from_fen_for("atomic", "6q1/8/8/8/8/3k4/6P1/4K2R b K - 0 1", Strict).unwrap();
    assert!(pos.castling_info.can_castle(FairyColor::first(), Kingside));
    let new_pos = pos.make_move_from_str("g8g2").unwrap();
    assert!(!new_pos.castling_info.can_castle(FairyColor::first(), Kingside));
    assert_eq!(new_pos.fen_no_rules(), "8/8/8/8/8/3k4/8/4K3 w - - 0 2");
    assert!(new_pos.is_draw_slow(&NoHistory::default()));

    let pos = FairyBoard::from_fen_for("atomic", "8/8/8/1q6/8/8/4k3/R3K3 w Q - 0 1", Strict).unwrap();
    let castling_move = FairyMove::from_text("e1a1", &pos).unwrap();
    assert!(pos.is_move_legal(castling_move));
    let pos = FairyBoard::from_fen_for("atomic", "8/8/8/8/8/8/4k3/R3K2q w Q - 0 1", Strict).unwrap();
    let castling_move = FairyMove::from_text("e1a1", &pos).unwrap();
    assert!(pos.is_move_legal(castling_move));
    let new_pos = pos.make_move(castling_move).unwrap();
    assert!(new_pos.in_check_bb(FairyColor::first()).is_zero());
    assert_eq!(new_pos.castling_info, FairyCastleInfo::default());

    let pos = FairyBoard::from_fen_for("atomic", "8/8/8/8/8/8/5k2/R3K2q w Q - 0 1", Strict).unwrap();
    let new_pos = pos.make_move_from_str("e1a1");
    assert!(new_pos.is_err());

    let pos = FairyBoard::from_fen_for("atomic", "8/8/8/8/8/8/3k4/R3K2q w Q - 0 1", Strict).unwrap();
    let new_pos = pos.make_move_from_str("e1a1").unwrap();
    assert!(!new_pos.is_in_check());
    assert_eq!(new_pos.castling_info, FairyCastleInfo::default());
}

#[test]
fn draw_test() {
    let pos = FairyBoard::from_fen_for("atomic", "8/8/8/8/8/8/6QQ/6kK w - - 99 99", Strict).unwrap();
    assert!(!pos.legal_moves_slow().is_empty());
    for c in pos.children() {
        assert_eq!(c.player_result_slow(&NoHistory::default()), Some(PlayerResult::Draw));
    }

    let mut hist = ZobristHistory::default();
    let mut pos = FairyBoard::from_fen_for("atomic", "8/8/8/4K2Q/8/8/8/1k6 w - - 0 1", Strict).unwrap();
    hist.push(pos.hash_pos());
    for m in ["h5h4", "b1a1", "h4h5", "a1b1", "h5h4", "b1a1", "h4h5"] {
        pos = pos.make_move_from_str(m).unwrap();
        assert!(pos.player_result_slow(&hist).is_none());
        hist.push(pos.hash_pos());
    }
    println!("{pos}");
    pos = pos.make_move_from_str("a1b1").unwrap();
    assert_eq!(pos.player_result_slow(&hist), Some(PlayerResult::Draw));
}
