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
        p( 127,  160),    p( 122,  159),    p( 115,  163),    p( 125,  147),    p( 111,  152),    p( 110,  155),    p(  69,  172),    p(  74,  170),
        p(  70,  116),    p(  68,  118),    p(  73,  111),    p(  81,  108),    p(  67,  102),    p( 121,  102),    p(  93,  121),    p(  93,  114),
        p(  53,  105),    p(  61,   98),    p(  57,   92),    p(  82,   96),    p(  88,   95),    p(  81,   82),    p(  74,   93),    p(  71,   89),
        p(  48,   91),    p(  51,   92),    p(  75,   90),    p(  94,   92),    p(  88,   94),    p(  83,   90),    p(  68,   83),    p(  59,   78),
        p(  41,   90),    p(  47,   85),    p(  69,   90),    p(  79,   93),    p(  80,   91),    p(  75,   90),    p(  66,   76),    p(  50,   78),
        p(  52,   93),    p(  56,   92),    p(  58,   92),    p(  53,   98),    p(  57,  100),    p(  73,   92),    p(  78,   81),    p(  57,   83),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 173,  282),    p( 192,  315),    p( 211,  326),    p( 251,  314),    p( 279,  316),    p( 198,  310),    p( 215,  310),    p( 201,  266),
        p( 266,  315),    p( 283,  319),    p( 298,  311),    p( 302,  314),    p( 301,  310),    p( 310,  299),    p( 271,  317),    p( 273,  306),
        p( 285,  311),    p( 305,  305),    p( 307,  311),    p( 321,  315),    p( 336,  310),    p( 350,  297),    p( 290,  306),    p( 287,  309),
        p( 301,  317),    p( 308,  310),    p( 325,  314),    p( 325,  321),    p( 325,  318),    p( 320,  317),    p( 311,  313),    p( 319,  311),
        p( 298,  318),    p( 303,  308),    p( 313,  313),    p( 320,  315),    p( 319,  319),    p( 324,  304),    p( 322,  304),    p( 312,  314),
        p( 275,  304),    p( 282,  303),    p( 297,  297),    p( 301,  310),    p( 306,  308),    p( 296,  291),    p( 302,  295),    p( 293,  309),
        p( 270,  312),    p( 281,  316),    p( 285,  305),    p( 294,  310),    p( 299,  305),    p( 290,  304),    p( 295,  309),    p( 289,  321),
        p( 241,  310),    p( 281,  307),    p( 267,  309),    p( 287,  313),    p( 296,  312),    p( 291,  302),    p( 288,  311),    p( 265,  309),
    ],
    // bishop
    [
        p( 275,  312),    p( 249,  317),    p( 236,  309),    p( 221,  318),    p( 213,  317),    p( 227,  308),    p( 273,  306),    p( 251,  310),
        p( 282,  304),    p( 278,  305),    p( 290,  307),    p( 276,  306),    p( 288,  303),    p( 291,  301),    p( 264,  310),    p( 273,  304),
        p( 295,  310),    p( 307,  306),    p( 289,  305),    p( 306,  300),    p( 302,  302),    p( 335,  305),    p( 315,  303),    p( 318,  313),
        p( 287,  311),    p( 289,  308),    p( 303,  302),    p( 305,  307),    p( 306,  303),    p( 298,  305),    p( 298,  308),    p( 280,  311),
        p( 288,  306),    p( 284,  308),    p( 294,  305),    p( 308,  304),    p( 302,  300),    p( 299,  302),    p( 285,  305),    p( 310,  300),
        p( 296,  308),    p( 298,  305),    p( 299,  307),    p( 299,  304),    p( 305,  307),    p( 297,  299),    p( 305,  295),    p( 307,  298),
        p( 307,  308),    p( 303,  301),    p( 308,  302),    p( 299,  309),    p( 301,  307),    p( 303,  306),    p( 311,  296),    p( 308,  295),
        p( 299,  305),    p( 309,  306),    p( 307,  308),    p( 290,  311),    p( 306,  310),    p( 294,  312),    p( 302,  300),    p( 301,  293),
    ],
    // rook
    [
        p( 450,  549),    p( 436,  560),    p( 427,  567),    p( 427,  564),    p( 440,  560),    p( 467,  554),    p( 474,  554),    p( 484,  547),
        p( 444,  554),    p( 441,  560),    p( 450,  561),    p( 464,  552),    p( 450,  555),    p( 465,  549),    p( 473,  546),    p( 490,  536),
        p( 443,  550),    p( 462,  545),    p( 456,  547),    p( 457,  542),    p( 482,  532),    p( 492,  528),    p( 508,  528),    p( 484,  531),
        p( 440,  550),    p( 446,  546),    p( 446,  548),    p( 451,  543),    p( 455,  535),    p( 466,  530),    p( 465,  535),    p( 466,  530),
        p( 433,  547),    p( 433,  546),    p( 434,  546),    p( 439,  542),    p( 446,  538),    p( 441,  537),    p( 453,  533),    p( 448,  530),
        p( 430,  544),    p( 430,  542),    p( 432,  541),    p( 435,  540),    p( 441,  534),    p( 452,  526),    p( 468,  517),    p( 454,  520),
        p( 432,  541),    p( 437,  539),    p( 443,  540),    p( 446,  536),    p( 452,  530),    p( 467,  520),    p( 474,  517),    p( 443,  526),
        p( 442,  545),    p( 439,  540),    p( 440,  544),    p( 445,  537),    p( 450,  531),    p( 456,  531),    p( 452,  531),    p( 448,  534),
    ],
    // queen
    [
        p( 865,  967),    p( 867,  981),    p( 880,  994),    p( 903,  987),    p( 903,  991),    p( 928,  979),    p( 967,  936),    p( 914,  966),
        p( 889,  953),    p( 866,  979),    p( 867, 1006),    p( 859, 1024),    p( 867, 1035),    p( 904,  999),    p( 908,  982),    p( 947,  965),
        p( 894,  959),    p( 888,  972),    p( 886,  993),    p( 888, 1002),    p( 899, 1010),    p( 947,  990),    p( 950,  966),    p( 941,  972),
        p( 881,  970),    p( 884,  980),    p( 881,  985),    p( 877, 1000),    p( 881, 1010),    p( 897, 1000),    p( 904, 1007),    p( 912,  984),
        p( 889,  963),    p( 879,  979),    p( 883,  979),    p( 885,  992),    p( 889,  991),    p( 891,  991),    p( 900,  987),    p( 909,  977),
        p( 887,  948),    p( 892,  964),    p( 888,  977),    p( 885,  979),    p( 890,  987),    p( 898,  976),    p( 910,  964),    p( 908,  952),
        p( 886,  953),    p( 888,  959),    p( 895,  960),    p( 895,  974),    p( 896,  974),    p( 897,  957),    p( 908,  938),    p( 916,  912),
        p( 874,  953),    p( 885,  942),    p( 886,  954),    p( 894,  955),    p( 897,  945),    p( 883,  949),    p( 884,  943),    p( 891,  925),
    ],
    // king
    [
        p( 160,  -67),    p(  68,  -15),    p(  95,   -7),    p(  23,   14),    p(  39,    4),    p(  13,   16),    p(  79,    3),    p( 200,  -68),
        p( -22,   16),    p( -49,   32),    p( -52,   40),    p(   7,   25),    p( -23,   34),    p( -50,   47),    p( -29,   36),    p(   5,   18),
        p( -45,   22),    p( -31,   24),    p( -71,   38),    p( -80,   43),    p( -47,   39),    p( -18,   32),    p( -58,   34),    p( -29,   22),
        p( -25,    8),    p( -92,   19),    p(-104,   33),    p(-130,   38),    p(-130,   39),    p(-109,   31),    p(-122,   23),    p(-105,   23),
        p( -43,   -1),    p(-105,    9),    p(-119,   24),    p(-146,   34),    p(-143,   33),    p(-121,   20),    p(-135,   13),    p(-120,   15),
        p( -37,    2),    p( -84,    2),    p(-110,   14),    p(-119,   20),    p(-116,   21),    p(-125,   15),    p(-101,    4),    p( -73,   10),
        p(  24,   -7),    p( -69,   -4),    p( -80,    2),    p( -98,    8),    p(-104,   12),    p( -91,    4),    p( -64,  -10),    p(   1,   -3),
        p(  48,  -19),    p(  41,  -32),    p(  38,  -20),    p( -22,   -5),    p(  29,  -19),    p( -19,   -5),    p(  33,  -26),    p(  59,  -32),
    ],
];

const BISHOP_PAIR: PhasedScore = p(24, 51);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(9, 21), p(11, 18), p(10, 8), p(6, 0), p(2, -7), p(-2, -16), p(-8, -24), p(-15, -38), p(-26, -45)];
const ROOK_OPEN_FILE: PhasedScore = p(15, 2);
const ROOK_CLOSED_FILE: PhasedScore = p(-10, -2);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(6, -1);
const KING_OPEN_FILE: PhasedScore = p(-43, 5);
const KING_CLOSED_FILE: PhasedScore = p(12, -7);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 9);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-5, 5), p(1, 7), p(-2, 6), p(2, 4), p(2, 6), p(3, 8), p(6, 4), p(19, 2)],
    // Closed
    [p(0, 0), p(0, 0), p(19, -15), p(-14, 10), p(-1, 11), p(-0, 4), p(-2, 7), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-12, 23), p(3, 16), p(0, 10), p(-1, 10), p(4, 6), p(0, 3), p(10, 6)],
    // SemiClosed
    [p(0, 0), p(12, -12), p(7, 6), p(3, -0), p(6, 1), p(0, 4), p(3, 5), p(2, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(22, 12),
    p(1, 5),
    p(-3, 6),
    p(-14, 6),
    p(7, 3),
    p(-10, -11),
    p(-2, -3),
    p(-7, -14),
    p(2, 0),
    p(-11, -1),
    p(-12, -16),
    p(-22, -9),
    p(7, -9),
    p(-2, -13),
    p(6, -9),
    p(1, 6),
    p(-5, -3),
    p(-25, -5),
    p(-21, 2),
    p(-51, 20),
    p(-19, 1),
    p(-21, -23),
    p(3, 20),
    p(-61, 26),
    p(-17, -18),
    p(-24, -17),
    p(-41, -31),
    p(-45, 10),
    p(-17, -5),
    p(8, -8),
    p(-97, 106),
    p(0, 0),
    p(-1, -2),
    p(-16, -5),
    p(-10, -5),
    p(-31, -3),
    p(-26, -2),
    p(-51, -22),
    p(-39, 36),
    p(-50, 23),
    p(-8, -5),
    p(-24, -10),
    p(-0, -13),
    p(-24, 31),
    p(-50, 15),
    p(-9, -34),
    p(0, 0),
    p(0, 0),
    p(-3, -16),
    p(-19, 4),
    p(-19, -57),
    p(0, 0),
    p(1, -17),
    p(-48, -10),
    p(0, 0),
    p(0, 0),
    p(-32, -6),
    p(-25, -14),
    p(-24, 16),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(25, 4),
    p(2, -2),
    p(-6, 0),
    p(-23, -4),
    p(6, -4),
    p(-31, -9),
    p(-21, -5),
    p(-42, -14),
    p(7, -3),
    p(-13, -8),
    p(-31, -10),
    p(-48, -3),
    p(-10, -5),
    p(-48, -6),
    p(-40, -15),
    p(-60, 58),
    p(10, -3),
    p(-4, -9),
    p(-7, -13),
    p(-29, -6),
    p(-13, -4),
    p(-23, -14),
    p(-30, -7),
    p(-87, 178),
    p(-8, -13),
    p(-32, -16),
    p(-42, -31),
    p(0, -84),
    p(-21, -11),
    p(-25, -19),
    p(-88, 58),
    p(0, 0),
    p(16, -1),
    p(1, -4),
    p(-14, -11),
    p(-24, -12),
    p(-1, -0),
    p(-30, -15),
    p(-20, -3),
    p(-36, -1),
    p(-1, -9),
    p(-24, -10),
    p(-30, -20),
    p(-43, -12),
    p(-13, -4),
    p(-51, -13),
    p(-19, 17),
    p(-71, 56),
    p(6, -5),
    p(-12, -5),
    p(-28, 51),
    p(0, 0),
    p(-19, -7),
    p(-26, 2),
    p(0, 0),
    p(0, 0),
    p(-17, -1),
    p(-45, 8),
    p(-40, -41),
    p(0, 0),
    p(0, -64),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-8, 9),    /*0b0000*/
    p(-14, 8),   /*0b0001*/
    p(-4, 11),   /*0b0010*/
    p(-8, 12),   /*0b0011*/
    p(-3, 4),    /*0b0100*/
    p(-27, 3),   /*0b0101*/
    p(-11, 5),   /*0b0110*/
    p(-18, -11), /*0b0111*/
    p(11, 6),    /*0b1000*/
    p(-2, 11),   /*0b1001*/
    p(2, 10),    /*0b1010*/
    p(0, 12),    /*0b1011*/
    p(1, 4),     /*0b1100*/
    p(-26, 6),   /*0b1101*/
    p(-8, 5),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(7, 11),    /*0b10000*/
    p(5, 8),     /*0b10001*/
    p(19, 10),   /*0b10010*/
    p(-3, 8),    /*0b10011*/
    p(-3, 4),    /*0b10100*/
    p(13, 12),   /*0b10101*/
    p(-19, 2),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(17, 11),   /*0b11000*/
    p(26, 15),   /*0b11001*/
    p(37, 26),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(14, 0),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(19, 4),    /*0b100000*/
    p(5, 10),    /*0b100001*/
    p(23, 3),    /*0b100010*/
    p(8, 1),     /*0b100011*/
    p(-6, 2),    /*0b100100*/
    p(-24, -6),  /*0b100101*/
    p(-22, 19),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(29, -1),   /*0b101000*/
    p(-0, 14),   /*0b101001*/
    p(21, -5),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-2, 7),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(16, 4),    /*0b110000*/
    p(22, 5),    /*0b110001*/
    p(29, -2),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(1, 18),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(29, -2),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(8, -2),    /*0b111111*/
    p(-13, 3),   /*0b00*/
    p(5, -13),   /*0b01*/
    p(36, -8),   /*0b10*/
    p(23, -43),  /*0b11*/
    p(44, -10),  /*0b100*/
    p(-7, -9),   /*0b101*/
    p(66, -42),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(65, -15),  /*0b1000*/
    p(17, -30),  /*0b1001*/
    p(76, -58),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(60, -37),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(13, 5),    /*0b1111*/
    p(21, 0),    /*0b00*/
    p(33, -11),  /*0b01*/
    p(26, -15),  /*0b10*/
    p(23, -39),  /*0b11*/
    p(40, -10),  /*0b100*/
    p(53, -20),  /*0b101*/
    p(25, -21),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(40, -3),   /*0b1000*/
    p(51, -17),  /*0b1001*/
    p(53, -38),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(43, -27),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(22, -43),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(-23, -34);
const STOPPABLE_PASSER: PhasedScore = p(37, -49);
const CLOSE_KING_PASSER: PhasedScore = p(-8, 28);
const IMMOBILE_PASSER: PhasedScore = p(-9, -35);
const PROTECTED_PASSER: PhasedScore = p(9, -4);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -41,   41),    p( -47,   54),    p( -67,   58),    p( -69,   51),    p( -59,   39),    p( -48,   39),    p( -31,   45),    p( -47,   46),
        p( -30,   36),    p( -53,   58),    p( -62,   54),    p( -63,   48),    p( -70,   48),    p( -57,   47),    p( -58,   63),    p( -42,   45),
        p( -25,   54),    p( -32,   57),    p( -59,   62),    p( -57,   62),    p( -65,   59),    p( -52,   58),    p( -59,   70),    p( -59,   66),
        p( -14,   71),    p( -18,   75),    p( -15,   65),    p( -39,   74),    p( -56,   75),    p( -43,   74),    p( -52,   84),    p( -59,   85),
        p(  -5,   72),    p(   5,   71),    p(  -0,   54),    p( -20,   40),    p( -15,   51),    p( -37,   58),    p( -47,   60),    p( -85,   81),
        p(  27,   60),    p(  22,   59),    p(  15,   63),    p(  25,   47),    p(  11,   52),    p(  10,   55),    p( -31,   72),    p( -26,   70),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(1, 5), p(5, 9), p(8, 18), p(18, 23), p(26, 67), p(13, 60)];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -6);
const DOUBLED_PAWN: PhasedScore = p(-7, -21);
const PHALANX: [PhasedScore; 6] = [p(0, -0), p(6, 2), p(9, 5), p(23, 19), p(62, 75), p(-101, 222)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 12), p(7, 15), p(15, 19), p(9, 8), p(-3, 17), p(-45, 10)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(38, 8), p(40, 32), p(51, -12), p(35, -41), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-45, -73),
        p(-25, -32),
        p(-13, -8),
        p(-4, 5),
        p(3, 16),
        p(10, 27),
        p(18, 31),
        p(24, 34),
        p(28, 33),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-32, -56),
        p(-19, -38),
        p(-8, -23),
        p(-0, -10),
        p(6, -1),
        p(12, 8),
        p(16, 13),
        p(21, 17),
        p(22, 23),
        p(29, 24),
        p(34, 25),
        p(41, 28),
        p(35, 38),
        p(49, 30),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-74, 13),
        p(-65, 26),
        p(-61, 32),
        p(-58, 37),
        p(-59, 44),
        p(-54, 50),
        p(-51, 55),
        p(-47, 59),
        p(-44, 64),
        p(-41, 68),
        p(-37, 71),
        p(-36, 76),
        p(-29, 78),
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
        p(-26, -48),
        p(-25, 2),
        p(-30, 57),
        p(-26, 75),
        p(-24, 94),
        p(-19, 100),
        p(-16, 112),
        p(-13, 118),
        p(-9, 123),
        p(-6, 124),
        p(-3, 128),
        p(1, 131),
        p(3, 132),
        p(5, 137),
        p(7, 139),
        p(11, 144),
        p(11, 151),
        p(14, 153),
        p(23, 151),
        p(37, 146),
        p(41, 149),
        p(84, 126),
        p(84, 130),
        p(105, 114),
        p(196, 82),
        p(243, 41),
        p(284, 23),
        p(326, -9),
    ],
    [
        p(-86, 6),
        p(-53, -7),
        p(-26, -7),
        p(1, -4),
        p(30, -3),
        p(51, -2),
        p(78, 3),
        p(99, 5),
        p(141, -4),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-9, 7), p(0, 0), p(23, 20), p(49, -10), p(20, -32), p(0, 0)],
    [p(-2, 11), p(19, 21), p(0, 0), p(30, 5), p(30, 52), p(0, 0)],
    [p(-3, 14), p(10, 12), p(18, 8), p(0, 0), p(41, 4), p(0, 0)],
    [p(-2, 5), p(2, 4), p(-0, 17), p(0, -1), p(0, 0), p(0, 0)],
    [p(60, 17), p(-28, 17), p(4, 15), p(-19, 8), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 8), p(8, 7), p(6, 10), p(12, 7), p(6, 19), p(10, 6)],
    [p(2, 8), p(11, 20), p(-120, -24), p(8, 13), p(10, 20), p(3, 6)],
    [p(2, 3), p(12, 7), p(8, 12), p(11, 10), p(9, 27), p(20, -3)],
    [p(1, 1), p(8, 1), p(6, -4), p(4, 13), p(-75, -238), p(4, -9)],
    [p(60, -3), p(39, 9), p(44, 3), p(22, 6), p(33, -3), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-18, -14), p(19, -9), p(9, -3), p(13, -11), p(-2, 12), p(1, 3)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(13, 20), p(33, 2), p(5, 32)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(5, -17), p(23, 30), p(15, 36), p(41, 23), p(68, 33)];
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(-7, -18), p(105, 36), p(148, -7), p(57, 145), p(0, 0), p(32, 14)];

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

    fn mobility(piece: ChessPieceType, mobility: usize) -> SingleFeatureScore<Self::Score>;

    fn threats(attacking: ChessPieceType, targeted: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn king_zone_attack(attacking: ChessPieceType) -> SingleFeatureScore<Self::Score>;

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

    fn pin(piece: ChessPieceType) -> SingleFeatureScore<Self::Score> {
        PIN[piece as usize]
    }

    fn discovered_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score> {
        DISCOVERED_CHECK[piece as usize]
    }
}
