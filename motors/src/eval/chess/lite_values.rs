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
        p( 128,  181),    p( 128,  181),    p( 118,  184),    p( 129,  166),    p( 117,  170),    p( 119,  173),    p(  81,  191),    p(  86,  188),
        p(  66,  121),    p(  65,  123),    p(  74,  112),    p(  81,  117),    p(  70,  120),    p( 117,  105),    p(  93,  130),    p(  85,  119),
        p(  51,  110),    p(  64,  104),    p(  60,   96),    p(  63,   88),    p(  79,   90),    p(  81,   88),    p(  75,   99),    p(  69,   92),
        p(  47,   97),    p(  57,   98),    p(  62,   87),    p(  70,   84),    p(  74,   84),    p(  76,   81),    p(  70,   89),    p(  56,   81),
        p(  39,   92),    p(  53,   88),    p(  53,   84),    p(  56,   91),    p(  65,   88),    p(  59,   84),    p(  69,   79),    p(  48,   79),
        p(  51,   96),    p(  63,   97),    p(  59,   90),    p(  57,   97),    p(  63,  101),    p(  74,   92),    p(  85,   85),    p(  52,   83),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 187,  270),    p( 209,  302),    p( 244,  314),    p( 269,  305),    p( 300,  306),    p( 213,  302),    p( 234,  301),    p( 216,  252),
        p( 276,  303),    p( 288,  312),    p( 302,  309),    p( 313,  312),    p( 305,  309),    p( 328,  298),    p( 286,  309),    p( 291,  295),
        p( 292,  301),    p( 302,  306),    p( 317,  315),    p( 320,  318),    p( 336,  312),    p( 360,  302),    p( 313,  303),    p( 309,  299),
        p( 304,  310),    p( 309,  308),    p( 316,  320),    p( 342,  322),    p( 318,  324),    p( 332,  321),    p( 313,  311),    p( 332,  304),
        p( 299,  313),    p( 297,  307),    p( 303,  320),    p( 310,  324),    p( 317,  325),    p( 314,  311),    p( 325,  304),    p( 314,  309),
        p( 274,  300),    p( 275,  303),    p( 282,  303),    p( 288,  317),    p( 295,  313),    p( 281,  298),    p( 296,  294),    p( 292,  304),
        p( 270,  305),    p( 280,  309),    p( 276,  303),    p( 288,  308),    p( 291,  303),    p( 284,  299),    p( 294,  300),    p( 290,  313),
        p( 243,  301),    p( 283,  299),    p( 265,  301),    p( 285,  306),    p( 295,  303),    p( 291,  292),    p( 289,  299),    p( 268,  298),
    ],
    // bishop
    [
        p( 280,  314),    p( 255,  313),    p( 250,  307),    p( 226,  314),    p( 224,  314),    p( 228,  307),    p( 283,  304),    p( 253,  308),
        p( 282,  303),    p( 289,  305),    p( 289,  306),    p( 282,  307),    p( 285,  303),    p( 295,  302),    p( 276,  308),    p( 275,  303),
        p( 300,  309),    p( 304,  304),    p( 295,  310),    p( 302,  302),    p( 306,  305),    p( 333,  308),    p( 320,  303),    p( 314,  312),
        p( 283,  310),    p( 299,  310),    p( 301,  305),    p( 318,  311),    p( 311,  307),    p( 306,  309),    p( 302,  308),    p( 282,  311),
        p( 292,  307),    p( 281,  311),    p( 299,  309),    p( 314,  308),    p( 311,  308),    p( 298,  306),    p( 290,  309),    p( 311,  299),
        p( 294,  306),    p( 304,  309),    p( 301,  309),    p( 304,  309),    p( 307,  310),    p( 303,  307),    p( 305,  302),    p( 311,  300),
        p( 309,  310),    p( 304,  300),    p( 311,  302),    p( 296,  310),    p( 302,  308),    p( 302,  305),    p( 313,  301),    p( 302,  298),
        p( 295,  304),    p( 313,  309),    p( 306,  306),    p( 289,  311),    p( 302,  309),    p( 295,  313),    p( 303,  298),    p( 300,  295),
    ],
    // rook
    [
        p( 459,  549),    p( 449,  559),    p( 447,  565),    p( 445,  562),    p( 457,  558),    p( 477,  553),    p( 485,  551),    p( 496,  543),
        p( 431,  554),    p( 429,  560),    p( 439,  561),    p( 454,  551),    p( 445,  553),    p( 464,  548),    p( 475,  545),    p( 489,  535),
        p( 435,  552),    p( 456,  547),    p( 453,  549),    p( 458,  544),    p( 484,  534),    p( 492,  529),    p( 516,  526),    p( 486,  528),
        p( 434,  551),    p( 444,  547),    p( 444,  550),    p( 449,  545),    p( 458,  537),    p( 467,  531),    p( 473,  533),    p( 469,  528),
        p( 431,  547),    p( 431,  546),    p( 431,  548),    p( 438,  545),    p( 444,  541),    p( 439,  539),    p( 457,  532),    p( 447,  530),
        p( 427,  544),    p( 427,  543),    p( 430,  542),    p( 433,  543),    p( 440,  537),    p( 448,  529),    p( 470,  516),    p( 452,  519),
        p( 430,  539),    p( 434,  539),    p( 440,  540),    p( 443,  538),    p( 451,  532),    p( 465,  522),    p( 474,  517),    p( 442,  525),
        p( 438,  543),    p( 435,  538),    p( 436,  543),    p( 441,  539),    p( 448,  533),    p( 454,  532),    p( 451,  529),    p( 446,  531),
    ],
    // queen
    [
        p( 874,  964),    p( 872,  980),    p( 888,  993),    p( 905,  989),    p( 904,  994),    p( 925,  980),    p( 974,  928),    p( 923,  957),
        p( 883,  956),    p( 859,  988),    p( 860, 1015),    p( 852, 1032),    p( 860, 1043),    p( 899, 1006),    p( 904,  984),    p( 945,  963),
        p( 891,  962),    p( 882,  982),    p( 882, 1005),    p( 879, 1014),    p( 902, 1016),    p( 942,  998),    p( 949,  968),    p( 937,  973),
        p( 876,  977),    p( 880,  988),    p( 874,  998),    p( 873, 1012),    p( 878, 1022),    p( 890, 1013),    p( 899, 1011),    p( 907,  987),
        p( 885,  968),    p( 871,  988),    p( 877,  991),    p( 877, 1009),    p( 879, 1006),    p( 881, 1005),    p( 895,  991),    p( 903,  981),
        p( 881,  954),    p( 886,  971),    p( 879,  987),    p( 876,  991),    p( 881,  998),    p( 888,  987),    p( 902,  968),    p( 902,  955),
        p( 884,  952),    p( 882,  960),    p( 888,  964),    p( 886,  977),    p( 887,  976),    p( 890,  960),    p( 900,  939),    p( 909,  911),
        p( 871,  947),    p( 880,  938),    p( 880,  953),    p( 888,  954),    p( 890,  948),    p( 879,  948),    p( 880,  936),    p( 884,  924),
    ],
    // king
    [
        p( 149, -105),    p(  57,  -51),    p(  80,  -42),    p(   3,  -10),    p(  24,  -23),    p(   6,  -13),    p(  60,  -23),    p( 216, -108),
        p( -22,   -5),    p( -63,   25),    p( -74,   36),    p(  -8,   25),    p( -40,   33),    p( -71,   46),    p( -33,   30),    p(  12,   -3),
        p( -43,    4),    p( -32,   22),    p( -78,   40),    p( -84,   48),    p( -50,   41),    p( -19,   34),    p( -55,   32),    p( -31,    9),
        p( -28,   -2),    p( -86,   22),    p(-104,   39),    p(-126,   49),    p(-125,   46),    p(-104,   38),    p(-110,   27),    p( -98,   14),
        p( -46,   -5),    p(-108,   18),    p(-119,   34),    p(-140,   48),    p(-146,   46),    p(-122,   31),    p(-134,   22),    p(-115,   11),
        p( -36,   -1),    p( -83,   13),    p(-112,   27),    p(-121,   37),    p(-115,   35),    p(-128,   27),    p( -98,   12),    p( -71,    9),
        p(  29,   -9),    p( -65,    7),    p( -78,   16),    p(-100,   26),    p(-104,   26),    p( -90,   17),    p( -55,   -0),    p(   6,   -4),
        p(  39,  -44),    p(  40,  -49),    p(  35,  -35),    p( -27,  -14),    p(  27,  -32),    p( -22,  -19),    p(  34,  -44),    p(  58,  -53),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  28,   81),    p(  28,   81),    p(  18,   84),    p(  29,   66),    p(  17,   70),    p(  19,   73),    p( -19,   91),    p( -14,   88),
        p(  29,  116),    p(  37,  115),    p(  31,   98),    p(  16,   66),    p(  27,   64),    p(  10,   92),    p(  -7,   96),    p( -35,  118),
        p(  11,   67),    p(   8,   67),    p(  18,   52),    p(  11,   44),    p(  -5,   44),    p(   1,   56),    p( -16,   71),    p( -18,   73),
        p(  -3,   41),    p( -12,   39),    p( -21,   34),    p( -12,   26),    p( -21,   29),    p( -18,   37),    p( -25,   50),    p( -18,   46),
        p(  -7,   10),    p( -21,   20),    p( -20,   18),    p( -18,    8),    p( -17,   13),    p( -15,   17),    p( -19,   33),    p(   3,   14),
        p( -15,   10),    p( -12,   14),    p( -13,   17),    p( -10,    4),    p(   1,    0),    p(   0,    6),    p(   5,   13),    p(   1,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(15, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -4);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(-2, -1);
const KING_OPEN_FILE: PhasedScore = p(-59, -4);
const KING_CLOSED_FILE: PhasedScore = p(14, -16);
const KING_SEMIOPEN_FILE: PhasedScore = p(-11, 2);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 5), p(-4, 6), p(-5, 8), p(2, 7), p(3, 9), p(3, 11), p(9, 10), p(21, 6), ],
    // Closed
    [p(0, 0), p(0, 0), p(17, -29), p(-13, 10), p(-1, 13), p(3, 4), p(2, 10), p(0, 6), ],
    // SemiOpen
    [p(0, 0), p(-15, 21), p(-0, 18), p(1, 14), p(-2, 18), p(3, 14), p(1, 11), p(11, 11), ],
    // SemiClosed
    [p(0, 0), p(12, -13), p(9, 7), p(6, 2), p(8, 5), p(4, 4), p(7, 7), p(2, 4), ],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-12, 5),   /*0b0000*/
    p(-18, 10),  /*0b0001*/
    p(-12, 6),   /*0b0010*/
    p(-6, 25),   /*0b0011*/
    p(-6, 6),    /*0b0100*/
    p(-29, 1),   /*0b0101*/
    p(-11, 16),  /*0b0110*/
    p(-10, -0),  /*0b0111*/
    p(2, 8),     /*0b1000*/
    p(-23, -12), /*0b1001*/
    p(-6, 8),    /*0b1010*/
    p(-6, 1),    /*0b1011*/
    p(-5, 4),    /*0b1100*/
    p(-40, -14), /*0b1101*/
    p(-6, 17),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-8, 15),   /*0b10000*/
    p(4, 10),    /*0b10001*/
    p(-7, -14),  /*0b10010*/
    p(-8, -2),   /*0b10011*/
    p(-7, 5),    /*0b10100*/
    p(9, 13),    /*0b10101*/
    p(-26, -9),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(14, 43),   /*0b11000*/
    p(23, 8),    /*0b11001*/
    p(26, 22),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(17, 17),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(11, 8),    /*0b100000*/
    p(-3, 11),   /*0b100001*/
    p(15, 1),    /*0b100010*/
    p(12, 12),   /*0b100011*/
    p(-29, -22), /*0b100100*/
    p(-39, -32), /*0b100101*/
    p(-28, 5),   /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(13, -1),   /*0b101000*/
    p(-21, -5),  /*0b101001*/
    p(14, -5),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-27, -19), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(18, 30),   /*0b110000*/
    p(28, 22),   /*0b110001*/
    p(17, -11),  /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(-3, 11),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(31, 34),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(-0, -12),  /*0b111111*/
    p(-26, -11), /*0b00*/
    p(6, -29),   /*0b01*/
    p(33, -15),  /*0b10*/
    p(33, -40),  /*0b11*/
    p(40, -21),  /*0b100*/
    p(-21, -61), /*0b101*/
    p(72, -53),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(53, -21),  /*0b1000*/
    p(18, -47),  /*0b1001*/
    p(62, -91),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(65, -17),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(7, -20),   /*0b1111*/
    p(11, -12),  /*0b00*/
    p(28, -25),  /*0b01*/
    p(22, -30),  /*0b10*/
    p(29, -43),  /*0b11*/
    p(28, -21),  /*0b100*/
    p(36, -58),  /*0b101*/
    p(20, -38),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(30, -16),  /*0b1000*/
    p(52, -32),  /*0b1001*/
    p(35, -82),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(44, -21),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(17, -72),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = [
    p(15, 13),
    p(3, 10),
    p(10, 14),
    p(7, 9),
    p(-4, 18),
    p(-43, 10),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(38, 10),
    p(41, 37),
    p(51, -8),
    p(36, -35),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-60, -58),
        p(-37, -20),
        p(-21, 1),
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
        p(-28, -48),
        p(-15, -31),
        p(-4, -15),
        p(2, -3),
        p(9, 6),
        p(13, 14),
        p(16, 18),
        p(18, 22),
        p(19, 26),
        p(25, 26),
        p(29, 25),
        p(38, 26),
        p(30, 33),
        p(43, 25),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-10, 63),
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
        p(-38, -35),
        p(-39, 20),
        p(-43, 69),
        p(-38, 84),
        p(-35, 102),
        p(-30, 107),
        p(-25, 118),
        p(-22, 124),
        p(-17, 128),
        p(-14, 130),
        p(-11, 134),
        p(-7, 136),
        p(-4, 137),
        p(-2, 142),
        p(0, 143),
        p(4, 145),
        p(5, 151),
        p(7, 150),
        p(16, 148),
        p(31, 139),
        p(35, 139),
        p(78, 115),
        p(78, 116),
        p(101, 96),
        p(193, 62),
        p(247, 16),
        p(291, -4),
        p(335, -35),
    ],
    [
        p(-82, 53),
        p(-50, 23),
        p(-24, 12),
        p(2, 4),
        p(29, -3),
        p(48, -11),
        p(70, -10),
        p(88, -18),
        p(131, -44),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(48, -14),
        p(21, -44),
        p(0, 0),
    ],
    [p(-2, 13), p(18, 21), p(-2, 7), p(28, 2), p(27, 56), p(0, 0)],
    [
        p(3, 18),
        p(21, 20),
        p(23, 21),
        p(-7, 10),
        p(43, -5),
        p(0, 0),
    ],
    [p(-0, -1), p(7, 12), p(-1, 29), p(0, 6), p(1, -18), p(0, 0)],
    [p(77, 35), p(-30, 21), p(3, 19), p(-33, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(11, 4), p(9, 10), p(16, 5), p(10, 16), p(13, 2)],
    [
        p(-3, 0),
        p(8, 17),
        p(-101, -37),
        p(6, 12),
        p(7, 16),
        p(5, 5),
    ],
    [p(2, 1), p(14, 3), p(9, 9), p(11, 7), p(12, 14), p(22, -6)],
    [
        p(3, -4),
        p(10, -3),
        p(9, -9),
        p(4, 15),
        p(-55, -259),
        p(7, -11),
    ],
    [
        p(55, -10),
        p(37, -1),
        p(43, -6),
        p(21, -3),
        p(33, -19),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-11, -10),
    p(16, -8),
    p(17, -3),
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
}
