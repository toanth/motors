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
use gears::games::DimT;
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
        p( 125,  160),    p( 122,  159),    p( 114,  163),    p( 124,  147),    p( 110,  152),    p( 109,  155),    p(  68,  172),    p(  72,  171),
        p(  69,  117),    p(  67,  119),    p(  73,  111),    p(  81,  108),    p(  68,  102),    p( 121,  103),    p(  94,  121),    p(  93,  115),
        p(  53,  105),    p(  59,   98),    p(  57,   92),    p(  82,   96),    p(  88,   95),    p(  78,   82),    p(  74,   93),    p(  70,   89),
        p(  47,   92),    p(  49,   93),    p(  73,   90),    p(  93,   92),    p(  85,   95),    p(  81,   90),    p(  64,   85),    p(  57,   79),
        p(  41,   90),    p(  46,   86),    p(  68,   91),    p(  78,   93),    p(  80,   92),    p(  75,   90),    p(  65,   77),    p(  49,   79),
        p(  51,   94),    p(  55,   92),    p(  58,   92),    p(  53,   98),    p(  57,  100),    p(  73,   92),    p(  77,   82),    p(  56,   84),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 169,  280),    p( 189,  314),    p( 206,  325),    p( 242,  314),    p( 273,  316),    p( 190,  310),    p( 210,  309),    p( 196,  264),
        p( 265,  313),    p( 282,  318),    p( 291,  311),    p( 297,  314),    p( 295,  310),    p( 305,  299),    p( 266,  317),    p( 271,  305),
        p( 285,  310),    p( 304,  306),    p( 301,  312),    p( 312,  315),    p( 328,  310),    p( 344,  298),    p( 287,  307),    p( 281,  308),
        p( 303,  316),    p( 312,  313),    p( 325,  315),    p( 324,  322),    p( 321,  323),    p( 323,  320),    p( 313,  319),    p( 320,  311),
        p( 299,  317),    p( 305,  309),    p( 314,  315),    p( 321,  318),    p( 318,  322),    p( 325,  308),    p( 323,  307),    p( 313,  314),
        p( 277,  303),    p( 284,  304),    p( 298,  299),    p( 302,  312),    p( 307,  310),    p( 298,  293),    p( 303,  296),    p( 294,  308),
        p( 270,  310),    p( 281,  315),    p( 285,  306),    p( 295,  310),    p( 299,  306),    p( 291,  304),    p( 295,  308),    p( 289,  319),
        p( 241,  307),    p( 281,  305),    p( 268,  308),    p( 288,  312),    p( 297,  310),    p( 292,  301),    p( 288,  309),    p( 264,  306),
    ],
    // bishop
    [
        p( 273,  313),    p( 246,  317),    p( 234,  310),    p( 216,  319),    p( 209,  318),    p( 223,  309),    p( 270,  307),    p( 248,  312),
        p( 281,  304),    p( 276,  306),    p( 288,  308),    p( 271,  307),    p( 283,  304),    p( 287,  302),    p( 261,  311),    p( 270,  305),
        p( 294,  310),    p( 305,  306),    p( 284,  307),    p( 302,  301),    p( 298,  304),    p( 331,  306),    p( 311,  305),    p( 314,  315),
        p( 289,  311),    p( 296,  306),    p( 305,  302),    p( 304,  308),    p( 308,  303),    p( 304,  304),    p( 309,  305),    p( 283,  310),
        p( 287,  307),    p( 286,  308),    p( 297,  304),    p( 310,  304),    p( 302,  301),    p( 305,  300),    p( 288,  304),    p( 310,  299),
        p( 295,  309),    p( 299,  305),    p( 301,  306),    p( 300,  304),    p( 305,  307),    p( 300,  299),    p( 306,  295),    p( 306,  299),
        p( 305,  309),    p( 303,  301),    p( 306,  302),    p( 298,  310),    p( 300,  308),    p( 302,  306),    p( 311,  296),    p( 307,  295),
        p( 298,  305),    p( 308,  306),    p( 306,  309),    p( 289,  312),    p( 305,  310),    p( 293,  312),    p( 302,  300),    p( 301,  293),
    ],
    // rook
    [
        p( 450,  549),    p( 436,  561),    p( 427,  568),    p( 427,  564),    p( 440,  560),    p( 467,  554),    p( 472,  555),    p( 483,  547),
        p( 443,  555),    p( 440,  561),    p( 449,  562),    p( 463,  553),    p( 449,  556),    p( 463,  550),    p( 472,  547),    p( 489,  536),
        p( 443,  551),    p( 462,  546),    p( 456,  548),    p( 455,  543),    p( 477,  534),    p( 490,  529),    p( 505,  530),    p( 482,  532),
        p( 443,  551),    p( 453,  546),    p( 454,  548),    p( 454,  543),    p( 458,  537),    p( 474,  530),    p( 475,  536),    p( 470,  531),
        p( 435,  548),    p( 437,  547),    p( 438,  547),    p( 444,  543),    p( 447,  540),    p( 445,  540),    p( 458,  535),    p( 451,  531),
        p( 432,  545),    p( 433,  542),    p( 436,  541),    p( 437,  540),    p( 443,  535),    p( 454,  528),    p( 470,  518),    p( 455,  521),
        p( 433,  540),    p( 437,  539),    p( 444,  540),    p( 447,  537),    p( 452,  531),    p( 468,  521),    p( 474,  517),    p( 443,  526),
        p( 442,  545),    p( 439,  540),    p( 441,  543),    p( 445,  537),    p( 450,  531),    p( 456,  531),    p( 452,  532),    p( 448,  534),
    ],
    // queen
    [
        p( 864,  969),    p( 867,  982),    p( 880,  995),    p( 903,  988),    p( 902,  992),    p( 928,  979),    p( 965,  937),    p( 913,  968),
        p( 889,  953),    p( 865,  981),    p( 865, 1008),    p( 857, 1026),    p( 864, 1038),    p( 901, 1002),    p( 907,  983),    p( 946,  966),
        p( 893,  961),    p( 888,  973),    p( 883,  996),    p( 885, 1005),    p( 897, 1012),    p( 945,  992),    p( 948,  969),    p( 940,  974),
        p( 886,  969),    p( 891,  975),    p( 886,  983),    p( 880,  999),    p( 886, 1009),    p( 906,  995),    p( 913, 1002),    p( 918,  980),
        p( 890,  962),    p( 883,  976),    p( 885,  979),    p( 888,  992),    p( 889,  992),    p( 896,  989),    p( 904,  986),    p( 911,  977),
        p( 887,  948),    p( 894,  964),    p( 890,  976),    p( 887,  979),    p( 891,  988),    p( 899,  975),    p( 911,  964),    p( 907,  953),
        p( 886,  953),    p( 888,  960),    p( 895,  962),    p( 894,  975),    p( 896,  974),    p( 898,  957),    p( 908,  939),    p( 915,  913),
        p( 874,  953),    p( 885,  943),    p( 885,  955),    p( 894,  956),    p( 896,  946),    p( 883,  950),    p( 884,  944),    p( 891,  925),
    ],
    // king
    [
        p( 158,  -64),    p(  67,  -13),    p(  96,   -5),    p(  25,   15),    p(  39,    5),    p(  15,   17),    p(  80,    4),    p( 202,  -66),
        p( -20,   16),    p( -43,   31),    p( -46,   39),    p(  12,   24),    p( -17,   33),    p( -44,   45),    p( -22,   34),    p(  10,   18),
        p( -43,   23),    p( -25,   23),    p( -66,   37),    p( -76,   42),    p( -42,   38),    p( -13,   31),    p( -52,   32),    p( -24,   22),
        p( -22,    8),    p( -84,   18),    p( -99,   32),    p(-126,   38),    p(-123,   38),    p(-103,   30),    p(-111,   21),    p(-100,   23),
        p( -41,   -0),    p( -98,    7),    p(-113,   22),    p(-140,   32),    p(-138,   32),    p(-112,   18),    p(-127,   11),    p(-115,   15),
        p( -36,    3),    p( -77,    0),    p(-104,   13),    p(-113,   19),    p(-109,   20),    p(-118,   13),    p( -94,    2),    p( -71,   10),
        p(  24,   -6),    p( -64,   -5),    p( -74,    1),    p( -92,    7),    p( -99,   11),    p( -84,    2),    p( -59,  -12),    p(   2,   -3),
        p(  44,  -17),    p(  39,  -30),    p(  37,  -18),    p( -24,   -3),    p(  26,  -16),    p( -20,   -4),    p(  32,  -24),    p(  56,  -29),
    ],
];

const BISHOP_PAIR: PhasedScore = p(24, 51);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(9, 21), p(11, 18), p(10, 7), p(6, 0), p(2, -7), p(-1, -16), p(-7, -24), p(-14, -37), p(-25, -43)];
const ROOK_OPEN_FILE: PhasedScore = p(15, 2);
const ROOK_CLOSED_FILE: PhasedScore = p(-10, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(5, -1);
const KING_OPEN_FILE: PhasedScore = p(-43, 5);
const KING_CLOSED_FILE: PhasedScore = p(12, -7);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 9);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 5), p(-0, 7), p(-2, 6), p(2, 4), p(3, 6), p(4, 7), p(7, 4), p(19, 2)],
    // Closed
    [p(0, 0), p(0, 0), p(18, -13), p(-14, 10), p(-1, 12), p(-0, 5), p(-1, 7), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-13, 24), p(3, 17), p(1, 11), p(0, 11), p(5, 6), p(2, 3), p(10, 6)],
    // SemiClosed
    [p(0, 0), p(9, -10), p(6, 7), p(2, 0), p(5, 2), p(1, 4), p(3, 5), p(2, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(23, 12),
    p(1, 5),
    p(-5, 6),
    p(-17, 8),
    p(6, 3),
    p(-10, -11),
    p(-5, -2),
    p(-9, -13),
    p(2, 0),
    p(-11, -1),
    p(-13, -16),
    p(-25, -6),
    p(7, -8),
    p(-3, -12),
    p(6, -10),
    p(1, 6),
    p(-6, -3),
    p(-25, -5),
    p(-20, 0),
    p(-49, 21),
    p(-19, 1),
    p(-21, -20),
    p(4, 20),
    p(-56, 25),
    p(-17, -18),
    p(-23, -17),
    p(-40, -33),
    p(-45, 11),
    p(-13, -6),
    p(11, -7),
    p(-101, 110),
    p(0, 0),
    p(-1, -3),
    p(-17, -5),
    p(-11, -5),
    p(-31, -2),
    p(-24, -3),
    p(-49, -23),
    p(-35, 35),
    p(-49, 21),
    p(-7, -5),
    p(-23, -10),
    p(1, -12),
    p(-23, 31),
    p(-45, 15),
    p(-3, -36),
    p(0, 0),
    p(0, 0),
    p(-3, -15),
    p(-19, 4),
    p(-17, -57),
    p(0, 0),
    p(8, -20),
    p(-41, -19),
    p(0, 0),
    p(0, 0),
    p(-29, -7),
    p(-23, -17),
    p(-24, 15),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(26, 4),
    p(3, -2),
    p(-5, 0),
    p(-23, -4),
    p(6, -4),
    p(-31, -9),
    p(-21, -4),
    p(-43, -14),
    p(7, -3),
    p(-14, -9),
    p(-33, -10),
    p(-51, -3),
    p(-10, -6),
    p(-49, -7),
    p(-43, -16),
    p(-65, 57),
    p(10, -3),
    p(-3, -9),
    p(-7, -13),
    p(-27, -6),
    p(-13, -3),
    p(-22, -13),
    p(-30, -5),
    p(-85, 176),
    p(-8, -12),
    p(-31, -16),
    p(-42, -32),
    p(8, -88),
    p(-20, -11),
    p(-23, -19),
    p(-87, 60),
    p(0, 0),
    p(16, -1),
    p(0, -5),
    p(-15, -10),
    p(-25, -12),
    p(-3, -1),
    p(-32, -15),
    p(-21, -2),
    p(-37, 0),
    p(-1, -9),
    p(-25, -9),
    p(-30, -19),
    p(-42, -11),
    p(-13, -5),
    p(-50, -12),
    p(-17, 17),
    p(-69, 55),
    p(3, -2),
    p(-14, -5),
    p(-31, 52),
    p(0, 0),
    p(-22, -5),
    p(-29, 0),
    p(0, 0),
    p(0, 0),
    p(-18, -3),
    p(-45, 6),
    p(-41, -40),
    p(0, 0),
    p(-2, -60),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-6, 8),    /*0b0000*/
    p(-14, 8),   /*0b0001*/
    p(-4, 11),   /*0b0010*/
    p(-9, 12),   /*0b0011*/
    p(-2, 3),    /*0b0100*/
    p(-27, 2),   /*0b0101*/
    p(-12, 5),   /*0b0110*/
    p(-19, -11), /*0b0111*/
    p(12, 5),    /*0b1000*/
    p(-2, 11),   /*0b1001*/
    p(2, 11),    /*0b1010*/
    p(0, 12),    /*0b1011*/
    p(2, 4),     /*0b1100*/
    p(-26, 7),   /*0b1101*/
    p(-9, 6),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(10, 10),   /*0b10000*/
    p(5, 8),     /*0b10001*/
    p(20, 10),   /*0b10010*/
    p(-3, 8),    /*0b10011*/
    p(-2, 3),    /*0b10100*/
    p(14, 12),   /*0b10101*/
    p(-20, 2),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(20, 10),   /*0b11000*/
    p(27, 15),   /*0b11001*/
    p(38, 25),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(16, -1),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(20, 4),    /*0b100000*/
    p(5, 11),    /*0b100001*/
    p(23, 4),    /*0b100010*/
    p(8, 1),     /*0b100011*/
    p(-5, 1),    /*0b100100*/
    p(-23, -6),  /*0b100101*/
    p(-23, 20),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(30, -1),   /*0b101000*/
    p(-1, 14),   /*0b101001*/
    p(20, -4),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-0, 6),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(19, 3),    /*0b110000*/
    p(23, 4),    /*0b110001*/
    p(31, -3),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(2, 17),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(32, -4),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(8, -2),    /*0b111111*/
    p(-12, 4),   /*0b00*/
    p(5, -12),   /*0b01*/
    p(34, -7),   /*0b10*/
    p(21, -41),  /*0b11*/
    p(45, -10),  /*0b100*/
    p(-8, -9),   /*0b101*/
    p(64, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(67, -15),  /*0b1000*/
    p(17, -29),  /*0b1001*/
    p(75, -58),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(61, -36),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(19, 1),    /*0b1111*/
    p(20, 1),    /*0b00*/
    p(30, -9),   /*0b01*/
    p(25, -14),  /*0b10*/
    p(20, -37),  /*0b11*/
    p(39, -9),   /*0b100*/
    p(51, -18),  /*0b101*/
    p(23, -20),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(39, -2),   /*0b1000*/
    p(49, -15),  /*0b1001*/
    p(52, -37),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(43, -26),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(19, -42),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(-25, -33);
const STOPPABLE_PASSER: PhasedScore = p(36, -48);
const CLOSE_KING_PASSER: PhasedScore = p(-7, 28);
const IMMOBILE_PASSER: PhasedScore = p(-3, -36);
const PROTECTED_PASSER: PhasedScore = p(8, -4);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -41,   41),    p( -48,   55),    p( -66,   58),    p( -68,   51),    p( -58,   39),    p( -47,   39),    p( -31,   45),    p( -46,   46),
        p( -30,   36),    p( -54,   58),    p( -62,   55),    p( -61,   48),    p( -69,   48),    p( -56,   47),    p( -58,   63),    p( -42,   45),
        p( -26,   55),    p( -32,   57),    p( -58,   62),    p( -55,   62),    p( -63,   58),    p( -51,   59),    p( -58,   70),    p( -59,   67),
        p( -15,   72),    p( -17,   75),    p( -17,   66),    p( -40,   75),    p( -57,   76),    p( -44,   75),    p( -52,   84),    p( -59,   86),
        p(   2,   71),    p(   7,   70),    p(   2,   54),    p( -18,   40),    p( -15,   52),    p( -37,   58),    p( -46,   60),    p( -83,   81),
        p(  25,   60),    p(  22,   59),    p(  14,   63),    p(  24,   47),    p(  10,   52),    p(   9,   55),    p( -32,   72),    p( -28,   71),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(1, 4), p(5, 8), p(8, 17), p(16, 22), p(17, 68), p(12, 60)];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -6);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);
const PHALANX: [PhasedScore; 6] = [p(0, -0), p(5, 3), p(9, 6), p(21, 21), p(59, 76), p(-98, 221)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(16, 14), p(7, 16), p(14, 20), p(9, 9), p(-3, 18), p(-45, 11)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(40, 10), p(41, 33), p(54, -12), p(38, -42), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, -5), p(14, 21), p(18, -7), p(16, 9), p(17, -10), p(26, -11)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-44, -68),
        p(-24, -29),
        p(-12, -6),
        p(-4, 7),
        p(3, 17),
        p(10, 28),
        p(17, 30),
        p(23, 33),
        p(27, 32),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-30, -57),
        p(-18, -38),
        p(-7, -23),
        p(0, -10),
        p(7, -1),
        p(11, 9),
        p(16, 14),
        p(21, 18),
        p(22, 23),
        p(29, 25),
        p(34, 25),
        p(41, 28),
        p(36, 38),
        p(50, 30),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-74, 15),
        p(-65, 28),
        p(-61, 34),
        p(-58, 39),
        p(-58, 45),
        p(-53, 51),
        p(-50, 56),
        p(-47, 60),
        p(-44, 64),
        p(-41, 69),
        p(-37, 72),
        p(-36, 77),
        p(-28, 79),
        p(-19, 77),
        p(-13, 74),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-26, -39),
        p(-25, 4),
        p(-29, 58),
        p(-25, 76),
        p(-23, 95),
        p(-18, 101),
        p(-15, 112),
        p(-11, 119),
        p(-8, 123),
        p(-4, 125),
        p(-2, 128),
        p(2, 131),
        p(5, 132),
        p(6, 137),
        p(9, 139),
        p(12, 143),
        p(13, 151),
        p(15, 153),
        p(25, 151),
        p(39, 145),
        p(43, 148),
        p(86, 126),
        p(85, 129),
        p(107, 113),
        p(198, 81),
        p(243, 41),
        p(283, 23),
        p(323, -7),
    ],
    [
        p(-81, 3),
        p(-50, -9),
        p(-25, -9),
        p(1, -5),
        p(28, -3),
        p(47, -1),
        p(72, 5),
        p(92, 7),
        p(132, -1),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-5, 12), p(0, 0), p(23, 19), p(49, -10), p(21, -33), p(0, 0)],
    [p(-2, 10), p(20, 21), p(0, 0), p(30, 5), p(30, 52), p(0, 0)],
    [p(-3, 14), p(10, 13), p(17, 9), p(0, 0), p(41, 3), p(0, 0)],
    [p(-2, 5), p(2, 4), p(-1, 18), p(0, -1), p(0, 0), p(0, 0)],
    [p(56, 19), p(-29, 19), p(4, 15), p(-20, 10), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 7), p(7, 6), p(5, 10), p(12, 6), p(6, 17), p(10, 5)],
    [p(2, 8), p(11, 20), p(-116, -24), p(8, 13), p(9, 20), p(3, 6)],
    [p(2, 3), p(12, 6), p(9, 12), p(11, 9), p(9, 26), p(20, -3)],
    [p(2, 1), p(8, 1), p(6, -4), p(4, 13), p(-77, -237), p(4, -9)],
    [p(58, -2), p(37, 10), p(42, 4), p(20, 7), p(32, -3), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-21, -13), p(23, -11), p(12, -4), p(16, -12), p(2, 12), p(9, -1)];
const EXTENDED_KING_ZONE_ATTACK: PhasedScore = p(7, -3);
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(25, 13), p(12, 20), p(32, 2), p(3, 33)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(5, -17), p(23, 30), p(15, 36), p(40, 24), p(68, 32)];
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(-6, -18), p(105, 36), p(149, -8), p(56, 145), p(0, 0), p(35, 13)];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static + StaticallyNamedEntity {
    type Score: ScoreType;

    fn psqt(&self, square: ChessSquare, piece: ChessPieceType, color: ChessColor) -> SingleFeatureScore<Self::Score>;

    fn passed_pawn(square: ChessSquare) -> SingleFeatureScore<Self::Score>;

    fn stoppable_passer() -> SingleFeatureScore<Self::Score>;

    fn close_king_passer() -> SingleFeatureScore<Self::Score>;

    fn immobile_passer() -> SingleFeatureScore<Self::Score>;

    fn passer_protection() -> SingleFeatureScore<Self::Score>;

    fn candidate_passer(rank: DimT) -> SingleFeatureScore<Self::Score>;

    fn unsupported_pawn() -> SingleFeatureScore<Self::Score>;

    fn doubled_pawn() -> SingleFeatureScore<Self::Score>;

    fn phalanx(rank: DimT) -> SingleFeatureScore<Self::Score>;

    fn bishop_pair() -> SingleFeatureScore<Self::Score>;

    fn bad_bishop(num_pawns: usize) -> SingleFeatureScore<Self::Score>;

    fn rook_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score>;

    fn king_openness(openness: FileOpenness) -> SingleFeatureScore<Self::Score>;

    fn bishop_openness(openness: FileOpenness, len: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_advanced_center(config: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_passive_center(config: usize) -> SingleFeatureScore<Self::Score>;

    fn pawn_shield(&self, color: ChessColor, config: usize) -> SingleFeatureScore<Self::Score>;

    fn pawnless_flank() -> SingleFeatureScore<Self::Score>;

    fn pawn_protection(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn pawn_attack(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn pawn_advance_threat(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn mobility(piece: ChessPieceType, mobility: usize) -> SingleFeatureScore<Self::Score>;

    fn threats(attacking: ChessPieceType, targeted: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn king_zone_attack(attacking: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn extended_king_zone_attack() -> SingleFeatureScore<Self::Score>;

    fn can_give_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn pin(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn discovered_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;
}

/// Eval values tuned on a combination of the lichess-big-3-resolved dataset and a dataset used by 4ku,
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

    fn stoppable_passer() -> PhasedScore {
        STOPPABLE_PASSER
    }

    fn close_king_passer() -> SingleFeatureScore<Self::Score> {
        CLOSE_KING_PASSER
    }

    fn immobile_passer() -> SingleFeatureScore<Self::Score> {
        IMMOBILE_PASSER
    }

    fn passer_protection() -> SingleFeatureScore<Self::Score> {
        PROTECTED_PASSER
    }

    fn candidate_passer(rank: DimT) -> SingleFeatureScore<Self::Score> {
        CANDIDATE_PASSER[rank as usize]
    }

    fn unsupported_pawn() -> PhasedScore {
        UNSUPPORTED_PAWN
    }

    fn doubled_pawn() -> PhasedScore {
        DOUBLED_PAWN
    }

    fn phalanx(rank: DimT) -> PhasedScore {
        PHALANX[rank as usize]
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

    fn pawnless_flank() -> SingleFeatureScore<Self::Score> {
        PAWNLESS_FLANK
    }

    fn pawn_protection(piece: ChessPieceType) -> PhasedScore {
        PAWN_PROTECTION[piece as usize]
    }

    fn pawn_attack(piece: ChessPieceType) -> PhasedScore {
        PAWN_ATTACKS[piece as usize]
    }

    fn pawn_advance_threat(piece: ChessPieceType) -> SingleFeatureScore<Self::Score> {
        PAWN_ADVANCE_THREAT[piece as usize]
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

    fn extended_king_zone_attack() -> SingleFeatureScore<Self::Score> {
        EXTENDED_KING_ZONE_ATTACK
    }

    fn can_give_check(piece: ChessPieceType) -> PhasedScore {
        CAN_GIVE_CHECK[piece as usize]
    }

    fn pin(piece: ChessPieceType) -> SingleFeatureScore<Self::Score> {
        PIN[piece as usize]
    }

    fn discovered_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score> {
        DISCOVERED_CHECK[piece as usize]
    }
}
