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

#[rustfmt::skip]const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
        p( 128,  182),    p( 128,  181),    p( 118,  184),    p( 128,  166),    p( 117,  171),    p( 118,  174),    p(  84,  191),    p(  89,  188),
        p(  66,  121),    p(  65,  123),    p(  75,  111),    p(  82,  116),    p(  71,  118),    p( 119,  103),    p(  92,  128),    p(  85,  118),
        p(  51,  109),    p(  64,  104),    p(  60,   95),    p(  63,   87),    p(  78,   90),    p(  79,   87),    p(  71,   98),    p(  66,   92),
        p(  47,   96),    p(  58,   98),    p(  62,   86),    p(  71,   83),    p(  75,   83),    p(  77,   80),    p(  71,   88),    p(  57,   80),
        p(  39,   92),    p(  54,   87),    p(  53,   83),    p(  57,   90),    p(  66,   87),    p(  63,   81),    p(  72,   77),    p(  52,   77),
        p(  52,   96),    p(  63,   96),    p(  60,   89),    p(  59,   95),    p(  63,   99),    p(  79,   89),    p(  88,   82),    p(  56,   81),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 188,  270),    p( 209,  302),    p( 244,  314),    p( 269,  305),    p( 300,  306),    p( 214,  301),    p( 235,  300),    p( 217,  252),
        p( 276,  303),    p( 288,  312),    p( 302,  309),    p( 313,  312),    p( 306,  309),    p( 328,  297),    p( 287,  309),    p( 291,  295),
        p( 292,  301),    p( 303,  306),    p( 318,  315),    p( 320,  318),    p( 336,  312),    p( 360,  302),    p( 313,  303),    p( 309,  299),
        p( 304,  310),    p( 309,  308),    p( 316,  320),    p( 342,  322),    p( 318,  324),    p( 332,  321),    p( 313,  311),    p( 332,  304),
        p( 299,  313),    p( 298,  307),    p( 303,  320),    p( 310,  324),    p( 317,  325),    p( 315,  311),    p( 325,  303),    p( 314,  309),
        p( 274,  300),    p( 275,  303),    p( 282,  303),    p( 288,  317),    p( 295,  313),    p( 281,  298),    p( 296,  295),    p( 292,  303),
        p( 270,  305),    p( 280,  309),    p( 276,  304),    p( 288,  309),    p( 292,  303),    p( 284,  300),    p( 294,  301),    p( 289,  314),
        p( 243,  301),    p( 283,  299),    p( 266,  301),    p( 285,  307),    p( 296,  303),    p( 291,  293),    p( 290,  299),    p( 269,  298),
    ],
    // bishop
    [
        p( 280,  314),    p( 255,  313),    p( 251,  307),    p( 226,  314),    p( 225,  313),    p( 229,  307),    p( 283,  304),    p( 253,  308),
        p( 281,  303),    p( 289,  305),    p( 290,  305),    p( 282,  307),    p( 286,  302),    p( 297,  301),    p( 277,  307),    p( 275,  303),
        p( 301,  309),    p( 305,  304),    p( 296,  309),    p( 302,  302),    p( 307,  305),    p( 333,  307),    p( 319,  302),    p( 314,  311),
        p( 283,  310),    p( 299,  309),    p( 301,  305),    p( 318,  310),    p( 311,  307),    p( 306,  309),    p( 302,  308),    p( 283,  310),
        p( 292,  307),    p( 281,  311),    p( 299,  309),    p( 314,  308),    p( 311,  308),    p( 298,  307),    p( 290,  309),    p( 311,  299),
        p( 294,  306),    p( 304,  309),    p( 301,  308),    p( 304,  309),    p( 307,  310),    p( 303,  307),    p( 305,  302),    p( 311,  300),
        p( 309,  310),    p( 304,  300),    p( 311,  302),    p( 296,  310),    p( 302,  308),    p( 302,  305),    p( 313,  301),    p( 302,  298),
        p( 295,  304),    p( 313,  309),    p( 306,  306),    p( 289,  311),    p( 302,  309),    p( 295,  313),    p( 302,  299),    p( 300,  295),
    ],
    // rook
    [
        p( 459,  549),    p( 449,  559),    p( 447,  565),    p( 446,  562),    p( 457,  558),    p( 477,  552),    p( 487,  551),    p( 497,  543),
        p( 431,  555),    p( 429,  560),    p( 439,  561),    p( 455,  551),    p( 445,  553),    p( 466,  547),    p( 475,  544),    p( 488,  535),
        p( 435,  552),    p( 456,  548),    p( 452,  549),    p( 458,  544),    p( 484,  534),    p( 493,  529),    p( 515,  526),    p( 485,  529),
        p( 434,  552),    p( 443,  548),    p( 443,  551),    p( 449,  545),    p( 457,  537),    p( 467,  531),    p( 472,  533),    p( 468,  529),
        p( 430,  548),    p( 430,  547),    p( 430,  548),    p( 438,  545),    p( 444,  542),    p( 438,  540),    p( 457,  533),    p( 446,  531),
        p( 427,  544),    p( 426,  543),    p( 430,  543),    p( 433,  543),    p( 440,  537),    p( 447,  530),    p( 470,  516),    p( 452,  521),
        p( 430,  539),    p( 434,  540),    p( 439,  541),    p( 443,  539),    p( 450,  532),    p( 464,  522),    p( 473,  518),    p( 441,  526),
        p( 438,  543),    p( 434,  539),    p( 435,  544),    p( 440,  540),    p( 447,  534),    p( 453,  533),    p( 450,  530),    p( 445,  532),
    ],
    // queen
    [
        p( 873,  965),    p( 873,  980),    p( 888,  994),    p( 905,  989),    p( 904,  994),    p( 925,  980),    p( 974,  928),    p( 923,  957),
        p( 883,  957),    p( 859,  988),    p( 860, 1015),    p( 852, 1033),    p( 860, 1043),    p( 900, 1006),    p( 903,  985),    p( 945,  963),
        p( 891,  962),    p( 882,  982),    p( 882, 1005),    p( 880, 1014),    p( 902, 1015),    p( 943,  998),    p( 950,  968),    p( 937,  973),
        p( 876,  977),    p( 881,  988),    p( 874,  998),    p( 873, 1012),    p( 878, 1022),    p( 891, 1012),    p( 900, 1010),    p( 907,  986),
        p( 886,  968),    p( 872,  988),    p( 877,  991),    p( 878, 1010),    p( 879, 1006),    p( 881, 1005),    p( 895,  991),    p( 903,  981),
        p( 881,  954),    p( 886,  971),    p( 879,  988),    p( 876,  991),    p( 881,  998),    p( 888,  988),    p( 902,  969),    p( 902,  956),
        p( 884,  952),    p( 883,  960),    p( 888,  964),    p( 887,  977),    p( 887,  976),    p( 890,  960),    p( 900,  939),    p( 909,  912),
        p( 871,  947),    p( 881,  938),    p( 880,  953),    p( 889,  954),    p( 890,  948),    p( 879,  949),    p( 880,  937),    p( 884,  925),
    ],
    // king
    [
        p( 145,  -89),    p(  52,  -41),    p(  77,  -33),    p(   1,   -2),    p(  16,  -13),    p( -17,   -1),    p(  43,  -11),    p( 212,  -94),
        p( -33,    1),    p( -78,   20),    p( -84,   29),    p( -21,   18),    p( -54,   26),    p( -88,   42),    p( -56,   28),    p(  -0,    2),
        p( -49,    8),    p( -39,   14),    p( -82,   31),    p( -89,   39),    p( -58,   34),    p( -25,   25),    p( -71,   27),    p( -37,   10),
        p( -29,   -0),    p( -89,   12),    p(-105,   30),    p(-126,   39),    p(-126,   36),    p(-105,   29),    p(-118,   17),    p(-101,   13),
        p( -41,   -5),    p(-102,    7),    p(-116,   25),    p(-142,   38),    p(-144,   36),    p(-117,   22),    p(-127,   12),    p(-108,    8),
        p( -32,   -1),    p( -81,    3),    p(-110,   18),    p(-118,   27),    p(-115,   26),    p(-126,   18),    p( -96,    3),    p( -62,    6),
        p(  31,   -9),    p( -68,   -1),    p( -78,    7),    p( -97,   16),    p(-103,   17),    p( -92,   10),    p( -61,   -7),    p(  10,   -5),
        p(  44,  -29),    p(  41,  -37),    p(  36,  -24),    p( -21,   -5),    p(  32,  -23),    p( -22,   -7),    p(  30,  -29),    p(  63,  -39),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  28,   82),    p(  28,   81),    p(  18,   84),    p(  28,   66),    p(  17,   71),    p(  18,   74),    p( -16,   91),    p( -11,   88),
        p(  29,  116),    p(  38,  116),    p(  30,  100),    p(  16,   68),    p(  27,   67),    p(   9,   95),    p(  -6,   99),    p( -33,  121),
        p(  11,   68),    p(   9,   67),    p(  18,   53),    p(  11,   45),    p(  -4,   45),    p(   3,   57),    p( -14,   73),    p( -17,   75),
        p(  -3,   41),    p( -12,   40),    p( -21,   34),    p( -13,   26),    p( -21,   30),    p( -20,   38),    p( -27,   52),    p( -19,   47),
        p(  -7,   11),    p( -21,   20),    p( -20,   18),    p( -18,    9),    p( -17,   13),    p( -18,   19),    p( -20,   33),    p(   2,   14),
        p( -15,   10),    p( -12,   14),    p( -12,   17),    p( -11,    5),    p(   1,    0),    p(  -1,    7),    p(   5,   12),    p(   1,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(15, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -4);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(-2, -1);
const KING_OPEN_FILE: PhasedScore = p(-57, -3);
const KING_CLOSED_FILE: PhasedScore = p(10, -13);
const KING_SEMIOPEN_FILE: PhasedScore = p(-12, 3);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 5), p(-4, 7), p(-5, 8), p(2, 7), p(3, 9), p(3, 11), p(9, 10), p(20, 6), ],
    // Closed
    [p(0, 0), p(0, 0), p(16, -27), p(-12, 10), p(-2, 13), p(3, 4), p(2, 10), p(1, 6), ],
    // SemiOpen
    [p(0, 0), p(-15, 22), p(-0, 18), p(1, 15), p(-2, 18), p(3, 15), p(1, 11), p(12, 11), ],
    // SemiClosed
    [p(0, 0), p(12, -13), p(9, 7), p(6, 2), p(8, 5), p(4, 4), p(7, 7), p(2, 5), ],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-15, 5),   /*0b0000*/
    p(-20, 9),   /*0b0001*/
    p(-12, 6),   /*0b0010*/
    p(-6, 23),   /*0b0011*/
    p(-9, 5),    /*0b0100*/
    p(-31, -0),  /*0b0101*/
    p(-12, 15),  /*0b0110*/
    p(-12, -2),  /*0b0111*/
    p(0, 8),     /*0b1000*/
    p(-25, -13), /*0b1001*/
    p(-6, 8),    /*0b1010*/
    p(-6, -1),   /*0b1011*/
    p(-5, 2),    /*0b1100*/
    p(-42, -15), /*0b1101*/
    p(-5, 14),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-8, 13),   /*0b10000*/
    p(3, 8),     /*0b10001*/
    p(-5, -17),  /*0b10010*/
    p(-7, -5),   /*0b10011*/
    p(-7, 4),    /*0b10100*/
    p(9, 10),    /*0b10101*/
    p(-24, -13), /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(14, 41),   /*0b11000*/
    p(24, 5),    /*0b11001*/
    p(27, 19),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(18, 14),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(8, 8),     /*0b100000*/
    p(-4, 10),   /*0b100001*/
    p(16, 0),    /*0b100010*/
    p(12, 9),    /*0b100011*/
    p(-30, -24), /*0b100100*/
    p(-41, -34), /*0b100101*/
    p(-27, 1),   /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(14, -2),   /*0b101000*/
    p(-22, -7),  /*0b101001*/
    p(16, -7),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-26, -21), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(19, 28),   /*0b110000*/
    p(28, 19),   /*0b110001*/
    p(20, -15),  /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(-2, 6),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(32, 32),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(1, -16),   /*0b111111*/
    p(-23, -7),  /*0b00*/
    p(11, -23),  /*0b01*/
    p(34, -10),  /*0b10*/
    p(37, -34),  /*0b11*/
    p(45, -15),  /*0b100*/
    p(-17, -56), /*0b101*/
    p(77, -46),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(54, -15),  /*0b1000*/
    p(22, -40),  /*0b1001*/
    p(65, -87),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(66, -10),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(13, -14),  /*0b1111*/
    p(10, -5),   /*0b00*/
    p(27, -17),  /*0b01*/
    p(23, -22),  /*0b10*/
    p(31, -36),  /*0b11*/
    p(29, -13),  /*0b100*/
    p(38, -50),  /*0b101*/
    p(22, -30),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(32, -8),   /*0b1000*/
    p(54, -25),  /*0b1001*/
    p(38, -75),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(46, -13),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(22, -67),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = [
    p(14, 14),
    p(3, 10),
    p(10, 14),
    p(8, 9),
    p(-4, 18),
    p(-41, 6),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(38, 10),
    p(41, 36),
    p(51, -9),
    p(37, -35),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-60, -58),
        p(-37, -20),
        p(-21, 2),
        p(-9, 13),
        p(2, 21),
        p(13, 29),
        p(24, 29),
        p(34, 28),
        p(42, 24),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-28, -49),
        p(-14, -32),
        p(-4, -16),
        p(2, -4),
        p(9, 5),
        p(13, 14),
        p(16, 18),
        p(18, 22),
        p(19, 26),
        p(25, 26),
        p(29, 25),
        p(37, 27),
        p(30, 34),
        p(44, 26),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-75, 17),
        p(-65, 29),
        p(-61, 33),
        p(-58, 38),
        p(-58, 44),
        p(-52, 48),
        p(-49, 52),
        p(-45, 54),
        p(-42, 58),
        p(-38, 61),
        p(-33, 63),
        p(-30, 67),
        p(-21, 67),
        p(-9, 63),
        p(-8, 64),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-37, -34),
        p(-38, 21),
        p(-42, 70),
        p(-37, 85),
        p(-35, 103),
        p(-30, 108),
        p(-25, 118),
        p(-22, 125),
        p(-17, 129),
        p(-14, 131),
        p(-11, 135),
        p(-7, 137),
        p(-4, 138),
        p(-2, 143),
        p(1, 144),
        p(4, 146),
        p(5, 152),
        p(8, 151),
        p(17, 148),
        p(31, 140),
        p(36, 139),
        p(79, 115),
        p(78, 117),
        p(102, 97),
        p(192, 63),
        p(248, 16),
        p(294, -5),
        p(337, -35),
    ],
    [
        p(-89, 25),
        p(-55, 3),
        p(-28, -1),
        p(0, -2),
        p(29, -3),
        p(50, -5),
        p(75, 0),
        p(96, -2),
        p(140, -20),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-8, -5),
        p(23, 17),
        p(48, -14),
        p(21, -44),
        p(0, 0),
    ],
    [p(-2, 13), p(18, 21), p(-2, 7), p(29, 2), p(27, 56), p(0, 0)],
    [
        p(3, 18),
        p(21, 20),
        p(23, 21),
        p(-7, 10),
        p(43, -5),
        p(0, 0),
    ],
    [p(-0, -1), p(7, 12), p(-0, 29), p(0, 6), p(1, -18), p(0, 0)],
    [p(29, 21), p(-30, 21), p(3, 20), p(-32, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(11, 4), p(9, 10), p(16, 5), p(10, 16), p(13, 2)],
    [
        p(-3, 0),
        p(8, 18),
        p(-104, -36),
        p(6, 12),
        p(7, 16),
        p(5, 5),
    ],
    [p(2, 1), p(13, 3), p(9, 9), p(11, 7), p(12, 14), p(22, -5)],
    [
        p(3, -4),
        p(10, -2),
        p(9, -9),
        p(4, 15),
        p(-57, -258),
        p(7, -10),
    ],
    [
        p(58, -3),
        p(40, 4),
        p(45, -0),
        p(23, 3),
        p(35, -13),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-14, -17),
    p(16, -8),
    p(17, -2),
    p(21, -12),
    p(5, 23),
    p(-6, 13),
];
const PAWN_STORM: [PhasedScore; 8] = [
    p(-50, 5),
    p(-13, -19),
    p(-56, -24),
    p(-0, 3),
    p(5, 0),
    p(-3, 0),
    p(-8, 6),
    p(-8, 6),
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

    fn bishop_openness(
        openness: FileOpenness,
        len: usize,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

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

    fn king_zone_attack(
        attacking: UncoloredChessPiece,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn pawn_storm(rank_diff: usize) -> <Self::Score as ScoreType>::SingleFeatureScore;
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

    fn bishop_openness(
        openness: FileOpenness,
        len: usize,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore {
        BISHOP_OPENNESS[openness as usize][len - 1]
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

    fn king_zone_attack(
        attacking: UncoloredChessPiece,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore {
        KING_ZONE_ATTACK[attacking as usize]
    }

    fn pawn_storm(rank_diff: usize) -> <Self::Score as ScoreType>::SingleFeatureScore {
        PAWN_STORM[rank_diff]
    }
}
