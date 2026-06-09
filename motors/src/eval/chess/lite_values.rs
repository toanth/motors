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
        p(  19,   51),    p(  21,   50),    p(  24,   46),    p(  27,   41),    p(  25,   45),    p(  30,   55),    p(   1,   59),    p(   5,   60),
        p( -29,   22),    p( -22,   21),    p( -12,   11),    p( -13,   12),    p( -18,   10),    p(  26,    9),    p(  18,   30),    p(  10,   26),
        p( -43,    0),    p( -34,   -4),    p( -36,  -11),    p( -13,   -7),    p(  -9,   -9),    p(  -9,  -18),    p(  -7,   -7),    p(  -7,   -9),
        p( -49,  -12),    p( -45,  -10),    p( -25,  -11),    p( -11,   -8),    p(  -6,   -9),    p(  -7,  -12),    p( -15,  -14),    p( -20,  -19),
        p( -58,  -13),    p( -48,  -16),    p( -30,   -9),    p( -18,   -7),    p( -18,   -6),    p( -15,   -9),    p( -11,  -24),    p( -23,  -21),
        p( -48,   -6),    p( -41,   -8),    p( -36,   -9),    p( -37,   -4),    p( -32,   -1),    p(  -5,  -12),    p(  12,  -23),    p( -13,  -21),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
    ],
    // knight
    [
        p( -62,  -45),    p( -65,    5),    p( -66,    9),    p( -50,   11),    p(  -8,   -1),    p( -64,    0),    p( -56,  -10),    p( -62,  -57),
        p( -19,    6),    p(  -7,   12),    p(   5,    1),    p(   8,    5),    p(  12,   -1),    p(  22,   -9),    p(   1,    3),    p( -15,   -7),
        p(  -7,    3),    p(  11,   -1),    p(  15,    5),    p(  26,    5),    p(  37,    3),    p(  54,   -8),    p(   1,   -1),    p(   2,   -3),
        p(  13,    8),    p(  20,    5),    p(  34,    9),    p(  33,   15),    p(  33,   13),    p(  35,   11),    p(  25,   10),    p(  35,    1),
        p(   7,    8),    p(  18,    2),    p(  21,    9),    p(  29,   11),    p(  27,   13),    p(  35,   -0),    p(  33,   -1),    p(  23,    7),
        p( -15,   -4),    p(  -8,   -5),    p(   1,   -6),    p(  10,    5),    p(  14,    3),    p(  12,  -17),    p(  13,  -11),    p(   7,   -3),
        p( -22,    2),    p(  -9,    5),    p(  -6,   -0),    p(   3,    1),    p(  10,   -3),    p(   5,   -7),    p(   8,    1),    p(   3,    9),
        p( -45,   -5),    p( -14,   -2),    p( -23,   -2),    p(  -2,    3),    p(   4,    3),    p(   4,   -7),    p(  -7,    1),    p( -21,    0),
    ],
    // bishop
    [
        p( -24,   16),    p( -39,    9),    p( -62,    5),    p( -60,   12),    p( -64,    9),    p( -59,    6),    p( -39,    9),    p( -41,    7),
        p( -11,    2),    p( -11,    4),    p(  -0,    2),    p( -15,    2),    p(  -7,   -2),    p(   1,   -2),    p( -39,   12),    p( -20,    1),
        p(   1,    6),    p(  11,    1),    p(  -3,    4),    p(   7,   -5),    p(   7,    0),    p(  38,   -1),    p(  23,    3),    p(  17,   11),
        p(  -4,    6),    p(   3,    2),    p(  11,   -3),    p(  12,    4),    p(  12,    1),    p(  12,    1),    p(  17,   -2),    p(  -2,    3),
        p(  -2,    1),    p(  -8,    3),    p(  -2,    3),    p(  12,   -1),    p(   8,    0),    p(  10,   -6),    p(  -3,   -1),    p(  23,   -9),
        p(  -3,    2),    p(   4,    0),    p(   2,    3),    p(  -0,    3),    p(   8,    1),    p(   6,   -4),    p(  14,  -10),    p(  13,   -2),
        p(   9,   -1),    p(   0,   -2),    p(  10,   -3),    p(   3,    5),    p(   5,    3),    p(  12,    1),    p(  18,   -6),    p(  12,   -2),
        p(   5,   -2),    p(  13,   -0),    p(   6,    5),    p(  -0,    4),    p(  13,    4),    p(  -6,    8),    p(   5,    5),    p(  10,   -6),
    ],
    // rook
    [
        p( -38,   47),    p( -42,   50),    p( -51,   55),    p( -53,   54),    p( -43,   49),    p( -26,   51),    p( -34,   52),    p(  -1,   34),
        p( -32,   45),    p( -32,   48),    p( -25,   48),    p( -10,   40),    p( -22,   42),    p(   4,   38),    p(   6,   38),    p(  14,   27),
        p( -32,   41),    p( -14,   34),    p( -20,   37),    p( -21,   30),    p(   3,   23),    p(  20,   19),    p(  26,   23),    p(   1,   24),
        p( -34,   40),    p( -25,   36),    p( -25,   37),    p( -22,   32),    p( -16,   28),    p(  -0,   25),    p(  -5,   30),    p( -10,   26),
        p( -44,   37),    p( -41,   35),    p( -40,   36),    p( -33,   31),    p( -26,   29),    p( -28,   29),    p( -12,   25),    p( -28,   25),
        p( -48,   32),    p( -42,   28),    p( -42,   29),    p( -39,   29),    p( -30,   22),    p( -20,   16),    p(  -5,   11),    p( -20,   16),
        p( -47,   30),    p( -42,   28),    p( -35,   27),    p( -31,   23),    p( -23,   19),    p( -10,   12),    p(  -0,    8),    p( -36,   22),
        p( -40,   34),    p( -41,   28),    p( -42,   32),    p( -37,   27),    p( -30,   20),    p( -29,   24),    p( -42,   32),    p( -44,   26),
    ],
    // queen
    [
        p( -11,   62),    p(  -9,   66),    p(  11,   68),    p(  31,   61),    p(  37,   63),    p(  55,   58),    p(  70,   28),    p(  24,   47),
        p(  23,   45),    p(  13,   53),    p(  14,   66),    p(  18,   71),    p(  27,   71),    p(  49,   66),    p(  38,   59),    p(  72,   46),
        p(  35,   42),    p(  31,   43),    p(  30,   60),    p(  29,   64),    p(  35,   67),    p(  67,   66),    p(  70,   56),    p(  62,   57),
        p(  23,   49),    p(  27,   52),    p(  26,   50),    p(  21,   63),    p(  27,   66),    p(  43,   61),    p(  46,   68),    p(  50,   56),
        p(  19,   50),    p(  18,   50),    p(  19,   48),    p(  21,   57),    p(  27,   59),    p(  30,   57),    p(  37,   58),    p(  40,   52),
        p(  19,   35),    p(  24,   38),    p(  20,   49),    p(  18,   52),    p(  22,   59),    p(  33,   45),    p(  40,   40),    p(  37,   31),
        p(  20,   32),    p(  18,   41),    p(  23,   42),    p(  23,   54),    p(  24,   55),    p(  28,   33),    p(  36,   20),    p(  25,   23),
        p(   4,   45),    p(  14,   34),    p(  14,   41),    p(  18,   47),    p(  20,   36),    p(   8,   43),    p(   2,   27),    p(  12,    9),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  58,   11),    p(  65,    8),    p(  72,   -1),    p(  71,  -41),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  55,   17),    p(  27,   27),    p(  31,   19),    p(   9,   12),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  19,   26),    p(  33,   20),    p(  27,   14),    p( -15,   16),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -45,   23),    p( -29,   17),    p( -29,    6),    p( -49,   11),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -62,   19),    p( -46,    7),    p( -46,   -3),    p( -62,    2),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -49,    9),    p( -47,    2),    p( -19,  -10),    p( -37,    3),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -40,    1),    p( -27,   -5),    p(   2,  -19),    p(  17,   -5),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  12,   -6),    p(  -1,   -2),    p(  23,  -15),    p(  30,  -17),
    ],
];

const MORE_MINORS_NO_PAWNS: PhasedScore = p(-67, -66);
const OPPOSITE_COLORED_BISHOPS: PhasedScore = p(13, -34);
const BISHOP_PAIR: PhasedScore = p(25, 47);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(16, 16), p(17, 12), p(15, 3), p(11, -4), p(6, -10), p(2, -17), p(-5, -23), p(-11, -35), p(-22, -37)];
const ROOK_OPEN_FILE: PhasedScore = p(12, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -0);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(4, -0);
const KING_OPEN_FILE: PhasedScore = p(-36, 5);
const KING_CLOSED_FILE: PhasedScore = p(13, -6);
const KING_SEMIOPEN_FILE: PhasedScore = p(-3, 10);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-4, 2), p(2, 4), p(3, 1), p(2, 1), p(5, 2), p(6, 3), p(12, -1), p(22, -4)],
    // Closed
    [p(0, 0), p(0, 0), p(16, -17), p(-17, 7), p(2, 7), p(2, 1), p(3, 3), p(3, -1)],
    // SemiOpen
    [p(0, 0), p(-2, 20), p(10, 13), p(-0, 8), p(2, 7), p(6, 3), p(6, -1), p(13, 1)],
    // SemiClosed
    [p(0, 0), p(13, -16), p(12, 1), p(2, -2), p(7, -1), p(2, 1), p(7, 1), p(5, -1)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(18, 9),
    p(1, 3),
    p(0, 3),
    p(-7, 4),
    p(5, 2),
    p(-8, -12),
    p(-3, -2),
    p(-4, -12),
    p(-2, -1),
    p(-13, -2),
    p(-7, -17),
    p(-14, -9),
    p(6, -7),
    p(0, -12),
    p(8, -8),
    p(7, 12),
    p(-7, -2),
    p(-21, -6),
    p(-12, -0),
    p(-35, 19),
    p(-17, 2),
    p(-15, -19),
    p(6, 24),
    p(-37, 25),
    p(-19, -16),
    p(-21, -16),
    p(-32, -30),
    p(-35, 10),
    p(-13, -2),
    p(17, -7),
    p(-63, 67),
    p(0, 0),
    p(-1, -5),
    p(-13, -7),
    p(1, -8),
    p(-17, -4),
    p(-19, -2),
    p(-41, -21),
    p(-21, 32),
    p(-26, 15),
    p(-6, -7),
    p(-18, -11),
    p(11, -14),
    p(-12, 29),
    p(-44, 17),
    p(4, -30),
    p(0, 0),
    p(0, 0),
    p(-1, -11),
    p(-10, 5),
    p(1, -52),
    p(0, 0),
    p(15, -13),
    p(-28, -14),
    p(0, 0),
    p(0, 0),
    p(-24, -6),
    p(-16, -12),
    p(-8, 14),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(23, 8),
    p(4, 0),
    p(-5, 4),
    p(-19, -2),
    p(10, -4),
    p(-22, -10),
    p(-11, -4),
    p(-28, -15),
    p(7, -0),
    p(-8, -8),
    p(-27, -7),
    p(-39, -3),
    p(1, -6),
    p(-33, -10),
    p(-27, -10),
    p(-40, 48),
    p(8, 2),
    p(-1, -7),
    p(-4, -9),
    p(-20, -2),
    p(-8, -2),
    p(-12, -16),
    p(-19, -1),
    p(-20, 65),
    p(-5, -9),
    p(-23, -15),
    p(-30, -29),
    p(6, -67),
    p(-9, -10),
    p(-6, -22),
    p(-66, 53),
    p(0, 0),
    p(10, 2),
    p(-3, -3),
    p(-16, -7),
    p(-23, -8),
    p(-0, -0),
    p(-23, -19),
    p(-10, -5),
    p(-21, -4),
    p(-1, -6),
    p(-19, -9),
    p(-24, -16),
    p(-32, -7),
    p(-6, -1),
    p(-42, -7),
    p(10, 10),
    p(-47, 46),
    p(3, 1),
    p(-9, -4),
    p(-24, 50),
    p(0, 0),
    p(-9, -7),
    p(-14, -0),
    p(0, 0),
    p(0, 0),
    p(-11, -1),
    p(-32, 4),
    p(-27, -44),
    p(0, 0),
    p(21, -59),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-15, 8),   /*0b0000*/
    p(-12, 6),   /*0b0001*/
    p(-4, 9),    /*0b0010*/
    p(-0, 8),    /*0b0011*/
    p(-8, 2),    /*0b0100*/
    p(-21, -2),  /*0b0101*/
    p(-7, 2),    /*0b0110*/
    p(-7, -13),  /*0b0111*/
    p(-1, 4),    /*0b1000*/
    p(-8, 7),    /*0b1001*/
    p(-1, 8),    /*0b1010*/
    p(7, 8),     /*0b1011*/
    p(-4, 1),    /*0b1100*/
    p(-15, 1),   /*0b1101*/
    p(-7, 2),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(1, 7),     /*0b10000*/
    p(5, 4),     /*0b10001*/
    p(20, 5),    /*0b10010*/
    p(1, 6),     /*0b10011*/
    p(-7, 1),    /*0b10100*/
    p(15, 9),    /*0b10101*/
    p(-27, -0),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(6, 8),     /*0b11000*/
    p(27, 8),    /*0b11001*/
    p(28, 23),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(9, -6),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(6, 1),     /*0b100000*/
    p(1, 4),     /*0b100001*/
    p(14, 1),    /*0b100010*/
    p(8, -3),    /*0b100011*/
    p(-5, -5),   /*0b100100*/
    p(-18, -13), /*0b100101*/
    p(-18, 16),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(13, -5),   /*0b101000*/
    p(0, 5),     /*0b101001*/
    p(10, -7),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-0, -3),   /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(2, -0),    /*0b110000*/
    p(15, 0),    /*0b110001*/
    p(17, -8),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(11, 11),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(16, -9),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(7, -6),    /*0b111111*/
    p(-9, 7),    /*0b00*/
    p(-0, -3),   /*0b01*/
    p(16, -2),   /*0b10*/
    p(-7, -23),  /*0b11*/
    p(18, 2),    /*0b100*/
    p(-9, -1),   /*0b101*/
    p(28, -23),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(33, -5),   /*0b1000*/
    p(-9, -13),  /*0b1001*/
    p(9, -30),   /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(-4, -10),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-36, 27),  /*0b1111*/
    p(-3, 9),    /*0b00*/
    p(10, -4),   /*0b01*/
    p(7, -7),    /*0b10*/
    p(7, -30),   /*0b11*/
    p(8, -5),    /*0b100*/
    p(22, -13),  /*0b101*/
    p(-3, -13),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(11, 2),    /*0b1000*/
    p(20, -9),   /*0b1001*/
    p(22, -36),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(5, -19),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-7, -36),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(1, -34);
const STOPPABLE_PASSER: PhasedScore = p(15, -39);
const CLOSE_KING_PASSER: PhasedScore = p(-2, 25);
const IMMOBILE_PASSER: PhasedScore = p(-8, -32);
const PROTECTED_PASSER: PhasedScore = p(7, 0);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -26,   35),    p( -27,   51),    p( -38,   44),    p( -40,   26),    p( -39,   27),    p( -29,   24),    p( -15,   27),    p( -40,   29),
        p( -19,   32),    p( -33,   52),    p( -35,   42),    p( -35,   28),    p( -48,   35),    p( -43,   32),    p( -33,   39),    p( -33,   27),
        p( -12,   44),    p( -21,   43),    p( -32,   42),    p( -27,   41),    p( -37,   45),    p( -33,   45),    p( -36,   52),    p( -55,   53),
        p(  -0,   53),    p(  -3,   51),    p(  -1,   42),    p( -12,   50),    p( -27,   57),    p( -19,   60),    p( -19,   63),    p( -40,   65),
        p(   7,   54),    p(  20,   48),    p(   7,   42),    p(  -6,   32),    p(  -1,   42),    p(  -8,   55),    p( -22,   56),    p( -43,   61),
        p(  19,   51),    p(  21,   50),    p(  24,   46),    p(  27,   41),    p(  25,   45),    p(  30,   55),    p(   1,   59),    p(   5,   60),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(0, 4), p(4, 8), p(7, 17), p(11, 27), p(18, 54), p(22, 54)];
const UNSUPPORTED_PAWN: PhasedScore = p(-9, -4);
const DOUBLED_PAWN: PhasedScore = p(-7, -23);
const PHALANX: [PhasedScore; 6] = [p(-3, 1), p(3, 4), p(8, 6), p(21, 19), p(58, 59), p(67, 64)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(15, 14), p(7, 16), p(14, 19), p(6, 10), p(-2, 11), p(-48, 12)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(48, 20), p(50, 42), p(58, 9), p(50, -6), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, -4), p(14, 20), p(18, -6), p(16, 10), p(17, -12), p(30, -12)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-41, -59),
        p(-19, -33),
        p(-7, -11),
        p(3, 1),
        p(10, 11),
        p(18, 20),
        p(26, 22),
        p(32, 24),
        p(36, 23),
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
        p(-31, -48),
        p(-17, -33),
        p(-6, -18),
        p(1, -6),
        p(9, 3),
        p(14, 11),
        p(19, 16),
        p(24, 19),
        p(26, 24),
        p(33, 25),
        p(38, 25),
        p(44, 28),
        p(38, 38),
        p(50, 29),
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
        p(-49, -19),
        p(-41, -4),
        p(-38, 3),
        p(-34, 8),
        p(-35, 16),
        p(-31, 22),
        p(-29, 28),
        p(-26, 32),
        p(-23, 38),
        p(-20, 42),
        p(-17, 45),
        p(-16, 50),
        p(-9, 51),
        p(-2, 50),
        p(0, 48),
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
        p(-0, -66),
        p(2, -65),
        p(-2, -35),
        p(2, -19),
        p(4, 0),
        p(8, 7),
        p(11, 19),
        p(14, 27),
        p(18, 34),
        p(21, 36),
        p(23, 41),
        p(27, 45),
        p(30, 46),
        p(32, 52),
        p(34, 55),
        p(39, 58),
        p(42, 63),
        p(47, 63),
        p(56, 63),
        p(64, 63),
        p(67, 65),
        p(68, 70),
        p(69, 71),
        p(67, 70),
        p(70, 75),
        p(66, 73),
        p(67, 75),
        p(64, 67),
    ],
    [
        p(-19, -19),
        p(-13, -23),
        p(-8, -17),
        p(-1, -8),
        p(7, -2),
        p(8, 5),
        p(17, 14),
        p(21, 21),
        p(44, 17),
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
const THREATS: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(-4, 13), p(0, 0), p(28, 25), p(56, 9), p(39, -3), p(0, 0)],
    [p(-2, 11), p(21, 24), p(0, 0), p(42, 21), p(50, 64), p(0, 0)],
    [p(-6, 20), p(9, 19), p(16, 16), p(0, 0), p(58, 51), p(0, 0)],
    [p(-4, 17), p(-1, 18), p(-1, 31), p(-4, 24), p(0, 0), p(0, 0)],
    [p(51, 21), p(-22, 23), p(5, 14), p(-14, 23), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 7), p(8, 6), p(6, 10), p(12, 7), p(8, 16), p(3, 8)],
    [p(3, 8), p(11, 20), p(-52, -30), p(9, 12), p(10, 18), p(5, 5)],
    [p(-3, 8), p(8, 12), p(4, 16), p(7, 13), p(4, 33), p(15, -1)],
    [p(-0, 11), p(6, 13), p(4, 8), p(3, 18), p(-71, -68), p(2, -1)],
    [p(24, 5), p(13, 17), p(18, 10), p(3, 10), p(16, -4), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(7, -19), p(18, -9), p(9, -2), p(15, -16), p(-5, 10), p(8, 0)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(9, 2), p(-5, 6), p(14, -6), p(-12, 33)];
const CHECK_STM: PhasedScore = p(38, 21);
const DISCOVERED_CHECK_STM: PhasedScore = p(64, 66);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(11, -6), p(63, 26), p(66, 10), p(66, 67), p(0, 0), p(27, -24)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(5, -16), p(26, 28), p(17, 30), p(41, 10), p(54, 12)];
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

    fn candidate_passer(rank: DimT) -> SingleFeatureScore<Self::Score>;

    fn unsupported_pawn() -> SingleFeatureScore<Self::Score>;

    fn doubled_pawn() -> SingleFeatureScore<Self::Score>;

    fn phalanx(rank: DimT) -> SingleFeatureScore<Self::Score>;

    fn bishop_pair() -> SingleFeatureScore<Self::Score>;

    fn bad_bishop(num_pawns: usize) -> SingleFeatureScore<Self::Score>;

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

    fn threats(attacking: PieceType, targeted: PieceType) -> SingleFeatureScore<Self::Score>;

    fn defended(protecting: PieceType, target: PieceType) -> SingleFeatureScore<Self::Score>;

    fn king_zone_attack(attacking: PieceType) -> SingleFeatureScore<Self::Score>;

    fn can_give_check(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn check_stm() -> SingleFeatureScore<Self::Score>;

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

    fn psqt(&self, square: Square, piece: PieceType, color: Color) -> Self::Score {
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

    fn close_king_passer() -> SingleFeatureScore<Self::Score> {
        CLOSE_KING_PASSER
    }

    fn immobile_passer() -> SingleFeatureScore<Self::Score> {
        IMMOBILE_PASSER
    }

    fn passer_protection() -> SingleFeatureScore<Self::Score> {
        PROTECTED_PASSER
    }

    fn candidate_passer(rank: DimT) -> SingleFeatureScore<Self::Score> {
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

    fn threats(attacking: PieceType, targeted: PieceType) -> PhasedScore {
        THREATS[attacking as usize - 1][targeted as usize]
    }
    fn defended(protecting: PieceType, target: PieceType) -> PhasedScore {
        DEFENDED[protecting as usize - 1][target as usize]
    }

    fn king_zone_attack(attacking: PieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }

    fn can_give_check(piece: PieceType) -> PhasedScore {
        CAN_GIVE_CHECK[piece as usize]
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
}
