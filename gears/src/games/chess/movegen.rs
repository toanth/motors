use crate::games::chess::castling::CastleRight;
use crate::games::chess::moves::ChessMoveFlags::*;
use crate::games::chess::moves::{ChessMove, ChessMoveFlags};
use crate::games::chess::pieces::UncoloredChessPiece::*;
use crate::games::chess::pieces::{ColoredChessPiece, UncoloredChessPiece};
use crate::games::chess::squares::ChessSquare;
use crate::games::chess::CastleRight::*;
use crate::games::chess::ChessColor::*;
use crate::games::chess::{ChessColor, ChessMoveList, Chessboard};
use crate::games::{Board, Color, ColoredPieceType};
use crate::general::bitboards::chess::{ChessBitboard, KINGS, KNIGHTS, PAWN_CAPTURES};
use crate::general::bitboards::RayDirections::{AntiDiagonal, Diagonal, Horizontal, Vertical};
use crate::general::bitboards::{Bitboard, RawBitboard, RawStandardBitboard};
use crate::general::moves::Move;

#[derive(Debug, Copy, Clone)]
enum SliderMove {
    Bishop,
    Rook,
}

impl Chessboard {
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
        piece: UncoloredChessPiece,
        color: ChessColor,
    ) -> ChessBitboard {
        let square_bb_if_occupied = square.bb() & self.occupied_bb();
        match piece {
            Pawn => Self::single_pawn_captures(color, square),
            Knight => Self::knight_moves_from_square(square),
            Bishop => {
                self.slider_attacks_from_square(square, SliderMove::Bishop, square_bb_if_occupied)
            }
            Rook => {
                self.slider_attacks_from_square(square, SliderMove::Rook, square_bb_if_occupied)
            }
            Queen => {
                self.slider_attacks_from_square(square, SliderMove::Bishop, square_bb_if_occupied)
                    | self.slider_attacks_from_square(
                        square,
                        SliderMove::Rook,
                        square_bb_if_occupied,
                    )
            }
            King => Self::normal_king_moves_from_square(square),
            Empty => ChessBitboard::default(),
        }
    }

    pub fn is_move_pseudolegal_impl(&self, mov: ChessMove) -> bool {
        let piece = mov.uncolored_piece();
        let src = mov.src_square();
        let color = self.active_player;
        if !self
            .colored_piece_bb(color, piece)
            .is_bit_set_at(src.bb_idx())
        {
            return false;
        }
        if mov.is_castle() {
            (self.rook_start_square(color, Kingside) == mov.dest_square()
                && self.is_castling_pseudolegal(Kingside))
                || (self.rook_start_square(color, Queenside) == mov.dest_square()
                    && self.is_castling_pseudolegal(Queenside))
        } else if mov.uncolored_piece() == Pawn {
            let capturable = self.colored_bb(color.other())
                | self.ep_square.map(ChessSquare::bb).unwrap_or_default();
            Self::single_pawn_moves(color, src, capturable, self.empty_bb())
                .is_bit_set_at(mov.dest_square().bb_idx())
        } else {
            (self.attacks_no_castle_or_pawn_push(src, mov.uncolored_piece(), color)
                & !self.active_player_bb())
            .is_bit_set_at(mov.dest_square().bb_idx())
        }
    }

    /// Used for castling and to implement `is_in_check`:
    /// Pretend there is a king of color `us` at `square` and test if it is in check.
    pub fn is_in_check_on_square(&self, us: ChessColor, square: ChessSquare) -> bool {
        (self.all_attacking(square) & self.colored_bb(us.other())).has_set_bit()
    }

    #[must_use]
    pub fn gen_all_pseudolegal_moves(&self) -> ChessMoveList {
        self.gen_pseudolegal_moves(!self.colored_bb(self.active_player), false)
    }

    #[must_use]
    pub fn gen_tactical_pseudolegal(&self) -> ChessMoveList {
        self.gen_pseudolegal_moves(self.colored_bb(self.active_player.other()), true)
    }

    #[must_use]
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
        let left_pawn_captures;
        let right_pawn_captures;
        let capturable = opponent | self.ep_square.map(ChessSquare::bb).unwrap_or_default();
        if color == White {
            regular_pawn_moves = (pawns.north() & free, 8);
            double_pawn_moves = (
                ((pawns & ChessBitboard::rank_no(1)) << 16) & free.north() & free,
                16,
            );
            right_pawn_captures = (pawns.north_east() & capturable, 9);
            left_pawn_captures = (pawns.north_west() & capturable, 7);
        } else {
            regular_pawn_moves = (pawns.south() & free, -8);
            double_pawn_moves = (
                ((pawns & ChessBitboard::rank_no(6)) >> 16) & free.south() & free,
                -16,
            );
            right_pawn_captures = (pawns.south_west() & capturable, -9);
            left_pawn_captures = (pawns.south_east() & capturable, -7);
        }
        for move_type in [
            right_pawn_captures,
            left_pawn_captures,
            regular_pawn_moves,
            double_pawn_moves,
        ] {
            let bb = move_type.0;
            for to in bb.ones() {
                let from = ChessSquare::from_bb_index((to.to_u8() as isize - move_type.1) as usize);
                let is_capture = from.file() != to.file();
                let mut flag = NormalPawnMove;
                if self.ep_square.is_some_and(|sq| sq == to) {
                    flag = EnPassant;
                } else if to.is_backrank() {
                    for flag in [PromoQueen, PromoKnight] {
                        list.push(ChessMove::new(from, to, flag));
                    }
                    if !only_tactical {
                        for flag in [PromoRook, PromoBishop] {
                            list.push(ChessMove::new(from, to, flag));
                        }
                    }
                    continue;
                } else if only_tactical && !is_capture {
                    continue;
                }
                list.push(ChessMove::new(from, to, flag));
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
        let (rook_free_bb, king_free_bb) = match side {
            Queenside => (
                ROOK_QUEENSIDE_BB[self.castling.rook_start_file(color, Queenside) as usize]
                    << (color as usize * 7 * 8),
                KING_QUEENSIDE_BB[king_file] << (color as usize * 7 * 8),
            ),
            Kingside => (
                ROOK_KINGSIDE_BB[self.castling.rook_start_file(color, Kingside) as usize]
                    << (color as usize * 7 * 8),
                KING_KINGSIDE_BB[king_file] << (color as usize * 7 * 8),
            ),
        };
        if self.castling.can_castle(color, side) {
            let rook = self.rook_start_square(color, side);
            if ((self.occupied_bb() ^ rook.bb()) & king_free_bb).is_zero()
                && ((self.occupied_bb() ^ king) & rook_free_bb).is_zero()
            {
                debug_assert_eq!(
                    self.colored_piece_on(rook).symbol,
                    ColoredChessPiece::new(color, Rook)
                );
                return true;
            }
        }
        false
    }

    fn gen_king_moves(&self, list: &mut ChessMoveList, filter: ChessBitboard, only_captures: bool) {
        let color = self.active_player;
        let king = self.colored_piece_bb(color, King);
        let king_square = ChessSquare::from_bb_index(king.trailing_zeros());
        let mut moves = Self::normal_king_moves_from_square(king_square) & filter;
        while moves.has_set_bit() {
            let target = moves.pop_lsb();
            list.push(ChessMove::new(
                king_square,
                ChessSquare::from_bb_index(target),
                NormalKingMove,
            ));
        }
        if only_captures {
            return;
        }
        // Castling, handling the general (D)FRC case.
        if self.is_castling_pseudolegal(Queenside) {
            let rook = self.rook_start_square(color, Queenside);
            list.push(ChessMove::new(king_square, rook, CastleQueenside));
        }
        if self.is_castling_pseudolegal(Kingside) {
            let rook = self.rook_start_square(color, Kingside);
            list.push(ChessMove::new(king_square, rook, CastleKingside));
        }
    }

    fn gen_knight_moves(&self, list: &mut ChessMoveList, filter: ChessBitboard) {
        let knights = self.colored_piece_bb(self.active_player, Knight);
        for from in knights.ones() {
            let attacks = Self::knight_moves_from_square(from) & filter;
            for to in attacks.ones() {
                list.push(ChessMove::new(from, to, KnightMove));
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
        let slider_type = match slider_move {
            SliderMove::Bishop => Bishop,
            SliderMove::Rook => Rook,
        };
        let non_queens = self.colored_piece_bb(color, slider_type);
        let queens = self.colored_piece_bb(color, Queen);
        let pieces = queens | non_queens;
        for from in pieces.ones() {
            let attacks = self.slider_attacks_from_square(from, slider_move, from.bb()) & filter;
            for to in attacks.ones() {
                let move_type = if queens.is_bit_set_at(from.bb_idx()) {
                    QueenMove
                } else {
                    ChessMoveFlags::normal_move(slider_type)
                };
                list.push(ChessMove::new(from, to, move_type));
            }
        }
    }

    /// All the following methods can be called with squares that do not contain the specified piece.
    /// This makes sense because it allows to find all pieces able to attack a given square.

    pub fn normal_king_moves_from_square(square: ChessSquare) -> ChessBitboard {
        KINGS[square.bb_idx()]
    }

    pub fn knight_moves_from_square(square: ChessSquare) -> ChessBitboard {
        KNIGHTS[square.bb_idx()]
    }

    pub fn single_pawn_captures(color: ChessColor, square: ChessSquare) -> ChessBitboard {
        PAWN_CAPTURES[color as usize][square.bb_idx()]
    }

    fn slider_attacks_from_square(
        &self,
        square: ChessSquare,
        slider_move: SliderMove,
        square_bb_if_occupied: ChessBitboard,
    ) -> ChessBitboard {
        let blockers = self.occupied_bb();
        match slider_move {
            SliderMove::Bishop => {
                ChessBitboard::bishop_attacks(square, blockers ^ square_bb_if_occupied)
            }
            SliderMove::Rook => {
                ChessBitboard::rook_attacks(square, blockers ^ square_bb_if_occupied)
            }
        }
    }

    pub fn all_attacking(&self, square: ChessSquare) -> ChessBitboard {
        let rook_sliders = self.piece_bb(Rook) | self.piece_bb(Queen);
        let bishop_sliders = self.piece_bb(Bishop) | self.piece_bb(Queen);
        let square_bb = square.bb();
        let square_bb_if_occupied = if self.is_occupied(square) {
            square_bb
        } else {
            ChessBitboard::default()
        };
        (rook_sliders
            & self.slider_attacks_from_square(square, SliderMove::Rook, square_bb_if_occupied))
            | (bishop_sliders
                & self.slider_attacks_from_square(
                    square,
                    SliderMove::Bishop,
                    square_bb_if_occupied,
                ))
            | (Self::knight_moves_from_square(square) & self.piece_bb(Knight))
            | (Self::normal_king_moves_from_square(square) & self.piece_bb(King))
            | Self::single_pawn_captures(Black, square) & self.colored_piece_bb(White, Pawn)
            | Self::single_pawn_captures(White, square) & self.colored_piece_bb(Black, Pawn)
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
