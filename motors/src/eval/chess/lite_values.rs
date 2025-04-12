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
        p( 127,  161),    p( 122,  159),    p( 115,  163),    p( 125,  147),    p( 111,  151),    p( 110,  155),    p(  69,  172),    p(  74,  171),
        p(  70,  116),    p(  68,  118),    p(  73,  111),    p(  80,  108),    p(  67,  103),    p( 121,  103),    p(  92,  122),    p(  93,  114),
        p(  53,  105),    p(  61,   98),    p(  57,   93),    p(  82,   96),    p(  88,   95),    p(  81,   82),    p(  74,   93),    p(  71,   89),
        p(  48,   91),    p(  51,   92),    p(  75,   90),    p(  94,   92),    p(  88,   94),    p(  83,   90),    p(  68,   83),    p(  59,   78),
        p(  41,   90),    p(  47,   85),    p(  69,   90),    p(  79,   93),    p(  80,   91),    p(  75,   90),    p(  66,   76),    p(  50,   78),
        p(  52,   93),    p(  56,   92),    p(  58,   92),    p(  54,   98),    p(  57,   99),    p(  74,   91),    p(  79,   81),    p(  57,   83),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 173,  281),    p( 191,  314),    p( 213,  326),    p( 251,  314),    p( 277,  316),    p( 197,  310),    p( 220,  309),    p( 201,  265),
        p( 266,  314),    p( 283,  319),    p( 298,  311),    p( 301,  314),    p( 301,  310),    p( 310,  300),    p( 273,  317),    p( 274,  306),
        p( 285,  311),    p( 305,  305),    p( 307,  311),    p( 321,  315),    p( 337,  309),    p( 350,  297),    p( 291,  306),    p( 287,  309),
        p( 301,  316),    p( 309,  310),    p( 325,  313),    p( 326,  321),    p( 325,  318),    p( 320,  317),    p( 311,  313),    p( 319,  312),
        p( 299,  318),    p( 303,  307),    p( 313,  312),    p( 321,  315),    p( 319,  319),    p( 325,  304),    p( 322,  304),    p( 312,  314),
        p( 275,  304),    p( 283,  303),    p( 297,  297),    p( 302,  310),    p( 306,  308),    p( 296,  291),    p( 302,  295),    p( 293,  308),
        p( 270,  311),    p( 281,  315),    p( 285,  305),    p( 294,  310),    p( 299,  305),    p( 290,  304),    p( 295,  309),    p( 289,  322),
        p( 241,  309),    p( 281,  306),    p( 267,  309),    p( 288,  313),    p( 296,  311),    p( 291,  302),    p( 288,  310),    p( 264,  310),
    ],
    // bishop
    [
        p( 275,  312),    p( 248,  316),    p( 236,  309),    p( 222,  318),    p( 215,  316),    p( 226,  308),    p( 269,  306),    p( 252,  310),
        p( 283,  304),    p( 278,  305),    p( 290,  307),    p( 275,  306),    p( 289,  302),    p( 292,  301),    p( 266,  309),    p( 273,  303),
        p( 295,  309),    p( 307,  305),    p( 289,  305),    p( 306,  300),    p( 302,  302),    p( 335,  305),    p( 315,  303),    p( 318,  313),
        p( 287,  312),    p( 289,  308),    p( 303,  302),    p( 305,  307),    p( 306,  303),    p( 298,  305),    p( 298,  308),    p( 280,  311),
        p( 288,  306),    p( 284,  308),    p( 294,  305),    p( 308,  304),    p( 302,  300),    p( 299,  302),    p( 285,  305),    p( 310,  300),
        p( 296,  308),    p( 298,  305),    p( 300,  306),    p( 299,  304),    p( 305,  306),    p( 298,  299),    p( 305,  296),    p( 307,  298),
        p( 307,  308),    p( 304,  301),    p( 308,  301),    p( 299,  309),    p( 301,  307),    p( 303,  306),    p( 311,  296),    p( 308,  295),
        p( 299,  305),    p( 309,  307),    p( 308,  308),    p( 290,  311),    p( 306,  310),    p( 294,  312),    p( 302,  301),    p( 301,  293),
    ],
    // rook
    [
        p( 451,  548),    p( 436,  560),    p( 427,  567),    p( 427,  564),    p( 440,  559),    p( 468,  553),    p( 474,  553),    p( 486,  546),
        p( 444,  554),    p( 441,  560),    p( 450,  561),    p( 464,  552),    p( 451,  554),    p( 465,  549),    p( 473,  546),    p( 491,  535),
        p( 444,  550),    p( 463,  545),    p( 457,  546),    p( 458,  542),    p( 482,  531),    p( 493,  528),    p( 508,  528),    p( 485,  530),
        p( 440,  550),    p( 446,  546),    p( 446,  548),    p( 451,  543),    p( 455,  534),    p( 466,  530),    p( 466,  535),    p( 466,  530),
        p( 434,  547),    p( 433,  545),    p( 434,  545),    p( 440,  542),    p( 446,  538),    p( 441,  537),    p( 454,  533),    p( 448,  530),
        p( 431,  544),    p( 431,  541),    p( 433,  540),    p( 435,  539),    p( 441,  534),    p( 453,  526),    p( 468,  517),    p( 454,  520),
        p( 433,  540),    p( 437,  539),    p( 443,  539),    p( 446,  536),    p( 452,  530),    p( 468,  521),    p( 474,  517),    p( 443,  526),
        p( 442,  545),    p( 439,  540),    p( 440,  543),    p( 445,  537),    p( 450,  530),    p( 456,  531),    p( 452,  532),    p( 448,  534),
    ],
    // queen
    [
        p( 866,  966),    p( 869,  979),    p( 881,  993),    p( 903,  987),    p( 903,  990),    p( 929,  977),    p( 968,  934),    p( 915,  965),
        p( 890,  952),    p( 867,  979),    p( 867, 1006),    p( 860, 1023),    p( 868, 1034),    p( 905,  999),    p( 908,  981),    p( 947,  964),
        p( 894,  959),    p( 889,  971),    p( 887,  992),    p( 888, 1001),    p( 899, 1009),    p( 948,  988),    p( 951,  966),    p( 942,  971),
        p( 882,  970),    p( 884,  979),    p( 882,  984),    p( 878,  999),    p( 882, 1009),    p( 898,  999),    p( 904, 1006),    p( 912,  983),
        p( 889,  963),    p( 880,  978),    p( 883,  979),    p( 886,  991),    p( 890,  990),    p( 892,  991),    p( 901,  986),    p( 909,  977),
        p( 888,  948),    p( 893,  964),    p( 889,  976),    p( 886,  978),    p( 891,  987),    p( 898,  975),    p( 910,  963),    p( 909,  951),
        p( 886,  953),    p( 888,  958),    p( 895,  960),    p( 895,  974),    p( 896,  973),    p( 898,  956),    p( 909,  937),    p( 916,  913),
        p( 874,  953),    p( 886,  942),    p( 887,  953),    p( 895,  955),    p( 897,  944),    p( 884,  949),    p( 885,  942),    p( 891,  924),
    ],
    // king
    [
        p( 150,  -69),    p(  68,  -24),    p(  81,  -13),    p(  19,   13),    p(  46,    2),    p(  28,   15),    p(  85,    4),    p( 208,  -71),
        p( -33,   16),    p( -58,   30),    p( -62,   37),    p(  -0,   27),    p( -31,   36),    p( -49,   50),    p( -28,   38),    p(   7,   17),
        p( -53,   22),    p( -41,   22),    p( -77,   34),    p( -86,   43),    p( -50,   39),    p( -16,   31),    p( -58,   34),    p( -25,   21),
        p( -28,    7),    p( -96,   15),    p(-112,   28),    p(-133,   37),    p(-132,   37),    p(-108,   30),    p(-119,   21),    p(-101,   22),
        p( -41,   -3),    p(-105,    4),    p(-122,   20),    p(-147,   33),    p(-144,   31),    p(-120,   19),    p(-135,   12),    p(-118,   14),
        p( -35,    0),    p( -83,   -1),    p(-111,   11),    p(-120,   21),    p(-117,   21),    p(-126,   14),    p(-102,    4),    p( -73,   10),
        p(  24,   -8),    p( -69,   -6),    p( -80,    1),    p( -99,   10),    p(-106,   13),    p( -92,    5),    p( -65,   -9),    p(   0,   -3),
        p(  50,  -21),    p(  41,  -33),    p(  39,  -21),    p( -22,   -3),    p(  29,  -18),    p( -19,   -4),    p(  34,  -25),    p(  60,  -31),
    ],
];

const BISHOP_PAIR: PhasedScore = p(24, 51);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(10, 21), p(11, 18), p(10, 7), p(6, 0), p(2, -7), p(-2, -16), p(-8, -24), p(-15, -37), p(-26, -45)];
const ROOK_OPEN_FILE: PhasedScore = p(15, 2);
const ROOK_CLOSED_FILE: PhasedScore = p(-10, -2);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(6, -1);
const KING_OPEN_FILE: PhasedScore = p(-42, 0);
const KING_CLOSED_FILE: PhasedScore = p(12, -7);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-5, 5), p(1, 7), p(-2, 6), p(2, 4), p(2, 6), p(4, 7), p(6, 4), p(19, 2)],
    // Closed
    [p(0, 0), p(0, 0), p(19, -15), p(-14, 10), p(-1, 11), p(-0, 4), p(-2, 7), p(-1, 4)],
    // SemiOpen
    [p(0, 0), p(-12, 23), p(3, 17), p(0, 10), p(-1, 10), p(4, 6), p(1, 3), p(10, 6)],
    // SemiClosed
    [p(0, 0), p(12, -12), p(7, 6), p(3, -0), p(6, 1), p(1, 4), p(3, 4), p(2, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(22, 12),
    p(1, 5),
    p(-3, 6),
    p(-14, 6),
    p(7, 3),
    p(-10, -11),
    p(-2, -4),
    p(-6, -14),
    p(2, 0),
    p(-11, -2),
    p(-12, -16),
    p(-22, -9),
    p(7, -9),
    p(-2, -13),
    p(7, -10),
    p(1, 5),
    p(-5, -2),
    p(-25, -5),
    p(-21, 2),
    p(-51, 21),
    p(-19, 1),
    p(-21, -23),
    p(4, 20),
    p(-61, 27),
    p(-17, -18),
    p(-24, -17),
    p(-41, -31),
    p(-45, 10),
    p(-17, -5),
    p(9, -9),
    p(-96, 105),
    p(0, 0),
    p(-1, -2),
    p(-16, -5),
    p(-10, -5),
    p(-31, -3),
    p(-26, -2),
    p(-51, -22),
    p(-39, 35),
    p(-50, 22),
    p(-8, -5),
    p(-23, -10),
    p(-0, -13),
    p(-24, 31),
    p(-50, 15),
    p(-9, -34),
    p(0, 0),
    p(0, 0),
    p(-2, -16),
    p(-19, 4),
    p(-19, -57),
    p(0, 0),
    p(-1, -16),
    p(-48, -10),
    p(0, 0),
    p(0, 0),
    p(-31, -6),
    p(-24, -15),
    p(-24, 18),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(25, 5),
    p(2, -1),
    p(-6, 0),
    p(-23, -4),
    p(6, -4),
    p(-31, -9),
    p(-21, -5),
    p(-42, -14),
    p(7, -3),
    p(-13, -9),
    p(-31, -10),
    p(-47, -3),
    p(-10, -5),
    p(-48, -6),
    p(-39, -15),
    p(-60, 58),
    p(10, -3),
    p(-4, -9),
    p(-7, -13),
    p(-28, -7),
    p(-13, -4),
    p(-23, -14),
    p(-30, -7),
    p(-87, 177),
    p(-8, -13),
    p(-32, -16),
    p(-42, -31),
    p(1, -84),
    p(-21, -11),
    p(-25, -20),
    p(-87, 57),
    p(0, 0),
    p(16, -1),
    p(1, -5),
    p(-14, -11),
    p(-24, -12),
    p(-1, 1),
    p(-30, -15),
    p(-20, -4),
    p(-36, -1),
    p(-1, -9),
    p(-24, -10),
    p(-29, -20),
    p(-42, -12),
    p(-13, -4),
    p(-51, -13),
    p(-19, 18),
    p(-70, 55),
    p(6, -5),
    p(-12, -5),
    p(-28, 51),
    p(0, 0),
    p(-19, -7),
    p(-25, 0),
    p(0, 0),
    p(0, 0),
    p(-17, -2),
    p(-44, 7),
    p(-40, -41),
    p(0, 0),
    p(1, -65),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-8, 9),    /*0b0000*/
    p(-14, 10),  /*0b0001*/
    p(-4, 10),   /*0b0010*/
    p(-8, 11),   /*0b0011*/
    p(-3, 5),    /*0b0100*/
    p(-28, 3),   /*0b0101*/
    p(-11, 4),   /*0b0110*/
    p(-18, -13), /*0b0111*/
    p(10, 9),    /*0b1000*/
    p(-3, 13),   /*0b1001*/
    p(2, 10),    /*0b1010*/
    p(1, 10),    /*0b1011*/
    p(0, 6),     /*0b1100*/
    p(-27, 7),   /*0b1101*/
    p(-8, 5),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(6, 13),    /*0b10000*/
    p(5, 8),     /*0b10001*/
    p(18, 10),   /*0b10010*/
    p(-3, 7),    /*0b10011*/
    p(-3, 4),    /*0b10100*/
    p(13, 12),   /*0b10101*/
    p(-19, 1),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(16, 12),   /*0b11000*/
    p(26, 15),   /*0b11001*/
    p(37, 26),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(14, 0),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(18, 8),    /*0b100000*/
    p(5, 11),    /*0b100001*/
    p(23, 3),    /*0b100010*/
    p(8, -0),    /*0b100011*/
    p(-7, 4),    /*0b100100*/
    p(-24, -5),  /*0b100101*/
    p(-22, 18),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(28, 3),    /*0b101000*/
    p(-1, 15),   /*0b101001*/
    p(20, -4),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-2, 8),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(16, 5),    /*0b110000*/
    p(22, 5),    /*0b110001*/
    p(29, -3),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(1, 18),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(28, -0),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(8, -3),    /*0b111111*/
    p(-12, -0),  /*0b00*/
    p(5, -12),   /*0b01*/
    p(36, -5),   /*0b10*/
    p(23, -42),  /*0b11*/
    p(45, -9),   /*0b100*/
    p(-8, -9),   /*0b101*/
    p(65, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(65, -11),  /*0b1000*/
    p(16, -28),  /*0b1001*/
    p(72, -51),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(62, -35),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(14, 4),    /*0b1111*/
    p(21, 1),    /*0b00*/
    p(33, -9),   /*0b01*/
    p(27, -15),  /*0b10*/
    p(24, -39),  /*0b11*/
    p(39, -6),   /*0b100*/
    p(53, -17),  /*0b101*/
    p(25, -21),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(40, -2),   /*0b1000*/
    p(51, -16),  /*0b1001*/
    p(54, -38),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(43, -25),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(23, -44),  /*0b1111*/
];
const STOPPABLE_PASSER: PhasedScore = p(37, -49);
const CLOSE_KING_PASSER: PhasedScore = p(-9, 30);
const IMMOBILE_PASSER: PhasedScore = p(-9, -35);
const PROTECTED_PASSER: PhasedScore = p(8, -3);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -41,   41),    p( -47,   54),    p( -67,   57),    p( -69,   50),    p( -58,   37),    p( -47,   38),    p( -31,   44),    p( -47,   46),
        p( -31,   36),    p( -53,   58),    p( -62,   54),    p( -62,   47),    p( -69,   46),    p( -57,   46),    p( -58,   61),    p( -42,   44),
        p( -25,   54),    p( -32,   57),    p( -58,   61),    p( -56,   61),    p( -65,   58),    p( -52,   57),    p( -59,   70),    p( -58,   66),
        p( -14,   71),    p( -18,   75),    p( -14,   64),    p( -39,   73),    p( -56,   75),    p( -42,   73),    p( -52,   84),    p( -58,   84),
        p(  -5,   72),    p(   5,   71),    p(  -1,   55),    p( -20,   39),    p( -15,   51),    p( -37,   58),    p( -47,   60),    p( -85,   82),
        p(  27,   61),    p(  22,   59),    p(  15,   63),    p(  25,   47),    p(  11,   51),    p(  10,   55),    p( -31,   72),    p( -26,   71),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(1, 5), p(5, 9), p(8, 18), p(18, 23), p(26, 67), p(13, 60)];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -6);
const DOUBLED_PAWN: PhasedScore = p(-7, -22);
const PHALANX: [PhasedScore; 6] = [p(0, -0), p(6, 2), p(9, 5), p(23, 19), p(62, 75), p(-101, 222)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 13), p(7, 15), p(15, 19), p(9, 8), p(-3, 17), p(-45, 11)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(38, 8), p(40, 32), p(51, -12), p(35, -41), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-45, -73),
        p(-25, -32),
        p(-13, -9),
        p(-4, 5),
        p(4, 16),
        p(10, 27),
        p(18, 30),
        p(24, 33),
        p(29, 32),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(7, -1),
        p(12, 8),
        p(16, 13),
        p(21, 17),
        p(22, 23),
        p(29, 24),
        p(34, 24),
        p(41, 27),
        p(36, 37),
        p(49, 29),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-53, 50),
        p(-50, 54),
        p(-47, 58),
        p(-44, 63),
        p(-40, 68),
        p(-37, 71),
        p(-36, 76),
        p(-28, 78),
        p(-19, 76),
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
        p(-25, -44),
        p(-24, 1),
        p(-29, 56),
        p(-25, 74),
        p(-23, 93),
        p(-18, 99),
        p(-15, 111),
        p(-12, 117),
        p(-8, 122),
        p(-5, 123),
        p(-2, 127),
        p(2, 129),
        p(4, 130),
        p(6, 136),
        p(8, 138),
        p(12, 142),
        p(12, 150),
        p(15, 152),
        p(24, 150),
        p(38, 145),
        p(41, 148),
        p(85, 126),
        p(86, 128),
        p(107, 112),
        p(199, 80),
        p(246, 41),
        p(286, 25),
        p(333, -8),
    ],
    [
        p(-87, 6),
        p(-53, -7),
        p(-26, -7),
        p(1, -4),
        p(31, -3),
        p(52, -2),
        p(79, 4),
        p(101, 5),
        p(145, -8),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-9, 7), p(0, 0), p(23, 20), p(50, -10), p(20, -31), p(0, 0)],
    [p(-2, 11), p(19, 21), p(0, 0), p(30, 6), p(30, 54), p(0, 0)],
    [p(-3, 14), p(10, 13), p(18, 9), p(0, 0), p(42, 4), p(0, 0)],
    [p(-2, 5), p(1, 4), p(-0, 17), p(0, -0), p(0, 0), p(0, 0)],
    [p(59, 20), p(-28, 18), p(4, 15), p(-18, 9), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 8), p(8, 7), p(6, 10), p(12, 7), p(6, 18), p(10, 6)],
    [p(2, 8), p(11, 20), p(-88, -32), p(8, 13), p(10, 20), p(3, 6)],
    [p(2, 3), p(12, 7), p(8, 12), p(11, 10), p(9, 27), p(20, -3)],
    [p(1, 1), p(8, 1), p(6, -4), p(4, 13), p(-72, -241), p(4, -9)],
    [p(60, -2), p(39, 9), p(44, 3), p(23, 6), p(34, -4), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-18, -15), p(19, -9), p(9, -3), p(14, -11), p(-2, 12), p(-0, 4)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(13, 20), p(33, 1), p(5, 33)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(5, -17), p(23, 30), p(15, 35), p(41, 22), p(68, 32)];
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(-7, -17), p(25, -27), p(55, -51), p(30, 45), p(0, 0), p(-61, -41)];
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

    fn pawn_protection(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn pawn_attack(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn mobility(piece: ChessPieceType, mobility: usize) -> SingleFeatureScore<Self::Score>;

    fn threats(attacking: ChessPieceType, targeted: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn defended(protecting: ChessPieceType, target: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn king_zone_attack(attacking: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn can_give_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn pin(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn discovered_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn discovered_check_stm() -> SingleFeatureScore<Self::Score>;
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

    fn discovered_check_stm() -> SingleFeatureScore<Self::Score> {
        DISCOVERED_CHECK_STM
    }
}
