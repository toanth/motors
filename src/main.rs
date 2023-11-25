#![feature(iter_intersperse)]
#![feature(trait_upcasting)]
#![feature(iter_advance_by)]
#![feature(str_split_whitespace_remainder)]

/// Games to try:
/// m,n,k games, connect 4, ultimate tic-tac-toe, chess, attax, thud, go, konquest, poker, ...

/// This project is grouped in 4 broad modules:
/// - The actual games (implementing the `Board` trait)
/// - The `Searcher`s (most of them are `Engine`s), optionally making use of an `Eval`
/// - The `UI`s
/// - The `MatchManager` organizing all of this (either the built-in one or one using UCI/UGI)
use std::ops::DerefMut;

use clap::Parser;

use crate::games::chess::Chessboard;
use crate::games::Board;
use crate::play::ugi::UGI;
use crate::play::{
    game_list, AbstractMatchManager, AnyMatch, BuiltInMatch, CreatableMatchManager, MatchManager,
};

mod general;

mod games;

mod search;

mod play;

mod eval;
mod ui;

///A collection of games and engines.
/// Currently implemented games: Chess, m,n,k.
#[derive(Parser, Debug)]
#[command(author="ToTheAnd", version, about, long_about=None)]
struct CommandLineArgs {
    #[arg(long, short, default_value = "ugi")]
    mode: String,
    #[arg(long, short, default_value = "chess")]
    game: String,
}

fn start_initial_game<B: Board>(mode: &str, game: &str) -> AnyMatch {
    match mode {
        "ugi" | "uci" => select_game::<UGI<B>>(Box::new(UGI::default()), game),
        "play" => select_game::<BuiltInMatch<B>>(Box::new(BuiltInMatch::default()), game),
        m => panic!("Unrecognized game mode '{m}'"),
    }
}

fn select_game<M: CreatableMatchManager>(mut manager: Box<M>, name: &str) -> Box<M> {
    for (key, value) in game_list::<M>() {
        if name == key {
            manager.set_next_match(Some(value));
        }
    }
    return manager;
}

fn main() {
    let args = CommandLineArgs::parse();

    // let mut ugi = UGI::<MNKBoard>::new(Box::new(
    //     GenericNegamax::<MNKBoard, SimpleMnkEval>::default(),
    // ));
    let mut ugi: AnyMatch =
        start_initial_game::<Chessboard>(args.mode.as_str(), args.game.as_str());
    // Box::new(UGI::<Chessboard>::new(default_engine()));
    loop {
        let _res = ugi.run();
        let next = ugi.next_match();
        if next.is_none() {
            break;
        }
        ugi = next.unwrap();
    }
}
