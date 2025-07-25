use anyhow::{anyhow, bail};
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::str::FromStr;

use arbitrary::Arbitrary;
use colored::{ColoredString, Colorize};
use itertools::Itertools;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, FromRepr};

use crate::games::chess::ChessColor::*;
use crate::games::chess::castling::CastleRight;
use crate::games::chess::castling::CastleRight::*;
use crate::games::chess::moves::ChessMoveFlags::*;
use crate::games::chess::pieces::ChessPieceType::*;
use crate::games::chess::pieces::{ChessPiece, ChessPieceType, ColoredChessPieceType};
use crate::games::chess::squares::{C_FILE_NUM, ChessSquare, ChessboardSize, D_FILE_NUM, F_FILE_NUM, G_FILE_NUM};
use crate::games::chess::zobrist::ZOBRIST_KEYS;
use crate::games::chess::{ChessColor, ChessSettings, Chessboard};
use crate::games::{
    AbstractPieceType, Board, CharType, Color, ColoredPiece, ColoredPieceType, DimT, NoHistory, PosHash, char_to_file,
    file_to_char,
};
use crate::general::bitboards::chessboard::ChessBitboard;
use crate::general::bitboards::{Bitboard, KnownSizeBitboard, RawBitboard};
use crate::general::board::{BitboardBoard, BoardHelpers};
use crate::general::common::Res;
use crate::general::moves::ExtendedFormat::Standard;
use crate::general::moves::Legality::PseudoLegal;
use crate::general::moves::{ExtendedFormat, Legality, Move, UntrustedMove};
use crate::general::squares::RectangularCoordinates;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Default, Debug, EnumIter, FromRepr)]
#[must_use]
pub enum ChessMoveFlags {
    #[default]
    NormalMove,
    DoublePawnPush,
    CastleKingside,
    CastleQueenside,
    EnPassant,
    PromoKnight,
    PromoBishop,
    PromoRook,
    PromoQueen,
}

impl ChessMoveFlags {
    fn is_promo(self) -> bool {
        self >= PromoKnight
    }

    fn promo_piece(self) -> ChessPieceType {
        if self < PromoKnight {
            Empty
        } else {
            ChessPieceType::from_repr(self as usize - PromoKnight as usize + Knight as usize).unwrap()
        }
    }
}

/// Members are stored as follows:
/// Bits 0-5: from square
/// Bits 6 - 11: To square
/// Bits 12-15: Move type
#[derive(Copy, Clone, Eq, PartialEq, Default, Ord, PartialOrd, Hash, Arbitrary)]
#[must_use]
#[repr(C)]
pub struct ChessMove(u16);

impl Debug for ChessMove {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ChessMove({0}{1}-{2:?})", self.src_square(), self.dest_square(), self.flags())
    }
}

impl ChessMove {
    pub fn new(from: ChessSquare, to: ChessSquare, flags: ChessMoveFlags) -> Self {
        let idx = from.bb_idx() + (to.bb_idx() << 6) + ((flags as usize) << 12);
        Self(idx as u16)
    }

    pub const NULL: Self = Self(0);

    #[inline]
    pub fn src_square(self) -> ChessSquare {
        ChessSquare::from_bb_idx((self.0 & 0x3f) as usize)
    }

    #[inline]
    /// For a castle move, this always returns the rook square, which allows disambiguating Chess960 castling moves.
    pub fn dest_square(self) -> ChessSquare {
        ChessSquare::from_bb_idx(((self.0 >> 6) & 0x3f) as usize)
    }

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
        debug_assert!(board.is_occupied(source), "{}", self.compact_formatter(board));
        debug_assert!(board.active_player_bb().is_bit_set(source), "{}", self.compact_formatter(board));
        ChessPiece::new(ColoredChessPieceType::new(board.active, self.piece_type(board)), source)
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

    pub fn piece_type(self, board: &Chessboard) -> ChessPieceType {
        board.piece_type_on(self.src_square())
    }

    pub fn piece_type_on_target(self, board: &Chessboard) -> ChessPieceType {
        board.piece_type_on(self.dest_square())
    }

    #[inline]
    pub fn is_capture(self, board: &Chessboard) -> bool {
        self.is_non_ep_capture(board) || self.is_ep()
    }

    pub fn is_ep(self) -> bool {
        self.flags() == EnPassant
    }

    pub fn is_non_ep_capture(self, board: &Chessboard) -> bool {
        board.inactive_player_bb().is_bit_set(self.dest_square())
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
        self.flags() == DoublePawnPush
    }

    pub fn promo_piece(self) -> ChessPieceType {
        self.flags().promo_piece()
    }

    pub fn is_castle(self) -> bool {
        self.flags() == CastleQueenside || self.flags() == CastleKingside
    }

    pub fn castle_side(self) -> CastleRight {
        debug_assert!(self.is_castle());
        if self.flags() == CastleQueenside { Queenside } else { Kingside }
    }

    pub(super) fn flags(self) -> ChessMoveFlags {
        ChessMoveFlags::iter().nth((self.0 >> 12) as usize).unwrap_or_default()
    }

    pub fn from_to_square(self) -> usize {
        (self.0 & 0xfff) as usize
    }
}

impl Move<Chessboard> for ChessMove {
    type Underlying = u16;

    #[inline]
    fn legality(_: &ChessSettings) -> Legality {
        PseudoLegal
    }

    #[inline]
    fn src_square_in(self, _pos: &Chessboard) -> Option<ChessSquare> {
        Some(self.src_square())
    }

    #[inline]
    fn dest_square_in(self, _pos: &Chessboard) -> ChessSquare {
        self.dest_square()
    }

    #[inline]
    fn is_tactical(self, board: &Chessboard) -> bool {
        self.is_capture(board) || self.flags() == PromoQueen || self.flags() == PromoKnight
    }

    fn description(self, board: &Chessboard) -> String {
        let from = self.src_square().to_string().bold();
        let to = self.dest_square().to_string().bold();
        let piece = self.piece(board).to_string().bold();
        if self.is_castle() {
            format!("Castle {}", self.castle_side())
        } else if self.is_ep() {
            format!(
                "Capture the pawn on {0} with the {piece} on {0} {1}",
                self.dest_square().pawn_advance_unchecked(board.inactive_player()),
                "en passant".bold()
            )
        } else if self.is_promotion() {
            let promo = self.promo_piece().to_name().bold();
            if self.is_capture(board) {
                let victim = self.captured(board).to_string().bold();
                return format!("Capture the {victim} on {to} with the {piece} on {from} and promote it to a {promo}");
            }
            format!("Promote the {piece} on {from} to a {promo} on {to}")
        } else if self.is_capture(board) {
            let victim = self.captured(board).to_string().bold();
            format!("Capture the {victim} on {to} with the {piece} on {from}")
        } else {
            format!("Move the {piece} on {from} to {to}")
        }
    }

    #[inline]
    fn format_compact(self, f: &mut Formatter<'_>, board: &Chessboard) -> fmt::Result {
        if self.is_null() {
            return write!(f, "0000");
        }
        let mut to = self.dest_square();
        if self.is_castle() && !board.settings().is_set(ChessSettings::dfrc_flag()) {
            let rank = self.src_square().rank();
            if self.flags() == CastleKingside {
                to = ChessSquare::from_rank_file(rank, G_FILE_NUM);
            } else {
                to = ChessSquare::from_rank_file(rank, C_FILE_NUM);
            };
        }
        let flag = match self.flags() {
            PromoKnight => "n",
            PromoBishop => "b",
            PromoRook => "r",
            PromoQueen => "q",
            _ => "",
        };
        write!(f, "{from}{to}{flag}", from = self.src_square())
    }

    fn format_extended(
        self,
        f: &mut Formatter<'_>,
        board: &Chessboard,
        format: ExtendedFormat,
        all_legals: Option<&[ChessMove]>,
    ) -> fmt::Result {
        let add_check_mate_suffix = |f: &mut Formatter| {
            let board = board.make_move(self).unwrap();
            if board.is_checkmate_slow() {
                write!(f, "#")?;
            } else if board.is_in_check() {
                write!(f, "+")?;
            }
            Ok(())
        };
        if self.is_castle() {
            match self.castle_side() {
                Queenside => write!(f, "O-O-O")?,
                Kingside => write!(f, "O-O")?,
            };
            return add_check_mate_suffix(f);
        }
        let piece = self.piece(board);
        let matches = |mov: &&ChessMove| {
            mov.piece(board).symbol == piece.symbol
                && mov.dest_square() == self.dest_square()
                && mov.promo_piece() == self.promo_piece()
        };
        let moves = if let Some(moves) = all_legals {
            moves.into_iter().filter(matches).copied().collect_vec()
        } else {
            board.legal_moves_slow().iter().filter(matches).copied().collect_vec()
        };
        if moves.is_empty() {
            return write!(f, "<Illegal move {}>", self.compact_formatter(board));
        }

        match piece.uncolored() {
            Pawn => {}
            uncolored => {
                let piece_char = if format == Standard {
                    uncolored.to_char(CharType::Ascii, board.settings())
                } else {
                    piece.to_char(CharType::Unicode, board.settings())
                };
                write!(f, "{piece_char}")?;
            }
        };

        if moves.len() > 1 {
            if moves.iter().filter(|mov| mov.src_square().file() == self.src_square().file()).count() <= 1 {
                write!(f, "{}", file_to_char(self.src_square().file()))?;
            } else if moves.iter().filter(|mov| mov.src_square().rank() == self.src_square().rank()).count() <= 1 {
                write!(f, "{}", self.src_square().rank() + 1)?;
            } else {
                write!(f, "{}", self.src_square())?;
            }
        } else if piece.uncolored() == Pawn && self.is_capture(board) {
            write!(f, "{}", file_to_char(self.src_square().file()))?;
        }

        if self.is_capture(board) {
            write!(f, "x")?;
        }
        write!(f, "{}", self.dest_square())?;
        if self.is_promotion() {
            write!(f, "=")?;
            let promo_char = if format == Standard {
                self.flags().promo_piece().to_char(CharType::Ascii, board.settings())
            } else {
                self.flags().promo_piece().to_char(CharType::Unicode, board.settings())
            };
            write!(f, "{promo_char}")?;
        }
        add_check_mate_suffix(f)
    }

    fn parse_compact_text<'a>(s: &'a str, board: &Chessboard) -> Res<(&'a str, ChessMove)> {
        if s.is_empty() {
            bail!("Empty input");
        }
        if s.len() < 4 {
            bail!(
                "Move too short: '{s}'. Must be <from square><to square>, e.g. e2e4, and possibly a promotion piece."
            );
        }
        // Need to check this before creating slices because splitting unicode character panics.
        if !s.get(..4).is_some_and(|s| s.is_ascii()) {
            bail!("The first 4 bytes of '{}' contain a non-ASCII character", s.red());
        }
        let from = ChessSquare::from_str(&s[..2])?;
        let mut to = ChessSquare::from_str(&s[2..4])?;
        let piece = board.colored_piece_on(from);
        let mut flags = NormalMove;
        let mut end_idx = 4;
        if let Some((promo_flags, idx)) = parse_short_promo_piece(s) {
            flags = promo_flags;
            end_idx = idx;
        } else if piece.uncolored() == King {
            let rook_capture =
                board.colored_piece_on(to).symbol == ColoredChessPieceType::new(piece.color().unwrap(), Rook);
            if rook_capture || to.file().abs_diff(from.file()) > 1 {
                let color = if from.rank() == 0 { White } else { Black };
                if !rook_capture {
                    // convert normal chess king-to castling notation to rook capture notation (necessary for chess960/DFRC)
                    let to_file = match to.file() {
                        C_FILE_NUM => board.castling.rook_start_file(color, Queenside),
                        G_FILE_NUM => board.castling.rook_start_file(color, Kingside),
                        _ => bail!(
                            "Invalid king move to square {to}, which is neither a normal king move nor a castling move"
                        ),
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
        } else if piece.uncolored() == Pawn && from.rank().abs_diff(to.rank()) > 1 {
            flags = DoublePawnPush;
        }
        let res = from.bb_idx() + (to.bb_idx() << 6) + ((flags as usize) << 12);
        let res = ChessMove(res as u16);
        if !board.is_move_pseudolegal(res) {
            bail!("The move '{0}' is not (pseudo)legal in position '{board}'", s.red())
        }
        Ok((&s[end_idx..], res))
    }

    fn parse_extended_text<'a>(s: &'a str, board: &Chessboard) -> Res<(&'a str, ChessMove)> {
        MoveParser::parse(s, board)
    }

    fn from_u64_unchecked(val: u64) -> UntrustedMove<Chessboard> {
        UntrustedMove::from_move(Self(val as u16))
    }

    fn to_underlying(self) -> Self::Underlying {
        self.0
    }
}

fn parse_short_promo_piece(s: &str) -> Option<(ChessMoveFlags, usize)> {
    if s.len() > 4 {
        let promo = s.chars().nth(4).unwrap().to_ascii_uppercase();
        let num_bytes = promo.len_utf8() + 4;
        let promo = ChessPieceType::parse_from_char(promo).or_else(|| ChessPieceType::parse_from_char(promo))?;
        return Some((
            match promo {
                Knight => PromoKnight,
                Bishop => PromoBishop,
                Rook => PromoRook,
                Queen => PromoQueen,
                _ => return None,
            },
            num_bytes,
        ));
    }
    None
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

    /// For most moves, this returns the hash after playing that move.
    /// Does not handle ep, promotions or castling moves
    pub fn approx_hash_after(&self, mov: ChessMove) -> PosHash {
        let us = self.active;
        let delta = Self::zobrist_delta(us, mov.piece_type(self), mov.src_square(), mov.dest_square());
        let delta = delta ^ ZOBRIST_KEYS.piece_key(self.piece_type_on(mov.dest_square()), !us, mov.dest_square());
        self.hash_pos() ^ delta ^ ZOBRIST_KEYS.side_to_move_key
    }

    /// Is only ever called on a copy of the board, so no need to undo the changes when a move gets aborted due to pseudo-legality.
    #[allow(clippy::too_many_lines)]
    pub(super) fn make_move_impl(mut self, mov: ChessMove) -> Self {
        let piece = mov.piece_type(&self);
        debug_assert_eq!(piece, self.piece_type_on(mov.src_square()));
        let us = self.active;
        let them = us.other();
        let from = mov.src_square();
        let mut to = mov.dest_square();
        let hash_delta = Self::zobrist_delta(us, piece, from, to);
        debug_assert_eq!(us, mov.piece(&self).color().unwrap());
        // `perft` doesn't check for draw conditions, so perft(1000) could overflow the counter.
        // In that case, we don't care about the counter value, and `wrapping_add` is the same speed as `+` in release mode.
        self.ply_100_ctr = self.ply_100_ctr.wrapping_add(1);
        if piece == Pawn {
            self.hashes.pawns ^= hash_delta;
        } else {
            self.hashes.nonpawns[us] ^= hash_delta;
            if piece.is_knb() {
                self.hashes.knb ^= hash_delta;
            }
        }
        // remove old castling flags and ep square, they'll later be set again
        let mut special_hash = PosHash(0);
        if us == White {
            special_hash ^= ZOBRIST_KEYS.side_to_move_key;
        }
        self.ep_square = None;
        self.mailbox[from] = Empty; // needs to be done before moving the rook in chess960 castling
        if mov.is_castle() {
            self.do_castle(mov, from, &mut to);
        } else if mov.is_ep() {
            let taken_pawn = mov.square_of_pawn_taken_by_ep().unwrap();
            debug_assert_eq!(self.colored_piece_on(taken_pawn).symbol, ColoredChessPieceType::new(them, Pawn));
            self.remove_piece_unchecked(taken_pawn, Pawn, them);
            self.hashes.pawns ^= ZOBRIST_KEYS.piece_key(Pawn, them, taken_pawn);
            self.ply_100_ctr = 0;
        } else if mov.is_non_ep_capture(&self) {
            let captured = self.piece_type_on(to);
            assert!(self.colored_piece_on(to).color().is_some(), "{to} {self} {mov:?} {captured}");
            debug_assert_eq!(self.colored_piece_on(to).color().unwrap(), them, "{self} {mov:?}");
            debug_assert_ne!(captured, King);
            self.remove_piece_unchecked(to, captured, them);
            if captured == Pawn {
                self.hashes.pawns ^= ZOBRIST_KEYS.piece_key(captured, them, to);
            } else {
                let removed = ZOBRIST_KEYS.piece_key(captured, them, to);
                self.hashes.nonpawns[them] ^= ZOBRIST_KEYS.piece_key(captured, them, to);
                if captured.is_knb() {
                    self.hashes.knb ^= removed;
                }
            }
            self.ply_100_ctr = 0;
        } else if piece == Pawn {
            self.ply_100_ctr = 0;
            if mov.is_double_pawn_push() {
                self.ep_square = self.calc_ep_sq(to, &mut special_hash, them);
            }
        }
        if piece == King {
            self.castling.clear_castle_rights(us);
        } else if from == self.rook_start_square(us, Queenside) {
            self.castling.unset_castle_right(us, Queenside);
        } else if from == self.rook_start_square(us, Kingside) {
            self.castling.unset_castle_right(us, Kingside);
        }
        if to == self.rook_start_square(them, Queenside) {
            self.castling.unset_castle_right(them, Queenside);
        } else if to == self.rook_start_square(them, Kingside) {
            self.castling.unset_castle_right(them, Kingside);
        }
        special_hash ^= ZOBRIST_KEYS.castle_keys[self.castling.allowed_castling_directions()];
        self.move_piece_no_mailbox(from, to, piece);
        self.mailbox[to] = piece;
        if mov.is_promotion() {
            let piece = mov.flags().promo_piece();
            let bb = to.bb();
            self.piece_bbs[Pawn] ^= bb;
            self.piece_bbs[piece] ^= bb;
            self.mailbox[to] = piece;
            self.hashes.pawns ^= ZOBRIST_KEYS.piece_key(Pawn, us, to);
            let new_piece = mov.flags().promo_piece();
            let new = ZOBRIST_KEYS.piece_key(piece, us, to);
            self.hashes.nonpawns[us] ^= new;
            if new_piece.is_knb() {
                self.hashes.knb ^= new;
            }
        }
        self.hashes.total = special_hash ^ self.hashes.pawns ^ self.hashes.nonpawns[0] ^ self.hashes.nonpawns[1];
        self.flip_side_to_move()
    }

    /// Called at the end of [`Self::make_nullmove`] and [`Self::make_move`].
    pub(super) fn flip_side_to_move(mut self) -> Self {
        self.ply += 1;
        let slider_gen = self.slider_generator();
        debug_assert!(!self.is_in_check_on_square(self.active, self.king_square(self.active), &slider_gen), "{self}");
        self.active = self.active.other();
        self.threats = self.calc_threats_of(self.inactive_player(), &slider_gen);
        self.set_checkers_and_pinned();
        debug_assert_eq!(self.hashes, self.compute_zobrist());
        self
    }

    pub(super) fn calc_ep_sq(
        &self,
        to: ChessSquare,
        special_hash: &mut PosHash,
        them: ChessColor,
    ) -> Option<ChessSquare> {
        let possible_ep_pawns = (to.bb().west() | to.bb().east()) & self.col_piece_bb(them, Pawn);
        if possible_ep_pawns.is_zero() {
            return None;
        }
        let king_sq = self.king_square(them);
        let ep_square = to.pawn_advance_unchecked(them);
        let not_pinned = possible_ep_pawns & !self.pinned[them];
        if not_pinned.is_zero() {
            for p in possible_ep_pawns.ones() {
                let mut pinning = self.ray_attacks(p, king_sq, self.occupied_bb());
                debug_assert!(pinning.is_single_piece());
                let pinning = ChessSquare::from_bb_idx(pinning.pop_lsb());
                let pin_ray = ChessBitboard::ray_inclusive(pinning, king_sq, ChessboardSize::default());
                if pin_ray.is_bit_set(ep_square) {
                    *special_hash ^= ZOBRIST_KEYS.ep_file_keys[to.file() as usize];
                    return Some(ep_square);
                }
            }
            return None;
        }
        if king_sq.rank() == to.rank() {
            // Only the moved pawn and the capturing pawn are between the king and a horizontal slider, so the pawn is effectively pinned for ep.
            let sq = possible_ep_pawns.ones().next().unwrap();
            let occ_bb = self.occupied_bb() ^ to.bb() ^ sq.bb();
            if self.ray_attacks(sq, king_sq, occ_bb).has_set_bit() {
                return None;
            }
        }
        *special_hash ^= ZOBRIST_KEYS.ep_file_keys[to.file() as usize];
        Some(ep_square)
    }

    fn do_castle(&mut self, mov: ChessMove, from: ChessSquare, to: &mut ChessSquare) {
        let color = self.active;
        let rook_file = to.file() as isize;
        let (side, to_file, rook_to_file) = if mov.flags() == CastleKingside {
            (Kingside, G_FILE_NUM, F_FILE_NUM)
        } else {
            (Queenside, C_FILE_NUM, D_FILE_NUM)
        };
        debug_assert_eq!(self.king_square(self.active), from);
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

        let king_ray = ChessBitboard::ray_inclusive(
            from,
            ChessSquare::from_rank_file(from.rank(), to_file),
            ChessboardSize::default(),
        );
        debug_assert!((king_ray & self.threats).is_zero());
        let rook_from = self.rook_start_square(color, side);
        let rook_to = ChessSquare::from_rank_file(from.rank(), rook_to_file);
        debug_assert!(self.colored_piece_on(rook_from).symbol == ColoredChessPieceType::new(color, Rook));
        self.move_piece_no_mailbox(rook_from, rook_to, Rook);
        self.mailbox[rook_from] = Empty;
        self.mailbox[rook_to] = Rook;
        let mut delta = PosHash(0);
        delta ^= ZOBRIST_KEYS.piece_key(King, color, *to);
        *to = ChessSquare::from_rank_file(from.rank(), to_file);
        delta ^= ZOBRIST_KEYS.piece_key(King, color, *to);
        self.hashes.knb ^= delta;
        delta ^= ZOBRIST_KEYS.piece_key(Rook, color, rook_to);
        delta ^= ZOBRIST_KEYS.piece_key(Rook, color, rook_from);
        self.hashes.nonpawns[color] ^= delta;
        debug_assert!(!self.is_in_check_on_square(
            self.active,
            ChessSquare::from_rank_file(from.rank(), to_file),
            &self.slider_generator(),
        ));
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

    pub fn parse(input: &'a str, board: &Chessboard) -> Res<(&'a str, ChessMove)> {
        match Self::parse_impl(input, board) {
            Ok(res) => Ok(res),
            Err(err) => {
                let msg = format!("Current position: '{board}'").dimmed();
                bail!("{err}. {msg}")
            }
        }
    }

    fn parse_impl(input: &'a str, board: &Chessboard) -> Res<(&'a str, ChessMove)> {
        let mut parser = MoveParser::new(input);
        if let Some(mov) = parser.parse_castling(board) {
            parser.parse_check_mate();
            parser.parse_annotation();
            if !board.is_move_pseudolegal(mov) {
                // can't use `to_extended_text` because that requires pseudolegal moves.
                bail!(
                    "Castling move '{}' is not pseudolegal in the current position",
                    mov.compact_formatter(board).to_string().red()
                );
            }
            parser.check_check_checkmate_captures_and_ep(mov, board)?; // check this once the move is known to be pseudolegal
            return Ok((parser.remaining(), mov));
        }
        parser.parse_piece()?;
        parser.parse_maybe_capture()?;
        parser.parse_square_rank_or_file()?;
        parser.parse_maybe_hyphen_or_capture()?;
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
        Ok((remaining, mov))
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
        let color = board.active;
        let king_square = board.king_square(color);
        if self.original_input.starts_with("0-0-0") || self.original_input.starts_with("O-O-O") {
            for _ in 0..5 {
                self.advance_char();
            }
            return Some(ChessMove::new(
                king_square,
                ChessSquare::from_rank_file(king_square.rank(), board.castling.rook_start_file(color, Queenside)),
                CastleQueenside,
            ));
        }
        if self.original_input.starts_with("0-0") || self.original_input.starts_with("O-O") {
            for _ in 0..3 {
                self.advance_char();
            }
            return Some(ChessMove::new(
                king_square,
                ChessSquare::from_rank_file(king_square.rank(), board.castling.rook_start_file(color, Kingside)),
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
            bail!("Empty move string");
        };
        match current {
            'a'..='h' | 'A' | 'C' | 'E'..='H' | 'x' | ':' | '×' => (),
            _ => {
                self.piece = ColoredChessPieceType::parse_from_char(current)
                    .map(ColoredChessPieceType::uncolor)
                    .or_else(|| ChessPieceType::parse_from_char(current))
                    .ok_or_else(|| {
                        anyhow!(
                            "The move '{}' starts with '{current}', which is not a piece or file",
                            self.original_input.split_ascii_whitespace().next().unwrap().red()
                        )
                    })?;
                self.advance_char();
            }
        };
        Ok(())
    }

    fn parse_maybe_hyphen_or_capture(&mut self) -> Res<()> {
        match self.current_char().unwrap_or(' ') {
            '–' | '—' | '−' | '‐' | '‒' | '‑' | '⁃' | '-' | '﹣' | '－' => {
                self.advance_char();
                Ok(())
            }
            _ => self.parse_maybe_capture(),
        }
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
                if matches!(c, '–' | '—' | '−' | '‐' | '‒' | '‑') {
                    self.advance_char();
                }
                Ok(())
            }
        }
    }

    fn parse_square_rank_or_file(&mut self) -> Res<()> {
        let Some(file) = self.current_char() else { bail!("Move '{}' is too short", self.consumed().red()) };
        self.advance_char();
        let Some(rank) = self.current_char() else { bail!("Move '{}' is too short", self.consumed().red()) };
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
                        bail!(
                            "A move must start with a valid file, rank or piece, but '{}' is neither",
                            x.to_string().red()
                        )
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
            ColoredChessPieceType::parse_from_char(c)
                .map(ColoredChessPieceType::uncolor)
                .or_else(|| ChessPieceType::parse_from_char(c))
        });
        if let Some(promo) = piece {
            self.promotion = promo;
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
            if parsed_plus && self.current_char().is_some_and(|c| matches!(c, '/' | '-' | '=')) {
                // actually not a check, but a position evaluation (which gets ignored, so no need to undo the parsing)
                self.gives_check = false;
            }
        }
    }

    fn parse_annotation(&mut self) {
        self.ignore_whitespace();
        let annotation_chars = [
            '!', '?', '⌓', '□', ' ', '⩲', '⩱', '±', '∓', '⨀', '○', '●', '⟳', '↑', '→', '⯹', '⨁', '⇆', '∞', '/', '+',
            '-', '=', '<', '>', '$',
        ];
        while self.current_char().is_some_and(|c| annotation_chars.contains(&c)) {
            if self.current_char().unwrap() != '$' {
                self.advance_char();
            } else {
                self.advance_char();
                while self.current_char().is_some_and(|c| c.is_ascii_digit()) {
                    self.advance_char();
                }
            }
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
            bail!("Missing the file of the target square in move '{}'", self.consumed());
        }
        if self.piece != Pawn && self.target_rank.is_none() {
            bail!("Missing the rank of the target square in move '{}'", self.consumed());
        }

        let mut moves = board
            .pseudolegal_moves()
            .into_iter()
            .filter(|mov| self.is_matching_pseudolegal(mov, board) && board.is_pseudolegal_move_legal(*mov));
        let res = match moves.next() {
            None => self.error_msg(board, original_piece)?,
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

    fn is_matching_pseudolegal(&self, mov: &ChessMove, pos: &Chessboard) -> bool {
        mov.piece_type(pos) == self.piece
            && mov.dest_square().file() == self.target_file.unwrap()
            && self.target_rank.is_none_or(|r| r == mov.dest_square().rank())
            && self.start_file.is_none_or(|f| f == mov.src_square().file())
            && self.start_rank.is_none_or(|r| r == mov.src_square().rank())
            && self.promotion == mov.promo_piece()
    }

    fn error_msg(&self, board: &Chessboard, original_piece: ChessPieceType) -> Res<ChessMove> {
        let us = board.active;
        let our_name = us.to_string().bold();
        let our_piece = self.piece.to_name().bold();
        let move_str = self.consumed().red();
        // invalid move, try to print a helpful error message
        let f = |file: Option<DimT>, rank: Option<DimT>| {
            if let Some(file) = file {
                match rank {
                    Some(rank) => {
                        let square = ChessSquare::from_rank_file(rank, file);
                        (square.to_string(), square.bb())
                    }
                    None => (format!("the {} file", file_to_char(file)), ChessBitboard::file(file)),
                }
            } else if let Some(rank) = rank {
                (format!("rank {rank}"), ChessBitboard::rank(rank))
            } else {
                ("any square".to_string(), !ChessBitboard::default())
            }
        };
        let (from, from_bb) = f(self.start_file, self.start_rank);
        let to = f(self.target_file, self.target_rank).0;
        let (from, to) = (from.bold(), to.bold());
        let mut additional = self.additional_msg(board, &our_name, &our_piece);
        if !additional.is_empty() {
            additional = format!(" ({})", additional.bold());
        }

        // moves without a piece but source and dest square have probably been meant as UCI moves, and not as pawn moves
        if original_piece == Empty && from_bb.is_single_piece() {
            let piece = board.colored_piece_on(from_bb.to_square().unwrap());
            if piece.is_empty() {
                bail!("The square {from} is {0}, so the move '{move_str}' is invalid{additional}", "empty".bold(),)
            } else if piece.color().unwrap() != us {
                bail!(
                    "There is a {0} on {from}, but it's {our_name}'s turn to move, so the move '{move_str}' is invalid{additional}",
                    piece.symbol.name().bold(),
                )
            } else {
                bail!(
                    "There is a {our_piece} on {from}, but it can't move to {to}, so the move '{move_str}' is invalid{additional}"
                )
            }
        }
        if (board.col_piece_bb(board.active, self.piece) & from_bb).is_zero() {
            bail!("There is no {our_name} {our_piece} on {from}, so the move '{move_str}' is invalid{additional}")
        } else {
            bail!(
                "There is no legal {our_name} {our_piece} move from {from} to {to}, so the move '{move_str}' is invalid{additional}"
            );
        }
    }

    fn additional_msg(&self, board: &Chessboard, our_name: &ColoredString, our_piece: &ColoredString) -> String {
        let us = board.active;
        let pinned = if self.target_rank.is_some() && self.target_file.is_some() {
            board.all_attacking(
                ChessSquare::from_rank_file(self.target_rank.unwrap(), self.target_file.unwrap()),
                &board.slider_generator(),
            ) & board.pinned[board.active]
                & board.col_piece_bb(us, self.piece)
        } else {
            ChessBitboard::default()
        };
        let target_sq = if self.target_rank.is_some() && self.target_file.is_some() {
            Some(ChessSquare::from_rank_file(self.target_rank.unwrap(), self.target_file.unwrap()))
        } else {
            None
        };
        let start_sq = if self.start_rank.is_some() && self.start_file.is_some() {
            Some(ChessSquare::from_rank_file(self.start_rank.unwrap(), self.start_file.unwrap()))
        } else {
            None
        };
        if board.is_checkmate_slow() {
            return format!("{us} has been checkmated");
        } else if board.is_in_check() {
            return format!("{us} is in check)");
        } else if let Some(sq) = pinned.ones().next() {
            return format!("the {our_piece} on {sq} is pinned");
        } else if board.pseudolegal_moves().iter().any(|m| self.is_matching_pseudolegal(m, board)) {
            return format!("it leaves the {us} king in check");
        } else if let Some(target) = target_sq {
            if board.player_bb(us).is_bit_set(target) {
                let piece = board.piece_type_on(target_sq.unwrap());
                return format!("there is already a {our_name} {0} on {1}", piece.to_name().bold(), target_sq.unwrap());
            } else if (self.promotion != Empty) != (self.piece == Pawn && target.is_backrank()) {
                return format!("promoting to a {0} is incorrect", self.promotion.to_name().bold());
            }
        }
        if start_sq.is_some() {
            let piece = board.colored_piece_on(start_sq.unwrap());
            if piece.is_empty() {
                return format!("there is no piece on {0}", start_sq.unwrap());
            } else if piece.symbol != ColoredChessPieceType::new(us, self.piece) {
                return format!("there is a {0} on {1}", piece.to_string().bold(), start_sq.unwrap());
            }
        } else if self.piece == King {
            // rank and file have already been checked to exist in the move description (only pawns can omit rank)
            let dest = ChessSquare::from_rank_file(self.target_rank.unwrap(), self.target_file.unwrap());
            if board.threats().is_bit_set(dest) {
                return format!("The king would be in check on the {dest} square");
            }
        }
        String::new()
    }

    // I love this name
    // assumes that the move has already been verified to be pseudolegal. TODO: Encode in type system
    fn check_check_checkmate_captures_and_ep(&self, mov: ChessMove, board: &Chessboard) -> Res<()> {
        let incorrect_mate = self.gives_mate && !board.is_game_won_after_slow(mov, NoHistory::default());
        let incorrect_check = self.gives_check && !board.gives_check(mov);
        let incorrect_capture = self.is_capture && !mov.is_capture(board);
        // Missing check / checkmate signs or ep annotations are ok, but incorrect ones aren't
        if (self.is_ep && mov.flags() != EnPassant) || incorrect_mate || incorrect_check || incorrect_capture {
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
                mov.compact_formatter(board).to_string().bold() // can't use to_extended_text() here, as that requires pseudolegal moves
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::games::Board;
    use crate::games::chess::ChessColor::White;
    use crate::games::chess::castling::CastleRight::Queenside;
    use crate::games::chess::moves::ChessMove;
    use crate::games::chess::pieces::ChessPieceType;
    use crate::games::chess::squares::ChessSquare;
    use crate::games::chess::{ChessSettings, Chessboard, UCI_CHESS960};
    use crate::games::generic_tests;
    use crate::general::bitboards::RawBitboard;
    use crate::general::board::BoardHelpers;
    use crate::general::board::Strictness::{Relaxed, Strict};
    use crate::general::board::UnverifiedBoard;
    use crate::general::moves::ExtendedFormat::{Alternative, Standard};
    use crate::general::moves::Move;
    use crate::general::perft::perft;
    use crate::output::pgn::parse_pgn;
    use crate::search::DepthPly;
    use itertools::Itertools;
    use std::sync::atomic::Ordering;

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
            ("Rb7d7", "Rd7"), // the move Rd1d7 is pseudolegal but not legal, so it shouldn't be disambiguated
            ("gf8:🨂", "gxf8=R"),
            (":d8🨂 checkmate", "exd8=R#"),
            ("exf♘", "exf8=N"),
            ("gf:♝", "gxf8=B"),
            ("xf5", "gxf5"),
            ("Ra7", "Rxa7"),
            ("rB8", "Rb8+"),
            ("nA7+", "Nxa7#"),
            ("N3a5", "Nba5"),
            ("Kg1-h1", "Kh1"),
            ("Rd1-c1", "Rxc1"),
        ];
        let pos = Chessboard::from_name("unusual").unwrap();
        {
            let pos = pos.make_move_from_str("Rb8").unwrap();
            assert!(pos.checkers.has_set_bit());
            assert!(!pos.legal_moves_slow().is_empty());
        }
        for (input, output) in transformations {
            let mov = ChessMove::from_extended_text(input, &pos).unwrap();
            let extended = mov.to_extended_text(&pos, Standard);
            assert_eq!(extended, output);
            assert_eq!(ChessMove::from_extended_text(&mov.to_extended_text(&pos, Alternative), &pos).unwrap(), mov);
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
        let moves = [0, 60449, 28220];
        for mov in moves {
            let mov = ChessMove::from_u64_unchecked(mov);
            for pos in Chessboard::bench_positions() {
                if let Some(mov) = mov.check_pseudolegal(&pos) {
                    _ = pos.make_move(mov);
                    // check that the move representation is unique
                    assert!(
                        pos.pseudolegal_moves().contains(&mov),
                        "{pos} -- {0} {1}",
                        mov.compact_formatter(&pos),
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
    fn ep_pinned() {
        let input = "8/8/2k5/8/2pPp3/8/6B1/2RK4 b - d3 0 1";
        assert!(Chessboard::from_fen(input, Strict).is_err());
        let pos = Chessboard::from_fen(input, Relaxed).unwrap();
        assert_eq!(pos.ep_square(), None);
        for m in pos.pseudolegal_moves() {
            assert!(!m.is_ep());
        }
    }

    #[test]
    fn castle_test() {
        assert!(!UCI_CHESS960.load(Ordering::Relaxed));
        let p = Chessboard::chess_960_startpos(42).unwrap();
        let p = p.remove_piece(ChessSquare::from_chars('f', '1').unwrap()).unwrap().verify(Strict).unwrap();
        assert!(p.settings().is_set(ChessSettings::shredder_fen_flag()));
        assert!(p.debug_verify_invariants(Strict).is_ok());
        let p2 = Chessboard::from_fen("bb1r2kr/p1ppppp1/1n2qn2/8/8/8/PPPPPPP1/BB1RQNKR b KQkq - 0 1", Relaxed).unwrap();
        let tests: &[(Chessboard, &[&str], u64)] = &[
            (Chessboard::from_name("kiwipete").unwrap(), &["0-0", "0-0-0", "e1g1", "e1h1", "e1a1", "e1c1"], 97862),
            (p, &["0-0", "g1h1"], 8953),
            (p2, &["0-0", "0-0-0", "g8h8", "g8c8", "g8d8"], 57107), // TODO: Allow `g8g8` for castling?
        ];
        for (i, (pos, moves, perft_nodes)) in tests.iter().enumerate() {
            for mov in *moves {
                let mov = ChessMove::from_text(mov, pos).unwrap();
                assert!(mov.is_castle());
                assert!(!mov.is_capture(pos));
                let _ = pos.make_move(mov).unwrap().debug_verify_invariants(Strict).unwrap();
                assert_eq!(pos.piece_type_on(mov.dest_square()), ChessPieceType::Rook);
                assert_eq!(i != 0, pos.settings.is_set(ChessSettings::dfrc_flag()));
                assert_eq!(*pos == p, pos.settings.is_set(ChessSettings::shredder_fen_flag()));
            }
            let perft_res = perft(DepthPly::new(3), *pos, false);
            assert_eq!(perft_res.nodes, *perft_nodes);
        }
        let pos = Chessboard::from_fen("5k2/8/8/8/8/8/8/4K2R w K - 0 1", Strict).unwrap();
        let mov = ChessMove::from_text("0-0", &pos).unwrap();
        assert_eq!(mov.extended_formatter(&pos, Standard, None).to_string(), "O-O+");
        let castling = |pos: Chessboard| {
            pos.legal_moves_slow()
                .iter()
                .filter_map(|m| if m.is_castle() { Some(m.compact_formatter(&pos).to_string()) } else { None })
                .sorted()
                .collect_vec()
        };
        let pos = Chessboard::from_name("kiwipete").unwrap();
        assert!(!pos.settings.is_set(ChessSettings::dfrc_flag()));
        let moves = castling(pos);
        assert_eq!(moves.len(), 2);
        assert_eq!(moves[0], "e1c1");
        assert_eq!(moves[1], "e1g1");
        // same as kiwipete, but in  shredder FEN notation
        let pos = Chessboard::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w HAha - 0 1", Strict)
            .unwrap();
        assert!(!pos.settings.is_set(ChessSettings::dfrc_flag()));
        assert!(pos.settings.is_set(ChessSettings::shredder_fen_flag()));
        let moves = castling(pos);
        assert_eq!(moves.len(), 2);
        assert_eq!(moves[0], "e1c1");
        assert_eq!(moves[1], "e1g1");
        // same as kiwipete, but the king is moved one file to the right
        let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R4K1R w KQkq - 1 2";
        assert!(Chessboard::from_fen(fen, Strict).is_err());
        let pos = Chessboard::from_fen(fen, Relaxed).unwrap();
        assert!(pos.settings.is_set(ChessSettings::dfrc_flag()));
        assert!(!pos.settings.is_set(ChessSettings::shredder_fen_flag()));
        let moves = castling(pos);
        assert_eq!(moves.len(), 2);
        assert_eq!(moves[0], "f1a1");
        assert_eq!(moves[1], "f1h1");
        let fen = "8/4k3/8/8/8/8/8/RK1b4 w A - 0 1";
        let mut pos = Chessboard::from_fen(fen, Strict).unwrap();
        assert!(pos.castling.can_castle(White, Queenside));
        assert!(pos.make_move_from_str("0-0-0").is_err());
        pos = pos.make_nullmove().unwrap();
        pos = pos.make_move_from_str("Be2").unwrap();
        assert!(pos.make_move_from_str("0-0-0").is_ok());
    }

    #[test]
    fn many_queens() {
        let pgn = "
Na3 ♞a6 2. ♘a3c4 a6c5 3. Na5 Nb3 4. Nc6 Nf6 5. Nf3 Ne4 6. Nh4 Ng5 7. Ng6 Nf3+ 8. e:f3 dxc6 9. Bc4 ♗f5! 10. Be6 Bd3 11. ab c5 12. Ra6 ba6: \
    13. b3b4 a5 14. b5 a4 15. cxd3 c4 16. d4 fxe6 17. d5 ♟e6e5 18. f4 hxg6 19. f5 Rh3 20. gxh3 e4 21. h4 g5 22. h5 g4 23. Ke2 g3 24. h4 e3 \
    25. Kf3 g2 26. h6 e2 27. h7 a3 28. h5 ♟a2 29. h6 a1=♛ 30. ♔g4 g1=Q+ 31. Kh5 g5 32. b4 a5 33. h8=Q Qb1 34. Qb2 a4 35. h7 a4a3 36. d4 c3 \
    37. d6 c5 38. d4d5 c4 39. f4 Qa7 40. h8=Q a3a2 41. Qhd4 Bh6 42. b6 Kf8 43. b7 Kg8 44. b8Q ♚h7 45. f6 g4 46. f7 g3 47. f8=Q e5 48. d7 e4 \
    49. ♙b4b5 g2 50. Qfb4 e1=Q 51. ♙f5 e3 52. f6 e2 53. Bf4 c2 54. f7 c1=Q 55. f8=Q g1♛ 56. d6 Qda5 57. d8=Q a1=Q \
    58. Qg5 Qeg3 59. d7 e1Q 60. d8=♕ c3 61. b6 c2 62. b7 ♕cd2 63. Qb8d6 c1=Q 64. b8=Q";
        let data = parse_pgn::<Chessboard>(pgn, Strict, None).unwrap();
        let pos = data.game.board;
        assert_eq!(pos.as_fen(), "rQ1Q1Q2/q6k/3Q3b/q5QK/1Q1Q1B2/6q1/1Q1q4/qqqQq1qR b - - 0 64");
        let perft_res = perft(DepthPly::new(3), pos, true);
        assert_eq!(perft_res.nodes, 492194);
    }
}
