use crate::eval::Eval;
use gears::games::Color;
use gears::games::chess::pieces::ChessPieceType;
use gears::games::chess::{ChessColor, Chessboard};
use gears::general::board::{BitboardBoard, Board};
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

impl Eval<Chessboard> for MaterialOnlyEval {
    fn piece_scale(&self) -> ScoreT {
        5
    }
    fn eval(&mut self, pos: &Chessboard, _ply: usize, _engine: ChessColor) -> Score {
        let mut color = pos.active_player();
        let mut score = 0;
        for _ in 0..2 {
            for piece in ChessPieceType::non_king_pieces() {
                let num_pieces = pos.col_piece_bb(color, piece).count_ones() as ScoreT;
                score += num_pieces * MATERIAL_VALUE[piece as usize];
            }
            score = -score;
            color = color.other();
        }
        Score(score)
    }
}
