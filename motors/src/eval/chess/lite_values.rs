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
        p( 131,  187),    p( 128,  185),    p( 119,  190),    p( 128,  172),    p( 115,  178),    p( 116,  181),    p(  76,  198),    p(  81,  196),
        p(  71,  120),    p(  72,  122),    p(  80,  115),    p(  88,  116),    p(  79,  113),    p( 127,  104),    p( 104,  125),    p(  98,  116),
        p(  53,  110),    p(  64,  103),    p(  63,   99),    p(  84,   98),    p(  93,   98),    p(  85,   87),    p(  78,   97),    p(  73,   92),
        p(  49,   97),    p(  54,   99),    p(  78,   93),    p(  93,   95),    p(  91,   97),    p(  87,   93),    p(  71,   88),    p(  61,   83),
        p(  43,   95),    p(  51,   90),    p(  72,   95),    p(  82,   97),    p(  84,   95),    p(  77,   93),    p(  70,   79),    p(  53,   82),
        p(  54,   99),    p(  59,   96),    p(  63,   96),    p(  60,  103),    p(  62,  105),    p(  77,   96),    p(  82,   84),    p(  59,   87),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 173,  280),    p( 195,  312),    p( 213,  324),    p( 252,  311),    p( 282,  314),    p( 199,  308),    p( 214,  308),    p( 201,  262),
        p( 267,  311),    p( 284,  317),    p( 300,  308),    p( 305,  313),    p( 303,  308),    p( 316,  297),    p( 275,  314),    p( 271,  304),
        p( 287,  308),    p( 307,  303),    p( 310,  310),    p( 323,  314),    p( 339,  308),    p( 354,  296),    p( 294,  303),    p( 287,  307),
        p( 303,  315),    p( 310,  309),    p( 327,  313),    p( 328,  320),    p( 326,  318),    p( 322,  317),    p( 312,  312),    p( 320,  310),
        p( 299,  318),    p( 305,  307),    p( 314,  313),    p( 322,  316),    p( 320,  319),    p( 326,  303),    p( 323,  304),    p( 312,  313),
        p( 276,  304),    p( 283,  303),    p( 297,  297),    p( 302,  310),    p( 306,  308),    p( 296,  291),    p( 302,  294),    p( 293,  308),
        p( 270,  313),    p( 281,  315),    p( 286,  304),    p( 295,  308),    p( 299,  303),    p( 290,  302),    p( 295,  306),    p( 289,  322),
        p( 239,  311),    p( 282,  305),    p( 267,  306),    p( 288,  311),    p( 296,  308),    p( 292,  298),    p( 288,  308),    p( 264,  312),
    ],
    // bishop
    [
        p( 276,  310),    p( 251,  314),    p( 240,  307),    p( 223,  317),    p( 217,  314),    p( 225,  308),    p( 273,  303),    p( 250,  309),
        p( 283,  302),    p( 279,  303),    p( 290,  306),    p( 277,  304),    p( 289,  301),    p( 292,  299),    p( 268,  308),    p( 271,  302),
        p( 297,  309),    p( 308,  304),    p( 291,  304),    p( 307,  299),    p( 306,  300),    p( 336,  304),    p( 317,  301),    p( 318,  313),
        p( 287,  312),    p( 292,  307),    p( 304,  302),    p( 307,  306),    p( 306,  304),    p( 299,  305),    p( 298,  309),    p( 280,  310),
        p( 290,  308),    p( 284,  309),    p( 296,  304),    p( 309,  305),    p( 302,  301),    p( 299,  303),    p( 285,  305),    p( 309,  302),
        p( 296,  310),    p( 299,  305),    p( 299,  307),    p( 299,  305),    p( 305,  307),    p( 298,  300),    p( 305,  296),    p( 307,  300),
        p( 308,  309),    p( 304,  300),    p( 308,  301),    p( 299,  309),    p( 301,  306),    p( 302,  305),    p( 312,  295),    p( 308,  296),
        p( 298,  305),    p( 309,  307),    p( 308,  307),    p( 290,  310),    p( 306,  308),    p( 294,  310),    p( 302,  298),    p( 302,  293),
    ],
    // rook
    [
        p( 461,  547),    p( 449,  557),    p( 443,  564),    p( 441,  561),    p( 453,  557),    p( 473,  551),    p( 483,  550),    p( 494,  542),
        p( 444,  553),    p( 442,  559),    p( 451,  560),    p( 466,  551),    p( 452,  554),    p( 468,  548),    p( 477,  545),    p( 492,  534),
        p( 446,  548),    p( 465,  543),    p( 459,  545),    p( 459,  540),    p( 485,  530),    p( 495,  527),    p( 512,  526),    p( 487,  528),
        p( 443,  549),    p( 449,  544),    p( 448,  546),    p( 454,  541),    p( 458,  533),    p( 469,  529),    p( 469,  532),    p( 469,  528),
        p( 436,  547),    p( 435,  544),    p( 436,  545),    p( 441,  541),    p( 448,  537),    p( 442,  536),    p( 455,  530),    p( 449,  529),
        p( 431,  544),    p( 431,  540),    p( 433,  539),    p( 436,  539),    p( 441,  534),    p( 452,  525),    p( 468,  514),    p( 455,  518),
        p( 433,  539),    p( 437,  537),    p( 443,  538),    p( 445,  535),    p( 452,  528),    p( 465,  519),    p( 473,  514),    p( 444,  524),
        p( 443,  543),    p( 439,  538),    p( 440,  541),    p( 445,  536),    p( 451,  529),    p( 457,  529),    p( 453,  528),    p( 448,  531),
    ],
    // queen
    [
        p( 879,  962),    p( 880,  976),    p( 895,  989),    p( 916,  983),    p( 913,  988),    p( 933,  975),    p( 979,  927),    p( 925,  958),
        p( 891,  948),    p( 865,  980),    p( 866, 1008),    p( 859, 1025),    p( 865, 1037),    p( 905,  998),    p( 907,  981),    p( 948,  960),
        p( 896,  953),    p( 888,  970),    p( 887,  992),    p( 886, 1004),    p( 909, 1005),    p( 947,  990),    p( 954,  961),    p( 943,  967),
        p( 883,  966),    p( 887,  975),    p( 880,  987),    p( 881,  999),    p( 883, 1012),    p( 896, 1002),    p( 906, 1003),    p( 913,  978),
        p( 891,  959),    p( 878,  980),    p( 884,  980),    p( 884,  997),    p( 888,  993),    p( 889,  994),    p( 902,  983),    p( 909,  975),
        p( 887,  949),    p( 893,  965),    p( 887,  980),    p( 884,  983),    p( 889,  991),    p( 896,  979),    p( 910,  962),    p( 908,  950),
        p( 887,  952),    p( 887,  959),    p( 894,  962),    p( 893,  975),    p( 895,  974),    p( 895,  958),    p( 908,  936),    p( 915,  910),
        p( 873,  953),    p( 885,  940),    p( 886,  952),    p( 894,  954),    p( 897,  943),    p( 883,  947),    p( 885,  940),    p( 890,  924),
    ],
    // king
    [
        p( 149,  -68),    p(  53,  -18),    p(  77,  -10),    p(   9,   17),    p(  39,    3),    p(  18,   16),    p(  75,    6),    p( 219,  -71),
        p( -39,   20),    p( -74,   40),    p( -75,   47),    p( -10,   35),    p( -43,   44),    p( -65,   57),    p( -38,   44),    p(   4,   19),
        p( -60,   28),    p( -49,   32),    p( -85,   44),    p( -96,   52),    p( -63,   47),    p( -31,   40),    p( -70,   42),    p( -38,   26),
        p( -40,   16),    p(-101,   26),    p(-115,   39),    p(-138,   46),    p(-136,   45),    p(-115,   38),    p(-131,   30),    p(-109,   28),
        p( -47,    5),    p(-109,   14),    p(-122,   29),    p(-147,   40),    p(-147,   38),    p(-124,   26),    p(-138,   18),    p(-121,   20),
        p( -33,    3),    p( -82,    4),    p(-110,   18),    p(-120,   26),    p(-117,   25),    p(-126,   17),    p(-101,    5),    p( -74,   13),
        p(  27,  -10),    p( -69,   -4),    p( -80,    3),    p(-100,   12),    p(-106,   13),    p( -91,    5),    p( -64,  -11),    p(   2,   -4),
        p(  50,  -28),    p(  41,  -36),    p(  39,  -25),    p( -21,   -8),    p(  29,  -23),    p( -18,  -10),    p(  33,  -31),    p(  60,  -36),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 52);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(9, 20), p(10, 18), p(10, 7), p(6, -1), p(2, -9), p(-1, -18), p(-7, -27), p(-14, -40), p(-25, -49)];
const ROOK_OPEN_FILE: PhasedScore = p(13, 5);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, 1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 5);
const KING_OPEN_FILE: PhasedScore = p(-48, -2);
const KING_CLOSED_FILE: PhasedScore = p(14, -14);
const KING_SEMIOPEN_FILE: PhasedScore = p(-8, 6);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-5, 4), p(1, 6), p(-1, 4), p(2, 3), p(3, 4), p(4, 6), p(7, 4), p(19, 1)],
    // Closed
    [p(0, 0), p(0, 0), p(21, -29), p(-14, 11), p(-1, 11), p(0, 5), p(-2, 8), p(-1, 5)],
    // SemiOpen
    [p(0, 0), p(-14, 24), p(3, 16), p(0, 10), p(-0, 9), p(4, 5), p(1, 2), p(10, 5)],
    // SemiClosed
    [p(0, 0), p(12, -13), p(7, 6), p(3, 0), p(6, 1), p(1, 4), p(3, 5), p(2, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(22, 6),
    p(2, 4),
    p(1, 3),
    p(-7, 11),
    p(6, 0),
    p(-10, -8),
    p(1, 3),
    p(-4, -5),
    p(1, -3),
    p(-11, 1),
    p(-9, -15),
    p(-17, 3),
    p(6, -6),
    p(-3, -2),
    p(8, -0),
    p(2, 25),
    p(-3, -3),
    p(-21, -1),
    p(-17, -3),
    p(-47, 20),
    p(-17, 4),
    p(-17, -17),
    p(9, 18),
    p(-57, 28),
    p(-16, -15),
    p(-21, -9),
    p(-39, -31),
    p(-41, 11),
    p(-21, 4),
    p(9, 2),
    p(-97, 117),
    p(0, 0),
    p(1, -2),
    p(-15, -0),
    p(-3, -2),
    p(-26, 8),
    p(-27, -8),
    p(-54, -21),
    p(-35, 36),
    p(-49, 33),
    p(-7, -1),
    p(-22, -1),
    p(5, -7),
    p(-21, 46),
    p(-55, 14),
    p(-13, -31),
    p(0, 0),
    p(0, 0),
    p(9, -3),
    p(-7, 21),
    p(-3, -53),
    p(0, 0),
    p(5, -11),
    p(-42, -10),
    p(0, 0),
    p(0, 0),
    p(-23, 8),
    p(-18, 8),
    p(-11, 16),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(21, 2),
    p(-1, -2),
    p(-5, 2),
    p(-21, -1),
    p(5, -3),
    p(-28, -4),
    p(-18, 3),
    p(-37, -6),
    p(4, -3),
    p(-15, -7),
    p(-28, -4),
    p(-43, 5),
    p(-10, -2),
    p(-44, 2),
    p(-38, -4),
    p(-54, 68),
    p(9, -3),
    p(-3, -5),
    p(-7, -11),
    p(-29, -5),
    p(-10, 2),
    p(-19, -7),
    p(-25, -4),
    p(-86, 182),
    p(-7, -10),
    p(-28, -10),
    p(-40, -27),
    p(3, -87),
    p(-17, -4),
    p(-20, -10),
    p(-81, 63),
    p(0, 0),
    p(15, -2),
    p(2, -2),
    p(-10, -5),
    p(-20, -7),
    p(-2, 1),
    p(-28, -12),
    p(-16, -1),
    p(-30, 2),
    p(1, -6),
    p(-21, -5),
    p(-25, -13),
    p(-37, -4),
    p(-11, -3),
    p(-49, -12),
    p(-16, 20),
    p(-64, 52),
    p(8, 1),
    p(-8, 0),
    p(-24, 56),
    p(0, 0),
    p(-16, 0),
    p(-21, -0),
    p(0, 0),
    p(0, 0),
    p(-12, 4),
    p(-38, 16),
    p(-33, -44),
    p(0, 0),
    p(10, -67),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-7, 6),    /*0b0000*/
    p(-14, 9),   /*0b0001*/
    p(-4, 9),    /*0b0010*/
    p(-7, 9),    /*0b0011*/
    p(-3, 4),    /*0b0100*/
    p(-26, 1),   /*0b0101*/
    p(-10, 2),   /*0b0110*/
    p(-16, -17), /*0b0111*/
    p(8, 14),    /*0b1000*/
    p(-2, 12),   /*0b1001*/
    p(1, 12),    /*0b1010*/
    p(0, 8),     /*0b1011*/
    p(0, 8),     /*0b1100*/
    p(-24, 9),   /*0b1101*/
    p(-9, 4),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(6, 17),    /*0b10000*/
    p(4, 11),    /*0b10001*/
    p(19, 12),   /*0b10010*/
    p(-3, 7),    /*0b10011*/
    p(-4, 6),    /*0b10100*/
    p(13, 12),   /*0b10101*/
    p(-19, -1),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(14, 20),   /*0b11000*/
    p(25, 18),   /*0b11001*/
    p(35, 31),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(13, 5),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(16, 12),   /*0b100000*/
    p(4, 15),    /*0b100001*/
    p(23, 5),    /*0b100010*/
    p(9, -1),    /*0b100011*/
    p(-6, 4),    /*0b100100*/
    p(-22, -6),  /*0b100101*/
    p(-20, 14),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(25, 9),    /*0b101000*/
    p(1, 20),    /*0b101001*/
    p(19, -0),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-3, 11),   /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(14, 13),   /*0b110000*/
    p(21, 9),    /*0b110001*/
    p(27, 3),    /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(2, 20),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(25, 9),    /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(7, -3),    /*0b111111*/
    p(-13, -1),  /*0b00*/
    p(2, -8),    /*0b01*/
    p(34, 0),    /*0b10*/
    p(21, -38),  /*0b11*/
    p(39, 2),    /*0b100*/
    p(-4, -7),   /*0b101*/
    p(62, -32),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(61, -2),   /*0b1000*/
    p(13, -23),  /*0b1001*/
    p(72, -47),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(52, -19),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(16, -3),   /*0b1111*/
    p(20, -2),   /*0b00*/
    p(31, -10),  /*0b01*/
    p(25, -15),  /*0b10*/
    p(22, -41),  /*0b11*/
    p(36, -4),   /*0b100*/
    p(51, -16),  /*0b101*/
    p(22, -19),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(37, 2),    /*0b1000*/
    p(49, -15),  /*0b1001*/
    p(50, -38),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(39, -20),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(20, -44),  /*0b1111*/
];
const REACHABLE_PAWN: PhasedScore = p(36, -49);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -23,   40),    p( -38,   62),    p( -47,   65),    p( -42,   53),    p( -30,   48),    p( -31,   55),    p( -25,   64),    p( -29,   58),
        p( -14,   41),    p( -45,   68),    p( -49,   64),    p( -47,   56),    p( -48,   62),    p( -41,   64),    p( -49,   82),    p( -24,   61),
        p(  -7,   67),    p( -20,   73),    p( -45,   77),    p( -39,   72),    p( -49,   75),    p( -38,   81),    p( -47,   93),    p( -40,   87),
        p(  11,   88),    p(   4,   92),    p(   7,   81),    p( -14,   85),    p( -34,   88),    p( -23,   95),    p( -34,  107),    p( -35,  106),
        p(  27,  132),    p(  31,  132),    p(  23,  115),    p(   5,   93),    p(   5,  103),    p( -13,  122),    p( -31,  123),    p( -57,  144),
        p(  31,   87),    p(  28,   85),    p(  19,   90),    p(  28,   72),    p(  15,   78),    p(  16,   81),    p( -24,   98),    p( -19,   96),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -9);
const DOUBLED_PAWN: PhasedScore = p(-7, -22);
const PHALANX: [PhasedScore; 6] = [p(-0, -1), p(5, 2), p(9, 6), p(24, 25), p(63, 84), p(-94, 228)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 12), p(7, 14), p(14, 18), p(9, 7), p(-3, 15), p(-47, 8)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(39, 8), p(40, 33), p(52, -11), p(35, -35), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-44, -74),
        p(-24, -32),
        p(-12, -9),
        p(-3, 4),
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
        p(-18, -38),
        p(-7, -23),
        p(-0, -10),
        p(7, -0),
        p(12, 8),
        p(17, 13),
        p(21, 18),
        p(23, 22),
        p(30, 24),
        p(35, 24),
        p(43, 27),
        p(39, 34),
        p(53, 27),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-75, 13),
        p(-66, 27),
        p(-62, 33),
        p(-59, 37),
        p(-59, 44),
        p(-53, 49),
        p(-50, 53),
        p(-46, 55),
        p(-42, 59),
        p(-38, 63),
        p(-34, 65),
        p(-33, 70),
        p(-25, 70),
        p(-16, 68),
        p(-15, 68),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-27, -43),
        p(-26, 10),
        p(-31, 61),
        p(-26, 80),
        p(-24, 98),
        p(-20, 104),
        p(-16, 114),
        p(-13, 120),
        p(-8, 124),
        p(-5, 125),
        p(-2, 127),
        p(2, 129),
        p(5, 130),
        p(7, 134),
        p(9, 135),
        p(13, 138),
        p(14, 145),
        p(16, 146),
        p(25, 144),
        p(39, 138),
        p(42, 140),
        p(86, 117),
        p(85, 120),
        p(106, 103),
        p(198, 71),
        p(243, 30),
        p(265, 24),
        p(330, -20),
    ],
    [
        p(-84, 8),
        p(-52, -5),
        p(-26, -6),
        p(1, -4),
        p(30, -3),
        p(51, -3),
        p(77, 3),
        p(100, 2),
        p(147, -15),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-3, 13), p(10, 15), p(17, 12), p(0, 0), p(45, -6), p(0, 0)],
    [p(-2, 4), p(2, 6), p(-1, 22), p(1, 1), p(0, 0), p(0, 0)],
    [p(72, 23), p(-35, 18), p(-8, 17), p(-20, 6), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 7), p(8, 7), p(6, 11), p(12, 7), p(6, 19), p(10, 6)],
    [p(2, 7), p(11, 22), p(-138, -17), p(8, 14), p(9, 20), p(3, 7)],
    [p(3, 1), p(14, 6), p(10, 11), p(12, 8), p(11, 21), p(21, -5)],
    [p(2, -2), p(9, 1), p(7, -4), p(4, 15), p(-61, -256), p(5, -11)],
    [p(58, 1), p(38, 6), p(44, 0), p(21, 5), p(34, -11), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-21, -11), p(19, -9), p(10, -3), p(15, -12), p(-1, 13), p(-8, 10)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(14, 19), p(34, -0), p(5, 32)];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static + StaticallyNamedEntity {
    type Score: ScoreType;

    fn psqt(&self, square: ChessSquare, piece: ChessPieceType, color: ChessColor) -> SingleFeatureScore<Self::Score>;

    fn passed_pawn(square: ChessSquare) -> SingleFeatureScore<Self::Score>;

    fn reachable_pawn() -> SingleFeatureScore<Self::Score>;

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

    fn reachable_pawn() -> PhasedScore {
        REACHABLE_PAWN
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
