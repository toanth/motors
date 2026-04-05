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
use gears::games::ataxx;
#[cfg(feature = "chess")]
use gears::games::chess;
#[cfg(feature = "fairy")]
use gears::games::fairy;
#[cfg(feature = "mnk")]
use gears::games::mnk;
#[cfg(feature = "uttt")]
use gears::games::uttt;
use gears::general::board::Strictness::Relaxed;
use gears::general::board::{BoardHelpers, BoardTrait};
use gears::general::common::Description::WithDescription;
use gears::general::common::anyhow::anyhow;
use gears::general::common::{Res, select_name_dyn};
use gears::general::perft::Bulkness::Bulk;
use gears::general::perft::{perft, split_perft};
use gears::itertools::Itertools;
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
struct BenchRun<B: BoardTrait> {
    engine: Box<dyn Engine<B>>,
    depth: Option<DepthPly>,
    with_nodes: bool,
}

impl<B: BoardTrait> BenchRun<B> {
    pub fn create(options: &EngineOpts, all_searchers: &SearcherList<B>, all_evals: &EvalList<B>) -> Res<Self> {
        let Bench(depth, with_nodes) = options.mode else { unreachable!() };
        let engine = create_engine_box_from_str(&options.engine, all_searchers, all_evals)?;
        Ok(Self { engine, depth, with_nodes })
    }
}

impl<B: BoardTrait> AbstractRun for BenchRun<B> {
    fn run(&mut self) -> Quitting {
        let engine = self.engine.as_mut();
        let nodes = if self.with_nodes { Some(SearchLimit::nodes(engine.default_bench_nodes())) } else { None };
        let depth = self.depth.unwrap_or(engine.default_bench_depth());
        let positions = B::bench_positions().into_iter().collect_vec();
        let res = run_bench_with(engine, SearchLimit::depth(depth), nodes, &positions, None);
        println!("{res}");
        QuitProgram
    }
}

#[derive(Debug, Default)]
struct PerftRun<B: BoardTrait> {
    depth: Option<DepthPly>,
    split: bool,
    pos_name: Option<String>,
    phantom_data: PhantomData<B>,
}

impl<B: BoardTrait> PerftRun<B> {
    pub fn create(depth: Option<DepthPly>, split: bool, pos_name: Option<String>) -> Self {
        Self { depth, split, pos_name, ..Self::default() }
    }
}

impl<B: BoardTrait> AbstractRun for PerftRun<B> {
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
            let res = split_perft(depth, pos, true, Bulk);
            println!("{res}");
        } else {
            let res = perft(depth, pos, true, Bulk);
            println!("{res}");
        }
        QuitProgram
    }
}

// TODO: A lot of this repetitiveness could be avoided with a macro

pub fn create_searcher_from_str<B: BoardTrait>(
    name: &str,
    searchers: &SearcherList<B>,
) -> Res<Box<dyn AbstractSearcherBuilder<B>>> {
    if name == "default" {
        let searcher = searchers.last().expect("No searcher -- check enabled cargo features");
        return Ok(clone_box(&**searcher));
    }
    Ok(clone_box(select_name_dyn(name, searchers, "searcher", &B::game_name(), WithDescription)?))
}

pub fn create_eval_from_str<B: BoardTrait>(name: &str, evals: &EvalList<B>) -> Res<Box<dyn AbstractEvalBuilder<B>>> {
    if name == "default" {
        return Ok(clone_box(&**evals.last().unwrap()));
    }
    Ok(clone_box(select_name_dyn(name, evals, "eval", &B::game_name(), WithDescription)?))
}

pub fn create_engine_from_str<B: BoardTrait>(
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

pub fn create_engine_box_from_str<B: BoardTrait>(
    name: &str,
    searchers: &SearcherList<B>,
    evals: &EvalList<B>,
) -> Res<Box<dyn Engine<B>>> {
    let (searcher, eval) = name.split_once('-').unwrap_or((name, "default"));

    let searcher_builder = create_searcher_from_str(searcher, searchers)?;
    let eval_builder = create_eval_from_str(eval, evals)?;
    Ok(searcher_builder.build(eval_builder.as_ref()))
}

pub fn create_match_for_game<B: BoardTrait>(
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
pub fn list_chess_outputs() -> OutputList<chess::Board> {
    normal_outputs::<chess::Board>(true)
}

#[cfg(feature = "ataxx")]
#[must_use]
pub fn list_ataxx_outputs() -> OutputList<ataxx::Board> {
    normal_outputs::<ataxx::Board>(true)
}

#[cfg(feature = "uttt")]
#[must_use]
pub fn list_uttt_outputs() -> OutputList<uttt::Board> {
    normal_outputs::<uttt::Board>(true)
}

#[cfg(feature = "mnk")]
#[must_use]
pub fn list_mnk_outputs() -> OutputList<mnk::Board> {
    normal_outputs::<mnk::Board>(true)
}

#[cfg(feature = "fairy")]
#[must_use]
pub fn list_fairy_outputs() -> OutputList<fairy::Board> {
    normal_outputs::<fairy::Board>(true)
}

#[must_use]
pub fn generic_evals<B: BoardTrait>() -> EvalList<B> {
    vec![Box::new(EvalBuilder::<B, RandEval>::default())]
}

#[cfg(feature = "chess")]
#[must_use]
pub fn list_chess_evals() -> EvalList<chess::Board> {
    let mut res = generic_evals::<chess::Board>();
    res.push(Box::new(EvalBuilder::<chess::Board, MaterialOnlyEval>::default()));
    res.push(Box::new(EvalBuilder::<chess::Board, PistonEval>::default()));
    res.push(Box::new(EvalBuilder::<chess::Board, KingGambot>::default()));
    res.push(Box::new(EvalBuilder::<chess::Board, LiTEval>::default()));
    res
}

#[cfg(feature = "ataxx")]
#[must_use]
pub fn list_ataxx_evals() -> EvalList<ataxx::Board> {
    let mut res = generic_evals();
    res.push(Box::new(EvalBuilder::<ataxx::Board, Bate>::default()));
    res
}

#[cfg(feature = "uttt")]
#[must_use]
pub fn list_uttt_evals() -> EvalList<uttt::Board> {
    let mut res = generic_evals();
    res.push(Box::new(EvalBuilder::<uttt::Board, Lute>::default()));
    res
}

#[cfg(feature = "mnk")]
#[must_use]
pub fn list_mnk_evals() -> EvalList<mnk::Board> {
    let mut res = generic_evals::<mnk::Board>();
    res.push(Box::new(EvalBuilder::<mnk::Board, BasicMnkEval>::default()));
    res
}

#[cfg(feature = "fairy")]
#[must_use]
pub fn list_fairy_evals() -> EvalList<fairy::Board> {
    generic_evals::<fairy::Board>()
    // TODO: Add special eval functions
}

#[must_use]
pub fn generic_searchers<B: BoardTrait>() -> SearcherList<B> {
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
pub fn list_chess_searchers() -> SearcherList<chess::Board> {
    let mut res = generic_searchers();
    // The last engine in this list is the default engine
    #[cfg(feature = "caps")]
    res.push(Box::new(SearcherBuilder::<chess::Board, Caps>::new()));
    res
}

#[cfg(feature = "ataxx")]
#[must_use]
pub fn list_ataxx_searchers() -> SearcherList<ataxx::Board> {
    generic_searchers()
}

#[cfg(feature = "uttt")]
#[must_use]
pub fn list_uttt_searchers() -> SearcherList<uttt::Board> {
    generic_searchers()
}

#[cfg(feature = "mnk")]
#[must_use]
pub fn list_mnk_searchers() -> SearcherList<mnk::Board> {
    generic_searchers()
}

#[cfg(feature = "fairy")]
#[must_use]
pub fn list_fairy_searchers() -> SearcherList<fairy::Board> {
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

fn version_string() -> String {
    let mut res = "Motors-".to_string();
    res += std::env::consts::ARCH;
    // currently, we're not using any intrinsics more modern than ssse3 apart from pext/pdep, but in practice most cpus should
    // support avx2 or even avx512. Rust even enables sse2 by default, so it's unlikely we'll get the -compat case.
    if cfg!(target_feature = "avx512f") {
        res += "-avx512"
    } else if cfg!(target_feature = "avx2") {
        res += "-avx2"
    } else if cfg!(target_feature = "avx") {
        res += "-avx"
    } else if cfg!(target_feature = "ssse3") {
        res += "-ssse3"
    } else if cfg!(target_feature = "sse2") {
        res += "-sse2";
    } else {
        res += "-compat";
    }
    if cfg!(debug_assertions) {
        res += " (debug version)";
    }
    if !cfg!(feature = "unsafe") {
        res += " [unsafe features disabled]";
    }
    res
}

pub fn run_program_with_args(args: ArgIter) -> Res<()> {
    println!("{}", version_string());
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
