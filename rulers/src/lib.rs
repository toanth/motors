use crate::command::UgiCommand::*;
use crate::command::{ExtraUgiCommands, UgiCommand};
use crate::response::UgiResponse;
use crate::response::UgiResponse::*;
use crate::UgiState::*;
use anyhow::bail;
use colored::Colorize;
use derive_more::Display;
use std::error::Error;
use std::fmt::{Debug, Formatter};

pub mod command;
pub mod command_response;
pub mod engine;
pub mod response;

pub type Res<T> = anyhow::Result<T>;

#[derive(Debug, Clone)]
pub struct Variation(Vec<String>);

pub trait CustomUgiState: Debug + Clone {}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Display)]
pub enum UgiState {
    #[default]
    StartNonUgi,
    // called `Initial` in `https://expositor.dev/uci/doc/uci-draft-1.pdf` because it's the initial UCI state
    Initial,
    Idle,
    SyncOrPing,
    Active,
    Halt,
    QuitNonUgi,
    Custom,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub enum Protocol {
    #[default]
    UCI,
    UAI,
    UGI,
}

impl Display for Protocol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Protocol::UCI => "uci",
            Protocol::UAI => "uai",
            Protocol::UGI => "ugi",
        };
        write!(f, "{str}")
    }
}

#[derive(Debug, Clone)]
pub struct UgiDFA<T: ExtraUgiCommands> {
    state: UgiState,
    previous_state: UgiState,
    pub proto: Protocol,
    // Because the custom transitions are checked first, they effectively overwrite the default behavior
    pub custom_command_transition: fn(UgiState, &mut UgiCommand<T>) -> Option<Res<UgiState>>,
    pub custom_response_transition: fn(UgiState, &mut UgiResponse) -> Option<Res<UgiState>>,
}

impl<T: ExtraUgiCommands> UgiDFA<T> {
    pub fn transition(&mut self, new_state: UgiState) {
        self.previous_state = self.state;
        self.state = new_state;
    }

    fn state(&self) -> UgiState {
        self.state
    }

    pub fn transition_command(&mut self, mut command: UgiCommand<T>) -> Res<()> {
        let previous = self.state;
        self.transition_command_impl(command)?;
        // the previous state gets reset to the current state on a client command, but not on an engine response
        self.previous_state = previous;
        Ok(())
    }

    pub fn transition_command_impl(&mut self, mut command: UgiCommand<T>) -> Res<()> {
        if let Some(res) = (self.custom_command_transition)(self.state, &mut command) {
            self.previous_state = self.state;
            self.transition(res?);
        }
        if matches!(command, UgiCommand::Empty) {
            return Ok(());
        }
        match self.state {
            StartNonUgi => {
                if matches!(command, InitialUgi(_)) {
                    self.transition(Initial);
                }
                Ok(())
            }
            Initial => {
                bail!(
                    "The Client can't send a '{0}' command until it has received '{0}ok'",
                    self.proto
                )
            }
            Idle => self.transition_idle_command(command),
            SyncOrPing => {
                bail!("The client can't send another command while waiting for 'readyok'")
            }
            Active => {
                if matches!(command, |IsReady(_)) {
                    self.transition(SyncOrPing);
                    Ok(())
                } else if matches!(command, Stop(_)) {
                    self.transition(Halt);
                    Ok(())
                } else {
                    bail!("The client can only send 'isready' or 'stop' while the engine is searching")
                }
            }
            Halt => {
                bail!("Afer a 'stop' command, the client must wait until it receives 'bestmove' before it can send another command")
            }
            Custom => {
                bail!("Unsupported command in custom state")
            }
            // technically, the client can do whatever now, but since a UGI engine would just ignore that there's no point
            QuitNonUgi => {
                bail!("The client should not send another command after 'quit', this will just be ignored")
            }
        }
    }

    fn transition_idle_command(&mut self, command: UgiCommand<T>) -> Res<()> {
        match command {
            Debug(_) | SetOption(_) | Register(_) | NewGame(_) | Position(_) | Stop(_) => Ok(()),
            Quit(_) => {
                self.transition(QuitNonUgi);
                Ok(())
            }
            IsReady(_) => {
                self.transition(SyncOrPing);
                Ok(())
            }
            Go(_) | PonderHit(_) => {
                self.transition(Active);
                Ok(())
            }
            Additional(x) => {
                bail!("Unsupported additional operation '{x}'")
            }
            UgiCommand::Empty => Ok(()),
            InitialUgi(_) => bail!("The client can't send the initial 'ugi' command again"),
        }
    }

    /// Technically, an engine message can never cause an error because the client should just ignore it.
    /// But this is not what these tests assume by default (although that can be changed through
    /// the custom response transition function)
    pub fn transition_response(&mut self, mut response: UgiResponse) -> Res<()> {
        if let Some(res) = (self.custom_response_transition)(self.state, &mut response) {
            self.transition(res?);
        }
        if matches!(response, UgiResponse::Empty) {
            return Ok(());
        }
        match self.state {
            StartNonUgi => {}
            Initial => {
                if matches!(response, UgiOk(_)) {
                    self.transition(Idle);
                } else if !matches!(response, Id(_) | Option_(_) | Protocol(_) | Info(_)) {
                    bail!("Only '{}ok', 'id', 'option', 'protocol' or 'info' messages are allowed in the initial state", self.proto)
                }
            }
            Idle => {
                if !matches!(response, Info(_)) {
                    bail!("Only 'info' is allowed in the idle state")
                }
            }
            SyncOrPing => {
                if matches!(response, BestMove(_)) && self.previous_state == Active {
                    self.transition(Idle);
                } else if matches!(response, ReadyOk(_)) {
                    self.transition(self.previous_state);
                } else if !matches!(response, Info(_)) {
                    bail!("Only 'info' and 'readyok' messages are allowed after an 'isready' command; \
                    if the engine is searching 'bestmove' is also allowed")
                }
            }
            Active => {
                if matches!(response, BestMove(_)) {
                    self.transition(Idle);
                } else if !matches!(response, Info(_)) {
                    bail!("Only 'bestmove' and 'info' are allowed while the engine is searching")
                }
            }
            Halt => {
                if matches!(response, BestMove(_)) {
                    self.transition(Idle)
                } else if !matches!(response, Info(_)) {
                    bail!("Only 'bestmove' or 'info' are allowed after the engine has received a 'stop' command")
                }
            }
            Custom => {
                bail!("Unsupported command in custom state")
            }
            QuitNonUgi => {
                // technically, the engine shouldn't send anything now, but it's possible that we've just transitioned here
                // and the engine started sending this before it realized that the stop command had been sent.
                let mut copy = self.clone();
                copy.state = self.previous_state;
                return copy.transition_response(response);
            }
        }
        Ok(())
    }
}
