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
        p( 134,  187),    p( 130,  185),    p( 121,  189),    p( 133,  169),    p( 120,  173),    p( 119,  177),    p(  81,  194),    p(  89,  193),
        p(  67,  123),    p(  66,  124),    p(  77,  120),    p(  85,  124),    p(  74,  124),    p( 121,  111),    p(  95,  131),    p(  93,  121),
        p(  55,  113),    p(  66,  108),    p(  64,  104),    p(  67,   99),    p(  82,   99),    p(  87,   94),    p(  79,  103),    p(  74,   95),
        p(  51,  100),    p(  58,  102),    p(  67,   94),    p(  76,   94),    p(  79,   93),    p(  79,   88),    p(  73,   92),    p(  62,   86),
        p(  46,   97),    p(  55,   92),    p(  59,   94),    p(  61,  100),    p(  69,   96),    p(  64,   92),    p(  72,   82),    p(  55,   85),
        p(  52,   99),    p(  55,   95),    p(  61,   97),    p(  60,  106),    p(  57,  107),    p(  73,   98),    p(  75,   83),    p(  58,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 175,  277),    p( 196,  310),    p( 213,  321),    p( 251,  310),    p( 282,  312),    p( 197,  308),    p( 214,  308),    p( 203,  260),
        p( 268,  310),    p( 283,  316),    p( 298,  307),    p( 301,  311),    p( 301,  306),    p( 315,  295),    p( 276,  312),    p( 271,  303),
        p( 286,  306),    p( 303,  302),    p( 305,  309),    p( 319,  312),    p( 337,  306),    p( 349,  296),    p( 291,  302),    p( 285,  307),
        p( 301,  314),    p( 308,  308),    p( 322,  312),    p( 325,  319),    p( 323,  316),    p( 318,  316),    p( 309,  311),    p( 318,  310),
        p( 299,  316),    p( 302,  306),    p( 311,  312),    p( 319,  315),    p( 317,  318),    p( 322,  302),    p( 321,  302),    p( 313,  312),
        p( 275,  303),    p( 281,  301),    p( 294,  296),    p( 300,  309),    p( 304,  307),    p( 293,  289),    p( 300,  292),    p( 293,  306),
        p( 269,  311),    p( 281,  313),    p( 283,  303),    p( 293,  307),    p( 297,  301),    p( 288,  300),    p( 294,  306),    p( 289,  320),
        p( 239,  310),    p( 282,  303),    p( 266,  305),    p( 287,  310),    p( 296,  307),    p( 291,  298),    p( 288,  306),    p( 265,  308),
    ],
    // bishop
    [
        p( 276,  310),    p( 252,  314),    p( 239,  306),    p( 222,  316),    p( 217,  313),    p( 222,  307),    p( 274,  303),    p( 251,  309),
        p( 281,  303),    p( 278,  303),    p( 290,  305),    p( 277,  303),    p( 288,  300),    p( 292,  298),    p( 269,  307),    p( 270,  302),
        p( 295,  309),    p( 306,  304),    p( 290,  304),    p( 306,  298),    p( 303,  299),    p( 335,  303),    p( 317,  300),    p( 317,  312),
        p( 285,  312),    p( 289,  307),    p( 303,  302),    p( 306,  306),    p( 307,  303),    p( 299,  304),    p( 297,  308),    p( 279,  309),
        p( 288,  308),    p( 283,  309),    p( 294,  304),    p( 309,  305),    p( 303,  300),    p( 298,  302),    p( 285,  304),    p( 308,  302),
        p( 296,  310),    p( 299,  305),    p( 300,  307),    p( 301,  304),    p( 306,  307),    p( 299,  298),    p( 306,  296),    p( 307,  299),
        p( 306,  310),    p( 304,  300),    p( 309,  300),    p( 298,  309),    p( 301,  305),    p( 305,  304),    p( 312,  295),    p( 308,  296),
        p( 297,  305),    p( 310,  306),    p( 308,  307),    p( 290,  309),    p( 306,  308),    p( 295,  310),    p( 306,  297),    p( 302,  292),
    ],
    // rook
    [
        p( 449,  548),    p( 437,  558),    p( 428,  565),    p( 428,  562),    p( 440,  558),    p( 466,  551),    p( 477,  550),    p( 485,  543),
        p( 443,  553),    p( 441,  558),    p( 450,  559),    p( 465,  550),    p( 451,  552),    p( 467,  547),    p( 475,  544),    p( 489,  534),
        p( 446,  547),    p( 463,  542),    p( 458,  544),    p( 457,  539),    p( 484,  528),    p( 493,  526),    p( 508,  526),    p( 486,  527),
        p( 442,  547),    p( 448,  543),    p( 447,  546),    p( 453,  539),    p( 457,  531),    p( 468,  527),    p( 467,  532),    p( 468,  527),
        p( 436,  545),    p( 435,  543),    p( 435,  544),    p( 441,  540),    p( 447,  536),    p( 441,  536),    p( 454,  531),    p( 449,  528),
        p( 431,  543),    p( 431,  539),    p( 433,  539),    p( 436,  538),    p( 442,  532),    p( 453,  524),    p( 469,  514),    p( 455,  517),
        p( 433,  538),    p( 437,  537),    p( 443,  538),    p( 446,  535),    p( 453,  528),    p( 467,  518),    p( 474,  514),    p( 444,  523),
        p( 443,  542),    p( 439,  537),    p( 440,  541),    p( 445,  535),    p( 450,  528),    p( 456,  528),    p( 453,  527),    p( 449,  529),
    ],
    // queen
    [
        p( 867,  962),    p( 869,  976),    p( 881,  991),    p( 905,  982),    p( 904,  985),    p( 929,  972),    p( 968,  928),    p( 914,  960),
        p( 889,  949),    p( 866,  976),    p( 867, 1003),    p( 859, 1021),    p( 868, 1031),    p( 906,  993),    p( 909,  976),    p( 948,  958),
        p( 894,  955),    p( 887,  970),    p( 886,  991),    p( 889,  997),    p( 901, 1004),    p( 948,  984),    p( 952,  959),    p( 944,  964),
        p( 881,  967),    p( 884,  976),    p( 882,  982),    p( 880,  996),    p( 884, 1007),    p( 898,  996),    p( 905, 1002),    p( 913,  977),
        p( 889,  961),    p( 879,  977),    p( 883,  979),    p( 886,  993),    p( 888,  990),    p( 890,  991),    p( 901,  984),    p( 910,  974),
        p( 887,  947),    p( 893,  964),    p( 888,  977),    p( 886,  980),    p( 890,  988),    p( 898,  975),    p( 909,  964),    p( 909,  949),
        p( 886,  949),    p( 887,  957),    p( 894,  960),    p( 894,  974),    p( 895,  974),    p( 896,  957),    p( 907,  936),    p( 916,  907),
        p( 874,  949),    p( 886,  938),    p( 886,  951),    p( 895,  952),    p( 897,  942),    p( 883,  946),    p( 885,  938),    p( 890,  920),
    ],
    // king
    [
        p( 158,  -85),    p(  64,  -38),    p(  86,  -29),    p(  12,    2),    p(  38,  -10),    p(  25,   -1),    p(  76,  -10),    p( 229,  -87),
        p( -31,    2),    p( -80,   19),    p( -81,   27),    p( -22,   17),    p( -52,   24),    p( -80,   39),    p( -51,   24),    p(   6,    0),
        p( -46,   10),    p( -48,   14),    p( -84,   29),    p( -94,   37),    p( -64,   32),    p( -32,   24),    p( -78,   26),    p( -37,   11),
        p( -25,    2),    p(-101,   13),    p(-114,   29),    p(-136,   38),    p(-136,   36),    p(-115,   28),    p(-134,   18),    p(-107,   18),
        p( -41,   -2),    p(-115,    8),    p(-126,   25),    p(-151,   39),    p(-153,   36),    p(-128,   23),    p(-145,   13),    p(-119,   13),
        p( -33,    2),    p( -92,    4),    p(-120,   19),    p(-127,   28),    p(-123,   27),    p(-134,   19),    p(-109,    5),    p( -74,   10),
        p(  25,   -8),    p( -78,   -2),    p( -90,    8),    p(-109,   17),    p(-115,   18),    p(-100,    8),    p( -73,   -9),    p(   3,   -5),
        p(  55,  -24),    p(  43,  -35),    p(  39,  -22),    p( -23,   -1),    p(  29,  -18),    p( -19,   -4),    p(  35,  -29),    p(  67,  -34),
    ],
];

const BISHOP_PAIR: PhasedScore = p(22, 53);
const BAD_BISHOP: [PhasedScore; 9] = [
    p(9, 19),
    p(11, 18),
    p(11, 6),
    p(7, -1),
    p(3, -10),
    p(-1, -19),
    p(-7, -28),
    p(-15, -41),
    p(-28, -52),
];
const ROOK_OPEN_FILE: PhasedScore = p(13, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -0);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 4);
const KING_OPEN_FILE: PhasedScore = p(-44, -2);
const KING_CLOSED_FILE: PhasedScore = p(14, -14);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 3), p(0, 5), p(-2, 4), p(2, 3), p(2, 4), p(3, 7), p(5, 4), p(18, 0)],
    // Closed
    [p(0, 0), p(0, 0), p(15, -21), p(-15, 9), p(-0, 10), p(2, 4), p(-0, 7), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-15, 22), p(4, 16), p(1, 9), p(0, 8), p(4, 4), p(-0, 2), p(10, 5)],
    // SemiClosed
    [p(0, 0), p(12, -13), p(7, 5), p(3, 0), p(7, 1), p(3, 4), p(4, 4), p(1, 4)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-4, 5),    /*0b0000*/
    p(-15, 8),   /*0b0001*/
    p(-3, 8),    /*0b0010*/
    p(-10, 13),  /*0b0011*/
    p(-3, 3),    /*0b0100*/
    p(-26, -0),  /*0b0101*/
    p(-14, 5),   /*0b0110*/
    p(-20, -15), /*0b0111*/
    p(9, 10),    /*0b1000*/
    p(-2, 11),   /*0b1001*/
    p(3, 11),    /*0b1010*/
    p(-2, 9),    /*0b1011*/
    p(-0, 5),    /*0b1100*/
    p(-23, 10),  /*0b1101*/
    p(-11, 4),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(4, 14),    /*0b10000*/
    p(2, 8),     /*0b10001*/
    p(21, 11),   /*0b10010*/
    p(-6, 6),    /*0b10011*/
    p(-5, 5),    /*0b10100*/
    p(13, 15),   /*0b10101*/
    p(-24, 2),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(16, 28),   /*0b11000*/
    p(30, 22),   /*0b11001*/
    p(43, 38),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(18, 10),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(17, 10),   /*0b100000*/
    p(3, 13),    /*0b100001*/
    p(26, 3),    /*0b100010*/
    p(6, -1),    /*0b100011*/
    p(-6, 2),    /*0b100100*/
    p(-21, -7),  /*0b100101*/
    p(-21, 15),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(24, 4),    /*0b101000*/
    p(-0, 16),   /*0b101001*/
    p(23, -2),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-4, 7),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(16, 17),   /*0b110000*/
    p(25, 11),   /*0b110001*/
    p(34, 9),    /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(12, 28),   /*0b110100*/
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
    p(8, -1),    /*0b111111*/
    p(-14, -3),  /*0b00*/
    p(11, -18),  /*0b01*/
    p(38, -8),   /*0b10*/
    p(21, -41),  /*0b11*/
    p(46, -10),  /*0b100*/
    p(5, -22),   /*0b101*/
    p(69, -41),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(64, -12),  /*0b1000*/
    p(20, -34),  /*0b1001*/
    p(80, -58),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(60, -19),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(21, -10),  /*0b1111*/
    p(21, -2),   /*0b00*/
    p(33, -12),  /*0b01*/
    p(27, -18),  /*0b10*/
    p(22, -42),  /*0b11*/
    p(37, -9),   /*0b100*/
    p(55, -20),  /*0b101*/
    p(25, -23),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(39, -3),   /*0b1000*/
    p(53, -18),  /*0b1001*/
    p(51, -41),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(44, -22),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(20, -43),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  34,   87),    p(  30,   85),    p(  21,   89),    p(  33,   69),    p(  20,   73),    p(  19,   77),    p( -19,   94),    p( -11,   93),
        p(  39,  124),    p(  48,  123),    p(  37,  100),    p(  21,   69),    p(  34,   69),    p(  16,   95),    p(   0,  104),    p( -31,  126),
        p(  23,   74),    p(  17,   71),    p(  22,   54),    p(  17,   43),    p(  -1,   46),    p(   7,   58),    p(  -9,   76),    p( -10,   79),
        p(   7,   46),    p(  -2,   44),    p( -15,   34),    p( -10,   24),    p( -17,   28),    p( -11,   39),    p( -18,   55),    p( -12,   51),
        p(   1,   14),    p( -12,   23),    p( -15,   16),    p( -16,    8),    p( -16,   14),    p(  -8,   17),    p( -15,   38),    p(   9,   17),
        p(  -5,   15),    p(  -2,   20),    p(  -9,   16),    p(  -9,    4),    p(   4,    1),    p(   6,    7),    p(  12,   18),    p(   7,   14),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-10, -10);
const DOUBLED_PAWN: PhasedScore = p(-7, -21);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = [
    p(13, 11),
    p(8, 13),
    p(14, 19),
    p(9, 7),
    p(-3, 16),
    p(-46, 7),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(38, 8), p(40, 33), p(52, -9), p(36, -38), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-49, -72),
        p(-28, -32),
        p(-14, -9),
        p(-5, 5),
        p(3, 16),
        p(11, 27),
        p(19, 30),
        p(27, 32),
        p(33, 30),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-19, -39),
        p(-8, -23),
        p(-0, -11),
        p(7, -1),
        p(12, 8),
        p(17, 13),
        p(21, 18),
        p(23, 23),
        p(30, 25),
        p(35, 24),
        p(44, 27),
        p(40, 33),
        p(55, 28),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-62, 32),
        p(-59, 36),
        p(-59, 42),
        p(-53, 47),
        p(-50, 51),
        p(-46, 54),
        p(-42, 58),
        p(-39, 61),
        p(-35, 64),
        p(-33, 68),
        p(-26, 69),
        p(-15, 66),
        p(-13, 66),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-27, -48),
        p(-26, 4),
        p(-30, 54),
        p(-25, 70),
        p(-23, 89),
        p(-18, 93),
        p(-14, 104),
        p(-11, 110),
        p(-7, 114),
        p(-4, 116),
        p(-1, 119),
        p(3, 121),
        p(6, 122),
        p(7, 126),
        p(10, 128),
        p(13, 131),
        p(14, 138),
        p(16, 139),
        p(26, 137),
        p(39, 131),
        p(43, 133),
        p(86, 110),
        p(86, 112),
        p(108, 95),
        p(206, 58),
        p(250, 20),
        p(281, 9),
        p(335, -26),
    ],
    [
        p(-95, 9),
        p(-59, -5),
        p(-29, -6),
        p(2, -4),
        p(34, -2),
        p(57, -3),
        p(85, 3),
        p(110, 2),
        p(159, -15),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
pub const PINNED: [PhasedScore; NUM_CHESS_PIECES - 1] =
    [p(5, -17), p(23, 29), p(15, 34), p(42, 21), p(68, 30)];
const THREATS: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [
        p(-8, 7),
        p(0, 0),
        p(24, 19),
        p(49, -11),
        p(20, -33),
        p(0, 0),
    ],
    [p(-3, 11), p(19, 21), p(0, 0), p(31, 4), p(30, 53), p(0, 0)],
    [p(-3, 14), p(11, 11), p(18, 8), p(0, 0), p(42, 2), p(0, 0)],
    [p(-2, 7), p(2, 3), p(0, 16), p(1, -2), p(0, 0), p(0, 0)],
    [
        p(71, 28),
        p(-36, 18),
        p(-10, 17),
        p(-25, 9),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 7), p(8, 7), p(6, 11), p(13, 7), p(7, 20), p(11, 6)],
    [
        p(1, 6),
        p(11, 22),
        p(-131, -27),
        p(8, 15),
        p(9, 21),
        p(3, 7),
    ],
    [p(2, 2), p(13, 5), p(9, 11), p(11, 8), p(11, 21), p(21, -5)],
    [
        p(2, -2),
        p(9, 1),
        p(7, -5),
        p(4, 15),
        p(-72, -241),
        p(5, -10),
    ],
    [p(63, -2), p(42, 7), p(47, 2), p(25, 5), p(37, -6), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-20, -18),
    p(19, -10),
    p(10, -4),
    p(14, -12),
    p(-2, 13),
    p(-14, 12),
];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(13, 20), p(34, 0), p(5, 31)];

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

    fn pinned(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn threats(
        attacking: ChessPieceType,
        targeted: ChessPieceType,
    ) -> SingleFeatureScore<Self::Score>;

    fn defended(
        protecting: ChessPieceType,
        target: ChessPieceType,
    ) -> SingleFeatureScore<Self::Score>;

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

    fn psqt(&self, square: ChessSquare, piece: ChessPieceType, color: ChessColor) -> PhasedScore {
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

    fn bad_bishop(num_pawns: usize) -> PhasedScore {
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

    fn bishop_openness(openness: FileOpenness, len: usize) -> PhasedScore {
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

    fn pinned(piece: ChessPieceType) -> PhasedScore {
        PINNED[piece as usize]
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
