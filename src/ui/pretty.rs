use std::fmt::Write;

use colored::{Color, Colorize};

use crate::games::Color::White;
use crate::games::{
    AbstractPieceType, Board, ColoredPiece, ColoredPieceType, Coordinates, Move, RectangularBoard,
    RectangularCoordinates,
};
use crate::play::MatchManager;
use crate::ui::text_ui::{display_message, get_move};
use crate::ui::{Graphics, Message, UI};

#[derive(Debug, Default)]
pub struct PrettyUI {}

fn color<B: Board>(
    piece: <B::Piece as ColoredPiece>::ColoredPieceType,
    square: B::Coordinates,
    last_move: Option<B::Move>,
) -> String
where
    B::Coordinates: RectangularCoordinates,
{
    // if (square.row() + square.column()) % 2 == 0 {
    //     '■'.to_string() //"▒".to_string()
    // } else {
    //     '□'.to_string() //"░".to_string()
    // }
    let white_bg_col = Color::White;
    let black_bg_col = Color::Black;
    let white_piece_col = Color::Green;
    let black_piece_col = Color::Cyan;
    let move_bg_color = Color::Red;
    let symbol = piece.to_utf8_char();
    let no_coordinates = B::Coordinates::no_coordinates();
    let bg_color = if square == last_move.map_or(no_coordinates, |m| m.from_square())
        || square == last_move.map_or(no_coordinates, |m| m.to_square())
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

impl<B: RectangularBoard> Graphics<B> for PrettyUI
where
    B::Coordinates: RectangularCoordinates,
{
    fn show(&mut self, m: &dyn MatchManager<B>) {
        let pos = m.board();
        let last_move = m.last_move();
        if last_move.is_none() {
            println!("Starting new game!");
        }
        for y in 0..pos.height() {
            let mut line = " ".to_string();
            for x in 0..pos.width() {
                let square = B::Coordinates::from_row_column(y, x).flip_up_down(pos.size());
                let piece = pos.piece_on(square);
                write!(
                    &mut line,
                    "{0}",
                    color::<B>(piece.colored_piece_type(), square, last_move)
                )
                .unwrap();
            }
            writeln!(line, " {0}", pos.height() - y).unwrap();
            print!("{line}");
        }
        println!(
            " {0}",
            ('A'..)
                .take(pos.width())
                .intersperse(' ')
                .collect::<String>()
        );
    }

    fn display_message(&mut self, typ: Message, message: &str) {
        display_message(typ, message)
    }
}

impl<B: RectangularBoard> UI<B> for PrettyUI
where
    B::Coordinates: RectangularCoordinates,
{
    fn get_move(&mut self, board: &B) -> B::Move {
        get_move::<B, PrettyUI>(self, board)
    }
}
