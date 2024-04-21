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
        182, 188, 177, 204, 181, 162, 58, 48,
        72, 88, 119, 124, 127, 154, 125, 84,
        54, 77, 77, 83, 102, 99, 92, 73,
        42, 68, 64, 80, 78, 80, 79, 59,
        40, 64, 58, 59, 72, 71, 93, 66,
        39, 62, 54, 45, 62, 88, 103, 58,
        0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // pawn eg
    [
        0, 0, 0, 0, 0, 0, 0, 0,
        224, 221, 221, 176, 180, 189, 237, 241,
        182, 186, 160, 139, 132, 122, 162, 160,
        123, 115, 99, 88, 82, 85, 104, 100,
        103, 100, 84, 78, 79, 81, 93, 86,
        99, 99, 84, 92, 88, 86, 92, 82,
        103, 104, 94, 98, 103, 92, 90, 84,
        0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // knight mg
    [
        153, 195, 244, 278, 322, 229, 230, 196,
        284, 307, 340, 355, 341, 399, 310, 323,
        303, 343, 366, 378, 412, 419, 365, 332,
        306, 323, 348, 372, 351, 377, 330, 344,
        294, 311, 328, 328, 338, 332, 332, 306,
        275, 300, 312, 320, 331, 317, 323, 291,
        260, 272, 292, 303, 305, 309, 297, 292,
        225, 269, 258, 273, 279, 295, 274, 248,
    ],
    // knight eg
    [
        267, 325, 344, 332, 330, 323, 322, 245,
        316, 337, 340, 341, 336, 320, 330, 302,
        328, 342, 356, 359, 344, 340, 335, 319,
        337, 357, 369, 370, 373, 366, 358, 328,
        337, 349, 370, 373, 375, 365, 350, 330,
        322, 341, 351, 363, 362, 347, 336, 324,
        313, 333, 339, 342, 341, 335, 324, 324,
        301, 294, 323, 327, 327, 315, 302, 295,
    ],
    // bishop mg
    [
        310, 288, 294, 267, 273, 277, 322, 291,
        325, 356, 352, 334, 360, 357, 350, 325,
        339, 367, 366, 388, 381, 403, 381, 370,
        333, 352, 371, 386, 381, 375, 352, 337,
        330, 344, 351, 373, 369, 351, 346, 339,
        338, 349, 348, 352, 354, 349, 351, 353,
        342, 343, 356, 334, 342, 355, 363, 348,
        318, 343, 324, 315, 320, 321, 345, 329,
    ],
    // bishop eg
    [
        348, 360, 356, 367, 364, 357, 351, 346,
        339, 354, 359, 361, 353, 354, 357, 339,
        360, 359, 367, 358, 364, 365, 358, 355,
        359, 373, 367, 377, 374, 372, 373, 359,
        354, 371, 377, 375, 374, 374, 368, 346,
        353, 363, 371, 369, 376, 369, 354, 345,
        347, 348, 347, 362, 363, 351, 351, 329,
        335, 344, 333, 353, 349, 350, 332, 324,
    ],
    // rook mg
    [
        444, 434, 440, 446, 459, 469, 478, 488,
        419, 423, 441, 458, 445, 480, 474, 490,
        404, 426, 427, 428, 459, 470, 500, 470,
        397, 406, 409, 419, 424, 434, 439, 441,
        390, 391, 390, 404, 405, 401, 424, 419,
        388, 389, 390, 396, 404, 411, 446, 427,
        388, 392, 399, 400, 408, 419, 431, 403,
        411, 405, 403, 412, 419, 424, 423, 415,
    ],
    // rook eg
    [
        627, 636, 639, 633, 629, 628, 626, 621,
        636, 643, 642, 631, 634, 624, 623, 610,
        634, 633, 632, 628, 617, 614, 609, 608,
        632, 630, 634, 628, 619, 616, 615, 607,
        622, 625, 627, 622, 619, 620, 609, 602,
        614, 616, 615, 615, 611, 606, 588, 588,
        610, 613, 614, 612, 606, 601, 594, 599,
        615, 614, 622, 615, 608, 611, 605, 603,
    ],
    // queen mg
    [
        846, 862, 888, 921, 918, 933, 964, 900,
        875, 863, 872, 866, 872, 910, 897, 934,
        883, 880, 887, 899, 913, 952, 952, 940,
        872, 878, 882, 884, 888, 899, 898, 907,
        878, 875, 877, 883, 886, 884, 897, 897,
        873, 884, 881, 880, 884, 891, 903, 896,
        872, 882, 892, 891, 890, 900, 906, 908,
        873, 864, 870, 886, 878, 867, 878, 874,
    ],
    // queen eg
    [
        1191, 1202, 1216, 1202, 1206, 1192, 1146, 1183,
        1161, 1199, 1231, 1246, 1262, 1220, 1201, 1172,
        1160, 1186, 1215, 1222, 1231, 1210, 1175, 1169,
        1167, 1192, 1208, 1229, 1241, 1226, 1213, 1187,
        1159, 1192, 1199, 1221, 1212, 1207, 1190, 1176,
        1152, 1164, 1187, 1184, 1186, 1180, 1160, 1147,
        1147, 1149, 1144, 1153, 1156, 1130, 1106, 1081,
        1138, 1146, 1152, 1141, 1144, 1137, 1117, 1116,
    ],
    // king mg
    [
        60, 20, 57, -45, 7, 6, 38, 148,
        -60, -12, -36, 44, 14, -10, 13, 1,
        -78, 18, -39, -49, -14, 34, 2, -29,
        -56, -58, -74, -97, -95, -76, -87, -102,
        -70, -63, -86, -100, -105, -94, -97, -113,
        -43, -18, -65, -71, -65, -74, -36, -52,
        44, 13, -3, -34, -38, -15, 23, 29,
        31, 65, 42, -51, 13, -39, 44, 41,
    ],
    // king eg
    [
        -103, -43, -37, -2, -19, -7, -10, -100,
        -10, 20, 27, 12, 26, 42, 37, 0,
        6, 25, 41, 51, 46, 45, 47, 17,
        -4, 31, 48, 58, 59, 51, 46, 19,
        -11, 19, 40, 54, 52, 40, 28, 9,
        -14, 4, 24, 34, 32, 23, 5, -8,
        -37, -12, -1, 10, 11, -1, -18, -35,
        -66, -58, -40, -13, -41, -21, -49, -77,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 31;
const ROOK_OPEN_FILE_EG: i32 = 11;
const ROOK_SEMIOPEN_FILE_MG: i32 = 7;
const ROOK_SEMIOPEN_FILE_EG: i32 = 11;
const ROOK_CLOSED_FILE_MG: i32 = -15;
const ROOK_CLOSED_FILE_EG: i32 = -5;
const KING_OPEN_FILE_MG: i32 = -76;
const KING_OPEN_FILE_EG: i32 = -6;
const KING_SEMIOPEN_FILE_MG: i32 = -34;
const KING_SEMIOPEN_FILE_EG: i32 = 11;
const KING_CLOSED_FILE_MG: i32 = 16;
const KING_CLOSED_FILE_EG: i32 = -13;
const PASSED_PAWN_MG: i32 = 5;
const PASSED_PAWN_EG: i32 = 30;

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
                        let in_front = if color == White {
                            A_FILE << (idx + 8)
                        } else {
                            A_FILE >> (64 - idx)
                        };
                        let hard_blockers = in_front & all_pawns;
                        if hard_blockers.is_zero() {
                            let neighboring_files = file.west() | file.east();
                            let blocking_squares = in_front.west() | in_front.east();
                            let blocking_pawns = blocking_squares & their_pawns;
                            let supporting_squares = neighboring_files & !blocking_squares;
                            let supporting_pawns = supporting_squares & our_pawns;
                            if blocking_pawns.0.count_ones() <= supporting_pawns.0.count_ones() {
                                mg += Score(PASSED_PAWN_MG);
                                eg += Score(PASSED_PAWN_EG);
                            }
                        }
                        // let file = file.west() | file | file.east();
                        // let blocking = (file & all_pawns) | hard_blockers;
                        // if blocking.is_zero() {
                        //     mg += Score(PAWN_MAJORITY_MG);
                        //     eg += Score(PAWN_MAJORITY_EG);
                        // }
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
