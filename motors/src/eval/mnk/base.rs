use crate::eval::Eval;
use gears::games::mnk::{Bitboard, Board, Color};
use gears::general::bitboards::BitboardTrait;
use gears::general::board::BitboardBoard;
use gears::general::common::StaticallyNamedEntity;
use gears::general::hq::BitReverseSliderGenerator;
use gears::score::{Score, ScoreT};
use std::fmt::Display;

/// `BasE` (Basic m,n,k Eval) is a m,n,k specific-eval. Currently very simple.
#[derive(Debug, Default, Clone)]
pub struct BasicMnkEval {}

fn eval_player(bb: Bitboard) -> ScoreT {
    let blockers = !bb;
    let generator = BitReverseSliderGenerator::new(blockers, None);
    let mut res = 0;
    for coords in bb.ones() {
        let run = generator.vertical_attacks(coords).count_ones();
        res += 1 << run;
        let run = generator.horizontal_attacks(coords).count_ones();
        res += 1 << run;
        let run = generator.diagonal_attacks(coords).count_ones();
        res += 1 << run;
        let run = generator.anti_diagonal_attacks(coords).count_ones();
        res += 1 << run;
    }
    res
}

impl StaticallyNamedEntity for BasicMnkEval {
    fn static_short_name() -> impl Display
    where
        Self: Sized,
    {
        "base"
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        "BasE: Basic m,n,k Eval".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "A very simple handcrafted eval for m,n,k games".to_string()
    }
}

impl Eval<Board> for BasicMnkEval {
    fn eval(&mut self, pos: &Board, _ply: usize, _engine: Color) -> Score {
        Score(eval_player(pos.active_player_bb()) - eval_player(pos.inactive_player_bb()))
    }
}
