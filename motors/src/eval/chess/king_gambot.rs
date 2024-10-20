/*
 *  Motors, a collection of board game engines.
 *  Copyright (C) 2024 ToTheAnd
 *
 *  Motors is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Motors is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Motors. If not, see <https://www.gnu.org/licenses/>.
 */
use crate::eval::chess::lite_values::{Lite, LiteValues};
use crate::eval::chess::FileOpenness;
use gears::games::chess::pieces::ChessPieceType;
use gears::games::chess::pieces::ChessPieceType::King;
use gears::games::chess::squares::ChessSquare;
use gears::games::chess::ChessColor;
use gears::games::chess::ChessColor::White;
use gears::general::common::StaticallyNamedEntity;
use gears::score::{p, PhasedScore};
use std::fmt::Display;

#[rustfmt::skip]
const KING_GAMBOT_VALUES: [PhasedScore; 64] =   [
    p(550, 200),    p(550, 200),    p(550, 200),    p(550, 200),    p(550, 200),    p(550, 200),    p(550, 200),    p(550, 200),
    p(500, 150),    p(500, 150),    p(500, 150),    p(500, 150),    p(500, 150),    p(500, 150),    p(500, 150),    p(500, 150),
    p(450, 100),    p(450, 100),    p(450, 100),    p(450, 100),    p(450, 100),    p(450, 100),    p(450, 100),    p(450, 100),
    p(400, 0),      p(400, 0),      p(400, 0),      p(400, 0),      p(400, 0),      p(400, 0),      p(400, 0),      p(400, 0),
    p(300, -100),   p(300, -100),   p(300, -100),   p(300, -100),   p(300, -100),   p(300, -100),   p(300, -100),   p(300, -100),
    p(200, -200),   p(200, -200),   p(200, -200),   p(200, -200),   p(200, -200),   p(200, -200),   p(200, -200),   p(200, -200),
    p(0, -200),     p(0, -200),     p(0, -200),     p(0, -200),     p(0, -200),     p(0, -200),     p(0, -200),     p(0, -200),
    p(-200, -200),  p(-200, -200),  p(-200, -200),  p(-200, -200),  p(-200, -200),  p(-200, -200),  p(-200, -200),  p(-200, -200),
];

#[derive(Debug, Default, Copy, Clone)]
pub struct KingGambotValues {
    pub us: ChessColor,
}

impl StaticallyNamedEntity for KingGambotValues {
    fn static_short_name() -> impl Display
    where
        Self: Sized,
    {
        "King_Gambot"
    }

    // Don't send 'Gᴀᴍʙᴏᴛ' because not all GUIs handle unicode characters well
    // so no mention of 'King Gᴀᴍʙᴏᴛ Ⅳ'
    fn static_long_name() -> String
    where
        Self: Sized,
    {
        "King Gambot".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "The King Leads his Army. More aggressive than the 1024 token challenge engine".to_string()
    }
}

impl LiteValues for KingGambotValues {
    type Score = PhasedScore;

    fn psqt(&self, square: ChessSquare, piece: ChessPieceType, color: ChessColor) -> PhasedScore {
        if color == self.us && piece == King {
            KING_GAMBOT_VALUES[square.flip_if(color == White).bb_idx()]
        } else {
            Lite::default().psqt(square, piece, color)
        }
    }

    fn passed_pawn(square: ChessSquare) -> PhasedScore {
        Lite::passed_pawn(square)
    }

    fn unsupported_pawn() -> PhasedScore {
        Lite::unsupported_pawn()
    }

    fn doubled_pawn() -> PhasedScore {
        Lite::doubled_pawn()
    }

    fn bishop_pair() -> PhasedScore {
        Lite::bishop_pair()
    }

    fn rook_openness(openness: FileOpenness) -> PhasedScore {
        Lite::rook_openness(openness)
    }

    fn king_openness(openness: FileOpenness) -> PhasedScore {
        Lite::king_openness(openness)
    }

    fn bishop_openness(openness: FileOpenness, len: usize) -> PhasedScore {
        Lite::bishop_openness(openness, len)
    }

    fn pawn_shield(&self, color: ChessColor, config: usize) -> PhasedScore {
        let value = Lite::default().pawn_shield(color, config) / 2;
        if self.us == color {
            value / 2
        } else {
            value
        }
    }

    fn pawn_protection(piece: ChessPieceType) -> PhasedScore {
        Lite::pawn_protection(piece)
    }

    fn pawn_attack(piece: ChessPieceType) -> PhasedScore {
        Lite::pawn_attack(piece)
    }

    fn mobility(piece: ChessPieceType, mobility: usize) -> PhasedScore {
        Lite::mobility(piece, mobility)
    }

    fn threats(attacking: ChessPieceType, targeted: ChessPieceType) -> PhasedScore {
        Lite::threats(attacking, targeted)
    }

    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> PhasedScore {
        Lite::defended(protecting, target)
    }

    fn king_zone_attack(attacking: ChessPieceType) -> PhasedScore {
        Lite::king_zone_attack(attacking)
    }
}
