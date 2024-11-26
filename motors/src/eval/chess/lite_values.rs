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
        p( 133,  186),    p( 130,  185),    p( 120,  188),    p( 133,  169),    p( 119,  173),    p( 119,  177),    p(  83,  194),    p(  91,  192),
        p(  65,  123),    p(  62,  124),    p(  74,  120),    p(  82,  123),    p(  67,  125),    p( 118,  110),    p(  91,  132),    p(  88,  122),
        p(  51,  113),    p(  63,  109),    p(  61,  103),    p(  66,   96),    p(  82,   97),    p(  83,   94),    p(  77,  103),    p(  71,   96),
        p(  48,  100),    p(  54,  102),    p(  64,   94),    p(  73,   93),    p(  76,   92),    p(  77,   88),    p(  69,   92),    p(  59,   86),
        p(  43,   97),    p(  50,   94),    p(  55,   94),    p(  59,   99),    p(  67,   96),    p(  61,   93),    p(  68,   84),    p(  54,   85),
        p(  49,   98),    p(  51,   97),    p(  58,   98),    p(  56,  105),    p(  55,  108),    p(  72,   98),    p(  72,   84),    p(  54,   87),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 183,  270),    p( 208,  303),    p( 240,  315),    p( 264,  306),    p( 297,  307),    p( 208,  303),    p( 228,  302),    p( 212,  253),
        p( 274,  304),    p( 286,  312),    p( 297,  310),    p( 310,  313),    p( 301,  310),    p( 322,  299),    p( 283,  309),    p( 288,  296),
        p( 289,  302),    p( 293,  308),    p( 312,  316),    p( 310,  320),    p( 328,  313),    p( 351,  303),    p( 308,  303),    p( 305,  300),
        p( 300,  310),    p( 305,  307),    p( 311,  319),    p( 337,  321),    p( 317,  323),    p( 331,  319),    p( 311,  311),    p( 330,  303),
        p( 298,  312),    p( 298,  308),    p( 305,  320),    p( 310,  323),    p( 318,  325),    p( 318,  313),    p( 328,  305),    p( 316,  309),
        p( 275,  301),    p( 279,  306),    p( 286,  306),    p( 291,  318),    p( 300,  316),    p( 285,  303),    p( 301,  300),    p( 295,  305),
        p( 271,  305),    p( 282,  309),    p( 278,  304),    p( 290,  308),    p( 293,  303),    p( 286,  300),    p( 296,  300),    p( 291,  314),
        p( 245,  300),    p( 283,  299),    p( 267,  301),    p( 286,  306),    p( 297,  303),    p( 293,  293),    p( 290,  299),    p( 269,  299),
    ],
    // bishop
    [
        p( 279,  314),    p( 254,  314),    p( 248,  306),    p( 223,  315),    p( 223,  314),    p( 224,  307),    p( 281,  304),    p( 252,  308),
        p( 279,  303),    p( 282,  306),    p( 285,  306),    p( 280,  308),    p( 282,  303),    p( 292,  303),    p( 269,  308),    p( 273,  303),
        p( 296,  309),    p( 295,  306),    p( 289,  312),    p( 294,  304),    p( 299,  307),    p( 326,  310),    p( 315,  304),    p( 311,  311),
        p( 279,  311),    p( 295,  310),    p( 295,  306),    p( 309,  313),    p( 308,  307),    p( 304,  309),    p( 302,  308),    p( 281,  311),
        p( 290,  308),    p( 280,  311),    p( 299,  309),    p( 313,  308),    p( 312,  308),    p( 299,  306),    p( 292,  309),    p( 310,  300),
        p( 293,  307),    p( 306,  309),    p( 302,  309),    p( 305,  309),    p( 309,  310),    p( 307,  306),    p( 309,  301),    p( 310,  300),
        p( 308,  311),    p( 303,  300),    p( 310,  303),    p( 296,  310),    p( 302,  308),    p( 303,  305),    p( 313,  300),    p( 302,  298),
        p( 295,  304),    p( 315,  308),    p( 306,  307),    p( 290,  311),    p( 304,  309),    p( 296,  313),    p( 305,  297),    p( 301,  295),
    ],
    // rook
    [
        p( 459,  550),    p( 449,  559),    p( 447,  565),    p( 444,  562),    p( 456,  558),    p( 477,  553),    p( 485,  552),    p( 496,  544),
        p( 433,  556),    p( 430,  562),    p( 438,  562),    p( 455,  552),    p( 444,  554),    p( 464,  549),    p( 475,  546),    p( 490,  537),
        p( 436,  553),    p( 451,  549),    p( 451,  550),    p( 454,  545),    p( 482,  534),    p( 490,  530),    p( 512,  527),    p( 486,  529),
        p( 433,  552),    p( 441,  548),    p( 441,  551),    p( 445,  546),    p( 457,  537),    p( 466,  532),    p( 475,  534),    p( 469,  529),
        p( 430,  548),    p( 432,  547),    p( 433,  548),    p( 437,  545),    p( 445,  541),    p( 441,  541),    p( 462,  533),    p( 448,  531),
        p( 429,  544),    p( 431,  543),    p( 434,  541),    p( 434,  542),    p( 444,  537),    p( 453,  530),    p( 476,  517),    p( 455,  520),
        p( 431,  538),    p( 435,  538),    p( 441,  540),    p( 444,  538),    p( 451,  531),    p( 466,  521),    p( 475,  516),    p( 443,  525),
        p( 440,  542),    p( 436,  538),    p( 438,  543),    p( 442,  539),    p( 450,  533),    p( 456,  532),    p( 454,  529),    p( 448,  529),
    ],
    // queen
    [
        p( 875,  968),    p( 876,  982),    p( 892,  995),    p( 908,  992),    p( 907,  995),    p( 928,  982),    p( 978,  931),    p( 923,  962),
        p( 885,  960),    p( 859,  993),    p( 862, 1019),    p( 854, 1037),    p( 861, 1048),    p( 901, 1008),    p( 905,  989),    p( 947,  966),
        p( 892,  965),    p( 883,  986),    p( 883, 1008),    p( 880, 1017),    p( 903, 1018),    p( 942, 1002),    p( 950,  971),    p( 938,  977),
        p( 877,  980),    p( 883,  990),    p( 875, 1000),    p( 874, 1014),    p( 880, 1025),    p( 892, 1015),    p( 903, 1013),    p( 910,  989),
        p( 888,  969),    p( 875,  989),    p( 880,  993),    p( 880, 1010),    p( 882, 1008),    p( 885, 1008),    p( 899,  993),    p( 906,  984),
        p( 883,  955),    p( 891,  974),    p( 883,  989),    p( 880,  992),    p( 886, 1000),    p( 893,  990),    p( 908,  973),    p( 906,  957),
        p( 887,  953),    p( 884,  962),    p( 891,  966),    p( 890,  979),    p( 891,  979),    p( 893,  963),    p( 903,  940),    p( 913,  912),
        p( 873,  951),    p( 884,  940),    p( 884,  955),    p( 892,  957),    p( 894,  949),    p( 883,  949),    p( 882,  940),    p( 887,  926),
    ],
    // king
    [
        p( 151, -103),    p(  55,  -49),    p(  80,  -41),    p(   5,   -9),    p(  25,  -21),    p(  10,  -11),    p(  64,  -21),    p( 219, -105),
        p( -23,   -3),    p( -69,   26),    p( -77,   36),    p( -11,   25),    p( -45,   34),    p( -71,   47),    p( -38,   32),    p(   9,   -1),
        p( -44,    5),    p( -37,   22),    p( -81,   40),    p( -87,   48),    p( -54,   42),    p( -22,   35),    p( -60,   33),    p( -32,   10),
        p( -28,   -1),    p( -92,   22),    p(-107,   39),    p(-129,   48),    p(-127,   46),    p(-110,   38),    p(-114,   27),    p(-101,   15),
        p( -47,   -4),    p(-113,   17),    p(-122,   34),    p(-145,   47),    p(-151,   46),    p(-128,   32),    p(-140,   23),    p(-120,   12),
        p( -37,   -1),    p( -88,   12),    p(-117,   27),    p(-125,   36),    p(-120,   35),    p(-134,   28),    p(-105,   14),    p( -75,    9),
        p(  28,  -10),    p( -69,    7),    p( -82,   15),    p(-103,   25),    p(-108,   25),    p( -93,   16),    p( -62,    0),    p(   4,   -4),
        p(  45,  -42),    p(  43,  -47),    p(  38,  -35),    p( -24,  -14),    p(  29,  -33),    p( -20,  -18),    p(  35,  -43),    p(  62,  -52),
    ],
];

const BISHOP_PAIR: PhasedScore = p(24, 58);
const ROOK_OPEN_FILE: PhasedScore = p(16, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(-0, 2);
const KING_OPEN_FILE: PhasedScore = p(-56, -2);
const KING_CLOSED_FILE: PhasedScore = p(16, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 6);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 5), p(-2, 6), p(-2, 9), p(4, 7), p(4, 9), p(5, 12), p(10, 11), p(22, 6)],
    // Closed
    [p(0, 0), p(0, 0), p(14, -34), p(-16, 9), p(-0, 13), p(3, 4), p(2, 10), p(-1, 6)],
    // SemiOpen
    [p(0, 0), p(-16, 22), p(1, 20), p(2, 14), p(-1, 19), p(4, 14), p(1, 11), p(12, 11)],
    // SemiClosed
    [p(0, 0), p(9, -13), p(7, 6), p(5, 1), p(7, 4), p(3, 4), p(7, 7), p(2, 4)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-8, 7),    /*0b0000*/
    p(-16, 12),  /*0b0001*/
    p(-4, 8),    /*0b0010*/
    p(-10, 14),  /*0b0011*/
    p(-6, 7),    /*0b0100*/
    p(-27, 4),   /*0b0101*/
    p(-14, 7),   /*0b0110*/
    p(-19, -16), /*0b0111*/
    p(5, 11),    /*0b1000*/
    p(-5, 11),   /*0b1001*/
    p(0, 9),     /*0b1010*/
    p(-3, 11),   /*0b1011*/
    p(-2, 7),    /*0b1100*/
    p(-23, 10),  /*0b1101*/
    p(-13, 5),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(1, 18),    /*0b10000*/
    p(4, 12),    /*0b10001*/
    p(21, 13),   /*0b10010*/
    p(-2, 9),    /*0b10011*/
    p(-6, 8),    /*0b10100*/
    p(12, 18),   /*0b10101*/
    p(-21, 3),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(11, 33),   /*0b11000*/
    p(31, 26),   /*0b11001*/
    p(40, 39),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(15, 14),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(14, 10),   /*0b100000*/
    p(3, 15),    /*0b100001*/
    p(24, 4),    /*0b100010*/
    p(7, 2),     /*0b100011*/
    p(-9, 3),    /*0b100100*/
    p(-23, -7),  /*0b100101*/
    p(-25, 17),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(19, 2),    /*0b101000*/
    p(-2, 17),   /*0b101001*/
    p(18, -2),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-6, 6),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(13, 21),   /*0b110000*/
    p(25, 17),   /*0b110001*/
    p(32, 12),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(6, 32),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(22, 16),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(6, -0),    /*0b111111*/
    p(-23, -9),  /*0b00*/
    p(8, -26),   /*0b01*/
    p(35, -14),  /*0b10*/
    p(25, -50),  /*0b11*/
    p(46, -18),  /*0b100*/
    p(-3, -28),  /*0b101*/
    p(74, -48),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(56, -20),  /*0b1000*/
    p(20, -44),  /*0b1001*/
    p(81, -65),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(56, -26),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(19, -5),   /*0b1111*/
    p(16, -10),  /*0b00*/
    p(32, -20),  /*0b01*/
    p(26, -27),  /*0b10*/
    p(24, -53),  /*0b11*/
    p(32, -18),  /*0b100*/
    p(53, -29),  /*0b101*/
    p(23, -34),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(37, -12),  /*0b1000*/
    p(54, -27),  /*0b1001*/
    p(51, -53),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(41, -31),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(24, -54),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  33,   86),    p(  30,   85),    p(  20,   88),    p(  33,   69),    p(  19,   73),    p(  19,   77),    p( -17,   94),    p(  -9,   92),
        p(  40,  122),    p(  47,  122),    p(  36,   98),    p(  20,   67),    p(  36,   65),    p(  14,   94),    p(  -0,  102),    p( -28,  123),
        p(  23,   72),    p(  17,   70),    p(  23,   53),    p(  15,   43),    p(  -1,   44),    p(   6,   57),    p( -11,   74),    p( -11,   77),
        p(   5,   46),    p(  -3,   43),    p( -16,   34),    p(  -8,   24),    p( -17,   29),    p( -11,   38),    p( -18,   53),    p( -12,   50),
        p(   1,   14),    p( -12,   22),    p( -15,   16),    p( -16,    8),    p( -14,   13),    p(  -9,   17),    p( -14,   36),    p(   9,   16),
        p(  -5,   15),    p(  -4,   20),    p( -12,   17),    p(  -9,    4),    p(   3,    1),    p(   5,    7),    p(  11,   18),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-10, -10);
const DOUBLED_PAWN: PhasedScore = p(-5, -21);
const OUTPOST: [PhasedScore; NUM_CHESS_PIECES - 1] =
    [p(22, 16), p(29, -6), p(21, 16), p(16, 3), p(10, 10)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = [
    p(13, 7),
    p(-0, -2),
    p(5, 13),
    p(4, -4),
    p(-6, 12),
    p(-44, 2),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(35, 5),
    p(40, 35),
    p(49, -10),
    p(36, -39),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-58, -57),
        p(-35, -19),
        p(-19, 2),
        p(-7, 13),
        p(3, 22),
        p(13, 30),
        p(24, 29),
        p(33, 28),
        p(41, 23),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-4, -15),
        p(3, -3),
        p(9, 6),
        p(13, 14),
        p(16, 18),
        p(18, 22),
        p(19, 26),
        p(26, 27),
        p(30, 25),
        p(41, 25),
        p(34, 33),
        p(48, 25),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-74, 14),
        p(-65, 28),
        p(-61, 33),
        p(-57, 37),
        p(-57, 44),
        p(-52, 48),
        p(-49, 52),
        p(-45, 55),
        p(-41, 58),
        p(-37, 62),
        p(-31, 64),
        p(-28, 67),
        p(-19, 67),
        p(-6, 64),
        p(-3, 64),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-34, -32),
        p(-35, 22),
        p(-38, 71),
        p(-33, 88),
        p(-30, 106),
        p(-25, 111),
        p(-21, 120),
        p(-17, 126),
        p(-12, 130),
        p(-9, 132),
        p(-6, 135),
        p(-2, 138),
        p(1, 139),
        p(2, 144),
        p(5, 145),
        p(9, 147),
        p(10, 153),
        p(13, 152),
        p(22, 149),
        p(37, 141),
        p(42, 141),
        p(85, 116),
        p(85, 117),
        p(110, 97),
        p(202, 62),
        p(254, 17),
        p(291, 1),
        p(343, -34),
    ],
    [
        p(-85, 50),
        p(-52, 22),
        p(-26, 11),
        p(0, 4),
        p(28, -2),
        p(47, -10),
        p(71, -10),
        p(92, -17),
        p(138, -42),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-10, 13),
        p(-8, -4),
        p(23, 17),
        p(51, -15),
        p(23, -44),
        p(0, 0),
    ],
    [p(-1, 13), p(19, 21), p(-3, 8), p(30, 2), p(27, 56), p(0, 0)],
    [p(3, 16), p(21, 20), p(23, 21), p(-8, 9), p(42, -4), p(0, 0)],
    [
        p(-0, -2),
        p(6, 11),
        p(-1, 29),
        p(-1, 6),
        p(-0, -18),
        p(0, 0),
    ],
    [
        p(79, 33),
        p(-33, 21),
        p(-0, 20),
        p(-34, 9),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(10, 4), p(9, 10), p(15, 5), p(9, 16), p(14, 3)],
    [p(-2, 1), p(7, 18), p(-90, -35), p(6, 12), p(7, 17), p(4, 5)],
    [p(2, 2), p(14, 4), p(9, 10), p(12, 7), p(12, 15), p(22, -6)],
    [
        p(3, -5),
        p(10, -2),
        p(8, -8),
        p(4, 15),
        p(-56, -262),
        p(7, -11),
    ],
    [
        p(60, -8),
        p(38, -1),
        p(44, -6),
        p(21, -3),
        p(34, -19),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-13, -10),
    p(18, -10),
    p(16, -3),
    p(23, -13),
    p(6, 22),
    p(7, 19),
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

    fn king_zone_attack(attacking: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn outpost(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;
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

    fn king_zone_attack(attacking: ChessPieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }

    fn outpost(piece: ChessPieceType) -> PhasedScore {
        OUTPOST[piece as usize - 1]
    }
}
