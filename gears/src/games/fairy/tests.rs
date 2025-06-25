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
#[cfg(test)]
mod atomic_tests;
#[cfg(test)]
mod chess_tests;

#[cfg(test)]
mod general {
    use crate::PlayerResult::{Draw, Lose};
    use crate::games::ataxx::AtaxxBoard;
    use crate::games::chess::Chessboard;
    use crate::games::fairy::Side::Kingside;
    use crate::games::fairy::attacks::MoveKind;
    use crate::games::fairy::moves::FairyMove;
    use crate::games::fairy::pieces::ColoredPieceId;
    use crate::games::fairy::{FairyBoard, FairyCastleInfo, FairyColor, FairyPiece, FairySquare};
    use crate::games::mnk::MNKBoard;
    use crate::games::{AbstractPieceType, BoardHistory, Color, Height, NoHistory, Width, ZobristHistory, chess};
    use crate::general::bitboards::{Bitboard, RawBitboard};
    use crate::general::board::Strictness::{Relaxed, Strict};
    use crate::general::board::{BitboardBoard, Board, BoardHelpers, UnverifiedBoard};
    use crate::general::moves::Move;
    use crate::general::perft::perft;
    use crate::general::squares::GridSize;
    use crate::search::Depth;
    use crate::{GameOverReason, GameResult, MatchResult};
    use itertools::Itertools;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use std::str::FromStr;

    #[test]
    fn simple_chess_startpos_test() {
        let fen = chess::START_FEN;
        let pos = FairyBoard::from_fen(fen, Strict).unwrap();
        let as_fen = pos.as_fen();
        assert_eq!("chess ".to_string() + fen, as_fen);
        let size = pos.size();
        assert_eq!(size, GridSize::new(Height(8), Width(8)));
        assert_eq!(pos.royal_bb().num_ones(), 2);
        assert_eq!(pos.active_player(), FairyColor::first());
        assert_eq!(pos.occupied_bb().num_ones(), 32);
        assert_eq!(pos.empty_bb().num_ones(), 32);
        assert_eq!(pos.player_bb(FairyColor::first()).raw(), 0xffff);
        let capture_bb = pos.capturing_attack_bb_of(FairyColor::first());
        assert_eq!(capture_bb.raw(), 0xff_ff_ff - 0x81);
        assert_eq!(22, capture_bb.num_ones());
        assert_eq!(22, pos.capturing_attack_bb_of(FairyColor::second()).num_ones());
        assert_eq!(pos.legal_moves_slow().len(), 20);
    }

    #[test]
    fn chess_makemove_test() {
        let chesspos = Chessboard::from_name("kiwipete").unwrap();
        let fen = chesspos.as_fen();
        let pos = FairyBoard::from_fen(&fen, Strict).unwrap();
        assert_eq!(pos.as_fen(), "chess ".to_string() + &fen);
        let moves = pos.legal_moves_slow();
        let chessmoves = chesspos.legal_moves_slow().into_iter().collect_vec();
        let num_castling = moves.iter().filter(|m| matches!(m.kind(), MoveKind::Castle(_))).count();
        assert_eq!(num_castling, 2);
        assert_eq!(moves.len(), chessmoves.len());
        for mov in moves {
            let new_pos = pos.clone().make_move(mov).unwrap();
            println!("{new_pos} | {}", mov.compact_formatter(&pos));
            let chess_pos = chessmoves
                .iter()
                .map(|&m| chesspos.make_move(m).unwrap())
                .find(|p| p.as_fen() == new_pos.fen_no_rules())
                .unwrap();
            let roundtrip = FairyBoard::from_fen(&new_pos.as_fen(), Strict).unwrap();
            assert_eq!(roundtrip.compute_hash(), new_pos.compute_hash());
            assert_eq!(new_pos, roundtrip);
            assert_eq!(chess_pos.num_legal_moves(), new_pos.num_legal_moves());
        }
    }

    #[test]
    fn simple_ep_test() {
        let pos =
            FairyBoard::from_fen("r3k2r/p2pqpb1/bn2pnp1/2pPN3/1pB1P3/2N2Q1p/PPPB1PPP/R3K2R w HAha c6 0 2", Strict)
                .unwrap();
        let moves = pos.legal_moves_slow();
        let mov = FairyMove::from_compact_text("d5c6", &pos).unwrap();
        assert!(moves.into_iter().contains(&mov));
        let new_pos = pos.make_move(mov).unwrap();
        assert!(new_pos.0.ep.is_none());
        assert!(new_pos.is_empty(FairySquare::from_str("c5").unwrap()));
        let moves = new_pos.legal_moves_slow();
        let mov = FairyMove::from_compact_text("e7c5", &new_pos).unwrap();
        assert!(moves.contains(&mov));
    }

    #[test]
    fn simple_chess_perft_test() {
        for chess_pos in Chessboard::bench_positions() {
            let fairy_pos = FairyBoard::from_fen(&chess_pos.as_fen(), Strict).unwrap();
            println!("{chess_pos}");
            let max = if cfg!(debug_assertions) { 3 } else { 5 };
            for i in 1..max {
                let depth = Depth::new(i);
                let chess_perft = perft(depth, chess_pos, false);
                let fairy_perft = perft(depth, fairy_pos.clone(), false);
                assert_eq!(chess_perft.depth, fairy_perft.depth);
                assert_eq!(chess_perft.nodes, fairy_perft.nodes, "{chess_pos} with depth {depth}");
                assert!(chess_perft.time.as_millis() * 100 + 1000 > fairy_perft.time.as_millis());
            }
        }
    }

    #[test]
    fn simple_chess960_test() {
        let fen = "1rqbkrbn/1ppppp1p/1n6/2N3p1/p7/2P4P/PP1PPPPB/1RQBKR1N w FBfb - 0 10";
        let pos = FairyBoard::from_fen(fen, Strict).unwrap();
        let chess_pos = Chessboard::from_fen(fen, Strict).unwrap();
        assert_eq!(pos.as_fen(), "chess ".to_string() + fen);
        let moves = pos.legal_moves_slow();
        let mov = FairyMove::from_compact_text("e1f1", &pos).unwrap();
        assert!(moves.contains(&mov));
        assert_eq!(moves.len(), chess_pos.legal_moves_slow().len());
        let fen = "rbbqn1kr/pp2p1pp/6n1/2pp1p2/2P4P/P7/BP1PPPP1/1RBQNNKR b Hha - 1 9";
        let pos = FairyBoard::from_fen(fen, Strict).unwrap();
        let mov = FairyMove::from_compact_text("g8h8", &pos).unwrap();
        let moves = pos.legal_moves_slow();
        assert!(moves.contains(&mov));
    }

    #[test]
    fn chess_game_over_test() {
        let pos = "chess rnbqkbnr/2pp1ppp/pp6/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w - - 0 4";
        let pos = FairyBoard::from_fen(pos, Strict).unwrap();
        assert!(pos.match_result_slow(&ZobristHistory::default()).is_none());
        let pos = pos.make_move_from_str("h5f7").unwrap();
        assert_eq!(pos.player_result_slow(&ZobristHistory::default()), Some(Lose));
        let mut pos = FairyBoard::from_name("kiwipete").unwrap();
        let original = pos.clone();
        let mut hist = ZobristHistory::default();
        for _ in 0..2 {
            for mov in ["e1f1", "e8f8", "f1e1", "f8e8"] {
                hist.push(pos.hash_pos());
                let mov = FairyMove::from_compact_text(mov, &pos).unwrap();
                pos = pos.make_move(mov).unwrap();
                assert!(pos.player_result_slow(&hist).is_none());
            }
        }
        pos = pos.make_move_from_str("e1f1").unwrap();
        assert_ne!(pos.castling_info, original.castling_info);
        assert_ne!(pos.hash_pos(), original.hash_pos());
        assert!(pos.player_result_slow(&hist).is_none());
        pos = pos.make_move_from_str("e8f8").unwrap();
        assert_eq!(pos.player_result_slow(&hist), Some(Draw));
        let fen = "chess 8/3k4/7p/2p3pP/1pPp1pP1/pP1PpP2/P3P3/2K5 w - - 57 1";
        let mut rng = StdRng::seed_from_u64(42);
        let mut pos = FairyBoard::from_fen(fen, Strict).unwrap();
        for i in 0..42 {
            assert_eq!(pos.draw_counter, 57 + i);
            let mov = pos.random_legal_move(&mut rng).unwrap();
            pos = pos.make_move(mov).unwrap();
            assert!(pos.player_result_slow(&ZobristHistory::default()).is_none());
        }
        let mov = pos.random_legal_move(&mut rng).unwrap();
        pos = pos.make_move(mov).unwrap();
        assert_eq!(pos.player_result_slow(&ZobristHistory::default()), Some(Draw));
        let fen = "5B1k/5B2/7K/8/8/8/3K4/8 b - - 0 1";
        assert!(FairyBoard::from_fen_for("chess", fen, Relaxed).is_err());
        let fen = "5B1k/5B2/7K/8/8/8/8/8 b - - 0 1";
        let pos = FairyBoard::from_fen_for("chess", fen, Strict).unwrap();
        assert!(pos.has_no_legal_moves());
        assert_eq!(
            pos.match_result_slow(&ZobristHistory::default()),
            Some(MatchResult { result: GameResult::Draw, reason: GameOverReason::Normal })
        );
    }

    #[test]
    fn simple_shatranj_startpos_test() {
        let pos = FairyBoard::variant_simple("shatranj").unwrap();
        let as_fen = pos.fen_no_rules();
        assert_eq!(as_fen, pos.rules().startpos_fen_part);
        let size = pos.size();
        assert_eq!(size, GridSize::new(Height(8), Width(8)));
        assert_eq!(pos.royal_bb().num_ones(), 2);
        assert_eq!(pos.active_player(), FairyColor::first());
        assert_eq!(pos.occupied_bb().num_ones(), 32);
        assert_eq!(pos.empty_bb().num_ones(), 32);
        assert_eq!(pos.player_bb(FairyColor::first()).raw(), 0xffff);
        let capture_bb = pos.capturing_attack_bb_of(FairyColor::first());
        assert_eq!(capture_bb.raw(), 16760150);
        assert_eq!(18, capture_bb.num_ones());
        assert_eq!(18, pos.capturing_attack_bb_of(FairyColor::second()).num_ones());
        assert_eq!(pos.legal_moves_slow().len(), 8 + 2 * 2 + 2 * 2);
    }

    #[test]
    fn simple_shatranj_test() {
        let pos = FairyBoard::from_fen_for("shatranj", "5k2/6r1/8/8/8/6R1/5K2/8 w 0 1", Strict).unwrap();
        let new_pos = pos.make_move_from_str("g3g7").unwrap();
        assert_eq!(new_pos.player_result_slow(&NoHistory::default()), Some(Draw));
        let pos = FairyBoard::from_fen_for("shatranj", "4k3/6r1/8/8/8/6R1/5K2/8 w 0 1", Strict).unwrap();
        let new_pos = pos.make_move_from_str("g3g7").unwrap();
        assert_eq!(new_pos.player_result_slow(&NoHistory::default()), Some(Lose));
    }

    #[test]
    fn simple_koth_test() {
        let pos = FairyBoard::from_fen_for("kingofthehill", "2k5/8/8/8/3K4/8/8/8 b - - 0 1", Strict).unwrap();
        assert_eq!(pos.player_result_slow(&ZobristHistory::default()), Some(Lose));
        let pos = FairyBoard::from_fen_for("kingofthehill", "8/8/3k4/8/8/4K3/8/8 b - - 99 6", Strict).unwrap();
        let new_pos = pos.clone().make_move_from_str("d6e5").unwrap();
        assert_eq!(new_pos.player_result_slow(&ZobristHistory::default()), Some(Lose));
        let settings = pos.settings();
        let pos = pos
            .place_piece(FairyPiece::new(
                ColoredPieceId::from_name("Q", &settings).unwrap(),
                FairySquare::algebraic('h', 5).unwrap(),
            ))
            .unwrap()
            .verify(Strict)
            .unwrap();
        assert!(pos.make_move_from_str("d6e5").is_err());
    }

    #[test]
    fn simple_horde_test() {
        let pos = FairyBoard::from_fen_for(
            "horde",
            "r2qkb1r/1Pp3Pp/2n1Pp2/2P2PP1/PPPPppPP/4PPp1/PPp5/PP1PP1PP w kq - 0 1",
            Strict,
        )
        .unwrap();
        assert!(pos.player_result_slow(&ZobristHistory::default()).is_none());
        let pos = pos.make_move_from_str("h1h3").unwrap();
        assert!(pos.ep.is_none());
        assert!(pos.clone().make_move_from_str("g3h2").is_err());
        let pos = pos.make_move_from_str("c2b1r").unwrap();
        _ = pos.debug_verify_invariants(Strict).unwrap();
        let pos = FairyBoard::from_fen_for("horde", "8/8/2k5/2P5/8/8/8/8 b - - 0 1", Strict).unwrap();
        assert!(pos.is_game_won_after_slow(FairyMove::from_text("c6c5", &pos).unwrap(), NoHistory::default()));
        let pos = FairyBoard::from_fen_for("horde", "8/4P3/8/bb2P3/kb2B3/b1p5/2P1B3/1P3B2 w - - 0 1", Strict).unwrap();
        let pos = pos.make_move_from_str("b1b3").unwrap();
        assert_eq!(pos.player_result_slow(&ZobristHistory::default()), Some(Lose));
    }

    #[test]
    fn simple_racing_kings_test() {
        let pos = FairyBoard::from_fen_for("racingkings", "8/7K/1k6/8/8/8/8/3R4 w - - 0 1", Strict).unwrap();
        assert!(pos.player_result_slow(&ZobristHistory::default()).is_none());
        let pos = pos.make_move_from_str("h7h8").unwrap();
        assert_eq!(pos.player_result_slow(&ZobristHistory::default()), Some(Lose));
        let pos = FairyBoard::from_fen_for("racingkings", "8/1k5K/4r3/8/8/8/8/3R4 w - - 0 1", Strict).unwrap();
        assert!(pos.player_result_slow(&ZobristHistory::default()).is_none());
        let new_pos = pos.clone().make_move_from_str("h7h8").unwrap();
        assert!(new_pos.player_result_slow(&ZobristHistory::default()).is_none());
        assert!(pos.clone().make_move_from_str("d1b1").is_err());
        assert!(pos.clone().make_move_from_str("h7h6").is_err());
        let pos = pos.make_nullmove().unwrap();
        let new_pos = pos.make_move_from_str("b7b8").unwrap();
        assert_eq!(new_pos.player_result_slow(&ZobristHistory::default()), Some(Lose));
    }

    #[test]
    fn simple_crazyhouse_test() {
        let pos = FairyBoard::variant_simple("crazyhouse").unwrap();
        assert!(pos.player_result_slow(&ZobristHistory::default()).is_none());
        assert_eq!(pos.num_legal_moves(), 20);
        let pos = FairyBoard::from_fen_for("crazyhouse", "k3R3/8/K7/8/8/8/8/8[n] b - - 0 1", Strict).unwrap();
        assert!(pos.is_in_check());
        assert_eq!(pos.num_legal_moves(), 3);
        assert!(pos.player_result_slow(&ZobristHistory::default()).is_none());
        let pos = pos.make_move_from_str("N@b8").unwrap();
        assert!(pos.is_in_check());
        let pos = FairyBoard::from_fen_for(
            "crazyhouse",
            "2kr1bQ~r/ppp1pp2/2n5/8/2q3b1/8/PPPP2PP/RNBQK1NR[PPPNpb] b KQ - 0 8",
            Strict,
        )
        .unwrap();
        let num_moves = pos.num_legal_moves();
        assert_eq!(num_moves, 128);
        let pos = pos.flip_side_to_move().unwrap();
        assert_eq!(pos.fen_no_rules(), "2kr1bQ~r/ppp1pp2/2n5/8/2q3b1/8/PPPP2PP/RNBQK1NR[NPPPbp] w KQ - 0 9");
        assert_eq!(pos, FairyBoard::from_fen(&pos.as_fen(), Strict).unwrap());
        assert_eq!(pos.num_legal_moves(), 99);
        let pos = FairyBoard::from_fen_for(
            "crazyhouse",
            "r1bqkbB~r/p1ppp3/1p6/8/8/5Q2/PPPP1PPP/RNB1KBNR[PPPNN] w KQkq - 0 7",
            Strict,
        )
        .unwrap();
        assert!(pos.is_game_won_after_slow(FairyMove::from_compact_text("f3f7", &pos).unwrap(), NoHistory::default()));
        assert!(pos.is_game_won_after_slow(FairyMove::from_compact_text("g8f7", &pos).unwrap(), NoHistory::default()));
        assert!(pos.is_game_won_after_slow(FairyMove::from_compact_text("P@f7", &pos).unwrap(), NoHistory::default()));
        let pos = FairyBoard::from_fen_for(
            "crazyhouse",
            "r1bqk2r~/p1p1p3/1p3p2/3p4/3P2p1/2N2N2/PP1BBPPP/R~3K2R[PPPNN] b KQkq - 0 7",
            Strict,
        )
        .unwrap();
        assert!(pos.castling_info.can_castle(FairyColor::second(), Kingside));
        let pos = pos.make_move_from_str("e8h8").unwrap();
        let pos = pos.make_move_from_str("e1h1").unwrap();
        assert_eq!(pos.castling_info, FairyCastleInfo::default());
        let pos = FairyBoard::from_fen_for("crazyhouse", "6r~1/7P/3k4/8/3K4/8/8/8[] w - - 0 1", Strict).unwrap();
        for m in pos.legal_moves_slow() {
            if m.is_capture() {
                println!("{}", m.compact_formatter(&pos));
            }
        }
        let pos = pos.make_move_from_str("h7g8q").unwrap();
        let pos = pos.flip_side_to_move().unwrap();
        for m in pos.legal_moves_slow() {
            if m.src_square_in(&pos).is_none() {
                assert_eq!(m.piece(&pos).name(&pos.rules).as_ref(), "white pawn");
            }
        }
        let pos = FairyBoard::from_fen("crazyhouse 7n/5p2/6p1/8/8/k7/7p/1K1Q4[] b - - 1 1", Strict).unwrap();
        let pos = pos.make_move_from_str("h2h1q").unwrap();
        assert_eq!(pos.fen_no_rules(), "7n/5p2/6p1/8/8/k7/8/1K1Q3q~[] w - - 0 2");
    }

    #[test]
    fn simple_ataxx_test() {
        for pos in AtaxxBoard::bench_positions() {
            let fen = pos.as_fen();
            println!("{fen}");
            let fairy_pos = FairyBoard::from_fen_for("ataxx", &fen, Strict).unwrap();
            let fairy_normal_part = fairy_pos.fen_no_rules();
            assert_eq!(fairy_normal_part, fen);
            let fairy_fen = fairy_pos.as_fen();
            assert_eq!(FairyBoard::from_fen(&fairy_fen, Strict).unwrap(), fairy_pos);
            assert_eq!(fairy_pos.empty_bb().num_ones(), pos.empty_bb().num_ones());
            assert_eq!(fairy_pos.active_player_bb().num_ones(), pos.active_player_bb().num_ones());
            assert_eq!(fairy_pos.active_player_bb().num_ones(), pos.active_player_bb().num_ones());
            assert_eq!(fairy_pos.num_legal_moves(), pos.num_legal_moves());
            for i in 1..=3 {
                let perft_res = perft(Depth::new(i), pos, false);
                let fairy_perft_res = perft(Depth::new(i), fairy_pos.clone(), false);
                assert_eq!(perft_res.depth, fairy_perft_res.depth, "{i} {pos}");
                assert_eq!(perft_res.nodes, fairy_perft_res.nodes, "{i} {pos}");
            }
        }
    }

    #[test]
    fn simple_mnk_test() {
        let pos = FairyBoard::from_fen("tictactoe 3 3 3 3/3/3 x 1", Strict).unwrap();
        assert_eq!(pos.size(), GridSize::tictactoe());
        assert_eq!(pos.active_player(), FairyColor::from_char('x', &pos.settings()).unwrap());
        assert!(pos.royal_bb().is_zero());
        assert_eq!(pos.empty_bb().num_ones(), 9);
        assert_eq!(pos.num_legal_moves(), 9);
        let mov = FairyMove::from_compact_text("a1", &pos).unwrap();
        let pos = pos.make_move(mov).unwrap();
        assert_eq!(pos.empty_bb().num_ones(), 8);
        assert_eq!(pos.num_legal_moves(), 8);
        assert_eq!(pos.as_fen(), "mnk 3 3 3 3/3/X2 o 1");
        let mov = FairyMove::from_compact_text("c2", &pos).unwrap();
        let pos = pos.make_move(mov).unwrap();
        assert_eq!(pos.num_legal_moves(), 7);
        assert_eq!(pos.as_fen(), "mnk 3 3 3 3/2O/X2 x 2");
        assert_eq!(pos.last_move, mov);
        let pos = FairyBoard::from_fen_for("mnk", "5 5 4 X4/O4/O2X1/O1X2/OX3 x 5", Strict).unwrap();
        assert!(pos.is_game_lost_slow(&NoHistory::default()));
        assert!(pos.cannot_call_movegen());
        // TODO: panic when starting search in won position
    }

    #[test]
    fn simple_mnk_perft_test() {
        for mnk_pos in MNKBoard::bench_positions() {
            let fairy_pos = FairyBoard::from_fen_for("mnk", &mnk_pos.as_fen(), Strict).unwrap();
            println!("{mnk_pos}");
            let max = if cfg!(debug_assertions) { 4 } else { 6 };
            for i in 1..max {
                let depth = Depth::new(i);
                let mnk_perft = perft(depth, mnk_pos, false);
                let fairy_perft = perft(depth, fairy_pos.clone(), false);
                assert_eq!(mnk_perft.depth, fairy_perft.depth);
                assert_eq!(mnk_perft.nodes, fairy_perft.nodes, "Depth {i}, pos: {mnk_pos}");
            }
        }
    }
}
