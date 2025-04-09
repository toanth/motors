use derive_more::Index;
use gears::games::PosHash;
#[cfg(feature = "chess")]
use gears::games::chess::Chessboard;
use gears::general::board::Board;
use gears::general::moves::{Move, UntrustedMove};
use gears::itertools::Itertools;
use gears::score::{CompactScoreT, SCORE_WON, Score, ScoreT};
use gears::search::NodeType;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelRefMutIterator;
#[cfg(all(feature = "unsafe", target_arch = "x86_64", target_feature = "sse"))]
use std::arch::x86_64::{_MM_HINT_T1, _mm_prefetch};
use std::fmt::{Display, Formatter};
use std::mem::size_of;
#[cfg(feature = "unsafe")]
use std::mem::transmute_copy;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::Relaxed;

#[derive(Debug, Default)]
#[repr(C)]
struct AtomicTTEntry {
    hash: AtomicU64,
    rest: AtomicU64,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, derive_more::Display)]
pub struct Age(u8);

impl Age {
    /// Incrementing the age can wrap around sooner than after 256 calls because the TT entry doesn't store the full 8 bits
    pub fn increment(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

fn pack_age_and_bound(age: Age, bound: NodeType) -> u8 {
    (age.0 << 2) | (bound as u8)
}

fn unpack_age_and_bound(val: u8) -> (Age, Option<NodeType>) {
    let age = Age(val >> 2);
    let bound = NodeType::from_repr(val & 0b11);
    (age, bound)
}

const NUM_ENTRIES_IN_BUCKET: usize = 4;

#[derive(Debug, Default, Index)]
#[repr(align(64))]
struct TTBucket([AtomicTTEntry; NUM_ENTRIES_IN_BUCKET]);

const _: () = assert!(size_of::<TTBucket>() == 64);

#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
#[repr(C)]
pub struct TTEntry<B: Board> {
    pub hash: PosHash,         // 8 bytes
    pub score: CompactScoreT,  // 2 bytes
    pub eval: CompactScoreT,   // 2 bytes
    pub mov: UntrustedMove<B>, // depends, 2 bytes for chess (atm never more)
    pub depth: u8,             // 1 byte
    age_and_bound: u8,         // 1 byte
}

impl<B: Board> Display for TTEntry<B>
where
    B::Move: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "move {0} score {1} bound {2} age {3} depth {4}",
            self.mov,
            self.score,
            self.bound(),
            self.age(),
            self.depth
        )
    }
}

impl<B: Board> TTEntry<B> {
    pub fn new(
        hash: PosHash,
        score: Score,
        eval: Score,
        mov: B::Move,
        depth: isize,
        bound: NodeType,
        age: Age,
    ) -> TTEntry<B> {
        let depth = depth.clamp(0, u8::MAX as isize) as u8;
        let age_and_bound = pack_age_and_bound(age, bound);
        Self {
            score: score.compact(),
            eval: eval.compact(),
            mov: UntrustedMove::from_move(mov),
            depth,
            age_and_bound,
            hash,
        }
    }

    pub fn is_empty(&self) -> bool {
        unpack_age_and_bound(self.age_and_bound).1.is_none()
    }

    fn is_atomic_entry_from_current_search(entry: &AtomicTTEntry, age: Age) -> bool {
        let e = Self::unpack(entry);
        let (a, b) = unpack_age_and_bound(e.age_and_bound);
        a == age && b.is_some()
    }

    pub fn bound(&self) -> NodeType {
        unpack_age_and_bound(self.age_and_bound).1.expect("Incorrect NodeType in packed value")
    }

    pub fn age(&self) -> Age {
        unpack_age_and_bound(self.age_and_bound).0
    }

    pub fn score(&self) -> Score {
        Score::from_compact(self.score)
    }

    pub fn raw_eval(&self) -> Score {
        Score::from_compact(self.eval)
    }

    #[cfg(feature = "unsafe")]
    fn pack_into(self, entry: &AtomicTTEntry) {
        debug_assert_eq!(size_of::<Self>(), 128 / 8);
        debug_assert_eq!(size_of::<AtomicTTEntry>(), size_of::<Self>());
        // `transmute_copy` is needed because otherwise the compiler complains that the sizes might not match.
        // SAFETY: Both types have the same size and all bit patterns are valid
        let e = unsafe { transmute_copy::<Self, u128>(&self) };
        entry.hash.store(e as u64, Relaxed);
        entry.rest.store((e >> 64) as u64, Relaxed);
    }

    #[cfg(not(feature = "unsafe"))]
    fn pack_into(self, entry: &AtomicTTEntry) {
        self.pack_fallback(entry)
    }

    #[allow(unused)]
    fn pack_fallback(self, entry: &AtomicTTEntry) {
        let score = self.score as u16; // don't sign extend negative scores
        let eval = self.eval as u16;
        let rest = ((score as u64) << (64 - 16))
            | ((eval as u64) << (64 - 32))
            | ((self.mov.to_underlying().into() as u64) << 16)
            | ((self.depth as u64) << 8)
            | self.age_and_bound as u64;

        entry.hash.store(self.hash.0, Relaxed);
        entry.rest.store(rest, Relaxed);
    }

    #[cfg(feature = "unsafe")]
    fn unpack(packed: &AtomicTTEntry) -> Self {
        debug_assert_eq!(size_of::<Self>(), 128 / 8);
        debug_assert_eq!(size_of::<AtomicTTEntry>(), size_of::<Self>());
        let hash = packed.hash.load(Relaxed) as u128;
        let val = ((packed.rest.load(Relaxed) as u128) << 64) | hash;
        // SAFETY: Both types have the same size and all bit patterns are valid
        unsafe { transmute_copy::<u128, Self>(&val) }
    }

    #[cfg(not(feature = "unsafe"))]
    fn unpack(val: &AtomicTTEntry) -> Self {
        Self::unpack_fallback(val)
    }

    #[allow(unused)]
    fn unpack_fallback(val: &AtomicTTEntry) -> Self {
        let hash = PosHash(val.hash.load(Relaxed));
        let rest = val.rest.load(Relaxed);
        let score = (rest >> (64 - 16)) as CompactScoreT;
        let eval = (rest >> (64 - 32)) as CompactScoreT;
        let mov = B::Move::from_u64_unchecked((rest >> 16) & 0xffff);
        let depth = (rest >> 8) as u8;
        let age_and_bound = rest as u8;
        Self { hash, score, eval, mov, depth, age_and_bound }
    }
}
#[cfg(feature = "chess")]
const _: () = assert!(size_of::<TTEntry<Chessboard>>() == 16);
#[cfg(feature = "chess")]
const _: () = assert!(size_of::<TTEntry<Chessboard>>() == size_of::<AtomicTTEntry>());

pub const DEFAULT_HASH_SIZE_MB: usize = 16;

/// Resizing the TT during search will wait until the search is finished (all threads will receive a new arc)
#[derive(Clone, Debug)]
pub struct TT(Arc<[TTBucket]>);

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
        let new_size = 1.max(size_in_bytes / (size_of::<AtomicTTEntry>() * NUM_ENTRIES_IN_BUCKET));
        let tt = if cfg!(feature = "unsafe") && size_in_bytes > 1024 * 1024 * 16 {
            let mut arr = Box::new_uninit_slice(new_size);
            arr.par_iter_mut().for_each(|elem| {
                _ = elem.write(TTBucket::default());
            });
            // SAFETY: The entire array just got initialized
            unsafe { arr.assume_init() }
        } else {
            let mut arr = Vec::with_capacity(new_size);
            arr.resize_with(new_size, TTBucket::default);
            arr.into_boxed_slice()
        };
        Self(tt.into())
    }

    pub fn size_in_buckets(&self) -> usize {
        self.0.len()
    }

    pub fn size_in_entries(&self) -> usize {
        self.size_in_buckets() * NUM_ENTRIES_IN_BUCKET
    }

    pub fn size_in_bytes(&self) -> usize {
        self.size_in_entries() * size_of::<AtomicTTEntry>()
    }

    pub fn size_in_mib(&self) -> usize {
        (self.size_in_bytes() + (1 << 19)) / (1 << 20)
    }

    pub fn forget(&mut self) {
        // TODO: Instead of overwriting every entry, simply increase the age such that old entries will be ignored
        for bucket in self.0.iter() {
            for entry in &bucket.0 {
                entry.hash.store(0, Relaxed);
                entry.rest.store(0, Relaxed);
            }
        }
    }

    /// Counts the number of non-empty entries in the first 1k entries
    pub fn estimate_hashfull<B: Board>(&self, age: Age) -> usize {
        let num_buckets = (1000 / NUM_ENTRIES_IN_BUCKET).min(self.size_in_buckets());
        let num_entries = num_buckets * NUM_ENTRIES_IN_BUCKET;
        let num_used = self
            .0
            .iter()
            .take(num_buckets)
            .flat_map(|bucket| bucket.0.iter())
            .filter(|e: &&AtomicTTEntry| TTEntry::<B>::is_atomic_entry_from_current_search(e, age))
            .count();
        if num_entries < 1000 { (num_used as f64 * 1000.0 / num_entries as f64).round() as usize } else { num_used }
    }

    fn bucket_index_of(&self, hash: PosHash) -> usize {
        // Uses the multiplication trick from here: <https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/>
        ((hash.0 as u128 * self.size_in_buckets() as u128) >> 64) as usize
    }

    // The lowest score is getting replaced
    fn entry_replacement_score<B: Board>(candidate: &TTEntry<B>, to_insert: &TTEntry<B>) -> usize {
        if to_insert.hash == candidate.hash {
            0
        } else if candidate.is_empty() {
            1
        } else {
            candidate.depth as usize + ((candidate.age() == to_insert.age()) as usize * 1000)
        }
    }

    pub fn store<B: Board>(&self, mut entry: TTEntry<B>, ply: usize) {
        debug_assert!(entry.score().abs() + ply as ScoreT <= SCORE_WON, "score {score} ply {ply}", score = entry.score);
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
        let bucket = self.bucket_index_of(entry.hash);
        let bucket = &self.0[bucket].0;
        let idx_in_bucket = bucket
            .iter()
            .map(|e| TTEntry::unpack(e))
            .position_min_by_key(|e| Self::entry_replacement_score(e, &entry))
            .unwrap();
        debug_assert!(
            entry.score().0.abs() <= SCORE_WON.0,
            "score {}, ply {ply}, won in {won}",
            entry.score,
            won = entry.score().plies_until_game_won().unwrap_or(-1),
        );
        entry.pack_into(&bucket[idx_in_bucket]);
    }

    pub fn load<B: Board>(&self, hash: PosHash, ply: usize) -> Option<TTEntry<B>> {
        let bucket = &self.0[self.bucket_index_of(hash)];
        let mut entry = bucket.0.iter().map(|e| TTEntry::<B>::unpack(e)).find(|e| e.hash == hash && !e.is_empty())?;
        // Mate score adjustments, see `store`
        if let Some(tt_plies) = entry.score().plies_until_game_won() {
            if tt_plies <= 0 {
                entry.score += ply as CompactScoreT;
            } else {
                entry.score -= ply as CompactScoreT;
            }
        }
        debug_assert!(entry.score().0.abs() <= SCORE_WON.0, "{} {ply} {entry:?}", entry.score().0);
        Some(entry)
    }

    #[inline(always)]
    #[allow(unused_variables)]
    pub fn prefetch(&self, hash: PosHash) {
        #[cfg(all(target_arch = "x86_64", target_feature = "sse", feature = "unsafe"))]
        // SAFETY: This function is safe to call and computing the pointer is also safe.
        unsafe {
            #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
            _mm_prefetch::<_MM_HINT_T1>(&raw const self.0[self.bucket_index_of(hash)] as *const i8);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::search::chess::caps::Caps;
    use crate::search::multithreading::AtomicSearchState;
    use crate::search::{Engine, NormalEngine, SearchParams};
    use gears::games::ZobristHistory;
    use gears::games::chess::moves::ChessMove;
    use gears::general::board::BoardHelpers;
    use gears::rand::distr::Uniform;
    use gears::rand::{Rng, RngCore, rng};
    use gears::score::{MAX_NORMAL_SCORE, MIN_NORMAL_SCORE};
    use gears::search::NodeType::{Exact, FailHigh, FailLow};
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
                Age((i * i / 3 - i % 2 * 7) as u8),
            );
            let converted = AtomicTTEntry::default();
            entry.pack_into(&converted);
            assert_eq!(TTEntry::unpack(&converted), entry);
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
            let tt = TT::new_with_bytes(size_in_bytes);
            for mov in pos.pseudolegal_moves() {
                let score = Score(rng().sample(Uniform::new(MIN_NORMAL_SCORE.0, MAX_NORMAL_SCORE.0).unwrap()));
                let depth = rng().sample(Uniform::new(1, 100).unwrap());
                let bound = NodeType::from_repr(rng().sample(Uniform::new(0, 3).unwrap()) + 1).unwrap();
                let age = rng().sample(Uniform::new(1, 100).unwrap());
                let age_and_bound = pack_age_and_bound(Age(age), bound);
                let entry: TTEntry<Chessboard> = TTEntry {
                    hash: pos.hash_pos(),
                    score: score.compact(),
                    eval: score.compact() - 1,
                    mov: UntrustedMove::from_move(mov),
                    depth,
                    age_and_bound,
                };
                let packed = AtomicTTEntry::default();
                entry.pack_into(&packed);
                let val = TTEntry::unpack(&packed);
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
        let sizes = [
            1, 2, 3, 4, 5, 8, 15, 16, 17, 63, 64, 65, 72, 79, 80, 81, 100, 159, 160, 176, 12345, 0x1ff_ffff, 0x200_0000,
        ];
        for num_bytes in sizes {
            let tt = TT::new_with_bytes(num_bytes);
            let num_buckets = tt.size_in_buckets();
            assert_eq!(num_buckets, 1.max(num_bytes / size_of::<TTBucket>()), "{num_bytes}");
            let size = tt.size_in_entries();
            assert_eq!(size, num_buckets * NUM_ENTRIES_IN_BUCKET, "{num_bytes}");
            let mut occurrences = vec![0_u64; num_buckets];
            let mut rng = rng();
            let num_samples = 200_000;
            for _ in 0..num_samples {
                let idx = tt.bucket_index_of(PosHash(rng.next_u64()));
                occurrences[idx] += 1;
            }
            let expected = num_samples as f64 / num_buckets as f64;
            let min = occurrences.iter().min().copied().unwrap_or_default();
            let max = occurrences.iter().max().copied().unwrap_or_default();
            let std_dev = (occurrences.iter().map(|x| x * x).sum::<u64>() as f64 / num_buckets as f64
                - expected * expected)
                .sqrt();
            assert!(std_dev <= num_samples as f64 / 128.0, "{std_dev} {expected} {num_buckets} {size} {num_bytes}");
            assert!(
                expected - min as f64 <= num_samples as f64 / 128.0,
                "{expected} {min} {num_buckets} {size} {num_bytes}"
            );
            assert!(
                max as f64 - expected <= num_samples as f64 / 128.0,
                "{expected} {max} {num_buckets} {size} {num_bytes}"
            );
        }
    }

    #[test]
    #[cfg(feature = "chess")]
    fn bucket_test() {
        assert_eq!(NUM_ENTRIES_IN_BUCKET, 4);
        let tt = TT::new_with_bytes(1024);
        assert_eq!(tt.size_in_buckets(), 16);
        let mov = ChessMove::default();
        let entry = TTEntry::<Chessboard>::new(PosHash(42), Score(0), Score(100), mov, 10, Exact, Age(0));
        tt.store(entry, 0);
        let bucket_idx = tt.bucket_index_of(entry.hash);
        let bucket = &tt.0[bucket_idx].0;
        assert_ne!(tt.bucket_index_of(PosHash(!0)), bucket_idx);
        let entry2 = TTEntry::<Chessboard>::new(PosHash(100), Score(10), Score(-20), mov, 5, FailHigh, Age(1));
        assert_eq!(bucket_idx, tt.bucket_index_of(entry2.hash));
        tt.store(entry2, 1);
        let entry3 = TTEntry::<Chessboard>::new(PosHash(0), Score(-1210), Score(-512), mov, 7, FailLow, Age(0));
        assert_eq!(bucket_idx, tt.bucket_index_of(entry3.hash));
        tt.store(entry3, 0);
        let entry4 = TTEntry::<Chessboard>::new(PosHash(0x100000), Score(1234), Score(9876), mov, 12, FailHigh, Age(0));
        assert_eq!(bucket_idx, tt.bucket_index_of(entry4.hash));
        tt.store(entry4, 0);
        let num_empty = bucket.iter().map(|e| TTEntry::<Chessboard>::unpack(e)).filter(|e| e.is_empty()).count();
        assert_eq!(num_empty, 0);

        let new_entry = TTEntry::<Chessboard>::new(PosHash(0x4200000), Score(100), Score(0), mov, 0, FailLow, Age(0));
        assert_eq!(bucket_idx, tt.bucket_index_of(new_entry.hash));
        tt.store(new_entry, 0);
        let has = |entry: TTEntry<Chessboard>| bucket.iter().map(|e| TTEntry::<Chessboard>::unpack(e)).contains(&entry);
        let has_entry2 = has(entry2);
        assert!(!has_entry2);
        assert!(has(entry));
        let entry_again = entry;
        tt.store(entry_again, 0);
        assert!(has(new_entry));
        tt.store(entry2, 0);
        assert!(!has(new_entry));
    }

    #[test]
    #[cfg(feature = "chess")]
    fn shared_tt_test() {
        let mut tt = TT::new_with_bytes(32_000_000);
        let pos = Chessboard::default();
        let mut engine = Caps::default();
        let bad_move = ChessMove::from_compact_text("a2a3", &pos).unwrap();
        let age = Age(42);
        let entry: TTEntry<Chessboard> =
            TTEntry::new(pos.hash_pos(), MAX_NORMAL_SCORE, MIN_NORMAL_SCORE, bad_move, 123, Exact, age);
        tt.store(entry, 0);
        let next_pos = pos.make_move(bad_move).unwrap();
        let next_entry: TTEntry<Chessboard> =
            TTEntry::new(next_pos.hash_pos(), MIN_NORMAL_SCORE, MAX_NORMAL_SCORE, ChessMove::NULL, 122, Exact, age);
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
        let mut age = engine.age;
        age.increment();
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
        let hashfull = tt.estimate_hashfull::<Chessboard>(age);
        assert!(hashfull > 0, "{hashfull}");
        let hashfull = tt.estimate_hashfull::<Chessboard>(Age(0));
        assert!(hashfull == 0, "{hashfull}");
        let entry = tt.load::<Chessboard>(pos.hash_pos(), 0).unwrap();
        let entry2 = tt.load::<Chessboard>(pos2.hash_pos(), 0).unwrap();
        assert_eq!(entry.hash, pos.hash_pos());
        assert_eq!(entry2.hash, pos2.hash_pos());
        assert_ne!(entry.age(), Age(0));
        assert!(pos.is_move_legal(mov));
        assert!(pos2.is_move_legal(mov));
    }
}
