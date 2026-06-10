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
        p(  18,   52),    p(  19,   51),    p(  23,   46),    p(  25,   41),    p(  24,   46),    p(  28,   56),    p(  -1,   60),    p(   3,   60),
        p( -29,   20),    p( -22,   19),    p( -11,    9),    p( -13,   11),    p( -19,    9),    p(  26,    8),    p(  17,   29),    p(   9,   25),
        p( -43,   -1),    p( -34,   -6),    p( -35,  -13),    p( -13,   -8),    p(  -9,  -10),    p(  -8,  -20),    p(  -7,   -9),    p(  -7,  -11),
        p( -49,  -14),    p( -45,  -12),    p( -25,  -12),    p( -11,   -9),    p(  -6,  -10),    p(  -7,  -13),    p( -15,  -16),    p( -19,  -21),
        p( -57,  -15),    p( -47,  -18),    p( -29,  -10),    p( -18,   -8),    p( -17,   -8),    p( -15,  -10),    p( -10,  -26),    p( -22,  -23),
        p( -46,   -8),    p( -40,  -10),    p( -35,  -11),    p( -36,   -7),    p( -31,   -4),    p(  -4,  -14),    p(  12,  -25),    p( -12,  -23),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
    ],
    // knight
    [
        p( -62,  -46),    p( -65,    3),    p( -66,    7),    p( -48,    8),    p(  -7,   -4),    p( -63,   -2),    p( -55,  -13),    p( -62,  -57),
        p( -19,    4),    p(  -7,   10),    p(   5,   -0),    p(   8,    3),    p(  12,   -3),    p(  21,  -11),    p(   1,    1),    p( -15,   -8),
        p(  -7,    2),    p(  12,   -3),    p(  16,    2),    p(  27,    3),    p(  38,    0),    p(  54,  -11),    p(   2,   -3),    p(   2,   -4),
        p(  14,    6),    p(  21,    3),    p(  35,    6),    p(  34,   13),    p(  34,   11),    p(  36,    8),    p(  26,    8),    p(  35,   -1),
        p(   8,    6),    p(  19,   -0),    p(  22,    6),    p(  31,    8),    p(  28,   10),    p(  36,   -3),    p(  34,   -4),    p(  25,    4),
        p( -14,   -6),    p(  -7,   -7),    p(   2,   -9),    p(  11,    2),    p(  16,    0),    p(  13,  -19),    p(  14,  -13),    p(   8,   -5),
        p( -21,   -0),    p(  -8,    3),    p(  -6,   -2),    p(   4,   -1),    p(  11,   -5),    p(   5,   -9),    p(   9,   -1),    p(   4,    7),
        p( -49,   -6),    p( -15,   -3),    p( -25,   -3),    p(  -5,    1),    p(   4,    0),    p(  -2,   -6),    p( -10,    0),    p( -21,   -2),
    ],
    // bishop
    [
        p( -21,   14),    p( -34,    7),    p( -59,    3),    p( -59,   11),    p( -64,    7),    p( -55,    4),    p( -35,    7),    p( -38,    4),
        p( -10,    1),    p( -10,    2),    p(  -0,    1),    p( -15,    1),    p(  -6,   -3),    p(  -1,   -2),    p( -40,   11),    p( -17,   -0),
        p(   3,    5),    p(  11,    1),    p(  -5,    4),    p(   7,   -6),    p(   6,    0),    p(  35,   -0),    p(  23,    2),    p(  19,   10),
        p(  -4,    5),    p(   5,    0),    p(  12,   -4),    p(  10,    4),    p(  11,    1),    p(  12,   -0),    p(  18,   -3),    p(   1,    0),
        p(  -1,    0),    p(  -7,    2),    p(  -2,    2),    p(  11,   -1),    p(   7,   -1),    p(  11,   -7),    p(   0,   -3),    p(  23,  -10),
        p(  -1,    1),    p(   5,   -1),    p(   2,    2),    p(   0,    2),    p(   9,    1),    p(   7,   -6),    p(  14,  -11),    p(  15,   -4),
        p(   9,   -1),    p(   1,   -3),    p(  11,   -4),    p(   3,    4),    p(   6,    1),    p(  11,    0),    p(  19,   -8),    p(  13,   -3),
        p(  -1,   -1),    p(   9,    1),    p(   4,    6),    p(  -3,    4),    p(  10,    4),    p( -11,    9),    p(   5,    2),    p(  11,   -8),
    ],
    // rook
    [
        p( -39,   44),    p( -42,   46),    p( -51,   52),    p( -53,   51),    p( -43,   46),    p( -28,   48),    p( -36,   49),    p(  -2,   31),
        p( -34,   43),    p( -36,   46),    p( -30,   46),    p( -16,   38),    p( -29,   41),    p(  -4,   37),    p(  -1,   36),    p(  12,   24),
        p( -34,   38),    p( -18,   32),    p( -24,   35),    p( -25,   29),    p(  -3,   22),    p(  12,   18),    p(  20,   22),    p(  -4,   23),
        p( -37,   38),    p( -29,   34),    p( -28,   34),    p( -27,   30),    p( -21,   26),    p(  -6,   23),    p( -11,   29),    p( -15,   25),
        p( -46,   34),    p( -44,   32),    p( -43,   33),    p( -37,   29),    p( -30,   27),    p( -34,   27),    p( -17,   24),    p( -33,   24),
        p( -49,   29),    p( -45,   25),    p( -46,   26),    p( -43,   26),    p( -34,   19),    p( -25,   14),    p( -12,   10),    p( -26,   15),
        p( -47,   27),    p( -44,   25),    p( -36,   24),    p( -32,   20),    p( -25,   16),    p( -17,   11),    p(  -4,    6),    p( -38,   19),
        p( -40,   33),    p( -43,   25),    p( -45,   29),    p( -41,   23),    p( -33,   18),    p( -33,   22),    p( -42,   28),    p( -42,   24),
    ],
    // queen
    [
        p(  -6,   57),    p(  -7,   62),    p(  13,   65),    p(  34,   58),    p(  39,   60),    p(  56,   55),    p(  70,   26),    p(  32,   39),
        p(  23,   42),    p(  12,   52),    p(  14,   63),    p(  16,   71),    p(  25,   71),    p(  47,   66),    p(  36,   59),    p(  72,   45),
        p(  36,   40),    p(  29,   42),    p(  28,   59),    p(  27,   64),    p(  33,   67),    p(  66,   65),    p(  70,   55),    p(  64,   56),
        p(  24,   47),    p(  27,   50),    p(  25,   48),    p(  18,   63),    p(  24,   66),    p(  41,   61),    p(  46,   67),    p(  51,   54),
        p(  19,   47),    p(  17,   47),    p(  18,   46),    p(  18,   57),    p(  25,   58),    p(  28,   56),    p(  37,   57),    p(  41,   50),
        p(  21,   30),    p(  23,   35),    p(  17,   48),    p(  16,   51),    p(  20,   57),    p(  31,   44),    p(  38,   40),    p(  38,   28),
        p(  23,   25),    p(  18,   37),    p(  22,   40),    p(  21,   53),    p(  24,   53),    p(  26,   31),    p(  34,   18),    p(  25,   21),
        p(  10,   39),    p(   9,   36),    p(  10,   42),    p(  13,   46),    p(  11,   40),    p(   1,   46),    p(  -4,   29),    p(  16,    3),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  59,   10),    p(  67,    7),    p(  73,   -2),    p(  72,  -41),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  55,   16),    p(  28,   26),    p(  32,   18),    p(  11,   11),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  18,   26),    p(  33,   19),    p(  28,   13),    p( -12,   16),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -46,   24),    p( -30,   17),    p( -29,    6),    p( -47,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -62,   19),    p( -47,    8),    p( -47,   -3),    p( -62,    2),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -50,    9),    p( -48,    2),    p( -21,  -10),    p( -38,    4),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -41,    2),    p( -28,   -5),    p(  -1,  -18),    p(  16,   -4),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  15,   -7),    p(   2,   -2),    p(  24,  -15),    p(  29,  -17),
    ],
];

const MORE_MINORS_NO_PAWNS: PhasedScore = p(-66, -65);
const BISHOP_PAIR: PhasedScore = p(25, 47);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(16, 16), p(17, 12), p(16, 2), p(11, -4), p(7, -11), p(3, -19), p(-4, -27), p(-10, -39), p(-21, -44)];
const ROOK_OPEN_FILE: PhasedScore = p(8, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -0);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(2, -1);
const KING_OPEN_FILE: PhasedScore = p(-35, 5);
const KING_CLOSED_FILE: PhasedScore = p(12, -6);
const KING_SEMIOPEN_FILE: PhasedScore = p(-4, 11);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-5, 1), p(2, 2), p(3, -0), p(3, -0), p(5, 1), p(5, 3), p(11, -1), p(21, -5)],
    // Closed
    [p(0, 0), p(0, 0), p(15, -16), p(-16, 7), p(3, 6), p(2, 1), p(5, 2), p(4, -1)],
    // SemiOpen
    [p(0, 0), p(-3, 20), p(10, 13), p(1, 7), p(3, 6), p(6, 2), p(7, -2), p(13, -0)],
    // SemiClosed
    [p(0, 0), p(11, -15), p(12, 1), p(3, -4), p(9, -3), p(3, -0), p(8, -1), p(6, -2)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(18, 10),
    p(1, 3),
    p(0, 4),
    p(-7, 3),
    p(5, 2),
    p(-8, -12),
    p(-3, -3),
    p(-4, -13),
    p(-2, -1),
    p(-13, -4),
    p(-7, -18),
    p(-14, -10),
    p(7, -8),
    p(0, -14),
    p(7, -9),
    p(6, 11),
    p(-7, -2),
    p(-21, -6),
    p(-12, -1),
    p(-34, 18),
    p(-17, 2),
    p(-15, -21),
    p(6, 22),
    p(-35, 21),
    p(-18, -17),
    p(-21, -18),
    p(-31, -31),
    p(-36, 8),
    p(-13, -3),
    p(17, -9),
    p(-61, 67),
    p(0, 0),
    p(-1, -5),
    p(-13, -8),
    p(1, -9),
    p(-17, -5),
    p(-19, -3),
    p(-41, -22),
    p(-23, 32),
    p(-26, 15),
    p(-6, -7),
    p(-18, -13),
    p(11, -15),
    p(-11, 26),
    p(-43, 15),
    p(4, -32),
    p(0, 0),
    p(0, 0),
    p(-1, -12),
    p(-10, 3),
    p(-0, -53),
    p(0, 0),
    p(17, -15),
    p(-29, -19),
    p(0, 0),
    p(0, 0),
    p(-24, -8),
    p(-16, -16),
    p(-10, 16),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(22, 9),
    p(3, 1),
    p(-5, 4),
    p(-18, -3),
    p(10, -4),
    p(-21, -11),
    p(-11, -5),
    p(-28, -16),
    p(7, -0),
    p(-8, -10),
    p(-26, -9),
    p(-37, -6),
    p(1, -7),
    p(-33, -11),
    p(-26, -13),
    p(-38, 45),
    p(7, 2),
    p(-1, -8),
    p(-4, -10),
    p(-20, -3),
    p(-8, -2),
    p(-12, -17),
    p(-18, -5),
    p(-23, 65),
    p(-5, -10),
    p(-22, -17),
    p(-30, -29),
    p(8, -68),
    p(-9, -11),
    p(-5, -25),
    p(-65, 53),
    p(0, 0),
    p(9, 3),
    p(-3, -4),
    p(-16, -7),
    p(-23, -9),
    p(-0, -0),
    p(-22, -20),
    p(-9, -7),
    p(-20, -7),
    p(-1, -6),
    p(-19, -10),
    p(-23, -18),
    p(-31, -11),
    p(-4, -3),
    p(-42, -9),
    p(10, 10),
    p(-49, 48),
    p(3, 1),
    p(-8, -5),
    p(-23, 50),
    p(0, 0),
    p(-9, -7),
    p(-14, -1),
    p(0, 0),
    p(0, 0),
    p(-10, -1),
    p(-31, 2),
    p(-26, -46),
    p(0, 0),
    p(16, -60),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-15, 8),   /*0b0000*/
    p(-12, 5),   /*0b0001*/
    p(-4, 9),    /*0b0010*/
    p(0, 8),     /*0b0011*/
    p(-8, 1),    /*0b0100*/
    p(-20, -3),  /*0b0101*/
    p(-7, 2),    /*0b0110*/
    p(-6, -14),  /*0b0111*/
    p(-1, 4),    /*0b1000*/
    p(-8, 7),    /*0b1001*/
    p(-1, 8),    /*0b1010*/
    p(9, 7),     /*0b1011*/
    p(-4, 1),    /*0b1100*/
    p(-14, 0),   /*0b1101*/
    p(-6, 1),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(0, 7),     /*0b10000*/
    p(6, 4),     /*0b10001*/
    p(20, 5),    /*0b10010*/
    p(2, 5),     /*0b10011*/
    p(-7, 1),    /*0b10100*/
    p(15, 8),    /*0b10101*/
    p(-26, -0),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(5, 8),     /*0b11000*/
    p(27, 8),    /*0b11001*/
    p(28, 23),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(8, -6),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(5, 1),     /*0b100000*/
    p(1, 4),     /*0b100001*/
    p(14, 1),    /*0b100010*/
    p(9, -3),    /*0b100011*/
    p(-4, -5),   /*0b100100*/
    p(-18, -14), /*0b100101*/
    p(-17, 17),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(13, -5),   /*0b101000*/
    p(1, 5),     /*0b101001*/
    p(10, -7),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(0, -4),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(1, 0),     /*0b110000*/
    p(15, 0),    /*0b110001*/
    p(18, -8),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(12, 10),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(16, -8),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(8, -6),    /*0b111111*/
    p(-12, 8),   /*0b00*/
    p(-3, -2),   /*0b01*/
    p(15, -1),   /*0b10*/
    p(-9, -23),  /*0b11*/
    p(14, 3),    /*0b100*/
    p(-9, -2),   /*0b101*/
    p(26, -23),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(31, -5),   /*0b1000*/
    p(-12, -12), /*0b1001*/
    p(8, -30),   /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(-7, -9),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-35, 23),  /*0b1111*/
    p(-5, 9),    /*0b00*/
    p(10, -4),   /*0b01*/
    p(4, -7),    /*0b10*/
    p(5, -31),   /*0b11*/
    p(6, -5),    /*0b100*/
    p(21, -13),  /*0b101*/
    p(-5, -12),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(8, 3),     /*0b1000*/
    p(19, -9),   /*0b1001*/
    p(20, -35),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(2, -19),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-9, -36),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(1, -35);
const STOPPABLE_PASSER: PhasedScore = p(15, -38);
const CLOSE_KING_PASSER: PhasedScore = p(-1, 26);
const IMMOBILE_PASSER: PhasedScore = p(-9, -33);
const PROTECTED_PASSER: PhasedScore = p(6, -0);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -26,   35),    p( -28,   51),    p( -37,   44),    p( -39,   26),    p( -37,   26),    p( -28,   25),    p( -15,   27),    p( -42,   30),
        p( -19,   32),    p( -33,   52),    p( -35,   42),    p( -34,   28),    p( -46,   35),    p( -43,   33),    p( -33,   39),    p( -33,   27),
        p( -12,   44),    p( -20,   43),    p( -31,   43),    p( -27,   41),    p( -36,   45),    p( -32,   46),    p( -35,   53),    p( -55,   53),
        p(   1,   53),    p(  -2,   51),    p(   0,   42),    p( -12,   50),    p( -26,   57),    p( -18,   60),    p( -18,   63),    p( -39,   65),
        p(   7,   55),    p(  20,   48),    p(   7,   42),    p(  -5,   32),    p(  -1,   43),    p(  -9,   55),    p( -21,   56),    p( -43,   61),
        p(  18,   52),    p(  19,   51),    p(  23,   46),    p(  25,   41),    p(  24,   46),    p(  28,   56),    p(  -1,   60),    p(   3,   60),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(0, 4), p(3, 8), p(7, 17), p(11, 28), p(17, 55), p(20, 55)];
const UNSUPPORTED_PAWN: PhasedScore = p(-9, -4);
const DOUBLED_PAWN: PhasedScore = p(-7, -23);
const PHALANX: [PhasedScore; 6] = [p(-3, 1), p(4, 4), p(8, 6), p(21, 19), p(58, 59), p(67, 64)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(15, 14), p(7, 16), p(14, 19), p(6, 10), p(-2, 11), p(-47, 12)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(47, 20), p(49, 43), p(60, 10), p(53, -3), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(1, -4), p(14, 20), p(18, -7), p(15, 10), p(17, -12), p(31, -12)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-40, -60),
        p(-18, -35),
        p(-6, -13),
        p(4, -1),
        p(11, 8),
        p(19, 18),
        p(27, 20),
        p(33, 22),
        p(37, 21),
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
        p(-32, -49),
        p(-18, -33),
        p(-6, -19),
        p(1, -7),
        p(8, 2),
        p(14, 11),
        p(19, 15),
        p(24, 19),
        p(26, 24),
        p(33, 25),
        p(39, 25),
        p(45, 28),
        p(41, 38),
        p(51, 30),
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
        p(-52, -32),
        p(-43, -15),
        p(-39, -3),
        p(-36, 7),
        p(-35, 14),
        p(-30, 19),
        p(-27, 24),
        p(-23, 29),
        p(-19, 34),
        p(-15, 38),
        p(-11, 41),
        p(-7, 46),
        p(1, 48),
        p(8, 47),
        p(10, 45),
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
        p(5, -65),
        p(-4, -66),
        p(-5, -42),
        p(2, -34),
        p(2, -2),
        p(5, 5),
        p(9, 16),
        p(12, 26),
        p(15, 33),
        p(19, 35),
        p(22, 40),
        p(25, 45),
        p(28, 46),
        p(31, 51),
        p(33, 55),
        p(39, 57),
        p(41, 62),
        p(46, 63),
        p(53, 63),
        p(63, 62),
        p(66, 64),
        p(68, 69),
        p(69, 70),
        p(67, 69),
        p(69, 75),
        p(66, 74),
        p(67, 74),
        p(63, 65),
    ],
    [
        p(-19, -18),
        p(-12, -22),
        p(-7, -17),
        p(-1, -8),
        p(7, -2),
        p(8, 5),
        p(16, 14),
        p(20, 21),
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
    [p(-2, 11), p(22, 24), p(-23, -9), p(41, 21), p(48, 63), p(0, 0)],
    [p(-2, 18), p(12, 19), p(18, 16), p(-17, 15), p(56, 49), p(0, 0)],
    [p(-2, 15), p(0, 17), p(-2, 31), p(-5, 23), p(3, 50), p(0, 0)],
    [p(49, 22), p(-23, 23), p(2, 17), p(-9, 21), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 8), p(8, 6), p(6, 10), p(12, 7), p(8, 16), p(3, 8)],
    [p(2, 8), p(12, 20), p(-53, -47), p(8, 12), p(-0, -4), p(5, 6)],
    [p(0, 7), p(13, 12), p(10, 15), p(2, 4), p(-2, 6), p(10, 2)],
    [p(1, 10), p(7, 12), p(7, 4), p(2, 19), p(-69, -67), p(4, -0)],
    [p(23, 5), p(13, 17), p(18, 11), p(6, 10), p(16, -6), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(8, -20), p(18, -9), p(10, -3), p(16, -16), p(-6, 10), p(7, 0)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(10, 1), p(-4, 7), p(14, -6), p(-12, 33)];
const CHECK_STM: PhasedScore = p(36, 22);
const DISCOVERED_CHECK_STM: PhasedScore = p(64, 66);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(10, -5), p(63, 25), p(66, 10), p(66, 67), p(0, 0), p(31, -25)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(5, -16), p(26, 28), p(17, 31), p(44, 10), p(56, 13)];
const MATERIAL: [PhasedScore; NUM_CHESS_PIECES + 1] =
    [p(100, 100), p(300, 300), p(300, 300), p(500, 500), p(900, 900), p(0, 0), p(0, 0)];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static + StaticallyNamedEntity {
    type Score: ScoreType;

    fn material(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn psqt(&self, square: Square, piece: PieceType, color: Color) -> SingleFeatureScore<Self::Score>;

    fn more_minors_but_no_pawns() -> SingleFeatureScore<Self::Score>;

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
