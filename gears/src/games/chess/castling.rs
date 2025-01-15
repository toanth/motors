use anyhow::{anyhow, bail};
use arbitrary::Arbitrary;
use std::fmt;
use std::fmt::Formatter;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::games::chess::castling::CastleRight::*;
use crate::games::chess::pieces::ChessPieceType::{King, Rook};
use crate::games::chess::pieces::ColoredChessPieceType;
use crate::games::chess::squares::{
    ChessSquare, A_FILE_NO, C_FILE_NO, D_FILE_NO, E_FILE_NO, F_FILE_NO, G_FILE_NO, H_FILE_NO,
    NUM_COLUMNS,
};
use crate::games::chess::ChessColor::*;
use crate::games::chess::{ChessColor, Chessboard};
use crate::games::{char_to_file, file_to_char, Board, Color, ColoredPieceType, DimT};
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
            Queenside => C_FILE_NO,
            Kingside => G_FILE_NO,
        }
    }

    #[must_use]
    pub fn rook_dest_file(self) -> DimT {
        match self {
            Queenside => D_FILE_NO,
            Kingside => F_FILE_NO,
        }
    }
}

#[derive(Default, Debug, Copy, Clone, Hash, Arbitrary)]
#[must_use]
/// Stores the queen/kingside castling files for white/black in 3 bits each and uses the upper 4 bits to store
/// if castling is legal. The bit at index 16 is not set iff the castling rights should be printed in X-FEN format
/// (which is backwards compatible to standard FEN, unlike Shredder FEN). This is set to be the format
/// in which the FEN is received (startpos and all non-chess960 FENs are X-FENs for maximum GUI support).
/// X-FENs are disambiguated as described on wikipedia.
/// More compact representations (fitting into 8 bits) are possible because e.g. queenside castling to the h file
/// is impossible, but don't really seem worth it because the size of the [`Chessboard`] doesn't change anyway.
pub struct CastlingFlags(u32);

impl PartialEq for CastlingFlags {
    fn eq(&self, other: &Self) -> bool {
        let ignore_format = (1 << X_FEN_FLAG_SHIFT) | (1 << COMPACT_CASTLING_MOVE_SHIFT);
        self.0 | ignore_format == other.0 | ignore_format
    }
}

impl Eq for CastlingFlags {}

const CASTLE_RIGHTS_SHIFT: usize = 32 - 4;
const X_FEN_FLAG_SHIFT: usize = 16;
const COMPACT_CASTLING_MOVE_SHIFT: usize = 17;

impl CastlingFlags {
    #[must_use]
    pub fn allowed_castling_directions(self) -> usize {
        (self.0 >> CASTLE_RIGHTS_SHIFT) as usize
    }

    /// This is set on finding the letter `q` or `k` in the FEN castling description
    pub fn is_x_fen(&self) -> bool {
        (self.0 >> X_FEN_FLAG_SHIFT) & 1 == 1
    }

    /// This is set alongside is_x_fen, but additionally requires that the castling rights look like normal chess:
    /// All castling rooks must be on the a and h files, and the king must be on the e file.
    pub fn default_uci_castling_move_fmt(&self) -> bool {
        (self.0 >> COMPACT_CASTLING_MOVE_SHIFT) & 1 == 1
    }

    fn shift(color: ChessColor, castle_right: CastleRight) -> usize {
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

    pub fn set_castle_right(
        &mut self,
        color: ChessColor,
        castle_right: CastleRight,
        file: DimT,
    ) -> Res<()> {
        debug_assert!((file as usize) < NUM_COLUMNS);
        if self.can_castle(color, castle_right) {
            bail!("Trying to set the {color} {castle_right} twice");
        }
        self.0 |= u32::from(file) << Self::shift(color, castle_right);
        self.0 |= 1 << (CASTLE_RIGHTS_SHIFT + color as usize * 2 + castle_right as usize);
        if file != 0 && file != 7 {
            self.0 &= !(1 << COMPACT_CASTLING_MOVE_SHIFT);
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

    pub fn parse_castling_rights(
        mut self,
        rights: &str,
        board: &Chessboard,
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

        // output compact castling moves as `<king square><dest square>`. Can get overridden below
        self.0 |= 1 << COMPACT_CASTLING_MOVE_SHIFT;

        for c in rights.chars() {
            let color = if c.is_ascii_uppercase() { White } else { Black };
            let rank = match color {
                White => 0,
                Black => 7,
            };
            // This is a precondition to calling `king_square` below
            let num_kings = board.colored_piece_bb(color, King).num_ones();
            if num_kings != 1 {
                bail!(
                    "the FEN must contain exactly one {color} king, but instead it contains {num_kings}"
                );
            }
            let king_square = board.king_square(color);
            let king_file = king_square.file();
            if king_square != ChessSquare::from_rank_file(rank, king_file) {
                bail!("Incorrect starting position for king. The king must be on the back rank, not on square {king_square}");
            }

            let side = |file: DimT| {
                if file < king_file {
                    Queenside
                } else {
                    Kingside
                }
            };
            // Unless in strict mode, support normal chess style (aka X-FEN) castling fens for chess960 and disambiguate by using
            // the outermost rook as demanded by <https://en.wikipedia.org/wiki/X-FEN#Encoding_castling_rights>
            // (`verify_position_legal` will return an error if there is no such rook).
            let mut find_rook = |side: CastleRight| {
                let strict_file = match side {
                    Queenside => A_FILE_NO,
                    Kingside => H_FILE_NO,
                };
                if strictness == Strict
                    && (!board.is_piece_on(
                        ChessSquare::from_rank_file(rank, strict_file),
                        ColoredChessPieceType::new(color, Rook),
                    ) || board.king_square(color).file() != E_FILE_NO)
                {
                    bail!("In strict mode, normal chess ('q' and 'k') castle rights can only be used for rooks on the a or h files and a king on the e file")
                }
                self.0 |= 1 << X_FEN_FLAG_SHIFT; // this sets the x fen flag
                match side {
                    Queenside => {
                        for file in A_FILE_NO..king_file {
                            if board.is_piece_on(
                                ChessSquare::from_rank_file(rank, file),
                                ColoredChessPieceType::new(color, Rook),
                            ) {
                                return self.set_castle_right(color, side, file);
                            }
                        }
                    }
                    Kingside => {
                        for file in (king_file..=H_FILE_NO).rev() {
                            if board.is_piece_on(
                                ChessSquare::from_rank_file(rank, file),
                                ColoredChessPieceType::new(color, Rook),
                            ) {
                                return self.set_castle_right(color, side, file);
                            }
                        }
                    }
                }
                Err(anyhow!(
                    "There is no {side} rook to castle with for the {color} player"
                ))
            };
            match c.to_ascii_lowercase() {
                'q' => find_rook(Queenside)?,
                'k' => find_rook(Kingside)?,
                x @ 'a'..='h' => {
                    let file = char_to_file(x);
                    self.set_castle_right(color, side(file), file)?;
                }
                x => bail!("invalid character in castling rights: '{x}'"),
            }
        }
        if !self.is_x_fen() {
            self.0 &= !(1 << COMPACT_CASTLING_MOVE_SHIFT);
        }
        for color in ChessColor::iter() {
            if (self.can_castle(color, Kingside) || self.can_castle(color, Queenside))
                && board.king_square(color).file() != E_FILE_NO
            {
                self.0 &= !(1 << COMPACT_CASTLING_MOVE_SHIFT);
            }
        }
        Ok(self)
    }

    pub(super) fn write_castle_rights(self, f: &mut Formatter, pos: &Chessboard) -> fmt::Result {
        let mut has_castling_righs = false;
        // Always output chess960 castling rights. FEN output isn't necessary for UCI
        // and almost all tools support chess960 FEN notation.
        for color in ChessColor::iter() {
            for side in CastleRight::iter().rev() {
                if self.can_castle(color, side) {
                    has_castling_righs = true;
                    let file = self.rook_start_file(color, side);
                    let found_rook = |file: DimT| {
                        pos.is_piece_on(
                            ChessSquare::from_rank_file(color as DimT * 7, file),
                            ColoredChessPieceType::new(color, Rook),
                        )
                    };
                    let mut file_char;
                    if self.is_x_fen() {
                        file_char = if side == Kingside { 'k' } else { 'q' };
                        match side {
                            Queenside => {
                                for file in A_FILE_NO..file {
                                    if found_rook(file) {
                                        file_char = file_to_char(file)
                                    }
                                }
                            }
                            Kingside => {
                                for file in (H_FILE_NO..file).rev() {
                                    if found_rook(file) {
                                        file_char = file_to_char(file)
                                    }
                                }
                            }
                        }
                    } else {
                        file_char = file_to_char(file)
                    };
                    if color == White {
                        file_char = file_char.to_ascii_uppercase();
                    }
                    write!(f, "{file_char}")?;
                }
            }
        }
        if !has_castling_righs {
            write!(f, "-")?;
        }
        Ok(())
    }
}
