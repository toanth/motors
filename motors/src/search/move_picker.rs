use arrayvec::ArrayVec;
use gears::games::Board;
use gears::general::move_list::MoveList;
use itertools::Itertools;

pub struct MovePicker<B: Board, const MAX_LEN: usize> {
    moves: B::MoveList,
    scores: ArrayVec<i32, MAX_LEN>,
}

impl<B: Board, const MAX_LEN: usize> MovePicker<B, MAX_LEN> {
    /// Assumes that better moves have a *higher* score.
    pub fn new<ScoreFn: Fn(B::Move) -> i32>(moves: B::MoveList, score_function: ScoreFn) -> Self {
        let mut scores = ArrayVec::default();
        for mov in moves.iter_moves() {
            scores.push(score_function(*mov))
        }
        Self { moves, scores }
    }

    pub fn next_move_and_score(&mut self) -> Option<(B::Move, i32)> {
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

pub struct MovePickIter<B: Board, const MAX_LEN: usize> {
    move_picker: MovePicker<B, MAX_LEN>,
}

impl<B: Board, const MAX_LEN: usize> Iterator for MovePickIter<B, MAX_LEN> {
    type Item = (B::Move, i32);

    fn next(&mut self) -> Option<Self::Item> {
        self.move_picker.next_move_and_score()
    }
}

impl<B: Board, const MAX_LEN: usize> IntoIterator for MovePicker<B, MAX_LEN> {
    type Item = (B::Move, i32);
    type IntoIter = MovePickIter<B, MAX_LEN>;

    fn into_iter(self) -> Self::IntoIter {
        MovePickIter { move_picker: self }
    }
}
