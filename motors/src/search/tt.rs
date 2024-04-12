use std::mem::size_of;
use std::sync::Arc;
use std::sync::atomic::Ordering::Relaxed;

use portable_atomic::AtomicU128;
use static_assertions::const_assert_eq;

use gears::games::{Board, Move, ZobristHash};
#[cfg(feature = "chess")]
use gears::games::chess::Chessboard;
use gears::search::{Score, SCORE_WON};

use crate::search::NodeType;

type AtomicTTEntry = AtomicU128;

#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
#[repr(C)]
pub(super) struct TTEntry<B: Board> {
    pub hash: ZobristHash, // 8 bytes
    pub score: Score,      // 4 bytes
    pub mov: B::Move,      // depends, 2 bytes for chess (atm never more)
    pub depth: u8,         // 1 byte
    pub bound: NodeType,   // 1 byte
}

impl<B: Board> TTEntry<B> {
    pub fn new(
        hash: ZobristHash,
        score: Score,
        mov: B::Move,
        depth: isize,
        bound: NodeType,
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
    #[cfg(feature = "unsafe")]
    unsafe fn to_packed(self) -> u128 {
        transmute::<Self, AtomicTTEntry>(self)
    }

    #[cfg(not(feature = "unsafe"))]
    fn to_packed(self) -> u128 {
        let score = self.score.0 as u32; // don't sign extend negative scores
        ((self.hash.0 as u128) << 64)
            | ((score as u128) << (64 - 32))
            | ((self.mov.to_underlying().into() as u128) << 16)
            | ((self.depth as u128) << 8)
            | self.bound as u128
    }

    #[cfg(feature = "unsafe")]
    unsafe fn from_packed(packed: u128) -> Self {
        transmute::<u128, Self>(packed)
    }

    #[cfg(not(feature = "unsafe"))]
    fn from_packed(val: u128) -> Self {
        let hash = ZobristHash((val >> 64) as u64);
        let score = Score(((val >> (64 - 32)) & 0xffff_ffff) as i32);
        let mov = B::Move::from_usize(((val >> 16) & 0xffff) as usize).unwrap();
        let depth = ((val >> 8) & 0xff) as u8;
        let bound = NodeType::from_repr((val & 0xff) as u8).unwrap();
        Self {
            hash,
            score,
            mov,
            depth,
            bound,
        }
    }
}
#[cfg(feature = "chess")]
const_assert_eq!(size_of::<TTEntry<Chessboard>>(), 16);
#[cfg(feature = "chess")]
const_assert_eq!(size_of::<TTEntry<Chessboard>>(), size_of::<AtomicTTEntry>());

pub(super) const DEFAULT_HASH_SIZE_MB: usize = 4;

/// Ideally, a TT would be something like an `Arc<[AtomicTTEntry]>`, but creating an `Arc<Slice>` isn't exactly stable Rust.
/// Another option would be to have some kind of TT owner and give each thread a reference to the TT, but that would require
/// tons of lifetime annotations.
/// So instead, a TT is an `Arc<Vec<AtomicTTEntry>>`. Note that resizing the TT during search will wait until the search is finished
/// (all threads will receive a new reference)

#[derive(Debug)]
pub(super) struct SharedTTState {
    arr: Vec<AtomicTTEntry>,
}

// TODO: Get rid of the double dereferencing by keeping a reference to the slice.
#[derive(Clone, Debug)]
pub struct TT {
    tt: Arc<SharedTTState>,
    mask: usize,
}

impl Default for TT {
    fn default() -> Self {
        TT::new_with_bytes(DEFAULT_HASH_SIZE_MB * 1_000_000)
    }
}

impl TT {
    pub fn new_with_bytes(size_in_bytes: usize) -> Self {
        let new_size = size_in_bytes / size_of::<AtomicTTEntry>();
        let num_bits = new_size.ilog2() as u64;
        let new_size = 1 << num_bits; // round down to power of two
        let mut arr = vec![];
        arr.resize_with(new_size, || AtomicU128::default());
        Self {
            tt: Arc::new(SharedTTState { arr }),
            mask: new_size - 1,
        }
    }

    pub fn forget(&mut self) {
        for entry in self.tt.arr.iter() {
            entry.store(0, Relaxed);
        }
    }

    fn index_of(&self, hash: ZobristHash) -> usize {
        hash.0 as usize & self.mask
    }

    pub fn store<B: Board>(&mut self, mut entry: TTEntry<B>, ply: usize) {
        debug_assert!(
            entry.score.0.abs() + ply as i32 <= SCORE_WON.0,
            "score {score} ply {ply}",
            score = entry.score.0
        );
        let idx = self.index_of(entry.hash);
        // Mate score adjustments: For the current search, we want to penalize later mates to prefer earlier ones,
        // where "later" means being found at greater depth (usually called the `ply` parameter in the search function).
        // But since the TT persists across searches and can also reach the same position at different plies though transpositions,
        // we undo that when storing mate scores, and reapply the penalty for the *current* ply when loading mate scores.
        if let Some(plies) = entry.score.plies_until_game_won() {
            if plies < 0 {
                entry.score.0 -= ply as i32;
            } else {
                entry.score.0 += ply as i32;
            }
        }
        debug_assert!(
            entry.score.0.abs() <= SCORE_WON.0,
            "score {}, ply {ply}, won in {won}",
            entry.score.0,
            won = entry.score.plies_until_game_won().unwrap_or(42),
        );
        self.tt.arr[idx].store(entry.to_packed(), Relaxed);
    }

    pub fn load<B: Board>(&self, hash: ZobristHash, ply: usize) -> TTEntry<B> {
        let idx = self.index_of(hash);
        let mut entry = TTEntry::from_packed(self.tt.arr[idx].load(Relaxed));
        // Mate score adjustments, see `store`
        if let Some(plies) = entry.score.plies_until_game_won() {
            if plies < 0 {
                entry.score.0 += ply as i32;
            } else {
                entry.score.0 -= ply as i32;
            }
        }
        debug_assert!(entry.score.0.abs() <= SCORE_WON.0);
        entry
    }
}

#[cfg(test)]
mod test {
    use rand::{Rng, thread_rng};
    use rand::distributions::Uniform;

    use gears::search::{MAX_NORMAL_SCORE, MIN_NORMAL_SCORE};

    use super::*;

    #[test]
    #[cfg(feature = "chess")]
    fn test_packing() {
        let board = Chessboard::from_name("kiwipete").unwrap();
        let mut i = 1;
        for mov in board.pseudolegal_moves() {
            let entry: TTEntry<Chessboard> = TTEntry::new(
                board.zobrist_hash(),
                Score(i * i * (i % 2 * 2 - 1)),
                mov,
                i as isize,
                NodeType::from_repr(i as u8 % 3).unwrap(),
            );
            let converted = entry.to_packed();
            assert_eq!(TTEntry::from_packed(converted), entry);
        }
    }

    #[test]
    #[cfg(feature = "chess")]
    fn test_load_store() {
        for pos in Chessboard::bench_positions() {
            let num_bytes_in_size = thread_rng().sample(Uniform::new(4, 25));
            let size_in_bytes = (1 << num_bytes_in_size)
                + thread_rng().sample(Uniform::new(0, 1 << num_bytes_in_size));
            let mut tt = TT::new_with_bytes(size_in_bytes);
            for mov in pos.pseudolegal_moves() {
                let score = Score(
                    thread_rng().sample(Uniform::new(MIN_NORMAL_SCORE.0, MAX_NORMAL_SCORE.0)),
                );
                let depth = thread_rng().sample(Uniform::new(1, 100));
                let bound = NodeType::from_repr(thread_rng().sample(Uniform::new(0, 3))).unwrap();
                let entry: TTEntry<Chessboard> = TTEntry {
                    hash: pos.zobrist_hash(),
                    score,
                    mov,
                    depth,
                    bound,
                };
                let packed = entry.to_packed();
                let val = TTEntry::from_packed(packed);
                assert_eq!(val, entry);
                let ply = thread_rng().sample(Uniform::new(0, 100));
                tt.store(entry.clone(), ply);
                let loaded = tt.load(entry.hash, ply);
                assert_eq!(entry, loaded);
            }
        }
    }
}