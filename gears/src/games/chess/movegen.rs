use std::iter::Peekable;

use derive_more::{Add, AddAssign, Sub, SubAssign};
use itertools::Itertools;
use strum::IntoEnumIterator;

use crate::games::chess::moves::ChessMoveFlags::*;
use crate::games::chess::pieces::UncoloredChessPiece::*;
use crate::games::chess::pieces::{
    ColoredChessPiece, UncoloredChessPiece, UncoloredChessPieceIter, NUM_CHESS_PIECES,
};
use crate::games::chess::squares::{
    ChessSquare, A_FILE_NO, C_FILE_NO, E_FILE_NO, G_FILE_NO, H_FILE_NO, NUM_COLUMNS,
};
use crate::games::chess::CastleRight::*;
use crate::games::chess::CastleRight::*;
use crate::games::chess::{ChessMove, ChessMoveList, Chessboard};
use crate::games::Color::*;
use crate::games::{
    sup_distance, AbstractPieceType, Board, Color, ColoredPiece, ColoredPieceType, Coordinates,
    DimT, Move,
};
use crate::general::bitboards::chess::{ChessBitboard, KNIGHTS};
use crate::general::bitboards::{Bitboard, RawBitboard};

#[derive(Debug, Copy, Clone)]
enum SliderMove {
    Bishop,
    Rook,
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Add, AddAssign, Sub, SubAssign)]
struct SeeScore(i32);

// TODO: Better values?
const SEE_SCORES: [SeeScore; NUM_CHESS_PIECES + 1] = [
    SeeScore(100),
    SeeScore(300),
    SeeScore(300),
    SeeScore(500),
    SeeScore(900),
    SeeScore(99999),
    SeeScore(0), // also give the empty square a see value to make the implementation simpler
];

fn piece_see_value(piece: UncoloredChessPiece) -> SeeScore {
    SEE_SCORES[piece.to_uncolored_idx()]
}

// TODO: Use the north(), west(), etc. methods

impl Chessboard {
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
                return ChessBitboard::from_u64(KNIGHTS[mov.src_square().index()])
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
        let attacks = ChessBitboard::from_u64(KNIGHTS[square.index()]);
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
        sup_distance(square, self.king_square(them)) <= 1
    }

    pub fn gen_all_pseudolegal_moves(&self) -> ChessMoveList {
        self.gen_pseudolegal_moves(!self.colored_bb(self.active_player), false)
    }

    pub fn gen_noisy_pseudolegal(&self) -> ChessMoveList {
        self.gen_pseudolegal_moves(self.colored_bb(self.active_player.other()), true)
    }

    fn gen_pseudolegal_moves(&self, filter: ChessBitboard, only_noisy: bool) -> ChessMoveList {
        let mut list = ChessMoveList::default();
        self.gen_slider_moves(SliderMove::Bishop, &mut list, filter);
        self.gen_slider_moves(SliderMove::Rook, &mut list, filter);
        self.gen_knight_moves(&mut list, filter);
        self.gen_king_moves(&mut list, filter, only_noisy);
        self.gen_pawn_moves(&mut list, only_noisy);
        list
    }

    fn gen_pawn_moves(&self, list: &mut ChessMoveList, only_noisy: bool) {
        let color = self.active_player;
        let pawns = self.colored_piece_bb(color, Pawn);
        let occupied = self.occupied_bb();
        let free = !occupied;
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
                ((pawns & ChessBitboard::rank_no(1)) << 16) & (free << 8) & free,
                16,
            );
            right_pawn_captures = (
                ((pawns & !ChessBitboard::file_no(H_FILE_NO)) << 9) & capturable,
                9,
            );
            left_pawn_captures = (
                ((pawns & !ChessBitboard::file_no(A_FILE_NO)) << 7) & capturable,
                7,
            );
        } else {
            regular_pawn_moves = ((pawns >> 8) & free, -8);
            double_pawn_moves = (
                ((pawns & ChessBitboard::rank_no(6)) >> 16) & (free >> 8) & free,
                -16,
            );
            right_pawn_captures = (
                ((pawns & !ChessBitboard::file_no(A_FILE_NO)) >> 9) & capturable,
                -9,
            );
            left_pawn_captures = (
                ((pawns & !ChessBitboard::file_no(H_FILE_NO)) >> 7) & capturable,
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
                let is_capture = from.file() != to.file();
                let mut flag = Normal;
                if to == self.ep_square.unwrap_or(ChessSquare::no_coordinates()) {
                    flag = EnPassant;
                } else if to.rank() == 0 || to.rank() == 7 {
                    for flag in [PromoQueen, PromoKnight] {
                        list.add_move(ChessMove::new(from, to, flag));
                    }
                    if !only_noisy || is_capture {
                        for flag in [PromoRook, PromoBishop] {
                            list.add_move(ChessMove::new(from, to, flag));
                        }
                    }
                    continue;
                } else if only_noisy && !is_capture {
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
        let king_rank = king_square.rank();
        // Since this is pseudolegal movegen, we only check if the king ends up in check / moves
        // over a square where it would be check when playing the move.
        if self.flags.can_castle(color, Queenside)
            && (0b1110 << (king_rank as usize * NUM_COLUMNS) & self.occupied_bb().0 == 0)
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
            && (0b110_0000 << (king_rank as usize * NUM_COLUMNS) & self.occupied_bb().0 == 0)
        {
            debug_assert_eq!(king_square.file(), E_FILE_NO);
            debug_assert_eq!(king_square.rank(), color as DimT * 7);
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
            let mut attacks = self.gen_sliders_from_square(from, slider_move, filter);
            while attacks.has_set_bit() {
                let to = ChessSquare::new(attacks.pop_lsb());
                list.add_move(ChessMove::new(from, to, Normal));
            }
        }
    }

    /// All `*_from_square` methods and `pawn_captures` can be called with squares that do not contain the specified piece.
    /// This makes sense because it allows to find all pieces able to attack a given square.

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
        [right_pawn_captures, left_pawn_captures]
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
        // TODO: Use lookup table and measure speedup
        let king = ChessBitboard::single_piece(square.index());
        let king_not_a_file = king & !ChessBitboard::file_no(A_FILE_NO);
        let king_not_h_file = king & !ChessBitboard::file_no(H_FILE_NO);
        let moves = (king << 8)
            | (king >> 8)
            | (king_not_a_file >> 1)
            | (king_not_a_file << 7)
            | (king_not_a_file >> 9)
            | (king_not_h_file << 1)
            | (king_not_h_file >> 7)
            | (king_not_h_file << 9);
        moves & filter
    }

    fn knight_moves_from_square(square: ChessSquare, filter: ChessBitboard) -> ChessBitboard {
        ChessBitboard::from_u64(KNIGHTS[square.index()]) & filter
    }

    fn gen_sliders_from_square(
        &self,
        square: ChessSquare,
        slider_move: SliderMove,
        filter: ChessBitboard,
    ) -> ChessBitboard {
        let blockers = self.occupied_bb();
        let idx = square.index();
        let this_piece = ChessBitboard::single_piece(idx);
        let mut attacks = match slider_move {
            SliderMove::Bishop => {
                ChessBitboard::bishop_attacks(ChessSquare::new(idx), blockers ^ this_piece)
            }
            SliderMove::Rook => {
                ChessBitboard::rook_attacks(ChessSquare::new(idx), blockers ^ this_piece)
            }
        };
        attacks & filter
    }

    fn all_attacking(&self, square: ChessSquare) -> ChessBitboard {
        let rook_sliders = self.piece_bb(Rook) | self.piece_bb(Queen);
        let bishop_sliders = self.piece_bb(Bishop) | self.piece_bb(Queen);
        let square_bb = ChessBitboard::single_piece(square.index());
        self.gen_sliders_from_square(square, SliderMove::Rook, rook_sliders)
            | self.gen_sliders_from_square(square, SliderMove::Bishop, bishop_sliders)
            | Self::knight_moves_from_square(square, self.piece_bb(Knight))
            | Self::normal_king_moves_from_square(square, self.piece_bb(King))
            | Self::all_pawn_captures(White, self.colored_piece_bb(Black, Pawn), square_bb)
            | Self::all_pawn_captures(Black, self.colored_piece_bb(White, Pawn), square_bb)
    }

    // TODO: Discovered attacks, test!
    fn next_see_attacker(
        &self,
        color: Color,
        all_attackers: ChessBitboard,
        current_attackers: &mut ChessBitboard,
        attacker_iter: &mut Peekable<UncoloredChessPieceIter>,
    ) -> Option<UncoloredChessPiece> {
        if current_attackers.is_zero() {
            while let Some(piece) = attacker_iter.peek().copied() {
                if piece == Empty {
                    return None;
                }
                *current_attackers = all_attackers & self.colored_piece_bb(color, piece);
                if current_attackers.has_set_bit() {
                    break;
                }
                attacker_iter.next();
            }
        }
        current_attackers.pop_lsb();
        Some(attacker_iter.peek().copied().unwrap())
    }

    fn see(&self, mov: ChessMove, mut alpha: SeeScore, mut beta: SeeScore) -> SeeScore {
        // TODO: Also handle EP.
        let square = mov.dest_square();
        let src_bb = mov.src_square().to_bitboard();
        debug_assert!(alpha < beta);
        let color = self.active_player;
        let all_attackers = self.all_attacking(square) ^ src_bb;
        let mut our_attacker_iter = UncoloredChessPiece::iter().peekable();
        let mut their_attacker_iter = UncoloredChessPiece::iter().peekable();
        let mut our_current_attackers = ChessBitboard::default();
        let mut their_current_attackers = ChessBitboard::default();
        let mut our_victim = self.piece_on(square).uncolored();
        let mut their_victim = self.piece_on(mov.src_square()).uncolored();
        let mut eval = piece_see_value(our_victim);
        loop {
            if eval <= alpha {
                return alpha;
            } else if eval < beta {
                beta = eval;
            }
            let Some(piece) = self.next_see_attacker(
                color.other(),
                all_attackers,
                &mut their_current_attackers,
                &mut their_attacker_iter,
            ) else {
                return eval.min(beta);
            };
            our_victim = piece;
            eval -= piece_see_value(their_victim);
            if eval >= beta {
                return beta;
            } else if eval > alpha {
                alpha = eval;
            }
            let Some(piece) = self.next_see_attacker(
                color,
                all_attackers,
                &mut our_current_attackers,
                &mut our_attacker_iter,
            ) else {
                return eval.max(alpha);
            };
            their_victim = piece;
            eval += piece_see_value(our_victim);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::chess::Chessboard;
    use crate::games::Board;
    use std::str::FromStr;

    #[test]
    fn trivia_see_test() {
        let board = Chessboard::from_name("kiwipete").unwrap();
        let see_score_no_capture = board.see(
            ChessMove::from_compact_text("a1b1", &board).unwrap(),
            SeeScore(-1000),
            SeeScore(1000),
        );
        assert_eq!(see_score_no_capture, SeeScore(0));
        let see_score_bishop_capture = board.see(
            ChessMove::from_compact_text("e2a6", &board).unwrap(),
            SeeScore(-1000),
            SeeScore(1000),
        );
        assert_eq!(see_score_bishop_capture, SeeScore(300));
        let see_score_bishop_capture = board.see(
            ChessMove::from_compact_text("e2a6", &board).unwrap(),
            SeeScore(0),
            SeeScore(1),
        );
        assert!(see_score_bishop_capture >= SeeScore(1));

        let see_score_bad_capture = board.see(
            ChessMove::from_compact_text("f3f6", &board).unwrap(),
            SeeScore(-9999),
            SeeScore(9999),
        );
        assert_eq!(see_score_bad_capture, SeeScore(-600));

        let see_score_bad_pawn_capture = board.see(
            ChessMove::from_compact_text("f3h3", &board).unwrap(),
            SeeScore(-9999),
            SeeScore(9999),
        );
        assert_eq!(see_score_bad_pawn_capture, SeeScore(-800));

        let see_score_good_pawn_capture = board.see(
            ChessMove::from_compact_text("g2h3", &board).unwrap(),
            SeeScore(-9999),
            SeeScore(9999),
        );
        assert_eq!(see_score_good_pawn_capture, SeeScore(100));
    }

    #[test]
    fn see_test() {
        let board = Chessboard::from_name("see_win_pawn").unwrap();
        let see_score = board.see(
            ChessMove::from_compact_text("f4e5", &board).unwrap(),
            SeeScore(-9999),
            SeeScore(9999),
        );
        assert_eq!(see_score, SeeScore(100));

        let see_score = board.see(
            ChessMove::from_compact_text("f4e5", &board).unwrap(),
            SeeScore(200),
            SeeScore(9999),
        );
        // TODO: Fail soft? It doesn't make sense to clamp to the window.
        assert_eq!(see_score, SeeScore(200));
    }

    #[test]
    fn see_xray_test() {
        let board = Chessboard::from_name("see_xray").unwrap();
        let see_score = board.see(
            ChessMove::from_compact_text("c4f4", &board).unwrap(),
            SeeScore(-9999),
            SeeScore(9999),
        );
        assert_eq!(see_score, SeeScore(-600));
    }
}
