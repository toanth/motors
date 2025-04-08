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
        p( 136,  189),    p( 131,  188),    p( 124,  191),    p( 135,  175),    p( 121,  179),    p( 120,  183),    p(  80,  199),    p(  83,  199),
        p(  70,  121),    p(  71,  123),    p(  79,  115),    p(  88,  117),    p(  77,  116),    p( 125,  107),    p( 101,  127),    p(  96,  119),
        p(  53,  111),    p(  64,  104),    p(  63,  100),    p(  84,   99),    p(  92,   98),    p(  85,   89),    p(  77,   99),    p(  72,   95),
        p(  49,   98),    p(  54,  100),    p(  78,   93),    p(  93,   96),    p(  91,   97),    p(  87,   94),    p(  70,   90),    p(  60,   85),
        p(  42,   96),    p(  50,   91),    p(  71,   96),    p(  82,   97),    p(  84,   95),    p(  77,   94),    p(  69,   82),    p(  52,   84),
        p(  53,   99),    p(  58,   97),    p(  62,   97),    p(  59,  104),    p(  61,  106),    p(  76,   98),    p(  81,   86),    p(  59,   89),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 174,  280),    p( 196,  312),    p( 213,  325),    p( 252,  314),    p( 281,  315),    p( 199,  310),    p( 216,  309),    p( 201,  264),
        p( 267,  313),    p( 283,  317),    p( 299,  310),    p( 304,  314),    p( 303,  310),    p( 315,  299),    p( 274,  316),    p( 271,  306),
        p( 287,  309),    p( 307,  304),    p( 309,  311),    p( 323,  315),    p( 339,  309),    p( 353,  297),    p( 294,  304),    p( 286,  308),
        p( 302,  315),    p( 310,  309),    p( 326,  313),    p( 327,  321),    p( 326,  318),    p( 321,  317),    p( 311,  312),    p( 320,  310),
        p( 299,  317),    p( 305,  306),    p( 314,  312),    p( 321,  314),    p( 320,  318),    p( 325,  303),    p( 323,  303),    p( 312,  312),
        p( 276,  303),    p( 283,  302),    p( 297,  297),    p( 301,  309),    p( 306,  307),    p( 296,  291),    p( 302,  293),    p( 293,  307),
        p( 270,  310),    p( 281,  314),    p( 286,  304),    p( 295,  309),    p( 299,  304),    p( 290,  302),    p( 295,  307),    p( 289,  321),
        p( 241,  308),    p( 282,  305),    p( 267,  307),    p( 287,  311),    p( 296,  309),    p( 292,  299),    p( 288,  308),    p( 264,  309),
    ],
    // bishop
    [
        p( 276,  311),    p( 251,  315),    p( 238,  308),    p( 221,  318),    p( 215,  316),    p( 224,  309),    p( 272,  305),    p( 249,  310),
        p( 283,  303),    p( 279,  304),    p( 290,  307),    p( 276,  306),    p( 288,  303),    p( 291,  301),    p( 267,  309),    p( 271,  304),
        p( 296,  309),    p( 307,  305),    p( 291,  305),    p( 306,  300),    p( 305,  302),    p( 336,  305),    p( 317,  302),    p( 318,  313),
        p( 287,  312),    p( 292,  307),    p( 303,  303),    p( 306,  307),    p( 306,  304),    p( 299,  305),    p( 298,  309),    p( 280,  311),
        p( 290,  306),    p( 284,  309),    p( 295,  304),    p( 308,  304),    p( 302,  301),    p( 299,  303),    p( 285,  305),    p( 309,  300),
        p( 296,  308),    p( 299,  304),    p( 299,  306),    p( 299,  304),    p( 305,  306),    p( 297,  300),    p( 305,  295),    p( 307,  298),
        p( 309,  306),    p( 304,  300),    p( 308,  301),    p( 299,  309),    p( 301,  306),    p( 302,  305),    p( 311,  296),    p( 308,  295),
        p( 299,  304),    p( 309,  306),    p( 307,  307),    p( 290,  311),    p( 306,  309),    p( 294,  310),    p( 302,  299),    p( 302,  292),
    ],
    // rook
    [
        p( 462,  545),    p( 450,  555),    p( 443,  563),    p( 441,  560),    p( 452,  556),    p( 474,  551),    p( 483,  549),    p( 493,  542),
        p( 444,  551),    p( 442,  557),    p( 451,  558),    p( 466,  549),    p( 452,  552),    p( 468,  546),    p( 477,  544),    p( 492,  532),
        p( 444,  547),    p( 464,  542),    p( 458,  544),    p( 458,  539),    p( 484,  529),    p( 495,  525),    p( 511,  524),    p( 485,  526),
        p( 441,  547),    p( 448,  543),    p( 447,  546),    p( 453,  540),    p( 457,  532),    p( 469,  527),    p( 469,  531),    p( 468,  527),
        p( 435,  545),    p( 434,  543),    p( 435,  543),    p( 440,  540),    p( 448,  535),    p( 442,  535),    p( 455,  530),    p( 449,  528),
        p( 430,  543),    p( 431,  540),    p( 432,  538),    p( 435,  537),    p( 441,  532),    p( 452,  525),    p( 468,  515),    p( 455,  518),
        p( 433,  539),    p( 437,  537),    p( 443,  538),    p( 446,  534),    p( 453,  528),    p( 465,  519),    p( 473,  515),    p( 444,  524),
        p( 443,  543),    p( 439,  539),    p( 440,  542),    p( 445,  536),    p( 451,  529),    p( 457,  530),    p( 453,  529),    p( 448,  532),
    ],
    // queen
    [
        p( 877,  963),    p( 879,  977),    p( 895,  989),    p( 916,  982),    p( 913,  988),    p( 934,  975),    p( 979,  928),    p( 924,  960),
        p( 888,  948),    p( 864,  980),    p( 864, 1007),    p( 858, 1024),    p( 864, 1036),    p( 904,  997),    p( 906,  982),    p( 946,  961),
        p( 893,  953),    p( 886,  969),    p( 885,  991),    p( 885, 1003),    p( 907, 1005),    p( 946,  989),    p( 953,  960),    p( 940,  967),
        p( 880,  966),    p( 885,  974),    p( 878,  985),    p( 879,  997),    p( 881, 1011),    p( 895, 1001),    p( 905, 1002),    p( 912,  978),
        p( 890,  959),    p( 877,  978),    p( 883,  978),    p( 883,  994),    p( 887,  992),    p( 888,  993),    p( 901,  983),    p( 908,  975),
        p( 886,  949),    p( 892,  965),    p( 886,  979),    p( 883,  980),    p( 888,  989),    p( 895,  979),    p( 909,  962),    p( 907,  951),
        p( 886,  952),    p( 886,  960),    p( 893,  962),    p( 892,  976),    p( 894,  974),    p( 895,  958),    p( 906,  937),    p( 914,  913),
        p( 872,  954),    p( 884,  942),    p( 885,  954),    p( 893,  956),    p( 896,  944),    p( 883,  949),    p( 884,  943),    p( 889,  926),
    ],
    // king
    [
        p( 153,  -69),    p(  65,  -23),    p(  81,  -12),    p(  15,   15),    p(  41,    3),    p(  23,   16),    p(  84,    4),    p( 217,  -73),
        p( -32,   16),    p( -60,   29),    p( -60,   35),    p(   4,   25),    p( -32,   35),    p( -50,   48),    p( -26,   36),    p(  10,   16),
        p( -52,   21),    p( -41,   20),    p( -79,   32),    p( -87,   42),    p( -53,   38),    p( -21,   31),    p( -59,   32),    p( -31,   20),
        p( -31,    7),    p( -96,   14),    p(-112,   28),    p(-133,   37),    p(-132,   36),    p(-110,   29),    p(-122,   21),    p(-103,   21),
        p( -42,   -3),    p(-105,    3),    p(-120,   19),    p(-145,   32),    p(-143,   30),    p(-118,   18),    p(-132,   11),    p(-116,   13),
        p( -31,   -0),    p( -79,   -2),    p(-107,   11),    p(-116,   20),    p(-113,   20),    p(-122,   13),    p( -98,    3),    p( -70,   10),
        p(  27,   -8),    p( -67,   -6),    p( -77,    1),    p( -97,   10),    p(-103,   12),    p( -89,    5),    p( -63,   -9),    p(   2,   -1),
        p(  48,  -20),    p(  41,  -31),    p(  39,  -19),    p( -21,   -2),    p(  29,  -17),    p( -18,   -3),    p(  33,  -24),    p(  59,  -30),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 52);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(9, 19), p(10, 18), p(10, 7), p(6, -0), p(2, -8), p(-1, -17), p(-7, -26), p(-14, -39), p(-25, -49)];
const ROOK_OPEN_FILE: PhasedScore = p(14, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, 0);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 1);
const KING_OPEN_FILE: PhasedScore = p(-50, -1);
const KING_CLOSED_FILE: PhasedScore = p(12, -9);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 6);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-5, 4), p(1, 6), p(-1, 5), p(2, 3), p(2, 5), p(4, 7), p(7, 5), p(19, 2)],
    // Closed
    [p(0, 0), p(0, 0), p(21, -30), p(-14, 9), p(-1, 11), p(-0, 4), p(-2, 7), p(-1, 4)],
    // SemiOpen
    [p(0, 0), p(-14, 24), p(3, 17), p(-0, 10), p(-1, 10), p(4, 6), p(0, 3), p(10, 5)],
    // SemiClosed
    [p(0, 0), p(12, -13), p(7, 6), p(3, -0), p(6, 1), p(1, 4), p(3, 5), p(2, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(22, 6),
    p(2, 5),
    p(1, 4),
    p(-7, 12),
    p(6, 1),
    p(-10, -7),
    p(1, 2),
    p(-4, -5),
    p(1, -2),
    p(-11, 2),
    p(-9, -14),
    p(-17, 3),
    p(6, -5),
    p(-2, -2),
    p(8, -1),
    p(2, 21),
    p(-3, -3),
    p(-21, 0),
    p(-17, -2),
    p(-48, 22),
    p(-17, 5),
    p(-17, -15),
    p(8, 21),
    p(-57, 29),
    p(-16, -14),
    p(-21, -7),
    p(-39, -30),
    p(-42, 14),
    p(-20, 4),
    p(9, 2),
    p(-96, 116),
    p(0, 0),
    p(1, -2),
    p(-15, 1),
    p(-4, -0),
    p(-27, 11),
    p(-28, -6),
    p(-53, -20),
    p(-36, 39),
    p(-47, 31),
    p(-7, 0),
    p(-22, 2),
    p(5, -5),
    p(-21, 48),
    p(-56, 16),
    p(-14, -28),
    p(0, 0),
    p(0, 0),
    p(10, -8),
    p(-7, 20),
    p(-5, -49),
    p(0, 0),
    p(3, -7),
    p(-42, -2),
    p(0, 0),
    p(0, 0),
    p(-24, 8),
    p(-17, 7),
    p(-13, 25),
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
    p(4, -3),
    p(-14, -7),
    p(-28, -4),
    p(-43, 6),
    p(-9, -1),
    p(-43, 3),
    p(-38, -5),
    p(-54, 70),
    p(10, -4),
    p(-3, -6),
    p(-7, -12),
    p(-29, -6),
    p(-11, 2),
    p(-19, -6),
    p(-26, -1),
    p(-84, 178),
    p(-6, -10),
    p(-28, -9),
    p(-41, -25),
    p(2, -85),
    p(-18, -3),
    p(-20, -10),
    p(-81, 63),
    p(0, 0),
    p(15, -2),
    p(2, -3),
    p(-11, -5),
    p(-20, -6),
    p(-2, 1),
    p(-29, -11),
    p(-17, 1),
    p(-30, 3),
    p(0, -6),
    p(-21, -4),
    p(-25, -13),
    p(-37, -3),
    p(-12, -2),
    p(-49, -9),
    p(-17, 23),
    p(-66, 59),
    p(8, -0),
    p(-8, 1),
    p(-25, 59),
    p(0, 0),
    p(-16, 1),
    p(-22, 3),
    p(0, 0),
    p(0, 0),
    p(-13, 5),
    p(-38, 17),
    p(-34, -41),
    p(0, 0),
    p(5, -54),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-6, 7),    /*0b0000*/
    p(-14, 10),  /*0b0001*/
    p(-4, 10),   /*0b0010*/
    p(-8, 12),   /*0b0011*/
    p(-3, 5),    /*0b0100*/
    p(-27, 4),   /*0b0101*/
    p(-11, 4),   /*0b0110*/
    p(-18, -12), /*0b0111*/
    p(11, 8),    /*0b1000*/
    p(-1, 12),   /*0b1001*/
    p(3, 9),     /*0b1010*/
    p(-1, 10),   /*0b1011*/
    p(1, 6),     /*0b1100*/
    p(-25, 10),  /*0b1101*/
    p(-9, 4),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(8, 11),    /*0b10000*/
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
    p(14, 0),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(19, 7),    /*0b100000*/
    p(6, 12),    /*0b100001*/
    p(24, 2),    /*0b100010*/
    p(8, -0),    /*0b100011*/
    p(-5, 3),    /*0b100100*/
    p(-23, -5),  /*0b100101*/
    p(-22, 16),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(28, 2),    /*0b101000*/
    p(2, 17),    /*0b101001*/
    p(21, -5),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-2, 9),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(17, 4),    /*0b110000*/
    p(23, 5),    /*0b110001*/
    p(29, -3),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(4, 18),    /*0b110100*/
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
    p(-11, -0),  /*0b00*/
    p(3, -11),   /*0b01*/
    p(36, -3),   /*0b10*/
    p(22, -39),  /*0b11*/
    p(44, -8),   /*0b100*/
    p(-3, -14),  /*0b101*/
    p(65, -39),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(65, -10),  /*0b1000*/
    p(15, -28),  /*0b1001*/
    p(73, -48),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(60, -35),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(16, 2),    /*0b1111*/
    p(20, 1),    /*0b00*/
    p(32, -7),   /*0b01*/
    p(25, -14),  /*0b10*/
    p(22, -37),  /*0b11*/
    p(38, -6),   /*0b100*/
    p(52, -15),  /*0b101*/
    p(23, -20),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(40, -2),   /*0b1000*/
    p(51, -16),  /*0b1001*/
    p(52, -41),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(42, -26),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(21, -43),  /*0b1111*/
];
const STOPPABLE_PASSER: PhasedScore = p(37, -48);
const CLOSE_KING_PASSER: PhasedScore = p(-7, 28);
const IMMOBILE_PASSER: PhasedScore = p(-6, -34);
const PROTECTED_PASSER: PhasedScore = p(7, 9);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -26,   34),    p( -40,   49),    p( -48,   48),    p( -43,   36),    p( -31,   23),    p( -33,   30),    p( -25,   39),    p( -31,   40),
        p( -16,   35),    p( -44,   56),    p( -49,   50),    p( -46,   39),    p( -49,   39),    p( -42,   42),    p( -50,   60),    p( -25,   42),
        p(  -6,   61),    p( -19,   63),    p( -41,   65),    p( -37,   60),    p( -45,   58),    p( -34,   62),    p( -47,   77),    p( -40,   74),
        p(  13,   84),    p(   8,   86),    p(  12,   74),    p(  -9,   79),    p( -28,   80),    p( -16,   83),    p( -31,   95),    p( -33,   98),
        p(  30,  131),    p(  36,  129),    p(  29,  113),    p(  13,   92),    p(  13,  100),    p(  -6,  116),    p( -22,  117),    p( -52,  140),
        p(  36,   89),    p(  31,   88),    p(  24,   91),    p(  35,   75),    p(  21,   79),    p(  20,   83),    p( -20,   99),    p( -17,   99),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -9);
const DOUBLED_PAWN: PhasedScore = p(-7, -22);
const PHALANX: [PhasedScore; 6] = [p(-0, -2), p(5, 1), p(9, 5), p(24, 23), p(65, 77), p(-99, 222)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 12), p(7, 15), p(14, 19), p(10, 5), p(-3, 15), p(-48, 11)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(39, 7), p(39, 33), p(51, -12), p(35, -37), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-44, -73),
        p(-25, -32),
        p(-12, -9),
        p(-3, 4),
        p(4, 15),
        p(11, 25),
        p(19, 29),
        p(25, 31),
        p(30, 29),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-31, -54),
        p(-19, -37),
        p(-7, -22),
        p(-0, -9),
        p(7, -0),
        p(12, 8),
        p(17, 13),
        p(21, 17),
        p(23, 22),
        p(30, 23),
        p(34, 22),
        p(42, 25),
        p(38, 33),
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
        p(-50, 52),
        p(-46, 56),
        p(-43, 61),
        p(-39, 65),
        p(-35, 68),
        p(-34, 73),
        p(-27, 74),
        p(-18, 72),
        p(-15, 71),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-29, -41),
        p(-28, 11),
        p(-32, 63),
        p(-28, 81),
        p(-26, 100),
        p(-22, 105),
        p(-18, 116),
        p(-14, 122),
        p(-10, 126),
        p(-7, 127),
        p(-4, 130),
        p(-0, 132),
        p(3, 132),
        p(4, 137),
        p(7, 139),
        p(10, 142),
        p(11, 149),
        p(13, 151),
        p(22, 149),
        p(35, 144),
        p(39, 146),
        p(81, 124),
        p(80, 128),
        p(102, 110),
        p(193, 78),
        p(237, 37),
        p(269, 25),
        p(319, -11),
    ],
    [
        p(-83, 2),
        p(-51, -9),
        p(-26, -8),
        p(1, -5),
        p(29, -3),
        p(49, -1),
        p(76, 4),
        p(98, 6),
        p(141, -6),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-2, 10), p(20, 22), p(0, 0), p(31, 5), p(30, 52), p(0, 0)],
    [p(-3, 14), p(10, 16), p(17, 13), p(0, 0), p(45, -4), p(0, 0)],
    [p(-2, 4), p(2, 6), p(-1, 22), p(1, 2), p(0, 0), p(0, 0)],
    [p(65, 19), p(-35, 18), p(-8, 16), p(-21, 8), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 8), p(8, 7), p(6, 10), p(12, 7), p(6, 19), p(10, 6)],
    [p(2, 7), p(11, 21), p(-146, -15), p(8, 13), p(10, 18), p(4, 6)],
    [p(3, 3), p(13, 8), p(9, 13), p(11, 9), p(10, 24), p(21, -4)],
    [p(2, -1), p(9, 1), p(7, -3), p(4, 14), p(-61, -254), p(5, -11)],
    [p(60, -2), p(37, 8), p(43, 2), p(21, 6), p(33, -10), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-18, -17), p(19, -9), p(10, -3), p(15, -11), p(-1, 12), p(-5, 4)];
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
