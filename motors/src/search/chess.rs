use crate::search::chess::histories::{
    CaptHist, ContHist, CorrHist, HIST_DIVISOR, HistoryHeuristic, write_single_hist_table,
};
use crate::search::{CustomInfo, MoveScore, Pv, SearchStackEntry, SearchState};
use gears::arrayvec::ArrayVec;
use gears::games::PosHash;
use gears::games::chess::moves::Move;
use gears::games::chess::squares::NUM_SQUARES;
use gears::games::chess::{Board, Color, MAX_CHESS_MOVES_IN_POS};
use gears::general::board::BoardTrait;
use gears::score::Score;
use gears::search::DepthPly;
use std::hash::{Hash, Hasher};

pub mod caps;
mod caps_values;
mod histories;
mod move_picker;

/// By how much the fractional depth increases each ID iteration.
const DEPTH_INCREMENT: usize = 128;

/// The maximum value of the uci `depth` parameter, i.e. the maximum number of Iterative Deepening iterations
const ID_ITERS_SOFT_LIMIT: DepthPly = DepthPly::new(225);
/// The maximum value of the `ply` parameter in main search, i.e. the maximum depth (in plies) before qsearch is reached
const PLY_HARD_LIMIT: usize = 255;

/// Qsearch can go more than 30 plies deeper than the depth hard limit if ther's more material on the board; in that case we simply
/// return the static eval.
const SEARCH_STACK_LEN: usize = PLY_HARD_LIMIT + 30;

/// The TT move and good captures have a higher score, all other moves have a lower score.
const KILLER_SCORE: MoveScore = MoveScore(8 * HIST_DIVISOR);

#[derive(Debug, Default, Clone)]
pub struct CapsSearchStackEntry {
    killer: Move,
    pv: Pv<Board, SEARCH_STACK_LEN>,
    tried_moves: ArrayVec<Move, MAX_CHESS_MOVES_IN_POS>,
    move_score: MoveScore,
    pos: Board,
    eval: Score,
}

impl SearchStackEntry<Board> for CapsSearchStackEntry {
    fn forget(&mut self) {
        self.killer = Move::default();
        self.pv.list.clear();
        self.tried_moves.clear();
        self.move_score = MoveScore(0);
        self.pos = Board::default();
        self.eval = Score::default();
    }

    fn pv(&self) -> Option<&[Move]> {
        Some(self.pv.list.as_slice())
    }

    fn last_played_move(&self) -> Option<Move> {
        self.tried_moves.last().copied()
    }

    fn hash(&self, hasher: &mut impl Hasher) {
        self.killer.hash(hasher);
        self.tried_moves.hash(hasher);
        self.move_score.0.hash(hasher);
        self.eval.hash(hasher);
        self.pos.hash_pos().0.hash(hasher);
    }
}

impl CapsSearchStackEntry {
    /// If this entry has a lower ply number than the current node, this is the tree edge that leads towards the current node.
    fn last_tried_move(&self) -> Move {
        *self.tried_moves.last().unwrap()
    }
}

#[derive(Debug, Clone, Default)]
pub struct CapsCustomInfo {
    history: HistoryHeuristic,
    /// Many moves have a "natural" response, so use that for move ordering:
    /// Instead of only learning which quiet moves are good, learn which quiet moves are good after our
    /// opponent played a given move.
    countermove_hist: ContHist,
    /// Often, a move works because it is immediately followed by some other move, which completes the tactic.
    /// Keep track of such quiet follow-up moves. This is exactly the same as the countermove history, but considers
    /// our previous move instead of the opponent's previous move, i.e. the move 2 plies ago instead of 1 ply ago.
    follow_up_move_hist: ContHist,
    capt_hist: CaptHist,
    corr_hist: CorrHist,
    repeated_before_root: Vec<PosHash>,
    nmp_disabled: [bool; 2],
    ply_hard_limit: usize,
    root_move_nodes: RootMoveNodes,
}

impl CapsCustomInfo {
    fn nmp_disabled_for(&mut self, color: Color) -> &mut bool {
        &mut self.nmp_disabled[color]
    }
}

impl CustomInfo<Board> for CapsCustomInfo {
    fn new_search(&mut self) {
        debug_assert!(!self.nmp_disabled[0]);
        debug_assert!(!self.nmp_disabled[1]);
        // don't update history values, malus and gravity already take care of that
        self.root_move_nodes.clear();
    }

    fn hard_forget_except_tt(&mut self) {
        for value in self.history.iter_mut().flatten() {
            *value = 0;
        }
        self.capt_hist.reset();
        for value in self.countermove_hist.iter_mut() {
            *value = 0;
        }
        for value in self.follow_up_move_hist.iter_mut() {
            *value = 0;
        }
        self.corr_hist.reset();
        self.root_move_nodes.clear();
    }

    fn write_internal_info(&self, pos: &Board) -> Option<String> {
        Some(
            write_single_hist_table(&self.history, pos, false)
                + "\n"
                + &write_single_hist_table(&self.history, pos, true),
        )
    }
}

#[derive(Debug, Clone)]
struct RootMoveNodes(Box<[[u64; NUM_SQUARES]; NUM_SQUARES]>);

impl Default for RootMoveNodes {
    fn default() -> Self {
        RootMoveNodes(Box::new([[0; NUM_SQUARES]; NUM_SQUARES]))
    }
}

impl RootMoveNodes {
    fn clear(&mut self) {
        for elem in self.0.iter_mut() {
            *elem = [0; NUM_SQUARES];
        }
    }
    fn update(&mut self, mov: Move, nodes: u64) {
        self.0[mov.src_square().bb_idx()][mov.dest_square().bb_idx()] += nodes;
    }

    fn frac_1024(&self, best_move: Move, total_nodes: u64) -> u64 {
        self.0[best_move.src_square().bb_idx()][best_move.dest_square().bb_idx()] * 1024 / total_nodes
    }
}

pub type CapsState = SearchState<Board, CapsSearchStackEntry, CapsCustomInfo>;

#[cfg(test)]
mod tests {
    use crate::eval::chess::lite::{KingGambot, LiTEval};
    use crate::eval::chess::material_only::MaterialOnlyEval;
    use crate::eval::chess::piston::PistonEval;
    use crate::eval::rand_eval::RandEval;
    use crate::search::chess::caps::Caps;
    use crate::search::generic::gaps::Gaps;
    use crate::search::generic::random_mover::RandomMover;
    use crate::search::multithreading::AtomicSearchState;
    use crate::search::tt::TT;
    use crate::search::{AbstractSearchState, Engine, SearchParams};
    use crate::{list_chess_evals, list_chess_searchers};
    use gears::PlayerResult::{Draw, Win};
    use gears::games::chess::Board;
    use gears::games::chess::moves::{Move, MoveFlags};
    use gears::games::chess::pieces::ColoredPieceType::BlackKnight;
    use gears::games::chess::pieces::Piece;
    use gears::games::chess::pieces::PieceType::{Bishop, Knight};
    use gears::games::chess::squares::Square;
    use gears::games::{BoardHistDyn, ZobristHistory, n_fold_repetition};
    use gears::general::board::Strictness::{Relaxed, Strict};
    use gears::general::board::{BoardHelpers, BoardTrait, UnverifiedBoardTrait};
    use gears::general::common::NamedEntity;
    use gears::general::moves::MoveTrait;
    use gears::output::pgn::parse_pgn;
    use gears::parse_ugi_pos_and_hist;
    use gears::rand::prelude::SmallRng;
    use gears::score::{NO_SCORE_YET, SCORE_LOST, SCORE_WON, Score, game_result_to_score};
    use gears::search::{DepthPly, NodesLimit, SearchLimit};
    use gears::ugi::load_ugi_pos_simple;
    use std::str::FromStr;
    use std::sync::Arc;
    use std::sync::atomic::Ordering::SeqCst;
    use std::sync::atomic::fence;
    use std::thread::{sleep, spawn};
    use std::time::{Duration, Instant};

    #[test]
    #[cfg(feature = "gaps")]
    fn generic_negamax_test() {
        generic_search_test(Gaps::<Board>::default());
    }

    #[test]
    #[cfg(feature = "caps")]
    fn caps_search_test() {
        generic_search_test(Caps::for_eval::<PistonEval>());
    }

    #[test]
    fn random_mover_test() {
        game_over_test(&mut RandomMover::<Board, SmallRng>::default());
    }

    fn game_over_test<E: Engine<Board>>(engine: &mut E) {
        let mated_pos = load_ugi_pos_simple("mate_in_1 moves h7a7", Strict, &Board::default()).unwrap();
        assert!(mated_pos.is_checkmate_slow());
        let tt = TT::minimal();
        for i in (1..123).step_by(11) {
            let res = engine.search_with_tt(mated_pos, SearchLimit::depth(DepthPly::new(i)), tt.clone());
            assert!(res.ponder_move.is_none());
            assert_eq!(res.chosen_move, Move::default());
            let res = engine.search_with_tt(mated_pos, SearchLimit::nodes_(i as u64), tt.clone());
            assert!(res.ponder_move.is_none());
            assert_eq!(res.chosen_move, Move::default());
        }
        let fen = "QQQQQQQQ/QQQQQQQQ/QQQQQQQQ/QQQQQQQQ/QQQQQQQQ/QQQQQQQQ/QQQQQQNN/KQQQQQNk w - - 0 1";
        let drawn_pos = Board::from_fen(fen, Relaxed).unwrap();
        assert!(drawn_pos.is_stalemate_slow());
        let res = engine.search_with_new_tt(drawn_pos, SearchLimit::nodes_(42));
        assert_eq!(res.ponder_move, None);
        assert_eq!(res.chosen_move, Move::default());
        // TODO: Ensure that this returns 0 instead of SCORE_TIME_UP
        // assert_eq!(res.score, Score(0));
        let drawn_pos = drawn_pos.make_nullmove().unwrap();
        let res = engine.search_with_new_tt(drawn_pos, SearchLimit::nodes_(42));
        // assert_eq!(res.score, Score(0));
        assert_eq!(res.chosen_move, Move::default());
        let mut pos = drawn_pos.replace_piece(Piece::new(BlackKnight, Square::from_str("g1").unwrap())).unwrap();
        pos.try_replace_piece(Square::from_str("g2").unwrap(), BlackKnight).unwrap();
        pos.try_replace_piece(Square::from_str("h2").unwrap(), BlackKnight).unwrap();
        let drawn_pos = pos.verify(Relaxed).unwrap();
        let res = engine.search_with_new_tt(drawn_pos, SearchLimit::nodes_(42));
        assert_eq!(res.ponder_move, None);
        assert_eq!(res.chosen_move, Move::default());
        // assert_eq!(res.score, Score(0));
    }

    fn generic_search_test<E: Engine<Board>>(mut engine: E) {
        let i = Instant::now();
        let fen = "7r/pBrkqQ1p/3b4/5b2/8/6P1/PP2PP1P/R1BR2K1 w - - 1 17";
        let board = Board::from_fen(fen, Strict).unwrap();
        let res = engine.search_with_new_tt(board, SearchLimit::mate(DepthPly::new(5)));
        assert_eq!(
            res.chosen_move,
            Move::new(Square::from_str("d1").unwrap(), Square::from_str("d6").unwrap(), MoveFlags::NormalCapture)
        );
        assert_eq!(res.score, SCORE_WON - 3);

        println!("{}", i.elapsed().as_millis());
        game_over_test(&mut engine);
        println!("{}", i.elapsed().as_millis());
        avoid_repetition(&mut engine);
        println!("{}", i.elapsed().as_millis());
        mate_beats_repetition(&mut engine);
        println!("{}", i.elapsed().as_millis());
        underpromo(&mut engine);
        println!("{}", i.elapsed().as_millis());

        two_threads_test::<E>();
        println!("{}", i.elapsed().as_millis());
    }

    fn avoid_repetition<E: Engine<Board>>(engine: &mut E) {
        let pgn = r#"[Variant "From Position"][FEN "8/3Q4/2K5/k7/6P1/8/8/8 w - - 0 1"]
                        1. Qd4 Ka6 2. Qd6 Ka5 3. Qd4 Ka6 4. Qd7 Ka5"#;
        let game = parse_pgn::<Board>(pgn, Strict, None).unwrap().game;
        let params = SearchParams::new_unshared(
            game.board,
            SearchLimit::depth(engine.default_bench_depth()),
            game.board_hist,
            TT::default(),
        );
        let res = engine.search(params);
        if let Some(plies_to_win) = res.score.plies_until_game_won() {
            assert!(plies_to_win > 4);
        } else {
            assert!(res.score >= Score(500));
        }
        assert_ne!(res.chosen_move, Move::from_text("Qd4", &game.board).unwrap());
    }

    fn mate_beats_repetition<E: Engine<Board>>(engine: &mut E) {
        let pos = Board::from_fen("8/3Q4/2K5/k7/6P1/8/8/8 w - - 99 99", Strict).unwrap();
        let res = engine.search_with_new_tt(pos, SearchLimit::depth(engine.default_bench_depth()));
        let score = res.score;
        assert!(score > Score(500));
        if let Some(plies_to_win) = res.score.plies_until_game_won() {
            assert!(plies_to_win > 2);
        }
        assert_eq!(res.chosen_move, Move::from_text("g5!", &pos).unwrap());
        let pos = Board::from_fen("8/8/k1K5/8/8/8/3Q2P1/8 w - - 99 99", Strict).unwrap();
        let res = engine.search_with_new_tt(pos, SearchLimit::depth_(3));
        let score = res.score;
        assert_eq!(score.plies_until_game_won(), Some(1), "{score}");
        assert_eq!(res.chosen_move, Move::from_text("d2a2", &pos).unwrap());
    }

    fn underpromo<E: Engine<Board>>(engine: &mut E) {
        let random = engine.engine_info().eval.unwrap().short == "random";
        let pos = Board::from_fen("8/k1P5/1pP5/1P6/8/1p6/pP6/K7 w - - 0 1", Strict).unwrap();
        let res = engine.search_with_new_tt(pos, SearchLimit::depth(engine.default_bench_depth()));
        let score = res.score;
        assert!(res.chosen_move.is_promotion());
        if random {
            // technically, we would expect even a random eval to give a root score > 0, but it's not entirely guaranteed
            return;
        }
        assert!(score > Score(0), "{score}");
        assert!([Knight, Bishop].contains(&res.chosen_move.promo_piece()));

        let pos = Board::from_fen("8/8/8/8/K2b4/8/kp6/3N4 b - - 0 1", Strict).unwrap();
        let res = engine.search_with_new_tt(pos, SearchLimit::depth(engine.default_bench_depth()));
        let score = res.score;
        assert!(random || score > Score(0), "{score}");
        assert!(res.chosen_move.is_promotion());
        // only a bishop promo wins, but we don't expect the engine to see that so quickly
        assert!([Knight, Bishop].contains(&res.chosen_move.promo_piece()));
    }

    fn two_threads_test<E: Engine<Board>>() {
        let fen = "2kr3r/2pb1p2/p2b1p2/1p4pp/B2R4/2P1P2P/PP2KPP1/R1B5 w - - 0 16";
        let board = Board::from_fen(fen, Strict).unwrap();
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
        // The bound of 500 is rather large because gaps does not produce very stable evals
        let max_diff = if engine.short_name() == "CAPS" { 50 } else { 500 };
        let handle = spawn(move || engine.search(params));
        let handle2 = spawn(move || engine2.search(params2));
        sleep(Duration::from_millis(300));
        atomic.set_stop(true);
        assert!(atomic.stop_flag());
        assert!(!atomic2.stop_flag());
        let res = handle.join().unwrap();
        assert!(!atomic.currently_searching());
        assert!(atomic2.currently_searching());
        fence(SeqCst);
        atomic2.set_stop(true);
        let res2 = handle2.join().unwrap();
        assert!(res.score.0.abs_diff(res2.score.0) <= max_diff, "{0} {1}", res.score, res2.score);
        assert_eq!(res.chosen_move.piece_type(&board), Bishop);
        assert_eq!(res2.chosen_move.src_square(), Square::from_str("a4").unwrap());
    }

    #[test]
    fn weird_position_test() {
        // this fen is actually a legal chess position
        let fen = "q2k2q1/2nqn2b/1n1P1n1b/2rnr2Q/1NQ1QN1Q/3Q3B/2RQR2B/Q2K2Q1 w - - 0 1";
        let board = Board::from_fen(fen, Strict).unwrap();
        let mut engine = Caps::for_eval::<LiTEval>();
        // TODO: New testcase that asserts that unfinished iterations can still change the score
        let res = engine.search_with_new_tt(board, SearchLimit::depth_(1));
        // let res = engine.search_with_new_tt(board, SearchLimit::nodes_(5_000));
        let score = res.score;
        assert!(res.score >= Score(1400), "{score}");
        // not a legal chess position, but search should still handle this
        let fen = "RRRRRRRR/RRRRRRRR/BBBBBBBB/BBBBBBBB/QQQQQQQQ/QQQQQQQQ/QPPPPPPP/K6k b - - 0 1";
        let board = Board::from_fen(fen, Relaxed).unwrap();
        assert!(board.pseudolegal_moves().len() <= 3);
        for i in (2..55).step_by(3) {
            // do this several times to get different random numbers
            let mut engine = Caps::for_eval::<RandEval>();
            let res = engine.search_with_new_tt(board, SearchLimit::depth(DepthPly::new(i)));
            assert_eq!(res.score, SCORE_LOST + 2);
            assert_eq!(res.chosen_move.compact_formatter(&board).to_string(), "h1g1");
        }
        let mut engine = Caps::for_eval::<LiTEval>();
        let res = engine.search_with_new_tt(board, SearchLimit::depth_(10));
        assert_eq!(res.score, SCORE_LOST + 2);
        let expected_move = Move::from_compact_text("h1g1", &board).unwrap();
        assert_eq!(res.chosen_move, expected_move);
        // caused a crash once
        let fen = "8/2k5/8/4P3/PPPP1PPP/PPPPPPPP/PPPPPPPP/QQQQKQQQ b - - 0 1";
        let board = Board::from_fen(fen, Relaxed).unwrap();
        let res = engine.search_with_new_tt(board, SearchLimit::depth(DepthPly::new(8)));
        assert!(res.score <= Score(7000), "{}", res.score);
    }

    #[test]
    fn repetition_test() {
        // the only winning move leads to a repeated position; all other moves lose
        let mut board = Board::from_fen("8/8/3k3q/8/1K6/8/8/R7 w - - 0 1", Strict).unwrap();
        let movelist = ["Ra6+", "Kd7", "Ra1", "Kd6"];
        let mut hist = ZobristHistory::default();
        for _ in 0..2 {
            for mov in movelist {
                let mov = Move::from_extended_text(mov, &board).unwrap();
                board = board.play(mov);
                assert!(board.calc_player_result(&hist).is_none());
                hist.push(board.hash_pos());
            }
        }
        let mov = Move::from_extended_text(movelist[0], &board).unwrap();
        let new_board = board.play(mov);
        assert!(new_board.is_in_check());
        assert!(new_board.is_3fold_repetition(&hist));
        assert!(new_board.calc_player_result(&hist).is_some_and(|r| r == Draw));
        assert!(n_fold_repetition(2, &hist, new_board.hash_pos(), new_board.ply_draw_clock(),));
        hist.pop();
        let mut engine = Caps::for_eval::<MaterialOnlyEval>();
        for depth in 1..10 {
            let res = engine.search(SearchParams::new_unshared(
                board,
                SearchLimit::depth(DepthPly::new(depth)),
                hist.clone(),
                TT::default(),
            ));
            assert_eq!(res.chosen_move, mov);
            assert_eq!(res.score, Score(0));
        }
        let state = parse_ugi_pos_and_hist::<Board>(
            "pos fen 1k6/8/2K5/Q7/8/8/8/8 w - - 0 1 moves a5d8 b8a7 d8a5 a7b8 a5d8 b8a7 d8a5 a7b8",
            Strict,
        )
        .unwrap();
        let hist = state.board_hist.clone();
        let res = engine.search(SearchParams::new_unshared(
            state.board,
            SearchLimit::nodes_(12_345),
            hist.clone(),
            TT::default(),
        ));
        assert_eq!(state.board.calc_player_result(&hist), Some(Draw));
        assert_eq!(res.score.plies_until_game_won(), Some(3));
        assert_eq!(res.chosen_move, Move::from_text("Qc7+", &state.board).unwrap());
    }

    #[test]
    fn mate_in_three() {
        let pos = Board::from_fen("r4r1k/7p/pp1pP2b/2p1p2P/2P2p2/3B3q/PP1BNP2/R1QR2K1 b - - 4 27", Strict).unwrap();
        let mut limit = SearchLimit::mate_in_moves(3);
        let engines: [(Box<dyn Engine<Board>>, u64); 2] = [
            (Box::new(Caps::for_eval::<KingGambot>()), 100_000),
            (Box::new(Caps::for_eval::<MaterialOnlyEval>()), 200_000),
            // TODO: Re-enable when Gaps has more features
            // (Box::new(Gaps::<Chessboard>::for_eval::<LiTEval>()), 900_000),
        ];
        for (mut engine, nodes) in engines.into_iter() {
            println!("{}", engine.engine_info().short_name());
            limit.nodes = NodesLimit::new(nodes).unwrap();
            let res = engine.search_with_new_tt(pos, limit);
            assert!(res.score.is_game_won_score(), "{}", res.score);
            assert_eq!(res.score.plies_until_game_won(), Some(5));
            assert_eq!(res.chosen_move, Move::from_text("f3", &pos).unwrap());
        }
    }

    #[test]
    fn multipv_mate() {
        let pos = Board::from_name("mate_in_1").unwrap();
        let limit = SearchLimit::depth_(5);

        let engines: [Box<dyn Engine<Board>>; 8] = [
            Box::new(Caps::for_eval::<LiTEval>()),
            Box::new(Caps::for_eval::<MaterialOnlyEval>()),
            Box::new(Caps::for_eval::<KingGambot>()),
            Box::new(Caps::for_eval::<RandEval>()),
            Box::new(Gaps::<Board>::for_eval::<LiTEval>()),
            Box::new(Gaps::<Board>::for_eval::<MaterialOnlyEval>()),
            Box::new(Gaps::<Board>::for_eval::<KingGambot>()),
            Box::new(Gaps::<Board>::for_eval::<RandEval>()),
        ];

        for mut engine in engines.into_iter() {
            println!("{}", engine.engine_info().short_name());
            let mut params = SearchParams::for_pos(pos, limit);
            params.num_multi_pv = 3;
            let res = engine.search(params);
            assert_eq!(res.chosen_move, Move::from_text("Ra7#", &pos).unwrap());
            assert_eq!(res.score, game_result_to_score(Win, 1));
            let pv_data = engine.search_state_dyn().pv_data();
            assert_eq!(pv_data.len(), 3);
            assert_eq!(pv_data[0].score, res.score);
            assert_eq!(pv_data[0].pv.list.first(), Some(&res.chosen_move));
            assert!(pv_data[1].score <= game_result_to_score(Win, 3));
            assert!(pv_data[1].score >= Score(1000));
            let second_best_move = Move::from_extended_text("e1Q+", &pos).unwrap();
            assert_eq!(
                pv_data[1].pv.list.first() == Some(&second_best_move),
                pv_data[1].score == game_result_to_score(Win, 3)
            );
            assert!(pv_data[2].score >= Score(700));
            assert!(!pv_data[2].pv.list.is_empty());
        }
    }

    #[test]
    fn deep_search() {
        let fen = "5b1k/p1p1p1p1/P1P1P1P1/8/4p1p1/PpPpP1P1/1P1P4/K1B3B1 w - - 0 1";
        let pos = Board::from_fen(fen, Relaxed).unwrap();
        let mut engine = Caps::for_eval::<PistonEval>();
        let res = engine.search_with_new_tt(pos, SearchLimit::depth_(9999));
        assert_eq!(res.score, Score(0));
    }

    #[test]
    fn doesnt_clear_check() {
        let fen = "kr5r/1p1q3p/8/1q6/R7/8/1RQ5/1K1B4 b - - 0 1";
        let pos = Board::from_fen(fen, Relaxed).unwrap();
        let mut engine = Caps::for_eval::<PistonEval>();
        let res = engine.search_with_new_tt(pos, SearchLimit::nodes_(3));
        if res.score == NO_SCORE_YET {
            return;
        }
        assert!(res.score >= Score(0), "{} {res:?}", res.score);
        let mov = res.chosen_move;
        assert!(pos.is_move_legal(mov));
    }

    #[test]
    fn ep_mate_in_one() {
        let input = "fen 3k4/2p4R/8/3P4/8/7B/3Q4/3KR3 b - - 0 1 moves c7c5";
        let pos = load_ugi_pos_simple(input, Strict, &Board::default()).unwrap();
        let mut engine = Caps::for_eval::<PistonEval>();
        let res = engine.search_with_new_tt(pos, SearchLimit::nodes_(200));
        assert_eq!(pos.ep_square(), Some(Square::from_str("c6").unwrap()));
        assert!(res.score.is_game_won_score());
        assert_eq!(res.score.plies_until_game_won(), Some(1));
        assert_eq!(res.chosen_move, Move::from_text(":c ep", &pos).unwrap());
    }

    #[test]
    fn weird_unbalanced() {
        let input = "fen krr5/rrr5/rrr5/8/8/8/QQQQQQQQ/QQQQKQQQ w - - 0 1";
        let pos = load_ugi_pos_simple(input, Relaxed, &Board::default()).unwrap();
        let evals = list_chess_evals();
        let tt = TT::minimal();
        for searcher in list_chess_searchers() {
            for eval in &evals {
                if eval.long_name().to_ascii_lowercase().contains("random")
                    || searcher.long_name().to_ascii_lowercase().contains("random")
                    || searcher.long_name().to_ascii_lowercase().contains("proof")
                {
                    continue;
                }
                let mut engine = searcher.build(eval.as_ref());
                println!("searching with {}", engine.engine_info().long_name());
                let eval = engine.static_eval(&pos, 0);
                assert!(eval > Score(1000), "{eval}");
                let res = engine.search_with_tt(pos, SearchLimit::nodes_(5000), tt.clone());
                assert!(res.score >= Score(1000), "{}", res.score);
                assert!(pos.is_move_legal(res.chosen_move));

                let fen = "qqqqqqqq/qqqqqqqq/qqqqqqqq/qqqqqqqq/qqqqrbnq/qqqqbKQn/qqqqrb1b/qqqqqrbk b - - 0 1";
                let pos = Board::from_fen(fen, Relaxed).unwrap();
                let res = engine.search_with_tt(pos, SearchLimit::nodes_(500), tt.clone());
                assert_eq!(res.score.plies_until_game_won(), Some(1));
                let pos = pos.make_nullmove().unwrap();
                let res = engine.search_with_tt(pos, SearchLimit::nodes_(500), tt.clone());
                assert_eq!(res.score.plies_until_game_won(), Some(1));
                assert_eq!(res.chosen_move, Move::from_text("Qg2", &pos).unwrap());
            }
        }
    }

    #[test]
    fn hash_collision() {
        // these two positions have the exact same zobrist hash
        let pos1 = "2n5/1Rp1K1pn/q6Q/1rrr4/k3Br2/7B/1n1N2Q1/1Nn2R2 w - - 0 1";
        let pos1 = Board::from_fen(pos1, Strict).unwrap();
        let pos2 = "1K1NQ3/q2RqR2/3rp3/Qr3rn1/1Q1bB3/1Q1b1PN1/pRp3P1/k1q1Bn1q w - - 0 1";
        let pos2 = Board::from_fen(pos2, Strict).unwrap();
        assert_eq!(pos1.hash_pos(), pos2.hash_pos());
        let limit = SearchLimit::nodes_(2222);
        let tt = TT::default();
        let mut engine = Caps::for_eval::<LiTEval>();
        let res1 = engine.search_with_tt(pos1, limit, tt.clone());
        assert_eq!(engine.uci_nodes(), 2222);
        assert!(pos1.is_move_legal(res1.chosen_move));
        let res2 = engine.search_with_tt(pos2, limit, tt.clone());
        assert_ne!(res2.chosen_move, res1.chosen_move);
        assert!(pos2.is_move_legal(res2.chosen_move));
        let entry = tt.load::<Board>(pos2.hash_pos(), 0).unwrap();
        assert_eq!(entry.move_untrusted().trust_unchecked(), res2.chosen_move);
        let entry1 = tt.load(pos1.hash_pos(), 0).unwrap();
        assert_eq!(entry1, entry);
        let res1 = engine.search_with_tt(pos1, SearchLimit::depth_(3), tt.clone());
        assert_ne!(res1.chosen_move, res2.chosen_move);
        assert!(pos1.is_move_legal(res1.chosen_move));
        assert_ne!(engine.uci_nodes(), 2222);
    }

    #[test]
    fn depth_one_startpos() {
        let mut engine = Caps::for_eval::<LiTEval>();
        let res = engine.search(SearchParams::for_pos(Board::default(), SearchLimit::depth_(1)));
        assert_eq!(engine.iterations().get(), 1);
        assert!(Board::default().is_move_legal(res.chosen_move));
        assert!(!res.score.is_won_lost_or_draw_score());
        assert_eq!(res.pos, Board::default());
        let nodes = engine.uci_nodes();
        assert!(nodes >= 22, "{nodes}");
        assert!(nodes <= 42, "{nodes}");
        engine.forget();
        let res2 = engine.search(SearchParams::for_pos(Board::default(), SearchLimit::nodes_(nodes)));
        assert_eq!(res, res2);
    }
}
