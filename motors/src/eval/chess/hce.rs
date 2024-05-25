use strum::IntoEnumIterator;

use gears::games::chess::pieces::UncoloredChessPiece::{Pawn, Rook};
use gears::games::chess::pieces::{UncoloredChessPiece, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::chess::Chessboard;
use gears::games::Color::{Black, White};
use gears::games::{Board, Color, DimT};
use gears::general::bitboards::chess::{ChessBitboard, A_FILE};
use gears::general::bitboards::Bitboard;
use gears::general::bitboards::RawBitboard;
use gears::search::Score;

use crate::eval::chess::hce::FileOpenness::{Closed, Open, SemiClosed, SemiOpen};
use crate::eval::Eval;

#[derive(Default, Debug)]
pub struct HandCraftedEval {}

/// Eval values tuned on a combination of the zurichess dataset and a dataset used by 4ku,
/// created by GCP using his engine Stoofvlees and filtered by cj5716 using Stockfish at depth 9,
/// using my own tuner `pliers`.
#[rustfmt::skip]
const PSQTS: [[i32; 64]; 12] = [
    // Pawn MG
    [
        0, 0, 0, 0, 0, 0, 0, 0,
        160, 170, 159, 186, 170, 151, 68, 47,
        78, 89, 110, 116, 123, 166, 148, 112,
        64, 87, 83, 88, 109, 105, 101, 86,
        52, 78, 74, 90, 88, 87, 87, 70,
        51, 74, 67, 68, 80, 80, 101, 77,
        50, 72, 63, 53, 70, 96, 111, 68,
        0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // Pawn EG
    [
        0, 0, 0, 0, 0, 0, 0, 0,
        275, 270, 266, 220, 220, 231, 278, 287,
        129, 128, 105, 106, 99, 89, 120, 114,
        121, 116, 101, 90, 90, 90, 107, 97,
        110, 112, 97, 94, 95, 95, 103, 91,
        107, 110, 96, 106, 103, 97, 101, 89,
        112, 113, 102, 106, 114, 103, 100, 91,
        0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // Knight MG
    [
        151, 196, 243, 276, 320, 226, 232, 194,
        282, 305, 339, 354, 339, 397, 308, 320,
        303, 342, 364, 376, 410, 419, 364, 331,
        305, 322, 346, 371, 350, 375, 329, 342,
        292, 309, 327, 326, 336, 331, 331, 304,
        273, 298, 311, 319, 330, 316, 321, 290,
        258, 270, 290, 302, 303, 308, 295, 290,
        223, 267, 256, 272, 278, 294, 272, 247,
    ],
    // Knight EG
    [
        270, 326, 346, 334, 332, 327, 322, 248,
        322, 339, 342, 344, 339, 323, 333, 307,
        330, 343, 358, 361, 347, 342, 338, 323,
        340, 360, 372, 373, 376, 370, 361, 332,
        343, 354, 373, 377, 378, 367, 353, 333,
        327, 346, 355, 367, 364, 350, 339, 327,
        319, 337, 342, 345, 344, 338, 326, 326,
        305, 298, 327, 330, 330, 317, 304, 299,
    ],
    // Bishop MG
    [
        307, 286, 291, 265, 273, 275, 320, 290,
        324, 353, 349, 331, 357, 355, 347, 321,
        338, 365, 365, 386, 379, 402, 380, 368,
        332, 351, 370, 385, 379, 374, 350, 335,
        328, 342, 349, 371, 367, 350, 344, 337,
        335, 348, 347, 351, 353, 348, 349, 351,
        341, 341, 354, 332, 341, 354, 362, 345,
        317, 341, 323, 313, 319, 319, 343, 328,
    ],
    // Bishop EG
    [
        351, 364, 360, 370, 366, 360, 355, 350,
        344, 357, 362, 365, 356, 357, 360, 344,
        363, 361, 369, 361, 366, 369, 360, 359,
        362, 376, 370, 381, 378, 375, 376, 361,
        359, 374, 380, 378, 377, 377, 371, 350,
        357, 367, 374, 372, 378, 372, 357, 349,
        352, 351, 351, 364, 365, 354, 353, 333,
        337, 348, 336, 356, 351, 351, 336, 325,
    ],
    // Rook MG
    [
        442, 431, 436, 443, 456, 465, 474, 486,
        417, 419, 438, 455, 442, 477, 471, 487,
        401, 425, 425, 426, 457, 469, 501, 469,
        393, 404, 408, 418, 422, 433, 438, 439,
        387, 388, 387, 401, 402, 399, 421, 415,
        385, 386, 388, 393, 402, 408, 444, 424,
        386, 390, 396, 398, 405, 417, 429, 400,
        409, 403, 401, 410, 417, 421, 421, 413,
    ],
    // Rook EG
    [
        631, 640, 644, 637, 634, 634, 632, 625,
        642, 647, 646, 635, 639, 628, 627, 616,
        639, 636, 636, 632, 621, 617, 610, 611,
        638, 635, 637, 631, 623, 619, 619, 611,
        629, 630, 631, 627, 623, 624, 614, 609,
        621, 621, 619, 619, 615, 610, 592, 593,
        616, 618, 618, 615, 609, 604, 597, 603,
        619, 617, 626, 619, 611, 614, 608, 607,
    ],
    // Queen MG
    [
        841, 859, 883, 914, 909, 929, 958, 895,
        874, 857, 867, 860, 866, 905, 891, 928,
        880, 876, 882, 893, 908, 947, 946, 935,
        868, 873, 878, 879, 883, 894, 892, 901,
        872, 869, 872, 878, 881, 879, 892, 892,
        868, 879, 876, 876, 880, 886, 898, 891,
        867, 878, 887, 887, 886, 896, 902, 903,
        869, 860, 865, 882, 873, 862, 872, 869,
    ],
    // Queen EG
    [
        1200, 1209, 1223, 1213, 1218, 1200, 1155, 1192,
        1170, 1209, 1238, 1255, 1270, 1229, 1212, 1184,
        1169, 1193, 1223, 1231, 1239, 1220, 1186, 1181,
        1179, 1203, 1217, 1238, 1249, 1236, 1224, 1198,
        1170, 1203, 1207, 1229, 1221, 1217, 1199, 1187,
        1162, 1173, 1196, 1193, 1195, 1187, 1168, 1155,
        1158, 1158, 1152, 1161, 1163, 1137, 1113, 1089,
        1146, 1153, 1159, 1148, 1151, 1143, 1126, 1125,
    ],
    // King MG
    [
        58, 22, 53, -44, 1, 3, 51, 155,
        -62, -16, -36, 36, 12, -14, 20, 15,
        -77, 18, -36, -51, -13, 36, 10, -18,
        -58, -57, -74, -98, -98, -74, -83, -95,
        -75, -66, -90, -106, -108, -94, -97, -114,
        -45, -22, -68, -73, -67, -74, -37, -53,
        45, 12, -4, -35, -39, -16, 23, 30,
        32, 64, 41, -51, 12, -39, 44, 41,
    ],
    // King EG
    [
        -106, -43, -36, -1, -18, -9, -14, -104,
        -13, 19, 26, 13, 25, 40, 31, -6,
        1, 23, 39, 50, 45, 42, 42, 11,
        -5, 29, 47, 57, 59, 50, 44, 14,
        -10, 19, 41, 56, 53, 40, 28, 10,
        -13, 6, 26, 36, 34, 24, 6, -6,
        -34, -9, 1, 11, 11, 0, -16, -33,
        -66, -56, -38, -12, -41, -20, -48, -77,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 31;
const ROOK_OPEN_FILE_EG: i32 = 12;
const ROOK_SEMIOPEN_FILE_MG: i32 = 6;
const ROOK_SEMIOPEN_FILE_EG: i32 = 12;
const ROOK_CLOSED_FILE_MG: i32 = -16;
const ROOK_CLOSED_FILE_EG: i32 = -3;
const KING_OPEN_FILE_MG: i32 = -77;
const KING_OPEN_FILE_EG: i32 = -9;
const KING_SEMIOPEN_FILE_MG: i32 = -35;
const KING_SEMIOPEN_FILE_EG: i32 = 8;
const KING_CLOSED_FILE_MG: i32 = 15;
const KING_CLOSED_FILE_EG: i32 = -16;
#[rustfmt::skip]
const PASSED_PAWNS: [[i32; 64]; 2] = [
    [
        0, 0, 0, 0, 0, 0, 0, 0,
        21, 16, 20, 20, 12, 9, -8, 1,
        33, 42, 33, 19, 18, 8, -32, -51,
        12, 8, 18, 17, 0, 5, -23, -18,
        -3, -13, -19, -12, -19, -10, -26, -16,
        -10, -23, -24, -19, -19, -15, -24, 2,
        -18, -9, -17, -20, -5, -4, 2, -4,
        0, 0, 0, 0, 0, 0, 0, 0,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0,
        -14, -13, -11, -12, -5, -7, -7, -9,
        109, 107, 94, 64, 68, 90, 97, 117,
        57, 54, 45, 37, 39, 45, 61, 63,
        31, 28, 26, 19, 22, 25, 39, 37,
        0, 7, 12, 2, 8, 9, 21, 5,
        3, 5, 15, 10, -1, 4, 6, 6,
        0, 0, 0, 0, 0, 0, 0, 0,
    ]
];

// TODO: Differentiate between rooks and kings in front of / behind pawns.

const PIECE_PHASE: [i32; 6] = [0, 1, 1, 2, 4, 0];

enum FileOpenness {
    Open,
    Closed,
    SemiOpen,
    SemiClosed,
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

            // Rooks on (semi)open/closed files (semi-closed files are handled by adjusting the base rook values during tuning)
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

                    // Passed pawns.
                    if piece == Pawn {
                        let in_front = if color == White {
                            A_FILE << (idx + 8)
                        } else {
                            A_FILE >> (64 - idx)
                        };
                        let blocking = in_front | in_front.west() | in_front.east();
                        if (in_front & our_pawns).is_zero() && (blocking & their_pawns).is_zero() {
                            mg += Score(PASSED_PAWNS[0][square]);
                            eg += Score(PASSED_PAWNS[1][square]);
                        }
                    }
                }
            }
            mg = -mg;
            eg = -eg;
        }
        let score = (mg * phase + eg * (24 - phase)) / 24;
        let tempo = Score(10);
        tempo
            + match pos.active_player() {
                White => score,
                Black => -score,
            }
    }
}
