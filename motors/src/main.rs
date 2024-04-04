use std::process::abort;

use motors::run_program;

fn main() {
    if let Err(err) = run_program() {
        eprintln!("{err}");
        abort();
    }
}
