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
        p( 142,  187),    p( 139,  186),    p( 128,  189),    p( 141,  170),    p( 127,  175),    p( 125,  179),    p(  94,  195),    p(  98,  194),
        p(  74,  124),    p(  70,  124),    p(  83,  121),    p(  90,  124),    p(  78,  124),    p( 126,  110),    p( 101,  132),    p(  98,  122),
        p(  59,  114),    p(  70,  109),    p(  67,  103),    p(  72,   96),    p(  88,   98),    p(  90,   94),    p(  84,  103),    p(  78,   96),
        p(  54,  100),    p(  60,  103),    p(  69,   95),    p(  79,   94),    p(  82,   92),    p(  83,   89),    p(  77,   92),    p(  66,   86),
        p(  48,   98),    p(  57,   95),    p(  60,   95),    p(  64,  100),    p(  72,   97),    p(  67,   93),    p(  75,   84),    p(  59,   86),
        p(  53,   99),    p(  56,   97),    p(  61,   98),    p(  60,  105),    p(  58,  108),    p(  76,   99),    p(  78,   85),    p(  58,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 185,  270),    p( 216,  302),    p( 257,  313),    p( 281,  303),    p( 303,  306),    p( 236,  298),    p( 237,  300),    p( 213,  253),
        p( 281,  304),    p( 293,  314),    p( 303,  311),    p( 314,  314),    p( 304,  310),    p( 334,  301),    p( 294,  310),    p( 302,  293),
        p( 297,  302),    p( 305,  308),    p( 323,  318),    p( 326,  321),    p( 341,  315),    p( 363,  306),    p( 317,  305),    p( 312,  300),
        p( 309,  310),    p( 314,  310),    p( 323,  322),    p( 347,  325),    p( 323,  327),    p( 338,  324),    p( 319,  312),    p( 339,  305),
        p( 303,  314),    p( 305,  309),    p( 310,  323),    p( 317,  326),    p( 323,  328),    p( 320,  315),    p( 332,  306),    p( 319,  310),
        p( 277,  301),    p( 281,  304),    p( 287,  307),    p( 296,  320),    p( 303,  317),    p( 297,  299),    p( 304,  296),    p( 302,  303),
        p( 274,  305),    p( 283,  310),    p( 282,  306),    p( 292,  311),    p( 303,  303),    p( 293,  301),    p( 303,  300),    p( 297,  312),
        p( 251,  299),    p( 277,  300),    p( 265,  304),    p( 283,  309),    p( 293,  306),    p( 285,  298),    p( 281,  303),    p( 277,  297),
    ],
    // bishop
    [
        p( 277,  316),    p( 261,  313),    p( 258,  306),    p( 233,  313),    p( 230,  314),    p( 240,  305),    p( 284,  304),    p( 259,  308),
        p( 288,  302),    p( 295,  306),    p( 295,  306),    p( 288,  308),    p( 291,  303),    p( 299,  304),    p( 283,  308),    p( 276,  304),
        p( 305,  309),    p( 306,  306),    p( 299,  312),    p( 308,  303),    p( 311,  306),    p( 339,  310),    p( 323,  304),    p( 319,  311),
        p( 285,  310),    p( 302,  310),    p( 307,  307),    p( 323,  313),    p( 316,  309),    p( 314,  310),    p( 304,  309),    p( 287,  310),
        p( 295,  308),    p( 286,  312),    p( 302,  310),    p( 319,  310),    p( 317,  310),    p( 301,  308),    p( 295,  309),    p( 313,  299),
        p( 298,  307),    p( 306,  310),    p( 303,  311),    p( 306,  311),    p( 310,  312),    p( 306,  308),    p( 309,  302),    p( 317,  299),
        p( 309,  310),    p( 305,  301),    p( 312,  303),    p( 298,  310),    p( 304,  308),    p( 306,  306),    p( 315,  300),    p( 306,  296),
        p( 298,  303),    p( 316,  309),    p( 304,  307),    p( 287,  313),    p( 298,  310),    p( 288,  316),    p( 295,  300),    p( 304,  294),
    ],
    // rook
    [
        p( 459,  554),    p( 452,  562),    p( 450,  568),    p( 447,  565),    p( 456,  562),    p( 476,  557),    p( 484,  556),    p( 492,  549),
        p( 444,  556),    p( 443,  561),    p( 452,  562),    p( 468,  551),    p( 455,  555),    p( 482,  549),    p( 487,  548),    p( 500,  538),
        p( 448,  553),    p( 465,  549),    p( 465,  550),    p( 466,  546),    p( 496,  535),    p( 506,  532),    p( 528,  528),    p( 497,  531),
        p( 445,  553),    p( 452,  550),    p( 453,  553),    p( 460,  547),    p( 467,  540),    p( 477,  534),    p( 483,  536),    p( 478,  532),
        p( 441,  549),    p( 443,  547),    p( 444,  549),    p( 451,  546),    p( 456,  543),    p( 448,  542),    p( 470,  534),    p( 456,  533),
        p( 438,  545),    p( 440,  543),    p( 444,  541),    p( 446,  542),    p( 454,  536),    p( 460,  530),    p( 484,  516),    p( 462,  521),
        p( 441,  539),    p( 446,  539),    p( 454,  540),    p( 457,  538),    p( 464,  532),    p( 476,  524),    p( 489,  516),    p( 451,  526),
        p( 450,  542),    p( 449,  538),    p( 450,  543),    p( 456,  538),    p( 464,  532),    p( 468,  531),    p( 468,  528),    p( 459,  528),
    ],
    // queen
    [
        p( 872,  972),    p( 873,  986),    p( 891,  996),    p( 903,  994),    p( 905,  995),    p( 926,  983),    p( 966,  938),    p( 916,  969),
        p( 895,  964),    p( 877,  992),    p( 878, 1014),    p( 873, 1028),    p( 877, 1041),    p( 913, 1008),    p( 925,  984),    p( 953,  971),
        p( 898,  972),    p( 894,  988),    p( 895, 1008),    p( 894, 1015),    p( 907, 1022),    p( 957, 1002),    p( 959,  975),    p( 945,  981),
        p( 886,  982),    p( 891,  992),    p( 887, 1002),    p( 887, 1015),    p( 893, 1022),    p( 902, 1016),    p( 911, 1014),    p( 916,  993),
        p( 892,  972),    p( 884,  991),    p( 889,  992),    p( 890, 1010),    p( 893, 1006),    p( 892, 1007),    p( 906,  991),    p( 912,  985),
        p( 890,  957),    p( 895,  972),    p( 890,  987),    p( 886,  992),    p( 893,  996),    p( 897,  988),    p( 913,  969),    p( 912,  958),
        p( 894,  951),    p( 891,  959),    p( 895,  966),    p( 895,  978),    p( 896,  978),    p( 900,  958),    p( 911,  936),    p( 921,  910),
        p( 883,  946),    p( 890,  940),    p( 887,  956),    p( 890,  956),    p( 896,  948),    p( 879,  953),    p( 884,  942),    p( 896,  926),
    ],
    // king
    [
        p( 150, -102),    p(  65,  -49),    p(  83,  -40),    p(  15,  -10),    p(  28,  -20),    p(   5,  -10),    p(  54,  -19),    p( 217, -105),
        p( -23,   -3),    p( -77,   29),    p( -83,   37),    p( -14,   27),    p( -49,   36),    p( -73,   49),    p( -50,   36),    p(   3,    0),
        p( -44,    6),    p( -42,   24),    p( -82,   41),    p( -89,   49),    p( -56,   43),    p( -21,   36),    p( -65,   35),    p( -31,   11),
        p( -30,   -1),    p( -97,   23),    p(-110,   40),    p(-131,   49),    p(-129,   47),    p(-111,   40),    p(-113,   28),    p(-104,   16),
        p( -49,   -3),    p(-115,   18),    p(-126,   35),    p(-147,   48),    p(-153,   46),    p(-132,   32),    p(-144,   23),    p(-124,   13),
        p( -41,    0),    p( -96,   14),    p(-124,   28),    p(-131,   37),    p(-127,   36),    p(-140,   28),    p(-114,   14),    p( -82,   10),
        p(  20,   -7),    p( -83,   10),    p( -92,   17),    p(-110,   26),    p(-115,   27),    p(-103,   18),    p( -76,    3),    p(  -8,   -2),
        p(  51,  -43),    p(  46,  -48),    p(  36,  -35),    p( -21,  -14),    p(  28,  -31),    p( -16,  -19),    p(  37,  -44),    p(  66,  -53),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(16, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(-0, 1);
const KING_OPEN_FILE: PhasedScore = p(-55, -1);
const KING_CLOSED_FILE: PhasedScore = p(17, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 6);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-4, 5), p(-0, 7), p(-2, 10), p(6, 7), p(7, 11), p(6, 13), p(13, 13), p(25, 9)],
    // Closed
    [p(0, 0), p(0, 0), p(8, -37), p(-16, 11), p(3, 14), p(4, 5), p(5, 12), p(3, 7)],
    // SemiOpen
    [p(0, 0), p(-8, 23), p(7, 22), p(4, 15), p(1, 20), p(4, 15), p(5, 12), p(15, 11)],
    // SemiClosed
    [p(0, 0), p(15, -13), p(8, 7), p(8, 2), p(12, 6), p(5, 6), p(11, 9), p(6, 5)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-6, 6),    /*0b0000*/
    p(-16, 12),  /*0b0001*/
    p(-4, 8),    /*0b0010*/
    p(-10, 15),  /*0b0011*/
    p(-4, 7),    /*0b0100*/
    p(-28, 4),   /*0b0101*/
    p(-15, 7),   /*0b0110*/
    p(-20, -15), /*0b0111*/
    p(6, 10),    /*0b1000*/
    p(-7, 11),   /*0b1001*/
    p(1, 9),     /*0b1010*/
    p(-5, 13),   /*0b1011*/
    p(-2, 7),    /*0b1100*/
    p(-23, 10),  /*0b1101*/
    p(-12, 5),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(1, 18),    /*0b10000*/
    p(4, 12),    /*0b10001*/
    p(20, 14),   /*0b10010*/
    p(-3, 10),   /*0b10011*/
    p(-5, 8),    /*0b10100*/
    p(13, 17),   /*0b10101*/
    p(-22, 4),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(12, 33),   /*0b11000*/
    p(30, 26),   /*0b11001*/
    p(42, 39),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(16, 14),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(14, 10),   /*0b100000*/
    p(3, 15),    /*0b100001*/
    p(24, 4),    /*0b100010*/
    p(6, 2),     /*0b100011*/
    p(-11, 4),   /*0b100100*/
    p(-24, -7),  /*0b100101*/
    p(-27, 19),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(18, 3),    /*0b101000*/
    p(-2, 18),   /*0b101001*/
    p(19, -2),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-7, 6),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(14, 21),   /*0b110000*/
    p(26, 17),   /*0b110001*/
    p(33, 12),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(7, 33),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(25, 17),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(3, 1),     /*0b111111*/
    p(-23, -8),  /*0b00*/
    p(6, -24),   /*0b01*/
    p(33, -12),  /*0b10*/
    p(24, -50),  /*0b11*/
    p(41, -17),  /*0b100*/
    p(-6, -29),  /*0b101*/
    p(71, -48),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(55, -18),  /*0b1000*/
    p(16, -41),  /*0b1001*/
    p(75, -62),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(51, -22),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(30, -13),  /*0b1111*/
    p(16, -10),  /*0b00*/
    p(31, -19),  /*0b01*/
    p(25, -26),  /*0b10*/
    p(21, -51),  /*0b11*/
    p(31, -17),  /*0b100*/
    p(53, -27),  /*0b101*/
    p(24, -32),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(36, -12),  /*0b1000*/
    p(52, -25),  /*0b1001*/
    p(49, -53),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(41, -30),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(19, -53),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  42,   87),    p(  39,   86),    p(  28,   89),    p(  41,   70),    p(  27,   75),    p(  25,   79),    p(  -6,   95),    p(  -2,   94),
        p(  43,  122),    p(  49,  123),    p(  38,   98),    p(  22,   68),    p(  34,   66),    p(  13,   95),    p(  -2,  102),    p( -28,  124),
        p(  24,   72),    p(  19,   69),    p(  24,   53),    p(  16,   43),    p(  -2,   44),    p(   7,   57),    p( -13,   74),    p( -11,   77),
        p(   6,   45),    p(  -2,   42),    p( -14,   33),    p(  -8,   24),    p( -17,   29),    p( -12,   38),    p( -21,   53),    p( -12,   49),
        p(   1,   13),    p( -13,   21),    p( -14,   15),    p( -14,    8),    p( -14,   13),    p(  -9,   17),    p( -16,   35),    p(   8,   16),
        p(  -5,   15),    p(  -3,   19),    p(  -9,   16),    p(  -8,    5),    p(   5,    1),    p(   7,    7),    p(  10,   18),    p(   6,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-10, -10);
const DOUBLED_PAWN: PhasedScore = p(-7, -22);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 7), p(2, 9), p(8, 13), p(5, 9), p(-7, 17), p(-44, 8)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(50, 17), p(56, 49), p(73, -1), p(62, -8), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-50, -54),
        p(-28, -16),
        p(-13, 5),
        p(-2, 16),
        p(8, 23),
        p(18, 31),
        p(28, 31),
        p(37, 29),
        p(44, 24),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-23, -44),
        p(-11, -26),
        p(-1, -11),
        p(6, -1),
        p(12, 8),
        p(16, 15),
        p(18, 19),
        p(20, 22),
        p(20, 26),
        p(26, 25),
        p(29, 23),
        p(36, 23),
        p(29, 29),
        p(40, 20),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-58, 10),
        p(-49, 26),
        p(-45, 33),
        p(-42, 38),
        p(-44, 44),
        p(-40, 49),
        p(-38, 54),
        p(-35, 57),
        p(-33, 61),
        p(-30, 65),
        p(-26, 66),
        p(-24, 70),
        p(-16, 70),
        p(-7, 68),
        p(-8, 70),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-22, -33),
        p(-23, 16),
        p(-27, 70),
        p(-23, 87),
        p(-20, 104),
        p(-16, 110),
        p(-11, 120),
        p(-8, 127),
        p(-4, 133),
        p(-1, 135),
        p(2, 140),
        p(6, 144),
        p(8, 144),
        p(10, 149),
        p(13, 151),
        p(17, 153),
        p(18, 158),
        p(21, 157),
        p(30, 154),
        p(43, 146),
        p(48, 146),
        p(91, 122),
        p(89, 123),
        p(114, 103),
        p(208, 66),
        p(256, 26),
        p(271, 17),
        p(345, -27),
    ],
    [
        p(-83, 48),
        p(-51, 20),
        p(-25, 10),
        p(2, 4),
        p(28, -2),
        p(48, -10),
        p(72, -10),
        p(95, -17),
        p(142, -42),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
const THREATS_INACTIVE: [[PhasedScore; NUM_CHESS_PIECES - 1]; NUM_CHESS_PIECES - 1] = [
    [p(-17, 7), p(-10, -11), p(28, 16), p(63, -5), p(45, -24)],
    [p(-9, 8), p(12, 16), p(-4, 6), p(36, 10), p(52, 93)],
    [p(-4, 14), p(15, 16), p(13, 18), p(-5, 9), p(64, 26)],
    [p(-4, -12), p(2, 9), p(-6, 25), p(1, 1), p(2, -22)],
    [p(57, 29), p(-39, 17), p(-18, 17), p(-46, 15), p(0, 0)],
];
const THREATS_ACTIVE: [[PhasedScore; NUM_CHESS_PIECES - 1]; NUM_CHESS_PIECES - 1] = [
    [p(-3, 24), p(-6, 7), p(33, 53), p(61, 100), p(86, 150)],
    [p(8, 24), p(32, 48), p(-0, 13), p(61, 158), p(118, 208)],
    [p(9, 30), p(29, 43), p(36, 44), p(-5, 24), p(92, 581)],
    [p(5, 24), p(18, 35), p(9, 61), p(-1, 41), p(7, 10)],
    [p(107, 44), p(-5, 60), p(43, 51), p(-26, 56), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(10, 6), p(8, 11), p(14, 5), p(8, 17), p(0, 0)],
    [
        p(-3, -0),
        p(8, 18),
        p(-58, -35),
        p(6, 11),
        p(7, 16),
        p(0, 0),
    ],
    [p(-3, 4), p(8, 9), p(3, 14), p(4, 11), p(6, 20), p(0, 0)],
    [p(3, -2), p(9, 3), p(9, -6), p(3, 19), p(-19, -251), p(0, 0)],
    [
        p(61, -8),
        p(33, 1),
        p(44, -3),
        p(23, -2),
        p(36, -20),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-12, -11),
    p(16, -9),
    p(16, -3),
    p(24, -15),
    p(6, 14),
    p(2, 19),
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
        is_active: bool,
    ) -> SingleFeatureScore<Self::Score>;

    fn defended(
        protecting: ChessPieceType,
        target: ChessPieceType,
    ) -> SingleFeatureScore<Self::Score>;

    fn king_zone_attack(attacking: ChessPieceType) -> SingleFeatureScore<Self::Score>;
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

    fn threats(
        attacking: ChessPieceType,
        targeted: ChessPieceType,
        is_active: bool,
    ) -> PhasedScore {
        if is_active {
            THREATS_INACTIVE[attacking as usize - 1][targeted as usize]
        } else {
            THREATS_ACTIVE[attacking as usize - 1][targeted as usize]
        }
    }
    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> PhasedScore {
        DEFENDED[protecting as usize - 1][target as usize]
    }

    fn king_zone_attack(attacking: ChessPieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }
}
