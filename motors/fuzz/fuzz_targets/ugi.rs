#![no_main]

use gears::cli::Game;
use libfuzzer_sys::fuzz_target;
use motors::io::cli::EngineOpts;
use motors::run_match;

fuzz_target!(|input: (Game, &str)| {
    eprintln!("Game: {}", input.0);
    let opts = EngineOpts::for_game(input.0, false);
    let mut ugi = run_match(opts).unwrap();
    for line in input.1.lines() {
        let _ = ugi.handle_input(line);
    }
    ugi.quit().unwrap();
});
