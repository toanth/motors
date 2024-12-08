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
        p( 131,  187),    p( 128,  185),    p( 119,  189),    p( 130,  169),    p( 117,  173),    p( 117,  177),    p(  79,  195),    p(  86,  193),
        p(  63,  123),    p(  60,  125),    p(  74,  120),    p(  82,  123),    p(  69,  123),    p( 118,  110),    p(  91,  131),    p(  89,  122),
        p(  49,  113),    p(  62,  109),    p(  60,  104),    p(  63,   98),    p(  78,   99),    p(  82,   94),    p(  75,  104),    p(  69,   95),
        p(  46,   99),    p(  53,  102),    p(  63,   94),    p(  72,   93),    p(  75,   93),    p(  75,   88),    p(  68,   93),    p(  57,   86),
        p(  42,   97),    p(  51,   93),    p(  55,   94),    p(  59,   99),    p(  66,   96),    p(  61,   92),    p(  68,   83),    p(  52,   85),
        p(  47,   99),    p(  51,   96),    p(  57,   97),    p(  57,  104),    p(  53,  107),    p(  70,   98),    p(  71,   84),    p(  53,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 178,  274),    p( 198,  307),    p( 215,  318),    p( 252,  307),    p( 283,  309),    p( 198,  304),    p( 213,  305),    p( 206,  257),
        p( 269,  307),    p( 284,  314),    p( 298,  307),    p( 302,  310),    p( 302,  305),    p( 314,  295),    p( 276,  310),    p( 271,  299),
        p( 286,  304),    p( 303,  303),    p( 304,  312),    p( 319,  315),    p( 336,  308),    p( 348,  298),    p( 291,  303),    p( 286,  304),
        p( 300,  313),    p( 307,  309),    p( 321,  315),    p( 324,  322),    p( 322,  319),    p( 317,  318),    p( 308,  312),    p( 317,  308),
        p( 297,  315),    p( 301,  306),    p( 309,  315),    p( 318,  317),    p( 316,  321),    p( 321,  304),    p( 320,  303),    p( 312,  311),
        p( 273,  302),    p( 279,  302),    p( 292,  298),    p( 298,  312),    p( 303,  309),    p( 292,  292),    p( 298,  293),    p( 292,  305),
        p( 269,  310),    p( 279,  312),    p( 281,  304),    p( 292,  309),    p( 296,  303),    p( 286,  300),    p( 293,  304),    p( 288,  319),
        p( 238,  307),    p( 279,  303),    p( 265,  304),    p( 285,  309),    p( 293,  307),    p( 291,  296),    p( 285,  306),    p( 263,  306),
    ],
    // bishop
    [
        p( 280,  314),    p( 257,  314),    p( 242,  307),    p( 221,  316),    p( 218,  315),    p( 225,  306),    p( 278,  305),    p( 253,  308),
        p( 285,  303),    p( 281,  305),    p( 290,  306),    p( 279,  307),    p( 287,  302),    p( 292,  301),    p( 268,  307),    p( 272,  304),
        p( 296,  310),    p( 307,  304),    p( 293,  309),    p( 307,  302),    p( 308,  304),    p( 335,  307),    p( 317,  302),    p( 315,  311),
        p( 283,  311),    p( 293,  310),    p( 303,  305),    p( 309,  311),    p( 307,  307),    p( 300,  309),    p( 295,  310),    p( 279,  311),
        p( 290,  309),    p( 282,  310),    p( 296,  307),    p( 310,  307),    p( 304,  305),    p( 298,  305),    p( 287,  308),    p( 308,  303),
        p( 294,  309),    p( 301,  308),    p( 300,  307),    p( 302,  306),    p( 307,  308),    p( 303,  303),    p( 306,  299),    p( 308,  301),
        p( 307,  312),    p( 303,  300),    p( 310,  302),    p( 298,  309),    p( 303,  308),    p( 304,  305),    p( 312,  299),    p( 306,  298),
        p( 296,  305),    p( 313,  308),    p( 307,  306),    p( 291,  312),    p( 304,  310),    p( 295,  313),    p( 306,  298),    p( 302,  296),
    ],
    // rook
    [
        p( 459,  546),    p( 448,  555),    p( 442,  562),    p( 439,  560),    p( 451,  556),    p( 471,  549),    p( 483,  547),    p( 493,  540),
        p( 443,  552),    p( 441,  557),    p( 450,  558),    p( 464,  549),    p( 451,  552),    p( 467,  546),    p( 476,  543),    p( 490,  534),
        p( 443,  547),    p( 461,  543),    p( 455,  544),    p( 455,  539),    p( 483,  529),    p( 491,  526),    p( 508,  526),    p( 484,  527),
        p( 439,  548),    p( 445,  544),    p( 445,  546),    p( 451,  540),    p( 455,  532),    p( 466,  528),    p( 465,  532),    p( 466,  527),
        p( 433,  546),    p( 432,  543),    p( 433,  544),    p( 439,  540),    p( 445,  536),    p( 439,  536),    p( 452,  530),    p( 446,  528),
        p( 429,  543),    p( 428,  540),    p( 431,  538),    p( 435,  538),    p( 440,  532),    p( 450,  524),    p( 466,  513),    p( 453,  517),
        p( 430,  538),    p( 434,  537),    p( 440,  538),    p( 443,  535),    p( 450,  528),    p( 462,  518),    p( 469,  514),    p( 440,  523),
        p( 440,  542),    p( 437,  537),    p( 439,  541),    p( 443,  535),    p( 448,  528),    p( 455,  528),    p( 451,  527),    p( 447,  530),
    ],
    // queen
    [
        p( 885,  951),    p( 890,  965),    p( 905,  977),    p( 925,  970),    p( 923,  974),    p( 941,  965),    p( 986,  918),    p( 932,  949),
        p( 895,  945),    p( 870,  976),    p( 875,  999),    p( 867, 1016),    p( 874, 1027),    p( 912,  992),    p( 912,  977),    p( 955,  954),
        p( 898,  953),    p( 892,  968),    p( 891,  989),    p( 894,  996),    p( 917,  998),    p( 951,  986),    p( 959,  957),    p( 948,  962),
        p( 885,  963),    p( 888,  974),    p( 885,  981),    p( 886,  993),    p( 888, 1006),    p( 900,  999),    p( 909, 1000),    p( 919,  972),
        p( 894,  956),    p( 880,  975),    p( 888,  974),    p( 888,  991),    p( 890,  988),    p( 892,  990),    p( 906,  978),    p( 914,  970),
        p( 889,  942),    p( 895,  957),    p( 890,  972),    p( 888,  974),    p( 894,  981),    p( 900,  972),    p( 914,  955),    p( 913,  943),
        p( 890,  943),    p( 888,  952),    p( 897,  951),    p( 896,  967),    p( 898,  966),    p( 898,  949),    p( 909,  928),    p( 918,  900),
        p( 878,  945),    p( 886,  935),    p( 888,  946),    p( 896,  948),    p( 899,  937),    p( 886,  942),    p( 887,  933),    p( 895,  915),
    ],
    // king
    [
        p( 154,  -84),    p(  57,  -38),    p(  81,  -30),    p(   6,    1),    p(  33,  -11),    p(  17,   -1),    p(  72,  -11),    p( 235,  -88),
        p( -30,    1),    p( -80,   19),    p( -82,   26),    p( -24,   17),    p( -55,   24),    p( -83,   39),    p( -50,   24),    p(   6,    1),
        p( -46,    9),    p( -49,   14),    p( -87,   28),    p( -97,   36),    p( -65,   31),    p( -32,   23),    p( -77,   25),    p( -36,   11),
        p( -27,    2),    p(-100,   12),    p(-114,   28),    p(-136,   37),    p(-136,   35),    p(-113,   27),    p(-133,   17),    p(-102,   17),
        p( -41,   -2),    p(-116,    8),    p(-126,   25),    p(-151,   38),    p(-153,   36),    p(-128,   22),    p(-143,   12),    p(-115,   13),
        p( -33,    2),    p( -92,    3),    p(-120,   18),    p(-126,   27),    p(-123,   26),    p(-133,   18),    p(-110,    4),    p( -72,   10),
        p(  26,   -8),    p( -79,   -2),    p( -90,    7),    p(-109,   16),    p(-115,   17),    p(-100,    8),    p( -73,   -9),    p(   4,   -4),
        p(  53,  -24),    p(  42,  -36),    p(  39,  -24),    p( -21,   -3),    p(  31,  -20),    p( -18,   -7),    p(  35,  -30),    p(  66,  -34),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 53);
const ROOK_OPEN_FILE: PhasedScore = p(14, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(2, 4);
const QUEEN_OPEN_FILE: PhasedScore = p(18, -12);
const QUEEN_CLOSED_FILE: PhasedScore = p(2, 10);
const QUEEN_SEMIOPEN_FILE: PhasedScore = p(2, 8);
const KING_OPEN_FILE: PhasedScore = p(-48, -2);
const KING_CLOSED_FILE: PhasedScore = p(14, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 5), p(-1, 7), p(-0, 8), p(3, 6), p(3, 8), p(4, 10), p(7, 9), p(18, 5)],
    // Closed
    [p(0, 0), p(0, 0), p(14, -22), p(-16, 10), p(-1, 12), p(1, 4), p(0, 8), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-17, 24), p(3, 21), p(1, 12), p(-0, 14), p(4, 9), p(1, 7), p(10, 9)],
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
    p(-4, 10),   /*0b1011*/
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
    p(17, 10),   /*0b11100*/
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
    p(-3, 7),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(17, 18),   /*0b110000*/
    p(25, 13),   /*0b110001*/
    p(33, 9),    /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(11, 29),   /*0b110100*/
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
    p(7, -21),   /*0b101*/
    p(70, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(64, -13),  /*0b1000*/
    p(21, -33),  /*0b1001*/
    p(83, -55),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(60, -20),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(19, -8),   /*0b1111*/
    p(21, -3),   /*0b00*/
    p(33, -13),  /*0b01*/
    p(27, -18),  /*0b10*/
    p(21, -42),  /*0b11*/
    p(38, -11),  /*0b100*/
    p(55, -21),  /*0b101*/
    p(25, -23),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(40, -4),   /*0b1000*/
    p(51, -17),  /*0b1001*/
    p(53, -43),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(45, -24),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(19, -43),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  31,   87),    p(  28,   85),    p(  19,   89),    p(  30,   69),    p(  17,   73),    p(  17,   77),    p( -21,   95),    p( -14,   93),
        p(  40,  124),    p(  48,  123),    p(  36,  100),    p(  20,   69),    p(  35,   69),    p(  14,   95),    p(  -1,  105),    p( -31,  125),
        p(  24,   74),    p(  17,   71),    p(  22,   54),    p(  16,   44),    p(  -1,   46),    p(   7,   59),    p( -10,   76),    p( -10,   79),
        p(   8,   46),    p(  -2,   44),    p( -15,   35),    p(  -9,   25),    p( -17,   29),    p( -10,   39),    p( -17,   55),    p( -10,   51),
        p(   2,   15),    p( -12,   24),    p( -15,   17),    p( -16,    9),    p( -15,   14),    p(  -7,   18),    p( -13,   37),    p(  10,   17),
        p(  -4,   15),    p(  -2,   20),    p(  -9,   17),    p(  -8,    5),    p(   5,    2),    p(   7,    8),    p(  12,   19),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-11, -10);
const DOUBLED_PAWN: PhasedScore = p(-7, -21);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = [
    p(11, 11),
    p(6, 13),
    p(12, 19),
    p(8, 8),
    p(-5, 13),
    p(-49, 7),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(36, 8), p(37, 37), p(49, -7), p(31, -32), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-50, -66),
        p(-30, -27),
        p(-17, -4),
        p(-7, 8),
        p(1, 18),
        p(9, 28),
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
        p(7, 4),
        p(11, 12),
        p(16, 17),
        p(20, 21),
        p(22, 26),
        p(30, 26),
        p(36, 26),
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
        p(-75, 12),
        p(-67, 26),
        p(-62, 31),
        p(-59, 35),
        p(-60, 42),
        p(-54, 47),
        p(-51, 51),
        p(-47, 54),
        p(-43, 58),
        p(-40, 61),
        p(-36, 64),
        p(-35, 68),
        p(-27, 68),
        p(-17, 65),
        p(-15, 64),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-23, -45),
        p(-24, 12),
        p(-27, 59),
        p(-23, 76),
        p(-20, 93),
        p(-15, 98),
        p(-11, 109),
        p(-8, 115),
        p(-3, 119),
        p(-0, 121),
        p(3, 123),
        p(8, 125),
        p(11, 125),
        p(13, 128),
        p(17, 128),
        p(21, 130),
        p(23, 135),
        p(27, 133),
        p(37, 129),
        p(52, 121),
        p(58, 119),
        p(105, 92),
        p(105, 95),
        p(131, 73),
        p(226, 38),
        p(271, -5),
        p(297, -13),
        p(362, -60),
    ],
    [
        p(-94, 7),
        p(-59, -6),
        p(-30, -6),
        p(0, -3),
        p(32, -2),
        p(56, -3),
        p(84, 3),
        p(111, 2),
        p(158, -16),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(20, -33),
        p(0, 0),
    ],
    [p(-2, 11), p(20, 23), p(0, 0), p(32, 3), p(34, 48), p(0, 0)],
    [p(-2, 12), p(10, 15), p(17, 12), p(0, 0), p(39, 2), p(0, 0)],
    [p(-3, 4), p(3, 5), p(3, 16), p(1, 1), p(0, 0), p(0, 0)],
    [
        p(71, 28),
        p(-35, 17),
        p(-8, 17),
        p(-22, 7),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 6), p(8, 5), p(5, 9), p(12, 5), p(6, 14), p(11, 5)],
    [
        p(-1, 1),
        p(9, 20),
        p(-117, -23),
        p(7, 13),
        p(7, 16),
        p(2, 6),
    ],
    [p(1, 2), p(12, 6), p(8, 11), p(9, 7), p(9, 18), p(18, -5)],
    [
        p(1, -1),
        p(8, -1),
        p(6, -7),
        p(4, 12),
        p(-62, -262),
        p(2, -12),
    ],
    [p(62, -1), p(40, 7), p(45, 1), p(24, 5), p(38, -13), p(0, 0)],
];

pub const NUM_DEFENDED: [PhasedScore; 17] = [
    p(30, -1),
    p(-15, -0),
    p(-22, -1),
    p(-19, -3),
    p(-13, -5),
    p(-13, -4),
    p(-14, -1),
    p(-10, -0),
    p(-6, 3),
    p(-2, 6),
    p(2, 13),
    p(6, 16),
    p(11, 19),
    p(13, 23),
    p(15, 9),
    p(14, -11),
    p(11, 254),
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-21, -17),
    p(19, -10),
    p(11, -4),
    p(14, -12),
    p(-2, 13),
    p(-12, 12),
];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(13, 20), p(34, -0), p(6, 31)];

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

    fn queen_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score>;

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

    fn queen_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score> {
        match openness {
            FileOpenness::Open => QUEEN_OPEN_FILE,
            FileOpenness::Closed => QUEEN_CLOSED_FILE,
            FileOpenness::SemiOpen => QUEEN_SEMIOPEN_FILE,
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
