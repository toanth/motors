use crate::eval::Eval;
use crate::games::chess::Chessboard;
use crate::search::Score;

#[derive(Default, Debug)]
struct PstOnlyEval {}

impl Eval<Chessboard> for PstOnlyEval {
    fn eval(&self, pos: Chessboard) -> Score {
        todo!()
    }
}
