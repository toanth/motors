// /*
//  *  Gears, a collection of board games.
//  *  Copyright (C) 2024 ToTheAnd
//  *
//  *  Gears is free software: you can redistribute it and/or modify
//  *  it under the terms of the GNU General Public License as published by
//  *  the Free Software Foundation, either version 3 of the License, or
//  *  (at your option) any later version.
//  *
//  *  Gears is distributed in the hope that it will be useful,
//  *  but WITHOUT ANY WARRANTY; without even the implied warranty of
//  *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
//  *  GNU General Public License for more details.
//  *
//  *  You should have received a copy of the GNU General Public License
//  *  along with Gears. If not, see <https://www.gnu.org/licenses/>.
//  */
// use crate::games::chess::pieces::NUM_COLORS;
// use crate::games::{
//     AbstractPieceType, BoardHistory, Color, ColoredPiece, ColoredPieceType, Coordinates,
//     GenericPiece, PieceType, Settings, ZobristHash,
// };
// use crate::general::bitboards::ExtendedRawBitboard;
// use crate::general::board::{
//     Board, BoardSize, ColPieceType, NameToPos, PieceTypeOf, SelfChecks, Strictness, UnverifiedBoard,
// };
// use crate::general::common::{EntityList, NamedEntity, Res, StaticallyNamedEntity, Tokens};
// use crate::general::move_list::{EagerNonAllocMoveList, MoveList};
// use crate::general::moves::{
//     ExtendedFormat, ExtendedFormatter, Legality, Move, MoveFlags, UntrustedMove,
// };
// use crate::general::squares::{GridCoordinates, SquareColor};
// use crate::output::text_output::{BoardFormatter, PieceToChar};
// use crate::search::Depth;
// use crate::{MatchResult, PlayerResult};
// use arbitrary::{Arbitrary, Unstructured};
// use arrayvec::ArrayVec;
// use derive_more::{IntoIterator, Neg, Not};
// use rand::Rng;
// use std::cell::{OnceCell, RefCell};
// use std::fmt;
// use std::fmt::{Debug, Display, Formatter};
// use std::rc::Rc;
// use std::sync::Arc;
// use strum_macros::EnumIter;
//
// type FairyBitboard = ExtendedRawBitboard; // TODO: Better bitboard type
//
// /// There can never be more than 32 piece types in a given game
// /// (For chess, the number would be 6; for ataxx, 1).
// /// Note that some effects can also be represented by one of these bitboards.
// const MAX_NUM_PIECE_TYPES: usize = 16;
//
// pub type FairySquare = GridCoordinates;
//
// impl LeapingMoves {
//     pub fn new(dx: isize, dy: isize) -> Self {
//         Self { dx, dy }
//     }
//
//     pub fn gen(self, square: FairySquare) -> FairyBitboard {}
// }
//
// #[derive(Debug, Clone)]
// pub enum MoveTypes {
//     Leaping(Box<dyn Fn(FairySquare) -> FairyBitboard>),
//     Sliding(Box<dyn Fn(FairySquare, FairyBitboard) -> FairyBitboard>),
//     Custom(Box<dyn Fn(&FairyBitboard) -> FairyBitboard>),
// }
//
// #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
// struct PieceId(usize);
//
// impl PieceId {
//     pub fn piece(&self) -> &Piece {
//         &FAIRY_RULES.borrow().pieces[self.0]
//     }
// }
//
// impl Display for PieceId {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", self.to_ascii_char())
//     }
// }
//
// impl AbstractPieceType for PieceId {
//     fn empty() -> Self {
//         Self(0)
//     }
//
//     fn to_ascii_char(self) -> char {
//         self.piece().uncolored_ascii_symbol
//     }
//
//     fn to_utf8_char(self) -> char {
//         self.piece().uncolored_ascii_symbol
//     }
//
//     fn from_ascii_char(c: char) -> Option<Self> {
//         FAIRY_RULES.borrow().pieces.find(|p| p.ascii_symbol == c)
//     }
//
//     fn from_utf8_char(c: char) -> Option<Self> {
//         FAIRY_RULES.borrow().pieces.find(|p| p.ascii_symbol == c)
//     }
//
//     fn to_uncolored_idx(self) -> usize {
//         self.0
//     }
// }
//
// impl PieceType<FairyBoard> for PieceId {
//     type Colored = ColoredPieceId;
//
//     fn from_idx(idx: usize) -> Self {
//         Self(idx)
//     }
// }
//
// #[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
// pub struct ColoredPieceId {
//     id: usize,
//     color: Option<FairyColor>,
// }
//
// impl ColoredPieceId {
//     pub fn piece(&self) -> &Piece {
//         &FAIRY_RULES.borrow().piecs[self.id]
//     }
// }
//
// impl Display for ColoredPieceId {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         write!(
//             f,
//             "{0} {1}",
//             FAIRY_RULES.borrow().colors[self.color as usize],
//             FAIRY_RULES.borrow().pieces[self.id]
//         )
//     }
// }
//
// impl AbstractPieceType for ColoredPieceId {
//     fn empty() -> Self {
//         Self { id: 0, color: None }
//     }
//
//     fn to_ascii_char(self) -> char {
//         if let Some(color) = self.color {
//             self.piece().player_symbol_ascii[color as usize]
//         } else {
//             self.piece().uncolored_ascii_symbol
//         }
//     }
//
//     fn to_utf8_char(self) -> char {
//         if let Some(color) = self.color {
//             self.piece().player_symbol_unicode[color as usize]
//         } else {
//             self.piece().uncolored_unicode_symbol
//         }
//     }
//
//     fn from_ascii_char(c: char) -> Option<Self> {
//         todo!()
//     }
//
//     fn from_utf8_char(c: char) -> Option<Self> {
//         todo!()
//     }
//
//     fn to_uncolored_idx(self) -> usize {
//         self.id
//     }
// }
//
// impl ColoredPieceType<FairyBoard> for ColoredPieceId {
//     type Uncolored = PieceId;
//
//     fn color(self) -> Option<FairyColor> {
//         self.color
//     }
//
//     fn uncolor(self) -> Self::Uncolored {
//         PieceId(self.id)
//     }
//
//     fn to_colored_idx(self) -> usize {
//         self.id
//     }
//
//     fn new(color: FairyColor, uncolored: Self::Uncolored) -> Self {
//         Self {
//             id: uncolored.0,
//             color: Some(color),
//         }
//     }
// }
//
// /// This struct defines the rules for a single piece.
// #[derive(Debug, Clone)]
// struct Piece {
//     name: String,
//     uncolored_ascii_symbol: char,
//     uncolored_unicode_symbol: char,
//     player_symbol_ascii: [char; 2],
//     player_symbol_unicode: [char; 2],
//     id: PieceId,
//     moves: MoveTypes,
//     filter: Box<dyn Fn(&FairyBoard) -> FairyBitboard>,
//     effects: Box<dyn Fn(&mut FairyBoard)>,
//     royal: bool,
// }
//
// #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
// struct FairyFlags(usize);
//
// impl MoveFlags for FairyFlags {}
//
// #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
// struct FairyMove {
//     from: FairySquare,
//     to: FairySquare,
//     piece: PieceId,
//     flags: FairyFlags,
// }
//
// impl Display for FairyMove {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         self.format_compact(f)
//     }
// }
//
// impl Move<FairyBoard> for FairyMove {
//     type Flags = FairyFlags;
//     type Underlying = usize; // TODO: u64
//
//     fn is_null(self) -> bool {
//         todo!()
//     }
//
//     fn legality() -> Legality {
//         FAIRY_RULES.borrow().legality
//     }
//
//     fn src_square(self) -> FairySquare {
//         self.from
//     }
//
//     fn dest_square(self) -> FairySquare {
//         self.to
//     }
//
//     fn flags(self) -> Self::Flags {
//         self.flags
//     }
//
//     fn is_tactical(self, board: &FairyBoard) -> bool {
//         todo!()
//     }
//
//     fn format_compact(self, f: &mut Formatter<'_>) -> fmt::Result {
//         todo!()
//     }
//
//     fn format_extended(
//         self,
//         f: &mut Formatter<'_>,
//         _board: &FairyBoard,
//         _format: ExtendedFormat,
//     ) -> fmt::Result {
//         todo!()
//     }
//
//     fn extended_formatter(
//         self,
//         pos: FairyBoard,
//         format: ExtendedFormat,
//     ) -> ExtendedFormatter<FairyBoard> {
//         todo!()
//     }
//
//     fn to_extended_text(self, board: &FairyBoard, format: ExtendedFormat) -> String {
//         todo!()
//     }
//
//     fn parse_compact_text<'a>(s: &'a str, board: &FairyBoard) -> Res<(&'a str, FairyMove)> {
//         todo!()
//     }
//
//     fn from_compact_text(s: &str, board: &FairyBoard) -> Res<FairyMove> {
//         todo!()
//     }
//
//     fn parse_extended_text<'a>(s: &'a str, board: &FairyBoard) -> Res<(&'a str, FairyMove)> {
//         todo!()
//     }
//
//     fn from_extended_text(s: &str, board: &FairyBoard) -> Res<FairyMove> {
//         todo!()
//     }
//
//     fn from_text(s: &str, board: &FairyBoard) -> Res<FairyMove> {
//         todo!()
//     }
//
//     // TODO: Doesn't really make sense, at least use u64
//     fn from_usize_unchecked(val: usize) -> UntrustedMove<FairyBoard> {
//         todo!()
//     }
//
//     fn to_underlying(self) -> Self::Underlying {
//         todo!()
//     }
// }
//
// /// Maximum number of pseudolegal moves in a position
// const MAX_MOVES: usize = 1024;
//
// type FairyMoveList = EagerNonAllocMoveList<FairyBoard, MAX_MOVES>;
//
// #[derive(Debug, Copy, Clone, Eq, PartialEq)]
// enum GameLoss {
//     Checkmate,
//     NoRoyals,
//     NoPieces,
//     NoMoves,
// }
//
// #[derive(Debug, Copy, Clone, Eq, PartialEq)]
// enum Draw {
//     NoMoves,
//     Counter,
//     Repetition(usize),
// }
//
// #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, derive_more::Display, Not, Hash, EnumIter)]
// enum FairyColor {
//     #[default]
//     First,
//     Second,
// }
//
// impl Color for FairyColor {
//     fn other(self) -> Self {
//         match self {
//             FairyColor::First => FairyColor::Second,
//             FairyColor::Second => FairyColor::First,
//         }
//     }
//
//     fn ascii_color_char(self) -> char {
//         todo!()
//     }
// }
//
// /// This struct defined the rules for each piece.
// /// Since the rules don't change during a game, but are expensive to copy and the board uses copy-make,
// /// the board contains an `Rc` to the rules
// #[derive(Debug, Eq, PartialEq)]
// struct Rules {
//     pieces: ArrayVec<Piece, MAX_NUM_PIECE_TYPES>,
//     counter: usize,
//     move_number: usize,
//     game_loss: GameLoss,
//     draw: Draw,
//     startpos_fen: String,
//     legality: Legality,
// }
//
// #[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
// struct RulesRef {}
//
// impl Settings for RulesRef {
//     fn text(&self) -> Option<String> {
//         todo!()
//     }
// }
//
// /// The least bad option to implement rules.
// /// In the future, it might make sense to explore an implementation where each piece, move, etc, contains
// /// a reference / Rc to the rules
// thread_local! {
//     pub static FAIRY_RULES: RefCell<Rules> = RefCell::default();
// }
//
// /// A FairyBoard is a retangular board for a chess-like variant.
// #[derive(Debug, Clone, Eq, PartialEq, Arbitrary)]
// pub struct UnverifiedFairyBoard {
//     piece_bitboards: ArrayVec<FairyBitboard, MAX_NUM_PIECE_TYPES>,
//     color_bitboards: [FairyBitboard; NUM_COLORS],
//     ply_since_start: usize,
//     // like the 50mr counter in chess TODO: Maybe make it count down?
//     draw_counter: usize,
//     active: FairyColor,
// }
//
// impl From<FairyBoard> for UnverifiedFairyBoard {
//     fn from(value: FairyBoard) -> Self {
//         value.0
//     }
// }
//
// impl UnverifiedBoard<FairyBoard> for UnverifiedFairyBoard {
//     fn verify_with_level(self, level: SelfChecks, strictness: Strictness) -> Res<FairyBoard> {
//         todo!()
//     }
//
//     fn size(&self) -> BoardSize<FairyBoard> {
//         todo!()
//     }
//
//     fn place_piece(self, piece: <FairyBoard as Board>::Piece) -> Res<Self> {
//         todo!()
//     }
//
//     fn place_piece_unchecked(self, coords: FairySquare, piece: ColPieceType<FairyBoard>) -> Self {
//         todo!()
//     }
//
//     fn remove_piece(self, coords: FairySquare) -> Res<Self> {
//         todo!()
//     }
//
//     fn remove_piece_unchecked(self, coords: FairySquare) -> Self {
//         todo!()
//     }
//
//     fn piece_on(&self, coords: FairySquare) -> Res<<FairyBoard as Board>::Piece> {
//         todo!()
//     }
//
//     fn set_active_player(mut self, player: FairyColor) -> Self {
//         self.active = player;
//         self
//     }
//
//     fn set_ply_since_start(mut self, ply: usize) -> Res<Self> {
//         self.ply_since_start = ply;
//         Ok(self)
//     }
// }
//
// #[derive(Debug, Clone, Arbitrary)]
// pub struct FairyBoard(UnverifiedFairyBoard);
//
// impl Eq for FairyBoard {}
//
// impl PartialEq<Self> for FairyBoard {
//     fn eq(&self, other: &Self) -> bool {
//         Rc::ptr_eq(&self.0.rules, &other.0.rules)
//             && self.0.color_bitboards == other.0.color_bitboards
//             && self.0.piece_bitboards == self.0.piece_bitboards
//     }
// }
//
// impl Default for FairyBoard {
//     fn default() -> Self {
//         todo!()
//     }
// }
//
// impl Display for FairyBoard {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", self.as_fen())
//     }
// }
//
// impl StaticallyNamedEntity for FairyBoard {
//     fn static_short_name() -> impl Display
//     where
//         Self: Sized,
//     {
//         "fairy"
//     }
//
//     fn static_long_name() -> String
//     where
//         Self: Sized,
//     {
//         "Fairy Chess Variant".to_string()
//     }
//
//     fn static_description() -> String
//     where
//         Self: Sized,
//     {
//         "One of many variants of chess".to_string()
//     }
// }
//
// impl Board for FairyBoard {
//     type EmptyRes = Self::Unverified;
//     type Settings = RulesRef;
//     type Coordinates = FairySquare;
//     type Color = FairyColor;
//     type Piece = GenericPiece<FairyBoard, ColoredPieceId>;
//     type Move = Move;
//     type MoveList = FairyMoveList;
//     type Unverified = UnverifiedFairyBoard;
//
//     fn empty_for_settings(settings: Self::Settings) -> Self::EmptyRes {
//         todo!()
//     }
//
//     fn startpos_for_settings(settings: Self::Settings) -> Self {
//         todo!()
//     }
//
//     fn startpos_with_current_settings(self) -> Self {
//         todo!()
//     }
//
//     fn startpos() -> Self {
//         todo!()
//     }
//
//     fn from_name(name: &str) -> Res<Self> {
//         todo!()
//     }
//
//     fn name_to_pos_map() -> EntityList<NameToPos<Self>> {
//         todo!()
//     }
//
//     fn bench_positions() -> Vec<Self> {
//         todo!()
//     }
//
//     fn settings(&self) -> Self::Settings {
//         todo!()
//     }
//
//     fn active_player(&self) -> Self::Color {
//         todo!()
//     }
//
//     fn fullmove_ctr_0_based(&self) -> usize {
//         todo!()
//     }
//
//     fn fullmove_ctr_1_based(&self) -> usize {
//         todo!()
//     }
//
//     fn halfmove_ctr_since_start(&self) -> usize {
//         todo!()
//     }
//
//     fn halfmove_repetition_clock(&self) -> usize {
//         todo!()
//     }
//
//     fn size(&self) -> <Self::Coordinates as Coordinates>::Size {
//         todo!()
//     }
//
//     fn num_squares(&self) -> usize {
//         todo!()
//     }
//
//     fn is_empty(&self, coords: Self::Coordinates) -> bool {
//         todo!()
//     }
//
//     fn is_piece_on(&self, coords: Self::Coordinates, piece: ColPieceType<Self>) -> bool {
//         todo!()
//     }
//
//     fn colored_piece_on(&self, coords: Self::Coordinates) -> Self::Piece {
//         todo!()
//     }
//
//     fn piece_type_on(&self, coords: Self::Coordinates) -> PieceTypeOf<Self> {
//         todo!()
//     }
//
//     fn default_perft_depth(&self) -> Depth {
//         todo!()
//     }
//
//     fn max_perft_depth() -> Depth {
//         todo!()
//     }
//
//     fn pseudolegal_moves(&self) -> Self::MoveList {
//         todo!()
//     }
//
//     fn gen_pseudolegal<T: MoveList<Self>>(&self, moves: &mut T) {
//         todo!()
//     }
//
//     fn gen_tactical_pseudolegal<T: MoveList<Self>>(&self, moves: &mut T) {
//         todo!()
//     }
//
//     fn tactical_pseudolegal(&self) -> Self::MoveList {
//         todo!()
//     }
//
//     fn legal_moves_slow(&self) -> Self::MoveList {
//         todo!()
//     }
//
//     fn random_legal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
//         todo!()
//     }
//
//     fn random_pseudolegal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
//         todo!()
//     }
//
//     fn make_move(self, mov: Self::Move) -> Option<Self> {
//         todo!()
//     }
//
//     fn make_nullmove(self) -> Option<Self> {
//         todo!()
//     }
//
//     fn is_move_pseudolegal(&self, mov: Self::Move) -> bool {
//         todo!()
//     }
//
//     fn is_move_legal(&self, mov: Self::Move) -> bool {
//         todo!()
//     }
//
//     fn is_pseudolegal_move_legal(&self, mov: Self::Move) -> bool {
//         todo!()
//     }
//
//     fn player_result_no_movegen<H: BoardHistory<Self>>(&self, history: &H) -> Option<PlayerResult> {
//         todo!()
//     }
//
//     fn player_result_slow<H: BoardHistory<Self>>(&self, history: &H) -> Option<PlayerResult> {
//         todo!()
//     }
//
//     fn match_result_slow<H: BoardHistory<Self>>(&self, history: &H) -> Option<MatchResult> {
//         todo!()
//     }
//
//     fn no_moves_result(&self) -> PlayerResult {
//         todo!()
//     }
//
//     fn is_game_lost_slow(&self) -> bool {
//         todo!()
//     }
//
//     fn is_game_won_after_slow(&self, mov: Self::Move) -> bool {
//         todo!()
//     }
//
//     fn can_reasonably_win(&self, player: Self::Color) -> bool {
//         todo!()
//     }
//
//     fn zobrist_hash(&self) -> ZobristHash {
//         todo!()
//     }
//
//     fn as_fen(&self) -> String {
//         todo!()
//     }
//
//     fn from_fen(string: &str, strictness: Strictness) -> Res<Self> {
//         todo!()
//     }
//
//     fn read_fen_and_advance_input(input: &mut Tokens, strictness: Strictness) -> Res<Self> {
//         todo!()
//     }
//
//     fn should_flip_visually() -> bool {
//         todo!()
//     }
//
//     fn as_ascii_diagram(&self, flip: bool) -> String {
//         todo!()
//     }
//
//     fn as_unicode_diagram(&self, flip: bool) -> String {
//         todo!()
//     }
//
//     fn display_pretty(&self, formatter: &mut dyn BoardFormatter<Self>) -> String {
//         todo!()
//     }
//
//     fn pretty_formatter(
//         &self,
//         piece: Option<PieceToChar>,
//         last_move: Option<Self::Move>,
//     ) -> Box<dyn BoardFormatter<Self>> {
//         todo!()
//     }
//
//     fn background_color(&self, coords: Self::Coordinates) -> SquareColor {
//         todo!()
//     }
//
//     fn debug_verify_invariants(self, strictness: Strictness) -> Res<Self> {
//         todo!()
//     }
//
//     fn place_piece(self, piece: Self::Piece) -> Res<Self::Unverified> {
//         todo!()
//     }
//
//     fn remove_piece(self, square: Self::Coordinates) -> Res<Self::Unverified> {
//         todo!()
//     }
//
//     fn set_active_player(self, new_active: Self::Color) -> Self::Unverified {
//         todo!()
//     }
//
//     fn set_ply_since_start(self, ply: usize) -> Res<Self::Unverified> {
//         todo!()
//     }
// }
