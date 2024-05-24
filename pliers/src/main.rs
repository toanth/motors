use colored::Colorize;
use gears::games::chess::Chessboard;
use pliers::eval::chess::caps_hce_eval::CapsHceEval;
use pliers::gd::Adam;
use pliers::{debug_eval_on_lucena, optimize, optimize_chess_eval};
use std::env::args;
use std::fs::read_dir;
use std::path::Path;
use std::process::exit;

fn main() {
    let mut files = args().skip(1).collect::<Vec<_>>();
    let default_path = "pliers/datasets/chess/";
    if files.is_empty() {
        let Ok(dir) = read_dir(default_path) else {
            eprintln!("No input files specified on the command line and couldn't open folder '{}', exiting", default_path.bold());
            exit(1);
        };
        for path in dir {
            if let Ok(file) = path {
                files.push(file.path().to_str().unwrap().to_string());
            }
        }
    }
    if let Err(err) = optimize_chess_eval::<CapsHceEval>(files.as_ref()) {
        eprintln!("{err}");
    }
}
