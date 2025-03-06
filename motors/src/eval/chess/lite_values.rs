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
        p( 131,  187),    p( 127,  185),    p( 119,  189),    p( 130,  169),    p( 117,  173),    p( 116,  177),    p(  79,  194),    p(  86,  193),
        p(  67,  123),    p(  66,  124),    p(  78,  120),    p(  86,  123),    p(  73,  123),    p( 122,  110),    p(  96,  130),    p(  93,  121),
        p(  54,  112),    p(  65,  108),    p(  65,  103),    p(  67,   98),    p(  82,   98),    p(  87,   94),    p(  79,  103),    p(  74,   95),
        p(  51,   99),    p(  58,  101),    p(  67,   94),    p(  76,   93),    p(  79,   93),    p(  79,   88),    p(  73,   92),    p(  62,   85),
        p(  47,   97),    p(  55,   92),    p(  59,   93),    p(  61,  100),    p(  69,   96),    p(  64,   92),    p(  73,   82),    p(  56,   85),
        p(  53,   98),    p(  55,   95),    p(  60,   98),    p(  59,  106),    p(  57,  108),    p(  74,   97),    p(  75,   83),    p(  59,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 176,  277),    p( 198,  309),    p( 215,  321),    p( 254,  310),    p( 284,  311),    p( 203,  306),    p( 213,  308),    p( 206,  260),
        p( 268,  310),    p( 283,  315),    p( 298,  307),    p( 302,  311),    p( 301,  307),    p( 313,  296),    p( 276,  312),    p( 272,  302),
        p( 286,  306),    p( 303,  303),    p( 306,  309),    p( 320,  312),    p( 337,  306),    p( 349,  295),    p( 291,  302),    p( 285,  307),
        p( 301,  314),    p( 308,  308),    p( 323,  312),    p( 326,  319),    p( 324,  316),    p( 319,  315),    p( 310,  311),    p( 319,  310),
        p( 299,  316),    p( 303,  306),    p( 312,  312),    p( 320,  315),    p( 318,  318),    p( 323,  302),    p( 321,  302),    p( 313,  311),
        p( 276,  303),    p( 281,  301),    p( 294,  296),    p( 300,  309),    p( 305,  306),    p( 293,  289),    p( 300,  292),    p( 293,  306),
        p( 270,  310),    p( 281,  312),    p( 283,  302),    p( 294,  306),    p( 298,  301),    p( 288,  299),    p( 295,  304),    p( 290,  320),
        p( 235,  311),    p( 279,  304),    p( 263,  306),    p( 284,  310),    p( 294,  307),    p( 285,  299),    p( 283,  308),    p( 266,  307),
    ],
    // bishop
    [
        p( 279,  309),    p( 254,  313),    p( 242,  305),    p( 222,  317),    p( 218,  313),    p( 229,  306),    p( 276,  302),    p( 254,  308),
        p( 281,  303),    p( 279,  302),    p( 290,  305),    p( 276,  303),    p( 286,  301),    p( 292,  299),    p( 268,  308),    p( 269,  302),
        p( 296,  309),    p( 307,  304),    p( 289,  304),    p( 306,  298),    p( 305,  299),    p( 333,  304),    p( 318,  300),    p( 317,  312),
        p( 284,  313),    p( 291,  306),    p( 303,  302),    p( 306,  305),    p( 306,  303),    p( 299,  304),    p( 297,  308),    p( 279,  309),
        p( 290,  307),    p( 282,  309),    p( 295,  303),    p( 308,  304),    p( 301,  300),    p( 299,  302),    p( 286,  303),    p( 308,  302),
        p( 297,  310),    p( 301,  304),    p( 299,  307),    p( 300,  304),    p( 306,  306),    p( 299,  298),    p( 306,  295),    p( 309,  298),
        p( 307,  310),    p( 305,  300),    p( 311,  300),    p( 297,  309),    p( 302,  304),    p( 305,  304),    p( 314,  294),    p( 309,  296),
        p( 290,  307),    p( 304,  308),    p( 305,  308),    p( 286,  310),    p( 302,  309),    p( 289,  312),    p( 302,  297),    p( 303,  291),
    ],
    // rook
    [
        p( 455,  549),    p( 446,  558),    p( 440,  565),    p( 439,  562),    p( 450,  558),    p( 468,  553),    p( 479,  551),    p( 488,  545),
        p( 438,  556),    p( 435,  561),    p( 445,  562),    p( 459,  553),    p( 445,  555),    p( 460,  550),    p( 470,  547),    p( 487,  537),
        p( 441,  550),    p( 458,  546),    p( 453,  547),    p( 452,  542),    p( 477,  532),    p( 485,  530),    p( 502,  529),    p( 479,  531),
        p( 438,  551),    p( 443,  546),    p( 443,  549),    p( 447,  543),    p( 453,  534),    p( 462,  531),    p( 461,  535),    p( 462,  530),
        p( 433,  548),    p( 431,  546),    p( 432,  546),    p( 436,  543),    p( 441,  539),    p( 435,  539),    p( 448,  533),    p( 442,  532),
        p( 429,  545),    p( 427,  542),    p( 429,  541),    p( 432,  540),    p( 436,  535),    p( 446,  527),    p( 460,  517),    p( 448,  521),
        p( 431,  540),    p( 434,  539),    p( 440,  540),    p( 442,  536),    p( 449,  530),    p( 455,  523),    p( 466,  516),    p( 438,  526),
        p( 441,  546),    p( 435,  539),    p( 435,  543),    p( 438,  537),    p( 444,  530),    p( 450,  532),    p( 448,  529),    p( 448,  532),
    ],
    // queen
    [
        p( 881,  959),    p( 885,  972),    p( 900,  985),    p( 922,  978),    p( 918,  982),    p( 939,  970),    p( 981,  924),    p( 930,  954),
        p( 889,  950),    p( 867,  978),    p( 870, 1004),    p( 860, 1023),    p( 868, 1033),    p( 908,  993),    p( 908,  980),    p( 948,  960),
        p( 894,  955),    p( 888,  971),    p( 889,  990),    p( 889, 1000),    p( 912, 1000),    p( 948,  987),    p( 955,  958),    p( 943,  965),
        p( 881,  967),    p( 889,  973),    p( 883,  983),    p( 885,  994),    p( 888, 1006),    p( 899,  997),    p( 907, 1000),    p( 914,  975),
        p( 892,  957),    p( 880,  977),    p( 888,  976),    p( 889,  992),    p( 891,  988),    p( 892,  990),    p( 904,  979),    p( 910,  973),
        p( 888,  947),    p( 895,  963),    p( 890,  976),    p( 888,  978),    p( 893,  985),    p( 900,  974),    p( 912,  960),    p( 909,  948),
        p( 889,  949),    p( 889,  956),    p( 895,  960),    p( 896,  972),    p( 897,  971),    p( 898,  954),    p( 909,  933),    p( 914,  909),
        p( 887,  945),    p( 884,  945),    p( 885,  955),    p( 892,  951),    p( 891,  949),    p( 882,  950),    p( 885,  941),    p( 897,  919),
    ],
    // king
    [
        p( 156,  -84),    p(  58,  -37),    p(  82,  -29),    p(   5,    3),    p(  34,  -10),    p(  22,   -1),    p(  74,   -9),    p( 235,  -89),
        p( -30,    2),    p( -84,   20),    p( -85,   27),    p( -25,   17),    p( -55,   24),    p( -84,   39),    p( -53,   25),    p(   6,    0),
        p( -47,   10),    p( -52,   14),    p( -89,   29),    p(-100,   37),    p( -67,   32),    p( -35,   24),    p( -81,   26),    p( -38,   11),
        p( -28,    2),    p(-105,   14),    p(-118,   30),    p(-140,   38),    p(-139,   36),    p(-119,   29),    p(-138,   18),    p(-108,   17),
        p( -44,   -2),    p(-119,    9),    p(-130,   26),    p(-154,   39),    p(-157,   37),    p(-132,   23),    p(-148,   13),    p(-120,   13),
        p( -35,    2),    p( -95,    4),    p(-123,   19),    p(-129,   28),    p(-127,   27),    p(-137,   19),    p(-113,    5),    p( -76,   10),
        p(  23,   -7),    p( -82,   -1),    p( -95,    8),    p(-113,   17),    p(-119,   18),    p(-103,    9),    p( -78,   -8),    p(   0,   -3),
        p(  55,  -24),    p(  44,  -36),    p(  43,  -23),    p( -19,   -3),    p(  32,  -20),    p( -15,   -6),    p(  36,  -30),    p(  66,  -35),
    ],
];

const BISHOP_PAIR: PhasedScore = p(22, 53);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(9, 19), p(10, 18), p(11, 6), p(7, -1), p(3, -10), p(-1, -19), p(-8, -28), p(-16, -42), p(-28, -55)];
const ROOK_OPEN_FILE: PhasedScore = p(9, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, 0);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(-1, 4);
const KING_OPEN_FILE: PhasedScore = p(-48, -1);
const KING_CLOSED_FILE: PhasedScore = p(14, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-8, 4), p(-1, 5), p(-3, 4), p(1, 3), p(3, 4), p(2, 7), p(4, 4), p(17, 1)],
    // Closed
    [p(0, 0), p(0, 0), p(15, -24), p(-16, 8), p(1, 10), p(2, 4), p(1, 6), p(1, 4)],
    // SemiOpen
    [p(0, 0), p(-15, 22), p(3, 16), p(1, 9), p(1, 8), p(3, 5), p(-1, 2), p(11, 4)],
    // SemiClosed
    [p(0, 0), p(9, -11), p(6, 6), p(3, -0), p(8, 1), p(4, 4), p(5, 4), p(2, 4)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-4, 5),    /*0b0000*/
    p(-14, 8),   /*0b0001*/
    p(-3, 8),    /*0b0010*/
    p(-10, 13),  /*0b0011*/
    p(-3, 3),    /*0b0100*/
    p(-25, -1),  /*0b0101*/
    p(-14, 5),   /*0b0110*/
    p(-19, -16), /*0b0111*/
    p(9, 10),    /*0b1000*/
    p(-2, 10),   /*0b1001*/
    p(3, 11),    /*0b1010*/
    p(-0, 9),    /*0b1011*/
    p(0, 4),     /*0b1100*/
    p(-23, 9),   /*0b1101*/
    p(-11, 4),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(3, 15),    /*0b10000*/
    p(3, 8),     /*0b10001*/
    p(20, 11),   /*0b10010*/
    p(-5, 6),    /*0b10011*/
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
    p(4, 13),    /*0b100001*/
    p(26, 3),    /*0b100010*/
    p(7, -1),    /*0b100011*/
    p(-5, 2),    /*0b100100*/
    p(-21, -7),  /*0b100101*/
    p(-20, 15),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(24, 4),    /*0b101000*/
    p(-0, 17),   /*0b101001*/
    p(23, -3),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-3, 7),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(16, 18),   /*0b110000*/
    p(25, 12),   /*0b110001*/
    p(34, 9),    /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(12, 28),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(26, 15),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(8, -1),    /*0b111111*/
    p(-15, -3),  /*0b00*/
    p(9, -17),   /*0b01*/
    p(38, -9),   /*0b10*/
    p(20, -41),  /*0b11*/
    p(44, -9),   /*0b100*/
    p(5, -21),   /*0b101*/
    p(69, -41),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(63, -12),  /*0b1000*/
    p(19, -34),  /*0b1001*/
    p(82, -56),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(57, -19),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(21, -13),  /*0b1111*/
    p(20, -2),   /*0b00*/
    p(34, -13),  /*0b01*/
    p(26, -17),  /*0b10*/
    p(22, -42),  /*0b11*/
    p(37, -9),   /*0b100*/
    p(56, -20),  /*0b101*/
    p(25, -23),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(38, -2),   /*0b1000*/
    p(53, -17),  /*0b1001*/
    p(50, -42),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(43, -21),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(20, -44),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  31,   87),    p(  27,   85),    p(  19,   89),    p(  30,   69),    p(  17,   73),    p(  16,   77),    p( -21,   94),    p( -14,   93),
        p(  38,  124),    p(  47,  123),    p(  36,  100),    p(  20,   69),    p(  34,   68),    p(  13,   95),    p(  -1,  104),    p( -33,  125),
        p(  22,   74),    p(  17,   71),    p(  22,   54),    p(  16,   43),    p(  -1,   46),    p(   7,   58),    p( -10,   76),    p( -11,   79),
        p(   7,   46),    p(  -3,   44),    p( -15,   34),    p( -10,   24),    p( -17,   28),    p( -11,   39),    p( -18,   55),    p( -12,   51),
        p(   1,   14),    p( -13,   23),    p( -16,   17),    p( -15,    8),    p( -15,   13),    p(  -7,   17),    p( -14,   37),    p(  10,   17),
        p(  -5,   15),    p(  -2,   20),    p(  -9,   16),    p(  -8,    4),    p(   7,   -0),    p(   8,    7),    p(  13,   18),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-10, -10);
const DOUBLED_PAWN: PhasedScore = p(-7, -21);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 11), p(8, 13), p(14, 19), p(9, 7), p(-3, 15), p(-45, 6)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(39, 8), p(39, 35), p(54, -8), p(37, -32), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-50, -71),
        p(-29, -31),
        p(-15, -8),
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
        p(-32, -56),
        p(-20, -39),
        p(-8, -23),
        p(-1, -11),
        p(6, -1),
        p(12, 8),
        p(17, 13),
        p(20, 18),
        p(23, 22),
        p(30, 24),
        p(36, 24),
        p(45, 26),
        p(45, 33),
        p(57, 28),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-78, 1),
        p(-69, 18),
        p(-64, 27),
        p(-62, 36),
        p(-60, 42),
        p(-54, 45),
        p(-50, 48),
        p(-44, 51),
        p(-39, 55),
        p(-34, 58),
        p(-30, 61),
        p(-25, 65),
        p(-16, 66),
        p(-8, 63),
        p(-7, 64),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-18, -53),
        p(-18, 3),
        p(-23, 54),
        p(-19, 72),
        p(-18, 91),
        p(-14, 95),
        p(-11, 106),
        p(-8, 113),
        p(-5, 116),
        p(-2, 118),
        p(-0, 121),
        p(3, 123),
        p(6, 124),
        p(7, 128),
        p(9, 130),
        p(12, 133),
        p(12, 141),
        p(14, 141),
        p(23, 139),
        p(36, 133),
        p(39, 135),
        p(82, 112),
        p(80, 116),
        p(104, 98),
        p(197, 64),
        p(239, 26),
        p(273, 13),
        p(332, -25),
    ],
    [
        p(-96, 8),
        p(-58, -4),
        p(-28, -6),
        p(2, -4),
        p(34, -3),
        p(57, -3),
        p(86, 3),
        p(112, 2),
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
    [p(-9, 7), p(0, 0), p(23, 19), p(49, -12), p(20, -34), p(0, 0)],
    [p(-3, 11), p(20, 23), p(-27, 22), p(31, 5), p(29, 55), p(0, 0)],
    [p(2, 11), p(14, 14), p(20, 12), p(-14, 0), p(45, -6), p(0, 0)],
    [p(-2, 4), p(0, 6), p(-2, 22), p(0, 1), p(0, 0), p(0, 0)],
    [p(71, 28), p(-36, 18), p(-10, 18), p(-18, 7), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 7), p(8, 7), p(6, 11), p(13, 7), p(7, 21), p(11, 6)],
    [p(-0, 7), p(11, 22), p(-138, -43), p(8, 15), p(4, 9), p(4, 8)],
    [p(5, 0), p(18, 5), p(15, 10), p(3, 3), p(2, 7), p(14, -2)],
    [p(1, -1), p(6, 3), p(6, -6), p(1, 19), p(-65, -251), p(-1, -7)],
    [p(63, -1), p(41, 7), p(46, 0), p(28, 4), p(41, -13), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-21, -18), p(19, -10), p(12, -4), p(13, -12), p(-2, 13), p(-14, 12)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(13, 19), p(33, -0), p(5, 31)];

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
