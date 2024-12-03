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
use gears::games::chess::pieces::ChessPieceType::Bishop;
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
        p(  50,  113),    p(  62,  109),    p(  59,  104),    p(  63,   98),    p(  78,   99),    p(  82,   94),    p(  75,  104),    p(  69,   95),
        p(  46,  100),    p(  53,  102),    p(  63,   95),    p(  71,   93),    p(  75,   93),    p(  75,   88),    p(  68,   93),    p(  57,   86),
        p(  42,   97),    p(  51,   93),    p(  54,   94),    p(  58,  100),    p(  65,   96),    p(  60,   92),    p(  68,   83),    p(  52,   85),
        p(  47,   99),    p(  51,   96),    p(  56,   98),    p(  55,  105),    p(  52,  107),    p(  70,   98),    p(  71,   84),    p(  53,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 177,  275),    p( 197,  307),    p( 215,  319),    p( 252,  308),    p( 283,  309),    p( 198,  305),    p( 216,  305),    p( 205,  257),
        p( 270,  308),    p( 284,  314),    p( 298,  307),    p( 302,  310),    p( 302,  305),    p( 315,  295),    p( 277,  310),    p( 271,  300),
        p( 287,  305),    p( 303,  304),    p( 304,  312),    p( 318,  315),    p( 336,  308),    p( 347,  298),    p( 291,  303),    p( 286,  304),
        p( 301,  313),    p( 307,  309),    p( 321,  315),    p( 324,  322),    p( 322,  319),    p( 317,  319),    p( 308,  312),    p( 318,  309),
        p( 298,  316),    p( 301,  306),    p( 310,  315),    p( 319,  318),    p( 317,  321),    p( 321,  304),    p( 320,  303),    p( 312,  311),
        p( 274,  302),    p( 280,  303),    p( 292,  299),    p( 298,  312),    p( 304,  309),    p( 293,  292),    p( 299,  293),    p( 292,  306),
        p( 269,  310),    p( 280,  313),    p( 282,  304),    p( 293,  309),    p( 297,  304),    p( 287,  301),    p( 293,  305),    p( 288,  319),
        p( 239,  308),    p( 280,  303),    p( 265,  305),    p( 285,  310),    p( 294,  308),    p( 290,  298),    p( 286,  306),    p( 264,  306),
    ],
    // bishop
    [
        p( 280,  315),    p( 255,  315),    p( 241,  308),    p( 220,  317),    p( 217,  316),    p( 223,  307),    p( 279,  306),    p( 253,  309),
        p( 285,  303),    p( 282,  305),    p( 290,  307),    p( 278,  308),    p( 288,  303),    p( 293,  302),    p( 270,  307),    p( 272,  304),
        p( 297,  310),    p( 307,  305),    p( 292,  310),    p( 307,  302),    p( 302,  305),    p( 334,  308),    p( 317,  303),    p( 316,  311),
        p( 283,  312),    p( 289,  311),    p( 304,  305),    p( 307,  311),    p( 307,  307),    p( 299,  309),    p( 295,  311),    p( 279,  312),
        p( 289,  311),    p( 282,  311),    p( 296,  306),    p( 310,  307),    p( 305,  305),    p( 298,  305),    p( 286,  309),    p( 308,  304),
        p( 295,  309),    p( 301,  308),    p( 300,  308),    p( 302,  306),    p( 307,  308),    p( 302,  303),    p( 307,  300),    p( 308,  301),
        p( 306,  312),    p( 303,  300),    p( 311,  302),    p( 297,  310),    p( 303,  309),    p( 304,  306),    p( 313,  299),    p( 306,  299),
        p( 296,  305),    p( 313,  308),    p( 307,  307),    p( 290,  313),    p( 304,  311),    p( 295,  315),    p( 306,  300),    p( 302,  296),
    ],
    // rook
    [
        p( 453,  547),    p( 441,  557),    p( 433,  564),    p( 432,  562),    p( 443,  557),    p( 472,  549),    p( 480,  548),    p( 490,  541),
        p( 445,  552),    p( 443,  557),    p( 452,  558),    p( 467,  549),    p( 452,  551),    p( 467,  546),    p( 477,  543),    p( 491,  534),
        p( 446,  546),    p( 464,  542),    p( 458,  544),    p( 458,  539),    p( 485,  529),    p( 493,  526),    p( 509,  526),    p( 486,  527),
        p( 442,  547),    p( 448,  543),    p( 447,  546),    p( 454,  540),    p( 458,  532),    p( 469,  528),    p( 467,  532),    p( 469,  527),
        p( 435,  545),    p( 435,  543),    p( 436,  544),    p( 442,  540),    p( 447,  537),    p( 442,  536),    p( 454,  530),    p( 449,  528),
        p( 431,  543),    p( 431,  539),    p( 434,  538),    p( 437,  538),    p( 444,  532),    p( 454,  524),    p( 469,  513),    p( 455,  517),
        p( 433,  538),    p( 437,  537),    p( 443,  538),    p( 446,  535),    p( 453,  528),    p( 468,  518),    p( 473,  513),    p( 443,  523),
        p( 443,  541),    p( 439,  537),    p( 441,  540),    p( 445,  534),    p( 451,  528),    p( 457,  527),    p( 453,  526),    p( 449,  529),
    ],
    // queen
    [
        p( 874,  955),    p( 877,  969),    p( 890,  983),    p( 911,  976),    p( 909,  980),    p( 936,  964),    p( 975,  921),    p( 920,  952),
        p( 895,  942),    p( 869,  972),    p( 872,  998),    p( 863, 1016),    p( 872, 1025),    p( 910,  988),    p( 913,  971),    p( 953,  952),
        p( 898,  950),    p( 891,  966),    p( 889,  989),    p( 892,  995),    p( 907, 1000),    p( 951,  981),    p( 957,  954),    p( 948,  959),
        p( 885,  962),    p( 887,  973),    p( 885,  980),    p( 883,  993),    p( 886, 1005),    p( 902,  994),    p( 909,  996),    p( 918,  971),
        p( 892,  958),    p( 881,  973),    p( 887,  973),    p( 888,  990),    p( 890,  988),    p( 893,  987),    p( 906,  976),    p( 914,  968),
        p( 889,  943),    p( 895,  957),    p( 890,  973),    p( 888,  977),    p( 894,  983),    p( 901,  971),    p( 914,  955),    p( 913,  944),
        p( 888,  944),    p( 889,  953),    p( 896,  954),    p( 895,  970),    p( 897,  969),    p( 898,  951),    p( 909,  931),    p( 917,  903),
        p( 877,  944),    p( 887,  933),    p( 888,  946),    p( 896,  948),    p( 899,  938),    p( 886,  942),    p( 887,  934),    p( 894,  914),
    ],
    // king
    [
        p( 153,  -84),    p(  59,  -38),    p(  83,  -30),    p(  12,    1),    p(  36,  -11),    p(  23,   -1),    p(  78,  -11),    p( 224,  -87),
        p( -29,    1),    p( -79,   20),    p( -83,   27),    p( -26,   18),    p( -55,   25),    p( -82,   39),    p( -52,   25),    p(   5,    1),
        p( -48,   10),    p( -50,   14),    p( -88,   30),    p( -96,   37),    p( -66,   32),    p( -33,   24),    p( -78,   26),    p( -37,   11),
        p( -26,    2),    p(-101,   13),    p(-115,   29),    p(-137,   38),    p(-138,   36),    p(-114,   28),    p(-132,   18),    p(-104,   17),
        p( -40,   -2),    p(-117,    9),    p(-128,   26),    p(-152,   39),    p(-154,   37),    p(-129,   23),    p(-145,   13),    p(-117,   13),
        p( -33,    2),    p( -93,    4),    p(-122,   19),    p(-128,   28),    p(-124,   26),    p(-134,   19),    p(-111,    5),    p( -73,   11),
        p(  25,   -7),    p( -80,   -1),    p( -92,    8),    p(-110,   17),    p(-116,   18),    p(-101,    8),    p( -74,   -8),    p(   3,   -4),
        p(  54,  -24),    p(  41,  -35),    p(  39,  -23),    p( -22,   -2),    p(  30,  -19),    p( -19,   -6),    p(  34,  -29),    p(  66,  -34),
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
    [p(-6, 6), p(0, 7), p(-1, 8), p(3, 6), p(3, 7), p(4, 10), p(6, 9), p(19, 4)],
    // Closed
    [p(0, 0), p(0, 0), p(13, -19), p(-15, 11), p(0, 11), p(2, 5), p(1, 8), p(-0, 5)],
    // SemiOpen
    [p(0, 0), p(-16, 25), p(3, 21), p(2, 13), p(1, 14), p(5, 10), p(1, 8), p(11, 10)],
    // SemiClosed
    [p(0, 0), p(11, -12), p(6, 7), p(3, 1), p(7, 3), p(3, 4), p(5, 6), p(1, 4)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-4, 6),    /*0b0000*/
    p(-14, 9),   /*0b0001*/
    p(-1, 8),    /*0b0010*/
    p(-11, 13),  /*0b0011*/
    p(-3, 4),    /*0b0100*/
    p(-25, 0),   /*0b0101*/
    p(-13, 5),   /*0b0110*/
    p(-20, -17), /*0b0111*/
    p(9, 11),    /*0b1000*/
    p(-1, 10),   /*0b1001*/
    p(4, 10),    /*0b1010*/
    p(-4, 10),   /*0b1011*/
    p(0, 5),     /*0b1100*/
    p(-22, 10),  /*0b1101*/
    p(-10, 4),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(4, 16),    /*0b10000*/
    p(3, 9),     /*0b10001*/
    p(23, 11),   /*0b10010*/
    p(-5, 5),    /*0b10011*/
    p(-4, 6),    /*0b10100*/
    p(12, 15),   /*0b10101*/
    p(-22, -0),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(16, 30),   /*0b11000*/
    p(29, 24),   /*0b11001*/
    p(43, 38),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(18, 11),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(17, 10),   /*0b100000*/
    p(4, 13),    /*0b100001*/
    p(26, 3),    /*0b100010*/
    p(6, -2),    /*0b100011*/
    p(-5, 2),    /*0b100100*/
    p(-19, -7),  /*0b100101*/
    p(-22, 15),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(22, 4),    /*0b101000*/
    p(2, 18),    /*0b101001*/
    p(22, -2),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-3, 8),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(17, 18),   /*0b110000*/
    p(25, 13),   /*0b110001*/
    p(34, 9),    /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(11, 30),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(26, 16),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(6, -2),    /*0b111111*/
    p(-15, -3),  /*0b00*/
    p(12, -17),  /*0b01*/
    p(38, -8),   /*0b10*/
    p(20, -39),  /*0b11*/
    p(48, -10),  /*0b100*/
    p(7, -20),   /*0b101*/
    p(70, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(64, -12),  /*0b1000*/
    p(21, -33),  /*0b1001*/
    p(90, -57),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(61, -19),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(20, -9),   /*0b1111*/
    p(21, -3),   /*0b00*/
    p(33, -12),  /*0b01*/
    p(28, -17),  /*0b10*/
    p(22, -41),  /*0b11*/
    p(38, -10),  /*0b100*/
    p(56, -20),  /*0b101*/
    p(26, -22),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(40, -4),   /*0b1000*/
    p(52, -17),  /*0b1001*/
    p(53, -41),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(45, -23),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(19, -42),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  31,   87),    p(  28,   86),    p(  19,   89),    p(  30,   69),    p(  17,   74),    p(  16,   77),    p( -21,   95),    p( -14,   93),
        p(  39,  124),    p(  48,  123),    p(  37,  100),    p(  21,   69),    p(  35,   69),    p(  15,   96),    p(  -1,  105),    p( -31,  125),
        p(  23,   74),    p(  17,   71),    p(  22,   54),    p(  16,   43),    p(  -1,   46),    p(   8,   59),    p( -10,   76),    p( -10,   79),
        p(   8,   47),    p(  -2,   44),    p( -15,   35),    p(  -9,   25),    p( -17,   29),    p( -10,   40),    p( -17,   55),    p( -10,   51),
        p(   1,   15),    p( -12,   24),    p( -15,   17),    p( -16,    8),    p( -16,   14),    p(  -8,   18),    p( -14,   38),    p(   9,   17),
        p(  -4,   15),    p(  -2,   20),    p(  -8,   17),    p(  -8,    4),    p(   4,    2),    p(   6,    8),    p(  12,   19),    p(   7,   14),
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
    [p(0, 0), p(36, 8), p(37, 36), p(49, -8), p(33, -33), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-50, -67),
        p(-30, -27),
        p(-16, -4),
        p(-7, 8),
        p(1, 18),
        p(9, 28),
        p(18, 30),
        p(26, 30),
        p(33, 27),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-28, -53),
        p(-17, -36),
        p(-6, -20),
        p(1, -7),
        p(7, 4),
        p(12, 12),
        p(17, 17),
        p(21, 21),
        p(23, 27),
        p(30, 27),
        p(37, 27),
        p(46, 28),
        p(41, 36),
        p(58, 28),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-75, 13),
        p(-67, 26),
        p(-63, 31),
        p(-59, 36),
        p(-60, 42),
        p(-54, 47),
        p(-51, 52),
        p(-47, 55),
        p(-43, 59),
        p(-40, 62),
        p(-36, 65),
        p(-34, 69),
        p(-26, 69),
        p(-16, 66),
        p(-13, 65),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-23, -50),
        p(-23, 5),
        p(-27, 54),
        p(-22, 70),
        p(-20, 88),
        p(-15, 93),
        p(-11, 104),
        p(-8, 111),
        p(-4, 115),
        p(-0, 117),
        p(3, 120),
        p(7, 122),
        p(10, 123),
        p(11, 127),
        p(14, 128),
        p(18, 131),
        p(19, 138),
        p(22, 138),
        p(31, 135),
        p(45, 128),
        p(49, 129),
        p(94, 103),
        p(94, 106),
        p(120, 85),
        p(221, 46),
        p(260, 8),
        p(294, -5),
        p(363, -52),
    ],
    [
        p(-94, 10),
        p(-58, -6),
        p(-29, -6),
        p(2, -3),
        p(34, -2),
        p(58, -4),
        p(86, 3),
        p(112, 1),
        p(159, -16),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-2, 11), p(20, 20), p(0, 0), p(31, -3), p(30, 38), p(0, 0)],
    [p(-3, 13), p(11, 12), p(18, 7), p(0, 0), p(42, -15), p(0, 0)],
    [p(-2, 5), p(2, 6), p(0, 17), p(2, -4), p(0, 0), p(0, 0)],
    [
        p(71, 28),
        p(-35, 17),
        p(-9, 17),
        p(-26, 8),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 6), p(8, 5), p(6, 8), p(12, 5), p(7, 14), p(10, 4)],
    [
        p(-1, 1),
        p(9, 20),
        p(-121, -23),
        p(7, 13),
        p(8, 17),
        p(2, 5),
    ],
    [p(1, 2), p(12, 5), p(7, 10), p(9, 8), p(8, 19), p(18, -5)],
    [
        p(2, -2),
        p(8, -1),
        p(6, -7),
        p(4, 11),
        p(-74, -249),
        p(2, -9),
    ],
    [p(62, -1), p(41, 8), p(46, 3), p(25, 5), p(37, -9), p(0, 0)],
];

pub const NUM_DEFENDED: [PhasedScore; 17] = [
    p(26, -0),
    p(-18, 1),
    p(-24, -0),
    p(-20, -2),
    p(-13, -4),
    p(-13, -4),
    p(-14, -1),
    p(-10, -0),
    p(-6, 3),
    p(-2, 6),
    p(2, 13),
    p(6, 16),
    p(11, 18),
    p(13, 23),
    p(15, 11),
    p(14, -9),
    p(11, 205),
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-20, -18),
    p(19, -10),
    p(11, -4),
    p(14, -12),
    p(-3, 13),
    p(-13, 12),
];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(12, 20), p(34, -1), p(6, 31)];
const PINNED: [[PhasedScore; 3]; NUM_CHESS_PIECES - 2] = [
    [p(-25, -56), p(-20, -29), p(-27, -3)],
    [p(-20, -8), p(-14, -38), p(-19, -17)],
    [p(-94, -259), p(-8, -12), p(-29, -41)],
    [p(-110, -789), p(-223, -523), p(-22, -30)],
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

    fn num_defended(num: usize) -> SingleFeatureScore<Self::Score>;

    fn king_zone_attack(attacking: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn can_give_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn pinned(pinning: ChessPieceType, piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;
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

    fn pinned(pinning: ChessPieceType, piece: ChessPieceType) -> SingleFeatureScore<Self::Score> {
        println!(
            "{0} {1} {2}",
            pinning as usize,
            Bishop as usize,
            pinning as usize - Bishop as usize
        );
        PINNED[piece as usize - 1][pinning as usize - Bishop as usize]
    }
}
