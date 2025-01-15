use crate::general::board::{Board, BoardHelpers, Strictness};
use crate::general::common::{tokens, tokens_to_string, NamedEntity, Res, Tokens};
use crate::general::moves::Move;
use anyhow::{anyhow, bail};
use colored::Colorize;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

/// Ugi-related helpers that are used by both `motors` and `monitors`.

#[derive(Default, Debug, Copy, Clone)]
#[must_use]
pub struct UgiCheck {
    pub val: bool,
    pub default: Option<bool>,
}

#[derive(Debug, Copy, Clone, Default)]
#[must_use]
pub struct UgiSpin {
    pub val: i64,
    pub default: Option<i64>,
    pub min: Option<i64>,
    pub max: Option<i64>,
}

#[derive(Debug, Clone, Default)]
#[must_use]
pub struct UgiCombo {
    pub val: String,
    pub default: Option<String>,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Default)]
#[must_use]
pub struct UgiString {
    pub val: String,
    pub default: Option<String>,
}

impl UgiString {
    pub fn value(&self) -> String {
        // The UCI spec demands to send empty strings as '<empty>'
        if self.val.is_empty() {
            "<empty>".to_string()
        } else {
            self.val.clone()
        }
    }
}

#[derive(Clone, Debug)]
#[must_use]
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

    pub fn value_to_str(&self) -> String {
        match self {
            EngineOptionType::Check(check) => check.val.to_string(),
            EngineOptionType::Spin(spin) => spin.val.to_string(),
            EngineOptionType::Combo(combo) => combo.val.to_string(),
            EngineOptionType::Button => "<Button>".to_string(),
            EngineOptionType::UString(string) => string.value(),
        }
    }
}
impl Display for EngineOptionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "type {}", self.type_to_str())?;
        match self {
            EngineOptionType::Check(c) => {
                if let Some(b) = c.default {
                    write!(f, " default {b}")?;
                }
            }
            EngineOptionType::Spin(s) => {
                let default = s
                    .default
                    .map(|x| format!(" default {x}"))
                    .unwrap_or_default();
                let min = s.min.map(|x| format!(" min {x}")).unwrap_or_default();
                let max = s.max.map(|x| format!(" max {x}")).unwrap_or_default();
                write!(f, "{default}{min}{max}")?;
            }
            EngineOptionType::Combo(c) => {
                let default = c
                    .default
                    .clone()
                    .map(|x| format!(" default {x}"))
                    .unwrap_or_default();
                for o in &c.options {
                    write!(f, " var {o}")?;
                }
                write!(f, "{default}")?;
            }
            EngineOptionType::Button => { /*nothing to do*/ }
            EngineOptionType::UString(s) => {
                if let Some(string) = &s.default {
                    write!(f, " default {string}")?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Hash, EnumIter)]
#[must_use]
pub enum EngineOptionName {
    Hash,
    Threads,
    Ponder,
    MultiPv,
    UciElo,
    UCIOpponent,
    UCIEngineAbout,
    UCIShowCurrLine,
    MoveOverhead,
    Strictness,
    RespondToMove,
    SetEngine,
    SetEval,
    Variant,
    Other(String),
}

impl NamedEntity for EngineOptionName {
    fn short_name(&self) -> String {
        self.name().to_string()
    }

    fn long_name(&self) -> String {
        self.short_name()
    }

    fn description(&self) -> Option<String> {
        let res = match self {
            EngineOptionName::Hash => "Size of the Transposition Table in MB",
            EngineOptionName::Threads => "Number of search threads",
            EngineOptionName::Ponder => "Pondering mode. Currently, pondering is supported even without this option, so it has no effect",
            EngineOptionName::MultiPv => "The number of Principal Variation (PV) lines to output",
            EngineOptionName::UciElo => "Limit strength to this elo. Currently not supported",
            EngineOptionName::UCIOpponent => "The opponent. Currently only used to output the name in PGNs",
            EngineOptionName::UCIEngineAbout => "Information about the engine. Can't be changed, only queried",
            EngineOptionName::UCIShowCurrLine => "Every now and then, print the line currently being searched",
            EngineOptionName::MoveOverhead => "Subtract this from the remaining time each move to account for overhead of sending the move",
            EngineOptionName::Strictness => "Be more restrictive about the positions to accept. By default, many non-standard positions are accepted",
            EngineOptionName::RespondToMove => "When the input is a single move, let the engine play one move in response",
            EngineOptionName::SetEngine => "Change the current searcher, and optionally the eval. Similar effect to `uginewgame`",
            EngineOptionName::SetEval => "Change the current evaluation function without resetting the engine state, such as clearing the TT",
            EngineOptionName::Variant => "Changes the current variant for 'fairy', e.g. 'chess' or 'shatranj'",
            EngineOptionName::Other(name) => { return Some(format!("Custom option named '{name}'")) }
        };
        Some(res.to_string())
    }
}

impl EngineOptionName {
    pub fn name(&self) -> &str {
        match self {
            EngineOptionName::Hash => "Hash",
            EngineOptionName::Threads => "Threads",
            EngineOptionName::Ponder => "Ponder",
            EngineOptionName::MultiPv => "MultiPV",
            EngineOptionName::UciElo => "UCI_Elo",
            EngineOptionName::UCIOpponent => "UCI_Opponent",
            EngineOptionName::UCIEngineAbout => "UCI_EngineAbout",
            EngineOptionName::UCIShowCurrLine => "UCI_ShowCurrLine",
            EngineOptionName::MoveOverhead => "MoveOverhead",
            EngineOptionName::Strictness => "Strict",
            EngineOptionName::RespondToMove => "RespondToMove",
            EngineOptionName::SetEngine => "Engine",
            EngineOptionName::SetEval => "SetEval",
            EngineOptionName::Variant => "Variant",
            EngineOptionName::Other(x) => x,
        }
    }
}

impl Display for EngineOptionName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl FromStr for EngineOptionName {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "tt" => EngineOptionName::Hash,
            "move overhead" => EngineOptionName::MoveOverhead,
            name => EngineOptionName::iter()
                .find(|n| n.name().eq_ignore_ascii_case(name))
                .unwrap_or_else(|| EngineOptionName::Other(s.to_string())),
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

impl NamedEntity for EngineOption {
    fn short_name(&self) -> String {
        self.name.name().to_string()
    }

    fn long_name(&self) -> String {
        format!("{self}")
    }

    fn description(&self) -> Option<String> {
        None
    }
}

pub fn parse_ugi_position_part_impl<B: Board>(
    first_word: &str,
    rest: &mut Tokens,
    old_board: &B,
    strictness: Strictness,
) -> Res<B> {
    Ok(match first_word.to_ascii_lowercase().as_str() {
        "fen" | "f" => B::read_fen_and_advance_input(rest, strictness)?,
        "startpos" | "s" => B::startpos_for_settings(old_board.settings()),
        "current" | "c" => old_board.clone(),
        name => match B::from_name(name) {
            Ok(res) => res,
            Err(err) => {
                bail!(
                    "{err} Additionally, '{0}', '{1}' and '{2}' are also always recognized.",
                    "startpos".bold(),
                    "fen <fen>".bold(),
                    "current".bold()
                )
            }
        },
    })
}

pub fn parse_ugi_position_part<B: Board>(
    first_word: &str,
    rest: &mut Tokens,
    allow_position_part: bool,
    old_board: &B,
    strictness: Strictness,
) -> Res<B> {
    let mut first = first_word;
    if allow_position_part
        && (first_word.eq_ignore_ascii_case("position")
            || first_word.eq_ignore_ascii_case("pos")
            || first_word.eq_ignore_ascii_case("p"))
    {
        let Some(pos_word) = rest.next() else {
            bail!("Missing position after '{}' option", "position".bold())
        };
        first = pos_word;
    }
    let remaining = rest.clone();
    let copy = rest.clone();
    let res = parse_ugi_position_part_impl(first, rest, old_board, strictness);
    let Err(err) = res else { return res };
    // If parsing the position failed, we try to insert 'fen' at the beginning and parse it again.
    // (So 'mnk 3 3 3 3/3/3 x 1' is valid)
    let original_string = tokens_to_string(first, copy.clone());
    let mut original_tokens = tokens(&original_string);
    let res = B::read_fen_and_advance_input(&mut original_tokens, strictness);
    *rest = copy;
    let advance_by = rest.clone().count() - original_tokens.count();
    for _ in 0..advance_by {
        _ = rest.next().unwrap();
    }
    let Err(_) = res else { return res };
    // If that failed as well, we try to parse it as a variant, then parse the rest as a position description.
    // (So 'shatranj startpos' is valid)
    *rest = remaining;
    let pos = B::variant(first, rest).map_err(|_| err)?;
    let first = rest.next().unwrap_or("startpos");
    parse_ugi_position_part_impl(first, rest, &pos, strictness).or(Ok(pos))
}

#[allow(clippy::too_many_arguments)]
pub fn parse_ugi_position_and_moves<
    B: Board,
    S,
    F: Fn(&mut S, B::Move) -> Res<()>,
    G: Fn(&mut S),
    H: Fn(&mut S) -> &mut B,
>(
    first_word: &str,
    rest: &mut Tokens,
    accept_pos_word: bool,
    strictness: Strictness,
    old_board: &B,
    state: &mut S,
    make_move: F,
    finish_pos: G,
    get_board: H,
) -> Res<()> {
    let pos = parse_ugi_position_part(first_word, rest, accept_pos_word, old_board, strictness);
    if let Ok(pos) = &pos {
        *(get_board(state)) = pos.clone();
        // don't reset the position if all we get was moves
        finish_pos(state);
    }
    let mut first_move_word = first_word;
    if pos.is_ok() {
        match rest.peek() {
            None => return Ok(()),
            Some(word) => first_move_word = *word,
        }
    }
    let mut parsed_move = false;
    if first_move_word.eq_ignore_ascii_case("moves") || first_move_word.eq_ignore_ascii_case("mv") {
        if pos.is_ok() {
            _ = rest.next();
        }
    } else {
        let Ok(first_move) = B::Move::from_text(first_move_word, get_board(state)) else {
            match pos {
                Ok(_) => return Ok(()),
                Err(err) => bail!(
                    "'{}' is not a valid position or move: {err}",
                    first_word.red()
                ),
            }
        };
        parsed_move = true;
        if pos.is_ok() {
            _ = rest.next();
        }
        debug_assert!(get_board(state).is_move_pseudolegal(first_move));
        make_move(state, first_move).map_err(|err| {
            anyhow!(
                "move '{0}' is pseudolegal but not legal in position '{1}': {err}",
                first_move
                    .compact_formatter(get_board(state))
                    .to_string()
                    .red(),
                *get_board(state)
            )
        })?;
    }
    // TODO: Handle flip / nullmove?
    while let Some(next_word) = rest.peek().copied() {
        let mov = match B::Move::from_text(next_word, get_board(state)) {
            Ok(mov) => mov,
            Err(err) => {
                if !parsed_move {
                    bail!(
                        "'{0}' must be followed by a move, but '{1}' is not a pseudolegal {2} move: {err}",
                        "moves".bold(),
                        next_word.red(),
                        B::game_name()
                    )
                }
                return Ok(()); // allow parsing other commands after a position subcommand
            }
        };
        _ = rest.next();
        debug_assert!(get_board(state).is_move_pseudolegal(mov));
        make_move(state, mov).map_err(|err| {
            anyhow!(
                "move '{0}' is not legal in position '{1}': {err}",
                mov.compact_formatter(get_board(state)).to_string().red(),
                *get_board(state)
            )
        })?;
        parsed_move = true;
    }
    if !parsed_move {
        bail!("Missing move after '{}'", "moves".bold())
    }
    Ok(())
}

pub fn load_ugi_position<B: Board>(
    first_word: &str,
    rest: &mut Tokens,
    accept_pos_word: bool,
    strictness: Strictness,
    old_board: &B,
    allow_partial: bool,
) -> Res<B> {
    let mut board = old_board.clone();
    match parse_ugi_position_and_moves(
        first_word,
        rest,
        accept_pos_word,
        strictness,
        old_board,
        &mut board,
        |pos, next_move| {
            debug_assert!(pos.is_move_legal(next_move));
            *pos = pos.clone().make_move(next_move).ok_or_else(|| {
                anyhow!(
                    "Move '{}' is not legal in position '{pos}' (but it is pseudolegal)",
                    next_move.compact_formatter(pos).to_string().red()
                )
            })?;
            Ok(())
        },
        |_| (),
        |board| board,
    ) {
        Ok(()) => Ok(board),
        Err(err) => {
            if allow_partial {
                Ok(board)
            } else {
                Err(err)
            }
        }
    }
}

pub fn load_ugi_pos_simple<B: Board>(pos: &str, strictness: Strictness, old_board: &B) -> Res<B> {
    let mut tokens = tokens(pos);
    let first = tokens.next().unwrap_or_default();
    load_ugi_position(first, &mut tokens, false, strictness, old_board, false)
}
