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
        p( 136,  189),    p( 132,  188),    p( 124,  192),    p( 135,  175),    p( 121,  179),    p( 120,  182),    p(  79,  199),    p(  83,  199),
        p(  70,  122),    p(  73,  124),    p(  81,  115),    p(  89,  116),    p(  86,  116),    p( 128,  105),    p( 106,  125),    p(  97,  117),
        p(  54,  112),    p(  65,  105),    p(  63,  101),    p(  85,   99),    p(  92,   99),    p(  86,   90),    p(  79,  100),    p(  74,   93),
        p(  48,   99),    p(  54,  101),    p(  78,   93),    p(  92,   96),    p(  92,   98),    p(  86,   95),    p(  69,   91),    p(  62,   84),
        p(  43,   97),    p(  53,   93),    p(  72,   96),    p(  83,   97),    p(  85,   96),    p(  81,   94),    p(  82,   82),    p(  64,   81),
        p(  54,  100),    p(  62,   99),    p(  65,   98),    p(  62,  104),    p(  67,  107),    p(  82,   98),    p(  90,   86),    p(  60,   88),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 175,  279),    p( 196,  312),    p( 212,  325),    p( 251,  314),    p( 281,  316),    p( 198,  310),    p( 215,  310),    p( 202,  263),
        p( 266,  313),    p( 284,  318),    p( 300,  310),    p( 304,  315),    p( 304,  310),    p( 315,  299),    p( 275,  316),    p( 271,  306),
        p( 287,  309),    p( 308,  304),    p( 310,  311),    p( 322,  315),    p( 339,  309),    p( 355,  297),    p( 291,  305),    p( 287,  308),
        p( 302,  315),    p( 310,  309),    p( 326,  313),    p( 328,  321),    p( 325,  318),    p( 321,  317),    p( 311,  313),    p( 320,  310),
        p( 299,  317),    p( 305,  306),    p( 313,  312),    p( 321,  315),    p( 320,  318),    p( 325,  303),    p( 323,  304),    p( 312,  313),
        p( 275,  303),    p( 283,  302),    p( 297,  297),    p( 302,  309),    p( 306,  307),    p( 296,  291),    p( 302,  294),    p( 294,  307),
        p( 270,  310),    p( 280,  314),    p( 285,  304),    p( 294,  309),    p( 299,  304),    p( 289,  303),    p( 296,  307),    p( 289,  323),
        p( 240,  307),    p( 281,  304),    p( 267,  307),    p( 287,  311),    p( 295,  309),    p( 293,  299),    p( 289,  308),    p( 267,  309),
    ],
    // bishop
    [
        p( 277,  311),    p( 252,  315),    p( 238,  308),    p( 221,  319),    p( 214,  317),    p( 225,  309),    p( 271,  305),    p( 249,  311),
        p( 283,  303),    p( 280,  304),    p( 290,  308),    p( 277,  306),    p( 287,  303),    p( 290,  302),    p( 266,  309),    p( 266,  305),
        p( 296,  308),    p( 309,  305),    p( 293,  305),    p( 307,  300),    p( 306,  302),    p( 336,  305),    p( 314,  303),    p( 317,  313),
        p( 286,  312),    p( 292,  307),    p( 305,  302),    p( 307,  306),    p( 307,  304),    p( 299,  305),    p( 298,  309),    p( 278,  311),
        p( 290,  306),    p( 285,  308),    p( 295,  304),    p( 309,  303),    p( 302,  300),    p( 300,  302),    p( 286,  305),    p( 309,  300),
        p( 296,  307),    p( 300,  303),    p( 300,  305),    p( 298,  304),    p( 306,  306),    p( 298,  299),    p( 306,  295),    p( 308,  297),
        p( 308,  306),    p( 304,  300),    p( 307,  301),    p( 299,  309),    p( 301,  306),    p( 302,  305),    p( 311,  297),    p( 308,  296),
        p( 298,  304),    p( 308,  306),    p( 308,  307),    p( 290,  311),    p( 306,  309),    p( 293,  311),    p( 302,  300),    p( 302,  293),
    ],
    // rook
    [
        p( 460,  547),    p( 447,  557),    p( 439,  564),    p( 438,  561),    p( 449,  557),    p( 469,  553),    p( 480,  551),    p( 492,  544),
        p( 443,  553),    p( 441,  559),    p( 450,  559),    p( 464,  550),    p( 449,  554),    p( 465,  548),    p( 474,  545),    p( 491,  535),
        p( 444,  549),    p( 465,  543),    p( 458,  545),    p( 457,  541),    p( 484,  530),    p( 493,  527),    p( 510,  526),    p( 486,  529),
        p( 442,  549),    p( 448,  544),    p( 447,  547),    p( 452,  541),    p( 456,  533),    p( 468,  529),    p( 469,  532),    p( 468,  529),
        p( 434,  546),    p( 434,  544),    p( 435,  544),    p( 439,  541),    p( 447,  536),    p( 440,  537),    p( 454,  530),    p( 448,  529),
        p( 430,  544),    p( 430,  541),    p( 432,  539),    p( 434,  538),    p( 439,  533),    p( 451,  524),    p( 467,  515),    p( 455,  518),
        p( 432,  539),    p( 437,  538),    p( 442,  538),    p( 445,  535),    p( 451,  529),    p( 463,  520),    p( 472,  517),    p( 442,  526),
        p( 442,  544),    p( 438,  539),    p( 439,  543),    p( 444,  536),    p( 449,  529),    p( 456,  531),    p( 452,  531),    p( 447,  534),
    ],
    // queen
    [
        p( 877,  964),    p( 877,  979),    p( 892,  991),    p( 913,  985),    p( 911,  990),    p( 932,  977),    p( 977,  931),    p( 924,  962),
        p( 888,  951),    p( 863,  982),    p( 863, 1010),    p( 857, 1026),    p( 863, 1039),    p( 901, 1000),    p( 901,  987),    p( 946,  964),
        p( 893,  956),    p( 886,  972),    p( 885,  994),    p( 883, 1006),    p( 906, 1007),    p( 944,  992),    p( 951,  964),    p( 941,  970),
        p( 880,  969),    p( 884,  978),    p( 878,  988),    p( 878, 1000),    p( 880, 1012),    p( 895, 1003),    p( 904, 1004),    p( 911,  982),
        p( 889,  961),    p( 876,  980),    p( 882,  980),    p( 882,  996),    p( 886,  994),    p( 887,  994),    p( 900,  985),    p( 908,  977),
        p( 885,  949),    p( 891,  965),    p( 885,  979),    p( 882,  980),    p( 887,  989),    p( 894,  979),    p( 909,  962),    p( 906,  952),
        p( 885,  953),    p( 886,  960),    p( 892,  962),    p( 892,  975),    p( 894,  974),    p( 895,  958),    p( 907,  938),    p( 915,  913),
        p( 871,  955),    p( 883,  943),    p( 884,  954),    p( 892,  957),    p( 895,  945),    p( 882,  950),    p( 886,  943),    p( 888,  927),
    ],
    // king
    [
        p( 138,  -74),    p(  54,  -21),    p(  72,  -12),    p(   3,   16),    p(  32,    4),    p(  15,   16),    p(  74,    6),    p( 238,  -77),
        p( -49,   10),    p( -70,   32),    p( -71,   39),    p(  -5,   28),    p( -38,   38),    p( -58,   51),    p( -34,   40),    p(  27,   13),
        p( -65,   15),    p( -48,   24),    p( -85,   35),    p( -94,   45),    p( -58,   41),    p( -26,   33),    p( -65,   36),    p( -15,   16),
        p( -41,    0),    p(-100,   17),    p(-116,   30),    p(-137,   39),    p(-135,   38),    p(-113,   32),    p(-127,   24),    p( -84,   16),
        p( -48,  -11),    p(-107,    6),    p(-123,   21),    p(-146,   33),    p(-142,   32),    p(-120,   20),    p(-133,   13),    p( -93,    7),
        p( -27,  -11),    p( -76,   -1),    p(-106,   12),    p(-115,   21),    p(-113,   21),    p(-120,   15),    p( -94,    4),    p( -44,    0),
        p(  39,  -23),    p( -62,   -4),    p( -74,    2),    p( -96,   11),    p(-101,   14),    p( -89,    6),    p( -60,   -7),    p(  20,  -15),
        p(  70,  -37),    p(  40,  -31),    p(  37,  -17),    p( -26,   -1),    p(  22,  -16),    p( -22,   -3),    p(  32,  -24),    p(  83,  -44),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 52);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(10, 20), p(10, 18), p(10, 7), p(6, -0), p(2, -8), p(-1, -17), p(-6, -26), p(-13, -39), p(-24, -48)];
const ROOK_OPEN_FILE: PhasedScore = p(13, 4);
const ROOK_CLOSED_FILE: PhasedScore = p(-12, -1);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 0);
const KING_OPEN_FILE: PhasedScore = p(-56, -0);
const KING_CLOSED_FILE: PhasedScore = p(13, -9);
const KING_SEMIOPEN_FILE: PhasedScore = p(-14, 6);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-5, 4), p(0, 7), p(-1, 6), p(3, 3), p(2, 5), p(4, 7), p(7, 4), p(18, 2)],
    // Closed
    [p(0, 0), p(0, 0), p(20, -33), p(-15, 10), p(-0, 11), p(-0, 4), p(-1, 6), p(-1, 4)],
    // SemiOpen
    [p(0, 0), p(-14, 24), p(3, 17), p(2, 10), p(-0, 10), p(4, 6), p(1, 2), p(10, 6)],
    // SemiClosed
    [p(0, 0), p(12, -13), p(7, 6), p(2, -0), p(6, 1), p(1, 4), p(3, 4), p(1, 4)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(22, 5),
    p(3, 5),
    p(1, 4),
    p(-8, 14),
    p(6, 1),
    p(-9, -7),
    p(-0, 3),
    p(-6, -4),
    p(0, -2),
    p(-12, 4),
    p(-8, -15),
    p(-18, 5),
    p(4, -4),
    p(-4, 0),
    p(5, -2),
    p(0, 23),
    p(-1, -2),
    p(-22, 0),
    p(-13, -3),
    p(-46, 25),
    p(-18, 4),
    p(-17, -15),
    p(9, 22),
    p(-54, 31),
    p(-15, -13),
    p(-20, -6),
    p(-34, -31),
    p(-37, 17),
    p(-23, 3),
    p(7, 4),
    p(-98, 115),
    p(0, 0),
    p(0, -2),
    p(-15, 2),
    p(-4, -1),
    p(-28, 13),
    p(-23, -6),
    p(-50, -18),
    p(-32, 40),
    p(-43, 37),
    p(-8, -0),
    p(-23, 3),
    p(5, -5),
    p(-22, 50),
    p(-55, 18),
    p(-9, -26),
    p(0, 0),
    p(0, 0),
    p(10, -6),
    p(-8, 21),
    p(-1, -49),
    p(0, 0),
    p(2, -6),
    p(-44, -2),
    p(0, 0),
    p(0, 0),
    p(-25, 8),
    p(-18, 5),
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
    p(-29, -3),
    p(-14, 5),
    p(-28, -5),
    p(5, -3),
    p(-11, -7),
    p(-26, -5),
    p(-34, 6),
    p(-5, 1),
    p(-37, 4),
    p(-29, -11),
    p(-36, 62),
    p(9, -3),
    p(-3, -6),
    p(-6, -11),
    p(-19, -3),
    p(-12, 2),
    p(-20, -5),
    p(-17, -2),
    p(-73, 182),
    p(-5, -10),
    p(-25, -9),
    p(-35, -26),
    p(18, -86),
    p(-12, -2),
    p(-12, -8),
    p(-63, 63),
    p(0, 0),
    p(14, -2),
    p(1, -2),
    p(-12, -5),
    p(-16, -3),
    p(-1, 1),
    p(-26, -12),
    p(-8, 3),
    p(-18, 7),
    p(2, -7),
    p(-18, -4),
    p(-23, -14),
    p(-27, -4),
    p(-3, -0),
    p(-35, -11),
    p(3, 23),
    p(-46, 60),
    p(6, 1),
    p(-9, 2),
    p(-23, 59),
    p(0, 0),
    p(-15, 3),
    p(-20, 9),
    p(0, 0),
    p(0, 0),
    p(-12, 5),
    p(-35, 17),
    p(-30, -42),
    p(0, 0),
    p(17, -52),
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
const STOPPABLE_PASSER: PhasedScore = p(39, -49);
const CLOSE_KING_PASSER: PhasedScore = p(-5, 30);
const IMMOBILE_PASSER: PhasedScore = p(-8, -36);
const PROTECTED_PASSER: PhasedScore = p(15, 5);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -24,   34),    p( -41,   49),    p( -46,   49),    p( -41,   38),    p( -33,   25),    p( -34,   32),    p( -28,   40),    p( -31,   41),
        p( -17,   34),    p( -52,   56),    p( -53,   50),    p( -47,   40),    p( -54,   40),    p( -49,   42),    p( -65,   60),    p( -33,   43),
        p(  -7,   61),    p( -21,   62),    p( -46,   65),    p( -42,   61),    p( -49,   58),    p( -40,   62),    p( -51,   76),    p( -42,   74),
        p(  12,   83),    p(   3,   86),    p(   6,   74),    p( -16,   79),    p( -34,   80),    p( -23,   84),    p( -37,   95),    p( -35,   99),
        p(  30,  130),    p(  33,  128),    p(  25,  114),    p(  10,   93),    p(   2,  100),    p(  -9,  117),    p( -29,  119),    p( -53,  142),
        p(  36,   89),    p(  32,   88),    p(  24,   92),    p(  35,   75),    p(  21,   79),    p(  20,   82),    p( -21,   99),    p( -17,   99),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const UNSUPPORTED_PAWN: PhasedScore = p(-6, -8);
const DOUBLED_PAWN: PhasedScore = p(-11, -23);
const PHALANX: [PhasedScore; 6] = [p(-2, -3), p(3, 0), p(11, 4), p(25, 22), p(66, 76), p(-99, 220)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(16, 10), p(7, 15), p(14, 20), p(9, 8), p(-3, 16), p(-43, 12)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(38, 7), p(39, 33), p(51, -12), p(35, -38), p(0, 0)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-45, -71),
        p(-25, -31),
        p(-12, -8),
        p(-4, 5),
        p(4, 16),
        p(11, 26),
        p(18, 29),
        p(25, 32),
        p(30, 31),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(7, -0),
        p(12, 8),
        p(16, 13),
        p(21, 17),
        p(23, 22),
        p(29, 24),
        p(34, 23),
        p(41, 26),
        p(39, 34),
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
        p(-59, 36),
        p(-59, 43),
        p(-54, 49),
        p(-50, 53),
        p(-46, 57),
        p(-43, 61),
        p(-40, 66),
        p(-36, 69),
        p(-35, 74),
        p(-28, 76),
        p(-19, 74),
        p(-16, 73),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-31, -32),
        p(-29, 16),
        p(-33, 65),
        p(-29, 83),
        p(-27, 100),
        p(-23, 106),
        p(-19, 116),
        p(-16, 122),
        p(-12, 126),
        p(-8, 128),
        p(-5, 130),
        p(-1, 133),
        p(2, 133),
        p(3, 138),
        p(6, 140),
        p(9, 143),
        p(10, 151),
        p(12, 152),
        p(21, 150),
        p(34, 145),
        p(38, 147),
        p(80, 126),
        p(80, 129),
        p(102, 113),
        p(193, 81),
        p(235, 42),
        p(260, 33),
        p(322, -9),
    ],
    [
        p(-85, -11),
        p(-53, -21),
        p(-28, -10),
        p(-2, -1),
        p(26, 0),
        p(46, 2),
        p(73, 7),
        p(94, 8),
        p(136, -5),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-3, 13), p(10, 16), p(17, 13), p(0, 0), p(45, -5), p(0, 0)],
    [p(-2, 4), p(2, 6), p(-1, 21), p(1, 2), p(0, 0), p(0, 0)],
    [p(62, 19), p(-36, 18), p(-8, 16), p(-23, 8), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(1, 7), p(8, 7), p(5, 10), p(12, 7), p(6, 19), p(10, 6)],
    [p(1, 8), p(11, 21), p(-131, -19), p(8, 14), p(9, 19), p(0, 8)],
    [p(3, 3), p(13, 8), p(9, 13), p(11, 10), p(10, 24), p(21, -4)],
    [p(2, -1), p(8, 2), p(7, -3), p(4, 15), p(-59, -258), p(4, -10)],
    [p(49, -3), p(37, 9), p(46, 2), p(20, 7), p(33, -9), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-18, -16), p(19, -9), p(10, -2), p(15, -11), p(-2, 13), p(-6, 4)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(28, 11), p(14, 19), p(34, 0), p(6, 32)];

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static + StaticallyNamedEntity {
    type Score: ScoreType;

    fn psqt(&self, square: ChessSquare, piece: ChessPieceType, color: ChessColor) -> SingleFeatureScore<Self::Score>;

    fn passed_pawn(square: ChessSquare) -> SingleFeatureScore<Self::Score>;

    fn stoppable_passer() -> SingleFeatureScore<Self::Score>;

    fn close_king_passer() -> SingleFeatureScore<Self::Score>;

    fn immobile_passer() -> SingleFeatureScore<Self::Score>;

    fn protected_passer() -> SingleFeatureScore<Self::Score>;

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

    fn protected_passer() -> SingleFeatureScore<Self::Score> {
        PROTECTED_PASSER
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
