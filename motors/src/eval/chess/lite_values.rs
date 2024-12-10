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
        p( 132,  187),    p( 129,  186),    p( 120,  189),    p( 131,  169),    p( 118,  174),    p( 118,  177),    p(  80,  195),    p(  87,  193),
        p(  65,  124),    p(  64,  125),    p(  75,  121),    p(  83,  124),    p(  71,  124),    p( 119,  111),    p(  94,  131),    p(  91,  122),
        p(  52,  113),    p(  63,  109),    p(  62,  105),    p(  64,   99),    p(  80,   99),    p(  84,   95),    p(  76,  104),    p(  72,   96),
        p(  48,  100),    p(  55,  103),    p(  64,   95),    p(  73,   94),    p(  77,   94),    p(  76,   89),    p(  70,   93),    p(  59,   87),
        p(  44,   98),    p(  52,   93),    p(  56,   95),    p(  59,  100),    p(  67,   97),    p(  61,   93),    p(  70,   83),    p(  53,   86),
        p(  49,  100),    p(  53,   96),    p(  59,   99),    p(  58,  106),    p(  55,  108),    p(  71,   99),    p(  73,   84),    p(  55,   89),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 178,  277),    p( 198,  309),    p( 215,  320),    p( 253,  309),    p( 283,  311),    p( 199,  306),    p( 214,  307),    p( 205,  260),
        p( 269,  310),    p( 284,  315),    p( 298,  307),    p( 303,  310),    p( 303,  306),    p( 315,  295),    p( 276,  312),    p( 272,  302),
        p( 286,  306),    p( 303,  303),    p( 304,  310),    p( 319,  313),    p( 336,  306),    p( 348,  296),    p( 290,  303),    p( 285,  307),
        p( 300,  314),    p( 307,  309),    p( 321,  313),    p( 324,  320),    p( 322,  317),    p( 317,  316),    p( 308,  312),    p( 318,  310),
        p( 297,  317),    p( 301,  306),    p( 309,  312),    p( 318,  315),    p( 316,  319),    p( 321,  303),    p( 319,  303),    p( 312,  312),
        p( 273,  304),    p( 280,  302),    p( 292,  296),    p( 298,  310),    p( 303,  307),    p( 292,  289),    p( 299,  293),    p( 291,  307),
        p( 268,  312),    p( 280,  313),    p( 281,  303),    p( 292,  307),    p( 296,  302),    p( 286,  300),    p( 293,  305),    p( 288,  321),
        p( 238,  310),    p( 279,  304),    p( 264,  305),    p( 285,  311),    p( 293,  308),    p( 291,  297),    p( 286,  307),    p( 264,  308),
    ],
    // bishop
    [
        p( 278,  309),    p( 255,  313),    p( 241,  305),    p( 223,  315),    p( 219,  312),    p( 225,  307),    p( 276,  302),    p( 253,  308),
        p( 284,  302),    p( 279,  303),    p( 291,  305),    p( 277,  303),    p( 288,  300),    p( 292,  298),    p( 268,  308),    p( 271,  301),
        p( 296,  309),    p( 306,  304),    p( 291,  304),    p( 306,  299),    p( 305,  300),    p( 335,  304),    p( 316,  300),    p( 317,  312),
        p( 286,  312),    p( 291,  306),    p( 303,  302),    p( 307,  307),    p( 307,  304),    p( 298,  305),    p( 296,  309),    p( 279,  309),
        p( 289,  307),    p( 283,  309),    p( 295,  304),    p( 309,  305),    p( 302,  301),    p( 298,  303),    p( 286,  304),    p( 308,  302),
        p( 295,  311),    p( 299,  304),    p( 300,  307),    p( 300,  304),    p( 306,  307),    p( 300,  298),    p( 305,  295),    p( 307,  299),
        p( 307,  310),    p( 303,  301),    p( 309,  300),    p( 298,  309),    p( 301,  305),    p( 304,  303),    p( 311,  296),    p( 308,  297),
        p( 297,  305),    p( 310,  306),    p( 307,  307),    p( 290,  309),    p( 305,  308),    p( 293,  310),    p( 306,  296),    p( 301,  293),
    ],
    // rook
    [
        p( 459,  547),    p( 448,  556),    p( 442,  563),    p( 440,  560),    p( 451,  556),    p( 472,  550),    p( 483,  548),    p( 493,  541),
        p( 443,  553),    p( 441,  558),    p( 450,  559),    p( 466,  549),    p( 451,  552),    p( 468,  547),    p( 476,  544),    p( 491,  534),
        p( 444,  548),    p( 462,  543),    p( 456,  545),    p( 456,  540),    p( 483,  530),    p( 492,  527),    p( 509,  526),    p( 484,  528),
        p( 441,  548),    p( 446,  544),    p( 445,  546),    p( 451,  540),    p( 456,  532),    p( 467,  528),    p( 466,  532),    p( 466,  527),
        p( 434,  546),    p( 433,  544),    p( 434,  544),    p( 439,  540),    p( 445,  537),    p( 440,  536),    p( 453,  530),    p( 447,  528),
        p( 429,  543),    p( 429,  540),    p( 432,  539),    p( 435,  538),    p( 441,  532),    p( 452,  524),    p( 467,  513),    p( 454,  517),
        p( 431,  538),    p( 435,  537),    p( 441,  538),    p( 443,  535),    p( 451,  528),    p( 463,  518),    p( 470,  514),    p( 441,  523),
        p( 441,  542),    p( 438,  537),    p( 439,  541),    p( 444,  535),    p( 449,  528),    p( 456,  528),    p( 452,  527),    p( 448,  529),
    ],
    // queen
    [
        p( 882,  955),    p( 886,  969),    p( 900,  982),    p( 921,  976),    p( 917,  980),    p( 938,  967),    p( 984,  919),    p( 928,  951),
        p( 891,  947),    p( 865,  979),    p( 868, 1005),    p( 859, 1022),    p( 867, 1033),    p( 908,  993),    p( 908,  978),    p( 950,  956),
        p( 895,  954),    p( 887,  971),    p( 886,  992),    p( 887, 1001),    p( 911, 1002),    p( 948,  987),    p( 956,  957),    p( 945,  963),
        p( 881,  966),    p( 886,  975),    p( 881,  985),    p( 881,  998),    p( 885, 1010),    p( 898,  999),    p( 907, 1000),    p( 915,  975),
        p( 891,  959),    p( 878,  978),    p( 885,  979),    p( 885,  997),    p( 888,  993),    p( 891,  992),    p( 904,  980),    p( 911,  973),
        p( 886,  948),    p( 893,  963),    p( 888,  978),    p( 886,  981),    p( 892,  986),    p( 899,  976),    p( 912,  960),    p( 910,  948),
        p( 886,  950),    p( 886,  958),    p( 894,  959),    p( 893,  974),    p( 895,  973),    p( 896,  955),    p( 907,  934),    p( 915,  907),
        p( 873,  950),    p( 884,  938),    p( 885,  951),    p( 893,  952),    p( 897,  942),    p( 884,  946),    p( 886,  937),    p( 891,  919),
    ],
    // king
    [
        p( 158,  -85),    p(  59,  -38),    p(  83,  -31),    p(   7,    1),    p(  36,  -12),    p(  19,   -2),    p(  73,  -10),    p( 235,  -89),
        p( -30,    1),    p( -79,   19),    p( -80,   26),    p( -22,   17),    p( -52,   24),    p( -81,   39),    p( -49,   24),    p(   8,   -0),
        p( -45,    9),    p( -47,   14),    p( -84,   28),    p( -94,   36),    p( -63,   31),    p( -30,   23),    p( -77,   26),    p( -35,   10),
        p( -25,    1),    p(-100,   13),    p(-113,   29),    p(-135,   37),    p(-135,   35),    p(-113,   27),    p(-132,   17),    p(-104,   17),
        p( -41,   -2),    p(-114,    8),    p(-125,   25),    p(-150,   38),    p(-152,   36),    p(-127,   22),    p(-144,   13),    p(-116,   13),
        p( -33,    2),    p( -92,    4),    p(-119,   18),    p(-125,   27),    p(-123,   26),    p(-133,   18),    p(-109,    4),    p( -72,   10),
        p(  25,   -8),    p( -78,   -2),    p( -90,    7),    p(-109,   16),    p(-115,   17),    p(-100,    8),    p( -73,   -9),    p(   4,   -4),
        p(  53,  -24),    p(  41,  -36),    p(  38,  -23),    p( -22,   -3),    p(  30,  -19),    p( -18,   -6),    p(  34,  -30),    p(  65,  -33),
    ],
];

const BISHOP_PAIR: PhasedScore = p(22, 53);
const BAD_BISHOP: [PhasedScore; 9] = [
    p(8, 19),
    p(9, 18),
    p(9, 7),
    p(6, -1),
    p(2, -9),
    p(-1, -20),
    p(-8, -30),
    p(-15, -46),
    p(-26, -61),
];
const ROOK_OPEN_FILE: PhasedScore = p(13, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 4);
const KING_OPEN_FILE: PhasedScore = p(-49, -1);
const KING_CLOSED_FILE: PhasedScore = p(14, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 3), p(-1, 5), p(-1, 4), p(2, 3), p(2, 5), p(2, 7), p(5, 5), p(17, 1)],
    // Closed
    [p(0, 0), p(0, 0), p(15, -22), p(-16, 9), p(-1, 11), p(1, 5), p(-1, 7), p(-2, 5)],
    // SemiOpen
    [p(0, 0), p(-17, 22), p(3, 16), p(0, 9), p(-0, 9), p(3, 5), p(-1, 2), p(9, 5)],
    // SemiClosed
    [p(0, 0), p(9, -12), p(6, 6), p(2, 1), p(6, 3), p(2, 5), p(4, 6), p(-0, 5)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-4, 5),    /*0b0000*/
    p(-14, 8),   /*0b0001*/
    p(-2, 8),    /*0b0010*/
    p(-11, 13),  /*0b0011*/
    p(-3, 3),    /*0b0100*/
    p(-26, -1),  /*0b0101*/
    p(-15, 5),   /*0b0110*/
    p(-22, -16), /*0b0111*/
    p(10, 11),   /*0b1000*/
    p(-2, 11),   /*0b1001*/
    p(3, 11),    /*0b1010*/
    p(-5, 11),   /*0b1011*/
    p(0, 5),     /*0b1100*/
    p(-23, 10),  /*0b1101*/
    p(-12, 3),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(4, 15),    /*0b10000*/
    p(2, 8),     /*0b10001*/
    p(21, 11),   /*0b10010*/
    p(-8, 6),    /*0b10011*/
    p(-5, 6),    /*0b10100*/
    p(12, 15),   /*0b10101*/
    p(-26, 1),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(16, 29),   /*0b11000*/
    p(28, 24),   /*0b11001*/
    p(43, 38),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(17, 10),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(17, 10),   /*0b100000*/
    p(4, 13),    /*0b100001*/
    p(25, 3),    /*0b100010*/
    p(5, -2),    /*0b100011*/
    p(-6, 2),    /*0b100100*/
    p(-21, -7),  /*0b100101*/
    p(-23, 15),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(24, 4),    /*0b101000*/
    p(2, 17),    /*0b101001*/
    p(22, -2),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-4, 8),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(17, 17),   /*0b110000*/
    p(25, 13),   /*0b110001*/
    p(33, 10),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(10, 29),   /*0b110100*/
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
    p(6, -1),    /*0b111111*/
    p(-13, -4),  /*0b00*/
    p(11, -17),  /*0b01*/
    p(39, -8),   /*0b10*/
    p(20, -40),  /*0b11*/
    p(48, -11),  /*0b100*/
    p(6, -20),   /*0b101*/
    p(69, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(65, -13),  /*0b1000*/
    p(20, -33),  /*0b1001*/
    p(82, -55),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(63, -22),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(20, -10),  /*0b1111*/
    p(21, -4),   /*0b00*/
    p(33, -13),  /*0b01*/
    p(26, -17),  /*0b10*/
    p(20, -41),  /*0b11*/
    p(38, -11),  /*0b100*/
    p(55, -20),  /*0b101*/
    p(25, -23),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(40, -5),   /*0b1000*/
    p(52, -17),  /*0b1001*/
    p(51, -42),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(45, -24),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(18, -43),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  32,   87),    p(  29,   86),    p(  20,   89),    p(  31,   69),    p(  18,   74),    p(  18,   77),    p( -20,   95),    p( -13,   93),
        p(  40,  124),    p(  47,  123),    p(  36,  100),    p(  20,   69),    p(  34,   69),    p(  15,   96),    p(  -1,  105),    p( -32,  126),
        p(  23,   74),    p(  17,   71),    p(  22,   54),    p(  16,   43),    p(  -1,   46),    p(   7,   59),    p(  -9,   76),    p( -10,   79),
        p(   8,   46),    p(  -2,   44),    p( -15,   34),    p( -10,   25),    p( -17,   29),    p(  -9,   40),    p( -17,   55),    p( -11,   51),
        p(   1,   15),    p( -12,   24),    p( -15,   17),    p( -16,    8),    p( -15,   14),    p(  -6,   18),    p( -14,   38),    p(  10,   17),
        p(  -5,   15),    p(  -2,   20),    p(  -8,   16),    p(  -8,    5),    p(   5,    1),    p(   7,    8),    p(  13,   19),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-10, -10);
const DOUBLED_PAWN: PhasedScore = p(-6, -21);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = [
    p(11, 12),
    p(6, 13),
    p(12, 19),
    p(7, 9),
    p(-6, 17),
    p(-50, 8),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(36, 9), p(37, 35), p(49, -7), p(33, -33), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-50, -71),
        p(-30, -31),
        p(-17, -7),
        p(-7, 6),
        p(1, 17),
        p(9, 28),
        p(17, 31),
        p(25, 33),
        p(32, 31),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-31, -56),
        p(-19, -38),
        p(-9, -22),
        p(-1, -9),
        p(6, 1),
        p(11, 9),
        p(16, 14),
        p(20, 18),
        p(22, 23),
        p(30, 25),
        p(35, 24),
        p(44, 25),
        p(41, 32),
        p(55, 26),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-76, 12),
        p(-67, 25),
        p(-62, 31),
        p(-60, 35),
        p(-60, 42),
        p(-54, 47),
        p(-51, 52),
        p(-47, 55),
        p(-43, 59),
        p(-40, 62),
        p(-36, 65),
        p(-35, 69),
        p(-27, 69),
        p(-18, 66),
        p(-16, 65),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-27, -47),
        p(-27, 8),
        p(-30, 56),
        p(-26, 73),
        p(-23, 91),
        p(-19, 96),
        p(-15, 107),
        p(-12, 113),
        p(-8, 118),
        p(-5, 119),
        p(-2, 122),
        p(2, 124),
        p(5, 124),
        p(7, 129),
        p(9, 130),
        p(13, 133),
        p(14, 139),
        p(17, 139),
        p(26, 136),
        p(40, 129),
        p(44, 129),
        p(89, 102),
        p(89, 106),
        p(114, 85),
        p(209, 49),
        p(254, 7),
        p(279, -2),
        p(345, -50),
    ],
    [
        p(-92, 5),
        p(-58, -7),
        p(-29, -6),
        p(1, -3),
        p(32, -1),
        p(56, -3),
        p(84, 4),
        p(110, 2),
        p(157, -16),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-8, 7),
        p(0, 0),
        p(23, 19),
        p(49, -12),
        p(20, -34),
        p(0, 0),
    ],
    [p(-2, 11), p(20, 23), p(0, 0), p(32, 4), p(31, 52), p(0, 0)],
    [p(-2, 12), p(11, 15), p(18, 12), p(0, 0), p(44, -5), p(0, 0)],
    [p(-1, 4), p(3, 5), p(0, 21), p(2, 0), p(0, 0), p(0, 0)],
    [
        p(70, 28),
        p(-35, 17),
        p(-8, 17),
        p(-21, 7),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 7), p(8, 6), p(5, 10), p(12, 6), p(6, 17), p(10, 6)],
    [
        p(0, 6),
        p(10, 21),
        p(-124, -26),
        p(7, 14),
        p(8, 18),
        p(2, 8),
    ],
    [p(1, 2), p(12, 6), p(8, 12), p(9, 8), p(8, 19), p(18, -4)],
    [
        p(1, -2),
        p(8, 0),
        p(6, -6),
        p(3, 13),
        p(-67, -252),
        p(2, -10),
    ],
    [p(62, -0), p(39, 7), p(45, 1), p(24, 5), p(37, -12), p(0, 0)],
];

pub const NUM_DEFENDED: [PhasedScore; 17] = [
    p(37, 2),
    p(-10, 2),
    p(-18, -0),
    p(-15, -2),
    p(-10, -5),
    p(-11, -4),
    p(-13, -2),
    p(-10, -1),
    p(-6, 2),
    p(-3, 5),
    p(1, 12),
    p(5, 15),
    p(10, 18),
    p(12, 23),
    p(14, 10),
    p(12, -10),
    p(10, 249),
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-21, -18),
    p(19, -10),
    p(11, -4),
    p(14, -12),
    p(-2, 13),
    p(-14, 11),
];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(14, 19), p(34, -0), p(6, 32)];

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

    fn num_defended(num: usize) -> SingleFeatureScore<Self::Score>;

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

    fn num_defended(num: usize) -> PhasedScore {
        NUM_DEFENDED[num]
    }

    fn king_zone_attack(attacking: ChessPieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }

    fn can_give_check(piece: ChessPieceType) -> PhasedScore {
        CAN_GIVE_CHECK[piece as usize]
    }
}
