use std::mem::size_of;

use static_assertions::const_assert_eq;

use crate::games::chess::Chessboard;
use crate::games::{Board, ZobristHash};
use crate::search::Score;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub(super) enum TTScoreType {
    #[default]
    Empty,
    LowerBound,
    Exact,
    UpperBound,
}

#[derive(Debug, Copy, Clone, Default)]
#[repr(C)] // probably unnecessary, but allows asserting that the size behaves as expected
pub(super) struct TTEntry<B: Board> {
    pub hash: ZobristHash,  // 8 bytes
    pub score: Score,       // 4 bytes
    pub mov: B::Move,       // depends, 2 bytes for chess (atm never more)
    pub depth: u8,          // 1 byte
    pub bound: TTScoreType, // 1 byte
}

impl<B: Board> TTEntry<B> {
    pub fn new(
        score: Score,
        mov: B::Move,
        depth: isize,
        bound: TTScoreType,
        hash: ZobristHash,
    ) -> TTEntry<B> {
        let depth = depth.clamp(0, u8::MAX as isize) as u8;
        Self {
            score,
            mov,
            depth,
            bound,
            hash,
        }
    }
}
const_assert_eq!(size_of::<TTEntry<Chessboard>>(), 16);

pub(super) const DEFAULT_HASH_SIZE_MB: usize = 4;

#[derive(Debug)]
pub(super) struct TT<B: Board> {
    arr: Vec<TTEntry<B>>,
    mask: u64,
}

impl<B: Board> Default for TT<B> {
    fn default() -> Self {
        TT::new_with_bytes(DEFAULT_HASH_SIZE_MB * 1000_000)
    }
}

impl<B: Board> TT<B> {
    pub fn empty() -> Self {
        Self {
            arr: vec![],
            mask: 0,
        }
    }

    pub fn new_with_bytes(size_in_bytes: usize) -> Self {
        let mut res = Self {
            arr: vec![],
            mask: 0,
        };
        res.resize_bytes(size_in_bytes);
        res
    }

    pub fn resize_bytes(&mut self, new_size_in_bytes: usize) {
        let new_size = new_size_in_bytes / size_of::<TTEntry<B>>();
        let num_bits = new_size.ilog2() as usize;
        let new_size = 1 << num_bits; // round down to power of two
        self.arr.resize_with(new_size, Default::default);
        self.mask = new_size as u64 - 1;
    }

    fn index_of(&self, hash: ZobristHash) -> usize {
        (hash.0 % self.mask) as usize
    }

    pub fn store(&mut self, mut entry: TTEntry<B>, ply: usize) {
        let idx = self.index_of(entry.hash);
        if let Some(plies) = entry.score.plies_until_game_won() {
            if plies < 0 {
                entry.score.0 -= ply as i32;
            } else {
                entry.score.0 += ply as i32;
            }
        }
        self.arr[idx] = entry;
    }

    pub fn load(&self, hash: ZobristHash, ply: usize) -> TTEntry<B> {
        let idx = self.index_of(hash);
        let mut entry = self.arr[idx];
        if let Some(plies) = entry.score.plies_until_game_won() {
            if plies < 0 {
                entry.score.0 += ply as i32;
            } else {
                entry.score.0 -= ply as i32;
            }
        }
        entry
    }
}
