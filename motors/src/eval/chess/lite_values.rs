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
        p( 131,  187),    p( 128,  186),    p( 118,  189),    p( 130,  169),    p( 117,  173),    p( 117,  177),    p(  79,  195),    p(  86,  193),
        p(  63,  123),    p(  60,  125),    p(  73,  120),    p(  81,  124),    p(  69,  123),    p( 117,  110),    p(  91,  131),    p(  89,  122),
        p(  49,  113),    p(  62,  109),    p(  59,  104),    p(  62,   98),    p(  78,   99),    p(  82,   94),    p(  75,  103),    p(  69,   95),
        p(  46,   99),    p(  53,  102),    p(  63,   95),    p(  71,   93),    p(  75,   93),    p(  75,   88),    p(  68,   93),    p(  57,   86),
        p(  42,   97),    p(  51,   93),    p(  54,   94),    p(  58,  100),    p(  65,   96),    p(  60,   92),    p(  68,   83),    p(  52,   85),
        p(  47,   99),    p(  50,   96),    p(  56,   98),    p(  55,  105),    p(  52,  107),    p(  69,   98),    p(  71,   84),    p(  52,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 178,  274),    p( 198,  306),    p( 215,  318),    p( 252,  307),    p( 283,  309),    p( 198,  304),    p( 213,  305),    p( 206,  257),
        p( 270,  307),    p( 284,  314),    p( 298,  307),    p( 302,  310),    p( 302,  305),    p( 314,  295),    p( 276,  310),    p( 271,  299),
        p( 286,  304),    p( 302,  303),    p( 304,  312),    p( 318,  315),    p( 335,  308),    p( 347,  298),    p( 290,  303),    p( 286,  304),
        p( 300,  313),    p( 306,  309),    p( 321,  315),    p( 324,  322),    p( 321,  319),    p( 317,  318),    p( 308,  312),    p( 317,  308),
        p( 297,  315),    p( 300,  306),    p( 309,  315),    p( 318,  317),    p( 316,  321),    p( 321,  304),    p( 320,  303),    p( 312,  310),
        p( 273,  302),    p( 279,  303),    p( 292,  298),    p( 298,  312),    p( 303,  309),    p( 292,  292),    p( 298,  293),    p( 292,  305),
        p( 269,  310),    p( 279,  312),    p( 281,  304),    p( 292,  309),    p( 296,  303),    p( 286,  300),    p( 293,  304),    p( 288,  319),
        p( 238,  307),    p( 279,  303),    p( 265,  304),    p( 285,  310),    p( 293,  307),    p( 291,  296),    p( 286,  306),    p( 264,  306),
    ],
    // bishop
    [
        p( 280,  314),    p( 257,  314),    p( 242,  307),    p( 221,  316),    p( 218,  315),    p( 224,  307),    p( 278,  305),    p( 253,  308),
        p( 285,  303),    p( 281,  305),    p( 289,  306),    p( 278,  307),    p( 287,  302),    p( 292,  301),    p( 268,  307),    p( 272,  304),
        p( 296,  310),    p( 306,  304),    p( 293,  309),    p( 306,  302),    p( 307,  305),    p( 335,  307),    p( 317,  302),    p( 316,  311),
        p( 283,  311),    p( 292,  310),    p( 303,  305),    p( 309,  311),    p( 307,  307),    p( 300,  309),    p( 295,  310),    p( 279,  311),
        p( 290,  310),    p( 282,  310),    p( 296,  307),    p( 310,  307),    p( 304,  305),    p( 298,  304),    p( 287,  308),    p( 308,  303),
        p( 294,  309),    p( 301,  308),    p( 300,  307),    p( 301,  306),    p( 307,  307),    p( 303,  303),    p( 306,  299),    p( 308,  301),
        p( 307,  312),    p( 303,  300),    p( 310,  302),    p( 298,  309),    p( 303,  308),    p( 304,  305),    p( 312,  299),    p( 306,  298),
        p( 296,  305),    p( 313,  308),    p( 307,  307),    p( 290,  312),    p( 304,  310),    p( 295,  313),    p( 306,  298),    p( 302,  296),
    ],
    // rook
    [
        p( 458,  546),    p( 448,  556),    p( 441,  563),    p( 439,  560),    p( 451,  556),    p( 471,  550),    p( 483,  547),    p( 493,  540),
        p( 443,  552),    p( 441,  557),    p( 450,  559),    p( 465,  549),    p( 451,  552),    p( 467,  546),    p( 476,  543),    p( 490,  534),
        p( 444,  547),    p( 461,  542),    p( 456,  544),    p( 456,  539),    p( 483,  529),    p( 491,  526),    p( 509,  526),    p( 484,  527),
        p( 440,  548),    p( 445,  543),    p( 445,  546),    p( 451,  540),    p( 456,  532),    p( 466,  528),    p( 466,  532),    p( 466,  527),
        p( 433,  546),    p( 432,  543),    p( 433,  544),    p( 439,  540),    p( 445,  536),    p( 439,  536),    p( 453,  530),    p( 447,  528),
        p( 429,  543),    p( 429,  539),    p( 431,  538),    p( 435,  538),    p( 440,  532),    p( 451,  524),    p( 466,  513),    p( 453,  517),
        p( 431,  538),    p( 435,  537),    p( 440,  538),    p( 443,  535),    p( 451,  527),    p( 462,  518),    p( 470,  513),    p( 441,  523),
        p( 440,  542),    p( 437,  537),    p( 439,  541),    p( 443,  535),    p( 448,  528),    p( 455,  528),    p( 451,  527),    p( 447,  529),
    ],
    // queen
    [
        p( 883,  953),    p( 887,  967),    p( 901,  980),    p( 921,  974),    p( 918,  978),    p( 939,  965),    p( 985,  917),    p( 929,  949),
        p( 893,  945),    p( 866,  976),    p( 869, 1002),    p( 860, 1020),    p( 868, 1030),    p( 909,  991),    p( 909,  975),    p( 951,  954),
        p( 896,  952),    p( 888,  969),    p( 887,  991),    p( 888,  999),    p( 912, 1000),    p( 949,  984),    p( 957,  955),    p( 945,  961),
        p( 882,  964),    p( 887,  973),    p( 882,  983),    p( 882,  996),    p( 885, 1008),    p( 899,  997),    p( 907,  998),    p( 916,  972),
        p( 892,  956),    p( 879,  976),    p( 885,  977),    p( 885,  994),    p( 888,  990),    p( 891,  990),    p( 905,  977),    p( 911,  970),
        p( 887,  945),    p( 893,  960),    p( 887,  976),    p( 885,  979),    p( 891,  985),    p( 898,  973),    p( 912,  957),    p( 910,  945),
        p( 887,  946),    p( 886,  955),    p( 894,  956),    p( 893,  971),    p( 895,  970),    p( 896,  952),    p( 907,  931),    p( 915,  904),
        p( 874,  947),    p( 884,  936),    p( 885,  948),    p( 894,  950),    p( 897,  939),    p( 884,  943),    p( 886,  934),    p( 892,  917),
    ],
    // king
    [
        p( 156,  -84),    p(  58,  -38),    p(  82,  -31),    p(   7,    1),    p(  35,  -11),    p(  19,   -2),    p(  73,  -11),    p( 235,  -88),
        p( -30,    1),    p( -80,   19),    p( -82,   26),    p( -23,   17),    p( -54,   24),    p( -82,   39),    p( -49,   24),    p(   6,    1),
        p( -45,    9),    p( -49,   14),    p( -86,   28),    p( -96,   36),    p( -65,   31),    p( -31,   23),    p( -77,   25),    p( -36,   10),
        p( -27,    2),    p(-100,   12),    p(-114,   28),    p(-136,   37),    p(-136,   35),    p(-113,   27),    p(-132,   17),    p(-102,   17),
        p( -41,   -2),    p(-116,    8),    p(-126,   25),    p(-151,   38),    p(-153,   36),    p(-128,   22),    p(-143,   12),    p(-115,   13),
        p( -33,    2),    p( -92,    3),    p(-120,   18),    p(-126,   27),    p(-123,   26),    p(-133,   18),    p(-110,    4),    p( -72,   10),
        p(  26,   -8),    p( -79,   -2),    p( -90,    7),    p(-109,   16),    p(-115,   17),    p(-100,    8),    p( -73,   -9),    p(   4,   -4),
        p(  53,  -24),    p(  42,  -36),    p(  39,  -24),    p( -21,   -3),    p(  30,  -20),    p( -18,   -7),    p(  35,  -30),    p(  66,  -34),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 53);
const ROOK_OPEN_FILE: PhasedScore = p(13, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 4);
const KING_OPEN_FILE: PhasedScore = p(-49, -2);
const KING_CLOSED_FILE: PhasedScore = p(14, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 5), p(-1, 7), p(-0, 8), p(3, 6), p(3, 8), p(4, 10), p(7, 9), p(18, 5)],
    // Closed
    [p(0, 0), p(0, 0), p(14, -22), p(-16, 10), p(-1, 12), p(1, 4), p(0, 8), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-17, 24), p(3, 21), p(1, 12), p(0, 14), p(4, 9), p(0, 7), p(10, 9)],
    // SemiClosed
    [p(0, 0), p(10, -13), p(6, 7), p(2, 1), p(7, 3), p(2, 4), p(5, 6), p(1, 4)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-5, 6),    /*0b0000*/
    p(-14, 9),   /*0b0001*/
    p(-2, 8),    /*0b0010*/
    p(-11, 13),  /*0b0011*/
    p(-3, 4),    /*0b0100*/
    p(-25, -0),  /*0b0101*/
    p(-14, 5),   /*0b0110*/
    p(-21, -17), /*0b0111*/
    p(9, 11),    /*0b1000*/
    p(-2, 10),   /*0b1001*/
    p(3, 10),    /*0b1010*/
    p(-5, 10),   /*0b1011*/
    p(0, 5),     /*0b1100*/
    p(-22, 10),  /*0b1101*/
    p(-11, 3),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(4, 16),    /*0b10000*/
    p(2, 9),     /*0b10001*/
    p(22, 11),   /*0b10010*/
    p(-6, 5),    /*0b10011*/
    p(-5, 6),    /*0b10100*/
    p(11, 15),   /*0b10101*/
    p(-23, -0),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(15, 30),   /*0b11000*/
    p(29, 24),   /*0b11001*/
    p(42, 38),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(17, 11),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(16, 10),   /*0b100000*/
    p(4, 13),    /*0b100001*/
    p(25, 3),    /*0b100010*/
    p(5, -2),    /*0b100011*/
    p(-6, 2),    /*0b100100*/
    p(-19, -8),  /*0b100101*/
    p(-23, 15),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(22, 4),    /*0b101000*/
    p(2, 18),    /*0b101001*/
    p(21, -2),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-4, 7),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(17, 18),   /*0b110000*/
    p(25, 13),   /*0b110001*/
    p(33, 9),    /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(10, 30),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(26, 16),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(6, -3),    /*0b111111*/
    p(-13, -4),  /*0b00*/
    p(12, -17),  /*0b01*/
    p(40, -9),   /*0b10*/
    p(21, -40),  /*0b11*/
    p(48, -11),  /*0b100*/
    p(7, -20),   /*0b101*/
    p(70, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(64, -13),  /*0b1000*/
    p(21, -33),  /*0b1001*/
    p(83, -55),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(61, -20),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(20, -10),  /*0b1111*/
    p(21, -3),   /*0b00*/
    p(33, -13),  /*0b01*/
    p(27, -17),  /*0b10*/
    p(21, -42),  /*0b11*/
    p(38, -11),  /*0b100*/
    p(55, -21),  /*0b101*/
    p(25, -23),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(40, -4),   /*0b1000*/
    p(52, -17),  /*0b1001*/
    p(53, -42),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(45, -24),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(19, -43),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  31,   87),    p(  28,   86),    p(  18,   89),    p(  30,   69),    p(  17,   73),    p(  17,   77),    p( -21,   95),    p( -14,   93),
        p(  39,  124),    p(  47,  123),    p(  36,  100),    p(  20,   69),    p(  34,   69),    p(  15,   95),    p(  -1,  105),    p( -31,  125),
        p(  23,   74),    p(  17,   71),    p(  22,   54),    p(  16,   43),    p(  -1,   46),    p(   7,   59),    p( -10,   76),    p( -10,   79),
        p(   8,   46),    p(  -2,   44),    p( -15,   35),    p(  -9,   25),    p( -17,   29),    p( -10,   40),    p( -17,   55),    p( -10,   51),
        p(   1,   15),    p( -12,   24),    p( -15,   17),    p( -16,    9),    p( -15,   14),    p(  -7,   18),    p( -13,   38),    p(  10,   17),
        p(  -4,   15),    p(  -2,   20),    p(  -8,   17),    p(  -8,    4),    p(   5,    2),    p(   7,    8),    p(  13,   19),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-11, -10);
const DOUBLED_PAWN: PhasedScore = p(-6, -21);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = [
    p(10, 11),
    p(6, 13),
    p(11, 19),
    p(8, 8),
    p(-5, 16),
    p(-49, 7),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(36, 8), p(37, 37), p(49, -8), p(33, -33), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-51, -66),
        p(-30, -27),
        p(-17, -4),
        p(-7, 8),
        p(1, 18),
        p(8, 28),
        p(17, 30),
        p(25, 30),
        p(32, 27),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-29, -53),
        p(-17, -35),
        p(-7, -19),
        p(0, -6),
        p(6, 4),
        p(11, 12),
        p(16, 17),
        p(20, 21),
        p(22, 26),
        p(29, 26),
        p(35, 26),
        p(44, 27),
        p(41, 34),
        p(57, 26),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-76, 13),
        p(-67, 26),
        p(-63, 31),
        p(-60, 35),
        p(-60, 42),
        p(-54, 47),
        p(-51, 51),
        p(-47, 54),
        p(-43, 58),
        p(-40, 62),
        p(-36, 64),
        p(-35, 68),
        p(-27, 68),
        p(-17, 65),
        p(-16, 64),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-27, -49),
        p(-27, 8),
        p(-31, 55),
        p(-26, 71),
        p(-24, 89),
        p(-19, 94),
        p(-15, 105),
        p(-12, 112),
        p(-7, 116),
        p(-4, 117),
        p(-1, 120),
        p(3, 123),
        p(6, 123),
        p(7, 127),
        p(10, 128),
        p(14, 131),
        p(14, 137),
        p(18, 137),
        p(27, 134),
        p(41, 127),
        p(46, 127),
        p(91, 100),
        p(90, 104),
        p(116, 82),
        p(211, 47),
        p(256, 5),
        p(283, -5),
        p(348, -52),
    ],
    [
        p(-94, 7),
        p(-59, -6),
        p(-30, -6),
        p(0, -3),
        p(33, -2),
        p(56, -3),
        p(85, 3),
        p(111, 2),
        p(159, -16),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(24, 20),
        p(49, -12),
        p(20, -34),
        p(0, 0),
    ],
    [p(-2, 11), p(20, 23), p(0, 0), p(32, 3), p(31, 53), p(0, 0)],
    [p(-2, 13), p(11, 15), p(18, 12), p(0, 0), p(44, -5), p(0, 0)],
    [p(-2, 4), p(3, 6), p(0, 20), p(2, 1), p(0, 0), p(0, 0)],
    [
        p(71, 28),
        p(-35, 17),
        p(-9, 17),
        p(-21, 7),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 6), p(8, 5), p(6, 9), p(12, 5), p(7, 14), p(11, 5)],
    [
        p(-1, 1),
        p(9, 20),
        p(-118, -23),
        p(7, 13),
        p(8, 16),
        p(2, 6),
    ],
    [p(1, 2), p(12, 6), p(8, 11), p(9, 7), p(8, 19), p(18, -5)],
    [
        p(2, -2),
        p(8, -1),
        p(6, -7),
        p(4, 12),
        p(-67, -256),
        p(2, -11),
    ],
    [p(62, -1), p(40, 7), p(46, 1), p(24, 5), p(38, -13), p(0, 0)],
];

pub const NUM_DEFENDED: [PhasedScore; 17] = [
    p(32, -1),
    p(-14, -1),
    p(-22, -1),
    p(-18, -3),
    p(-12, -5),
    p(-12, -4),
    p(-14, -2),
    p(-10, -0),
    p(-6, 3),
    p(-2, 6),
    p(1, 14),
    p(6, 17),
    p(10, 20),
    p(13, 25),
    p(15, 13),
    p(13, -7),
    p(10, 247),
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-21, -17),
    p(19, -10),
    p(11, -4),
    p(14, -12),
    p(-2, 13),
    p(-12, 12),
];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(13, 20), p(34, -0), p(5, 32)];

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
