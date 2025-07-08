use anyhow::{anyhow, bail};
use arbitrary::Arbitrary;
use std::fmt;
use std::fmt::Formatter;
use std::sync::atomic::Ordering::Relaxed;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::games::chess::ChessColor::*;
use crate::games::chess::castling::CastleRight::*;
use crate::games::chess::pieces::ChessPieceType::{King, Rook};
use crate::games::chess::pieces::ColoredChessPieceType;
use crate::games::chess::squares::{
    A_FILE_NUM, C_FILE_NUM, ChessSquare, D_FILE_NUM, E_FILE_NUM, F_FILE_NUM, G_FILE_NUM, H_FILE_NUM, NUM_COLUMNS,
};
use crate::games::chess::{ChessColor, ChessSettings, Chessboard, UCI_CHESS960};
use crate::games::{Board, Color, ColoredPieceType, DimT, char_to_file, file_to_char};
use crate::general::bitboards::RawBitboard;
use crate::general::board::Strictness::Strict;
use crate::general::board::{BitboardBoard, Strictness};
use crate::general::common::Res;
use crate::general::squares::RectangularCoordinates;

#[derive(EnumIter, Copy, Clone, Eq, PartialEq, Debug, derive_more::Display)]
#[must_use]
pub enum CastleRight {
    Queenside,
    Kingside,
}

impl CastleRight {
    #[must_use]
    pub fn king_dest_file(self) -> DimT {
        match self {
            Queenside => C_FILE_NUM,
            Kingside => G_FILE_NUM,
        }
    }

    #[must_use]
    pub fn rook_dest_file(self) -> DimT {
        match self {
            Queenside => D_FILE_NUM,
            Kingside => F_FILE_NUM,
        }
    }
}

/// Stores the queen/kingside castling files for white/black in 3 bits each and uses the upper 4 bits to store
/// if castling is legal. The bit at index 16 is not set iff the castling rights should be printed in X-FEN format
/// (which is backwards compatible to standard FEN, unlike Shredder FEN). This is set to be the format
/// in which the FEN is received (startpos and all non-chess960 FENs are X-FENs for maximum GUI support).
/// X-FENs are disambiguated as described on wikipedia.
/// More compact representations (fitting into 8 bits) are possible because e.g. queenside castling to the h file
/// is impossible, but don't really seem worth it because the size of the [`Chessboard`] doesn't change anyway.

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub struct CastlingFlags(u16);

const CASTLE_RIGHTS_SHIFT: usize = 12;
impl CastlingFlags {
    pub(super) const fn for_startpos() -> Self {
        let mut res = Self(0);
        res.set_castle_right_impl(White, Queenside, 0);
        res.set_castle_right_impl(Black, Queenside, 0);
        res.set_castle_right_impl(White, Kingside, 7);
        res.set_castle_right_impl(Black, Kingside, 7);
        res
    }

    #[must_use]
    pub fn allowed_castling_directions(self) -> usize {
        (self.0 >> CASTLE_RIGHTS_SHIFT) as usize
    }

    const fn shift(color: ChessColor, castle_right: CastleRight) -> usize {
        color as usize * 6 + castle_right as usize * 3
    }

    /// This return value of this function can only be used if `can_castle` would return `true`.
    #[must_use]
    pub fn rook_start_file(self, color: ChessColor, castle_right: CastleRight) -> DimT {
        ((self.0 >> Self::shift(color, castle_right)) & 0x7) as DimT
    }

    /// Returns true iff castling rights haven't been lost. Note that this doesn't consider the current position,
    /// i.e. checks or pieces blocking the castling move aren't handled here.
    #[must_use]
    pub fn can_castle(self, color: ChessColor, castle_right: CastleRight) -> bool {
        1 == 1 & (self.0 >> (CASTLE_RIGHTS_SHIFT + color as usize * 2 + castle_right as usize))
    }

    const fn set_castle_right_impl(&mut self, color: ChessColor, castle_right: CastleRight, file: u16) {
        self.0 |= file << Self::shift(color, castle_right);
        self.0 |= 1 << (CASTLE_RIGHTS_SHIFT + color as usize * 2 + castle_right as usize);
    }

    pub fn set_castle_right(
        &mut self,
        color: ChessColor,
        castle_right: CastleRight,
        file: DimT,
        settings: &mut ChessSettings,
    ) -> Res<()> {
        debug_assert!((file as usize) < NUM_COLUMNS);
        if self.can_castle(color, castle_right) {
            bail!("Trying to set the {color} {castle_right} castle right twice");
        }
        self.set_castle_right_impl(color, castle_right, u16::from(file));
        if file != A_FILE_NUM && file != H_FILE_NUM {
            settings.set_flag(ChessSettings::dfrc_flag(), true);
        }
        Ok(())
    }

    pub fn unset_castle_right(&mut self, color: ChessColor, castle_right: CastleRight) {
        self.0 &= !(0x1 << ((color as usize * 2 + castle_right as usize) + CASTLE_RIGHTS_SHIFT));
        self.0 &= !(0x7 << Self::shift(color, castle_right));
    }

    pub fn clear_castle_rights(&mut self, color: ChessColor) {
        self.0 &= !(0x3 << (color as usize * 2 + CASTLE_RIGHTS_SHIFT));
        self.0 &= !(0x3f << (color as usize * 6));
    }

    pub(super) fn parse_castling_rights(
        mut self,
        rights: &str,
        board: &mut Chessboard,
        strictness: Strictness,
    ) -> Res<Self> {
        self.0 = 0;
        if rights == "-" {
            return Ok(self);
        } else if rights.is_empty() {
            bail!("Empty castling rights string");
        } else if rights.len() > 4 {
            bail!("Invalid castling rights string: '{rights}' is more than 4 characters long");
        }

        let mut settings = board.settings;
        // will be overwritten when we find an incompatible castling right char
        settings.set_flag(ChessSettings::shredder_fen_flag(), true);
        // Can later be set to true when we try to add a non-corner castling rook or non-e file king
        settings.set_flag(ChessSettings::dfrc_flag(), false);

        for c in rights.chars() {
            let color = if c.is_ascii_uppercase() { White } else { Black };
            let rank = match color {
                White => 0,
                Black => 7,
            };
            // This is a precondition to calling `king_square` below
            let num_kings = board.col_piece_bb(color, King).num_ones();
            if num_kings != 1 {
                bail!("the FEN must contain exactly one {color} king, but instead it contains {num_kings}");
            }
            let king_square = board.king_square(color);
            let king_file = king_square.file();
            if king_square != ChessSquare::from_rank_file(rank, king_file) {
                bail!(
                    "Incorrect starting position for king. The king must be on the back rank, not on square {king_square}"
                );
            }

            let side = |file: DimT| {
                if file < king_file { Queenside } else { Kingside }
            };
            // Unless in strict mode, support normal chess style (aka X-FEN) castling fens for chess960 and disambiguate by using
            // the outermost rook as demanded by <https://en.wikipedia.org/wiki/X-FEN#Encoding_castling_rights>
            // (`verify_position_legal` will return an error if there is no such rook).
            let mut find_rook = |side: CastleRight| {
                let strict_file = match side {
                    Queenside => A_FILE_NUM,
                    Kingside => H_FILE_NUM,
                };
                let rook_on = |file: DimT| {
                    board.is_piece_on(ChessSquare::from_rank_file(rank, file), ColoredChessPieceType::new(color, Rook))
                };
                if strictness == Strict
                    && !UCI_CHESS960.load(Relaxed)
                    && (!rook_on(strict_file) || board.king_square(color).file() != E_FILE_NUM)
                {
                    bail!(
                        "In strict mode, X-FEN chess castle rights ('q' and 'k') can only be used for rooks on the a or h files and\
                        a king on the e file unless the UCI_Chess960 option is set"
                    )
                }
                settings.set_flag(ChessSettings::shredder_fen_flag(), false);
                match side {
                    Queenside => {
                        for file in A_FILE_NUM..king_file {
                            if rook_on(file) {
                                return self.set_castle_right(color, side, file, &mut settings);
                            }
                        }
                    }
                    Kingside => {
                        for file in (king_file..=H_FILE_NUM).rev() {
                            if rook_on(file) {
                                return self.set_castle_right(color, side, file, &mut settings);
                            }
                        }
                    }
                }
                Err(anyhow!("There is no {side} rook to castle with for the {color} player"))
            };
            match c.to_ascii_lowercase() {
                'q' => find_rook(Queenside)?,
                'k' => find_rook(Kingside)?,
                x @ 'a'..='h' => {
                    // verifying the UnverifiedChessboard will ensure there actually is a rook there.
                    let file = char_to_file(x);
                    self.set_castle_right(color, side(file), file, &mut settings)?;
                }
                x => bail!("invalid character in castling rights: '{x}'"),
            }
        }
        for color in ChessColor::iter() {
            let can_castle = self.can_castle(color, Kingside) || self.can_castle(color, Queenside);
            if can_castle && board.king_square(color).file() != E_FILE_NUM {
                settings.set_flag(ChessSettings::dfrc_flag(), true);
            }
        }
        board.settings = settings;
        Ok(self)
    }

    pub(super) fn write_castle_rights(self, f: &mut Formatter, pos: &Chessboard) -> fmt::Result {
        let mut has_castling_rights = false;
        let settings = pos.settings;
        // Always output chess960 castling rights. FEN output isn't necessary for UCI
        // and almost all tools support chess960 FEN notation.
        for color in ChessColor::iter() {
            for side in CastleRight::iter().rev() {
                if self.can_castle(color, side) {
                    has_castling_rights = true;
                    let rook_file = self.rook_start_file(color, side);
                    let rook_on = |file: DimT| {
                        pos.is_piece_on(
                            ChessSquare::from_rank_file(color as DimT * 7, file),
                            ColoredChessPieceType::new(color, Rook),
                        )
                    };
                    let mut file_char;
                    if settings.is_set(ChessSettings::shredder_fen_flag()) {
                        file_char = file_to_char(rook_file)
                    } else {
                        file_char = if side == Kingside { 'k' } else { 'q' };
                        match side {
                            Queenside => {
                                for test_file in A_FILE_NUM..rook_file {
                                    if rook_on(test_file) {
                                        file_char = file_to_char(rook_file)
                                    }
                                }
                            }
                            Kingside => {
                                for test_file in ((rook_file + 1)..=H_FILE_NUM).rev() {
                                    if rook_on(test_file) {
                                        file_char = file_to_char(rook_file)
                                    }
                                }
                            }
                        }
                    }
                    if color == White {
                        file_char = file_char.to_ascii_uppercase();
                    }
                    write!(f, "{file_char}")?;
                }
            }
        }
        if !has_castling_rights {
            write!(f, "-")?;
        }
        Ok(())
    }
}
