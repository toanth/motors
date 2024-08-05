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
use gears::games::chess::pieces::{ChessPieceType, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::chess::ChessColor;
use gears::games::chess::ChessColor::White;
use gears::score::{p, PhasedScore};
use std::fmt::Debug;

#[rustfmt::skip]
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
        p( 132,  161),    p( 129,  162),    p( 117,  167),    p( 127,  152),    p( 117,  157),    p( 118,  160),    p(  85,  174),    p(  93,  169),
        p(  65,  122),    p(  63,  122),    p(  75,  115),    p(  83,  120),    p(  68,  119),    p( 118,  105),    p(  92,  126),    p(  87,  118),
        p(  52,  113),    p(  64,  107),    p(  62,  101),    p(  67,   94),    p(  83,   95),    p(  84,   91),    p(  78,  100),    p(  72,   94),
        p(  48,   99),    p(  56,  101),    p(  64,   94),    p(  74,   92),    p(  77,   90),    p(  78,   86),    p(  72,   90),    p(  60,   85),
        p(  43,   97),    p(  52,   94),    p(  56,   93),    p(  60,   98),    p(  68,   95),    p(  62,   91),    p(  70,   83),    p(  54,   85),
        p(  50,   98),    p(  52,   96),    p(  58,   98),    p(  58,  104),    p(  55,  107),    p(  73,   98),    p(  73,   84),    p(  55,   87),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 183,  273),    p( 209,  304),    p( 243,  317),    p( 268,  306),    p( 299,  308),    p( 214,  302),    p( 232,  301),    p( 213,  254),
        p( 275,  305),    p( 287,  314),    p( 300,  310),    p( 313,  313),    p( 304,  310),    p( 327,  299),    p( 286,  310),    p( 290,  295),
        p( 291,  305),    p( 302,  307),    p( 320,  316),    p( 321,  320),    p( 337,  314),    p( 361,  303),    p( 314,  303),    p( 308,  300),
        p( 304,  312),    p( 310,  310),    p( 317,  321),    p( 343,  323),    p( 320,  324),    p( 334,  321),    p( 315,  312),    p( 333,  305),
        p( 300,  315),    p( 299,  309),    p( 305,  321),    p( 312,  325),    p( 319,  326),    p( 316,  313),    p( 327,  305),    p( 316,  311),
        p( 275,  302),    p( 276,  304),    p( 284,  305),    p( 290,  318),    p( 298,  315),    p( 283,  299),    p( 298,  297),    p( 293,  306),
        p( 271,  307),    p( 281,  311),    p( 277,  306),    p( 289,  310),    p( 292,  304),    p( 284,  302),    p( 294,  302),    p( 290,  316),
        p( 242,  304),    p( 282,  301),    p( 266,  303),    p( 285,  309),    p( 296,  306),    p( 291,  296),    p( 289,  302),    p( 266,  305),
    ],
    // bishop
    [
        p( 279,  317),    p( 256,  315),    p( 249,  307),    p( 225,  315),    p( 222,  316),    p( 226,  306),    p( 282,  305),    p( 251,  309),
        p( 280,  303),    p( 285,  306),    p( 288,  307),    p( 282,  308),    p( 284,  304),    p( 294,  304),    p( 272,  308),    p( 274,  304),
        p( 298,  309),    p( 303,  305),    p( 296,  310),    p( 303,  303),    p( 307,  306),    p( 333,  309),    p( 318,  305),    p( 312,  311),
        p( 282,  310),    p( 298,  310),    p( 300,  305),    p( 317,  312),    p( 311,  307),    p( 307,  309),    p( 302,  308),    p( 282,  312),
        p( 292,  308),    p( 281,  311),    p( 300,  310),    p( 314,  308),    p( 313,  308),    p( 298,  307),    p( 291,  309),    p( 311,  300),
        p( 293,  308),    p( 304,  310),    p( 301,  310),    p( 304,  310),    p( 307,  311),    p( 304,  308),    p( 306,  302),    p( 310,  301),
        p( 309,  312),    p( 303,  301),    p( 311,  303),    p( 296,  310),    p( 302,  309),    p( 303,  306),    p( 313,  302),    p( 302,  300),
        p( 294,  305),    p( 314,  311),    p( 306,  307),    p( 290,  312),    p( 303,  309),    p( 295,  314),    p( 303,  300),    p( 301,  297),
    ],
    // rook
    [
        p( 458,  552),    p( 449,  561),    p( 446,  568),    p( 444,  565),    p( 457,  561),    p( 477,  555),    p( 483,  555),    p( 494,  547),
        p( 432,  558),    p( 429,  564),    p( 438,  564),    p( 454,  554),    p( 444,  557),    p( 464,  552),    p( 475,  548),    p( 489,  539),
        p( 437,  555),    p( 456,  550),    p( 453,  552),    p( 457,  548),    p( 485,  536),    p( 493,  532),    p( 516,  530),    p( 487,  532),
        p( 435,  555),    p( 442,  551),    p( 443,  553),    p( 448,  548),    p( 457,  540),    p( 466,  535),    p( 473,  537),    p( 469,  532),
        p( 430,  551),    p( 429,  549),    p( 430,  551),    p( 436,  548),    p( 443,  543),    p( 437,  542),    p( 457,  535),    p( 446,  534),
        p( 427,  547),    p( 426,  545),    p( 429,  544),    p( 431,  545),    p( 439,  538),    p( 447,  531),    p( 470,  518),    p( 451,  523),
        p( 429,  541),    p( 433,  541),    p( 439,  543),    p( 442,  541),    p( 449,  534),    p( 464,  524),    p( 472,  519),    p( 440,  529),
        p( 439,  544),    p( 435,  541),    p( 436,  546),    p( 441,  541),    p( 448,  535),    p( 454,  535),    p( 452,  532),    p( 446,  533),
    ],
    // queen
    [
        p( 873,  971),    p( 876,  985),    p( 890,  998),    p( 907,  994),    p( 905,  998),    p( 926,  985),    p( 973,  935),    p( 920,  966),
        p( 884,  961),    p( 858,  995),    p( 860, 1023),    p( 852, 1040),    p( 859, 1052),    p( 899, 1013),    p( 901,  994),    p( 944,  970),
        p( 892,  966),    p( 883,  987),    p( 883, 1011),    p( 880, 1022),    p( 902, 1024),    p( 942, 1008),    p( 949,  977),    p( 937,  982),
        p( 876,  982),    p( 882,  993),    p( 875, 1004),    p( 873, 1019),    p( 878, 1030),    p( 891, 1020),    p( 900, 1018),    p( 907,  994),
        p( 887,  973),    p( 873,  994),    p( 879,  997),    p( 879, 1014),    p( 880, 1013),    p( 882, 1012),    p( 897,  996),    p( 904,  988),
        p( 882,  959),    p( 887,  977),    p( 880,  994),    p( 878,  997),    p( 883, 1004),    p( 889,  994),    p( 904,  975),    p( 903,  962),
        p( 884,  958),    p( 882,  967),    p( 889,  970),    p( 888,  983),    p( 889,  983),    p( 891,  966),    p( 900,  943),    p( 910,  917),
        p( 871,  955),    p( 882,  944),    p( 882,  959),    p( 890,  960),    p( 892,  954),    p( 880,  954),    p( 881,  943),    p( 884,  933),
    ],
    // king
    [
        p( 168,  -75),    p(  74,  -23),    p(  97,  -16),    p(  27,   12),    p(  47,   -0),    p(  24,   13),    p(  82,   -0),    p( 226,  -77),
        p(  -6,   18),    p( -40,   45),    p( -52,   54),    p(  14,   43),    p( -26,   53),    p( -43,   65),    p( -12,   51),    p(  21,   19),
        p( -40,   24),    p( -24,   37),    p( -72,   52),    p( -83,   60),    p( -47,   55),    p( -11,   48),    p( -44,   48),    p( -18,   26),
        p( -25,   13),    p( -78,   30),    p(-105,   45),    p(-131,   53),    p(-128,   51),    p(-108,   45),    p(-105,   36),    p( -97,   27),
        p( -41,    2),    p(-103,   20),    p(-120,   34),    p(-146,   46),    p(-152,   44),    p(-127,   32),    p(-136,   25),    p(-116,   20),
        p( -33,   -0),    p( -80,    9),    p(-114,   22),    p(-124,   31),    p(-120,   30),    p(-133,   23),    p(-102,   10),    p( -74,   12),
        p(  27,  -12),    p( -64,    0),    p( -77,    7),    p( -99,   16),    p(-105,   17),    p( -89,    9),    p( -58,   -5),    p(   4,   -4),
        p(  41,  -41),    p(  42,  -48),    p(  37,  -39),    p( -24,  -21),    p(  29,  -38),    p( -21,  -23),    p(  34,  -44),    p(  58,  -49),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  32,   61),    p(  29,   62),    p(  17,   67),    p(  27,   52),    p(  17,   57),    p(  18,   60),    p( -15,   74),    p(  -7,   69),
        p(  36,   79),    p(  39,   87),    p(  27,   72),    p(  18,   50),    p(  38,   54),    p(  18,   77),    p(   5,   78),    p( -22,   92),
        p(  13,   38),    p(   5,   45),    p(  18,   39),    p(  17,   37),    p(   1,   38),    p(  10,   46),    p( -11,   59),    p(  -9,   57),
        p(  -4,   20),    p( -11,   30),    p( -15,   29),    p(  -7,   22),    p( -14,   26),    p(  -8,   31),    p( -19,   45),    p( -11,   38),
        p(  -4,   -1),    p( -14,   19),    p( -14,   16),    p( -15,    8),    p( -12,   11),    p(  -7,   14),    p( -14,   32),    p(   9,   12),
        p(  -8,   -0),    p(  -2,   16),    p(  -9,   17),    p(  -8,    4),    p(   6,   -0),    p(   8,    6),    p(  12,   17),    p(   7,    9),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-11, -10);
const DOUBLED_PAWN: PhasedScore = p(-7, -21);
const PASSED_PAWN_OUTSIDE_SQUARE_RULE: PhasedScore = p(-1, 62);

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(16, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(0, 2);
const KING_OPEN_FILE: PhasedScore = p(-56, -2);
const KING_CLOSED_FILE: PhasedScore = p(16, -14);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 5);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 7), p(-3, 8), p(-3, 10), p(3, 8), p(4, 11), p(5, 12), p(10, 12), p(21, 7), ],
    // Closed
    [p(0, 0), p(0, 0), p(13, -33), p(-15, 9), p(0, 14), p(3, 5), p(2, 11), p(-0, 7), ],
    // SemiOpen
    [p(0, 0), p(-18, 24), p(1, 22), p(1, 16), p(-1, 20), p(3, 15), p(1, 12), p(12, 12), ],
    // SemiClosed
    [p(0, 0), p(11, -13), p(8, 7), p(5, 2), p(8, 5), p(3, 5), p(8, 7), p(3, 5), ],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-8, 6),    /*0b0000*/
    p(-16, 11),  /*0b0001*/
    p(-3, 6),    /*0b0010*/
    p(-10, 12),  /*0b0011*/
    p(-5, 6),    /*0b0100*/
    p(-27, 4),   /*0b0101*/
    p(-14, 4),   /*0b0110*/
    p(-18, -19), /*0b0111*/
    p(5, 12),    /*0b1000*/
    p(-6, 11),   /*0b1001*/
    p(1, 9),     /*0b1010*/
    p(-3, 9),    /*0b1011*/
    p(-2, 9),    /*0b1100*/
    p(-25, 11),  /*0b1101*/
    p(-12, 3),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(1, 18),    /*0b10000*/
    p(4, 12),    /*0b10001*/
    p(20, 12),   /*0b10010*/
    p(-3, 8),    /*0b10011*/
    p(-6, 8),    /*0b10100*/
    p(13, 17),   /*0b10101*/
    p(-21, -1),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(11, 36),   /*0b11000*/
    p(30, 27),   /*0b11001*/
    p(41, 40),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(15, 15),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(13, 11),   /*0b100000*/
    p(3, 17),    /*0b100001*/
    p(25, 3),    /*0b100010*/
    p(6, -0),    /*0b100011*/
    p(-11, 4),   /*0b100100*/
    p(-23, -7),  /*0b100101*/
    p(-25, 14),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(18, 6),    /*0b101000*/
    p(-4, 20),   /*0b101001*/
    p(19, -2),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-8, 9),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(13, 23),   /*0b110000*/
    p(25, 18),   /*0b110001*/
    p(32, 12),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(7, 31),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(22, 21),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(6, -3),    /*0b111111*/
    p(-19, -7),  /*0b00*/
    p(9, -14),   /*0b01*/
    p(37, -4),   /*0b10*/
    p(24, -41),  /*0b11*/
    p(45, -6),   /*0b100*/
    p(-7, -15),  /*0b101*/
    p(73, -38),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(54, -7),   /*0b1000*/
    p(17, -29),  /*0b1001*/
    p(79, -54),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(52, -4),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(12, 10),   /*0b1111*/
    p(15, -8),   /*0b00*/
    p(32, -16),  /*0b01*/
    p(24, -23),  /*0b10*/
    p(23, -50),  /*0b11*/
    p(30, -11),  /*0b100*/
    p(51, -22),  /*0b101*/
    p(22, -28),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(35, -7),   /*0b1000*/
    p(53, -21),  /*0b1001*/
    p(49, -47),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(38, -22),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(21, -51),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 7), p(2, 9), p(10, 13), p(9, 9), p(-4, 18), p(-46, 10)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(37, 12),
    p(41, 38),
    p(50, -5),
    p(36, -32),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-58, -58),
        p(-35, -19),
        p(-19, 3),
        p(-7, 15),
        p(3, 24),
        p(13, 32),
        p(24, 32),
        p(34, 31),
        p(42, 26),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-4, -14),
        p(3, -3),
        p(9, 7),
        p(13, 15),
        p(16, 19),
        p(18, 23),
        p(19, 27),
        p(25, 28),
        p(28, 26),
        p(36, 28),
        p(29, 36),
        p(41, 28),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-75, 16),
        p(-66, 30),
        p(-61, 35),
        p(-58, 40),
        p(-59, 47),
        p(-53, 51),
        p(-50, 55),
        p(-46, 57),
        p(-42, 61),
        p(-39, 65),
        p(-33, 67),
        p(-30, 71),
        p(-21, 70),
        p(-8, 67),
        p(-5, 68),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-36, -27),
        p(-37, 27),
        p(-40, 76),
        p(-35, 94),
        p(-32, 111),
        p(-27, 116),
        p(-23, 126),
        p(-20, 133),
        p(-15, 137),
        p(-12, 139),
        p(-9, 142),
        p(-5, 145),
        p(-2, 146),
        p(-1, 151),
        p(2, 152),
        p(5, 154),
        p(6, 160),
        p(9, 159),
        p(18, 156),
        p(32, 149),
        p(37, 147),
        p(80, 124),
        p(79, 125),
        p(106, 103),
        p(193, 72),
        p(242, 27),
        p(278, 9),
        p(344, -35),
    ],
    [
        p(-78, 37),
        p(-48, 13),
        p(-24, 6),
        p(1, 2),
        p(26, -1),
        p(44, -7),
        p(65, -6),
        p(84, -11),
        p(129, -33),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-11, 12),
        p(-6, -3),
        p(23, 19),
        p(48, -10),
        p(21, -40),
        p(0, 0),
    ],
    [p(-1, 13), p(18, 22), p(-2, 9), p(28, 6), p(27, 61), p(0, 0)],
    [p(3, 18), p(22, 22), p(23, 23), p(-6, 12), p(42, 2), p(0, 0)],
    [p(-1, -0), p(7, 13), p(-0, 31), p(-0, 7), p(1, -16), p(0, 0)],
    [
        p(70, 32),
        p(-31, 23),
        p(1, 21),
        p(-35, 12),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(11, 5), p(9, 10), p(15, 5), p(9, 16), p(14, 3)],
    [p(-3, 1), p(7, 18), p(-96, -33), p(6, 12), p(7, 17), p(4, 5)],
    [p(3, 1), p(14, 4), p(9, 10), p(11, 7), p(12, 15), p(22, -5)],
    [
        p(3, -5),
        p(10, -2),
        p(8, -8),
        p(4, 15),
        p(-58, -251),
        p(7, -11),
    ],
    [
        p(58, -4),
        p(37, 1),
        p(42, -4),
        p(20, -0),
        p(32, -17),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-11, -5),
    p(16, -8),
    p(17, -2),
    p(23, -13),
    p(6, 23),
    p(7, 19),
];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static {
    type Score: ScoreType;

    fn psqt(
        square: ChessSquare,
        piece: ChessPieceType,
        color: ChessColor,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn passed_pawn(square: ChessSquare) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn unsupported_pawn() -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn doubled_pawn() -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn passed_pawn_outside_square_rule() -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn bishop_pair() -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn rook_openness(openness: FileOpenness) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn king_openness(openness: FileOpenness) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn bishop_openness(
        openness: FileOpenness,
        len: usize,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn pawn_shield(config: usize) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn pawn_protection(piece: ChessPieceType) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn pawn_attack(piece: ChessPieceType) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn mobility(
        piece: ChessPieceType,
        mobility: usize,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn threats(
        attacking: ChessPieceType,
        targeted: ChessPieceType,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn defended(
        protecting: ChessPieceType,
        target: ChessPieceType,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn king_zone_attack(
        attacking: ChessPieceType,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;
}

/// Eval values tuned on a combination of the zurichess dataset and a dataset used by 4ku,
/// created by GCP using his engine Stoofvlees and filtered by cj5716 using Stockfish at depth 9,
/// using my own tuner `pliers`.
#[derive(Debug, Default, Copy, Clone)]
pub struct Lite {}

impl LiteValues for Lite {
    type Score = PhasedScore;

    fn psqt(square: ChessSquare, piece: ChessPieceType, color: ChessColor) -> Self::Score {
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

    fn passed_pawn_outside_square_rule() -> <Self::Score as ScoreType>::SingleFeatureScore {
        PASSED_PAWN_OUTSIDE_SQUARE_RULE
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

    fn pawn_protection(piece: ChessPieceType) -> Self::Score {
        PAWN_PROTECTION[piece as usize]
    }

    fn pawn_attack(piece: ChessPieceType) -> Self::Score {
        PAWN_ATTACKS[piece as usize]
    }

    fn mobility(piece: ChessPieceType, mobility: usize) -> Self::Score {
        MOBILITY[piece as usize - 1][mobility]
    }

    fn threats(attacking: ChessPieceType, targeted: ChessPieceType) -> Self::Score {
        THREATS[attacking as usize - 1][targeted as usize]
    }
    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> Self::Score {
        DEFENDED[protecting as usize - 1][target as usize]
    }

    fn king_zone_attack(
        attacking: ChessPieceType,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore {
        KING_ZONE_ATTACK[attacking as usize]
    }
}
