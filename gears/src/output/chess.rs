use crate::games::{Color, ColoredPiece};
use crate::general::board::RectangularBoard;
use crate::general::common::{ColorMsg, NamedEntity, Res, StaticallyNamedEntity};
use crate::general::squares::{RectangularCoordinates, SquareColor};
use crate::output::text_output::{
    display_color, p1_color, p2_color, AdaptFormatter, BoardFormatter, PieceToChar, TextStream,
    TextWriter,
};
use crate::output::Message::Info;
use crate::output::{AbstractOutput, Message, Output, OutputBox, OutputBuilder};
use crate::GameState;
use anyhow::bail;
use crossterm::style;
use crossterm::style::Color::{Reset, Rgb};
use crossterm::style::Stylize;
use std::fmt::{Display, Write};
use std::io::stdout;

#[derive(Debug)]
pub(super) struct ChessOutput {
    writer: TextWriter,
}

impl Default for ChessOutput {
    fn default() -> Self {
        Self {
            writer: TextWriter::new_for(TextStream::Stdout(stdout()), vec![Info]),
        }
    }
}

impl NamedEntity for ChessOutput {
    fn short_name(&self) -> String {
        ChessOutputBuilder::static_short_name().to_string()
    }

    fn long_name(&self) -> String {
        ChessOutputBuilder::static_long_name().to_string()
    }

    fn description(&self) -> Option<String> {
        Some(ChessOutputBuilder::static_description())
    }
}

impl AbstractOutput for ChessOutput {
    fn output_name(&self) -> String {
        self.writer.stream.name()
    }

    fn display_message(&mut self, typ: Message, message: &str) {
        self.writer.display_message(typ, message);
    }
}

impl<B: RectangularBoard> Output<B> for ChessOutput {
    fn as_string(&self, m: &dyn GameState<B>) -> String {
        let mut res = String::default();
        let pos = m.get_board();
        let last_move = m.last_move();
        if last_move.is_none() {
            writeln!(res, "Starting new game!").unwrap();
        }
        pretty_as_chessboard(
            &pos,
            pos.pretty_formatter(Some(PieceToChar::Ascii), last_move),
        )
    }
}

/// Except for RGB colors, how these colors are displayed depends on the style of the terminal.
/// We still try to guess some value
pub fn guess_colorgrad_color(color: style::Color) -> colorgrad::Color {
    let name = match color {
        style::Color::Reset => return guess_colorgrad_color(style::Color::White),
        style::Color::Black => "black",
        style::Color::DarkGrey => "darkgrey",
        style::Color::Red => "red",
        style::Color::DarkRed => "darkred",
        style::Color::Green => "green",
        style::Color::DarkGreen => "darkgreen",
        style::Color::Yellow => "yellow",
        style::Color::DarkYellow => "darkyellow",
        style::Color::Blue => "blue",
        style::Color::DarkBlue => "darkblue",
        style::Color::Magenta => "magenta",
        style::Color::DarkMagenta => "darkmagenta",
        style::Color::Cyan => "cyan",
        style::Color::DarkCyan => "darkcyan",
        style::Color::White => "white",
        style::Color::Grey => "grey",
        Rgb { r, g, b } => return colorgrad::Color::from([r, g, b]),
        style::Color::AnsiValue(_) => {
            panic!("ANSI color codes are not supported")
        }
    };
    colorgrad::Color::from_html(name).expect("incorrect color name")
}

fn pretty_as_chessboard<B: RectangularBoard>(
    pos: &B,
    formatter: Box<dyn BoardFormatter<B>>,
) -> String {
    let pos = *pos;
    let flip = formatter.flip_board();
    let formatter = AdaptFormatter {
        underlying: formatter,
        color_frame: Box::new(|_, color| color),
        display_piece: Box::new(move |square, width, _| {
            let piece = pos.colored_piece_on(square);
            if let Some(color) = piece.color() {
                format!("{0:^1$}", piece.to_ascii_char(), width)
                    .with(display_color(color))
                    .to_string()
            } else {
                " ".repeat(width)
            }
        }),
        horizontal_spacer_interval: None,
        vertical_spacer_interval: None,
        square_width: None,
    };
    let mut res = String::default();
    for y in 0..pos.height() {
        let mut line = "".to_string();
        for x in 0..pos.width() {
            let display_x = if flip { pos.width() - 1 - x } else { x };
            let display_y = if flip { y } else { pos.height() - 1 - y };
            let square = B::Coordinates::from_row_column(display_y, display_x);
            let color = pos.colored_piece_on(square).color();
            let bg_color = match pos.background_color(square) {
                SquareColor::White => colorgrad::Color::from_html("aliceblue").unwrap(),
                SquareColor::Black => colorgrad::Color::from_html("darkslategrey").unwrap(),
            };
            let bg_color = match formatter.frame_color(square) {
                None => bg_color,
                Some(col) => bg_color.interpolate_rgb(&guess_colorgrad_color(col), 0.25),
            };

            let color = match color {
                None => Reset,
                Some(x) => {
                    if x.is_first() {
                        p1_color()
                    } else {
                        p2_color()
                    }
                }
            };
            let [r, g, b, _] = bg_color.to_rgba8();
            let bg_color = Rgb { r, g, b };
            let piece = formatter
                .display_piece(square, 3)
                .with(color)
                .important()
                .on(bg_color);
            write!(&mut line, "{piece}").unwrap();
        }
        let y = if flip { y + 1 } else { pos.height() - y };
        writeln!(res, " {y:>2} {line}").unwrap();
    }
    res += "    ";
    for x in 0..pos.get_width() {
        let idx = if flip { pos.get_width() - 1 - x } else { x };
        write!(res, "{:^3}", ('A'..).nth(idx).unwrap()).unwrap();
    }
    res + "\n"
}

#[derive(Default, Copy, Clone, Debug)]
pub struct ChessOutputBuilder {}

impl StaticallyNamedEntity for ChessOutputBuilder {
    fn static_short_name() -> impl Display {
        "chess"
    }

    fn static_long_name() -> String {
        "Chessboard Text-based Output".to_string()
    }

    fn static_description() -> String {
        "A text-based output for rectangular boards, using unicode characters for pieces and different (background) colors as in a chessboard".to_string()
    }
}

impl<B: RectangularBoard> OutputBuilder<B> for ChessOutputBuilder {
    fn for_engine(&mut self, _state: &dyn GameState<B>) -> Res<OutputBox<B>> {
        Ok(Box::<ChessOutput>::default())
    }

    fn add_option(&mut self, _option: String) -> Res<()> {
        bail!("The {} output doesn't accept any options", self.long_name())
    }
}
