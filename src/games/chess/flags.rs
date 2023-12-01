use itertools::Itertools;
use strum_macros::EnumIter;

use crate::games::chess::flags::CastleRight::*;
use crate::games::Color;
use crate::games::Color::*;

#[derive(EnumIter, Copy, Clone, Eq, PartialEq, Debug, derive_more::Display)]
pub enum CastleRight {
    Queenside,
    Kingside,
}

#[derive(Eq, PartialEq, Default, Debug, Ord, PartialOrd, Copy, Clone)]
pub struct ChessFlags(u8);

impl ChessFlags {
    pub fn castling_flags(self) -> u8 {
        self.0 & 0xf
    }

    pub fn can_castle(self, color: Color, castle_right: CastleRight) -> bool {
        (self.0 >> (color as u8 * 2 + castle_right as u8)) & 1 == 0
    }

    pub fn set_castle_right(&mut self, color: Color, castle_right: CastleRight) {
        self.0 &= !(1 << (color as u8 * 2 + castle_right as u8));
    }

    pub fn unset_castle_right(&mut self, color: Color, castle_right: CastleRight) {
        self.0 |= 1 << (color as u8 * 2 + castle_right as u8);
    }

    pub fn clear_castle_rights(&mut self, color: Color) {
        self.0 |= 0x3 << (color as u8 * 2);
    }

    pub(super) fn parse_castling_rights(mut self, rights: &str) -> Result<Self, String> {
        self.clear_castle_rights(White);
        self.clear_castle_rights(Black);
        if rights == "-" {
            return Ok(self);
        } else if rights.is_empty() {
            return Err("Empty castling rights string".to_string());
        } else if rights.len() > 4 {
            return Err(format!(
                "Invalid castling rights string: '{rights}' contains an invalid letter"
            ));
        }
        if !rights.chars().all_unique() {
            return Err(format!("duplicate castling right letter in '{rights}'"));
        }

        for c in rights.chars() {
            match c {
                // TODO: Support chess960 notation (a-h)
                'K' | 'H' => self.set_castle_right(White, Kingside),
                'k' | 'h' => self.set_castle_right(Black, Kingside),
                'Q' | 'A' => self.set_castle_right(White, Queenside),
                'q' | 'a' => self.set_castle_right(Black, Queenside),
                x => return Err(format!("invalid character in castling rights: '{x}'")),
            }
        }
        Ok(self)
    }
}
