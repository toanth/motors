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
use gears::games::chess::pieces::ChessPieceType::Knight;
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
        p( 133,  186),    p( 130,  185),    p( 120,  188),    p( 133,  169),    p( 120,  174),    p( 120,  177),    p(  83,  195),    p(  92,  192),
        p(  65,  123),    p(  62,  124),    p(  75,  120),    p(  83,  124),    p(  67,  125),    p( 118,  110),    p(  91,  132),    p(  88,  122),
        p(  51,  113),    p(  63,  109),    p(  61,  103),    p(  66,   96),    p(  82,   98),    p(  83,   94),    p(  77,  103),    p(  71,   96),
        p(  48,  100),    p(  54,  102),    p(  64,   95),    p(  73,   93),    p(  76,   92),    p(  77,   88),    p(  70,   92),    p(  60,   86),
        p(  43,   97),    p(  50,   94),    p(  55,   94),    p(  59,  100),    p(  67,   97),    p(  61,   93),    p(  69,   84),    p(  54,   85),
        p(  49,   98),    p(  51,   97),    p(  58,   98),    p(  57,  105),    p(  55,  108),    p(  72,   98),    p(  73,   84),    p(  55,   87),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 183,  270),    p( 208,  302),    p( 241,  315),    p( 264,  306),    p( 297,  307),    p( 208,  303),    p( 228,  302),    p( 212,  253),
        p( 275,  304),    p( 286,  312),    p( 297,  310),    p( 311,  313),    p( 301,  310),    p( 322,  299),    p( 283,  309),    p( 288,  296),
        p( 289,  302),    p( 293,  308),    p( 312,  316),    p( 310,  320),    p( 329,  313),    p( 351,  303),    p( 308,  303),    p( 305,  300),
        p( 300,  310),    p( 305,  307),    p( 312,  319),    p( 337,  321),    p( 317,  323),    p( 331,  320),    p( 311,  312),    p( 330,  304),
        p( 298,  312),    p( 298,  308),    p( 305,  320),    p( 310,  323),    p( 318,  325),    p( 317,  313),    p( 328,  305),    p( 316,  309),
        p( 275,  301),    p( 279,  306),    p( 286,  307),    p( 291,  318),    p( 300,  316),    p( 285,  303),    p( 301,  300),    p( 295,  305),
        p( 271,  305),    p( 282,  309),    p( 278,  304),    p( 290,  308),    p( 293,  303),    p( 286,  300),    p( 296,  300),    p( 291,  314),
        p( 244,  300),    p( 282,  299),    p( 267,  301),    p( 286,  306),    p( 297,  303),    p( 292,  293),    p( 290,  299),    p( 268,  299),
    ],
    // bishop
    [
        p( 279,  314),    p( 254,  314),    p( 248,  307),    p( 223,  315),    p( 223,  314),    p( 224,  307),    p( 282,  304),    p( 252,  308),
        p( 279,  303),    p( 282,  306),    p( 285,  307),    p( 280,  308),    p( 282,  303),    p( 292,  303),    p( 269,  308),    p( 273,  304),
        p( 296,  309),    p( 295,  306),    p( 289,  312),    p( 294,  304),    p( 300,  307),    p( 326,  310),    p( 315,  304),    p( 311,  311),
        p( 279,  311),    p( 295,  310),    p( 295,  307),    p( 309,  313),    p( 308,  307),    p( 304,  309),    p( 301,  308),    p( 281,  311),
        p( 290,  308),    p( 280,  311),    p( 299,  309),    p( 313,  308),    p( 312,  308),    p( 299,  306),    p( 292,  309),    p( 310,  300),
        p( 293,  307),    p( 306,  309),    p( 302,  309),    p( 305,  309),    p( 309,  310),    p( 307,  306),    p( 309,  301),    p( 310,  300),
        p( 308,  311),    p( 303,  300),    p( 310,  303),    p( 296,  310),    p( 302,  308),    p( 303,  305),    p( 313,  300),    p( 302,  298),
        p( 295,  304),    p( 315,  308),    p( 306,  307),    p( 290,  311),    p( 304,  309),    p( 295,  313),    p( 305,  297),    p( 301,  295),
    ],
    // rook
    [
        p( 459,  550),    p( 450,  559),    p( 447,  565),    p( 445,  562),    p( 457,  558),    p( 476,  553),    p( 485,  552),    p( 495,  544),
        p( 433,  556),    p( 429,  562),    p( 438,  562),    p( 454,  552),    p( 444,  554),    p( 464,  549),    p( 475,  546),    p( 490,  536),
        p( 438,  553),    p( 456,  548),    p( 454,  550),    p( 457,  545),    p( 485,  534),    p( 494,  530),    p( 516,  527),    p( 488,  529),
        p( 435,  552),    p( 443,  548),    p( 443,  551),    p( 449,  546),    p( 458,  537),    p( 466,  532),    p( 473,  534),    p( 470,  529),
        p( 431,  548),    p( 431,  547),    p( 432,  548),    p( 438,  545),    p( 444,  541),    p( 439,  540),    p( 459,  532),    p( 448,  531),
        p( 428,  544),    p( 427,  542),    p( 430,  541),    p( 432,  542),    p( 440,  535),    p( 448,  528),    p( 471,  515),    p( 453,  520),
        p( 430,  538),    p( 434,  539),    p( 440,  540),    p( 443,  538),    p( 451,  531),    p( 465,  521),    p( 474,  516),    p( 442,  525),
        p( 439,  542),    p( 436,  539),    p( 437,  543),    p( 442,  539),    p( 449,  533),    p( 455,  532),    p( 453,  529),    p( 447,  530),
    ],
    // queen
    [
        p( 874,  968),    p( 877,  982),    p( 892,  995),    p( 908,  992),    p( 907,  995),    p( 927,  982),    p( 978,  930),    p( 923,  962),
        p( 884,  961),    p( 859,  993),    p( 862, 1019),    p( 854, 1037),    p( 861, 1047),    p( 901, 1008),    p( 904,  989),    p( 947,  966),
        p( 892,  965),    p( 884,  985),    p( 884, 1008),    p( 882, 1016),    p( 905, 1018),    p( 944, 1002),    p( 951,  971),    p( 939,  977),
        p( 877,  980),    p( 884,  990),    p( 877, 1000),    p( 876, 1014),    p( 881, 1024),    p( 893, 1014),    p( 903, 1013),    p( 909,  989),
        p( 889,  969),    p( 875,  989),    p( 881,  993),    p( 881, 1010),    p( 882, 1008),    p( 885, 1007),    p( 899,  991),    p( 906,  984),
        p( 884,  954),    p( 890,  972),    p( 883,  989),    p( 880,  992),    p( 885,  999),    p( 891,  989),    p( 906,  970),    p( 906,  957),
        p( 887,  953),    p( 884,  962),    p( 891,  966),    p( 890,  979),    p( 890,  979),    p( 892,  962),    p( 902,  939),    p( 912,  912),
        p( 873,  951),    p( 884,  940),    p( 884,  955),    p( 892,  957),    p( 894,  949),    p( 883,  949),    p( 882,  940),    p( 887,  926),
    ],
    // king
    [
        p( 151, -103),    p(  56,  -49),    p(  80,  -41),    p(   4,   -9),    p(  26,  -21),    p(   9,  -11),    p(  64,  -21),    p( 219, -105),
        p( -23,   -3),    p( -69,   26),    p( -77,   36),    p( -11,   25),    p( -44,   34),    p( -71,   47),    p( -37,   32),    p(   9,   -1),
        p( -44,    5),    p( -37,   22),    p( -81,   40),    p( -86,   48),    p( -53,   42),    p( -20,   35),    p( -58,   33),    p( -32,   10),
        p( -26,   -1),    p( -90,   22),    p(-106,   39),    p(-128,   48),    p(-126,   46),    p(-107,   38),    p(-112,   27),    p(-100,   15),
        p( -46,   -4),    p(-112,   17),    p(-121,   34),    p(-144,   47),    p(-150,   45),    p(-127,   31),    p(-140,   22),    p(-119,   12),
        p( -36,   -1),    p( -87,   12),    p(-117,   27),    p(-125,   36),    p(-120,   34),    p(-134,   27),    p(-104,   12),    p( -75,    9),
        p(  28,  -10),    p( -69,    6),    p( -82,   15),    p(-103,   24),    p(-108,   25),    p( -93,   16),    p( -62,    0),    p(   4,   -4),
        p(  45,  -42),    p(  43,  -47),    p(  38,  -35),    p( -24,  -14),    p(  29,  -33),    p( -20,  -18),    p(  35,  -43),    p(  62,  -52),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  33,   86),    p(  30,   85),    p(  20,   88),    p(  33,   69),    p(  20,   74),    p(  20,   77),    p( -17,   95),    p(  -8,   92),
        p(  40,  123),    p(  47,  122),    p(  37,   99),    p(  21,   67),    p(  36,   66),    p(  14,   94),    p(   0,  102),    p( -27,  123),
        p(  23,   72),    p(  17,   70),    p(  23,   53),    p(  15,   43),    p(  -1,   44),    p(   7,   57),    p( -11,   74),    p( -11,   77),
        p(   6,   46),    p(  -2,   43),    p( -16,   34),    p(  -8,   24),    p( -17,   29),    p( -11,   38),    p( -18,   54),    p( -12,   50),
        p(   2,   14),    p( -12,   22),    p( -15,   16),    p( -16,    8),    p( -14,   13),    p(  -9,   17),    p( -14,   36),    p(   9,   16),
        p(  -5,   15),    p(  -4,   20),    p( -12,   17),    p(  -9,    5),    p(   3,    1),    p(   5,    7),    p(  11,   18),    p(   7,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-10, -10);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);

const BISHOP_PAIR: PhasedScore = p(24, 58);
const ROOK_OPEN_FILE: PhasedScore = p(16, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(0, 2);
const KING_OPEN_FILE: PhasedScore = p(-56, -1);
const KING_CLOSED_FILE: PhasedScore = p(16, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 6);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-7, 5), p(-2, 6), p(-2, 9), p(4, 7), p(4, 9), p(5, 12), p(10, 11), p(22, 6), ],
    // Closed
    [p(0, 0), p(0, 0), p(13, -33), p(-16, 9), p(-1, 13), p(2, 4), p(2, 10), p(-1, 6), ],
    // SemiOpen
    [p(0, 0), p(-16, 22), p(1, 20), p(2, 14), p(-1, 19), p(4, 14), p(1, 11), p(11, 11), ],
    // SemiClosed
    [p(0, 0), p(9, -13), p(7, 6), p(5, 1), p(7, 4), p(3, 4), p(7, 7), p(2, 4), ],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-8, 7),    /*0b0000*/
    p(-16, 12),  /*0b0001*/
    p(-4, 8),    /*0b0010*/
    p(-10, 14),  /*0b0011*/
    p(-6, 7),    /*0b0100*/
    p(-27, 4),   /*0b0101*/
    p(-14, 7),   /*0b0110*/
    p(-19, -16), /*0b0111*/
    p(5, 11),    /*0b1000*/
    p(-4, 11),   /*0b1001*/
    p(1, 9),     /*0b1010*/
    p(-2, 11),   /*0b1011*/
    p(-2, 7),    /*0b1100*/
    p(-23, 10),  /*0b1101*/
    p(-13, 5),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(1, 18),    /*0b10000*/
    p(4, 12),    /*0b10001*/
    p(21, 13),   /*0b10010*/
    p(-3, 10),   /*0b10011*/
    p(-6, 8),    /*0b10100*/
    p(12, 18),   /*0b10101*/
    p(-21, 3),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(11, 33),   /*0b11000*/
    p(31, 26),   /*0b11001*/
    p(40, 40),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(15, 14),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(14, 10),   /*0b100000*/
    p(3, 15),    /*0b100001*/
    p(25, 4),    /*0b100010*/
    p(7, 2),     /*0b100011*/
    p(-9, 3),    /*0b100100*/
    p(-23, -7),  /*0b100101*/
    p(-25, 17),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(19, 2),    /*0b101000*/
    p(-2, 17),   /*0b101001*/
    p(19, -2),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-6, 6),    /*0b101100*/
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
    p(6, -0),    /*0b111111*/
    p(-22, -9),  /*0b00*/
    p(8, -26),   /*0b01*/
    p(36, -14),  /*0b10*/
    p(25, -50),  /*0b11*/
    p(46, -18),  /*0b100*/
    p(-3, -28),  /*0b101*/
    p(74, -49),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(56, -20),  /*0b1000*/
    p(20, -44),  /*0b1001*/
    p(79, -64),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(57, -26),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(18, -6),   /*0b1111*/
    p(16, -10),  /*0b00*/
    p(32, -20),  /*0b01*/
    p(26, -27),  /*0b10*/
    p(24, -53),  /*0b11*/
    p(32, -18),  /*0b100*/
    p(53, -29),  /*0b101*/
    p(23, -33),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(37, -12),  /*0b1000*/
    p(54, -27),  /*0b1001*/
    p(51, -53),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(41, -31),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(23, -54),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 7), p(-0, -2), p(5, 13), p(9, 9), p(-5, 19), p(-46, 9)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(35, 5), p(40, 35), p(51, -9), p(37, -38), p(0, 0)];

const OUTPOSTS: [PhasedScore; 2] = [p(22, 16), p(29, -6)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-58, -57),
        p(-35, -19),
        p(-19, 2),
        p(-8, 13),
        p(3, 22),
        p(13, 30),
        p(24, 29),
        p(33, 28),
        p(41, 24),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-26, -48),
        p(-14, -30),
        p(-4, -14),
        p(3, -3),
        p(9, 6),
        p(13, 14),
        p(16, 18),
        p(18, 22),
        p(19, 26),
        p(26, 26),
        p(30, 25),
        p(40, 25),
        p(34, 33),
        p(48, 25),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-65, 28),
        p(-61, 32),
        p(-57, 37),
        p(-58, 44),
        p(-52, 48),
        p(-49, 52),
        p(-45, 54),
        p(-41, 58),
        p(-37, 62),
        p(-32, 63),
        p(-29, 67),
        p(-19, 67),
        p(-7, 63),
        p(-3, 64),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-34, -33),
        p(-34, 21),
        p(-38, 71),
        p(-32, 88),
        p(-30, 105),
        p(-24, 110),
        p(-20, 120),
        p(-17, 126),
        p(-12, 130),
        p(-9, 132),
        p(-6, 135),
        p(-2, 138),
        p(1, 139),
        p(2, 144),
        p(5, 145),
        p(8, 148),
        p(9, 153),
        p(12, 152),
        p(22, 149),
        p(36, 141),
        p(41, 141),
        p(84, 116),
        p(84, 118),
        p(108, 97),
        p(200, 62),
        p(251, 18),
        p(287, 2),
        p(340, -33),
    ],
    [
        p(-84, 49),
        p(-52, 22),
        p(-26, 11),
        p(1, 4),
        p(28, -2),
        p(47, -10),
        p(70, -10),
        p(91, -17),
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
        p(-10, 13),
        p(-8, -4),
        p(23, 17),
        p(51, -15),
        p(23, -44),
        p(0, 0),
    ],
    [p(-1, 13), p(19, 21), p(-3, 8), p(30, 2), p(27, 56), p(0, 0)],
    [
        p(4, 17),
        p(21, 20),
        p(23, 21),
        p(-5, 11),
        p(42, -4),
        p(0, 0),
    ],
    [p(-0, -1), p(6, 12), p(-1, 29), p(-0, 6), p(2, -17), p(0, 0)],
    [
        p(79, 34),
        p(-33, 21),
        p(-0, 20),
        p(-33, 9),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(10, 4), p(9, 10), p(15, 5), p(9, 16), p(14, 3)],
    [p(-2, 1), p(7, 18), p(-93, -35), p(6, 12), p(7, 17), p(4, 5)],
    [p(2, 2), p(14, 4), p(9, 10), p(11, 7), p(12, 15), p(22, -6)],
    [
        p(3, -5),
        p(10, -2),
        p(8, -8),
        p(4, 15),
        p(-55, -264),
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
    p(18, -10),
    p(16, -3),
    p(23, -13),
    p(6, 23),
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

    fn outpost(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

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

    fn outpost(piece: ChessPieceType) -> PhasedScore {
        OUTPOSTS[piece as usize - Knight as usize]
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
