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
use gears::games::chess::pieces::UncoloredChessPiece::Bishop;
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
        p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),
        p( 121,  174),    p( 119,  168),    p( 102,  181),    p( 108,  165),    p(  95,  170),    p( 103,  169),    p(  74,  182),    p(  79,  177),
        p(  56,  120),    p(  56,  124),    p(  55,  116),    p(  65,  115),    p(  44,  125),    p(  92,  108),    p(  73,  129),    p(  68,  117),
        p(  45,  108),    p(  56,  103),    p(  51,   95),    p(  49,   88),    p(  66,   91),    p(  65,   89),    p(  65,   98),    p(  57,   90),
        p(  35,   97),    p(  49,   98),    p(  51,   86),    p(  60,   86),    p(  61,   86),    p(  63,   82),    p(  58,   89),    p(  43,   81),
        p(  31,   92),    p(  44,   87),    p(  42,   83),    p(  49,   87),    p(  58,   87),    p(  44,   84),    p(  58,   78),    p(  38,   79),
        p(  37,  100),    p(  52,   97),    p(  50,   89),    p(  46,   97),    p(  54,  101),    p(  59,   91),    p(  69,   85),    p(  35,   86),
        p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),    p(  96,   96),
    ],
    // knight
    [
        p( 179,  260),    p( 196,  289),    p( 220,  301),    p( 234,  295),    p( 263,  299),    p( 193,  291),    p( 219,  286),    p( 215,  237),
        p( 257,  292),    p( 264,  301),    p( 270,  300),    p( 283,  302),    p( 284,  295),    p( 304,  290),    p( 263,  298),    p( 287,  280),
        p( 266,  292),    p( 276,  295),    p( 290,  305),    p( 297,  307),    p( 310,  301),    p( 330,  294),    p( 295,  292),    p( 297,  287),
        p( 279,  299),    p( 284,  298),    p( 290,  311),    p( 316,  312),    p( 305,  310),    p( 317,  308),    p( 302,  298),    p( 318,  291),
        p( 273,  303),    p( 271,  298),    p( 279,  309),    p( 287,  313),    p( 294,  315),    p( 290,  302),    p( 300,  296),    p( 289,  299),
        p( 247,  290),    p( 250,  292),    p( 259,  293),    p( 267,  306),    p( 275,  303),    p( 260,  287),    p( 273,  284),    p( 266,  294),
        p( 243,  296),    p( 253,  299),    p( 253,  293),    p( 265,  299),    p( 268,  293),    p( 266,  285),    p( 270,  292),    p( 264,  304),
        p( 221,  288),    p( 257,  287),    p( 237,  291),    p( 259,  296),    p( 269,  292),    p( 263,  285),    p( 266,  287),    p( 244,  291),
    ],
    // bishop
    [
        p( 222,  295),    p( 206,  283),    p( 194,  303),    p( 170,  293),    p( 176,  303),    p( 184,  296),    p( 205,  294),    p( 198,  282),
        p( 276,  268),    p( 280,  269),    p( 287,  269),    p( 275,  282),    p( 284,  279),    p( 302,  268),    p( 242,  279),    p( 293,  268),
        p( 273,  287),    p( 298,  268),    p( 295,  279),    p( 309,  271),    p( 320,  273),    p( 307,  280),    p( 311,  274),    p( 295,  285),
        p( 262,  290),    p( 285,  306),    p( 296,  291),    p( 300,  306),    p( 295,  285),    p( 297,  298),    p( 286,  289),    p( 263,  294),
        p( 287,  285),    p( 263,  298),    p( 287,  294),    p( 282,  290),    p( 287,  295),    p( 272,  279),    p( 269,  292),    p( 292,  258),
        p( 271,  282),    p( 284,  296),    p( 276,  300),    p( 284,  304),    p( 268,  298),    p( 275,  296),    p( 262,  279),    p( 282,  277),
        p( 278,  294),    p( 292,  281),    p( 309,  281),    p( 276,  282),    p( 285,  292),    p( 291,  283),    p( 286,  287),    p( 265,  279),
        p( 260,  272),    p( 292,  285),    p( 277,  292),    p( 270,  292),    p( 262,  284),    p( 266,  300),    p( 259,  270),    p( 268,  265),
    ],
    // rook
    [
        p( 481,  460),    p( 440,  492),    p( 444,  498),    p( 446,  490),    p( 461,  485),    p( 467,  507),    p( 439,  482),    p( 503,  446),
        p( 422,  518),    p( 415,  530),    p( 407,  528),    p( 407,  518),    p( 392,  524),    p( 409,  509),    p( 418,  534),    p( 420,  527),
        p( 386,  530),    p( 402,  520),    p( 397,  516),    p( 402,  513),    p( 420,  501),    p( 429,  508),    p( 432,  507),    p( 398,  524),
        p( 393,  526),    p( 394,  518),    p( 398,  511),    p( 403,  505),    p( 406,  499),    p( 413,  504),    p( 407,  506),    p( 411,  509),
        p( 397,  519),    p( 382,  520),    p( 382,  515),    p( 389,  512),    p( 389,  508),    p( 385,  515),    p( 396,  509),    p( 396,  510),
        p( 387,  514),    p( 387,  514),    p( 387,  510),    p( 382,  510),    p( 388,  503),    p( 408,  495),    p( 426,  487),    p( 387,  499),
        p( 400,  505),    p( 382,  510),    p( 394,  504),    p( 398,  499),    p( 402,  495),    p( 412,  503),    p( 408,  488),    p( 409,  491),
        p( 402,  507),    p( 397,  507),    p( 391,  511),    p( 394,  508),    p( 404,  500),    p( 412,  499),    p( 401,  498),    p( 405,  497),
    ],
    // queen
    [
        p( 772,  741),    p( 767,  764),    p( 787,  755),    p( 774,  786),    p( 785,  795),    p( 830,  735),    p( 841,  690),    p( 830,  657),
        p( 776,  803),    p( 748,  831),    p( 758,  828),    p( 766,  797),    p( 777,  766),    p( 789,  740),    p( 729,  794),    p( 771,  811),
        p( 779,  776),    p( 771,  782),    p( 772,  793),    p( 757,  815),    p( 772,  843),    p( 790,  827),    p( 843,  715),    p( 805,  759),
        p( 752,  820),    p( 757,  819),    p( 743,  842),    p( 742,  838),    p( 735,  839),    p( 777,  777),    p( 763,  819),    p( 777,  796),
        p( 758,  818),    p( 735,  843),    p( 744,  835),    p( 730,  853),    p( 744,  825),    p( 741,  810),    p( 768,  781),    p( 765,  791),
        p( 750,  807),    p( 752,  809),    p( 753,  807),    p( 745,  809),    p( 755,  786),    p( 746,  816),    p( 764,  798),    p( 764,  769),
        p( 766,  756),    p( 770,  748),    p( 758,  773),    p( 753,  794),    p( 757,  788),    p( 767,  785),    p( 782,  741),    p( 783,  716),
        p( 741,  778),    p( 761,  765),    p( 750,  780),    p( 769,  754),    p( 761,  751),    p( 752,  763),    p( 744,  774),    p( 754,  741),
    ],
    // king
    [
        p( 110, -117),    p(  22,  -53),    p(  55,  -45),    p(  13,  -22),    p(  31,  -31),    p(  24,  -21),    p(  61,  -27),    p( 242, -126),
        p( -21,  -13),    p( -42,   34),    p( -65,   45),    p(  -2,   33),    p( -13,   38),    p( -42,   51),    p(   5,   38),    p(  24,   -8),
        p( -47,   -2),    p( -17,   30),    p( -74,   48),    p( -64,   54),    p( -33,   46),    p(   1,   40),    p( -24,   38),    p( -23,    4),
        p( -23,   -8),    p( -72,   29),    p( -86,   47),    p(-119,   57),    p(-106,   53),    p( -90,   45),    p( -91,   34),    p( -90,   10),
        p( -38,   -9),    p( -93,   26),    p(-110,   44),    p(-128,   58),    p(-136,   55),    p(-113,   41),    p(-119,   31),    p( -97,    7),
        p( -31,   -1),    p( -72,   25),    p( -99,   38),    p(-107,   48),    p(-104,   47),    p(-118,   39),    p( -88,   25),    p( -61,    8),
        p(  22,   -3),    p( -56,   21),    p( -65,   28),    p( -83,   37),    p( -89,   38),    p( -74,   28),    p( -44,   14),    p(   6,   -2),
        p(  21,  -44),    p(  30,  -48),    p(  29,  -37),    p( -33,  -18),    p(  18,  -36),    p( -32,  -20),    p(  25,  -44),    p(  50,  -61),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  25,   78),    p(  23,   72),    p(   6,   85),    p(  12,   69),    p(  -1,   74),    p(   7,   73),    p( -22,   86),    p( -17,   80),
        p(  29,  110),    p(  34,  109),    p(  24,   99),    p(   5,   74),    p(  16,   64),    p(   5,   90),    p( -10,   91),    p( -34,  113),
        p(  10,   64),    p(   7,   65),    p(  14,   53),    p(   6,   45),    p( -10,   47),    p(   1,   55),    p( -21,   71),    p( -21,   72),
        p(  -3,   40),    p( -13,   38),    p( -19,   34),    p( -10,   24),    p( -18,   29),    p( -22,   39),    p( -31,   51),    p( -21,   47),
        p(  -7,   10),    p( -21,   19),    p( -20,   18),    p( -17,   10),    p( -19,   14),    p( -19,   18),    p( -28,   33),    p(  -2,   15),
        p( -13,    9),    p( -10,   13),    p( -14,   17),    p( -13,    2),    p(  -0,   -0),    p( -10,    9),    p(  -5,   12),    p(  -4,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];

const BISHOP_PAIR: PhasedScore = p(23, 51);
const ROOK_OPEN_FILE: PhasedScore = p(17, 0);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -5);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(-4, -0);
const KING_OPEN_FILE: PhasedScore = p(-57, -4);
const KING_CLOSED_FILE: PhasedScore = p(15, -18);
const KING_SEMIOPEN_FILE: PhasedScore = p(-10, 0);
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-10, 7),   /*0b0000*/
    p(-16, 11),  /*0b0001*/
    p(-10, 6),   /*0b0010*/
    p(-8, 26),   /*0b0011*/
    p(-4, 6),    /*0b0100*/
    p(-26, 0),   /*0b0101*/
    p(-13, 17),  /*0b0110*/
    p(-11, -4),  /*0b0111*/
    p(6, 9),     /*0b1000*/
    p(-20, -12), /*0b1001*/
    p(-3, 7),    /*0b1010*/
    p(-14, 8),   /*0b1011*/
    p(-1, 3),    /*0b1100*/
    p(-30, -20), /*0b1101*/
    p(-5, 15),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-3, 15),   /*0b10000*/
    p(7, 9),     /*0b10001*/
    p(-3, -15),  /*0b10010*/
    p(-8, -1),   /*0b10011*/
    p(-4, 6),    /*0b10100*/
    p(14, 9),    /*0b10101*/
    p(-24, -11), /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(19, 44),   /*0b11000*/
    p(26, 6),    /*0b11001*/
    p(24, 26),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(18, 18),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(13, 9),    /*0b100000*/
    p(-0, 10),   /*0b100001*/
    p(13, 2),    /*0b100010*/
    p(7, 13),    /*0b100011*/
    p(-22, -22), /*0b100100*/
    p(-26, -37), /*0b100101*/
    p(-34, 6),   /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(19, -2),   /*0b101000*/
    p(-18, -7),  /*0b101001*/
    p(16, -6),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-16, -25), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(22, 32),   /*0b110000*/
    p(34, 19),   /*0b110001*/
    p(15, -8),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(2, 8),     /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(34, 32),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(3, -16),   /*0b111111*/
    p(-32, -10), /*0b00*/
    p(8, -33),   /*0b01*/
    p(21, -15),  /*0b10*/
    p(32, -45),  /*0b11*/
    p(45, -28),  /*0b100*/
    p(-3, -71),  /*0b101*/
    p(76, -58),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(59, -29),  /*0b1000*/
    p(26, -57),  /*0b1001*/
    p(45, -83),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(65, -27),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(20, -39),  /*0b1111*/
    p(3, -12),   /*0b00*/
    p(19, -26),  /*0b01*/
    p(16, -32),  /*0b10*/
    p(22, -46),  /*0b11*/
    p(25, -24),  /*0b100*/
    p(27, -60),  /*0b101*/
    p(17, -42),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(28, -17),  /*0b1000*/
    p(44, -35),  /*0b1001*/
    p(36, -85),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(43, -24),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(13, -76),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = [
    p(13, 12),
    p(4, 9),
    p(10, 10),
    p(10, 9),
    p(-5, 19),
    p(-40, 7),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(36, 11), p(41, 36), p(48, 1), p(41, -22), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-78, -57),
        p(-55, -22),
        p(-41, -2),
        p(-29, 10),
        p(-19, 18),
        p(-9, 26),
        p(1, 26),
        p(10, 25),
        p(19, 19),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-44, -13),
        p(-30, -2),
        p(-21, 5),
        p(-16, 10),
        p(-10, 11),
        p(-6, 10),
        p(-3, 7),
        p(-1, 3),
        p(1, -0),
        p(7, -9),
        p(11, -14),
        p(20, -23),
        p(15, -21),
        p(24, -36),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-108, 20),
        p(-100, 34),
        p(-97, 37),
        p(-96, 40),
        p(-98, 45),
        p(-96, 46),
        p(-96, 50),
        p(-95, 49),
        p(-94, 51),
        p(-92, 52),
        p(-89, 51),
        p(-89, 53),
        p(-85, 51),
        p(-79, 46),
        p(-88, 49),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-236, 233),
        p(-244, 190),
        p(-255, 195),
        p(-257, 159),
        p(-262, 127),
        p(-265, 87),
        p(-269, 48),
        p(-273, 7),
        p(-276, -37),
        p(-281, -82),
        p(-286, -126),
        p(-289, -171),
        p(-293, -220),
        p(-299, -265),
        p(-303, -313),
        p(-306, -361),
        p(-311, -406),
        p(-315, -457),
        p(-311, -512),
        p(-302, -572),
        p(-296, -626),
        p(-253, -706),
        p(-253, -761),
        p(-229, -832),
        p(-142, -924),
        p(-117, -999),
        p(-46, -1088),
        p(-2, -1175),
    ],
    [
        p(-56, 82),
        p(-33, 39),
        p(-16, 25),
        p(2, 12),
        p(22, -1),
        p(35, -14),
        p(49, -17),
        p(63, -31),
        p(105, -65),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(43, -9),
        p(25, -48),
        p(0, 0),
    ],
    [
        p(-7, 14),
        p(11, 21),
        p(-8, 11),
        p(27, 3),
        p(23, 63),
        p(0, 0),
    ],
    [p(1, 18), p(17, 21), p(20, 22), p(-1, 8), p(36, 9), p(0, 0)],
    [p(-0, 1), p(3, 14), p(-5, 37), p(-6, 18), p(0, -16), p(0, 0)],
    [
        p(60, 39),
        p(-32, 24),
        p(-5, 22),
        p(-41, 16),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 6), p(10, 4), p(9, 7), p(15, 4), p(10, 12), p(13, 3)],
    [p(-2, -14), p(8, 5), p(-26, -97), p(7, 2), p(6, 6), p(2, -3)],
    [p(-1, -0), p(9, 3), p(8, 7), p(10, 7), p(11, 10), p(16, -5)],
    [
        p(-5, -52),
        p(2, -51),
        p(-0, -54),
        p(-2, -36),
        p(11, -249),
        p(5, -66),
    ],
    [
        p(49, -14),
        p(29, -6),
        p(36, -12),
        p(17, -10),
        p(22, -21),
        p(0, 0),
    ],
];
#[rustfmt::skip]
const ATTACKED_SQUARES: [[PhasedScore; NUM_SQUARES]; 3] = [
    // bishop
    [
        p( -17,   16),    p( -22,   16),    p( -15,   20),    p( -19,   15),    p( -23,   16),    p( -12,   11),    p( -33,   18),    p(  10,    6),
        p(  -4,   10),    p(  10,    2),    p(   5,    8),    p(   0,    4),    p(  -2,    9),    p(  14,    3),    p(  12,    4),    p(   7,    6),
        p(   1,    6),    p(   3,    4),    p(  10,   -2),    p(  12,    4),    p(  25,   -2),    p(  11,    7),    p(  23,    3),    p(   9,    6),
        p(  -1,    9),    p(   2,    7),    p(   5,    7),    p(  11,   12),    p(  15,   11),    p(   4,   10),    p(   3,   10),    p(   0,   12),
        p(  -3,    8),    p(   1,    3),    p(   5,    2),    p(   0,    9),    p(  -0,    9),    p(   2,    6),    p(   0,    3),    p(  -3,    6),
        p(  -9,    3),    p( -13,   12),    p(   5,    8),    p(  -3,    7),    p(  -5,   11),    p(   2,    6),    p( -10,   16),    p(  -5,    5),
        p(  -4,    2),    p(  -3,    9),    p(  -3,    7),    p(   6,   -1),    p(   4,    1),    p(   8,   13),    p(  -2,   10),    p(   9,    2),
        p(  -6,   -3),    p(  -3,   -2),    p(  -3,   11),    p(  -3,    6),    p(  -0,    7),    p(  -3,    8),    p(   9,   -9),    p(   7,   -1),
    ],
    // rook
    [
        p(  -8,    1),    p( -18,   14),    p( -13,   17),    p( -11,   16),    p(  -6,   15),    p(  -1,   19),    p( -11,    5),    p(  31,  -17),
        p(  10,   -5),    p(  10,   -0),    p(   2,    3),    p(  -8,    4),    p(  -8,    7),    p(   5,   -9),    p(  21,    0),    p(  24,   -3),
        p(   0,    3),    p(  11,   -3),    p(  11,   -2),    p(  11,    1),    p(  25,   -5),    p(  14,   -3),    p(  22,   -6),    p(  -1,    6),
        p(  -4,    8),    p(   1,    2),    p(   7,   -0),    p(  13,   -1),    p(  10,    1),    p(   7,   -0),    p(   6,    1),    p(   1,    6),
        p(   2,    4),    p(   2,    1),    p(   3,    1),    p(   7,    1),    p(   6,    0),    p(   3,   -1),    p(  10,   -1),    p(   4,    3),
        p(  -3,    5),    p(   5,    1),    p(   5,    3),    p(   3,    3),    p(   3,    2),    p(   9,   -4),    p(  13,   -0),    p(  -9,    5),
        p(   5,    2),    p(   1,    1),    p(   4,    1),    p(   7,   -1),    p(   4,    2),    p(   3,    7),    p(   2,    3),    p(  10,   -0),
        p(  -4,    4),    p(   5,    1),    p(   3,    3),    p(   4,    4),    p(   5,    4),    p(   3,    1),    p(  -2,    5),    p(  -4,    2),
    ],
    // queen
    [
        p(  11,   41),    p(   4,   55),    p(  -4,   65),    p(  -5,   59),    p(  -8,   67),    p(   6,   59),    p(   9,   57),    p(  50,   16),
        p(   6,   48),    p(   5,   59),    p(   3,   51),    p(   2,   54),    p(   7,   42),    p(   5,   39),    p(   2,   62),    p(   2,   67),
        p(   7,   43),    p(   9,   34),    p(  14,   39),    p(  15,   47),    p(  16,   72),    p(  15,   63),    p(  28,   31),    p(  13,   58),
        p(   7,   40),    p(   8,   41),    p(   7,   47),    p(  13,   47),    p(   9,   54),    p(  11,   46),    p(   6,   58),    p(   7,   57),
        p(   8,   44),    p(  11,   45),    p(  10,   43),    p(   8,   44),    p(  10,   43),    p(  11,   40),    p(  10,   48),    p(   9,   49),
        p(   5,   50),    p(  10,   46),    p(  13,   41),    p(   9,   42),    p(  12,   39),    p(   6,   56),    p(  10,   49),    p(   7,   48),
        p(   6,   45),    p(   8,   48),    p(   5,   54),    p(  10,   54),    p(   6,   60),    p(   6,   60),    p(   9,   43),    p(  12,   41),
        p(  -5,   55),    p(   4,   53),    p(   3,   57),    p(   9,   51),    p(   7,   49),    p(   3,   49),    p(  -1,   52),    p(   6,   43),
    ]
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

    fn attacked_squares(
        piece: UncoloredChessPiece,
        square: ChessSquare,
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

    fn attacked_squares(
        piece: UncoloredChessPiece,
        square: ChessSquare,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore {
        ATTACKED_SQUARES[piece as usize - Bishop as usize][square.bb_idx()]
    }
}
