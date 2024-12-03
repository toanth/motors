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
use crate::eval::{ScoreType, SingleFeatureScore};
use gears::games::chess::pieces::{ChessPieceType, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::chess::ChessColor;
use gears::games::chess::ChessColor::White;
use gears::general::common::StaticallyNamedEntity;
use gears::score::{p, PhasedScore};
use std::fmt::{Debug, Display};

#[rustfmt::skip]
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
        p( 131,  187),    p( 128,  186),    p( 119,  189),    p( 130,  169),    p( 117,  174),    p( 116,  177),    p(  79,  195),    p(  86,  193),
        p(  63,  123),    p(  60,  125),    p(  73,  120),    p(  81,  124),    p(  69,  123),    p( 117,  111),    p(  91,  131),    p(  88,  122),
        p(  49,  113),    p(  62,  109),    p(  59,  104),    p(  62,   98),    p(  78,   99),    p(  82,   94),    p(  75,  104),    p(  69,   95),
        p(  46,  100),    p(  53,  102),    p(  63,   95),    p(  71,   93),    p(  75,   93),    p(  75,   88),    p(  68,   93),    p(  57,   86),
        p(  42,   97),    p(  51,   93),    p(  54,   94),    p(  58,  100),    p(  65,   96),    p(  60,   92),    p(  68,   83),    p(  52,   85),
        p(  47,   99),    p(  50,   96),    p(  56,   98),    p(  55,  105),    p(  52,  107),    p(  69,   98),    p(  71,   84),    p(  53,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 177,  274),    p( 197,  307),    p( 214,  318),    p( 251,  308),    p( 283,  309),    p( 197,  305),    p( 215,  304),    p( 205,  257),
        p( 269,  307),    p( 284,  314),    p( 297,  307),    p( 301,  310),    p( 301,  305),    p( 314,  295),    p( 276,  310),    p( 271,  300),
        p( 286,  304),    p( 302,  303),    p( 303,  312),    p( 317,  315),    p( 335,  308),    p( 347,  298),    p( 291,  302),    p( 286,  304),
        p( 300,  313),    p( 306,  309),    p( 320,  315),    p( 324,  321),    p( 321,  319),    p( 317,  318),    p( 308,  312),    p( 317,  308),
        p( 297,  315),    p( 301,  306),    p( 310,  315),    p( 318,  317),    p( 316,  321),    p( 321,  304),    p( 320,  303),    p( 312,  311),
        p( 274,  302),    p( 279,  302),    p( 292,  298),    p( 298,  312),    p( 304,  309),    p( 292,  292),    p( 299,  293),    p( 292,  305),
        p( 269,  310),    p( 280,  313),    p( 282,  304),    p( 292,  309),    p( 296,  303),    p( 286,  301),    p( 293,  305),    p( 288,  319),
        p( 238,  308),    p( 280,  303),    p( 265,  304),    p( 285,  310),    p( 294,  308),    p( 290,  298),    p( 286,  306),    p( 263,  305),
    ],
    // bishop
    [
        p( 280,  315),    p( 256,  314),    p( 241,  307),    p( 220,  316),    p( 217,  316),    p( 222,  307),    p( 279,  306),    p( 253,  309),
        p( 285,  303),    p( 281,  305),    p( 289,  306),    p( 278,  307),    p( 287,  302),    p( 292,  301),    p( 268,  307),    p( 272,  304),
        p( 296,  310),    p( 306,  304),    p( 292,  310),    p( 307,  302),    p( 305,  305),    p( 334,  307),    p( 317,  303),    p( 316,  311),
        p( 283,  311),    p( 290,  311),    p( 304,  304),    p( 308,  311),    p( 307,  307),    p( 300,  309),    p( 295,  310),    p( 279,  311),
        p( 289,  310),    p( 282,  310),    p( 296,  306),    p( 310,  307),    p( 305,  305),    p( 298,  305),    p( 287,  308),    p( 308,  303),
        p( 294,  309),    p( 301,  307),    p( 300,  307),    p( 302,  306),    p( 307,  308),    p( 303,  303),    p( 307,  299),    p( 308,  301),
        p( 307,  312),    p( 303,  300),    p( 310,  302),    p( 298,  309),    p( 303,  308),    p( 304,  305),    p( 312,  299),    p( 307,  298),
        p( 296,  305),    p( 313,  308),    p( 307,  307),    p( 291,  313),    p( 304,  310),    p( 295,  315),    p( 306,  299),    p( 302,  296),
    ],
    // rook
    [
        p( 449,  548),    p( 437,  558),    p( 428,  566),    p( 428,  563),    p( 440,  558),    p( 467,  551),    p( 477,  549),    p( 486,  542),
        p( 443,  552),    p( 441,  557),    p( 450,  558),    p( 465,  549),    p( 451,  552),    p( 466,  546),    p( 475,  544),    p( 489,  534),
        p( 445,  547),    p( 462,  542),    p( 457,  544),    p( 456,  539),    p( 484,  529),    p( 492,  526),    p( 507,  526),    p( 485,  527),
        p( 441,  548),    p( 446,  544),    p( 446,  546),    p( 452,  540),    p( 456,  532),    p( 467,  528),    p( 466,  532),    p( 467,  527),
        p( 434,  546),    p( 433,  544),    p( 434,  544),    p( 440,  541),    p( 446,  537),    p( 440,  536),    p( 453,  530),    p( 447,  528),
        p( 430,  543),    p( 430,  540),    p( 432,  539),    p( 436,  538),    p( 442,  532),    p( 452,  524),    p( 468,  513),    p( 454,  517),
        p( 431,  538),    p( 436,  537),    p( 441,  538),    p( 444,  535),    p( 452,  528),    p( 466,  518),    p( 472,  513),    p( 441,  523),
        p( 441,  542),    p( 438,  537),    p( 440,  541),    p( 444,  535),    p( 450,  528),    p( 456,  528),    p( 452,  527),    p( 448,  529),
    ],
    // queen
    [
        p( 872,  956),    p( 875,  970),    p( 886,  985),    p( 909,  977),    p( 908,  981),    p( 934,  966),    p( 974,  921),    p( 919,  953),
        p( 895,  943),    p( 869,  973),    p( 871,  998),    p( 863, 1016),    p( 872, 1026),    p( 910,  988),    p( 912,  971),    p( 953,  953),
        p( 897,  952),    p( 890,  967),    p( 888,  990),    p( 892,  996),    p( 904, 1003),    p( 951,  982),    p( 956,  955),    p( 948,  960),
        p( 884,  964),    p( 886,  975),    p( 884,  981),    p( 882,  994),    p( 885, 1006),    p( 901,  995),    p( 908,  997),    p( 918,  973),
        p( 891,  959),    p( 881,  975),    p( 886,  975),    p( 887,  992),    p( 890,  989),    p( 893,  989),    p( 906,  977),    p( 913,  970),
        p( 888,  944),    p( 894,  958),    p( 889,  974),    p( 887,  978),    p( 893,  984),    p( 900,  973),    p( 913,  956),    p( 912,  945),
        p( 888,  945),    p( 888,  954),    p( 896,  956),    p( 894,  971),    p( 896,  971),    p( 898,  952),    p( 908,  931),    p( 917,  904),
        p( 876,  945),    p( 886,  935),    p( 887,  948),    p( 896,  949),    p( 898,  940),    p( 885,  943),    p( 887,  935),    p( 893,  916),
    ],
    // king
    [
        p( 157,  -85),    p(  63,  -39),    p(  83,  -31),    p(  10,    0),    p(  35,  -11),    p(  20,   -2),    p(  74,  -11),    p( 229,  -87),
        p( -31,    1),    p( -80,   19),    p( -82,   26),    p( -25,   17),    p( -55,   24),    p( -82,   39),    p( -50,   24),    p(   3,    1),
        p( -46,    9),    p( -49,   13),    p( -86,   28),    p( -95,   36),    p( -65,   31),    p( -32,   23),    p( -77,   25),    p( -36,   11),
        p( -27,    1),    p(-100,   12),    p(-114,   28),    p(-137,   37),    p(-137,   35),    p(-113,   27),    p(-132,   17),    p(-103,   17),
        p( -40,   -2),    p(-116,    8),    p(-126,   25),    p(-151,   38),    p(-153,   36),    p(-128,   22),    p(-144,   13),    p(-116,   13),
        p( -34,    2),    p( -92,    3),    p(-121,   18),    p(-127,   27),    p(-124,   26),    p(-134,   18),    p(-110,    4),    p( -73,   10),
        p(  25,   -8),    p( -80,   -2),    p( -91,    7),    p(-109,   16),    p(-116,   17),    p(-100,    8),    p( -74,   -9),    p(   4,   -4),
        p(  54,  -24),    p(  42,  -36),    p(  39,  -24),    p( -21,   -3),    p(  31,  -20),    p( -18,   -7),    p(  35,  -30),    p(  67,  -34),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 52);
const ROOK_OPEN_FILE: PhasedScore = p(13, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 4);
const KING_OPEN_FILE: PhasedScore = p(-44, -2);
const KING_CLOSED_FILE: PhasedScore = p(14, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 7);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 6), p(-1, 7), p(-0, 8), p(3, 6), p(3, 8), p(4, 10), p(7, 9), p(18, 4)],
    // Closed
    [p(0, 0), p(0, 0), p(13, -20), p(-15, 11), p(-0, 12), p(1, 4), p(1, 8), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-17, 25), p(4, 21), p(2, 13), p(0, 15), p(5, 10), p(1, 8), p(10, 9)],
    // SemiClosed
    [p(0, 0), p(10, -12), p(7, 7), p(3, 1), p(7, 3), p(2, 4), p(5, 6), p(1, 4)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-5, 6),    /*0b0000*/
    p(-14, 9),   /*0b0001*/
    p(-2, 8),    /*0b0010*/
    p(-11, 13),  /*0b0011*/
    p(-3, 4),    /*0b0100*/
    p(-26, 0),   /*0b0101*/
    p(-14, 5),   /*0b0110*/
    p(-21, -17), /*0b0111*/
    p(8, 11),    /*0b1000*/
    p(-2, 10),   /*0b1001*/
    p(3, 10),    /*0b1010*/
    p(-4, 10),   /*0b1011*/
    p(-0, 5),    /*0b1100*/
    p(-22, 10),  /*0b1101*/
    p(-11, 3),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(3, 16),    /*0b10000*/
    p(2, 9),     /*0b10001*/
    p(22, 11),   /*0b10010*/
    p(-6, 5),    /*0b10011*/
    p(-5, 6),    /*0b10100*/
    p(11, 15),   /*0b10101*/
    p(-23, -0),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(15, 30),   /*0b11000*/
    p(29, 24),   /*0b11001*/
    p(43, 38),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(17, 11),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(16, 10),   /*0b100000*/
    p(4, 13),    /*0b100001*/
    p(25, 3),    /*0b100010*/
    p(5, -2),    /*0b100011*/
    p(-6, 2),    /*0b100100*/
    p(-19, -8),  /*0b100101*/
    p(-22, 15),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(21, 4),    /*0b101000*/
    p(1, 18),    /*0b101001*/
    p(22, -2),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-4, 8),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(16, 18),   /*0b110000*/
    p(25, 13),   /*0b110001*/
    p(33, 9),    /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(10, 30),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(25, 16),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(6, -2),    /*0b111111*/
    p(-13, -4),  /*0b00*/
    p(13, -17),  /*0b01*/
    p(39, -9),   /*0b10*/
    p(21, -40),  /*0b11*/
    p(48, -11),  /*0b100*/
    p(7, -21),   /*0b101*/
    p(70, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(64, -13),  /*0b1000*/
    p(21, -33),  /*0b1001*/
    p(83, -56),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(61, -20),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(20, -9),   /*0b1111*/
    p(21, -3),   /*0b00*/
    p(32, -13),  /*0b01*/
    p(27, -18),  /*0b10*/
    p(21, -42),  /*0b11*/
    p(38, -11),  /*0b100*/
    p(55, -20),  /*0b101*/
    p(25, -23),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(39, -4),   /*0b1000*/
    p(51, -17),  /*0b1001*/
    p(53, -42),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(45, -24),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(19, -42),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  31,   87),    p(  28,   86),    p(  19,   89),    p(  30,   69),    p(  17,   74),    p(  16,   77),    p( -21,   95),    p( -14,   93),
        p(  40,  124),    p(  48,  123),    p(  37,  100),    p(  20,   69),    p(  34,   69),    p(  15,   96),    p(  -0,  104),    p( -31,  125),
        p(  23,   74),    p(  17,   71),    p(  23,   54),    p(  16,   43),    p(  -1,   46),    p(   7,   59),    p( -10,   76),    p( -10,   79),
        p(   8,   47),    p(  -2,   44),    p( -15,   35),    p(  -9,   25),    p( -17,   29),    p( -10,   40),    p( -17,   55),    p( -10,   51),
        p(   2,   15),    p( -12,   24),    p( -15,   17),    p( -16,    8),    p( -16,   14),    p(  -8,   18),    p( -13,   38),    p(   9,   17),
        p(  -4,   15),    p(  -2,   20),    p(  -8,   17),    p(  -8,    4),    p(   4,    2),    p(   6,    8),    p(  11,   19),    p(   7,   14),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-11, -10);
const DOUBLED_PAWN: PhasedScore = p(-6, -21);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = [
    p(10, 11),
    p(6, 13),
    p(12, 19),
    p(7, 9),
    p(-5, 16),
    p(-49, 7),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(36, 8), p(37, 37), p(50, -8), p(33, -33), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-50, -66),
        p(-30, -27),
        p(-17, -4),
        p(-7, 9),
        p(1, 19),
        p(9, 29),
        p(18, 30),
        p(26, 31),
        p(32, 27),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-29, -53),
        p(-17, -36),
        p(-7, -19),
        p(0, -6),
        p(7, 4),
        p(12, 12),
        p(16, 17),
        p(20, 21),
        p(22, 26),
        p(30, 27),
        p(36, 27),
        p(45, 28),
        p(41, 35),
        p(57, 27),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-59, 35),
        p(-59, 42),
        p(-54, 47),
        p(-50, 51),
        p(-46, 54),
        p(-43, 58),
        p(-39, 62),
        p(-35, 64),
        p(-34, 69),
        p(-26, 69),
        p(-15, 65),
        p(-12, 65),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-24, -52),
        p(-25, 3),
        p(-29, 51),
        p(-24, 68),
        p(-22, 86),
        p(-17, 91),
        p(-13, 102),
        p(-10, 109),
        p(-5, 113),
        p(-2, 115),
        p(1, 118),
        p(5, 121),
        p(8, 121),
        p(9, 126),
        p(12, 127),
        p(16, 130),
        p(16, 137),
        p(20, 137),
        p(29, 134),
        p(43, 127),
        p(48, 128),
        p(93, 102),
        p(92, 105),
        p(117, 84),
        p(216, 46),
        p(260, 6),
        p(289, -4),
        p(355, -51),
    ],
    [
        p(-95, 10),
        p(-59, -6),
        p(-30, -5),
        p(1, -3),
        p(33, -2),
        p(57, -3),
        p(85, 3),
        p(111, 2),
        p(158, -16),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-8, 8),
        p(0, 0),
        p(24, 20),
        p(49, -11),
        p(20, -33),
        p(0, 0),
    ],
    [p(-2, 11), p(20, 21), p(0, 0), p(32, 3), p(30, 53), p(0, 0)],
    [p(-2, 13), p(11, 12), p(18, 8), p(0, 0), p(41, 2), p(0, 0)],
    [p(-2, 5), p(2, 4), p(1, 15), p(1, -2), p(0, 0), p(0, 0)],
    [
        p(71, 28),
        p(-35, 17),
        p(-10, 17),
        p(-25, 7),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 6), p(8, 5), p(6, 8), p(12, 5), p(7, 14), p(10, 4)],
    [
        p(-1, 1),
        p(9, 20),
        p(-122, -23),
        p(7, 13),
        p(8, 17),
        p(2, 5),
    ],
    [p(1, 2), p(12, 5), p(7, 10), p(9, 8), p(8, 19), p(18, -5)],
    [
        p(2, -2),
        p(8, -1),
        p(6, -7),
        p(4, 12),
        p(-77, -245),
        p(2, -10),
    ],
    [p(62, -1), p(41, 8), p(46, 2), p(25, 5), p(37, -7), p(0, 0)],
];

pub const NUM_DEFENDED: [PhasedScore; 17] = [
    p(26, -1),
    p(-18, 0),
    p(-24, -1),
    p(-19, -2),
    p(-13, -5),
    p(-13, -4),
    p(-14, -2),
    p(-10, -0),
    p(-6, 2),
    p(-2, 6),
    p(2, 13),
    p(7, 16),
    p(11, 18),
    p(14, 23),
    p(15, 11),
    p(14, -9),
    p(11, 218),
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-20, -17),
    p(19, -10),
    p(10, -4),
    p(14, -12),
    p(-3, 13),
    p(-13, 12),
];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(13, 20), p(34, -1), p(6, 31)];
const PINNED: [PhasedScore; NUM_CHESS_PIECES - 2] =
    [p(-23, -29), p(-15, -34), p(-42, -20), p(-69, -28)];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues:
    Debug + Default + Copy + Clone + Send + 'static + StaticallyNamedEntity
{
    type Score: ScoreType;

    fn psqt(
        &self,
        square: ChessSquare,
        piece: ChessPieceType,
        color: ChessColor,
    ) -> SingleFeatureScore<Self::Score>;

    fn passed_pawn(square: ChessSquare) -> SingleFeatureScore<Self::Score>;

    fn unsupported_pawn() -> SingleFeatureScore<Self::Score>;

    fn doubled_pawn() -> SingleFeatureScore<Self::Score>;

    fn bishop_pair() -> SingleFeatureScore<Self::Score>;

    fn rook_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score>;

    fn king_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score>;

    fn bishop_openness(openness: FileOpenness, len: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_shield(&self, color: ChessColor, config: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_protection(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn pawn_attack(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn mobility(piece: ChessPieceType, mobility: usize) -> SingleFeatureScore<Self::Score>;

    fn threats(
        attacking: ChessPieceType,
        targeted: ChessPieceType,
    ) -> SingleFeatureScore<Self::Score>;

    fn defended(
        protecting: ChessPieceType,
        target: ChessPieceType,
    ) -> SingleFeatureScore<Self::Score>;

    fn num_defended(num: usize) -> SingleFeatureScore<Self::Score>;

    fn king_zone_attack(attacking: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn can_give_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn pinned(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;
}

/// Eval values tuned on a combination of the zurichess dataset and a dataset used by 4ku,
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

    fn unsupported_pawn() -> PhasedScore {
        UNSUPPORTED_PAWN
    }

    fn doubled_pawn() -> PhasedScore {
        DOUBLED_PAWN
    }

    fn bishop_pair() -> PhasedScore {
        BISHOP_PAIR
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

    fn bishop_openness(
        openness: FileOpenness,
        len: usize,
    ) -> <PhasedScore as ScoreType>::SingleFeatureScore {
        BISHOP_OPENNESS[openness as usize][len - 1]
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

    fn num_defended(num: usize) -> PhasedScore {
        NUM_DEFENDED[num]
    }

    fn king_zone_attack(attacking: ChessPieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }

    fn can_give_check(piece: ChessPieceType) -> PhasedScore {
        CAN_GIVE_CHECK[piece as usize]
    }

    fn pinned(piece: ChessPieceType) -> SingleFeatureScore<Self::Score> {
        PINNED[piece as usize - 1]
    }
}
