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
use gears::games::chess::ChessColor;
use gears::games::chess::ChessColor::White;
use gears::games::chess::pieces::{ChessPieceType, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::general::common::StaticallyNamedEntity;
use gears::score::{PhasedScore, p};
use std::fmt::{Debug, Display};

#[rustfmt::skip]
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),
        p( 117,  170),    p( 113,  168),    p( 104,  172),    p( 116,  152),    p( 103,  157),    p( 102,  160),    p(  65,  178),    p(  72,  176),
        p(  67,  123),    p(  66,  124),    p(  77,  120),    p(  85,  123),    p(  73,  123),    p( 121,  110),    p(  96,  130),    p(  93,  121),
        p(  54,  112),    p(  66,  108),    p(  64,  104),    p(  67,   98),    p(  82,   98),    p(  87,   94),    p(  79,  103),    p(  74,   95),
        p(  51,   99),    p(  58,  102),    p(  67,   94),    p(  76,   93),    p(  79,   93),    p(  78,   88),    p(  73,   92),    p(  62,   86),
        p(  46,   97),    p(  55,   92),    p(  59,   93),    p(  61,   99),    p(  69,   96),    p(  63,   92),    p(  72,   82),    p(  55,   85),
        p(  52,   99),    p(  55,   95),    p(  61,   97),    p(  60,  105),    p(  57,  107),    p(  73,   98),    p(  75,   83),    p(  58,   88),
        p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),
    ],
    // knight
    [
        p( 119,  222),    p( 140,  254),    p( 157,  266),    p( 195,  255),    p( 225,  257),    p( 141,  252),    p( 155,  253),    p( 147,  205),
        p( 211,  255),    p( 226,  260),    p( 241,  252),    p( 245,  256),    p( 244,  251),    p( 257,  241),    p( 218,  257),    p( 214,  247),
        p( 229,  251),    p( 246,  248),    p( 248,  254),    p( 262,  257),    p( 279,  251),    p( 291,  241),    p( 233,  247),    p( 228,  252),
        p( 243,  259),    p( 250,  253),    p( 265,  257),    p( 268,  264),    p( 265,  262),    p( 261,  261),    p( 252,  256),    p( 261,  255),
        p( 241,  261),    p( 245,  251),    p( 254,  257),    p( 262,  260),    p( 260,  263),    p( 265,  247),    p( 263,  247),    p( 255,  257),
        p( 217,  248),    p( 223,  246),    p( 236,  241),    p( 242,  254),    p( 246,  252),    p( 235,  234),    p( 242,  237),    p( 235,  251),
        p( 212,  256),    p( 223,  258),    p( 225,  247),    p( 235,  251),    p( 240,  246),    p( 230,  244),    p( 237,  250),    p( 231,  265),
        p( 181,  255),    p( 224,  248),    p( 208,  250),    p( 229,  254),    p( 238,  251),    p( 234,  241),    p( 231,  250),    p( 208,  253),
    ],
    // bishop
    [
        p( 192,  228),    p( 168,  230),    p( 153,  221),    p( 138,  231),    p( 133,  227),    p( 137,  223),    p( 189,  220),    p( 167,  227),
        p( 196,  220),    p( 194,  220),    p( 205,  221),    p( 191,  216),    p( 202,  214),    p( 207,  214),    p( 184,  225),    p( 185,  218),
        p( 209,  224),    p( 221,  220),    p( 207,  220),    p( 220,  213),    p( 219,  214),    p( 252,  220),    p( 232,  216),    p( 230,  228),
        p( 200,  227),    p( 205,  220),    p( 216,  217),    p( 223,  222),    p( 223,  220),    p( 213,  219),    p( 211,  222),    p( 194,  224),
        p( 205,  222),    p( 197,  223),    p( 209,  218),    p( 225,  222),    p( 218,  217),    p( 212,  217),    p( 199,  217),    p( 223,  216),
        p( 209,  226),    p( 215,  220),    p( 216,  223),    p( 214,  219),    p( 220,  221),    p( 215,  214),    p( 220,  211),    p( 220,  214),
        p( 222,  226),    p( 219,  218),    p( 224,  216),    p( 212,  223),    p( 215,  218),    p( 219,  219),    p( 228,  213),    p( 223,  213),
        p( 213,  223),    p( 226,  223),    p( 221,  222),    p( 206,  224),    p( 221,  223),    p( 208,  224),    p( 221,  213),    p( 218,  210),
    ],
    // rook
    [
        p( 374,  453),    p( 363,  463),    p( 357,  470),    p( 355,  467),    p( 366,  463),    p( 386,  457),    p( 398,  455),    p( 407,  449),
        p( 359,  459),    p( 357,  465),    p( 366,  465),    p( 381,  456),    p( 367,  459),    p( 384,  453),    p( 392,  451),    p( 407,  441),
        p( 361,  454),    p( 378,  449),    p( 373,  451),    p( 372,  446),    p( 399,  436),    p( 408,  433),    p( 425,  432),    p( 401,  434),
        p( 357,  454),    p( 363,  450),    p( 362,  453),    p( 368,  446),    p( 372,  438),    p( 383,  434),    p( 383,  438),    p( 383,  433),
        p( 351,  452),    p( 350,  450),    p( 350,  451),    p( 355,  447),    p( 361,  443),    p( 356,  442),    p( 369,  436),    p( 364,  434),
        p( 346,  449),    p( 346,  446),    p( 348,  445),    p( 351,  445),    p( 356,  439),    p( 367,  431),    p( 383,  419),    p( 370,  424),
        p( 348,  444),    p( 352,  443),    p( 358,  444),    p( 360,  442),    p( 367,  435),    p( 380,  425),    p( 387,  420),    p( 359,  429),
        p( 358,  448),    p( 354,  443),    p( 355,  447),    p( 360,  441),    p( 365,  435),    p( 371,  435),    p( 368,  433),    p( 364,  435),
    ],
    // queen
    [
        p( 681,  769),    p( 684,  783),    p( 698,  796),    p( 719,  790),    p( 716,  794),    p( 736,  782),    p( 782,  734),    p( 727,  766),
        p( 691,  760),    p( 666,  789),    p( 668,  816),    p( 660,  833),    p( 668,  844),    p( 709,  804),    p( 709,  789),    p( 750,  769),
        p( 695,  766),    p( 688,  781),    p( 688,  802),    p( 688,  811),    p( 712,  812),    p( 749,  797),    p( 757,  767),    p( 744,  775),
        p( 682,  778),    p( 688,  785),    p( 682,  795),    p( 683,  806),    p( 686,  818),    p( 699,  809),    p( 707,  810),    p( 715,  786),
        p( 692,  768),    p( 680,  788),    p( 686,  788),    p( 687,  805),    p( 689,  801),    p( 691,  802),    p( 704,  790),    p( 710,  784),
        p( 688,  758),    p( 695,  774),    p( 689,  788),    p( 687,  791),    p( 691,  798),    p( 698,  787),    p( 712,  771),    p( 710,  759),
        p( 689,  759),    p( 689,  768),    p( 696,  771),    p( 695,  785),    p( 696,  783),    p( 697,  767),    p( 708,  745),    p( 717,  717),
        p( 675,  761),    p( 687,  749),    p( 688,  762),    p( 696,  763),    p( 698,  752),    p( 685,  757),    p( 687,  748),    p( 692,  732),
    ],
    // king
    [
        p( 156,  -84),    p(  59,  -36),    p(  83,  -29),    p(   7,    3),    p(  36,  -10),    p(  22,    0),    p(  74,   -9),    p( 235,  -89),
        p( -30,    1),    p( -80,   20),    p( -82,   27),    p( -21,   17),    p( -51,   24),    p( -81,   39),    p( -50,   25),    p(   9,   -1),
        p( -46,    9),    p( -47,   14),    p( -85,   29),    p( -95,   37),    p( -64,   32),    p( -32,   24),    p( -78,   26),    p( -36,   10),
        p( -26,    1),    p(-101,   14),    p(-114,   30),    p(-136,   38),    p(-136,   36),    p(-115,   28),    p(-134,   18),    p(-106,   16),
        p( -42,   -2),    p(-115,    9),    p(-126,   26),    p(-151,   39),    p(-154,   37),    p(-128,   23),    p(-144,   13),    p(-118,   12),
        p( -33,    1),    p( -91,    4),    p(-119,   19),    p(-126,   28),    p(-123,   27),    p(-134,   19),    p(-109,    5),    p( -73,    9),
        p(  25,   -8),    p( -77,   -1),    p( -90,    8),    p(-109,   17),    p(-114,   18),    p( -99,    9),    p( -72,   -9),    p(   4,   -5),
        p(  54,  -24),    p(  42,  -35),    p(  38,  -21),    p( -24,   -1),    p(  28,  -18),    p( -19,   -5),    p(  34,  -29),    p(  66,  -35),
    ],
];

const BISHOP_PAIR: PhasedScore = p(22, 53);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(36, 44), p(38, 43), p(38, 31), p(35, 24), p(31, 15), p(27, 6), p(20, -3), p(12, -17), p(-0, -27)];
const ROOK_OPEN_FILE: PhasedScore = p(13, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, 0);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 4);
const KING_OPEN_FILE: PhasedScore = p(-49, -1);
const KING_CLOSED_FILE: PhasedScore = p(14, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(11, 21), p(16, 22), p(15, 22), p(18, 21), p(18, 24), p(21, 26), p(22, 23), p(34, 17)],
    // Closed
    [p(0, 0), p(0, 0), p(33, -6), p(1, 27), p(15, 30), p(20, 23), p(16, 26), p(15, 22)],
    // SemiOpen
    [p(0, 0), p(1, 39), p(20, 35), p(17, 27), p(16, 29), p(22, 24), p(16, 21), p(26, 21)],
    // SemiClosed
    [p(0, 0), p(28, 4), p(24, 23), p(20, 18), p(23, 21), p(21, 23), p(21, 24), p(17, 21)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-4, 5),    /*0b0000*/
    p(-14, 7),   /*0b0001*/
    p(-3, 7),    /*0b0010*/
    p(-10, 12),  /*0b0011*/
    p(-3, 2),    /*0b0100*/
    p(-26, -1),  /*0b0101*/
    p(-14, 4),   /*0b0110*/
    p(-20, -16), /*0b0111*/
    p(10, 10),   /*0b1000*/
    p(-2, 10),   /*0b1001*/
    p(3, 10),    /*0b1010*/
    p(-3, 10),   /*0b1011*/
    p(0, 4),     /*0b1100*/
    p(-23, 9),   /*0b1101*/
    p(-12, 4),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(4, 14),    /*0b10000*/
    p(3, 7),     /*0b10001*/
    p(20, 10),   /*0b10010*/
    p(-6, 6),    /*0b10011*/
    p(-5, 5),    /*0b10100*/
    p(13, 14),   /*0b10101*/
    p(-24, 1),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(16, 28),   /*0b11000*/
    p(30, 22),   /*0b11001*/
    p(43, 37),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(18, 9),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(17, 9),    /*0b100000*/
    p(3, 12),    /*0b100001*/
    p(26, 2),    /*0b100010*/
    p(6, -2),    /*0b100011*/
    p(-6, 1),    /*0b100100*/
    p(-21, -8),  /*0b100101*/
    p(-22, 15),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(24, 3),    /*0b101000*/
    p(0, 16),    /*0b101001*/
    p(23, -3),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-4, 6),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(16, 17),   /*0b110000*/
    p(25, 12),   /*0b110001*/
    p(33, 9),    /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(11, 28),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(27, 14),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(8, -2),    /*0b111111*/
    p(-14, -3),  /*0b00*/
    p(10, -17),  /*0b01*/
    p(38, -8),   /*0b10*/
    p(21, -40),  /*0b11*/
    p(46, -10),  /*0b100*/
    p(6, -20),   /*0b101*/
    p(69, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(64, -12),  /*0b1000*/
    p(20, -33),  /*0b1001*/
    p(80, -55),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(60, -19),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(22, -10),  /*0b1111*/
    p(20, -1),   /*0b00*/
    p(32, -12),  /*0b01*/
    p(26, -17),  /*0b10*/
    p(21, -40),  /*0b11*/
    p(36, -9),   /*0b100*/
    p(55, -19),  /*0b101*/
    p(24, -22),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(38, -2),   /*0b1000*/
    p(52, -17),  /*0b1001*/
    p(50, -41),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(43, -21),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(20, -43),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  50,  103),    p(  46,  101),    p(  37,  105),    p(  49,   85),    p(  36,   89),    p(  35,   93),    p(  -2,  111),    p(   5,  109),
        p(  39,  123),    p(  47,  123),    p(  37,  100),    p(  20,   69),    p(  34,   69),    p(  15,   95),    p(  -1,  104),    p( -32,  125),
        p(  23,   74),    p(  17,   71),    p(  22,   54),    p(  17,   43),    p(  -0,   46),    p(   7,   58),    p( -10,   76),    p( -10,   79),
        p(   7,   46),    p(  -2,   44),    p( -15,   34),    p( -10,   24),    p( -17,   28),    p( -10,   39),    p( -17,   55),    p( -11,   51),
        p(   1,   14),    p( -12,   23),    p( -15,   17),    p( -16,    8),    p( -15,   13),    p(  -7,   17),    p( -14,   37),    p(  10,   17),
        p(  -5,   15),    p(  -2,   20),    p(  -9,   16),    p(  -8,    4),    p(   5,    1),    p(   7,    7),    p(  13,   18),    p(   7,   14),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-10, -10);
const DOUBLED_PAWN: PhasedScore = p(-7, -21);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 11), p(8, 13), p(14, 19), p(9, 7), p(-3, 16), p(-46, 6)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(38, 9), p(39, 35), p(51, -8), p(35, -34), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(8, -16),
        p(28, 23),
        p(42, 46),
        p(52, 60),
        p(59, 71),
        p(67, 82),
        p(76, 84),
        p(83, 87),
        p(89, 85),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-7, -33),
        p(5, -16),
        p(16, -1),
        p(23, 12),
        p(30, 22),
        p(35, 30),
        p(39, 35),
        p(44, 40),
        p(46, 45),
        p(53, 47),
        p(58, 46),
        p(66, 49),
        p(63, 55),
        p(78, 50),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(7, 105),
        p(16, 119),
        p(20, 125),
        p(23, 129),
        p(23, 136),
        p(29, 140),
        p(32, 144),
        p(36, 147),
        p(40, 151),
        p(44, 155),
        p(47, 157),
        p(49, 162),
        p(56, 162),
        p(65, 160),
        p(67, 160),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(161, 148),
        p(161, 203),
        p(158, 251),
        p(162, 268),
        p(165, 286),
        p(170, 290),
        p(174, 300),
        p(177, 306),
        p(181, 310),
        p(184, 311),
        p(187, 314),
        p(191, 316),
        p(194, 317),
        p(195, 321),
        p(198, 322),
        p(201, 326),
        p(202, 333),
        p(204, 333),
        p(213, 331),
        p(227, 324),
        p(231, 326),
        p(274, 303),
        p(274, 306),
        p(297, 288),
        p(391, 253),
        p(436, 214),
        p(466, 202),
        p(519, 167),
    ],
    [
        p(-93, 8),
        p(-58, -5),
        p(-28, -5),
        p(2, -3),
        p(33, -1),
        p(56, -2),
        p(83, 4),
        p(109, 3),
        p(158, -14),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-8, 7), p(0, 0), p(23, 19), p(49, -12), p(20, -33), p(0, 0)],
    [p(-3, 11), p(20, 23), p(0, 0), p(31, 5), p(31, 53), p(0, 0)],
    [p(-3, 13), p(11, 15), p(18, 12), p(0, 0), p(45, -5), p(0, 0)],
    [p(-2, 5), p(2, 6), p(-0, 21), p(1, 1), p(0, 0), p(0, 0)],
    [p(70, 28), p(-35, 18), p(-9, 17), p(-22, 7), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 7), p(8, 7), p(6, 11), p(13, 7), p(7, 20), p(11, 6)],
    [p(1, 6), p(11, 22), p(-127, -28), p(8, 15), p(9, 20), p(4, 7)],
    [p(2, 2), p(14, 6), p(9, 11), p(11, 8), p(11, 21), p(21, -5)],
    [p(2, -2), p(9, 1), p(7, -5), p(4, 15), p(-60, -253), p(5, -11)],
    [p(63, -1), p(40, 6), p(46, 0), p(24, 5), p(37, -12), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-20, -18), p(19, -10), p(11, -4), p(14, -12), p(-1, 12), p(-13, 11)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(13, 19), p(34, -0), p(5, 32)];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static + StaticallyNamedEntity {
    type Score: ScoreType;

    fn psqt(&self, square: ChessSquare, piece: ChessPieceType, color: ChessColor) -> SingleFeatureScore<Self::Score>;

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

    fn threats(attacking: ChessPieceType, targeted: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> SingleFeatureScore<Self::Score>;

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

    fn bishop_openness(openness: FileOpenness, len: usize) -> <PhasedScore as ScoreType>::SingleFeatureScore {
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
