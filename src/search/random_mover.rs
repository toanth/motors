use std::fmt::{Debug, Formatter};
use std::time::{Duration, Instant};

use rand::Rng;

use crate::games::Board;
use crate::search::{
    BenchResult, Engine, InfoCallback, Score, SearchInfo, SearchLimit, SearchResult, Searcher,
    TimeControl,
};

pub struct RandomMover<B: Board, R: Rng + Default> {
    pub rng: R,
    info_callback: InfoCallback<B>,
    chosen_move: B::Move,
}

impl<B: Board, R: Rng + Default> Debug for RandomMover<B, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("random mover")
    }
}

impl<B: Board, R: Rng + Default> Default for RandomMover<B, R> {
    fn default() -> Self {
        Self {
            rng: R::default(),
            info_callback: InfoCallback::default(),
            chosen_move: B::Move::default(),
        }
    }
}

impl<B: Board, R: Rng + Default + 'static> Searcher<B> for RandomMover<B, R> {
    fn search(&mut self, pos: B, _: SearchLimit) -> SearchResult<B> {
        self.chosen_move = pos
            .random_legal_move(&mut self.rng)
            .expect("search() called in a position with no legal moves");
        SearchResult::move_only(self.chosen_move)
    }

    fn time_up(&self, _: TimeControl, _: Duration, _: Instant) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "Random Mover"
    }
}

impl<B: Board, R: Rng + Default + 'static> Engine<B> for RandomMover<B, R> {
    fn bench(&mut self, position: B, depth: usize) -> BenchResult {
        BenchResult {
            nodes: 1,
            time: Duration::default(),
            depth: 0,
        }
    }

    fn default_bench_depth(&self) -> usize {
        1 // ignored as the engine will just pick a random move no matter what
    }

    fn stop(&mut self) -> Result<SearchResult<B>, String> {
        Ok(SearchResult::default())
    }

    fn set_info_callback(&mut self, f: InfoCallback<B>) {
        self.info_callback = f;
    }

    fn search_info(&self) -> SearchInfo<B> {
        SearchInfo {
            best_move: self.chosen_move,
            depth: 0,
            seldepth: None,
            time: Duration::default(),
            nodes: 1,
            pv: vec![self.chosen_move],
            score: Score(0),
            hashfull: None,
            additional: None,
        }
    }

    fn forget(&mut self) {
        // nothing to do
    }

    fn nodes(&self) -> u64 {
        1
    }
}
