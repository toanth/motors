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
        p( 133,  186),    p( 130,  185),    p( 120,  189),    p( 133,  169),    p( 119,  174),    p( 120,  177),    p(  83,  195),    p(  91,  192),
        p(  65,  123),    p(  62,  124),    p(  74,  120),    p(  82,  124),    p(  67,  125),    p( 117,  111),    p(  92,  132),    p(  88,  122),
        p(  51,  113),    p(  63,  109),    p(  61,  103),    p(  66,   97),    p(  82,   98),    p(  83,   94),    p(  77,  103),    p(  71,   96),
        p(  48,  100),    p(  55,  102),    p(  64,   95),    p(  73,   94),    p(  77,   92),    p(  77,   88),    p(  71,   92),    p(  59,   86),
        p(  43,   97),    p(  51,   94),    p(  55,   94),    p(  59,  100),    p(  67,   97),    p(  62,   93),    p(  70,   84),    p(  54,   85),
        p(  50,   98),    p(  51,   97),    p(  58,   98),    p(  57,  105),    p(  54,  108),    p(  72,   98),    p(  73,   84),    p(  55,   87),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 187,  275),    p( 211,  307),    p( 245,  319),    p( 269,  310),    p( 301,  311),    p( 215,  307),    p( 232,  306),    p( 215,  257),
        p( 277,  309),    p( 288,  317),    p( 302,  314),    p( 315,  317),    p( 306,  314),    p( 329,  302),    p( 288,  313),    p( 292,  300),
        p( 293,  306),    p( 304,  311),    p( 321,  320),    p( 323,  324),    p( 339,  317),    p( 363,  307),    p( 315,  308),    p( 310,  304),
        p( 306,  315),    p( 312,  313),    p( 319,  325),    p( 345,  327),    p( 322,  328),    p( 336,  325),    p( 317,  316),    p( 334,  309),
        p( 302,  318),    p( 301,  312),    p( 307,  325),    p( 314,  328),    p( 321,  329),    p( 318,  316),    p( 329,  308),    p( 318,  314),
        p( 277,  305),    p( 278,  308),    p( 286,  308),    p( 292,  322),    p( 300,  319),    p( 285,  303),    p( 300,  299),    p( 296,  308),
        p( 273,  310),    p( 283,  313),    p( 280,  309),    p( 291,  313),    p( 294,  308),    p( 287,  304),    p( 297,  305),    p( 292,  318),
        p( 246,  305),    p( 284,  304),    p( 268,  306),    p( 288,  311),    p( 298,  309),    p( 293,  298),    p( 291,  304),    p( 270,  304),
    ],
    // bishop
    [
        p( 281,  316),    p( 261,  323),    p( 251,  317),    p( 232,  327),    p( 230,  326),    p( 229,  317),    p( 288,  314),    p( 253,  310),
        p( 285,  312),    p( 289,  314),    p( 294,  319),    p( 288,  321),    p( 291,  316),    p( 300,  315),    p( 276,  317),    p( 279,  313),
        p( 301,  319),    p( 309,  317),    p( 303,  322),    p( 308,  316),    p( 312,  319),    p( 340,  320),    p( 326,  315),    p( 315,  322),
        p( 289,  322),    p( 305,  322),    p( 306,  319),    p( 323,  323),    p( 317,  319),    p( 312,  323),    p( 308,  321),    p( 290,  323),
        p( 299,  320),    p( 287,  324),    p( 305,  323),    p( 321,  320),    p( 319,  320),    p( 304,  320),    p( 297,  322),    p( 319,  311),
        p( 296,  318),    p( 311,  322),    p( 308,  321),    p( 310,  323),    p( 313,  324),    p( 311,  318),    p( 312,  314),    p( 313,  311),
        p( 314,  320),    p( 306,  309),    p( 317,  315),    p( 303,  323),    p( 309,  321),    p( 310,  318),    p( 317,  309),    p( 308,  308),
        p( 296,  306),    p( 319,  318),    p( 309,  317),    p( 298,  323),    p( 311,  321),    p( 299,  324),    p( 308,  307),    p( 302,  297),
    ],
    // rook
    [
        p( 458,  552),    p( 450,  561),    p( 447,  567),    p( 445,  564),    p( 457,  560),    p( 477,  555),    p( 485,  553),    p( 495,  546),
        p( 433,  558),    p( 430,  563),    p( 439,  564),    p( 455,  554),    p( 445,  556),    p( 465,  551),    p( 476,  547),    p( 491,  538),
        p( 438,  554),    p( 456,  550),    p( 454,  551),    p( 458,  547),    p( 485,  536),    p( 494,  532),    p( 516,  529),    p( 489,  531),
        p( 436,  554),    p( 443,  550),    p( 444,  553),    p( 449,  547),    p( 458,  539),    p( 467,  534),    p( 474,  536),    p( 470,  531),
        p( 431,  550),    p( 431,  548),    p( 432,  549),    p( 438,  546),    p( 445,  542),    p( 439,  541),    p( 458,  534),    p( 448,  533),
        p( 428,  545),    p( 427,  544),    p( 431,  543),    p( 433,  543),    p( 440,  537),    p( 448,  530),    p( 471,  517),    p( 453,  521),
        p( 431,  540),    p( 434,  540),    p( 440,  542),    p( 443,  540),    p( 451,  533),    p( 465,  523),    p( 474,  518),    p( 442,  527),
        p( 440,  544),    p( 436,  540),    p( 438,  545),    p( 442,  541),    p( 450,  535),    p( 456,  534),    p( 453,  531),    p( 447,  532),
    ],
    // queen
    [
        p( 871,  959),    p( 873,  973),    p( 887,  986),    p( 904,  983),    p( 902,  986),    p( 922,  974),    p( 972,  922),    p( 918,  953),
        p( 880,  952),    p( 856,  983),    p( 857, 1010),    p( 850, 1028),    p( 857, 1039),    p( 897,  999),    p( 900,  980),    p( 942,  958),
        p( 888,  956),    p( 880,  976),    p( 880,  999),    p( 878, 1008),    p( 900, 1009),    p( 940,  993),    p( 947,  962),    p( 934,  968),
        p( 873,  971),    p( 879,  981),    p( 872,  991),    p( 871, 1005),    p( 877, 1015),    p( 889, 1005),    p( 898, 1004),    p( 905,  981),
        p( 884,  960),    p( 871,  980),    p( 877,  983),    p( 877, 1001),    p( 878,  999),    p( 880,  998),    p( 895,  983),    p( 902,  975),
        p( 880,  946),    p( 885,  964),    p( 878,  980),    p( 876,  983),    p( 881,  990),    p( 887,  980),    p( 902,  961),    p( 901,  948),
        p( 882,  944),    p( 880,  953),    p( 887,  956),    p( 886,  970),    p( 887,  970),    p( 889,  952),    p( 898,  930),    p( 908,  903),
        p( 869,  941),    p( 880,  931),    p( 880,  946),    p( 888,  947),    p( 890,  940),    p( 878,  940),    p( 879,  930),    p( 883,  918),
    ],
    // king
    [
        p(  79,  -78),    p(  43,  -42),    p(  84,  -37),    p(   5,   -5),    p(  15,  -15),    p(   7,   -6),    p(  67,  -17),    p(  86,  -85),
        p(  -6,    6),    p( -27,   25),    p( -36,   34),    p(  -3,   29),    p( -37,   37),    p( -49,   48),    p( -36,   37),    p(   0,   -0),
        p( -13,   13),    p( -26,   25),    p( -73,   44),    p( -78,   51),    p( -46,   46),    p( -12,   38),    p( -36,   35),    p(  -8,    7),
        p(  -8,    8),    p( -83,   25),    p( -99,   43),    p(-121,   52),    p(-119,   50),    p(-100,   42),    p(-105,   31),    p( -98,   15),
        p( -25,    4),    p(-105,   21),    p(-115,   38),    p(-138,   51),    p(-144,   49),    p(-120,   35),    p(-133,   26),    p(-117,   12),
        p( -10,    7),    p( -80,   16),    p(-110,   30),    p(-119,   40),    p(-113,   38),    p(-127,   31),    p( -98,   16),    p( -74,    9),
        p(  49,   -1),    p( -63,   10),    p( -75,   19),    p( -96,   28),    p(-101,   29),    p( -86,   20),    p( -55,    4),    p(   5,   -4),
        p(  64,  -33),    p(  45,  -43),    p(  40,  -31),    p( -21,  -10),    p(  32,  -29),    p( -18,  -14),    p(  38,  -39),    p(  60,  -52),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(16, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(0, 2);
const KING_OPEN_FILE: PhasedScore = p(-56, -1);
const KING_CLOSED_FILE: PhasedScore = p(16, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 7);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 6), p(-4, 5), p(-4, 3), p(0, 3), p(-1, 1), p(2, 6), p(6, 2), p(19, 3)],
    // Closed
    [p(0, 0), p(0, 0), p(10, -10), p(-15, 13), p(-5, 4), p(1, -2), p(-2, 1), p(-3, 2)],
    // SemiOpen
    [p(0, 0), p(-21, 21), p(0, 14), p(-2, 10), p(-6, 10), p(1, 9), p(-3, 2), p(9, 7)],
    // SemiClosed
    [p(0, 0), p(10, -12), p(7, 0), p(2, -3), p(3, -4), p(2, -1), p(4, -2), p(0, 0)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-9, 4),    /*0b0000*/
    p(-17, 9),   /*0b0001*/
    p(-5, 5),    /*0b0010*/
    p(-12, 12),  /*0b0011*/
    p(-7, 4),    /*0b0100*/
    p(-28, 2),   /*0b0101*/
    p(-16, 4),   /*0b0110*/
    p(-21, -18), /*0b0111*/
    p(4, 8),     /*0b1000*/
    p(-8, 8),    /*0b1001*/
    p(-0, 6),    /*0b1010*/
    p(-6, 6),    /*0b1011*/
    p(-4, 4),    /*0b1100*/
    p(-27, 7),   /*0b1101*/
    p(-14, 2),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-1, 15),   /*0b10000*/
    p(2, 9),     /*0b10001*/
    p(19, 11),   /*0b10010*/
    p(-5, 7),    /*0b10011*/
    p(-7, 6),    /*0b10100*/
    p(11, 15),   /*0b10101*/
    p(-23, 1),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(10, 31),   /*0b11000*/
    p(29, 24),   /*0b11001*/
    p(39, 37),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(14, 11),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(12, 7),    /*0b100000*/
    p(2, 12),    /*0b100001*/
    p(23, 1),    /*0b100010*/
    p(4, -1),    /*0b100011*/
    p(-12, 1),   /*0b100100*/
    p(-25, -9),  /*0b100101*/
    p(-22, 20),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(17, -0),   /*0b101000*/
    p(-5, 15),   /*0b101001*/
    p(18, -5),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-9, 8),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(12, 18),   /*0b110000*/
    p(24, 15),   /*0b110001*/
    p(30, 10),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(5, 29),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(22, 13),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(4, -3),    /*0b111111*/
    p(-41, -17), /*0b00*/
    p(-11, -33), /*0b01*/
    p(20, -21),  /*0b10*/
    p(5, -57),   /*0b11*/
    p(26, -25),  /*0b100*/
    p(-25, -35), /*0b101*/
    p(55, -55),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(28, -27),  /*0b1000*/
    p(-1, -52),  /*0b1001*/
    p(71, -71),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(34, -34),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-5, 4),    /*0b1111*/
    p(16, -9),   /*0b00*/
    p(33, -19),  /*0b01*/
    p(26, -26),  /*0b10*/
    p(24, -51),  /*0b11*/
    p(32, -17),  /*0b100*/
    p(54, -28),  /*0b101*/
    p(24, -32),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(37, -11),  /*0b1000*/
    p(55, -25),  /*0b1001*/
    p(51, -51),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(41, -30),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(23, -53),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  33,   86),    p(  30,   85),    p(  20,   89),    p(  33,   69),    p(  19,   74),    p(  20,   77),    p( -17,   95),    p(  -9,   92),
        p(  41,  122),    p(  48,  122),    p(  37,   99),    p(  21,   67),    p(  36,   66),    p(  15,   94),    p(   0,  102),    p( -28,  123),
        p(  24,   72),    p(  17,   70),    p(  23,   53),    p(  16,   43),    p(  -1,   44),    p(   7,   57),    p( -11,   74),    p( -10,   77),
        p(   8,   45),    p(  -3,   43),    p( -15,   34),    p(  -9,   24),    p( -17,   29),    p( -11,   37),    p( -19,   54),    p( -11,   50),
        p(   2,   14),    p( -12,   22),    p( -15,   16),    p( -16,    8),    p( -14,   13),    p(  -9,   17),    p( -14,   36),    p(   9,   16),
        p(  -4,   15),    p(  -2,   20),    p(  -9,   16),    p(  -7,    6),    p(   4,    0),    p(   7,    7),    p(  12,   18),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-11, -10);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 7), p(2, 9), p(10, 14), p(9, 9), p(-4, 19), p(-46, 9)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(38, 9), p(42, 36), p(51, -9), p(36, -37), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-60, -62),
        p(-37, -23),
        p(-21, -2),
        p(-9, 9),
        p(2, 18),
        p(12, 26),
        p(23, 26),
        p(32, 24),
        p(40, 20),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-25, -47),
        p(-13, -29),
        p(-3, -14),
        p(3, -2),
        p(10, 7),
        p(14, 15),
        p(16, 19),
        p(19, 23),
        p(20, 27),
        p(26, 27),
        p(29, 26),
        p(37, 27),
        p(31, 35),
        p(44, 27),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-67, 26),
        p(-62, 31),
        p(-59, 36),
        p(-59, 42),
        p(-54, 46),
        p(-51, 50),
        p(-47, 53),
        p(-43, 57),
        p(-40, 60),
        p(-34, 62),
        p(-31, 66),
        p(-21, 65),
        p(-9, 62),
        p(-6, 63),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-31, -27),
        p(-32, 32),
        p(-35, 79),
        p(-30, 96),
        p(-27, 113),
        p(-22, 118),
        p(-18, 129),
        p(-15, 135),
        p(-10, 139),
        p(-7, 141),
        p(-4, 145),
        p(-0, 147),
        p(2, 148),
        p(4, 153),
        p(6, 154),
        p(10, 157),
        p(11, 163),
        p(14, 162),
        p(23, 159),
        p(37, 151),
        p(42, 150),
        p(85, 126),
        p(85, 127),
        p(109, 107),
        p(200, 73),
        p(253, 28),
        p(288, 12),
        p(340, -23),
    ],
    [
        p(-82, 46),
        p(-51, 19),
        p(-26, 8),
        p(-1, 1),
        p(25, -5),
        p(43, -12),
        p(64, -12),
        p(84, -19),
        p(128, -44),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-11, 11),
        p(-6, -3),
        p(23, 17),
        p(49, -15),
        p(21, -44),
        p(0, 0),
    ],
    [p(-1, 13), p(19, 21), p(-2, 7), p(28, 2), p(27, 56), p(0, 0)],
    [
        p(3, 17),
        p(22, 20),
        p(23, 21),
        p(-6, 10),
        p(43, -5),
        p(0, 0),
    ],
    [p(-0, -1), p(7, 12), p(-0, 30), p(0, 5), p(2, -17), p(0, 0)],
    [p(78, 34), p(-30, 22), p(2, 19), p(-33, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 5), p(11, 4), p(9, 10), p(15, 5), p(9, 16), p(13, 3)],
    [
        p(-3, 1),
        p(8, 18),
        p(-100, -35),
        p(6, 12),
        p(7, 16),
        p(4, 5),
    ],
    [p(2, 2), p(14, 4), p(9, 10), p(11, 7), p(12, 15), p(22, -6)],
    [
        p(3, -5),
        p(10, -2),
        p(8, -8),
        p(4, 15),
        p(-56, -261),
        p(7, -11),
    ],
    [
        p(59, -8),
        p(37, -0),
        p(42, -6),
        p(20, -2),
        p(32, -18),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-11, -11),
    p(17, -8),
    p(17, -3),
    p(23, -13),
    p(6, 22),
    p(7, 19),
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
