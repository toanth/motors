use std::fmt::Write;
use std::io::stdout;

use colored::{Color, Colorize};

use crate::games::Color::*;
use crate::games::{
    AbstractPieceType, Board, ColoredPiece, ColoredPieceType, Coordinates, Move, RectangularBoard,
};
use crate::general::common::{NamedEntity, Res, StaticallyNamedEntity};
use crate::general::squares::RectangularCoordinates;
use crate::output::text_output::{TextStream, TextWriter};
use crate::output::Message::Info;
use crate::output::{AbstractOutput, Message, Output, OutputBox, OutputBuilder};
use crate::GameState;

// TODO: Should be a BoardToString variant
#[derive(Debug)]
pub(super) struct PrettyUI {
    writer: TextWriter,
}

impl Default for PrettyUI {
    fn default() -> Self {
        Self {
            writer: TextWriter::new_for(TextStream::Stdout(stdout()), vec![Info]),
        }
    }
}

fn color<B: Board>(
    piece: <B::Piece as ColoredPiece>::ColoredPieceType,
    square: B::Coordinates,
    last_move: Option<B::Move>,
) -> String
where
    B::Coordinates: RectangularCoordinates,
{
    let white_bg_col = Color::White;
    let black_bg_col = Color::Black;
    let white_piece_col = Color::Green;
    let black_piece_col = Color::Cyan;
    let move_bg_color = Color::Red;
    let symbol = piece.uncolor().to_utf8_char();
    let no_coordinates = B::Coordinates::no_coordinates();
    let bg_color = if square == last_move.map_or(no_coordinates, |m| m.src_square())
        || square == last_move.map_or(no_coordinates, |m| m.dest_square())
    {
        move_bg_color
    } else if (square.row() + square.column()) % 2 == 0 {
        black_bg_col
    } else {
        white_bg_col
    };

    if piece == <B::Piece as ColoredPiece>::ColoredPieceType::empty() {
        "  ".to_string().color(Color::Black)
    } else if piece.color().unwrap() == White {
        (symbol.to_string() + " ").color(white_piece_col)
    } else {
        (symbol.to_string() + " ").color(black_piece_col)
    }
    .on_color(bg_color)
    .to_string()
}

impl NamedEntity for PrettyUI {
    fn short_name(&self) -> &str {
        PrettyUIBuilder::static_short_name()
    }

    fn long_name(&self) -> &str {
        PrettyUIBuilder::static_long_name()
    }

    fn description(&self) -> Option<&str> {
        Some(PrettyUIBuilder::static_description())
    }
}

impl AbstractOutput for PrettyUI {
    fn output_name(&self) -> String {
        self.writer.stream.name()
    }

    fn display_message(&mut self, typ: Message, message: &str) {
        self.writer.display_message(typ, message)
    }
}

impl<B: RectangularBoard> Output<B> for PrettyUI
where
    B::Coordinates: RectangularCoordinates,
{
    fn as_string(&self, m: &dyn GameState<B>) -> String {
        let mut res = String::default();
        let pos = m.get_board();
        let last_move = m.last_move();
        if last_move.is_none() {
            writeln!(res, "Starting new game!").unwrap();
        }
        for y in 0..pos.height() {
            let mut line = " ".to_string();
            for x in 0..pos.width() {
                let square = B::Coordinates::from_row_column(y, x).flip_up_down(pos.size());
                let piece = pos.colored_piece_on(square);
                write!(
                    &mut line,
                    "{0}",
                    color::<B>(piece.colored_piece_type(), square, last_move)
                )
                .unwrap();
            }
            writeln!(line, " {0}", pos.height() - y).unwrap();
            write!(res, "{line}").unwrap();
        }
        _ = writeln!(
            res,
            " {0}",
            itertools::intersperse(('A'..).take(pos.width() as usize), ' ').collect::<String>()
        );
        res
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct PrettyUIBuilder {}

impl StaticallyNamedEntity for PrettyUIBuilder {
    fn static_short_name() -> &'static str {
        "pretty"
    }

    fn static_long_name() -> &'static str {
        "Pretty Text-based UI"
    }

    fn static_description() -> &'static str {
        "A text-based UI for rectangular boards, using unicode characters for pieces and different (background) colors"
    }
}

impl<B: RectangularBoard> OutputBuilder<B> for PrettyUIBuilder
where
    B::Coordinates: RectangularCoordinates,
{
    fn for_engine(&mut self, _state: &dyn GameState<B>) -> Res<OutputBox<B>> {
        Ok(Box::<PrettyUI>::default())
    }

    fn add_option(&mut self, _option: String) -> Res<()> {
        Err(format!(
            "The {} output doesn't accept any options",
            self.long_name()
        ))
    }
}
