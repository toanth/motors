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
        p(  14,   60),    p(  14,   60),    p(  20,   54),    p(  23,   48),    p(  21,   55),    p(  22,   69),    p( -14,   75),    p(  -6,   75),
        p( -31,   19),    p( -29,   20),    p( -15,   11),    p( -15,    8),    p( -19,    3),    p(  29,    5),    p(  15,   20),    p(  18,   13),
        p( -49,    2),    p( -39,   -2),    p( -41,   -8),    p( -14,   -5),    p( -10,   -7),    p( -13,  -15),    p( -12,   -6),    p( -10,   -8),
        p( -55,  -11),    p( -50,   -8),    p( -29,   -9),    p( -13,   -9),    p(  -8,  -10),    p( -10,  -11),    p( -20,  -12),    p( -24,  -17),
        p( -65,  -11),    p( -54,  -15),    p( -33,   -9),    p( -21,   -7),    p( -21,   -7),    p( -19,   -8),    p( -16,  -23),    p( -28,  -20),
        p( -54,   -4),    p( -46,   -8),    p( -40,   -7),    p( -42,   -2),    p( -36,    1),    p( -10,  -10),    p(   7,  -22),    p( -18,  -20),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
    ],
    // knight
    [
        p(-113,  -18),    p(-111,   23),    p( -91,   22),    p( -58,   19),    p( -14,    6),    p( -92,   13),    p( -69,   -4),    p(-103,  -43),
        p( -25,   15),    p( -14,   21),    p(  -3,   10),    p(   0,   13),    p(   5,    7),    p(  13,   -0),    p(  -6,   12),    p( -20,    2),
        p( -13,   12),    p(   4,    7),    p(   9,   12),    p(  20,   13),    p(  31,   10),    p(  51,   -3),    p(  -5,    6),    p(  -4,    6),
        p(   7,   18),    p(  15,   13),    p(  28,   15),    p(  28,   23),    p(  28,   20),    p(  30,   17),    p(  19,   18),    p(  29,   10),
        p(   0,   18),    p(  13,    9),    p(  15,   15),    p(  23,   18),    p(  21,   20),    p(  29,    7),    p(  27,    6),    p(  17,   16),
        p( -22,    5),    p( -14,    3),    p(  -5,   -1),    p(   4,   11),    p(   8,   10),    p(   6,  -11),    p(   7,   -3),    p(   0,    6),
        p( -29,   12),    p( -15,   15),    p( -13,    8),    p(  -4,   10),    p(   4,    5),    p(  -2,    1),    p(   2,   10),    p(  -4,   19),
        p( -58,    8),    p( -20,    7),    p( -30,    7),    p(  -9,   12),    p(  -3,   12),    p(  -3,    2),    p( -13,    9),    p( -29,   11),
    ],
    // bishop
    [
        p( -24,   19),    p( -38,   12),    p( -70,    9),    p( -74,   17),    p( -86,   15),    p( -65,   10),    p( -39,   12),    p( -40,    8),
        p( -10,    6),    p( -12,    6),    p(  -4,    6),    p( -19,    6),    p( -11,    2),    p(  -3,    2),    p( -42,   15),    p( -20,    5),
        p(   0,    9),    p(   6,    6),    p(  -8,    9),    p(   6,   -1),    p(   5,    5),    p(  35,    3),    p(  19,    6),    p(  15,   15),
        p( -11,   10),    p(   0,    6),    p(  10,    1),    p(  10,    8),    p(  10,    5),    p(  11,    4),    p(  14,    2),    p(  -8,    7),
        p( -10,    6),    p( -11,    7),    p(  -4,    6),    p(  11,    3),    p(   7,    3),    p(   8,   -2),    p(  -6,    3),    p(  16,   -5),
        p(  -5,    6),    p(  -1,    5),    p(  -1,    7),    p(  -1,    6),    p(   7,    5),    p(   3,   -1),    p(   9,   -6),    p(  12,    1),
        p(   9,    3),    p(  -1,    2),    p(   6,    2),    p(  -0,    9),    p(   1,    7),    p(   8,    5),    p(  16,   -4),    p(  12,    2),
        p(  -2,    4),    p(  10,    5),    p(   5,    9),    p(  -7,    9),    p(   6,    9),    p(  -8,   12),    p(   5,    7),    p(   6,   -2),
    ],
    // rook
    [
        p( -52,   61),    p( -56,   65),    p( -68,   71),    p( -72,   71),    p( -58,   65),    p( -38,   65),    p( -48,   67),    p(  -9,   45),
        p( -47,   61),    p( -49,   65),    p( -43,   65),    p( -27,   57),    p( -40,   59),    p( -12,   54),    p(  -9,   53),    p(   3,   41),
        p( -47,   56),    p( -30,   49),    p( -36,   52),    p( -38,   46),    p( -13,   39),    p(   5,   34),    p(  13,   38),    p( -12,   38),
        p( -50,   56),    p( -41,   51),    p( -42,   52),    p( -39,   48),    p( -32,   44),    p( -16,   40),    p( -21,   46),    p( -25,   40),
        p( -61,   53),    p( -59,   51),    p( -58,   52),    p( -51,   48),    p( -43,   46),    p( -45,   46),    p( -30,   42),    p( -45,   41),
        p( -65,   48),    p( -60,   44),    p( -61,   46),    p( -59,   46),    p( -49,   40),    p( -39,   34),    p( -25,   28),    p( -38,   32),
        p( -64,   46),    p( -60,   44),    p( -53,   44),    p( -50,   41),    p( -43,   37),    p( -29,   30),    p( -19,   24),    p( -53,   36),
        p( -58,   50),    p( -57,   43),    p( -58,   48),    p( -54,   42),    p( -47,   36),    p( -47,   40),    p( -58,   47),    p( -61,   41),
    ],
    // queen
    [
        p( -30,   88),    p( -29,   93),    p( -15,  103),    p(   6,   94),    p(  11,   99),    p(  37,   87),    p(  71,   41),    p(   8,   70),
        p(   2,   74),    p( -11,   90),    p( -12,  107),    p( -20,  129),    p( -20,  144),    p(  20,  117),    p(  14,   98),    p(  58,   69),
        p(  14,   69),    p(   8,   78),    p(   4,  101),    p(  -0,  114),    p(   2,  125),    p(  43,  115),    p(  57,   86),    p(  41,   88),
        p(   2,   77),    p(   4,   86),    p(   1,   91),    p(  -8,  114),    p(  -5,  122),    p(  14,  113),    p(  18,  116),    p(  27,   92),
        p(  -3,   77),    p(  -6,   83),    p(  -6,   89),    p(  -5,  103),    p(  -0,  106),    p(   3,  104),    p(  12,  100),    p(  16,   87),
        p(  -3,   62),    p(   1,   72),    p(  -5,   86),    p(  -6,   91),    p(  -3,  101),    p(   7,   90),    p(  14,   83),    p(  14,   62),
        p(  -1,   55),    p(  -4,   70),    p(   0,   74),    p(  -0,   87),    p(   1,   90),    p(   4,   71),    p(  13,   55),    p(   6,   47),
        p( -21,   70),    p(  -7,   58),    p(  -7,   66),    p(  -2,   71),    p(  -1,   61),    p( -15,   73),    p( -18,   52),    p(  -7,   26),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  70,    4),    p(  82,   -0),    p( 102,  -12),    p( 191,  -69),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  10,   24),    p( -31,   36),    p( -21,   26),    p(  -4,   14),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -43,   35),    p( -23,   28),    p( -30,   21),    p( -34,   20),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-113,   33),    p( -89,   25),    p( -85,   12),    p( -71,   14),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-130,   27),    p(-104,   14),    p(-104,    3),    p( -94,    7),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-107,   15),    p(-106,    8),    p( -78,   -5),    p( -57,    5),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -96,    6),    p( -83,   -1),    p( -55,  -15),    p(   2,   -4),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  22,  -15),    p(   7,  -10),    p(  30,  -23),    p(  57,  -25),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 51);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(13, 21), p(13, 18), p(13, 8), p(9, 1), p(4, -6), p(0, -15), p(-6, -23), p(-12, -36), p(-23, -40)];
const ROOK_OPEN_FILE: PhasedScore = p(14, 1);
const ROOK_CLOSED_FILE: PhasedScore = p(-11, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(6, -3);
const KING_OPEN_FILE: PhasedScore = p(-37, 6);
const KING_CLOSED_FILE: PhasedScore = p(12, -6);
const KING_SEMIOPEN_FILE: PhasedScore = p(-3, 11);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-3, 6), p(0, 9), p(1, 7), p(4, 6), p(4, 7), p(4, 8), p(9, 5), p(20, 1)],
    // Closed
    [p(0, 0), p(0, 0), p(18, -16), p(-14, 13), p(2, 12), p(1, 6), p(1, 8), p(1, 4)],
    // SemiOpen
    [p(0, 0), p(-6, 28), p(9, 20), p(2, 13), p(1, 12), p(4, 7), p(4, 4), p(11, 6)],
    // SemiClosed
    [p(0, 0), p(11, -12), p(10, 7), p(4, 2), p(8, 4), p(2, 5), p(5, 5), p(4, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(23, 8),
    p(4, 3),
    p(2, 4),
    p(-6, 6),
    p(7, 3),
    p(-7, -9),
    p(-3, 1),
    p(-4, -8),
    p(1, -2),
    p(-10, -2),
    p(-7, -15),
    p(-15, -5),
    p(6, -5),
    p(-1, -8),
    p(7, -4),
    p(6, 16),
    p(-6, -2),
    p(-22, -4),
    p(-13, 2),
    p(-38, 22),
    p(-20, 5),
    p(-19, -16),
    p(3, 28),
    p(-45, 30),
    p(-18, -16),
    p(-22, -15),
    p(-34, -27),
    p(-40, 15),
    p(-16, 2),
    p(13, -2),
    p(-84, 100),
    p(0, 0),
    p(1, -5),
    p(-13, -6),
    p(-2, -6),
    p(-21, 0),
    p(-20, -1),
    p(-44, -18),
    p(-26, 39),
    p(-33, 30),
    p(-6, -5),
    p(-20, -9),
    p(7, -9),
    p(-16, 35),
    p(-48, 20),
    p(-1, -27),
    p(0, 0),
    p(0, 0),
    p(-4, -11),
    p(-16, 8),
    p(-6, -52),
    p(0, 0),
    p(7, -9),
    p(-37, -12),
    p(0, 0),
    p(0, 0),
    p(-29, -4),
    p(-23, -10),
    p(-18, 20),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(25, 5),
    p(4, -1),
    p(-5, 4),
    p(-21, -0),
    p(11, -4),
    p(-23, -8),
    p(-15, -1),
    p(-33, -10),
    p(8, -2),
    p(-9, -8),
    p(-28, -5),
    p(-43, 1),
    p(-1, -5),
    p(-38, -6),
    p(-31, -7),
    p(-47, 60),
    p(10, 1),
    p(-1, -7),
    p(-4, -8),
    p(-21, -1),
    p(-9, -1),
    p(-14, -11),
    p(-22, 1),
    p(-66, 179),
    p(-4, -9),
    p(-24, -14),
    p(-32, -27),
    p(11, -80),
    p(-12, -8),
    p(-10, -18),
    p(-77, 70),
    p(0, 0),
    p(12, 1),
    p(-2, -3),
    p(-17, -5),
    p(-27, -5),
    p(0, 1),
    p(-25, -16),
    p(-15, -0),
    p(-27, 2),
    p(-1, -6),
    p(-21, -7),
    p(-27, -13),
    p(-36, -3),
    p(-6, -1),
    p(-46, -5),
    p(4, 16),
    p(-57, 61),
    p(4, 1),
    p(-11, -1),
    p(-28, 60),
    p(0, 0),
    p(-12, -2),
    p(-19, 7),
    p(0, 0),
    p(0, 0),
    p(-13, 2),
    p(-37, 10),
    p(-31, -39),
    p(0, 0),
    p(18, -61),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-13, 9),   /*0b0000*/
    p(-13, 6),   /*0b0001*/
    p(-5, 11),   /*0b0010*/
    p(-4, 10),   /*0b0011*/
    p(-8, 2),    /*0b0100*/
    p(-24, -1),  /*0b0101*/
    p(-11, 4),   /*0b0110*/
    p(-13, -12), /*0b0111*/
    p(0, 5),     /*0b1000*/
    p(-9, 9),    /*0b1001*/
    p(-2, 10),   /*0b1010*/
    p(2, 10),    /*0b1011*/
    p(-5, 3),    /*0b1100*/
    p(-19, 3),   /*0b1101*/
    p(-10, 3),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(1, 9),     /*0b10000*/
    p(3, 6),     /*0b10001*/
    p(19, 7),    /*0b10010*/
    p(-2, 6),    /*0b10011*/
    p(-9, 2),    /*0b10100*/
    p(12, 12),   /*0b10101*/
    p(-30, 2),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(7, 9),     /*0b11000*/
    p(24, 11),   /*0b11001*/
    p(26, 25),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(6, -3),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(6, 2),     /*0b100000*/
    p(-1, 7),    /*0b100001*/
    p(13, 3),    /*0b100010*/
    p(5, -2),    /*0b100011*/
    p(-6, -3),   /*0b100100*/
    p(-21, -13), /*0b100101*/
    p(-21, 17),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(15, -3),   /*0b101000*/
    p(-1, 7),    /*0b101001*/
    p(9, -6),    /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-1, -1),   /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(2, 2),     /*0b110000*/
    p(12, 3),    /*0b110001*/
    p(14, -5),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(9, 12),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(18, -6),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(4, -5),    /*0b111111*/
    p(12, -0),   /*0b00*/
    p(20, -10),  /*0b01*/
    p(36, -10),  /*0b10*/
    p(11, -31),  /*0b11*/
    p(40, -5),   /*0b100*/
    p(12, -10),  /*0b101*/
    p(48, -30),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(55, -12),  /*0b1000*/
    p(10, -19),  /*0b1001*/
    p(28, -37),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(20, -19),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-19, 25),  /*0b1111*/
    p(20, 1),    /*0b00*/
    p(30, -12),  /*0b01*/
    p(27, -14),  /*0b10*/
    p(25, -38),  /*0b11*/
    p(29, -11),  /*0b100*/
    p(42, -20),  /*0b101*/
    p(18, -20),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(33, -4),   /*0b1000*/
    p(40, -16),  /*0b1001*/
    p(45, -45),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(27, -26),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(12, -43),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(-1, -37);
const STOPPABLE_PASSER: PhasedScore = p(25, -48);
const CLOSE_KING_PASSER: PhasedScore = p(3, 24);
const IMMOBILE_PASSER: PhasedScore = p(-5, -36);
const PROTECTED_PASSER: PhasedScore = p(8, -3);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -31,   46),    p( -39,   66),    p( -54,   60),    p( -58,   44),    p( -60,   46),    p( -47,   42),    p( -31,   43),    p( -57,   46),
        p( -24,   43),    p( -44,   67),    p( -50,   58),    p( -51,   44),    p( -68,   53),    p( -62,   50),    p( -49,   56),    p( -49,   43),
        p( -18,   55),    p( -27,   55),    p( -45,   59),    p( -43,   58),    p( -54,   63),    p( -49,   63),    p( -50,   71),    p( -70,   70),
        p(  -7,   70),    p( -11,   68),    p( -11,   60),    p( -29,   72),    p( -44,   80),    p( -36,   85),    p( -39,   91),    p( -61,   92),
        p(  -4,   62),    p(  13,   51),    p(  -1,   43),    p( -16,   38),    p( -15,   55),    p( -32,   70),    p( -51,   80),    p( -80,   91),
        p(  14,   60),    p(  14,   60),    p(  20,   54),    p(  23,   48),    p(  21,   55),    p(  22,   69),    p( -14,   75),    p(  -6,   75),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(1, 4), p(5, 8), p(8, 17), p(15, 22), p(17, 71), p(13, 64)];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -6);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);
const PHALANX: [PhasedScore; 6] = [p(-1, 0), p(5, 3), p(9, 6), p(21, 20), p(59, 73), p(-79, 214)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 14), p(7, 18), p(14, 21), p(7, 10), p(-3, 13), p(-48, 12)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(48, 21), p(51, 45), p(66, 4), p(51, -8), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(1, -5), p(14, 20), p(19, -7), p(16, 10), p(16, -9), p(30, -12)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-40, -67),
        p(-20, -27),
        p(-7, -4),
        p(2, 9),
        p(9, 20),
        p(16, 30),
        p(24, 33),
        p(30, 36),
        p(34, 34),
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
        p(-31, -56),
        p(-18, -37),
        p(-6, -21),
        p(1, -9),
        p(8, 1),
        p(13, 10),
        p(18, 15),
        p(23, 19),
        p(25, 25),
        p(32, 26),
        p(38, 26),
        p(43, 30),
        p(38, 41),
        p(51, 32),
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
        p(-67, 13),
        p(-58, 27),
        p(-54, 34),
        p(-50, 39),
        p(-51, 46),
        p(-46, 52),
        p(-43, 57),
        p(-40, 61),
        p(-37, 67),
        p(-33, 71),
        p(-29, 73),
        p(-28, 79),
        p(-20, 80),
        p(-12, 78),
        p(-8, 75),
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
        p(-17, -34),
        p(-17, -1),
        p(-22, 56),
        p(-18, 72),
        p(-16, 90),
        p(-12, 96),
        p(-8, 108),
        p(-5, 114),
        p(-1, 120),
        p(2, 122),
        p(5, 126),
        p(9, 129),
        p(12, 130),
        p(14, 135),
        p(17, 137),
        p(22, 140),
        p(23, 146),
        p(27, 147),
        p(38, 144),
        p(53, 137),
        p(58, 137),
        p(100, 115),
        p(99, 119),
        p(128, 96),
        p(215, 65),
        p(256, 28),
        p(287, 16),
        p(268, 12),
    ],
    [
        p(-85, -1),
        p(-57, -11),
        p(-30, -9),
        p(-3, -5),
        p(27, -3),
        p(49, -0),
        p(77, 6),
        p(101, 9),
        p(147, 0),
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
    [p(-4, 14), p(0, 0), p(28, 26), p(60, 7), p(39, 0), p(0, 0)],
    [p(-2, 12), p(21, 25), p(0, 0), p(42, 22), p(46, 92), p(0, 0)],
    [p(-4, 19), p(11, 18), p(19, 14), p(0, 0), p(66, 45), p(0, 0)],
    [p(-2, 9), p(2, 9), p(1, 25), p(-0, 13), p(0, 0), p(0, 0)],
    [p(63, 21), p(-20, 23), p(8, 17), p(-13, 23), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 9), p(8, 7), p(6, 11), p(12, 8), p(8, 14), p(3, 8)],
    [p(2, 9), p(11, 22), p(-1, -44), p(9, 12), p(10, 20), p(4, 7)],
    [p(2, 4), p(13, 9), p(9, 14), p(11, 10), p(9, 29), p(19, -3)],
    [p(2, 3), p(8, 4), p(7, -1), p(5, 13), p(-65, -219), p(5, -10)],
    [p(47, 1), p(34, 12), p(40, 7), p(25, 6), p(36, -9), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-19, -14), p(17, -9), p(9, -3), p(16, -16), p(-3, 4), p(-6, 3)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(8, 2), p(-5, 7), p(16, -8), p(-10, 24)];
const CHECK_STM: PhasedScore = p(38, 22);
const DISCOVERED_CHECK_STM: PhasedScore = p(175, 60);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(11, -7), p(62, 1), p(101, -33), p(56, 89), p(0, 0), p(-25, -23)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(5, -17), p(26, 30), p(16, 34), p(43, 9), p(62, 3)];
const MATERIAL: [PhasedScore; NUM_CHESS_PIECES + 1] =
    [p(100, 100), p(300, 300), p(300, 300), p(500, 500), p(900, 900), p(0, 0), p(0, 0)];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static + StaticallyNamedEntity {
    type Score: ScoreType;

    fn material(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn psqt(&self, square: Square, piece: PieceType, color: Color) -> SingleFeatureScore<Self::Score>;

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
