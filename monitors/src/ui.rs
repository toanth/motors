use crate::play::ugi_client::Client;
use dyn_clone::DynClone;
use gears::general::board::Board;
use gears::general::common::anyhow::bail;
use gears::general::common::{EntityList, NamedEntity, Res, StaticallyNamedEntity};
use std::sync::{Arc, Mutex};

pub mod text_input;

/// An `Input` tells the [`MatchState`] what to do. It isn't necessarily just a way for a human to enter input,
/// it can also automatically run games, like a SPRT runner. Since the `Input` is in complete control of the match,
/// this trait is almost empty
pub trait Input<B: Board>: StaticallyNamedEntity {
    fn assume_control(&mut self, ugi_client: Arc<Mutex<Client<B>>>);

    /// Called upon program termination. Should clean up and make sure that any threads are joined.
    /// An explicit method instead of relying on `drop` so that implementations can't forget this.
    fn join_threads(&mut self);
}

pub trait InputBuilder<B: Board>: NamedEntity + DynClone {
    fn build(&self) -> Box<dyn Input<B>>;

    fn set_option(&mut self, option: &str) -> Res<()> {
        if option.is_empty() {
            Ok(())
        } else {
            bail!(
                "Unrecognized option {option} for match input '{}'",
                self.long_name()
            )
        }
    }
}

pub type InputList<B> = EntityList<Box<dyn InputBuilder<B>>>;
