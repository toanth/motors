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
        p( 131,  187),    p( 128,  185),    p( 120,  188),    p( 129,  171),    p( 116,  176),    p( 116,  180),    p(  75,  197),    p(  80,  196),
        p(  70,  122),    p(  71,  124),    p(  79,  117),    p(  87,  118),    p(  77,  117),    p( 126,  108),    p( 101,  128),    p(  96,  119),
        p(  53,  111),    p(  64,  105),    p(  62,  101),    p(  84,  100),    p(  92,   99),    p(  84,   90),    p(  77,  100),    p(  72,   95),
        p(  49,   98),    p(  54,  101),    p(  78,   93),    p(  93,   97),    p(  91,   98),    p(  87,   94),    p(  70,   91),    p(  60,   85),
        p(  42,   96),    p(  50,   92),    p(  72,   96),    p(  82,   98),    p(  83,   96),    p(  77,   95),    p(  69,   83),    p(  52,   85),
        p(  53,   99),    p(  58,   98),    p(  62,   98),    p(  59,  105),    p(  61,  107),    p(  76,   99),    p(  81,   87),    p(  59,   89),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 174,  279),    p( 196,  312),    p( 213,  324),    p( 251,  312),    p( 282,  314),    p( 200,  308),    p( 215,  309),    p( 201,  263),
        p( 267,  313),    p( 284,  317),    p( 300,  309),    p( 305,  313),    p( 303,  309),    p( 316,  298),    p( 275,  315),    p( 271,  305),
        p( 287,  309),    p( 307,  304),    p( 310,  310),    p( 323,  314),    p( 339,  308),    p( 353,  297),    p( 294,  304),    p( 287,  308),
        p( 302,  316),    p( 310,  310),    p( 326,  314),    p( 328,  321),    p( 326,  318),    p( 322,  317),    p( 311,  312),    p( 320,  311),
        p( 299,  318),    p( 305,  308),    p( 314,  314),    p( 321,  317),    p( 320,  319),    p( 325,  304),    p( 323,  304),    p( 312,  314),
        p( 275,  305),    p( 283,  303),    p( 297,  298),    p( 302,  311),    p( 306,  309),    p( 296,  292),    p( 302,  294),    p( 293,  308),
        p( 270,  313),    p( 281,  315),    p( 286,  304),    p( 295,  309),    p( 299,  304),    p( 289,  302),    p( 295,  307),    p( 289,  322),
        p( 239,  312),    p( 282,  305),    p( 267,  307),    p( 287,  312),    p( 296,  309),    p( 292,  298),    p( 288,  308),    p( 264,  312),
    ],
    // bishop
    [
        p( 276,  310),    p( 251,  314),    p( 239,  307),    p( 223,  317),    p( 217,  314),    p( 225,  308),    p( 273,  303),    p( 250,  309),
        p( 283,  302),    p( 279,  303),    p( 290,  306),    p( 277,  304),    p( 289,  301),    p( 292,  299),    p( 268,  308),    p( 271,  302),
        p( 296,  309),    p( 308,  304),    p( 292,  304),    p( 307,  299),    p( 306,  300),    p( 336,  304),    p( 317,  302),    p( 318,  313),
        p( 287,  312),    p( 292,  307),    p( 304,  302),    p( 307,  306),    p( 306,  304),    p( 299,  305),    p( 298,  309),    p( 280,  311),
        p( 289,  308),    p( 284,  309),    p( 295,  304),    p( 308,  305),    p( 302,  301),    p( 299,  303),    p( 285,  306),    p( 309,  302),
        p( 296,  310),    p( 299,  305),    p( 299,  307),    p( 299,  305),    p( 305,  307),    p( 298,  300),    p( 305,  296),    p( 307,  300),
        p( 308,  309),    p( 303,  300),    p( 308,  302),    p( 299,  309),    p( 301,  306),    p( 302,  305),    p( 311,  296),    p( 308,  296),
        p( 298,  305),    p( 309,  307),    p( 308,  307),    p( 290,  310),    p( 306,  308),    p( 294,  310),    p( 302,  298),    p( 301,  293),
    ],
    // rook
    [
        p( 460,  548),    p( 447,  558),    p( 441,  565),    p( 439,  562),    p( 451,  557),    p( 471,  553),    p( 480,  551),    p( 492,  544),
        p( 443,  554),    p( 441,  559),    p( 451,  560),    p( 465,  551),    p( 451,  554),    p( 467,  548),    p( 476,  545),    p( 491,  535),
        p( 445,  549),    p( 465,  544),    p( 458,  546),    p( 458,  541),    p( 484,  531),    p( 494,  528),    p( 512,  526),    p( 486,  529),
        p( 442,  550),    p( 448,  545),    p( 448,  548),    p( 453,  542),    p( 457,  534),    p( 468,  530),    p( 468,  533),    p( 468,  529),
        p( 435,  547),    p( 434,  546),    p( 435,  546),    p( 440,  542),    p( 447,  538),    p( 441,  537),    p( 454,  531),    p( 449,  529),
        p( 430,  545),    p( 430,  541),    p( 432,  540),    p( 435,  540),    p( 440,  534),    p( 451,  526),    p( 467,  515),    p( 454,  519),
        p( 432,  540),    p( 436,  539),    p( 442,  539),    p( 445,  536),    p( 452,  530),    p( 464,  520),    p( 472,  515),    p( 443,  526),
        p( 442,  544),    p( 438,  539),    p( 440,  542),    p( 444,  537),    p( 450,  530),    p( 456,  530),    p( 452,  529),    p( 448,  533),
    ],
    // queen
    [
        p( 878,  962),    p( 879,  977),    p( 895,  989),    p( 916,  984),    p( 913,  988),    p( 933,  976),    p( 979,  928),    p( 925,  959),
        p( 890,  949),    p( 864,  981),    p( 865, 1009),    p( 858, 1026),    p( 864, 1037),    p( 904,  999),    p( 906,  983),    p( 948,  961),
        p( 895,  955),    p( 887,  972),    p( 886,  994),    p( 885, 1005),    p( 908, 1007),    p( 946,  991),    p( 954,  962),    p( 942,  968),
        p( 881,  968),    p( 886,  977),    p( 879,  988),    p( 880, 1000),    p( 881, 1014),    p( 896, 1003),    p( 905, 1004),    p( 912,  979),
        p( 890,  960),    p( 877,  981),    p( 883,  981),    p( 883,  998),    p( 887,  994),    p( 888,  995),    p( 901,  984),    p( 908,  976),
        p( 886,  950),    p( 892,  966),    p( 886,  981),    p( 883,  984),    p( 888,  992),    p( 895,  980),    p( 909,  962),    p( 908,  951),
        p( 886,  953),    p( 886,  960),    p( 893,  963),    p( 892,  976),    p( 894,  975),    p( 895,  959),    p( 907,  938),    p( 914,  912),
        p( 872,  954),    p( 884,  941),    p( 885,  953),    p( 893,  955),    p( 896,  944),    p( 883,  948),    p( 884,  942),    p( 889,  926),
    ],
    // king
    [
        p( 159,  -72),    p(  65,  -23),    p(  84,  -13),    p(  19,   14),    p(  45,    2),    p(  24,   14),    p(  84,    3),    p( 221,  -75),
        p( -27,   15),    p( -64,   33),    p( -65,   40),    p(   1,   30),    p( -35,   39),    p( -56,   52),    p( -32,   40),    p(   9,   16),
        p( -49,   20),    p( -42,   23),    p( -80,   35),    p( -90,   45),    p( -55,   41),    p( -24,   32),    p( -63,   35),    p( -31,   19),
        p( -30,    6),    p( -97,   16),    p(-114,   30),    p(-135,   39),    p(-134,   38),    p(-112,   30),    p(-125,   22),    p(-102,   20),
        p( -43,   -3),    p(-108,    6),    p(-123,   21),    p(-148,   35),    p(-146,   32),    p(-121,   20),    p(-134,   13),    p(-117,   14),
        p( -33,    1),    p( -82,    1),    p(-111,   14),    p(-119,   23),    p(-116,   23),    p(-125,   16),    p(-101,    6),    p( -72,   11),
        p(  27,   -7),    p( -70,   -3),    p( -81,    3),    p(-100,   13),    p(-106,   15),    p( -92,    7),    p( -66,   -7),    p(   2,   -2),
        p(  50,  -23),    p(  42,  -33),    p(  39,  -22),    p( -20,   -5),    p(  30,  -20),    p( -17,   -6),    p(  34,  -27),    p(  61,  -33),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 52);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(9, 21), p(10, 18), p(10, 7), p(6, -0), p(2, -8), p(-1, -18), p(-7, -26), p(-14, -39), p(-25, -50)];
const ROOK_OPEN_FILE: PhasedScore = p(14, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 4);
const KING_OPEN_FILE: PhasedScore = p(-49, -0);
const KING_CLOSED_FILE: PhasedScore = p(13, -9);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 10);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-5, 4), p(0, 6), p(-1, 4), p(2, 3), p(2, 5), p(4, 7), p(7, 5), p(19, 2)],
    // Closed
    [p(0, 0), p(0, 0), p(21, -31), p(-15, 11), p(-1, 11), p(-0, 5), p(-2, 8), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-15, 24), p(2, 17), p(-0, 10), p(-0, 9), p(4, 6), p(0, 2), p(10, 5)],
    // SemiClosed
    [p(0, 0), p(12, -13), p(7, 6), p(3, 0), p(6, 1), p(1, 4), p(3, 5), p(2, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(23, 5),
    p(2, 4),
    p(2, 3),
    p(-7, 12),
    p(7, 0),
    p(-10, -7),
    p(1, 3),
    p(-4, -5),
    p(1, -3),
    p(-11, 2),
    p(-9, -14),
    p(-17, 4),
    p(6, -4),
    p(-3, -0),
    p(7, 0),
    p(1, 25),
    p(-3, -3),
    p(-22, 1),
    p(-17, -2),
    p(-48, 23),
    p(-17, 5),
    p(-18, -15),
    p(8, 21),
    p(-58, 30),
    p(-16, -14),
    p(-21, -7),
    p(-39, -30),
    p(-41, 13),
    p(-21, 5),
    p(8, 4),
    p(-96, 117),
    p(0, 0),
    p(1, -2),
    p(-15, 1),
    p(-4, -1),
    p(-27, 10),
    p(-28, -6),
    p(-54, -18),
    p(-35, 38),
    p(-49, 34),
    p(-8, 1),
    p(-23, 2),
    p(5, -5),
    p(-22, 48),
    p(-56, 17),
    p(-14, -26),
    p(0, 0),
    p(0, 0),
    p(9, -3),
    p(-8, 24),
    p(-5, -47),
    p(0, 0),
    p(3, -4),
    p(-43, -6),
    p(0, 0),
    p(0, 0),
    p(-24, 11),
    p(-19, 10),
    p(-13, 22),
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
    p(-21, 1),
    p(5, -3),
    p(-28, -3),
    p(-19, 4),
    p(-38, -4),
    p(4, -3),
    p(-15, -6),
    p(-28, -3),
    p(-43, 7),
    p(-10, -0),
    p(-44, 4),
    p(-38, -3),
    p(-55, 73),
    p(10, -4),
    p(-3, -5),
    p(-7, -11),
    p(-29, -4),
    p(-10, 3),
    p(-19, -6),
    p(-26, -1),
    p(-84, 179),
    p(-7, -10),
    p(-29, -8),
    p(-41, -25),
    p(2, -82),
    p(-18, -3),
    p(-20, -8),
    p(-82, 67),
    p(0, 0),
    p(16, -2),
    p(2, -2),
    p(-11, -5),
    p(-20, -5),
    p(-2, 1),
    p(-29, -10),
    p(-17, 2),
    p(-30, 5),
    p(0, -6),
    p(-22, -3),
    p(-25, -12),
    p(-38, -1),
    p(-12, -1),
    p(-50, -8),
    p(-17, 25),
    p(-66, 60),
    p(8, 1),
    p(-8, 1),
    p(-25, 58),
    p(0, 0),
    p(-16, 2),
    p(-22, 3),
    p(0, 0),
    p(0, 0),
    p(-12, 6),
    p(-39, 18),
    p(-34, -41),
    p(0, 0),
    p(6, -56),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-6, 8),    /*0b0000*/
    p(-14, 9),   /*0b0001*/
    p(-4, 10),   /*0b0010*/
    p(-8, 12),   /*0b0011*/
    p(-3, 4),    /*0b0100*/
    p(-26, 3),   /*0b0101*/
    p(-11, 4),   /*0b0110*/
    p(-18, -13), /*0b0111*/
    p(11, 8),    /*0b1000*/
    p(-1, 11),   /*0b1001*/
    p(3, 10),    /*0b1010*/
    p(-0, 10),   /*0b1011*/
    p(1, 5),     /*0b1100*/
    p(-24, 8),   /*0b1101*/
    p(-9, 4),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(8, 12),    /*0b10000*/
    p(5, 8),     /*0b10001*/
    p(20, 10),   /*0b10010*/
    p(-3, 7),    /*0b10011*/
    p(-3, 4),    /*0b10100*/
    p(13, 13),   /*0b10101*/
    p(-19, 0),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(17, 12),   /*0b11000*/
    p(27, 14),   /*0b11001*/
    p(37, 27),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(14, 1),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(19, 7),    /*0b100000*/
    p(6, 12),    /*0b100001*/
    p(24, 3),    /*0b100010*/
    p(8, 0),     /*0b100011*/
    p(-5, 3),    /*0b100100*/
    p(-22, -6),  /*0b100101*/
    p(-22, 16),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(29, 2),    /*0b101000*/
    p(3, 16),    /*0b101001*/
    p(20, -4),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-1, 8),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(17, 6),    /*0b110000*/
    p(22, 6),    /*0b110001*/
    p(29, -1),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(3, 18),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(28, -0),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(7, -3),    /*0b111111*/
    p(-12, 0),   /*0b00*/
    p(4, -13),   /*0b01*/
    p(37, -5),   /*0b10*/
    p(23, -41),  /*0b11*/
    p(44, -10),  /*0b100*/
    p(-1, -15),  /*0b101*/
    p(66, -41),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(66, -12),  /*0b1000*/
    p(16, -31),  /*0b1001*/
    p(74, -51),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(60, -37),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(18, -4),   /*0b1111*/
    p(21, 1),    /*0b00*/
    p(33, -9),   /*0b01*/
    p(26, -15),  /*0b10*/
    p(23, -39),  /*0b11*/
    p(39, -8),   /*0b100*/
    p(54, -19),  /*0b101*/
    p(24, -22),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(41, -3),   /*0b1000*/
    p(52, -17),  /*0b1001*/
    p(52, -40),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(43, -27),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(21, -45),  /*0b1111*/
];
const STOPPABLE_PASSER: PhasedScore = p(37, -51);
const CLOSE_KING_PASSER: PhasedScore = p(-7, 30);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -22,   35),    p( -36,   51),    p( -44,   50),    p( -38,   37),    p( -26,   26),    p( -28,   33),    p( -23,   42),    p( -27,   42),
        p( -12,   35),    p( -42,   58),    p( -44,   51),    p( -42,   40),    p( -44,   40),    p( -39,   43),    p( -48,   62),    p( -22,   44),
        p(  -4,   62),    p( -15,   63),    p( -40,   64),    p( -35,   59),    p( -43,   57),    p( -32,   62),    p( -43,   76),    p( -38,   74),
        p(  13,   83),    p(   7,   85),    p(  11,   71),    p( -11,   76),    p( -30,   77),    p( -17,   81),    p( -30,   94),    p( -32,   97),
        p(  29,  128),    p(  33,  127),    p(  26,  109),    p(   8,   87),    p(   9,   95),    p(  -8,  113),    p( -27,  115),    p( -55,  138),
        p(  31,   87),    p(  28,   85),    p(  20,   88),    p(  29,   71),    p(  16,   76),    p(  16,   80),    p( -25,   97),    p( -20,   96),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -9);
const DOUBLED_PAWN: PhasedScore = p(-7, -23);
const PHALANX: [PhasedScore; 6] = [p(-0, -2), p(5, 1), p(9, 5), p(24, 24), p(66, 81), p(-90, 223)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 12), p(7, 14), p(14, 18), p(9, 7), p(-3, 15), p(-46, 8)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(39, 8), p(39, 34), p(52, -10), p(35, -35), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-44, -73),
        p(-25, -32),
        p(-12, -8),
        p(-3, 5),
        p(4, 16),
        p(11, 27),
        p(19, 30),
        p(25, 32),
        p(30, 31),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-19, -38),
        p(-8, -22),
        p(-0, -10),
        p(7, -0),
        p(12, 8),
        p(17, 13),
        p(21, 18),
        p(23, 23),
        p(30, 24),
        p(35, 24),
        p(42, 27),
        p(38, 35),
        p(53, 27),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-67, 28),
        p(-62, 34),
        p(-59, 39),
        p(-60, 45),
        p(-54, 50),
        p(-51, 54),
        p(-47, 57),
        p(-43, 61),
        p(-39, 64),
        p(-35, 67),
        p(-34, 72),
        p(-26, 73),
        p(-17, 70),
        p(-15, 69),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-28, -40),
        p(-28, 13),
        p(-32, 64),
        p(-28, 82),
        p(-26, 101),
        p(-21, 106),
        p(-18, 117),
        p(-14, 122),
        p(-10, 126),
        p(-7, 127),
        p(-4, 130),
        p(1, 132),
        p(4, 132),
        p(5, 136),
        p(8, 138),
        p(11, 141),
        p(12, 148),
        p(14, 149),
        p(24, 147),
        p(37, 141),
        p(41, 143),
        p(84, 120),
        p(84, 123),
        p(104, 106),
        p(196, 73),
        p(241, 32),
        p(262, 26),
        p(326, -17),
    ],
    [
        p(-87, 10),
        p(-54, -4),
        p(-27, -5),
        p(1, -3),
        p(30, -3),
        p(52, -2),
        p(79, 2),
        p(104, 0),
        p(150, -14),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-9, 7), p(0, 0), p(23, 19), p(49, -11), p(20, -33), p(0, 0)],
    [p(-2, 10), p(20, 23), p(0, 0), p(31, 5), p(30, 53), p(0, 0)],
    [p(-3, 13), p(10, 15), p(17, 12), p(0, 0), p(45, -5), p(0, 0)],
    [p(-2, 4), p(2, 6), p(-1, 22), p(1, 2), p(0, 0), p(0, 0)],
    [p(69, 24), p(-35, 18), p(-9, 18), p(-21, 7), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 7), p(8, 7), p(6, 10), p(12, 7), p(6, 19), p(10, 6)],
    [p(2, 7), p(11, 22), p(-140, -15), p(8, 14), p(9, 20), p(3, 8)],
    [p(3, 1), p(14, 6), p(10, 11), p(12, 8), p(11, 21), p(22, -5)],
    [p(2, -2), p(9, 1), p(7, -4), p(4, 15), p(-61, -256), p(5, -11)],
    [p(61, -3), p(39, 6), p(44, -0), p(22, 4), p(34, -12), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-20, -14), p(19, -9), p(10, -3), p(15, -11), p(-1, 12), p(-8, 7)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(14, 19), p(34, 0), p(5, 32)];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static + StaticallyNamedEntity {
    type Score: ScoreType;

    fn psqt(&self, square: ChessSquare, piece: ChessPieceType, color: ChessColor) -> SingleFeatureScore<Self::Score>;

    fn passed_pawn(square: ChessSquare) -> SingleFeatureScore<Self::Score>;

    fn stoppable_passer() -> SingleFeatureScore<Self::Score>;

    fn close_king_passer() -> SingleFeatureScore<Self::Score>;

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
