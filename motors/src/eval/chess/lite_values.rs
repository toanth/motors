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

#[rustfmt::skip]const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
        p( 133,  180),    p( 133,  179),    p( 124,  182),    p( 137,  163),    p( 128,  166),    p( 128,  170),    p(  93,  187),    p(  89,  187),
        p(  73,  118),    p(  72,  121),    p(  86,  108),    p(  95,  112),    p( 103,  106),    p( 140,   96),    p( 134,  118),    p( 102,  112),
        p(  55,  108),    p(  68,  102),    p(  63,   94),    p(  66,   87),    p(  86,   87),    p(  86,   86),    p(  81,   97),    p(  73,   90),
        p(  48,   95),    p(  60,   97),    p(  63,   86),    p(  73,   83),    p(  75,   84),    p(  77,   80),    p(  72,   88),    p(  58,   80),
        p(  42,   90),    p(  54,   88),    p(  54,   83),    p(  56,   91),    p(  66,   87),    p(  60,   83),    p(  71,   79),    p(  51,   78),
        p(  52,   96),    p(  63,   96),    p(  59,   90),    p(  56,   97),    p(  65,  101),    p(  75,   92),    p(  88,   84),    p(  55,   83),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 158,  268),    p( 196,  300),    p( 243,  312),    p( 274,  300),    p( 308,  301),    p( 215,  298),    p( 222,  298),    p( 198,  246),
        p( 266,  300),    p( 282,  312),    p( 300,  312),    p( 321,  312),    p( 309,  308),    p( 341,  297),    p( 280,  308),    p( 289,  290),
        p( 285,  302),    p( 307,  309),    p( 329,  322),    p( 341,  321),    p( 360,  314),    p( 378,  306),    p( 327,  303),    p( 314,  295),
        p( 298,  310),    p( 312,  314),    p( 326,  327),    p( 352,  329),    p( 339,  326),    p( 352,  325),    p( 327,  311),    p( 334,  301),
        p( 290,  314),    p( 299,  312),    p( 314,  329),    p( 320,  331),    p( 328,  334),    p( 323,  319),    p( 328,  308),    p( 304,  309),
        p( 269,  299),    p( 288,  304),    p( 299,  311),    p( 307,  325),    p( 319,  319),    p( 308,  304),    p( 308,  296),    p( 290,  303),
        p( 263,  301),    p( 276,  309),    p( 286,  307),    p( 304,  309),    p( 306,  307),    p( 297,  303),    p( 292,  301),    p( 289,  308),
        p( 233,  296),    p( 275,  295),    p( 264,  303),    p( 283,  309),    p( 291,  306),    p( 289,  297),    p( 279,  298),    p( 254,  294),
    ],
    // bishop
    [
        p( 272,  318),    p( 248,  321),    p( 251,  315),    p( 229,  322),    p( 238,  319),    p( 231,  316),    p( 287,  310),    p( 254,  310),
        p( 282,  308),    p( 290,  313),    p( 293,  313),    p( 284,  317),    p( 296,  311),    p( 310,  309),    p( 288,  313),    p( 283,  308),
        p( 297,  318),    p( 312,  311),    p( 305,  318),    p( 319,  310),    p( 324,  313),    p( 351,  314),    p( 336,  309),    p( 324,  317),
        p( 288,  316),    p( 306,  320),    p( 314,  316),    p( 335,  320),    p( 326,  316),    p( 323,  318),    p( 306,  320),    p( 299,  315),
        p( 295,  314),    p( 293,  320),    p( 310,  321),    p( 330,  318),    p( 327,  317),    p( 304,  319),    p( 300,  317),    p( 309,  307),
        p( 294,  315),    p( 313,  315),    p( 312,  317),    p( 316,  320),    p( 315,  323),    p( 313,  315),    p( 310,  309),    p( 308,  309),
        p( 309,  315),    p( 312,  307),    p( 320,  307),    p( 305,  318),    p( 315,  315),    p( 316,  309),    p( 326,  306),    p( 309,  302),
        p( 295,  307),    p( 319,  311),    p( 301,  314),    p( 291,  317),    p( 298,  315),    p( 294,  320),    p( 306,  301),    p( 302,  296),
    ],
    // rook
    [
        p( 456,  551),    p( 448,  560),    p( 451,  564),    p( 453,  560),    p( 462,  557),    p( 481,  552),    p( 488,  551),    p( 497,  545),
        p( 437,  554),    p( 437,  559),    p( 451,  558),    p( 469,  548),    p( 459,  549),    p( 479,  545),    p( 485,  542),    p( 495,  534),
        p( 432,  554),    p( 454,  549),    p( 452,  550),    p( 456,  545),    p( 483,  535),    p( 499,  528),    p( 526,  524),    p( 492,  527),
        p( 429,  554),    p( 440,  550),    p( 441,  553),    p( 449,  547),    p( 457,  538),    p( 470,  531),    p( 479,  531),    p( 472,  528),
        p( 426,  550),    p( 428,  549),    p( 431,  550),    p( 442,  546),    p( 445,  542),    p( 443,  539),    p( 463,  531),    p( 451,  529),
        p( 427,  545),    p( 429,  544),    p( 434,  543),    p( 441,  542),    p( 448,  535),    p( 456,  527),    p( 480,  512),    p( 458,  517),
        p( 431,  540),    p( 437,  540),    p( 446,  541),    p( 451,  537),    p( 458,  530),    p( 462,  524),    p( 476,  516),    p( 442,  526),
        p( 451,  544),    p( 450,  538),    p( 452,  544),    p( 460,  537),    p( 468,  530),    p( 466,  533),    p( 465,  526),    p( 457,  529),
    ],
    // queen
    [
        p( 854,  988),    p( 850, 1006),    p( 868, 1019),    p( 891, 1012),    p( 890, 1017),    p( 914, 1003),    p( 960,  952),    p( 907,  984),
        p( 874,  972),    p( 855, 1000),    p( 859, 1027),    p( 851, 1046),    p( 859, 1058),    p( 903, 1020),    p( 900,  999),    p( 937,  983),
        p( 882,  971),    p( 876,  987),    p( 880, 1009),    p( 883, 1022),    p( 903, 1030),    p( 944, 1012),    p( 948,  983),    p( 934,  992),
        p( 872,  979),    p( 879,  989),    p( 879, 1002),    p( 882, 1017),    p( 886, 1030),    p( 894, 1026),    p( 899, 1022),    p( 906, 1004),
        p( 880,  972),    p( 874,  993),    p( 884,  993),    p( 889, 1008),    p( 889, 1004),    p( 890, 1004),    p( 898,  998),    p( 901,  994),
        p( 880,  960),    p( 890,  971),    p( 891,  981),    p( 890,  983),    p( 896,  982),    p( 901,  977),    p( 910,  963),    p( 903,  959),
        p( 884,  957),    p( 889,  957),    p( 896,  957),    p( 901,  958),    p( 901,  962),    p( 904,  937),    p( 910,  918),    p( 911,  905),
        p( 881,  949),    p( 885,  947),    p( 888,  954),    p( 896,  958),    p( 897,  944),    p( 882,  942),    p( 883,  929),    p( 883,  928),
    ],
    // king
    [
        p(  74,  -97),    p(  37,  -49),    p(  56,  -40),    p( -24,   -7),    p(   1,  -20),    p(  -6,  -12),    p(  43,  -20),    p( 155, -102),
        p( -23,   -7),    p(   9,   22),    p( -12,   36),    p(  55,   25),    p(  28,   32),    p(  -5,   46),    p(  35,   29),    p(  16,   -4),
        p( -49,    4),    p(  37,   20),    p( -12,   40),    p( -24,   49),    p(  14,   42),    p(  44,   35),    p(  20,   30),    p( -31,   11),
        p( -33,    0),    p( -23,   21),    p( -52,   43),    p( -71,   52),    p( -75,   51),    p( -58,   42),    p( -60,   30),    p(-105,   18),
        p( -52,   -1),    p( -44,   18),    p( -68,   38),    p( -90,   53),    p( -98,   51),    p( -76,   36),    p( -76,   24),    p(-119,   17),
        p( -35,    2),    p(  -7,   11),    p( -53,   30),    p( -67,   41),    p( -64,   39),    p( -62,   29),    p( -28,   12),    p( -66,   13),
        p(  25,   -6),    p(  10,    5),    p(  -9,   16),    p( -32,   26),    p( -34,   25),    p( -22,   17),    p(  22,   -3),    p(   7,   -1),
        p( -24,  -34),    p(  31,  -47),    p(  20,  -32),    p( -48,  -10),    p(   8,  -32),    p( -45,  -13),    p(  19,  -41),    p(   6,  -47),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  33,   80),    p(  33,   79),    p(  24,   82),    p(  37,   63),    p(  28,   66),    p(  28,   70),    p(  -7,   87),    p( -11,   87),
        p(  27,  116),    p(  40,  114),    p(  28,   98),    p(  15,   66),    p(  16,   69),    p(   7,   94),    p( -19,  100),    p( -40,  120),
        p(  13,   67),    p(  11,   66),    p(  19,   52),    p(  13,   43),    p(  -4,   44),    p(   8,   54),    p( -11,   70),    p( -15,   72),
        p(  -1,   40),    p( -10,   39),    p( -18,   33),    p( -11,   25),    p( -19,   28),    p( -14,   35),    p( -23,   49),    p( -16,   46),
        p(  -7,   11),    p( -19,   19),    p( -20,   18),    p( -15,    7),    p( -16,   13),    p( -12,   16),    p( -17,   32),    p(   4,   13),
        p( -15,   11),    p(  -9,   14),    p( -14,   17),    p( -10,    4),    p(   2,   -1),    p(   1,    5),    p(   4,   13),    p(  -0,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];

const BISHOP_PAIR: PhasedScore = p(22, 54);
const ROOK_OPEN_FILE: PhasedScore = p(22, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-14, -4);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, -1);
const KING_OPEN_FILE: PhasedScore = p(-59, -3);
const KING_CLOSED_FILE: PhasedScore = p(16, -17);
const KING_SEMIOPEN_FILE: PhasedScore = p(-13, 3);
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-40, 11),  /*0b0000*/
    p(-24, 9),   /*0b0001*/
    p(-14, 4),   /*0b0010*/
    p(8, 20),    /*0b0011*/
    p(-11, 4),   /*0b0100*/
    p(-16, -5),  /*0b0101*/
    p(3, 11),    /*0b0110*/
    p(23, -10),  /*0b0111*/
    p(-26, 13),  /*0b1000*/
    p(-25, -15), /*0b1001*/
    p(-13, 7),   /*0b1010*/
    p(14, -8),   /*0b1011*/
    p(-10, 2),   /*0b1100*/
    p(-21, -21), /*0b1101*/
    p(11, 9),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-36, 21),  /*0b10000*/
    p(-4, 9),    /*0b10001*/
    p(-14, -14), /*0b10010*/
    p(7, -9),    /*0b10011*/
    p(-12, 4),   /*0b10100*/
    p(24, 7),    /*0b10101*/
    p(-9, -18),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(-14, 49),  /*0b11000*/
    p(17, 9),    /*0b11001*/
    p(22, 22),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(11, 16),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(-14, 12),  /*0b100000*/
    p(-8, 9),    /*0b100001*/
    p(6, 0),     /*0b100010*/
    p(22, 7),    /*0b100011*/
    p(-31, -25), /*0b100100*/
    p(-23, -39), /*0b100101*/
    p(-13, -2),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(-15, 4),   /*0b101000*/
    p(-27, -6),  /*0b101001*/
    p(6, -6),    /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-30, -23), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(-4, 34),   /*0b110000*/
    p(27, 18),   /*0b110001*/
    p(12, -12),  /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(-3, 5),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(3, 40),    /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(15, -18),  /*0b111111*/
    p(-67, -3),  /*0b00*/
    p(-12, -25), /*0b01*/
    p(12, -12),  /*0b10*/
    p(39, -42),  /*0b11*/
    p(4, -15),   /*0b100*/
    p(-38, -47), /*0b101*/
    p(50, -49),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(19, -16),  /*0b1000*/
    p(-1, -43),  /*0b1001*/
    p(48, -89),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(35, -14),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(12, -18),  /*0b1111*/
    p(-34, -4),  /*0b00*/
    p(-1, -20),  /*0b01*/
    p(-2, -26),  /*0b10*/
    p(23, -43),  /*0b11*/
    p(-16, -14), /*0b100*/
    p(13, -56),  /*0b101*/
    p(-5, -34),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(-15, -8),  /*0b1000*/
    p(19, -27),  /*0b1001*/
    p(9, -79),   /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(2, -13),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(10, -74),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(15, 13), p(1, 9), p(3, 12), p(5, 8), p(-7, 21), p(-29, 5)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(41, 10),
    p(48, 34),
    p(55, -7),
    p(43, -34),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-32, -49),
        p(-14, -14),
        p(-3, 5),
        p(3, 14),
        p(9, 21),
        p(15, 27),
        p(22, 25),
        p(28, 22),
        p(33, 15),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-16, -35),
        p(-5, -17),
        p(3, -2),
        p(8, 8),
        p(15, 16),
        p(20, 24),
        p(23, 29),
        p(26, 32),
        p(29, 36),
        p(34, 36),
        p(42, 34),
        p(52, 35),
        p(56, 40),
        p(69, 31),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-62, 14),
        p(-52, 31),
        p(-48, 35),
        p(-45, 38),
        p(-47, 45),
        p(-43, 47),
        p(-41, 51),
        p(-39, 52),
        p(-37, 55),
        p(-35, 58),
        p(-31, 60),
        p(-30, 63),
        p(-24, 62),
        p(-18, 59),
        p(-22, 61),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-17, -57),
        p(-17, -11),
        p(-23, 41),
        p(-20, 61),
        p(-19, 80),
        p(-15, 88),
        p(-13, 102),
        p(-11, 111),
        p(-8, 117),
        p(-6, 120),
        p(-4, 125),
        p(-1, 129),
        p(1, 131),
        p(1, 137),
        p(2, 141),
        p(5, 145),
        p(5, 152),
        p(6, 153),
        p(14, 152),
        p(26, 147),
        p(28, 149),
        p(68, 128),
        p(62, 134),
        p(85, 117),
        p(172, 85),
        p(209, 51),
        p(243, 32),
        p(272, 3),
    ],
    [
        p(12, 34),
        p(16, 12),
        p(11, 4),
        p(5, 1),
        p(0, -2),
        p(-11, -7),
        p(-20, -3),
        p(-33, -10),
        p(-13, -37),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-10, 11),
        p(-4, -4),
        p(24, 15),
        p(47, -14),
        p(20, -42),
        p(0, 0),
    ],
    [p(-0, 15), p(18, 18), p(3, 4), p(29, 1), p(31, 53), p(0, 0)],
    [p(3, 16), p(15, 17), p(18, 18), p(-1, 8), p(39, -5), p(0, 0)],
    [p(-1, 3), p(4, 12), p(-2, 29), p(0, -1), p(0, -19), p(0, 0)],
    [
        p(48, 42),
        p(-40, 24),
        p(-17, 22),
        p(-60, 18),
        p(0, 0),
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
}
