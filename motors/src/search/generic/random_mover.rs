use std::fmt::{Debug, Display, Formatter};
use std::time::{Duration, Instant};

use gears::general::board::Board;
use rand::{thread_rng, Rng, RngCore, SeedableRng};

use crate::eval::Eval;
use gears::general::common::{Res, StaticallyNamedEntity};
use gears::score::Score;
use gears::search::{Depth, NodesLimit, SearchInfo, SearchLimit, SearchResult, TimeControl};

use crate::search::tt::TT;
use crate::search::{
    ABSearchState, AbstractEngine, BestMoveCustomInfo, EmptySearchStackEntry, Engine, EngineInfo,
    SearchState,
};

pub trait SeedRng: Rng + SeedableRng {}

impl<T> SeedRng for T where T: Rng + SeedableRng {}

pub struct RandomMover<B: Board, R: SeedRng> {
    pub rng: R,
    chosen_move: B::Move,
    state: ABSearchState<B, EmptySearchStackEntry, BestMoveCustomInfo<B>>,
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
            state: ABSearchState::new(Depth::new(1)),
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
    fn static_short_name() -> impl Display
    where
        Self: Sized,
    {
        "random"
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        "Random Mover".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "A very simple engine that always chooses a legal move uniformly at random. Doesn't need an eval".to_string()
    }
}

// impl<B: Board, R: SeedRng + Clone + Send + 'static> EngineBase for RandomMover<B, R> {}

impl<B: Board, R: SeedRng + Clone + Send + 'static> AbstractEngine<B> for RandomMover<B, R> {
    fn max_bench_depth(&self) -> Depth {
        Depth::new(1)
    }

    fn engine_info(&self) -> EngineInfo {
        EngineInfo::new_without_eval(
            self,
            "0.1.0",
            Depth::new(1),
            NodesLimit::new(1).unwrap(),
            vec![],
        )
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
        search_moves: Vec<B::Move>,
        _multi_pv: usize,
        _: SearchLimit,
    ) -> Res<SearchResult<B>> {
        self.state.statistics.next_id_iteration();
        // there's no `is_empty` method
        let len = search_moves.len();
        if len != 0 {
            self.chosen_move = search_moves
                .into_iter()
                .nth(self.rng.gen_range(0..len))
                .unwrap();
        } else {
            self.chosen_move = pos.random_legal_move(&mut self.rng).unwrap_or_default();
        }
        Ok(SearchResult::move_only(self.chosen_move))
    }

    fn search_info(&self) -> SearchInfo<B> {
        SearchInfo {
            best_move: self.chosen_move,
            depth: Depth::new(0),
            seldepth: None,
            time: Duration::default(),
            nodes: NodesLimit::new(1).unwrap(),
            pv_num: 1,
            pv: vec![self.chosen_move],
            score: Score(0),
            hashfull: None,
            additional: None,
        }
    }

    fn set_tt(&mut self, _tt: TT) {
        // do nothing
    }

    fn search_state(&self) -> &impl SearchState<B> {
        &self.state
    }

    fn search_state_mut(&mut self) -> &mut impl SearchState<B> {
        &mut self.state
    }

    fn static_eval(&mut self, _pos: B) -> Score {
        Score(0)
    }

    fn with_eval(_eval: Box<dyn Eval<B>>) -> Self {
        Self::default()
    }

    fn set_eval(&mut self, _eval: Box<dyn Eval<B>>) {
        // do nothing
    }
}
