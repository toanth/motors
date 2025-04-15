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
const TEMPO: PhasedScore = p(10, 10);
// const TEMPO: PhasedScore = p(24, 14);
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p(24, 14),
        p(100, 100),
        p(100, 100),
        p(100, 100),
        p(100, 100),
        p(100, 100),
        p(100, 100),
        p(100, 100),
        p(100, 100),
        p(129, 163),
        p(125, 162),
        p(117, 166),
        p(125, 150),
        p(112, 155),
        p(110, 159),
        p(70, 175),
        p(74, 174),
        p(79, 119),
        p(75, 121),
        p(80, 114),
        p(88, 109),
        p(76, 104),
        p(129, 105),
        p(102, 124),
        p(103, 116),
        p(61, 107),
        p(67, 101),
        p(63, 94),
        p(83, 94),
        p(93, 93),
        p(86, 84),
        p(80, 95),
        p(77, 91),
        p(55, 95),
        p(55, 96),
        p(76, 87),
        p(93, 98),
        p(95, 90),
        p(87, 84),
        p(71, 87),
        p(65, 82),
        p(46, 93),
        p(53, 88),
        p(67, 92),
        p(79, 92),
        p(83, 90),
        p(82, 92),
        p(72, 79),
        p(55, 81),
        p(56, 96),
        p(61, 94),
        p(63, 93),
        p(58, 99),
        p(62, 101),
        p(79, 93),
        p(84, 83),
        p(61, 86),
        p(100, 100),
        p(100, 100),
        p(100, 100),
        p(100, 100),
        p(100, 100),
        p(100, 100),
        p(100, 100),
    ],
    // knight
    [
        p(300, 300),
        p(181, 279),
        p(197, 315),
        p(216, 325),
        p(248, 313),
        p(278, 313),
        p(199, 308),
        p(222, 307),
        p(208, 263),
        p(275, 316),
        p(291, 320),
        p(298, 310),
        p(299, 313),
        p(299, 307),
        p(309, 300),
        p(276, 319),
        p(285, 304),
        p(294, 312),
        p(306, 307),
        p(305, 313),
        p(316, 314),
        p(330, 310),
        p(347, 299),
        p(293, 308),
        p(289, 309),
        p(310, 318),
        p(317, 313),
        p(330, 314),
        p(329, 322),
        p(325, 323),
        p(326, 320),
        p(319, 319),
        p(332, 312),
        p(304, 319),
        p(312, 309),
        p(317, 314),
        p(326, 317),
        p(323, 321),
        p(330, 306),
        p(330, 307),
        p(319, 315),
        p(280, 304),
        p(288, 303),
        p(298, 298),
        p(305, 311),
        p(310, 309),
        p(298, 292),
        p(308, 295),
        p(298, 309),
        p(276, 312),
        p(286, 317),
        p(289, 306),
        p(297, 310),
        p(301, 306),
        p(295, 304),
        p(301, 309),
        p(294, 320),
        p(250, 312),
        p(281, 307),
        p(274, 311),
        p(294, 314),
        p(302, 311),
        p(296, 301),
        p(288, 310),
    ],
    // bishop
    [
        p(277, 311),
        p(352, 394),
        p(258, 318),
        p(240, 304),
        p(227, 316),
        p(217, 315),
        p(229, 303),
        p(277, 307),
        p(336, 390),
        p(297, 307),
        p(282, 307),
        p(297, 308),
        p(282, 306),
        p(292, 303),
        p(295, 304),
        p(270, 311),
        p(281, 308),
        p(298, 306),
        p(309, 308),
        p(294, 311),
        p(310, 303),
        p(305, 305),
        p(343, 310),
        p(320, 307),
        p(321, 308),
        p(295, 310),
        p(303, 305),
        p(314, 303),
        p(314, 314),
        p(318, 310),
        p(314, 305),
        p(316, 304),
        p(294, 307),
        p(294, 305),
        p(295, 306),
        p(301, 306),
        p(319, 310),
        p(311, 307),
        p(309, 301),
        p(296, 303),
        p(318, 297),
        p(298, 303),
        p(303, 306),
        p(307, 310),
        p(303, 305),
        p(309, 308),
        p(305, 303),
        p(311, 294),
        p(310, 293),
        p(311, 310),
        p(304, 300),
        p(310, 303),
        p(302, 309),
        p(304, 307),
        p(306, 307),
        p(311, 296),
        p(315, 295),
        p(376, 385),
        p(315, 310),
        p(305, 304),
        p(295, 310),
        p(310, 308),
        p(292, 308),
        p(308, 302),
    ],
    // rook
    [
        p(382, 373),
        p(596, 652),
        p(583, 663),
        p(575, 669),
        p(575, 666),
        p(587, 661),
        p(613, 656),
        p(614, 658),
        p(626, 651),
        p(596, 657),
        p(594, 662),
        p(601, 663),
        p(615, 653),
        p(600, 657),
        p(620, 652),
        p(624, 651),
        p(642, 640),
        p(594, 652),
        p(612, 647),
        p(607, 648),
        p(606, 643),
        p(633, 635),
        p(645, 632),
        p(659, 633),
        p(634, 635),
        p(593, 653),
        p(602, 648),
        p(603, 650),
        p(605, 646),
        p(611, 639),
        p(626, 634),
        p(623, 641),
        p(619, 636),
        p(584, 650),
        p(588, 648),
        p(589, 649),
        p(596, 645),
        p(600, 642),
        p(596, 642),
        p(608, 638),
        p(598, 635),
        p(579, 647),
        p(582, 645),
        p(585, 643),
        p(587, 642),
        p(594, 636),
        p(603, 630),
        p(619, 619),
        p(602, 624),
        p(580, 644),
        p(585, 641),
        p(592, 642),
        p(595, 639),
        p(601, 633),
        p(616, 623),
        p(621, 619),
        p(591, 629),
        p(583, 647),
        p(584, 642),
        p(585, 646),
        p(589, 639),
        p(595, 632),
        p(599, 633),
        p(596, 633),
    ],
    // queen
    [
        p(586, 635),
        p(865, 954),
        p(867, 967),
        p(881, 978),
        p(903, 969),
        p(905, 973),
        p(929, 965),
        p(962, 922),
        p(909, 956),
        p(896, 946),
        p(874, 970),
        p(874, 993),
        p(868, 1008),
        p(873, 1022),
        p(908, 991),
        p(920, 968),
        p(951, 957),
        p(894, 956),
        p(890, 964),
        p(887, 985),
        p(889, 992),
        p(891, 1006),
        p(949, 980),
        p(951, 959),
        p(941, 965),
        p(887, 961),
        p(892, 967),
        p(888, 975),
        p(882, 990),
        p(889, 999),
        p(907, 985),
        p(915, 990),
        p(919, 971),
        p(888, 954),
        p(885, 967),
        p(885, 968),
        p(887, 983),
        p(892, 979),
        p(895, 979),
        p(904, 972),
        p(911, 966),
        p(887, 939),
        p(891, 954),
        p(888, 966),
        p(883, 969),
        p(890, 977),
        p(895, 967),
        p(908, 953),
        p(907, 942),
        p(885, 940),
        p(886, 946),
        p(890, 951),
        p(890, 966),
        p(891, 964),
        p(895, 947),
        p(906, 928),
        p(916, 901),
        p(871, 945),
        p(886, 931),
        p(885, 945),
        p(886, 949),
        p(894, 937),
        p(882, 940),
        p(884, 932),
    ],
    // king
    [
        p(891, 916),
        p(159, -61),
        p(73, -9),
        p(98, -1),
        p(33, 18),
        p(44, 9),
        p(10, 21),
        p(64, 10),
        p(198, -63),
        p(-19, 17),
        p(-55, 35),
        p(-56, 41),
        p(5, 26),
        p(-29, 36),
        p(-52, 48),
        p(-41, 38),
        p(-11, 21),
        p(-51, 25),
        p(-41, 26),
        p(-75, 39),
        p(-84, 44),
        p(-47, 41),
        p(-17, 33),
        p(-69, 36),
        p(-32, 23),
        p(-33, 10),
        p(-96, 20),
        p(-110, 34),
        p(-136, 39),
        p(-131, 39),
        p(-112, 32),
        p(-120, 23),
        p(-114, 25),
        p(-49, 1),
        p(-109, 8),
        p(-125, 24),
        p(-151, 33),
        p(-147, 33),
        p(-125, 20),
        p(-139, 13),
        p(-126, 15),
        p(-46, 4),
        p(-92, 2),
        p(-118, 14),
        p(-125, 20),
        p(-123, 20),
        p(-133, 15),
        p(-110, 4),
        p(-82, 10),
        p(13, -5),
        p(-82, -3),
        p(-91, 3),
        p(-109, 9),
        p(-116, 12),
        p(-101, 3),
        p(-77, -11),
        p(-12, -3),
        p(39, -14),
        p(27, -26),
        p(22, -14),
        p(-34, 2),
        p(9, -12),
        p(-29, -0),
        p(18, -22),
    ],
];

const BISHOP_PAIR: PhasedScore = p(23, 51);
const BAD_BISHOP: [PhasedScore; 9] =
    [p(13, 21), p(14, 18), p(14, 8), p(9, 1), p(5, -6), p(1, -15), p(-4, -23), p(-11, -35), p(-22, -42)];
const ROOK_OPEN_FILE: PhasedScore = p(14, 1);
const ROOK_CLOSED_FILE: PhasedScore = p(-11, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(5, -2);
const KING_OPEN_FILE: PhasedScore = p(-41, 5);
const KING_CLOSED_FILE: PhasedScore = p(13, -7);
const KING_SEMIOPEN_FILE: PhasedScore = p(-7, 10);
#[rustfmt::skip]
const BISHOP_OPENNESS: [[PhasedScore; 8]; 4] = [
    // Open
    [p(-73, -65), p(-0, 12), p(4, 14), p(5, 8), p(6, 9), p(5, 10), p(10, 5), p(19, -1)],
    // Closed
    [p(0, 0), p(0, 0), p(20, 1), p(-13, 17), p(2, 16), p(2, 9), p(3, 9), p(1, 3)],
    // SemiOpen
    [p(0, 0), p(-10, 34), p(16, 30), p(5, 17), p(3, 15), p(5, 9), p(5, 4), p(10, 3)],
    // SemiClosed
    [p(0, 0), p(12, -8), p(13, 15), p(6, 4), p(10, 6), p(3, 7), p(7, 6), p(3, 2)],
];
const PAWN_ADVANCED_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(22, 3),
    p(3, 2),
    p(3, -5),
    p(-6, 1),
    p(3, 0),
    p(-10, -9),
    p(-4, -5),
    p(-7, -9),
    p(1, -1),
    p(-10, 3),
    p(-5, -20),
    p(-13, -6),
    p(4, -3),
    p(-2, -3),
    p(7, -5),
    p(5, 22),
    p(-2, -7),
    p(-19, -4),
    p(-9, -5),
    p(-31, 18),
    p(-17, 2),
    p(-17, -15),
    p(10, 20),
    p(-39, 29),
    p(-13, -15),
    p(-18, -9),
    p(-27, -32),
    p(-38, 15),
    p(-12, 3),
    p(15, 8),
    p(-92, 110),
    p(0, 0),
    p(-2, -7),
    p(-15, -5),
    p(-3, -13),
    p(-22, -4),
    p(-26, -2),
    p(-50, -16),
    p(-32, 37),
    p(-44, 31),
    p(-8, -2),
    p(-21, -2),
    p(8, -11),
    p(-16, 40),
    p(-49, 23),
    p(1, -18),
    p(0, 0),
    p(0, 0),
    p(-2, -15),
    p(-17, 11),
    p(-7, -58),
    p(0, 0),
    p(-0, -3),
    p(-45, -1),
    p(0, 0),
    p(0, 0),
    p(-29, 0),
    p(-24, 4),
    p(-17, 17),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_PASSIVE_CENTER: [PhasedScore; NUM_PAWN_CENTER_CONFIGURATIONS] = [
    p(26, 6),
    p(10, 0),
    p(0, 5),
    p(-12, 1),
    p(9, -0),
    p(-20, -4),
    p(-12, 2),
    p(-26, -6),
    p(5, -2),
    p(-9, -7),
    p(-28, -4),
    p(-42, 3),
    p(-10, -1),
    p(-42, -0),
    p(-37, -8),
    p(-52, 71),
    p(8, -2),
    p(2, -8),
    p(-2, -10),
    p(-17, 0),
    p(-13, 1),
    p(-15, -9),
    p(-24, 4),
    p(-68, 175),
    p(-12, -12),
    p(-28, -14),
    p(-41, -26),
    p(5, -74),
    p(-22, -7),
    p(-18, -15),
    p(-77, 57),
    p(0, 0),
    p(14, 1),
    p(5, -2),
    p(-11, -5),
    p(-16, -6),
    p(0, 5),
    p(-21, -10),
    p(-12, 5),
    p(-19, 8),
    p(-5, -7),
    p(-21, -7),
    p(-28, -13),
    p(-34, -2),
    p(-12, -0),
    p(-48, -2),
    p(-8, 23),
    p(-54, 57),
    p(3, -5),
    p(-7, -5),
    p(-27, 55),
    p(0, 0),
    p(-16, -2),
    p(-19, 8),
    p(0, 0),
    p(0, 0),
    p(-20, -3),
    p(-41, 6),
    p(-41, -47),
    p(0, 0),
    p(2, -54),
    p(0, 0),
    p(0, 0),
    p(0, 0),
];
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-6, 6),    /*0b0000*/
    p(-16, 6),   /*0b0001*/
    p(-7, 9),    /*0b0010*/
    p(-11, 10),  /*0b0011*/
    p(-4, 1),    /*0b0100*/
    p(-31, 0),   /*0b0101*/
    p(-15, 3),   /*0b0110*/
    p(-23, -12), /*0b0111*/
    p(12, 3),    /*0b1000*/
    p(-6, 9),    /*0b1001*/
    p(1, 8),     /*0b1010*/
    p(-4, 12),   /*0b1011*/
    p(-1, 3),    /*0b1100*/
    p(-25, 5),   /*0b1101*/
    p(-11, 4),   /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(8, 7),     /*0b10000*/
    p(3, 5),     /*0b10001*/
    p(18, 8),    /*0b10010*/
    p(-3, 4),    /*0b10011*/
    p(-4, 1),    /*0b10100*/
    p(10, 10),   /*0b10101*/
    p(-22, -0),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(18, 7),    /*0b11000*/
    p(28, 11),   /*0b11001*/
    p(37, 22),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(13, -3),   /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(19, 1),    /*0b100000*/
    p(2, 8),     /*0b100001*/
    p(20, 1),    /*0b100010*/
    p(5, -2),    /*0b100011*/
    p(-8, -1),   /*0b100100*/
    p(-23, -8),  /*0b100101*/
    p(-27, 18),  /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(30, -3),   /*0b101000*/
    p(4, 10),    /*0b101001*/
    p(20, -7),   /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-2, 3),    /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(17, 1),    /*0b110000*/
    p(21, 3),    /*0b110001*/
    p(32, -7),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(2, 15),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(33, -5),   /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(3, -4),    /*0b111111*/
    p(-13, 6),   /*0b00*/
    p(1, -11),   /*0b01*/
    p(34, -6),   /*0b10*/
    p(22, -43),  /*0b11*/
    p(42, -11),  /*0b100*/
    p(-6, -13),  /*0b101*/
    p(62, -40),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(67, -12),  /*0b1000*/
    p(13, -26),  /*0b1001*/
    p(71, -50),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(55, -32),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(34, -5),   /*0b1111*/
    p(20, 2),    /*0b00*/
    p(29, -8),   /*0b01*/
    p(24, -13),  /*0b10*/
    p(20, -37),  /*0b11*/
    p(38, -7),   /*0b100*/
    p(52, -16),  /*0b101*/
    p(23, -18),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(39, -2),   /*0b1000*/
    p(48, -15),  /*0b1001*/
    p(50, -40),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(43, -24),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(18, -42),  /*0b1111*/
];
const PAWNLESS_FLANK: PhasedScore = p(-25, -34);
const STOPPABLE_PASSER: PhasedScore = p(36, -49);
const CLOSE_KING_PASSER: PhasedScore = p(-9, 28);
const IMMOBILE_PASSER: PhasedScore = p(-3, -37);
const PROTECTED_PASSER: PhasedScore = p(8, -3);

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p( -41,   40),    p( -49,   54),    p( -68,   57),    p( -69,   52),    p( -60,   40),    p( -48,   40),    p( -33,   44),    p( -48,   47),
        p( -32,   36),    p( -54,   57),    p( -63,   55),    p( -62,   48),    p( -70,   49),    p( -59,   49),    p( -60,   63),    p( -43,   46),
        p( -30,   55),    p( -33,   57),    p( -58,   63),    p( -55,   63),    p( -63,   60),    p( -54,   60),    p( -60,   70),    p( -62,   67),
        p( -14,   73),    p( -15,   75),    p( -15,   67),    p( -39,   76),    p( -55,   77),    p( -45,   76),    p( -53,   85),    p( -59,   87),
        p(   1,   70),    p(   7,   69),    p(   1,   54),    p( -19,   41),    p( -18,   53),    p( -40,   59),    p( -49,   61),    p( -85,   81),
        p(  29,   63),    p(  25,   62),    p(  17,   66),    p(  25,   50),    p(  12,   55),    p(  10,   59),    p( -30,   75),    p( -26,   74),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const CANDIDATE_PASSER: [PhasedScore; 6] = [p(1, 5), p(5, 8), p(8, 17), p(14, 22), p(16, 70), p(15, 63)];
const UNSUPPORTED_PAWN: PhasedScore = p(-7, -7);
const DOUBLED_PAWN: PhasedScore = p(-6, -22);
const PHALANX: [PhasedScore; 6] = [p(-1, -1), p(5, 3), p(8, 5), p(21, 20), p(56, 75), p(-95, 225)];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 14), p(7, 18), p(15, 21), p(7, 10), p(-3, 14), p(-44, 11)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [p(0, 0), p(60, 25), p(61, 49), p(75, 9), p(67, -18), p(0, 0)];
const PAWN_ADVANCE_THREAT: [PhasedScore; NUM_CHESS_PIECES] =
    [p(0, -5), p(14, 21), p(19, -7), p(16, 10), p(16, -9), p(27, -11)];

pub const MAX_MOBILITY: usize = 7 + 7 + 7 + 6;
const MOBILITY: [[PhasedScore; MAX_MOBILITY + 1]; NUM_CHESS_PIECES - 1] = [
    [
        p(-43, -66),
        p(-22, -26),
        p(-9, -2),
        p(0, 11),
        p(8, 22),
        p(15, 33),
        p(24, 36),
        p(30, 39),
        p(35, 37),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-34, -59),
        p(-21, -40),
        p(-9, -25),
        p(-2, -12),
        p(5, -2),
        p(10, 8),
        p(15, 13),
        p(19, 17),
        p(21, 23),
        p(28, 24),
        p(34, 24),
        p(39, 28),
        p(33, 38),
        p(46, 30),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-205, -81),
        p(-197, -66),
        p(-192, -60),
        p(-190, -54),
        p(-190, -47),
        p(-186, -41),
        p(-183, -36),
        p(-180, -32),
        p(-177, -26),
        p(-174, -22),
        p(-170, -19),
        p(-169, -14),
        p(-162, -12),
        p(-153, -14),
        p(-148, -18),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
        p(-9, 3),
        p(-7, 27),
        p(-12, 83),
        p(-7, 99),
        p(-5, 118),
        p(-0, 124),
        p(4, 134),
        p(7, 142),
        p(11, 147),
        p(15, 149),
        p(18, 153),
        p(22, 157),
        p(25, 158),
        p(27, 163),
        p(30, 166),
        p(35, 169),
        p(36, 175),
        p(40, 177),
        p(50, 174),
        p(64, 167),
        p(70, 167),
        p(112, 144),
        p(114, 147),
        p(139, 126),
        p(229, 94),
        p(279, 51),
        p(309, 40),
        p(310, 28),
    ],
    [
        p(-85, -2),
        p(-54, -13),
        p(-27, -12),
        p(0, -7),
        p(29, -4),
        p(49, -1),
        p(76, 5),
        p(99, 9),
        p(141, 2),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
        p(0, 0),
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
    [p(-4, 15), p(0, 0), p(32, 29), p(66, 13), p(52, -8), p(0, 0)],
    [p(-2, 13), p(22, 28), p(0, 0), p(50, 28), p(62, 81), p(0, 0)],
    [p(-4, 19), p(11, 19), p(19, 15), p(0, 0), p(76, 34), p(0, 0)],
    [p(-2, 10), p(2, 9), p(1, 25), p(1, 12), p(0, 0), p(0, 0)],
    [p(57, 22), p(-23, 26), p(16, 17), p(-16, 25), p(0, 0), p(0, 0)],
];
const DEFENDED: [[PhasedScore; NUM_CHESS_PIECES]; NUM_CHESS_PIECES - 1] = [
    [p(2, 9), p(8, 8), p(6, 11), p(13, 6), p(7, 16), p(11, 6)],
    [p(2, 9), p(11, 22), p(-47, -34), p(8, 12), p(10, 20), p(3, 7)],
    [p(1, 4), p(11, 9), p(8, 13), p(10, 10), p(8, 28), p(20, -4)],
    [p(2, 2), p(8, 3), p(6, -2), p(4, 13), p(-61, -229), p(5, -11)],
    [p(62, -2), p(39, 11), p(44, 5), p(21, 8), p(32, -3), p(0, 0)],
];
const KING_ZONE_ATTACK: [PhasedScore; 6] = [p(-19, -14), p(20, -10), p(9, -3), p(15, -15), p(-1, 5), p(0, 2)];
const CAN_GIVE_CHECK: [PhasedScore; 5] = [p(0, 0), p(21, 7), p(6, 15), p(26, -1), p(-2, 31)];
const CHECK_STM: PhasedScore = p(15, 8);
const DISCOVERED_CHECK_STM: PhasedScore = p(161, 37);
const DISCOVERED_CHECK: [PhasedScore; NUM_CHESS_PIECES] =
    [p(-7, -17), p(60, 0), p(97, -30), p(55, 83), p(0, 0), p(-28, -22)];
const PIN: [PhasedScore; NUM_CHESS_PIECES - 1] = [p(6, -17), p(26, 29), p(16, 33), p(43, 7), p(59, 3)];

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
