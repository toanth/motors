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
        p( 131,  191),    p( 127,  189),    p( 120,  193),    p( 129,  177),    p( 116,  181),    p( 115,  184),    p(  75,  201),    p(  79,  201),
        p(  71,  120),    p(  72,  122),    p(  80,  114),    p(  88,  116),    p(  77,  114),    p( 125,  106),    p( 100,  127),    p(  96,  118),
        p(  54,  110),    p(  64,  104),    p(  64,   99),    p(  84,   99),    p(  92,   98),    p(  85,   88),    p(  77,   99),    p(  73,   94),
        p(  50,   97),    p(  54,   99),    p(  77,   93),    p(  95,   95),    p(  92,   97),    p(  86,   94),    p(  70,   89),    p(  61,   84),
        p(  43,   95),    p(  51,   91),    p(  71,   95),    p(  82,   96),    p(  84,   95),    p(  77,   93),    p(  69,   81),    p(  53,   84),
        p(  54,   98),    p(  58,   97),    p(  63,   96),    p(  60,  104),    p(  62,  105),    p(  77,   97),    p(  81,   86),    p(  59,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 174,  280),    p( 195,  313),    p( 212,  326),    p( 251,  314),    p( 281,  316),    p( 198,  311),    p( 214,  310),    p( 201,  264),
        p( 266,  314),    p( 283,  319),    p( 298,  311),    p( 303,  315),    p( 301,  311),    p( 315,  299),    p( 273,  317),    p( 271,  306),
        p( 285,  310),    p( 306,  305),    p( 308,  311),    p( 322,  315),    p( 338,  310),    p( 352,  298),    p( 293,  305),    p( 285,  309),
        p( 301,  316),    p( 309,  310),    p( 325,  314),    p( 326,  321),    p( 325,  318),    p( 321,  317),    p( 311,  313),    p( 319,  311),
        p( 298,  317),    p( 303,  307),    p( 313,  313),    p( 320,  315),    p( 319,  319),    p( 325,  303),    p( 322,  304),    p( 312,  313),
        p( 275,  303),    p( 282,  302),    p( 296,  297),    p( 301,  310),    p( 305,  308),    p( 295,  292),    p( 301,  294),    p( 293,  307),
        p( 269,  311),    p( 280,  315),    p( 285,  305),    p( 294,  309),    p( 298,  304),    p( 289,  303),    p( 294,  307),    p( 289,  321),
        p( 240,  308),    p( 281,  305),    p( 266,  308),    p( 287,  312),    p( 295,  310),    p( 292,  299),    p( 287,  309),    p( 264,  309),
    ],
    // bishop
    [
        p( 275,  311),    p( 250,  316),    p( 238,  309),    p( 221,  319),    p( 215,  317),    p( 223,  310),    p( 271,  305),    p( 249,  311),
        p( 282,  303),    p( 278,  305),    p( 289,  308),    p( 275,  307),    p( 287,  303),    p( 291,  302),    p( 267,  309),    p( 270,  304),
        p( 295,  310),    p( 307,  305),    p( 291,  305),    p( 306,  301),    p( 305,  302),    p( 335,  306),    p( 316,  303),    p( 318,  313),
        p( 286,  312),    p( 291,  308),    p( 303,  303),    p( 306,  307),    p( 306,  304),    p( 299,  306),    p( 298,  309),    p( 280,  311),
        p( 289,  306),    p( 283,  309),    p( 295,  304),    p( 308,  304),    p( 302,  301),    p( 299,  303),    p( 285,  306),    p( 309,  300),
        p( 296,  308),    p( 299,  304),    p( 299,  306),    p( 299,  304),    p( 305,  306),    p( 297,  300),    p( 305,  296),    p( 307,  298),
        p( 308,  306),    p( 303,  301),    p( 307,  301),    p( 299,  309),    p( 301,  307),    p( 302,  305),    p( 311,  296),    p( 308,  295),
        p( 298,  304),    p( 309,  306),    p( 307,  308),    p( 290,  311),    p( 306,  309),    p( 294,  311),    p( 302,  299),    p( 301,  293),
    ],
    // rook
    [
        p( 460,  547),    p( 447,  558),    p( 440,  564),    p( 438,  561),    p( 450,  557),    p( 471,  552),    p( 479,  551),    p( 491,  544),
        p( 443,  553),    p( 440,  559),    p( 449,  560),    p( 464,  551),    p( 450,  554),    p( 466,  548),    p( 475,  545),    p( 491,  535),
        p( 443,  549),    p( 462,  544),    p( 456,  546),    p( 456,  541),    p( 482,  531),    p( 492,  527),    p( 510,  526),    p( 484,  529),
        p( 440,  549),    p( 446,  545),    p( 446,  547),    p( 451,  542),    p( 455,  533),    p( 467,  529),    p( 467,  533),    p( 466,  529),
        p( 434,  546),    p( 432,  545),    p( 434,  544),    p( 438,  541),    p( 446,  536),    p( 440,  536),    p( 453,  531),    p( 448,  529),
        p( 429,  543),    p( 429,  540),    p( 431,  539),    p( 434,  538),    p( 439,  532),    p( 450,  525),    p( 466,  514),    p( 454,  518),
        p( 432,  539),    p( 436,  538),    p( 442,  538),    p( 444,  535),    p( 451,  529),    p( 464,  519),    p( 472,  515),    p( 443,  525),
        p( 442,  544),    p( 438,  539),    p( 439,  542),    p( 444,  536),    p( 449,  529),    p( 456,  530),    p( 452,  530),    p( 447,  533),
    ],
    // queen
    [
        p( 877,  963),    p( 878,  977),    p( 894,  990),    p( 914,  984),    p( 912,  989),    p( 932,  977),    p( 978,  930),    p( 924,  961),
        p( 887,  952),    p( 862,  982),    p( 863, 1009),    p( 857, 1026),    p( 863, 1038),    p( 903,  999),    p( 904,  985),    p( 945,  964),
        p( 892,  959),    p( 885,  972),    p( 884,  993),    p( 884, 1005),    p( 906, 1006),    p( 944,  991),    p( 952,  964),    p( 939,  971),
        p( 879,  970),    p( 884,  977),    p( 878,  987),    p( 878,  998),    p( 880, 1011),    p( 895, 1002),    p( 904, 1004),    p( 911,  981),
        p( 890,  959),    p( 876,  979),    p( 882,  978),    p( 882,  993),    p( 887,  991),    p( 888,  992),    p( 901,  982),    p( 907,  975),
        p( 885,  948),    p( 892,  963),    p( 886,  976),    p( 883,  978),    p( 888,  986),    p( 895,  976),    p( 909,  959),    p( 907,  950),
        p( 886,  952),    p( 886,  958),    p( 892,  960),    p( 892,  974),    p( 894,  972),    p( 895,  956),    p( 907,  935),    p( 914,  911),
        p( 871,  954),    p( 883,  941),    p( 884,  953),    p( 892,  955),    p( 895,  943),    p( 882,  948),    p( 883,  942),    p( 888,  926),
    ],
    // king
    [
        p( 156,  -68),    p(  65,  -22),    p(  82,  -12),    p(  15,   15),    p(  42,    4),    p(  25,   16),    p(  84,    5),    p( 218,  -71),
        p( -32,   17),    p( -62,   31),    p( -64,   37),    p(   1,   27),    p( -33,   37),    p( -53,   50),    p( -28,   38),    p(  12,   17),
        p( -50,   22),    p( -41,   22),    p( -80,   34),    p( -89,   44),    p( -53,   39),    p( -19,   32),    p( -58,   34),    p( -29,   21),
        p( -29,    7),    p( -97,   15),    p(-113,   29),    p(-134,   37),    p(-132,   37),    p(-109,   30),    p(-121,   22),    p(-101,   22),
        p( -42,   -3),    p(-106,    4),    p(-121,   20),    p(-147,   33),    p(-145,   31),    p(-120,   19),    p(-134,   12),    p(-117,   14),
        p( -33,    0),    p( -82,   -2),    p(-111,   11),    p(-119,   20),    p(-117,   21),    p(-126,   14),    p(-102,    3),    p( -72,   10),
        p(  26,   -8),    p( -70,   -5),    p( -80,    1),    p(-100,   10),    p(-105,   12),    p( -92,    5),    p( -66,   -9),    p(   2,   -1),
        p(  49,  -20),    p(  41,  -32),    p(  39,  -20),    p( -21,   -3),    p(  29,  -17),    p( -18,   -4),    p(  34,  -25),    p(  60,  -31),
    ],
];

const BISHOP_PAIR: PhasedScore = p(24, 51);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(9, 20), p(10, 18), p(10, 7), p(6, -0), p(1, -7), p(-2, -17), p(-8, -25), p(-15, -38), p(-26, -47)];
const ROOK_OPEN_FILE: PhasedScore = p(15, 3);
const ROOK_CLOSED_FILE: PhasedScore = p(-11, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(5, -1);
const KING_OPEN_FILE: PhasedScore = p(-50, 0);
const KING_CLOSED_FILE: PhasedScore = p(13, -9);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 7);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 4), p(0, 6), p(-2, 5), p(2, 3), p(2, 5), p(3, 7), p(6, 5), p(18, 2)],
    // Closed
    [p(0, 0), p(0, 0), p(20, -29), p(-14, 9), p(-1, 12), p(-0, 5), p(-2, 7), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-14, 24), p(2, 17), p(-0, 10), p(-1, 10), p(4, 6), p(0, 3), p(10, 5)],
    // SemiClosed
    [p(0, 0), p(12, -13), p(7, 6), p(3, -0), p(6, 1), p(0, 4), p(3, 5), p(2, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(21, 7),
    p(2, 5),
    p(-2, 6),
    p(-9, 13),
    p(6, 1),
    p(-9, -8),
    p(-0, 2),
    p(-4, -6),
    p(0, -2),
    p(-10, 2),
    p(-10, -13),
    p(-17, 3),
    p(7, -5),
    p(-0, -3),
    p(8, -2),
    p(4, 20),
    p(-4, -3),
    p(-21, -0),
    p(-20, -0),
    p(-49, 24),
    p(-17, 5),
    p(-16, -15),
    p(7, 21),
    p(-59, 32),
    p(-15, -14),
    p(-18, -8),
    p(-39, -29),
    p(-39, 14),
    p(-19, 3),
    p(12, 1),
    p(-95, 115),
    p(0, 0),
    p(1, -2),
    p(-14, 0),
    p(-5, 1),
    p(-27, 11),
    p(-27, -6),
    p(-52, -20),
    p(-36, 38),
    p(-46, 29),
    p(-6, -0),
    p(-20, 1),
    p(5, -4),
    p(-20, 47),
    p(-54, 15),
    p(-10, -28),
    p(0, 0),
    p(0, 0),
    p(10, -10),
    p(-5, 18),
    p(-6, -49),
    p(0, 0),
    p(4, -7),
    p(-42, -0),
    p(0, 0),
    p(0, 0),
    p(-22, 6),
    p(-14, 5),
    p(-14, 27),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(21, 1),
    p(0, -3),
    p(-5, 2),
    p(-19, -1),
    p(5, -4),
    p(-27, -5),
    p(-18, 2),
    p(-36, -6),
    p(5, -3),
    p(-13, -7),
    p(-27, -4),
    p(-41, 5),
    p(-9, -2),
    p(-42, 2),
    p(-36, -7),
    p(-53, 70),
    p(9, -5),
    p(-2, -7),
    p(-7, -13),
    p(-28, -7),
    p(-10, 0),
    p(-18, -8),
    p(-26, -3),
    p(-83, 177),
    p(-6, -11),
    p(-27, -10),
    p(-40, -27),
    p(3, -85),
    p(-17, -5),
    p(-18, -12),
    p(-80, 62),
    p(0, 0),
    p(15, -3),
    p(2, -3),
    p(-11, -6),
    p(-19, -7),
    p(-2, 0),
    p(-28, -12),
    p(-17, 0),
    p(-30, 2),
    p(0, -7),
    p(-21, -5),
    p(-25, -14),
    p(-36, -4),
    p(-13, -3),
    p(-48, -11),
    p(-17, 22),
    p(-65, 58),
    p(7, -1),
    p(-8, 1),
    p(-26, 59),
    p(0, 0),
    p(-18, 1),
    p(-22, 3),
    p(0, 0),
    p(0, 0),
    p(-13, 5),
    p(-38, 17),
    p(-35, -41),
    p(0, 0),
    p(3, -55),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-6, 8),    /*0b0000*/
    p(-13, 9),   /*0b0001*/
    p(-4, 10),   /*0b0010*/
    p(-9, 11),   /*0b0011*/
    p(-3, 5),    /*0b0100*/
    p(-27, 3),   /*0b0101*/
    p(-12, 3),   /*0b0110*/
    p(-19, -13), /*0b0111*/
    p(11, 8),    /*0b1000*/
    p(-1, 12),   /*0b1001*/
    p(3, 9),     /*0b1010*/
    p(-1, 9),    /*0b1011*/
    p(1, 6),     /*0b1100*/
    p(-25, 10),  /*0b1101*/
    p(-9, 4),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(8, 13),    /*0b10000*/
    p(5, 8),     /*0b10001*/
    p(20, 10),   /*0b10010*/
    p(-3, 6),    /*0b10011*/
    p(-3, 4),    /*0b10100*/
    p(12, 12),   /*0b10101*/
    p(-19, -1),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(17, 12),   /*0b11000*/
    p(27, 14),   /*0b11001*/
    p(37, 26),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(15, 0),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(19, 8),    /*0b100000*/
    p(6, 12),    /*0b100001*/
    p(23, 2),    /*0b100010*/
    p(8, -1),    /*0b100011*/
    p(-5, 3),    /*0b100100*/
    p(-23, -5),  /*0b100101*/
    p(-22, 16),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(29, 3),    /*0b101000*/
    p(1, 17),    /*0b101001*/
    p(20, -4),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-2, 9),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(17, 5),    /*0b110000*/
    p(22, 5),    /*0b110001*/
    p(30, -3),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(3, 18),    /*0b110100*/
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
    p(-10, -0),  /*0b00*/
    p(3, -11),   /*0b01*/
    p(36, -4),   /*0b10*/
    p(23, -41),  /*0b11*/
    p(45, -8),   /*0b100*/
    p(-6, -13),  /*0b101*/
    p(65, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(66, -11),  /*0b1000*/
    p(16, -28),  /*0b1001*/
    p(74, -50),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(60, -36),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(14, 0),    /*0b1111*/
    p(22, 1),    /*0b00*/
    p(33, -8),   /*0b01*/
    p(26, -14),  /*0b10*/
    p(22, -38),  /*0b11*/
    p(39, -6),   /*0b100*/
    p(54, -16),  /*0b101*/
    p(24, -20),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(41, -1),   /*0b1000*/
    p(52, -16),  /*0b1001*/
    p(53, -40),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(43, -26),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(22, -45),  /*0b1111*/
];
const STOPPABLE_PASSER: PhasedScore = p(37, -49);
const CLOSE_KING_PASSER: PhasedScore = p(-8, 30);
const IMMOBILE_PASSER: PhasedScore = p(-9, -35);
const PROTECTED_PASSER: PhasedScore = p(9, -3);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -41,   43),    p( -49,   56),    p( -70,   60),    p( -73,   52),    p( -61,   38),    p( -49,   40),    p( -31,   45),    p( -47,   48),
        p( -28,   42),    p( -52,   62),    p( -62,   58),    p( -64,   49),    p( -70,   49),    p( -55,   49),    p( -56,   65),    p( -39,   49),
        p( -19,   68),    p( -28,   69),    p( -56,   73),    p( -56,   70),    p( -64,   67),    p( -47,   69),    p( -54,   82),    p( -52,   80),
        p(   2,   89),    p(  -5,   92),    p(  -4,   81),    p( -29,   88),    p( -47,   88),    p( -30,   90),    p( -40,  100),    p( -43,  103),
        p(  19,  136),    p(  25,  134),    p(  17,  120),    p(  -3,   99),    p(  -1,  107),    p( -17,  122),    p( -31,  122),    p( -63,  146),
        p(  31,   91),    p(  27,   89),    p(  20,   93),    p(  29,   77),    p(  16,   81),    p(  15,   84),    p( -25,  101),    p( -21,  101),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -8);
const DOUBLED_PAWN: PhasedScore = p(-7, -22);
const PHALANX: [PhasedScore; 6] = [p(-1, -2), p(5, 2), p(9, 5), p(24, 22), p(68, 75), p(-98, 221)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 12), p(7, 15), p(14, 19), p(10, 8), p(-3, 17), p(-46, 11)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(38, 8), p(39, 33), p(51, -12), p(34, -37), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-45, -72),
        p(-25, -32),
        p(-13, -8),
        p(-4, 5),
        p(3, 16),
        p(10, 26),
        p(17, 30),
        p(24, 32),
        p(28, 31),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-1, -10),
        p(6, -0),
        p(11, 8),
        p(16, 13),
        p(20, 17),
        p(22, 22),
        p(29, 24),
        p(33, 23),
        p(40, 26),
        p(36, 35),
        p(50, 27),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-76, 11),
        p(-67, 25),
        p(-63, 31),
        p(-60, 36),
        p(-60, 43),
        p(-55, 49),
        p(-52, 54),
        p(-48, 57),
        p(-44, 62),
        p(-41, 67),
        p(-38, 70),
        p(-37, 76),
        p(-29, 77),
        p(-21, 75),
        p(-17, 74),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-29, -46),
        p(-29, 9),
        p(-33, 60),
        p(-29, 79),
        p(-27, 97),
        p(-22, 103),
        p(-19, 114),
        p(-15, 120),
        p(-11, 124),
        p(-8, 126),
        p(-5, 129),
        p(-1, 131),
        p(1, 132),
        p(3, 137),
        p(5, 139),
        p(9, 143),
        p(9, 151),
        p(11, 152),
        p(21, 151),
        p(34, 145),
        p(38, 148),
        p(81, 125),
        p(80, 130),
        p(102, 113),
        p(194, 81),
        p(238, 41),
        p(268, 31),
        p(322, -8),
    ],
    [
        p(-86, 3),
        p(-53, -8),
        p(-26, -8),
        p(1, -4),
        p(30, -3),
        p(51, -1),
        p(79, 4),
        p(102, 5),
        p(147, -8),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-9, 7), p(0, 0), p(23, 19), p(48, -11), p(20, -32), p(0, 0)],
    [p(-2, 10), p(20, 22), p(0, 0), p(31, 5), p(30, 53), p(0, 0)],
    [p(-3, 13), p(10, 16), p(17, 13), p(0, 0), p(45, -4), p(0, 0)],
    [p(-2, 3), p(2, 7), p(-1, 23), p(1, 3), p(0, 0), p(0, 0)],
    [p(62, 19), p(-35, 18), p(-8, 17), p(-21, 7), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 8), p(8, 7), p(6, 10), p(12, 7), p(6, 18), p(10, 6)],
    [p(2, 7), p(11, 21), p(-142, -16), p(8, 13), p(10, 18), p(4, 7)],
    [p(2, 3), p(13, 8), p(9, 13), p(11, 10), p(10, 25), p(21, -4)],
    [p(2, 0), p(9, 1), p(7, -3), p(4, 14), p(-58, -260), p(5, -10)],
    [p(61, -2), p(38, 8), p(44, 2), p(22, 6), p(34, -10), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-19, -16), p(19, -9), p(10, -3), p(15, -11), p(-1, 12), p(-2, 4)];
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

    fn passer_protection() -> SingleFeatureScore<Self::Score>;

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

    fn passer_protection() -> SingleFeatureScore<Self::Score> {
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
