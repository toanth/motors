use crate::GameState;
use crate::MatchStatus::*;
use crate::games::{AbstractPieceType, CharType, Color, ColoredPiece, ColoredPieceType, Coordinates, DimT, Settings};
use crate::general::board::{AxesFormat, Board, BoardHelpers, BoardOrientation, ColPieceTypeOf, RectangularBoard};
use crate::general::common::{NamedEntity, Res};
use crate::general::move_list::MoveList;
use crate::general::moves::ExtendedFormat::Alternative;
use crate::general::moves::Move;
use crate::general::squares::SquareColor::Black;
use crate::general::squares::{RectangularCoordinates, SquareColor};
use crate::output::pgn::match_to_pgn_string;
use crate::output::text_output::DisplayType::*;
use crate::output::text_output::PrintType::Simple;
use crate::output::{AbstractOutput, Message, Output, OutputBox, OutputBuilder, OutputOpts};
use anyhow::{anyhow, bail, ensure};
use colored::Colorize;
use std::fmt::Write;
use std::fs::File;
use std::io::{Stderr, Stdout, stderr, stdout};
use std::mem::swap;
use std::path::Path;
use std::str::SplitWhitespace;
use std::{fmt, io};
use strum_macros::EnumIter;

#[derive(Debug)]
pub enum TextStream {
    File(File, String), // Don't use a BufWriter to ensure the log is always up-to-date.
    Stdout(Stdout),
    Stderr(Stderr),
}

impl TextStream {
    pub fn write(&mut self, prefix: &str, msg: &fmt::Arguments) {
        _ = writeln!(self.stream(), "{prefix} {msg}");
    }

    pub fn stream(&mut self) -> &mut dyn io::Write {
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
        // Although files of course don't have to contain a '.', requiring that feels like a good way to
        // catch errors like typos where the user didn't mean to specify a file name.
        ensure!(
            name.contains('.'),
            "'{name}' does not appear to be a valid log filename (it does not contain a '.'). \
                Expected either a filename, 'stdout', 'stderr', or 'none'."
        );
        let path = Path::new(name);
        let file = File::create(path).map_err(|err| anyhow!("Couldn't create log file: {err}"))?;
        Ok(TextStream::File(
            file,
            path.canonicalize().ok().as_ref().and_then(|p| p.to_str()).unwrap_or(name).to_string(),
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
    pub fn display_message(&mut self, typ: Message, message: &fmt::Arguments) {
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
    PrettyAscii,
    Unicode,
    Ascii,
    Fen,
    Pgn,
    Moves, // Prints all legal moves
    Uci,
    Ugi,  // The same as `UCI`, but with a different name so that the user can write both 'print uci' and 'print ugi'
    Hash, // The hash of the current position
    MsgOnly, // Doesn't print the state at all, but a text output with that display type would still display messages.
}

impl NamedEntity for DisplayType {
    fn short_name(&self) -> String {
        match self {
            Pretty => "pretty",
            PrettyAscii => "prettyascii",
            Unicode => "unicode",
            Ascii => "ascii",
            Fen => "fen",
            Pgn => "pgn",
            Moves => "moves",
            Uci => "uci",
            Ugi => "ugi",
            Hash => "hash",
            MsgOnly => "messages",
        }
        .to_string()
    }

    fn long_name(&self) -> String {
        match self {
            Pretty => "Pretty Unicode Text Diagram",
            PrettyAscii => "Pretty Ascii Text Diagram",
            Unicode => "Unicode Diagram",
            Ascii => "ASCII Diagram",
            Fen => "Fen",
            Pgn => "PGN",
            Moves => "Moves",
            Uci => "UCI",
            Ugi => "UGI",
            Hash => "Hash",
            MsgOnly => "Only Messages",
        }
        .to_string()
    }

    fn description(&self) -> Option<String> {
        Some(match self {
            Pretty => "A textual 2D representation of the position that's meant to look pretty. ",
            PrettyAscii => "A textual 2D representation of the position that's meant to look pretty, using ASCII characters for pieces. ",
            Unicode => "A textual 2D representation of the position using unicode characters. For many games, this is the same as the ASCII representation, but e.g. for chess it uses chess symbols like '♔'",
            Ascii => "A textual 2D representation of the position using \"normal\" english characters. E.g. for chess, this represents the white king as 'K' and a black queen as 'q'",
            Fen => "A compact textual representation of the position. For chess, this is the Forsyth–Edwards Notation, and for other games it's a similar notation based on chess FENs",
            Pgn => "A textual representation of the entire match. For chess, this is the Portable Games Notation, and for other games it's a similar notation based on chess PGNs",
            Moves => "A space-separated list of all legal moves, intended mostly for debugging",
            Uci => "A textual representation of the match using the machine-readable UGI notation that gets used for engine-GUI communication. UCI for chess and the very slightly different UGI protocol for other games",
            Ugi => "Same as 'UCI'",
            Hash => "The hash of the current position (does not include the board history)",
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
        match_to_pgn_string(m)
    }

    fn list_moves<B: Board>(m: &dyn GameState<B>) -> String {
        use fmt::Write;
        let mut res = String::default();
        let pos = m.get_board();
        for mov in pos.legal_moves_slow().iter_moves() {
            write!(&mut res, "{} ", mov.to_extended_text(pos, Alternative)).unwrap();
        }
        res
    }

    fn match_to_ugi<B: Board>(m: &dyn GameState<B>) -> String {
        use std::fmt::Write;
        let mut pos = m.initial_pos().clone();
        if m.move_history().is_empty() {
            format!("position fen {pos}")
        } else {
            let mut res = format!("position fen {pos} moves ");
            for &mov in m.move_history() {
                write!(&mut res, "{} ", mov.compact_formatter(&pos)).unwrap();
                let Some(new) = pos.make_move(mov) else {
                    write!(&mut res, "{}", "(invalid move)".red()).unwrap();
                    return res;
                };
                pos = new;
            }
            res
        }
    }

    pub fn as_string<B: Board>(&self, m: &dyn GameState<B>, opts: OutputOpts) -> String {
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
        let game_result = match m.match_status() {
            NotStarted | Ongoing => "".to_string(),
            Over(res) => {
                format!("\n{}", res.result)
            }
        };
        let repetitions = m.board_hist().num_repetitions(m.get_board().hash_pos());
        let reps = if repetitions == 0 {
            String::new()
        } else if repetitions == 1 {
            "Position occurred once before".to_string()
        } else {
            format!("Position occurred {repetitions} times before")
        };
        let flipped = !m.active_player().is_first();
        if flipped {
            swap(&mut time_below, &mut time_above);
        }
        match self.typ {
            Pretty => {
                let mut formatter = m.get_board().pretty_formatter(Some(CharType::Unicode), m.last_move(), opts);
                format!(
                    "{time_above}{}{time_below}{reps}{game_result}",
                    m.get_board().display_pretty(formatter.as_mut())
                )
            }
            PrettyAscii => {
                let mut formatter = m.get_board().pretty_formatter(Some(CharType::Ascii), m.last_move(), opts);
                format!(
                    "{time_above}{}{time_below}{reps}{game_result}",
                    m.get_board().display_pretty(formatter.as_mut())
                )
            }
            Ascii => {
                format!(
                    "{time_above}{}{time_below}{reps}{game_result}",
                    m.get_board().as_diagram(CharType::Ascii, flipped, true)
                )
            }
            Unicode => {
                format!(
                    "{time_above}{}{time_below}{reps}{game_result}",
                    m.get_board().as_diagram(CharType::Unicode, flipped, true)
                )
            }
            Fen => m.get_board().as_fen(),
            Pgn => Self::match_to_pgn(m),
            Moves => Self::list_moves(m),
            Uci | Ugi => BoardToText::match_to_ugi(m),
            Hash => m.get_board().hash_pos().to_string(),
            MsgOnly => String::default(),
        }
    }
}

#[derive(Debug)]
pub(super) struct TextOutput {
    writer: TextWriter,
    to_text: BoardToText,
    name: Option<String>,
}

impl TextOutput {
    fn new(typ: DisplayType, is_engine: bool, writer: TextWriter, name: Option<String>) -> Self {
        Self { to_text: BoardToText { typ, is_engine }, writer, name }
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

    fn display_message(&mut self, typ: Message, message: &fmt::Arguments) {
        self.writer.display_message(typ, message);
    }
}

impl<B: Board> Output<B> for TextOutput {
    fn as_string(&self, m: &dyn GameState<B>, opts: OutputOpts) -> String {
        self.to_text.as_string(m, opts)
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
        Self { typ: self.typ, stream: None, accepted: self.accepted.clone(), short_name: self.short_name.clone() }
    }
}

impl TextOutputBuilder {
    pub fn new(typ: DisplayType) -> Self {
        Self { typ, stream: None, accepted: vec![], short_name: None }
    }

    pub fn messages_for(accepted: Vec<Message>, short_name: &str) -> Self {
        Self { typ: MsgOnly, stream: None, accepted, short_name: Some(short_name.to_string()) }
    }

    pub fn build<B: Board>(&mut self, is_engine: bool) -> Res<OutputBox<B>> {
        let stream = self.stream.take().unwrap_or_else(|| TextStream::Stderr(stderr()));
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
        self.short_name.clone().unwrap_or_else(|| self.typ.short_name())
    }

    fn long_name(&self) -> String {
        self.short_name.clone().map_or_else(|| self.typ.long_name(), |s| s.to_string())
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

struct SquareInfo {
    piece_char: char,
    square_color: SquareColor,
    empty: bool,
    // Some(true) for first player, Some(false) for second, None for no player
    is_first_player: Option<bool>,
}

/// Type-erase the `Board` to reduce unnecessary code bloat
trait AbstractPrettyBoardPrinter {
    fn square_info(&self, row: DimT, column: DimT) -> SquareInfo;
    fn width(&self) -> DimT;
    fn height(&self) -> DimT;
    fn get_width(&self) -> usize {
        self.width() as usize
    }
    fn get_height(&self) -> usize {
        self.height() as usize
    }
    fn max_piece_width(&self) -> usize;
    fn fen(&self) -> String;
    fn side_to_move(&self) -> String;
    fn is_first_active(&self) -> bool;
    fn settings_text(&self) -> Option<String>;
    fn formatter(&self) -> &dyn AbstractBoardFormatter;
}

#[allow(type_alias_bounds)]
type SimplePieceFormatter<B: Board> = dyn Fn(B::Piece, CharType, &B::Settings) -> char;

enum PrintType<'a, B: Board> {
    Formatter(&'a dyn BoardFormatter<B>),
    Simple(&'a SimplePieceFormatter<B>, CharType),
}

struct PrettyBoardPrinter<'a, B: RectangularBoard> {
    board: &'a B,
    print_type: PrintType<'a, B>,
}

impl<'a, B: RectangularBoard> AbstractPrettyBoardPrinter for PrettyBoardPrinter<'a, B> {
    fn square_info(&self, rank: DimT, file: DimT) -> SquareInfo {
        let square = B::Coordinates::from_rank_file(rank, file);
        let piece = self.board.colored_piece_on(square);
        let Simple(piece_to_char, char_type) = &self.print_type else { unreachable!() };
        let piece_char = (piece_to_char)(piece, *char_type, self.board.settings());
        let square_color = square.square_color();
        let empty = self.board.is_empty(square);
        let is_first_player = piece.color().map(|c| c.is_first());
        SquareInfo { piece_char, square_color, empty, is_first_player }
    }

    fn width(&self) -> DimT {
        self.board.width()
    }

    fn height(&self) -> DimT {
        self.board.height()
    }

    fn max_piece_width(&self) -> usize {
        ColPieceTypeOf::<B>::max_num_chars(self.board.settings())
    }

    fn fen(&self) -> String {
        self.board.as_fen()
    }

    fn side_to_move(&self) -> String {
        self.board.active_player().name(self.board.settings()).to_string()
    }

    fn is_first_active(&self) -> bool {
        self.board.active_player().is_first()
    }

    fn settings_text(&self) -> Option<String> {
        self.board.settings().text()
    }

    fn formatter(&self) -> &dyn AbstractBoardFormatter {
        let PrintType::Formatter(formatter) = self.print_type else { unreachable!() };
        formatter
    }
}

fn board_to_string_impl(
    printer: &dyn AbstractPrettyBoardPrinter,
    flip: bool,
    axes_format: AxesFormat,
    mark_active: bool,
) -> String {
    use std::fmt::Write;
    let mut res = String::new();
    if let Some(text) = printer.settings_text() {
        res = format!("{text}\n");
    }
    let active = if printer.is_first_active() != flip { 0 } else { printer.height() - 1 };
    for y in 0..printer.height() {
        let y = if flip { y } else { printer.height() - 1 - y };
        write!(&mut res, "{:>2} ", axes_format.ith_y_axis_entry(y, printer.height(), Some(2), flip)).unwrap();
        for x in 0..printer.width() {
            let x = if flip { printer.width() - 1 - x } else { x };
            let info = printer.square_info(y, x);
            let piece = if let Some(is_first_player) = info.is_first_player {
                info.piece_char.to_string().color(display_color_of(is_first_player)).to_string()
            } else if info.empty && info.square_color == Black {
                info.piece_char.to_string().dimmed().to_string()
            } else {
                info.piece_char.to_string()
            };
            write!(&mut res, " {piece}").unwrap();
        }
        if y == active && mark_active {
            write!(&mut res, " (*) ").unwrap();
        }
        res += "\n";
    }
    res += "   ";
    for x in 0..printer.width() {
        let x = axes_format.ith_x_axis_entry(x, printer.width(), Some(1), flip);
        write!(&mut res, " {x}").unwrap();
    }
    res += "\n";
    res
}

pub fn board_to_string<B: RectangularBoard, F: Fn(B::Piece, CharType, &B::Settings) -> char + 'static>(
    pos: &B,
    piece_to_char: F,
    typ: CharType,
    request_flip: bool,
    mark_active: bool,
) -> String {
    let printer = PrettyBoardPrinter { board: pos, print_type: Simple(&piece_to_char, typ) };
    board_to_string_impl(&printer, request_flip, pos.axes_format(), mark_active)
}

pub fn p1_color() -> colored::Color {
    colored::Color::Blue
    // #258ad1
}

pub fn p2_color() -> colored::Color {
    colored::Color::Magenta
    // #d23681
}

fn display_color_of(is_first: bool) -> colored::Color {
    if is_first { p1_color() } else { p2_color() }
}

pub fn display_color<C: Color>(color: C) -> colored::Color {
    display_color_of(color.is_first())
}

fn with_color(text: &str, color: Option<colored::Color>, highlight: bool) -> String {
    if let Some(color) = color {
        text.color(color).bold().to_string()
    } else if highlight {
        text.cyan().bold().to_string()
    } else {
        text.dimmed().to_string()
    }
}

const VERTICAL_BAR: &str = "│";
const HORIZONTAL_BAR: &str = "─";
const CROSS: &str = "┼";

const HEAVY_VERTICAL_BAR: &str = "┃";
const HEAVY_HORIZONTAL_BAR: &str = "━";
#[allow(unused)]
const HEAVY_CROSS: &str = "╋";

const LIGHT_UPPER_LEFT_CORNER: &str = "┌";
const LIGHT_UPPER_RIGHT_CORNER: &str = "┐";
const LIGHT_LOWER_LEFT_CORNER: &str = "└";
const LIGHT_LOWER_RIGHT_CORNER: &str = "┘";

const HEAVY_UPPER_LEFT_CORNER: &str = "┏";
const HEAVY_UPPER_RIGHT_CORNER: &str = "┓";
const HEAVY_LOWER_LEFT_CORNER: &str = "┗";
const HEAVY_LOWER_RIGHT_CORNER: &str = "┛";

const LEFT_BORDER: &str = "┠";
const RIGHT_BORDER: &str = "┨";
const LOWER_BORDER: &str = "┷";
const UPPER_BORDER: &str = "┯";

const GOLD: colored::Color = colored::Color::TrueColor { r: 255, g: 215, b: 0 };

fn flip_if(flip: bool, c: &'static str) -> &'static str {
    if !flip {
        return c;
    }
    match c {
        LIGHT_UPPER_LEFT_CORNER => LIGHT_LOWER_LEFT_CORNER,
        LIGHT_UPPER_RIGHT_CORNER => LIGHT_LOWER_RIGHT_CORNER,
        LIGHT_LOWER_LEFT_CORNER => LIGHT_UPPER_LEFT_CORNER,
        LIGHT_LOWER_RIGHT_CORNER => LIGHT_UPPER_RIGHT_CORNER,
        HEAVY_UPPER_LEFT_CORNER => HEAVY_LOWER_LEFT_CORNER,
        HEAVY_UPPER_RIGHT_CORNER => HEAVY_LOWER_RIGHT_CORNER,
        HEAVY_LOWER_LEFT_CORNER => HEAVY_UPPER_LEFT_CORNER,
        HEAVY_LOWER_RIGHT_CORNER => HEAVY_UPPER_RIGHT_CORNER,
        LOWER_BORDER => UPPER_BORDER,
        UPPER_BORDER => LOWER_BORDER,
        c => c,
    }
}

fn border_cross(printer: &dyn AbstractPrettyBoardPrinter, y: usize, x: usize, cross: &'static str) -> &'static str {
    if cross != CROSS {
        return cross;
    }
    if x == 0 {
        if y == 0 {
            HEAVY_UPPER_LEFT_CORNER
        } else if y == printer.get_height() {
            HEAVY_LOWER_LEFT_CORNER
        } else {
            LEFT_BORDER
        }
    } else if x == printer.get_width() {
        if y == 0 {
            HEAVY_UPPER_RIGHT_CORNER
        } else if y == printer.get_height() {
            HEAVY_LOWER_RIGHT_CORNER
        } else {
            RIGHT_BORDER
        }
    } else if y == 0 {
        UPPER_BORDER
    } else if y == printer.get_height() {
        LOWER_BORDER
    } else {
        CROSS
    }
}

fn write_horizontal_bar(
    y: usize,
    printer: &dyn AbstractPrettyBoardPrinter,
    colors: &[Vec<Option<colored::Color>>],
    flip: bool,
    sq_width: usize,
) -> String {
    use fmt::Write;
    let fmt = printer.formatter();
    // let flip = fmt.flip_board() && B::should_flip_visually();
    let y_spacer = y % fmt.vertical_spacer_interval() == 0;
    let mut res = "    ".to_string();
    let bar = if y == 0 || y == printer.get_height() { HEAVY_HORIZONTAL_BAR } else { HORIZONTAL_BAR };
    for x in 0..=printer.get_width() {
        let x_spacer = x % fmt.horizontal_spacer_interval() == 0;
        let mut col = colors[y][x];
        let mut cross = CROSS;
        if col.is_some() {
            cross = LIGHT_UPPER_LEFT_CORNER;
        }
        if col.is_none() && y > 0 {
            col = colors[y - 1][x];
            if col.is_some() {
                cross = LIGHT_LOWER_LEFT_CORNER;
            }
        }
        let mut cross_color = col;
        if cross_color.is_none() && x > 0 {
            cross_color = colors[y][x - 1];
            if cross_color.is_some() {
                cross = LIGHT_UPPER_RIGHT_CORNER;
            }
            if cross_color.is_none() && y > 0 {
                cross_color = colors[y - 1][x - 1];
                if cross_color.is_some() {
                    cross = LIGHT_LOWER_RIGHT_CORNER
                }
            }
        }
        if cross_color.is_none() && ![0, printer.get_width()].contains(&x) && ![0, printer.get_height()].contains(&y) {
            if y_spacer && !x_spacer {
                cross = HORIZONTAL_BAR;
            } else if !y_spacer && x_spacer {
                cross = VERTICAL_BAR;
            }
        }
        let plus = with_color(flip_if(!flip, border_cross(printer, y, x, cross)), cross_color, y_spacer || x_spacer);
        if x == printer.get_width() {
            res += &plus;
        } else {
            let bar = with_color(&bar.repeat(sq_width), col, y_spacer);
            write!(&mut res, "{plus}{bar}").unwrap();
        }
    }
    res
}

// most of this function deals with coloring the frame of a square
fn display_board_pretty_impl(printer: &dyn AbstractPrettyBoardPrinter, flip: bool) -> String {
    let fmt = printer.formatter();
    let sq_width = fmt.overwrite_width().unwrap_or(3) + printer.max_piece_width().saturating_sub(1);
    let mut colors = vec![vec![None; printer.get_width() + 1]; printer.get_height() + 1];
    #[allow(clippy::needless_range_loop)]
    for y in 0..printer.get_height() {
        for x in 0..printer.get_width() {
            if flip {
                colors[y][printer.get_width() - 1 - x] = fmt.frame_color_rank_file(y, x);
            } else {
                colors[y][x] = fmt.frame_color_rank_file(y, x);
            }
        }
    }
    let mut res: Vec<String> = vec![];
    for y in 0..printer.get_height() {
        res.push(write_horizontal_bar(y, printer, &colors, flip, sq_width));
        let y_axis_token =
            printer.formatter().axes_format().ith_y_axis_entry(y as DimT, printer.height(), Some(2), false);
        let mut line = format!(" {y_axis_token:>2} ").dimmed().to_string();
        for x in 0..printer.get_width() {
            let mut col = colors[y][x];
            if col.is_none() && x > 0 {
                col = colors[y][x - 1];
            }
            let bar = if x == 0 { HEAVY_VERTICAL_BAR } else { VERTICAL_BAR };
            line += &with_color(bar, col, x % fmt.horizontal_spacer_interval() == 0);
            let xc = if flip { printer.get_width() - 1 - x } else { x };
            line += &fmt.display_piece_rank_file(y, xc, sq_width);
        }
        line += &with_color(
            HEAVY_VERTICAL_BAR,
            colors[y][printer.get_width() - 1],
            printer.get_width() % fmt.horizontal_spacer_interval() == 0,
        );
        res.push(line);
    }
    res.push(write_horizontal_bar(printer.get_height(), printer, &colors, flip, sq_width));

    let last_row = res.len() - 2;
    let (active, inactive) = if printer.is_first_active() { (1, last_row) } else { (last_row, 1) };
    write!(&mut res[active], " (*) ").unwrap();
    write!(&mut res[inactive], "     ").unwrap();
    write!(&mut res[1], "{}", printer.formatter().hand(true)).unwrap();
    write!(&mut res[last_row], "{}", printer.formatter().hand(false)).unwrap();

    if !flip {
        res.reverse();
    }
    if let Some(text) = printer.settings_text() {
        res.insert(0, text);
    }
    res.insert(
        0,
        format!(
            "{0} '{1}'{2} {3} {4}",
            "Fen:".dimmed(),
            printer.fen(),
            ",".dimmed(),
            printer.side_to_move(),
            "to move".dimmed()
        ),
    );
    let mut line = "    ".to_string();
    for x in 0..printer.width() {
        let x = printer.formatter().axes_format().ith_x_axis_entry(x, printer.width(), Some(sq_width), flip);
        line += &format!(" {x:^sq_width$}").dimmed().to_string();
    }
    res.push(line);
    res.join("\n") + "\n"
}

pub fn display_board_pretty<B: RectangularBoard>(pos: &B, fmt: &mut dyn BoardFormatter<B>) -> String {
    let flip = fmt.flip_board() && pos.axes_format().orientation == BoardOrientation::PlayerPov;
    let printer = PrettyBoardPrinter { board: pos, print_type: PrintType::Formatter::<B>(fmt) };
    display_board_pretty_impl(&printer, flip)
}

pub trait AbstractBoardFormatter {
    fn display_piece_rank_file(&self, rank: usize, file: usize, width: usize) -> String;

    fn hand(&self, first_player: bool) -> String;

    fn frame_color_rank_file(&self, rank: usize, file: usize) -> Option<colored::Color>;

    fn flip_board(&self) -> bool;

    fn horizontal_spacer_interval(&self) -> usize;

    fn vertical_spacer_interval(&self) -> usize;

    fn overwrite_width(&self) -> Option<usize>;

    fn axes_format(&self) -> AxesFormat;
}

pub trait BoardFormatter<B: Board>: AbstractBoardFormatter {
    fn display_piece(&self, coords: B::Coordinates, width: usize) -> String;

    fn frame_color(&self, coords: B::Coordinates) -> Option<colored::Color>;
}

pub struct DefaultBoardFormatter<B: RectangularBoard> {
    pub piece_to_char: CharType,
    pub pos: B,
    pub last_move: Option<B::Move>,
    pub flip: bool,
    pub vertical_spacer_interval: usize,
    pub horizontal_spacer_interval: usize,
}

impl<B: RectangularBoard> DefaultBoardFormatter<B> {
    pub fn new(pos: B, piece_to_char: Option<CharType>, last_move: Option<B::Move>, opts: OutputOpts) -> Self {
        let piece_to_char = piece_to_char.unwrap_or(CharType::Ascii);
        let flip = (pos.active_player() == B::Color::second()) && !opts.disable_flipping;
        Self {
            vertical_spacer_interval: pos.get_height(),
            horizontal_spacer_interval: pos.get_width(),
            piece_to_char,
            pos,
            last_move,
            flip,
        }
    }
}

impl<B: RectangularBoard> AbstractBoardFormatter for DefaultBoardFormatter<B> {
    fn display_piece_rank_file(&self, rank: usize, file: usize, width: usize) -> String {
        self.display_piece(B::Coordinates::from_rank_file(rank as DimT, file as DimT), width)
    }

    fn hand(&self, first_player: bool) -> String {
        let c = if first_player { B::Color::first() } else { B::Color::second() };
        let mut hand = self.pos.hand(c);
        if hand.next().is_none() {
            return String::new();
        }
        let mut res = "[".dimmed().to_string();
        for (count, piece) in self.pos.hand(c) {
            let piece = piece.to_display_char(self.piece_to_char, self.pos.settings());
            if count == 1 {
                write!(&mut res, "{piece}").unwrap()
            } else {
                write!(&mut res, "{count}{piece}",).unwrap();
            }
        }
        write!(&mut res, "{}", "]".dimmed()).unwrap();
        res
    }

    fn frame_color_rank_file(&self, rank: usize, file: usize) -> Option<colored::Color> {
        self.frame_color(B::Coordinates::from_rank_file(rank as DimT, file as DimT))
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

    fn overwrite_width(&self) -> Option<usize> {
        None
    }

    fn axes_format(&self) -> AxesFormat {
        self.pos.axes_format()
    }
}

impl<B: RectangularBoard> BoardFormatter<B> for DefaultBoardFormatter<B> {
    fn display_piece(&self, square: B::Coordinates, width: usize) -> String {
        let piece = self.pos.colored_piece_on(square);
        let sq = if piece.is_empty() {
            let c = if self.pos.background_color(square) == Black { '*' } else { ' ' };
            format!("{c:^width$}")
        } else {
            format!(
                "{:^width$}",
                piece.colored_piece_type().str_formatter(self.pos.settings(), self.piece_to_char, true).to_string()
            )
        };
        let Some(color) = piece.color() else {
            if piece.is_empty() {
                return sq.dimmed().to_string();
            }
            return sq.bold().to_string();
        };
        // some (but not all) terminals have trouble with colored bold symbols, and using `bold` would remove the color in some cases.
        // For some reason, only using the ansi color codes (.green(), .red(), etc) creates these problems, but true colors work fine
        sq.color(display_color(color)).to_string()
    }

    fn frame_color(&self, square: B::Coordinates) -> Option<colored::Color> {
        if self
            .last_move
            .is_some_and(|m| m.src_square_in(&self.pos) == Some(square) || m.dest_square_in(&self.pos) == square)
        {
            Some(GOLD)
        } else {
            None
        }
    }
}

#[allow(type_alias_bounds)]
pub type AdaptPieceDisplay<B: RectangularBoard> =
    dyn Fn(B::Coordinates, Option<colored::Color>) -> Option<colored::Color>;

pub struct AdaptFormatter<B: Board> {
    pub underlying: Box<dyn BoardFormatter<B>>,
    pub color_frame: Box<AdaptPieceDisplay<B>>,
    pub display_piece: Box<dyn Fn(B::Coordinates, usize, String) -> String>,
    pub horizontal_spacer_interval: Option<usize>,
    pub vertical_spacer_interval: Option<usize>,
    pub square_width: Option<usize>,
}

impl<B: Board> AbstractBoardFormatter for AdaptFormatter<B> {
    fn display_piece_rank_file(&self, rank: usize, file: usize, width: usize) -> String {
        let sq = B::Coordinates::from_x_y(rank, file);
        self.display_piece(sq, width)
    }

    fn hand(&self, first_player: bool) -> String {
        self.underlying.hand(first_player)
    }

    fn frame_color_rank_file(&self, rank: usize, file: usize) -> Option<colored::Color> {
        let sq = B::Coordinates::from_x_y(rank, file);
        self.frame_color(sq)
    }

    fn flip_board(&self) -> bool {
        self.underlying.flip_board()
    }

    fn horizontal_spacer_interval(&self) -> usize {
        self.horizontal_spacer_interval.unwrap_or_else(|| self.underlying.horizontal_spacer_interval())
    }

    fn vertical_spacer_interval(&self) -> usize {
        self.vertical_spacer_interval.unwrap_or_else(|| self.underlying.vertical_spacer_interval())
    }

    fn overwrite_width(&self) -> Option<usize> {
        self.square_width
    }

    fn axes_format(&self) -> AxesFormat {
        self.underlying.axes_format()
    }
}

impl<B: Board> BoardFormatter<B> for AdaptFormatter<B> {
    fn display_piece(&self, square: B::Coordinates, width: usize) -> String {
        let underlying_res = self.underlying.display_piece(square, width);
        (self.display_piece)(square, width, underlying_res)
    }

    fn frame_color(&self, coords: B::Coordinates) -> Option<colored::Color> {
        let color = self.underlying.frame_color(coords);
        (self.color_frame)(coords, color)
    }
}
