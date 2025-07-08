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
use crate::games::fairy::moves::FairyMove;
use crate::games::fairy::pieces::{ColoredPieceId, PieceId, SHOGI_PAWN_IDX};
use crate::games::fairy::rules::GameEndEager;
use crate::games::fairy::{FairyBitboard, FairyBoard, FairyColor, FairySquare, RawFairyBitboard, Side};
use crate::games::{ColoredPieceType, NoHistory};
use crate::general::bitboards::Bitboard;
use crate::general::board::{Board, UnverifiedBoard};
use arbitrary::{Arbitrary, Unstructured};
use std::fmt::Debug;

/// Events are meant to be *fast*, so they are generally low-level and don't check invariants.
/// For example, `PlacePiece` simply assumes that it's ok to place the given piece at the given square.
pub trait Event: Copy {
    fn execute(self, _pos: &mut FairyBoard)
    where
        Self: Sized,
    {
        // default implementation: Do nothing
    }

    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()>;
}

fn notify<T: Event>(observers: &[Box<ObsFn<T>>], event: T, pos: &mut FairyBoard) -> Option<()> {
    for observer in observers {
        observer(event, pos)?;
    }
    Some(())
}

#[derive(Debug, Copy, Clone)]
pub struct NoMoves;

#[derive(Debug, Copy, Clone)]
pub struct InCheck {
    pub color: FairyColor,
    #[allow(unused)]
    pub last_move: FairyMove,
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct GameEndEagerEvent<'a>(&'a GameEndEager);

#[derive(Debug, Copy, Clone)]
pub struct ResetDrawCtr;

#[derive(Debug, Copy, Clone)]
pub struct PlaceSinglePiece {
    pub square: FairySquare,
    pub piece: ColoredPieceId,
}

#[derive(Debug, Copy, Clone)]
pub struct RemoveSinglePiece {
    pub square: FairySquare,
}

#[derive(Debug, Copy, Clone)]
pub struct RemoveAll {
    pub bb: RawFairyBitboard,
}

#[derive(Debug, Copy, Clone)]
pub struct SetEp {
    pub square: FairySquare,
}

#[derive(Debug, Copy, Clone)]
pub struct ResetEp;

#[derive(Debug, Copy, Clone)]
pub struct RemoveCastlingRight {
    pub color: FairyColor,
    pub side: Side,
}

#[derive(Debug, Copy, Clone)]
pub struct RemovePieceFromHand {
    pub piece: PieceId,
    pub color: FairyColor,
}

#[derive(Debug, Copy, Clone)]
pub struct AddPieceToHand {
    pub piece: PieceId,
    pub color: FairyColor,
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct Capture {
    pub square: FairySquare,
    pub captured: ColoredPieceId,
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct Promote {
    pub piece: ColoredPieceId,
    pub square: FairySquare,
}

#[derive(Debug, Copy, Clone)]
pub struct ConvertOne {
    pub square: FairySquare,
}

// by default, this doesn't create `ConvertOne` events (though this can be set up though an observer, of course)
#[derive(Debug, Copy, Clone)]
pub struct ConvertAll {
    pub bb: RawFairyBitboard,
}

#[derive(Debug, Copy, Clone)]
#[allow(dead_code)]
pub struct MovePiece {
    pub piece: ColoredPieceId,
}

#[derive(Debug, Copy, Clone)]
pub struct AfterMove {
    pub last_move: FairyMove,
}

impl Event for NoMoves {
    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()> {
        notify(observers.no_moves.as_slice(), self, pos)
    }
}

impl Event for InCheck {
    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()> {
        notify(observers.in_check.as_slice(), self, pos)
    }
}

impl<'a> Event for GameEndEagerEvent<'a> {
    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()> {
        notify(observers.game_end_eager.as_slice(), self, pos)
    }
}

impl Event for ResetDrawCtr {
    fn execute(self, pos: &mut FairyBoard) {
        pos.0.draw_counter = 0;
    }

    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()> {
        notify(observers.reset_draw_ctr.as_slice(), self, pos)
    }
}

impl Event for PlaceSinglePiece {
    fn execute(self, pos: &mut FairyBoard) {
        pos.0.place_piece(self.square, self.piece);
    }

    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()> {
        notify(observers.place_single_piece.as_slice(), self, pos)
    }
}

impl Event for RemoveSinglePiece {
    fn execute(self, pos: &mut FairyBoard) {
        pos.0.remove_piece(self.square);
    }

    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()> {
        notify(observers.remove_single_piece.as_slice(), self, pos)
    }
}

impl Event for RemoveAll {
    fn execute(self, pos: &mut FairyBoard) {
        pos.0.remove_all_pieces(self.bb);
    }

    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()> {
        notify(observers.remove_all.as_slice(), self, pos)
    }
}

impl Event for SetEp {
    fn execute(self, pos: &mut FairyBoard) {
        pos.0.ep = Some(self.square);
    }

    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()> {
        notify(observers.set_ep.as_slice(), self, pos)
    }
}

impl Event for ResetEp {
    fn execute(self, pos: &mut FairyBoard) {
        pos.0.ep = None;
    }

    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()> {
        notify(observers.reset_ep.as_slice(), self, pos)
    }
}

impl Event for RemoveCastlingRight {
    fn execute(self, pos: &mut FairyBoard) {
        pos.0.castling_info.unset(self.color, self.side)
    }

    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()> {
        notify(observers.remove_castling_right.as_slice(), self, pos)
    }
}

impl Event for RemovePieceFromHand {
    fn execute(self, pos: &mut FairyBoard) {
        debug_assert!(pos.0.in_hand[self.color][self.piece.val()] > 0);
        pos.0.in_hand[self.color][self.piece.val()] -= 1;
    }

    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()> {
        notify(observers.remove_piece_from_hand.as_slice(), self, pos)
    }
}

impl Event for AddPieceToHand {
    fn execute(self, pos: &mut FairyBoard) {
        let val = &mut pos.0.in_hand[self.color][self.piece.val()];
        *val = val.saturating_add(1);
    }

    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()> {
        notify(observers.add_piece_to_hand.as_slice(), self, pos)
    }
}

impl Event for Capture {
    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()> {
        notify(observers.capture.as_slice(), self, pos)
    }
}

impl Event for Promote {
    fn execute(self, _pos: &mut FairyBoard) {
        // not all captures replace a piece, e.g. en passant.
        // because captures are so common and varied, we deal with the actual capturing mechanics explicitly in make_move,
        // so executing this has no effect
    }

    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()> {
        notify(observers.promote.as_slice(), self, pos)
    }
}

impl Event for ConvertAll {
    fn execute(self, pos: &mut FairyBoard) {
        pos.0.color_bitboards[0] ^= self.bb;
        pos.0.color_bitboards[1] ^= self.bb;
    }

    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()> {
        notify(observers.convert_all.as_slice(), self, pos)
    }
}

impl Event for ConvertOne {
    fn execute(self, pos: &mut FairyBoard) {
        let bb = FairyBitboard::single_piece_for(self.square, pos.size()).raw();
        pos.0.color_bitboards[0] ^= bb;
        pos.0.color_bitboards[1] ^= bb;
    }

    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()> {
        notify(observers.convert_one.as_slice(), self, pos)
    }
}

impl Event for MovePiece {
    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()> {
        notify(observers.move_piece.as_slice(), self, pos)
    }
}

impl Event for AfterMove {
    fn notify(self, observers: &Observers, pos: &mut FairyBoard) -> Option<()> {
        notify(observers.finish_move.as_slice(), self, pos)
    }
}

#[allow(type_alias_bounds)]
type ObsFn<T: Event> = dyn Fn(T, &mut FairyBoard) -> Option<()> + Sync + Send;

#[allow(type_alias_bounds)]
type ObsList<T: Event> = Vec<Box<ObsFn<T>>>;

type GameEndEagerObsFn = dyn Fn(GameEndEagerEvent<'_>, &mut FairyBoard) -> Option<()> + Sync + Send;

/// Observers are meant for custom hooks that trigger additional effects on certain events.
/// However, since they're comparatively slow and hard to configure at runtime (can't create new functions at runtime),
/// many of the common effects, such as removing a piece when it is captured, are still hardcoded.
#[derive(Default)]
pub struct Observers {
    pub(super) no_moves: ObsList<NoMoves>,
    pub(super) in_check: ObsList<InCheck>,
    pub(super) game_end_eager: Vec<Box<GameEndEagerObsFn>>,
    pub(super) reset_draw_ctr: ObsList<ResetDrawCtr>,
    pub(super) place_single_piece: ObsList<PlaceSinglePiece>,
    pub(super) remove_single_piece: ObsList<RemoveSinglePiece>,
    pub(super) remove_all: ObsList<RemoveAll>,
    pub(super) set_ep: ObsList<SetEp>,
    pub(super) reset_ep: ObsList<ResetEp>,
    pub(super) remove_castling_right: ObsList<RemoveCastlingRight>,
    pub(super) remove_piece_from_hand: ObsList<RemovePieceFromHand>,
    pub(super) add_piece_to_hand: ObsList<AddPieceToHand>,
    pub(super) capture: ObsList<Capture>,
    pub(super) promote: ObsList<Promote>,
    pub(super) convert_one: ObsList<ConvertOne>,
    pub(super) convert_all: ObsList<ConvertAll>,
    pub(super) move_piece: ObsList<MovePiece>,
    pub(super) finish_move: ObsList<AfterMove>,
}

impl Debug for Observers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Observers")
    }
}

impl Arbitrary<'_> for Observers {
    fn arbitrary(_u: &mut Unstructured<'_>) -> arbitrary::Result<Self> {
        Ok(Observers::default())
    }
}

impl Observers {
    pub fn chess() -> Self {
        // chess events are hardcoded (in the sense that there's a switch over an enum for whether they apply)
        // because they're very common
        Self::default()
    }

    pub fn atomic(pawn: PieceId) -> Self {
        let mut res = Self::chess();
        let explosion = move |event: Capture, pos: &mut FairyBoard| {
            let bb = FairyBitboard::single_piece_for(event.square, pos.size());
            // Could use precomputed king attacks for this
            let bb = (bb | (bb.moore_neighbors() & !pos.piece_bb(pawn))).raw();
            pos.emit(RemoveAll { bb })
        };
        res.capture.push(Box::new(explosion));
        res
    }

    pub fn add_captured_to_hand() -> Box<ObsFn<Capture>> {
        let add_to_hand = |event: Capture, pos: &mut FairyBoard| {
            let mut piece = event.captured.uncolor();
            if let Some(unpromoted) = piece.get(pos.rules()).promotions.promoted_from {
                piece = unpromoted;
            }
            pos.emit(AddPieceToHand { piece, color: pos.active_player() })
        };
        Box::new(add_to_hand)
    }

    pub fn n_check() -> Self {
        let mut res = Self::chess();
        let incr_check_ctr = |event: InCheck, pos: &mut FairyBoard| {
            pos.0.additional_ctrs[event.color] += 1;
            Some(())
        };
        res.in_check.push(Box::new(incr_check_ctr));
        res
    }

    pub fn crazyhouse() -> Self {
        let mut res = Self::chess();
        res.capture.push(Self::add_captured_to_hand());
        res
    }

    pub fn shogi() -> Self {
        let mut res = Self::chess();
        res.capture.push(Self::add_captured_to_hand());
        let no_pawn_drop_mate = |event: AfterMove, pos: &mut FairyBoard| {
            let m = event.last_move;
            if m.piece(pos).uncolor().val() == SHOGI_PAWN_IDX
                && m.is_drop()
                && pos.is_in_check()
                && pos.is_game_lost_slow(&NoHistory::default())
            {
                return None;
            }
            Some(())
        };
        res.finish_move.push(Box::new(no_pawn_drop_mate));
        res
    }

    pub fn ataxx() -> Self {
        let mut res = Self::default();
        let place_piece_fn = |event: PlaceSinglePiece, pos: &mut FairyBoard| {
            let bb = FairyBitboard::single_piece_for(event.square, pos.size());
            // Could use precomputed attacks for this
            let bb = bb.moore_neighbors().raw();
            pos.emit(ConvertAll { bb })
        };
        res.place_single_piece.push(Box::new(place_piece_fn));
        res
    }

    pub fn mnk() -> Self {
        Self::default()
    }
}

impl FairyBoard {
    pub fn emit<E: Event>(&mut self, e: E) -> Option<()> {
        e.execute(self);
        // unfortunately, this Arc clone is necessary to satisfy the borrow checker -- TODO: maybe find a way to avoid that
        let rules = self.rules.clone();
        let observers = &rules.get().observers;
        e.notify(observers, self)
    }
}
