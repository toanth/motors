use std::io::stdin;

use crate::games::{Board, Move};
use crate::play::{AdjudicationReason, GameOverReason, GameResult, MatchManager, MatchStatus};
use crate::ui::Message::Error;
use crate::ui::{Graphics, Message, UI};

pub fn display_message(typ: Message, message: &str) {
    println!("{0}{message}", typ.message_prefix());
}

pub fn get_move<B: Board, U: UI<B>>(ui: &mut U, board: &B) -> B::Move {
    loop {
        let mut input = String::new();
        let read = stdin().read_line(&mut input);
        if read.is_err() {
            ui.display_message(
                Error,
                format!("Couldn't get input: {}", read.err().unwrap().to_string()).as_str(),
            );
            continue;
        }
        let res = B::Move::from_text(&input, &board);
        if res.is_ok() {
            return res.unwrap();
        }
        ui.display_message(
            Error,
            format!(
                "Input '{0}' is not a valid move: {1}",
                input.trim(),
                res.err().unwrap_or("Unknown error".to_string())
            )
            .as_str(),
        );
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub enum DisplayType {
    #[default]
    Unicode,
    Ascii,
    Fen,
    Pgn,
    Uci,
}

#[derive(Debug, Default)]
pub struct TextUI {
    typ: DisplayType,
}

impl TextUI {
    pub fn new(typ: DisplayType) -> Self {
        Self { typ }
    }
}

fn match_to_pgn<B: Board>(m: &dyn MatchManager<B>) -> String {
    let result = match m.match_status() {
        MatchStatus::Over(r) => match r.result {
            GameResult::P1Win => "\"1-0\"",
            GameResult::P2Win => "\"0-1\"",
            GameResult::Draw => "\"1/2-1/2\"",
            GameResult::Aborted => "\"??\"",
        },
        _ => "\"??\"",
    };
    let termination = match m.match_status() {
        MatchStatus::NotStarted => "\"not started\"",
        MatchStatus::Ongoing => "\"unterminated\"",
        MatchStatus::Over(res) => match res.reason {
            GameOverReason::Normal => "\"normal\"",
            GameOverReason::Adjudication(reason) => match reason {
                AdjudicationReason::TimeUp => "\"time forfeit\"",
                AdjudicationReason::InvalidMove => "\"rules infraction\"",
                AdjudicationReason::AbortedByUser => "\"abandoned\"",
                AdjudicationReason::EngineError => "\"emergency\"",
            },
        },
    };
    let mut res = format!(
        "[Event \"'motors' {game} match\"]\n\
        [Site \"github.com/toanth/motors\"]\n\
        [Date \"{date}\"]\n\
        [Round \"1\"]\n\
        [White \"??\"]\n\
        [Black \"??\"]\n\
        [Result \"{result}\"]\n\
        [TimeControl \"??\"]\n\
        [Termination \"{termination}\"]\n\
        [Variant \"From Position\"]\n\
        [FEN \"{fen}\"]\n\
        ; automatically generated '{game}' pgn",
        game = m.game_name(),
        date = chrono::offset::Utc::now(),
        fen = m.initial_pos().as_fen()
    );
    let mut board = m.initial_pos();
    for (ply, mov) in m.move_hist().iter().enumerate() {
        let mov_str = mov.to_extended_text(&board);
        if ply % 2 == 0 {
            res += &format!("\n{}. {mov_str}", ply / 2 + 1);
        } else {
            res += &format!(" {mov_str}")
        }
        board = board.make_move(*mov).unwrap();
    }
    res
}

fn match_to_uci<B: Board>(m: &dyn MatchManager<B>) -> String {
    let mut res = format!("position fen {} moves ", m.initial_pos().as_fen());
    for mov in m.move_hist() {
        res += mov.to_compact_text().as_str();
        res.push(' ');
    }
    res
}

impl<B: Board> Graphics<B> for TextUI {
    fn show(&mut self, m: &dyn MatchManager<B>) {
        let message = match self.typ {
            DisplayType::Ascii => m.board().as_ascii_diagram(),
            DisplayType::Unicode => m.board().as_unicode_diagram(),
            DisplayType::Fen => m.board().as_fen(),
            DisplayType::Pgn => match_to_pgn(m),
            DisplayType::Uci => match_to_uci(m),
        };
        println!("{message}");
    }

    fn display_message(&mut self, typ: Message, message: &str) {
        display_message(typ, message)
    }
}

impl<B: Board> UI<B> for TextUI {
    fn get_move(&mut self, board: &B) -> B::Move {
        get_move::<B, TextUI>(self, board)
    }
}
