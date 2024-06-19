use std::time::{Duration, Instant};

use rand::thread_rng;

use gears::games::{Board, BoardHistory};
use gears::general::common::{NamedEntity, Res, StaticallyNamedEntity};
use gears::score::{game_result_to_score, Score, SCORE_LOST, SCORE_TIME_UP, SCORE_WON};
use gears::search::{Depth, SearchLimit, SearchResult, TimeControl};
use gears::ugi::EngineOptionName;

use crate::eval::Eval;
use crate::search::statistics::SearchType::MainSearch;
use crate::search::tt::TT;
use crate::search::NodeType::{Exact, FailHigh, FailLow};
use crate::search::{
    ABSearchState, BenchResult, Benchable, EmptySearchStackEntry, Engine, EngineInfo, NoCustomInfo,
    SearchState,
};

const MAX_DEPTH: Depth = Depth::new(100);

#[derive(Debug)]
pub struct GenericNegamax<B: Board, E: Eval<B>> {
    state: ABSearchState<B, EmptySearchStackEntry, NoCustomInfo>,
    eval: E,
    tt: TT,
}

impl<B: Board, E: Eval<B>> Default for GenericNegamax<B, E> {
    fn default() -> Self {
        Self {
            state: ABSearchState::new(MAX_DEPTH),
            eval: E::default(),
            tt: TT::default(),
        }
    }
}

impl<B: Board, E: Eval<B>> StaticallyNamedEntity for GenericNegamax<B, E> {
    fn static_short_name() -> &'static str
    where
        Self: Sized,
    {
        "generic_negamax"
    }

    fn static_long_name() -> String {
        "Generic Negamax".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "A simple alpha-bete pruning negamax implementation that doesn't use any game-specific information".to_string()
    }
}

// impl<B: Board, E: Eval<B>> EngineBase for GenericNegamax<B, E> {}

impl<B: Board, E: Eval<B>> Benchable<B> for GenericNegamax<B, E> {
    fn bench(&mut self, pos: B, depth: Depth) -> BenchResult {
        self.state.forget(true);
        let mut limit = SearchLimit::infinite();
        limit.depth = MAX_DEPTH.min(depth);
        self.negamax(
            pos,
            limit,
            0,
            limit.depth.get() as isize,
            SCORE_LOST,
            SCORE_WON,
        );
        // TODO: Handle stop command in bench
        self.state.to_bench_res()
    }

    fn engine_info(&self) -> EngineInfo {
        EngineInfo {
            short_name: self.short_name().to_string(),
            name: self.long_name().to_string(),
            version: "0.0.0".to_string(),
            default_bench_depth: Depth::new(4),
            options: Vec::default(),
            description: "A game-independent negamax engine. Currently very basic.".to_string(),
        }
    }

    fn set_option(&mut self, option: EngineOptionName, value: String) -> Res<()> {
        Err(format!("Searcher {0} doesn't implement any options, so can't set option '{option}' to '{value}'", self.long_name()))
    }
}

impl<B: Board, E: Eval<B>> Engine<B> for GenericNegamax<B, E> {
    fn can_use_multiple_threads() -> bool
    where
        Self: Sized,
    {
        true
    }

    fn do_search(&mut self, pos: B, mut limit: SearchLimit) -> Res<SearchResult<B>> {
        let mut chosen_move = self.state.best_move;
        let max_depth = MAX_DEPTH.min(limit.depth).get() as isize;
        limit.fixed_time = limit.fixed_time.min(limit.tc.remaining);

        self.state.statistics.next_id_iteration();

        for depth in 1..=max_depth {
            let iteration_score = self.negamax(pos, limit, 0, depth, SCORE_LOST, SCORE_WON);
            if self.state.search_cancelled() {
                break;
            }
            self.state.score = iteration_score;
            chosen_move = self.state.best_move; // only set now so that incomplete iterations are discarded
            self.state.sender.send_search_info(self.search_info());
            // increases the depth. do this after sending the search info, but before deciding if the depth limit has been exceeded.
            self.state.statistics.next_id_iteration();
            if self.should_not_start_next_iteration(limit.fixed_time, max_depth, limit.mate) {
                break;
            }
        }

        Ok(SearchResult::move_and_score(
            chosen_move.unwrap_or_else(|| {
                eprintln!("Warning: Not even a single iteration finished");
                let mut rng = thread_rng();
                pos.random_legal_move(&mut rng)
                    .expect("search() called in a position with no legal moves")
            }),
            self.state.score,
        ))
    }

    fn time_up(&self, tc: TimeControl, hard_limit: Duration, start_time: Instant) -> bool {
        let elapsed = start_time.elapsed();
        elapsed >= hard_limit.min(tc.remaining / 32 + tc.increment / 2)
    }

    fn set_tt(&mut self, tt: TT) {
        self.tt = tt;
    }

    fn search_state(&self) -> &impl SearchState<B> {
        &self.state
    }

    fn search_state_mut(&mut self) -> &mut impl SearchState<B> {
        &mut self.state
    }

    fn static_eval(&mut self, pos: B) -> Score {
        self.eval.eval(pos)
    }
}

impl<B: Board, E: Eval<B>> GenericNegamax<B, E> {
    #[allow(clippy::too_many_arguments)]
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
        debug_assert!(ply <= MAX_DEPTH.get() * 2);
        debug_assert!(depth <= MAX_DEPTH.get() as isize);
        self.state
            .statistics
            .count_node_started(MainSearch, ply, true);

        if let Some(res) = pos.player_result_no_movegen(&self.state.board_history) {
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
            num_children += 1;
            self.state.statistics.count_legal_make_move(MainSearch);

            self.state.board_history.push(&pos);

            let score = -self.negamax(new_pos.unwrap(), limit, ply + 1, depth - 1, -beta, -alpha);

            self.state.board_history.pop();

            if self.should_stop(limit) {
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
        let node_type = if best_score >= beta {
            FailHigh
        } else if best_score <= alpha {
            FailLow
        } else {
            Exact
        };
        self.state
            .statistics
            .count_complete_node(MainSearch, node_type, depth, ply, num_children);
        if num_children == 0 {
            game_result_to_score(pos.no_moves_result(), ply)
        } else {
            best_score
        }
    }
}
