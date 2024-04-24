use std::fmt::{Display, Formatter};
use std::num::NonZeroU64;
use std::ops::{Div, Mul};
use std::str::FromStr;
use std::time::{Duration, Instant};
use std::usize;

use derive_more::{Add, AddAssign, Neg, Sub, SubAssign};

use crate::games::{Board, Move};
use crate::general::common::parse_fp_from_str;
use crate::PlayerResult;

/// Anything related to search that is also used by `monitors`, and therefore doesn't belong in `motors`.

// TODO: Turn this into an enum that can also represent a win in n plies (and maybe a draw?)
#[derive(
    Default, Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Add, Sub, Neg, AddAssign, SubAssign,
)]
pub struct Score(pub i32);

impl Add<i32> for Score {
    type Output = Score;

    fn add(self, rhs: i32) -> Self::Output {
        Score(self.0 + rhs)
    }
}

impl Sub<i32> for Score {
    type Output = Score;

    fn sub(self, rhs: i32) -> Self::Output {
        Score(self.0 - rhs)
    }
}

impl Mul<i32> for Score {
    type Output = Score;

    fn mul(self, rhs: i32) -> Self::Output {
        Score(self.0 * rhs)
    }
}

impl Div<i32> for Score {
    type Output = Score;

    fn div(self, rhs: i32) -> Self::Output {
        Score(self.0 / rhs)
    }
}

impl Score {
    pub fn is_game_won_score(self) -> bool {
        self >= MIN_SCORE_WON
    }
    pub fn is_game_lost_score(self) -> bool {
        self <= MAX_SCORE_LOST
    }
    pub fn is_game_over_score(self) -> bool {
        self.is_game_won_score() || self.is_game_lost_score()
    }
    /// Returns a negative number of plies if the game is lost
    pub fn plies_until_game_won(self) -> Option<isize> {
        if self.is_game_won_score() {
            Some((SCORE_WON - self).0 as isize)
        } else if self.is_game_lost_score() {
            Some((SCORE_LOST - self).0 as isize)
        } else {
            None
        }
    }
    /// Returns a negative number if the game is lost
    pub fn moves_until_game_won(self) -> Option<isize> {
        self.plies_until_game_won()
            .map(|n| (n as f32 / 2f32).ceil() as isize)
    }

    pub fn plies_until_game_over(self) -> Option<isize> {
        self.plies_until_game_won().map(|x| x.abs())
    }

    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }
}

pub const SCORE_LOST: Score = Score(-31_000);
pub const SCORE_WON: Score = Score(31_000);
pub const SCORE_TIME_UP: Score = Score(SCORE_WON.0 + 1000);
// can't use + directly because derive_more's + isn't `const`
pub const MIN_SCORE_WON: Score = Score(SCORE_WON.0 - 1000);
pub const MAX_SCORE_LOST: Score = Score(SCORE_LOST.0 + 1000);
pub const MIN_NORMAL_SCORE: Score = Score(MAX_SCORE_LOST.0 + 1);
pub const MAX_NORMAL_SCORE: Score = Score(MIN_SCORE_WON.0 - 1);
pub const NO_SCORE_YET: Score = Score(SCORE_LOST.0 - 100);

pub const MAX_DEPTH: Depth = Depth(10_000);

pub fn game_result_to_score(res: PlayerResult, ply: usize) -> Score {
    match res {
        PlayerResult::Win => SCORE_WON - ply as i32,
        PlayerResult::Lose => SCORE_LOST + ply as i32,
        PlayerResult::Draw => Score(0),
    }
}

#[derive(Eq, PartialEq, Debug, Default)]
pub struct SearchResult<B: Board> {
    pub chosen_move: B::Move,
    pub score: Option<Score>,
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
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct SearchInfo<B: Board> {
    pub best_move: B::Move,
    pub depth: Depth,
    pub seldepth: Option<usize>,
    pub time: Duration,
    pub nodes: Nodes,
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
            nodes: Nodes::MAX,
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
               "info depth {depth}{seldepth} score {score_str} time {time} nodes {nodes} nps {nps} pv {pv}{hashfull}{string}",
               depth = self.depth.get(), time = self.time.as_millis(), nodes = self.nodes.get(),
               seldepth = self.seldepth.map(|d| format!(" seldepth {d}")).unwrap_or_default(),
               nps = self.nps(),
               pv = self.pv.iter().map(|mv| mv.to_compact_text()).collect::<Vec<_>>().join(" "),
               hashfull = self.hashfull.map(|f| format!(" hashfull {f}")).unwrap_or_default(),
               string = self.additional.clone().map(|s| format!(" string {s}")).unwrap_or_default()
        )
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
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
        if self.remaining == Duration::MAX {
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
        let start_time = parse_fp_from_str::<f64>(start_time.trim(), "the start time")?;
        let start_time = Duration::from_millis((1000.0 * start_time) as u64);
        let mut increment = Duration::default();
        if let Some(inc_str) = parts.next() {
            increment = Duration::from_millis(
                (1000.0 * parse_fp_from_str::<f64>(inc_str.trim(), "the increment")?) as u64,
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
        self.remaining == Duration::MAX
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

#[derive(Debug, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Depth(usize);

impl Depth {
    pub const MIN: Self = Depth(0);
    pub const MAX: Self = MAX_DEPTH;

    pub const fn get(self) -> usize {
        self.0
    }

    pub const fn new(val: usize) -> Self {
        assert!(val <= Self::MAX.get());
        Self(val)
    }
}

pub type Nodes = NonZeroU64;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SearchLimit {
    pub tc: TimeControl,
    pub fixed_time: Duration,
    pub depth: Depth,
    pub nodes: Nodes,
}

impl Default for SearchLimit {
    fn default() -> Self {
        SearchLimit {
            tc: TimeControl::default(),
            fixed_time: Duration::MAX,
            depth: MAX_DEPTH,
            nodes: Nodes::new(u64::MAX).unwrap(),
        }
    }
}

impl SearchLimit {
    pub fn infinite() -> Self {
        Self::default()
    }

    pub fn tc(tc: TimeControl) -> Self {
        let mut res = Self::infinite();
        res.tc = tc;
        res
    }

    pub fn per_move(time: Duration) -> Self {
        let mut res = Self::infinite();
        res.fixed_time = time;
        res
    }

    pub fn depth(depth: Depth) -> Self {
        let mut res = Self::infinite();
        res.depth = depth;
        res
    }

    pub fn nodes(nodes: Nodes) -> Self {
        let mut res = Self::infinite();
        res.nodes = nodes;
        res
    }

    pub fn nodes_(nodes: u64) -> Self {
        Self::nodes(Nodes::new(nodes).unwrap())
    }

    pub fn max_move_time(&self) -> Duration {
        self.fixed_time.min(self.tc.remaining)
    }
}
