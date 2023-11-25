use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

use strum_macros::Display;

use crate::games::{Board, CreateGraphics, GraphicsList, RectangularBoard, RectangularCoordinates};
use crate::play::MatchManager;
use crate::ui::no_graphic::NoGraphics;
use crate::ui::pretty::PrettyUI;
use crate::ui::text_ui::Display::{Ascii, Fen, Pgn, Uci, Unicode};
use crate::ui::text_ui::TextUI;

pub mod no_graphic;
pub mod pretty;
pub mod text_ui;

#[derive(Debug, Display, Eq, PartialEq)]
pub enum Message {
    Info,
    Warning,
    Error,
}

impl Message {
    fn message_prefix(self) -> &'static str {
        match self {
            Message::Info => "",
            Message::Warning => "Warning: ",
            Message::Error => "Error: ",
        }
    }
}

// TODO: Allow the user to abort / change settings etc? Should probably go in a different trait then
pub trait Graphics<B: Board>: Debug + 'static {
    // TODO: Try to remove the dyn to see if rust compiles that (probably doesn't)
    fn show(&mut self, m: &dyn MatchManager<B>);

    fn display_message(&mut self, typ: Message, message: &str);
}

// A UI can also get a move, which is necessary for a human player
pub trait UI<B: Board>: Graphics<B> {
    fn get_move(&mut self, board: &B) -> B::Move;
}

pub type GraphicsHandle<B> = Rc<RefCell<dyn Graphics<B>>>;

pub type UIHandle<B> = Rc<RefCell<dyn UI<B>>>;

pub fn to_graphics_handle<B: Board, G: Graphics<B>>(graphics: G) -> GraphicsHandle<B> {
    Rc::new(RefCell::new(graphics))
}

pub fn to_ui_handle<B: Board, U: UI<B>>(ui: U) -> UIHandle<B> {
    Rc::new(RefCell::new(ui))
}

// Some default implementations (I hate this)
pub struct RequiredGraphics {}

impl<B: Board> GraphicsList<B> for RequiredGraphics {
    fn list_graphics() -> Vec<(String, CreateGraphics<B>)> {
        vec![
            ("none".to_string(), |_| {
                to_graphics_handle(NoGraphics::default())
            }),
            (
                "text".to_string(),
                |_| to_graphics_handle(TextUI::default()),
            ),
            ("ascii".to_string(), |_| {
                to_graphics_handle(TextUI::new(Ascii))
            }),
            ("unicode".to_string(), |_| {
                to_graphics_handle(TextUI::new(Unicode))
            }),
            ("fen".to_string(), |_| to_graphics_handle(TextUI::new(Fen))),
            ("uci".to_string(), |_| to_graphics_handle(TextUI::new(Uci))),
            ("pgn".to_string(), |_| to_graphics_handle(TextUI::new(Pgn))),
        ]
    }
}

pub struct NormalGraphics {}

impl<B: RectangularBoard> GraphicsList<B> for NormalGraphics
where
    B::Coordinates: RectangularCoordinates,
{
    fn list_graphics() -> Vec<(String, CreateGraphics<B>)> {
        let mut graphics = RequiredGraphics::list_graphics();
        graphics.push(("pretty".to_string(), |_| {
            to_graphics_handle(PrettyUI::default())
        }));
        graphics
    }
}
