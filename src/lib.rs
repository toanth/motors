#![feature(iter_intersperse)]
#![feature(str_split_whitespace_remainder)]
#![feature(trait_upcasting)]

use std::fmt::{Display, Formatter};
/// Games to try:
/// m,n,k games, connect 4, ultimate tic-tac-toe, chess, attax, thud, go, konquest, poker, ...

/// This project is grouped in 4 broad modules:
/// - The actual games (implementing the `Board` trait)
/// - The `Searcher`s (most of them are `Engine`s), optionally making use of an `Eval`
/// - The `UI`s
/// - The `MatchManager` organizing all of this (either the built-in one or one using UCI/UGI)
use std::ops::DerefMut;
use std::process::exit;

use clap::{Parser, ValueEnum};

use crate::games::chess::Chessboard;
use crate::games::mnk::MNKBoard;
use crate::games::{Board, EngineList};
use crate::play::run_match::BuiltInMatch;
use crate::play::ugi::UGI;
use crate::play::{
    select_from_name, set_engine_from_str, set_graphics_from_str, AnyMatch, MatchManager,
};
use crate::search::run_bench;

pub mod general;

pub mod games;

pub mod search;

pub mod play;

pub mod eval;
pub mod ui;

///A collection of games and engines.
/// Currently implemented games: Chess, m,n,k.
#[derive(Parser, Debug)]
#[command(name="Motors", author="ToTheAnd", version, about, long_about=None)]
pub struct CommandLineArgs {
    #[arg(value_enum, default_value_t=Mode::Ugi)]
    mode: Mode,
    #[arg(value_enum, long, short, default_value_t=Game::Chess)]
    game: Game,
    #[arg(value_enum, default_value_t=Engine::Negamax)]
    engine: Engine,
    #[arg(short, long, default_value = "none")]
    ui: String,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ValueEnum, Default, Debug)]
pub enum Game {
    /// Normal Chess. Chess960 support WIP.
    #[default]
    Chess,
    /// m,n,k games are a generalization of Tic-Tac-Toe
    Mnk,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ValueEnum, Default, Debug)]
pub enum Mode {
    /// Run and report bench, then exit. Used by OpenBench.
    Bench,
    /// Start the UGI/UCI loop.
    #[default]
    Ugi,
    /// Interactively play on the command line. WIP.
    Play,
    // TODO: Add GUI mode where this program takes the role of cutechess and coordinates matches
}

/// An enum of all possible engine names. Note that not all engine / game combinations are valid.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ValueEnum, Default, Debug)]
pub enum Engine {
    Random,
    NaiveSlowNegamax,
    GenericNegamax,
    #[default]
    Negamax,
}

impl Display for Game {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Game::Chess => write!(f, "chess"),
            Game::Mnk => write!(f, "mnk"),
        }
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Bench => write!(f, "bench"),
            Mode::Ugi => write!(f, "ugi"),
            Mode::Play => write!(f, "play"),
        }
    }
}

impl Display for Engine {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Engine::Random => write!(f, "random"),
            Engine::NaiveSlowNegamax => write!(f, "naive_negamax"),
            Engine::GenericNegamax => write!(f, "generic_negamax"),
            Engine::Negamax => write!(f, "negamax"),
        }
    }
}

fn select_mode<B: Board>(mode: Mode, engine: Engine, ui: &str) -> AnyMatch {
    let mut manager: Box<dyn MatchManager<B>> = match mode {
        Mode::Ugi => Box::new(UGI::<B>::default()),
        Mode::Play => Box::new(BuiltInMatch::default()),
        Mode::Bench => {
            let engine = select_from_name(
                &engine.to_string(),
                B::EngineList::list_engines(),
                "engine",
                &B::game_name(),
            )
            .unwrap();
            let res = run_bench(engine("").deref_mut(), 5); // TODO: Allow giving an optional bench depth
            println!("{res}");
            exit(0);
        }
    };
    set_engine_from_str(manager.deref_mut(), &engine.to_string()).unwrap();
    set_graphics_from_str(manager.deref_mut(), ui).unwrap();
    manager
}

pub fn start_initial_game(mode: Mode, game: Game, engine: Engine, ui: &str) -> AnyMatch {
    // This match is necessary because the engine and match manager aren't type erased over the game.
    // An alternative would be to just create a Chessgame and use the set_next_match method to correctly
    // create the next match, ten cancel the original match. TODO: Refactor
    match game {
        Game::Chess => select_mode::<Chessboard>(mode, engine, ui),
        Game::Mnk => select_mode::<MNKBoard>(mode, engine, ui),
    }
}

pub fn run_games_loop(mut ugi: AnyMatch) {
    loop {
        let _res = ugi.run();
        let next = ugi.next_match();
        if next.is_none() {
            break;
        }
        ugi = next.unwrap();
    }
}

pub fn run_program() {
    let args = CommandLineArgs::parse();
    let ugi: AnyMatch = start_initial_game(args.mode, args.game, args.engine, &args.ui);
    run_games_loop(ugi);
}
