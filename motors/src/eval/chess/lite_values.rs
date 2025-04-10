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
        p( 126,  160),    p( 122,  159),    p( 115,  163),    p( 125,  147),    p( 111,  151),    p( 110,  154),    p(  70,  171),    p(  73,  170),
        p(  70,  116),    p(  68,  118),    p(  73,  111),    p(  81,  108),    p(  67,  103),    p( 122,  103),    p(  93,  122),    p(  94,  114),
        p(  53,  105),    p(  61,   98),    p(  57,   93),    p(  81,   96),    p(  88,   95),    p(  81,   82),    p(  74,   93),    p(  71,   89),
        p(  48,   91),    p(  51,   92),    p(  75,   90),    p(  94,   92),    p(  88,   94),    p(  83,   90),    p(  68,   83),    p(  58,   78),
        p(  41,   90),    p(  47,   85),    p(  69,   90),    p(  79,   93),    p(  81,   91),    p(  75,   90),    p(  66,   76),    p(  50,   78),
        p(  52,   93),    p(  55,   92),    p(  58,   92),    p(  54,   98),    p(  57,   99),    p(  73,   92),    p(  79,   81),    p(  57,   83),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 174,  281),    p( 195,  314),    p( 212,  326),    p( 251,  314),    p( 281,  316),    p( 199,  311),    p( 215,  310),    p( 202,  265),
        p( 266,  314),    p( 283,  319),    p( 299,  311),    p( 303,  315),    p( 302,  311),    p( 315,  300),    p( 273,  317),    p( 272,  306),
        p( 285,  311),    p( 305,  305),    p( 308,  311),    p( 322,  315),    p( 337,  310),    p( 352,  298),    p( 292,  305),    p( 285,  309),
        p( 301,  316),    p( 308,  310),    p( 325,  314),    p( 326,  321),    p( 325,  318),    p( 321,  317),    p( 311,  313),    p( 319,  311),
        p( 298,  318),    p( 303,  307),    p( 313,  313),    p( 320,  315),    p( 319,  319),    p( 324,  304),    p( 322,  304),    p( 312,  313),
        p( 275,  304),    p( 282,  303),    p( 296,  297),    p( 301,  310),    p( 305,  308),    p( 295,  291),    p( 301,  294),    p( 293,  308),
        p( 269,  311),    p( 280,  315),    p( 284,  305),    p( 294,  309),    p( 298,  305),    p( 289,  303),    p( 294,  308),    p( 289,  322),
        p( 241,  309),    p( 281,  306),    p( 266,  308),    p( 287,  312),    p( 295,  310),    p( 292,  300),    p( 287,  310),    p( 265,  310),
    ],
    // bishop
    [
        p( 275,  312),    p( 251,  316),    p( 238,  309),    p( 222,  318),    p( 216,  316),    p( 223,  310),    p( 271,  305),    p( 248,  311),
        p( 283,  303),    p( 278,  305),    p( 290,  308),    p( 276,  306),    p( 288,  303),    p( 292,  302),    p( 268,  309),    p( 271,  304),
        p( 295,  309),    p( 307,  305),    p( 290,  305),    p( 306,  301),    p( 304,  302),    p( 335,  305),    p( 316,  303),    p( 317,  313),
        p( 286,  312),    p( 291,  308),    p( 303,  303),    p( 305,  306),    p( 306,  304),    p( 299,  305),    p( 298,  309),    p( 280,  311),
        p( 289,  306),    p( 284,  309),    p( 295,  304),    p( 308,  304),    p( 301,  301),    p( 299,  302),    p( 285,  305),    p( 309,  300),
        p( 296,  308),    p( 299,  304),    p( 299,  306),    p( 299,  304),    p( 305,  307),    p( 297,  300),    p( 305,  296),    p( 307,  298),
        p( 308,  307),    p( 303,  300),    p( 307,  302),    p( 299,  309),    p( 301,  307),    p( 302,  306),    p( 311,  296),    p( 307,  295),
        p( 298,  304),    p( 309,  306),    p( 307,  307),    p( 290,  311),    p( 305,  309),    p( 294,  311),    p( 301,  299),    p( 301,  293),
    ],
    // rook
    [
        p( 460,  547),    p( 447,  558),    p( 440,  565),    p( 438,  562),    p( 450,  558),    p( 470,  553),    p( 479,  552),    p( 491,  545),
        p( 443,  554),    p( 441,  560),    p( 450,  560),    p( 464,  551),    p( 450,  554),    p( 467,  549),    p( 475,  546),    p( 491,  535),
        p( 442,  550),    p( 461,  545),    p( 456,  547),    p( 456,  542),    p( 482,  531),    p( 492,  528),    p( 509,  527),    p( 483,  530),
        p( 439,  550),    p( 445,  546),    p( 445,  548),    p( 450,  542),    p( 454,  534),    p( 465,  530),    p( 466,  534),    p( 465,  530),
        p( 433,  546),    p( 432,  545),    p( 433,  545),    p( 438,  541),    p( 446,  537),    p( 440,  537),    p( 453,  531),    p( 447,  529),
        p( 429,  544),    p( 429,  541),    p( 431,  540),    p( 433,  539),    p( 439,  533),    p( 451,  525),    p( 467,  515),    p( 454,  519),
        p( 432,  540),    p( 436,  538),    p( 441,  539),    p( 444,  536),    p( 451,  529),    p( 464,  520),    p( 472,  515),    p( 443,  525),
        p( 441,  544),    p( 438,  540),    p( 439,  543),    p( 443,  537),    p( 449,  530),    p( 455,  531),    p( 452,  530),    p( 447,  534),
    ],
    // queen
    [
        p( 877,  965),    p( 879,  978),    p( 894,  991),    p( 915,  984),    p( 913,  989),    p( 933,  978),    p( 979,  930),    p( 924,  962),
        p( 888,  953),    p( 864,  982),    p( 865, 1009),    p( 858, 1026),    p( 864, 1039),    p( 904, 1000),    p( 905,  985),    p( 946,  965),
        p( 892,  960),    p( 886,  974),    p( 885,  994),    p( 885, 1005),    p( 907, 1007),    p( 945,  991),    p( 952,  964),    p( 939,  972),
        p( 880,  971),    p( 885,  978),    p( 879,  987),    p( 878,  999),    p( 881, 1012),    p( 895, 1002),    p( 904, 1005),    p( 911,  982),
        p( 890,  961),    p( 877,  979),    p( 883,  979),    p( 883,  994),    p( 888,  992),    p( 889,  993),    p( 901,  983),    p( 908,  976),
        p( 886,  949),    p( 892,  964),    p( 887,  977),    p( 883,  979),    p( 889,  988),    p( 896,  977),    p( 910,  960),    p( 907,  951),
        p( 885,  953),    p( 887,  959),    p( 893,  961),    p( 893,  975),    p( 894,  973),    p( 895,  957),    p( 907,  936),    p( 914,  913),
        p( 872,  955),    p( 883,  942),    p( 885,  954),    p( 893,  956),    p( 895,  945),    p( 882,  949),    p( 883,  943),    p( 889,  927),
    ],
    // king
    [
        p( 151,  -69),    p(  63,  -23),    p(  80,  -13),    p(  13,   14),    p(  41,    3),    p(  24,   15),    p(  83,    5),    p( 213,  -71),
        p( -34,   17),    p( -61,   31),    p( -63,   37),    p(   2,   27),    p( -31,   37),    p( -51,   50),    p( -27,   38),    p(   9,   17),
        p( -53,   23),    p( -39,   22),    p( -79,   34),    p( -88,   44),    p( -51,   39),    p( -18,   32),    p( -58,   34),    p( -31,   21),
        p( -29,    7),    p( -96,   15),    p(-112,   29),    p(-132,   37),    p(-131,   37),    p(-109,   30),    p(-120,   22),    p(-102,   22),
        p( -43,   -2),    p(-105,    4),    p(-121,   20),    p(-146,   33),    p(-144,   31),    p(-120,   19),    p(-134,   12),    p(-117,   14),
        p( -34,    0),    p( -81,   -2),    p(-110,   11),    p(-118,   20),    p(-116,   21),    p(-125,   14),    p(-100,    3),    p( -72,   10),
        p(  25,   -8),    p( -68,   -6),    p( -79,    1),    p( -98,   10),    p(-104,   13),    p( -91,    5),    p( -64,   -9),    p(   1,   -2),
        p(  48,  -21),    p(  41,  -33),    p(  39,  -21),    p( -21,   -3),    p(  29,  -18),    p( -18,   -4),    p(  33,  -26),    p(  59,  -31),
    ],
];

const BISHOP_PAIR: PhasedScore = p(24, 51);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(9, 21), p(11, 18), p(10, 7), p(6, 0), p(1, -7), p(-2, -16), p(-9, -24), p(-16, -38), p(-26, -46)];
const ROOK_OPEN_FILE: PhasedScore = p(16, 3);
const ROOK_CLOSED_FILE: PhasedScore = p(-10, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(6, -0);
const KING_OPEN_FILE: PhasedScore = p(-50, 1);
const KING_CLOSED_FILE: PhasedScore = p(12, -8);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 8);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-6, 4), p(0, 6), p(-2, 5), p(2, 4), p(2, 6), p(3, 7), p(6, 5), p(18, 2)],
    // Closed
    [p(0, 0), p(0, 0), p(21, -19), p(-15, 10), p(-1, 12), p(-0, 4), p(-2, 7), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-15, 24), p(2, 17), p(-0, 10), p(-1, 10), p(4, 6), p(0, 2), p(9, 5)],
    // SemiClosed
    [p(0, 0), p(12, -12), p(7, 6), p(3, -0), p(6, 1), p(0, 4), p(3, 5), p(2, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(23, 12),
    p(1, 5),
    p(-3, 6),
    p(-14, 5),
    p(7, 3),
    p(-10, -11),
    p(-2, -4),
    p(-7, -14),
    p(2, 0),
    p(-11, -2),
    p(-12, -16),
    p(-22, -9),
    p(7, -9),
    p(-2, -13),
    p(6, -10),
    p(1, 6),
    p(-5, -2),
    p(-25, -5),
    p(-22, 2),
    p(-51, 21),
    p(-19, 1),
    p(-21, -23),
    p(3, 20),
    p(-61, 27),
    p(-18, -18),
    p(-24, -17),
    p(-41, -32),
    p(-46, 10),
    p(-17, -5),
    p(9, -9),
    p(-96, 106),
    p(0, 0),
    p(-1, -2),
    p(-16, -5),
    p(-10, -5),
    p(-31, -3),
    p(-26, -2),
    p(-51, -22),
    p(-39, 35),
    p(-50, 24),
    p(-8, -5),
    p(-24, -10),
    p(0, -13),
    p(-24, 31),
    p(-50, 15),
    p(-10, -33),
    p(0, 0),
    p(0, 0),
    p(-3, -16),
    p(-19, 4),
    p(-18, -57),
    p(0, 0),
    p(-2, -17),
    p(-48, -10),
    p(0, 0),
    p(0, 0),
    p(-32, -6),
    p(-24, -17),
    p(-24, 19),
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
    p(-30, -9),
    p(-21, -5),
    p(-42, -14),
    p(7, -3),
    p(-13, -9),
    p(-31, -10),
    p(-47, -3),
    p(-10, -6),
    p(-47, -6),
    p(-40, -15),
    p(-60, 58),
    p(10, -3),
    p(-4, -9),
    p(-7, -13),
    p(-29, -6),
    p(-13, -4),
    p(-23, -14),
    p(-30, -6),
    p(-85, 172),
    p(-8, -13),
    p(-32, -16),
    p(-42, -31),
    p(-2, -82),
    p(-21, -12),
    p(-24, -20),
    p(-87, 59),
    p(0, 0),
    p(16, -1),
    p(1, -5),
    p(-14, -11),
    p(-24, -12),
    p(-1, 1),
    p(-30, -15),
    p(-20, -4),
    p(-35, -2),
    p(-1, -9),
    p(-24, -10),
    p(-29, -20),
    p(-42, -12),
    p(-12, -5),
    p(-51, -14),
    p(-20, 17),
    p(-69, 50),
    p(6, -5),
    p(-12, -6),
    p(-28, 51),
    p(0, 0),
    p(-19, -7),
    p(-26, 0),
    p(0, 0),
    p(0, 0),
    p(-17, -1),
    p(-44, 7),
    p(-41, -42),
    p(0, 0),
    p(1, -65),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-7, 9),    /*0b0000*/
    p(-13, 9),   /*0b0001*/
    p(-4, 10),   /*0b0010*/
    p(-8, 11),   /*0b0011*/
    p(-3, 5),    /*0b0100*/
    p(-27, 2),   /*0b0101*/
    p(-12, 3),   /*0b0110*/
    p(-18, -13), /*0b0111*/
    p(11, 9),    /*0b1000*/
    p(-2, 13),   /*0b1001*/
    p(2, 10),    /*0b1010*/
    p(0, 11),    /*0b1011*/
    p(1, 5),     /*0b1100*/
    p(-26, 7),   /*0b1101*/
    p(-9, 4),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(7, 13),    /*0b10000*/
    p(5, 8),     /*0b10001*/
    p(18, 10),   /*0b10010*/
    p(-3, 7),    /*0b10011*/
    p(-3, 4),    /*0b10100*/
    p(13, 12),   /*0b10101*/
    p(-20, 1),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(17, 12),   /*0b11000*/
    p(26, 16),   /*0b11001*/
    p(37, 25),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(14, -0),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(18, 8),    /*0b100000*/
    p(5, 11),    /*0b100001*/
    p(23, 3),    /*0b100010*/
    p(8, 0),     /*0b100011*/
    p(-7, 3),    /*0b100100*/
    p(-24, -5),  /*0b100101*/
    p(-22, 18),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(28, 3),    /*0b101000*/
    p(-1, 16),   /*0b101001*/
    p(20, -5),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-1, 8),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(16, 6),    /*0b110000*/
    p(22, 6),    /*0b110001*/
    p(28, -3),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(1, 18),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(28, 0),    /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(7, -2),    /*0b111111*/
    p(-11, -0),  /*0b00*/
    p(4, -11),   /*0b01*/
    p(36, -5),   /*0b10*/
    p(24, -41),  /*0b11*/
    p(44, -8),   /*0b100*/
    p(-7, -9),   /*0b101*/
    p(66, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(66, -11),  /*0b1000*/
    p(16, -29),  /*0b1001*/
    p(71, -47),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(61, -36),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(15, 2),    /*0b1111*/
    p(21, 1),    /*0b00*/
    p(33, -9),   /*0b01*/
    p(26, -14),  /*0b10*/
    p(23, -39),  /*0b11*/
    p(39, -6),   /*0b100*/
    p(53, -17),  /*0b101*/
    p(24, -21),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(41, -2),   /*0b1000*/
    p(52, -16),  /*0b1001*/
    p(54, -39),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(43, -25),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(22, -44),  /*0b1111*/
];
const STOPPABLE_PASSER: PhasedScore = p(37, -49);
const CLOSE_KING_PASSER: PhasedScore = p(-8, 30);
const IMMOBILE_PASSER: PhasedScore = p(-9, -35);
const PROTECTED_PASSER: PhasedScore = p(8, -3);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -41,   41),    p( -47,   54),    p( -67,   57),    p( -69,   50),    p( -58,   37),    p( -48,   38),    p( -30,   44),    p( -47,   45),
        p( -31,   36),    p( -53,   58),    p( -62,   54),    p( -62,   47),    p( -70,   47),    p( -56,   45),    p( -58,   61),    p( -42,   44),
        p( -25,   54),    p( -32,   57),    p( -59,   61),    p( -56,   61),    p( -65,   58),    p( -51,   57),    p( -59,   70),    p( -58,   66),
        p( -14,   71),    p( -18,   74),    p( -15,   64),    p( -39,   73),    p( -56,   74),    p( -43,   73),    p( -53,   84),    p( -58,   84),
        p(  -5,   73),    p(   5,   72),    p(  -1,   56),    p( -20,   40),    p( -16,   52),    p( -38,   59),    p( -47,   61),    p( -86,   83),
        p(  26,   60),    p(  22,   59),    p(  15,   63),    p(  25,   47),    p(  11,   51),    p(  10,   54),    p( -30,   71),    p( -27,   70),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(1, 5), p(5, 9), p(8, 18), p(18, 23), p(26, 66), p(12, 60)];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -6);
const DOUBLED_PAWN: PhasedScore = p(-7, -22);
const PHALANX: [PhasedScore; 6] = [p(0, -0), p(6, 2), p(9, 5), p(23, 19), p(62, 75), p(-98, 222)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 12), p(7, 15), p(14, 19), p(9, 8), p(-3, 17), p(-45, 11)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(38, 8), p(39, 33), p(51, -11), p(34, -37), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-46, -73),
        p(-26, -32),
        p(-13, -8),
        p(-4, 5),
        p(3, 16),
        p(10, 27),
        p(17, 30),
        p(24, 33),
        p(28, 32),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-32, -55),
        p(-19, -37),
        p(-8, -22),
        p(-1, -10),
        p(6, -0),
        p(11, 8),
        p(16, 13),
        p(20, 17),
        p(22, 22),
        p(29, 24),
        p(33, 23),
        p(40, 26),
        p(36, 36),
        p(50, 27),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-66, 26),
        p(-62, 32),
        p(-59, 36),
        p(-60, 43),
        p(-54, 49),
        p(-51, 54),
        p(-48, 58),
        p(-45, 63),
        p(-42, 67),
        p(-38, 71),
        p(-37, 76),
        p(-30, 77),
        p(-22, 76),
        p(-18, 74),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-28, -46),
        p(-27, 6),
        p(-32, 59),
        p(-27, 78),
        p(-25, 96),
        p(-21, 102),
        p(-18, 113),
        p(-14, 119),
        p(-10, 124),
        p(-7, 125),
        p(-5, 128),
        p(-1, 131),
        p(2, 132),
        p(3, 137),
        p(6, 139),
        p(9, 143),
        p(10, 151),
        p(12, 152),
        p(21, 150),
        p(35, 145),
        p(38, 148),
        p(82, 125),
        p(81, 130),
        p(102, 113),
        p(194, 81),
        p(237, 41),
        p(270, 30),
        p(330, -11),
    ],
    [
        p(-85, 3),
        p(-52, -7),
        p(-26, -7),
        p(1, -4),
        p(30, -3),
        p(50, -1),
        p(77, 4),
        p(100, 5),
        p(143, -8),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-9, 7), p(0, 0), p(23, 19), p(49, -11), p(20, -33), p(0, 0)],
    [p(-2, 10), p(20, 23), p(0, 0), p(31, 5), p(30, 53), p(0, 0)],
    [p(-3, 13), p(10, 16), p(17, 13), p(0, 0), p(45, -4), p(0, 0)],
    [p(-2, 3), p(2, 7), p(-1, 23), p(1, 3), p(0, 0), p(0, 0)],
    [p(63, 19), p(-35, 18), p(-9, 16), p(-21, 7), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 8), p(8, 7), p(6, 10), p(12, 7), p(7, 18), p(10, 6)],
    [p(2, 8), p(11, 21), p(-139, -18), p(8, 13), p(10, 18), p(4, 7)],
    [p(2, 3), p(12, 8), p(8, 13), p(11, 10), p(9, 25), p(20, -4)],
    [p(1, 0), p(8, 2), p(7, -3), p(4, 14), p(-60, -258), p(4, -10)],
    [p(60, -2), p(38, 8), p(43, 2), p(21, 6), p(34, -10), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-18, -15), p(19, -9), p(10, -3), p(14, -11), p(-1, 12), p(0, 4)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(14, 19), p(34, 0), p(5, 32)];

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
}
