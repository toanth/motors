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
use gears::games::DimT;
use gears::general::common::StaticallyNamedEntity;
use gears::score::{p, PhasedScore};
use std::fmt::{Debug, Display};

#[rustfmt::skip]
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
        p( 141,  181),    p( 144,  178),    p( 135,  182),    p( 144,  162),    p( 134,  166),    p( 137,  169),    p( 100,  186),    p( 101,  187),
        p(  70,  109),    p(  69,  107),    p(  84,  105),    p(  91,  107),    p(  81,  108),    p( 129,   94),    p( 101,  113),    p(  98,  106),
        p(  53,  101),    p(  61,   95),    p(  65,   93),    p(  67,   87),    p(  83,   87),    p(  88,   83),    p(  76,   88),    p(  74,   84),
        p(  41,   93),    p(  43,   95),    p(  57,   90),    p(  67,   89),    p(  70,   89),    p(  69,   84),    p(  59,   85),    p(  53,   81),
        p(  37,   90),    p(  43,   85),    p(  52,   88),    p(  54,   93),    p(  62,   91),    p(  57,   87),    p(  62,   75),    p(  49,   78),
        p(  45,   90),    p(  45,   87),    p(  55,   90),    p(  54,   96),    p(  53,  100),    p(  68,   90),    p(  67,   75),    p(  52,   79),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 176,  278),    p( 197,  311),    p( 215,  322),    p( 252,  311),    p( 282,  313),    p( 197,  309),    p( 211,  310),    p( 203,  261),
        p( 268,  312),    p( 283,  317),    p( 299,  308),    p( 303,  312),    p( 302,  308),    p( 315,  297),    p( 275,  314),    p( 272,  303),
        p( 286,  308),    p( 304,  304),    p( 306,  310),    p( 320,  314),    p( 337,  307),    p( 350,  296),    p( 291,  304),    p( 286,  308),
        p( 302,  315),    p( 308,  309),    p( 323,  313),    p( 326,  320),    p( 324,  318),    p( 319,  317),    p( 310,  312),    p( 319,  311),
        p( 299,  317),    p( 303,  307),    p( 312,  313),    p( 320,  316),    p( 318,  319),    p( 323,  303),    p( 321,  303),    p( 313,  312),
        p( 275,  304),    p( 281,  302),    p( 294,  297),    p( 300,  310),    p( 305,  308),    p( 293,  290),    p( 300,  293),    p( 293,  307),
        p( 270,  312),    p( 281,  314),    p( 284,  304),    p( 294,  308),    p( 298,  302),    p( 288,  300),    p( 295,  305),    p( 290,  320),
        p( 239,  310),    p( 282,  304),    p( 267,  306),    p( 287,  310),    p( 296,  307),    p( 292,  297),    p( 289,  306),    p( 266,  308),
    ],
    // bishop
    [
        p( 276,  310),    p( 252,  314),    p( 238,  306),    p( 222,  317),    p( 218,  313),    p( 222,  308),    p( 273,  303),    p( 251,  309),
        p( 281,  303),    p( 277,  303),    p( 289,  306),    p( 277,  304),    p( 288,  301),    p( 292,  299),    p( 266,  308),    p( 270,  301),
        p( 295,  309),    p( 306,  305),    p( 291,  304),    p( 306,  299),    p( 305,  300),    p( 335,  304),    p( 317,  301),    p( 317,  313),
        p( 285,  313),    p( 292,  307),    p( 303,  302),    p( 307,  306),    p( 307,  304),    p( 299,  304),    p( 297,  309),    p( 279,  310),
        p( 289,  308),    p( 283,  309),    p( 295,  304),    p( 309,  305),    p( 302,  301),    p( 298,  303),    p( 286,  304),    p( 308,  302),
        p( 296,  311),    p( 300,  305),    p( 300,  307),    p( 300,  304),    p( 306,  307),    p( 299,  298),    p( 305,  296),    p( 307,  299),
        p( 307,  309),    p( 303,  300),    p( 309,  301),    p( 298,  309),    p( 301,  305),    p( 304,  304),    p( 312,  295),    p( 308,  296),
        p( 297,  305),    p( 311,  306),    p( 307,  307),    p( 290,  309),    p( 306,  308),    p( 294,  309),    p( 305,  296),    p( 302,  292),
    ],
    // rook
    [
        p( 457,  548),    p( 447,  557),    p( 441,  564),    p( 439,  561),    p( 450,  557),    p( 470,  552),    p( 481,  550),    p( 491,  543),
        p( 442,  554),    p( 440,  559),    p( 450,  560),    p( 464,  550),    p( 450,  553),    p( 467,  548),    p( 474,  545),    p( 490,  535),
        p( 445,  548),    p( 462,  544),    p( 456,  545),    p( 457,  540),    p( 483,  530),    p( 492,  527),    p( 510,  526),    p( 485,  528),
        p( 442,  549),    p( 447,  544),    p( 446,  547),    p( 452,  541),    p( 457,  532),    p( 467,  528),    p( 467,  532),    p( 467,  527),
        p( 435,  546),    p( 434,  544),    p( 434,  545),    p( 439,  541),    p( 445,  537),    p( 440,  536),    p( 453,  530),    p( 447,  529),
        p( 430,  544),    p( 430,  540),    p( 431,  539),    p( 435,  539),    p( 440,  533),    p( 451,  525),    p( 467,  514),    p( 454,  518),
        p( 432,  539),    p( 436,  538),    p( 442,  539),    p( 444,  536),    p( 451,  529),    p( 464,  519),    p( 471,  514),    p( 443,  523),
        p( 442,  543),    p( 438,  538),    p( 439,  542),    p( 444,  536),    p( 449,  529),    p( 455,  529),    p( 452,  528),    p( 448,  530),
    ],
    // queen
    [
        p( 878,  961),    p( 881,  976),    p( 895,  989),    p( 917,  982),    p( 915,  986),    p( 934,  974),    p( 979,  927),    p( 924,  959),
        p( 888,  952),    p( 862,  982),    p( 866, 1008),    p( 858, 1026),    p( 865, 1037),    p( 906,  997),    p( 905,  982),    p( 947,  961),
        p( 893,  957),    p( 886,  974),    p( 885,  994),    p( 886, 1003),    p( 910, 1005),    p( 946,  989),    p( 955,  960),    p( 942,  967),
        p( 880,  970),    p( 886,  977),    p( 880,  987),    p( 881,  999),    p( 884, 1011),    p( 897, 1001),    p( 905, 1003),    p( 913,  978),
        p( 890,  961),    p( 878,  980),    p( 884,  981),    p( 884,  998),    p( 887,  994),    p( 889,  994),    p( 902,  982),    p( 908,  976),
        p( 886,  950),    p( 893,  966),    p( 887,  980),    p( 885,  983),    p( 890,  990),    p( 896,  979),    p( 910,  962),    p( 908,  950),
        p( 886,  952),    p( 887,  960),    p( 893,  963),    p( 893,  977),    p( 894,  976),    p( 895,  959),    p( 907,  937),    p( 915,  909),
        p( 873,  953),    p( 885,  941),    p( 885,  954),    p( 894,  955),    p( 896,  944),    p( 883,  949),    p( 886,  939),    p( 889,  923),
    ],
    // king
    [
        p( 159,  -84),    p(  58,  -35),    p(  83,  -28),    p(   7,    4),    p(  35,   -9),    p(  23,    1),    p(  75,   -8),    p( 236,  -88),
        p( -31,    4),    p( -83,   22),    p( -85,   29),    p( -25,   20),    p( -54,   27),    p( -83,   42),    p( -52,   27),    p(   7,    2),
        p( -45,   11),    p( -50,   17),    p( -88,   31),    p( -97,   39),    p( -66,   34),    p( -34,   26),    p( -81,   29),    p( -37,   12),
        p( -25,    3),    p(-102,   15),    p(-116,   31),    p(-138,   40),    p(-137,   37),    p(-117,   30),    p(-136,   20),    p(-106,   18),
        p( -40,   -1),    p(-116,   10),    p(-128,   27),    p(-152,   40),    p(-155,   38),    p(-129,   24),    p(-145,   15),    p(-118,   13),
        p( -32,    2),    p( -92,    5),    p(-120,   20),    p(-127,   29),    p(-124,   28),    p(-134,   20),    p(-109,    6),    p( -72,   10),
        p(  26,   -7),    p( -78,   -1),    p( -91,    9),    p(-111,   18),    p(-116,   19),    p(-101,   10),    p( -73,   -8),    p(   3,   -3),
        p(  57,  -25),    p(  43,  -35),    p(  39,  -22),    p( -22,   -1),    p(  29,  -18),    p( -18,   -5),    p(  35,  -30),    p(  67,  -35),
    ],
];

const BISHOP_PAIR: PhasedScore = p(22, 53);
const BAD_BISHOP: [PhasedScore; 9] = [
    p(9, 19),
    p(10, 18),
    p(11, 7),
    p(7, -1),
    p(3, -9),
    p(-1, -19),
    p(-8, -28),
    p(-16, -40),
    p(-27, -50),
];
const ROOK_OPEN_FILE: PhasedScore = p(13, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -0);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 4);
const KING_OPEN_FILE: PhasedScore = p(-48, -0);
const KING_CLOSED_FILE: PhasedScore = p(14, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-7, 9);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 4), p(-0, 6), p(-1, 4), p(2, 3), p(2, 5), p(3, 7), p(6, 4), p(18, 1)],
    // Closed
    [p(0, 0), p(0, 0), p(16, -24), p(-16, 9), p(-1, 10), p(2, 4), p(-0, 7), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-15, 23), p(3, 17), p(1, 9), p(-0, 8), p(4, 5), p(-0, 2), p(10, 5)],
    // SemiClosed
    [p(0, 0), p(11, -12), p(7, 5), p(3, 0), p(7, 1), p(3, 4), p(4, 5), p(1, 4)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-4, 5),    /*0b0000*/
    p(-15, 8),   /*0b0001*/
    p(-2, 9),    /*0b0010*/
    p(-9, 14),   /*0b0011*/
    p(-3, 3),    /*0b0100*/
    p(-25, -0),  /*0b0101*/
    p(-13, 7),   /*0b0110*/
    p(-18, -14), /*0b0111*/
    p(9, 11),    /*0b1000*/
    p(-2, 10),   /*0b1001*/
    p(3, 11),    /*0b1010*/
    p(-1, 14),   /*0b1011*/
    p(0, 5),     /*0b1100*/
    p(-22, 10),  /*0b1101*/
    p(-9, 9),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(4, 16),    /*0b10000*/
    p(4, 9),     /*0b10001*/
    p(23, 11),   /*0b10010*/
    p(-4, 11),   /*0b10011*/
    p(-4, 7),    /*0b10100*/
    p(13, 11),   /*0b10101*/
    p(-22, 6),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(15, 31),   /*0b11000*/
    p(28, 21),   /*0b11001*/
    p(41, 36),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(18, 10),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(17, 10),   /*0b100000*/
    p(3, 13),    /*0b100001*/
    p(25, 3),    /*0b100010*/
    p(8, 4),     /*0b100011*/
    p(-6, 2),    /*0b100100*/
    p(-18, -5),  /*0b100101*/
    p(-22, 20),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(23, 4),    /*0b101000*/
    p(1, 16),    /*0b101001*/
    p(23, -0),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-5, 8),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(16, 20),   /*0b110000*/
    p(26, 13),   /*0b110001*/
    p(33, 8),    /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(9, 27),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(26, 18),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(6, 0),     /*0b111111*/
    p(-14, -3),  /*0b00*/
    p(12, -17),  /*0b01*/
    p(38, -8),   /*0b10*/
    p(23, -39),  /*0b11*/
    p(46, -10),  /*0b100*/
    p(10, -18),  /*0b101*/
    p(70, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(65, -12),  /*0b1000*/
    p(20, -34),  /*0b1001*/
    p(84, -53),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(60, -18),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(26, -8),   /*0b1111*/
    p(21, -2),   /*0b00*/
    p(34, -13),  /*0b01*/
    p(29, -17),  /*0b10*/
    p(24, -40),  /*0b11*/
    p(38, -8),   /*0b100*/
    p(58, -19),  /*0b101*/
    p(26, -24),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(41, -3),   /*0b1000*/
    p(54, -17),  /*0b1001*/
    p(56, -40),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(45, -20),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(25, -41),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  41,   81),    p(  44,   78),    p(  35,   82),    p(  44,   62),    p(  34,   66),    p(  37,   69),    p(  -0,   86),    p(   1,   87),
        p(  39,  128),    p(  47,  128),    p(  34,  103),    p(  19,   72),    p(  32,   71),    p(  12,   98),    p(  -2,  108),    p( -34,  130),
        p(  25,   77),    p(  19,   75),    p(  21,   55),    p(  16,   44),    p(  -1,   47),    p(   6,   59),    p(  -7,   79),    p(  -9,   81),
        p(  13,   47),    p(   5,   44),    p( -12,   33),    p(  -7,   24),    p( -15,   28),    p(  -7,   38),    p( -10,   55),    p(  -7,   51),
        p(   7,   16),    p(  -7,   24),    p( -14,   16),    p( -14,    8),    p( -14,   13),    p(  -5,   17),    p( -10,   37),    p(  14,   18),
        p(  -0,   17),    p(   1,   21),    p(  -8,   17),    p(  -7,    6),    p(   5,    1),    p(   8,    7),    p(  16,   18),    p(  11,   15),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-9, -5);
const DOUBLED_PAWN: PhasedScore = p(-4, -20);
const PHALANX: [PhasedScore; 7] = [
    p(0, 0),
    p(-49, 17),
    p(-9, 18),
    p(-2, 13),
    p(10, 4),
    p(7, 5),
    p(6, 7),
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = [
    p(12, 8),
    p(8, 13),
    p(14, 19),
    p(10, 7),
    p(-3, 16),
    p(-45, 6),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(38, 8), p(39, 34), p(53, -9), p(35, -35), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-49, -70),
        p(-28, -31),
        p(-15, -9),
        p(-5, 4),
        p(3, 15),
        p(10, 26),
        p(19, 29),
        p(26, 32),
        p(33, 30),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-31, -55),
        p(-19, -38),
        p(-7, -23),
        p(-0, -10),
        p(7, -1),
        p(12, 8),
        p(17, 13),
        p(21, 17),
        p(23, 22),
        p(30, 24),
        p(35, 23),
        p(44, 26),
        p(40, 32),
        p(55, 27),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-75, 13),
        p(-66, 27),
        p(-62, 32),
        p(-59, 36),
        p(-59, 43),
        p(-53, 48),
        p(-50, 52),
        p(-46, 54),
        p(-42, 58),
        p(-38, 62),
        p(-34, 64),
        p(-33, 68),
        p(-24, 69),
        p(-16, 66),
        p(-13, 67),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-27, -48),
        p(-27, 10),
        p(-31, 59),
        p(-26, 75),
        p(-24, 93),
        p(-19, 98),
        p(-15, 108),
        p(-11, 114),
        p(-7, 118),
        p(-4, 119),
        p(-1, 122),
        p(2, 124),
        p(5, 124),
        p(6, 129),
        p(9, 130),
        p(12, 133),
        p(13, 140),
        p(16, 140),
        p(25, 138),
        p(38, 132),
        p(43, 134),
        p(86, 110),
        p(86, 113),
        p(109, 95),
        p(204, 60),
        p(248, 21),
        p(281, 8),
        p(328, -24),
    ],
    [
        p(-96, 10),
        p(-59, -3),
        p(-29, -4),
        p(2, -3),
        p(35, -2),
        p(58, -3),
        p(87, 2),
        p(113, 1),
        p(163, -16),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-9, 7),
        p(0, 0),
        p(23, 19),
        p(49, -12),
        p(20, -34),
        p(0, 0),
    ],
    [p(-3, 11), p(20, 22), p(0, 0), p(31, 4), p(31, 53), p(0, 0)],
    [p(-3, 13), p(11, 15), p(17, 12), p(0, 0), p(45, -6), p(0, 0)],
    [p(-2, 5), p(2, 5), p(-0, 22), p(1, 1), p(0, 0), p(0, 0)],
    [
        p(71, 28),
        p(-35, 18),
        p(-9, 17),
        p(-22, 7),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 7), p(8, 7), p(6, 11), p(13, 7), p(7, 20), p(11, 6)],
    [
        p(1, 6),
        p(11, 22),
        p(-127, -27),
        p(8, 14),
        p(9, 20),
        p(4, 7),
    ],
    [p(2, 1), p(14, 6), p(9, 11), p(11, 8), p(11, 21), p(21, -6)],
    [
        p(2, -2),
        p(9, 1),
        p(7, -5),
        p(4, 15),
        p(-62, -252),
        p(5, -11),
    ],
    [
        p(63, -2),
        p(41, 6),
        p(47, -0),
        p(25, 4),
        p(38, -13),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-21, -18),
    p(19, -10),
    p(11, -4),
    p(14, -12),
    p(-1, 12),
    p(-13, 12),
];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(13, 19), p(34, -1), p(5, 32)];

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

    fn phalanx(rank: DimT) -> SingleFeatureScore<Self::Score>;

    fn bishop_pair() -> SingleFeatureScore<Self::Score>;

    fn bad_bishop(num_pawns: usize) -> SingleFeatureScore<Self::Score>;

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

    fn can_give_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;
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

    fn phalanx(rank: DimT) -> SingleFeatureScore<Self::Score> {
        PHALANX[rank as usize]
    }

    fn bishop_pair() -> PhasedScore {
        BISHOP_PAIR
    }

    fn bad_bishop(num_pawns: usize) -> SingleFeatureScore<Self::Score> {
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

    fn bishop_openness(
        openness: FileOpenness,
        len: usize,
    ) -> <PhasedScore as ScoreType>::SingleFeatureScore {
        BISHOP_OPENNESS[openness as usize][len - 1]
    }

    fn pawn_shield(&self, _color: ChessColor, config: usize) -> PhasedScore {
        PAWN_SHIELDS[config]
    }

    fn pawn_protection(piece: ChessPieceType) -> PhasedScore {
        PAWN_PROTECTION[piece as usize]
    }

    fn pawn_attack(piece: ChessPieceType) -> PhasedScore {
        PAWN_ATTACKS[piece as usize]
    }

    fn mobility(piece: ChessPieceType, mobility: usize) -> PhasedScore {
        MOBILITY[piece as usize - 1][mobility]
    }

    fn threats(attacking: ChessPieceType, targeted: ChessPieceType) -> PhasedScore {
        THREATS[attacking as usize - 1][targeted as usize]
    }
    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> PhasedScore {
        DEFENDED[protecting as usize - 1][target as usize]
    }

    fn king_zone_attack(attacking: ChessPieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }

    fn can_give_check(piece: ChessPieceType) -> PhasedScore {
        CAN_GIVE_CHECK[piece as usize]
    }
}
