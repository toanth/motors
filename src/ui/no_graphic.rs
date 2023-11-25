use std::fmt::Debug;

use crate::games::Board;
use crate::play::MatchManager;
use crate::ui::Message::Error;
use crate::ui::{Graphics, Message};

#[derive(Debug, Default)]
pub struct NoGraphics {}

impl<B: Board> Graphics<B> for NoGraphics {
    fn show(&mut self, _: &dyn MatchManager<B>) {
        // do nothing
    }

    fn display_message(&mut self, typ: Message, message: &str) {
        if typ == Error {
            eprintln!("{message}");
        }
    }
}
