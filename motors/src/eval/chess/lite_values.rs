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
        p( 132,  187),    p( 129,  186),    p( 119,  189),    p( 132,  170),    p( 119,  174),    p( 119,  177),    p(  81,  195),    p(  89,  193),
        p(  62,  124),    p(  60,  125),    p(  72,  120),    p(  81,  125),    p(  65,  125),    p( 116,  111),    p(  89,  132),    p(  86,  123),
        p(  49,  114),    p(  61,  110),    p(  59,  104),    p(  64,   97),    p(  80,   98),    p(  82,   95),    p(  75,  104),    p(  69,   97),
        p(  46,  100),    p(  53,  103),    p(  62,   95),    p(  71,   94),    p(  74,   93),    p(  75,   89),    p(  68,   93),    p(  57,   87),
        p(  41,   98),    p(  49,   95),    p(  53,   95),    p(  57,  100),    p(  66,   97),    p(  60,   94),    p(  67,   85),    p(  52,   86),
        p(  47,   99),    p(  50,   98),    p(  56,   99),    p(  56,  105),    p(  53,  109),    p(  71,   99),    p(  71,   85),    p(  53,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 186,  270),    p( 210,  302),    p( 245,  314),    p( 269,  305),    p( 300,  306),    p( 218,  301),    p( 232,  301),    p( 214,  253),
        p( 276,  304),    p( 287,  313),    p( 300,  310),    p( 314,  312),    p( 306,  309),    p( 327,  298),    p( 285,  309),    p( 289,  296),
        p( 291,  302),    p( 301,  307),    p( 319,  316),    p( 321,  319),    p( 337,  313),    p( 360,  304),    p( 313,  304),    p( 308,  300),
        p( 304,  311),    p( 310,  309),    p( 317,  321),    p( 343,  323),    p( 321,  324),    p( 334,  321),    p( 315,  312),    p( 332,  305),
        p( 299,  314),    p( 298,  308),    p( 304,  321),    p( 311,  324),    p( 318,  326),    p( 315,  313),    p( 326,  305),    p( 315,  310),
        p( 274,  301),    p( 276,  304),    p( 284,  304),    p( 289,  318),    p( 297,  315),    p( 283,  298),    p( 297,  296),    p( 293,  305),
        p( 271,  306),    p( 281,  310),    p( 277,  305),    p( 289,  310),    p( 292,  304),    p( 284,  300),    p( 294,  301),    p( 289,  315),
        p( 243,  301),    p( 281,  300),    p( 265,  302),    p( 285,  308),    p( 295,  305),    p( 291,  294),    p( 288,  301),    p( 267,  299),
    ],
    // bishop
    [
        p( 281,  314),    p( 258,  314),    p( 250,  306),    p( 226,  315),    p( 223,  315),    p( 230,  305),    p( 283,  304),    p( 252,  308),
        p( 281,  303),    p( 285,  306),    p( 288,  307),    p( 281,  308),    p( 284,  303),    p( 293,  303),    p( 271,  308),    p( 273,  304),
        p( 298,  309),    p( 303,  305),    p( 295,  311),    p( 302,  303),    p( 306,  306),    p( 331,  310),    p( 318,  304),    p( 311,  311),
        p( 281,  310),    p( 297,  309),    p( 300,  306),    p( 316,  313),    p( 311,  308),    p( 306,  310),    p( 301,  308),    p( 282,  311),
        p( 292,  308),    p( 280,  311),    p( 299,  310),    p( 314,  309),    p( 312,  309),    p( 298,  307),    p( 290,  309),    p( 312,  300),
        p( 293,  308),    p( 303,  310),    p( 300,  310),    p( 304,  310),    p( 308,  311),    p( 304,  307),    p( 306,  302),    p( 310,  301),
        p( 308,  312),    p( 303,  301),    p( 310,  304),    p( 296,  310),    p( 302,  308),    p( 303,  306),    p( 312,  301),    p( 302,  299),
        p( 294,  305),    p( 314,  309),    p( 306,  306),    p( 290,  312),    p( 303,  309),    p( 295,  314),    p( 304,  298),    p( 300,  296),
    ],
    // rook
    [
        p( 460,  549),    p( 452,  558),    p( 449,  564),    p( 447,  561),    p( 459,  557),    p( 479,  552),    p( 488,  550),    p( 498,  543),
        p( 434,  556),    p( 431,  561),    p( 439,  562),    p( 455,  552),    p( 446,  553),    p( 465,  549),    p( 478,  545),    p( 491,  536),
        p( 439,  552),    p( 457,  548),    p( 455,  549),    p( 458,  545),    p( 485,  534),    p( 494,  530),    p( 516,  527),    p( 488,  529),
        p( 436,  551),    p( 443,  548),    p( 443,  551),    p( 449,  545),    p( 458,  537),    p( 467,  532),    p( 474,  534),    p( 470,  529),
        p( 431,  548),    p( 431,  546),    p( 432,  547),    p( 438,  544),    p( 445,  540),    p( 439,  539),    p( 459,  532),    p( 448,  531),
        p( 428,  543),    p( 428,  542),    p( 431,  540),    p( 434,  541),    p( 441,  534),    p( 449,  527),    p( 472,  515),    p( 453,  519),
        p( 430,  538),    p( 435,  538),    p( 440,  540),    p( 443,  537),    p( 451,  530),    p( 465,  520),    p( 474,  516),    p( 442,  525),
        p( 440,  541),    p( 436,  538),    p( 438,  543),    p( 443,  538),    p( 450,  532),    p( 456,  531),    p( 454,  529),    p( 448,  530),
    ],
    // queen
    [
        p( 880,  962),    p( 882,  976),    p( 897,  989),    p( 913,  986),    p( 910,  990),    p( 930,  978),    p( 982,  924),    p( 927,  956),
        p( 888,  957),    p( 860,  991),    p( 862, 1018),    p( 853, 1036),    p( 860, 1047),    p( 901, 1007),    p( 905,  987),    p( 950,  963),
        p( 894,  962),    p( 886,  984),    p( 885, 1007),    p( 882, 1016),    p( 904, 1018),    p( 944, 1002),    p( 952,  970),    p( 942,  974),
        p( 880,  976),    p( 884,  989),    p( 877,  999),    p( 875, 1015),    p( 881, 1025),    p( 894, 1014),    p( 904, 1012),    p( 912,  987),
        p( 890,  967),    p( 876,  987),    p( 882,  991),    p( 882, 1010),    p( 883, 1008),    p( 887, 1006),    p( 901,  991),    p( 908,  983),
        p( 885,  952),    p( 890,  970),    p( 883,  988),    p( 881,  991),    p( 887,  996),    p( 893,  989),    p( 908,  969),    p( 908,  956),
        p( 886,  951),    p( 884,  961),    p( 892,  963),    p( 891,  976),    p( 892,  977),    p( 894,  959),    p( 903,  937),    p( 913,  910),
        p( 874,  947),    p( 884,  937),    p( 884,  952),    p( 892,  953),    p( 895,  947),    p( 884,  946),    p( 885,  935),    p( 889,  922),
    ],
    // king
    [
        p( 152, -103),    p(  55,  -50),    p(  80,  -42),    p(   3,  -10),    p(  25,  -22),    p(   7,  -12),    p(  61,  -22),    p( 218, -106),
        p( -23,   -4),    p( -68,   27),    p( -77,   36),    p( -13,   26),    p( -46,   34),    p( -72,   47),    p( -37,   32),    p(   7,   -1),
        p( -44,    5),    p( -37,   23),    p( -81,   41),    p( -86,   48),    p( -54,   42),    p( -19,   35),    p( -57,   33),    p( -31,   10),
        p( -26,   -1),    p( -91,   22),    p(-106,   39),    p(-128,   49),    p(-126,   46),    p(-106,   38),    p(-111,   28),    p( -97,   14),
        p( -45,   -4),    p(-112,   17),    p(-121,   34),    p(-144,   47),    p(-150,   45),    p(-126,   31),    p(-139,   22),    p(-117,   12),
        p( -37,   -1),    p( -88,   12),    p(-118,   27),    p(-125,   36),    p(-120,   34),    p(-134,   27),    p(-106,   12),    p( -75,    9),
        p(  28,  -10),    p( -71,    7),    p( -83,   16),    p(-103,   25),    p(-109,   25),    p( -94,   16),    p( -63,    0),    p(   4,   -4),
        p(  44,  -42),    p(  41,  -48),    p(  36,  -36),    p( -24,  -15),    p(  29,  -33),    p( -21,  -19),    p(  34,  -43),    p(  60,  -51),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 59);
const ROOK_OPEN_FILE: PhasedScore = p(16, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(0, 2);
const KING_OPEN_FILE: PhasedScore = p(-56, -2);
const KING_CLOSED_FILE: PhasedScore = p(16, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 6);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 5), p(-3, 6), p(-2, 9), p(3, 6), p(4, 10), p(4, 12), p(9, 11), p(21, 7)],
    // Closed
    [p(0, 0), p(0, 0), p(11, -29), p(-16, 10), p(0, 14), p(3, 5), p(2, 11), p(-0, 6)],
    // SemiOpen
    [p(0, 0), p(-17, 23), p(1, 21), p(1, 15), p(-1, 19), p(3, 15), p(1, 12), p(11, 11)],
    // SemiClosed
    [p(0, 0), p(10, -12), p(7, 7), p(4, 2), p(8, 6), p(3, 5), p(7, 8), p(2, 5)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-7, 7),    /*0b0000*/
    p(-15, 12),  /*0b0001*/
    p(-2, 8),    /*0b0010*/
    p(-10, 14),  /*0b0011*/
    p(-5, 7),    /*0b0100*/
    p(-26, 4),   /*0b0101*/
    p(-14, 7),   /*0b0110*/
    p(-20, -17), /*0b0111*/
    p(6, 11),    /*0b1000*/
    p(-5, 12),   /*0b1001*/
    p(2, 9),     /*0b1010*/
    p(-4, 11),   /*0b1011*/
    p(-2, 7),    /*0b1100*/
    p(-26, 11),  /*0b1101*/
    p(-13, 4),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(2, 18),    /*0b10000*/
    p(4, 12),    /*0b10001*/
    p(21, 14),   /*0b10010*/
    p(-4, 9),    /*0b10011*/
    p(-5, 8),    /*0b10100*/
    p(13, 17),   /*0b10101*/
    p(-23, 3),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(12, 34),   /*0b11000*/
    p(30, 27),   /*0b11001*/
    p(41, 41),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(15, 13),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(15, 11),   /*0b100000*/
    p(5, 16),    /*0b100001*/
    p(25, 4),    /*0b100010*/
    p(6, 1),     /*0b100011*/
    p(-10, 4),   /*0b100100*/
    p(-24, -6),  /*0b100101*/
    p(-26, 16),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(19, 3),    /*0b101000*/
    p(-2, 19),   /*0b101001*/
    p(19, -3),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-8, 8),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(14, 21),   /*0b110000*/
    p(25, 18),   /*0b110001*/
    p(32, 13),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(7, 32),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(23, 17),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(4, -1),    /*0b111111*/
    p(-21, -10), /*0b00*/
    p(9, -25),   /*0b01*/
    p(37, -13),  /*0b10*/
    p(23, -49),  /*0b11*/
    p(47, -18),  /*0b100*/
    p(-7, -26),  /*0b101*/
    p(73, -48),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(56, -20),  /*0b1000*/
    p(18, -43),  /*0b1001*/
    p(78, -63),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(56, -26),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(13, -6),   /*0b1111*/
    p(16, -10),  /*0b00*/
    p(32, -20),  /*0b01*/
    p(25, -26),  /*0b10*/
    p(23, -52),  /*0b11*/
    p(32, -18),  /*0b100*/
    p(53, -28),  /*0b101*/
    p(23, -33),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(38, -13),  /*0b1000*/
    p(53, -26),  /*0b1001*/
    p(51, -51),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(42, -32),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(20, -54),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  32,   87),    p(  29,   86),    p(  19,   89),    p(  32,   70),    p(  19,   74),    p(  19,   77),    p( -19,   95),    p( -11,   93),
        p(  42,  122),    p(  48,  123),    p(  37,   99),    p(  21,   67),    p(  36,   66),    p(  15,   95),    p(   1,  103),    p( -29,  124),
        p(  24,   72),    p(  18,   70),    p(  23,   53),    p(  16,   43),    p(  -2,   45),    p(   7,   58),    p( -11,   75),    p( -10,   77),
        p(   8,   46),    p(  -3,   44),    p( -15,   34),    p(  -9,   24),    p( -17,   29),    p( -10,   38),    p( -18,   54),    p( -10,   49),
        p(   2,   14),    p( -12,   22),    p( -15,   16),    p( -16,    9),    p( -14,   14),    p(  -8,   17),    p( -13,   36),    p(   9,   16),
        p(  -4,   15),    p(  -2,   20),    p(  -9,   16),    p(  -8,    5),    p(   5,    1),    p(   7,    7),    p(  12,   19),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-11, -10);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(12, 7), p(1, 8), p(9, 14), p(7, 10), p(-6, 18), p(-50, 9)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(44, 11),
    p(49, 38),
    p(56, -5),
    p(44, -37),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-58, -57),
        p(-36, -18),
        p(-20, 3),
        p(-8, 14),
        p(2, 23),
        p(12, 31),
        p(24, 31),
        p(33, 30),
        p(41, 24),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-26, -48),
        p(-14, -30),
        p(-4, -14),
        p(2, -2),
        p(8, 7),
        p(13, 15),
        p(16, 19),
        p(18, 22),
        p(19, 27),
        p(25, 26),
        p(29, 25),
        p(38, 25),
        p(31, 33),
        p(44, 24),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-62, 33),
        p(-59, 38),
        p(-59, 44),
        p(-54, 49),
        p(-51, 53),
        p(-47, 56),
        p(-43, 60),
        p(-39, 63),
        p(-34, 65),
        p(-30, 69),
        p(-21, 68),
        p(-7, 64),
        p(-3, 63),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-33, -31),
        p(-33, 24),
        p(-36, 72),
        p(-31, 89),
        p(-28, 106),
        p(-24, 112),
        p(-19, 123),
        p(-16, 130),
        p(-11, 134),
        p(-8, 136),
        p(-5, 140),
        p(-1, 142),
        p(2, 143),
        p(3, 148),
        p(6, 148),
        p(10, 151),
        p(11, 156),
        p(14, 155),
        p(24, 151),
        p(39, 141),
        p(45, 139),
        p(90, 111),
        p(89, 114),
        p(116, 89),
        p(208, 54),
        p(261, 6),
        p(294, -8),
        p(355, -53),
    ],
    [
        p(-83, 49),
        p(-52, 21),
        p(-27, 11),
        p(-1, 5),
        p(27, -1),
        p(47, -9),
        p(69, -9),
        p(91, -17),
        p(136, -42),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-6, 15),
        p(1, -1),
        p(30, 19),
        p(55, -12),
        p(29, -45),
        p(0, 0),
    ],
    [p(5, 16), p(26, 24), p(6, 12), p(36, 4), p(34, 56), p(0, 0)],
    [p(9, 21), p(27, 24), p(30, 25), p(1, 14), p(49, -3), p(0, 0)],
    [p(6, 1), p(14, 13), p(7, 31), p(8, 6), p(9, -13), p(0, 0)],
    [
        p(79, 39),
        p(-27, 26),
        p(7, 23),
        p(-29, 13),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(4, 5), p(10, 3), p(8, 8), p(15, 3), p(9, 12), p(13, 2)],
    [
        p(-3, -1),
        p(7, 16),
        p(-92, -35),
        p(6, 11),
        p(7, 13),
        p(3, 5),
    ],
    [p(1, 2), p(13, 4), p(8, 10), p(10, 6), p(10, 13), p(20, -5)],
    [
        p(3, -6),
        p(9, -4),
        p(7, -11),
        p(4, 12),
        p(-63, -259),
        p(5, -11),
    ],
    [
        p(59, -8),
        p(37, -0),
        p(42, -6),
        p(21, -3),
        p(33, -19),
        p(0, 0),
    ],
];

pub const NUM_DEFENDED: [PhasedScore; 17] = [
    p(35, -2),
    p(-14, -1),
    p(-22, -2),
    p(-19, -3),
    p(-12, -5),
    p(-11, -4),
    p(-11, -2),
    p(-7, -1),
    p(-3, 2),
    p(0, 5),
    p(3, 12),
    p(6, 16),
    p(9, 21),
    p(10, 27),
    p(11, 18),
    p(9, 5),
    p(6, 57),
];

pub const NUM_ATTACKED: [PhasedScore; 9] = [
    p(13, 3),
    p(4, -4),
    p(-5, -5),
    p(-13, -1),
    p(-20, 6),
    p(-28, 34),
    p(-35, 55),
    p(-31, 32),
    p(-28, -15),
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-13, -10),
    p(17, -8),
    p(17, -3),
    p(23, -13),
    p(6, 22),
    p(5, 18),
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

    fn num_defended(num: usize) -> SingleFeatureScore<Self::Score>;

    fn num_attacked(num: usize) -> SingleFeatureScore<Self::Score>;

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

    fn num_defended(num: usize) -> SingleFeatureScore<Self::Score> {
        NUM_DEFENDED[num]
    }

    fn num_attacked(num: usize) -> SingleFeatureScore<Self::Score> {
        NUM_ATTACKED[num]
    }

    fn king_zone_attack(attacking: ChessPieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }
}
