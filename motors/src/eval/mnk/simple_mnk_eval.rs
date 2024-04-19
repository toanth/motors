use strum::IntoEnumIterator;

use gears::games::{Board, GridSize, Size};
use gears::games::mnk::{MnkBitboard, MNKBoard};
use gears::general::bitboards::{Bitboard, RawBitboard, RayDirections};
use gears::general::common::pop_lsb128;
use gears::search::Score;

use crate::eval::Eval;

#[derive(Debug, Default)]
pub struct SimpleMnkEval {}

fn eval_player(bb: MnkBitboard, size: GridSize) -> i32 {
    let mut remaining = bb;
    let blockers = !bb;
    let mut res = 0;
    while remaining.0 != 0 {
        let idx = pop_lsb128(&mut remaining.0) as usize;

        for dir in RayDirections::iter() {
            // TODO: Don't bitand with bb, bitand with !other_bb?
            let run = (MnkBitboard::slider_attacks(size.to_coordinates(idx), blockers, dir) & bb)
                .to_primitive()
                .count_ones();
            res += 1 << run;
        }
    }
    res
}

impl Eval<MNKBoard> for SimpleMnkEval {
    fn eval(&self, pos: MNKBoard) -> Score {
        Score(
            eval_player(pos.active_player_bb(), pos.size())
                - eval_player(pos.inactive_player_bb(), pos.size()),
        )
    }
}
