use crate::search::move_picker::MovePickerState::*;
use crate::search::{Engine, MoveScore, MoveScorer, SearchStateFor};
use gears::arrayvec::{ArrayVec, IntoIter};
use gears::general::board::Board;
use gears::general::move_list::MoveList;
use gears::general::moves::Move;
use gears::itertools::Itertools;

#[expect(type_alias_bounds)]
pub type ScoredMove<B: Board> = (B::Move, MoveScore);

#[expect(type_alias_bounds)]
type ScoredMoveList<B: Board, const MAX_LEN: usize> = ArrayVec<ScoredMove<B>, MAX_LEN>;

struct UnscoredMoveIter<B: Board, const MAX_LEN: usize>(IntoIter<ScoredMove<B>, MAX_LEN>);

impl<B: Board, const MAX_LEN: usize> Iterator for UnscoredMoveIter<B, MAX_LEN> {
    type Item = B::Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(mov, _score)| mov)
    }
}

#[derive(Debug)]
struct MoveListScorer<'a, B: Board, E: Engine<B>, const MAX_LEN: usize, Scorer: MoveScorer<B, E>> {
    list: &'a mut ScoredMoveList<B, MAX_LEN>,
    scorer: &'a Scorer,
    state: &'a SearchStateFor<B, E>,
    excluded: B::Move,
}

impl<B: Board, E: Engine<B>, const MAX_LEN: usize, Scorer: MoveScorer<B, E>> IntoIterator
    for MoveListScorer<'_, B, E, MAX_LEN, Scorer>
{
    type Item = B::Move;
    type IntoIter = UnscoredMoveIter<B, MAX_LEN>;

    fn into_iter(self) -> Self::IntoIter {
        UnscoredMoveIter(self.list.take().into_iter())
    }
}

impl<B: Board, E: Engine<B>, const MAX_LEN: usize, Scorer: MoveScorer<B, E>> MoveList<B>
    for MoveListScorer<'_, B, E, MAX_LEN, Scorer>
{
    fn add_move(&mut self, mov: B::Move) {
        if self.excluded != mov {
            let score = self.scorer.score_move_eager_part(mov, self.state);
            self.list.push((mov, score));
        }
    }

    fn num_moves(&self) -> usize {
        self.list.len()
    }

    fn swap_remove_move(&mut self, idx: usize) -> B::Move {
        self.list.swap_remove(idx).0
    }

    fn iter_moves(&self) -> impl Iterator<Item = &B::Move> {
        self.list.iter().map(|(mov, _)| mov)
    }

    fn remove(&mut self, to_remove: B::Move) {
        if let Some((idx, _)) = self.list.iter().find_position(|(mov, _)| *mov == to_remove) {
            _ = self.swap_remove_move(idx);
        }
    }

    fn filter_moves<F: Fn(&mut B::Move) -> bool>(&mut self, predicate: F) {
        self.list.retain(|(mov, _)| predicate(mov));
    }
}

enum MovePickerState {
    TTMove,
    List,
    DeferredList,
}

pub struct MovePicker<B: Board, const MAX_LEN: usize> {
    state: MovePickerState,
    list: ScoredMoveList<B, MAX_LEN>,
    pos: B,
    tactical_only: bool,
    tt_move: B::Move,
    ignored_prefix: usize,
}

impl<B: Board, const MAX_LEN: usize> MovePicker<B, MAX_LEN> {
    /// Assumes that better moves have a *higher* score.
    pub fn new(pos: B, best: B::Move, tactical_only: bool) -> Self {
        let state = if pos.is_generated_move_pseudolegal(best) && (!tactical_only || best.is_tactical(&pos)) {
            TTMove
        } else {
            List
        };
        Self {
            state,
            list: ScoredMoveList::<B, MAX_LEN>::default(),
            pos,
            tactical_only,
            tt_move: best,
            ignored_prefix: usize::MAX,
        }
    }

    pub fn next<E: Engine<B>, Scorer: MoveScorer<B, E>>(
        &mut self,
        scorer: &Scorer,
        state: &SearchStateFor<B, E>,
    ) -> Option<ScoredMove<B>> {
        match self.state {
            TTMove => {
                self.state = List;
                Some((self.tt_move, MoveScore::MAX))
            }
            List => {
                if self.ignored_prefix == usize::MAX {
                    let mut list_scorer =
                        MoveListScorer { list: &mut self.list, scorer, state, excluded: self.tt_move };
                    if self.tactical_only {
                        self.pos.gen_tactical_pseudolegal(&mut list_scorer);
                    } else {
                        self.pos.gen_pseudolegal(&mut list_scorer);
                    }
                    self.ignored_prefix = 0;
                }
                if let Some(res) = self.next_from_list(scorer) {
                    return Some(res);
                }
                self.state = DeferredList;
                self.ignored_prefix = 0;
                self.next_from_deferred(Scorer::DEFERRED_OFFSET)
            }
            DeferredList => self.next_from_deferred(Scorer::DEFERRED_OFFSET),
        }
    }

    fn next_from_list<E: Engine<B>, Scorer: MoveScorer<B, E>>(&mut self, scorer: &Scorer) -> Option<ScoredMove<B>> {
        loop {
            if self.ignored_prefix >= self.list.len() {
                return None;
            }
            let idx = self.list[self.ignored_prefix..].iter().map(|(_mov, score)| score).position_max()?
                + self.ignored_prefix;
            if scorer.defer_playing_move(self.list[idx].0) {
                self.list.swap(self.ignored_prefix, idx);
                self.ignored_prefix += 1;
                continue;
            } else {
                return Some(self.list.swap_remove(idx));
            }
        }
    }

    fn next_from_deferred(&mut self, offset: MoveScore) -> Option<ScoredMove<B>> {
        let res = self.list.get(self.ignored_prefix);
        self.ignored_prefix += 1;
        res.copied().map(|m| (m.0, m.1 + offset))
    }
}
