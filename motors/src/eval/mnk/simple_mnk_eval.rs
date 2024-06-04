use strum::IntoEnumIterator;

use gears::games::mnk::{MNKBoard, MnkBitboard};
use gears::games::Board;
use gears::general::bitboards::{Bitboard, RawBitboard, RayDirections};
use gears::general::squares::GridSize;
use gears::search::Score;

use crate::eval::Eval;

#[derive(Debug, Default)]
pub struct SimpleMnkEval {}

fn eval_player(bb: MnkBitboard, size: GridSize) -> i32 {
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

impl Eval<MNKBoard> for SimpleMnkEval {
    fn eval(&self, pos: MNKBoard) -> Score {
        Score(
            eval_player(pos.active_player_bb(), pos.size())
                - eval_player(pos.inactive_player_bb(), pos.size()),
        )
    }
}
