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
        p( 129,  188),    p( 130,  188),    p( 112,  193),    p( 124,  172),    p( 109,  176),    p( 115,  178),    p(  76,  200),    p( 102,  196),
        p(  63,  127),    p(  61,  134),    p(  67,  122),    p(  80,  120),    p(  69,  121),    p( 113,  109),    p(  99,  138),    p( 102,  122),
        p(  51,  111),    p(  63,  105),    p(  59,   97),    p(  63,   87),    p(  79,   88),    p(  81,   88),    p(  78,   98),    p(  75,   92),
        p(  48,   97),    p(  59,   97),    p(  62,   85),    p(  72,   81),    p(  76,   82),    p(  77,   78),    p(  72,   86),    p(  62,   79),
        p(  40,   92),    p(  56,   86),    p(  54,   82),    p(  59,   88),    p(  68,   85),    p(  60,   80),    p(  75,   74),    p(  55,   75),
        p(  52,   96),    p(  65,   95),    p(  61,   88),    p(  59,   95),    p(  64,   98),    p(  77,   88),    p(  88,   80),    p(  54,   78),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 187,  271),    p( 208,  301),    p( 244,  314),    p( 267,  305),    p( 300,  306),    p( 213,  302),    p( 235,  299),    p( 218,  252),
        p( 276,  303),    p( 288,  312),    p( 301,  309),    p( 312,  312),    p( 305,  308),    p( 329,  297),    p( 288,  308),    p( 292,  295),
        p( 291,  301),    p( 302,  306),    p( 317,  315),    p( 320,  318),    p( 336,  312),    p( 360,  302),    p( 313,  303),    p( 309,  299),
        p( 304,  310),    p( 309,  308),    p( 316,  320),    p( 342,  322),    p( 318,  323),    p( 332,  320),    p( 314,  311),    p( 332,  304),
        p( 299,  313),    p( 298,  308),    p( 303,  320),    p( 310,  324),    p( 317,  325),    p( 315,  311),    p( 325,  304),    p( 314,  309),
        p( 274,  300),    p( 275,  303),    p( 282,  304),    p( 288,  317),    p( 296,  314),    p( 282,  298),    p( 296,  295),    p( 292,  303),
        p( 271,  305),    p( 280,  309),    p( 276,  304),    p( 289,  309),    p( 292,  303),    p( 284,  300),    p( 294,  301),    p( 289,  314),
        p( 243,  301),    p( 283,  300),    p( 266,  301),    p( 285,  307),    p( 296,  303),    p( 291,  293),    p( 289,  299),    p( 267,  300),
    ],
    // bishop
    [
        p( 280,  314),    p( 255,  314),    p( 250,  308),    p( 224,  315),    p( 224,  314),    p( 228,  307),    p( 285,  303),    p( 254,  308),
        p( 281,  303),    p( 288,  306),    p( 289,  306),    p( 281,  307),    p( 285,  303),    p( 297,  302),    p( 278,  307),    p( 277,  303),
        p( 300,  309),    p( 304,  305),    p( 295,  309),    p( 302,  302),    p( 307,  305),    p( 333,  307),    p( 319,  303),    p( 314,  312),
        p( 283,  310),    p( 299,  309),    p( 301,  305),    p( 318,  310),    p( 311,  306),    p( 306,  309),    p( 302,  308),    p( 283,  311),
        p( 292,  307),    p( 282,  311),    p( 299,  309),    p( 314,  308),    p( 311,  308),    p( 298,  306),    p( 290,  309),    p( 311,  299),
        p( 295,  306),    p( 304,  309),    p( 301,  308),    p( 304,  309),    p( 307,  310),    p( 303,  307),    p( 306,  301),    p( 311,  299),
        p( 309,  310),    p( 304,  300),    p( 311,  302),    p( 296,  310),    p( 302,  308),    p( 302,  306),    p( 313,  301),    p( 302,  298),
        p( 296,  304),    p( 314,  309),    p( 306,  306),    p( 289,  311),    p( 303,  308),    p( 294,  313),    p( 303,  299),    p( 299,  296),
    ],
    // rook
    [
        p( 460,  548),    p( 451,  558),    p( 449,  564),    p( 447,  561),    p( 459,  557),    p( 479,  552),    p( 486,  551),    p( 498,  543),
        p( 433,  554),    p( 431,  559),    p( 441,  560),    p( 457,  550),    p( 447,  552),    p( 467,  546),    p( 478,  543),    p( 491,  535),
        p( 436,  552),    p( 457,  547),    p( 454,  549),    p( 459,  544),    p( 485,  533),    p( 495,  528),    p( 518,  526),    p( 486,  529),
        p( 434,  552),    p( 444,  547),    p( 444,  550),    p( 450,  545),    p( 459,  537),    p( 468,  531),    p( 474,  533),    p( 468,  530),
        p( 431,  547),    p( 432,  546),    p( 432,  548),    p( 439,  545),    p( 445,  541),    p( 440,  539),    p( 458,  532),    p( 448,  531),
        p( 428,  544),    p( 428,  542),    p( 431,  542),    p( 434,  543),    p( 441,  537),    p( 449,  529),    p( 472,  516),    p( 453,  520),
        p( 431,  539),    p( 435,  539),    p( 440,  540),    p( 444,  538),    p( 451,  532),    p( 465,  522),    p( 474,  517),    p( 442,  527),
        p( 439,  543),    p( 435,  538),    p( 437,  544),    p( 441,  540),    p( 449,  533),    p( 454,  534),    p( 451,  531),    p( 446,  531),
    ],
    // queen
    [
        p( 873,  966),    p( 873,  981),    p( 888,  994),    p( 905,  990),    p( 903,  995),    p( 925,  980),    p( 975,  927),    p( 925,  956),
        p( 882,  958),    p( 859,  989),    p( 860, 1016),    p( 852, 1033),    p( 860, 1043),    p( 901, 1004),    p( 907,  982),    p( 948,  961),
        p( 890,  963),    p( 882,  982),    p( 881, 1006),    p( 880, 1014),    p( 903, 1015),    p( 943,  998),    p( 950,  969),    p( 936,  975),
        p( 876,  977),    p( 880,  989),    p( 874,  998),    p( 873, 1012),    p( 879, 1022),    p( 891, 1013),    p( 899, 1011),    p( 907,  987),
        p( 885,  968),    p( 872,  988),    p( 877,  991),    p( 878, 1009),    p( 879, 1006),    p( 881, 1005),    p( 895,  991),    p( 903,  981),
        p( 881,  953),    p( 887,  971),    p( 879,  988),    p( 876,  991),    p( 881,  997),    p( 888,  987),    p( 902,  969),    p( 902,  956),
        p( 885,  951),    p( 883,  961),    p( 888,  964),    p( 886,  977),    p( 887,  976),    p( 890,  960),    p( 900,  939),    p( 909,  912),
        p( 871,  947),    p( 881,  939),    p( 881,  953),    p( 889,  954),    p( 891,  948),    p( 879,  948),    p( 881,  936),    p( 883,  927),
    ],
    // king
    [
        p( 149, -107),    p(  62,  -59),    p(  82,  -46),    p(   4,  -13),    p(  25,  -23),    p(   3,  -13),    p(  50,  -23),    p( 209, -112),
        p( -19,   -7),    p( -54,   19),    p( -66,   33),    p(   2,   23),    p( -34,   34),    p( -66,   48),    p( -42,   36),    p(   7,   -2),
        p( -37,    2),    p( -23,   15),    p( -68,   37),    p( -71,   45),    p( -40,   42),    p( -10,   36),    p( -55,   36),    p( -30,   10),
        p( -22,   -4),    p( -77,   14),    p( -94,   35),    p(-112,   45),    p(-110,   45),    p( -94,   40),    p(-107,   30),    p( -98,   15),
        p( -44,   -6),    p(-102,    8),    p(-114,   30),    p(-128,   43),    p(-132,   43),    p(-116,   32),    p(-133,   25),    p(-118,   13),
        p( -37,   -4),    p( -82,    4),    p(-110,   23),    p(-110,   32),    p(-104,   33),    p(-123,   28),    p( -99,   16),    p( -74,   10),
        p(  26,  -15),    p( -65,   -3),    p( -76,   11),    p( -87,   20),    p( -90,   23),    p( -83,   18),    p( -57,    4),    p(   2,   -4),
        p(  32,  -50),    p(  33,  -59),    p(  29,  -38),    p( -20,  -20),    p(  35,  -36),    p( -23,  -17),    p(  24,  -36),    p(  51,  -51),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  29,   88),    p(  30,   88),    p(  12,   93),    p(  24,   72),    p(   9,   76),    p(  15,   78),    p( -24,  100),    p(   2,   96),
        p(  28,  116),    p(  37,  115),    p(  32,   98),    p(  16,   69),    p(  25,   68),    p(   9,   92),    p( -11,   96),    p( -39,  118),
        p(  10,   68),    p(   9,   67),    p(  18,   53),    p(  11,   44),    p(  -4,   43),    p(   1,   56),    p( -16,   71),    p( -20,   73),
        p(  -4,   41),    p( -12,   39),    p( -21,   34),    p( -12,   26),    p( -20,   28),    p( -17,   36),    p( -23,   49),    p( -17,   45),
        p(  -7,   11),    p( -19,   19),    p( -19,   17),    p( -18,    8),    p( -17,   13),    p( -14,   17),    p( -18,   32),    p(   4,   13),
        p( -15,   10),    p( -11,   14),    p( -11,   17),    p( -10,    5),    p(   2,   -0),    p(  -1,    6),    p(   6,   12),    p(   4,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(15, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -4);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(-3, -0);
const KING_OPEN_FILE: PhasedScore = p(-56, -5);
const KING_CLOSED_FILE: PhasedScore = p(7, -10);
const KING_SEMIOPEN_FILE: PhasedScore = p(-12, 3);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 5), p(-4, 7), p(-5, 8), p(2, 7), p(3, 9), p(4, 11), p(9, 10), p(20, 7), ],
    // Closed
    [p(0, 0), p(0, 0), p(18, -24), p(-12, 9), p(-1, 13), p(3, 4), p(2, 10), p(1, 6), ],
    // SemiOpen
    [p(0, 0), p(-15, 22), p(1, 17), p(1, 14), p(-2, 18), p(3, 15), p(1, 11), p(12, 11), ],
    // SemiClosed
    [p(0, 0), p(12, -12), p(9, 7), p(6, 2), p(8, 5), p(4, 4), p(7, 7), p(2, 4), ],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-11, 5),   /*0b0000*/
    p(-17, 8),   /*0b0001*/
    p(-8, 2),    /*0b0010*/
    p(-4, 23),   /*0b0011*/
    p(-4, 4),    /*0b0100*/
    p(-26, -1),  /*0b0101*/
    p(-6, 14),   /*0b0110*/
    p(-6, -0),   /*0b0111*/
    p(2, 10),    /*0b1000*/
    p(-23, -11), /*0b1001*/
    p(-4, 7),    /*0b1010*/
    p(-6, 0),    /*0b1011*/
    p(-5, 7),    /*0b1100*/
    p(-37, -10), /*0b1101*/
    p(-2, 18),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-8, 14),   /*0b10000*/
    p(2, 9),     /*0b10001*/
    p(-6, -15),  /*0b10010*/
    p(-9, -1),   /*0b10011*/
    p(-6, 5),    /*0b10100*/
    p(9, 14),    /*0b10101*/
    p(-25, -8),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(11, 48),   /*0b11000*/
    p(20, 12),   /*0b11001*/
    p(24, 25),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(16, 22),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(8, 10),    /*0b100000*/
    p(-6, 13),   /*0b100001*/
    p(13, 1),    /*0b100010*/
    p(9, 13),    /*0b100011*/
    p(-32, -20), /*0b100100*/
    p(-42, -30), /*0b100101*/
    p(-31, 7),   /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(10, 4),    /*0b101000*/
    p(-26, 0),   /*0b101001*/
    p(11, -2),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-32, -12), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(13, 35),   /*0b110000*/
    p(21, 27),   /*0b110001*/
    p(10, -6),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(-8, 15),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(25, 43),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(-2, -9),   /*0b111111*/
    p(-29, -15), /*0b00*/
    p(10, -37),  /*0b01*/
    p(31, -22),  /*0b10*/
    p(32, -43),  /*0b11*/
    p(38, -23),  /*0b100*/
    p(-20, -63), /*0b101*/
    p(72, -57),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(37, -21),  /*0b1000*/
    p(19, -52),  /*0b1001*/
    p(57, -89),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(56, -13),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-1, -14),  /*0b1111*/
    p(7, -11),   /*0b00*/
    p(22, -22),  /*0b01*/
    p(22, -30),  /*0b10*/
    p(27, -39),  /*0b11*/
    p(18, -16),  /*0b100*/
    p(25, -48),  /*0b101*/
    p(14, -32),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(24, -13),  /*0b1000*/
    p(43, -27),  /*0b1001*/
    p(31, -80),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(32, -11),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(8, -62),   /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = [
    p(15, 13),
    p(3, 10),
    p(10, 14),
    p(8, 9),
    p(-4, 18),
    p(-43, 10),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(38, 10),
    p(41, 37),
    p(52, -9),
    p(37, -35),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-59, -58),
        p(-36, -20),
        p(-20, 2),
        p(-8, 13),
        p(3, 21),
        p(13, 29),
        p(24, 29),
        p(34, 28),
        p(43, 24),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-15, -31),
        p(-4, -15),
        p(2, -3),
        p(9, 6),
        p(13, 14),
        p(16, 18),
        p(18, 22),
        p(19, 26),
        p(26, 26),
        p(29, 25),
        p(38, 26),
        p(30, 34),
        p(44, 25),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-76, 16),
        p(-66, 29),
        p(-62, 33),
        p(-58, 38),
        p(-58, 44),
        p(-53, 48),
        p(-50, 52),
        p(-46, 54),
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
        p(-38, -35),
        p(-39, 23),
        p(-42, 71),
        p(-37, 85),
        p(-34, 103),
        p(-29, 108),
        p(-25, 118),
        p(-21, 125),
        p(-16, 129),
        p(-13, 131),
        p(-10, 134),
        p(-6, 137),
        p(-3, 138),
        p(-2, 142),
        p(1, 144),
        p(5, 146),
        p(5, 152),
        p(8, 151),
        p(17, 148),
        p(32, 140),
        p(36, 140),
        p(79, 116),
        p(79, 117),
        p(102, 97),
        p(193, 63),
        p(248, 16),
        p(290, -1),
        p(338, -35),
    ],
    [
        p(-70, 42),
        p(-42, 19),
        p(-21, 11),
        p(1, 6),
        p(25, -0),
        p(42, -8),
        p(61, -7),
        p(78, -15),
        p(119, -40),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-7, -5),
        p(23, 17),
        p(48, -14),
        p(21, -44),
        p(0, 0),
    ],
    [p(-2, 13), p(18, 21), p(-2, 7), p(29, 2), p(26, 57), p(0, 0)],
    [p(3, 18), p(21, 21), p(23, 21), p(-7, 9), p(43, -5), p(0, 0)],
    [p(-0, -1), p(7, 12), p(-1, 29), p(0, 6), p(1, -18), p(0, 0)],
    [p(66, 31), p(-29, 21), p(4, 18), p(-32, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(11, 4), p(9, 10), p(15, 5), p(10, 15), p(13, 2)],
    [
        p(-3, 1),
        p(8, 17),
        p(-104, -32),
        p(7, 12),
        p(7, 16),
        p(5, 5),
    ],
    [p(2, 1), p(13, 4), p(9, 9), p(11, 7), p(12, 15), p(22, -5)],
    [
        p(3, -4),
        p(10, -2),
        p(9, -9),
        p(4, 15),
        p(-55, -261),
        p(7, -10),
    ],
    [
        p(50, -5),
        p(34, -1),
        p(40, -5),
        p(18, -3),
        p(29, -18),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-9, -9),
    p(16, -8),
    p(16, -2),
    p(20, -11),
    p(4, 23),
    p(6, 20),
];
const PAWN_STORM: [PhasedScore; NUM_SQUARES] = [
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(-99, -103),
    p(-55, -104),
    p(56, -102),
    p(50, -63),
    p(101, -64),
    p(31, -57),
    p(19, -97),
    p(-63, -109),
    p(22, -65),
    p(27, -71),
    p(71, -68),
    p(21, -32),
    p(36, -35),
    p(27, -29),
    p(-8, -45),
    p(-37, -29),
    p(-1, -26),
    p(5, -12),
    p(7, -15),
    p(11, -2),
    p(-2, 4),
    p(7, -9),
    p(-14, -2),
    p(-21, -8),
    p(-16, -1),
    p(-18, 4),
    p(-4, 2),
    p(-9, 11),
    p(-12, 9),
    p(-4, 6),
    p(-9, 9),
    p(-23, 10),
    p(-17, 6),
    p(-37, 15),
    p(-6, 6),
    p(-14, 10),
    p(-14, 9),
    p(-1, 8),
    p(-26, 17),
    p(-28, 16),
    p(-9, 7),
    p(-29, 17),
    p(-10, 5),
    p(-5, 5),
    p(-3, 6),
    p(-9, 10),
    p(-17, 17),
    p(-12, 19),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];

// Here end the constants, now follows code that should not be overwritten when tuning different values.

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

    fn pawn_storm(square: ChessSquare) -> <Self::Score as ScoreType>::SingleFeatureScore;
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

    fn pawn_storm(square: ChessSquare) -> <Self::Score as ScoreType>::SingleFeatureScore {
        PAWN_STORM[square.bb_idx()]
    }
}
