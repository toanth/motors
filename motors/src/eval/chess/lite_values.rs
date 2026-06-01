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
use gears::games::chess::Color;
use gears::games::chess::Color::White;
use gears::games::chess::pieces::{NUM_CHESS_PIECES, PieceType};
use gears::games::chess::squares::{NUM_SQUARES, Square};
use gears::general::common::StaticallyNamedEntity;
use gears::score::{PhasedScore, p};
use std::fmt::{Debug, Display};

#[rustfmt::skip]
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
        p( 114,  161),    p( 114,  160),    p( 120,  154),    p( 123,  148),    p( 120,  155),    p( 122,  169),    p(  86,  176),    p(  94,  175),
        p(  69,  119),    p(  71,  120),    p(  85,  111),    p(  85,  108),    p(  81,  103),    p( 129,  105),    p( 115,  120),    p( 118,  113),
        p(  51,  102),    p(  61,   98),    p(  59,   92),    p(  86,   95),    p(  89,   93),    p(  87,   85),    p(  88,   94),    p(  90,   92),
        p(  45,   89),    p(  49,   92),    p(  71,   90),    p(  87,   91),    p(  92,   90),    p(  89,   89),    p(  80,   88),    p(  76,   83),
        p(  35,   89),    p(  46,   85),    p(  67,   91),    p(  79,   93),    p(  79,   93),    p(  81,   92),    p(  84,   77),    p(  72,   80),
        p(  46,   96),    p(  54,   92),    p(  60,   93),    p(  58,   98),    p(  64,  101),    p(  90,   90),    p( 107,   78),    p(  82,   80),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 187,  282),    p( 189,  323),    p( 209,  322),    p( 242,  319),    p( 286,  306),    p( 208,  313),    p( 231,  296),    p( 197,  257),
        p( 275,  315),    p( 286,  321),    p( 297,  310),    p( 300,  313),    p( 305,  307),    p( 313,  300),    p( 294,  312),    p( 279,  302),
        p( 287,  312),    p( 304,  307),    p( 309,  312),    p( 319,  313),    p( 331,  310),    p( 351,  297),    p( 295,  306),    p( 296,  306),
        p( 307,  318),    p( 314,  313),    p( 328,  315),    p( 328,  323),    p( 327,  320),    p( 330,  317),    p( 319,  318),    p( 329,  310),
        p( 300,  318),    p( 312,  309),    p( 315,  316),    p( 323,  318),    p( 321,  320),    p( 328,  307),    p( 327,  306),    p( 316,  316),
        p( 278,  305),    p( 286,  303),    p( 295,  299),    p( 304,  311),    p( 308,  310),    p( 306,  289),    p( 306,  297),    p( 300,  306),
        p( 271,  312),    p( 284,  315),    p( 287,  308),    p( 296,  310),    p( 304,  305),    p( 298,  301),    p( 301,  310),    p( 296,  319),
        p( 242,  308),    p( 280,  307),    p( 269,  307),    p( 290,  312),    p( 297,  312),    p( 297,  302),    p( 287,  309),    p( 271,  311),
    ],
    // bishop
    [
        p( 276,  319),    p( 263,  312),    p( 230,  309),    p( 226,  318),    p( 214,  316),    p( 235,  310),    p( 261,  312),    p( 260,  308),
        p( 290,  306),    p( 288,  306),    p( 296,  306),    p( 281,  307),    p( 289,  302),    p( 297,  302),    p( 258,  315),    p( 280,  305),
        p( 300,  310),    p( 306,  306),    p( 292,  309),    p( 306,  299),    p( 305,  305),    p( 335,  304),    p( 319,  306),    p( 315,  315),
        p( 289,  311),    p( 300,  306),    p( 310,  301),    p( 310,  308),    p( 310,  305),    p( 311,  305),    p( 314,  303),    p( 292,  307),
        p( 290,  307),    p( 289,  307),    p( 296,  307),    p( 311,  303),    p( 307,  303),    p( 308,  298),    p( 294,  303),    p( 316,  296),
        p( 295,  306),    p( 299,  305),    p( 299,  307),    p( 299,  306),    p( 307,  305),    p( 302,  299),    p( 309,  294),    p( 312,  301),
        p( 309,  303),    p( 299,  302),    p( 306,  302),    p( 299,  310),    p( 301,  307),    p( 308,  305),    p( 316,  296),    p( 312,  302),
        p( 298,  305),    p( 311,  305),    p( 305,  310),    p( 293,  310),    p( 306,  309),    p( 292,  312),    p( 305,  307),    p( 306,  298),
    ],
    // rook
    [
        p( 447,  562),    p( 443,  565),    p( 431,  572),    p( 428,  571),    p( 441,  565),    p( 462,  565),    p( 451,  567),    p( 490,  546),
        p( 453,  562),    p( 451,  565),    p( 457,  566),    p( 472,  557),    p( 459,  560),    p( 488,  554),    p( 491,  554),    p( 502,  541),
        p( 453,  556),    p( 469,  550),    p( 463,  553),    p( 462,  547),    p( 487,  539),    p( 505,  534),    p( 512,  538),    p( 487,  539),
        p( 450,  556),    p( 459,  552),    p( 458,  553),    p( 461,  548),    p( 468,  544),    p( 483,  541),    p( 478,  546),    p( 475,  541),
        p( 439,  553),    p( 441,  551),    p( 442,  553),    p( 449,  549),    p( 457,  546),    p( 454,  546),    p( 469,  543),    p( 455,  541),
        p( 435,  549),    p( 439,  545),    p( 438,  547),    p( 440,  547),    p( 450,  540),    p( 460,  535),    p( 474,  529),    p( 461,  532),
        p( 435,  546),    p( 440,  545),    p( 446,  545),    p( 450,  541),    p( 457,  538),    p( 470,  530),    p( 481,  525),    p( 447,  537),
        p( 442,  551),    p( 442,  544),    p( 441,  548),    p( 446,  543),    p( 452,  537),    p( 453,  541),    p( 441,  547),    p( 439,  541),
    ],
    // queen
    [
        p( 869,  989),    p( 870,  994),    p( 884, 1004),    p( 905,  994),    p( 910, 1000),    p( 936,  988),    p( 970,  942),    p( 907,  970),
        p( 902,  975),    p( 888,  991),    p( 887, 1008),    p( 879, 1030),    p( 878, 1045),    p( 919, 1018),    p( 913,  999),    p( 957,  970),
        p( 913,  970),    p( 907,  979),    p( 903, 1002),    p( 899, 1015),    p( 901, 1026),    p( 942, 1016),    p( 955,  987),    p( 940,  989),
        p( 901,  978),    p( 903,  987),    p( 900,  992),    p( 891, 1015),    p( 894, 1023),    p( 913, 1014),    p( 917, 1018),    p( 926,  993),
        p( 896,  978),    p( 893,  984),    p( 893,  990),    p( 894, 1005),    p( 899, 1008),    p( 902, 1005),    p( 911, 1001),    p( 915,  988),
        p( 896,  963),    p( 900,  973),    p( 894,  988),    p( 892,  992),    p( 895, 1003),    p( 906,  991),    p( 913,  984),    p( 913,  963),
        p( 898,  956),    p( 895,  971),    p( 899,  975),    p( 899,  988),    p( 900,  991),    p( 903,  972),    p( 912,  956),    p( 905,  947),
        p( 878,  971),    p( 892,  959),    p( 892,  967),    p( 897,  972),    p( 898,  962),    p( 884,  974),    p( 881,  953),    p( 892,  926),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  70,    3),    p(  83,   -0),    p( 103,  -13),    p( 193,  -69),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   7,   24),    p( -34,   36),    p( -24,   27),    p(  -4,   14),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -46,   36),    p( -26,   28),    p( -33,   21),    p( -34,   20),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-116,   33),    p( -91,   25),    p( -88,   13),    p( -72,   14),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-133,   27),    p(-106,   14),    p(-107,    3),    p( -94,    7),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-110,   15),    p(-108,    8),    p( -81,   -5),    p( -57,    6),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -98,    6),    p( -86,   -0),    p( -57,  -14),    p(   2,   -4),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  22,  -16),    p(   7,  -11),    p(  30,  -24),    p(  59,  -26),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 52);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(13, 21), p(13, 18), p(13, 8), p(9, 1), p(4, -6), p(0, -15), p(-6, -23), p(-12, -36), p(-23, -40)];
const ROOK_OPEN_FILE: PhasedScore = p(14, 1);
const ROOK_CLOSED_FILE: PhasedScore = p(-11, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(6, -3);
const KING_OPEN_FILE: PhasedScore = p(-37, 6);
const KING_CLOSED_FILE: PhasedScore = p(12, -6);
const KING_SEMIOPEN_FILE: PhasedScore = p(-3, 11);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-3, 6), p(-0, 9), p(1, 7), p(4, 6), p(4, 7), p(4, 8), p(9, 5), p(20, 1)],
    // Closed
    [p(0, 0), p(0, 0), p(18, -16), p(-14, 13), p(2, 12), p(1, 6), p(1, 8), p(1, 5)],
    // SemiOpen
    [p(0, 0), p(-6, 28), p(9, 21), p(2, 13), p(2, 13), p(4, 7), p(4, 4), p(11, 6)],
    // SemiClosed
    [p(0, 0), p(11, -11), p(10, 7), p(4, 2), p(8, 4), p(2, 5), p(5, 5), p(4, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(22, 8),
    p(4, 3),
    p(2, 5),
    p(-7, 7),
    p(6, 3),
    p(-7, -9),
    p(-3, 1),
    p(-4, -8),
    p(1, -2),
    p(-10, -2),
    p(-7, -15),
    p(-15, -5),
    p(6, -5),
    p(-1, -8),
    p(7, -3),
    p(6, 17),
    p(-6, -2),
    p(-22, -4),
    p(-13, 2),
    p(-38, 23),
    p(-20, 5),
    p(-19, -16),
    p(3, 28),
    p(-45, 30),
    p(-18, -16),
    p(-21, -14),
    p(-34, -27),
    p(-39, 16),
    p(-17, 2),
    p(13, -2),
    p(-84, 100),
    p(0, 0),
    p(1, -5),
    p(-13, -6),
    p(-2, -5),
    p(-21, 1),
    p(-21, -1),
    p(-44, -18),
    p(-26, 39),
    p(-32, 30),
    p(-6, -5),
    p(-19, -9),
    p(7, -9),
    p(-16, 36),
    p(-48, 21),
    p(-0, -27),
    p(0, 0),
    p(0, 0),
    p(-4, -11),
    p(-16, 8),
    p(-7, -52),
    p(0, 0),
    p(6, -8),
    p(-37, -12),
    p(0, 0),
    p(0, 0),
    p(-29, -4),
    p(-23, -10),
    p(-19, 21),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(25, 5),
    p(4, -1),
    p(-5, 4),
    p(-21, -0),
    p(11, -4),
    p(-23, -8),
    p(-15, -0),
    p(-34, -9),
    p(8, -2),
    p(-9, -8),
    p(-28, -5),
    p(-43, 1),
    p(-1, -5),
    p(-37, -5),
    p(-31, -6),
    p(-47, 61),
    p(10, 0),
    p(-0, -7),
    p(-4, -8),
    p(-21, -1),
    p(-9, -1),
    p(-14, -11),
    p(-22, 1),
    p(-66, 180),
    p(-4, -10),
    p(-24, -14),
    p(-31, -26),
    p(11, -79),
    p(-11, -8),
    p(-10, -18),
    p(-77, 70),
    p(0, 0),
    p(12, 0),
    p(-2, -3),
    p(-17, -5),
    p(-26, -5),
    p(0, 1),
    p(-25, -15),
    p(-15, -0),
    p(-27, 3),
    p(-1, -7),
    p(-21, -7),
    p(-27, -12),
    p(-36, -3),
    p(-6, -1),
    p(-45, -5),
    p(4, 17),
    p(-57, 62),
    p(4, 1),
    p(-11, -1),
    p(-28, 60),
    p(0, 0),
    p(-13, -2),
    p(-19, 7),
    p(0, 0),
    p(0, 0),
    p(-13, 2),
    p(-38, 10),
    p(-31, -39),
    p(0, 0),
    p(18, -60),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-13, 9),   /*0b0000*/
    p(-13, 7),   /*0b0001*/
    p(-5, 11),   /*0b0010*/
    p(-4, 10),   /*0b0011*/
    p(-9, 3),    /*0b0100*/
    p(-24, -1),  /*0b0101*/
    p(-11, 4),   /*0b0110*/
    p(-13, -11), /*0b0111*/
    p(0, 5),     /*0b1000*/
    p(-9, 9),    /*0b1001*/
    p(-2, 10),   /*0b1010*/
    p(2, 11),    /*0b1011*/
    p(-5, 3),    /*0b1100*/
    p(-19, 3),   /*0b1101*/
    p(-10, 3),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(1, 9),     /*0b10000*/
    p(3, 6),     /*0b10001*/
    p(19, 7),    /*0b10010*/
    p(-2, 6),    /*0b10011*/
    p(-9, 3),    /*0b10100*/
    p(11, 12),   /*0b10101*/
    p(-30, 2),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(7, 10),    /*0b11000*/
    p(24, 11),   /*0b11001*/
    p(26, 25),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(6, -3),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(6, 3),     /*0b100000*/
    p(-1, 7),    /*0b100001*/
    p(13, 3),    /*0b100010*/
    p(5, -1),    /*0b100011*/
    p(-6, -3),   /*0b100100*/
    p(-21, -13), /*0b100101*/
    p(-21, 18),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(15, -2),   /*0b101000*/
    p(-1, 8),    /*0b101001*/
    p(9, -5),    /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-1, -1),   /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(2, 2),     /*0b110000*/
    p(12, 4),    /*0b110001*/
    p(14, -5),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(9, 13),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(18, -6),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(3, -5),    /*0b111111*/
    p(13, -1),   /*0b00*/
    p(21, -11),  /*0b01*/
    p(37, -10),  /*0b10*/
    p(12, -31),  /*0b11*/
    p(41, -5),   /*0b100*/
    p(13, -10),  /*0b101*/
    p(49, -30),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(56, -13),  /*0b1000*/
    p(11, -20),  /*0b1001*/
    p(29, -37),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(21, -20),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-18, 25),  /*0b1111*/
    p(21, 0),    /*0b00*/
    p(31, -12),  /*0b01*/
    p(28, -15),  /*0b10*/
    p(26, -39),  /*0b11*/
    p(31, -12),  /*0b100*/
    p(43, -20),  /*0b101*/
    p(19, -20),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(34, -4),   /*0b1000*/
    p(41, -16),  /*0b1001*/
    p(46, -45),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(28, -26),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(13, -44),  /*0b1111*/
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
        p(  -5,   62),    p(  13,   52),    p(  -2,   44),    p( -16,   38),    p( -16,   55),    p( -32,   70),    p( -52,   80),    p( -81,   91),
        p(  14,   61),    p(  14,   60),    p(  20,   54),    p(  23,   48),    p(  20,   55),    p(  22,   69),    p( -14,   76),    p(  -6,   75),
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
        p(-41, -67),
        p(-20, -27),
        p(-8, -4),
        p(1, 10),
        p(9, 20),
        p(16, 31),
        p(24, 34),
        p(30, 36),
        p(34, 35),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-31, -56),
        p(-18, -37),
        p(-6, -21),
        p(1, -9),
        p(8, 1),
        p(13, 10),
        p(18, 15),
        p(23, 19),
        p(25, 25),
        p(32, 26),
        p(38, 26),
        p(43, 30),
        p(38, 41),
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
        p(-68, 14),
        p(-59, 28),
        p(-55, 35),
        p(-51, 39),
        p(-52, 46),
        p(-46, 53),
        p(-44, 58),
        p(-40, 62),
        p(-37, 67),
        p(-34, 71),
        p(-29, 74),
        p(-28, 79),
        p(-21, 81),
        p(-12, 79),
        p(-8, 76),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-19, -32),
        p(-19, 1),
        p(-24, 58),
        p(-19, 74),
        p(-18, 93),
        p(-13, 98),
        p(-10, 110),
        p(-6, 116),
        p(-3, 122),
        p(1, 124),
        p(4, 128),
        p(8, 131),
        p(11, 132),
        p(13, 137),
        p(16, 139),
        p(20, 142),
        p(22, 148),
        p(26, 149),
        p(36, 146),
        p(51, 139),
        p(57, 139),
        p(99, 117),
        p(98, 120),
        p(127, 97),
        p(216, 66),
        p(257, 28),
        p(289, 16),
        p(271, 12),
    ],
    [
        p(-89, -0),
        p(-59, -10),
        p(-31, -9),
        p(-2, -5),
        p(28, -3),
        p(51, -1),
        p(80, 5),
        p(105, 8),
        p(152, -1),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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

    fn psqt(&self, square: Square, piece: PieceType, color: Color) -> SingleFeatureScore<Self::Score>;

    fn passed_pawn(square: Square) -> SingleFeatureScore<Self::Score>;

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

    fn pawn_shield(&self, color: Color, config: usize) -> SingleFeatureScore<Self::Score>;

    fn pawnless_flank() -> SingleFeatureScore<Self::Score>;

    fn pawn_protection(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn pawn_attack(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn pawn_advance_threat(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn mobility(piece: PieceType, mobility: usize) -> SingleFeatureScore<Self::Score>;

    fn threats(attacking: PieceType, targeted: PieceType) -> SingleFeatureScore<Self::Score>;

    fn defended(protecting: PieceType, target: PieceType) -> SingleFeatureScore<Self::Score>;

    fn king_zone_attack(attacking: PieceType) -> SingleFeatureScore<Self::Score>;

    fn can_give_check(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn check_stm() -> SingleFeatureScore<Self::Score>;

    fn discovered_check_stm() -> SingleFeatureScore<Self::Score>;

    fn discovered_check(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn pin(piece: PieceType) -> SingleFeatureScore<Self::Score>;
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

    fn psqt(&self, square: Square, piece: PieceType, color: Color) -> Self::Score {
        PSQTS[piece as usize][square.flip_if(color == White).bb_idx()]
    }

    fn passed_pawn(square: Square) -> PhasedScore {
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

    fn pawn_shield(&self, _color: Color, config: usize) -> PhasedScore {
        PAWN_SHIELDS[config]
    }

    fn pawnless_flank() -> PhasedScore {
        PAWNLESS_FLANK
    }

    fn pawn_protection(piece: PieceType) -> PhasedScore {
        PAWN_PROTECTION[piece as usize]
    }

    fn pawn_attack(piece: PieceType) -> PhasedScore {
        PAWN_ATTACKS[piece as usize]
    }

    fn pawn_advance_threat(piece: PieceType) -> PhasedScore {
        PAWN_ADVANCE_THREAT[piece as usize]
    }

    fn mobility(piece: PieceType, mobility: usize) -> PhasedScore {
        MOBILITY[piece as usize - 1][mobility]
    }

    fn threats(attacking: PieceType, targeted: PieceType) -> PhasedScore {
        THREATS[attacking as usize - 1][targeted as usize]
    }
    fn defended(protecting: PieceType, target: PieceType) -> PhasedScore {
        DEFENDED[protecting as usize - 1][target as usize]
    }

    fn king_zone_attack(attacking: PieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }

    fn can_give_check(piece: PieceType) -> PhasedScore {
        CAN_GIVE_CHECK[piece as usize]
    }

    fn discovered_check_stm() -> PhasedScore {
        DISCOVERED_CHECK_STM
    }

    fn pin(piece: PieceType) -> PhasedScore {
        PIN[piece as usize]
    }

    fn discovered_check(piece: PieceType) -> PhasedScore {
        DISCOVERED_CHECK[piece as usize]
    }

    fn check_stm() -> PhasedScore {
        CHECK_STM
    }
}
