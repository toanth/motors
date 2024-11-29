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
        p( 132,  186),    p( 130,  185),    p( 120,  187),    p( 134,  166),    p( 118,  172),    p( 114,  176),    p(  77,  195),    p(  78,  194),
        p(  62,  124),    p(  62,  124),    p(  76,  119),    p(  81,  125),    p(  74,  122),    p( 115,  110),    p( 108,  128),    p(  83,  122),
        p(  49,  113),    p(  62,  109),    p(  60,  103),    p(  66,   97),    p(  82,   98),    p(  83,   94),    p(  77,  104),    p(  71,   95),
        p(  45,  100),    p(  53,  103),    p(  62,   95),    p(  72,   94),    p(  75,   93),    p(  76,   89),    p(  69,   93),    p(  58,   86),
        p(  40,   98),    p(  48,   95),    p(  53,   95),    p(  56,  100),    p(  64,   98),    p(  61,   94),    p(  67,   85),    p(  53,   86),
        p(  46,   99),    p(  49,   97),    p(  55,   99),    p(  54,  107),    p(  52,  110),    p(  71,  100),    p(  74,   85),    p(  54,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 172,  269),    p( 202,  302),    p( 241,  314),    p( 265,  304),    p( 300,  306),    p( 226,  297),    p( 233,  298),    p( 206,  250),
        p( 275,  301),    p( 285,  312),    p( 298,  311),    p( 312,  314),    p( 302,  311),    p( 333,  297),    p( 289,  307),    p( 292,  292),
        p( 289,  302),    p( 300,  309),    p( 319,  319),    p( 321,  323),    p( 338,  316),    p( 360,  307),    p( 312,  305),    p( 310,  299),
        p( 300,  310),    p( 308,  311),    p( 316,  324),    p( 343,  326),    p( 318,  328),    p( 334,  325),    p( 312,  314),    p( 329,  305),
        p( 295,  313),    p( 295,  310),    p( 304,  326),    p( 313,  328),    p( 320,  330),    p( 315,  317),    p( 324,  307),    p( 312,  309),
        p( 270,  300),    p( 277,  306),    p( 285,  308),    p( 292,  323),    p( 304,  318),    p( 297,  301),    p( 296,  298),    p( 293,  303),
        p( 268,  303),    p( 280,  309),    p( 279,  307),    p( 292,  310),    p( 301,  305),    p( 288,  303),    p( 294,  301),    p( 288,  312),
        p( 239,  297),    p( 274,  299),    p( 260,  303),    p( 279,  309),    p( 289,  306),    p( 284,  297),    p( 277,  303),    p( 261,  297),
    ],
    // bishop
    [
        p( 275,  316),    p( 255,  314),    p( 250,  305),    p( 223,  314),    p( 224,  312),    p( 226,  305),    p( 283,  303),    p( 248,  310),
        p( 281,  302),    p( 287,  304),    p( 287,  306),    p( 284,  306),    p( 286,  302),    p( 293,  301),    p( 269,  307),    p( 267,  304),
        p( 299,  307),    p( 302,  304),    p( 298,  310),    p( 306,  303),    p( 308,  306),    p( 331,  310),    p( 314,  303),    p( 312,  310),
        p( 283,  308),    p( 300,  308),    p( 302,  306),    p( 316,  313),    p( 311,  309),    p( 306,  309),    p( 303,  307),    p( 284,  309),
        p( 292,  305),    p( 284,  309),    p( 300,  308),    p( 315,  308),    p( 312,  308),    p( 301,  306),    p( 296,  306),    p( 310,  298),
        p( 294,  305),    p( 302,  307),    p( 302,  308),    p( 303,  309),    p( 309,  309),    p( 309,  304),    p( 305,  299),    p( 311,  298),
        p( 308,  309),    p( 302,  299),    p( 309,  301),    p( 303,  307),    p( 309,  305),    p( 308,  303),    p( 313,  298),    p( 304,  297),
        p( 296,  304),    p( 311,  309),    p( 300,  308),    p( 289,  310),    p( 299,  308),    p( 291,  313),    p( 300,  297),    p( 300,  295),
    ],
    // rook
    [
        p( 450,  554),    p( 441,  563),    p( 439,  569),    p( 438,  566),    p( 448,  562),    p( 468,  557),    p( 478,  555),    p( 487,  548),
        p( 433,  558),    p( 432,  563),    p( 444,  563),    p( 460,  553),    p( 448,  556),    p( 470,  549),    p( 478,  547),    p( 488,  539),
        p( 436,  555),    p( 455,  550),    p( 453,  552),    p( 456,  548),    p( 483,  537),    p( 490,  534),    p( 516,  529),    p( 484,  533),
        p( 432,  555),    p( 440,  551),    p( 441,  554),    p( 449,  548),    p( 454,  541),    p( 464,  536),    p( 471,  537),    p( 465,  533),
        p( 427,  551),    p( 427,  551),    p( 430,  551),    p( 437,  548),    p( 442,  545),    p( 434,  545),    p( 455,  536),    p( 444,  534),
        p( 425,  547),    p( 427,  545),    p( 431,  545),    p( 437,  543),    p( 443,  538),    p( 447,  532),    p( 470,  518),    p( 451,  523),
        p( 428,  541),    p( 434,  541),    p( 443,  542),    p( 445,  540),    p( 454,  533),    p( 461,  525),    p( 471,  518),    p( 437,  528),
        p( 448,  541),    p( 445,  537),    p( 448,  542),    p( 455,  536),    p( 463,  530),    p( 460,  533),    p( 458,  529),    p( 454,  528),
    ],
    // queen
    [
        p( 866,  973),    p( 869,  986),    p( 883, 1000),    p( 900,  996),    p( 901,  998),    p( 921,  986),    p( 971,  934),    p( 912,  969),
        p( 884,  960),    p( 865,  987),    p( 867, 1014),    p( 856, 1035),    p( 864, 1045),    p( 907, 1003),    p( 907,  985),    p( 942,  970),
        p( 892,  965),    p( 886,  982),    p( 890, 1002),    p( 887, 1011),    p( 907, 1014),    p( 946,  999),    p( 951,  971),    p( 937,  979),
        p( 878,  978),    p( 885,  988),    p( 884,  991),    p( 884, 1004),    p( 889, 1014),    p( 898, 1009),    p( 904, 1011),    p( 911,  990),
        p( 887,  969),    p( 878,  983),    p( 887,  983),    p( 891,  996),    p( 892,  995),    p( 893,  995),    p( 904,  986),    p( 907,  982),
        p( 884,  952),    p( 893,  964),    p( 893,  975),    p( 891,  977),    p( 899,  980),    p( 905,  971),    p( 916,  955),    p( 909,  951),
        p( 887,  950),    p( 891,  952),    p( 898,  953),    p( 900,  963),    p( 903,  962),    p( 905,  940),    p( 912,  920),    p( 915,  903),
        p( 877,  944),    p( 884,  938),    p( 886,  949),    p( 897,  948),    p( 899,  939),    p( 882,  940),    p( 880,  935),    p( 887,  924),
    ],
    // king
    [
        p(  67,  -88),    p(  32,  -41),    p(  55,  -33),    p( -24,    0),    p(   1,  -13),    p( -11,   -3),    p(  43,  -13),    p( 154,  -94),
        p( -30,    0),    p(  26,   21),    p(  14,   31),    p(  77,   21),    p(  46,   30),    p(  26,   41),    p(  59,   27),    p(  16,    0),
        p( -49,    9),    p(  59,   17),    p(  12,   35),    p(   5,   43),    p(  41,   37),    p(  76,   30),    p(  41,   28),    p( -21,   12),
        p( -30,    2),    p(  -5,   18),    p( -22,   36),    p( -39,   44),    p( -44,   43),    p( -31,   35),    p( -36,   25),    p( -94,   17),
        p( -47,   -1),    p( -25,   13),    p( -42,   31),    p( -63,   44),    p( -70,   42),    p( -51,   28),    p( -57,   18),    p(-108,   14),
        p( -34,    2),    p(   7,    6),    p( -32,   22),    p( -45,   32),    p( -41,   31),    p( -42,   21),    p( -13,    7),    p( -60,   11),
        p(  24,   -6),    p(  19,    2),    p(   1,   11),    p( -18,   19),    p( -22,   20),    p( -11,   12),    p(  25,   -4),    p(   6,   -0),
        p( -25,  -31),    p(  27,  -42),    p(  11,  -26),    p( -48,   -7),    p(   6,  -26),    p( -45,  -10),    p(  14,  -35),    p(   2,  -43),
    ],
];

const BISHOP_PAIR: PhasedScore = p(22, 58);
const ROOK_OPEN_FILE: PhasedScore = p(21, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -2);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(6, 2);
const KING_OPEN_FILE: PhasedScore = p(-55, -2);
const KING_CLOSED_FILE: PhasedScore = p(16, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-10, 6);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 6), p(-3, 7), p(-1, 11), p(5, 8), p(3, 12), p(4, 14), p(12, 14), p(23, 8)],
    // Closed
    [p(0, 0), p(0, 0), p(4, -28), p(-21, 7), p(-5, 11), p(-3, 0), p(-0, 6), p(-3, 0)],
    // SemiOpen
    [p(0, 0), p(-14, 20), p(3, 21), p(3, 14), p(-2, 21), p(2, 15), p(4, 12), p(12, 12)],
    // SemiClosed
    [p(0, 0), p(5, -19), p(1, 3), p(-1, -3), p(3, 2), p(-2, 0), p(5, 3), p(0, -1)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-36, 10),  /*0b0000*/
    p(-22, 12),  /*0b0001*/
    p(-6, 6),    /*0b0010*/
    p(4, 11),    /*0b0011*/
    p(-13, 7),   /*0b0100*/
    p(-12, 0),   /*0b0101*/
    p(-0, 3),    /*0b0110*/
    p(17, -23),  /*0b0111*/
    p(-25, 14),  /*0b1000*/
    p(-12, 11),  /*0b1001*/
    p(-8, 9),    /*0b1010*/
    p(8, 7),     /*0b1011*/
    p(-10, 6),   /*0b1100*/
    p(-9, 5),    /*0b1101*/
    p(1, 0),     /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-29, 22),  /*0b10000*/
    p(-6, 12),   /*0b10001*/
    p(12, 13),   /*0b10010*/
    p(9, 5),     /*0b10011*/
    p(-14, 8),   /*0b10100*/
    p(25, 14),   /*0b10101*/
    p(-11, -1),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(-19, 37),  /*0b11000*/
    p(19, 25),   /*0b11001*/
    p(33, 39),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(6, 12),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(-16, 14),  /*0b100000*/
    p(-3, 14),   /*0b100001*/
    p(14, 3),    /*0b100010*/
    p(18, -3),   /*0b100011*/
    p(-18, 4),   /*0b100100*/
    p(-9, -11),  /*0b100101*/
    p(-8, 11),   /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(-14, 6),   /*0b101000*/
    p(-8, 17),   /*0b101001*/
    p(8, -3),    /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-16, 5),   /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(-16, 25),  /*0b110000*/
    p(16, 17),   /*0b110001*/
    p(22, 11),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(-0, 31),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(-8, 20),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(18, -4),   /*0b111111*/
    p(-63, -1),  /*0b00*/
    p(-10, -21), /*0b01*/
    p(15, -9),   /*0b10*/
    p(24, -49),  /*0b11*/
    p(3, -11),   /*0b100*/
    p(-32, -24), /*0b101*/
    p(48, -45),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(14, -13),  /*0b1000*/
    p(-6, -39),  /*0b1001*/
    p(53, -59),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(10, -20),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(15, -10),  /*0b1111*/
    p(-36, -1),  /*0b00*/
    p(-1, -14),  /*0b01*/
    p(-3, -21),  /*0b10*/
    p(12, -50),  /*0b11*/
    p(-25, -8),  /*0b100*/
    p(18, -23),  /*0b101*/
    p(-10, -28), /*0b110*/
    p(0, 0),     /*0b111*/
    p(-17, -4),  /*0b1000*/
    p(16, -21),  /*0b1001*/
    p(17, -47),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(-14, -22), /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(6, -53),   /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  32,   86),    p(  30,   85),    p(  20,   87),    p(  34,   66),    p(  18,   72),    p(  14,   76),    p( -23,   95),    p( -22,   94),
        p(  43,  122),    p(  48,  122),    p(  36,   98),    p(  23,   64),    p(  28,   67),    p(  14,   94),    p(  -7,  103),    p( -33,  123),
        p(  25,   72),    p(  18,   69),    p(  24,   52),    p(  16,   42),    p(  -1,   43),    p(   9,   56),    p(  -9,   73),    p( -11,   77),
        p(   9,   45),    p(  -2,   43),    p( -15,   33),    p(  -9,   24),    p( -16,   29),    p(  -9,   37),    p( -17,   53),    p( -10,   49),
        p(   3,   14),    p( -11,   22),    p( -14,   16),    p( -16,    8),    p( -13,   13),    p(  -7,   16),    p( -12,   35),    p(  10,   16),
        p(  -5,   15),    p(  -0,   19),    p(  -8,   16),    p(  -9,    5),    p(   6,   -0),    p(   9,    6),    p(  12,   18),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-11, -10);
const DOUBLED_PAWN: PhasedScore = p(-5, -22);
pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-52, -50),
        p(-31, -14),
        p(-17, 6),
        p(-7, 15),
        p(3, 23),
        p(11, 30),
        p(21, 29),
        p(30, 27),
        p(37, 21),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-5, -15),
        p(1, -4),
        p(7, 5),
        p(11, 13),
        p(13, 17),
        p(15, 20),
        p(15, 25),
        p(20, 24),
        p(23, 22),
        p(31, 23),
        p(24, 29),
        p(36, 21),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-63, 13),
        p(-53, 29),
        p(-49, 32),
        p(-46, 36),
        p(-49, 44),
        p(-44, 48),
        p(-42, 52),
        p(-40, 54),
        p(-37, 58),
        p(-35, 61),
        p(-32, 63),
        p(-30, 67),
        p(-24, 67),
        p(-16, 64),
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
        p(-16, -70),
        p(-19, -3),
        p(-24, 52),
        p(-20, 69),
        p(-19, 87),
        p(-15, 94),
        p(-11, 105),
        p(-9, 112),
        p(-6, 118),
        p(-3, 121),
        p(-2, 126),
        p(2, 130),
        p(4, 131),
        p(4, 137),
        p(6, 140),
        p(8, 143),
        p(8, 151),
        p(10, 151),
        p(18, 149),
        p(31, 142),
        p(34, 144),
        p(77, 122),
        p(72, 126),
        p(97, 107),
        p(188, 72),
        p(236, 32),
        p(266, 20),
        p(329, -18),
    ],
    [
        p(22, 28),
        p(22, 7),
        p(14, 1),
        p(8, -2),
        p(1, -3),
        p(-14, -6),
        p(-28, -1),
        p(-45, -5),
        p(-36, -25),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
const THREATS: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES] = [
    [
        p(18, -2),
        p(41, 10),
        p(43, 38),
        p(56, -9),
        p(38, -40),
        p(0, 0),
    ],
    [
        p(-12, 14),
        p(-2, 2),
        p(24, 19),
        p(48, -14),
        p(23, -43),
        p(0, 0),
    ],
    [p(-2, 16), p(20, 23), p(0, 12), p(28, 2), p(27, 54), p(0, 0)],
    [
        p(-4, 18),
        p(17, 20),
        p(18, 20),
        p(-4, 9),
        p(37, -6),
        p(0, 0),
    ],
    [p(-3, 1), p(5, 13), p(-3, 30), p(-2, 6), p(4, -24), p(0, 0)],
    [
        p(64, 35),
        p(-18, 20),
        p(-2, 19),
        p(-35, 10),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES] = [
    [p(18, 7), p(10, 8), p(12, 13), p(12, 9), p(0, 17), p(-33, 7)],
    [p(10, 2), p(22, 3), p(13, 7), p(16, 6), p(13, 12), p(0, 0)],
    [
        p(6, 7),
        p(14, 18),
        p(-66, -66),
        p(5, 15),
        p(14, 13),
        p(0, 0),
    ],
    [p(3, -1), p(14, 1), p(5, 7), p(2, 9), p(7, 19), p(0, 0)],
    [p(2, 2), p(7, 5), p(7, -6), p(-3, 18), p(-71, -242), p(0, 0)],
    [p(6, -1), p(7, 2), p(14, -2), p(-5, -0), p(6, -19), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(30, -16),
    p(18, -8),
    p(19, -4),
    p(26, -14),
    p(7, 21),
    p(35, 14),
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

    // fn pawn_protection(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    // fn pawn_attack(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

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

    // fn pawn_protection(piece: ChessPieceType) -> PhasedScore {
    //     PAWN_PROTECTION[piece as usize]
    // }
    //
    // fn pawn_attack(piece: ChessPieceType) -> PhasedScore {
    //     PAWN_ATTACKS[piece as usize]
    // }

    fn mobility(piece: ChessPieceType, mobility: usize) -> PhasedScore {
        MOBILITY[piece as usize - 1][mobility]
    }

    fn threats(attacking: ChessPieceType, targeted: ChessPieceType) -> PhasedScore {
        THREATS[attacking as usize][targeted as usize]
    }
    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> PhasedScore {
        DEFENDED[protecting as usize][target as usize]
    }

    fn king_zone_attack(attacking: ChessPieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }
}
