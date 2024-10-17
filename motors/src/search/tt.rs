use std::arch::x86_64::{_mm_prefetch, _MM_HINT_T1};
use std::fmt::{Display, Formatter};
use std::mem::size_of;
#[cfg(feature = "unsafe")]
use std::mem::transmute_copy;
use std::ptr::addr_of;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

use portable_atomic::AtomicU128;
use static_assertions::const_assert_eq;
use strum_macros::FromRepr;

use crate::search::NodeType;
#[cfg(feature = "chess")]
use gears::games::chess::Chessboard;
use gears::games::ZobristHash;
use gears::general::board::Board;
use gears::general::moves::{Move, UntrustedMove};
use gears::score::{Score, ScoreT, SCORE_WON};
use OptionalNodeType::*;

type AtomicTTEntry = AtomicU128;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, FromRepr)]
#[repr(u8)]
enum OptionalNodeType {
    #[default]
    Empty = 0,
    NodeTypeFailHigh = NodeType::FailHigh as u8,
    NodeTypeExact = NodeType::Exact as u8,
    NodeTypeFailLow = NodeType::FailLow as u8,
}

#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
#[repr(C)]
pub struct TTEntry<B: Board> {
    pub hash: ZobristHash,     // 8 bytes
    pub score: Score,          // 4 bytes
    pub mov: UntrustedMove<B>, // depends, 2 bytes for chess (atm never more)
    pub depth: u8,             // 1 byte
    bound: OptionalNodeType,   // 1 byte
}

impl<B: Board> Display for TTEntry<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "move {0} score {1} bound {2} depth {3}",
            self.mov,
            self.score,
            self.bound(),
            self.depth
        )
    }
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
            mov: UntrustedMove::from_move(mov),
            depth,
            bound: OptionalNodeType::from_repr(bound as u8).unwrap(),
            hash,
        }
    }

    pub fn bound(&self) -> NodeType {
        debug_assert!(self.bound != Empty);
        NodeType::from_repr(self.bound as u8).unwrap()
    }

    #[cfg(feature = "unsafe")]
    fn to_packed(self) -> u128 {
        if size_of::<Self>() == 128 / 8 {
            // `transmute_copy` is needed because otherwise the compiler complains that the sizes might not match.
            unsafe { transmute_copy::<Self, u128>(&self) }
        } else {
            self.to_packed_fallback()
        }
    }

    fn to_packed_fallback(self) -> u128 {
        let score = self.score.0 as u32; // don't sign extend negative scores
        ((self.hash.0 as u128) << 64)
            | ((score as u128) << (64 - 32))
            | ((self.mov.to_underlying().into() as u128) << 16)
            | ((self.depth as u128) << 8)
            | self.bound as u128
    }

    #[cfg(not(feature = "unsafe"))]
    fn to_packed(self) -> u128 {
        self.to_packed_fallback()
    }

    #[cfg(feature = "unsafe")]
    fn from_packed(packed: u128) -> Self {
        if size_of::<Self>() == 128 / 8 {
            unsafe { transmute_copy::<u128, Self>(&packed) }
        } else {
            Self::from_packed_fallback(packed)
        }
    }

    #[cfg(not(feature = "unsafe"))]
    fn from_packed(val: u128) -> Self {
        Self::from_packed_fallback(val)
    }

    fn from_packed_fallback(val: u128) -> Self {
        let hash = ZobristHash((val >> 64) as u64);
        let score = Score(((val >> (64 - 32)) & 0xffff_ffff) as ScoreT);
        let mov = B::Move::from_usize_unchecked(((val >> 16) & 0xffff) as usize);
        let depth = ((val >> 8) & 0xff) as u8;
        let bound = OptionalNodeType::from_repr((val & 0xff) as u8).unwrap();
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

pub const DEFAULT_HASH_SIZE_MB: usize = 16;

/// Note that resizing the TT during search will wait until the search is finished
/// (all threads will receive a new arc)
// TODO: TT handle
#[derive(Clone, Debug)]
pub struct TT(pub Arc<[AtomicTTEntry]>);

impl Default for TT {
    fn default() -> Self {
        TT::new_with_bytes(DEFAULT_HASH_SIZE_MB * 1_000_000)
    }
}

impl TT {
    pub fn minimal() -> Self {
        Self::new_with_bytes(0)
    }

    pub fn new_with_bytes(size_in_bytes: usize) -> Self {
        let new_size = 1.max(size_in_bytes / size_of::<AtomicTTEntry>());
        let mut arr = Vec::with_capacity(new_size);
        arr.resize_with(new_size, AtomicU128::default);
        let tt = arr.into_boxed_slice().into();
        Self(tt)
    }

    pub fn size_in_entries(&self) -> usize {
        self.0.len()
    }

    pub fn size_in_bytes(&self) -> usize {
        self.size_in_entries() * size_of::<AtomicTTEntry>()
    }

    pub fn size_in_mb(&self) -> usize {
        (self.size_in_bytes() + 500_000) / 1_000_000
    }

    pub fn forget(&mut self) {
        // TODO: Instead of overwriting every entry, simply increase the age such that old entries will be ignored
        for entry in self.0.iter() {
            entry.store(0, Relaxed);
        }
    }

    /// Counts the number of non-empty entries in the first 1k entries
    // TODO: Use age for a better estimate
    pub fn estimate_hashfull<B: Board>(&self) -> usize {
        let len = 1000.min(self.size_in_entries());
        let num_used = self
            .0
            .iter()
            .take(len)
            .filter(|e: &&AtomicTTEntry| TTEntry::<B>::from_packed(e.load(Relaxed)).bound != Empty)
            .count();
        if len < 1000 {
            (num_used as f64 * 1000.0 / len as f64).round() as usize
        } else {
            num_used
        }
    }

    fn index_of(&self, hash: ZobristHash) -> usize {
        // Uses the multiplication trick from here: <https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/>
        ((hash.0 as u128 * self.size_in_entries() as u128) >> usize::BITS as usize) as usize
    }

    pub(super) fn store<B: Board>(&mut self, mut entry: TTEntry<B>, ply: usize) {
        debug_assert!(
            entry.score.0.abs() + ply as ScoreT <= SCORE_WON.0,
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
                entry.score.0 -= ply as ScoreT;
            } else {
                entry.score.0 += ply as ScoreT;
            }
        }
        debug_assert!(
            entry.score.0.abs() <= SCORE_WON.0,
            "score {}, ply {ply}, won in {won}",
            entry.score.0,
            won = entry.score.plies_until_game_won().unwrap_or(-1),
        );
        self.0[idx].store(entry.to_packed(), Relaxed);
    }

    pub(super) fn load<B: Board>(&self, hash: ZobristHash, ply: usize) -> Option<TTEntry<B>> {
        let idx = self.index_of(hash);
        let mut entry = TTEntry::from_packed(self.0[idx].load(Relaxed));
        // Mate score adjustments, see `store`
        if let Some(plies) = entry.score.plies_until_game_won() {
            if plies < 0 {
                entry.score.0 += ply as ScoreT;
            } else {
                entry.score.0 -= ply as ScoreT;
            }
        }
        debug_assert!(entry.score.0.abs() <= SCORE_WON.0);
        if entry.hash != hash || entry.bound == Empty {
            None
        } else {
            Some(entry)
        }
    }

    #[inline(always)]
    pub fn prefetch(&self, hash: ZobristHash) {
        if cfg!(feature = "unsafe") {
            unsafe {
                #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
                _mm_prefetch::<_MM_HINT_T1>(addr_of!(self.0[self.index_of(hash)]) as *const i8);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::search::chess::caps::Caps;
    use crate::search::multithreading::AtomicSearchState;
    use crate::search::NodeType::Exact;
    use crate::search::{Engine, SearchParams};
    use gears::games::chess::moves::ChessMove;
    use gears::games::ZobristHistory;
    use gears::score::{MAX_NORMAL_SCORE, MIN_NORMAL_SCORE};
    use gears::search::{Depth, SearchLimit};
    use rand::distr::Uniform;
    use rand::{thread_rng, Rng, RngCore};
    use std::thread::{sleep, spawn};
    use std::time::Duration;

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
                NodeType::from_repr(i as u8 % 3 + 1).unwrap(),
            );
            let converted = entry.to_packed();
            assert_eq!(TTEntry::from_packed(converted), entry);
            i += 1;
        }
    }

    #[test]
    #[cfg(feature = "chess")]
    fn test_load_store() {
        for pos in Chessboard::bench_positions() {
            let num_bytes_in_size = thread_rng().sample(Uniform::new(4, 25).unwrap());
            let size_in_bytes = (1 << num_bytes_in_size)
                + thread_rng().sample(Uniform::new(0, 1 << num_bytes_in_size).unwrap());
            let mut tt = TT::new_with_bytes(size_in_bytes);
            for mov in pos.pseudolegal_moves() {
                let score = Score(
                    thread_rng()
                        .sample(Uniform::new(MIN_NORMAL_SCORE.0, MAX_NORMAL_SCORE.0).unwrap()),
                );
                let depth = thread_rng().sample(Uniform::new(1, 100).unwrap());
                let bound = OptionalNodeType::from_repr(
                    thread_rng().sample(Uniform::new(0, 3).unwrap()) + 1,
                )
                .unwrap();
                let entry: TTEntry<Chessboard> = TTEntry {
                    hash: pos.zobrist_hash(),
                    score,
                    mov: UntrustedMove::from_move(mov),
                    depth,
                    bound,
                };
                let packed = entry.to_packed();
                let val = TTEntry::from_packed(packed);
                assert_eq!(val, entry);
                let ply = thread_rng().sample(Uniform::new(0, 100).unwrap());
                tt.store(entry, ply);
                let loaded = tt.load(entry.hash, ply).unwrap();
                assert_eq!(entry, loaded);
            }
        }
    }

    #[test]
    fn test_size() {
        let sizes = [
            1, 2, 3, 4, 8, 15, 16, 17, 79, 80, 81, 100, 12345, 0x1ff_ffff, 0x200_0000,
        ];
        for num_bytes in sizes {
            let tt = TT::new_with_bytes(num_bytes);
            let size = tt.size_in_entries();
            assert_eq!(size, 1.max(num_bytes / size_of::<AtomicTTEntry>()));
            let mut occurrences = vec![0_u64; size];
            let mut gen = thread_rng();
            let num_samples = 200_000;
            for _ in 0..num_samples {
                let idx = tt.index_of(ZobristHash(gen.next_u64()));
                occurrences[idx] += 1;
            }
            let expected = num_samples as f64 / size as f64;
            let min = occurrences.iter().min().copied().unwrap_or_default();
            let max = occurrences.iter().max().copied().unwrap_or_default();
            let std_dev = (occurrences.iter().map(|x| x * x).sum::<u64>() as f64 / size as f64
                - expected * expected)
                .sqrt();
            assert!(
                std_dev <= num_samples as f64 / 128.0,
                "{std_dev} {expected} {size} {num_bytes}"
            );
            assert!(
                expected - min as f64 <= num_samples as f64 / 128.0,
                "{expected} {min} {size} {num_bytes}"
            );
            assert!(
                max as f64 - expected <= num_samples as f64 / 128.0,
                "{expected} {max} {size} {num_bytes}"
            );
        }
    }

    #[test]
    #[cfg(feature = "chess")]
    fn shared_tt_test() {
        let mut tt = TT::new_with_bytes(32_000_000);
        let pos = Chessboard::default();
        let mut engine = Caps::default();
        let bad_move = ChessMove::from_compact_text("a2a3", &pos).unwrap();
        let entry: TTEntry<Chessboard> =
            TTEntry::new(pos.zobrist_hash(), MAX_NORMAL_SCORE, bad_move, 123, Exact);
        tt.store(entry, 0);
        let next_pos = pos.make_move(bad_move).unwrap();
        let next_entry: TTEntry<Chessboard> = TTEntry::new(
            next_pos.zobrist_hash(),
            MIN_NORMAL_SCORE,
            ChessMove::NULL,
            122,
            Exact,
        );
        tt.store(next_entry, 1);
        let mov = engine
            .search_with_tt(pos, SearchLimit::depth(Depth::new_unchecked(1)), tt.clone())
            .chosen_move;
        assert_eq!(mov, bad_move);
        let limit = SearchLimit::depth(Depth::new_unchecked(3));
        let mut engine2 = Caps::default();
        _ = engine2.search_with_new_tt(pos, limit);
        let nodes = engine2.search_state().uci_nodes();
        engine2.forget();
        let _ = engine.search_with_tt(pos, SearchLimit::depth(Depth::new_unchecked(5)), tt.clone());
        let entry = tt.load::<Chessboard>(pos.zobrist_hash(), 0);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().depth, 5);
        _ = engine2.search_with_tt(pos, limit, tt.clone());
        assert!(engine2.search_state().uci_nodes() <= nodes);
        tt.forget();
        let atomic = Arc::new(AtomicSearchState::default());
        let params = SearchParams::with_atomic_state(
            pos,
            SearchLimit::infinite(),
            ZobristHistory::default(),
            tt.clone(),
            atomic.clone(),
        )
        .set_tt(tt.clone());
        assert_eq!(params.tt.0.as_ptr(), tt.0.as_ptr());
        let atomic2 = Arc::new(AtomicSearchState::default());
        let mut params2 = params.auxiliary(atomic2.clone());
        let pos2 = Chessboard::from_name("kiwipete").unwrap();
        params2.pos = pos2;
        let handle = spawn(move || engine.search(params));
        let handle2 = spawn(
            move || engine2.search(params2), /*SearchResult::<Chessboard>::move_only(ChessMove::NULL)*/
        );
        sleep(Duration::from_millis(1000));
        atomic.set_stop(true);
        atomic2.set_stop(true);
        let res1 = handle.join().unwrap();
        let res2 = handle2.join().unwrap();
        assert_ne!(
            res1.chosen_move, res2.chosen_move,
            "{} {}",
            res1.chosen_move, res2.chosen_move
        );
        let hashfull = tt.estimate_hashfull::<Chessboard>();
        assert!(hashfull > 0, "{hashfull}");
        let entry = tt.load::<Chessboard>(pos.zobrist_hash(), 0).unwrap();
        let entry2 = tt.load::<Chessboard>(pos2.zobrist_hash(), 0).unwrap();
        assert_eq!(entry.hash, pos.zobrist_hash());
        assert_eq!(entry2.hash, pos2.zobrist_hash());
        assert!(pos.is_move_legal(mov));
        assert!(pos2.is_move_legal(mov));
    }
}
