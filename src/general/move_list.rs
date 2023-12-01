use crate::games::Board;

/// A list of moves as returned by the board's `pseudolegal_moves`.
/// Moves may or may not be ordered and may or may not be computed lazily.
pub trait MoveList<B: Board>: Iterator<Item = B::Move> {
    /// Returns false iff this struct is essentially a Vec-like container.
    fn is_lazy() -> bool;

    // /// Returns true iff the moves are sorted such that more promising moves appear first.
    // fn is_sorted(&self) -> bool;

    fn is_empty(&self) -> bool;
}

/// A list of moves that is computed all at once and stored in-place.
#[derive(Debug, Eq, PartialEq)]
pub struct EagerNonAllocMoveList<B: Board, const N: usize> {
    list: [B::Move; N],
    num_moves: usize,
}

impl<B: Board, const N: usize> EagerNonAllocMoveList<B, N> {
    pub fn add_move(&mut self, mov: B::Move) -> &mut Self {
        self.list[self.num_moves] = mov;
        self.num_moves += 1;
        self
    }

    pub fn len(&self) -> usize {
        self.num_moves
    }

    pub fn as_slice(&self) -> &[B::Move] {
        &self.list[..self.num_moves]
    }

    pub fn as_mut_slice(&mut self) -> &mut [B::Move] {
        &mut self.list[..self.num_moves]
    }
}

impl<B: Board, const N: usize> Iterator for EagerNonAllocMoveList<B, N> {
    type Item = B::Move;

    fn next(&mut self) -> Option<Self::Item> {
        if self.num_moves == 0 {
            return None;
        }
        self.num_moves -= 1;
        Some(self.list[self.num_moves])
    }
}

impl<B: Board, const N: usize> FromIterator<B::Move> for EagerNonAllocMoveList<B, N> {
    fn from_iter<T: IntoIterator<Item = B::Move>>(iter: T) -> Self {
        let mut res = Self::default();
        for mov in iter.into_iter() {
            res.add_move(mov);
        }
        res
    }
}

impl<B: Board, const N: usize> MoveList<B> for EagerNonAllocMoveList<B, N> {
    fn is_lazy() -> bool {
        false
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<B: Board, const N: usize> Default for EagerNonAllocMoveList<B, N> {
    fn default() -> Self {
        EagerNonAllocMoveList {
            list: [Default::default(); N],
            num_moves: 0,
        }
    }
}
