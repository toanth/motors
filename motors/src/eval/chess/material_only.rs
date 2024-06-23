use crate::eval::Eval;
use gears::games::chess::pieces::UncoloredChessPiece;
use gears::games::chess::Chessboard;
use gears::games::Board;
use gears::general::common::StaticallyNamedEntity;
use gears::score::{Score, ScoreT};

#[derive(Debug, Default)]
pub struct MaterialOnlyEval {}

const MATERIAL_VALUE: [ScoreT; 5] = [100, 300, 320, 500, 900];

impl StaticallyNamedEntity for MaterialOnlyEval {
    fn static_short_name() -> &'static str
    where
        Self: Sized,
    {
        "MateOnCE"
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        "Material Only Chess Eval".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "An evaluation function that does not consider anything apart from material, using the classical 1,3,3,5,9 piece values".to_string()
    }
}

impl Eval<Chessboard> for MaterialOnlyEval {
    fn eval(&mut self, pos: &Chessboard) -> Score {
        let mut color = pos.active_player();
        let mut score = 0;
        for _ in 0..2 {
            for piece in UncoloredChessPiece::non_king_pieces() {
                let num_pieces = pos.colored_piece_bb(color, piece).0.count_ones() as ScoreT;
                score += num_pieces * MATERIAL_VALUE[piece as usize];
            }
            score = -score;
            color = color.other();
        }
        Score(score)
    }
}
