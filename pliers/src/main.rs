use colored::Colorize;
use gears::games::chess::Chessboard;
use pliers::eval::chess::caps_hce_eval::CapsHceEval;
use pliers::gd::Adam;
use pliers::{debug_eval_on_lucena, optimize, optimize_chess_eval, run};
use std::env::args;
use std::fs::read_dir;
use std::path::Path;
use std::process::exit;

fn main() {
    run::<Chessboard, CapsHceEval>();
}
