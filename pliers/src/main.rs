use gears::games::chess::Chessboard;
use pliers::eval::chess::caps_hce_eval::TuneLiTEval;
use pliers::{debug_eval_on_lucena, run};

type Eval = TuneLiTEval;

fn main() {
    debug_eval_on_lucena::<Eval>();
    run::<Chessboard, Eval>();
}
