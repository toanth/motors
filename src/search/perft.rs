use std::fmt;
use std::fmt::{Display, Formatter};
use std::time::{Duration, Instant};

use crate::games::{Board, BoardHistory, Move, ZobristHistoryBase};
use crate::search::{SearchLimit, SearchResult, Searcher, TimeControl};

#[derive(Copy, Clone, Debug, Default)]
pub struct PerftRes {
    pub time: Duration,
    pub nodes: u64,
    pub depth: usize,
}

impl Display for PerftRes {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "info depth {depth} nodes {nodes} time {time} nps {nps}",
            depth = self.depth,
            nodes = self.nodes,
            time = self.time.as_millis(),
            nps = self.nodes * 1_000_000 / self.time.as_micros() as u64
        )
    }
}

#[derive(Default, Debug)]
pub struct SplitPerftRes<T: Board> {
    pub perft_res: PerftRes,
    pub children: Vec<(T::Move, u64)>,
}

impl<B: Board> Display for SplitPerftRes<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "info depth {depth} nodes {nodes} time {time} nps {nps}",
            depth = self.perft_res.depth,
            nodes = self.perft_res.nodes,
            time = self.perft_res.time.as_millis(),
            nps = self.perft_res.nodes * 1_000_000 / self.perft_res.time.as_micros() as u64
        )?;
        for child in &self.children {
            write!(f, "\n{0}\t{1}", child.0, child.1)?;
        }
        write!(f, "") // TODO: This is probably a bad idea, it just makes the compiler happy
    }
}

fn do_perft<T: Board>(depth: usize, pos: T) -> u64 {
    let mut nodes = 0;
    if depth == 0 {
        return 1;
    }
    for mov in pos.pseudolegal_moves() {
        if let Some(new_pos) = pos.make_move(mov) {
            nodes += do_perft(depth - 1, new_pos);
        }
    }
    nodes
}

pub fn perft<T: Board>(depth: usize, pos: T) -> PerftRes {
    let start = Instant::now();
    let nodes = do_perft(depth, pos);
    let time = start.elapsed();

    PerftRes { time, nodes, depth }
}

pub fn split_perft<T: Board>(depth: usize, pos: T) -> SplitPerftRes<T> {
    assert!(depth > 0);
    let mut nodes = 0;
    let mut res: SplitPerftRes<T> = Default::default();
    let start = Instant::now();
    for mov in pos.pseudolegal_moves() {
        if let Some(new_pos) = pos.make_move(mov) {
            let child_nodes = do_perft(depth - 1, new_pos);
            res.children.push((mov, child_nodes));
            nodes += child_nodes;
        }
    }
    let time = start.elapsed();
    res.children
        .sort_by(|a, b| a.0.to_compact_text().cmp(&b.0.to_compact_text()));
    res.perft_res = PerftRes { time, nodes, depth };
    res
}

#[derive(Debug, Default)]
pub struct PerftSearcher {
    pub result: PerftRes,
}

// TODO: This is completely unnecessary, remove
impl<B: Board> Searcher<B> for PerftSearcher {
    fn search(
        &mut self,
        pos: B,
        limit: SearchLimit,
        _history: ZobristHistoryBase,
    ) -> SearchResult<B> {
        self.result = perft(limit.depth, pos);
        let mut res = SearchResult::default();
        res.additional
            .insert("nodes".to_string(), self.result.nodes.to_string());
        res.additional
            .insert("depth".to_string(), self.result.depth.to_string());
        res.additional
            .insert("time".to_string(), self.result.time.as_millis().to_string());
        res
    }

    fn time_up(&self, _: TimeControl, _: Duration, _: Instant) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "Perft"
    }
}

#[derive(Debug, Default)]
pub struct SplitPerftSearcher<B: Board> {
    result: SplitPerftRes<B>,
}

// TODO: Also completely unnecessary, remove this as well
impl<B: Board> Searcher<B> for SplitPerftSearcher<B> {
    fn search(
        &mut self,
        pos: B,
        limit: SearchLimit,
        _history: ZobristHistoryBase,
    ) -> SearchResult<B> {
        self.result = split_perft(limit.depth, pos);
        let mut res = SearchResult::default();
        res.additional
            .insert("nodes".to_string(), self.result.perft_res.nodes.to_string());
        res.additional
            .insert("depth".to_string(), self.result.perft_res.depth.to_string());
        res.additional.insert(
            "time".to_string(),
            self.result.perft_res.time.as_millis().to_string(),
        );
        for child in self.result.children.iter() {
            res.additional
                .insert(child.0.to_string(), child.1.to_string());
        }
        res
    }

    fn time_up(&self, _tc: TimeControl, _hard_limit: Duration, _start_time: Instant) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "Split Perft"
    }
}
