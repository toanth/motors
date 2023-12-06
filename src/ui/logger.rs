use std::fs::File;
use std::io::{stderr, stdout, BufWriter, Stderr, Stdout, Write};
use std::path::Path;

use crate::games::Board;
use crate::play::MatchManager;
use crate::ui::text_ui::{DisplayType, TextUI};
use crate::ui::{to_graphics_handle, Graphics, GraphicsHandle, Message};

/// `Option<Box<dyn Write>>` doesn't implement `Debug`, which is a problem because `UGI` should implement `Debug`.
#[derive(Debug)]
pub enum LogStream {
    None,
    File(BufWriter<File>),
    Stdout(Stdout),
    Stderr(Stderr),
}

impl LogStream {
    pub fn write(&mut self, prefix: &str, msg: &str) {
        match self {
            LogStream::None => {}
            LogStream::File(f) => _ = writeln!(f, "{prefix} {msg}"),
            LogStream::Stdout(out) => _ = writeln!(out, "{prefix} {msg}"),
            LogStream::Stderr(err) => _ = writeln!(err, "{prefix} {msg}"),
        }
    }
}

#[derive(Debug)]
pub struct Logger<B: Board> {
    pub stream: LogStream,
    pub graphics: GraphicsHandle<B>,
}

impl<B: Board> Logger<B> {
    // TODO: Support other text_ui enum variants eventually, such as pgn.
    pub fn new(stream: LogStream) -> Self {
        Self {
            stream,
            graphics: to_graphics_handle(TextUI::new(DisplayType::Fen)),
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        let stream = match s.trim() {
            "" => return Self::from_str("debug_output.log"),
            "none" => LogStream::None,
            "stdout" => LogStream::Stdout(stdout()),
            "stderr" => LogStream::Stderr(stderr()),
            s => {
                if !s.contains('.') {
                    return Err(format!("'{s}' does not appear to be a valid filename (it does not contain a '.'). \
             Expected either a filename, 'stdout', 'stderr', or 'none'."));
                }
                let path = Path::new(s);
                let file =
                    File::create(path).map_err(|err| format!("Couldn't create log file: {err}"))?;
                LogStream::File(BufWriter::new(file))
            }
        };
        Ok(Self::new(stream))
    }

    pub fn is_active(&self) -> bool {
        !matches!(self.stream, LogStream::None)
    }
}

impl<B: Board> Graphics<B> for Logger<B> {
    fn show(&mut self, m: &dyn MatchManager<B>) {
        let msg = self.as_string(m);
        self.stream.write("Board:\n", &msg);
    }

    fn as_string(&mut self, m: &dyn MatchManager<B>) -> String {
        self.graphics.borrow_mut().as_string(m)
    }

    fn display_message_simple(&mut self, typ: Message, message: &str) {
        self.stream.write(typ.message_prefix(), message);
    }

    fn display_message(&mut self, m: &dyn MatchManager<B>, typ: Message, message: &str) {
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
