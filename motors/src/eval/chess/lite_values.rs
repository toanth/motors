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
        p( 133,  186),    p( 131,  185),    p( 121,  187),    p( 134,  167),    p( 118,  172),    p( 115,  176),    p(  77,  195),    p(  78,  194),
        p(  65,  123),    p(  66,  124),    p(  78,  119),    p(  83,  125),    p(  77,  122),    p( 118,  110),    p( 113,  128),    p(  86,  122),
        p(  51,  114),    p(  64,  109),    p(  62,  103),    p(  67,   97),    p(  83,   99),    p(  85,   94),    p(  79,  104),    p(  72,   96),
        p(  48,  100),    p(  55,  102),    p(  64,   95),    p(  73,   94),    p(  77,   93),    p(  78,   89),    p(  71,   93),    p(  60,   86),
        p(  42,   98),    p(  51,   95),    p(  54,   95),    p(  58,  100),    p(  66,   98),    p(  62,   94),    p(  70,   85),    p(  55,   85),
        p(  48,   99),    p(  51,   97),    p(  57,   99),    p(  56,  107),    p(  52,  110),    p(  73,  100),    p(  75,   86),    p(  57,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 171,  272),    p( 203,  303),    p( 241,  314),    p( 266,  304),    p( 300,  306),    p( 225,  298),    p( 232,  300),    p( 205,  253),
        p( 274,  303),    p( 285,  313),    p( 299,  310),    p( 312,  313),    p( 302,  310),    p( 334,  296),    p( 289,  307),    p( 292,  293),
        p( 290,  301),    p( 302,  307),    p( 321,  316),    p( 322,  319),    p( 339,  313),    p( 362,  303),    p( 313,  303),    p( 310,  298),
        p( 301,  310),    p( 309,  309),    p( 317,  321),    p( 343,  323),    p( 319,  325),    p( 334,  321),    p( 313,  312),    p( 329,  304),
        p( 298,  313),    p( 298,  308),    p( 305,  322),    p( 312,  325),    p( 319,  326),    p( 318,  313),    p( 327,  304),    p( 315,  308),
        p( 272,  300),    p( 278,  304),    p( 287,  304),    p( 293,  319),    p( 304,  314),    p( 297,  297),    p( 299,  296),    p( 296,  303),
        p( 268,  305),    p( 280,  309),    p( 278,  305),    p( 291,  309),    p( 300,  304),    p( 288,  301),    p( 294,  302),    p( 290,  314),
        p( 242,  299),    p( 275,  301),    p( 263,  303),    p( 283,  307),    p( 293,  305),    p( 286,  297),    p( 278,  304),    p( 264,  299),
    ],
    // bishop
    [
        p( 275,  316),    p( 254,  314),    p( 249,  305),    p( 224,  314),    p( 224,  313),    p( 226,  305),    p( 282,  304),    p( 248,  309),
        p( 281,  303),    p( 287,  305),    p( 286,  307),    p( 284,  305),    p( 286,  301),    p( 292,  303),    p( 269,  308),    p( 268,  304),
        p( 300,  307),    p( 303,  305),    p( 298,  310),    p( 304,  303),    p( 307,  306),    p( 333,  309),    p( 315,  304),    p( 312,  310),
        p( 283,  308),    p( 301,  307),    p( 301,  306),    p( 315,  314),    p( 311,  310),    p( 306,  310),    p( 304,  306),    p( 284,  309),
        p( 294,  305),    p( 285,  309),    p( 299,  310),    p( 314,  310),    p( 311,  310),    p( 300,  307),    p( 295,  306),    p( 312,  298),
        p( 294,  306),    p( 303,  310),    p( 303,  309),    p( 304,  310),    p( 309,  310),    p( 309,  305),    p( 306,  302),    p( 312,  298),
        p( 307,  310),    p( 303,  301),    p( 310,  303),    p( 302,  306),    p( 308,  304),    p( 307,  305),    p( 313,  300),    p( 303,  297),
        p( 296,  304),    p( 313,  309),    p( 301,  308),    p( 288,  311),    p( 299,  309),    p( 293,  313),    p( 301,  298),    p( 299,  295),
    ],
    // rook
    [
        p( 451,  553),    p( 442,  562),    p( 440,  568),    p( 440,  565),    p( 449,  562),    p( 469,  556),    p( 478,  554),    p( 487,  547),
        p( 434,  557),    p( 433,  562),    p( 444,  562),    p( 461,  552),    p( 448,  555),    p( 470,  549),    p( 479,  546),    p( 488,  538),
        p( 437,  554),    p( 457,  550),    p( 455,  551),    p( 457,  547),    p( 485,  536),    p( 493,  532),    p( 517,  528),    p( 485,  532),
        p( 433,  554),    p( 442,  550),    p( 443,  553),    p( 450,  547),    p( 455,  540),    p( 465,  535),    p( 473,  536),    p( 467,  532),
        p( 429,  550),    p( 429,  549),    p( 432,  550),    p( 439,  547),    p( 443,  544),    p( 436,  544),    p( 458,  535),    p( 446,  533),
        p( 427,  546),    p( 429,  544),    p( 433,  543),    p( 438,  543),    p( 444,  537),    p( 449,  531),    p( 473,  517),    p( 453,  521),
        p( 431,  539),    p( 437,  540),    p( 447,  540),    p( 449,  538),    p( 458,  531),    p( 464,  524),    p( 474,  517),    p( 440,  527),
        p( 450,  541),    p( 446,  537),    p( 448,  542),    p( 455,  536),    p( 463,  530),    p( 461,  531),    p( 459,  528),    p( 455,  528),
    ],
    // queen
    [
        p( 866,  972),    p( 869,  985),    p( 884,  998),    p( 902,  994),    p( 901,  996),    p( 921,  985),    p( 971,  933),    p( 912,  969),
        p( 884,  960),    p( 865,  986),    p( 867, 1013),    p( 857, 1033),    p( 865, 1044),    p( 906, 1003),    p( 907,  985),    p( 941,  970),
        p( 892,  966),    p( 887,  981),    p( 890, 1000),    p( 886, 1010),    p( 907, 1013),    p( 947,  998),    p( 952,  970),    p( 937,  979),
        p( 878,  978),    p( 886,  986),    p( 883,  990),    p( 883, 1003),    p( 889, 1013),    p( 898, 1008),    p( 904, 1011),    p( 910,  990),
        p( 888,  970),    p( 879,  982),    p( 887,  982),    p( 890,  995),    p( 892,  993),    p( 893,  994),    p( 903,  985),    p( 907,  982),
        p( 885,  951),    p( 894,  964),    p( 892,  973),    p( 890,  976),    p( 897,  979),    p( 904,  970),    p( 916,  954),    p( 910,  950),
        p( 887,  950),    p( 891,  952),    p( 899,  953),    p( 901,  962),    p( 903,  961),    p( 906,  938),    p( 912,  919),    p( 915,  903),
        p( 879,  945),    p( 886,  939),    p( 887,  949),    p( 897,  948),    p( 899,  939),    p( 883,  938),    p( 882,  933),    p( 888,  923),
    ],
    // king
    [
        p(  69,  -91),    p(  33,  -43),    p(  56,  -34),    p( -25,   -2),    p(   2,  -14),    p( -10,   -5),    p(  43,  -15),    p( 155,  -96),
        p( -30,   -1),    p(  23,   22),    p(  11,   32),    p(  75,   22),    p(  45,   30),    p(  24,   42),    p(  57,   28),    p(  16,   -0),
        p( -48,    8),    p(  58,   18),    p(  10,   36),    p(   3,   44),    p(  38,   38),    p(  75,   31),    p(  40,   29),    p( -21,   11),
        p( -30,    2),    p(  -6,   18),    p( -24,   36),    p( -42,   45),    p( -47,   43),    p( -33,   36),    p( -38,   26),    p( -95,   17),
        p( -48,   -1),    p( -27,   14),    p( -44,   31),    p( -65,   45),    p( -73,   43),    p( -54,   29),    p( -60,   19),    p(-109,   14),
        p( -34,    2),    p(   6,    7),    p( -35,   23),    p( -47,   33),    p( -43,   31),    p( -45,   23),    p( -14,    8),    p( -61,   11),
        p(  24,   -6),    p(  17,    3),    p(  -1,   12),    p( -21,   20),    p( -24,   21),    p( -12,   13),    p(  24,   -2),    p(   5,   -0),
        p( -24,  -33),    p(  28,  -44),    p(  12,  -28),    p( -47,   -9),    p(   7,  -28),    p( -44,  -12),    p(  14,  -37),    p(   3,  -45),
    ],
];

const BISHOP_PAIR: PhasedScore = p(22, 58);
const ROOK_OPEN_FILE: PhasedScore = p(20, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -2);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(4, 2);
const KING_OPEN_FILE: PhasedScore = p(-55, -2);
const KING_CLOSED_FILE: PhasedScore = p(16, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 6);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 5), p(-3, 6), p(-2, 10), p(4, 7), p(2, 12), p(4, 13), p(12, 12), p(22, 8)],
    // Closed
    [p(0, 0), p(0, 0), p(7, -31), p(-18, 9), p(-3, 14), p(-1, 5), p(1, 11), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-13, 20), p(3, 20), p(3, 13), p(-2, 21), p(3, 15), p(4, 11), p(13, 11)],
    // SemiClosed
    [p(0, 0), p(8, -11), p(4, 8), p(2, 2), p(4, 7), p(-0, 6), p(6, 8), p(1, 4)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-37, 9),   /*0b0000*/
    p(-23, 12),  /*0b0001*/
    p(-7, 6),    /*0b0010*/
    p(4, 12),    /*0b0011*/
    p(-13, 7),   /*0b0100*/
    p(-13, 1),   /*0b0101*/
    p(-1, 4),    /*0b0110*/
    p(18, -22),  /*0b0111*/
    p(-26, 13),  /*0b1000*/
    p(-13, 11),  /*0b1001*/
    p(-9, 8),    /*0b1010*/
    p(10, 8),    /*0b1011*/
    p(-10, 6),   /*0b1100*/
    p(-10, 6),   /*0b1101*/
    p(1, 1),     /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-30, 21),  /*0b10000*/
    p(-5, 11),   /*0b10001*/
    p(11, 13),   /*0b10010*/
    p(10, 6),    /*0b10011*/
    p(-15, 7),   /*0b10100*/
    p(26, 14),   /*0b10101*/
    p(-10, -0),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(-20, 36),  /*0b11000*/
    p(20, 25),   /*0b11001*/
    p(33, 38),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(6, 12),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(-17, 13),  /*0b100000*/
    p(-4, 14),   /*0b100001*/
    p(14, 3),    /*0b100010*/
    p(19, -2),   /*0b100011*/
    p(-19, 4),   /*0b100100*/
    p(-10, -10), /*0b100101*/
    p(-7, 12),   /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(-14, 5),   /*0b101000*/
    p(-11, 16),  /*0b101001*/
    p(8, -4),    /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-15, 5),   /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(-17, 23),  /*0b110000*/
    p(16, 16),   /*0b110001*/
    p(21, 11),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(0, 30),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(-8, 17),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(19, -4),   /*0b111111*/
    p(-63, -3),  /*0b00*/
    p(-11, -22), /*0b01*/
    p(15, -10),  /*0b10*/
    p(24, -48),  /*0b11*/
    p(3, -13),   /*0b100*/
    p(-31, -24), /*0b101*/
    p(49, -46),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(14, -14),  /*0b1000*/
    p(-6, -40),  /*0b1001*/
    p(54, -60),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(8, -22),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(12, -5),   /*0b1111*/
    p(-35, -3),  /*0b00*/
    p(-1, -15),  /*0b01*/
    p(-4, -22),  /*0b10*/
    p(11, -50),  /*0b11*/
    p(-25, -11), /*0b100*/
    p(19, -24),  /*0b101*/
    p(-10, -30), /*0b110*/
    p(0, 0),     /*0b111*/
    p(-17, -6),  /*0b1000*/
    p(17, -23),  /*0b1001*/
    p(16, -48),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(-15, -24), /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(6, -53),   /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  33,   86),    p(  31,   85),    p(  21,   87),    p(  34,   67),    p(  18,   72),    p(  15,   76),    p( -23,   95),    p( -22,   94),
        p(  42,  122),    p(  47,  122),    p(  36,   98),    p(  23,   64),    p(  27,   67),    p(  13,   94),    p( -11,  103),    p( -34,  124),
        p(  25,   72),    p(  18,   69),    p(  24,   53),    p(  16,   42),    p(  -1,   43),    p(   9,   56),    p(  -9,   73),    p( -11,   77),
        p(   8,   45),    p(  -2,   43),    p( -15,   33),    p(  -9,   24),    p( -16,   29),    p( -10,   37),    p( -18,   53),    p( -10,   49),
        p(   2,   14),    p( -12,   22),    p( -14,   16),    p( -15,    8),    p( -13,   13),    p(  -7,   16),    p( -13,   35),    p(   9,   16),
        p(  -5,   15),    p(  -1,   20),    p(  -9,   16),    p(  -8,    4),    p(   6,   -0),    p(   8,    6),    p(  12,   18),    p(   6,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-10, -10);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(15, 7), p(3, 10), p(9, 14), p(8, 9), p(-5, 19), p(-32, 7)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(39, 9), p(42, 35), p(54, -9), p(37, -39), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-52, -53),
        p(-31, -17),
        p(-17, 4),
        p(-6, 14),
        p(4, 22),
        p(13, 30),
        p(23, 30),
        p(31, 29),
        p(39, 24),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-24, -45),
        p(-14, -27),
        p(-4, -12),
        p(2, -2),
        p(8, 7),
        p(12, 14),
        p(14, 18),
        p(16, 21),
        p(17, 25),
        p(22, 25),
        p(25, 23),
        p(33, 23),
        p(27, 30),
        p(39, 21),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-61, 12),
        p(-51, 27),
        p(-47, 31),
        p(-44, 36),
        p(-47, 43),
        p(-43, 47),
        p(-41, 52),
        p(-38, 54),
        p(-36, 57),
        p(-34, 61),
        p(-30, 63),
        p(-29, 67),
        p(-22, 67),
        p(-15, 65),
        p(-14, 66),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-15, -71),
        p(-19, -5),
        p(-24, 50),
        p(-20, 67),
        p(-19, 85),
        p(-15, 92),
        p(-12, 104),
        p(-9, 112),
        p(-6, 117),
        p(-4, 121),
        p(-2, 125),
        p(1, 130),
        p(3, 131),
        p(4, 137),
        p(6, 140),
        p(8, 144),
        p(9, 151),
        p(10, 151),
        p(19, 149),
        p(32, 142),
        p(35, 144),
        p(78, 122),
        p(74, 126),
        p(99, 107),
        p(190, 73),
        p(237, 33),
        p(269, 20),
        p(332, -19),
    ],
    [
        p(22, 29),
        p(21, 8),
        p(14, 2),
        p(7, -1),
        p(1, -3),
        p(-14, -7),
        p(-27, -2),
        p(-42, -7),
        p(-32, -28),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
const THREATS: [[PhasedScore; NUM_CHESS_PIECES - 1]; NUM_CHESS_PIECES - 1] = [
    [p(-13, 14), p(-4, -1), p(23, 19), p(48, -14), p(22, -42)],
    [p(-3, 15), p(19, 22), p(-1, 11), p(28, 2), p(28, 54)],
    [p(-4, 18), p(16, 20), p(18, 20), p(-4, 9), p(38, -6)],
    [p(-4, 2), p(4, 14), p(-3, 31), p(-2, 7), p(2, -20)],
    [p(61, 34), p(-19, 20), p(-1, 18), p(-35, 10), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 4), p(15, 3), p(12, 10), p(17, 5), p(12, 16), p(0, 0)],
    [
        p(1, -1),
        p(10, 16),
        p(-80, -40),
        p(6, 14),
        p(13, 10),
        p(0, 0),
    ],
    [p(-2, 0), p(10, 2), p(4, 8), p(3, 8), p(7, 18), p(0, 0)],
    [p(1, 1), p(3, 10), p(5, 0), p(-2, 19), p(-71, -236), p(0, 0)],
    [p(5, -3), p(4, 1), p(14, -3), p(-4, -1), p(5, -16), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(29, -15),
    p(18, -8),
    p(18, -4),
    p(27, -14),
    p(8, 21),
    p(34, 15),
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

    fn king_zone_attack(attacking: ChessPieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }
}
