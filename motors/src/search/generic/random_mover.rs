use std::fmt::{Debug, Formatter};
use std::time::{Duration, Instant};

use rand::{Rng, RngCore, SeedableRng, thread_rng};

use gears::games::Board;
use gears::general::common::{NamedEntity, Res, StaticallyNamedEntity};
use gears::search::{Depth, Nodes, Score, SearchInfo, SearchLimit, SearchResult, TimeControl};

use crate::search::{
    ABSearchState, Benchable, BenchResult, EmptySearchStackEntry, Engine, EngineInfo, NoCustomInfo,
    SearchState,
};
use crate::search::multithreading::SearchSender;
use crate::search::tt::TT;

pub trait SeedRng: Rng + SeedableRng {}

impl<T> SeedRng for T where T: Rng + SeedableRng {}

pub struct RandomMover<B: Board, R: SeedRng> {
    pub rng: R,
    chosen_move: B::Move,
    _state: ABSearchState<B, EmptySearchStackEntry, NoCustomInfo>,
}

impl<B: Board, R: SeedRng> Debug for RandomMover<B, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("random mover")
    }
}

impl<B: Board, R: SeedRng> Default for RandomMover<B, R> {
    fn default() -> Self {
        Self {
            rng: R::seed_from_u64(thread_rng().next_u64()),
            chosen_move: B::Move::default(),
            _state: ABSearchState::new(Depth::new(1)),
        }
    }
}

// impl<B: Board, R: SeedableRng + Rng + Clone + Send + 'static> RandomMover<B, R> {
//     pub fn with_rng(rng: R) -> EngineOwner<B> {
//         EngineOwner::new_with(|| Self {
//             rng: rng.clone(),
//             chosen_move: B::Move::default(),
//             _state: SimpleSearchState::default(),
//         })
//     }
// }

impl<B: Board, R: SeedRng + 'static> StaticallyNamedEntity for RandomMover<B, R> {
    fn static_short_name() -> &'static str
    where
        Self: Sized,
    {
        "random_mover"
    }

    fn static_long_name() -> &'static str
    where
        Self: Sized,
    {
        "Random Mover"
    }

    fn static_description() -> &'static str
    where
        Self: Sized,
    {
        "A very simple engine that always chooses a legal move uniformly at random."
    }
}

// impl<B: Board, R: SeedRng + Clone + Send + 'static> EngineBase for RandomMover<B, R> {}

impl<B: Board, R: SeedRng + Clone + Send + 'static> Benchable<B> for RandomMover<B, R> {
    fn bench(&mut self, _position: B, _depth: Depth) -> BenchResult {
        BenchResult::default()
    }

    fn engine_info(&self) -> EngineInfo {
        EngineInfo {
            name: self.long_name().to_string(),
            version: "0.1.0".to_string(),
            default_bench_depth: Depth::new(1), // ignored as the engine will just pick a random move no matter what
            options: Vec::default(),
            description: "An Engine that simply plays a random legal move".to_string(),
        }
    }
}

impl<B: Board, R: SeedRng + Clone + Send + 'static> Engine<B> for RandomMover<B, R> {
    fn time_up(&self, _: TimeControl, _: Duration, _: Instant) -> bool {
        false
    }

    fn can_use_multiple_threads() -> bool
    where
        Self: Sized,
    {
        false
    }

    fn do_search(
        &mut self,
        pos: B,
        _: SearchLimit,
        _sender: &mut SearchSender<B>,
    ) -> Res<SearchResult<B>> {
        self.chosen_move = pos
            .random_legal_move(&mut self.rng)
            .expect("search() called in a position with no legal moves");
        Ok(SearchResult::move_only(self.chosen_move))
    }

    fn search_info(&self) -> SearchInfo<B> {
        SearchInfo {
            best_move: self.chosen_move,
            depth: Depth::new(0),
            seldepth: None,
            time: Duration::default(),
            nodes: Nodes::new(1).unwrap(),
            pv: vec![self.chosen_move],
            score: Score(0),
            hashfull: None,
            additional: None,
        }
    }

    fn forget(&mut self) {
        // nothing to do
    }

    fn nodes(&self) -> Nodes {
        Nodes::new(1).unwrap()
    }

    fn set_tt(&mut self, _tt: TT) {
        // do nothing
    }

    fn search_state(&self) -> &impl SearchState<B> {
        &self._state
    }

    fn search_state_mut(&mut self) -> &mut impl SearchState<B> {
        &mut self._state
    }

    fn get_static_eval(&mut self, _pos: B) -> Score {
        Score(0)
    }
}
