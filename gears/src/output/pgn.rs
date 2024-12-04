/*
 *  Gears, a collection of board games.
 *  Copyright (C) 2024 ToTheAnd
 *
 *  Gears is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Gears is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Gears. If not, see <https://www.gnu.org/licenses/>.
 */

//! <See https://ia902908.us.archive.org/26/items/pgn-standard-1994-03-12/PGN_standard_1994-03-12.txt>

use crate::games::{BoardHistory, Color};
use crate::general::board::Board;
use crate::general::common::{ColorMsg, Res};
use crate::general::moves::ExtendedFormat::Standard;
use crate::general::moves::Move;
use crate::output::pgn::RoundNumber::{Custom, Number, Unimportant, Unknown};
use crate::output::pgn::TagPair::{Black, Date, Event, Other, Result, Round, Site, White};
use crate::MatchStatus::*;
use crate::ProgramStatus::Run;
use crate::{AdjudicationReason, GameOverReason, GameResult, GameState, MatchResult, MatchState};
use anyhow::bail;
use std::fmt::Display;
use std::iter::Peekable;
use std::mem::take;
use std::str::{Chars, FromStr};

pub fn match_to_pgn_string<B: Board>(m: &dyn GameState<B>) -> String {
    let result = match m.match_status() {
        Over(r) => r.result.to_canonical_string(),
        _ => "\"*\"".to_string(),
    };
    let status = m.match_status();
    let termination = match &status {
        NotStarted => "not started",
        Ongoing => "unterminated",
        Over(ref res) => match res.reason {
            GameOverReason::Normal => "normal",
            GameOverReason::Adjudication(ref reason) => match reason {
                AdjudicationReason::TimeUp => "time forfeit",
                AdjudicationReason::InvalidMove => "rules infraction",
                AdjudicationReason::AbortedByUser => "abandoned",
                AdjudicationReason::EngineError => "emergency",
                AdjudicationReason::Adjudicator(ref reason) => reason,
            },
        },
    };
    let mut res = format!(
        "[Event \"{event}\"]\n\
        [Site \"{site}\"]\n\
        [Date \"{date}\"]\n\
        [Round \"1\"]\n\
        [{p1_name} \"{p1}\"]\n\
        [{p2_name} \"{p2}\"]\n\
        [Result \"{result}\"]\n\
        [FEN \"{fen}\"]\n\
        [Termination \"{termination}\"]\n\
        [TimeControl \"??\"]\n\
        [Variant \"From Position\"]\n\n\
        % automatically generated {game} pgn",
        game = m.game_name(),
        event = m.event(),
        site = m.site(),
        // the standard requires `YYYY.MM.DD`, but that doesn't have a high enough resolution
        date = chrono::offset::Utc::now().to_rfc2822(),
        fen = m.initial_pos().as_fen(),
        p1 = m.player_name(B::Color::first()).unwrap_or("??".to_string()),
        p2 = m
            .player_name(B::Color::second())
            .unwrap_or("??".to_string()),
        p1_name = B::Color::first(),
        p2_name = B::Color::second(),
    );
    let mut board = m.initial_pos();
    for (ply, mov) in m.move_history().iter().enumerate() {
        let mov_str = mov.extended_formatter(board, Standard);
        if ply % 2 == 0 {
            res += &format!("\n{}. {mov_str}", (ply + 1) / 2 + 1);
        } else {
            if ply == 0 && !m.initial_pos().active_player().is_first() {
                res += &format!("\n1... {mov_str}");
            }
            res += &format!(" {mov_str}");
        }
        board = board.make_move(*mov).unwrap();
    }
    if let Over(x) = m.match_status() {
        if !matches!(x.result, GameResult::Aborted) {
            res.push(' ');
            res += &result;
        }
    }
    res
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UnknownTagPair {
    pub tag: String,
    pub value: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RoundNumber {
    Number(isize),
    Unknown,
    Unimportant,
    Custom(String),
}

impl Display for RoundNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Number(n) => n.to_string(),
            Unknown => "?".to_string(),
            Unimportant => "-".to_string(),
            Custom(s) => s.clone(),
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, derive_more::Display)]
pub enum PlayerType {
    Human,
    Program,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TagPair {
    Event(String),
    Site(String),
    Date(String),
    Round(RoundNumber),
    White(String),
    Black(String),
    Result(GameResult),
    WhiteElo(isize),
    BlackElo(isize),
    WhiteTitle(String),
    BlackTitle(String),
    WhiteType(PlayerType),
    BlackType(PlayerType),
    Other(UnknownTagPair),
    SetUp(bool),
    Fen(String),
}

impl TagPair {
    pub fn parse(tag: String, value: String) -> Res<Self> {
        Ok(match tag.as_str() {
            "Event" => Event(value),
            "Site" => Site(value),
            "Date" => Date(value),
            "Round" => {
                let value = value.trim_ascii();
                Round(if value == "?" {
                    Unknown
                } else if value == "-" {
                    Unimportant
                } else if let Ok(n) = value.parse::<isize>() {
                    Number(n)
                } else {
                    Custom(value.to_string())
                })
            }
            "White" => White(value),
            "Black" => Black(value),
            "Result" => Result(GameResult::from_str(value.trim_ascii())?),
            _ => Other(UnknownTagPair { tag, value }),
        })
    }

    fn value(&self) -> String {
        match self {
            Event(value) => value.clone(),
            Site(value) => value.clone(),
            Date(value) => value.clone(),
            Round(value) => value.to_string(),
            White(value) => value.clone(),
            Black(value) => value.clone(),
            Result(value) => value.to_canonical_string(),
            TagPair::WhiteElo(value) => value.to_string(),
            TagPair::BlackElo(value) => value.to_string(),
            TagPair::WhiteTitle(value) => value.clone(),
            TagPair::BlackTitle(value) => value.clone(),
            TagPair::WhiteType(value) => value.to_string(),
            TagPair::BlackType(value) => value.to_string(),
            Other(value) => value.value.clone(),
            TagPair::SetUp(value) => value.to_string(),
            TagPair::Fen(value) => value.clone(),
        }
    }
}

#[derive(Debug, Default, Clone)]
struct PgnData<B: Board> {
    tag_pairs: Vec<TagPair>,
    game: MatchState<B>,
}

struct PgnParser<'a, B: Board> {
    first_in_line: bool,
    byte_idx: usize,
    original_input: &'a str,
    unread: Peekable<Chars<'a>>,
    res: PgnData<B>,
}

impl<'a, B: Board> PgnParser<'a, B> {
    fn new(input: &'a str) -> Self {
        Self {
            first_in_line: true,
            byte_idx: 0,
            original_input: input,
            unread: input.chars().peekable(),
            res: PgnData::default(),
        }
    }

    fn is_symbol_char(c: char) -> bool {
        c.is_ascii_digit() || c.is_alphabetic() || matches!(c, '_' | '+' | '#' | '=' | ':' | '-')
    }

    fn eat(&mut self) -> Option<char> {
        self.first_in_line = self.unread.peek().is_some_and(|&c| c == '\n');
        let res = self.unread.next();
        if let Some(c) = res {
            self.byte_idx += c.len_utf8();
        }
        res
    }

    fn ignore_percent_comment(&mut self) -> bool {
        if self.first_in_line && self.unread.peek().is_some_and(|&c| c == '%') {
            loop {
                self.eat();
                if self.first_in_line {
                    return true;
                }
            }
        }
        false
    }

    fn ignore_whitespace(&mut self) -> Res<()> {
        while let Some(&c) = self.unread.peek() {
            if self.ignore_percent_comment() {
                continue;
            }
            if c == '{' {
                self.parse_brace_comment()?;
                continue;
            }
            if !c.is_whitespace() {
                return Ok(());
            }
            self.eat();
        }
        Ok(())
    }

    fn parse_brace_comment(&mut self) -> Res<()> {
        assert!(self.unread.peek().is_some_and(|&c| c == '{'));
        self.eat();
        while let Some(&c) = self.unread.peek() {
            if self.ignore_percent_comment() {
                continue;
            }
            self.eat();
            if c == '}' {
                return Ok(());
            }
        }
        bail!("Unclosed brace '{{'")
    }

    fn parse_tag_pair(&mut self) -> Res<TagPair> {
        debug_assert!(self.unread.peek().is_some_and(|&c| c == '['));
        self.eat();
        self.ignore_whitespace()?;
        let mut name = String::new();
        while let Some(&c) = self.unread.peek() {
            if c.is_alphanumeric() || c == '_' {
                name.push(c);
                self.eat().unwrap();
            } else {
                break;
            }
        }
        if name.is_empty() {
            bail!("Empty tag after starting a tag pair with '['")
        }
        self.ignore_whitespace()?;
        if !self.unread.peek().is_some_and(|&c| c == '"') {
            bail!("Expected the tag value to start with a quote ('\"')")
        }
        self.eat();
        let mut value = String::new();
        while let Some(c) = self.eat() {
            if c == '\\' {
                let Some(next) = self.eat() else {
                    bail!("Input ends after a backslash while in a string in a tag pair")
                };
                value.push(next);
            } else if c == '"' {
                break;
            }
            value.push(c);
        }
        self.ignore_whitespace()?;
        if !self.unread.peek().is_some_and(|&c| c == ']') {
            bail!("Expected the tag pair to end with a closing bracket (']')")
        }
        self.eat();
        TagPair::parse(name, value)
    }

    fn parse_all_tag_pairs(&mut self) -> Res<()> {
        self.ignore_whitespace()?;
        while let Some(&c) = self.unread.peek() {
            if c == '[' {
                let tag_pair = self.parse_tag_pair()?;
                self.res.tag_pairs.push(tag_pair);
                self.ignore_whitespace()?;
            } else {
                break;
            }
        }
        Ok(())
    }

    // TODO: Support for Variations with (moves)
    fn parse_move(&mut self) -> Res<()> {
        self.ignore_whitespace()?;
        if self.unread.peek().is_none() {
            return Ok(());
        }
        let string = &self.original_input[self.byte_idx..];
        let next_word = string.split_ascii_whitespace().next().unwrap_or_default();
        if let Ok(result) = GameResult::from_str(next_word) {
            self.res.game.status = Run(Over(MatchResult {
                result,
                reason: GameOverReason::Normal,
            }));
            for _ in 0..next_word.len() {
                self.eat().unwrap();
            }
            return Ok(());
        }
        if self.unread.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.eat();
            while self.unread.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.eat();
            }
            self.ignore_whitespace()?;
            while self.unread.peek().is_some_and(|&c| c == '.') {
                self.eat();
            }
            self.ignore_whitespace()?;
        }
        let string = &self.original_input[self.byte_idx..];
        if let Run(Over(_)) = self.res.game.status {
            bail!(
                "The game has already ended, cannot parse additional moves at start of '{}'",
                string.important()
            )
        }
        let prev_board = &self.res.game.board;
        let (remaining, mov) = B::Move::parse_extended_text(string, prev_board)?;
        let Some(new_board) = prev_board.make_move(mov) else {
            bail!("Illegal psuedolegal move '{}'", mov.to_string().error());
        };
        self.res.game.board_hist.push(prev_board);
        self.res.game.mov_hist.push(mov);
        self.res.game.board = new_board;
        if let Some(res) = self
            .res
            .game
            .board
            .match_result_slow(&self.res.game.board_hist)
        {
            if let Run(st) = &mut self.res.game.status {
                *st = Over(res);
            }
        }
        for _ in 0..string.len() - remaining.len() {
            self.eat().unwrap();
        }
        Ok(())
    }

    fn parse_all_moves(&mut self) -> Res<()> {
        while self.unread.peek().is_some() {
            self.parse_move()?;
        }
        Ok(())
    }

    fn parse(&mut self) -> Res<PgnData<B>> {
        self.parse_all_tag_pairs()?;
        self.parse_all_moves()?;
        Ok(take(&mut self.res))
    }
}

pub fn parse_pgn<B: Board>(pgn: &str) -> Res<PgnData<B>> {
    let mut parser: PgnParser<'_, B> = PgnParser::new(pgn);
    parser.parse()
}

mod tests {
    use super::*;
    use crate::games::chess::moves::ChessMove;
    use crate::games::chess::pieces::ChessPieceType::Bishop;
    use crate::games::chess::squares::ChessSquare;
    use crate::games::chess::Chessboard;

    #[test]
    fn parse_one_ply_pgn() {
        let pgn = "1. e4";
        let mut parser: PgnParser<'_, Chessboard> = PgnParser::new(pgn);
        let data = parser.parse().unwrap();
        let pos = Chessboard::default();
        let pos = pos
            .make_move(ChessMove::from_text("e4", &pos).unwrap())
            .unwrap();
        assert_eq!(data.game.pos_before_moves, Chessboard::default());
        assert_eq!(data.game.mov_hist.len(), 1);
        assert_eq!(data.game.board, pos);
        assert_eq!(
            data.game.mov_hist[0],
            ChessMove::from_text("e4", &Chessboard::default()).unwrap()
        );
        assert_eq!(
            data.game.board_hist.0[0],
            Chessboard::default().zobrist_hash()
        );
    }

    #[test]
    fn parse_two_ply_pgn() {
        let pgn = "{this}1e4{is} \n%a\nd5 {test}";
        let mut parser: PgnParser<'_, Chessboard> = PgnParser::new(pgn);

        let data = parser.parse().unwrap();
        let pos = Chessboard::default();
        let pos = pos
            .make_move(ChessMove::from_text("e4", &pos).unwrap())
            .unwrap();
        let pos = pos
            .make_move(ChessMove::from_text("d5", &pos).unwrap())
            .unwrap();
        assert_eq!(data.game.mov_hist.len(), 2);
        assert_eq!(data.game.pos_before_moves, Chessboard::default());
        assert_eq!(data.game.board, pos);
        assert!(data.tag_pairs.is_empty());
    }

    #[test]
    // pgn adapted from https://en.wikipedia.org/wiki/Portable_Game_Notation
    fn parse_simple_pgn() {
        let pgn = r#"%
[Event "F/S Return Match"]
[Site "Belgrade, Serbia JUG"{}]
[{Result} Date "1992.11.04"]
[Round
%
"29"]
[White  "Fischer, Robert J."]
[ Black "Spassky, Boris V."]
[Result{} "1/2-1/2"]

1.e4 e5 2.Nf3 Nc6 3.Bb5 {This opening is called the Ruy Lopez.} 3...a6
4.Ba4 Nf6 5.O-O Be7 6.Re1 b5 7.Bb3 d6 8.c3 O-O 9.h3 Nb8 10.d4 Nbd7
%test%\
11.c4 c6 12.cxb5 axb5 13.Nc3 Bb7 14.Bg5 b4 15.Nb1 h6 16.Bh4 c5 17.dxe5
Nxe4 18.Bxe7 Qxe7 19.exd6 Qf6 20.Nbd2 Nxd6 21.Nc4 Nxc4 22.Bxc4 Nb6
23.Ne5 Rae8 24.Bxf7+ Rxf7 25.Nxf7 Rxe1+ 26.Qxe1 Kxf7 27.Qe3 Qg5 28.Qxg5
hxg5 29.b3 Ke6 30.a3 Kd6 31.axb4 cxb4 32.Ra5 Nd5 33.f3 Bc8 34.Kf2 Bf5
{another test %}
{yet another
% test}}}}"\\\
 %}
35.Ra7 g6 36.Ra6+ Kc5 37.Ke1{}Nf4 38.g3 Nxh3 39.Kd2 Kb5 40.Rd6 Kc5 41.Ra6
Nf2 42.g4 Bd3 43.Re6 1/2-1/2"#;
        let mut parser: PgnParser<'_, Chessboard> = PgnParser::new(pgn);
        let data = parser.parse().unwrap();
        assert_eq!(data.tag_pairs.len(), 7);
        assert!(matches!(data.tag_pairs[0], Event(_)));
        assert_eq!(data.tag_pairs[0], Event("F/S Return Match".to_string()));
        assert_eq!(data.tag_pairs[1], Site("Belgrade, Serbia JUG".into()));
        assert_eq!(data.tag_pairs[2], Date("1992.11.04".into()));
        assert_eq!(data.tag_pairs[3], Round(Number(29)));
        assert_eq!(data.tag_pairs[4], White("Fischer, Robert J.".into()));
        assert_eq!(data.tag_pairs[5], Black("Spassky, Boris V.".into()));
        assert_eq!(data.tag_pairs[6], Result(GameResult::Draw));
        assert_eq!(data.game.pos_before_moves, Chessboard::default());
        assert_eq!(data.game.mov_hist.len(), 42 * 2 + 1);
        assert_eq!(data.game.board_hist.len(), data.game.mov_hist.len());
        assert_eq!(
            data.game.mov_hist[42].dest_square(),
            ChessSquare::from_chars('c', '4').unwrap()
        );
        assert_eq!(data.game.mov_hist[42].piece_type(), Bishop);
    }
}
