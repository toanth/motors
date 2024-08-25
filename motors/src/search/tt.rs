use std::arch::x86_64::{_mm_prefetch, _MM_HINT_T1};
use std::cmp::Ordering;
use std::mem::{size_of, transmute_copy};
use std::ptr::addr_of;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

use portable_atomic::AtomicU128;
use static_assertions::const_assert_eq;
use strum_macros::FromRepr;

use crate::search::tt::TTEntryLookup::{Found, OtherPos};
use crate::search::NodeType;
#[cfg(feature = "chess")]
use gears::games::chess::Chessboard;
use gears::games::ZobristHash;
use gears::general::board::Board;
use gears::general::moves::{Move, UntrustedMove};
use gears::score::{CompactScoreT, Score, ScoreT, SCORE_WON};
use OptionalNodeType::*;

type AtomicTTEntry = AtomicU128;

pub type AgeT = u16;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct Age(pub AgeT);

impl Age {
    pub fn increment(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }

    pub fn age_diff(self, other: Age) -> isize {
        assert_eq!(size_of::<AgeT>(), 2);
        (self.0.wrapping_sub(other.0) as i16) as isize
    }
}

impl PartialOrd for Age {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Age {
    fn cmp(&self, other: &Self) -> Ordering {
        assert_eq!(size_of::<AgeT>(), 2);
        let diff = self.0.wrapping_sub(other.0) as i16;
        diff.cmp(&0)
    }
}

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
    pub score: CompactScoreT,  // 2 bytes
    pub mov: UntrustedMove<B>, // depends, 2 bytes for chess (atm never more)
    pub age: Age, // 2 bytes, should eventually use only 1 byte and decrease TTEntry size
    pub depth: u8, // 1 byte
    bound: OptionalNodeType, // 1 byte, private because it should only be accessed through the `bound()` method
}

impl<B: Board> TTEntry<B> {
    pub fn new(
        hash: ZobristHash,
        score: Score,
        mov: B::Move,
        depth: isize,
        bound: NodeType,
        age: Age,
    ) -> TTEntry<B> {
        let score = CompactScoreT::try_from(score.0).unwrap();
        let depth = depth.clamp(0, u8::MAX as isize) as u8;
        Self {
            hash,
            score,
            mov: UntrustedMove::new(mov),
            age,
            depth,
            bound: OptionalNodeType::from_repr(bound as u8).unwrap(),
        }
    }

    pub fn bound(&self) -> NodeType {
        debug_assert!(self.bound != Empty);
        NodeType::from_repr(self.bound as u8).unwrap()
    }

    pub fn score(&self) -> Score {
        Score::from_compact(self.score)
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
        debug_assert_eq!(size_of_val(&self.bound), 1);
        debug_assert_eq!(size_of_val(&self.depth), 1);
        debug_assert_eq!(size_of_val(&self.age), 2); // TODO: Update eventually
        debug_assert_eq!(size_of_val(&self.score), 2);
        debug_assert_eq!(size_of_val(&self.hash), 8);

        let score = self.score as u16; // don't sign extend negative scores
        ((self.hash.0 as u128) << 64)
            | ((score as u128) << (64 - 8 * size_of::<CompactScoreT>()))
            | ((self.mov.to_underlying().into() as u128) << 32)
            | ((self.age.0 as u128) << 16)
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
        let score_size = 8 * size_of::<CompactScoreT>();
        let score = ((val >> (64 - score_size)) & 0xffff_ffff) as CompactScoreT;
        let mov = B::Move::untrusted_from_repr(((val >> 32) & 0xffff_ffff) as usize);
        let age = Age(((val >> 16) & 0xffff) as AgeT);
        let depth = ((val >> 8) & 0xff) as u8;
        let bound = OptionalNodeType::from_repr((val & 0xff) as u8).unwrap();
        Self {
            hash,
            score,
            mov,
            age,
            depth,
            bound,
        }
    }
}
#[cfg(feature = "chess")]
const_assert_eq!(size_of::<TTEntry<Chessboard>>(), 16);
#[cfg(feature = "chess")]
const_assert_eq!(size_of::<TTEntry<Chessboard>>(), size_of::<AtomicTTEntry>());

#[derive(Debug, Copy, Clone)]
pub enum TTEntryLookup<B: Board> {
    Found(TTEntry<B>),
    OtherPos(TTEntry<B>),
    Empty,
}

impl<B: Board> TTEntryLookup<B> {
    pub fn found(self) -> bool {
        matches!(self, Found(_))
    }

    pub fn unwrap_found(self) -> TTEntry<B> {
        if let Found(entry) = self {
            entry
        } else {
            panic!(
                "Expected a TT entry with  matching hash, but got {}",
                if let TTEntryLookup::Empty = self {
                    "an empty TT entry"
                } else {
                    "a different TT entry"
                }
            )
        }
    }
}

pub(super) const DEFAULT_HASH_SIZE_MB: usize = 4;

/// Note that resizing the TT during search will wait until the search is finished
/// (all threads will receive a new arc)
#[derive(Clone, Debug)]
pub struct TT(pub Arc<[AtomicTTEntry]>);

impl Default for TT {
    fn default() -> Self {
        TT::new_with_bytes(DEFAULT_HASH_SIZE_MB * 1_000_000)
    }
}

impl TT {
    pub fn new_with_bytes(size_in_bytes: usize) -> Self {
        let new_size = 1.max(size_in_bytes / size_of::<AtomicTTEntry>());
        let mut arr = Vec::with_capacity(new_size);
        arr.resize_with(new_size, AtomicU128::default);
        let tt = arr.into_boxed_slice().into();
        Self(tt)
    }

    pub fn size(&self) -> usize {
        self.0.len()
    }

    pub fn forget(&mut self) {
        for entry in self.0.iter() {
            entry.store(0, Relaxed);
        }
    }

    /// Counts the number of non-empty entries in the first 1k entries
    pub fn estimate_hashfull<B: Board>(&self) -> usize {
        let len = 1000.min(self.size());
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
        ((hash.0 as u128 * self.size() as u128) >> usize::BITS as usize) as usize
    }

    pub fn store<B: Board>(&mut self, mut entry: TTEntry<B>, ply: usize) {
        debug_assert!(
            entry.score().0.abs() + ply as ScoreT <= SCORE_WON.0,
            "score {score} ply {ply}",
            score = entry.score().0
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
            entry.score().0,
            won = entry.score().plies_until_game_won().unwrap_or(-1),
        );
        self.0[idx].store(entry.to_packed(), Relaxed);
    }

    pub fn load<B: Board>(&self, hash: ZobristHash, ply: usize) -> TTEntryLookup<B> {
        let idx = self.index_of(hash);
        let mut entry = TTEntry::from_packed(self.0[idx].load(Relaxed));
        // Mate score adjustments, see `store`
        if let Some(plies) = entry.score().plies_until_game_won() {
            if plies < 0 {
                entry.score += ply as CompactScoreT;
            } else {
                entry.score -= ply as CompactScoreT;
            }
        }
        debug_assert!(entry.score().0.abs() <= SCORE_WON.0);
        if entry.bound == Empty {
            TTEntryLookup::Empty
        } else if entry.hash != hash {
            OtherPos(entry)
        } else {
            Found(entry)
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
    use crate::search::multithreading::SearchSender;
    use crate::search::NodeType::Exact;
    use crate::search::SearchState;
    use crate::search::{Benchable, Engine};
    use gears::games::chess::moves::ChessMove;
    use gears::games::ZobristHistory;
    use gears::score::{MAX_NORMAL_SCORE, MIN_NORMAL_SCORE};
    use gears::search::{Depth, SearchLimit};
    use rand::distributions::Uniform;
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
                Age(42),
            );
            let converted = entry.to_packed();
            assert_eq!(TTEntry::from_packed(converted), entry);
            i += 1;
        }
    }

    #[test]
    #[cfg(feature = "chess")]
    fn test_load_store() {
        let mut age = Age(0);
        for pos in Chessboard::bench_positions() {
            age.increment();
            let num_bytes_in_size = thread_rng().sample(Uniform::new(4, 25));
            let size_in_bytes = (1 << num_bytes_in_size)
                + thread_rng().sample(Uniform::new(0, 1 << num_bytes_in_size));
            let mut tt = TT::new_with_bytes(size_in_bytes);
            for mov in pos.pseudolegal_moves() {
                let score = Score(
                    thread_rng().sample(Uniform::new(MIN_NORMAL_SCORE.0, MAX_NORMAL_SCORE.0)),
                )
                .compact();
                let depth = thread_rng().sample(Uniform::new(1, 100));
                let bound =
                    OptionalNodeType::from_repr(thread_rng().sample(Uniform::new(0, 3)) + 1)
                        .unwrap();
                let entry: TTEntry<Chessboard> = TTEntry {
                    hash: pos.zobrist_hash(),
                    score,
                    mov: UntrustedMove::new(mov),
                    age,
                    depth,
                    bound,
                };
                let packed = entry.to_packed();
                let val = TTEntry::from_packed(packed);
                assert_eq!(val, entry);
                let ply = thread_rng().sample(Uniform::new(0, 100));
                tt.store(entry, ply);
                let Found(loaded) = tt.load(entry.hash, ply) else {
                    panic!()
                };
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
            let size = tt.size();
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
        engine.set_tt(tt.clone());
        let bad_move = ChessMove::from_compact_text("a2a3", &pos).unwrap();
        let mut entry: TTEntry<Chessboard> = TTEntry::new(
            pos.zobrist_hash(),
            MAX_NORMAL_SCORE,
            bad_move,
            123,
            Exact,
            Age(1),
        );
        tt.store(entry, 0);
        let next_pos = pos.make_move(bad_move).unwrap();
        let next_entry: TTEntry<Chessboard> = TTEntry::new(
            next_pos.zobrist_hash(),
            MIN_NORMAL_SCORE,
            ChessMove::NULL,
            122,
            Exact,
            Age(42),
        );
        tt.store(next_entry, 1);
        let mov = engine
            .search_from_pos(pos, SearchLimit::depth(Depth::new(1)))
            .unwrap()
            .chosen_move;
        assert_eq!(mov, bad_move);
        let loaded = tt.load::<Chessboard>(pos.zobrist_hash(), 0).unwrap_found();
        assert_eq!(loaded.hash, pos.zobrist_hash());
        // assert_eq!(loaded.age.0, 42);
        // assert_eq!(loaded.depth, 123);
        let limit = SearchLimit::depth(Depth::new(4));
        let mut engine2 = Caps::default();
        engine2.set_tt(TT::new_with_bytes(32_000_000));
        engine2.search_from_pos(pos, limit).unwrap(); // search with a default TT
        let old_nodes = engine2.search_state().uci_nodes();
        // assert_eq!(
        //     tt.load::<Chessboard>(pos.zobrist_hash(), 0)
        //         .unwrap_found()
        //         .depth,
        //     123
        // );
        let _ = engine
            .search_from_pos(pos, SearchLimit::depth(Depth::new(5)))
            .unwrap();
        let loaded = tt.load::<Chessboard>(pos.zobrist_hash(), 0);
        assert!(loaded.found());
        // assert_eq!(loaded.unwrap_found().depth, 123);
        entry.depth = 1;
        tt.store(entry, 0);
        let _ = engine
            .search_from_pos(pos, SearchLimit::depth(Depth::new(5)))
            .unwrap();
        let loaded = tt.load::<Chessboard>(pos.zobrist_hash(), 0);
        assert!(loaded.found());
        assert_eq!(loaded.unwrap_found().depth, 5);
        assert_ne!(
            loaded.unwrap_found().mov.to_underlying(),
            bad_move.to_underlying()
        );
        engine2.forget();
        engine2.set_tt(tt.clone());
        engine2.search_from_pos(pos, limit).unwrap();
        engine2.search_from_pos(pos, limit).unwrap(); // search again with a prefilled TT
        let new_nodes = engine2.search_state().uci_nodes();
        assert!(new_nodes <= old_nodes);
        let mut sender = SearchSender::no_sender();
        let sender1 = sender.clone();
        engine.forget();
        engine2.forget();
        let handle = spawn(move || {
            engine.search(
                pos,    
                SearchLimit::infinite(),
                ZobristHistory::default(),
                sender1,
            )
        });
        let pos2 = Chessboard::from_name("kiwipete").unwrap();
        let sender2 = sender.clone();
        let handle2 = spawn(move || {
            engine2.search(
                pos2,
                SearchLimit::infinite(),
                ZobristHistory::default(),
                sender2,
            )
        });
        sleep(Duration::from_millis(500));
        sender.send_stop();
        let res1 = handle.join().unwrap().unwrap();
        let res2 = handle2.join().unwrap().unwrap();
        assert_ne!(res1.chosen_move, res2.chosen_move);
        let entry = tt.load::<Chessboard>(pos.zobrist_hash(), 0).unwrap_found();
        let entry2 = tt.load::<Chessboard>(pos2.zobrist_hash(), 0).unwrap_found();
        assert_eq!(entry.hash, pos.zobrist_hash());
        assert_eq!(entry2.hash, pos2.zobrist_hash());
        assert!(pos.is_move_legal(mov));
        assert!(pos2.is_move_legal(mov));
    }
}
