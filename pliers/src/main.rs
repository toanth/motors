use gears::games::chess::Chessboard;
#[cfg(feature = "caps")]
use pliers::eval::chess::caps_hce_eval::CapsHceEval;
use pliers::eval::chess::piston_eval::PistonEval;
use pliers::{debug_eval_on_lucena, run};

#[cfg(feature = "caps")]
type Engine = CapsHceEval;

#[cfg(not(feature = "caps"))]
type Engine = PistonEval;

fn main() {
    debug_eval_on_lucena::<Engine>();
    run::<Chessboard, Engine>();
}
