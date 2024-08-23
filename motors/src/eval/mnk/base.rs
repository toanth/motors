use std::fmt::Display;
use strum::IntoEnumIterator;

use gears::games::mnk::{MNKBoard, MnkBitboard};
use gears::general::bitboards::{Bitboard, RawBitboard, RayDirections};
use gears::general::board::Board;
use gears::general::common::StaticallyNamedEntity;
use gears::general::squares::GridSize;
use gears::score::{Score, ScoreT};

use crate::eval::Eval;

/// BasE (Basic m,n,k Eval) is a m,n,k specific-eval. Currently very simple.
#[derive(Debug, Default, Clone)]
pub struct BasicMnkEval {}

fn eval_player(bb: MnkBitboard, size: GridSize) -> ScoreT {
    let blockers = !bb;
    let mut res = 0;
    for coords in bb.ones_for_size(size) {
        for dir in RayDirections::iter() {
            // TODO: Don't bitand with bb, bitand with !other_bb?
            let run = (MnkBitboard::slider_attacks(coords, blockers, dir) & bb)
                .to_primitive()
                .count_ones();
            res += 1 << run;
        }
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

impl Eval<MNKBoard> for BasicMnkEval {
    fn eval(&mut self, pos: &MNKBoard) -> Score {
        Score(
            eval_player(pos.active_player_bb(), pos.size())
                - eval_player(pos.inactive_player_bb(), pos.size()),
        )
    }
}