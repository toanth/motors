use crate::games::{Color, ColoredPiece, DimT, Settings};
use crate::general::board::{Board, RectangularBoard};
use crate::general::common::{NamedEntity, Res};
use crate::general::move_list::MoveList;
use crate::general::moves::ExtendedFormat::{Alternative, Standard};
use crate::general::moves::Move;
use crate::general::squares::RectangularCoordinates;
use crate::output::text_output::DisplayType::*;
use crate::output::{AbstractOutput, Message, Output, OutputBox, OutputBuilder};
use crate::MatchStatus::*;
use crate::{AdjudicationReason, GameOverReason, GameResult, GameState};
use anyhow::{anyhow, bail};
use crossterm::style;
use crossterm::style::Stylize;
use std::fmt;
use std::fs::File;
use std::io::{stderr, stdout, Stderr, Stdout, Write};
use std::mem::swap;
use std::path::Path;
use std::str::SplitWhitespace;
use strum_macros::EnumIter;

#[derive(Debug)]
pub enum TextStream {
    File(File, String), // Don't use a BufWriter to ensure the log is always up-to-date.
    Stdout(Stdout),
    Stderr(Stderr),
}

impl TextStream {
    pub fn write(&mut self, prefix: &str, msg: &str) {
        _ = writeln!(self.stream(), "{prefix} {msg}");
    }

    pub fn stream(&mut self) -> &mut dyn Write {
        match self {
            TextStream::File(f, _) => f,
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
            bail!(
                "'{name}' does not appear to be a valid log filename (it does not contain a '.'). \
                Expected either a filename, 'stdout', 'stderr', or 'none'."
            );
        }
        let path = Path::new(name);
        let file = File::create(path).map_err(|err| anyhow!("Couldn't create log file: {err}"))?;
        Ok(TextStream::File(
            file,
            path.canonicalize()
                .ok()
                .as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or(name)
                .to_string(),
        ))
    }

    pub fn name(&self) -> String {
        match self {
            TextStream::File(_, name) => name.clone(),
            TextStream::Stdout(_) => "stdout".to_string(),
            TextStream::Stderr(_) => "stderr".to_string(),
        }
    }
}

#[derive(Debug)]
#[must_use]
pub struct TextWriter {
    pub stream: TextStream,
    pub accepted: Vec<Message>,
}

impl TextWriter {
    pub fn display_message(&mut self, typ: Message, message: &str) {
        if self.accepted.contains(&typ) {
            self.stream.write(typ.message_prefix(), message);
        }
    }

    pub fn new_for(stream: TextStream, accepted: Vec<Message>) -> Self {
        Self { stream, accepted }
    }
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, EnumIter)]
pub enum DisplayType {
    #[default]
    Pretty,
    Unicode,
    Ascii,
    Fen,
    Pgn,
    Moves, // Prints all legal moves
    Uci,
    Ugi, // The same as `UCI`, but with a different name so that the user can write both 'print uci' and 'print ugi'
    MsgOnly, // Doesn't print the state at all, but a text output with that display type would still display messages.
}

impl NamedEntity for DisplayType {
    fn short_name(&self) -> String {
        match self {
            Pretty => "pretty",
            Unicode => "unicode",
            Ascii => "ascii",
            Fen => "fen",
            Pgn => "pgn",
            Moves => "moves",
            Uci => "uci",
            Ugi => "ugi",
            MsgOnly => "messages",
        }
        .to_string()
    }

    fn long_name(&self) -> String {
        match self {
            Pretty => "Pretty Text Diagram",
            Unicode => "Unicode Diagram",
            Ascii => "ASCII Diagram",
            Fen => "Fen",
            Pgn => "PGN",
            Moves => "Moves",
            Uci => "UCI",
            Ugi => "UGI",
            MsgOnly => "Only Messages",
        }
        .to_string()
    }

    fn description(&self) -> Option<String> {
        Some(match self {
            Pretty => "A textual 2D representation of the position that's meant to look pretty. ",
            Unicode => "A textual 2D representation of the position using unicode characters. For many games, this is the same as the ASCII representation, but e.g. for chess it uses chess symbols like '♔'",
            Ascii => "A textual 2D representation of the position using \"normal\" english characters. E.g. for chess, this represents the white king as 'K' and a black queen as 'q'",
            Fen => "A compact textual representation of the position. For chess, this is the Forsyth–Edwards Notation, and for other games it's a similar notation based on chess FENs",
            Pgn => "A textual representation of the entire match. For chess, this is the Portable Games Notation, and for other games it's a similar notation based on chess PGNs",
            Moves => "A space-separated list of all legal moves, intended mostly for debugging",
            Uci => "A textual representation of the match using the machine-readable UGI notation that gets used for engine-GUI communication. UCI for chess and the very slightly different UGI protocol for other games",
            Ugi => "Same as 'UCI'",
            MsgOnly => "Doesn't print the match or current position at all, but will display messages",
        }.to_string())
    }
}

#[derive(Debug)]
pub struct BoardToText {
    pub typ: DisplayType,
    pub is_engine: bool,
}

impl BoardToText {
    fn match_to_pgn<B: Board>(m: &dyn GameState<B>) -> String {
        let result = match m.match_status() {
            Over(r) => match r.result {
                GameResult::P1Win => "1-0",
                GameResult::P2Win => "0-1",
                GameResult::Draw => "1/2-1/2",
                GameResult::Aborted => "??",
            },
            _ => "\"??\"",
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
                res += &format!("\n{}. {mov_str}", ply / 2 + 1);
            } else {
                res += &format!(" {mov_str}");
            }
            board = board.make_move(*mov).unwrap();
        }
        if let Over(x) = m.match_status() {
            if !matches!(x.result, GameResult::Aborted) {
                res += &(" ".to_string() + result);
            }
        }
        res
    }

    fn list_moves<B: Board>(m: &dyn GameState<B>) -> String {
        use fmt::Write;
        let mut res = String::default();
        let pos = m.get_board();
        for mov in pos.legal_moves_slow().iter_moves() {
            write!(&mut res, "{} ", mov.to_extended_text(&pos, Alternative)).unwrap();
        }
        res
    }

    fn match_to_ugi<B: Board>(m: &dyn GameState<B>) -> String {
        use std::fmt::Write;
        let pos = m.initial_pos().as_fen();
        if m.move_history().is_empty() {
            format!("position fen {pos}")
        } else {
            let mut res = format!("position fen {} moves ", m.initial_pos().as_fen());
            for mov in m.move_history() {
                write!(&mut res, "{mov} ").unwrap();
            }
            res
        }
    }

    pub fn as_string<B: Board>(&self, m: &dyn GameState<B>) -> String {
        let mut time_below = String::default();
        let mut time_above = String::default();
        if m.match_status() == Ongoing {
            time_below = m
                .time(B::Color::first())
                .map(|tc| tc.remaining_to_string(m.thinking_since(B::Color::first())))
                .unwrap_or_default();
            time_above = m
                .time(B::Color::second())
                .map(|tc| tc.remaining_to_string(m.thinking_since(B::Color::second())))
                .unwrap_or_default();
        }
        let flipped = m.active_player() == B::Color::second();
        if flipped {
            swap(&mut time_below, &mut time_above);
        }
        match self.typ {
            Pretty => {
                let mut formatter = m.get_board().pretty_formatter(m.last_move(), flipped);
                format!(
                    "{time_above}{}{time_below}",
                    m.get_board().display_pretty(formatter.as_mut())
                )
            }
            Ascii => format!(
                "{time_above}{}{time_below}",
                m.get_board().as_ascii_diagram(flipped)
            ),
            Unicode => format!(
                "{time_above}{}{time_below}",
                m.get_board().as_unicode_diagram(flipped)
            ),
            Fen => m.get_board().as_fen(),
            Pgn => Self::match_to_pgn(m),
            Moves => Self::list_moves(m),
            Uci | Ugi => BoardToText::match_to_ugi(m),
            MsgOnly => String::default(),
        }
    }
}

#[derive(Debug)]
struct TextOutput {
    writer: TextWriter,
    to_text: BoardToText,
    name: Option<String>,
}

impl TextOutput {
    fn new(typ: DisplayType, is_engine: bool, writer: TextWriter, name: Option<String>) -> Self {
        Self {
            to_text: BoardToText { typ, is_engine },
            writer,
            name,
        }
    }
}

impl NamedEntity for TextOutput {
    fn short_name(&self) -> String {
        self.name.clone().unwrap_or(self.to_text.typ.short_name())
    }

    fn long_name(&self) -> String {
        self.to_text.typ.long_name()
    }

    fn description(&self) -> Option<String> {
        self.to_text.typ.description()
    }
}

impl AbstractOutput for TextOutput {
    fn prints_board(&self) -> bool {
        self.to_text.typ != MsgOnly
    }

    fn output_name(&self) -> String {
        self.writer.stream.name()
    }

    fn display_message(&mut self, typ: Message, message: &str) {
        self.writer.display_message(typ, message);
    }
}

impl<B: Board> Output<B> for TextOutput {
    fn as_string(&self, m: &dyn GameState<B>) -> String {
        self.to_text.as_string(m)
    }
}

#[derive(Default, Debug)]
#[must_use]
pub struct TextOutputBuilder {
    pub typ: DisplayType,
    pub stream: Option<TextStream>,
    pub accepted: Vec<Message>,
    short_name: Option<String>,
}

impl Clone for TextOutputBuilder {
    fn clone(&self) -> Self {
        Self {
            typ: self.typ,
            stream: None,
            accepted: self.accepted.clone(),
            short_name: self.short_name.clone(),
        }
    }
}

impl TextOutputBuilder {
    pub fn new(typ: DisplayType) -> Self {
        Self {
            typ,
            stream: None,
            accepted: vec![],
            short_name: None,
        }
    }

    pub fn messages_for(accepted: Vec<Message>, short_name: &str) -> Self {
        Self {
            typ: MsgOnly,
            stream: None,
            accepted,
            short_name: Some(short_name.to_string()),
        }
    }

    pub fn build<B: Board>(&mut self, is_engine: bool) -> Res<OutputBox<B>> {
        let stream = self
            .stream
            .take()
            .unwrap_or_else(|| TextStream::Stderr(stderr()));
        Ok(Box::new(TextOutput::new(
            self.typ,
            is_engine,
            TextWriter::new_for(stream, self.accepted.clone()),
            self.short_name.clone(),
        )))
    }
}

impl NamedEntity for TextOutputBuilder {
    fn short_name(&self) -> String {
        self.short_name
            .clone()
            .unwrap_or_else(|| self.typ.short_name())
    }

    fn long_name(&self) -> String {
        self.short_name
            .clone()
            .map_or_else(|| self.typ.long_name(), |s| s.to_string())
    }

    fn description(&self) -> Option<String> {
        self.typ.description()
    }
}

impl<B: Board> OutputBuilder<B> for TextOutputBuilder {
    fn for_engine(&mut self, _state: &dyn GameState<B>) -> Res<OutputBox<B>> {
        self.build(true)
    }

    fn for_client(&mut self, _state: &dyn GameState<B>) -> Res<OutputBox<B>> {
        self.build(false)
    }

    fn add_option(&mut self, _option: String) -> Res<()> {
        bail!("TextOutputBuilder doesn't support any additional options")
    }
}

pub fn board_to_string<B: RectangularBoard, F: Fn(B::Piece) -> char>(
    pos: &B,
    piece_to_char: F,
    flip: bool,
) -> String {
    use std::fmt::Write;
    let flip = flip && B::should_flip_visually();
    let mut res = pos.settings().text().unwrap_or_default();
    for y in 0..pos.height() {
        let yc = if flip { y } else { pos.height() - 1 - y };
        write!(&mut res, "{:>2} ", yc + 1).unwrap();
        for x in 0..pos.width() {
            let xc = if flip { pos.width() - 1 - x } else { x };
            let c = piece_to_char(pos.colored_piece_on(B::Coordinates::from_row_column(yc, xc)));
            write!(&mut res, " {c}").unwrap();
        }
        res += "\n";
    }
    res += "   ";
    for x in 0..pos.get_width() {
        let xc = if flip { pos.get_width() - 1 - x } else { x };
        write!(&mut res, " {}", ('A'..).nth(xc).unwrap()).unwrap();
    }
    res += "\n";
    res
}

pub fn p1_color() -> style::Color {
    style::Color::DarkBlue
}

pub fn p2_color() -> style::Color {
    style::Color::DarkMagenta
}

fn with_color(text: &str, color: Option<style::Color>, highlight: bool) -> String {
    if let Some(color) = color {
        text.with(color).bold().to_string()
    } else if highlight {
        text.dark_cyan().bold().to_string()
    } else {
        text.dim().to_string()
    }
}

// most of this function deals with coloring the frame of a square
pub fn display_board_pretty<B: RectangularBoard>(
    pos: &B,
    fmt: &mut dyn BoardFormatter<B>,
) -> String {
    use fmt::Write;
    let flip = fmt.flip_board() && B::should_flip_visually();
    let mut colors = vec![vec![None; pos.get_width() + 1]; pos.get_height() + 1];
    for y in 0..pos.get_height() {
        for x in 0..pos.get_width() {
            let square = B::Coordinates::from_row_column(y as u8, x as u8);
            if flip {
                colors[y][pos.get_width() - 1 - x] = fmt.frame_color(square);
            } else {
                colors[y][x] = fmt.frame_color(square);
            }
        }
    }
    let write_vertical_bar = |y: usize, bold: bool| -> String {
        let mut res = "    ".to_string();
        for x in 0..pos.get_width() {
            let mut col = colors[y][x];
            if col.is_none() && y > 0 {
                col = colors[y - 1][x]
            }
            let mut plus_color = col;
            if plus_color.is_none() && x > 0 {
                plus_color = colors[y][x - 1];
                if plus_color.is_none() && y > 0 {
                    plus_color = colors[y - 1][x - 1];
                }
            }
            let plus = with_color(
                "+",
                plus_color,
                bold || x % fmt.horizontal_spacer_interval() == 0,
            );
            let bar = with_color("---", col, bold);
            write!(&mut res, "{plus}{bar}").unwrap();
        }
        let mut plus_color = colors[y][pos.get_width() - 1];
        if plus_color.is_none() && y > 0 {
            plus_color = colors[y - 1][pos.get_width() - 1];
        }
        let plus = with_color(
            "+",
            plus_color,
            pos.get_width() % fmt.horizontal_spacer_interval() == 0,
        );
        write!(&mut res, "{plus}").unwrap();
        res
    };
    let mut res: Vec<String> = vec![];
    for y in 0..pos.get_height() {
        res.push(write_vertical_bar(
            y,
            y % fmt.vertical_spacer_interval() == 0,
        ));
        let mut line = format!(" {:>2} ", (y + 1).to_string());
        for x in 0..pos.get_width() {
            let mut col = colors[y][x];
            if col.is_none() && x > 0 {
                col = colors[y][x - 1];
            }
            line += &with_color("|", col, x % fmt.horizontal_spacer_interval() == 0);
            let xc = if flip { pos.get_width() - 1 - x } else { x };
            line += &fmt.display_piece(B::Coordinates::from_row_column(y as DimT, xc as DimT), 3);
        }
        line += &with_color(
            "|",
            colors[y][pos.get_width() - 1],
            pos.get_width() % fmt.horizontal_spacer_interval() == 0,
        );
        res.push(line);
    }
    res.push(write_vertical_bar(
        pos.get_height(),
        pos.get_height() % fmt.vertical_spacer_interval() == 0,
    ));
    if !flip {
        res.reverse();
    }
    if let Some(text) = pos.settings().text() {
        res.insert(0, text);
    }
    res.insert(0, format!("Fen: '{}'", pos.as_fen()));
    let mut line = "    ".to_string();
    for x in 0..pos.get_width() {
        let xc = if flip { pos.get_width() - 1 - x } else { x };
        write!(&mut line, " {:^3}", ('A'..).nth(xc).unwrap().to_string()).unwrap();
    }
    res.push(line);
    res.join("\n") + "\n"
}

pub trait BoardFormatter<B: Board> {
    fn display_piece(&self, coords: B::Coordinates, width: usize) -> String;

    fn frame_color(&self, coords: B::Coordinates) -> Option<style::Color>;

    fn flip_board(&self) -> bool;

    fn horizontal_spacer_interval(&self) -> usize;

    fn vertical_spacer_interval(&self) -> usize;
}

pub struct DefaultBoardFormatter<B: RectangularBoard> {
    pub pos: B,
    pub last_move: Option<B::Move>,
    pub flip: bool,
    pub vertical_spacer_interval: usize,
    pub horizontal_spacer_interval: usize,
}

impl<B: RectangularBoard> DefaultBoardFormatter<B> {
    pub fn new(pos: B, last_move: Option<B::Move>, flip: bool) -> Self {
        Self {
            pos,
            last_move,
            flip,
            vertical_spacer_interval: pos.get_height(),
            horizontal_spacer_interval: pos.get_width(),
        }
    }
}

impl<B: RectangularBoard> BoardFormatter<B> for DefaultBoardFormatter<B> {
    fn display_piece(&self, coords: B::Coordinates, width: usize) -> String {
        let piece = self.pos.colored_piece_on(coords);
        let c = format!("{0:^1$}", piece.to_utf8_char(), width);

        let Some(color) = piece.color() else { return c };
        if color.is_first() {
            c.with(p1_color()).bold().to_string()
        } else {
            c.with(p2_color()).bold().to_string()
        }
    }

    fn frame_color(&self, coords: B::Coordinates) -> Option<style::Color> {
        if self
            .last_move
            .is_some_and(|m| m.src_square() == coords || m.dest_square() == coords)
        {
            Some(style::Color::Rgb {
                r: 128,
                g: 64,
                b: 16,
            })
        } else {
            None
        }
    }

    fn flip_board(&self) -> bool {
        self.flip
    }

    fn horizontal_spacer_interval(&self) -> usize {
        self.horizontal_spacer_interval
    }

    fn vertical_spacer_interval(&self) -> usize {
        self.vertical_spacer_interval
    }
}

pub struct AdaptFormatter<B: Board> {
    pub underlying: Box<dyn BoardFormatter<B>>,
    pub color_frame: Box<dyn Fn(B::Coordinates) -> Option<style::Color>>,
    pub display_piece: Box<dyn Fn(B::Coordinates, usize) -> Option<String>>,
    pub horizontal_spacer_interval: Option<usize>,
    pub vertical_spacer_interval: Option<usize>,
}

impl<B: Board> BoardFormatter<B> for AdaptFormatter<B> {
    fn display_piece(&self, square: B::Coordinates, width: usize) -> String {
        (self.display_piece)(square, width)
            .unwrap_or_else(|| self.underlying.display_piece(square, width))
    }

    fn frame_color(&self, coords: B::Coordinates) -> Option<style::Color> {
        (self.color_frame)(coords).or_else(|| self.underlying.frame_color(coords))
    }

    fn flip_board(&self) -> bool {
        self.underlying.flip_board()
    }

    fn horizontal_spacer_interval(&self) -> usize {
        self.horizontal_spacer_interval
            .unwrap_or_else(|| self.underlying.horizontal_spacer_interval())
    }

    fn vertical_spacer_interval(&self) -> usize {
        self.vertical_spacer_interval
            .unwrap_or_else(|| self.underlying.vertical_spacer_interval())
    }
}
