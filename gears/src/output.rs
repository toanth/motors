use std::fmt;
use std::fmt::Debug;

use dyn_clone::DynClone;
use itertools::Itertools;
use strum::IntoEnumIterator;
use strum_macros::Display;

use crate::games::OutputList;
use crate::general::board::{Board, RectangularBoard};
use crate::general::common::{NamedEntity, Res, Tokens};
use crate::output::chess::ChessOutputBuilder;
use crate::output::engine_state::EngineStateOutputBuilder;
use crate::output::logger::LoggerBuilder;
use crate::output::text_output::{DisplayType, TextOutputBuilder};
use crate::output::Message::*;
use crate::search::SearchInfo;
use crate::{GameOverReason, GameState, MatchResult, MatchStatus};

pub mod chess;
pub mod engine_state;
pub mod logger;
pub mod pgn;
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
    use std::fmt::Write;
    let mut msg = String::new();
    writeln!(msg, "!!! {} !!!", result.result).unwrap();
    match result.reason {
        GameOverReason::Normal => msg,
        GameOverReason::Adjudication(reason) => {
            writeln!(msg, "({reason})").unwrap();
            msg
        }
    }
}

/// An `AbstractOutput` contains the parts of an `Output` that are independent of the `Board`
pub trait AbstractOutput: NamedEntity + Debug + Send + 'static {
    fn is_logger(&self) -> bool {
        false
    }

    /// True iff the output can print the board in some form, such as by outputting a FEN, PGN, diagram, or graphical representation.
    fn prints_board(&self) -> bool {
        true
    }

    fn output_name(&self) -> String;

    fn write_ugi_output(&mut self, _message: &fmt::Arguments, _player: Option<&str>) {
        // do nothing (most UIs don't log all UGI commands)
    }

    fn write_ugi_input(&mut self, _message: Tokens, _player: Option<&str>) {
        // do nothing (most UIs don't log all UGI commands)
    }

    fn display_message(&mut self, typ: Message, message: &fmt::Arguments);
}

#[derive(Debug, Default, Copy, Clone)]
pub struct OutputOpts {
    pub disable_flipping: bool,
}

impl OutputOpts {
    pub fn dont_flip() -> Self {
        Self {
            disable_flipping: true,
        }
    }
}

/// An Output prints the board and shows messages.
pub trait Output<B: Board>: AbstractOutput {
    fn show(&mut self, m: &dyn GameState<B>, opts: OutputOpts) {
        println!("{}", self.as_string(m, opts));
    }

    fn inform_game_over(&mut self, m: &dyn GameState<B>) {
        match m.match_status() {
            MatchStatus::Over(res) => {
                self.display_message(Info, &format_args!("{}", game_over_message(res)))
            }
            _ => panic!("Internal error: the match isn't over"),
        }
    }

    fn as_string(&self, m: &dyn GameState<B>, opts: OutputOpts) -> String;

    fn display_message_with_state(
        &mut self,
        _: &dyn GameState<B>,
        typ: Message,
        message: &fmt::Arguments,
    ) {
        self.display_message(typ, message);
    }

    fn update_engine_info(&mut self, engine_name: &str, info: &SearchInfo<B>) {
        self.display_message(Info, &format_args!("{engine_name}: {info}"));
    }
    fn upcast_box(self: Box<Self>) -> Box<dyn AbstractOutput>
    where
        Self: Sized,
    {
        self
    }
}

pub trait OutputBuilderOption<B: Board> {
    fn set_option(&mut self, option: &str) -> Res<()>;
}

/// Factory to create the specified Output; the `monitors` crate has a `UIBuilder` trait that inherits from this.
pub trait OutputBuilder<B: Board>: NamedEntity + DynClone + Send {
    fn for_engine(&mut self, state: &dyn GameState<B>) -> Res<OutputBox<B>>;

    fn for_client(&mut self, state: &dyn GameState<B>) -> Res<OutputBox<B>> {
        self.for_engine(state)
    }

    fn add_option(&mut self, option: String) -> Res<()>;

    fn add_options(&mut self, options: &[String]) -> Res<()> {
        for option in options {
            self.add_option(option.clone())?;
        }
        Ok(())
    }
}

pub type OutputBox<B> = Box<dyn Output<B>>;

#[must_use]
pub fn required_outputs<B: Board>() -> OutputList<B> {
    let mut res: OutputList<B> = vec![];
    for display_type in DisplayType::iter().dropping_back(1) {
        res.push(Box::new(TextOutputBuilder::new(display_type)));
    }
    res.push(Box::new(TextOutputBuilder::messages_for(
        vec![Warning, Error],
        "error",
    )));
    res.push(Box::new(TextOutputBuilder::messages_for(
        vec![Debug],
        "debug",
    )));
    res.push(Box::new(TextOutputBuilder::messages_for(
        vec![Info],
        "info",
    )));
    #[allow(clippy::box_default)]
    res.push(Box::new(LoggerBuilder::default()));
    res
}

#[must_use]
pub fn normal_outputs<B: RectangularBoard>(for_engine: bool) -> OutputList<B> {
    let mut res: OutputList<B> = vec![Box::<ChessOutputBuilder>::default()];
    if for_engine {
        res.push(Box::<EngineStateOutputBuilder>::default());
    }
    res.append(&mut required_outputs());
    res
}
