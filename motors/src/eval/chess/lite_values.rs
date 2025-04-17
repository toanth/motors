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
        p( 114,  161),    p( 114,  160),    p( 120,  154),    p( 122,  148),    p( 120,  155),    p( 121,  169),    p(  88,  175),    p(  94,  175),
        p(  70,  120),    p(  71,  121),    p(  85,  112),    p(  85,  109),    p(  82,  103),    p( 129,  106),    p( 116,  121),    p( 120,  113),
        p(  51,  103),    p(  61,   99),    p(  60,   93),    p(  84,   98),    p(  88,   96),    p(  89,   85),    p(  89,   95),    p(  90,   92),
        p(  45,   90),    p(  50,   93),    p(  72,   92),    p(  90,   95),    p(  95,   94),    p(  91,   91),    p(  80,   89),    p(  77,   83),
        p(  36,   90),    p(  47,   86),    p(  68,   91),    p(  82,   93),    p(  81,   94),    p(  82,   92),    p(  85,   79),    p(  73,   81),
        p(  47,   96),    p(  54,   93),    p(  60,   94),    p(  59,   99),    p(  64,  102),    p(  90,   91),    p( 107,   79),    p(  83,   81),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 187,  284),    p( 191,  324),    p( 208,  325),    p( 240,  322),    p( 283,  309),    p( 208,  315),    p( 231,  298),    p( 197,  259),
        p( 275,  317),    p( 286,  323),    p( 297,  312),    p( 301,  316),    p( 305,  310),    p( 312,  302),    p( 295,  314),    p( 279,  304),
        p( 286,  313),    p( 302,  308),    p( 308,  314),    p( 320,  315),    p( 331,  312),    p( 348,  298),    p( 292,  307),    p( 292,  307),
        p( 306,  318),    p( 314,  313),    p( 328,  315),    p( 327,  325),    p( 327,  321),    p( 330,  318),    p( 319,  317),    p( 328,  310),
        p( 300,  318),    p( 313,  310),    p( 315,  317),    p( 323,  319),    p( 321,  322),    p( 329,  308),    p( 327,  308),    p( 317,  317),
        p( 279,  307),    p( 286,  305),    p( 296,  302),    p( 305,  313),    p( 309,  311),    p( 307,  292),    p( 307,  299),    p( 301,  308),
        p( 271,  314),    p( 285,  317),    p( 288,  311),    p( 297,  312),    p( 305,  307),    p( 299,  303),    p( 302,  312),    p( 297,  320),
        p( 243,  309),    p( 281,  309),    p( 271,  310),    p( 291,  314),    p( 298,  314),    p( 298,  305),    p( 288,  311),    p( 272,  312),
    ],
    // bishop
    [
        p( 275,  318),    p( 259,  313),    p( 228,  310),    p( 223,  321),    p( 210,  320),    p( 238,  310),    p( 258,  313),    p( 259,  308),
        p( 286,  307),    p( 287,  306),    p( 295,  307),    p( 279,  310),    p( 287,  305),    p( 294,  304),    p( 258,  314),    p( 273,  306),
        p( 298,  308),    p( 302,  305),    p( 289,  310),    p( 304,  301),    p( 304,  306),    p( 329,  304),    p( 316,  306),    p( 312,  312),
        p( 287,  312),    p( 298,  307),    p( 308,  301),    p( 308,  308),    p( 308,  304),    p( 310,  304),    p( 311,  303),    p( 289,  308),
        p( 289,  309),    p( 288,  309),    p( 296,  306),    p( 309,  302),    p( 305,  302),    p( 308,  298),    p( 292,  305),    p( 315,  298),
        p( 296,  306),    p( 298,  306),    p( 298,  309),    p( 299,  307),    p( 307,  306),    p( 301,  301),    p( 309,  295),    p( 312,  301),
        p( 307,  304),    p( 299,  302),    p( 306,  303),    p( 299,  313),    p( 300,  310),    p( 308,  307),    p( 317,  296),    p( 310,  303),
        p( 297,  304),    p( 309,  305),    p( 306,  311),    p( 292,  313),    p( 306,  313),    p( 293,  313),    p( 304,  307),    p( 305,  297),
    ],
    // rook
    [
        p( 452,  559),    p( 448,  563),    p( 436,  569),    p( 432,  568),    p( 447,  562),    p( 467,  563),    p( 455,  565),    p( 496,  543),
        p( 456,  559),    p( 454,  563),    p( 461,  563),    p( 476,  555),    p( 463,  558),    p( 492,  552),    p( 495,  551),    p( 507,  538),
        p( 455,  554),    p( 472,  547),    p( 466,  550),    p( 466,  544),    p( 491,  537),    p( 505,  531),    p( 514,  535),    p( 489,  536),
        p( 453,  553),    p( 461,  549),    p( 461,  550),    p( 464,  546),    p( 471,  541),    p( 487,  537),    p( 481,  543),    p( 477,  538),
        p( 443,  550),    p( 445,  549),    p( 446,  550),    p( 452,  546),    p( 460,  543),    p( 458,  543),    p( 474,  540),    p( 459,  538),
        p( 439,  546),    p( 444,  542),    p( 442,  544),    p( 445,  544),    p( 454,  538),    p( 465,  532),    p( 479,  527),    p( 466,  530),
        p( 440,  544),    p( 444,  542),    p( 451,  543),    p( 454,  539),    p( 462,  535),    p( 474,  528),    p( 485,  522),    p( 452,  534),
        p( 446,  548),    p( 446,  542),    p( 446,  546),    p( 451,  540),    p( 457,  534),    p( 458,  539),    p( 446,  545),    p( 443,  539),
    ],
    // queen
    [
        p( 871,  979),    p( 872,  984),    p( 885,  994),    p( 906,  985),    p( 911,  991),    p( 937,  979),    p( 971,  933),    p( 908,  961),
        p( 902,  966),    p( 888,  982),    p( 887,  999),    p( 879, 1021),    p( 879, 1036),    p( 918, 1009),    p( 913,  990),    p( 956,  962),
        p( 912,  962),    p( 906,  969),    p( 902,  993),    p( 899, 1006),    p( 901, 1017),    p( 939, 1005),    p( 953,  976),    p( 938,  980),
        p( 901,  969),    p( 903,  977),    p( 900,  982),    p( 891, 1006),    p( 893, 1013),    p( 914, 1003),    p( 916, 1007),    p( 925,  983),
        p( 897,  969),    p( 894,  974),    p( 893,  980),    p( 894,  995),    p( 899,  998),    p( 903,  995),    p( 912,  992),    p( 916,  980),
        p( 897,  953),    p( 901,  964),    p( 895,  978),    p( 893,  982),    p( 896,  992),    p( 907,  982),    p( 914,  976),    p( 915,  955),
        p( 900,  947),    p( 896,  961),    p( 900,  965),    p( 900,  978),    p( 901,  981),    p( 904,  963),    p( 913,  946),    p( 906,  939),
        p( 880,  961),    p( 893,  949),    p( 893,  958),    p( 898,  963),    p( 900,  954),    p( 886,  965),    p( 882,  944),    p( 895,  916),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  69,    7),    p(  84,    3),    p( 103,   -9),    p( 192,  -66),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   7,   26),    p( -35,   39),    p( -26,   30),    p(  -5,   17),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -47,   38),    p( -28,   30),    p( -34,   24),    p( -33,   21),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-118,   35),    p( -92,   27),    p( -89,   14),    p( -73,   15),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-135,   29),    p(-107,   16),    p(-108,    5),    p( -95,    8),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-111,   17),    p(-109,   10),    p( -81,   -3),    p( -57,    7),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-100,    7),    p( -87,    1),    p( -59,  -13),    p(   2,   -3),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  21,  -12),    p(   7,   -7),    p(  30,  -20),    p(  59,  -23),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 51);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(13, 21), p(14, 19), p(13, 8), p(9, 1), p(5, -5), p(1, -14), p(-5, -21), p(-11, -33), p(-22, -37)];
const ROOK_OPEN_FILE: PhasedScore = p(15, 1);
const ROOK_CLOSED_FILE: PhasedScore = p(-11, -4);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(6, -4);
const KING_OPEN_FILE: PhasedScore = p(-37, 6);
const KING_CLOSED_FILE: PhasedScore = p(12, -6);
const KING_SEMIOPEN_FILE: PhasedScore = p(-2, 11);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-4, 6), p(2, 8), p(-1, 7), p(4, 5), p(4, 5), p(5, 8), p(8, 5), p(21, 2)],
    // Closed
    [p(0, 0), p(0, 0), p(16, -14), p(-14, 12), p(1, 10), p(1, 6), p(0, 9), p(2, 6)],
    // SemiOpen
    [p(0, 0), p(-3, 28), p(8, 21), p(2, 12), p(1, 10), p(5, 7), p(3, 4), p(12, 7)],
    // SemiClosed
    [p(0, 0), p(13, -11), p(8, 7), p(4, 1), p(8, 1), p(2, 5), p(4, 5), p(4, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(22, 10),
    p(2, 5),
    p(-2, 5),
    p(-11, 6),
    p(3, 4),
    p(-11, -9),
    p(-6, -1),
    p(-8, -12),
    p(0, 0),
    p(-11, -0),
    p(-12, -15),
    p(-21, -7),
    p(2, -5),
    p(-5, -10),
    p(3, -7),
    p(1, 11),
    p(-6, -1),
    p(-22, -3),
    p(-17, -1),
    p(-42, 21),
    p(-22, 5),
    p(-21, -15),
    p(1, 25),
    p(-47, 22),
    p(-18, -15),
    p(-22, -14),
    p(-36, -32),
    p(-43, 10),
    p(-19, 1),
    p(10, -2),
    p(-88, 99),
    p(0, 0),
    p(1, -4),
    p(-12, -5),
    p(-5, -6),
    p(-24, -2),
    p(-23, -2),
    p(-47, -20),
    p(-28, 36),
    p(-34, 23),
    p(-6, -5),
    p(-19, -8),
    p(4, -9),
    p(-20, 34),
    p(-51, 19),
    p(-3, -29),
    p(0, 0),
    p(0, 0),
    p(-2, -11),
    p(-14, 7),
    p(-9, -54),
    p(0, 0),
    p(5, -12),
    p(-42, -11),
    p(0, 0),
    p(0, 0),
    p(-29, -5),
    p(-23, -13),
    p(-22, 15),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(23, 3),
    p(3, -2),
    p(-8, 3),
    p(-24, -0),
    p(8, -5),
    p(-27, -8),
    p(-20, -3),
    p(-39, -10),
    p(7, -2),
    p(-11, -8),
    p(-31, -5),
    p(-47, 1),
    p(-5, -6),
    p(-42, -5),
    p(-36, -9),
    p(-54, 61),
    p(11, -2),
    p(0, -9),
    p(-5, -11),
    p(-23, -1),
    p(-9, -2),
    p(-15, -11),
    p(-24, -1),
    p(-68, 179),
    p(-4, -11),
    p(-24, -14),
    p(-33, -29),
    p(11, -81),
    p(-13, -9),
    p(-12, -17),
    p(-80, 71),
    p(0, 0),
    p(12, -2),
    p(-2, -5),
    p(-18, -7),
    p(-28, -6),
    p(-2, -2),
    p(-28, -18),
    p(-17, -1),
    p(-30, 1),
    p(-1, -8),
    p(-21, -7),
    p(-28, -12),
    p(-38, -2),
    p(-8, -3),
    p(-48, -4),
    p(-0, 16),
    p(-64, 65),
    p(1, -2),
    p(-14, -3),
    p(-33, 56),
    p(0, 0),
    p(-17, -6),
    p(-24, 5),
    p(0, 0),
    p(0, 0),
    p(-16, -1),
    p(-41, 9),
    p(-35, -44),
    p(0, 0),
    p(14, -61),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-11, 9),   /*0b0000*/
    p(-12, 7),   /*0b0001*/
    p(-3, 12),   /*0b0010*/
    p(-3, 11),   /*0b0011*/
    p(-6, 3),    /*0b0100*/
    p(-22, 0),   /*0b0101*/
    p(-9, 6),    /*0b0110*/
    p(-12, -10), /*0b0111*/
    p(3, 6),     /*0b1000*/
    p(-6, 9),    /*0b1001*/
    p(0, 11),    /*0b1010*/
    p(3, 12),    /*0b1011*/
    p(-3, 4),    /*0b1100*/
    p(-18, 5),   /*0b1101*/
    p(-9, 4),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(3, 10),    /*0b10000*/
    p(5, 7),     /*0b10001*/
    p(22, 8),    /*0b10010*/
    p(-0, 7),    /*0b10011*/
    p(-6, 3),    /*0b10100*/
    p(13, 13),   /*0b10101*/
    p(-29, 3),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(10, 10),   /*0b11000*/
    p(27, 11),   /*0b11001*/
    p(29, 26),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(9, -3),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(9, 3),     /*0b100000*/
    p(1, 8),     /*0b100001*/
    p(15, 4),    /*0b100010*/
    p(6, -1),    /*0b100011*/
    p(-3, -3),   /*0b100100*/
    p(-19, -12), /*0b100101*/
    p(-20, 19),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(17, -2),   /*0b101000*/
    p(1, 8),     /*0b101001*/
    p(12, -5),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(2, -1),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(6, 3),     /*0b110000*/
    p(14, 3),    /*0b110001*/
    p(16, -4),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(12, 13),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(21, -6),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(5, -4),    /*0b111111*/
    p(16, 2),    /*0b00*/
    p(22, -7),   /*0b01*/
    p(40, -8),   /*0b10*/
    p(13, -28),  /*0b11*/
    p(43, -2),   /*0b100*/
    p(15, -7),   /*0b101*/
    p(50, -27),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(59, -9),   /*0b1000*/
    p(12, -16),  /*0b1001*/
    p(33, -35),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(24, -16),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-18, 29),  /*0b1111*/
    p(23, 3),    /*0b00*/
    p(34, -9),   /*0b01*/
    p(30, -11),  /*0b10*/
    p(27, -35),  /*0b11*/
    p(33, -9),   /*0b100*/
    p(47, -18),  /*0b101*/
    p(21, -17),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(36, -1),   /*0b1000*/
    p(43, -13),  /*0b1001*/
    p(48, -41),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(30, -23),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(14, -41),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(-1, -38);
const STOPPABLE_PASSER: PhasedScore = p(25, -48);
const CLOSE_KING_PASSER: PhasedScore = p(3, 24);
const IMMOBILE_PASSER: PhasedScore = p(-2, -25);
const PROTECTED_PASSER: PhasedScore = p(8, -3);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -31,   46),    p( -39,   66),    p( -54,   61),    p( -59,   44),    p( -60,   46),    p( -47,   42),    p( -30,   43),    p( -57,   46),
        p( -24,   43),    p( -45,   67),    p( -50,   58),    p( -52,   44),    p( -69,   53),    p( -63,   50),    p( -49,   56),    p( -49,   44),
        p( -18,   56),    p( -27,   55),    p( -46,   59),    p( -44,   57),    p( -54,   62),    p( -49,   63),    p( -50,   71),    p( -69,   70),
        p(  -8,   71),    p( -12,   68),    p( -12,   59),    p( -30,   71),    p( -45,   79),    p( -37,   84),    p( -40,   90),    p( -61,   92),
        p(  -8,   61),    p(  10,   50),    p(  -4,   42),    p( -18,   36),    p( -19,   54),    p( -34,   68),    p( -54,   78),    p( -84,   89),
        p(  14,   61),    p(  14,   60),    p(  20,   54),    p(  22,   48),    p(  20,   55),    p(  21,   69),    p( -12,   75),    p(  -6,   75),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(1, 4), p(5, 7), p(8, 16), p(15, 22), p(20, 72), p(12, 63)];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -6);
const DOUBLED_PAWN: PhasedScore = p(-7, -22);
const IMMOBILE_PAWN: PhasedScore = p(-3, -14);
const PHALANX: [PhasedScore; 6] = [p(-1, 0), p(5, 3), p(9, 6), p(21, 20), p(57, 74), p(-82, 217)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 15), p(7, 17), p(14, 20), p(7, 9), p(-3, 13), p(-47, 11)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(49, 21), p(51, 46), p(66, 5), p(52, -8), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(-1, -8), p(13, 21), p(18, -7), p(15, 11), p(16, -9), p(29, -12)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-41, -68),
        p(-20, -28),
        p(-8, -5),
        p(1, 8),
        p(8, 19),
        p(15, 29),
        p(23, 32),
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
        p(-17, -37),
        p(-5, -22),
        p(2, -9),
        p(9, 0),
        p(14, 10),
        p(19, 15),
        p(24, 19),
        p(25, 24),
        p(32, 25),
        p(38, 26),
        p(44, 29),
        p(38, 40),
        p(51, 31),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-72, 16),
        p(-63, 29),
        p(-59, 37),
        p(-55, 42),
        p(-55, 49),
        p(-50, 56),
        p(-47, 61),
        p(-44, 65),
        p(-41, 71),
        p(-37, 75),
        p(-33, 78),
        p(-32, 83),
        p(-24, 84),
        p(-16, 82),
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
        p(-19, -26),
        p(-19, 6),
        p(-24, 64),
        p(-19, 80),
        p(-17, 99),
        p(-13, 105),
        p(-9, 117),
        p(-6, 124),
        p(-2, 130),
        p(2, 132),
        p(5, 136),
        p(9, 140),
        p(12, 142),
        p(14, 147),
        p(17, 150),
        p(22, 153),
        p(23, 159),
        p(27, 160),
        p(38, 157),
        p(53, 150),
        p(59, 151),
        p(100, 128),
        p(101, 131),
        p(128, 109),
        p(216, 78),
        p(260, 39),
        p(290, 28),
        p(265, 27),
    ],
    [
        p(-87, -3),
        p(-57, -13),
        p(-29, -11),
        p(0, -6),
        p(31, -3),
        p(54, -1),
        p(83, 6),
        p(109, 10),
        p(157, 2),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-5, 14), p(0, 0), p(28, 26), p(61, 8), p(40, 1), p(0, 0)],
    [p(-2, 12), p(21, 25), p(0, 0), p(43, 23), p(47, 92), p(0, 0)],
    [p(-4, 18), p(11, 17), p(19, 13), p(0, 0), p(66, 45), p(0, 0)],
    [p(-2, 9), p(2, 9), p(1, 24), p(0, 13), p(0, 0), p(0, 0)],
    [p(65, 19), p(-17, 22), p(15, 14), p(-10, 23), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 8), p(8, 7), p(6, 10), p(12, 7), p(8, 14), p(3, 9)],
    [p(2, 9), p(11, 21), p(-4, -44), p(9, 12), p(10, 19), p(4, 6)],
    [p(2, 4), p(13, 9), p(9, 14), p(11, 11), p(9, 29), p(19, -3)],
    [p(2, 2), p(8, 3), p(7, -1), p(5, 14), p(-62, -225), p(5, -11)],
    [p(49, 1), p(36, 13), p(41, 7), p(26, 7), p(38, -8), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-22, -14), p(17, -9), p(10, -3), p(16, -16), p(1, 4), p(-6, 2)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(10, -0), p(-6, 8), p(14, -8), p(-14, 24)];
const CHECK_STM: PhasedScore = p(36, 22);
const DISCOVERED_CHECK_STM: PhasedScore = p(145, 37);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(-6, -18), p(67, -1), p(105, -33), p(62, 82), p(0, 0), p(-25, -21)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(6, -17), p(27, 30), p(17, 34), p(44, 9), p(63, 3)];

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

    fn immobile_pawn() -> SingleFeatureScore<Self::Score>;

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

    fn immobile_pawn() -> PhasedScore {
        IMMOBILE_PAWN
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
