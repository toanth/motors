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
        p( 114,  161),    p( 115,  160),    p( 119,  154),    p( 122,  148),    p( 120,  155),    p( 121,  169),    p(  88,  175),    p(  94,  175),
        p(  70,  119),    p(  71,  120),    p(  85,  111),    p(  85,  108),    p(  81,  102),    p( 129,  105),    p( 116,  120),    p( 120,  112),
        p(  51,  102),    p(  61,   98),    p(  59,   92),    p(  84,   97),    p(  88,   95),    p(  88,   85),    p(  89,   94),    p(  90,   92),
        p(  45,   89),    p(  50,   92),    p(  72,   92),    p(  89,   94),    p(  94,   93),    p(  90,   90),    p(  80,   88),    p(  76,   83),
        p(  35,   89),    p(  46,   85),    p(  68,   91),    p(  81,   93),    p(  81,   93),    p(  82,   91),    p(  84,   77),    p(  72,   80),
        p(  46,   96),    p(  54,   92),    p(  60,   93),    p(  58,   98),    p(  64,  101),    p(  91,   90),    p( 107,   78),    p(  82,   80),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 187,  284),    p( 191,  324),    p( 208,  324),    p( 240,  321),    p( 283,  309),    p( 208,  314),    p( 231,  298),    p( 197,  259),
        p( 275,  317),    p( 286,  323),    p( 297,  311),    p( 300,  315),    p( 305,  309),    p( 312,  301),    p( 295,  313),    p( 279,  305),
        p( 288,  314),    p( 304,  309),    p( 309,  314),    p( 319,  314),    p( 331,  311),    p( 351,  299),    p( 295,  308),    p( 295,  308),
        p( 307,  319),    p( 315,  315),    p( 329,  317),    p( 328,  325),    p( 328,  322),    p( 330,  319),    p( 320,  320),    p( 329,  312),
        p( 301,  319),    p( 313,  311),    p( 316,  317),    p( 324,  320),    p( 322,  322),    p( 329,  308),    p( 327,  308),    p( 317,  318),
        p( 279,  306),    p( 286,  304),    p( 296,  301),    p( 305,  312),    p( 309,  311),    p( 307,  291),    p( 307,  298),    p( 301,  307),
        p( 271,  314),    p( 285,  316),    p( 288,  310),    p( 297,  311),    p( 305,  307),    p( 299,  302),    p( 302,  312),    p( 297,  320),
        p( 242,  310),    p( 281,  309),    p( 270,  309),    p( 291,  314),    p( 298,  314),    p( 298,  305),    p( 288,  311),    p( 272,  313),
    ],
    // bishop
    [
        p( 276,  318),    p( 259,  313),    p( 229,  310),    p( 224,  321),    p( 210,  319),    p( 239,  309),    p( 259,  312),    p( 260,  308),
        p( 287,  307),    p( 288,  306),    p( 295,  307),    p( 279,  309),    p( 287,  304),    p( 294,  303),    p( 259,  314),    p( 273,  306),
        p( 300,  310),    p( 305,  307),    p( 290,  310),    p( 304,  300),    p( 304,  306),    p( 333,  305),    p( 318,  307),    p( 315,  316),
        p( 288,  314),    p( 298,  308),    p( 309,  301),    p( 309,  308),    p( 309,  304),    p( 311,  305),    p( 312,  305),    p( 290,  310),
        p( 289,  310),    p( 287,  310),    p( 296,  307),    p( 310,  302),    p( 306,  303),    p( 308,  298),    p( 292,  305),    p( 314,  299),
        p( 296,  306),    p( 298,  306),    p( 297,  308),    p( 299,  307),    p( 306,  305),    p( 301,  300),    p( 309,  295),    p( 312,  301),
        p( 307,  304),    p( 299,  301),    p( 305,  303),    p( 298,  312),    p( 300,  310),    p( 307,  306),    p( 316,  296),    p( 310,  303),
        p( 297,  304),    p( 309,  305),    p( 306,  310),    p( 291,  313),    p( 305,  313),    p( 293,  313),    p( 304,  307),    p( 305,  297),
    ],
    // rook
    [
        p( 452,  559),    p( 448,  563),    p( 436,  569),    p( 432,  569),    p( 447,  563),    p( 466,  563),    p( 455,  565),    p( 496,  543),
        p( 456,  559),    p( 455,  563),    p( 461,  564),    p( 476,  555),    p( 463,  558),    p( 492,  552),    p( 495,  551),    p( 507,  539),
        p( 456,  554),    p( 474,  548),    p( 467,  551),    p( 466,  544),    p( 491,  537),    p( 509,  532),    p( 517,  536),    p( 492,  536),
        p( 454,  554),    p( 463,  550),    p( 462,  551),    p( 465,  546),    p( 472,  542),    p( 488,  538),    p( 483,  544),    p( 479,  539),
        p( 443,  551),    p( 445,  549),    p( 446,  551),    p( 453,  547),    p( 461,  544),    p( 458,  544),    p( 473,  540),    p( 459,  539),
        p( 439,  546),    p( 443,  542),    p( 442,  544),    p( 445,  545),    p( 454,  538),    p( 464,  532),    p( 478,  527),    p( 465,  530),
        p( 439,  544),    p( 444,  542),    p( 450,  543),    p( 454,  539),    p( 461,  535),    p( 474,  528),    p( 485,  522),    p( 451,  534),
        p( 446,  548),    p( 446,  542),    p( 445,  546),    p( 450,  540),    p( 457,  534),    p( 457,  539),    p( 445,  545),    p( 443,  539),
    ],
    // queen
    [
        p( 870,  979),    p( 871,  984),    p( 885,  994),    p( 906,  985),    p( 910,  991),    p( 936,  979),    p( 970,  933),    p( 907,  961),
        p( 902,  966),    p( 888,  982),    p( 887,  999),    p( 879, 1021),    p( 879, 1036),    p( 918, 1009),    p( 913,  990),    p( 956,  961),
        p( 914,  961),    p( 908,  969),    p( 904,  992),    p( 899, 1006),    p( 902, 1017),    p( 943, 1007),    p( 956,  978),    p( 941,  980),
        p( 902,  968),    p( 904,  978),    p( 901,  982),    p( 892, 1006),    p( 895, 1014),    p( 914, 1004),    p( 918, 1008),    p( 926,  984),
        p( 897,  969),    p( 894,  975),    p( 894,  980),    p( 895,  995),    p( 899,  998),    p( 903,  996),    p( 912,  992),    p( 916,  980),
        p( 897,  953),    p( 901,  963),    p( 895,  978),    p( 893,  982),    p( 896,  992),    p( 906,  982),    p( 914,  975),    p( 914,  955),
        p( 899,  947),    p( 895,  961),    p( 900,  964),    p( 899,  978),    p( 900,  981),    p( 904,  963),    p( 912,  947),    p( 906,  938),
        p( 879,  960),    p( 892,  949),    p( 893,  957),    p( 897,  963),    p( 899,  954),    p( 885,  965),    p( 881,  944),    p( 894,  916),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  69,    6),    p(  84,    2),    p( 103,  -10),    p( 194,  -68),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   5,   27),    p( -36,   40),    p( -26,   30),    p(  -4,   16),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -48,   39),    p( -29,   31),    p( -34,   24),    p( -33,   21),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-118,   36),    p( -93,   28),    p( -89,   15),    p( -71,   15),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-135,   30),    p(-108,   17),    p(-108,    6),    p( -94,    8),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-112,   18),    p(-110,   11),    p( -82,   -2),    p( -57,    7),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-101,    9),    p( -88,    2),    p( -60,  -11),    p(   2,   -3),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  21,  -13),    p(   7,   -8),    p(  30,  -21),    p(  60,  -25),
    ],
];

const BISHOP_PAIR: PhasedScore = p(24, 51);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(13, 21), p(14, 19), p(13, 8), p(9, 1), p(5, -6), p(1, -14), p(-5, -22), p(-11, -35), p(-22, -40)];
const ROOK_OPEN_FILE: PhasedScore = p(15, 1);
const ROOK_CLOSED_FILE: PhasedScore = p(-11, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(6, -3);
const KING_OPEN_FILE: PhasedScore = p(-37, 6);
const KING_CLOSED_FILE: PhasedScore = p(12, -6);
const KING_SEMIOPEN_FILE: PhasedScore = p(-2, 11);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-4, 6), p(2, 8), p(-1, 7), p(4, 5), p(5, 5), p(5, 8), p(8, 5), p(21, 1)],
    // Closed
    [p(0, 0), p(0, 0), p(16, -15), p(-13, 12), p(2, 10), p(1, 6), p(0, 8), p(2, 6)],
    // SemiOpen
    [p(0, 0), p(-4, 28), p(7, 20), p(2, 12), p(1, 11), p(5, 7), p(3, 4), p(12, 7)],
    // SemiClosed
    [p(0, 0), p(13, -11), p(8, 7), p(4, 1), p(8, 2), p(2, 5), p(4, 5), p(4, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(22, 11),
    p(2, 5),
    p(-2, 5),
    p(-12, 6),
    p(3, 4),
    p(-12, -10),
    p(-6, -1),
    p(-8, -12),
    p(0, 0),
    p(-12, -1),
    p(-12, -16),
    p(-21, -7),
    p(2, -5),
    p(-5, -9),
    p(4, -6),
    p(2, 11),
    p(-6, -1),
    p(-23, -4),
    p(-16, 0),
    p(-42, 20),
    p(-22, 4),
    p(-22, -18),
    p(2, 24),
    p(-46, 21),
    p(-19, -16),
    p(-23, -15),
    p(-36, -31),
    p(-44, 9),
    p(-20, 0),
    p(9, -3),
    p(-89, 97),
    p(0, 0),
    p(1, -4),
    p(-13, -6),
    p(-5, -7),
    p(-25, -3),
    p(-23, -2),
    p(-47, -20),
    p(-28, 35),
    p(-34, 24),
    p(-6, -5),
    p(-20, -10),
    p(3, -11),
    p(-20, 33),
    p(-51, 19),
    p(-4, -30),
    p(0, 0),
    p(0, 0),
    p(-2, -12),
    p(-15, 6),
    p(-9, -54),
    p(0, 0),
    p(5, -10),
    p(-42, -9),
    p(0, 0),
    p(0, 0),
    p(-29, -5),
    p(-23, -13),
    p(-22, 16),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(24, 4),
    p(3, -2),
    p(-8, 3),
    p(-25, -1),
    p(8, -6),
    p(-27, -9),
    p(-20, -3),
    p(-39, -11),
    p(7, -3),
    p(-11, -8),
    p(-31, -6),
    p(-47, 1),
    p(-5, -6),
    p(-42, -6),
    p(-36, -7),
    p(-53, 62),
    p(11, -2),
    p(-0, -9),
    p(-4, -11),
    p(-23, -3),
    p(-9, -3),
    p(-16, -13),
    p(-25, -1),
    p(-69, 178),
    p(-4, -11),
    p(-25, -15),
    p(-33, -29),
    p(10, -82),
    p(-13, -9),
    p(-13, -19),
    p(-81, 69),
    p(0, 0),
    p(13, -2),
    p(-2, -5),
    p(-19, -7),
    p(-29, -6),
    p(-1, -1),
    p(-27, -18),
    p(-18, -3),
    p(-31, -0),
    p(-2, -8),
    p(-22, -8),
    p(-29, -14),
    p(-39, -4),
    p(-8, -3),
    p(-48, -5),
    p(-1, 16),
    p(-63, 61),
    p(2, -2),
    p(-14, -3),
    p(-32, 57),
    p(0, 0),
    p(-17, -6),
    p(-24, 6),
    p(0, 0),
    p(0, 0),
    p(-16, -0),
    p(-41, 9),
    p(-35, -44),
    p(0, 0),
    p(14, -63),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-11, 10),  /*0b0000*/
    p(-12, 8),   /*0b0001*/
    p(-3, 12),   /*0b0010*/
    p(-3, 11),   /*0b0011*/
    p(-6, 4),    /*0b0100*/
    p(-22, 0),   /*0b0101*/
    p(-9, 5),    /*0b0110*/
    p(-12, -11), /*0b0111*/
    p(3, 6),     /*0b1000*/
    p(-6, 10),   /*0b1001*/
    p(0, 11),    /*0b1010*/
    p(3, 12),    /*0b1011*/
    p(-3, 4),    /*0b1100*/
    p(-18, 4),   /*0b1101*/
    p(-9, 4),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(4, 10),    /*0b10000*/
    p(5, 7),     /*0b10001*/
    p(21, 8),    /*0b10010*/
    p(-1, 6),    /*0b10011*/
    p(-6, 4),    /*0b10100*/
    p(13, 13),   /*0b10101*/
    p(-28, 3),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(10, 11),   /*0b11000*/
    p(26, 12),   /*0b11001*/
    p(29, 26),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(9, -2),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(9, 4),     /*0b100000*/
    p(1, 8),     /*0b100001*/
    p(15, 4),    /*0b100010*/
    p(6, -1),    /*0b100011*/
    p(-4, -2),   /*0b100100*/
    p(-19, -12), /*0b100101*/
    p(-20, 19),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(17, -1),   /*0b101000*/
    p(1, 9),     /*0b101001*/
    p(11, -4),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(2, -0),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(5, 3),     /*0b110000*/
    p(14, 4),    /*0b110001*/
    p(16, -4),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(12, 13),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(21, -5),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(5, -4),    /*0b111111*/
    p(16, 2),    /*0b00*/
    p(22, -8),   /*0b01*/
    p(40, -8),   /*0b10*/
    p(13, -29),  /*0b11*/
    p(44, -3),   /*0b100*/
    p(15, -8),   /*0b101*/
    p(50, -28),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(58, -10),  /*0b1000*/
    p(12, -17),  /*0b1001*/
    p(31, -35),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(24, -17),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-18, 28),  /*0b1111*/
    p(23, 3),    /*0b00*/
    p(34, -10),  /*0b01*/
    p(30, -13),  /*0b10*/
    p(27, -37),  /*0b11*/
    p(33, -9),   /*0b100*/
    p(46, -19),  /*0b101*/
    p(21, -18),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(36, -2),   /*0b1000*/
    p(43, -14),  /*0b1001*/
    p(48, -43),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(30, -24),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(14, -42),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(-1, -37);
const STOPPABLE_PASSER: PhasedScore = p(25, -48);
const CLOSE_KING_PASSER: PhasedScore = p(3, 24);
const IMMOBILE_PASSER: PhasedScore = p(-5, -36);
const PROTECTED_PASSER: PhasedScore = p(8, -3);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -31,   46),    p( -39,   66),    p( -54,   61),    p( -58,   44),    p( -59,   46),    p( -47,   42),    p( -31,   43),    p( -57,   46),
        p( -24,   43),    p( -44,   67),    p( -50,   58),    p( -51,   44),    p( -68,   53),    p( -62,   50),    p( -49,   56),    p( -49,   43),
        p( -18,   55),    p( -27,   55),    p( -45,   59),    p( -43,   58),    p( -53,   63),    p( -48,   63),    p( -50,   71),    p( -69,   70),
        p(  -7,   71),    p( -11,   68),    p( -10,   60),    p( -28,   72),    p( -44,   80),    p( -36,   85),    p( -39,   91),    p( -61,   92),
        p(  -6,   62),    p(  11,   51),    p(  -3,   43),    p( -17,   37),    p( -17,   55),    p( -33,   69),    p( -53,   80),    p( -82,   91),
        p(  14,   61),    p(  15,   60),    p(  19,   54),    p(  22,   48),    p(  20,   55),    p(  21,   69),    p( -12,   75),    p(  -6,   75),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(1, 4), p(5, 8), p(8, 17), p(15, 22), p(18, 72), p(12, 64)];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -6);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);
const PHALANX: [PhasedScore; 6] = [p(-1, 0), p(5, 3), p(9, 6), p(21, 20), p(58, 74), p(-82, 216)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 14), p(7, 18), p(14, 21), p(7, 10), p(-3, 14), p(-48, 11)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(49, 21), p(52, 45), p(67, 4), p(52, -8), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(1, -5), p(14, 21), p(19, -7), p(16, 10), p(16, -9), p(29, -12)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-41, -69),
        p(-20, -29),
        p(-8, -5),
        p(1, 8),
        p(8, 19),
        p(15, 30),
        p(23, 33),
        p(29, 35),
        p(33, 34),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-30, -56),
        p(-17, -38),
        p(-6, -22),
        p(2, -9),
        p(9, 0),
        p(14, 9),
        p(19, 14),
        p(24, 19),
        p(26, 24),
        p(33, 26),
        p(39, 26),
        p(44, 30),
        p(38, 40),
        p(51, 32),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-71, 17),
        p(-63, 30),
        p(-59, 37),
        p(-54, 42),
        p(-55, 49),
        p(-50, 56),
        p(-47, 61),
        p(-44, 65),
        p(-41, 70),
        p(-37, 74),
        p(-33, 77),
        p(-31, 82),
        p(-24, 84),
        p(-15, 82),
        p(-11, 79),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-19, -23),
        p(-19, 9),
        p(-24, 67),
        p(-19, 83),
        p(-17, 102),
        p(-13, 107),
        p(-9, 119),
        p(-6, 125),
        p(-2, 132),
        p(2, 133),
        p(5, 137),
        p(9, 141),
        p(12, 142),
        p(14, 148),
        p(17, 150),
        p(21, 153),
        p(23, 159),
        p(27, 160),
        p(37, 157),
        p(52, 150),
        p(58, 150),
        p(99, 128),
        p(99, 131),
        p(127, 108),
        p(215, 78),
        p(257, 40),
        p(288, 27),
        p(261, 28),
    ],
    [
        p(-88, 0),
        p(-58, -10),
        p(-29, -9),
        p(0, -5),
        p(32, -3),
        p(55, -1),
        p(85, 5),
        p(111, 7),
        p(158, -1),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-4, 14), p(0, 0), p(28, 27), p(61, 8), p(40, -0), p(0, 0)],
    [p(-2, 12), p(21, 26), p(0, 0), p(43, 23), p(47, 92), p(0, 0)],
    [p(-4, 19), p(11, 17), p(19, 14), p(0, 0), p(66, 45), p(0, 0)],
    [p(-2, 9), p(2, 9), p(1, 25), p(0, 13), p(0, 0), p(0, 0)],
    [p(66, 21), p(-17, 22), p(16, 15), p(-11, 23), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 8), p(8, 7), p(6, 10), p(12, 7), p(8, 14), p(3, 8)],
    [p(2, 9), p(11, 22), p(-4, -44), p(9, 12), p(10, 20), p(4, 6)],
    [p(2, 4), p(13, 8), p(9, 13), p(11, 10), p(9, 28), p(19, -3)],
    [p(2, 2), p(8, 3), p(7, -2), p(5, 13), p(-61, -226), p(5, -11)],
    [p(50, 0), p(36, 12), p(41, 6), p(27, 6), p(38, -9), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-22, -14), p(17, -9), p(10, -3), p(16, -16), p(0, 4), p(-6, 3)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(10, 0), p(-6, 8), p(14, -8), p(-14, 24)];
const CHECK_STM: PhasedScore = p(36, 22);
const DISCOVERED_CHECK_STM: PhasedScore = p(145, 37);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(-6, -18), p(66, -1), p(104, -32), p(62, 81), p(0, 0), p(-25, -21)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(6, -17), p(26, 30), p(16, 34), p(44, 9), p(63, 3)];

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

    fn candidate_passer(rank: DimT) -> SingleFeatureScore<Self::Score>;

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

    fn pawnless_flank() -> SingleFeatureScore<Self::Score>;

    fn pawn_protection(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn pawn_attack(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn pawn_advance_threat(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn mobility(piece: ChessPieceType, mobility: usize) -> SingleFeatureScore<Self::Score>;

    fn threats(attacking: ChessPieceType, targeted: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn king_zone_attack(attacking: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn can_give_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn check_stm() -> SingleFeatureScore<Self::Score>;

    fn discovered_check_stm() -> SingleFeatureScore<Self::Score>;

    fn discovered_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn pin(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;
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

    fn candidate_passer(rank: DimT) -> SingleFeatureScore<Self::Score> {
        CANDIDATE_PASSER[rank as usize]
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

    fn pawnless_flank() -> PhasedScore {
        PAWNLESS_FLANK
    }

    fn pawn_protection(piece: ChessPieceType) -> PhasedScore {
        PAWN_PROTECTION[piece as usize]
    }

    fn pawn_attack(piece: ChessPieceType) -> PhasedScore {
        PAWN_ATTACKS[piece as usize]
    }

    fn pawn_advance_threat(piece: ChessPieceType) -> PhasedScore {
        PAWN_ADVANCE_THREAT[piece as usize]
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

    fn discovered_check_stm() -> PhasedScore {
        DISCOVERED_CHECK_STM
    }

    fn pin(piece: ChessPieceType) -> PhasedScore {
        PIN[piece as usize]
    }

    fn discovered_check(piece: ChessPieceType) -> PhasedScore {
        DISCOVERED_CHECK[piece as usize]
    }

    fn check_stm() -> PhasedScore {
        CHECK_STM
    }
}
