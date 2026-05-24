/*
 *  Motors, a collection of board game engines.
 *  Copyright (C) 2024 ToTheAnd
 *
 *  Motors is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Motors is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Motors. If not, see <https://www.gnu.org/licenses/>.
 */
use crate::search::MoveScore;
use crate::search::chess::caps_values::cc;
use crate::search::chess::histories::{HIST_DIVISOR, HistScoreT};
use crate::search::chess::move_picker::MovePickerState::*;
use crate::search::chess::*;
use gears::games::chess::Board;
use gears::games::chess::moves::Move;
use gears::games::chess::see::SeeScore;
use gears::general::moves::MoveTrait;
use gears::itertools::Itertools;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
// Merge move and score into a single u32 to make the hot loop of finding the best move faster. Idea from 87flowers.
pub struct ScoredMove(u32);

impl ScoredMove {
    pub fn new(mov: Move, score: MoveScore) -> Self {
        // ScoredMove(score, mov)
        Self(((score.0.wrapping_sub(i16::MIN) as u16 as u32) << 16) | (mov.to_underlying() as u32))
    }

    pub fn mov(&self) -> Move {
        Move::from_u64_unchecked((self.0 & 0xff_ff) as u64).trust_unchecked()
    }

    pub fn score(&self) -> MoveScore {
        MoveScore(((self.0 >> 16) as i16).wrapping_add(i16::MIN))
    }
}

pub type ScoredMoveList = ArrayVec<ScoredMove, MAX_CHESS_MOVES_IN_POS>;

pub struct MoveScorer<'a> {
    pos: &'a Board,
    ply: usize,
}

impl<'a> MoveScorer<'a> {
    pub fn new(pos: &'a Board, ply: usize) -> Self {
        Self { pos, ply }
    }

    pub fn score_tactical(&self, mov: Move, state: &CapsState) -> MoveScore {
        debug_assert!(mov.is_tactical(self.pos), "{mov:?} {}", self.pos);
        let captured = mov.captured(self.pos);
        let base_val = MoveScore(HIST_DIVISOR * 10);
        let hist_val = state.capt_hist.get(mov, self.pos);
        base_val + MoveScore(captured as i16 * HIST_DIVISOR) + hist_val
    }

    pub fn score_quiet_nonkiller(&self, mov: Move, state: &CapsState) -> MoveScore {
        debug_assert!(!mov.is_tactical(self.pos), "{mov:?} {}", self.pos);
        let countermove_score = if self.ply > 0 {
            let prev_move = state.search_stack[self.ply - 1].last_tried_move();
            state.countermove_hist.score(mov, self.pos, prev_move, &state.search_stack[self.ply - 1].pos)
        } else {
            0
        };
        let follow_up_score = if self.ply > 1 {
            let prev_move = state.search_stack[self.ply - 2].last_tried_move();
            state.follow_up_move_hist.score(mov, self.pos, prev_move, &state.search_stack[self.ply - 2].pos)
        } else {
            0
        };
        let main_hist_score = state.history.score(mov, self.pos.threats());
        // TODO: Divide at the end (changes bench)
        let score = main_hist_score * cc::main_hist_weight() / 1024
            + countermove_score * cc::countermove_weight() / 1024
            + follow_up_score * cc::follow_up_weight() / 1024;
        MoveScore(score as HistScoreT)
    }

    /// Order moves so that the most promising moves are searched first.
    /// The most promising move is always the TT move, because that is backed up by search.
    /// After that follow various heuristics.
    pub fn score_move_eager_part(&self, mov: Move, state: &CapsState) -> MoveScore {
        // The move list is iterated backwards, which is why better moves get higher scores
        // No need to check against the TT move because that's already handled by the move picker
        if mov.is_tactical(self.pos) {
            self.score_tactical(mov, state)
        } else if mov == state.search_stack[self.ply].killer {
            // `else` ensures that tactical moves can't be killers
            KILLER_SCORE
        } else {
            self.score_quiet_nonkiller(mov, state)
        }
    }

    pub fn complete_move_score(&self, mov: Move, state: &CapsState) -> MoveScore {
        let eager = self.score_move_eager_part(mov, state);
        if mov.is_tactical(self.pos) && defer_playing_move(self.pos, mov, eager) {
            eager + BAD_SEE_OFFSET
        } else {
            eager
        }
    }
}

// Only compute SEE scores for moves when we're actually trying to play them.
// Idea from Cosmo.
fn defer_playing_move(pos: &Board, mov: Move, hist_score: MoveScore) -> bool {
    debug_assert!(mov.is_tactical(pos), "{mov:?} {}", pos);
    let threshold = cc::bad_capt_threshold() - hist_score.0 as i32 * cc::bad_capt_hist_mult() / 1024;
    !pos.see_at_least(mov, SeeScore(threshold))
}

const BAD_SEE_OFFSET: MoveScore = MoveScore(HIST_DIVISOR * -30);

enum MovePickerState {
    TTMove,
    GenCaptures,
    GoodCaptures,
    Killer,
    GenQuiets,
    Quiets,
    BadCaptures,
}

pub struct MovePicker<'a> {
    state: MovePickerState,
    list: ScoredMoveList,
    pos: &'a Board,
    tactical_only: bool,
    tt_move: Move,
    ignored_prefix: usize,
    ply: usize,
}

impl<'a> MovePicker<'a> {
    /// Assumes that better moves have a *higher* score.
    pub fn new(pos: &'a Board, ply: usize, best: Move, tactical_only: bool) -> Self {
        let state = if pos.is_generated_move_pseudolegal(best) && (!tactical_only || best.is_tactical(pos)) {
            TTMove
        } else {
            GenCaptures
        };
        Self { state, list: ScoredMoveList::default(), pos, tactical_only, tt_move: best, ignored_prefix: 0, ply }
    }

    pub fn complete_move_score(&self, mov: Move, state: &CapsState) -> MoveScore {
        let scorer = MoveScorer::new(self.pos, self.ply);
        scorer.complete_move_score(mov, state)
    }

    pub fn next(&mut self, state: &CapsState) -> Option<ScoredMove> {
        loop {
            return match self.state {
                TTMove => {
                    self.state = GenCaptures;
                    Some(ScoredMove::new(self.tt_move, MoveScore::MAX))
                }
                GenCaptures => {
                    let scorer = MoveScorer::new(self.pos, self.ply);
                    let add_move = |mov: Move| {
                        if self.tt_move != mov {
                            let score = scorer.score_tactical(mov, state);
                            self.list.push(ScoredMove::new(mov, score));
                        }
                    };
                    self.pos.gen_tactical_pseudolegal(add_move);
                    self.state = GoodCaptures;
                    continue;
                }
                GoodCaptures => {
                    if let Some(res) = self.next_good_tactical() {
                        return Some(res);
                    }
                    if self.tactical_only {
                        self.ignored_prefix = 0;
                        self.state = BadCaptures
                    } else {
                        self.state = Killer
                    };
                    continue;
                }
                Killer => {
                    let killer = state.search_stack[self.ply].killer;
                    debug_assert!(!self.tactical_only);
                    debug_assert_eq!(self.ignored_prefix, self.list.len());
                    self.state = GenQuiets;
                    if !self.pos.is_generated_move_pseudolegal(killer) || killer.is_tactical(self.pos) {
                        continue;
                    }
                    Some(ScoredMove::new(killer, KILLER_SCORE))
                }
                GenQuiets => {
                    debug_assert_eq!(self.ignored_prefix, self.list.len());
                    let scorer = MoveScorer::new(self.pos, self.ply);
                    let killer = state.search_stack[self.ply].killer;
                    let add_move = |mov: Move| {
                        if self.tt_move != mov && killer != mov {
                            let score = scorer.score_quiet_nonkiller(mov, state);
                            self.list.push(ScoredMove::new(mov, score));
                        }
                    };
                    self.pos.gen_quiet_pseudolegal(add_move);
                    self.state = Quiets;
                    continue;
                }
                Quiets => {
                    debug_assert!(!self.tactical_only);
                    if let Some(res) = self.next_quiet() {
                        return Some(res);
                    }
                    self.state = BadCaptures;
                    self.ignored_prefix = 0;
                    continue;
                }
                BadCaptures => {
                    let res = self.list.get(self.ignored_prefix);
                    self.ignored_prefix += 1;
                    // TODO: Can probably be optimized / simplified
                    res.copied().map(|m| ScoredMove::new(m.mov(), m.score() + BAD_SEE_OFFSET))
                }
            };
        }
    }

    fn next_good_tactical(&mut self) -> Option<ScoredMove> {
        loop {
            let idx = self.list[self.ignored_prefix..].iter().position_max()? + self.ignored_prefix;
            debug_assert!(self.list[idx].mov().is_tactical(self.pos));
            let score = self.list[idx].score();
            if defer_playing_move(self.pos, self.list[idx].mov(), score) {
                self.list.swap(self.ignored_prefix, idx);
                self.ignored_prefix += 1;
                continue;
            } else {
                return Some(self.list.swap_remove(idx));
            }
        }
    }

    fn next_quiet(&mut self) -> Option<ScoredMove> {
        let idx = self.list[self.ignored_prefix..].iter().position_max()? + self.ignored_prefix;
        debug_assert!(!self.list[idx].mov().is_tactical(self.pos));
        Some(self.list.swap_remove(idx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::MoveScore;
    use gears::games::chess::Board;
    use gears::general::board::BoardTrait;
    use proptest::proptest;

    proptest! {
        #[test]
        fn chess_scored_moves(score in i16::MIN..=i16::MAX) {
            let score = MoveScore(score);
            for p in Board::bench_positions() {
                for m in p.pseudolegal_moves() {
                    let sm = ScoredMove::new(m, score);
                    assert_eq!(m, sm.mov());
                    assert_eq!(score, sm.score());
                }
            }
        }
    }
}
