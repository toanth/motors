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
        p( 133,  187),    p( 130,  186),    p( 120,  189),    p( 132,  169),    p( 119,  174),    p( 120,  177),    p(  83,  195),    p(  91,  193),
        p(  65,  122),    p(  62,  121),    p(  74,  116),    p(  82,  122),    p(  66,  122),    p( 117,  107),    p(  91,  129),    p(  88,  121),
        p(  51,  112),    p(  63,  106),    p(  60,  100),    p(  65,   93),    p(  81,   95),    p(  83,   90),    p(  77,  100),    p(  71,   94),
        p(  48,   98),    p(  55,   99),    p(  63,   91),    p(  72,   90),    p(  75,   89),    p(  76,   85),    p(  70,   89),    p(  59,   84),
        p(  43,   96),    p(  50,   90),    p(  54,   90),    p(  58,   97),    p(  66,   93),    p(  60,   88),    p(  68,   80),    p(  53,   84),
        p(  49,   97),    p(  50,   93),    p(  57,   94),    p(  57,  103),    p(  53,  105),    p(  71,   95),    p(  71,   80),    p(  55,   86),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 185,  271),    p( 209,  303),    p( 243,  315),    p( 267,  305),    p( 299,  307),    p( 212,  302),    p( 229,  302),    p( 212,  253),
        p( 275,  304),    p( 286,  313),    p( 300,  310),    p( 313,  312),    p( 304,  309),    p( 326,  298),    p( 286,  309),    p( 290,  295),
        p( 291,  302),    p( 302,  306),    p( 319,  315),    p( 321,  319),    p( 337,  313),    p( 360,  303),    p( 313,  303),    p( 308,  300),
        p( 304,  310),    p( 310,  309),    p( 317,  320),    p( 343,  322),    p( 320,  324),    p( 334,  321),    p( 315,  312),    p( 332,  305),
        p( 300,  314),    p( 299,  308),    p( 305,  320),    p( 312,  324),    p( 319,  325),    p( 316,  312),    p( 327,  304),    p( 316,  309),
        p( 275,  301),    p( 276,  304),    p( 284,  304),    p( 290,  318),    p( 298,  314),    p( 283,  299),    p( 298,  295),    p( 294,  304),
        p( 271,  305),    p( 281,  309),    p( 277,  305),    p( 289,  309),    p( 292,  303),    p( 285,  300),    p( 295,  301),    p( 290,  314),
        p( 243,  301),    p( 282,  300),    p( 266,  302),    p( 285,  307),    p( 296,  304),    p( 291,  293),    p( 289,  300),    p( 267,  299),
    ],
    // bishop
    [
        p( 280,  315),    p( 255,  314),    p( 248,  307),    p( 224,  315),    p( 222,  315),    p( 225,  307),    p( 281,  305),    p( 252,  308),
        p( 279,  303),    p( 285,  305),    p( 288,  306),    p( 281,  308),    p( 284,  303),    p( 294,  303),    p( 272,  308),    p( 274,  303),
        p( 297,  309),    p( 303,  304),    p( 295,  310),    p( 303,  302),    p( 307,  305),    p( 333,  309),    p( 318,  303),    p( 311,  311),
        p( 281,  310),    p( 298,  309),    p( 300,  305),    p( 317,  311),    p( 311,  307),    p( 307,  309),    p( 302,  308),    p( 282,  311),
        p( 292,  307),    p( 281,  311),    p( 300,  309),    p( 315,  308),    p( 312,  308),    p( 298,  307),    p( 290,  309),    p( 311,  299),
        p( 293,  307),    p( 304,  310),    p( 301,  310),    p( 304,  310),    p( 307,  310),    p( 304,  307),    p( 306,  301),    p( 310,  300),
        p( 309,  311),    p( 303,  300),    p( 311,  303),    p( 296,  310),    p( 302,  308),    p( 303,  305),    p( 313,  301),    p( 303,  298),
        p( 294,  304),    p( 314,  309),    p( 306,  306),    p( 290,  311),    p( 303,  309),    p( 296,  313),    p( 304,  297),    p( 301,  295),
    ],
    // rook
    [
        p( 457,  550),    p( 449,  560),    p( 446,  565),    p( 444,  563),    p( 456,  559),    p( 475,  554),    p( 484,  552),    p( 494,  545),
        p( 432,  556),    p( 429,  562),    p( 438,  562),    p( 454,  552),    p( 444,  554),    p( 464,  549),    p( 475,  546),    p( 489,  537),
        p( 437,  553),    p( 455,  549),    p( 454,  550),    p( 457,  546),    p( 485,  534),    p( 493,  531),    p( 516,  527),    p( 487,  530),
        p( 435,  552),    p( 442,  548),    p( 443,  551),    p( 449,  546),    p( 457,  538),    p( 466,  532),    p( 472,  534),    p( 469,  530),
        p( 430,  548),    p( 429,  547),    p( 431,  548),    p( 437,  545),    p( 444,  541),    p( 438,  540),    p( 457,  533),    p( 447,  531),
        p( 427,  544),    p( 426,  543),    p( 429,  541),    p( 432,  542),    p( 439,  536),    p( 447,  528),    p( 470,  516),    p( 452,  520),
        p( 430,  539),    p( 433,  539),    p( 439,  541),    p( 442,  538),    p( 450,  532),    p( 464,  522),    p( 473,  516),    p( 441,  525),
        p( 439,  543),    p( 435,  539),    p( 436,  544),    p( 441,  539),    p( 449,  534),    p( 455,  533),    p( 452,  529),    p( 446,  530),
    ],
    // queen
    [
        p( 874,  968),    p( 876,  982),    p( 891,  995),    p( 907,  991),    p( 906,  995),    p( 925,  983),    p( 976,  931),    p( 921,  962),
        p( 883,  960),    p( 859,  992),    p( 861, 1019),    p( 853, 1037),    p( 861, 1047),    p( 900, 1008),    p( 903,  989),    p( 945,  966),
        p( 891,  965),    p( 883,  985),    p( 883, 1008),    p( 881, 1016),    p( 903, 1018),    p( 943, 1002),    p( 950,  971),    p( 938,  977),
        p( 877,  980),    p( 882,  990),    p( 876, 1000),    p( 875, 1014),    p( 880, 1024),    p( 892, 1014),    p( 902, 1013),    p( 909,  989),
        p( 887,  969),    p( 874,  989),    p( 880,  992),    p( 880, 1010),    p( 881, 1007),    p( 883, 1007),    p( 898,  992),    p( 905,  983),
        p( 883,  954),    p( 889,  972),    p( 882,  989),    p( 879,  992),    p( 884,  998),    p( 890,  989),    p( 905,  970),    p( 904,  957),
        p( 886,  952),    p( 883,  962),    p( 890,  965),    p( 889,  978),    p( 890,  978),    p( 892,  961),    p( 901,  939),    p( 911,  911),
        p( 872,  950),    p( 883,  940),    p( 883,  954),    p( 891,  956),    p( 893,  949),    p( 881,  949),    p( 882,  938),    p( 886,  926),
    ],
    // king
    [
        p( 150, -104),    p(  57,  -50),    p(  81,  -42),    p(   4,  -10),    p(  26,  -22),    p(  10,  -13),    p(  63,  -22),    p( 219, -107),
        p( -24,   -3),    p( -69,   27),    p( -78,   37),    p( -12,   26),    p( -45,   34),    p( -72,   47),    p( -37,   33),    p(  10,   -2),
        p( -45,    5),    p( -37,   23),    p( -82,   41),    p( -87,   49),    p( -53,   43),    p( -21,   36),    p( -58,   34),    p( -33,   10),
        p( -28,   -1),    p( -91,   22),    p(-107,   40),    p(-128,   49),    p(-127,   47),    p(-107,   39),    p(-112,   28),    p(-100,   14),
        p( -47,   -4),    p(-112,   18),    p(-122,   35),    p(-144,   48),    p(-150,   46),    p(-126,   32),    p(-140,   23),    p(-119,   12),
        p( -37,   -0),    p( -88,   13),    p(-117,   27),    p(-126,   37),    p(-120,   35),    p(-134,   28),    p(-105,   13),    p( -75,    9),
        p(  28,   -9),    p( -70,    7),    p( -83,   16),    p(-103,   25),    p(-109,   26),    p( -93,   17),    p( -62,    1),    p(   4,   -4),
        p(  46,  -43),    p(  42,  -48),    p(  37,  -36),    p( -24,  -15),    p(  29,  -34),    p( -20,  -19),    p(  36,  -44),    p(  62,  -53),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  33,   87),    p(  30,   86),    p(  20,   89),    p(  32,   69),    p(  19,   74),    p(  20,   77),    p( -17,   95),    p(  -9,   93),
        p(  41,  125),    p(  48,  126),    p(  37,  103),    p(  22,   69),    p(  37,   68),    p(  15,   97),    p(   1,  105),    p( -28,  125),
        p(  24,   75),    p(  18,   73),    p(  24,   57),    p(  16,   46),    p(  -1,   47),    p(   7,   61),    p( -11,   77),    p( -10,   79),
        p(   8,   48),    p(  -3,   47),    p( -14,   38),    p(  -7,   27),    p( -16,   32),    p( -10,   41),    p( -19,   57),    p( -11,   52),
        p(   1,   16),    p( -12,   26),    p( -13,   21),    p( -14,   12),    p( -13,   17),    p(  -8,   21),    p( -14,   39),    p(   9,   18),
        p(  -4,   17),    p(  -2,   23),    p(  -8,   21),    p(  -7,    8),    p(   7,    5),    p(   8,   11),    p(  12,   22),    p(   7,   15),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];

const UNSUPPORTED_PAWN: PhasedScore = p(-11, -12);

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(16, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(-0, 1);
const KING_OPEN_FILE: PhasedScore = p(-56, -2);
const KING_CLOSED_FILE: PhasedScore = p(15, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-10, 5);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 5), p(-2, 6), p(-2, 9), p(3, 7), p(4, 10), p(5, 12), p(10, 11), p(21, 7), ],
    // Closed
    [p(0, 0), p(0, 0), p(13, -30), p(-15, 8), p(1, 12), p(3, 3), p(2, 10), p(-0, 6), ],
    // SemiOpen
    [p(0, 0), p(-15, 22), p(2, 20), p(1, 14), p(-1, 19), p(3, 14), p(1, 11), p(12, 11), ],
    // SemiClosed
    [p(0, 0), p(11, -13), p(8, 6), p(5, 1), p(8, 4), p(4, 4), p(8, 7), p(3, 4), ],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-8, 8),    /*0b0000*/
    p(-16, 13),  /*0b0001*/
    p(-3, 10),   /*0b0010*/
    p(-10, 16),  /*0b0011*/
    p(-6, 8),    /*0b0100*/
    p(-26, 6),   /*0b0101*/
    p(-14, 8),   /*0b0110*/
    p(-19, -14), /*0b0111*/
    p(5, 12),    /*0b1000*/
    p(-10, -3),  /*0b1001*/
    p(2, 10),    /*0b1010*/
    p(-8, -7),   /*0b1011*/
    p(-2, 9),    /*0b1100*/
    p(-30, -3),  /*0b1101*/
    p(-13, 6),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(1, 19),    /*0b10000*/
    p(4, 13),    /*0b10001*/
    p(17, -2),   /*0b10010*/
    p(-8, -9),   /*0b10011*/
    p(-6, 9),    /*0b10100*/
    p(13, 21),   /*0b10101*/
    p(-27, -16), /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(11, 34),   /*0b11000*/
    p(26, 9),    /*0b11001*/
    p(37, 24),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(16, 16),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(13, 11),   /*0b100000*/
    p(4, 17),    /*0b100001*/
    p(25, 5),    /*0b100010*/
    p(6, 2),     /*0b100011*/
    p(-15, -12), /*0b100100*/
    p(-28, -21), /*0b100101*/
    p(-30, -2),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(19, 5),    /*0b101000*/
    p(-7, 5),    /*0b101001*/
    p(20, -1),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-11, -7),  /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(13, 22),   /*0b110000*/
    p(26, 19),   /*0b110001*/
    p(28, -5),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(2, 16),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(24, 18),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(1, -18),   /*0b111111*/
    p(-21, -10), /*0b00*/
    p(8, -26),   /*0b01*/
    p(36, -14),  /*0b10*/
    p(25, -51),  /*0b11*/
    p(46, -18),  /*0b100*/
    p(-10, -47), /*0b101*/
    p(74, -50),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(56, -20),  /*0b1000*/
    p(19, -44),  /*0b1001*/
    p(73, -80),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(56, -27),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(8, -29),   /*0b1111*/
    p(16, -10),  /*0b00*/
    p(33, -22),  /*0b01*/
    p(25, -27),  /*0b10*/
    p(24, -53),  /*0b11*/
    p(32, -17),  /*0b100*/
    p(50, -45),  /*0b101*/
    p(24, -34),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(37, -12),  /*0b1000*/
    p(55, -27),  /*0b1001*/
    p(45, -71),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(41, -31),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(17, -74),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(14, 8), p(2, 9), p(10, 14), p(9, 9), p(-5, 19), p(-46, 9)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(38, 9), p(42, 36), p(51, -9), p(37, -38), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-58, -57),
        p(-35, -19),
        p(-19, 2),
        p(-7, 13),
        p(4, 22),
        p(14, 30),
        p(25, 30),
        p(34, 29),
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
        p(-25, -47),
        p(-14, -30),
        p(-4, -14),
        p(3, -3),
        p(9, 6),
        p(13, 14),
        p(16, 18),
        p(18, 22),
        p(19, 26),
        p(25, 26),
        p(29, 24),
        p(37, 25),
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
        p(-75, 14),
        p(-66, 28),
        p(-61, 33),
        p(-58, 37),
        p(-58, 44),
        p(-53, 48),
        p(-50, 52),
        p(-46, 55),
        p(-42, 58),
        p(-39, 62),
        p(-33, 64),
        p(-30, 68),
        p(-20, 67),
        p(-8, 64),
        p(-4, 65),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-35, -35),
        p(-35, 21),
        p(-39, 70),
        p(-33, 87),
        p(-31, 104),
        p(-26, 109),
        p(-21, 119),
        p(-18, 126),
        p(-14, 130),
        p(-10, 132),
        p(-8, 135),
        p(-4, 138),
        p(-1, 139),
        p(0, 144),
        p(3, 145),
        p(7, 148),
        p(8, 154),
        p(10, 153),
        p(20, 149),
        p(34, 141),
        p(39, 141),
        p(82, 117),
        p(82, 118),
        p(105, 98),
        p(198, 63),
        p(250, 18),
        p(286, 2),
        p(335, -30),
    ],
    [
        p(-84, 52),
        p(-52, 23),
        p(-26, 12),
        p(1, 5),
        p(28, -2),
        p(48, -11),
        p(71, -11),
        p(92, -18),
        p(138, -44),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-11, 11),
        p(-6, -4),
        p(23, 17),
        p(49, -15),
        p(21, -44),
        p(0, 0),
    ],
    [p(-1, 13), p(19, 21), p(-2, 9), p(28, 2), p(27, 55), p(0, 0)],
    [
        p(3, 17),
        p(22, 21),
        p(23, 21),
        p(-6, 11),
        p(43, -4),
        p(0, 0),
    ],
    [p(-0, -2), p(7, 12), p(-0, 30), p(-0, 6), p(2, -17), p(0, 0)],
    [p(80, 33), p(-30, 22), p(2, 19), p(-33, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 5), p(11, 4), p(9, 10), p(15, 5), p(9, 16), p(13, 3)],
    [p(-3, 1), p(8, 17), p(-99, -35), p(6, 12), p(7, 16), p(4, 5)],
    [p(3, 2), p(14, 4), p(9, 10), p(11, 7), p(12, 15), p(22, -5)],
    [
        p(3, -4),
        p(10, -2),
        p(8, -8),
        p(4, 15),
        p(-57, -260),
        p(7, -11),
    ],
    [
        p(60, -9),
        p(38, -1),
        p(43, -6),
        p(21, -3),
        p(34, -19),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-13, -9),
    p(17, -8),
    p(17, -3),
    p(23, -13),
    p(6, 22),
    p(5, 19),
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

    fn unsupported_pawn() -> <Self::Score as ScoreType>::SingleFeatureScore;

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

    fn unsupported_pawn() -> <Self::Score as ScoreType>::SingleFeatureScore {
        UNSUPPORTED_PAWN    
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
