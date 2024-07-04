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

use crate::eval::chess::{FileOpenness, NUM_PAWN_SHIELD_CONFIGURATIONS};
use crate::eval::ScoreType;
use gears::games::chess::pieces::{UncoloredChessPiece, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::Color;
use gears::games::Color::*;
use gears::score::{p, PhasedScore};
use std::fmt::Debug;

#[rustfmt::skip]
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
        p( 127,  182),    p( 126,  181),    p( 117,  184),    p( 129,  166),    p( 116,  170),    p( 118,  173),    p(  81,  191),    p(  86,  188),
        p(  64,  122),    p(  65,  123),    p(  73,  112),    p(  82,  117),    p(  70,  120),    p( 116,  105),    p(  92,  130),    p(  83,  119),
        p(  52,  109),    p(  65,  104),    p(  59,   96),    p(  62,   88),    p(  78,   90),    p(  81,   88),    p(  74,   99),    p(  69,   92),
        p(  47,   96),    p(  58,   98),    p(  61,   86),    p(  69,   85),    p(  72,   86),    p(  76,   81),    p(  70,   89),    p(  57,   81),
        p(  41,   91),    p(  54,   88),    p(  52,   84),    p(  56,   91),    p(  66,   88),    p(  58,   84),    p(  70,   79),    p(  50,   79),
        p(  52,   96),    p(  64,   96),    p(  60,   90),    p(  58,   97),    p(  65,  100),    p(  75,   92),    p(  86,   84),    p(  53,   83),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 187,  271),    p( 209,  302),    p( 244,  314),    p( 269,  304),    p( 301,  306),    p( 213,  302),    p( 234,  301),    p( 217,  252),
        p( 276,  303),    p( 288,  312),    p( 302,  309),    p( 313,  312),    p( 305,  309),    p( 328,  298),    p( 287,  309),    p( 292,  295),
        p( 291,  301),    p( 303,  306),    p( 318,  314),    p( 320,  318),    p( 336,  312),    p( 360,  302),    p( 313,  303),    p( 308,  299),
        p( 304,  310),    p( 309,  309),    p( 316,  320),    p( 342,  322),    p( 319,  323),    p( 331,  321),    p( 313,  311),    p( 331,  304),
        p( 299,  313),    p( 298,  307),    p( 303,  320),    p( 310,  324),    p( 317,  324),    p( 314,  311),    p( 325,  303),    p( 314,  309),
        p( 274,  300),    p( 275,  303),    p( 282,  303),    p( 288,  317),    p( 295,  313),    p( 280,  298),    p( 296,  294),    p( 291,  304),
        p( 271,  305),    p( 280,  309),    p( 276,  303),    p( 288,  308),    p( 291,  303),    p( 283,  299),    p( 294,  300),    p( 290,  313),
        p( 244,  301),    p( 283,  300),    p( 265,  301),    p( 285,  306),    p( 295,  303),    p( 291,  293),    p( 289,  299),    p( 268,  298),
    ],
    // bishop
    [
        p( 283,  315),    p( 253,  321),    p( 249,  316),    p( 227,  322),    p( 226,  321),    p( 227,  316),    p( 280,  311),    p( 257,  309),
        p( 281,  309),    p( 292,  312),    p( 293,  314),    p( 284,  317),    p( 285,  313),    p( 298,  311),    p( 279,  314),    p( 276,  309),
        p( 294,  318),    p( 309,  311),    p( 300,  318),    p( 304,  313),    p( 308,  316),    p( 336,  316),    p( 323,  310),    p( 308,  320),
        p( 285,  315),    p( 301,  319),    p( 303,  317),    p( 322,  321),    p( 316,  317),    p( 308,  320),    p( 304,  318),    p( 284,  317),
        p( 294,  313),    p( 283,  320),    p( 300,  321),    p( 319,  318),    p( 315,  318),    p( 301,  319),    p( 292,  318),    p( 314,  304),
        p( 292,  314),    p( 306,  317),    p( 304,  318),    p( 306,  321),    p( 310,  323),    p( 305,  317),    p( 310,  309),    p( 308,  308),
        p( 305,  316),    p( 308,  307),    p( 313,  309),    p( 298,  320),    p( 303,  318),    p( 305,  312),    p( 315,  308),    p( 300,  304),
        p( 294,  306),    p( 315,  312),    p( 310,  314),    p( 291,  317),    p( 305,  315),    p( 298,  321),    p( 307,  301),    p( 299,  297),
    ],
    // rook
    [
        p( 461,  548),    p( 451,  558),    p( 448,  564),    p( 447,  561),    p( 458,  558),    p( 479,  552),    p( 487,  550),    p( 498,  543),
        p( 433,  554),    p( 430,  560),    p( 440,  560),    p( 456,  550),    p( 445,  552),    p( 465,  547),    p( 476,  544),    p( 490,  535),
        p( 436,  552),    p( 458,  547),    p( 455,  548),    p( 459,  544),    p( 485,  533),    p( 494,  529),    p( 517,  526),    p( 487,  528),
        p( 436,  551),    p( 445,  547),    p( 446,  550),    p( 452,  544),    p( 460,  536),    p( 468,  531),    p( 474,  533),    p( 471,  528),
        p( 432,  547),    p( 433,  545),    p( 433,  547),    p( 440,  544),    p( 446,  540),    p( 441,  539),    p( 459,  531),    p( 449,  529),
        p( 429,  543),    p( 429,  542),    p( 432,  541),    p( 435,  542),    p( 442,  536),    p( 450,  528),    p( 472,  515),    p( 454,  519),
        p( 432,  538),    p( 437,  538),    p( 442,  540),    p( 445,  537),    p( 452,  531),    p( 466,  521),    p( 476,  516),    p( 444,  524),
        p( 441,  542),    p( 436,  538),    p( 438,  543),    p( 442,  539),    p( 449,  533),    p( 456,  532),    p( 453,  528),    p( 448,  530),
    ],
    // queen
    [
        p( 874,  963),    p( 873,  979),    p( 888,  992),    p( 905,  988),    p( 904,  993),    p( 925,  979),    p( 974,  927),    p( 925,  955),
        p( 883,  955),    p( 859,  987),    p( 859, 1014),    p( 852, 1032),    p( 859, 1043),    p( 899, 1005),    p( 904,  983),    p( 945,  962),
        p( 890,  961),    p( 883,  980),    p( 882, 1003),    p( 879, 1013),    p( 902, 1015),    p( 943,  997),    p( 949,  967),    p( 935,  973),
        p( 876,  976),    p( 880,  987),    p( 874,  996),    p( 874, 1010),    p( 879, 1020),    p( 891, 1011),    p( 899, 1009),    p( 907,  986),
        p( 885,  966),    p( 872,  986),    p( 877,  989),    p( 878, 1007),    p( 879, 1004),    p( 881, 1003),    p( 896,  989),    p( 903,  980),
        p( 881,  952),    p( 886,  970),    p( 880,  985),    p( 876,  990),    p( 881,  996),    p( 888,  985),    p( 903,  967),    p( 902,  954),
        p( 884,  950),    p( 884,  958),    p( 888,  962),    p( 887,  975),    p( 888,  975),    p( 890,  958),    p( 901,  937),    p( 909,  911),
        p( 872,  945),    p( 881,  937),    p( 880,  952),    p( 888,  953),    p( 890,  947),    p( 879,  947),    p( 881,  935),    p( 884,  923),
    ],
    // king
    [
        p( 148, -105),    p(  57,  -51),    p(  80,  -43),    p(   3,  -11),    p(  24,  -23),    p(   6,  -14),    p(  60,  -23),    p( 216, -108),
        p( -23,   -5),    p( -63,   25),    p( -74,   36),    p(  -8,   25),    p( -40,   33),    p( -70,   46),    p( -33,   30),    p(  12,   -3),
        p( -44,    4),    p( -32,   22),    p( -78,   40),    p( -83,   47),    p( -50,   41),    p( -19,   34),    p( -55,   32),    p( -31,    9),
        p( -28,   -2),    p( -86,   22),    p(-103,   39),    p(-125,   48),    p(-124,   46),    p(-103,   38),    p(-110,   27),    p( -98,   14),
        p( -47,   -5),    p(-107,   17),    p(-118,   34),    p(-139,   47),    p(-145,   46),    p(-122,   31),    p(-133,   22),    p(-114,   11),
        p( -36,   -1),    p( -82,   12),    p(-111,   27),    p(-121,   37),    p(-115,   35),    p(-128,   27),    p( -98,   12),    p( -70,    9),
        p(  28,   -9),    p( -65,    7),    p( -78,   16),    p(-101,   26),    p(-105,   26),    p( -90,   17),    p( -54,   -0),    p(   7,   -5),
        p(  39,  -44),    p(  40,  -49),    p(  35,  -35),    p( -27,  -14),    p(  27,  -32),    p( -23,  -18),    p(  33,  -44),    p(  59,  -53),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  27,   82),    p(  26,   81),    p(  17,   84),    p(  29,   66),    p(  16,   70),    p(  18,   73),    p( -19,   91),    p( -14,   88),
        p(  28,  115),    p(  37,  115),    p(  30,   98),    p(  16,   67),    p(  27,   65),    p(   9,   92),    p(  -7,   97),    p( -35,  118),
        p(  10,   67),    p(   9,   67),    p(  18,   52),    p(  10,   44),    p(  -5,   45),    p(   1,   56),    p( -16,   71),    p( -18,   73),
        p(  -3,   41),    p( -12,   40),    p( -21,   34),    p( -12,   25),    p( -20,   29),    p( -19,   37),    p( -25,   50),    p( -18,   47),
        p(  -8,   11),    p( -21,   20),    p( -20,   18),    p( -17,    8),    p( -17,   14),    p( -15,   17),    p( -19,   33),    p(   2,   14),
        p( -15,   10),    p( -12,   14),    p( -14,   17),    p( -10,    5),    p(   1,    1),    p(  -1,    6),    p(   5,   13),    p(   1,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(15, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -4);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(-2, -1);
const KING_OPEN_FILE: PhasedScore = p(-59, -3);
const KING_CLOSED_FILE: PhasedScore = p(14, -16);
const KING_SEMIOPEN_FILE: PhasedScore = p(-12, 3);
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-12, 5),   /*0b0000*/
    p(-18, 10),  /*0b0001*/
    p(-12, 7),   /*0b0010*/
    p(-6, 25),   /*0b0011*/
    p(-5, 6),    /*0b0100*/
    p(-28, 1),   /*0b0101*/
    p(-11, 17),  /*0b0110*/
    p(-10, -1),  /*0b0111*/
    p(1, 9),     /*0b1000*/
    p(-23, -12), /*0b1001*/
    p(-6, 8),    /*0b1010*/
    p(-5, 1),    /*0b1011*/
    p(-5, 4),    /*0b1100*/
    p(-38, -14), /*0b1101*/
    p(-4, 16),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-9, 15),   /*0b10000*/
    p(4, 9),     /*0b10001*/
    p(-7, -14),  /*0b10010*/
    p(-8, -2),   /*0b10011*/
    p(-7, 5),    /*0b10100*/
    p(10, 13),   /*0b10101*/
    p(-26, -9),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(12, 44),   /*0b11000*/
    p(21, 9),    /*0b11001*/
    p(26, 22),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(14, 18),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(12, 8),    /*0b100000*/
    p(-1, 11),   /*0b100001*/
    p(13, 2),    /*0b100010*/
    p(10, 13),   /*0b100011*/
    p(-29, -22), /*0b100100*/
    p(-39, -32), /*0b100101*/
    p(-32, 6),   /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(12, -1),   /*0b101000*/
    p(-22, -5),  /*0b101001*/
    p(14, -5),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-28, -18), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(20, 30),   /*0b110000*/
    p(32, 21),   /*0b110001*/
    p(15, -10),  /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(-5, 11),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(29, 36),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(-3, -11),  /*0b111111*/
    p(-26, -11), /*0b00*/
    p(6, -29),   /*0b01*/
    p(34, -15),  /*0b10*/
    p(35, -41),  /*0b11*/
    p(40, -21),  /*0b100*/
    p(-23, -60), /*0b101*/
    p(71, -52),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(53, -21),  /*0b1000*/
    p(19, -47),  /*0b1001*/
    p(61, -91),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(65, -17),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(9, -22),   /*0b1111*/
    p(10, -12),  /*0b00*/
    p(27, -24),  /*0b01*/
    p(21, -30),  /*0b10*/
    p(31, -44),  /*0b11*/
    p(28, -21),  /*0b100*/
    p(36, -59),  /*0b101*/
    p(20, -38),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(30, -16),  /*0b1000*/
    p(49, -31),  /*0b1001*/
    p(35, -82),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(46, -21),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(17, -73),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(14, 13), p(3, 9), p(8, 15), p(7, 9), p(-4, 18), p(-43, 9)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(37, 10),
    p(41, 35),
    p(52, -8),
    p(36, -35),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-61, -58),
        p(-37, -20),
        p(-21, 1),
        p(-9, 13),
        p(2, 21),
        p(12, 29),
        p(24, 29),
        p(33, 28),
        p(41, 24),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-28, -35),
        p(-14, -19),
        p(-5, -3),
        p(2, 8),
        p(10, 17),
        p(15, 24),
        p(19, 28),
        p(22, 32),
        p(25, 36),
        p(33, 36),
        p(40, 34),
        p(52, 35),
        p(51, 41),
        p(67, 32),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-77, 17),
        p(-67, 30),
        p(-63, 34),
        p(-60, 38),
        p(-60, 44),
        p(-55, 48),
        p(-52, 52),
        p(-48, 54),
        p(-44, 58),
        p(-40, 61),
        p(-35, 64),
        p(-33, 67),
        p(-24, 67),
        p(-12, 64),
        p(-11, 65),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-39, -34),
        p(-40, 21),
        p(-43, 67),
        p(-38, 83),
        p(-35, 101),
        p(-30, 107),
        p(-26, 117),
        p(-22, 124),
        p(-18, 129),
        p(-14, 131),
        p(-11, 134),
        p(-7, 137),
        p(-4, 138),
        p(-3, 142),
        p(-0, 144),
        p(3, 146),
        p(4, 152),
        p(7, 151),
        p(16, 149),
        p(30, 140),
        p(34, 140),
        p(77, 116),
        p(77, 117),
        p(100, 98),
        p(191, 63),
        p(247, 17),
        p(288, -1),
        p(335, -34),
    ],
    [
        p(-83, 53),
        p(-51, 23),
        p(-25, 12),
        p(2, 4),
        p(29, -3),
        p(48, -11),
        p(70, -11),
        p(88, -18),
        p(132, -44),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [
        p(-11, 12),
        p(-8, -5),
        p(24, 17),
        p(48, -14),
        p(21, -45),
        p(0, 0),
    ],
    [p(-3, 15), p(20, 20), p(-1, 6), p(29, 2), p(28, 56), p(0, 0)],
    [
        p(3, 18),
        p(21, 20),
        p(23, 21),
        p(-7, 10),
        p(43, -5),
        p(0, 0),
    ],
    [p(-0, -1), p(7, 12), p(-0, 29), p(0, 6), p(1, -18), p(0, 0)],
    [p(77, 35), p(-30, 21), p(3, 19), p(-33, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(11, 4), p(9, 10), p(16, 5), p(10, 16), p(13, 2)],
    [
        p(-0, -5),
        p(10, 17),
        p(-90, -40),
        p(7, 12),
        p(8, 17),
        p(5, 5),
    ],
    [p(1, 1), p(13, 4), p(9, 9), p(11, 7), p(12, 14), p(22, -5)],
    [
        p(3, -4),
        p(10, -2),
        p(9, -8),
        p(4, 15),
        p(-56, -258),
        p(7, -10),
    ],
    [
        p(55, -9),
        p(38, -1),
        p(43, -7),
        p(21, -3),
        p(33, -20),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-11, -10),
    p(16, -8),
    p(18, -3),
    p(22, -13),
    p(5, 22),
    p(2, 20),
];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static {
    type Score: ScoreType;

    fn psqt(
        square: ChessSquare,
        piece: UncoloredChessPiece,
        color: Color,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn passed_pawn(square: ChessSquare) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn bishop_pair() -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn rook_openness(openness: FileOpenness) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn king_openness(openness: FileOpenness) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn pawn_shield(config: usize) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn pawn_protection(
        piece: UncoloredChessPiece,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn pawn_attack(piece: UncoloredChessPiece) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn mobility(
        piece: UncoloredChessPiece,
        mobility: usize,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn threats(
        attacking: UncoloredChessPiece,
        targeted: UncoloredChessPiece,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn defended(
        protecting: UncoloredChessPiece,
        target: UncoloredChessPiece,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn king_zone_attack(
        attacking: UncoloredChessPiece,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;
}

/// Eval values tuned on a combination of the zurichess dataset and a dataset used by 4ku,
/// created by GCP using his engine Stoofvlees and filtered by cj5716 using Stockfish at depth 9,
/// using my own tuner `pliers`.
#[derive(Debug, Default, Copy, Clone)]
pub struct Lite {}

impl LiteValues for Lite {
    type Score = PhasedScore;

    fn psqt(square: ChessSquare, piece: UncoloredChessPiece, color: Color) -> Self::Score {
        PSQTS[piece as usize][square.flip_if(color == White).bb_idx()]
    }

    fn passed_pawn(square: ChessSquare) -> Self::Score {
        PASSED_PAWNS[square.bb_idx()]
    }

    fn bishop_pair() -> Self::Score {
        BISHOP_PAIR
    }

    fn rook_openness(openness: FileOpenness) -> Self::Score {
        match openness {
            FileOpenness::Open => ROOK_OPEN_FILE,
            FileOpenness::Closed => ROOK_CLOSED_FILE,
            FileOpenness::SemiOpen => ROOK_SEMIOPEN_FILE,
            FileOpenness::SemiClosed => Self::Score::default(),
        }
    }

    fn king_openness(openness: FileOpenness) -> Self::Score {
        match openness {
            FileOpenness::Open => KING_OPEN_FILE,
            FileOpenness::Closed => KING_CLOSED_FILE,
            FileOpenness::SemiOpen => KING_SEMIOPEN_FILE,
            FileOpenness::SemiClosed => Self::Score::default(),
        }
    }

    fn pawn_shield(config: usize) -> Self::Score {
        PAWN_SHIELDS[config]
    }

    fn pawn_protection(piece: UncoloredChessPiece) -> Self::Score {
        PAWN_PROTECTION[piece as usize]
    }

    fn pawn_attack(piece: UncoloredChessPiece) -> Self::Score {
        PAWN_ATTACKS[piece as usize]
    }

    fn mobility(piece: UncoloredChessPiece, mobility: usize) -> Self::Score {
        MOBILITY[piece as usize - 1][mobility]
    }

    fn threats(attacking: UncoloredChessPiece, targeted: UncoloredChessPiece) -> Self::Score {
        THREATS[attacking as usize - 1][targeted as usize]
    }
    fn defended(protecting: UncoloredChessPiece, target: UncoloredChessPiece) -> Self::Score {
        DEFENDED[protecting as usize - 1][target as usize]
    }

    fn king_zone_attack(
        attacking: UncoloredChessPiece,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore {
        KING_ZONE_ATTACK[attacking as usize]
    }
}
