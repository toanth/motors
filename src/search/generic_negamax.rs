use std::time::{Duration, Instant};

use rand::thread_rng;

use crate::eval::Eval;
use crate::games::Board;
use crate::search::{
    should_stop, stop_engine, BenchResult, Engine, EngineOptionType, EngineState, InfoCallback,
    Score, SearchInfo, SearchLimit, SearchResult, Searcher, SimpleSearchState, TimeControl,
    SCORE_LOST, SCORE_TIME_UP, SCORE_WON,
};

const MAX_DEPTH: usize = 100;

#[derive(Debug, Default)]
pub struct GenericNegamax<B: Board, E: Eval<B>> {
    state: SimpleSearchState<B>,
    eval: E,
}

impl<B: Board, E: Eval<B>> Searcher<B> for GenericNegamax<B, E> {
    fn search(&mut self, pos: B, limit: SearchLimit) -> SearchResult<B> {
        self.state = SimpleSearchState::initial_state(pos, self.state.info_callback);
        let mut chosen_move = self.state.best_move;
        let max_depth = MAX_DEPTH.min(limit.depth) as isize;

        for depth in 1..=max_depth {
            self.state.depth = depth as usize;
            let iteration_score = self.negamax(pos, limit, 0, depth, SCORE_LOST, SCORE_WON);
            if self.state.search_cancelled || should_stop(&limit, self, self.state.start_time) {
                break;
            }
            self.state.score = Score(iteration_score);
            chosen_move = self.state.best_move;
            self.state.info_callback.call(self.search_info())
        }

        SearchResult::move_only(chosen_move.unwrap_or_else(|| {
            println!("Warning: Not even a single iteration finished");
            let mut rng = thread_rng();
            pos.random_legal_move(&mut rng)
                .expect("search() called in a position with no legal moves")
        }))
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
        "Generic Negamax"
    }
}

impl<B: Board, E: Eval<B>> Engine<B> for GenericNegamax<B, E> {
    fn bench(&mut self, pos: B, depth: usize) -> BenchResult {
        self.state = SimpleSearchState::initial_state(pos, self.state.info_callback);
        let mut limit = SearchLimit::infinite();
        limit.depth = MAX_DEPTH.min(depth);
        self.state.depth = limit.depth;
        self.negamax(pos, limit, 0, limit.depth as isize, SCORE_LOST, SCORE_WON);
        self.state.to_bench_res()
    }

    fn stop(&mut self) -> Result<SearchResult<B>, String> {
        stop_engine(
            &self.state.initial_pos,
            self.state.best_move,
            self.state.score,
        )
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

    fn set_option(&mut self, option: &str, value: &str) -> Result<(), String> {
        Err(format!("Searcher {0} doesn't implement any options, so can't set option '{option}' to '{value}'", self.name()))
    }

    fn get_options(&self) -> Vec<EngineOptionType> {
        return Vec::default();
    }
}

impl<B: Board, E: Eval<B>> GenericNegamax<B, E> {
    fn negamax(
        &mut self,
        pos: B,
        limit: SearchLimit,
        ply: usize,
        depth: isize,
        mut alpha: i32,
        beta: i32,
    ) -> i32 {
        assert!(alpha < beta);
        assert!(ply <= MAX_DEPTH * 2);
        assert!(depth <= MAX_DEPTH as isize);

        if pos.is_game_lost() {
            return SCORE_LOST + ply as i32;
        }
        if pos.is_draw() {
            return 0;
        }
        if depth <= 0 {
            return self.eval.eval(pos).0;
        }

        let mut best_score = SCORE_LOST;

        for mov in pos.pseudolegal_moves() {
            let new_pos = pos.make_move(mov);
            if new_pos.is_none() {
                continue; // illegal pseudolegal move
            }
            self.state.nodes += 1;

            let score = -self.negamax(new_pos.unwrap(), limit, ply + 1, depth - 1, -beta, -alpha);

            if self.state.search_cancelled || should_stop(&limit, self, self.state.start_time) {
                self.state.search_cancelled = true;
                return SCORE_TIME_UP;
            }

            if score <= best_score {
                continue;
            }
            alpha = alpha.max(score);
            best_score = score;
            if ply == 0 {
                self.state.best_move = Some(mov);
            }
            if score < beta {
                continue;
            }
            break;
        }
        best_score
    }
}
