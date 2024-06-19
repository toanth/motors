use std::collections::HashMap;
use std::env::Args;
use std::iter::Peekable;
use std::num::{NonZeroU64, NonZeroUsize};
use std::ops::Add;
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;
use std::sync::MutexGuard;
use std::time::Duration;

use itertools::Itertools;
use num::PrimInt;

use gears::cli::{get_next_arg, get_next_int, get_next_nonzero_usize, parse_output, ArgIter, Game};
use gears::general::common::{
    nonzero_u64, nonzero_usize, parse_duration_ms, parse_fp_from_str, parse_int_from_str, Res,
};
use gears::score::Score;
use gears::search::{Depth, TimeControl};
use gears::OutputArgs;

use crate::cli::PlayerArgs::{Engine, Human};
use crate::cli::Protocol::{Uci, Ugi};
use crate::play::adjudication::ScoreAdjudication;
use crate::play::player::{Protocol, TimeMargin};
use crate::play::ugi_client::Client;

/// Since clap doesn't handle long arguments with a single `-`, but cutechess (and fastchess) use that format,
/// this just writes the parser by hand
pub struct CommandLineArgs {
    /// The game to run.
    pub game: Game,

    /// Sets both the main output and input. The output determines how information is shown to the user, such as via terminal
    /// or though a GUI. The input can change the match state and is often coupled to the output, such as in a GUI.
    /// Another important input instance is the SPRT runner (TODO: Implement).
    pub ui: String,

    /// Used to debug the GUI-Engine communication. Enables logging as if by using `logger` as additional output.
    /// When using a built-in engine, also passes --debug to them.
    pub debug: bool,

    /// All players (usually there's at most one human). Currently, no tournaments are implemented, so this is limited to 2.
    pub players: Vec<PlayerArgs>,

    /// how may matches to run in parallel
    pub concurrency: NonZeroUsize, // TODO: Use

    /// Adjudicate a match as draw if the score of both engines is close to zero for a prolonged period of time
    pub draw_adjudication: Option<ScoreAdjudication>,

    /// Adjudicate a match as resignation if an engine's score is below a negated threshold and the other engine's
    /// score is above that threshold for a prolonged period of time.
    pub resign_adjudication: Option<ScoreAdjudication>,

    /// Adjudicate matches where the number of moves exceeds this number as draws.
    pub max_moves: Option<NonZeroUsize>,

    /// The name of the event as displayed in a PGN
    pub event: Option<String>,

    /// The name of the site as displayed in a PGN
    pub site: Option<String>,

    /// Store the PGNs in this file
    pub pgn_out: Option<PathBuf>,

    /// Store the FENs of the last positions in this file
    pub fen_out: Option<PathBuf>,

    /// Wait for the specified duration after each match (defaults to 0).
    pub wait_after_match: Duration,

    pub start_pos: Option<String>,

    /// If true, engines are restarted on failure (this still counts as a lost match).
    /// If false, the program simply exits.
    pub recover: bool,

    // /// Print the rating after a multiple of this number of matches
    // pub rating_interval: Option<usize>,
    //
    // /// Print the results after a multiple of this number of matches
    // pub outcome_interval: Option<usize>,
    /// Additional ways ot print the current match state. Can be used for logging or to get more beautiful / relevant
    /// outputs, such as printing the current FEN after each move, export the match as a PGN, or pretty-print the board.
    /// This can also be changed on the fly while the program is running.
    pub additional_outputs: Vec<OutputArgs>,
}

#[derive(Debug, Default, Clone)]
pub struct ClientEngineCliArgs {
    /// This name will be displayed in the GUI and be used for logfiles.
    /// If not given, this defaults to the UGI 'id name' or, if this isn't send either, the executable name
    /// If there are two engines with the same display name, one of them gets a number appended to make the names unique,
    /// e.g. caps_2, etc.
    pub display_name: Option<String>,

    /// The executable to run, e.g. "stockfish"
    pub cmd: String,

    /// The path to the executable, e.g. "~/Documents/engines/stockfish" or "C:\Desktop\stockfish"
    pub path: Option<PathBuf>,

    /// Command line arguments to pass to the engine, e.g. "--debug"
    pub engine_args: Vec<String>,

    /// Text sent to the engine before sending 'UGI' or 'UCI'
    pub init_string: Option<String>,

    /// Redirect Stderr to this file. If debug is set but this option isn't set, the filename is determined
    /// based on the name (i.e. `display_name`, not the name the engine sends though UGI).
    pub stderr: Option<PathBuf>,

    /// Only 'Uci' and 'Ugi' are supported, and if the engine doesn't respond to the initial 'uci' / 'ugi', the
    /// GUI tries the other protocol. So specifying this protocol really only makes sense for compatibility.
    pub proto: Option<Protocol>,

    /// The time control to use for this engine
    pub tc: Option<TimeControl>,

    /// Limit the engine to the given number of seconds per move.
    pub move_time: Option<Duration>,

    /// The engine is allowed to exceed the remaining time by this amount.
    pub time_margin: Option<TimeMargin>,

    /// If true, the engine's score is always from white's perspective and needs to be flipped for black (default: false).
    pub white_pov: bool,

    /// Limit the depth the engine searches to.
    pub depth: Option<Depth>,

    /// Try to find a mate in n *moves* (not plies), searches forever if there isn't one (unless another limit is also specified)
    pub mate: Option<Depth>,

    /// Limit the engine to the given number of nodes
    pub nodes: Option<NonZeroU64>,

    /// Set custom UCI/UGI options for the engine. No validation is performed by the GUI.
    pub custom_options: HashMap<String, String>,

    /// Add `--debug` to flags if the engine is built-in, and it's not already given in `engine_args`
    pub add_debug_flag: bool,
}

// TODO: Not really suited for cli.rs anymore
#[derive(Debug, Default, Clone)]
pub struct HumanArgs {
    pub tc: Option<TimeControl>,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub enum PlayerArgs {
    Human(HumanArgs),
    Engine(ClientEngineCliArgs),
}

fn parse_key_equals_value(arg: &str) -> Res<(&str, Res<&str>)> {
    let mut parts = arg.split('=');
    let key = parts.next().unwrap();
    let value = parts
        .next()
        .ok_or_else(|| format!("Expected '=<value>' after '{key}'"));
    if let Some(rest) = parts.next() {
        let rest = rest.to_string().add(&parts.join("="));
        return Err(format!(
            "Expected an argument of the form 'key=value' or 'key' but got '{key}={}={rest}'",
            value.unwrap()
        ));
    }
    Ok((key, value))
}

fn parse_game(args: &mut ArgIter, res: &mut CommandLineArgs) -> Res<()> {
    let game = get_next_arg(args, "game")?.to_lowercase();
    res.game = Game::from_str(&game).map_err(|err| err.to_string())?;
    Ok(())
}

fn parse_ui(args: &mut ArgIter, res: &mut CommandLineArgs) -> Res<()> {
    let ui = get_next_arg(args, "ui")?;
    res.ui = ui;
    Ok(())
}

fn parse_adjudication(args: &mut ArgIter, is_draw: bool) -> Res<ScoreAdjudication> {
    let mut res = ScoreAdjudication::default();
    let mut twosided = is_draw;
    while args.peek().is_some_and(|a| !a.starts_with('-')) {
        let arg = get_next_arg(args, "resign or draw adjudication")?;

        let (key, val) = parse_key_equals_value(&arg)?;
        let val = val?;
        match key {
            "movecount" => res.move_number = parse_int_from_str(&val, "movecount")?,
            "movenumber" => res.start_after = parse_int_from_str(&val, "movenumber")?,
            "score" => res.score_threshold = Score(parse_int_from_str(&val, "score")?),
            "twosided" => twosided = bool::from_str(val).map_err(|err| err.to_string())?,
            _ => {
                return Err(format!(
                    "Invalid adjudication setting '{val}' with unknown key '{key}'"
                ))
            }
        }
    }
    if !twosided {
        eprintln!("Warning: the 'twosided' option is implicitly set for adjudication settings and cannot be disabled");
    }
    Ok(res)
}

// Channeling my inner C++ programmer to write a function accepting a generic iterator.
pub fn parse_engine<Iter: Iterator<Item = String>>(
    args: &mut Peekable<Iter>,
) -> Res<ClientEngineCliArgs> {
    let mut res = ClientEngineCliArgs::default();
    while let Some(arg) = args.peek() {
        if arg.starts_with('-') {
            return Ok(res);
        }
        let arg = args.next().unwrap();
        let (key, value) = parse_key_equals_value(&arg)?;
        match key {
            "conf" => todo!("Engine config files aren't supported for now"),
            "name" => res.display_name = Some(value?.to_string()),
            "cmd" => res.cmd = value?.to_string(),
            "dir" => res.path = Some(PathBuf::from_str(value?).map_err(|err| err.to_string())?),
            "arg" => res.engine_args.push(value?.to_string()),
            "initstr" => res.init_string = Some(value?.to_string()),
            "stderr" => res.stderr = Some(PathBuf::from_str(value?).map_err(|err| err.to_string())?),
            "restart" => todo!(),
            "trust" => eprintln!("The 'trust' engine option is always ignored and only exist for compatibility"),
            "proto" => match value?.to_ascii_lowercase().as_str() {
                "ugi" => res.proto = Some(Ugi),
                "uci" => res.proto = Some(Uci),
                x => return Err(format!("Unrecognized engine protocol '{x}'. Only 'uci', 'ugi' or simply not specifying this argument are valid"))
            },
            "tc" => res.tc = Some(TimeControl::from_str(value?)?),
            "st" => res.move_time = Some(Duration::from_secs_f64(parse_fp_from_str(value?, "st (move time)")?)),
            "timemargin" => res.time_margin = Some(TimeMargin(parse_duration_ms(&mut value?.split_whitespace(), "timemargin")?)),
            "book" => todo!(),
            "bookdepth" => todo!(),
            "whitepov" => res.white_pov = true,
            "depth" => res.depth = Some(Depth::new(parse_int_from_str(value?, "depth")?)),
            "mate" => res.mate = Some(Depth::new(parse_int_from_str(value?, "mate")?)),
            "nodes" => res.nodes = Some(nonzero_u64(parse_int_from_str(value?, "nodes")?, "nodes")?),
            "ponder" => todo!("'ponder' isn't yet implemented"),
            "tscale" => todo!("'tscale' isn't yet implemented"),
            x => match x.strip_prefix("option.") {
                None => return Err(format!("Unknown engine option {x}")),
                Some(opt) => { res.custom_options.insert(x.to_string(), opt.to_string()); },
            },
        }
    }
    Ok(res)
}

pub fn parse_human<Iter: Iterator<Item = String>>(args: &mut Peekable<Iter>) -> Res<HumanArgs> {
    let mut res = HumanArgs::default();
    while let Some(arg) = args.peek() {
        if arg.starts_with('-') {
            return Ok(res);
        }
        let arg = args.next().unwrap();
        let (key, value) = parse_key_equals_value(&arg)?;
        match key {
            "tc" => res.tc = Some(TimeControl::from_str(value?)?),
            "name" => res.name = Some(value?.to_string()),
            x => return Err(format!("Unknown argument '{x}' for a human player")),
        }
    }
    return Ok(res);
}

fn print_help_message() {
    todo!()
}

fn print_version() {
    println!("monitors {}", get_version());
    exit(0)
}

fn get_version() -> &'static str {
    option_env!("CARGO_PKG_VERSION").unwrap_or("<unknown version>")
}

pub fn combine_engine_args(
    engine: &mut ClientEngineCliArgs,
    each: &ClientEngineCliArgs,
    add_debug_flag: bool,
) {
    // Logically, this function performs |= on each contained `Option`. Unfortunately,
    // Rust doesn't provide a built-in |= operator for `Option`s.
    engine.display_name = engine
        .display_name
        .clone()
        .or_else(|| each.display_name.clone());
    if engine.cmd.is_empty() {
        engine.cmd = each.cmd.clone();
    }
    engine.path = engine.path.clone().or_else(|| each.path.clone());
    if engine.engine_args.is_empty() {
        engine.engine_args = each.engine_args.clone();
    }
    engine.add_debug_flag = add_debug_flag;
    engine.init_string = engine
        .init_string
        .clone()
        .or_else(|| each.init_string.clone());
    engine.stderr = engine.stderr.clone().or_else(|| each.stderr.clone());
    engine.proto = engine.proto.clone().or_else(|| each.proto.clone());
    engine.tc = engine.tc.clone().or_else(|| each.tc.clone());
    engine.move_time = engine.move_time.clone().or_else(|| each.move_time.clone());
    engine.time_margin = engine
        .time_margin
        .clone()
        .or_else(|| each.time_margin.clone());
    engine.white_pov |= each.white_pov;
    engine.depth = engine.depth.clone().or_else(|| each.depth.clone());
    engine.nodes = engine.nodes.clone().or_else(|| each.nodes.clone());
    each.custom_options.iter().for_each(|(key, value)| {
        engine
            .custom_options
            .entry(key.clone())
            .or_insert(value.clone());
    });
}

pub fn parse_cli() -> Res<CommandLineArgs> {
    let mut args = std::env::args().peekable();
    let _name = args.next().expect("The program name is missing?!");
    if args.peek().is_some_and(|a| a == "motors") {
        args.next().unwrap();
        motors::run_program_with_args(args)?;
        exit(0);
    }

    let mut res = CommandLineArgs {
        game: Game::default(),
        ui: "text".to_string(), // TODO: Change default
        debug: false,
        players: vec![],
        concurrency: NonZeroUsize::new(1).unwrap(),
        draw_adjudication: None,
        resign_adjudication: None,
        max_moves: None,
        event: None,
        site: None,
        pgn_out: None,
        fen_out: None,
        wait_after_match: Duration::default(),
        start_pos: None,
        recover: false,
        additional_outputs: vec![],
    };

    let mut each = ClientEngineCliArgs::default();

    while let Some(mut arg) = args.next() {
        // cutechess-cli expects top-level arguments to always start with a single '-',
        // but also supporting the much more common '--long' syntax is probably a good idea
        if arg.starts_with("--") {
            arg.remove(0);
        }
        match arg.as_str() {
            "-h" | "-help" => print_help_message(),
            "-v" | "-version" => print_version(),
            "-g" | "-game" | "-variant" => parse_game(&mut args, &mut res)?,
            "-ui" => parse_ui(&mut args, &mut res)?,
            "-d" | "-debug" => res.debug = true,
            "-additional-output" | "-output" | "-o" => {
                parse_output(&mut args, &mut res.additional_outputs)?
            }
            "-engine" => res.players.push(Engine(parse_engine(&mut args)?)),
            "-human" => res.players.push(Human(parse_human(&mut args)?)),
            "-each" => each = parse_engine(&mut args)?,
            "-concurrency" => res.concurrency = get_next_nonzero_usize(&mut args, "concurrency")?,
            "-resign" => res.resign_adjudication = Some(parse_adjudication(&mut args, false)?),
            "-draw" => res.draw_adjudication = Some(parse_adjudication(&mut args, true)?),
            "-maxmoves" => res.max_moves = Some(get_next_nonzero_usize(&mut args, "maxmoves")?),
            "-tournament" => todo!(),
            "-event" => res.event = Some(get_next_arg(&mut args, "event")?),
            "-games" => todo!(),
            "-rounds" => todo!(),
            "-sprt" => todo!(),
            "-ratinginterval" => todo!(),
            "-outcomeinterval" => todo!(),
            "-openings" => todo!(),
            "-bookmode" => todo!(),
            "-pgnout" => todo!(),
            "-epdout" | "-fenout" => todo!(),
            "-recover" => res.recover = true,
            "-noswap" => todo!(),
            "-reverse" => todo!(),
            "-seeds" => todo!(),
            "-site" => res.site = Some(get_next_arg(&mut args, "site")?),
            "-srand" => todo!(),
            "-wait" => {
                res.wait_after_match =
                    Duration::from_millis(get_next_int::<i64>(&mut args, "wait")?.max(1) as u64)
            }
            "-resultformat" => todo!(),
            "-startpos" => todo!(), // set one startpos for all matches. Incompatible with sprt.
            x => {
                return Err(format!(
                    "Unrecognized option '{x}'. Type --help for a list of all valid options"
                ))
            }
        }
    }

    for player in res.players.iter_mut() {
        match player {
            Human(_) => {}
            Engine(args) => combine_engine_args(args, &each, args.add_debug_flag),
        }
    }

    Ok(res)
}
