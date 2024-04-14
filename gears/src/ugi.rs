use std::fmt::{Display, Formatter};
use std::str::{FromStr, SplitWhitespace};

use crate::games::Board;
use crate::general::common::{select_name_static, Res};

/// Ugi-related helpers that are used by both `motors` and `monitors`.

#[derive(Default, Debug, Copy, Clone)]
pub struct UgiCheck {
    pub val: bool,
    pub default: Option<bool>,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct UgiSpin {
    pub val: i64,
    pub default: Option<i64>,
    pub min: Option<i64>,
    pub max: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct UgiCombo {
    pub val: String,
    pub default: Option<String>,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UgiString {
    pub val: String,
    pub default: Option<String>,
}

#[derive(Clone, Debug)]
pub enum EngineOptionType {
    Check(UgiCheck),
    Spin(UgiSpin),
    Combo(UgiCombo),
    Button,
    UString(UgiString),
}

impl EngineOptionType {
    pub fn type_to_str(&self) -> &'static str {
        match self {
            EngineOptionType::Check(_) => "check",
            EngineOptionType::Spin(_) => "spin",
            EngineOptionType::Combo(_) => "combo",
            EngineOptionType::Button => "button",
            EngineOptionType::UString(_) => "string",
        }
    }
}
impl Display for EngineOptionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "type {}", self.type_to_str())?;
        match self {
            EngineOptionType::Check(c) => {
                if let Some(b) = c.default {
                    write!(f, "default {b}")?;
                }
            }
            EngineOptionType::Spin(s) => {
                let default = s
                    .default
                    .map(|x| format!(" default {}", x))
                    .unwrap_or_else(String::default);
                let min = s
                    .min
                    .map(|x| format!(" min {}", x))
                    .unwrap_or_else(String::default);
                let max = s
                    .max
                    .map(|x| format!(" max {}", x))
                    .unwrap_or_else(String::default);
                write!(f, "{default}{min}{max}")?;
            }
            EngineOptionType::Combo(c) => {
                let default = c
                    .default
                    .clone()
                    .map(|_x| "default x".to_string())
                    .unwrap_or_else(String::default);
                for o in &c.options {
                    write!(f, " var {o}")?;
                }
                write!(f, " default {default}")?;
            }
            EngineOptionType::Button => { /*nothing to do*/ }
            EngineOptionType::UString(s) => {
                if let Some(string) = &s.default {
                    write!(f, "value {string}")?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum EngineOptionName {
    Hash,
    Threads,
    Ponder,
    MultiPv,
    UciElo,
    Other(String),
}

impl Display for EngineOptionName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EngineOptionName::Hash => "Hash",
            EngineOptionName::Threads => "Threads",
            EngineOptionName::Ponder => "Ponder",
            EngineOptionName::MultiPv => "MultiPV",
            EngineOptionName::UciElo => "UCI_Elo",
            EngineOptionName::Other(x) => x,
        };
        write!(f, "{s}")
    }
}

impl FromStr for EngineOptionName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "hash" => EngineOptionName::Hash,
            "threads" => EngineOptionName::Threads,
            "ponder" => EngineOptionName::Ponder,
            "multipv" => EngineOptionName::MultiPv,
            _ => EngineOptionName::Other(s.to_string()),
        })
    }
}

#[derive(Debug, Clone)]
pub struct EngineOption {
    pub name: EngineOptionName,
    pub value: EngineOptionType,
}

impl Default for EngineOption {
    fn default() -> Self {
        EngineOption {
            name: EngineOptionName::Other(String::default()),
            value: EngineOptionType::Button,
        }
    }
}

impl Display for EngineOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "name {name} {value}",
            name = self.name,
            value = self.value
        )
    }
}

pub fn parse_ugi_position<B: Board>(words: &mut SplitWhitespace, settings: B::Settings) -> Res<B> {
    // let input = words.remainder().unwrap_or_default().trim();
    let position_word = words
        .next()
        .ok_or_else(|| "Missing position after 'position' command".to_string())?;
    Ok(match position_word {
        "fen" | "f" => B::read_fen_and_advance_input(words)?,
        "startpos" | "s" => B::startpos(settings),
        name => (select_name_static(name, &B::name_to_pos_map(), "position", B::game_name())
            .map_err(|err| format!("{err} 'startpos' and 'fen <fen>' are also always recognized."))?
            .val)(),
    })
}
