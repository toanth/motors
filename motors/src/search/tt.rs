use std::arch::x86_64::{_mm_prefetch, _MM_HINT_T1};
use std::fmt::{Display, Formatter};
use std::mem::size_of;
#[cfg(feature = "unsafe")]
use std::mem::transmute_copy;
use std::ptr::addr_of;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

use portable_atomic::AtomicU128;
use rayon::prelude::IntoParallelRefMutIterator;
use rayon::prelude::ParallelIterator;
use static_assertions::const_assert_eq;
use strum_macros::FromRepr;

#[cfg(feature = "chess")]
use gears::games::chess::Chessboard;
use gears::games::PosHash;
use gears::general::board::Board;
use gears::general::moves::{Move, UntrustedMove};
use gears::score::{CompactScoreT, Score, ScoreT, SCORE_WON};
use gears::search::NodeType;
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
    pub hash: PosHash,         // 8 bytes
    pub score: CompactScoreT,  // 2 bytes
    pub eval: CompactScoreT,   // 2 bytes
    pub mov: UntrustedMove<B>, // depends, 2 bytes for chess (atm never more)
    pub depth: u8,             // 1 byte
    bound: OptionalNodeType,   // 1 byte
}

impl<B: Board> Display for TTEntry<B>
where
    B::Move: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "move {0} score {1} bound {2} depth {3}", self.mov, self.score, self.bound(), self.depth)
    }
}

impl<B: Board> TTEntry<B> {
    pub fn new(hash: PosHash, score: Score,eval: Score, mov: B::Move, depth: isize, bound: NodeType) -> TTEntry<B> {
        let depth = depth.clamp(0, u8::MAX as isize) as u8;
        Self {
            score: score.compact(),
            eval: eval.compact(),
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

    pub fn score(&self) -> Score {
        Score::from_compact(self.score)
    }

    pub fn raw_eval(&self) -> Score {
        Score::from_compact(self.eval)
    }

    #[cfg(feature = "unsafe")]
    fn pack(self) -> u128 {
        if size_of::<Self>() == 128 / 8 {
            // `transmute_copy` is needed because otherwise the compiler complains that the sizes might not match.
            unsafe { transmute_copy::<Self, u128>(&self) }
        } else {
            self.pack_fallback()
        }
    }

    fn pack_fallback(self) -> u128 {
        let score = self.score as u16; // don't sign extend negative scores
        let eval = self.eval as u16;
        ((self.hash.0 as u128) << 64)
            | ((score as u128) << (64 - 16))
            | ((eval as u128) << (64 - 32))
            | ((self.mov.to_underlying().into() as u128) << 16)
            | ((self.depth as u128) << 8)
            | self.bound as u128
    }

    #[cfg(not(feature = "unsafe"))]
    fn pack(self) -> u128 {
        self.pack_fallback()
    }

    #[cfg(feature = "unsafe")]
    fn unpack(packed: u128) -> Self {
        if size_of::<Self>() == 128 / 8 {
            unsafe { transmute_copy::<u128, Self>(&packed) }
        } else {
            Self::unpack_fallback(packed)
        }
    }

    #[cfg(not(feature = "unsafe"))]
    fn unpack(val: u128) -> Self {
        Self::unpack_fallback(val)
    }

    fn unpack_fallback(val: u128) -> Self {
        let hash = PosHash((val >> 64) as u64);
        let score = ((val >> (64 - 16)) & 0xffff) as CompactScoreT;
        let eval = ((val >> (64 - 32)) & 0xffff) as CompactScoreT;
        let mov = B::Move::from_u64_unchecked(((val >> 16) & 0xffff) as u64);
        let depth = ((val >> 8) & 0xff) as u8;
        let bound = OptionalNodeType::from_repr((val & 0xff) as u8).unwrap();
        Self { hash, score,eval, mov, depth, bound }
    }
}
#[cfg(feature = "chess")]
const_assert_eq!(size_of::<TTEntry<Chessboard>>(), 16);
#[cfg(feature = "chess")]
const_assert_eq!(size_of::<TTEntry<Chessboard>>(), size_of::<AtomicTTEntry>());

pub const DEFAULT_HASH_SIZE_MB: usize = 16;

/// Resizing the TT during search will wait until the search is finished (all threads will receive a new arc)
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

    /// Technically, the UCI document specifies MB instead of MiB, but almost every engine uses MiB and
    /// the upcoming expositor UCI spec will use MiB as well
    pub fn new_with_mib(size_in_mib: usize) -> Self {
        Self::new_with_bytes(size_in_mib * (1 << 20))
    }

    fn new_with_bytes(size_in_bytes: usize) -> Self {
        let new_size = 1.max(size_in_bytes / size_of::<AtomicTTEntry>());
        let tt = if cfg!(feature = "unsafe") && size_in_bytes > 1024 * 1024 * 16 {
            let mut arr = Box::new_uninit_slice(new_size);
            arr.par_iter_mut().for_each(|elem| {
                _ = elem.write(AtomicU128::default());
            });
            unsafe { arr.assume_init() }
        } else {
            let mut arr = Vec::with_capacity(new_size);
            arr.resize_with(new_size, AtomicU128::default);
            arr.into_boxed_slice()
        };
        Self(tt.into())
    }

    pub fn size_in_entries(&self) -> usize {
        self.0.len()
    }

    pub fn size_in_bytes(&self) -> usize {
        self.size_in_entries() * size_of::<AtomicTTEntry>()
    }

    pub fn size_in_mib(&self) -> usize {
        (self.size_in_bytes() + 500_000) / (1 << 20)
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
            .filter(|e: &&AtomicTTEntry| TTEntry::<B>::unpack(e.load(Relaxed)).bound != Empty)
            .count();
        if len < 1000 {
            (num_used as f64 * 1000.0 / len as f64).round() as usize
        } else {
            num_used
        }
    }

    fn index_of(&self, hash: PosHash) -> usize {
        // Uses the multiplication trick from here: <https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/>
        ((hash.0 as u128 * self.size_in_entries() as u128) >> 64) as usize
    }

    pub(super) fn store<B: Board>(&mut self, mut entry: TTEntry<B>, ply: usize) {
        debug_assert!(
            entry.score().abs() + ply as ScoreT <= SCORE_WON,
            "score {score} ply {ply}",
            score = entry.score
        );
        let idx = self.index_of(entry.hash);
        // Mate score adjustments: For the current search, we want to penalize later mates to prefer earlier ones,
        // where "later" means being found at greater depth (usually called the `ply` parameter in the search function).
        // But since the TT persists across searches and can also reach the same position at different plies though transpositions,
        // we undo that when storing mate scores, and reapply the penalty for the *current* ply when loading mate scores.
        if let Some(plies) = entry.score().plies_until_game_won() {
            if plies < 0 {
                entry.score -= ply as CompactScoreT;
            } else {
                entry.score += ply as CompactScoreT;
            }
        }
        debug_assert!(
            entry.score().0.abs() <= SCORE_WON.0,
            "score {}, ply {ply}, won in {won}",
            entry.score,
            won = entry.score().plies_until_game_won().unwrap_or(-1),
        );
        self.0[idx].store(entry.pack(), Relaxed);
    }

    pub(super) fn load<B: Board>(&self, hash: PosHash, ply: usize) -> Option<TTEntry<B>> {
        let idx = self.index_of(hash);
        let mut entry = TTEntry::unpack(self.0[idx].load(Relaxed));
        // Mate score adjustments, see `store`
        if let Some(plies) = entry.score().plies_until_game_won() {
            if plies < 0 {
                entry.score += ply as CompactScoreT;
            } else {
                entry.score -= ply as CompactScoreT;
            }
        }
        debug_assert!(entry.score().0.abs() <= SCORE_WON.0);
        if entry.hash != hash || entry.bound == Empty {
            None
        } else {
            Some(entry)
        }
    }

    #[inline(always)]
    pub fn prefetch(&self, hash: PosHash) {
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
    use super::*;
    use crate::search::chess::caps::Caps;
    use crate::search::multithreading::AtomicSearchState;
    use crate::search::{Engine, SearchParams};
    use gears::games::chess::moves::ChessMove;
    use gears::games::ZobristHistory;
    use gears::general::board::BoardHelpers;
    use gears::rand::distr::Uniform;
    use gears::rand::{rng, Rng, RngCore};
    use gears::score::{MAX_NORMAL_SCORE, MIN_NORMAL_SCORE};
    use gears::search::NodeType::Exact;
    use gears::search::{Depth, SearchLimit};
    use std::thread::{sleep, spawn};
    use std::time::Duration;

    #[test]
    #[cfg(feature = "chess")]
    fn test_packing() {
        let board = Chessboard::from_name("kiwipete").unwrap();
        let mut i = 1;
        for mov in board.pseudolegal_moves() {
            let entry: TTEntry<Chessboard> = TTEntry::new(
                board.hash_pos(),
                Score(i * i * (i % 2 * 2 - 1)),
                Score((i % -3) * (i % 5 + i)),
                mov,
                i as isize,
                NodeType::from_repr(i as u8 % 3 + 1).unwrap(),
            );
            let converted = entry.pack();
            assert_eq!(TTEntry::unpack(converted), entry);
            i += 1;
        }
    }

    #[test]
    #[cfg(feature = "chess")]
    fn test_load_store() {
        for pos in Chessboard::bench_positions() {
            let num_bytes_in_size = rng().sample(Uniform::new(4, 25).unwrap());
            let size_in_bytes =
                (1 << num_bytes_in_size) + rng().sample(Uniform::new(0, 1 << num_bytes_in_size).unwrap());
            let mut tt = TT::new_with_bytes(size_in_bytes);
            for mov in pos.pseudolegal_moves() {
                let score = Score(rng().sample(Uniform::new(MIN_NORMAL_SCORE.0, MAX_NORMAL_SCORE.0).unwrap()));
                let depth = rng().sample(Uniform::new(1, 100).unwrap());
                let bound = OptionalNodeType::from_repr(rng().sample(Uniform::new(0, 3).unwrap()) + 1).unwrap();
                let entry: TTEntry<Chessboard> =
                    TTEntry { hash: pos.hash_pos(), score: score.compact(),
                    eval: score.compact() - 1, mov: UntrustedMove::from_move(mov), depth, bound };
                let packed = entry.pack();
                let val = TTEntry::unpack(packed);
                assert_eq!(val, entry);
                let ply = rng().sample(Uniform::new(0, 100).unwrap());
                tt.store(entry, ply);
                let loaded = tt.load(entry.hash, ply).unwrap();
                assert_eq!(entry, loaded);
            }
        }
    }

    #[test]
    fn test_size() {
        let sizes = [1, 2, 3, 4, 8, 15, 16, 17, 79, 80, 81, 100, 12345, 0x1ff_ffff, 0x200_0000];
        for num_bytes in sizes {
            let tt = TT::new_with_bytes(num_bytes);
            let size = tt.size_in_entries();
            assert_eq!(size, 1.max(num_bytes / size_of::<AtomicTTEntry>()));
            let mut occurrences = vec![0_u64; size];
            let mut gen = rng();
            let num_samples = 200_000;
            for _ in 0..num_samples {
                let idx = tt.index_of(PosHash(gen.next_u64()));
                occurrences[idx] += 1;
            }
            let expected = num_samples as f64 / size as f64;
            let min = occurrences.iter().min().copied().unwrap_or_default();
            let max = occurrences.iter().max().copied().unwrap_or_default();
            let std_dev =
                (occurrences.iter().map(|x| x * x).sum::<u64>() as f64 / size as f64 - expected * expected).sqrt();
            assert!(std_dev <= num_samples as f64 / 128.0, "{std_dev} {expected} {size} {num_bytes}");
            assert!(expected - min as f64 <= num_samples as f64 / 128.0, "{expected} {min} {size} {num_bytes}");
            assert!(max as f64 - expected <= num_samples as f64 / 128.0, "{expected} {max} {size} {num_bytes}");
        }
    }

    #[test]
    #[cfg(feature = "chess")]
    fn shared_tt_test() {
        let mut tt = TT::new_with_bytes(32_000_000);
        let pos = Chessboard::default();
        let mut engine = Caps::default();
        let bad_move = ChessMove::from_compact_text("a2a3", &pos).unwrap();
        let entry: TTEntry<Chessboard> = TTEntry::new(
            pos.hash_pos(),
            MAX_NORMAL_SCORE,
            MIN_NORMAL_SCORE,
            bad_move,
            123,
            Exact,
        );
        tt.store(entry, 0);
        let next_pos = pos.make_move(bad_move).unwrap();
        let next_entry: TTEntry<Chessboard> =
            TTEntry::new(next_pos.hash_pos(), MIN_NORMAL_SCORE,MAX_NORMAL_SCORE, ChessMove::NULL, 122, Exact);
        tt.store(next_entry, 1);
        let mov = engine.search_with_tt(pos, SearchLimit::depth(Depth::new(1)), tt.clone()).chosen_move;
        assert_eq!(mov, bad_move);
        let limit = SearchLimit::depth(Depth::new(3));
        let mut engine2 = Caps::default();
        _ = engine2.search_with_new_tt(pos, limit);
        let nodes = engine2.search_state().uci_nodes();
        engine2.forget();
        let _ = engine.search_with_tt(pos, SearchLimit::depth(Depth::new(5)), tt.clone());
        let entry = tt.load::<Chessboard>(pos.hash_pos(), 0);
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
        let handle2 =
            spawn(move || engine2.search(params2) /*SearchResult::<Chessboard>::move_only(ChessMove::NULL)*/);
        sleep(Duration::from_millis(1000));
        atomic.set_stop(true);
        atomic2.set_stop(true);
        let res1 = handle.join().unwrap();
        let res2 = handle2.join().unwrap();
        assert_ne!(
            res1.chosen_move,
            res2.chosen_move,
            "{} {}",
            res1.chosen_move.compact_formatter(&pos),
            res2.chosen_move.compact_formatter(&pos)
        );
        let hashfull = tt.estimate_hashfull::<Chessboard>();
        assert!(hashfull > 0, "{hashfull}");
        let entry = tt.load::<Chessboard>(pos.hash_pos(), 0).unwrap();
        let entry2 = tt.load::<Chessboard>(pos2.hash_pos(), 0).unwrap();
        assert_eq!(entry.hash, pos.hash_pos());
        assert_eq!(entry2.hash, pos2.hash_pos());
        assert!(pos.is_move_legal(mov));
        assert!(pos2.is_move_legal(mov));
    }
}
