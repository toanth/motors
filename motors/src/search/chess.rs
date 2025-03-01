#[cfg(feature = "caps")]
pub mod caps;
mod caps_values;

#[cfg(test)]
mod tests {
    use gears::rand::rngs::StdRng;
    use std::str::FromStr;
    use std::sync::Arc;
    use std::sync::atomic::Ordering::SeqCst;
    use std::sync::atomic::fence;
    use std::thread::{sleep, spawn};
    use std::time::Duration;

    use gears::PlayerResult::Draw;
    use gears::games::chess::Chessboard;
    use gears::games::chess::moves::{ChessMove, ChessMoveFlags};
    use gears::games::chess::pieces::ChessPieceType::Bishop;
    use gears::games::chess::squares::ChessSquare;
    use gears::games::{BoardHistory, ZobristHistory, n_fold_repetition};
    use gears::general::board::Board;
    use gears::general::board::Strictness::{Relaxed, Strict};
    use gears::general::common::tokens;
    use gears::general::moves::Move;
    use gears::output::pgn::parse_pgn;
    use gears::score::{SCORE_LOST, SCORE_WON, Score};
    use gears::search::{Depth, SearchLimit};
    use gears::ugi::load_ugi_position;

    use crate::eval::chess::lite::LiTEval;
    use crate::eval::chess::material_only::MaterialOnlyEval;
    use crate::eval::chess::piston::PistonEval;
    use crate::eval::rand_eval::RandEval;
    use crate::search::chess::caps::Caps;
    use crate::search::generic::gaps::Gaps;
    use crate::search::generic::random_mover::RandomMover;
    use crate::search::multithreading::AtomicSearchState;
    use crate::search::tt::TT;
    use crate::search::{Engine, SearchParams};

    #[test]
    #[cfg(feature = "gaps")]
    fn generic_negamax_test() {
        generic_search_test(Gaps::<Chessboard>::default());
    }

    #[test]
    #[cfg(feature = "caps")]
    fn caps_search_test() {
        generic_search_test(Caps::for_eval::<PistonEval>());
    }

    #[test]
    #[cfg(feature = "random_mover")]
    fn random_mover_test() {
        mated_test(&mut RandomMover::<Chessboard, StdRng>::default());
    }

    fn mated_test<E: Engine<Chessboard>>(engine: &mut E) {
        let game_over_pos = load_ugi_position(
            "position",
            &mut tokens("mate_in_1 moves h7a7"),
            true,
            Strict,
            &Chessboard::default(),
        )
        .unwrap();
        assert!(game_over_pos.is_game_lost_slow());
        for i in (1..123).step_by(11) {
            let res = engine
                .search_with_new_tt(game_over_pos, SearchLimit::depth(Depth::new_unchecked(i)));
            assert!(res.ponder_move.is_none());
            assert_eq!(res.chosen_move, ChessMove::default());
            let res = engine.search_with_new_tt(game_over_pos, SearchLimit::nodes_(i as u64));
            assert!(res.ponder_move.is_none());
            assert_eq!(res.chosen_move, ChessMove::default());
        }
    }

    fn generic_search_test<E: Engine<Chessboard>>(mut engine: E) {
        let fen = "7r/pBrkqQ1p/3b4/5b2/8/6P1/PP2PP1P/R1BR2K1 w - - 1 17";
        let board = Chessboard::from_fen(fen, Strict).unwrap();
        let res = engine.search_with_new_tt(board, SearchLimit::mate(Depth::new_unchecked(5)));
        assert_eq!(
            res.chosen_move,
            ChessMove::new(
                ChessSquare::from_str("d1").unwrap(),
                ChessSquare::from_str("d6").unwrap(),
                ChessMoveFlags::RookMove
            )
        );
        assert!(res.score.is_some());
        assert_eq!(res.score.unwrap(), SCORE_WON - 3);

        mated_test(&mut engine);
        avoid_repetition(&mut engine);
        mate_beats_repetition(&mut engine);

        two_threads_test::<E>();
    }

    fn avoid_repetition<E: Engine<Chessboard>>(engine: &mut E) {
        let pgn = r#"[Variant "From Position"][FEN "8/3Q4/2K5/k7/6P1/8/8/8 w - - 0 1"]
                        1. Qd4 Ka6 2. Qd6 Ka5 3. Qd4 Ka6 4. Qd7 Ka5"#;
        let game = parse_pgn::<Chessboard>(pgn).unwrap().game;
        let params = SearchParams::new_unshared(
            game.board,
            SearchLimit::depth(engine.default_bench_depth()),
            game.board_hist,
            TT::default(),
        );
        let res = engine.search(params);
        if let Some(plies_to_win) = res.score.unwrap().plies_until_game_won() {
            assert!(plies_to_win > 4);
        } else {
            assert!(res.score.unwrap() >= Score(500));
        }
        assert_ne!(
            res.chosen_move,
            ChessMove::from_text("Qd4", &game.board).unwrap()
        );
    }

    fn mate_beats_repetition<E: Engine<Chessboard>>(engine: &mut E) {
        let pos = Chessboard::from_fen("8/3Q4/2K5/k7/6P1/8/8/8 w - - 99 99", Strict).unwrap();
        let res = engine.search_with_new_tt(pos, SearchLimit::depth(engine.default_bench_depth()));
        let score = res.score.unwrap();
        assert!(score > Score(500));
        if let Some(plies_to_win) = res.score.unwrap().plies_until_game_won() {
            assert!(plies_to_win > 2);
        }
        assert_eq!(res.chosen_move, ChessMove::from_text("g5!", &pos).unwrap());
        let pos = Chessboard::from_fen("8/8/k1K5/8/8/8/3Q2P1/8 w - - 99 99", Strict).unwrap();
        let res = engine.search_with_new_tt(pos, SearchLimit::depth_(3));
        let score = res.score.unwrap();
        assert_eq!(score.plies_until_game_won(), Some(1), "{score}");
        assert_eq!(res.chosen_move, ChessMove::from_text("d2a2", &pos).unwrap());
    }

    fn two_threads_test<E: Engine<Chessboard>>() {
        let fen = "2kr3r/2pb1p2/p2b1p2/1p4pp/B2R4/2P1P2P/PP2KPP1/R1B5 w - - 0 16";
        let board = Chessboard::from_fen(fen, Strict).unwrap();
        let mut engine = E::for_eval::<LiTEval>();
        let mut engine2 = E::for_eval::<LiTEval>();
        let tt = TT::default();
        let atomic = Arc::new(AtomicSearchState::default());
        let atomic2 = Arc::new(AtomicSearchState::default());
        let params = SearchParams::with_atomic_state(
            board,
            SearchLimit::infinite(),
            ZobristHistory::default(),
            tt.clone(),
            atomic.clone(),
        );
        let params2 = SearchParams::with_atomic_state(
            board,
            SearchLimit::infinite(),
            ZobristHistory::default(),
            tt.clone(),
            atomic2.clone(),
        );
        let handle = spawn(move || engine.search(params));
        let handle2 = spawn(move || engine2.search(params2));
        sleep(Duration::from_millis(500));
        atomic.set_stop(true);
        assert!(atomic.stop_flag());
        assert!(!atomic2.stop_flag());
        let res = handle.join().unwrap();
        assert!(!atomic.currently_searching());
        assert!(atomic2.currently_searching());
        fence(SeqCst);
        atomic2.set_stop(true);
        let res2 = handle2.join().unwrap();
        // The bound of 400 is rather large because gaps does not produce very stable evals
        assert!(
            res.score.unwrap().0.abs_diff(res2.score.unwrap().0) <= 400,
            "{0} {1}",
            res.score.unwrap(),
            res2.score.unwrap()
        );
        assert_eq!(res.chosen_move.piece_type(), Bishop);
        assert_eq!(
            res2.chosen_move.src_square(),
            ChessSquare::from_str("a4").unwrap()
        );
    }

    #[test]
    fn weird_position_test() {
        // this fen is actually a legal chess position
        let fen = "q2k2q1/2nqn2b/1n1P1n1b/2rnr2Q/1NQ1QN1Q/3Q3B/2RQR2B/Q2K2Q1 w - - 0 1";
        let board = Chessboard::from_fen(fen, Strict).unwrap();
        let mut engine = Caps::for_eval::<LiTEval>();
        // TODO: New testcase that asserts that unfinished iterations can still change the score
        let res = engine.search_with_new_tt(board, SearchLimit::depth_(1));
        // let res = engine.search_with_new_tt(board, SearchLimit::nodes_(5_000));
        let score = res.score.unwrap();
        assert!(res.score.unwrap() >= Score(1400), "{score}");
        // not a legal chess position, but search should still handle this
        let fen = "RRRRRRRR/RRRRRRRR/BBBBBBBB/BBBBBBBB/QQQQQQQQ/QQQQQQQQ/QPPPPPPP/K6k b - - 0 1";
        let board = Chessboard::from_fen(fen, Relaxed).unwrap();
        assert_eq!(board.pseudolegal_moves().len(), 3);
        for i in (2..55).step_by(3) {
            // do this several times to get different random numbers
            let mut engine = Caps::for_eval::<RandEval>();
            let res = engine.search_with_new_tt(board, SearchLimit::depth(Depth::new_unchecked(i)));
            assert_eq!(res.score.unwrap(), SCORE_LOST + 2);
            assert_eq!(res.chosen_move.to_string(), "h1g1");
        }
        let mut engine = Caps::for_eval::<LiTEval>();
        let res = engine.search_with_new_tt(board, SearchLimit::depth_(10));
        assert_eq!(res.score.unwrap(), SCORE_LOST + 2);
        assert_eq!(res.chosen_move.to_string(), "h1g1");
    }

    #[test]
    fn repetition_test() {
        // the only winning move leads to a repeated position; all other moves lose
        let mut board = Chessboard::from_fen("8/8/3k3q/8/1K6/8/8/R7 w - - 0 1", Strict).unwrap();
        let movelist = ["Ra6+", "Kd7", "Ra1", "Kd6"];
        let mut hist = ZobristHistory::default();
        for _ in 0..2 {
            for mov in movelist {
                let mov = ChessMove::from_extended_text(mov, &board).unwrap();
                board = board.make_move(mov).unwrap();
                assert!(board.player_result_slow(&hist).is_none());
                hist.push(&board);
            }
        }
        let mov = ChessMove::from_extended_text(movelist[0], &board).unwrap();
        let new_board = board.make_move(mov).unwrap();
        assert!(new_board.is_in_check());
        assert!(new_board.is_3fold_repetition(&hist));
        assert!(
            new_board
                .player_result_slow(&hist)
                .is_some_and(|r| r == Draw)
        );
        assert!(n_fold_repetition(
            2,
            &hist,
            &new_board,
            new_board.halfmove_repetition_clock(),
        ));
        hist.pop();
        let mut engine = Caps::for_eval::<MaterialOnlyEval>();
        for depth in 1..10 {
            let res = engine.search(SearchParams::new_unshared(
                board,
                SearchLimit::depth(Depth::new_unchecked(depth)),
                hist.clone(),
                TT::default(),
            ));
            assert_eq!(res.chosen_move, mov);
            assert_eq!(res.score.unwrap(), Score(0));
        }
    }
}
