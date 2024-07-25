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
        p( 128,  182),    p( 128,  181),    p( 118,  184),    p( 129,  166),    p( 117,  170),    p( 118,  173),    p(  79,  191),    p(  88,  188),
        p(  66,  121),    p(  66,  123),    p(  76,  111),    p(  83,  116),    p(  72,  117),    p( 120,  103),    p(  94,  127),    p(  87,  117),
        p(  52,  109),    p(  65,  103),    p(  60,   95),    p(  64,   86),    p(  80,   89),    p(  83,   85),    p(  74,   98),    p(  69,   91),
        p(  48,   96),    p(  59,   97),    p(  63,   85),    p(  72,   82),    p(  76,   82),    p(  78,   79),    p(  72,   87),    p(  58,   80),
        p(  40,   91),    p(  55,   87),    p(  54,   82),    p(  58,   89),    p(  68,   86),    p(  64,   80),    p(  72,   76),    p(  52,   76),
        p(  52,   96),    p(  64,   96),    p(  61,   88),    p(  60,   94),    p(  64,   98),    p(  79,   88),    p(  88,   81),    p(  56,   79),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 188,  270),    p( 209,  302),    p( 244,  314),    p( 268,  305),    p( 301,  306),    p( 214,  301),    p( 235,  301),    p( 217,  252),
        p( 276,  303),    p( 288,  312),    p( 302,  309),    p( 313,  312),    p( 306,  308),    p( 329,  297),    p( 287,  309),    p( 291,  295),
        p( 291,  301),    p( 303,  306),    p( 317,  315),    p( 321,  318),    p( 336,  312),    p( 360,  302),    p( 313,  303),    p( 309,  299),
        p( 304,  310),    p( 309,  309),    p( 316,  320),    p( 342,  322),    p( 319,  323),    p( 332,  320),    p( 314,  311),    p( 332,  304),
        p( 299,  313),    p( 298,  307),    p( 303,  320),    p( 310,  324),    p( 317,  325),    p( 314,  311),    p( 325,  303),    p( 314,  309),
        p( 274,  300),    p( 275,  303),    p( 282,  303),    p( 288,  317),    p( 295,  313),    p( 281,  298),    p( 296,  295),    p( 292,  303),
        p( 270,  305),    p( 280,  309),    p( 276,  304),    p( 289,  309),    p( 292,  303),    p( 284,  300),    p( 294,  300),    p( 290,  313),
        p( 244,  301),    p( 283,  299),    p( 266,  301),    p( 285,  306),    p( 296,  303),    p( 291,  293),    p( 289,  299),    p( 268,  298),
    ],
    // bishop
    [
        p( 280,  314),    p( 255,  313),    p( 251,  307),    p( 226,  314),    p( 224,  313),    p( 228,  307),    p( 284,  303),    p( 253,  308),
        p( 281,  303),    p( 289,  306),    p( 289,  306),    p( 282,  307),    p( 285,  302),    p( 297,  301),    p( 276,  307),    p( 276,  303),
        p( 301,  309),    p( 305,  304),    p( 295,  310),    p( 302,  302),    p( 307,  305),    p( 333,  308),    p( 319,  303),    p( 314,  311),
        p( 283,  310),    p( 299,  309),    p( 301,  305),    p( 318,  310),    p( 311,  307),    p( 306,  309),    p( 302,  308),    p( 283,  310),
        p( 292,  307),    p( 281,  311),    p( 299,  309),    p( 314,  308),    p( 311,  308),    p( 298,  307),    p( 290,  309),    p( 311,  299),
        p( 294,  306),    p( 304,  309),    p( 301,  309),    p( 304,  309),    p( 307,  310),    p( 303,  307),    p( 306,  301),    p( 311,  300),
        p( 309,  310),    p( 304,  300),    p( 311,  302),    p( 296,  310),    p( 302,  308),    p( 302,  305),    p( 313,  301),    p( 302,  298),
        p( 295,  304),    p( 314,  309),    p( 306,  306),    p( 289,  311),    p( 302,  309),    p( 295,  313),    p( 303,  298),    p( 300,  295),
    ],
    // rook
    [
        p( 460,  549),    p( 450,  558),    p( 448,  564),    p( 447,  561),    p( 459,  557),    p( 479,  552),    p( 487,  550),    p( 498,  543),
        p( 432,  554),    p( 430,  560),    p( 440,  560),    p( 456,  550),    p( 446,  552),    p( 467,  547),    p( 477,  544),    p( 489,  535),
        p( 435,  552),    p( 457,  547),    p( 453,  549),    p( 458,  544),    p( 484,  533),    p( 494,  528),    p( 517,  526),    p( 487,  528),
        p( 434,  552),    p( 444,  547),    p( 444,  550),    p( 450,  545),    p( 458,  537),    p( 468,  531),    p( 473,  533),    p( 469,  529),
        p( 430,  548),    p( 431,  546),    p( 431,  548),    p( 439,  545),    p( 444,  541),    p( 439,  539),    p( 458,  532),    p( 447,  531),
        p( 427,  544),    p( 427,  543),    p( 430,  542),    p( 433,  543),    p( 441,  537),    p( 448,  529),    p( 471,  516),    p( 452,  520),
        p( 430,  539),    p( 434,  539),    p( 440,  541),    p( 444,  538),    p( 451,  532),    p( 465,  522),    p( 473,  518),    p( 442,  526),
        p( 439,  543),    p( 435,  539),    p( 436,  544),    p( 441,  539),    p( 448,  534),    p( 454,  533),    p( 450,  530),    p( 446,  531),
    ],
    // queen
    [
        p( 873,  965),    p( 873,  980),    p( 889,  994),    p( 905,  989),    p( 904,  994),    p( 926,  979),    p( 975,  928),    p( 924,  957),
        p( 883,  957),    p( 859,  989),    p( 860, 1016),    p( 852, 1033),    p( 860, 1043),    p( 902, 1004),    p( 905,  983),    p( 946,  962),
        p( 891,  962),    p( 882,  982),    p( 881, 1006),    p( 880, 1014),    p( 903, 1015),    p( 943,  998),    p( 950,  968),    p( 937,  973),
        p( 876,  977),    p( 880,  989),    p( 874,  998),    p( 873, 1012),    p( 878, 1022),    p( 891, 1013),    p( 900, 1011),    p( 907,  987),
        p( 885,  968),    p( 872,  988),    p( 877,  991),    p( 878, 1010),    p( 879, 1006),    p( 881, 1006),    p( 895,  991),    p( 903,  981),
        p( 881,  954),    p( 886,  971),    p( 879,  988),    p( 876,  991),    p( 881,  998),    p( 888,  988),    p( 903,  969),    p( 902,  956),
        p( 884,  952),    p( 882,  961),    p( 888,  964),    p( 887,  977),    p( 887,  977),    p( 890,  959),    p( 901,  939),    p( 909,  912),
        p( 871,  947),    p( 881,  939),    p( 880,  954),    p( 889,  954),    p( 890,  948),    p( 879,  948),    p( 880,  937),    p( 884,  925),
    ],
    // king
    [
        p( 149,  -97),    p(  56,  -47),    p(  82,  -38),    p(   6,   -7),    p(  25,  -19),    p(   1,   -9),    p(  57,  -19),    p( 211, -101),
        p( -25,   -4),    p( -67,   19),    p( -74,   29),    p( -10,   19),    p( -41,   27),    p( -74,   41),    p( -42,   26),    p(   1,   -1),
        p( -46,    4),    p( -33,   14),    p( -75,   32),    p( -83,   41),    p( -49,   36),    p( -14,   26),    p( -57,   26),    p( -34,    8),
        p( -27,   -3),    p( -85,   13),    p(-102,   32),    p(-122,   42),    p(-121,   40),    p( -97,   32),    p(-100,   18),    p( -96,   12),
        p( -41,   -7),    p(-103,    9),    p(-116,   27),    p(-139,   41),    p(-142,   39),    p(-116,   25),    p(-126,   15),    p(-112,    9),
        p( -32,   -3),    p( -79,    5),    p(-109,   21),    p(-118,   29),    p(-114,   28),    p(-124,   22),    p( -96,    7),    p( -68,    7),
        p(  31,  -10),    p( -66,    3),    p( -78,   11),    p( -97,   19),    p(-102,   20),    p( -90,   13),    p( -59,   -1),    p(   7,   -5),
        p(  42,  -35),    p(  40,  -39),    p(  34,  -25),    p( -25,   -8),    p(  29,  -25),    p( -22,   -9),    p(  30,  -31),    p(  60,  -45),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  28,   82),    p(  28,   81),    p(  18,   84),    p(  29,   66),    p(  17,   70),    p(  18,   73),    p( -21,   91),    p( -12,   88),
        p(  28,  116),    p(  37,  116),    p(  30,   99),    p(  16,   67),    p(  26,   67),    p(   7,   94),    p( -10,   99),    p( -36,  121),
        p(  11,   68),    p(   9,   67),    p(  18,   53),    p(  11,   45),    p(  -4,   45),    p(   1,   57),    p( -16,   72),    p( -18,   75),
        p(  -4,   41),    p( -12,   40),    p( -21,   34),    p( -13,   26),    p( -20,   30),    p( -19,   37),    p( -26,   51),    p( -18,   47),
        p(  -7,   11),    p( -20,   20),    p( -20,   18),    p( -18,    9),    p( -16,   13),    p( -17,   18),    p( -18,   32),    p(   2,   14),
        p( -15,   10),    p( -11,   14),    p( -12,   16),    p( -10,    5),    p(   2,    0),    p(  -0,    7),    p(   6,   12),    p(   1,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(15, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -4);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(-2, -1);
const KING_OPEN_FILE: PhasedScore = p(-57, -3);
const KING_CLOSED_FILE: PhasedScore = p(8, -10);
const KING_SEMIOPEN_FILE: PhasedScore = p(-13, 4);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 5), p(-4, 6), p(-5, 8), p(2, 7), p(3, 9), p(3, 11), p(9, 10), p(21, 6), ],
    // Closed
    [p(0, 0), p(0, 0), p(17, -28), p(-12, 10), p(-2, 13), p(3, 4), p(2, 10), p(1, 6), ],
    // SemiOpen
    [p(0, 0), p(-16, 22), p(-0, 18), p(1, 14), p(-2, 18), p(3, 14), p(1, 11), p(12, 11), ],
    // SemiClosed
    [p(0, 0), p(12, -13), p(9, 7), p(6, 2), p(8, 5), p(4, 4), p(8, 7), p(3, 4), ],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-13, 5),   /*0b0000*/
    p(-17, 8),   /*0b0001*/
    p(-9, 4),    /*0b0010*/
    p(-3, 23),   /*0b0011*/
    p(-6, 3),    /*0b0100*/
    p(-28, -3),  /*0b0101*/
    p(-8, 14),   /*0b0110*/
    p(-8, -2),   /*0b0111*/
    p(-1, 9),    /*0b1000*/
    p(-25, -12), /*0b1001*/
    p(-6, 7),    /*0b1010*/
    p(-4, -1),   /*0b1011*/
    p(-7, 5),    /*0b1100*/
    p(-44, -12), /*0b1101*/
    p(-6, 17),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-9, 14),   /*0b10000*/
    p(3, 8),     /*0b10001*/
    p(-6, -16),  /*0b10010*/
    p(-7, -3),   /*0b10011*/
    p(-7, 4),    /*0b10100*/
    p(8, 12),    /*0b10101*/
    p(-25, -11), /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(9, 45),    /*0b11000*/
    p(18, 11),   /*0b11001*/
    p(23, 24),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(13, 20),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(7, 9),     /*0b100000*/
    p(-6, 12),   /*0b100001*/
    p(15, 0),    /*0b100010*/
    p(11, 12),   /*0b100011*/
    p(-34, -21), /*0b100100*/
    p(-43, -32), /*0b100101*/
    p(-30, 6),   /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(7, 3),     /*0b101000*/
    p(-28, -2),  /*0b101001*/
    p(10, -4),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-35, -14), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(13, 32),   /*0b110000*/
    p(23, 24),   /*0b110001*/
    p(15, -9),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(-10, 15),  /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(23, 40),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(-5, -9),   /*0b111111*/
    p(-26, -9),  /*0b00*/
    p(13, -29),  /*0b01*/
    p(36, -15),  /*0b10*/
    p(41, -40),  /*0b11*/
    p(44, -17),  /*0b100*/
    p(-14, -61), /*0b101*/
    p(77, -50),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(51, -17),  /*0b1000*/
    p(23, -44),  /*0b1001*/
    p(63, -88),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(63, -10),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(10, -15),  /*0b1111*/
    p(15, -9),   /*0b00*/
    p(32, -22),  /*0b01*/
    p(29, -28),  /*0b10*/
    p(35, -40),  /*0b11*/
    p(29, -15),  /*0b100*/
    p(36, -51),  /*0b101*/
    p(23, -33),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(33, -11),  /*0b1000*/
    p(55, -28),  /*0b1001*/
    p(39, -78),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(43, -13),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(17, -64),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = [
    p(15, 14),
    p(3, 10),
    p(10, 14),
    p(8, 9),
    p(-4, 18),
    p(-41, 8),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(38, 10),
    p(41, 36),
    p(52, -8),
    p(37, -35),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-60, -59),
        p(-37, -20),
        p(-21, 2),
        p(-9, 13),
        p(2, 21),
        p(13, 29),
        p(24, 29),
        p(34, 28),
        p(42, 24),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-28, -49),
        p(-15, -31),
        p(-4, -15),
        p(2, -3),
        p(9, 6),
        p(13, 14),
        p(16, 18),
        p(18, 22),
        p(19, 26),
        p(26, 26),
        p(29, 25),
        p(38, 26),
        p(30, 34),
        p(44, 25),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-75, 17),
        p(-66, 29),
        p(-62, 33),
        p(-58, 38),
        p(-59, 44),
        p(-53, 48),
        p(-50, 52),
        p(-46, 54),
        p(-42, 58),
        p(-39, 61),
        p(-34, 63),
        p(-31, 67),
        p(-22, 67),
        p(-9, 63),
        p(-9, 64),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-38, -34),
        p(-38, 22),
        p(-42, 70),
        p(-37, 85),
        p(-35, 103),
        p(-29, 108),
        p(-25, 118),
        p(-21, 125),
        p(-17, 129),
        p(-13, 131),
        p(-10, 135),
        p(-6, 137),
        p(-3, 138),
        p(-2, 142),
        p(1, 144),
        p(4, 146),
        p(5, 152),
        p(8, 151),
        p(17, 148),
        p(32, 139),
        p(36, 139),
        p(79, 115),
        p(79, 116),
        p(102, 97),
        p(194, 62),
        p(248, 16),
        p(296, -6),
        p(339, -37),
    ],
    [
        p(-83, 36),
        p(-51, 11),
        p(-25, 4),
        p(2, 0),
        p(30, -3),
        p(50, -8),
        p(73, -4),
        p(92, -9),
        p(134, -30),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(23, 17),
        p(49, -14),
        p(21, -44),
        p(0, 0),
    ],
    [p(-2, 13), p(18, 21), p(-2, 7), p(29, 2), p(27, 56), p(0, 0)],
    [
        p(3, 18),
        p(21, 20),
        p(23, 21),
        p(-7, 10),
        p(43, -5),
        p(0, 0),
    ],
    [p(-0, -1), p(7, 12), p(-1, 29), p(0, 6), p(1, -18), p(0, 0)],
    [p(63, 28), p(-30, 21), p(3, 19), p(-32, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(11, 4), p(9, 10), p(16, 5), p(10, 16), p(13, 2)],
    [
        p(-3, 0),
        p(8, 17),
        p(-103, -34),
        p(6, 12),
        p(7, 16),
        p(4, 5),
    ],
    [p(2, 1), p(13, 4), p(9, 9), p(11, 7), p(12, 14), p(22, -5)],
    [
        p(3, -4),
        p(10, -2),
        p(9, -9),
        p(4, 15),
        p(-56, -259),
        p(7, -10),
    ],
    [
        p(52, -3),
        p(38, 2),
        p(44, -3),
        p(22, -0),
        p(33, -15),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-11, -13),
    p(16, -8),
    p(17, -2),
    p(20, -11),
    p(5, 23),
    p(1, 16),
];
const PAWN_STORM: [PhasedScore; 8] = [
    p(-29, 3),
    p(10, -17),
    p(-29, -25),
    p(-10, 3),
    p(-2, 5),
    p(-10, 7),
    p(-15, 14),
    p(-13, 15),
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

    fn bishop_openness(
        openness: FileOpenness,
        len: usize,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

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

    fn pawn_storm(rank_diff: usize) -> <Self::Score as ScoreType>::SingleFeatureScore;
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

    fn bishop_openness(
        openness: FileOpenness,
        len: usize,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore {
        BISHOP_OPENNESS[openness as usize][len - 1]
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

    fn pawn_storm(rank_diff: usize) -> <Self::Score as ScoreType>::SingleFeatureScore {
        PAWN_STORM[rank_diff]
    }
}
