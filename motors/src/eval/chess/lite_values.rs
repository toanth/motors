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
        p(  20,   53),    p(  22,   52),    p(  25,   47),    p(  28,   43),    p(  27,   47),    p(  30,   57),    p(   2,   61),    p(   6,   60),
        p( -28,   22),    p( -24,   20),    p( -12,   10),    p( -14,   13),    p( -20,   11),    p(  23,   10),    p(  13,   31),    p(   5,   27),
        p( -40,   -1),    p( -32,   -5),    p( -35,  -13),    p( -12,   -7),    p(  -8,   -9),    p(  -9,  -19),    p(  -8,   -8),    p( -10,   -9),
        p( -47,  -14),    p( -44,  -12),    p( -24,  -11),    p( -12,   -9),    p(  -6,  -10),    p(  -7,  -13),    p( -17,  -16),    p( -23,  -20),
        p( -56,  -15),    p( -47,  -18),    p( -28,  -10),    p( -18,   -8),    p( -17,   -8),    p( -14,  -10),    p( -12,  -26),    p( -25,  -22),
        p( -47,   -7),    p( -42,  -10),    p( -36,  -11),    p( -38,   -6),    p( -32,   -4),    p(  -6,  -15),    p(   8,  -25),    p( -15,  -22),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
    ],
    // knight
    [
        p( -63,  -44),    p( -66,    8),    p( -67,   12),    p( -47,   14),    p(  -4,    0),    p( -64,    2),    p( -56,   -9),    p( -63,  -56),
        p( -19,    9),    p(  -6,   16),    p(   9,    4),    p(  11,    6),    p(  12,   -0),    p(  23,   -7),    p(   4,    5),    p( -15,   -5),
        p(  -5,    6),    p(  18,    0),    p(  27,    4),    p(  41,    4),    p(  45,    1),    p(  55,  -10),    p(   2,   -1),    p(   3,   -2),
        p(  12,   12),    p(  20,    7),    p(  39,    9),    p(  41,   15),    p(  40,   13),    p(  42,    8),    p(  26,   10),    p(  36,    3),
        p(  10,   13),    p(  23,    6),    p(  30,   13),    p(  33,   15),    p(  37,   16),    p(  43,    4),    p(  39,    3),    p(  26,   13),
        p( -20,   -1),    p( -10,   -3),    p(   0,   -4),    p(   8,    7),    p(  12,    6),    p(  10,  -14),    p(  10,   -8),    p(   2,   -0),
        p( -28,    5),    p( -14,    8),    p( -11,    3),    p(  -2,    5),    p(   5,    0),    p(   1,   -5),    p(   0,    3),    p(  -3,   12),
        p( -50,   -3),    p( -22,    3),    p( -29,    0),    p(  -9,    5),    p(  -4,    5),    p(  -2,   -6),    p( -15,    5),    p( -23,    1),
    ],
    // bishop
    [
        p( -26,   17),    p( -36,    9),    p( -60,    5),    p( -56,   12),    p( -61,    8),    p( -53,    6),    p( -30,    8),    p( -39,    7),
        p( -16,    5),    p(  -9,    4),    p(   1,    2),    p( -12,    2),    p(  -4,   -3),    p(  -4,   -1),    p( -36,   12),    p( -24,    3),
        p(   2,    6),    p(   7,    3),    p(  -2,    5),    p(   7,   -5),    p(   3,    1),    p(  30,    1),    p(  18,    3),    p(  11,   11),
        p(  -5,    7),    p(   4,    3),    p(   8,   -1),    p(  11,    5),    p(  14,    1),    p(  14,    1),    p(  19,   -1),    p(  -2,    3),
        p(   1,    2),    p(  -2,    3),    p(   5,    1),    p(  14,    0),    p(  15,   -0),    p(  19,   -7),    p(   8,   -4),    p(  28,   -9),
        p(  -3,    3),    p(   1,    1),    p(   1,    4),    p(  -0,    3),    p(   6,    3),    p(   4,   -3),    p(  11,   -8),    p(  13,   -2),
        p(   7,   -0),    p(   0,   -0),    p(   6,   -2),    p(  -0,    6),    p(   2,    3),    p(   8,    1),    p(  17,   -5),    p(  13,   -1),
        p(   2,   -0),    p(   9,    1),    p(   2,    7),    p(  -7,    5),    p(   5,    6),    p(  -8,    8),    p(   4,    3),    p(  11,   -6),
    ],
    // rook
    [
        p( -38,   43),    p( -41,   45),    p( -51,   50),    p( -53,   49),    p( -44,   44),    p( -28,   46),    p( -36,   46),    p(  -8,   31),
        p( -23,   39),    p( -22,   42),    p( -13,   41),    p(   5,   32),    p(  -8,   34),    p(  17,   30),    p(  14,   29),    p(  14,   21),
        p( -27,   37),    p(  -8,   30),    p( -12,   32),    p( -10,   25),    p(  15,   18),    p(  29,   14),    p(  31,   18),    p(   0,   21),
        p( -30,   36),    p( -20,   32),    p( -20,   33),    p( -13,   28),    p(  -7,   24),    p(   4,   21),    p(  -4,   25),    p( -10,   22),
        p( -37,   34),    p( -31,   32),    p( -30,   33),    p( -26,   29),    p( -17,   27),    p( -18,   27),    p(  -2,   21),    p( -23,   21),
        p( -44,   29),    p( -37,   24),    p( -38,   25),    p( -35,   25),    p( -26,   19),    p( -17,   14),    p(  -3,    7),    p( -19,   12),
        p( -43,   26),    p( -36,   24),    p( -30,   24),    p( -25,   20),    p( -19,   16),    p(  -8,   10),    p(   1,    4),    p( -34,   18),
        p( -35,   29),    p( -36,   24),    p( -37,   28),    p( -32,   24),    p( -25,   18),    p( -23,   21),    p( -37,   27),    p( -39,   22),
    ],
    // queen
    [
        p( -24,   54),    p( -21,   52),    p( -10,   60),    p(  11,   53),    p(  17,   52),    p(  40,   41),    p(  64,    4),    p(  10,   30),
        p(  12,   40),    p(  13,   43),    p(  21,   48),    p(  18,   62),    p(  15,   69),    p(  41,   54),    p(  29,   50),    p(  57,   27),
        p(  30,   32),    p(  29,   34),    p(  30,   49),    p(  28,   54),    p(  28,   58),    p(  51,   52),    p(  59,   36),    p(  44,   42),
        p(  19,   42),    p(  24,   49),    p(  22,   48),    p(  14,   62),    p(  22,   58),    p(  38,   51),    p(  36,   61),    p(  40,   47),
        p(  16,   44),    p(  17,   46),    p(  16,   49),    p(  17,   57),    p(  26,   55),    p(  30,   51),    p(  37,   48),    p(  38,   43),
        p(  11,   32),    p(  15,   37),    p(  11,   49),    p(  10,   53),    p(  13,   60),    p(  25,   43),    p(  30,   37),    p(  30,   22),
        p(  13,   23),    p(  10,   33),    p(  15,   37),    p(  15,   46),    p(  16,   46),    p(  18,   30),    p(  25,   13),    p(  17,   16),
        p(  -4,   32),    p(   6,   26),    p(   6,   33),    p(  10,   36),    p(  12,   31),    p(   1,   34),    p(  -3,   15),    p(   6,   -0),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  69,    8),    p(  69,    4),    p(  71,   -3),    p(  67,  -38),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  51,   16),    p(  25,   26),    p(  19,   19),    p(   7,    8),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  15,   26),    p(  30,   19),    p(  20,   13),    p(  -4,   12),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -42,   24),    p( -26,   17),    p( -30,    6),    p( -41,    8),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -55,   18),    p( -35,    6),    p( -38,   -5),    p( -58,   -0),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -46,    9),    p( -43,    2),    p( -23,  -10),    p( -35,    1),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -47,    3),    p( -35,   -2),    p( -13,  -16),    p(   9,   -7),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   9,   -3),    p(   1,   -1),    p(  20,  -14),    p(  28,  -17),
    ],
];

const MORE_MINORS_NO_PAWNS: PhasedScore = p(-66, -65);
const BISHOP_PAIR: PhasedScore = p(24, 47);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(16, 19), p(17, 15), p(16, 5), p(12, -2), p(7, -9), p(3, -17), p(-3, -25), p(-10, -37), p(-21, -39)];
const ROOK_OPEN_FILE: PhasedScore = p(14, 3);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -2);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(4, -1);
const KING_OPEN_FILE: PhasedScore = p(-25, 4);
const KING_CLOSED_FILE: PhasedScore = p(12, -7);
const KING_SEMIOPEN_FILE: PhasedScore = p(-2, 9);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-5, 3), p(-1, 5), p(1, 2), p(4, 2), p(5, 3), p(7, 4), p(13, 0), p(22, -3)],
    // Closed
    [p(0, 0), p(0, 0), p(11, -18), p(-14, 8), p(3, 8), p(2, 3), p(5, 4), p(3, 1)],
    // SemiOpen
    [p(0, 0), p(-10, 24), p(11, 15), p(0, 9), p(2, 8), p(6, 3), p(8, -1), p(13, 2)],
    // SemiClosed
    [p(0, 0), p(9, -12), p(10, 3), p(3, -1), p(9, -0), p(3, 2), p(9, 1), p(6, -1)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(18, 10),
    p(1, 2),
    p(1, 4),
    p(-7, 3),
    p(4, 2),
    p(-7, -13),
    p(-3, -3),
    p(-2, -14),
    p(-2, -2),
    p(-13, -4),
    p(-8, -18),
    p(-16, -10),
    p(4, -8),
    p(-1, -14),
    p(5, -8),
    p(5, 12),
    p(-6, -2),
    p(-20, -7),
    p(-11, 0),
    p(-34, 17),
    p(-16, 2),
    p(-14, -22),
    p(9, 21),
    p(-36, 19),
    p(-17, -17),
    p(-20, -18),
    p(-30, -31),
    p(-39, 13),
    p(-14, -3),
    p(15, -9),
    p(-63, 68),
    p(0, 0),
    p(0, -4),
    p(-11, -8),
    p(2, -8),
    p(-18, -4),
    p(-19, -3),
    p(-41, -22),
    p(-22, 33),
    p(-26, 21),
    p(-6, -7),
    p(-18, -13),
    p(8, -15),
    p(-14, 27),
    p(-43, 13),
    p(-0, -31),
    p(0, 0),
    p(0, 0),
    p(3, -12),
    p(-5, 3),
    p(6, -53),
    p(0, 0),
    p(17, -16),
    p(-28, -19),
    p(0, 0),
    p(0, 0),
    p(-23, -7),
    p(-15, -16),
    p(-6, 11),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(22, 10),
    p(2, 1),
    p(-3, 3),
    p(-18, -4),
    p(10, -4),
    p(-23, -11),
    p(-10, -6),
    p(-29, -19),
    p(7, -1),
    p(-8, -10),
    p(-26, -9),
    p(-39, -5),
    p(-1, -7),
    p(-35, -11),
    p(-26, -13),
    p(-43, 46),
    p(8, 3),
    p(-1, -7),
    p(-3, -10),
    p(-19, -3),
    p(-7, -2),
    p(-14, -17),
    p(-19, -4),
    p(-23, 65),
    p(-7, -9),
    p(-25, -16),
    p(-32, -30),
    p(0, -65),
    p(-11, -11),
    p(-10, -24),
    p(-71, 53),
    p(0, 0),
    p(11, 3),
    p(-3, -3),
    p(-14, -7),
    p(-22, -9),
    p(-1, 0),
    p(-26, -20),
    p(-12, -5),
    p(-25, -4),
    p(-1, -7),
    p(-19, -11),
    p(-23, -18),
    p(-32, -11),
    p(-6, -4),
    p(-45, -13),
    p(12, 6),
    p(-50, 45),
    p(5, 1),
    p(-8, -5),
    p(-21, 49),
    p(0, 0),
    p(-11, -5),
    p(-16, 0),
    p(0, 0),
    p(0, 0),
    p(-12, -1),
    p(-35, 3),
    p(-24, -39),
    p(0, 0),
    p(21, -66),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-9, 8),    /*0b0000*/
    p(-13, 6),   /*0b0001*/
    p(-2, 8),    /*0b0010*/
    p(-3, 8),    /*0b0011*/
    p(-7, 2),    /*0b0100*/
    p(-23, -1),  /*0b0101*/
    p(-8, 2),    /*0b0110*/
    p(-10, -14), /*0b0111*/
    p(2, 4),     /*0b1000*/
    p(-6, 7),    /*0b1001*/
    p(0, 8),     /*0b1010*/
    p(5, 8),     /*0b1011*/
    p(-4, 2),    /*0b1100*/
    p(-19, 2),   /*0b1101*/
    p(-8, 1),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(3, 7),     /*0b10000*/
    p(5, 4),     /*0b10001*/
    p(21, 4),    /*0b10010*/
    p(-1, 5),    /*0b10011*/
    p(-6, 1),    /*0b10100*/
    p(13, 10),   /*0b10101*/
    p(-25, 0),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(6, 9),     /*0b11000*/
    p(26, 8),    /*0b11001*/
    p(28, 22),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(9, -6),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(8, 1),     /*0b100000*/
    p(-2, 5),    /*0b100001*/
    p(14, 0),    /*0b100010*/
    p(5, -4),    /*0b100011*/
    p(-5, -4),   /*0b100100*/
    p(-21, -12), /*0b100101*/
    p(-18, 16),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(14, -3),   /*0b101000*/
    p(-4, 6),    /*0b101001*/
    p(10, -7),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-2, -1),   /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(2, 1),     /*0b110000*/
    p(13, 0),    /*0b110001*/
    p(19, -8),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(10, 13),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(15, -5),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(6, -7),    /*0b111111*/
    p(5, 6),     /*0b00*/
    p(7, -2),    /*0b01*/
    p(20, -1),   /*0b10*/
    p(-3, -24),  /*0b11*/
    p(24, 2),    /*0b100*/
    p(4, -2),    /*0b101*/
    p(34, -26),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(47, -6),   /*0b1000*/
    p(-3, -11),  /*0b1001*/
    p(19, -30),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(8, -9),    /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-30, 28),  /*0b1111*/
    p(8, 8),     /*0b00*/
    p(15, -5),   /*0b01*/
    p(13, -7),   /*0b10*/
    p(11, -30),  /*0b11*/
    p(15, -4),   /*0b100*/
    p(27, -13),  /*0b101*/
    p(4, -12),   /*0b110*/
    p(0, 0),     /*0b111*/
    p(19, 2),    /*0b1000*/
    p(26, -12),  /*0b1001*/
    p(30, -37),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(11, -17),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(1, -35),   /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(-4, -34);
const STOPPABLE_PASSER: PhasedScore = p(13, -39);
const CLOSE_KING_PASSER: PhasedScore = p(-2, 25);
const IMMOBILE_PASSER: PhasedScore = p(-6, -33);
const PROTECTED_PASSER: PhasedScore = p(6, -0);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -23,   36),    p( -23,   51),    p( -33,   44),    p( -33,   27),    p( -35,   27),    p( -28,   25),    p( -15,   28),    p( -38,   31),
        p( -19,   33),    p( -31,   53),    p( -33,   43),    p( -31,   29),    p( -46,   36),    p( -41,   33),    p( -32,   40),    p( -33,   28),
        p( -12,   44),    p( -19,   44),    p( -28,   43),    p( -25,   41),    p( -35,   46),    p( -30,   46),    p( -31,   54),    p( -52,   53),
        p(   0,   53),    p(  -2,   51),    p(   0,   42),    p( -10,   51),    p( -25,   58),    p( -15,   61),    p( -16,   64),    p( -36,   64),
        p(   9,   55),    p(  22,   49),    p(   8,   44),    p(  -4,   33),    p(   2,   44),    p(  -5,   56),    p( -16,   57),    p( -39,   61),
        p(  20,   53),    p(  22,   52),    p(  25,   47),    p(  28,   43),    p(  27,   47),    p(  30,   57),    p(   2,   61),    p(   6,   60),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(-2, 5), p(4, 9), p(7, 18), p(11, 29), p(19, 55), p(23, 55)];
const UNSUPPORTED_PAWN: PhasedScore = p(-8, -4);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);
const PHALANX: [PhasedScore; 6] = [p(-2, 1), p(4, 4), p(8, 6), p(20, 19), p(59, 59), p(67, 64)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(15, 14), p(7, 16), p(13, 20), p(7, 9), p(-2, 11), p(-45, 12)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(50, 22), p(52, 42), p(60, 8), p(50, -3), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(2, -2), p(17, 16), p(20, -4), p(17, 10), p(15, -4), p(33, -10)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-42, -55),
        p(-20, -34),
        p(-7, -14),
        p(4, -3),
        p(13, 8),
        p(21, 19),
        p(29, 23),
        p(36, 27),
        p(37, 30),
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
        p(-45, -43),
        p(-28, -36),
        p(-12, -24),
        p(-1, -13),
        p(10, -4),
        p(19, 5),
        p(27, 11),
        p(34, 16),
        p(37, 23),
        p(45, 27),
        p(49, 31),
        p(52, 38),
        p(44, 53),
        p(45, 53),
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
        p(-55, -13),
        p(-41, -9),
        p(-31, -8),
        p(-22, -7),
        p(-17, -2),
        p(-7, 4),
        p(-1, 10),
        p(6, 15),
        p(11, 22),
        p(16, 28),
        p(22, 33),
        p(24, 41),
        p(29, 46),
        p(33, 49),
        p(30, 52),
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
        p(-18, -61),
        p(-26, -50),
        p(-34, -8),
        p(-30, 2),
        p(-26, 15),
        p(-19, 15),
        p(-13, 23),
        p(-7, 27),
        p(-1, 31),
        p(5, 32),
        p(10, 35),
        p(16, 39),
        p(22, 39),
        p(26, 43),
        p(30, 45),
        p(36, 49),
        p(40, 55),
        p(45, 57),
        p(54, 57),
        p(62, 57),
        p(64, 62),
        p(67, 67),
        p(68, 70),
        p(66, 69),
        p(70, 77),
        p(66, 73),
        p(68, 77),
        p(63, 66),
    ],
    [
        p(5, 2),
        p(1, -15),
        p(1, -18),
        p(1, -12),
        p(-0, -6),
        p(-7, 2),
        p(-2, 12),
        p(-5, 21),
        p(9, 23),
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
        p(9, 4),
        p(8, 6),
        p(5, 5),
        p(1, 2),
        p(-1, 1),
        p(-3, -4),
        p(3, -10),
        p(8, -21),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
    ],
    [
        p(21, -8),
        p(15, 4),
        p(9, 6),
        p(3, 6),
        p(-2, 5),
        p(-5, 3),
        p(-7, -0),
        p(-9, -2),
        p(-11, -8),
        p(-16, -8),
        p(-0, -19),
        p(-25, -22),
        p(-7, -45),
        p(65, -37),
        p(0, 0),
    ],
    [
        p(1, 7),
        p(-7, 24),
        p(-14, 31),
        p(-21, 34),
        p(-29, 35),
        p(-35, 35),
        p(-38, 32),
        p(-42, 30),
        p(-42, 26),
        p(-40, 21),
        p(-32, 14),
        p(-13, 8),
        p(1, -2),
        p(37, -11),
        p(63, -49),
    ],
    [
        p(20, 9),
        p(29, 33),
        p(33, 39),
        p(32, 43),
        p(29, 50),
        p(25, 55),
        p(21, 57),
        p(17, 58),
        p(13, 59),
        p(11, 58),
        p(11, 51),
        p(9, 48),
        p(11, 45),
        p(12, 38),
        p(38, 7),
    ],
    [
        p(-39, -22),
        p(-24, -2),
        p(-14, 7),
        p(-3, 9),
        p(15, 5),
        p(32, -1),
        p(36, -0),
        p(52, -7),
        p(72, -22),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
    ],
];
const THREATS: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(-5, 12), p(0, 0), p(25, 25), p(53, 8), p(34, -5), p(0, 0)],
    [p(-2, 12), p(18, 27), p(0, 0), p(40, 21), p(48, 63), p(0, 0)],
    [p(-9, 20), p(5, 21), p(12, 17), p(0, 0), p(51, 53), p(0, 0)],
    [p(-3, 16), p(-2, 19), p(-2, 30), p(-1, 17), p(0, 0), p(0, 0)],
    [p(51, 22), p(-11, 20), p(20, 14), p(-5, 14), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 8), p(7, 7), p(5, 9), p(11, 7), p(6, 17), p(3, 7)],
    [p(2, 9), p(10, 22), p(-61, -31), p(7, 13), p(8, 22), p(2, 6)],
    [p(-3, 8), p(7, 12), p(4, 17), p(8, 12), p(3, 32), p(12, -4)],
    [p(0, 8), p(6, 11), p(5, 5), p(3, 18), p(-71, -68), p(0, -9)],
    [p(27, 3), p(14, 14), p(20, 9), p(7, 9), p(19, 2), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(6, -19), p(10, -9), p(3, -4), p(13, -19), p(6, -4), p(16, -2)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(4, -2), p(-9, 9), p(1, -8), p(-24, 14)];
const SAFE_CHECK: [PhasedScore; 5] = [p(0, 0), p(55, -7), p(11, 1), p(41, -4), p(28, 42)];
const CHECK_STM: PhasedScore = p(35, 14);
const SAFE_CHECK_STM: PhasedScore = p(35, 5);
const DISCOVERED_CHECK_STM: PhasedScore = p(65, 66);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(11, -3), p(63, 31), p(65, 21), p(67, 68), p(0, 0), p(53, -28)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(13, -9), p(35, 37), p(30, 41), p(49, 19), p(59, 13)];
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

    fn safe_squares(piece: PieceType, num_squares: usize) -> SingleFeatureScore<Self::Score>;

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

    fn safe_squares(piece: PieceType, num_squares: usize) -> PhasedScore {
        SAFE_SQUARES[piece as usize - 1][num_squares]
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
