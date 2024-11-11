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
        p( 133,  186),    p( 130,  185),    p( 120,  188),    p( 133,  168),    p( 118,  173),    p( 119,  176),    p(  84,  194),    p(  89,  192),    
        p(  64,  123),    p(  62,  124),    p(  74,  119),    p(  81,  123),    p(  65,  122),    p( 114,  108),    p(  93,  130),    p(  85,  120),    
        p(  51,  113),    p(  63,  109),    p(  61,  103),    p(  66,   96),    p(  82,   97),    p(  83,   93),    p(  77,  103),    p(  71,   95),    
        p(  48,  100),    p(  55,  102),    p(  64,   95),    p(  73,   94),    p(  77,   92),    p(  77,   88),    p(  71,   92),    p(  59,   85),    
        p(  43,   97),    p(  51,   94),    p(  55,   94),    p(  59,   99),    p(  67,   97),    p(  62,   93),    p(  69,   84),    p(  54,   85),    
        p(  50,   98),    p(  51,   97),    p(  58,   98),    p(  58,  105),    p(  54,  108),    p(  72,   98),    p(  73,   84),    p(  55,   87),    
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    
    ],
    // knight
    [
        p( 186,  275),    p( 210,  308),    p( 245,  320),    p( 269,  310),    p( 301,  312),    p( 215,  307),    p( 231,  307),    p( 215,  258),    
        p( 277,  309),    p( 288,  318),    p( 301,  315),    p( 315,  317),    p( 306,  314),    p( 329,  302),    p( 288,  313),    p( 292,  300),    
        p( 293,  307),    p( 303,  311),    p( 321,  320),    p( 323,  324),    p( 339,  317),    p( 362,  308),    p( 315,  308),    p( 310,  305),    
        p( 305,  315),    p( 311,  313),    p( 319,  325),    p( 345,  327),    p( 322,  328),    p( 335,  325),    p( 316,  316),    p( 334,  309),    
        p( 302,  318),    p( 301,  312),    p( 307,  325),    p( 314,  329),    p( 320,  330),    p( 318,  317),    p( 329,  309),    p( 318,  314),    
        p( 277,  305),    p( 278,  308),    p( 286,  308),    p( 292,  322),    p( 299,  319),    p( 285,  303),    p( 300,  300),    p( 295,  308),    
        p( 273,  310),    p( 283,  314),    p( 279,  309),    p( 291,  314),    p( 294,  308),    p( 286,  305),    p( 296,  305),    p( 292,  319),    
        p( 246,  305),    p( 284,  304),    p( 268,  306),    p( 287,  312),    p( 297,  309),    p( 293,  298),    p( 291,  304),    p( 269,  304),    
    ],
    // bishop
    [
        p( 282,  317),    p( 263,  323),    p( 252,  318),    p( 234,  326),    p( 232,  326),    p( 229,  318),    p( 289,  313),    p( 254,  310),    
        p( 286,  312),    p( 289,  315),    p( 295,  320),    p( 290,  320),    p( 292,  316),    p( 301,  316),    p( 276,  318),    p( 281,  313),    
        p( 301,  320),    p( 310,  318),    p( 304,  321),    p( 309,  317),    p( 313,  320),    p( 341,  320),    p( 326,  316),    p( 315,  323),    
        p( 290,  322),    p( 306,  322),    p( 307,  320),    p( 324,  324),    p( 318,  320),    p( 313,  323),    p( 310,  320),    p( 292,  322),    
        p( 301,  319),    p( 289,  324),    p( 306,  324),    p( 321,  321),    p( 319,  321),    p( 305,  321),    p( 299,  322),    p( 321,  311),    
        p( 297,  319),    p( 311,  323),    p( 309,  321),    p( 311,  324),    p( 314,  325),    p( 312,  318),    p( 313,  315),    p( 314,  312),    
        p( 315,  320),    p( 306,  310),    p( 318,  316),    p( 305,  323),    p( 311,  321),    p( 310,  319),    p( 317,  310),    p( 309,  307),    
        p( 296,  307),    p( 321,  317),    p( 309,  318),    p( 299,  323),    p( 312,  320),    p( 299,  324),    p( 310,  306),    p( 303,  297),    
    ],
    // rook
    [
        p( 457,  553),    p( 449,  562),    p( 446,  568),    p( 444,  565),    p( 456,  561),    p( 476,  556),    p( 484,  555),    p( 494,  547),    
        p( 432,  559),    p( 429,  564),    p( 438,  565),    p( 454,  555),    p( 444,  557),    p( 464,  551),    p( 475,  548),    p( 490,  539),    
        p( 437,  555),    p( 455,  551),    p( 454,  552),    p( 457,  548),    p( 485,  537),    p( 493,  533),    p( 516,  529),    p( 488,  532),    
        p( 435,  555),    p( 442,  551),    p( 443,  554),    p( 448,  548),    p( 457,  540),    p( 466,  535),    p( 473,  537),    p( 469,  532),    
        p( 430,  551),    p( 430,  549),    p( 431,  550),    p( 437,  547),    p( 444,  543),    p( 438,  542),    p( 457,  535),    p( 447,  533),    
        p( 427,  546),    p( 426,  545),    p( 430,  544),    p( 432,  544),    p( 439,  538),    p( 447,  531),    p( 470,  518),    p( 452,  522),    
        p( 430,  541),    p( 434,  541),    p( 440,  543),    p( 442,  541),    p( 450,  534),    p( 464,  524),    p( 473,  519),    p( 441,  527),    
        p( 439,  545),    p( 435,  541),    p( 437,  546),    p( 441,  542),    p( 449,  536),    p( 455,  535),    p( 452,  532),    p( 447,  532),    
    ],
    // queen
    [
        p( 874,  973),    p( 876,  987),    p( 891, 1000),    p( 907,  997),    p( 906, 1000),    p( 926,  988),    p( 976,  936),    p( 922,  967),    
        p( 884,  965),    p( 859,  997),    p( 861, 1024),    p( 853, 1042),    p( 861, 1053),    p( 900, 1013),    p( 903,  994),    p( 946,  971),    
        p( 891,  970),    p( 883,  990),    p( 883, 1012),    p( 881, 1021),    p( 904, 1023),    p( 943, 1007),    p( 951,  976),    p( 938,  982),    
        p( 876,  985),    p( 883,  995),    p( 876, 1004),    p( 875, 1019),    p( 880, 1029),    p( 892, 1019),    p( 902, 1017),    p( 909,  994),    
        p( 888,  974),    p( 874,  994),    p( 880,  997),    p( 880, 1015),    p( 881, 1012),    p( 884, 1012),    p( 898,  996),    p( 905,  988),    
        p( 883,  959),    p( 889,  977),    p( 882,  994),    p( 879,  997),    p( 884, 1003),    p( 891,  993),    p( 905,  975),    p( 905,  962),    
        p( 886,  957),    p( 884,  967),    p( 890,  970),    p( 889,  983),    p( 890,  983),    p( 892,  966),    p( 902,  943),    p( 912,  916),    
        p( 873,  955),    p( 883,  945),    p( 883,  959),    p( 891,  961),    p( 894,  954),    p( 882,  954),    p( 883,  943),    p( 886,  931),    
    ],
    // king
    [
        p(  54,  -54),    p(  23,  -23),    p(  66,  -18),    p( -15,   15),    p(  -4,    4),    p( -13,   13),    p(  48,    2),    p(  58,  -58),    
        p(  -7,    7),    p(  13,    7),    p(   5,   17),    p(  72,    6),    p(  40,   15),    p(  12,   28),    p(  46,   14),    p(  25,   -0),    
        p( -16,   15),    p(  45,    3),    p(   1,   22),    p(  -2,   29),    p(  31,   24),    p(  62,   16),    p(  27,   14),    p( -12,   12),    
        p( -11,   11),    p(  -3,    2),    p( -21,   20),    p( -44,   30),    p( -45,   28),    p( -21,   19),    p( -34,    9),    p( -85,   18),    
        p( -25,    8),    p( -30,   -2),    p( -41,   15),    p( -64,   29),    p( -70,   27),    p( -47,   12),    p( -59,    4),    p(-103,   16),    
        p( -13,   12),    p(  -5,   -7),    p( -35,    8),    p( -44,   18),    p( -39,   16),    p( -52,    9),    p( -22,   -6),    p( -62,   14),    
        p(  48,    3),    p(  13,  -13),    p(   1,   -4),    p( -20,    6),    p( -26,    6),    p( -10,   -3),    p(  21,  -19),    p(  16,   -1),    
        p(  -1,   -0),    p(  25,  -23),    p(  20,  -10),    p( -41,   10),    p(  12,   -8),    p( -38,    7),    p(  18,  -18),    p(   8,  -19),    
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(16, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(0, 2);
const KING_OPEN_FILE: PhasedScore = p(-56, -1);
const KING_CLOSED_FILE: PhasedScore = p(16, -16);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 5);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 6), p(-5, 7), p(-3, 2), p(-0, 3), p(-2, 2), p(2, 6), p(6, 1), p(19, 2)], 
    // Closed
    [p(-5, 7), p(-3, 2), p(-0, 3), p(-2, 2), p(2, 6), p(6, 1), p(19, 2), p(0, 0)], 
    // SemiOpen
    [p(-3, 2), p(-0, 3), p(-2, 2), p(2, 6), p(6, 1), p(19, 2), p(0, 0), p(0, 0)], 
    // SemiClosed
    [p(-0, 3), p(-2, 2), p(2, 6), p(6, 1), p(19, 2), p(0, 0), p(0, 0), p(9, -10)], 
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-6, 1),    /*0b0000*/
    p(-14, 6),   /*0b0001*/
    p(-2, 2),    /*0b0010*/
    p(-9, 9),    /*0b0011*/
    p(-4, 1),    /*0b0100*/
    p(-25, 0),   /*0b0101*/
    p(-13, 1),   /*0b0110*/
    p(-17, -21), /*0b0111*/
    p(7, 5),     /*0b1000*/
    p(-4, 5),    /*0b1001*/
    p(3, 3),     /*0b1010*/
    p(-2, 3),    /*0b1011*/
    p(-0, 2),    /*0b1100*/
    p(-24, 5),   /*0b1101*/
    p(-11, -0),  /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(2, 13),    /*0b10000*/
    p(5, 7),     /*0b10001*/
    p(22, 8),    /*0b10010*/
    p(-1, 3),    /*0b10011*/
    p(-4, 3),    /*0b10100*/
    p(15, 13),   /*0b10101*/
    p(-20, -3),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(13, 28),   /*0b11000*/
    p(32, 21),   /*0b11001*/
    p(43, 34),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(17, 9),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(15, 5),    /*0b100000*/
    p(5, 10),    /*0b100001*/
    p(27, -2),   /*0b100010*/
    p(8, -4),    /*0b100011*/
    p(-9, -2),   /*0b100100*/
    p(-22, -12), /*0b100101*/
    p(-19, 16),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(20, -3),   /*0b101000*/
    p(-2, 13),   /*0b101001*/
    p(21, -8),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-5, 4),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(15, 17),   /*0b110000*/
    p(27, 12),   /*0b110001*/
    p(34, 7),    /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(9, 27),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(25, 11),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(7, -7),    /*0b111111*/
    p(-56, -2),  /*0b00*/
    p(-27, -17), /*0b01*/
    p(5, -6),    /*0b10*/
    p(-11, -42), /*0b11*/
    p(11, -11),  /*0b100*/
    p(-40, -21), /*0b101*/
    p(39, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(14, -12),  /*0b1000*/
    p(-16, -36), /*0b1001*/
    p(56, -57),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(19, -19),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-20, 20),  /*0b1111*/
    p(-12, 7),   /*0b00*/
    p(4, -3),    /*0b01*/
    p(-2, -9),   /*0b10*/
    p(-4, -36),  /*0b11*/
    p(4, -2),    /*0b100*/
    p(26, -13),  /*0b101*/
    p(-5, -16),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(9, 4),     /*0b1000*/
    p(26, -10),  /*0b1001*/
    p(23, -36),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(13, -13),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-6, -38),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    
        p(  33,   86),    p(  30,   85),    p(  20,   88),    p(  33,   68),    p(  18,   73),    p(  19,   76),    p( -16,   94),    p( -11,   92),    
        p(  42,  122),    p(  48,  122),    p(  38,   99),    p(  22,   67),    p(  36,   67),    p(  16,   95),    p(   1,  103),    p( -28,  124),    
        p(  24,   72),    p(  18,   70),    p(  23,   53),    p(  16,   43),    p(  -2,   45),    p(   7,   57),    p( -10,   74),    p( -10,   77),    
        p(   8,   45),    p(  -3,   43),    p( -15,   33),    p(  -9,   24),    p( -17,   29),    p( -11,   37),    p( -19,   53),    p( -11,   49),    
        p(   2,   14),    p( -12,   22),    p( -15,   16),    p( -15,    8),    p( -13,   13),    p(  -9,   16),    p( -14,   35),    p(   9,   16),    
        p(  -4,   15),    p(  -2,   20),    p(  -9,   16),    p(  -7,    6),    p(   5,   -0),    p(   7,    6),    p(  12,   18),    p(   7,   13),    
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    
];
const UNSUPPORTED_PAWN: PhasedScore = p(-11, -10);
const DOUBLED_PAWN: PhasedScore = p(-6, -21);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 7), p(2, 9), p(10, 14), p(9, 9), p(-4, 19), p(-46, 8)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(38, 9), p(42, 35), p(51, -9), p(36, -37), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-59, -62),
        p(-37, -24),
        p(-21, -3),
        p(-9, 9),
        p(2, 17),
        p(12, 25),
        p(23, 25),
        p(32, 24),
        p(40, 19),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-26, -47),
        p(-14, -29),
        p(-4, -14),
        p(3, -3),
        p(9, 7),
        p(13, 15),
        p(16, 19),
        p(18, 22),
        p(19, 27),
        p(25, 27),
        p(29, 25),
        p(37, 26),
        p(30, 34),
        p(43, 26),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-75, 11),
        p(-66, 25),
        p(-61, 30),
        p(-58, 35),
        p(-59, 41),
        p(-53, 45),
        p(-50, 49),
        p(-46, 52),
        p(-42, 55),
        p(-39, 59),
        p(-33, 61),
        p(-30, 65),
        p(-20, 64),
        p(-8, 61),
        p(-5, 62),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-35, -40),
        p(-35, 16),
        p(-39, 64),
        p(-33, 82),
        p(-31, 99),
        p(-26, 104),
        p(-21, 114),
        p(-18, 121),
        p(-13, 125),
        p(-10, 127),
        p(-7, 130),
        p(-3, 133),
        p(-1, 134),
        p(1, 139),
        p(3, 140),
        p(7, 143),
        p(8, 149),
        p(11, 148),
        p(20, 144),
        p(34, 136),
        p(39, 136),
        p(83, 111),
        p(82, 113),
        p(106, 93),
        p(197, 58),
        p(250, 13),
        p(288, -4),
        p(336, -37),
    ],
    [
        p(26, -10),
        p(25, -23),
        p(17, -18),
        p(11, -11),
        p(5, -2),
        p(-9, 6),
        p(-21, 22),
        p(-33, 29),
        p(-17, 16),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-6, -2),
        p(23, 17),
        p(49, -15),
        p(21, -44),
        p(0, 0),
    ],
    [p(-1, 13), p(19, 21), p(-2, 7), p(28, 2), p(27, 56), p(0, 0)],
    [
        p(3, 17),
        p(22, 20),
        p(23, 21),
        p(-6, 10),
        p(43, -5),
        p(0, 0),
    ],
    [p(-0, -1), p(7, 12), p(-0, 30), p(0, 5), p(2, -17), p(0, 0)],
    [p(76, 34), p(-30, 22), p(2, 19), p(-33, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 5), p(11, 4), p(9, 10), p(15, 5), p(9, 16), p(13, 3)],
    [p(-3, 0), p(8, 18), p(44, -44), p(6, 12), p(7, 16), p(4, 5)],
    [p(3, 2), p(14, 4), p(9, 10), p(11, 7), p(12, 15), p(22, -6)],
    [
        p(3, -5),
        p(10, -2),
        p(8, -8),
        p(4, 15),
        p(-57, -260),
        p(7, -11),
    ],
    [p(27, 7), p(5, 14), p(10, 9), p(-12, 12), p(1, -3), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(24, -19),
    p(17, -9),
    p(17, -3),
    p(23, -13),
    p(6, 22),
    p(10, 20),
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

    fn bishop_pair() -> SingleFeatureScore<Self::Score> {
        BISHOP_PAIR
    }

    fn rook_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score> {
        match openness {
            FileOpenness::Open => ROOK_OPEN_FILE,
            FileOpenness::Closed => ROOK_CLOSED_FILE,
            FileOpenness::SemiOpen => ROOK_SEMIOPEN_FILE,
            FileOpenness::SemiClosed => Self::Score::default(),
        }
    }

    fn king_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score> {
        match openness {
            FileOpenness::Open => KING_OPEN_FILE,
            FileOpenness::Closed => KING_CLOSED_FILE,
            FileOpenness::SemiOpen => KING_SEMIOPEN_FILE,
            FileOpenness::SemiClosed => Self::Score::default(),
        }
    }

    fn bishop_openness(openness: FileOpenness, len: usize) -> PhasedScore {
        BISHOP_OPENNESS[openness as usize][len - 1]
    }

    fn pawn_shield(&self, _color: ChessColor, config: usize) -> SingleFeatureScore<Self::Score> {
        PAWN_SHIELDS[config]
    }

    fn pawn_protection(piece: ChessPieceType) -> SingleFeatureScore<Self::Score> {
        PAWN_PROTECTION[piece as usize]
    }

    fn pawn_attack(piece: ChessPieceType) -> SingleFeatureScore<Self::Score> {
        PAWN_ATTACKS[piece as usize]
    }

    fn mobility(piece: ChessPieceType, mobility: usize) -> SingleFeatureScore<Self::Score> {
        MOBILITY[piece as usize - 1][mobility]
    }

    fn threats(
        attacking: ChessPieceType,
        targeted: ChessPieceType,
    ) -> SingleFeatureScore<Self::Score> {
        THREATS[attacking as usize - 1][targeted as usize]
    }
    fn defended(
        protecting: ChessPieceType,
        target: ChessPieceType,
    ) -> SingleFeatureScore<Self::Score> {
        DEFENDED[protecting as usize - 1][target as usize]
    }

    fn king_zone_attack(attacking: ChessPieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }
}
