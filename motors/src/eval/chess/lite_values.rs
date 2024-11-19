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
        p( 133,  186),    p( 130,  185),    p( 120,  189),    p( 133,  169),    p( 119,  174),    p( 120,  177),    p(  83,  195),    p(  92,  192),    
        p(  65,  123),    p(  62,  124),    p(  74,  120),    p(  82,  124),    p(  67,  125),    p( 117,  110),    p(  92,  132),    p(  88,  122),    
        p(  51,  113),    p(  63,  109),    p(  61,  103),    p(  66,   97),    p(  82,   98),    p(  83,   94),    p(  77,  103),    p(  71,   96),    
        p(  48,  100),    p(  55,  102),    p(  64,   95),    p(  73,   94),    p(  77,   92),    p(  77,   88),    p(  71,   92),    p(  59,   86),    
        p(  43,   97),    p(  51,   94),    p(  55,   94),    p(  59,  100),    p(  67,   97),    p(  62,   93),    p(  70,   84),    p(  54,   85),    
        p(  50,   98),    p(  51,   97),    p(  58,   98),    p(  57,  105),    p(  54,  108),    p(  72,   98),    p(  73,   84),    p(  55,   87),    
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    
    ],
    // knight
    [
        p( 185,  270),    p( 209,  303),    p( 243,  315),    p( 267,  305),    p( 299,  307),    p( 213,  303),    p( 230,  302),    p( 213,  253),    
        p( 275,  304),    p( 286,  313),    p( 300,  310),    p( 313,  313),    p( 304,  309),    p( 327,  298),    p( 286,  309),    p( 290,  296),    
        p( 291,  302),    p( 302,  307),    p( 319,  316),    p( 321,  319),    p( 337,  313),    p( 360,  303),    p( 313,  303),    p( 308,  300),    
        p( 304,  311),    p( 310,  309),    p( 317,  320),    p( 343,  322),    p( 320,  324),    p( 334,  321),    p( 315,  312),    p( 332,  305),    
        p( 300,  314),    p( 299,  308),    p( 305,  320),    p( 312,  324),    p( 319,  325),    p( 316,  312),    p( 327,  304),    p( 316,  309),    
        p( 275,  300),    p( 276,  304),    p( 284,  304),    p( 290,  318),    p( 297,  314),    p( 283,  298),    p( 298,  295),    p( 294,  304),    
        p( 271,  305),    p( 281,  309),    p( 277,  305),    p( 289,  309),    p( 292,  303),    p( 285,  300),    p( 295,  301),    p( 290,  314),    
        p( 244,  301),    p( 282,  300),    p( 266,  302),    p( 285,  307),    p( 296,  304),    p( 291,  293),    p( 289,  300),    p( 268,  299),    
    ],
    // bishop
    [
        p( 280,  315),    p( 255,  314),    p( 248,  307),    p( 224,  315),    p( 223,  315),    p( 225,  307),    p( 282,  304),    p( 252,  308),    
        p( 280,  303),    p( 285,  306),    p( 288,  306),    p( 281,  308),    p( 284,  303),    p( 294,  303),    p( 272,  308),    p( 274,  303),    
        p( 298,  309),    p( 303,  305),    p( 295,  310),    p( 303,  302),    p( 307,  306),    p( 333,  309),    p( 319,  303),    p( 311,  311),    
        p( 281,  310),    p( 298,  309),    p( 300,  305),    p( 317,  311),    p( 311,  307),    p( 307,  309),    p( 302,  308),    p( 282,  311),    
        p( 292,  308),    p( 281,  311),    p( 300,  309),    p( 314,  308),    p( 313,  308),    p( 298,  307),    p( 291,  309),    p( 311,  299),    
        p( 293,  307),    p( 304,  310),    p( 301,  310),    p( 304,  310),    p( 307,  310),    p( 304,  307),    p( 306,  302),    p( 310,  300),    
        p( 309,  311),    p( 303,  300),    p( 311,  303),    p( 296,  310),    p( 302,  308),    p( 303,  305),    p( 313,  301),    p( 303,  298),    
        p( 294,  304),    p( 314,  309),    p( 306,  307),    p( 290,  311),    p( 303,  309),    p( 296,  313),    p( 303,  297),    p( 301,  295),    
    ],
    // rook
    [
        p( 458,  550),    p( 449,  559),    p( 446,  565),    p( 444,  562),    p( 456,  559),    p( 476,  553),    p( 484,  552),    p( 494,  545),    
        p( 432,  556),    p( 429,  562),    p( 438,  562),    p( 454,  552),    p( 444,  554),    p( 464,  549),    p( 475,  546),    p( 490,  537),    
        p( 437,  553),    p( 455,  548),    p( 453,  550),    p( 457,  545),    p( 484,  534),    p( 493,  530),    p( 516,  527),    p( 488,  530),    
        p( 435,  552),    p( 442,  548),    p( 443,  551),    p( 448,  546),    p( 457,  537),    p( 466,  532),    p( 473,  534),    p( 469,  529),    
        p( 430,  548),    p( 430,  547),    p( 431,  548),    p( 437,  545),    p( 444,  541),    p( 438,  540),    p( 457,  533),    p( 447,  531),    
        p( 427,  544),    p( 426,  542),    p( 430,  541),    p( 432,  542),    p( 439,  536),    p( 447,  528),    p( 470,  515),    p( 452,  520),    
        p( 430,  538),    p( 433,  539),    p( 439,  540),    p( 442,  538),    p( 450,  531),    p( 464,  521),    p( 473,  516),    p( 441,  525),    
        p( 439,  542),    p( 435,  539),    p( 437,  543),    p( 441,  539),    p( 449,  533),    p( 455,  532),    p( 452,  529),    p( 446,  530),    
    ],
    // queen
    [
        p( 874,  968),    p( 876,  982),    p( 890,  995),    p( 907,  992),    p( 906,  995),    p( 925,  983),    p( 976,  931),    p( 921,  962),    
        p( 883,  961),    p( 859,  992),    p( 861, 1020),    p( 853, 1037),    p( 861, 1048),    p( 900, 1008),    p( 903,  989),    p( 946,  967),    
        p( 891,  965),    p( 883,  985),    p( 883, 1008),    p( 881, 1017),    p( 903, 1018),    p( 943, 1002),    p( 950,  971),    p( 938,  977),    
        p( 876,  980),    p( 882,  990),    p( 876, 1000),    p( 875, 1014),    p( 880, 1024),    p( 892, 1014),    p( 902, 1013),    p( 909,  990),    
        p( 887,  969),    p( 874,  989),    p( 880,  992),    p( 880, 1010),    p( 881, 1008),    p( 883, 1007),    p( 898,  992),    p( 905,  984),    
        p( 883,  955),    p( 889,  973),    p( 882,  989),    p( 879,  992),    p( 884,  999),    p( 890,  989),    p( 905,  970),    p( 904,  957),    
        p( 886,  953),    p( 883,  962),    p( 890,  966),    p( 889,  979),    p( 890,  979),    p( 892,  962),    p( 902,  939),    p( 911,  912),    
        p( 872,  950),    p( 883,  940),    p( 883,  955),    p( 891,  956),    p( 893,  949),    p( 882,  949),    p( 882,  938),    p( 886,  927),    
    ],
    // king
    [
        p( 152, -103),    p(  57,  -49),    p(  81,  -41),    p(   4,   -9),    p(  26,  -21),    p(  10,  -12),    p(  64,  -21),    p( 220, -106),    
        p( -23,   -3),    p( -69,   26),    p( -77,   36),    p( -12,   25),    p( -44,   34),    p( -72,   47),    p( -37,   32),    p(   9,   -1),    
        p( -44,    5),    p( -37,   22),    p( -81,   40),    p( -87,   48),    p( -53,   42),    p( -20,   35),    p( -57,   33),    p( -32,   10),    
        p( -27,   -1),    p( -91,   22),    p(-106,   39),    p(-128,   49),    p(-127,   46),    p(-107,   38),    p(-112,   27),    p( -99,   15),    
        p( -46,   -4),    p(-112,   17),    p(-122,   34),    p(-145,   47),    p(-150,   45),    p(-127,   31),    p(-140,   22),    p(-118,   12),    
        p( -37,   -1),    p( -88,   12),    p(-118,   27),    p(-126,   36),    p(-121,   34),    p(-135,   27),    p(-105,   12),    p( -75,    9),    
        p(  28,  -10),    p( -70,    6),    p( -82,   15),    p(-103,   24),    p(-109,   25),    p( -93,   16),    p( -62,    0),    p(   4,   -4),    
        p(  46,  -42),    p(  43,  -47),    p(  37,  -35),    p( -24,  -14),    p(  29,  -33),    p( -20,  -18),    p(  36,  -43),    p(  62,  -52),    
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 58);
const ROOK_OPEN_FILE: PhasedScore = p(16, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(0, 2);
const KING_OPEN_FILE: PhasedScore = p(-56, -1);
const KING_CLOSED_FILE: PhasedScore = p(16, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 7);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 5), p(-2, 7), p(-2, 9), p(3, 7), p(4, 10), p(5, 12), p(10, 11), p(21, 6)], 
    // Closed
    [p(-2, 7), p(-2, 9), p(3, 7), p(4, 10), p(5, 12), p(10, 11), p(21, 6), p(0, 0)], 
    // SemiOpen
    [p(-2, 9), p(3, 7), p(4, 10), p(5, 12), p(10, 11), p(21, 6), p(0, 0), p(0, 0)], 
    // SemiClosed
    [p(3, 7), p(4, 10), p(5, 12), p(10, 11), p(21, 6), p(0, 0), p(0, 0), p(13, -31)], 
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-7, 7),    /*0b0000*/
    p(-16, 12),  /*0b0001*/
    p(-3, 8),    /*0b0010*/
    p(-10, 15),  /*0b0011*/
    p(-5, 7),    /*0b0100*/
    p(-27, 4),   /*0b0101*/
    p(-14, 7),   /*0b0110*/
    p(-19, -16), /*0b0111*/
    p(5, 10),    /*0b1000*/
    p(-6, 11),   /*0b1001*/
    p(1, 9),     /*0b1010*/
    p(-3, 11),   /*0b1011*/
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
    p(12, 33),   /*0b11000*/
    p(30, 26),   /*0b11001*/
    p(41, 40),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(15, 14),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(14, 10),   /*0b100000*/
    p(4, 15),    /*0b100001*/
    p(25, 4),    /*0b100010*/
    p(6, 2),     /*0b100011*/
    p(-11, 4),   /*0b100100*/
    p(-23, -7),  /*0b100101*/
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
    p(7, 32),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(23, 16),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(5, -0),    /*0b111111*/
    p(-21, -10), /*0b00*/
    p(8, -25),   /*0b01*/
    p(36, -14),  /*0b10*/
    p(24, -50),  /*0b11*/
    p(46, -18),  /*0b100*/
    p(-5, -28),  /*0b101*/
    p(74, -49),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(55, -20),  /*0b1000*/
    p(19, -44),  /*0b1001*/
    p(78, -64),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(55, -26),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(14, -5),   /*0b1111*/
    p(16, -10),  /*0b00*/
    p(32, -21),  /*0b01*/
    p(26, -27),  /*0b10*/
    p(24, -53),  /*0b11*/
    p(32, -18),  /*0b100*/
    p(54, -29),  /*0b101*/
    p(23, -34),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(37, -12),  /*0b1000*/
    p(54, -27),  /*0b1001*/
    p(51, -52),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(41, -31),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(22, -54),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    
        p(  33,   86),    p(  30,   85),    p(  20,   89),    p(  33,   69),    p(  19,   74),    p(  20,   77),    p( -17,   95),    p(  -8,   92),    
        p(  41,  122),    p(  48,  122),    p(  37,   99),    p(  21,   67),    p(  36,   66),    p(  15,   94),    p(   1,  102),    p( -27,  123),    
        p(  24,   72),    p(  17,   70),    p(  23,   53),    p(  16,   43),    p(  -1,   44),    p(   7,   57),    p( -11,   74),    p( -10,   77),    
        p(   8,   45),    p(  -3,   43),    p( -15,   34),    p(  -9,   24),    p( -17,   29),    p( -11,   37),    p( -19,   54),    p( -11,   50),    
        p(   2,   14),    p( -12,   22),    p( -15,   16),    p( -16,    8),    p( -14,   13),    p(  -8,   17),    p( -14,   36),    p(   9,   16),    
        p(  -4,   15),    p(  -2,   20),    p(  -9,   16),    p(  -8,    5),    p(   6,    1),    p(   7,    7),    p(  12,   18),    p(   7,   13),    
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    
];
const UNSUPPORTED_PAWN: PhasedScore = p(-11, -10);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 7), p(2, 9), p(10, 14), p(9, 9), p(-4, 19), p(-46, 9)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(38, 9), p(42, 36), p(51, -9), p(37, -39), p(0, 0)];

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
        p(-14, -30),
        p(-4, -15),
        p(3, -3),
        p(9, 6),
        p(13, 14),
        p(16, 18),
        p(18, 22),
        p(19, 26),
        p(25, 26),
        p(29, 25),
        p(37, 26),
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
        p(-46, 54),
        p(-42, 58),
        p(-39, 62),
        p(-33, 64),
        p(-30, 68),
        p(-20, 67),
        p(-8, 64),
        p(-5, 65),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-35, -36),
        p(-35, 21),
        p(-39, 70),
        p(-33, 87),
        p(-31, 104),
        p(-26, 109),
        p(-21, 119),
        p(-18, 126),
        p(-13, 130),
        p(-10, 132),
        p(-7, 136),
        p(-3, 138),
        p(-1, 139),
        p(1, 144),
        p(3, 145),
        p(7, 148),
        p(8, 154),
        p(11, 153),
        p(20, 150),
        p(34, 141),
        p(39, 141),
        p(82, 117),
        p(82, 118),
        p(106, 98),
        p(198, 63),
        p(250, 18),
        p(286, 2),
        p(338, -32),
    ],
    [
        p(-84, 49),
        p(-52, 22),
        p(-26, 11),
        p(1, 4),
        p(28, -2),
        p(48, -10),
        p(71, -10),
        p(92, -17),
        p(138, -42),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-6, -3),
        p(23, 17),
        p(49, -15),
        p(21, -44),
        p(0, 0),
    ],
    [p(-1, 13), p(19, 21), p(-2, 9), p(28, 2), p(27, 56), p(0, 0)],
    [
        p(3, 17),
        p(22, 21),
        p(23, 21),
        p(-6, 11),
        p(43, -4),
        p(0, 0),
    ],
    [p(-0, -2), p(7, 12), p(-0, 30), p(0, 6), p(2, -17), p(0, 0)],
    [p(79, 34), p(-30, 22), p(2, 19), p(-33, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 5), p(11, 4), p(9, 10), p(15, 5), p(9, 16), p(13, 3)],
    [p(-3, 1), p(8, 18), p(-98, -35), p(6, 12), p(7, 16), p(4, 5)],
    [p(3, 2), p(14, 4), p(9, 10), p(11, 7), p(12, 15), p(22, -6)],
    [
        p(3, -5),
        p(10, -2),
        p(8, -8),
        p(4, 15),
        p(-57, -260),
        p(7, -11),
    ],
    [
        p(60, -8),
        p(38, -1),
        p(43, -6),
        p(21, -2),
        p(34, -18),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-13, -10),
    p(17, -8),
    p(17, -3),
    p(23, -13),
    p(6, 22),
    p(6, 19),
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

    fn bishop_pair() -> SingleFeatureScore<Self::Score> {
        BISHOP_PAIR
    }

    fn rook_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score> {
        match openness {
            FileOpenness::Open => ROOK_OPEN_FILE,
            FileOpenness::Closed => ROOK_CLOSED_FILE,
            FileOpenness::SemiOpen => ROOK_SEMIOPEN_FILE,
            FileOpenness::SemiClosed => Self::Score::default(),
        }
    }

    fn king_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score> {
        match openness {
            FileOpenness::Open => KING_OPEN_FILE,
            FileOpenness::Closed => KING_CLOSED_FILE,
            FileOpenness::SemiOpen => KING_SEMIOPEN_FILE,
            FileOpenness::SemiClosed => Self::Score::default(),
        }
    }

    fn bishop_openness(openness: FileOpenness, len: usize) -> PhasedScore {
        BISHOP_OPENNESS[openness as usize][len - 1]
    }

    fn pawn_shield(&self, _color: ChessColor, config: usize) -> SingleFeatureScore<Self::Score> {
        PAWN_SHIELDS[config]
    }

    fn pawn_protection(piece: ChessPieceType) -> SingleFeatureScore<Self::Score> {
        PAWN_PROTECTION[piece as usize]
    }

    fn pawn_attack(piece: ChessPieceType) -> SingleFeatureScore<Self::Score> {
        PAWN_ATTACKS[piece as usize]
    }

    fn mobility(piece: ChessPieceType, mobility: usize) -> SingleFeatureScore<Self::Score> {
        MOBILITY[piece as usize - 1][mobility]
    }

    fn threats(
        attacking: ChessPieceType,
        targeted: ChessPieceType,
    ) -> SingleFeatureScore<Self::Score> {
        THREATS[attacking as usize - 1][targeted as usize]
    }
    fn defended(
        protecting: ChessPieceType,
        target: ChessPieceType,
    ) -> SingleFeatureScore<Self::Score> {
        DEFENDED[protecting as usize - 1][target as usize]
    }

    fn king_zone_attack(attacking: ChessPieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }
}
