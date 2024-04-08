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

#[derive(Debug, Copy, Clone, Default)]
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
        ((self.hash.0 as u128) << 64)
            | ((self.score.0 as u128) << (64 - 32))
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
        let hash = ZobristHash(((val >> 64) & 0xffff_ffff_ffff_ffff) as u64);
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
