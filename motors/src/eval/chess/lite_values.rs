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

use crate::eval::chess::{FileOpenness, NUM_PAWN_CENTER_CONFIGURATIONS, NUM_PAWN_SHIELD_CONFIGURATIONS};
use crate::eval::{ScoreType, SingleFeatureScore};
use gears::games::DimT;
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
        p( 137,  190),    p( 132,  188),    p( 125,  192),    p( 136,  175),    p( 122,  179),    p( 121,  183),    p(  81,  200),    p(  84,  199),
        p(  70,  121),    p(  71,  122),    p(  79,  115),    p(  88,  116),    p(  77,  115),    p( 125,  106),    p( 101,  127),    p(  96,  119),
        p(  53,  110),    p(  64,  104),    p(  63,   99),    p(  84,   99),    p(  93,   98),    p(  85,   89),    p(  77,   99),    p(  72,   94),
        p(  49,   97),    p(  54,  100),    p(  78,   93),    p(  93,   95),    p(  91,   97),    p(  87,   94),    p(  70,   89),    p(  60,   84),
        p(  42,   95),    p(  50,   91),    p(  72,   95),    p(  82,   97),    p(  83,   95),    p(  77,   94),    p(  69,   81),    p(  52,   84),
        p(  53,   99),    p(  58,   97),    p(  62,   97),    p(  59,  104),    p(  61,  106),    p(  76,   97),    p(  82,   86),    p(  59,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 174,  280),    p( 196,  313),    p( 212,  325),    p( 251,  314),    p( 281,  315),    p( 199,  310),    p( 215,  310),    p( 201,  264),
        p( 266,  313),    p( 284,  318),    p( 299,  310),    p( 304,  315),    p( 303,  310),    p( 315,  299),    p( 275,  316),    p( 271,  306),
        p( 286,  309),    p( 307,  304),    p( 309,  311),    p( 323,  315),    p( 339,  309),    p( 353,  298),    p( 294,  304),    p( 286,  308),
        p( 302,  315),    p( 310,  309),    p( 326,  313),    p( 327,  321),    p( 326,  318),    p( 322,  317),    p( 311,  313),    p( 320,  311),
        p( 299,  317),    p( 305,  306),    p( 314,  313),    p( 321,  315),    p( 320,  318),    p( 325,  303),    p( 323,  304),    p( 312,  313),
        p( 276,  303),    p( 283,  302),    p( 297,  297),    p( 301,  310),    p( 306,  308),    p( 296,  292),    p( 302,  294),    p( 293,  307),
        p( 270,  310),    p( 281,  314),    p( 286,  304),    p( 295,  309),    p( 299,  304),    p( 290,  302),    p( 295,  307),    p( 289,  321),
        p( 241,  308),    p( 282,  305),    p( 267,  307),    p( 287,  312),    p( 296,  309),    p( 292,  299),    p( 288,  308),    p( 264,  309),
    ],
    // bishop
    [
        p( 276,  311),    p( 251,  316),    p( 238,  309),    p( 221,  319),    p( 215,  317),    p( 224,  310),    p( 271,  305),    p( 249,  311),
        p( 282,  303),    p( 279,  305),    p( 290,  308),    p( 276,  306),    p( 288,  303),    p( 291,  302),    p( 267,  309),    p( 270,  304),
        p( 296,  309),    p( 307,  305),    p( 291,  305),    p( 306,  300),    p( 305,  302),    p( 336,  305),    p( 316,  303),    p( 318,  313),
        p( 287,  312),    p( 292,  308),    p( 304,  303),    p( 306,  307),    p( 306,  304),    p( 299,  305),    p( 298,  309),    p( 280,  311),
        p( 290,  306),    p( 284,  309),    p( 295,  304),    p( 308,  304),    p( 302,  300),    p( 299,  303),    p( 285,  306),    p( 309,  300),
        p( 296,  308),    p( 299,  304),    p( 299,  306),    p( 299,  304),    p( 305,  306),    p( 297,  300),    p( 305,  296),    p( 307,  298),
        p( 309,  306),    p( 304,  300),    p( 308,  301),    p( 299,  309),    p( 301,  307),    p( 302,  305),    p( 311,  296),    p( 308,  295),
        p( 298,  304),    p( 309,  306),    p( 307,  307),    p( 290,  311),    p( 306,  309),    p( 294,  310),    p( 302,  299),    p( 302,  292),
    ],
    // rook
    [
        p( 461,  547),    p( 449,  557),    p( 442,  564),    p( 440,  561),    p( 451,  557),    p( 472,  552),    p( 482,  551),    p( 492,  544),
        p( 443,  553),    p( 442,  559),    p( 451,  560),    p( 465,  551),    p( 451,  553),    p( 468,  548),    p( 477,  545),    p( 491,  535),
        p( 444,  549),    p( 465,  544),    p( 458,  545),    p( 458,  541),    p( 484,  530),    p( 494,  527),    p( 512,  526),    p( 485,  529),
        p( 441,  549),    p( 448,  544),    p( 447,  547),    p( 453,  541),    p( 457,  533),    p( 468,  529),    p( 469,  533),    p( 467,  529),
        p( 435,  546),    p( 434,  544),    p( 435,  544),    p( 440,  540),    p( 448,  536),    p( 442,  536),    p( 455,  530),    p( 449,  529),
        p( 430,  543),    p( 430,  540),    p( 432,  539),    p( 435,  538),    p( 440,  533),    p( 451,  525),    p( 467,  515),    p( 454,  518),
        p( 433,  539),    p( 437,  538),    p( 443,  538),    p( 445,  535),    p( 452,  529),    p( 465,  520),    p( 473,  515),    p( 443,  525),
        p( 442,  544),    p( 439,  539),    p( 440,  542),    p( 445,  536),    p( 450,  529),    p( 456,  530),    p( 453,  530),    p( 448,  533),
    ],
    // queen
    [
        p( 878,  964),    p( 879,  978),    p( 895,  990),    p( 915,  985),    p( 912,  990),    p( 933,  978),    p( 979,  930),    p( 924,  961),
        p( 889,  951),    p( 863,  982),    p( 864, 1009),    p( 858, 1026),    p( 864, 1038),    p( 904,  999),    p( 905,  985),    p( 947,  963),
        p( 894,  956),    p( 887,  972),    p( 885,  994),    p( 884, 1005),    p( 907, 1007),    p( 946,  991),    p( 953,  963),    p( 941,  970),
        p( 881,  969),    p( 885,  977),    p( 878,  987),    p( 879,  999),    p( 881, 1013),    p( 895, 1003),    p( 905, 1005),    p( 912,  981),
        p( 890,  960),    p( 877,  980),    p( 883,  979),    p( 883,  995),    p( 887,  993),    p( 888,  994),    p( 901,  984),    p( 908,  977),
        p( 886,  950),    p( 892,  965),    p( 886,  979),    p( 883,  981),    p( 888,  990),    p( 895,  979),    p( 909,  962),    p( 907,  952),
        p( 886,  953),    p( 886,  960),    p( 893,  962),    p( 892,  976),    p( 894,  974),    p( 895,  958),    p( 906,  938),    p( 914,  913),
        p( 872,  955),    p( 884,  942),    p( 885,  954),    p( 893,  956),    p( 896,  945),    p( 883,  949),    p( 884,  943),    p( 889,  927),
    ],
    // king
    [
        p( 154,  -68),    p(  65,  -22),    p(  82,  -12),    p(  14,   15),    p(  43,    4),    p(  24,   16),    p(  84,    5),    p( 219,  -72),
        p( -33,   17),    p( -61,   30),    p( -62,   36),    p(   3,   26),    p( -31,   36),    p( -51,   49),    p( -27,   38),    p(  11,   17),
        p( -51,   22),    p( -40,   21),    p( -78,   33),    p( -87,   43),    p( -51,   39),    p( -19,   31),    p( -58,   34),    p( -30,   21),
        p( -29,    7),    p( -95,   15),    p(-110,   28),    p(-132,   37),    p(-130,   36),    p(-108,   30),    p(-120,   21),    p(-101,   21),
        p( -41,   -3),    p(-104,    4),    p(-119,   19),    p(-145,   32),    p(-143,   30),    p(-118,   18),    p(-132,   11),    p(-115,   13),
        p( -31,    0),    p( -79,   -3),    p(-108,   10),    p(-116,   19),    p(-114,   20),    p(-123,   13),    p( -99,    2),    p( -70,   10),
        p(  27,   -8),    p( -67,   -6),    p( -78,    0),    p( -97,   10),    p(-103,   12),    p( -90,    4),    p( -63,  -10),    p(   2,   -1),
        p(  49,  -19),    p(  41,  -31),    p(  39,  -19),    p( -21,   -2),    p(  29,  -17),    p( -18,   -3),    p(  33,  -24),    p(  59,  -30),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 52);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(9, 20), p(10, 18), p(10, 7), p(6, -0), p(2, -8), p(-1, -17), p(-7, -26), p(-14, -39), p(-25, -49)];
const ROOK_OPEN_FILE: PhasedScore = p(14, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(4, 0);
const KING_OPEN_FILE: PhasedScore = p(-50, 0);
const KING_CLOSED_FILE: PhasedScore = p(12, -9);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 7);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-5, 4), p(1, 6), p(-1, 5), p(2, 3), p(2, 5), p(4, 7), p(7, 5), p(19, 2)],
    // Closed
    [p(0, 0), p(0, 0), p(21, -30), p(-14, 9), p(-1, 11), p(-0, 4), p(-2, 7), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-13, 23), p(3, 17), p(-0, 10), p(-0, 10), p(4, 6), p(0, 2), p(10, 5)],
    // SemiClosed
    [p(0, 0), p(12, -13), p(7, 6), p(3, -0), p(6, 1), p(1, 4), p(3, 5), p(2, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(23, 6),
    p(2, 5),
    p(1, 4),
    p(-7, 11),
    p(6, 1),
    p(-10, -8),
    p(1, 2),
    p(-4, -6),
    p(1, -2),
    p(-11, 2),
    p(-9, -15),
    p(-17, 2),
    p(6, -5),
    p(-3, -3),
    p(8, -1),
    p(2, 20),
    p(-3, -3),
    p(-21, 0),
    p(-17, -2),
    p(-48, 22),
    p(-17, 5),
    p(-18, -15),
    p(8, 22),
    p(-58, 32),
    p(-16, -14),
    p(-21, -7),
    p(-39, -31),
    p(-42, 14),
    p(-20, 4),
    p(9, 1),
    p(-95, 115),
    p(0, 0),
    p(1, -2),
    p(-15, 1),
    p(-4, -1),
    p(-27, 10),
    p(-28, -6),
    p(-54, -20),
    p(-36, 39),
    p(-46, 30),
    p(-8, 0),
    p(-22, 1),
    p(5, -6),
    p(-21, 46),
    p(-56, 16),
    p(-14, -27),
    p(0, 0),
    p(0, 0),
    p(9, -9),
    p(-8, 19),
    p(-4, -49),
    p(0, 0),
    p(3, -7),
    p(-43, -3),
    p(0, 0),
    p(0, 0),
    p(-24, 7),
    p(-18, 6),
    p(-13, 24),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(21, 1),
    p(-1, -3),
    p(-5, 2),
    p(-21, -0),
    p(5, -3),
    p(-28, -4),
    p(-18, 3),
    p(-37, -5),
    p(4, -3),
    p(-14, -7),
    p(-28, -4),
    p(-43, 6),
    p(-10, -1),
    p(-44, 3),
    p(-38, -6),
    p(-54, 70),
    p(10, -3),
    p(-3, -6),
    p(-7, -11),
    p(-29, -5),
    p(-10, 2),
    p(-19, -6),
    p(-26, -1),
    p(-85, 180),
    p(-7, -10),
    p(-29, -9),
    p(-41, -25),
    p(2, -84),
    p(-18, -4),
    p(-20, -10),
    p(-81, 64),
    p(0, 0),
    p(16, -2),
    p(2, -2),
    p(-11, -5),
    p(-20, -6),
    p(-2, 1),
    p(-29, -11),
    p(-17, 1),
    p(-30, 3),
    p(0, -6),
    p(-22, -4),
    p(-25, -13),
    p(-38, -3),
    p(-12, -2),
    p(-49, -10),
    p(-17, 23),
    p(-65, 59),
    p(8, -0),
    p(-8, 1),
    p(-25, 58),
    p(0, 0),
    p(-16, 1),
    p(-22, 3),
    p(0, 0),
    p(0, 0),
    p(-13, 5),
    p(-38, 17),
    p(-34, -41),
    p(0, 0),
    p(6, -56),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-6, 9),    /*0b0000*/
    p(-14, 9),   /*0b0001*/
    p(-4, 10),   /*0b0010*/
    p(-8, 11),   /*0b0011*/
    p(-3, 5),    /*0b0100*/
    p(-27, 3),   /*0b0101*/
    p(-11, 3),   /*0b0110*/
    p(-18, -14), /*0b0111*/
    p(11, 8),    /*0b1000*/
    p(-1, 12),   /*0b1001*/
    p(3, 9),     /*0b1010*/
    p(-0, 9),    /*0b1011*/
    p(1, 6),     /*0b1100*/
    p(-25, 10),  /*0b1101*/
    p(-9, 4),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(8, 12),    /*0b10000*/
    p(5, 8),     /*0b10001*/
    p(19, 10),   /*0b10010*/
    p(-3, 6),    /*0b10011*/
    p(-2, 3),    /*0b10100*/
    p(13, 12),   /*0b10101*/
    p(-19, -1),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(17, 12),   /*0b11000*/
    p(27, 14),   /*0b11001*/
    p(37, 26),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(14, 0),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(19, 8),    /*0b100000*/
    p(6, 12),    /*0b100001*/
    p(24, 2),    /*0b100010*/
    p(8, -1),    /*0b100011*/
    p(-5, 3),    /*0b100100*/
    p(-23, -6),  /*0b100101*/
    p(-22, 16),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(28, 3),    /*0b101000*/
    p(2, 17),    /*0b101001*/
    p(21, -4),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-2, 9),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(17, 5),    /*0b110000*/
    p(22, 5),    /*0b110001*/
    p(29, -2),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(4, 18),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(29, -1),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(7, -4),    /*0b111111*/
    p(-11, 1),   /*0b00*/
    p(3, -11),   /*0b01*/
    p(36, -4),   /*0b10*/
    p(22, -40),  /*0b11*/
    p(44, -8),   /*0b100*/
    p(-3, -14),  /*0b101*/
    p(65, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(65, -10),  /*0b1000*/
    p(15, -28),  /*0b1001*/
    p(72, -49),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(59, -35),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(15, 0),    /*0b1111*/
    p(20, 3),    /*0b00*/
    p(32, -7),   /*0b01*/
    p(25, -14),  /*0b10*/
    p(22, -38),  /*0b11*/
    p(38, -5),   /*0b100*/
    p(52, -15),  /*0b101*/
    p(23, -20),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(40, -1),   /*0b1000*/
    p(51, -16),  /*0b1001*/
    p(51, -40),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(42, -25),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(21, -44),  /*0b1111*/
];
const STOPPABLE_PASSER: PhasedScore = p(37, -49);
const CLOSE_KING_PASSER: PhasedScore = p(-7, 29);
const IMMOBILE_PASSER: PhasedScore = p(-8, -36);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -21,   36),    p( -36,   51),    p( -43,   50),    p( -37,   38),    p( -25,   26),    p( -28,   33),    p( -23,   42),    p( -26,   41),
        p( -11,   36),    p( -40,   58),    p( -43,   51),    p( -40,   40),    p( -43,   40),    p( -38,   44),    p( -46,   62),    p( -21,   44),
        p(  -3,   63),    p( -14,   64),    p( -37,   66),    p( -32,   62),    p( -41,   60),    p( -31,   63),    p( -43,   78),    p( -37,   75),
        p(  15,   85),    p(  10,   87),    p(  15,   75),    p(  -7,   81),    p( -26,   81),    p( -13,   85),    p( -28,   97),    p( -30,   99),
        p(  32,  132),    p(  38,  131),    p(  31,  115),    p(  14,   94),    p(  15,  102),    p(  -4,  118),    p( -20,  119),    p( -50,  142),
        p(  37,   90),    p(  32,   88),    p(  25,   92),    p(  36,   75),    p(  22,   79),    p(  21,   83),    p( -19,  100),    p( -16,   99),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -8);
const DOUBLED_PAWN: PhasedScore = p(-7, -22);
const PHALANX: [PhasedScore; 6] = [p(-0, -2), p(5, 1), p(9, 5), p(24, 22), p(65, 76), p(-100, 221)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 12), p(7, 15), p(14, 19), p(10, 8), p(-3, 16), p(-48, 11)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(39, 7), p(39, 33), p(51, -12), p(35, -37), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-44, -73),
        p(-25, -32),
        p(-12, -9),
        p(-3, 5),
        p(4, 15),
        p(11, 26),
        p(19, 29),
        p(25, 32),
        p(30, 30),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-31, -55),
        p(-19, -37),
        p(-8, -22),
        p(-0, -10),
        p(7, -0),
        p(12, 8),
        p(17, 13),
        p(21, 17),
        p(23, 22),
        p(30, 23),
        p(35, 23),
        p(42, 26),
        p(38, 34),
        p(53, 26),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-75, 10),
        p(-66, 24),
        p(-62, 30),
        p(-59, 35),
        p(-59, 42),
        p(-53, 48),
        p(-50, 53),
        p(-46, 56),
        p(-43, 61),
        p(-39, 66),
        p(-36, 69),
        p(-35, 74),
        p(-27, 76),
        p(-19, 74),
        p(-15, 73),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-28, -44),
        p(-28, 10),
        p(-32, 61),
        p(-28, 80),
        p(-26, 98),
        p(-22, 104),
        p(-18, 115),
        p(-14, 121),
        p(-10, 125),
        p(-7, 126),
        p(-4, 129),
        p(0, 132),
        p(3, 132),
        p(4, 137),
        p(7, 139),
        p(10, 142),
        p(11, 150),
        p(13, 151),
        p(23, 149),
        p(36, 144),
        p(39, 146),
        p(82, 124),
        p(82, 128),
        p(103, 111),
        p(194, 80),
        p(240, 38),
        p(265, 30),
        p(322, -10),
    ],
    [
        p(-83, 1),
        p(-52, -9),
        p(-26, -9),
        p(1, -5),
        p(29, -3),
        p(49, -1),
        p(76, 5),
        p(99, 6),
        p(143, -6),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-9, 7), p(0, 0), p(23, 19), p(49, -11), p(20, -32), p(0, 0)],
    [p(-2, 10), p(20, 22), p(0, 0), p(31, 5), p(30, 53), p(0, 0)],
    [p(-3, 13), p(10, 16), p(17, 13), p(0, 0), p(45, -4), p(0, 0)],
    [p(-2, 4), p(2, 6), p(-1, 22), p(1, 2), p(0, 0), p(0, 0)],
    [p(65, 19), p(-35, 18), p(-9, 17), p(-21, 7), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 7), p(8, 7), p(6, 10), p(12, 7), p(6, 19), p(10, 6)],
    [p(2, 7), p(11, 21), p(-142, -16), p(8, 14), p(10, 18), p(4, 7)],
    [p(3, 3), p(13, 9), p(9, 14), p(11, 10), p(10, 24), p(21, -4)],
    [p(2, -1), p(9, 2), p(7, -3), p(4, 15), p(-61, -255), p(5, -10)],
    [p(60, -1), p(37, 8), p(43, 2), p(21, 6), p(33, -10), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-18, -16), p(19, -9), p(10, -3), p(15, -11), p(-1, 12), p(-4, 4)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(14, 19), p(34, 0), p(5, 32)];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static + StaticallyNamedEntity {
    type Score: ScoreType;

    fn psqt(&self, square: ChessSquare, piece: ChessPieceType, color: ChessColor) -> SingleFeatureScore<Self::Score>;

    fn passed_pawn(square: ChessSquare) -> SingleFeatureScore<Self::Score>;

    fn stoppable_passer() -> SingleFeatureScore<Self::Score>;

    fn close_king_passer() -> SingleFeatureScore<Self::Score>;

    fn immobile_passer() -> SingleFeatureScore<Self::Score>;

    fn unsupported_pawn() -> SingleFeatureScore<Self::Score>;

    fn doubled_pawn() -> SingleFeatureScore<Self::Score>;

    fn phalanx(rank: DimT) -> SingleFeatureScore<Self::Score>;

    fn bishop_pair() -> SingleFeatureScore<Self::Score>;

    fn bad_bishop(num_pawns: usize) -> SingleFeatureScore<Self::Score>;

    fn rook_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score>;

    fn king_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score>;

    fn bishop_openness(openness: FileOpenness, len: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_advanced_center(config: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_passive_center(config: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_shield(&self, color: ChessColor, config: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_protection(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn pawn_attack(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn mobility(piece: ChessPieceType, mobility: usize) -> SingleFeatureScore<Self::Score>;

    fn threats(attacking: ChessPieceType, targeted: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn king_zone_attack(attacking: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn can_give_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;
}

/// Eval values tuned on a combination of the lichess-big-3-resolved dataset and a dataset used by 4ku,
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

    fn stoppable_passer() -> PhasedScore {
        STOPPABLE_PASSER
    }

    fn close_king_passer() -> SingleFeatureScore<Self::Score> {
        CLOSE_KING_PASSER
    }

    fn immobile_passer() -> SingleFeatureScore<Self::Score> {
        IMMOBILE_PASSER
    }

    fn unsupported_pawn() -> PhasedScore {
        UNSUPPORTED_PAWN
    }

    fn doubled_pawn() -> PhasedScore {
        DOUBLED_PAWN
    }

    fn phalanx(rank: DimT) -> PhasedScore {
        PHALANX[rank as usize]
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

    fn bishop_openness(openness: FileOpenness, len: usize) -> PhasedScore {
        BISHOP_OPENNESS[openness as usize][len - 1]
    }

    fn pawn_advanced_center(config: usize) -> PhasedScore {
        PAWN_ADVANCED_CENTER[config]
    }

    fn pawn_passive_center(config: usize) -> PhasedScore {
        PAWN_PASSIVE_CENTER[config]
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
