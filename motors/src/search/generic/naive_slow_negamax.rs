use std::time::{Duration, Instant};

use rand::thread_rng;

use gears::games::{Board, BoardHistory, ZobristHistoryBase, ZobristRepetition2Fold};
use gears::general::common::{NamedEntity, Res, StaticallyNamedEntity};
use gears::search::{
    game_result_to_score, Score, SCORE_LOST, SCORE_TIME_UP, SearchLimit, SearchResult, TimeControl,
};

use crate::search::{
    ABSearchState, BenchResult, Engine, EngineInfo, Searcher, SearcherBase, SearchState,
    should_stop,
};
use crate::search::multithreading::{EngineReceiver, EngineWrapper};
use crate::search::multithreading::EngineSends::Info;

const MAX_DEPTH: usize = 100;

#[derive(Debug)]
pub struct NaiveSlowNegamax<B: Board> {
    state: ABSearchState<B>,
    communicator: EngineReceiver<B>,
}

impl<B: Board> StaticallyNamedEntity for NaiveSlowNegamax<B> {
    fn static_short_name() -> &'static str
    where
        Self: Sized,
    {
        "naive_negamax"
    }

    fn static_long_name() -> &'static str
    where
        Self: Sized,
    {
        "Naive Slow Negamax"
    }

    fn static_description() -> &'static str
    where
        Self: Sized,
    {
        "A very simple engine that searches the *entire* game tree, without using alpha-beta pruning or an eval function. Will not finish except for very simple games"
    }
}

impl<B: Board> SearcherBase for NaiveSlowNegamax<B> {
    fn time_up(&self, tc: TimeControl, hard_limit: Duration, start_time: Instant) -> bool {
        if self.state.nodes % 1024 != 0 {
            false
        } else {
            let elapsed = start_time.elapsed();
            elapsed >= hard_limit.min(tc.remaining / 32 + tc.increment / 2)
        }
    }
}

impl<B: Board> Searcher<B> for NaiveSlowNegamax<B> {
    fn can_use_multiple_threads() -> bool
    where
        Self: Sized,
    {
        false
    }

    fn search(
        &mut self,
        pos: B,
        limit: SearchLimit,
        history: ZobristHistoryBase,
    ) -> Res<SearchResult<B>> {
        self.state.new_search(ZobristRepetition2Fold(history));

        self.state.score = self.negamax(pos, limit, 0);
        self.send(Info(self.search_info())).unwrap();
        Ok(SearchResult::move_and_score(
            self.state.best_move.unwrap_or_else(|| {
                // Sadly, this is the expected case since there is no iterative deepening
                let mut rng = thread_rng();
                pos.random_legal_move(&mut rng)
                    .expect("search() called in a position with no legal moves")
            }),
            self.state.score,
        ))
    }
}

impl<B: Board> Engine<B> for NaiveSlowNegamax<B> {
    fn bench(&mut self, pos: B, depth: usize) -> Res<BenchResult> {
        self.state = Default::default();
        let limit = SearchLimit {
            depth,
            ..Default::default()
        };

        self.negamax(pos, limit, 0);
        // TODO: Handle stop command
        Ok(self.state.to_bench_res())
    }

    fn clone_for_multithreading(&self) -> EngineWrapper<B> {
        EngineWrapper::new::<Self>()
    }

    fn engine_info(&self) -> EngineInfo {
        EngineInfo {
            name: self.long_name().to_string(),
            version: "0.1.0".to_string(),
            default_bench_depth: 0, // ignored as the engine will search until terminal nodes anyway
            options: Vec::default(),
            description: "An engine that searches the *entire* search tree. Useless except for very simple games like Tic-Tac-Toe".to_string(),
        }
    }

    type State = ABSearchState<B>;

    fn new(communicator: EngineReceiver<B>) -> Self {
        Self {
            state: ABSearchState::default(),
            communicator,
        }
    }

    fn search_state(&self) -> &Self::State {
        &self.state
    }

    fn search_state_mut(&mut self) -> &mut Self::State {
        &mut self.state
    }

    fn communicator(&mut self) -> &mut EngineReceiver<B> {
        &mut self.communicator
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

            if should_stop(self, limit) {
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
