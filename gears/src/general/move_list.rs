use crate::general::board::Board;
use arrayvec::ArrayVec;
use smallvec::SmallVec;
use std::fmt::Debug;

/// A list of moves as returned by the board's `pseudolegal_moves`.
/// Moves may or may not be ordered and may or may not be computed lazily.
pub trait MoveList<B: Board>: IntoIterator<Item = B::Move, IntoIter: Send> + Debug {
    fn add_move(&mut self, mov: B::Move);

    fn num_moves(&self) -> usize;

    /// Moves the last currently considered move to the `idx`th element and returns that.
    fn swap_remove_move(&mut self, idx: usize) -> B::Move;

    /// Doesn't guarantee any particular iteration order
    fn iter_moves(&self) -> impl Iterator<Item = &B::Move> + Send;

    fn remove(&mut self, to_remove: B::Move);

    fn filter_moves<F: Fn(&mut B::Move) -> bool>(&mut self, predicate: F);
}

#[allow(type_alias_bounds)]
pub type MoveIter<B: Board> = <B::MoveList as IntoIterator>::IntoIter;

/// A list of moves that is computed all at once and stored in-place.
#[allow(type_alias_bounds)]
pub type InplaceMoveList<B: Board, const N: usize> = ArrayVec<B::Move, N>;

impl<B: Board, const N: usize> MoveList<B> for InplaceMoveList<B, N> {
    fn add_move(&mut self, mov: B::Move) {
        self.push(mov);
    }

    fn num_moves(&self) -> usize {
        self.len()
    }

    fn swap_remove_move(&mut self, idx: usize) -> B::Move {
        self.swap_remove(idx)
    }

    fn iter_moves(&self) -> impl Iterator<Item = &B::Move> {
        self.iter()
    }

    fn remove(&mut self, to_remove: B::Move) {
        if let Some(idx) = self.iter().position(|m| *m == to_remove) {
            _ = self.swap_remove(idx);
        }
    }

    fn filter_moves<F: Fn(&mut B::Move) -> bool>(&mut self, predicate: F) {
        self.retain(predicate)
    }
}

#[allow(type_alias_bounds)]
pub type SboMoveList<B: Board, const N: usize> = SmallVec<B::Move, N>;

impl<B: Board, const N: usize> MoveList<B> for SboMoveList<B, N> {
    fn add_move(&mut self, mov: B::Move) {
        self.push(mov)
    }

    fn num_moves(&self) -> usize {
        self.len()
    }

    fn swap_remove_move(&mut self, idx: usize) -> B::Move {
        self.swap_remove(idx)
    }

    fn iter_moves(&self) -> impl Iterator<Item = &B::Move> + Send {
        self.iter()
    }

    fn remove(&mut self, to_remove: B::Move) {
        if let Some(idx) = self.iter().position(|m| *m == to_remove) {
            _ = self.swap_remove(idx);
        }
    }

    fn filter_moves<F: Fn(&mut B::Move) -> bool>(&mut self, predicate: F) {
        self.retain(predicate)
    }
}
