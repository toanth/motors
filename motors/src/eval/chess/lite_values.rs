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
use gears::games::chess::ChessColor;
use gears::games::chess::ChessColor::White;
use gears::games::chess::pieces::{ChessPieceType, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::general::common::StaticallyNamedEntity;
use gears::score::{PhasedScore, p};
use std::fmt::{Debug, Display};

#[rustfmt::skip]
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
        p( 129,  181),    p( 120,  182),    p( 115,  183),    p( 125,  165),    p( 112,  169),    p( 112,  172),    p(  76,  189),    p(  88,  186),
        p(  61,  116),    p(  62,  117),    p(  72,  115),    p(  79,  118),    p(  69,  123),    p( 111,  107),    p(  88,  125),    p(  85,  115),
        p(  50,  105),    p(  61,  102),    p(  60,   97),    p(  62,   93),    p(  76,   93),    p(  82,   88),    p(  74,   97),    p(  70,   89),
        p(  46,   94),    p(  53,   96),    p(  62,   88),    p(  72,   87),    p(  74,   87),    p(  74,   82),    p(  68,   86),    p(  57,   80),
        p(  42,   91),    p(  50,   87),    p(  54,   88),    p(  57,   94),    p(  65,   90),    p(  60,   86),    p(  68,   77),    p(  51,   80),
        p(  48,   93),    p(  50,   89),    p(  56,   92),    p(  55,  100),    p(  53,  101),    p(  69,   92),    p(  71,   78),    p(  55,   82),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 184,  276),    p( 201,  306),    p( 213,  313),    p( 243,  307),    p( 274,  306),    p( 195,  305),    p( 226,  301),    p( 209,  261),
        p( 261,  302),    p( 272,  310),    p( 289,  300),    p( 292,  304),    p( 291,  301),    p( 304,  291),    p( 267,  306),    p( 267,  298),
        p( 277,  300),    p( 292,  295),    p( 294,  300),    p( 308,  304),    p( 323,  298),    p( 330,  291),    p( 281,  296),    p( 279,  301),
        p( 291,  306),    p( 297,  301),    p( 310,  304),    p( 312,  311),    p( 311,  307),    p( 307,  307),    p( 299,  303),    p( 307,  303),
        p( 290,  308),    p( 291,  300),    p( 300,  303),    p( 307,  306),    p( 305,  308),    p( 311,  295),    p( 307,  296),    p( 302,  305),
        p( 267,  297),    p( 272,  295),    p( 284,  288),    p( 289,  301),    p( 293,  299),    p( 283,  283),    p( 289,  287),    p( 282,  300),
        p( 261,  305),    p( 272,  305),    p( 274,  295),    p( 284,  298),    p( 288,  293),    p( 278,  292),    p( 284,  299),    p( 279,  310),
        p( 233,  303),    p( 275,  294),    p( 257,  299),    p( 277,  302),    p( 285,  300),    p( 282,  291),    p( 281,  297),    p( 256,  303),
    ],
    // bishop
    [
        p( 275,  305),    p( 258,  309),    p( 236,  303),    p( 222,  313),    p( 218,  310),    p( 220,  307),    p( 280,  301),    p( 262,  305),
        p( 283,  301),    p( 271,  299),    p( 284,  303),    p( 272,  301),    p( 284,  300),    p( 287,  297),    p( 263,  304),    p( 274,  299),
        p( 288,  304),    p( 303,  302),    p( 288,  300),    p( 302,  296),    p( 303,  297),    p( 328,  301),    p( 310,  298),    p( 311,  309),
        p( 284,  308),    p( 289,  303),    p( 300,  299),    p( 303,  302),    p( 303,  300),    p( 295,  301),    p( 295,  305),    p( 278,  306),
        p( 287,  304),    p( 283,  306),    p( 293,  301),    p( 306,  301),    p( 299,  298),    p( 297,  300),    p( 284,  302),    p( 305,  299),
        p( 292,  307),    p( 296,  303),    p( 298,  305),    p( 298,  301),    p( 303,  304),    p( 297,  296),    p( 302,  294),    p( 301,  297),
        p( 309,  307),    p( 299,  297),    p( 304,  299),    p( 295,  307),    p( 298,  303),    p( 301,  301),    p( 308,  292),    p( 310,  294),
        p( 293,  301),    p( 312,  301),    p( 302,  303),    p( 287,  306),    p( 301,  306),    p( 290,  305),    p( 307,  294),    p( 296,  293),
    ],
    // rook
    [
        p( 441,  532),    p( 433,  541),    p( 427,  546),    p( 427,  542),    p( 437,  539),    p( 456,  535),    p( 468,  533),    p( 478,  526),
        p( 427,  538),    p( 425,  543),    p( 434,  543),    p( 448,  534),    p( 437,  536),    p( 452,  531),    p( 460,  529),    p( 476,  519),
        p( 428,  534),    p( 443,  530),    p( 438,  531),    p( 438,  526),    p( 459,  517),    p( 468,  514),    p( 486,  512),    p( 466,  515),
        p( 424,  534),    p( 428,  530),    p( 429,  532),    p( 433,  526),    p( 438,  519),    p( 448,  515),    p( 449,  518),    p( 451,  513),
        p( 419,  531),    p( 418,  529),    p( 418,  530),    p( 422,  526),    p( 429,  521),    p( 427,  520),    p( 437,  515),    p( 434,  514),
        p( 416,  528),    p( 415,  526),    p( 416,  525),    p( 420,  523),    p( 425,  517),    p( 437,  510),    p( 451,  500),    p( 440,  504),
        p( 417,  524),    p( 420,  523),    p( 426,  524),    p( 429,  521),    p( 436,  514),    p( 448,  506),    p( 455,  500),    p( 428,  510),
        p( 429,  526),    p( 424,  522),    p( 426,  525),    p( 430,  521),    p( 435,  514),    p( 442,  514),    p( 439,  512),    p( 438,  512),
    ],
    // queen
    [
        p( 844,  956),    p( 845,  969),    p( 856,  979),    p( 878,  970),    p( 879,  974),    p( 901,  961),    p( 932,  929),    p( 896,  947),
        p( 857,  937),    p( 833,  966),    p( 838,  987),    p( 829, 1005),    p( 834, 1018),    p( 877,  977),    p( 878,  961),    p( 915,  941),
        p( 862,  940),    p( 856,  957),    p( 856,  974),    p( 855,  984),    p( 879,  984),    p( 913,  972),    p( 919,  944),    p( 907,  953),
        p( 850,  954),    p( 854,  962),    p( 849,  969),    p( 849,  981),    p( 853,  993),    p( 866,  984),    p( 873,  985),    p( 881,  962),
        p( 859,  947),    p( 848,  962),    p( 855,  963),    p( 854,  978),    p( 857,  975),    p( 859,  975),    p( 871,  965),    p( 877,  960),
        p( 854,  940),    p( 863,  949),    p( 858,  962),    p( 856,  964),    p( 861,  969),    p( 867,  959),    p( 879,  943),    p( 876,  938),
        p( 855,  939),    p( 857,  944),    p( 864,  944),    p( 863,  959),    p( 865,  957),    p( 865,  942),    p( 877,  921),    p( 882,  902),
        p( 845,  937),    p( 855,  928),    p( 855,  939),    p( 866,  936),    p( 866,  930),    p( 854,  932),    p( 854,  927),    p( 858,  918),
    ],
    // king
    [
        p( 137,  -68),    p(  47,  -24),    p(  64,  -19),    p(  -8,    9),    p(  25,   -2),    p(  27,    4),    p(  70,   -2),    p( 202,  -66),
        p( -14,    2),    p( -77,   22),    p( -74,   30),    p( -35,   23),    p( -50,   27),    p( -79,   41),    p( -38,   26),    p(  29,   -0),
        p( -27,    8),    p( -38,   14),    p( -76,   28),    p( -80,   35),    p( -55,   30),    p( -29,   23),    p( -59,   25),    p( -28,   10),
        p( -18,   -0),    p( -89,   13),    p( -96,   26),    p(-116,   33),    p(-119,   32),    p(-102,   26),    p(-118,   17),    p( -90,   14),
        p( -40,   -1),    p(-107,   10),    p(-116,   24),    p(-139,   36),    p(-140,   34),    p(-120,   22),    p(-134,   14),    p(-101,   11),
        p( -27,    3),    p( -86,    7),    p(-111,   20),    p(-117,   26),    p(-115,   26),    p(-124,   19),    p(-102,    7),    p( -62,    9),
        p(  22,   -5),    p( -72,    1),    p( -83,   10),    p(-100,   17),    p(-106,   18),    p( -92,    9),    p( -67,   -6),    p(   6,   -3),
        p(  43,  -19),    p(  35,  -30),    p(  34,  -20),    p( -22,   -2),    p(  26,  -18),    p( -21,   -5),    p(  30,  -27),    p(  61,  -32),
    ],
];

const BISHOP_PAIR: PhasedScore = p(22, 49);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(1, 15), p(2, 13), p(3, 2), p(0, -6), p(-4, -14), p(-7, -22), p(-13, -31), p(-21, -44), p(-31, -55)];
const ROOK_OPEN_FILE: PhasedScore = p(15, 3);
const ROOK_CLOSED_FILE: PhasedScore = p(-11, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 4);
const KING_OPEN_FILE: PhasedScore = p(-46, 1);
const KING_CLOSED_FILE: PhasedScore = p(12, -13);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 9);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-9, 1), p(-10, 2), p(-2, 1), p(-3, -1), p(-2, 1), p(-2, 3), p(3, -0), p(14, -2)],
    // Closed
    [p(0, 0), p(0, 0), p(10, -35), p(-16, 4), p(-4, 5), p(-3, 1), p(-4, 3), p(-5, 2)],
    // SemiOpen
    [p(0, 0), p(-32, 21), p(-2, 14), p(-4, 6), p(-3, 4), p(-1, 1), p(-4, -2), p(5, 1)],
    // SemiClosed
    [p(0, 0), p(1, -15), p(5, 3), p(-1, -3), p(2, -2), p(-2, 0), p(1, 1), p(-4, 2)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-5, 5),    /*0b0000*/
    p(-13, 6),   /*0b0001*/
    p(-3, 8),    /*0b0010*/
    p(-10, 12),  /*0b0011*/
    p(-3, 2),    /*0b0100*/
    p(-24, -1),  /*0b0101*/
    p(-13, 6),   /*0b0110*/
    p(-19, -13), /*0b0111*/
    p(10, 9),    /*0b1000*/
    p(-1, 9),    /*0b1001*/
    p(4, 10),    /*0b1010*/
    p(-4, 9),    /*0b1011*/
    p(1, 2),     /*0b1100*/
    p(-23, 9),   /*0b1101*/
    p(-11, 5),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(4, 14),    /*0b10000*/
    p(3, 8),     /*0b10001*/
    p(17, 11),   /*0b10010*/
    p(-7, 8),    /*0b10011*/
    p(-4, 6),    /*0b10100*/
    p(12, 14),   /*0b10101*/
    p(-23, 3),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(16, 25),   /*0b11000*/
    p(28, 23),   /*0b11001*/
    p(37, 35),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(17, 9),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(16, 9),    /*0b100000*/
    p(5, 10),    /*0b100001*/
    p(24, 3),    /*0b100010*/
    p(6, -1),    /*0b100011*/
    p(-6, 3),    /*0b100100*/
    p(-18, -8),  /*0b100101*/
    p(-17, 10),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(23, 4),    /*0b101000*/
    p(-0, 13),   /*0b101001*/
    p(21, -0),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-3, 6),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(17, 16),   /*0b110000*/
    p(23, 12),   /*0b110001*/
    p(29, 12),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(10, 27),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(25, 13),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(10, -0),   /*0b111111*/
    p(-16, -2),  /*0b00*/
    p(7, -13),   /*0b01*/
    p(33, -9),   /*0b10*/
    p(20, -41),  /*0b11*/
    p(42, -8),   /*0b100*/
    p(-4, -13),  /*0b101*/
    p(63, -34),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(48, -7),   /*0b1000*/
    p(19, -30),  /*0b1001*/
    p(53, -25),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(58, -15),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(17, -23),  /*0b1111*/
    p(13, -2),   /*0b00*/
    p(26, -13),  /*0b01*/
    p(20, -15),  /*0b10*/
    p(15, -39),  /*0b11*/
    p(32, -9),   /*0b100*/
    p(42, -18),  /*0b101*/
    p(19, -21),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(33, -3),   /*0b1000*/
    p(45, -16),  /*0b1001*/
    p(43, -33),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(38, -19),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(16, -42),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  29,   81),    p(  20,   82),    p(  15,   83),    p(  25,   65),    p(  12,   69),    p(  12,   72),    p( -24,   89),    p( -12,   86),
        p(  37,  120),    p(  40,  119),    p(  31,   97),    p(  18,   67),    p(  29,   62),    p(  14,   89),    p(  -4,   99),    p( -29,  122),
        p(  18,   72),    p(  15,   68),    p(  20,   52),    p(  16,   40),    p(   0,   43),    p(   5,   56),    p( -10,   72),    p( -14,   78),
        p(   8,   44),    p(  -2,   41),    p( -13,   31),    p(  -9,   22),    p( -14,   25),    p(  -8,   36),    p( -16,   49),    p( -10,   50),
        p(   1,   13),    p( -12,   21),    p( -14,   14),    p( -15,    6),    p( -12,   12),    p(  -4,   14),    p( -10,   32),    p(  11,   16),
        p(  -5,   12),    p(  -1,   17),    p(  -8,   14),    p(  -9,    4),    p(   4,    0),    p(   9,    6),    p(  17,   16),    p(   9,   11),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-10, -9);
const DOUBLED_PAWN: PhasedScore = p(-7, -19);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(12, 10), p(8, 12), p(12, 18), p(9, 7), p(-3, 16), p(-39, 5)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(36, 10), p(38, 33), p(52, -8), p(35, -36), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-58, -75),
        p(-39, -39),
        p(-27, -18),
        p(-17, -6),
        p(-10, 5),
        p(-3, 15),
        p(5, 18),
        p(11, 21),
        p(16, 21),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-33, -55),
        p(-23, -39),
        p(-13, -25),
        p(-6, -14),
        p(1, -4),
        p(5, 3),
        p(9, 8),
        p(13, 12),
        p(15, 17),
        p(20, 19),
        p(24, 19),
        p(32, 22),
        p(30, 27),
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
    ],
    [
        p(-91, -2),
        p(-82, 10),
        p(-78, 15),
        p(-75, 19),
        p(-75, 24),
        p(-70, 29),
        p(-67, 32),
        p(-63, 35),
        p(-60, 38),
        p(-57, 42),
        p(-54, 44),
        p(-53, 48),
        p(-47, 49),
        p(-39, 47),
        p(-39, 47),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-77, -47),
        p(-74, -11),
        p(-76, 32),
        p(-72, 46),
        p(-69, 61),
        p(-65, 67),
        p(-61, 76),
        p(-58, 81),
        p(-55, 86),
        p(-52, 88),
        p(-50, 92),
        p(-47, 94),
        p(-44, 96),
        p(-43, 100),
        p(-41, 102),
        p(-39, 106),
        p(-38, 112),
        p(-36, 114),
        p(-31, 115),
        p(-20, 112),
        p(-16, 114),
        p(11, 103),
        p(19, 100),
        p(34, 90),
        p(99, 66),
        p(138, 39),
        p(124, 48),
        p(178, 15),
    ],
    [
        p(-82, 12),
        p(-50, -3),
        p(-24, -5),
        p(3, -3),
        p(31, -2),
        p(52, -4),
        p(76, 2),
        p(100, 0),
        p(144, -16),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-8, 6), p(0, 0), p(23, 17), p(46, -11), p(20, -37), p(0, 0)],
    [p(-2, 10), p(18, 21), p(0, 0), p(29, 4), p(31, 41), p(0, 0)],
    [p(-2, 12), p(9, 13), p(16, 11), p(0, 0), p(43, -11), p(0, 0)],
    [p(-1, 3), p(2, 3), p(1, 16), p(2, -2), p(0, 0), p(0, 0)],
    [p(63, 28), p(-27, 16), p(-7, 15), p(-18, 8), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 6), p(8, 7), p(6, 10), p(12, 7), p(7, 16), p(10, 6)],
    [p(1, 6), p(10, 20), p(-141, -28), p(8, 14), p(9, 18), p(4, 7)],
    [p(2, 2), p(12, 5), p(9, 10), p(10, 8), p(10, 18), p(20, -4)],
    [p(2, -2), p(8, 1), p(7, -4), p(4, 14), p(-42, -251), p(4, -8)],
    [p(59, -2), p(37, 6), p(41, 0), p(22, 3), p(33, -10), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-17, -17), p(16, -8), p(9, -4), p(12, -11), p(-1, 12), p(-4, 7)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(26, 12), p(13, 18), p(30, 2), p(5, 30)];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static + StaticallyNamedEntity {
    type Score: ScoreType;

    fn psqt(&self, square: ChessSquare, piece: ChessPieceType, color: ChessColor) -> SingleFeatureScore<Self::Score>;

    fn passed_pawn(square: ChessSquare) -> SingleFeatureScore<Self::Score>;

    fn unsupported_pawn() -> SingleFeatureScore<Self::Score>;

    fn doubled_pawn() -> SingleFeatureScore<Self::Score>;

    fn bishop_pair() -> SingleFeatureScore<Self::Score>;

    fn bad_bishop(num_pawns: usize) -> SingleFeatureScore<Self::Score>;

    fn rook_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score>;

    fn king_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score>;

    fn bishop_openness(openness: FileOpenness, len: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_shield(&self, color: ChessColor, config: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_protection(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn pawn_attack(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn mobility(piece: ChessPieceType, mobility: usize) -> SingleFeatureScore<Self::Score>;

    fn threats(attacking: ChessPieceType, targeted: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn king_zone_attack(attacking: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn can_give_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;
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

    fn bad_bishop(num_pawns: usize) -> SingleFeatureScore<Self::Score> {
        BAD_BISHOP[num_pawns]
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

    fn bishop_openness(openness: FileOpenness, len: usize) -> <PhasedScore as ScoreType>::SingleFeatureScore {
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

    fn can_give_check(piece: ChessPieceType) -> PhasedScore {
        CAN_GIVE_CHECK[piece as usize]
    }
}
