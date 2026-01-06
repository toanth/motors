use crate::games::chess::CastleRight::*;
use crate::games::chess::Color::*;
use crate::games::chess::castling::CastleRight;
use crate::games::chess::moves::Move;
use crate::games::chess::moves::MoveFlags::*;
use crate::games::chess::pieces::PieceType::*;
use crate::games::chess::pieces::{ColoredPieceType, PieceType};
use crate::games::chess::squares::{C_FILE_NUM, ChessboardSize, G_FILE_NUM, Square};
use crate::games::chess::{Board, ChessBitboardTrait, Color, PAWN_CAPTURES};
use crate::games::{BoardTrait, ColorTrait, ColoredPieceTypeTrait};
use crate::general::bitboards::chessboard::{BISHOPS, Bitboard, INFINITE_RAYS, KINGS, KNIGHTS, RAYS_INCLUSIVE, ROOKS};
use crate::general::bitboards::{BitboardTrait, KnownSizeBitboard, RawBitboardTrait};
use crate::general::board::BitboardBoard;
use crate::general::hq::{ChessSliderGenerator, all_bishop_attacks, all_rook_attacks};
use crate::general::squares::RectangularCoordinates;

impl Board {
    pub fn slider_generator(&self) -> ChessSliderGenerator {
        ChessSliderGenerator::new(self.occupied_bb())
    }

    fn single_pawn_moves(color: Color, square: Square, capture_filter: Bitboard, push_filter: Bitboard) -> Bitboard {
        let captures = Self::single_pawn_captures(color, square) & capture_filter;
        // the bitand here is necessary to prevent double pushes across blockers
        let mut pushes = square.bb().pawn_advance(color) & push_filter;
        if square.is_pawn_start_rank() {
            pushes |= pushes.pawn_advance(color) & push_filter;
        }
        captures | pushes
    }

    /// This doesn't include castle moves and pawn pushes because those can never capture and are generally special:
    /// For example, it's possible that a normal king move is legal, but a
    /// chess960 castling move with the same source and dest square as the normal king move isn't, or the other way around.
    pub fn threatening_attacks(
        square: Square,
        piece: PieceType,
        color: Color,
        slider_generator: &ChessSliderGenerator,
    ) -> Bitboard {
        match piece {
            Pawn => Self::single_pawn_captures(color, square),
            Knight => Self::knight_attacks_from(square),
            Bishop => slider_generator.bishop_attacks(square),
            Rook => slider_generator.rook_attacks(square),
            Queen => slider_generator.queen_attacks(square),
            King => Self::normal_king_attacks_from(square),
            Empty => Bitboard::default(),
        }
    }

    fn check_castling_move_pseudolegal(&self, mov: Move, color: Color) -> bool {
        self.col_piece_bb(color, King).has(mov.src_square())
            && ((self.rook_start_square(color, Kingside) == mov.dest_square()
                && mov.castle_side() == Kingside
                && self.is_castling_pseudolegal(Kingside))
                || (self.rook_start_square(color, Queenside) == mov.dest_square()
                    && mov.castle_side() == Queenside
                    && self.is_castling_pseudolegal(Queenside)))
    }

    pub fn is_move_pseudolegal_impl(&self, mov: Move) -> bool {
        let src = mov.src_square();
        let dest = mov.dest_square();
        let us = self.active;
        if !self.player_bb(us).has(src) || mov.try_get_flags().is_none() {
            return false;
        }
        if mov.is_castle() {
            return self.check_castling_move_pseudolegal(mov, us);
        }
        let piece = mov.piece_type(self);
        let invalid = if piece == King {
            self.threats.has(dest)
        } else {
            match self.checkers.num_ones() {
                0 => false,
                1 => {
                    if mov.is_ep() {
                        false
                    } else {
                        let checker = self.checkers().to_square().unwrap();
                        !RAYS_INCLUSIVE[self.king_sq(us)][checker].is_bit_set_at(dest.bb_idx())
                    }
                }
                _ => true,
            }
        };
        if invalid {
            return false;
        };

        if piece == Pawn {
            if mov.is_ep() {
                return Some(dest) == self.ep_square && src.bb().pawn_attacks(us).has(dest);
            }
            let incorrect_promo = mov.is_promotion() != dest.is_backrank();
            let capturable = self.player_bb(us.other());
            !incorrect_promo && Self::single_pawn_moves(us, src, capturable, self.empty_bb()).has(dest)
        } else {
            if mov.is_promotion() || mov.is_ep() {
                return false;
            }
            let generator = self.slider_generator();
            (Self::threatening_attacks(src, mov.piece_type(self), us, &generator) & !self.active_player_bb()).has(dest)
        }
    }

    /// Used for verifying FENs and in assertions:
    /// Pretend there is a king of color `us` at `square` and test if it is in check.
    pub fn is_in_check_on_square(&self, us: Color, square: Square) -> bool {
        let slider_gen = self.slider_generator();
        self.all_attacking(square, slider_gen).intersects(self.player_bb(us.other()))
    }

    pub(super) fn gen_pseudolegal_moves<const ONLY_TACTICAL: bool>(
        &self,
        callback: &mut impl FnMut(Move),
        mut filter: Bitboard,
    ) {
        let slider_generator = self.slider_generator();
        self.gen_king_moves(callback, filter, ONLY_TACTICAL);
        let mut check_ray = !Bitboard::default();
        match self.checkers.num_ones() {
            0 => {}
            1 => {
                let checker = Square::from_bb_idx(self.checkers().pop_lsb());
                check_ray = Bitboard::ray_inclusive(self.king_sq(self.active), checker, ChessboardSize::default());
                filter &= check_ray;
            }
            // in a double check, only generate king moves. We support loading FENs with more than 2 checkers.
            _ => return,
        }
        self.gen_slider_moves::<{ Bishop as usize }>(callback, filter, &slider_generator);
        self.gen_slider_moves::<{ Rook as usize }>(callback, filter, &slider_generator);
        self.gen_slider_moves::<{ Queen as usize }>(callback, filter, &slider_generator);
        self.gen_knight_moves(callback, filter);
        self.gen_pawn_moves::<ONLY_TACTICAL>(callback, check_ray);
    }

    fn gen_pawn_moves<const ONLY_TACTICAL: bool>(&self, callback: &mut impl FnMut(Move), filter: Bitboard) {
        let us = self.active;
        let pawns = self.col_piece_bb(us, Pawn);
        let free = !self.occupied_bb();
        let mut free_filter = free & filter;
        if ONLY_TACTICAL {
            free_filter &= Bitboard::backranks();
        }
        let opponent = self.player_bb(!us) & filter;
        let regular_pawn_moves;
        let double_pawn_moves;
        let left_pawn_captures;
        let right_pawn_captures;
        let capturable = opponent | self.ep_square.map(Square::bb).unwrap_or_default();
        if us == White {
            regular_pawn_moves = (pawns.north() & free_filter, 8);
            double_pawn_moves = (((pawns & Bitboard::rank(1)) << 16) & free.north() & free_filter, 16);
            right_pawn_captures = (pawns.north_east() & capturable, 9);
            left_pawn_captures = (pawns.north_west() & capturable, 7);
        } else {
            regular_pawn_moves = (pawns.south() & free_filter, -8);
            double_pawn_moves = (((pawns & Bitboard::rank(6)) >> 16) & free.south() & free_filter, -16);
            right_pawn_captures = (pawns.south_west() & capturable, -9);
            left_pawn_captures = (pawns.south_east() & capturable, -7);
        }
        for capture in [right_pawn_captures, left_pawn_captures] {
            let bb = capture.0;
            for to in bb {
                let from = Square::from_bb_idx((to.to_u8() as isize - capture.1) as usize);
                if self.ep_square.is_some_and(|sq| sq == to) {
                    callback(Move::new(from, to, EnPassant));
                } else if !to.is_backrank() {
                    callback(Move::new(from, to, NormalMove));
                } else {
                    callback(Move::new(from, to, PromoQueen));
                    callback(Move::new(from, to, PromoKnight));
                    // even a capturing rook or bishop promo is not considered tactical
                    if !ONLY_TACTICAL {
                        callback(Move::new(from, to, PromoRook));
                        callback(Move::new(from, to, PromoBishop));
                    }
                    continue;
                }
            }
        }
        let bb = regular_pawn_moves.0;
        for to in bb {
            let from = Square::from_bb_idx((to.to_u8() as isize - regular_pawn_moves.1) as usize);
            if to.is_backrank() {
                callback(Move::new(from, to, PromoQueen));
                callback(Move::new(from, to, PromoKnight));
                if !ONLY_TACTICAL {
                    callback(Move::new(from, to, PromoRook));
                    callback(Move::new(from, to, PromoBishop));
                }
            } else {
                debug_assert!(!ONLY_TACTICAL);
                callback(Move::new(from, to, NormalMove));
            }
        }
        if !ONLY_TACTICAL {
            for to in double_pawn_moves.0 {
                let from = Square::from_bb_idx((to.to_u8() as isize - double_pawn_moves.1) as usize);
                callback(Move::new(from, to, NormalMove));
            }
        }
    }
    pub(super) fn pawn_advance_dests(&self) -> Bitboard {
        let us = self.active;
        let pawns = self.col_piece_bb(us, Pawn);
        let empty = self.empty_bb();
        let res = pawns.pawn_advance(us);
        let res = res | (res & Bitboard::pawn_ranks() & empty).pawn_advance(us);
        res & empty
    }

    fn is_castling_pseudolegal(&self, side: CastleRight) -> bool {
        let color = self.active;
        let king_square = self.king_sq(color);
        let king = self.col_piece_bb(color, King);
        // Castling, handling the general (D)FRC case.
        let king_file = king_square.file() as usize;
        const KING_QUEENSIDE_BB: [Bitboard; 8] = [
            Bitboard::new(!0), // impossible
            Bitboard::new(0b0000_0100),
            Bitboard::new(0b0000_0000), // no square to check
            Bitboard::new(0b0000_0100),
            Bitboard::new(0b0000_1100),
            Bitboard::new(0b0001_1100),
            Bitboard::new(0b0011_1100),
            Bitboard::new(!0), // impossible
        ];
        const KING_KINGSIDE_BB: [Bitboard; 8] = [
            Bitboard::new(!0), // impossible
            Bitboard::new(0b0111_1100),
            Bitboard::new(0b0111_1000),
            Bitboard::new(0b0111_0000),
            Bitboard::new(0b0110_0000),
            Bitboard::new(0b0100_0000),
            Bitboard::new(0b0000_0000),
            Bitboard::new(!0), // impossible
        ];
        const ROOK_QUEENSIDE_BB: [Bitboard; 8] = [
            Bitboard::new(0b0000_1110),
            Bitboard::new(0b0000_1100),
            Bitboard::new(0b0000_1000),
            Bitboard::new(0b0000_0000),
            Bitboard::new(0b0000_1000),
            Bitboard::new(0b0001_1000),
            Bitboard::new(!0), // impossible
            Bitboard::new(!0), // impossible
        ];
        const ROOK_KINGSIDE_BB: [Bitboard; 8] = [
            Bitboard::new(!0), // impossible
            Bitboard::new(!0), // impossible
            Bitboard::new(0b0011_1000),
            Bitboard::new(0b0011_0000),
            Bitboard::new(0b0010_0000),
            Bitboard::new(0b0000_0000),
            Bitboard::new(0b0010_0000),
            Bitboard::new(0b0110_0000),
        ];
        let (rook_free_bb, king_free_bb) = match side {
            Queenside => (
                ROOK_QUEENSIDE_BB[self.castling.rook_start_file(color, Queenside) as usize] << (color as usize * 7 * 8),
                KING_QUEENSIDE_BB[king_file] << (color as usize * 7 * 8),
            ),
            Kingside => (
                ROOK_KINGSIDE_BB[self.castling.rook_start_file(color, Kingside) as usize] << (color as usize * 7 * 8),
                KING_KINGSIDE_BB[king_file] << (color as usize * 7 * 8),
            ),
        };
        if self.castling.can_castle(color, side) {
            let rook = self.rook_start_square(color, side);
            if ((self.occupied_bb() ^ rook.bb()) & king_free_bb).is_zero()
                && ((self.occupied_bb() ^ king) & rook_free_bb).is_zero()
            {
                debug_assert_eq!(self.colored_piece_on(rook).symbol, ColoredPieceType::new(color, Rook));
                return true;
            }
        }
        false
    }

    fn gen_king_moves(&self, callback: &mut impl FnMut(Move), filter: Bitboard, only_captures: bool) {
        let filter = filter & !self.threats;
        let us = self.active;
        let king_square = self.king_sq(us);
        let mut attacks = Self::normal_king_attacks_from(king_square) & filter;
        while attacks.has_any() {
            let target = attacks.pop_lsb();
            callback(Move::new(king_square, Square::from_bb_idx(target), NormalMove));
        }
        if only_captures {
            return;
        }
        // Castling, handling the general (D)FRC case.
        if self.is_castling_pseudolegal(Queenside) {
            let rook = self.rook_start_square(us, Queenside);
            callback(Move::new(king_square, rook, CastleQueenside));
        }
        if self.is_castling_pseudolegal(Kingside) {
            let rook = self.rook_start_square(us, Kingside);
            callback(Move::new(king_square, rook, CastleKingside));
        }
    }

    fn gen_knight_moves(&self, callback: &mut impl FnMut(Move), filter: Bitboard) {
        let knights = self.col_piece_bb(self.active, Knight);
        for from in knights {
            let attacks = Self::knight_attacks_from(from) & filter;
            for to in attacks {
                callback(Move::new(from, to, NormalMove));
            }
        }
    }

    fn gen_slider_moves<const SLIDER: usize>(
        &self,
        callback: &mut impl FnMut(Move),
        filter: Bitboard,
        generator: &ChessSliderGenerator,
    ) {
        let piece = if SLIDER == Bishop as usize {
            Bishop
        } else if SLIDER == Rook as usize {
            Rook
        } else {
            debug_assert_eq!(SLIDER, Queen as usize);
            Queen
        };
        let color = self.active;
        let pieces = self.col_piece_bb(color, piece);
        for from in pieces {
            let attacks = match piece {
                Bishop => generator.bishop_attacks(from),
                Rook => generator.rook_attacks(from),
                _ => generator.queen_attacks(from),
            };
            let attacks = attacks & filter;
            for to in attacks {
                callback(Move::new(from, to, NormalMove));
            }
        }
    }

    // All the following methods can be called with squares that do not contain the specified piece.
    // This makes sense because it allows to find all pieces able to attack a given square.

    pub const fn normal_king_attacks_from(square: Square) -> Bitboard {
        KINGS[square.bb_idx()]
    }

    pub const fn knight_attacks_from(square: Square) -> Bitboard {
        KNIGHTS[square.bb_idx()]
    }

    pub const fn single_pawn_captures(color: Color, square: Square) -> Bitboard {
        PAWN_CAPTURES[color as usize][square.bb_idx()]
    }

    /// Returns a Bitboard of any slider in `self` that attacks `target` through `ray_square`, assuming `blockers`.
    /// This bitboard will always have either no or exactly one set bits.
    pub fn ray_attacks(&self, target: Square, ray_square: Square, blockers: Bitboard) -> Bitboard {
        let generator = ChessSliderGenerator::new(blockers);
        let file_diff = target.file().wrapping_sub(ray_square.file());
        let rank_diff = target.rank().wrapping_sub(ray_square.rank());
        if file_diff == 0 {
            generator.vertical_attacks(target) & (self.piece_bb(Rook) | self.piece_bb(Queen))
        } else if rank_diff == 0 {
            generator.horizontal_attacks(target) & (self.piece_bb(Rook) | self.piece_bb(Queen))
        } else if file_diff == rank_diff {
            generator.diagonal_attacks(target) & (self.piece_bb(Bishop) | self.piece_bb(Queen))
        } else if file_diff == 0_u8.wrapping_sub(rank_diff) {
            generator.anti_diagonal_attacks(target) & (self.piece_bb(Bishop) | self.piece_bb(Queen))
        } else {
            Bitboard::default()
        }
    }

    pub fn all_attacking(&self, square: Square, slider_gen: ChessSliderGenerator) -> Bitboard {
        let rook_sliders = self.piece_bb(Rook) | self.piece_bb(Queen);
        let bishop_sliders = self.piece_bb(Bishop) | self.piece_bb(Queen);
        rook_sliders & slider_gen.rook_attacks(square)
            | bishop_sliders & slider_gen.bishop_attacks(square)
            | (Self::knight_attacks_from(square) & self.piece_bb(Knight))
            | (Self::normal_king_attacks_from(square) & self.piece_bb(King))
            | Self::single_pawn_captures(Black, square) & self.col_piece_bb(White, Pawn)
            | Self::single_pawn_captures(White, square) & self.col_piece_bb(Black, Pawn)
    }

    pub fn checkers(&self) -> Bitboard {
        self.checkers
    }

    /// Calculate a bitboard of all squares that are attacked by the given player.
    /// This only counts hypothetical captures, so no pawn pushes or castling moves.
    pub(super) fn calc_threats_of(&self, player: Color) -> Bitboard {
        let mut res = Self::normal_king_attacks_from(self.king_sq(player));
        for knight in self.col_piece_bb(player, Knight) {
            res |= Self::knight_attacks_from(knight);
        }
        let bishop_sliders = self.piece_bb(Bishop) | self.piece_bb(Queen);
        let rook_sliders = self.piece_bb(Rook) | self.piece_bb(Queen);
        let us = self.player_bb(player);
        let empty = self.empty_bb();
        res |= all_rook_attacks(rook_sliders & us, empty);
        res |= all_bishop_attacks(bishop_sliders & us, empty);
        res |= self.col_piece_bb(player, Pawn).pawn_attacks(player);
        res
    }

    pub fn threats(&self) -> Bitboard {
        self.threats
    }

    // This doesn't calculate checks from a king because those can't happen in a legal position, but
    // this means that it can't be used to verify that a position is legal
    pub fn set_checkers_and_pinned(&mut self) {
        let us = self.active_player();
        let their_bb = self.player_bb(!us);
        let our_bb = self.player_bb(us);
        let occupied = our_bb | their_bb;
        let rook_sliders = self.piece_bb(Rook) | self.piece_bb(Queen);
        let bishop_sliders = self.piece_bb(Bishop) | self.piece_bb(Queen);
        let king = self.king_sq(us);
        self.checkers = ((Self::knight_attacks_from(king) & self.piece_bb(Knight))
            | Self::single_pawn_captures(us, king) & self.col_piece_bb(!us, Pawn))
            & their_bb;
        self.pinned = Bitboard::default();
        let mut update = |slider: Square| {
            let ray = Bitboard::ray_exclusive(slider, king, ChessboardSize::default());
            if !ray.intersects(occupied) {
                self.checkers |= slider.bb();
            } else if !ray.intersects(their_bb) && (ray & our_bb).is_single_piece() {
                self.pinned |= ray & our_bb;
            }
        };
        for slider in rook_sliders & their_bb & ROOKS[king] {
            update(slider);
        }
        for slider in bishop_sliders & their_bb & BISHOPS[king] {
            update(slider);
        }
        let king = self.king_sq(!us);
        let mut update_our_pinned = |slider: Square| {
            let ray = Bitboard::ray_exclusive(slider, king, ChessboardSize::default());
            if !ray.intersects(our_bb) && (ray & their_bb).is_single_piece() {
                self.pinned |= ray & their_bb;
            }
        };
        for slider in rook_sliders & our_bb & ROOKS[king] {
            update_our_pinned(slider);
        }
        for slider in bishop_sliders & our_bb & BISHOPS[king] {
            update_our_pinned(slider);
        }
    }

    pub(super) fn is_pseudolegal_legal_impl(&self, mov: Move) -> bool {
        let src = mov.src_square();
        let dest = mov.dest_square();
        let piece = mov.piece_type(self);
        if piece == King {
            if mov.is_castle() {
                let to_file = if mov.flags() == CastleKingside { G_FILE_NUM } else { C_FILE_NUM };
                let king_ray = Bitboard::ray_inclusive(
                    src,
                    Square::from_rank_file(src.rank(), to_file),
                    ChessboardSize::default(),
                );
                return (king_ray & self.threats).is_zero() && !self.pinned.has(dest);
            }
            debug_assert!(!self.threats.has(dest));
            if self.checkers().is_zero() {
                return true;
            }
            let slider_gen = ChessSliderGenerator::new(self.occupied_bb() ^ self.col_piece_bb(self.active, King));
            let rook_sliders = (self.piece_bb(Rook) | self.piece_bb(Queen)) & self.inactive_player_bb();
            let bishop_sliders = (self.piece_bb(Bishop) | self.piece_bb(Queen)) & self.inactive_player_bb();
            return ((rook_sliders & slider_gen.rook_attacks(dest))
                | (bishop_sliders & slider_gen.bishop_attacks(dest)))
            .is_zero();
        }
        let king_sq = self.king_sq(self.active);
        debug_assert!(!self.checkers().more_than_one_bit_set());
        // no need to special case ep: If the capturing pawn is pinned, either the ep square isn't set or it's the same logic
        // as with non-ep moves
        if self.checkers().has_any() {
            debug_assert!(self.checkers.is_single_piece());
            if self.pinned.has(src) {
                return false;
            }
            if mov.is_ep() {
                return self.checkers == self.ep_square.unwrap().pawn_advance_unchecked(!self.active).bb();
            }
            if cfg!(debug_assertions) {
                let checker = self.checkers.to_square().unwrap();
                let ray = Bitboard::ray_inclusive(checker, king_sq, ChessboardSize::default());
                debug_assert!(ray.has(dest));
            }
        } else if self.pinned.has(src) {
            return Bitboard::new(INFINITE_RAYS[src][king_sq]).has(dest);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::general::board::BoardHelpers;
    use crate::general::board::Strictness::Strict;
    use crate::general::moves::MoveTrait;
    use std::str::FromStr;

    #[test]
    fn attack_test() {
        for pos in Board::bench_positions() {
            for mov in pos.legal_moves_slow() {
                let child = pos.make_move(mov).unwrap();
                let slider_gen = child.slider_generator();
                let mut threats = Bitboard::default();
                for sq in Square::iter() {
                    let attacks = child.all_attacking(sq, slider_gen);
                    if attacks.intersects(child.inactive_player_bb()) {
                        threats |= sq.bb();
                    }
                }
                assert_eq!(threats, child.threats(), "{child} {:?}", threats ^ child.threats());
            }
        }
    }

    #[test]
    fn simple_is_move_pseudolegal_test() {
        let pos = Board::from_fen("3k4/1P6/8/8/7K/8/r7/2R5 w - - 0 1", Strict).unwrap();
        let mov = Move::new(Square::from_str("b7").unwrap(), Square::from_str("b8").unwrap(), NormalMove);
        assert!(!pos.is_move_pseudolegal(mov));
    }

    #[test]
    fn is_move_pseudolegal_test() {
        for p in Board::bench_positions() {
            let moves = p.pseudolegal_moves();
            for n in 0..u16::MAX {
                let m = Move::from_u64_unchecked(n as u64);
                let m = m.trust_unchecked();
                assert_eq!(moves.contains(&m), p.is_move_pseudolegal(m), "{p} {n:0x} {m:?}");
            }
            let Some(p) = p.make_nullmove() else { continue };
            let moves = p.pseudolegal_moves();
            for n in 0..u16::MAX {
                let m = Move::from_u64_unchecked(n as u64);
                let m = m.trust_unchecked();
                assert_eq!(moves.contains(&m), p.is_move_pseudolegal(m), "{p} {n:0x} {m:?}");
            }
        }
    }

    #[test]
    fn failed_proptest() {
        let pos = Board::from_fen("2kb1b2/pR2P1P1/P1N1P3/1p2Pp2/P5P1/1N6/4P2B/2qR2K1 w - f6 99 123", Strict).unwrap();
        let mov = Move::new(Square::from_str("e5").unwrap(), Square::from_str("f6").unwrap(), NormalMove);
        assert!(!pos.is_move_pseudolegal(mov));
        assert!(!pos.is_generated_move_pseudolegal(mov));
        let mov = Move::new(mov.src_square(), mov.dest_square(), EnPassant);
        assert!(pos.is_move_pseudolegal(mov));
        assert!(pos.is_generated_move_pseudolegal(mov));
    }
}
