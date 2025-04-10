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
        p( 132,  191),    p( 128,  190),    p( 120,  194),    p( 130,  177),    p( 116,  182),    p( 115,  185),    p(  74,  202),    p(  79,  201),
        p(  71,  120),    p(  72,  122),    p(  80,  114),    p(  88,  116),    p(  77,  115),    p( 125,  106),    p( 100,  127),    p(  95,  118),
        p(  54,  110),    p(  64,  104),    p(  64,   99),    p(  84,   99),    p(  92,   98),    p(  85,   88),    p(  77,   99),    p(  73,   94),
        p(  50,   97),    p(  54,   99),    p(  78,   93),    p(  95,   95),    p(  92,   97),    p(  86,   94),    p(  70,   89),    p(  61,   84),
        p(  43,   95),    p(  51,   91),    p(  72,   95),    p(  82,   97),    p(  84,   95),    p(  77,   93),    p(  69,   81),    p(  53,   84),
        p(  54,   98),    p(  58,   97),    p(  63,   96),    p(  60,  104),    p(  62,  105),    p(  77,   97),    p(  81,   85),    p(  59,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 173,  280),    p( 194,  313),    p( 211,  326),    p( 250,  314),    p( 279,  316),    p( 198,  311),    p( 215,  310),    p( 201,  264),
        p( 266,  314),    p( 283,  319),    p( 298,  311),    p( 303,  315),    p( 301,  310),    p( 313,  299),    p( 272,  317),    p( 272,  306),
        p( 285,  310),    p( 306,  305),    p( 308,  311),    p( 322,  315),    p( 337,  310),    p( 351,  298),    p( 292,  305),    p( 286,  309),
        p( 301,  316),    p( 309,  310),    p( 325,  314),    p( 326,  321),    p( 325,  318),    p( 321,  317),    p( 311,  313),    p( 319,  311),
        p( 298,  318),    p( 304,  307),    p( 313,  313),    p( 321,  315),    p( 319,  319),    p( 325,  304),    p( 322,  304),    p( 312,  313),
        p( 275,  303),    p( 283,  302),    p( 297,  297),    p( 301,  310),    p( 306,  308),    p( 296,  292),    p( 302,  294),    p( 293,  307),
        p( 269,  311),    p( 280,  315),    p( 285,  305),    p( 294,  309),    p( 298,  305),    p( 289,  303),    p( 294,  308),    p( 289,  321),
        p( 241,  308),    p( 281,  305),    p( 267,  308),    p( 287,  312),    p( 296,  310),    p( 292,  300),    p( 288,  309),    p( 264,  309),
    ],
    // bishop
    [
        p( 275,  312),    p( 249,  316),    p( 235,  309),    p( 220,  319),    p( 212,  317),    p( 227,  309),    p( 271,  305),    p( 251,  311),
        p( 282,  303),    p( 277,  305),    p( 289,  308),    p( 275,  306),    p( 287,  303),    p( 291,  301),    p( 263,  310),    p( 273,  303),
        p( 295,  309),    p( 307,  305),    p( 290,  305),    p( 306,  300),    p( 304,  302),    p( 335,  305),    p( 314,  303),    p( 317,  313),
        p( 286,  312),    p( 291,  308),    p( 303,  303),    p( 306,  307),    p( 306,  304),    p( 298,  305),    p( 298,  309),    p( 280,  311),
        p( 289,  306),    p( 284,  309),    p( 295,  304),    p( 308,  304),    p( 302,  301),    p( 299,  302),    p( 285,  305),    p( 309,  300),
        p( 296,  308),    p( 299,  304),    p( 299,  306),    p( 299,  304),    p( 305,  307),    p( 298,  300),    p( 305,  296),    p( 307,  298),
        p( 308,  307),    p( 304,  301),    p( 307,  301),    p( 299,  310),    p( 301,  307),    p( 303,  306),    p( 311,  296),    p( 308,  295),
        p( 298,  304),    p( 309,  307),    p( 307,  308),    p( 291,  311),    p( 306,  310),    p( 293,  312),    p( 303,  300),    p( 301,  292),
    ],
    // rook
    [
        p( 453,  547),    p( 439,  558),    p( 431,  566),    p( 430,  563),    p( 442,  558),    p( 468,  552),    p( 475,  552),    p( 486,  545),
        p( 443,  553),    p( 440,  559),    p( 450,  560),    p( 463,  552),    p( 450,  554),    p( 464,  549),    p( 474,  545),    p( 490,  535),
        p( 444,  549),    p( 463,  544),    p( 457,  546),    p( 457,  541),    p( 479,  532),    p( 492,  527),    p( 508,  527),    p( 484,  529),
        p( 441,  549),    p( 447,  545),    p( 447,  547),    p( 450,  544),    p( 455,  534),    p( 466,  529),    p( 467,  533),    p( 466,  529),
        p( 434,  546),    p( 433,  544),    p( 433,  546),    p( 439,  541),    p( 447,  537),    p( 441,  537),    p( 453,  532),    p( 448,  529),
        p( 430,  543),    p( 430,  541),    p( 432,  540),    p( 435,  539),    p( 441,  533),    p( 452,  525),    p( 467,  516),    p( 454,  519),
        p( 433,  540),    p( 437,  539),    p( 443,  539),    p( 445,  535),    p( 452,  529),    p( 466,  520),    p( 473,  516),    p( 443,  526),
        p( 442,  544),    p( 439,  539),    p( 440,  543),    p( 445,  536),    p( 450,  530),    p( 456,  530),    p( 452,  531),    p( 448,  533),
    ],
    // queen
    [
        p( 869,  964),    p( 871,  977),    p( 883,  991),    p( 905,  984),    p( 905,  988),    p( 928,  977),    p( 969,  932),    p( 916,  963),
        p( 888,  951),    p( 864,  979),    p( 865, 1006),    p( 858, 1023),    p( 864, 1035),    p( 903,  998),    p( 906,  982),    p( 946,  964),
        p( 893,  958),    p( 887,  971),    p( 885,  992),    p( 886, 1001),    p( 901, 1008),    p( 946,  988),    p( 950,  965),    p( 940,  971),
        p( 881,  969),    p( 884,  977),    p( 880,  984),    p( 878,  998),    p( 880, 1010),    p( 896,  999),    p( 903, 1006),    p( 911,  982),
        p( 889,  961),    p( 877,  978),    p( 882,  978),    p( 884,  991),    p( 888,  990),    p( 890,  990),    p( 900,  986),    p( 908,  976),
        p( 886,  947),    p( 892,  964),    p( 887,  976),    p( 884,  977),    p( 889,  986),    p( 897,  975),    p( 909,  962),    p( 907,  950),
        p( 886,  952),    p( 887,  958),    p( 894,  960),    p( 893,  973),    p( 895,  972),    p( 896,  956),    p( 907,  937),    p( 915,  911),
        p( 873,  952),    p( 884,  941),    p( 885,  952),    p( 894,  954),    p( 896,  943),    p( 883,  947),    p( 884,  942),    p( 890,  924),
    ],
    // king
    [
        p( 156,  -69),    p(  66,  -23),    p(  83,  -13),    p(  17,   14),    p(  43,    3),    p(  27,   15),    p(  85,    4),    p( 214,  -71),
        p( -33,   17),    p( -62,   30),    p( -63,   37),    p(  -1,   27),    p( -34,   37),    p( -52,   50),    p( -30,   38),    p(   8,   18),
        p( -50,   22),    p( -41,   22),    p( -79,   34),    p( -88,   43),    p( -54,   39),    p( -18,   32),    p( -58,   34),    p( -29,   21),
        p( -28,    7),    p( -96,   15),    p(-112,   28),    p(-134,   37),    p(-133,   37),    p(-108,   30),    p(-119,   22),    p(-101,   22),
        p( -41,   -3),    p(-106,    4),    p(-122,   20),    p(-148,   33),    p(-145,   31),    p(-120,   19),    p(-135,   12),    p(-117,   14),
        p( -34,    1),    p( -83,   -1),    p(-111,   12),    p(-120,   20),    p(-117,   21),    p(-126,   14),    p(-102,    3),    p( -73,   10),
        p(  25,   -8),    p( -70,   -5),    p( -81,    1),    p(-100,   10),    p(-106,   13),    p( -92,    5),    p( -66,  -10),    p(   1,   -2),
        p(  50,  -20),    p(  41,  -32),    p(  39,  -20),    p( -21,   -3),    p(  30,  -17),    p( -18,   -4),    p(  34,  -25),    p(  60,  -31),
    ],
];

const BISHOP_PAIR: PhasedScore = p(24, 51);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(9, 21), p(10, 18), p(10, 7), p(6, -0), p(2, -7), p(-2, -17), p(-8, -25), p(-15, -38), p(-26, -48)];
const ROOK_OPEN_FILE: PhasedScore = p(14, 2);
const ROOK_CLOSED_FILE: PhasedScore = p(-11, -2);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(5, -1);
const KING_OPEN_FILE: PhasedScore = p(-44, 0);
const KING_CLOSED_FILE: PhasedScore = p(13, -8);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 7);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 4), p(1, 6), p(-2, 5), p(2, 3), p(2, 5), p(4, 7), p(6, 5), p(19, 2)],
    // Closed
    [p(0, 0), p(0, 0), p(19, -25), p(-14, 9), p(-1, 11), p(-0, 5), p(-2, 7), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-12, 24), p(2, 17), p(-0, 10), p(-1, 10), p(4, 6), p(0, 2), p(10, 5)],
    // SemiClosed
    [p(0, 0), p(12, -12), p(7, 6), p(3, 0), p(6, 1), p(0, 4), p(3, 5), p(2, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(20, 7),
    p(2, 5),
    p(-2, 6),
    p(-9, 13),
    p(6, 1),
    p(-9, -8),
    p(-0, 2),
    p(-4, -6),
    p(0, -2),
    p(-10, 2),
    p(-11, -13),
    p(-17, 4),
    p(7, -5),
    p(-0, -3),
    p(8, -2),
    p(4, 19),
    p(-4, -3),
    p(-21, -0),
    p(-20, -0),
    p(-49, 24),
    p(-17, 5),
    p(-16, -15),
    p(7, 21),
    p(-59, 33),
    p(-15, -14),
    p(-19, -8),
    p(-39, -29),
    p(-39, 14),
    p(-19, 3),
    p(12, 1),
    p(-95, 114),
    p(0, 0),
    p(1, -2),
    p(-13, 0),
    p(-5, 1),
    p(-27, 11),
    p(-27, -6),
    p(-52, -21),
    p(-36, 38),
    p(-47, 29),
    p(-6, -0),
    p(-20, 0),
    p(5, -4),
    p(-20, 47),
    p(-53, 15),
    p(-10, -29),
    p(0, 0),
    p(0, 0),
    p(11, -10),
    p(-5, 18),
    p(-6, -49),
    p(0, 0),
    p(4, -7),
    p(-43, -0),
    p(0, 0),
    p(0, 0),
    p(-22, 6),
    p(-15, 5),
    p(-14, 26),
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
    p(-27, -4),
    p(-18, 2),
    p(-36, -6),
    p(5, -3),
    p(-13, -7),
    p(-27, -4),
    p(-42, 6),
    p(-9, -2),
    p(-42, 2),
    p(-36, -7),
    p(-52, 70),
    p(9, -5),
    p(-2, -7),
    p(-7, -13),
    p(-28, -7),
    p(-10, 1),
    p(-18, -8),
    p(-26, -3),
    p(-84, 183),
    p(-6, -11),
    p(-27, -11),
    p(-40, -27),
    p(4, -87),
    p(-17, -5),
    p(-18, -12),
    p(-81, 62),
    p(0, 0),
    p(15, -3),
    p(2, -3),
    p(-11, -5),
    p(-19, -7),
    p(-3, -0),
    p(-28, -12),
    p(-17, 0),
    p(-30, 2),
    p(0, -7),
    p(-20, -5),
    p(-25, -13),
    p(-37, -4),
    p(-13, -3),
    p(-48, -11),
    p(-16, 21),
    p(-66, 60),
    p(7, -1),
    p(-8, 1),
    p(-26, 58),
    p(0, 0),
    p(-18, 1),
    p(-22, 3),
    p(0, 0),
    p(0, 0),
    p(-13, 5),
    p(-38, 17),
    p(-35, -40),
    p(0, 0),
    p(2, -54),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-6, 8),    /*0b0000*/
    p(-14, 10),  /*0b0001*/
    p(-4, 10),   /*0b0010*/
    p(-9, 11),   /*0b0011*/
    p(-4, 5),    /*0b0100*/
    p(-27, 4),   /*0b0101*/
    p(-12, 3),   /*0b0110*/
    p(-19, -13), /*0b0111*/
    p(11, 8),    /*0b1000*/
    p(-1, 12),   /*0b1001*/
    p(3, 9),     /*0b1010*/
    p(-1, 9),    /*0b1011*/
    p(1, 6),     /*0b1100*/
    p(-26, 10),  /*0b1101*/
    p(-9, 4),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(8, 12),    /*0b10000*/
    p(5, 8),     /*0b10001*/
    p(20, 10),   /*0b10010*/
    p(-3, 6),    /*0b10011*/
    p(-3, 4),    /*0b10100*/
    p(12, 12),   /*0b10101*/
    p(-19, -0),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(17, 12),   /*0b11000*/
    p(27, 14),   /*0b11001*/
    p(38, 27),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(14, 1),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(19, 7),    /*0b100000*/
    p(5, 12),    /*0b100001*/
    p(24, 2),    /*0b100010*/
    p(8, -0),    /*0b100011*/
    p(-5, 3),    /*0b100100*/
    p(-23, -5),  /*0b100101*/
    p(-22, 16),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(29, 3),    /*0b101000*/
    p(1, 17),    /*0b101001*/
    p(21, -4),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-3, 9),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(17, 4),    /*0b110000*/
    p(22, 5),    /*0b110001*/
    p(30, -2),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(3, 17),    /*0b110100*/
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
    p(-11, -0),  /*0b00*/
    p(4, -12),   /*0b01*/
    p(35, -5),   /*0b10*/
    p(22, -41),  /*0b11*/
    p(45, -9),   /*0b100*/
    p(-6, -12),  /*0b101*/
    p(65, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(66, -11),  /*0b1000*/
    p(16, -28),  /*0b1001*/
    p(73, -52),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(61, -35),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(13, 1),    /*0b1111*/
    p(22, 1),    /*0b00*/
    p(33, -8),   /*0b01*/
    p(26, -15),  /*0b10*/
    p(23, -39),  /*0b11*/
    p(40, -6),   /*0b100*/
    p(54, -16),  /*0b101*/
    p(25, -20),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(41, -2),   /*0b1000*/
    p(51, -16),  /*0b1001*/
    p(53, -39),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(43, -25),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(22, -45),  /*0b1111*/
];
const STOPPABLE_PASSER: PhasedScore = p(37, -49);
const CLOSE_KING_PASSER: PhasedScore = p(-9, 30);
const IMMOBILE_PASSER: PhasedScore = p(-9, -35);
const PROTECTED_PASSER: PhasedScore = p(9, -3);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -41,   43),    p( -48,   56),    p( -70,   59),    p( -73,   52),    p( -61,   38),    p( -49,   40),    p( -32,   45),    p( -47,   48),
        p( -27,   42),    p( -52,   62),    p( -61,   58),    p( -64,   49),    p( -70,   49),    p( -55,   49),    p( -56,   65),    p( -39,   49),
        p( -19,   68),    p( -28,   69),    p( -56,   73),    p( -56,   70),    p( -64,   67),    p( -47,   69),    p( -54,   82),    p( -52,   80),
        p(   2,   90),    p(  -4,   92),    p(  -4,   81),    p( -29,   88),    p( -46,   88),    p( -30,   90),    p( -39,  100),    p( -43,  103),
        p(  19,  136),    p(  26,  135),    p(  17,  120),    p(  -2,  100),    p(   0,  107),    p( -16,  122),    p( -30,  122),    p( -62,  146),
        p(  32,   91),    p(  28,   90),    p(  20,   94),    p(  30,   77),    p(  16,   82),    p(  15,   85),    p( -26,  102),    p( -21,  101),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -8);
const DOUBLED_PAWN: PhasedScore = p(-7, -22);
const PHALANX: [PhasedScore; 6] = [p(-1, -1), p(5, 2), p(9, 5), p(24, 22), p(68, 75), p(-100, 221)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 12), p(7, 15), p(14, 19), p(9, 8), p(-3, 16), p(-47, 11)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(38, 8), p(40, 33), p(51, -13), p(35, -41), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-45, -73),
        p(-25, -32),
        p(-13, -8),
        p(-4, 5),
        p(3, 16),
        p(10, 26),
        p(18, 30),
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
        p(-31, -55),
        p(-19, -37),
        p(-8, -22),
        p(-0, -10),
        p(6, -0),
        p(12, 8),
        p(16, 13),
        p(21, 17),
        p(22, 23),
        p(29, 24),
        p(33, 24),
        p(40, 26),
        p(36, 35),
        p(49, 27),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-75, 12),
        p(-67, 26),
        p(-62, 32),
        p(-59, 37),
        p(-60, 44),
        p(-54, 50),
        p(-51, 54),
        p(-47, 58),
        p(-44, 63),
        p(-41, 67),
        p(-37, 70),
        p(-36, 76),
        p(-28, 77),
        p(-19, 75),
        p(-14, 74),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-28, -47),
        p(-27, 5),
        p(-32, 58),
        p(-28, 77),
        p(-25, 95),
        p(-21, 101),
        p(-17, 112),
        p(-14, 118),
        p(-10, 122),
        p(-7, 124),
        p(-4, 127),
        p(-0, 130),
        p(3, 131),
        p(4, 136),
        p(7, 138),
        p(10, 142),
        p(11, 150),
        p(13, 152),
        p(22, 150),
        p(36, 145),
        p(40, 148),
        p(83, 125),
        p(83, 128),
        p(104, 112),
        p(196, 80),
        p(242, 40),
        p(274, 28),
        p(335, -13),
    ],
    [
        p(-87, 5),
        p(-53, -8),
        p(-26, -8),
        p(1, -4),
        p(31, -3),
        p(52, -1),
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
    [p(-2, 10), p(20, 23), p(0, 0), p(31, 6), p(31, 54), p(0, 0)],
    [p(-3, 14), p(10, 14), p(18, 8), p(0, 0), p(44, -0), p(0, 0)],
    [p(-2, 5), p(2, 5), p(-0, 17), p(-0, 4), p(0, 0), p(0, 0)],
    [p(60, 19), p(-32, 18), p(5, 15), p(-22, 8), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 8), p(8, 7), p(6, 10), p(12, 7), p(6, 18), p(10, 6)],
    [p(2, 7), p(11, 21), p(-132, -18), p(8, 13), p(10, 20), p(3, 6)],
    [p(2, 3), p(13, 8), p(9, 12), p(11, 10), p(10, 26), p(21, -4)],
    [p(2, 1), p(9, 1), p(7, -4), p(4, 14), p(-66, -253), p(5, -9)],
    [p(61, -2), p(39, 9), p(44, 3), p(23, 6), p(34, -5), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-18, -16), p(19, -9), p(10, -3), p(15, -11), p(-1, 12), p(-3, 4)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(14, 19), p(33, 1), p(4, 33)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(4, -17), p(20, 14), p(13, 36), p(33, -2), p(22, 47)];
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(-5, -16), p(64, 16), p(146, -8), p(97, -53), p(0, 0), p(30, 0)];

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

    fn pin(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn discovered_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;
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

    fn pin(piece: ChessPieceType) -> SingleFeatureScore<Self::Score> {
        PIN[piece as usize]
    }

    fn discovered_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score> {
        DISCOVERED_CHECK[piece as usize]
    }
}
