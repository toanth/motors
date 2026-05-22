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
use gears::games::chess::Color;
use gears::games::chess::Color::White;
use gears::games::chess::pieces::{NUM_CHESS_PIECES, PieceType};
use gears::games::chess::squares::{NUM_SQUARES, Square};
use gears::general::common::StaticallyNamedEntity;
use gears::score::{PhasedScore, p};
use std::fmt::{Debug, Display};

#[rustfmt::skip]
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
        p( 115,  163),    p( 115,  162),    p( 120,  156),    p( 123,  150),    p( 121,  157),    p( 122,  171),    p(  86,  178),    p(  94,  177),
        p(  69,  119),    p(  71,  121),    p(  85,  111),    p(  85,  108),    p(  81,  103),    p( 129,  105),    p( 115,  120),    p( 118,  113),
        p(  51,  102),    p(  61,   98),    p(  59,   92),    p(  86,   98),    p(  90,   96),    p(  87,   85),    p(  88,   94),    p(  90,   92),
        p(  45,   89),    p(  49,   92),    p(  70,   91),    p(  87,   94),    p(  92,   93),    p(  89,   90),    p(  80,   88),    p(  76,   83),
        p(  35,   89),    p(  46,   85),    p(  66,   92),    p(  82,   93),    p(  82,   94),    p(  80,   92),    p(  84,   77),    p(  72,   80),
        p(  46,   96),    p(  54,   92),    p(  60,   93),    p(  58,   98),    p(  64,  101),    p(  90,   90),    p( 107,   78),    p(  82,   80),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 187,  285),    p( 190,  325),    p( 209,  325),    p( 242,  322),    p( 286,  309),    p( 208,  315),    p( 231,  298),    p( 198,  259),
        p( 275,  317),    p( 286,  324),    p( 298,  312),    p( 301,  316),    p( 305,  310),    p( 313,  302),    p( 294,  314),    p( 280,  305),
        p( 288,  315),    p( 304,  309),    p( 309,  314),    p( 320,  315),    p( 331,  312),    p( 352,  299),    p( 295,  309),    p( 296,  308),
        p( 307,  320),    p( 315,  316),    p( 329,  318),    p( 328,  325),    p( 328,  323),    p( 330,  320),    p( 320,  321),    p( 329,  313),
        p( 301,  320),    p( 313,  312),    p( 315,  318),    p( 324,  321),    p( 322,  323),    p( 329,  309),    p( 327,  309),    p( 317,  318),
        p( 278,  307),    p( 286,  305),    p( 295,  302),    p( 305,  314),    p( 309,  312),    p( 306,  292),    p( 307,  299),    p( 301,  309),
        p( 271,  315),    p( 285,  317),    p( 287,  311),    p( 297,  312),    p( 304,  308),    p( 299,  303),    p( 302,  313),    p( 297,  321),
        p( 242,  311),    p( 280,  310),    p( 270,  310),    p( 291,  315),    p( 298,  315),    p( 297,  305),    p( 288,  312),    p( 271,  314),
    ],
    // bishop
    [
        p( 275,  322),    p( 260,  313),    p( 231,  310),    p( 223,  319),    p( 211,  317),    p( 236,  310),    p( 259,  313),    p( 259,  311),
        p( 287,  307),    p( 287,  305),    p( 295,  307),    p( 279,  308),    p( 287,  303),    p( 296,  303),    p( 257,  314),    p( 277,  306),
        p( 301,  310),    p( 305,  307),    p( 289,  308),    p( 305,  300),    p( 305,  306),    p( 331,  303),    p( 318,  307),    p( 316,  316),
        p( 286,  312),    p( 298,  307),    p( 310,  302),    p( 308,  308),    p( 308,  304),    p( 311,  306),    p( 312,  304),    p( 289,  308),
        p( 287,  308),    p( 287,  309),    p( 296,  308),    p( 309,  302),    p( 305,  303),    p( 308,  299),    p( 292,  304),    p( 313,  297),
        p( 296,  307),    p( 298,  306),    p( 295,  306),    p( 299,  308),    p( 307,  306),    p( 299,  299),    p( 308,  295),    p( 313,  302),
        p( 306,  304),    p( 298,  301),    p( 304,  303),    p( 298,  311),    p( 300,  309),    p( 307,  306),    p( 315,  295),    p( 309,  303),
        p( 297,  307),    p( 308,  306),    p( 306,  310),    p( 290,  311),    p( 303,  311),    p( 293,  313),    p( 303,  308),    p( 305,  301),
    ],
    // rook
    [
        p( 448,  561),    p( 444,  565),    p( 432,  572),    p( 429,  571),    p( 442,  565),    p( 463,  565),    p( 452,  567),    p( 491,  546),
        p( 453,  561),    p( 451,  565),    p( 457,  566),    p( 473,  557),    p( 460,  560),    p( 489,  554),    p( 492,  553),    p( 503,  541),
        p( 453,  556),    p( 470,  550),    p( 464,  553),    p( 463,  547),    p( 487,  539),    p( 505,  534),    p( 513,  538),    p( 488,  539),
        p( 450,  556),    p( 459,  552),    p( 459,  553),    p( 462,  548),    p( 469,  544),    p( 484,  541),    p( 479,  546),    p( 476,  541),
        p( 440,  553),    p( 442,  551),    p( 442,  553),    p( 450,  549),    p( 457,  546),    p( 455,  546),    p( 470,  542),    p( 456,  541),
        p( 435,  548),    p( 440,  545),    p( 439,  546),    p( 441,  547),    p( 451,  540),    p( 461,  534),    p( 475,  529),    p( 462,  532),
        p( 436,  546),    p( 441,  544),    p( 447,  545),    p( 450,  541),    p( 458,  537),    p( 471,  530),    p( 481,  524),    p( 448,  536),
        p( 443,  550),    p( 443,  544),    p( 442,  548),    p( 447,  542),    p( 453,  536),    p( 454,  541),    p( 442,  547),    p( 440,  541),
    ],
    // queen
    [
        p( 866,  979),    p( 868,  985),    p( 882,  995),    p( 902,  985),    p( 908,  990),    p( 933,  979),    p( 967,  933),    p( 904,  961),
        p( 899,  966),    p( 885,  982),    p( 884,  999),    p( 876, 1021),    p( 876, 1036),    p( 916, 1009),    p( 910,  990),    p( 954,  960),
        p( 910,  961),    p( 904,  970),    p( 900,  993),    p( 896, 1006),    p( 898, 1017),    p( 939, 1007),    p( 953,  978),    p( 937,  980),
        p( 898,  968),    p( 900,  978),    p( 897,  983),    p( 888, 1006),    p( 891, 1014),    p( 910, 1005),    p( 914, 1008),    p( 923,  984),
        p( 893,  969),    p( 891,  975),    p( 890,  981),    p( 891,  995),    p( 896,  998),    p( 899,  996),    p( 908,  992),    p( 912,  979),
        p( 893,  953),    p( 897,  964),    p( 891,  978),    p( 890,  983),    p( 893,  993),    p( 903,  982),    p( 910,  975),    p( 910,  954),
        p( 895,  947),    p( 892,  962),    p( 896,  965),    p( 896,  979),    p( 897,  981),    p( 900,  963),    p( 909,  946),    p( 902,  938),
        p( 875,  961),    p( 889,  950),    p( 889,  958),    p( 894,  963),    p( 896,  953),    p( 881,  964),    p( 878,  943),    p( 889,  917),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  65,    8),    p(  78,    4),    p(  98,   -8),    p( 193,  -65),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   2,   29),    p( -39,   41),    p( -30,   31),    p(  -5,   19),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -51,   40),    p( -32,   33),    p( -38,   26),    p( -35,   25),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-121,   38),    p( -97,   30),    p( -93,   17),    p( -72,   19),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-138,   32),    p(-112,   19),    p(-112,    8),    p( -95,   12),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-115,   20),    p(-114,   13),    p( -86,    0),    p( -58,   11),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-104,   11),    p( -91,    4),    p( -63,  -10),    p(   1,    1),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  17,  -11),    p(   2,   -6),    p(  25,  -19),    p(  58,  -21),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 52);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(13, 20), p(14, 18), p(13, 7), p(9, -0), p(5, -7), p(1, -16), p(-5, -24), p(-11, -37), p(-22, -41)];
const ROOK_OPEN_FILE: PhasedScore = p(14, 1);
const ROOK_CLOSED_FILE: PhasedScore = p(-11, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(6, -3);
const KING_OPEN_FILE: PhasedScore = p(-37, 6);
const KING_CLOSED_FILE: PhasedScore = p(12, -6);
const KING_SEMIOPEN_FILE: PhasedScore = p(-3, 11);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-5, 3), p(1, 9), p(-2, 8), p(4, 6), p(5, 7), p(3, 9), p(8, 5), p(21, 3)],
    // Closed
    [p(0, 0), p(0, 0), p(16, -15), p(-14, 13), p(2, 12), p(-1, 6), p(-0, 9), p(2, 7)],
    // SemiOpen
    [p(0, 0), p(-5, 29), p(7, 21), p(2, 14), p(2, 13), p(3, 7), p(3, 4), p(12, 8)],
    // SemiClosed
    [p(0, 0), p(12, -11), p(8, 8), p(4, 2), p(8, 4), p(0, 6), p(3, 6), p(4, 6)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(23, 11),
    p(4, 5),
    p(-1, 6),
    p(-8, 7),
    p(4, 4),
    p(-9, -9),
    p(-3, -1),
    p(-4, -11),
    p(2, 1),
    p(-9, 0),
    p(-9, -14),
    p(-16, -5),
    p(4, -4),
    p(-2, -8),
    p(7, -6),
    p(7, 13),
    p(-6, -2),
    p(-21, -4),
    p(-16, 1),
    p(-40, 21),
    p(-22, 4),
    p(-20, -18),
    p(3, 24),
    p(-45, 26),
    p(-17, -16),
    p(-21, -15),
    p(-36, -29),
    p(-41, 13),
    p(-18, 0),
    p(11, -4),
    p(-84, 95),
    p(0, 0),
    p(1, -4),
    p(-12, -6),
    p(-4, -7),
    p(-23, -2),
    p(-23, -2),
    p(-46, -20),
    p(-26, 35),
    p(-32, 25),
    p(-6, -5),
    p(-18, -9),
    p(5, -11),
    p(-17, 33),
    p(-50, 19),
    p(-2, -30),
    p(0, 0),
    p(0, 0),
    p(-4, -13),
    p(-15, 5),
    p(-9, -56),
    p(0, 0),
    p(4, -12),
    p(-39, -16),
    p(0, 0),
    p(0, 0),
    p(-29, -6),
    p(-22, -13),
    p(-21, 16),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(23, 5),
    p(3, -2),
    p(-10, 3),
    p(-25, -2),
    p(6, -5),
    p(-28, -10),
    p(-23, -3),
    p(-41, -12),
    p(7, -3),
    p(-9, -9),
    p(-32, -7),
    p(-47, -0),
    p(-5, -6),
    p(-41, -7),
    p(-38, -9),
    p(-53, 59),
    p(11, -1),
    p(2, -9),
    p(-5, -11),
    p(-22, -3),
    p(-10, -3),
    p(-15, -14),
    p(-27, -2),
    p(-70, 177),
    p(-2, -11),
    p(-21, -16),
    p(-32, -29),
    p(11, -82),
    p(-12, -10),
    p(-10, -20),
    p(-81, 67),
    p(0, 0),
    p(14, -1),
    p(0, -5),
    p(-19, -7),
    p(-27, -7),
    p(-2, -1),
    p(-26, -18),
    p(-19, -3),
    p(-31, -0),
    p(1, -8),
    p(-18, -9),
    p(-28, -15),
    p(-36, -5),
    p(-7, -4),
    p(-46, -7),
    p(0, 14),
    p(-60, 59),
    p(4, -1),
    p(-10, -2),
    p(-31, 58),
    p(0, 0),
    p(-16, -4),
    p(-21, 5),
    p(0, 0),
    p(0, 0),
    p(-12, 0),
    p(-36, 8),
    p(-33, -41),
    p(0, 0),
    p(16, -63),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-12, 11),  /*0b0000*/
    p(-12, 9),   /*0b0001*/
    p(-3, 13),   /*0b0010*/
    p(-3, 12),   /*0b0011*/
    p(-7, 5),    /*0b0100*/
    p(-23, 1),   /*0b0101*/
    p(-10, 7),   /*0b0110*/
    p(-12, -9),  /*0b0111*/
    p(2, 7),     /*0b1000*/
    p(-8, 11),   /*0b1001*/
    p(-0, 13),   /*0b1010*/
    p(3, 13),    /*0b1011*/
    p(-4, 5),    /*0b1100*/
    p(-18, 5),   /*0b1101*/
    p(-9, 6),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(2, 12),    /*0b10000*/
    p(5, 8),     /*0b10001*/
    p(21, 10),   /*0b10010*/
    p(-1, 8),    /*0b10011*/
    p(-8, 5),    /*0b10100*/
    p(13, 14),   /*0b10101*/
    p(-29, 4),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(8, 12),    /*0b11000*/
    p(25, 13),   /*0b11001*/
    p(27, 28),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(8, -1),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(7, 5),     /*0b100000*/
    p(0, 9),     /*0b100001*/
    p(14, 6),    /*0b100010*/
    p(6, 1),     /*0b100011*/
    p(-5, -1),   /*0b100100*/
    p(-20, -11), /*0b100101*/
    p(-20, 20),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(16, -0),   /*0b101000*/
    p(-0, 10),   /*0b101001*/
    p(10, -3),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(0, 1),     /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(4, 4),     /*0b110000*/
    p(13, 6),    /*0b110001*/
    p(15, -2),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(10, 15),   /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(19, -3),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(5, -2),    /*0b111111*/
    p(10, 1),    /*0b00*/
    p(17, -9),   /*0b01*/
    p(34, -8),   /*0b10*/
    p(9, -29),   /*0b11*/
    p(38, -3),   /*0b100*/
    p(10, -8),   /*0b101*/
    p(45, -28),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(52, -11),  /*0b1000*/
    p(8, -18),   /*0b1001*/
    p(26, -36),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(18, -18),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-21, 26),  /*0b1111*/
    p(17, 2),    /*0b00*/
    p(28, -10),  /*0b01*/
    p(24, -13),  /*0b10*/
    p(23, -37),  /*0b11*/
    p(27, -10),  /*0b100*/
    p(40, -19),  /*0b101*/
    p(16, -18),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(30, -2),   /*0b1000*/
    p(38, -14),  /*0b1001*/
    p(43, -43),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(25, -24),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(10, -42),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(-1, -37);
const STOPPABLE_PASSER: PhasedScore = p(25, -48);
const CLOSE_KING_PASSER: PhasedScore = p(3, 24);
const IMMOBILE_PASSER: PhasedScore = p(-5, -36);
const PROTECTED_PASSER: PhasedScore = p(8, -3);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -31,   46),    p( -39,   66),    p( -54,   61),    p( -59,   44),    p( -60,   46),    p( -48,   42),    p( -31,   43),    p( -57,   46),
        p( -24,   43),    p( -45,   67),    p( -50,   58),    p( -51,   44),    p( -68,   53),    p( -62,   50),    p( -49,   56),    p( -49,   44),
        p( -18,   55),    p( -27,   55),    p( -46,   59),    p( -43,   58),    p( -54,   63),    p( -49,   63),    p( -50,   71),    p( -70,   70),
        p(  -8,   71),    p( -11,   69),    p( -11,   60),    p( -29,   72),    p( -45,   80),    p( -36,   85),    p( -40,   91),    p( -61,   92),
        p(  -5,   62),    p(  12,   52),    p(  -2,   44),    p( -16,   38),    p( -16,   55),    p( -32,   70),    p( -52,   81),    p( -81,   91),
        p(  15,   63),    p(  15,   62),    p(  20,   56),    p(  23,   50),    p(  21,   57),    p(  22,   71),    p( -14,   78),    p(  -6,   77),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(1, 4), p(5, 8), p(8, 17), p(15, 22), p(17, 71), p(12, 59)];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -6);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);
const PHALANX: [PhasedScore; 6] = [p(-1, 0), p(5, 3), p(9, 6), p(21, 20), p(58, 74), p(-81, 216)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 14), p(7, 18), p(14, 21), p(7, 10), p(-3, 13), p(-48, 12)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(48, 21), p(51, 45), p(66, 4), p(51, -8), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(1, -5), p(14, 20), p(19, -7), p(16, 10), p(16, -9), p(29, -12)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-41, -69),
        p(-20, -29),
        p(-8, -6),
        p(1, 7),
        p(8, 18),
        p(15, 28),
        p(23, 31),
        p(30, 34),
        p(33, 32),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-29, -57),
        p(-16, -38),
        p(-4, -22),
        p(3, -10),
        p(10, 0),
        p(15, 9),
        p(20, 14),
        p(25, 18),
        p(27, 24),
        p(34, 25),
        p(40, 25),
        p(45, 29),
        p(40, 40),
        p(53, 31),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-69, 14),
        p(-60, 28),
        p(-56, 35),
        p(-52, 40),
        p(-53, 47),
        p(-47, 53),
        p(-45, 58),
        p(-41, 62),
        p(-38, 67),
        p(-35, 72),
        p(-30, 74),
        p(-29, 80),
        p(-21, 81),
        p(-13, 79),
        p(-9, 76),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-16, -23),
        p(-16, 11),
        p(-21, 68),
        p(-17, 83),
        p(-15, 102),
        p(-10, 108),
        p(-7, 119),
        p(-4, 126),
        p(0, 132),
        p(3, 133),
        p(6, 137),
        p(10, 140),
        p(14, 141),
        p(16, 146),
        p(18, 149),
        p(23, 151),
        p(25, 158),
        p(29, 158),
        p(39, 155),
        p(54, 148),
        p(60, 149),
        p(101, 126),
        p(101, 129),
        p(129, 106),
        p(218, 75),
        p(260, 38),
        p(292, 25),
        p(273, 21),
    ],
    [
        p(-82, -0),
        p(-52, -10),
        p(-24, -9),
        p(5, -5),
        p(36, -3),
        p(59, -1),
        p(88, 5),
        p(113, 8),
        p(160, -1),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-4, 14), p(0, 0), p(28, 26), p(60, 7), p(39, 0), p(0, 0)],
    [p(-2, 12), p(21, 25), p(0, 0), p(42, 22), p(46, 92), p(0, 0)],
    [p(-4, 19), p(12, 18), p(19, 14), p(0, 0), p(66, 45), p(0, 0)],
    [p(-2, 9), p(2, 9), p(1, 25), p(-0, 13), p(0, 0), p(0, 0)],
    [p(64, 21), p(-20, 23), p(8, 17), p(-13, 23), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 9), p(8, 7), p(6, 11), p(12, 8), p(8, 14), p(3, 8)],
    [p(2, 9), p(11, 22), p(-1, -44), p(9, 12), p(10, 20), p(4, 7)],
    [p(2, 4), p(13, 9), p(9, 14), p(11, 10), p(9, 29), p(19, -3)],
    [p(2, 3), p(8, 3), p(7, -2), p(5, 13), p(-63, -221), p(5, -10)],
    [p(48, 0), p(36, 12), p(41, 6), p(26, 6), p(37, -9), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-20, -14), p(17, -9), p(9, -3), p(16, -16), p(-3, 4), p(-7, 3)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(8, 2), p(-5, 7), p(16, -8), p(-10, 24)];
const CHECK_STM: PhasedScore = p(38, 22);
const DISCOVERED_CHECK_STM: PhasedScore = p(176, 59);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(11, -7), p(62, 1), p(101, -33), p(55, 89), p(0, 0), p(-26, -23)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(5, -17), p(26, 30), p(16, 34), p(43, 9), p(62, 3)];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static + StaticallyNamedEntity {
    type Score: ScoreType;

    fn psqt(&self, square: Square, piece: PieceType, color: Color) -> SingleFeatureScore<Self::Score>;

    fn passed_pawn(square: Square) -> SingleFeatureScore<Self::Score>;

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

    fn pawn_shield(&self, color: Color, config: usize) -> SingleFeatureScore<Self::Score>;

    fn pawnless_flank() -> SingleFeatureScore<Self::Score>;

    fn pawn_protection(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn pawn_attack(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn pawn_advance_threat(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn mobility(piece: PieceType, mobility: usize) -> SingleFeatureScore<Self::Score>;

    fn threats(attacking: PieceType, targeted: PieceType) -> SingleFeatureScore<Self::Score>;

    fn defended(protecting: PieceType, target: PieceType) -> SingleFeatureScore<Self::Score>;

    fn king_zone_attack(attacking: PieceType) -> SingleFeatureScore<Self::Score>;

    fn can_give_check(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn check_stm() -> SingleFeatureScore<Self::Score>;

    fn discovered_check_stm() -> SingleFeatureScore<Self::Score>;

    fn discovered_check(piece: PieceType) -> SingleFeatureScore<Self::Score>;

    fn pin(piece: PieceType) -> SingleFeatureScore<Self::Score>;
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

    fn psqt(&self, square: Square, piece: PieceType, color: Color) -> Self::Score {
        PSQTS[piece as usize][square.flip_if(color == White).bb_idx()]
    }

    fn passed_pawn(square: Square) -> PhasedScore {
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

    fn pawn_shield(&self, _color: Color, config: usize) -> PhasedScore {
        PAWN_SHIELDS[config]
    }

    fn pawnless_flank() -> PhasedScore {
        PAWNLESS_FLANK
    }

    fn pawn_protection(piece: PieceType) -> PhasedScore {
        PAWN_PROTECTION[piece as usize]
    }

    fn pawn_attack(piece: PieceType) -> PhasedScore {
        PAWN_ATTACKS[piece as usize]
    }

    fn pawn_advance_threat(piece: PieceType) -> PhasedScore {
        PAWN_ADVANCE_THREAT[piece as usize]
    }

    fn mobility(piece: PieceType, mobility: usize) -> PhasedScore {
        MOBILITY[piece as usize - 1][mobility]
    }

    fn threats(attacking: PieceType, targeted: PieceType) -> PhasedScore {
        THREATS[attacking as usize - 1][targeted as usize]
    }
    fn defended(protecting: PieceType, target: PieceType) -> PhasedScore {
        DEFENDED[protecting as usize - 1][target as usize]
    }

    fn king_zone_attack(attacking: PieceType) -> PhasedScore {
        KING_ZONE_ATTACK[attacking as usize]
    }

    fn can_give_check(piece: PieceType) -> PhasedScore {
        CAN_GIVE_CHECK[piece as usize]
    }

    fn discovered_check_stm() -> PhasedScore {
        DISCOVERED_CHECK_STM
    }

    fn pin(piece: PieceType) -> PhasedScore {
        PIN[piece as usize]
    }

    fn discovered_check(piece: PieceType) -> PhasedScore {
        DISCOVERED_CHECK[piece as usize]
    }

    fn check_stm() -> PhasedScore {
        CHECK_STM
    }
}
