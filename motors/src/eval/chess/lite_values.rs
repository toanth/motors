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
        p( 136,  189),    p( 132,  188),    p( 124,  192),    p( 135,  175),    p( 122,  179),    p( 120,  183),    p(  79,  199),    p(  83,  199),
        p(  70,  121),    p(  72,  123),    p(  82,  115),    p(  89,  116),    p(  86,  115),    p( 128,  105),    p( 106,  124),    p(  97,  117),
        p(  54,  111),    p(  65,  105),    p(  63,  100),    p(  84,   99),    p(  92,   99),    p(  86,   89),    p(  79,   99),    p(  74,   93),
        p(  48,   98),    p(  54,  101),    p(  78,   93),    p(  92,   96),    p(  92,   98),    p(  86,   94),    p(  69,   90),    p(  61,   83),
        p(  43,   96),    p(  52,   92),    p(  72,   96),    p(  83,   97),    p(  84,   96),    p(  81,   94),    p(  81,   80),    p(  64,   80),
        p(  55,  100),    p(  62,   98),    p(  65,   98),    p(  62,  104),    p(  67,  107),    p(  82,   98),    p(  90,   85),    p(  60,   87),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 175,  280),    p( 196,  312),    p( 213,  325),    p( 252,  313),    p( 281,  315),    p( 198,  310),    p( 215,  310),    p( 202,  263),
        p( 266,  313),    p( 284,  318),    p( 299,  310),    p( 304,  315),    p( 304,  310),    p( 314,  299),    p( 275,  316),    p( 271,  306),
        p( 287,  309),    p( 308,  304),    p( 309,  311),    p( 322,  315),    p( 339,  309),    p( 355,  297),    p( 291,  305),    p( 287,  308),
        p( 302,  315),    p( 310,  309),    p( 326,  313),    p( 328,  321),    p( 325,  318),    p( 321,  317),    p( 311,  312),    p( 320,  310),
        p( 299,  317),    p( 305,  306),    p( 313,  312),    p( 320,  315),    p( 320,  318),    p( 325,  303),    p( 323,  303),    p( 312,  313),
        p( 275,  303),    p( 283,  302),    p( 297,  297),    p( 301,  309),    p( 305,  307),    p( 296,  291),    p( 302,  294),    p( 294,  307),
        p( 269,  310),    p( 280,  314),    p( 285,  304),    p( 294,  309),    p( 299,  304),    p( 289,  303),    p( 295,  307),    p( 289,  323),
        p( 240,  307),    p( 281,  304),    p( 267,  307),    p( 287,  311),    p( 295,  309),    p( 293,  299),    p( 289,  308),    p( 267,  309),
    ],
    // bishop
    [
        p( 277,  311),    p( 252,  315),    p( 238,  308),    p( 221,  319),    p( 214,  317),    p( 225,  309),    p( 271,  305),    p( 249,  311),
        p( 283,  303),    p( 280,  304),    p( 290,  308),    p( 277,  306),    p( 287,  303),    p( 290,  302),    p( 267,  308),    p( 266,  305),
        p( 296,  308),    p( 309,  305),    p( 292,  305),    p( 307,  300),    p( 306,  302),    p( 336,  305),    p( 314,  303),    p( 317,  313),
        p( 286,  312),    p( 292,  307),    p( 304,  302),    p( 307,  306),    p( 307,  304),    p( 299,  305),    p( 298,  309),    p( 278,  311),
        p( 289,  306),    p( 284,  308),    p( 295,  303),    p( 309,  303),    p( 302,  300),    p( 300,  302),    p( 286,  305),    p( 309,  300),
        p( 296,  307),    p( 300,  303),    p( 300,  305),    p( 298,  304),    p( 306,  306),    p( 298,  299),    p( 306,  295),    p( 308,  297),
        p( 308,  306),    p( 304,  300),    p( 307,  301),    p( 299,  309),    p( 301,  306),    p( 302,  305),    p( 311,  297),    p( 308,  296),
        p( 298,  304),    p( 308,  306),    p( 308,  307),    p( 290,  311),    p( 306,  309),    p( 293,  311),    p( 302,  300),    p( 302,  293),
    ],
    // rook
    [
        p( 459,  547),    p( 447,  557),    p( 439,  564),    p( 437,  561),    p( 449,  557),    p( 469,  552),    p( 480,  551),    p( 491,  544),
        p( 443,  553),    p( 441,  559),    p( 450,  559),    p( 464,  550),    p( 449,  554),    p( 465,  548),    p( 474,  545),    p( 491,  535),
        p( 444,  549),    p( 464,  543),    p( 458,  545),    p( 457,  541),    p( 483,  530),    p( 493,  527),    p( 510,  526),    p( 486,  529),
        p( 441,  549),    p( 448,  544),    p( 447,  547),    p( 452,  541),    p( 455,  533),    p( 468,  528),    p( 469,  532),    p( 467,  529),
        p( 434,  546),    p( 434,  544),    p( 434,  544),    p( 439,  541),    p( 447,  536),    p( 440,  537),    p( 454,  530),    p( 448,  529),
        p( 429,  544),    p( 430,  541),    p( 431,  539),    p( 434,  538),    p( 439,  533),    p( 451,  524),    p( 467,  515),    p( 454,  518),
        p( 432,  539),    p( 436,  538),    p( 442,  538),    p( 444,  535),    p( 451,  528),    p( 463,  520),    p( 472,  516),    p( 441,  526),
        p( 441,  544),    p( 438,  539),    p( 439,  542),    p( 444,  536),    p( 449,  529),    p( 456,  531),    p( 452,  531),    p( 447,  534),
    ],
    // queen
    [
        p( 876,  964),    p( 876,  979),    p( 892,  991),    p( 913,  985),    p( 910,  990),    p( 932,  977),    p( 976,  931),    p( 924,  962),
        p( 888,  951),    p( 863,  982),    p( 863, 1009),    p( 857, 1026),    p( 862, 1039),    p( 901, 1000),    p( 901,  987),    p( 946,  964),
        p( 893,  956),    p( 886,  972),    p( 884,  994),    p( 883, 1006),    p( 906, 1007),    p( 944,  991),    p( 950,  964),    p( 941,  970),
        p( 880,  969),    p( 884,  977),    p( 878,  987),    p( 878,  999),    p( 880, 1012),    p( 894, 1003),    p( 904, 1004),    p( 911,  982),
        p( 889,  961),    p( 876,  980),    p( 882,  979),    p( 881,  996),    p( 886,  994),    p( 887,  994),    p( 900,  985),    p( 907,  977),
        p( 885,  950),    p( 891,  965),    p( 885,  979),    p( 882,  981),    p( 887,  989),    p( 894,  979),    p( 909,  962),    p( 906,  952),
        p( 885,  954),    p( 886,  960),    p( 892,  962),    p( 892,  975),    p( 893,  974),    p( 894,  959),    p( 906,  938),    p( 915,  914),
        p( 871,  955),    p( 882,  943),    p( 884,  954),    p( 892,  957),    p( 895,  945),    p( 882,  950),    p( 885,  944),    p( 888,  927),
    ],
    // king
    [
        p( 139,  -74),    p(  54,  -21),    p(  71,  -11),    p(   3,   16),    p(  32,    4),    p(  15,   17),    p(  75,    6),    p( 238,  -76),
        p( -49,   10),    p( -69,   32),    p( -70,   38),    p(  -5,   28),    p( -39,   38),    p( -59,   51),    p( -34,   40),    p(  26,   13),
        p( -66,   15),    p( -48,   23),    p( -86,   35),    p( -94,   45),    p( -59,   40),    p( -27,   33),    p( -66,   36),    p( -16,   17),
        p( -42,    0),    p(-102,   17),    p(-116,   29),    p(-138,   38),    p(-136,   38),    p(-114,   31),    p(-128,   24),    p( -85,   17),
        p( -49,  -11),    p(-108,    5),    p(-123,   21),    p(-146,   33),    p(-143,   32),    p(-120,   20),    p(-133,   13),    p( -93,    7),
        p( -28,  -10),    p( -76,   -1),    p(-107,   12),    p(-116,   21),    p(-114,   21),    p(-120,   15),    p( -94,    4),    p( -44,    0),
        p(  39,  -22),    p( -62,   -5),    p( -74,    2),    p( -96,   11),    p(-101,   13),    p( -89,    6),    p( -61,   -7),    p(  21,  -15),
        p(  70,  -37),    p(  41,  -31),    p(  37,  -16),    p( -26,   -1),    p(  22,  -16),    p( -21,   -3),    p(  32,  -24),    p(  83,  -43),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 52);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(10, 20), p(10, 18), p(10, 7), p(6, -0), p(2, -8), p(-1, -17), p(-6, -26), p(-13, -39), p(-24, -47)];
const ROOK_OPEN_FILE: PhasedScore = p(14, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -0);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 0);
const KING_OPEN_FILE: PhasedScore = p(-56, -0);
const KING_CLOSED_FILE: PhasedScore = p(13, -9);
const KING_SEMIOPEN_FILE: PhasedScore = p(-14, 7);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-5, 4), p(0, 7), p(-1, 5), p(3, 3), p(2, 5), p(4, 7), p(7, 4), p(19, 2)],
    // Closed
    [p(0, 0), p(0, 0), p(20, -33), p(-15, 10), p(-1, 11), p(-0, 4), p(-1, 6), p(-1, 4)],
    // SemiOpen
    [p(0, 0), p(-15, 24), p(3, 17), p(2, 10), p(-0, 10), p(4, 6), p(1, 2), p(10, 6)],
    // SemiClosed
    [p(0, 0), p(12, -13), p(7, 6), p(2, -0), p(6, 1), p(0, 4), p(3, 4), p(1, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(23, 5),
    p(3, 4),
    p(1, 3),
    p(-9, 12),
    p(6, 1),
    p(-10, -8),
    p(-0, 3),
    p(-6, -4),
    p(0, -2),
    p(-12, 3),
    p(-9, -15),
    p(-19, 4),
    p(3, -4),
    p(-5, -1),
    p(5, -2),
    p(-0, 20),
    p(-2, -3),
    p(-21, 0),
    p(-13, -3),
    p(-45, 24),
    p(-17, 5),
    p(-17, -14),
    p(9, 22),
    p(-54, 31),
    p(-16, -14),
    p(-20, -6),
    p(-35, -31),
    p(-38, 15),
    p(-23, 4),
    p(7, 3),
    p(-98, 115),
    p(0, 0),
    p(0, -2),
    p(-16, 1),
    p(-4, -1),
    p(-29, 12),
    p(-24, -7),
    p(-51, -20),
    p(-32, 40),
    p(-44, 36),
    p(-8, 0),
    p(-23, 2),
    p(6, -4),
    p(-23, 49),
    p(-55, 16),
    p(-13, -28),
    p(0, 0),
    p(0, 0),
    p(8, -8),
    p(-8, 21),
    p(-1, -48),
    p(0, 0),
    p(3, -5),
    p(-44, -2),
    p(0, 0),
    p(0, 0),
    p(-26, 8),
    p(-18, 6),
    p(-10, 19),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(20, 1),
    p(-1, -3),
    p(-6, 2),
    p(-17, 2),
    p(3, -3),
    p(-30, -4),
    p(-15, 3),
    p(-29, -6),
    p(5, -3),
    p(-11, -7),
    p(-27, -5),
    p(-35, 5),
    p(-5, 0),
    p(-37, 3),
    p(-29, -12),
    p(-37, 61),
    p(9, -3),
    p(-3, -5),
    p(-5, -11),
    p(-20, -4),
    p(-12, 3),
    p(-20, -4),
    p(-17, -3),
    p(-73, 179),
    p(-5, -9),
    p(-25, -9),
    p(-35, -26),
    p(17, -88),
    p(-12, -2),
    p(-12, -9),
    p(-62, 59),
    p(0, 0),
    p(14, -2),
    p(1, -2),
    p(-12, -4),
    p(-17, -4),
    p(-1, 1),
    p(-26, -11),
    p(-8, 2),
    p(-18, 5),
    p(1, -6),
    p(-18, -5),
    p(-22, -13),
    p(-28, -4),
    p(-3, -2),
    p(-35, -13),
    p(3, 22),
    p(-46, 58),
    p(6, 0),
    p(-9, 2),
    p(-23, 58),
    p(0, 0),
    p(-15, 3),
    p(-20, 9),
    p(0, 0),
    p(0, 0),
    p(-12, 5),
    p(-35, 17),
    p(-29, -44),
    p(0, 0),
    p(15, -53),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(0, 0), /*0b0000*/
    p(0, 0), /*0b0001*/
    p(0, 0), /*0b0010*/
    p(0, 0), /*0b0011*/
    p(0, 0), /*0b0100*/
    p(0, 0), /*0b0101*/
    p(0, 0), /*0b0110*/
    p(0, 0), /*0b0111*/
    p(0, 0), /*0b1000*/
    p(0, 0), /*0b1001*/
    p(0, 0), /*0b1010*/
    p(0, 0), /*0b1011*/
    p(0, 0), /*0b1100*/
    p(0, 0), /*0b1101*/
    p(0, 0), /*0b1110*/
    p(0, 0), /*0b1111*/
    p(0, 0), /*0b10000*/
    p(0, 0), /*0b10001*/
    p(0, 0), /*0b10010*/
    p(0, 0), /*0b10011*/
    p(0, 0), /*0b10100*/
    p(0, 0), /*0b10101*/
    p(0, 0), /*0b10110*/
    p(0, 0), /*0b10111*/
    p(0, 0), /*0b11000*/
    p(0, 0), /*0b11001*/
    p(0, 0), /*0b11010*/
    p(0, 0), /*0b11011*/
    p(0, 0), /*0b11100*/
    p(0, 0), /*0b11101*/
    p(0, 0), /*0b11110*/
    p(0, 0), /*0b11111*/
    p(0, 0), /*0b100000*/
    p(0, 0), /*0b100001*/
    p(0, 0), /*0b100010*/
    p(0, 0), /*0b100011*/
    p(0, 0), /*0b100100*/
    p(0, 0), /*0b100101*/
    p(0, 0), /*0b100110*/
    p(0, 0), /*0b100111*/
    p(0, 0), /*0b101000*/
    p(0, 0), /*0b101001*/
    p(0, 0), /*0b101010*/
    p(0, 0), /*0b101011*/
    p(0, 0), /*0b101100*/
    p(0, 0), /*0b101101*/
    p(0, 0), /*0b101110*/
    p(0, 0), /*0b101111*/
    p(0, 0), /*0b110000*/
    p(0, 0), /*0b110001*/
    p(0, 0), /*0b110010*/
    p(0, 0), /*0b110011*/
    p(0, 0), /*0b110100*/
    p(0, 0), /*0b110101*/
    p(0, 0), /*0b110110*/
    p(0, 0), /*0b110111*/
    p(0, 0), /*0b111000*/
    p(0, 0), /*0b111001*/
    p(0, 0), /*0b111010*/
    p(0, 0), /*0b111011*/
    p(0, 0), /*0b111100*/
    p(0, 0), /*0b111101*/
    p(0, 0), /*0b111110*/
    p(0, 0), /*0b111111*/
    p(0, 0), /*0b00*/
    p(0, 0), /*0b01*/
    p(0, 0), /*0b10*/
    p(0, 0), /*0b11*/
    p(0, 0), /*0b100*/
    p(0, 0), /*0b101*/
    p(0, 0), /*0b110*/
    p(0, 0), /*0b111*/
    p(0, 0), /*0b1000*/
    p(0, 0), /*0b1001*/
    p(0, 0), /*0b1010*/
    p(0, 0), /*0b1011*/
    p(0, 0), /*0b1100*/
    p(0, 0), /*0b1101*/
    p(0, 0), /*0b1110*/
    p(0, 0), /*0b1111*/
    p(0, 0), /*0b00*/
    p(0, 0), /*0b01*/
    p(0, 0), /*0b10*/
    p(0, 0), /*0b11*/
    p(0, 0), /*0b100*/
    p(0, 0), /*0b101*/
    p(0, 0), /*0b110*/
    p(0, 0), /*0b111*/
    p(0, 0), /*0b1000*/
    p(0, 0), /*0b1001*/
    p(0, 0), /*0b1010*/
    p(0, 0), /*0b1011*/
    p(0, 0), /*0b1100*/
    p(0, 0), /*0b1101*/
    p(0, 0), /*0b1110*/
    p(0, 0), /*0b1111*/
];
const STOPPABLE_PASSER: PhasedScore = p(38, -49);
const CLOSE_KING_PASSER: PhasedScore = p(-4, 30);
const IMMOBILE_PASSER: PhasedScore = p(-8, -36);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -23,   35),    p( -39,   50),    p( -46,   50),    p( -41,   38),    p( -33,   26),    p( -34,   33),    p( -28,   42),    p( -31,   42),
        p( -12,   36),    p( -43,   58),    p( -47,   51),    p( -42,   40),    p( -47,   41),    p( -43,   44),    p( -55,   63),    p( -26,   45),
        p(  -4,   62),    p( -15,   64),    p( -40,   66),    p( -34,   62),    p( -43,   59),    p( -33,   63),    p( -45,   78),    p( -38,   75),
        p(  14,   85),    p(   8,   87),    p(  13,   75),    p(  -9,   81),    p( -27,   81),    p( -17,   85),    p( -33,   97),    p( -33,  101),
        p(  31,  132),    p(  36,  130),    p(  28,  115),    p(  12,   94),    p(   5,  101),    p(  -6,  118),    p( -26,  121),    p( -52,  143),
        p(  36,   89),    p(  32,   88),    p(  24,   92),    p(  35,   75),    p(  22,   79),    p(  20,   83),    p( -21,   99),    p( -17,   99),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-6, -8);
const DOUBLED_PAWN: PhasedScore = p(-11, -22);
const PHALANX: [PhasedScore; 6] = [p(-2, -3), p(3, 0), p(10, 4), p(25, 22), p(65, 75), p(-99, 219)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(16, 11), p(7, 15), p(14, 20), p(9, 8), p(-3, 16), p(-43, 13)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(38, 7), p(39, 33), p(51, -12), p(35, -38), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-45, -72),
        p(-25, -32),
        p(-13, -8),
        p(-4, 5),
        p(4, 16),
        p(10, 26),
        p(18, 29),
        p(25, 32),
        p(29, 31),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-18, -37),
        p(-7, -23),
        p(-0, -10),
        p(7, -1),
        p(12, 8),
        p(16, 13),
        p(21, 17),
        p(22, 22),
        p(29, 24),
        p(34, 23),
        p(41, 26),
        p(38, 35),
        p(51, 27),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-74, 11),
        p(-65, 25),
        p(-62, 31),
        p(-59, 35),
        p(-59, 42),
        p(-54, 48),
        p(-50, 52),
        p(-46, 56),
        p(-43, 61),
        p(-40, 65),
        p(-36, 69),
        p(-35, 74),
        p(-28, 75),
        p(-20, 73),
        p(-16, 72),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-31, -35),
        p(-29, 16),
        p(-34, 64),
        p(-30, 82),
        p(-28, 99),
        p(-23, 105),
        p(-20, 115),
        p(-16, 121),
        p(-12, 125),
        p(-9, 127),
        p(-6, 129),
        p(-2, 132),
        p(1, 132),
        p(3, 137),
        p(5, 139),
        p(9, 142),
        p(9, 150),
        p(11, 151),
        p(21, 149),
        p(33, 144),
        p(37, 147),
        p(80, 125),
        p(79, 129),
        p(101, 111),
        p(192, 80),
        p(235, 40),
        p(262, 31),
        p(324, -11),
    ],
    [
        p(-86, -12),
        p(-53, -22),
        p(-28, -10),
        p(-2, -1),
        p(26, 0),
        p(46, 2),
        p(73, 7),
        p(94, 9),
        p(136, -4),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-2, 10), p(19, 23), p(0, 0), p(31, 5), p(31, 52), p(0, 0)],
    [p(-3, 13), p(10, 16), p(17, 13), p(0, 0), p(45, -4), p(0, 0)],
    [p(-2, 4), p(2, 6), p(-1, 22), p(1, 2), p(0, 0), p(0, 0)],
    [p(62, 19), p(-36, 18), p(-8, 16), p(-23, 8), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 7), p(8, 7), p(5, 10), p(12, 7), p(6, 19), p(10, 6)],
    [p(1, 8), p(11, 21), p(-133, -19), p(8, 14), p(9, 19), p(0, 8)],
    [p(3, 3), p(13, 8), p(9, 13), p(11, 10), p(10, 24), p(21, -4)],
    [p(2, -1), p(9, 2), p(7, -3), p(4, 15), p(-60, -257), p(4, -11)],
    [p(49, -2), p(37, 9), p(46, 2), p(20, 7), p(33, -9), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-18, -16), p(19, -9), p(10, -2), p(16, -11), p(-2, 13), p(-7, 4)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(27, 11), p(14, 19), p(34, 1), p(6, 32)];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static + StaticallyNamedEntity {
    type Score: ScoreType;

    fn psqt(&self, square: ChessSquare, piece: ChessPieceType, color: ChessColor) -> SingleFeatureScore<Self::Score>;

    fn passed_pawn(square: ChessSquare) -> SingleFeatureScore<Self::Score>;

    fn stoppable_passer() -> SingleFeatureScore<Self::Score>;

    fn close_king_passer() -> SingleFeatureScore<Self::Score>;

    fn immobile_passer() -> SingleFeatureScore<Self::Score>;

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
