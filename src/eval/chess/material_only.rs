use crate::eval::Eval;
use crate::games::chess::pieces::UncoloredChessPiece;
use crate::games::chess::Chessboard;
use crate::games::Board;
use crate::search::Score;

#[derive(Debug, Default)]
pub struct MaterialOnlyEval {}

const MATERIAL_VALUE: [i32; 5] = [100, 300, 320, 500, 900];

impl Eval<Chessboard> for MaterialOnlyEval {
    fn eval(&self, pos: Chessboard) -> Score {
        let mut color = pos.active_player();
        let mut score = 0;
        for i in 0..2 {
            for piece in UncoloredChessPiece::non_king_pieces() {
                let num_pieces = pos.colored_piece_bb(color, piece).0.count_ones() as i32;
                score += num_pieces * MATERIAL_VALUE[piece as usize];
            }
            score = -score;
            color = color.other();
        }
        Score(score)
    }
}
