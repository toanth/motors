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
        p( 132,  186),    p( 129,  185),    p( 120,  188),    p( 132,  169),    p( 119,  174),    p( 120,  177),    p(  83,  194),    p(  90,  192),
        p(  64,  124),    p(  57,  125),    p(  73,  120),    p(  81,  123),    p(  65,  125),    p( 117,  110),    p(  89,  132),    p(  87,  122),
        p(  49,  114),    p(  58,  110),    p(  59,  103),    p(  64,   96),    p(  80,   97),    p(  82,   93),    p(  74,  103),    p(  71,   95),
        p(  46,  100),    p(  48,  103),    p(  61,   94),    p(  71,   92),    p(  74,   92),    p(  76,   87),    p(  67,   92),    p(  59,   85),
        p(  40,   98),    p(  44,   95),    p(  52,   93),    p(  56,   98),    p(  64,   96),    p(  59,   92),    p(  65,   83),    p(  53,   85),
        p(  47,   98),    p(  44,   97),    p(  54,   97),    p(  54,  103),    p(  51,  106),    p(  70,   97),    p(  69,   83),    p(  54,   86),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 171,  268),    p( 197,  300),    p( 229,  313),    p( 252,  304),    p( 286,  305),    p( 198,  301),    p( 216,  300),    p( 199,  251),
        p( 262,  302),    p( 274,  311),    p( 287,  308),    p( 300,  311),    p( 291,  308),    p( 315,  296),    p( 273,  307),    p( 278,  293),
        p( 280,  300),    p( 290,  305),    p( 306,  314),    p( 306,  318),    p( 324,  311),    p( 347,  302),    p( 302,  302),    p( 295,  298),
        p( 295,  308),    p( 305,  306),    p( 311,  317),    p( 336,  319),    p( 316,  321),    p( 330,  318),    p( 311,  310),    p( 326,  301),
        p( 296,  310),    p( 298,  305),    p( 304,  318),    p( 310,  321),    p( 318,  323),    p( 317,  311),    p( 328,  303),    p( 315,  306),
        p( 274,  298),    p( 278,  303),    p( 285,  303),    p( 291,  316),    p( 299,  314),    p( 285,  299),    p( 300,  295),    p( 294,  302),
        p( 270,  303),    p( 282,  308),    p( 279,  304),    p( 290,  308),    p( 294,  303),    p( 287,  300),    p( 297,  301),    p( 290,  312),
        p( 243,  299),    p( 283,  299),    p( 268,  301),    p( 286,  306),    p( 297,  304),    p( 293,  293),    p( 291,  299),    p( 267,  298),
    ],
    // bishop
    [
        p( 272,  313),    p( 248,  313),    p( 239,  305),    p( 217,  313),    p( 216,  313),    p( 216,  305),    p( 276,  303),    p( 244,  307),
        p( 274,  301),    p( 276,  304),    p( 280,  305),    p( 274,  306),    p( 277,  302),    p( 288,  301),    p( 263,  306),    p( 268,  302),
        p( 290,  306),    p( 296,  303),    p( 288,  309),    p( 295,  301),    p( 300,  304),    p( 326,  307),    p( 313,  301),    p( 304,  309),
        p( 277,  308),    p( 297,  307),    p( 297,  304),    p( 313,  309),    p( 310,  305),    p( 305,  307),    p( 303,  307),    p( 280,  309),
        p( 290,  306),    p( 281,  310),    p( 300,  308),    p( 313,  307),    p( 313,  307),    p( 299,  307),    p( 292,  309),    p( 312,  298),
        p( 292,  305),    p( 306,  310),    p( 301,  309),    p( 304,  309),    p( 308,  310),    p( 305,  307),    p( 308,  302),    p( 310,  299),
        p( 310,  310),    p( 303,  300),    p( 311,  303),    p( 296,  310),    p( 303,  309),    p( 304,  306),    p( 314,  301),    p( 303,  298),
        p( 295,  303),    p( 317,  309),    p( 306,  306),    p( 291,  311),    p( 305,  309),    p( 296,  313),    p( 306,  298),    p( 302,  295),
    ],
    // rook
    [
        p( 450,  546),    p( 442,  555),    p( 439,  561),    p( 436,  558),    p( 449,  554),    p( 469,  549),    p( 478,  547),    p( 486,  541),
        p( 425,  552),    p( 424,  557),    p( 431,  558),    p( 446,  548),    p( 436,  550),    p( 458,  545),    p( 471,  541),    p( 482,  533),
        p( 430,  549),    p( 448,  544),    p( 446,  545),    p( 449,  541),    p( 477,  530),    p( 487,  526),    p( 509,  523),    p( 480,  526),
        p( 430,  548),    p( 439,  544),    p( 440,  546),    p( 444,  541),    p( 455,  533),    p( 466,  528),    p( 473,  530),    p( 466,  525),
        p( 428,  544),    p( 430,  543),    p( 431,  544),    p( 436,  541),    p( 444,  538),    p( 440,  537),    p( 460,  530),    p( 447,  527),
        p( 427,  540),    p( 428,  539),    p( 431,  538),    p( 433,  539),    p( 441,  534),    p( 449,  527),    p( 473,  514),    p( 453,  517),
        p( 429,  535),    p( 435,  537),    p( 441,  538),    p( 444,  536),    p( 451,  530),    p( 466,  521),    p( 474,  516),    p( 442,  523),
        p( 439,  540),    p( 436,  538),    p( 437,  542),    p( 442,  537),    p( 450,  532),    p( 456,  532),    p( 454,  530),    p( 447,  528),
    ],
    // queen
    [
        p( 867,  968),    p( 869,  982),    p( 884,  995),    p( 900,  991),    p( 899,  995),    p( 920,  982),    p( 971,  930),    p( 916,  962),
        p( 877,  961),    p( 852,  994),    p( 854, 1019),    p( 847, 1036),    p( 854, 1047),    p( 894, 1008),    p( 896,  990),    p( 940,  966),
        p( 884,  967),    p( 877,  985),    p( 876, 1008),    p( 874, 1016),    p( 897, 1018),    p( 937, 1002),    p( 945,  971),    p( 930,  978),
        p( 874,  979),    p( 883,  987),    p( 875,  997),    p( 873, 1011),    p( 881, 1021),    p( 893, 1013),    p( 904, 1011),    p( 908,  988),
        p( 888,  966),    p( 877,  985),    p( 881,  990),    p( 881, 1007),    p( 883, 1006),    p( 887, 1006),    p( 901,  991),    p( 907,  982),
        p( 884,  951),    p( 892,  970),    p( 884,  987),    p( 881,  990),    p( 887,  997),    p( 894,  989),    p( 909,  970),    p( 907,  955),
        p( 886,  950),    p( 886,  960),    p( 893,  964),    p( 891,  977),    p( 893,  978),    p( 895,  962),    p( 905,  939),    p( 913,  910),
        p( 874,  948),    p( 886,  939),    p( 886,  954),    p( 893,  956),    p( 897,  949),    p( 885,  950),    p( 886,  939),    p( 888,  926),
    ],
    // king
    [
        p( 153, -105),    p(  59,  -51),    p(  83,  -43),    p(   7,  -11),    p(  29,  -23),    p(  12,  -13),    p(  67,  -22),    p( 225, -108),
        p( -22,   -5),    p( -67,   25),    p( -76,   35),    p( -10,   24),    p( -43,   33),    p( -70,   46),    p( -37,   32),    p(  11,   -3),
        p( -42,    4),    p( -35,   21),    p( -80,   39),    p( -85,   47),    p( -52,   41),    p( -19,   34),    p( -57,   33),    p( -29,    9),
        p( -25,   -3),    p( -90,   21),    p(-104,   38),    p(-127,   47),    p(-125,   45),    p(-106,   38),    p(-112,   27),    p( -96,   13),
        p( -46,   -6),    p(-111,   17),    p(-120,   33),    p(-143,   46),    p(-149,   45),    p(-127,   31),    p(-140,   23),    p(-116,   11),
        p( -37,   -1),    p( -87,   12),    p(-117,   27),    p(-125,   36),    p(-120,   35),    p(-134,   28),    p(-106,   14),    p( -74,    9),
        p(  28,  -10),    p( -70,    7),    p( -82,   16),    p(-103,   25),    p(-109,   26),    p( -94,   18),    p( -63,    2),    p(   3,   -3),
        p(  45,  -43),    p(  42,  -47),    p(  37,  -35),    p( -24,  -14),    p(  28,  -32),    p( -21,  -17),    p(  34,  -41),    p(  61,  -52),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(16, 6);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(-1, 2);
const KING_OPEN_FILE: PhasedScore = p(-56, -1);
const KING_CLOSED_FILE: PhasedScore = p(15, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 6);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-8, 4), p(-3, 6), p(-2, 9), p(3, 6), p(4, 9), p(5, 11), p(10, 10), p(21, 6)],
    // Closed
    [p(0, 0), p(0, 0), p(11, -31), p(-16, 8), p(-1, 12), p(3, 3), p(1, 9), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-17, 21), p(5, 19), p(1, 14), p(-2, 18), p(3, 13), p(1, 10), p(12, 10)],
    // SemiClosed
    [p(0, 0), p(9, -12), p(8, 6), p(4, 1), p(8, 3), p(3, 3), p(7, 6), p(2, 3)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-8, 7),    /*0b0000*/
    p(-17, 12),  /*0b0001*/
    p(-4, 8),    /*0b0010*/
    p(-10, 14),  /*0b0011*/
    p(-6, 7),    /*0b0100*/
    p(-28, 5),   /*0b0101*/
    p(-14, 6),   /*0b0110*/
    p(-18, -17), /*0b0111*/
    p(5, 11),    /*0b1000*/
    p(-7, 12),   /*0b1001*/
    p(0, 10),    /*0b1010*/
    p(-1, 11),   /*0b1011*/
    p(-3, 8),    /*0b1100*/
    p(-27, 11),  /*0b1101*/
    p(-12, 5),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(0, 19),    /*0b10000*/
    p(4, 12),    /*0b10001*/
    p(20, 14),   /*0b10010*/
    p(-1, 9),    /*0b10011*/
    p(-6, 9),    /*0b10100*/
    p(11, 18),   /*0b10101*/
    p(-19, 3),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(11, 34),   /*0b11000*/
    p(31, 26),   /*0b11001*/
    p(38, 40),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(15, 14),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(13, 11),   /*0b100000*/
    p(2, 16),    /*0b100001*/
    p(24, 4),    /*0b100010*/
    p(7, 2),     /*0b100011*/
    p(-11, 5),   /*0b100100*/
    p(-25, -6),  /*0b100101*/
    p(-24, 17),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(18, 4),    /*0b101000*/
    p(-5, 19),   /*0b101001*/
    p(18, -2),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-8, 8),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(13, 22),   /*0b110000*/
    p(25, 18),   /*0b110001*/
    p(32, 13),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(4, 33),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(22, 17),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(5, 1),     /*0b111111*/
    p(-21, -10), /*0b00*/
    p(10, -26),  /*0b01*/
    p(38, -14),  /*0b10*/
    p(28, -50),  /*0b11*/
    p(47, -18),  /*0b100*/
    p(-3, -30),  /*0b101*/
    p(76, -49),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(57, -19),  /*0b1000*/
    p(21, -44),  /*0b1001*/
    p(82, -64),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(59, -26),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(21, -1),   /*0b1111*/
    p(16, -10),  /*0b00*/
    p(32, -20),  /*0b01*/
    p(26, -27),  /*0b10*/
    p(25, -53),  /*0b11*/
    p(32, -18),  /*0b100*/
    p(53, -29),  /*0b101*/
    p(23, -33),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(37, -12),  /*0b1000*/
    p(55, -26),  /*0b1001*/
    p(49, -53),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(41, -30),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(26, -54),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  32,   86),    p(  29,   85),    p(  20,   88),    p(  32,   69),    p(  19,   74),    p(  20,   77),    p( -17,   94),    p( -10,   92),
        p(  40,  122),    p(  49,  121),    p(  36,   98),    p(  21,   67),    p(  35,   65),    p(  14,   94),    p(  -1,  102),    p( -28,  123),
        p(  22,   71),    p(  19,   69),    p(  22,   52),    p(  16,   42),    p(  -1,   44),    p(   6,   57),    p( -13,   74),    p( -12,   77),
        p(   6,   45),    p(  -3,   43),    p( -16,   34),    p( -10,   24),    p( -18,   29),    p( -12,   37),    p( -21,   53),    p( -14,   49),
        p(   0,   13),    p( -13,   21),    p( -18,   17),    p( -20,    9),    p( -17,   13),    p( -12,   17),    p( -16,   35),    p(   7,   16),
        p(  -6,   15),    p(  -4,   20),    p( -12,   17),    p( -10,    5),    p(   2,    1),    p(   2,    7),    p(  10,   17),    p(   5,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-9, -10);
const DOUBLED_PAWN: PhasedScore = p(-4, -21);
const OUTPOST: [PhasedScore; NUM_CHESS_PIECES - 1] =
    [p(15, 4), p(9, 4), p(9, 7), p(11, 2), p(-6, 6)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(12, 7), p(2, 9), p(10, 14), p(9, 9), p(-5, 19), p(-45, 8)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(34, 4),
    p(40, 32),
    p(49, -13),
    p(34, -43),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-58, -58),
        p(-36, -20),
        p(-20, 1),
        p(-8, 11),
        p(2, 20),
        p(12, 28),
        p(23, 27),
        p(31, 26),
        p(38, 21),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-26, -48),
        p(-14, -30),
        p(-4, -15),
        p(3, -4),
        p(9, 5),
        p(13, 13),
        p(16, 17),
        p(18, 21),
        p(19, 25),
        p(25, 25),
        p(28, 24),
        p(36, 25),
        p(30, 33),
        p(42, 25),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-74, 13),
        p(-65, 26),
        p(-60, 32),
        p(-57, 36),
        p(-57, 43),
        p(-52, 47),
        p(-49, 51),
        p(-45, 53),
        p(-41, 56),
        p(-37, 59),
        p(-31, 61),
        p(-28, 65),
        p(-20, 64),
        p(-8, 61),
        p(-6, 61),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-33, -38),
        p(-34, 19),
        p(-38, 69),
        p(-33, 86),
        p(-30, 103),
        p(-25, 108),
        p(-20, 117),
        p(-17, 123),
        p(-12, 127),
        p(-9, 129),
        p(-6, 132),
        p(-2, 135),
        p(1, 136),
        p(2, 141),
        p(5, 143),
        p(8, 145),
        p(9, 152),
        p(11, 151),
        p(20, 148),
        p(34, 140),
        p(39, 140),
        p(81, 116),
        p(81, 118),
        p(104, 98),
        p(195, 63),
        p(248, 18),
        p(287, 2),
        p(340, -34),
    ],
    [
        p(-85, 52),
        p(-53, 23),
        p(-27, 12),
        p(-0, 5),
        p(27, -2),
        p(47, -10),
        p(70, -10),
        p(91, -17),
        p(139, -42),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-9, 13),
        p(-6, -3),
        p(23, 16),
        p(49, -15),
        p(21, -45),
        p(0, 0),
    ],
    [p(-1, 13), p(19, 21), p(-2, 9), p(28, 2), p(27, 56), p(0, 0)],
    [
        p(3, 16),
        p(22, 20),
        p(23, 21),
        p(-5, 10),
        p(42, -5),
        p(0, 0),
    ],
    [p(-1, -2), p(7, 13), p(-1, 30), p(-1, 7), p(2, -18), p(0, 0)],
    [p(81, 33), p(-30, 21), p(1, 19), p(-33, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(10, 4), p(9, 10), p(15, 4), p(9, 15), p(13, 3)],
    [p(-3, 1), p(8, 18), p(-96, -37), p(6, 12), p(8, 17), p(4, 5)],
    [p(2, 2), p(14, 3), p(9, 9), p(11, 6), p(12, 14), p(22, -6)],
    [
        p(3, -4),
        p(10, -2),
        p(8, -9),
        p(4, 15),
        p(-55, -266),
        p(7, -11),
    ],
    [
        p(60, -8),
        p(39, -1),
        p(44, -6),
        p(22, -2),
        p(34, -18),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-12, -11),
    p(18, -9),
    p(17, -3),
    p(23, -12),
    p(6, 23),
    p(7, 19),
];

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

    fn king_zone_attack(attacking: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn outpost(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;
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

    fn king_zone_attack(attacking: ChessPieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }

    fn outpost(piece: ChessPieceType) -> PhasedScore {
        OUTPOST[piece as usize - 1]
    }
}
