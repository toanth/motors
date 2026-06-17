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
        p(  20,   53),    p(  23,   52),    p(  26,   48),    p(  28,   43),    p(  28,   47),    p(  31,   57),    p(   5,   60),    p(   8,   60),
        p( -28,   23),    p( -24,   22),    p( -11,   12),    p( -14,   14),    p( -20,   13),    p(  25,   12),    p(  12,   33),    p(   6,   28),
        p( -40,    1),    p( -32,   -3),    p( -35,  -10),    p( -12,   -6),    p(  -7,   -8),    p( -10,  -16),    p(  -9,   -6),    p( -11,   -7),
        p( -48,  -12),    p( -44,  -10),    p( -24,  -10),    p( -12,   -8),    p(  -6,   -9),    p(  -7,  -11),    p( -18,  -14),    p( -24,  -17),
        p( -56,  -13),    p( -47,  -16),    p( -28,   -9),    p( -18,   -7),    p( -18,   -6),    p( -15,   -9),    p( -13,  -23),    p( -26,  -20),
        p( -47,   -5),    p( -42,   -8),    p( -36,   -9),    p( -38,   -3),    p( -33,   -1),    p(  -6,  -12),    p(   8,  -22),    p( -16,  -19),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
    ],
    // knight
    [
        p( -63,  -43),    p( -66,    9),    p( -67,   14),    p( -47,   16),    p(  -5,    3),    p( -64,    4),    p( -58,   -6),    p( -62,  -55),
        p( -19,   10),    p(  -7,   17),    p(   8,    5),    p(  13,    7),    p(  15,    0),    p(  24,   -6),    p(   4,    5),    p( -14,   -4),
        p(  -5,    6),    p(  17,    1),    p(  25,    4),    p(  40,    5),    p(  44,    2),    p(  54,   -9),    p(   3,   -0),    p(   4,   -1),
        p(  11,   13),    p(  19,    8),    p(  38,   10),    p(  41,   16),    p(  43,   13),    p(  44,    9),    p(  24,   12),    p(  36,    4),
        p(   9,   14),    p(  22,    7),    p(  29,   13),    p(  33,   15),    p(  36,   17),    p(  42,    5),    p(  37,    4),    p(  26,   14),
        p( -21,    1),    p( -11,   -1),    p(  -0,   -3),    p(   8,    8),    p(  12,    7),    p(  10,  -12),    p(   9,   -7),    p(   2,    2),
        p( -29,    6),    p( -14,    9),    p( -11,    4),    p(  -2,    7),    p(   5,    2),    p(   1,   -4),    p(  -0,    5),    p(  -3,   14),
        p( -50,   -1),    p( -22,    5),    p( -29,    2),    p(  -8,    7),    p(  -4,    7),    p(  -1,   -4),    p( -15,    7),    p( -22,    3),
    ],
    // bishop
    [
        p( -27,   18),    p( -38,   10),    p( -60,    6),    p( -57,   12),    p( -62,    9),    p( -52,    6),    p( -36,   10),    p( -42,    9),
        p( -16,    5),    p(  -9,    5),    p(   0,    2),    p( -13,    2),    p(  -4,   -2),    p(  -6,   -0),    p( -35,   12),    p( -22,    3),
        p(   0,    7),    p(   6,    3),    p(  -3,    5),    p(   7,   -5),    p(   2,    1),    p(  28,    1),    p(  15,    4),    p(   8,   12),
        p(  -5,    8),    p(   4,    3),    p(   8,   -1),    p(  10,    5),    p(  14,    1),    p(  13,    1),    p(  19,   -2),    p(  -3,    4),
        p(   2,    1),    p(  -1,    3),    p(   5,    1),    p(  15,    0),    p(  15,    0),    p(  18,   -7),    p(   8,   -3),    p(  29,   -9),
        p(  -3,    3),    p(   2,    1),    p(   2,    4),    p(  -0,    4),    p(   6,    3),    p(   4,   -2),    p(  12,   -8),    p(  12,   -1),
        p(   7,    0),    p(   0,   -0),    p(   7,   -1),    p(   0,    6),    p(   2,    4),    p(   9,    2),    p(  17,   -4),    p(  13,    0),
        p(   3,   -0),    p(   9,    2),    p(   2,    7),    p(  -6,    6),    p(   6,    6),    p(  -8,    9),    p(   3,    7),    p(  11,   -4),
    ],
    // rook
    [
        p( -35,   44),    p( -38,   46),    p( -48,   51),    p( -50,   49),    p( -41,   45),    p( -27,   47),    p( -34,   47),    p(  -7,   32),
        p( -24,   40),    p( -23,   43),    p( -14,   43),    p(   3,   34),    p(  -9,   36),    p(  17,   31),    p(  14,   31),    p(  12,   23),
        p( -28,   37),    p(  -8,   30),    p( -12,   32),    p( -11,   25),    p(  14,   18),    p(  29,   14),    p(  31,   18),    p(   0,   21),
        p( -30,   36),    p( -20,   31),    p( -20,   33),    p( -13,   27),    p(  -7,   23),    p(   4,   21),    p(  -3,   25),    p(  -9,   22),
        p( -37,   34),    p( -31,   31),    p( -30,   33),    p( -25,   29),    p( -16,   27),    p( -18,   27),    p(  -1,   21),    p( -21,   22),
        p( -44,   30),    p( -36,   24),    p( -37,   26),    p( -34,   26),    p( -25,   19),    p( -17,   15),    p(  -2,    7),    p( -17,   13),
        p( -43,   27),    p( -36,   24),    p( -29,   24),    p( -24,   20),    p( -18,   16),    p(  -7,   10),    p(   2,    5),    p( -34,   19),
        p( -35,   30),    p( -36,   24),    p( -37,   29),    p( -32,   24),    p( -25,   19),    p( -23,   23),    p( -36,   28),    p( -40,   23),
    ],
    // queen
    [
        p( -23,   56),    p( -20,   54),    p( -10,   61),    p(  10,   54),    p(  14,   53),    p(  36,   44),    p(  62,    6),    p(   9,   33),
        p(  11,   43),    p(  12,   45),    p(  19,   50),    p(  15,   63),    p(  12,   70),    p(  36,   55),    p(  26,   52),    p(  55,   29),
        p(  30,   34),    p(  28,   34),    p(  29,   48),    p(  29,   53),    p(  28,   57),    p(  47,   52),    p(  55,   37),    p(  41,   43),
        p(  19,   44),    p(  25,   48),    p(  24,   48),    p(  15,   60),    p(  24,   57),    p(  39,   50),    p(  37,   60),    p(  37,   50),
        p(  17,   44),    p(  19,   47),    p(  18,   49),    p(  19,   58),    p(  28,   54),    p(  30,   52),    p(  38,   48),    p(  38,   46),
        p(  13,   34),    p(  17,   38),    p(  13,   50),    p(  12,   54),    p(  15,   60),    p(  26,   44),    p(  32,   38),    p(  31,   26),
        p(  14,   25),    p(  12,   34),    p(  17,   39),    p(  17,   47),    p(  18,   47),    p(  20,   31),    p(  26,   14),    p(  18,   19),
        p(  -3,   34),    p(   7,   27),    p(   8,   34),    p(  12,   37),    p(  14,   33),    p(   2,   36),    p(  -1,   17),    p(   7,    2),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  67,    7),    p(  66,    4),    p(  70,   -3),    p(  67,  -40),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  53,   18),    p(  26,   27),    p(  17,   22),    p(   3,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  19,   27),    p(  31,   20),    p(  21,   15),    p(  -6,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -36,   24),    p( -21,   18),    p( -27,    7),    p( -40,    8),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -49,   18),    p( -27,    6),    p( -33,   -5),    p( -57,   -0),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -40,    9),    p( -38,    2),    p( -20,   -9),    p( -35,    0),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -45,    4),    p( -33,   -2),    p( -11,  -15),    p(   9,   -7),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   6,   -4),    p(  -3,   -2),    p(  17,  -15),    p(  25,  -19),
    ],
];

const MORE_MINORS_NO_PAWNS: PhasedScore = p(-67, -66);
const OPPOSITE_COLORED_BISHOPS: PhasedScore = p(14, -35);
const BISHOP_PAIR: PhasedScore = p(25, 48);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(16, 19), p(17, 15), p(16, 5), p(11, -1), p(7, -8), p(2, -15), p(-4, -22), p(-11, -33), p(-23, -33)];
const ROOK_OPEN_FILE: PhasedScore = p(14, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(4, -0);
const KING_OPEN_FILE: PhasedScore = p(-25, 3);
const KING_CLOSED_FILE: PhasedScore = p(12, -7);
const KING_SEMIOPEN_FILE: PhasedScore = p(-1, 9);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 5), p(-1, 6), p(1, 3), p(3, 3), p(5, 4), p(7, 5), p(13, 0), p(22, -3)],
    // Closed
    [p(0, 0), p(0, 0), p(15, -16), p(-14, 7), p(3, 8), p(2, 3), p(5, 5), p(2, 1)],
    // SemiOpen
    [p(0, 0), p(-9, 23), p(5, 17), p(-1, 10), p(1, 9), p(6, 4), p(8, -1), p(13, 2)],
    // SemiClosed
    [p(0, 0), p(9, -13), p(10, 3), p(3, -1), p(8, 1), p(3, 3), p(9, 2), p(6, -0)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(18, 9),
    p(1, 2),
    p(1, 4),
    p(-7, 4),
    p(5, 2),
    p(-7, -12),
    p(-3, -2),
    p(-2, -14),
    p(-2, -2),
    p(-13, -3),
    p(-7, -17),
    p(-15, -9),
    p(4, -7),
    p(-2, -12),
    p(5, -7),
    p(4, 12),
    p(-6, -2),
    p(-20, -6),
    p(-11, 1),
    p(-34, 18),
    p(-16, 2),
    p(-15, -19),
    p(9, 23),
    p(-40, 30),
    p(-17, -16),
    p(-20, -16),
    p(-31, -29),
    p(-39, 10),
    p(-15, -1),
    p(14, -7),
    p(-62, 69),
    p(0, 0),
    p(0, -4),
    p(-11, -7),
    p(2, -7),
    p(-18, -2),
    p(-19, -2),
    p(-42, -20),
    p(-20, 34),
    p(-24, 17),
    p(-7, -7),
    p(-18, -11),
    p(8, -13),
    p(-14, 30),
    p(-44, 15),
    p(-0, -28),
    p(0, 0),
    p(0, 0),
    p(3, -10),
    p(-5, 7),
    p(6, -51),
    p(0, 0),
    p(15, -12),
    p(-28, -16),
    p(0, 0),
    p(0, 0),
    p(-25, -4),
    p(-17, -12),
    p(-8, 14),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(22, 8),
    p(2, 1),
    p(-4, 4),
    p(-18, -3),
    p(9, -4),
    p(-23, -10),
    p(-10, -6),
    p(-29, -18),
    p(7, -1),
    p(-9, -9),
    p(-26, -7),
    p(-39, -3),
    p(-1, -6),
    p(-35, -10),
    p(-27, -11),
    p(-43, 48),
    p(9, 2),
    p(-1, -7),
    p(-3, -8),
    p(-18, -2),
    p(-7, -2),
    p(-14, -15),
    p(-18, -1),
    p(-21, 65),
    p(-7, -9),
    p(-25, -15),
    p(-32, -28),
    p(-0, -62),
    p(-11, -10),
    p(-9, -22),
    p(-71, 56),
    p(0, 0),
    p(11, 3),
    p(-3, -2),
    p(-14, -6),
    p(-22, -7),
    p(-1, 1),
    p(-26, -18),
    p(-12, -4),
    p(-25, -2),
    p(-1, -6),
    p(-20, -9),
    p(-23, -16),
    p(-32, -7),
    p(-8, -1),
    p(-45, -11),
    p(13, 10),
    p(-49, 47),
    p(5, 1),
    p(-8, -3),
    p(-20, 50),
    p(0, 0),
    p(-10, -4),
    p(-16, 3),
    p(0, 0),
    p(0, 0),
    p(-12, 0),
    p(-35, 6),
    p(-23, -38),
    p(0, 0),
    p(20, -61),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-8, 8),    /*0b0000*/
    p(-13, 6),   /*0b0001*/
    p(-1, 8),    /*0b0010*/
    p(-3, 8),    /*0b0011*/
    p(-7, 3),    /*0b0100*/
    p(-23, -0),  /*0b0101*/
    p(-7, 2),    /*0b0110*/
    p(-9, -13),  /*0b0111*/
    p(2, 4),     /*0b1000*/
    p(-6, 7),    /*0b1001*/
    p(0, 8),     /*0b1010*/
    p(5, 8),     /*0b1011*/
    p(-4, 2),    /*0b1100*/
    p(-19, 3),   /*0b1101*/
    p(-7, 2),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(3, 7),     /*0b10000*/
    p(5, 4),     /*0b10001*/
    p(21, 4),    /*0b10010*/
    p(-0, 6),    /*0b10011*/
    p(-6, 2),    /*0b10100*/
    p(14, 10),   /*0b10101*/
    p(-24, 0),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(6, 9),     /*0b11000*/
    p(26, 9),    /*0b11001*/
    p(28, 23),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(9, -6),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(8, 1),     /*0b100000*/
    p(-2, 5),    /*0b100001*/
    p(14, 0),    /*0b100010*/
    p(5, -3),    /*0b100011*/
    p(-5, -3),   /*0b100100*/
    p(-21, -11), /*0b100101*/
    p(-17, 16),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(13, -3),   /*0b101000*/
    p(-4, 7),    /*0b101001*/
    p(9, -7),    /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-2, -1),   /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(2, 1),     /*0b110000*/
    p(13, 0),    /*0b110001*/
    p(19, -8),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(11, 13),   /*0b110100*/
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
    p(7, -6),    /*0b111111*/
    p(4, 5),     /*0b00*/
    p(6, -3),    /*0b01*/
    p(16, -3),   /*0b10*/
    p(-5, -25),  /*0b11*/
    p(22, 1),    /*0b100*/
    p(2, -1),    /*0b101*/
    p(30, -27),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(42, -7),   /*0b1000*/
    p(-6, -12),  /*0b1001*/
    p(12, -30),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(7, -10),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-35, 30),  /*0b1111*/
    p(5, 7),     /*0b00*/
    p(11, -5),   /*0b01*/
    p(11, -8),   /*0b10*/
    p(8, -31),   /*0b11*/
    p(11, -4),   /*0b100*/
    p(23, -13),  /*0b101*/
    p(1, -13),   /*0b110*/
    p(0, 0),     /*0b111*/
    p(15, 1),    /*0b1000*/
    p(23, -13),  /*0b1001*/
    p(28, -38),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(8, -18),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-2, -36),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(-5, -34);
const STOPPABLE_PASSER: PhasedScore = p(13, -39);
const CLOSE_KING_PASSER: PhasedScore = p(-2, 25);
const IMMOBILE_PASSER: PhasedScore = p(-7, -32);
const PROTECTED_PASSER: PhasedScore = p(6, -0);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -23,   36),    p( -23,   51),    p( -33,   44),    p( -32,   26),    p( -34,   27),    p( -28,   25),    p( -16,   28),    p( -37,   31),
        p( -19,   33),    p( -31,   53),    p( -32,   43),    p( -31,   28),    p( -45,   35),    p( -41,   33),    p( -32,   39),    p( -33,   28),
        p( -12,   44),    p( -19,   44),    p( -27,   43),    p( -24,   41),    p( -34,   45),    p( -30,   46),    p( -32,   54),    p( -52,   53),
        p(   0,   53),    p(  -2,   51),    p(   0,   43),    p(  -9,   50),    p( -24,   58),    p( -16,   61),    p( -17,   64),    p( -35,   65),
        p(  10,   55),    p(  23,   49),    p(   8,   44),    p(  -4,   34),    p(   4,   43),    p(  -6,   56),    p( -16,   56),    p( -39,   61),
        p(  20,   53),    p(  23,   52),    p(  26,   48),    p(  28,   43),    p(  28,   47),    p(  31,   57),    p(   5,   60),    p(   8,   60),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(-2, 4), p(4, 9), p(7, 18), p(11, 29), p(18, 55), p(24, 55)];
const UNSUPPORTED_PAWN: PhasedScore = p(-8, -4);
const DOUBLED_PAWN: PhasedScore = p(-6, -23);
const PHALANX: [PhasedScore; 6] = [p(-3, 1), p(4, 4), p(8, 6), p(20, 19), p(60, 60), p(67, 65)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(15, 14), p(7, 16), p(13, 20), p(7, 9), p(-2, 10), p(-45, 12)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(50, 22), p(53, 43), p(60, 8), p(51, -2), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(2, -2), p(17, 16), p(19, -3), p(17, 10), p(15, -5), p(35, -10)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-42, -53),
        p(-20, -31),
        p(-7, -12),
        p(4, -1),
        p(13, 9),
        p(20, 19),
        p(29, 23),
        p(35, 26),
        p(37, 28),
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
        p(-28, -32),
        p(-12, -22),
        p(-0, -11),
        p(10, -3),
        p(19, 6),
        p(27, 11),
        p(34, 15),
        p(37, 22),
        p(45, 25),
        p(49, 29),
        p(51, 35),
        p(42, 50),
        p(42, 48),
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
        p(-55, -5),
        p(-41, -2),
        p(-31, -1),
        p(-22, 0),
        p(-17, 4),
        p(-7, 9),
        p(-1, 14),
        p(5, 19),
        p(11, 25),
        p(16, 30),
        p(22, 34),
        p(23, 42),
        p(28, 46),
        p(31, 48),
        p(28, 50),
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
        p(-16, -57),
        p(-24, -43),
        p(-32, 1),
        p(-28, 11),
        p(-24, 23),
        p(-18, 23),
        p(-12, 30),
        p(-6, 34),
        p(1, 38),
        p(6, 38),
        p(11, 41),
        p(18, 44),
        p(23, 43),
        p(27, 47),
        p(31, 49),
        p(37, 51),
        p(40, 57),
        p(45, 58),
        p(54, 57),
        p(62, 57),
        p(64, 62),
        p(67, 67),
        p(68, 69),
        p(66, 68),
        p(69, 77),
        p(65, 71),
        p(66, 76),
        p(62, 62),
    ],
    [
        p(12, 13),
        p(6, -8),
        p(3, -12),
        p(0, -8),
        p(-3, -4),
        p(-10, 1),
        p(-6, 8),
        p(-10, 14),
        p(2, 14),
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
        p(7, 7),
        p(5, 7),
        p(1, 5),
        p(-1, 4),
        p(-2, 1),
        p(3, -4),
        p(9, -15),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
    ],
    [
        p(21, -11),
        p(15, 2),
        p(8, 5),
        p(3, 7),
        p(-2, 6),
        p(-5, 5),
        p(-7, 2),
        p(-9, 1),
        p(-10, -4),
        p(-13, -4),
        p(4, -14),
        p(-14, -18),
        p(4, -41),
        p(65, -31),
        p(0, 0),
    ],
    [
        p(1, 5),
        p(-7, 22),
        p(-15, 30),
        p(-21, 33),
        p(-29, 35),
        p(-34, 36),
        p(-38, 34),
        p(-42, 32),
        p(-41, 30),
        p(-39, 26),
        p(-31, 20),
        p(-11, 14),
        p(5, 5),
        p(41, -3),
        p(63, -42),
    ],
    [
        p(19, 4),
        p(29, 29),
        p(32, 36),
        p(32, 41),
        p(28, 48),
        p(25, 54),
        p(21, 56),
        p(17, 58),
        p(14, 59),
        p(12, 59),
        p(11, 54),
        p(10, 51),
        p(12, 49),
        p(13, 43),
        p(39, 14),
    ],
    [
        p(-40, -27),
        p(-24, -5),
        p(-14, 5),
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
    [p(-5, 13), p(0, 0), p(25, 25), p(54, 8), p(35, -5), p(0, 0)],
    [p(-2, 13), p(18, 27), p(0, 0), p(41, 21), p(49, 63), p(0, 0)],
    [p(-8, 22), p(4, 23), p(12, 18), p(0, 0), p(52, 54), p(0, 0)],
    [p(-4, 18), p(-2, 21), p(-2, 31), p(-1, 17), p(0, 0), p(0, 0)],
    [p(54, 20), p(-13, 21), p(23, 11), p(-1, 16), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 8), p(7, 7), p(5, 9), p(11, 7), p(5, 16), p(3, 7)],
    [p(2, 9), p(10, 22), p(-62, -30), p(7, 13), p(8, 22), p(2, 6)],
    [p(-3, 8), p(7, 13), p(3, 17), p(8, 12), p(3, 33), p(12, -4)],
    [p(-0, 9), p(5, 11), p(5, 6), p(3, 18), p(-71, -68), p(0, -8)],
    [p(24, 2), p(13, 12), p(18, 8), p(6, 8), p(18, 0), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(1, -19), p(2, -12), p(0, -7), p(6, -23), p(2, -7), p(-17, -2)];
const DOUBLE_KINGZONE_ATTACK: PhasedScore = p(51, 2);
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(4, -2), p(-9, 8), p(0, -8), p(-23, 14)];
const SAFE_CHECK: [PhasedScore; 5] = [p(0, 0), p(56, -7), p(11, 1), p(42, -5), p(27, 42)];
const CHECK_STM: PhasedScore = p(36, 14);
const SAFE_CHECK_STM: PhasedScore = p(35, 5);
const DISCOVERED_CHECK_STM: PhasedScore = p(65, 66);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(11, -3), p(63, 33), p(65, 22), p(67, 68), p(0, 0), p(55, -29)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(12, -7), p(34, 40), p(31, 43), p(49, 23), p(60, 18)];
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
