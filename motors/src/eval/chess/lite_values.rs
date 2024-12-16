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
        p( 133,  186),    p( 130,  185),    p( 121,  188),    p( 133,  169),    p( 119,  173),    p( 119,  176),    p(  81,  194),    p(  89,  193),
        p(  67,  123),    p(  66,  124),    p(  77,  120),    p(  86,  123),    p(  74,  123),    p( 122,  110),    p(  96,  130),    p(  93,  121),
        p(  55,  112),    p(  66,  108),    p(  64,  104),    p(  67,   98),    p(  82,   98),    p(  87,   94),    p(  79,  103),    p(  74,   95),
        p(  51,   99),    p(  58,  102),    p(  67,   94),    p(  76,   93),    p(  79,   93),    p(  79,   88),    p(  73,   92),    p(  62,   86),
        p(  46,   97),    p(  55,   92),    p(  59,   93),    p(  61,   99),    p(  69,   96),    p(  63,   92),    p(  72,   82),    p(  55,   85),
        p(  52,   99),    p(  55,   95),    p(  61,   97),    p(  60,  105),    p(  57,  107),    p(  73,   98),    p(  75,   83),    p(  58,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 176,  277),    p( 197,  309),    p( 214,  320),    p( 252,  310),    p( 282,  311),    p( 198,  307),    p( 212,  308),    p( 204,  260),
        p( 269,  310),    p( 283,  315),    p( 298,  307),    p( 302,  311),    p( 301,  306),    p( 315,  295),    p( 276,  312),    p( 272,  302),
        p( 286,  306),    p( 303,  302),    p( 305,  309),    p( 320,  312),    p( 336,  306),    p( 349,  295),    p( 291,  302),    p( 285,  307),
        p( 301,  314),    p( 307,  308),    p( 322,  312),    p( 325,  319),    p( 323,  316),    p( 318,  315),    p( 309,  311),    p( 318,  310),
        p( 298,  316),    p( 302,  306),    p( 311,  312),    p( 319,  315),    p( 317,  318),    p( 322,  302),    p( 320,  302),    p( 312,  311),
        p( 274,  303),    p( 281,  301),    p( 293,  296),    p( 299,  309),    p( 304,  307),    p( 292,  289),    p( 300,  292),    p( 292,  306),
        p( 269,  311),    p( 280,  312),    p( 283,  302),    p( 293,  306),    p( 297,  301),    p( 287,  299),    p( 294,  304),    p( 289,  320),
        p( 239,  310),    p( 281,  303),    p( 266,  304),    p( 286,  309),    p( 295,  306),    p( 291,  296),    p( 288,  305),    p( 265,  307),
    ],
    // bishop
    [
        p( 276,  310),    p( 254,  314),    p( 239,  306),    p( 223,  316),    p( 218,  312),    p( 223,  308),    p( 274,  303),    p( 251,  309),
        p( 282,  303),    p( 278,  303),    p( 290,  305),    p( 278,  303),    p( 288,  300),    p( 292,  298),    p( 268,  308),    p( 271,  302),
        p( 295,  309),    p( 306,  304),    p( 292,  304),    p( 306,  298),    p( 305,  299),    p( 336,  304),    p( 317,  300),    p( 316,  313),
        p( 285,  312),    p( 292,  306),    p( 303,  302),    p( 307,  306),    p( 307,  303),    p( 299,  304),    p( 297,  308),    p( 279,  309),
        p( 290,  307),    p( 283,  309),    p( 295,  303),    p( 309,  305),    p( 302,  301),    p( 298,  303),    p( 286,  304),    p( 308,  301),
        p( 296,  310),    p( 300,  304),    p( 300,  307),    p( 300,  304),    p( 306,  307),    p( 300,  298),    p( 305,  295),    p( 307,  299),
        p( 308,  309),    p( 303,  301),    p( 309,  300),    p( 298,  309),    p( 301,  305),    p( 304,  303),    p( 312,  295),    p( 308,  297),
        p( 297,  305),    p( 311,  306),    p( 307,  307),    p( 291,  308),    p( 306,  307),    p( 294,  309),    p( 306,  296),    p( 302,  292),
    ],
    // rook
    [
        p( 457,  547),    p( 447,  557),    p( 440,  563),    p( 438,  561),    p( 450,  557),    p( 469,  551),    p( 481,  549),    p( 491,  542),
        p( 442,  553),    p( 440,  558),    p( 449,  559),    p( 464,  550),    p( 450,  553),    p( 467,  547),    p( 475,  544),    p( 490,  535),
        p( 444,  548),    p( 461,  543),    p( 456,  545),    p( 456,  540),    p( 482,  529),    p( 492,  527),    p( 509,  526),    p( 484,  528),
        p( 440,  548),    p( 446,  544),    p( 445,  546),    p( 451,  540),    p( 456,  532),    p( 466,  528),    p( 466,  532),    p( 466,  527),
        p( 434,  546),    p( 433,  544),    p( 433,  544),    p( 438,  541),    p( 445,  537),    p( 439,  536),    p( 453,  530),    p( 447,  528),
        p( 430,  543),    p( 429,  540),    p( 431,  539),    p( 434,  538),    p( 439,  533),    p( 450,  524),    p( 466,  513),    p( 453,  517),
        p( 432,  538),    p( 435,  537),    p( 441,  538),    p( 443,  535),    p( 451,  528),    p( 463,  519),    p( 471,  514),    p( 442,  523),
        p( 441,  542),    p( 437,  537),    p( 439,  541),    p( 443,  535),    p( 448,  528),    p( 455,  529),    p( 452,  527),    p( 447,  529),
    ],
    // queen
    [
        p( 878,  961),    p( 881,  975),    p( 895,  988),    p( 916,  982),    p( 914,  985),    p( 934,  973),    p( 979,  926),    p( 924,  958),
        p( 888,  951),    p( 863,  981),    p( 865, 1007),    p( 857, 1025),    p( 865, 1035),    p( 906,  996),    p( 906,  981),    p( 947,  960),
        p( 893,  957),    p( 885,  973),    p( 885,  993),    p( 886, 1002),    p( 909, 1004),    p( 946,  988),    p( 954,  959),    p( 942,  966),
        p( 879,  969),    p( 885,  976),    p( 879,  986),    p( 880,  998),    p( 883, 1010),    p( 896, 1000),    p( 905, 1002),    p( 912,  978),
        p( 890,  960),    p( 877,  980),    p( 884,  980),    p( 884,  996),    p( 887,  992),    p( 888,  994),    p( 901,  982),    p( 908,  975),
        p( 885,  950),    p( 892,  965),    p( 887,  980),    p( 884,  982),    p( 889,  989),    p( 896,  978),    p( 909,  962),    p( 907,  950),
        p( 886,  951),    p( 886,  959),    p( 893,  962),    p( 892,  976),    p( 894,  975),    p( 894,  958),    p( 906,  936),    p( 914,  909),
        p( 872,  952),    p( 884,  940),    p( 885,  953),    p( 893,  954),    p( 896,  943),    p( 882,  948),    p( 885,  939),    p( 889,  923),
    ],
    // king
    [
        p( 157,  -84),    p(  60,  -38),    p(  85,  -30),    p(   8,    2),    p(  38,  -11),    p(  23,   -1),    p(  76,  -10),    p( 236,  -88),
        p( -30,    2),    p( -80,   19),    p( -81,   26),    p( -21,   16),    p( -51,   23),    p( -81,   38),    p( -50,   24),    p(   9,    0),
        p( -46,    9),    p( -47,   13),    p( -85,   28),    p( -95,   36),    p( -63,   31),    p( -32,   23),    p( -78,   25),    p( -37,   11),
        p( -26,    2),    p(-101,   12),    p(-114,   28),    p(-136,   37),    p(-136,   35),    p(-115,   27),    p(-134,   17),    p(-106,   17),
        p( -42,   -2),    p(-114,    8),    p(-126,   24),    p(-150,   38),    p(-153,   36),    p(-128,   22),    p(-144,   12),    p(-118,   13),
        p( -33,    2),    p( -91,    3),    p(-119,   18),    p(-126,   27),    p(-123,   26),    p(-133,   18),    p(-108,    4),    p( -73,   10),
        p(  26,   -8),    p( -77,   -3),    p( -89,    7),    p(-109,   16),    p(-114,   17),    p( -99,    8),    p( -72,  -10),    p(   4,   -4),
        p(  55,  -24),    p(  43,  -36),    p(  40,  -23),    p( -22,   -2),    p(  29,  -19),    p( -18,   -6),    p(  36,  -30),    p(  67,  -34),
    ],
];

const BISHOP_PAIR: PhasedScore = p(22, 53);
const BAD_BISHOP: [PhasedScore; 9] = [
    p(9, 19),
    p(10, 17),
    p(10, 6),
    p(7, -2),
    p(3, -10),
    p(-1, -20),
    p(-8, -29),
    p(-16, -42),
    p(-28, -53),
];
const ROOK_OPEN_FILE: PhasedScore = p(13, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -0);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 4);
const KING_OPEN_FILE: PhasedScore = p(-49, -1);
const KING_CLOSED_FILE: PhasedScore = p(14, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 3), p(-1, 5), p(-1, 4), p(2, 3), p(2, 5), p(2, 7), p(6, 4), p(18, 0)],
    // Closed
    [p(0, 0), p(0, 0), p(16, -24), p(-16, 9), p(-1, 11), p(2, 4), p(-0, 7), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-16, 22), p(4, 16), p(1, 9), p(-0, 9), p(3, 4), p(-0, 2), p(10, 4)],
    // SemiClosed
    [p(0, 0), p(11, -13), p(7, 5), p(3, 0), p(7, 2), p(3, 4), p(5, 5), p(1, 4)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-5, 5),    /*0b0000*/
    p(-15, 8),   /*0b0001*/
    p(-3, 8),    /*0b0010*/
    p(-11, 13),  /*0b0011*/
    p(-4, 3),    /*0b0100*/
    p(-26, -1),  /*0b0101*/
    p(-15, 5),   /*0b0110*/
    p(-21, -15), /*0b0111*/
    p(9, 10),    /*0b1000*/
    p(-3, 10),   /*0b1001*/
    p(3, 11),    /*0b1010*/
    p(-3, 10),   /*0b1011*/
    p(-0, 4),    /*0b1100*/
    p(-23, 9),   /*0b1101*/
    p(-12, 4),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(3, 15),    /*0b10000*/
    p(2, 8),     /*0b10001*/
    p(20, 10),   /*0b10010*/
    p(-7, 6),    /*0b10011*/
    p(-6, 6),    /*0b10100*/
    p(12, 15),   /*0b10101*/
    p(-25, 1),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(16, 29),   /*0b11000*/
    p(29, 23),   /*0b11001*/
    p(42, 38),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(17, 10),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(16, 10),   /*0b100000*/
    p(3, 13),    /*0b100001*/
    p(25, 3),    /*0b100010*/
    p(6, -1),    /*0b100011*/
    p(-7, 2),    /*0b100100*/
    p(-21, -7),  /*0b100101*/
    p(-22, 15),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(24, 4),    /*0b101000*/
    p(-0, 17),   /*0b101001*/
    p(22, -2),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-5, 7),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(16, 18),   /*0b110000*/
    p(25, 12),   /*0b110001*/
    p(33, 9),    /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(11, 29),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(27, 15),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(7, -1),    /*0b111111*/
    p(-13, -4),  /*0b00*/
    p(11, -18),  /*0b01*/
    p(39, -9),   /*0b10*/
    p(22, -41),  /*0b11*/
    p(47, -11),  /*0b100*/
    p(6, -22),   /*0b101*/
    p(70, -41),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(65, -13),  /*0b1000*/
    p(21, -34),  /*0b1001*/
    p(81, -56),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(61, -20),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(22, -11),  /*0b1111*/
    p(21, -3),   /*0b00*/
    p(33, -13),  /*0b01*/
    p(27, -18),  /*0b10*/
    p(22, -42),  /*0b11*/
    p(37, -10),  /*0b100*/
    p(56, -21),  /*0b101*/
    p(25, -24),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(39, -4),   /*0b1000*/
    p(53, -18),  /*0b1001*/
    p(51, -43),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(44, -23),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(21, -44),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  33,   86),    p(  30,   85),    p(  21,   88),    p(  33,   69),    p(  19,   73),    p(  19,   76),    p( -19,   94),    p( -11,   93),
        p(  39,  123),    p(  47,  123),    p(  37,  100),    p(  20,   69),    p(  34,   69),    p(  15,   95),    p(  -1,  104),    p( -32,  125),
        p(  23,   74),    p(  17,   71),    p(  22,   54),    p(  17,   43),    p(  -0,   46),    p(   7,   58),    p( -10,   76),    p( -10,   79),
        p(   7,   46),    p(  -2,   44),    p( -15,   34),    p( -10,   24),    p( -17,   28),    p( -10,   39),    p( -18,   55),    p( -11,   51),
        p(   1,   14),    p( -12,   23),    p( -15,   17),    p( -16,    8),    p( -15,   13),    p(  -7,   17),    p( -14,   37),    p(  10,   17),
        p(  -5,   15),    p(  -2,   20),    p(  -9,   16),    p(  -8,    4),    p(   5,    1),    p(   7,    7),    p(  13,   18),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-10, -10);
const DOUBLED_PAWN: PhasedScore = p(-7, -21);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = [
    p(13, 11),
    p(8, 13),
    p(14, 19),
    p(9, 7),
    p(-3, 16),
    p(-46, 6),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(38, 9), p(39, 35), p(51, -8), p(35, -34), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-49, -71),
        p(-28, -31),
        p(-15, -8),
        p(-5, 5),
        p(3, 16),
        p(10, 27),
        p(19, 30),
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
        p(-8, -23),
        p(-0, -10),
        p(7, -0),
        p(12, 8),
        p(16, 13),
        p(20, 18),
        p(22, 23),
        p(29, 25),
        p(35, 24),
        p(43, 27),
        p(40, 33),
        p(55, 28),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-75, 11),
        p(-66, 25),
        p(-62, 31),
        p(-59, 35),
        p(-59, 42),
        p(-53, 46),
        p(-50, 51),
        p(-46, 53),
        p(-42, 57),
        p(-38, 61),
        p(-35, 63),
        p(-33, 68),
        p(-26, 68),
        p(-17, 66),
        p(-15, 66),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-29, -47),
        p(-29, 8),
        p(-33, 57),
        p(-28, 73),
        p(-25, 91),
        p(-20, 96),
        p(-17, 106),
        p(-13, 112),
        p(-9, 116),
        p(-6, 117),
        p(-3, 120),
        p(1, 122),
        p(3, 122),
        p(5, 127),
        p(7, 128),
        p(11, 131),
        p(11, 138),
        p(14, 139),
        p(23, 137),
        p(36, 130),
        p(40, 132),
        p(84, 109),
        p(83, 112),
        p(106, 94),
        p(201, 59),
        p(245, 20),
        p(276, 8),
        p(329, -27),
    ],
    [
        p(-95, 7),
        p(-59, -5),
        p(-30, -6),
        p(1, -4),
        p(32, -2),
        p(56, -3),
        p(84, 3),
        p(110, 2),
        p(159, -15),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(20, -33),
        p(0, 0),
    ],
    [p(-3, 11), p(20, 23), p(0, 0), p(31, 5), p(31, 53), p(0, 0)],
    [p(-3, 13), p(11, 15), p(18, 12), p(0, 0), p(45, -5), p(0, 0)],
    [p(-2, 5), p(2, 5), p(-0, 21), p(1, 1), p(0, 0), p(0, 0)],
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
        p(-127, -28),
        p(8, 15),
        p(9, 20),
        p(4, 7),
    ],
    [p(2, 2), p(13, 6), p(9, 11), p(11, 8), p(11, 21), p(21, -6)],
    [
        p(2, -2),
        p(9, 1),
        p(7, -5),
        p(4, 15),
        p(-61, -252),
        p(5, -11),
    ],
    [p(64, -2), p(41, 6), p(46, 0), p(25, 5), p(37, -12), p(0, 0)],
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
