#![deny(unused_results)]

use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use gears::dyn_clone::clone_box;
use gears::rand::rngs::StdRng;

use crate::Mode::{Bench, Perft};
#[cfg(feature = "ataxx")]
use crate::eval::ataxx::bate::Bate;
#[cfg(feature = "chess")]
use crate::eval::chess::lite::KingGambot;
#[cfg(feature = "chess")]
use crate::eval::chess::lite::LiTEval;
#[cfg(feature = "chess")]
use crate::eval::chess::material_only::MaterialOnlyEval;
#[cfg(feature = "chess")]
use crate::eval::chess::piston::PistonEval;
#[cfg(feature = "mnk")]
use crate::eval::mnk::base::BasicMnkEval;
use crate::eval::rand_eval::RandEval;
#[cfg(feature = "uttt")]
use crate::eval::uttt::lute::Lute;
use crate::io::EngineUGI;
use crate::io::cli::{EngineOpts, parse_cli};
use crate::io::ugi_output::UgiOutput;
#[cfg(feature = "caps")]
use crate::search::chess::caps::Caps;
#[cfg(feature = "gaps")]
use crate::search::generic::gaps::Gaps;
#[cfg(feature = "proof_number")]
use crate::search::generic::proof_number::ProofNumberSearcher;
use crate::search::generic::random_mover::RandomMover;
use crate::search::multithreading::EngineWrapper;
use crate::search::tt::TT;
use crate::search::{
    AbstractEvalBuilder, AbstractSearcherBuilder, Engine, EvalBuilder, EvalList, SearcherBuilder, SearcherList,
    run_bench_with,
};
use gears::Quitting::*;
use gears::cli::{ArgIter, Game};
use gears::games::OutputList;
#[cfg(feature = "ataxx")]
use gears::games::ataxx::AtaxxBoard;
#[cfg(feature = "chess")]
use gears::games::chess::Chessboard;
#[cfg(feature = "fairy")]
use gears::games::fairy::FairyBoard;
#[cfg(feature = "mnk")]
use gears::games::mnk::MNKBoard;
#[cfg(feature = "uttt")]
use gears::games::uttt::UtttBoard;
use gears::general::board::Strictness::Relaxed;
use gears::general::board::{Board, BoardHelpers};
use gears::general::common::Description::WithDescription;
use gears::general::common::anyhow::anyhow;
use gears::general::common::{Res, select_name_dyn};
use gears::general::perft::{perft, split_perft};
use gears::output::normal_outputs;
use gears::search::{DepthPly, SearchLimit};
use gears::ugi::load_ugi_pos_simple;
use gears::{AbstractRun, AnyRunnable, OutputArgs, Quitting, create_selected_output_builders};
use std::fmt::{Display, Formatter};

pub mod eval;
pub mod io;
pub mod search;

#[derive(Debug, Default, Copy, Clone)]
pub enum Mode {
    #[default]
    Engine,
    Bench(Option<DepthPly>, bool),
    Perft(Option<DepthPly>, bool),
}

impl Display for Mode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Engine => write!(f, "engine"),
            Bench(_, _) => write!(f, "bench"),
            Perft(_, false) => write!(f, "perft"),
            Perft(_, true) => write!(f, "splitperft"),
        }
    }
}

#[derive(Debug)]
struct BenchRun<B: Board> {
    engine: Box<dyn Engine<B>>,
    depth: Option<DepthPly>,
    with_nodes: bool,
}

impl<B: Board> BenchRun<B> {
    pub fn create(options: &EngineOpts, all_searchers: &SearcherList<B>, all_evals: &EvalList<B>) -> Res<Self> {
        let Bench(depth, with_nodes) = options.mode else { unreachable!() };
        let engine = create_engine_box_from_str(&options.engine, all_searchers, all_evals)?;
        Ok(Self { engine, depth, with_nodes })
    }
}

impl<B: Board> AbstractRun for BenchRun<B> {
    fn run(&mut self) -> Quitting {
        let engine = self.engine.as_mut();
        let nodes = if self.with_nodes { Some(SearchLimit::nodes(engine.default_bench_nodes())) } else { None };
        let depth = self.depth.unwrap_or(engine.default_bench_depth());
        let res = run_bench_with(engine, SearchLimit::depth(depth), nodes, &B::bench_positions(), None);
        println!("{res}");
        QuitProgram
    }
}

#[derive(Debug, Default)]
struct PerftRun<B: Board> {
    depth: Option<DepthPly>,
    split: bool,
    pos_name: Option<String>,
    phantom_data: PhantomData<B>,
}

impl<B: Board> PerftRun<B> {
    pub fn create(depth: Option<DepthPly>, split: bool, pos_name: Option<String>) -> Self {
        Self { depth, split, pos_name, ..Self::default() }
    }
}

impl<B: Board> AbstractRun for PerftRun<B> {
    fn run(&mut self) -> Quitting {
        let pos = if let Some(name) = &self.pos_name {
            match load_ugi_pos_simple(name, Relaxed, &B::default()) {
                Ok(pos) => pos,
                Err(e) => {
                    eprintln!("Couldn't parse position to run perft: {e}");
                    return QuitProgram;
                }
            }
        } else {
            B::default()
        };
        let depth = self.depth.unwrap_or(pos.default_perft_depth());
        if self.split {
            let res = split_perft(depth, pos, true);
            println!("{res}");
        } else {
            let res = perft(depth, pos, true);
            println!("{res}");
        }
        QuitProgram
    }
}

// TODO: A lot of this repetitiveness could be avoided with a macro

pub fn create_searcher_from_str<B: Board>(
    name: &str,
    searchers: &SearcherList<B>,
) -> Res<Box<dyn AbstractSearcherBuilder<B>>> {
    if name == "default" {
        let searcher = searchers.last().expect("No searcher -- check enabled cargo features");
        return Ok(clone_box(&**searcher));
    }
    Ok(clone_box(select_name_dyn(name, searchers, "searcher", &B::game_name(), WithDescription)?))
}

pub fn create_eval_from_str<B: Board>(name: &str, evals: &EvalList<B>) -> Res<Box<dyn AbstractEvalBuilder<B>>> {
    if name == "default" {
        return Ok(clone_box(&**evals.last().unwrap()));
    }
    Ok(clone_box(select_name_dyn(name, evals, "eval", &B::game_name(), WithDescription)?))
}

pub fn create_engine_from_str<B: Board>(
    name: &str,
    searchers: &SearcherList<B>,
    evals: &EvalList<B>,
    output: Arc<Mutex<UgiOutput<B>>>,
    tt: TT,
) -> Res<EngineWrapper<B>> {
    let (searcher, eval) = name.split_once('-').unwrap_or((name, "default"));

    let searcher_builder = create_searcher_from_str(searcher, searchers)?;
    let eval_builder = create_eval_from_str(eval, evals)?;
    Ok(EngineWrapper::new(tt, output, searcher_builder, eval_builder))
}

pub fn create_engine_box_from_str<B: Board>(
    name: &str,
    searchers: &SearcherList<B>,
    evals: &EvalList<B>,
) -> Res<Box<dyn Engine<B>>> {
    let (searcher, eval) = name.split_once('-').unwrap_or((name, "default"));

    let searcher_builder = create_searcher_from_str(searcher, searchers)?;
    let eval_builder = create_eval_from_str(eval, evals)?;
    Ok(searcher_builder.build(eval_builder.as_ref()))
}

pub fn create_match_for_game<B: Board>(
    mut args: EngineOpts,
    searchers: SearcherList<B>,
    evals: EvalList<B>,
    outputs: OutputList<B>,
) -> Res<AnyRunnable> {
    match args.mode {
        Bench(_, _) => Ok(Box::new(BenchRun::create(&args, &searchers, &evals)?)),
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
        Perft(depth, split) => Ok(Box::new(PerftRun::<B>::create(depth, split, args.pos_name.clone()))),
    }
}

#[cfg(feature = "chess")]
#[must_use]
pub fn list_chess_outputs() -> OutputList<Chessboard> {
    normal_outputs::<Chessboard>(true)
}

#[cfg(feature = "ataxx")]
#[must_use]
pub fn list_ataxx_outputs() -> OutputList<AtaxxBoard> {
    normal_outputs::<AtaxxBoard>(true)
}

#[cfg(feature = "uttt")]
#[must_use]
pub fn list_uttt_outputs() -> OutputList<UtttBoard> {
    normal_outputs::<UtttBoard>(true)
}

#[cfg(feature = "mnk")]
#[must_use]
pub fn list_mnk_outputs() -> OutputList<MNKBoard> {
    normal_outputs::<MNKBoard>(true)
}

#[cfg(feature = "fairy")]
#[must_use]
pub fn list_fairy_outputs() -> OutputList<FairyBoard> {
    normal_outputs::<FairyBoard>(true)
}

#[must_use]
pub fn generic_evals<B: Board>() -> EvalList<B> {
    vec![Box::new(EvalBuilder::<B, RandEval>::default())]
}

#[cfg(feature = "chess")]
#[must_use]
pub fn list_chess_evals() -> EvalList<Chessboard> {
    let mut res = generic_evals::<Chessboard>();
    res.push(Box::new(EvalBuilder::<Chessboard, MaterialOnlyEval>::default()));
    res.push(Box::new(EvalBuilder::<Chessboard, PistonEval>::default()));
    res.push(Box::new(EvalBuilder::<Chessboard, KingGambot>::default()));
    res.push(Box::new(EvalBuilder::<Chessboard, LiTEval>::default()));
    res
}

#[cfg(feature = "ataxx")]
#[must_use]
pub fn list_ataxx_evals() -> EvalList<AtaxxBoard> {
    let mut res = generic_evals();
    res.push(Box::new(EvalBuilder::<AtaxxBoard, Bate>::default()));
    res
}

#[cfg(feature = "uttt")]
#[must_use]
pub fn list_uttt_evals() -> EvalList<UtttBoard> {
    let mut res = generic_evals();
    res.push(Box::new(EvalBuilder::<UtttBoard, Lute>::default()));
    res
}

#[cfg(feature = "mnk")]
#[must_use]
pub fn list_mnk_evals() -> EvalList<MNKBoard> {
    let mut res = generic_evals::<MNKBoard>();
    res.push(Box::new(EvalBuilder::<MNKBoard, BasicMnkEval>::default()));
    res
}

#[cfg(feature = "fairy")]
#[must_use]
pub fn list_fairy_evals() -> EvalList<FairyBoard> {
    generic_evals::<FairyBoard>()
    // TODO: Add special eval functions
}

#[must_use]
pub fn generic_searchers<B: Board>() -> SearcherList<B> {
    vec![
        Box::new(SearcherBuilder::<B, RandomMover<B, StdRng>>::default()),
        #[cfg(feature = "proof_number")]
        Box::new(SearcherBuilder::<B, ProofNumberSearcher<B>>::default()),
        #[cfg(feature = "gaps")]
        Box::new(SearcherBuilder::<B, Gaps<B>>::default()),
    ]
}

/// Lists all user-selectable searchers that can play chess.
/// An engine is the combination of a searcher and an eval.
#[cfg(feature = "chess")]
#[must_use]
pub fn list_chess_searchers() -> SearcherList<Chessboard> {
    let mut res = generic_searchers();
    // The last engine in this list is the default engine
    #[cfg(feature = "caps")]
    res.push(Box::new(SearcherBuilder::<Chessboard, Caps>::new()));
    res
}

#[cfg(feature = "ataxx")]
#[must_use]
pub fn list_ataxx_searchers() -> SearcherList<AtaxxBoard> {
    generic_searchers()
}

#[cfg(feature = "uttt")]
#[must_use]
pub fn list_uttt_searchers() -> SearcherList<UtttBoard> {
    generic_searchers()
}

#[cfg(feature = "mnk")]
#[must_use]
pub fn list_mnk_searchers() -> SearcherList<MNKBoard> {
    generic_searchers()
}

#[cfg(feature = "fairy")]
#[must_use]
pub fn list_fairy_searchers() -> SearcherList<FairyBoard> {
    generic_searchers()
}

pub fn create_match(args: EngineOpts) -> Res<AnyRunnable> {
    match args.game {
        #[cfg(feature = "chess")]
        Game::Chess => create_match_for_game(args, list_chess_searchers(), list_chess_evals(), list_chess_outputs()),
        #[cfg(feature = "ataxx")]
        Game::Ataxx => create_match_for_game(args, list_ataxx_searchers(), list_ataxx_evals(), list_ataxx_outputs()),
        #[cfg(feature = "uttt")]
        Game::Uttt => create_match_for_game(args, list_uttt_searchers(), list_uttt_evals(), list_uttt_outputs()),
        #[cfg(feature = "mnk")]
        Game::Mnk => create_match_for_game(args, list_mnk_searchers(), list_mnk_evals(), list_mnk_outputs()),
        #[cfg(feature = "fairy")]
        Game::Fairy => create_match_for_game(args, list_fairy_searchers(), list_fairy_evals(), list_fairy_outputs()),
    }
}

pub fn run_program_with_args(args: ArgIter) -> Res<()> {
    let args = parse_cli(args).map_err(|err| anyhow!("Failed to parse command line arguments: {err}"))?;
    let mode = args.mode;
    let mut the_match = create_match(args).map_err(|err| anyhow!("Couldn't start the {mode}: {err}"))?;
    _ = the_match.run();
    Ok(())
}

pub fn run_program() -> Res<()> {
    let mut args = std::env::args().peekable();
    _ = args.next(); // remove the program name
    run_program_with_args(args)
}
