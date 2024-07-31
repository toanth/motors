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
        p( 133,  186),    p( 130,  185),    p( 120,  189),    p( 133,  169),    p( 119,  173),    p( 120,  177),    p(  83,  195),    p(  92,  192),
        p(  65,  123),    p(  63,  124),    p(  74,  120),    p(  82,  124),    p(  67,  125),    p( 118,  110),    p(  92,  132),    p(  89,  122),
        p(  51,  113),    p(  63,  109),    p(  61,  103),    p(  66,   97),    p(  82,   98),    p(  83,   94),    p(  77,  103),    p(  71,   96),
        p(  48,  100),    p(  55,  102),    p(  64,   95),    p(  73,   93),    p(  77,   92),    p(  77,   88),    p(  71,   92),    p(  59,   86),
        p(  43,   97),    p(  51,   94),    p(  56,   94),    p(  59,  100),    p(  67,   97),    p(  62,   93),    p(  70,   84),    p(  53,   85),
        p(  50,   98),    p(  51,   97),    p(  58,   98),    p(  58,  105),    p(  54,  108),    p(  73,   98),    p(  73,   84),    p(  55,   87),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 186,  268),    p( 210,  300),    p( 231,  314),    p( 266,  303),    p( 282,  308),    p( 206,  301),    p( 222,  301),    p( 213,  251),
        p( 277,  302),    p( 286,  310),    p( 296,  307),    p( 305,  311),    p( 294,  308),    p( 311,  297),    p( 276,  309),    p( 282,  295),
        p( 293,  300),    p( 304,  303),    p( 318,  311),    p( 327,  313),    p( 328,  310),    p( 346,  301),    p( 286,  305),    p( 287,  302),
        p( 306,  308),    p( 313,  305),    p( 321,  315),    p( 344,  318),    p( 330,  317),    p( 329,  317),    p( 320,  306),    p( 328,  303),
        p( 309,  309),    p( 301,  305),    p( 309,  317),    p( 316,  319),    p( 319,  321),    p( 317,  308),    p( 315,  304),    p( 316,  307),
        p( 283,  298),    p( 285,  299),    p( 286,  300),    p( 295,  314),    p( 302,  310),    p( 285,  293),    p( 299,  292),    p( 296,  301),
        p( 274,  307),    p( 290,  306),    p( 286,  300),    p( 291,  306),    p( 297,  301),    p( 289,  296),    p( 298,  297),    p( 294,  312),
        p( 269,  299),    p( 286,  302),    p( 275,  299),    p( 294,  303),    p( 298,  302),    p( 296,  291),    p( 293,  299),    p( 274,  298),
    ],
    // bishop
    [
        p( 280,  315),    p( 256,  314),    p( 249,  307),    p( 225,  315),    p( 222,  315),    p( 226,  306),    p( 282,  304),    p( 252,  308),
        p( 280,  303),    p( 285,  306),    p( 288,  306),    p( 281,  308),    p( 284,  303),    p( 293,  303),    p( 272,  308),    p( 275,  303),
        p( 297,  309),    p( 303,  305),    p( 296,  311),    p( 303,  303),    p( 306,  306),    p( 333,  309),    p( 319,  303),    p( 311,  311),
        p( 281,  310),    p( 298,  310),    p( 300,  306),    p( 317,  311),    p( 311,  307),    p( 307,  309),    p( 301,  308),    p( 283,  311),
        p( 292,  308),    p( 281,  311),    p( 300,  309),    p( 314,  308),    p( 313,  308),    p( 298,  307),    p( 291,  309),    p( 311,  300),
        p( 293,  307),    p( 304,  310),    p( 301,  310),    p( 304,  310),    p( 307,  311),    p( 304,  307),    p( 306,  302),    p( 310,  300),
        p( 309,  311),    p( 303,  300),    p( 311,  303),    p( 296,  310),    p( 302,  308),    p( 303,  306),    p( 313,  301),    p( 303,  298),
        p( 294,  304),    p( 314,  309),    p( 306,  306),    p( 290,  312),    p( 303,  309),    p( 296,  313),    p( 304,  297),    p( 301,  295),
    ],
    // rook
    [
        p( 458,  550),    p( 449,  560),    p( 446,  565),    p( 445,  563),    p( 457,  559),    p( 476,  553),    p( 485,  552),    p( 495,  545),
        p( 433,  556),    p( 430,  562),    p( 439,  562),    p( 454,  552),    p( 444,  554),    p( 464,  549),    p( 476,  546),    p( 490,  537),
        p( 438,  553),    p( 456,  548),    p( 454,  550),    p( 457,  545),    p( 485,  534),    p( 493,  531),    p( 516,  527),    p( 488,  530),
        p( 435,  552),    p( 442,  548),    p( 443,  551),    p( 449,  546),    p( 457,  538),    p( 466,  532),    p( 473,  534),    p( 470,  530),
        p( 431,  548),    p( 430,  547),    p( 431,  548),    p( 437,  545),    p( 444,  541),    p( 438,  540),    p( 458,  533),    p( 447,  531),
        p( 427,  544),    p( 427,  542),    p( 430,  541),    p( 432,  542),    p( 439,  536),    p( 448,  528),    p( 470,  515),    p( 452,  520),
        p( 430,  539),    p( 434,  539),    p( 440,  541),    p( 442,  538),    p( 450,  532),    p( 464,  521),    p( 473,  516),    p( 441,  525),
        p( 439,  543),    p( 436,  539),    p( 437,  544),    p( 442,  539),    p( 449,  533),    p( 455,  532),    p( 453,  530),    p( 447,  530),
    ],
    // queen
    [
        p( 874,  969),    p( 876,  983),    p( 891,  996),    p( 907,  992),    p( 906,  995),    p( 927,  983),    p( 976,  932),    p( 922,  963),
        p( 883,  961),    p( 859,  993),    p( 861, 1020),    p( 853, 1037),    p( 861, 1048),    p( 900, 1008),    p( 904,  989),    p( 946,  967),
        p( 891,  966),    p( 883,  986),    p( 883, 1008),    p( 881, 1017),    p( 904, 1018),    p( 944, 1002),    p( 951,  971),    p( 938,  977),
        p( 876,  980),    p( 882,  990),    p( 876, 1000),    p( 875, 1014),    p( 880, 1024),    p( 892, 1014),    p( 902, 1013),    p( 909,  990),
        p( 888,  970),    p( 874,  989),    p( 880,  993),    p( 880, 1010),    p( 881, 1008),    p( 884, 1007),    p( 898,  992),    p( 905,  984),
        p( 883,  955),    p( 889,  973),    p( 882,  989),    p( 879,  992),    p( 884,  999),    p( 891,  989),    p( 905,  971),    p( 905,  958),
        p( 886,  953),    p( 884,  963),    p( 890,  966),    p( 889,  979),    p( 890,  979),    p( 892,  962),    p( 902,  939),    p( 912,  912),
        p( 872,  951),    p( 883,  941),    p( 883,  956),    p( 891,  957),    p( 893,  950),    p( 882,  950),    p( 883,  939),    p( 886,  927),
    ],
    // king
    [
        p( 152, -102),    p(  59,  -49),    p(  82,  -41),    p(   6,   -9),    p(  29,  -21),    p(  12,  -11),    p(  65,  -21),    p( 224, -106),
        p( -21,   -3),    p( -66,   26),    p( -74,   36),    p(  -8,   25),    p( -41,   34),    p( -68,   46),    p( -34,   32),    p(  13,   -1),
        p( -42,    6),    p( -34,   22),    p( -78,   40),    p( -84,   48),    p( -51,   42),    p( -17,   35),    p( -54,   33),    p( -27,   10),
        p( -25,   -1),    p( -88,   22),    p(-103,   39),    p(-125,   48),    p(-123,   46),    p(-104,   38),    p(-109,   27),    p( -95,   15),
        p( -44,   -4),    p(-108,   17),    p(-117,   34),    p(-141,   47),    p(-146,   45),    p(-123,   31),    p(-136,   22),    p(-114,   12),
        p( -36,   -0),    p( -85,   12),    p(-114,   27),    p(-121,   36),    p(-116,   34),    p(-131,   27),    p(-102,   12),    p( -71,    9),
        p(  28,   -9),    p( -68,    7),    p( -79,   15),    p(-100,   25),    p(-105,   25),    p( -90,   16),    p( -60,    0),    p(   6,   -4),
        p(  44,  -41),    p(  42,  -47),    p(  36,  -35),    p( -24,  -14),    p(  28,  -32),    p( -21,  -18),    p(  35,  -42),    p(  61,  -50),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  33,   86),    p(  30,   85),    p(  20,   89),    p(  33,   69),    p(  19,   73),    p(  20,   77),    p( -17,   95),    p(  -8,   92),
        p(  41,  123),    p(  47,  122),    p(  37,   98),    p(  22,   67),    p(  37,   65),    p(  15,   94),    p(   1,  102),    p( -28,  123),
        p(  24,   72),    p(  17,   70),    p(  23,   53),    p(  16,   43),    p(  -1,   44),    p(   7,   57),    p( -11,   74),    p( -10,   77),
        p(   8,   45),    p(  -3,   43),    p( -15,   34),    p(  -9,   24),    p( -17,   29),    p( -11,   38),    p( -19,   54),    p( -11,   50),
        p(   2,   14),    p( -12,   22),    p( -15,   16),    p( -16,    8),    p( -14,   13),    p(  -8,   17),    p( -14,   36),    p(   9,   16),
        p(  -4,   15),    p(  -2,   20),    p(  -9,   16),    p(  -8,    5),    p(   6,    1),    p(   7,    7),    p(  12,   18),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-11, -10);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(16, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(0, 2);
const KING_OPEN_FILE: PhasedScore = p(-56, -1);
const KING_CLOSED_FILE: PhasedScore = p(16, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 7);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 5), p(-3, 7), p(-2, 9), p(3, 7), p(4, 10), p(5, 12), p(10, 11), p(21, 6), ],
    // Closed
    [p(0, 0), p(0, 0), p(14, -32), p(-15, 9), p(1, 13), p(3, 4), p(2, 10), p(-0, 6), ],
    // SemiOpen
    [p(0, 0), p(-16, 22), p(2, 20), p(1, 14), p(-1, 19), p(4, 14), p(2, 11), p(12, 11), ],
    // SemiClosed
    [p(0, 0), p(11, -13), p(8, 6), p(5, 1), p(9, 4), p(4, 4), p(8, 7), p(3, 4), ],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-7, 7),    /*0b0000*/
    p(-15, 12),  /*0b0001*/
    p(-2, 8),    /*0b0010*/
    p(-10, 15),  /*0b0011*/
    p(-5, 7),    /*0b0100*/
    p(-27, 4),   /*0b0101*/
    p(-14, 7),   /*0b0110*/
    p(-19, -16), /*0b0111*/
    p(7, 11),    /*0b1000*/
    p(-5, 11),   /*0b1001*/
    p(3, 9),     /*0b1010*/
    p(-3, 12),   /*0b1011*/
    p(-1, 7),    /*0b1100*/
    p(-25, 10),  /*0b1101*/
    p(-12, 5),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(1, 18),    /*0b10000*/
    p(4, 12),    /*0b10001*/
    p(21, 13),   /*0b10010*/
    p(-3, 10),   /*0b10011*/
    p(-5, 9),    /*0b10100*/
    p(13, 18),   /*0b10101*/
    p(-22, 3),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(12, 33),   /*0b11000*/
    p(31, 26),   /*0b11001*/
    p(42, 40),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(16, 14),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(15, 10),   /*0b100000*/
    p(4, 15),    /*0b100001*/
    p(26, 4),    /*0b100010*/
    p(6, 2),     /*0b100011*/
    p(-9, 4),    /*0b100100*/
    p(-22, -7),  /*0b100101*/
    p(-24, 17),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(20, 2),    /*0b101000*/
    p(-2, 18),   /*0b101001*/
    p(21, -3),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-6, 6),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(14, 21),   /*0b110000*/
    p(26, 17),   /*0b110001*/
    p(33, 12),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(8, 32),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(24, 16),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(6, -0),    /*0b111111*/
    p(-21, -10), /*0b00*/
    p(8, -25),   /*0b01*/
    p(36, -14),  /*0b10*/
    p(23, -49),  /*0b11*/
    p(45, -18),  /*0b100*/
    p(-5, -27),  /*0b101*/
    p(73, -48),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(55, -20),  /*0b1000*/
    p(20, -44),  /*0b1001*/
    p(78, -64),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(55, -26),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(13, -6),   /*0b1111*/
    p(14, -10),  /*0b00*/
    p(30, -20),  /*0b01*/
    p(24, -27),  /*0b10*/
    p(21, -52),  /*0b11*/
    p(31, -18),  /*0b100*/
    p(52, -29),  /*0b101*/
    p(21, -33),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(35, -12),  /*0b1000*/
    p(52, -26),  /*0b1001*/
    p(48, -52),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(38, -31),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(20, -54),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 7), p(2, 9), p(10, 14), p(9, 9), p(-5, 19), p(-46, 9)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(38, 9), p(42, 35), p(51, -9), p(37, -39), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-54, -63),
        p(-31, -25),
        p(-15, -3),
        p(-3, 8),
        p(8, 17),
        p(18, 26),
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
        p(30, 33),
        p(43, 25),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-66, 27),
        p(-61, 32),
        p(-58, 37),
        p(-58, 43),
        p(-53, 48),
        p(-50, 52),
        p(-46, 54),
        p(-42, 58),
        p(-38, 61),
        p(-33, 63),
        p(-30, 67),
        p(-20, 66),
        p(-8, 63),
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
        p(-35, -34),
        p(-35, 20),
        p(-38, 69),
        p(-33, 87),
        p(-30, 104),
        p(-25, 109),
        p(-21, 119),
        p(-17, 125),
        p(-13, 129),
        p(-9, 131),
        p(-7, 135),
        p(-3, 138),
        p(0, 139),
        p(1, 143),
        p(4, 144),
        p(8, 147),
        p(9, 153),
        p(11, 152),
        p(21, 149),
        p(35, 141),
        p(40, 140),
        p(83, 116),
        p(83, 118),
        p(106, 98),
        p(199, 62),
        p(250, 18),
        p(286, 2),
        p(339, -33),
    ],
    [
        p(-84, 50),
        p(-51, 22),
        p(-25, 11),
        p(1, 4),
        p(28, -2),
        p(47, -10),
        p(70, -9),
        p(90, -16),
        p(137, -41),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-6, -4),
        p(23, 16),
        p(51, -15),
        p(21, -44),
        p(0, 0),
    ],
    [p(-1, 13), p(19, 21), p(-2, 9), p(28, 2), p(27, 55), p(0, 0)],
    [
        p(3, 17),
        p(21, 21),
        p(23, 21),
        p(-6, 11),
        p(43, -5),
        p(0, 0),
    ],
    [p(-0, -2), p(7, 12), p(-0, 30), p(0, 6), p(2, -17), p(0, 0)],
    [p(79, 34), p(-3, 12), p(3, 19), p(-34, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(11, 5), p(9, 9), p(15, 5), p(9, 17), p(13, 2)],
    [
        p(-3, 1),
        p(8, 18),
        p(-102, -34),
        p(6, 12),
        p(7, 16),
        p(4, 5),
    ],
    [p(3, 2), p(14, 4), p(9, 10), p(11, 7), p(12, 15), p(22, -6)],
    [
        p(3, -5),
        p(10, -1),
        p(8, -8),
        p(4, 15),
        p(-55, -262),
        p(7, -11),
    ],
    [
        p(60, -8),
        p(38, -1),
        p(43, -6),
        p(21, -3),
        p(33, -18),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-13, -10),
    p(5, -6),
    p(17, -3),
    p(23, -13),
    p(6, 22),
    p(6, 19),
];
const KNIGHT_DISTANCE: [PhasedScore; 15] = [
    p(0, 0),
    p(36, -12),
    p(48, -0),
    p(0, 3),
    p(16, 7),
    p(-4, 6),
    p(-6, 10),
    p(-8, 7),
    p(-9, 5),
    p(-5, 7),
    p(-13, 9),
    p(-15, 8),
    p(-2, -2),
    p(-31, -4),
    p(-52, 38),
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
