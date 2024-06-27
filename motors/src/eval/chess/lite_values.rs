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

use crate::eval::chess::{FileOpenness, NUM_PAWN_SHIELD_CONFIGURATIONS};
use crate::eval::ScoreType;
use gears::games::chess::pieces::{UncoloredChessPiece, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::Color;
use gears::games::Color::*;
use gears::score::{p, PhasedScore};
use std::fmt::Debug;

#[rustfmt::skip]
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
        p( 132,  177),    p( 132,  176),    p( 130,  175),    p( 143,  152),    p( 131,  155),    p( 122,  160),    p(  70,  183),    p(  68,  185),
        p(  73,  114),    p(  82,  114),    p(  96,   99),    p( 103,  104),    p( 109,   93),    p( 146,   86),    p( 135,  111),    p( 102,  105),
        p(  59,  103),    p(  77,   98),    p(  72,   89),    p(  77,   81),    p(  98,   81),    p(  92,   80),    p(  90,   92),    p(  77,   85),
        p(  48,   94),    p(  68,   93),    p(  68,   83),    p(  80,   81),    p(  83,   81),    p(  84,   78),    p(  81,   85),    p(  59,   78),
        p(  43,   88),    p(  56,   86),    p(  58,   81),    p(  59,   90),    p(  69,   87),    p(  63,   82),    p(  74,   78),    p(  53,   76),
        p(  51,   95),    p(  66,   95),    p(  59,   90),    p(  52,  100),    p(  64,  102),    p(  76,   94),    p(  90,   85),    p(  53,   83),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 143,  250),    p( 184,  300),    p( 229,  318),    p( 257,  307),    p( 302,  306),    p( 211,  300),    p( 221,  297),    p( 182,  229),
        p( 267,  296),    p( 286,  312),    p( 324,  310),    p( 341,  311),    p( 319,  309),    p( 377,  291),    p( 290,  306),    p( 304,  282),
        p( 286,  303),    p( 327,  310),    p( 353,  325),    p( 359,  327),    p( 393,  314),    p( 400,  309),    p( 354,  301),    p( 314,  296),
        p( 290,  312),    p( 311,  323),    p( 335,  336),    p( 361,  335),    p( 340,  338),    p( 361,  333),    p( 320,  323),    p( 325,  303),
        p( 279,  311),    p( 296,  319),    p( 320,  336),    p( 321,  339),    p( 332,  340),    p( 323,  327),    p( 320,  314),    p( 291,  302),
        p( 260,  296),    p( 285,  311),    p( 303,  319),    p( 314,  331),    p( 324,  326),    p( 310,  311),    p( 306,  300),    p( 277,  296),
        p( 253,  293),    p( 265,  308),    p( 282,  312),    p( 296,  314),    p( 296,  312),    p( 295,  306),    p( 281,  298),    p( 279,  297),
        p( 222,  280),    p( 269,  283),    p( 256,  301),    p( 273,  304),    p( 282,  302),    p( 282,  292),    p( 272,  287),    p( 241,  275),
    ],
    // bishop
    [
        p( 268,  320),    p( 244,  326),    p( 241,  321),    p( 216,  329),    p( 233,  326),    p( 226,  322),    p( 282,  317),    p( 253,  317),
        p( 282,  312),    p( 304,  312),    p( 298,  314),    p( 280,  318),    p( 307,  309),    p( 310,  310),    p( 303,  314),    p( 284,  310),
        p( 298,  321),    p( 319,  313),    p( 317,  316),    p( 331,  307),    p( 331,  311),    p( 360,  314),    p( 342,  311),    p( 329,  319),
        p( 292,  318),    p( 308,  320),    p( 320,  315),    p( 338,  322),    p( 330,  319),    p( 326,  318),    p( 307,  320),    p( 299,  318),
        p( 294,  315),    p( 299,  319),    p( 306,  323),    p( 328,  320),    p( 323,  319),    p( 306,  318),    p( 301,  317),    p( 304,  310),
        p( 295,  316),    p( 309,  317),    p( 310,  319),    p( 312,  318),    p( 312,  322),    p( 312,  315),    p( 310,  310),    p( 309,  308),
        p( 302,  318),    p( 311,  310),    p( 317,  305),    p( 297,  318),    p( 309,  315),    p( 311,  311),    p( 323,  311),    p( 302,  304),
        p( 294,  308),    p( 312,  313),    p( 298,  311),    p( 288,  317),    p( 296,  317),    p( 289,  322),    p( 303,  302),    p( 296,  298),
    ],
    // rook
    [
        p( 469,  549),    p( 455,  559),    p( 458,  562),    p( 463,  557),    p( 473,  554),    p( 490,  550),    p( 498,  549),    p( 510,  542),
        p( 447,  559),    p( 447,  564),    p( 466,  563),    p( 483,  554),    p( 472,  556),    p( 506,  543),    p( 504,  542),    p( 515,  533),
        p( 433,  557),    p( 455,  553),    p( 453,  553),    p( 455,  548),    p( 485,  538),    p( 499,  532),    p( 534,  526),    p( 495,  531),
        p( 428,  556),    p( 438,  552),    p( 438,  554),    p( 449,  547),    p( 452,  539),    p( 468,  535),    p( 475,  535),    p( 470,  531),
        p( 424,  549),    p( 426,  548),    p( 428,  548),    p( 439,  543),    p( 442,  540),    p( 441,  538),    p( 459,  531),    p( 448,  529),
        p( 426,  542),    p( 425,  541),    p( 430,  540),    p( 438,  539),    p( 446,  532),    p( 452,  525),    p( 478,  510),    p( 457,  515),
        p( 428,  537),    p( 434,  537),    p( 441,  538),    p( 443,  536),    p( 452,  529),    p( 457,  522),    p( 471,  514),    p( 438,  523),
        p( 450,  540),    p( 445,  536),    p( 447,  542),    p( 455,  536),    p( 462,  529),    p( 460,  533),    p( 460,  524),    p( 455,  526),
    ],
    // queen
    [
        p( 855,  993),    p( 856, 1006),    p( 871, 1019),    p( 897, 1009),    p( 897, 1015),    p( 917, 1004),    p( 963,  951),    p( 907,  988),
        p( 874,  979),    p( 857, 1003),    p( 862, 1028),    p( 853, 1046),    p( 861, 1059),    p( 905, 1021),    p( 903,  999),    p( 937,  987),
        p( 880,  973),    p( 875,  987),    p( 878, 1007),    p( 882, 1019),    p( 902, 1027),    p( 940, 1011),    p( 946,  983),    p( 932,  991),
        p( 869,  981),    p( 876,  988),    p( 878, 1000),    p( 879, 1014),    p( 882, 1027),    p( 891, 1021),    p( 895, 1018),    p( 903, 1002),
        p( 876,  974),    p( 873,  992),    p( 880,  988),    p( 884, 1006),    p( 886,  999),    p( 888,  998),    p( 895,  993),    p( 896,  993),
        p( 877,  962),    p( 887,  969),    p( 888,  980),    p( 888,  975),    p( 894,  977),    p( 897,  975),    p( 908,  963),    p( 899,  961),
        p( 879,  962),    p( 886,  960),    p( 894,  956),    p( 894,  963),    p( 894,  965),    p( 899,  941),    p( 905,  922),    p( 906,  909),
        p( 879,  953),    p( 878,  952),    p( 883,  957),    p( 891,  964),    p( 891,  949),    p( 873,  950),    p( 878,  934),    p( 878,  934),
    ],
    // king
    [
        p(  74,  -86),    p(  42,  -43),    p(  64,  -35),    p( -22,   -3),    p(   6,  -15),    p(  14,   -9),    p(  57,  -15),    p( 149,  -87),
        p( -22,   -4),    p(  40,   14),    p(  22,   22),    p(  90,   10),    p(  64,   23),    p(  43,   36),    p(  70,   28),    p(  24,    2),
        p( -37,   10),    p(  69,   18),    p(  16,   35),    p(   4,   45),    p(  41,   41),    p(  89,   39),    p(  64,   36),    p(  -6,   19),
        p( -23,    5),    p(  -2,   23),    p( -25,   42),    p( -47,   53),    p( -48,   54),    p( -24,   45),    p( -24,   36),    p( -84,   24),
        p( -49,    3),    p( -22,   15),    p( -43,   36),    p( -68,   52),    p( -72,   50),    p( -51,   35),    p( -55,   23),    p(-110,   23),
        p( -35,    4),    p(   9,    4),    p( -36,   24),    p( -47,   36),    p( -42,   33),    p( -48,   22),    p( -13,    4),    p( -64,   14),
        p(  24,   -5),    p(  24,   -7),    p(   4,    5),    p( -21,   16),    p( -22,   15),    p(  -9,    5),    p(  31,  -13),    p(   9,   -3),
        p( -31,  -22),    p(  26,  -44),    p(  14,  -29),    p( -53,   -8),    p(   2,  -30),    p( -49,  -12),    p(  15,  -38),    p(   2,  -40),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
    p(  32,   77),    p(  32,   76),    p(  30,   75),    p(  43,   52),    p(  31,   55),    p(  22,   60),    p( -30,   83),    p( -32,   85),
    p(  28,  114),    p(  37,  112),    p(  31,   93),    p(  17,   59),    p(  19,   65),    p(   4,   87),    p( -28,   96),    p( -54,  119),
    p(  12,   66),    p(  13,   62),    p(  20,   48),    p(  15,   38),    p(  -2,   38),    p(  10,   46),    p( -11,   63),    p( -16,   68),
    p(   0,   39),    p(  -9,   36),    p( -16,   30),    p(  -9,   21),    p( -17,   24),    p( -10,   31),    p( -21,   44),    p( -12,   43),
    p(  -7,   11),    p( -15,   18),    p( -19,   17),    p( -13,    6),    p( -14,   12),    p(  -9,   13),    p( -12,   30),    p(   5,   13),
    p( -14,    9),    p(  -7,   12),    p( -11,   17),    p( -11,    5),    p(   3,   -0),    p(   3,    5),    p(   6,   12),    p(   1,   10),
    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const BISHOP_PAIR: PhasedScore = p(23, 56);
const ROOK_OPEN_FILE: PhasedScore = p(23, 6);
const ROOK_CLOSED_FILE: PhasedScore = p(-14, 0);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(3, 9);
const KING_OPEN_FILE: PhasedScore = p(-56, -8);
const KING_CLOSED_FILE: PhasedScore = p(15, -13);
const KING_SEMIOPEN_FILE: PhasedScore = p(-14, 9);
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-36, 10),  /*0b0000*/
    p(-24, 7),   /*0b0001*/
    p(-15, 3),   /*0b0010*/
    p(5, 22),    /*0b0011*/
    p(-9, 2),    /*0b0100*/
    p(-17, -6),  /*0b0101*/
    p(2, 14),    /*0b0110*/
    p(18, -4),   /*0b0111*/
    p(-21, 9),   /*0b1000*/
    p(-25, -17), /*0b1001*/
    p(-11, 5),   /*0b1010*/
    p(10, -7),   /*0b1011*/
    p(-8, 1),    /*0b1100*/
    p(-20, -22), /*0b1101*/
    p(12, 12),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-31, 13),  /*0b10000*/
    p(-2, 4),    /*0b10001*/
    p(-14, -18), /*0b10010*/
    p(5, -9),    /*0b10011*/
    p(-10, -0),  /*0b10100*/
    p(23, 9),    /*0b10101*/
    p(-12, -17), /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(-6, 43),   /*0b11000*/
    p(19, 4),    /*0b11001*/
    p(24, 17),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(17, 12),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(-8, 6),    /*0b100000*/
    p(-5, 7),    /*0b100001*/
    p(8, -3),    /*0b100010*/
    p(23, 6),    /*0b100011*/
    p(-30, -26), /*0b100100*/
    p(-23, -37), /*0b100101*/
    p(-15, -1),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(-10, -3),  /*0b101000*/
    p(-26, -12), /*0b101001*/
    p(6, -11),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-32, -25), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(2, 28),    /*0b110000*/
    p(30, 14),   /*0b110001*/
    p(14, -18),  /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(-0, 3),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(8, 35),    /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(17, -20),  /*0b111111*/
    p(-68, 3),   /*0b00*/
    p(-14, -23), /*0b01*/
    p(16, -12),  /*0b10*/
    p(35, -39),  /*0b11*/
    p(3, -18),   /*0b100*/
    p(-40, -44), /*0b101*/
    p(52, -54),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(25, -17),  /*0b1000*/
    p(0, -43),   /*0b1001*/
    p(40, -83),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(33, -16),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(16, -26),  /*0b1111*/
    p(-38, 2),   /*0b00*/
    p(-4, -16),  /*0b01*/
    p(-7, -20),  /*0b10*/
    p(19, -38),  /*0b11*/
    p(-18, -12), /*0b100*/
    p(9, -55),   /*0b101*/
    p(-5, -32),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(-18, -8),  /*0b1000*/
    p(18, -27),  /*0b1001*/
    p(5, -73),   /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(2, -12),   /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(8, -73),   /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] = [
    p(15, 17),
    p(3, 12),
    p(1, 12),
    p(3, 8),
    p(-8, 18),
    p(-38, 13),
];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(33, 13),
    p(46, 29),
    p(51, -2),
    p(38, -31),
    p(0, 0),
];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;

const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-25, 11),
        p(-9, 7),
        p(-1, 12),
        p(2, 10),
        p(3, 13),
        p(2, 17),
        p(-1, 18),
        p(-2, 18),
        p(-1, 13),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-16, -37),
        p(-8, -23),
        p(-2, -14),
        p(-0, -5),
        p(5, 7),
        p(12, 17),
        p(17, 21),
        p(19, 27),
        p(19, 34),
        p(20, 34),
        p(22, 34),
        p(25, 36),
        p(23, 40),
        p(46, 31),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-64, 14),
        p(-55, 30),
        p(-51, 32),
        p(-47, 36),
        p(-50, 43),
        p(-46, 44),
        p(-43, 46),
        p(-41, 47),
        p(-41, 53),
        p(-39, 56),
        p(-37, 59),
        p(-36, 61),
        p(-36, 64),
        p(-30, 63),
        p(-31, 60),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-15, -3),
        p(-16, -18),
        p(-23, 41),
        p(-21, 49),
        p(-19, 58),
        p(-17, 67),
        p(-14, 74),
        p(-15, 89),
        p(-14, 98),
        p(-13, 102),
        p(-12, 111),
        p(-12, 116),
        p(-12, 122),
        p(-11, 128),
        p(-10, 132),
        p(-9, 137),
        p(-8, 141),
        p(-10, 150),
        p(-7, 154),
        p(-6, 155),
        p(3, 153),
        p(6, 153),
        p(5, 156),
        p(13, 154),
        p(28, 142),
        p(69, 129),
        p(81, 121),
        p(133, 99),
    ],
    [
        p(24, 17),
        p(30, -5),
        p(23, -12),
        p(15, -9),
        p(5, -3),
        p(-17, -0),
        p(-29, 3),
        p(-48, 6),
        p(-61, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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

/// This is a trait because there are two different instantiations:
/// The normal eval values and the version used by the tuner, where these functions return traces.
pub trait LiteValues: Debug + Default + Copy + Clone + Send + 'static {
    type Score: ScoreType;

    fn psqt(square: ChessSquare, piece: UncoloredChessPiece, color: Color) -> Self::Score;
    fn passed_pawn(square: ChessSquare) -> Self::Score;
    fn bishop_pair() -> Self::Score;
    fn rook_openness(openness: FileOpenness) -> Self::Score;
    fn king_openness(openness: FileOpenness) -> Self::Score;
    fn pawn_shield(config: usize) -> Self::Score;
    fn pawn_protection(piece: UncoloredChessPiece) -> Self::Score;
    fn pawn_attack(piece: UncoloredChessPiece) -> Self::Score;
    fn mobility(piece: UncoloredChessPiece, mobility: usize) -> Self::Score;
}

/// Eval values tuned on a combination of the zurichess dataset and a dataset used by 4ku,
/// created by GCP using his engine Stoofvlees and filtered by cj5716 using Stockfish at depth 9,
/// using my own tuner `pliers`.
#[derive(Debug, Default, Copy, Clone)]
pub struct Lite {}

impl LiteValues for Lite {
    type Score = PhasedScore;

    fn psqt(square: ChessSquare, piece: UncoloredChessPiece, color: Color) -> Self::Score {
        PSQTS[piece as usize][square.flip_if(color == White).bb_idx()]
    }

    fn passed_pawn(square: ChessSquare) -> Self::Score {
        PASSED_PAWNS[square.bb_idx()]
    }

    fn bishop_pair() -> Self::Score {
        BISHOP_PAIR
    }

    fn rook_openness(openness: FileOpenness) -> Self::Score {
        match openness {
            FileOpenness::Open => ROOK_OPEN_FILE,
            FileOpenness::Closed => ROOK_CLOSED_FILE,
            FileOpenness::SemiOpen => ROOK_SEMIOPEN_FILE,
            FileOpenness::SemiClosed => Self::Score::default(),
        }
    }

    fn king_openness(openness: FileOpenness) -> Self::Score {
        match openness {
            FileOpenness::Open => KING_OPEN_FILE,
            FileOpenness::Closed => KING_CLOSED_FILE,
            FileOpenness::SemiOpen => KING_SEMIOPEN_FILE,
            FileOpenness::SemiClosed => Self::Score::default(),
        }
    }

    fn pawn_shield(config: usize) -> Self::Score {
        PAWN_SHIELDS[config]
    }

    fn pawn_protection(piece: UncoloredChessPiece) -> Self::Score {
        PAWN_PROTECTION[piece as usize]
    }

    fn pawn_attack(piece: UncoloredChessPiece) -> Self::Score {
        PAWN_ATTACKS[piece as usize]
    }

    fn mobility(piece: UncoloredChessPiece, mobility: usize) -> Self::Score {
        MOBILITY[piece as usize - 1][mobility]
    }
}
