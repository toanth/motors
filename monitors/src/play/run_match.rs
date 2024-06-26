// use std::time::Duration;
//
// use gears::games::Board;
// use gears::games::mnk::MNKBoard;
// use gears::general::common::parse_int_from_stdin;
// use crate::play::AdjudicationReason::*;
// use crate::play::PlayerResult::*;
// use crate::search::{Searcher, SearchLimit, TimeControl};
// use crate::ui::Message::*;
//
// #[derive(Debug, Default)]
// struct BuiltInInfoCallback {}

// impl<B: Board> InfoCallback<B> for BuiltInInfoCallback {
//     fn print_info(&self, info: SearchInfo<B>) {
//         println!(
//             "After {0} milliseconds: Move {1}, score {2}",
//             info.time.as_millis(),
//             info.best_move,
//             info.score.0
//         )
//     }
// }

// pub fn play() {
//     play_mnk(); // the only game that's implemented for now
// }
//
// pub fn play_mnk() {
//     let limit = SearchLimit::tc(TimeControl {
//         remaining: Duration::new(20, 0),
//         increment: Duration::new(0, 200_000_000),
//         moves_to_go: 0,
//     });
//
//     let engines = MNKBoard::list_engines();
//     println!("Please enter the height:");
//     let height = parse_int_from_stdin().unwrap_or(3);
//     println!("Please enter the width:");
//     let width = parse_int_from_stdin().unwrap_or(3);
//     println!("Please enter k:");
//     let k = parse_int_from_stdin().unwrap_or(3);
//     println!("Please enter strength (between 1 and {}):", engines.len());
//     let strength = parse_int_from_stdin().unwrap_or_else(|e| {
//         println!("Error: {e}");
//         3
//     });
//     // let engine = engines
//     //     .get(strength - 1)
//     //     .unwrap_or_else(engines.last().unwrap());
//     // let computer = (engine.1)("");
//     todo!();
//     // println!("Playing against {0}",  computer.engine_info().name);
//     // let output = to_ui_handle(PrettyUI::default());
//     // let mnk_settings = MnkSettings::try_new(Height(height), Width(width), k);
//     // if mnk_settings.is_none() {
//     //     println!("Invalid m,n,k settings, please try again");
//     //     return;
//     // }
//     // let mut the_match = BuiltInMatch::new(
//     //     mnk_settings.unwrap(),
//     //     Player::human(output.clone()),
//     //     computer,
//     //     output.clone(),
//     // );
//     //
//     // let res = the_match.run();
//     // if let Adjudication(x) = res.reason {
//     //     println!("Adjudication: {x}");
//     // }
//     // match res.result {
//     //     GameResult::P1Win => println!("Player 1 won!"),
//     //     GameResult::P2Win => println!("Player 2 won!"),
//     //     GameResult::Draw => println!("The game ended in a draw."),
//     //     GameResult::Aborted => println!("The game was aborted."),
//     // }
// }

// /// A player for the built-in match manager. TODO: Refactor
// #[derive(Debug)]
// pub struct Player<B: Board> {
//     pub searcher: AnySearcher<B>,
//     pub limit: SearchLimit,
//     pub original_limit: SearchLimit,
//     pub retry_on_invalid_move: bool,
//     phantom: PhantomData<B>,
// }
//
// impl<B: Board> Player<B> {
//     pub fn make_move(&mut self, pos: B, history: ZobristHistoryBase) -> SearchResult<B> {
//         // self.searcher.search(
//         //     pos,
//         //     self.limit,
//         //     history,
//         //     Box::new(BuiltInInfoCallback::default()),
//         // )
//         todo!()
//     }
//
//     pub fn update_time(&mut self, time_spent_last_move: Duration) -> MatchStatus {
//         let max_time = self.limit.tc.remaining.max(self.limit.fixed_time);
//         if time_spent_last_move > max_time {
//             return Over(MatchResult {
//                 result: Aborted,
//                 reason: Adjudication(TimeUp),
//             });
//         }
//         self.limit.tc.remaining -= time_spent_last_move;
//         if self.limit.tc.moves_to_go == 1 {
//             self.limit.tc.moves_to_go = self.original_limit.tc.moves_to_go;
//             self.limit.tc.remaining += self.original_limit.tc.remaining;
//         } else if self.limit.tc.moves_to_go != 0 {
//             self.limit.tc.moves_to_go -= 1;
//         }
//         MatchStatus::Ongoing
//     }
//
//     pub fn new_for_searcher<S: Searcher<B>>(searcher: S, limit: SearchLimit) -> Self {
//         Self::new(Box::new(searcher), limit)
//     }
//
//     pub fn new(searcher: AnySearcher<B>, limit: SearchLimit) -> Self {
//         Self {
//             searcher,
//             limit,
//             original_limit: limit,
//             retry_on_invalid_move: false,
//             phantom: Default::default(),
//         }
//     }
//
//     pub fn human(output: UIHandle<B>) -> Self {
//         Player {
//             searcher: Box::new(Human::new(output)),
//             limit: SearchLimit::infinite(),
//             original_limit: SearchLimit::infinite(),
//             retry_on_invalid_move: true,
//             phantom: Default::default(),
//         }
//     }
// }
//
// struct MoveRes<B: Board> {
//     mov: B::Move,
//     board: B,
// }
//
// fn make_move<B: Board>(
//     pos: B,
//     history: &mut ZobristRepetition3Fold,
//     player: &mut Player<B>,
//     graphics: UIHandle<B>,
//     move_hist: &mut Vec<B::Move>,
// ) -> Result<MoveRes<B>, GameOver> {
//     if let Some(result) = pos.game_result_slow() {
//         return Err(GameOver {
//             result,
//             reason: GameOverReason::Normal,
//         });
//     }
//     history.push(&pos);
//
//     graphics.borrow_mut().display_message_simple(
//         Info,
//         format!("Player: {0}", player.searcher.name()).as_str(),
//     );
//
//     let new_pos;
//     let mut response;
//
//     loop {
//         let start_time = Instant::now();
//         response = player.make_move(pos, history.0.clone());
//         let duration = start_time.elapsed();
//
//         if let Over(res) = player.update_time(duration) {
//             assert_eq!(res.result, Aborted);
//             return Err(GameOver {
//                 result: Lose,
//                 reason: res.reason,
//             });
//         }
//
//         let mov = response.chosen_move;
//         if pos.is_move_legal(mov) {
//             new_pos = pos.make_move(mov).unwrap();
//             break;
//         }
//
//         if player.retry_on_invalid_move {
//             graphics
//                 .borrow_mut()
//                 .display_message_simple(Warning, "Invalid move. Try again:");
//             continue;
//         }
//         move_hist.push(mov);
//         return Err(GameOver {
//             result: Lose,
//             reason: Adjudication(InvalidMove),
//         });
//     }
//
//     graphics.borrow_mut().display_message_simple(
//         Info,
//         format!(
//             "Eval: {0}",
//             response
//                 .score
//                 .map_or("no score".to_string(), |s| s.0.to_string())
//         )
//         .as_str(),
//     );
//
//     if let Some(res) = new_pos.game_result_slow() {
//         Err(GameOver {
//             result: res,
//             reason: GameOverReason::Normal,
//         })
//     } else {
//         Ok(MoveRes {
//             mov: response.chosen_move,
//             board: new_pos,
//         })
//     }
// }
//
// fn player_res_to_match_res(game_over: GameOver, is_p1: bool) -> MatchResult {
//     let result = match game_over.result {
//         Draw => GameResult::Draw,
//         res => {
//             if is_p1 == (res == Win) {
//                 GameResult::P1Win
//             } else {
//                 GameResult::P2Win
//             }
//         }
//     };
//     MatchResult {
//         result,
//         reason: game_over.reason,
//     }
// }
//
// // TODO: Rework the built-in match manager, use UGI exclusively to communicate with players to support external engines
// #[derive(Debug)]
// pub struct BuiltInMatch<B: Board> {
//     board: B,
//     move_hist: Vec<B::Move>,
//     status: MatchStatus,
//     p1: Player<B>,
//     p2: Player<B>,
//     graphics: GraphicsHandle<B>,
//     next_match: Option<AnyMatch>,
// }

// impl<B: Board> AbstractMatchManager for BuiltInMatch<B> {
//     fn active_player(&self) -> Option<Color> {
//         if let MatchStatus::Ongoing = self.status {
//             Some(self.board.active_player())
//         } else {
//             None
//         }
//     }
//
//     fn match_status(&self) -> MatchStatus {
//         self.status
//     }
//
//     /// Runs the entire game in this function.
//     /// TODO: Another implementation would be to run this asynchronously, but I don't want to deal with multithreading right now
//     fn run(&mut self) -> MatchResult {
//         self.graphics.borrow_mut().show(self);
//         let mut history = ZobristRepetition3Fold::default();
//         loop {
//             let res = make_move(
//                 self.board,
//                 &mut history,
//                 &mut self.p1,
//                 self.graphics.clone(),
//                 &mut self.move_hist,
//             );
//             if res.is_err() {
//                 return player_res_to_match_res(res.err().unwrap(), true);
//             }
//             self.make_move(res.unwrap());
//
//             let res = make_move(
//                 self.board,
//                 &mut history,
//                 &mut self.p2,
//                 self.graphics.clone(),
//                 &mut self.move_hist,
//             );
//             if res.is_err() {
//                 return player_res_to_match_res(res.err().unwrap(), false);
//             }
//             self.make_move(res.unwrap());
//         }
//     }
//
//     fn abort(&mut self) -> Res<MatchStatus> {
//         self.status = MatchStatus::aborted();
//         Ok(self.status)
//     }
//
//     fn game_name(&self) -> String {
//         B::game_name()
//     }
//
//     fn next_match(&mut self) -> Option<AnyMatch> {
//         self.next_match.take()
//     }
//
//     fn set_next_match(&mut self, next: Option<AnyMatch>) {
//         self.next_match = next;
//     }
//
//     fn debug_mode(&self) -> bool {
//         false
//     }
// }
//
// impl<B: Board> MatchManager<B> for BuiltInMatch<B> {
//     fn board(&self) -> B {
//         self.board
//     }
//
//     fn graphics(&self) -> GraphicsHandle<B> {
//         self.graphics.clone()
//     }
//
//     fn initial_pos(&self) -> B {
//         B::startpos(self.board.settings())
//     }
//
//     fn move_history(&self) -> &[B::Move] {
//         self.move_hist.as_slice()
//     }
//
//     fn set_graphics(&mut self, graphics: GraphicsHandle<B>) {
//         self.graphics = graphics;
//         self.graphics.borrow_mut().show(self);
//     }
//
//     // fn set_engine(&mut self, idx: usize, engine: AnyEngine<B>) {
//     //     // match idx {
//     //     //     0 => self.p1.searcher = engine,
//     //     //     1 => self.p2.searcher = engine,
//     //     //     _ => panic!("Player number has to be 0 or 1"),
//     //     // }
//     //     todo!()
//     // }
//
//     // fn searcher(&self, idx: usize) -> &dyn Searcher<B> {
//     //     match idx {
//     //         0 => self.p1.searcher.as_ref(),
//     //         1 => self.p2.searcher.as_ref(),
//     //         _ => panic!("Can only get searcher 0 or 1"),
//     //     }
//     // }
//
//     fn set_board(&mut self, board: B) {
//         self.board = board;
//     }
// }
//
// impl<B: Board> CreatableMatchManager for BuiltInMatch<B> {
//     type ForGame<C: Board> = BuiltInMatch<C>;
//
//     fn with_engine_and_ui<C: Board>(engine: AnyEngine<C>, output: UIHandle<C>) -> Self::ForGame<C> {
//         // let player_1 = Player::human(output.clone());
//         // let limit = SearchLimit::per_move(Duration::from_millis(1_000));
//         // let player_2 = Player::new(engine, limit);
//         // <Self::ForGame<C>>::new(C::Settings::default(), player_1, player_2, output)
//         todo!()
//     }
// }
//
// impl<B: Board> Default for BuiltInMatch<B> {
//     fn default() -> Self {
//         Self::with_engine_and_ui(default_engine(), to_ui_handle(TextUI::default()))
//     }
// }
//
// impl<B: Board> BuiltInMatch<B> {
//     pub fn new(
//         game_settings: B::Settings,
//         player_1: Player<B>,
//         player_2: Player<B>,
//         graphics: GraphicsHandle<B>,
//     ) -> Self {
//         Self::from_position(B::startpos(game_settings), player_1, player_2, graphics)
//     }
//
//     pub fn from_position(
//         pos: B,
//         p1: Player<B>,
//         p2: Player<B>,
//         graphics: GraphicsHandle<B>,
//     ) -> Self {
//         BuiltInMatch {
//             board: pos,
//             move_hist: vec![],
//             status: MatchStatus::NotStarted,
//             p1,
//             p2,
//             graphics,
//             next_match: None,
//         }
//     }
//
//     fn make_move(&mut self, move_res: MoveRes<B>) {
//         self.board = move_res.board;
//         self.move_hist.push(move_res.mov);
//         self.graphics.borrow_mut().show(self);
//     }
// }
