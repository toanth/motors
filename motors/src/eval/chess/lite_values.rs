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
        p( 133,  187),    p( 129,  186),    p( 119,  189),    p( 132,  169),    p( 119,  174),    p( 119,  177),    p(  82,  195),    p(  90,  193),
        p(  65,  123),    p(  63,  124),    p(  74,  120),    p(  82,  124),    p(  67,  125),    p( 117,  111),    p(  92,  132),    p(  88,  122),
        p(  51,  113),    p(  63,  109),    p(  61,  103),    p(  65,   97),    p(  82,   98),    p(  83,   94),    p(  77,  103),    p(  71,   96),
        p(  48,  100),    p(  55,  102),    p(  63,   95),    p(  73,   94),    p(  76,   93),    p(  77,   88),    p(  71,   92),    p(  60,   86),
        p(  44,   97),    p(  51,   94),    p(  55,   94),    p(  59,  100),    p(  67,   97),    p(  62,   93),    p(  70,   84),    p(  54,   85),
        p(  50,   98),    p(  51,   97),    p(  57,   98),    p(  57,  105),    p(  54,  108),    p(  72,   99),    p(  73,   85),    p(  55,   87),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 183,  271),    p( 209,  303),    p( 242,  315),    p( 266,  306),    p( 299,  307),    p( 211,  303),    p( 229,  302),    p( 212,  253),
        p( 275,  304),    p( 286,  313),    p( 298,  310),    p( 311,  313),    p( 302,  310),    p( 324,  298),    p( 284,  309),    p( 290,  296),
        p( 291,  302),    p( 300,  307),    p( 315,  316),    p( 314,  320),    p( 332,  314),    p( 358,  303),    p( 312,  303),    p( 307,  300),
        p( 303,  311),    p( 309,  309),    p( 315,  320),    p( 341,  322),    p( 319,  324),    p( 333,  321),    p( 314,  312),    p( 332,  305),
        p( 300,  314),    p( 299,  308),    p( 305,  320),    p( 312,  324),    p( 319,  325),    p( 317,  312),    p( 327,  304),    p( 317,  309),
        p( 276,  300),    p( 277,  304),    p( 285,  304),    p( 291,  318),    p( 298,  314),    p( 284,  299),    p( 299,  296),    p( 294,  304),
        p( 271,  305),    p( 281,  309),    p( 278,  304),    p( 289,  309),    p( 292,  303),    p( 285,  300),    p( 295,  301),    p( 291,  314),
        p( 244,  301),    p( 282,  299),    p( 267,  302),    p( 286,  307),    p( 296,  304),    p( 292,  293),    p( 290,  299),    p( 268,  299),
    ],
    // bishop
    [
        p( 280,  314),    p( 256,  314),    p( 248,  307),    p( 224,  316),    p( 223,  315),    p( 225,  307),    p( 282,  304),    p( 252,  307),
        p( 279,  303),    p( 283,  307),    p( 286,  308),    p( 280,  309),    p( 282,  304),    p( 292,  304),    p( 270,  309),    p( 274,  304),
        p( 297,  310),    p( 300,  307),    p( 292,  313),    p( 296,  307),    p( 301,  309),    p( 331,  310),    p( 317,  305),    p( 311,  312),
        p( 280,  311),    p( 297,  310),    p( 297,  308),    p( 313,  315),    p( 309,  309),    p( 306,  310),    p( 302,  308),    p( 282,  312),
        p( 291,  308),    p( 281,  311),    p( 300,  309),    p( 314,  309),    p( 313,  308),    p( 299,  305),    p( 291,  308),    p( 311,  300),
        p( 293,  307),    p( 305,  308),    p( 301,  309),    p( 304,  309),    p( 308,  309),    p( 305,  305),    p( 308,  299),    p( 310,  300),
        p( 309,  311),    p( 303,  301),    p( 310,  303),    p( 296,  310),    p( 302,  308),    p( 303,  306),    p( 313,  301),    p( 303,  298),
        p( 294,  304),    p( 315,  308),    p( 306,  307),    p( 290,  312),    p( 303,  309),    p( 296,  313),    p( 304,  297),    p( 301,  295),
    ],
    // rook
    [
        p( 459,  550),    p( 450,  559),    p( 447,  565),    p( 445,  562),    p( 457,  558),    p( 476,  553),    p( 485,  551),    p( 495,  545),
        p( 433,  556),    p( 430,  561),    p( 440,  562),    p( 456,  552),    p( 445,  554),    p( 466,  549),    p( 476,  545),    p( 490,  536),
        p( 438,  552),    p( 456,  548),    p( 453,  549),    p( 456,  545),    p( 484,  533),    p( 494,  530),    p( 517,  526),    p( 488,  529),
        p( 435,  552),    p( 443,  548),    p( 443,  551),    p( 448,  545),    p( 458,  537),    p( 467,  532),    p( 474,  534),    p( 470,  529),
        p( 431,  548),    p( 431,  547),    p( 432,  548),    p( 438,  545),    p( 445,  541),    p( 439,  541),    p( 459,  533),    p( 448,  531),
        p( 429,  544),    p( 428,  543),    p( 431,  541),    p( 433,  542),    p( 441,  537),    p( 449,  529),    p( 472,  517),    p( 454,  520),
        p( 431,  538),    p( 435,  538),    p( 441,  540),    p( 444,  537),    p( 451,  531),    p( 465,  521),    p( 474,  516),    p( 442,  524),
        p( 440,  542),    p( 436,  538),    p( 438,  543),    p( 442,  539),    p( 450,  533),    p( 456,  532),    p( 453,  529),    p( 447,  529),
    ],
    // queen
    [
        p( 875,  968),    p( 877,  982),    p( 892,  995),    p( 908,  992),    p( 907,  995),    p( 927,  983),    p( 977,  931),    p( 923,  962),
        p( 884,  960),    p( 860,  992),    p( 862, 1019),    p( 855, 1037),    p( 862, 1047),    p( 901, 1008),    p( 905,  988),    p( 947,  966),
        p( 892,  965),    p( 885,  985),    p( 885, 1007),    p( 882, 1015),    p( 905, 1017),    p( 944, 1002),    p( 952,  971),    p( 939,  977),
        p( 877,  980),    p( 884,  989),    p( 877,  999),    p( 876, 1013),    p( 881, 1023),    p( 893, 1014),    p( 903, 1012),    p( 910,  989),
        p( 889,  969),    p( 875,  989),    p( 881,  992),    p( 882, 1010),    p( 882, 1007),    p( 885, 1007),    p( 899,  992),    p( 906,  983),
        p( 884,  954),    p( 890,  973),    p( 883,  989),    p( 880,  992),    p( 885,  999),    p( 892,  989),    p( 906,  971),    p( 906,  956),
        p( 887,  952),    p( 884,  961),    p( 891,  965),    p( 890,  978),    p( 891,  978),    p( 893,  961),    p( 903,  939),    p( 913,  911),
        p( 874,  950),    p( 884,  940),    p( 884,  954),    p( 892,  955),    p( 894,  949),    p( 882,  949),    p( 884,  938),    p( 887,  925),
    ],
    // king
    [
        p( 153, -105),    p(  57,  -51),    p(  79,  -41),    p(   5,  -10),    p(  23,  -21),    p(   9,  -12),    p(  64,  -23),    p( 219, -107),
        p( -22,   -4),    p( -66,   25),    p( -76,   36),    p(  -9,   25),    p( -44,   34),    p( -69,   46),    p( -36,   32),    p(  10,   -2),
        p( -45,    5),    p( -36,   22),    p( -82,   40),    p( -86,   47),    p( -53,   42),    p( -21,   35),    p( -58,   33),    p( -31,   10),
        p( -27,   -2),    p( -92,   22),    p(-106,   39),    p(-129,   48),    p(-127,   46),    p(-109,   39),    p(-114,   28),    p(-101,   15),
        p( -48,   -4),    p(-113,   18),    p(-123,   35),    p(-146,   48),    p(-151,   46),    p(-128,   33),    p(-140,   24),    p(-119,   12),
        p( -38,   -0),    p( -88,   14),    p(-118,   28),    p(-126,   37),    p(-121,   36),    p(-135,   29),    p(-106,   15),    p( -75,    9),
        p(  28,  -10),    p( -70,    8),    p( -82,   16),    p(-103,   26),    p(-109,   26),    p( -94,   17),    p( -62,    1),    p(   4,   -4),
        p(  46,  -44),    p(  42,  -48),    p(  37,  -36),    p( -25,  -15),    p(  29,  -33),    p( -20,  -19),    p(  36,  -44),    p(  62,  -54),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  33,   87),    p(  29,   86),    p(  19,   89),    p(  32,   69),    p(  19,   74),    p(  19,   77),    p( -18,   95),    p( -10,   93),
        p(  39,  123),    p(  43,  123),    p(  33,  100),    p(  18,   68),    p(  32,   67),    p(  11,   95),    p(  -4,  103),    p( -30,  123),
        p(  21,   73),    p(  11,   71),    p(  16,   55),    p(   8,   45),    p( -10,   46),    p(  -0,   59),    p( -18,   76),    p( -13,   78),
        p(   2,   47),    p( -11,   45),    p( -23,   36),    p( -17,   26),    p( -24,   31),    p( -20,   40),    p( -27,   55),    p( -17,   51),
        p(  -5,   15),    p( -23,   25),    p( -22,   18),    p( -21,   10),    p( -23,   15),    p( -16,   18),    p( -27,   39),    p(   0,   18),
        p(  -5,   15),    p(  -6,   20),    p( -12,   17),    p(  -9,    5),    p(   4,    1),    p(   4,    7),    p(   9,   18),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-11, -10);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);

const BISHOP_PAIR: PhasedScore = p(24, 58);
const ROOK_OPEN_FILE: PhasedScore = p(16, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -2);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(0, 3);
const KING_OPEN_FILE: PhasedScore = p(-56, -2);
const KING_CLOSED_FILE: PhasedScore = p(16, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 7);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 5), p(-2, 6), p(-2, 9), p(4, 6), p(4, 9), p(5, 11), p(10, 11), p(21, 6), ],
    // Closed
    [p(0, 0), p(0, 0), p(13, -30), p(-16, 10), p(-0, 13), p(3, 5), p(2, 10), p(-0, 7), ],
    // SemiOpen
    [p(0, 0), p(-17, 23), p(2, 20), p(1, 14), p(-1, 19), p(4, 14), p(1, 12), p(12, 11), ],
    // SemiClosed
    [p(0, 0), p(10, -10), p(7, 8), p(5, 2), p(8, 6), p(3, 5), p(8, 8), p(2, 5), ],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-8, 7),    /*0b0000*/
    p(-16, 13),  /*0b0001*/
    p(-3, 8),    /*0b0010*/
    p(-10, 15),  /*0b0011*/
    p(-6, 7),    /*0b0100*/
    p(-27, 5),   /*0b0101*/
    p(-15, 7),   /*0b0110*/
    p(-19, -16), /*0b0111*/
    p(5, 11),    /*0b1000*/
    p(-5, 12),   /*0b1001*/
    p(1, 9),     /*0b1010*/
    p(-3, 12),   /*0b1011*/
    p(-2, 7),    /*0b1100*/
    p(-25, 10),  /*0b1101*/
    p(-13, 5),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(1, 18),    /*0b10000*/
    p(4, 12),    /*0b10001*/
    p(21, 13),   /*0b10010*/
    p(-3, 10),   /*0b10011*/
    p(-6, 9),    /*0b10100*/
    p(13, 18),   /*0b10101*/
    p(-22, 3),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(11, 33),   /*0b11000*/
    p(31, 26),   /*0b11001*/
    p(41, 39),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(15, 14),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(14, 10),   /*0b100000*/
    p(4, 15),    /*0b100001*/
    p(25, 4),    /*0b100010*/
    p(6, 2),     /*0b100011*/
    p(-10, 4),   /*0b100100*/
    p(-24, -6),  /*0b100101*/
    p(-25, 17),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(18, 2),    /*0b101000*/
    p(-3, 18),   /*0b101001*/
    p(19, -3),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-7, 7),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(13, 21),   /*0b110000*/
    p(25, 17),   /*0b110001*/
    p(32, 12),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(8, 32),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(22, 16),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(5, -0),    /*0b111111*/
    p(-22, -10), /*0b00*/
    p(8, -26),   /*0b01*/
    p(37, -14),  /*0b10*/
    p(24, -51),  /*0b11*/
    p(47, -19),  /*0b100*/
    p(-4, -29),  /*0b101*/
    p(73, -49),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(56, -20),  /*0b1000*/
    p(20, -45),  /*0b1001*/
    p(76, -64),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(55, -27),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(14, -5),   /*0b1111*/
    p(16, -11),  /*0b00*/
    p(32, -21),  /*0b01*/
    p(26, -28),  /*0b10*/
    p(24, -54),  /*0b11*/
    p(31, -19),  /*0b100*/
    p(54, -30),  /*0b101*/
    p(23, -34),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(37, -13),  /*0b1000*/
    p(54, -28),  /*0b1001*/
    p(51, -54),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(41, -32),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(23, -55),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 7), p(1, 8), p(7, 20), p(9, 1), p(-4, 16), p(-42, -0)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(37, 8),
    p(41, 36),
    p(51, -10),
    p(37, -39),
    p(0, 0),
];

const OUTPOSTS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(18, -5),
    p(17, -0),
    p(26, -21),
    p(12, 14),
    p(-1, 10),
    p(25, 20),
];

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
        p(-25, -48),
        p(-13, -30),
        p(-4, -14),
        p(3, -3),
        p(9, 6),
        p(13, 14),
        p(16, 19),
        p(18, 22),
        p(19, 27),
        p(25, 27),
        p(29, 25),
        p(39, 25),
        p(33, 32),
        p(46, 23),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-74, 14),
        p(-65, 28),
        p(-60, 33),
        p(-57, 37),
        p(-57, 44),
        p(-52, 48),
        p(-49, 52),
        p(-45, 54),
        p(-41, 58),
        p(-37, 62),
        p(-32, 63),
        p(-29, 67),
        p(-19, 67),
        p(-7, 64),
        p(-4, 64),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-33, -35),
        p(-34, 22),
        p(-37, 70),
        p(-32, 88),
        p(-29, 105),
        p(-24, 110),
        p(-20, 120),
        p(-16, 127),
        p(-12, 131),
        p(-9, 133),
        p(-6, 136),
        p(-2, 139),
        p(1, 140),
        p(2, 144),
        p(5, 146),
        p(8, 148),
        p(9, 154),
        p(12, 153),
        p(21, 150),
        p(36, 141),
        p(41, 141),
        p(84, 117),
        p(83, 119),
        p(109, 98),
        p(199, 63),
        p(250, 19),
        p(288, 3),
        p(338, -30),
    ],
    [
        p(-85, 53),
        p(-53, 25),
        p(-26, 13),
        p(0, 5),
        p(28, -2),
        p(48, -11),
        p(71, -12),
        p(91, -19),
        p(137, -43),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(50, -15),
        p(22, -44),
        p(0, 0),
    ],
    [p(-1, 12), p(19, 20), p(-2, 8), p(29, 1), p(27, 55), p(0, 0)],
    [p(2, 17), p(21, 21), p(22, 21), p(-9, 8), p(42, -5), p(0, 0)],
    [p(-1, -2), p(7, 11), p(-1, 29), p(-0, 6), p(2, -19), p(0, 0)],
    [p(77, 34), p(-32, 22), p(1, 20), p(-33, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(11, 4), p(9, 10), p(15, 5), p(9, 16), p(14, 3)],
    [
        p(-2, -1),
        p(7, 18),
        p(-92, -36),
        p(6, 12),
        p(7, 17),
        p(4, 5),
    ],
    [p(2, 2), p(13, 4), p(9, 10), p(11, 7), p(12, 15), p(22, -6)],
    [
        p(3, -5),
        p(10, -2),
        p(8, -8),
        p(4, 15),
        p(-56, -264),
        p(7, -11),
    ],
    [
        p(60, -9),
        p(38, -1),
        p(43, -6),
        p(21, -3),
        p(34, -20),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-12, -10),
    p(17, -9),
    p(17, -2),
    p(23, -13),
    p(6, 22),
    p(8, 19),
];

#[allow(type_alias_bounds)]
pub type SingleFeatureScore<L: LiteValues> = <L::Score as ScoreType>::SingleFeatureScore;

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static {
    type Score: ScoreType;

    fn psqt(
        square: ChessSquare,
        piece: ChessPieceType,
        color: ChessColor,
    ) -> SingleFeatureScore<Self>;

    fn passed_pawn(square: ChessSquare) -> SingleFeatureScore<Self>;

    fn unsupported_pawn() -> SingleFeatureScore<Self>;

    fn doubled_pawn() -> SingleFeatureScore<Self>;

    fn bishop_pair() -> SingleFeatureScore<Self>;

    fn rook_openness(openness: FileOpenness) -> SingleFeatureScore<Self>;

    fn king_openness(openness: FileOpenness) -> SingleFeatureScore<Self>;

    fn bishop_openness(openness: FileOpenness, len: usize) -> SingleFeatureScore<Self>;

    fn outpost(piece: ChessPieceType) -> SingleFeatureScore<Self>;

    fn pawn_shield(config: usize) -> SingleFeatureScore<Self>;

    fn pawn_protection(piece: ChessPieceType) -> SingleFeatureScore<Self>;

    fn pawn_attack(piece: ChessPieceType) -> SingleFeatureScore<Self>;

    fn mobility(piece: ChessPieceType, mobility: usize) -> SingleFeatureScore<Self>;

    fn threats(attacking: ChessPieceType, targeted: ChessPieceType) -> SingleFeatureScore<Self>;

    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> SingleFeatureScore<Self>;

    fn king_zone_attack(attacking: ChessPieceType) -> SingleFeatureScore<Self>;
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

    fn unsupported_pawn() -> SingleFeatureScore<Self> {
        UNSUPPORTED_PAWN
    }

    fn doubled_pawn() -> SingleFeatureScore<Self> {
        DOUBLED_PAWN
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

    fn bishop_openness(openness: FileOpenness, len: usize) -> SingleFeatureScore<Self> {
        BISHOP_OPENNESS[openness as usize][len - 1]
    }

    fn outpost(piece: ChessPieceType) -> SingleFeatureScore<Self> {
        OUTPOSTS[piece as usize]
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

    fn king_zone_attack(attacking: ChessPieceType) -> SingleFeatureScore<Self> {
        KING_ZONE_ATTACK[attacking as usize]
    }
}
