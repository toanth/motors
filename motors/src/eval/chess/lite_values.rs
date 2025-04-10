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
        p( 126,  158),    p( 123,  157),    p( 115,  161),    p( 126,  143),    p( 112,  147),    p( 112,  151),    p(  75,  168),    p(  77,  168),
        p(  69,  116),    p(  67,  119),    p(  72,  112),    p(  80,  109),    p(  65,  105),    p( 119,  104),    p(  90,  124),    p(  92,  115),
        p(  53,  105),    p(  60,   99),    p(  57,   93),    p(  81,   96),    p(  88,   96),    p(  80,   83),    p(  73,   94),    p(  70,   89),
        p(  48,   91),    p(  50,   93),    p(  74,   90),    p(  94,   92),    p(  88,   94),    p(  83,   90),    p(  67,   84),    p(  58,   79),
        p(  41,   90),    p(  47,   86),    p(  68,   91),    p(  79,   93),    p(  80,   91),    p(  75,   90),    p(  66,   77),    p(  50,   78),
        p(  52,   93),    p(  55,   93),    p(  58,   92),    p(  53,   98),    p(  57,  100),    p(  73,   92),    p(  78,   82),    p(  57,   83),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 173,  281),    p( 195,  313),    p( 212,  326),    p( 251,  315),    p( 280,  317),    p( 198,  311),    p( 211,  312),    p( 201,  264),
        p( 266,  315),    p( 282,  319),    p( 298,  311),    p( 303,  315),    p( 301,  311),    p( 315,  300),    p( 273,  317),    p( 271,  307),
        p( 285,  310),    p( 305,  305),    p( 307,  312),    p( 321,  315),    p( 337,  310),    p( 351,  298),    p( 292,  306),    p( 284,  309),
        p( 300,  316),    p( 308,  310),    p( 325,  314),    p( 326,  321),    p( 324,  318),    p( 320,  317),    p( 310,  313),    p( 318,  312),
        p( 298,  317),    p( 303,  307),    p( 312,  313),    p( 320,  315),    p( 319,  319),    p( 324,  304),    p( 322,  304),    p( 312,  313),
        p( 275,  303),    p( 282,  302),    p( 296,  297),    p( 301,  310),    p( 305,  308),    p( 295,  291),    p( 301,  294),    p( 292,  308),
        p( 269,  311),    p( 280,  315),    p( 284,  305),    p( 294,  309),    p( 298,  304),    p( 289,  303),    p( 294,  308),    p( 289,  322),
        p( 241,  308),    p( 281,  306),    p( 266,  308),    p( 287,  312),    p( 295,  310),    p( 292,  300),    p( 287,  309),    p( 265,  308),
    ],
    // bishop
    [
        p( 275,  311),    p( 250,  316),    p( 239,  309),    p( 222,  318),    p( 216,  316),    p( 223,  310),    p( 271,  306),    p( 247,  312),
        p( 282,  304),    p( 278,  305),    p( 290,  307),    p( 276,  306),    p( 288,  303),    p( 292,  302),    p( 267,  310),    p( 270,  305),
        p( 295,  309),    p( 307,  305),    p( 290,  305),    p( 306,  301),    p( 304,  302),    p( 335,  306),    p( 316,  302),    p( 317,  313),
        p( 286,  312),    p( 291,  307),    p( 303,  303),    p( 305,  306),    p( 306,  304),    p( 298,  305),    p( 298,  309),    p( 280,  311),
        p( 289,  305),    p( 283,  309),    p( 295,  304),    p( 308,  304),    p( 301,  301),    p( 299,  303),    p( 285,  305),    p( 309,  300),
        p( 296,  308),    p( 299,  304),    p( 299,  307),    p( 299,  304),    p( 305,  307),    p( 297,  300),    p( 305,  296),    p( 307,  298),
        p( 308,  306),    p( 303,  300),    p( 307,  301),    p( 299,  309),    p( 301,  307),    p( 302,  306),    p( 311,  296),    p( 307,  295),
        p( 298,  304),    p( 309,  306),    p( 307,  307),    p( 290,  311),    p( 305,  309),    p( 294,  310),    p( 302,  299),    p( 301,  292),
    ],
    // rook
    [
        p( 459,  548),    p( 446,  558),    p( 440,  565),    p( 438,  562),    p( 450,  558),    p( 469,  553),    p( 478,  553),    p( 490,  545),
        p( 443,  554),    p( 440,  560),    p( 449,  560),    p( 464,  551),    p( 450,  554),    p( 466,  549),    p( 474,  546),    p( 490,  536),
        p( 442,  550),    p( 461,  545),    p( 455,  546),    p( 456,  542),    p( 481,  531),    p( 491,  528),    p( 509,  527),    p( 482,  530),
        p( 439,  550),    p( 445,  546),    p( 444,  548),    p( 450,  542),    p( 454,  534),    p( 465,  530),    p( 466,  534),    p( 465,  529),
        p( 433,  546),    p( 432,  545),    p( 433,  545),    p( 438,  541),    p( 445,  537),    p( 439,  537),    p( 453,  531),    p( 447,  529),
        p( 429,  543),    p( 429,  541),    p( 431,  539),    p( 433,  538),    p( 439,  533),    p( 450,  526),    p( 466,  515),    p( 453,  519),
        p( 432,  540),    p( 435,  538),    p( 441,  539),    p( 444,  536),    p( 451,  530),    p( 463,  520),    p( 471,  516),    p( 442,  526),
        p( 441,  545),    p( 437,  540),    p( 438,  543),    p( 443,  537),    p( 449,  530),    p( 455,  531),    p( 451,  530),    p( 447,  534),
    ],
    // queen
    [
        p( 877,  965),    p( 879,  978),    p( 894,  991),    p( 915,  985),    p( 912,  989),    p( 933,  977),    p( 979,  930),    p( 923,  963),
        p( 887,  955),    p( 863,  983),    p( 864, 1010),    p( 857, 1026),    p( 863, 1039),    p( 904,  999),    p( 904,  985),    p( 945,  966),
        p( 891,  961),    p( 885,  975),    p( 885,  995),    p( 884, 1005),    p( 907, 1007),    p( 945,  991),    p( 953,  963),    p( 939,  972),
        p( 878,  972),    p( 884,  979),    p( 878,  988),    p( 878, 1000),    p( 881, 1012),    p( 895, 1002),    p( 904, 1005),    p( 911,  983),
        p( 889,  962),    p( 876,  980),    p( 882,  979),    p( 882,  995),    p( 887,  992),    p( 888,  993),    p( 901,  983),    p( 907,  977),
        p( 885,  950),    p( 891,  965),    p( 886,  978),    p( 883,  980),    p( 888,  988),    p( 895,  978),    p( 909,  961),    p( 906,  951),
        p( 885,  953),    p( 886,  960),    p( 892,  962),    p( 892,  975),    p( 894,  974),    p( 894,  958),    p( 906,  937),    p( 913,  914),
        p( 871,  955),    p( 883,  943),    p( 884,  955),    p( 892,  957),    p( 894,  946),    p( 881,  950),    p( 882,  943),    p( 888,  928),
    ],
    // king
    [
        p( 156,  -77),    p(  73,  -32),    p(  87,  -21),    p(  17,    7),    p(  45,   -3),    p(  29,    8),    p(  86,   -2),    p( 218,  -80),
        p( -27,   11),    p( -58,   25),    p( -57,   30),    p(   5,   21),    p( -26,   30),    p( -50,   45),    p( -27,   33),    p(  10,   12),
        p( -44,   16),    p( -36,   17),    p( -75,   29),    p( -83,   39),    p( -46,   34),    p( -13,   27),    p( -57,   31),    p( -30,   17),
        p( -22,    2),    p( -93,   12),    p(-107,   26),    p(-127,   35),    p(-126,   35),    p(-105,   28),    p(-117,   19),    p(-101,   18),
        p( -38,   -5),    p(-103,    3),    p(-117,   19),    p(-143,   33),    p(-142,   32),    p(-117,   19),    p(-131,   11),    p(-114,   11),
        p( -31,   -1),    p( -79,   -1),    p(-109,   13),    p(-116,   22),    p(-114,   23),    p(-123,   16),    p( -99,    4),    p( -70,    9),
        p(  26,   -8),    p( -67,   -4),    p( -79,    4),    p( -99,   13),    p(-104,   16),    p( -91,    8),    p( -63,   -7),    p(   2,   -1),
        p(  47,  -22),    p(  42,  -33),    p(  38,  -20),    p( -23,   -1),    p(  28,  -17),    p( -20,   -3),    p(  33,  -26),    p(  59,  -33),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 52);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(9, 21), p(10, 18), p(10, 7), p(5, -0), p(1, -7), p(-3, -16), p(-9, -24), p(-16, -37), p(-26, -46)];
const ROOK_OPEN_FILE: PhasedScore = p(16, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-10, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(5, 0);
const KING_OPEN_FILE: PhasedScore = p(-50, 1);
const KING_CLOSED_FILE: PhasedScore = p(13, -9);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 4), p(1, 6), p(-2, 5), p(2, 4), p(2, 6), p(3, 7), p(6, 5), p(18, 2)],
    // Closed
    [p(0, 0), p(0, 0), p(19, -17), p(-15, 9), p(-1, 11), p(-0, 4), p(-2, 7), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-15, 23), p(2, 16), p(-0, 11), p(-1, 10), p(4, 6), p(0, 2), p(9, 5)],
    // SemiClosed
    [p(0, 0), p(12, -12), p(7, 6), p(3, -0), p(6, 1), p(0, 4), p(3, 5), p(2, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(23, 12),
    p(1, 6),
    p(-3, 7),
    p(-13, 6),
    p(7, 3),
    p(-10, -11),
    p(-2, -4),
    p(-7, -14),
    p(2, 0),
    p(-11, -1),
    p(-12, -16),
    p(-22, -9),
    p(7, -9),
    p(-2, -13),
    p(6, -9),
    p(1, 6),
    p(-5, -2),
    p(-25, -4),
    p(-22, 4),
    p(-52, 24),
    p(-19, 1),
    p(-21, -23),
    p(3, 23),
    p(-62, 30),
    p(-18, -17),
    p(-25, -15),
    p(-42, -31),
    p(-48, 14),
    p(-17, -4),
    p(8, -8),
    p(-96, 108),
    p(0, 0),
    p(-1, -2),
    p(-16, -5),
    p(-10, -5),
    p(-32, -1),
    p(-27, -1),
    p(-51, -21),
    p(-40, 39),
    p(-50, 19),
    p(-8, -5),
    p(-24, -9),
    p(-0, -13),
    p(-25, 33),
    p(-51, 18),
    p(-11, -30),
    p(0, 0),
    p(0, 0),
    p(-3, -17),
    p(-19, 2),
    p(-20, -53),
    p(0, 0),
    p(-3, -15),
    p(-49, -8),
    p(0, 0),
    p(0, 0),
    p(-32, -6),
    p(-24, -17),
    p(-25, 23),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(25, 4),
    p(2, -2),
    p(-6, 1),
    p(-23, -4),
    p(6, -4),
    p(-31, -9),
    p(-21, -5),
    p(-42, -14),
    p(7, -3),
    p(-13, -8),
    p(-31, -9),
    p(-48, -3),
    p(-10, -5),
    p(-48, -6),
    p(-39, -14),
    p(-61, 58),
    p(10, -3),
    p(-4, -9),
    p(-7, -13),
    p(-30, -4),
    p(-13, -4),
    p(-23, -15),
    p(-31, -4),
    p(-87, 174),
    p(-8, -12),
    p(-32, -15),
    p(-42, -30),
    p(-3, -79),
    p(-21, -10),
    p(-25, -19),
    p(-89, 64),
    p(0, 0),
    p(16, -1),
    p(1, -5),
    p(-14, -10),
    p(-24, -12),
    p(-1, 1),
    p(-30, -16),
    p(-21, -2),
    p(-36, -0),
    p(-1, -9),
    p(-25, -9),
    p(-29, -20),
    p(-42, -12),
    p(-13, -2),
    p(-52, -11),
    p(-21, 20),
    p(-70, 52),
    p(6, -5),
    p(-12, -5),
    p(-29, 53),
    p(0, 0),
    p(-20, -5),
    p(-26, 0),
    p(0, 0),
    p(0, 0),
    p(-17, -1),
    p(-45, 8),
    p(-41, -39),
    p(0, 0),
    p(-0, -61),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-8, 9),    /*0b0000*/
    p(-14, 9),   /*0b0001*/
    p(-5, 11),   /*0b0010*/
    p(-8, 12),   /*0b0011*/
    p(-3, 5),    /*0b0100*/
    p(-27, 2),   /*0b0101*/
    p(-12, 4),   /*0b0110*/
    p(-18, -12), /*0b0111*/
    p(11, 8),    /*0b1000*/
    p(-3, 12),   /*0b1001*/
    p(3, 10),    /*0b1010*/
    p(0, 12),    /*0b1011*/
    p(1, 5),     /*0b1100*/
    p(-26, 7),   /*0b1101*/
    p(-9, 5),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(7, 13),    /*0b10000*/
    p(5, 8),     /*0b10001*/
    p(18, 10),   /*0b10010*/
    p(-3, 8),    /*0b10011*/
    p(-3, 4),    /*0b10100*/
    p(13, 12),   /*0b10101*/
    p(-20, 2),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(17, 12),   /*0b11000*/
    p(26, 16),   /*0b11001*/
    p(36, 26),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(14, -0),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(18, 7),    /*0b100000*/
    p(6, 11),    /*0b100001*/
    p(23, 3),    /*0b100010*/
    p(8, 1),     /*0b100011*/
    p(-7, 4),    /*0b100100*/
    p(-23, -5),  /*0b100101*/
    p(-23, 19),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(29, 1),    /*0b101000*/
    p(0, 15),    /*0b101001*/
    p(20, -5),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-1, 7),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(16, 5),    /*0b110000*/
    p(22, 6),    /*0b110001*/
    p(28, -2),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(1, 19),    /*0b110100*/
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
    p(7, -1),    /*0b111111*/
    p(-15, -1),  /*0b00*/
    p(9, -16),   /*0b01*/
    p(38, -9),   /*0b10*/
    p(26, -43),  /*0b11*/
    p(48, -13),  /*0b100*/
    p(-5, -14),  /*0b101*/
    p(69, -44),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(66, -15),  /*0b1000*/
    p(19, -33),  /*0b1001*/
    p(74, -50),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(66, -44),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(21, -6),   /*0b1111*/
    p(20, 1),    /*0b00*/
    p(34, -11),  /*0b01*/
    p(27, -16),  /*0b10*/
    p(24, -40),  /*0b11*/
    p(40, -9),   /*0b100*/
    p(54, -20),  /*0b101*/
    p(25, -23),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(42, -4),   /*0b1000*/
    p(53, -18),  /*0b1001*/
    p(55, -41),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(44, -28),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(23, -45),  /*0b1111*/
];
const STOPPABLE_PASSER: PhasedScore = p(15, -37);
const CLOSE_KING_PASSER: PhasedScore = p(-8, 26);
const IMMOBILE_PASSER: PhasedScore = p(-9, -36);
const PROTECTED_PASSER: PhasedScore = p(7, 2);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -31,   23),    p( -21,   26),    p( -40,   29),    p( -41,   21),    p( -31,    8),    p( -21,    7),    p(   1,    8),    p( -21,   18),
        p( -22,   19),    p( -29,   31),    p( -36,   26),    p( -36,   18),    p( -44,   20),    p( -32,   18),    p( -31,   30),    p( -18,   18),
        p( -18,   40),    p( -22,   38),    p( -36,   36),    p( -31,   35),    p( -40,   32),    p( -28,   31),    p( -38,   43),    p( -39,   45),
        p(  -9,   59),    p( -11,   59),    p(  -6,   47),    p( -16,   50),    p( -32,   51),    p( -22,   50),    p( -36,   62),    p( -41,   67),
        p(  -0,   65),    p(  10,   61),    p(   5,   42),    p( -12,   25),    p(   3,   32),    p( -21,   40),    p( -30,   43),    p( -70,   70),
        p(  26,   58),    p(  23,   57),    p(  15,   61),    p(  26,   43),    p(  12,   47),    p(  12,   51),    p( -25,   68),    p( -23,   68),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(1, 6), p(4, 10), p(8, 19), p(18, 24), p(25, 66), p(14, 57)];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -5);
const DOUBLED_PAWN: PhasedScore = p(-6, -23);
const PHALANX: [PhasedScore; 6] = [p(-0, -0), p(6, 2), p(9, 5), p(24, 18), p(64, 74), p(-105, 231)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 12), p(7, 15), p(14, 20), p(9, 8), p(-3, 17), p(-45, 10)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(38, 7), p(39, 33), p(50, -11), p(34, -37), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-46, -72),
        p(-26, -32),
        p(-14, -8),
        p(-5, 5),
        p(2, 16),
        p(9, 27),
        p(17, 30),
        p(23, 32),
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
        p(-32, -55),
        p(-19, -37),
        p(-8, -22),
        p(-1, -10),
        p(6, -0),
        p(11, 8),
        p(16, 13),
        p(20, 17),
        p(22, 22),
        p(29, 23),
        p(33, 23),
        p(41, 25),
        p(37, 34),
        p(51, 26),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-66, 26),
        p(-62, 31),
        p(-59, 36),
        p(-60, 43),
        p(-55, 50),
        p(-52, 54),
        p(-48, 58),
        p(-45, 63),
        p(-42, 67),
        p(-39, 70),
        p(-38, 76),
        p(-30, 77),
        p(-23, 76),
        p(-19, 75),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-29, -45),
        p(-28, 7),
        p(-33, 60),
        p(-29, 79),
        p(-27, 97),
        p(-22, 103),
        p(-19, 114),
        p(-16, 120),
        p(-12, 125),
        p(-9, 126),
        p(-6, 129),
        p(-2, 132),
        p(1, 133),
        p(2, 138),
        p(4, 140),
        p(8, 144),
        p(8, 151),
        p(11, 153),
        p(20, 151),
        p(33, 145),
        p(37, 148),
        p(81, 125),
        p(79, 130),
        p(101, 113),
        p(194, 81),
        p(238, 40),
        p(271, 29),
        p(332, -13),
    ],
    [
        p(-85, 8),
        p(-52, -4),
        p(-26, -5),
        p(1, -3),
        p(30, -3),
        p(50, -2),
        p(77, 2),
        p(99, 3),
        p(142, -11),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-2, 10), p(20, 22), p(0, 0), p(31, 5), p(30, 53), p(0, 0)],
    [p(-3, 13), p(10, 16), p(17, 13), p(0, 0), p(45, -4), p(0, 0)],
    [p(-2, 4), p(2, 7), p(-1, 23), p(1, 3), p(0, 0), p(0, 0)],
    [p(60, 21), p(-35, 18), p(-9, 16), p(-22, 8), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 7), p(8, 7), p(6, 10), p(12, 7), p(7, 18), p(11, 6)],
    [p(2, 7), p(11, 21), p(-136, -15), p(8, 13), p(10, 18), p(4, 7)],
    [p(2, 3), p(12, 9), p(8, 14), p(11, 10), p(9, 25), p(20, -4)],
    [p(2, 0), p(8, 2), p(7, -3), p(4, 14), p(-59, -259), p(4, -10)],
    [p(60, -4), p(38, 7), p(43, 1), p(21, 5), p(34, -12), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-15, -17), p(19, -10), p(10, -3), p(14, -11), p(-0, 12), p(0, 3)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(14, 19), p(34, 0), p(5, 33)];

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
