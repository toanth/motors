use std::fmt::Debug;
use std::str::SplitWhitespace;

use dyn_clone::DynClone;
use strum_macros::Display;

use crate::{GameOverReason, GameState, MatchResult, MatchStatus};
use crate::games::{Board, OutputList, RectangularBoard, RectangularCoordinates};
use crate::general::common::{NamedEntity, Res};
use crate::output::logger::LoggerBuilder;
use crate::output::Message::Info;
use crate::output::pretty::PrettyUIBuilder;
use crate::output::text_output::DisplayType::*;
use crate::output::text_output::TextOutputBuilder;
use crate::search::SearchInfo;

pub mod logger;
pub mod pretty;
pub mod text_output;

#[derive(Debug, Display, Eq, PartialEq, Copy, Clone)]
pub enum Message {
    Info,
    Warning,
    /// Text-based outputs print Error and Debug messages to `stderr` and all other types of messages to `stdout`
    Error,
    /// Also printed to `stderr` on text-based outputs.
    Debug,
}

impl Message {
    fn message_prefix(self) -> &'static str {
        match self {
            Message::Info => "",
            Message::Warning => "Warning:",
            Message::Error => "Error:",
            Message::Debug => "Debug:",
        }
    }
}

pub fn game_over_message(result: MatchResult) -> String {
    let mut msg = String::new();
    use std::fmt::Write;
    writeln!(msg, "!!! {} !!!", result.result).unwrap();
    match result.reason {
        GameOverReason::Normal => msg,
        GameOverReason::Adjudication(reason) => {
            writeln!(msg, "({reason})").unwrap();
            msg
        }
    }
}

/// A UI prints the board.The Ugi Gui Match calls `make_interactive` once at the start, which makes the UI listen
/// to allow handling inputs such as moves and match making.
/// The UI object lives in the same thread as the UgiGui, but usually uses multithreading internally to allow processing user input when
/// make_interactive has been called. Otherwise, it can just print the board on demand in the same thread (this is the case for the logger).
// TODO: Split into Input and Output parts to avoid reference cycles; an `Input` has a reference to the UgiGui, and the UgiGui has a Vec of Outputs
/// There is no trait for Input because it's literally just something that contains a `Weak<Mutex<UgiGui>>`
pub trait Output<B: Board>: NamedEntity + Debug + Send + 'static {
    fn show(&mut self, m: &dyn GameState<B>) {
        println!("{}", self.as_string(m));
    }

    fn inform_game_over(&mut self, m: &dyn GameState<B>) {
        match m.match_status() {
            MatchStatus::Over(res) => self.display_message_simple(Info, &game_over_message(res)),
            _ => panic!("Internal error: the match isn't over"),
        }
    }

    fn is_logger(&self) -> bool {
        false
    }

    fn as_string(&self, m: &dyn GameState<B>) -> String;

    fn write_ugi_output(&mut self, _message: &str, _player: Option<&str>) {
        // do nothing (most UIs don't log all UGI commands)
    }

    fn write_ugi_input(&mut self, _message: SplitWhitespace, _player: Option<&str>) {
        // do nothing (most UIs don't log all UGI commands)
    }

    fn display_message_simple(&mut self, typ: Message, message: &str);

    // TODO: Remove or rename

    fn display_message(&mut self, m: &dyn GameState<B>, typ: Message, message: &str) {
        if matches!(typ, Message::Debug) && !m.debug_info_enabled() {
            return;
        }
        self.display_message_simple(typ, message);
    }

    fn update_engine_info(&mut self, engine_name: &str, info: &SearchInfo<B>) {
        self.display_message_simple(Info, &format!("{engine_name}: {}", info))
    }
}

pub trait OutputBuilderOption<B: Board> {
    fn set_option(&mut self, option: &str) -> Res<()>;
}

/// Factory to create the specified Output; the `monitors` crate has a `UIBuilder` trait that inherits from this.
pub trait OutputBuilder<B: Board>: NamedEntity + DynClone + Send {
    fn for_engine(&self, state: &dyn GameState<B>) -> Res<OutputBox<B>>;

    fn for_client(&self, state: &dyn GameState<B>) -> Res<OutputBox<B>> {
        self.for_engine(state)
    }

    fn add_option(&mut self, option: String) -> Res<()>;

    fn add_options(&mut self, options: &[String]) -> Res<()> {
        for option in options {
            self.add_option(option.clone())?
        }
        Ok(())
    }
}

pub type OutputBox<B> = Box<dyn Output<B>>;

pub fn required_outputs<B: Board>() -> OutputList<B> {
    vec![
        Box::new(TextOutputBuilder::new(Ascii)),
        Box::new(TextOutputBuilder::new(Unicode)),
        Box::new(TextOutputBuilder::new(Fen)),
        Box::new(TextOutputBuilder::new(Uci)),
        Box::new(TextOutputBuilder::new(Ugi)),
        Box::new(TextOutputBuilder::new(Pgn)),
        #[allow(clippy::box_default)]
        Box::new(LoggerBuilder::default()),
    ]
}

pub fn normal_outputs<B: RectangularBoard>() -> OutputList<B>
where
    <B as Board>::Coordinates: RectangularCoordinates,
{
    let mut res = required_outputs();
    res.push(Box::<PrettyUIBuilder>::default());
    res
}