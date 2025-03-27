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

use crate::eval::chess::{FileOpenness, NUM_PAWN_CENTER_CONFIGURATIONS, NUM_PAWN_SHIELD_CONFIGURATIONS};
use crate::eval::{ScoreType, SingleFeatureScore};
use gears::games::DimT;
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
        p( 132,  185),    p( 131,  182),    p( 121,  186),    p( 132,  167),    p( 119,  172),    p( 119,  175),    p(  81,  192),    p(  86,  192),
        p(  69,  122),    p(  70,  124),    p(  78,  119),    p(  86,  120),    p(  74,  120),    p( 123,  110),    p(  99,  131),    p(  94,  120),
        p(  53,  111),    p(  63,  105),    p(  62,  101),    p(  84,  100),    p(  93,   99),    p(  84,   90),    p(  76,  100),    p(  72,   94),
        p(  49,   98),    p(  54,  100),    p(  78,   93),    p(  92,   96),    p(  91,   98),    p(  87,   94),    p(  70,   90),    p(  60,   84),
        p(  42,   96),    p(  50,   91),    p(  72,   96),    p(  81,   97),    p(  83,   95),    p(  77,   94),    p(  69,   81),    p(  52,   83),
        p(  53,   99),    p(  58,   97),    p(  62,   97),    p(  59,  104),    p(  61,  106),    p(  76,   97),    p(  81,   85),    p(  59,   87),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 173,  278),    p( 195,  311),    p( 214,  323),    p( 252,  312),    p( 282,  313),    p( 198,  309),    p( 210,  310),    p( 201,  261),
        p( 267,  312),    p( 284,  317),    p( 299,  309),    p( 304,  313),    p( 303,  308),    p( 315,  298),    p( 274,  314),    p( 270,  305),
        p( 287,  307),    p( 307,  304),    p( 309,  311),    p( 323,  314),    p( 339,  308),    p( 352,  297),    p( 293,  304),    p( 286,  308),
        p( 302,  315),    p( 310,  309),    p( 326,  313),    p( 328,  321),    p( 326,  318),    p( 321,  317),    p( 311,  313),    p( 319,  311),
        p( 299,  317),    p( 305,  307),    p( 313,  313),    p( 321,  316),    p( 320,  319),    p( 325,  303),    p( 323,  304),    p( 312,  313),
        p( 275,  304),    p( 283,  303),    p( 297,  297),    p( 302,  311),    p( 306,  308),    p( 296,  291),    p( 302,  293),    p( 293,  307),
        p( 270,  312),    p( 281,  314),    p( 286,  304),    p( 295,  308),    p( 299,  303),    p( 290,  300),    p( 295,  306),    p( 289,  321),
        p( 240,  310),    p( 282,  304),    p( 267,  306),    p( 287,  311),    p( 296,  308),    p( 292,  297),    p( 288,  306),    p( 265,  308),
    ],
    // bishop
    [
        p( 276,  310),    p( 251,  314),    p( 240,  306),    p( 223,  317),    p( 218,  314),    p( 225,  308),    p( 273,  303),    p( 250,  309),
        p( 282,  303),    p( 279,  303),    p( 290,  306),    p( 277,  304),    p( 289,  301),    p( 292,  299),    p( 267,  309),    p( 271,  302),
        p( 296,  309),    p( 307,  305),    p( 291,  304),    p( 307,  299),    p( 305,  300),    p( 336,  305),    p( 317,  301),    p( 318,  313),
        p( 286,  313),    p( 292,  307),    p( 304,  303),    p( 307,  306),    p( 306,  304),    p( 299,  305),    p( 298,  309),    p( 280,  310),
        p( 289,  308),    p( 284,  310),    p( 295,  304),    p( 308,  305),    p( 302,  301),    p( 299,  303),    p( 285,  305),    p( 309,  302),
        p( 296,  310),    p( 299,  305),    p( 299,  307),    p( 299,  304),    p( 305,  307),    p( 298,  299),    p( 305,  296),    p( 307,  299),
        p( 308,  308),    p( 303,  300),    p( 308,  301),    p( 299,  309),    p( 301,  305),    p( 302,  305),    p( 312,  295),    p( 308,  296),
        p( 298,  305),    p( 309,  306),    p( 308,  307),    p( 290,  310),    p( 306,  308),    p( 294,  309),    p( 303,  297),    p( 302,  292),
    ],
    // rook
    [
        p( 460,  547),    p( 448,  557),    p( 442,  563),    p( 440,  561),    p( 452,  556),    p( 472,  551),    p( 483,  549),    p( 493,  543),
        p( 444,  553),    p( 442,  558),    p( 451,  559),    p( 466,  550),    p( 452,  553),    p( 468,  547),    p( 476,  544),    p( 491,  535),
        p( 446,  548),    p( 465,  543),    p( 459,  545),    p( 458,  540),    p( 484,  530),    p( 494,  527),    p( 512,  526),    p( 486,  528),
        p( 443,  548),    p( 449,  544),    p( 448,  546),    p( 453,  541),    p( 457,  532),    p( 468,  528),    p( 469,  532),    p( 469,  527),
        p( 436,  546),    p( 435,  544),    p( 436,  544),    p( 441,  540),    p( 448,  536),    p( 442,  536),    p( 455,  530),    p( 449,  528),
        p( 431,  543),    p( 431,  539),    p( 433,  539),    p( 436,  538),    p( 441,  533),    p( 452,  524),    p( 468,  513),    p( 455,  517),
        p( 433,  538),    p( 437,  537),    p( 443,  537),    p( 445,  535),    p( 452,  528),    p( 465,  519),    p( 473,  513),    p( 444,  523),
        p( 443,  542),    p( 439,  537),    p( 440,  541),    p( 445,  535),    p( 450,  529),    p( 457,  528),    p( 453,  527),    p( 449,  529),
    ],
    // queen
    [
        p( 879,  960),    p( 880,  975),    p( 895,  988),    p( 916,  982),    p( 914,  985),    p( 933,  974),    p( 980,  924),    p( 925,  956),
        p( 889,  950),    p( 865,  979),    p( 866, 1007),    p( 858, 1024),    p( 865, 1035),    p( 905,  996),    p( 907,  980),    p( 948,  959),
        p( 895,  955),    p( 887,  971),    p( 887,  992),    p( 886, 1003),    p( 909, 1004),    p( 947,  988),    p( 955,  959),    p( 943,  965),
        p( 881,  968),    p( 886,  976),    p( 879,  986),    p( 881,  997),    p( 883, 1010),    p( 897, 1000),    p( 906, 1001),    p( 913,  977),
        p( 891,  959),    p( 878,  979),    p( 884,  979),    p( 884,  996),    p( 888,  992),    p( 889,  993),    p( 902,  981),    p( 909,  974),
        p( 887,  947),    p( 893,  964),    p( 886,  979),    p( 883,  982),    p( 889,  990),    p( 896,  978),    p( 910,  960),    p( 908,  949),
        p( 887,  951),    p( 887,  958),    p( 893,  960),    p( 893,  974),    p( 895,  973),    p( 895,  957),    p( 907,  935),    p( 915,  908),
        p( 873,  951),    p( 885,  939),    p( 886,  951),    p( 894,  953),    p( 896,  942),    p( 883,  947),    p( 885,  939),    p( 890,  922),
    ],
    // king
    [
        p( 156,  -87),    p(  61,  -38),    p(  85,  -29),    p(  11,    1),    p(  40,  -12),    p(  24,   -1),    p(  76,   -9),    p( 226,  -89),
        p( -30,    3),    p( -74,   23),    p( -73,   29),    p( -12,   19),    p( -42,   27),    p( -71,   42),    p( -44,   28),    p(   7,    2),
        p( -43,   10),    p( -41,   17),    p( -76,   31),    p( -86,   40),    p( -55,   34),    p( -24,   27),    p( -68,   29),    p( -36,   12),
        p( -25,    3),    p( -94,   16),    p(-105,   31),    p(-127,   40),    p(-126,   38),    p(-107,   30),    p(-126,   20),    p(-104,   18),
        p( -38,   -2),    p(-107,   11),    p(-118,   27),    p(-142,   40),    p(-143,   38),    p(-120,   24),    p(-135,   15),    p(-116,   13),
        p( -31,    2),    p( -83,    6),    p(-112,   21),    p(-120,   29),    p(-117,   28),    p(-126,   20),    p(-102,    7),    p( -71,   10),
        p(  27,   -7),    p( -71,    1),    p( -84,   10),    p(-104,   19),    p(-109,   20),    p( -94,   11),    p( -66,   -7),    p(   2,   -3),
        p(  50,  -26),    p(  42,  -35),    p(  38,  -21),    p( -24,   -2),    p(  28,  -19),    p( -20,   -5),    p(  34,  -30),    p(  62,  -38),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 53);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(9, 20), p(10, 18), p(10, 7), p(6, -1), p(2, -9), p(-1, -19), p(-7, -27), p(-14, -40), p(-25, -50)];
const ROOK_OPEN_FILE: PhasedScore = p(13, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, 0);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 4);
const KING_OPEN_FILE: PhasedScore = p(-49, -1);
const KING_CLOSED_FILE: PhasedScore = p(14, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-5, 3), p(1, 5), p(-1, 4), p(2, 2), p(2, 4), p(4, 7), p(7, 4), p(19, 1)],
    // Closed
    [p(0, 0), p(0, 0), p(19, -27), p(-14, 10), p(-1, 11), p(-0, 5), p(-2, 7), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-15, 23), p(2, 16), p(0, 9), p(-0, 9), p(4, 5), p(0, 2), p(10, 5)],
    // SemiClosed
    [p(0, 0), p(12, -13), p(7, 6), p(3, 0), p(6, 1), p(1, 5), p(3, 6), p(2, 5)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(24, 5),
    p(3, 5),
    p(3, 3),
    p(-7, 12),
    p(7, 0),
    p(-10, -8),
    p(1, 3),
    p(-4, -5),
    p(1, -3),
    p(-11, 1),
    p(-8, -14),
    p(-17, 5),
    p(5, -5),
    p(-3, -1),
    p(8, 1),
    p(1, 27),
    p(-2, -2),
    p(-21, 1),
    p(-16, -3),
    p(-48, 23),
    p(-17, 4),
    p(-18, -16),
    p(9, 20),
    p(-60, 34),
    p(-16, -13),
    p(-22, -7),
    p(-39, -30),
    p(-42, 11),
    p(-22, 5),
    p(8, 4),
    p(-98, 118),
    p(0, 0),
    p(1, -1),
    p(-15, 1),
    p(-3, -2),
    p(-27, 10),
    p(-28, -7),
    p(-55, -20),
    p(-35, 37),
    p(-49, 28),
    p(-8, 1),
    p(-22, 1),
    p(5, -6),
    p(-22, 47),
    p(-56, 16),
    p(-15, -28),
    p(0, 0),
    p(0, 0),
    p(7, 1),
    p(-8, 23),
    p(-4, -51),
    p(0, 0),
    p(3, -9),
    p(-44, -2),
    p(0, 0),
    p(0, 0),
    p(-25, 11),
    p(-19, 9),
    p(-12, 21),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(21, 1),
    p(-1, -3),
    p(-5, 2),
    p(-21, 0),
    p(5, -3),
    p(-28, -4),
    p(-18, 4),
    p(-38, -4),
    p(4, -3),
    p(-15, -7),
    p(-28, -3),
    p(-43, 6),
    p(-10, -0),
    p(-44, 4),
    p(-38, -1),
    p(-55, 70),
    p(10, -3),
    p(-3, -5),
    p(-7, -10),
    p(-29, -4),
    p(-10, 3),
    p(-19, -6),
    p(-26, -0),
    p(-88, 187),
    p(-6, -10),
    p(-28, -9),
    p(-40, -25),
    p(0, -81),
    p(-17, -2),
    p(-20, -8),
    p(-84, 74),
    p(0, 0),
    p(16, -2),
    p(2, -3),
    p(-10, -5),
    p(-20, -6),
    p(-2, 1),
    p(-28, -12),
    p(-17, 1),
    p(-30, 4),
    p(0, -5),
    p(-22, -4),
    p(-25, -12),
    p(-38, -2),
    p(-12, -1),
    p(-50, -10),
    p(-18, 25),
    p(-65, 56),
    p(9, 2),
    p(-8, 0),
    p(-25, 57),
    p(0, 0),
    p(-16, 1),
    p(-21, 0),
    p(0, 0),
    p(0, 0),
    p(-12, 5),
    p(-38, 17),
    p(-34, -40),
    p(0, 0),
    p(9, -64),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-7, 7),    /*0b0000*/
    p(-14, 9),   /*0b0001*/
    p(-4, 10),   /*0b0010*/
    p(-8, 11),   /*0b0011*/
    p(-3, 4),    /*0b0100*/
    p(-25, 1),   /*0b0101*/
    p(-11, 4),   /*0b0110*/
    p(-17, -14), /*0b0111*/
    p(9, 12),    /*0b1000*/
    p(-2, 12),   /*0b1001*/
    p(2, 11),    /*0b1010*/
    p(-0, 10),   /*0b1011*/
    p(1, 6),     /*0b1100*/
    p(-23, 7),   /*0b1101*/
    p(-9, 5),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(6, 17),    /*0b10000*/
    p(4, 10),    /*0b10001*/
    p(18, 13),   /*0b10010*/
    p(-3, 8),    /*0b10011*/
    p(-4, 6),    /*0b10100*/
    p(13, 13),   /*0b10101*/
    p(-20, 2),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(15, 18),   /*0b11000*/
    p(26, 17),   /*0b11001*/
    p(35, 31),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(13, 3),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(17, 11),   /*0b100000*/
    p(5, 13),    /*0b100001*/
    p(23, 5),    /*0b100010*/
    p(8, 1),     /*0b100011*/
    p(-6, 3),    /*0b100100*/
    p(-22, -7),  /*0b100101*/
    p(-21, 16),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(27, 6),    /*0b101000*/
    p(2, 17),    /*0b101001*/
    p(19, -1),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-2, 9),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(14, 11),   /*0b110000*/
    p(22, 8),    /*0b110001*/
    p(27, 4),    /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(2, 21),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(26, 5),    /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(6, -1),    /*0b111111*/
    p(-17, -3),  /*0b00*/
    p(9, -18),   /*0b01*/
    p(37, -8),   /*0b10*/
    p(25, -45),  /*0b11*/
    p(46, -10),  /*0b100*/
    p(3, -19),   /*0b101*/
    p(68, -41),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(64, -11),  /*0b1000*/
    p(19, -34),  /*0b1001*/
    p(78, -54),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(62, -36),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(25, -16),  /*0b1111*/
    p(20, -3),   /*0b00*/
    p(34, -14),  /*0b01*/
    p(27, -19),  /*0b10*/
    p(24, -43),  /*0b11*/
    p(39, -9),   /*0b100*/
    p(55, -22),  /*0b101*/
    p(25, -24),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(40, -2),   /*0b1000*/
    p(52, -19),  /*0b1001*/
    p(52, -42),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(42, -27),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(22, -45),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  32,   85),    p(  31,   82),    p(  21,   86),    p(  32,   67),    p(  19,   72),    p(  19,   75),    p( -19,   92),    p( -14,   92),
        p(  34,  121),    p(  40,  118),    p(  34,   95),    p(  19,   68),    p(  31,   68),    p(  10,   90),    p(  -8,   97),    p( -36,  122),
        p(  21,   72),    p(  16,   70),    p(  21,   53),    p(  17,   43),    p(  -1,   45),    p(   5,   58),    p( -11,   73),    p( -12,   77),
        p(   6,   46),    p(  -2,   43),    p( -13,   34),    p(  -4,   25),    p( -15,   29),    p(  -7,   39),    p( -19,   55),    p( -14,   51),
        p(   0,   14),    p( -12,   23),    p( -13,   16),    p( -11,    7),    p( -13,   15),    p(  -9,   18),    p( -17,   38),    p(   8,   17),
        p(  -8,   14),    p(  -5,   18),    p( -10,   16),    p(  -6,    4),    p(   6,   -0),    p(   4,    7),    p(   9,   17),    p(   5,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -8);
const DOUBLED_PAWN: PhasedScore = p(-7, -23);
const PHALANX: [PhasedScore; 6] = [p(-100, 234), p(65, 82), p(24, 24), p(9, 5), p(5, 2), p(-0, -1)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 12), p(7, 14), p(14, 18), p(9, 7), p(-3, 15), p(-45, 6)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(39, 7), p(40, 33), p(51, -9), p(35, -35), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-45, -72),
        p(-25, -31),
        p(-13, -9),
        p(-3, 5),
        p(4, 15),
        p(11, 26),
        p(19, 29),
        p(25, 31),
        p(30, 30),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-31, -55),
        p(-19, -38),
        p(-8, -22),
        p(-0, -10),
        p(7, -0),
        p(12, 8),
        p(17, 13),
        p(21, 17),
        p(23, 22),
        p(30, 24),
        p(35, 23),
        p(43, 26),
        p(40, 33),
        p(54, 26),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-67, 26),
        p(-62, 32),
        p(-59, 36),
        p(-60, 43),
        p(-54, 48),
        p(-51, 52),
        p(-46, 55),
        p(-43, 59),
        p(-39, 62),
        p(-35, 65),
        p(-33, 69),
        p(-26, 70),
        p(-17, 68),
        p(-16, 68),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-27, -46),
        p(-27, 8),
        p(-31, 59),
        p(-26, 77),
        p(-24, 96),
        p(-20, 102),
        p(-16, 112),
        p(-13, 118),
        p(-8, 122),
        p(-5, 123),
        p(-2, 125),
        p(2, 127),
        p(5, 128),
        p(6, 132),
        p(9, 134),
        p(13, 137),
        p(13, 144),
        p(16, 144),
        p(25, 142),
        p(39, 135),
        p(42, 138),
        p(86, 114),
        p(85, 117),
        p(107, 100),
        p(199, 68),
        p(248, 25),
        p(266, 22),
        p(331, -22),
    ],
    [
        p(-88, 16),
        p(-54, 0),
        p(-27, -2),
        p(2, -3),
        p(32, -4),
        p(54, -5),
        p(80, -0),
        p(104, -2),
        p(151, -21),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-9, 7), p(0, 0), p(23, 19), p(49, -11), p(20, -33), p(0, 0)],
    [p(-2, 10), p(20, 22), p(0, 0), p(31, 5), p(30, 53), p(0, 0)],
    [p(-3, 13), p(10, 15), p(17, 12), p(0, 0), p(45, -6), p(0, 0)],
    [p(-2, 4), p(2, 6), p(-1, 22), p(1, 2), p(0, 0), p(0, 0)],
    [p(68, 26), p(-36, 18), p(-8, 17), p(-21, 7), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 7), p(8, 7), p(6, 11), p(12, 7), p(6, 19), p(10, 6)],
    [p(2, 6), p(11, 22), p(-134, -15), p(8, 14), p(10, 20), p(4, 7)],
    [p(3, 2), p(14, 6), p(9, 11), p(11, 8), p(11, 21), p(21, -5)],
    [p(2, -2), p(9, 1), p(7, -4), p(4, 15), p(-60, -257), p(5, -11)],
    [p(61, -3), p(39, 5), p(45, -1), p(23, 3), p(35, -13), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-17, -17), p(20, -10), p(10, -4), p(15, -12), p(-1, 12), p(-7, 9)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(14, 19), p(34, -0), p(5, 32)];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static + StaticallyNamedEntity {
    type Score: ScoreType;

    fn psqt(&self, square: ChessSquare, piece: ChessPieceType, color: ChessColor) -> SingleFeatureScore<Self::Score>;

    fn passed_pawn(square: ChessSquare) -> SingleFeatureScore<Self::Score>;

    fn unsupported_pawn() -> SingleFeatureScore<Self::Score>;

    fn doubled_pawn() -> SingleFeatureScore<Self::Score>;

    fn phalanx(rank: DimT) -> SingleFeatureScore<Self::Score>;

    fn bishop_pair() -> SingleFeatureScore<Self::Score>;

    fn bad_bishop(num_pawns: usize) -> SingleFeatureScore<Self::Score>;

    fn rook_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score>;

    fn king_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score>;

    fn bishop_openness(openness: FileOpenness, len: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_advanced_center(config: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_passive_center(config: usize) -> SingleFeatureScore<Self::Score>;

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

    fn phalanx(rank: DimT) -> SingleFeatureScore<Self::Score> {
        PHALANX[rank as usize]
    }

    fn bishop_pair() -> PhasedScore {
        BISHOP_PAIR
    }

    fn bad_bishop(num_pawns: usize) -> PhasedScore {
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

    fn bishop_openness(openness: FileOpenness, len: usize) -> PhasedScore {
        BISHOP_OPENNESS[openness as usize][len - 1]
    }

    fn pawn_advanced_center(config: usize) -> PhasedScore {
        PAWN_ADVANCED_CENTER[config]
    }

    fn pawn_passive_center(config: usize) -> PhasedScore {
        PAWN_PASSIVE_CENTER[config]
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
