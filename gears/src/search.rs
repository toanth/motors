use std::fmt::{Display, Formatter};
use std::num::NonZeroU64;
use std::str::FromStr;
use std::time::{Duration, Instant};

use derive_more::{Add, AddAssign, SubAssign};

use crate::games::{Board, Move};
use crate::general::common::parse_fp_from_str;
use crate::score::Score;

pub const MAX_DEPTH: Depth = Depth(10_000);

#[derive(Eq, PartialEq, Debug, Default)]
pub struct SearchResult<B: Board> {
    pub chosen_move: B::Move,
    pub score: Option<Score>,
    pub ponder_move: Option<B::Move>,
}

impl<B: Board> SearchResult<B> {
    pub fn move_only(chosen_move: B::Move) -> Self {
        Self {
            chosen_move,
            ..Default::default()
        }
    }

    pub fn move_and_score(chosen_move: B::Move, score: Score) -> Self {
        Self {
            chosen_move,
            score: Some(score),
            ponder_move: None,
        }
    }

    pub fn new(chosen_move: B::Move, score: Score, ponder_move: Option<B::Move>) -> Self {
        Self {
            chosen_move,
            score: Some(score),
            ponder_move,
        }
    }

    pub fn new_from_pv(score: Score, pv: &[B::Move]) -> Self {
        // the pv may be empty if search is called in a position where the game is over
        Self::new(
            pv.get(0).copied().unwrap_or_default(),
            score,
            pv.get(1).copied(),
        )
    }

    pub fn ponder_move(&self) -> Option<B::Move> {
        self.ponder_move
    }
}

#[derive(Debug)]
pub struct SearchInfo<B: Board> {
    pub best_move: B::Move,
    pub depth: Depth,
    pub seldepth: Option<usize>,
    pub time: Duration,
    pub nodes: NodesLimit,
    pub pv_num: usize,
    pub pv: Vec<B::Move>,
    pub score: Score,
    pub hashfull: Option<usize>,
    pub additional: Option<String>,
}

impl<B: Board> Default for SearchInfo<B> {
    fn default() -> Self {
        Self {
            best_move: B::Move::default(),
            depth: Depth::default(),
            seldepth: None,
            time: Duration::default(),
            nodes: NodesLimit::MAX,
            pv_num: 1,
            pv: vec![],
            score: Score::default(),
            hashfull: None,
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

    /// This function is the default for the info callback function.
    pub fn ignore(self) {
        // do nothing.
    }

    pub fn to_search_result(&self) -> SearchResult<B> {
        SearchResult::move_and_score(self.best_move, self.score)
    }
}

impl<B: Board> Display for SearchInfo<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let score_str = if let Some(moves_until_over) = self.score.moves_until_game_won() {
            format!("mate {moves_until_over}")
        } else {
            format!("cp {0}", self.score.0) // TODO: WDL normalization
        };

        write!(f,
               "info depth {depth}{seldepth} multipv {multipv} score {score_str} time {time} nodes {nodes} nps {nps}{hashfull} pv {pv}{string}",
               depth = self.depth.get(), time = self.time.as_millis(), nodes = self.nodes.get(),
               seldepth = self.seldepth.map(|d| format!(" seldepth {d}")).unwrap_or_default(),
               multipv = self.pv_num + 1,
               nps = self.nps(),
               pv = self.pv.iter().map(|mv| mv.to_compact_text()).collect::<Vec<_>>().join(" "),
               hashfull = self.hashfull.map(|f| format!(" hashfull {f}")).unwrap_or_default(),
               string = self.additional.clone().map(|s| format!(" string {s}")).unwrap_or_default()
        )
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
        if self.remaining >= Duration::MAX / 2 {
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
    type Err = String;

    // assume that the start time and increment strings don't contain a `+`

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "infinite" || s == "∞" {
            return Ok(TimeControl::infinite());
        }
        // For now, don't support movestogo TODO: Add support eventually
        let mut parts = s.split('+');
        let start_time = parts.next().ok_or_else(|| "Empty TC".to_string())?;
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
        self.remaining >= Duration::MAX - Duration::from_secs(1000)
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
    Debug, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Add, AddAssign, SubAssign,
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

    pub const fn new(val: usize) -> Self {
        debug_assert!(val <= Self::MAX.get());
        Self(val)
    }
}

pub type NodesLimit = NonZeroU64;

// Don't derive Eq because that allows code like `limit == SearchLimit::infinite()`, which is bad because the remaining
// time of `limit` might be slightly less while still being considered infinite.
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
            mate: Depth(0),
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

    pub fn mate(depth: Depth) -> Self {
        Self {
            mate: depth,
            ..Self::infinite()
        }
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

    pub fn is_infinite(&self) -> bool {
        let inf = Self::infinite();
        self.tc.is_infinite()
            && self.mate == inf.mate
            && self.depth == inf.depth
            && self.nodes == inf.nodes
            && self.fixed_time >= inf.fixed_time - Duration::from_secs(1000)
    }
}
