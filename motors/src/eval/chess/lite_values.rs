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
        p( 133,  186),    p( 130,  185),    p( 120,  189),    p( 133,  169),    p( 120,  174),    p( 120,  177),    p(  83,  195),    p(  92,  192),
        p(  66,  123),    p(  63,  124),    p(  74,  120),    p(  82,  124),    p(  67,  125),    p( 118,  110),    p(  92,  132),    p(  89,  122),
        p(  51,  113),    p(  63,  109),    p(  61,  103),    p(  66,   97),    p(  82,   98),    p(  83,   94),    p(  77,  103),    p(  71,   96),
        p(  48,  100),    p(  55,  102),    p(  64,   95),    p(  73,   94),    p(  76,   92),    p(  77,   88),    p(  71,   92),    p(  59,   86),
        p(  43,   97),    p(  51,   94),    p(  55,   94),    p(  59,   99),    p(  67,   97),    p(  62,   93),    p(  70,   84),    p(  54,   85),
        p(  50,   98),    p(  51,   97),    p(  58,   98),    p(  58,  105),    p(  54,  108),    p(  73,   98),    p(  73,   84),    p(  55,   87),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 188,  269),    p( 211,  301),    p( 236,  313),    p( 255,  305),    p( 279,  309),    p( 195,  303),    p( 217,  302),    p( 207,  253),
        p( 280,  302),    p( 289,  310),    p( 293,  307),    p( 302,  310),    p( 291,  307),    p( 307,  298),    p( 263,  310),    p( 279,  295),
        p( 296,  300),    p( 304,  302),    p( 311,  312),    p( 308,  315),    p( 311,  314),    p( 333,  304),    p( 285,  306),    p( 285,  303),
        p( 310,  307),    p( 309,  306),    p( 311,  316),    p( 318,  321),    p( 308,  319),    p( 322,  316),    p( 305,  308),    p( 321,  303),
        p( 306,  310),    p( 303,  303),    p( 300,  315),    p( 305,  318),    p( 312,  319),    p( 309,  306),    p( 320,  300),    p( 310,  305),
        p( 283,  298),    p( 282,  299),    p( 289,  298),    p( 294,  311),    p( 302,  308),    p( 288,  292),    p( 302,  290),    p( 298,  300),
        p( 281,  304),    p( 290,  306),    p( 286,  300),    p( 297,  305),    p( 301,  299),    p( 293,  296),    p( 303,  297),    p( 299,  312),
        p( 256,  299),    p( 295,  297),    p( 279,  299),    p( 298,  304),    p( 308,  301),    p( 304,  290),    p( 302,  297),    p( 280,  297),
    ],
    // bishop
    [
        p( 280,  315),    p( 256,  314),    p( 249,  307),    p( 225,  315),    p( 223,  315),    p( 226,  306),    p( 283,  304),    p( 253,  308),
        p( 280,  303),    p( 285,  306),    p( 288,  306),    p( 282,  308),    p( 284,  303),    p( 294,  303),    p( 272,  308),    p( 274,  304),
        p( 298,  309),    p( 303,  305),    p( 296,  311),    p( 302,  303),    p( 307,  306),    p( 333,  309),    p( 319,  303),    p( 311,  311),
        p( 281,  310),    p( 298,  310),    p( 301,  306),    p( 317,  311),    p( 311,  307),    p( 306,  309),    p( 302,  308),    p( 283,  311),
        p( 292,  308),    p( 281,  312),    p( 300,  309),    p( 315,  308),    p( 313,  308),    p( 298,  307),    p( 291,  309),    p( 311,  300),
        p( 293,  307),    p( 304,  310),    p( 301,  310),    p( 304,  310),    p( 308,  311),    p( 304,  307),    p( 306,  302),    p( 310,  300),
        p( 309,  311),    p( 303,  300),    p( 311,  303),    p( 296,  310),    p( 302,  308),    p( 303,  305),    p( 313,  301),    p( 303,  298),
        p( 294,  304),    p( 314,  309),    p( 306,  307),    p( 290,  312),    p( 303,  309),    p( 296,  313),    p( 303,  298),    p( 301,  295),
    ],
    // rook
    [
        p( 458,  550),    p( 450,  559),    p( 447,  565),    p( 446,  562),    p( 457,  558),    p( 477,  553),    p( 486,  552),    p( 496,  545),
        p( 432,  556),    p( 429,  562),    p( 438,  562),    p( 454,  552),    p( 444,  554),    p( 464,  549),    p( 476,  546),    p( 490,  537),
        p( 437,  553),    p( 456,  548),    p( 454,  550),    p( 457,  545),    p( 484,  534),    p( 493,  530),    p( 516,  527),    p( 488,  530),
        p( 435,  552),    p( 442,  548),    p( 443,  551),    p( 448,  546),    p( 457,  538),    p( 466,  532),    p( 473,  534),    p( 469,  530),
        p( 430,  548),    p( 430,  547),    p( 431,  548),    p( 437,  545),    p( 444,  541),    p( 438,  540),    p( 458,  533),    p( 447,  531),
        p( 427,  544),    p( 426,  542),    p( 430,  541),    p( 432,  542),    p( 439,  536),    p( 447,  528),    p( 470,  515),    p( 452,  520),
        p( 430,  539),    p( 434,  539),    p( 440,  540),    p( 442,  538),    p( 450,  532),    p( 464,  521),    p( 473,  516),    p( 441,  525),
        p( 439,  543),    p( 435,  539),    p( 437,  544),    p( 442,  539),    p( 449,  533),    p( 455,  532),    p( 453,  529),    p( 447,  530),
    ],
    // queen
    [
        p( 874,  969),    p( 877,  982),    p( 892,  996),    p( 908,  992),    p( 907,  995),    p( 927,  983),    p( 978,  931),    p( 922,  963),
        p( 883,  961),    p( 859,  993),    p( 861, 1020),    p( 853, 1037),    p( 861, 1047),    p( 900, 1009),    p( 904,  989),    p( 946,  967),
        p( 891,  966),    p( 883,  986),    p( 883, 1008),    p( 881, 1017),    p( 904, 1018),    p( 944, 1001),    p( 951,  971),    p( 938,  977),
        p( 876,  980),    p( 883,  991),    p( 876, 1000),    p( 875, 1014),    p( 880, 1024),    p( 893, 1014),    p( 902, 1013),    p( 909,  989),
        p( 888,  970),    p( 874,  989),    p( 880,  992),    p( 880, 1010),    p( 881, 1008),    p( 884, 1007),    p( 898,  992),    p( 905,  984),
        p( 883,  955),    p( 889,  973),    p( 882,  989),    p( 879,  992),    p( 884,  999),    p( 891,  989),    p( 905,  971),    p( 905,  958),
        p( 886,  953),    p( 883,  963),    p( 890,  966),    p( 889,  980),    p( 890,  979),    p( 892,  963),    p( 902,  941),    p( 912,  912),
        p( 872,  951),    p( 883,  941),    p( 883,  956),    p( 891,  957),    p( 894,  950),    p( 882,  951),    p( 883,  939),    p( 887,  926),
    ],
    // king
    [
        p( 152, -102),    p(  60,  -49),    p(  84,  -41),    p(   8,   -9),    p(  30,  -22),    p(  13,  -12),    p(  67,  -21),    p( 225, -107),
        p( -20,   -3),    p( -61,   26),    p( -69,   36),    p(  -4,   25),    p( -37,   34),    p( -64,   47),    p( -29,   32),    p(  14,   -2),
        p( -40,    5),    p( -30,   23),    p( -73,   40),    p( -79,   48),    p( -46,   42),    p( -13,   35),    p( -50,   33),    p( -26,   10),
        p( -23,   -1),    p( -83,   22),    p( -98,   39),    p(-120,   48),    p(-118,   46),    p( -99,   38),    p(-105,   27),    p( -95,   14),
        p( -41,   -4),    p(-104,   17),    p(-112,   33),    p(-135,   47),    p(-140,   45),    p(-118,   30),    p(-132,   22),    p(-113,   11),
        p( -35,   -0),    p( -80,   12),    p(-107,   26),    p(-115,   35),    p(-109,   34),    p(-124,   26),    p( -97,   12),    p( -70,    8),
        p(  28,   -9),    p( -64,    7),    p( -76,   15),    p( -95,   24),    p(-100,   25),    p( -87,   16),    p( -56,    0),    p(   7,   -5),
        p(  42,  -41),    p(  39,  -46),    p(  33,  -34),    p( -27,  -13),    p(  26,  -31),    p( -23,  -17),    p(  32,  -41),    p(  59,  -51),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  33,   86),    p(  30,   85),    p(  20,   89),    p(  33,   69),    p(  20,   74),    p(  20,   77),    p( -17,   95),    p(  -8,   92),
        p(  41,  123),    p(  48,  122),    p(  37,   98),    p(  21,   67),    p(  37,   66),    p(  15,   94),    p(   1,  102),    p( -27,  123),
        p(  24,   72),    p(  18,   70),    p(  24,   53),    p(  16,   43),    p(  -1,   44),    p(   7,   57),    p( -11,   74),    p( -10,   77),
        p(   8,   45),    p(  -3,   43),    p( -15,   34),    p(  -9,   24),    p( -17,   29),    p( -11,   38),    p( -19,   54),    p( -11,   50),
        p(   2,   14),    p( -12,   22),    p( -14,   16),    p( -16,    9),    p( -14,   13),    p(  -8,   17),    p( -14,   36),    p(   9,   16),
        p(  -4,   15),    p(  -2,   20),    p(  -9,   16),    p(  -8,    5),    p(   6,    1),    p(   7,    7),    p(  12,   18),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-11, -10);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);

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
    [p(-7, 5), p(-2, 7), p(-2, 9), p(3, 7), p(4, 9), p(5, 12), p(10, 11), p(21, 7), ],
    // Closed
    [p(0, 0), p(0, 0), p(13, -31), p(-15, 9), p(1, 13), p(3, 4), p(2, 10), p(-0, 6), ],
    // SemiOpen
    [p(0, 0), p(-16, 22), p(2, 20), p(1, 15), p(-1, 19), p(4, 14), p(2, 11), p(12, 11), ],
    // SemiClosed
    [p(0, 0), p(11, -13), p(8, 6), p(5, 1), p(9, 4), p(4, 4), p(8, 7), p(3, 4), ],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-6, 7),    /*0b0000*/
    p(-16, 12),  /*0b0001*/
    p(-2, 8),    /*0b0010*/
    p(-10, 15),  /*0b0011*/
    p(-5, 7),    /*0b0100*/
    p(-27, 5),   /*0b0101*/
    p(-14, 7),   /*0b0110*/
    p(-19, -16), /*0b0111*/
    p(7, 10),    /*0b1000*/
    p(-5, 11),   /*0b1001*/
    p(3, 9),     /*0b1010*/
    p(-3, 12),   /*0b1011*/
    p(-2, 7),    /*0b1100*/
    p(-26, 10),  /*0b1101*/
    p(-12, 5),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(2, 18),    /*0b10000*/
    p(4, 12),    /*0b10001*/
    p(22, 14),   /*0b10010*/
    p(-3, 10),   /*0b10011*/
    p(-5, 9),    /*0b10100*/
    p(13, 18),   /*0b10101*/
    p(-22, 4),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(13, 33),   /*0b11000*/
    p(31, 26),   /*0b11001*/
    p(42, 39),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(16, 13),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(15, 10),   /*0b100000*/
    p(3, 15),    /*0b100001*/
    p(26, 4),    /*0b100010*/
    p(6, 2),     /*0b100011*/
    p(-10, 4),   /*0b100100*/
    p(-24, -7),  /*0b100101*/
    p(-25, 18),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(20, 2),    /*0b101000*/
    p(-3, 18),   /*0b101001*/
    p(21, -3),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-7, 7),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(14, 21),   /*0b110000*/
    p(25, 18),   /*0b110001*/
    p(33, 12),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(8, 32),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(24, 15),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(5, -0),    /*0b111111*/
    p(-21, -9),  /*0b00*/
    p(9, -25),   /*0b01*/
    p(35, -14),  /*0b10*/
    p(23, -50),  /*0b11*/
    p(44, -17),  /*0b100*/
    p(-2, -28),  /*0b101*/
    p(72, -48),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(55, -20),  /*0b1000*/
    p(19, -44),  /*0b1001*/
    p(77, -63),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(53, -25),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(11, -4),   /*0b1111*/
    p(15, -10),  /*0b00*/
    p(30, -20),  /*0b01*/
    p(24, -27),  /*0b10*/
    p(22, -53),  /*0b11*/
    p(30, -18),  /*0b100*/
    p(52, -28),  /*0b101*/
    p(21, -33),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(35, -11),  /*0b1000*/
    p(52, -26),  /*0b1001*/
    p(50, -52),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(39, -30),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(21, -54),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 7), p(3, 8), p(10, 14), p(9, 9), p(-5, 19), p(-46, 9)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(38, 9), p(42, 36), p(51, -9), p(37, -39), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-54, -65),
        p(-31, -27),
        p(-15, -5),
        p(-3, 7),
        p(8, 16),
        p(18, 25),
        p(29, 26),
        p(38, 25),
        p(46, 21),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-25, -48),
        p(-14, -30),
        p(-4, -15),
        p(3, -4),
        p(9, 6),
        p(13, 14),
        p(16, 18),
        p(18, 21),
        p(19, 26),
        p(25, 26),
        p(29, 24),
        p(37, 25),
        p(31, 33),
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
        p(-75, 14),
        p(-66, 28),
        p(-61, 32),
        p(-58, 37),
        p(-58, 43),
        p(-53, 48),
        p(-50, 52),
        p(-46, 54),
        p(-42, 58),
        p(-38, 61),
        p(-33, 63),
        p(-29, 67),
        p(-20, 67),
        p(-8, 64),
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
        p(-34, -37),
        p(-35, 21),
        p(-38, 69),
        p(-33, 87),
        p(-30, 104),
        p(-25, 109),
        p(-21, 119),
        p(-17, 126),
        p(-13, 129),
        p(-10, 132),
        p(-7, 135),
        p(-3, 138),
        p(-0, 139),
        p(1, 144),
        p(4, 145),
        p(7, 147),
        p(8, 153),
        p(11, 152),
        p(20, 149),
        p(35, 141),
        p(40, 141),
        p(83, 116),
        p(83, 118),
        p(107, 98),
        p(199, 62),
        p(251, 18),
        p(287, 2),
        p(336, -32),
    ],
    [
        p(-83, 51),
        p(-51, 23),
        p(-25, 11),
        p(2, 4),
        p(29, -3),
        p(47, -10),
        p(68, -10),
        p(87, -17),
        p(133, -42),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(50, -15),
        p(21, -44),
        p(0, 0),
    ],
    [p(-1, 13), p(19, 21), p(-2, 9), p(28, 2), p(27, 55), p(0, 0)],
    [
        p(3, 17),
        p(21, 21),
        p(23, 21),
        p(-5, 11),
        p(43, -5),
        p(0, 0),
    ],
    [p(-0, -2), p(7, 13), p(-0, 30), p(0, 6), p(2, -17), p(0, 0)],
    [p(78, 34), p(19, 20), p(2, 19), p(-33, 10), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(11, 5), p(9, 10), p(15, 5), p(9, 17), p(13, 3)],
    [p(-3, 1), p(8, 18), p(-96, -36), p(6, 12), p(7, 16), p(4, 5)],
    [p(3, 2), p(14, 4), p(9, 11), p(11, 7), p(12, 15), p(22, -5)],
    [
        p(3, -5),
        p(10, -2),
        p(8, -8),
        p(4, 15),
        p(-58, -259),
        p(7, -11),
    ],
    [
        p(60, -8),
        p(38, -1),
        p(43, -6),
        p(21, -3),
        p(33, -19),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-13, -10),
    p(-8, -10),
    p(17, -3),
    p(23, -13),
    p(5, 23),
    p(7, 18),
];
const KNIGHT_DISTANCE: [PhasedScore; 8] = [
    p(0, 0),
    p(86, 6),
    p(54, -5),
    p(29, 12),
    p(2, 9),
    p(-9, 7),
    p(-13, 7),
    p(-17, 8),
];

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

    fn unsupported_pawn() -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn doubled_pawn() -> <Self::Score as ScoreType>::SingleFeatureScore;

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

    fn knight_distance(distance: usize) -> <Self::Score as ScoreType>::SingleFeatureScore;
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

    fn unsupported_pawn() -> <Self::Score as ScoreType>::SingleFeatureScore {
        UNSUPPORTED_PAWN
    }

    fn doubled_pawn() -> <Self::Score as ScoreType>::SingleFeatureScore {
        DOUBLED_PAWN
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

    fn knight_distance(distance: usize) -> <Self::Score as ScoreType>::SingleFeatureScore {
        KNIGHT_DISTANCE[distance]
    }
}
