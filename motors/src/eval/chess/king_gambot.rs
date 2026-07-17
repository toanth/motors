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
use gears::games::chess::pieces::PieceType;
use gears::games::chess::pieces::PieceType::King;
use gears::games::chess::squares::Square;
use gears::games::chess::Color::{Black, White};
use gears::games::chess::{Board, Color};
use gears::games::DimT;
use gears::general::board::BoardTrait;
use gears::general::common::StaticallyNamedEntity;
use gears::general::squares::manhattan_distance;
use gears::score::{p, PhasedScore, Score, ScoreT};
use std::fmt::Display;

#[rustfmt::skip]
const KING_GAMBOT_VALUES: [PhasedScore; 64] =   [
    p(850, 200),    p(850, 200),    p(870, 200),    p(900, 200),    p(900, 200),    p(870, 200),    p(850, 200),    p(850, 200),
    p(700, 250),    p(700, 250),    p(720, 250),    p(750, 250),    p(750, 250),    p(720, 250),    p(700, 250),    p(700, 250),
    p(600, 200),    p(600, 200),    p(620, 200),    p(650, 200),    p(650, 200),    p(620, 200),    p(600, 200),    p(600, 200),
    p(450, 100),    p(450, 100),    p(470, 100),    p(500, 100),    p(500, 100),    p(470, 100),    p(450, 100),    p(450, 100),
    p(300, 0),      p(300, 0),      p(320, 0),      p(350, 0),      p(350, 0),      p(320, 0),      p(300, 0),      p(300, 0),
    p(100, -100),   p(100, -100),   p(120, -100),   p(150, -100),   p(150, -100),   p(120, -100),   p(100, -100),   p(100, -100),
    p(-100, -200),  p(-100, -200),  p(-80, -200),   p(-50, -200),   p(-50, -200),   p(-80, -200),   p(-100, -200),  p(-100, -200),
    p(-300, -400),  p(-300, -400),  p(-280, -400),  p(-250, -400),  p(-250, -400),  p(-280, -400),  p(-300, -400),  p(-300, -400),
];

#[derive(Debug, Default, Copy, Clone)]
pub struct KingGambotValues {
    pub us: Color,
}

impl KingGambotValues {
    pub fn king_closeness(&self, pos: &Board) -> Score {
        let dist = manhattan_distance(pos.king_sq(White), pos.king_sq(Black));
        let score = Score(256 - (dist as ScoreT) * 16);
        if pos.active_player() == self.us { score } else { -score }
    }
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

    fn material(piece: PieceType) -> PhasedScore {
        Lite::material(piece)
    }

    fn psqt(&self, square: Square, piece: PieceType, color: Color) -> PhasedScore {
        if color == self.us && piece == King {
            KING_GAMBOT_VALUES[square.flip_if(color == White).bb_idx()]
        } else {
            Lite::default().psqt(square, piece, color)
        }
    }

    fn more_minors_but_no_pawns() -> PhasedScore {
        Lite::more_minors_but_no_pawns()
    }

    fn opposite_colored_bishops() -> PhasedScore {
        Lite::opposite_colored_bishops()
    }

    fn passed_pawn(square: Square) -> PhasedScore {
        Lite::passed_pawn(square)
    }

    fn stoppable_passer() -> PhasedScore {
        Lite::stoppable_passer()
    }

    fn close_king_passer() -> PhasedScore {
        Lite::close_king_passer()
    }

    fn immobile_passer() -> PhasedScore {
        Lite::immobile_passer()
    }

    fn passer_protection() -> PhasedScore {
        Lite::passer_protection()
    }

    fn passer_can_push() -> PhasedScore {
        Lite::passer_can_push()
    }

    fn candidate_passer(rank: DimT) -> PhasedScore {
        Lite::candidate_passer(rank)
    }

    fn unsupported_pawn() -> PhasedScore {
        Lite::unsupported_pawn()
    }

    fn doubled_pawn() -> PhasedScore {
        Lite::doubled_pawn()
    }

    fn phalanx(rank: DimT) -> PhasedScore {
        Lite::phalanx(rank)
    }

    fn bishop_pair() -> PhasedScore {
        Lite::bishop_pair()
    }

    fn bad_bishop(num_pawns: usize) -> PhasedScore {
        Lite::bad_bishop(num_pawns)
    }

    fn rook_openness(openness: FileOpenness) -> PhasedScore {
        Lite::rook_openness(openness)
    }

    fn king_openness(openness: FileOpenness) -> PhasedScore {
        Lite::king_openness(openness) / 2
    }

    fn bishop_openness(openness: FileOpenness, len: usize) -> PhasedScore {
        Lite::bishop_openness(openness, len)
    }

    fn pawn_advanced_center(config: usize) -> PhasedScore {
        Lite::pawn_advanced_center(config)
    }

    fn pawn_passive_center(config: usize) -> PhasedScore {
        Lite::pawn_passive_center(config)
    }

    fn pawn_shield(&self, color: Color, config: usize) -> PhasedScore {
        let value = Lite::default().pawn_shield(color, config);
        if self.us == color { value / 2 } else { value }
    }

    fn pawnless_flank() -> PhasedScore {
        Lite::pawnless_flank()
    }

    fn pawn_protection(piece: PieceType) -> PhasedScore {
        Lite::pawn_protection(piece)
    }

    fn pawn_attack(piece: PieceType) -> PhasedScore {
        Lite::pawn_attack(piece)
    }

    fn pawn_advance_threat(piece: PieceType) -> PhasedScore {
        Lite::pawn_advance_threat(piece)
    }

    fn mobility(piece: PieceType, mobility: usize) -> PhasedScore {
        Lite::mobility(piece, mobility)
    }

    fn safe_squares(piece: PieceType, num: usize) -> PhasedScore {
        Lite::safe_squares(piece, num)
    }

    fn threats(attacking: PieceType, targeted: PieceType) -> PhasedScore {
        Lite::threats(attacking, targeted)
    }

    fn defended(protecting: PieceType, target: PieceType) -> PhasedScore {
        Lite::defended(protecting, target)
    }

    fn double_kingzone_attack() -> PhasedScore {
        Lite::double_kingzone_attack() / 2
    }

    fn king_zone_attack(attacking: PieceType) -> PhasedScore {
        Lite::king_zone_attack(attacking) / 2
    }

    fn can_give_check(piece: PieceType) -> PhasedScore {
        Lite::can_give_check(piece) / 2
    }

    fn safe_check(piece: PieceType) -> PhasedScore {
        Lite::safe_check(piece) / 2
    }

    fn pin(piece: PieceType) -> PhasedScore {
        Lite::pin(piece)
    }

    fn discovered_check(piece: PieceType) -> PhasedScore {
        Lite::discovered_check(piece) / 2
    }

    fn discovered_check_stm() -> PhasedScore {
        Lite::discovered_check_stm() / 2
    }

    fn check_stm() -> PhasedScore {
        Lite::check_stm() / 2
    }

    fn safe_check_stm() -> PhasedScore {
        Lite::safe_check_stm() / 2
    }
}
