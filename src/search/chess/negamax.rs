use std::time::{Duration, Instant};

use itertools::Itertools;
use rand::thread_rng;

use crate::eval::Eval;
use crate::games::chess::moves::ChessMove;
use crate::games::chess::pieces::UncoloredChessPiece::Empty;
use crate::games::chess::{ChessMoveList, Chessboard};
use crate::games::{Board, ColoredPiece};
use crate::search::{
    game_result_to_score, should_stop, stop_engine, BenchResult, Engine, EngineOptionType,
    EngineState, InfoCallback, Score, SearchInfo, SearchLimit, SearchResult, SearchStateWithPv,
    Searcher, TimeControl, SCORE_LOST, SCORE_TIME_UP, SCORE_WON,
};

const DEPTH_SOFT_LIMIT: usize = 100;
const DEPTH_HARD_LIMIT: usize = 128;

#[derive(Debug, Default)]
pub struct Negamax<E: Eval<Chessboard>> {
    state: SearchStateWithPv<Chessboard, DEPTH_HARD_LIMIT>,
    eval: E,
}

impl<E: Eval<Chessboard>> Searcher<Chessboard> for Negamax<E> {
    fn name(&self) -> &'static str {
        "Chess Negamax"
    }

    fn time_up(&self, tc: TimeControl, hard_limit: Duration, start_time: Instant) -> bool {
        if self.state.nodes % 1024 != 0 {
            false
        } else {
            let elapsed = start_time.elapsed();
            elapsed >= hard_limit.min(tc.remaining / 32 + tc.increment / 2)
        }
    }

    fn search(&mut self, pos: Chessboard, limit: SearchLimit) -> SearchResult<Chessboard> {
        self.state = SearchStateWithPv::initial_state(pos, self.state.info_callback);
        let mut chosen_move = self.state.best_move;
        let max_depth = DEPTH_SOFT_LIMIT.min(limit.depth) as isize;

        println!(
            "starting search with limit {time} ms, {fixed} fixed, {depth} depth, {nodes} nodes",
            time = limit.tc.remaining.as_millis(),
            depth = limit.depth,
            nodes = limit.nodes,
            fixed = limit.fixed_time.as_millis()
        );

        for depth in 1..=max_depth {
            self.state.depth = depth as usize;
            let iteration_score = self.negamax(pos, limit, 0, depth, SCORE_LOST, SCORE_WON);
            if self.state.search_cancelled || should_stop(&limit, self, self.state.start_time) {
                break;
            }
            self.state.score = iteration_score;
            chosen_move = self.state.best_move; // only set now so that incomplete iterations are discarded
            self.state.info_callback.call(self.search_info())
        }

        SearchResult::move_and_score(
            chosen_move.unwrap_or_else(|| {
                eprintln!("Warning: Not even a single iteration finished");
                let mut rng = thread_rng();
                pos.random_legal_move(&mut rng)
                    .expect("search() called in a position with no legal moves")
            }),
            self.state.score,
        )
    }
}

impl<E: Eval<Chessboard>> Negamax<E> {
    fn negamax(
        &mut self,
        pos: Chessboard,
        limit: SearchLimit,
        ply: usize,
        depth: isize,
        mut alpha: Score,
        beta: Score,
    ) -> Score {
        debug_assert!(alpha < beta);
        debug_assert!(ply <= DEPTH_HARD_LIMIT * 2);
        debug_assert!(depth <= DEPTH_SOFT_LIMIT as isize);

        if let Some(res) = pos.game_result_no_movegen() {
            return game_result_to_score(res, ply);
        }
        if depth <= 0 {
            return self.eval.eval(pos);
        }

        let mut best_score = SCORE_LOST;
        let mut num_children = 0;

        let all_moves = Self::order_moves(pos.pseudolegal_moves(), &pos);
        for mov in all_moves {
            let new_pos = pos.make_move(mov);
            if new_pos.is_none() {
                continue; // illegal pseudolegal move
            }
            self.state.nodes += 1;
            num_children += 1;

            let score = -self.negamax(new_pos.unwrap(), limit, ply + 1, depth - 1, -beta, -alpha);

            if self.state.search_cancelled || should_stop(&limit, self, self.state.start_time) {
                self.state.search_cancelled = true;
                return SCORE_TIME_UP;
            }

            best_score = best_score.max(score);
            if score <= alpha {
                continue;
            }
            alpha = score;
            // TODO: This lost a smallish 2 digit amount of elo, so retest eventually
            // (will probably be much less important as the search gets slower)
            self.state.pv_table.new_pv_move(ply, mov);
            if ply == 0 {
                self.state.best_move = Some(mov);
            }
            if score < beta {
                continue;
            }
            break;
        }
        if num_children == 0 {
            game_result_to_score(pos.no_moves_result(), ply)
        } else {
            best_score
        }
    }

    fn order_moves(moves: ChessMoveList, board: &Chessboard) -> ChessMoveList {
        /// The move list is iterated backwards, which is why better moves get higher scores
        let score_function = |mov: &ChessMove| {
            let captured = mov.captured(board);
            if captured == Empty {
                0
            } else {
                10 + captured as usize * 10 - mov.piece(board).uncolored_piece_type() as usize
            }
        };
        moves.sorted_unstable_by_key(score_function).collect()
    }
}

impl<E: Eval<Chessboard>> Engine<Chessboard> for Negamax<E> {
    fn bench(&mut self, pos: Chessboard, depth: usize) -> BenchResult {
        self.state = SearchStateWithPv::initial_state(pos, self.state.info_callback);
        let mut limit = SearchLimit::infinite();
        limit.depth = DEPTH_SOFT_LIMIT.min(depth);
        self.state.depth = limit.depth;
        self.negamax(pos, limit, 0, limit.depth as isize, SCORE_LOST, SCORE_WON);
        self.state.to_bench_res()
    }

    fn default_bench_depth(&self) -> usize {
        6
    }

    fn stop(&mut self) -> Result<SearchResult<Chessboard>, String> {
        stop_engine(
            &self.state.initial_pos,
            self.state.best_move,
            self.state.score,
        )
    }

    fn set_info_callback(&mut self, f: InfoCallback<Chessboard>) {
        self.state.info_callback = f;
    }

    fn search_info(&self) -> SearchInfo<Chessboard> {
        self.state.to_info()
    }

    fn forget(&mut self) {
        self.state.forget();
    }

    fn nodes(&self) -> u64 {
        self.state.nodes
    }

    fn set_option(&mut self, option: &str, value: &str) -> Result<(), String> {
        Err(format!("Searcher {0} doesn't implement any options, so can't set option '{option}' to '{value}'", self.name()))
    }

    fn get_options(&self) -> Vec<EngineOptionType> {
        return Vec::default();
    }
}
