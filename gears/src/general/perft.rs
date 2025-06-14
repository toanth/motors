use crate::general::board::{Board, BoardHelpers};
use crate::general::moves::Move;
use crate::search::Depth;
use colored::Colorize;
use itertools::Itertools;
use rayon::iter::ParallelIterator;
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelBridge};
use std::collections::HashSet;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

#[derive(Copy, Clone, Debug)]
pub struct PerftRes {
    pub time: Duration,
    pub nodes: u64, // Can't use NodesLimit because it's possible to have 0 leafs at the given depth
    pub depth: Depth,
}

impl Display for PerftRes {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "info depth {depth} nodes {nodes} time {time} nps {nps}",
            depth = self.depth.get(),
            nodes = self.nodes.to_string().bold(),
            time = self.time.as_millis(),
            nps = self.nodes * 1_000_000 / self.time.as_micros().max(1) as u64
        )
    }
}

#[derive(Debug)]
pub struct SplitPerftRes<B: Board> {
    pub perft_res: PerftRes,
    pub children: Vec<(B::Move, u64)>,
    pub pos: B,
}

impl<B: Board> Display for SplitPerftRes<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "info depth {depth} nodes {nodes} time {time} nps {nps}",
            depth = self.perft_res.depth.get(),
            nodes = self.perft_res.nodes,
            time = self.perft_res.time.as_millis(),
            nps = self.perft_res.nodes * 1_000_000 / self.perft_res.time.as_micros().max(1) as u64
        )?;
        for child in &self.children {
            write!(f, "\n{0}\t{1}", child.0.compact_formatter(&self.pos), child.1)?;
        }
        Ok(())
    }
}

fn do_perft<B: Board>(depth: usize, pos: B) -> u64 {
    let mut nodes = 0;
    // We don't want to check for all game-over conditions, e.g. chess doesn't care about insufficient material, 50mr, or 3fold repetition.
    // However, some conditions do need to be checked in perft, e.g. mnk winning. This is done here.
    if pos.cannot_call_movegen() {
        return 0;
    }
    if depth == 1 {
        return pos.num_legal_moves() as u64;
    }
    for new_pos in pos.children() {
        nodes += do_perft(depth - 1, new_pos);
    }
    // no need to handle the case of no legal moves, since `children()` and `num_legal_moves()`
    // already take care of forced passing moves.
    nodes
}

pub fn perft<B: Board>(depth: Depth, pos: B, parallelize: bool) -> PerftRes {
    let start = Instant::now();
    let nodes = if depth.get() == 0 {
        1
    } else if depth.get() > 2 && parallelize {
        pos.children().par_bridge().map(|pos| do_perft(depth.get() - 1, pos)).sum()
    } else {
        do_perft(depth.get(), pos)
    };
    let time = start.elapsed();

    PerftRes { time, nodes, depth }
}

pub fn split_perft<B: Board>(depth: Depth, pos: B, parallelize: bool) -> SplitPerftRes<B> {
    assert!(depth.get() > 0);
    let mut nodes = 0;
    let start = Instant::now();
    let mut children: Vec<(B::Move, u64)> = vec![];
    if depth.get() > 2 && parallelize {
        pos.legal_moves_slow()
            .into_iter()
            .collect_vec()
            .par_iter()
            .map(|&mov| {
                let child_nodes = do_perft(depth.get() - 1, pos.clone().make_move(mov).unwrap());
                (mov, child_nodes)
            })
            .collect_into_vec(&mut children);
        nodes = children.iter().map(|(_, num)| num).sum();
    } else {
        // Use legal_moves_slow instead of pseudolegal_moves here to handle a forced passing move
        // because the current player has no other legal moves
        for mov in pos.legal_moves_slow() {
            let new_pos = pos.clone().make_move(mov).expect("playing a legal move cannot fail");
            let child_nodes = if depth.get() == 1 { 1 } else { do_perft(depth.get() - 1, new_pos) };
            children.push((mov, child_nodes));
            nodes += child_nodes;
        }
    }
    let time = start.elapsed();
    children.sort_by(|a, b| a.0.compact_formatter(&pos).to_string().cmp(&b.0.compact_formatter(&pos).to_string()));
    let perft_res = PerftRes { time, nodes, depth };
    SplitPerftRes { perft_res, children, pos }
}

pub fn perft_for<B: Board>(depth: Depth, positions: &[B], parallelize: bool) -> PerftRes {
    let mut res = PerftRes { time: Duration::default(), nodes: 0, depth };
    for pos in positions {
        let depth = if depth.get() == 0 { pos.default_perft_depth() } else { depth };
        let this_res = perft(depth, pos.clone(), parallelize);
        res.time += this_res.time;
        res.nodes += this_res.nodes;
        res.depth = res.depth.max(this_res.depth);
    }
    res
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct PerftState<B: Board> {
    pos: B,
    moves: Vec<B::Move>,
    num_visited_children: usize,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct PosIter<B: Board> {
    depth: Depth,
    states: Vec<PerftState<B>>,
}

impl<B: Board> PosIter<B> {
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

impl<B: Board> Iterator for PosIter<B> {
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
pub fn descendants_up_to<B: Board>(depth: Depth, pos: B) -> PosIter<B> {
    let moves = pos.pseudolegal_moves().into_iter().collect();
    let state = PerftState { pos, moves, num_visited_children: 0 };
    PosIter { depth, states: vec![state] }
}

#[derive(Debug, Eq, PartialEq)]
struct HashWrapper<B: Board>(B);

impl<B: Board> Hash for HashWrapper<B> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.hash_pos().0);
        // state.write_u64(0);
    }
}

pub fn num_unique_positions_up_to<B: Board>(depth: Depth, pos: B) -> u64 {
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
    use crate::games::ataxx::AtaxxBoard;
    use crate::games::chess::Chessboard;
    use crate::games::fairy::FairyBoard;
    use crate::games::mnk::MNKBoard;
    use crate::general::board::Strictness::Strict;

    #[test]
    fn all_positions_at_mnk_test() {
        let pos = MNKBoard::from_name("tictactoe").unwrap();
        let root = descendants_up_to(Depth::new(0), pos);
        assert_eq!(root.count(), 1);
        let children = descendants_up_to(Depth::new(1), pos);
        assert_eq!(children.count(), 9);
        let grand_children = descendants_up_to(Depth::new(2), pos);
        assert_eq!(grand_children.count(), 9 * 8 + 9);
        let depth_3 = descendants_up_to(Depth::new(3), pos);
        assert_eq!(depth_3.count(), 9 * 8 * 7 + 9 * 8 + 9);
        assert_eq!(descendants_up_to(Depth::new(9), pos).count(), descendants_up_to(Depth::new(10), pos).count());
    }

    #[test]
    fn num_unique_positions_in_tictactoe_test() {
        let pos = MNKBoard::default();
        let res = num_unique_positions_up_to(Depth::new(9), pos);
        assert_eq!(res, num_unique_positions_up_to(Depth::new(10), pos));
        assert_eq!(res, num_unique_positions_up_to(Depth::new(11), pos));
    }

    #[test]
    fn num_unique_positions_at_chess_test() {
        let pos = Chessboard::from_name("mate_in_1").unwrap();
        assert_eq!(num_unique_positions_up_to(Depth::new(0), pos), 1);
        assert_eq!(num_unique_positions_up_to(Depth::new(1), pos), 23 + 1);
        assert_eq!(num_unique_positions_up_to(Depth::new(2), pos), 53 + 23 + 1);
    }

    #[test]
    fn num_unique_fairy_ataxx_positons_tests() {
        let pos = AtaxxBoard::default();
        let fairy_pos = FairyBoard::variant_simple("ataxx").unwrap();
        let mut res1 = 0;
        let mut res2 = 0;
        for i in 0..3 {
            res1 += perft(Depth::new(i), pos, false).nodes;
            res2 += perft(Depth::new(i), fairy_pos.clone(), false).nodes;
            assert_eq!(res1, res2);
            assert_eq!(num_unique_positions_up_to(Depth::new(i), pos), res2, "{i}");
            assert_eq!(num_unique_positions_up_to(Depth::new(i), fairy_pos.clone()), res2);
        }
        let fen = "7/7/7/7/-------/-------/xxxx1oo o 0 3";
        let pos = AtaxxBoard::from_fen(fen, Strict).unwrap();
        let fairy_pos = FairyBoard::from_fen_for("ataxx", fen, Strict).unwrap();
        assert_eq!(num_unique_positions_up_to(Depth::new(0), pos), 1);
        assert_eq!(num_unique_positions_up_to(Depth::new(1), pos), 3);
        assert_eq!(num_unique_positions_up_to(Depth::new(2), pos), 4);
        assert_eq!(num_unique_positions_up_to(Depth::new(3), pos), 6);
        assert_eq!(perft(Depth::new(1), pos, false).nodes, 2);
        assert_eq!(perft(Depth::new(2), pos, false).nodes, 1);
        assert_eq!(perft(Depth::new(3), pos, false).nodes, 2);
        assert_eq!(perft(Depth::new(1), fairy_pos.clone(), false).nodes, 2);
        for p in descendants_up_to(Depth::new(2), pos) {
            println!("{p}");
        }
        assert_eq!(perft(Depth::new(2), fairy_pos.clone(), false).nodes, 1);
        assert_eq!(perft(Depth::new(3), fairy_pos.clone(), false).nodes, 2);
    }
}
