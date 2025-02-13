use crate::general::board::Board;
use crate::search::Depth;
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
            nodes = self.nodes,
            time = self.time.as_millis(),
            nps = self.nodes * 1_000_000 / self.time.as_micros() as u64
        )
    }
}

#[derive(Debug)]
pub struct SplitPerftRes<B: Board> {
    pub perft_res: PerftRes,
    pub children: Vec<(B::Move, u64)>,
}

impl<B: Board> Display for SplitPerftRes<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "info depth {depth} nodes {nodes} time {time} nps {nps}",
            depth = self.perft_res.depth.get(),
            nodes = self.perft_res.nodes,
            time = self.perft_res.time.as_millis(),
            nps = self.perft_res.nodes * 1_000_000 / self.perft_res.time.as_micros() as u64
        )?;
        for child in &self.children {
            write!(f, "\n{0}\t{1}", child.0, child.1)?;
        }
        Ok(())
    }
}

fn do_perft<B: Board>(depth: usize, pos: B) -> u64 {
    let mut nodes = 0;
    if depth == 1 {
        return pos.legal_moves_slow().into_iter().count() as u64;
    }
    // if pos.game_result_no_movegen().is_some() {
    //     return 0; // the game is over (e.g. 50mr)
    // }
    for mov in pos.pseudolegal_moves() {
        if let Some(new_pos) = pos.make_move(mov) {
            nodes += do_perft(depth - 1, new_pos);
        }
    }
    // no need to handle the case of no legal moves, since we already return 0.
    nodes
}

pub fn perft<B: Board>(depth: Depth, pos: B) -> PerftRes {
    let depth = depth.min(B::max_perft_depth());
    let start = Instant::now();
    let nodes = if depth.get() == 0 {
        1
    } else {
        do_perft(depth.get(), pos)
    };
    let time = start.elapsed();

    PerftRes { time, nodes, depth }
}

pub fn split_perft<B: Board>(depth: Depth, pos: B) -> SplitPerftRes<B> {
    assert!(depth.get() > 0);
    let depth = depth.min(B::max_perft_depth());
    let mut nodes = 0;
    let start = Instant::now();
    let mut children = vec![];
    for mov in pos.pseudolegal_moves() {
        if let Some(new_pos) = pos.make_move(mov) {
            let child_nodes = if depth.get() == 1 {
                1
            } else {
                do_perft(depth.get() - 1, new_pos)
            };
            children.push((mov, child_nodes));
            nodes += child_nodes;
        }
    }
    let time = start.elapsed();
    children.sort_by(|a, b| a.0.to_string().cmp(&b.0.to_string()));
    let perft_res = PerftRes { time, nodes, depth };
    SplitPerftRes {
        perft_res,
        children,
    }
}

pub fn perft_for<B: Board>(depth: Depth, positions: &[B]) -> PerftRes {
    let mut res = PerftRes {
        time: Duration::default(),
        nodes: 0,
        depth,
    };
    for pos in positions {
        let depth = if depth.get() == 0 || depth >= B::max_perft_depth() {
            pos.default_perft_depth()
        } else {
            depth
        };
        let this_res = perft(depth, *pos);
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
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct PosIter<B: Board> {
    depth: Depth,
    states: Vec<PerftState<B>>,
    only_leaves: bool,
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
                self.states.pop();
                self.depth += 1;
                return self.next();
            };
            let Some(new_pos) = s.pos.make_move(m) else {
                return self.next();
            };
            Some(new_pos)
        } else {
            let s = self.states.last_mut().unwrap();
            let Some(m) = s.moves.pop() else {
                self.states.pop();
                self.depth += 1;
                return self.next();
            };
            let Some(new_pos) = s.pos.make_move(m) else {
                return self.next();
            };
            let new_state = PerftState {
                pos: new_pos,
                moves: new_pos.pseudolegal_moves().into_iter().collect(),
            };
            self.states.push(new_state);
            self.depth -= 1;
            if self.only_leaves {
                self.next()
            } else {
                Some(new_pos)
            }
        }
    }
}

pub fn all_positions_at<B: Board>(depth: Depth, pos: B) -> PosIter<B> {
    let state = PerftState {
        pos,
        moves: pos.pseudolegal_moves().into_iter().collect(),
    };
    PosIter {
        depth,
        states: vec![state],
        only_leaves: false,
    }
}

#[derive(Debug, Eq, PartialEq)]
struct HashWrapper<B: Board>(B);

impl<B: Board> Hash for HashWrapper<B> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.zobrist_hash().0);
        // state.write_u64(0);
    }
}

pub fn num_unique_positions_at<B: Board>(depth: Depth, pos: B) -> usize {
    let mut set = all_positions_at(depth, pos)
        .map(|b| HashWrapper(b))
        .collect::<HashSet<_>>();
    set.insert(HashWrapper(pos));
    set.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::chess::Chessboard;
    use crate::games::mnk::MNKBoard;

    #[test]
    fn all_positions_at_mnk_test() {
        let pos = MNKBoard::from_name("tictactoe").unwrap();
        let root = all_positions_at(Depth::new_unchecked(0), pos);
        assert_eq!(root.count(), 1);
        let children = all_positions_at(Depth::new_unchecked(1), pos);
        assert_eq!(children.count(), 9);
        let grand_children = all_positions_at(Depth::new_unchecked(2), pos);
        assert_eq!(grand_children.count(), 9 * 8 + 9);
        let depth_3 = all_positions_at(Depth::new_unchecked(3), pos);
        assert_eq!(depth_3.count(), 9 * 8 * 7 + 9 * 8 + 9);
        assert_eq!(
            all_positions_at(Depth::new_unchecked(9), pos).count(),
            all_positions_at(Depth::new_unchecked(10), pos).count()
        );
    }

    #[test]
    fn num_unique_positions_in_tictactoe_test() {
        let pos = MNKBoard::default();
        let res = num_unique_positions_at(Depth::new_unchecked(9), pos);
        assert_eq!(res, num_unique_positions_at(Depth::new_unchecked(10), pos));
        assert_eq!(res, num_unique_positions_at(Depth::new_unchecked(11), pos));
    }

    #[test]
    fn num_unique_positions_at_chess_test() {
        let pos = Chessboard::from_name("mate_in_1").unwrap();
        assert_eq!(num_unique_positions_at(Depth::new_unchecked(0), pos), 1);
        assert_eq!(
            num_unique_positions_at(Depth::new_unchecked(1), pos),
            23 + 1
        );
        assert_eq!(
            num_unique_positions_at(Depth::new_unchecked(2), pos),
            53 + 23 + 1
        );
    }
}
