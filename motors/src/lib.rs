use std::borrow::BorrowMut;
use std::ops::Deref;

use dyn_clone::clone_box;
use rand::rngs::StdRng;

use gears::cli::{ArgIter, Game};
#[cfg(feature = "chess")]
use gears::games::chess::Chessboard;
#[cfg(feature = "mnk")]
use gears::games::mnk::MNKBoard;
use gears::games::{Board, OutputList};
use gears::general::common::Description::WithDescription;
use gears::general::common::{select_name_dyn, Res};
use gears::output::normal_outputs;
use gears::search::DepthLimit;
use gears::{create_selected_output_builders, AbstractRun, AnyRunnable, OutputArgs};

use crate::cli::Mode::Bench;
use crate::cli::{parse_cli, EngineOpts, Mode};
use crate::eval::chess::hce::HandCraftedEval;
#[cfg(feature = "chess")]
use crate::eval::chess::pst_only::PstOnlyEval;
#[cfg(feature = "mnk")]
use crate::eval::mnk::simple_mnk_eval::SimpleMnkEval;
#[cfg(feature = "caps")]
use crate::search::chess::caps::Caps;
#[cfg(feature = "generic_negamax")]
use crate::search::generic::generic_negamax::GenericNegamax;
#[cfg(feature = "random_mover")]
use crate::search::generic::random_mover::RandomMover;
use crate::search::multithreading::{EngineWrapper, SearchSender};
use crate::search::{
    run_bench, run_bench_with_depth, AbstractEngineBuilder, Benchable, EngineBuilder, EngineList,
    EngineWrapperBuilder,
};
use crate::ugi_engine::EngineUGI;

pub mod cli;
pub mod eval;
pub mod search;
mod ugi_engine;

#[derive(Debug)]
struct BenchRun<B: Board> {
    engine: Box<dyn Benchable<B>>,
    depth: Option<DepthLimit>,
}

impl<B: Board> BenchRun<B> {
    pub fn create(options: EngineOpts, all_engines: EngineList<B>) -> Res<Self> {
        let Bench(depth) = options.mode else { panic!() };
        let engine = create_engine_bench_from_str(&options.engine, &all_engines)?;
        Ok(Self { engine, depth })
    }
}

impl<B: Board> AbstractRun for BenchRun<B> {
    fn run(&mut self) {
        let engine = self.engine.as_mut();
        let res = match self.depth {
            None => run_bench(engine),
            Some(depth) => run_bench_with_depth(engine, depth),
        };
        println!("{res}");
    }
}

pub fn create_engine_from_str_impl<B: Board>(
    name: &str,
    engines: &EngineList<B>,
) -> Res<Box<dyn AbstractEngineBuilder<B>>> {
    if name == "default" {
        return Ok(clone_box(engines.last().unwrap().deref()));
    }
    Ok(clone_box(select_name_dyn(
        name,
        engines,
        "engine",
        B::game_name(),
        WithDescription,
    )?))
}

pub fn create_engine_from_str<B: Board>(
    name: &str,
    engines: &EngineList<B>,
    search_sender: SearchSender<B>,
) -> Res<EngineWrapper<B>> {
    let builder = create_engine_from_str_impl(name, engines)?;
    let builder = EngineWrapperBuilder::new(builder, search_sender);
    Ok(builder.build())
}

pub fn create_engine_bench_from_str<B: Board>(
    name: &str,
    engines: &EngineList<B>,
) -> Res<Box<dyn Benchable<B>>> {
    let builder = create_engine_from_str_impl(name, engines)?;
    Ok(builder.build_for_bench())
}

pub fn create_match_for_game<B: Board>(
    mut args: EngineOpts,
    engines: EngineList<B>,
    outputs: OutputList<B>,
) -> Res<AnyRunnable> {
    match args.mode {
        Bench(_) => Ok(Box::new(BenchRun::create(args, engines)?)),
        Mode::Engine => {
            if args.debug {
                args.outputs.push(OutputArgs::new("logger".to_string()));
            }
            Ok(Box::new(EngineUGI::create(
                args.clone(),
                create_selected_output_builders(&args.outputs, &outputs)?,
                outputs,
                engines,
            )?))
        }
    }
}

#[cfg(feature = "chess")]
fn list_chess_outputs() -> OutputList<Chessboard> {
    normal_outputs::<Chessboard>()
}

#[cfg(feature = "mnk")]
fn list_mnk_outputs() -> OutputList<MNKBoard> {
    normal_outputs::<MNKBoard>()
}

pub fn generic_engines<B: Board>() -> EngineList<B> {
    vec![
        #[cfg(feature = "random_mover")]
        Box::new(EngineBuilder::<B, RandomMover<B, StdRng>>::default()),
        // Does not contain GenericNegamax because that takes the eval function as generic argument, which
        // depends on the game (TODO: include with a game-independent eval?)
        // #[cfg(feature = "generic_negamax")]
        // Box::new(EngineBuilder::<B, GenericNegamax<B, RandEval>>::new()),
    ]
}

#[cfg(feature = "chess")]
pub fn list_chess_engines() -> EngineList<Chessboard> {
    let mut res = generic_engines();
    #[cfg(feature = "generic_negamax")]
    res.push(Box::new(EngineBuilder::<
        Chessboard,
        GenericNegamax<Chessboard, PstOnlyEval>,
    >::new()));
    #[cfg(feature = "caps")]
    res.push(Box::new(
        EngineBuilder::<Chessboard, Caps<HandCraftedEval>>::new(),
    ));
    res
}

#[cfg(feature = "mnk")]
pub fn list_mnk_engine() -> EngineList<MNKBoard> {
    let mut res = generic_engines();
    #[cfg(feature = "generic_negamax")]
    res.push(Box::new(EngineBuilder::<
        MNKBoard,
        GenericNegamax<MNKBoard, SimpleMnkEval>,
    >::new()));
    res
}

pub fn create_match(args: EngineOpts) -> Res<AnyRunnable> {
    match args.game {
        #[cfg(feature = "chess")]
        Game::Chess => create_match_for_game(args, list_chess_engines(), list_chess_outputs()),
        #[cfg(feature = "mnk")]
        Game::Mnk => create_match_for_game(args, list_mnk_engine(), list_mnk_outputs()),
    }
}

pub fn run_program_with_args(args: ArgIter) -> Res<()> {
    let args =
        parse_cli(args).map_err(|err| format!("Failed to parse command line arguments: {err}"))?;
    let mode = args.mode;
    let mut the_match =
        create_match(args).map_err(|err| format!("Couldn't start the {mode}: {err}"))?;
    the_match.run();
    Ok(())
}

pub fn run_program() -> Res<()> {
    let mut args = std::env::args().peekable();
    args.next(); // remove the program name
    run_program_with_args(args)
}
