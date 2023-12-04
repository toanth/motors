use std::fmt::{Display, Formatter};
use std::str::FromStr;

use itertools::Itertools;
use num::iter;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::games::chess::flags::CastleRight;
use crate::games::chess::flags::CastleRight::*;
use crate::games::chess::moves::ChessMoveFlags::*;
use crate::games::chess::pieces::UncoloredChessPiece::*;
use crate::games::chess::pieces::{ChessPiece, ColoredChessPiece, UncoloredChessPiece};
use crate::games::chess::squares::{ChessSquare, A_FILE_NO, D_FILE_NO, F_FILE_NO, H_FILE_NO};
use crate::games::chess::zobrist::PRECOMPUTED_ZOBRIST_KEYS;
use crate::games::chess::Chessboard;
use crate::games::{
    legal_moves_slow, AbstractPieceType, Board, BoardHistory, Color, ColoredPiece,
    ColoredPieceType, Move, MoveFlags,
};
use crate::general::bitboards::{Bitboard, ChessBitboard};

#[derive(Copy, Clone, Eq, PartialEq, Default, Debug, EnumIter)]
pub enum ChessMoveFlags {
    #[default]
    Normal,
    EnPassant,
    Castle,
    PromoKnight,
    PromoBishop,
    PromoRook,
    PromoQueen,
}

impl ChessMoveFlags {
    pub fn is_promo(self) -> bool {
        // TODO: Could also maybe do this on the u16 move directly, by comparing against 1 << (6+6+2)
        self as usize >= PromoKnight as usize
    }

    pub fn promo_piece(self) -> UncoloredChessPiece {
        debug_assert!(self.is_promo());
        UncoloredChessPiece::iter()
            .nth((self as usize) - PromoKnight as usize + Knight as usize)
            .unwrap()
    }
}

impl MoveFlags for ChessMoveFlags {}

/// Members are stored as follows:
/// Bits 0-5: from square
/// Bits 6 - 11: To square
/// Bits 12-13: Move type
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Ord, PartialOrd)]
pub struct ChessMove(u16);

impl ChessMove {
    pub fn new(from: ChessSquare, to: ChessSquare, flags: ChessMoveFlags) -> Self {
        let idx = from.index() + (to.index() << 6) + ((flags as usize) << 12);
        Self(idx as u16)
    }

    pub(super) fn square_of_pawn_taken_by_ep(self) -> Option<ChessSquare> {
        if self.flags() != EnPassant {
            return None;
        }
        let to = self.to_square();
        if to.rank() == 2 {
            Some(ChessSquare::from_rank_file(3, to.file()))
        } else {
            Some(ChessSquare::from_rank_file(4, to.file()))
        }
    }
    pub fn piece(self, board: &Chessboard) -> ChessPiece {
        board.piece_on(self.from_square())
    }

    pub fn piece_on_target(self, board: &Chessboard) -> ChessPiece {
        board.piece_on(self.to_square())
    }

    pub fn is_capture(self, board: &Chessboard) -> bool {
        self.flags() == EnPassant || self.is_non_ep_capture(board)
    }

    pub fn is_non_ep_capture(self, board: &Chessboard) -> bool {
        !self.is_castle() && board.is_occupied(self.to_square())
    }

    pub fn captured(self, board: &Chessboard) -> UncoloredChessPiece {
        if self.flags() == EnPassant {
            Pawn
        } else if self.flags() == Castle {
            Empty
        } else {
            board.piece_on(self.to_square()).uncolored()
        }
    }

    pub fn is_promotion(self) -> bool {
        self.flags().is_promo()
    }

    pub fn is_castle(self) -> bool {
        self.flags() == Castle
    }

    pub fn castle_side(self) -> CastleRight {
        if self.to_square().file() < self.from_square().file() {
            Queenside
        } else {
            Kingside
        }
    }

    pub fn from_to_square(self) -> usize {
        (self.0 & 0xfff) as usize
    }
}

impl Display for ChessMove {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{0}", self.to_compact_text())
    }
}

impl Move<Chessboard> for ChessMove {
    type Flags = ChessMoveFlags;

    fn from_square(self) -> ChessSquare {
        ChessSquare::new((self.0 & 0x3f) as usize)
    }

    fn to_square(self) -> ChessSquare {
        ChessSquare::new(((self.0 >> 6) & 0x3f) as usize)
    }

    fn flags(self) -> Self::Flags {
        ChessMoveFlags::iter().nth((self.0 >> 12) as usize).unwrap()
    }

    fn to_compact_text(self) -> String {
        let flag = match self.flags() {
            PromoKnight => "n",
            PromoBishop => "b",
            PromoRook => "r",
            PromoQueen => "q",
            _ => "",
        };
        format!(
            "{from}{to}{flag}",
            from = self.from_square(),
            to = self.to_square()
        )
    }

    fn from_compact_text(s: &str, board: &Chessboard) -> Result<Self, String> {
        let from = ChessSquare::from_str(&s[..2])?;
        let to = ChessSquare::from_str(&s[2..4])?;
        let mut flags = Normal;
        let s = s.trim();
        if s.len() > 4 {
            let promo = s.chars().nth(4).unwrap();
            match promo {
                'n' => flags = PromoKnight,
                'b' => flags = PromoBishop,
                'r' => flags = PromoRook,
                'q' => flags = PromoQueen,
                _ => return Err(format!("Invalid character after to square: '{promo}'")),
            }
        } else if board.piece_on(from).uncolored() == King && to.file().abs_diff(from.file()) > 1 {
            flags = Castle;
        } else if board.piece_on(from).uncolored() == Pawn
            && board.piece_on(to).is_empty()
            && from.file() != to.file()
        {
            flags = EnPassant;
        }
        let res = from.index() + (to.index() << 6) + ((flags as usize) << 12);
        Ok(ChessMove(res as u16))
    }

    fn to_extended_text(self, board: &Chessboard) -> String {
        let piece = self.piece(board);
        let mut res = piece.to_ascii_char().to_ascii_uppercase().to_string();
        if piece.uncolored() == Pawn {
            if self.is_capture(board) {
                res = self
                    .from_square()
                    .to_string()
                    .chars()
                    .nth(0)
                    .unwrap()
                    .to_string();
            } else {
                res = String::default();
            }
        } else if self.is_castle() {
            return match self.castle_side() {
                Queenside => "O-O-O".to_string(),
                Kingside => "O-O".to_string(),
            };
        }
        let moves = legal_moves_slow(board)
            .filter(|mov| mov.piece(board) == piece && mov.to_square() == self.to_square())
            .collect_vec();
        assert!(moves.len() >= 1);
        if moves.len() > 1 {
            if moves
                .iter()
                .filter(|mov| mov.from_square().file() == self.from_square().file())
                .count()
                <= 1
            {
                res.push(self.from_square().to_string().chars().nth(0).unwrap());
            } else if moves
                .iter()
                .filter(|mov| mov.from_square().rank() == self.from_square().rank())
                .count()
                <= 1
            {
                res.push(self.from_square().to_string().chars().nth(1).unwrap());
            } else {
                res += &self.from_square().to_string();
            }
        }
        if self.is_capture(board) {
            res.push('x');
        }
        res += &self.to_square().to_string();
        if self.is_promotion() {
            res.push('=');
            res.push(self.flags().promo_piece().to_ascii_char());
        }
        let board = board.make_move(self).unwrap();
        if board.is_game_lost_slow() {
            res.push('#');
        } else if board.is_in_check() {
            res.push('+');
        }
        res
    }
    // can't parse pgn because I don't think it's worth it to implement that
}

impl Chessboard {
    pub(super) fn rook_start_square(&self, color: Color, side: CastleRight) -> ChessSquare {
        let idx = color as usize * 2 + side as usize;
        match idx {
            0 => ChessSquare::from_rank_file(0, A_FILE_NO),
            1 => ChessSquare::from_rank_file(0, H_FILE_NO),
            2 => ChessSquare::from_rank_file(7, A_FILE_NO),
            3 => ChessSquare::from_rank_file(7, H_FILE_NO),
            _ => panic!("Internal error"),
        }
    }

    pub(super) fn make_move_impl(mut self, mov: ChessMove) -> Option<Self> {
        let piece = mov.piece(&self).symbol;
        let uncolored = piece.uncolor();
        let color = self.active_player;
        let other = color.other();
        let from = mov.from_square();
        let to = mov.to_square();
        assert_eq!(color, piece.color().unwrap());
        self.ply_100_ctr += 1;
        // remove old castling flags
        self.hash ^= PRECOMPUTED_ZOBRIST_KEYS.castle_keys[self.flags.castling_flags() as usize];
        if let Some(square) = self.ep_square {
            self.hash ^= PRECOMPUTED_ZOBRIST_KEYS.ep_file_keys[square.file()];
        }
        self.ep_square = None;
        if mov.is_castle() {
            // TODO: Correct Chess960 castling
            let from_file = from.file() as isize;
            let to_file = to.file() as isize;
            let side = if from_file < to_file {
                Kingside
            } else {
                Queenside
            };

            for file in iter::range_step(from_file, to_file, if side == Kingside { 1 } else { -1 })
            {
                if self.is_in_check_on_square(
                    color,
                    ChessSquare::from_rank_file(from.rank(), file as usize),
                ) {
                    return None;
                }
            }
            let mut rook_from = ChessSquare::from_rank_file(from.rank(), H_FILE_NO);
            let mut rook_to = ChessSquare::from_rank_file(from.rank(), F_FILE_NO);
            if side == Queenside {
                rook_from = ChessSquare::from_rank_file(rook_from.rank(), A_FILE_NO);
                rook_to = ChessSquare::from_rank_file(rook_from.rank(), D_FILE_NO);
            }
            debug_assert!(self.piece_on(rook_from).symbol == ColoredChessPiece::new(color, Rook));
            self.move_piece(rook_from, rook_to, ColoredChessPiece::new(color, Rook));
        } else if mov.flags() == EnPassant {
            let taken_pawn = mov.square_of_pawn_taken_by_ep().unwrap();
            debug_assert_eq!(
                self.piece_on(taken_pawn).symbol,
                ColoredChessPiece::new(other, Pawn)
            );
            self.remove_piece(taken_pawn, ColoredChessPiece::new(other, Pawn));
            self.hash ^= PRECOMPUTED_ZOBRIST_KEYS.piece_key(Pawn, other, taken_pawn);
            self.ply_100_ctr = 0;
        } else if mov.is_non_ep_capture(&self) {
            let captured = self.piece_on(to).symbol;
            debug_assert_eq!(self.piece_on(to).color().unwrap(), other);
            debug_assert_ne!(self.piece_on(to).uncolored(), King);
            self.remove_piece(to, captured);
            self.hash ^= PRECOMPUTED_ZOBRIST_KEYS.piece_key(
                captured.uncolor(),
                captured.color().unwrap(),
                to,
            );
            self.ply_100_ctr = 0;
        } else if uncolored == Pawn {
            self.ply_100_ctr = 0;
            if from.rank().abs_diff(to.rank()) == 2 {
                self.ep_square = Some(ChessSquare::from_rank_file(
                    (to.rank() + from.rank()) / 2,
                    to.file(),
                ));
                self.hash ^= PRECOMPUTED_ZOBRIST_KEYS.ep_file_keys[to.file()];
            }
        }
        if uncolored == King {
            self.flags.clear_castle_rights(color);
        } else if from == self.rook_start_square(color, Queenside) {
            self.flags.unset_castle_right(color, Queenside);
        } else if from == self.rook_start_square(color, Kingside) {
            self.flags.unset_castle_right(color, Kingside);
        }
        if to == self.rook_start_square(other, Queenside) {
            self.flags.unset_castle_right(other, Queenside);
        } else if to == self.rook_start_square(other, Kingside) {
            self.flags.unset_castle_right(other, Kingside);
        }
        self.hash ^= PRECOMPUTED_ZOBRIST_KEYS.castle_keys[self.flags.castling_flags() as usize];
        self.move_piece(from, to, piece);
        if mov.is_promotion() {
            let bb = ChessBitboard::single_piece(self.to_idx(to));
            self.piece_bbs[Pawn as usize] ^= bb;
            self.piece_bbs[mov.flags().promo_piece() as usize] ^= bb;
            self.hash ^= PRECOMPUTED_ZOBRIST_KEYS.piece_key(Pawn, color, to);
            self.hash ^= PRECOMPUTED_ZOBRIST_KEYS.piece_key(mov.flags().promo_piece(), color, to);
        }
        self.ply += 1;
        self.flip_side_to_move()
    }

    /// Called at the end of make_nullmove and make_move.
    pub(super) fn flip_side_to_move(mut self) -> Option<Self> {
        if self.is_in_check() {
            None
        } else {
            self.active_player = self.active_player.other();
            if self.ep_square.is_some() {
                self.hash ^= PRECOMPUTED_ZOBRIST_KEYS.ep_file_keys[self.ep_square.unwrap().file()];
                self.ep_square = None;
            }
            self.hash ^= PRECOMPUTED_ZOBRIST_KEYS.side_to_move_key;
            debug_assert_eq!(self.hash, self.zobrist_hash());
            Some(self)
        }
    }
}
