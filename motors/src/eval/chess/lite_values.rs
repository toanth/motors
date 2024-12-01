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
        p( 130,  187),    p( 127,  185),    p( 117,  189),    p( 129,  169),    p( 116,  173),    p( 116,  177),    p(  77,  195),    p(  87,  193),
        p(  62,  123),    p(  60,  124),    p(  71,  120),    p(  80,  124),    p(  66,  124),    p( 116,  111),    p(  87,  132),    p(  87,  122),
        p(  49,  113),    p(  61,  109),    p(  59,  104),    p(  61,   98),    p(  78,   99),    p(  81,   94),    p(  75,  103),    p(  69,   95),
        p(  45,   99),    p(  53,  102),    p(  62,   95),    p(  71,   93),    p(  74,   93),    p(  76,   88),    p(  68,   93),    p(  57,   86),
        p(  41,   97),    p(  51,   93),    p(  54,   94),    p(  57,  100),    p(  66,   96),    p(  61,   92),    p(  69,   83),    p(  52,   85),
        p(  47,   99),    p(  50,   96),    p(  56,   98),    p(  55,  106),    p(  53,  108),    p(  72,   98),    p(  72,   84),    p(  53,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 180,  276),    p( 198,  309),    p( 233,  319),    p( 256,  310),    p( 291,  310),    p( 198,  306),    p( 224,  306),    p( 208,  258),
        p( 270,  309),    p( 284,  315),    p( 297,  309),    p( 310,  311),    p( 305,  307),    p( 319,  296),    p( 277,  312),    p( 279,  301),
        p( 287,  305),    p( 302,  306),    p( 322,  312),    p( 321,  317),    p( 338,  310),    p( 357,  300),    p( 312,  302),    p( 301,  304),
        p( 298,  314),    p( 308,  310),    p( 320,  317),    p( 345,  320),    p( 324,  321),    p( 338,  317),    p( 311,  314),    p( 327,  309),
        p( 295,  317),    p( 299,  307),    p( 308,  316),    p( 316,  320),    p( 322,  322),    p( 319,  307),    p( 328,  303),    p( 311,  313),
        p( 272,  303),    p( 276,  303),    p( 289,  299),    p( 294,  314),    p( 301,  311),    p( 288,  293),    p( 297,  294),    p( 290,  307),
        p( 267,  310),    p( 278,  313),    p( 279,  304),    p( 290,  309),    p( 293,  304),    p( 285,  300),    p( 290,  305),    p( 285,  320),
        p( 238,  307),    p( 278,  304),    p( 263,  304),    p( 283,  310),    p( 293,  308),    p( 289,  297),    p( 285,  305),    p( 260,  305),
    ],
    // bishop
    [
        p( 280,  315),    p( 255,  314),    p( 243,  308),    p( 219,  316),    p( 220,  315),    p( 219,  307),    p( 280,  305),    p( 252,  309),
        p( 282,  303),    p( 281,  305),    p( 286,  307),    p( 278,  308),    p( 278,  304),    p( 289,  302),    p( 265,  308),    p( 272,  305),
        p( 295,  311),    p( 303,  305),    p( 293,  310),    p( 299,  302),    p( 302,  305),    p( 327,  308),    p( 316,  304),    p( 308,  313),
        p( 281,  311),    p( 292,  310),    p( 297,  305),    p( 310,  311),    p( 302,  307),    p( 301,  309),    p( 293,  310),    p( 282,  312),
        p( 290,  309),    p( 278,  310),    p( 298,  307),    p( 307,  306),    p( 306,  305),    p( 297,  304),    p( 289,  308),    p( 306,  303),
        p( 291,  309),    p( 302,  308),    p( 299,  307),    p( 302,  307),    p( 307,  308),    p( 303,  303),    p( 306,  299),    p( 309,  302),
        p( 309,  313),    p( 302,  300),    p( 311,  303),    p( 297,  309),    p( 303,  308),    p( 304,  305),    p( 312,  300),    p( 303,  299),
        p( 296,  305),    p( 315,  309),    p( 307,  307),    p( 291,  313),    p( 305,  310),    p( 296,  315),    p( 305,  299),    p( 302,  297),
    ],
    // rook
    [
        p( 466,  547),    p( 455,  556),    p( 451,  562),    p( 449,  560),    p( 460,  555),    p( 478,  550),    p( 489,  548),    p( 502,  540),
        p( 441,  555),    p( 440,  559),    p( 451,  560),    p( 466,  550),    p( 456,  552),    p( 476,  545),    p( 486,  542),    p( 498,  534),
        p( 443,  549),    p( 463,  543),    p( 459,  544),    p( 461,  539),    p( 489,  529),    p( 495,  527),    p( 520,  524),    p( 490,  527),
        p( 440,  548),    p( 447,  544),    p( 448,  546),    p( 454,  540),    p( 461,  532),    p( 469,  529),    p( 475,  531),    p( 473,  527),
        p( 433,  546),    p( 433,  544),    p( 435,  544),    p( 441,  541),    p( 448,  537),    p( 441,  537),    p( 460,  530),    p( 450,  529),
        p( 429,  543),    p( 429,  540),    p( 432,  539),    p( 436,  539),    p( 444,  532),    p( 451,  525),    p( 473,  512),    p( 454,  519),
        p( 430,  538),    p( 435,  537),    p( 441,  538),    p( 445,  535),    p( 453,  528),    p( 466,  518),    p( 474,  513),    p( 441,  524),
        p( 440,  540),    p( 437,  536),    p( 439,  541),    p( 444,  536),    p( 451,  530),    p( 457,  529),    p( 454,  526),    p( 448,  528),
    ],
    // queen
    [
        p( 882,  962),    p( 884,  977),    p( 898,  989),    p( 916,  985),    p( 913,  988),    p( 933,  975),    p( 983,  925),    p( 929,  956),
        p( 890,  955),    p( 864,  985),    p( 866, 1011),    p( 858, 1029),    p( 865, 1040),    p( 906,  999),    p( 908,  982),    p( 951,  962),
        p( 895,  961),    p( 887,  978),    p( 886, 1001),    p( 886, 1009),    p( 909, 1010),    p( 947,  994),    p( 955,  964),    p( 942,  974),
        p( 880,  976),    p( 885,  985),    p( 880,  993),    p( 879, 1007),    p( 884, 1017),    p( 896, 1007),    p( 905, 1009),    p( 913,  985),
        p( 890,  967),    p( 876,  985),    p( 883,  986),    p( 883, 1004),    p( 885, 1002),    p( 889, 1000),    p( 902,  987),    p( 909,  982),
        p( 884,  953),    p( 891,  967),    p( 885,  983),    p( 882,  987),    p( 889,  992),    p( 896,  983),    p( 910,  966),    p( 908,  956),
        p( 885,  953),    p( 884,  961),    p( 891,  962),    p( 891,  975),    p( 893,  975),    p( 895,  956),    p( 904,  936),    p( 912,  912),
        p( 873,  950),    p( 883,  940),    p( 883,  954),    p( 892,  957),    p( 895,  950),    p( 883,  948),    p( 883,  939),    p( 888,  925),
    ],
    // king
    [
        p( 159,  -85),    p(  56,  -38),    p(  82,  -31),    p(   6,    1),    p(  29,  -11),    p(  10,   -1),    p(  66,  -10),    p( 226,  -88),
        p( -27,    0),    p( -83,   19),    p( -86,   26),    p( -26,   16),    p( -61,   25),    p( -84,   38),    p( -55,   24),    p(   1,    1),
        p( -48,    8),    p( -51,   13),    p( -90,   28),    p(-100,   37),    p( -67,   31),    p( -37,   23),    p( -79,   24),    p( -42,   11),
        p( -29,    1),    p(-105,   13),    p(-118,   29),    p(-140,   38),    p(-141,   35),    p(-118,   28),    p(-137,   17),    p(-106,   16),
        p( -43,   -3),    p(-122,    8),    p(-132,   25),    p(-157,   39),    p(-160,   37),    p(-135,   22),    p(-148,   13),    p(-119,   13),
        p( -35,    2),    p( -96,    3),    p(-125,   19),    p(-133,   27),    p(-129,   26),    p(-142,   19),    p(-114,    4),    p( -75,   11),
        p(  27,   -7),    p( -81,   -2),    p( -93,    7),    p(-113,   16),    p(-119,   17),    p(-104,    7),    p( -73,   -9),    p(   3,   -3),
        p(  53,  -25),    p(  44,  -37),    p(  39,  -24),    p( -21,   -4),    p(  32,  -21),    p( -18,   -8),    p(  37,  -32),    p(  67,  -35),
    ],
];

const BISHOP_PAIR: PhasedScore = p(22, 57);
const ROOK_OPEN_FILE: PhasedScore = p(16, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -2);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 2);
const KING_OPEN_FILE: PhasedScore = p(-56, -1);
const KING_CLOSED_FILE: PhasedScore = p(16, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 6), p(-1, 8), p(-1, 9), p(3, 7), p(2, 9), p(4, 11), p(7, 10), p(17, 6)],
    // Closed
    [p(0, 0), p(0, 0), p(15, -25), p(-16, 10), p(-1, 12), p(1, 4), p(0, 8), p(-1, 4)],
    // SemiOpen
    [p(0, 0), p(-18, 25), p(3, 20), p(1, 13), p(-1, 14), p(4, 9), p(0, 7), p(9, 8)],
    // SemiClosed
    [p(0, 0), p(10, -12), p(6, 7), p(3, 2), p(6, 4), p(2, 4), p(5, 7), p(1, 5)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-7, 6),    /*0b0000*/
    p(-14, 8),   /*0b0001*/
    p(-2, 7),    /*0b0010*/
    p(-10, 12),  /*0b0011*/
    p(-4, 4),    /*0b0100*/
    p(-25, -0),  /*0b0101*/
    p(-15, 5),   /*0b0110*/
    p(-20, -17), /*0b0111*/
    p(6, 10),    /*0b1000*/
    p(-5, 9),    /*0b1001*/
    p(1, 10),    /*0b1010*/
    p(-5, 9),    /*0b1011*/
    p(-2, 4),    /*0b1100*/
    p(-25, 8),   /*0b1101*/
    p(-13, 3),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(2, 17),    /*0b10000*/
    p(4, 11),    /*0b10001*/
    p(22, 12),   /*0b10010*/
    p(-5, 6),    /*0b10011*/
    p(-6, 7),    /*0b10100*/
    p(12, 16),   /*0b10101*/
    p(-24, 1),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(12, 31),   /*0b11000*/
    p(29, 24),   /*0b11001*/
    p(41, 39),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(15, 11),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(14, 10),   /*0b100000*/
    p(5, 13),    /*0b100001*/
    p(25, 3),    /*0b100010*/
    p(6, -2),    /*0b100011*/
    p(-9, 1),    /*0b100100*/
    p(-22, -9),  /*0b100101*/
    p(-26, 14),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(18, 3),    /*0b101000*/
    p(-1, 17),   /*0b101001*/
    p(19, -3),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-8, 6),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(15, 19),   /*0b110000*/
    p(26, 15),   /*0b110001*/
    p(32, 11),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(7, 31),    /*0b110100*/
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
    p(4, -3),    /*0b111111*/
    p(-18, -3),  /*0b00*/
    p(11, -18),  /*0b01*/
    p(39, -9),   /*0b10*/
    p(25, -42),  /*0b11*/
    p(49, -11),  /*0b100*/
    p(-1, -19),  /*0b101*/
    p(75, -41),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(59, -13),  /*0b1000*/
    p(20, -35),  /*0b1001*/
    p(82, -57),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(60, -20),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(21, -4),   /*0b1111*/
    p(20, -3),   /*0b00*/
    p(36, -14),  /*0b01*/
    p(28, -18),  /*0b10*/
    p(26, -43),  /*0b11*/
    p(35, -11),  /*0b100*/
    p(56, -21),  /*0b101*/
    p(25, -25),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(41, -4),   /*0b1000*/
    p(57, -18),  /*0b1001*/
    p(54, -44),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(45, -23),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(22, -44),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  30,   87),    p(  27,   85),    p(  17,   89),    p(  29,   69),    p(  16,   73),    p(  16,   77),    p( -23,   95),    p( -13,   93),
        p(  40,  123),    p(  46,  123),    p(  36,   99),    p(  20,   68),    p(  35,   69),    p(  15,   95),    p(   1,  104),    p( -30,  125),
        p(  23,   74),    p(  16,   71),    p(  22,   54),    p(  16,   43),    p(  -1,   45),    p(   6,   59),    p( -10,   76),    p( -10,   79),
        p(   8,   47),    p(  -3,   44),    p( -15,   35),    p(  -9,   25),    p( -16,   29),    p( -11,   40),    p( -18,   55),    p( -11,   51),
        p(   2,   15),    p( -13,   24),    p( -15,   17),    p( -16,    9),    p( -14,   14),    p(  -8,   18),    p( -13,   37),    p(   9,   17),
        p(  -4,   15),    p(  -2,   20),    p(  -8,   16),    p(  -8,    4),    p(   6,    1),    p(   7,    7),    p(  13,   19),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-11, -10);
const DOUBLED_PAWN: PhasedScore = p(-6, -21);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = [
    p(10, 11),
    p(6, 14),
    p(11, 19),
    p(8, 9),
    p(-6, 17),
    p(-51, 7),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(35, 8), p(37, 36), p(49, -8), p(33, -34), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-51, -66),
        p(-30, -26),
        p(-17, -3),
        p(-7, 10),
        p(2, 20),
        p(10, 31),
        p(19, 32),
        p(27, 33),
        p(35, 29),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-30, -54),
        p(-18, -36),
        p(-7, -20),
        p(-0, -6),
        p(6, 4),
        p(11, 13),
        p(16, 18),
        p(20, 22),
        p(22, 27),
        p(29, 28),
        p(34, 27),
        p(43, 28),
        p(39, 36),
        p(53, 28),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-75, 12),
        p(-66, 25),
        p(-61, 30),
        p(-58, 34),
        p(-58, 42),
        p(-53, 47),
        p(-50, 52),
        p(-45, 55),
        p(-41, 59),
        p(-37, 63),
        p(-32, 65),
        p(-29, 69),
        p(-20, 69),
        p(-8, 65),
        p(-6, 65),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-31, -45),
        p(-31, 9),
        p(-35, 59),
        p(-30, 77),
        p(-27, 95),
        p(-22, 101),
        p(-18, 112),
        p(-15, 119),
        p(-10, 124),
        p(-7, 127),
        p(-4, 131),
        p(-0, 134),
        p(2, 136),
        p(4, 141),
        p(7, 142),
        p(10, 145),
        p(11, 152),
        p(14, 151),
        p(23, 148),
        p(38, 139),
        p(43, 139),
        p(89, 112),
        p(88, 115),
        p(114, 93),
        p(207, 57),
        p(260, 11),
        p(284, 2),
        p(351, -45),
    ],
    [
        p(-94, 16),
        p(-58, -3),
        p(-29, -5),
        p(0, -4),
        p(32, -3),
        p(55, -4),
        p(83, 3),
        p(109, 1),
        p(159, -17),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-8, 8),
        p(0, 0),
        p(23, 20),
        p(48, -12),
        p(20, -35),
        p(0, 0),
    ],
    [p(-1, 12), p(21, 23), p(0, 0), p(32, 3), p(31, 53), p(0, 0)],
    [p(-5, 14), p(9, 16), p(16, 13), p(0, 0), p(42, -3), p(0, 0)],
    [p(-2, 4), p(3, 7), p(-0, 21), p(2, 2), p(0, 0), p(0, 0)],
    [
        p(72, 28),
        p(-36, 14),
        p(-3, 22),
        p(-27, 8),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 7), p(9, 5), p(6, 10), p(13, 6), p(7, 14), p(11, 5)],
    [
        p(-1, 2),
        p(10, 19),
        p(-100, -31),
        p(7, 13),
        p(8, 16),
        p(4, 7),
    ],
    [p(0, 3), p(11, 6), p(7, 11), p(8, 8), p(8, 16), p(18, -3)],
    [
        p(2, -3),
        p(9, -2),
        p(7, -8),
        p(3, 13),
        p(-66, -255),
        p(4, -9),
    ],
    [p(63, -0), p(41, 6), p(46, 1), p(25, 5), p(37, -11), p(0, 0)],
];

pub const NUM_DEFENDED: [PhasedScore; 17] = [
    p(28, 1),
    p(-18, 2),
    p(-25, 1),
    p(-21, -1),
    p(-14, -4),
    p(-13, -4),
    p(-14, -2),
    p(-9, -1),
    p(-5, 1),
    p(-1, 4),
    p(2, 11),
    p(7, 14),
    p(11, 17),
    p(13, 23),
    p(15, 11),
    p(13, -10),
    p(10, 245),
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-18, -17),
    p(16, -9),
    p(19, -2),
    p(27, -12),
    p(6, 21),
    p(-9, 12),
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

    fn king_zone_attack(attacking: ChessPieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }
}
