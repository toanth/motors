use crate::search::move_picker::MovePickerState::{List, NoTTMove, TTMove};
use crate::search::{MoveScore, MoveScorer};
use arrayvec::ArrayVec;
use gears::games::Board;
use gears::general::move_list::MoveList;
use itertools::Itertools;

pub struct MovePicker<B: Board, const MAX_LEN: usize> {
    state: MovePickerState<B, MAX_LEN>,
    pos: B,
    tactical_only: bool,
}

struct ScoredMoveList<B: Board, const MAX_LEN: usize> {
    moves: B::MoveList,
    scores: ArrayVec<MoveScore, MAX_LEN>,
}

impl<B: Board, const MAX_LEN: usize> ScoredMoveList<B, MAX_LEN> {
    fn new<Scorer: MoveScorer<B>>(
        tactical_only: bool,
        pos: &B,
        scorer: &Scorer,
        state: &Scorer::State,
    ) -> Self {
        let moves = if tactical_only {
            pos.tactical_pseudolegal()
        } else {
            pos.pseudolegal_moves()
        };
        let mut scores = ArrayVec::default();
        for mov in moves.iter_moves() {
            scores.push(scorer.score_move(*mov, state))
        }
        Self { moves, scores }
    }

    fn next(&mut self) -> Option<(B::Move, MoveScore)> {
        if self.scores.is_empty() {
            return None;
        }
        let idx = self.scores.iter().position_max().unwrap();
        Some((
            self.moves.swap_remove_move(idx),
            self.scores.swap_remove(idx),
        ))
    }
}

enum MovePickerState<B: Board, const MAX_LEN: usize> {
    NoTTMove,
    TTMove(B::Move),
    List(ScoredMoveList<B, MAX_LEN>),
}

impl<B: Board, const MAX_LEN: usize> MovePicker<B, MAX_LEN> {
    /// Assumes that better moves have a *higher* score.
    pub fn new(pos: B, best: B::Move, tactical_only: bool) -> Self {
        // if pos.is_move_pseudolegal(best) {
        //     Self {
        //         state: TTMove(best),
        //         pos,
        //         tactical_only,
        //     }
        // } else {
        Self {
            state: NoTTMove,
            pos,
            tactical_only,
        }
        // }
    }

    pub fn next<Scorer: MoveScorer<B>>(
        &mut self,
        scorer: &Scorer,
        state: &Scorer::State,
    ) -> Option<(B::Move, MoveScore)> {
        match &mut self.state {
            TTMove(mov) => {
                let res = Some((*mov, MoveScore::MAX));
                self.state = List(ScoredMoveList::new(
                    self.tactical_only,
                    &self.pos,
                    scorer,
                    state,
                ));
                res
            }
            NoTTMove => {
                let mut list = ScoredMoveList::new(self.tactical_only, &self.pos, scorer, state);
                let res = list.next();
                self.state = List(list);
                res
            }
            List(list) => list.next(),
        }
    }
}
