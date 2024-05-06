use crate::games::Board;
use arrayvec::ArrayVec;

/// A list of moves as returned by the board's `pseudolegal_moves`.
/// Moves may or may not be ordered and may or may not be computed lazily.
pub trait MoveList<B: Board>: IntoIterator<Item = B::Move> {
    /// Returns false iff this struct is essentially a Vec-like container.
    fn is_lazy() -> bool;

    fn add_move(&mut self, mov: B::Move);
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
}
