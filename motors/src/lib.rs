use std::ops::Deref;

use dyn_clone::clone_box;
use rand::rngs::StdRng;

use gears::cli::{ArgIter, Game};
use gears::games::ataxx::AtaxxBoard;
#[cfg(feature = "chess")]
use gears::games::chess::Chessboard;
#[cfg(feature = "mnk")]
use gears::games::mnk::MNKBoard;
use gears::games::{Board, OutputList};
use gears::general::common::Description::WithDescription;
use gears::general::common::{select_name_dyn, Res};
use gears::output::normal_outputs;
use gears::search::Depth;
use gears::Quitting::QuitMatch;
use gears::{create_selected_output_builders, AbstractRun, AnyRunnable, OutputArgs, Quitting};

use crate::cli::Mode::Bench;
use crate::cli::{parse_cli, EngineOpts, Mode};
use crate::eval::chess::lite::LiTEval;
use crate::eval::chess::material_only::MaterialOnlyEval;
#[cfg(feature = "chess")]
use crate::eval::chess::piston::PistonEval;
#[cfg(feature = "mnk")]
use crate::eval::mnk::simple_mnk_eval::SimpleMnkEval;
use crate::eval::rand_eval::RandEval;
#[cfg(feature = "caps")]
use crate::search::chess::caps::Caps;
#[cfg(feature = "generic_negamax")]
use crate::search::generic::gaps::Gaps;
#[cfg(feature = "random_mover")]
use crate::search::generic::random_mover::RandomMover;
use crate::search::multithreading::{EngineWrapper, SearchSender};
use crate::search::{
    run_bench, run_bench_with_depth, AbstractEvalBuilder, AbstractSearcherBuilder, Benchable,
    EngineBuilder, EvalBuilder, EvalList, SearcherBuilder, SearcherList,
};
use crate::ugi_engine::EngineUGI;

pub mod cli;
pub mod eval;
pub mod search;
mod ugi_engine;

#[derive(Debug)]
struct BenchRun<B: Board> {
    engine: Box<dyn Benchable<B>>,
    depth: Option<Depth>,
}

impl<B: Board> BenchRun<B> {
    pub fn create(
        options: EngineOpts,
        all_searchers: SearcherList<B>,
        all_evals: EvalList<B>,
    ) -> Res<Self> {
        let Bench(depth) = options.mode else { panic!() };
        let engine = create_engine_bench_from_str(&options.engine, &all_searchers, &all_evals)?;
        Ok(Self { engine, depth })
    }
}

impl<B: Board> AbstractRun for BenchRun<B> {
    fn run(&mut self) -> Quitting {
        let engine = self.engine.as_mut();
        let res = match self.depth {
            None => run_bench(engine),
            Some(depth) => run_bench_with_depth(engine, depth),
        };
        println!("{res}");
        QuitMatch
    }
}

// TODO: A lot of this repetitiveness could be avoided with a macro

pub fn create_searcher_from_str<B: Board>(
    name: &str,
    searchers: &SearcherList<B>,
) -> Res<Box<dyn AbstractSearcherBuilder<B>>> {
    if name == "default" {
        return Ok(clone_box(searchers.last().unwrap().deref()));
    }
    Ok(clone_box(select_name_dyn(
        name,
        searchers,
        "searcher",
        &B::game_name(),
        WithDescription,
    )?))
}

pub fn create_eval_from_str<B: Board>(
    name: &str,
    evals: &EvalList<B>,
) -> Res<Box<dyn AbstractEvalBuilder<B>>> {
    if name == "default" {
        return Ok(clone_box(evals.last().unwrap().deref()));
    }
    Ok(clone_box(select_name_dyn(
        name,
        evals,
        "eval",
        &B::game_name(),
        WithDescription,
    )?))
}

pub fn create_engine_from_str<B: Board>(
    name: &str,
    searchers: &SearcherList<B>,
    evals: &EvalList<B>,
    search_sender: SearchSender<B>,
) -> Res<EngineWrapper<B>> {
    let (searcher, eval) = name.split_once('-').unwrap_or((name, "default"));

    let searcher_builder = create_searcher_from_str(searcher, searchers)?;
    let eval_builder = create_eval_from_str(eval, evals)?;
    let builder = EngineBuilder::new(searcher_builder, eval_builder, search_sender);
    Ok(builder.build_wrapper())
}

pub fn create_engine_bench_from_str<B: Board>(
    name: &str,
    searchers: &SearcherList<B>,
    evals: &EvalList<B>,
) -> Res<Box<dyn Benchable<B>>> {
    let (searcher, eval) = name.split_once('-').unwrap_or((name, "default"));

    let searcher_builder = create_searcher_from_str(searcher, searchers)?;
    let eval_builder = create_eval_from_str(eval, evals)?;
    Ok(searcher_builder.build_for_bench(eval_builder.as_ref()))
}

pub fn create_match_for_game<B: Board>(
    mut args: EngineOpts,
    searchers: SearcherList<B>,
    evals: EvalList<B>,
    outputs: OutputList<B>,
) -> Res<AnyRunnable> {
    match args.mode {
        Bench(_) => Ok(Box::new(BenchRun::create(args, searchers, evals)?)),
        Mode::Engine => {
            if args.debug {
                args.outputs.push(OutputArgs::new("logger".to_string()));
            }
            Ok(Box::new(EngineUGI::create(
                args.clone(),
                create_selected_output_builders(&args.outputs, &outputs)?,
                outputs,
                searchers,
                evals,
            )?))
        }
    }
}

#[cfg(feature = "chess")]
fn list_chess_outputs() -> OutputList<Chessboard> {
    normal_outputs::<Chessboard>()
}

#[cfg(feature = "ataxx")]
fn list_ataxx_outputs() -> OutputList<AtaxxBoard> {
    normal_outputs::<AtaxxBoard>()
}

#[cfg(feature = "mnk")]
fn list_mnk_outputs() -> OutputList<MNKBoard> {
    normal_outputs::<MNKBoard>()
}

pub fn generic_evals<B: Board>() -> EvalList<B> {
    vec![Box::new(EvalBuilder::<B, RandEval>::default())]
}

#[cfg(feature = "chess")]
pub fn list_chess_evals() -> EvalList<Chessboard> {
    let mut res = generic_evals::<Chessboard>();
    res.push(Box::new(
        EvalBuilder::<Chessboard, MaterialOnlyEval>::default(),
    ));
    res.push(Box::new(EvalBuilder::<Chessboard, PistonEval>::default()));
    res.push(Box::new(EvalBuilder::<Chessboard, LiTEval>::default()));
    res
}

#[cfg(feature = "ataxx")]
pub fn list_ataxx_evals() -> EvalList<AtaxxBoard> {
    generic_evals()
}

#[cfg(feature = "mnk")]
pub fn list_mnk_evals() -> EvalList<MNKBoard> {
    let mut res = generic_evals::<MNKBoard>();
    res.push(Box::new(EvalBuilder::<MNKBoard, SimpleMnkEval>::default()));
    res
}

pub fn generic_searchers<B: Board>() -> SearcherList<B> {
    vec![
        #[cfg(feature = "random_mover")]
        Box::new(SearcherBuilder::<B, RandomMover<B, StdRng>>::default()),
        #[cfg(feature = "generic_negamax")]
        Box::new(SearcherBuilder::<B, Gaps<B>>::default()),
    ]
}

/// Lists all user-selectable searchers that can play chess.
/// An engine is the combination of a searcher and an eval.
#[cfg(feature = "chess")]
pub fn list_chess_searchers() -> SearcherList<Chessboard> {
    let mut res = generic_searchers();
    // The last engine in this list is the default engine
    #[cfg(feature = "caps")]
    res.push(Box::new(SearcherBuilder::<Chessboard, Caps>::new()));
    res
}

#[cfg(feature = "ataxx")]
pub fn list_ataxx_searchers() -> SearcherList<AtaxxBoard> {
    generic_searchers()
}

#[cfg(feature = "mnk")]
pub fn list_mnk_searchers() -> SearcherList<MNKBoard> {
    generic_searchers()
}

pub fn create_match(args: EngineOpts) -> Res<AnyRunnable> {
    match args.game {
        #[cfg(feature = "chess")]
        Game::Chess => create_match_for_game(
            args,
            list_chess_searchers(),
            list_chess_evals(),
            list_chess_outputs(),
        ),
        #[cfg(feature = "ataxx")]
        Game::Ataxx => create_match_for_game(
            args,
            list_ataxx_searchers(),
            list_ataxx_evals(),
            list_ataxx_outputs(),
        ),
        #[cfg(feature = "mnk")]
        Game::Mnk => create_match_for_game(
            args,
            list_mnk_searchers(),
            list_mnk_evals(),
            list_mnk_outputs(),
        ),
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
