use crate::search::statistics::Mode::{Average, Percentage};
use crate::search::statistics::NodeCounterType::{Begun, Completed};
use crate::search::statistics::SearchCounter::*;
use crate::search::statistics::SearchType::*;
use derive_more::Display;
#[expect(unused_imports)]
use gears::itertools::Itertools;
use gears::search::NodeType;
use std::fmt::Formatter;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Debug, Default, Copy, Clone)]
pub struct NodeTypeCtr {
    pub fail_highs: u64,
    pub exact: u64,
    pub fail_lows: u64,
}

impl NodeTypeCtr {
    #[allow(unused)]
    fn increment(&mut self, node_type: NodeType) {
        let ctr = match node_type {
            NodeType::FailHigh => &mut self.fail_highs,
            NodeType::Exact => &mut self.exact,
            NodeType::FailLow => &mut self.fail_lows,
        };
        *ctr += 1;
    }

    #[must_use]
    pub fn sum(&self) -> u64 {
        self.exact + self.fail_lows + self.fail_highs
    }

    fn aggregate(&mut self, other: NodeTypeCtr) {
        self.fail_lows += other.fail_lows;
        self.fail_highs += other.fail_highs;
        self.exact += other.exact;
    }
}

#[derive(Debug, Display, EnumIter)]
enum SearchCounter {
    DepthAvg,
    PlyAvg,
    TTMisses,
    CutoffAfterFirstChild,
    NodesStarted,
    LegalMakeMoveCalls,
    NumCounters,
}

#[derive(Copy, Clone)]
enum NodeCounterType {
    Begun,
    Completed,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct SearchTypeStatistics {
    node_ctr: NodeTypeCtr,
    tt_cutoffs: NodeTypeCtr,
    counters: [u64; NumCounters as usize],
}

impl SearchTypeStatistics {
    fn aggregate(&mut self, other: &SearchTypeStatistics) {
        for i in 0..NumCounters as usize {
            self.counters[i] += other.counters[i];
        }
        self.node_ctr.aggregate(other.node_ctr);
        self.tt_cutoffs.aggregate(other.tt_cutoffs);
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct IDStatistics {
    main_search: SearchTypeStatistics,
    qsearch: SearchTypeStatistics,
    // with stages movegen, this can also count how often we've reached different phases
    lmr_first_retry: u64,
    lmr_second_retry: u64,
    in_check: u64,
    aw: NodeTypeCtr,
}

impl IDStatistics {
    pub fn search(&self, search_type: SearchType) -> &SearchTypeStatistics {
        match search_type {
            MainSearch => &self.main_search,
            Qsearch => &self.qsearch,
        }
    }

    pub fn search_mut(&mut self, search_type: SearchType) -> &mut SearchTypeStatistics {
        match search_type {
            MainSearch => &mut self.main_search,
            Qsearch => &mut self.qsearch,
        }
    }

    pub fn aggregate(&mut self, other: &IDStatistics) {
        self.lmr_first_retry += other.lmr_first_retry;
        self.lmr_second_retry += other.lmr_second_retry;
        self.in_check += other.in_check;
        self.main_search.aggregate(&other.main_search);
        self.qsearch.aggregate(&other.qsearch);
        self.aw.aggregate(other.aw);
    }
}

#[cfg(feature = "statistics")]
#[derive(Debug, Default, Clone)]
pub struct Statistics {
    iterations: Vec<IDStatistics>,
    /// can be 1 smaller than the length of `iterations` because it only counts completed depths
    depth: usize,
    soft_limit_stop: usize, // 1 iff the current search was stopped after reaching the soft limit, 0 otherwise.
    num_searches: usize,
}

#[cfg(not(feature = "statistics"))]
#[derive(Debug, Default, Clone)]
pub struct Statistics {}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum SearchType {
    MainSearch,
    Qsearch,
}

impl Statistics {}

#[cfg(feature = "statistics")]
impl Statistics {
    fn cur(&self) -> &IDStatistics {
        self.iterations.last().unwrap()
    }

    fn cur_mut(&mut self) -> &mut IDStatistics {
        self.iterations.last_mut().unwrap()
    }

    pub fn search(&self, search_type: SearchType) -> &SearchTypeStatistics {
        self.cur().search(search_type)
    }

    pub fn search_mut(&mut self, search_type: SearchType) -> &mut SearchTypeStatistics {
        self.cur_mut().search_mut(search_type)
    }

    pub fn nodes_started(&self, search_type: SearchType) -> u64 {
        let search = self.iterations.last().unwrap().search(search_type);
        search.counters[NodesStarted as usize]
    }

    /// Returns the number of *completed* iterations of ID, so one less than the current depth if search is ongoing.
    pub fn depth(&self) -> usize {
        self.depth
    }

    #[inline(always)]
    pub fn count_legal_make_move(&mut self, search_type: SearchType) {
        self.search_mut(search_type).counters[LegalMakeMoveCalls as usize] += 1;
    }

    #[inline(always)]
    pub fn main_search_nodes(&self) -> u64 {
        self.search(MainSearch).counters[LegalMakeMoveCalls as usize]
    }

    #[inline(always)]
    pub fn uci_nodes(&self) -> u64 {
        // + 1 because the root node also counts
        self.search(MainSearch).counters[LegalMakeMoveCalls as usize]
            + self.search(Qsearch).counters[LegalMakeMoveCalls as usize]
            + 1
    }

    #[inline(always)]
    pub fn aw_node_type(&mut self, node_type: NodeType) {
        match node_type {
            NodeType::FailHigh => self.cur_mut().aw.fail_highs += 1,
            NodeType::Exact => self.cur_mut().aw.exact += 1,
            NodeType::FailLow => self.cur_mut().aw.fail_lows += 1,
        }
    }

    pub fn next_id_iteration(&mut self) {
        self.iterations.push(IDStatistics::default());
        self.depth += 1;
    }

    pub fn soft_limit_stop(&mut self) {
        self.soft_limit_stop = 1;
    }

    pub fn count_complete_node(
        &mut self,
        search_type: SearchType,
        node_type: NodeType,
        depth: isize,
        ply: usize,
        visited_children: usize,
    ) {
        let search = self.search_mut(search_type);
        search.node_ctr.increment(node_type);
        search.counters[PlyAvg as usize] += ply as u64;
        search.counters[DepthAvg as usize] += depth as u64;
        if visited_children == 1 {
            search.counters[CutoffAfterFirstChild as usize] += 1;
        }
    }

    /// This counts all nodes (except the root node), unlike `count_complete_node`,
    /// which only counts nodes where the moves loop has completed, so it doesn't count TT cutoffs.
    pub fn count_node_started(&mut self, search_type: SearchType) {
        self.search_mut(search_type).counters[NodesStarted as usize] += 1;
    }

    pub fn in_check(&mut self) {
        self.cur_mut().in_check += 1;
    }

    pub fn tt_miss(&mut self, search_type: SearchType) {
        self.search_mut(search_type).counters[TTMisses as usize] += 1;
    }

    pub fn tt_cutoff(&mut self, search_type: SearchType, node_type: NodeType) {
        self.search_mut(search_type).tt_cutoffs.increment(node_type);
    }

    pub fn lmr_first_retry(&mut self) {
        self.cur_mut().lmr_first_retry += 1;
    }

    pub fn lmr_second_retry(&mut self) {
        self.cur_mut().lmr_second_retry += 1;
    }

    pub fn end_search(&mut self) {
        // saturating because it's possible to abort te search before even starting depth 1
        self.depth = self.depth.saturating_sub(1);
    }

    #[must_use]
    pub fn aggregate_iterations(&self) -> IDStatistics {
        let mut res = IDStatistics::default();
        for s in self.iterations.iter() {
            res.aggregate(s);
        }
        res
    }

    pub fn aggregate_searches(&mut self, other: &Statistics) {
        self.depth = self.depth.max(other.depth);
        self.soft_limit_stop += other.soft_limit_stop;
        self.iterations.resize(
            self.iterations.len().max(other.iterations.len()),
            IDStatistics::default(),
        );
        for i in 0..self.iterations.len().min(other.iterations.len()) {
            self.iterations[i].aggregate(&other.iterations[i]);
        }
        self.num_searches += 1;
    }
}

#[cfg(not(feature = "statistics"))]
impl Statistics {
    #[inline(always)]
    pub fn next_id_iteration(&mut self) {}

    #[inline(always)]
    pub fn count_complete_node(
        &mut self,
        _search_type: SearchType,
        _node_type: NodeType,
        _depth: isize,
        _ply: usize,
        _children_visited: usize,
    ) {
    }

    #[inline(always)]
    pub fn count_node_started(&mut self, _search_type: SearchType) {}

    #[inline(always)]
    pub fn in_check(&mut self) {}

    #[inline(always)]
    pub fn tt_miss(&mut self, _search_type: SearchType) {}

    #[inline(always)]
    pub fn tt_cutoff(&mut self, _search_type: SearchType, _node_type: NodeType) {}

    #[inline(always)]
    pub fn lmr_first_retry(&mut self) {}

    #[inline(always)]
    pub fn lmr_second_retry(&mut self) {}

    #[must_use]
    pub fn aggregate_iterations(&self) -> IDStatistics {
        IDStatistics::default()
    }

    #[inline(always)]
    pub fn aggregate_searches(&mut self, _other: &Statistics) { /*do nothing*/
    }

    #[inline(always)]
    pub fn depth(&self) -> usize {
        0
    }

    #[inline(always)]
    pub fn sel_depth(&self) -> usize {
        0
    }

    #[inline(always)]
    pub fn nodes_started(&self, _search_type: SearchType) -> u64 {
        0
    }
    #[inline(always)]
    pub fn aw_node_type(&mut self, _node_type: NodeType) {}

    #[inline(always)]
    pub fn soft_limit_stop(&mut self) {}

    #[inline(always)]
    pub fn count_legal_make_move(&mut self, _search_type: SearchType) {}

    #[inline(always)]
    pub fn main_search_nodes(&self) -> u64 {
        0
    }

    #[inline(always)]
    pub fn uci_nodes(&self) -> u64 {
        1
    }

    pub fn end_search(&mut self) {}
}

#[derive(Copy, Clone)]
enum Mode {
    Percentage,
    Average,
}

pub struct IDSummary {
    nodes: u64,
    statistics: IDStatistics,
    depth: u64,
}

impl IDSummary {
    pub fn new(statistics: &IDStatistics, depth: u64) -> Self {
        let nodes = statistics.search(MainSearch).counters[NodesStarted as usize];
        Self {
            nodes,
            statistics: *statistics,
            depth,
        }
    }

    fn format_ctr(mode: Mode, val: u64, total: u64, total_completed: Option<u64>) -> String {
        let relative = |total: u64| val as f64 / total as f64;
        let res = match mode {
            Percentage => {
                format!(
                    "{val} ({:.1}%)",
                    relative(total) * 100.0 /*multiply by 100.0 last for better precision*/
                )
            }
            Average => {
                format!("avg {:.1}", relative(total))
            }
        };
        if let Some(total) = total_completed {
            match mode {
                Percentage => format!("{res} [{:.1}%]", relative(total) * 100.0),
                Average => format!("{res} [{:.1}]", relative(total)),
            }
        } else {
            res
        }
    }
}

impl Display for IDSummary {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            " - depth {depth}, total nodes {nodes}, in check {in_check}",
            depth = self.depth,
            nodes = self.nodes,
            in_check = self.statistics.in_check
        )?;
        let write_node_ctr =
            |ctr: NodeTypeCtr, total: u64, name: &str, f: &mut Formatter<'_>| -> std::fmt::Result {
                let total_completed = ctr.sum();
                write!(
                    f,
                    ",  {name} fail low: {0}, exact: {1}, fail high: {2}",
                    Self::format_ctr(Percentage, ctr.fail_lows, total, Some(total_completed)),
                    Self::format_ctr(Percentage, ctr.exact, total, Some(total_completed)),
                    Self::format_ctr(Percentage, ctr.fail_highs, total, Some(total_completed)),
                )
            };
        write_node_ctr(
            self.statistics.aw,
            self.statistics.aw.sum(),
            "asp windows",
            f,
        )?;
        let main_nodes = [
            self.statistics.main_search.counters[NodesStarted as usize],
            self.statistics.main_search.node_ctr.sum(),
        ];
        let qsearch_nodes = [
            self.statistics.qsearch.counters[NodesStarted as usize],
            self.statistics.qsearch.node_ctr.sum(),
        ];
        assert!(main_nodes[Begun as usize] >= main_nodes[Completed as usize]);
        assert!(qsearch_nodes[Begun as usize] >= qsearch_nodes[Completed as usize]);
        write!(
            f,
            ",  completed: {0} and {1}",
            Self::format_ctr(Percentage, main_nodes[1], main_nodes[0], None),
            Self::format_ctr(Percentage, qsearch_nodes[1], qsearch_nodes[0], None),
        )?;
        for i in 0..NumCounters as usize {
            let ctr = SearchCounter::iter().nth(i).unwrap();
            let (mode, typ) = match ctr {
                DepthAvg | PlyAvg => (Average, Completed),
                _ => (Percentage, Begun),
            };
            let total_completed = match ctr {
                CutoffAfterFirstChild => Some(Completed as usize),
                _ => None,
            };
            let main = self.statistics.main_search.counters[i];
            let qsearch = self.statistics.qsearch.counters[i];
            write!(
                f,
                ",  {ctr}: {0} and {1}",
                Self::format_ctr(
                    mode,
                    main,
                    main_nodes[typ as usize],
                    total_completed.map(|i| main_nodes[i])
                ),
                Self::format_ctr(
                    mode,
                    qsearch,
                    qsearch_nodes[typ as usize],
                    total_completed.map(|i| qsearch_nodes[i])
                ),
            )?;
        }
        write_node_ctr(
            self.statistics.main_search.node_ctr,
            main_nodes[Begun as usize],
            "main search",
            f,
        )?;
        write_node_ctr(
            self.statistics.qsearch.node_ctr,
            qsearch_nodes[Begun as usize],
            "quiescent search",
            f,
        )?;
        Ok(())
    }
}

pub struct Summary {
    id_summary: Vec<IDSummary>,
    total: IDSummary,
    soft_limit_stop: usize,
    num_searches: usize,
}

impl Summary {
    #[cfg(feature = "statistics")]
    pub fn new(statistics: &Statistics) -> Self {
        let id_summary = statistics
            .iterations
            .iter()
            .enumerate()
            .filter(|(_, stats)| stats.aw.sum() > 0)
            .map(|(depth, stats)| IDSummary::new(stats, depth as u64 + 1))
            .collect_vec();
        let total = IDSummary::new(
            &statistics
                .iterations
                .iter()
                .fold(IDStatistics::default(), |mut a, b| {
                    a.aggregate(b);
                    a
                }),
            id_summary.len() as u64,
        );
        Self {
            id_summary,
            total,
            soft_limit_stop: statistics.soft_limit_stop,
            num_searches: statistics.num_searches + 1,
        }
    }
    #[cfg(not(feature = "statistics"))]
    pub fn new(_statistics: &Statistics) -> Self {
        panic!("Cannot generate summaries for statistics unless the 'statistics' feature has been enabled at compile time");
    }
}

impl Display for Summary {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "(Completed nodes are nodes where the code reached the return at the end, which means they are \
        the nodes where at least one move was considered. If two percentages are given, those in between (parentheses) \
        are as a percentage of all nodes, while those in between [brackets] are as a percentage of completed nodes.\
        If values are listed as 'x and y', then x is the main search and y is quiescent search.")?;
        writeln!(f, "aggregated {} search(es)", self.num_searches)?;
        writeln!(
            f,
            "Stopped after reaching the soft limit: {}",
            self.soft_limit_stop
        )?;
        for id in &self.id_summary {
            writeln!(f, "{id}")?;
        }
        writeln!(f, "Total: {}", self.total)
    }
}
