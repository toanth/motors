use strum::IntoEnumIterator;

use crate::eval::Eval;
use crate::games::mnk::MNKBoard;
use crate::games::{Board, GridSize, Size};
use crate::general::bitboards::{Bitboard, ExtendedBitboard, SliderAttacks};
use crate::general::common::pop_lsb128;
use crate::search::Score;

#[derive(Debug, Default)]
pub struct SimpleMnkEval {}

fn eval_player(bb: ExtendedBitboard, size: GridSize) -> i32 {
    let mut remaining = bb;
    let blockers = !bb;
    let mut res = 0;
    while remaining.0 != 0 {
        let idx = pop_lsb128(&mut remaining.0) as usize;

        for dir in SliderAttacks::iter() {
            // TODO: Don't bitand with bb, bitand with !other_bb?
            let run =
                (ExtendedBitboard::slider_attacks(size.to_coordinates(idx), blockers, size, dir)
                    & bb)
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
