use crate::general::board::Board;
use crate::general::common::{Res, parse_fp_from_str};
use crate::general::moves::Move;
use crate::score::Score;
use crate::search::MpvType::{MainOfMultiple, OnlyLine, SecondaryLine};
use NodeType::*;
use anyhow::{anyhow, bail};
use colored::{ColoredString, Colorize};
use derive_more::{Add, AddAssign, Sub, SubAssign};
use itertools::Itertools;
use std::fmt::{Display, Formatter};
use std::num::NonZeroU64;
use std::ops::{Add, AddAssign, Sub, SubAssign};
use std::str::FromStr;
use std::time::{Duration, Instant};
use strum_macros::FromRepr;

pub const MAX_DEPTH: DepthPly = DepthPly(10_000);

pub const MAX_BUDGET: Budget = Budget(1 << 20);

#[derive(Eq, PartialEq, Debug, Default, Copy, Clone)]
#[must_use]
pub struct SearchResult<B: Board> {
    pub chosen_move: B::Move,
    pub score: Score,
    // TODO: NodeType to represent UCI upper bound and lower bound scores
    pub ponder_move: Option<B::Move>,
    pub pos: B,
}

impl<B: Board> SearchResult<B> {
    pub fn move_only(chosen_move: B::Move, pos: B) -> Self {
        debug_assert!(chosen_move.is_null() || pos.is_move_legal(chosen_move));
        Self { chosen_move, pos, ..Default::default() }
    }

    pub fn move_and_score(chosen_move: B::Move, score: Score, pos: B) -> Self {
        Self::new(chosen_move, score, None, pos)
    }

    pub fn new(chosen_move: B::Move, score: Score, ponder_move: Option<B::Move>, pos: B) -> Self {
        debug_assert!(score.is_valid());
        debug_assert!(chosen_move.is_null() || pos.is_move_legal(chosen_move));
        #[cfg(debug_assertions)]
        if !chosen_move.is_null() {
            let new_pos = pos.clone().make_move(chosen_move).unwrap();
            if let Some(ponder) = ponder_move {
                debug_assert!(new_pos.is_move_legal(ponder));
            }
        }
        Self { chosen_move, score, ponder_move, pos }
    }

    pub fn new_from_pv(score: Score, pos: B, pv: &[B::Move]) -> Self {
        debug_assert!(score.is_valid());
        // the pv may be empty if search is called in a position where the game is over
        Self::new(pv.first().copied().unwrap_or_default(), score, pv.get(1).copied(), pos)
    }

    pub fn ponder_move(&self) -> Option<B::Move> {
        self.ponder_move
    }
}

impl<B: Board> Display for SearchResult<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "bestmove {}", self.chosen_move.compact_formatter(&self.pos))?;
        if let Some(ponder) = self.ponder_move() {
            // currently, this is unnecessary, but this might change in the future, and in any case
            // using a board to format a move that's not legal in that position would be *extremely* strange
            let new_pos = self.pos.clone().make_move(self.chosen_move).unwrap();
            write!(f, " ponder {}", ponder.compact_formatter(&new_pos))?;
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MpvType {
    OnlyLine,
    MainOfMultiple,
    SecondaryLine,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, FromRepr)]
#[repr(u8)]
#[must_use]
pub enum NodeType {
    /// Don't use 0 because that's used to represent the empty node type for the internal TT representation
    /// score is a lower bound >= beta, cut-node (the most common node type)
    FailHigh = 1,
    /// score known exactly in `(alpha, beta)`, PV node (very rare, but those are the most important nodes)
    Exact = 2,
    /// score between alpha and beta, PV node (important node!)
    FailLow = 3, // score is an upper bound <= alpha, all-node (relatively rare, but makes parent a cut-node)
}

impl Display for NodeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FailHigh => write!(f, "Lower Bound"),
            Exact => write!(f, "Exact"),
            FailLow => write!(f, "Upper Bound"),
        }
    }
}

impl NodeType {
    pub fn inverse(self) -> Self {
        // Could maybe try some bit twiddling tricks in case the compiler doesn't already do that
        match self {
            FailHigh => FailLow,
            Exact => Exact,
            FailLow => FailHigh,
        }
    }

    pub fn lower_bound() -> Self {
        FailHigh
    }

    pub fn upper_bound() -> Self {
        FailLow
    }

    pub fn comparison_str(&self, aligned: bool) -> ColoredString {
        match self {
            FailHigh => "≥".green(),
            Exact => (if aligned { " " } else { "" }).into(),
            FailLow => "≤".red(),
        }
    }
}

#[derive(Debug)]
#[must_use]
pub struct SearchInfo<'a, B: Board> {
    pub best_move_of_all_pvs: B::Move,
    pub iterations: DepthPly,
    pub budget: Budget,
    pub seldepth: DepthPly,
    pub time: Duration,
    pub nodes: NodesLimit,
    pub pv_num: usize,
    pub max_num_pvs: usize,
    pub pv: &'a [B::Move],
    pub score: Score,
    pub hashfull: usize,
    pub pos: B,
    pub bound: Option<NodeType>,
    pub num_threads: usize,
    pub additional: Option<String>,
    pub final_info: bool,
}

impl<B: Board> Default for SearchInfo<'_, B> {
    fn default() -> Self {
        Self {
            best_move_of_all_pvs: B::Move::default(),
            budget: Budget::default(),
            iterations: DepthPly::default(),
            seldepth: DepthPly::default(),
            time: Duration::default(),
            nodes: NodesLimit::MAX,
            pv_num: 1,
            max_num_pvs: 1,
            pv: &[],
            score: Score::default(),
            hashfull: 0,
            pos: B::default(),
            bound: None,
            num_threads: 1,
            additional: None,
            final_info: false,
        }
    }
}

impl<B: Board> SearchInfo<'_, B> {
    pub fn nps(&self) -> usize {
        let micros = self.time.as_micros() as f64;
        if micros == 0.0 { 0 } else { ((self.nodes.get() as f64 * 1_000_000.0) / micros) as usize }
    }

    pub fn mpv_type(&self) -> MpvType {
        if self.max_num_pvs == 1 {
            OnlyLine
        } else if self.pv_num == 0 {
            MainOfMultiple
        } else {
            SecondaryLine
        }
    }
}

impl<B: Board> Display for SearchInfo<'_, B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let bound = match self.bound.unwrap_or(Exact) {
            FailHigh => " lowerbound",
            Exact => "",
            FailLow => " upperbound",
        };
        // we write the number of iterations as the depth, because this is what `go depth` does and because it's what users would expect
        write!(
            f,
            "info depth {iterations} seldepth {seldepth} multipv {multipv} score {score}{bound} time {time} nodes {nodes} nps {nps} hashfull {hashfull} pv",
            iterations = self.iterations,
            score = self.score,
            time = self.time.as_millis(),
            nodes = self.nodes.get(),
            seldepth = self.seldepth.0,
            multipv = self.pv_num + 1,
            nps = self.nps(),
            hashfull = self.hashfull,
        )?;
        let mut pos = self.pos.clone();
        for &mov in self.pv {
            write!(f, " {}", mov.compact_formatter(&pos))?;
            pos = pos.make_move(mov).unwrap();
        }
        if let Some(ref additional) = self.additional {
            write!(f, " string {additional}")?;
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[must_use]
pub struct TimeControl {
    pub remaining: Duration,
    pub increment: Duration,
    pub moves_to_go: Option<usize>,
}

impl Default for TimeControl {
    fn default() -> Self {
        Self::infinite()
    }
}

impl Display for TimeControl {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_infinite() {
            write!(f, "infinite")
        } else {
            write!(f, "{0}ms + {1}ms", self.remaining.as_millis(), self.increment.as_millis())
        }
    }
}

impl FromStr for TimeControl {
    type Err = anyhow::Error;

    // assume that the start time and increment strings don't contain a `+`

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "infinite" || s == "∞" {
            return Ok(TimeControl::infinite());
        }
        // For now, don't support movestogo TODO: Add support
        let mut parts = s.split('+');
        let start_time = parts.next().ok_or_else(|| anyhow!("Empty TC"))?;
        let start_time = parse_fp_from_str::<f64>(start_time.trim(), "the start time")?.max(0.0);
        let start_time = Duration::from_secs_f64(start_time);
        let mut increment = Duration::default();
        if let Some(inc_str) = parts.next() {
            increment = Duration::from_secs_f64(parse_fp_from_str::<f64>(inc_str.trim(), "the increment")?.max(0.0));
        }
        Ok(TimeControl { remaining: start_time, increment, moves_to_go: None })
    }
}

impl TimeControl {
    pub fn infinite() -> Self {
        TimeControl {
            // allows doing arithmetic with infinite TCs without fear of overflows
            remaining: Duration::MAX / (1 << 16),
            increment: Duration::from_millis(0),
            moves_to_go: None,
        }
    }

    pub fn is_infinite(&self) -> bool {
        self.remaining > Duration::MAX / (1 << 31)
    }

    pub fn update(&mut self, elapsed: Duration) {
        if !self.is_infinite() {
            self.remaining += self.increment;
            self.remaining = self.remaining.saturating_sub(elapsed); // In this order to avoid computing negative intermediate values
            // TODO: Handle movestogo
        }
    }

    pub fn remaining(&self, start: Option<Instant>) -> Duration {
        if self.is_infinite() {
            self.remaining
        } else {
            let elapsed = start.map(|t| t.elapsed()).unwrap_or_default();
            self.remaining.saturating_sub(elapsed)
        }
    }

    pub fn remaining_to_string(&self, start: Option<Instant>) -> String {
        if self.is_infinite() {
            "infinite\n".to_string()
        } else {
            let t = self.remaining(start).as_millis() / 100;
            format!("{min:02}:{s:02}.{ds:01}\n", min = t / 600, s = t % 600 / 10, ds = t % 10)
        }
    }

    pub fn combine(self, other: Self) -> Self {
        Self {
            remaining: self.remaining.min(other.remaining),
            increment: self.increment.max(other.increment),
            moves_to_go: self.moves_to_go.or(other.moves_to_go),
        }
    }
}

#[derive(
    Debug, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Add, AddAssign, Sub, SubAssign, derive_more::Display,
)]
#[must_use]
/// Can be fractional, unlike [`DepthPly`].
pub struct Budget(usize);

impl Budget {
    pub const MIN: Self = Budget(0);
    pub const MAX: Self = MAX_BUDGET;

    pub const fn get(self) -> usize {
        self.0
    }

    pub const fn isize(self) -> isize {
        self.0 as isize
    }

    pub const fn new(val: usize) -> Self {
        assert!(val <= Self::MAX.get());
        Self(val)
    }

    pub fn try_new(val: isize) -> Res<Self> {
        if val < 0 {
            bail!("Budget must not be negative, but it is {val}")
        } else if val > Self::MAX.get() as isize {
            bail!("Budget must be at most {}, not {val}", Self::MAX.get())
        } else {
            Ok(Self(val as usize))
        }
    }
}

#[derive(
    Debug, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Add, AddAssign, SubAssign, derive_more::Display,
)]
#[must_use]
/// Depth expressed in ply. This is in contrast to [`Budget`], which can be fractional.
///
/// The UCI term "depth" is taken to mean iterations and usually expressed in [`DepthPly`].
/// Within a searcher, the current (possibly fractional) depth (if applicable) is represented as [`Budget`].
pub struct DepthPly(usize);

impl DepthPly {
    pub const MIN: Self = DepthPly(0);
    pub const MAX: Self = MAX_DEPTH;

    pub const fn get(self) -> usize {
        self.0
    }

    pub const fn isize(self) -> isize {
        self.0 as isize
    }

    pub const fn new(val: usize) -> Self {
        assert!(val <= Self::MAX.get());
        Self(val)
    }

    pub fn try_new(val: isize) -> Res<Self> {
        if val < 0 {
            bail!("Depth must not be negative, but it is {val}")
        } else if val > Self::MAX.get() as isize {
            bail!("Depth must be at most {}, not {val}", Self::MAX.get())
        } else {
            Ok(Self(val as usize))
        }
    }
}

impl AddAssign<usize> for DepthPly {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl Add<usize> for DepthPly {
    type Output = Self;

    fn add(mut self, rhs: usize) -> Self::Output {
        self += rhs;
        self
    }
}

impl SubAssign<usize> for DepthPly {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs;
    }
}

impl Sub<usize> for DepthPly {
    type Output = Self;

    fn sub(mut self, rhs: usize) -> Self::Output {
        self -= rhs;
        self
    }
}

pub type NodesLimit = NonZeroU64;

// Don't derive Eq because that allows code like `limit == SearchLimit::infinite()`, which is bad because the remaining
// time of `limit` might be slightly less while still being considered infinite.
// Note that nodes are counted per thread, and a node limit limits all threads to the set amount of nodes // TODO: Change?
#[derive(Copy, Clone, Debug)]
#[must_use]
pub struct SearchLimit {
    pub tc: TimeControl,
    pub fixed_time: Duration,
    pub byoyomi: Duration,
    pub depth: DepthPly,
    pub nodes: NodesLimit,
    pub soft_nodes: NodesLimit,
    pub mate: DepthPly,
    pub start_time: Instant,
}

impl Default for SearchLimit {
    fn default() -> Self {
        SearchLimit {
            tc: TimeControl::default(),
            fixed_time: Duration::MAX,
            byoyomi: Duration::ZERO,
            depth: MAX_DEPTH,
            nodes: NodesLimit::new(u64::MAX).unwrap(),
            soft_nodes: NodesLimit::new(u64::MAX).unwrap(),
            mate: DepthPly::new(0), // only finding a mate in 0 would stop the search
            start_time: Instant::now(),
        }
    }
}

impl Display for SearchLimit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_infinite() {
            return write!(f, "infinite");
        }
        let mut limits = vec![];
        if !self.tc.is_infinite() {
            limits.push(format!("{}", self.tc));
        }
        if !self.is_infinite_fixed_time() {
            limits.push(format!("{} ms fixed", self.fixed_time.as_millis()));
        }
        if self.depth != MAX_DEPTH {
            limits.push(format!("{} depth", self.depth.get()));
        }
        if self.nodes.get() != u64::MAX {
            limits.push(format!("{} nodes", self.nodes.get()));
        }
        if self.soft_nodes.get() != u64::MAX {
            limits.push(format!("{} soft nodes", self.soft_nodes.get()));
        }
        if self.mate != DepthPly::new(0) {
            limits.push(format!("mate in {} plies", self.mate.get()));
        }
        if limits.len() == 1 { write!(f, "{}", limits[0]) } else { write!(f, "[{}]", limits.iter().format(",")) }
    }
}

impl SearchLimit {
    pub fn infinite() -> Self {
        Self::default()
    }

    pub fn tc(tc: TimeControl) -> Self {
        Self { tc, ..Self::infinite() }
    }

    pub fn per_move(fixed_time: Duration) -> Self {
        Self { fixed_time, ..Self::infinite() }
    }

    pub fn depth(depth: DepthPly) -> Self {
        Self { depth, ..Self::infinite() }
    }

    pub fn depth_(depth: usize) -> Self {
        Self::depth(DepthPly::new(depth))
    }

    pub fn mate(depth: DepthPly) -> Self {
        Self { mate: depth, ..Self::infinite() }
    }

    pub fn mate_in_moves(num_moves: usize) -> Self {
        Self::mate(DepthPly::new(num_moves * 2))
    }

    pub fn nodes(nodes: NodesLimit) -> Self {
        Self { nodes, ..Self::infinite() }
    }

    pub fn nodes_(nodes: u64) -> Self {
        Self::nodes(NodesLimit::new(nodes).unwrap())
    }

    pub fn soft_nodes(soft_nodes: NodesLimit) -> Self {
        Self { soft_nodes, ..Self::infinite() }
    }

    pub fn soft_nodes_(soft_nodes: u64) -> Self {
        Self::soft_nodes(NodesLimit::new(soft_nodes).unwrap())
    }

    pub fn and(self, other: Self) -> Self {
        Self {
            tc: self.tc.combine(other.tc),
            fixed_time: self.fixed_time.min(other.fixed_time),
            byoyomi: self.byoyomi.max(other.byoyomi),
            depth: self.depth.min(other.depth),
            nodes: self.nodes.min(other.nodes),
            soft_nodes: self.soft_nodes.min(other.soft_nodes),
            mate: self.mate.max(other.mate),
            start_time: self.start_time.min(other.start_time),
        }
    }

    pub fn max_move_time(&self) -> Duration {
        self.fixed_time.min(self.tc.remaining)
    }

    pub fn is_infinite_fixed_time(&self) -> bool {
        is_duration_infinite(self.fixed_time)
    }

    pub fn is_infinite(&self) -> bool {
        let inf = Self::infinite();
        self.tc.is_infinite()
            && self.is_infinite_fixed_time()
            && self.mate == inf.mate
            && self.depth == inf.depth
            && self.nodes == inf.nodes
            && self.soft_nodes == inf.soft_nodes
    }

    pub fn is_only_time_based(&self) -> bool {
        let inf = Self::infinite();
        self.mate == inf.mate
            && self.depth == inf.depth
            && self.nodes == inf.nodes
            && self.soft_nodes == inf.soft_nodes
            && (!self.tc.is_infinite() || !self.is_infinite_fixed_time())
    }
}

pub fn is_duration_infinite(duration: Duration) -> bool {
    duration >= Duration::MAX / 2
}
