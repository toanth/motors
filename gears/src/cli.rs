use std::env::Args;

use std::iter::Peekable;
use std::num::NonZeroUsize;

use crate::general::common::Description::NoDescription;
use crate::general::common::{
    nonzero_usize, parse_int_from_str, select_name_static, NamedEntity, Res,
};
use crate::OutputArgs;
use anyhow::anyhow;
use derive_more::Display;
use itertools::Itertools;
use num::PrimInt;
use std::str::FromStr;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Display, derive_more::FromStr, EnumIter,
)]
pub enum Game {
    /// Normal Chess, Chess960 or Double Fisher Random Chess.
    #[cfg(feature = "chess")]
    Chess,
    /// See <https://en.wikipedia.org/wiki/Ataxx> and <https://github.com/EngineProgramming/engine-list?tab=readme-ov-file#ataxx-engines>
    #[cfg(feature = "ataxx")]
    Ataxx,
    /// Ultimate Tic-Tac-Toe, see <https://en.wikipedia.org/wiki/Ultimate_tic-tac-toe>.
    #[cfg(feature = "uttt")]
    Uttt,
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

impl NamedEntity for Game {
    fn short_name(&self) -> String {
        self.to_string()
    }

    fn long_name(&self) -> String {
        self.short_name().to_string()
    }

    fn description(&self) -> Option<String> {
        Some(match self {
            #[cfg(feature = "chess")]
            Game::Chess => "Normal Chess, Chess960 or Double Fisher Random Chess.",
            #[cfg(feature = "ataxx")]
            Game::Ataxx => "Ataxx is a simple but challenging game played on a 7x7 grid where your goal is to convert your opponent's pieces",
            #[cfg(feature = "mnk")]
            Game::Mnk => "m,n,k games are a generalization of Tic-Tac-Toe or Gomoku. Currently, this implementation \
                only supports boards up to 128 squares.",
            #[cfg(feature = "uttt")]
            Game::Uttt => "Ultimate Tic-Tac-Toe is a challenging version of Tic-Tac-Toe where every square is itself a Tic-Tac-Toe board.",
            #[expect(unreachable_patterns)]
            _ => return None,
        }.to_string())
    }
}

pub fn select_game(game_name: &str) -> Res<Game> {
    select_name_static(
        game_name,
        Game::iter().collect_vec().iter(), // lol
        "game",
        "no such game has been implemented",
        NoDescription,
    )
    .copied()
}

pub type ArgIter = Peekable<Args>;

pub fn get_next_arg(args: &mut ArgIter, name: &str) -> Res<String> {
    match args.next() {
        None => Err(anyhow!("Missing value for {name} (args ended)")),
        Some(arg) => {
            if arg.starts_with('-') {
                Err(anyhow!("Missing value for {name} (next arg was '{arg}'"))
            } else {
                Ok(arg)
            }
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
