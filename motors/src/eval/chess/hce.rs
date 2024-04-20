use strum::IntoEnumIterator;

use gears::games::{Board, Color, DimT};
use gears::games::chess::Chessboard;
use gears::games::chess::pieces::UncoloredChessPiece;
use gears::games::chess::pieces::UncoloredChessPiece::{Pawn, Rook};
use gears::games::chess::squares::ChessSquare;
use gears::games::Color::{Black, White};
use gears::general::bitboards::Bitboard;
use gears::general::bitboards::chess::{A_FILE, ChessBitboard};
use gears::general::bitboards::RawBitboard;
use gears::search::Score;

use crate::eval::chess::hce::FileOpenness::{Closed, Open, SemiClosed, SemiOpen};
use crate::eval::Eval;

#[derive(Default, Debug)]
pub struct HandCraftedEval {}

/// Psqt values tuned on a combination of the zurichess and a dataset used by 4ku,
/// created by GCP using his engine Stoofvlees and filtered by cj5716 using Stockfish at depth 9,
/// using this tuner: https://github.com/GediminasMasaitis/texel-tuner.
#[rustfmt::skip]
const PSQTS: [[i32; 64]; 12] = [
    // pawn mg
    [
        0, 0, 0, 0, 0, 0, 0, 0,
        192, 198, 188, 214, 192, 171, 69, 59,
        80, 95, 128, 135, 138, 160, 131, 91,
        62, 85, 85, 91, 109, 105, 98, 81,
        51, 76, 72, 90, 87, 86, 85, 67,
        49, 72, 67, 69, 80, 78, 99, 74,
        48, 70, 63, 53, 70, 95, 110, 66,
        0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // pawn eg
    [
        0, 0, 0, 0, 0, 0, 0, 0,
        229, 227, 226, 181, 185, 193, 241, 246,
        188, 192, 164, 143, 137, 127, 167, 166,
        130, 123, 107, 96, 92, 93, 112, 107,
        111, 109, 94, 89, 91, 92, 102, 94,
        106, 108, 94, 103, 100, 95, 101, 90,
        111, 114, 102, 107, 112, 101, 100, 93,
        0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // knight mg
    [
        152, 194, 244, 277, 321, 228, 229, 196,
        282, 306, 339, 354, 340, 397, 309, 322,
        302, 342, 365, 377, 411, 418, 365, 332,
        305, 322, 347, 371, 350, 376, 330, 342,
        293, 310, 327, 327, 337, 331, 331, 305,
        274, 299, 312, 319, 330, 316, 322, 291,
        259, 272, 291, 302, 304, 308, 296, 291,
        224, 268, 257, 272, 278, 294, 273, 247,
    ],
    // knight eg
    [
        267, 325, 343, 332, 330, 323, 323, 244,
        318, 338, 340, 341, 336, 320, 331, 303,
        328, 341, 355, 358, 344, 340, 335, 320,
        336, 356, 369, 369, 373, 366, 357, 328,
        337, 349, 370, 373, 374, 364, 349, 329,
        322, 341, 350, 363, 361, 347, 335, 323,
        313, 332, 338, 341, 340, 334, 323, 322,
        301, 294, 322, 327, 327, 314, 301, 294,
    ],
    // bishop mg
    [
        309, 288, 293, 266, 271, 276, 321, 290,
        324, 355, 350, 332, 358, 355, 348, 323,
        339, 366, 366, 388, 380, 403, 381, 369,
        332, 351, 370, 385, 379, 374, 351, 336,
        329, 342, 350, 372, 368, 350, 345, 338,
        337, 348, 347, 351, 353, 348, 350, 352,
        341, 342, 355, 333, 341, 354, 362, 346,
        317, 342, 323, 314, 319, 320, 344, 328,
    ],
    // bishop eg
    [
        347, 359, 356, 367, 364, 357, 350, 347,
        340, 354, 358, 361, 353, 353, 357, 339,
        360, 358, 366, 357, 363, 365, 357, 355,
        358, 373, 367, 377, 374, 372, 372, 358,
        353, 369, 376, 374, 373, 373, 367, 346,
        352, 362, 370, 368, 374, 368, 353, 344,
        346, 347, 346, 361, 362, 350, 350, 328,
        334, 344, 332, 352, 348, 348, 331, 323,
    ],
    // rook mg
    [
        442, 432, 438, 445, 458, 468, 477, 487,
        417, 421, 440, 457, 443, 478, 471, 489,
        404, 426, 427, 427, 459, 470, 500, 470,
        396, 406, 409, 419, 424, 435, 439, 441,
        389, 390, 389, 403, 404, 400, 423, 418,
        387, 388, 389, 394, 403, 410, 445, 426,
        388, 391, 398, 399, 407, 418, 430, 402,
        410, 404, 402, 411, 418, 423, 422, 414,
    ],
    // rook eg
    [
        627, 636, 639, 632, 629, 628, 626, 621,
        636, 643, 642, 631, 634, 624, 624, 610,
        634, 632, 632, 628, 617, 614, 607, 607,
        632, 630, 634, 627, 618, 615, 613, 606,
        622, 624, 626, 622, 618, 619, 608, 601,
        614, 616, 614, 614, 610, 605, 587, 587,
        610, 612, 613, 611, 605, 600, 594, 598,
        615, 613, 622, 614, 607, 610, 604, 603,
    ],
    // queen mg
    [
        842, 858, 884, 916, 914, 928, 961, 897,
        872, 858, 867, 862, 868, 906, 893, 932,
        879, 877, 883, 894, 909, 948, 949, 937,
        868, 873, 878, 880, 884, 895, 894, 903,
        873, 870, 873, 879, 882, 880, 893, 893,
        868, 880, 877, 876, 881, 887, 899, 892,
        868, 878, 888, 887, 886, 896, 902, 904,
        869, 860, 866, 882, 874, 862, 874, 869,
    ],
    // queen eg
    [
        1191, 1200, 1215, 1202, 1205, 1192, 1145, 1181,
        1161, 1201, 1230, 1246, 1261, 1220, 1201, 1171,
        1160, 1184, 1215, 1222, 1230, 1210, 1175, 1170,
        1167, 1192, 1208, 1228, 1240, 1226, 1213, 1187,
        1158, 1192, 1198, 1219, 1211, 1207, 1189, 1176,
        1152, 1163, 1186, 1183, 1185, 1178, 1158, 1146,
        1147, 1148, 1143, 1152, 1154, 1128, 1105, 1080,
        1137, 1145, 1151, 1140, 1143, 1136, 1116, 1115,
    ],
    // king mg
    [
        62, 20, 58, -44, 6, 4, 38, 151,
        -59, -9, -37, 45, 12, -11, 14, 3,
        -77, 19, -38, -49, -16, 33, 5, -26,
        -56, -57, -75, -98, -98, -76, -86, -99,
        -70, -64, -87, -103, -107, -94, -97, -112,
        -44, -20, -67, -72, -66, -74, -35, -52,
        43, 12, -4, -35, -38, -15, 23, 30,
        31, 65, 41, -52, 12, -39, 44, 41,
    ],
    // king eg
    [
        -103, -42, -37, -2, -19, -7, -10, -101,
        -10, 19, 28, 11, 26, 42, 36, -1,
        5, 25, 41, 51, 46, 44, 46, 16,
        -4, 30, 48, 58, 59, 51, 46, 18,
        -12, 19, 40, 54, 52, 40, 28, 9,
        -14, 5, 24, 34, 32, 23, 5, -8,
        -36, -12, -1, 10, 10, -1, -18, -35,
        -66, -57, -39, -13, -41, -21, -49, -78,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 31;
const ROOK_OPEN_FILE_EG: i32 = 11;
const ROOK_SEMIOPEN_FILE_MG: i32 = 6;
const ROOK_SEMIOPEN_FILE_EG: i32 = 9;
const ROOK_CLOSED_FILE_MG: i32 = -16;
const ROOK_CLOSED_FILE_EG: i32 = -5;
const KING_OPEN_FILE_MG: i32 = -77;
const KING_OPEN_FILE_EG: i32 = -8;
const KING_SEMIOPEN_FILE_MG: i32 = -35;
const KING_SEMIOPEN_FILE_EG: i32 = 8;
const KING_CLOSED_FILE_MG: i32 = 15;
const KING_CLOSED_FILE_EG: i32 = -15;
const PASSED_PAWN_MG: i32 = -7;
const PASSED_PAWN_EG: i32 = 23;

// TODO: Differentiate between rooks and kings in front of / behind pawns.

const PIECE_PHASE: [i32; 6] = [0, 1, 1, 2, 4, 0];

enum FileOpenness {
    Open,
    SemiOpen,
    SemiClosed,
    Closed,
}

fn file_openness(file: DimT, our_pawns: ChessBitboard, their_pawns: ChessBitboard) -> FileOpenness {
    let file = ChessBitboard::file_no(file);
    if (file & our_pawns).is_zero() && (file & their_pawns).is_zero() {
        Open
    } else if (file & our_pawns).is_zero() {
        SemiOpen
    } else if (file & our_pawns).has_set_bit() && (file & their_pawns).has_set_bit() {
        Closed
    } else {
        SemiClosed
    }
}

impl Eval<Chessboard> for HandCraftedEval {
    fn eval(&self, pos: Chessboard) -> Score {
        let mut mg = Score(0);
        let mut eg = Score(0);
        let mut phase = 0;

        for color in Color::iter() {
            let our_pawns = pos.colored_piece_bb(color, Pawn);
            let their_pawns = pos.colored_piece_bb(color.other(), Pawn);

            /// Rooks on (semi)open/closed files (semi-closed files are handled by adjusting the base rook values during tuning)
            let mut rooks = pos.colored_piece_bb(color, Rook);
            while rooks.has_set_bit() {
                let idx = rooks.pop_lsb();
                match file_openness(ChessSquare::new(idx).file(), our_pawns, their_pawns) {
                    Open => {
                        mg += Score(ROOK_OPEN_FILE_MG);
                        eg += Score(ROOK_OPEN_FILE_EG);
                    }
                    SemiOpen => {
                        mg += Score(ROOK_SEMIOPEN_FILE_MG);
                        eg += Score(ROOK_SEMIOPEN_FILE_EG);
                    }
                    SemiClosed => {}
                    Closed => {
                        mg += Score(ROOK_CLOSED_FILE_MG);
                        eg += Score(ROOK_CLOSED_FILE_EG);
                    }
                }
            }
            // King on (semi)open/closed file
            let king_file = pos.king_square(color).file();
            match file_openness(king_file, our_pawns, their_pawns) {
                Open => {
                    mg += Score(KING_OPEN_FILE_MG);
                    eg += Score(KING_OPEN_FILE_EG);
                }
                SemiOpen => {
                    mg += Score(KING_SEMIOPEN_FILE_MG);
                    eg += Score(KING_SEMIOPEN_FILE_EG);
                }
                SemiClosed => {}
                Closed => {
                    mg += Score(KING_CLOSED_FILE_MG);
                    eg += Score(KING_CLOSED_FILE_EG);
                }
            }

            for piece in UncoloredChessPiece::pieces() {
                let mut bb = pos.colored_piece_bb(color, piece);
                while bb.has_set_bit() {
                    let idx = bb.pop_lsb();
                    let mg_table = piece as usize * 2;
                    let eg_table = mg_table + 1;
                    let square = match color {
                        White => idx ^ 0b111_000,
                        Black => idx,
                    };
                    mg += Score(PSQTS[mg_table][square]);
                    eg += Score(PSQTS[eg_table][square]);
                    phase += PIECE_PHASE[piece as usize];

                    // Isolated pawns
                    if piece == Pawn {
                        let file = ChessBitboard::file_no(ChessSquare::new(idx).file());
                        // if (our_pawns.west() & file).is_zero()
                        //     && (our_pawns.east() & file).is_zero()
                        // {
                        //     mg += Score(ISOLATED_PAWN_MG);
                        //     eg += Score(ISOLATED_PAWN_EG);
                        // }
                        let blocking = if color == White {
                            A_FILE << (idx + 8)
                        } else {
                            A_FILE >> (64 - idx)
                        };
                        let blocking = blocking.west() | blocking | blocking.east();
                        let blocking = blocking & their_pawns;
                        if blocking.is_zero() {
                            mg += Score(PASSED_PAWN_MG);
                            eg += Score(PASSED_PAWN_EG);
                        }
                    }
                }
            }
            mg = -mg;
            eg = -eg;
        }
        let score = (mg * phase + eg * (24 - phase)) / 24;
        match pos.active_player() {
            White => score,
            Black => -score,
        }
    }
}
