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
        p( 194,  262),    p( 204,  294),    p( 229,  306),    p( 244,  299),    p( 270,  304),    p( 201,  296),    p( 225,  291),    p( 223,  242),
        p( 262,  298),    p( 274,  305),    p( 284,  303),    p( 296,  306),    p( 296,  299),    p( 320,  291),    p( 274,  302),    p( 293,  286),
        p( 273,  297),    p( 284,  299),    p( 302,  309),    p( 311,  310),    p( 326,  304),    p( 342,  298),    p( 308,  295),    p( 305,  292),
        p( 286,  305),    p( 292,  303),    p( 299,  315),    p( 326,  317),    p( 316,  314),    p( 326,  312),    p( 309,  303),    p( 325,  296),
        p( 281,  308),    p( 280,  303),    p( 286,  315),    p( 293,  319),    p( 300,  321),    p( 298,  307),    p( 309,  301),    p( 298,  304),
        p( 256,  295),    p( 258,  298),    p( 265,  299),    p( 270,  313),    p( 278,  310),    p( 265,  294),    p( 281,  290),    p( 275,  299),
        p( 254,  300),    p( 262,  305),    p( 259,  299),    p( 271,  303),    p( 275,  298),    p( 267,  294),    p( 277,  297),    p( 274,  308),
        p( 229,  292),    p( 267,  293),    p( 248,  295),    p( 268,  302),    p( 278,  298),    p( 274,  288),    p( 274,  293),    p( 252,  296),
    ],
    // bishop
    [
        p( 261,  309),    p( 230,  316),    p( 218,  313),    p( 182,  322),    p( 207,  316),    p( 206,  312),    p( 249,  309),    p( 235,  305),
        p( 267,  303),    p( 269,  308),    p( 270,  310),    p( 259,  313),    p( 270,  309),    p( 282,  305),    p( 269,  307),    p( 270,  304),
        p( 275,  313),    p( 288,  306),    p( 278,  315),    p( 291,  309),    p( 294,  312),    p( 317,  313),    p( 309,  306),    p( 302,  313),
        p( 267,  310),    p( 281,  315),    p( 287,  313),    p( 307,  316),    p( 299,  314),    p( 296,  314),    p( 283,  316),    p( 275,  311),
        p( 273,  308),    p( 265,  316),    p( 283,  317),    p( 299,  315),    p( 298,  313),    p( 278,  317),    p( 272,  314),    p( 295,  302),
        p( 274,  311),    p( 288,  312),    p( 283,  315),    p( 287,  316),    p( 288,  318),    p( 282,  314),    p( 288,  306),    p( 287,  306),
        p( 287,  313),    p( 287,  303),    p( 293,  305),    p( 276,  316),    p( 281,  314),    p( 282,  308),    p( 293,  305),    p( 281,  301),
        p( 274,  304),    p( 297,  309),    p( 289,  309),    p( 269,  314),    p( 282,  311),    p( 276,  316),    p( 284,  300),    p( 278,  295),
    ],
    // rook
    [
        p( 423,  539),    p( 410,  548),    p( 406,  555),    p( 405,  552),    p( 422,  546),    p( 430,  545),    p( 437,  543),    p( 453,  535),
        p( 400,  543),    p( 399,  548),    p( 407,  549),    p( 422,  540),    p( 416,  540),    p( 437,  534),    p( 444,  534),    p( 458,  524),
        p( 398,  544),    p( 417,  539),    p( 416,  540),    p( 422,  535),    p( 444,  526),    p( 459,  519),    p( 484,  515),    p( 458,  517),
        p( 396,  544),    p( 404,  540),    p( 405,  543),    p( 410,  537),    p( 422,  528),    p( 435,  522),    p( 443,  522),    p( 441,  518),
        p( 393,  539),    p( 393,  539),    p( 394,  540),    p( 400,  537),    p( 408,  533),    p( 410,  528),    p( 428,  523),    p( 418,  521),
        p( 392,  536),    p( 390,  534),    p( 393,  534),    p( 397,  534),    p( 405,  527),    p( 418,  518),    p( 441,  504),    p( 423,  509),
        p( 395,  531),    p( 397,  531),    p( 403,  531),    p( 407,  529),    p( 415,  522),    p( 430,  512),    p( 442,  505),    p( 411,  515),
        p( 404,  533),    p( 399,  529),    p( 400,  534),    p( 405,  531),    p( 412,  525),    p( 420,  523),    p( 419,  517),    p( 414,  520),
    ],
    // queen
    [
        p( 755,  951),    p( 755,  972),    p( 770,  989),    p( 769, 1001),    p( 792,  994),    p( 819,  965),    p( 852,  921),    p( 807,  941),
        p( 761,  967),    p( 736,  997),    p( 734, 1026),    p( 730, 1037),    p( 738, 1053),    p( 779, 1016),    p( 781,  992),    p( 821,  972),
        p( 764,  967),    p( 755,  985),    p( 756, 1008),    p( 757, 1022),    p( 775, 1032),    p( 821, 1011),    p( 826,  983),    p( 814,  998),
        p( 749,  981),    p( 751,  995),    p( 749, 1006),    p( 747, 1027),    p( 751, 1041),    p( 764, 1031),    p( 773, 1031),    p( 785, 1009),
        p( 757,  973),    p( 745,  997),    p( 747, 1006),    p( 748, 1023),    p( 751, 1019),    p( 751, 1024),    p( 766, 1011),    p( 776, 1004),
        p( 753,  962),    p( 756,  985),    p( 748,  999),    p( 745, 1004),    p( 749, 1007),    p( 758, 1002),    p( 773,  987),    p( 775,  975),
        p( 755,  963),    p( 752,  970),    p( 757,  975),    p( 754,  986),    p( 755,  986),    p( 756,  975),    p( 770,  952),    p( 781,  926),
        p( 739,  958),    p( 750,  951),    p( 749,  963),    p( 757,  964),    p( 758,  958),    p( 745,  962),    p( 751,  948),    p( 753,  940),
    ],
    // king
    [
        p( 113, -118),    p(  21,  -56),    p(  50,  -47),    p(   8,  -25),    p(  28,  -34),    p(  22,  -24),    p(  61,  -29),    p( 234, -124),
        p( -24,  -13),    p( -40,   30),    p( -60,   41),    p(   3,   29),    p( -12,   35),    p( -46,   47),    p(   4,   35),    p(  19,   -6),
        p( -48,   -2),    p( -17,   26),    p( -72,   45),    p( -62,   50),    p( -29,   42),    p(   1,   37),    p( -25,   35),    p( -28,    6),
        p( -25,   -9),    p( -72,   25),    p( -87,   44),    p(-119,   54),    p(-107,   50),    p( -92,   43),    p( -92,   31),    p( -95,   11),
        p( -39,  -11),    p( -92,   23),    p(-109,   41),    p(-130,   55),    p(-137,   53),    p(-113,   38),    p(-120,   28),    p(-104,    8),
        p( -32,   -3),    p( -70,   21),    p( -98,   35),    p(-107,   45),    p(-103,   44),    p(-115,   36),    p( -85,   21),    p( -65,    9),
        p(  24,   -5),    p( -52,   17),    p( -63,   25),    p( -83,   34),    p( -89,   35),    p( -74,   26),    p( -40,   10),    p(   9,   -3),
        p(  19,  -46),    p(  33,  -52),    p(  31,  -40),    p( -29,  -22),    p(  23,  -39),    p( -28,  -24),    p(  30,  -48),    p(  46,  -61),
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
    p(-11, 7),   /*0b0000*/
    p(-17, 11),  /*0b0001*/
    p(-10, 7),   /*0b0010*/
    p(-8, 26),   /*0b0011*/
    p(-4, 6),    /*0b0100*/
    p(-28, 1),   /*0b0101*/
    p(-10, 16),  /*0b0110*/
    p(-12, -2),  /*0b0111*/
    p(5, 9),     /*0b1000*/
    p(-22, -11), /*0b1001*/
    p(-3, 8),    /*0b1010*/
    p(-8, 6),    /*0b1011*/
    p(-2, 4),    /*0b1100*/
    p(-35, -17), /*0b1101*/
    p(-3, 15),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-4, 16),   /*0b10000*/
    p(7, 10),    /*0b10001*/
    p(-1, -15),  /*0b10010*/
    p(-4, -1),   /*0b10011*/
    p(-3, 5),    /*0b10100*/
    p(11, 12),   /*0b10101*/
    p(-21, -12), /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(18, 44),   /*0b11000*/
    p(26, 7),    /*0b11001*/
    p(27, 26),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(18, 18),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(15, 9),    /*0b100000*/
    p(1, 11),    /*0b100001*/
    p(16, 1),    /*0b100010*/
    p(9, 14),    /*0b100011*/
    p(-23, -22), /*0b100100*/
    p(-28, -34), /*0b100101*/
    p(-28, 4),   /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(17, -1),   /*0b101000*/
    p(-18, -6),  /*0b101001*/
    p(17, -6),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-19, -23), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(25, 30),   /*0b110000*/
    p(36, 20),   /*0b110001*/
    p(20, -10),  /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(2, 8),     /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(34, 33),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(1, -15),   /*0b111111*/
    p(-33, -11), /*0b00*/
    p(6, -34),   /*0b01*/
    p(21, -17),  /*0b10*/
    p(32, -48),  /*0b11*/
    p(40, -28),  /*0b100*/
    p(-6, -61),  /*0b101*/
    p(73, -59),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(57, -30),  /*0b1000*/
    p(22, -58),  /*0b1001*/
    p(44, -83),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(63, -26),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(9, -26),   /*0b1111*/
    p(5, -15),   /*0b00*/
    p(20, -29),  /*0b01*/
    p(17, -35),  /*0b10*/
    p(24, -50),  /*0b11*/
    p(27, -27),  /*0b100*/
    p(31, -64),  /*0b101*/
    p(18, -45),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(28, -20),  /*0b1000*/
    p(45, -37),  /*0b1001*/
    p(37, -88),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(45, -28),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(15, -80),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(13, 12), p(4, 9), p(9, 14), p(8, 9), p(-4, 19), p(-40, 7)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(36, 11), p(42, 38), p(47, 2), p(39, -20), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-88, -62),
        p(-65, -28),
        p(-50, -7),
        p(-38, 4),
        p(-28, 13),
        p(-18, 21),
        p(-7, 21),
        p(2, 19),
        p(11, 13),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-56, -40),
        p(-43, -23),
        p(-34, -8),
        p(-26, 4),
        p(-18, 12),
        p(-12, 19),
        p(-7, 23),
        p(-3, 26),
        p(1, 30),
        p(9, 29),
        p(17, 27),
        p(29, 27),
        p(28, 33),
        p(45, 24),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-116, 1),
        p(-107, 17),
        p(-104, 22),
        p(-100, 25),
        p(-100, 32),
        p(-95, 36),
        p(-92, 41),
        p(-88, 43),
        p(-84, 47),
        p(-80, 50),
        p(-74, 52),
        p(-71, 55),
        p(-62, 55),
        p(-50, 51),
        p(-51, 54),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-247, 38),
        p(-245, 46),
        p(-248, 97),
        p(-243, 113),
        p(-240, 130),
        p(-235, 138),
        p(-231, 149),
        p(-227, 157),
        p(-222, 162),
        p(-218, 165),
        p(-215, 170),
        p(-210, 173),
        p(-206, 173),
        p(-204, 178),
        p(-200, 179),
        p(-195, 180),
        p(-192, 184),
        p(-187, 182),
        p(-175, 177),
        p(-158, 166),
        p(-145, 161),
        p(-93, 130),
        p(-86, 126),
        p(-55, 105),
        p(42, 60),
        p(81, 32),
        p(149, -7),
        p(209, -53),
    ],
    [
        p(-61, 81),
        p(-35, 41),
        p(-16, 24),
        p(2, 11),
        p(22, -2),
        p(34, -15),
        p(47, -17),
        p(59, -31),
        p(99, -65),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(1, -6), p(11, 13), p(-9, -55), p(8, 11), p(9, 15), p(6, 5)],
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
