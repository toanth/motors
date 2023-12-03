use std::time::{Duration, Instant};

use rand::thread_rng;

use crate::eval::Eval;
use crate::games::{Board, BoardHistory, ZobristHistoryBase, ZobristRepetition2Fold};
use crate::search::{
    game_result_to_score, should_stop, BenchResult, Engine, EngineOptionName, EngineOptionType,
    InfoCallback, Score, SearchInfo, SearchLimit, SearchResult, Searcher, SimpleSearchState,
    TimeControl, SCORE_LOST, SCORE_TIME_UP, SCORE_WON,
};

const MAX_DEPTH: usize = 100;

#[derive(Debug, Default)]
pub struct GenericNegamax<B: Board, E: Eval<B>> {
    state: SimpleSearchState<B>,
    eval: E,
}

impl<B: Board, E: Eval<B>> Searcher<B> for GenericNegamax<B, E> {
    fn search(
        &mut self,
        pos: B,
        limit: SearchLimit,
        history: ZobristHistoryBase,
    ) -> SearchResult<B> {
        self.state.new_search(ZobristRepetition2Fold(history));
        let mut chosen_move = self.state.best_move;
        let max_depth = MAX_DEPTH.min(limit.depth) as isize;

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

        SearchResult::move_only(chosen_move.unwrap_or_else(|| {
            eprintln!("Warning: Not even a single iteration finished");
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
        self.state.new_search(ZobristRepetition2Fold::default());
        let mut limit = SearchLimit::infinite();
        limit.depth = MAX_DEPTH.min(depth);
        self.state.depth = limit.depth;
        self.negamax(pos, limit, 0, limit.depth as isize, SCORE_LOST, SCORE_WON);
        self.state.to_bench_res()
    }

    fn default_bench_depth(&self) -> usize {
        4 // overly conversative for most games, but better too little than too much
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

    fn set_option(&mut self, option: EngineOptionName, value: &str) -> Result<(), String> {
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
        mut alpha: Score,
        beta: Score,
    ) -> Score {
        debug_assert!(alpha < beta);
        debug_assert!(ply <= MAX_DEPTH * 2);
        debug_assert!(depth <= MAX_DEPTH as isize);

        if let Some(res) = pos.game_result_no_movegen() {
            return game_result_to_score(res, ply);
        }
        if depth <= 0 {
            return self.eval.eval(pos);
        }

        let mut best_score = SCORE_LOST;
        let mut num_children = 0;

        for mov in pos.pseudolegal_moves() {
            let new_pos = pos.make_move(mov);
            if new_pos.is_none() {
                continue; // illegal pseudolegal move
            }
            self.state.nodes += 1;
            num_children += 1;

            let score = -self.negamax(new_pos.unwrap(), limit, ply + 1, depth - 1, -beta, -alpha);

            self.state.board_history.pop(&new_pos.unwrap());

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
        if num_children == 0 {
            game_result_to_score(pos.no_moves_result(), ply)
        } else {
            best_score
        }
    }
}
