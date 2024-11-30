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
        p( 132,  187),    p( 129,  186),    p( 119,  189),    p( 132,  169),    p( 118,  174),    p( 119,  177),    p(  82,  195),    p(  90,  193),
        p(  63,  124),    p(  60,  125),    p(  73,  120),    p(  81,  125),    p(  66,  125),    p( 116,  111),    p(  90,  132),    p(  87,  122),
        p(  49,  114),    p(  61,  110),    p(  59,  104),    p(  63,   97),    p(  80,   98),    p(  81,   95),    p(  74,  104),    p(  69,   96),
        p(  46,  100),    p(  53,  103),    p(  62,   96),    p(  71,   94),    p(  74,   93),    p(  75,   89),    p(  68,   93),    p(  57,   87),
        p(  41,   98),    p(  49,   95),    p(  53,   95),    p(  57,  100),    p(  66,   97),    p(  60,   94),    p(  68,   85),    p(  52,   86),
        p(  47,   99),    p(  50,   97),    p(  56,   99),    p(  55,  106),    p(  52,  109),    p(  71,   99),    p(  71,   85),    p(  53,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 184,  272),    p( 209,  304),    p( 243,  315),    p( 267,  306),    p( 299,  307),    p( 213,  303),    p( 230,  303),    p( 212,  255),
        p( 275,  306),    p( 287,  314),    p( 301,  310),    p( 315,  311),    p( 307,  308),    p( 328,  297),    p( 285,  310),    p( 289,  297),
        p( 291,  303),    p( 302,  307),    p( 320,  315),    p( 321,  319),    p( 338,  312),    p( 361,  302),    p( 314,  303),    p( 308,  301),
        p( 303,  312),    p( 310,  309),    p( 318,  319),    p( 344,  322),    p( 321,  323),    p( 334,  320),    p( 315,  312),    p( 332,  306),
        p( 299,  315),    p( 299,  308),    p( 306,  319),    p( 313,  323),    p( 320,  324),    p( 317,  310),    p( 327,  304),    p( 315,  311),
        p( 273,  302),    p( 277,  304),    p( 285,  302),    p( 290,  317),    p( 299,  313),    p( 285,  296),    p( 297,  296),    p( 292,  306),
        p( 270,  308),    p( 280,  311),    p( 277,  305),    p( 289,  310),    p( 292,  304),    p( 284,  300),    p( 293,  303),    p( 288,  317),
        p( 242,  303),    p( 280,  302),    p( 265,  303),    p( 284,  309),    p( 294,  307),    p( 291,  295),    p( 286,  303),    p( 264,  302),
    ],
    // bishop
    [
        p( 281,  315),    p( 258,  314),    p( 249,  306),    p( 225,  315),    p( 223,  315),    p( 227,  306),    p( 283,  304),    p( 252,  308),
        p( 281,  303),    p( 286,  305),    p( 288,  307),    p( 281,  308),    p( 284,  304),    p( 293,  303),    p( 272,  307),    p( 273,  304),
        p( 298,  309),    p( 303,  305),    p( 295,  311),    p( 303,  303),    p( 307,  306),    p( 332,  309),    p( 318,  304),    p( 312,  311),
        p( 281,  311),    p( 298,  309),    p( 301,  305),    p( 318,  311),    p( 313,  307),    p( 306,  309),    p( 302,  308),    p( 282,  311),
        p( 292,  308),    p( 280,  311),    p( 300,  309),    p( 316,  307),    p( 314,  307),    p( 300,  306),    p( 291,  309),    p( 312,  300),
        p( 293,  308),    p( 304,  310),    p( 300,  310),    p( 304,  309),    p( 308,  310),    p( 305,  306),    p( 306,  301),    p( 310,  301),
        p( 307,  312),    p( 303,  301),    p( 310,  303),    p( 296,  310),    p( 302,  309),    p( 303,  306),    p( 312,  301),    p( 302,  299),
        p( 294,  305),    p( 313,  309),    p( 306,  307),    p( 289,  312),    p( 303,  310),    p( 295,  314),    p( 304,  298),    p( 300,  296),
    ],
    // rook
    [
        p( 460,  548),    p( 452,  558),    p( 449,  563),    p( 447,  561),    p( 459,  556),    p( 479,  551),    p( 488,  549),    p( 499,  542),
        p( 435,  554),    p( 432,  560),    p( 440,  561),    p( 456,  550),    p( 446,  552),    p( 466,  547),    p( 478,  544),    p( 492,  535),
        p( 439,  551),    p( 457,  546),    p( 455,  547),    p( 459,  543),    p( 486,  532),    p( 494,  529),    p( 516,  526),    p( 489,  528),
        p( 436,  550),    p( 444,  546),    p( 444,  549),    p( 450,  544),    p( 459,  535),    p( 468,  531),    p( 474,  533),    p( 471,  528),
        p( 431,  547),    p( 431,  545),    p( 433,  546),    p( 439,  543),    p( 445,  539),    p( 440,  538),    p( 460,  531),    p( 448,  530),
        p( 428,  543),    p( 428,  541),    p( 432,  540),    p( 434,  540),    p( 442,  533),    p( 450,  526),    p( 473,  514),    p( 454,  519),
        p( 430,  538),    p( 435,  537),    p( 441,  539),    p( 443,  536),    p( 452,  529),    p( 466,  519),    p( 474,  515),    p( 442,  524),
        p( 440,  541),    p( 437,  537),    p( 438,  542),    p( 443,  537),    p( 451,  531),    p( 457,  530),    p( 454,  528),    p( 448,  529),
    ],
    // queen
    [
        p( 880,  963),    p( 883,  977),    p( 897,  989),    p( 914,  986),    p( 911,  990),    p( 931,  977),    p( 983,  925),    p( 927,  957),
        p( 888,  958),    p( 862,  990),    p( 864, 1017),    p( 856, 1034),    p( 863, 1045),    p( 903, 1005),    p( 907,  986),    p( 950,  964),
        p( 895,  962),    p( 887,  982),    p( 887, 1005),    p( 885, 1013),    p( 908, 1015),    p( 947,  998),    p( 954,  968),    p( 943,  974),
        p( 880,  977),    p( 886,  987),    p( 881,  997),    p( 879, 1012),    p( 885, 1021),    p( 897, 1012),    p( 907, 1010),    p( 914,  987),
        p( 891,  968),    p( 877,  986),    p( 885,  989),    p( 886, 1007),    p( 887, 1005),    p( 890, 1004),    p( 903,  990),    p( 910,  982),
        p( 886,  953),    p( 892,  970),    p( 885,  986),    p( 884,  989),    p( 890,  995),    p( 896,  987),    p( 911,  969),    p( 909,  956),
        p( 886,  953),    p( 885,  961),    p( 893,  963),    p( 892,  976),    p( 893,  977),    p( 896,  958),    p( 905,  937),    p( 913,  911),
        p( 873,  949),    p( 883,  940),    p( 883,  954),    p( 892,  955),    p( 895,  949),    p( 884,  948),    p( 884,  937),    p( 888,  924),
    ],
    // king
    [
        p( 153, -102),    p(  56,  -49),    p(  80,  -41),    p(   3,   -9),    p(  25,  -21),    p(   6,  -11),    p(  62,  -20),    p( 218, -104),
        p( -22,   -3),    p( -68,   26),    p( -76,   35),    p( -13,   25),    p( -46,   34),    p( -72,   47),    p( -37,   32),    p(   7,   -1),
        p( -44,    5),    p( -37,   22),    p( -80,   40),    p( -86,   47),    p( -53,   42),    p( -19,   33),    p( -57,   32),    p( -31,   10),
        p( -26,   -1),    p( -90,   21),    p(-105,   39),    p(-128,   48),    p(-126,   45),    p(-105,   37),    p(-111,   26),    p( -97,   14),
        p( -45,   -4),    p(-112,   17),    p(-121,   33),    p(-145,   47),    p(-150,   45),    p(-126,   30),    p(-139,   22),    p(-116,   12),
        p( -36,   -0),    p( -88,   12),    p(-118,   26),    p(-125,   36),    p(-120,   34),    p(-135,   27),    p(-106,   12),    p( -74,    9),
        p(  28,   -9),    p( -71,    6),    p( -83,   15),    p(-103,   24),    p(-109,   25),    p( -94,   16),    p( -63,   -0),    p(   4,   -4),
        p(  44,  -40),    p(  41,  -46),    p(  36,  -35),    p( -24,  -14),    p(  29,  -32),    p( -21,  -18),    p(  34,  -42),    p(  60,  -50),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(16, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(1, 2);
const KING_OPEN_FILE: PhasedScore = p(-56, -1);
const KING_CLOSED_FILE: PhasedScore = p(16, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 6);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 5), p(-3, 7), p(-2, 10), p(3, 7), p(4, 10), p(4, 12), p(10, 11), p(21, 7)],
    // Closed
    [p(0, 0), p(0, 0), p(10, -29), p(-15, 9), p(1, 12), p(2, 4), p(3, 9), p(-0, 6)],
    // SemiOpen
    [p(0, 0), p(-16, 21), p(3, 19), p(2, 13), p(0, 17), p(5, 13), p(3, 10), p(12, 10)],
    // SemiClosed
    [p(0, 0), p(10, -12), p(7, 7), p(5, 2), p(9, 5), p(3, 5), p(8, 8), p(2, 5)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-6, 7),    /*0b0000*/
    p(-14, 12),  /*0b0001*/
    p(-2, 8),    /*0b0010*/
    p(-10, 14),  /*0b0011*/
    p(-4, 7),    /*0b0100*/
    p(-26, 4),   /*0b0101*/
    p(-14, 6),   /*0b0110*/
    p(-20, -17), /*0b0111*/
    p(7, 11),    /*0b1000*/
    p(-5, 12),   /*0b1001*/
    p(2, 9),     /*0b1010*/
    p(-4, 11),   /*0b1011*/
    p(-1, 7),    /*0b1100*/
    p(-25, 11),  /*0b1101*/
    p(-12, 4),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(2, 18),    /*0b10000*/
    p(4, 12),    /*0b10001*/
    p(22, 13),   /*0b10010*/
    p(-5, 9),    /*0b10011*/
    p(-5, 8),    /*0b10100*/
    p(13, 17),   /*0b10101*/
    p(-23, 2),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(12, 33),   /*0b11000*/
    p(30, 26),   /*0b11001*/
    p(42, 40),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(15, 13),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(15, 10),   /*0b100000*/
    p(5, 15),    /*0b100001*/
    p(25, 4),    /*0b100010*/
    p(6, 0),     /*0b100011*/
    p(-9, 4),    /*0b100100*/
    p(-23, -6),  /*0b100101*/
    p(-25, 16),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(19, 3),    /*0b101000*/
    p(-1, 19),   /*0b101001*/
    p(19, -3),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-7, 8),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(15, 21),   /*0b110000*/
    p(26, 17),   /*0b110001*/
    p(32, 12),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(8, 32),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(23, 17),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(4, -1),    /*0b111111*/
    p(-21, -9),  /*0b00*/
    p(9, -24),   /*0b01*/
    p(37, -13),  /*0b10*/
    p(23, -48),  /*0b11*/
    p(47, -18),  /*0b100*/
    p(-6, -26),  /*0b101*/
    p(73, -47),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(56, -19),  /*0b1000*/
    p(18, -42),  /*0b1001*/
    p(77, -62),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(57, -26),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(12, -3),   /*0b1111*/
    p(17, -10),  /*0b00*/
    p(32, -19),  /*0b01*/
    p(25, -26),  /*0b10*/
    p(22, -51),  /*0b11*/
    p(33, -18),  /*0b100*/
    p(53, -28),  /*0b101*/
    p(23, -33),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(38, -12),  /*0b1000*/
    p(53, -25),  /*0b1001*/
    p(51, -51),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(42, -31),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(20, -53),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  32,   87),    p(  29,   86),    p(  19,   89),    p(  32,   69),    p(  18,   74),    p(  19,   77),    p( -18,   95),    p( -10,   93),
        p(  41,  123),    p(  47,  123),    p(  36,   99),    p(  21,   67),    p(  35,   67),    p(  14,   95),    p(   0,  103),    p( -28,  124),
        p(  24,   72),    p(  17,   70),    p(  23,   53),    p(  16,   43),    p(  -2,   45),    p(   7,   58),    p( -11,   75),    p( -10,   77),
        p(   8,   46),    p(  -3,   43),    p( -15,   34),    p(  -9,   24),    p( -16,   29),    p( -10,   38),    p( -18,   54),    p( -10,   50),
        p(   2,   14),    p( -12,   23),    p( -14,   16),    p( -16,    9),    p( -14,   13),    p(  -8,   17),    p( -13,   36),    p(   9,   16),
        p(  -4,   15),    p(  -2,   20),    p(  -9,   16),    p(  -8,    5),    p(   5,    1),    p(   7,    7),    p(  12,   19),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-11, -10);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(11, 7), p(0, 9), p(8, 14), p(6, 10), p(-7, 18), p(-50, 9)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(42, 5),
    p(46, 32),
    p(55, -13),
    p(41, -43),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-56, -60),
        p(-34, -21),
        p(-19, 1),
        p(-8, 13),
        p(2, 22),
        p(12, 31),
        p(22, 32),
        p(32, 31),
        p(40, 26),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-24, -50),
        p(-13, -31),
        p(-4, -15),
        p(3, -3),
        p(9, 6),
        p(13, 14),
        p(15, 19),
        p(18, 22),
        p(18, 27),
        p(25, 27),
        p(28, 26),
        p(37, 25),
        p(29, 34),
        p(42, 25),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-58, 37),
        p(-59, 43),
        p(-54, 48),
        p(-51, 53),
        p(-47, 56),
        p(-43, 60),
        p(-40, 64),
        p(-35, 66),
        p(-31, 69),
        p(-22, 68),
        p(-9, 64),
        p(-5, 64),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-30, -38),
        p(-30, 17),
        p(-33, 65),
        p(-28, 83),
        p(-26, 100),
        p(-21, 106),
        p(-17, 117),
        p(-14, 124),
        p(-9, 128),
        p(-6, 131),
        p(-4, 135),
        p(0, 138),
        p(3, 139),
        p(4, 144),
        p(7, 145),
        p(10, 148),
        p(11, 154),
        p(14, 152),
        p(24, 149),
        p(38, 140),
        p(44, 138),
        p(89, 111),
        p(87, 114),
        p(114, 90),
        p(205, 55),
        p(258, 8),
        p(287, -5),
        p(352, -51),
    ],
    [
        p(-82, 46),
        p(-51, 18),
        p(-26, 10),
        p(-0, 4),
        p(28, -1),
        p(47, -9),
        p(70, -8),
        p(92, -15),
        p(136, -40),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-9, 8),
        p(-5, -5),
        p(26, 13),
        p(51, -19),
        p(25, -51),
        p(0, 0),
    ],
    [p(1, 11), p(21, 18), p(0, 7), p(32, -2), p(29, 51), p(0, 0)],
    [p(5, 15), p(24, 18), p(26, 19), p(-5, 9), p(43, -7), p(0, 0)],
    [p(2, -5), p(10, 9), p(3, 26), p(3, 1), p(2, -17), p(0, 0)],
    [p(80, 31), p(-29, 19), p(5, 16), p(-31, 6), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(4, 6), p(10, 4), p(8, 9), p(14, 4), p(8, 13), p(12, 3)],
    [p(-4, 0), p(6, 18), p(-96, -33), p(5, 12), p(6, 13), p(3, 5)],
    [p(1, 3), p(12, 5), p(7, 11), p(9, 7), p(9, 14), p(19, -5)],
    [
        p(2, -5),
        p(8, -2),
        p(6, -9),
        p(3, 12),
        p(-64, -256),
        p(4, -10),
    ],
    [
        p(59, -7),
        p(37, 0),
        p(42, -5),
        p(21, -2),
        p(33, -19),
        p(0, 0),
    ],
];

pub const NUM_DEFENDED: [PhasedScore; 17] = [
    p(32, -2),
    p(-18, -1),
    p(-25, -2),
    p(-22, -3),
    p(-15, -5),
    p(-14, -4),
    p(-14, -2),
    p(-9, -1),
    p(-5, 2),
    p(0, 4),
    p(3, 11),
    p(7, 18),
    p(11, 21),
    p(13, 29),
    p(15, 19),
    p(14, 1),
    p(13, 340),
];

pub const NUM_ATTACKED: [PhasedScore; 9] = [
    p(12, -8),
    p(4, -6),
    p(-1, -1),
    p(-5, 6),
    p(-8, 18),
    p(-11, 35),
    p(-13, 55),
    p(-13, 50),
    p(-8, 38),
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-12, -11),
    p(16, -8),
    p(17, -3),
    p(23, -13),
    p(5, 23),
    p(6, 18),
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

    fn num_defended(num: usize) -> SingleFeatureScore<Self::Score>;

    fn num_attacked(num: usize) -> SingleFeatureScore<Self::Score>;

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

    fn bishop_pair() -> PhasedScore {
        BISHOP_PAIR
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

    fn num_defended(num: usize) -> SingleFeatureScore<Self::Score> {
        NUM_DEFENDED[num]
    }

    fn num_attacked(num: usize) -> SingleFeatureScore<Self::Score> {
        NUM_ATTACKED[num]
    }

    fn king_zone_attack(attacking: ChessPieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }
}
