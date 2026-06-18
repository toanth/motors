use crate::games::PosHash;
use crate::general::board::{BoardHelpers, BoardTrait};
use crate::general::common::PcgXslRr128_64Oneseq;
use crate::general::moves::MoveTrait;
use crate::general::perft::Bulkness::Bulk;
use crate::search::DepthPly;
use colored::Colorize;
use itertools::Itertools;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::Relaxed;
use std::time::{Duration, Instant};
use std::{fmt, iter};

#[derive(Copy, Clone, Debug)]
pub struct PerftRes {
    pub time: Duration,
    pub nodes: u64, // Can't use NodesLimit because it's possible to have 0 leafs at the given depth
    pub depth: DepthPly,
}

impl Display for PerftRes {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Finished perft depth {depth} in {time}ms ({nps} nps)\nNodes searched: {nodes}",
            depth = self.depth.get(),
            nodes = self.nodes.to_string().bold(),
            time = self.time.as_millis(),
            nps = self.nodes * 1_000_000 / self.time.as_micros().max(1) as u64
        )
    }
}

#[derive(Debug)]
pub struct SplitPerftRes<B: BoardTrait> {
    pub perft_res: PerftRes,
    pub children: Vec<(B::Move, u64)>,
    pub pos: B,
}

impl<B: BoardTrait> Display for SplitPerftRes<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for child in &self.children {
            writeln!(f, "{0}\t{1}", child.0.compact_formatter(&self.pos), child.1)?;
        }
        write!(
            f,
            "info depth {depth} nodes {nodes} time {time} nps {nps} children {children}",
            depth = self.perft_res.depth.get(),
            nodes = self.perft_res.nodes,
            time = self.perft_res.time.as_millis(),
            nps = self.perft_res.nodes * 1_000_000 / self.perft_res.time.as_micros().max(1) as u64,
            children = self.children.len(),
        )?;
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum Bulkness {
    #[default]
    Bulk,
    NoBulk,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub enum Parallelize {
    #[default]
    SingleThreaded,
    Parallel,
}

#[derive(Debug, Default)]
pub struct PerftTTEntry {
    hash: AtomicU64,
    // The lower 8 bits encode the depth, the uper 56 bits encode the nodes.
    // If either depth or nodes don't fit, we don't store to the TT.
    nodes_and_depth: AtomicU64,
}

const DEPTH_KEYS: [PosHash; 256] = {
    let mut res = [PosHash(0); 256];
    let mut rng = PcgXslRr128_64Oneseq::new(0x123456789abcdef);
    let mut i = 0;
    while i < res.len() {
        res[i] = rng.generate();
        i += 1;
    }
    res
};

#[derive(Debug, Copy, Clone)]
pub struct PerftTTRef<'a> {
    array: &'a [PerftTTEntry],
}

impl<'a> PerftTTRef<'a> {
    fn new(array: &'a Option<Box<[PerftTTEntry]>>) -> Option<Self> {
        Some(Self { array: array.as_ref()? })
    }

    fn len(&self) -> usize {
        self.array.len()
    }

    fn index_of(&self, hash: PosHash) -> usize {
        // Like the TT in motors, this uses Lemire's multiplication trick:
        // <https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/>
        ((hash.0 as u128 * self.len() as u128) >> 64) as usize
    }

    fn lookup(&self, hash: PosHash, depth: usize) -> Option<u64> {
        if depth > 255 {
            return None;
        }
        let hash = hash ^ DEPTH_KEYS[depth];
        let idx = self.index_of(hash);
        let entry = &self.array[idx];
        let (entry_hash, nodes_and_depth) = (entry.hash.load(Relaxed), entry.nodes_and_depth.load(Relaxed));
        let entry_depth = nodes_and_depth & 0xff;
        let entry_nodes = nodes_and_depth >> 8;
        if entry_hash == hash.0 && entry_depth as usize == depth {
            return Some(entry_nodes);
        }
        None
    }

    fn store(&self, hash: PosHash, depth: usize, nodes: u64) {
        if depth > 255 || nodes > u64::MAX >> 8 {
            return;
        }
        let hash = hash ^ DEPTH_KEYS[depth];
        let idx = self.index_of(hash);
        let nodes_and_hash = (nodes << 8) | depth as u64;
        self.array[idx].hash.store(hash.0, Relaxed);
        self.array[idx].nodes_and_depth.store(nodes_and_hash, Relaxed);
    }
}

fn do_perft<B: BoardTrait>(
    depth: usize,
    pos: B,
    pseudo_bulk: Bulkness,
    parallelize: bool,
    tt: Option<PerftTTRef>,
) -> u64 {
    if depth == 0 {
        return 1;
    }
    // We don't want to check for all game-over conditions, e.g. chess doesn't care about insufficient material, 50mr, or 3fold repetition.
    // However, some conditions do need to be checked in perft, e.g. mnk winning. This is done here.
    if pos.cannot_call_movegen() {
        return 0;
    }
    if pseudo_bulk == Bulk && depth == 1 {
        return pos.num_legal_moves() as u64;
    }
    let hash = pos.hash_pos();
    // Returning TT entries can of course give us incorrect results in case of hash collision,
    // but those should be *relatively* rare.
    if let Some(tt) = tt
        && let Some(nodes) = tt.lookup(hash, depth)
    {
        return nodes;
    }
    let mut nodes = 0;
    if depth + 2 > pos.default_perft_depth().get() && parallelize {
        nodes = pos.children().par_bridge().map(|pos| do_perft(depth - 1, pos, pseudo_bulk, parallelize, tt)).sum();
    } else {
        let mut has_children = false;
        pos.gen_pseudolegal(|m| {
            let Some(new_pos) = pos.clone().make_move(m) else { return };
            nodes += do_perft(depth - 1, new_pos, pseudo_bulk, parallelize, tt);
            has_children = true;
        });
        // Unlike the other move generation functions, `gen_pseudolegal` doesn't deal with forced passing moves,
        // so we have to do that here ourselves
        if !has_children && pos.no_moves_result().is_none() {
            nodes += do_perft(depth - 1, pos.make_nullmove().unwrap(), pseudo_bulk, parallelize, tt);
        }
    }
    // Currently, we're just always replacing. In the future, it might make sense to use buckets and a better replacement policy.
    if let Some(tt) = tt {
        tt.store(hash, depth, nodes);
    }
    nodes
    // no need to handle the case of no legal moves, since `children()` and `num_legal_moves()`
    // already take care of forced passing moves.
}

fn create_tt(tt_bytes: Option<usize>) -> Option<Box<[PerftTTEntry]>> {
    let len = tt_bytes? / size_of::<PerftTTEntry>().min(1);
    Some(if cfg!(feature = "unsafe") {
        unsafe { Box::new_zeroed_slice(len).assume_init() }
    } else {
        iter::repeat_with(|| PerftTTEntry::default()).take(len).collect_vec().into_boxed_slice()
    })
}

pub fn perft<B: BoardTrait>(
    depth: DepthPly,
    pos: B,
    parallelize: Parallelize,
    pseudo_bulk: Bulkness,
    tt_bytes: Option<usize>,
) -> PerftRes {
    let tt = create_tt(tt_bytes);
    let tt = PerftTTRef::new(&tt);
    let start = Instant::now();
    let nodes = do_perft(depth.get(), pos, pseudo_bulk, parallelize == Parallelize::Parallel, tt);
    let time = start.elapsed();

    PerftRes { time, nodes, depth }
}

pub fn split_perft<B: BoardTrait>(
    depth: DepthPly,
    pos: B,
    parallelize: Parallelize,
    pseudo_bulk: Bulkness,
    tt_bytes: Option<usize>,
) -> SplitPerftRes<B> {
    assert!(depth.get() > 0);
    let tt = create_tt(tt_bytes);
    let tt = PerftTTRef::new(&tt);
    let mut nodes = 0;
    let start = Instant::now();
    let mut children: Vec<(B::Move, u64)> = vec![];
    let parallelize = parallelize == Parallelize::Parallel;
    if depth.get() > 2 && parallelize {
        pos.legal_moves_slow()
            .into_iter()
            .collect_vec()
            .par_iter()
            .map(|&mov| {
                let child_nodes =
                    do_perft(depth.get() - 1, pos.clone().make_move(mov).unwrap(), pseudo_bulk, parallelize, tt);
                (mov, child_nodes)
            })
            .collect_into_vec(&mut children);
        nodes = children.iter().map(|(_, num)| num).sum();
    } else {
        for mov in pos.legal_moves_slow() {
            let new_pos = pos.clone().make_move(mov).expect("playing a legal move cannot fail");
            let child_nodes =
                if depth.get() == 1 { 1 } else { do_perft(depth.get() - 1, new_pos, pseudo_bulk, parallelize, tt) };
            children.push((mov, child_nodes));
            nodes += child_nodes;
        }
    }
    let time = start.elapsed();
    children.sort_by_key(|(m, _)| m.compact_formatter(&pos).to_string());
    let perft_res = PerftRes { time, nodes, depth };
    SplitPerftRes { perft_res, children, pos }
}

pub fn perft_for<B: BoardTrait>(
    depth: DepthPly,
    positions: &[B],
    parallelize: Parallelize,
    pseudo_bulk: Bulkness,
    tt_bytes: Option<usize>,
) -> PerftRes {
    let mut res = PerftRes { time: Duration::default(), nodes: 0, depth };
    for pos in positions {
        let depth = if depth.get() == 0 { pos.default_perft_depth() } else { depth };
        let this_res = perft(depth, pos.clone(), parallelize, pseudo_bulk, tt_bytes);
        res.time += this_res.time;
        res.nodes += this_res.nodes;
        res.depth = res.depth.max(this_res.depth);
    }
    res
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct PerftState<B: BoardTrait> {
    pos: B,
    moves: Vec<B::Move>,
    num_visited_children: usize,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct PosIter<B: BoardTrait> {
    depth: DepthPly,
    states: Vec<PerftState<B>>,
}

impl<B: BoardTrait> PosIter<B> {
    fn no_moves(&mut self) -> Option<B> {
        let s = self.states.last_mut().unwrap();
        if s.num_visited_children == 0 && s.pos.no_moves_result().is_none() {
            s.num_visited_children = 1;
            let new_pos = s.pos.clone().make_nullmove().expect("A forced passing move must be legal");
            let moves = new_pos.pseudolegal_moves().into_iter().collect();
            let new_state = PerftState { pos: new_pos.clone(), moves, num_visited_children: 0 };
            self.states.push(new_state);
            self.depth -= 1;
            return Some(new_pos);
        }
        _ = self.states.pop();
        self.depth += 1;
        self.next()
    }
}

impl<B: BoardTrait> Iterator for PosIter<B> {
    type Item = B;

    fn next(&mut self) -> Option<Self::Item> {
        if self.states.is_empty() {
            None
        } else if self.depth.get() == 0 {
            Some(self.states.pop()?.pos)
        } else if self.depth.get() == 1 {
            let s = self.states.last_mut().unwrap();
            let Some(m) = s.moves.pop() else {
                return self.no_moves();
            };
            let Some(new_pos) = s.pos.clone().make_move(m) else {
                return self.next();
            };
            s.num_visited_children += 1;
            Some(new_pos)
        } else {
            let s = self.states.last_mut().unwrap();
            let Some(m) = s.moves.pop() else {
                return self.no_moves();
            };
            let Some(new_pos) = s.pos.clone().make_move(m) else {
                return self.next();
            };
            s.num_visited_children += 1;
            let moves = new_pos.pseudolegal_moves().into_iter().collect();
            let new_state = PerftState { pos: new_pos.clone(), moves, num_visited_children: 0 };
            self.states.push(new_state);
            self.depth -= 1;
            Some(new_pos)
        }
    }
}

/// excludes the root
pub fn descendants_up_to<B: BoardTrait>(depth: DepthPly, pos: B) -> PosIter<B> {
    let moves = pos.pseudolegal_moves().into_iter().collect();
    let state = PerftState { pos, moves, num_visited_children: 0 };
    PosIter { depth, states: vec![state] }
}

#[derive(Debug, Eq, PartialEq)]
struct HashWrapper<B: BoardTrait>(B);

impl<B: BoardTrait> Hash for HashWrapper<B> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.hash_pos().0);
        // state.write_u64(0);
    }
}

pub fn num_unique_positions_up_to<B: BoardTrait>(depth: DepthPly, pos: B) -> u64 {
    if depth.get() == 0 {
        return 1;
    }
    let subtree_res =
        pos.children().par_bridge().flat_map_iter(|c| descendants_up_to(depth - 1, c).map(|b| HashWrapper(b)));
    let mut set = subtree_res.collect::<HashSet<_>>();
    // let mut set = all_positions_at(depth, pos.clone()).map(|b| HashWrapper(b)).collect::<HashSet<_>>();
    for c in pos.children() {
        _ = set.insert(HashWrapper(c));
    }
    _ = set.insert(HashWrapper(pos));
    set.len() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::{ataxx, chess, fairy, mnk};
    use crate::general::board::Strictness::Strict;
    use crate::general::perft::Bulkness::NoBulk;
    use crate::general::perft::Parallelize::*;

    #[test]
    fn all_positions_at_mnk_test() {
        let pos = mnk::Board::from_name("tictactoe").unwrap();
        let root = descendants_up_to(DepthPly::new(0), pos);
        assert_eq!(root.count(), 1);
        let children = descendants_up_to(DepthPly::new(1), pos);
        assert_eq!(children.count(), 9);
        let grand_children = descendants_up_to(DepthPly::new(2), pos);
        assert_eq!(grand_children.count(), 9 * 8 + 9);
        let depth_3 = descendants_up_to(DepthPly::new(3), pos);
        assert_eq!(depth_3.count(), 9 * 8 * 7 + 9 * 8 + 9);
        assert_eq!(descendants_up_to(DepthPly::new(9), pos).count(), descendants_up_to(DepthPly::new(10), pos).count());
    }

    #[test]
    fn num_unique_positions_in_tictactoe_test() {
        let pos = mnk::Board::default();
        assert_eq!(num_unique_positions_up_to(DepthPly::new(1), pos), 10);
        let res = num_unique_positions_up_to(DepthPly::new(9), pos);
        assert_eq!(res, num_unique_positions_up_to(DepthPly::new(10), pos));
    }

    #[test]
    fn num_unique_positions_at_chess_test() {
        let pos = chess::Board::from_name("mate_in_1").unwrap();
        assert_eq!(num_unique_positions_up_to(DepthPly::new(0), pos), 1);
        assert_eq!(num_unique_positions_up_to(DepthPly::new(1), pos), 23 + 1);
        assert_eq!(num_unique_positions_up_to(DepthPly::new(2), pos), 53 + 23 + 1);
    }

    #[test]
    fn num_unique_fairy_ataxx_positons_tests() {
        let pos = ataxx::Board::default();
        let fairy_pos = fairy::Board::variant_simple("ataxx").unwrap();
        let mut res1 = 0;
        let mut res2 = 0;
        for i in 0..3 {
            res1 += perft(DepthPly::new(i), pos, SingleThreaded, Bulk, None).nodes;
            res2 += perft(DepthPly::new(i), fairy_pos.clone(), SingleThreaded, Bulk, None).nodes;
            assert_eq!(res1, res2);
            assert_eq!(num_unique_positions_up_to(DepthPly::new(i), pos), res2, "{i}");
            assert_eq!(num_unique_positions_up_to(DepthPly::new(i), fairy_pos.clone()), res2);
        }
        let fen = "7/7/7/7/-------/-------/xxxx1oo o 0 3";
        let pos = ataxx::Board::from_fen(fen, Strict).unwrap();
        let fairy_pos = fairy::Board::from_fen_for("ataxx", fen, Strict).unwrap();
        assert_eq!(pos.as_fen(), fairy_pos.fen_no_rules());
        assert_eq!(num_unique_positions_up_to(DepthPly::new(0), pos), 1);
        assert_eq!(num_unique_positions_up_to(DepthPly::new(1), pos), 3);
        assert_eq!(num_unique_positions_up_to(DepthPly::new(2), pos), 4);
        assert_eq!(num_unique_positions_up_to(DepthPly::new(3), pos), 6);
        assert_eq!(perft(DepthPly::new(1), pos, SingleThreaded, Bulk, None).nodes, 2);
        assert_eq!(perft(DepthPly::new(2), pos, SingleThreaded, Bulk, None).nodes, 1);
        assert_eq!(perft(DepthPly::new(3), pos, SingleThreaded, NoBulk, Some(1024)).nodes, 2);
        assert_eq!(perft(DepthPly::new(1), fairy_pos.clone(), SingleThreaded, Bulk, None).nodes, 2);
        for p in descendants_up_to(DepthPly::new(2), pos) {
            println!("{p}");
        }
        assert_eq!(perft(DepthPly::new(2), fairy_pos.clone(), SingleThreaded, Bulk, None).nodes, 1);
        assert_eq!(perft(DepthPly::new(3), fairy_pos.clone(), SingleThreaded, Bulk, Some(12)).nodes, 2);
    }
}
