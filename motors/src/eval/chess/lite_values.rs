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
        p(  20,   52),    p(  22,   51),    p(  25,   46),    p(  28,   41),    p(  27,   46),    p(  30,   56),    p(   2,   60),    p(   6,   60),
        p( -29,   21),    p( -22,   20),    p( -11,   10),    p( -13,   12),    p( -18,   10),    p(  25,    9),    p(  16,   29),    p(   7,   26),
        p( -42,   -1),    p( -33,   -6),    p( -36,  -13),    p( -13,   -7),    p(  -8,   -9),    p(  -8,  -19),    p(  -8,   -9),    p(  -9,  -10),
        p( -48,  -14),    p( -44,  -12),    p( -25,  -12),    p( -11,   -9),    p(  -6,  -10),    p(  -6,  -13),    p( -16,  -16),    p( -21,  -20),
        p( -57,  -15),    p( -47,  -18),    p( -30,  -10),    p( -18,   -8),    p( -18,   -7),    p( -16,  -10),    p( -12,  -26),    p( -24,  -23),
        p( -47,   -7),    p( -40,  -10),    p( -35,  -12),    p( -37,   -6),    p( -31,   -3),    p(  -5,  -14),    p(  10,  -25),    p( -13,  -22),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
    ],
    // knight
    [
        p( -62,  -45),    p( -65,    4),    p( -66,    9),    p( -48,   10),    p(  -8,   -2),    p( -64,   -1),    p( -57,  -12),    p( -62,  -57),
        p( -19,    6),    p(  -7,   12),    p(   5,    1),    p(   7,    4),    p(  12,   -2),    p(  21,  -10),    p(   1,    2),    p( -16,   -7),
        p(  -6,    3),    p(  12,   -1),    p(  17,    3),    p(  27,    4),    p(  38,    1),    p(  53,  -10),    p(   7,   -3),    p(   7,   -5),
        p(  14,    8),    p(  21,    4),    p(  34,    8),    p(  37,   12),    p(  34,   12),    p(  35,    8),    p(  26,    9),    p(  36,    0),
        p(   7,    7),    p(  18,    1),    p(  22,    8),    p(  29,   10),    p(  28,   11),    p(  34,   -1),    p(  31,   -2),    p(  23,    6),
        p( -15,   -5),    p(  -8,   -6),    p(   1,   -8),    p(  10,    4),    p(  14,    2),    p(  11,  -18),    p(  13,  -12),    p(   7,   -4),
        p( -22,    1),    p(  -8,    4),    p(  -6,   -1),    p(   3,    1),    p(  10,   -3),    p(   5,   -8),    p(   9,   -0),    p(   5,    8),
        p( -44,   -6),    p( -13,   -2),    p( -23,   -3),    p(  -2,    1),    p(   3,    1),    p(   3,   -9),    p(  -8,    1),    p( -19,   -1),
    ],
    // bishop
    [
        p( -25,   15),    p( -36,    8),    p( -60,    4),    p( -57,   11),    p( -62,    8),    p( -54,    5),    p( -30,    7),    p( -38,    5),
        p( -13,    3),    p( -10,    3),    p(  -0,    2),    p( -13,    2),    p(  -6,   -2),    p(   0,   -2),    p( -36,   11),    p( -23,    1),
        p(   1,    6),    p(   9,    2),    p(  -4,    5),    p(   7,   -5),    p(   7,    1),    p(  37,    0),    p(  26,    2),    p(  17,   11),
        p(  -6,    6),    p(   3,    2),    p(  10,   -2),    p(  12,    4),    p(  11,    1),    p(  13,    1),    p(  17,   -2),    p(  -2,    2),
        p(  -5,    2),    p(  -8,    3),    p(  -2,    2),    p(  11,   -0),    p(   8,   -0),    p(  10,   -6),    p(  -3,   -2),    p(  21,   -9),
        p(  -3,    2),    p(   3,   -0),    p(   2,    2),    p(   1,    2),    p(   8,    1),    p(   5,   -5),    p(  13,  -10),    p(  13,   -3),
        p(  10,   -2),    p(   1,   -2),    p(  10,   -3),    p(   3,    4),    p(   4,    2),    p(  12,    0),    p(  18,   -6),    p(  15,   -3),
        p(   5,   -2),    p(  13,   -1),    p(   6,    5),    p(  -3,    4),    p(  10,    4),    p(  -7,    8),    p(   6,    2),    p(  11,   -8),
    ],
    // rook
    [
        p( -35,   46),    p( -37,   48),    p( -45,   53),    p( -45,   51),    p( -35,   47),    p( -20,   48),    p( -33,   51),    p(  -4,   34),
        p( -31,   45),    p( -29,   48),    p( -21,   47),    p(  -4,   38),    p( -17,   40),    p(  10,   36),    p(  10,   36),    p(  13,   27),
        p( -32,   40),    p( -11,   33),    p( -17,   35),    p( -17,   29),    p(   7,   22),    p(  22,   18),    p(  26,   23),    p(  -1,   24),
        p( -33,   40),    p( -23,   35),    p( -23,   36),    p( -19,   31),    p( -13,   27),    p(   1,   24),    p(  -5,   29),    p( -12,   26),
        p( -44,   37),    p( -40,   34),    p( -39,   36),    p( -32,   31),    p( -26,   29),    p( -28,   29),    p( -13,   26),    p( -29,   25),
        p( -47,   32),    p( -41,   27),    p( -41,   29),    p( -38,   29),    p( -29,   22),    p( -20,   17),    p(  -6,   11),    p( -21,   16),
        p( -47,   29),    p( -41,   27),    p( -34,   27),    p( -29,   23),    p( -22,   20),    p( -10,   13),    p(   0,    8),    p( -37,   21),
        p( -40,   33),    p( -40,   27),    p( -41,   31),    p( -36,   26),    p( -29,   20),    p( -27,   24),    p( -40,   31),    p( -43,   25),
    ],
    // queen
    [
        p( -13,   60),    p(  -6,   57),    p(   8,   63),    p(  31,   58),    p(  36,   60),    p(  55,   50),    p(  69,   16),    p(  19,   40),
        p(  13,   47),    p(   7,   53),    p(  13,   60),    p(  12,   70),    p(  18,   71),    p(  44,   64),    p(  28,   60),    p(  59,   39),
        p(  33,   38),    p(  29,   40),    p(  27,   56),    p(  26,   60),    p(  30,   63),    p(  61,   61),    p(  66,   48),    p(  59,   51),
        p(  22,   45),    p(  26,   51),    p(  24,   49),    p(  16,   61),    p(  23,   60),    p(  40,   54),    p(  41,   64),    p(  47,   51),
        p(  18,   47),    p(  16,   48),    p(  16,   51),    p(  19,   58),    p(  24,   56),    p(  28,   53),    p(  34,   53),    p(  38,   48),
        p(  18,   31),    p(  22,   37),    p(  17,   48),    p(  16,   52),    p(  20,   58),    p(  31,   42),    p(  37,   37),    p(  35,   27),
        p(  20,   26),    p(  17,   35),    p(  21,   38),    p(  22,   48),    p(  23,   49),    p(  26,   30),    p(  33,   14),    p(  23,   21),
        p(   4,   36),    p(  14,   25),    p(  14,   33),    p(  18,   38),    p(  20,   32),    p(   6,   37),    p(   1,   21),    p(  12,    5),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  69,    8),    p(  69,    6),    p(  73,   -2),    p(  70,  -41),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  55,   16),    p(  32,   25),    p(  29,   18),    p(   6,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  24,   25),    p(  37,   19),    p(  29,   12),    p(  -7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -37,   22),    p( -21,   16),    p( -25,    4),    p( -44,    8),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -57,   18),    p( -38,    6),    p( -44,   -4),    p( -61,    1),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -45,    8),    p( -44,    1),    p( -23,  -11),    p( -36,    2),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -43,    1),    p( -31,   -5),    p(  -9,  -18),    p(  12,   -6),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  10,   -4),    p(  -1,   -1),    p(  17,  -13),    p(  26,  -16),
    ],
];

const MORE_MINORS_NO_PAWNS: PhasedScore = p(-66, -65);
const BISHOP_PAIR: PhasedScore = p(24, 47);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(17, 16), p(17, 13), p(16, 3), p(12, -4), p(7, -10), p(3, -19), p(-4, -26), p(-10, -38), p(-22, -41)];
const ROOK_OPEN_FILE: PhasedScore = p(13, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(4, -0);
const KING_OPEN_FILE: PhasedScore = p(-25, 4);
const KING_CLOSED_FILE: PhasedScore = p(12, -6);
const KING_SEMIOPEN_FILE: PhasedScore = p(-2, 10);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-2, 1), p(3, 3), p(4, 0), p(4, 0), p(5, 2), p(6, 3), p(12, -1), p(22, -4)],
    // Closed
    [p(0, 0), p(0, 0), p(15, -18), p(-16, 8), p(3, 6), p(2, 1), p(4, 3), p(3, -1)],
    // SemiOpen
    [p(0, 0), p(-4, 22), p(10, 14), p(2, 8), p(3, 7), p(6, 2), p(6, -2), p(13, 1)],
    // SemiClosed
    [p(0, 0), p(13, -15), p(12, 1), p(3, -3), p(8, -2), p(3, 0), p(7, 0), p(6, -1)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(18, 10),
    p(1, 3),
    p(1, 4),
    p(-6, 4),
    p(5, 2),
    p(-7, -12),
    p(-3, -3),
    p(-3, -14),
    p(-2, -1),
    p(-12, -3),
    p(-8, -17),
    p(-15, -10),
    p(6, -8),
    p(0, -14),
    p(6, -9),
    p(6, 8),
    p(-7, -2),
    p(-20, -6),
    p(-11, 0),
    p(-34, 17),
    p(-16, 2),
    p(-14, -21),
    p(8, 22),
    p(-35, 19),
    p(-18, -17),
    p(-20, -18),
    p(-32, -30),
    p(-37, 11),
    p(-12, -3),
    p(17, -9),
    p(-65, 67),
    p(0, 0),
    p(-0, -5),
    p(-12, -8),
    p(1, -9),
    p(-17, -4),
    p(-18, -3),
    p(-40, -22),
    p(-23, 32),
    p(-28, 20),
    p(-6, -7),
    p(-17, -13),
    p(9, -14),
    p(-12, 26),
    p(-42, 14),
    p(2, -33),
    p(0, 0),
    p(0, 0),
    p(1, -13),
    p(-8, 2),
    p(3, -51),
    p(0, 0),
    p(18, -16),
    p(-30, -16),
    p(0, 0),
    p(0, 0),
    p(-23, -8),
    p(-15, -18),
    p(-9, 13),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(23, 9),
    p(4, 1),
    p(-4, 3),
    p(-18, -3),
    p(10, -4),
    p(-23, -11),
    p(-11, -6),
    p(-29, -18),
    p(8, -0),
    p(-7, -10),
    p(-26, -9),
    p(-39, -5),
    p(-0, -7),
    p(-35, -11),
    p(-27, -12),
    p(-42, 47),
    p(8, 2),
    p(-1, -7),
    p(-3, -11),
    p(-20, -2),
    p(-8, -2),
    p(-13, -16),
    p(-18, -5),
    p(-22, 65),
    p(-6, -10),
    p(-23, -16),
    p(-30, -30),
    p(5, -66),
    p(-10, -11),
    p(-7, -24),
    p(-67, 52),
    p(0, 0),
    p(10, 3),
    p(-3, -3),
    p(-15, -7),
    p(-23, -9),
    p(-1, -0),
    p(-25, -20),
    p(-11, -6),
    p(-22, -7),
    p(-1, -6),
    p(-19, -10),
    p(-23, -17),
    p(-32, -10),
    p(-4, -4),
    p(-44, -11),
    p(14, 5),
    p(-50, 48),
    p(4, 1),
    p(-8, -4),
    p(-22, 49),
    p(0, 0),
    p(-9, -6),
    p(-14, -1),
    p(0, 0),
    p(0, 0),
    p(-11, -1),
    p(-32, 3),
    p(-22, -44),
    p(0, 0),
    p(23, -64),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-12, 8),   /*0b0000*/
    p(-13, 6),   /*0b0001*/
    p(-3, 9),    /*0b0010*/
    p(-2, 9),    /*0b0011*/
    p(-7, 2),    /*0b0100*/
    p(-21, -1),  /*0b0101*/
    p(-7, 2),    /*0b0110*/
    p(-9, -12),  /*0b0111*/
    p(2, 3),     /*0b1000*/
    p(-8, 7),    /*0b1001*/
    p(1, 8),     /*0b1010*/
    p(6, 8),     /*0b1011*/
    p(-2, 2),    /*0b1100*/
    p(-15, 2),   /*0b1101*/
    p(-7, 2),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(2, 6),     /*0b10000*/
    p(5, 4),     /*0b10001*/
    p(21, 4),    /*0b10010*/
    p(2, 6),     /*0b10011*/
    p(-7, 1),    /*0b10100*/
    p(14, 9),    /*0b10101*/
    p(-25, 0),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(8, 7),     /*0b11000*/
    p(27, 8),    /*0b11001*/
    p(29, 23),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(10, -6),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(8, 0),     /*0b100000*/
    p(0, 5),     /*0b100001*/
    p(15, 1),    /*0b100010*/
    p(7, -2),    /*0b100011*/
    p(-5, -4),   /*0b100100*/
    p(-18, -12), /*0b100101*/
    p(-18, 16),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(16, -5),   /*0b101000*/
    p(0, 6),     /*0b101001*/
    p(11, -7),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-1, -2),   /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(3, -1),    /*0b110000*/
    p(15, 1),    /*0b110001*/
    p(20, -8),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(11, 12),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(19, -8),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(6, -5),    /*0b111111*/
    p(-3, 7),    /*0b00*/
    p(-1, -2),   /*0b01*/
    p(17, -1),   /*0b10*/
    p(-9, -24),  /*0b11*/
    p(16, 3),    /*0b100*/
    p(-7, -1),   /*0b101*/
    p(28, -24),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(39, -5),   /*0b1000*/
    p(-10, -10), /*0b1001*/
    p(10, -28),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(3, -10),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-39, 28),  /*0b1111*/
    p(0, 8),     /*0b00*/
    p(10, -4),   /*0b01*/
    p(6, -8),    /*0b10*/
    p(4, -30),   /*0b11*/
    p(10, -4),   /*0b100*/
    p(22, -11),  /*0b101*/
    p(-2, -12),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(11, 2),    /*0b1000*/
    p(20, -10),  /*0b1001*/
    p(21, -37),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(6, -17),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-8, -35),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(1, -35);
const STOPPABLE_PASSER: PhasedScore = p(14, -39);
const CLOSE_KING_PASSER: PhasedScore = p(-2, 26);
const IMMOBILE_PASSER: PhasedScore = p(-8, -34);
const PROTECTED_PASSER: PhasedScore = p(7, -0);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -24,   36),    p( -25,   52),    p( -35,   44),    p( -36,   27),    p( -37,   27),    p( -29,   25),    p( -16,   28),    p( -39,   30),
        p( -19,   33),    p( -31,   53),    p( -33,   43),    p( -32,   29),    p( -47,   36),    p( -42,   33),    p( -33,   40),    p( -33,   28),
        p( -12,   44),    p( -20,   43),    p( -30,   43),    p( -26,   41),    p( -36,   45),    p( -32,   46),    p( -33,   53),    p( -54,   53),
        p(   1,   53),    p(  -2,   51),    p(   0,   42),    p( -10,   51),    p( -25,   58),    p( -17,   61),    p( -17,   64),    p( -37,   65),
        p(   9,   55),    p(  21,   49),    p(   7,   43),    p(  -4,   32),    p(   1,   43),    p(  -6,   55),    p( -18,   56),    p( -41,   61),
        p(  20,   52),    p(  22,   51),    p(  25,   46),    p(  28,   41),    p(  27,   46),    p(  30,   56),    p(   2,   60),    p(   6,   60),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(-1, 4), p(4, 8), p(7, 18), p(12, 28), p(19, 55), p(23, 55)];
const UNSUPPORTED_PAWN: PhasedScore = p(-9, -4);
const DOUBLED_PAWN: PhasedScore = p(-7, -23);
const PHALANX: [PhasedScore; 6] = [p(-3, 1), p(4, 4), p(8, 6), p(21, 19), p(59, 59), p(67, 64)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(15, 14), p(7, 17), p(14, 20), p(6, 10), p(-3, 12), p(-46, 12)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(48, 21), p(50, 42), p(58, 8), p(48, -3), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, -4), p(14, 20), p(18, -7), p(16, 9), p(16, -12), p(28, -11)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-40, -59),
        p(-18, -34),
        p(-6, -12),
        p(4, -0),
        p(11, 9),
        p(18, 19),
        p(26, 21),
        p(32, 23),
        p(35, 22),
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
        p(-30, -47),
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
        p(-48, -18),
        p(-40, -3),
        p(-36, 5),
        p(-33, 9),
        p(-34, 16),
        p(-30, 22),
        p(-27, 27),
        p(-25, 32),
        p(-22, 37),
        p(-20, 41),
        p(-16, 44),
        p(-16, 49),
        p(-10, 50),
        p(-1, 49),
        p(3, 46),
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
        p(3, -64),
        p(4, -62),
        p(0, -25),
        p(4, -9),
        p(6, 11),
        p(10, 17),
        p(13, 28),
        p(16, 36),
        p(19, 42),
        p(22, 44),
        p(24, 49),
        p(28, 53),
        p(31, 53),
        p(33, 57),
        p(35, 58),
        p(40, 60),
        p(43, 63),
        p(46, 63),
        p(55, 60),
        p(62, 58),
        p(64, 60),
        p(67, 62),
        p(67, 65),
        p(65, 60),
        p(66, 73),
        p(63, 63),
        p(63, 67),
        p(61, 49),
    ],
    [
        p(-24, -22),
        p(-17, -23),
        p(-10, -16),
        p(-3, -7),
        p(7, -1),
        p(11, 5),
        p(21, 14),
        p(29, 20),
        p(56, 16),
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
    [p(-4, 13), p(0, 0), p(28, 24), p(55, 8), p(37, -5), p(0, 0)],
    [p(-1, 11), p(21, 24), p(0, 0), p(42, 20), p(50, 63), p(0, 0)],
    [p(-6, 19), p(8, 19), p(16, 14), p(0, 0), p(56, 51), p(0, 0)],
    [p(-3, 17), p(-1, 19), p(-0, 29), p(-1, 18), p(0, 0), p(0, 0)],
    [p(51, 22), p(-17, 22), p(10, 16), p(-17, 16), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 8), p(8, 7), p(6, 10), p(12, 7), p(8, 17), p(4, 7)],
    [p(3, 8), p(12, 19), p(-61, -32), p(9, 12), p(10, 20), p(3, 6)],
    [p(-3, 7), p(8, 11), p(4, 16), p(7, 12), p(4, 34), p(11, -3)],
    [p(-0, 9), p(7, 11), p(5, 6), p(3, 17), p(-72, -69), p(0, -8)],
    [p(25, 4), p(12, 16), p(18, 10), p(5, 11), p(16, 6), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(9, -20), p(18, -8), p(12, -2), p(19, -13), p(12, 8), p(3, 0)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(3, -2), p(-10, 9), p(7, -7), p(-26, 15)];
const SAFE_CHECK: [PhasedScore; 5] = [p(0, 0), p(57, -7), p(10, 0), p(36, -5), p(30, 42)];
const CHECK_STM: PhasedScore = p(35, 14);
const SAFE_CHECK_STM: PhasedScore = p(36, 4);
const DISCOVERED_CHECK_STM: PhasedScore = p(65, 66);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(11, -5), p(64, 29), p(65, 18), p(67, 67), p(0, 0), p(46, -25)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(9, -13), p(32, 33), p(25, 36), p(43, 13), p(58, 2)];
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
