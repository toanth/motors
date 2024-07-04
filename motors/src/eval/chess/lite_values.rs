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
        p(  97,   97),    p(  97,   97),    p(  97,   97),    p(  97,   97),    p(  97,   97),    p(  97,   97),    p(  97,   97),    p(  97,   97),
        p( 120,  174),    p( 119,  168),    p( 108,  180),    p( 114,  164),    p( 101,  169),    p( 108,  167),    p(  77,  181),    p(  81,  177),
        p(  58,  119),    p(  55,  124),    p(  63,  113),    p(  70,  114),    p(  55,  122),    p(  98,  106),    p(  82,  128),    p(  69,  117),
        p(  43,  108),    p(  54,  104),    p(  50,   95),    p(  52,   88),    p(  68,   90),    p(  68,   88),    p(  64,   98),    p(  58,   90),
        p(  39,   95),    p(  47,   98),    p(  51,   86),    p(  60,   84),    p(  64,   85),    p(  64,   82),    p(  59,   88),    p(  46,   80),
        p(  32,   91),    p(  44,   87),    p(  44,   84),    p(  47,   90),    p(  57,   88),    p(  48,   84),    p(  58,   79),    p(  38,   79),
        p(  44,   96),    p(  53,   97),    p(  50,   89),    p(  48,   96),    p(  54,  101),    p(  63,   93),    p(  73,   85),    p(  43,   83),
        p(  97,   97),    p(  97,   97),    p(  97,   97),    p(  97,   97),    p(  97,   97),    p(  97,   97),    p(  97,   97),    p(  97,   97),
    ],
    // knight
    [
        p( 188,  257),    p( 198,  289),    p( 223,  301),    p( 238,  294),    p( 263,  299),    p( 195,  291),    p( 218,  286),    p( 216,  236),
        p( 256,  293),    p( 268,  300),    p( 277,  298),    p( 290,  301),    p( 290,  294),    p( 314,  285),    p( 267,  296),    p( 286,  280),
        p( 268,  292),    p( 278,  294),    p( 295,  304),    p( 305,  305),    p( 320,  299),    p( 335,  293),    p( 301,  290),    p( 300,  286),
        p( 280,  299),    p( 286,  297),    p( 293,  310),    p( 320,  311),    p( 310,  309),    p( 320,  307),    p( 303,  298),    p( 319,  290),
        p( 275,  303),    p( 274,  298),    p( 280,  310),    p( 287,  314),    p( 294,  316),    p( 291,  302),    p( 302,  295),    p( 292,  298),
        p( 250,  290),    p( 251,  293),    p( 259,  294),    p( 264,  308),    p( 272,  304),    p( 259,  289),    p( 274,  285),    p( 269,  293),
        p( 248,  295),    p( 255,  299),    p( 253,  294),    p( 265,  298),    p( 269,  292),    p( 261,  289),    p( 270,  292),    p( 268,  302),
        p( 222,  287),    p( 261,  287),    p( 242,  290),    p( 262,  296),    p( 272,  292),    p( 268,  282),    p( 268,  288),    p( 245,  290),
    ],
    // bishop
    [
        p( 264,  302),    p( 236,  304),    p( 223,  300),    p( 187,  309),    p( 211,  303),    p( 210,  299),    p( 255,  296),    p( 237,  298),
        p( 269,  291),    p( 270,  297),    p( 269,  298),    p( 264,  297),    p( 276,  293),    p( 281,  292),    p( 268,  296),    p( 271,  293),
        p( 285,  300),    p( 285,  295),    p( 279,  300),    p( 292,  293),    p( 294,  296),    p( 319,  298),    p( 308,  294),    p( 311,  300),
        p( 272,  299),    p( 285,  299),    p( 287,  297),    p( 305,  301),    p( 296,  299),    p( 296,  299),    p( 288,  300),    p( 280,  299),
        p( 278,  297),    p( 270,  302),    p( 285,  300),    p( 296,  299),    p( 296,  298),    p( 279,  300),    p( 277,  299),    p( 298,  291),
        p( 280,  299),    p( 288,  300),    p( 285,  300),    p( 288,  300),    p( 288,  301),    p( 286,  298),    p( 286,  295),    p( 293,  293),
        p( 294,  302),    p( 286,  292),    p( 294,  294),    p( 281,  299),    p( 286,  298),    p( 282,  296),    p( 294,  293),    p( 285,  289),
        p( 280,  296),    p( 297,  301),    p( 289,  297),    p( 274,  302),    p( 286,  299),    p( 277,  304),    p( 283,  292),    p( 285,  287),
    ],
    // rook
    [
        p( 411,  531),    p( 398,  541),    p( 395,  548),    p( 393,  545),    p( 411,  539),    p( 419,  538),    p( 426,  536),    p( 441,  527),
        p( 389,  536),    p( 388,  540),    p( 397,  542),    p( 412,  532),    p( 406,  532),    p( 426,  527),    p( 433,  527),    p( 447,  516),
        p( 388,  536),    p( 406,  532),    p( 405,  533),    p( 411,  527),    p( 433,  518),    p( 448,  511),    p( 474,  508),    p( 448,  509),
        p( 385,  536),    p( 393,  532),    p( 394,  536),    p( 398,  530),    p( 411,  520),    p( 424,  514),    p( 432,  515),    p( 430,  511),
        p( 382,  532),    p( 382,  531),    p( 383,  533),    p( 389,  530),    p( 396,  525),    p( 399,  521),    p( 417,  515),    p( 407,  513),
        p( 381,  528),    p( 379,  527),    p( 382,  526),    p( 385,  527),    p( 394,  520),    p( 407,  510),    p( 430,  497),    p( 412,  502),
        p( 383,  523),    p( 386,  524),    p( 392,  524),    p( 396,  522),    p( 404,  514),    p( 419,  505),    p( 431,  498),    p( 400,  507),
        p( 392,  526),    p( 388,  522),    p( 389,  527),    p( 393,  524),    p( 401,  517),    p( 409,  516),    p( 408,  510),    p( 402,  512),
    ],
    // queen
    [
        p( 736,  936),    p( 736,  957),    p( 752,  973),    p( 750,  986),    p( 774,  978),    p( 801,  950),    p( 833,  906),    p( 787,  926),
        p( 743,  952),    p( 718,  981),    p( 716, 1010),    p( 712, 1022),    p( 720, 1037),    p( 761, 1000),    p( 763,  976),    p( 803,  957),
        p( 748,  951),    p( 737,  970),    p( 738,  993),    p( 739, 1007),    p( 757, 1016),    p( 802,  996),    p( 808,  967),    p( 797,  982),
        p( 731,  965),    p( 733,  980),    p( 730,  990),    p( 728, 1012),    p( 732, 1026),    p( 746, 1016),    p( 755, 1016),    p( 766,  993),
        p( 739,  958),    p( 726,  982),    p( 728,  991),    p( 729, 1008),    p( 732, 1004),    p( 732, 1009),    p( 748,  996),    p( 758,  988),
        p( 735,  946),    p( 737,  970),    p( 729,  984),    p( 727,  989),    p( 730,  992),    p( 739,  987),    p( 754,  972),    p( 757,  959),
        p( 736,  948),    p( 733,  956),    p( 738,  961),    p( 736,  971),    p( 737,  971),    p( 737,  960),    p( 751,  937),    p( 763,  910),
        p( 720,  943),    p( 732,  936),    p( 731,  947),    p( 739,  949),    p( 740,  943),    p( 727,  946),    p( 732,  933),    p( 735,  924),
    ],
    // king
    [
        p( 113, -118),    p(  21,  -56),    p(  50,  -48),    p(   8,  -26),    p(  29,  -35),    p(  22,  -24),    p(  61,  -29),    p( 235, -124),
        p( -23,  -13),    p( -39,   29),    p( -60,   40),    p(   3,   28),    p( -12,   34),    p( -46,   47),    p(   4,   34),    p(  19,   -6),
        p( -47,   -2),    p( -17,   26),    p( -72,   44),    p( -62,   49),    p( -29,   41),    p(   1,   36),    p( -25,   34),    p( -28,    6),
        p( -25,   -9),    p( -72,   25),    p( -87,   43),    p(-119,   54),    p(-107,   49),    p( -92,   42),    p( -91,   30),    p( -96,   11),
        p( -38,  -10),    p( -91,   22),    p(-109,   40),    p(-130,   55),    p(-137,   52),    p(-113,   37),    p(-120,   28),    p(-105,    9),
        p( -32,   -3),    p( -69,   20),    p( -98,   35),    p(-107,   45),    p(-103,   43),    p(-114,   35),    p( -85,   21),    p( -66,    9),
        p(  24,   -5),    p( -52,   16),    p( -61,   24),    p( -82,   33),    p( -87,   34),    p( -73,   25),    p( -40,    9),    p(   8,   -3),
        p(  18,  -45),    p(  33,  -53),    p(  31,  -41),    p( -28,  -23),    p(  24,  -40),    p( -27,  -25),    p(  30,  -49),    p(  46,  -61),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  23,   77),    p(  22,   71),    p(  11,   83),    p(  17,   67),    p(   4,   72),    p(  11,   70),    p( -20,   84),    p( -16,   80),
        p(  26,  111),    p(  34,  109),    p(  26,   97),    p(  10,   72),    p(  23,   63),    p(   7,   89),    p(  -8,   91),    p( -34,  113),
        p(   9,   65),    p(   5,   65),    p(  15,   53),    p(  10,   44),    p(  -8,   47),    p(  -0,   55),    p( -20,   70),    p( -19,   71),
        p(  -5,   40),    p( -14,   39),    p( -20,   34),    p( -12,   25),    p( -21,   30),    p( -20,   38),    p( -29,   51),    p( -17,   46),
        p(  -7,   10),    p( -22,   20),    p( -20,   18),    p( -16,    9),    p( -18,   14),    p( -16,   17),    p( -22,   32),    p(   3,   14),
        p( -15,   10),    p( -13,   13),    p( -13,   17),    p( -10,    1),    p(   2,   -0),    p(  -3,    7),    p(   0,   11),    p(  -2,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];

const BISHOP_PAIR: PhasedScore = p(22, 52);
const ROOK_OPEN_FILE: PhasedScore = p(16, 2);
const ROOK_CLOSED_FILE: PhasedScore = p(-13, -5);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(-3, -2);
const KING_OPEN_FILE: PhasedScore = p(-57, -4);
const KING_CLOSED_FILE: PhasedScore = p(15, -18);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 1);
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [
        p(-19, 5),
        p(-15, 7),
        p(-14, 8),
        p(-9, 7),
        p(-10, 10),
        p(-7, 11),
        p(2, 10),
        p(11, 7),
    ],
    // Closed
    [
        p(0, 0),
        p(0, 0),
        p(8, -1),
        p(-21, 9),
        p(-14, 15),
        p(-8, 5),
        p(-7, 10),
        p(-10, 6),
    ],
    // SemiOpen
    [
        p(0, 0),
        p(-33, 22),
        p(-10, 16),
        p(-11, 13),
        p(-15, 18),
        p(-8, 14),
        p(-8, 10),
        p(1, 10),
    ],
    // SemiClosed
    [
        p(0, 0),
        p(2, -16),
        p(-0, 7),
        p(-5, 1),
        p(-4, 6),
        p(-7, 4),
        p(-1, 7),
        p(-7, 5),
    ],
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-11, 7),   /*0b0000*/
    p(-18, 11),  /*0b0001*/
    p(-11, 7),   /*0b0010*/
    p(-8, 26),   /*0b0011*/
    p(-5, 6),    /*0b0100*/
    p(-30, 1),   /*0b0101*/
    p(-11, 16),  /*0b0110*/
    p(-13, -2),  /*0b0111*/
    p(5, 9),     /*0b1000*/
    p(-22, -11), /*0b1001*/
    p(-4, 8),    /*0b1010*/
    p(-10, 7),   /*0b1011*/
    p(-2, 4),    /*0b1100*/
    p(-37, -16), /*0b1101*/
    p(-5, 16),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-4, 16),   /*0b10000*/
    p(6, 10),    /*0b10001*/
    p(-2, -14),  /*0b10010*/
    p(-5, -1),   /*0b10011*/
    p(-3, 6),    /*0b10100*/
    p(10, 12),   /*0b10101*/
    p(-22, -11), /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(19, 45),   /*0b11000*/
    p(27, 7),    /*0b11001*/
    p(26, 26),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(20, 18),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(13, 9),    /*0b100000*/
    p(-1, 11),   /*0b100001*/
    p(17, 1),    /*0b100010*/
    p(10, 14),   /*0b100011*/
    p(-23, -21), /*0b100100*/
    p(-29, -34), /*0b100101*/
    p(-25, 3),   /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(18, -1),   /*0b101000*/
    p(-18, -6),  /*0b101001*/
    p(17, -5),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-19, -23), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(22, 31),   /*0b110000*/
    p(31, 22),   /*0b110001*/
    p(20, -10),  /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(3, 8),     /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(35, 32),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(2, -15),   /*0b111111*/
    p(-34, -12), /*0b00*/
    p(6, -35),   /*0b01*/
    p(20, -18),  /*0b10*/
    p(31, -48),  /*0b11*/
    p(40, -29),  /*0b100*/
    p(-6, -72),  /*0b101*/
    p(74, -60),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(57, -31),  /*0b1000*/
    p(22, -59),  /*0b1001*/
    p(49, -87),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(62, -27),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(12, -39),  /*0b1111*/
    p(6, -16),   /*0b00*/
    p(21, -30),  /*0b01*/
    p(18, -36),  /*0b10*/
    p(23, -50),  /*0b11*/
    p(27, -28),  /*0b100*/
    p(31, -65),  /*0b101*/
    p(18, -45),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(28, -21),  /*0b1000*/
    p(47, -38),  /*0b1001*/
    p(37, -89),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(43, -27),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(15, -80),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = [
    p(13, 12),
    p(4, 9),
    p(11, 13),
    p(8, 10),
    p(-4, 19),
    p(-41, 7),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, 0), p(36, 11), p(41, 39), p(46, 2), p(39, -20), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-81, -57),
        p(-59, -22),
        p(-43, -2),
        p(-32, 10),
        p(-21, 18),
        p(-12, 26),
        p(-1, 26),
        p(9, 24),
        p(18, 18),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-40, -48),
        p(-27, -30),
        p(-17, -15),
        p(-10, -3),
        p(-3, 5),
        p(2, 13),
        p(6, 17),
        p(9, 21),
        p(11, 24),
        p(17, 24),
        p(21, 23),
        p(29, 23),
        p(21, 31),
        p(35, 22),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-106, 9),
        p(-97, 24),
        p(-93, 29),
        p(-89, 33),
        p(-89, 39),
        p(-84, 44),
        p(-81, 48),
        p(-77, 50),
        p(-73, 54),
        p(-69, 57),
        p(-63, 59),
        p(-60, 63),
        p(-51, 62),
        p(-38, 58),
        p(-40, 61),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-228, 58),
        p(-227, 62),
        p(-230, 115),
        p(-225, 131),
        p(-222, 148),
        p(-217, 156),
        p(-213, 166),
        p(-209, 174),
        p(-204, 178),
        p(-200, 181),
        p(-196, 186),
        p(-192, 189),
        p(-188, 189),
        p(-185, 194),
        p(-181, 194),
        p(-176, 195),
        p(-173, 200),
        p(-169, 198),
        p(-157, 192),
        p(-139, 181),
        p(-126, 177),
        p(-74, 145),
        p(-67, 141),
        p(-36, 120),
        p(61, 76),
        p(99, 48),
        p(173, 8),
        p(241, -37),
    ],
    [
        p(-60, 81),
        p(-34, 39),
        p(-16, 23),
        p(3, 10),
        p(22, -3),
        p(33, -16),
        p(47, -18),
        p(58, -32),
        p(98, -66),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-11, 10),
        p(-6, -3),
        p(22, 15),
        p(43, -10),
        p(24, -48),
        p(0, 0),
    ],
    [p(1, 12), p(17, 20), p(-1, 11), p(26, 4), p(26, 67), p(0, 0)],
    [p(7, 15), p(22, 17), p(25, 19), p(-4, 10), p(43, 4), p(0, 0)],
    [p(-0, 2), p(6, 13), p(-2, 36), p(-5, 18), p(0, -15), p(0, 0)],
    [
        p(60, 39),
        p(-33, 24),
        p(-5, 21),
        p(-43, 16),
        p(0, 0),
        p(0, 0),
    ],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(5, 5), p(11, 4), p(9, 7), p(16, 4), p(10, 13), p(14, 2)],
    [
        p(-2, -1),
        p(9, 13),
        p(-38, -89),
        p(7, 10),
        p(8, 14),
        p(5, 5),
    ],
    [p(2, 1), p(14, 3), p(11, 8), p(12, 7), p(13, 16), p(23, -5)],
    [
        p(4, -4),
        p(11, -4),
        p(10, -8),
        p(5, 14),
        p(15, -306),
        p(10, -16),
    ],
    [
        p(48, -14),
        p(29, -6),
        p(35, -12),
        p(13, -8),
        p(25, -23),
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

    fn bishop_openness(
        openness: FileOpenness,
        len: usize,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore;

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

    fn bishop_openness(
        openness: FileOpenness,
        len: usize,
    ) -> <Self::Score as ScoreType>::SingleFeatureScore {
        BISHOP_OPENNESS[openness as usize][len - 1]
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
