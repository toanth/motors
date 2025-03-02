use crate::games::chess::castling::CastleRight;
use crate::games::chess::moves::ChessMoveFlags::*;
use crate::games::chess::moves::{ChessMove, ChessMoveFlags};
use crate::games::chess::pieces::ChessPieceType::*;
use crate::games::chess::pieces::{ChessPieceType, ColoredChessPieceType};
use crate::games::chess::squares::ChessSquare;
use crate::games::chess::CastleRight::*;
use crate::games::chess::ChessColor::*;
use crate::games::chess::{ChessBitboardTrait, ChessColor, Chessboard, PAWN_CAPTURES};
use crate::games::{Board, Color, ColoredPieceType};
use crate::general::bitboards::chessboard::{ChessBitboard, KINGS, KNIGHTS};
use crate::general::bitboards::{Bitboard, KnownSizeBitboard, RawBitboard};
use crate::general::board::BitboardBoard;
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

    /// Castling moves can be special: For example, it's possible that a normal king move is legal, but a
    /// chess960 castling move with the same source and dest square as the normal king move isn't, or the other way around.
    /// For pawns, there's a difference between attacks and pushes, and this function ignores pushes.
    pub fn attacks_no_castle_or_pawn_push(
        &self,
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

    pub fn is_move_pseudolegal_impl(&self, mov: ChessMove) -> bool {
        let Ok(flags) = mov.untrusted_flags() else {
            return false;
        };
        let piece = flags.piece_type();
        let src = mov.src_square();
        let color = self.active_player;
        if !self.colored_piece_bb(color, piece).is_bit_set_at(src.bb_idx()) {
            return false;
        }
        if mov.is_castle() {
            (self.rook_start_square(color, Kingside) == mov.dest_square()
                && mov.castle_side() == Kingside
                && self.is_castling_pseudolegal(Kingside))
                || (self.rook_start_square(color, Queenside) == mov.dest_square()
                    && mov.castle_side() == Queenside
                    && self.is_castling_pseudolegal(Queenside))
        } else if mov.piece_type() == Pawn {
            let mut incorrect = false;
            incorrect |= mov.is_ep() && self.ep_square() != Some(mov.dest_square());
            incorrect |= mov.is_promotion() && !mov.dest_square().is_backrank();
            let capturable = self.player_bb(color.other()) | self.ep_square.map(ChessSquare::bb).unwrap_or_default();
            !incorrect
                && Self::single_pawn_moves(color, src, capturable, self.empty_bb())
                    .is_bit_set_at(mov.dest_square().bb_idx())
        } else {
            let gen = self.slider_generator();
            (self.attacks_no_castle_or_pawn_push(src, mov.piece_type(), color, &gen) & !self.active_player_bb())
                .is_bit_set_at(mov.dest_square().bb_idx())
        }
    }

    /// Used for castling and to implement `is_in_check`:
    /// Pretend there is a king of color `us` at `square` and test if it is in check.
    pub fn is_in_check_on_square(&self, us: ChessColor, square: ChessSquare, gen: &ChessSliderGenerator) -> bool {
        (self.all_attacking(square, gen) & self.player_bb(us.other())).has_set_bit()
    }

    pub(super) fn gen_pseudolegal_moves<T: MoveList<Self>>(
        &self,
        moves: &mut T,
        filter: ChessBitboard,
        only_tactical: bool,
    ) {
        let slider_generator = self.slider_generator();
        self.gen_slider_moves::<T, { Bishop as usize }>(moves, filter, &slider_generator);
        self.gen_slider_moves::<T, { Rook as usize }>(moves, filter, &slider_generator);
        self.gen_slider_moves::<T, { Queen as usize }>(moves, filter, &slider_generator);
        // self.gen_slider_moves::<T, true>(moves, filter, &slider_generator);
        // self.gen_slider_moves::<T, false>(moves, filter, &slider_generator);
        self.gen_knight_moves(moves, filter);
        self.gen_king_moves(moves, filter, only_tactical);
        self.gen_pawn_moves(moves, only_tactical);
    }

    fn gen_pawn_moves<T: MoveList<Self>>(&self, moves: &mut T, only_tactical: bool) {
        let color = self.active_player;
        let pawns = self.colored_piece_bb(color, Pawn);
        let occupied = self.occupied_bb();
        let free = !occupied;
        let opponent = self.player_bb(color.other());
        let regular_pawn_moves;
        let double_pawn_moves;
        let left_pawn_captures;
        let right_pawn_captures;
        let capturable = opponent | self.ep_square.map(ChessSquare::bb).unwrap_or_default();
        if color == White {
            regular_pawn_moves = (pawns.north() & free, 8);
            double_pawn_moves = (((pawns & ChessBitboard::rank(1)) << 16) & free.north() & free, 16);
            right_pawn_captures = (pawns.north_east() & capturable, 9);
            left_pawn_captures = (pawns.north_west() & capturable, 7);
        } else {
            regular_pawn_moves = (pawns.south() & free, -8);
            double_pawn_moves = (((pawns & ChessBitboard::rank(6)) >> 16) & free.south() & free, -16);
            right_pawn_captures = (pawns.south_west() & capturable, -9);
            left_pawn_captures = (pawns.south_east() & capturable, -7);
        }
        for move_type in [right_pawn_captures, left_pawn_captures, regular_pawn_moves, double_pawn_moves] {
            let bb = move_type.0;
            for to in bb.ones() {
                let from = ChessSquare::from_bb_index((to.to_u8() as isize - move_type.1) as usize);
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
        let king = self.colored_piece_bb(color, King);
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
        let color = self.active_player;
        let king = self.colored_piece_bb(color, King);
        let king_square = ChessSquare::from_bb_index(king.num_trailing_zeros());
        let mut attacks = Self::normal_king_attacks_from(king_square) & filter;
        while attacks.has_set_bit() {
            let target = attacks.pop_lsb();
            moves.add_move(ChessMove::new(king_square, ChessSquare::from_bb_index(target), NormalKingMove));
        }
        if only_captures {
            return;
        }
        // Castling, handling the general (D)FRC case.
        if self.is_castling_pseudolegal(Queenside) {
            let rook = self.rook_start_square(color, Queenside);
            moves.add_move(ChessMove::new(king_square, rook, CastleQueenside));
        }
        if self.is_castling_pseudolegal(Kingside) {
            let rook = self.rook_start_square(color, Kingside);
            moves.add_move(ChessMove::new(king_square, rook, CastleKingside));
        }
    }

    fn gen_knight_moves<T: MoveList<Self>>(&self, moves: &mut T, filter: ChessBitboard) {
        let knights = self.colored_piece_bb(self.active_player, Knight);
        for from in knights.ones() {
            let attacks = Self::knight_attacks_from(from) & filter;
            for to in attacks.ones() {
                moves.add_move(ChessMove::new(from, to, KnightMove));
            }
        }
    }

    // fn gen_slider_moves<T: MoveList<Self>, const IS_BISHOP: bool>(
    //     &self,
    //     moves: &mut T,
    //     filter: ChessBitboard,
    //     gen: &ChessSliderGenerator,
    // ) {
    //     let color = self.active_player;
    //     let slider_type = if IS_BISHOP { Bishop } else { Rook };
    //     let non_queens = self.colored_piece_bb(color, slider_type);
    //     let queens = self.colored_piece_bb(color, Queen);
    //     let pieces = queens | non_queens;
    //     let blockers = self.occupied_bb();
    //     for from in pieces.ones() {
    //         let attacks = if IS_BISHOP { gen.bishop_attacks(from) & filter } else { gen.rook_attacks(from) & filter };
    //         for to in attacks.ones() {
    //             let move_type = if queens.is_bit_set_at(from.bb_idx()) {
    //                 QueenMove
    //             } else {
    //                 ChessMoveFlags::normal_move(slider_type)
    //             };
    //             moves.add_move(ChessMove::new(from, to, move_type));
    //         }
    //     }
    // }
    // TODO: This version should (test!) be faster, but makes a testcase fail, which is *probably* just due to random noise induced by
    // the different order in which moves are being generated. Investigate!
    fn gen_slider_moves<T: MoveList<Self>, const SLIDER: usize>(
        &self,
        moves: &mut T,
        filter: ChessBitboard,
        gen: &ChessSliderGenerator,
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
        let pieces = self.colored_piece_bb(color, piece);
        for from in pieces.ones() {
            let attacks = match piece {
                Bishop => gen.bishop_attacks(from),
                Rook => gen.rook_attacks(from),
                _ => gen.queen_attacks(from),
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

    pub fn normal_king_attacks_from(square: ChessSquare) -> ChessBitboard {
        KINGS[square.bb_idx()]
    }

    pub fn knight_attacks_from(square: ChessSquare) -> ChessBitboard {
        KNIGHTS[square.bb_idx()]
    }

    pub fn single_pawn_captures(color: ChessColor, square: ChessSquare) -> ChessBitboard {
        PAWN_CAPTURES[color as usize][square.bb_idx()]
    }

    pub fn all_attacking(&self, square: ChessSquare, slider_gen: &ChessSliderGenerator) -> ChessBitboard {
        let rook_sliders = self.piece_bb(Rook) | self.piece_bb(Queen);
        let bishop_sliders = self.piece_bb(Bishop) | self.piece_bb(Queen);
        rook_sliders & slider_gen.rook_attacks(square)
            | bishop_sliders & slider_gen.bishop_attacks(square)
            | (Self::knight_attacks_from(square) & self.piece_bb(Knight))
            | (Self::normal_king_attacks_from(square) & self.piece_bb(King))
            | Self::single_pawn_captures(Black, square) & self.colored_piece_bb(White, Pawn)
            | Self::single_pawn_captures(White, square) & self.colored_piece_bb(Black, Pawn)
    }

    // TODO: Use precomputed rays
    pub fn ray_attacks(&self, target: ChessSquare, ray_square: ChessSquare, blockers: ChessBitboard) -> ChessBitboard {
        let gen = ChessSliderGenerator::new(blockers);
        let file_diff = target.file().wrapping_sub(ray_square.file());
        let rank_diff = target.rank().wrapping_sub(ray_square.rank());
        if file_diff == 0 {
            gen.vertical_attacks(target) & (self.piece_bb(Rook) | self.piece_bb(Queen))
        } else if rank_diff == 0 {
            gen.horizontal_attacks(target) & (self.piece_bb(Rook) | self.piece_bb(Queen))
        } else if file_diff == rank_diff {
            gen.diagonal_attacks(target) & (self.piece_bb(Bishop) | self.piece_bb(Queen))
        } else if file_diff == 0_u8.wrapping_sub(rank_diff) {
            gen.anti_diagonal_attacks(target) & (self.piece_bb(Bishop) | self.piece_bb(Queen))
        } else {
            ChessBitboard::default()
        }
    }
}
