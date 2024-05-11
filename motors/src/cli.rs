use std::fmt::{Display, Formatter};
use std::str::FromStr;

use gears::cli::{get_next_arg, get_next_int, parse_output, ArgIter, Game};
use gears::general::common::Res;
use gears::search::DepthLimit;
use gears::OutputArgs;

use crate::cli::Mode::{Bench, Engine};

#[derive(Debug, Default, Copy, Clone)]
pub enum Mode {
    #[default]
    Engine,
    Bench(Option<DepthLimit>),
}

impl Display for Mode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Engine => write!(f, "engine"),
            Mode::Bench(_) => write!(f, "bench"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EngineOpts {
    pub game: Game,
    /// The name of the engine
    pub engine: String,
    /// An output prints the current position after each move and is also used to show (error) messages.
    pub outputs: Vec<OutputArgs>,
    /// Used to debug the engine. Enables logging as if by using `logger` as additional output.
    pub debug: bool,

    pub mode: Mode,
}

fn parse_bench(args: &mut ArgIter) -> Res<Option<DepthLimit>> {
    let mut res = None;
    if let Some(next) = args.peek() {
        if next == "-d" || next == "--depth" {
            args.next();
            if args.peek().is_some_and(|a| a != "default") {
                res = Some(DepthLimit::new(get_next_int(args, "depth")?));
            }
        }
    }
    Ok(res)
}

fn parse_option(args: &mut ArgIter, opts: &mut EngineOpts) -> Res<()> {
    let mut key = args.next().unwrap_or_default().clone();
    // since we already accept -<long> in monitors for cutechess compatibility,
    // we might as well also accept it in motors.
    if key.starts_with("--") {
        key.remove(0);
    }
    match key.as_str() {
        "bench" | "-bench" | "-b" => opts.mode = Bench(parse_bench(args)?),
        "-engine" | "-e" => opts.engine = get_next_arg(args, "engine")?,
        "-game" | "-g" => opts.game = Game::from_str(&get_next_arg(args, "engine")?.to_lowercase()).map_err(|err| err.to_string())?,
        "-debug" | "-d" => opts.debug = true,
        "-additional-output" | "-output" | "-o" => parse_output(args, &mut opts.outputs)?,
        x => return Err(format!("Unrecognized option '{x}'. Only 'bench', '--engine', '--game', '--debug' and '--outputs' are valid."))
    }
    Ok(())
}

pub fn parse_cli(mut args: ArgIter) -> Res<EngineOpts> {
    let mut res = EngineOpts {
        game: Default::default(),
        engine: "default".to_string(),
        outputs: vec![],
        debug: false,
        mode: Engine,
    };
    while args.peek().is_some() {
        parse_option(&mut args, &mut res)?;
    }
    Ok(res)
}
