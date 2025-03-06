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
        p( 133,  186),    p( 130,  185),    p( 121,  188),    p( 133,  169),    p( 119,  173),    p( 119,  176),    p(  81,  194),    p(  89,  193),
        p(  67,  123),    p(  66,  124),    p(  77,  120),    p(  86,  123),    p(  74,  124),    p( 122,  110),    p(  97,  130),    p(  93,  121),
        p(  55,  112),    p(  66,  108),    p(  65,  104),    p(  67,   98),    p(  83,   98),    p(  87,   94),    p(  79,  103),    p(  75,   95),
        p(  51,   99),    p(  58,  101),    p(  67,   94),    p(  76,   93),    p(  80,   93),    p(  79,   88),    p(  73,   92),    p(  62,   86),
        p(  46,   97),    p(  55,   92),    p(  59,   93),    p(  61,   99),    p(  70,   96),    p(  63,   92),    p(  73,   81),    p(  55,   85),
        p(  52,   99),    p(  55,   95),    p(  61,   97),    p(  60,  104),    p(  58,  107),    p(  73,   96),    p(  75,   82),    p(  58,   87),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 181,  277),    p( 201,  310),    p( 217,  322),    p( 255,  311),    p( 285,  313),    p( 201,  308),    p( 215,  310),    p( 208,  261),
        p( 272,  311),    p( 287,  317),    p( 302,  308),    p( 306,  312),    p( 304,  307),    p( 318,  296),    p( 279,  313),    p( 275,  303),
        p( 289,  307),    p( 307,  303),    p( 309,  310),    p( 323,  312),    p( 340,  306),    p( 353,  295),    p( 294,  303),    p( 288,  307),
        p( 304,  315),    p( 311,  308),    p( 326,  312),    p( 329,  318),    p( 326,  315),    p( 321,  315),    p( 312,  311),    p( 321,  309),
        p( 301,  317),    p( 305,  306),    p( 313,  311),    p( 321,  314),    p( 312,  318),    p( 317,  302),    p( 315,  303),    p( 307,  311),
        p( 278,  303),    p( 282,  301),    p( 295,  295),    p( 294,  309),    p( 297,  307),    p( 286,  290),    p( 295,  292),    p( 287,  307),
        p( 272,  311),    p( 282,  313),    p( 286,  302),    p( 287,  307),    p( 298,  300),    p( 281,  300),    p( 297,  305),    p( 282,  322),
        p( 239,  311),    p( 282,  303),    p( 266,  304),    p( 280,  309),    p( 287,  307),    p( 284,  296),    p( 284,  307),    p( 258,  309),
    ],
    // bishop
    [
        p( 276,  309),    p( 253,  313),    p( 240,  306),    p( 224,  316),    p( 219,  312),    p( 225,  308),    p( 275,  303),    p( 254,  308),
        p( 279,  303),    p( 278,  303),    p( 290,  305),    p( 279,  302),    p( 289,  300),    p( 294,  299),    p( 271,  308),    p( 272,  301),
        p( 295,  309),    p( 305,  304),    p( 291,  303),    p( 307,  298),    p( 307,  299),    p( 337,  303),    p( 319,  300),    p( 318,  313),
        p( 286,  312),    p( 291,  305),    p( 302,  302),    p( 307,  305),    p( 307,  303),    p( 301,  304),    p( 297,  307),    p( 280,  308),
        p( 290,  307),    p( 284,  309),    p( 295,  303),    p( 308,  305),    p( 302,  300),    p( 297,  302),    p( 286,  303),    p( 306,  301),
        p( 297,  311),    p( 301,  305),    p( 301,  306),    p( 299,  304),    p( 304,  307),    p( 297,  298),    p( 303,  296),    p( 305,  299),
        p( 308,  309),    p( 305,  301),    p( 310,  300),    p( 299,  308),    p( 299,  304),    p( 305,  303),    p( 310,  295),    p( 308,  296),
        p( 298,  305),    p( 311,  306),    p( 308,  307),    p( 291,  307),    p( 303,  307),    p( 294,  309),    p( 303,  296),    p( 300,  292),
    ],
    // rook
    [
        p( 459,  546),    p( 449,  555),    p( 443,  562),    p( 441,  559),    p( 453,  555),    p( 473,  550),    p( 484,  548),    p( 493,  541),
        p( 444,  552),    p( 442,  557),    p( 451,  558),    p( 466,  549),    p( 452,  552),    p( 470,  546),    p( 478,  543),    p( 492,  533),
        p( 446,  547),    p( 463,  542),    p( 458,  544),    p( 458,  538),    p( 484,  528),    p( 494,  525),    p( 511,  524),    p( 486,  526),
        p( 442,  547),    p( 447,  543),    p( 447,  545),    p( 453,  539),    p( 458,  530),    p( 469,  526),    p( 469,  530),    p( 469,  525),
        p( 436,  544),    p( 435,  542),    p( 435,  543),    p( 440,  539),    p( 447,  535),    p( 443,  534),    p( 455,  529),    p( 449,  527),
        p( 431,  542),    p( 431,  538),    p( 433,  538),    p( 436,  537),    p( 441,  531),    p( 454,  523),    p( 469,  511),    p( 456,  516),
        p( 434,  537),    p( 438,  535),    p( 443,  537),    p( 446,  534),    p( 454,  526),    p( 466,  517),    p( 473,  512),    p( 444,  521),
        p( 443,  541),    p( 439,  536),    p( 441,  539),    p( 445,  534),    p( 450,  527),    p( 457,  527),    p( 453,  526),    p( 449,  528),
    ],
    // queen
    [
        p( 878,  960),    p( 881,  974),    p( 896,  988),    p( 917,  982),    p( 914,  986),    p( 935,  973),    p( 980,  925),    p( 925,  957),
        p( 887,  951),    p( 863,  980),    p( 865, 1007),    p( 858, 1025),    p( 865, 1037),    p( 906,  995),    p( 906,  980),    p( 947,  959),
        p( 892,  956),    p( 885,  972),    p( 885,  993),    p( 886, 1002),    p( 909, 1004),    p( 946,  987),    p( 954,  957),    p( 942,  964),
        p( 878,  970),    p( 885,  975),    p( 879,  985),    p( 880,  997),    p( 883, 1008),    p( 897,  998),    p( 905,  999),    p( 913,  975),
        p( 889,  960),    p( 877,  981),    p( 884,  978),    p( 884,  995),    p( 887,  990),    p( 889,  990),    p( 902,  978),    p( 909,  971),
        p( 885,  950),    p( 892,  966),    p( 886,  980),    p( 885,  979),    p( 889,  986),    p( 896,  974),    p( 910,  958),    p( 908,  946),
        p( 886,  948),    p( 886,  957),    p( 893,  959),    p( 892,  973),    p( 895,  970),    p( 895,  954),    p( 906,  932),    p( 915,  905),
        p( 872,  949),    p( 884,  937),    p( 885,  949),    p( 894,  949),    p( 896,  939),    p( 884,  942),    p( 885,  935),    p( 888,  920),
    ],
    // king
    [
        p( 155,  -85),    p(  60,  -37),    p(  84,  -29),    p(   7,    3),    p(  35,  -10),    p(  21,   -0),    p(  73,   -9),    p( 227,  -89),
        p( -33,    0),    p( -83,   18),    p( -85,   24),    p( -24,   15),    p( -55,   22),    p( -86,   37),    p( -55,   22),    p(  -1,   -0),
        p( -48,    8),    p( -52,   12),    p( -89,   26),    p( -98,   34),    p( -68,   28),    p( -37,   20),    p( -83,   23),    p( -45,    9),
        p( -30,   -0),    p(-105,   11),    p(-119,   26),    p(-141,   35),    p(-140,   32),    p(-120,   25),    p(-139,   15),    p(-116,   16),
        p( -47,   -3),    p(-120,    7),    p(-132,   23),    p(-157,   36),    p(-160,   33),    p(-133,   20),    p(-149,   11),    p(-127,   11),
        p( -36,    1),    p( -95,    3),    p(-125,   18),    p(-134,   26),    p(-131,   25),    p(-139,   17),    p(-110,    3),    p( -79,    9),
        p(  24,   -8),    p( -78,   -3),    p( -95,    7),    p(-117,   16),    p(-120,   16),    p(-105,    7),    p( -73,  -10),    p(  -2,   -5),
        p(  58,  -23),    p(  46,  -34),    p(  40,  -19),    p( -24,    1),    p(  27,  -15),    p( -16,   -3),    p(  39,  -28),    p(  66,  -33),
    ],
];

const BISHOP_PAIR: PhasedScore = p(22, 53);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(8, 19), p(10, 17), p(10, 6), p(7, -2), p(3, -9), p(-1, -19), p(-8, -28), p(-16, -41), p(-28, -52)];
const ROOK_OPEN_FILE: PhasedScore = p(13, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -0);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(2, 4);
const KING_OPEN_FILE: PhasedScore = p(-49, -2);
const KING_CLOSED_FILE: PhasedScore = p(14, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 3), p(0, 5), p(-2, 3), p(2, 2), p(2, 5), p(2, 7), p(4, 4), p(17, 0)],
    // Closed
    [p(0, 0), p(0, 0), p(15, -25), p(-15, 8), p(-1, 11), p(2, 4), p(-1, 7), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-16, 22), p(3, 16), p(0, 9), p(-0, 9), p(3, 4), p(-1, 2), p(9, 4)],
    // SemiClosed
    [p(0, 0), p(12, -13), p(7, 5), p(3, 0), p(7, 2), p(3, 4), p(4, 5), p(0, 4)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-8, 3),    /*0b0000*/
    p(-16, 7),   /*0b0001*/
    p(-4, 7),    /*0b0010*/
    p(-11, 15),  /*0b0011*/
    p(-5, 2),    /*0b0100*/
    p(-26, 1),   /*0b0101*/
    p(-14, 7),   /*0b0110*/
    p(-18, -11), /*0b0111*/
    p(6, 8),     /*0b1000*/
    p(-4, 10),   /*0b1001*/
    p(2, 10),    /*0b1010*/
    p(-5, 13),   /*0b1011*/
    p(-1, 4),    /*0b1100*/
    p(-22, 12),  /*0b1101*/
    p(-11, 6),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-0, 12),   /*0b10000*/
    p(1, 8),     /*0b10001*/
    p(19, 10),   /*0b10010*/
    p(-6, 9),    /*0b10011*/
    p(-7, 5),    /*0b10100*/
    p(13, 17),   /*0b10101*/
    p(-24, 4),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(13, 26),   /*0b11000*/
    p(28, 23),   /*0b11001*/
    p(42, 38),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(17, 9),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(13, 7),    /*0b100000*/
    p(1, 13),    /*0b100001*/
    p(24, 2),    /*0b100010*/
    p(7, 1),     /*0b100011*/
    p(-8, 2),    /*0b100100*/
    p(-20, -4),  /*0b100101*/
    p(-21, 18),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(20, 2),    /*0b101000*/
    p(-1, 17),   /*0b101001*/
    p(21, -2),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-5, 8),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(12, 15),   /*0b110000*/
    p(24, 12),   /*0b110001*/
    p(31, 10),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(10, 29),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(24, 12),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(9, 2),     /*0b111111*/
    p(-14, -5),  /*0b00*/
    p(13, -18),  /*0b01*/
    p(40, -8),   /*0b10*/
    p(25, -38),  /*0b11*/
    p(46, -12),  /*0b100*/
    p(8, -20),   /*0b101*/
    p(72, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(64, -13),  /*0b1000*/
    p(22, -33),  /*0b1001*/
    p(82, -54),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(60, -21),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(23, -8),   /*0b1111*/
    p(25, -5),   /*0b00*/
    p(39, -13),  /*0b01*/
    p(33, -19),  /*0b10*/
    p(30, -40),  /*0b11*/
    p(41, -11),  /*0b100*/
    p(61, -19),  /*0b101*/
    p(31, -23),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(44, -6),   /*0b1000*/
    p(59, -18),  /*0b1001*/
    p(57, -43),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(48, -24),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(28, -41),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  33,   86),    p(  30,   85),    p(  21,   88),    p(  33,   69),    p(  19,   73),    p(  19,   76),    p( -19,   94),    p( -11,   93),
        p(  39,  123),    p(  47,  123),    p(  37,   99),    p(  20,   69),    p(  33,   68),    p(  14,   95),    p(  -3,  104),    p( -32,  125),
        p(  23,   73),    p(  17,   71),    p(  22,   53),    p(  17,   43),    p(  -0,   45),    p(   7,   58),    p( -10,   75),    p( -11,   79),
        p(   7,   46),    p(  -2,   43),    p( -15,   34),    p( -10,   24),    p( -17,   28),    p( -10,   39),    p( -17,   54),    p( -11,   51),
        p(   2,   14),    p( -12,   23),    p( -15,   16),    p( -15,    8),    p( -15,   13),    p(  -6,   16),    p( -14,   37),    p(  10,   17),
        p(  -5,   15),    p(  -2,   20),    p(  -9,   16),    p(  -8,    5),    p(   6,    0),    p(   8,    7),    p(  13,   18),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-10, -10);
const DOUBLED_PAWN: PhasedScore = p(-7, -21);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 11), p(7, 13), p(14, 19), p(9, 7), p(-3, 16), p(-44, 9)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(38, 9), p(39, 35), p(51, -8), p(35, -33), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-52, -73),
        p(-31, -33),
        p(-18, -10),
        p(-8, 3),
        p(-1, 14),
        p(7, 25),
        p(16, 28),
        p(23, 30),
        p(29, 29),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-1, -10),
        p(6, -0),
        p(11, 8),
        p(16, 13),
        p(20, 18),
        p(22, 22),
        p(29, 24),
        p(34, 24),
        p(42, 26),
        p(38, 33),
        p(53, 27),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-77, 13),
        p(-69, 28),
        p(-64, 33),
        p(-61, 37),
        p(-61, 44),
        p(-55, 48),
        p(-52, 52),
        p(-48, 55),
        p(-44, 59),
        p(-40, 63),
        p(-37, 65),
        p(-35, 70),
        p(-27, 70),
        p(-18, 67),
        p(-16, 67),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-28, -48),
        p(-28, 8),
        p(-32, 57),
        p(-27, 73),
        p(-24, 91),
        p(-19, 95),
        p(-15, 105),
        p(-12, 111),
        p(-8, 114),
        p(-5, 115),
        p(-2, 118),
        p(2, 120),
        p(5, 120),
        p(6, 125),
        p(9, 126),
        p(12, 129),
        p(13, 136),
        p(16, 136),
        p(25, 133),
        p(39, 126),
        p(42, 128),
        p(86, 104),
        p(85, 107),
        p(109, 89),
        p(203, 54),
        p(247, 15),
        p(277, 3),
        p(329, -33),
    ],
    [
        p(-93, 4),
        p(-59, -6),
        p(-29, -6),
        p(1, -4),
        p(33, -2),
        p(57, -3),
        p(85, 3),
        p(112, 3),
        p(163, -13),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-9, 7), p(0, 0), p(23, 19), p(49, -12), p(20, -33), p(0, 0)],
    [p(-3, 11), p(20, 22), p(0, 0), p(31, 5), p(31, 53), p(0, 0)],
    [p(-3, 13), p(11, 15), p(18, 12), p(0, 0), p(45, -5), p(0, 0)],
    [p(-2, 5), p(2, 5), p(-0, 21), p(1, 1), p(0, 0), p(0, 0)],
    [p(71, 28), p(-35, 18), p(-8, 17), p(-21, 8), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 7), p(8, 7), p(6, 11), p(13, 8), p(7, 20), p(12, 5)],
    [p(1, 6), p(11, 22), p(-126, -28), p(8, 15), p(10, 20), p(5, 7)],
    [p(3, 1), p(14, 5), p(9, 11), p(11, 8), p(11, 21), p(23, -6)],
    [p(2, -3), p(9, 0), p(7, -5), p(4, 14), p(-62, -253), p(6, -14)],
    [p(62, -3), p(42, 5), p(47, 1), p(25, 5), p(37, -9), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-21, -18), p(19, -10), p(11, -4), p(14, -12), p(-1, 12), p(-16, 10)];
const KING_ZONE_DEFENSE: [PhasedScore; 6] = [p(6, 6), p(10, 3), p(4, 2), p(-1, 0), p(-2, 8), p(0, 0)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(13, 19), p(34, -1), p(5, 32)];

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

    fn king_zone_defense(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

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

    fn king_zone_defense(piece: ChessPieceType) -> SingleFeatureScore<Self::Score> {
        KING_ZONE_DEFENSE[piece as usize]
    }

    fn can_give_check(piece: ChessPieceType) -> PhasedScore {
        CAN_GIVE_CHECK[piece as usize]
    }
}
