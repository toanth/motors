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
        p(  19,   52),    p(  21,   51),    p(  25,   46),    p(  27,   41),    p(  26,   46),    p(  30,   56),    p(   1,   60),    p(   5,   60),
        p( -29,   21),    p( -22,   20),    p( -11,    9),    p( -13,   12),    p( -18,    9),    p(  26,    9),    p(  17,   29),    p(   8,   25),
        p( -42,   -1),    p( -33,   -6),    p( -35,  -13),    p( -13,   -8),    p(  -8,   -9),    p(  -8,  -19),    p(  -7,   -9),    p(  -8,  -10),
        p( -49,  -14),    p( -45,  -12),    p( -25,  -12),    p( -11,   -9),    p(  -6,  -10),    p(  -6,  -13),    p( -16,  -16),    p( -20,  -21),
        p( -57,  -15),    p( -47,  -18),    p( -29,  -10),    p( -18,   -8),    p( -18,   -8),    p( -15,  -10),    p( -11,  -26),    p( -23,  -23),
        p( -47,   -8),    p( -40,  -10),    p( -35,  -12),    p( -37,   -6),    p( -31,   -3),    p(  -5,  -14),    p(  11,  -25),    p( -13,  -23),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
    ],
    // knight
    [
        p( -62,  -46),    p( -65,    4),    p( -66,    8),    p( -48,    9),    p(  -8,   -3),    p( -64,   -2),    p( -54,  -13),    p( -62,  -57),
        p( -19,    5),    p(  -7,   11),    p(   5,    0),    p(   8,    3),    p(  12,   -3),    p(  22,  -11),    p(   1,    1),    p( -15,   -8),
        p(  -6,    2),    p(  12,   -2),    p(  16,    3),    p(  27,    3),    p(  38,    0),    p(  54,  -11),    p(   4,   -3),    p(   4,   -5),
        p(  14,    7),    p(  21,    3),    p(  34,    7),    p(  35,   13),    p(  34,   11),    p(  35,    8),    p(  26,    9),    p(  36,   -1),
        p(   8,    6),    p(  19,    0),    p(  22,    7),    p(  30,    9),    p(  28,   11),    p(  35,   -2),    p(  32,   -3),    p(  24,    5),
        p( -14,   -6),    p(  -7,   -7),    p(   1,   -9),    p(  10,    3),    p(  15,    1),    p(  12,  -19),    p(  13,  -13),    p(   8,   -5),
        p( -21,    0),    p(  -8,    3),    p(  -6,   -2),    p(   3,   -0),    p(  11,   -4),    p(   6,   -9),    p(   9,   -1),    p(   5,    7),
        p( -44,   -7),    p( -13,   -4),    p( -23,   -4),    p(  -2,    1),    p(   4,    1),    p(   4,   -9),    p(  -7,   -1),    p( -20,   -2),
    ],
    // bishop
    [
        p( -24,   14),    p( -36,    8),    p( -61,    4),    p( -58,   11),    p( -63,    8),    p( -56,    5),    p( -34,    7),    p( -39,    5),
        p( -13,    2),    p( -10,    3),    p(  -0,    1),    p( -14,    2),    p(  -7,   -3),    p(   0,   -3),    p( -36,   11),    p( -22,    0),
        p(   1,    6),    p(   9,    2),    p(  -4,    4),    p(   6,   -5),    p(   7,    1),    p(  37,    0),    p(  24,    2),    p(  16,   11),
        p(  -5,    5),    p(   3,    1),    p(  10,   -3),    p(  12,    4),    p(  11,    1),    p(  13,    0),    p(  17,   -2),    p(  -2,    1),
        p(  -4,    1),    p(  -8,    2),    p(  -2,    2),    p(  11,   -1),    p(   8,   -1),    p(  10,   -6),    p(  -3,   -2),    p(  22,   -9),
        p(  -3,    1),    p(   3,   -0),    p(   2,    2),    p(   0,    2),    p(   8,    1),    p(   6,   -5),    p(  13,  -11),    p(  13,   -3),
        p(  10,   -2),    p(   0,   -2),    p(  10,   -4),    p(   3,    4),    p(   5,    2),    p(  12,   -0),    p(  18,   -7),    p(  14,   -3),
        p(   5,   -3),    p(  13,   -1),    p(   6,    4),    p(  -2,    3),    p(  11,    3),    p(  -7,    7),    p(   6,    1),    p(  11,   -8),
    ],
    // rook
    [
        p( -36,   44),    p( -39,   46),    p( -48,   52),    p( -48,   50),    p( -38,   46),    p( -23,   47),    p( -35,   49),    p(  -2,   31),
        p( -31,   43),    p( -31,   46),    p( -23,   45),    p(  -7,   37),    p( -20,   39),    p(   7,   35),    p(   9,   35),    p(  15,   24),
        p( -31,   38),    p( -12,   31),    p( -18,   34),    p( -18,   27),    p(   5,   21),    p(  21,   16),    p(  26,   21),    p(   0,   22),
        p( -34,   38),    p( -24,   33),    p( -24,   34),    p( -20,   29),    p( -14,   25),    p(   1,   22),    p(  -4,   27),    p( -11,   24),
        p( -44,   35),    p( -40,   32),    p( -39,   34),    p( -32,   29),    p( -26,   27),    p( -28,   27),    p( -12,   24),    p( -29,   23),
        p( -47,   30),    p( -41,   25),    p( -41,   27),    p( -38,   27),    p( -29,   20),    p( -20,   15),    p(  -5,   10),    p( -20,   14),
        p( -46,   27),    p( -41,   26),    p( -34,   25),    p( -30,   21),    p( -22,   18),    p( -10,   11),    p(   1,    6),    p( -36,   19),
        p( -40,   31),    p( -40,   25),    p( -41,   29),    p( -36,   25),    p( -29,   19),    p( -28,   23),    p( -41,   29),    p( -43,   24),
    ],
    // queen
    [
        p( -13,   60),    p(  -9,   61),    p(   8,   65),    p(  29,   58),    p(  35,   60),    p(  54,   52),    p(  68,   21),    p(  21,   43),
        p(  17,   47),    p(  10,   53),    p(  13,   63),    p(  14,   70),    p(  22,   71),    p(  46,   65),    p(  37,   58),    p(  65,   44),
        p(  33,   42),    p(  29,   43),    p(  28,   59),    p(  27,   62),    p(  32,   66),    p(  64,   64),    p(  68,   53),    p(  60,   55),
        p(  22,   48),    p(  25,   52),    p(  24,   50),    p(  18,   62),    p(  24,   64),    p(  40,   59),    p(  43,   66),    p(  48,   54),
        p(  17,   48),    p(  16,   48),    p(  17,   48),    p(  19,   57),    p(  24,   57),    p(  28,   55),    p(  35,   55),    p(  39,   49),
        p(  18,   32),    p(  23,   36),    p(  18,   47),    p(  16,   50),    p(  21,   57),    p(  31,   42),    p(  38,   37),    p(  36,   27),
        p(  19,   28),    p(  17,   36),    p(  22,   38),    p(  22,   50),    p(  23,   51),    p(  27,   31),    p(  35,   16),    p(  24,   21),
        p(   3,   41),    p(  14,   29),    p(  14,   37),    p(  17,   42),    p(  20,   33),    p(   7,   39),    p(   1,   23),    p(  12,    6),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  67,    8),    p(  69,    6),    p(  73,   -2),    p(  71,  -41),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  55,   16),    p(  31,   25),    p(  31,   17),    p(  10,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  23,   25),    p(  36,   19),    p(  28,   13),    p(  -9,   14),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -40,   23),    p( -24,   17),    p( -26,    5),    p( -46,    9),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -60,   18),    p( -42,    7),    p( -45,   -4),    p( -62,    1),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -47,    9),    p( -46,    2),    p( -21,  -11),    p( -36,    2),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -42,    1),    p( -29,   -5),    p(  -4,  -18),    p(  14,   -6),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  10,   -5),    p(  -1,   -1),    p(  19,  -14),    p(  28,  -17),
    ],
];

const MORE_MINORS_NO_PAWNS: PhasedScore = p(-66, -65);
const BISHOP_PAIR: PhasedScore = p(25, 47);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(16, 16), p(17, 12), p(16, 2), p(11, -4), p(7, -11), p(3, -19), p(-4, -27), p(-10, -38), p(-21, -42)];
const ROOK_OPEN_FILE: PhasedScore = p(13, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(4, -0);
const KING_OPEN_FILE: PhasedScore = p(-30, 5);
const KING_CLOSED_FILE: PhasedScore = p(12, -6);
const KING_SEMIOPEN_FILE: PhasedScore = p(-3, 10);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-3, 1), p(2, 2), p(3, -0), p(4, -0), p(5, 1), p(6, 3), p(12, -1), p(22, -5)],
    // Closed
    [p(0, 0), p(0, 0), p(15, -18), p(-16, 8), p(2, 6), p(2, 1), p(3, 3), p(3, -1)],
    // SemiOpen
    [p(0, 0), p(-4, 21), p(10, 13), p(1, 7), p(2, 7), p(6, 2), p(6, -2), p(13, 0)],
    // SemiClosed
    [p(0, 0), p(13, -15), p(12, 1), p(3, -3), p(8, -2), p(3, -0), p(7, -0), p(6, -2)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(18, 10),
    p(1, 3),
    p(1, 4),
    p(-6, 4),
    p(5, 2),
    p(-7, -12),
    p(-3, -2),
    p(-3, -14),
    p(-2, -1),
    p(-12, -3),
    p(-8, -18),
    p(-14, -10),
    p(6, -8),
    p(1, -14),
    p(7, -9),
    p(7, 9),
    p(-7, -2),
    p(-20, -6),
    p(-11, -0),
    p(-35, 17),
    p(-17, 2),
    p(-14, -21),
    p(7, 22),
    p(-35, 16),
    p(-18, -17),
    p(-20, -18),
    p(-32, -31),
    p(-35, 10),
    p(-12, -3),
    p(17, -9),
    p(-64, 67),
    p(0, 0),
    p(-0, -5),
    p(-12, -8),
    p(1, -9),
    p(-17, -5),
    p(-18, -3),
    p(-40, -22),
    p(-22, 32),
    p(-26, 19),
    p(-6, -7),
    p(-17, -13),
    p(10, -15),
    p(-12, 26),
    p(-43, 14),
    p(2, -33),
    p(0, 0),
    p(0, 0),
    p(0, -13),
    p(-8, 2),
    p(1, -51),
    p(0, 0),
    p(15, -16),
    p(-29, -13),
    p(0, 0),
    p(0, 0),
    p(-24, -9),
    p(-15, -18),
    p(-10, 12),
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
    p(-18, -3),
    p(10, -4),
    p(-22, -11),
    p(-11, -6),
    p(-29, -17),
    p(8, -0),
    p(-8, -10),
    p(-26, -9),
    p(-39, -5),
    p(0, -7),
    p(-34, -11),
    p(-26, -13),
    p(-41, 47),
    p(8, 2),
    p(-1, -7),
    p(-3, -11),
    p(-19, -3),
    p(-8, -2),
    p(-12, -16),
    p(-18, -5),
    p(-20, 65),
    p(-6, -10),
    p(-23, -17),
    p(-30, -30),
    p(5, -67),
    p(-9, -11),
    p(-7, -24),
    p(-67, 52),
    p(0, 0),
    p(10, 3),
    p(-3, -3),
    p(-15, -7),
    p(-23, -9),
    p(-1, -0),
    p(-24, -20),
    p(-11, -6),
    p(-21, -8),
    p(-1, -6),
    p(-19, -10),
    p(-23, -17),
    p(-31, -11),
    p(-4, -4),
    p(-42, -11),
    p(14, 6),
    p(-50, 48),
    p(4, 1),
    p(-8, -5),
    p(-23, 49),
    p(0, 0),
    p(-9, -6),
    p(-14, -1),
    p(0, 0),
    p(0, 0),
    p(-11, -1),
    p(-31, 2),
    p(-24, -44),
    p(0, 0),
    p(23, -63),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-13, 8),   /*0b0000*/
    p(-13, 6),   /*0b0001*/
    p(-3, 9),    /*0b0010*/
    p(-1, 9),    /*0b0011*/
    p(-7, 1),    /*0b0100*/
    p(-21, -2),  /*0b0101*/
    p(-7, 3),    /*0b0110*/
    p(-8, -12),  /*0b0111*/
    p(1, 3),     /*0b1000*/
    p(-8, 7),    /*0b1001*/
    p(0, 8),     /*0b1010*/
    p(6, 8),     /*0b1011*/
    p(-3, 2),    /*0b1100*/
    p(-14, 2),   /*0b1101*/
    p(-7, 2),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(2, 6),     /*0b10000*/
    p(5, 4),     /*0b10001*/
    p(21, 4),    /*0b10010*/
    p(1, 6),     /*0b10011*/
    p(-7, 1),    /*0b10100*/
    p(14, 9),    /*0b10101*/
    p(-26, 0),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(7, 7),     /*0b11000*/
    p(27, 8),    /*0b11001*/
    p(29, 23),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(9, -6),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(7, 0),     /*0b100000*/
    p(0, 5),     /*0b100001*/
    p(15, 1),    /*0b100010*/
    p(7, -2),    /*0b100011*/
    p(-5, -5),   /*0b100100*/
    p(-18, -13), /*0b100101*/
    p(-18, 16),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(15, -5),   /*0b101000*/
    p(0, 6),     /*0b101001*/
    p(10, -7),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-0, -2),   /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(3, -0),    /*0b110000*/
    p(15, 1),    /*0b110001*/
    p(18, -8),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(11, 11),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(18, -9),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(7, -5),    /*0b111111*/
    p(-6, 7),    /*0b00*/
    p(-1, -3),   /*0b01*/
    p(16, -1),   /*0b10*/
    p(-8, -23),  /*0b11*/
    p(17, 2),    /*0b100*/
    p(-9, -3),   /*0b101*/
    p(28, -23),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(37, -5),   /*0b1000*/
    p(-10, -10), /*0b1001*/
    p(8, -29),   /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(0, -10),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-37, 26),  /*0b1111*/
    p(-1, 8),    /*0b00*/
    p(10, -4),   /*0b01*/
    p(6, -8),    /*0b10*/
    p(6, -30),   /*0b11*/
    p(10, -5),   /*0b100*/
    p(22, -12),  /*0b101*/
    p(-2, -12),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(11, 1),    /*0b1000*/
    p(20, -9),   /*0b1001*/
    p(22, -37),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(5, -18),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-8, -35),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(1, -35);
const STOPPABLE_PASSER: PhasedScore = p(15, -39);
const CLOSE_KING_PASSER: PhasedScore = p(-1, 26);
const IMMOBILE_PASSER: PhasedScore = p(-8, -33);
const PROTECTED_PASSER: PhasedScore = p(7, -0);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -25,   35),    p( -27,   52),    p( -37,   44),    p( -38,   27),    p( -38,   27),    p( -29,   25),    p( -16,   27),    p( -40,   30),
        p( -19,   33),    p( -32,   53),    p( -35,   43),    p( -34,   29),    p( -48,   36),    p( -43,   33),    p( -34,   40),    p( -34,   28),
        p( -12,   44),    p( -20,   43),    p( -31,   43),    p( -27,   41),    p( -36,   45),    p( -33,   46),    p( -34,   53),    p( -55,   53),
        p(   1,   53),    p(  -2,   51),    p(   0,   42),    p( -11,   51),    p( -25,   57),    p( -17,   61),    p( -18,   64),    p( -38,   65),
        p(   8,   55),    p(  20,   48),    p(   7,   43),    p(  -5,   32),    p(  -0,   43),    p(  -7,   55),    p( -20,   56),    p( -42,   61),
        p(  19,   52),    p(  21,   51),    p(  25,   46),    p(  27,   41),    p(  26,   46),    p(  30,   56),    p(   1,   60),    p(   5,   60),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(-0, 4), p(4, 8), p(7, 17), p(12, 28), p(18, 55), p(22, 55)];
const UNSUPPORTED_PAWN: PhasedScore = p(-9, -4);
const DOUBLED_PAWN: PhasedScore = p(-7, -23);
const PHALANX: [PhasedScore; 6] = [p(-3, 1), p(4, 4), p(8, 6), p(21, 19), p(58, 59), p(67, 64)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(15, 14), p(7, 17), p(14, 20), p(6, 10), p(-3, 11), p(-47, 12)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(48, 20), p(50, 41), p(58, 7), p(48, -1), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, -4), p(14, 20), p(18, -7), p(16, 9), p(16, -12), p(29, -11)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-40, -60),
        p(-18, -35),
        p(-6, -13),
        p(4, -1),
        p(11, 8),
        p(18, 18),
        p(26, 20),
        p(33, 22),
        p(36, 21),
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
        p(-30, -48),
        p(-17, -33),
        p(-5, -19),
        p(2, -7),
        p(9, 2),
        p(14, 11),
        p(19, 15),
        p(24, 18),
        p(26, 23),
        p(33, 24),
        p(38, 24),
        p(43, 27),
        p(37, 37),
        p(47, 29),
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
        p(-48, -20),
        p(-40, -5),
        p(-37, 3),
        p(-33, 8),
        p(-34, 15),
        p(-30, 21),
        p(-28, 26),
        p(-25, 31),
        p(-22, 36),
        p(-20, 40),
        p(-16, 43),
        p(-15, 48),
        p(-9, 49),
        p(-1, 48),
        p(2, 45),
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
        p(1, -65),
        p(3, -64),
        p(-1, -32),
        p(3, -16),
        p(5, 3),
        p(9, 10),
        p(12, 22),
        p(15, 30),
        p(18, 36),
        p(21, 38),
        p(24, 43),
        p(28, 47),
        p(30, 49),
        p(32, 53),
        p(34, 56),
        p(39, 59),
        p(42, 63),
        p(46, 63),
        p(54, 62),
        p(63, 61),
        p(65, 63),
        p(67, 67),
        p(68, 69),
        p(66, 67),
        p(68, 75),
        p(64, 70),
        p(65, 74),
        p(62, 61),
    ],
    [
        p(-22, -19),
        p(-15, -22),
        p(-9, -16),
        p(-2, -7),
        p(7, -1),
        p(10, 5),
        p(20, 14),
        p(26, 20),
        p(52, 16),
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
    [p(-4, 13), p(0, 0), p(28, 24), p(56, 8), p(38, -3), p(0, 0)],
    [p(-1, 11), p(21, 24), p(0, 0), p(42, 19), p(50, 64), p(0, 0)],
    [p(-6, 19), p(9, 19), p(16, 15), p(0, 0), p(56, 54), p(0, 0)],
    [p(-4, 17), p(-0, 18), p(-1, 30), p(-2, 22), p(0, 0), p(0, 0)],
    [p(50, 22), p(-19, 23), p(8, 17), p(-12, 13), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 8), p(8, 6), p(6, 10), p(12, 7), p(8, 16), p(3, 7)],
    [p(3, 8), p(12, 19), p(-52, -34), p(9, 11), p(10, 18), p(3, 6)],
    [p(-3, 7), p(8, 11), p(4, 16), p(7, 12), p(4, 33), p(14, -3)],
    [p(-0, 11), p(6, 12), p(5, 8), p(3, 18), p(-72, -69), p(1, -4)],
    [p(25, 4), p(13, 16), p(19, 10), p(4, 11), p(16, -1), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(8, -20), p(18, -8), p(12, -2), p(17, -13), p(4, 5), p(6, 0)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(10, 1), p(-7, 10), p(14, -6), p(-18, 35)];
const CHECK_STM: PhasedScore = p(32, 9);
const SAFE_CHECK_STM: PhasedScore = p(58, 8);
const DISCOVERED_CHECK_STM: PhasedScore = p(65, 66);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(11, -5), p(63, 28), p(65, 15), p(67, 67), p(0, 0), p(35, -24)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(7, -13), p(30, 31), p(21, 35), p(40, 16), p(58, 1)];
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

    fn safe_check_stm() -> PhasedScore {
        SAFE_CHECK_STM
    }
}
