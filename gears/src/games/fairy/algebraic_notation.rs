/*
 *  Gears, a collection of board games.
 *  Copyright (C) 2024 ToTheAnd
 *
 *  Gears is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Gears is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Gears. If not, see <https://www.gnu.org/licenses/>.
 */
use crate::PlayerResult::{Draw, Lose, Win};
use crate::games::CharType::{Ascii, Unicode};
use crate::games::fairy::Side::{Kingside, Queenside};
use crate::games::fairy::attacks::GenAttackKind::Drop;
use crate::games::fairy::attacks::MoveKind;
use crate::games::fairy::moves::FairyMove;
use crate::games::fairy::pieces::{ColoredPieceId, PieceId};
use crate::games::fairy::rules::{NoMovesCondition, PromoFenModifier, PromoMoveChar};
use crate::games::fairy::{FairyBitboard, FairyBoard, FairyColor, FairySquare};
use crate::games::{
    AbstractPieceType, CharType, Color, ColoredPiece, ColoredPieceType, DimT, NoHistory, char_to_file, file_to_char,
};
use crate::general::bitboards::{Bitboard, RawBitboard};
use crate::general::board::{BitboardBoard, Board, BoardHelpers, RectangularBoard, UnverifiedBoard};
use crate::general::common::{Res, parse_int_from_str};
use crate::general::moves::ExtendedFormat::Standard;
use crate::general::moves::{ExtendedFormat, Move};
use crate::general::squares::{CompactSquare, RectangularCoordinates};
use anyhow::{anyhow, bail};
use colored::Colorize;
use itertools::Itertools;
use std::fmt;
use std::fmt::Formatter;

pub fn format_san(
    f: &mut Formatter,
    mov: FairyMove,
    pos: &FairyBoard,
    format: ExtendedFormat,
    all_legals: Option<&[FairyMove]>,
) -> fmt::Result {
    format_san_impl(f, mov, pos, format, all_legals)?;
    if !pos.is_move_pseudolegal(mov) {
        write!(f, "<Illegal move '{}' (not pseudolegal)>", mov.compact_formatter(pos))?;
        return Ok(());
    }
    let Some(pos) = pos.clone().make_move(mov) else {
        write!(f, "<Illegal move '{}' (pseudolegal)>", mov.compact_formatter(pos))?;
        return Ok(());
    };
    let no_legals = if let Some(moves) = all_legals { moves.is_empty() } else { pos.has_no_legal_moves() };
    if no_legals {
        for (cond, res) in &pos.rules().game_end_no_moves {
            if matches!(cond, NoMovesCondition::InCheck)
                && cond.satisfied(&pos)
                && res.to_res(&pos, &NoHistory::default()) == Lose
            {
                return write!(f, "#");
            }
        }
    }
    if pos.is_in_check() && pos.rules().format_rules.promo_move_char != PromoMoveChar::Plus {
        write!(f, "+")?;
    }
    Ok(())
}

fn format_san_impl(
    f: &mut Formatter,
    mov: FairyMove,
    pos: &FairyBoard,
    format: ExtendedFormat,
    all_legals: Option<&[FairyMove]>,
) -> fmt::Result {
    let rules = pos.rules();
    if let MoveKind::Castle(side) = mov.kind() {
        return match side {
            Queenside => write!(f, "O-O-O"),
            Kingside => write!(f, "O-O"),
        };
    } else if let MoveKind::Drop(_) = mov.kind() {
        return mov.format_compact(f, pos);
    }
    let colored = mov.piece(pos);
    let piece = colored.uncolor();
    let matches = |m: &&FairyMove| {
        m.piece(pos).uncolor() == piece
            && m.dest_square_in(pos) == mov.dest_square_in(pos)
            && m.promo_piece() == mov.promo_piece()
    };
    let moves = if let Some(moves) = all_legals {
        moves.into_iter().filter(matches).copied().collect_vec()
    } else {
        pos.legal_moves_slow().iter().filter(matches).copied().collect_vec()
    };
    if moves.is_empty() {
        return Ok(()); // the calling function will write an error message
    }

    if !piece.get(rules).unwrap().output_omit_piece || rules.is_usi_fmt() {
        let char_type = match format {
            Standard => Ascii,
            ExtendedFormat::Alternative => Unicode,
        };
        piece.write_as_str(rules, char_type, false, f)?;
    }
    format_origin(f, mov, pos, &moves, piece)?;

    if mov.is_capture() {
        write!(f, "x")?;
    } else if pos.rules().is_usi_fmt() {
        write!(f, "-")?;
    }
    write!(f, "{}", pos.square_formatter(mov.dest_square_in(pos)))?;
    if let Some(promo) = mov.promo_piece() {
        if rules.format_rules.promo_move_char == PromoMoveChar::Plus {
            return write!(f, "+");
        }
        write!(f, "=")?;
        let promo_char = if format == Standard {
            promo.to_char(CharType::Ascii, rules)
        } else {
            promo.to_char(CharType::Unicode, rules)
        };
        write!(f, "{promo_char}")?;
    }
    Ok(())
}

fn format_origin(
    f: &mut Formatter,
    mov: FairyMove,
    pos: &FairyBoard,
    moves: &[FairyMove],
    piece: PieceId,
) -> fmt::Result {
    let Some(sq) = mov.src_square_in(pos) else { return Ok(()) };
    // if there are drops, always write the source square to disambiguate except when in USI shogi format
    if pos.rules().pieces().any(|(_, p)| p.attacks.iter().any(|a| a.kind == Drop)) && !pos.rules().is_usi_fmt() {
        return write!(f, "{}", pos.square_formatter(sq));
    }
    if moves.len() > 1 {
        if pos.rules().is_usi_fmt() {
            write!(f, "{}", pos.square_formatter(sq))?;
        } else if moves.iter().filter(|m| m.src_square_in(pos).is_some_and(|s| s.file() == sq.file())).count() <= 1 {
            write!(f, "{}", file_to_char(sq.file()))?
        } else if moves.iter().filter(|mov| mov.src_square_in(pos).is_some_and(|s| s.rank() == sq.rank())).count() <= 1
        {
            write!(f, "{}", sq.rank() + 1)?;
        } else {
            write!(f, "{}", pos.square_formatter(sq))?;
        }
    } else if piece.get(pos.rules()).unwrap().output_omit_piece && mov.is_capture() {
        // pawn captures
        write!(f, "{}", file_to_char(sq.file()))?;
    }
    Ok(())
}

/// A lenient parser that can parse a move in a generalized short or long algebraic notation, intended to be used for human input.
/// Based on the chess SAN parser [`chess::moves::MoveParser`].
/// Does not currently accept shogi formats based on usi.
pub struct MoveParser<'a> {
    original_input: &'a str,
    num_bytes_read: usize,
    start_rank: Option<DimT>,
    start_file: Option<DimT>,
    target_rank: Option<DimT>,
    target_file: Option<DimT>,
    piece: PieceId,
    is_capture: bool,
    is_ep: bool,
    is_drop: bool,
    gives_check: bool,
    gives_mate: bool,
    promotion: PieceId,
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
            piece: PieceId::empty(),
            is_capture: false,
            is_ep: false,
            is_drop: false,
            gives_check: false,
            gives_mate: false,
            promotion: PieceId::empty(),
        }
    }

    pub fn parse(input: &'a str, board: &FairyBoard) -> Res<(&'a str, FairyMove)> {
        match Self::parse_impl(input, board) {
            Ok(res) => Ok(res),
            Err(err) => {
                let msg = format!("Current position: '{board}'").dimmed();
                bail!("{err}. {msg}")
            }
        }
    }

    fn parse_impl(input: &'a str, board: &FairyBoard) -> Res<(&'a str, FairyMove)> {
        let mut parser = MoveParser::new(input);
        parser.parse_stm(board)?;
        if let Some(mov) = parser.parse_castling(board) {
            parser.parse_check_mate();
            parser.parse_annotation();
            if !board.is_move_legal(mov) {
                for m in board.legal_moves_slow() {
                    println!("{}", m.compact_formatter(board));
                }
                // can't use `to_extended_text` because that requires pseudolegal moves.
                bail!(
                    "Castling move '{}' is not legal in the current position",
                    mov.compact_formatter(board).to_string().red()
                );
            }
            parser.check_check_checkmate_captures(mov, board)?; // check this once the move is known to be pseudolegal
            return Ok((parser.remaining(), mov));
        }
        parser.parse_piece(board)?;
        parser.parse_maybe_drop_or_capture()?;
        parser.parse_square_rank_or_file(board)?;
        parser.parse_maybe_hyphen_or_capture()?;
        parser.parse_second_square(board);
        parser.parse_maybe_capture()?;
        parser.parse_promotion(board)?;
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

    fn parse_stm(&mut self, pos: &FairyBoard) -> Res<()> {
        let Some(c) = self.current_char() else { bail!("Empty move string") };
        // used in shogi notation
        let mut col = match c {
            '▲' | '☗' | '●' => FairyColor::first(),
            '△' | '☖' | '○' => FairyColor::second(),
            _ => return Ok(()),
        };
        self.advance_char();
        if pos.rules().colors[0].name.eq_ignore_ascii_case("white") {
            col = !col;
        }
        if col != pos.active {
            bail!(
                "The move notation starts with '{0}' but it's {1}'s turn to move",
                self.consumed().red(),
                pos.active.name(pos.settings())
            )
        }
        Ok(())
    }

    fn parse_castling(&mut self, board: &FairyBoard) -> Option<FairyMove> {
        let color = board.active;
        let info = board.castling_info.players[color];
        let king_square = board.king_square(color)?;
        let side = if self.original_input.starts_with("0-0-0") || self.original_input.starts_with("O-O-O") {
            for _ in 0..5 {
                self.advance_char();
            }
            Queenside
        } else if self.original_input.starts_with("0-0") || self.original_input.starts_with("O-O") {
            for _ in 0..3 {
                self.advance_char();
            }
            Kingside
        } else {
            return None;
        };
        let kind = MoveKind::Castle(side);
        let from = CompactSquare::new(king_square, board.size());
        let to = CompactSquare::new(info.king_dest_sq(side)?, board.size());
        Some(FairyMove::new(from, to, kind, false))
    }

    fn parse_piece_char(c: char, pos: &FairyBoard) -> Option<PieceId> {
        if let Some(piece) = ColoredPieceId::from_char(c, pos.rules()) {
            Some(piece.uncolor())
        } else if let Some(piece) = PieceId::from_char(c, pos.rules()) {
            Some(piece)
        } else {
            None
        }
    }

    fn parse_piece(&mut self, pos: &FairyBoard) -> Res<()> {
        // All lowercase letters that are valid files (i.e. < 'a' + width) are assumed to be part of a square
        // while uppercase letters and other ascii lowercase letters are assumed to be a piece
        let Some(mut current) = self.current_char() else {
            bail!("Move '{}' is too short", self.consumed().red());
        };
        let mut shogi_promoted = false;
        if current == '+' && pos.settings().format_rules.promo_fen_modifier == PromoFenModifier::Shogi {
            shogi_promoted = true;
            self.advance_char();
            let Some(c) = self.current_char() else {
                bail!("Move '{}' is too short, ends after a shogi-style promotion modifier", self.consumed().red());
            };
            current = c;
        }
        let max_file = file_to_char(pos.width() - 1);
        match current {
            file @ 'a'..='z' if file <= max_file && !shogi_promoted => return Ok(()),
            'x' | ':' | '×' | '@' if !shogi_promoted => return Ok(()),
            _ => (),
        };
        let Some(piece) =
            Self::parse_piece_char(current, pos).or_else(|| Self::parse_piece_char(current.to_ascii_lowercase(), pos))
        else {
            bail!(
                "No piece found for character '{0}' in variant '{1}', so the move '{2}' is invalid",
                current.to_string().red(),
                pos.rules().name.bold(),
                self.consumed().red()
            );
        };
        self.advance_char();
        let Some(p) = piece.get(pos.rules()) else {
            bail!("Cannot move the empty piece, so the move '{0}' is invalid", self.consumed().red());
        };
        if !shogi_promoted {
            if self.current_char() == Some('~')
                && pos.rules().format_rules.promo_fen_modifier == PromoFenModifier::Crazyhouse
            {
                self.advance_char();
                let Some(p) = p.promotions.promoted_version else {
                    bail!(
                        "The piece '{0}' can't be promoted, so the move '{1}' is invalid",
                        piece.name(pos.settings()),
                        self.consumed().red()
                    );
                };
                self.piece = p;
            } else {
                self.piece = piece;
            }
        } else {
            let Some(p) = p.promotions.promoted_version else {
                bail!(
                    "The piece '{0}' can't be promoted, so the move '{1}' is invalid",
                    piece.name(pos.settings()),
                    self.consumed().red()
                );
            };
            self.piece = p
        }
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

    fn parse_maybe_drop_or_capture(&mut self) -> Res<()> {
        if self.current_char().unwrap_or(' ') == '@' {
            self.advance_char();
            self.is_drop = true;
            Ok(())
        } else {
            self.parse_maybe_capture()
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
                    if c == 'x'
                        && self.remaining().starts_with(|c: char| c.is_ascii_digit())
                        && !(self.target_file.is_none() && self.target_rank.is_none())
                    {
                        // this is a move like `Nx2` where the `x` is actually a file
                        return Ok(());
                    }
                    self.is_capture = true;
                    self.advance_char();
                }
                if matches!(c, '-' | '–' | '—' | '−' | '‐' | '‒' | '‑') {
                    self.advance_char();
                }
                Ok(())
            }
        }
    }

    fn parse_rank(input: &str) -> Res<(DimT, &str)> {
        let (rank_str, _remaining) = input.split_once(|c: char| !c.is_ascii_digit()).unwrap_or((input, ""));
        let rank: Res<DimT> =
            parse_int_from_str(rank_str, "rank").map_err(|err| anyhow!("Invalid rank in move: {err}"));
        match rank {
            Ok(rank) => {
                if rank == 0 {
                    bail!("Rank can't be zero");
                }
                Ok((rank - 1, rank_str))
            }
            Err(err) => Err(err),
        }
    }

    fn parse_square_rank_or_file(&mut self, pos: &FairyBoard) -> Res<()> {
        let Some(file) = self.current_char() else { bail!("Move '{}' is too short", self.consumed().red()) };
        if pos.settings().is_usi_fmt() && file.is_ascii_digit() {
            if let Some(sq) = self.parse_usi_square(pos) {
                self.start_rank = Some(sq.rank());
                self.start_file = Some(sq.file());
                return Ok(());
            }
        }
        if file.is_ascii_alphabetic() && file <= file_to_char(pos.width() - 1) {
            self.start_file = Some(char_to_file(file.to_ascii_lowercase()));
            self.advance_char();
        }
        if let Ok((rank, rank_str)) = Self::parse_rank(self.remaining()) {
            self.num_bytes_read += rank_str.len();
            self.start_rank = Some(rank);
        }
        if self.start_rank.is_none() && self.start_file.is_none() {
            if self.piece == PieceId::empty() && !self.is_capture {
                bail!(
                    "A move must start with a valid file, rank or piece, but '{}' is neither",
                    self.current_char().unwrap_or_default().to_string().red()
                )
            }
            bail!("'{}' is not a valid file or rank", file.to_string().red())
        }
        Ok(())
    }

    // The second square is the target square, which must always be a complete square (as opposed to only being a row / column)
    // except for pawn captures.
    fn parse_second_square(&mut self, pos: &FairyBoard) {
        let read_so_far = self.num_bytes_read;
        let Some(file) = self.current_char() else { return };
        if file.is_ascii_digit() && pos.rules().is_usi_fmt() {
            if let Some(sq) = self.parse_usi_square(pos) {
                self.target_rank = Some(sq.rank());
                self.target_file = Some(sq.file());
                return;
            }
        }
        self.advance_char();
        if let Ok((rank, rank_str)) = Self::parse_rank(self.remaining()) {
            if let Ok(square) = FairySquare::algebraic(file, rank as usize + 1) {
                self.num_bytes_read += rank_str.len();
                self.target_file = Some(square.file());
                self.target_rank = Some(square.rank());
                return;
            }
        }
        // pawn capture
        if self.piece == PieceId::empty() && file.is_ascii_lowercase() && file <= file_to_char(pos.width() - 1) {
            self.target_file = Some(char_to_file(file));
            return;
        }
        self.num_bytes_read = read_so_far;
    }

    fn parse_usi_square(&mut self, pos: &FairyBoard) -> Option<FairySquare> {
        if let Ok((file, file_str)) = Self::parse_rank(self.remaining()) {
            self.num_bytes_read += file_str.len();
            let Some(rank) = self.current_char() else {
                self.num_bytes_read -= file_str.len();
                return None;
            };
            let rank = char_to_file(rank);
            let rank = pos.height().wrapping_sub(rank.wrapping_add(1));
            let file = pos.width().wrapping_sub(file.wrapping_add(1));
            let sq = FairySquare::from_rank_file(rank, file);
            if let Ok(square) = pos.check_coordinates(sq) {
                self.advance_char();
                return Some(square);
            } else {
                self.num_bytes_read -= file_str.len();
            }
        }
        None
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

    fn parse_promotion(&mut self, pos: &FairyBoard) -> Res<()> {
        let mut allow_fail = true;
        if self.current_char() == Some('+') && pos.rules().format_rules.promo_move_char == PromoMoveChar::Plus {
            self.advance_char();
            let piece = self.piece.get(pos.settings()).unwrap_or(&pos.settings().pieces[0]);
            if let Some(promo) = piece.promotions.promoted_version {
                self.promotion = promo;
                return Ok(());
            }
            bail!(
                "The move '{0}' contains a shogi-style promotion but a {1} can't promote",
                self.consumed().red(),
                piece.name.bold()
            )
        }
        if self.current_char() == Some('=') {
            self.advance_char();
            allow_fail = false;
        }
        if let Some(c) = self.current_char() {
            if let Some(promo) = Self::parse_piece_char(c, pos) {
                self.promotion = promo;
                self.advance_char();
                return Ok(());
            }
        }
        if !allow_fail {
            bail!("Missing promotion piece after '{0}' in {1}", "=".bold(), self.consumed().red());
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
            let c = self.current_char().unwrap();
            self.advance_char();
            if c == '$' {
                while self.current_char().is_some_and(|c| c.is_ascii_digit()) {
                    self.advance_char();
                }
            }
        }
    }

    fn into_move(mut self, board: &FairyBoard) -> Res<FairyMove> {
        assert!(self.start_file.is_some() || self.start_rank.is_some()); // TODO: Remove, support drops
        if self.target_file.is_none() && self.target_rank.is_none() {
            self.target_file = self.start_file;
            self.target_rank = self.start_rank;
            self.start_file = None;
            self.start_rank = None;
        }

        if let Some(file) = self.target_file {
            self.check_file(file, board, "target")?;
        } else {
            bail!("Missing the file of the target square in move '{}'", self.consumed().red());
        }
        if let Some(rank) = self.target_rank {
            self.check_rank(rank, board, "target")?;
        } else if self.piece.get(board.rules()).is_some_and(|p| !p.output_omit_piece) {
            bail!("Missing the rank of the target square in move '{}'", self.consumed().red());
        }

        if let Some(rank) = self.start_rank {
            self.check_rank(rank, board, "source")?;
        }
        if let Some(file) = self.start_file {
            self.check_file(file, board, "source")?;
        }

        // assert_ne!(self.piece, Pawn); // Pawns aren't written as `p` in SAN, but the parser still accepts this.
        let original_piece = self.piece;
        if self.piece == PieceId::empty() {
            let mut no_symbol = board.rules().matching_piece_ids(|p| p.output_omit_piece && !p.uncolored);
            if self.start_rank.is_some() && self.start_file.is_some() {
                // this allows parsing UGI notation and other input where the piece is clear from context
                let sq = FairySquare::from_rank_file(self.start_rank.unwrap(), self.start_file.unwrap());
                self.piece = board.piece_type_on(sq);
            } else if let Some(id) = no_symbol.next() {
                if no_symbol.next().is_none() {
                    self.piece = id;
                } else {
                    bail!(
                        "Missing piece in move '{0}', but it is not clear what piece is meant. There are several piece types that omit the piece symbol",
                        self.consumed().red()
                    )
                }
            } else {
                bail!(
                    "Missing piece in move '{}', but there are no pawns (or other pieces that omit the piece symbol) in the current variant",
                    self.consumed().red()
                );
            }
        }

        let mut moves = board
            .pseudolegal_moves()
            .into_iter()
            .filter(|&mov| self.is_matching_pseudolegal(mov, board, true) && board.is_pseudolegal_move_legal(mov));
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

        self.check_check_checkmate_captures(res, board)?;

        debug_assert!(board.is_move_legal(res));
        Ok(res)
    }

    fn check_rank(&self, rank: DimT, pos: &FairyBoard, descr: &str) -> Res<()> {
        if rank >= pos.height() {
            bail!(
                "The {descr} rank of the move '{0}' is {rank}, but the board height is only {height}",
                self.consumed().red(),
                rank = rank.saturating_add(1).to_string().red(),
                height = pos.height()
            );
        }
        Ok(())
    }

    fn check_file(&self, file: DimT, pos: &FairyBoard, descr: &str) -> Res<()> {
        let width = pos.width();
        if file >= width {
            bail!(
                "The {descr} file of the move '{0}' is {file}, but the board width is only {width}, which means the maximum file is the {1} file",
                self.consumed().red(),
                file_to_char(pos.width() - 1).to_string().bold(),
                file = file_to_char(file).to_string().red(),
            );
        }
        Ok(())
    }

    // allows parsing standard chess ugi castling moves instead of chess960 castling moves
    fn matching_castling_move(&self, mov: FairyMove, pos: &FairyBoard) -> bool {
        let from = mov.src_square_in(pos).unwrap();
        let Some(start_rank) = self.start_rank else { return false };
        let Some(start_file) = self.start_file else { return false };
        let Some(target_file) = self.target_file else { return false };
        let Some(target_rank) = self.target_rank else { return false };
        let start = FairySquare::from_rank_file(start_rank, start_file);
        let target = FairySquare::from_rank_file(target_rank, target_file);
        if start != from {
            return false;
        }
        let info = pos.castling_info.players[pos.active];
        let MoveKind::Castle(side) = mov.kind() else { return false };
        Some(target) == info.king_dest_sq(side)
    }

    fn is_matching_pseudolegal(&self, mov: FairyMove, pos: &FairyBoard, cmp_promo: bool) -> bool {
        let dest = mov.dest_square_in(pos);
        let from = mov.src_square_in(pos);
        if mov.is_castle() {
            return self.matching_castling_move(mov, pos);
        }
        let mut res = true;
        res &= mov.piece(pos).uncolor() == self.piece;
        res &= dest.file() == self.target_file.unwrap();
        res &= self.target_rank.is_none_or(|r| r == dest.rank());
        // drop annotations are optional (except for ambiguity) but can only apply to drops
        res &= !(self.is_drop && !matches!(mov.kind(), MoveKind::Drop(_)));
        if cmp_promo {
            if let Some(promo) = mov.promo_piece() {
                let mut matching_promo = promo == self.promotion;
                if let Some(parsed_promo) = self.promotion.get(pos.rules()) {
                    matching_promo |= parsed_promo.promotions.promoted_version == Some(promo)
                }
                res &= matching_promo;
            } else {
                res &= self.promotion == PieceId::empty();
            }
        }
        res &= match from {
            None => self.start_file.is_none() && self.start_rank.is_none(),
            Some(from) => {
                self.start_file.is_none_or(|f| f == from.file()) && self.start_rank.is_none_or(|r| r == from.rank())
            }
        };
        res
    }

    fn error_msg(&self, board: &FairyBoard, original_piece: PieceId) -> Res<FairyMove> {
        let us = board.active;
        let our_name = us.name(board.settings()).bold();
        let piece_name = self.piece.name(board.settings()).bold();
        // invalid move, try to print a helpful error message
        let f = |file: Option<DimT>, rank: Option<DimT>| {
            if let Some(file) = file {
                match rank {
                    Some(rank) => {
                        let square = FairySquare::from_rank_file(rank, file);
                        (square.to_string(), FairyBitboard::single_piece_for(square, board.size()))
                    }
                    None => (format!("the {} file", file_to_char(file)), FairyBitboard::file_for(file, board.size())),
                }
            } else if let Some(rank) = rank {
                (format!("rank {rank}"), FairyBitboard::rank_for(rank, board.size()))
            } else {
                ("anywhere".to_string(), board.mask_bb())
            }
        };

        let (from, from_bb) = f(self.start_file, self.start_rank);
        let to = f(self.target_file, self.target_rank).0;
        let target_sq = if self.target_rank.is_some() && self.target_file.is_some() {
            Some(FairySquare::from_rank_file(self.target_rank.unwrap(), self.target_file.unwrap()))
        } else {
            None
        };
        let start_sq = if self.start_rank.is_some() && self.start_file.is_some() {
            Some(FairySquare::from_rank_file(self.start_rank.unwrap(), self.start_file.unwrap()))
        } else {
            None
        };
        let (from, to) = (from.bold(), to.bold());
        let mut additional = String::new();
        if let Some(res) = board.player_result_slow(&NoHistory::default()) {
            let outcome = match res {
                Win => format!("{our_name} won"),
                Lose => format!("{our_name} lost"),
                Draw => "it's a draw".to_string(),
            };
            additional = format!(" (the game is over, {outcome})");
        } else if board.is_in_check() {
            additional = format!(" ({our_name} is in check)");
        } else if board.pseudolegal_moves().into_iter().any(|m| self.is_matching_pseudolegal(m, board, true)) {
            additional = " (it is pseudolegal but not legal)".to_string();
        } else if target_sq.is_some_and(|sq| board.player_bb(us).is_bit_set(sq)) {
            let piece = board.piece_type_on(target_sq.unwrap());
            additional = format!(
                " (there is already a {our_name} {0} on {1})",
                piece.name(board.settings()).bold(),
                target_sq.unwrap()
            );
        } else if start_sq.is_some() {
            let piece = board.colored_piece_on(start_sq.unwrap());
            if piece.is_empty() {
                additional = format!(" (there is no piece on {0})", start_sq.unwrap());
            } else {
                let c = piece
                    .color()
                    .map(|c: FairyColor| c.name(board.settings()).bold().to_string())
                    .unwrap_or(String::new());
                additional = format!(
                    " (there is a {c} {0} on {1})",
                    piece.uncolored().name(board.settings()).bold(),
                    start_sq.unwrap()
                );
            }
        }
        let additional = additional.bold();
        let mov = self.consumed().bold();

        // moves without a piece but source and dest square have probably been meant as UCI moves, and not as pawn moves
        if original_piece == PieceId::empty() && from_bb.is_single_piece() {
            let piece = board.colored_piece_on(from_bb.to_square().unwrap());
            let piece_name = piece.symbol.name(board.settings()).as_ref().bold();
            if piece.uncolored() == PieceId::empty() {
                bail!("The square {from} is {0}, so the move '{mov}' is invalid{additional}", "empty".bold(),)
            } else if piece.color() != Some(us) {
                bail!(
                    "There is a {piece_name} on {from}, but it's {our_name}'s turn to move, so the move '{mov}' is invalid{additional}",
                )
            } else {
                bail!(
                    "There is a {piece_name} on {from}, but it can't move to {to}, so the move '{mov}' is invalid{additional}",
                )
            }
        }
        if (board.col_piece_bb(board.active, self.piece) & from_bb).is_zero() {
            bail!("There is no {our_name} {piece_name} on {from}, so the move '{mov}' is invalid{additional}",)
        } else {
            if board.pseudolegal_moves().into_iter().any(|m| self.is_matching_pseudolegal(m, board, false)) {
                bail!(
                    "Incorrect or missing {0} for moving a {our_name} {piece_name} from {from} to {to}, so the move '{mov}' is invalid{additional}",
                    "promotion piece".bold()
                )
            }
            bail!(
                "There is no legal {our_name} {piece_name} move from {from} to {to}, so the move '{mov}' is invalid{additional}",
            );
        }
    }

    // I love this name
    // assumes that the move has already been verified to be pseudolegal. TODO: Encode in type system
    fn check_check_checkmate_captures(&self, mov: FairyMove, board: &FairyBoard) -> Res<()> {
        let incorrect_mate = self.gives_mate && !board.is_game_won_after_slow(mov, NoHistory::default());
        let incorrect_check = self.gives_check && !board.gives_check_slow(mov);
        let incorrect_capture = self.is_capture && !mov.is_capture();
        // Missing check / checkmate signs or ep annotations are ok, but incorrect ones aren't.
        // Currently ignores ep annotations.
        if incorrect_mate || incorrect_check || incorrect_capture {
            let typ = if incorrect_mate {
                "delivers checkmate"
            } else if incorrect_check {
                "gives check"
            } else {
                "captures something"
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
    use crate::games::fairy::FairyBoard;
    use crate::games::fairy::moves::FairyMove;
    use crate::games::generic_tests;
    use crate::general::board::BoardHelpers;
    use crate::general::board::Strictness::Strict;
    use crate::general::moves::ExtendedFormat::{Alternative, Standard};
    use crate::general::moves::Move;
    use crate::general::perft::perft;
    use crate::output::pgn::parse_pgn;
    use crate::search::DepthPly;
    use crate::ugi::load_ugi_pos_simple;

    type GenericTests = generic_tests::GenericTests<FairyBoard>;

    const CHESS_TEST_POS: &str = "2kb1b2/pR2P1P1/P1N1P3/1p2Pp2/P5P1/1N6/4P2B/2qR2K1 w - f6 99 123";

    #[test]
    fn valid_algebraic_notation_chess_test() {
        let transformations = [
            ("Na1", "Na1"),
            ("nxA7 mate", "Nxa7#"),
            ("RC1:", "Rxc1"),
            ("e2e4", "e4"),
            ("e8Q", "e8=Q"),
            ("e5f6:e.p.", "exf6"),
            ("ef:e.p.", "exf6"),
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
        let pos = FairyBoard::from_fen_for("chess", CHESS_TEST_POS, Strict).unwrap();
        for (input, output) in transformations {
            println!("{input}, {output}");
            let mov = FairyMove::from_extended_text(input, &pos).unwrap();
            let extended = mov.to_extended_text(&pos, Standard);
            assert_eq!(extended, output);
            assert_eq!(FairyMove::from_extended_text(&mov.to_extended_text(&pos, Alternative), &pos).unwrap(), mov);
        }
    }

    #[test]
    fn failed_chess_test() {
        let pos = FairyBoard::from_fen("8/7r/8/K1k5/8/8/4p3/8 b - - 10 11", Strict).unwrap();
        let mov = FairyMove::from_extended_text("e1=Q+", &pos).unwrap();
        assert!(pos.is_move_legal(mov));
    }

    #[test]
    fn invalid_algebraic_notation_chess_test() {
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
        let pos = FairyBoard::from_fen_for("chess", CHESS_TEST_POS, Strict).unwrap();
        for input in inputs {
            assert!(FairyMove::from_extended_text(input, &pos).is_err());
        }
    }

    // TODO: Cargo fuzz fairy
    // #[test]
    // fn invalid_moves_test() {
    // }

    #[test]
    fn valid_algebraic_notation_variants_test() {
        let transformations = [
            ("shogi startpos", "☗Gd1-e2", "Gd1e2"),
            ("shogi startpos", "玉d2", "Ke1d2"),
            ("shogi 8l/1l+R2P3/p2pBG1pp/kps1p4/Nn1P2G2/P1P1P2PP/1PS6/1KSG3+r1/LN2+p3L w Sgbn3p 124", "Nc3+", "Nx7g+"),
            ("shogi 8l/1l+R2P3/p2pBG1pp/kps6/Nn1P5/P1P1P1rPP/1PS6/1KSG3+r1/LN2+p3L w Sgbn3p 124", "+R2hxh4", "+Rx2f"),
            ("shogi 8l/1l+R2P3/p2pBG1pp/kps6/Nn1P5/P1P1P1+rPP/1PS6/1KSG3+r1/LN2+p3L w Sgbn3p 124", "+Rhxh4", "+R2hx2f"),
            ("atomic r7/8/8/8/8/8/3k1q2/R3K2R w KQ - 0 1", "e1c1", "O-O-O"),
            ("atomic r7/8/8/8/8/8/3k1q2/R3K2R w KQ - 0 1", "0-0-0", "O-O-O"),
            ("antichess rn1qkb1r/ppp1pppp/8/8/6b1/3p4/PPP1NP1P/R1BQKB1R w - - 0 7", "Qdd3", "Qxd3"),
            ("antichess r3k3/1PK5/8/8/8/2R5/8/8 w - - 4 3", "baQ", "bxa8=Q"), // not a check
            ("crazyhouse r3k3/1PK5/8/8/8/2R5/8/8[] w - - 4 3", "baR", "b7xa8=R+"),
            ("crazyhouse r3k3/QPK5/8/8/8/2R5/8/8[Q] w - - 4 3", "Q7b8", "Qa7b8+"),
            ("crazyhouse r3k3/Q~PK5/8/8/8/2R5/8/8[Q] w - - 4 3", "Q~b8", "Q~a7b8+"),
            ("crazyhouse r3k3/QPK5/8/8/8/2R5/8/8[Q] w - - 4 3", "Q@b8", "Q@b8+"),
            ("atomic rnbqkbnr/ppp1pppp/8/8/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 2", "Qd2#", "Qxd2"),
            ("tictactoe startpos", "X@b2", "b2"),
            ("ataxx startpos", "@f1", "f1"),
            ("ataxx startpos", "g1-e3", "g1e3"),
        ];
        let old = FairyBoard::default();
        for (fen, input, output) in transformations {
            let pos = load_ugi_pos_simple(fen, Strict, &old).unwrap();
            println!("{fen}, {input}, {output}");
            let mov = FairyMove::from_extended_text(input, &pos).unwrap();
            let extended = mov.to_extended_text(&pos, Standard);
            assert_eq!(extended, output);
            assert_eq!(FairyMove::from_extended_text(&mov.to_extended_text(&pos, Alternative), &pos).unwrap(), mov);
        }
    }

    #[test]
    fn algebraic_notation_roundtrip_test() {
        GenericTests::long_notation_roundtrip_test();
    }

    #[test]
    fn many_queens() {
        let pgn = "
Na3 ♞a6 2. ♘a3c4 a6c5 3. Na5 Nb3 4. Nc6 Ng8-f6 5. Nf3 Ne4 6. Nh4 Ng5 7. Ng6 Nf3+ 8. e:f3 dxc6 9. Bc4 ♗f5! 10. Be6 Bd3 11. ab c5 12. Ra6 ba6: \
    13. b3b4 a5 14. b5 a4 15. cxd3 c4 16. d4 fxe6 17. d5 ♟e6e5 18. f4 hxg6 19. f5 Rh3 20. gxh3 e4 21. h4 g5 22. h5 g4 23. Ke2 g3 24. h4 e3 \
    25. Kf3 g2 26. h5-h6 e2 27. h7 a3 28. h5 ♟a2 29. h6 a1=♛ 30. ♔g4 g1=Q+ 31. Kh5 g5 32. b4 a5 33. h8=Q Qb1 34. Qb2 a4 35. h7 a4a3 36. d4 c3 \
    37. d6 c5 38. d4d5 c4 39. f4 Qa7 40. h8=Q a3a2 41. Qhd4 Bh6 42. b6 Kf8 43. b7 Kg8 44. b8Q ♚h7 45. f6 g4 46. f7 g3 47. f8=Q e5 48. d7 e4 \
    49. ♙b4b5 g2 50. Qfb4 e1=Q 51. ♙f5 e3 52. f6 e2 53. Bf4 c2 54. f7 c1=Q 55. f8=Q g1♛ 56. d6 Qda5 57. d8=Q a1=Q \
    58. Qg5 Qeg3 59. d7 e1Q 60. d8=♕ c3 61. b6 c2 62. b7 ♕cd2 63. Qb8d6 c1=Q 64. b8=Q";
        let data = parse_pgn::<FairyBoard>(pgn, Strict, None).unwrap();
        let pos = data.game.board;
        assert_eq!(pos.fen_no_rules(), "rQ1Q1Q2/q6k/3Q3b/q5QK/1Q1Q1B2/6q1/1Q1q4/qqqQq1qR b - - 0 64");
        let perft_res = perft(DepthPly::new(3), pos, true);
        assert_eq!(perft_res.nodes, 492194);
    }
}

// TODO: Testcase for board of maximum width
