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
        p( 126,  186),    p( 127,  186),    p( 114,  192),    p( 124,  176),    p( 113,  180),    p( 116,  182),    p(  79,  198),    p(  89,  195),
        p(  65,  125),    p(  64,  129),    p(  73,  119),    p(  80,  122),    p(  70,  124),    p( 117,  112),    p(  92,  138),    p(  83,  127),
        p(  51,  110),    p(  64,  105),    p(  60,   96),    p(  64,   88),    p(  79,   90),    p(  82,   87),    p(  74,  100),    p(  68,   92),
        p(  48,   96),    p(  59,   97),    p(  63,   85),    p(  72,   82),    p(  76,   82),    p(  78,   79),    p(  73,   86),    p(  59,   79),
        p(  40,   91),    p(  55,   86),    p(  54,   82),    p(  59,   88),    p(  68,   85),    p(  65,   79),    p(  73,   75),    p(  53,   76),
        p(  52,   95),    p(  64,   95),    p(  61,   87),    p(  60,   93),    p(  65,   97),    p(  79,   87),    p(  88,   80),    p(  56,   79),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 187,  271),    p( 209,  302),    p( 244,  314),    p( 268,  305),    p( 301,  306),    p( 214,  301),    p( 236,  299),    p( 217,  252),
        p( 276,  304),    p( 288,  312),    p( 302,  309),    p( 313,  312),    p( 306,  308),    p( 329,  297),    p( 287,  308),    p( 291,  295),
        p( 291,  301),    p( 302,  306),    p( 317,  315),    p( 321,  318),    p( 336,  312),    p( 360,  302),    p( 313,  303),    p( 309,  299),
        p( 304,  310),    p( 309,  308),    p( 316,  320),    p( 343,  322),    p( 319,  323),    p( 332,  321),    p( 314,  311),    p( 332,  304),
        p( 299,  313),    p( 298,  308),    p( 303,  320),    p( 310,  324),    p( 317,  325),    p( 314,  311),    p( 325,  303),    p( 314,  309),
        p( 274,  300),    p( 275,  303),    p( 282,  303),    p( 288,  317),    p( 295,  313),    p( 281,  298),    p( 296,  295),    p( 292,  304),
        p( 270,  305),    p( 280,  309),    p( 276,  304),    p( 289,  309),    p( 292,  303),    p( 283,  300),    p( 294,  301),    p( 289,  314),
        p( 243,  301),    p( 283,  299),    p( 266,  301),    p( 285,  307),    p( 296,  303),    p( 291,  293),    p( 289,  300),    p( 268,  299),
    ],
    // bishop
    [
        p( 280,  314),    p( 255,  314),    p( 250,  308),    p( 225,  315),    p( 225,  313),    p( 228,  307),    p( 284,  303),    p( 253,  308),
        p( 281,  303),    p( 289,  306),    p( 289,  306),    p( 281,  307),    p( 285,  303),    p( 297,  302),    p( 276,  307),    p( 276,  303),
        p( 300,  309),    p( 304,  304),    p( 295,  310),    p( 302,  302),    p( 306,  305),    p( 333,  308),    p( 319,  303),    p( 314,  311),
        p( 283,  310),    p( 299,  310),    p( 301,  305),    p( 318,  310),    p( 311,  306),    p( 306,  309),    p( 302,  308),    p( 282,  311),
        p( 292,  307),    p( 281,  311),    p( 299,  309),    p( 314,  308),    p( 311,  308),    p( 298,  306),    p( 290,  309),    p( 311,  299),
        p( 295,  306),    p( 304,  309),    p( 301,  308),    p( 304,  309),    p( 307,  310),    p( 303,  307),    p( 306,  301),    p( 311,  300),
        p( 309,  310),    p( 304,  300),    p( 311,  302),    p( 296,  310),    p( 302,  308),    p( 302,  305),    p( 313,  301),    p( 302,  298),
        p( 295,  304),    p( 313,  309),    p( 306,  306),    p( 289,  311),    p( 303,  308),    p( 295,  313),    p( 302,  299),    p( 300,  295),
    ],
    // rook
    [
        p( 460,  548),    p( 450,  558),    p( 448,  564),    p( 447,  561),    p( 459,  557),    p( 479,  552),    p( 486,  551),    p( 497,  543),
        p( 432,  554),    p( 430,  559),    p( 440,  560),    p( 456,  550),    p( 446,  552),    p( 467,  547),    p( 477,  544),    p( 489,  535),
        p( 435,  552),    p( 457,  547),    p( 453,  549),    p( 458,  544),    p( 484,  534),    p( 494,  529),    p( 517,  526),    p( 486,  529),
        p( 434,  552),    p( 444,  547),    p( 444,  550),    p( 450,  545),    p( 458,  537),    p( 468,  531),    p( 473,  533),    p( 469,  529),
        p( 430,  547),    p( 431,  546),    p( 431,  548),    p( 439,  545),    p( 444,  541),    p( 439,  539),    p( 458,  532),    p( 447,  530),
        p( 428,  544),    p( 427,  543),    p( 431,  542),    p( 433,  543),    p( 441,  537),    p( 449,  529),    p( 471,  516),    p( 452,  520),
        p( 431,  539),    p( 434,  539),    p( 440,  540),    p( 444,  538),    p( 451,  532),    p( 465,  522),    p( 473,  518),    p( 441,  527),
        p( 439,  543),    p( 435,  539),    p( 436,  544),    p( 441,  540),    p( 448,  533),    p( 454,  534),    p( 450,  531),    p( 446,  532),
    ],
    // queen
    [
        p( 873,  966),    p( 873,  980),    p( 888,  994),    p( 905,  990),    p( 904,  994),    p( 925,  980),    p( 974,  928),    p( 923,  957),
        p( 882,  957),    p( 859,  989),    p( 860, 1015),    p( 852, 1033),    p( 860, 1043),    p( 901, 1005),    p( 904,  984),    p( 946,  963),
        p( 891,  962),    p( 882,  982),    p( 881, 1006),    p( 879, 1014),    p( 902, 1016),    p( 943,  998),    p( 949,  969),    p( 937,  974),
        p( 875,  978),    p( 880,  989),    p( 874,  998),    p( 873, 1012),    p( 878, 1022),    p( 890, 1013),    p( 899, 1011),    p( 907,  987),
        p( 885,  968),    p( 872,  988),    p( 877,  991),    p( 878, 1009),    p( 879, 1006),    p( 881, 1005),    p( 895,  991),    p( 903,  981),
        p( 881,  954),    p( 886,  971),    p( 879,  988),    p( 876,  991),    p( 881,  998),    p( 888,  988),    p( 902,  969),    p( 902,  956),
        p( 884,  952),    p( 882,  961),    p( 888,  964),    p( 886,  977),    p( 887,  977),    p( 890,  960),    p( 900,  939),    p( 909,  912),
        p( 871,  948),    p( 881,  939),    p( 880,  954),    p( 888,  955),    p( 890,  949),    p( 879,  949),    p( 880,  938),    p( 884,  926),
    ],
    // king
    [
        p( 151, -105),    p(  61,  -53),    p(  82,  -44),    p(   6,  -13),    p(  29,  -25),    p(   8,  -14),    p(  60,  -25),    p( 215, -110),
        p( -20,   -6),    p( -63,   25),    p( -72,   35),    p(  -4,   24),    p( -38,   33),    p( -66,   46),    p( -38,   34),    p(   9,   -2),
        p( -38,    4),    p( -30,   21),    p( -74,   39),    p( -79,   46),    p( -46,   41),    p( -12,   34),    p( -52,   34),    p( -27,   10),
        p( -20,   -3),    p( -82,   20),    p( -99,   38),    p(-119,   46),    p(-117,   45),    p( -97,   38),    p(-105,   28),    p( -95,   14),
        p( -41,   -5),    p(-104,   15),    p(-115,   33),    p(-136,   45),    p(-141,   44),    p(-119,   31),    p(-132,   23),    p(-115,   12),
        p( -33,   -2),    p( -80,   11),    p(-110,   26),    p(-119,   34),    p(-114,   33),    p(-127,   27),    p( -98,   13),    p( -70,    8),
        p(  31,  -12),    p( -64,    5),    p( -76,   14),    p( -98,   23),    p(-102,   24),    p( -89,   17),    p( -56,    2),    p(   7,   -6),
        p(  41,  -47),    p(  41,  -50),    p(  34,  -34),    p( -25,  -17),    p(  28,  -34),    p( -22,  -17),    p(  30,  -40),    p(  59,  -53),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  26,   86),    p(  27,   86),    p(  14,   92),    p(  24,   76),    p(  13,   80),    p(  16,   82),    p( -21,   98),    p( -11,   95),
        p(  28,  116),    p(  37,  114),    p(  29,   97),    p(  15,   69),    p(  24,   68),    p(   7,   93),    p( -10,   96),    p( -39,  118),
        p(  11,   67),    p(   9,   67),    p(  18,   52),    p(  11,   44),    p(  -4,   44),    p(   1,   56),    p( -15,   71),    p( -18,   73),
        p(  -3,   40),    p( -11,   39),    p( -21,   34),    p( -12,   26),    p( -20,   29),    p( -18,   37),    p( -24,   50),    p( -17,   46),
        p(  -7,   10),    p( -20,   19),    p( -19,   17),    p( -18,    9),    p( -16,   13),    p( -15,   18),    p( -18,   32),    p(   3,   14),
        p( -15,   10),    p( -11,   14),    p( -12,   16),    p( -10,    5),    p(   2,   -0),    p(  -0,    7),    p(   6,   13),    p(   1,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(15, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -4);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(-2, -1);
const KING_OPEN_FILE: PhasedScore = p(-56, -5);
const KING_CLOSED_FILE: PhasedScore = p(8, -10);
const KING_SEMIOPEN_FILE: PhasedScore = p(-12, 3);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 5), p(-4, 7), p(-5, 8), p(2, 7), p(3, 9), p(4, 11), p(9, 10), p(20, 6), ],
    // Closed
    [p(0, 0), p(0, 0), p(16, -25), p(-13, 10), p(-2, 13), p(3, 4), p(2, 10), p(1, 6), ],
    // SemiOpen
    [p(0, 0), p(-16, 22), p(-1, 18), p(1, 14), p(-2, 18), p(3, 14), p(1, 11), p(12, 11), ],
    // SemiClosed
    [p(0, 0), p(12, -13), p(9, 7), p(6, 1), p(8, 5), p(4, 4), p(8, 7), p(3, 4), ],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-12, 5),   /*0b0000*/
    p(-17, 8),   /*0b0001*/
    p(-8, 3),    /*0b0010*/
    p(-3, 24),   /*0b0011*/
    p(-5, 4),    /*0b0100*/
    p(-28, -1),  /*0b0101*/
    p(-8, 14),   /*0b0110*/
    p(-8, -0),   /*0b0111*/
    p(-1, 10),   /*0b1000*/
    p(-26, -10), /*0b1001*/
    p(-6, 7),    /*0b1010*/
    p(-4, -0),   /*0b1011*/
    p(-7, 6),    /*0b1100*/
    p(-44, -10), /*0b1101*/
    p(-6, 18),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-8, 14),   /*0b10000*/
    p(3, 9),     /*0b10001*/
    p(-6, -15),  /*0b10010*/
    p(-7, -1),   /*0b10011*/
    p(-7, 5),    /*0b10100*/
    p(8, 14),    /*0b10101*/
    p(-25, -10), /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(9, 48),    /*0b11000*/
    p(18, 13),   /*0b11001*/
    p(22, 26),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(12, 21),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(7, 10),    /*0b100000*/
    p(-6, 13),   /*0b100001*/
    p(15, 1),    /*0b100010*/
    p(11, 13),   /*0b100011*/
    p(-34, -19), /*0b100100*/
    p(-43, -29), /*0b100101*/
    p(-31, 7),   /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(6, 5),     /*0b101000*/
    p(-29, 1),   /*0b101001*/
    p(10, -2),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-35, -12), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(13, 35),   /*0b110000*/
    p(22, 26),   /*0b110001*/
    p(14, -7),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(-10, 16),  /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(22, 43),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(-5, -7),   /*0b111111*/
    p(-25, -12), /*0b00*/
    p(12, -32),  /*0b01*/
    p(37, -19),  /*0b10*/
    p(38, -40),  /*0b11*/
    p(43, -20),  /*0b100*/
    p(-16, -62), /*0b101*/
    p(76, -52),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(50, -20),  /*0b1000*/
    p(22, -48),  /*0b1001*/
    p(62, -87),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(61, -11),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(7, -14),   /*0b1111*/
    p(14, -12),  /*0b00*/
    p(31, -25),  /*0b01*/
    p(27, -32),  /*0b10*/
    p(33, -43),  /*0b11*/
    p(28, -19),  /*0b100*/
    p(34, -53),  /*0b101*/
    p(21, -36),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(31, -15),  /*0b1000*/
    p(52, -30),  /*0b1001*/
    p(37, -83),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(41, -15),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(15, -66),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = [
    p(15, 13),
    p(3, 9),
    p(10, 14),
    p(7, 9),
    p(-4, 18),
    p(-42, 10),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(38, 10),
    p(41, 37),
    p(52, -10),
    p(37, -35),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-60, -58),
        p(-37, -20),
        p(-21, 2),
        p(-8, 13),
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
        p(-15, -31),
        p(-4, -15),
        p(2, -3),
        p(9, 6),
        p(13, 14),
        p(16, 18),
        p(18, 22),
        p(19, 26),
        p(26, 26),
        p(29, 24),
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
        p(-75, 17),
        p(-66, 30),
        p(-62, 34),
        p(-58, 38),
        p(-58, 44),
        p(-53, 48),
        p(-50, 52),
        p(-46, 54),
        p(-42, 58),
        p(-38, 61),
        p(-33, 63),
        p(-30, 67),
        p(-22, 67),
        p(-10, 64),
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
        p(-38, -34),
        p(-39, 21),
        p(-43, 70),
        p(-38, 85),
        p(-35, 103),
        p(-29, 108),
        p(-25, 118),
        p(-21, 125),
        p(-17, 129),
        p(-13, 131),
        p(-10, 134),
        p(-6, 137),
        p(-3, 137),
        p(-2, 142),
        p(1, 143),
        p(5, 145),
        p(5, 152),
        p(8, 151),
        p(17, 148),
        p(32, 139),
        p(36, 139),
        p(79, 116),
        p(79, 117),
        p(102, 97),
        p(192, 64),
        p(246, 17),
        p(295, -4),
        p(328, -31),
    ],
    [
        p(-77, 44),
        p(-47, 19),
        p(-23, 10),
        p(2, 5),
        p(28, -1),
        p(47, -9),
        p(69, -8),
        p(87, -16),
        p(131, -42),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(49, -14),
        p(21, -44),
        p(0, 0),
    ],
    [p(-2, 13), p(18, 21), p(-2, 7), p(28, 2), p(27, 57), p(0, 0)],
    [p(3, 18), p(21, 21), p(23, 21), p(-7, 9), p(43, -5), p(0, 0)],
    [p(-0, -0), p(7, 12), p(-1, 29), p(0, 6), p(1, -18), p(0, 0)],
    [p(65, 32), p(-30, 21), p(3, 18), p(-32, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(11, 4), p(9, 10), p(16, 5), p(10, 15), p(13, 2)],
    [
        p(-3, 1),
        p(8, 17),
        p(-104, -34),
        p(7, 12),
        p(7, 16),
        p(5, 5),
    ],
    [p(2, 1), p(13, 4), p(9, 9), p(11, 7), p(12, 14), p(22, -5)],
    [
        p(3, -4),
        p(10, -3),
        p(9, -9),
        p(4, 15),
        p(-56, -259),
        p(7, -10),
    ],
    [
        p(51, -5),
        p(37, -1),
        p(42, -6),
        p(21, -4),
        p(31, -18),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-11, -9),
    p(16, -8),
    p(17, -2),
    p(20, -11),
    p(4, 23),
    p(5, 20),
];
const PAWN_STORM: [PhasedScore; 7] = [
    p(0, 0),
    p(-15, -81),
    p(10, -42),
    p(2, -7),
    p(-9, 6),
    p(-15, 11),
    p(-11, 13),
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
