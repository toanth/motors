use itertools::Itertools;
use strum_macros::EnumIter;

use crate::games::chess::castling::CastleRight::*;
use crate::games::chess::pieces::ColoredChessPiece;
use crate::games::chess::pieces::UncoloredChessPiece::Rook;
use crate::games::chess::squares::{
    ChessSquare, A_FILE_NO, C_FILE_NO, D_FILE_NO, F_FILE_NO, G_FILE_NO, H_FILE_NO, NUM_COLUMNS,
};
use crate::games::chess::Chessboard;
use crate::games::Color::*;
use crate::games::{char_to_file, Board, Color, ColoredPieceType, DimT};
use crate::general::common::Res;

#[derive(EnumIter, Copy, Clone, Eq, PartialEq, Debug, derive_more::Display)]
pub enum CastleRight {
    Queenside,
    Kingside,
}

impl CastleRight {
    pub fn king_dest_file(self) -> DimT {
        match self {
            Queenside => C_FILE_NO,
            Kingside => G_FILE_NO,
        }
    }

    pub fn rook_dest_file(self) -> DimT {
        match self {
            Queenside => D_FILE_NO,
            Kingside => F_FILE_NO,
        }
    }
}

#[derive(Eq, PartialEq, Default, Debug, Ord, PartialOrd, Copy, Clone)]
/// Stores the queen/kingside castling files for white/black in 3 bits each and uses the upper 4 bits to store
/// if castling is legal. More compact representations are possible because e.e. queenside castling to the h file
/// is impossible, but don't really seem worth it.
pub struct CastlingFlags(u16);

impl CastlingFlags {
    pub fn allowed_castling_directions(self) -> usize {
        (self.0 >> 12) as usize
    }

    fn shift(color: Color, castle_right: CastleRight) -> usize {
        color as usize * 6 + castle_right as usize * 3
    }

    /// This return value of this function can only be used if `can_castle` would return `true`.
    pub fn rook_start_file(self, color: Color, castle_right: CastleRight) -> DimT {
        ((self.0 >> Self::shift(color, castle_right)) & 0x7) as DimT
    }

    /// Returns true iff castling rights haven't been lost. Note that this doesn't consider the current position,
    /// i.e. checks or pieces blocking the castling move aren't handled here.
    pub fn can_castle(self, color: Color, castle_right: CastleRight) -> bool {
        1 == 1
            & (self.allowed_castling_directions() >> (color as usize * 2 + castle_right as usize))
    }

    pub(super) fn set_castle_right(
        &mut self,
        color: Color,
        castle_right: CastleRight,
        file: DimT,
    ) -> Res<()> {
        debug_assert!((file as usize) < NUM_COLUMNS);
        if self.can_castle(color, castle_right) {
            return Err(format!("Trying to set the {color} {castle_right} twice"));
        }
        self.0 |= (file as u16) << Self::shift(color, castle_right);
        self.0 |= 1 << (12 + color as usize * 2 + castle_right as usize);
        Ok(())
    }

    pub(super) fn unset_castle_right(&mut self, color: Color, castle_right: CastleRight) {
        self.0 &= !(0x1 << ((color as usize * 2 + castle_right as usize) + 12));
        self.0 &= !(0x7 << Self::shift(color, castle_right));
    }

    pub(super) fn clear_castle_rights(&mut self, color: Color) {
        self.0 &= !(0x3 << (color as usize * 2 + 12));
        self.0 &= !(0x3f << (color as usize * 6));
    }

    pub(super) fn parse_castling_rights(mut self, rights: &str, board: &Chessboard) -> Res<Self> {
        self.0 = 0;
        if rights == "-" {
            return Ok(self);
        } else if rights.is_empty() {
            return Err("Empty castling rights string".to_string());
        } else if rights.len() > 4 {
            // XFEN support isn't a priority
            return Err(format!(
                "Invalid castling rights string: '{rights}' is more than 4 characters long"
            ));
        }
        if !rights.chars().all_unique() {
            return Err(format!("Duplicate castling right letter in '{rights}'"));
        }

        for c in rights.chars() {
            let color = if c.is_ascii_uppercase() { White } else { Black };
            let rank = match color {
                White => 0,
                Black => 7,
            };
            let king_file = board.king_square(color).file();
            debug_assert_eq!(
                board.king_square(color),
                ChessSquare::from_rank_file(rank, king_file)
            );
            let side = |file: DimT| {
                if file < king_file {
                    Queenside
                } else {
                    Kingside
                }
            };
            // support normal chess style castling fens for chess960 and hope it's unambiguous
            // (`verify_position_legal` will return an error if there is no such rook).
            let mut find_rook = |side: CastleRight| {
                let (start, end) = match side {
                    Queenside => (A_FILE_NO, king_file),
                    Kingside => (king_file, H_FILE_NO),
                };
                for file in start..end {
                    if board.is_piece_on(
                        ChessSquare::from_rank_file(rank, file),
                        ColoredChessPiece::new(color, Rook),
                    ) {
                        self.set_castle_right(color, side, file)?;
                        return Ok(());
                    }
                }
                Err(format!(
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
                        ColoredChessPiece::new(color, Rook),
                    ) {
                        self.set_castle_right(color, Kingside, H_FILE_NO)?;
                    } else {
                        find_rook(Kingside)?
                    }
                }
                x @ 'a'..='h' => {
                    let file = char_to_file(x);
                    self.set_castle_right(color, side(file), file)?;
                }
                x => return Err(format!("invalid character in castling rights: '{x}'")),
            }
        }
        Ok(self)
    }
}
