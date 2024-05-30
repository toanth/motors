#[cfg(feature = "caps")]
pub mod caps;

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use gears::games::chess::moves::{ChessMove, ChessMoveFlags};
    use gears::games::chess::squares::ChessSquare;
    use gears::games::chess::Chessboard;
    use gears::games::{Board, Move};
    use gears::search::{Depth, Score, SearchLimit, SCORE_LOST, SCORE_WON};

    use crate::eval::chess::hce::HandCraftedEval;
    use crate::eval::rand_eval::RandEval;
    use crate::search::chess::caps::Caps;
    use crate::search::generic::generic_negamax::GenericNegamax;
    use crate::search::Engine;

    #[test]
    fn generic_negamax_test() {
        generic_search_test::<GenericNegamax<Chessboard, RandEval>>()
    }

    #[test]
    fn caps_search_test() {
        generic_search_test::<Caps<RandEval>>()
    }

    fn generic_search_test<E: Engine<Chessboard>>() {
        let fen = "7r/pBrkqQ1p/3b4/5b2/8/6P1/PP2PP1P/R1BR2K1 w - - 1 17";
        let board = Chessboard::from_fen(fen).unwrap();
        let mut engine = E::default();
        let res = engine
            .search_from_pos(board, SearchLimit::mate(Depth::new(5)))
            .unwrap();
        assert_eq!(
            res.chosen_move,
            ChessMove::new(
                ChessSquare::from_str("d1").unwrap(),
                ChessSquare::from_str("d6").unwrap(),
                ChessMoveFlags::Normal
            )
        );
        assert!(res.score.is_some());
        assert_eq!(res.score.unwrap(), SCORE_WON - 3);
    }

    #[test]
    fn weird_position_test() {
        // this fen is actually a legal chess position
        let fen = "q2k2q1/2nqn2b/1n1P1n1b/2rnr2Q/1NQ1QN1Q/3Q3B/2RQR2B/Q2K2Q1 w - - 0 1";
        let board = Chessboard::from_fen(fen).unwrap();
        let mut engine = Caps::<HandCraftedEval>::default();
        let res = engine
            .search_from_pos(board, SearchLimit::nodes_(5_000))
            .unwrap();
        assert!(res.score.unwrap() >= Score(1400));
        // not a legal chess position, but search with random eval should handle this
        let fen = "RRRRRRRR/RRRRRRRR/BBBBBBBB/BBBBBBBB/QQQQQQQQ/QQQQQQQQ/QPPPPPPP/K6k b - - 0 1";
        let board = Chessboard::from_fen(fen).unwrap();
        assert_eq!(board.pseudolegal_moves().len(), 3);
        for i in (2..100).step_by(3) {
            // do this several times to get different random numbers
            let mut engine = Caps::<RandEval>::default();
            let res = engine
                .search_from_pos(board, SearchLimit::depth(Depth::new(i)))
                .unwrap();
            assert_eq!(res.score.unwrap(), SCORE_LOST + 2);
            assert_eq!(res.chosen_move.to_compact_text(), "h1g1");
        }
    }
}
