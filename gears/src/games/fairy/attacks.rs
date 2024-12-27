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
// use crate::games::chess::castling::CastleRight;
// use crate::games::fairy::attacks::AttackBitboardFilter::NotUs;
// use crate::games::fairy::attacks::AttackKind::Normal;
// use crate::games::fairy::attacks::AttackTypes::{Leaping, Sliding};
// use crate::games::fairy::attacks::GenAttacksCondition::Always;
// use crate::games::fairy::attacks::MoveEffect::{
//     ClearSquares, PlaceSinglePiece, RemoveSinglePiece, ResetDrawCtr, SetColorTo,
// };
// use crate::games::fairy::{
//     rules, ColoredPieceId, FairyBitboard, FairyBoard, FairyColor, FairyMove, FairyPiece, FairySize,
//     FairySquare, PieceId, RawFairyBitboard, UnverifiedFairyBoard,
// };
// use crate::games::{Color, ColoredPiece, ColoredPieceType, DimT, Size};
// use crate::general::bitboards::{precompute_single_leaper_attacks, Bitboard, RawBitboard};
// use crate::general::board::SelfChecks::Verify;
// use crate::general::board::Strictness::Strict;
// use crate::general::board::{BitboardBoard, Board, BoardHelpers, UnverifiedBoard};
// use crate::general::move_list::MoveList;
// use crate::general::squares::{CompactSquare, RectangularCoordinates, RectangularSize};
// use crate::shift_left;
// use arbitrary::Arbitrary;
// use arrayvec::ArrayVec;
// use std::any::Any;
// use std::iter::once;
// use strum_macros::FromRepr;
//
// ///
// /// The general organization of movegen is that of a pipeline, where a stage communicates with the next through enums
// /// that are then interpreted by the next stage, and usually don't contain a lot of information.
// /// The general steps for generating and playing a move are as follows:
// /// - A piece of the current player is selected.
// /// - An attack kind (enum) is selected for this piece (e.g. castling, or normal king move)
// /// - A condition is evaluated to decide if attacks should be generated for this attack kind
// /// - A blocker configuration bitboard is generated based on the attack kind and position
// /// - A bitboard of attacks is generated for this attack kind and blocker bitboard, often precomputed or done through hyperbola quintessence
// /// - A filter bitboard is generated based on the attack kind and the position
// /// - Both bitboards are combined through a bitwise and
// /// - A struct is passed on that contains the bitboard and attack kind
// /// - This struct is turned into a sublist of moves using the current position
// /// - The entire movelist is returned and a move can be selected and played
// /// - The move is filtered. This can do more expensive checks than during movegen
// /// - A MoveKind enum is generated based on the attack kind enum. It can include additional information like captures
// /// - The move is played, bitboards are updated
// /// - The move effects are executed based on the move kind and new position
// /// - The move is filtered again
// /// - The new position is returned
//
// #[derive(Debug, Copy, Clone)]
// pub enum SliderDirections {
//     Horizontal,
//     Vertical,
//     Diagonal,
//     AntiDiagonal,
//     Rook,
//     Bishop,
//     Queen,
// }
//
// // not const, which allows using ranges and for loops
// pub fn leaper_attack_range<Iter1: Iterator<Item = isize>, Iter2: Iterator<Item = isize> + Clone>(
//     mut horizontal_range: Iter1,
//     mut vertical_range: Iter2,
//     square: FairySquare,
//     size: FairySize,
// ) -> RawFairyBitboard {
//     let mut res = RawFairyBitboard::default();
//     let width = size.width().val() as isize;
//     for dx in horizontal_range {
//         for dy in vertical_range.clone() {
//             let shift = dx + dy * width;
//             let bb = FairyBitboard::single_piece_for(square, size);
//             if square.file() as isize >= -dx && square.file() as isize + dx < width {
//                 res |= shift_left!(bb.raw(), shift);
//             }
//         }
//     }
//     res
// }
//
// pub(super) struct LeapingBitboards(Box<[RawFairyBitboard]>);
//
// impl LeapingBitboards {
//     pub(super) fn fixed(n: usize, m: usize, size: FairySize) -> Self {
//         let mut res = vec![RawFairyBitboard::default(); size.num_squares()].into_boxed_slice();
//         for idx in 0..size.num_squares() {
//             let bb = precompute_single_leaper_attacks(idx, n, m, size.width.val());
//             res[idx] = bb as u128; // TODO: Make this work for u128 too
//         }
//         LeapingBitboards(res)
//     }
//
//     pub(super) fn range<
//         Iter1: Iterator<Item = isize> + Clone,
//         Iter2: Iterator<Item = isize> + Clone,
//     >(
//         horizontal_range: Iter1,
//         vertical_range: Iter2,
//         size: FairySize,
//     ) -> Self {
//         let mut res = vec![RawFairyBitboard::default(); size.num_squares()].into_boxed_slice();
//         for idx in 0..size.num_squares() {
//             let sq = size.idx_to_coordinates(idx as DimT);
//             let bb =
//                 leaper_attack_range(horizontal_range.clone(), vertical_range.clone(), sq, size);
//             res[idx] = bb;
//         }
//         LeapingBitboards(res)
//     }
//
//     pub(super) fn combine(mut self, other: LeapingBitboards) -> Self {
//         assert_eq!(self.0.len(), other.0.len());
//         self.0
//             .iter_mut()
//             .zip(other.0.iter())
//             .for_each(|(a, b)| *a |= b);
//         self
//     }
// }
//
// #[must_use]
// pub enum AttackTypes {
//     Leaping(LeapingBitboards),
//     Sliding(SliderDirections),
//     HardCoded {
//         source: FairySquare,
//         target: FairySquare,
//     },
// }
//
// impl AttackTypes {
//     pub fn leaping(n: usize, m: usize, size: FairySize) -> Self {
//         Leaping(LeapingBitboards::fixed(n, m, size))
//     }
// }
//
// impl GenPieceAttackKind {
//     pub fn attacks_for_filter(&self, piece: FairyPiece, filter: FairyBitboard) -> PieceAttackBB {
//         let piece_id = piece.symbol;
//         let piece = piece.coordinates;
//         let size = filter.size();
//         let res = match &self.typ {
//             Leaping(precomputed) => precomputed.0[size.internal_key(piece)],
//             Sliding(sliding) => {
//                 let res = match sliding {
//                     SliderDirections::Horizontal => {
//                         FairyBitboard::horizontal_attacks(piece, filter)
//                     }
//                     SliderDirections::Vertical => FairyBitboard::vertical_attacks(piece, filter),
//                     SliderDirections::Diagonal => FairyBitboard::diagonal_attacks(piece, filter),
//                     SliderDirections::AntiDiagonal => {
//                         FairyBitboard::anti_diagonal_attacks(piece, filter)
//                     }
//                     SliderDirections::Rook => FairyBitboard::rook_attacks(piece, filter),
//                     SliderDirections::Bishop => FairyBitboard::bishop_attacks(piece, filter),
//                     SliderDirections::Queen => FairyBitboard::queen_attacks(piece, filter),
//                 };
//                 res.raw()
//             }
//             &AttackTypes::HardCoded { source, target } => {
//                 if source == piece {
//                     FairyBitboard::single_piece_for(target, size).raw()
//                 } else {
//                     RawFairyBitboard::default()
//                 }
//             }
//         };
//         PieceAttackBB {
//             bb: res & filter.raw(),
//             kind: self.kind,
//             piece: piece_id,
//         }
//     }
//
//     pub fn attacks(&self, piece: FairyPiece, pos: &FairyBoard) -> PieceAttackBB {
//         let filter = FairyBitboard::new(
//             self.bitboard_filter.bb(piece.color().unwrap(), pos),
//             pos.size(),
//         );
//         self.attacks_for_filter(piece, filter)
//     }
// }
//
// #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
// #[must_use]
// pub enum AttackKind {
//     #[default]
//     Normal,
//     Castle,
// }
//
// /// Attacks are about bitboards for performance reasons, but there are also moves that aren't fully represented with bitboards.
// /// This struct is only about attacks, so there are move types that are generated separately
// #[must_use]
// pub struct GenPieceAttackKind {
//     // first, the condition is checked
//     pub condition: GenAttacksCondition,
//     // then, the bitboard of attacks is generated
//     pub typ: AttackTypes,
//     // it is then filtered (e.g., that's how pawn captures of opponent pieces and ep squares are done)
//     pub bitboard_filter: AttackBitboardFilter,
//     // this is annotated with the move kind (e.g. castling)
//     pub kind: AttackKind,
// }
//
// impl GenPieceAttackKind {
//     pub fn simple(typ: AttackTypes) -> Self {
//         Self {
//             typ,
//             condition: Always,
//             bitboard_filter: NotUs,
//             kind: Normal,
//         }
//     }
// }
//
// #[must_use]
// pub struct PieceAttackBB {
//     pub bb: RawFairyBitboard,
//     pub kind: AttackKind,
//     pub piece: ColoredPieceId,
// }
//
// impl PieceAttackBB {
//     pub fn insert_moves<L: MoveList<FairyBoard>>(
//         &self,
//         list: &mut L,
//         pos: &FairyBoard,
//         piece: FairyPiece,
//     ) {
//         let mut bb = FairyBitboard::new(self.bb, pos.size());
//         let from = piece.coordinates;
//         for to in bb.ones() {
//             let mut move_kinds = ArrayVec::new();
//             MoveKind::insert(&mut move_kinds, self, to, pos);
//             for kind in move_kinds {
//                 let mov = FairyMove {
//                     from: CompactSquare::new(from, pos.size()),
//                     to: CompactSquare::new(to, pos.size()),
//                     kind,
//                     is_capture: false,
//                 };
//                 list.add_move(mov);
//             }
//         }
//     }
// }
//
// // no pont in making this a trait as I don't want it to be extensible like at compile time
// // restriction: don't use traits or Box<dyn Fn> for "extensibility", just use enums.
// /// Bitand the generated attack bitboard with a bitboard given by this enum
// #[derive(Debug, Copy, Clone)]
// #[must_use]
// pub enum AttackBitboardFilter {
//     EmptySquares,
//     Them,
//     Us,
//     NotUs,
//     NotThem,
//     Rank(DimT),
//     File(DimT),
//     PawnCapture, // Them | {ep_square}
//     Custom(RawFairyBitboard),
// }
//
// impl AttackBitboardFilter {
//     pub fn bb(self, us: FairyColor, pos: &FairyBoard) -> RawFairyBitboard {
//         let bb = match self {
//             AttackBitboardFilter::EmptySquares => pos.empty_bb(),
//             AttackBitboardFilter::Them => pos.player_bb(!us),
//             AttackBitboardFilter::Us => pos.player_bb(us),
//             NotUs => !pos.player_bb(us),
//             AttackBitboardFilter::NotThem => !pos.player_bb(!us),
//             AttackBitboardFilter::Rank(rank) => FairyBitboard::rank_for(rank, pos.size()),
//             AttackBitboardFilter::File(file) => FairyBitboard::file_for(file, pos.size()),
//             AttackBitboardFilter::PawnCapture => {
//                 let ep_bb = pos
//                     .0
//                     .ep
//                     .map(|sq| FairyBitboard::single_piece_for(sq, pos.size()).raw())
//                     .unwrap_or_default();
//                 return ep_bb | pos.player_bb(!us).raw();
//             }
//             AttackBitboardFilter::Custom(bb) => return bb,
//         };
//         bb.raw()
//     }
// }
//
// #[must_use]
// pub enum GenAttacksCondition {
//     Always,
//     Side(FairyColor),
//     CanCastle(FairyColor),
//     OnRank(usize, FairyColor),
// }
//
// #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
// #[must_use]
// pub enum MoveKind {
//     #[default]
//     Normal,
//     // the given piece appears at the target square. Can be a promotion, or a drop like in m,n,k games
//     Drop(u8),
//     Castle,
//     Clone,
//     Conversion,
// }
//
// // this is also an upper bound of the number of pieces a pawn can promote to
// pub const MAX_MOVE_KINDS_PER_ATTACK: usize = 32;
//
// impl MoveKind {
//     fn insert(
//         list: &mut ArrayVec<MoveKind, MAX_MOVE_KINDS_PER_ATTACK>,
//         attack: &PieceAttackBB,
//         target: FairySquare,
//         pos: &FairyBoard,
//     ) {
//         // TODO: Checking `rules()` for every attack is unnecessarily slow
//         let promos = &rules().pieces[attack.piece.uncolor().0].promotions;
//         if !promos
//             .squares
//             .is_bit_set_at(pos.size().internal_key(target))
//         {
//             if attack.kind == AttackKind::Castle {
//                 list.push(MoveKind::Castle);
//             } else {
//                 list.push(MoveKind::Normal);
//             }
//         } else {
//             for &piece in &promos.pieces {
//                 list.push(MoveKind::Drop(piece.0 as u8));
//             }
//         }
//     }
// }
//
// /// Effect rules are stored in the rules and are used to determine the effect of each move.
// #[derive(Debug, Copy, Clone)]
// pub struct EffectRules {
//     reset_draw_counter_on_capture: bool,
//     // capture_on_ep: bool,
//     // capture_on_piece_move: Vec<PieceId>,
//     conversion_radius: usize,
//     explosion_radius: usize,
// }
//
// /// Each move has a set of properties, which is expressed as a bitset of up to 32 elements.
// /// Each property is associated with a list of `MoveEffect`s. The move itself only stores this struct,
// /// and the rules are used to turn it into a list of effects to apply
// #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
// pub struct MovePropertySet(pub u16);
//
// // Up to 3 move properties can use a single byte each to store additional data, such as the promotion piece.
// #[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
// pub struct MovePropertyData(pub [u8; 3]);
//
// /// A list of Move effects, associated with a MoveProperty
// type MoveEffectList = Vec<MoveEffect>;
//
// /// A MoveEffect is a low-level description of a way that a move can change the game state.
// /// Conceptually, each move kind is associated with a set of properties.
// /// Each property is associated with list of MoveEffects, for example resetting the draw counter on a capture.
// #[derive(Debug, Clone, Copy)]
// #[must_use]
// pub enum MoveEffect {
//     ResetDrawCtr,
//     PlaceSinglePiece(FairySquare, ColoredPieceId),
//     // if the source square is not valid, this effect will be ignored
//     RemoveSinglePiece(FairySquare),
//     ClearSquares(RawFairyBitboard),
//     SetColorTo(RawFairyBitboard, FairyColor),
//     SetEp(FairySquare),
// }
//
// impl MoveEffect {
//     fn apply(&self, pos: &mut UnverifiedFairyBoard) {
//         match *self {
//             MoveEffect::ResetDrawCtr => pos.draw_counter = 0,
//             PlaceSinglePiece(square, piece) => {
//                 debug_assert!(pos.is_empty(square));
//                 *pos = pos.place_piece(square, piece);
//             }
//             RemoveSinglePiece(square) => {
//                 *pos = pos.remove_piece(square);
//             }
//             ClearSquares(to_remove) => {
//                 // TODO: Maybe some kind of death callback would make sense? That's definitely not in the first version though
//                 // TODO: Maybe this should only clear some bitboards, e.g. there may be environmental bitboards that shouldn't be cleared
//                 for bb in &mut pos.piece_bitboards {
//                     *bb &= !to_remove;
//                 }
//                 for bb in &mut pos.color_bitboards {
//                     *bb &= !to_remove;
//                 }
//             }
//             SetColorTo(to_flip, color) => {
//                 let flipped = pos.color_bitboards[color.other() as usize] & to_flip;
//                 pos.color_bitboards[!color as usize] ^= flipped;
//                 pos.color_bitboards[color as usize] ^= flipped;
//             }
//             MoveEffect::SetEp(sq) => {
//                 pos.ep = Some(sq);
//             }
//         }
//     }
// }
// fn effects_for(mov: FairyMove, pos: &FairyBoard, r: EffectRules, list: &mut Vec<MoveEffect>) {
//     let from = mov.source(pos.size());
//     let to = mov.dest(pos.size());
//     let piece = pos.colored_piece_on(from).symbol;
//     let piece_rules = &rules().pieces[piece.uncolor().0];
//     if mov.is_capture {
//         let is_ep = piece_rules.can_ep_capture && Some(to) == pos.0.ep && mov.is_capture;
//         if is_ep {
//             list.push(RemoveSinglePiece(pos.0.ep.unwrap()));
//         } else {
//             list.push(RemoveSinglePiece(to));
//         }
//     }
//     match mov.kind {
//         MoveKind::Normal => {
//             list.push(RemoveSinglePiece(from));
//             list.push(PlaceSinglePiece(to, piece));
//         }
//         MoveKind::Drop(piece) => {
//             list.push(RemoveSinglePiece(from));
//             let piece = ColoredPieceId::from_u8(piece);
//             list.push(PlaceSinglePiece(to, piece));
//         }
//         MoveKind::Castle => 'castle: {
//             list.push(RemoveSinglePiece(from));
//             let castling_info = rules().castling_flags.players[pos.active_player() as usize];
//             for side in castling_info.sides {
//                 let Some(side) = side else { continue };
//                 if side.king_dest_square != to {
//                     continue;
//                 };
//                 let rook = pos.colored_piece_on(side.rook_square).symbol;
//                 list.push(RemoveSinglePiece(side.rook_square));
//                 list.push(PlaceSinglePiece(to, piece));
//                 list.push(PlaceSinglePiece(side.rook_dest_square, rook));
//                 break 'castle;
//             }
//             unreachable!("Castling move from {from} to {to} not valid");
//         }
//         MoveKind::Clone => {
//             list.push(PlaceSinglePiece(to, piece));
//         }
//         MoveKind::Conversion => {
//             let bb = FairyBitboard::single_piece_for(to, pos.size())
//                 .extended_moore_neighbors(r.conversion_radius);
//             list.push(SetColorTo(bb.raw(), pos.active_player()));
//         }
//     }
//     if (r.reset_draw_counter_on_capture && mov.is_capture) || piece_rules.reset_draw_counter {
//         list.push(ResetDrawCtr);
//     }
// }
//
// impl FairyBoard {
//     fn make_move_impl(mut self, mov: FairyMove) -> Option<Self> {
//         let mut effects = vec![];
//         let effect_rules = rules().effect_rules;
//         effects_for(mov, &self, effect_rules, &mut effects);
//         for effect in effects {
//             effect.apply(&mut self.0);
//         }
//         self.0.verify_with_level(Verify, Strict).ok()
//     }
// }
