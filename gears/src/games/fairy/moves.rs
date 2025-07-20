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
use crate::games::CharType::Ascii;
use crate::games::fairy::Side::{Kingside, Queenside};
use crate::games::fairy::attacks::{EffectRules, MoveKind};
use crate::games::fairy::effects::{AfterMove, InCheck};
use crate::games::fairy::moves::MoveEffect::{
    PlaceSinglePiece, RemoveCastlingRight, RemovePieceFromHand, RemoveSinglePiece, ResetDrawCtr, ResetEp, SetColorTo,
    SetEp,
};
use crate::games::fairy::pieces::{ColoredPieceId, PieceId};
use crate::games::fairy::rules::{PromoMoveChar, Rules};
use crate::games::fairy::{
    FairyBitboard, FairyBoard, FairyColor, FairySize, FairySquare, RawFairyBitboard, Side, effects,
};
use crate::games::{AbstractPieceType, Color, ColoredPieceType, DimT, Size};
use crate::general::bitboards::{Bitboard, RawBitboard};
use crate::general::board::SelfChecks::Verify;
use crate::general::board::Strictness::{Relaxed, Strict};
use crate::general::board::{BitboardBoard, Board, BoardHelpers, UnverifiedBoard};
use crate::general::common::{Res, tokens};
use crate::general::moves::{ExtendedFormat, Legality, Move, UntrustedMove};
use crate::general::squares::{CompactSquare, RectangularCoordinates};
use anyhow::bail;
use arbitrary::Arbitrary;
use colored::Colorize;
use num::range_step;
use std::fmt;
use std::fmt::Formatter;
use strum::IntoEnumIterator;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Arbitrary)]
#[must_use]
pub struct FairyMove {
    pub(super) from: CompactSquare,
    pub(super) to: CompactSquare,
    pub(super) packed: u16,
}

impl FairyMove {
    pub fn new(from: CompactSquare, to: CompactSquare, kind: MoveKind, is_capture: bool) -> Self {
        Self { from, to, packed: Self::pack(kind, is_capture) }
    }

    pub fn drop_move(piece: PieceId, to: FairySquare, size: FairySize) -> Self {
        Self::new(
            CompactSquare::new(FairySquare::no_coordinates(), size),
            CompactSquare::new(to, size),
            MoveKind::Drop(piece.to_uncolored_idx() as u8),
            false,
        )
    }

    pub(super) fn pack(kind: MoveKind, is_capture: bool) -> u16 {
        let (discriminant, val) = match kind {
            MoveKind::Normal => (0, 1), // ensure that the default value of `FairyMove` is never legal
            MoveKind::Drop(val) => (1, val),
            MoveKind::Promotion(val) => (2, val),
            MoveKind::Castle(side) => (3, side as u8),
            MoveKind::DoublePawnPush => (4, 0),
        };
        let discriminant = discriminant | ((is_capture as u16) << 7);
        ((val as u16) << 8) | discriminant
    }

    pub(super) fn unpack(val: u16) -> (MoveKind, bool) {
        let discriminant = val & ((1 << 7) - 1);
        let is_capture = val & (1 << 7) != 0;
        let val = (val >> 8) as u8;
        match discriminant {
            0 => (MoveKind::Normal, is_capture),
            1 => (MoveKind::Drop(val), is_capture),
            2 => (MoveKind::Promotion(val), is_capture),
            3 => (MoveKind::Castle(Side::from_repr(val as usize).unwrap()), is_capture),
            4 => (MoveKind::DoublePawnPush, is_capture),
            _ => unreachable!(),
        }
    }

    pub fn dest(self, size: FairySize) -> FairySquare {
        self.to.square(size)
    }

    pub fn source(self, size: FairySize) -> FairySquare {
        self.from.square(size)
    }

    pub fn piece(self, pos: &FairyBoard) -> ColoredPieceId {
        if let MoveKind::Drop(piece) = self.kind() {
            ColoredPieceId::new(pos.active_player(), PieceId::new(piece as usize))
        } else {
            pos.colored_piece_on(self.source(pos.size())).symbol
        }
    }

    pub fn kind(self) -> MoveKind {
        Self::unpack(self.packed).0
    }

    pub fn is_capture(self) -> bool {
        Self::unpack(self.packed).1
    }

    pub fn is_drop(self) -> bool {
        self.from.0 == DimT::MAX
    }
}

impl Move<FairyBoard> for FairyMove {
    type Underlying = u32;

    fn legality(rules: &Rules) -> Legality {
        rules.legality
    }

    fn src_square_in(self, pos: &FairyBoard) -> Option<FairySquare> {
        let sq = self.from.square(pos.size());
        if pos.size().coordinates_valid(sq) { Some(sq) } else { None }
    }

    fn dest_square_in(self, pos: &FairyBoard) -> FairySquare {
        self.to.square(pos.size())
    }

    fn is_tactical(self, _board: &FairyBoard) -> bool {
        self.is_capture()
    }

    fn format_compact(self, f: &mut Formatter<'_>, board: &FairyBoard) -> fmt::Result {
        format_move_compact(f, self, board).unwrap_or_else(|| write!(f, "<Invalid Fairy Move '{self:?}'>"))
    }

    fn format_extended(self, f: &mut Formatter<'_>, board: &FairyBoard, _format: ExtendedFormat) -> fmt::Result {
        // TODO: Actual implementation
        self.format_compact(f, board)
    }

    fn parse_compact_text<'a>(s: &'a str, board: &FairyBoard) -> Res<(&'a str, FairyMove)> {
        if s.is_empty() {
            bail!("empty move")
        } else if let Some(rest) = s.strip_prefix("0000") {
            return Ok((rest, Self::default()));
        }
        let moves = board.legal_moves_slow();
        let mut longest_match = (s, FairyMove::default());
        for m in &moves {
            let as_string = m.compact_formatter(board).to_string();
            if let Some(remaining) = s.strip_prefix(&as_string) {
                // it's common for a legal move to be a prefix of another legal move, e.g. in shogi-style promotions
                if remaining.len() < longest_match.0.len() {
                    longest_match = (remaining, *m);
                }
            }
        }
        if longest_match.0.len() != s.len() {
            return Ok(longest_match);
        }
        let moves_msg =
            format!("There are {0} legal moves in this position (type 'show moves' to view them)", moves.len());
        bail!("No legal move matches '{0}'. {1}", tokens(s).next().unwrap_or_default().red(), moves_msg.dimmed())
    }

    fn parse_extended_text<'a>(s: &'a str, board: &FairyBoard) -> Res<(&'a str, FairyMove)> {
        Self::parse_compact_text(s, board)
    }

    fn from_u64_unchecked(val: u64) -> UntrustedMove<FairyBoard> {
        UntrustedMove::from_move(Self {
            from: CompactSquare(val as DimT),
            to: CompactSquare((val >> 8) as DimT),
            packed: (val >> 16) as u16,
        })
    }

    fn to_underlying(self) -> Self::Underlying {
        (self.from.underlying() as u32) | ((self.to.underlying() as u32) << 8) | ((self.packed as u32) << 16)
    }
}

fn format_move_compact(f: &mut Formatter<'_>, mov: FairyMove, pos: &FairyBoard) -> Option<fmt::Result> {
    // don't check if coordinates are valid or similar because this function isn't supposed to panic
    // -- it might be called to print invalid moves from user input.
    let size = pos.size();
    let from = mov.from.square(size);
    let to = mov.to.square(size);
    let from = pos.square_formatter(from);
    let to = pos.square_formatter(to);
    Some(match mov.kind() {
        MoveKind::Normal | MoveKind::DoublePawnPush => {
            write!(f, "{from}{to}")
        }
        MoveKind::Promotion(new_piece) => {
            let piece = ColoredPieceId::from_u8(new_piece).to_uncolored_idx();
            let promo_char = match pos.rules().format_rules.promo_move_char {
                PromoMoveChar::Piece => pos.rules().pieces.get(piece)?.uncolored_symbol[Ascii].to_ascii_lowercase(),
                PromoMoveChar::Plus => '+',
            };
            write!(f, "{from}{to}{promo_char}")
        }
        MoveKind::Castle(side) => {
            let rook_sq = pos.0.castling_info.player(pos.active_player()).rook_sq(side)?;
            write!(f, "{from}{}", pos.square_formatter(rook_sq))
        }
        MoveKind::Drop(piece) => {
            if pos.rules().pieces.iter().filter(|p| !p.uncolored).count() <= 1 {
                write!(f, "{to}")
            } else {
                let piece = pos.rules().pieces.get(piece as usize)?;
                // although lichess doesn't use `P` for pawn drops in their human-readable notation,
                // fairy sf and cutechess do (at least in crazyhouse) in their UCI notation
                let drop_str = &pos.rules().format_rules.drop_str;
                write!(f, "{}{drop_str}{to}", piece.uncolored_symbol[Ascii])
            }
        }
    })
}

/// The rules describe which effects are triggered; triggering an effect can trigger other effects based on the rules
/// (e.g. the rules could say that the `Capture` effect triggers the `ResetDrawCtr` effect)
#[derive(Debug, Clone)]
#[must_use]
pub enum MoveEffect {
    Win,
    Draw,
    Lose,
    ResetDrawCtr,
    PlaceSinglePiece(FairySquare, ColoredPieceId),
    // if the source square is not valid, this effect will be ignored
    RemoveSinglePiece(FairySquare, ColoredPieceId),
    // ClearSquares(RawFairyBitboard),
    SetColorTo(RawFairyBitboard, FairyColor),
    SetEp(FairySquare),
    ResetEp,
    RemoveCastlingRight(FairyColor, Side),
    RemovePieceFromHand(PieceId, FairyColor),
    Capture(FairySquare),
    Promote(ColoredPieceId),
    ConvertOne(FairySquare),
    ConvertAll(FairyBitboard),
    MovesPiece(ColoredPieceId),
}

impl MoveEffect {
    fn apply(&self, pos: &mut FairyBoard) {
        let board = &mut pos.0;
        match *self {
            MoveEffect::ResetDrawCtr => board.draw_counter = 0,
            PlaceSinglePiece(square, piece) => {
                debug_assert!(board.is_empty(square), "{pos} {square}");
                board.place_piece(square, piece);
            }
            RemoveSinglePiece(square, piece) => {
                debug_assert_eq!(board.piece_on(square).symbol, piece);
                board.remove_piece_impl(square, piece);
            }
            // ClearSquares(to_remove) => {
            //     // TODO: Maybe some kind of death callback would make sense? That's definitely not in the first version though
            //     // TODO: Maybe this should only clear some bitboards, e.g. there may be environmental bitboards that shouldn't be cleared
            //     for bb in &mut board.piece_bitboards {
            //         *bb &= !to_remove;
            //     }
            //     for bb in &mut board.color_bitboards {
            //         *bb &= !to_remove;
            //     }
            // }
            SetColorTo(to_flip, color) => {
                let flipped = board.color_bitboards[color.other().idx()] & to_flip;
                board.color_bitboards[color.other().idx()] ^= flipped;
                board.color_bitboards[color.idx()] ^= flipped;
            }
            SetEp(sq) => {
                board.ep = Some(sq);
            }
            ResetEp => {
                board.ep = None;
            }
            RemoveCastlingRight(color, side) => {
                board.castling_info.unset(color, side);
            }
            RemovePieceFromHand(piece, color) => {
                board.in_hand[color][piece.val()] -= 1;
            }
            MoveEffect::Win => {}
            MoveEffect::Draw => {}
            MoveEffect::Lose => {}
            MoveEffect::Capture(_) => {}
            MoveEffect::Promote(_) => {}
            MoveEffect::ConvertOne(_) => {}
            MoveEffect::ConvertAll(_) => {}
            MoveEffect::MovesPiece(_) => {}
        }
    }
}

fn effects_for(mov: FairyMove, pos: &mut FairyBoard, r: EffectRules) -> Option<()> {
    let from = mov.from.square(pos.size());
    let to = mov.dest(pos.size());
    let piece = mov.piece(pos);
    let piece_rules = &pos.rules().pieces[piece.uncolor().val()];
    let mut set_ep = None;
    let mut captured = None;
    if mov.is_capture() {
        let is_ep = piece_rules.can_ep_capture && Some(to) == pos.0.ep;
        if is_ep {
            let sq = pos.0.ep.unwrap().pawn_push(!piece.color().unwrap().is_first());
            let capt = pos.piece_on(sq).symbol;
            RemoveSinglePiece(sq, capt).apply(pos);
            captured = Some(capt);
        } else {
            let capt = pos.piece_on(to).symbol;
            RemoveSinglePiece(to, capt).apply(pos);
            captured = Some(capt);
        }
    }
    // TODO: Needlessy inefficient because it needs to look up the piece bitboards from placing and removing,
    // should also use event handling system at least for the more niche use cases (probably fine to hard-code
    // normal move and drop)
    match mov.kind() {
        MoveKind::Normal => {
            RemoveSinglePiece(from, piece).apply(pos);
            PlaceSinglePiece(to, piece).apply(pos);
        }
        MoveKind::DoublePawnPush => {
            RemoveSinglePiece(from, piece).apply(pos);
            PlaceSinglePiece(to, piece).apply(pos);
            let ep_capture_bb = FairyBitboard::single_piece_for(to, pos.size());
            let ep_capture_bb = ep_capture_bb.west() | ep_capture_bb.east();
            if (pos.col_piece_bb(piece.color().unwrap().other(), piece.uncolor()) & ep_capture_bb).has_set_bit() {
                set_ep = Some(to.pawn_push(!piece.color().unwrap().is_first()));
            }
        }
        MoveKind::Drop(piece) => {
            let piece = PieceId::new(piece as usize);
            let col_piece = ColoredPieceId::create(piece, Some(pos.active_player()));
            PlaceSinglePiece(to, col_piece).apply(pos);
            RemovePieceFromHand(piece, pos.active_player()).apply(pos);
        }
        MoveKind::Promotion(new_piece) => {
            RemoveSinglePiece(from, piece).apply(pos);
            let piece = ColoredPieceId::from_u8(new_piece);
            PlaceSinglePiece(to, piece).apply(pos);
        }
        MoveKind::Castle(side) => {
            debug_assert!(pos.0.castling_info.can_castle(pos.active_player(), side));
            let castling_info = pos.0.castling_info.players[pos.active_player().idx()];
            debug_assert_eq!(castling_info.king_dest_sq(side), Some(to));
            let rook_sq = castling_info.rook_sq(side).unwrap();
            let rook = pos.colored_piece_on(rook_sq).symbol;
            RemoveSinglePiece(from, piece).apply(pos);
            RemoveSinglePiece(rook_sq, rook).apply(pos);
            PlaceSinglePiece(to, piece).apply(pos);
            PlaceSinglePiece(castling_info.rook_dest_sq(side).unwrap(), rook).apply(pos);
        }
    }
    if r.conversion_radius > 0 {
        let bb = FairyBitboard::single_piece_for(to, pos.size()).extended_moore_neighbors(r.conversion_radius);
        SetColorTo(bb.raw(), pos.active_player()).apply(pos);
    }
    let piece_rules = &pos.rules().pieces[piece.uncolor().val()];
    if (r.reset_draw_counter_on_capture && mov.is_capture()) || piece_rules.resets_draw_counter.reset(mov) {
        ResetDrawCtr.apply(pos);
    }
    if let Some(ep) = set_ep {
        SetEp(ep).apply(pos);
    } else {
        ResetEp.apply(pos);
    }
    if pos.rules().has_castling {
        for color in FairyColor::iter() {
            let castling_bb = pos.castling_bb() & pos.player_bb(color);
            if (mov.src_square_in(pos).is_some() && castling_bb.is_bit_set_at(pos.size().internal_key(from)))
                || castling_bb.is_bit_set_at(pos.size().internal_key(to))
            {
                RemoveCastlingRight(color, Kingside).apply(pos);
                RemoveCastlingRight(color, Queenside).apply(pos);
            }
            for side in Side::iter() {
                if [Some(from), Some(to)].contains(&pos.0.castling_info.player(color).rook_sq(side)) {
                    RemoveCastlingRight(color, side).apply(pos)
                }
            }
        }
    }
    if mov.is_capture() {
        let event = effects::Capture { square: to, captured: captured.unwrap() };
        pos.emit(event)?;
    }
    Some(())
}

impl FairyBoard {
    // can temporarily modify self
    fn can_make_move(&mut self, mov: FairyMove) -> bool {
        let MoveKind::Castle(side) = mov.kind() else {
            return true;
        };
        // Castling legality works like this: First, we see if we're in check before making the move.
        // Then, while the king isn't on its dest square, we move it one square closer to that and see if we're in check.
        // When the king reaches its dest square, we immediately put the rook on its dest square before seeing if we're in check.
        // (This isn't done as part of this function, but instead by testing if the new position leaves us in check.)
        // If the king crosses the rook square during castling, we temporarily remove the rook while the king is on that square.
        let us = self.active_player();
        let castling = self.0.castling_info.player(us);
        let from = mov.source(self.size());
        let to = mov.dest(self.size());
        let king = self.piece_type_on(from);
        let rook_sq = castling.rook_sq(side).unwrap();
        let rook_dest_sq = castling.rook_dest_sq(side).unwrap();
        debug_assert!(self.castling_bb().is_bit_set_at(self.size().internal_key(from)));
        debug_assert_eq!(castling.king_dest_sq(side).unwrap(), to);
        debug_assert_eq!(to.rank(), from.rank());
        debug_assert_eq!(castling.rank, to.rank());
        debug_assert!(Some(to) == castling.king_dest_sq(Queenside) || Some(to) == castling.king_dest_sq(Kingside));
        let occupied = self.occupied_bb()
            ^ FairyBitboard::single_piece_for(rook_sq, self.size())
            ^ FairyBitboard::single_piece_for(from, self.size());
        let ray = FairyBitboard::ray_inclusive(from, to, self.size())
            | FairyBitboard::ray_inclusive(rook_sq, rook_dest_sq, self.size());
        if (occupied & ray).has_set_bit() {
            return false;
        }
        // For chess, we could simply compute the attack bitboard of the opponent and intersect that with te squares that
        // our king is crossing. However, variants like atomic have more complicated rules for being in check,
        // so we have to simulate the castling move step by step
        let mut res = true;
        self.0.xor_given_piece_at(from, king, us);
        let mut rook = None;
        let step = if to.file() < from.file() { -1 } else { 1 };
        for file in range_step(from.file() as isize, to.file() as isize, step) {
            let sq = FairySquare::from_rank_file(from.rank(), file as DimT);
            if sq == rook_sq {
                let r = self.piece_type_on(sq);
                self.0.xor_given_piece_at(sq, r, us);
                rook = Some(r);
            }
            self.0.xor_given_piece_at(sq, king, us);
            res &= !self.compute_is_in_check(us);
            self.0.xor_given_piece_at(sq, king, us);
            if sq == rook_sq {
                self.0.xor_given_piece_at(sq, rook.unwrap(), us);
            }
        }
        self.0.xor_given_piece_at(from, king, us);
        if from.file() == to.file() {
            // we need to test explicitly whether we're in check before the move
            res &= !self.is_in_check();
        }
        // testing the dest square is unnecessary because that already gets done after playing the move
        res
    }

    pub(super) fn make_move_impl(mut self, mov: FairyMove) -> Option<Self> {
        if cfg!(debug_assertions) {
            _ = self.debug_verify_invariants(Strict).unwrap();
        }
        // pseudolegal movegen: Some expensive conditions are checked here instead of when generating the move.
        // `end_move` does further expensive checks, like testing if the new sntm is in check
        if !self.can_make_move(mov) {
            return None;
        }
        self.0.draw_counter += 1; // do this before an effect could reset it to zero
        let effect_rules = self.rules().effect_rules;
        effects_for(mov, &mut self, effect_rules)?;
        if self.rules().store_last_move {
            self.0.last_move = mov;
        }
        self.end_move(mov)
    }

    pub(super) fn end_move(mut self, mov: FairyMove) -> Option<Self> {
        if self.settings().must_preserve_own_king[self.active.idx()] && self.royal_bb_for(self.active).is_zero() {
            return None;
        }
        self.adjust_castling_rights();
        for c in FairyColor::iter() {
            self.0.in_check[c] = self.compute_is_in_check(c);
            if self.in_check[c] {
                self.emit(InCheck { color: c, last_move: mov })?;
            }
        }
        self.flip_side_to_move(mov)
    }

    fn adjust_castling_rights(&mut self) {
        for color in FairyColor::iter() {
            let info = self.castling_info.player(color);
            if (self.castling_bb_for(color) & FairyBitboard::rank_for(info.rank, self.size())).is_zero() {
                self.0.castling_info.unset_both_sides(color);
            }
            for side in Side::iter() {
                let info = self.castling_info.player(color);
                let Some(sq) = info.rook_sq(side) else { continue };
                if !self.player_bb(color).is_bit_set_at(self.size().internal_key(sq)) {
                    self.0.castling_info.unset(color, side);
                }
            }
        }
    }

    /// Called at the end of [`Self::make_nullmove`] and [`Self::make_move`].
    /// `last_move` might be a null move
    pub(super) fn flip_side_to_move(mut self, last_move: FairyMove) -> Option<Self> {
        self.flip_stm_unchecked();
        if !self.rules().check_rules.satisfied(&self) {
            return None;
        }
        self.emit(AfterMove { last_move })?;
        if cfg!(debug_assertions) {
            // unlike `debug_assert!(.is_ok())`, this prints the error in case of a failure
            _ = self.0.clone().verify_with_level(Verify, Relaxed).unwrap();
        }
        Some(self)
    }

    fn flip_stm_unchecked(&mut self) {
        self.0.active = !self.0.active;
        self.0.hash = self.compute_hash();
        self.0.ply_since_start += 1;
    }
}
