#[cfg(feature = "caps")]
pub mod caps;

#[cfg(test)]
mod tests {
    use rand::rngs::StdRng;
    use std::str::FromStr;

    use gears::games::chess::moves::{ChessMove, ChessMoveFlags};
    use gears::games::chess::squares::ChessSquare;
    use gears::games::chess::Chessboard;
    use gears::games::{n_fold_repetition, BoardHistory, ZobristHistory};
    use gears::general::board::Board;
    use gears::general::moves::Move;
    use gears::score::{Score, SCORE_LOST, SCORE_WON};
    use gears::search::{Depth, SearchLimit};
    use gears::ugi::parse_ugi_position_and_moves;
    use gears::PlayerResult::Draw;

    use crate::eval::chess::lite::LiTEval;
    use crate::eval::chess::material_only::MaterialOnlyEval;
    use crate::eval::chess::piston::PistonEval;
    use crate::eval::rand_eval::RandEval;
    use crate::search::chess::caps::Caps;
    use crate::search::generic::gaps::Gaps;
    use crate::search::generic::random_mover::RandomMover;
    use crate::search::tt::TT;
    use crate::search::{Engine, SearchParams};

    #[test]
    fn generic_negamax_test() {
        generic_search_test(Gaps::<Chessboard>::default());
    }

    #[test]
    fn caps_search_test() {
        generic_search_test(Caps::for_eval::<PistonEval>());
    }

    #[test]
    fn random_mover_test() {
        mated_test(RandomMover::<Chessboard, StdRng>::default());
    }

    fn mated_test<E: Engine<Chessboard>>(mut engine: E) {
        let game_over_pos = parse_ugi_position_and_moves(
            &mut "mate_in_1 moves h7a7".split_whitespace().peekable(),
            &Chessboard::default(),
        )
        .unwrap();
        println!("{game_over_pos}");
        assert!(game_over_pos.is_game_lost_slow());
        for i in 1..123 {
            let res = engine.search_with_new_tt(game_over_pos, SearchLimit::depth(Depth::new(i)));
            assert!(res.ponder_move.is_none());
            assert_eq!(res.chosen_move, ChessMove::default());
            let res = engine.search_with_new_tt(game_over_pos, SearchLimit::nodes_(i as u64));
            assert!(res.ponder_move.is_none());
            assert_eq!(res.chosen_move, ChessMove::default());
        }
    }

    fn generic_search_test<E: Engine<Chessboard>>(mut engine: E) {
        let fen = "7r/pBrkqQ1p/3b4/5b2/8/6P1/PP2PP1P/R1BR2K1 w - - 1 17";
        let board = Chessboard::from_fen(fen).unwrap();
        let res = engine.search_with_new_tt(board, SearchLimit::mate(Depth::new(5)));
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

        mated_test(engine);
    }

    #[test]
    fn weird_position_test() {
        // this fen is actually a legal chess position
        let fen = "q2k2q1/2nqn2b/1n1P1n1b/2rnr2Q/1NQ1QN1Q/3Q3B/2RQR2B/Q2K2Q1 w - - 0 1";
        let board = Chessboard::from_fen(fen).unwrap();
        let mut engine = Caps::for_eval::<LiTEval>();
        let res = engine.search_with_new_tt(board, SearchLimit::nodes_(5_000));
        let score = res.score.unwrap();
        assert!(res.score.unwrap() >= Score(1400), "{score}");
        // not a legal chess position, but search with random eval should handle this
        let fen = "RRRRRRRR/RRRRRRRR/BBBBBBBB/BBBBBBBB/QQQQQQQQ/QQQQQQQQ/QPPPPPPP/K6k b - - 0 1";
        let board = Chessboard::from_fen(fen).unwrap();
        assert_eq!(board.pseudolegal_moves().len(), 3);
        for i in (2..55).step_by(3) {
            // do this several times to get different random numbers
            let mut engine = Caps::for_eval::<RandEval>();
            let res = engine.search_with_new_tt(board, SearchLimit::depth(Depth::new(i)));
            assert_eq!(res.score.unwrap(), SCORE_LOST + 2);
            assert_eq!(res.chosen_move.to_string(), "h1g1");
        }
    }

    #[test]
    fn repetition_test() {
        // the only winning move leads to a repeated position; all other moves lose
        let mut board = Chessboard::from_fen("8/8/3k3q/8/1K6/8/8/R7 w - - 0 1").unwrap();
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
        assert!(new_board
            .player_result_slow(&hist)
            .is_some_and(|r| r == Draw));
        assert!(n_fold_repetition(
            2,
            &hist,
            &new_board,
            new_board.halfmove_repetition_clock(),
        ));
        hist.pop();
        let mut engine = Caps::for_eval::<MaterialOnlyEval>();
        for depth in 1..10 {
            let res = engine.search(SearchParams::new_simple(
                board,
                SearchLimit::depth(Depth::new(depth)),
                hist.clone(),
                TT::default(),
            ));
            assert_eq!(res.chosen_move, mov);
            assert_eq!(res.score.unwrap(), Score(0));
        }
    }
}
