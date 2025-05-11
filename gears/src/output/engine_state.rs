/*
 *  Gears, a collection of board games.
 *  Copyright (C) 2024 ToTheAnd
 *
 *  Gears is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Gears is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Gears. If not, see <https://www.gnu.org/licenses/>.
 */
use crate::GameState;
use crate::general::board::Board;
use crate::general::common::{NamedEntity, Res, StaticallyNamedEntity};
use crate::output::Message::Info;
use crate::output::text_output::{TextStream, TextWriter};
use crate::output::{AbstractOutput, Message, Output, OutputBox, OutputBuilder, OutputOpts};
use anyhow::bail;
use std::fmt;
use std::fmt::Display;
use std::io::stdout;

#[derive(Debug)]
pub(super) struct EngineStateOutput {
    writer: TextWriter,
}

impl Default for EngineStateOutput {
    fn default() -> Self {
        Self { writer: TextWriter::new_for(TextStream::Stdout(stdout()), vec![Info]) }
    }
}

impl NamedEntity for EngineStateOutput {
    fn short_name(&self) -> String {
        EngineStateOutputBuilder::static_short_name().to_string()
    }

    fn long_name(&self) -> String {
        EngineStateOutputBuilder::static_long_name().to_string()
    }

    fn description(&self) -> Option<String> {
        Some(EngineStateOutputBuilder::static_description())
    }
}

impl AbstractOutput for EngineStateOutput {
    fn output_name(&self) -> String {
        self.writer.stream.name()
    }

    fn display_message(&mut self, typ: Message, message: &fmt::Arguments) {
        self.writer.display_message(typ, message);
    }
}

impl<B: Board> Output<B> for EngineStateOutput {
    fn as_string(&self, m: &dyn GameState<B>, _opts: OutputOpts) -> String {
        m.print_engine_state().unwrap_or_else(|e| e.to_string())
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct EngineStateOutputBuilder {}

impl StaticallyNamedEntity for EngineStateOutputBuilder {
    fn static_short_name() -> impl Display {
        "engine_state"
    }

    fn static_long_name() -> String {
        "Internal Engine State Output".to_string()
    }

    fn static_description() -> String {
        "A human readable display of parts of the engine's internal state, if supported by the engine".to_string()
    }
}

impl<B: Board> OutputBuilder<B> for EngineStateOutputBuilder {
    fn for_engine(&mut self, _state: &dyn GameState<B>) -> Res<OutputBox<B>> {
        Ok(Box::<EngineStateOutput>::default())
    }

    fn add_option(&mut self, _option: String) -> Res<()> {
        bail!("The {} output doesn't accept any options", self.long_name())
    }
}
