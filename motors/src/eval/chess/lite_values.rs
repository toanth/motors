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
        p( 132,  186),    p( 127,  184),    p( 120,  187),    p( 129,  170),    p( 116,  175),    p( 116,  179),    p(  74,  195),    p(  81,  196),
        p(  70,  117),    p(  68,  118),    p(  79,  114),    p(  87,  115),    p(  77,  114),    p( 126,  104),    p( 100,  123),    p(  97,  115),
        p(  52,  107),    p(  60,   99),    p(  62,   99),    p(  85,   98),    p(  93,   97),    p(  85,   88),    p(  74,   94),    p(  73,   92),
        p(  48,   94),    p(  50,   94),    p(  78,   92),    p(  91,   94),    p(  90,   96),    p(  87,   93),    p(  67,   85),    p(  60,   82),
        p(  41,   91),    p(  47,   87),    p(  73,   94),    p(  81,   96),    p(  83,   95),    p(  78,   93),    p(  67,   78),    p(  52,   80),
        p(  52,   94),    p(  55,   92),    p(  61,   95),    p(  58,  101),    p(  61,  104),    p(  75,   95),    p(  79,   82),    p(  58,   84),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 174,  279),    p( 195,  312),    p( 213,  324),    p( 251,  313),    p( 282,  314),    p( 199,  309),    p( 214,  309),    p( 201,  263),
        p( 267,  313),    p( 283,  317),    p( 300,  309),    p( 305,  313),    p( 303,  309),    p( 315,  298),    p( 274,  314),    p( 271,  305),
        p( 287,  309),    p( 307,  304),    p( 309,  310),    p( 323,  314),    p( 339,  308),    p( 353,  297),    p( 294,  304),    p( 287,  308),
        p( 303,  316),    p( 310,  309),    p( 327,  314),    p( 328,  321),    p( 326,  318),    p( 322,  317),    p( 312,  313),    p( 320,  311),
        p( 299,  318),    p( 305,  308),    p( 314,  314),    p( 321,  317),    p( 320,  319),    p( 326,  304),    p( 323,  304),    p( 312,  314),
        p( 276,  305),    p( 283,  303),    p( 297,  298),    p( 302,  311),    p( 306,  309),    p( 296,  292),    p( 302,  294),    p( 293,  309),
        p( 270,  313),    p( 281,  315),    p( 286,  304),    p( 295,  309),    p( 299,  304),    p( 290,  302),    p( 295,  307),    p( 289,  322),
        p( 240,  311),    p( 282,  305),    p( 267,  307),    p( 288,  311),    p( 296,  309),    p( 292,  298),    p( 288,  308),    p( 264,  312),
    ],
    // bishop
    [
        p( 276,  310),    p( 251,  314),    p( 239,  307),    p( 223,  317),    p( 216,  315),    p( 224,  308),    p( 272,  304),    p( 250,  309),
        p( 283,  302),    p( 278,  304),    p( 290,  306),    p( 277,  305),    p( 289,  301),    p( 292,  300),    p( 266,  308),    p( 271,  302),
        p( 296,  309),    p( 308,  305),    p( 291,  305),    p( 307,  299),    p( 306,  300),    p( 336,  305),    p( 317,  302),    p( 318,  313),
        p( 287,  312),    p( 292,  307),    p( 304,  302),    p( 307,  306),    p( 306,  304),    p( 299,  305),    p( 298,  309),    p( 280,  311),
        p( 289,  309),    p( 284,  309),    p( 296,  304),    p( 309,  305),    p( 302,  301),    p( 299,  303),    p( 285,  306),    p( 309,  302),
        p( 296,  310),    p( 299,  305),    p( 299,  307),    p( 299,  305),    p( 305,  307),    p( 298,  300),    p( 305,  296),    p( 307,  300),
        p( 308,  309),    p( 304,  300),    p( 308,  302),    p( 299,  309),    p( 301,  307),    p( 302,  305),    p( 312,  296),    p( 308,  296),
        p( 298,  305),    p( 309,  307),    p( 308,  307),    p( 290,  310),    p( 306,  308),    p( 294,  310),    p( 302,  298),    p( 302,  293),
    ],
    // rook
    [
        p( 460,  548),    p( 448,  558),    p( 441,  565),    p( 439,  562),    p( 451,  558),    p( 471,  553),    p( 480,  552),    p( 492,  545),
        p( 443,  554),    p( 441,  560),    p( 451,  560),    p( 465,  552),    p( 451,  554),    p( 467,  549),    p( 475,  546),    p( 491,  535),
        p( 446,  549),    p( 465,  544),    p( 458,  546),    p( 458,  541),    p( 484,  531),    p( 494,  528),    p( 512,  526),    p( 486,  529),
        p( 443,  550),    p( 448,  545),    p( 448,  548),    p( 453,  542),    p( 457,  534),    p( 468,  530),    p( 468,  533),    p( 468,  529),
        p( 435,  548),    p( 434,  546),    p( 435,  546),    p( 440,  542),    p( 448,  538),    p( 441,  537),    p( 454,  531),    p( 449,  530),
        p( 430,  545),    p( 430,  541),    p( 432,  540),    p( 435,  540),    p( 440,  535),    p( 451,  526),    p( 467,  515),    p( 454,  519),
        p( 432,  540),    p( 436,  539),    p( 442,  539),    p( 445,  537),    p( 452,  530),    p( 464,  520),    p( 472,  516),    p( 443,  525),
        p( 442,  544),    p( 439,  539),    p( 440,  543),    p( 444,  537),    p( 450,  530),    p( 456,  530),    p( 453,  530),    p( 448,  533),
    ],
    // queen
    [
        p( 878,  963),    p( 880,  977),    p( 895,  990),    p( 916,  984),    p( 914,  988),    p( 934,  977),    p( 979,  929),    p( 925,  960),
        p( 890,  950),    p( 863,  982),    p( 865, 1010),    p( 858, 1027),    p( 865, 1039),    p( 905, 1000),    p( 905,  984),    p( 949,  961),
        p( 895,  956),    p( 888,  972),    p( 886,  994),    p( 886, 1006),    p( 908, 1008),    p( 946,  992),    p( 955,  963),    p( 942,  969),
        p( 882,  968),    p( 886,  977),    p( 880,  989),    p( 880, 1001),    p( 882, 1014),    p( 896, 1004),    p( 906, 1004),    p( 913,  980),
        p( 891,  961),    p( 878,  981),    p( 884,  982),    p( 884,  999),    p( 888,  995),    p( 889,  996),    p( 902,  984),    p( 909,  976),
        p( 887,  950),    p( 893,  967),    p( 886,  982),    p( 883,  984),    p( 889,  992),    p( 896,  981),    p( 910,  963),    p( 908,  951),
        p( 887,  954),    p( 887,  961),    p( 893,  963),    p( 893,  977),    p( 895,  975),    p( 895,  960),    p( 907,  939),    p( 915,  912),
        p( 872,  955),    p( 884,  942),    p( 886,  954),    p( 894,  955),    p( 896,  945),    p( 883,  949),    p( 885,  942),    p( 889,  926),
    ],
    // king
    [
        p( 161,  -72),    p(  67,  -23),    p(  85,  -14),    p(  20,   14),    p(  47,    1),    p(  26,   14),    p(  86,    3),    p( 222,  -74),
        p( -27,   16),    p( -64,   34),    p( -65,   40),    p(   0,   30),    p( -35,   39),    p( -56,   53),    p( -32,   40),    p(   9,   17),
        p( -49,   20),    p( -42,   24),    p( -80,   35),    p( -90,   46),    p( -55,   41),    p( -24,   32),    p( -63,   35),    p( -31,   20),
        p( -30,    6),    p( -97,   17),    p(-114,   30),    p(-135,   39),    p(-134,   38),    p(-112,   31),    p(-125,   22),    p(-101,   21),
        p( -43,   -3),    p(-107,    7),    p(-123,   22),    p(-148,   35),    p(-146,   33),    p(-121,   21),    p(-134,   13),    p(-117,   15),
        p( -33,    1),    p( -82,    1),    p(-111,   14),    p(-120,   23),    p(-117,   23),    p(-125,   16),    p(-102,    6),    p( -72,   11),
        p(  26,   -7),    p( -71,   -3),    p( -81,    3),    p(-101,   13),    p(-107,   15),    p( -93,    7),    p( -66,   -7),    p(   1,   -1),
        p(  50,  -23),    p(  42,  -34),    p(  40,  -23),    p( -20,   -6),    p(  30,  -21),    p( -17,   -7),    p(  34,  -27),    p(  61,  -33),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 52);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(9, 21), p(11, 19), p(11, 7), p(6, -1), p(2, -8), p(-1, -17), p(-7, -26), p(-14, -37), p(-25, -46)];
const ROOK_OPEN_FILE: PhasedScore = p(14, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 4);
const KING_OPEN_FILE: PhasedScore = p(-48, 1);
const KING_CLOSED_FILE: PhasedScore = p(13, -9);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 11);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-5, 4), p(1, 6), p(-1, 4), p(2, 3), p(2, 5), p(4, 7), p(7, 5), p(19, 2)],
    // Closed
    [p(0, 0), p(0, 0), p(20, -31), p(-15, 11), p(-1, 11), p(0, 5), p(-2, 8), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-14, 23), p(3, 17), p(0, 10), p(-0, 9), p(4, 6), p(1, 2), p(10, 5)],
    // SemiClosed
    [p(0, 0), p(12, -13), p(7, 5), p(3, 0), p(6, 1), p(1, 5), p(3, 5), p(2, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(24, 6),
    p(2, 4),
    p(3, 4),
    p(-7, 9),
    p(8, 1),
    p(-9, -8),
    p(2, 2),
    p(-4, -8),
    p(0, -2),
    p(-11, 1),
    p(-8, -14),
    p(-18, 2),
    p(6, -7),
    p(-3, -3),
    p(8, -2),
    p(2, 20),
    p(-2, -2),
    p(-23, -1),
    p(-17, -2),
    p(-51, 21),
    p(-17, 3),
    p(-18, -23),
    p(7, 20),
    p(-58, 25),
    p(-16, -13),
    p(-22, -8),
    p(-39, -30),
    p(-44, 14),
    p(-21, 4),
    p(7, -3),
    p(-97, 113),
    p(0, 0),
    p(2, -1),
    p(-14, 1),
    p(-4, -3),
    p(-27, 10),
    p(-27, -7),
    p(-53, -19),
    p(-36, 38),
    p(-49, 26),
    p(-8, 1),
    p(-23, 1),
    p(4, -11),
    p(-22, 45),
    p(-57, 15),
    p(-15, -27),
    p(0, 0),
    p(0, 0),
    p(8, -3),
    p(-9, 19),
    p(-7, -53),
    p(0, 0),
    p(0, -12),
    p(-45, -23),
    p(0, 0),
    p(0, 0),
    p(-25, 9),
    p(-20, 1),
    p(-16, 11),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(21, 2),
    p(-2, -2),
    p(-4, 2),
    p(-22, -2),
    p(5, -3),
    p(-29, -4),
    p(-17, 1),
    p(-38, -8),
    p(4, -3),
    p(-17, -7),
    p(-28, -5),
    p(-45, 5),
    p(-11, -4),
    p(-46, 1),
    p(-39, -7),
    p(-56, 67),
    p(11, -2),
    p(-4, -4),
    p(-6, -12),
    p(-30, -6),
    p(-9, 3),
    p(-20, -11),
    p(-26, -4),
    p(-86, 178),
    p(-6, -10),
    p(-30, -9),
    p(-40, -27),
    p(-1, -83),
    p(-18, -3),
    p(-22, -14),
    p(-83, 61),
    p(0, 0),
    p(17, -1),
    p(1, -1),
    p(-10, -7),
    p(-21, -6),
    p(-1, -0),
    p(-29, -13),
    p(-17, -0),
    p(-32, -0),
    p(0, -6),
    p(-24, -4),
    p(-26, -19),
    p(-40, -9),
    p(-14, -1),
    p(-51, -11),
    p(-18, 21),
    p(-67, 52),
    p(10, 2),
    p(-8, -1),
    p(-26, 52),
    p(0, 0),
    p(-17, -4),
    p(-24, -11),
    p(0, 0),
    p(0, 0),
    p(-12, 3),
    p(-40, 10),
    p(-36, -53),
    p(0, 0),
    p(7, -57),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-7, 8),    /*0b0000*/
    p(-14, 9),   /*0b0001*/
    p(-4, 10),   /*0b0010*/
    p(-8, 12),   /*0b0011*/
    p(-4, 5),    /*0b0100*/
    p(-26, 3),   /*0b0101*/
    p(-12, 4),   /*0b0110*/
    p(-18, -14), /*0b0111*/
    p(10, 8),    /*0b1000*/
    p(-3, 11),   /*0b1001*/
    p(2, 10),    /*0b1010*/
    p(-0, 12),   /*0b1011*/
    p(1, 6),     /*0b1100*/
    p(-24, 9),   /*0b1101*/
    p(-9, 7),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(7, 13),    /*0b10000*/
    p(5, 9),     /*0b10001*/
    p(21, 11),   /*0b10010*/
    p(-2, 10),   /*0b10011*/
    p(-3, 5),    /*0b10100*/
    p(12, 9),    /*0b10101*/
    p(-19, 3),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(16, 14),   /*0b11000*/
    p(25, 9),    /*0b11001*/
    p(36, 23),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(14, -1),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(18, 8),    /*0b100000*/
    p(5, 11),    /*0b100001*/
    p(23, 3),    /*0b100010*/
    p(9, 4),     /*0b100011*/
    p(-6, 3),    /*0b100100*/
    p(-21, -4),  /*0b100101*/
    p(-22, 20),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(27, 2),    /*0b101000*/
    p(1, 15),    /*0b101001*/
    p(20, -2),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-3, 9),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(16, 7),    /*0b110000*/
    p(22, 4),    /*0b110001*/
    p(28, -5),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(2, 14),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(28, 1),    /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(5, -4),    /*0b111111*/
    p(-12, 0),   /*0b00*/
    p(5, -12),   /*0b01*/
    p(37, -5),   /*0b10*/
    p(24, -41),  /*0b11*/
    p(44, -9),   /*0b100*/
    p(2, -11),   /*0b101*/
    p(67, -41),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(67, -12),  /*0b1000*/
    p(17, -32),  /*0b1001*/
    p(75, -48),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(61, -36),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(21, -3),   /*0b1111*/
    p(20, 0),    /*0b00*/
    p(33, -9),   /*0b01*/
    p(27, -15),  /*0b10*/
    p(24, -40),  /*0b11*/
    p(40, -7),   /*0b100*/
    p(55, -17),  /*0b101*/
    p(25, -23),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(41, -3),   /*0b1000*/
    p(53, -17),  /*0b1001*/
    p(55, -38),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(43, -27),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(24, -44),  /*0b1111*/
];
const STOPPABLE_PASSER: PhasedScore = p(37, -51);
const CLOSE_KING_PASSER: PhasedScore = p(-7, 30);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -19,   39),    p( -34,   53),    p( -44,   51),    p( -38,   39),    p( -26,   27),    p( -28,   34),    p( -22,   43),    p( -25,   44),
        p(  -9,   38),    p( -40,   60),    p( -44,   51),    p( -42,   40),    p( -44,   40),    p( -39,   44),    p( -46,   64),    p( -20,   46),
        p(  -2,   65),    p( -12,   67),    p( -39,   65),    p( -35,   59),    p( -43,   58),    p( -32,   62),    p( -41,   80),    p( -36,   76),
        p(  16,   86),    p(  10,   89),    p(  11,   71),    p( -11,   76),    p( -30,   77),    p( -18,   81),    p( -27,   98),    p( -30,   99),
        p(  31,  131),    p(  36,  130),    p(  26,  110),    p(   8,   88),    p(   9,   96),    p(  -8,  114),    p( -25,  118),    p( -54,  140),
        p(  32,   86),    p(  27,   84),    p(  20,   87),    p(  29,   70),    p(  16,   75),    p(  16,   79),    p( -26,   95),    p( -19,   96),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const BACKWARDS_PAWN: PhasedScore = p(-6, 1);
const ISOLATED_PAWN: PhasedScore = p(-9, -8);
const DOUBLED_PAWN: PhasedScore = p(-6, -21);
const PHALANX: [PhasedScore; 6] = [p(1, 3), p(5, 4), p(9, 8), p(24, 25), p(65, 83), p(-92, 225)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(16, 11), p(7, 14), p(14, 18), p(10, 7), p(-3, 15), p(-45, 7)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(39, 8), p(40, 33), p(52, -10), p(35, -35), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-44, -72),
        p(-25, -31),
        p(-12, -8),
        p(-3, 5),
        p(4, 16),
        p(11, 27),
        p(19, 30),
        p(26, 33),
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
        p(-19, -37),
        p(-7, -22),
        p(-0, -9),
        p(7, 0),
        p(12, 9),
        p(17, 14),
        p(21, 18),
        p(23, 23),
        p(30, 24),
        p(35, 24),
        p(42, 27),
        p(39, 35),
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
        p(-75, 15),
        p(-67, 29),
        p(-62, 35),
        p(-59, 39),
        p(-60, 46),
        p(-54, 51),
        p(-51, 55),
        p(-46, 58),
        p(-42, 61),
        p(-39, 65),
        p(-35, 67),
        p(-33, 72),
        p(-25, 73),
        p(-17, 71),
        p(-13, 70),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-28, -39),
        p(-27, 14),
        p(-31, 65),
        p(-27, 83),
        p(-25, 102),
        p(-20, 107),
        p(-17, 118),
        p(-13, 123),
        p(-9, 127),
        p(-6, 128),
        p(-3, 131),
        p(1, 133),
        p(4, 133),
        p(6, 137),
        p(9, 139),
        p(12, 142),
        p(13, 149),
        p(15, 150),
        p(25, 148),
        p(38, 142),
        p(42, 144),
        p(85, 121),
        p(85, 124),
        p(105, 107),
        p(198, 73),
        p(242, 34),
        p(259, 28),
        p(325, -15),
    ],
    [
        p(-88, 10),
        p(-54, -3),
        p(-27, -4),
        p(1, -3),
        p(31, -3),
        p(53, -3),
        p(81, 1),
        p(105, -0),
        p(152, -15),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-2, 10), p(20, 23), p(0, 0), p(31, 4), p(30, 53), p(0, 0)],
    [p(-3, 13), p(10, 15), p(17, 12), p(0, 0), p(45, -5), p(0, 0)],
    [p(-2, 4), p(2, 6), p(-1, 22), p(1, 2), p(0, 0), p(0, 0)],
    [p(69, 24), p(-35, 18), p(-9, 17), p(-21, 7), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 7), p(8, 7), p(6, 10), p(12, 7), p(6, 19), p(11, 6)],
    [p(2, 7), p(11, 22), p(-141, -14), p(8, 14), p(9, 20), p(4, 8)],
    [p(3, 1), p(14, 6), p(10, 11), p(12, 8), p(11, 21), p(22, -5)],
    [p(2, -2), p(9, 1), p(7, -4), p(4, 15), p(-62, -256), p(5, -11)],
    [p(61, -3), p(39, 6), p(45, -0), p(23, 4), p(35, -12), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-20, -13), p(19, -9), p(10, -3), p(15, -11), p(-1, 12), p(-8, 7)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(14, 19), p(34, 0), p(5, 32)];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static + StaticallyNamedEntity {
    type Score: ScoreType;

    fn psqt(&self, square: ChessSquare, piece: ChessPieceType, color: ChessColor) -> SingleFeatureScore<Self::Score>;

    fn passed_pawn(square: ChessSquare) -> SingleFeatureScore<Self::Score>;

    fn stoppable_passer() -> SingleFeatureScore<Self::Score>;

    fn close_king_passer() -> SingleFeatureScore<Self::Score>;

    fn backwards_pawn() -> SingleFeatureScore<Self::Score>;

    fn isolated_pawn() -> SingleFeatureScore<Self::Score>;

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

    fn close_king_passer() -> PhasedScore {
        CLOSE_KING_PASSER
    }

    fn backwards_pawn() -> PhasedScore {
        BACKWARDS_PAWN
    }

    fn isolated_pawn() -> PhasedScore {
        ISOLATED_PAWN
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
