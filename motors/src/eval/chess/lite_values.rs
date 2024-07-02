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
        p( 121,  175),    p( 120,  170),    p( 109,  182),    p( 116,  166),    p( 102,  171),    p( 109,  168),    p(  78,  183),    p(  82,  178),
        p(  56,  119),    p(  54,  124),    p(  62,  113),    p(  71,  114),    p(  55,  122),    p(  97,  107),    p(  81,  128),    p(  67,  117),
        p(  44,  108),    p(  54,  103),    p(  49,   95),    p(  50,   88),    p(  67,   90),    p(  67,   88),    p(  64,   98),    p(  58,   90),
        p(  39,   94),    p(  48,   98),    p(  51,   86),    p(  59,   85),    p(  61,   86),    p(  64,   81),    p(  59,   88),    p(  47,   80),
        p(  34,   90),    p(  44,   87),    p(  43,   84),    p(  47,   90),    p(  58,   88),    p(  47,   85),    p(  59,   79),    p(  40,   78),
        p(  44,   96),    p(  55,   96),    p(  51,   89),    p(  49,   95),    p(  56,  100),    p(  64,   93),    p(  74,   85),    p(  43,   83),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 193,  262),    p( 203,  294),    p( 228,  306),    p( 243,  299),    p( 268,  304),    p( 200,  296),    p( 223,  291),    p( 221,  241),
        p( 261,  298),    p( 273,  305),    p( 282,  303),    p( 294,  306),    p( 294,  299),    p( 318,  290),    p( 272,  302),    p( 291,  285),
        p( 271,  297),    p( 283,  299),    p( 301,  309),    p( 309,  310),    p( 324,  304),    p( 340,  298),    p( 306,  294),    p( 304,  292),
        p( 284,  304),    p( 291,  302),    p( 298,  315),    p( 325,  316),    p( 315,  314),    p( 324,  312),    p( 307,  303),    p( 324,  295),
        p( 280,  308),    p( 278,  303),    p( 284,  315),    p( 291,  319),    p( 298,  321),    p( 296,  307),    p( 307,  300),    p( 296,  303),
        p( 255,  295),    p( 256,  298),    p( 264,  299),    p( 269,  313),    p( 276,  309),    p( 263,  294),    p( 279,  290),    p( 273,  299),
        p( 252,  299),    p( 260,  304),    p( 257,  299),    p( 270,  303),    p( 273,  297),    p( 265,  294),    p( 275,  297),    p( 273,  307),
        p( 227,  292),    p( 266,  292),    p( 246,  295),    p( 266,  302),    p( 276,  298),    p( 272,  287),    p( 272,  293),    p( 250,  296),
    ],
    // bishop
    [
        p( 259,  310),    p( 228,  316),    p( 216,  313),    p( 180,  323),    p( 205,  317),    p( 204,  313),    p( 247,  309),    p( 233,  305),
        p( 265,  303),    p( 267,  309),    p( 268,  311),    p( 257,  313),    p( 268,  310),    p( 280,  306),    p( 267,  308),    p( 268,  304),
        p( 273,  314),    p( 286,  307),    p( 276,  315),    p( 289,  309),    p( 292,  312),    p( 315,  313),    p( 307,  306),    p( 300,  313),
        p( 265,  311),    p( 279,  315),    p( 285,  313),    p( 305,  316),    p( 297,  314),    p( 294,  315),    p( 281,  316),    p( 273,  311),
        p( 271,  309),    p( 263,  317),    p( 281,  317),    p( 297,  315),    p( 296,  314),    p( 276,  317),    p( 270,  314),    p( 293,  302),
        p( 272,  312),    p( 286,  312),    p( 281,  315),    p( 285,  317),    p( 286,  319),    p( 280,  314),    p( 286,  307),    p( 285,  307),
        p( 285,  314),    p( 285,  304),    p( 291,  306),    p( 274,  316),    p( 279,  315),    p( 280,  308),    p( 291,  305),    p( 279,  301),
        p( 272,  305),    p( 295,  309),    p( 287,  310),    p( 267,  314),    p( 280,  311),    p( 274,  317),    p( 282,  300),    p( 276,  295),
    ],
    // rook
    [
        p( 419,  540),    p( 405,  549),    p( 402,  557),    p( 400,  553),    p( 418,  547),    p( 426,  546),    p( 433,  544),    p( 448,  536),
        p( 396,  544),    p( 394,  549),    p( 403,  550),    p( 418,  541),    p( 412,  541),    p( 433,  536),    p( 440,  535),    p( 454,  525),
        p( 394,  545),    p( 413,  540),    p( 412,  541),    p( 418,  536),    p( 440,  527),    p( 455,  520),    p( 480,  516),    p( 454,  518),
        p( 392,  545),    p( 400,  541),    p( 400,  544),    p( 406,  538),    p( 418,  529),    p( 431,  523),    p( 438,  524),    p( 437,  519),
        p( 388,  541),    p( 389,  540),    p( 389,  541),    p( 396,  538),    p( 403,  534),    p( 406,  530),    p( 424,  524),    p( 414,  522),
        p( 388,  537),    p( 386,  535),    p( 389,  535),    p( 392,  535),    p( 401,  528),    p( 413,  519),    p( 437,  505),    p( 419,  510),
        p( 390,  532),    p( 393,  532),    p( 399,  532),    p( 402,  530),    p( 411,  523),    p( 426,  513),    p( 438,  507),    p( 407,  516),
        p( 400,  535),    p( 394,  531),    p( 396,  536),    p( 400,  532),    p( 408,  526),    p( 416,  524),    p( 415,  519),    p( 410,  521),
    ],
    // queen
    [
        p( 747,  959),    p( 746,  979),    p( 762,  996),    p( 760, 1009),    p( 784, 1001),    p( 811,  973),    p( 844,  929),    p( 799,  948),
        p( 753,  975),    p( 728, 1004),    p( 726, 1033),    p( 722, 1045),    p( 730, 1060),    p( 771, 1024),    p( 773,  999),    p( 813,  979),
        p( 756,  974),    p( 747,  993),    p( 748, 1015),    p( 749, 1030),    p( 767, 1039),    p( 813, 1018),    p( 817,  990),    p( 805, 1006),
        p( 741,  988),    p( 743, 1002),    p( 741, 1013),    p( 739, 1034),    p( 742, 1048),    p( 756, 1039),    p( 765, 1038),    p( 777, 1016),
        p( 749,  981),    p( 737, 1004),    p( 739, 1013),    p( 740, 1031),    p( 742, 1027),    p( 743, 1031),    p( 758, 1019),    p( 768, 1011),
        p( 745,  969),    p( 747,  992),    p( 739, 1006),    p( 737, 1011),    p( 740, 1014),    p( 749, 1009),    p( 764,  995),    p( 767,  982),
        p( 747,  970),    p( 744,  978),    p( 749,  983),    p( 746,  993),    p( 747,  993),    p( 748,  982),    p( 761,  959),    p( 773,  933),
        p( 731,  966),    p( 742,  958),    p( 740,  970),    p( 749,  972),    p( 750,  966),    p( 737,  969),    p( 742,  956),    p( 745,  947),
    ],
    // king
    [
        p( 113, -116),    p(  22,  -57),    p(  51,  -49),    p(   9,  -26),    p(  30,  -35),    p(  23,  -25),    p(  62,  -30),    p( 238, -123),
        p( -24,  -12),    p( -39,   29),    p( -59,   39),    p(   4,   28),    p( -11,   33),    p( -45,   46),    p(   5,   34),    p(  21,   -5),
        p( -48,   -1),    p( -16,   25),    p( -71,   44),    p( -61,   49),    p( -28,   41),    p(   2,   35),    p( -24,   34),    p( -26,    7),
        p( -25,   -7),    p( -71,   24),    p( -86,   43),    p(-117,   53),    p(-105,   49),    p( -90,   41),    p( -90,   30),    p( -94,   12),
        p( -39,   -9),    p( -90,   22),    p(-108,   39),    p(-129,   54),    p(-136,   52),    p(-111,   37),    p(-119,   27),    p(-102,   10),
        p( -32,   -1),    p( -68,   20),    p( -97,   34),    p(-106,   44),    p(-102,   43),    p(-114,   35),    p( -83,   20),    p( -64,   10),
        p(  23,   -3),    p( -51,   16),    p( -61,   24),    p( -82,   33),    p( -87,   34),    p( -73,   25),    p( -39,    9),    p(  10,   -2),
        p(  18,  -44),    p(  34,  -53),    p(  32,  -41),    p( -27,  -23),    p(  25,  -40),    p( -26,  -25),    p(  31,  -49),    p(  48,  -60),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  21,   75),    p(  20,   70),    p(   9,   82),    p(  16,   66),    p(   2,   71),    p(   9,   68),    p( -22,   83),    p( -18,   78),
        p(  26,  111),    p(  34,  109),    p(  25,   97),    p(  10,   72),    p(  23,   63),    p(   6,   89),    p(  -7,   91),    p( -33,  113),
        p(   8,   65),    p(   5,   65),    p(  15,   53),    p(  10,   44),    p(  -9,   47),    p(  -0,   55),    p( -20,   70),    p( -19,   71),
        p(  -5,   40),    p( -14,   39),    p( -20,   34),    p( -12,   25),    p( -21,   29),    p( -20,   39),    p( -29,   51),    p( -18,   46),
        p(  -8,   10),    p( -22,   20),    p( -21,   18),    p( -16,    9),    p( -18,   14),    p( -16,   17),    p( -23,   32),    p(   2,   14),
        p( -15,   10),    p( -13,   13),    p( -14,   17),    p( -10,    1),    p(   2,   -0),    p(  -3,    7),    p(   0,   11),    p(  -1,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];

const BISHOP_PAIR: PhasedScore = p(22, 52);
const ROOK_OPEN_FILE: PhasedScore = p(16, 2);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -5);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(-3, -2);
const KING_OPEN_FILE: PhasedScore = p(-57, -4);
const KING_CLOSED_FILE: PhasedScore = p(15, -18);
const KING_SEMIOPEN_FILE: PhasedScore = p(-10, 1);
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-11, 8),   /*0b0000*/
    p(-18, 12),  /*0b0001*/
    p(-11, 8),   /*0b0010*/
    p(-8, 27),   /*0b0011*/
    p(-4, 7),    /*0b0100*/
    p(-29, 2),   /*0b0101*/
    p(-11, 17),  /*0b0110*/
    p(-13, -1),  /*0b0111*/
    p(4, 10),    /*0b1000*/
    p(-22, -10), /*0b1001*/
    p(-4, 9),    /*0b1010*/
    p(-8, 7),    /*0b1011*/
    p(-2, 5),    /*0b1100*/
    p(-36, -16), /*0b1101*/
    p(-4, 16),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-4, 17),   /*0b10000*/
    p(6, 11),    /*0b10001*/
    p(-2, -14),  /*0b10010*/
    p(-5, 0),    /*0b10011*/
    p(-4, 7),    /*0b10100*/
    p(11, 13),   /*0b10101*/
    p(-22, -11), /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(17, 46),   /*0b11000*/
    p(25, 9),    /*0b11001*/
    p(26, 27),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(18, 19),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(15, 10),   /*0b100000*/
    p(-0, 12),   /*0b100001*/
    p(16, 3),    /*0b100010*/
    p(8, 15),    /*0b100011*/
    p(-23, -21), /*0b100100*/
    p(-29, -33), /*0b100101*/
    p(-29, 6),   /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(17, -0),   /*0b101000*/
    p(-19, -5),  /*0b101001*/
    p(17, -5),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-19, -22), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(24, 32),   /*0b110000*/
    p(35, 21),   /*0b110001*/
    p(19, -9),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(1, 9),     /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(33, 34),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(0, -14),   /*0b111111*/
    p(-32, -13), /*0b00*/
    p(7, -36),   /*0b01*/
    p(22, -19),  /*0b10*/
    p(33, -50),  /*0b11*/
    p(41, -30),  /*0b100*/
    p(-6, -73),  /*0b101*/
    p(74, -61),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(58, -32),  /*0b1000*/
    p(24, -60),  /*0b1001*/
    p(51, -88),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(65, -28),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(14, -41),  /*0b1111*/
    p(4, -17),   /*0b00*/
    p(19, -30),  /*0b01*/
    p(17, -37),  /*0b10*/
    p(24, -52),  /*0b11*/
    p(26, -28),  /*0b100*/
    p(30, -66),  /*0b101*/
    p(17, -46),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(27, -22),  /*0b1000*/
    p(44, -38),  /*0b1001*/
    p(37, -90),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(44, -29),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(14, -82),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 12), p(4, 9), p(9, 14), p(8, 9), p(-4, 19), p(-40, 7)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(36, 11), p(42, 38), p(47, 2), p(39, -20), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-86, -62),
        p(-63, -27),
        p(-48, -7),
        p(-37, 5),
        p(-26, 13),
        p(-17, 21),
        p(-6, 21),
        p(4, 20),
        p(13, 13),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-54, -41),
        p(-41, -23),
        p(-32, -8),
        p(-24, 3),
        p(-16, 11),
        p(-10, 19),
        p(-5, 22),
        p(-1, 26),
        p(3, 29),
        p(11, 29),
        p(19, 27),
        p(31, 26),
        p(30, 33),
        p(47, 24),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-112, 0),
        p(-103, 16),
        p(-99, 20),
        p(-96, 24),
        p(-96, 31),
        p(-91, 35),
        p(-88, 40),
        p(-84, 42),
        p(-80, 45),
        p(-75, 49),
        p(-70, 51),
        p(-67, 54),
        p(-58, 54),
        p(-45, 50),
        p(-47, 53),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-239, 36),
        p(-237, 39),
        p(-240, 89),
        p(-235, 106),
        p(-232, 123),
        p(-227, 131),
        p(-223, 142),
        p(-219, 150),
        p(-214, 155),
        p(-210, 158),
        p(-207, 162),
        p(-202, 166),
        p(-198, 165),
        p(-195, 170),
        p(-192, 171),
        p(-187, 172),
        p(-184, 177),
        p(-179, 175),
        p(-167, 169),
        p(-150, 159),
        p(-137, 154),
        p(-85, 122),
        p(-78, 119),
        p(-47, 97),
        p(50, 53),
        p(89, 25),
        p(161, -14),
        p(232, -60),
    ],
    [
        p(-65, 81),
        p(-38, 41),
        p(-20, 24),
        p(-1, 10),
        p(18, -2),
        p(30, -15),
        p(44, -17),
        p(55, -31),
        p(96, -65),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-10, 10),
        p(-6, -3),
        p(22, 15),
        p(43, -10),
        p(25, -48),
        p(0, 0),
    ],
    [p(-1, 14), p(19, 19), p(0, 9), p(26, 4), p(27, 67), p(0, 0)],
    [p(7, 15), p(22, 17), p(25, 19), p(-4, 10), p(43, 4), p(0, 0)],
    [p(-0, 2), p(5, 13), p(-2, 36), p(-5, 18), p(0, -15), p(0, 0)],
    [
        p(61, 39),
        p(-33, 24),
        p(-5, 21),
        p(-43, 16),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(11, 4), p(9, 7), p(16, 4), p(10, 13), p(14, 2)],
    [
        p(1, -6),
        p(11, 13),
        p(-32, -91),
        p(8, 11),
        p(9, 15),
        p(6, 5),
    ],
    [p(2, 1), p(14, 3), p(11, 8), p(12, 7), p(12, 16), p(22, -5)],
    [
        p(4, -4),
        p(11, -3),
        p(10, -8),
        p(5, 14),
        p(14, -305),
        p(9, -16),
    ],
    [
        p(48, -13),
        p(30, -6),
        p(36, -12),
        p(13, -8),
        p(26, -24),
        p(0, 0),
    ],
];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static {
    type Score: ScoreType;

    fn psqt(
        square: ChessSquare,
        piece: UncoloredChessPiece,
        color: Color,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn passed_pawn(square: ChessSquare) -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn bishop_pair() -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn rook_openness(openness: FileOpenness) -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn king_openness(openness: FileOpenness) -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn pawn_shield(config: usize) -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn pawn_protection(
        piece: UncoloredChessPiece,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn pawn_attack(piece: UncoloredChessPiece) -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn mobility(
        piece: UncoloredChessPiece,
        mobility: usize,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn threats(
        attacking: UncoloredChessPiece,
        targeted: UncoloredChessPiece,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;
    fn defended(
        protecting: UncoloredChessPiece,
        target: UncoloredChessPiece,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;
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
