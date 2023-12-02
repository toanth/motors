use itertools::Itertools;

use crate::games::chess::moves::ChessMoveFlags::*;
use crate::games::chess::pieces::ColoredChessPiece;
use crate::games::chess::pieces::UncoloredChessPiece::*;
use crate::games::chess::squares::{
    ChessSquare, A_FILE_NO, C_FILE_NO, E_FILE_NO, G_FILE_NO, H_FILE_NO, NUM_COLUMNS,
};
use crate::games::chess::CastleRight::*;
use crate::games::chess::{ChessMove, ChessMoveList, Chessboard};
use crate::games::Color::*;
use crate::games::{sup_distance, Board, Color, ColoredPiece, ColoredPieceType, Move};
use crate::general::bitboards::{Bitboard, ChessBitboard, KNIGHTS};

enum SliderMove {
    Bishop,
    Rook,
}

impl Chessboard {
    pub(super) fn is_move_pseudolegal_impl(&self, mov: ChessMove) -> bool {
        let piece = mov.piece(self);
        if piece.is_empty() || piece.color().unwrap() != self.active_player {
            return false;
        }
        let mut list = ChessMoveList::default();
        let filter = !self.colored_bb(self.active_player);
        match piece.uncolored() {
            Pawn => self.gen_pawn_moves(&mut list, filter),
            Knight => {
                return ChessBitboard(KNIGHTS[mov.from_square().index()])
                    .is_bit_set_at(mov.to_square().index());
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
    pub(super) fn is_in_check_on_square(&self, us: Color, square: ChessSquare) -> bool {
        let them = us.other();
        let idx = square.index();
        let attacks = ChessBitboard(KNIGHTS[square.index()]);
        if (attacks & self.colored_piece_bb(them, Knight)).has_set_bit() {
            return true;
        }
        let bb = ChessBitboard::single_piece(idx);
        let blockers = self.occupied_bb() & !bb;
        let attacks = ChessBitboard::bishop_attacks(square, blockers, self.size());
        if (attacks & (self.colored_piece_bb(them, Bishop) | self.colored_piece_bb(them, Queen)))
            .has_set_bit()
        {
            return true;
        }
        let attacks = ChessBitboard::rook_attacks(square, blockers, self.size());
        if (attacks & (self.colored_piece_bb(them, Rook) | self.colored_piece_bb(them, Queen)))
            .has_set_bit()
        {
            return true;
        }
        let their_pawns = self.colored_piece_bb(them, Pawn);
        let pawn_attacks = match us {
            White => {
                ((their_pawns & !ChessBitboard::file(A_FILE_NO, self.size())) >> 9)
                    | ((their_pawns & !ChessBitboard::file(H_FILE_NO, self.size())) >> 7)
            }
            Black => {
                ((their_pawns & !ChessBitboard::file(H_FILE_NO, self.size())) << 9)
                    | ((their_pawns & !ChessBitboard::file(A_FILE_NO, self.size())) << 7)
            }
        };
        if pawn_attacks.is_bit_set_at(idx) {
            return true;
        }
        // this can't happen in a legal position, but it can happen in pseudolegal movegen
        sup_distance(square, self.king_square(them)) <= 1
    }

    pub(super) fn gen_all_pseudolegal_moves(&self) -> ChessMoveList {
        self.gen_pseudolegal_moves(!self.colored_bb(self.active_player), false)
    }

    pub(super) fn gen_pseudolegal_captures(&self) -> ChessMoveList {
        self.gen_pseudolegal_moves(self.colored_bb(self.active_player.other()), true)
    }

    fn gen_pseudolegal_moves(&self, filter: ChessBitboard, only_captures: bool) -> ChessMoveList {
        let mut list = ChessMoveList::default();
        self.gen_slider_moves(SliderMove::Bishop, &mut list, filter);
        self.gen_slider_moves(SliderMove::Rook, &mut list, filter);
        self.gen_knight_moves(&mut list, filter);
        self.gen_king_moves(&mut list, filter, only_captures);
        self.gen_pawn_moves(&mut list, filter);
        list
    }

    fn gen_pawn_moves(&self, list: &mut ChessMoveList, filter: ChessBitboard) {
        let color = self.active_player;
        let pawns = self.colored_piece_bb(color, Pawn);
        let occupied = self.occupied_bb();
        let free = !occupied & filter;
        let opponent = self.colored_bb(color.other());
        let regular_pawn_moves;
        let double_pawn_moves;
        let left_pawn_captures;
        let right_pawn_captures;
        let capturable = opponent
            | self
                .ep_square
                .map(|s| ChessBitboard::single_piece(s.index()))
                .unwrap_or_default();
        if color == White {
            regular_pawn_moves = ((pawns << 8) & free, 8);
            double_pawn_moves = (
                ((pawns & ChessBitboard::rank(1, self.size())) << 16) & (free << 8) & free,
                16,
            );
            right_pawn_captures = (
                ((pawns & !ChessBitboard::file(H_FILE_NO, self.size())) << 9) & capturable,
                9,
            );
            left_pawn_captures = (
                ((pawns & !ChessBitboard::file(A_FILE_NO, self.size())) << 7) & capturable,
                7,
            );
        } else {
            regular_pawn_moves = ((pawns >> 8) & free, -8);
            double_pawn_moves = (
                ((pawns & ChessBitboard::rank(6, self.size())) >> 16) & (free >> 8) & free,
                -16,
            );
            right_pawn_captures = (
                ((pawns & !ChessBitboard::file(A_FILE_NO, self.size())) >> 9) & capturable,
                -9,
            );
            left_pawn_captures = (
                ((pawns & !ChessBitboard::file(H_FILE_NO, self.size())) >> 7) & capturable,
                -7,
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
                let mut flag = Normal;
                if to == self.ep_square.unwrap_or(ChessSquare::unchecked(64)) {
                    flag = EnPassant;
                } else if to.rank() == 0 || to.rank() == 7 {
                    for flag in [PromoQueen, PromoKnight, PromoRook, PromoBishop] {
                        list.add_move(ChessMove::new(from, to, flag));
                    }
                    continue;
                }
                list.add_move(ChessMove::new(from, to, flag));
            }
        }
    }

    fn gen_king_moves(&self, list: &mut ChessMoveList, filter: ChessBitboard, only_captures: bool) {
        let color = self.active_player;
        let king = self.colored_piece_bb(color, King);
        let king_not_a_file = king & !ChessBitboard::file(A_FILE_NO, self.size());
        let king_not_h_file = king & !ChessBitboard::file(H_FILE_NO, self.size());
        let moves = (king << 8)
            | (king >> 8)
            | (king_not_a_file >> 1)
            | (king_not_a_file << 7)
            | (king_not_a_file >> 9)
            | (king_not_h_file << 1)
            | (king_not_h_file >> 7)
            | (king_not_h_file << 9);
        let mut moves = moves & filter;
        let king_square = ChessSquare::new(king.trailing_zeros());
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
        let king_rank = king_square.rank();
        // Since this is pseudolegal movegen, we only check if the king ends up in check / moves
        // over a square where it would be check when playing the move.
        if self.flags.can_castle(color, Queenside)
            && (0b1110 << (king_rank * NUM_COLUMNS) & self.occupied_bb().0 == 0)
        {
            debug_assert_eq!(king_square.file(), E_FILE_NO);
            debug_assert_eq!(king_square.rank() % 7, 0);
            debug_assert_eq!(
                self.piece_on(self.rook_start_square(color, Queenside))
                    .symbol,
                ColoredChessPiece::new(color, Rook)
            );
            list.add_move(ChessMove::new(
                king_square,
                ChessSquare::from_rank_file(king_rank, C_FILE_NO),
                Castle,
            ));
        }
        if self.flags.can_castle(color, Kingside)
            && (0b110_0000 << (king_rank * NUM_COLUMNS) & self.occupied_bb().0 == 0)
        {
            debug_assert_eq!(king_square.file(), E_FILE_NO);
            debug_assert_eq!(king_square.rank(), color as usize * 7);
            debug_assert_eq!(
                self.piece_on(self.rook_start_square(color, Kingside))
                    .symbol,
                ColoredChessPiece::new(color, Rook)
            );
            list.add_move(ChessMove::new(
                king_square,
                ChessSquare::from_rank_file(king_rank, G_FILE_NO),
                Castle,
            ));
        }
    }

    fn gen_knight_moves(&self, list: &mut ChessMoveList, filter: ChessBitboard) {
        let mut knights = self.colored_piece_bb(self.active_player, Knight);
        while knights.has_set_bit() {
            let square_idx = knights.pop_lsb();
            let mut attacks = ChessBitboard(KNIGHTS[square_idx]) & filter;
            while attacks.has_set_bit() {
                let to = ChessSquare::new(attacks.pop_lsb());
                list.add_move(ChessMove::new(ChessSquare::new(square_idx), to, Normal));
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
        let blockers = self.occupied_bb();
        while pieces.has_set_bit() {
            let idx = pieces.pop_lsb();
            let this_piece = ChessBitboard(1 << idx);
            let mut attacks = match slider_move {
                SliderMove::Bishop => ChessBitboard::bishop_attacks(
                    ChessSquare::new(idx),
                    blockers ^ this_piece,
                    self.size(),
                ),
                SliderMove::Rook => ChessBitboard::rook_attacks(
                    ChessSquare::new(idx),
                    blockers ^ this_piece,
                    self.size(),
                ),
            };
            attacks &= filter;
            let from = ChessSquare::new(idx);
            while attacks.has_set_bit() {
                let to = ChessSquare::new(attacks.pop_lsb());
                list.add_move(ChessMove::new(from, to, Normal));
            }
        }
    }
}
