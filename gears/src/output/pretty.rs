use std::fmt::{Display, Write};
use std::io::stdout;

use colored::{Color, Colorize};

use crate::games::{AbstractPieceType, ColoredPiece, ColoredPieceType, Coordinates};
use crate::general::board::{Board, ColPieceType, RectangularBoard};
use crate::general::common::{IterIntersperse, NamedEntity, Res, StaticallyNamedEntity};
use crate::general::moves::Move;
use crate::general::squares::RectangularCoordinates;
use crate::output::text_output::{TextStream, TextWriter};
use crate::output::Message::Info;
use crate::output::{AbstractOutput, Message, Output, OutputBox, OutputBuilder};
use crate::{games, GameState};

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
    piece: ColPieceType<B>,
    square: B::Coordinates,
    last_move: Option<B::Move>,
) -> String
where
    B::Coordinates: RectangularCoordinates,
{
    let p1_bg_col = Color::White;
    let p2_bg_col = Color::Black;
    let p1_piece_col = Color::Green;
    let p2_piece_col = Color::Cyan;
    let move_bg_color = Color::Red;
    let symbol = piece.uncolor().to_utf8_char();
    let no_coordinates = B::Coordinates::no_coordinates();
    let bg_color = if square == last_move.map_or(no_coordinates, B::Move::src_square)
        || square == last_move.map_or(no_coordinates, B::Move::dest_square)
    {
        move_bg_color
    } else if (square.row() + square.column()) % 2 == 0 {
        p2_bg_col
    } else {
        p1_bg_col
    };

    if piece == ColPieceType::<B>::empty() {
        "  ".to_string().color(Color::Black)
    } else if piece.color().unwrap() == <B::Color as games::Color>::first() {
        (symbol.to_string() + " ").color(p1_piece_col)
    } else {
        (symbol.to_string() + " ").color(p2_piece_col)
    }
    .on_color(bg_color)
    .to_string()
}

impl NamedEntity for PrettyUI {
    fn short_name(&self) -> String {
        PrettyUIBuilder::static_short_name().to_string()
    }

    fn long_name(&self) -> String {
        PrettyUIBuilder::static_long_name().to_string()
    }

    fn description(&self) -> Option<String> {
        Some(PrettyUIBuilder::static_description())
    }
}

impl AbstractOutput for PrettyUI {
    fn output_name(&self) -> String {
        self.writer.stream.name()
    }

    fn display_message(&mut self, typ: Message, message: &str) {
        self.writer.display_message(typ, message);
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
            ('A'..)
                .take(pos.width() as usize)
                .intersperse_(' ')
                .collect::<String>()
        );
        res
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct PrettyUIBuilder {}

impl StaticallyNamedEntity for PrettyUIBuilder {
    fn static_short_name() -> impl Display {
        "pretty"
    }

    fn static_long_name() -> String {
        "Pretty Text-based UI".to_string()
    }

    fn static_description() -> String {
        "A text-based UI for rectangular boards, using unicode characters for pieces and different (background) colors".to_string()
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
