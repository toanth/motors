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
use crate::eval::{ScoreType, SingleFeatureScore};
use gears::games::chess::pieces::{ChessPieceType, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::chess::ChessColor;
use gears::games::chess::ChessColor::White;
use gears::general::common::StaticallyNamedEntity;
use gears::score::{p, PhasedScore};
use std::fmt::{Debug, Display};

#[rustfmt::skip]
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
        p( 133,  186),    p( 130,  185),    p( 120,  188),    p( 133,  169),    p( 118,  173),    p( 119,  176),    p(  84,  194),    p(  89,  192),
        p(  64,  123),    p(  63,  124),    p(  74,  119),    p(  82,  123),    p(  65,  122),    p( 114,  108),    p(  94,  130),    p(  85,  120),
        p(  51,  113),    p(  63,  109),    p(  61,  103),    p(  66,   96),    p(  82,   97),    p(  83,   93),    p(  78,  103),    p(  71,   95),
        p(  48,  100),    p(  55,  102),    p(  64,   95),    p(  73,   94),    p(  77,   92),    p(  78,   88),    p(  71,   92),    p(  60,   85),
        p(  43,   97),    p(  52,   94),    p(  56,   94),    p(  59,  100),    p(  68,   97),    p(  62,   93),    p(  70,   84),    p(  54,   85),
        p(  50,   98),    p(  51,   97),    p(  58,   98),    p(  58,  105),    p(  54,  108),    p(  73,   98),    p(  73,   84),    p(  55,   87),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 185,  267),    p( 210,  300),    p( 244,  312),    p( 269,  302),    p( 301,  304),    p( 214,  299),    p( 231,  299),    p( 214,  249),
        p( 276,  301),    p( 288,  310),    p( 301,  307),    p( 314,  309),    p( 305,  306),    p( 328,  294),    p( 287,  305),    p( 291,  292),
        p( 292,  298),    p( 303,  303),    p( 320,  312),    p( 322,  316),    p( 339,  309),    p( 362,  299),    p( 314,  300),    p( 310,  296),
        p( 305,  307),    p( 311,  305),    p( 318,  317),    p( 344,  319),    p( 321,  320),    p( 335,  317),    p( 316,  308),    p( 334,  301),
        p( 302,  310),    p( 300,  304),    p( 307,  317),    p( 313,  320),    p( 320,  321),    p( 317,  309),    p( 328,  300),    p( 317,  306),
        p( 276,  297),    p( 278,  300),    p( 285,  300),    p( 291,  314),    p( 299,  311),    p( 284,  295),    p( 299,  291),    p( 295,  300),
        p( 272,  302),    p( 282,  305),    p( 279,  301),    p( 290,  305),    p( 293,  300),    p( 286,  297),    p( 296,  297),    p( 291,  311),
        p( 245,  297),    p( 283,  296),    p( 267,  298),    p( 287,  303),    p( 297,  301),    p( 292,  290),    p( 290,  296),    p( 269,  296),
    ],
    // bishop
    [
        p( 288,  312),    p( 256,  311),    p( 251,  306),    p( 230,  312),    p( 228,  312),    p( 228,  306),    p( 283,  301),    p( 260,  306),
        p( 281,  299),    p( 286,  305),    p( 288,  302),    p( 287,  305),    p( 289,  301),    p( 294,  298),    p( 273,  307),    p( 276,  300),
        p( 301,  308),    p( 303,  300),    p( 299,  308),    p( 304,  298),    p( 308,  301),    p( 336,  307),    p( 319,  298),    p( 315,  310),
        p( 286,  308),    p( 304,  307),    p( 301,  301),    p( 316,  307),    p( 310,  303),    p( 307,  304),    p( 307,  305),    p( 288,  308),
        p( 297,  305),    p( 286,  309),    p( 301,  305),    p( 313,  304),    p( 311,  304),    p( 299,  302),    p( 296,  306),    p( 317,  297),
        p( 296,  306),    p( 305,  305),    p( 304,  308),    p( 305,  305),    p( 308,  306),    p( 307,  305),    p( 306,  297),    p( 313,  299),
        p( 310,  307),    p( 303,  300),    p( 311,  298),    p( 302,  307),    p( 308,  306),    p( 304,  301),    p( 314,  300),    p( 304,  295),
        p( 302,  302),    p( 315,  305),    p( 309,  306),    p( 295,  309),    p( 308,  306),    p( 299,  312),    p( 305,  294),    p( 309,  293),
    ],
    // rook
    [
        p( 467,  538),    p( 458,  547),    p( 455,  553),    p( 454,  550),    p( 466,  546),    p( 485,  541),    p( 493,  540),    p( 504,  533),
        p( 442,  544),    p( 439,  549),    p( 448,  550),    p( 464,  540),    p( 454,  542),    p( 474,  537),    p( 485,  533),    p( 499,  524),
        p( 447,  540),    p( 465,  536),    p( 463,  537),    p( 467,  533),    p( 494,  522),    p( 502,  518),    p( 525,  515),    p( 497,  517),
        p( 444,  540),    p( 451,  536),    p( 452,  539),    p( 458,  533),    p( 467,  525),    p( 476,  520),    p( 482,  522),    p( 479,  517),
        p( 440,  536),    p( 439,  534),    p( 440,  535),    p( 447,  532),    p( 453,  529),    p( 447,  528),    p( 467,  520),    p( 456,  519),
        p( 437,  532),    p( 436,  530),    p( 439,  529),    p( 441,  530),    p( 448,  523),    p( 457,  516),    p( 479,  503),    p( 461,  507),
        p( 439,  526),    p( 443,  527),    p( 449,  528),    p( 452,  526),    p( 459,  519),    p( 474,  509),    p( 482,  504),    p( 451,  513),
        p( 448,  530),    p( 445,  526),    p( 446,  531),    p( 451,  527),    p( 458,  521),    p( 464,  520),    p( 462,  517),    p( 456,  518),
    ],
    // queen
    [
        p( 878,  957),    p( 880,  971),    p( 895,  985),    p( 911,  981),    p( 910,  984),    p( 930,  972),    p( 980,  920),    p( 926,  951),
        p( 887,  950),    p( 863,  982),    p( 865, 1009),    p( 857, 1027),    p( 865, 1037),    p( 904,  998),    p( 907,  978),    p( 950,  956),
        p( 895,  955),    p( 887,  975),    p( 887,  997),    p( 885, 1006),    p( 908, 1008),    p( 947,  992),    p( 955,  960),    p( 942,  966),
        p( 880,  969),    p( 886,  979),    p( 880,  989),    p( 879, 1004),    p( 884, 1013),    p( 896, 1004),    p( 906, 1002),    p( 913,  979),
        p( 892,  959),    p( 878,  979),    p( 884,  982),    p( 884, 1000),    p( 885,  997),    p( 888,  996),    p( 902,  981),    p( 909,  973),
        p( 887,  944),    p( 893,  962),    p( 886,  978),    p( 883,  981),    p( 888,  988),    p( 894,  978),    p( 909,  960),    p( 909,  946),
        p( 890,  942),    p( 887,  952),    p( 894,  955),    p( 893,  968),    p( 894,  968),    p( 896,  951),    p( 906,  928),    p( 916,  901),
        p( 876,  940),    p( 887,  929),    p( 887,  944),    p( 895,  946),    p( 898,  938),    p( 886,  939),    p( 887,  928),    p( 890,  916),
    ],
    // king
    [
        p(  96,  -72),    p(  43,  -29),    p(  67,  -21),    p( -11,   11),    p(  13,   -1),    p(  -4,    9),    p(  49,   -0),    p( 170,  -78),
        p(  -7,   -4),    p(  21,    2),    p(  12,   12),    p(  79,    1),    p(  47,   10),    p(  19,   23),    p(  53,    9),    p(  32,   -6),
        p( -29,    5),    p(  52,   -2),    p(   8,   17),    p(   4,   24),    p(  38,   19),    p(  69,   11),    p(  33,    9),    p( -10,    7),
        p( -12,   -0),    p(  -3,   -3),    p( -19,   16),    p( -38,   25),    p( -38,   23),    p( -23,   15),    p( -28,    4),    p( -79,   13),
        p( -31,   -3),    p( -23,   -7),    p( -34,   10),    p( -57,   24),    p( -64,   22),    p( -40,    7),    p( -52,   -2),    p( -97,   11),
        p( -22,    2),    p(   2,  -12),    p( -28,    3),    p( -37,   13),    p( -32,   11),    p( -45,    4),    p( -15,  -11),    p( -56,    9),
        p(  43,   -8),    p(  20,  -18),    p(   7,   -9),    p( -13,    1),    p( -19,    1),    p(  -3,   -8),    p(  28,  -24),    p(  23,   -6),
        p( -10,  -10),    p(  27,  -25),    p(  21,  -13),    p( -40,    7),    p(  13,  -11),    p( -36,    4),    p(  20,  -21),    p(  10,  -23),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(16, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(0, 2);
const KING_OPEN_FILE: PhasedScore = p(-56, -1);
const KING_CLOSED_FILE: PhasedScore = p(16, -16);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 5);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-13, 6), p(-3, 7), p(-1, 8), p(3, 8), p(2, 10), p(4, 13), p(13, 13), p(23, 7)],
    // Closed
    [p(-3, 7), p(-1, 8), p(3, 8), p(2, 10), p(4, 13), p(13, 13), p(23, 7), p(0, 0)],
    // SemiOpen
    [p(-1, 8), p(3, 8), p(2, 10), p(4, 13), p(13, 13), p(23, 7), p(0, 0), p(0, 0)],
    // SemiClosed
    [p(3, 8), p(2, 10), p(4, 13), p(13, 13), p(23, 7), p(0, 0), p(0, 0), p(13, -31)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-6, 2),    /*0b0000*/
    p(-14, 7),   /*0b0001*/
    p(-2, 3),    /*0b0010*/
    p(-8, 10),   /*0b0011*/
    p(-3, 2),    /*0b0100*/
    p(-25, 1),   /*0b0101*/
    p(-13, 2),   /*0b0110*/
    p(-17, -21), /*0b0111*/
    p(7, 5),     /*0b1000*/
    p(-4, 6),    /*0b1001*/
    p(3, 4),     /*0b1010*/
    p(-1, 6),    /*0b1011*/
    p(-0, 2),    /*0b1100*/
    p(-24, 6),   /*0b1101*/
    p(-11, 0),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(3, 14),    /*0b10000*/
    p(6, 8),     /*0b10001*/
    p(23, 8),    /*0b10010*/
    p(-1, 5),    /*0b10011*/
    p(-4, 4),    /*0b10100*/
    p(15, 14),   /*0b10101*/
    p(-20, -2),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(13, 29),   /*0b11000*/
    p(32, 21),   /*0b11001*/
    p(43, 35),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(17, 9),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(16, 5),    /*0b100000*/
    p(5, 10),    /*0b100001*/
    p(27, -2),   /*0b100010*/
    p(8, -3),    /*0b100011*/
    p(-9, -2),   /*0b100100*/
    p(-22, -11), /*0b100101*/
    p(-23, 12),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(20, -2),   /*0b101000*/
    p(-2, 13),   /*0b101001*/
    p(21, -8),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-5, 1),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(15, 17),   /*0b110000*/
    p(27, 13),   /*0b110001*/
    p(34, 7),    /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(9, 27),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(25, 12),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(7, -6),    /*0b111111*/
    p(-49, 7),   /*0b00*/
    p(-20, -8),  /*0b01*/
    p(8, 3),     /*0b10*/
    p(-5, -34),  /*0b11*/
    p(17, -2),   /*0b100*/
    p(-33, -12), /*0b101*/
    p(45, -33),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(28, -4),   /*0b1000*/
    p(-9, -27),  /*0b1001*/
    p(50, -49),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(26, -10),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-14, 10),  /*0b1111*/
    p(-17, 10),  /*0b00*/
    p(-0, -0),   /*0b01*/
    p(-7, -6),   /*0b10*/
    p(-9, -33),  /*0b11*/
    p(-1, 2),    /*0b100*/
    p(21, -10),  /*0b101*/
    p(-9, -13),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(4, 7),     /*0b1000*/
    p(22, -7),   /*0b1001*/
    p(19, -33),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(8, -11),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-10, -35), /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  33,   86),    p(  30,   85),    p(  20,   88),    p(  33,   69),    p(  18,   73),    p(  19,   76),    p( -16,   94),    p( -11,   92),
        p(  42,  122),    p(  48,  122),    p(  38,   99),    p(  22,   67),    p(  36,   67),    p(  16,   96),    p(   1,  103),    p( -28,  124),
        p(  24,   72),    p(  18,   70),    p(  24,   53),    p(  16,   43),    p(  -2,   45),    p(   7,   58),    p( -10,   74),    p( -10,   77),
        p(   8,   45),    p(  -3,   43),    p( -15,   34),    p(  -9,   24),    p( -17,   29),    p( -11,   37),    p( -19,   53),    p( -11,   49),
        p(   2,   14),    p( -12,   22),    p( -15,   16),    p( -16,    8),    p( -14,   13),    p(  -8,   16),    p( -14,   36),    p(   9,   16),
        p(  -4,   15),    p(  -2,   20),    p(  -9,   16),    p(  -8,    5),    p(   6,    1),    p(   7,    6),    p(  12,   18),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-11, -10);
const DOUBLED_PAWN: PhasedScore = p(-7, -21);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 7), p(2, 9), p(10, 14), p(9, 9), p(-5, 19), p(-46, 8)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(38, 9), p(42, 35), p(51, -9), p(37, -39), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-58, -54),
        p(-35, -15),
        p(-19, 6),
        p(-7, 17),
        p(3, 26),
        p(13, 34),
        p(24, 34),
        p(34, 32),
        p(42, 28),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-28, -46),
        p(-16, -29),
        p(-7, -13),
        p(0, -2),
        p(6, 7),
        p(11, 15),
        p(13, 19),
        p(16, 23),
        p(16, 27),
        p(22, 28),
        p(26, 26),
        p(34, 27),
        p(27, 35),
        p(40, 27),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-83, 27),
        p(-74, 41),
        p(-70, 46),
        p(-67, 50),
        p(-67, 57),
        p(-61, 61),
        p(-58, 65),
        p(-55, 67),
        p(-51, 71),
        p(-47, 75),
        p(-41, 76),
        p(-38, 80),
        p(-29, 80),
        p(-16, 76),
        p(-13, 77),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-36, -23),
        p(-37, 32),
        p(-40, 81),
        p(-35, 99),
        p(-32, 116),
        p(-27, 121),
        p(-23, 131),
        p(-19, 137),
        p(-15, 141),
        p(-12, 143),
        p(-9, 147),
        p(-5, 150),
        p(-2, 151),
        p(-1, 155),
        p(2, 156),
        p(6, 159),
        p(7, 165),
        p(9, 164),
        p(19, 161),
        p(33, 152),
        p(38, 152),
        p(82, 128),
        p(81, 129),
        p(105, 109),
        p(197, 74),
        p(250, 29),
        p(287, 12),
        p(332, -19),
    ],
    [
        p(27, -17),
        p(24, -29),
        p(15, -24),
        p(6, -15),
        p(-2, -6),
        p(-17, 3),
        p(-31, 20),
        p(-45, 27),
        p(-31, 15),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-11, 12),
        p(-6, -3),
        p(23, 17),
        p(49, -15),
        p(21, -44),
        p(0, 0),
    ],
    [p(-1, 13), p(19, 21), p(-2, 9), p(28, 2), p(27, 56), p(0, 0)],
    [
        p(3, 17),
        p(22, 20),
        p(23, 21),
        p(-6, 11),
        p(43, -4),
        p(0, 0),
    ],
    [p(-0, -2), p(7, 12), p(-0, 30), p(0, 6), p(2, -17), p(0, 0)],
    [p(76, 34), p(-30, 22), p(2, 19), p(-33, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 5), p(11, 4), p(9, 10), p(15, 5), p(9, 16), p(14, 3)],
    [
        p(-3, 1),
        p(8, 18),
        p(-100, -35),
        p(6, 12),
        p(7, 16),
        p(4, 5),
    ],
    [p(3, 2), p(14, 4), p(9, 10), p(11, 7), p(12, 15), p(22, -6)],
    [
        p(3, -5),
        p(10, -2),
        p(8, -8),
        p(4, 15),
        p(-57, -260),
        p(7, -11),
    ],
    [p(25, 8), p(3, 15), p(8, 10), p(-14, 13), p(-1, -3), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(26, -19),
    p(17, -9),
    p(17, -3),
    p(23, -13),
    p(6, 22),
    p(10, 20),
];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues:
    Debug + Default + Copy + Clone + Send + 'static + StaticallyNamedEntity
{
    type Score: ScoreType;

    fn psqt(
        &self,
        square: ChessSquare,
        piece: ChessPieceType,
        color: ChessColor,
    ) -> SingleFeatureScore<Self::Score>;

    fn passed_pawn(square: ChessSquare) -> SingleFeatureScore<Self::Score>;

    fn unsupported_pawn() -> SingleFeatureScore<Self::Score>;

    fn doubled_pawn() -> SingleFeatureScore<Self::Score>;

    fn bishop_pair() -> SingleFeatureScore<Self::Score>;

    fn rook_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score>;

    fn king_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score>;

    fn bishop_openness(openness: FileOpenness, len: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_shield(&self, color: ChessColor, config: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_protection(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn pawn_attack(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn mobility(piece: ChessPieceType, mobility: usize) -> SingleFeatureScore<Self::Score>;

    fn threats(
        attacking: ChessPieceType,
        targeted: ChessPieceType,
    ) -> SingleFeatureScore<Self::Score>;

    fn defended(
        protecting: ChessPieceType,
        target: ChessPieceType,
    ) -> SingleFeatureScore<Self::Score>;

    fn king_zone_attack(attacking: ChessPieceType) -> SingleFeatureScore<Self::Score>;
}

/// Eval values tuned on a combination of the zurichess dataset and a dataset used by 4ku,
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

    fn psqt(&self, square: ChessSquare, piece: ChessPieceType, color: ChessColor) -> Self::Score {
        PSQTS[piece as usize][square.flip_if(color == White).bb_idx()]
    }

    fn passed_pawn(square: ChessSquare) -> PhasedScore {
        PASSED_PAWNS[square.bb_idx()]
    }

    fn unsupported_pawn() -> PhasedScore {
        UNSUPPORTED_PAWN
    }

    fn doubled_pawn() -> PhasedScore {
        DOUBLED_PAWN
    }

    fn bishop_pair() -> SingleFeatureScore<Self::Score> {
        BISHOP_PAIR
    }

    fn rook_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score> {
        match openness {
            FileOpenness::Open => ROOK_OPEN_FILE,
            FileOpenness::Closed => ROOK_CLOSED_FILE,
            FileOpenness::SemiOpen => ROOK_SEMIOPEN_FILE,
            FileOpenness::SemiClosed => Self::Score::default(),
        }
    }

    fn king_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score> {
        match openness {
            FileOpenness::Open => KING_OPEN_FILE,
            FileOpenness::Closed => KING_CLOSED_FILE,
            FileOpenness::SemiOpen => KING_SEMIOPEN_FILE,
            FileOpenness::SemiClosed => Self::Score::default(),
        }
    }

    fn bishop_openness(openness: FileOpenness, len: usize) -> PhasedScore {
        BISHOP_OPENNESS[openness as usize][len - 1]
    }

    fn pawn_shield(&self, _color: ChessColor, config: usize) -> SingleFeatureScore<Self::Score> {
        PAWN_SHIELDS[config]
    }

    fn pawn_protection(piece: ChessPieceType) -> SingleFeatureScore<Self::Score> {
        PAWN_PROTECTION[piece as usize]
    }

    fn pawn_attack(piece: ChessPieceType) -> SingleFeatureScore<Self::Score> {
        PAWN_ATTACKS[piece as usize]
    }

    fn mobility(piece: ChessPieceType, mobility: usize) -> SingleFeatureScore<Self::Score> {
        MOBILITY[piece as usize - 1][mobility]
    }

    fn threats(
        attacking: ChessPieceType,
        targeted: ChessPieceType,
    ) -> SingleFeatureScore<Self::Score> {
        THREATS[attacking as usize - 1][targeted as usize]
    }
    fn defended(
        protecting: ChessPieceType,
        target: ChessPieceType,
    ) -> SingleFeatureScore<Self::Score> {
        DEFENDED[protecting as usize - 1][target as usize]
    }

    fn king_zone_attack(attacking: ChessPieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }
}
