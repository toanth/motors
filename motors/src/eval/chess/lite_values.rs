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

// const TEMPO: PhasedScore = p(10, 10);
const TEMPO: PhasedScore = p(24, 14);

#[rustfmt::skip]
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
        p( 129,  162),    p( 125,  161),    p( 117,  165),    p( 125,  149),    p( 112,  154),    p( 111,  157),    p(  71,  174),    p(  74,  173),
        p(  75,  119),    p(  72,  121),    p(  78,  113),    p(  85,  109),    p(  73,  104),    p( 127,  104),    p(  98,  123),    p( 100,  116),
        p(  58,  107),    p(  64,  100),    p(  61,   94),    p(  83,   96),    p(  89,   95),    p(  83,   84),    p(  78,   95),    p(  75,   91),
        p(  52,   94),    p(  53,   96),    p(  75,   91),    p(  94,   93),    p(  87,   95),    p(  83,   91),    p(  69,   86),    p(  63,   81),
        p(  44,   92),    p(  50,   88),    p(  71,   91),    p(  80,   94),    p(  82,   92),    p(  77,   91),    p(  69,   78),    p(  54,   81),
        p(  54,   96),    p(  59,   94),    p(  62,   93),    p(  57,   99),    p(  60,  101),    p(  77,   93),    p(  82,   83),    p(  60,   85),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 178,  280),    p( 194,  316),    p( 214,  326),    p( 246,  314),    p( 275,  316),    p( 196,  309),    p( 221,  307),    p( 204,  263),
        p( 271,  315),    p( 287,  320),    p( 296,  311),    p( 298,  314),    p( 298,  310),    p( 310,  301),    p( 275,  319),    p( 284,  304),
        p( 290,  312),    p( 305,  307),    p( 306,  313),    p( 316,  315),    p( 330,  311),    p( 347,  300),    p( 292,  309),    p( 286,  310),
        p( 307,  318),    p( 315,  315),    p( 329,  315),    p( 329,  323),    p( 324,  323),    p( 325,  322),    p( 317,  320),    p( 329,  312),
        p( 302,  319),    p( 309,  310),    p( 316,  316),    p( 325,  318),    p( 321,  322),    p( 329,  308),    p( 328,  308),    p( 317,  316),
        p( 279,  305),    p( 287,  305),    p( 299,  299),    p( 304,  312),    p( 309,  311),    p( 298,  293),    p( 306,  296),    p( 297,  310),
        p( 274,  312),    p( 285,  317),    p( 288,  307),    p( 296,  312),    p( 300,  307),    p( 294,  306),    p( 299,  311),    p( 293,  321),
        p( 246,  311),    p( 282,  307),    p( 271,  311),    p( 292,  314),    p( 300,  312),    p( 295,  302),    p( 288,  310),    p( 272,  310),
    ],
    // bishop
    [
        p( 277,  314),    p( 251,  317),    p( 242,  309),    p( 223,  319),    p( 213,  318),    p( 230,  308),    p( 271,  307),    p( 257,  311),
        p( 288,  305),    p( 282,  307),    p( 292,  308),    p( 277,  307),    p( 288,  303),    p( 292,  303),    p( 271,  311),    p( 275,  307),
        p( 297,  312),    p( 306,  308),    p( 288,  307),    p( 305,  301),    p( 301,  304),    p( 337,  307),    p( 316,  306),    p( 319,  314),
        p( 290,  312),    p( 298,  306),    p( 309,  302),    p( 309,  308),    p( 312,  304),    p( 310,  304),    p( 312,  305),    p( 289,  310),
        p( 289,  307),    p( 289,  308),    p( 298,  305),    p( 313,  304),    p( 305,  301),    p( 306,  300),    p( 291,  304),    p( 314,  299),
        p( 298,  309),    p( 300,  306),    p( 302,  306),    p( 300,  304),    p( 306,  307),    p( 300,  299),    p( 307,  295),    p( 309,  299),
        p( 306,  309),    p( 304,  301),    p( 307,  303),    p( 298,  310),    p( 300,  308),    p( 302,  307),    p( 312,  297),    p( 308,  295),
        p( 299,  306),    p( 309,  308),    p( 306,  309),    p( 290,  313),    p( 306,  311),    p( 293,  313),    p( 302,  301),    p( 303,  294),
    ],
    // rook
    [
        p( 454,  554),    p( 440,  565),    p( 431,  572),    p( 431,  568),    p( 443,  564),    p( 467,  559),    p( 469,  560),    p( 483,  553),
        p( 454,  559),    p( 452,  564),    p( 459,  565),    p( 474,  555),    p( 460,  558),    p( 481,  553),    p( 484,  552),    p( 502,  541),
        p( 451,  555),    p( 469,  549),    p( 464,  550),    p( 464,  545),    p( 491,  537),    p( 504,  533),    p( 518,  534),    p( 493,  536),
        p( 450,  555),    p( 460,  550),    p( 460,  552),    p( 462,  548),    p( 469,  541),    p( 484,  536),    p( 482,  543),    p( 477,  538),
        p( 441,  552),    p( 445,  551),    p( 446,  550),    p( 453,  547),    p( 457,  544),    p( 454,  544),    p( 466,  539),    p( 457,  537),
        p( 437,  549),    p( 440,  546),    p( 442,  545),    p( 444,  544),    p( 451,  539),    p( 461,  531),    p( 477,  522),    p( 460,  525),
        p( 438,  546),    p( 443,  543),    p( 449,  544),    p( 453,  541),    p( 459,  535),    p( 473,  526),    p( 479,  522),    p( 448,  531),
        p( 444,  549),    p( 443,  545),    p( 444,  548),    p( 448,  541),    p( 454,  534),    p( 459,  535),    p( 455,  535),    p( 448,  537),
    ],
    // queen
    [
        p( 869,  966),    p( 871,  979),    p( 883,  992),    p( 904,  983),    p( 908,  986),    p( 929,  980),    p( 966,  935),    p( 911,  970),
        p( 900,  958),    p( 878,  983),    p( 879, 1005),    p( 873, 1019),    p( 879, 1033),    p( 914, 1002),    p( 927,  978),    p( 956,  969),
        p( 899,  967),    p( 895,  976),    p( 892,  997),    p( 895, 1003),    p( 898, 1016),    p( 956,  991),    p( 957,  970),    p( 947,  976),
        p( 892,  973),    p( 897,  978),    p( 895,  986),    p( 889, 1001),    p( 895, 1010),    p( 914,  995),    p( 921, 1000),    p( 925,  981),
        p( 895,  964),    p( 890,  979),    p( 891,  979),    p( 893,  994),    p( 898,  990),    p( 902,  990),    p( 911,  983),    p( 918,  976),
        p( 893,  950),    p( 898,  963),    p( 895,  976),    p( 891,  978),    p( 897,  987),    p( 903,  975),    p( 916,  963),    p( 913,  952),
        p( 891,  951),    p( 893,  957),    p( 899,  960),    p( 898,  975),    p( 900,  974),    p( 902,  958),    p( 913,  939),    p( 921,  914),
        p( 878,  955),    p( 891,  943),    p( 890,  956),    p( 896,  957),    p( 901,  948),    p( 888,  952),    p( 889,  943),    p( 896,  928),
    ],
    // king
    [
        p( 179,  -65),    p(  95,  -15),    p( 116,   -6),    p(  52,   13),    p(  65,    4),    p(  28,   17),    p(  83,    5),    p( 221,  -67),
        p(  -3,   16),    p( -37,   31),    p( -41,   38),    p(  17,   24),    p( -14,   33),    p( -36,   45),    p( -23,   35),    p(   9,   19),
        p( -35,   23),    p( -24,   23),    p( -59,   37),    p( -68,   41),    p( -31,   38),    p(   2,   30),    p( -51,   33),    p( -13,   22),
        p( -17,    8),    p( -79,   17),    p( -95,   31),    p(-122,   37),    p(-115,   37),    p( -97,   30),    p(-104,   21),    p( -95,   24),
        p( -38,    0),    p( -95,    6),    p(-111,   22),    p(-137,   32),    p(-134,   31),    p(-110,   18),    p(-126,   11),    p(-111,   15),
        p( -35,    4),    p( -80,    0),    p(-106,   13),    p(-114,   19),    p(-111,   19),    p(-121,   13),    p( -98,    2),    p( -68,   10),
        p(  24,   -5),    p( -71,   -5),    p( -80,    1),    p( -98,    7),    p(-105,   11),    p( -90,    2),    p( -66,  -12),    p(   1,   -3),
        p(  50,  -15),    p(  41,  -29),    p(  37,  -17),    p( -21,   -2),    p(  25,  -15),    p( -17,   -3),    p(  32,  -24),    p(  61,  -29),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 51);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(12, 22), p(14, 19), p(13, 8), p(9, 1), p(4, -6), p(1, -15), p(-5, -23), p(-12, -35), p(-23, -41)];
const ROOK_OPEN_FILE: PhasedScore = p(15, 1);
const ROOK_CLOSED_FILE: PhasedScore = p(-11, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(5, -2);
const KING_OPEN_FILE: PhasedScore = p(-41, 5);
const KING_CLOSED_FILE: PhasedScore = p(12, -7);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 10);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-4, 6), p(2, 8), p(-1, 7), p(4, 5), p(5, 6), p(5, 8), p(8, 5), p(19, 2)],
    // Closed
    [p(0, 0), p(0, 0), p(17, -11), p(-13, 11), p(1, 12), p(1, 6), p(0, 8), p(1, 5)],
    // SemiOpen
    [p(0, 0), p(-5, 27), p(7, 21), p(3, 12), p(2, 12), p(5, 7), p(3, 3), p(11, 6)],
    // SemiClosed
    [p(0, 0), p(12, -11), p(7, 7), p(4, 0), p(8, 2), p(2, 5), p(5, 5), p(3, 5)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(21, 11),
    p(1, 5),
    p(-5, 6),
    p(-15, 8),
    p(5, 3),
    p(-9, -10),
    p(-5, -2),
    p(-7, -11),
    p(2, 0),
    p(-11, -1),
    p(-12, -15),
    p(-21, -6),
    p(8, -7),
    p(0, -11),
    p(9, -9),
    p(5, 8),
    p(-6, -3),
    p(-23, -4),
    p(-19, 2),
    p(-44, 23),
    p(-18, 2),
    p(-18, -19),
    p(5, 22),
    p(-48, 23),
    p(-16, -18),
    p(-20, -16),
    p(-37, -30),
    p(-43, 12),
    p(-11, -4),
    p(16, -5),
    p(-92, 97),
    p(0, 0),
    p(-1, -3),
    p(-16, -5),
    p(-9, -5),
    p(-29, -0),
    p(-23, -2),
    p(-47, -21),
    p(-32, 37),
    p(-44, 28),
    p(-6, -4),
    p(-20, -9),
    p(3, -10),
    p(-20, 35),
    p(-44, 16),
    p(4, -33),
    p(0, 0),
    p(0, 0),
    p(-2, -14),
    p(-17, 6),
    p(-15, -52),
    p(0, 0),
    p(8, -11),
    p(-40, -16),
    p(0, 0),
    p(0, 0),
    p(-27, -5),
    p(-22, -11),
    p(-24, 17),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(24, 4),
    p(2, -1),
    p(-5, 1),
    p(-22, -4),
    p(6, -4),
    p(-29, -8),
    p(-19, -4),
    p(-38, -12),
    p(7, -3),
    p(-13, -8),
    p(-31, -8),
    p(-48, -1),
    p(-10, -5),
    p(-46, -5),
    p(-40, -14),
    p(-58, 60),
    p(9, -3),
    p(-2, -9),
    p(-5, -13),
    p(-24, -4),
    p(-12, -2),
    p(-20, -11),
    p(-26, -2),
    p(-75, 168),
    p(-7, -12),
    p(-29, -14),
    p(-39, -29),
    p(7, -84),
    p(-18, -10),
    p(-19, -17),
    p(-85, 62),
    p(0, 0),
    p(15, -2),
    p(1, -4),
    p(-14, -10),
    p(-23, -10),
    p(-1, -0),
    p(-28, -15),
    p(-16, -3),
    p(-29, 0),
    p(-0, -9),
    p(-22, -8),
    p(-26, -17),
    p(-38, -8),
    p(-10, -4),
    p(-48, -10),
    p(-7, 14),
    p(-62, 56),
    p(3, -3),
    p(-12, -4),
    p(-29, 53),
    p(0, 0),
    p(-18, -4),
    p(-27, 7),
    p(0, 0),
    p(0, 0),
    p(-16, -1),
    p(-42, 8),
    p(-36, -48),
    p(0, 0),
    p(7, -62),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-4, 8),    /*0b0000*/
    p(-13, 8),   /*0b0001*/
    p(-4, 11),   /*0b0010*/
    p(-9, 12),   /*0b0011*/
    p(-2, 3),    /*0b0100*/
    p(-28, 3),   /*0b0101*/
    p(-13, 5),   /*0b0110*/
    p(-20, -11), /*0b0111*/
    p(14, 5),    /*0b1000*/
    p(-3, 11),   /*0b1001*/
    p(3, 11),    /*0b1010*/
    p(-1, 14),   /*0b1011*/
    p(2, 5),     /*0b1100*/
    p(-23, 7),   /*0b1101*/
    p(-9, 6),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(10, 10),   /*0b10000*/
    p(6, 8),     /*0b10001*/
    p(20, 11),   /*0b10010*/
    p(-2, 7),    /*0b10011*/
    p(-2, 3),    /*0b10100*/
    p(13, 13),   /*0b10101*/
    p(-20, 2),   /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(19, 10),   /*0b11000*/
    p(29, 13),   /*0b11001*/
    p(39, 25),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(15, -0),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(21, 3),    /*0b100000*/
    p(5, 10),    /*0b100001*/
    p(23, 4),    /*0b100010*/
    p(8, 0),     /*0b100011*/
    p(-6, 1),    /*0b100100*/
    p(-21, -6),  /*0b100101*/
    p(-23, 20),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(32, -1),   /*0b101000*/
    p(5, 12),    /*0b101001*/
    p(22, -4),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(1, 5),     /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(19, 3),    /*0b110000*/
    p(23, 5),    /*0b110001*/
    p(33, -4),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(3, 18),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(34, -3),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(6, -2),    /*0b111111*/
    p(-7, 4),    /*0b00*/
    p(6, -12),   /*0b01*/
    p(40, -8),   /*0b10*/
    p(25, -44),  /*0b11*/
    p(47, -11),  /*0b100*/
    p(-4, -13),  /*0b101*/
    p(67, -41),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(73, -14),  /*0b1000*/
    p(18, -27),  /*0b1001*/
    p(71, -50),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(62, -34),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(29, 3),    /*0b1111*/
    p(24, 1),    /*0b00*/
    p(33, -9),   /*0b01*/
    p(27, -14),  /*0b10*/
    p(22, -38),  /*0b11*/
    p(41, -8),   /*0b100*/
    p(56, -18),  /*0b101*/
    p(25, -19),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(42, -3),   /*0b1000*/
    p(51, -15),  /*0b1001*/
    p(54, -41),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(46, -24),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(21, -42),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(-25, -34);
const STOPPABLE_PASSER: PhasedScore = p(36, -49);
const CLOSE_KING_PASSER: PhasedScore = p(-8, 28);
const IMMOBILE_PASSER: PhasedScore = p(-3, -37);
const PROTECTED_PASSER: PhasedScore = p(8, -3);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -41,   40),    p( -49,   54),    p( -67,   57),    p( -68,   51),    p( -60,   40),    p( -47,   40),    p( -33,   45),    p( -48,   47),
        p( -31,   36),    p( -54,   58),    p( -63,   55),    p( -61,   47),    p( -69,   49),    p( -58,   48),    p( -60,   63),    p( -43,   45),
        p( -28,   55),    p( -33,   57),    p( -58,   62),    p( -55,   62),    p( -63,   59),    p( -53,   60),    p( -60,   70),    p( -60,   67),
        p( -15,   73),    p( -16,   75),    p( -16,   67),    p( -39,   76),    p( -56,   77),    p( -45,   76),    p( -53,   85),    p( -59,   87),
        p(   2,   69),    p(   7,   68),    p(   1,   53),    p( -19,   40),    p( -16,   51),    p( -40,   58),    p( -49,   59),    p( -84,   80),
        p(  29,   62),    p(  25,   61),    p(  17,   65),    p(  25,   49),    p(  12,   54),    p(  11,   57),    p( -29,   74),    p( -26,   73),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(1, 5), p(5, 8), p(8, 17), p(15, 22), p(17, 71), p(15, 62)];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -7);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);
const PHALANX: [PhasedScore; 6] = [p(-0, -0), p(5, 3), p(8, 6), p(21, 21), p(56, 76), p(-92, 221)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 14), p(8, 18), p(15, 21), p(7, 10), p(-3, 14), p(-44, 11)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(49, 21), p(52, 45), p(67, 4), p(52, -9), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, -5), p(14, 21), p(19, -7), p(16, 10), p(16, -9), p(28, -11)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-43, -68),
        p(-22, -28),
        p(-10, -5),
        p(-1, 8),
        p(7, 19),
        p(14, 30),
        p(22, 33),
        p(28, 35),
        p(32, 34),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-30, -56),
        p(-17, -37),
        p(-6, -22),
        p(1, -10),
        p(8, 0),
        p(14, 9),
        p(19, 14),
        p(24, 18),
        p(25, 24),
        p(32, 25),
        p(38, 26),
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
        p(-69, 15),
        p(-60, 29),
        p(-56, 36),
        p(-54, 41),
        p(-54, 48),
        p(-49, 54),
        p(-46, 59),
        p(-43, 63),
        p(-40, 68),
        p(-37, 72),
        p(-33, 75),
        p(-32, 81),
        p(-24, 82),
        p(-16, 80),
        p(-11, 76),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-19, -21),
        p(-16, 7),
        p(-21, 64),
        p(-17, 81),
        p(-15, 100),
        p(-10, 106),
        p(-6, 117),
        p(-3, 124),
        p(1, 130),
        p(4, 131),
        p(7, 135),
        p(12, 139),
        p(15, 140),
        p(17, 145),
        p(19, 148),
        p(24, 151),
        p(26, 157),
        p(29, 159),
        p(39, 156),
        p(53, 149),
        p(59, 150),
        p(100, 127),
        p(101, 130),
        p(126, 109),
        p(213, 79),
        p(266, 34),
        p(290, 27),
        p(289, 15),
    ],
    [
        p(-87, 3),
        p(-54, -10),
        p(-27, -9),
        p(1, -5),
        p(30, -3),
        p(52, -1),
        p(80, 5),
        p(105, 8),
        p(150, -1),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-4, 14), p(0, 0), p(28, 27), p(60, 8), p(40, 0), p(0, 0)],
    [p(-2, 12), p(21, 26), p(0, 0), p(42, 23), p(47, 92), p(0, 0)],
    [p(-3, 18), p(11, 18), p(19, 14), p(0, 0), p(66, 44), p(0, 0)],
    [p(-2, 9), p(2, 9), p(1, 25), p(0, 12), p(0, 0), p(0, 0)],
    [p(59, 21), p(-22, 23), p(15, 15), p(-13, 23), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 8), p(8, 7), p(6, 10), p(13, 7), p(7, 15), p(11, 6)],
    [p(2, 9), p(12, 21), p(-24, -41), p(8, 12), p(10, 20), p(3, 6)],
    [p(2, 4), p(12, 8), p(8, 13), p(11, 10), p(9, 28), p(20, -4)],
    [p(2, 3), p(8, 4), p(6, -2), p(4, 14), p(-59, -224), p(5, -10)],
    [p(62, -2), p(40, 10), p(44, 5), p(22, 7), p(34, -4), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-20, -13), p(20, -10), p(9, -3), p(15, -15), p(-0, 4), p(-1, 2)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(11, -0), p(-5, 8), p(15, -8), p(-13, 24)];
const CHECK_STM: PhasedScore = p(36, 22);
const DISCOVERED_CHECK_STM: PhasedScore = p(144, 38);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(-7, -18), p(67, -1), p(104, -32), p(64, 81), p(0, 0), p(-23, -21)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(6, -17), p(26, 30), p(16, 34), p(42, 9), p(64, 2)];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static + StaticallyNamedEntity {
    type Score: ScoreType;

    fn tempo() -> SingleFeatureScore<Self::Score>;

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

    fn check_stm() -> SingleFeatureScore<Self::Score>;

    fn discovered_check_stm() -> SingleFeatureScore<Self::Score>;

    fn discovered_check(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;

    fn pin(piece: ChessPieceType) -> SingleFeatureScore<Self::Score>;
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

    fn tempo() -> PhasedScore {
        TEMPO
    }

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

    fn check_stm() -> PhasedScore {
        CHECK_STM
    }
}
