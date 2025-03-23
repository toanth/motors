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
        p( 133,  187),    p( 130,  186),    p( 121,  189),    p( 133,  169),    p( 120,  174),    p( 119,  177),    p(  81,  195),    p(  88,  193),
        p(  68,  123),    p(  67,  124),    p(  78,  120),    p(  87,  123),    p(  75,  124),    p( 123,  110),    p(  96,  131),    p(  93,  121),
        p(  55,  112),    p(  67,  108),    p(  65,  103),    p(  68,   98),    p(  83,   98),    p(  88,   93),    p(  80,  102),    p(  74,   94),
        p(  51,   99),    p(  58,  102),    p(  67,   95),    p(  75,   94),    p(  75,   95),    p(  74,   90),    p(  71,   93),    p(  59,   87),
        p(  46,   97),    p(  55,   92),    p(  58,   94),    p(  60,  101),    p(  64,   98),    p(  60,   94),    p(  70,   83),    p(  54,   86),
        p(  52,   99),    p(  55,   95),    p(  61,   98),    p(  60,  105),    p(  54,  109),    p(  70,   99),    p(  73,   85),    p(  56,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 177,  276),    p( 199,  308),    p( 216,  318),    p( 254,  309),    p( 283,  311),    p( 200,  307),    p( 216,  307),    p( 206,  259),
        p( 270,  308),    p( 285,  314),    p( 300,  305),    p( 304,  310),    p( 303,  305),    p( 317,  294),    p( 277,  311),    p( 274,  301),
        p( 288,  304),    p( 304,  301),    p( 307,  307),    p( 321,  312),    p( 338,  306),    p( 351,  295),    p( 292,  302),    p( 287,  306),
        p( 302,  312),    p( 309,  305),    p( 324,  310),    p( 327,  318),    p( 324,  317),    p( 320,  316),    p( 310,  313),    p( 319,  309),
        p( 299,  315),    p( 303,  305),    p( 312,  311),    p( 319,  316),    p( 314,  319),    p( 318,  305),    p( 317,  305),    p( 310,  314),
        p( 276,  302),    p( 282,  299),    p( 293,  296),    p( 300,  309),    p( 300,  308),    p( 290,  291),    p( 296,  294),    p( 292,  307),
        p( 270,  310),    p( 282,  312),    p( 283,  302),    p( 294,  305),    p( 295,  301),    p( 286,  300),    p( 293,  306),    p( 288,  320),
        p( 240,  310),    p( 282,  302),    p( 266,  305),    p( 287,  310),    p( 290,  309),    p( 290,  297),    p( 288,  306),    p( 264,  308),
    ],
    // bishop
    [
        p( 276,  309),    p( 253,  313),    p( 241,  305),    p( 225,  316),    p( 220,  313),    p( 226,  307),    p( 275,  302),    p( 254,  308),
        p( 280,  302),    p( 280,  301),    p( 291,  304),    p( 279,  303),    p( 290,  299),    p( 294,  298),    p( 271,  306),    p( 271,  301),
        p( 296,  307),    p( 307,  303),    p( 293,  302),    p( 309,  296),    p( 307,  297),    p( 339,  302),    p( 319,  299),    p( 318,  312),
        p( 287,  312),    p( 294,  305),    p( 305,  299),    p( 309,  305),    p( 310,  302),    p( 301,  303),    p( 298,  308),    p( 281,  310),
        p( 291,  307),    p( 285,  308),    p( 296,  303),    p( 310,  305),    p( 300,  302),    p( 296,  304),    p( 284,  305),    p( 307,  304),
        p( 297,  309),    p( 302,  303),    p( 302,  308),    p( 301,  305),    p( 303,  308),    p( 296,  300),    p( 303,  297),    p( 306,  300),
        p( 308,  309),    p( 305,  300),    p( 310,  300),    p( 299,  309),    p( 299,  306),    p( 303,  303),    p( 311,  296),    p( 307,  296),
        p( 298,  305),    p( 311,  306),    p( 308,  307),    p( 291,  309),    p( 303,  309),    p( 294,  309),    p( 305,  296),    p( 301,  293),
    ],
    // rook
    [
        p( 460,  543),    p( 451,  552),    p( 444,  559),    p( 443,  557),    p( 454,  553),    p( 473,  549),    p( 486,  545),    p( 495,  539),
        p( 445,  551),    p( 443,  556),    p( 453,  557),    p( 468,  547),    p( 454,  551),    p( 471,  544),    p( 479,  541),    p( 492,  532),
        p( 447,  546),    p( 464,  541),    p( 459,  543),    p( 460,  537),    p( 487,  527),    p( 497,  524),    p( 513,  523),    p( 487,  525),
        p( 444,  545),    p( 449,  540),    p( 448,  543),    p( 455,  537),    p( 460,  529),    p( 471,  525),    p( 471,  529),    p( 469,  525),
        p( 437,  542),    p( 436,  539),    p( 436,  541),    p( 441,  537),    p( 444,  536),    p( 438,  536),    p( 453,  528),    p( 447,  527),
        p( 433,  539),    p( 432,  536),    p( 433,  536),    p( 436,  535),    p( 437,  533),    p( 448,  526),    p( 466,  513),    p( 453,  517),
        p( 434,  535),    p( 438,  534),    p( 443,  536),    p( 446,  533),    p( 449,  529),    p( 462,  520),    p( 470,  514),    p( 442,  523),
        p( 444,  539),    p( 441,  534),    p( 441,  539),    p( 445,  533),    p( 446,  531),    p( 453,  530),    p( 452,  527),    p( 448,  529),
    ],
    // queen
    [
        p( 879,  956),    p( 883,  970),    p( 897,  984),    p( 920,  977),    p( 918,  982),    p( 937,  971),    p( 985,  921),    p( 928,  954),
        p( 889,  947),    p( 864,  975),    p( 867, 1004),    p( 859, 1023),    p( 867, 1035),    p( 908,  995),    p( 911,  975),    p( 949,  958),
        p( 894,  953),    p( 886,  968),    p( 886,  990),    p( 888, 1000),    p( 911, 1002),    p( 949,  989),    p( 956,  959),    p( 943,  967),
        p( 881,  964),    p( 887,  970),    p( 882,  979),    p( 882,  994),    p( 885, 1008),    p( 898, 1000),    p( 907, 1002),    p( 913,  979),
        p( 892,  954),    p( 880,  972),    p( 885,  976),    p( 885,  992),    p( 884,  994),    p( 886,  995),    p( 900,  982),    p( 907,  976),
        p( 888,  943),    p( 895,  958),    p( 888,  975),    p( 884,  979),    p( 886,  989),    p( 892,  980),    p( 907,  963),    p( 906,  952),
        p( 888,  946),    p( 888,  955),    p( 894,  958),    p( 893,  972),    p( 891,  975),    p( 894,  955),    p( 906,  934),    p( 915,  907),
        p( 875,  948),    p( 887,  936),    p( 886,  950),    p( 894,  951),    p( 890,  947),    p( 880,  947),    p( 884,  938),    p( 889,  921),
    ],
    // king
    [
        p( 157,  -88),    p(  60,  -40),    p(  83,  -31),    p(   8,    1),    p(  37,  -12),    p(  21,   -2),    p(  75,  -11),    p( 236,  -91),
        p( -30,   -1),    p( -82,   16),    p( -84,   24),    p( -22,   14),    p( -53,   23),    p( -83,   37),    p( -53,   22),    p(   8,   -2),
        p( -47,    7),    p( -48,   10),    p( -87,   26),    p( -95,   34),    p( -65,   30),    p( -33,   22),    p( -78,   23),    p( -35,    8),
        p( -26,   -1),    p(-101,    9),    p(-115,   26),    p(-136,   35),    p(-134,   33),    p(-114,   26),    p(-130,   14),    p(-104,   15),
        p( -40,   -4),    p(-113,    6),    p(-125,   24),    p(-150,   37),    p(-153,   36),    p(-128,   22),    p(-143,   12),    p(-115,   11),
        p( -32,    0),    p( -92,    3),    p(-119,   18),    p(-127,   27),    p(-125,   26),    p(-135,   18),    p(-110,    5),    p( -72,    9),
        p(  25,   -8),    p( -79,   -1),    p( -91,    8),    p(-111,   17),    p(-117,   18),    p(-101,    9),    p( -74,   -8),    p(   3,   -4),
        p(  52,  -23),    p(  39,  -33),    p(  37,  -20),    p( -26,    0),    p(  25,  -15),    p( -20,   -3),    p(  33,  -27),    p(  64,  -32),
    ],
];

const BISHOP_PAIR: PhasedScore = p(22, 53);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(8, 19), p(10, 18), p(10, 7), p(7, -1), p(3, -9), p(-1, -19), p(-8, -28), p(-16, -41), p(-27, -53)];
const ROOK_OPEN_FILE: PhasedScore = p(14, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -0);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 3);
const KING_OPEN_FILE: PhasedScore = p(-47, -2);
const KING_CLOSED_FILE: PhasedScore = p(14, -14);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-5, 3), p(1, 5), p(-2, 4), p(2, 2), p(2, 4), p(3, 7), p(5, 4), p(18, 1)],
    // Closed
    [p(0, 0), p(0, 0), p(14, -22), p(-16, 7), p(-1, 10), p(1, 4), p(-1, 7), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-14, 18), p(3, 14), p(1, 7), p(-0, 7), p(4, 4), p(-1, 2), p(10, 5)],
    // SemiClosed
    [p(0, 0), p(12, -14), p(6, 6), p(3, 0), p(7, 1), p(2, 4), p(4, 5), p(1, 4)],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-2, 5),    /*0b0000*/
    p(-12, 7),   /*0b0001*/
    p(1, 6),     /*0b0010*/
    p(-8, 12),   /*0b0011*/
    p(-2, 2),    /*0b0100*/
    p(-24, -2),  /*0b0101*/
    p(-12, 4),   /*0b0110*/
    p(-18, -16), /*0b0111*/
    p(9, 11),    /*0b1000*/
    p(-0, 10),   /*0b1001*/
    p(4, 12),    /*0b1010*/
    p(-3, 12),   /*0b1011*/
    p(-1, 4),    /*0b1100*/
    p(-21, 8),   /*0b1101*/
    p(-11, 5),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(5, 14),    /*0b10000*/
    p(1, 10),    /*0b10001*/
    p(24, 8),    /*0b10010*/
    p(-7, 9),    /*0b10011*/
    p(-5, 8),    /*0b10100*/
    p(12, 18),   /*0b10101*/
    p(-25, 4),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(17, 28),   /*0b11000*/
    p(28, 25),   /*0b11001*/
    p(44, 37),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(16, 12),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(17, 10),   /*0b100000*/
    p(3, 13),    /*0b100001*/
    p(25, 4),    /*0b100010*/
    p(5, 1),     /*0b100011*/
    p(-7, 2),    /*0b100100*/
    p(-20, -8),  /*0b100101*/
    p(-21, 18),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(22, 4),    /*0b101000*/
    p(4, 13),    /*0b101001*/
    p(20, 2),    /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-7, 8),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(17, 17),   /*0b110000*/
    p(23, 14),   /*0b110001*/
    p(34, 10),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(9, 31),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(27, 14),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(6, 4),     /*0b111111*/
    p(-11, -3),  /*0b00*/
    p(12, -19),  /*0b01*/
    p(40, -9),   /*0b10*/
    p(21, -39),  /*0b11*/
    p(47, -10),  /*0b100*/
    p(8, -22),   /*0b101*/
    p(67, -36),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(64, -11),  /*0b1000*/
    p(17, -30),  /*0b1001*/
    p(77, -53),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(58, -17),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(17, -4),   /*0b1111*/
    p(23, -2),   /*0b00*/
    p(34, -13),  /*0b01*/
    p(28, -18),  /*0b10*/
    p(22, -40),  /*0b11*/
    p(36, -8),   /*0b100*/
    p(55, -20),  /*0b101*/
    p(23, -20),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(40, -3),   /*0b1000*/
    p(50, -14),  /*0b1001*/
    p(53, -43),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(43, -21),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(18, -38),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  33,   87),    p(  30,   86),    p(  21,   89),    p(  33,   69),    p(  20,   74),    p(  19,   77),    p( -19,   95),    p( -12,   93),
        p(  39,  124),    p(  47,  123),    p(  37,  100),    p(  20,   70),    p(  33,   68),    p(  15,   95),    p(  -1,  104),    p( -32,  126),
        p(  23,   74),    p(  17,   71),    p(  22,   54),    p(  17,   43),    p(  -0,   46),    p(   8,   58),    p(  -9,   76),    p( -10,   79),
        p(   7,   47),    p(  -2,   44),    p( -15,   34),    p(  -9,   24),    p( -16,   27),    p(  -9,   38),    p( -17,   54),    p( -11,   51),
        p(   1,   15),    p( -12,   24),    p( -15,   16),    p( -15,    7),    p( -14,   13),    p(  -6,   16),    p( -13,   37),    p(  10,   16),
        p(  -5,   15),    p(  -2,   21),    p(  -9,   17),    p(  -8,    5),    p(   6,    0),    p(   8,    6),    p(  14,   18),    p(   8,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-10, -10);
const DOUBLED_PAWN: PhasedScore = p(-7, -21);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(12, 13), p(6, 15), p(12, 20), p(8, 9), p(-5, 17), p(-53, 13)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(38, 12), p(39, 38), p(51, -5), p(35, -31), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-49, -72),
        p(-28, -32),
        p(-15, -8),
        p(-6, 5),
        p(2, 16),
        p(10, 27),
        p(19, 29),
        p(27, 31),
        p(33, 29),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-30, -56),
        p(-19, -38),
        p(-8, -23),
        p(-1, -10),
        p(6, -0),
        p(11, 8),
        p(16, 13),
        p(20, 18),
        p(22, 22),
        p(30, 24),
        p(35, 23),
        p(43, 26),
        p(40, 32),
        p(56, 26),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-77, 12),
        p(-68, 26),
        p(-63, 31),
        p(-61, 36),
        p(-61, 42),
        p(-55, 48),
        p(-52, 52),
        p(-48, 55),
        p(-44, 58),
        p(-41, 62),
        p(-37, 65),
        p(-35, 69),
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
        p(-31, -47),
        p(-29, 7),
        p(-33, 56),
        p(-28, 73),
        p(-25, 91),
        p(-20, 96),
        p(-17, 107),
        p(-13, 113),
        p(-9, 117),
        p(-6, 118),
        p(-3, 121),
        p(1, 123),
        p(4, 124),
        p(5, 128),
        p(8, 129),
        p(11, 132),
        p(12, 139),
        p(15, 140),
        p(24, 137),
        p(38, 130),
        p(42, 132),
        p(85, 109),
        p(85, 112),
        p(107, 94),
        p(202, 59),
        p(247, 18),
        p(274, 9),
        p(332, -28),
    ],
    [
        p(-94, 7),
        p(-58, -5),
        p(-28, -5),
        p(2, -3),
        p(34, -1),
        p(58, -3),
        p(87, 4),
        p(114, 3),
        p(165, -14),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-9, 12), p(0, 0), p(23, 23), p(49, -9), p(19, -30), p(0, 0)],
    [p(-3, 16), p(19, 27), p(0, 0), p(31, 8), p(30, 57), p(0, 0)],
    [p(-4, 18), p(10, 19), p(17, 17), p(0, 0), p(45, -3), p(0, 0)],
    [p(-3, 10), p(2, 10), p(-1, 26), p(1, 5), p(0, 0), p(0, 0)],
    [p(69, 32), p(-36, 21), p(-10, 20), p(-22, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 8), p(8, 8), p(6, 12), p(13, 8), p(7, 19), p(8, 11)],
    [p(0, 8), p(10, 23), p(-133, -24), p(8, 15), p(8, 22), p(2, 12)],
    [p(2, 3), p(13, 7), p(8, 13), p(10, 10), p(9, 23), p(17, 1)],
    [p(2, -1), p(8, 2), p(6, -3), p(4, 15), p(-63, -249), p(1, -6)],
    [p(60, 4), p(36, 11), p(41, 6), p(22, 8), p(33, -7), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-21, -18), p(19, -8), p(10, -3), p(14, -9), p(-3, 15), p(-13, 11)];
const FLANK_ATTACK: PhasedScore = p(2, -13);
const FLANK_DEFENSE: PhasedScore = p(8, -9);
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(14, 19), p(34, -0), p(6, 32)];

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

    fn flank_attack() -> SingleFeatureScore<Self::Score>;

    fn flank_defense() -> SingleFeatureScore<Self::Score>;

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

    fn flank_attack() -> PhasedScore {
        FLANK_ATTACK
    }

    fn flank_defense() -> PhasedScore {
        FLANK_DEFENSE
    }

    fn can_give_check(piece: ChessPieceType) -> PhasedScore {
        CAN_GIVE_CHECK[piece as usize]
    }
}
