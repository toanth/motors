use crate::search::move_picker::MovePickerState::{BeginList, List, TTMove};
use crate::search::{MoveScore, MoveScorer};
use arrayvec::ArrayVec;
use gears::games::{Board, Move};
use gears::general::move_list::MoveList;
use itertools::Itertools;

pub struct MovePicker<B: Board, const MAX_LEN: usize> {
    state: MovePickerState<B, MAX_LEN>,
    pos: B,
    tactical_only: bool,
    tt_move: B::Move,
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
        exclude: B::Move,
    ) -> Self {
        let mut moves = if tactical_only {
            pos.tactical_pseudolegal()
        } else {
            pos.pseudolegal_moves()
        };
        if exclude != B::Move::default() {
            moves.remove(exclude);
        }
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
        if self.scores[idx] == MoveScore::IGNORE_MOVE {
            return None;
        }
        Some((
            self.moves.swap_remove_move(idx),
            self.scores.swap_remove(idx),
        ))
    }
}

enum MovePickerState<B: Board, const MAX_LEN: usize> {
    TTMove,
    BeginList,
    List(ScoredMoveList<B, MAX_LEN>),
}

impl<B: Board, const MAX_LEN: usize> MovePicker<B, MAX_LEN> {
    /// Assumes that better moves have a *higher* score.
    pub fn new(pos: B, best: B::Move, tactical_only: bool) -> Self {
        // TODO: Test always playing the TT move in qsearch, even if not tactical
        let state = if pos.is_move_pseudolegal(best) && (!tactical_only || best.is_tactical(&pos)) {
            TTMove
        } else {
            BeginList
        };
        Self {
            state,
            pos,
            tactical_only,
            tt_move: best,
        }
    }

    pub fn next<Scorer: MoveScorer<B>>(
        &mut self,
        scorer: &Scorer,
        state: &Scorer::State,
    ) -> Option<(B::Move, MoveScore)> {
        match &mut self.state {
            TTMove => {
                self.state = BeginList;
                Some((self.tt_move, MoveScore::MAX))
            }
            BeginList => {
                let mut list =
                    ScoredMoveList::new(self.tactical_only, &self.pos, scorer, state, self.tt_move);
                let res = list.next();
                self.state = List(list);
                res
            }
            List(list) => list.next(),
        }
    }
}
