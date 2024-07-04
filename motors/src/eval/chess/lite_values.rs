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
        p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),
        p( 105,  159),    p( 103,  154),    p(  93,  165),    p( 100,  150),    p(  87,  155),    p(  95,  152),    p(  62,  167),    p(  67,  162),
        p(  56,  119),    p(  55,  124),    p(  64,  114),    p(  72,  115),    p(  58,  124),    p( 104,  108),    p(  82,  129),    p(  72,  119),
        p(  44,  108),    p(  55,  103),    p(  49,   95),    p(  51,   88),    p(  68,   91),    p(  70,   88),    p(  64,   98),    p(  60,   91),
        p(  38,   95),    p(  49,   97),    p(  51,   86),    p(  58,   85),    p(  61,   85),    p(  64,   81),    p(  59,   88),    p(  47,   80),
        p(  33,   91),    p(  44,   87),    p(  43,   84),    p(  47,   90),    p(  56,   88),    p(  48,   84),    p(  59,   79),    p(  41,   78),
        p(  43,   96),    p(  55,   96),    p(  51,   89),    p(  48,   96),    p(  55,  100),    p(  64,   93),    p(  74,   84),    p(  44,   83),
        p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),    p(  67,   67),
    ],
    // knight
    [
        p( 131,  203),    p( 141,  235),    p( 166,  248),    p( 174,  243),    p( 199,  248),    p( 134,  239),    p( 162,  232),    p( 156,  184),
        p( 201,  239),    p( 213,  246),    p( 221,  244),    p( 225,  250),    p( 225,  243),    p( 249,  235),    p( 212,  243),    p( 224,  228),
        p( 212,  238),    p( 223,  240),    p( 239,  250),    p( 240,  254),    p( 254,  249),    p( 272,  242),    p( 237,  239),    p( 234,  235),
        p( 225,  245),    p( 231,  243),    p( 237,  256),    p( 263,  258),    p( 242,  260),    p( 254,  257),    p( 236,  248),    p( 254,  240),
        p( 221,  249),    p( 220,  243),    p( 225,  256),    p( 231,  260),    p( 240,  261),    p( 236,  248),    p( 246,  242),    p( 236,  244),
        p( 196,  236),    p( 198,  238),    p( 204,  240),    p( 209,  254),    p( 216,  250),    p( 203,  234),    p( 219,  231),    p( 215,  239),
        p( 194,  240),    p( 201,  245),    p( 198,  239),    p( 210,  244),    p( 214,  238),    p( 206,  235),    p( 216,  237),    p( 213,  248),
        p( 169,  232),    p( 206,  233),    p( 187,  235),    p( 207,  242),    p( 217,  238),    p( 213,  228),    p( 213,  234),    p( 191,  236),
    ],
    // bishop
    [
        p( 205,  253),    p( 175,  259),    p( 162,  256),    p( 124,  266),    p( 141,  261),    p( 144,  256),    p( 187,  253),    p( 177,  249),
        p( 208,  248),    p( 216,  252),    p( 214,  255),    p( 203,  257),    p( 203,  255),    p( 219,  250),    p( 205,  253),    p( 205,  249),
        p( 218,  257),    p( 232,  251),    p( 222,  259),    p( 226,  254),    p( 228,  257),    p( 250,  258),    p( 244,  251),    p( 231,  260),
        p( 210,  255),    p( 224,  259),    p( 224,  257),    p( 243,  261),    p( 238,  258),    p( 232,  259),    p( 227,  259),    p( 209,  256),
        p( 216,  252),    p( 205,  261),    p( 223,  261),    p( 240,  259),    p( 238,  258),    p( 224,  260),    p( 216,  257),    p( 237,  245),
        p( 214,  255),    p( 229,  256),    p( 227,  259),    p( 228,  261),    p( 232,  263),    p( 227,  258),    p( 232,  250),    p( 230,  250),
        p( 227,  258),    p( 231,  247),    p( 235,  250),    p( 220,  260),    p( 226,  259),    p( 228,  252),    p( 238,  249),    p( 224,  245),
        p( 216,  248),    p( 237,  253),    p( 232,  254),    p( 213,  258),    p( 226,  255),    p( 220,  260),    p( 227,  244),    p( 221,  239),
    ],
    // rook
    [
        p( 334,  451),    p( 321,  461),    p( 315,  469),    p( 312,  466),    p( 328,  461),    p( 338,  459),    p( 345,  457),    p( 362,  448),
        p( 311,  456),    p( 309,  461),    p( 316,  463),    p( 330,  454),    p( 322,  456),    p( 344,  449),    p( 351,  449),    p( 366,  438),
        p( 315,  455),    p( 334,  450),    p( 332,  451),    p( 337,  446),    p( 360,  437),    p( 367,  433),    p( 391,  430),    p( 363,  432),
        p( 313,  454),    p( 322,  450),    p( 323,  453),    p( 328,  447),    p( 338,  439),    p( 347,  435),    p( 351,  436),    p( 351,  432),
        p( 309,  449),    p( 310,  449),    p( 310,  450),    p( 316,  447),    p( 324,  443),    p( 320,  442),    p( 336,  436),    p( 328,  433),
        p( 308,  445),    p( 306,  444),    p( 309,  444),    p( 312,  444),    p( 320,  438),    p( 328,  430),    p( 350,  418),    p( 334,  422),
        p( 311,  440),    p( 314,  440),    p( 320,  441),    p( 323,  439),    p( 330,  432),    p( 344,  423),    p( 355,  417),    p( 325,  425),
        p( 320,  444),    p( 314,  439),    p( 316,  444),    p( 320,  441),    p( 328,  435),    p( 335,  433),    p( 332,  429),    p( 328,  431),
    ],
    // queen
    [
        p( 569,  752),    p( 570,  770),    p( 584,  787),    p( 581,  800),    p( 603,  792),    p( 631,  764),    p( 664,  721),    p( 617,  741),
        p( 577,  768),    p( 553,  797),    p( 550,  827),    p( 544,  838),    p( 551,  853),    p( 593,  815),    p( 595,  793),    p( 634,  772),
        p( 581,  775),    p( 573,  794),    p( 574,  816),    p( 570,  825),    p( 587,  831),    p( 633,  811),    p( 638,  783),    p( 623,  796),
        p( 564,  793),    p( 566,  808),    p( 562,  811),    p( 560,  829),    p( 565,  840),    p( 577,  830),    p( 586,  831),    p( 596,  807),
        p( 572,  785),    p( 559,  802),    p( 561,  811),    p( 563,  828),    p( 564,  824),    p( 565,  827),    p( 580,  812),    p( 588,  803),
        p( 568,  767),    p( 570,  791),    p( 563,  807),    p( 559,  813),    p( 563,  820),    p( 571,  810),    p( 586,  792),    p( 588,  777),
        p( 569,  769),    p( 567,  778),    p( 571,  785),    p( 569,  800),    p( 570,  800),    p( 572,  784),    p( 585,  759),    p( 597,  729),
        p( 554,  764),    p( 564,  758),    p( 562,  776),    p( 571,  779),    p( 572,  772),    p( 560,  770),    p( 565,  756),    p( 568,  746),
    ],
    // king
    [
        p( 133, -109),    p(  31,  -50),    p(  59,  -41),    p(  16,  -19),    p(  36,  -28),    p(  26,  -17),    p(  63,  -22),    p( 249, -117),
        p( -22,  -10),    p( -58,   25),    p( -78,   35),    p( -16,   24),    p( -32,   29),    p( -65,   42),    p( -17,   30),    p(  14,   -4),
        p( -49,    0),    p( -35,   21),    p( -90,   39),    p( -79,   44),    p( -48,   36),    p( -17,   31),    p( -47,   31),    p( -31,    6),
        p( -25,   -8),    p( -88,   20),    p(-100,   37),    p(-133,   47),    p(-119,   43),    p(-103,   36),    p(-105,   25),    p( -96,   10),
        p( -41,  -10),    p(-108,   17),    p(-122,   34),    p(-141,   48),    p(-147,   46),    p(-124,   31),    p(-135,   22),    p(-106,    7),
        p( -33,   -3),    p( -87,   15),    p(-113,   29),    p(-120,   38),    p(-115,   36),    p(-129,   29),    p(-100,   14),    p( -65,    6),
        p(  22,   -4),    p( -69,   12),    p( -81,   19),    p(-101,   28),    p(-105,   29),    p( -92,   21),    p( -58,    5),    p(   8,   -4),
        p(  36,  -38),    p(  38,  -46),    p(  33,  -33),    p( -23,  -16),    p(  27,  -32),    p( -22,  -18),    p(  33,  -42),    p(  61,  -54),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  38,   92),    p(  36,   87),    p(  26,   98),    p(  33,   83),    p(  20,   88),    p(  28,   85),    p(  -5,  100),    p(   0,   95),
        p(  25,  111),    p(  33,  109),    p(  24,   98),    p(   9,   72),    p(  24,   62),    p(   2,   89),    p(  -8,   90),    p( -35,  112),
        p(   9,   65),    p(   5,   65),    p(  15,   53),    p(  10,   44),    p(  -7,   46),    p(  -2,   55),    p( -20,   70),    p( -20,   72),
        p(  -5,   40),    p( -14,   39),    p( -21,   34),    p( -12,   25),    p( -21,   30),    p( -21,   39),    p( -30,   51),    p( -19,   47),
        p(  -8,   10),    p( -22,   20),    p( -21,   18),    p( -16,    9),    p( -18,   14),    p( -17,   18),    p( -22,   32),    p(   1,   15),
        p( -15,   10),    p( -13,   13),    p( -14,   17),    p( -11,    2),    p(   2,    0),    p(  -3,    7),    p(   2,   11),    p(  -1,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];

const BISHOP_PAIR: PhasedScore = p(22, 52);
const ROOK_OPEN_FILE: PhasedScore = p(16, 2);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -5);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(-2, -2);
const KING_OPEN_FILE: PhasedScore = p(-57, -5);
const KING_CLOSED_FILE: PhasedScore = p(14, -17);
const KING_SEMIOPEN_FILE: PhasedScore = p(-10, 2);
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-14, 7),   /*0b0000*/
    p(-18, 10),  /*0b0001*/
    p(-11, 6),   /*0b0010*/
    p(-5, 24),   /*0b0011*/
    p(-6, 5),    /*0b0100*/
    p(-27, -2),  /*0b0101*/
    p(-10, 15),  /*0b0110*/
    p(-8, -4),   /*0b0111*/
    p(-0, 9),    /*0b1000*/
    p(-24, -12), /*0b1001*/
    p(-6, 7),    /*0b1010*/
    p(-7, 5),    /*0b1011*/
    p(-4, 3),    /*0b1100*/
    p(-36, -19), /*0b1101*/
    p(-4, 14),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-9, 15),   /*0b10000*/
    p(4, 9),     /*0b10001*/
    p(-4, -16),  /*0b10010*/
    p(-5, -3),   /*0b10011*/
    p(-7, 5),    /*0b10100*/
    p(11, 10),   /*0b10101*/
    p(-24, -12), /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(11, 44),   /*0b11000*/
    p(20, 7),    /*0b11001*/
    p(22, 25),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(13, 17),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(10, 9),    /*0b100000*/
    p(-1, 10),   /*0b100001*/
    p(13, 2),    /*0b100010*/
    p(10, 12),   /*0b100011*/
    p(-26, -22), /*0b100100*/
    p(-28, -36), /*0b100101*/
    p(-28, 3),   /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(11, -1),   /*0b101000*/
    p(-21, -7),  /*0b101001*/
    p(13, -6),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-24, -24), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(18, 30),   /*0b110000*/
    p(32, 19),   /*0b110001*/
    p(14, -10),  /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(-4, 9),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(25, 33),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(-2, -15),  /*0b111111*/
    p(-29, -9),  /*0b00*/
    p(12, -33),  /*0b01*/
    p(26, -15),  /*0b10*/
    p(40, -46),  /*0b11*/
    p(43, -24),  /*0b100*/
    p(2, -71),   /*0b101*/
    p(77, -57),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(55, -25),  /*0b1000*/
    p(25, -55),  /*0b1001*/
    p(51, -83),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(60, -21),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(18, -36),  /*0b1111*/
    p(8, -11),   /*0b00*/
    p(26, -25),  /*0b01*/
    p(21, -31),  /*0b10*/
    p(32, -47),  /*0b11*/
    p(26, -21),  /*0b100*/
    p(31, -58),  /*0b101*/
    p(20, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(29, -15),  /*0b1000*/
    p(49, -32),  /*0b1001*/
    p(40, -83),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(44, -22),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(19, -76),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(14, 12), p(4, 9), p(9, 14), p(7, 10), p(-4, 17), p(-38, 7)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(36, 11), p(40, 38), p(48, 1), p(38, -22), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-27, -2),
        p(-4, 33),
        p(11, 54),
        p(22, 65),
        p(33, 73),
        p(43, 82),
        p(53, 82),
        p(63, 80),
        p(72, 74),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(2, 16),
        p(15, 33),
        p(24, 48),
        p(31, 59),
        p(39, 67),
        p(44, 75),
        p(47, 79),
        p(51, 82),
        p(54, 85),
        p(61, 85),
        p(68, 83),
        p(79, 83),
        p(74, 90),
        p(92, 81),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-30, 92),
        p(-21, 107),
        p(-18, 112),
        p(-15, 116),
        p(-15, 122),
        p(-10, 126),
        p(-7, 131),
        p(-4, 134),
        p(-0, 138),
        p(3, 142),
        p(8, 144),
        p(10, 148),
        p(18, 148),
        p(29, 145),
        p(28, 148),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-58, 237),
        p(-56, 239),
        p(-60, 288),
        p(-55, 304),
        p(-52, 321),
        p(-47, 329),
        p(-43, 339),
        p(-39, 347),
        p(-34, 350),
        p(-31, 353),
        p(-28, 357),
        p(-23, 359),
        p(-20, 358),
        p(-17, 361),
        p(-14, 361),
        p(-9, 361),
        p(-6, 365),
        p(-1, 361),
        p(11, 355),
        p(28, 343),
        p(41, 337),
        p(93, 305),
        p(100, 299),
        p(131, 277),
        p(227, 233),
        p(265, 203),
        p(331, 168),
        p(396, 125),
    ],
    [
        p(-89, 68),
        p(-54, 31),
        p(-26, 17),
        p(1, 7),
        p(30, -2),
        p(52, -13),
        p(76, -13),
        p(97, -23),
        p(144, -51),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-11, 10),
        p(-7, -2),
        p(22, 15),
        p(43, -10),
        p(25, -48),
        p(0, 0),
    ],
    [p(-4, 14), p(19, 19), p(-2, 9), p(26, 5), p(27, 67), p(0, 0)],
    [p(3, 17), p(21, 18), p(23, 20), p(-5, 11), p(43, 5), p(0, 0)],
    [
        p(-1, -1),
        p(5, 12),
        p(-2, 34),
        p(-5, 18),
        p(-0, -16),
        p(0, 0),
    ],
    [p(70, 37), p(-25, 21), p(6, 19), p(-27, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 5), p(11, 4), p(9, 7), p(16, 3), p(10, 13), p(13, 3)],
    [
        p(1, -6),
        p(10, 13),
        p(-37, -89),
        p(8, 11),
        p(8, 17),
        p(5, 5),
    ],
    [p(1, 2), p(13, 4), p(10, 9), p(12, 7), p(12, 16), p(22, -4)],
    [
        p(3, -4),
        p(11, -5),
        p(10, -9),
        p(5, 14),
        p(15, -302),
        p(9, -16),
    ],
    [
        p(55, -10),
        p(39, -2),
        p(45, -8),
        p(23, -4),
        p(34, -20),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-11, -10),
    p(15, -6),
    p(18, -2),
    p(23, -12),
    p(5, 26),
    p(-1, 22),
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

    fn bishop_pair() -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn rook_openness(openness: FileOpenness) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn king_openness(openness: FileOpenness) -> <Self::Score as ScoreType>::SingleFeatureScore;

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
}
