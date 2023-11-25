use std::time::Duration;

use rand::rngs::ThreadRng;

use crate::eval::mnk::simple_mnk_eval::SimpleMnkEval;
use crate::games::mnk::{MNKBoard, MnkSettings};
use crate::games::{Height, Width};
use crate::general::common::parse_int_from_stdin;
use crate::play::GameOverReason::Adjudication;
use crate::play::{AbstractMatchManager, BuiltInMatch, GameResult, Player};
use crate::search::generic_negamax::GenericNegamax;
use crate::search::naive_slow_negamax::NaiveSlowNegamax;
use crate::search::random_mover::RandomMover;
use crate::search::{SearchLimit, TimeControl};
use crate::ui::pretty::PrettyUI;
use crate::ui::to_ui_handle;

// TODO: Remove this file / move the play_match.rs file into this

pub fn play() {
    play_mnk(); // the only game that's implemented for now
}

pub fn play_mnk() {
    let limit = SearchLimit::tc(TimeControl {
        remaining: Duration::new(20, 0),
        increment: Duration::new(0, 200_000_000),
        moves_to_go: 0,
    });

    println!("Please enter the height:");
    let height = parse_int_from_stdin().unwrap_or(3);
    println!("Please enter the width:");
    let width = parse_int_from_stdin().unwrap_or(3);
    println!("Please enter k:");
    let k = parse_int_from_stdin().unwrap_or(3);
    println!("Please enter strength (between 1 and 3):");
    let strength = parse_int_from_stdin().unwrap_or_else(|e| {
        println!("Error: {e}");
        3
    });
    let computer = match strength {
        1 => Player::new_for_searcher(RandomMover::<MNKBoard, ThreadRng>::default(), limit),
        2 => Player::new_for_searcher(NaiveSlowNegamax::default(), limit),
        _ => Player::new_for_searcher(GenericNegamax::<MNKBoard, SimpleMnkEval>::default(), limit),
    };
    println!("Playing against {0}", computer.searcher.name());
    let ui = to_ui_handle(PrettyUI::default());
    let mnk_settings = MnkSettings::try_new(Height(height), Width(width), k);
    if mnk_settings.is_none() {
        println!("Invalid m,n,k settings, please try again");
        return;
    }
    let mut the_match = BuiltInMatch::new(
        mnk_settings.unwrap(),
        Player::human(ui.clone()),
        computer,
        ui.clone(),
    );

    let res = the_match.run();
    if let Adjudication(x) = res.reason {
        println!("Adjudication: {x}");
    }
    match res.result {
        GameResult::P1Win => println!("Player 1 won!"),
        GameResult::P2Win => println!("Player 2 won!"),
        GameResult::Draw => println!("The game ended in a draw."),
        GameResult::Aborted => println!("The game was aborted."),
    }
}
