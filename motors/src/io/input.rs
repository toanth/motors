/*
 *  Motors, a collection of board game engines.
 *  Copyright (C) 2024 ToTheAnd
 *
 *  Motors is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Motors is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Motors. If not, see <https://www.gnu.org/licenses/>.
 */
use crate::io::autocomplete::CommandAutocomplete;
use crate::io::input::InputEnum::{Interactive, NonInteractive};
use crate::io::{AbstractEngineUgi, AbstractEngineUgiState, EngineUGI};
use gears::colored::Colorize;
use gears::games::Color;
use gears::general::board::Board;
use gears::general::common::Res;
use gears::general::common::anyhow::{anyhow, bail};
use gears::output::OutputOpts;
use inquire::Text;
use std::io::{IsTerminal, stdin, stdout};
use std::rc::Rc;

trait GetLine<B: Board> {
    fn get_line(&mut self, ugi: &mut EngineUGI<B>) -> Res<String>;
}

#[derive(Debug)]
struct InteractiveInput<B: Board> {
    autocompletion: CommandAutocomplete<B>,
}

impl<B: Board> GetLine<B> for InteractiveInput<B> {
    fn get_line(&mut self, ugi: &mut EngineUGI<B>) -> Res<String> {
        // If reading the input failed, always terminate. This probably means that the pipe is broken or similar,
        // so there's no point in continuing.
        // Since Inquire doesn't seem to have an option to do anything about this (like re-drawing the prompt after each line of output),
        // we just disable it while a `go` command is running

        self.autocompletion.state.go_state = ugi.state.go_state.clone();
        if ugi.state.is_currently_searching() {
            ugi.write_ugi(&format_args!(" [{0} Type '{1}' to cancel]", "Searching...".bold(), "stop".bold()));
            let pv_spacer = if ugi.state.pos().active_player().is_first() { "" } else { "    " };
            ugi.write_ugi(&format_args!(
                "{}",
                format!(
                    "\nIter    Seldepth    Score      Time       Nodes   (New)     NPS  Branch     TT     {pv_spacer}PV"
                )
                .bold(),
            ));
            NonInteractiveInput::default().get_line(ugi)
        } else {
            // not very efficient, but that doesn't really matter here
            let options = ugi.get_options();
            self.autocompletion.state.options = Rc::new(options);
            let help = "Type 'help' for a list of commands, '?' for a list of moves";
            let prompt = "Enter a command, move, variation, FEN or PGN:".bold().to_string();
            Ok(if let Some(failed) = &ugi.failed_cmd {
                Text::new(&"Please retry (press Ctrl+C to discard input)".bold().to_string())
                    .with_help_message(help)
                    .with_autocomplete(self.autocompletion.clone())
                    .with_initial_value(failed)
                    .prompt()?
            } else {
                Text::new(&prompt).with_help_message(help).with_autocomplete(self.autocompletion.clone()).prompt()?
            })
        }
    }
}

impl<B: Board> InteractiveInput<B> {
    fn new(ugi: &mut EngineUGI<B>) -> Self {
        let res = Self { autocompletion: CommandAutocomplete::new(ugi) };
        // technically, we could also use an inquire formatter, but that doesn't seem to handle multi-line messages well
        ugi.print_board(OutputOpts::default());
        res
    }
}

#[derive(Debug, Default)]
struct NonInteractiveInput {}

impl<B: Board> GetLine<B> for NonInteractiveInput {
    fn get_line(&mut self, _ugi: &mut EngineUGI<B>) -> Res<String> {
        let mut input = String::new();
        let count = stdin().read_line(&mut input)?;
        if count == 0 {
            bail!("Read 0 bytes. Terminating the program.")
        }
        Ok(input)
    }
}

#[derive(Debug)]
enum InputEnum<B: Board> {
    Interactive(InteractiveInput<B>),
    NonInteractive(NonInteractiveInput),
}

#[derive(Debug)]
pub struct Input<B: Board> {
    typ: InputEnum<B>,
}

impl<B: Board> Input<B> {
    pub fn new(mut interactive: bool, ugi: &mut EngineUGI<B>) -> (Self, bool) {
        if !stdout().is_terminal() {
            interactive = false;
        }
        let typ = if interactive {
            Interactive(InteractiveInput::new(ugi))
        } else {
            NonInteractive(NonInteractiveInput::default())
        };

        (Self { typ }, interactive)
    }

    pub fn set_interactive(&mut self, value: bool, ugi: &mut EngineUGI<B>) {
        if value {
            if !matches!(self.typ, Interactive(_)) {
                self.typ = Interactive(InteractiveInput::new(ugi));
            }
        } else {
            self.typ = NonInteractive(NonInteractiveInput::default());
        }
    }

    pub fn get_line(&mut self, ugi: &mut EngineUGI<B>) -> Res<String> {
        match &mut self.typ {
            Interactive(i) => match i.get_line(ugi) {
                Ok(res) => Ok(res),
                Err(err) => {
                    self.set_interactive(false, ugi);
                    self.get_line(ugi).map_err(|err2| {
                        anyhow!("{err}. After falling back to non-interactive backend, another error occurred: {err2}")
                    })
                }
            },
            NonInteractive(n) => n.get_line(ugi).map_err(|err| anyhow!("Couldn't read input: {err}")),
        }
    }
}
