pub mod bitboards;
pub mod common;
pub mod move_list;

pub mod perft;
pub mod squares;
mod tests;

// TODO: There are probably quite a few bugs in here from assuming that str.len() returns the number of characters
// TODO: Use .peekable() on iterators instead of custom solutions throughout the project
