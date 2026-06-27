use std::fmt::Display;

use gears::games::BoardHistDyn;
use gears::general::board::BoardTrait;

use crate::eval::rand_eval::RandEval;
use crate::eval::Eval;
use crate::search::multithreading::ThreadData;
use crate::search::{
    AbstractSearchState, EmptySearchStackEntry, Engine, EngineInfo, NoCustomInfo, NormalEngine, SearchState,
    SearchStateFor,
};
use gears::general::common::StaticallyNamedEntity;
use gears::num::traits::WrappingAdd;
use gears::score::{
    game_result_to_score, Score, MAX_NORMAL_SCORE, MIN_NORMAL_SCORE, SCORE_LOST, SCORE_TIME_UP, SCORE_WON,
};
use gears::search::NodeType::*;
use gears::search::{Budget, DepthPly, NodesLimit, SearchResult};

const MAX_DEPTH: DepthPly = DepthPly::new(100);

type DefaultEval = RandEval;

#[derive(Debug)]
pub struct Gaps<B: BoardTrait> {
    state: SearchState<B, EmptySearchStackEntry, NoCustomInfo>,
    eval: Box<dyn Eval<B>>,
}

impl<B: BoardTrait> Default for Gaps<B> {
    fn default() -> Self {
        Self::new(ThreadData::single_and_no_output(), Box::new(DefaultEval::default()))
    }
}

impl<B: BoardTrait> StaticallyNamedEntity for Gaps<B> {
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

impl<B: BoardTrait> Engine<B> for Gaps<B> {
    type SearchStackEntry = EmptySearchStackEntry;
    type CustomInfo = NoCustomInfo;

    fn new(thread_data: ThreadData<B>, eval: Box<dyn Eval<B>>) -> Self {
        Self { state: SearchState::new(thread_data, MAX_DEPTH), eval }
    }

    fn static_eval(&mut self, pos: &B, ply: usize) -> Score {
        self.eval.eval(pos, ply, self.state.params.pos.active_player()).clamp(MIN_NORMAL_SCORE, MAX_NORMAL_SCORE)
    }

    fn max_bench_depth(&self) -> DepthPly {
        MAX_DEPTH
    }

    fn search_state_dyn(&self) -> &dyn AbstractSearchState<B> {
        &self.state
    }

    fn search_state_mut_dyn(&mut self) -> &mut dyn AbstractSearchState<B> {
        &mut self.state
    }

    fn engine_info(&self) -> EngineInfo {
        EngineInfo::new(
            self,
            self.eval.as_ref(),
            "0.0.1",
            DepthPly::new(4),
            NodesLimit::new(50_000).unwrap(),
            None,
            vec![],
        )
    }

    fn set_eval(&mut self, eval: Box<dyn Eval<B>>) {
        self.eval = eval;
    }

    fn get_eval(&mut self) -> Option<&dyn Eval<B>> {
        Some(self.eval.as_ref())
    }

    fn do_search(&mut self) -> SearchResult<B> {
        let mut limit = self.state.params.limit;
        let max_depth = MAX_DEPTH.min(limit.depth).isize();
        let pos = self.state.params.pos.clone();
        limit.fixed_time = limit.fixed_time.min(limit.tc.remaining);

        self.state.search_params_mut().limit = limit;

        'id: for depth in 1..=max_depth {
            self.state.budget = Budget::new(depth as usize);
            self.state.atomic().set_iteration(depth as usize);
            self.state.atomic().update_seldepth(depth as usize);
            for pv_num in 0..self.state.multi_pv() {
                let elapsed = self.state.start_time().elapsed();
                if self.should_not_start_negamax(
                    elapsed,
                    limit.fixed_time,
                    limit.soft_nodes.get(),
                    depth,
                    max_depth,
                    limit.mate,
                ) {
                    break 'id;
                }

                self.state.current_pv_num = pv_num;
                self.state.atomic().set_iteration(depth as usize);
                self.state.atomic().update_seldepth(depth as usize);
                let iteration_score = self.negamax(pos.clone(), 0, depth, SCORE_LOST, SCORE_WON);
                if self.state.stop_flag() {
                    self.state.cur_pv_data_mut().bound = None;
                    break 'id;
                }
                self.state.cur_pv_data_mut().score = iteration_score;
                self.state.cur_pv_data_mut().bound = Some(Exact);
                // only set now so that incomplete iterations are discarded
                let best_mpv_move = self.state.cur_pv_data().pv.get(0).unwrap_or_default();
                if pv_num == 0 {
                    self.state.atomic().set_score(iteration_score);
                    self.state.atomic().set_best_move(best_mpv_move);
                }
                self.search_state().send_search_info(false);
                self.state.excluded_moves.push(best_mpv_move);
            }
            self.state.excluded_moves.truncate(self.state.excluded_moves.len() - self.state.multi_pv());
        }
        if !self.state.stop_flag() {
            // count an additional node to ensure the game remains reproducible
            _ = self.state.atomic().count_node();
        }
        if self.search_state().output_minimal() {
            self.search_state().send_search_info(true);
        }

        SearchResult::move_and_score(self.state.atomic().best_move(), self.state.atomic().score(), pos)
    }
}

impl<B: BoardTrait> NormalEngine<B> for Gaps<B> {
    fn search_state(&self) -> &SearchStateFor<B, Self> {
        &self.state
    }

    fn search_state_mut(&mut self) -> &mut SearchStateFor<B, Self> {
        &mut self.state
    }
}

impl<B: BoardTrait> Gaps<B> {
    fn eval(&mut self, pos: &B, ply: usize) -> Score {
        let us = self.state.params.pos.active_player();
        let res = self.static_eval(pos, ply);
        let res = if us == pos.active_player() {
            res.wrapping_add(&self.state.params.contempt)
        } else {
            res.wrapping_add(&-self.state.params.contempt)
        };
        res.clamp(MIN_NORMAL_SCORE, MAX_NORMAL_SCORE)
    }

    #[allow(clippy::too_many_arguments)]
    fn negamax(&mut self, pos: B, ply: usize, depth: isize, mut alpha: Score, beta: Score) -> Score {
        debug_assert!(alpha < beta);
        debug_assert!(ply <= MAX_DEPTH.get() * 2);
        debug_assert!(depth <= MAX_DEPTH.isize());

        if self.count_node_and_test_stop() {
            return SCORE_TIME_UP;
        }

        if let Some(res) = pos.player_result_no_movegen(&self.state.params.history) {
            return game_result_to_score(res, ply);
        }
        if depth <= 0 {
            return self.eval(&pos, ply);
        }

        let mut best_score = SCORE_LOST;
        let mut num_children = 0;

        for mov in pos.pseudolegal_moves() {
            let new_pos = pos.clone().make_move(mov);
            if new_pos.is_none() {
                continue; // illegal pseudolegal move
            }
            num_children += 1;
            // increment `num_children` even if the child is excluded
            if ply == 0 && self.state.excluded_moves.contains(&mov) {
                continue;
            }

            self.state.params.history.push(pos.hash_pos());

            let score = -self.negamax(new_pos.unwrap(), ply + 1, depth - 1, -beta, -alpha);

            self.state.params.history.pop();

            if self.state.stop_flag() {
                return SCORE_TIME_UP;
            }

            if score <= best_score {
                continue;
            }
            alpha = alpha.max(score);
            best_score = score;
            if ply == 0 {
                // don't set score here because it's set in `do_search`, which handles situations like the position
                // being checkmate
                self.state.cur_pv_data_mut().pv.reset_to_move(mov);
            }
            if score < beta {
                continue;
            }
            break;
        }
        if num_children == 0 {
            if let Some(res) = pos.no_moves_result() {
                return game_result_to_score(res, ply);
            }
            // if there are no legal moves, the player must pass, and this has to be legal.
            let new_pos = pos.make_nullmove().unwrap();
            best_score = self.negamax(new_pos, ply + 1, depth - 1, -beta, -alpha);
        }
        best_score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::ataxx::bate::Bate;
    use crate::eval::chess::lite::LiTEval;
    use crate::eval::mnk::base::BasicMnkEval;
    use crate::eval::uttt::lute::Lute;
    use crate::search::tests::generic_engine_test;
    use gears::games::chess;
    use gears::games::fairy;
    use gears::games::{ataxx, mnk, uttt};

    #[test]
    fn generic_chess_test() {
        generic_engine_test::<chess::Board, Gaps<chess::Board>>(Gaps::for_eval::<LiTEval>());
    }

    #[test]
    fn generic_mnk_test() {
        generic_engine_test::<mnk::Board, Gaps<mnk::Board>>(Gaps::for_eval::<BasicMnkEval>());
    }

    #[test]
    fn generic_ataxx_test() {
        generic_engine_test::<ataxx::Board, Gaps<ataxx::Board>>(Gaps::for_eval::<Bate>());
    }

    #[test]
    fn generic_uttt_test() {
        generic_engine_test::<uttt::Board, Gaps<uttt::Board>>(Gaps::for_eval::<Lute>());
    }

    #[test]
    fn generic_fairy_test() {
        generic_engine_test::<fairy::Board, Gaps<fairy::Board>>(Gaps::for_eval::<RandEval>())
    }
}
