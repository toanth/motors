use std::iter::Peekable;

use derive_more::{Add, AddAssign, Sub, SubAssign};
use itertools::Itertools;
use strum::IntoEnumIterator;

use crate::games::chess::castling::CastleRight;
use crate::games::chess::moves::ChessMove;
use crate::games::chess::moves::ChessMoveFlags::*;
use crate::games::chess::pieces::UncoloredChessPiece::*;
use crate::games::chess::pieces::{
    ColoredChessPiece, UncoloredChessPiece, UncoloredChessPieceIter, NUM_CHESS_PIECES,
};
use crate::games::chess::squares::{
    ChessSquare, A_FILE_NO, B_FILE_NO, C_FILE_NO, E_FILE_NO, G_FILE_NO, H_FILE_NO, NUM_COLUMNS,
};
use crate::games::chess::CastleRight::*;
use crate::games::chess::CastleRight::*;
use crate::games::chess::{ChessMoveList, Chessboard};
use crate::games::Color::*;
use crate::games::{
    sup_distance, AbstractPieceType, Board, Color, ColoredPiece, ColoredPieceType, Coordinates,
    DimT, Move,
};
use crate::general::bitboards::chess::{ChessBitboard, KINGS, KNIGHTS};
use crate::general::bitboards::RayDirections::{AntiDiagonal, Diagonal, Horizontal, Vertical};
use crate::general::bitboards::{Bitboard, Direction, RawBitboard, RawStandardBitboard};

#[derive(Debug, Copy, Clone)]
enum SliderMove {
    Bishop,
    Rook,
}

// TODO: Use the north(), west(), etc. methods

impl Chessboard {
    // TODO: More efficient impl for sliders
    pub fn is_move_pseudolegal_impl(&self, mov: ChessMove) -> bool {
        let piece = mov.piece(self);
        if piece.is_empty() || piece.color().unwrap() != self.active_player {
            return false;
        }
        let mut list = ChessMoveList::default();
        let filter = !self.colored_bb(self.active_player);
        match piece.uncolored() {
            Pawn => self.gen_pawn_moves(&mut list, false),
            Knight => {
                return Self::knight_moves_from_square(mov.src_square(), filter)
                    .is_bit_set_at(mov.dest_square().index());
            }
            Bishop => self.gen_slider_moves(SliderMove::Bishop, &mut list, filter),
            Rook => self.gen_slider_moves(SliderMove::Rook, &mut list, filter),
            Queen => {
                self.gen_slider_moves(SliderMove::Rook, &mut list, filter);
                self.gen_slider_moves(SliderMove::Bishop, &mut list, filter);
            }
            King => self.gen_king_moves(&mut list, filter, false),
            Empty => panic!(),
        }
        list.contains(&mov)
    }

    /// used for castling and to implement `is_in_check`:
    /// Pretend there is a king of color `us` at `square` and test if it is in check.
    pub fn is_in_check_on_square(&self, us: Color, square: ChessSquare) -> bool {
        let them = us.other();
        let idx = square.index();
        let attacks = KNIGHTS[square.index()];
        if (attacks & self.colored_piece_bb(them, Knight)).has_set_bit() {
            return true;
        }
        let bb = ChessBitboard::single_piece(idx);
        let blockers = self.occupied_bb() & !bb;
        let attacks = ChessBitboard::bishop_attacks(square, blockers);
        if (attacks & (self.colored_piece_bb(them, Bishop) | self.colored_piece_bb(them, Queen)))
            .has_set_bit()
        {
            return true;
        }
        let attacks = ChessBitboard::rook_attacks(square, blockers);
        if (attacks & (self.colored_piece_bb(them, Rook) | self.colored_piece_bb(them, Queen)))
            .has_set_bit()
        {
            return true;
        }
        let their_pawns = self.colored_piece_bb(them, Pawn);
        let pawn_attacks = match us {
            White => {
                ((their_pawns & !ChessBitboard::file_no(A_FILE_NO)) >> 9)
                    | ((their_pawns & !ChessBitboard::file_no(H_FILE_NO)) >> 7)
            }
            Black => {
                ((their_pawns & !ChessBitboard::file_no(H_FILE_NO)) << 9)
                    | ((their_pawns & !ChessBitboard::file_no(A_FILE_NO)) << 7)
            }
        };
        if pawn_attacks.is_bit_set_at(idx) {
            return true;
        }
        // this can't happen in a legal position, but it can happen in pseudolegal movegen
        (KINGS[square.index()] & self.colored_piece_bb(them, King)).has_set_bit()
    }

    pub fn gen_all_pseudolegal_moves(&self) -> ChessMoveList {
        self.gen_pseudolegal_moves(!self.colored_bb(self.active_player), false)
    }

    pub fn gen_tactical_pseudolegal(&self) -> ChessMoveList {
        self.gen_pseudolegal_moves(self.colored_bb(self.active_player.other()), true)
    }

    fn gen_pseudolegal_moves(&self, filter: ChessBitboard, only_tactical: bool) -> ChessMoveList {
        let mut list = ChessMoveList::default();
        self.gen_slider_moves(SliderMove::Bishop, &mut list, filter);
        self.gen_slider_moves(SliderMove::Rook, &mut list, filter);
        self.gen_knight_moves(&mut list, filter);
        self.gen_king_moves(&mut list, filter, only_tactical);
        self.gen_pawn_moves(&mut list, only_tactical);
        list
    }

    fn gen_pawn_moves(&self, list: &mut ChessMoveList, only_tactical: bool) {
        let color = self.active_player;
        let pawns = self.colored_piece_bb(color, Pawn);
        let occupied = self.occupied_bb();
        let free = !occupied;
        let opponent = self.colored_bb(color.other());
        let regular_pawn_moves;
        let double_pawn_moves;
        let capturable = opponent
            | self
                .ep_square
                .map(|s| ChessBitboard::single_piece(s.index()))
                .unwrap_or_default();
        let [left_pawn_captures, right_pawn_captures] =
            Self::pawn_captures(color, pawns, capturable);
        if color == White {
            regular_pawn_moves = ((pawns << 8) & free, 8);
            double_pawn_moves = (
                ((pawns & ChessBitboard::rank_no(1)) << 16) & (free << 8) & free,
                16,
            );
        } else {
            regular_pawn_moves = ((pawns >> 8) & free, -8);
            double_pawn_moves = (
                ((pawns & ChessBitboard::rank_no(6)) >> 16) & (free >> 8) & free,
                -16,
            );
        }
        for move_type in [
            right_pawn_captures,
            left_pawn_captures,
            regular_pawn_moves,
            double_pawn_moves,
        ] {
            let mut bb = move_type.0;
            while bb.has_set_bit() {
                let idx = bb.pop_lsb();
                let from = ChessSquare::new((idx as isize - move_type.1) as usize);
                let to = ChessSquare::new(idx);
                let is_capture = from.file() != to.file();
                let mut flag = Normal;
                if to == self.ep_square.unwrap_or(ChessSquare::no_coordinates()) {
                    flag = EnPassant;
                } else if to.rank() == 0 || to.rank() == 7 {
                    for flag in [PromoQueen, PromoKnight] {
                        list.add_move(ChessMove::new(from, to, flag));
                    }
                    if !only_tactical {
                        for flag in [PromoRook, PromoBishop] {
                            list.add_move(ChessMove::new(from, to, flag));
                        }
                    }
                    continue;
                } else if only_tactical && !is_capture {
                    continue;
                }
                list.add_move(ChessMove::new(from, to, flag));
            }
        }
    }

    fn gen_king_moves(&self, list: &mut ChessMoveList, filter: ChessBitboard, only_captures: bool) {
        let color = self.active_player;
        let king = self.colored_piece_bb(color, King);
        let king_square = ChessSquare::new(king.trailing_zeros());
        let mut moves = Self::normal_king_moves_from_square(king_square, filter);
        while moves.has_set_bit() {
            let target = moves.pop_lsb();
            list.add_move(ChessMove::new(
                king_square,
                ChessSquare::new(target),
                Normal,
            ));
        }
        if only_captures {
            return;
        }
        // Castling, handling the general (D)FRC case.
        let king_file = king_square.file() as usize;
        const KING_QUEENSIDE_BB: [ChessBitboard; 8] = [
            ChessBitboard::from_u64(!0), // impossible
            ChessBitboard::from_u64(0b0000_0100),
            ChessBitboard::from_u64(0b0000_0000), // no square to check
            ChessBitboard::from_u64(0b0000_0100),
            ChessBitboard::from_u64(0b0000_1100),
            ChessBitboard::from_u64(0b0001_1100),
            ChessBitboard::from_u64(0b0011_1100),
            ChessBitboard::from_u64(!0), // impossible
        ];
        const KING_KINGSIDE_BB: [ChessBitboard; 8] = [
            ChessBitboard::from_u64(!0), // impossible
            ChessBitboard::from_u64(0b0111_1100),
            ChessBitboard::from_u64(0b0111_1000),
            ChessBitboard::from_u64(0b0111_0000),
            ChessBitboard::from_u64(0b0110_0000),
            ChessBitboard::from_u64(0b0100_0000),
            ChessBitboard::from_u64(0b0000_0000),
            ChessBitboard::from_u64(!0), // impossible
        ];
        const ROOK_QUEENSIDE_BB: [ChessBitboard; 8] = [
            ChessBitboard::new(RawStandardBitboard(0b0000_1110)),
            ChessBitboard::new(RawStandardBitboard(0b0000_1100)),
            ChessBitboard::new(RawStandardBitboard(0b0000_1000)),
            ChessBitboard::new(RawStandardBitboard(0b0000_0000)),
            ChessBitboard::new(RawStandardBitboard(0b0000_1000)),
            ChessBitboard::new(RawStandardBitboard(0b0001_1000)),
            ChessBitboard::from_u64(!0), // impossible
            ChessBitboard::from_u64(!0), // impossible
        ];
        const ROOK_KINGSIDE_BB: [ChessBitboard; 8] = [
            ChessBitboard::from_u64(!0), // impossible
            ChessBitboard::from_u64(!0), // impossible
            ChessBitboard::new(RawStandardBitboard(0b0011_1000)),
            ChessBitboard::new(RawStandardBitboard(0b0011_0000)),
            ChessBitboard::new(RawStandardBitboard(0b0010_0000)),
            ChessBitboard::new(RawStandardBitboard(0b0000_0000)),
            ChessBitboard::new(RawStandardBitboard(0b0010_0000)),
            ChessBitboard::new(RawStandardBitboard(0b0110_0000)),
        ];
        if self.castling.can_castle(color, Queenside) {
            let rook = self.rook_start_square(color, Queenside);
            let queenside_rook_bb = rook.bb();
            let rook_free_bb = ROOK_QUEENSIDE_BB
                [self.castling.rook_start_file(color, Queenside) as usize]
                << (color as usize * 7 * 8);
            let king_free_bb = KING_QUEENSIDE_BB[king_file] << (color as usize * 7 * 8);
            if ((self.occupied_bb() ^ queenside_rook_bb) & king_free_bb).is_zero()
                && ((self.occupied_bb() ^ king) & rook_free_bb).is_zero()
            {
                debug_assert_eq!(
                    self.piece_on(rook).symbol,
                    ColoredChessPiece::new(color, Rook)
                );
                list.add_move(ChessMove::new(king_square, rook, CastleQueenside));
            }
        }
        if self.castling.can_castle(color, Kingside) {
            let rook = self.rook_start_square(color, Kingside);
            let kingside_rook_bb = rook.bb();
            let rook_free_bb = ROOK_KINGSIDE_BB
                [self.castling.rook_start_file(color, Kingside) as usize]
                << (color as usize * 7 * 8);
            let king_free_bb = KING_KINGSIDE_BB[king_file] << (color as usize * 7 * 8);
            if ((self.occupied_bb() ^ kingside_rook_bb) & king_free_bb).is_zero()
                && ((self.occupied_bb() ^ king) & rook_free_bb).is_zero()
            {
                debug_assert_eq!(
                    self.piece_on(rook).symbol,
                    ColoredChessPiece::new(color, Rook)
                );
                list.add_move(ChessMove::new(king_square, rook, CastleKingside));
            }
        }
    }

    fn gen_knight_moves(&self, list: &mut ChessMoveList, filter: ChessBitboard) {
        let mut knights = self.colored_piece_bb(self.active_player, Knight);
        while knights.has_set_bit() {
            let square_idx = knights.pop_lsb();
            let from = ChessSquare::new(square_idx);
            let mut attacks = Self::knight_moves_from_square(from, filter);
            while attacks.has_set_bit() {
                let to = ChessSquare::new(attacks.pop_lsb());
                list.add_move(ChessMove::new(from, to, Normal));
            }
        }
    }

    fn gen_slider_moves(
        &self,
        slider_move: SliderMove,
        list: &mut ChessMoveList,
        filter: ChessBitboard,
    ) {
        let color = self.active_player;
        let non_queens = self.colored_piece_bb(
            color,
            match slider_move {
                SliderMove::Bishop => Bishop,
                SliderMove::Rook => Rook,
            },
        );
        let queens = self.colored_piece_bb(color, Queen);
        let mut pieces = non_queens | queens;
        while pieces.has_set_bit() {
            let idx = pieces.pop_lsb();
            let from = ChessSquare::new(idx);
            let mut attacks = self.gen_sliders_from_square(
                from,
                slider_move,
                filter,
                ChessBitboard::single_piece(idx),
            );
            while attacks.has_set_bit() {
                let to = ChessSquare::new(attacks.pop_lsb());
                list.add_move(ChessMove::new(from, to, Normal));
            }
        }
    }

    /// All `*_from_square` methods and `pawn_captures` can be called with squares that do not contain the specified piece.
    /// This makes sense because it allows to find all pieces able to attack a given square.

    // TODO: This seems to be a noticeable (SPRT failing) slowdown over inlining it in `gen_pawn_moves`.
    // Optimize movegen in general, using speedup.py.
    fn pawn_captures(
        color: Color,
        pawns: ChessBitboard,
        capturable: ChessBitboard,
    ) -> [(ChessBitboard, isize); 2] {
        let left_pawn_captures;
        let right_pawn_captures;
        match color {
            White => {
                right_pawn_captures = (
                    ((pawns & !ChessBitboard::file_no(H_FILE_NO)) << 9) & capturable,
                    9,
                );
                left_pawn_captures = (
                    ((pawns & !ChessBitboard::file_no(A_FILE_NO)) << 7) & capturable,
                    7,
                );
            }
            Black => {
                right_pawn_captures = (
                    ((pawns & !ChessBitboard::file_no(A_FILE_NO)) >> 9) & capturable,
                    -9,
                );
                left_pawn_captures = (
                    ((pawns & !ChessBitboard::file_no(H_FILE_NO)) >> 7) & capturable,
                    -7,
                );
            }
        }
        [left_pawn_captures, right_pawn_captures]
    }

    fn all_pawn_captures(
        color: Color,
        pawns: ChessBitboard,
        capturable: ChessBitboard,
    ) -> ChessBitboard {
        let res = Self::pawn_captures(color, pawns, capturable);
        res[0].0 | res[1].0
    }

    fn normal_king_moves_from_square(square: ChessSquare, filter: ChessBitboard) -> ChessBitboard {
        KINGS[square.index()] & filter
    }

    fn knight_moves_from_square(square: ChessSquare, filter: ChessBitboard) -> ChessBitboard {
        KNIGHTS[square.index()] & filter
    }

    fn gen_sliders_from_square(
        &self,
        square: ChessSquare,
        slider_move: SliderMove,
        filter: ChessBitboard,
        square_bb_if_occupied: ChessBitboard,
    ) -> ChessBitboard {
        let blockers = self.occupied_bb();
        let mut attacks = match slider_move {
            SliderMove::Bishop => {
                ChessBitboard::bishop_attacks(square, blockers ^ square_bb_if_occupied)
            }
            SliderMove::Rook => {
                ChessBitboard::rook_attacks(square, blockers ^ square_bb_if_occupied)
            }
        };
        attacks & filter
    }

    pub fn all_attacking(&self, square: ChessSquare) -> ChessBitboard {
        let rook_sliders = self.piece_bb(Rook) | self.piece_bb(Queen);
        let bishop_sliders = self.piece_bb(Bishop) | self.piece_bb(Queen);
        let square_bb = ChessBitboard::single_piece(square.index());
        let square_bb_if_occupied = if self.is_occupied(square) {
            square_bb
        } else {
            ChessBitboard::default()
        };
        self.gen_sliders_from_square(
            square,
            SliderMove::Rook,
            rook_sliders,
            square_bb_if_occupied,
        ) | self.gen_sliders_from_square(
            square,
            SliderMove::Bishop,
            bishop_sliders,
            square_bb_if_occupied,
        ) | Self::knight_moves_from_square(square, self.piece_bb(Knight))
            | Self::normal_king_moves_from_square(square, self.piece_bb(King))
            | Self::all_pawn_captures(Black, square_bb, self.colored_piece_bb(White, Pawn))
            | Self::all_pawn_captures(White, square_bb, self.colored_piece_bb(Black, Pawn))
    }

    pub fn ray_attacks(
        &self,
        target: ChessSquare,
        ray_square: ChessSquare,
        blockers: ChessBitboard,
    ) -> ChessBitboard {
        let file_diff = target.file().wrapping_sub(ray_square.file());
        let rank_diff = target.rank().wrapping_sub(ray_square.rank());
        if file_diff == 0 {
            ChessBitboard::slider_attacks(target, blockers, Vertical)
                & (self.piece_bb(Rook) | self.piece_bb(Queen))
        } else if rank_diff == 0 {
            ChessBitboard::slider_attacks(target, blockers, Horizontal)
                & (self.piece_bb(Rook) | self.piece_bb(Queen))
        } else if file_diff == rank_diff {
            ChessBitboard::slider_attacks(target, blockers, Diagonal)
                & (self.piece_bb(Bishop) | self.piece_bb(Queen))
        } else if file_diff == 0_u8.wrapping_sub(rank_diff) {
            ChessBitboard::slider_attacks(target, blockers, AntiDiagonal)
                & (self.piece_bb(Bishop) | self.piece_bb(Queen))
        } else {
            ChessBitboard::default()
        }
    }
}
