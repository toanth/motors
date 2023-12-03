use std::time::{Duration, Instant};

use rand::thread_rng;

use crate::games::{Board, BoardHistory, ZobristHistoryBase, ZobristRepetition2Fold};
use crate::search::{
    game_result_to_score, should_stop, BenchResult, Engine, InfoCallback, Score, SearchInfo,
    SearchLimit, SearchResult, Searcher, SimpleSearchState, TimeControl, SCORE_LOST, SCORE_TIME_UP,
};

const MAX_DEPTH: usize = 100;

#[derive(Debug, Default)]
pub struct NaiveSlowNegamax<B: Board> {
    state: SimpleSearchState<B>,
}

impl<B: Board> Searcher<B> for NaiveSlowNegamax<B> {
    fn search(
        &mut self,
        pos: B,
        limit: SearchLimit,
        history: ZobristHistoryBase,
    ) -> SearchResult<B> {
        self.state.new_search(ZobristRepetition2Fold(history));

        self.state.score = self.negamax(pos, limit, 0);
        self.state.send_new_info();
        SearchResult::move_and_score(
            self.state.best_move.unwrap_or_else(|| {
                // Sadly, this is the expected case since there is no iterative deepening
                let mut rng = thread_rng();
                pos.random_legal_move(&mut rng)
                    .expect("search() called in a position with no legal moves")
            }),
            self.state.score,
        )
    }

    fn time_up(&self, tc: TimeControl, hard_limit: Duration, start_time: Instant) -> bool {
        if self.state.nodes % 1024 != 0 {
            false
        } else {
            let elapsed = start_time.elapsed();
            elapsed >= hard_limit.min(tc.remaining / 32 + tc.increment / 2)
        }
    }

    fn name(&self) -> &'static str {
        "Naive Slow Negamax"
    }
}

impl<B: Board> Engine<B> for NaiveSlowNegamax<B> {
    fn bench(&mut self, pos: B, depth: usize) -> BenchResult {
        self.state = Default::default();
        let mut limit = SearchLimit::default();
        limit.depth = depth;

        self.negamax(pos, limit, 0);
        self.state.to_bench_res()
    }

    fn default_bench_depth(&self) -> usize {
        1 // ignored as the engine will search until terminal nodes anyway
    }

    fn stop(&mut self) {
        self.state.search_cancelled = true;
    }

    fn set_info_callback(&mut self, f: InfoCallback<B>) {
        self.state.info_callback = f;
    }

    fn search_info(&self) -> SearchInfo<B> {
        self.state.to_info()
    }

    fn forget(&mut self) {
        self.state.forget();
    }

    fn nodes(&self) -> u64 {
        self.state.nodes
    }
}

impl<B: Board> NaiveSlowNegamax<B> {
    fn negamax(&mut self, pos: B, limit: SearchLimit, ply: usize) -> Score {
        assert!(ply <= MAX_DEPTH);

        if let Some(res) = pos.game_result_no_movegen() {
            return game_result_to_score(res, ply);
        }

        let mut best_score = SCORE_LOST;

        for mov in pos.pseudolegal_moves() {
            let new_pos = pos.make_move(mov);
            if new_pos.is_none() {
                continue; // illegal pseudolegal move
            }

            self.state.board_history.push(&pos);
            let score = -self.negamax(new_pos.unwrap(), limit, ply + 1);
            self.state.board_history.pop(&pos);

            if self.state.search_cancelled || should_stop(&limit, self, self.state.start_time) {
                self.state.search_cancelled = true;
                return SCORE_TIME_UP;
            }

            if score <= best_score {
                continue;
            }
            best_score = score;
            if ply == 0 {
                self.state.best_move = Some(mov);
            }
        }
        best_score
    }
}
