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
        p( 126,  161),    p( 122,  159),    p( 114,  163),    p( 124,  147),    p( 110,  152),    p( 110,  155),    p(  68,  172),    p(  72,  171),
        p(  70,  117),    p(  67,  119),    p(  73,  112),    p(  80,  108),    p(  67,  102),    p( 121,  103),    p(  93,  121),    p(  93,  115),
        p(  53,  105),    p(  59,   98),    p(  56,   92),    p(  81,   96),    p(  88,   95),    p(  78,   82),    p(  73,   93),    p(  70,   89),
        p(  47,   92),    p(  48,   94),    p(  73,   90),    p(  93,   92),    p(  85,   95),    p(  81,   90),    p(  64,   85),    p(  57,   79),
        p(  41,   90),    p(  46,   86),    p(  68,   91),    p(  78,   93),    p(  80,   92),    p(  74,   90),    p(  64,   77),    p(  49,   79),
        p(  51,   94),    p(  55,   92),    p(  58,   92),    p(  53,   98),    p(  57,  100),    p(  73,   92),    p(  77,   82),    p(  56,   84),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 171,  279),    p( 189,  313),    p( 209,  324),    p( 245,  312),    p( 273,  315),    p( 191,  308),    p( 215,  308),    p( 199,  263),
        p( 266,  312),    p( 282,  317),    p( 295,  310),    p( 299,  313),    p( 297,  309),    p( 307,  299),    p( 271,  316),    p( 274,  303),
        p( 285,  309),    p( 304,  306),    p( 303,  312),    p( 314,  314),    p( 331,  309),    p( 346,  298),    p( 289,  306),    p( 283,  307),
        p( 303,  316),    p( 312,  313),    p( 326,  314),    p( 326,  321),    p( 323,  321),    p( 323,  319),    p( 315,  318),    p( 322,  310),
        p( 299,  317),    p( 305,  309),    p( 314,  315),    p( 322,  317),    p( 319,  321),    p( 326,  307),    p( 325,  306),    p( 314,  314),
        p( 277,  303),    p( 284,  304),    p( 298,  299),    p( 301,  312),    p( 306,  310),    p( 297,  293),    p( 303,  295),    p( 294,  308),
        p( 270,  310),    p( 281,  315),    p( 285,  306),    p( 295,  310),    p( 299,  306),    p( 291,  304),    p( 295,  308),    p( 289,  319),
        p( 241,  307),    p( 282,  305),    p( 268,  308),    p( 288,  312),    p( 297,  310),    p( 292,  301),    p( 288,  309),    p( 265,  307),
    ],
    // bishop
    [
        p( 274,  313),    p( 246,  317),    p( 235,  309),    p( 219,  318),    p( 212,  317),    p( 224,  309),    p( 268,  307),    p( 251,  310),
        p( 281,  304),    p( 276,  306),    p( 288,  308),    p( 273,  307),    p( 285,  303),    p( 289,  301),    p( 265,  310),    p( 272,  304),
        p( 294,  310),    p( 304,  307),    p( 287,  306),    p( 303,  301),    p( 299,  303),    p( 333,  306),    p( 312,  304),    p( 316,  314),
        p( 289,  311),    p( 297,  305),    p( 306,  302),    p( 305,  307),    p( 309,  303),    p( 304,  304),    p( 311,  304),    p( 284,  309),
        p( 288,  306),    p( 287,  307),    p( 297,  304),    p( 310,  303),    p( 302,  301),    p( 305,  300),    p( 288,  304),    p( 312,  299),
        p( 296,  308),    p( 299,  305),    p( 300,  306),    p( 299,  304),    p( 305,  307),    p( 299,  299),    p( 306,  295),    p( 307,  298),
        p( 306,  309),    p( 303,  301),    p( 306,  302),    p( 298,  310),    p( 300,  308),    p( 302,  306),    p( 311,  296),    p( 307,  295),
        p( 299,  305),    p( 309,  306),    p( 307,  308),    p( 289,  312),    p( 305,  310),    p( 293,  312),    p( 302,  300),    p( 301,  293),
    ],
    // rook
    [
        p( 450,  550),    p( 435,  561),    p( 427,  568),    p( 427,  564),    p( 440,  560),    p( 468,  554),    p( 472,  555),    p( 484,  547),
        p( 443,  555),    p( 440,  561),    p( 449,  562),    p( 463,  553),    p( 450,  555),    p( 465,  550),    p( 472,  547),    p( 490,  536),
        p( 443,  551),    p( 462,  546),    p( 456,  548),    p( 457,  542),    p( 481,  532),    p( 492,  529),    p( 507,  529),    p( 483,  531),
        p( 443,  551),    p( 453,  546),    p( 453,  548),    p( 454,  543),    p( 461,  536),    p( 475,  530),    p( 476,  536),    p( 470,  531),
        p( 435,  548),    p( 437,  547),    p( 438,  547),    p( 444,  543),    p( 448,  540),    p( 446,  540),    p( 458,  535),    p( 451,  531),
        p( 431,  545),    p( 433,  543),    p( 435,  542),    p( 437,  541),    p( 443,  535),    p( 454,  528),    p( 470,  518),    p( 455,  521),
        p( 433,  541),    p( 437,  540),    p( 443,  540),    p( 446,  537),    p( 452,  531),    p( 468,  521),    p( 474,  518),    p( 443,  527),
        p( 442,  545),    p( 439,  541),    p( 440,  544),    p( 445,  538),    p( 450,  531),    p( 456,  531),    p( 452,  532),    p( 448,  534),
    ],
    // queen
    [
        p( 865,  968),    p( 867,  981),    p( 880,  995),    p( 901,  988),    p( 901,  992),    p( 929,  979),    p( 966,  936),    p( 913,  967),
        p( 889,  953),    p( 865,  981),    p( 865, 1008),    p( 858, 1026),    p( 866, 1037),    p( 903, 1001),    p( 907,  983),    p( 947,  965),
        p( 893,  960),    p( 887,  973),    p( 885,  995),    p( 886, 1004),    p( 898, 1012),    p( 945,  992),    p( 949,  968),    p( 940,  974),
        p( 885,  968),    p( 892,  975),    p( 887,  982),    p( 880,  999),    p( 886, 1009),    p( 906,  995),    p( 915, 1001),    p( 919,  980),
        p( 891,  962),    p( 884,  976),    p( 885,  979),    p( 887,  992),    p( 890,  992),    p( 897,  989),    p( 905,  985),    p( 913,  976),
        p( 888,  948),    p( 894,  964),    p( 890,  976),    p( 886,  979),    p( 892,  987),    p( 900,  975),    p( 911,  963),    p( 908,  953),
        p( 886,  953),    p( 887,  960),    p( 895,  961),    p( 894,  975),    p( 896,  974),    p( 897,  957),    p( 908,  939),    p( 915,  913),
        p( 873,  954),    p( 885,  942),    p( 886,  954),    p( 894,  956),    p( 897,  946),    p( 883,  949),    p( 884,  942),    p( 891,  924),
    ],
    // king
    [
        p( 157,  -64),    p(  68,  -14),    p(  95,   -5),    p(  23,   15),    p(  43,    4),    p(  15,   16),    p(  78,    4),    p( 202,  -67),
        p( -20,   16),    p( -45,   31),    p( -49,   39),    p(  10,   24),    p( -19,   33),    p( -46,   46),    p( -25,   35),    p(   9,   18),
        p( -45,   23),    p( -29,   24),    p( -68,   37),    p( -77,   42),    p( -43,   38),    p( -14,   31),    p( -54,   33),    p( -24,   22),
        p( -24,    9),    p( -86,   18),    p(-101,   32),    p(-127,   38),    p(-125,   38),    p(-104,   30),    p(-113,   21),    p(-101,   23),
        p( -42,   -0),    p(-100,    7),    p(-116,   23),    p(-142,   33),    p(-139,   32),    p(-115,   18),    p(-129,   11),    p(-116,   15),
        p( -36,    3),    p( -80,    0),    p(-107,   13),    p(-115,   19),    p(-112,   20),    p(-121,   14),    p( -98,    2),    p( -72,   10),
        p(  24,   -6),    p( -67,   -5),    p( -78,    1),    p( -95,    8),    p(-102,   11),    p( -88,    3),    p( -61,  -11),    p(   1,   -3),
        p(  46,  -17),    p(  40,  -30),    p(  38,  -19),    p( -23,   -3),    p(  28,  -18),    p( -20,   -4),    p(  32,  -25),    p(  57,  -30),
    ],
];

const BISHOP_PAIR: PhasedScore = p(24, 51);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(10, 21), p(11, 18), p(11, 7), p(7, 0), p(2, -7), p(-1, -16), p(-7, -24), p(-14, -37), p(-24, -43)];
const ROOK_OPEN_FILE: PhasedScore = p(15, 2);
const ROOK_CLOSED_FILE: PhasedScore = p(-10, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(5, -1);
const KING_OPEN_FILE: PhasedScore = p(-43, 5);
const KING_CLOSED_FILE: PhasedScore = p(12, -7);
const KING_SEMIOPEN_FILE: PhasedScore = p(-9, 10);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-5, 5), p(0, 7), p(-2, 6), p(3, 4), p(3, 5), p(4, 7), p(7, 4), p(18, 1)],
    // Closed
    [p(0, 0), p(0, 0), p(19, -13), p(-13, 10), p(-1, 11), p(0, 5), p(-1, 7), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-13, 24), p(4, 17), p(1, 11), p(1, 11), p(5, 6), p(2, 3), p(10, 6)],
    // SemiClosed
    [p(0, 0), p(9, -10), p(6, 7), p(3, 0), p(6, 2), p(1, 4), p(3, 5), p(2, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(23, 12),
    p(1, 5),
    p(-4, 6),
    p(-17, 8),
    p(7, 3),
    p(-10, -11),
    p(-5, -2),
    p(-9, -13),
    p(3, 0),
    p(-12, -1),
    p(-13, -16),
    p(-25, -7),
    p(7, -8),
    p(-3, -12),
    p(6, -10),
    p(1, 7),
    p(-6, -3),
    p(-25, -5),
    p(-21, 0),
    p(-50, 21),
    p(-19, 2),
    p(-21, -20),
    p(4, 20),
    p(-56, 25),
    p(-18, -18),
    p(-24, -17),
    p(-40, -32),
    p(-46, 12),
    p(-14, -5),
    p(10, -7),
    p(-102, 111),
    p(0, 0),
    p(-1, -3),
    p(-17, -5),
    p(-11, -5),
    p(-32, -1),
    p(-23, -3),
    p(-49, -23),
    p(-35, 35),
    p(-50, 22),
    p(-7, -5),
    p(-24, -10),
    p(-0, -11),
    p(-24, 32),
    p(-46, 15),
    p(-3, -36),
    p(0, 0),
    p(0, 0),
    p(-3, -15),
    p(-20, 4),
    p(-18, -57),
    p(0, 0),
    p(6, -18),
    p(-41, -19),
    p(0, 0),
    p(0, 0),
    p(-31, -7),
    p(-24, -15),
    p(-27, 17),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(27, 4),
    p(3, -2),
    p(-5, 0),
    p(-23, -4),
    p(6, -4),
    p(-31, -8),
    p(-21, -4),
    p(-42, -14),
    p(7, -3),
    p(-14, -8),
    p(-32, -10),
    p(-51, -3),
    p(-11, -5),
    p(-50, -6),
    p(-44, -16),
    p(-65, 57),
    p(10, -2),
    p(-3, -9),
    p(-7, -13),
    p(-28, -5),
    p(-13, -3),
    p(-23, -12),
    p(-31, -4),
    p(-86, 177),
    p(-8, -12),
    p(-31, -15),
    p(-42, -32),
    p(7, -87),
    p(-21, -10),
    p(-24, -19),
    p(-87, 60),
    p(0, 0),
    p(16, -1),
    p(0, -5),
    p(-15, -10),
    p(-25, -11),
    p(-2, -1),
    p(-32, -15),
    p(-20, -2),
    p(-36, 0),
    p(-1, -9),
    p(-25, -9),
    p(-29, -19),
    p(-42, -11),
    p(-13, -5),
    p(-51, -11),
    p(-17, 19),
    p(-68, 54),
    p(3, -3),
    p(-14, -5),
    p(-31, 53),
    p(0, 0),
    p(-21, -5),
    p(-29, 0),
    p(0, 0),
    p(0, 0),
    p(-18, -2),
    p(-45, 6),
    p(-40, -41),
    p(0, 0),
    p(-1, -60),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-7, 9),    /*0b0000*/
    p(-14, 8),   /*0b0001*/
    p(-4, 11),   /*0b0010*/
    p(-8, 12),   /*0b0011*/
    p(-3, 4),    /*0b0100*/
    p(-27, 2),   /*0b0101*/
    p(-12, 5),   /*0b0110*/
    p(-18, -12), /*0b0111*/
    p(11, 6),    /*0b1000*/
    p(-2, 11),   /*0b1001*/
    p(2, 11),    /*0b1010*/
    p(1, 12),    /*0b1011*/
    p(1, 5),     /*0b1100*/
    p(-26, 7),   /*0b1101*/
    p(-8, 5),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(7, 11),    /*0b10000*/
    p(5, 8),     /*0b10001*/
    p(18, 11),   /*0b10010*/
    p(-2, 7),    /*0b10011*/
    p(-3, 4),    /*0b10100*/
    p(13, 12),   /*0b10101*/
    p(-19, 2),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(16, 11),   /*0b11000*/
    p(27, 15),   /*0b11001*/
    p(37, 25),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(14, -0),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(19, 4),    /*0b100000*/
    p(5, 11),    /*0b100001*/
    p(23, 4),    /*0b100010*/
    p(9, -0),    /*0b100011*/
    p(-6, 1),    /*0b100100*/
    p(-23, -6),  /*0b100101*/
    p(-22, 19),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(29, -1),   /*0b101000*/
    p(-0, 14),   /*0b101001*/
    p(20, -4),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-1, 6),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(16, 4),    /*0b110000*/
    p(22, 4),    /*0b110001*/
    p(30, -3),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(1, 17),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(30, -3),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(8, -3),    /*0b111111*/
    p(-12, 4),   /*0b00*/
    p(4, -12),   /*0b01*/
    p(35, -8),   /*0b10*/
    p(23, -42),  /*0b11*/
    p(45, -10),  /*0b100*/
    p(-11, -9),  /*0b101*/
    p(64, -41),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(66, -15),  /*0b1000*/
    p(16, -29),  /*0b1001*/
    p(75, -57),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(61, -36),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(20, 0),    /*0b1111*/
    p(20, 1),    /*0b00*/
    p(32, -10),  /*0b01*/
    p(25, -14),  /*0b10*/
    p(22, -38),  /*0b11*/
    p(38, -9),   /*0b100*/
    p(53, -19),  /*0b101*/
    p(23, -20),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(39, -2),   /*0b1000*/
    p(50, -16),  /*0b1001*/
    p(52, -37),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(42, -25),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(21, -43),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(-25, -33);
const STOPPABLE_PASSER: PhasedScore = p(36, -49);
const CLOSE_KING_PASSER: PhasedScore = p(-8, 28);
const IMMOBILE_PASSER: PhasedScore = p(-3, -36);
const PROTECTED_PASSER: PhasedScore = p(8, -4);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -41,   41),    p( -48,   55),    p( -67,   58),    p( -68,   51),    p( -59,   39),    p( -47,   40),    p( -31,   45),    p( -47,   46),
        p( -31,   36),    p( -54,   58),    p( -62,   55),    p( -62,   48),    p( -69,   48),    p( -57,   48),    p( -58,   63),    p( -42,   45),
        p( -27,   55),    p( -32,   57),    p( -58,   62),    p( -56,   62),    p( -63,   58),    p( -52,   59),    p( -58,   70),    p( -59,   67),
        p( -15,   72),    p( -17,   75),    p( -17,   66),    p( -40,   75),    p( -57,   76),    p( -45,   75),    p( -52,   84),    p( -59,   86),
        p(   2,   71),    p(   8,   70),    p(   2,   54),    p( -18,   40),    p( -14,   51),    p( -37,   58),    p( -45,   60),    p( -82,   81),
        p(  26,   61),    p(  22,   59),    p(  14,   63),    p(  24,   47),    p(  10,   52),    p(  10,   55),    p( -32,   72),    p( -28,   71),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(1, 4), p(5, 8), p(8, 17), p(16, 22), p(16, 68), p(12, 60)];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -6);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);
const PHALANX: [PhasedScore; 6] = [p(0, -0), p(5, 3), p(9, 6), p(21, 21), p(59, 76), p(-99, 222)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 14), p(7, 16), p(14, 20), p(9, 9), p(-3, 17), p(-44, 11)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(40, 10), p(42, 33), p(54, -11), p(38, -42), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, -5), p(14, 21), p(18, -7), p(16, 9), p(17, -10), p(26, -11)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-44, -68),
        p(-24, -29),
        p(-12, -6),
        p(-3, 7),
        p(4, 17),
        p(11, 27),
        p(18, 30),
        p(25, 32),
        p(28, 31),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-7, -23),
        p(-0, -10),
        p(7, -0),
        p(12, 8),
        p(17, 13),
        p(22, 17),
        p(24, 23),
        p(31, 24),
        p(37, 24),
        p(43, 27),
        p(38, 37),
        p(52, 29),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-46, 59),
        p(-43, 64),
        p(-40, 68),
        p(-36, 71),
        p(-35, 77),
        p(-27, 78),
        p(-18, 77),
        p(-12, 74),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-27, -36),
        p(-25, 5),
        p(-30, 59),
        p(-25, 77),
        p(-23, 96),
        p(-18, 102),
        p(-15, 113),
        p(-11, 120),
        p(-7, 124),
        p(-4, 126),
        p(-1, 129),
        p(3, 132),
        p(6, 132),
        p(7, 138),
        p(10, 140),
        p(13, 144),
        p(14, 151),
        p(17, 153),
        p(26, 151),
        p(41, 146),
        p(44, 149),
        p(87, 126),
        p(87, 129),
        p(110, 112),
        p(199, 81),
        p(244, 42),
        p(287, 22),
        p(319, -4),
    ],
    [
        p(-83, 3),
        p(-51, -9),
        p(-26, -8),
        p(1, -5),
        p(29, -3),
        p(48, -1),
        p(75, 4),
        p(96, 7),
        p(136, -2),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-5, 12), p(0, 0), p(23, 19), p(50, -10), p(22, -31), p(0, 0)],
    [p(-2, 10), p(19, 21), p(0, 0), p(31, 6), p(30, 53), p(0, 0)],
    [p(-3, 14), p(11, 13), p(17, 9), p(0, 0), p(42, 4), p(0, 0)],
    [p(-2, 5), p(2, 4), p(-0, 18), p(0, -1), p(0, 0), p(0, 0)],
    [p(57, 19), p(-28, 19), p(5, 15), p(-18, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 7), p(8, 6), p(6, 10), p(12, 6), p(6, 17), p(10, 5)],
    [p(2, 8), p(11, 20), p(-86, -35), p(8, 13), p(10, 20), p(3, 6)],
    [p(2, 3), p(12, 6), p(9, 12), p(11, 9), p(9, 26), p(20, -4)],
    [p(2, 1), p(8, 1), p(6, -4), p(4, 13), p(-77, -235), p(4, -9)],
    [p(59, -2), p(38, 10), p(43, 4), p(21, 7), p(32, -3), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-19, -13), p(20, -9), p(9, -3), p(14, -11), p(-1, 12), p(1, 3)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(13, 20), p(33, 1), p(5, 32)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(5, -18), p(24, 30), p(15, 36), p(40, 23), p(68, 31)];
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(-7, -18), p(25, -26), p(56, -51), p(29, 46), p(0, 0), p(-60, -42)];
const DISCOVERED_CHECK_STM: PhasedScore = p(224, 79);

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

    fn can_give_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn discovered_check_stm() -> SingleFeatureScore<Self::Score>;

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

    fn pawnless_flank() -> PhasedScore {
        PAWNLESS_FLANK
    }

    fn pawn_protection(piece: ChessPieceType) -> PhasedScore {
        PAWN_PROTECTION[piece as usize]
    }

    fn pawn_attack(piece: ChessPieceType) -> PhasedScore {
        PAWN_ATTACKS[piece as usize]
    }

    fn pawn_advance_threat(piece: ChessPieceType) -> PhasedScore {
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

    fn can_give_check(piece: ChessPieceType) -> PhasedScore {
        CAN_GIVE_CHECK[piece as usize]
    }

    fn discovered_check_stm() -> PhasedScore {
        DISCOVERED_CHECK_STM
    }

    fn pin(piece: ChessPieceType) -> PhasedScore {
        PIN[piece as usize]
    }

    fn discovered_check(piece: ChessPieceType) -> PhasedScore {
        DISCOVERED_CHECK[piece as usize]
    }
}
