use itertools::Itertools;
use std::fmt::Display;
use std::time::{Duration, Instant};

use gears::games::BoardHistory;
use gears::general::board::Board;
use rand::thread_rng;

use gears::general::common::{NamedEntity, Res, StaticallyNamedEntity};
use gears::score::{game_result_to_score, Score, SCORE_LOST, SCORE_TIME_UP, SCORE_WON};
use gears::search::{Depth, NodesLimit, SearchLimit, SearchResult, TimeControl};
use gears::ugi::EngineOptionName;

use crate::eval::rand_eval::RandEval;
use crate::eval::Eval;
use crate::search::statistics::SearchType::MainSearch;
use crate::search::tt::TT;
use crate::search::NodeType::{Exact, FailHigh, FailLow};
use crate::search::{
    ABSearchState, AbstractEngine, BestMoveCustomInfo, EmptySearchStackEntry, Engine, EngineInfo,
    SearchState,
};

const MAX_DEPTH: Depth = Depth::new(100);

type DefaultEval = RandEval;

#[derive(Debug)]
pub struct Gaps<B: Board> {
    state: ABSearchState<B, EmptySearchStackEntry, BestMoveCustomInfo<B>>,
    eval: Box<dyn Eval<B>>,
    tt: TT,
}

impl<B: Board> Default for Gaps<B> {
    fn default() -> Self {
        Self::with_eval(Box::new(DefaultEval::default()))
    }
}

impl<B: Board> StaticallyNamedEntity for Gaps<B> {
    fn static_short_name() -> impl Display
    where
        Self: Sized,
    {
        "GAPS"
    }

    fn static_long_name() -> String {
        "GAPS: Generic Alpha-beta Pruning Search".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "A simple alpha-bete pruning negamax implementation that doesn't use any game-specific information".to_string()
    }
}

impl<B: Board> AbstractEngine<B> for Gaps<B> {
    fn max_bench_depth(&self) -> Depth {
        MAX_DEPTH
    }

    fn engine_info(&self) -> EngineInfo {
        EngineInfo::new(
            self,
            self.eval.as_ref(),
            "0.0.1",
            Depth::new(4),
            NodesLimit::new(50_000).unwrap(),
            vec![],
        )
    }

    fn set_option(&mut self, option: EngineOptionName, value: String) -> Res<()> {
        Err(format!("Searcher {0} doesn't implement any options, so can't set option '{option}' to '{value}'", self.long_name()))
    }
}

impl<B: Board> Engine<B> for Gaps<B> {
    fn can_use_multiple_threads() -> bool
    where
        Self: Sized,
    {
        true
    }

    fn do_search(
        &mut self,
        pos: B,
        search_moves: Vec<B::Move>,
        multi_pv: usize,
        mut limit: SearchLimit,
    ) -> Res<SearchResult<B>> {
        let mut chosen_move = self.state.custom.chosen_move;
        let mut score = Score::default();
        let max_depth = MAX_DEPTH.min(limit.depth).isize();
        limit.fixed_time = limit.fixed_time.min(limit.tc.remaining);
        if search_moves.is_empty() {
            self.state.excluded_moves = vec![];
        } else {
            self.state.excluded_moves = pos
                .pseudolegal_moves()
                .into_iter()
                .filter(|m| !search_moves.contains(m))
                .collect_vec();
        }

        self.state.statistics.next_id_iteration();
        self.state.limit = limit;

        'id: for depth in 1..=max_depth {
            for pv_num in 0..multi_pv {
                if self.should_not_start_iteration(limit.fixed_time, max_depth, limit.mate) {
                    break 'id;
                }
                self.state.pv_num = pv_num;
                let iteration_score = self.negamax(pos, 0, depth, SCORE_LOST, SCORE_WON);
                if self.state.search_cancelled() {
                    break 'id;
                }
                self.state.score = iteration_score;
                if pv_num == 0 {
                    chosen_move = self.state.custom.chosen_move; // only set now so that incomplete iterations are discarded
                    score = self.state.score;
                }
                self.state
                    .excluded_moves
                    .push(self.state.custom.chosen_move.unwrap_or_default());
                self.state.sender.send_search_info(self.search_info());
                // increases the depth. do this after sending the search info, but before deciding if the depth limit has been exceeded.
            }
            self.state
                .excluded_moves
                .truncate(self.state.excluded_moves.len() - multi_pv);
            self.state.statistics.next_id_iteration();
        }

        Ok(SearchResult::move_and_score(
            chosen_move.unwrap_or_else(|| {
                eprintln!("Warning: Not even a single iteration finished");
                let mut rng = thread_rng();
                pos.random_legal_move(&mut rng).unwrap_or_default()
            }),
            score,
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

    fn with_eval(eval: Box<dyn Eval<B>>) -> Self {
        Self {
            state: ABSearchState::new(MAX_DEPTH),
            eval,
            tt: TT::default(),
        }
    }

    fn static_eval(&mut self, pos: B) -> Score {
        self.eval.eval(&pos)
    }

    fn set_eval(&mut self, eval: Box<dyn Eval<B>>) {
        self.eval = eval;
    }
}

impl<B: Board> Gaps<B> {
    #[allow(clippy::too_many_arguments)]
    fn negamax(
        &mut self,
        pos: B,
        ply: usize,
        depth: isize,
        mut alpha: Score,
        beta: Score,
    ) -> Score {
        debug_assert!(alpha < beta);
        debug_assert!(ply <= MAX_DEPTH.get() * 2);
        debug_assert!(depth <= MAX_DEPTH.isize());
        self.state
            .statistics
            .count_node_started(MainSearch, ply, true);

        if let Some(res) = pos.player_result_no_movegen(&self.state.board_history) {
            return game_result_to_score(res, ply);
        }
        if depth <= 0 {
            return self.eval.eval(&pos);
        }

        let mut best_score = SCORE_LOST;
        let mut num_children = 0;

        for mov in pos.pseudolegal_moves() {
            if ply == 0 && self.state.excluded_moves.contains(&mov) {
                continue;
            }
            let new_pos = pos.make_move(mov);
            if new_pos.is_none() {
                continue; // illegal pseudolegal move
            }
            num_children += 1;
            self.state.statistics.count_legal_make_move(MainSearch);

            self.state.board_history.push(&pos);

            let score = -self.negamax(new_pos.unwrap(), ply + 1, depth - 1, -beta, -alpha);

            self.state.board_history.pop();

            if self.should_stop(self.state.limit) {
                return SCORE_TIME_UP;
            }

            if score <= best_score {
                continue;
            }
            alpha = alpha.max(score);
            best_score = score;
            if ply == 0 {
                self.state.custom.chosen_move = Some(mov);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::chess::lite::LiTEval;
    use crate::eval::mnk::base::BasicMnkEval;
    use crate::search::tests::generic_engine_test;
    use gears::games::ataxx::AtaxxBoard;
    use gears::games::chess::Chessboard;
    use gears::games::mnk::MNKBoard;

    #[test]
    fn generic_test() {
        generic_engine_test::<Chessboard, Gaps<Chessboard>>(Gaps::for_eval::<LiTEval>());
        generic_engine_test::<MNKBoard, Gaps<MNKBoard>>(Gaps::for_eval::<BasicMnkEval>());
        generic_engine_test::<AtaxxBoard, Gaps<AtaxxBoard>>(Gaps::for_eval::<RandEval>());
    }
}
