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
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
        p( 134,  187),    p( 130,  185),    p( 121,  189),    p( 133,  169),    p( 119,  174),    p( 119,  177),    p(  80,  195),    p(  89,  193),
        p(  67,  123),    p(  66,  124),    p(  77,  120),    p(  85,  123),    p(  73,  124),    p( 121,  110),    p(  96,  131),    p(  92,  121),
        p(  55,  112),    p(  66,  108),    p(  65,  104),    p(  67,   98),    p(  83,   99),    p(  87,   94),    p(  79,  103),    p(  74,   95),
        p(  51,   99),    p(  58,  102),    p(  67,   94),    p(  76,   94),    p(  79,   93),    p(  79,   88),    p(  73,   92),    p(  62,   86),
        p(  46,   97),    p(  55,   92),    p(  59,   93),    p(  61,  100),    p(  69,   96),    p(  63,   92),    p(  72,   82),    p(  55,   85),
        p(  52,   99),    p(  55,   95),    p(  61,   97),    p(  60,  105),    p(  57,  107),    p(  73,   97),    p(  75,   83),    p(  58,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 176,  277),    p( 196,  310),    p( 214,  321),    p( 252,  310),    p( 281,  312),    p( 198,  307),    p( 213,  308),    p( 203,  260),
        p( 269,  310),    p( 283,  316),    p( 298,  307),    p( 302,  311),    p( 301,  306),    p( 314,  295),    p( 274,  312),    p( 272,  302),
        p( 287,  306),    p( 303,  303),    p( 305,  309),    p( 319,  312),    p( 335,  306),    p( 347,  296),    p( 290,  302),    p( 286,  307),
        p( 301,  314),    p( 308,  308),    p( 322,  312),    p( 325,  319),    p( 323,  316),    p( 318,  316),    p( 309,  311),    p( 318,  310),
        p( 298,  316),    p( 302,  306),    p( 311,  312),    p( 319,  315),    p( 317,  318),    p( 322,  302),    p( 320,  303),    p( 312,  312),
        p( 275,  303),    p( 281,  301),    p( 293,  296),    p( 300,  309),    p( 304,  307),    p( 293,  289),    p( 300,  292),    p( 293,  306),
        p( 269,  311),    p( 280,  313),    p( 283,  303),    p( 293,  306),    p( 297,  301),    p( 288,  300),    p( 294,  305),    p( 289,  320),
        p( 239,  310),    p( 282,  303),    p( 266,  305),    p( 287,  310),    p( 295,  307),    p( 291,  297),    p( 288,  306),    p( 265,  308),
    ],
    // bishop
    [
        p( 276,  310),    p( 251,  314),    p( 236,  306),    p( 222,  316),    p( 215,  313),    p( 228,  306),    p( 274,  303),    p( 253,  308),
        p( 281,  303),    p( 278,  303),    p( 289,  305),    p( 277,  303),    p( 287,  300),    p( 292,  297),    p( 264,  308),    p( 272,  301),
        p( 295,  309),    p( 306,  304),    p( 290,  304),    p( 306,  298),    p( 304,  299),    p( 335,  304),    p( 315,  300),    p( 316,  312),
        p( 285,  313),    p( 291,  306),    p( 303,  302),    p( 307,  305),    p( 307,  303),    p( 299,  304),    p( 296,  308),    p( 279,  309),
        p( 289,  307),    p( 283,  309),    p( 295,  303),    p( 309,  305),    p( 302,  300),    p( 298,  302),    p( 285,  304),    p( 308,  302),
        p( 296,  310),    p( 300,  304),    p( 300,  307),    p( 300,  304),    p( 306,  307),    p( 299,  298),    p( 305,  296),    p( 307,  298),
        p( 307,  309),    p( 304,  300),    p( 309,  300),    p( 298,  309),    p( 301,  305),    p( 305,  304),    p( 312,  295),    p( 308,  297),
        p( 297,  305),    p( 310,  306),    p( 308,  307),    p( 291,  309),    p( 306,  309),    p( 294,  310),    p( 306,  297),    p( 301,  292),
    ],
    // rook
    [
        p( 451,  547),    p( 440,  557),    p( 432,  564),    p( 431,  561),    p( 443,  557),    p( 468,  551),    p( 478,  549),    p( 487,  543),
        p( 443,  553),    p( 441,  558),    p( 450,  559),    p( 465,  550),    p( 451,  552),    p( 466,  547),    p( 475,  544),    p( 490,  534),
        p( 446,  547),    p( 464,  542),    p( 458,  544),    p( 457,  539),    p( 480,  530),    p( 492,  526),    p( 509,  526),    p( 486,  527),
        p( 442,  547),    p( 447,  543),    p( 447,  546),    p( 451,  541),    p( 456,  532),    p( 467,  527),    p( 467,  531),    p( 468,  526),
        p( 436,  545),    p( 435,  543),    p( 434,  545),    p( 440,  541),    p( 446,  537),    p( 441,  536),    p( 454,  531),    p( 448,  528),
        p( 431,  543),    p( 431,  540),    p( 433,  539),    p( 436,  539),    p( 442,  533),    p( 453,  524),    p( 468,  514),    p( 455,  518),
        p( 433,  539),    p( 437,  537),    p( 443,  538),    p( 446,  535),    p( 452,  528),    p( 466,  519),    p( 473,  515),    p( 443,  523),
        p( 443,  542),    p( 439,  537),    p( 440,  541),    p( 445,  535),    p( 450,  528),    p( 456,  529),    p( 453,  528),    p( 449,  529),
    ],
    // queen
    [
        p( 869,  960),    p( 872,  974),    p( 884,  989),    p( 907,  981),    p( 906,  984),    p( 929,  972),    p( 970,  927),    p( 916,  959),
        p( 889,  949),    p( 865,  977),    p( 866, 1003),    p( 858, 1021),    p( 866, 1032),    p( 906,  993),    p( 908,  977),    p( 947,  958),
        p( 893,  955),    p( 887,  970),    p( 886,  991),    p( 888,  998),    p( 903, 1004),    p( 947,  984),    p( 953,  959),    p( 943,  965),
        p( 880,  967),    p( 884,  976),    p( 881,  982),    p( 880,  996),    p( 883, 1007),    p( 898,  997),    p( 905, 1002),    p( 912,  977),
        p( 889,  960),    p( 879,  977),    p( 883,  979),    p( 885,  994),    p( 888,  991),    p( 890,  991),    p( 901,  984),    p( 909,  974),
        p( 886,  948),    p( 892,  965),    p( 887,  978),    p( 885,  980),    p( 890,  988),    p( 897,  976),    p( 909,  964),    p( 908,  949),
        p( 886,  950),    p( 887,  958),    p( 894,  960),    p( 893,  974),    p( 895,  973),    p( 896,  957),    p( 906,  936),    p( 915,  907),
        p( 874,  950),    p( 885,  938),    p( 886,  951),    p( 894,  952),    p( 897,  941),    p( 883,  946),    p( 885,  938),    p( 890,  920),
    ],
    // king
    [
        p( 158,  -85),    p(  61,  -38),    p(  85,  -30),    p(  10,    2),    p(  37,  -11),    p(  25,   -1),    p(  77,  -10),    p( 231,  -88),
        p( -30,    1),    p( -81,   19),    p( -82,   27),    p( -23,   17),    p( -52,   24),    p( -80,   39),    p( -52,   24),    p(   5,    0),
        p( -45,    9),    p( -48,   14),    p( -85,   29),    p( -95,   37),    p( -65,   32),    p( -31,   24),    p( -79,   26),    p( -37,   11),
        p( -25,    2),    p(-101,   13),    p(-114,   29),    p(-137,   38),    p(-137,   36),    p(-115,   28),    p(-133,   18),    p(-106,   17),
        p( -41,   -2),    p(-115,    9),    p(-127,   25),    p(-152,   39),    p(-154,   36),    p(-129,   23),    p(-145,   13),    p(-119,   13),
        p( -34,    2),    p( -93,    4),    p(-120,   19),    p(-128,   28),    p(-124,   27),    p(-135,   19),    p(-110,    5),    p( -74,   10),
        p(  25,   -8),    p( -78,   -2),    p( -91,    8),    p(-110,   17),    p(-116,   18),    p(-100,    9),    p( -73,   -9),    p(   3,   -5),
        p(  56,  -24),    p(  43,  -35),    p(  39,  -22),    p( -23,   -1),    p(  29,  -18),    p( -19,   -5),    p(  35,  -29),    p(  68,  -35),
    ],
];

const BISHOP_PAIR: PhasedScore = p(22, 53);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(9, 19), p(10, 18), p(11, 6), p(7, -2), p(3, -10), p(-1, -20), p(-7, -28), p(-15, -42), p(-28, -53)];
const ROOK_OPEN_FILE: PhasedScore = p(13, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 3);
const KING_OPEN_FILE: PhasedScore = p(-42, -2);
const KING_CLOSED_FILE: PhasedScore = p(14, -14);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 3), p(0, 5), p(-2, 4), p(2, 3), p(2, 4), p(3, 7), p(5, 4), p(18, 1)],
    // Closed
    [p(0, 0), p(0, 0), p(14, -20), p(-15, 9), p(-0, 10), p(2, 4), p(-1, 7), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-13, 22), p(3, 17), p(1, 9), p(0, 8), p(4, 4), p(-1, 2), p(10, 4)],
    // SemiClosed
    [p(0, 0), p(11, -13), p(7, 6), p(3, 0), p(7, 1), p(3, 4), p(4, 5), p(1, 4)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-4, 5),    /*0b0000*/
    p(-14, 8),   /*0b0001*/
    p(-2, 7),    /*0b0010*/
    p(-10, 13),  /*0b0011*/
    p(-3, 3),    /*0b0100*/
    p(-26, -0),  /*0b0101*/
    p(-14, 5),   /*0b0110*/
    p(-20, -15), /*0b0111*/
    p(10, 10),   /*0b1000*/
    p(-2, 11),   /*0b1001*/
    p(4, 11),    /*0b1010*/
    p(-3, 10),   /*0b1011*/
    p(-0, 5),    /*0b1100*/
    p(-23, 10),  /*0b1101*/
    p(-11, 4),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(4, 14),    /*0b10000*/
    p(2, 8),     /*0b10001*/
    p(21, 11),   /*0b10010*/
    p(-6, 7),    /*0b10011*/
    p(-5, 5),    /*0b10100*/
    p(13, 15),   /*0b10101*/
    p(-24, 2),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(16, 28),   /*0b11000*/
    p(29, 23),   /*0b11001*/
    p(43, 38),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(18, 10),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(17, 9),    /*0b100000*/
    p(3, 13),    /*0b100001*/
    p(26, 2),    /*0b100010*/
    p(6, -1),    /*0b100011*/
    p(-6, 2),    /*0b100100*/
    p(-21, -6),  /*0b100101*/
    p(-22, 15),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(24, 4),    /*0b101000*/
    p(-0, 17),   /*0b101001*/
    p(23, -2),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-5, 8),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(16, 17),   /*0b110000*/
    p(25, 12),   /*0b110001*/
    p(34, 9),    /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(11, 28),   /*0b110100*/
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
    p(12, -18),  /*0b01*/
    p(38, -9),   /*0b10*/
    p(21, -41),  /*0b11*/
    p(47, -11),  /*0b100*/
    p(6, -20),   /*0b101*/
    p(69, -41),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(64, -13),  /*0b1000*/
    p(21, -33),  /*0b1001*/
    p(80, -58),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(61, -19),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(21, -10),  /*0b1111*/
    p(21, -3),   /*0b00*/
    p(33, -13),  /*0b01*/
    p(28, -18),  /*0b10*/
    p(22, -42),  /*0b11*/
    p(38, -10),  /*0b100*/
    p(56, -20),  /*0b101*/
    p(26, -23),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(40, -4),   /*0b1000*/
    p(53, -18),  /*0b1001*/
    p(51, -41),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(45, -22),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(21, -43),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  34,   87),    p(  30,   85),    p(  21,   89),    p(  33,   69),    p(  19,   74),    p(  19,   77),    p( -20,   95),    p( -11,   93),
        p(  40,  124),    p(  48,  123),    p(  37,  100),    p(  21,   69),    p(  35,   69),    p(  16,   95),    p(  -0,  104),    p( -31,  126),
        p(  23,   74),    p(  17,   71),    p(  22,   54),    p(  17,   43),    p(  -0,   46),    p(   7,   58),    p(  -9,   76),    p( -10,   79),
        p(   7,   46),    p(  -2,   44),    p( -15,   34),    p( -10,   24),    p( -17,   29),    p( -10,   39),    p( -18,   55),    p( -12,   51),
        p(   1,   14),    p( -12,   23),    p( -15,   17),    p( -16,    8),    p( -16,   13),    p(  -7,   17),    p( -15,   37),    p(   9,   17),
        p(  -5,   15),    p(  -2,   20),    p(  -9,   16),    p(  -9,    4),    p(   4,    1),    p(   7,    7),    p(  12,   19),    p(   6,   14),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-10, -10);
const DOUBLED_PAWN: PhasedScore = p(-7, -21);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 11), p(8, 13), p(14, 19), p(9, 7), p(-3, 16), p(-46, 7)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(38, 9), p(40, 34), p(51, -10), p(36, -39), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-49, -71),
        p(-28, -32),
        p(-15, -9),
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
        p(-30, -56),
        p(-19, -39),
        p(-7, -23),
        p(-0, -11),
        p(7, -1),
        p(12, 8),
        p(16, 13),
        p(20, 18),
        p(22, 23),
        p(29, 25),
        p(35, 24),
        p(43, 27),
        p(40, 33),
        p(54, 28),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-67, 27),
        p(-62, 33),
        p(-59, 37),
        p(-60, 43),
        p(-54, 48),
        p(-50, 52),
        p(-46, 54),
        p(-42, 58),
        p(-39, 62),
        p(-35, 64),
        p(-34, 69),
        p(-26, 69),
        p(-16, 66),
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
        p(-27, -47),
        p(-27, 6),
        p(-31, 56),
        p(-26, 72),
        p(-24, 91),
        p(-19, 95),
        p(-15, 105),
        p(-12, 111),
        p(-8, 115),
        p(-5, 117),
        p(-2, 120),
        p(2, 122),
        p(5, 122),
        p(6, 127),
        p(9, 128),
        p(12, 131),
        p(13, 139),
        p(15, 139),
        p(25, 137),
        p(38, 131),
        p(42, 133),
        p(85, 110),
        p(86, 111),
        p(108, 94),
        p(203, 59),
        p(249, 20),
        p(282, 7),
        p(341, -31),
    ],
    [
        p(-95, 9),
        p(-59, -5),
        p(-29, -6),
        p(2, -4),
        p(34, -2),
        p(57, -3),
        p(86, 3),
        p(111, 2),
        p(161, -15),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-8, 7), p(0, 0), p(24, 19), p(49, -12), p(20, -33), p(0, 0)],
    [p(-3, 11), p(20, 23), p(0, 0), p(31, 6), p(31, 54), p(0, 0)],
    [p(-3, 14), p(11, 13), p(18, 7), p(0, 0), p(43, -1), p(0, 0)],
    [p(-2, 6), p(2, 4), p(0, 15), p(1, 2), p(0, 0), p(0, 0)],
    [p(68, 28), p(-32, 18), p(4, 16), p(-23, 8), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 7), p(8, 7), p(6, 11), p(13, 7), p(7, 20), p(11, 6)],
    [p(1, 6), p(11, 22), p(-119, -29), p(8, 15), p(9, 22), p(3, 6)],
    [p(2, 2), p(13, 5), p(9, 10), p(11, 8), p(11, 22), p(21, -6)],
    [p(2, -2), p(9, 1), p(7, -6), p(4, 14), p(-70, -243), p(5, -10)],
    [p(64, -2), p(42, 7), p(47, 2), p(25, 5), p(37, -7), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-20, -18), p(19, -10), p(11, -4), p(14, -12), p(-1, 12), p(-14, 12)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(14, 19), p(34, 1), p(5, 33)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(4, -17), p(21, 13), p(14, 35), p(34, -3), p(24, 44)];
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(-3, -21), p(65, 16), p(146, -8), p(95, -49), p(0, 0), p(28, 1)];

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

    fn pin(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn discovered_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;
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

    fn pin(piece: ChessPieceType) -> PhasedScore {
        PIN[piece as usize]
    }

    fn discovered_check(piece: ChessPieceType) -> PhasedScore {
        DISCOVERED_CHECK[piece as usize]
    }
}
