use std::sync::{Arc, Mutex};
use dyn_clone::DynClone;
use gears::games::Board;
use gears::general::common::{EntityList, NamedEntity, Res, StaticallyNamedEntity};
use gears::output::{OutputBox, OutputBuilder};
use crate::play::ugi_client::Client;

pub mod text_input;

//
// /// Factory to create the specified Output and (when the `for_gui` method is called) Input.
// pub trait UIBuilder<B: Board>: OutputBuilder<B> {
//     /// logically, the following method consumes self, but unfortunately passing `self` would make this trait not object safe
//     /// as of the current Rust version.
//     fn for_gui(&mut self, gui: Arc<Mutex<UgiGui<B>>>) -> OutputBox<B>;
// }
//
// impl<B: Board, T: OutputBuilder<B>> UIBuilder<B> for T {
//     fn for_gui(&mut self, gui: Arc<Mutex<UgiGui<B>>>) -> OutputBox<B> {
//         todo!()
//     }
// }


/// An `Input` tells the MatchState what to do. It isn't necessarily just a way for a human to enter input,
/// it can also automatically run games, like a SPRT runner. Since the `Input` is in complete control of the match,
/// this trait is almost empty
pub trait Input<B: Board> : StaticallyNamedEntity {
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
            Err(format!("Unrecognized option {option} for match input '{}'", self.long_name()))
        }
    }
}

pub type InputList<B> = EntityList<Box<dyn InputBuilder<B>>>;