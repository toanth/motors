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
        p( 132,  187),    p( 129,  186),    p( 119,  189),    p( 131,  170),    p( 118,  174),    p( 119,  178),    p(  81,  196),    p(  90,  193),
        p(  62,  124),    p(  60,  125),    p(  72,  120),    p(  80,  125),    p(  65,  125),    p( 115,  111),    p(  89,  132),    p(  86,  123),
        p(  49,  114),    p(  61,  110),    p(  58,  104),    p(  63,   97),    p(  79,   99),    p(  81,   95),    p(  74,  105),    p(  69,   97),
        p(  46,  101),    p(  53,  103),    p(  61,   96),    p(  71,   94),    p(  74,   93),    p(  75,   90),    p(  68,   94),    p(  57,   87),
        p(  41,   98),    p(  49,   95),    p(  53,   95),    p(  57,  100),    p(  66,   97),    p(  60,   94),    p(  67,   85),    p(  52,   86),
        p(  47,   99),    p(  49,   98),    p(  56,   99),    p(  55,  106),    p(  52,  109),    p(  70,   99),    p(  71,   86),    p(  52,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 186,  271),    p( 211,  302),    p( 244,  315),    p( 268,  305),    p( 300,  307),    p( 214,  302),    p( 231,  302),    p( 214,  253),
        p( 276,  304),    p( 287,  313),    p( 300,  310),    p( 314,  312),    p( 306,  309),    p( 327,  298),    p( 286,  309),    p( 290,  296),
        p( 291,  302),    p( 301,  307),    p( 318,  316),    p( 320,  320),    p( 336,  313),    p( 359,  304),    p( 313,  304),    p( 308,  300),
        p( 303,  311),    p( 309,  310),    p( 316,  321),    p( 342,  323),    p( 319,  325),    p( 332,  322),    p( 314,  313),    p( 332,  306),
        p( 299,  314),    p( 298,  308),    p( 304,  321),    p( 311,  325),    p( 318,  326),    p( 315,  313),    p( 326,  305),    p( 316,  310),
        p( 274,  301),    p( 276,  304),    p( 283,  304),    p( 289,  319),    p( 297,  315),    p( 283,  298),    p( 297,  296),    p( 293,  305),
        p( 271,  306),    p( 281,  310),    p( 276,  306),    p( 288,  310),    p( 291,  304),    p( 283,  301),    p( 293,  302),    p( 289,  315),
        p( 243,  301),    p( 280,  301),    p( 265,  302),    p( 284,  308),    p( 294,  306),    p( 291,  295),    p( 287,  301),    p( 266,  300),
    ],
    // bishop
    [
        p( 282,  314),    p( 257,  314),    p( 249,  306),    p( 225,  315),    p( 223,  315),    p( 226,  306),    p( 283,  304),    p( 253,  308),
        p( 281,  303),    p( 285,  306),    p( 288,  307),    p( 281,  308),    p( 284,  303),    p( 293,  303),    p( 272,  308),    p( 274,  303),
        p( 298,  309),    p( 303,  305),    p( 295,  311),    p( 302,  303),    p( 306,  306),    p( 332,  309),    p( 318,  304),    p( 312,  311),
        p( 281,  310),    p( 298,  309),    p( 300,  306),    p( 316,  312),    p( 311,  308),    p( 306,  310),    p( 301,  308),    p( 282,  311),
        p( 291,  308),    p( 280,  311),    p( 299,  309),    p( 314,  309),    p( 312,  309),    p( 298,  307),    p( 291,  309),    p( 311,  300),
        p( 293,  308),    p( 303,  310),    p( 300,  310),    p( 303,  310),    p( 308,  310),    p( 304,  307),    p( 306,  302),    p( 310,  301),
        p( 307,  312),    p( 303,  301),    p( 310,  304),    p( 296,  310),    p( 303,  308),    p( 302,  306),    p( 312,  301),    p( 302,  299),
        p( 294,  305),    p( 313,  309),    p( 306,  307),    p( 289,  312),    p( 303,  309),    p( 295,  314),    p( 303,  298),    p( 300,  296),
    ],
    // rook
    [
        p( 459,  550),    p( 450,  559),    p( 447,  565),    p( 446,  562),    p( 458,  558),    p( 478,  552),    p( 486,  551),    p( 497,  543),
        p( 433,  556),    p( 430,  561),    p( 439,  562),    p( 455,  552),    p( 445,  554),    p( 465,  549),    p( 476,  546),    p( 490,  537),
        p( 437,  553),    p( 456,  548),    p( 454,  550),    p( 457,  545),    p( 484,  535),    p( 493,  531),    p( 516,  528),    p( 487,  530),
        p( 435,  552),    p( 442,  548),    p( 442,  551),    p( 448,  546),    p( 457,  538),    p( 466,  533),    p( 473,  535),    p( 469,  530),
        p( 429,  548),    p( 430,  547),    p( 431,  548),    p( 437,  545),    p( 444,  541),    p( 438,  540),    p( 458,  533),    p( 447,  531),
        p( 427,  544),    p( 426,  543),    p( 430,  541),    p( 432,  542),    p( 440,  535),    p( 448,  528),    p( 471,  515),    p( 452,  520),
        p( 429,  539),    p( 433,  539),    p( 439,  540),    p( 442,  538),    p( 450,  531),    p( 464,  521),    p( 472,  516),    p( 440,  525),
        p( 439,  542),    p( 435,  539),    p( 437,  543),    p( 441,  539),    p( 449,  533),    p( 455,  532),    p( 453,  529),    p( 447,  530),
    ],
    // queen
    [
        p( 879,  963),    p( 882,  977),    p( 896,  990),    p( 912,  986),    p( 911,  990),    p( 930,  977),    p( 982,  924),    p( 926,  957),
        p( 887,  957),    p( 860,  991),    p( 863, 1018),    p( 855, 1035),    p( 863, 1045),    p( 902, 1006),    p( 906,  986),    p( 949,  963),
        p( 894,  962),    p( 885,  983),    p( 884, 1007),    p( 883, 1016),    p( 905, 1017),    p( 945, 1001),    p( 953,  969),    p( 941,  974),
        p( 879,  976),    p( 883,  989),    p( 878,  998),    p( 876, 1015),    p( 881, 1024),    p( 894, 1014),    p( 904, 1012),    p( 912,  987),
        p( 889,  968),    p( 875,  988),    p( 881,  991),    p( 882, 1010),    p( 883, 1008),    p( 886, 1006),    p( 901,  990),    p( 908,  983),
        p( 884,  953),    p( 889,  970),    p( 883,  988),    p( 881,  991),    p( 887,  996),    p( 894,  988),    p( 908,  969),    p( 907,  956),
        p( 886,  952),    p( 884,  961),    p( 891,  963),    p( 890,  977),    p( 892,  977),    p( 894,  959),    p( 903,  937),    p( 912,  911),
        p( 873,  948),    p( 883,  939),    p( 883,  953),    p( 892,  954),    p( 894,  948),    p( 883,  948),    p( 883,  937),    p( 888,  923),
    ],
    // king
    [
        p( 152, -103),    p(  56,  -50),    p(  80,  -42),    p(   3,  -10),    p(  25,  -22),    p(   6,  -12),    p(  61,  -22),    p( 218, -106),
        p( -23,   -4),    p( -68,   27),    p( -76,   36),    p( -13,   26),    p( -46,   34),    p( -72,   47),    p( -37,   32),    p(   6,   -1),
        p( -44,    5),    p( -37,   23),    p( -81,   41),    p( -86,   48),    p( -53,   42),    p( -19,   35),    p( -57,   33),    p( -31,   10),
        p( -26,   -2),    p( -91,   22),    p(-106,   39),    p(-128,   49),    p(-126,   46),    p(-106,   38),    p(-111,   28),    p( -97,   14),
        p( -45,   -4),    p(-112,   18),    p(-121,   34),    p(-144,   47),    p(-149,   45),    p(-126,   31),    p(-139,   22),    p(-117,   12),
        p( -36,   -1),    p( -89,   12),    p(-118,   27),    p(-125,   36),    p(-120,   34),    p(-135,   27),    p(-106,   12),    p( -75,    9),
        p(  28,  -10),    p( -71,    7),    p( -83,   16),    p(-103,   25),    p(-109,   26),    p( -94,   16),    p( -63,    0),    p(   4,   -4),
        p(  45,  -42),    p(  41,  -47),    p(  36,  -36),    p( -23,  -15),    p(  30,  -33),    p( -21,  -19),    p(  34,  -43),    p(  60,  -51),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(16, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(0, 2);
const KING_OPEN_FILE: PhasedScore = p(-56, -2);
const KING_CLOSED_FILE: PhasedScore = p(16, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 6);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 5), p(-3, 7), p(-2, 9), p(3, 7), p(4, 10), p(4, 12), p(10, 12), p(20, 7)],
    // Closed
    [p(0, 0), p(0, 0), p(11, -28), p(-15, 9), p(-0, 14), p(2, 5), p(2, 11), p(-1, 6)],
    // SemiOpen
    [p(0, 0), p(-17, 23), p(2, 21), p(1, 14), p(-1, 20), p(3, 15), p(1, 12), p(11, 12)],
    // SemiClosed
    [p(0, 0), p(9, -12), p(6, 7), p(4, 2), p(8, 6), p(3, 5), p(7, 8), p(2, 5)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-6, 7),    /*0b0000*/
    p(-15, 12),  /*0b0001*/
    p(-2, 8),    /*0b0010*/
    p(-10, 14),  /*0b0011*/
    p(-4, 7),    /*0b0100*/
    p(-26, 4),   /*0b0101*/
    p(-15, 7),   /*0b0110*/
    p(-20, -17), /*0b0111*/
    p(6, 11),    /*0b1000*/
    p(-5, 12),   /*0b1001*/
    p(2, 9),     /*0b1010*/
    p(-5, 12),   /*0b1011*/
    p(-1, 7),    /*0b1100*/
    p(-25, 11),  /*0b1101*/
    p(-13, 4),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(2, 18),    /*0b10000*/
    p(4, 12),    /*0b10001*/
    p(22, 14),   /*0b10010*/
    p(-5, 9),    /*0b10011*/
    p(-5, 8),    /*0b10100*/
    p(12, 17),   /*0b10101*/
    p(-23, 3),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(13, 34),   /*0b11000*/
    p(30, 27),   /*0b11001*/
    p(42, 40),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(15, 14),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(15, 11),   /*0b100000*/
    p(5, 15),    /*0b100001*/
    p(25, 4),    /*0b100010*/
    p(5, 1),     /*0b100011*/
    p(-10, 4),   /*0b100100*/
    p(-23, -7),  /*0b100101*/
    p(-26, 17),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(19, 3),    /*0b101000*/
    p(-1, 19),   /*0b101001*/
    p(19, -3),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-7, 7),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(15, 21),   /*0b110000*/
    p(26, 18),   /*0b110001*/
    p(32, 13),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(7, 33),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(24, 17),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(4, -1),    /*0b111111*/
    p(-21, -10), /*0b00*/
    p(8, -24),   /*0b01*/
    p(37, -13),  /*0b10*/
    p(23, -49),  /*0b11*/
    p(47, -18),  /*0b100*/
    p(-6, -26),  /*0b101*/
    p(73, -48),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(56, -20),  /*0b1000*/
    p(18, -43),  /*0b1001*/
    p(79, -63),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(57, -27),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(11, -3),   /*0b1111*/
    p(17, -10),  /*0b00*/
    p(33, -20),  /*0b01*/
    p(26, -26),  /*0b10*/
    p(23, -52),  /*0b11*/
    p(33, -18),  /*0b100*/
    p(54, -29),  /*0b101*/
    p(23, -33),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(38, -13),  /*0b1000*/
    p(54, -26),  /*0b1001*/
    p(51, -51),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(43, -32),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(20, -53),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  32,   87),    p(  29,   86),    p(  19,   89),    p(  31,   70),    p(  18,   74),    p(  19,   78),    p( -19,   96),    p( -10,   93),
        p(  41,  123),    p(  47,  123),    p(  37,   99),    p(  21,   67),    p(  35,   67),    p(  15,   95),    p(   0,  103),    p( -28,  124),
        p(  24,   73),    p(  17,   70),    p(  23,   53),    p(  15,   43),    p(  -1,   44),    p(   7,   58),    p( -10,   75),    p( -10,   77),
        p(   8,   46),    p(  -3,   44),    p( -15,   34),    p(  -9,   24),    p( -17,   29),    p( -10,   38),    p( -19,   54),    p( -11,   50),
        p(   2,   14),    p( -12,   22),    p( -14,   16),    p( -16,    9),    p( -14,   13),    p(  -8,   17),    p( -13,   36),    p(   9,   16),
        p(  -4,   15),    p(  -2,   20),    p(  -9,   16),    p(  -8,    5),    p(   6,    1),    p(   7,    7),    p(  12,   19),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-11, -10);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(12, 7), p(1, 8), p(8, 14), p(7, 10), p(-7, 18), p(-50, 10)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(39, 9), p(43, 35), p(52, -9), p(38, -40), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-59, -57),
        p(-36, -18),
        p(-21, 3),
        p(-9, 14),
        p(2, 23),
        p(12, 32),
        p(23, 31),
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
        p(-26, -49),
        p(-14, -30),
        p(-5, -14),
        p(2, -2),
        p(8, 7),
        p(12, 15),
        p(15, 19),
        p(18, 22),
        p(19, 27),
        p(25, 27),
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
        p(-59, 37),
        p(-59, 44),
        p(-54, 49),
        p(-51, 53),
        p(-48, 56),
        p(-44, 60),
        p(-40, 64),
        p(-34, 65),
        p(-31, 69),
        p(-21, 68),
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
        p(-32, -35),
        p(-33, 21),
        p(-36, 69),
        p(-31, 87),
        p(-28, 104),
        p(-24, 110),
        p(-19, 120),
        p(-16, 127),
        p(-12, 132),
        p(-9, 134),
        p(-6, 138),
        p(-2, 141),
        p(1, 141),
        p(3, 146),
        p(6, 147),
        p(9, 149),
        p(10, 155),
        p(13, 153),
        p(23, 150),
        p(38, 140),
        p(43, 138),
        p(89, 111),
        p(88, 112),
        p(114, 89),
        p(207, 53),
        p(259, 6),
        p(288, -6),
        p(356, -55),
    ],
    [
        p(-83, 48),
        p(-51, 20),
        p(-26, 11),
        p(-0, 5),
        p(28, -1),
        p(48, -10),
        p(70, -9),
        p(92, -17),
        p(136, -43),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(23, 16),
        p(49, -16),
        p(22, -46),
        p(0, 0),
    ],
    [p(-1, 12), p(19, 21), p(-1, 8), p(29, 1), p(28, 53), p(0, 0)],
    [
        p(4, 16),
        p(22, 20),
        p(24, 21),
        p(-5, 10),
        p(43, -6),
        p(0, 0),
    ],
    [p(1, -3), p(8, 11), p(0, 29), p(1, 4), p(2, -17), p(0, 0)],
    [p(78, 34), p(-31, 22), p(2, 19), p(-33, 8), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(4, 5), p(10, 3), p(8, 8), p(15, 3), p(9, 12), p(13, 2)],
    [
        p(-3, -1),
        p(7, 16),
        p(-96, -34),
        p(6, 11),
        p(6, 13),
        p(3, 5),
    ],
    [p(1, 2), p(13, 4), p(8, 10), p(9, 6), p(10, 13), p(19, -5)],
    [
        p(3, -6),
        p(9, -3),
        p(7, -11),
        p(4, 12),
        p(-62, -260),
        p(4, -11),
    ],
    [
        p(59, -8),
        p(37, -0),
        p(42, -6),
        p(21, -3),
        p(34, -19),
        p(0, 0),
    ],
];

pub const NUM_DEFENDED: [PhasedScore; 17] = [
    p(33, -2),
    p(-16, -1),
    p(-24, -1),
    p(-21, -2),
    p(-14, -4),
    p(-14, -3),
    p(-14, -1),
    p(-10, 0),
    p(-5, 3),
    p(-1, 5),
    p(3, 12),
    p(7, 17),
    p(11, 20),
    p(14, 27),
    p(16, 16),
    p(15, -5),
    p(14, 361),
];

pub const NUM_HANGING: [PhasedScore; 6] = [
    p(-2, 1),
    p(4, 2),
    p(-8, -11),
    p(-17, -40),
    p(-55, -52),
    p(-139, -54),
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-13, -10),
    p(16, -8),
    p(17, -3),
    p(23, -13),
    p(5, 23),
    p(6, 18),
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

    fn num_hanging_for_white(num: usize) -> SingleFeatureScore<Self::Score>;

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

    fn num_hanging_for_white(num: usize) -> SingleFeatureScore<Self::Score> {
        NUM_HANGING[num.min(5)]
    }

    fn king_zone_attack(attacking: ChessPieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }
}
