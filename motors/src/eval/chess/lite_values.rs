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
        p( 114,  161),    p( 114,  160),    p( 120,  154),    p( 123,  148),    p( 120,  155),    p( 121,  169),    p(  88,  175),    p(  94,  175),
        p(  71,  119),    p(  72,  121),    p(  87,  112),    p(  86,  110),    p(  83,  103),    p( 131,  105),    p( 117,  121),    p( 121,  113),
        p(  52,  103),    p(  62,   99),    p(  61,   93),    p(  85,   98),    p(  89,   95),    p(  89,   85),    p(  90,   94),    p(  91,   92),
        p(  46,   90),    p(  51,   93),    p(  73,   92),    p(  89,   95),    p(  95,   93),    p(  92,   90),    p(  82,   88),    p(  78,   83),
        p(  37,   89),    p(  48,   86),    p(  69,   91),    p(  82,   93),    p(  81,   94),    p(  84,   91),    p(  86,   78),    p(  74,   81),
        p(  47,   96),    p(  55,   93),    p(  62,   94),    p(  60,   99),    p(  65,  102),    p(  93,   91),    p( 108,   78),    p(  83,   81),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 187,  283),    p( 190,  324),    p( 208,  325),    p( 240,  321),    p( 284,  309),    p( 208,  314),    p( 231,  297),    p( 197,  258),
        p( 274,  316),    p( 285,  323),    p( 297,  312),    p( 300,  315),    p( 305,  309),    p( 312,  302),    p( 294,  314),    p( 278,  304),
        p( 285,  314),    p( 301,  308),    p( 307,  314),    p( 320,  315),    p( 332,  312),    p( 348,  298),    p( 292,  308),    p( 291,  308),
        p( 306,  320),    p( 313,  314),    p( 328,  316),    p( 326,  325),    p( 326,  322),    p( 329,  318),    p( 317,  318),    p( 327,  312),
        p( 301,  320),    p( 313,  311),    p( 315,  317),    p( 323,  319),    p( 321,  322),    p( 329,  308),    p( 327,  309),    p( 317,  319),
        p( 282,  309),    p( 289,  306),    p( 297,  302),    p( 305,  312),    p( 309,  311),    p( 310,  294),    p( 311,  301),    p( 305,  310),
        p( 271,  313),    p( 285,  316),    p( 288,  310),    p( 297,  310),    p( 304,  306),    p( 299,  302),    p( 302,  312),    p( 297,  319),
        p( 243,  309),    p( 281,  308),    p( 271,  309),    p( 292,  313),    p( 299,  313),    p( 298,  304),    p( 288,  310),    p( 272,  312),
    ],
    // bishop
    [
        p( 275,  318),    p( 259,  313),    p( 228,  310),    p( 223,  320),    p( 209,  319),    p( 238,  310),    p( 258,  313),    p( 259,  307),
        p( 286,  307),    p( 287,  306),    p( 294,  307),    p( 279,  309),    p( 286,  305),    p( 293,  303),    p( 258,  314),    p( 272,  306),
        p( 297,  309),    p( 301,  306),    p( 288,  310),    p( 304,  300),    p( 304,  306),    p( 329,  305),    p( 315,  306),    p( 311,  314),
        p( 286,  313),    p( 297,  307),    p( 308,  301),    p( 308,  308),    p( 307,  304),    p( 309,  305),    p( 309,  303),    p( 287,  310),
        p( 289,  310),    p( 288,  310),    p( 296,  307),    p( 309,  302),    p( 305,  303),    p( 307,  299),    p( 293,  306),    p( 315,  299),
        p( 299,  309),    p( 301,  308),    p( 298,  309),    p( 299,  306),    p( 307,  305),    p( 304,  303),    p( 312,  298),    p( 316,  304),
        p( 307,  303),    p( 299,  301),    p( 305,  302),    p( 298,  312),    p( 300,  309),    p( 307,  306),    p( 317,  295),    p( 310,  302),
        p( 297,  304),    p( 309,  305),    p( 306,  310),    p( 291,  312),    p( 305,  312),    p( 293,  313),    p( 304,  307),    p( 305,  297),
    ],
    // rook
    [
        p( 451,  560),    p( 447,  563),    p( 435,  570),    p( 431,  569),    p( 446,  563),    p( 465,  563),    p( 454,  565),    p( 494,  544),
        p( 455,  559),    p( 453,  563),    p( 459,  564),    p( 474,  556),    p( 461,  558),    p( 490,  552),    p( 493,  551),    p( 506,  539),
        p( 454,  555),    p( 470,  548),    p( 465,  551),    p( 465,  545),    p( 489,  538),    p( 504,  532),    p( 513,  536),    p( 488,  537),
        p( 451,  555),    p( 460,  550),    p( 460,  551),    p( 463,  547),    p( 469,  543),    p( 486,  539),    p( 480,  544),    p( 476,  540),
        p( 442,  552),    p( 444,  550),    p( 445,  551),    p( 451,  547),    p( 460,  545),    p( 458,  544),    p( 473,  542),    p( 459,  540),
        p( 439,  547),    p( 444,  544),    p( 442,  545),    p( 445,  545),    p( 454,  538),    p( 466,  534),    p( 482,  528),    p( 468,  531),
        p( 439,  544),    p( 444,  543),    p( 450,  543),    p( 454,  539),    p( 461,  535),    p( 474,  528),    p( 485,  522),    p( 451,  535),
        p( 446,  548),    p( 447,  542),    p( 446,  546),    p( 451,  540),    p( 457,  534),    p( 458,  539),    p( 446,  545),    p( 443,  539),
    ],
    // queen
    [
        p( 870,  978),    p( 871,  984),    p( 884,  994),    p( 905,  985),    p( 909,  990),    p( 935,  979),    p( 970,  932),    p( 907,  961),
        p( 901,  966),    p( 886,  982),    p( 885, 1000),    p( 877, 1022),    p( 877, 1036),    p( 916, 1009),    p( 911,  991),    p( 956,  961),
        p( 911,  962),    p( 904,  970),    p( 901,  993),    p( 897, 1007),    p( 900, 1018),    p( 937, 1007),    p( 952,  977),    p( 936,  980),
        p( 900,  970),    p( 901,  978),    p( 899,  984),    p( 889, 1008),    p( 892, 1015),    p( 912, 1004),    p( 914, 1008),    p( 924,  984),
        p( 898,  970),    p( 893,  976),    p( 893,  981),    p( 892,  996),    p( 898,  999),    p( 902,  997),    p( 912,  994),    p( 916,  982),
        p( 901,  953),    p( 903,  964),    p( 896,  977),    p( 893,  981),    p( 896,  992),    p( 909,  984),    p( 918,  978),    p( 919,  957),
        p( 900,  945),    p( 896,  960),    p( 900,  963),    p( 900,  977),    p( 901,  979),    p( 904,  962),    p( 913,  946),    p( 906,  938),
        p( 880,  959),    p( 893,  948),    p( 893,  957),    p( 898,  962),    p( 900,  953),    p( 886,  964),    p( 882,  943),    p( 894,  915),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  71,    7),    p(  86,    3),    p( 104,   -9),    p( 192,  -67),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   6,   27),    p( -35,   39),    p( -26,   30),    p(  -3,   16),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -47,   39),    p( -28,   31),    p( -34,   24),    p( -33,   21),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-118,   36),    p( -93,   28),    p( -89,   15),    p( -74,   16),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-135,   30),    p(-108,   16),    p(-107,    5),    p( -95,    9),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-112,   18),    p(-108,   11),    p( -81,   -2),    p( -56,    8),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-102,    8),    p( -89,    2),    p( -61,  -13),    p(   1,   -2),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  20,  -12),    p(   6,   -7),    p(  29,  -20),    p(  58,  -23),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 51);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(13, 22), p(14, 19), p(13, 9), p(9, 1), p(5, -5), p(1, -14), p(-5, -22), p(-11, -34), p(-22, -38)];
const ROOK_OPEN_FILE: PhasedScore = p(15, -0);
const ROOK_CLOSED_FILE: PhasedScore = p(-10, -5);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(6, -5);
const KING_OPEN_FILE: PhasedScore = p(-37, 3);
const KING_CLOSED_FILE: PhasedScore = p(12, -6);
const KING_SEMIOPEN_FILE: PhasedScore = p(-2, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-4, 6), p(2, 8), p(-1, 7), p(5, 5), p(5, 5), p(5, 8), p(8, 5), p(21, 2)],
    // Closed
    [p(0, 0), p(0, 0), p(15, -14), p(-14, 13), p(2, 10), p(1, 6), p(1, 9), p(2, 6)],
    // SemiOpen
    [p(0, 0), p(-4, 27), p(8, 20), p(3, 12), p(1, 11), p(5, 7), p(3, 4), p(12, 7)],
    // SemiClosed
    [p(0, 0), p(13, -11), p(8, 7), p(5, 2), p(8, 2), p(2, 5), p(4, 5), p(5, 5)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(21, 10),
    p(2, 4),
    p(-2, 5),
    p(-11, 5),
    p(3, 4),
    p(-11, -9),
    p(-5, -1),
    p(-8, -12),
    p(0, -0),
    p(-11, -1),
    p(-11, -16),
    p(-20, -7),
    p(3, -5),
    p(-6, -9),
    p(3, -6),
    p(-0, 11),
    p(-6, -1),
    p(-22, -3),
    p(-16, -0),
    p(-41, 20),
    p(-21, 5),
    p(-21, -16),
    p(3, 24),
    p(-44, 22),
    p(-18, -16),
    p(-21, -15),
    p(-34, -32),
    p(-42, 10),
    p(-19, 1),
    p(10, -2),
    p(-87, 98),
    p(0, 0),
    p(1, -4),
    p(-12, -6),
    p(-4, -6),
    p(-24, -2),
    p(-23, -2),
    p(-47, -20),
    p(-27, 36),
    p(-32, 20),
    p(-6, -5),
    p(-19, -9),
    p(4, -10),
    p(-20, 34),
    p(-50, 19),
    p(-4, -28),
    p(0, 0),
    p(0, 0),
    p(-2, -11),
    p(-14, 7),
    p(-8, -52),
    p(0, 0),
    p(6, -10),
    p(-40, -11),
    p(0, 0),
    p(0, 0),
    p(-29, -5),
    p(-23, -12),
    p(-20, 16),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(23, 4),
    p(2, -2),
    p(-8, 3),
    p(-24, -1),
    p(8, -6),
    p(-27, -9),
    p(-19, -2),
    p(-38, -10),
    p(6, -3),
    p(-10, -8),
    p(-31, -5),
    p(-47, 1),
    p(-5, -6),
    p(-42, -5),
    p(-36, -8),
    p(-54, 61),
    p(10, -2),
    p(0, -8),
    p(-4, -11),
    p(-21, -2),
    p(-9, -2),
    p(-14, -12),
    p(-23, -1),
    p(-67, 180),
    p(-4, -11),
    p(-23, -14),
    p(-32, -29),
    p(14, -81),
    p(-12, -9),
    p(-10, -18),
    p(-78, 71),
    p(0, 0),
    p(12, -2),
    p(-2, -5),
    p(-17, -7),
    p(-27, -6),
    p(-1, -2),
    p(-27, -18),
    p(-16, -1),
    p(-29, 1),
    p(-1, -8),
    p(-20, -7),
    p(-27, -13),
    p(-37, -3),
    p(-7, -3),
    p(-46, -4),
    p(1, 16),
    p(-60, 63),
    p(2, -2),
    p(-13, -3),
    p(-31, 57),
    p(0, 0),
    p(-16, -6),
    p(-23, 6),
    p(0, 0),
    p(0, 0),
    p(-16, -1),
    p(-39, 9),
    p(-33, -45),
    p(0, 0),
    p(16, -62),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-11, 13),  /*0b0000*/
    p(-12, 10),  /*0b0001*/
    p(-3, 11),   /*0b0010*/
    p(-3, 9),    /*0b0011*/
    p(-7, 6),    /*0b0100*/
    p(-23, 1),   /*0b0101*/
    p(-9, 4),    /*0b0110*/
    p(-12, -14), /*0b0111*/
    p(3, 10),    /*0b1000*/
    p(-8, 13),   /*0b1001*/
    p(1, 11),    /*0b1010*/
    p(3, 10),    /*0b1011*/
    p(-3, 7),    /*0b1100*/
    p(-19, 7),   /*0b1101*/
    p(-9, 2),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(4, 11),    /*0b10000*/
    p(6, 6),     /*0b10001*/
    p(22, 8),    /*0b10010*/
    p(1, 5),     /*0b10011*/
    p(-6, 3),    /*0b10100*/
    p(13, 11),   /*0b10101*/
    p(-29, 2),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(10, 12),   /*0b11000*/
    p(26, 11),   /*0b11001*/
    p(29, 26),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(9, -3),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(9, 7),     /*0b100000*/
    p(1, 10),    /*0b100001*/
    p(15, 4),    /*0b100010*/
    p(6, -3),    /*0b100011*/
    p(-4, 0),    /*0b100100*/
    p(-20, -10), /*0b100101*/
    p(-20, 18),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(17, 2),    /*0b101000*/
    p(-1, 12),   /*0b101001*/
    p(12, -5),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(1, 2),     /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(6, 4),     /*0b110000*/
    p(14, 3),    /*0b110001*/
    p(16, -4),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(12, 13),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(21, -4),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(4, -6),    /*0b111111*/
    p(16, 5),    /*0b00*/
    p(22, -10),  /*0b01*/
    p(40, -6),   /*0b10*/
    p(13, -31),  /*0b11*/
    p(44, -2),   /*0b100*/
    p(16, -9),   /*0b101*/
    p(51, -28),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(58, -6),   /*0b1000*/
    p(13, -17),  /*0b1001*/
    p(32, -33),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(24, -16),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-19, 28),  /*0b1111*/
    p(23, 6),    /*0b00*/
    p(34, -8),   /*0b01*/
    p(30, -14),  /*0b10*/
    p(27, -38),  /*0b11*/
    p(33, -6),   /*0b100*/
    p(47, -16),  /*0b101*/
    p(21, -18),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(36, -1),   /*0b1000*/
    p(43, -15),  /*0b1001*/
    p(49, -43),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(31, -22),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(13, -43),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(-1, -37);
const STOPPABLE_PASSER: PhasedScore = p(25, -48);
const CLOSE_KING_PASSER: PhasedScore = p(3, 24);
const IMMOBILE_PASSER: PhasedScore = p(-1, -29);
const PROTECTED_PASSER: PhasedScore = p(8, -3);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -32,   46),    p( -40,   66),    p( -55,   61),    p( -59,   44),    p( -60,   46),    p( -48,   42),    p( -30,   42),    p( -57,   46),
        p( -24,   44),    p( -45,   67),    p( -50,   59),    p( -51,   44),    p( -68,   53),    p( -63,   50),    p( -49,   57),    p( -49,   44),
        p( -18,   56),    p( -27,   55),    p( -46,   59),    p( -44,   58),    p( -55,   63),    p( -49,   63),    p( -50,   71),    p( -69,   71),
        p(  -8,   71),    p( -12,   69),    p( -12,   59),    p( -31,   71),    p( -46,   79),    p( -37,   84),    p( -41,   91),    p( -61,   92),
        p(  -8,   63),    p(  10,   52),    p(  -5,   43),    p( -18,   37),    p( -18,   54),    p( -35,   69),    p( -54,   80),    p( -83,   91),
        p(  14,   61),    p(  14,   60),    p(  20,   54),    p(  23,   48),    p(  20,   55),    p(  21,   69),    p( -12,   75),    p(  -6,   75),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(1, 4), p(5, 7), p(8, 16), p(16, 22), p(19, 71), p(12, 64)];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -6);
const DOUBLED_PAWN: PhasedScore = p(-3, -14);
const IMMOBILE_PAWN: PhasedScore = p(-5, -9);
const PHALANX: [PhasedScore; 6] = [p(-1, 0), p(5, 3), p(9, 6), p(21, 20), p(57, 74), p(-83, 216)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 15), p(7, 17), p(14, 20), p(7, 9), p(-2, 12), p(-47, 9)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(49, 21), p(51, 45), p(66, 4), p(52, -9), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(-2, -7), p(12, 19), p(18, -8), p(14, 10), p(15, -9), p(28, -12)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-40, -68),
        p(-19, -28),
        p(-7, -5),
        p(2, 9),
        p(9, 19),
        p(16, 30),
        p(24, 33),
        p(31, 36),
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
        p(-30, -56),
        p(-17, -37),
        p(-5, -22),
        p(2, -9),
        p(9, 1),
        p(14, 10),
        p(19, 15),
        p(24, 19),
        p(26, 24),
        p(33, 26),
        p(39, 26),
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
        p(-72, 17),
        p(-63, 31),
        p(-59, 38),
        p(-55, 43),
        p(-56, 50),
        p(-51, 57),
        p(-48, 62),
        p(-44, 66),
        p(-41, 72),
        p(-37, 76),
        p(-33, 78),
        p(-31, 83),
        p(-24, 85),
        p(-15, 83),
        p(-10, 80),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-20, -22),
        p(-20, 10),
        p(-25, 67),
        p(-20, 83),
        p(-18, 102),
        p(-13, 108),
        p(-10, 120),
        p(-6, 127),
        p(-2, 133),
        p(1, 135),
        p(4, 139),
        p(9, 143),
        p(12, 144),
        p(14, 149),
        p(17, 151),
        p(22, 154),
        p(23, 160),
        p(27, 161),
        p(38, 157),
        p(53, 150),
        p(58, 151),
        p(100, 128),
        p(100, 131),
        p(127, 108),
        p(216, 77),
        p(259, 39),
        p(289, 27),
        p(264, 25),
    ],
    [
        p(-87, -1),
        p(-57, -11),
        p(-29, -10),
        p(0, -5),
        p(31, -3),
        p(54, -1),
        p(84, 5),
        p(110, 8),
        p(159, -0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-5, 14), p(0, 0), p(28, 26), p(61, 8), p(40, 0), p(0, 0)],
    [p(-2, 12), p(22, 25), p(0, 0), p(43, 22), p(47, 93), p(0, 0)],
    [p(-2, 18), p(12, 17), p(19, 14), p(0, 0), p(66, 45), p(0, 0)],
    [p(-1, 8), p(2, 9), p(1, 25), p(0, 13), p(0, 0), p(0, 0)],
    [p(65, 19), p(-17, 22), p(15, 15), p(-10, 23), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 8), p(8, 7), p(6, 10), p(12, 7), p(8, 14), p(3, 9)],
    [p(2, 9), p(11, 21), p(4, -45), p(9, 12), p(10, 20), p(4, 6)],
    [p(3, 4), p(12, 9), p(9, 13), p(11, 10), p(9, 28), p(19, -4)],
    [p(2, 3), p(8, 3), p(7, -2), p(5, 13), p(-62, -225), p(5, -11)],
    [p(49, 2), p(35, 12), p(41, 6), p(26, 6), p(38, -9), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-22, -14), p(17, -9), p(10, -3), p(17, -16), p(1, 4), p(-5, 3)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(10, -0), p(-6, 8), p(14, -8), p(-14, 25)];
const CHECK_STM: PhasedScore = p(36, 22);
const DISCOVERED_CHECK_STM: PhasedScore = p(145, 37);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(-6, -19), p(67, -2), p(105, -33), p(62, 82), p(0, 0), p(-26, -21)];
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
