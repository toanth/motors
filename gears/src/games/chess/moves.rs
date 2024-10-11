use anyhow::{anyhow, bail};
use colored::Colorize;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use arbitrary::Arbitrary;

use itertools::Itertools;
use num::iter;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, FromRepr};

use crate::games::chess::castling::CastleRight;
use crate::games::chess::castling::CastleRight::*;
use crate::games::chess::moves::ChessMoveFlags::*;
use crate::games::chess::pieces::ChessPieceType::*;
use crate::games::chess::pieces::{ChessPiece, ChessPieceType, ColoredChessPieceType};
use crate::games::chess::squares::{ChessSquare, C_FILE_NO, D_FILE_NO, F_FILE_NO, G_FILE_NO};
use crate::games::chess::zobrist::PRECOMPUTED_ZOBRIST_KEYS;
use crate::games::chess::ChessColor::*;
use crate::games::chess::{ChessColor, Chessboard};
use crate::games::{
    char_to_file, file_to_char, AbstractPieceType, Board, Color, ColoredPiece, ColoredPieceType,
    DimT, ZobristHash,
};
use crate::general::bitboards::chess::ChessBitboard;
use crate::general::bitboards::{Bitboard, RawBitboard};
use crate::general::common::Res;
use crate::general::moves::ExtendedFormat::Standard;
use crate::general::moves::Legality::PseudoLegal;
use crate::general::moves::{ExtendedFormat, Legality, Move, MoveFlags, UntrustedMove};
use crate::general::squares::RectangularCoordinates;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Default, Debug, EnumIter, FromRepr)]
#[must_use]
pub enum ChessMoveFlags {
    #[default]
    NormalPawnMove,
    KnightMove,
    BishopMove,
    RookMove,
    QueenMove,
    NormalKingMove,
    CastleKingside,
    CastleQueenside,
    EnPassant,
    PromoKnight,
    PromoBishop,
    PromoRook,
    PromoQueen,
}

impl ChessMoveFlags {
    pub fn normal_move(piece: ChessPieceType) -> Self {
        Self::from_repr(piece as usize).unwrap()
    }

    fn is_promo(self) -> bool {
        self >= PromoKnight
    }

    fn promo_piece(self) -> ChessPieceType {
        if self < PromoKnight {
            Empty
        } else {
            ChessPieceType::from_repr(self as usize - PromoKnight as usize + Knight as usize)
                .unwrap()
        }
    }

    pub fn piece_type(self) -> ChessPieceType {
        if self <= NormalKingMove {
            ChessPieceType::from_repr(self as usize).unwrap()
        } else if self >= EnPassant {
            Pawn
        } else {
            King
        }
    }
}

impl MoveFlags for ChessMoveFlags {}

/// Members are stored as follows:
/// Bits 0-5: from square
/// Bits 6 - 11: To square
/// Bits 12-15: Move type
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Ord, PartialOrd, Hash, Arbitrary)]
#[must_use]
#[repr(C)]
pub struct ChessMove(u16);

impl ChessMove {
    pub fn new(from: ChessSquare, to: ChessSquare, flags: ChessMoveFlags) -> Self {
        let idx = from.bb_idx() + (to.bb_idx() << 6) + ((flags as usize) << 12);
        Self(idx as u16)
    }

    pub const NULL: Self = Self(0);

    pub fn square_of_pawn_taken_by_ep(self) -> Option<ChessSquare> {
        // TODO: Use board.ep_square instead
        if self.flags() != EnPassant {
            return None;
        }
        let to = self.dest_square();
        if to.rank() == 2 {
            Some(ChessSquare::from_rank_file(3, to.file()))
        } else {
            Some(ChessSquare::from_rank_file(4, to.file()))
        }
    }

    pub fn piece(self, board: &Chessboard) -> ChessPiece {
        let source = self.src_square();
        debug_assert!(board.is_occupied(source));
        debug_assert!(board.active_player_bb().is_bit_set_at(source.bb_idx()));
        ChessPiece::new(
            ColoredChessPieceType::new(board.active_player, self.flags().piece_type()),
            source,
        )
    }

    pub fn untrusted_flags(self) -> Res<ChessMoveFlags> {
        let flags = self.0 >> 12;
        if flags <= 12 {
            Ok(self.flags())
        } else {
            bail!(
                "Invalid flags {flags}, which means this move is never valid \
            and most likely the result of interpreting corrupt data as a chess move"
            )
        }
    }

    pub fn piece_type(self) -> ChessPieceType {
        self.flags().piece_type()
    }

    pub fn piece_type_on_target(self, board: &Chessboard) -> ChessPieceType {
        board.piece_type_on(self.dest_square())
    }

    pub fn is_capture(self, board: &Chessboard) -> bool {
        self.is_ep() || self.is_non_ep_capture(board)
    }

    pub fn is_ep(self) -> bool {
        // TODO: Don't store that as flag, use board.ep_square
        self.flags() == EnPassant
    }

    pub fn is_non_ep_capture(self, board: &Chessboard) -> bool {
        board
            .colored_bb(board.active_player.other())
            .is_bit_set_at(self.dest_square().bb_idx())
    }

    pub fn captured(self, board: &Chessboard) -> ChessPieceType {
        if self.is_ep() {
            Pawn
        } else if self.is_castle() {
            Empty
        } else {
            board.piece_type_on(self.dest_square())
        }
    }

    pub fn is_promotion(self) -> bool {
        self.flags().is_promo()
    }

    pub fn is_double_pawn_push(self) -> bool {
        self.piece_type() == Pawn
            && self.dest_square().rank().abs_diff(self.src_square().rank()) == 2
    }

    pub fn promo_piece(self) -> ChessPieceType {
        self.flags().promo_piece()
    }

    pub fn is_castle(self) -> bool {
        self.flags() == CastleQueenside || self.flags() == CastleKingside
    }

    pub fn castle_side(self) -> CastleRight {
        debug_assert!(self.is_castle());
        if self.flags() == CastleQueenside {
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
        self.format_compact(f)
    }
}

impl Move<Chessboard> for ChessMove {
    type Flags = ChessMoveFlags;
    type Underlying = u16;

    fn legality() -> Legality {
        PseudoLegal
    }

    fn src_square(self) -> ChessSquare {
        ChessSquare::from_bb_index((self.0 & 0x3f) as usize)
    }

    fn dest_square(self) -> ChessSquare {
        ChessSquare::from_bb_index(((self.0 >> 6) & 0x3f) as usize)
    }

    fn flags(self) -> ChessMoveFlags {
        ChessMoveFlags::iter().nth((self.0 >> 12) as usize).unwrap()
    }

    fn is_tactical(self, board: &Chessboard) -> bool {
        self.is_capture(board) || self.flags() == PromoQueen || self.flags() == PromoKnight
    }

    fn format_compact(self, f: &mut Formatter<'_>) -> fmt::Result {
        if self == Self::NULL {
            return write!(f, "0000");
        }
        let flag = match self.flags() {
            PromoKnight => "n",
            PromoBishop => "b",
            PromoRook => "r",
            PromoQueen => "q",
            _ => "",
        };
        write!(
            f,
            "{from}{to}{flag}",
            from = self.src_square(),
            to = self.dest_square()
        )
    }

    fn from_compact_text(s: &str, board: &Chessboard) -> Res<Self> {
        let s = s.trim();
        if s.is_empty() {
            bail!("Empty input");
        }
        // Need to check this before creating slices because splitting unicode character panics.
        if !s.is_ascii() {
            bail!("Move '{}' contains a non-ASCII character", s.red());
        }
        if s.len() < 4 {
            bail!("Move too short: '{s}'. Must be <from square><to square>, e.g. e2e4, and possibly a promotion piece.");
        }
        let from = ChessSquare::from_str(&s[..2])?;
        let mut to = ChessSquare::from_str(&s[2..4])?;
        let piece = board.colored_piece_on(from);
        let mut flags = ChessMoveFlags::normal_move(piece.uncolored());
        if s.len() > 4 {
            let promo = s.chars().nth(4).unwrap();
            match promo {
                'n' => flags = PromoKnight,
                'b' => flags = PromoBishop,
                'r' => flags = PromoRook,
                'q' => flags = PromoQueen,
                _ => bail!("Invalid character after to square: '{promo}'"),
            }
        } else if piece.uncolored() == King {
            let rook_capture = board.colored_piece_on(to).symbol
                == ColoredChessPieceType::new(piece.color().unwrap(), Rook);
            if rook_capture || to.file().abs_diff(from.file()) > 1 {
                let color = if from.rank() == 0 { White } else { Black };
                if !rook_capture {
                    // convert normal chess king-to castling notation to rook capture notation (necessary for chess960/DFRC)
                    let to_file = match to.file() {
                        C_FILE_NO => board.castling.rook_start_file(color, Queenside),
                        G_FILE_NO => board.castling.rook_start_file(color, Kingside),
                        _ => bail!("Invalid king move to square {to}, which is neither a normal king move nor a castling move")
                    };
                    to = ChessSquare::from_rank_file(to.rank(), to_file);
                }
                // handle KxR notation (e.g. e1h1 for kingside castling)
                flags = if to.file() == board.castling.rook_start_file(color, Queenside) {
                    CastleQueenside
                } else {
                    CastleKingside
                }
            }
        } else if piece.uncolored() == Pawn && board.is_empty(to) && from.file() != to.file() {
            flags = EnPassant;
        }
        let res = from.bb_idx() + (to.bb_idx() << 6) + ((flags as usize) << 12);
        let res = ChessMove(res as u16);
        if !board.is_move_pseudolegal(res) {
            bail!(
                "The move '{0}' is not (pseudo)legal in position '{board}'",
                s.red()
            )
        }
        Ok(res)
    }

    fn format_extended(
        self,
        f: &mut Formatter<'_>,
        board: &Chessboard,
        format: ExtendedFormat,
    ) -> fmt::Result {
        if self.is_castle() {
            return match self.castle_side() {
                Queenside => write!(f, "O-O-O"),
                Kingside => write!(f, "O-O"),
            };
        }
        let piece = self.piece(board);
        let mut res = match piece.uncolored() {
            Pawn => String::default(),
            uncolored => {
                if format == Standard {
                    uncolored.to_ascii_char().to_string()
                } else {
                    piece.to_utf8_char().to_string()
                }
            }
        };
        let mut from_str = if piece.uncolored() == Pawn && self.is_capture(board) {
            self.src_square()
                .to_string()
                .chars()
                .nth(0)
                .unwrap()
                .to_string()
        } else {
            String::default()
        };
        let moves = board
            // we have to use .pseudolegal instead of legal moves here because that's what the rules demand.
            .pseudolegal_moves()
            .into_iter()
            .filter(|mov| {
                mov.piece(board).symbol == piece.symbol
                    && mov.dest_square() == self.dest_square()
                    && mov.promo_piece() == self.promo_piece()
            })
            .collect_vec();
        if moves.is_empty() {
            return write!(f, "<Illegal move {}>", self);
        }

        if moves.len() > 1 {
            from_str = if moves
                .iter()
                .filter(|mov| mov.src_square().file() == self.src_square().file())
                .count()
                <= 1
            {
                self.src_square()
                    .to_string()
                    .chars()
                    .nth(0)
                    .unwrap()
                    .to_string()
            } else if moves
                .iter()
                .filter(|mov| mov.src_square().rank() == self.src_square().rank())
                .count()
                <= 1
            {
                self.src_square()
                    .to_string()
                    .chars()
                    .nth(1)
                    .unwrap()
                    .to_string()
            } else {
                self.src_square().to_string()
            }
        };
        res += &from_str;
        if self.is_capture(board) {
            res.push('x');
        }
        res += &self.dest_square().to_string();
        if self.is_promotion() {
            res.push('=');
            if format == Standard {
                res.push(self.flags().promo_piece().to_ascii_char());
            } else {
                res.push(self.flags().promo_piece().to_utf8_char());
            }
        }
        let board = board.make_move(self).unwrap();
        if board.is_game_lost_slow() {
            res.push('#');
        } else if board.is_in_check() {
            res.push('+');
        }
        write!(f, "{res}")
    }

    fn from_extended_text(s: &str, board: &Chessboard) -> Res<Self> {
        let res = MoveParser::parse(s, board)?;
        if !res.1.is_empty() {
            bail!("Additional input after move {0}: '{1}'", res.0, res.1);
        }
        Ok(res.0)
    }

    fn from_usize_unchecked(val: usize) -> UntrustedMove<Chessboard> {
        UntrustedMove::from_move(Self(val as u16))
    }

    fn to_underlying(self) -> Self::Underlying {
        self.0
    }

    // TODO: Parse pgn (not here though)
}

impl Chessboard {
    pub fn backrank(color: ChessColor) -> DimT {
        7 * color as DimT
    }

    pub fn rook_start_file(&self, color: ChessColor, side: CastleRight) -> DimT {
        self.castling.rook_start_file(color, side)
    }

    pub fn rook_start_square(&self, color: ChessColor, side: CastleRight) -> ChessSquare {
        let file = self.rook_start_file(color, side);
        let rank = Self::backrank(color);
        ChessSquare::from_rank_file(rank, file)
    }

    pub fn make_move_and_prefetch_tt<F: Fn(ZobristHash)>(
        self,
        mov: ChessMove,
        prefetch: F,
    ) -> Option<Self> {
        self.make_move_impl(mov, prefetch)
    }

    /// Is only ever called on a copy of the board, so no need to undo the changes when a move gets aborted due to pseudo-legality.
    #[allow(clippy::too_many_lines)]
    pub(super) fn make_move_impl<F: Fn(ZobristHash)>(
        mut self,
        mov: ChessMove,
        prefetch: F,
    ) -> Option<Self> {
        let piece = mov.piece_type();
        let mut new_hash = Self::approximate_zobrist_after_move(
            self.hash,
            self.active_player,
            piece,
            mov.src_square(),
            mov.dest_square(),
        );
        // this is only an approximation of the new hash, but that is good enough
        prefetch(new_hash ^ PRECOMPUTED_ZOBRIST_KEYS.side_to_move_key);
        prefetch(new_hash);
        debug_assert_eq!(piece, self.piece_type_on(mov.src_square()));
        let color = self.active_player;
        let other = color.other();
        let from = mov.src_square();
        let mut to = mov.dest_square();
        debug_assert_eq!(color, mov.piece(&self).color().unwrap());
        self.ply_100_ctr += 1;
        // remove old castling flags
        new_hash ^=
            PRECOMPUTED_ZOBRIST_KEYS.castle_keys[self.castling.allowed_castling_directions()];
        if let Some(square) = self.ep_square {
            new_hash ^= PRECOMPUTED_ZOBRIST_KEYS.ep_file_keys[square.file() as usize];
        }
        self.ep_square = None;
        if mov.is_castle() {
            let from_file = from.file() as isize;
            let rook_file = to.file() as isize;
            let (side, to_file, rook_to_file) = if mov.flags() == CastleKingside {
                (Kingside, G_FILE_NO, F_FILE_NO)
            } else {
                (Queenside, C_FILE_NO, D_FILE_NO)
            };
            debug_assert_eq!(
                side == Kingside,
                self.castling.can_castle(color, Kingside)
                    && rook_file == self.castling.rook_start_file(color, Kingside) as isize
            );
            debug_assert_eq!(
                side == Queenside,
                self.castling.can_castle(color, Queenside)
                    && rook_file == self.castling.rook_start_file(color, Queenside) as isize
            );

            // Explicitly test if the current square is in check in case the following for loop is empty
            // because the king doesn't move -- in that case, testing for check after the castle might obscure the
            // check with the rook, e.g. black in 'rbbqQ1kr/1p2p1pp/p5n1/2pp1p2/2P4P/P7/BP1PPPP1/R1B1NNKR b HAha - 0 10'
            if self.is_in_check() {
                return None;
            }
            let step = if side == Kingside { 1 } else { -1 };
            // no need to test for check on the target square as that will be done at the end of this function after
            // the rook has moved
            for file in iter::range_step(from_file + step, to_file as isize, step) {
                if self.is_in_check_on_square(
                    color,
                    ChessSquare::from_rank_file(from.rank(), file as DimT),
                ) {
                    return None;
                }
            }
            let rook_from = self.rook_start_square(color, side);
            let rook_to = ChessSquare::from_rank_file(from.rank(), rook_to_file);
            debug_assert!(
                self.colored_piece_on(rook_from).symbol == ColoredChessPieceType::new(color, Rook)
            );
            self.move_piece(rook_from, rook_to, Rook);
            new_hash ^= PRECOMPUTED_ZOBRIST_KEYS.piece_key(Rook, color, rook_to);
            new_hash ^= PRECOMPUTED_ZOBRIST_KEYS.piece_key(Rook, color, rook_from);
            new_hash ^= PRECOMPUTED_ZOBRIST_KEYS.piece_key(King, color, to);
            to = ChessSquare::from_rank_file(from.rank(), to_file);
            new_hash ^= PRECOMPUTED_ZOBRIST_KEYS.piece_key(King, color, to);
        } else if mov.is_ep() {
            let taken_pawn = mov.square_of_pawn_taken_by_ep().unwrap();
            debug_assert_eq!(
                self.colored_piece_on(taken_pawn).symbol,
                ColoredChessPieceType::new(other, Pawn)
            );
            self.remove_piece_unchecked(taken_pawn, Pawn, other);
            new_hash ^= PRECOMPUTED_ZOBRIST_KEYS.piece_key(Pawn, other, taken_pawn);
            self.ply_100_ctr = 0;
        } else if mov.is_non_ep_capture(&self) {
            let captured = self.piece_type_on(to);
            debug_assert_eq!(self.colored_piece_on(to).color().unwrap(), other);
            debug_assert_ne!(captured, King);
            self.remove_piece_unchecked(to, captured, other);
            new_hash ^= PRECOMPUTED_ZOBRIST_KEYS.piece_key(captured, other, to);
            self.ply_100_ctr = 0;
        } else if piece == Pawn {
            self.ply_100_ctr = 0;
            let possible_ep_pawns =
                (to.bb().west() | to.bb().east()) & self.colored_piece_bb(other, Pawn);
            if from.rank().abs_diff(to.rank()) == 2 && possible_ep_pawns.has_set_bit() {
                self.ep_square = Some(ChessSquare::from_rank_file(
                    (to.rank() + from.rank()) / 2,
                    to.file(),
                ));
                new_hash ^= PRECOMPUTED_ZOBRIST_KEYS.ep_file_keys[to.file() as usize];
            }
        }
        if piece == King {
            self.castling.clear_castle_rights(color);
        } else if from == self.rook_start_square(color, Queenside) {
            self.castling.unset_castle_right(color, Queenside);
        } else if from == self.rook_start_square(color, Kingside) {
            self.castling.unset_castle_right(color, Kingside);
        }
        if to == self.rook_start_square(other, Queenside) {
            self.castling.unset_castle_right(other, Queenside);
        } else if to == self.rook_start_square(other, Kingside) {
            self.castling.unset_castle_right(other, Kingside);
        }
        new_hash ^=
            PRECOMPUTED_ZOBRIST_KEYS.castle_keys[self.castling.allowed_castling_directions()];
        self.move_piece(from, to, piece);
        if mov.is_promotion() {
            let bb = to.bb().raw();
            self.piece_bbs[Pawn as usize] ^= bb;
            self.piece_bbs[mov.flags().promo_piece() as usize] ^= bb;
            new_hash ^= PRECOMPUTED_ZOBRIST_KEYS.piece_key(Pawn, color, to);
            new_hash ^= PRECOMPUTED_ZOBRIST_KEYS.piece_key(mov.flags().promo_piece(), color, to);
        }
        self.ply += 1;
        self.hash = new_hash;
        self.flip_side_to_move()
    }

    /// Called at the end of `make_nullmove` and `make_move`.
    #[must_use]
    pub fn flip_side_to_move(mut self) -> Option<Self> {
        if self.is_in_check() {
            None
        } else {
            self.active_player = self.active_player.other();
            Some(self)
        }
    }
}

/// A lenient parser that can parse a move in short or long algebraic notation, intended to be used for human input.
pub struct MoveParser<'a> {
    original_input: &'a str,
    num_bytes_read: usize,
    start_rank: Option<DimT>,
    start_file: Option<DimT>,
    target_rank: Option<DimT>,
    target_file: Option<DimT>,
    piece: ChessPieceType,
    is_capture: bool,
    is_ep: bool,
    gives_check: bool,
    gives_mate: bool,
    promotion: ChessPieceType,
}

impl<'a> MoveParser<'a> {
    fn new(original_input: &'a str) -> Self {
        Self {
            original_input,
            num_bytes_read: 0,
            start_rank: None,
            start_file: None,
            target_rank: None,
            target_file: None,
            piece: Empty,
            is_capture: false,
            is_ep: false,
            gives_check: false,
            gives_mate: false,
            promotion: Empty,
        }
    }

    pub fn parse(input: &'a str, board: &Chessboard) -> Res<(ChessMove, &'a str)> {
        let mut parser = MoveParser::new(input);
        if let Some(mov) = parser.parse_castling(board) {
            parser.parse_check_mate();
            parser.parse_annotation();
            if !board.is_move_pseudolegal(mov) {
                // can't use `to_extended_text` because that requires pseudolegal moves.
                bail!(
                    "Castling move '{}' is not pseudolegal in the current position",
                    mov.to_string().red()
                );
            }
            parser.check_check_checkmate_captures_and_ep(mov, board)?; // check this once the move is known to be pseudolegal
            return Ok((mov, parser.remaining()));
        }
        parser.parse_piece()?;
        parser.parse_maybe_capture()?;
        parser.parse_square_rank_or_file()?;
        parser.parse_maybe_capture()?;
        parser.parse_second_square();
        parser.parse_maybe_capture()?;
        parser.parse_promotion()?;
        parser.parse_ep();
        parser.parse_check_mate();
        parser.parse_ep();
        parser.parse_annotation();
        let remaining = parser.remaining();
        let mov = parser.into_move(board)?;
        // this also consumes the character after the move if it exists, but that's probably fine
        // (I wonder at what point it will turn out to not be fine)
        Ok((mov, remaining))
    }

    fn consumed(&self) -> &'a str {
        &self.original_input[..self.num_bytes_read]
    }

    fn remaining(&self) -> &'a str {
        &self.original_input[self.num_bytes_read..]
    }

    fn current_char(&mut self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn advance_char(&mut self) {
        if let Some(c) = self.current_char() {
            self.num_bytes_read += c.len_utf8();
        }
    }

    fn ignore_whitespace(&mut self) {
        while self.current_char().is_some_and(char::is_whitespace) {
            self.advance_char();
        }
    }

    fn parse_str_dont_consume_last_char(&mut self, s: &str) -> bool {
        if self.remaining().starts_with(s) {
            let mut chars = s.chars().peekable();
            // Consume one character less.
            // This makes it easier to use this function as part of an if that otherwise only checks a single character
            while chars.next().is_some() {
                if chars.peek().is_some() {
                    self.advance_char();
                }
            }
            return true;
        }
        false
    }

    fn parse_castling(&mut self, board: &Chessboard) -> Option<ChessMove> {
        let color = board.active_player;
        let king_square = board.king_square(color);
        if self.original_input.starts_with("0-0-0") || self.original_input.starts_with("O-O-O") {
            for _ in 0..5 {
                self.advance_char();
            }
            return Some(ChessMove::new(
                king_square,
                ChessSquare::from_rank_file(
                    king_square.rank(),
                    board.castling.rook_start_file(color, Queenside),
                ),
                CastleQueenside,
            ));
        }
        if self.original_input.starts_with("0-0") || self.original_input.starts_with("O-O") {
            for _ in 0..3 {
                self.advance_char();
            }
            return Some(ChessMove::new(
                king_square,
                ChessSquare::from_rank_file(
                    king_square.rank(),
                    board.castling.rook_start_file(color, Kingside),
                ),
                CastleKingside,
            ));
        }
        None
    }

    fn parse_piece(&mut self) -> Res<()> {
        // Almost completely ignore unicode piece colors -- uncolored pieces are almost never used, so it's normal to use
        // white unicode symbols for black pieces. This also allows the user to enter ascii algebraic notation without
        // needing to worry about capitalization.
        // However, bishops can introduce ambiguity when ignoring case because b4 could refer to a square or a bishop on the 4th rank.
        // For example, the input `b4xe5` could refer to a pawn on e4 capturing on e5, or (very unlikely but possible)
        // to a bishop on the 4th rank capturing on e5 while there's another bishop on the same file but different rank that could also capture on e5.
        // To handle this, 'b' is assumed to never refer to a bishop (but `B`, '🨃', '♗' and '♝' always refer to bishops).
        // The same is true for 'D' in German notation.
        let Some(current) = self.current_char() else {
            bail!("Empty move");
        };
        match current {
            'a'..='h' | 'A' | 'C' | 'E'..='H' | 'x' | ':' | '×' => (),
            _ => {
                self.piece = ColoredChessPieceType::from_utf8_char(current)
                    .map(ColoredChessPieceType::uncolor)
                    .or_else(|| ChessPieceType::from_utf8_char(current))
                    .ok_or_else(|| {
                        anyhow!("The move starts with '{current}', which is not a piece or file")
                    })?;
                self.advance_char();
            }
        };
        Ok(())
    }

    fn parse_maybe_capture(&mut self) -> Res<()> {
        match self.current_char() {
            None => Ok(()),
            Some(c) => {
                if matches!(c, 'x' | ':' | '×') {
                    if self.is_capture {
                        bail!("Multiple capture symbols");
                    }
                    self.is_capture = true;
                    self.advance_char();
                }
                Ok(())
            }
        }
    }

    fn parse_square_rank_or_file(&mut self) -> Res<()> {
        let Some(file) = self.current_char() else {
            bail!("Move '{}' is too short", self.consumed().red())
        };
        self.advance_char();
        let Some(rank) = self.current_char() else {
            bail!("Move '{}' is too short", self.consumed().red())
        };
        match ChessSquare::from_chars(file, rank) {
            Ok(sq) => {
                self.advance_char();
                self.start_file = Some(sq.file());
                self.start_rank = Some(sq.rank());
            }
            Err(_) => match file {
                'a'..='h' => self.start_file = Some(char_to_file(file)),
                '1'..='8' => self.start_rank = Some(file as DimT - b'1'),
                x => {
                    // doesn't reset the current char, but that's fine because we're aborting anyway
                    if self.piece == Empty && !self.is_capture {
                        bail!("A move must start with a valid file, rank or piece, but '{}' is neither", x.to_string().red())
                    } else {
                        bail!("'{}' is not a valid file or rank", x.to_string().red())
                    }
                }
            },
        }
        Ok(())
    }

    // The second square is the target square, which must always be a complete square (as opposed to only being a row / column of omitted)
    // except for pawn captures.
    fn parse_second_square(&mut self) {
        let read_so_far = self.num_bytes_read;
        let file = self.current_char();
        self.advance_char();
        let rank = self.current_char();
        if file.is_some() && rank.is_some() {
            if let Ok(square) = ChessSquare::from_chars(file.unwrap(), rank.unwrap()) {
                self.advance_char();
                self.target_file = Some(square.file());
                self.target_rank = Some(square.rank());
                return;
            }
        }
        if self.piece == Empty && file.is_some() && matches!(file.unwrap(), 'a'..='h') {
            self.target_file = file.map(char_to_file);
            return;
        }
        self.num_bytes_read = read_so_far;
    }

    fn parse_ep(&mut self) {
        self.ignore_whitespace();
        if self.current_char().is_some_and(|c| c == 'e') {
            let read_so_far = self.num_bytes_read;
            self.advance_char();
            if self.current_char().is_some_and(|c| c == '.') {
                self.advance_char();
            }
            self.ignore_whitespace();
            if self.current_char().is_some_and(|c| c == 'p') {
                self.advance_char();
                if self.current_char().is_some_and(|c| c == '.') {
                    self.advance_char();
                }
                self.is_ep = true;
                return;
            }
            self.num_bytes_read = read_so_far;
        }
    }

    fn parse_promotion(&mut self) -> Res<()> {
        let mut allow_fail = true;
        if self.current_char().is_some_and(|c| c == '=') {
            self.advance_char();
            allow_fail = false;
        }
        let piece = self.current_char().and_then(|c| {
            ColoredChessPieceType::from_utf8_char(c)
                .map(ColoredChessPieceType::uncolor)
                .or_else(|| ChessPieceType::from_utf8_char(c))
        });
        if piece.is_some() {
            self.promotion = piece.unwrap();
            self.advance_char();
        } else if !allow_fail {
            bail!("Missing promotion piece after '='");
        }
        Ok(())
    }

    fn parse_check_mate(&mut self) {
        self.ignore_whitespace();
        assert!(!self.gives_check); // the implementation relies on the fact that this function is only called once per move
        if self.current_char().is_some_and(|c| {
            matches!(c, '#' | '‡')
                || self.parse_str_dont_consume_last_char("mate")
                || self.parse_str_dont_consume_last_char("checkmate")
        }) {
            self.advance_char();
            self.gives_mate = true;
            self.gives_check = true;
        } else if self.current_char().is_some_and(|c| {
            matches!(c, '+' | '†')
                // test for 'check' before 'ch' because otherwise 'ch' would accept for input 'check' and 'eck' would remain.
                || self.parse_str_dont_consume_last_char("check")
                || self.parse_str_dont_consume_last_char("ch")
        }) {
            let parsed_plus = self.current_char().unwrap() == '+';
            self.advance_char();
            self.gives_check = true;
            if parsed_plus
                && self
                    .current_char()
                    .is_some_and(|c| matches!(c, '/' | '-' | '='))
            {
                // actually not a check, but a position evaluation (which gets ignored, so no need to undo the parsing)
                self.gives_check = false;
            }
        }
    }

    fn parse_annotation(&mut self) {
        self.ignore_whitespace();
        let annotation_chars = [
            '!', '?', '⌓', '□', ' ', '⩲', '⩱', '±', '∓', '∞', '/', '+', '-', '=',
        ];
        while self
            .current_char()
            .is_some_and(|c| annotation_chars.contains(&c))
        {
            self.advance_char();
        }
    }

    fn into_move(mut self, board: &Chessboard) -> Res<ChessMove> {
        assert!(self.start_file.is_some() || self.start_rank.is_some());
        if self.target_file.is_none() && self.target_rank.is_none() {
            self.target_file = self.start_file;
            self.target_rank = self.start_rank;
            self.start_file = None;
            self.start_rank = None;
        }

        // assert_ne!(self.piece, Pawn); // Pawns aren't written as `p` in SAN, but the parser still accepts this.
        let original_piece = self.piece;
        if self.piece == Empty {
            self.piece = Pawn;
        }

        if self.target_file.is_none() {
            bail!(
                "Missing the file of the target square in move '{}'",
                self.consumed()
            );
        }
        if self.piece != Pawn && self.target_rank.is_none() {
            bail!(
                "Missing the rank of the target square in move '{}'",
                self.consumed()
            );
        }

        let mut moves = board.pseudolegal_moves().into_iter().filter(|mov| {
            mov.piece_type() == self.piece
                && mov.dest_square().file() == self.target_file.unwrap()
                && !self
                    .target_rank
                    .is_some_and(|r| r != mov.dest_square().rank())
                && !self
                    .start_file
                    .is_some_and(|f| f != mov.src_square().file())
                && !self
                    .start_rank
                    .is_some_and(|r| r != mov.src_square().rank())
                && self.promotion == mov.promo_piece()
                && board.is_pseudolegal_move_legal(*mov)
        });
        let res = match moves.next() {
            None => {
                // invalid move, try to print a helpful error message
                let f = |file: Option<DimT>, rank: Option<DimT>| {
                    if let Some(file) = file {
                        match rank {
                            Some(rank) => {
                                let square = ChessSquare::from_rank_file(rank, file);
                                (square.to_string(), square.bb())
                            }
                            None => (
                                format!("the {} file", file_to_char(file)),
                                ChessBitboard::file_no(file),
                            ),
                        }
                    } else if let Some(rank) = rank {
                        (format!("rank {}", rank), ChessBitboard::rank_no(rank))
                    } else {
                        ("any square".to_string(), !ChessBitboard::default())
                    }
                };
                let (from, from_bb) = f(self.start_file, self.start_rank);
                let to = f(self.target_file, self.target_rank).0;
                let (from, to) = (from.bold(), to.bold());
                let mut additional = String::new();
                if board.is_game_lost_slow() {
                    additional = format!(" ({} has been checkmated)", board.active_player);
                } else if board.is_in_check() {
                    additional = format!(" ({} is in check)", board.active_player);
                }
                // moves without a piece but source and dest square have probably been meant as UCI moves, and not as pawn moves
                if original_piece == Empty && from_bb.is_single_piece() {
                    let piece = board.colored_piece_on(from_bb.to_square().unwrap());
                    if piece.is_empty() {
                        bail!(
                            "The square {from} is empty, so the move '{}' is invalid{1}",
                            self.consumed().bold(),
                            additional
                        )
                    } else if piece.color().unwrap() != board.active_player {
                        bail!("There is a {0} on {from}, but it's {1}'s turn to move, so the move '{2}' is invalid{3}",
                            piece.symbol.name(), board.active_player, self.consumed().bold(), additional)
                    } else {
                        bail!("There is a {0} on {from}, but it can't move to {to}, so the move '{1}' is invalid{2}",
                            piece.symbol.name(), self.consumed().bold(), additional)
                    }
                }
                if (board.colored_piece_bb(board.active_player, self.piece) & from_bb).is_zero() {
                    bail!(
                        "There is no {0} {1} on {from}, so the move '{2}' is invalid{3}",
                        board.active_player,
                        self.piece.name(),
                        self.consumed().bold(),
                        additional
                    )
                } else {
                    bail!(
                        "There is no legal {0} {1} move from {from} to {to}, so the move '{2}' is invalid{3}",
                        board.active_player,
                        self.piece.name(),
                        self.consumed().bold(),
                        additional
                    );
                }
            }
            Some(mov) => {
                if let Some(other) = moves.next() {
                    bail!(
                        "Move '{0}' is ambiguous, because it could refer to {1} or {2}",
                        self.consumed(),
                        mov.to_extended_text(board, Standard),
                        other.to_extended_text(board, Standard)
                    );
                }
                mov
            }
        };

        self.check_check_checkmate_captures_and_ep(res, board)?;

        debug_assert!(board.is_move_legal(res));
        Ok(res)
    }

    // I love this name
    // assumes that the move has already been verified to be pseudolegal. TODO: Encode in type system
    fn check_check_checkmate_captures_and_ep(&self, mov: ChessMove, board: &Chessboard) -> Res<()> {
        let incorrect_mate = self.gives_mate && !board.is_game_won_after_slow(mov);
        let incorrect_check = self.gives_check && !board.gives_check(mov);
        let incorrect_capture = self.is_capture && !mov.is_capture(board);
        // Missing check / checkmate signs or ep annotations are ok, but incorrect ones aren't
        if (self.is_ep && mov.flags() != EnPassant)
            || incorrect_mate
            || incorrect_check
            || incorrect_capture
        {
            let typ = if incorrect_mate {
                "delivers checkmate"
            } else if incorrect_check {
                "gives check"
            } else if incorrect_capture {
                "captures something"
            } else {
                "captures en passant"
            };
            bail!(
                "The move notation '{0}' claims that it {typ}, but the move {1} actually doesn't",
                self.consumed().red(),
                mov.to_string().bold() // can't use to_extended_text() here, as that requires pseudolegal moves
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::games::chess::moves::ChessMove;
    use crate::games::chess::pieces::ChessPieceType::Bishop;
    use crate::games::chess::squares::ChessSquare;
    use crate::games::chess::ChessColor::White;
    use crate::games::chess::Chessboard;
    use crate::games::generic_tests;
    use crate::games::Board;
    use crate::general::board::Strictness::Strict;
    use crate::general::moves::ExtendedFormat::{Alternative, Standard};
    use crate::general::moves::Move;

    type GenericTests = generic_tests::GenericTests<Chessboard>;

    #[test]
    fn valid_algebraic_notation_test() {
        let transformations = [
            ("Na1", "Na1"),
            ("nxA7 mate", "Nxa7#"),
            ("RC1:", "Rxc1"),
            ("e2e4", "e4"),
            ("e8D", "e8=Q"),
            ("e5f6:e.p.", "exf6"),
            ("ef:e.p.", "exf6"),
            //("f:e.p.", "exf6"), // TODO: Make this work?
            ("e:fep", "exf6"),
            ("b:", "axb5"),
            ("🨅e4", "e4"),
            ("♚f2", "Kf2"),
            ("♖b8+", "Rb8+"),
            ("Rb7d7", "Rbd7"), // even though the move Rd1d7 isn't legal, it's still necessary to disambiguate with Rbd7
            ("gf8:🨂", "gxf8=R"),
            (":d8🨂 checkmate", "exd8=R#"),
            ("exf♘", "exf8=N"),
            ("gf:♝", "gxf8=B"),
            ("xf5", "gxf5"),
            ("Ra7", "Rxa7"),
            ("rB8", "Rb8+"),
            ("nA7+", "Nxa7#"),
            ("N3a5", "Nba5"),
        ];
        let pos = Chessboard::from_name("unusual").unwrap();
        for (input, output) in transformations {
            let mov = ChessMove::from_extended_text(input, &pos).unwrap();
            assert_eq!(mov.to_extended_text(&pos, Standard), output);
            assert_eq!(
                ChessMove::from_extended_text(&mov.to_extended_text(&pos, Alternative), &pos)
                    .unwrap(),
                mov
            );
        }
    }

    #[test]
    fn failed_test() {
        let pos = Chessboard::from_fen("8/7r/8/K1k5/8/8/4p3/8 b - - 10 11", Strict).unwrap();
        let mov = ChessMove::from_extended_text("e1=Q+", &pos).unwrap();
        assert!(pos.is_move_legal(mov));
    }

    #[test]
    fn invalid_algebraic_notation_test() {
        let inputs = [
            "resign",
            "Robert'); DROP TABLE Students;--",
            "Raa",
            "R4",
            "Raaa4",
            "Qi1",
            "Ra8D",
            "f e.p.",
            "O-O-O-O",
            ":f8🨂", // ambiguous
            "Rb8#", // check but not checkmate
            "Rd2",  // only pseudolegal
            "e3+",  // doesn't give check
            "a2aß", // non-ASCII character in an unexpected position, mut not panic
        ];
        let pos = Chessboard::from_name("unusual").unwrap();
        for input in inputs {
            assert!(ChessMove::from_extended_text(input, &pos).is_err());
        }
    }

    #[test]
    fn invalid_moves_test() {
        // moves (except for 0) have been found through cargo fuzz
        let moves = [0, 60449, 38481, 28220];
        for mov in moves {
            let mov = ChessMove::from_usize_unchecked(mov);
            for pos in Chessboard::bench_positions() {
                if let Some(mov) = mov.check_pseudolegal(&pos) {
                    println!("{pos} -- {mov} {}", mov.flags() as usize);
                    pos.make_move(mov);
                    // check that the move representation is unique
                    assert!(
                        pos.pseudolegal_moves().contains(&mov),
                        "{pos} -- {mov} {}",
                        mov.flags() as usize
                    );
                }
            }
        }
    }

    #[test]
    fn algebraic_notation_roundtrip_test() {
        GenericTests::long_notation_roundtrip_test();
    }

    #[test]
    fn castle_test() {
        let mut p = Chessboard::chess_960_startpos(42).unwrap();
        p.remove_piece_unchecked(ChessSquare::from_chars('f', '1').unwrap(), Bishop, White);
        let tests: &[(Chessboard, &[&str])] = &[
            (
                Chessboard::from_name("kiwipete").unwrap(),
                &["0-0", "0-0-0", "e1g1", "e1h1", "e1a1", "e1c1"],
            ),
            (p, &["0-0", "g1h1"]),
        ];
        for (pos, moves) in tests {
            for mov in *moves {
                let mov = ChessMove::from_text(mov, pos);
                if let Err(err) = mov {
                    eprintln!("{err}");
                    panic!();
                }
                let mov = mov.unwrap();
                assert!(mov.is_castle());
                assert!(!mov.is_capture(pos));
            }
        }
    }
}

// TODO: PGN import test (not here though)
