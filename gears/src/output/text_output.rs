use crate::games::{Color, ColoredPiece, DimT, Settings};
use crate::general::board::{Board, RectangularBoard};
use crate::general::common::{NamedEntity, Res};
use crate::general::move_list::MoveList;
use crate::general::moves::ExtendedFormat::Alternative;
use crate::general::moves::Move;
use crate::general::squares::RectangularCoordinates;
use crate::general::squares::SquareColor::Black;
use crate::output::pgn::match_to_pgn_string;
use crate::output::text_output::DisplayType::*;
use crate::output::{AbstractOutput, Message, Output, OutputBox, OutputBuilder, OutputOpts};
use crate::GameState;
use crate::MatchStatus::*;
use anyhow::{anyhow, bail};
use colored::Colorize;
use std::fmt;
use std::fs::File;
use std::io::{stderr, stdout, Stderr, Stdout, Write};
use std::mem::swap;
use std::path::Path;
use std::str::SplitWhitespace;
use strum_macros::EnumIter;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PieceToChar {
    Ascii,
    Unicode,
}

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
    PrettyAscii,
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
            PrettyAscii => "prettyascii",
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
            Pretty => "Pretty Unicode Text Diagram",
            PrettyAscii => "Pretty Ascii Text Diagram",
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
            PrettyAscii => "A textual 2D representation of the position that's meant to look pretty, using ASCII characters for pieces. ",
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
        match_to_pgn_string(m)
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
        let flipped = m.active_player() == B::Color::second();
        if flipped {
            swap(&mut time_below, &mut time_above);
        }
        match self.typ {
            Pretty => {
                let mut formatter =
                    m.get_board()
                        .pretty_formatter(Some(PieceToChar::Unicode), m.last_move(), opts);
                format!(
                    "{time_above}{}{time_below}",
                    m.get_board().display_pretty(formatter.as_mut())
                )
            }
            PrettyAscii => {
                let mut formatter =
                    m.get_board()
                        .pretty_formatter(Some(PieceToChar::Ascii), m.last_move(), opts);
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
    let mut res = String::new();
    if let Some(text) = pos.settings().text() {
        res = format!("{text}\n");
    }
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

pub fn p1_color() -> colored::Color {
    colored::Color::Blue
    // #258ad1
}

pub fn p2_color() -> colored::Color {
    colored::Color::Magenta
    // #d23681
}

pub fn display_color<C: Color>(color: C) -> colored::Color {
    if color.is_first() {
        p1_color()
    } else {
        p2_color()
    }
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

const GOLD: colored::Color = colored::Color::TrueColor {
    r: 255,
    g: 215,
    b: 0,
};

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

fn border_cross<B: RectangularBoard>(
    pos: &B,
    y: usize,
    x: usize,
    cross: &'static str,
) -> &'static str {
    if cross != CROSS {
        return cross;
    }
    if x == 0 {
        if y == 0 {
            HEAVY_UPPER_LEFT_CORNER
        } else if y == pos.get_height() {
            HEAVY_LOWER_LEFT_CORNER
        } else {
            LEFT_BORDER
        }
    } else if x == pos.get_width() {
        if y == 0 {
            HEAVY_UPPER_RIGHT_CORNER
        } else if y == pos.get_height() {
            HEAVY_LOWER_RIGHT_CORNER
        } else {
            RIGHT_BORDER
        }
    } else if y == 0 {
        UPPER_BORDER
    } else if y == pos.get_height() {
        LOWER_BORDER
    } else {
        CROSS
    }
}

fn write_horizontal_bar<B: RectangularBoard>(
    y: usize,
    pos: &B,
    colors: &[Vec<Option<colored::Color>>],
    fmt: &dyn BoardFormatter<B>,
) -> String {
    use fmt::Write;
    let sq_width = fmt.overwrite_width().unwrap_or(3);
    let flip = fmt.flip_board() && B::should_flip_visually();
    let y_spacer = y % fmt.vertical_spacer_interval() == 0;
    let mut res = "    ".to_string();
    let bar = if y == 0 || y == pos.get_height() {
        HEAVY_HORIZONTAL_BAR
    } else {
        HORIZONTAL_BAR
    };
    for x in 0..=pos.get_width() {
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
        if cross_color.is_none()
            && ![0, pos.get_width()].contains(&x)
            && ![0, pos.get_height()].contains(&y)
        {
            if y_spacer && !x_spacer {
                cross = HORIZONTAL_BAR;
            } else if !y_spacer && x_spacer {
                cross = VERTICAL_BAR;
            }
        }
        let plus = with_color(
            flip_if(!flip, border_cross(pos, y, x, cross)),
            cross_color,
            y_spacer || x_spacer,
        );
        if x == pos.get_width() {
            res += &plus;
        } else {
            let bar = with_color(&bar.repeat(sq_width), col, y_spacer);
            write!(&mut res, "{plus}{bar}").unwrap();
        }
    }
    res
}

// most of this function deals with coloring the frame of a square
pub fn display_board_pretty<B: RectangularBoard>(
    pos: &B,
    fmt: &mut dyn BoardFormatter<B>,
) -> String {
    let flip = fmt.flip_board() && B::should_flip_visually();
    let sq_width = fmt.overwrite_width().unwrap_or(3);
    let mut colors = vec![vec![None; pos.get_width() + 1]; pos.get_height() + 1];
    #[allow(clippy::needless_range_loop)]
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
    let mut res: Vec<String> = vec![];
    for y in 0..pos.get_height() {
        res.push(write_horizontal_bar(y, pos, &colors, fmt));
        let mut line = format!(" {:>2} ", (y + 1)).dimmed().to_string();
        for x in 0..pos.get_width() {
            let mut col = colors[y][x];
            if col.is_none() && x > 0 {
                col = colors[y][x - 1];
            }
            let bar = if x == 0 {
                HEAVY_VERTICAL_BAR
            } else {
                VERTICAL_BAR
            };
            line += &with_color(bar, col, x % fmt.horizontal_spacer_interval() == 0);
            let xc = if flip { pos.get_width() - 1 - x } else { x };
            line += &fmt.display_piece(
                B::Coordinates::from_row_column(y as DimT, xc as DimT),
                sq_width,
            );
        }
        line += &with_color(
            HEAVY_VERTICAL_BAR,
            colors[y][pos.get_width() - 1],
            pos.get_width() % fmt.horizontal_spacer_interval() == 0,
        );
        res.push(line);
    }
    res.push(write_horizontal_bar(pos.get_height(), pos, &colors, fmt));
    if !flip {
        res.reverse();
    }
    if let Some(text) = pos.settings().text() {
        res.insert(0, text);
    }
    res.insert(0, format!("{0} '{1}'", "Fen:".dimmed(), pos.as_fen()));
    let mut line = "    ".to_string();
    for x in 0..pos.get_width() {
        let xc = if flip { pos.get_width() - 1 - x } else { x };
        line += &format!(" {:^sq_width$}", ('A'..).nth(xc).unwrap())
            .dimmed()
            .to_string();
    }
    res.push(line);
    res.join("\n") + "\n"
}

pub trait BoardFormatter<B: Board> {
    fn display_piece(&self, coords: B::Coordinates, width: usize) -> String;

    fn frame_color(&self, coords: B::Coordinates) -> Option<colored::Color>;

    fn flip_board(&self) -> bool;

    fn horizontal_spacer_interval(&self) -> usize;

    fn vertical_spacer_interval(&self) -> usize;

    fn overwrite_width(&self) -> Option<usize>;
}

pub struct DefaultBoardFormatter<B: RectangularBoard> {
    pub piece_to_char: PieceToChar,
    pub pos: B,
    pub last_move: Option<B::Move>,
    pub flip: bool,
    pub vertical_spacer_interval: usize,
    pub horizontal_spacer_interval: usize,
}

impl<B: RectangularBoard> DefaultBoardFormatter<B> {
    pub fn new(
        pos: B,
        piece_to_char: Option<PieceToChar>,
        last_move: Option<B::Move>,
        opts: OutputOpts,
    ) -> Self {
        let piece_to_char = piece_to_char.unwrap_or(PieceToChar::Ascii);
        let flip = (pos.active_player() == B::Color::second()) && !opts.disable_flipping;
        Self {
            piece_to_char,
            pos,
            last_move,
            flip,
            vertical_spacer_interval: pos.get_height(),
            horizontal_spacer_interval: pos.get_width(),
        }
    }
}

impl<B: RectangularBoard> BoardFormatter<B> for DefaultBoardFormatter<B> {
    fn display_piece(&self, square: B::Coordinates, width: usize) -> String {
        let piece = self.pos.colored_piece_on(square);
        let c = if piece.is_empty() {
            if self.pos.background_color(square) == Black {
                '*'
            } else {
                ' '
            }
        } else if self.piece_to_char == PieceToChar::Ascii {
            // for most games, it makes sense to always upper case letters. Chess overwrites this behavior
            piece.to_ascii_char().to_ascii_uppercase()
        } else {
            piece.to_utf8_char()
        };
        let c = format!("{c:^0$}", width);

        let Some(color) = piece.color() else {
            return c.dimmed().to_string();
        };
        // some (but not all) terminals have trouble with colored bold symbols, and using `bold` would remove the color in some cases.
        // For some reason, only using the ansi colore codes (.green(), .red(), etc) creates these problems, but true colors work fine
        c.color(display_color(color)).to_string()
    }

    fn frame_color(&self, square: B::Coordinates) -> Option<colored::Color> {
        if self
            .last_move
            .is_some_and(|m| m.src_square() == square || m.dest_square() == square)
        {
            Some(GOLD)
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

    fn overwrite_width(&self) -> Option<usize> {
        None
    }
}

#[allow(type_alias_bounds)]
pub type AdaptPieceDisplay<B: Board> =
    dyn Fn(B::Coordinates, Option<colored::Color>) -> Option<colored::Color>;

pub struct AdaptFormatter<B: Board> {
    pub underlying: Box<dyn BoardFormatter<B>>,
    pub color_frame: Box<AdaptPieceDisplay<B>>,
    pub display_piece: Box<dyn Fn(B::Coordinates, usize, String) -> String>,
    pub horizontal_spacer_interval: Option<usize>,
    pub vertical_spacer_interval: Option<usize>,
    pub square_width: Option<usize>,
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

    fn overwrite_width(&self) -> Option<usize> {
        self.square_width
    }
}
