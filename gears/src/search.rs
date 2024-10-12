use anyhow::{anyhow, bail};
use std::fmt::{Display, Formatter};
use std::num::NonZeroU64;
use std::ops::Sub;
use std::str::FromStr;
use std::time::{Duration, Instant};

use crate::general::board::Board;
use derive_more::{Add, AddAssign, SubAssign};
use itertools::Itertools;

use crate::general::common::{parse_fp_from_str, Res};
use crate::score::Score;
use crate::search::MpvType::{MainOfMultiple, OnlyLine, SecondaryLine};

pub const MAX_DEPTH: Depth = Depth(10_000);

#[derive(Eq, PartialEq, Debug, Default, Copy, Clone)]
#[must_use]
pub struct SearchResult<B: Board> {
    pub chosen_move: B::Move,
    pub score: Option<Score>,
    // TODO: NodeType to represent UCI upper bound and lower bound scores
    pub ponder_move: Option<B::Move>,
    pub pos: B,
}

impl<B: Board> SearchResult<B> {
    pub fn move_only(chosen_move: B::Move, pos: B) -> Self {
        Self {
            chosen_move,
            pos,
            ..Default::default()
        }
    }

    pub fn move_and_score(chosen_move: B::Move, score: Score, pos: B) -> Self {
        Self::new(chosen_move, score, None, pos)
    }

    pub fn new(chosen_move: B::Move, score: Score, ponder_move: Option<B::Move>, pos: B) -> Self {
        debug_assert!(score.verify_valid().is_some());
        Self {
            chosen_move,
            score: Some(score),
            ponder_move,
            pos,
        }
    }

    pub fn new_from_pv(score: Score, pos: B, pv: &[B::Move]) -> Self {
        debug_assert!(score.verify_valid().is_some());
        // the pv may be empty if search is called in a position where the game is over
        Self::new(
            pv.first().copied().unwrap_or_default(),
            score,
            pv.get(1).copied(),
            pos,
        )
    }

    pub fn ponder_move(&self) -> Option<B::Move> {
        self.ponder_move
    }
}

impl<B: Board> Display for SearchResult<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(ponder) = self.ponder_move() {
            write!(f, "bestmove {} ponder {ponder}", self.chosen_move)
        } else {
            write!(f, "bestmove {}", self.chosen_move)
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MpvType {
    OnlyLine,
    MainOfMultiple,
    SecondaryLine,
}

#[derive(Debug)]
#[must_use]
pub struct SearchInfo<B: Board> {
    pub best_move_of_all_pvs: B::Move,
    pub depth: Depth,
    pub seldepth: Depth,
    pub time: Duration,
    pub nodes: NodesLimit,
    pub pv_num: usize,
    pub max_num_pvs: usize,
    pub pv: Vec<B::Move>,
    pub score: Score,
    pub hashfull: usize,
    pub pos: B,
    pub additional: Option<String>,
}

impl<B: Board> Default for SearchInfo<B> {
    fn default() -> Self {
        Self {
            best_move_of_all_pvs: B::Move::default(),
            depth: Depth::default(),
            seldepth: Depth::default(),
            time: Duration::default(),
            nodes: NodesLimit::MAX,
            pv_num: 1,
            max_num_pvs: 1,
            pv: vec![],
            score: Score::default(),
            hashfull: 0,
            pos: B::default(),
            additional: None,
        }
    }
}

impl<B: Board> SearchInfo<B> {
    pub fn nps(&self) -> usize {
        let micros = self.time.as_micros() as f64;
        if micros == 0.0 {
            0
        } else {
            ((self.nodes.get() as f64 * 1_000_000.0) / micros) as usize
        }
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

impl<B: Board> Display for SearchInfo<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,
               "info depth {depth} seldepth {seldepth} multipv {multipv} score {score} time {time} nodes {nodes} nps {nps} hashfull {hashfull} pv",
               depth = self.depth.get(),
               score = self.score,
               time = self.time.as_millis(),
               nodes = self.nodes.get(),
               seldepth = self.seldepth.0,
               multipv = self.pv_num + 1,
               nps = self.nps(),
               hashfull = self.hashfull,
        )?;
        for mov in &self.pv {
            write!(f, " {mov}")?;
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
            write!(
                f,
                "{0}ms + {1}ms",
                self.remaining.as_millis(),
                self.increment.as_millis()
            )
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
            increment = Duration::from_secs_f64(
                parse_fp_from_str::<f64>(inc_str.trim(), "the increment")?.max(0.0),
            );
        }
        Ok(TimeControl {
            remaining: start_time,
            increment,
            moves_to_go: None,
        })
    }
}

impl TimeControl {
    pub fn infinite() -> Self {
        TimeControl {
            remaining: Duration::MAX,
            increment: Duration::from_millis(0),
            moves_to_go: None,
        }
    }

    pub fn is_infinite(&self) -> bool {
        self.remaining >= Duration::MAX / 2
    }

    pub fn update(&mut self, elapsed: Duration) {
        if !self.is_infinite() {
            self.remaining += self.increment;
            self.remaining -= elapsed; // In this order to avoid computing negative intermediate values (which panics)
                                       // TODO: This probably still panics when remaining + increment - elapsed is less than 0 but greater than -time_margin.
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
            format!(
                "{min:02}:{s:02}.{ds:01}\n",
                min = t / 600,
                s = t % 600 / 10,
                ds = t % 10
            )
        }
    }
}

#[derive(
    Debug,
    Default,
    Copy,
    Clone,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Add,
    AddAssign,
    SubAssign,
    derive_more::Display,
)]
#[must_use]
pub struct Depth(usize);

impl Depth {
    pub const MIN: Self = Depth(0);
    pub const MAX: Self = MAX_DEPTH;

    pub const fn get(self) -> usize {
        self.0
    }

    pub const fn isize(self) -> isize {
        self.0 as isize
    }

    pub const fn new_unchecked(val: usize) -> Self {
        debug_assert!(val <= Self::MAX.get());
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

impl AddAssign<usize> for Depth {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl Add<usize> for Depth {
    type Output = Self;

    fn add(mut self, rhs: usize) -> Self::Output {
        self += rhs;
        self
    }
}

impl SubAssign<usize> for Depth {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs;
    }
}

impl Sub<usize> for Depth {
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
    pub depth: Depth,
    pub nodes: NodesLimit,
    pub mate: Depth,
}

impl Default for SearchLimit {
    fn default() -> Self {
        SearchLimit {
            tc: TimeControl::default(),
            fixed_time: Duration::MAX,
            depth: MAX_DEPTH,
            nodes: NodesLimit::new(u64::MAX).unwrap(),
            mate: Depth::new_unchecked(0), // only finding a mate in 0 would stop the search
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
        if self.mate != Depth::new_unchecked(0) {
            limits.push(format!("mate in {} plies", self.mate.get()));
        }
        if limits.len() == 1 {
            write!(f, "{}", limits[0])
        } else {
            write!(f, "[{}]", limits.iter().format(","))
        }
    }
}

impl SearchLimit {
    pub fn infinite() -> Self {
        Self::default()
    }

    pub fn tc(tc: TimeControl) -> Self {
        Self {
            tc,
            ..Self::infinite()
        }
    }

    pub fn per_move(fixed_time: Duration) -> Self {
        Self {
            fixed_time,
            ..Self::infinite()
        }
    }

    pub fn depth(depth: Depth) -> Self {
        Self {
            depth,
            ..Self::infinite()
        }
    }

    pub fn depth_(depth: usize) -> Self {
        Self::depth(Depth::new_unchecked(depth))
    }

    pub fn mate(depth: Depth) -> Self {
        Self {
            mate: depth,
            ..Self::infinite()
        }
    }

    pub fn mate_in_moves(num_moves: usize) -> Self {
        Self::mate(Depth::new_unchecked(num_moves * 2))
    }

    pub fn nodes(nodes: NodesLimit) -> Self {
        Self {
            nodes,
            ..Self::infinite()
        }
    }

    pub fn nodes_(nodes: u64) -> Self {
        Self::nodes(NodesLimit::new(nodes).unwrap())
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
    }

    pub fn is_only_time_based(&self) -> bool {
        let inf = Self::infinite();
        self.mate == inf.mate
            && self.depth == inf.depth
            && self.nodes == inf.nodes
            && (!self.tc.is_infinite() || !self.is_infinite_fixed_time())
    }
}

pub fn is_duration_infinite(duration: Duration) -> bool {
    duration >= Duration::MAX / 2
}
