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
use gears::games::DimT;
use gears::games::chess::Color;
use gears::games::chess::Color::White;
use gears::games::chess::pieces::{NUM_CHESS_PIECES, PieceType};
use gears::games::chess::squares::{NUM_SQUARES, Square};
use gears::general::common::StaticallyNamedEntity;
use gears::score::{PhasedScore, p};
use std::fmt::{Debug, Display};

#[rustfmt::skip]
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  19,   51),    p(  20,   51),    p(  24,   46),    p(  26,   41),    p(  25,   45),    p(  30,   55),    p(  -0,   60),    p(   4,   59),
        p( -29,   20),    p( -22,   19),    p( -11,    9),    p( -13,   12),    p( -18,    9),    p(  26,    8),    p(  18,   29),    p(  10,   25),
        p( -42,   -1),    p( -33,   -6),    p( -35,  -13),    p( -13,   -7),    p(  -9,   -9),    p(  -8,  -20),    p(  -6,   -9),    p(  -7,  -11),
        p( -49,  -14),    p( -45,  -12),    p( -25,  -12),    p( -11,   -9),    p(  -6,  -10),    p(  -6,  -13),    p( -15,  -16),    p( -19,  -21),
        p( -57,  -15),    p( -48,  -18),    p( -29,  -10),    p( -18,   -8),    p( -18,   -7),    p( -15,  -10),    p( -10,  -26),    p( -22,  -23),
        p( -47,   -8),    p( -41,  -10),    p( -35,  -11),    p( -37,   -6),    p( -31,   -3),    p(  -5,  -14),    p(  12,  -25),    p( -13,  -23),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
    ],
    // knight
    [
        p( -62,  -46),    p( -65,    4),    p( -66,    8),    p( -49,    9),    p(  -8,   -3),    p( -64,   -1),    p( -55,  -12),    p( -62,  -57),
        p( -19,    5),    p(  -7,   11),    p(   5,    0),    p(   8,    3),    p(  12,   -3),    p(  22,  -10),    p(   1,    2),    p( -15,   -7),
        p(  -7,    3),    p(  11,   -2),    p(  15,    3),    p(  26,    4),    p(  37,    1),    p(  54,  -10),    p(   1,   -2),    p(   2,   -4),
        p(  14,    7),    p(  21,    4),    p(  34,    7),    p(  34,   14),    p(  33,   12),    p(  35,    9),    p(  25,    9),    p(  35,    0),
        p(   8,    7),    p(  18,    1),    p(  21,    7),    p(  30,    9),    p(  27,   11),    p(  35,   -2),    p(  33,   -3),    p(  24,    5),
        p( -15,   -6),    p(  -7,   -7),    p(   1,   -8),    p(  10,    3),    p(  15,    1),    p(  12,  -19),    p(  13,  -12),    p(   7,   -4),
        p( -22,    1),    p(  -8,    4),    p(  -6,   -2),    p(   3,   -0),    p(  11,   -4),    p(   5,   -9),    p(   8,   -0),    p(   3,    7),
        p( -45,   -6),    p( -13,   -3),    p( -23,   -3),    p(  -2,    1),    p(   4,    2),    p(   4,   -8),    p(  -6,   -1),    p( -21,   -1),
    ],
    // bishop
    [
        p( -24,   15),    p( -37,    8),    p( -62,    4),    p( -59,   11),    p( -64,    8),    p( -59,    5),    p( -38,    8),    p( -41,    5),
        p( -11,    2),    p( -11,    3),    p(   0,    1),    p( -15,    1),    p(  -6,   -3),    p(   1,   -3),    p( -39,   12),    p( -20,    1),
        p(   1,    6),    p(  11,    1),    p(  -4,    4),    p(   8,   -5),    p(   7,    0),    p(  38,   -0),    p(  23,    2),    p(  17,   11),
        p(  -4,    6),    p(   3,    1),    p(  11,   -4),    p(  12,    4),    p(  12,    1),    p(  12,    0),    p(  17,   -2),    p(  -2,    2),
        p(  -3,    1),    p(  -8,    2),    p(  -2,    2),    p(  12,   -1),    p(   8,   -1),    p(  10,   -6),    p(  -3,   -2),    p(  22,   -9),
        p(  -3,    1),    p(   4,    0),    p(   2,    3),    p(  -0,    2),    p(   8,    1),    p(   6,   -5),    p(  14,  -11),    p(  13,   -3),
        p(   9,   -2),    p(   0,   -2),    p(  10,   -3),    p(   3,    4),    p(   5,    2),    p(  12,    0),    p(  18,   -7),    p(  12,   -3),
        p(   5,   -2),    p(  13,   -1),    p(   6,    5),    p(  -1,    4),    p(  13,    4),    p(  -6,    7),    p(   6,    2),    p(  10,   -7),
    ],
    // rook
    [
        p( -38,   44),    p( -41,   47),    p( -51,   53),    p( -53,   51),    p( -43,   47),    p( -26,   48),    p( -33,   50),    p(   1,   31),
        p( -31,   43),    p( -32,   46),    p( -25,   46),    p( -10,   38),    p( -22,   40),    p(   4,   36),    p(   6,   35),    p(  15,   24),
        p( -32,   38),    p( -14,   32),    p( -20,   34),    p( -21,   28),    p(   3,   21),    p(  20,   16),    p(  27,   21),    p(   1,   22),
        p( -34,   38),    p( -25,   33),    p( -25,   34),    p( -22,   30),    p( -16,   26),    p(  -0,   22),    p(  -5,   28),    p( -10,   23),
        p( -44,   35),    p( -41,   32),    p( -40,   33),    p( -33,   29),    p( -26,   27),    p( -28,   27),    p( -12,   23),    p( -28,   22),
        p( -48,   30),    p( -42,   25),    p( -42,   26),    p( -39,   26),    p( -29,   19),    p( -19,   14),    p(  -5,    9),    p( -20,   14),
        p( -47,   28),    p( -42,   25),    p( -34,   25),    p( -30,   21),    p( -23,   17),    p( -10,   10),    p(  -0,    5),    p( -36,   19),
        p( -40,   32),    p( -40,   26),    p( -41,   29),    p( -37,   24),    p( -30,   18),    p( -29,   22),    p( -41,   29),    p( -44,   23),
    ],
    // queen
    [
        p( -12,   60),    p( -10,   64),    p(   9,   66),    p(  30,   60),    p(  36,   62),    p(  54,   56),    p(  70,   25),    p(  23,   44),
        p(  22,   43),    p(  12,   52),    p(  13,   64),    p(  15,   71),    p(  25,   71),    p(  47,   65),    p(  37,   58),    p(  71,   44),
        p(  34,   40),    p(  30,   41),    p(  29,   58),    p(  27,   64),    p(  34,   67),    p(  66,   65),    p(  70,   54),    p(  61,   55),
        p(  22,   47),    p(  26,   50),    p(  25,   48),    p(  20,   63),    p(  25,   65),    p(  41,   61),    p(  44,   68),    p(  49,   55),
        p(  18,   48),    p(  17,   48),    p(  18,   47),    p(  20,   56),    p(  25,   58),    p(  28,   56),    p(  36,   57),    p(  39,   51),
        p(  18,   32),    p(  23,   36),    p(  19,   47),    p(  17,   51),    p(  21,   57),    p(  32,   44),    p(  39,   39),    p(  36,   29),
        p(  19,   29),    p(  17,   39),    p(  22,   40),    p(  22,   52),    p(  23,   53),    p(  27,   31),    p(  35,   18),    p(  24,   20),
        p(   3,   42),    p(  13,   32),    p(  13,   39),    p(  17,   45),    p(  19,   34),    p(   7,   40),    p(   1,   25),    p(  12,    5),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  59,    9),    p(  67,    6),    p(  73,   -2),    p(  71,  -41),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  55,   16),    p(  28,   26),    p(  33,   18),    p(  10,   11),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  18,   26),    p(  34,   19),    p(  28,   13),    p( -14,   15),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -46,   23),    p( -30,   17),    p( -28,    5),    p( -48,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -62,   19),    p( -46,    7),    p( -46,   -3),    p( -62,    2),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -49,    9),    p( -47,    2),    p( -19,  -11),    p( -37,    3),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -40,    1),    p( -27,   -5),    p(   2,  -19),    p(  17,   -5),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  12,   -6),    p(  -1,   -2),    p(  23,  -14),    p(  30,  -17),
    ],
];

const MORE_MINORS_NO_PAWNS: PhasedScore = p(-68, -66);
const BISHOP_PAIR: PhasedScore = p(25, 47);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(15, 18), p(17, 12), p(16, 2), p(11, -5), p(6, -11), p(2, -19), p(-4, -26), p(-10, -39), p(-21, -42)];
const ROOK_OPEN_FILE: PhasedScore = p(12, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(4, -1);
const KING_OPEN_FILE: PhasedScore = p(-36, 5);
const KING_CLOSED_FILE: PhasedScore = p(13, -6);
const KING_SEMIOPEN_FILE: PhasedScore = p(-3, 11);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-4, 1), p(2, 3), p(3, -0), p(2, 0), p(5, 2), p(6, 3), p(12, -1), p(22, -5)],
    // Closed
    [p(0, 0), p(0, 0), p(16, -18), p(-17, 8), p(2, 6), p(2, 1), p(3, 3), p(3, -1)],
    // SemiOpen
    [p(0, 0), p(-4, 21), p(10, 13), p(0, 7), p(2, 6), p(6, 2), p(6, -2), p(13, 0)],
    // SemiClosed
    [p(0, 0), p(13, -15), p(12, 1), p(2, -3), p(8, -2), p(2, 0), p(7, -0), p(5, -2)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(19, 10),
    p(1, 3),
    p(1, 4),
    p(-6, 4),
    p(5, 2),
    p(-7, -12),
    p(-3, -2),
    p(-3, -13),
    p(-2, -1),
    p(-13, -3),
    p(-7, -17),
    p(-14, -10),
    p(7, -8),
    p(1, -13),
    p(8, -9),
    p(7, 11),
    p(-7, -2),
    p(-21, -6),
    p(-12, -0),
    p(-35, 17),
    p(-17, 2),
    p(-15, -21),
    p(7, 23),
    p(-36, 19),
    p(-18, -17),
    p(-20, -18),
    p(-32, -31),
    p(-35, 9),
    p(-12, -3),
    p(17, -9),
    p(-63, 67),
    p(0, 0),
    p(-0, -5),
    p(-13, -8),
    p(2, -9),
    p(-17, -5),
    p(-19, -3),
    p(-41, -22),
    p(-21, 31),
    p(-27, 18),
    p(-5, -7),
    p(-18, -13),
    p(11, -15),
    p(-11, 27),
    p(-43, 15),
    p(4, -33),
    p(0, 0),
    p(0, 0),
    p(-1, -13),
    p(-9, 2),
    p(0, -53),
    p(0, 0),
    p(16, -15),
    p(-28, -16),
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
    p(3, 0),
    p(-5, 4),
    p(-19, -3),
    p(10, -4),
    p(-22, -11),
    p(-11, -5),
    p(-28, -17),
    p(7, -0),
    p(-8, -9),
    p(-27, -9),
    p(-38, -5),
    p(1, -8),
    p(-33, -11),
    p(-26, -13),
    p(-39, 46),
    p(8, 2),
    p(-1, -7),
    p(-4, -10),
    p(-20, -2),
    p(-8, -2),
    p(-12, -16),
    p(-18, -4),
    p(-20, 65),
    p(-5, -10),
    p(-23, -16),
    p(-30, -30),
    p(7, -68),
    p(-9, -11),
    p(-6, -24),
    p(-65, 52),
    p(0, 0),
    p(10, 3),
    p(-3, -3),
    p(-16, -7),
    p(-23, -9),
    p(0, -0),
    p(-23, -20),
    p(-10, -6),
    p(-21, -6),
    p(-1, -6),
    p(-19, -10),
    p(-24, -18),
    p(-31, -10),
    p(-4, -3),
    p(-41, -9),
    p(11, 8),
    p(-48, 47),
    p(4, 1),
    p(-8, -4),
    p(-23, 50),
    p(0, 0),
    p(-9, -6),
    p(-14, -1),
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
    p(-15, 8),   /*0b0000*/
    p(-12, 5),   /*0b0001*/
    p(-4, 9),    /*0b0010*/
    p(-0, 8),    /*0b0011*/
    p(-7, 1),    /*0b0100*/
    p(-20, -3),  /*0b0101*/
    p(-7, 3),    /*0b0110*/
    p(-7, -13),  /*0b0111*/
    p(-1, 4),    /*0b1000*/
    p(-8, 7),    /*0b1001*/
    p(-1, 8),    /*0b1010*/
    p(6, 8),     /*0b1011*/
    p(-4, 1),    /*0b1100*/
    p(-15, 1),   /*0b1101*/
    p(-7, 2),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(1, 7),     /*0b10000*/
    p(5, 4),     /*0b10001*/
    p(20, 5),    /*0b10010*/
    p(1, 5),     /*0b10011*/
    p(-7, 1),    /*0b10100*/
    p(15, 9),    /*0b10101*/
    p(-27, 0),   /*0b10110*/
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
    p(-18, -14), /*0b100101*/
    p(-18, 17),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(13, -5),   /*0b101000*/
    p(0, 5),     /*0b101001*/
    p(10, -7),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-0, -3),   /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(2, 0),     /*0b110000*/
    p(15, 1),    /*0b110001*/
    p(17, -8),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(11, 11),   /*0b110100*/
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
    p(7, -6),    /*0b111111*/
    p(-9, 7),    /*0b00*/
    p(-0, -3),   /*0b01*/
    p(16, -1),   /*0b10*/
    p(-7, -23),  /*0b11*/
    p(17, 3),    /*0b100*/
    p(-8, -3),   /*0b101*/
    p(28, -23),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(33, -5),   /*0b1000*/
    p(-10, -11), /*0b1001*/
    p(9, -29),   /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(-4, -9),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-36, 28),  /*0b1111*/
    p(-3, 9),    /*0b00*/
    p(11, -4),   /*0b01*/
    p(7, -7),    /*0b10*/
    p(7, -30),   /*0b11*/
    p(8, -5),    /*0b100*/
    p(22, -13),  /*0b101*/
    p(-2, -13),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(11, 2),    /*0b1000*/
    p(20, -9),   /*0b1001*/
    p(22, -36),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(5, -19),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-7, -35),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(2, -35);
const STOPPABLE_PASSER: PhasedScore = p(15, -39);
const CLOSE_KING_PASSER: PhasedScore = p(-1, 25);
const IMMOBILE_PASSER: PhasedScore = p(-7, -33);
const PROTECTED_PASSER: PhasedScore = p(7, -0);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -25,   35),    p( -27,   51),    p( -38,   44),    p( -40,   26),    p( -38,   26),    p( -28,   24),    p( -15,   26),    p( -40,   29),
        p( -19,   32),    p( -33,   52),    p( -35,   42),    p( -35,   28),    p( -48,   35),    p( -43,   32),    p( -34,   39),    p( -33,   27),
        p( -12,   43),    p( -20,   42),    p( -32,   43),    p( -27,   41),    p( -37,   45),    p( -33,   45),    p( -35,   52),    p( -55,   53),
        p(   0,   53),    p(  -2,   51),    p(  -0,   42),    p( -12,   50),    p( -26,   57),    p( -18,   60),    p( -19,   63),    p( -40,   65),
        p(   7,   54),    p(  20,   48),    p(   7,   42),    p(  -5,   31),    p(  -1,   43),    p(  -8,   55),    p( -22,   56),    p( -43,   61),
        p(  19,   51),    p(  20,   51),    p(  24,   46),    p(  26,   41),    p(  25,   45),    p(  30,   55),    p(  -0,   60),    p(   4,   59),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(-0, 4), p(4, 8), p(7, 17), p(11, 27), p(18, 54), p(21, 54)];
const UNSUPPORTED_PAWN: PhasedScore = p(-9, -4);
const DOUBLED_PAWN: PhasedScore = p(-7, -23);
const PHALANX: [PhasedScore; 6] = [p(-3, 1), p(4, 4), p(8, 6), p(21, 19), p(58, 59), p(67, 64)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(15, 14), p(7, 16), p(14, 19), p(6, 10), p(-2, 11), p(-48, 12)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(48, 20), p(50, 42), p(58, 9), p(50, -6), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, -4), p(14, 20), p(18, -7), p(15, 10), p(17, -12), p(30, -11)];

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
        p(33, 23),
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
        p(-31, -49),
        p(-17, -33),
        p(-6, -19),
        p(2, -7),
        p(9, 2),
        p(14, 11),
        p(19, 15),
        p(24, 19),
        p(26, 24),
        p(32, 25),
        p(38, 25),
        p(43, 29),
        p(37, 39),
        p(49, 31),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-49, -21),
        p(-41, -6),
        p(-38, 1),
        p(-34, 6),
        p(-35, 14),
        p(-30, 20),
        p(-28, 26),
        p(-25, 30),
        p(-23, 36),
        p(-20, 40),
        p(-16, 43),
        p(-16, 48),
        p(-9, 50),
        p(-2, 49),
        p(1, 46),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(1, -65),
        p(-3, -35),
        p(1, -19),
        p(3, -0),
        p(7, 7),
        p(10, 19),
        p(13, 26),
        p(17, 33),
        p(20, 36),
        p(22, 40),
        p(26, 44),
        p(29, 46),
        p(31, 51),
        p(33, 55),
        p(38, 57),
        p(41, 62),
        p(46, 63),
        p(54, 62),
        p(63, 62),
        p(66, 65),
        p(68, 69),
        p(69, 71),
        p(67, 69),
        p(69, 75),
        p(66, 72),
        p(67, 75),
        p(63, 65),
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
        p(45, 17),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-1, 11), p(21, 24), p(0, 0), p(42, 21), p(50, 64), p(0, 0)],
    [p(-6, 20), p(9, 19), p(16, 16), p(0, 0), p(58, 51), p(0, 0)],
    [p(-4, 17), p(-1, 18), p(-1, 31), p(-4, 23), p(0, 0), p(0, 0)],
    [p(50, 22), p(-22, 23), p(3, 17), p(-14, 23), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 8), p(8, 6), p(6, 10), p(12, 7), p(8, 16), p(3, 8)],
    [p(3, 8), p(11, 20), p(-51, -32), p(9, 12), p(10, 18), p(5, 6)],
    [p(-3, 8), p(8, 12), p(4, 16), p(7, 13), p(4, 33), p(15, -1)],
    [p(-0, 10), p(6, 12), p(5, 8), p(3, 18), p(-71, -68), p(2, -2)],
    [p(24, 5), p(13, 16), p(18, 11), p(3, 10), p(15, -4), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(8, -20), p(18, -8), p(9, -2), p(15, -16), p(-5, 9), p(7, -0)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(9, 2), p(-5, 7), p(14, -6), p(-11, 32)];
const CHECK_STM: PhasedScore = p(38, 21);
const DISCOVERED_CHECK_STM: PhasedScore = p(64, 66);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(10, -5), p(63, 26), p(66, 11), p(66, 67), p(0, 0), p(26, -23)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(5, -17), p(26, 28), p(17, 31), p(41, 9), p(54, 12)];
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
