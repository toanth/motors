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
        p( 131,  180),    p( 133,  178),    p( 128,  180),    p( 140,  159),    p( 128,  163),    p( 125,  167),    p(  81,  187),    p(  81,  187),
        p(  68,  119),    p(  79,  119),    p(  93,  106),    p( 101,  110),    p( 106,  102),    p( 140,   95),    p( 130,  117),    p(  99,  110),
        p(  55,  106),    p(  74,  101),    p(  70,   93),    p(  71,   87),    p(  92,   87),    p(  89,   85),    p(  87,   96),    p(  75,   88),
        p(  47,   95),    p(  64,   97),    p(  66,   86),    p(  77,   84),    p(  77,   86),    p(  83,   79),    p(  77,   88),    p(  59,   79),
        p(  41,   89),    p(  55,   87),    p(  56,   83),    p(  59,   91),    p(  69,   88),    p(  63,   84),    p(  77,   78),    p(  53,   76),
        p(  49,   96),    p(  68,   95),    p(  61,   91),    p(  53,  100),    p(  69,  101),    p(  79,   93),    p(  93,   84),    p(  53,   84),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 149,  236),    p( 188,  287),    p( 231,  313),    p( 263,  301),    p( 303,  301),    p( 219,  294),    p( 221,  283),    p( 189,  213),
        p( 256,  286),    p( 280,  307),    p( 302,  322),    p( 323,  321),    p( 313,  316),    p( 348,  303),    p( 276,  302),    p( 278,  275),
        p( 281,  297),    p( 311,  319),    p( 341,  336),    p( 350,  338),    p( 371,  327),    p( 388,  320),    p( 331,  311),    p( 310,  289),
        p( 291,  304),    p( 317,  323),    p( 335,  343),    p( 357,  343),    p( 350,  340),    p( 366,  336),    p( 334,  318),    p( 330,  291),
        p( 282,  302),    p( 302,  321),    p( 323,  342),    p( 322,  344),    p( 331,  347),    p( 328,  332),    p( 323,  314),    p( 293,  292),
        p( 265,  289),    p( 290,  313),    p( 312,  327),    p( 319,  339),    p( 329,  334),    p( 316,  319),    p( 305,  306),    p( 280,  287),
        p( 256,  280),    p( 269,  299),    p( 288,  313),    p( 300,  316),    p( 302,  311),    p( 299,  308),    p( 287,  286),    p( 279,  283),
        p( 213,  267),    p( 262,  272),    p( 255,  295),    p( 274,  294),    p( 278,  296),    p( 285,  282),    p( 268,  273),    p( 233,  260),
    ],
    // bishop
    [
        p( 275,  321),    p( 257,  327),    p( 250,  322),    p( 224,  332),    p( 240,  328),    p( 238,  323),    p( 297,  316),    p( 259,  317),
        p( 285,  313),    p( 288,  317),    p( 296,  319),    p( 280,  323),    p( 295,  318),    p( 307,  315),    p( 284,  319),    p( 274,  315),
        p( 299,  321),    p( 316,  315),    p( 306,  320),    p( 326,  313),    p( 328,  316),    p( 352,  318),    p( 336,  313),    p( 331,  321),
        p( 294,  318),    p( 300,  321),    p( 318,  316),    p( 335,  321),    p( 326,  319),    p( 322,  320),    p( 303,  322),    p( 307,  315),
        p( 297,  316),    p( 300,  319),    p( 313,  318),    p( 324,  317),    p( 318,  316),    p( 313,  316),    p( 305,  316),    p( 308,  310),
        p( 300,  316),    p( 314,  311),    p( 314,  315),    p( 312,  314),    p( 316,  316),    p( 312,  310),    p( 314,  303),    p( 309,  307),
        p( 316,  313),    p( 315,  306),    p( 323,  304),    p( 310,  315),    p( 315,  314),    p( 319,  306),    p( 320,  308),    p( 313,  299),
        p( 299,  305),    p( 316,  309),    p( 307,  311),    p( 299,  315),    p( 306,  313),    p( 300,  316),    p( 313,  299),    p( 297,  296),
    ],
    // rook
    [
        p( 470,  544),    p( 459,  554),    p( 463,  557),    p( 464,  553),    p( 474,  550),    p( 487,  547),    p( 499,  544),    p( 512,  537),
        p( 454,  550),    p( 458,  553),    p( 477,  552),    p( 494,  542),    p( 485,  543),    p( 502,  537),    p( 505,  535),    p( 512,  527),
        p( 443,  547),    p( 470,  541),    p( 465,  542),    p( 468,  536),    p( 496,  526),    p( 509,  521),    p( 539,  518),    p( 502,  522),
        p( 441,  548),    p( 453,  543),    p( 455,  545),    p( 462,  539),    p( 470,  529),    p( 483,  525),    p( 489,  526),    p( 484,  522),
        p( 435,  545),    p( 439,  544),    p( 441,  545),    p( 451,  540),    p( 457,  536),    p( 457,  533),    p( 473,  526),    p( 461,  524),
        p( 434,  541),    p( 435,  540),    p( 441,  539),    p( 449,  538),    p( 459,  531),    p( 463,  525),    p( 492,  509),    p( 470,  512),
        p( 435,  535),    p( 441,  536),    p( 448,  538),    p( 448,  536),    p( 459,  528),    p( 473,  519),    p( 482,  512),    p( 446,  521),
        p( 453,  534),    p( 449,  531),    p( 451,  536),    p( 459,  531),    p( 466,  525),    p( 467,  528),    p( 466,  522),    p( 460,  521),
    ],
    // queen
    [
        p( 861,  971),    p( 870,  982),    p( 887,  996),    p( 910,  989),    p( 908,  994),    p( 928,  981),    p( 973,  929),    p( 914,  963),
        p( 880,  954),    p( 870,  980),    p( 879, 1007),    p( 869, 1025),    p( 877, 1037),    p( 915, 1000),    p( 912,  979),    p( 937,  964),
        p( 888,  948),    p( 888,  967),    p( 895,  987),    p( 903,  998),    p( 920, 1006),    p( 953,  991),    p( 957,  963),    p( 937,  970),
        p( 876,  960),    p( 888,  970),    p( 894,  982),    p( 897,  995),    p( 900, 1009),    p( 906, 1004),    p( 905, 1003),    p( 912,  981),
        p( 884,  952),    p( 881,  978),    p( 890,  977),    p( 893,  996),    p( 897,  987),    p( 896,  989),    p( 903,  980),    p( 903,  972),
        p( 880,  945),    p( 891,  956),    p( 891,  971),    p( 890,  969),    p( 897,  971),    p( 901,  968),    p( 912,  951),    p( 903,  940),
        p( 878,  943),    p( 888,  944),    p( 895,  941),    p( 894,  951),    p( 896,  948),    p( 902,  931),    p( 905,  916),    p( 903,  895),
        p( 873,  931),    p( 874,  928),    p( 880,  936),    p( 893,  931),    p( 890,  928),    p( 877,  928),    p( 879,  919),    p( 877,  910),
    ],
    // king
    [
        p(  98,  -92),    p(  44,  -45),    p(  69,  -37),    p( -16,   -4),    p(   8,  -16),    p(   1,   -7),    p(  57,  -16),    p( 183,  -96),
        p( -18,   -8),    p(   3,   13),    p( -11,   22),    p(  46,   14),    p(  19,   22),    p(  -3,   34),    p(  39,   18),    p(  35,   -7),
        p( -34,    2),    p(  38,    9),    p( -15,   29),    p( -29,   40),    p(  11,   33),    p(  42,   25),    p(  17,   22),    p( -15,    6),
        p( -22,   -1),    p( -31,   15),    p( -56,   36),    p( -71,   46),    p( -75,   44),    p( -54,   36),    p( -63,   23),    p( -92,   17),
        p( -43,   -2),    p( -46,   14),    p( -71,   35),    p( -94,   49),    p( -96,   48),    p( -73,   33),    p( -76,   22),    p(-109,   16),
        p( -32,    4),    p( -12,    9),    p( -59,   28),    p( -70,   38),    p( -67,   37),    p( -67,   28),    p( -32,   12),    p( -60,   13),
        p(  22,   -4),    p(   1,    1),    p( -20,   12),    p( -47,   22),    p( -47,   22),    p( -33,   13),    p(  12,   -5),    p(   7,   -1),
        p( -23,  -24),    p(  31,  -42),    p(  19,  -27),    p( -46,   -7),    p(  11,  -28),    p( -46,  -10),    p(  22,  -37),    p(   8,  -37),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  31,   80),    p(  33,   78),    p(  28,   80),    p(  40,   59),    p(  28,   63),    p(  25,   67),    p( -19,   87),    p( -19,   87),
        p(  30,  114),    p(  38,  114),    p(  30,   96),    p(  16,   65),    p(  16,   70),    p(   7,   91),    p( -21,  100),    p( -43,  121),
        p(  13,   68),    p(  14,   64),    p(  20,   50),    p(  14,   41),    p(  -2,   42),    p(   8,   52),    p( -10,   68),    p( -16,   72),
        p(   1,   40),    p(  -7,   38),    p( -16,   32),    p( -11,   23),    p( -16,   26),    p( -12,   35),    p( -20,   47),    p( -14,   45),
        p(  -5,   12),    p( -13,   19),    p( -18,   18),    p( -15,    8),    p( -15,   13),    p(  -7,   14),    p( -12,   31),    p(   5,   14),
        p( -12,   11),    p(  -7,   14),    p( -12,   17),    p( -11,    5),    p(   3,   -0),    p(   4,    5),    p(   7,   12),    p(   1,   11),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];

const BISHOP_PAIR: PhasedScore = p(17, 53);
const ROOK_OPEN_FILE: PhasedScore = p(21, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -4);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(8, 0);
const KING_OPEN_FILE: PhasedScore = p(-58, -4);
const KING_CLOSED_FILE: PhasedScore = p(16, -16);
const KING_SEMIOPEN_FILE: PhasedScore = p(-12, 5);
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-45, 8),   /*0b0000*/
    p(-25, 7),   /*0b0001*/
    p(-13, 5),   /*0b0010*/
    p(6, 22),    /*0b0011*/
    p(-12, 3),   /*0b0100*/
    p(-17, -5),  /*0b0101*/
    p(3, 13),    /*0b0110*/
    p(21, -3),   /*0b0111*/
    p(-30, 9),   /*0b1000*/
    p(-19, -18), /*0b1001*/
    p(-13, 8),   /*0b1010*/
    p(11, -4),   /*0b1011*/
    p(-11, 0),   /*0b1100*/
    p(-16, -24), /*0b1101*/
    p(11, 10),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-41, 14),  /*0b10000*/
    p(-4, 7),    /*0b10001*/
    p(-12, -18), /*0b10010*/
    p(3, -9),    /*0b10011*/
    p(-16, 3),   /*0b10100*/
    p(20, 9),    /*0b10101*/
    p(-14, -15), /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(-14, 41),  /*0b11000*/
    p(22, 4),    /*0b11001*/
    p(24, 17),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(10, 13),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(-18, 7),   /*0b100000*/
    p(-8, 8),    /*0b100001*/
    p(7, -0),    /*0b100010*/
    p(23, 5),    /*0b100011*/
    p(-33, -26), /*0b100100*/
    p(-20, -40), /*0b100101*/
    p(-10, -2),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(-22, -1),  /*0b101000*/
    p(-17, -14), /*0b101001*/
    p(1, -8),    /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-40, -21), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(-9, 29),   /*0b110000*/
    p(23, 20),   /*0b110001*/
    p(10, -14),  /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(-4, 6),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(-4, 35),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(15, -17),  /*0b111111*/
    p(-65, -4),  /*0b00*/
    p(-7, -23),  /*0b01*/
    p(17, -14),  /*0b10*/
    p(39, -30),  /*0b11*/
    p(2, -17),   /*0b100*/
    p(-38, -45), /*0b101*/
    p(53, -49),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(20, -18),  /*0b1000*/
    p(3, -41),   /*0b1001*/
    p(50, -91),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(33, -19),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(18, -14),  /*0b1111*/
    p(-36, -3),  /*0b00*/
    p(-0, -19),  /*0b01*/
    p(-1, -21),  /*0b10*/
    p(24, -31),  /*0b11*/
    p(-19, -16), /*0b100*/
    p(11, -55),  /*0b101*/
    p(-5, -31),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(-18, -9),  /*0b1000*/
    p(19, -26),  /*0b1001*/
    p(11, -75),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(-1, -16),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(9, -62),   /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(15, 17), p(9, 15), p(4, 17), p(7, 6), p(-6, 17), p(-28, 6)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(39, 12),
    p(38, 34),
    p(53, -7),
    p(39, -35),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-12, 41),
        p(-2, 37),
        p(1, 26),
        p(4, 24),
        p(5, 19),
        p(7, 17),
        p(11, 10),
        p(15, 7),
        p(21, -0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-22, -28),
        p(-5, -23),
        p(2, -9),
        p(9, 2),
        p(15, 11),
        p(21, 18),
        p(26, 24),
        p(32, 27),
        p(35, 31),
        p(40, 33),
        p(42, 34),
        p(52, 35),
        p(62, 33),
        p(78, 34),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-74, 41),
        p(-69, 48),
        p(-66, 51),
        p(-63, 53),
        p(-58, 54),
        p(-52, 55),
        p(-49, 56),
        p(-45, 60),
        p(-42, 62),
        p(-35, 61),
        p(-31, 61),
        p(-26, 62),
        p(-23, 61),
        p(-17, 60),
        p(10, 57),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-24, 105),
        p(-18, 105),
        p(-15, 113),
        p(-13, 121),
        p(-13, 131),
        p(-12, 133),
        p(-13, 140),
        p(-12, 145),
        p(-13, 151),
        p(-14, 156),
        p(-14, 158),
        p(-17, 166),
        p(-17, 168),
        p(-18, 173),
        p(-16, 176),
        p(-16, 182),
        p(-13, 183),
        p(-3, 176),
        p(13, 173),
        p(37, 157),
        p(93, 131),
        p(176, 94),
        p(292, 32),
        p(447, -36),
        p(763, -194),
        p(1033, -316),
        p(1115, -382),
        p(1102, -455),
    ],
    [
        p(9, -9),
        p(4, -3),
        p(-5, 4),
        p(-22, 9),
        p(-28, 9),
        p(-25, 2),
        p(-30, 3),
        p(-42, 4),
        p(8, -15),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-14, 8),
        p(6, -3),
        p(24, 22),
        p(46, -11),
        p(18, -32),
        p(0, 0),
    ],
    [
        p(-7, 8),
        p(15, 22),
        p(-7, -27),
        p(31, 3),
        p(29, 51),
        p(0, 0),
    ],
    [
        p(-8, 12),
        p(2, 9),
        p(11, 7),
        p(-20, -39),
        p(35, -6),
        p(0, 0),
    ],
    [
        p(-4, 2),
        p(-1, -2),
        p(-2, 15),
        p(2, -4),
        p(12, -83),
        p(0, 0),
    ],
    [
        p(47, 33),
        p(-34, 18),
        p(-22, 31),
        p(-49, 17),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(3, -6), p(10, -6), p(3, 1), p(4, -0), p(1, 14), p(5, -7)],
    [
        p(4, -1),
        p(11, 16),
        p(-15, -43),
        p(5, 14),
        p(8, 21),
        p(12, 5),
    ],
    [p(4, -5), p(9, -2), p(5, 2), p(6, 5), p(4, 25), p(8, -5)],
    [
        p(1, -1),
        p(5, 3),
        p(7, -15),
        p(1, 8),
        p(-58, -270),
        p(-4, -3),
    ],
    [p(4, -1), p(9, 2), p(11, -1), p(-4, 1), p(2, -15), p(0, 0)],
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
