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

use crate::eval::chess::{FileOpenness, NUM_PAWN_SHIELD_CONFIGURATIONS};
use crate::eval::ScoreType;
use gears::games::chess::pieces::{UncoloredChessPiece, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::Color;
use gears::games::Color::*;
use gears::score::{p, PhasedScore};
use std::fmt::Debug;

#[rustfmt::skip]
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
        p( 128,  181),    p( 132,  179),    p( 125,  181),    p( 134,  162),    p( 124,  165),    p( 124,  168),    p(  87,  187),    p(  82,  187),
        p(  68,  118),    p(  79,  119),    p(  90,  107),    p(  98,  111),    p( 100,  103),    p( 136,   96),    p( 129,  117),    p(  97,  109),
        p(  55,  106),    p(  75,  101),    p(  67,   94),    p(  69,   87),    p(  89,   87),    p(  86,   85),    p(  87,   95),    p(  73,   88),
        p(  47,   95),    p(  65,   96),    p(  67,   85),    p(  79,   83),    p(  80,   85),    p(  84,   79),    p(  76,   87),    p(  57,   79),
        p(  42,   89),    p(  57,   86),    p(  59,   82),    p(  62,   90),    p(  72,   86),    p(  66,   82),    p(  75,   76),    p(  50,   76),
        p(  51,   96),    p(  67,   95),    p(  60,   89),    p(  60,   97),    p(  68,   99),    p(  77,   91),    p(  88,   83),    p(  52,   82),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 211,  276),    p( 204,  311),    p( 244,  324),    p( 273,  312),    p( 312,  312),    p( 208,  310),    p( 232,  307),    p( 243,  257),
        p( 271,  310),    p( 292,  319),    p( 302,  320),    p( 321,  319),    p( 312,  313),    p( 334,  304),    p( 281,  316),    p( 287,  300),
        p( 294,  308),    p( 315,  315),    p( 331,  315),    p( 338,  318),    p( 357,  309),    p( 371,  301),    p( 334,  309),    p( 319,  302),
        p( 306,  316),    p( 321,  321),    p( 325,  322),    p( 344,  325),    p( 336,  322),    p( 355,  318),    p( 337,  318),    p( 344,  304),
        p( 296,  315),    p( 306,  317),    p( 312,  322),    p( 310,  326),    p( 320,  328),    p( 316,  312),    p( 331,  310),    p( 308,  305),
        p( 273,  302),    p( 287,  312),    p( 293,  306),    p( 302,  319),    p( 309,  314),    p( 295,  300),    p( 307,  300),    p( 288,  301),
        p( 268,  305),    p( 276,  313),    p( 284,  312),    p( 296,  315),    p( 299,  311),    p( 292,  307),    p( 290,  301),    p( 288,  310),
        p( 263,  311),    p( 282,  298),    p( 264,  305),    p( 283,  307),    p( 291,  306),    p( 290,  294),    p( 287,  299),    p( 286,  302),
    ],
    // bishop
    [
        p( 285,  325),    p( 257,  331),    p( 251,  326),    p( 231,  333),    p( 247,  330),    p( 238,  326),    p( 294,  321),    p( 268,  321),
        p( 286,  319),    p( 287,  320),    p( 291,  320),    p( 274,  323),    p( 292,  316),    p( 300,  316),    p( 284,  321),    p( 278,  321),
        p( 299,  327),    p( 311,  319),    p( 301,  320),    p( 313,  314),    p( 317,  317),    p( 348,  318),    p( 335,  316),    p( 331,  325),
        p( 293,  324),    p( 296,  323),    p( 307,  318),    p( 325,  322),    p( 312,  321),    p( 310,  321),    p( 297,  323),    p( 302,  322),
        p( 298,  321),    p( 289,  322),    p( 300,  321),    p( 311,  321),    p( 309,  317),    p( 297,  318),    p( 291,  320),    p( 309,  315),
        p( 303,  321),    p( 308,  317),    p( 303,  317),    p( 302,  317),    p( 304,  319),    p( 303,  313),    p( 309,  308),    p( 316,  313),
        p( 317,  322),    p( 312,  311),    p( 312,  309),    p( 294,  319),    p( 300,  317),    p( 301,  314),    p( 319,  311),    p( 308,  307),
        p( 308,  314),    p( 320,  317),    p( 313,  316),    p( 295,  321),    p( 307,  319),    p( 300,  323),    p( 310,  308),    p( 310,  304),
    ],
    // rook
    [
        p( 477,  547),    p( 462,  557),    p( 464,  561),    p( 464,  558),    p( 474,  554),    p( 488,  551),    p( 499,  548),    p( 514,  541),
        p( 451,  554),    p( 449,  559),    p( 463,  559),    p( 479,  549),    p( 470,  550),    p( 487,  544),    p( 495,  541),    p( 509,  532),
        p( 439,  552),    p( 457,  547),    p( 450,  548),    p( 451,  542),    p( 479,  533),    p( 493,  527),    p( 524,  524),    p( 498,  526),
        p( 435,  552),    p( 441,  548),    p( 439,  551),    p( 445,  545),    p( 451,  535),    p( 468,  530),    p( 474,  531),    p( 476,  527),
        p( 429,  551),    p( 428,  549),    p( 428,  550),    p( 436,  547),    p( 441,  542),    p( 445,  538),    p( 459,  531),    p( 451,  530),
        p( 427,  548),    p( 424,  547),    p( 426,  547),    p( 430,  547),    p( 441,  540),    p( 451,  530),    p( 475,  515),    p( 458,  520),
        p( 430,  544),    p( 431,  544),    p( 436,  546),    p( 438,  544),    p( 447,  536),    p( 462,  525),    p( 474,  518),    p( 444,  528),
        p( 440,  547),    p( 435,  542),    p( 435,  547),    p( 442,  542),    p( 447,  536),    p( 455,  535),    p( 454,  530),    p( 449,  534),
    ],
    // queen
    [
        p( 889,  974),    p( 886,  988),    p( 898, 1002),    p( 920,  993),    p( 918, 1000),    p( 937,  988),    p( 990,  936),    p( 940,  969),
        p( 889,  969),    p( 869,  991),    p( 873, 1015),    p( 864, 1031),    p( 873, 1043),    p( 911, 1008),    p( 916,  986),    p( 949,  980),
        p( 892,  963),    p( 888,  973),    p( 887,  994),    p( 892, 1005),    p( 913, 1011),    p( 953,  992),    p( 959,  967),    p( 945,  985),
        p( 879,  974),    p( 885,  977),    p( 886,  989),    p( 888, 1000),    p( 890, 1014),    p( 899, 1009),    p( 904, 1009),    p( 914,  995),
        p( 888,  965),    p( 878,  986),    p( 886,  981),    p( 888,  999),    p( 890,  992),    p( 892,  994),    p( 900,  990),    p( 907,  988),
        p( 886,  958),    p( 894,  965),    p( 890,  977),    p( 886,  976),    p( 892,  980),    p( 898,  977),    p( 913,  964),    p( 908,  961),
        p( 886,  961),    p( 888,  960),    p( 894,  957),    p( 892,  966),    p( 893,  965),    p( 894,  955),    p( 906,  937),    p( 911,  923),
        p( 870,  961),    p( 880,  945),    p( 880,  954),    p( 889,  956),    p( 888,  951),    p( 876,  954),    p( 881,  944),    p( 886,  936),
    ],
    // king
    [
        p( 104,  -76),    p(  31,  -24),    p(  62,  -17),    p( -24,   16),    p(  -2,    3),    p( -14,   14),    p(  39,    4),    p( 167,  -79),
        p( -18,   -0),    p(  34,   -9),    p(  23,    2),    p(  83,   -8),    p(  54,    0),    p(  25,   13),    p(  62,   -2),    p(  17,    0),
        p( -36,    9),    p(  61,  -10),    p(  10,   10),    p(  -4,   21),    p(  33,   14),    p(  58,    7),    p(  34,    3),    p( -28,   13),
        p( -26,    6),    p( -11,   -4),    p( -39,   18),    p( -59,   30),    p( -64,   28),    p( -45,   20),    p( -55,    6),    p(-104,   23),
        p( -44,    5),    p( -30,   -5),    p( -55,   17),    p( -82,   32),    p( -86,   31),    p( -62,   16),    p( -66,    4),    p(-116,   21),
        p( -36,   10),    p(  -5,   -9),    p( -41,   10),    p( -52,   20),    p( -48,   18),    p( -58,   11),    p( -24,   -7),    p( -69,   19),
        p(  28,    0),    p(  14,  -18),    p(   2,   -7),    p( -21,    3),    p( -25,    3),    p( -11,   -5),    p(  27,  -26),    p(  13,    1),
        p( -12,  -10),    p(  18,  -23),    p(  14,  -10),    p( -51,   12),    p(   6,   -8),    p( -47,    7),    p(  14,  -19),    p(  14,  -25),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  28,   81),    p(  32,   79),    p(  25,   81),    p(  34,   62),    p(  24,   65),    p(  24,   68),    p( -13,   87),    p( -18,   87),
        p(  25,  116),    p(  33,  116),    p(  28,   98),    p(  11,   67),    p(  16,   72),    p(   7,   93),    p( -21,  101),    p( -44,  123),
        p(  10,   68),    p(  10,   65),    p(  18,   51),    p(  12,   42),    p(  -4,   44),    p(   7,   53),    p( -11,   70),    p( -17,   73),
        p(  -2,   41),    p( -10,   38),    p( -18,   32),    p( -11,   24),    p( -17,   26),    p( -15,   36),    p( -22,   49),    p( -15,   46),
        p(  -8,   12),    p( -17,   20),    p( -20,   18),    p( -15,    8),    p( -16,   13),    p( -13,   16),    p( -15,   34),    p(   3,   15),
        p( -15,   10),    p( -10,   14),    p( -12,   18),    p( -12,    5),    p(   1,    1),    p(   1,    6),    p(   5,   13),    p(   0,   11),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];

const BISHOP_PAIR: PhasedScore = p(22, 55);
const ROOK_OPEN_FILE: PhasedScore = p(15, 3);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -2);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(2, -1);
const KING_OPEN_FILE: PhasedScore = p(-58, -3);
const KING_CLOSED_FILE: PhasedScore = p(15, -17);
const KING_SEMIOPEN_FILE: PhasedScore = p(-14, 4);
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-8, 3),    /*0b0000*/
    p(-13, 5),   /*0b0001*/
    p(-8, 4),    /*0b0010*/
    p(-4, 19),   /*0b0011*/
    p(1, 1),     /*0b0100*/
    p(-26, -6),  /*0b0101*/
    p(-8, 12),   /*0b0110*/
    p(-12, -6),  /*0b0111*/
    p(9, 4),     /*0b1000*/
    p(-14, -18), /*0b1001*/
    p(-1, 7),    /*0b1010*/
    p(-2, -6),   /*0b1011*/
    p(3, 0),     /*0b1100*/
    p(-33, -22), /*0b1101*/
    p(2, 11),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-3, 10),   /*0b10000*/
    p(9, 7),     /*0b10001*/
    p(-2, -17),  /*0b10010*/
    p(-7, -8),   /*0b10011*/
    p(-2, 3),    /*0b10100*/
    p(12, 8),    /*0b10101*/
    p(-23, -16), /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(24, 38),   /*0b11000*/
    p(28, 5),    /*0b11001*/
    p(34, 19),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(27, 14),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(20, 3),    /*0b100000*/
    p(5, 7),     /*0b100001*/
    p(20, -1),   /*0b100010*/
    p(14, 5),    /*0b100011*/
    p(-21, -27), /*0b100100*/
    p(-32, -39), /*0b100101*/
    p(-23, -2),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(19, -5),   /*0b101000*/
    p(-16, -10), /*0b101001*/
    p(16, -8),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-25, -22), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(29, 25),   /*0b110000*/
    p(37, 18),   /*0b110001*/
    p(23, -14),  /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(6, 6),     /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(37, 32),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(4, -20),   /*0b111111*/
    p(-47, 6),   /*0b00*/
    p(-15, -13), /*0b01*/
    p(14, -3),   /*0b10*/
    p(12, -23),  /*0b11*/
    p(22, -6),   /*0b100*/
    p(-39, -35), /*0b101*/
    p(52, -38),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(41, -8),   /*0b1000*/
    p(-1, -29),  /*0b1001*/
    p(46, -79),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(53, -6),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-1, -11),  /*0b1111*/
    p(-12, 7),   /*0b00*/
    p(5, -9),    /*0b01*/
    p(-3, -11),  /*0b10*/
    p(4, -23),   /*0b11*/
    p(10, -4),   /*0b100*/
    p(17, -44),  /*0b101*/
    p(-3, -18),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(6, 3),     /*0b1000*/
    p(27, -15),  /*0b1001*/
    p(12, -63),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(24, -0),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-5, -53),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(14, 17), p(7, 13), p(6, 19), p(5, 5), p(-5, 15), p(-49, 7)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(39, 10),
    p(34, 34),
    p(52, -6),
    p(35, -28),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(0, 0),
        p(0, 0),
        p(-58, -14),
        p(-18, 2),
        p(-13, 9),
        p(0, 0),
        p(-1, 16),
        p(0, 0),
        p(17, 26),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(0, 0),
        p(-57, -44),
        p(-35, -37),
        p(-26, -11),
        p(-10, -2),
        p(-4, 5),
        p(9, 18),
        p(15, 22),
        p(24, 28),
        p(26, 33),
        p(32, 37),
        p(36, 37),
        p(42, 38),
        p(63, 33),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(0, 0),
        p(0, 0),
        p(-76, 18),
        p(-71, 31),
        p(-67, 38),
        p(-63, 43),
        p(-61, 46),
        p(-59, 52),
        p(-54, 52),
        p(-49, 55),
        p(-45, 58),
        p(-41, 60),
        p(-37, 63),
        p(-29, 65),
        p(-19, 61),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(-41, 26),
        p(-51, -84),
        p(-20, -29),
        p(-22, 7),
        p(-25, 58),
        p(-20, 74),
        p(-20, 91),
        p(-16, 97),
        p(-13, 108),
        p(-10, 118),
        p(-8, 121),
        p(-4, 125),
        p(-3, 132),
        p(-2, 137),
        p(1, 142),
        p(0, 149),
        p(1, 153),
        p(2, 161),
        p(4, 162),
        p(6, 163),
        p(13, 161),
        p(12, 163),
        p(24, 161),
        p(57, 142),
        p(115, 114),
    ],
    [
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(13, -24),
        p(0, 0),
        p(11, -15),
        p(0, 0),
        p(0, 0),
        p(-27, 18),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [
        p(-12, 5),
        p(0, 0),
        p(24, 19),
        p(47, -10),
        p(20, -32),
        p(0, 0),
    ],
    [p(-6, 7), p(17, 23), p(0, 0), p(31, 4), p(29, 57), p(0, 0)],
    [p(-2, 12), p(9, 12), p(16, 10), p(0, 0), p(44, -6), p(0, 0)],
    [p(-5, 7), p(-1, 6), p(-4, 23), p(1, 2), p(0, 0), p(0, 0)],
    [
        p(44, 35),
        p(-45, 20),
        p(-17, 28),
        p(-47, 16),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(-4, 2), p(1, 0), p(-2, 4), p(4, 1), p(-1, 14), p(2, -0)],
    [p(0, -7), p(10, 13), p(-30, -24), p(0, 8), p(4, 16), p(0, 1)],
    [p(2, -5), p(12, -0), p(9, 4), p(9, 3), p(11, 11), p(18, -8)],
    [
        p(-2, -3),
        p(5, -3),
        p(4, -8),
        p(-1, 13),
        p(-67, -256),
        p(4, -13),
    ],
    [p(30, -1), p(13, 6), p(18, 2), p(-5, 6), p(7, -9), p(0, 0)],
];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static {
    type Score: ScoreType;

    fn psqt(square: ChessSquare, piece: UncoloredChessPiece, color: Color) -> Self::Score;
    fn passed_pawn(square: ChessSquare) -> Self::Score;
    fn bishop_pair() -> Self::Score;
    fn rook_openness(openness: FileOpenness) -> Self::Score;
    fn king_openness(openness: FileOpenness) -> Self::Score;
    fn pawn_shield(config: usize) -> Self::Score;
    fn pawn_protection(piece: UncoloredChessPiece) -> Self::Score;
    fn pawn_attack(piece: UncoloredChessPiece) -> Self::Score;
    fn mobility(piece: UncoloredChessPiece, mobility: usize) -> Self::Score;
    fn threats(attacking: UncoloredChessPiece, targeted: UncoloredChessPiece) -> Self::Score;
    fn defended(protecting: UncoloredChessPiece, target: UncoloredChessPiece) -> Self::Score;
}

/// Eval values tuned on a combination of the zurichess dataset and a dataset used by 4ku,
/// created by GCP using his engine Stoofvlees and filtered by cj5716 using Stockfish at depth 9,
/// using my own tuner `pliers`.
#[derive(Debug, Default, Copy, Clone)]
pub struct Lite {}

impl LiteValues for Lite {
    type Score = PhasedScore;

    fn psqt(square: ChessSquare, piece: UncoloredChessPiece, color: Color) -> Self::Score {
        PSQTS[piece as usize][square.flip_if(color == White).bb_idx()]
    }

    fn passed_pawn(square: ChessSquare) -> Self::Score {
        PASSED_PAWNS[square.bb_idx()]
    }

    fn bishop_pair() -> Self::Score {
        BISHOP_PAIR
    }

    fn rook_openness(openness: FileOpenness) -> Self::Score {
        match openness {
            FileOpenness::Open => ROOK_OPEN_FILE,
            FileOpenness::Closed => ROOK_CLOSED_FILE,
            FileOpenness::SemiOpen => ROOK_SEMIOPEN_FILE,
            FileOpenness::SemiClosed => Self::Score::default(),
        }
    }

    fn king_openness(openness: FileOpenness) -> Self::Score {
        match openness {
            FileOpenness::Open => KING_OPEN_FILE,
            FileOpenness::Closed => KING_CLOSED_FILE,
            FileOpenness::SemiOpen => KING_SEMIOPEN_FILE,
            FileOpenness::SemiClosed => Self::Score::default(),
        }
    }

    fn pawn_shield(config: usize) -> Self::Score {
        PAWN_SHIELDS[config]
    }

    fn pawn_protection(piece: UncoloredChessPiece) -> Self::Score {
        PAWN_PROTECTION[piece as usize]
    }

    fn pawn_attack(piece: UncoloredChessPiece) -> Self::Score {
        PAWN_ATTACKS[piece as usize]
    }

    fn mobility(piece: UncoloredChessPiece, mobility: usize) -> Self::Score {
        MOBILITY[piece as usize - 1][mobility]
    }

    fn threats(attacking: UncoloredChessPiece, targeted: UncoloredChessPiece) -> Self::Score {
        THREATS[attacking as usize - 1][targeted as usize]
    }
    fn defended(protecting: UncoloredChessPiece, target: UncoloredChessPiece) -> Self::Score {
        DEFENDED[protecting as usize - 1][target as usize]
    }
}
