use std::fmt::Display;

use gears::games::BoardHistory;
use gears::general::board::{Board, BoardHelpers};

use crate::eval::Eval;
use crate::eval::rand_eval::RandEval;
use crate::search::statistics::SearchType::MainSearch;
use crate::search::{
    AbstractSearchState, EmptySearchStackEntry, Engine, EngineInfo, NoCustomInfo, NormalEngine, SearchState,
    SearchStateFor,
};
use gears::general::common::StaticallyNamedEntity;
use gears::score::{
    MAX_NORMAL_SCORE, MIN_NORMAL_SCORE, SCORE_LOST, SCORE_TIME_UP, SCORE_WON, Score, game_result_to_score,
};
use gears::search::NodeType::*;
use gears::search::{Depth, NodesLimit, SearchResult};

const MAX_DEPTH: Depth = Depth::new(100);

type DefaultEval = RandEval;

#[derive(Debug)]
pub struct Gaps<B: Board> {
    state: SearchState<B, EmptySearchStackEntry, NoCustomInfo>,
    eval: Box<dyn Eval<B>>,
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

impl<B: Board> Engine<B> for Gaps<B> {
    type SearchStackEntry = EmptySearchStackEntry;
    type CustomInfo = NoCustomInfo;

    fn with_eval(eval: Box<dyn Eval<B>>) -> Self {
        Self { state: SearchState::new(MAX_DEPTH), eval }
    }

    fn static_eval(&mut self, pos: &B, ply: usize) -> Score {
        self.eval.eval(pos, ply, self.state.params.pos.active_player()).clamp(MIN_NORMAL_SCORE, MAX_NORMAL_SCORE)
    }

    fn max_bench_depth(&self) -> Depth {
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
            Depth::new(4),
            NodesLimit::new(50_000).unwrap(),
            None,
            vec![],
        )
    }

    fn set_eval(&mut self, eval: Box<dyn Eval<B>>) {
        self.eval = eval;
    }

    fn do_search(&mut self) -> SearchResult<B> {
        let mut limit = self.state.params.limit;
        let max_depth = MAX_DEPTH.min(limit.depth).isize();
        let pos = self.state.params.pos.clone();
        limit.fixed_time = limit.fixed_time.min(limit.tc.remaining);

        self.state.statistics.next_id_iteration();
        self.state.search_params_mut().limit = limit;

        'id: for depth in 1..=max_depth {
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
                self.state.atomic().set_depth(depth);
                self.state.atomic().update_seldepth(depth as usize);
                _ = self.state.atomic().count_node();
                let iteration_score = self.negamax(pos.clone(), 0, depth, SCORE_LOST, SCORE_WON);
                self.state.cur_pv_data_mut().score = iteration_score;
                if self.state.stop_flag() {
                    self.state.cur_pv_data_mut().bound = None;
                    break 'id;
                }
                self.state.cur_pv_data_mut().bound = Some(Exact);
                // only set now so that incomplete iterations are discarded
                let best_mpv_move = self.state.cur_pv_data().pv.get(0).unwrap_or_default();
                if pv_num == 0 {
                    self.state.atomic().set_score(iteration_score);
                    self.state.atomic().set_best_move(best_mpv_move);
                }
                self.search_state().send_search_info();
                self.state.excluded_moves.push(best_mpv_move);
            }
            self.state.excluded_moves.truncate(self.state.excluded_moves.len() - self.state.multi_pv());
            self.state.statistics.next_id_iteration();
        }

        SearchResult::move_and_score(self.state.atomic().best_move(), self.state.atomic().score(), pos)
    }
}

impl<B: Board> NormalEngine<B> for Gaps<B> {
    fn search_state(&self) -> &SearchStateFor<B, Self> {
        &self.state
    }

    fn search_state_mut(&mut self) -> &mut SearchStateFor<B, Self> {
        &mut self.state
    }
}

impl<B: Board> Gaps<B> {
    #[allow(clippy::too_many_arguments)]
    fn negamax(&mut self, pos: B, ply: usize, depth: isize, mut alpha: Score, beta: Score) -> Score {
        debug_assert!(alpha < beta);
        debug_assert!(ply <= MAX_DEPTH.get() * 2);
        debug_assert!(depth <= MAX_DEPTH.isize());
        self.state.statistics.count_node_started(MainSearch);

        if let Some(res) = pos.player_result_no_movegen(&self.state.params.history) {
            return game_result_to_score(res, ply);
        }
        if depth <= 0 {
            return self.static_eval(&pos, ply);
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
            self.state.statistics.count_legal_make_move(MainSearch);
            _ = self.state.atomic().count_node();

            self.state.params.history.push(pos.hash_pos());

            let score = -self.negamax(new_pos.unwrap(), ply + 1, depth - 1, -beta, -alpha);

            self.state.params.history.pop();

            if self.should_stop(self.state.uci_nodes()) {
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
        let node_type = best_score.node_type(alpha, beta);
        self.state.statistics.count_complete_node(MainSearch, node_type, depth, ply, num_children);
        if num_children == 0 { game_result_to_score(pos.no_moves_result(), ply) } else { best_score }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::ataxx::bate::Bate;
    use crate::eval::chess::lite::LiTEval;
    use crate::eval::mnk::base::BasicMnkEval;
    use crate::search::tests::generic_engine_test;
    use gears::games::ataxx::AtaxxBoard;
    use gears::games::chess::Chessboard;
    use gears::games::fairy::FairyBoard;
    use gears::games::mnk::MNKBoard;

    #[test]
    fn generic_test() {
        generic_engine_test::<Chessboard, Gaps<Chessboard>>(Gaps::for_eval::<LiTEval>());
        generic_engine_test::<MNKBoard, Gaps<MNKBoard>>(Gaps::for_eval::<BasicMnkEval>());
        generic_engine_test::<AtaxxBoard, Gaps<AtaxxBoard>>(Gaps::for_eval::<Bate>());
        generic_engine_test::<FairyBoard, Gaps<FairyBoard>>(Gaps::for_eval::<RandEval>())
    }
}
