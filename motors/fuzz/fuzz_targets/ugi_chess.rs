#![no_main]

use gears::cli::Game::Chess;
use gears::create_selected_output_builders;
use libfuzzer_sys::fuzz_target;
use motors::cli::EngineOpts;
use motors::ugi_engine::EngineUGI;
use motors::{list_chess_evals, list_chess_outputs, list_chess_searchers};

fuzz_target!(|str: &str| {
    let opts = EngineOpts::for_game(Chess, false);
    let outputs = list_chess_outputs();
    let mut ugi = EngineUGI::create(
        opts.clone(),
        create_selected_output_builders(&opts.outputs, &outputs).unwrap(),
        outputs,
        list_chess_searchers(),
        list_chess_evals(),
    )
    .unwrap();
    for line in str.split_whitespace() {
        let _ = ugi.parse_input(line.split_whitespace().peekable());
    }
});
