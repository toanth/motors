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
        p( 115,  161),    p( 114,  160),    p( 120,  154),    p( 123,  148),    p( 121,  155),    p( 122,  169),    p(  86,  176),    p(  94,  175),
        p(  69,  119),    p(  71,  121),    p(  85,  111),    p(  85,  108),    p(  81,  103),    p( 129,  105),    p( 115,  120),    p( 118,  113),
        p(  51,  102),    p(  61,   98),    p(  59,   92),    p(  84,   97),    p(  88,   95),    p(  87,   85),    p(  88,   94),    p(  90,   92),
        p(  45,   89),    p(  49,   92),    p(  72,   92),    p(  89,   94),    p(  94,   93),    p(  90,   90),    p(  80,   88),    p(  76,   83),
        p(  35,   89),    p(  46,   85),    p(  67,   91),    p(  81,   93),    p(  81,   93),    p(  82,   91),    p(  84,   77),    p(  72,   80),
        p(  46,   96),    p(  54,   92),    p(  60,   93),    p(  58,   98),    p(  64,  101),    p(  90,   90),    p( 107,   78),    p(  82,   80),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 187,  284),    p( 190,  324),    p( 209,  324),    p( 242,  321),    p( 286,  308),    p( 208,  314),    p( 231,  297),    p( 198,  258),
        p( 275,  316),    p( 286,  323),    p( 298,  311),    p( 301,  315),    p( 305,  309),    p( 313,  301),    p( 295,  313),    p( 280,  304),
        p( 288,  314),    p( 304,  308),    p( 309,  313),    p( 320,  314),    p( 331,  311),    p( 352,  298),    p( 295,  308),    p( 296,  307),
        p( 307,  319),    p( 315,  315),    p( 329,  317),    p( 328,  324),    p( 328,  322),    p( 330,  319),    p( 320,  320),    p( 329,  312),
        p( 301,  319),    p( 313,  311),    p( 316,  317),    p( 324,  320),    p( 322,  322),    p( 329,  308),    p( 327,  308),    p( 317,  318),
        p( 279,  306),    p( 286,  304),    p( 296,  301),    p( 305,  313),    p( 309,  311),    p( 307,  291),    p( 307,  298),    p( 301,  308),
        p( 271,  314),    p( 285,  316),    p( 287,  310),    p( 297,  311),    p( 304,  307),    p( 299,  302),    p( 302,  312),    p( 297,  320),
        p( 242,  310),    p( 280,  309),    p( 270,  309),    p( 291,  314),    p( 298,  314),    p( 297,  304),    p( 288,  311),    p( 272,  313),
    ],
    // bishop
    [
        p( 275,  318),    p( 260,  313),    p( 230,  310),    p( 225,  321),    p( 212,  319),    p( 236,  310),    p( 259,  312),    p( 259,  308),
        p( 287,  306),    p( 288,  306),    p( 295,  307),    p( 280,  309),    p( 287,  305),    p( 297,  303),    p( 258,  314),    p( 278,  305),
        p( 301,  310),    p( 305,  307),    p( 290,  310),    p( 305,  299),    p( 304,  306),    p( 333,  305),    p( 318,  307),    p( 316,  316),
        p( 288,  314),    p( 299,  308),    p( 309,  301),    p( 309,  307),    p( 309,  304),    p( 311,  305),    p( 312,  305),    p( 290,  310),
        p( 289,  310),    p( 287,  310),    p( 296,  307),    p( 310,  302),    p( 306,  303),    p( 308,  298),    p( 292,  305),    p( 314,  299),
        p( 296,  306),    p( 298,  306),    p( 297,  308),    p( 298,  307),    p( 306,  306),    p( 301,  300),    p( 309,  295),    p( 312,  301),
        p( 307,  304),    p( 299,  301),    p( 305,  303),    p( 298,  312),    p( 300,  310),    p( 307,  306),    p( 316,  296),    p( 310,  303),
        p( 297,  304),    p( 308,  305),    p( 305,  310),    p( 291,  313),    p( 305,  313),    p( 293,  313),    p( 303,  307),    p( 305,  297),
    ],
    // rook
    [
        p( 451,  559),    p( 447,  563),    p( 435,  569),    p( 432,  569),    p( 445,  563),    p( 466,  563),    p( 455,  565),    p( 494,  543),
        p( 456,  559),    p( 454,  563),    p( 460,  563),    p( 476,  555),    p( 463,  557),    p( 492,  552),    p( 494,  551),    p( 506,  539),
        p( 456,  554),    p( 473,  547),    p( 467,  550),    p( 466,  544),    p( 490,  537),    p( 508,  532),    p( 516,  536),    p( 491,  536),
        p( 453,  553),    p( 462,  549),    p( 462,  550),    p( 465,  546),    p( 472,  542),    p( 487,  538),    p( 482,  544),    p( 479,  538),
        p( 443,  551),    p( 445,  549),    p( 445,  550),    p( 453,  546),    p( 460,  544),    p( 458,  544),    p( 473,  540),    p( 459,  538),
        p( 438,  546),    p( 443,  542),    p( 442,  544),    p( 444,  544),    p( 454,  538),    p( 464,  532),    p( 478,  526),    p( 465,  530),
        p( 439,  544),    p( 444,  542),    p( 450,  542),    p( 453,  539),    p( 461,  535),    p( 474,  528),    p( 484,  522),    p( 451,  534),
        p( 446,  548),    p( 446,  541),    p( 445,  546),    p( 450,  540),    p( 456,  534),    p( 457,  538),    p( 445,  545),    p( 443,  539),
    ],
    // queen
    [
        p( 869,  979),    p( 871,  984),    p( 884,  994),    p( 905,  985),    p( 910,  990),    p( 936,  978),    p( 970,  932),    p( 907,  960),
        p( 902,  965),    p( 888,  982),    p( 887,  999),    p( 879, 1021),    p( 878, 1036),    p( 919, 1008),    p( 913,  990),    p( 957,  960),
        p( 913,  960),    p( 907,  969),    p( 903,  992),    p( 899, 1005),    p( 901, 1016),    p( 942, 1007),    p( 955,  977),    p( 940,  979),
        p( 901,  968),    p( 903,  977),    p( 900,  982),    p( 891, 1005),    p( 894, 1013),    p( 913, 1004),    p( 917, 1008),    p( 926,  983),
        p( 896,  968),    p( 893,  975),    p( 893,  980),    p( 894,  995),    p( 899,  998),    p( 902,  995),    p( 911,  991),    p( 915,  978),
        p( 896,  953),    p( 900,  963),    p( 894,  978),    p( 892,  983),    p( 896,  993),    p( 906,  981),    p( 913,  974),    p( 913,  953),
        p( 898,  946),    p( 895,  961),    p( 899,  965),    p( 899,  979),    p( 900,  981),    p( 903,  963),    p( 912,  946),    p( 905,  938),
        p( 878,  961),    p( 892,  949),    p( 892,  957),    p( 897,  963),    p( 898,  952),    p( 884,  964),    p( 881,  943),    p( 892,  917),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  70,    6),    p(  82,    2),    p( 102,  -10),    p( 193,  -68),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   6,   27),    p( -35,   39),    p( -26,   29),    p(  -5,   15),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -47,   38),    p( -27,   31),    p( -34,   24),    p( -35,   21),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-117,   36),    p( -92,   28),    p( -89,   15),    p( -72,   15),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-134,   30),    p(-107,   17),    p(-108,    6),    p( -95,    8),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-111,   18),    p(-110,   11),    p( -82,   -2),    p( -58,    7),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -99,    9),    p( -87,    2),    p( -58,  -12),    p(   2,   -3),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  21,  -13),    p(   6,   -8),    p(  30,  -21),    p(  59,  -25),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 52);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(13, 21), p(14, 19), p(13, 8), p(9, 1), p(4, -6), p(1, -14), p(-5, -22), p(-12, -35), p(-23, -39)];
const ROOK_OPEN_FILE: PhasedScore = p(14, 1);
const ROOK_CLOSED_FILE: PhasedScore = p(-11, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(6, -3);
const KING_OPEN_FILE: PhasedScore = p(-37, 6);
const KING_CLOSED_FILE: PhasedScore = p(12, -6);
const KING_SEMIOPEN_FILE: PhasedScore = p(-3, 11);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-4, 6), p(2, 8), p(-1, 7), p(4, 5), p(4, 5), p(4, 8), p(8, 5), p(21, 2)],
    // Closed
    [p(0, 0), p(0, 0), p(16, -16), p(-14, 12), p(2, 10), p(1, 6), p(0, 8), p(2, 5)],
    // SemiOpen
    [p(0, 0), p(-5, 28), p(7, 20), p(2, 12), p(2, 10), p(5, 7), p(3, 4), p(12, 7)],
    // SemiClosed
    [p(0, 0), p(13, -12), p(8, 7), p(4, 1), p(8, 2), p(2, 5), p(4, 5), p(4, 5)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(22, 11),
    p(3, 5),
    p(-2, 5),
    p(-12, 6),
    p(3, 4),
    p(-11, -10),
    p(-6, -1),
    p(-7, -12),
    p(0, 0),
    p(-12, -1),
    p(-12, -15),
    p(-21, -7),
    p(2, -5),
    p(-6, -10),
    p(4, -7),
    p(2, 12),
    p(-5, -1),
    p(-22, -4),
    p(-16, 0),
    p(-42, 20),
    p(-22, 4),
    p(-22, -18),
    p(1, 24),
    p(-47, 25),
    p(-18, -16),
    p(-22, -16),
    p(-37, -30),
    p(-44, 12),
    p(-20, -0),
    p(9, -5),
    p(-86, 95),
    p(0, 0),
    p(1, -4),
    p(-13, -6),
    p(-5, -7),
    p(-25, -2),
    p(-23, -2),
    p(-48, -20),
    p(-27, 35),
    p(-34, 25),
    p(-6, -5),
    p(-20, -10),
    p(3, -12),
    p(-20, 32),
    p(-51, 18),
    p(-4, -31),
    p(0, 0),
    p(0, 0),
    p(-2, -12),
    p(-15, 6),
    p(-8, -55),
    p(0, 0),
    p(5, -11),
    p(-39, -16),
    p(0, 0),
    p(0, 0),
    p(-28, -6),
    p(-23, -13),
    p(-21, 16),
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
    p(-20, -2),
    p(-39, -11),
    p(7, -2),
    p(-11, -8),
    p(-32, -6),
    p(-48, 1),
    p(-5, -6),
    p(-42, -6),
    p(-36, -8),
    p(-53, 60),
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
    p(-33, -28),
    p(9, -80),
    p(-13, -9),
    p(-12, -19),
    p(-80, 68),
    p(0, 0),
    p(13, -2),
    p(-2, -5),
    p(-19, -7),
    p(-29, -7),
    p(-1, -1),
    p(-27, -17),
    p(-18, -3),
    p(-31, 1),
    p(-1, -8),
    p(-22, -8),
    p(-29, -14),
    p(-39, -4),
    p(-8, -3),
    p(-48, -6),
    p(0, 15),
    p(-61, 60),
    p(2, -2),
    p(-14, -3),
    p(-32, 57),
    p(0, 0),
    p(-17, -5),
    p(-24, 5),
    p(0, 0),
    p(0, 0),
    p(-16, -0),
    p(-41, 9),
    p(-36, -42),
    p(0, 0),
    p(13, -63),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-11, 10),  /*0b0000*/
    p(-12, 7),   /*0b0001*/
    p(-3, 12),   /*0b0010*/
    p(-2, 11),   /*0b0011*/
    p(-7, 3),    /*0b0100*/
    p(-22, 0),   /*0b0101*/
    p(-9, 5),    /*0b0110*/
    p(-11, -11), /*0b0111*/
    p(2, 6),     /*0b1000*/
    p(-7, 10),   /*0b1001*/
    p(0, 11),    /*0b1010*/
    p(4, 11),    /*0b1011*/
    p(-3, 4),    /*0b1100*/
    p(-17, 4),   /*0b1101*/
    p(-8, 4),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(3, 10),    /*0b10000*/
    p(5, 7),     /*0b10001*/
    p(21, 8),    /*0b10010*/
    p(-0, 7),    /*0b10011*/
    p(-7, 4),    /*0b10100*/
    p(13, 13),   /*0b10101*/
    p(-28, 3),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(9, 10),    /*0b11000*/
    p(26, 12),   /*0b11001*/
    p(28, 26),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(8, -2),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(8, 3),     /*0b100000*/
    p(1, 8),     /*0b100001*/
    p(15, 4),    /*0b100010*/
    p(6, -1),    /*0b100011*/
    p(-5, -2),   /*0b100100*/
    p(-19, -12), /*0b100101*/
    p(-19, 18),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(16, -1),   /*0b101000*/
    p(0, 8),     /*0b101001*/
    p(11, -4),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(1, -0),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(4, 3),     /*0b110000*/
    p(14, 4),    /*0b110001*/
    p(16, -4),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(10, 14),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(20, -5),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(5, -4),    /*0b111111*/
    p(14, 2),    /*0b00*/
    p(22, -8),   /*0b01*/
    p(39, -8),   /*0b10*/
    p(13, -29),  /*0b11*/
    p(43, -3),   /*0b100*/
    p(14, -8),   /*0b101*/
    p(50, -28),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(57, -10),  /*0b1000*/
    p(13, -17),  /*0b1001*/
    p(31, -35),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(23, -17),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-17, 27),  /*0b1111*/
    p(22, 3),    /*0b00*/
    p(33, -10),  /*0b01*/
    p(29, -13),  /*0b10*/
    p(28, -36),  /*0b11*/
    p(32, -10),  /*0b100*/
    p(45, -18),  /*0b101*/
    p(20, -18),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(35, -2),   /*0b1000*/
    p(43, -14),  /*0b1001*/
    p(47, -43),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(30, -24),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(14, -41),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(-1, -37);
const STOPPABLE_PASSER: PhasedScore = p(25, -48);
const CLOSE_KING_PASSER: PhasedScore = p(3, 24);
const IMMOBILE_PASSER: PhasedScore = p(-5, -36);
const PROTECTED_PASSER: PhasedScore = p(8, -3);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -31,   46),    p( -39,   66),    p( -54,   61),    p( -59,   44),    p( -60,   46),    p( -48,   42),    p( -31,   43),    p( -57,   46),
        p( -24,   43),    p( -45,   67),    p( -50,   58),    p( -51,   44),    p( -68,   53),    p( -62,   50),    p( -49,   56),    p( -49,   44),
        p( -18,   55),    p( -27,   55),    p( -46,   59),    p( -43,   58),    p( -54,   63),    p( -49,   63),    p( -50,   71),    p( -70,   70),
        p(  -8,   71),    p( -11,   69),    p( -11,   60),    p( -29,   72),    p( -45,   80),    p( -36,   85),    p( -40,   91),    p( -61,   92),
        p(  -5,   62),    p(  12,   52),    p(  -2,   44),    p( -16,   38),    p( -16,   55),    p( -32,   70),    p( -52,   81),    p( -81,   91),
        p(  15,   61),    p(  14,   60),    p(  20,   54),    p(  23,   48),    p(  21,   55),    p(  22,   69),    p( -14,   76),    p(  -6,   75),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(1, 4), p(5, 8), p(8, 17), p(15, 22), p(17, 71), p(13, 64)];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -6);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);
const PHALANX: [PhasedScore; 6] = [p(-1, 0), p(5, 3), p(9, 6), p(21, 20), p(58, 74), p(-81, 216)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 14), p(7, 18), p(14, 21), p(7, 10), p(-3, 13), p(-48, 12)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(48, 21), p(51, 45), p(66, 4), p(51, -8), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(1, -5), p(14, 20), p(19, -7), p(16, 10), p(16, -9), p(29, -12)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-41, -69),
        p(-20, -29),
        p(-8, -5),
        p(1, 8),
        p(8, 19),
        p(15, 29),
        p(23, 32),
        p(29, 35),
        p(33, 33),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-6, -22),
        p(2, -9),
        p(9, 1),
        p(14, 10),
        p(19, 14),
        p(24, 19),
        p(26, 24),
        p(33, 26),
        p(38, 26),
        p(44, 29),
        p(39, 40),
        p(52, 32),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-63, 30),
        p(-59, 37),
        p(-55, 42),
        p(-56, 49),
        p(-50, 55),
        p(-48, 60),
        p(-44, 65),
        p(-41, 70),
        p(-38, 74),
        p(-33, 77),
        p(-32, 82),
        p(-24, 83),
        p(-16, 82),
        p(-12, 78),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-19, -22),
        p(-19, 11),
        p(-24, 68),
        p(-20, 84),
        p(-18, 103),
        p(-13, 108),
        p(-10, 120),
        p(-7, 126),
        p(-3, 132),
        p(0, 134),
        p(3, 138),
        p(8, 141),
        p(11, 142),
        p(13, 147),
        p(15, 149),
        p(20, 152),
        p(22, 158),
        p(26, 159),
        p(36, 156),
        p(51, 149),
        p(57, 149),
        p(98, 127),
        p(98, 130),
        p(127, 107),
        p(215, 76),
        p(257, 38),
        p(289, 26),
        p(270, 22),
    ],
    [
        p(-86, -1),
        p(-57, -11),
        p(-29, -9),
        p(0, -5),
        p(31, -3),
        p(54, -1),
        p(83, 5),
        p(108, 8),
        p(155, -1),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-4, 14), p(0, 0), p(28, 26), p(60, 7), p(39, 0), p(0, 0)],
    [p(-2, 12), p(21, 25), p(0, 0), p(42, 22), p(46, 92), p(0, 0)],
    [p(-4, 19), p(12, 18), p(19, 14), p(0, 0), p(66, 45), p(0, 0)],
    [p(-2, 9), p(2, 9), p(1, 25), p(-0, 13), p(0, 0), p(0, 0)],
    [p(64, 21), p(-20, 23), p(8, 17), p(-13, 23), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 9), p(8, 7), p(6, 11), p(12, 8), p(8, 14), p(3, 8)],
    [p(2, 9), p(11, 22), p(-1, -44), p(9, 12), p(10, 20), p(4, 7)],
    [p(2, 4), p(13, 9), p(9, 14), p(11, 10), p(9, 29), p(19, -3)],
    [p(2, 3), p(8, 3), p(7, -2), p(5, 13), p(-63, -221), p(5, -10)],
    [p(48, 0), p(36, 12), p(41, 6), p(26, 6), p(37, -9), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-20, -14), p(17, -9), p(9, -3), p(16, -16), p(-3, 4), p(-7, 3)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(8, 2), p(-5, 7), p(16, -8), p(-10, 24)];
const CHECK_STM: PhasedScore = p(38, 22);
const DISCOVERED_CHECK_STM: PhasedScore = p(176, 59);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(11, -7), p(62, 1), p(101, -33), p(55, 89), p(0, 0), p(-26, -23)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(5, -17), p(26, 30), p(16, 34), p(43, 9), p(62, 3)];

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
