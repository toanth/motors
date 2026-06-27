#![deny(unused_results)]

use std::sync::{Arc, Mutex};

use gears::dyn_clone::clone_box;

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
use crate::io::cli::{parse_cli, EngineOpts};
use crate::io::ugi_output::UgiOutput;
use crate::io::EngineUGI;
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
};
use gears::cli::{ArgIter, Game};
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
use gears::games::OutputList;
use gears::general::board::{BoardHelpers, BoardTrait};
use gears::general::common::anyhow::anyhow;
use gears::general::common::Description::WithDescription;
use gears::general::common::{select_name_dyn, Res};
use gears::output::normal_outputs;
use gears::rand::prelude::SmallRng;
use gears::{create_selected_output_builders, AnyRunnable, OutputArgs, Quitting};

pub mod eval;
pub mod io;
pub mod search;

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
) -> Res<EngineUGI<B>> {
    if args.debug {
        args.outputs.push(OutputArgs::new("logger".to_string()));
    }
    EngineUGI::create(
        args.clone(),
        create_selected_output_builders(&args.outputs, &outputs)?,
        outputs,
        searchers,
        evals,
    )
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
        Box::new(SearcherBuilder::<B, RandomMover<B, SmallRng>>::default()),
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
    Ok(match args.game {
        #[cfg(feature = "chess")]
        Game::Chess => {
            Box::new(create_match_for_game(args, list_chess_searchers(), list_chess_evals(), list_chess_outputs())?)
        }
        #[cfg(feature = "ataxx")]
        Game::Ataxx => {
            Box::new(create_match_for_game(args, list_ataxx_searchers(), list_ataxx_evals(), list_ataxx_outputs())?)
        }
        #[cfg(feature = "uttt")]
        Game::Uttt => {
            Box::new(create_match_for_game(args, list_uttt_searchers(), list_uttt_evals(), list_uttt_outputs())?)
        }
        #[cfg(feature = "mnk")]
        Game::Mnk => Box::new(create_match_for_game(args, list_mnk_searchers(), list_mnk_evals(), list_mnk_outputs())?),
        #[cfg(feature = "fairy")]
        Game::Fairy => {
            Box::new(create_match_for_game(args, list_fairy_searchers(), list_fairy_evals(), list_fairy_outputs())?)
        }
    })
}

pub fn run_match(args: EngineOpts) -> Res<Quitting> {
    Ok(create_match(args)?.run())
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
    run_match(args).map(|_| ())
}

pub fn run_program() -> Res<()> {
    let mut args = std::env::args().peekable();
    _ = args.next(); // remove the program name
    run_program_with_args(args)
}
