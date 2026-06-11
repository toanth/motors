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
        p(  20,   52),    p(  24,   51),    p(  25,   47),    p(  28,   42),    p(  27,   46),    p(  32,   56),    p(   9,   60),    p(   8,   60),
        p( -28,   22),    p( -22,   21),    p( -10,   10),    p( -12,   13),    p( -17,   11),    p(  27,   10),    p(  18,   30),    p(   9,   27),
        p( -41,   -0),    p( -32,   -5),    p( -35,  -12),    p( -12,   -7),    p(  -8,   -9),    p(  -7,  -18),    p(  -6,   -8),    p(  -7,   -9),
        p( -48,  -13),    p( -44,  -11),    p( -25,  -11),    p( -11,   -8),    p(  -6,   -9),    p(  -6,  -13),    p( -15,  -15),    p( -19,  -20),
        p( -57,  -14),    p( -47,  -18),    p( -29,  -10),    p( -18,   -7),    p( -18,   -7),    p( -15,  -10),    p( -10,  -25),    p( -22,  -22),
        p( -47,   -7),    p( -40,   -9),    p( -35,  -11),    p( -37,   -5),    p( -31,   -2),    p(  -4,  -13),    p(  12,  -24),    p( -12,  -22),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
    ],
    // knight
    [
        p( -62,  -45),    p( -65,    4),    p( -62,    8),    p( -37,    7),    p(  -1,   -3),    p( -45,   -3),    p( -49,  -13),    p( -62,  -57),
        p( -17,    4),    p(  -7,   11),    p(   6,    1),    p(  10,    3),    p(  14,   -2),    p(  22,  -10),    p(   6,   -0),    p( -16,   -8),
        p(  -6,    2),    p(  12,   -1),    p(  16,    3),    p(  26,    4),    p(  37,    1),    p(  53,  -10),    p(   3,   -2),    p(   7,   -5),
        p(  13,    6),    p(  21,    3),    p(  33,    7),    p(  35,   14),    p(  34,   12),    p(  35,    9),    p(  25,    9),    p(  36,   -0),
        p(   7,    6),    p(  18,    0),    p(  22,    7),    p(  29,   10),    p(  28,   12),    p(  35,   -1),    p(  32,   -2),    p(  24,    5),
        p( -15,   -6),    p(  -8,   -7),    p(   1,   -8),    p(  10,    4),    p(  15,    2),    p(  12,  -17),    p(  13,  -12),    p(   8,   -4),
        p( -22,   -0),    p(  -8,    3),    p(  -6,   -2),    p(   3,    0),    p(  10,   -4),    p(   5,   -8),    p(   8,   -1),    p(   4,    7),
        p( -45,   -7),    p( -14,   -3),    p( -23,   -4),    p(  -2,    1),    p(   4,    1),    p(   4,   -8),    p(  -7,   -1),    p( -20,   -2),
    ],
    // bishop
    [
        p( -22,   14),    p( -31,    7),    p( -55,    3),    p( -53,   10),    p( -61,    7),    p( -31,    1),    p( -28,    6),    p( -39,    5),
        p( -12,    2),    p( -10,    3),    p(  -0,    1),    p( -13,    2),    p(  -6,   -3),    p(  -1,   -3),    p( -37,   11),    p( -26,    2),
        p(   1,    5),    p(   8,    1),    p(  -4,    5),    p(   6,   -5),    p(   6,    1),    p(  36,    1),    p(  23,    1),    p(  16,   11),
        p(  -7,    5),    p(   3,    1),    p(   9,   -3),    p(  11,    4),    p(  11,    1),    p(  13,    0),    p(  17,   -2),    p(  -3,    2),
        p(  -6,    1),    p(  -9,    2),    p(  -2,    2),    p(  11,   -0),    p(   9,   -0),    p(  10,   -6),    p(  -3,   -1),    p(  20,   -9),
        p(  -4,    1),    p(   2,   -0),    p(   2,    3),    p(   0,    3),    p(   8,    1),    p(   6,   -5),    p(  13,  -10),    p(  13,   -3),
        p(  10,   -2),    p(   0,   -2),    p(   9,   -4),    p(   2,    4),    p(   4,    2),    p(  11,    0),    p(  18,   -7),    p(  14,   -3),
        p(   4,   -2),    p(  13,   -1),    p(   6,    4),    p(  -4,    3),    p(  10,    3),    p(  -7,    7),    p(   7,    1),    p(  11,   -8),
    ],
    // rook
    [
        p( -36,   42),    p( -40,   45),    p( -48,   51),    p( -49,   50),    p( -37,   45),    p( -21,   46),    p( -33,   49),    p(  -2,   30),
        p( -30,   42),    p( -31,   45),    p( -24,   44),    p(  -8,   36),    p( -20,   38),    p(   6,   34),    p(   8,   33),    p(  14,   24),
        p( -31,   38),    p( -12,   31),    p( -19,   33),    p( -18,   27),    p(   5,   20),    p(  21,   15),    p(  27,   20),    p(   1,   21),
        p( -33,   37),    p( -24,   32),    p( -23,   33),    p( -20,   29),    p( -14,   24),    p(   1,   22),    p(  -4,   26),    p( -10,   23),
        p( -44,   34),    p( -41,   32),    p( -39,   33),    p( -32,   29),    p( -25,   27),    p( -27,   27),    p( -12,   23),    p( -28,   22),
        p( -47,   29),    p( -41,   25),    p( -41,   26),    p( -38,   26),    p( -29,   20),    p( -19,   15),    p(  -5,   10),    p( -20,   14),
        p( -46,   27),    p( -41,   25),    p( -34,   25),    p( -29,   21),    p( -22,   18),    p( -10,   11),    p(   0,    6),    p( -36,   19),
        p( -40,   31),    p( -40,   25),    p( -41,   29),    p( -36,   25),    p( -29,   19),    p( -28,   23),    p( -41,   30),    p( -43,   24),
    ],
    // queen
    [
        p( -14,   60),    p(  -9,   60),    p(   7,   64),    p(  30,   57),    p(  38,   58),    p(  55,   52),    p(  69,   21),    p(  20,   42),
        p(  16,   45),    p(   3,   55),    p(  10,   63),    p(  12,   70),    p(  22,   71),    p(  45,   65),    p(  26,   64),    p(  64,   45),
        p(  33,   40),    p(  28,   42),    p(  25,   60),    p(  27,   62),    p(  33,   66),    p(  64,   64),    p(  68,   53),    p(  61,   55),
        p(  21,   47),    p(  25,   51),    p(  24,   48),    p(  16,   63),    p(  23,   64),    p(  41,   59),    p(  43,   66),    p(  48,   54),
        p(  18,   46),    p(  16,   47),    p(  17,   47),    p(  19,   56),    p(  25,   58),    p(  29,   56),    p(  36,   56),    p(  39,   49),
        p(  17,   31),    p(  23,   35),    p(  19,   46),    p(  17,   50),    p(  22,   57),    p(  32,   44),    p(  39,   39),    p(  36,   27),
        p(  19,   27),    p(  17,   36),    p(  22,   37),    p(  22,   50),    p(  24,   51),    p(  28,   31),    p(  35,   17),    p(  24,   20),
        p(   2,   40),    p(  13,   28),    p(  13,   36),    p(  17,   42),    p(  20,   33),    p(   7,   40),    p(   1,   24),    p(  11,    6),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  68,    7),    p(  69,    5),    p(  73,   -4),    p(  71,  -43),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  56,   17),    p(  34,   26),    p(  34,   17),    p(  12,    9),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  26,   26),    p(  38,   20),    p(  31,   12),    p(  -7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -38,   24),    p( -22,   17),    p( -24,    5),    p( -45,    9),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -58,   19),    p( -41,    8),    p( -44,   -3),    p( -62,    1),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -46,   10),    p( -46,    3),    p( -22,  -10),    p( -37,    2),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -42,    2),    p( -30,   -3),    p(  -5,  -17),    p(  14,   -5),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   9,   -6),    p(  -2,   -2),    p(  19,  -15),    p(  27,  -18),
    ],
];

const MORE_MINORS_NO_PAWNS: PhasedScore = p(-66, -65);
const BISHOP_PAIR: PhasedScore = p(25, 46);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(16, 16), p(17, 13), p(16, 3), p(12, -4), p(7, -11), p(3, -19), p(-3, -26), p(-10, -39), p(-20, -43)];
const ROOK_OPEN_FILE: PhasedScore = p(13, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(4, 2);
const KING_OPEN_FILE: PhasedScore = p(-30, 5);
const KING_CLOSED_FILE: PhasedScore = p(12, -7);
const KING_SEMIOPEN_FILE: PhasedScore = p(-3, 10);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-3, 1), p(2, 2), p(4, -0), p(5, 0), p(6, 2), p(6, 3), p(12, -0), p(22, -4)],
    // Closed
    [p(0, 0), p(0, 0), p(14, -22), p(-16, 7), p(3, 6), p(2, 0), p(3, 2), p(2, -1)],
    // SemiOpen
    [p(0, 0), p(1, 12), p(11, 8), p(3, 5), p(3, 6), p(6, 2), p(6, -2), p(13, 0)],
    // SemiClosed
    [p(0, 0), p(13, -15), p(12, 1), p(4, -3), p(8, -2), p(3, 0), p(7, 0), p(5, -1)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(18, 10),
    p(1, 3),
    p(0, 4),
    p(-6, 4),
    p(5, 2),
    p(-7, -12),
    p(-3, -2),
    p(-3, -13),
    p(-2, -1),
    p(-12, -3),
    p(-8, -17),
    p(-14, -9),
    p(6, -7),
    p(1, -13),
    p(7, -8),
    p(7, 11),
    p(-7, -2),
    p(-20, -6),
    p(-10, 0),
    p(-34, 18),
    p(-16, 2),
    p(-14, -21),
    p(7, 24),
    p(-39, 21),
    p(-18, -16),
    p(-19, -17),
    p(-30, -30),
    p(-35, 12),
    p(-12, -2),
    p(18, -9),
    p(-65, 67),
    p(0, 0),
    p(-0, -5),
    p(-12, -7),
    p(2, -9),
    p(-17, -4),
    p(-17, -3),
    p(-38, -21),
    p(-22, 32),
    p(-22, 13),
    p(-6, -7),
    p(-17, -13),
    p(10, -14),
    p(-11, 26),
    p(-42, 15),
    p(2, -32),
    p(0, 0),
    p(0, 0),
    p(0, -12),
    p(-7, 3),
    p(2, -51),
    p(0, 0),
    p(16, -15),
    p(-29, -12),
    p(0, 0),
    p(0, 0),
    p(-24, -6),
    p(-15, -17),
    p(-8, 12),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(22, 9),
    p(3, 1),
    p(-4, 3),
    p(-18, -2),
    p(10, -4),
    p(-22, -11),
    p(-11, -5),
    p(-29, -16),
    p(8, -0),
    p(-7, -9),
    p(-26, -8),
    p(-38, -5),
    p(0, -7),
    p(-33, -11),
    p(-27, -11),
    p(-41, 48),
    p(8, 2),
    p(-1, -7),
    p(-3, -10),
    p(-20, -0),
    p(-8, -2),
    p(-12, -16),
    p(-17, -5),
    p(-20, 65),
    p(-6, -10),
    p(-23, -16),
    p(-30, -28),
    p(7, -66),
    p(-9, -10),
    p(-6, -23),
    p(-66, 53),
    p(0, 0),
    p(9, 3),
    p(-3, -3),
    p(-15, -7),
    p(-22, -9),
    p(-1, -0),
    p(-24, -20),
    p(-10, -6),
    p(-21, -7),
    p(-1, -6),
    p(-19, -10),
    p(-23, -17),
    p(-31, -10),
    p(-4, -3),
    p(-42, -9),
    p(15, 5),
    p(-50, 49),
    p(3, 1),
    p(-8, -4),
    p(-22, 50),
    p(0, 0),
    p(-9, -6),
    p(-14, -1),
    p(0, 0),
    p(0, 0),
    p(-10, -1),
    p(-31, 3),
    p(-24, -44),
    p(0, 0),
    p(23, -63),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-12, 8),   /*0b0000*/
    p(-13, 6),   /*0b0001*/
    p(-3, 9),    /*0b0010*/
    p(-1, 9),    /*0b0011*/
    p(-7, 2),    /*0b0100*/
    p(-21, -2),  /*0b0101*/
    p(-7, 3),    /*0b0110*/
    p(-9, -12),  /*0b0111*/
    p(1, 4),     /*0b1000*/
    p(-7, 7),    /*0b1001*/
    p(1, 8),     /*0b1010*/
    p(6, 9),     /*0b1011*/
    p(-3, 2),    /*0b1100*/
    p(-15, 1),   /*0b1101*/
    p(-7, 2),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(2, 7),     /*0b10000*/
    p(6, 4),     /*0b10001*/
    p(22, 5),    /*0b10010*/
    p(1, 6),     /*0b10011*/
    p(-6, 1),    /*0b10100*/
    p(14, 9),    /*0b10101*/
    p(-26, 1),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(8, 8),     /*0b11000*/
    p(27, 8),    /*0b11001*/
    p(30, 22),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(9, -6),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(8, 1),     /*0b100000*/
    p(1, 5),     /*0b100001*/
    p(15, 1),    /*0b100010*/
    p(7, -2),    /*0b100011*/
    p(-5, -4),   /*0b100100*/
    p(-18, -14), /*0b100101*/
    p(-18, 17),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(15, -5),   /*0b101000*/
    p(1, 5),     /*0b101001*/
    p(11, -7),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(0, -3),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(4, 0),     /*0b110000*/
    p(15, 1),    /*0b110001*/
    p(18, -8),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(11, 11),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(19, -9),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(6, -5),    /*0b111111*/
    p(-6, 7),    /*0b00*/
    p(-2, -4),   /*0b01*/
    p(15, -2),   /*0b10*/
    p(-9, -24),  /*0b11*/
    p(16, 2),    /*0b100*/
    p(-14, -0),  /*0b101*/
    p(27, -24),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(37, -5),   /*0b1000*/
    p(-11, -12), /*0b1001*/
    p(6, -30),   /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(0, -12),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-40, 27),  /*0b1111*/
    p(-2, 8),    /*0b00*/
    p(10, -6),   /*0b01*/
    p(5, -8),    /*0b10*/
    p(5, -31),   /*0b11*/
    p(9, -5),    /*0b100*/
    p(22, -13),  /*0b101*/
    p(-3, -14),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(11, 1),    /*0b1000*/
    p(20, -11),  /*0b1001*/
    p(22, -38),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(5, -19),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-8, -36),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(2, -35);
const STOPPABLE_PASSER: PhasedScore = p(15, -39);
const CLOSE_KING_PASSER: PhasedScore = p(-1, 26);
const IMMOBILE_PASSER: PhasedScore = p(-7, -34);
const PROTECTED_PASSER: PhasedScore = p(6, -1);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -24,   36),    p( -26,   52),    p( -36,   45),    p( -37,   27),    p( -37,   27),    p( -28,   25),    p( -16,   28),    p( -39,   31),
        p( -18,   33),    p( -31,   53),    p( -34,   44),    p( -33,   29),    p( -47,   36),    p( -42,   33),    p( -33,   40),    p( -33,   29),
        p( -11,   44),    p( -19,   44),    p( -30,   44),    p( -27,   42),    p( -36,   46),    p( -32,   46),    p( -34,   54),    p( -54,   54),
        p(   1,   53),    p(  -2,   52),    p(   0,   43),    p( -11,   51),    p( -25,   58),    p( -17,   61),    p( -17,   64),    p( -38,   65),
        p(   8,   55),    p(  21,   48),    p(   7,   43),    p(  -5,   32),    p(  -0,   42),    p(  -7,   55),    p( -20,   57),    p( -41,   61),
        p(  20,   52),    p(  24,   51),    p(  25,   47),    p(  28,   42),    p(  27,   46),    p(  32,   56),    p(   9,   60),    p(   8,   60),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(-0, 4), p(4, 8), p(7, 18), p(12, 28), p(19, 55), p(24, 55)];
const UNSUPPORTED_PAWN: PhasedScore = p(-9, -4);
const DOUBLED_PAWN: PhasedScore = p(-6, -23);
const PHALANX: [PhasedScore; 6] = [p(-3, 1), p(4, 4), p(8, 6), p(21, 19), p(58, 59), p(67, 64)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(15, 13), p(6, 15), p(14, 19), p(5, 10), p(-3, 11), p(-48, 11)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(49, 21), p(50, 41), p(58, 7), p(47, 0), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, -4), p(14, 19), p(18, -7), p(16, 8), p(16, -12), p(28, -11)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-39, -59),
        p(-17, -34),
        p(-5, -12),
        p(4, -1),
        p(12, 9),
        p(19, 18),
        p(27, 20),
        p(34, 21),
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
        p(-30, -47),
        p(-17, -32),
        p(-5, -18),
        p(2, -7),
        p(9, 2),
        p(14, 10),
        p(19, 14),
        p(24, 18),
        p(26, 22),
        p(33, 23),
        p(38, 23),
        p(44, 26),
        p(39, 36),
        p(48, 28),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-48, -17),
        p(-40, -2),
        p(-37, 5),
        p(-33, 10),
        p(-34, 16),
        p(-30, 22),
        p(-28, 27),
        p(-25, 31),
        p(-22, 36),
        p(-19, 39),
        p(-16, 42),
        p(-15, 47),
        p(-8, 48),
        p(-0, 47),
        p(2, 44),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(3, -65),
        p(5, -63),
        p(1, -27),
        p(4, -12),
        p(6, 7),
        p(10, 13),
        p(13, 25),
        p(16, 32),
        p(19, 38),
        p(22, 40),
        p(24, 44),
        p(28, 48),
        p(30, 49),
        p(32, 54),
        p(35, 56),
        p(39, 59),
        p(42, 63),
        p(46, 64),
        p(55, 62),
        p(63, 61),
        p(65, 63),
        p(68, 67),
        p(68, 69),
        p(66, 67),
        p(68, 75),
        p(64, 69),
        p(65, 74),
        p(62, 61),
    ],
    [
        p(-22, -14),
        p(-15, -19),
        p(-9, -14),
        p(-2, -6),
        p(7, -1),
        p(10, 4),
        p(19, 12),
        p(25, 18),
        p(53, 13),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-4, 11), p(0, 0), p(27, 27), p(51, 20), p(35, 15), p(0, 0)],
    [p(-1, 8), p(21, 24), p(0, 0), p(35, 35), p(46, 64), p(0, 0)],
    [p(-5, 12), p(9, 14), p(16, 10), p(0, 0), p(53, 64), p(0, 0)],
    [p(-4, 15), p(-1, 15), p(-2, 24), p(-3, 14), p(0, 0), p(0, 0)],
    [p(42, 19), p(-35, 22), p(-6, 18), p(-22, 11), p(0, 0), p(0, 0)],
];
const HANGING: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(13, 9), p(-2, -24), p(4, -12), p(19, -28), p(6, -32), p(0, 0)],
    [p(8, 16), p(15, -13), p(9, -6), p(26, -35), p(11, -13), p(0, 0)],
    [p(9, 24), p(19, 18), p(12, 21), p(12, 20), p(15, -41), p(0, 0)],
    [p(11, 5), p(24, 16), p(17, 34), p(40, 4), p(14, 24), p(0, 0)],
    [p(52, 11), p(68, 3), p(54, -1), p(67, -5), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 6), p(8, 5), p(6, 9), p(12, 6), p(8, 16), p(3, 7)],
    [p(2, 6), p(11, 18), p(-47, -33), p(8, 11), p(10, 18), p(3, 5)],
    [p(-3, 5), p(8, 8), p(4, 13), p(7, 11), p(4, 31), p(13, -4)],
    [p(-1, 9), p(6, 10), p(4, 5), p(3, 16), p(-72, -69), p(1, -5)],
    [p(25, 2), p(12, 14), p(18, 8), p(4, 10), p(17, -3), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(8, -19), p(19, -8), p(13, -2), p(17, -11), p(5, 6), p(7, 0)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(9, -1), p(-8, 9), p(13, -8), p(-19, 34)];
const CHECK_STM: PhasedScore = p(34, 12);
const SAFE_CHECK_STM: PhasedScore = p(58, 8);
const DISCOVERED_CHECK_STM: PhasedScore = p(64, 66);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(10, -8), p(63, 24), p(65, 11), p(66, 67), p(0, 0), p(35, -23)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(7, -11), p(30, 33), p(23, 37), p(40, 22), p(59, 20)];
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

    fn hanging(attacking: PieceType, targeted: PieceType) -> SingleFeatureScore<Self::Score>;

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

    fn hanging(attacking: PieceType, target: PieceType) -> PhasedScore {
        HANGING[attacking as usize - 1][target as usize]
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
