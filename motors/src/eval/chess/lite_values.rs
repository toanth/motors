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
use gears::games::chess::pieces::{ChessPieceType, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::chess::ChessColor;
use gears::games::chess::ChessColor::White;
use gears::score::{p, PhasedScore};
use std::fmt::Debug;

#[rustfmt::skip]
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
        p( 131,  156),    p( 127,  157),    p( 115,  162),    p( 125,  148),    p( 112,  154),    p( 114,  156),    p(  80,  171),    p(  88,  166),
        p(  67,  121),    p(  64,  121),    p(  77,  113),    p(  86,  117),    p(  73,  114),    p( 123,  100),    p(  96,  122),    p(  93,  116),
        p(  52,  112),    p(  64,  106),    p(  62,  100),    p(  67,   92),    p(  84,   92),    p(  86,   87),    p(  80,   97),    p(  73,   93),
        p(  48,   99),    p(  56,  100),    p(  65,   92),    p(  75,   90),    p(  78,   88),    p(  79,   84),    p(  73,   87),    p(  60,   84),
        p(  43,   97),    p(  52,   93),    p(  56,   92),    p(  61,   97),    p(  69,   93),    p(  64,   89),    p(  71,   80),    p(  55,   84),
        p(  50,   98),    p(  52,   95),    p(  59,   96),    p(  59,  102),    p(  56,  105),    p(  75,   95),    p(  74,   81),    p(  56,   86),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 184,  273),    p( 209,  304),    p( 243,  317),    p( 270,  305),    p( 300,  308),    p( 215,  302),    p( 235,  300),    p( 215,  254),
        p( 277,  305),    p( 288,  313),    p( 301,  310),    p( 315,  313),    p( 306,  310),    p( 329,  298),    p( 288,  309),    p( 293,  295),
        p( 293,  304),    p( 303,  307),    p( 320,  316),    p( 323,  319),    p( 339,  313),    p( 363,  302),    p( 315,  302),    p( 311,  299),
        p( 305,  312),    p( 311,  309),    p( 318,  320),    p( 344,  323),    p( 322,  324),    p( 335,  321),    p( 316,  311),    p( 334,  304),
        p( 301,  315),    p( 300,  309),    p( 306,  321),    p( 313,  325),    p( 319,  326),    p( 317,  312),    p( 328,  305),    p( 317,  311),
        p( 275,  302),    p( 277,  304),    p( 285,  304),    p( 291,  318),    p( 298,  315),    p( 284,  299),    p( 298,  297),    p( 294,  306),
        p( 272,  307),    p( 282,  311),    p( 278,  306),    p( 290,  310),    p( 293,  305),    p( 285,  303),    p( 295,  303),    p( 291,  316),
        p( 243,  304),    p( 282,  301),    p( 266,  303),    p( 286,  309),    p( 296,  306),    p( 291,  296),    p( 289,  302),    p( 267,  306),
    ],
    // bishop
    [
        p( 279,  317),    p( 256,  315),    p( 248,  307),    p( 225,  315),    p( 222,  316),    p( 226,  306),    p( 282,  306),    p( 252,  309),
        p( 281,  303),    p( 285,  306),    p( 288,  307),    p( 282,  308),    p( 284,  304),    p( 295,  304),    p( 273,  308),    p( 276,  304),
        p( 299,  309),    p( 303,  305),    p( 296,  310),    p( 303,  302),    p( 308,  306),    p( 333,  309),    p( 318,  305),    p( 312,  311),
        p( 282,  310),    p( 299,  310),    p( 301,  305),    p( 318,  311),    p( 312,  307),    p( 307,  309),    p( 302,  308),    p( 283,  312),
        p( 292,  308),    p( 282,  311),    p( 300,  309),    p( 315,  308),    p( 313,  308),    p( 299,  307),    p( 291,  310),    p( 312,  300),
        p( 293,  308),    p( 304,  310),    p( 301,  310),    p( 304,  310),    p( 307,  311),    p( 304,  308),    p( 306,  303),    p( 310,  301),
        p( 309,  312),    p( 303,  301),    p( 311,  303),    p( 296,  310),    p( 303,  309),    p( 303,  306),    p( 313,  302),    p( 303,  300),
        p( 294,  305),    p( 314,  311),    p( 306,  307),    p( 290,  312),    p( 303,  309),    p( 296,  314),    p( 303,  300),    p( 301,  297),
    ],
    // rook
    [
        p( 459,  552),    p( 450,  561),    p( 447,  568),    p( 445,  565),    p( 458,  561),    p( 479,  555),    p( 485,  554),    p( 494,  547),
        p( 433,  558),    p( 431,  563),    p( 439,  564),    p( 455,  554),    p( 445,  557),    p( 466,  552),    p( 477,  548),    p( 491,  539),
        p( 438,  555),    p( 457,  550),    p( 455,  551),    p( 458,  547),    p( 486,  536),    p( 495,  532),    p( 517,  529),    p( 488,  532),
        p( 436,  554),    p( 443,  550),    p( 444,  553),    p( 450,  548),    p( 458,  539),    p( 467,  534),    p( 474,  536),    p( 469,  532),
        p( 431,  551),    p( 431,  549),    p( 431,  550),    p( 438,  548),    p( 445,  543),    p( 439,  542),    p( 458,  535),    p( 447,  534),
        p( 428,  547),    p( 427,  545),    p( 430,  544),    p( 432,  545),    p( 439,  538),    p( 448,  531),    p( 470,  518),    p( 452,  523),
        p( 430,  541),    p( 434,  541),    p( 440,  543),    p( 443,  541),    p( 450,  534),    p( 464,  524),    p( 473,  519),    p( 441,  529),
        p( 439,  545),    p( 435,  541),    p( 437,  546),    p( 442,  541),    p( 449,  535),    p( 455,  535),    p( 452,  532),    p( 446,  534),
    ],
    // queen
    [
        p( 873,  973),    p( 875,  987),    p( 890, 1000),    p( 907,  996),    p( 906, 1000),    p( 927,  986),    p( 976,  936),    p( 923,  966),
        p( 886,  962),    p( 860,  996),    p( 861, 1024),    p( 854, 1040),    p( 860, 1053),    p( 901, 1013),    p( 905,  995),    p( 948,  970),
        p( 893,  967),    p( 885,  987),    p( 884, 1011),    p( 881, 1022),    p( 904, 1024),    p( 943, 1009),    p( 949,  979),    p( 936,  985),
        p( 878,  982),    p( 883,  993),    p( 876, 1004),    p( 875, 1019),    p( 880, 1030),    p( 892, 1020),    p( 901, 1019),    p( 909,  995),
        p( 888,  974),    p( 874,  994),    p( 880,  997),    p( 880, 1015),    p( 881, 1013),    p( 884, 1012),    p( 898,  997),    p( 905,  989),
        p( 883,  960),    p( 889,  977),    p( 882,  994),    p( 879,  997),    p( 884, 1004),    p( 891,  994),    p( 905,  975),    p( 905,  963),
        p( 886,  958),    p( 884,  967),    p( 890,  970),    p( 889,  983),    p( 890,  983),    p( 892,  966),    p( 902,  944),    p( 912,  917),
        p( 872,  956),    p( 883,  945),    p( 883,  960),    p( 891,  960),    p( 894,  954),    p( 882,  954),    p( 883,  943),    p( 886,  934),
    ],
    // king
    [
        p( 151,  -66),    p(  59,  -16),    p(  84,   -9),    p(  14,   18),    p(  38,    6),    p(  16,   19),    p(  73,    5),    p( 220,  -70),
        p( -23,   24),    p( -58,   47),    p( -69,   56),    p(   2,   44),    p( -38,   54),    p( -51,   65),    p( -24,   52),    p(  14,   23),
        p( -58,   31),    p( -44,   40),    p( -89,   55),    p( -93,   63),    p( -57,   57),    p( -22,   50),    p( -55,   49),    p( -25,   30),
        p( -39,   18),    p( -95,   32),    p(-116,   48),    p(-139,   56),    p(-135,   54),    p(-114,   47),    p(-111,   36),    p(-100,   31),
        p( -53,    5),    p(-114,   19),    p(-127,   35),    p(-149,   46),    p(-155,   45),    p(-131,   32),    p(-140,   23),    p(-121,   22),
        p( -40,    1),    p( -86,    5),    p(-117,   20),    p(-126,   29),    p(-121,   28),    p(-134,   20),    p(-102,    5),    p( -76,   12),
        p(  26,  -14),    p( -66,   -4),    p( -80,    4),    p(-100,   13),    p(-105,   14),    p( -89,    5),    p( -58,  -11),    p(   3,   -5),
        p(  41,  -39),    p(  42,  -47),    p(  36,  -37),    p( -24,  -19),    p(  29,  -36),    p( -19,  -22),    p(  35,  -43),    p(  58,  -45),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  31,   56),    p(  27,   57),    p(  15,   62),    p(  25,   48),    p(  12,   54),    p(  14,   56),    p( -20,   71),    p( -12,   66),
        p(  31,   70),    p(  35,   78),    p(  21,   66),    p(   5,   45),    p(  14,   53),    p(  -7,   77),    p( -13,   75),    p( -42,   88),
        p(  11,   27),    p(   2,   35),    p(  16,   29),    p(   2,   31),    p( -24,   37),    p( -16,   46),    p( -25,   56),    p( -25,   51),
        p(  -4,    7),    p(  -5,   17),    p( -15,   17),    p( -22,   15),    p( -38,   23),    p( -33,   30),    p( -30,   39),    p( -26,   30),
        p(  13,  -16),    p(   2,    3),    p( -12,    2),    p( -29,   -0),    p( -36,    9),    p( -28,   12),    p( -19,   24),    p(  -4,    4),
        p(   8,  -15),    p(  11,    0),    p( -10,    4),    p( -25,   -2),    p( -18,   -2),    p( -16,    4),    p(  -1,   12),    p(  -7,    2),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-10, -10);
const DOUBLED_PAWN: PhasedScore = p(-7, -20);
const PASSED_PAWN_INSIDE_SQUARE_RULE: [PhasedScore; 8] = [
    p(-1, 73),
    p(22, -15),
    p(42, -8),
    p(27, 7),
    p(11, 17),
    p(-9, 24),
    p(-21, 22),
    p(-40, 7),
];

const BISHOP_PAIR: PhasedScore = p(23, 57);
const ROOK_OPEN_FILE: PhasedScore = p(16, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -2);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(0, 3);
const KING_OPEN_FILE: PhasedScore = p(-53, -3);
const KING_CLOSED_FILE: PhasedScore = p(15, -14);
const KING_SEMIOPEN_FILE: PhasedScore = p(-5, -3);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 7), p(-2, 8), p(-2, 11), p(4, 8), p(4, 10), p(5, 12), p(11, 12), p(22, 7), ],
    // Closed
    [p(0, 0), p(0, 0), p(15, -34), p(-14, 9), p(1, 13), p(3, 5), p(2, 11), p(-0, 7), ],
    // SemiOpen
    [p(0, 0), p(-18, 24), p(2, 22), p(1, 16), p(-1, 20), p(4, 15), p(2, 12), p(12, 12), ],
    // SemiClosed
    [p(0, 0), p(12, -12), p(9, 7), p(5, 2), p(9, 5), p(4, 5), p(8, 7), p(3, 5), ],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-7, 5),    /*0b0000*/
    p(-14, 11),  /*0b0001*/
    p(-3, 5),    /*0b0010*/
    p(-8, 10),   /*0b0011*/
    p(-6, 7),    /*0b0100*/
    p(-28, 5),   /*0b0101*/
    p(-12, 1),   /*0b0110*/
    p(-16, -25), /*0b0111*/
    p(3, 15),    /*0b1000*/
    p(-1, 11),   /*0b1001*/
    p(-0, 10),   /*0b1010*/
    p(-1, 6),    /*0b1011*/
    p(-5, 12),   /*0b1100*/
    p(-29, 16),  /*0b1101*/
    p(-12, 1),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-1, 20),   /*0b10000*/
    p(2, 12),    /*0b10001*/
    p(21, 13),   /*0b10010*/
    p(-2, 6),    /*0b10011*/
    p(-6, 8),    /*0b10100*/
    p(13, 15),   /*0b10101*/
    p(-19, -2),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(7, 40),    /*0b11000*/
    p(28, 28),   /*0b11001*/
    p(40, 41),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(13, 17),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(11, 14),   /*0b100000*/
    p(-0, 20),   /*0b100001*/
    p(24, 4),    /*0b100010*/
    p(7, -2),    /*0b100011*/
    p(-10, 5),   /*0b100100*/
    p(-26, -4),  /*0b100101*/
    p(-23, 13),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(12, 12),   /*0b101000*/
    p(-9, 28),   /*0b101001*/
    p(17, -1),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-11, 14),  /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(9, 27),    /*0b110000*/
    p(23, 19),   /*0b110001*/
    p(31, 14),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(7, 32),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(17, 26),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(6, -4),    /*0b111111*/
    p(-16, -6),  /*0b00*/
    p(4, -11),   /*0b01*/
    p(32, -0),   /*0b10*/
    p(22, -41),  /*0b11*/
    p(38, 1),    /*0b100*/
    p(-5, -13),  /*0b101*/
    p(68, -34),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(47, -1),   /*0b1000*/
    p(12, -24),  /*0b1001*/
    p(70, -45),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(46, 3),    /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(2, 17),    /*0b1111*/
    p(16, -8),   /*0b00*/
    p(29, -11),  /*0b01*/
    p(25, -22),  /*0b10*/
    p(24, -51),  /*0b11*/
    p(25, -4),   /*0b100*/
    p(46, -13),  /*0b101*/
    p(20, -25),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(33, -2),   /*0b1000*/
    p(52, -19),  /*0b1001*/
    p(48, -44),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(35, -16),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(20, -49),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 7), p(2, 9), p(10, 13), p(9, 8), p(-4, 18), p(-49, 13)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(37, 12),
    p(41, 39),
    p(52, -6),
    p(36, -31),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-56, -59),
        p(-33, -19),
        p(-18, 3),
        p(-6, 15),
        p(5, 24),
        p(15, 32),
        p(26, 32),
        p(35, 31),
        p(43, 26),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-25, -48),
        p(-13, -30),
        p(-3, -14),
        p(3, -3),
        p(10, 7),
        p(14, 15),
        p(16, 19),
        p(19, 23),
        p(19, 27),
        p(25, 28),
        p(29, 27),
        p(36, 28),
        p(29, 36),
        p(40, 28),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-74, 16),
        p(-65, 31),
        p(-61, 36),
        p(-58, 41),
        p(-58, 47),
        p(-52, 51),
        p(-49, 55),
        p(-45, 58),
        p(-41, 61),
        p(-38, 65),
        p(-32, 67),
        p(-29, 71),
        p(-19, 70),
        p(-7, 68),
        p(-3, 67),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-34, -32),
        p(-35, 26),
        p(-38, 75),
        p(-33, 93),
        p(-30, 110),
        p(-25, 115),
        p(-21, 126),
        p(-17, 132),
        p(-13, 136),
        p(-10, 138),
        p(-7, 142),
        p(-3, 145),
        p(-0, 146),
        p(1, 151),
        p(4, 152),
        p(8, 154),
        p(9, 160),
        p(12, 159),
        p(21, 156),
        p(35, 148),
        p(41, 147),
        p(83, 124),
        p(83, 125),
        p(108, 104),
        p(196, 72),
        p(244, 28),
        p(286, 7),
        p(341, -32),
    ],
    [
        p(-78, 28),
        p(-48, 7),
        p(-24, 1),
        p(0, 0),
        p(26, -1),
        p(44, -5),
        p(67, -3),
        p(86, -6),
        p(130, -26),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-6, -3),
        p(23, 19),
        p(49, -11),
        p(21, -39),
        p(0, 0),
    ],
    [p(-1, 12), p(18, 22), p(-2, 9), p(29, 6), p(27, 62), p(0, 0)],
    [p(2, 19), p(22, 22), p(23, 23), p(-6, 12), p(43, 3), p(0, 0)],
    [p(-0, -0), p(7, 13), p(-0, 31), p(0, 7), p(2, -15), p(0, 0)],
    [
        p(72, 28),
        p(-29, 22),
        p(2, 21),
        p(-33, 11),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(11, 5), p(9, 10), p(15, 5), p(9, 16), p(13, 3)],
    [
        p(-3, 2),
        p(7, 18),
        p(-100, -34),
        p(6, 12),
        p(7, 17),
        p(5, 5),
    ],
    [p(3, 1), p(14, 4), p(10, 10), p(12, 7), p(12, 15), p(22, -5)],
    [
        p(4, -5),
        p(10, -2),
        p(8, -8),
        p(4, 15),
        p(-58, -252),
        p(7, -11),
    ],
    [p(55, 2), p(37, 3), p(41, -2), p(20, 1), p(32, -14), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-16, -4),
    p(15, -7),
    p(16, -2),
    p(23, -13),
    p(5, 22),
    p(2, 20),
];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static {
    type Score: ScoreType;

    fn psqt(
        square: ChessSquare,
        piece: ChessPieceType,
        color: ChessColor,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn passed_pawn(square: ChessSquare) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn unsupported_pawn() -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn doubled_pawn() -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn passed_pawns_outside_square_rule(
        distance: usize,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn bishop_pair() -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn rook_openness(openness: FileOpenness) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn king_openness(openness: FileOpenness) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn bishop_openness(
        openness: FileOpenness,
        len: usize,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn pawn_shield(config: usize) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn pawn_protection(piece: ChessPieceType) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn pawn_attack(piece: ChessPieceType) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn mobility(
        piece: ChessPieceType,
        mobility: usize,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn threats(
        attacking: ChessPieceType,
        targeted: ChessPieceType,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn defended(
        protecting: ChessPieceType,
        target: ChessPieceType,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

    fn king_zone_attack(
        attacking: ChessPieceType,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;
}

/// Eval values tuned on a combination of the zurichess dataset and a dataset used by 4ku,
/// created by GCP using his engine Stoofvlees and filtered by cj5716 using Stockfish at depth 9,
/// using my own tuner `pliers`.
#[derive(Debug, Default, Copy, Clone)]
pub struct Lite {}

impl LiteValues for Lite {
    type Score = PhasedScore;

    fn psqt(square: ChessSquare, piece: ChessPieceType, color: ChessColor) -> Self::Score {
        PSQTS[piece as usize][square.flip_if(color == White).bb_idx()]
    }

    fn passed_pawn(square: ChessSquare) -> Self::Score {
        PASSED_PAWNS[square.bb_idx()]
    }

    fn unsupported_pawn() -> <Self::Score as ScoreType>::SingleFeatureScore {
        UNSUPPORTED_PAWN
    }

    fn doubled_pawn() -> <Self::Score as ScoreType>::SingleFeatureScore {
        DOUBLED_PAWN
    }

    fn passed_pawns_outside_square_rule(
        distance: usize,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore {
        PASSED_PAWN_INSIDE_SQUARE_RULE[distance]
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

    fn pawn_protection(piece: ChessPieceType) -> Self::Score {
        PAWN_PROTECTION[piece as usize]
    }

    fn pawn_attack(piece: ChessPieceType) -> Self::Score {
        PAWN_ATTACKS[piece as usize]
    }

    fn mobility(piece: ChessPieceType, mobility: usize) -> Self::Score {
        MOBILITY[piece as usize - 1][mobility]
    }

    fn threats(attacking: ChessPieceType, targeted: ChessPieceType) -> Self::Score {
        THREATS[attacking as usize - 1][targeted as usize]
    }
    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> Self::Score {
        DEFENDED[protecting as usize - 1][target as usize]
    }

    fn king_zone_attack(
        attacking: ChessPieceType,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore {
        KING_ZONE_ATTACK[attacking as usize]
    }
}
