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
        191, 197, 187, 214, 191, 170, 69, 58,
        80, 95, 128, 135, 137, 160, 131, 91,
        62, 85, 85, 91, 109, 104, 98, 81,
        50, 76, 72, 89, 87, 86, 85, 67,
        49, 72, 67, 69, 80, 78, 99, 74,
        48, 70, 63, 53, 70, 95, 110, 65,
        0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // pawn eg
    [
        0, 0, 0, 0, 0, 0, 0, 0,
        228, 225, 224, 179, 183, 192, 240, 245,
        187, 190, 163, 141, 136, 126, 166, 164,
        128, 121, 106, 94, 91, 92, 111, 106,
        109, 108, 93, 88, 90, 91, 101, 92,
        104, 107, 92, 101, 99, 95, 100, 89,
        109, 112, 101, 105, 111, 101, 99, 91,
        0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // knight mg
    [
        152, 194, 244, 277, 321, 228, 229, 196,
        282, 306, 339, 354, 340, 397, 309, 322,
        302, 342, 365, 377, 411, 419, 365, 332,
        305, 322, 347, 371, 350, 376, 330, 343,
        293, 310, 327, 327, 337, 331, 331, 305,
        274, 299, 312, 319, 330, 316, 322, 291,
        259, 272, 291, 302, 304, 308, 296, 291,
        224, 268, 257, 272, 278, 294, 273, 247,
    ],
    // knight eg
    [
        267, 325, 343, 332, 330, 323, 322, 245,
        318, 338, 340, 341, 336, 321, 332, 304,
        328, 341, 355, 359, 344, 340, 335, 320,
        336, 356, 369, 370, 373, 366, 358, 328,
        337, 349, 370, 373, 375, 364, 350, 329,
        322, 341, 350, 363, 361, 347, 335, 323,
        313, 332, 338, 341, 340, 334, 323, 323,
        301, 294, 323, 327, 327, 314, 302, 294,
    ],
    // bishop mg
    [
        309, 288, 293, 266, 271, 276, 321, 291,
        324, 355, 350, 332, 359, 356, 348, 324,
        339, 366, 366, 388, 380, 403, 381, 369,
        332, 351, 370, 385, 379, 374, 351, 336,
        329, 342, 350, 372, 368, 350, 345, 338,
        337, 348, 347, 352, 353, 348, 350, 352,
        341, 342, 355, 333, 341, 354, 362, 346,
        317, 342, 323, 314, 319, 320, 344, 328,
    ],
    // bishop eg
    [
        347, 359, 356, 368, 364, 357, 350, 347,
        340, 354, 359, 362, 353, 353, 357, 339,
        360, 358, 366, 357, 363, 365, 357, 355,
        358, 373, 367, 377, 375, 372, 372, 358,
        353, 370, 376, 374, 373, 373, 367, 346,
        352, 362, 370, 368, 374, 368, 354, 345,
        346, 348, 347, 361, 362, 350, 350, 329,
        334, 344, 332, 353, 348, 349, 331, 323,
    ],
    // rook mg
    [
        442, 432, 438, 445, 458, 468, 477, 487,
        417, 421, 440, 457, 444, 478, 472, 489,
        404, 426, 427, 427, 459, 471, 500, 470,
        396, 406, 409, 419, 424, 435, 439, 441,
        390, 390, 390, 403, 404, 400, 423, 418,
        387, 388, 389, 394, 403, 410, 446, 426,
        388, 391, 398, 399, 407, 418, 430, 402,
        411, 404, 403, 411, 418, 423, 422, 414,
    ],
    // rook eg
    [
        627, 636, 639, 632, 629, 628, 626, 621,
        636, 643, 641, 631, 634, 624, 624, 610,
        634, 632, 632, 628, 616, 613, 607, 607,
        631, 629, 633, 627, 618, 615, 613, 605,
        621, 624, 626, 621, 618, 619, 608, 601,
        614, 615, 614, 614, 610, 605, 587, 587,
        610, 612, 613, 611, 604, 600, 593, 597,
        615, 613, 622, 614, 607, 610, 604, 602,
    ],
    // queen mg
    [
        842, 858, 884, 916, 914, 928, 960, 897,
        872, 858, 867, 862, 868, 906, 893, 932,
        879, 876, 883, 894, 909, 948, 948, 937,
        868, 873, 878, 880, 884, 895, 893, 903,
        873, 870, 873, 879, 881, 880, 893, 893,
        868, 880, 876, 876, 881, 887, 899, 892,
        868, 878, 888, 887, 886, 896, 902, 904,
        869, 860, 866, 882, 874, 862, 874, 869,
    ],
    // queen eg
    [
        1191, 1201, 1216, 1202, 1206, 1193, 1146, 1182,
        1162, 1201, 1231, 1246, 1261, 1221, 1202, 1172,
        1160, 1185, 1216, 1223, 1231, 1211, 1175, 1170,
        1167, 1192, 1208, 1228, 1241, 1226, 1213, 1188,
        1159, 1192, 1198, 1220, 1212, 1207, 1190, 1177,
        1152, 1164, 1186, 1183, 1186, 1178, 1159, 1146,
        1147, 1149, 1143, 1152, 1155, 1129, 1106, 1081,
        1138, 1146, 1152, 1141, 1143, 1136, 1117, 1116,
    ],
    // king mg
    [
        64, 22, 58, -44, 6, 5, 39, 152,
        -59, -9, -36, 46, 13, -10, 15, 3,
        -76, 20, -37, -47, -15, 34, 6, -25,
        -55, -56, -75, -97, -97, -76, -85, -99,
        -70, -64, -87, -102, -108, -94, -97, -113,
        -44, -20, -68, -73, -67, -75, -36, -53,
        43, 11, -4, -35, -39, -16, 23, 29,
        31, 64, 41, -52, 12, -39, 44, 41,
    ],
    // king eg
    [
        -104, -42, -37, -2, -19, -7, -11, -101,
        -10, 19, 28, 11, 26, 41, 36, -1,
        5, 25, 41, 50, 46, 44, 45, 16,
        -5, 30, 48, 58, 59, 51, 46, 17,
        -12, 19, 40, 55, 52, 40, 28, 9,
        -14, 5, 25, 35, 33, 23, 5, -8,
        -36, -11, 0, 11, 11, -1, -18, -35,
        -66, -57, -39, -12, -41, -21, -49, -78,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 31;
const ROOK_OPEN_FILE_EG: i32 = 12;
const ROOK_SEMIOPEN_FILE_MG: i32 = 6;
const ROOK_SEMIOPEN_FILE_EG: i32 = 11;
const ROOK_CLOSED_FILE_MG: i32 = -16;
const ROOK_CLOSED_FILE_EG: i32 = -4;
const KING_OPEN_FILE_MG: i32 = -77;
const KING_OPEN_FILE_EG: i32 = -7;
const KING_SEMIOPEN_FILE_MG: i32 = -35;
const KING_SEMIOPEN_FILE_EG: i32 = 9;
const KING_CLOSED_FILE_MG: i32 = 15;
const KING_CLOSED_FILE_EG: i32 = -14;
const PASSED_PAWN_MG: i32 = -7;
const PASSED_PAWN_EG: i32 = 27;

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

        let all_pawns = pos.piece_bb(Pawn);
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
                        let file = if color == White {
                            A_FILE << (idx + 8)
                        } else {
                            A_FILE >> (64 - idx)
                        };
                        let hard_blockers = file & our_pawns;
                        let file = file.west() | file | file.east();
                        let blocking = (file & all_pawns) | hard_blockers;
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
