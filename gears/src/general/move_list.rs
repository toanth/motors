use crate::games::Board;
use arrayvec::ArrayVec;

/// A list of moves as returned by the board's `pseudolegal_moves`.
/// Moves may or may not be ordered and may or may not be computed lazily.
pub trait MoveList<B: Board>: IntoIterator<Item = B::Move> {
    /// Returns false iff this struct is essentially a Vec-like container.
    fn is_lazy() -> bool;

    fn add_move(&mut self, mov: B::Move);

    /// Moves the last currently considered move to the `idx`th element and returns that.
    fn swap_remove_move(&mut self, idx: usize) -> B::Move;

    fn iter_moves(&self) -> impl Iterator<Item = &B::Move>;
}

/// A list of moves that is computed all at once and stored in-place.
pub type EagerNonAllocMoveList<B: Board, const N: usize> = ArrayVec<B::Move, N>;

impl<B: Board, const N: usize> MoveList<B> for EagerNonAllocMoveList<B, N> {
    fn is_lazy() -> bool {
        false
    }

    fn add_move(&mut self, mov: B::Move) {
        self.push(mov)
    }

    fn swap_remove_move(&mut self, idx: usize) -> B::Move {
        self.swap_remove(idx)
    }

    fn iter_moves(&self) -> impl Iterator<Item = &B::Move> {
        self.iter()
    }
}
