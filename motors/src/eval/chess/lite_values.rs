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

use crate::eval::chess::{FileOpenness, NUM_PAWN_CENTER_CONFIGURATIONS, NUM_PAWN_SHIELD_CONFIGURATIONS};
use crate::eval::{ScoreType, SingleFeatureScore};
use gears::games::chess::pieces::PieceType::Rook;
use gears::games::chess::pieces::{PieceType, NUM_CHESS_PIECES};
use gears::games::chess::squares::{Square, NUM_SQUARES};
use gears::games::chess::Color;
use gears::games::chess::Color::White;
use gears::games::DimT;
use gears::general::common::StaticallyNamedEntity;
use gears::score::{p, PhasedScore};
use std::fmt::{Debug, Display};

#[rustfmt::skip]
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  21,   49),    p(  23,   48),    p(  27,   44),    p(  29,   39),    p(  28,   44),    p(  31,   54),    p(   5,   57),    p(   6,   58),
        p( -26,   19),    p( -22,   19),    p( -10,   10),    p( -12,   11),    p( -18,   10),    p(  27,    9),    p(  14,   28),    p(   9,   23),
        p( -39,    0),    p( -32,   -4),    p( -34,  -11),    p( -12,   -7),    p(  -8,   -9),    p(  -9,  -17),    p(  -8,   -7),    p( -10,   -8),
        p( -47,  -13),    p( -44,  -11),    p( -24,  -11),    p( -11,   -8),    p(  -6,   -9),    p(  -7,  -12),    p( -18,  -14),    p( -24,  -18),
        p( -56,  -14),    p( -46,  -18),    p( -28,  -10),    p( -18,   -7),    p( -17,   -7),    p( -14,   -9),    p( -12,  -24),    p( -26,  -20),
        p( -47,   -6),    p( -41,   -9),    p( -35,  -10),    p( -38,   -4),    p( -32,   -2),    p(  -6,  -13),    p(   8,  -23),    p( -16,  -20),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
    ],
    // knight
    [
        p( -63,  -43),    p( -66,    9),    p( -67,   15),    p( -48,   17),    p(  -7,    4),    p( -64,    5),    p( -58,   -5),    p( -62,  -55),
        p( -19,   10),    p(  -7,   16),    p(   8,    5),    p(  13,    8),    p(  14,    1),    p(  23,   -5),    p(   4,    6),    p( -15,   -4),
        p(  -5,    6),    p(  17,    0),    p(  25,    5),    p(  40,    5),    p(  44,    3),    p(  53,   -8),    p(   2,    0),    p(   3,   -1),
        p(  12,   12),    p(  19,    8),    p(  38,   10),    p(  40,   16),    p(  42,   13),    p(  43,    9),    p(  24,   11),    p(  36,    4),
        p(   9,   14),    p(  22,    6),    p(  29,   13),    p(  32,   15),    p(  36,   17),    p(  42,    5),    p(  37,    4),    p(  26,   14),
        p( -21,    0),    p( -10,   -2),    p(  -0,   -3),    p(   8,    9),    p(  12,    7),    p(  10,  -13),    p(   9,   -7),    p(   2,    1),
        p( -28,    6),    p( -14,    8),    p( -11,    3),    p(  -2,    6),    p(   6,    1),    p(   1,   -5),    p(   0,    5),    p(  -3,   14),
        p( -50,   -2),    p( -22,    4),    p( -29,    2),    p(  -8,    6),    p(  -4,    7),    p(  -2,   -5),    p( -15,    7),    p( -23,    3),
    ],
    // bishop
    [
        p( -26,   18),    p( -38,   11),    p( -60,    6),    p( -56,   12),    p( -62,    9),    p( -52,    7),    p( -35,    9),    p( -41,    8),
        p( -15,    5),    p(  -7,    5),    p(   2,    2),    p( -12,    2),    p(  -3,   -3),    p(  -4,   -1),    p( -34,   13),    p( -21,    3),
        p(   1,    7),    p(   8,    3),    p(  -1,    5),    p(   8,   -5),    p(   5,    0),    p(  29,    0),    p(  16,    3),    p(   9,   12),
        p(  -4,    8),    p(   4,    2),    p(  10,   -1),    p(  12,    5),    p(  14,    2),    p(  15,    1),    p(  19,   -0),    p(  -3,    5),
        p(   3,    1),    p(  -1,    4),    p(   6,    2),    p(  15,    1),    p(  16,    1),    p(  18,   -5),    p(   8,   -1),    p(  29,   -8),
        p(  -3,    3),    p(   2,    1),    p(   2,    4),    p(  -0,    3),    p(   5,    4),    p(   4,   -2),    p(  11,   -7),    p(  13,   -1),
        p(   8,   -0),    p(  -0,    1),    p(   6,   -2),    p(  -0,    6),    p(   2,    3),    p(   8,    2),    p(  18,   -5),    p(  12,    1),
        p(   3,   -0),    p(  10,    1),    p(   2,    7),    p(  -7,    5),    p(   6,    6),    p(  -7,    8),    p(   3,    7),    p(  11,   -5),
    ],
    // rook
    [
        p( -36,   45),    p( -40,   47),    p( -49,   53),    p( -52,   51),    p( -43,   46),    p( -29,   49),    p( -35,   49),    p(  -8,   34),
        p( -25,   42),    p( -24,   45),    p( -15,   44),    p(   2,   35),    p( -11,   37),    p(  16,   33),    p(  12,   33),    p(  10,   25),
        p( -29,   37),    p(  -9,   30),    p( -13,   32),    p( -12,   25),    p(  13,   17),    p(  29,   14),    p(  30,   18),    p(  -1,   21),
        p( -30,   35),    p( -20,   30),    p( -20,   31),    p( -13,   25),    p(  -8,   22),    p(   4,   20),    p(  -2,   24),    p(  -9,   20),
        p( -37,   32),    p( -31,   30),    p( -30,   31),    p( -26,   27),    p( -16,   25),    p( -18,   25),    p(  -1,   19),    p( -21,   20),
        p( -43,   27),    p( -36,   23),    p( -37,   24),    p( -34,   24),    p( -26,   18),    p( -17,   13),    p(  -2,    6),    p( -17,   12),
        p( -43,   25),    p( -36,   23),    p( -29,   23),    p( -25,   19),    p( -19,   15),    p(  -8,   10),    p(   1,    4),    p( -34,   17),
        p( -35,   29),    p( -36,   23),    p( -37,   28),    p( -32,   23),    p( -25,   18),    p( -23,   22),    p( -37,   28),    p( -40,   22),
    ],
    // queen
    [
        p( -25,   59),    p( -23,   58),    p( -13,   64),    p(   8,   56),    p(  12,   55),    p(  32,   48),    p(  61,    8),    p(   7,   36),
        p(  11,   44),    p(  10,   47),    p(  16,   52),    p(  13,   64),    p(  10,   70),    p(  34,   57),    p(  24,   54),    p(  54,   32),
        p(  28,   34),    p(  26,   35),    p(  27,   48),    p(  25,   54),    p(  25,   58),    p(  45,   52),    p(  53,   38),    p(  38,   45),
        p(  19,   43),    p(  24,   47),    p(  21,   49),    p(  14,   59),    p(  21,   57),    p(  36,   51),    p(  34,   62),    p(  37,   51),
        p(  17,   43),    p(  17,   47),    p(  16,   48),    p(  17,   57),    p(  27,   54),    p(  28,   52),    p(  36,   49),    p(  37,   46),
        p(  12,   33),    p(  17,   35),    p(  11,   49),    p(  11,   53),    p(  14,   59),    p(  25,   43),    p(  32,   36),    p(  30,   26),
        p(  15,   23),    p(  11,   35),    p(  17,   37),    p(  17,   47),    p(  18,   46),    p(  21,   29),    p(  26,   13),    p(  18,   20),
        p(  -3,   35),    p(   8,   26),    p(   7,   34),    p(  12,   36),    p(  14,   32),    p(   2,   35),    p(  -1,   17),    p(   7,    3),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  67,    8),    p(  66,    5),    p(  69,   -3),    p(  67,  -40),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  52,   18),    p(  25,   27),    p(  16,   21),    p(   3,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  17,   26),    p(  30,   20),    p(  20,   15),    p(  -5,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -36,   24),    p( -22,   18),    p( -25,    6),    p( -39,    8),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -49,   18),    p( -27,    7),    p( -32,   -5),    p( -57,    0),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -40,    9),    p( -38,    1),    p( -19,  -11),    p( -35,    0),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -45,    3),    p( -33,   -3),    p( -11,  -16),    p(   9,   -7),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   6,   -4),    p(  -3,   -1),    p(  17,  -15),    p(  24,  -18),
    ],
];

const MORE_MINORS_NO_PAWNS: PhasedScore = p(-67, -66);
const OPPOSITE_COLORED_BISHOPS: PhasedScore = p(13, -35);
const BISHOP_PAIR: PhasedScore = p(10, 50);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(16, 19), p(17, 14), p(16, 5), p(12, -1), p(7, -7), p(2, -14), p(-4, -20), p(-11, -32), p(-23, -31)];
const BISHOP_CANT_ATTACK: [PhasedScore; 3] = [p(-3, -2), p(-14, 3), p(-6, -0)];
const ROOK_OPEN_FILE: PhasedScore = p(13, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -0);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(4, -2);
const KING_OPEN_FILE: PhasedScore = p(-25, 4);
const KING_CLOSED_FILE: PhasedScore = p(12, -6);
const KING_SEMIOPEN_FILE: PhasedScore = p(-1, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 4), p(-1, 6), p(1, 3), p(3, 3), p(5, 4), p(7, 5), p(14, 1), p(22, -2)],
    // Closed
    [p(0, 0), p(0, 0), p(17, -19), p(-14, 7), p(3, 9), p(2, 3), p(5, 5), p(3, 1)],
    // SemiOpen
    [p(0, 0), p(-9, 23), p(5, 17), p(-1, 10), p(1, 9), p(6, 4), p(8, -1), p(13, 2)],
    // SemiClosed
    [p(0, 0), p(10, -15), p(10, 3), p(4, -1), p(8, 1), p(3, 3), p(9, 2), p(6, -0)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(18, 10),
    p(1, 2),
    p(0, 4),
    p(-7, 4),
    p(4, 1),
    p(-7, -12),
    p(-3, -2),
    p(-2, -14),
    p(-2, -2),
    p(-13, -3),
    p(-7, -17),
    p(-15, -9),
    p(4, -8),
    p(-2, -13),
    p(6, -8),
    p(5, 10),
    p(-6, -2),
    p(-20, -7),
    p(-11, -0),
    p(-34, 18),
    p(-16, 2),
    p(-14, -20),
    p(9, 22),
    p(-38, 24),
    p(-17, -16),
    p(-20, -17),
    p(-32, -30),
    p(-38, 8),
    p(-15, -3),
    p(14, -8),
    p(-63, 69),
    p(0, 0),
    p(0, -4),
    p(-11, -7),
    p(2, -8),
    p(-18, -3),
    p(-19, -3),
    p(-42, -21),
    p(-19, 33),
    p(-23, 18),
    p(-7, -7),
    p(-18, -11),
    p(9, -14),
    p(-13, 31),
    p(-43, 14),
    p(1, -29),
    p(0, 0),
    p(0, 0),
    p(3, -10),
    p(-5, 6),
    p(6, -53),
    p(0, 0),
    p(16, -13),
    p(-27, -14),
    p(0, 0),
    p(0, 0),
    p(-25, -5),
    p(-15, -13),
    p(-7, 12),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(22, 9),
    p(3, 1),
    p(-4, 4),
    p(-17, -3),
    p(10, -4),
    p(-23, -10),
    p(-10, -6),
    p(-28, -18),
    p(7, -1),
    p(-8, -9),
    p(-26, -7),
    p(-38, -3),
    p(-1, -6),
    p(-35, -10),
    p(-28, -11),
    p(-42, 46),
    p(8, 2),
    p(-1, -7),
    p(-4, -8),
    p(-17, -4),
    p(-7, -2),
    p(-14, -16),
    p(-20, -1),
    p(-23, 65),
    p(-7, -9),
    p(-25, -15),
    p(-32, -29),
    p(1, -64),
    p(-11, -11),
    p(-9, -23),
    p(-71, 55),
    p(0, 0),
    p(10, 3),
    p(-3, -2),
    p(-14, -6),
    p(-22, -8),
    p(-1, 0),
    p(-26, -19),
    p(-11, -6),
    p(-23, -6),
    p(-1, -7),
    p(-19, -10),
    p(-23, -16),
    p(-32, -9),
    p(-7, -3),
    p(-45, -12),
    p(16, 5),
    p(-45, 43),
    p(4, 1),
    p(-8, -4),
    p(-21, 50),
    p(0, 0),
    p(-10, -5),
    p(-15, 2),
    p(0, 0),
    p(0, 0),
    p(-13, 0),
    p(-34, 5),
    p(-25, -37),
    p(0, 0),
    p(19, -62),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-8, 8),    /*0b0000*/
    p(-13, 7),   /*0b0001*/
    p(-1, 8),    /*0b0010*/
    p(-2, 7),    /*0b0011*/
    p(-7, 3),    /*0b0100*/
    p(-23, 0),   /*0b0101*/
    p(-7, 2),    /*0b0110*/
    p(-9, -15),  /*0b0111*/
    p(2, 4),     /*0b1000*/
    p(-6, 8),    /*0b1001*/
    p(-1, 8),    /*0b1010*/
    p(5, 7),     /*0b1011*/
    p(-4, 3),    /*0b1100*/
    p(-18, 4),   /*0b1101*/
    p(-7, 1),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(3, 7),     /*0b10000*/
    p(5, 4),     /*0b10001*/
    p(20, 5),    /*0b10010*/
    p(1, 3),     /*0b10011*/
    p(-6, 2),    /*0b10100*/
    p(14, 10),   /*0b10101*/
    p(-24, -1),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(5, 9),     /*0b11000*/
    p(26, 8),    /*0b11001*/
    p(27, 23),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(9, -6),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(7, 1),     /*0b100000*/
    p(-2, 5),    /*0b100001*/
    p(14, 0),    /*0b100010*/
    p(6, -4),    /*0b100011*/
    p(-5, -3),   /*0b100100*/
    p(-21, -10), /*0b100101*/
    p(-16, 15),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(12, -2),   /*0b101000*/
    p(-4, 8),    /*0b101001*/
    p(9, -7),    /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-2, 0),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(2, 1),     /*0b110000*/
    p(13, 0),    /*0b110001*/
    p(19, -9),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(10, 15),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(15, -6),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(8, -7),    /*0b111111*/
    p(3, 6),     /*0b00*/
    p(4, -2),    /*0b01*/
    p(16, -2),   /*0b10*/
    p(-4, -25),  /*0b11*/
    p(21, 2),    /*0b100*/
    p(0, 0),     /*0b101*/
    p(30, -26),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(42, -6),   /*0b1000*/
    p(-7, -12),  /*0b1001*/
    p(13, -28),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(6, -10),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-36, 31),  /*0b1111*/
    p(4, 8),     /*0b00*/
    p(11, -4),   /*0b01*/
    p(10, -7),   /*0b10*/
    p(9, -31),   /*0b11*/
    p(10, -3),   /*0b100*/
    p(24, -12),  /*0b101*/
    p(1, -12),   /*0b110*/
    p(0, 0),     /*0b111*/
    p(15, 2),    /*0b1000*/
    p(23, -13),  /*0b1001*/
    p(27, -38),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(7, -17),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-1, -37),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(-6, -33);
const STOPPABLE_PASSER: PhasedScore = p(14, -40);
const CLOSE_KING_PASSER: PhasedScore = p(-1, 23);
const IMMOBILE_PASSER: PhasedScore = p(-11, -20);
const PROTECTED_PASSER: PhasedScore = p(6, 0);
const PASSER_CAN_PUSH: PhasedScore = p(-10, 25);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -17,   19),    p( -18,   35),    p( -29,   28),    p( -28,   12),    p( -30,   12),    p( -23,    9),    p( -10,   11),    p( -33,   13),
        p( -13,   16),    p( -26,   37),    p( -28,   28),    p( -27,   14),    p( -42,   21),    p( -36,   18),    p( -27,   23),    p( -28,   11),
        p(  -7,   28),    p( -13,   29),    p( -24,   30),    p( -22,   30),    p( -32,   34),    p( -28,   34),    p( -29,   40),    p( -48,   38),
        p(   3,   42),    p(   2,   40),    p(   3,   33),    p(  -8,   42),    p( -23,   49),    p( -15,   53),    p( -18,   56),    p( -35,   55),
        p(  10,   49),    p(  25,   42),    p(  10,   37),    p(  -3,   29),    p(   4,   38),    p(  -6,   52),    p( -18,   53),    p( -42,   58),
        p(  21,   49),    p(  23,   48),    p(  27,   44),    p(  29,   39),    p(  28,   44),    p(  31,   54),    p(   5,   57),    p(   6,   58),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(-2, 4), p(5, 8), p(8, 18), p(13, 25), p(19, 51), p(24, 52)];
const UNSUPPORTED_PAWN: PhasedScore = p(-8, -5);
const DOUBLED_PAWN: PhasedScore = p(-7, -22);
const PHALANX: [PhasedScore; 6] = [p(-2, 1), p(4, 4), p(8, 6), p(20, 20), p(60, 60), p(67, 65)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(15, 14), p(7, 17), p(13, 20), p(7, 10), p(-2, 11), p(-46, 14)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(50, 21), p(53, 42), p(59, 13), p(50, 0), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(2, -2), p(17, 16), p(19, -1), p(17, 9), p(15, -3), p(32, -5)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-42, -53),
        p(-20, -31),
        p(-7, -12),
        p(4, -1),
        p(12, 9),
        p(20, 19),
        p(29, 22),
        p(36, 25),
        p(37, 27),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
    ],
    [
        p(-45, -39),
        p(-28, -33),
        p(-12, -22),
        p(-0, -11),
        p(10, -3),
        p(19, 6),
        p(27, 11),
        p(33, 16),
        p(37, 22),
        p(45, 25),
        p(48, 29),
        p(51, 35),
        p(42, 51),
        p(41, 48),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
    ],
    [
        p(-56, -6),
        p(-41, -3),
        p(-31, -2),
        p(-22, -1),
        p(-17, 3),
        p(-8, 8),
        p(-1, 13),
        p(5, 18),
        p(11, 24),
        p(16, 29),
        p(21, 34),
        p(22, 42),
        p(27, 46),
        p(30, 49),
        p(27, 50),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
    ],
    [
        p(-18, -58),
        p(-26, -45),
        p(-34, -3),
        p(-30, 8),
        p(-26, 21),
        p(-20, 22),
        p(-14, 29),
        p(-8, 33),
        p(-1, 36),
        p(5, 37),
        p(10, 40),
        p(16, 43),
        p(22, 42),
        p(25, 46),
        p(30, 48),
        p(36, 51),
        p(39, 57),
        p(44, 58),
        p(53, 58),
        p(61, 57),
        p(64, 62),
        p(67, 67),
        p(68, 70),
        p(66, 68),
        p(70, 77),
        p(65, 72),
        p(67, 76),
        p(62, 63),
    ],
    [
        p(13, 10),
        p(6, -10),
        p(3, -14),
        p(0, -9),
        p(-3, -4),
        p(-11, 2),
        p(-6, 8),
        p(-10, 16),
        p(2, 17),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
    ],
];

pub const MAX_SAFE_MOBILITY: usize = 14;
const SAFE_SQUARES: [[PhasedScore; MAX_SAFE_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(11, -11),
        p(9, 5),
        p(8, 7),
        p(5, 6),
        p(2, 5),
        p(-0, 4),
        p(-2, -0),
        p(4, -6),
        p(12, -18),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
    ],
    [
        p(20, -9),
        p(15, 3),
        p(8, 7),
        p(3, 7),
        p(-1, 6),
        p(-3, 5),
        p(-5, 1),
        p(-6, -1),
        p(-7, -6),
        p(-10, -7),
        p(5, -17),
        p(-7, -22),
        p(4, -45),
        p(65, -36),
        p(0, 0),
    ],
    [
        p(1, 5),
        p(-8, 22),
        p(-15, 30),
        p(-22, 33),
        p(-29, 35),
        p(-35, 36),
        p(-39, 34),
        p(-42, 32),
        p(-42, 29),
        p(-39, 25),
        p(-32, 19),
        p(-12, 14),
        p(4, 5),
        p(38, -2),
        p(63, -40),
    ],
    [
        p(19, 5),
        p(28, 30),
        p(32, 36),
        p(31, 41),
        p(28, 48),
        p(24, 54),
        p(20, 57),
        p(16, 59),
        p(12, 60),
        p(10, 59),
        p(9, 54),
        p(8, 51),
        p(10, 49),
        p(10, 43),
        p(35, 14),
    ],
    [
        p(-40, -27),
        p(-24, -5),
        p(-13, 5),
        p(-1, 8),
        p(17, 7),
        p(33, 3),
        p(35, 5),
        p(51, 1),
        p(72, -12),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
    ],
];
const THREATS: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(-5, 13), p(0, 0), p(25, 25), p(54, 8), p(35, -6), p(0, 0)],
    [p(-2, 13), p(18, 27), p(0, 0), p(40, 19), p(46, 63), p(0, 0)],
    [p(-9, 22), p(4, 23), p(12, 19), p(0, 0), p(52, 54), p(0, 0)],
    [p(-4, 18), p(-2, 21), p(-2, 31), p(-1, 18), p(0, 0), p(0, 0)],
    [p(55, 19), p(-12, 20), p(22, 11), p(0, 15), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 8), p(7, 7), p(6, 9), p(11, 7), p(5, 17), p(2, 7)],
    [p(2, 9), p(10, 22), p(-61, -32), p(8, 13), p(9, 23), p(2, 6)],
    [p(-3, 8), p(7, 13), p(3, 18), p(7, 13), p(4, 35), p(12, -3)],
    [p(-0, 9), p(5, 11), p(5, 6), p(3, 18), p(-71, -68), p(-0, -8)],
    [p(24, 3), p(12, 14), p(18, 8), p(6, 9), p(18, 1), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(0, -19), p(2, -12), p(1, -8), p(6, -23), p(1, -7), p(-17, -0)];
const DOUBLE_KINGZONE_ATTACK: PhasedScore = p(51, 3);
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(4, -2), p(-10, 7), p(0, -8), p(-24, 14)];
const SAFE_CHECK: [PhasedScore; 5] = [p(0, 0), p(56, -7), p(12, 1), p(42, -4), p(27, 42)];
const CHECK_STM: PhasedScore = p(36, 14);
const SAFE_CHECK_STM: PhasedScore = p(35, 5);
const DISCOVERED_CHECK_STM: PhasedScore = p(65, 66);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(11, -3), p(63, 33), p(65, 22), p(67, 68), p(0, 0), p(55, -28)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(12, -7), p(34, 40), p(31, 43), p(49, 24), p(60, 19)];
const MATERIAL: [PhasedScore; NUM_CHESS_PIECES + 1] =
    [p(100, 100), p(300, 300), p(300, 300), p(500, 500), p(900, 900), p(0, 0), p(0, 0)];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static + StaticallyNamedEntity {
    type Score: ScoreType;

    fn material(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn psqt(&self, square: Square, piece: PieceType, color: Color) -> SingleFeatureScore<Self::Score>;

    fn more_minors_but_no_pawns() -> SingleFeatureScore<Self::Score>;

    fn opposite_colored_bishops() -> SingleFeatureScore<Self::Score>;

    fn passed_pawn(square: Square) -> SingleFeatureScore<Self::Score>;

    fn stoppable_passer() -> SingleFeatureScore<Self::Score>;

    fn close_king_passer() -> SingleFeatureScore<Self::Score>;

    fn immobile_passer() -> SingleFeatureScore<Self::Score>;

    fn passer_protection() -> SingleFeatureScore<Self::Score>;

    fn passer_can_push() -> SingleFeatureScore<Self::Score>;

    fn candidate_passer(rank: DimT) -> SingleFeatureScore<Self::Score>;

    fn unsupported_pawn() -> SingleFeatureScore<Self::Score>;

    fn doubled_pawn() -> SingleFeatureScore<Self::Score>;

    fn phalanx(rank: DimT) -> SingleFeatureScore<Self::Score>;

    fn bishop_pair() -> SingleFeatureScore<Self::Score>;

    fn bad_bishop(num_pawns: usize) -> SingleFeatureScore<Self::Score>;

    fn bishop_cant_attack(major_piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn rook_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score>;

    fn king_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score>;

    fn bishop_openness(openness: FileOpenness, len: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_advanced_center(config: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_passive_center(config: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_shield(&self, color: Color, config: usize) -> SingleFeatureScore<Self::Score>;

    fn pawnless_flank() -> SingleFeatureScore<Self::Score>;

    fn pawn_protection(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn pawn_attack(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn pawn_advance_threat(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn mobility(piece: PieceType, mobility: usize) -> SingleFeatureScore<Self::Score>;

    fn safe_squares(piece: PieceType, num_squares: usize) -> SingleFeatureScore<Self::Score>;

    fn threats(attacking: PieceType, targeted: PieceType) -> SingleFeatureScore<Self::Score>;

    fn defended(protecting: PieceType, target: PieceType) -> SingleFeatureScore<Self::Score>;

    fn double_kingzone_attack() -> SingleFeatureScore<Self::Score>;

    fn king_zone_attack(attacking: PieceType) -> SingleFeatureScore<Self::Score>;

    fn can_give_check(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn safe_check(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn check_stm() -> SingleFeatureScore<Self::Score>;

    fn safe_check_stm() -> SingleFeatureScore<Self::Score>;

    fn discovered_check_stm() -> SingleFeatureScore<Self::Score>;

    fn discovered_check(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn pin(piece: PieceType) -> SingleFeatureScore<Self::Score>;
}

/// Eval values tuned on a combination of the lichess-big-3-resolved dataset and a dataset used by 4ku,
/// created by GCP using his engine Stoofvlees and filtered by cj5716 using Stockfish at depth 9,
/// using my own tuner `pliers`.
#[derive(Debug, Default, Copy, Clone)]
pub struct Lite {}

impl StaticallyNamedEntity for Lite {
    fn static_short_name() -> impl Display
    where
        Self: Sized,
    {
        "LiTE"
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        "Chess LiTE: Linear Tuned Eval for Chess".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "A classical evaluation for chess, tuned using 'pliers'".to_string()
    }
}

impl LiteValues for Lite {
    type Score = PhasedScore;

    fn material(piece: PieceType) -> PhasedScore {
        MATERIAL[piece as usize]
    }

    fn psqt(&self, square: Square, piece: PieceType, color: Color) -> PhasedScore {
        PSQTS[piece as usize][square.flip_if(color == White).bb_idx()]
    }

    fn more_minors_but_no_pawns() -> PhasedScore {
        MORE_MINORS_NO_PAWNS
    }

    fn opposite_colored_bishops() -> PhasedScore {
        OPPOSITE_COLORED_BISHOPS
    }

    fn passed_pawn(square: Square) -> PhasedScore {
        PASSED_PAWNS[square.bb_idx()]
    }

    fn stoppable_passer() -> PhasedScore {
        STOPPABLE_PASSER
    }

    fn close_king_passer() -> PhasedScore {
        CLOSE_KING_PASSER
    }

    fn immobile_passer() -> PhasedScore {
        IMMOBILE_PASSER
    }

    fn passer_protection() -> PhasedScore {
        PROTECTED_PASSER
    }

    fn passer_can_push() -> PhasedScore {
        PASSER_CAN_PUSH
    }
    fn candidate_passer(rank: DimT) -> PhasedScore {
        CANDIDATE_PASSER[rank as usize]
    }

    fn unsupported_pawn() -> PhasedScore {
        UNSUPPORTED_PAWN
    }

    fn doubled_pawn() -> PhasedScore {
        DOUBLED_PAWN
    }

    fn phalanx(rank: DimT) -> PhasedScore {
        PHALANX[rank as usize]
    }

    fn bishop_pair() -> PhasedScore {
        BISHOP_PAIR
    }

    fn bad_bishop(num_pawns: usize) -> PhasedScore {
        BAD_BISHOP[num_pawns]
    }

    fn bishop_cant_attack(major_piece: PieceType) -> PhasedScore {
        BISHOP_CANT_ATTACK[major_piece as usize - Rook as usize]
    }

    fn rook_openness(openness: FileOpenness) -> PhasedScore {
        match openness {
            FileOpenness::Open => ROOK_OPEN_FILE,
            FileOpenness::Closed => ROOK_CLOSED_FILE,
            FileOpenness::SemiOpen => ROOK_SEMIOPEN_FILE,
            FileOpenness::SemiClosed => PhasedScore::default(),
        }
    }

    fn king_openness(openness: FileOpenness) -> PhasedScore {
        match openness {
            FileOpenness::Open => KING_OPEN_FILE,
            FileOpenness::Closed => KING_CLOSED_FILE,
            FileOpenness::SemiOpen => KING_SEMIOPEN_FILE,
            FileOpenness::SemiClosed => PhasedScore::default(),
        }
    }

    fn bishop_openness(openness: FileOpenness, len: usize) -> PhasedScore {
        BISHOP_OPENNESS[openness as usize][len - 1]
    }

    fn pawn_advanced_center(config: usize) -> PhasedScore {
        PAWN_ADVANCED_CENTER[config]
    }

    fn pawn_passive_center(config: usize) -> PhasedScore {
        PAWN_PASSIVE_CENTER[config]
    }

    fn pawn_shield(&self, _color: Color, config: usize) -> PhasedScore {
        PAWN_SHIELDS[config]
    }

    fn pawnless_flank() -> PhasedScore {
        PAWNLESS_FLANK
    }

    fn pawn_protection(piece: PieceType) -> PhasedScore {
        PAWN_PROTECTION[piece as usize]
    }

    fn pawn_attack(piece: PieceType) -> PhasedScore {
        PAWN_ATTACKS[piece as usize]
    }

    fn pawn_advance_threat(piece: PieceType) -> PhasedScore {
        PAWN_ADVANCE_THREAT[piece as usize]
    }

    fn mobility(piece: PieceType, mobility: usize) -> PhasedScore {
        MOBILITY[piece as usize - 1][mobility]
    }

    fn safe_squares(piece: PieceType, num_squares: usize) -> PhasedScore {
        SAFE_SQUARES[piece as usize - 1][num_squares]
    }

    fn threats(attacking: PieceType, targeted: PieceType) -> PhasedScore {
        THREATS[attacking as usize - 1][targeted as usize]
    }

    fn defended(protecting: PieceType, target: PieceType) -> PhasedScore {
        DEFENDED[protecting as usize - 1][target as usize]
    }

    fn double_kingzone_attack() -> PhasedScore {
        DOUBLE_KINGZONE_ATTACK
    }

    fn king_zone_attack(attacking: PieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }

    fn can_give_check(piece: PieceType) -> PhasedScore {
        CAN_GIVE_CHECK[piece as usize]
    }

    fn safe_check(piece: PieceType) -> PhasedScore {
        SAFE_CHECK[piece as usize]
    }

    fn discovered_check_stm() -> PhasedScore {
        DISCOVERED_CHECK_STM
    }

    fn pin(piece: PieceType) -> PhasedScore {
        PIN[piece as usize]
    }

    fn discovered_check(piece: PieceType) -> PhasedScore {
        DISCOVERED_CHECK[piece as usize]
    }

    fn check_stm() -> PhasedScore {
        CHECK_STM
    }

    fn safe_check_stm() -> PhasedScore {
        SAFE_CHECK_STM
    }
}
