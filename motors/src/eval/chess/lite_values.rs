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
        p( 137,  193),    p( 134,  192),    p( 124,  195),    p( 136,  176),    p( 123,  180),    p( 124,  183),    p(  87,  201),    p(  95,  199),
        p(  66,  124),    p(  62,  125),    p(  74,  120),    p(  82,  125),    p(  66,  126),    p( 117,  111),    p(  90,  133),    p(  88,  123),
        p(  52,  114),    p(  63,  110),    p(  60,  105),    p(  65,   98),    p(  81,   99),    p(  83,   95),    p(  76,  104),    p(  71,   97),
        p(  48,  101),    p(  54,  104),    p(  63,   97),    p(  72,   96),    p(  76,   94),    p(  77,   90),    p(  70,   94),    p(  59,   87),
        p(  43,   99),    p(  50,   96),    p(  54,   96),    p(  58,  102),    p(  66,   99),    p(  61,   95),    p(  68,   86),    p(  53,   86),
        p(  49,  100),    p(  50,   99),    p(  57,  100),    p(  56,  107),    p(  53,  111),    p(  71,  100),    p(  71,   87),    p(  54,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 183,  274),    p( 208,  305),    p( 242,  317),    p( 266,  308),    p( 299,  307),    p( 213,  302),    p( 230,  301),    p( 212,  255),
        p( 270,  316),    p( 281,  325),    p( 295,  321),    p( 309,  323),    p( 300,  320),    p( 323,  308),    p( 281,  320),    p( 285,  307),
        p( 287,  312),    p( 299,  314),    p( 315,  324),    p( 317,  330),    p( 333,  322),    p( 358,  308),    p( 311,  310),    p( 305,  308),
        p( 301,  317),    p( 308,  313),    p( 315,  326),    p( 341,  329),    p( 319,  328),    p( 333,  322),    p( 314,  314),    p( 331,  310),
        p( 299,  317),    p( 298,  311),    p( 305,  323),    p( 311,  327),    p( 318,  327),    p( 316,  312),    p( 327,  305),    p( 316,  311),
        p( 275,  302),    p( 276,  304),    p( 284,  304),    p( 290,  319),    p( 298,  314),    p( 283,  297),    p( 298,  294),    p( 293,  304),
        p( 271,  307),    p( 281,  310),    p( 277,  305),    p( 289,  309),    p( 292,  303),    p( 285,  299),    p( 295,  300),    p( 290,  314),
        p( 244,  302),    p( 282,  300),    p( 266,  302),    p( 285,  308),    p( 296,  304),    p( 292,  293),    p( 289,  299),    p( 268,  299),
    ],
    // bishop
    [
        p( 278,  318),    p( 257,  312),    p( 247,  309),    p( 224,  316),    p( 223,  314),    p( 226,  305),    p( 285,  300),    p( 252,  308),
        p( 276,  313),    p( 281,  317),    p( 284,  316),    p( 279,  318),    p( 281,  313),    p( 291,  311),    p( 269,  318),    p( 271,  313),
        p( 294,  318),    p( 300,  311),    p( 293,  319),    p( 299,  312),    p( 303,  314),    p( 331,  314),    p( 316,  309),    p( 309,  318),
        p( 279,  315),    p( 297,  313),    p( 299,  310),    p( 315,  318),    p( 310,  311),    p( 306,  311),    p( 301,  309),    p( 281,  314),
        p( 291,  310),    p( 280,  313),    p( 299,  310),    p( 314,  310),    p( 312,  308),    p( 299,  305),    p( 291,  308),    p( 311,  300),
        p( 293,  308),    p( 304,  308),    p( 301,  309),    p( 304,  309),    p( 307,  309),    p( 304,  305),    p( 306,  299),    p( 310,  300),
        p( 309,  309),    p( 303,  300),    p( 311,  301),    p( 296,  310),    p( 303,  307),    p( 303,  302),    p( 313,  299),    p( 303,  295),
        p( 294,  306),    p( 315,  307),    p( 306,  307),    p( 290,  311),    p( 303,  308),    p( 296,  312),    p( 304,  293),    p( 301,  295),
    ],
    // rook
    [
        p( 457,  549),    p( 450,  558),    p( 447,  564),    p( 444,  561),    p( 457,  557),    p( 478,  552),    p( 486,  551),    p( 494,  543),
        p( 427,  554),    p( 424,  559),    p( 433,  560),    p( 449,  550),    p( 439,  552),    p( 460,  547),    p( 471,  543),    p( 485,  534),
        p( 433,  550),    p( 453,  546),    p( 449,  547),    p( 452,  543),    p( 479,  532),    p( 491,  527),    p( 514,  524),    p( 485,  527),
        p( 434,  549),    p( 441,  545),    p( 442,  548),    p( 446,  543),    p( 457,  535),    p( 466,  529),    p( 473,  531),    p( 469,  526),
        p( 430,  545),    p( 430,  544),    p( 431,  545),    p( 437,  542),    p( 444,  538),    p( 438,  538),    p( 458,  530),    p( 447,  528),
        p( 427,  541),    p( 427,  540),    p( 430,  539),    p( 432,  540),    p( 440,  534),    p( 448,  527),    p( 471,  513),    p( 452,  518),
        p( 430,  536),    p( 434,  537),    p( 440,  539),    p( 443,  536),    p( 451,  530),    p( 465,  520),    p( 473,  515),    p( 442,  523),
        p( 439,  541),    p( 436,  538),    p( 437,  542),    p( 442,  538),    p( 449,  532),    p( 455,  531),    p( 453,  528),    p( 447,  529),
    ],
    // queen
    [
        p( 875,  968),    p( 877,  982),    p( 891,  995),    p( 908,  992),    p( 907,  995),    p( 927,  982),    p( 976,  931),    p( 922,  962),
        p( 882,  962),    p( 858,  993),    p( 860, 1020),    p( 852, 1038),    p( 860, 1049),    p( 899, 1009),    p( 902,  990),    p( 944,  968),
        p( 891,  966),    p( 883,  986),    p( 883, 1008),    p( 880, 1017),    p( 902, 1019),    p( 944, 1002),    p( 950,  972),    p( 938,  977),
        p( 877,  980),    p( 883,  990),    p( 876,  999),    p( 875, 1014),    p( 881, 1023),    p( 893, 1014),    p( 902, 1013),    p( 909,  989),
        p( 888,  969),    p( 875,  988),    p( 881,  992),    p( 881, 1009),    p( 882, 1007),    p( 885, 1006),    p( 899,  991),    p( 906,  983),
        p( 884,  954),    p( 890,  972),    p( 883,  988),    p( 880,  991),    p( 885,  998),    p( 892,  988),    p( 906,  969),    p( 905,  956),
        p( 887,  952),    p( 884,  962),    p( 891,  965),    p( 890,  978),    p( 891,  978),    p( 893,  961),    p( 903,  938),    p( 913,  910),
        p( 873,  950),    p( 884,  940),    p( 884,  954),    p( 892,  956),    p( 894,  949),    p( 883,  949),    p( 884,  937),    p( 887,  926),
    ],
    // king
    [
        p( 153, -104),    p(  58,  -50),    p(  83,  -42),    p(   5,   -9),    p(  27,  -22),    p(   9,  -12),    p(  65,  -22),    p( 219, -106),
        p( -20,   -5),    p( -67,   26),    p( -76,   36),    p( -10,   25),    p( -42,   34),    p( -69,   47),    p( -36,   33),    p(  13,   -2),
        p( -42,    4),    p( -35,   23),    p( -79,   41),    p( -84,   48),    p( -51,   43),    p( -19,   35),    p( -56,   33),    p( -28,   10),
        p( -26,   -2),    p( -90,   22),    p(-105,   40),    p(-127,   49),    p(-126,   47),    p(-107,   39),    p(-112,   28),    p( -97,   15),
        p( -47,   -5),    p(-113,   18),    p(-122,   35),    p(-145,   48),    p(-151,   46),    p(-129,   32),    p(-142,   23),    p(-118,   12),
        p( -39,   -1),    p( -90,   13),    p(-120,   28),    p(-127,   37),    p(-123,   35),    p(-137,   28),    p(-108,   13),    p( -75,   10),
        p(  26,  -10),    p( -72,    8),    p( -85,   16),    p(-105,   25),    p(-111,   26),    p( -96,   17),    p( -65,    1),    p(   3,   -4),
        p(  45,  -43),    p(  42,  -48),    p(  37,  -35),    p( -25,  -15),    p(  28,  -33),    p( -21,  -18),    p(  35,  -43),    p(  63,  -52),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  37,   93),    p(  34,   92),    p(  24,   95),    p(  36,   76),    p(  23,   80),    p(  24,   83),    p( -13,  101),    p(  -5,   99),
        p(  47,  135),    p(  55,  134),    p(  45,  111),    p(  29,   79),    p(  45,   78),    p(  24,  107),    p(  10,  114),    p( -19,  135),
        p(  30,   84),    p(  25,   82),    p(  31,   65),    p(  23,   55),    p(   7,   56),    p(  15,   70),    p(  -2,   86),    p(  -2,   89),
        p(  13,   58),    p(   3,   56),    p(  -8,   46),    p(  -1,   36),    p(  -9,   41),    p(  -3,   50),    p( -10,   66),    p(  -4,   62),
        p(   7,   26),    p(  -5,   34),    p(  -7,   29),    p(  -9,   21),    p(  -6,   25),    p(   1,   29),    p(  -3,   47),    p(  17,   28),
        p(  -0,   28),    p(   4,   32),    p(  -1,   29),    p(  -1,   18),    p(  13,   14),    p(  13,   20),    p(  22,   30),    p(  14,   25),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-11, -9);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);

const BISHOP_PAIR: PhasedScore = p(23, 57);
const ROOK_OPEN_FILE: PhasedScore = p(16, 6);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(1, 4);
const KING_OPEN_FILE: PhasedScore = p(-56, -1);
const KING_CLOSED_FILE: PhasedScore = p(15, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-10, 7);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 6), p(-3, 9), p(-3, 10), p(3, 9), p(3, 11), p(4, 12), p(10, 12), p(21, 6), ],
    // Closed
    [p(0, 0), p(0, 0), p(12, -26), p(-15, 11), p(-0, 14), p(3, 5), p(1, 13), p(-1, 8), ],
    // SemiOpen
    [p(0, 0), p(-17, 21), p(2, 20), p(0, 16), p(-2, 20), p(3, 16), p(1, 13), p(11, 13), ],
    // SemiClosed
    [p(0, 0), p(10, -10), p(8, 7), p(5, 3), p(8, 5), p(3, 5), p(7, 9), p(2, 5), ],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-7, 6),    /*0b0000*/
    p(-16, 12),  /*0b0001*/
    p(-3, 8),    /*0b0010*/
    p(-10, 15),  /*0b0011*/
    p(-5, 7),    /*0b0100*/
    p(-27, 4),   /*0b0101*/
    p(-15, 7),   /*0b0110*/
    p(-20, -15), /*0b0111*/
    p(6, 10),    /*0b1000*/
    p(-6, 11),   /*0b1001*/
    p(1, 9),     /*0b1010*/
    p(-4, 13),   /*0b1011*/
    p(-2, 7),    /*0b1100*/
    p(-25, 9),   /*0b1101*/
    p(-13, 6),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(1, 18),    /*0b10000*/
    p(4, 12),    /*0b10001*/
    p(21, 13),   /*0b10010*/
    p(-3, 10),   /*0b10011*/
    p(-6, 9),    /*0b10100*/
    p(13, 18),   /*0b10101*/
    p(-22, 3),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(12, 33),   /*0b11000*/
    p(30, 26),   /*0b11001*/
    p(41, 39),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(16, 13),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(14, 10),   /*0b100000*/
    p(4, 15),    /*0b100001*/
    p(25, 3),    /*0b100010*/
    p(6, 2),     /*0b100011*/
    p(-11, 4),   /*0b100100*/
    p(-24, -7),  /*0b100101*/
    p(-26, 18),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(19, 1),    /*0b101000*/
    p(-3, 16),   /*0b101001*/
    p(19, -3),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-7, 6),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(13, 21),   /*0b110000*/
    p(25, 17),   /*0b110001*/
    p(32, 11),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(7, 31),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(23, 16),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(5, -1),    /*0b111111*/
    p(-19, -10), /*0b00*/
    p(11, -25),  /*0b01*/
    p(38, -14),  /*0b10*/
    p(26, -50),  /*0b11*/
    p(49, -18),  /*0b100*/
    p(-2, -22),  /*0b101*/
    p(76, -49),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(58, -20),  /*0b1000*/
    p(21, -45),  /*0b1001*/
    p(80, -62),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(58, -27),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(16, -7),   /*0b1111*/
    p(16, -11),  /*0b00*/
    p(32, -21),  /*0b01*/
    p(26, -27),  /*0b10*/
    p(24, -53),  /*0b11*/
    p(32, -19),  /*0b100*/
    p(54, -30),  /*0b101*/
    p(23, -34),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(37, -13),  /*0b1000*/
    p(55, -27),  /*0b1001*/
    p(51, -51),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(41, -31),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(23, -55),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(14, 6), p(2, 9), p(10, 14), p(9, 10), p(-5, 19), p(-46, 9)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(37, 13),
    p(41, 40),
    p(50, -11),
    p(37, -39),
    p(0, 0),
];

pub const OUTPOSTS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(-8, -12),
    p(6, -14),
    p(5, -15),
    p(7, 2),
    p(2, -2),
    p(-6, 1),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-58, -56),
        p(-35, -17),
        p(-20, 5),
        p(-8, 16),
        p(3, 25),
        p(12, 34),
        p(23, 34),
        p(32, 34),
        p(40, 30),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(3, -2),
        p(9, 7),
        p(13, 16),
        p(16, 20),
        p(18, 24),
        p(18, 29),
        p(24, 30),
        p(27, 29),
        p(35, 31),
        p(28, 39),
        p(41, 32),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-58, 37),
        p(-59, 44),
        p(-53, 48),
        p(-50, 52),
        p(-47, 54),
        p(-43, 58),
        p(-39, 61),
        p(-34, 63),
        p(-31, 67),
        p(-22, 66),
        p(-9, 64),
        p(-9, 63),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-33, -37),
        p(-34, 21),
        p(-37, 70),
        p(-32, 87),
        p(-30, 104),
        p(-25, 109),
        p(-20, 119),
        p(-17, 126),
        p(-12, 130),
        p(-9, 132),
        p(-7, 136),
        p(-3, 139),
        p(0, 140),
        p(1, 145),
        p(4, 146),
        p(7, 148),
        p(8, 154),
        p(11, 154),
        p(20, 151),
        p(35, 142),
        p(39, 142),
        p(82, 118),
        p(81, 120),
        p(106, 99),
        p(198, 64),
        p(251, 19),
        p(292, 0),
        p(344, -34),
    ],
    [
        p(-87, 52),
        p(-54, 24),
        p(-27, 13),
        p(0, 6),
        p(28, -1),
        p(49, -9),
        p(72, -9),
        p(94, -17),
        p(141, -42),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-11, 10),
        p(-6, -4),
        p(23, 17),
        p(48, -14),
        p(21, -44),
        p(0, 0),
    ],
    [p(-1, 13), p(18, 21), p(-2, 9), p(28, 3), p(27, 56), p(0, 0)],
    [
        p(3, 17),
        p(22, 19),
        p(23, 20),
        p(-7, 10),
        p(42, -5),
        p(0, 0),
    ],
    [p(-0, -2), p(7, 12), p(-0, 30), p(-0, 6), p(2, -17), p(0, 0)],
    [p(79, 34), p(-30, 21), p(2, 18), p(-32, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 5), p(10, 5), p(9, 10), p(15, 5), p(9, 16), p(13, 3)],
    [p(-3, 1), p(7, 18), p(-98, -32), p(6, 13), p(7, 16), p(4, 6)],
    [p(2, 2), p(13, 4), p(9, 10), p(11, 7), p(12, 15), p(22, -6)],
    [
        p(3, -4),
        p(10, -2),
        p(8, -8),
        p(4, 15),
        p(-58, -260),
        p(7, -11),
    ],
    [
        p(61, -9),
        p(39, -1),
        p(44, -6),
        p(22, -3),
        p(35, -19),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-13, -10),
    p(17, -8),
    p(17, -3),
    p(23, -13),
    p(6, 22),
    p(8, 18),
];

#[allow(type_alias_bounds)]
pub type SingleFeatureScore<L: LiteValues> = <L::Score as ScoreType>::SingleFeatureScore;

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static {
    type Score: ScoreType;

    fn psqt(
        square: ChessSquare,
        piece: ChessPieceType,
        color: ChessColor,
    ) -> SingleFeatureScore<Self>;

    fn passed_pawn(square: ChessSquare) -> SingleFeatureScore<Self>;

    fn unsupported_pawn() -> SingleFeatureScore<Self>;

    fn doubled_pawn() -> SingleFeatureScore<Self>;

    fn bishop_pair() -> SingleFeatureScore<Self>;

    fn rook_openness(openness: FileOpenness) -> SingleFeatureScore<Self>;

    fn king_openness(openness: FileOpenness) -> SingleFeatureScore<Self>;

    fn bishop_openness(openness: FileOpenness, len: usize) -> SingleFeatureScore<Self>;

    fn outpost(piece: ChessPieceType) -> SingleFeatureScore<Self>;

    fn pawn_shield(config: usize) -> SingleFeatureScore<Self>;

    fn pawn_protection(piece: ChessPieceType) -> SingleFeatureScore<Self>;

    fn pawn_attack(piece: ChessPieceType) -> SingleFeatureScore<Self>;

    fn mobility(piece: ChessPieceType, mobility: usize) -> SingleFeatureScore<Self>;

    fn threats(attacking: ChessPieceType, targeted: ChessPieceType) -> SingleFeatureScore<Self>;

    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> SingleFeatureScore<Self>;

    fn king_zone_attack(attacking: ChessPieceType) -> SingleFeatureScore<Self>;
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

    fn unsupported_pawn() -> SingleFeatureScore<Self> {
        UNSUPPORTED_PAWN
    }

    fn doubled_pawn() -> SingleFeatureScore<Self> {
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

    fn bishop_openness(openness: FileOpenness, len: usize) -> SingleFeatureScore<Self> {
        BISHOP_OPENNESS[openness as usize][len - 1]
    }

    fn outpost(piece: ChessPieceType) -> SingleFeatureScore<Self> {
        OUTPOSTS[piece as usize]
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

    fn king_zone_attack(attacking: ChessPieceType) -> SingleFeatureScore<Self> {
        KING_ZONE_ATTACK[attacking as usize]
    }
}
