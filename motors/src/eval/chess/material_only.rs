use crate::eval::Eval;
use gears::games::ColorTrait;
use gears::games::chess::pieces::PieceType;
use gears::games::chess::{Board, Color};
use gears::general::board::{BitboardBoard, BoardTrait};
use gears::general::common::StaticallyNamedEntity;
use gears::score::{Score, ScoreT};
use std::fmt::Display;

#[derive(Debug, Default, Clone)]
pub struct MaterialOnlyEval {}

const MATERIAL_VALUE: [ScoreT; 5] = [100, 300, 320, 500, 900];

impl StaticallyNamedEntity for MaterialOnlyEval {
    fn static_short_name() -> impl Display
    where
        Self: Sized,
    {
        "MateOnCE"
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        "MateOnCE: Material Only Chess Eval".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "A chess evaluation function that does not consider anything apart from material, using the classical 1,3,3,5,9 piece values".to_string()
    }
}

impl Eval<Board> for MaterialOnlyEval {
    fn piece_scale(&self) -> ScoreT {
        5
    }
    fn eval(&mut self, pos: &Board, _ply: usize, _engine: Color) -> Score {
        let mut color = pos.active_player();
        let mut score = 0;
        for _ in 0..2 {
            for piece in PieceType::non_king_pieces() {
                let num_pieces = pos.col_piece_bb(color, piece).count_ones() as ScoreT;
                score += num_pieces * MATERIAL_VALUE[piece as usize];
            }
            score = -score;
            color = color.other();
        }
        Score(score)
    }
}
