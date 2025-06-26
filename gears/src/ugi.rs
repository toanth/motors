use crate::general::board::{Board, BoardHelpers, Strictness};
use crate::general::common::{NamedEntity, Res, Tokens, TokensToString, tokens, tokens_to_string};
use crate::general::moves::Move;
use crate::output::pgn::parse_pgn_moves_format;
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
        if self.val.is_empty() { "<empty>".to_string() } else { self.val.clone() }
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
                let default = s.default.map(|x| format!(" default {x}")).unwrap_or_default();
                let min = s.min.map(|x| format!(" min {x}")).unwrap_or_default();
                let max = s.max.map(|x| format!(" max {x}")).unwrap_or_default();
                write!(f, "{default}{min}{max}")?;
            }
            EngineOptionType::Combo(c) => {
                let default = c.default.clone().map(|x| format!(" default {x}")).unwrap_or_default();
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
    UCIVariant,
    UciElo,
    UCIOpponent,
    UCIEngineAbout,
    UCIShowRefutations,
    UCIShowCurrLine,
    CurrlineNullmove,
    MoveOverhead,
    Strictness,
    RespondToMove,
    SetEngine,
    SetEval,
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
            EngineOptionName::Hash => "Size of the Transposition Table in MiB",
            EngineOptionName::Threads => "Number of search threads",
            EngineOptionName::Ponder => {
                "Pondering mode. Pondering is supported even without this option, so it has no effect"
            }
            EngineOptionName::MultiPv => "The number of Principal Variation (PV) lines to output",
            EngineOptionName::UCIVariant => "Changes the current variant for 'fairy', e.g. 'chess' or 'shatranj'",
            EngineOptionName::UciElo => "Limit strength to this elo. Currently not supported",
            EngineOptionName::UCIOpponent => "The opponent. Currently only used to output the name in PGNs",
            EngineOptionName::UCIEngineAbout => "Information about the engine. Can't be changed, only queried",
            EngineOptionName::UCIShowRefutations => "Print the top alternative moves using the UCI Refutation command",
            EngineOptionName::UCIShowCurrLine => "Every now and then, print the line currently being searched",
            EngineOptionName::CurrlineNullmove => {
                "Print nullmoves in non-interactive `currline`, if they exist. Option is ignored if currline isn't printed"
            }
            EngineOptionName::MoveOverhead => {
                "Subtract this from the remaining time each move to account for overhead of sending the move"
            }
            EngineOptionName::Strictness => {
                "Be more restrictive about the positions to accept. By default, many non-standard positions are accepted"
            }
            EngineOptionName::RespondToMove => {
                "When the input is a single move, let the engine play one move in response"
            }
            EngineOptionName::SetEngine => {
                "Change the current searcher, and optionally the eval. Similar effect to `uginewgame`"
            }
            EngineOptionName::SetEval => {
                "Change the current evaluation function without resetting the engine state, such as clearing the TT"
            }
            EngineOptionName::Other(name) => return Some(format!("Custom option named '{name}'")),
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
            EngineOptionName::UCIVariant => "UCI_Variant",
            EngineOptionName::UciElo => "UCI_Elo",
            EngineOptionName::UCIOpponent => "UCI_Opponent",
            EngineOptionName::UCIEngineAbout => "UCI_EngineAbout",
            EngineOptionName::UCIShowRefutations => "UCI_ShowRefutations",
            EngineOptionName::UCIShowCurrLine => "UCI_ShowCurrLine",
            EngineOptionName::CurrlineNullmove => "CurrlineNullmove",
            EngineOptionName::MoveOverhead => "MoveOverhead",
            EngineOptionName::Strictness => "Strict",
            EngineOptionName::RespondToMove => "RespondToMove",
            EngineOptionName::SetEngine => "Engine",
            EngineOptionName::SetEval => "SetEval",
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
        EngineOption { name: EngineOptionName::Other(String::default()), value: EngineOptionType::Button }
    }
}

impl Display for EngineOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "name {name} {value}", name = self.name, value = self.value)
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
    current_pos: &B,
    strictness: Strictness,
) -> Res<B> {
    Ok(match first_word.to_ascii_lowercase().as_str() {
        "fen" | "f" => B::read_fen_and_advance_input_for(rest, strictness, current_pos.settings_ref())?,
        "startpos" | "s" => B::startpos_for_settings(current_pos.settings_ref()),
        "current" | "c" => current_pos.clone(),
        // this is effectively just ignored, but it's nice to get autocompletion specifically for position names
        "name" | "pos_name" => {
            let Some(next) = rest.next() else { bail!("Expected a position name after 'name' subcommand") };
            parse_ugi_position_part_impl(next, rest, current_pos, strictness)?
        }
        "" => bail!("Empty position description, expected 'fen', 'startpos', 'current' or a name"),
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
    current_pos: &B,
    strictness: Strictness,
) -> Res<B> {
    let mut first = first_word;
    if allow_position_part
        && (first_word.eq_ignore_ascii_case("position")
            || first_word.eq_ignore_ascii_case("pos")
            || first_word.eq_ignore_ascii_case("p"))
    {
        let Some(pos_word) = rest.next() else { bail!("Missing position after '{}' option", "position".bold()) };
        first = pos_word;
    }
    let remaining = rest.clone();
    let copy = rest.clone();
    let res = parse_ugi_position_part_impl(first, rest, current_pos, strictness);
    let Err(err) = res else { return res };
    // If parsing the position failed, we try to insert 'fen' at the beginning and parse it again.
    // (So 'mnk 3 3 3 3/3/3 x 1' is valid)
    let original_string = tokens_to_string(first, copy.clone());
    let mut original_tokens = tokens(&original_string);
    let res = B::read_fen_and_advance_input_for(&mut original_tokens, strictness, current_pos.settings_ref());
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

fn ignore_move_number(input: &str) -> Option<&str> {
    let remaining = input.trim_start_matches(|c: char| c.is_ascii_digit());
    if remaining == input {
        return None;
    }
    if let Some(remaining) = remaining.strip_prefix("...") {
        return Some(remaining.trim_ascii_start());
    } else if let Some(remaining) = remaining.strip_prefix('.') {
        return Some(remaining.trim_ascii_start());
    }
    None
}

fn parse_ugi_moves_part<B: Board>(
    words: &mut Tokens,
    state: &mut dyn ParseUgiPosState<B>,
    allow_empty: bool,
) -> Res<()> {
    let mut parsed_move = false;
    let mut has_moves_word = false;
    let mut current_word = words.peek().copied();
    if current_word.is_some_and(|f| f.eq_ignore_ascii_case("moves") || f.eq_ignore_ascii_case("mv")) {
        _ = words.next();
        current_word = words.peek().copied();
        has_moves_word = true
    }
    // TODO: Handle flip / nullmove?
    let mut accept_move_number = true;
    while let Some(next_word) = current_word {
        let mov = match B::Move::from_text(next_word, state.pos()) {
            Ok(mov) => mov,
            Err(err) => {
                if accept_move_number {
                    if let Some(remaining) = ignore_move_number(next_word) {
                        accept_move_number = false;
                        if remaining.is_empty() {
                            _ = words.next();
                            current_word = words.peek().copied();
                        } else {
                            current_word = Some(remaining);
                        }
                        continue;
                    }
                }
                if !parsed_move && has_moves_word {
                    bail!(
                        "'{0}' must be followed by a move, but '{1}' is not a legal {2} move: {err}",
                        "moves".bold(),
                        next_word.red(),
                        B::game_name()
                    )
                } else if !parsed_move && !allow_empty {
                    bail!("Expected a valid move, got '{}'", next_word.red())
                }
                return Ok(()); // allow parsing other commands after a position subcommand
            }
        };
        accept_move_number = true;
        debug_assert!(state.pos().is_move_pseudolegal(mov));
        state.make_move(mov).map_err(|err| {
            anyhow!(
                "move '{0}' is pseudolegal but not legal in position '{1}': {err}",
                mov.compact_formatter(state.pos()).to_string().red(),
                *state.pos()
            )
        })?;
        parsed_move = true;
        _ = words.next();
        current_word = words.peek().copied();
    }
    if !parsed_move && !allow_empty {
        bail!("Missing {0} move'", B::game_name())
    }
    Ok(())
}

pub fn parse_moves<B: Board>(words: &mut Tokens, state: &mut dyn ParseUgiPosState<B>, allow_empty: bool) -> Res<()> {
    let mut input_copy = words.clone();
    if let Err(err) = parse_ugi_moves_part(words, state, allow_empty) {
        if input_copy.peek().is_some_and(|f| f.eq_ignore_ascii_case("moves") || f.eq_ignore_ascii_case("mv")) {
            _ = input_copy.next();
            if input_copy.peek().is_none() {
                bail!("Expected moves after '{}' command", "moves".bold())
            }
        }
        let moves = input_copy.string();
        *words = input_copy;
        let Ok(pgn_data) = parse_pgn_moves_format(&moves, state.pos()) else {
            return Err(err);
        };
        for m in pgn_data.game.mov_hist {
            state.make_move(m)?
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn parse_ugi_position_and_moves<B: Board>(
    first_word: &str,
    rest: &mut Tokens,
    accept_pos_word: bool,
    strictness: Strictness,
    state: &mut dyn ParseUgiPosState<B>,
) -> Res<()> {
    let mut input_copy = rest.clone();
    let pos = parse_ugi_position_part(first_word, rest, accept_pos_word, state.pos(), strictness);
    // don't reset the position if all we got was moves
    // (i.e. 'p mv e4' allows going back to a position before the current position, unlike `p c mv e4`)
    if let Ok(pos) = &pos {
        state.finish_pos_part(pos);
    }
    let res = if pos.is_ok() {
        if rest.peek().is_none() {
            return Ok(());
        }
        parse_moves(rest, state, true)
    } else {
        // pretend that we parsed the current position and try again
        let string = tokens_to_string(first_word, input_copy.clone());
        let mut tokens = tokens(&string);
        let res = parse_moves(&mut tokens, state, false);
        let advance_by = rest.clone().count().saturating_sub(tokens.clone().count());
        for _ in 0..advance_by {
            _ = rest.next().unwrap();
        }
        res
    };
    if res.is_err() && pos.is_err() {
        bail!("'{0}' is not a valid position or move: {1}", input_copy.string().red(), pos.err().unwrap())
    }
    res
}

pub fn only_load_ugi_position<B: Board>(
    first_word: &str,
    rest: &mut Tokens,
    current_pos: &B,
    strictness: Strictness,
    allow_pos_word: bool,
    allow_partial: bool,
) -> Res<B> {
    let mut state =
        SimpleParseUgiPosState { pos: current_pos.clone(), initial_pos: current_pos.clone(), previous_pos: None };
    match parse_ugi_position_and_moves(first_word, rest, allow_pos_word, strictness, &mut state) {
        Ok(()) => Ok(state.pos.clone()),
        Err(err) => {
            if allow_partial {
                Ok(state.pos.clone())
            } else {
                Err(err)
            }
        }
    }
}

pub trait ParseUgiPosState<B: Board> {
    fn pos(&mut self) -> &mut B;
    fn initial(&self) -> &B;
    fn previous(&self) -> Option<&B>;
    fn finish_pos_part(&mut self, pos: &B);
    fn make_move(&mut self, mov: B::Move) -> Res<()>;
}

struct SimpleParseUgiPosState<B: Board> {
    pos: B,
    initial_pos: B,
    previous_pos: Option<B>,
}

impl<B: Board> ParseUgiPosState<B> for SimpleParseUgiPosState<B> {
    fn pos(&mut self) -> &mut B {
        &mut self.pos
    }
    fn initial(&self) -> &B {
        &self.initial_pos
    }
    fn previous(&self) -> Option<&B> {
        self.previous_pos.as_ref()
    }
    fn finish_pos_part(&mut self, pos: &B) {
        self.pos = pos.clone();
    }

    fn make_move(&mut self, mov: B::Move) -> Res<()> {
        debug_assert!(self.pos.is_move_legal(mov));
        self.pos = self.pos.clone().make_move(mov).ok_or_else(|| {
            anyhow!(
                "Move '{0}' is not legal in position '{1}' (but it is pseudolegal)",
                mov.compact_formatter(&self.pos).to_string().red(),
                self.pos
            )
        })?;
        Ok(())
    }
}

pub fn load_ugi_pos_simple<B: Board>(pos: &str, strictness: Strictness, old_board: &B) -> Res<B> {
    let mut tokens = tokens(pos);
    let first = tokens.next().unwrap_or_default();
    let res = only_load_ugi_position(first, &mut tokens, old_board, strictness, false, false)?;
    if let Some(next) = tokens.next() {
        bail!(
            "After loading position '{0}', there is unconsumed input starting with the token '{1}'",
            pos.bold(),
            next.red()
        )
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::chess::Chessboard;
    use crate::general::board::Strictness::{Relaxed, Strict};

    #[cfg(feature = "chess")]
    #[test]
    fn test_chess_parsing() {
        let input = "startpos moves e2e4 e7e5 yolo";
        let mut pos = Chessboard::startpos();
        assert!(load_ugi_pos_simple(input, Relaxed, &pos).is_err());
        assert!(only_load_ugi_position("position", &mut tokens(input), &pos, Strict, false, false).is_err());
        assert!(only_load_ugi_position("lol", &mut tokens(input), &pos, Strict, true, false).is_err());
        let mut input_tokens = tokens(input);
        let res = only_load_ugi_position("position", &mut input_tokens, &pos, Strict, true, false).unwrap();
        pos = pos.make_move_from_str("e2e4").unwrap();
        pos = pos.make_move_from_str("e7e5").unwrap();
        assert_eq!(res, pos);
        assert_eq!(input_tokens.next(), Some("yolo"));

        let mut pos = Chessboard::from_name("kiwipete").unwrap();
        let moves = " 0-0 e8h8 a2a3";
        let input = pos.as_fen() + moves;
        assert!(
            only_load_ugi_position("position", &mut tokens(moves), &Chessboard::default(), Relaxed, true, false)
                .is_err()
        );
        assert!(
            only_load_ugi_position("pos", &mut tokens(&input), &Chessboard::default(), Relaxed, true, false).is_ok()
        );
        let res = load_ugi_pos_simple(&input, Strict, &pos).unwrap();
        pos = pos.make_move_from_str("O-O").unwrap();
        pos = pos.make_move_from_str("0-0 ?").unwrap();
        pos = pos.make_move_from_str("a3!!").unwrap();
        assert_eq!(res, pos);

        let pos = Chessboard::from_name("lucena").unwrap();
        let input = "lucena moves not";
        assert!(
            only_load_ugi_position("position", &mut tokens(input), &Chessboard::default(), Relaxed, true, false)
                .is_err()
        );
        let res = only_load_ugi_position("p", &mut tokens(input), &Chessboard::default(), Relaxed, true, true).unwrap();
        assert_eq!(pos, res);
    }
}
