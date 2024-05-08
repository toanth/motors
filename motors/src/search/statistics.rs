use crate::search::statistics::Mode::{Average, Percentage};
use crate::search::statistics::SearchCounter::{
    DepthSum, NoTTMoveCutoff, Nodes, NumCounters, PlySum, TTMisses,
};
use crate::search::statistics::SearchType::Qsearch;
use crate::search::NodeType;
use arrayvec::ArrayVec;
use derive_more::Display;
use std::fmt::Formatter;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

const MAX_ID_ITERATIONS: usize = 256;

#[derive(Debug, Default, Copy, Clone)]
pub struct NodeTypeCtr {
    pub fail_highs: u64,
    pub exact: u64,
    pub fail_lows: u64,
}

impl NodeTypeCtr {
    fn increment(&mut self, node_type: NodeType) {
        let ctr = &mut match node_type {
            NodeType::Empty => panic!(),
            NodeType::LowerBound => self.fail_lows,
            NodeType::Exact => self.exact,
            NodeType::UpperBound => self.fail_highs,
        };
        *ctr += 1;
    }
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
    DepthSum,
    PlySum,
    TTMisses,
    TTMoveMisses,
    NoTTMoveCutoff,
    Nodes,
    NumCounters,
}

#[derive(Debug, Default, Copy, Clone)]
struct SearchTypeStatistics {
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
}

impl IDStatistics {
    pub fn search(&mut self, search_type: SearchType) -> &mut SearchTypeStatistics {
        match search_type {
            SearchType::MainSearch => &mut self.main_search,
            SearchType::Qsearch => &mut self.qsearch,
        }
    }
    pub fn aggregate(&mut self, other: &IDStatistics) {
        self.lmr_first_retry += other.lmr_first_retry;
        self.lmr_second_retry += other.lmr_second_retry;
        self.in_check += other.in_check;
        self.main_search.aggregate(&other.main_search);
        self.qsearch.aggregate(&other.qsearch);
    }
}

#[cfg(feature = "statistics")]
#[derive(Debug, Default, Clone)]
pub struct Statistics {
    iterations: ArrayVec<IDStatistics, MAX_ID_ITERATIONS>,
    aw: NodeTypeCtr,
}

#[cfg(not(feature = "statistics"))]
#[derive(Debug, Default, Clone)]
pub struct Statistics {
    nodes: u64,
    aw: NodeTypeCtr,
    id_iterations: usize,
}

#[derive(Debug, Eq, PartialEq)]
pub enum SearchType {
    MainSearch,
    Qsearch,
}

/// Functions that exist even if there are no statistics being collected,
/// either because they're cheap or because they're necessary
impl Statistics {
    pub fn aw_fail_high(&mut self) {
        self.aw.fail_highs += 1;
    }
    pub fn aw_fail_low(&mut self) {
        self.aw.fail_highs += 1;
    }
    pub fn aw_exact(&mut self) {
        self.aw.exact += 1;
        self.next_id_iteration();
    }
}

#[cfg(feature = "statistics")]
impl Statistics {
    fn cur(&mut self) -> &mut IDStatistics {
        self.iterations.last_mut().unwrap()
    }
    pub fn search(&mut self, search_type: SearchType) -> &mut SearchTypeStatistics {
        self.cur().search(search_type)
    }
    pub fn next_id_iteration(&mut self) {
        self.iterations.push(IDStatistics::default());
    }

    pub fn count_complete_node(
        &mut self,
        search_type: SearchType,
        node_type: NodeType,
        depth: isize,
        ply: usize,
        visited_children: usize,
    ) {
        let search = self.search(search_type);
        search.node_ctr.increment(node_type);
        search.counters[PlySum as usize] += ply as u64;
        search.counters[DepthSum as usize] += depth as u64;
        if visited_children > 1 {
            search.counters[NoTTMoveCutoff as usize] += 1;
        }
    }

    /// This counts all nodes (except the root node), unlike `count_complete_node`,
    /// which only counts nodes where the moves loop has completed, so it doesn't count TT cutoffs.
    pub fn count_move(&mut self, search_type: SearchType) {
        self.search(search_type).counters[Nodes as usize] += 1;
    }

    pub fn in_check(&mut self) {
        self.cur().in_check += 1;
    }

    pub fn tt_miss(&mut self, search_type: SearchType) {
        self.search(search_type).counters[TTMisses as usize] += 1;
    }

    pub fn tt_cutoff(&mut self, search_type: SearchType, node_type: NodeType) {
        self.search(search_type).tt_cutoffs.increment(node_type);
    }

    pub fn lmr_first_retry(&mut self) {
        self.cur().lmr_first_retry += 1;
    }

    pub fn lmr_second_retry(&mut self) {
        self.cur().lmr_second_retry += 1;
    }

    pub fn aggregate_iterations(&self) -> IDStatistics {
        let mut res = IDStatistics::default();
        for s in self.iterations.iter() {
            res.aggregate(s);
        }
        res
    }
    pub fn depth(&self) -> usize {
        self.iterations.len()
    }
}

#[cfg(not(feature = "statistics"))]
impl Statistics {
    pub fn next_id_iteration(&mut self) {
        self.id_iterations += 1;
    }

    pub fn count_complete_node(
        &mut self,
        _search_type: SearchType,
        _node_type: NodeType,
        _depth: isize,
        _ply: usize,
        _children_visited: usize,
    ) {
    }

    pub fn count_move(&mut self, search_type: SearchType) {
        if search_type != Qsearch {
            self.nodes += 1;
        }
    }

    pub fn in_check(&mut self) {}

    pub fn tt_miss(&mut self, _search_type: SearchType) {}

    pub fn tt_cutoff(&mut self, _search_type: SearchType, _node_type: NodeType) {}

    pub fn lmr_first_retry(&mut self) {}

    pub fn lmr_second_retry(&mut self) {}

    pub fn aggregate_iterations(&self) -> IDStatistics {
        IDStatistics::default()
    }

    pub fn depth(&self) -> usize {
        self.id_iterations
    }
}

#[derive(Copy, Clone)]
enum Mode {
    Percentage,
    Average,
}

pub struct Summary {
    nodes: u64,
    node_statistics: IDStatistics,
    aw: NodeTypeCtr,
    depth: u64,
}

impl Summary {
    pub fn new(statistics: &Statistics) -> Self {
        let node_statistics = statistics.aggregate_iterations();
        let nodes = node_statistics.main_search.counters[Nodes as usize]
            + node_statistics.qsearch.counters[Nodes as usize];
        Self {
            nodes,
            node_statistics,
            aw: statistics.aw,
            depth: statistics.depth() as u64,
        }
    }

    fn format_ctr(val: u64, total: u64, mode: Mode) -> String {
        match mode {
            Percentage => {
                format!(
                    "{val} ({:.1}%)",
                    val as f64 / total as f64 * 100.0 /*multiply by 100.0 last for better precision*/
                )
            }
            Average => {
                format!("avg {:.1}", val as f64 / total as f64)
            }
        }
    }
}

impl Display for Summary {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "depth {depth}, total nodes {nodes}, in check {in_check}",
            depth = self.depth,
            nodes = self.nodes,
            in_check = self.node_statistics.in_check
        )
        .unwrap();
        let mut write_node_ctr =
            |ctr: NodeTypeCtr, total: u64, name: &str, f: &mut Formatter<'_>| {
                write!(
                    f,
                    ",  {name} fail low: {0}, exact: {1}, fail high: {2}",
                    Self::format_ctr(ctr.fail_lows, total, Percentage),
                    Self::format_ctr(ctr.exact, total, Percentage),
                    Self::format_ctr(ctr.fail_highs, total, Percentage)
                )
                .unwrap();
            };
        write_node_ctr(self.aw, self.aw.sum(), "asp windows", f);
        let main_nodes = self.node_statistics.main_search.counters[Nodes as usize];
        let qsearch_nodes = self.node_statistics.qsearch.counters[Nodes as usize];
        for i in 0..NumCounters as usize {
            let ctr = SearchCounter::iter().get(i).unwrap();
            let mode = match ctr {
                DepthSum | PlySum => Average,
                _ => Percentage,
            };
            let main = self.node_statistics.main_search.counters[i];
            let qsearch = self.node_statistics.qsearch.counters[i];
            write!(
                f,
                ",  {ctr}: {0} and {1}",
                Self::format_ctr(main, main_nodes, mode),
                Self::format_ctr(qsearch, qsearch_nodes, mode),
            )
            .unwrap();
        }
        write_node_ctr(
            self.node_statistics.main_search.node_ctr,
            main_nodes,
            "main search",
            f,
        );
        write_node_ctr(
            self.node_statistics.qsearch.node_ctr,
            qsearch_nodes,
            "quiescent search",
            f,
        );
        Ok(())
    }
}
