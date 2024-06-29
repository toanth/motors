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
use gears::games::chess::pieces::{UncoloredChessPiece, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::Color;
use gears::games::Color::*;
use gears::score::{p, PhasedScore};
use std::fmt::Debug;

#[rustfmt::skip]
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
        p( 131,  179),    p( 129,  179),    p( 122,  181),    p( 133,  162),    p( 118,  166),    p( 118,  169),    p(  81,  187),    p(  82,  186),
        p(  67,  118),    p(  74,  120),    p(  82,  108),    p(  91,  110),    p(  93,  106),    p( 125,   95),    p( 112,  119),    p(  84,  111),
        p(  56,  106),    p(  71,  101),    p(  67,   93),    p(  66,   86),    p(  85,   86),    p(  86,   84),    p(  81,   95),    p(  71,   88),
        p(  49,   94),    p(  63,   96),    p(  66,   85),    p(  75,   83),    p(  77,   84),    p(  80,   79),    p(  74,   87),    p(  57,   79),
        p(  44,   89),    p(  56,   86),    p(  56,   82),    p(  59,   91),    p(  71,   87),    p(  62,   83),    p(  74,   76),    p(  51,   76),
        p(  52,   95),    p(  65,   95),    p(  61,   89),    p(  55,   97),    p(  66,   99),    p(  77,   91),    p(  88,   83),    p(  53,   83),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 166,  267),    p( 192,  303),    p( 228,  317),    p( 254,  306),    p( 289,  306),    p( 195,  303),    p( 220,  299),    p( 196,  247),
        p( 258,  304),    p( 278,  313),    p( 285,  312),    p( 297,  313),    p( 284,  308),    p( 312,  297),    p( 271,  309),    p( 273,  294),
        p( 280,  302),    p( 292,  307),    p( 311,  315),    p( 319,  316),    p( 330,  310),    p( 341,  303),    p( 292,  305),    p( 295,  298),
        p( 294,  311),    p( 308,  311),    p( 318,  318),    p( 340,  322),    p( 329,  318),    p( 346,  314),    p( 318,  309),    p( 327,  302),
        p( 291,  313),    p( 298,  308),    p( 311,  318),    p( 313,  321),    p( 319,  324),    p( 318,  309),    p( 322,  303),    p( 303,  307),
        p( 270,  298),    p( 280,  304),    p( 293,  302),    p( 298,  315),    p( 305,  313),    p( 293,  297),    p( 299,  294),    p( 286,  301),
        p( 263,  302),    p( 275,  309),    p( 280,  304),    p( 290,  310),    p( 293,  304),    p( 286,  300),    p( 286,  301),    p( 282,  311),
        p( 232,  298),    p( 275,  297),    p( 260,  300),    p( 280,  304),    p( 288,  303),    p( 290,  290),    p( 282,  298),    p( 253,  295),
    ],
    // bishop
    [
        p( 279,  315),    p( 251,  322),    p( 246,  317),    p( 225,  323),    p( 229,  322),    p( 231,  317),    p( 281,  313),    p( 256,  310),
        p( 279,  309),    p( 282,  315),    p( 286,  317),    p( 270,  321),    p( 277,  316),    p( 287,  314),    p( 267,  318),    p( 271,  308),
        p( 287,  318),    p( 304,  314),    p( 293,  322),    p( 300,  317),    p( 299,  321),    p( 324,  321),    p( 313,  314),    p( 303,  318),
        p( 281,  316),    p( 293,  322),    p( 298,  321),    p( 317,  326),    p( 307,  325),    p( 306,  323),    p( 293,  321),    p( 288,  315),
        p( 286,  314),    p( 280,  322),    p( 302,  321),    p( 313,  323),    p( 310,  321),    p( 300,  319),    p( 291,  319),    p( 300,  308),
        p( 289,  313),    p( 305,  315),    p( 305,  318),    p( 305,  318),    p( 308,  322),    p( 306,  315),    p( 307,  307),    p( 305,  306),
        p( 304,  313),    p( 307,  307),    p( 311,  308),    p( 297,  320),    p( 304,  318),    p( 302,  312),    p( 313,  308),    p( 297,  302),
        p( 291,  304),    p( 311,  309),    p( 307,  312),    p( 292,  317),    p( 304,  313),    p( 298,  318),    p( 306,  300),    p( 297,  294),
    ],
    // rook
    [
        p( 453,  553),    p( 437,  564),    p( 438,  569),    p( 435,  566),    p( 446,  561),    p( 460,  558),    p( 475,  553),    p( 489,  547),
        p( 435,  558),    p( 435,  563),    p( 449,  563),    p( 463,  554),    p( 455,  554),    p( 472,  548),    p( 478,  544),    p( 489,  537),
        p( 427,  556),    p( 449,  551),    p( 444,  553),    p( 445,  547),    p( 474,  537),    p( 487,  532),    p( 515,  528),    p( 479,  532),
        p( 427,  556),    p( 435,  552),    p( 436,  555),    p( 441,  550),    p( 450,  539),    p( 461,  535),    p( 466,  534),    p( 464,  530),
        p( 421,  554),    p( 421,  554),    p( 422,  555),    p( 431,  551),    p( 437,  547),    p( 435,  544),    p( 452,  535),    p( 443,  532),
        p( 419,  551),    p( 417,  551),    p( 419,  551),    p( 422,  552),    p( 432,  545),    p( 441,  536),    p( 466,  519),    p( 449,  522),
        p( 420,  546),    p( 424,  547),    p( 427,  550),    p( 429,  548),    p( 438,  540),    p( 453,  529),    p( 463,  522),    p( 434,  530),
        p( 431,  549),    p( 425,  546),    p( 427,  551),    p( 432,  546),    p( 439,  540),    p( 446,  540),    p( 445,  534),    p( 440,  536),
    ],
    // queen
    [
        p( 855,  994),    p( 851, 1015),    p( 867, 1030),    p( 885, 1024),    p( 880, 1026),    p( 897, 1015),    p( 951,  962),    p( 903,  986),
        p( 875,  983),    p( 855, 1019),    p( 861, 1045),    p( 852, 1061),    p( 853, 1073),    p( 888, 1034),    p( 894, 1012),    p( 931,  985),
        p( 880,  982),    p( 877, 1005),    p( 874, 1035),    p( 873, 1047),    p( 889, 1050),    p( 915, 1031),    p( 927,  997),    p( 907,  996),
        p( 870,  995),    p( 876, 1012),    p( 872, 1030),    p( 872, 1049),    p( 875, 1057),    p( 885, 1044),    p( 891, 1036),    p( 899, 1005),
        p( 880,  987),    p( 866, 1020),    p( 874, 1024),    p( 875, 1044),    p( 877, 1039),    p( 881, 1035),    p( 892, 1018),    p( 898, 1000),
        p( 873,  980),    p( 882,  998),    p( 876, 1019),    p( 872, 1021),    p( 878, 1027),    p( 888, 1017),    p( 901,  997),    p( 895,  977),
        p( 872,  978),    p( 876,  987),    p( 882,  990),    p( 880, 1004),    p( 882, 1001),    p( 883,  990),    p( 894,  967),    p( 897,  936),
        p( 859,  972),    p( 866,  967),    p( 868,  982),    p( 879,  980),    p( 877,  978),    p( 866,  976),    p( 869,  963),    p( 870,  947),
    ],
    // king
    [
        p( 158, -101),    p(  52,  -50),    p(  81,  -43),    p(   4,  -10),    p(  31,  -24),    p(  13,  -13),    p(  69,  -21),    p( 232, -105),
        p( -14,  -10),    p( -63,   23),    p( -66,   30),    p(  -1,   20),    p( -29,   28),    p( -64,   42),    p( -36,   30),    p(  25,   -9),
        p( -37,    1),    p( -31,   19),    p( -72,   36),    p( -80,   45),    p( -44,   38),    p( -21,   31),    p( -53,   31),    p( -24,    5),
        p( -18,   -4),    p( -86,   21),    p(-103,   39),    p(-117,   49),    p(-117,   46),    p( -98,   39),    p(-121,   28),    p( -92,   13),
        p( -35,   -6),    p( -96,   17),    p(-109,   36),    p(-132,   50),    p(-132,   48),    p(-107,   33),    p(-118,   23),    p(-108,   13),
        p( -29,   -0),    p( -68,   12),    p( -96,   29),    p(-106,   39),    p(-101,   37),    p(-109,   30),    p( -81,   14),    p( -66,   11),
        p(  26,  -10),    p( -56,    6),    p( -66,   16),    p( -89,   26),    p( -90,   26),    p( -79,   18),    p( -42,   -2),    p(   6,   -7),
        p(  26,  -34),    p(  37,  -47),    p(  33,  -32),    p( -32,  -11),    p(  19,  -30),    p( -26,  -16),    p(  31,  -42),    p(  43,  -44),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  31,   79),    p(  29,   79),    p(  22,   81),    p(  33,   62),    p(  18,   66),    p(  18,   69),    p( -19,   87),    p( -18,   86),
        p(  31,  115),    p(  38,  114),    p(  32,   97),    p(  15,   68),    p(  10,   72),    p(   9,   94),    p( -20,  102),    p( -35,  122),
        p(  12,   67),    p(  11,   65),    p(  19,   52),    p(  13,   43),    p(  -4,   45),    p(   5,   55),    p( -12,   71),    p( -15,   73),
        p(  -1,   41),    p( -10,   39),    p( -17,   33),    p( -10,   24),    p( -17,   27),    p( -15,   36),    p( -20,   49),    p( -14,   46),
        p(  -6,   11),    p( -18,   20),    p( -20,   18),    p( -12,    7),    p( -14,   12),    p( -11,   16),    p( -13,   33),    p(   4,   15),
        p( -15,   11),    p( -11,   14),    p( -16,   18),    p(  -9,    4),    p(   6,   -0),    p(  -1,    7),    p(   5,   13),    p(  -0,   11),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];

const BISHOP_PAIR: PhasedScore = p(22, 57);
const ROOK_OPEN_FILE: PhasedScore = p(17, 6);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(2, 0);
const KING_OPEN_FILE: PhasedScore = p(-59, -2);
const KING_CLOSED_FILE: PhasedScore = p(16, -17);
const KING_SEMIOPEN_FILE: PhasedScore = p(-10, 3);
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-8, 5),    /*0b0000*/
    p(-16, 7),   /*0b0001*/
    p(-8, 5),    /*0b0010*/
    p(-8, 23),   /*0b0011*/
    p(-1, 2),    /*0b0100*/
    p(-30, -3),  /*0b0101*/
    p(-11, 15),  /*0b0110*/
    p(-16, 3),   /*0b0111*/
    p(4, 7),     /*0b1000*/
    p(-18, -16), /*0b1001*/
    p(-5, 8),    /*0b1010*/
    p(-4, -3),   /*0b1011*/
    p(-4, 1),    /*0b1100*/
    p(-39, -17), /*0b1101*/
    p(-5, 14),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-6, 13),   /*0b10000*/
    p(5, 8),     /*0b10001*/
    p(-5, -16),  /*0b10010*/
    p(-10, -4),  /*0b10011*/
    p(-4, 5),    /*0b10100*/
    p(8, 13),    /*0b10101*/
    p(-25, -12), /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(13, 42),   /*0b11000*/
    p(20, 9),    /*0b11001*/
    p(27, 21),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(16, 15),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(15, 6),    /*0b100000*/
    p(-2, 8),    /*0b100001*/
    p(15, 0),    /*0b100010*/
    p(9, 9),     /*0b100011*/
    p(-21, -26), /*0b100100*/
    p(-37, -36), /*0b100101*/
    p(-28, 3),   /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(10, -1),   /*0b101000*/
    p(-24, -7),  /*0b101001*/
    p(12, -6),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-29, -22), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(21, 29),   /*0b110000*/
    p(31, 19),   /*0b110001*/
    p(16, -12),  /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(2, 9),     /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(24, 38),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(-0, -14),  /*0b111111*/
    p(-25, -8),  /*0b00*/
    p(3, -23),   /*0b01*/
    p(27, -14),  /*0b10*/
    p(24, -28),  /*0b11*/
    p(36, -19),  /*0b100*/
    p(-18, -48), /*0b101*/
    p(63, -51),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(54, -21),  /*0b1000*/
    p(16, -41),  /*0b1001*/
    p(53, -88),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(60, -17),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(7, -16),   /*0b1111*/
    p(12, -8),   /*0b00*/
    p(24, -22),  /*0b01*/
    p(20, -22),  /*0b10*/
    p(24, -31),  /*0b11*/
    p(28, -19),  /*0b100*/
    p(33, -56),  /*0b101*/
    p(19, -30),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(31, -12),  /*0b1000*/
    p(47, -30),  /*0b1001*/
    p(40, -77),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(44, -16),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(16, -62),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = [
    p(14, 18),
    p(8, 14),
    p(10, 20),
    p(7, 6),
    p(-2, 13),
    p(-42, 6),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(44, 10),
    p(41, 35),
    p(58, -8),
    p(41, -34),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-37, -37),
        p(-22, -15),
        p(-12, 1),
        p(-6, 10),
        p(-2, 17),
        p(2, 23),
        p(8, 23),
        p(13, 23),
        p(17, 20),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-18, -23),
        p(-7, -7),
        p(-2, 5),
        p(3, 15),
        p(8, 21),
        p(11, 27),
        p(13, 30),
        p(16, 32),
        p(18, 34),
        p(22, 35),
        p(28, 33),
        p(36, 34),
        p(48, 29),
        p(23, 34),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-63, 19),
        p(-58, 34),
        p(-55, 40),
        p(-52, 44),
        p(-53, 51),
        p(-49, 53),
        p(-45, 55),
        p(-41, 55),
        p(-36, 56),
        p(-32, 57),
        p(-27, 56),
        p(-24, 56),
        p(-22, 54),
        p(-22, 54),
        p(-31, 53),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-43, 59),
        p(-34, 97),
        p(-30, 115),
        p(-26, 126),
        p(-25, 135),
        p(-23, 139),
        p(-20, 139),
        p(-17, 138),
        p(-15, 139),
        p(-12, 138),
        p(-8, 136),
        p(-4, 132),
        p(1, 129),
        p(-0, 133),
        p(8, 125),
        p(12, 127),
        p(22, 119),
        p(47, 103),
        p(46, 109),
        p(97, 75),
        p(71, 92),
        p(105, 81),
        p(94, 83),
        p(82, 87),
        p(-349, 305),
        p(-2, 128),
        p(540, -236),
        p(-66, 131),
    ],
    [
        p(-47, -5),
        p(-26, 7),
        p(-10, 10),
        p(8, 8),
        p(32, 3),
        p(53, -6),
        p(69, -6),
        p(89, -15),
        p(145, -47),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-7, 7),
        p(0, 0),
        p(28, 19),
        p(55, -12),
        p(24, -35),
        p(0, 0),
    ],
    [p(2, 9), p(24, 19), p(0, 0), p(41, 0), p(36, 48), p(0, 0)],
    [p(3, 10), p(13, 10), p(21, 8), p(0, 0), p(47, -11), p(0, 0)],
    [p(-0, 2), p(4, 2), p(2, 18), p(5, 2), p(0, 0), p(0, 0)],
    [p(66, 30), p(-29, 14), p(5, 22), p(-15, 8), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 5), p(6, 3), p(4, 9), p(10, 7), p(5, 20), p(8, 4)],
    [
        p(-1, -7),
        p(8, 15),
        p(-26, -27),
        p(5, 12),
        p(6, 19),
        p(4, 2),
    ],
    [p(3, -4), p(12, -1), p(9, 3), p(11, 3), p(11, 11), p(22, -9)],
    [
        p(-0, -7),
        p(7, -6),
        p(6, -14),
        p(3, 12),
        p(-49, -263),
        p(6, -13),
    ],
    [
        p(46, -6),
        p(29, 2),
        p(33, -3),
        p(12, 0),
        p(24, -20),
        p(0, 0),
    ],
];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static {
    type Score: ScoreType;

    fn psqt(square: ChessSquare, piece: UncoloredChessPiece, color: Color) -> Self::Score;
    fn passed_pawn(square: ChessSquare) -> Self::Score;
    fn bishop_pair() -> Self::Score;
    fn rook_openness(openness: FileOpenness) -> Self::Score;
    fn king_openness(openness: FileOpenness) -> Self::Score;
    fn pawn_shield(config: usize) -> Self::Score;
    fn pawn_protection(piece: UncoloredChessPiece) -> Self::Score;
    fn pawn_attack(piece: UncoloredChessPiece) -> Self::Score;
    fn mobility(piece: UncoloredChessPiece, mobility: usize) -> Self::Score;
    fn threats(attacking: UncoloredChessPiece, targeted: UncoloredChessPiece) -> Self::Score;
    fn defended(protecting: UncoloredChessPiece, target: UncoloredChessPiece) -> Self::Score;
}

/// Eval values tuned on a combination of the zurichess dataset and a dataset used by 4ku,
/// created by GCP using his engine Stoofvlees and filtered by cj5716 using Stockfish at depth 9,
/// using my own tuner `pliers`.
#[derive(Debug, Default, Copy, Clone)]
pub struct Lite {}

impl LiteValues for Lite {
    type Score = PhasedScore;

    fn psqt(square: ChessSquare, piece: UncoloredChessPiece, color: Color) -> Self::Score {
        PSQTS[piece as usize][square.flip_if(color == White).bb_idx()]
    }

    fn passed_pawn(square: ChessSquare) -> Self::Score {
        PASSED_PAWNS[square.bb_idx()]
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

    fn pawn_shield(config: usize) -> Self::Score {
        PAWN_SHIELDS[config]
    }

    fn pawn_protection(piece: UncoloredChessPiece) -> Self::Score {
        PAWN_PROTECTION[piece as usize]
    }

    fn pawn_attack(piece: UncoloredChessPiece) -> Self::Score {
        PAWN_ATTACKS[piece as usize]
    }

    fn mobility(piece: UncoloredChessPiece, mobility: usize) -> Self::Score {
        MOBILITY[piece as usize - 1][mobility]
    }

    fn threats(attacking: UncoloredChessPiece, targeted: UncoloredChessPiece) -> Self::Score {
        THREATS[attacking as usize - 1][targeted as usize]
    }
    fn defended(protecting: UncoloredChessPiece, target: UncoloredChessPiece) -> Self::Score {
        DEFENDED[protecting as usize - 1][target as usize]
    }
}
