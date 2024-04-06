use std::fmt::{Display, Formatter};

use std::str::{FromStr};

use itertools::Itertools;
use num::iter;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::games::{
    AbstractPieceType, Board, Color, ColoredPiece, ColoredPieceType, Move, MoveFlags,
};
use crate::games::chess::Chessboard;
use crate::games::chess::flags::CastleRight;
use crate::games::chess::flags::CastleRight::*;
use crate::games::chess::moves::ChessMoveFlags::*;
use crate::games::chess::pieces::{ChessPiece, ColoredChessPiece, UncoloredChessPiece};
use crate::games::chess::pieces::UncoloredChessPiece::*;
use crate::games::chess::squares::{
    A_FILE_NO, C_FILE_NO, ChessSquare, D_FILE_NO, F_FILE_NO, G_FILE_NO, H_FILE_NO,
};
use crate::games::chess::zobrist::PRECOMPUTED_ZOBRIST_KEYS;
use crate::general::bitboards::{Bitboard, ChessBitboard};
use crate::general::common::Res;

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

    pub fn square_of_pawn_taken_by_ep(self) -> Option<ChessSquare> {
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
        board.piece_on(self.src_square())
    }

    pub fn piece_on_target(self, board: &Chessboard) -> ChessPiece {
        board.piece_on(self.dest_square())
    }

    pub fn is_noisy(self, board: &Chessboard) -> bool {
        self.is_capture(board) || self.flags() == PromoQueen || self.flags() == PromoKnight
    }

    pub fn is_capture(self, board: &Chessboard) -> bool {
        self.flags() == EnPassant || self.is_non_ep_capture(board)
    }

    pub fn is_non_ep_capture(self, board: &Chessboard) -> bool {
        !self.is_castle() && board.is_occupied(self.dest_square())
    }

    pub fn captured(self, board: &Chessboard) -> UncoloredChessPiece {
        if self.flags() == EnPassant {
            Pawn
        } else if self.flags() == Castle {
            Empty
        } else {
            board.piece_on(self.dest_square()).uncolored()
        }
    }

    pub fn is_promotion(self) -> bool {
        self.flags().is_promo()
    }

    pub fn promo_piece(self) -> UncoloredChessPiece {
        if self.is_promotion() {
            self.flags().promo_piece()
        } else {
            Empty
        }
    }

    pub fn is_castle(self) -> bool {
        self.flags() == Castle
    }

    pub fn castle_side(self) -> CastleRight {
        if self.dest_square().file() < self.src_square().file() {
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

    fn src_square(self) -> ChessSquare {
        ChessSquare::new((self.0 & 0x3f) as usize)
    }

    fn dest_square(self) -> ChessSquare {
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
            from = self.src_square(),
            to = self.dest_square()
        )
    }

    fn from_compact_text(s: &str, board: &Chessboard) -> Res<Self> {
        let s = s.trim();
        if s.is_empty() {
            return Err("Empty input".to_string());
        }
        if s.len() < 4 {
            return Err(format!("Move too short: '{s}'. Must be <from square><to square>, e.g. e2e4, and possibly a promotion piece."));
        }
        let from = ChessSquare::from_str(&s[..2])?;
        let to = ChessSquare::from_str(&s[2..4])?;
        let mut flags = Normal;
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
                    .src_square()
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
        let moves = board.legal_moves_slow()
            .filter(|mov| mov.piece(board).symbol == piece.symbol && mov.dest_square() == self.dest_square())
            .collect_vec();
        if moves.is_empty() {
            return format!("<Illegal move {}>", self.to_compact_text());
        }

        if moves.len() > 1 {
            if moves
                .iter()
                .filter(|mov| mov.src_square().file() == self.src_square().file())
                .count()
                <= 1
            {
                res.push(self.src_square().to_string().chars().nth(0).unwrap());
            } else if moves
                .iter()
                .filter(|mov| mov.src_square().rank() == self.src_square().rank())
                .count()
                <= 1
            {
                res.push(self.src_square().to_string().chars().nth(1).unwrap());
            } else {
                res += &self.src_square().to_string();
            }
        }
        if self.is_capture(board) {
            res.push('x');
        }
        res += &self.dest_square().to_string();
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

    fn from_extended_text(s: &str, board: &Chessboard) -> Res<Self> {
        let res = MoveParser::parse(s, board)?;
        if !res.1.is_empty() {
            return Err(format!(
                "Additional input after move {0}: '{1}'",
                res.0.to_extended_text(board),
                res.1
            ));
        }
        Ok(res.0)
    }

    // TODO: Parse pgn (not here though)
}

impl Chessboard {
    pub fn rook_start_square(&self, color: Color, side: CastleRight) -> ChessSquare {
        let idx = color as usize * 2 + side as usize;
        match idx {
            0 => ChessSquare::from_rank_file(0, A_FILE_NO),
            1 => ChessSquare::from_rank_file(0, H_FILE_NO),
            2 => ChessSquare::from_rank_file(7, A_FILE_NO),
            3 => ChessSquare::from_rank_file(7, H_FILE_NO),
            _ => panic!("Internal error"),
        }
    }

    pub fn make_move_impl(mut self, mov: ChessMove) -> Option<Self> {
        let piece = mov.piece(&self).symbol;
        let uncolored = piece.uncolor();
        let color = self.active_player;
        let other = color.other();
        let from = mov.src_square();
        let to = mov.dest_square();
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
    pub fn flip_side_to_move(mut self) -> Option<Self> {
        if self.is_in_check() {
            None
        } else {
            self.active_player = self.active_player.other();
            self.hash ^= PRECOMPUTED_ZOBRIST_KEYS.side_to_move_key;
            debug_assert_eq!(self.hash, self.zobrist_hash());
            Some(self)
        }
    }
}

/// A lenient parser that can parse a move in short or long algebraic notation, intended to be used for human input.
pub struct MoveParser<'a> {
    original_input: &'a str,
    num_bytes_read: usize,
    start_rank: Option<usize>,
    start_file: Option<usize>,
    target_rank: Option<usize>,
    target_file: Option<usize>,
    piece: UncoloredChessPiece,
    is_capture: bool,
    is_ep: bool,
    gives_check: bool,
    gives_mate: bool,
    promotion: UncoloredChessPiece,
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
            parser.check_check_checkmate_captures_and_ep(mov, board)?;
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
            self.num_bytes_read += c.len_utf8()
        }
    }

    fn ignore_whitespace(&mut self) {
        while self.current_char().is_some_and(|c| c.is_whitespace()) {
            self.advance_char();
        }
    }

    // assumes that the last char in `s` is an ASCII char, i.e. takes exactly 1 byte
    fn parse_str_dont_consume_last_char(&mut self, s: &str) -> bool {
        let mut chars = s.chars();
        if self.remaining().starts_with(s) {
            self.num_bytes_read += 0.max(s.len() - 1);
        }
        if !s.is_empty() && self
                .current_char()
                .is_some_and(|c| c == chars.next().unwrap()) && self.remaining() == chars.as_str() {
            // don't call self.advance_char() here so that one character less is consumed.
            // This makes it easier to use this function as part of an if that otherwise only checks a single character
            while chars.next().is_some() {
                self.advance_char();
            }
            return true
        }
        false
    }

    fn parse_castling(&mut self, board: &Chessboard) -> Option<ChessMove> {
        let king_square = board.king_square(board.active_player);
        if self.original_input.starts_with("0-0-0") || self.original_input.starts_with("O-O-O") {
            for _ in 0..5 {
                self.advance_char();
            }
            return Some(ChessMove::new(
                king_square,
                ChessSquare::from_rank_file(king_square.rank(), C_FILE_NO),
                Castle,
            ));
        }
        if self.original_input.starts_with("0-0") || self.original_input.starts_with("O-O") {
            for _ in 0..3 {
                self.advance_char();
            }
            return Some(ChessMove::new(
                king_square,
                ChessSquare::from_rank_file(king_square.rank(), G_FILE_NO),
                Castle,
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
        let current = self
            .current_char()
            .ok_or_else(|| "Empty move".to_string())?;
        match current {
            'a'..='h' | 'A' | 'C' | 'E'..='H' => (),
            _ => {
                self.piece = ColoredChessPiece::from_utf8_char(current)
                    .map(|c| c.uncolor())
                    .or_else(|| UncoloredChessPiece::from_utf8_char(current))
                    .ok_or_else(|| {
                        format!("The move starts with '{current}', which is not a piece or file")
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
                        return Err("Multiple capture symbols".to_string());
                    }
                    self.is_capture = true;
                    self.advance_char();
                }
                Ok(())
            }
        }
    }

    fn parse_square_rank_or_file(&mut self) -> Res<()> {
        let file = self
            .current_char()
            .ok_or_else(|| format!("Move '{}' is too short", self.consumed()))?;
        self.advance_char();
        let rank = self
            .current_char()
            .ok_or_else(|| format!("Move '{}' is too short", self.consumed()))?;
        match ChessSquare::from_chars(file, rank) {
            Ok(sq) => {
                self.advance_char();
                self.start_file = Some(sq.file());
                self.start_rank = Some(sq.rank());
            }
            Err(_) => match file {
                'a'..='h' => self.start_file = Some(file as usize - 'a' as usize),
                '1'..='8' => self.start_rank = Some(file as usize - '1' as usize),
                x => {
                    // doesn't reset the current char, but that's fine because we're aborting anyway
                    return Err(if self.piece == Empty && !self.is_capture {
                        format!("A move must start with a valid file, rank or piece, but '{x}' is neither")
                    } else {
                        format!("'{x}' is not a valid file or rank")
                    });
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
            self.target_file = file.map(|c| c as usize - 'a' as usize);
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
            ColoredChessPiece::from_utf8_char(c)
                .map(|p| p.uncolor())
                .or_else(|| UncoloredChessPiece::from_utf8_char(c))
        });
        if piece.is_some() {
            self.promotion = piece.unwrap();
            self.advance_char();
        } else if !allow_fail {
            return Err("Missing promotion piece after '='".to_string());
        }
        Ok(())
    }

    fn parse_check_mate(&mut self) {
        self.ignore_whitespace();
        assert!(!self.gives_check); // the implementation relies on the fact that this function is only called once per move
        if self
            .current_char()
            .is_some_and(|c| matches!(c, '+' | '†') || self.parse_str_dont_consume_last_char("ch"))
        {
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
        } else if self
            .current_char()
            .is_some_and(|c| matches!(c, '#' | '‡') || self.parse_str_dont_consume_last_char("mate"))
        {
            self.advance_char();
            self.gives_mate = true;
            self.gives_check = true;
        }
    }

    fn parse_annotation(&mut self) {
        self.ignore_whitespace();
        while self.current_char().is_some_and(|c| {
            matches!(
                c,
                '!' | '?'
                    | '⌓'
                    | '□'
                    | ' '
                    | '⩲'
                    | '⩱'
                    | '±'
                    | '∓'
                    | '∞'
                    | '/'
                    | '+'
                    | '-'
                    | '='
            )
        }) {
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

        assert_ne!(self.piece, Pawn);
        if self.piece == Empty {
            self.piece = Pawn;
        }

        if self.target_file.is_none() {
            return Err(format!(
                "Missing the file of the target square in move '{}'",
                self.consumed()
            ));
        }
        if self.piece != Pawn && self.target_rank.is_none() {
            return Err(format!("Missing the rank of the target square in move '{}'", self.consumed()))
        }

        let mut moves = board.gen_all_pseudolegal_moves().filter(|mov| {
            mov.piece(board).uncolored() == self.piece
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
                let f = |file: Option<usize>, rank: Option<usize>| {
                    if file.is_some() {
                        match rank {
                            Some(rank) => ChessSquare::from_rank_file(rank, file.unwrap()).to_string(),
                            None => format!("the {} file", ('a' as usize + file.unwrap()) as u8 as char)
                        }
                    } else if rank.is_some() {
                        format!("rank {}", rank.unwrap())
                    } else {
                        "any square".to_string()
                    }
                };
                let mut additional = "".to_string();
                if board.is_game_lost_slow() {
                    additional = format!(" ({} has been checkmated)", board.active_player);
                } else if board.is_in_check() {
                    additional = format!(" ({} is in check)", board.active_player);
                }
                return Err(format!(
                    "There is no legal {0} {1} move from {2} to {3}, so the move '{4}' is invalid{5}",
                    board.active_player,
                    self.piece.name(),
                    f(self.start_file, self.start_rank),
                    f(self.target_file, self.target_rank),
                    self.consumed(),
                    additional
                ));
            }
            Some(mov) => {
                if let Some(other) = moves.next() {
                    return Err(format!(
                        "Move '{0}' is ambiguous, because it could refer to {1} or {2}",
                        self.consumed(),
                        mov.to_extended_text(board),
                        other.to_extended_text(board)
                    ));
                }
                mov
            }
        };

        assert!(board.is_move_legal(res));

        self.check_check_checkmate_captures_and_ep(res, board)?;
        Ok(res)
    }

    // I love this name
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
            let typ = match incorrect_mate {
                true => "delivers checkmate",
                false => match incorrect_check {
                    true => "gives check",
                    false => match incorrect_capture {
                        true => "captures something",
                        false => "captures en passant",
                    },
                },
            };
            return Err(format!(
                "The move notation '{0}' claims that it {typ}, but the move {1} actually doesn't",
                self.consumed(),
                mov.to_extended_text(board)
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::games::{Board, Move};
    use crate::games::chess::Chessboard;
    use crate::games::chess::moves::ChessMove;

    #[test]
    fn simple_algebraic_notation_test() {
        // TODO: Finish writing this testcase
        let transformations = [("Na1", "Na1"),
            ("nxA7 mate", "Nxa7#"),
            ("RD1:+", "Rxd1+"),
            ("e2e4", "e4"),
            ("f8D", "f8Q"),
            ("a5b6:e.p.+", "axb6+"),
            ("b:ep+", "axb6+"),
            ("🨅e4", "e4"), // TODO: more (Un)colored unicode pieces
        ];
        // TODO: Implement
    }

    #[test]
    fn invalid_algebraic_notation_test() {
        let inputs = ["resign", "Robert'); DROP TABLE Students;--", "Raa", "R4", "Raaa4", "Qi1", "Ra8D", "ef e.p.", "O-O-O-O"];
        // TODO: Implement
    }

    #[test]
    fn algebraic_notation_roundtrip_test() {
        let positions = Chessboard::name_to_pos_map();
        for pos in positions.into_iter() {
            let pos = (pos.val)();
            for mov in pos.legal_moves_slow() {
                let encoded = mov.to_extended_text(&pos);
                let decoded = ChessMove::from_extended_text(&encoded, &pos);
                assert!(decoded.is_ok());
                assert_eq!(decoded.unwrap(), mov);
            }
        }
    }
}
