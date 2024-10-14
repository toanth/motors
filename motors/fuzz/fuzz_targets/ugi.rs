#![no_main]

use gears::cli::Game;
use gears::rand::prelude::IteratorRandom;
use gears::rand::rngs::StdRng;
use gears::rand::SeedableRng;
use gears::strum::IntoEnumIterator;
use libfuzzer_sys::fuzz_target;
use motors::create_match;
use motors::io::cli::EngineOpts;

fuzz_target!(|str: &str| {
    let str = "g\n\n";
    let mut rng = StdRng::seed_from_u64(str.len() as u64);
    let game = Game::iter().choose(&mut rng).unwrap();
    eprintln!("Game: {game}");
    let opts = EngineOpts::for_game(game, false);
    let mut ugi = create_match(opts).unwrap();
    for line in str.lines() {
        let _ = ugi.handle_input(line);
    }
    ugi.quit().unwrap();
});
