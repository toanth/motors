use std::fmt;
use std::fmt::{Display, Formatter};
use std::time::{Duration, Instant};

use crate::games::{Board, Move};
use crate::search::Depth;

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
pub struct SplitPerftRes<T: Board> {
    pub perft_res: PerftRes,
    pub children: Vec<(T::Move, u64)>,
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

fn do_perft<T: Board>(depth: usize, pos: T) -> u64 {
    let mut nodes = 0;
    if depth == 0 {
        return 1;
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

pub fn perft<T: Board>(depth: Depth, pos: T) -> PerftRes {
    let start = Instant::now();
    let nodes = do_perft(depth.get(), pos);
    let time = start.elapsed();

    PerftRes { time, nodes, depth }
}

pub fn split_perft<T: Board>(depth: Depth, pos: T) -> SplitPerftRes<T> {
    assert!(depth.get() > 0);
    let mut nodes = 0;
    let start = Instant::now();
    let mut children = vec![];
    for mov in pos.pseudolegal_moves() {
        if let Some(new_pos) = pos.make_move(mov) {
            let child_nodes = do_perft(depth.get() - 1, new_pos);
            children.push((mov, child_nodes));
            nodes += child_nodes;
        }
    }
    let time = start.elapsed();
    children.sort_by(|a, b| a.0.to_compact_text().cmp(&b.0.to_compact_text()));
    let perft_res = PerftRes { time, nodes, depth };
    SplitPerftRes {
        perft_res,
        children,
    }
}
