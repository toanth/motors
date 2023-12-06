use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

use strum_macros::Display;

use crate::games::{Board, CreateGraphics, GraphicsList, RectangularBoard, RectangularCoordinates};
use crate::play::MatchManager;
use crate::ui::logger::Logger;
use crate::ui::no_graphic::NoGraphics;
use crate::ui::pretty::PrettyUI;
use crate::ui::text_ui::DisplayType::{Ascii, Fen, Pgn, Uci, Unicode};
use crate::ui::text_ui::TextUI;

pub mod logger;
pub mod no_graphic;
pub mod pretty;
pub mod text_ui;

#[derive(Debug, Display, Eq, PartialEq, Copy, Clone)]
pub enum Message {
    Info,
    Warning,
    Error,
    Debug,
}

impl Message {
    fn message_prefix(self) -> &'static str {
        match self {
            Message::Info => "",
            Message::Warning => "Warning: ",
            Message::Error => "Error: ",
            Message::Debug => "Debug: ",
        }
    }
}

// TODO: Allow the user to abort / change settings etc? Should probably go in a different trait then
pub trait Graphics<B: Board>: Debug + 'static {
    // TODO: Try to remove the dyn to see if rust compiles that (probably doesn't)
    fn show(&mut self, m: &dyn MatchManager<B>) {
        println!("{}", self.as_string(m));
    }

    fn as_string(&mut self, m: &dyn MatchManager<B>) -> String;

    fn display_message_simple(&mut self, typ: Message, message: &str);

    fn display_message(&mut self, _m: &dyn MatchManager<B>, typ: Message, message: &str) {
        if matches!(typ, Message::Debug) && !_m.debug_mode() {
            return;
        }
        self.display_message_simple(typ, message);
    }
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
            ("logger".to_string(), |stream| {
                // TODO: CreateGraphics should return a `Result<>` to account for invalid input
                to_graphics_handle(
                    Logger::from_str(stream)
                        .or_else(|err| {
                            eprintln!(
                                "Error while setting log stream, falling back to default: {err}'"
                            );
                            Logger::from_str("")
                        })
                        .unwrap(),
                )
            }),
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
