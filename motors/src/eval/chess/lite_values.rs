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
        p( 120,  173),    p( 118,  168),    p( 107,  180),    p( 114,  164),    p( 100,  169),    p( 107,  166),    p(  76,  181),    p(  80,  176),
        p(  56,  119),    p(  54,  124),    p(  62,  113),    p(  71,  114),    p(  55,  122),    p(  97,  107),    p(  81,  128),    p(  67,  117),
        p(  44,  108),    p(  54,  103),    p(  49,   95),    p(  50,   88),    p(  67,   90),    p(  67,   88),    p(  64,   98),    p(  58,   90),
        p(  39,   94),    p(  48,   98),    p(  51,   86),    p(  59,   85),    p(  61,   86),    p(  64,   81),    p(  59,   88),    p(  47,   80),
        p(  34,   90),    p(  44,   87),    p(  43,   84),    p(  47,   90),    p(  58,   88),    p(  47,   85),    p(  59,   79),    p(  40,   78),
        p(  44,   96),    p(  55,   96),    p(  51,   89),    p(  49,   95),    p(  56,  100),    p(  64,   93),    p(  74,   85),    p(  43,   83),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 187,  255),    p( 197,  287),    p( 222,  299),    p( 237,  292),    p( 262,  297),    p( 194,  289),    p( 217,  284),    p( 216,  234),
        p( 255,  291),    p( 267,  298),    p( 276,  296),    p( 289,  299),    p( 289,  292),    p( 312,  284),    p( 267,  295),    p( 286,  279),
        p( 266,  290),    p( 277,  292),    p( 295,  302),    p( 304,  303),    p( 319,  297),    p( 334,  291),    p( 300,  288),    p( 298,  285),
        p( 279,  297),    p( 285,  296),    p( 292,  308),    p( 319,  310),    p( 309,  307),    p( 318,  305),    p( 302,  296),    p( 318,  289),
        p( 274,  301),    p( 273,  296),    p( 278,  308),    p( 286,  312),    p( 293,  314),    p( 290,  300),    p( 301,  294),    p( 290,  296),
        p( 249,  288),    p( 250,  291),    p( 258,  292),    p( 263,  306),    p( 271,  303),    p( 257,  287),    p( 273,  283),    p( 267,  292),
        p( 247,  293),    p( 254,  297),    p( 251,  292),    p( 264,  296),    p( 267,  290),    p( 260,  287),    p( 269,  290),    p( 267,  301),
        p( 222,  285),    p( 260,  285),    p( 240,  288),    p( 260,  295),    p( 270,  291),    p( 266,  280),    p( 267,  286),    p( 244,  289),
    ],
    // bishop
    [
        p( 255,  302),    p( 223,  309),    p( 211,  306),    p( 175,  316),    p( 200,  310),    p( 199,  306),    p( 242,  302),    p( 229,  298),
        p( 260,  296),    p( 263,  302),    p( 263,  304),    p( 252,  306),    p( 263,  303),    p( 275,  299),    p( 262,  301),    p( 263,  297),
        p( 268,  307),    p( 281,  300),    p( 271,  308),    p( 285,  302),    p( 287,  305),    p( 310,  306),    p( 302,  299),    p( 295,  306),
        p( 260,  304),    p( 274,  308),    p( 280,  306),    p( 300,  309),    p( 292,  307),    p( 289,  307),    p( 277,  309),    p( 268,  304),
        p( 267,  302),    p( 258,  310),    p( 276,  310),    p( 292,  308),    p( 291,  307),    p( 271,  310),    p( 265,  307),    p( 288,  295),
        p( 267,  304),    p( 281,  305),    p( 276,  308),    p( 280,  310),    p( 281,  312),    p( 275,  307),    p( 281,  300),    p( 280,  300),
        p( 281,  307),    p( 281,  297),    p( 286,  299),    p( 269,  309),    p( 274,  308),    p( 276,  301),    p( 287,  298),    p( 274,  294),
        p( 267,  298),    p( 290,  302),    p( 283,  303),    p( 262,  307),    p( 275,  304),    p( 270,  309),    p( 277,  293),    p( 271,  288),
    ],
    // rook
    [
        p( 413,  528),    p( 400,  537),    p( 396,  545),    p( 395,  541),    p( 412,  535),    p( 420,  534),    p( 427,  533),    p( 442,  524),
        p( 390,  533),    p( 389,  537),    p( 397,  538),    p( 412,  529),    p( 406,  529),    p( 427,  524),    p( 434,  524),    p( 448,  513),
        p( 388,  533),    p( 407,  528),    p( 406,  529),    p( 412,  524),    p( 434,  515),    p( 449,  508),    p( 474,  505),    p( 448,  506),
        p( 386,  533),    p( 394,  529),    p( 395,  532),    p( 400,  527),    p( 412,  517),    p( 425,  511),    p( 432,  512),    p( 431,  507),
        p( 383,  529),    p( 383,  528),    p( 384,  530),    p( 390,  526),    p( 398,  522),    p( 400,  518),    p( 418,  512),    p( 408,  510),
        p( 382,  525),    p( 380,  524),    p( 383,  523),    p( 386,  523),    p( 395,  516),    p( 408,  507),    p( 431,  493),    p( 413,  499),
        p( 385,  520),    p( 387,  520),    p( 393,  521),    p( 397,  519),    p( 405,  511),    p( 420,  502),    p( 432,  495),    p( 401,  504),
        p( 394,  523),    p( 389,  519),    p( 390,  524),    p( 394,  520),    p( 402,  514),    p( 410,  512),    p( 409,  507),    p( 404,  509),
    ],
    // queen
    [
        p( 733,  929),    p( 733,  950),    p( 748,  966),    p( 747,  979),    p( 770,  972),    p( 798,  943),    p( 830,  899),    p( 785,  918),
        p( 739,  945),    p( 714,  974),    p( 713, 1003),    p( 708, 1015),    p( 716, 1030),    p( 757,  994),    p( 759,  970),    p( 800,  950),
        p( 743,  944),    p( 734,  963),    p( 734,  985),    p( 736, 1000),    p( 753, 1009),    p( 799,  988),    p( 804,  961),    p( 792,  976),
        p( 727,  958),    p( 729,  973),    p( 727,  983),    p( 725, 1005),    p( 729, 1018),    p( 742, 1009),    p( 751, 1009),    p( 763,  986),
        p( 735,  951),    p( 723,  975),    p( 725,  983),    p( 726, 1001),    p( 729,  997),    p( 729, 1001),    p( 744,  989),    p( 754,  982),
        p( 732,  940),    p( 734,  962),    p( 726,  976),    p( 724,  982),    p( 727,  985),    p( 736,  980),    p( 751,  965),    p( 753,  952),
        p( 733,  941),    p( 731,  948),    p( 735,  953),    p( 733,  963),    p( 733,  963),    p( 734,  953),    p( 748,  929),    p( 760,  904),
        p( 717,  936),    p( 729,  928),    p( 727,  941),    p( 735,  942),    p( 736,  936),    p( 723,  939),    p( 729,  926),    p( 731,  917),
    ],
    // king
    [
        p( 113, -118),    p(  21,  -56),    p(  50,  -47),    p(   8,  -25),    p(  28,  -34),    p(  22,  -24),    p(  61,  -29),    p( 234, -125),
        p( -24,  -13),    p( -40,   30),    p( -60,   41),    p(   3,   29),    p( -12,   35),    p( -46,   47),    p(   4,   35),    p(  19,   -6),
        p( -48,   -2),    p( -17,   26),    p( -72,   45),    p( -62,   50),    p( -29,   42),    p(   1,   37),    p( -25,   35),    p( -28,    5),
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
        p(  23,   77),    p(  22,   72),    p(  11,   84),    p(  18,   68),    p(   4,   73),    p(  11,   70),    p( -20,   85),    p( -16,   80),
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
    p(-10, 6),   /*0b0010*/
    p(-8, 26),   /*0b0011*/
    p(-4, 6),    /*0b0100*/
    p(-28, 1),   /*0b0101*/
    p(-10, 16),  /*0b0110*/
    p(-12, -2),  /*0b0111*/
    p(5, 9),     /*0b1000*/
    p(-22, -11), /*0b1001*/
    p(-3, 7),    /*0b1010*/
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
    p(1, 10),    /*0b100001*/
    p(16, 1),    /*0b100010*/
    p(9, 14),    /*0b100011*/
    p(-23, -22), /*0b100100*/
    p(-28, -35), /*0b100101*/
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
    p(72, -59),  /*0b110*/
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
    p(30, -64),  /*0b101*/
    p(17, -45),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(28, -20),  /*0b1000*/
    p(45, -36),  /*0b1001*/
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
        p(-80, -55),
        p(-58, -20),
        p(-43, 0),
        p(-31, 12),
        p(-21, 20),
        p(-11, 28),
        p(0, 28),
        p(9, 26),
        p(19, 20),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-49, -33),
        p(-36, -16),
        p(-27, -1),
        p(-20, 10),
        p(-11, 18),
        p(-5, 26),
        p(-0, 29),
        p(4, 33),
        p(8, 36),
        p(16, 36),
        p(24, 34),
        p(36, 33),
        p(35, 40),
        p(52, 31),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-106, 12),
        p(-97, 28),
        p(-94, 32),
        p(-90, 36),
        p(-90, 43),
        p(-85, 47),
        p(-82, 51),
        p(-78, 54),
        p(-74, 57),
        p(-70, 61),
        p(-64, 62),
        p(-61, 66),
        p(-52, 65),
        p(-40, 61),
        p(-41, 65),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-225, 58),
        p(-224, 68),
        p(-227, 119),
        p(-222, 136),
        p(-219, 152),
        p(-214, 161),
        p(-209, 171),
        p(-205, 180),
        p(-200, 184),
        p(-197, 187),
        p(-193, 192),
        p(-188, 195),
        p(-185, 195),
        p(-182, 200),
        p(-178, 201),
        p(-173, 202),
        p(-170, 207),
        p(-166, 204),
        p(-153, 199),
        p(-136, 188),
        p(-123, 184),
        p(-71, 152),
        p(-64, 149),
        p(-34, 127),
        p(64, 83),
        p(102, 54),
        p(170, 16),
        p(229, -30),
    ],
    [
        p(-61, 82),
        p(-35, 41),
        p(-17, 24),
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
