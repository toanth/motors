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
        p(  21,   49),    p(  23,   48),    p(  26,   44),    p(  28,   39),    p(  28,   44),    p(  31,   54),    p(   5,   57),    p(   6,   58),
        p( -26,   19),    p( -22,   19),    p( -10,   10),    p( -12,   11),    p( -18,   10),    p(  26,    9),    p(  14,   28),    p(   9,   23),
        p( -39,    0),    p( -32,   -4),    p( -35,  -11),    p( -12,   -7),    p(  -8,   -8),    p(  -9,  -17),    p(  -8,   -7),    p( -10,   -8),
        p( -47,  -13),    p( -44,  -11),    p( -24,  -11),    p( -12,   -8),    p(  -6,   -9),    p(  -7,  -12),    p( -18,  -14),    p( -24,  -18),
        p( -56,  -14),    p( -46,  -18),    p( -28,  -10),    p( -18,   -7),    p( -17,   -7),    p( -15,   -9),    p( -12,  -24),    p( -26,  -20),
        p( -47,   -6),    p( -41,   -9),    p( -35,  -10),    p( -38,   -4),    p( -32,   -2),    p(  -6,  -13),    p(   8,  -23),    p( -16,  -20),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
    ],
    // knight
    [
        p( -63,  -43),    p( -66,    9),    p( -67,   15),    p( -49,   17),    p(  -6,    4),    p( -64,    5),    p( -58,   -5),    p( -62,  -55),
        p( -19,    9),    p(  -7,   17),    p(   8,    5),    p(  13,    8),    p(  14,    1),    p(  23,   -5),    p(   4,    6),    p( -14,   -4),
        p(  -5,    6),    p(  17,    0),    p(  25,    5),    p(  40,    5),    p(  44,    3),    p(  53,   -8),    p(   2,    0),    p(   4,   -1),
        p(  11,   12),    p(  19,    8),    p(  38,   10),    p(  41,   15),    p(  43,   13),    p(  43,    9),    p(  24,   12),    p(  36,    4),
        p(   9,   13),    p(  22,    6),    p(  29,   13),    p(  33,   15),    p(  36,   17),    p(  42,    5),    p(  37,    4),    p(  26,   14),
        p( -21,    0),    p( -10,   -2),    p(  -0,   -3),    p(   8,    9),    p(  12,    7),    p(  10,  -13),    p(   9,   -7),    p(   2,    1),
        p( -28,    6),    p( -14,    8),    p( -10,    3),    p(  -2,    6),    p(   6,    2),    p(   1,   -5),    p(  -0,    5),    p(  -3,   14),
        p( -50,   -2),    p( -22,    4),    p( -29,    1),    p(  -8,    6),    p(  -3,    7),    p(  -1,   -5),    p( -15,    7),    p( -22,    3),
    ],
    // bishop
    [
        p( -27,   18),    p( -39,   11),    p( -61,    6),    p( -57,   12),    p( -62,    9),    p( -53,    7),    p( -36,   10),    p( -42,    8),
        p( -16,    5),    p(  -9,    5),    p(   0,    3),    p( -13,    2),    p(  -4,   -2),    p(  -6,   -1),    p( -35,   13),    p( -22,    3),
        p(   0,    7),    p(   6,    3),    p(  -3,    5),    p(   7,   -5),    p(   2,    1),    p(  28,    1),    p(  15,    4),    p(   8,   12),
        p(  -5,    8),    p(   4,    3),    p(   8,   -1),    p(  10,    5),    p(  13,    2),    p(  13,    2),    p(  19,   -1),    p(  -3,    5),
        p(   2,    2),    p(  -1,    4),    p(   5,    2),    p(  14,    1),    p(  15,    1),    p(  18,   -5),    p(   8,   -1),    p(  29,   -8),
        p(  -4,    3),    p(   2,    1),    p(   2,    4),    p(  -0,    4),    p(   6,    3),    p(   4,   -2),    p(  12,   -7),    p(  12,   -0),
        p(   7,   -0),    p(   0,   -0),    p(   7,   -2),    p(   0,    6),    p(   2,    3),    p(   9,    2),    p(  17,   -4),    p(  13,    0),
        p(   3,   -0),    p(   9,    1),    p(   3,    7),    p(  -6,    5),    p(   6,    6),    p(  -7,    9),    p(   3,    6),    p(  12,   -4),
    ],
    // rook
    [
        p( -36,   45),    p( -39,   48),    p( -49,   53),    p( -52,   51),    p( -43,   46),    p( -29,   49),    p( -35,   49),    p(  -8,   34),
        p( -25,   42),    p( -24,   45),    p( -15,   45),    p(   2,   35),    p( -10,   37),    p(  16,   33),    p(  12,   33),    p(  10,   25),
        p( -28,   38),    p(  -8,   31),    p( -12,   33),    p( -11,   25),    p(  14,   18),    p(  29,   14),    p(  31,   19),    p(  -0,   21),
        p( -30,   35),    p( -20,   31),    p( -19,   32),    p( -13,   26),    p(  -7,   22),    p(   5,   20),    p(  -2,   24),    p(  -9,   21),
        p( -36,   33),    p( -30,   30),    p( -30,   32),    p( -25,   27),    p( -16,   25),    p( -17,   25),    p(  -1,   20),    p( -21,   21),
        p( -43,   28),    p( -36,   23),    p( -37,   25),    p( -34,   25),    p( -25,   18),    p( -17,   14),    p(  -2,    6),    p( -17,   12),
        p( -43,   26),    p( -36,   24),    p( -29,   24),    p( -24,   20),    p( -18,   16),    p(  -8,   10),    p(   1,    5),    p( -34,   18),
        p( -35,   29),    p( -36,   24),    p( -37,   28),    p( -32,   24),    p( -25,   19),    p( -23,   23),    p( -37,   29),    p( -40,   23),
    ],
    // queen
    [
        p( -24,   59),    p( -22,   58),    p( -11,   64),    p(   9,   56),    p(  13,   55),    p(  34,   47),    p(  61,    9),    p(   8,   36),
        p(  10,   44),    p(  11,   47),    p(  18,   52),    p(  14,   64),    p(  12,   70),    p(  36,   57),    p(  25,   54),    p(  54,   32),
        p(  30,   34),    p(  28,   35),    p(  29,   48),    p(  28,   53),    p(  27,   57),    p(  47,   52),    p(  54,   38),    p(  40,   45),
        p(  19,   43),    p(  25,   48),    p(  24,   47),    p(  16,   59),    p(  23,   57),    p(  38,   51),    p(  36,   61),    p(  37,   52),
        p(  17,   44),    p(  19,   47),    p(  18,   48),    p(  19,   57),    p(  28,   54),    p(  30,   52),    p(  37,   49),    p(  38,   46),
        p(  13,   33),    p(  17,   37),    p(  13,   49),    p(  12,   53),    p(  15,   59),    p(  26,   43),    p(  32,   37),    p(  31,   26),
        p(  15,   24),    p(  12,   33),    p(  17,   38),    p(  17,   46),    p(  18,   46),    p(  20,   31),    p(  26,   14),    p(  18,   20),
        p(  -3,   34),    p(   7,   27),    p(   8,   34),    p(  12,   37),    p(  14,   33),    p(   2,   36),    p(  -2,   18),    p(   7,    4),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  66,    8),    p(  66,    5),    p(  69,   -3),    p(  67,  -40),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  52,   18),    p(  25,   27),    p(  16,   21),    p(   4,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  17,   27),    p(  30,   20),    p(  20,   15),    p(  -4,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -36,   24),    p( -21,   18),    p( -25,    6),    p( -39,    9),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -48,   18),    p( -27,    7),    p( -32,   -5),    p( -57,    0),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -40,    9),    p( -38,    1),    p( -19,  -11),    p( -35,   -0),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -44,    3),    p( -33,   -3),    p( -11,  -16),    p(   9,   -7),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   6,   -4),    p(  -3,   -1),    p(  17,  -15),    p(  24,  -18),
    ],
];

const MORE_MINORS_NO_PAWNS: PhasedScore = p(-67, -66);
const OPPOSITE_COLORED_BISHOPS: PhasedScore = p(13, -35);
const BISHOP_PAIR: PhasedScore = p(25, 48);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(16, 19), p(17, 14), p(16, 5), p(11, -1), p(6, -7), p(2, -15), p(-5, -21), p(-12, -32), p(-23, -32)];
const ROOK_OPEN_FILE: PhasedScore = p(13, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -0);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(4, -1);
const KING_OPEN_FILE: PhasedScore = p(-25, 4);
const KING_CLOSED_FILE: PhasedScore = p(12, -6);
const KING_SEMIOPEN_FILE: PhasedScore = p(-1, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 4), p(-1, 6), p(1, 3), p(3, 3), p(5, 4), p(7, 5), p(13, 1), p(21, -2)],
    // Closed
    [p(0, 0), p(0, 0), p(17, -19), p(-14, 7), p(3, 9), p(2, 3), p(5, 5), p(2, 1)],
    // SemiOpen
    [p(0, 0), p(-9, 23), p(5, 17), p(-1, 10), p(1, 9), p(6, 4), p(8, -1), p(13, 2)],
    // SemiClosed
    [p(0, 0), p(9, -14), p(10, 3), p(3, -1), p(8, 1), p(3, 3), p(9, 2), p(6, -0)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(18, 10),
    p(1, 2),
    p(1, 3),
    p(-7, 3),
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
    p(5, -8),
    p(5, 9),
    p(-6, -2),
    p(-20, -7),
    p(-11, -0),
    p(-33, 18),
    p(-16, 2),
    p(-14, -20),
    p(9, 22),
    p(-38, 26),
    p(-17, -16),
    p(-20, -16),
    p(-31, -30),
    p(-38, 8),
    p(-15, -3),
    p(14, -7),
    p(-62, 69),
    p(0, 0),
    p(0, -4),
    p(-11, -7),
    p(2, -8),
    p(-18, -3),
    p(-19, -3),
    p(-42, -21),
    p(-20, 33),
    p(-25, 19),
    p(-7, -7),
    p(-18, -11),
    p(9, -14),
    p(-14, 31),
    p(-43, 14),
    p(0, -28),
    p(0, 0),
    p(0, 0),
    p(3, -10),
    p(-5, 6),
    p(5, -52),
    p(0, 0),
    p(16, -13),
    p(-29, -15),
    p(0, 0),
    p(0, 0),
    p(-24, -5),
    p(-16, -12),
    p(-7, 13),
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
    p(9, -4),
    p(-23, -10),
    p(-10, -6),
    p(-29, -18),
    p(7, -1),
    p(-8, -9),
    p(-26, -7),
    p(-38, -4),
    p(-1, -6),
    p(-35, -10),
    p(-26, -12),
    p(-42, 47),
    p(8, 2),
    p(-1, -7),
    p(-4, -9),
    p(-18, -3),
    p(-7, -2),
    p(-14, -15),
    p(-18, -2),
    p(-21, 65),
    p(-7, -9),
    p(-25, -15),
    p(-32, -29),
    p(-0, -63),
    p(-10, -11),
    p(-9, -23),
    p(-71, 55),
    p(0, 0),
    p(11, 3),
    p(-3, -2),
    p(-14, -6),
    p(-22, -8),
    p(-2, 0),
    p(-26, -18),
    p(-11, -6),
    p(-25, -3),
    p(-1, -7),
    p(-19, -10),
    p(-23, -16),
    p(-32, -8),
    p(-7, -3),
    p(-44, -12),
    p(14, 8),
    p(-48, 45),
    p(4, 1),
    p(-8, -4),
    p(-21, 49),
    p(0, 0),
    p(-11, -4),
    p(-16, 2),
    p(0, 0),
    p(0, 0),
    p(-12, -0),
    p(-34, 5),
    p(-23, -38),
    p(0, 0),
    p(20, -62),
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
    p(-7, 8),    /*0b1001*/
    p(-0, 8),    /*0b1010*/
    p(6, 7),     /*0b1011*/
    p(-4, 3),    /*0b1100*/
    p(-19, 3),   /*0b1101*/
    p(-7, 1),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(3, 7),     /*0b10000*/
    p(5, 4),     /*0b10001*/
    p(20, 5),    /*0b10010*/
    p(1, 4),     /*0b10011*/
    p(-6, 2),    /*0b10100*/
    p(14, 10),   /*0b10101*/
    p(-24, -0),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(6, 9),     /*0b11000*/
    p(26, 9),    /*0b11001*/
    p(28, 23),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(9, -6),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(7, 1),     /*0b100000*/
    p(-2, 5),    /*0b100001*/
    p(14, 0),    /*0b100010*/
    p(6, -5),    /*0b100011*/
    p(-5, -3),   /*0b100100*/
    p(-21, -11), /*0b100101*/
    p(-17, 15),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(13, -2),   /*0b101000*/
    p(-5, 8),    /*0b101001*/
    p(9, -7),    /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-2, 0),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(2, 1),     /*0b110000*/
    p(13, 0),    /*0b110001*/
    p(19, -8),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(11, 14),   /*0b110100*/
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
    p(7, -7),    /*0b111111*/
    p(4, 6),     /*0b00*/
    p(5, -2),    /*0b01*/
    p(16, -2),   /*0b10*/
    p(-5, -25),  /*0b11*/
    p(22, 2),    /*0b100*/
    p(2, -0),    /*0b101*/
    p(30, -26),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(42, -6),   /*0b1000*/
    p(-7, -11),  /*0b1001*/
    p(13, -28),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(7, -10),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-36, 32),  /*0b1111*/
    p(5, 8),     /*0b00*/
    p(11, -4),   /*0b01*/
    p(11, -7),   /*0b10*/
    p(8, -31),   /*0b11*/
    p(11, -3),   /*0b100*/
    p(23, -11),  /*0b101*/
    p(1, -12),   /*0b110*/
    p(0, 0),     /*0b111*/
    p(15, 2),    /*0b1000*/
    p(23, -13),  /*0b1001*/
    p(28, -39),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(8, -17),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-2, -36),  /*0b1111*/
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
        p( -17,   19),    p( -18,   35),    p( -29,   28),    p( -28,   12),    p( -30,   12),    p( -24,    9),    p( -10,   11),    p( -33,   13),
        p( -13,   16),    p( -27,   37),    p( -28,   28),    p( -27,   14),    p( -42,   21),    p( -36,   18),    p( -27,   23),    p( -28,   11),
        p(  -7,   28),    p( -14,   29),    p( -24,   30),    p( -22,   30),    p( -32,   34),    p( -28,   34),    p( -29,   40),    p( -48,   38),
        p(   3,   42),    p(   1,   40),    p(   3,   33),    p(  -8,   42),    p( -23,   49),    p( -15,   53),    p( -18,   56),    p( -35,   56),
        p(  10,   49),    p(  25,   42),    p(  10,   37),    p(  -2,   29),    p(   4,   38),    p(  -6,   52),    p( -18,   53),    p( -42,   58),
        p(  21,   49),    p(  23,   48),    p(  26,   44),    p(  28,   39),    p(  28,   44),    p(  31,   54),    p(   5,   57),    p(   6,   58),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(-2, 4), p(5, 8), p(8, 18), p(13, 25), p(19, 51), p(24, 52)];
const UNSUPPORTED_PAWN: PhasedScore = p(-8, -5);
const DOUBLED_PAWN: PhasedScore = p(-7, -22);
const PHALANX: [PhasedScore; 6] = [p(-2, 1), p(4, 3), p(8, 6), p(20, 20), p(60, 60), p(67, 65)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(15, 14), p(7, 17), p(13, 20), p(6, 10), p(-2, 11), p(-46, 14)];
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
        p(13, 9),
        p(20, 19),
        p(29, 22),
        p(35, 25),
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
        p(-28, -32),
        p(-12, -21),
        p(-1, -11),
        p(10, -3),
        p(19, 6),
        p(27, 11),
        p(34, 15),
        p(37, 21),
        p(45, 24),
        p(49, 28),
        p(52, 34),
        p(42, 50),
        p(42, 47),
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
        p(-55, -6),
        p(-41, -3),
        p(-31, -2),
        p(-22, -1),
        p(-17, 3),
        p(-7, 8),
        p(-1, 13),
        p(6, 18),
        p(11, 24),
        p(16, 30),
        p(21, 34),
        p(22, 42),
        p(27, 47),
        p(30, 49),
        p(28, 51),
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
        p(-16, -58),
        p(-24, -44),
        p(-32, -0),
        p(-28, 9),
        p(-24, 21),
        p(-18, 22),
        p(-12, 29),
        p(-6, 33),
        p(0, 37),
        p(6, 37),
        p(11, 40),
        p(17, 43),
        p(23, 42),
        p(26, 46),
        p(31, 48),
        p(37, 51),
        p(40, 57),
        p(45, 58),
        p(54, 58),
        p(62, 57),
        p(64, 62),
        p(67, 67),
        p(68, 70),
        p(66, 68),
        p(70, 77),
        p(65, 72),
        p(67, 76),
        p(62, 64),
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
        p(1, 17),
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
        p(11, -10),
        p(9, 5),
        p(7, 7),
        p(5, 7),
        p(1, 5),
        p(-1, 4),
        p(-2, -0),
        p(3, -6),
        p(11, -18),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
    ],
    [
        p(20, -10),
        p(15, 3),
        p(8, 6),
        p(3, 7),
        p(-2, 6),
        p(-5, 5),
        p(-7, 2),
        p(-8, -0),
        p(-10, -5),
        p(-13, -6),
        p(3, -17),
        p(-10, -21),
        p(4, -45),
        p(65, -35),
        p(0, 0),
    ],
    [
        p(1, 6),
        p(-8, 23),
        p(-15, 31),
        p(-21, 33),
        p(-29, 35),
        p(-35, 36),
        p(-38, 34),
        p(-42, 33),
        p(-41, 29),
        p(-39, 25),
        p(-32, 20),
        p(-12, 14),
        p(4, 5),
        p(38, -2),
        p(63, -39),
    ],
    [
        p(19, 4),
        p(28, 30),
        p(32, 36),
        p(31, 41),
        p(28, 48),
        p(24, 54),
        p(21, 57),
        p(17, 59),
        p(14, 60),
        p(11, 59),
        p(11, 54),
        p(10, 51),
        p(12, 49),
        p(13, 43),
        p(38, 14),
    ],
    [
        p(-40, -27),
        p(-24, -5),
        p(-13, 5),
        p(-1, 8),
        p(17, 6),
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
    [p(-2, 13), p(18, 27), p(0, 0), p(41, 21), p(49, 63), p(0, 0)],
    [p(-9, 22), p(4, 23), p(12, 19), p(0, 0), p(52, 54), p(0, 0)],
    [p(-4, 18), p(-2, 21), p(-2, 31), p(-1, 17), p(0, 0), p(0, 0)],
    [p(55, 19), p(-13, 20), p(23, 11), p(-0, 15), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 8), p(7, 7), p(5, 9), p(11, 7), p(6, 16), p(3, 7)],
    [p(2, 9), p(10, 22), p(-61, -33), p(7, 13), p(8, 21), p(2, 5)],
    [p(-3, 8), p(7, 14), p(3, 18), p(7, 13), p(3, 34), p(12, -3)],
    [p(-0, 9), p(5, 12), p(5, 6), p(3, 18), p(-71, -68), p(0, -8)],
    [p(24, 3), p(12, 14), p(18, 9), p(6, 8), p(18, 1), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(1, -19), p(2, -12), p(0, -7), p(6, -23), p(2, -7), p(-17, -0)];
const DOUBLE_KINGZONE_ATTACK: PhasedScore = p(51, 3);
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(4, -2), p(-9, 8), p(-0, -8), p(-23, 14)];
const SAFE_CHECK: [PhasedScore; 5] = [p(0, 0), p(56, -7), p(11, 1), p(42, -4), p(27, 42)];
const CHECK_STM: PhasedScore = p(36, 14);
const SAFE_CHECK_STM: PhasedScore = p(35, 5);
const DISCOVERED_CHECK_STM: PhasedScore = p(65, 66);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(11, -2), p(63, 33), p(65, 22), p(67, 68), p(0, 0), p(55, -28)];
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
