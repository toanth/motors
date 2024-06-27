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
        p( 136,  175),    p( 134,  173),    p( 133,  172),    p( 147,  150),    p( 133,  153),    p( 125,  157),    p(  72,  181),    p(  71,  183),
        p(  74,  114),    p(  81,  114),    p(  99,   98),    p( 105,  101),    p( 112,   92),    p( 150,   84),    p( 135,  111),    p( 104,  106),
        p(  57,  103),    p(  75,   97),    p(  72,   88),    p(  75,   80),    p(  98,   80),    p(  93,   78),    p(  89,   91),    p(  77,   85),
        p(  45,   93),    p(  65,   93),    p(  62,   83),    p(  76,   80),    p(  78,   80),    p(  78,   78),    p(  79,   84),    p(  58,   77),
        p(  38,   88),    p(  51,   86),    p(  51,   81),    p(  50,   92),    p(  61,   87),    p(  57,   83),    p(  73,   78),    p(  50,   76),
        p(  45,   96),    p(  60,   96),    p(  55,   92),    p(  39,   97),    p(  57,  103),    p(  74,   94),    p(  89,   85),    p(  51,   84),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 141,  261),    p( 190,  309),    p( 232,  330),    p( 262,  318),    p( 307,  316),    p( 213,  312),    p( 222,  306),    p( 182,  239),
        p( 270,  305),    p( 291,  322),    p( 325,  326),    p( 340,  327),    p( 319,  324),    p( 377,  307),    p( 292,  317),    p( 306,  291),
        p( 290,  314),    p( 327,  326),    p( 350,  338),    p( 357,  341),    p( 391,  327),    p( 397,  322),    p( 352,  317),    p( 315,  307),
        p( 291,  323),    p( 309,  339),    p( 331,  350),    p( 355,  351),    p( 337,  353),    p( 359,  347),    p( 317,  340),    p( 326,  314),
        p( 279,  323),    p( 294,  335),    p( 313,  352),    p( 313,  355),    p( 324,  357),    p( 315,  344),    p( 317,  330),    p( 290,  314),
        p( 259,  309),    p( 283,  326),    p( 297,  336),    p( 305,  349),    p( 313,  344),    p( 301,  331),    p( 304,  317),    p( 274,  309),
        p( 248,  304),    p( 260,  321),    p( 280,  328),    p( 290,  331),    p( 291,  328),    p( 297,  322),    p( 285,  310),    p( 282,  309),
        p( 215,  291),    p( 255,  286),    p( 247,  313),    p( 262,  315),    p( 269,  316),    p( 283,  303),    p( 260,  292),    p( 237,  286),
    ],
    // bishop
    [
        p( 274,  332),    p( 255,  342),    p( 255,  339),    p( 231,  348),    p( 243,  345),    p( 237,  341),    p( 290,  336),    p( 257,  330),
        p( 293,  324),    p( 321,  335),    p( 316,  339),    p( 297,  344),    p( 325,  335),    p( 326,  336),    p( 316,  338),    p( 294,  324),
        p( 308,  339),    p( 332,  338),    p( 334,  347),    p( 351,  338),    p( 351,  343),    p( 379,  344),    p( 355,  337),    p( 339,  337),
        p( 300,  339),    p( 320,  352),    p( 337,  348),    p( 353,  359),    p( 347,  355),    p( 344,  352),    p( 320,  352),    p( 307,  339),
        p( 299,  337),    p( 311,  350),    p( 319,  358),    p( 339,  356),    p( 335,  356),    p( 319,  353),    p( 314,  349),    p( 306,  329),
        p( 301,  335),    p( 316,  344),    p( 315,  351),    p( 320,  351),    p( 320,  356),    p( 316,  350),    p( 316,  335),    p( 316,  327),
        p( 306,  332),    p( 310,  329),    p( 322,  330),    p( 300,  342),    p( 309,  344),    p( 319,  334),    p( 329,  331),    p( 311,  314),
        p( 287,  315),    p( 308,  328),    p( 290,  315),    p( 284,  333),    p( 288,  330),    p( 288,  330),    p( 303,  317),    p( 296,  304),
    ],
    // rook
    [
        p( 423,  603),    p( 412,  612),    p( 418,  614),    p( 424,  608),    p( 435,  606),    p( 447,  605),    p( 454,  603),    p( 464,  597),
        p( 400,  612),    p( 402,  617),    p( 420,  616),    p( 436,  607),    p( 424,  609),    p( 458,  598),    p( 455,  596),    p( 465,  587),
        p( 385,  611),    p( 407,  606),    p( 407,  607),    p( 409,  601),    p( 439,  592),    p( 453,  586),    p( 483,  581),    p( 445,  585),
        p( 377,  609),    p( 389,  605),    p( 390,  607),    p( 399,  601),    p( 405,  592),    p( 418,  589),    p( 425,  588),    p( 421,  583),
        p( 371,  601),    p( 373,  601),    p( 373,  602),    p( 385,  597),    p( 388,  593),    p( 386,  593),    p( 407,  584),    p( 398,  580),
        p( 370,  594),    p( 371,  594),    p( 373,  592),    p( 377,  592),    p( 385,  586),    p( 391,  582),    p( 425,  563),    p( 404,  566),
        p( 373,  588),    p( 377,  589),    p( 382,  590),    p( 383,  588),    p( 391,  581),    p( 404,  575),    p( 420,  566),    p( 386,  574),
        p( 394,  591),    p( 388,  589),    p( 387,  598),    p( 395,  592),    p( 402,  584),    p( 404,  587),    p( 407,  579),    p( 401,  576),
    ],
    // queen
    [
        p( 816, 1142),    p( 833, 1150),    p( 854, 1165),    p( 883, 1155),    p( 878, 1160),    p( 896, 1144),    p( 931, 1097),    p( 868, 1134),
        p( 848, 1111),    p( 834, 1147),    p( 842, 1176),    p( 833, 1194),    p( 841, 1207),    p( 880, 1167),    p( 872, 1147),    p( 905, 1121),
        p( 854, 1110),    p( 851, 1133),    p( 859, 1160),    p( 867, 1170),    p( 884, 1178),    p( 922, 1158),    p( 920, 1126),    p( 904, 1123),
        p( 842, 1119),    p( 851, 1140),    p( 856, 1155),    p( 857, 1174),    p( 861, 1187),    p( 870, 1173),    p( 868, 1164),    p( 877, 1137),
        p( 847, 1114),    p( 847, 1141),    p( 852, 1145),    p( 857, 1166),    p( 858, 1159),    p( 859, 1152),    p( 869, 1136),    p( 867, 1127),
        p( 846, 1101),    p( 857, 1113),    p( 854, 1136),    p( 855, 1130),    p( 860, 1131),    p( 865, 1124),    p( 878, 1103),    p( 868, 1093),
        p( 843, 1100),    p( 854, 1101),    p( 862, 1096),    p( 860, 1106),    p( 860, 1106),    p( 869, 1081),    p( 876, 1059),    p( 873, 1039),
        p( 842, 1090),    p( 836, 1094),    p( 840, 1104),    p( 855, 1092),    p( 848, 1094),    p( 837, 1086),    p( 841, 1072),    p( 843, 1067),
    ],
    // king
    [
        p(  84,  -91),    p(  30,  -39),    p(  54,  -31),    p( -33,    1),    p(   2,  -13),    p(  11,   -5),    p(  60,  -12),    p( 169,  -92),
        p( -31,   -3),    p(  -6,   19),    p( -25,   27),    p(  42,   15),    p(  18,   28),    p(  -3,   41),    p(  32,   32),    p(  18,    5),
        p( -44,   12),    p(  29,   23),    p( -28,   40),    p( -40,   50),    p(  -2,   45),    p(  48,   43),    p(  23,   41),    p( -16,   22),
        p( -27,    5),    p( -42,   28),    p( -62,   47),    p( -85,   58),    p( -83,   58),    p( -58,   50),    p( -60,   41),    p( -89,   26),
        p( -50,    3),    p( -56,   20),    p( -78,   41),    p(-100,   57),    p(-102,   55),    p( -81,   40),    p( -86,   29),    p(-114,   24),
        p( -34,    4),    p( -15,    7),    p( -64,   28),    p( -75,   40),    p( -69,   37),    p( -72,   27),    p( -37,    9),    p( -64,   15),
        p(  26,   -6),    p(   0,   -3),    p( -14,    7),    p( -42,   18),    p( -42,   18),    p( -29,    8),    p(  15,  -10),    p(   6,   -1),
        p( -16,  -26),    p(  32,  -43),    p(  25,  -32),    p( -49,   -7),    p(   9,  -33),    p( -48,  -12),    p(  22,  -38),    p(  12,  -39),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] =[
    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
    p(  36,   75),    p(  34,   73),    p(  33,   72),    p(  47,   50),    p(  33,   53),    p(  25,   57),    p( -28,   81),    p( -29,   83),
    p(  31,  112),    p(  40,  109),    p(  31,   90),    p(  19,   59),    p(  20,   63),    p(   3,   86),    p( -28,   94),    p( -53,  116),
    p(  14,   65),    p(  13,   61),    p(  20,   47),    p(  16,   37),    p(  -2,   38),    p(   9,   47),    p( -13,   64),    p( -14,   67),
    p(   2,   38),    p(  -8,   37),    p( -15,   30),    p( -10,   22),    p( -18,   26),    p( -10,   31),    p( -21,   45),    p( -12,   43),
    p(  -5,   11),    p( -14,   18),    p( -19,   18),    p( -12,    6),    p( -15,   13),    p(  -8,   15),    p( -13,   31),    p(   7,   13),
    p( -12,   10),    p(  -6,   13),    p( -12,   17),    p( -12,    9),    p(   2,    1),    p(   2,    6),    p(   6,   13),    p(   2,   10),
    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];

const BISHOP_PAIR: PhasedScore = p(24, 58);
const ROOK_OPEN_FILE: PhasedScore = p(29, 11);
const ROOK_CLOSED_FILE: PhasedScore = p(-14, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(8, 12);
const KING_OPEN_FILE: PhasedScore = p(-56, -10);
const KING_CLOSED_FILE: PhasedScore = p(16, -14);
const KING_SEMIOPEN_FILE: PhasedScore = p(-13, 8);
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-49, 11),  /*0b0000*/
    p(-29, 10),  /*0b0001*/
    p(-15, 4),   /*0b0010*/
    p(7, 22),    /*0b0011*/
    p(-10, 4),   /*0b0100*/
    p(-14, -5),  /*0b0101*/
    p(6, 13),    /*0b0110*/
    p(27, -6),   /*0b0111*/
    p(-35, 8),   /*0b1000*/
    p(-31, -15), /*0b1001*/
    p(-17, 7),   /*0b1010*/
    p(14, -6),   /*0b1011*/
    p(-12, 1),   /*0b1100*/
    p(-19, -22), /*0b1101*/
    p(14, 10),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-45, 12),  /*0b10000*/
    p(-8, 6),    /*0b10001*/
    p(-21, -16), /*0b10010*/
    p(5, -12),   /*0b10011*/
    p(-13, -0),  /*0b10100*/
    p(27, 7),    /*0b10101*/
    p(-7, -19),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(-22, 40),  /*0b11000*/
    p(7, 8),     /*0b11001*/
    p(14, 19),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(10, 8),    /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(-20, 5),   /*0b100000*/
    p(-13, 9),   /*0b100001*/
    p(2, -2),    /*0b100010*/
    p(23, 4),    /*0b100011*/
    p(-29, -25), /*0b100100*/
    p(-20, -39), /*0b100101*/
    p(-7, -5),   /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(-26, -6),  /*0b101000*/
    p(-33, -8),  /*0b101001*/
    p(-2, -10),  /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-34, -26), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(-12, 27),  /*0b110000*/
    p(22, 15),   /*0b110001*/
    p(7, -16),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(-1, 1),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(-9, 31),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(17, -21),  /*0b111111*/
    p(-70, 4),   /*0b00*/
    p(-5, -25),  /*0b01*/
    p(20, -12),  /*0b10*/
    p(46, -35),  /*0b11*/
    p(-1, -17),  /*0b100*/
    p(-32, -46), /*0b101*/
    p(55, -55),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(18, -15),  /*0b1000*/
    p(4, -43),   /*0b1001*/
    p(48, -85),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(26, -17),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(25, -23),  /*0b1111*/
    p(-37, 1),   /*0b00*/
    p(2, -18),   /*0b01*/
    p(4, -24),   /*0b10*/
    p(31, -35),  /*0b11*/
    p(-21, -13), /*0b100*/
    p(14, -56),  /*0b101*/
    p(-0, -33),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(-20, -8),  /*0b1000*/
    p(22, -29),  /*0b1001*/
    p(17, -75),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(-5, -13),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(17, -69),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 17), p(6, 13), p(0, 6), p(6, 8), p(-7, 12), p(-30, 15)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(33, 13),
    p(48, 32),
    p(50, -2),
    p(39, -31),
    p(0, 0),
];

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
}
