use anyhow::{anyhow, bail};
use arbitrary::Arbitrary;
use itertools::Itertools;
use strum_macros::EnumIter;

use crate::games::chess::castling::CastleRight::*;
use crate::games::chess::pieces::ChessPieceType::{King, Rook};
use crate::games::chess::pieces::ColoredChessPieceType;
use crate::games::chess::squares::{
    ChessSquare, A_FILE_NO, C_FILE_NO, D_FILE_NO, F_FILE_NO, G_FILE_NO, H_FILE_NO, NUM_COLUMNS,
};
use crate::games::chess::ChessColor::*;
use crate::games::chess::{ChessColor, Chessboard};
use crate::games::{char_to_file, Board, ColoredPieceType, DimT};
use crate::general::bitboards::RawBitboard;
use crate::general::board::Strictness;
use crate::general::board::Strictness::Strict;
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

#[derive(Eq, PartialEq, Default, Debug, Ord, PartialOrd, Copy, Clone, Arbitrary)]
#[must_use]
/// Stores the queen/kingside castling files for white/black in 3 bits each and uses the upper 4 bits to store
/// if castling is legal. More compact representations are possible because e.e. queenside castling to the h file
/// is impossible, but don't really seem worth it.
pub struct CastlingFlags(u16);

impl CastlingFlags {
    #[must_use]
    pub fn allowed_castling_directions(self) -> usize {
        (self.0 >> 12) as usize
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
        1 == 1
            & (self.allowed_castling_directions() >> (color as usize * 2 + castle_right as usize))
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
        self.0 |= u16::from(file) << Self::shift(color, castle_right);
        self.0 |= 1 << (12 + color as usize * 2 + castle_right as usize);
        Ok(())
    }

    pub fn unset_castle_right(&mut self, color: ChessColor, castle_right: CastleRight) {
        self.0 &= !(0x1 << ((color as usize * 2 + castle_right as usize) + 12));
        self.0 &= !(0x7 << Self::shift(color, castle_right));
    }

    pub fn clear_castle_rights(&mut self, color: ChessColor) {
        self.0 &= !(0x3 << (color as usize * 2 + 12));
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
            // XFEN support isn't a priority
            bail!("Invalid castling rights string: '{rights}' is more than 4 characters long");
        }
        if !rights.chars().all_unique() {
            bail!("Duplicate castling right letter in '{rights}'");
        }

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
                    "FEN must contain exactly one {color} king, but contains {num_kings} instead"
                );
            }
            let king_square = board.king_square(color);
            let king_file = king_square.file();
            if king_square != ChessSquare::from_rank_file(rank, king_file) {
                bail!("Incorrect starting position for king, must be on the back rank, not on square {king_square}");
            }

            let side = |file: DimT| {
                if file < king_file {
                    Queenside
                } else {
                    Kingside
                }
            };
            // Unless in strict mode, support normal chess style castling fens for chess960 and hope it's unambiguous
            // (`verify_position_legal` will return an error if there is no such rook).
            let mut find_rook = |side: CastleRight| {
                let (start, end, strict_file) = match side {
                    Queenside => (A_FILE_NO, king_file, A_FILE_NO),
                    Kingside => (king_file, H_FILE_NO, H_FILE_NO),
                };
                if strictness == Strict
                    && !board.is_piece_on(
                        ChessSquare::from_rank_file(rank, strict_file),
                        ColoredChessPieceType::new(color, Rook),
                    )
                {
                    bail!("In strict mode, normal chess ('q' and 'k') castle rights can only be used for rooks on the a and h file")
                }
                for file in start..end {
                    if board.is_piece_on(
                        ChessSquare::from_rank_file(rank, file),
                        ColoredChessPieceType::new(color, Rook),
                    ) {
                        self.set_castle_right(color, side, file)?;
                        return Ok(());
                    }
                }
                Err(anyhow!(
                    "There is no {side} rook to castle with for the {color} player"
                ))
            };
            match c.to_ascii_lowercase() {
                'q' => find_rook(Queenside)?,
                'k' => {
                    // the `if` isn't just a small performance improvement, it's also necessary if there is a rook on the
                    // g file, for example (for chess960, this is ambiguous, but just picking one is fine).
                    if board.is_piece_on(
                        ChessSquare::from_rank_file(rank, H_FILE_NO),
                        ColoredChessPieceType::new(color, Rook),
                    ) {
                        self.set_castle_right(color, Kingside, H_FILE_NO)?;
                    } else {
                        find_rook(Kingside)?;
                    }
                }
                x @ 'a'..='h' => {
                    let file = char_to_file(x);
                    self.set_castle_right(color, side(file), file)?;
                }
                x => bail!("invalid character in castling rights: '{x}'"),
            }
        }
        Ok(self)
    }
}
