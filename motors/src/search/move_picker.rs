use crate::search::move_picker::MovePickerState::*;
use crate::search::{Engine, MoveScore, MoveScorer, SearchStateFor};
use gears::arrayvec::{ArrayVec, IntoIter};
use gears::general::board::Board;
use gears::general::move_list::MoveList;
use gears::general::moves::Move;
use gears::itertools::Itertools;
use std::cmp::Ordering;
use std::marker::PhantomData;

#[derive(Debug, Clone, Eq, PartialEq)]
// Merge move and score into a single u32 to make the hot loop of finding the best move faster. Idea from 87flowers.
pub struct ScoredMove<B: Board>(u32, PhantomData<B>);

impl<B: Board> Copy for ScoredMove<B> {}

impl<B: Board> PartialOrd for ScoredMove<B> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl<B: Board> Ord for ScoredMove<B> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<B: Board> ScoredMove<B> {
    pub fn new(mov: B::Move, score: MoveScore) -> Self {
        // ScoredMove(score, mov)
        // TODO: Doesn't work for 32 bit moves (which currently don't use this struct, so it's fine for now)
        debug_assert!(mov.to_underlying().into() <= u16::MAX.into());
        Self(((score.0.wrapping_sub(i16::MIN) as u16 as u32) << 16) | (mov.to_underlying().into() as u32), PhantomData)
    }

    pub fn mov(&self) -> B::Move {
        // self.1
        B::Move::from_u64_unchecked((self.0 & 0xff_ff) as u64).trust_unchecked()
    }

    pub fn score(&self) -> MoveScore {
        MoveScore(((self.0 >> 16) as i16).wrapping_add(i16::MIN))
        // self.0
    }
}

#[expect(type_alias_bounds)]
type ScoredMoveList<B: Board, const MAX_LEN: usize> = ArrayVec<ScoredMove<B>, MAX_LEN>;

struct UnscoredMoveIter<B: Board, const MAX_LEN: usize>(IntoIter<ScoredMove<B>, MAX_LEN>);

impl<B: Board, const MAX_LEN: usize> Iterator for UnscoredMoveIter<B, MAX_LEN> {
    type Item = B::Move;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|sm| sm.mov())
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
            self.list.push(ScoredMove::new(mov, score));
        }
    }

    fn num_moves(&self) -> usize {
        self.list.len()
    }

    fn swap_remove_move(&mut self, idx: usize) -> B::Move {
        self.list.swap_remove(idx).mov()
    }

    fn iter_moves(&self) -> impl Iterator<Item = B::Move> {
        self.list.iter().map(|sm| sm.mov())
    }

    fn remove(&mut self, to_remove: B::Move) {
        if let Some((idx, _)) = self.list.iter().find_position(|sm| sm.mov() == to_remove) {
            _ = self.swap_remove_move(idx);
        }
    }

    fn filter_moves<F: Fn(&mut B::Move) -> bool>(&mut self, predicate: F) {
        self.list.retain(|sm| predicate(&mut sm.mov()));
    }
}

enum MovePickerState {
    TTMove,
    List,
    DeferredList,
}

pub struct MovePicker<'a, B: Board, const MAX_LEN: usize> {
    state: MovePickerState,
    list: ScoredMoveList<B, MAX_LEN>,
    pos: &'a B,
    tactical_only: bool,
    tt_move: B::Move,
    ignored_prefix: usize,
}

impl<'a, B: Board, const MAX_LEN: usize> MovePicker<'a, B, MAX_LEN> {
    /// Assumes that better moves have a *higher* score.
    pub fn new(pos: &'a B, best: B::Move, tactical_only: bool) -> Self {
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
                Some(ScoredMove::new(self.tt_move, MoveScore::MAX))
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
            DeferredList => {
                debug_assert!(self.list[self.ignored_prefix..].iter().rev().is_sorted());
                self.next_from_deferred(Scorer::DEFERRED_OFFSET)
            }
        }
    }

    fn next_from_list<E: Engine<B>, Scorer: MoveScorer<B, E>>(&mut self, scorer: &Scorer) -> Option<ScoredMove<B>> {
        loop {
            let idx = self.list[self.ignored_prefix..].into_iter().position_max()? + self.ignored_prefix;
            if scorer.defer_playing_move(self.list[idx].mov()) {
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
        // TODO: Can probably be optimized / simplified
        res.copied().map(|m| ScoredMove::new(m.mov(), m.score() + offset))
    }
}

#[cfg(test)]
mod tests {
    use crate::search::MoveScore;
    use crate::search::move_picker::ScoredMove;
    use gears::games::chess::Chessboard;
    use gears::general::board::{Board, BoardHelpers};
    use proptest::proptest;

    proptest! {
        #[test]
        fn chess_scored_moves(score in i16::MIN..=i16::MAX) {
            let score = MoveScore(score);
            for p in Chessboard::bench_positions() {
                for m in p.pseudolegal_moves() {
                    let sm = ScoredMove::<Chessboard>::new(m, score);
                    assert_eq!(m, sm.mov());
                    assert_eq!(score, sm.score());
                }
            }
        }
    }
}
