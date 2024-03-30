use std::fs::File;
use std::io::{Stderr, stderr, Stdout, stdout, Write};
use std::path::Path;
use std::str::SplitWhitespace;
use crate::games::{Board, Move};

use crate::{AdjudicationReason, GameOverReason, GameResult, GameState, MatchStatus};
use crate::games::Color::{Black, White};
use crate::general::common::{NamedEntity, Res};
use crate::MatchStatus::Ongoing;
use crate::output::{Message, Output, OutputBox, OutputBuilder};
use crate::output::Message::{Debug, Error};



#[derive(Debug)]
pub enum TextStream {
    File(File), // Don't use a BufWriter to ensure the log is always up-to-date.
    Stdout(Stdout),
    Stderr(Stderr),
}

impl TextStream {
    pub fn write(&mut self, prefix: &str, msg: &str) {
        _ = writeln!(self.stream(), "{prefix} {msg}")
    }

    pub fn stream(&mut self) -> &mut dyn Write {
        match self {
            TextStream::File(f) => f,
            TextStream::Stdout(out) => out,
            TextStream::Stderr(err) => err,
        }
    }

    pub fn from_words(mut words: SplitWhitespace, fallback_name: &str) -> Res<Self> {
        let stream = match words.next().unwrap_or_default() {
            "" => return Self::from_words(fallback_name.split_whitespace(), ""),
            "stdout" => TextStream::Stdout(stdout()),
            "stderr" => TextStream::Stderr(stderr()),
            s => TextStream::from_filename(s)?,
        };
        Ok(stream)
    }

    pub fn from_filename(name: &str) -> Res<Self> {
        if !name.contains('.') {
            // Although files of course don't have to contain a '.', requiring that feels like a good way to
            // catch errors like typos where the user didn't mean to specify a file name.
            return Err(format!("'{name}' does not appear to be a valid log filename (it does not contain a '.'). \
                Expected either a filename, 'stdout', 'stderr', or 'none'."));
        }
        let path = Path::new(name);
        let file = File::create(path).map_err(|err| format!("Couldn't create log file: {err}"))?;
        Ok(TextStream::File(file))
    }
}

#[derive(Debug)]
pub struct TextWriter {
    normal: TextStream,
    error: Option<TextStream>,
}

impl TextWriter {
    pub fn display_message(&mut self, typ: Message, message: &str) {
        if self.error.is_some() && (typ == Error || typ == Debug) {
            if let Some(ref mut error) = self.error {
                error.write(typ.message_prefix(), message);
                return
            }
        }
        self.normal.write(typ.message_prefix(), message);
    }

    pub fn file(out: File) -> Self {
        Self::new(TextStream::File(out))
    }

    pub fn new(out: TextStream) -> Self {
        Self { normal: out, error: None }
    }

    pub fn new_with_err(out: TextStream, err: TextStream) -> Self {
        Self { normal: out, error: Some(err) }
    }
}

impl Default for TextWriter {
    fn default() -> Self {
        Self::new_with_err(TextStream::Stdout(stdout()), TextStream::Stderr(stderr()))
    }
}

// TODO: Option to flip the board so that it's viewed from the perspecive of the current player

// pub fn display_message(typ: Message, message: &str) {
//     if typ == Error || typ == Debug {
//         eprintln!("{0}{message}", typ.message_prefix());
//     } else {
//         println!("{0}{message}", typ.message_prefix());
//     }
// }

#[derive(Default, Debug, Copy, Clone)]
pub enum DisplayType {
    #[default]
    Unicode,
    Ascii,
    Fen,
    Pgn,
    Uci,
    Ugi, // The same as `UCI`, but with a different name so that the user can write both 'print uci' and 'print ugi'
}

impl NamedEntity for DisplayType {
    fn short_name(&self) -> &str {
        match self {
            DisplayType::Unicode => "unicode",
            DisplayType::Ascii => "ascii",
            DisplayType::Fen => "fen",
            DisplayType::Pgn => "pgn",
            DisplayType::Uci => "uci",
            DisplayType::Ugi => "ugi",
        }
    }

    fn long_name(&self) -> &str {
        match self {
            DisplayType::Unicode => "Unicode Diagram",
            DisplayType::Ascii => "ASCII Diagram",
            DisplayType::Fen => "Fen",
            DisplayType::Pgn => "PGN",
            DisplayType::Uci => "UCI",
            DisplayType::Ugi => "UGI",
        }
    }

    fn description(&self) -> Option<&str> {
        Some(match self {
            DisplayType::Unicode => "A textual 2D representation of the position using unicode characters. For many games, this is the same as the ASCII representation, but e.g. for chess it uses chess symbols like '♔'.",
            DisplayType::Ascii => "A textual 2D representation of the position using \"normal\" english characters. E.g. for chess, this represents the white king as 'K' and a black queen as 'q'.",
            DisplayType::Fen => "A compact textual representation of the position. For chess, this is the Forsyth–Edwards Notation, and for other games it's a similar notation based on chess FENs.",
            DisplayType::Pgn => "A textual representation of the entire match. For chess, this is the Portable Games Notation, and for other games it's a similar notation based on chess PGNs.",
            DisplayType::Uci => "A textual representation of the match using the machine-readable UGI notation that gets used for engine-GUI communication. UCI for chess and the very slightly different UGI protocol for other games.",
            DisplayType::Ugi => "Same as 'UCI'",
        })
    }
}

#[derive(Debug)]
struct TextOutput {
    typ: DisplayType,
    is_engine: bool,
    writer: TextWriter,
}

impl TextOutput {

    fn with_writer(typ: DisplayType, is_engine: bool, writer: TextWriter) -> Self {
        Self {typ, is_engine, writer }
    }

    pub fn match_to_pgn<B: Board>(&self, m: &dyn GameState<B>) -> String {
        let result = match m.match_status() {
            MatchStatus::Over(r) => match r.result {
                GameResult::P1Win => "1-0",
                GameResult::P2Win => "0-1",
                GameResult::Draw => "1/2-1/2",
                GameResult::Aborted => "??",
            },
            _ => "\"??\"",
        };
        let status = m.match_status();
        let termination = match &status {
            MatchStatus::NotStarted => "not started",
            MatchStatus::Ongoing => "unterminated",
            MatchStatus::Over(ref res) => match res.reason {
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
        [White \"{white}\"]\n\
        [Black \"{black}\"]\n\
        [Result \"{result}\"]\n\
        [TimeControl \"??\"]\n\
        [Termination \"{termination}\"]\n\
        [Variant \"From Position\"]\n\
        [FEN \"{fen}\"]\n\
        ; automatically generated {game} pgn",
            game = m.game_name(),
            event = m.event(),
            site = m.site(),
            date = chrono::offset::Utc::now().to_rfc2822(),
            fen = m.initial_pos().as_fen(),
            white = m.player_name(White).unwrap_or("??"),
            black = m.player_name(Black).unwrap_or("??"),
        );
        let mut board = m.initial_pos();
        for (ply, mov) in m.move_history().iter().enumerate() {
            let mov_str = mov.to_extended_text(&board);
            if ply % 2 == 0 {
                res += &format!("\n{}. {mov_str}", ply / 2 + 1);
            } else {
                res += &format!(" {mov_str}")
            }
            board = board.make_move(*mov).unwrap();
        }
        if let MatchStatus::Over(x) =m.match_status() {
            if !matches!(x.result, GameResult::Aborted) {
                res += &(" ".to_string() + result);
            }
        }
        res
    }

    fn match_to_ugi<B: Board>(m: &dyn GameState<B>) -> String {
        let pos = m.initial_pos().as_fen();
        if m.move_history().is_empty() {
            format!("position fen {pos}")
        } else {
            let mut res = format!("position fen {} moves ", m.initial_pos().as_fen());
            for mov in m.move_history() {
                res += mov.to_compact_text().as_str();
                res.push(' ');
            }
            res
        }
    }
}

impl NamedEntity for TextOutput {
    fn short_name(&self) -> &str {
        self.typ.short_name()
    }

    fn long_name(&self) -> &str {
        self.typ.long_name()
    }

    fn description(&self) -> Option<&str> {
        self.typ.description()
    }
}

impl<B: Board> Output<B> for TextOutput {

    fn as_string(&self, m: &dyn GameState<B>) -> String {
        // TODO: Option to flip the board?
        let mut white_time = String::default();
        let mut black_time = String::default();
        if m.match_status() == Ongoing {
            white_time = m.time(White).map(|tc| tc.remaining_to_string(m.thinking_since(White))).unwrap_or_default();
            black_time = m.time(Black).map(|tc| tc.remaining_to_string(m.thinking_since(Black))).unwrap_or_default();
        }
        match self.typ {
            DisplayType::Ascii => format!("{black_time}{}{white_time}", m.get_board().as_ascii_diagram()),
            DisplayType::Unicode => format!("{black_time}{}{white_time}", m.get_board().as_unicode_diagram()),
            DisplayType::Fen => m.get_board().as_fen(),
            DisplayType::Pgn => self.match_to_pgn(m),
            DisplayType::Uci | DisplayType::Ugi => TextOutput::match_to_ugi(m),
        }
    }

    fn display_message_simple(&mut self, typ: Message, message: &str) {
        self.writer.display_message(typ, message)
    }
}

#[derive(Default, Clone, Debug)]
pub struct TextOutputBuilder {
    typ: DisplayType,
    options: Vec<String>,
}

impl TextOutputBuilder {
    pub fn new(typ: DisplayType) -> Self {
        Self { typ, options: vec![] }
    }
    pub fn build<B: Board>(&self, is_engine: bool) -> Res<OutputBox<B>> {
        let mut stream = TextStream::Stdout(stdout());
        let mut err_stream = TextStream::Stderr(stderr());
        for option in &self.options {
            if let Some(file) = option.strip_prefix("file=") {
                stream = TextStream::from_filename(file)?;
            } else if let Some(err) = option.strip_prefix("err=") {
                err_stream = TextStream::from_filename(err)?;
            } else {
                return Err(format!("Unrecognized option '{option}' for output {}", self.typ.long_name()))
            }
        }
        Ok(Box::new(TextOutput::with_writer(self.typ, is_engine, TextWriter::new_with_err(stream, err_stream))))
    }
}

impl NamedEntity for TextOutputBuilder {
    fn short_name(&self) -> &str {
        self.typ.short_name()
    }

    fn long_name(&self) -> &str {
        self.typ.long_name()
    }

    fn description(&self) -> Option<&str> {
        self.typ.description()
    }
}

impl<B: Board> OutputBuilder<B> for TextOutputBuilder {
    fn for_engine(&self, _state: &dyn GameState<B>) -> Res<OutputBox<B>> {
        self.build(true)
    }

    fn for_client(&self, _state: &dyn GameState<B>) -> Res<OutputBox<B>> {
        self.build(false)
    }

    fn add_option(&mut self, option: String) -> Res<()> {
        self.options.push(option);
        Ok(())
    }
}
