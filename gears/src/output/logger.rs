
use std::io::{Write};


use std::str::SplitWhitespace;

use itertools::Itertools;

use crate::games::{Board};
use crate::{GameState};
use crate::general::common::{NamedEntity, Res, StaticallyNamedEntity};
use crate::output::{Message, Output, OutputBox, OutputBuilder};
use crate::output::text_output::{DisplayType, TextOutputBuilder, TextStream};

#[derive(Debug)]
pub struct Logger<B: Board> {
    pub stream: TextStream,
    pub output: OutputBox<B>,
}

impl<B: Board> Logger<B> {
    fn new(stream: TextStream) -> Self {
        let mut res = Self {
            stream,
            output: TextOutputBuilder::new(DisplayType::Fen).build(false).unwrap(),
        };
        res.display_message_simple(Message::Info, &format!("[Starting logging at {}]", chrono::offset::Utc::now().to_rfc2822()));
        res
    }

    fn from_words(words: SplitWhitespace, fallback_name: &str) -> Res<Self> {
        Ok(Self::new(TextStream::from_words(words, fallback_name)?))
    }
}

impl<B: Board> NamedEntity for Logger<B> {
    fn short_name(&self) -> &str {
        LoggerBuilder::static_short_name()
    }

    fn long_name(&self) -> &str {
        LoggerBuilder::static_long_name()
    }

    fn description(&self) -> Option<&str> {
        Some(LoggerBuilder::static_description())
    }
}

impl<B: Board> Output<B> for Logger<B> {
    fn show(&mut self, m: &dyn GameState<B>) {
        let msg = self.as_string(m);
        self.stream.write("Board:\n", &msg);
    }

    fn is_logger(&self) -> bool {
        true
    }

    fn as_string(&self, m: &dyn GameState<B>) -> String {
        self.output.as_string(m)
    }

    fn write_ugi_output(&mut self, message: &str, player: Option<&str>) {
        match player {
            None => self.stream.write("<", message),
            Some(name) => self.stream.write(&format!("<({name})"), message)
        }
    }

    fn write_ugi_input(&mut self, mut message: SplitWhitespace, player: Option<&str>) {
        match player {
            None => self.stream.write(">", &message.join(" ")),
            Some(name) => self.stream.write(&format!("({name})>"), &message.join(" "))
        }
    }

    fn display_message_simple(&mut self, typ: Message, message: &str) {
        self.stream.write(typ.message_prefix(), message);
    }

    fn display_message(&mut self, m: &dyn GameState<B>, typ: Message, message: &str) {
        self.display_message_simple(typ, message);
        match typ {
            Message::Info => {}
            _ => {
                let str = self.as_string(m);
                self.stream.write(typ.message_prefix(), &str)
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct LoggerBuilder {
    stream_name: String,
    options: Vec<String>,
}

impl LoggerBuilder {
    pub fn new(stream: &str) -> Self {
        Self {
            stream_name: stream.to_string(),
            options: vec![],
        }
    }

    pub fn from_words(mut words: SplitWhitespace) -> Self {
        Self::new(&words.join(" "))
    }

    pub fn build<B: Board>(&self, name: &str) -> Res<OutputBox<B>> {
        let fallback_name = format!("debug_output_{name}.log");
        Ok(Box::new(Logger::from_words(self.stream_name.split_whitespace(), &fallback_name).unwrap_or_else(|err| {
            eprintln!("Error while setting log stream, falling back to default: {err}'");
            Logger::from_words("".split_whitespace(), &fallback_name).unwrap()
        })))
    }
}

impl StaticallyNamedEntity for LoggerBuilder {
    fn static_short_name() -> &'static str {
        "logger"
    }

    fn static_long_name() -> &'static str {
        "UCI Logger"
    }

    fn static_description() -> &'static str {
        "A logger for all UCI communication"
    }
}

impl<B: Board> OutputBuilder<B> for LoggerBuilder {
    fn for_engine(&self, state: &dyn GameState<B>) -> Res<OutputBox<B>> {
        // Use the (hopefully unique) name to ensure that engines or the GUI don't try to write to the same file if they both have
        // debug logging enabled, which happens if the --debug flag is passed to the GUI with two built-in engines.
        self.build(&format!("engine_{}", state.name()))
    }

    fn for_client(&self, state: &dyn GameState<B>) -> Res<OutputBox<B>> {
        self.build(state.name())
    }

    fn add_option(&mut self, option: String) -> Res<()> {
        self.options.push(option);
        Ok(())
    }
}
