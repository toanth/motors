use gears::games::chess::Chessboard;
use pliers::eval::chess::caps_hce_eval::CapsHceEval;
use pliers::{debug_eval_on_lucena, run};

type Engine = CapsHceEval;

fn main() {
    debug_eval_on_lucena::<Engine>();
    run::<Chessboard, Engine>();
}
