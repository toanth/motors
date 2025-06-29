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

use crate::eval::chess::lite::NUM_SAFE_SQUARE_ENTRIES;
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
        p( 115,  160),    p( 115,  160),    p( 120,  154),    p( 123,  149),    p( 121,  155),    p( 122,  169),    p(  87,  176),    p(  95,  175),
        p(  70,  120),    p(  71,  122),    p(  85,  112),    p(  85,  110),    p(  81,  104),    p( 130,  106),    p( 115,  122),    p( 119,  113),
        p(  52,  103),    p(  61,   99),    p(  60,   93),    p(  85,   98),    p(  89,   96),    p(  88,   85),    p(  89,   95),    p(  91,   92),
        p(  45,   90),    p(  50,   93),    p(  72,   92),    p(  89,   94),    p(  94,   93),    p(  90,   91),    p(  80,   89),    p(  76,   83),
        p(  35,   89),    p(  46,   86),    p(  68,   91),    p(  81,   93),    p(  81,   94),    p(  82,   92),    p(  84,   78),    p(  72,   81),
        p(  46,   96),    p(  54,   94),    p(  60,   94),    p(  58,   99),    p(  63,  102),    p(  90,   91),    p( 107,   79),    p(  82,   81),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 188,  282),    p( 193,  323),    p( 210,  324),    p( 244,  321),    p( 288,  307),    p( 209,  314),    p( 235,  295),    p( 200,  256),
        p( 276,  316),    p( 287,  323),    p( 297,  313),    p( 300,  316),    p( 305,  310),    p( 313,  303),    p( 296,  313),    p( 282,  303),
        p( 289,  313),    p( 304,  310),    p( 309,  316),    p( 319,  317),    p( 330,  314),    p( 351,  301),    p( 296,  309),    p( 298,  307),
        p( 308,  320),    p( 316,  316),    p( 330,  319),    p( 330,  326),    p( 330,  323),    p( 332,  320),    p( 321,  321),    p( 330,  312),
        p( 301,  320),    p( 313,  313),    p( 317,  319),    p( 325,  321),    p( 322,  323),    p( 330,  310),    p( 327,  309),    p( 317,  318),
        p( 278,  308),    p( 286,  306),    p( 296,  303),    p( 304,  315),    p( 308,  314),    p( 307,  293),    p( 306,  300),    p( 300,  308),
        p( 270,  314),    p( 285,  317),    p( 287,  311),    p( 296,  313),    p( 304,  309),    p( 298,  304),    p( 301,  313),    p( 295,  321),
        p( 243,  309),    p( 280,  309),    p( 271,  309),    p( 292,  314),    p( 298,  314),    p( 297,  305),    p( 287,  311),    p( 271,  311),
    ],
    // bishop
    [
        p( 277,  317),    p( 262,  312),    p( 231,  309),    p( 226,  320),    p( 214,  318),    p( 238,  309),    p( 261,  312),    p( 260,  307),
        p( 289,  306),    p( 289,  306),    p( 296,  307),    p( 280,  309),    p( 287,  305),    p( 296,  304),    p( 259,  315),    p( 279,  305),
        p( 301,  310),    p( 306,  307),    p( 291,  311),    p( 304,  300),    p( 303,  307),    p( 333,  306),    p( 318,  308),    p( 316,  316),
        p( 289,  314),    p( 299,  309),    p( 309,  302),    p( 309,  309),    p( 310,  305),    p( 310,  306),    p( 313,  305),    p( 291,  310),
        p( 289,  311),    p( 288,  311),    p( 295,  308),    p( 310,  303),    p( 306,  304),    p( 308,  299),    p( 292,  306),    p( 314,  299),
        p( 296,  307),    p( 298,  308),    p( 297,  310),    p( 299,  307),    p( 306,  306),    p( 300,  301),    p( 308,  296),    p( 312,  301),
        p( 306,  305),    p( 298,  303),    p( 305,  304),    p( 298,  313),    p( 300,  310),    p( 307,  307),    p( 316,  297),    p( 310,  303),
        p( 297,  304),    p( 308,  306),    p( 306,  310),    p( 292,  313),    p( 306,  313),    p( 293,  312),    p( 304,  307),    p( 305,  297),
    ],
    // rook
    [
        p( 448,  565),    p( 443,  569),    p( 431,  576),    p( 427,  574),    p( 441,  568),    p( 463,  568),    p( 452,  570),    p( 492,  549),
        p( 454,  565),    p( 452,  569),    p( 457,  569),    p( 473,  561),    p( 461,  563),    p( 489,  557),    p( 492,  556),    p( 504,  544),
        p( 455,  559),    p( 471,  553),    p( 465,  556),    p( 463,  550),    p( 488,  542),    p( 507,  536),    p( 515,  540),    p( 490,  541),
        p( 453,  558),    p( 462,  554),    p( 461,  555),    p( 464,  551),    p( 472,  546),    p( 487,  542),    p( 482,  548),    p( 479,  542),
        p( 443,  555),    p( 444,  553),    p( 445,  555),    p( 453,  551),    p( 460,  548),    p( 458,  548),    p( 473,  544),    p( 459,  542),
        p( 438,  550),    p( 443,  546),    p( 442,  548),    p( 444,  549),    p( 453,  542),    p( 464,  536),    p( 478,  530),    p( 465,  533),
        p( 439,  548),    p( 443,  547),    p( 450,  547),    p( 454,  543),    p( 460,  539),    p( 473,  532),    p( 484,  526),    p( 451,  538),
        p( 445,  553),    p( 446,  546),    p( 445,  550),    p( 450,  544),    p( 456,  538),    p( 456,  543),    p( 445,  549),    p( 442,  543),
    ],
    // queen
    [
        p( 867,  979),    p( 867,  986),    p( 879,  997),    p( 898,  989),    p( 904,  994),    p( 931,  982),    p( 968,  934),    p( 905,  961),
        p( 900,  966),    p( 884,  984),    p( 881, 1002),    p( 873, 1024),    p( 872, 1040),    p( 910, 1014),    p( 908,  993),    p( 955,  960),
        p( 912,  961),    p( 904,  970),    p( 898,  995),    p( 890, 1011),    p( 892, 1022),    p( 936, 1010),    p( 952,  980),    p( 938,  981),
        p( 900,  968),    p( 902,  977),    p( 898,  983),    p( 889, 1006),    p( 891, 1015),    p( 910, 1006),    p( 916, 1009),    p( 924,  985),
        p( 896,  968),    p( 893,  975),    p( 893,  980),    p( 894,  995),    p( 898,  998),    p( 901,  996),    p( 909,  993),    p( 914,  980),
        p( 896,  953),    p( 899,  964),    p( 894,  979),    p( 892,  983),    p( 894,  994),    p( 904,  983),    p( 911,  976),    p( 912,  955),
        p( 897,  947),    p( 893,  962),    p( 898,  967),    p( 898,  980),    p( 898,  983),    p( 902,  965),    p( 910,  948),    p( 904,  939),
        p( 877,  962),    p( 890,  950),    p( 890,  959),    p( 895,  964),    p( 896,  955),    p( 883,  966),    p( 880,  944),    p( 891,  917),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  72,    4),    p(  85,    0),    p( 104,  -12),    p( 196,  -71),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   5,   27),    p( -34,   39),    p( -26,   30),    p(  -2,   14),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -46,   38),    p( -27,   31),    p( -34,   24),    p( -32,   19),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-117,   36),    p( -92,   28),    p( -89,   16),    p( -70,   14),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-134,   30),    p(-107,   17),    p(-109,    6),    p( -93,    7),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-112,   19),    p(-111,   11),    p( -83,   -1),    p( -56,    6),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-100,    9),    p( -87,    3),    p( -59,  -11),    p(   1,   -3),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  21,  -14),    p(   7,   -9),    p(  30,  -22),    p(  59,  -26),
    ],
];

const BISHOP_PAIR: PhasedScore = p(24, 51);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(12, 24), p(13, 21), p(13, 10), p(9, 2), p(4, -5), p(1, -13), p(-5, -21), p(-12, -34), p(-24, -38)];
const ROOK_OPEN_FILE: PhasedScore = p(15, 1);
const ROOK_CLOSED_FILE: PhasedScore = p(-11, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(6, -3);
const KING_OPEN_FILE: PhasedScore = p(-37, 6);
const KING_CLOSED_FILE: PhasedScore = p(12, -6);
const KING_SEMIOPEN_FILE: PhasedScore = p(-3, 11);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-4, 6), p(2, 8), p(-2, 8), p(4, 5), p(4, 6), p(5, 9), p(8, 6), p(21, 3)],
    // Closed
    [p(0, 0), p(0, 0), p(15, -15), p(-13, 12), p(1, 11), p(1, 7), p(0, 9), p(2, 6)],
    // SemiOpen
    [p(0, 0), p(-5, 29), p(6, 22), p(2, 13), p(1, 12), p(4, 8), p(3, 5), p(11, 8)],
    // SemiClosed
    [p(0, 0), p(13, -11), p(8, 8), p(4, 1), p(7, 2), p(2, 6), p(4, 6), p(4, 5)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(22, 11),
    p(3, 5),
    p(-3, 5),
    p(-11, 7),
    p(3, 4),
    p(-11, -9),
    p(-5, -1),
    p(-7, -11),
    p(0, 1),
    p(-11, -0),
    p(-12, -15),
    p(-20, -6),
    p(1, -5),
    p(-6, -9),
    p(3, -6),
    p(2, 14),
    p(-5, -1),
    p(-22, -4),
    p(-16, 1),
    p(-41, 21),
    p(-22, 4),
    p(-22, -18),
    p(2, 24),
    p(-45, 26),
    p(-18, -15),
    p(-22, -15),
    p(-37, -29),
    p(-43, 12),
    p(-19, 0),
    p(9, -4),
    p(-86, 96),
    p(0, 0),
    p(1, -4),
    p(-12, -6),
    p(-5, -6),
    p(-24, -2),
    p(-23, -1),
    p(-47, -20),
    p(-27, 36),
    p(-33, 25),
    p(-7, -4),
    p(-19, -9),
    p(3, -11),
    p(-20, 33),
    p(-51, 19),
    p(-4, -28),
    p(0, 0),
    p(0, 0),
    p(-2, -11),
    p(-14, 7),
    p(-9, -54),
    p(0, 0),
    p(5, -10),
    p(-38, -15),
    p(0, 0),
    p(0, 0),
    p(-28, -5),
    p(-21, -12),
    p(-20, 17),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(24, 3),
    p(2, -2),
    p(-8, 3),
    p(-25, -0),
    p(8, -6),
    p(-27, -9),
    p(-21, -1),
    p(-41, -8),
    p(7, -3),
    p(-11, -8),
    p(-32, -6),
    p(-48, 2),
    p(-5, -5),
    p(-42, -5),
    p(-37, -6),
    p(-56, 65),
    p(11, -2),
    p(-0, -9),
    p(-4, -11),
    p(-22, -3),
    p(-9, -3),
    p(-16, -13),
    p(-24, -1),
    p(-68, 178),
    p(-4, -11),
    p(-24, -14),
    p(-33, -28),
    p(9, -80),
    p(-13, -9),
    p(-12, -18),
    p(-79, 68),
    p(0, 0),
    p(13, -2),
    p(-2, -4),
    p(-19, -7),
    p(-28, -6),
    p(-1, -1),
    p(-27, -17),
    p(-18, -2),
    p(-31, 2),
    p(-1, -8),
    p(-22, -8),
    p(-29, -13),
    p(-39, -2),
    p(-9, -2),
    p(-48, -5),
    p(-1, 16),
    p(-61, 62),
    p(2, -2),
    p(-14, -3),
    p(-31, 56),
    p(0, 0),
    p(-16, -5),
    p(-23, 5),
    p(0, 0),
    p(0, 0),
    p(-16, 0),
    p(-41, 9),
    p(-34, -42),
    p(0, 0),
    p(12, -62),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-11, 9),   /*0b0000*/
    p(-12, 8),   /*0b0001*/
    p(-3, 12),   /*0b0010*/
    p(-3, 12),   /*0b0011*/
    p(-7, 3),    /*0b0100*/
    p(-22, 0),   /*0b0101*/
    p(-9, 6),    /*0b0110*/
    p(-11, -10), /*0b0111*/
    p(3, 6),     /*0b1000*/
    p(-8, 10),   /*0b1001*/
    p(0, 12),    /*0b1010*/
    p(4, 12),    /*0b1011*/
    p(-3, 4),    /*0b1100*/
    p(-17, 4),   /*0b1101*/
    p(-8, 5),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(4, 10),    /*0b10000*/
    p(5, 7),     /*0b10001*/
    p(21, 9),    /*0b10010*/
    p(0, 7),     /*0b10011*/
    p(-7, 4),    /*0b10100*/
    p(13, 14),   /*0b10101*/
    p(-28, 3),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(10, 10),   /*0b11000*/
    p(27, 12),   /*0b11001*/
    p(28, 26),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(8, -2),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(9, 3),     /*0b100000*/
    p(1, 8),     /*0b100001*/
    p(15, 5),    /*0b100010*/
    p(6, 0),     /*0b100011*/
    p(-5, -2),   /*0b100100*/
    p(-19, -11), /*0b100101*/
    p(-19, 19),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(17, -2),   /*0b101000*/
    p(0, 8),     /*0b101001*/
    p(11, -4),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(1, -0),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(5, 3),     /*0b110000*/
    p(14, 5),    /*0b110001*/
    p(16, -3),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(11, 14),   /*0b110100*/
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
    p(6, -3),    /*0b111111*/
    p(16, 0),    /*0b00*/
    p(23, -9),   /*0b01*/
    p(40, -8),   /*0b10*/
    p(14, -29),  /*0b11*/
    p(44, -4),   /*0b100*/
    p(15, -8),   /*0b101*/
    p(51, -28),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(59, -11),  /*0b1000*/
    p(13, -18),  /*0b1001*/
    p(30, -35),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(24, -18),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-15, 26),  /*0b1111*/
    p(24, 1),    /*0b00*/
    p(33, -10),  /*0b01*/
    p(30, -13),  /*0b10*/
    p(28, -36),  /*0b11*/
    p(33, -10),  /*0b100*/
    p(45, -18),  /*0b101*/
    p(21, -18),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(36, -3),   /*0b1000*/
    p(43, -13),  /*0b1001*/
    p(48, -43),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(30, -24),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(15, -42),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(-3, -37);
const STOPPABLE_PASSER: PhasedScore = p(25, -48);
const CLOSE_KING_PASSER: PhasedScore = p(4, 24);
const IMMOBILE_PASSER: PhasedScore = p(-5, -36);
const PROTECTED_PASSER: PhasedScore = p(8, -3);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -31,   46),    p( -39,   66),    p( -54,   61),    p( -58,   44),    p( -59,   46),    p( -48,   43),    p( -31,   43),    p( -58,   46),
        p( -24,   43),    p( -45,   67),    p( -51,   59),    p( -51,   45),    p( -68,   53),    p( -63,   51),    p( -50,   57),    p( -49,   44),
        p( -17,   55),    p( -27,   55),    p( -45,   59),    p( -43,   58),    p( -54,   63),    p( -49,   64),    p( -50,   71),    p( -69,   70),
        p(  -7,   70),    p( -11,   69),    p( -11,   60),    p( -29,   72),    p( -45,   80),    p( -36,   85),    p( -39,   91),    p( -60,   92),
        p(  -4,   62),    p(  13,   52),    p(  -1,   44),    p( -16,   38),    p( -15,   55),    p( -32,   70),    p( -51,   80),    p( -80,   91),
        p(  15,   60),    p(  15,   60),    p(  20,   54),    p(  23,   49),    p(  21,   55),    p(  22,   69),    p( -13,   76),    p(  -5,   75),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(1, 4), p(5, 8), p(8, 17), p(15, 22), p(18, 71), p(13, 64)];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -6);
const DOUBLED_PAWN: PhasedScore = p(-7, -22);
const PHALANX: [PhasedScore; 6] = [p(-1, 0), p(5, 3), p(9, 6), p(21, 20), p(59, 73), p(-83, 216)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 14), p(7, 18), p(15, 21), p(7, 10), p(-2, 12), p(-49, 13)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(49, 21), p(52, 45), p(66, 4), p(52, -9), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(1, -5), p(14, 20), p(18, -7), p(15, 10), p(16, -9), p(29, -12)];
const SAFE_SQUARES: [PhasedScore; NUM_SAFE_SQUARE_ENTRIES] = [
    p(243, 23),
    p(9, 13),
    p(-30, 11),
    p(-6, 1),
    p(7, -6),
    p(6, -6),
    p(-3, -0),
    p(-11, 7),
    p(-13, 20),
    p(-40, 34),
    p(39, 24),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-41, -66),
        p(-20, -26),
        p(-8, -3),
        p(1, 10),
        p(8, 20),
        p(15, 31),
        p(23, 34),
        p(29, 37),
        p(33, 36),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-18, -36),
        p(-6, -22),
        p(2, -9),
        p(9, 1),
        p(14, 10),
        p(19, 15),
        p(24, 20),
        p(25, 26),
        p(32, 28),
        p(37, 29),
        p(40, 34),
        p(32, 46),
        p(41, 39),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-73, 20),
        p(-64, 32),
        p(-60, 39),
        p(-55, 43),
        p(-56, 50),
        p(-51, 57),
        p(-48, 62),
        p(-44, 66),
        p(-41, 72),
        p(-38, 76),
        p(-35, 80),
        p(-34, 86),
        p(-28, 88),
        p(-21, 87),
        p(-19, 85),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-26, 8),
        p(-25, 34),
        p(-29, 88),
        p(-24, 103),
        p(-22, 120),
        p(-16, 124),
        p(-13, 135),
        p(-9, 141),
        p(-4, 146),
        p(-1, 147),
        p(3, 150),
        p(7, 153),
        p(10, 154),
        p(12, 159),
        p(15, 161),
        p(19, 164),
        p(20, 170),
        p(24, 171),
        p(34, 168),
        p(48, 162),
        p(53, 163),
        p(94, 141),
        p(93, 145),
        p(121, 122),
        p(208, 92),
        p(248, 55),
        p(276, 45),
        p(260, 40),
    ],
    [
        p(-87, 1),
        p(-57, -10),
        p(-28, -10),
        p(0, -6),
        p(31, -4),
        p(54, -2),
        p(82, 5),
        p(106, 8),
        p(152, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-4, 14), p(0, 0), p(28, 26), p(61, 7), p(40, -1), p(0, 0)],
    [p(-1, 11), p(22, 25), p(0, 0), p(43, 21), p(47, 92), p(0, 0)],
    [p(-3, 18), p(12, 18), p(19, 14), p(0, 0), p(66, 45), p(0, 0)],
    [p(-2, 9), p(2, 10), p(1, 26), p(-0, 13), p(0, 0), p(0, 0)],
    [p(64, 21), p(-19, 23), p(9, 16), p(-14, 24), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 9), p(8, 8), p(6, 10), p(12, 8), p(8, 14), p(3, 9)],
    [p(2, 9), p(11, 23), p(-4, -45), p(9, 12), p(10, 20), p(4, 7)],
    [p(2, 4), p(12, 10), p(9, 14), p(11, 11), p(9, 29), p(19, -3)],
    [p(2, 3), p(8, 4), p(7, -2), p(5, 13), p(-62, -224), p(5, -10)],
    [p(49, 0), p(36, 12), p(41, 7), p(27, 5), p(37, -10), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-20, -14), p(17, -9), p(9, -2), p(15, -16), p(-3, 4), p(-5, 2)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(8, 1), p(-5, 7), p(15, -8), p(-10, 24)];
const CHECK_STM: PhasedScore = p(38, 22);
const DISCOVERED_CHECK_STM: PhasedScore = p(177, 59);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(11, -7), p(63, 0), p(102, -34), p(55, 90), p(0, 0), p(-25, -23)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(6, -17), p(25, 30), p(15, 34), p(43, 8), p(62, 4)];

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

    fn safe_squares(num_squares: usize) -> SingleFeatureScore<Self::Score>;

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

    fn safe_squares(i: usize) -> PhasedScore {
        SAFE_SQUARES[i]
    }
}
