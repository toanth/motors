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
        p( 133,  190),    p( 130,  188),    p( 122,  192),    p( 133,  175),    p( 120,  179),    p( 119,  183),    p(  79,  199),    p(  81,  200),
        p(  71,  121),    p(  71,  122),    p(  79,  114),    p(  88,  116),    p(  77,  115),    p( 126,  106),    p( 101,  127),    p(  96,  119),
        p(  54,  110),    p(  64,  104),    p(  63,   99),    p(  84,   99),    p(  93,   98),    p(  85,   89),    p(  77,   99),    p(  73,   94),
        p(  49,   97),    p(  54,  100),    p(  78,   93),    p(  93,   95),    p(  92,   97),    p(  87,   94),    p(  71,   89),    p(  61,   84),
        p(  43,   95),    p(  51,   91),    p(  72,   96),    p(  82,   97),    p(  84,   95),    p(  77,   94),    p(  70,   81),    p(  53,   84),
        p(  54,   99),    p(  58,   97),    p(  62,   97),    p(  59,  104),    p(  61,  106),    p(  77,   97),    p(  82,   86),    p(  59,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 174,  280),    p( 196,  313),    p( 212,  325),    p( 251,  314),    p( 281,  316),    p( 199,  310),    p( 215,  310),    p( 201,  264),
        p( 266,  313),    p( 284,  318),    p( 299,  310),    p( 304,  315),    p( 302,  310),    p( 315,  299),    p( 274,  316),    p( 271,  306),
        p( 286,  309),    p( 306,  304),    p( 309,  311),    p( 323,  315),    p( 339,  309),    p( 353,  298),    p( 294,  304),    p( 286,  308),
        p( 302,  315),    p( 310,  309),    p( 326,  313),    p( 327,  321),    p( 326,  318),    p( 321,  317),    p( 311,  312),    p( 320,  311),
        p( 299,  317),    p( 304,  306),    p( 313,  312),    p( 321,  315),    p( 320,  318),    p( 325,  303),    p( 323,  304),    p( 312,  313),
        p( 276,  303),    p( 283,  302),    p( 297,  297),    p( 301,  310),    p( 306,  307),    p( 295,  292),    p( 302,  294),    p( 293,  307),
        p( 270,  310),    p( 281,  314),    p( 285,  304),    p( 295,  309),    p( 299,  304),    p( 289,  302),    p( 295,  307),    p( 289,  321),
        p( 241,  308),    p( 282,  305),    p( 267,  307),    p( 287,  311),    p( 296,  309),    p( 292,  299),    p( 288,  308),    p( 265,  309),
    ],
    // bishop
    [
        p( 276,  311),    p( 251,  316),    p( 238,  309),    p( 221,  319),    p( 215,  317),    p( 224,  310),    p( 272,  305),    p( 249,  311),
        p( 282,  303),    p( 279,  305),    p( 290,  308),    p( 276,  306),    p( 288,  303),    p( 292,  302),    p( 267,  309),    p( 270,  304),
        p( 296,  309),    p( 307,  305),    p( 291,  305),    p( 306,  300),    p( 305,  302),    p( 336,  305),    p( 316,  303),    p( 318,  313),
        p( 287,  312),    p( 292,  308),    p( 303,  303),    p( 306,  307),    p( 306,  304),    p( 299,  305),    p( 298,  309),    p( 280,  311),
        p( 290,  306),    p( 284,  309),    p( 295,  304),    p( 308,  304),    p( 302,  300),    p( 299,  303),    p( 285,  305),    p( 310,  300),
        p( 296,  308),    p( 299,  304),    p( 299,  306),    p( 299,  304),    p( 305,  306),    p( 297,  300),    p( 305,  296),    p( 307,  298),
        p( 309,  306),    p( 303,  300),    p( 308,  301),    p( 299,  309),    p( 301,  306),    p( 302,  305),    p( 311,  296),    p( 308,  295),
        p( 298,  304),    p( 309,  306),    p( 307,  307),    p( 290,  311),    p( 306,  309),    p( 294,  310),    p( 302,  299),    p( 302,  293),
    ],
    // rook
    [
        p( 460,  547),    p( 448,  557),    p( 441,  564),    p( 439,  561),    p( 451,  557),    p( 472,  552),    p( 481,  551),    p( 492,  544),
        p( 443,  553),    p( 442,  559),    p( 451,  559),    p( 465,  550),    p( 451,  553),    p( 468,  548),    p( 477,  545),    p( 492,  535),
        p( 445,  549),    p( 464,  544),    p( 458,  545),    p( 458,  541),    p( 484,  530),    p( 494,  527),    p( 512,  526),    p( 485,  529),
        p( 441,  549),    p( 448,  544),    p( 447,  547),    p( 453,  541),    p( 457,  533),    p( 468,  528),    p( 469,  532),    p( 467,  529),
        p( 435,  545),    p( 434,  544),    p( 435,  544),    p( 439,  540),    p( 448,  536),    p( 442,  536),    p( 455,  530),    p( 449,  528),
        p( 430,  543),    p( 430,  540),    p( 432,  538),    p( 435,  537),    p( 440,  532),    p( 451,  524),    p( 467,  514),    p( 454,  517),
        p( 433,  538),    p( 437,  537),    p( 442,  537),    p( 445,  534),    p( 452,  528),    p( 465,  519),    p( 473,  515),    p( 443,  524),
        p( 442,  543),    p( 439,  539),    p( 440,  542),    p( 444,  535),    p( 450,  529),    p( 456,  529),    p( 452,  529),    p( 448,  532),
    ],
    // queen
    [
        p( 878,  963),    p( 879,  977),    p( 895,  989),    p( 915,  984),    p( 912,  989),    p( 933,  977),    p( 979,  929),    p( 925,  960),
        p( 889,  950),    p( 864,  981),    p( 865, 1008),    p( 858, 1025),    p( 864, 1037),    p( 904,  998),    p( 905,  984),    p( 948,  962),
        p( 894,  956),    p( 887,  971),    p( 886,  992),    p( 885, 1004),    p( 908, 1006),    p( 945,  990),    p( 953,  963),    p( 941,  970),
        p( 881,  968),    p( 885,  976),    p( 878,  986),    p( 879,  998),    p( 881, 1011),    p( 895, 1001),    p( 905, 1003),    p( 912,  981),
        p( 890,  959),    p( 877,  978),    p( 883,  977),    p( 883,  993),    p( 887,  991),    p( 888,  992),    p( 901,  982),    p( 908,  975),
        p( 886,  948),    p( 892,  963),    p( 886,  977),    p( 883,  978),    p( 888,  987),    p( 895,  977),    p( 909,  960),    p( 907,  950),
        p( 886,  952),    p( 886,  958),    p( 893,  960),    p( 892,  974),    p( 894,  972),    p( 895,  956),    p( 907,  936),    p( 914,  911),
        p( 872,  953),    p( 884,  941),    p( 885,  953),    p( 893,  954),    p( 896,  943),    p( 882,  948),    p( 884,  941),    p( 889,  925),
    ],
    // king
    [
        p( 154,  -67),    p(  64,  -22),    p(  83,  -12),    p(  15,   16),    p(  44,    4),    p(  25,   16),    p(  84,    5),    p( 220,  -71),
        p( -33,   17),    p( -61,   30),    p( -62,   36),    p(   3,   26),    p( -31,   36),    p( -51,   49),    p( -27,   38),    p(  12,   18),
        p( -51,   23),    p( -39,   21),    p( -78,   33),    p( -87,   43),    p( -51,   39),    p( -18,   31),    p( -57,   34),    p( -29,   21),
        p( -29,    8),    p( -95,   14),    p(-110,   28),    p(-132,   37),    p(-130,   36),    p(-107,   29),    p(-119,   21),    p(-100,   22),
        p( -41,   -3),    p(-104,    3),    p(-119,   19),    p(-145,   32),    p(-143,   30),    p(-118,   18),    p(-132,   11),    p(-115,   14),
        p( -32,    0),    p( -80,   -3),    p(-108,   10),    p(-117,   19),    p(-114,   20),    p(-123,   13),    p( -99,    2),    p( -70,   10),
        p(  27,   -8),    p( -68,   -6),    p( -78,    0),    p( -98,   10),    p(-104,   12),    p( -90,    4),    p( -64,  -10),    p(   2,   -1),
        p(  48,  -19),    p(  41,  -31),    p(  39,  -19),    p( -21,   -2),    p(  29,  -17),    p( -18,   -3),    p(  33,  -24),    p(  59,  -30),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 52);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(9, 20), p(10, 18), p(10, 7), p(6, -0), p(2, -8), p(-1, -17), p(-7, -26), p(-14, -39), p(-25, -48)];
const ROOK_OPEN_FILE: PhasedScore = p(15, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -0);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(4, 1);
const KING_OPEN_FILE: PhasedScore = p(-50, -0);
const KING_CLOSED_FILE: PhasedScore = p(13, -8);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 7);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-5, 4), p(1, 6), p(-2, 5), p(2, 3), p(2, 5), p(4, 7), p(7, 5), p(19, 2)],
    // Closed
    [p(0, 0), p(0, 0), p(21, -29), p(-14, 10), p(-1, 11), p(-0, 4), p(-2, 7), p(-1, 4)],
    // SemiOpen
    [p(0, 0), p(-14, 24), p(2, 17), p(-0, 10), p(-1, 10), p(4, 6), p(0, 2), p(10, 5)],
    // SemiClosed
    [p(0, 0), p(12, -13), p(7, 6), p(3, -0), p(6, 1), p(1, 4), p(3, 5), p(2, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(22, 6),
    p(2, 5),
    p(1, 4),
    p(-7, 12),
    p(6, 1),
    p(-10, -8),
    p(1, 2),
    p(-4, -6),
    p(0, -2),
    p(-11, 2),
    p(-9, -14),
    p(-16, 3),
    p(6, -5),
    p(-2, -3),
    p(8, -1),
    p(2, 21),
    p(-3, -3),
    p(-21, 0),
    p(-17, -2),
    p(-47, 23),
    p(-17, 5),
    p(-17, -15),
    p(8, 22),
    p(-58, 31),
    p(-16, -14),
    p(-20, -7),
    p(-38, -30),
    p(-40, 14),
    p(-20, 4),
    p(9, 1),
    p(-97, 116),
    p(0, 0),
    p(1, -2),
    p(-15, 1),
    p(-4, -0),
    p(-26, 10),
    p(-28, -6),
    p(-53, -20),
    p(-36, 39),
    p(-47, 30),
    p(-7, 0),
    p(-22, 1),
    p(5, -5),
    p(-21, 46),
    p(-56, 16),
    p(-15, -27),
    p(0, 0),
    p(0, 0),
    p(9, -9),
    p(-7, 18),
    p(-5, -48),
    p(0, 0),
    p(3, -7),
    p(-43, -3),
    p(0, 0),
    p(0, 0),
    p(-24, 7),
    p(-17, 6),
    p(-13, 27),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(21, 1),
    p(-0, -3),
    p(-5, 2),
    p(-20, -0),
    p(5, -3),
    p(-28, -4),
    p(-18, 3),
    p(-37, -5),
    p(5, -3),
    p(-14, -7),
    p(-28, -4),
    p(-42, 6),
    p(-9, -1),
    p(-43, 3),
    p(-37, -6),
    p(-54, 70),
    p(10, -4),
    p(-3, -6),
    p(-7, -12),
    p(-28, -6),
    p(-10, 2),
    p(-19, -7),
    p(-26, -1),
    p(-84, 180),
    p(-6, -10),
    p(-28, -9),
    p(-40, -25),
    p(3, -84),
    p(-17, -4),
    p(-19, -10),
    p(-81, 64),
    p(0, 0),
    p(15, -2),
    p(2, -3),
    p(-11, -5),
    p(-20, -6),
    p(-2, 1),
    p(-29, -11),
    p(-17, 1),
    p(-30, 3),
    p(1, -6),
    p(-21, -4),
    p(-25, -13),
    p(-37, -3),
    p(-12, -2),
    p(-49, -10),
    p(-17, 23),
    p(-65, 59),
    p(8, -0),
    p(-8, 1),
    p(-25, 58),
    p(0, 0),
    p(-17, 1),
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
    p(-18, -13), /*0b0111*/
    p(11, 8),    /*0b1000*/
    p(-1, 12),   /*0b1001*/
    p(3, 9),     /*0b1010*/
    p(-0, 9),    /*0b1011*/
    p(1, 5),     /*0b1100*/
    p(-25, 10),  /*0b1101*/
    p(-9, 4),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(8, 12),    /*0b10000*/
    p(5, 7),     /*0b10001*/
    p(20, 9),    /*0b10010*/
    p(-3, 6),    /*0b10011*/
    p(-2, 3),    /*0b10100*/
    p(13, 12),   /*0b10101*/
    p(-19, -1),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(17, 11),   /*0b11000*/
    p(27, 13),   /*0b11001*/
    p(37, 26),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(15, 0),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(19, 7),    /*0b100000*/
    p(6, 11),    /*0b100001*/
    p(24, 2),    /*0b100010*/
    p(8, -1),    /*0b100011*/
    p(-5, 3),    /*0b100100*/
    p(-23, -5),  /*0b100101*/
    p(-22, 16),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(28, 3),    /*0b101000*/
    p(1, 17),    /*0b101001*/
    p(21, -5),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-2, 9),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(17, 4),    /*0b110000*/
    p(22, 5),    /*0b110001*/
    p(29, -2),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(4, 17),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(29, -2),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(7, -4),    /*0b111111*/
    p(-11, 1),   /*0b00*/
    p(3, -10),   /*0b01*/
    p(37, -5),   /*0b10*/
    p(23, -40),  /*0b11*/
    p(44, -9),   /*0b100*/
    p(-4, -15),  /*0b101*/
    p(65, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(65, -11),  /*0b1000*/
    p(15, -28),  /*0b1001*/
    p(74, -49),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(60, -36),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(15, 0),    /*0b1111*/
    p(20, 3),    /*0b00*/
    p(32, -7),   /*0b01*/
    p(25, -13),  /*0b10*/
    p(22, -38),  /*0b11*/
    p(38, -6),   /*0b100*/
    p(52, -15),  /*0b101*/
    p(23, -20),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(40, -2),   /*0b1000*/
    p(51, -16),  /*0b1001*/
    p(52, -40),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(42, -26),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(21, -44),  /*0b1111*/
];
const STOPPABLE_PASSER: PhasedScore = p(37, -49);
const CLOSE_KING_PASSER: PhasedScore = p(-8, 28);
const IMMOBILE_PASSER: PhasedScore = p(-8, -36);
const PROTECTED_PASSER: PhasedScore = p(7, 3);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -27,   37),    p( -43,   53),    p( -54,   52),    p( -49,   41),    p( -36,   28),    p( -38,   35),    p( -28,   43),    p( -32,   42),
        p( -16,   37),    p( -45,   59),    p( -48,   52),    p( -48,   42),    p( -49,   42),    p( -44,   45),    p( -50,   63),    p( -27,   45),
        p(  -9,   64),    p( -19,   65),    p( -43,   67),    p( -40,   63),    p( -47,   60),    p( -38,   64),    p( -47,   78),    p( -43,   76),
        p(  10,   86),    p(   6,   88),    p(   8,   76),    p( -14,   82),    p( -33,   82),    p( -20,   85),    p( -32,   97),    p( -35,  100),
        p(  26,  133),    p(  32,  131),    p(  26,  115),    p(   7,   94),    p(   9,  102),    p( -10,  118),    p( -25,  119),    p( -55,  142),
        p(  33,   90),    p(  30,   88),    p(  22,   92),    p(  33,   75),    p(  20,   79),    p(  19,   83),    p( -21,   99),    p( -19,  100),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -8);
const DOUBLED_PAWN: PhasedScore = p(-8, -22);
const PHALANX: [PhasedScore; 6] = [p(-0, -2), p(5, 1), p(9, 5), p(24, 23), p(64, 76), p(-102, 222)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 12), p(7, 15), p(14, 19), p(10, 8), p(-3, 17), p(-47, 12)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(38, 7), p(39, 33), p(51, -12), p(35, -38), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-45, -73),
        p(-25, -32),
        p(-12, -9),
        p(-3, 4),
        p(4, 15),
        p(11, 26),
        p(19, 29),
        p(25, 31),
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
        p(-32, -55),
        p(-19, -37),
        p(-8, -22),
        p(-0, -10),
        p(7, -0),
        p(12, 8),
        p(17, 13),
        p(21, 17),
        p(23, 22),
        p(30, 23),
        p(34, 23),
        p(42, 25),
        p(38, 34),
        p(52, 25),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-54, 48),
        p(-51, 52),
        p(-47, 56),
        p(-43, 61),
        p(-40, 65),
        p(-36, 68),
        p(-35, 74),
        p(-28, 75),
        p(-19, 73),
        p(-16, 72),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(10, 143),
        p(11, 150),
        p(13, 151),
        p(23, 150),
        p(36, 144),
        p(39, 147),
        p(82, 124),
        p(82, 128),
        p(103, 112),
        p(194, 80),
        p(239, 39),
        p(267, 30),
        p(316, -6),
    ],
    [
        p(-84, 0),
        p(-52, -10),
        p(-26, -9),
        p(1, -5),
        p(29, -3),
        p(50, -1),
        p(77, 5),
        p(99, 6),
        p(144, -6),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(64, 19), p(-35, 18), p(-9, 17), p(-21, 7), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 8), p(8, 7), p(6, 10), p(12, 7), p(6, 19), p(11, 6)],
    [p(2, 7), p(11, 21), p(-143, -15), p(8, 13), p(10, 18), p(4, 7)],
    [p(3, 3), p(13, 9), p(9, 14), p(11, 10), p(10, 24), p(21, -4)],
    [p(2, -1), p(9, 2), p(7, -3), p(4, 15), p(-59, -259), p(5, -10)],
    [p(60, -1), p(38, 8), p(43, 2), p(21, 6), p(33, -9), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-18, -16), p(19, -9), p(10, -3), p(15, -11), p(-1, 13), p(-3, 4)];
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

    fn protected_passer_advance() -> SingleFeatureScore<Self::Score>;

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

    fn protected_passer_advance() -> SingleFeatureScore<Self::Score> {
        PROTECTED_PASSER
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
