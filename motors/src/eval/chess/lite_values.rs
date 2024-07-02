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
        p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),
        p( 125,  179),    p( 125,  179),    p( 115,  182),    p( 126,  163),    p( 113,  168),    p( 115,  171),    p(  79,  188),    p(  82,  186),
        p(  64,  121),    p(  64,  123),    p(  72,  112),    p(  80,  116),    p(  67,  118),    p( 110,  104),    p(  91,  128),    p(  78,  118),
        p(  53,  109),    p(  64,  104),    p(  58,   96),    p(  60,   88),    p(  78,   89),    p(  79,   88),    p(  74,   99),    p(  68,   91),
        p(  48,   96),    p(  58,   98),    p(  61,   87),    p(  69,   85),    p(  71,   86),    p(  75,   81),    p(  70,   89),    p(  56,   81),
        p(  42,   91),    p(  54,   88),    p(  52,   84),    p(  57,   91),    p(  67,   87),    p(  57,   84),    p(  70,   79),    p(  49,   79),
        p(  53,   96),    p(  64,   96),    p(  60,   90),    p(  58,   96),    p(  65,  100),    p(  75,   92),    p(  85,   84),    p(  52,   83),
        p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),
    ],
    // knight
    [
        p( 181,  262),    p( 203,  293),    p( 238,  304),    p( 270,  292),    p( 303,  294),    p( 212,  291),    p( 227,  292),    p( 215,  241),
        p( 268,  296),    p( 280,  304),    p( 295,  300),    p( 315,  300),    p( 307,  296),    p( 330,  285),    p( 280,  299),    p( 292,  285),
        p( 282,  293),    p( 295,  297),    p( 311,  306),    p( 322,  306),    p( 339,  299),    p( 361,  290),    p( 315,  291),    p( 310,  287),
        p( 295,  302),    p( 300,  301),    p( 309,  311),    p( 336,  312),    p( 324,  309),    p( 334,  308),    p( 317,  297),    p( 334,  292),
        p( 290,  306),    p( 288,  299),    p( 294,  312),    p( 301,  316),    p( 308,  317),    p( 306,  303),    p( 317,  295),    p( 306,  301),
        p( 264,  293),    p( 265,  296),    p( 273,  296),    p( 279,  309),    p( 287,  305),    p( 272,  291),    p( 288,  287),    p( 282,  296),
        p( 261,  298),    p( 271,  301),    p( 267,  296),    p( 280,  301),    p( 282,  295),    p( 274,  292),    p( 285,  293),    p( 281,  306),
        p( 234,  294),    p( 274,  292),    p( 256,  294),    p( 276,  299),    p( 286,  296),    p( 281,  286),    p( 280,  291),    p( 258,  291),
    ],
    // bishop
    [
        p( 276,  311),    p( 246,  316),    p( 243,  311),    p( 223,  317),    p( 230,  315),    p( 226,  311),    p( 280,  306),    p( 254,  304),
        p( 278,  303),    p( 282,  308),    p( 287,  309),    p( 277,  313),    p( 290,  307),    p( 299,  305),    p( 280,  308),    p( 278,  303),
        p( 288,  313),    p( 303,  306),    p( 293,  313),    p( 307,  307),    p( 312,  310),    p( 340,  310),    p( 326,  304),    p( 316,  312),
        p( 279,  311),    p( 295,  314),    p( 303,  311),    p( 323,  314),    p( 314,  312),    p( 310,  314),    p( 298,  313),    p( 287,  310),
        p( 288,  308),    p( 281,  315),    p( 298,  316),    p( 315,  313),    p( 313,  312),    p( 293,  315),    p( 286,  314),    p( 310,  300),
        p( 289,  309),    p( 303,  311),    p( 298,  313),    p( 302,  315),    p( 303,  318),    p( 298,  312),    p( 303,  304),    p( 303,  303),
        p( 303,  311),    p( 302,  302),    p( 309,  304),    p( 291,  315),    p( 296,  314),    p( 297,  308),    p( 308,  303),    p( 295,  299),
        p( 290,  302),    p( 313,  306),    p( 304,  309),    p( 285,  313),    p( 298,  310),    p( 292,  316),    p( 302,  295),    p( 293,  293),
    ],
    // rook
    [
        p( 453,  537),    p( 443,  546),    p( 443,  551),    p( 443,  548),    p( 456,  543),    p( 474,  539),    p( 482,  538),    p( 492,  531),
        p( 425,  541),    p( 424,  547),    p( 435,  547),    p( 452,  536),    p( 443,  538),    p( 462,  533),    p( 473,  530),    p( 486,  521),
        p( 423,  541),    p( 445,  536),    p( 442,  538),    p( 448,  533),    p( 473,  522),    p( 490,  515),    p( 514,  512),    p( 486,  514),
        p( 423,  542),    p( 432,  537),    p( 432,  540),    p( 438,  535),    p( 448,  526),    p( 460,  519),    p( 469,  519),    p( 465,  515),
        p( 419,  538),    p( 420,  537),    p( 420,  538),    p( 428,  535),    p( 434,  531),    p( 435,  526),    p( 454,  519),    p( 442,  518),
        p( 417,  534),    p( 416,  533),    p( 420,  533),    p( 423,  533),    p( 431,  526),    p( 442,  516),    p( 467,  502),    p( 446,  508),
        p( 420,  530),    p( 424,  530),    p( 429,  531),    p( 432,  529),    p( 441,  521),    p( 456,  511),    p( 466,  505),    p( 434,  515),
        p( 428,  533),    p( 424,  529),    p( 425,  534),    p( 430,  530),    p( 438,  523),    p( 445,  522),    p( 443,  518),    p( 437,  520),
    ],
    // queen
    [
        p( 851,  959),    p( 849,  977),    p( 865,  990),    p( 884,  986),    p( 884,  990),    p( 903,  976),    p( 954,  924),    p( 905,  951),
        p( 858,  951),    p( 833,  983),    p( 835, 1010),    p( 828, 1027),    p( 836, 1039),    p( 876, 1003),    p( 881,  978),    p( 924,  958),
        p( 863,  950),    p( 856,  968),    p( 855,  992),    p( 858, 1006),    p( 880, 1012),    p( 922,  993),    p( 927,  963),    p( 917,  971),
        p( 851,  961),    p( 856,  972),    p( 851,  988),    p( 851, 1004),    p( 855, 1017),    p( 868, 1009),    p( 877, 1005),    p( 887,  984),
        p( 861,  952),    p( 848,  979),    p( 854,  981),    p( 854,  999),    p( 856,  996),    p( 858,  997),    p( 873,  985),    p( 881,  977),
        p( 857,  945),    p( 862,  960),    p( 855,  974),    p( 853,  978),    p( 857,  981),    p( 866,  974),    p( 880,  959),    p( 880,  949),
        p( 861,  941),    p( 860,  947),    p( 865,  950),    p( 863,  959),    p( 864,  958),    p( 864,  947),    p( 876,  927),    p( 885,  904),
        p( 848,  936),    p( 858,  927),    p( 856,  937),    p( 864,  937),    p( 866,  931),    p( 854,  937),    p( 858,  924),    p( 860,  913),
    ],
    // king
    [
        p( 126, -113),    p(  49,  -59),    p(  74,  -51),    p(  -5,  -19),    p(  20,  -32),    p(   3,  -22),    p(  58,  -32),    p( 203, -116),
        p( -25,   -8),    p( -43,   29),    p( -55,   40),    p(  12,   29),    p( -18,   36),    p( -50,   50),    p( -10,   34),    p(  19,   -5),
        p( -44,    2),    p( -12,   25),    p( -59,   44),    p( -65,   52),    p( -29,   45),    p(   0,   38),    p( -31,   36),    p( -26,    9),
        p( -29,   -2),    p( -69,   26),    p( -89,   45),    p(-108,   54),    p(-109,   51),    p( -91,   43),    p( -95,   32),    p( -95,   15),
        p( -46,   -5),    p( -89,   22),    p(-104,   40),    p(-126,   54),    p(-133,   52),    p(-109,   37),    p(-117,   27),    p(-110,   13),
        p( -35,   -0),    p( -64,   17),    p( -95,   33),    p(-106,   43),    p(-101,   41),    p(-113,   33),    p( -81,   18),    p( -68,   11),
        p(  29,  -10),    p( -46,   11),    p( -58,   21),    p( -82,   31),    p( -86,   31),    p( -71,   22),    p( -35,    3),    p(  10,   -4),
        p(  22,  -51),    p(  36,  -57),    p(  34,  -44),    p( -32,  -22),    p(  24,  -41),    p( -28,  -26),    p(  31,  -53),    p(  47,  -61),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  29,   83),    p(  29,   83),    p(  19,   86),    p(  30,   67),    p(  17,   72),    p(  19,   75),    p( -17,   92),    p( -14,   90),
        p(  28,  116),    p(  38,  115),    p(  31,   98),    p(  16,   67),    p(  26,   66),    p(  12,   93),    p(  -6,   97),    p( -33,  119),
        p(  10,   67),    p(   9,   66),    p(  18,   52),    p(  11,   44),    p(  -7,   46),    p(   2,   56),    p( -15,   71),    p( -17,   72),
        p(  -3,   41),    p( -12,   39),    p( -20,   34),    p( -12,   25),    p( -20,   29),    p( -18,   37),    p( -25,   50),    p( -17,   46),
        p(  -9,   11),    p( -21,   20),    p( -21,   17),    p( -17,    8),    p( -17,   14),    p( -14,   16),    p( -20,   33),    p(   3,   14),
        p( -15,   10),    p( -12,   14),    p( -14,   17),    p(  -9,    4),    p(   1,    0),    p(  -1,    6),    p(   4,   12),    p(   1,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(15, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-14, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(-3, -0);
const KING_OPEN_FILE: PhasedScore = p(-60, -3);
const KING_CLOSED_FILE: PhasedScore = p(15, -17);
const KING_SEMIOPEN_FILE: PhasedScore = p(-11, 1);
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-10, 7),   /*0b0000*/
    p(-18, 12),  /*0b0001*/
    p(-12, 8),   /*0b0010*/
    p(-8, 28),   /*0b0011*/
    p(-4, 7),    /*0b0100*/
    p(-30, 4),   /*0b0101*/
    p(-11, 19),  /*0b0110*/
    p(-14, 2),   /*0b0111*/
    p(5, 9),     /*0b1000*/
    p(-21, -12), /*0b1001*/
    p(-4, 9),    /*0b1010*/
    p(-5, 2),    /*0b1011*/
    p(-3, 6),    /*0b1100*/
    p(-38, -12), /*0b1101*/
    p(-4, 18),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-5, 16),   /*0b10000*/
    p(6, 11),    /*0b10001*/
    p(-5, -13),  /*0b10010*/
    p(-7, -0),   /*0b10011*/
    p(-4, 6),    /*0b10100*/
    p(10, 15),   /*0b10101*/
    p(-23, -9),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(18, 44),   /*0b11000*/
    p(26, 10),   /*0b11001*/
    p(30, 23),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(18, 20),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(16, 8),    /*0b100000*/
    p(-0, 12),   /*0b100001*/
    p(16, 2),    /*0b100010*/
    p(8, 15),    /*0b100011*/
    p(-26, -21), /*0b100100*/
    p(-40, -30), /*0b100101*/
    p(-32, 8),   /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(18, -0),   /*0b101000*/
    p(-19, -3),  /*0b101001*/
    p(17, -5),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-23, -18), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(26, 30),   /*0b110000*/
    p(36, 22),   /*0b110001*/
    p(20, -10),  /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(-0, 11),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(36, 36),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(-0, -10),  /*0b111111*/
    p(-29, -16), /*0b00*/
    p(1, -32),   /*0b01*/
    p(29, -19),  /*0b10*/
    p(29, -44),  /*0b11*/
    p(38, -27),  /*0b100*/
    p(-32, -61), /*0b101*/
    p(68, -57),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(57, -28),  /*0b1000*/
    p(17, -51),  /*0b1001*/
    p(64, -98),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(69, -26),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(5, -25),   /*0b1111*/
    p(6, -18),   /*0b00*/
    p(20, -30),  /*0b01*/
    p(15, -35),  /*0b10*/
    p(22, -49),  /*0b11*/
    p(27, -29),  /*0b100*/
    p(34, -66),  /*0b101*/
    p(16, -44),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(27, -23),  /*0b1000*/
    p(43, -37),  /*0b1001*/
    p(31, -89),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(45, -28),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(12, -79),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 13), p(3, 10), p(9, 15), p(8, 8), p(-4, 19), p(-45, 9)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(37, 10),
    p(44, 35),
    p(50, -8),
    p(38, -34),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-51, -52),
        p(-28, -14),
        p(-12, 8),
        p(0, 19),
        p(11, 28),
        p(21, 36),
        p(33, 35),
        p(42, 34),
        p(51, 29),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-24, -30),
        p(-10, -14),
        p(-0, 2),
        p(7, 13),
        p(16, 21),
        p(22, 29),
        p(27, 33),
        p(31, 36),
        p(35, 40),
        p(43, 40),
        p(52, 38),
        p(64, 38),
        p(67, 44),
        p(82, 36),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-66, 26),
        p(-57, 39),
        p(-53, 43),
        p(-49, 47),
        p(-49, 53),
        p(-44, 57),
        p(-40, 61),
        p(-36, 63),
        p(-32, 66),
        p(-27, 69),
        p(-22, 70),
        p(-18, 73),
        p(-8, 72),
        p(5, 68),
        p(6, 70),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-21, -22),
        p(-22, 31),
        p(-25, 78),
        p(-20, 95),
        p(-17, 113),
        p(-12, 119),
        p(-7, 131),
        p(-4, 138),
        p(1, 143),
        p(5, 145),
        p(8, 150),
        p(12, 154),
        p(15, 155),
        p(17, 161),
        p(20, 163),
        p(23, 166),
        p(24, 174),
        p(27, 173),
        p(36, 172),
        p(51, 164),
        p(55, 166),
        p(98, 142),
        p(97, 145),
        p(121, 126),
        p(212, 92),
        p(268, 46),
        p(317, 23),
        p(364, -8),
    ],
    [
        p(-59, 68),
        p(-35, 34),
        p(-18, 20),
        p(-0, 9),
        p(17, -2),
        p(27, -13),
        p(39, -14),
        p(48, -26),
        p(86, -58),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-10, 11),
        p(-7, -5),
        p(24, 16),
        p(48, -14),
        p(21, -45),
        p(0, 0),
    ],
    [p(-0, 15), p(20, 21), p(1, 5), p(29, 1), p(27, 56), p(0, 0)],
    [p(7, 16), p(22, 19), p(25, 20), p(-6, 9), p(43, -6), p(0, 0)],
    [p(1, 2), p(7, 12), p(0, 30), p(0, 5), p(1, -16), p(0, 0)],
    [
        p(68, 37),
        p(-39, 25),
        p(-9, 21),
        p(-48, 16),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(11, 5), p(9, 10), p(16, 5), p(9, 16), p(14, 2)],
    [
        p(1, -5),
        p(10, 17),
        p(-84, -42),
        p(8, 12),
        p(9, 16),
        p(6, 5),
    ],
    [p(2, 1), p(14, 3), p(10, 9), p(12, 7), p(13, 14), p(22, -6)],
    [
        p(3, -3),
        p(10, -1),
        p(9, -7),
        p(4, 16),
        p(-54, -262),
        p(7, -11),
    ],
    [
        p(47, -13),
        p(28, -5),
        p(34, -10),
        p(12, -7),
        p(25, -24),
        p(0, 0),
    ],
];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static {
    type Score: ScoreType;

    fn psqt(
        square: ChessSquare,
        piece: UncoloredChessPiece,
        color: Color,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn passed_pawn(square: ChessSquare) -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn bishop_pair() -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn rook_openness(openness: FileOpenness) -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn king_openness(openness: FileOpenness) -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn pawn_shield(config: usize) -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn pawn_protection(
        piece: UncoloredChessPiece,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn pawn_attack(piece: UncoloredChessPiece) -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn mobility(
        piece: UncoloredChessPiece,
        mobility: usize,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn threats(
        attacking: UncoloredChessPiece,
        targeted: UncoloredChessPiece,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn defended(
        protecting: UncoloredChessPiece,
        target: UncoloredChessPiece,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;
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
