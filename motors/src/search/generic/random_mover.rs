use gears::itertools::Itertools;
use std::fmt::{Debug, Display, Formatter};
use std::time::Duration;

use gears::general::board::Board;
use gears::rand::{Rng, RngCore, SeedableRng, rng};

use crate::eval::Eval;
use crate::eval::rand_eval::RandEval;
use crate::search::{AbstractSearchState, EmptySearchStackEntry, Engine, EngineInfo, NoCustomInfo, SearchState};
use gears::general::common::StaticallyNamedEntity;
use gears::score::Score;
use gears::search::NodeType::Exact;
use gears::search::{Budget, DepthPly, NodesLimit, SearchInfo, SearchResult};

pub trait SeedRng: Rng + SeedableRng {}

impl<T> SeedRng for T where T: Rng + SeedableRng {}

pub struct RandomMover<B: Board, R: SeedRng> {
    pub rng: R,
    state: SearchState<B, EmptySearchStackEntry, NoCustomInfo>,
}

impl<B: Board, R: SeedRng> Debug for RandomMover<B, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("random mover")
    }
}

impl<B: Board, R: SeedRng> Default for RandomMover<B, R> {
    fn default() -> Self {
        Self { rng: R::seed_from_u64(rng().next_u64()), state: SearchState::new(DepthPly::new(1)) }
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

impl<B: Board, R: SeedRng + Clone + Send + 'static> Engine<B> for RandomMover<B, R> {
    type SearchStackEntry = EmptySearchStackEntry;
    type CustomInfo = NoCustomInfo;

    fn max_bench_depth(&self) -> DepthPly {
        DepthPly::new(1)
    }

    fn engine_info(&self) -> EngineInfo {
        let mut res = EngineInfo::new(
            self,
            &RandEval::default(),
            "0.1.0",
            DepthPly::new(1),
            NodesLimit::new(1).unwrap(),
            Some(1),
            vec![],
        );
        res.eval = None;
        res
    }

    fn set_eval(&mut self, _eval: Box<dyn Eval<B>>) {
        // do nothing
    }

    fn do_search(&mut self) -> SearchResult<B> {
        self.state.statistics.next_id_iteration();
        let pos = &self.state.params.pos;

        let moves = pos.legal_moves_slow().into_iter().filter(|m| self.state.excluded_moves.contains(m)).collect_vec();
        let best_move = if moves.is_empty() {
            pos.random_legal_move(&mut self.rng).unwrap_or_default()
        } else {
            moves[self.rng.random_range(0..moves.len())]
        };
        self.state.atomic().set_best_move(best_move);
        let info = &mut self.state.multi_pvs[self.state.current_pv_num];
        info.pv.reset_to_move(best_move);
        SearchResult::move_only(best_move, pos.clone())
    }

    fn static_eval(&mut self, _pos: &B, _ply: usize) -> Score {
        Score(0)
    }

    fn search_info(&self) -> SearchInfo<B> {
        SearchInfo {
            best_move_of_all_pvs: self.state.best_move(),
            iterations: DepthPly::new(0),
            budget: Budget::new(0),
            seldepth: DepthPly::new(0),
            time: Duration::default(),
            nodes: NodesLimit::new(1).unwrap(),
            pv_num: 1,
            max_num_pvs: self.state.search_params().num_multi_pv,
            pv: self.state.current_mpv_pv(),
            score: Score(0),
            hashfull: 0,
            pos: self.state.params.pos.clone(),
            bound: Some(Exact),
            num_threads: 1,
            additional: None,
        }
    }

    fn with_eval(_eval: Box<dyn Eval<B>>) -> Self {
        Self::default()
    }

    fn search_state_dyn(&self) -> &dyn AbstractSearchState<B> {
        &self.state
    }

    fn search_state_mut_dyn(&mut self) -> &mut dyn AbstractSearchState<B> {
        &mut self.state
    }
}
