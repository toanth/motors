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

use crate::eval::chess::{FileOpenness, NUM_PAWN_CENTER_CONFIGURATIONS, NUM_PAWN_SHIELD_CONFIGURATIONS};
use crate::eval::{ScoreType, SingleFeatureScore};
use gears::games::chess::ChessColor;
use gears::games::chess::ChessColor::White;
use gears::games::chess::pieces::{ChessPieceType, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::general::common::StaticallyNamedEntity;
use gears::score::{PhasedScore, p};
use std::fmt::{Debug, Display};

#[rustfmt::skip]
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
        p( 133,  186),    p( 129,  185),    p( 121,  188),    p( 132,  168),    p( 119,  173),    p( 119,  176),    p(  81,  194),    p(  88,  192),
        p(  69,  122),    p(  69,  124),    p(  78,  119),    p(  86,  123),    p(  74,  123),    p( 124,  110),    p(  98,  130),    p(  94,  120),
        p(  55,  113),    p(  67,  108),    p(  66,  103),    p(  85,  102),    p(  93,  102),    p(  88,   94),    p(  80,  103),    p(  75,   95),
        p(  51,  100),    p(  58,  102),    p(  78,   96),    p(  95,   96),    p(  93,   96),    p(  89,   95),    p(  73,   92),    p(  62,   86),
        p(  45,   97),    p(  55,   93),    p(  74,   97),    p(  83,   99),    p(  85,   96),    p(  80,   95),    p(  73,   83),    p(  55,   85),
        p(  53,   99),    p(  57,   96),    p(  62,   97),    p(  59,  105),    p(  59,  107),    p(  75,   97),    p(  78,   83),    p(  58,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 174,  278),    p( 196,  310),    p( 214,  321),    p( 252,  310),    p( 282,  312),    p( 198,  308),    p( 212,  309),    p( 202,  261),
        p( 267,  311),    p( 283,  316),    p( 299,  308),    p( 304,  311),    p( 303,  307),    p( 315,  296),    p( 275,  313),    p( 270,  303),
        p( 287,  306),    p( 305,  303),    p( 308,  309),    p( 322,  313),    p( 338,  306),    p( 351,  296),    p( 292,  303),    p( 285,  307),
        p( 302,  314),    p( 309,  308),    p( 325,  312),    p( 327,  319),    p( 325,  317),    p( 320,  316),    p( 310,  312),    p( 319,  310),
        p( 299,  316),    p( 305,  306),    p( 313,  312),    p( 321,  315),    p( 319,  318),    p( 325,  302),    p( 322,  303),    p( 312,  312),
        p( 275,  303),    p( 283,  301),    p( 296,  296),    p( 301,  309),    p( 305,  307),    p( 295,  290),    p( 301,  292),    p( 293,  306),
        p( 270,  311),    p( 281,  313),    p( 285,  303),    p( 294,  307),    p( 298,  302),    p( 289,  299),    p( 295,  305),    p( 289,  320),
        p( 239,  310),    p( 282,  303),    p( 266,  305),    p( 287,  310),    p( 296,  307),    p( 292,  296),    p( 288,  306),    p( 265,  308),
    ],
    // bishop
    [
        p( 276,  309),    p( 252,  313),    p( 240,  305),    p( 223,  316),    p( 218,  313),    p( 225,  307),    p( 273,  303),    p( 250,  309),
        p( 282,  302),    p( 278,  302),    p( 290,  305),    p( 278,  303),    p( 289,  300),    p( 292,  299),    p( 267,  308),    p( 270,  302),
        p( 296,  308),    p( 307,  304),    p( 291,  303),    p( 307,  298),    p( 305,  299),    p( 336,  303),    p( 317,  300),    p( 318,  312),
        p( 286,  312),    p( 292,  306),    p( 303,  302),    p( 307,  305),    p( 306,  304),    p( 299,  304),    p( 298,  307),    p( 280,  309),
        p( 290,  307),    p( 284,  309),    p( 295,  303),    p( 308,  305),    p( 302,  300),    p( 299,  302),    p( 286,  304),    p( 309,  301),
        p( 296,  310),    p( 300,  305),    p( 300,  307),    p( 299,  304),    p( 305,  307),    p( 298,  298),    p( 305,  296),    p( 307,  299),
        p( 308,  309),    p( 304,  300),    p( 308,  301),    p( 299,  308),    p( 302,  304),    p( 303,  304),    p( 312,  295),    p( 308,  296),
        p( 298,  305),    p( 309,  306),    p( 308,  307),    p( 291,  309),    p( 306,  307),    p( 294,  309),    p( 303,  296),    p( 302,  292),
    ],
    // rook
    [
        p( 460,  546),    p( 448,  556),    p( 442,  562),    p( 440,  560),    p( 451,  556),    p( 471,  550),    p( 483,  548),    p( 492,  542),
        p( 444,  552),    p( 442,  558),    p( 451,  558),    p( 466,  549),    p( 452,  552),    p( 468,  546),    p( 476,  544),    p( 491,  534),
        p( 446,  547),    p( 464,  542),    p( 458,  544),    p( 458,  539),    p( 483,  529),    p( 494,  525),    p( 511,  525),    p( 486,  527),
        p( 442,  547),    p( 448,  543),    p( 447,  545),    p( 453,  539),    p( 457,  531),    p( 468,  527),    p( 468,  531),    p( 468,  526),
        p( 436,  545),    p( 434,  543),    p( 435,  544),    p( 441,  539),    p( 448,  535),    p( 442,  535),    p( 454,  529),    p( 449,  527),
        p( 431,  542),    p( 431,  539),    p( 432,  538),    p( 435,  538),    p( 441,  532),    p( 451,  524),    p( 468,  512),    p( 455,  517),
        p( 433,  537),    p( 437,  536),    p( 442,  537),    p( 445,  535),    p( 452,  528),    p( 464,  518),    p( 472,  513),    p( 444,  522),
        p( 442,  542),    p( 439,  536),    p( 440,  540),    p( 444,  534),    p( 450,  528),    p( 456,  528),    p( 453,  526),    p( 449,  528),
    ],
    // queen
    [
        p( 879,  958),    p( 881,  973),    p( 896,  985),    p( 917,  980),    p( 914,  983),    p( 934,  971),    p( 980,  923),    p( 925,  955),
        p( 889,  949),    p( 864,  978),    p( 866, 1005),    p( 859, 1022),    p( 866, 1033),    p( 906,  994),    p( 907,  978),    p( 948,  958),
        p( 895,  954),    p( 887,  969),    p( 886,  990),    p( 887, 1000),    p( 909, 1002),    p( 947,  986),    p( 955,  956),    p( 943,  963),
        p( 881,  966),    p( 887,  974),    p( 880,  984),    p( 882,  995),    p( 883, 1008),    p( 897,  998),    p( 906,  999),    p( 914,  975),
        p( 891,  957),    p( 878,  977),    p( 884,  977),    p( 884,  994),    p( 889,  990),    p( 890,  991),    p( 903,  980),    p( 909,  973),
        p( 887,  946),    p( 893,  962),    p( 887,  977),    p( 884,  980),    p( 889,  988),    p( 896,  976),    p( 910,  959),    p( 909,  947),
        p( 887,  949),    p( 887,  956),    p( 894,  959),    p( 893,  973),    p( 895,  972),    p( 896,  955),    p( 908,  933),    p( 916,  906),
        p( 873,  949),    p( 885,  938),    p( 886,  950),    p( 894,  951),    p( 897,  940),    p( 883,  945),    p( 885,  938),    p( 890,  920),
    ],
    // king
    [
        p( 157,  -85),    p(  61,  -37),    p(  87,  -30),    p(  10,    2),    p(  39,  -11),    p(  23,   -1),    p(  76,   -9),    p( 231,  -89),
        p( -31,    2),    p( -79,   20),    p( -79,   27),    p( -19,   17),    p( -48,   25),    p( -79,   40),    p( -49,   25),    p(   6,    1),
        p( -45,   10),    p( -46,   15),    p( -83,   29),    p( -93,   38),    p( -61,   32),    p( -30,   24),    p( -76,   27),    p( -38,   11),
        p( -26,    2),    p( -99,   14),    p(-111,   30),    p(-133,   38),    p(-133,   36),    p(-112,   29),    p(-131,   18),    p(-106,   17),
        p( -41,   -2),    p(-113,    9),    p(-124,   26),    p(-148,   39),    p(-150,   36),    p(-126,   23),    p(-142,   13),    p(-118,   13),
        p( -33,    2),    p( -89,    5),    p(-117,   20),    p(-125,   28),    p(-122,   27),    p(-132,   19),    p(-107,    5),    p( -74,   10),
        p(  26,   -7),    p( -75,   -1),    p( -88,    8),    p(-108,   18),    p(-113,   18),    p( -99,    9),    p( -71,   -8),    p(   2,   -4),
        p(  54,  -25),    p(  43,  -35),    p(  40,  -21),    p( -22,   -2),    p(  29,  -18),    p( -18,   -5),    p(  35,  -29),    p(  65,  -35),
    ],
];

const BISHOP_PAIR: PhasedScore = p(22, 53);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(9, 19), p(10, 18), p(10, 7), p(7, -1), p(3, -9), p(-1, -19), p(-7, -28), p(-15, -41), p(-28, -52)];
const ROOK_OPEN_FILE: PhasedScore = p(13, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, 0);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 4);
const KING_OPEN_FILE: PhasedScore = p(-49, -1);
const KING_CLOSED_FILE: PhasedScore = p(14, -15);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-5, 3), p(0, 5), p(-1, 4), p(2, 2), p(2, 5), p(4, 7), p(7, 4), p(18, 1)],
    // Closed
    [p(0, 0), p(0, 0), p(17, -28), p(-16, 9), p(-2, 11), p(-1, 5), p(-2, 7), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-15, 22), p(2, 17), p(0, 9), p(-1, 9), p(4, 5), p(0, 2), p(10, 5)],
    // SemiClosed
    [p(0, 0), p(11, -13), p(7, 5), p(3, -0), p(6, 2), p(1, 5), p(3, 5), p(1, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(21, 4),
    p(3, 3),
    p(-4, 3),
    p(5, 18),
    p(4, 2),
    p(-10, -7),
    p(5, 8),
    p(8, 4),
    p(-1, -3),
    p(-11, 2),
    p(-14, -13),
    p(-5, 13),
    p(19, 5),
    p(13, 9),
    p(18, 11),
    p(21, 39),
    p(-5, -3),
    p(-21, -2),
    p(-23, -3),
    p(-36, 27),
    p(-19, 5),
    p(-18, -16),
    p(13, 24),
    p(-45, 42),
    p(-20, -13),
    p(-22, -8),
    p(-45, -30),
    p(-30, 17),
    p(-8, 14),
    p(23, 13),
    p(-82, 123),
    p(0, 0),
    p(-1, -2),
    p(-15, -0),
    p(-7, -3),
    p(-13, 15),
    p(-31, -5),
    p(-54, -19),
    p(-30, 40),
    p(-34, 33),
    p(-9, -1),
    p(-21, -0),
    p(1, -6),
    p(-9, 55),
    p(-41, 23),
    p(3, -22),
    p(0, 0),
    p(0, 0),
    p(52, 42),
    p(40, 65),
    p(41, -9),
    p(0, 0),
    p(49, 35),
    p(7, 41),
    p(0, 0),
    p(0, 0),
    p(21, 53),
    p(30, 53),
    p(25, 63),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(19, 1),
    p(-3, -2),
    p(-6, 2),
    p(-14, 1),
    p(3, -2),
    p(-29, -2),
    p(-12, 5),
    p(-29, -2),
    p(3, -3),
    p(-16, -6),
    p(-29, -3),
    p(-36, 7),
    p(-5, 1),
    p(-38, 6),
    p(-29, 2),
    p(-46, 75),
    p(9, -2),
    p(-2, -4),
    p(-7, -11),
    p(-22, -4),
    p(-9, 4),
    p(-17, -5),
    p(-17, 1),
    p(-75, 187),
    p(-7, -8),
    p(-27, -7),
    p(-41, -24),
    p(8, -80),
    p(-10, -0),
    p(-13, -5),
    p(-73, 75),
    p(0, 0),
    p(13, -1),
    p(-1, -2),
    p(-11, -4),
    p(-14, -3),
    p(-5, 2),
    p(-30, -10),
    p(-10, 3),
    p(-21, 6),
    p(-0, -4),
    p(-22, -2),
    p(-26, -11),
    p(-32, 0),
    p(-7, -0),
    p(-46, -7),
    p(-10, 25),
    p(-54, 58),
    p(14, 7),
    p(-1, 8),
    p(-18, 63),
    p(0, 0),
    p(-9, 8),
    p(-14, 6),
    p(0, 0),
    p(0, 0),
    p(-5, 12),
    p(-30, 26),
    p(-27, -31),
    p(0, 0),
    p(20, -53),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-6, 6),    /*0b0000*/
    p(-15, 8),   /*0b0001*/
    p(-4, 9),    /*0b0010*/
    p(-9, 11),   /*0b0011*/
    p(-4, 3),    /*0b0100*/
    p(-26, 0),   /*0b0101*/
    p(-13, 4),   /*0b0110*/
    p(-18, -16), /*0b0111*/
    p(8, 11),    /*0b1000*/
    p(-3, 11),   /*0b1001*/
    p(2, 10),    /*0b1010*/
    p(-1, 8),    /*0b1011*/
    p(-0, 5),    /*0b1100*/
    p(-24, 9),   /*0b1101*/
    p(-11, 4),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(4, 15),    /*0b10000*/
    p(3, 9),     /*0b10001*/
    p(18, 12),   /*0b10010*/
    p(-5, 6),    /*0b10011*/
    p(-5, 5),    /*0b10100*/
    p(11, 14),   /*0b10101*/
    p(-23, 0),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(15, 24),   /*0b11000*/
    p(29, 21),   /*0b11001*/
    p(40, 34),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(16, 8),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(16, 10),   /*0b100000*/
    p(2, 12),    /*0b100001*/
    p(23, 4),    /*0b100010*/
    p(7, -1),    /*0b100011*/
    p(-7, 2),    /*0b100100*/
    p(-22, -7),  /*0b100101*/
    p(-24, 13),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(24, 5),    /*0b101000*/
    p(-1, 16),   /*0b101001*/
    p(20, -1),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-4, 8),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(15, 16),   /*0b110000*/
    p(26, 11),   /*0b110001*/
    p(32, 8),    /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(2, 26),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(28, 9),    /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(8, -2),    /*0b111111*/
    p(-14, -3),  /*0b00*/
    p(11, -18),  /*0b01*/
    p(39, -9),   /*0b10*/
    p(24, -41),  /*0b11*/
    p(46, -10),  /*0b100*/
    p(4, -20),   /*0b101*/
    p(70, -41),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(65, -12),  /*0b1000*/
    p(21, -34),  /*0b1001*/
    p(79, -54),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(63, -19),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(25, -11),  /*0b1111*/
    p(22, -3),   /*0b00*/
    p(35, -13),  /*0b01*/
    p(28, -18),  /*0b10*/
    p(26, -42),  /*0b11*/
    p(39, -10),  /*0b100*/
    p(56, -21),  /*0b101*/
    p(26, -24),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(41, -3),   /*0b1000*/
    p(54, -18),  /*0b1001*/
    p(53, -43),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(48, -22),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(23, -44),  /*0b1111*/
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  33,   86),    p(  29,   85),    p(  21,   88),    p(  32,   68),    p(  19,   73),    p(  19,   76),    p( -19,   94),    p( -12,   92),
        p(  36,  123),    p(  43,  122),    p(  35,   99),    p(  19,   69),    p(  32,   69),    p(  12,   94),    p(  -4,  103),    p( -33,  125),
        p(  21,   73),    p(  15,   70),    p(  19,   54),    p(  18,   43),    p(  -1,   46),    p(   4,   58),    p( -12,   75),    p( -12,   79),
        p(   6,   46),    p(  -4,   43),    p( -13,   33),    p(  -3,   25),    p( -15,   29),    p(  -6,   38),    p( -20,   54),    p( -13,   51),
        p(   0,   14),    p( -14,   23),    p( -14,   15),    p( -11,    7),    p( -13,   15),    p(  -9,   17),    p( -17,   37),    p(   9,   16),
        p(  -7,   14),    p(  -4,   19),    p( -10,   16),    p(  -6,    4),    p(   6,    0),    p(   5,    7),    p(  11,   18),    p(   5,   13),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-8, -10);
const DOUBLED_PAWN: PhasedScore = p(-7, -22);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(15, 10), p(7, 13), p(14, 18), p(9, 7), p(-3, 16), p(-45, 6)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(39, 8), p(39, 34), p(52, -8), p(35, -35), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-46, -73),
        p(-26, -32),
        p(-13, -9),
        p(-4, 4),
        p(4, 15),
        p(11, 26),
        p(19, 29),
        p(26, 31),
        p(31, 30),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-31, -56),
        p(-19, -38),
        p(-8, -23),
        p(-1, -10),
        p(6, -1),
        p(12, 8),
        p(16, 13),
        p(21, 17),
        p(23, 22),
        p(30, 24),
        p(34, 23),
        p(42, 26),
        p(39, 33),
        p(54, 27),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-75, 12),
        p(-67, 26),
        p(-62, 31),
        p(-59, 36),
        p(-60, 42),
        p(-54, 47),
        p(-51, 51),
        p(-46, 54),
        p(-42, 58),
        p(-39, 61),
        p(-35, 64),
        p(-33, 68),
        p(-26, 69),
        p(-17, 67),
        p(-15, 67),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-26, -56),
        p(-26, 7),
        p(-30, 57),
        p(-26, 75),
        p(-24, 93),
        p(-19, 98),
        p(-15, 109),
        p(-12, 114),
        p(-8, 118),
        p(-5, 119),
        p(-2, 122),
        p(3, 124),
        p(5, 124),
        p(7, 129),
        p(10, 130),
        p(13, 133),
        p(14, 140),
        p(16, 141),
        p(26, 138),
        p(39, 132),
        p(43, 134),
        p(86, 110),
        p(86, 113),
        p(109, 96),
        p(204, 61),
        p(249, 21),
        p(280, 9),
        p(332, -26),
    ],
    [
        p(-94, 9),
        p(-58, -4),
        p(-28, -5),
        p(2, -3),
        p(33, -3),
        p(57, -4),
        p(85, 2),
        p(110, 1),
        p(160, -17),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-9, 7), p(0, 0), p(23, 19), p(49, -12), p(20, -33), p(0, 0)],
    [p(-2, 10), p(20, 22), p(0, 0), p(31, 5), p(30, 53), p(0, 0)],
    [p(-3, 13), p(10, 15), p(17, 12), p(0, 0), p(45, -6), p(0, 0)],
    [p(-2, 4), p(2, 6), p(-1, 22), p(1, 1), p(0, 0), p(0, 0)],
    [p(70, 28), p(-35, 18), p(-9, 17), p(-21, 7), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 7), p(8, 7), p(6, 11), p(12, 7), p(6, 19), p(11, 6)],
    [p(2, 6), p(11, 22), p(-128, -28), p(8, 14), p(9, 20), p(4, 7)],
    [p(3, 1), p(14, 6), p(9, 11), p(11, 8), p(11, 21), p(21, -6)],
    [p(2, -2), p(9, 1), p(7, -4), p(4, 15), p(-60, -256), p(5, -11)],
    [p(63, -2), p(41, 6), p(46, -0), p(24, 4), p(37, -12), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-20, -18), p(19, -10), p(10, -4), p(15, -12), p(-1, 12), p(-11, 11)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(14, 19), p(34, -0), p(5, 32)];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static + StaticallyNamedEntity {
    type Score: ScoreType;

    fn psqt(&self, square: ChessSquare, piece: ChessPieceType, color: ChessColor) -> SingleFeatureScore<Self::Score>;

    fn passed_pawn(square: ChessSquare) -> SingleFeatureScore<Self::Score>;

    fn unsupported_pawn() -> SingleFeatureScore<Self::Score>;

    fn doubled_pawn() -> SingleFeatureScore<Self::Score>;

    fn bishop_pair() -> SingleFeatureScore<Self::Score>;

    fn bad_bishop(num_pawns: usize) -> SingleFeatureScore<Self::Score>;

    fn rook_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score>;

    fn king_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score>;

    fn bishop_openness(openness: FileOpenness, len: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_advanced_center(config: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_passive_center(config: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_shield(&self, color: ChessColor, config: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_protection(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn pawn_attack(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn mobility(piece: ChessPieceType, mobility: usize) -> SingleFeatureScore<Self::Score>;

    fn threats(attacking: ChessPieceType, targeted: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn king_zone_attack(attacking: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn can_give_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;
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

    fn bishop_pair() -> PhasedScore {
        BISHOP_PAIR
    }

    fn bad_bishop(num_pawns: usize) -> PhasedScore {
        BAD_BISHOP[num_pawns]
    }

    fn rook_openness(openness: FileOpenness) -> PhasedScore {
        match openness {
            FileOpenness::Open => ROOK_OPEN_FILE,
            FileOpenness::Closed => ROOK_CLOSED_FILE,
            FileOpenness::SemiOpen => ROOK_SEMIOPEN_FILE,
            FileOpenness::SemiClosed => PhasedScore::default(),
        }
    }

    fn king_openness(openness: FileOpenness) -> PhasedScore {
        match openness {
            FileOpenness::Open => KING_OPEN_FILE,
            FileOpenness::Closed => KING_CLOSED_FILE,
            FileOpenness::SemiOpen => KING_SEMIOPEN_FILE,
            FileOpenness::SemiClosed => PhasedScore::default(),
        }
    }

    fn bishop_openness(openness: FileOpenness, len: usize) -> PhasedScore {
        BISHOP_OPENNESS[openness as usize][len - 1]
    }

    fn pawn_advanced_center(config: usize) -> PhasedScore {
        PAWN_ADVANCED_CENTER[config]
    }

    fn pawn_passive_center(config: usize) -> PhasedScore {
        PAWN_PASSIVE_CENTER[config]
    }

    fn pawn_shield(&self, _color: ChessColor, config: usize) -> PhasedScore {
        PAWN_SHIELDS[config]
    }

    fn pawn_protection(piece: ChessPieceType) -> PhasedScore {
        PAWN_PROTECTION[piece as usize]
    }

    fn pawn_attack(piece: ChessPieceType) -> PhasedScore {
        PAWN_ATTACKS[piece as usize]
    }

    fn mobility(piece: ChessPieceType, mobility: usize) -> PhasedScore {
        MOBILITY[piece as usize - 1][mobility]
    }

    fn threats(attacking: ChessPieceType, targeted: ChessPieceType) -> PhasedScore {
        THREATS[attacking as usize - 1][targeted as usize]
    }
    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> PhasedScore {
        DEFENDED[protecting as usize - 1][target as usize]
    }

    fn king_zone_attack(attacking: ChessPieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }

    fn can_give_check(piece: ChessPieceType) -> PhasedScore {
        CAN_GIVE_CHECK[piece as usize]
    }
}
