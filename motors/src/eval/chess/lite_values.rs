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
        p(  19,   51),    p(  20,   51),    p(  24,   46),    p(  26,   41),    p(  25,   46),    p(  30,   56),    p(  -0,   60),    p(   4,   60),
        p( -29,   20),    p( -22,   19),    p( -11,    8),    p( -13,   12),    p( -18,    9),    p(  26,    7),    p(  17,   28),    p(   9,   24),
        p( -42,   -1),    p( -33,   -6),    p( -35,  -13),    p( -13,   -7),    p(  -9,   -9),    p(  -8,  -19),    p(  -6,   -9),    p(  -7,  -10),
        p( -49,  -14),    p( -45,  -12),    p( -25,  -12),    p( -11,   -9),    p(  -6,  -10),    p(  -7,  -13),    p( -15,  -16),    p( -19,  -21),
        p( -58,  -15),    p( -48,  -18),    p( -29,  -10),    p( -17,   -8),    p( -18,   -7),    p( -15,  -10),    p( -10,  -26),    p( -22,  -24),
        p( -47,   -8),    p( -41,  -11),    p( -35,  -12),    p( -37,   -6),    p( -31,   -4),    p(  -5,  -14),    p(  12,  -25),    p( -13,  -23),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
    ],
    // knight
    [
        p( -62,  -46),    p( -65,    3),    p( -66,    7),    p( -50,    9),    p(  -8,   -3),    p( -64,   -2),    p( -55,  -12),    p( -62,  -57),
        p( -19,    4),    p(  -7,   10),    p(   6,   -0),    p(   9,    3),    p(  13,   -3),    p(  22,  -11),    p(   1,    1),    p( -14,   -8),
        p(  -6,    2),    p(  11,   -3),    p(  15,    3),    p(  26,    3),    p(  37,    1),    p(  54,  -10),    p(   2,   -3),    p(   2,   -4),
        p(  14,    7),    p(  21,    3),    p(  34,    6),    p(  34,   13),    p(  33,   11),    p(  35,    8),    p(  25,    8),    p(  35,   -1),
        p(   8,    6),    p(  19,    0),    p(  21,    7),    p(  30,    9),    p(  28,   11),    p(  35,   -2),    p(  33,   -3),    p(  24,    5),
        p( -15,   -6),    p(  -7,   -7),    p(   1,   -9),    p(  10,    3),    p(  15,    1),    p(  12,  -19),    p(  13,  -13),    p(   7,   -5),
        p( -22,    0),    p(  -8,    3),    p(  -6,   -2),    p(   3,   -1),    p(  11,   -5),    p(   5,   -9),    p(   9,    0),    p(   3,    9),
        p( -44,   -6),    p( -13,   -3),    p( -23,   -4),    p(  -2,    1),    p(   4,    1),    p(   4,   -9),    p(  -6,   -2),    p( -21,   -2),
    ],
    // bishop
    [
        p( -24,   15),    p( -37,    8),    p( -62,    4),    p( -59,   11),    p( -64,    8),    p( -59,    5),    p( -38,    8),    p( -41,    5),
        p( -11,    2),    p( -11,    2),    p(   0,    1),    p( -14,    2),    p(  -6,   -3),    p(   2,   -2),    p( -39,   11),    p( -20,    0),
        p(   1,    5),    p(  11,    1),    p(  -4,    4),    p(   8,   -5),    p(   7,    1),    p(  37,   -1),    p(  23,    1),    p(  17,    9),
        p(  -4,    5),    p(   3,    1),    p(  11,   -3),    p(  12,    4),    p(  12,    1),    p(  12,    0),    p(  17,   -2),    p(  -2,    2),
        p(  -3,    1),    p(  -7,    3),    p(  -2,    2),    p(  12,   -1),    p(   8,   -1),    p(  10,   -6),    p(  -3,   -2),    p(  23,   -9),
        p(  -3,    1),    p(   4,    0),    p(   2,    3),    p(  -0,    2),    p(   8,    1),    p(   6,   -5),    p(  14,  -11),    p(  13,   -4),
        p(   9,   -2),    p(   0,   -2),    p(  10,   -4),    p(   3,    4),    p(   5,    2),    p(  12,    0),    p(  18,   -6),    p(  12,   -1),
        p(   5,   -3),    p(  13,   -1),    p(   6,    5),    p(  -0,    4),    p(  13,    3),    p(  -6,    7),    p(   6,    1),    p(  10,   -8),
    ],
    // rook
    [
        p( -37,   44),    p( -41,   47),    p( -50,   53),    p( -52,   52),    p( -41,   47),    p( -24,   48),    p( -33,   50),    p(   1,   31),
        p( -31,   40),    p( -31,   43),    p( -24,   43),    p(  -9,   34),    p( -22,   36),    p(   3,   33),    p(   5,   33),    p(  14,   21),
        p( -32,   39),    p( -14,   32),    p( -20,   35),    p( -21,   29),    p(   3,   22),    p(  20,   17),    p(  27,   21),    p(   1,   22),
        p( -34,   38),    p( -25,   34),    p( -25,   35),    p( -22,   30),    p( -16,   26),    p(  -1,   23),    p(  -5,   28),    p( -10,   24),
        p( -44,   35),    p( -41,   32),    p( -40,   34),    p( -33,   30),    p( -26,   27),    p( -28,   27),    p( -12,   23),    p( -28,   23),
        p( -48,   30),    p( -42,   25),    p( -42,   26),    p( -39,   27),    p( -30,   20),    p( -20,   14),    p(  -5,    9),    p( -20,   14),
        p( -47,   27),    p( -42,   25),    p( -34,   25),    p( -30,   21),    p( -23,   17),    p( -10,   10),    p(   0,    6),    p( -36,   19),
        p( -40,   32),    p( -40,   26),    p( -41,   30),    p( -37,   25),    p( -30,   18),    p( -29,   21),    p( -41,   29),    p( -44,   23),
    ],
    // queen
    [
        p( -12,   61),    p( -10,   65),    p(  10,   67),    p(  30,   60),    p(  36,   63),    p(  54,   57),    p(  69,   26),    p(  23,   45),
        p(  22,   42),    p(  12,   50),    p(  13,   63),    p(  15,   71),    p(  24,   71),    p(  47,   65),    p(  37,   56),    p(  71,   42),
        p(  34,   41),    p(  30,   41),    p(  28,   59),    p(  27,   64),    p(  34,   67),    p(  66,   64),    p(  70,   51),    p(  61,   54),
        p(  22,   48),    p(  26,   51),    p(  25,   49),    p(  19,   64),    p(  25,   65),    p(  41,   60),    p(  44,   68),    p(  49,   55),
        p(  17,   49),    p(  16,   49),    p(  18,   48),    p(  20,   56),    p(  25,   58),    p(  28,   57),    p(  36,   58),    p(  39,   51),
        p(  18,   34),    p(  23,   37),    p(  19,   48),    p(  17,   50),    p(  21,   58),    p(  31,   44),    p(  39,   39),    p(  36,   29),
        p(  19,   30),    p(  17,   39),    p(  22,   40),    p(  22,   53),    p(  23,   54),    p(  27,   32),    p(  35,   19),    p(  25,   21),
        p(   3,   43),    p(  13,   32),    p(  13,   40),    p(  17,   46),    p(  19,   34),    p(   7,   40),    p(   1,   23),    p(  12,    6),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  65,   10),    p(  69,    8),    p(  73,   -1),    p(  70,  -39),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  55,   16),    p(  27,   25),    p(  32,   17),    p(   8,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  14,   25),    p(  29,   19),    p(  24,   12),    p( -18,   14),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -48,   23),    p( -32,   17),    p( -30,    5),    p( -49,    9),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -63,   18),    p( -47,    7),    p( -47,   -4),    p( -62,    0),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -49,    8),    p( -47,    1),    p( -19,  -12),    p( -37,    1),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -38,    1),    p( -25,   -6),    p(   4,  -20),    p(  18,   -6),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  12,   -4),    p(   0,    0),    p(  24,  -13),    p(  30,  -16),
    ],
];

const MORE_MINORS_NO_PAWNS: PhasedScore = p(-66, -65);
const BISHOP_PAIR: PhasedScore = p(25, 47);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(16, 16), p(17, 12), p(16, 2), p(11, -4), p(6, -11), p(2, -19), p(-4, -26), p(-10, -38), p(-21, -41)];
const ROOK_OPEN_FILE: PhasedScore = p(12, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(4, -1);
const KING_OPEN_FILE: PhasedScore = p(-36, 5);
const KING_CLOSED_FILE: PhasedScore = p(13, -7);
const KING_SEMIOPEN_FILE: PhasedScore = p(-3, 11);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-4, 1), p(2, 3), p(3, 0), p(2, 0), p(5, 1), p(6, 3), p(12, -1), p(22, -5)],
    // Closed
    [p(0, 0), p(0, 0), p(16, -17), p(-17, 8), p(2, 6), p(2, 0), p(3, 3), p(3, -1)],
    // SemiOpen
    [p(0, 0), p(-4, 21), p(10, 14), p(0, 8), p(2, 6), p(6, 2), p(6, -2), p(13, 1)],
    // SemiClosed
    [p(0, 0), p(13, -16), p(12, 1), p(2, -3), p(8, -2), p(3, 0), p(7, -1), p(5, -2)],
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
    p(-7, -17),
    p(-14, -10),
    p(7, -8),
    p(1, -14),
    p(8, -10),
    p(7, 12),
    p(-7, -2),
    p(-21, -6),
    p(-12, -0),
    p(-35, 18),
    p(-17, 2),
    p(-15, -21),
    p(7, 22),
    p(-35, 19),
    p(-18, -17),
    p(-20, -18),
    p(-32, -31),
    p(-35, 10),
    p(-12, -3),
    p(17, -9),
    p(-62, 67),
    p(0, 0),
    p(-1, -5),
    p(-13, -8),
    p(2, -9),
    p(-17, -4),
    p(-19, -3),
    p(-41, -22),
    p(-21, 32),
    p(-26, 18),
    p(-5, -7),
    p(-17, -13),
    p(11, -15),
    p(-11, 27),
    p(-43, 14),
    p(4, -32),
    p(0, 0),
    p(0, 0),
    p(-1, -12),
    p(-9, 2),
    p(0, -54),
    p(0, 0),
    p(16, -16),
    p(-28, -17),
    p(0, 0),
    p(0, 0),
    p(-24, -8),
    p(-16, -16),
    p(-9, 14),
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
    p(-19, -3),
    p(10, -4),
    p(-22, -12),
    p(-11, -5),
    p(-28, -17),
    p(7, -0),
    p(-8, -9),
    p(-26, -9),
    p(-38, -5),
    p(1, -8),
    p(-33, -11),
    p(-26, -13),
    p(-39, 46),
    p(8, 2),
    p(-1, -7),
    p(-4, -10),
    p(-20, -1),
    p(-8, -2),
    p(-12, -16),
    p(-18, -3),
    p(-20, 65),
    p(-5, -10),
    p(-23, -16),
    p(-30, -30),
    p(7, -68),
    p(-9, -11),
    p(-6, -24),
    p(-65, 51),
    p(0, 0),
    p(10, 3),
    p(-3, -3),
    p(-16, -7),
    p(-23, -9),
    p(-0, -0),
    p(-23, -21),
    p(-10, -5),
    p(-21, -6),
    p(-1, -6),
    p(-19, -10),
    p(-24, -17),
    p(-31, -10),
    p(-5, -3),
    p(-41, -9),
    p(10, 9),
    p(-47, 48),
    p(4, 1),
    p(-8, -4),
    p(-24, 50),
    p(0, 0),
    p(-9, -6),
    p(-14, -0),
    p(0, 0),
    p(0, 0),
    p(-10, -1),
    p(-31, 3),
    p(-27, -44),
    p(0, 0),
    p(21, -61),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-15, 7),   /*0b0000*/
    p(-12, 4),   /*0b0001*/
    p(-3, 8),    /*0b0010*/
    p(-0, 8),    /*0b0011*/
    p(-7, 0),    /*0b0100*/
    p(-20, -4),  /*0b0101*/
    p(-7, 2),    /*0b0110*/
    p(-8, -7),   /*0b0111*/
    p(-1, 3),    /*0b1000*/
    p(-8, 6),    /*0b1001*/
    p(-1, 7),    /*0b1010*/
    p(7, 8),     /*0b1011*/
    p(-4, 0),    /*0b1100*/
    p(-14, -2),  /*0b1101*/
    p(-7, 1),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(1, 6),     /*0b10000*/
    p(6, 3),     /*0b10001*/
    p(21, 3),    /*0b10010*/
    p(2, 3),     /*0b10011*/
    p(-7, 0),    /*0b10100*/
    p(15, 8),    /*0b10101*/
    p(-26, -1),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(6, 8),     /*0b11000*/
    p(27, 7),    /*0b11001*/
    p(28, 21),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(9, -7),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(6, 0),     /*0b100000*/
    p(1, 3),     /*0b100001*/
    p(14, -0),   /*0b100010*/
    p(8, -5),    /*0b100011*/
    p(-5, -6),   /*0b100100*/
    p(-18, -15), /*0b100101*/
    p(-17, 17),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(13, -5),   /*0b101000*/
    p(1, 3),     /*0b101001*/
    p(10, -9),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(0, -4),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(2, -0),    /*0b110000*/
    p(15, -1),   /*0b110001*/
    p(17, -10),  /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(11, 10),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(17, -8),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(7, -5),    /*0b111111*/
    p(-10, 7),   /*0b00*/
    p(-0, -3),   /*0b01*/
    p(15, -2),   /*0b10*/
    p(-8, -20),  /*0b11*/
    p(17, 3),    /*0b100*/
    p(-8, -3),   /*0b101*/
    p(27, -25),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(32, -5),   /*0b1000*/
    p(-10, -12), /*0b1001*/
    p(7, -32),   /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(-4, -9),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-37, 32),  /*0b1111*/
    p(-3, 8),    /*0b00*/
    p(11, -6),   /*0b01*/
    p(6, -8),    /*0b10*/
    p(6, -28),   /*0b11*/
    p(8, -5),    /*0b100*/
    p(21, -16),  /*0b101*/
    p(-3, -13),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(10, 2),    /*0b1000*/
    p(20, -12),  /*0b1001*/
    p(22, -35),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(5, -19),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-7, -34),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(1, -35);
const STOPPABLE_PASSER: PhasedScore = p(15, -38);
const CLOSE_KING_PASSER: PhasedScore = p(-1, 26);
const IMMOBILE_PASSER: PhasedScore = p(-7, -33);
const PROTECTED_PASSER: PhasedScore = p(7, -0);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -25,   35),    p( -27,   51),    p( -38,   44),    p( -40,   27),    p( -38,   26),    p( -28,   24),    p( -15,   27),    p( -40,   30),
        p( -19,   33),    p( -33,   52),    p( -35,   42),    p( -34,   28),    p( -47,   35),    p( -43,   33),    p( -33,   40),    p( -33,   28),
        p( -12,   44),    p( -20,   43),    p( -31,   43),    p( -27,   41),    p( -37,   45),    p( -33,   46),    p( -35,   53),    p( -55,   53),
        p(   1,   53),    p(  -2,   51),    p(  -0,   42),    p( -12,   50),    p( -26,   57),    p( -18,   60),    p( -18,   64),    p( -39,   65),
        p(   7,   55),    p(  20,   49),    p(   7,   42),    p(  -5,   32),    p(  -1,   43),    p(  -8,   56),    p( -21,   57),    p( -43,   61),
        p(  19,   51),    p(  20,   51),    p(  24,   46),    p(  26,   41),    p(  25,   46),    p(  30,   56),    p(  -0,   60),    p(   4,   60),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(-0, 4), p(4, 8), p(7, 17), p(11, 28), p(18, 55), p(21, 55)];
const UNSUPPORTED_PAWN: PhasedScore = p(-9, -4);
const DOUBLED_PAWN: PhasedScore = p(-7, -23);
const PHALANX: [PhasedScore; 6] = [p(-3, 1), p(4, 4), p(8, 6), p(21, 19), p(58, 59), p(67, 64)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(15, 14), p(7, 16), p(14, 20), p(6, 10), p(-3, 11), p(-47, 11)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(48, 20), p(50, 42), p(58, 9), p(50, -6), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, -4), p(14, 20), p(18, -7), p(15, 10), p(17, -11), p(30, -11)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-41, -60),
        p(-19, -35),
        p(-6, -13),
        p(3, -1),
        p(11, 9),
        p(18, 19),
        p(26, 21),
        p(33, 22),
        p(36, 22),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-31, -49),
        p(-17, -33),
        p(-6, -19),
        p(2, -7),
        p(9, 2),
        p(14, 11),
        p(19, 15),
        p(24, 19),
        p(26, 24),
        p(33, 25),
        p(38, 25),
        p(43, 29),
        p(37, 39),
        p(47, 31),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-49, -22),
        p(-41, -7),
        p(-37, 1),
        p(-34, 6),
        p(-35, 13),
        p(-30, 20),
        p(-28, 25),
        p(-25, 30),
        p(-23, 35),
        p(-20, 40),
        p(-16, 42),
        p(-15, 48),
        p(-9, 49),
        p(-2, 48),
        p(-0, 45),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-1, -65),
        p(0, -65),
        p(-3, -35),
        p(1, -20),
        p(3, -0),
        p(7, 6),
        p(10, 19),
        p(13, 26),
        p(17, 33),
        p(20, 35),
        p(22, 40),
        p(26, 44),
        p(29, 46),
        p(31, 51),
        p(33, 54),
        p(38, 57),
        p(41, 62),
        p(45, 63),
        p(54, 62),
        p(63, 62),
        p(66, 64),
        p(68, 68),
        p(69, 70),
        p(67, 69),
        p(69, 75),
        p(66, 72),
        p(67, 75),
        p(63, 66),
    ],
    [
        p(-19, -9),
        p(-13, -18),
        p(-7, -16),
        p(-1, -9),
        p(7, -3),
        p(8, 4),
        p(16, 13),
        p(19, 20),
        p(41, 16),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-4, 13), p(0, 0), p(28, 25), p(56, 9), p(39, -2), p(0, 0)],
    [p(-1, 11), p(21, 24), p(0, 0), p(42, 21), p(50, 64), p(0, 0)],
    [p(-6, 20), p(9, 19), p(16, 16), p(0, 0), p(58, 51), p(0, 0)],
    [p(-4, 17), p(-1, 18), p(-1, 31), p(-4, 23), p(0, 0), p(0, 0)],
    [p(49, 22), p(-22, 23), p(4, 17), p(-13, 23), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 8), p(8, 6), p(6, 10), p(12, 7), p(8, 16), p(3, 8)],
    [p(3, 8), p(11, 20), p(-49, -32), p(9, 12), p(10, 18), p(5, 6)],
    [p(-3, 8), p(8, 12), p(4, 16), p(7, 13), p(4, 33), p(15, -1)],
    [p(-0, 11), p(6, 12), p(5, 7), p(3, 18), p(-71, -68), p(2, -1)],
    [p(24, 5), p(13, 16), p(18, 11), p(3, 9), p(15, -4), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(8, -20), p(18, -9), p(9, -3), p(15, -17), p(-4, 7), p(9, 0)];
const WEAK_BACKRANK: PhasedScore = p(-4, 24);
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(9, 2), p(-5, 6), p(14, -7), p(-11, 32)];
const CHECK_STM: PhasedScore = p(38, 21);
const DISCOVERED_CHECK_STM: PhasedScore = p(64, 66);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(10, -5), p(63, 26), p(66, 11), p(66, 67), p(0, 0), p(26, -24)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(5, -15), p(26, 28), p(17, 32), p(41, 10), p(54, 13)];
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

    fn weak_backrank() -> SingleFeatureScore<Self::Score>;

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

    fn weak_backrank() -> PhasedScore {
        WEAK_BACKRANK
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
