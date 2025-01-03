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
        p( 133,  186),    p( 130,  185),    p( 121,  188),    p( 133,  169),    p( 120,  173),    p( 119,  176),    p(  82,  194),    p(  89,  193),
        p(  67,  123),    p(  66,  124),    p(  77,  120),    p(  85,  123),    p(  73,  123),    p( 122,  110),    p(  96,  130),    p(  93,  121),
        p(  55,  112),    p(  66,  108),    p(  65,  104),    p(  67,   98),    p(  82,   98),    p(  87,   94),    p(  79,  103),    p(  74,   95),
        p(  51,   99),    p(  58,  102),    p(  67,   94),    p(  76,   93),    p(  79,   93),    p(  78,   88),    p(  73,   92),    p(  62,   86),
        p(  46,   97),    p(  55,   92),    p(  59,   93),    p(  61,   99),    p(  69,   96),    p(  63,   92),    p(  72,   82),    p(  55,   85),
        p(  52,   99),    p(  55,   95),    p(  61,   97),    p(  60,  105),    p(  57,  107),    p(  73,   98),    p(  75,   83),    p(  58,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 176,  277),    p( 197,  309),    p( 214,  321),    p( 252,  310),    p( 282,  312),    p( 198,  307),    p( 212,  308),    p( 205,  260),
        p( 269,  310),    p( 283,  316),    p( 298,  307),    p( 302,  311),    p( 301,  307),    p( 315,  296),    p( 276,  312),    p( 272,  302),
        p( 286,  306),    p( 303,  303),    p( 305,  309),    p( 320,  313),    p( 336,  306),    p( 349,  296),    p( 291,  302),    p( 285,  307),
        p( 301,  314),    p( 308,  308),    p( 322,  312),    p( 325,  319),    p( 323,  317),    p( 318,  316),    p( 309,  311),    p( 318,  310),
        p( 298,  316),    p( 302,  306),    p( 311,  312),    p( 319,  315),    p( 317,  318),    p( 322,  302),    p( 320,  303),    p( 312,  311),
        p( 275,  303),    p( 280,  301),    p( 293,  296),    p( 299,  309),    p( 304,  307),    p( 292,  289),    p( 300,  292),    p( 292,  306),
        p( 269,  311),    p( 280,  313),    p( 283,  303),    p( 293,  306),    p( 297,  301),    p( 287,  299),    p( 294,  305),    p( 289,  320),
        p( 239,  310),    p( 282,  303),    p( 266,  304),    p( 286,  309),    p( 295,  306),    p( 291,  296),    p( 288,  306),    p( 265,  307),
    ],
    // bishop
    [
        p( 276,  310),    p( 253,  314),    p( 239,  306),    p( 223,  317),    p( 218,  313),    p( 224,  308),    p( 273,  303),    p( 252,  309),
        p( 281,  303),    p( 278,  303),    p( 290,  305),    p( 277,  303),    p( 287,  301),    p( 293,  299),    p( 268,  308),    p( 270,  302),
        p( 295,  309),    p( 306,  304),    p( 291,  304),    p( 306,  298),    p( 305,  299),    p( 335,  304),    p( 317,  300),    p( 316,  313),
        p( 285,  313),    p( 291,  307),    p( 303,  302),    p( 307,  305),    p( 307,  303),    p( 299,  304),    p( 296,  309),    p( 279,  310),
        p( 289,  308),    p( 283,  309),    p( 295,  303),    p( 309,  305),    p( 302,  301),    p( 298,  303),    p( 285,  304),    p( 308,  302),
        p( 296,  311),    p( 300,  304),    p( 300,  307),    p( 300,  304),    p( 306,  307),    p( 299,  298),    p( 305,  296),    p( 307,  299),
        p( 307,  309),    p( 303,  300),    p( 309,  300),    p( 298,  309),    p( 300,  305),    p( 304,  304),    p( 312,  295),    p( 308,  297),
        p( 297,  305),    p( 310,  306),    p( 308,  307),    p( 290,  309),    p( 305,  308),    p( 294,  309),    p( 305,  296),    p( 301,  292),
    ],
    // rook
    [
        p( 457,  548),    p( 446,  557),    p( 440,  564),    p( 438,  561),    p( 449,  558),    p( 469,  552),    p( 481,  549),    p( 491,  542),
        p( 440,  555),    p( 438,  561),    p( 447,  561),    p( 463,  551),    p( 449,  554),    p( 465,  548),    p( 473,  546),    p( 489,  536),
        p( 445,  549),    p( 462,  544),    p( 457,  545),    p( 456,  540),    p( 483,  530),    p( 491,  527),    p( 508,  527),    p( 484,  529),
        p( 442,  548),    p( 448,  544),    p( 447,  546),    p( 453,  540),    p( 457,  531),    p( 465,  529),    p( 465,  533),    p( 466,  528),
        p( 437,  545),    p( 435,  543),    p( 435,  543),    p( 441,  540),    p( 447,  536),    p( 438,  537),    p( 451,  532),    p( 446,  529),
        p( 433,  541),    p( 432,  538),    p( 434,  538),    p( 436,  538),    p( 442,  532),    p( 449,  526),    p( 465,  515),    p( 452,  518),
        p( 435,  536),    p( 438,  535),    p( 444,  537),    p( 446,  534),    p( 453,  528),    p( 461,  521),    p( 469,  516),    p( 441,  524),
        p( 444,  540),    p( 440,  536),    p( 441,  540),    p( 445,  534),    p( 451,  527),    p( 452,  533),    p( 449,  530),    p( 446,  532),
    ],
    // queen
    [
        p( 877,  959),    p( 881,  974),    p( 895,  987),    p( 916,  980),    p( 913,  984),    p( 934,  971),    p( 979,  924),    p( 924,  956),
        p( 888,  950),    p( 863,  979),    p( 865, 1006),    p( 857, 1023),    p( 865, 1034),    p( 906,  994),    p( 906,  979),    p( 947,  958),
        p( 893,  956),    p( 885,  971),    p( 885,  992),    p( 886, 1001),    p( 909, 1002),    p( 946,  987),    p( 954,  957),    p( 941,  965),
        p( 879,  968),    p( 885,  975),    p( 879,  985),    p( 880,  996),    p( 884, 1008),    p( 896,  999),    p( 905, 1001),    p( 912,  976),
        p( 890,  958),    p( 877,  978),    p( 884,  978),    p( 884,  995),    p( 887,  991),    p( 888,  992),    p( 901,  980),    p( 908,  974),
        p( 886,  948),    p( 892,  964),    p( 887,  978),    p( 884,  981),    p( 889,  988),    p( 896,  976),    p( 909,  960),    p( 907,  948),
        p( 886,  949),    p( 886,  958),    p( 893,  960),    p( 892,  975),    p( 894,  973),    p( 895,  956),    p( 906,  935),    p( 914,  907),
        p( 872,  951),    p( 884,  938),    p( 885,  951),    p( 893,  952),    p( 896,  942),    p( 882,  946),    p( 885,  937),    p( 888,  922),
    ],
    // king
    [
        p( 156,  -84),    p(  59,  -37),    p(  84,  -29),    p(   7,    3),    p(  36,  -10),    p(  22,   -0),    p(  74,   -9),    p( 233,  -89),
        p( -30,    2),    p( -81,   20),    p( -83,   27),    p( -21,   17),    p( -52,   24),    p( -82,   39),    p( -51,   25),    p(   6,   -0),
        p( -46,   10),    p( -49,   14),    p( -86,   29),    p( -96,   37),    p( -64,   32),    p( -32,   24),    p( -79,   26),    p( -39,   11),
        p( -27,    2),    p(-102,   13),    p(-115,   29),    p(-137,   38),    p(-136,   36),    p(-116,   28),    p(-135,   18),    p(-109,   17),
        p( -42,   -2),    p(-116,    8),    p(-127,   25),    p(-152,   39),    p(-154,   37),    p(-129,   23),    p(-145,   13),    p(-121,   12),
        p( -34,    2),    p( -93,    4),    p(-121,   19),    p(-128,   27),    p(-124,   27),    p(-135,   19),    p(-110,    5),    p( -76,   10),
        p(  25,   -8),    p( -78,   -2),    p( -91,    8),    p(-111,   17),    p(-115,   17),    p(-100,    8),    p( -73,   -9),    p(  -1,   -3),
        p(  54,  -24),    p(  43,  -35),    p(  36,  -20),    p( -24,   -1),    p(  29,  -18),    p( -19,   -5),    p(  35,  -29),    p(  62,  -33),
    ],
];

const BISHOP_PAIR: PhasedScore = p(22, 53);
const BAD_BISHOP: [PhasedScore; 9] = [
    p(9, 19),
    p(10, 17),
    p(11, 6),
    p(7, -2),
    p(3, -10),
    p(-1, -20),
    p(-8, -28),
    p(-16, -42),
    p(-28, -52),
];
const ROOK_OPEN_FILE: PhasedScore = p(13, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, 1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(2, 5);
const KING_OPEN_FILE: PhasedScore = p(-49, -1);
const KING_CLOSED_FILE: PhasedScore = p(14, -14);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 3), p(0, 5), p(-2, 4), p(2, 2), p(2, 4), p(3, 7), p(5, 4), p(18, 1)],
    // Closed
    [p(0, 0), p(0, 0), p(16, -24), p(-16, 9), p(-0, 10), p(2, 4), p(-1, 7), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-16, 22), p(3, 16), p(1, 9), p(0, 8), p(4, 4), p(-1, 2), p(10, 5)],
    // SemiClosed
    [p(0, 0), p(11, -13), p(7, 5), p(3, 0), p(7, 1), p(3, 4), p(4, 5), p(1, 4)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-4, 5),    /*0b0000*/
    p(-15, 8),   /*0b0001*/
    p(-3, 8),    /*0b0010*/
    p(-11, 13),  /*0b0011*/
    p(-4, 3),    /*0b0100*/
    p(-26, -1),  /*0b0101*/
    p(-14, 5),   /*0b0110*/
    p(-20, -16), /*0b0111*/
    p(9, 10),    /*0b1000*/
    p(-2, 10),   /*0b1001*/
    p(3, 11),    /*0b1010*/
    p(-3, 10),   /*0b1011*/
    p(-0, 4),    /*0b1100*/
    p(-23, 9),   /*0b1101*/
    p(-12, 4),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(4, 15),    /*0b10000*/
    p(2, 8),     /*0b10001*/
    p(20, 10),   /*0b10010*/
    p(-6, 6),    /*0b10011*/
    p(-5, 6),    /*0b10100*/
    p(13, 15),   /*0b10101*/
    p(-24, 1),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(16, 29),   /*0b11000*/
    p(29, 23),   /*0b11001*/
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
    p(-22, 15),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(24, 4),    /*0b101000*/
    p(-0, 17),   /*0b101001*/
    p(22, -2),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-4, 7),    /*0b101100*/
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
    p(27, 15),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(8, -1),    /*0b111111*/
    p(-14, -3),  /*0b00*/
    p(10, -17),  /*0b01*/
    p(38, -8),   /*0b10*/
    p(21, -40),  /*0b11*/
    p(47, -10),  /*0b100*/
    p(7, -22),   /*0b101*/
    p(69, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(64, -12),  /*0b1000*/
    p(20, -33),  /*0b1001*/
    p(79, -54),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(60, -19),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(21, -10),  /*0b1111*/
    p(23, -2),   /*0b00*/
    p(35, -12),  /*0b01*/
    p(28, -17),  /*0b10*/
    p(23, -40),  /*0b11*/
    p(40, -9),   /*0b100*/
    p(57, -19),  /*0b101*/
    p(27, -22),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(41, -2),   /*0b1000*/
    p(55, -17),  /*0b1001*/
    p(53, -42),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(46, -21),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(22, -43),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  33,   86),    p(  30,   85),    p(  21,   88),    p(  33,   69),    p(  20,   73),    p(  19,   76),    p( -18,   94),    p( -11,   93),
        p(  39,  123),    p(  48,  123),    p(  37,  100),    p(  21,   69),    p(  34,   68),    p(  15,   95),    p(  -1,  104),    p( -32,  125),
        p(  23,   74),    p(  17,   71),    p(  22,   54),    p(  17,   43),    p(  -1,   46),    p(   7,   58),    p( -10,   76),    p( -10,   79),
        p(   7,   46),    p(  -2,   44),    p( -15,   34),    p( -10,   24),    p( -17,   29),    p( -10,   39),    p( -18,   55),    p( -11,   51),
        p(   1,   14),    p( -12,   23),    p( -15,   17),    p( -16,    8),    p( -15,   13),    p(  -7,   17),    p( -14,   38),    p(  10,   17),
        p(  -5,   15),    p(  -2,   20),    p(  -9,   16),    p(  -8,    4),    p(   5,    1),    p(   7,    7),    p(  13,   18),    p(   7,   14),
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
    p(-45, 6),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(38, 9), p(39, 35), p(51, -8), p(35, -34), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-49, -71),
        p(-28, -32),
        p(-15, -9),
        p(-5, 5),
        p(3, 16),
        p(10, 27),
        p(19, 29),
        p(26, 32),
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
        p(-19, -38),
        p(-8, -23),
        p(-0, -11),
        p(7, -1),
        p(12, 8),
        p(16, 13),
        p(20, 18),
        p(22, 22),
        p(29, 24),
        p(35, 24),
        p(43, 26),
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
        p(-78, 15),
        p(-70, 29),
        p(-65, 34),
        p(-62, 38),
        p(-62, 45),
        p(-56, 50),
        p(-53, 53),
        p(-49, 56),
        p(-45, 60),
        p(-41, 63),
        p(-37, 65),
        p(-36, 70),
        p(-28, 70),
        p(-19, 67),
        p(-18, 68),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-28, -46),
        p(-28, 9),
        p(-32, 58),
        p(-27, 74),
        p(-25, 93),
        p(-20, 97),
        p(-16, 107),
        p(-13, 113),
        p(-9, 117),
        p(-6, 118),
        p(-3, 121),
        p(1, 123),
        p(4, 124),
        p(5, 128),
        p(8, 130),
        p(11, 133),
        p(12, 140),
        p(14, 140),
        p(23, 138),
        p(37, 132),
        p(41, 134),
        p(84, 110),
        p(84, 113),
        p(107, 95),
        p(201, 61),
        p(245, 21),
        p(276, 10),
        p(332, -27),
    ],
    [
        p(-93, 8),
        p(-57, -5),
        p(-27, -6),
        p(4, -4),
        p(35, -2),
        p(58, -3),
        p(87, 3),
        p(113, 2),
        p(162, -15),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-8, 7),
        p(0, 0),
        p(23, 19),
        p(49, -12),
        p(20, -33),
        p(0, 0),
    ],
    [p(-3, 11), p(20, 23), p(0, 0), p(31, 5), p(31, 53), p(0, 0)],
    [p(-3, 12), p(10, 15), p(17, 12), p(0, 0), p(45, -5), p(0, 0)],
    [p(-2, 5), p(2, 5), p(-0, 21), p(1, 1), p(0, 0), p(0, 0)],
    [
        p(71, 28),
        p(-35, 18),
        p(-9, 18),
        p(-21, 7),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 7), p(8, 7), p(6, 11), p(13, 7), p(7, 20), p(12, 6)],
    [
        p(1, 6),
        p(11, 22),
        p(-129, -28),
        p(8, 15),
        p(9, 20),
        p(3, 8),
    ],
    [p(2, 1), p(14, 5), p(9, 11), p(11, 8), p(11, 20), p(21, -6)],
    [
        p(2, -2),
        p(9, 1),
        p(7, -5),
        p(4, 15),
        p(-61, -253),
        p(5, -11),
    ],
    [p(64, -1), p(41, 6), p(46, 0), p(25, 5), p(37, -11), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-21, -18),
    p(19, -10),
    p(11, -4),
    p(10, -4),
    p(-1, 12),
    p(-14, 11),
];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(13, 19), p(35, -1), p(5, 31)];
const ROOK_KING: PhasedScore = p(8, -11);

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

    fn rook_king() -> SingleFeatureScore<Self::Score>;
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

    fn king_zone_attack(attacking: ChessPieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }

    fn can_give_check(piece: ChessPieceType) -> PhasedScore {
        CAN_GIVE_CHECK[piece as usize]
    }

    fn rook_king() -> SingleFeatureScore<Self::Score> {
        ROOK_KING
    }
}
