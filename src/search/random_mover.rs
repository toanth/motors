use std::fmt::{Debug, Formatter};
use std::time::{Duration, Instant};

use rand::Rng;

use crate::games::{Board, ZobristHistoryBase};
use crate::general::common::Res;
use crate::search::multithreading::{EngineCommunicator, EngineOwner};
use crate::search::{
    BenchResult, Engine, EngineInfo, Score, SearchInfo, SearchLimit, SearchResult, Searcher,
    SimpleSearchState, TimeControl,
};

pub struct RandomMover<B: Board, R: Rng> {
    pub rng: R,
    chosen_move: B::Move,
    communicator: EngineCommunicator<B>,
}

impl<B: Board, R: Rng> Debug for RandomMover<B, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("random mover")
    }
}

impl<B: Board, R: Rng + Clone + Send + 'static> RandomMover<B, R> {
    pub fn with_rng(rng: R) -> EngineOwner<B> {
        EngineOwner::new_with(|communicator| Self {
            rng: rng.clone(),
            chosen_move: B::Move::default(),
            communicator,
        })
    }
}

impl<B: Board, R: Rng + 'static> Searcher<B> for RandomMover<B, R> {
    fn search(&mut self, pos: B, _: SearchLimit, _: ZobristHistoryBase) -> Res<SearchResult<B>> {
        self.chosen_move = pos
            .random_legal_move(&mut self.rng)
            .expect("search() called in a position with no legal moves");
        Ok(SearchResult::move_only(self.chosen_move))
    }

    fn time_up(&self, _: TimeControl, _: Duration, _: Instant) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "Random Mover"
    }
}

impl<B: Board, R: Rng + Clone + Send + 'static> Engine<B> for RandomMover<B, R> {
    fn bench(&mut self, position: B, depth: usize) -> Res<BenchResult> {
        Ok(BenchResult {
            nodes: 1,
            time: Duration::default(),
            depth: 0,
        })
    }

    fn stop(&mut self) {
        // do nothing
    }

    fn clone_for_multithreading(&self) -> EngineOwner<B> {
        // EngineOwner::new(Self::with_rng(self.rng.clone()))
        EngineOwner::new::<Self>()
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

    fn engine_info(&self) -> EngineInfo {
        EngineInfo {
            name: self.name().to_string(),
            version: "0.1.0".to_string(),
            default_bench_depth: 1, // ignored as the engine will just pick a random move no matter what
            options: Vec::default(),
            description: "An Engine that simply plays a random legal move".to_string(),
        }
    }

    type State = SimpleSearchState<B>;

    fn new(communicator: EngineCommunicator<B>) -> Self {
        Self {
            rng: todo!(),
            chosen_move: B::Move::default(),
            communicator,
        }
    }

    fn search_state(&self) -> &Self::State {
        todo!()
    }

    fn search_state_mut(&mut self) -> &mut Self::State {
        todo!()
    }

    fn communicator(&mut self) -> &mut EngineCommunicator<B> {
        &mut self.communicator
    }
}
