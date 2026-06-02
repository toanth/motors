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
        p( 114,  161),    p( 114,  160),    p( 119,  154),    p( 122,  149),    p( 120,  156),    p( 121,  170),    p(  86,  176),    p(  94,  176),
        p(  69,  119),    p(  70,  121),    p(  85,  111),    p(  85,  108),    p(  81,  103),    p( 129,  105),    p( 115,  120),    p( 118,  113),
        p(  51,  102),    p(  61,   98),    p(  59,   92),    p(  86,   95),    p(  89,   93),    p(  87,   85),    p(  88,   95),    p(  90,   92),
        p(  45,   89),    p(  49,   92),    p(  71,   90),    p(  87,   91),    p(  92,   90),    p(  89,   89),    p(  80,   88),    p(  76,   83),
        p(  35,   89),    p(  46,   85),    p(  67,   91),    p(  79,   93),    p(  79,   93),    p(  81,   92),    p(  84,   77),    p(  72,   80),
        p(  46,   95),    p(  54,   92),    p(  60,   93),    p(  58,   98),    p(  64,  100),    p(  90,   90),    p( 107,   78),    p(  82,   80),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 187,  282),    p( 189,  323),    p( 209,  322),    p( 242,  319),    p( 286,  306),    p( 208,  313),    p( 231,  296),    p( 197,  257),
        p( 275,  315),    p( 286,  321),    p( 297,  310),    p( 300,  313),    p( 305,  307),    p( 313,  300),    p( 294,  312),    p( 279,  303),
        p( 287,  312),    p( 304,  307),    p( 309,  311),    p( 320,  312),    p( 331,  310),    p( 351,  297),    p( 295,  307),    p( 296,  306),
        p( 307,  318),    p( 315,  313),    p( 328,  315),    p( 328,  322),    p( 328,  320),    p( 330,  317),    p( 319,  318),    p( 329,  311),
        p( 300,  318),    p( 312,  309),    p( 315,  315),    p( 323,  318),    p( 321,  320),    p( 329,  306),    p( 327,  306),    p( 317,  316),
        p( 278,  305),    p( 286,  303),    p( 295,  299),    p( 304,  310),    p( 309,  309),    p( 306,  289),    p( 306,  296),    p( 300,  306),
        p( 271,  313),    p( 285,  315),    p( 287,  308),    p( 296,  309),    p( 304,  305),    p( 298,  301),    p( 302,  310),    p( 296,  319),
        p( 242,  309),    p( 280,  307),    p( 270,  307),    p( 291,  312),    p( 297,  312),    p( 297,  302),    p( 287,  309),    p( 271,  312),
    ],
    // bishop
    [
        p( 277,  318),    p( 263,  311),    p( 230,  308),    p( 226,  316),    p( 214,  315),    p( 236,  308),    p( 262,  312),    p( 260,  307),
        p( 290,  305),    p( 288,  305),    p( 296,  304),    p( 281,  306),    p( 289,  301),    p( 297,  301),    p( 258,  313),    p( 280,  304),
        p( 300,  308),    p( 306,  305),    p( 292,  308),    p( 305,  299),    p( 305,  305),    p( 335,  303),    p( 319,  306),    p( 316,  314),
        p( 289,  309),    p( 300,  305),    p( 310,  300),    p( 311,  307),    p( 310,  303),    p( 312,  304),    p( 314,  301),    p( 292,  306),
        p( 290,  306),    p( 289,  307),    p( 297,  306),    p( 311,  302),    p( 307,  303),    p( 309,  297),    p( 294,  303),    p( 316,  294),
        p( 296,  304),    p( 299,  304),    p( 299,  306),    p( 299,  306),    p( 307,  304),    p( 303,  298),    p( 310,  293),    p( 312,  300),
        p( 309,  303),    p( 299,  301),    p( 306,  301),    p( 299,  309),    p( 301,  307),    p( 308,  304),    p( 317,  296),    p( 312,  302),
        p( 298,  303),    p( 311,  303),    p( 305,  308),    p( 293,  309),    p( 306,  308),    p( 293,  311),    p( 306,  306),    p( 306,  297),
    ],
    // rook
    [
        p( 447,  562),    p( 443,  565),    p( 431,  572),    p( 428,  571),    p( 441,  565),    p( 462,  566),    p( 451,  568),    p( 490,  546),
        p( 452,  562),    p( 450,  566),    p( 456,  566),    p( 472,  558),    p( 459,  560),    p( 488,  555),    p( 491,  554),    p( 502,  541),
        p( 452,  557),    p( 469,  550),    p( 463,  553),    p( 462,  547),    p( 486,  540),    p( 504,  535),    p( 512,  538),    p( 487,  539),
        p( 449,  556),    p( 459,  552),    p( 458,  553),    p( 461,  549),    p( 468,  544),    p( 483,  541),    p( 478,  547),    p( 474,  541),
        p( 439,  554),    p( 441,  552),    p( 441,  553),    p( 449,  549),    p( 457,  547),    p( 454,  547),    p( 469,  543),    p( 455,  541),
        p( 434,  549),    p( 439,  545),    p( 438,  547),    p( 440,  547),    p( 450,  541),    p( 460,  535),    p( 474,  529),    p( 461,  532),
        p( 435,  547),    p( 440,  545),    p( 446,  545),    p( 449,  542),    p( 457,  538),    p( 470,  530),    p( 480,  525),    p( 447,  537),
        p( 442,  551),    p( 442,  544),    p( 441,  548),    p( 446,  543),    p( 452,  537),    p( 453,  541),    p( 441,  548),    p( 439,  542),
    ],
    // queen
    [
        p( 869,  989),    p( 870,  994),    p( 884, 1004),    p( 905,  995),    p( 910, 1000),    p( 936,  988),    p( 970,  942),    p( 907,  971),
        p( 901,  975),    p( 888,  992),    p( 887, 1009),    p( 878, 1031),    p( 878, 1046),    p( 918, 1018),    p( 912, 1000),    p( 956,  970),
        p( 913,  971),    p( 906,  979),    p( 903, 1002),    p( 899, 1015),    p( 901, 1026),    p( 942, 1017),    p( 955,  987),    p( 940,  990),
        p( 901,  978),    p( 903,  987),    p( 900,  992),    p( 891, 1015),    p( 894, 1023),    p( 913, 1014),    p( 917, 1018),    p( 925,  994),
        p( 896,  978),    p( 893,  985),    p( 893,  990),    p( 894, 1005),    p( 898, 1008),    p( 902, 1005),    p( 910, 1002),    p( 915,  988),
        p( 896,  963),    p( 899,  973),    p( 894,  988),    p( 892,  993),    p( 895, 1003),    p( 905,  991),    p( 913,  984),    p( 913,  963),
        p( 898,  956),    p( 894,  971),    p( 899,  975),    p( 899,  989),    p( 899,  991),    p( 903,  973),    p( 912,  956),    p( 905,  948),
        p( 878,  971),    p( 891,  959),    p( 892,  967),    p( 896,  973),    p( 898,  962),    p( 884,  974),    p( 880,  953),    p( 892,  927),
    ],
    // king
    [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  71,    3),    p(  83,   -1),    p( 102,  -12),    p( 194,  -70),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   7,   24),    p( -33,   36),    p( -24,   26),    p(  -4,   14),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -45,   36),    p( -27,   28),    p( -32,   21),    p( -35,   20),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-115,   33),    p( -91,   25),    p( -87,   12),    p( -71,   14),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-133,   27),    p(-107,   14),    p(-107,    3),    p( -95,    7),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(-110,   15),    p(-109,    8),    p( -81,   -5),    p( -57,    6),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p( -99,    6),    p( -86,   -0),    p( -58,  -14),    p(   2,   -4),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(  22,  -16),    p(   7,  -11),    p(  30,  -24),    p(  59,  -26),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 52);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(13, 18), p(14, 17), p(13, 6), p(9, -0), p(4, -7), p(0, -15), p(-6, -23), p(-12, -35), p(-23, -40)];
const ROOK_OPEN_FILE: PhasedScore = p(14, 1);
const ROOK_CLOSED_FILE: PhasedScore = p(-11, -4);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(6, -3);
const KING_OPEN_FILE: PhasedScore = p(-37, 6);
const KING_CLOSED_FILE: PhasedScore = p(12, -6);
const KING_SEMIOPEN_FILE: PhasedScore = p(-3, 11);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-3, 5), p(0, 7), p(1, 6), p(4, 5), p(5, 6), p(4, 7), p(9, 4), p(20, -0)],
    // Closed
    [p(0, 0), p(0, 0), p(18, -18), p(-14, 11), p(2, 11), p(1, 5), p(1, 7), p(1, 4)],
    // SemiOpen
    [p(0, 0), p(-6, 27), p(9, 20), p(2, 12), p(2, 11), p(5, 6), p(4, 3), p(11, 5)],
    // SemiClosed
    [p(0, 0), p(11, -11), p(10, 6), p(4, 1), p(8, 3), p(2, 4), p(5, 4), p(4, 3)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(22, 8),
    p(4, 3),
    p(2, 5),
    p(-7, 7),
    p(6, 3),
    p(-7, -9),
    p(-3, 1),
    p(-4, -8),
    p(1, -2),
    p(-10, -1),
    p(-7, -15),
    p(-15, -5),
    p(6, -4),
    p(-1, -8),
    p(7, -3),
    p(6, 17),
    p(-6, -2),
    p(-22, -4),
    p(-13, 2),
    p(-38, 23),
    p(-20, 5),
    p(-19, -16),
    p(3, 27),
    p(-45, 30),
    p(-18, -15),
    p(-21, -14),
    p(-34, -27),
    p(-39, 15),
    p(-17, 2),
    p(13, -2),
    p(-84, 100),
    p(0, 0),
    p(1, -5),
    p(-13, -6),
    p(-2, -5),
    p(-21, 0),
    p(-21, -1),
    p(-44, -18),
    p(-26, 38),
    p(-33, 32),
    p(-6, -5),
    p(-19, -9),
    p(7, -9),
    p(-16, 36),
    p(-48, 21),
    p(-0, -28),
    p(0, 0),
    p(0, 0),
    p(-4, -12),
    p(-16, 7),
    p(-7, -52),
    p(0, 0),
    p(7, -9),
    p(-38, -9),
    p(0, 0),
    p(0, 0),
    p(-29, -4),
    p(-23, -9),
    p(-19, 21),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(25, 5),
    p(4, -1),
    p(-5, 4),
    p(-21, -0),
    p(11, -4),
    p(-23, -8),
    p(-15, -1),
    p(-33, -10),
    p(8, -2),
    p(-9, -8),
    p(-28, -5),
    p(-43, 1),
    p(-1, -5),
    p(-37, -6),
    p(-31, -7),
    p(-47, 61),
    p(10, 0),
    p(-0, -8),
    p(-4, -9),
    p(-21, -0),
    p(-9, -1),
    p(-14, -12),
    p(-22, 1),
    p(-66, 180),
    p(-4, -10),
    p(-24, -14),
    p(-31, -27),
    p(11, -79),
    p(-11, -8),
    p(-10, -18),
    p(-77, 70),
    p(0, 0),
    p(12, 0),
    p(-2, -3),
    p(-17, -5),
    p(-26, -5),
    p(0, 1),
    p(-25, -16),
    p(-14, -0),
    p(-27, 3),
    p(-1, -7),
    p(-21, -7),
    p(-27, -13),
    p(-36, -3),
    p(-6, -1),
    p(-46, -4),
    p(4, 16),
    p(-57, 62),
    p(4, 1),
    p(-11, -1),
    p(-28, 60),
    p(0, 0),
    p(-13, -2),
    p(-19, 7),
    p(0, 0),
    p(0, 0),
    p(-13, 2),
    p(-38, 10),
    p(-31, -39),
    p(0, 0),
    p(18, -61),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-13, 9),   /*0b0000*/
    p(-13, 7),   /*0b0001*/
    p(-5, 11),   /*0b0010*/
    p(-4, 10),   /*0b0011*/
    p(-9, 3),    /*0b0100*/
    p(-24, -1),  /*0b0101*/
    p(-11, 4),   /*0b0110*/
    p(-13, -12), /*0b0111*/
    p(0, 5),     /*0b1000*/
    p(-9, 9),    /*0b1001*/
    p(-2, 10),   /*0b1010*/
    p(2, 10),    /*0b1011*/
    p(-5, 3),    /*0b1100*/
    p(-19, 3),   /*0b1101*/
    p(-10, 3),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(1, 10),    /*0b10000*/
    p(3, 6),     /*0b10001*/
    p(19, 7),    /*0b10010*/
    p(-2, 6),    /*0b10011*/
    p(-9, 3),    /*0b10100*/
    p(11, 12),   /*0b10101*/
    p(-30, 2),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(7, 10),    /*0b11000*/
    p(24, 11),   /*0b11001*/
    p(26, 25),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(6, -3),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(6, 3),     /*0b100000*/
    p(-1, 7),    /*0b100001*/
    p(13, 3),    /*0b100010*/
    p(5, -1),    /*0b100011*/
    p(-6, -3),   /*0b100100*/
    p(-21, -13), /*0b100101*/
    p(-21, 18),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(15, -2),   /*0b101000*/
    p(-1, 8),    /*0b101001*/
    p(9, -5),    /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-1, -1),   /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(2, 2),     /*0b110000*/
    p(12, 4),    /*0b110001*/
    p(14, -5),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(9, 13),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(18, -6),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(3, -5),    /*0b111111*/
    p(13, -1),   /*0b00*/
    p(21, -11),  /*0b01*/
    p(37, -10),  /*0b10*/
    p(12, -31),  /*0b11*/
    p(41, -5),   /*0b100*/
    p(14, -11),  /*0b101*/
    p(49, -30),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(56, -13),  /*0b1000*/
    p(11, -19),  /*0b1001*/
    p(30, -37),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(21, -20),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(-18, 25),  /*0b1111*/
    p(21, 0),    /*0b00*/
    p(31, -12),  /*0b01*/
    p(28, -15),  /*0b10*/
    p(26, -39),  /*0b11*/
    p(31, -12),  /*0b100*/
    p(43, -21),  /*0b101*/
    p(19, -20),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(34, -4),   /*0b1000*/
    p(41, -16),  /*0b1001*/
    p(46, -45),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(28, -26),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(13, -44),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(-1, -37);
const STOPPABLE_PASSER: PhasedScore = p(25, -48);
const CLOSE_KING_PASSER: PhasedScore = p(3, 25);
const IMMOBILE_PASSER: PhasedScore = p(-5, -37);
const PROTECTED_PASSER: PhasedScore = p(8, -4);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -31,   46),    p( -39,   67),    p( -54,   61),    p( -59,   44),    p( -60,   46),    p( -47,   42),    p( -31,   43),    p( -57,   46),
        p( -24,   44),    p( -45,   68),    p( -50,   59),    p( -51,   46),    p( -68,   54),    p( -62,   51),    p( -49,   57),    p( -49,   44),
        p( -18,   55),    p( -26,   55),    p( -46,   59),    p( -43,   58),    p( -54,   63),    p( -49,   64),    p( -50,   71),    p( -70,   70),
        p(  -8,   72),    p( -11,   70),    p( -11,   61),    p( -29,   74),    p( -45,   82),    p( -36,   86),    p( -40,   92),    p( -62,   94),
        p(  -4,   62),    p(  13,   51),    p(  -2,   43),    p( -16,   38),    p( -16,   54),    p( -32,   70),    p( -51,   81),    p( -81,   91),
        p(  14,   61),    p(  14,   60),    p(  19,   54),    p(  22,   49),    p(  20,   56),    p(  21,   70),    p( -14,   76),    p(  -6,   76),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(1, 4), p(5, 8), p(8, 17), p(15, 22), p(17, 71), p(13, 64)];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -6);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);
const PHALANX: [PhasedScore; 6] = [p(-1, 0), p(5, 3), p(9, 6), p(21, 20), p(58, 75), p(-81, 215)];
const PROMO_SUPPORTING_BISHOP: PhasedScore = p(-1, 9);
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 14), p(7, 17), p(14, 21), p(7, 10), p(-3, 13), p(-48, 12)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(48, 21), p(51, 45), p(66, 4), p(51, -8), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(1, -5), p(14, 20), p(19, -7), p(16, 10), p(16, -9), p(30, -12)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-40, -68),
        p(-20, -28),
        p(-8, -5),
        p(1, 9),
        p(9, 20),
        p(16, 30),
        p(24, 33),
        p(30, 36),
        p(34, 35),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-18, -37),
        p(-6, -22),
        p(1, -9),
        p(8, 1),
        p(13, 10),
        p(19, 15),
        p(23, 18),
        p(25, 24),
        p(32, 25),
        p(38, 25),
        p(44, 29),
        p(38, 39),
        p(51, 31),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-68, 14),
        p(-59, 28),
        p(-55, 35),
        p(-51, 39),
        p(-52, 46),
        p(-47, 53),
        p(-44, 58),
        p(-40, 62),
        p(-37, 67),
        p(-34, 71),
        p(-30, 74),
        p(-28, 79),
        p(-21, 81),
        p(-12, 79),
        p(-8, 76),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-19, -32),
        p(-19, 1),
        p(-24, 58),
        p(-20, 74),
        p(-18, 93),
        p(-13, 98),
        p(-10, 110),
        p(-7, 116),
        p(-3, 122),
        p(0, 124),
        p(3, 128),
        p(7, 131),
        p(11, 132),
        p(13, 137),
        p(15, 139),
        p(20, 142),
        p(22, 148),
        p(26, 149),
        p(36, 146),
        p(51, 139),
        p(57, 139),
        p(98, 117),
        p(98, 120),
        p(126, 97),
        p(215, 66),
        p(258, 28),
        p(289, 16),
        p(269, 12),
    ],
    [
        p(-89, -1),
        p(-59, -10),
        p(-31, -9),
        p(-2, -5),
        p(28, -3),
        p(51, -1),
        p(80, 5),
        p(105, 8),
        p(153, -1),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-4, 14), p(0, 0), p(28, 26), p(60, 8), p(40, -0), p(0, 0)],
    [p(-2, 12), p(21, 26), p(0, 0), p(42, 22), p(46, 92), p(0, 0)],
    [p(-4, 19), p(12, 17), p(19, 14), p(0, 0), p(66, 45), p(0, 0)],
    [p(-2, 9), p(2, 9), p(1, 25), p(-0, 13), p(0, 0), p(0, 0)],
    [p(64, 21), p(-20, 23), p(8, 17), p(-13, 23), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 9), p(8, 7), p(6, 11), p(12, 8), p(8, 14), p(3, 9)],
    [p(2, 9), p(11, 22), p(-1, -42), p(9, 12), p(10, 20), p(4, 6)],
    [p(2, 4), p(13, 9), p(9, 14), p(11, 10), p(9, 28), p(19, -3)],
    [p(2, 3), p(8, 3), p(7, -2), p(5, 13), p(-63, -220), p(5, -10)],
    [p(48, 0), p(36, 12), p(41, 6), p(26, 6), p(37, -9), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-20, -14), p(17, -9), p(9, -3), p(16, -16), p(-3, 4), p(-7, 3)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(8, 2), p(-5, 7), p(16, -8), p(-10, 24)];
const CHECK_STM: PhasedScore = p(38, 22);
const DISCOVERED_CHECK_STM: PhasedScore = p(176, 59);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(11, -7), p(62, 1), p(101, -33), p(56, 89), p(0, 0), p(-26, -23)];
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

    fn promo_supporting_bishop() -> SingleFeatureScore<Self::Score>;

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

    fn promo_supporting_bishop() -> PhasedScore {
        PROMO_SUPPORTING_BISHOP
    }
}
