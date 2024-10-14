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
use gears::games::chess::pieces::ChessPieceType::Knight;
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
        p( 133,  186),    p( 130,  185),    p( 120,  189),    p( 133,  169),    p( 120,  173),    p( 120,  177),    p(  83,  195),    p(  92,  192),
        p(  65,  123),    p(  62,  125),    p(  74,  120),    p(  82,  124),    p(  66,  125),    p( 117,  111),    p(  91,  132),    p(  88,  122),
        p(  51,  113),    p(  63,  109),    p(  60,  104),    p(  65,   97),    p(  81,   98),    p(  83,   94),    p(  77,  103),    p(  71,   96),
        p(  48,  100),    p(  55,  102),    p(  63,   95),    p(  72,   94),    p(  76,   93),    p(  77,   89),    p(  70,   92),    p(  59,   86),
        p(  43,   97),    p(  51,   95),    p(  55,   95),    p(  58,  100),    p(  67,   97),    p(  61,   93),    p(  69,   84),    p(  54,   85),
        p(  50,   98),    p(  51,   97),    p(  57,   98),    p(  57,  105),    p(  53,  108),    p(  72,   99),    p(  72,   85),    p(  55,   87),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 184,  271),    p( 209,  303),    p( 242,  315),    p( 266,  306),    p( 298,  307),    p( 211,  303),    p( 228,  302),    p( 212,  253),
        p( 275,  304),    p( 286,  313),    p( 298,  311),    p( 311,  313),    p( 302,  310),    p( 324,  298),    p( 284,  309),    p( 289,  296),
        p( 291,  302),    p( 300,  307),    p( 315,  316),    p( 314,  321),    p( 332,  314),    p( 358,  303),    p( 312,  303),    p( 307,  300),
        p( 303,  311),    p( 309,  309),    p( 315,  320),    p( 341,  322),    p( 319,  324),    p( 333,  321),    p( 314,  312),    p( 332,  305),
        p( 300,  314),    p( 299,  308),    p( 305,  320),    p( 312,  324),    p( 319,  325),    p( 316,  312),    p( 327,  304),    p( 316,  310),
        p( 275,  300),    p( 277,  304),    p( 285,  304),    p( 290,  318),    p( 298,  314),    p( 284,  299),    p( 299,  296),    p( 294,  304),
        p( 271,  305),    p( 281,  309),    p( 278,  305),    p( 289,  309),    p( 292,  303),    p( 285,  300),    p( 295,  301),    p( 290,  314),
        p( 244,  301),    p( 282,  299),    p( 266,  302),    p( 286,  307),    p( 296,  304),    p( 292,  293),    p( 290,  299),    p( 268,  299),
    ],
    // bishop
    [
        p( 280,  315),    p( 256,  314),    p( 248,  307),    p( 224,  315),    p( 223,  315),    p( 225,  307),    p( 282,  304),    p( 252,  308),
        p( 279,  303),    p( 283,  307),    p( 286,  308),    p( 280,  309),    p( 282,  304),    p( 292,  304),    p( 270,  309),    p( 274,  304),
        p( 297,  310),    p( 300,  307),    p( 291,  313),    p( 296,  307),    p( 301,  309),    p( 331,  310),    p( 317,  305),    p( 311,  312),
        p( 280,  311),    p( 297,  310),    p( 297,  308),    p( 313,  315),    p( 309,  309),    p( 306,  310),    p( 301,  308),    p( 282,  312),
        p( 291,  308),    p( 281,  311),    p( 300,  309),    p( 314,  309),    p( 313,  308),    p( 299,  305),    p( 291,  308),    p( 311,  300),
        p( 293,  307),    p( 305,  308),    p( 301,  309),    p( 305,  309),    p( 308,  309),    p( 305,  305),    p( 307,  299),    p( 310,  300),
        p( 308,  311),    p( 303,  301),    p( 310,  303),    p( 296,  310),    p( 302,  308),    p( 303,  306),    p( 313,  301),    p( 302,  298),
        p( 294,  304),    p( 314,  308),    p( 306,  307),    p( 290,  312),    p( 303,  309),    p( 296,  313),    p( 304,  297),    p( 301,  295),
    ],
    // rook
    [
        p( 458,  550),    p( 449,  559),    p( 446,  565),    p( 444,  562),    p( 456,  558),    p( 476,  553),    p( 485,  552),    p( 495,  545),
        p( 433,  556),    p( 430,  561),    p( 439,  562),    p( 455,  552),    p( 445,  554),    p( 465,  549),    p( 476,  546),    p( 490,  537),
        p( 438,  553),    p( 456,  548),    p( 454,  550),    p( 457,  545),    p( 485,  534),    p( 493,  530),    p( 516,  527),    p( 488,  529),
        p( 435,  552),    p( 442,  548),    p( 443,  551),    p( 449,  546),    p( 458,  537),    p( 466,  532),    p( 473,  534),    p( 470,  529),
        p( 431,  548),    p( 430,  547),    p( 431,  548),    p( 438,  544),    p( 444,  541),    p( 438,  540),    p( 458,  532),    p( 447,  531),
        p( 428,  544),    p( 427,  542),    p( 430,  541),    p( 432,  542),    p( 440,  535),    p( 448,  528),    p( 471,  515),    p( 453,  520),
        p( 430,  538),    p( 434,  539),    p( 440,  540),    p( 443,  538),    p( 450,  531),    p( 465,  521),    p( 473,  516),    p( 442,  525),
        p( 439,  542),    p( 436,  538),    p( 437,  543),    p( 442,  539),    p( 449,  533),    p( 455,  532),    p( 453,  529),    p( 447,  530),
    ],
    // queen
    [
        p( 874,  968),    p( 876,  982),    p( 891,  995),    p( 908,  992),    p( 907,  995),    p( 926,  983),    p( 976,  931),    p( 922,  962),
        p( 884,  961),    p( 860,  992),    p( 861, 1019),    p( 854, 1037),    p( 861, 1048),    p( 901, 1008),    p( 904,  989),    p( 946,  967),
        p( 892,  965),    p( 884,  985),    p( 884, 1008),    p( 882, 1017),    p( 904, 1018),    p( 944, 1002),    p( 951,  972),    p( 938,  977),
        p( 877,  980),    p( 883,  990),    p( 877, 1000),    p( 876, 1014),    p( 881, 1024),    p( 893, 1014),    p( 902, 1013),    p( 909,  989),
        p( 888,  969),    p( 874,  989),    p( 881,  992),    p( 881, 1010),    p( 882, 1007),    p( 884, 1007),    p( 899,  992),    p( 906,  984),
        p( 884,  954),    p( 889,  973),    p( 882,  989),    p( 880,  992),    p( 885,  999),    p( 891,  989),    p( 906,  970),    p( 905,  957),
        p( 886,  952),    p( 884,  962),    p( 891,  966),    p( 889,  979),    p( 890,  979),    p( 892,  962),    p( 902,  939),    p( 912,  911),
        p( 873,  950),    p( 883,  940),    p( 883,  955),    p( 892,  956),    p( 894,  949),    p( 882,  949),    p( 883,  938),    p( 887,  926),
    ],
    // king
    [
        p( 152, -103),    p(  57,  -49),    p(  80,  -41),    p(   4,   -9),    p(  26,  -21),    p(   9,  -11),    p(  64,  -22),    p( 220, -105),
        p( -23,   -3),    p( -69,   26),    p( -78,   36),    p( -12,   25),    p( -44,   34),    p( -72,   47),    p( -38,   32),    p(   9,   -1),
        p( -44,    5),    p( -38,   23),    p( -82,   40),    p( -87,   48),    p( -53,   42),    p( -20,   35),    p( -58,   34),    p( -32,   10),
        p( -27,   -1),    p( -91,   22),    p(-106,   39),    p(-128,   49),    p(-127,   46),    p(-107,   38),    p(-113,   28),    p(-100,   15),
        p( -46,   -4),    p(-112,   17),    p(-122,   34),    p(-145,   47),    p(-151,   45),    p(-127,   31),    p(-140,   22),    p(-119,   12),
        p( -37,   -1),    p( -88,   12),    p(-118,   27),    p(-126,   36),    p(-121,   34),    p(-135,   27),    p(-105,   12),    p( -75,    9),
        p(  28,  -10),    p( -70,    7),    p( -83,   15),    p(-103,   25),    p(-109,   25),    p( -94,   16),    p( -62,    0),    p(   4,   -4),
        p(  46,  -42),    p(  43,  -48),    p(  37,  -35),    p( -24,  -15),    p(  29,  -33),    p( -20,  -18),    p(  36,  -43),    p(  63,  -52),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  33,   86),    p(  30,   85),    p(  20,   89),    p(  33,   69),    p(  20,   73),    p(  20,   77),    p( -17,   95),    p(  -8,   92),
        p(  41,  122),    p(  48,  122),    p(  37,   99),    p(  22,   67),    p(  37,   65),    p(  15,   94),    p(   1,  102),    p( -27,  123),
        p(  24,   72),    p(  18,   70),    p(  24,   53),    p(  16,   43),    p(  -1,   44),    p(   7,   57),    p( -11,   74),    p( -10,   77),
        p(   6,   46),    p(  -3,   43),    p( -15,   34),    p(  -8,   23),    p( -17,   29),    p( -11,   37),    p( -19,   54),    p( -11,   50),
        p(   2,   14),    p( -13,   22),    p( -15,   16),    p( -15,    8),    p( -13,   13),    p(  -9,   16),    p( -14,   36),    p(   9,   16),
        p(  -5,   15),    p(  -3,   20),    p( -11,   17),    p(  -9,    5),    p(   4,    1),    p(   5,    7),    p(  11,   19),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-11, -9);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);

const BISHOP_PAIR: PhasedScore = p(24, 58);
const ROOK_OPEN_FILE: PhasedScore = p(16, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(0, 2);
const KING_OPEN_FILE: PhasedScore = p(-56, -1);
const KING_CLOSED_FILE: PhasedScore = p(16, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 7);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 5), p(-2, 6), p(-2, 9), p(4, 6), p(4, 9), p(5, 11), p(10, 11), p(21, 6), ],
    // Closed
    [p(0, 0), p(0, 0), p(13, -29), p(-16, 10), p(-0, 13), p(3, 5), p(2, 10), p(-0, 6), ],
    // SemiOpen
    [p(0, 0), p(-17, 22), p(1, 20), p(1, 14), p(-1, 19), p(3, 14), p(1, 12), p(12, 12), ],
    // SemiClosed
    [p(0, 0), p(10, -10), p(7, 8), p(5, 2), p(8, 6), p(3, 5), p(8, 8), p(2, 5), ],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-7, 7),    /*0b0000*/
    p(-16, 12),  /*0b0001*/
    p(-3, 8),    /*0b0010*/
    p(-10, 15),  /*0b0011*/
    p(-5, 7),    /*0b0100*/
    p(-27, 4),   /*0b0101*/
    p(-14, 7),   /*0b0110*/
    p(-19, -15), /*0b0111*/
    p(5, 10),    /*0b1000*/
    p(-5, 11),   /*0b1001*/
    p(1, 9),     /*0b1010*/
    p(-3, 12),   /*0b1011*/
    p(-2, 7),    /*0b1100*/
    p(-25, 10),  /*0b1101*/
    p(-13, 5),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(1, 18),    /*0b10000*/
    p(4, 12),    /*0b10001*/
    p(21, 13),   /*0b10010*/
    p(-4, 10),   /*0b10011*/
    p(-6, 9),    /*0b10100*/
    p(13, 18),   /*0b10101*/
    p(-22, 4),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(12, 33),   /*0b11000*/
    p(31, 26),   /*0b11001*/
    p(41, 39),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(15, 14),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(14, 10),   /*0b100000*/
    p(4, 15),    /*0b100001*/
    p(25, 3),    /*0b100010*/
    p(6, 2),     /*0b100011*/
    p(-10, 4),   /*0b100100*/
    p(-24, -6),  /*0b100101*/
    p(-26, 17),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(19, 2),    /*0b101000*/
    p(-3, 18),   /*0b101001*/
    p(19, -3),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-7, 6),    /*0b101100*/
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
    p(5, 0),     /*0b111111*/
    p(-21, -10), /*0b00*/
    p(8, -26),   /*0b01*/
    p(36, -14),  /*0b10*/
    p(24, -50),  /*0b11*/
    p(47, -18),  /*0b100*/
    p(-5, -27),  /*0b101*/
    p(74, -49),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(56, -20),  /*0b1000*/
    p(19, -44),  /*0b1001*/
    p(78, -64),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(56, -26),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(15, -6),   /*0b1111*/
    p(16, -10),  /*0b00*/
    p(32, -21),  /*0b01*/
    p(26, -27),  /*0b10*/
    p(24, -53),  /*0b11*/
    p(32, -18),  /*0b100*/
    p(54, -29),  /*0b101*/
    p(23, -34),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(37, -12),  /*0b1000*/
    p(55, -27),  /*0b1001*/
    p(51, -52),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(41, -31),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(22, -54),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 7), p(1, 8), p(7, 20), p(9, 9), p(-5, 19), p(-46, 9)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(37, 8), p(41, 36), p(51, -9), p(37, -39), p(0, 0)];

const OUTPOSTS: [PhasedScore; 2] = [p(17, -1), p(25, -22)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-58, -57),
        p(-35, -19),
        p(-19, 2),
        p(-7, 13),
        p(3, 22),
        p(13, 30),
        p(24, 30),
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
        p(-4, -14),
        p(3, -3),
        p(9, 6),
        p(13, 15),
        p(16, 19),
        p(18, 22),
        p(19, 27),
        p(25, 27),
        p(29, 25),
        p(39, 25),
        p(32, 31),
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
        p(-74, 13),
        p(-65, 27),
        p(-60, 32),
        p(-57, 37),
        p(-58, 43),
        p(-52, 48),
        p(-49, 52),
        p(-46, 54),
        p(-42, 58),
        p(-38, 62),
        p(-33, 63),
        p(-29, 67),
        p(-20, 67),
        p(-8, 64),
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
        p(-34, -36),
        p(-34, 21),
        p(-38, 70),
        p(-33, 87),
        p(-30, 104),
        p(-25, 109),
        p(-20, 120),
        p(-17, 126),
        p(-13, 130),
        p(-9, 132),
        p(-7, 135),
        p(-3, 138),
        p(0, 139),
        p(1, 144),
        p(4, 145),
        p(8, 148),
        p(9, 154),
        p(11, 153),
        p(21, 150),
        p(35, 141),
        p(40, 141),
        p(83, 117),
        p(83, 118),
        p(108, 97),
        p(199, 63),
        p(251, 18),
        p(287, 2),
        p(341, -33),
    ],
    [
        p(-85, 50),
        p(-52, 22),
        p(-26, 11),
        p(1, 4),
        p(28, -2),
        p(48, -10),
        p(71, -10),
        p(92, -17),
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
        p(-11, 12),
        p(-6, -4),
        p(23, 16),
        p(50, -15),
        p(22, -44),
        p(0, 0),
    ],
    [p(-1, 12), p(19, 20), p(-2, 8), p(29, 1), p(28, 55), p(0, 0)],
    [
        p(3, 17),
        p(21, 21),
        p(23, 22),
        p(-5, 11),
        p(42, -4),
        p(0, 0),
    ],
    [p(-0, -1), p(7, 11), p(-1, 29), p(-0, 6), p(2, -17), p(0, 0)],
    [p(79, 33), p(-32, 22), p(0, 21), p(-33, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 5), p(11, 4), p(9, 10), p(15, 5), p(9, 16), p(13, 3)],
    [
        p(-2, -1),
        p(7, 18),
        p(-95, -35),
        p(6, 12),
        p(7, 17),
        p(4, 5),
    ],
    [p(2, 2), p(13, 5), p(9, 11), p(11, 7), p(12, 15), p(22, -6)],
    [
        p(3, -5),
        p(10, -2),
        p(8, -8),
        p(4, 15),
        p(-56, -262),
        p(7, -11),
    ],
    [
        p(61, -8),
        p(38, -1),
        p(43, -6),
        p(22, -3),
        p(34, -18),
        p(0, 0),
    ],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [
    p(-13, -10),
    p(17, -9),
    p(17, -2),
    p(23, -13),
    p(6, 22),
    p(7, 19),
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
        OUTPOSTS[piece as usize - Knight as usize]
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
