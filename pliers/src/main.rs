use gears::games::chess::Chessboard;
use pliers::eval::chess::lite::TuneLiTEval;
use pliers::{debug_eval_on_lucena, run};

type Eval = TuneLiTEval;

fn main() {
    debug_eval_on_lucena::<Eval>();
    run::<Chessboard, Eval>();

    // if let Err(err) = rescore_lichess_with_caps() {
    //     eprintln!("{}", err);
    // }
}
