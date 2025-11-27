use gears::games::chess::Board;
use pliers::eval::chess::lite::TuneLiTEval;
use pliers::{debug_eval_on_lucena, run};

type Eval = TuneLiTEval;

fn main() {
    debug_eval_on_lucena::<Eval>();
    run::<Board, Eval>();
}
