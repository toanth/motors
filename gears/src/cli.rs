use std::env::Args;

use std::iter::Peekable;
use std::num::NonZeroUsize;

use std::str::FromStr;
use num::PrimInt;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use crate::general::common::{nonzero_usize, parse_int_from_str, Res};
use crate::OutputArgs;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, derive_more::FromStr, EnumIter)]
pub enum Game {
    /// Normal Chess. Chess960 support WIP.
    #[cfg(feature = "chess")]
    Chess,
    /// m,n,k games are a generalization of Tic-Tac-Toe or Gomoku. Currently, this implementation only supports boards
    /// up to 128 squares.
    #[cfg(feature = "mnk")]
    Mnk,
}

impl Default for Game {
    fn default() -> Self {
        Game::iter().next().unwrap()
    }
}

pub type ArgIter = Peekable<Args>;

pub fn get_next_arg(args: &mut ArgIter, name: &str) -> Res<String> {
    match args.next() {
        None => Err(format!("Missing value for {name} (args ended)")),
        Some(arg) => if arg.starts_with('-') {
            Err(format!("Missing value for {name} (next arg was '{arg}'"))
        } else {
            Ok(arg)
        }
    }
}

pub fn get_next_int<T: PrimInt + FromStr>(args: &mut ArgIter, name: &str) -> Res<T> {
    parse_int_from_str(&get_next_arg(args, name)?, name)
}

pub fn get_next_nonzero_usize(args: &mut ArgIter, name: &str) -> Res<NonZeroUsize> {
    nonzero_usize(get_next_int(args, name)?, name)
}


pub fn parse_output(args: &mut ArgIter, outputs: &mut Vec<OutputArgs>) -> Res<()> {
    let name = get_next_arg(args, "output")?;
    outputs.push(OutputArgs { name, opts: vec![] });
    while let Some(opt) = args.peek() {
        if opt.starts_with('-') || opt == "bench" {
            break;
        }
        outputs.last_mut().unwrap().opts.push(opt.clone());
        args.next();
    }
    Ok(())
}