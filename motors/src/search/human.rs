// use std::fmt::Debug;
// use std::marker::PhantomData;
// use std::time::{Duration, Instant};
//
// use gears::games::{Board, ZobristHistoryBase};
// use gears::general::common::{Res, StaticallyNamedEntity};
// use gears::search::{SearchLimit, SearchResult, TimeControl};
//
// #[derive(Debug)]
// pub struct Human<B: Board> {
//     // output: UIHandle<B>,
//     _todoRemove: PhantomData<B>,
// }
//
// // impl<B: Board> Human<B> {
// //     pub fn new(output: UIHandle<B>) -> Self {
// //         Self { output }
// //     }
// // }
//
// // impl<B: Board> Debug for Human<B> {
// //     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
// //         Debug::fmt(&self.output, f)
// //     }
// // }
//
// impl<B: Board> Default for Human<B> {
//     fn default() -> Self {
//         Human {
//             // output: to_ui_handle(TextUI::default()),
//             _todoRemove: Default::default(),
//         }
//     }
// }
//
// impl<B: Board> StaticallyNamedEntity for Human<B> {
//     fn static_short_name() -> &'static str
//     where
//         Self: Sized,
//     {
//         "human"
//     }
//
//     fn static_long_name() -> &'static str
//     where
//         Self: Sized,
//     {
//         "Human Player"
//     }
//
//     fn static_description() -> &'static str
//     where
//         Self: Sized,
//     {
//         "A human, using the UI to play this game."
//     }
// }
//
// impl<B: Board> SearcherBase for Human<B> {
//     fn time_up(&self, tc: TimeControl, hard_limit: Duration, start_time: Instant) -> bool {
//         let elapsed = start_time.elapsed();
//         elapsed > tc.remaining.min(hard_limit)
//     }
// }
//
// impl<B: Board> Searcher<B> for Human<B> {
//     fn can_use_multiple_threads() -> bool
//     where
//         Self: Sized,
//     {
//         false
//     }
//
//     fn search(&mut self, pos: B, _: SearchLimit, _: ZobristHistoryBase) -> Res<SearchResult<B>> {
//         // let mut handle = self.output.borrow_mut();
//         // Ok(SearchResult::move_only(handle.get_move(&pos)?))
//         todo!()
//     }
// }
