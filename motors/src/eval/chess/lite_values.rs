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
        p( 127,  181),    p( 127,  181),    p( 117,  184),    p( 128,  165),    p( 115,  170),    p( 117,  173),    p(  81,  190),    p(  84,  188),
        p(  64,  121),    p(  64,  123),    p(  72,  112),    p(  80,  116),    p(  67,  118),    p( 110,  104),    p(  91,  128),    p(  78,  118),
        p(  53,  109),    p(  64,  104),    p(  58,   96),    p(  60,   88),    p(  78,   89),    p(  79,   88),    p(  74,   99),    p(  68,   91),
        p(  48,   96),    p(  58,   98),    p(  61,   87),    p(  69,   85),    p(  71,   86),    p(  75,   81),    p(  70,   89),    p(  56,   81),
        p(  42,   91),    p(  54,   88),    p(  52,   84),    p(  57,   91),    p(  67,   87),    p(  57,   84),    p(  70,   79),    p(  49,   79),
        p(  53,   96),    p(  64,   96),    p(  60,   90),    p(  59,   96),    p(  65,  100),    p(  75,   92),    p(  85,   84),    p(  52,   83),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 189,  270),    p( 212,  301),    p( 247,  312),    p( 279,  300),    p( 311,  302),    p( 220,  299),    p( 235,  300),    p( 223,  249),
        p( 276,  303),    p( 288,  311),    p( 303,  308),    p( 323,  308),    p( 315,  304),    p( 339,  293),    p( 289,  307),    p( 300,  292),
        p( 291,  301),    p( 303,  305),    p( 319,  314),    p( 330,  314),    p( 348,  307),    p( 369,  298),    p( 324,  299),    p( 319,  295),
        p( 304,  310),    p( 309,  309),    p( 317,  319),    p( 344,  320),    p( 333,  317),    p( 342,  316),    p( 326,  305),    p( 342,  300),
        p( 298,  314),    p( 297,  307),    p( 302,  320),    p( 310,  324),    p( 317,  325),    p( 315,  311),    p( 326,  303),    p( 314,  309),
        p( 273,  301),    p( 274,  304),    p( 281,  304),    p( 287,  317),    p( 295,  313),    p( 280,  299),    p( 297,  295),    p( 290,  304),
        p( 269,  306),    p( 279,  309),    p( 275,  304),    p( 288,  309),    p( 291,  303),    p( 283,  300),    p( 294,  300),    p( 290,  313),
        p( 242,  302),    p( 282,  300),    p( 264,  302),    p( 284,  307),    p( 294,  304),    p( 290,  293),    p( 289,  299),    p( 267,  299),
    ],
    // bishop
    [
        p( 282,  316),    p( 252,  321),    p( 249,  316),    p( 228,  322),    p( 236,  320),    p( 231,  316),    p( 286,  311),    p( 259,  309),
        p( 283,  309),    p( 288,  313),    p( 292,  314),    p( 282,  318),    p( 296,  312),    p( 304,  310),    p( 286,  313),    p( 283,  308),
        p( 294,  318),    p( 308,  312),    p( 299,  319),    p( 312,  312),    p( 317,  315),    p( 346,  315),    p( 332,  309),    p( 322,  317),
        p( 285,  316),    p( 301,  319),    p( 308,  316),    p( 329,  319),    p( 320,  317),    p( 316,  319),    p( 304,  319),    p( 293,  316),
        p( 294,  313),    p( 286,  320),    p( 303,  321),    p( 321,  318),    p( 318,  318),    p( 299,  320),    p( 291,  319),    p( 315,  305),
        p( 295,  315),    p( 308,  317),    p( 303,  318),    p( 308,  321),    p( 308,  323),    p( 303,  317),    p( 308,  309),    p( 308,  309),
        p( 308,  316),    p( 308,  307),    p( 314,  309),    p( 296,  321),    p( 302,  319),    p( 303,  313),    p( 314,  309),    p( 301,  304),
        p( 295,  307),    p( 318,  312),    p( 310,  314),    p( 290,  318),    p( 304,  315),    p( 297,  321),    p( 307,  301),    p( 298,  298),
    ],
    // rook
    [
        p( 466,  546),    p( 456,  555),    p( 456,  560),    p( 455,  557),    p( 468,  552),    p( 487,  548),    p( 494,  547),    p( 504,  540),
        p( 438,  550),    p( 436,  556),    p( 447,  556),    p( 464,  545),    p( 456,  547),    p( 474,  542),    p( 485,  539),    p( 498,  530),
        p( 436,  550),    p( 458,  545),    p( 455,  547),    p( 460,  542),    p( 486,  531),    p( 502,  524),    p( 527,  521),    p( 498,  523),
        p( 435,  551),    p( 444,  546),    p( 444,  549),    p( 450,  544),    p( 461,  535),    p( 473,  528),    p( 482,  528),    p( 477,  524),
        p( 431,  547),    p( 432,  546),    p( 433,  547),    p( 440,  544),    p( 446,  540),    p( 447,  535),    p( 467,  528),    p( 455,  527),
        p( 429,  543),    p( 429,  542),    p( 432,  542),    p( 435,  542),    p( 444,  535),    p( 455,  525),    p( 480,  511),    p( 459,  517),
        p( 433,  539),    p( 436,  539),    p( 442,  540),    p( 445,  538),    p( 454,  530),    p( 468,  520),    p( 479,  514),    p( 446,  524),
        p( 441,  542),    p( 437,  538),    p( 438,  543),    p( 443,  539),    p( 450,  532),    p( 457,  531),    p( 456,  527),    p( 450,  529),
    ],
    // queen
    [
        p( 873,  972),    p( 870,  990),    p( 887, 1003),    p( 905,  999),    p( 905, 1003),    p( 925,  989),    p( 976,  937),    p( 927,  964),
        p( 880,  964),    p( 854,  996),    p( 857, 1023),    p( 850, 1040),    p( 858, 1052),    p( 897, 1016),    p( 902,  991),    p( 945,  971),
        p( 885,  963),    p( 878,  981),    p( 877, 1005),    p( 879, 1019),    p( 902, 1025),    p( 944, 1006),    p( 949,  976),    p( 938,  984),
        p( 873,  974),    p( 877,  985),    p( 873, 1001),    p( 873, 1017),    p( 877, 1030),    p( 890, 1022),    p( 899, 1018),    p( 909,  997),
        p( 883,  965),    p( 870,  992),    p( 876,  994),    p( 876, 1012),    p( 878, 1009),    p( 880, 1010),    p( 894,  998),    p( 903,  990),
        p( 879,  958),    p( 884,  973),    p( 877,  987),    p( 875,  991),    p( 879,  994),    p( 887,  987),    p( 902,  972),    p( 902,  962),
        p( 883,  954),    p( 881,  960),    p( 887,  963),    p( 885,  972),    p( 885,  971),    p( 886,  960),    p( 898,  940),    p( 907,  917),
        p( 869,  949),    p( 880,  940),    p( 878,  950),    p( 886,  950),    p( 888,  944),    p( 876,  950),    p( 879,  937),    p( 882,  926),
    ],
    // king
    [
        p( 127, -114),    p(  47,  -57),    p(  72,  -49),    p(  -7,  -17),    p(  18,  -30),    p(   1,  -20),    p(  56,  -30),    p( 199, -117),
        p( -24,   -9),    p( -45,   31),    p( -57,   42),    p(  10,   31),    p( -20,   39),    p( -52,   52),    p( -12,   36),    p(  17,   -6),
        p( -43,    1),    p( -14,   27),    p( -61,   46),    p( -67,   54),    p( -31,   48),    p(  -2,   40),    p( -34,   38),    p( -28,    7),
        p( -28,   -4),    p( -72,   28),    p( -91,   47),    p(-110,   56),    p(-111,   53),    p( -93,   45),    p( -98,   34),    p( -97,   14),
        p( -45,   -6),    p( -91,   24),    p(-106,   42),    p(-128,   56),    p(-135,   54),    p(-111,   39),    p(-120,   29),    p(-112,   12),
        p( -34,   -1),    p( -66,   19),    p( -98,   35),    p(-108,   45),    p(-103,   43),    p(-115,   36),    p( -83,   20),    p( -70,   10),
        p(  30,  -11),    p( -48,   13),    p( -60,   23),    p( -84,   33),    p( -89,   33),    p( -73,   24),    p( -37,    5),    p(   8,   -5),
        p(  23,  -52),    p(  34,  -55),    p(  32,  -42),    p( -34,  -20),    p(  22,  -39),    p( -30,  -24),    p(  29,  -51),    p(  45,  -62),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  27,   81),    p(  27,   81),    p(  17,   84),    p(  28,   65),    p(  15,   70),    p(  17,   73),    p( -19,   90),    p( -16,   88),
        p(  29,  116),    p(  39,  115),    p(  31,   98),    p(  16,   67),    p(  26,   66),    p(  13,   93),    p(  -6,   97),    p( -33,  119),
        p(  10,   67),    p(   9,   66),    p(  18,   52),    p(  11,   44),    p(  -7,   46),    p(   2,   56),    p( -15,   71),    p( -17,   72),
        p(  -3,   41),    p( -12,   39),    p( -20,   34),    p( -12,   25),    p( -20,   29),    p( -18,   37),    p( -25,   50),    p( -17,   46),
        p(  -9,   11),    p( -21,   20),    p( -21,   17),    p( -17,    8),    p( -17,   14),    p( -14,   16),    p( -20,   33),    p(   3,   14),
        p( -15,   10),    p( -12,   14),    p( -14,   17),    p(  -9,    4),    p(   1,    0),    p(  -1,    6),    p(   4,   12),    p(   1,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(15, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-14, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(-3, -0);
const KING_OPEN_FILE: PhasedScore = p(-60, -3);
const KING_CLOSED_FILE: PhasedScore = p(15, -17);
const KING_SEMIOPEN_FILE: PhasedScore = p(-11, 1);
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-9, 6),    /*0b0000*/
    p(-18, 11),  /*0b0001*/
    p(-11, 7),   /*0b0010*/
    p(-8, 27),   /*0b0011*/
    p(-3, 6),    /*0b0100*/
    p(-29, 3),   /*0b0101*/
    p(-11, 17),  /*0b0110*/
    p(-13, 1),   /*0b0111*/
    p(6, 8),     /*0b1000*/
    p(-20, -13), /*0b1001*/
    p(-3, 7),    /*0b1010*/
    p(-5, 1),    /*0b1011*/
    p(-2, 5),    /*0b1100*/
    p(-37, -13), /*0b1101*/
    p(-3, 17),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-4, 15),   /*0b10000*/
    p(7, 10),    /*0b10001*/
    p(-4, -14),  /*0b10010*/
    p(-7, -1),   /*0b10011*/
    p(-3, 5),    /*0b10100*/
    p(11, 14),   /*0b10101*/
    p(-23, -10), /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(19, 43),   /*0b11000*/
    p(27, 9),    /*0b11001*/
    p(30, 22),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(19, 19),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(17, 7),    /*0b100000*/
    p(0, 11),    /*0b100001*/
    p(17, 1),    /*0b100010*/
    p(9, 14),    /*0b100011*/
    p(-25, -22), /*0b100100*/
    p(-39, -31), /*0b100101*/
    p(-32, 7),   /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(19, -2),   /*0b101000*/
    p(-19, -5),  /*0b101001*/
    p(18, -6),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-23, -19), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(27, 29),   /*0b110000*/
    p(37, 21),   /*0b110001*/
    p(21, -11),  /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(1, 10),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(37, 35),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(0, -12),   /*0b111111*/
    p(-31, -14), /*0b00*/
    p(-1, -30),  /*0b01*/
    p(27, -17),  /*0b10*/
    p(26, -42),  /*0b11*/
    p(36, -25),  /*0b100*/
    p(-28, -51), /*0b101*/
    p(65, -55),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(54, -26),  /*0b1000*/
    p(15, -49),  /*0b1001*/
    p(54, -92),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(66, -23),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(2, -15),   /*0b1111*/
    p(6, -16),   /*0b00*/
    p(20, -28),  /*0b01*/
    p(16, -33),  /*0b10*/
    p(23, -47),  /*0b11*/
    p(28, -27),  /*0b100*/
    p(34, -64),  /*0b101*/
    p(16, -42),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(27, -21),  /*0b1000*/
    p(44, -35),  /*0b1001*/
    p(32, -87),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(45, -26),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(12, -77),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 13), p(3, 10), p(9, 15), p(8, 8), p(-4, 19), p(-45, 9)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(37, 10),
    p(44, 35),
    p(50, -8),
    p(38, -34),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-60, -60),
        p(-36, -22),
        p(-20, 0),
        p(-8, 11),
        p(3, 20),
        p(13, 28),
        p(24, 27),
        p(34, 26),
        p(43, 21),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-29, -35),
        p(-15, -19),
        p(-6, -3),
        p(2, 8),
        p(11, 16),
        p(17, 24),
        p(21, 28),
        p(26, 31),
        p(30, 35),
        p(38, 35),
        p(46, 33),
        p(59, 33),
        p(62, 39),
        p(76, 30),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-79, 17),
        p(-69, 30),
        p(-65, 34),
        p(-62, 38),
        p(-62, 44),
        p(-56, 48),
        p(-53, 52),
        p(-48, 54),
        p(-44, 57),
        p(-39, 60),
        p(-34, 61),
        p(-30, 64),
        p(-20, 63),
        p(-7, 59),
        p(-7, 61),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-42, -31),
        p(-43, 17),
        p(-46, 65),
        p(-41, 82),
        p(-38, 100),
        p(-32, 106),
        p(-28, 117),
        p(-24, 125),
        p(-19, 130),
        p(-16, 132),
        p(-13, 137),
        p(-8, 140),
        p(-5, 142),
        p(-4, 147),
        p(-1, 150),
        p(3, 153),
        p(4, 160),
        p(6, 160),
        p(16, 158),
        p(30, 151),
        p(34, 152),
        p(77, 129),
        p(76, 132),
        p(100, 113),
        p(192, 78),
        p(248, 33),
        p(290, 10),
        p(321, -22),
    ],
    [
        p(-55, 67),
        p(-31, 34),
        p(-14, 19),
        p(3, 8),
        p(21, -2),
        p(31, -13),
        p(42, -14),
        p(52, -27),
        p(90, -59),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-10, 11),
        p(-7, -5),
        p(24, 16),
        p(48, -14),
        p(21, -45),
        p(0, 0),
    ],
    [p(-0, 15), p(20, 21), p(1, 5), p(29, 1), p(27, 56), p(0, 0)],
    [p(7, 16), p(22, 19), p(25, 20), p(-6, 9), p(43, -6), p(0, 0)],
    [p(1, 2), p(7, 12), p(0, 30), p(1, 5), p(1, -16), p(0, 0)],
    [
        p(68, 37),
        p(-39, 25),
        p(-9, 21),
        p(-48, 16),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(11, 5), p(9, 10), p(16, 5), p(9, 16), p(14, 2)],
    [
        p(1, -5),
        p(10, 17),
        p(-25, -25),
        p(8, 12),
        p(9, 16),
        p(6, 5),
    ],
    [p(2, 1), p(14, 3), p(10, 9), p(12, 7), p(13, 14), p(22, -6)],
    [
        p(3, -3),
        p(10, -1),
        p(9, -7),
        p(4, 16),
        p(-54, -262),
        p(7, -11),
    ],
    [
        p(47, -13),
        p(28, -5),
        p(34, -10),
        p(12, -7),
        p(25, -24),
        p(0, 0),
    ],
];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static {
    type Score: ScoreType;

    fn psqt(square: ChessSquare, piece: UncoloredChessPiece, color: Color) -> Self::Score;
    fn passed_pawn(square: ChessSquare) -> Self::Score;
    fn bishop_pair() -> Self::Score;
    fn rook_openness(openness: FileOpenness) -> Self::Score;
    fn king_openness(openness: FileOpenness) -> Self::Score;
    fn pawn_shield(config: usize) -> Self::Score;
    fn pawn_protection(piece: UncoloredChessPiece) -> Self::Score;
    fn pawn_attack(piece: UncoloredChessPiece) -> Self::Score;
    fn mobility(piece: UncoloredChessPiece, mobility: usize) -> Self::Score;
    fn threats(attacking: UncoloredChessPiece, targeted: UncoloredChessPiece) -> Self::Score;
    fn defended(protecting: UncoloredChessPiece, target: UncoloredChessPiece) -> Self::Score;
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
}
