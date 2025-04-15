use crate::games::chess::CastleRight::*;
use crate::games::chess::ChessColor::*;
use crate::games::chess::castling::CastleRight;
use crate::games::chess::moves::ChessMoveFlags::*;
use crate::games::chess::moves::{ChessMove, ChessMoveFlags};
use crate::games::chess::pieces::ChessPieceType::*;
use crate::games::chess::pieces::{ChessPieceType, ColoredChessPieceType};
use crate::games::chess::squares::{C_FILE_NO, ChessSquare, ChessboardSize, G_FILE_NO};
use crate::games::chess::{ChessBitboardTrait, ChessColor, Chessboard, PAWN_CAPTURES};
use crate::games::{Board, Color, ColoredPieceType};
use crate::general::bitboards::chessboard::{ChessBitboard, KINGS, KNIGHTS};
use crate::general::bitboards::{Bitboard, KnownSizeBitboard, RawBitboard};
use crate::general::board::{BitboardBoard, BoardHelpers};
use crate::general::hq::ChessSliderGenerator;
use crate::general::move_list::MoveList;
use crate::general::squares::RectangularCoordinates;

impl Chessboard {
    pub fn slider_generator(&self) -> ChessSliderGenerator {
        ChessSliderGenerator::new(self.occupied_bb())
    }

    fn single_pawn_moves(
        color: ChessColor,
        square: ChessSquare,
        capture_filter: ChessBitboard,
        push_filter: ChessBitboard,
    ) -> ChessBitboard {
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
        square: ChessSquare,
        piece: ChessPieceType,
        color: ChessColor,
        slider_generator: &ChessSliderGenerator,
    ) -> ChessBitboard {
        match piece {
            Pawn => Self::single_pawn_captures(color, square),
            Knight => Self::knight_attacks_from(square),
            Bishop => slider_generator.bishop_attacks(square),
            Rook => slider_generator.rook_attacks(square),
            Queen => slider_generator.queen_attacks(square),
            King => Self::normal_king_attacks_from(square),
            Empty => ChessBitboard::default(),
        }
    }

    fn check_castling_move_pseudolegal(&self, mov: ChessMove, color: ChessColor) -> bool {
        self.king_square(color) == mov.src_square()
            && (self.rook_start_square(color, Kingside) == mov.dest_square()
                && mov.castle_side() == Kingside
                && self.is_castling_pseudolegal(Kingside))
            || (self.rook_start_square(color, Queenside) == mov.dest_square()
                && mov.castle_side() == Queenside
                && self.is_castling_pseudolegal(Queenside))
    }

    fn simple_illegal(&self, piece: ChessPieceType, dest: ChessSquare) -> bool {
        if piece == King {
            if self.threats().is_bit_set(dest) {
                return true;
            }
        } else if self.checkers.more_than_one_bit_set() {
            return true;
        }
        false
    }

    pub fn is_move_pseudolegal_impl(&self, mov: ChessMove) -> bool {
        let Ok(flags) = mov.untrusted_flags() else {
            return false;
        };
        let piece = flags.piece_type();
        let src = mov.src_square();
        let color = self.active_player;
        if !self.col_piece_bb(color, piece).is_bit_set(src) {
            return false;
        }
        if mov.is_castle() {
            self.check_castling_move_pseudolegal(mov, color)
        } else if piece == Pawn {
            let mut incorrect = false;
            incorrect |= mov.is_ep() && self.ep_square() != Some(mov.dest_square());
            incorrect |= mov.is_promotion() && !mov.dest_square().is_backrank();
            if mov.is_ep() {
                return Some(mov.dest_square()) == self.ep_square;
            }
            let capturable = self.player_bb(color.other());
            !incorrect && Self::single_pawn_moves(color, src, capturable, self.empty_bb()).is_bit_set(mov.dest_square())
        } else {
            if self.simple_illegal(piece, mov.dest_square()) {
                return false;
            }
            let generator = self.slider_generator();
            (Self::threatening_attacks(src, mov.piece_type(), color, &generator) & !self.active_player_bb())
                .is_bit_set(mov.dest_square())
        }
    }

    /// Unlike [`Self::is_move_pseudolegal`], this assumes that `mov` used to be pseudolegal in *some* arbitrary position.
    /// This means that checking pseudolegality is less expensive
    pub fn is_generated_move_pseudolegal_impl(&self, mov: ChessMove) -> bool {
        let us = self.active_player;
        let src = mov.src_square();
        let piece = mov.flags().piece_type();
        if !self.col_piece_bb(us, piece).is_bit_set(src) {
            // this check is still necessary because otherwise we could e.g. accept a move with piece 'bishop' from a queen.
            return false;
        }
        if mov.is_castle() {
            // we can't assume that the position was from the same game, so we still have to check that rook and king positions match
            self.check_castling_move_pseudolegal(mov, us)
        } else if piece == Pawn {
            if mov.is_ep() {
                return self.ep_square() == Some(mov.dest_square());
            }
            let capturable = self.player_bb(!us);
            // we still need to check this because this could have been a pawn move from the other player
            Self::single_pawn_moves(us, src, capturable, self.empty_bb()).is_bit_set(mov.dest_square())
        } else {
            if self.simple_illegal(piece, mov.dest_square()) {
                return false;
            }
            let ray = ChessBitboard::ray_exclusive(src, mov.dest_square(), ChessboardSize {});
            let on_ray = ray & self.occupied_bb();
            on_ray.is_zero() && !self.player_bb(us).is_bit_set(mov.dest_square())
        }
    }

    /// Used for checking castling legality and verifying FENs:
    /// Pretend there is a king of color `us` at `square` and test if it is in check.
    pub fn is_in_check_on_square(&self, us: ChessColor, square: ChessSquare, generator: &ChessSliderGenerator) -> bool {
        (self.all_attacking(square, generator) & self.player_bb(us.other())).has_set_bit()
    }

    pub(super) fn gen_pseudolegal_moves<T: MoveList<Self>>(
        &self,
        moves: &mut T,
        mut filter: ChessBitboard,
        only_tactical: bool,
    ) {
        let slider_generator = self.slider_generator();
        self.gen_king_moves(moves, filter, only_tactical);
        // in a double check, only generate king moves. We support loading FENs with more than 2 checkers.
        if self.checkers.more_than_one_bit_set() {
            return;
        }
        let mut check_ray = !ChessBitboard::default();
        if self.checkers.has_set_bit() {
            let checker = ChessSquare::from_bb_idx(self.checkers().pop_lsb());
            check_ray =
                ChessBitboard::ray_inclusive(self.king_square(self.active_player), checker, ChessboardSize::default());
            filter &= check_ray;
        }
        self.gen_slider_moves::<T, { Bishop as usize }>(moves, filter, &slider_generator);
        self.gen_slider_moves::<T, { Rook as usize }>(moves, filter, &slider_generator);
        self.gen_slider_moves::<T, { Queen as usize }>(moves, filter, &slider_generator);
        self.gen_knight_moves(moves, filter);
        self.gen_pawn_moves(moves, check_ray, only_tactical);

        if cfg!(debug_assertions) {
            for &m in moves.iter_moves() {
                debug_assert!(self.is_generated_move_pseudolegal(m));
            }
        }
    }

    fn gen_pawn_moves<T: MoveList<Self>>(&self, moves: &mut T, filter: ChessBitboard, only_tactical: bool) {
        let color = self.active_player;
        let pawns = self.col_piece_bb(color, Pawn);
        let free = !self.occupied_bb();
        let free_filter = free & filter;
        let opponent = self.player_bb(color.other()) & filter;
        let regular_pawn_moves;
        let double_pawn_moves;
        let left_pawn_captures;
        let right_pawn_captures;
        let capturable = opponent | self.ep_square.map(ChessSquare::bb).unwrap_or_default();
        if color == White {
            regular_pawn_moves = (pawns.north() & free_filter, 8);
            double_pawn_moves = (((pawns & ChessBitboard::rank(1)) << 16) & free.north() & free_filter, 16);
            right_pawn_captures = (pawns.north_east() & capturable, 9);
            left_pawn_captures = (pawns.north_west() & capturable, 7);
        } else {
            regular_pawn_moves = (pawns.south() & free_filter, -8);
            double_pawn_moves = (((pawns & ChessBitboard::rank(6)) >> 16) & free.south() & free_filter, -16);
            right_pawn_captures = (pawns.south_west() & capturable, -9);
            left_pawn_captures = (pawns.south_east() & capturable, -7);
        }
        for move_type in [right_pawn_captures, left_pawn_captures, regular_pawn_moves, double_pawn_moves] {
            let bb = move_type.0;
            for to in bb.ones() {
                let from = ChessSquare::from_bb_idx((to.to_u8() as isize - move_type.1) as usize);
                let is_capture = from.file() != to.file();
                let mut flag = NormalPawnMove;
                if self.ep_square.is_some_and(|sq| sq == to) {
                    flag = EnPassant;
                } else if to.is_backrank() {
                    for flag in [PromoQueen, PromoKnight] {
                        moves.add_move(ChessMove::new(from, to, flag));
                    }
                    if !only_tactical {
                        for flag in [PromoRook, PromoBishop] {
                            moves.add_move(ChessMove::new(from, to, flag));
                        }
                    }
                    continue;
                } else if only_tactical && !is_capture {
                    continue;
                }
                moves.add_move(ChessMove::new(from, to, flag));
            }
        }
    }

    fn is_castling_pseudolegal(&self, side: CastleRight) -> bool {
        let color = self.active_player;
        let king_square = self.king_square(color);
        let king = self.col_piece_bb(color, King);
        // Castling, handling the general (D)FRC case.
        let king_file = king_square.file() as usize;
        const KING_QUEENSIDE_BB: [ChessBitboard; 8] = [
            ChessBitboard::new(!0), // impossible
            ChessBitboard::new(0b0000_0100),
            ChessBitboard::new(0b0000_0000), // no square to check
            ChessBitboard::new(0b0000_0100),
            ChessBitboard::new(0b0000_1100),
            ChessBitboard::new(0b0001_1100),
            ChessBitboard::new(0b0011_1100),
            ChessBitboard::new(!0), // impossible
        ];
        const KING_KINGSIDE_BB: [ChessBitboard; 8] = [
            ChessBitboard::new(!0), // impossible
            ChessBitboard::new(0b0111_1100),
            ChessBitboard::new(0b0111_1000),
            ChessBitboard::new(0b0111_0000),
            ChessBitboard::new(0b0110_0000),
            ChessBitboard::new(0b0100_0000),
            ChessBitboard::new(0b0000_0000),
            ChessBitboard::new(!0), // impossible
        ];
        const ROOK_QUEENSIDE_BB: [ChessBitboard; 8] = [
            ChessBitboard::new(0b0000_1110),
            ChessBitboard::new(0b0000_1100),
            ChessBitboard::new(0b0000_1000),
            ChessBitboard::new(0b0000_0000),
            ChessBitboard::new(0b0000_1000),
            ChessBitboard::new(0b0001_1000),
            ChessBitboard::new(!0), // impossible
            ChessBitboard::new(!0), // impossible
        ];
        const ROOK_KINGSIDE_BB: [ChessBitboard; 8] = [
            ChessBitboard::new(!0), // impossible
            ChessBitboard::new(!0), // impossible
            ChessBitboard::new(0b0011_1000),
            ChessBitboard::new(0b0011_0000),
            ChessBitboard::new(0b0010_0000),
            ChessBitboard::new(0b0000_0000),
            ChessBitboard::new(0b0010_0000),
            ChessBitboard::new(0b0110_0000),
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
                debug_assert_eq!(self.colored_piece_on(rook).symbol, ColoredChessPieceType::new(color, Rook));
                return true;
            }
        }
        false
    }

    fn gen_king_moves<T: MoveList<Self>>(&self, moves: &mut T, filter: ChessBitboard, only_captures: bool) {
        let filter = filter & !self.threats;
        let us = self.active_player;
        let king_square = self.king_square(us);
        let mut attacks = Self::normal_king_attacks_from(king_square) & filter;
        while attacks.has_set_bit() {
            let target = attacks.pop_lsb();
            moves.add_move(ChessMove::new(king_square, ChessSquare::from_bb_idx(target), NormalKingMove));
        }
        if only_captures {
            return;
        }
        // Castling, handling the general (D)FRC case.
        if self.is_castling_pseudolegal(Queenside) {
            let rook = self.rook_start_square(us, Queenside);
            moves.add_move(ChessMove::new(king_square, rook, CastleQueenside));
        }
        if self.is_castling_pseudolegal(Kingside) {
            let rook = self.rook_start_square(us, Kingside);
            moves.add_move(ChessMove::new(king_square, rook, CastleKingside));
        }
    }

    fn gen_knight_moves<T: MoveList<Self>>(&self, moves: &mut T, filter: ChessBitboard) {
        let knights = self.col_piece_bb(self.active_player, Knight);
        for from in knights.ones() {
            let attacks = Self::knight_attacks_from(from) & filter;
            for to in attacks.ones() {
                moves.add_move(ChessMove::new(from, to, KnightMove));
            }
        }
    }

    fn gen_slider_moves<T: MoveList<Self>, const SLIDER: usize>(
        &self,
        moves: &mut T,
        filter: ChessBitboard,
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
        let color = self.active_player;
        let pieces = self.col_piece_bb(color, piece);
        for from in pieces.ones() {
            let attacks = match piece {
                Bishop => generator.bishop_attacks(from),
                Rook => generator.rook_attacks(from),
                _ => generator.queen_attacks(from),
            };
            let attacks = attacks & filter;
            for to in attacks.ones() {
                let move_type = ChessMoveFlags::normal_move(piece);
                moves.add_move(ChessMove::new(from, to, move_type));
            }
        }
    }

    // All the following methods can be called with squares that do not contain the specified piece.
    // This makes sense because it allows to find all pieces able to attack a given square.

    pub const fn normal_king_attacks_from(square: ChessSquare) -> ChessBitboard {
        KINGS[square.bb_idx()]
    }

    pub const fn knight_attacks_from(square: ChessSquare) -> ChessBitboard {
        KNIGHTS[square.bb_idx()]
    }

    pub const fn single_pawn_captures(color: ChessColor, square: ChessSquare) -> ChessBitboard {
        PAWN_CAPTURES[color as usize][square.bb_idx()]
    }

    // TODO: Use precomputed rays
    pub fn ray_attacks(&self, target: ChessSquare, ray_square: ChessSquare, blockers: ChessBitboard) -> ChessBitboard {
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
            ChessBitboard::default()
        }
    }

    pub fn all_attacking(&self, square: ChessSquare, slider_gen: &ChessSliderGenerator) -> ChessBitboard {
        let rook_sliders = self.piece_bb(Rook) | self.piece_bb(Queen);
        let bishop_sliders = self.piece_bb(Bishop) | self.piece_bb(Queen);
        rook_sliders & slider_gen.rook_attacks(square)
            | bishop_sliders & slider_gen.bishop_attacks(square)
            | (Self::knight_attacks_from(square) & self.piece_bb(Knight))
            | (Self::normal_king_attacks_from(square) & self.piece_bb(King))
            | Self::single_pawn_captures(Black, square) & self.col_piece_bb(White, Pawn)
            | Self::single_pawn_captures(White, square) & self.col_piece_bb(Black, Pawn)
    }

    pub fn checkers(&self) -> ChessBitboard {
        self.checkers
    }

    /// Calculate a bitboard of all squares that are attacked by the given player.
    /// This only counts hypothetical captures, so no pawn pushes or castling moves.
    pub(super) fn calc_threats_of(&self, player: ChessColor, slider_gen: &ChessSliderGenerator) -> ChessBitboard {
        let mut res = Self::normal_king_attacks_from(self.king_square(player));
        for knight in self.col_piece_bb(player, Knight).ones() {
            res |= Self::knight_attacks_from(knight);
        }
        let bishop_sliders = self.piece_bb(Bishop) | self.piece_bb(Queen);
        let rook_sliders = self.piece_bb(Rook) | self.piece_bb(Queen);
        let us = self.player_bb(player);
        res |= slider_gen.all_bishop_attacks(bishop_sliders & us);
        res |= slider_gen.all_rook_attacks(rook_sliders & us);
        res |= self.col_piece_bb(player, Pawn).pawn_attacks(player);
        res
    }

    pub fn threats(&self) -> ChessBitboard {
        self.threats
    }

    // This doesn't calculate checks from a king because those can't happen in a legal position, but
    // this means that it can't be used to verify that a position is legal
    pub fn set_checkers_and_pinned(&mut self) {
        let us = self.active_player();
        let our_bb = self.player_bb(us);
        let their_bb = self.player_bb(!us);
        let king = self.king_square(us);
        self.pinned = ChessBitboard::default();
        self.checkers = ((Self::knight_attacks_from(king) & self.piece_bb(Knight))
            | Self::single_pawn_captures(us, king) & self.col_piece_bb(!us, Pawn))
            & their_bb;
        let slider_gen = ChessSliderGenerator::new(their_bb);
        let rook_sliders = (self.piece_bb(Rook) | self.piece_bb(Queen)) & their_bb;
        let bishop_sliders = (self.piece_bb(Bishop) | self.piece_bb(Queen)) & their_bb;

        let mut update = |slider: ChessSquare| {
            let on_ray = ChessBitboard::ray_exclusive(slider, king, ChessboardSize::default()) & our_bb;
            if on_ray.is_zero() {
                self.checkers |= slider.bb();
            } else if on_ray.is_single_piece() {
                self.pinned |= on_ray;
            }
        };

        for slider in (rook_sliders & slider_gen.rook_attacks(king)).ones() {
            update(slider);
        }
        for slider in (bishop_sliders & slider_gen.bishop_attacks(king)).ones() {
            update(slider);
        }
    }

    pub(super) fn is_pseudolegal_legal_impl(&self, mov: ChessMove) -> bool {
        let src = mov.src_square();
        let dest = mov.dest_square();
        if mov.is_castle() {
            let to_file = if mov.flags() == CastleKingside { G_FILE_NO } else { C_FILE_NO };
            let king_ray = ChessBitboard::ray_inclusive(
                mov.src_square(),
                ChessSquare::from_rank_file(src.rank(), to_file),
                ChessboardSize::default(),
            );
            (king_ray & self.threats).is_zero() && !self.pinned.is_bit_set(dest)
        } else if mov.flags() == NormalKingMove {
            debug_assert!(!self.threats.is_bit_set(dest));
            if self.checkers().is_zero() {
                return true;
            }
            let slider_gen =
                ChessSliderGenerator::new(self.occupied_bb() ^ self.col_piece_bb(self.active_player, King));
            let rook_sliders = (self.piece_bb(Rook) | self.piece_bb(Queen)) & self.inactive_player_bb();
            let bishop_sliders = (self.piece_bb(Bishop) | self.piece_bb(Queen)) & self.inactive_player_bb();
            ((rook_sliders & slider_gen.rook_attacks(dest)) | (bishop_sliders & slider_gen.bishop_attacks(dest)))
                .is_zero()
        } else {
            let king_sq = self.king_square(self.active_player);
            debug_assert!(!self.checkers().more_than_one_bit_set());
            if mov.is_ep() {
                let mut b = *self;
                b.remove_piece_unchecked(mov.square_of_pawn_taken_by_ep().unwrap(), Pawn, self.inactive_player());
                b.move_piece_no_mailbox(src, dest, Pawn);
                // no need to update the mailbox
                return !b.is_in_check_on_square(b.active_player(), king_sq, &b.slider_generator());
            } else if self.checkers().has_set_bit() {
                if self.pinned.is_bit_set(src) {
                    return false;
                }
                let checker = ChessSquare::from_bb_idx(self.checkers().pop_lsb());
                let ray = ChessBitboard::ray_inclusive(checker, king_sq, ChessboardSize::default());
                return ray.is_bit_set(mov.dest_square());
            } else if self.pinned.is_bit_set(src) {
                let mut pinning = self.ray_attacks(src, king_sq, self.occupied_bb());
                debug_assert!(pinning.is_single_piece());
                let pinning = ChessSquare::from_bb_idx(pinning.pop_lsb());
                let pin_ray = ChessBitboard::ray_inclusive(pinning, king_sq, ChessboardSize::default());
                return pin_ray.is_bit_set(mov.dest_square());
            }
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::general::board::Strictness::Strict;
    use std::str::FromStr;

    #[test]
    fn attack_test() {
        for pos in Chessboard::bench_positions() {
            for mov in pos.legal_moves_slow() {
                let child = pos.make_move(mov).unwrap();
                let slider_gen = child.slider_generator();
                let mut threats = ChessBitboard::default();
                for sq in ChessSquare::iter() {
                    let attacks = child.all_attacking(sq, &slider_gen);
                    if (attacks & child.inactive_player_bb()).has_set_bit() {
                        threats |= sq.bb();
                    }
                }
                assert_eq!(threats, child.threats(), "{child} {:?}", threats ^ child.threats());
            }
        }
    }
    #[test]
    fn failed_proptest() {
        let pos =
            Chessboard::from_fen("2kb1b2/pR2P1P1/P1N1P3/1p2Pp2/P5P1/1N6/4P2B/2qR2K1 w - f6 99 123", Strict).unwrap();
        let mov =
            ChessMove::new(ChessSquare::from_str("e5").unwrap(), ChessSquare::from_str("f6").unwrap(), NormalPawnMove);
        assert!(!pos.is_move_pseudolegal(mov));
        assert!(!pos.is_generated_move_pseudolegal(mov));
        let mov = ChessMove::new(mov.src_square(), mov.dest_square(), EnPassant);
        assert!(pos.is_move_pseudolegal(mov));
        assert!(pos.is_generated_move_pseudolegal(mov));
    }
}
