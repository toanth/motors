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
        190, 195, 184, 210, 187, 169, 67, 55,
        77, 91, 124, 130, 133, 157, 128, 88,
        59, 82, 82, 88, 106, 103, 97, 79,
        47, 74, 70, 86, 85, 84, 84, 64,
        46, 70, 64, 65, 78, 76, 98, 72,
        45, 68, 60, 51, 68, 93, 108, 63,
        0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // pawn eg
    [
        0, 0, 0, 0, 0, 0, 0, 0,
        221, 218, 219, 174, 176, 187, 234, 238,
        179, 181, 156, 135, 128, 115, 156, 156,
        119, 109, 93, 83, 76, 78, 96, 95,
        99, 93, 79, 73, 73, 75, 85, 80,
        94, 91, 79, 88, 83, 79, 84, 77,
        99, 97, 87, 90, 96, 84, 83, 79,
        0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // knight mg
    [
        152, 194, 244, 276, 322, 229, 229, 196,
        284, 307, 340, 355, 340, 398, 311, 323,
        303, 343, 366, 378, 412, 419, 365, 333,
        305, 323, 347, 372, 350, 376, 330, 343,
        294, 310, 328, 328, 337, 332, 332, 305,
        274, 299, 312, 319, 331, 317, 322, 291,
        259, 272, 291, 303, 304, 309, 297, 291,
        225, 269, 258, 272, 279, 295, 273, 247,
    ],
    // knight eg
    [
        266, 324, 342, 331, 329, 321, 321, 243,
        313, 335, 338, 340, 335, 318, 327, 298,
        326, 340, 354, 357, 342, 338, 333, 317,
        336, 355, 368, 369, 372, 365, 356, 327,
        336, 348, 369, 372, 373, 364, 349, 329,
        321, 340, 349, 362, 360, 346, 335, 323,
        311, 331, 338, 341, 340, 334, 322, 322,
        299, 292, 321, 326, 326, 313, 301, 294,
    ],
    // bishop mg
    [
        309, 288, 293, 265, 271, 277, 321, 290,
        325, 355, 351, 333, 359, 356, 348, 325,
        339, 366, 366, 388, 381, 403, 382, 369,
        333, 351, 371, 385, 380, 375, 351, 337,
        329, 343, 350, 372, 368, 351, 346, 338,
        337, 349, 348, 352, 354, 349, 350, 353,
        342, 342, 355, 333, 342, 355, 363, 347,
        318, 342, 324, 314, 320, 320, 345, 328,
    ],
    // bishop eg
    [
        348, 359, 356, 368, 365, 356, 350, 345,
        337, 353, 358, 361, 353, 353, 356, 338,
        359, 358, 367, 357, 363, 365, 357, 354,
        357, 373, 367, 377, 374, 372, 372, 358,
        353, 370, 376, 374, 373, 373, 367, 345,
        352, 362, 370, 368, 374, 369, 354, 344,
        346, 347, 346, 361, 363, 350, 350, 328,
        334, 344, 332, 353, 348, 349, 331, 323,
    ],
    // rook mg
    [
        444, 433, 440, 446, 460, 469, 478, 489,
        419, 423, 441, 458, 445, 480, 473, 491,
        405, 428, 428, 428, 460, 472, 502, 471,
        397, 407, 410, 420, 425, 435, 440, 441,
        390, 390, 390, 404, 405, 401, 424, 419,
        388, 389, 390, 395, 404, 411, 446, 427,
        389, 392, 399, 400, 408, 419, 431, 403,
        412, 405, 404, 412, 419, 424, 423, 415,
    ],
    // rook eg
    [
        625, 634, 636, 630, 627, 625, 623, 618,
        633, 640, 639, 629, 632, 621, 619, 606,
        632, 630, 630, 626, 615, 611, 605, 605,
        630, 628, 632, 625, 617, 614, 612, 605,
        620, 623, 625, 620, 617, 618, 607, 600,
        611, 613, 612, 612, 608, 603, 585, 585,
        607, 610, 612, 609, 603, 598, 591, 595,
        613, 611, 620, 612, 605, 608, 602, 601,
    ],
    // queen mg
    [
        845, 860, 886, 919, 917, 932, 962, 899,
        874, 861, 870, 864, 870, 908, 896, 934,
        882, 878, 886, 897, 912, 950, 950, 939,
        870, 876, 881, 882, 886, 897, 896, 905,
        875, 872, 875, 881, 884, 882, 895, 895,
        870, 882, 879, 878, 883, 889, 901, 894,
        870, 880, 890, 889, 888, 898, 904, 906,
        871, 862, 868, 884, 876, 865, 876, 872,
    ],
    // queen eg
    [
        1188, 1198, 1213, 1200, 1202, 1189, 1144, 1179,
        1157, 1196, 1228, 1245, 1259, 1217, 1197, 1167,
        1157, 1183, 1213, 1220, 1228, 1208, 1173, 1167,
        1164, 1190, 1206, 1227, 1239, 1224, 1211, 1185,
        1157, 1190, 1197, 1218, 1210, 1205, 1187, 1174,
        1150, 1162, 1184, 1181, 1183, 1177, 1157, 1144,
        1145, 1147, 1142, 1150, 1153, 1127, 1104, 1078,
        1136, 1144, 1150, 1139, 1141, 1134, 1114, 1113,
    ],
    // king mg
    [
        62, 20, 59, -44, 6, 4, 38, 148,
        -57, -10, -38, 42, 12, -11, 11, 3,
        -72, 18, -40, -51, -14, 33, 3, -27,
        -54, -58, -75, -99, -97, -77, -88, -102,
        -67, -63, -87, -101, -107, -94, -98, -113,
        -42, -17, -66, -72, -66, -74, -36, -52,
        44, 13, -3, -34, -38, -15, 23, 30,
        32, 65, 42, -51, 13, -39, 44, 41,
    ],
    // king eg
    [
        -102, -42, -38, -2, -19, -6, -10, -100,
        -11, 20, 28, 12, 27, 42, 37, -1,
        4, 26, 41, 51, 47, 46, 47, 17,
        -4, 31, 48, 58, 59, 52, 48, 19,
        -12, 19, 40, 54, 52, 41, 29, 10,
        -15, 3, 23, 34, 32, 23, 5, -8,
        -38, -13, -2, 9, 10, -2, -18, -35,
        -66, -58, -40, -14, -42, -21, -49, -77,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 31;
const ROOK_OPEN_FILE_EG: i32 = 12;
const ROOK_SEMIOPEN_FILE_MG: i32 = 7;
const ROOK_SEMIOPEN_FILE_EG: i32 = 11;
const ROOK_CLOSED_FILE_MG: i32 = -16;
const ROOK_CLOSED_FILE_EG: i32 = -5;
const KING_OPEN_FILE_MG: i32 = -76;
const KING_OPEN_FILE_EG: i32 = -6;
const KING_SEMIOPEN_FILE_MG: i32 = -34;
const KING_SEMIOPEN_FILE_EG: i32 = 10;
const KING_CLOSED_FILE_MG: i32 = 16;
const KING_CLOSED_FILE_EG: i32 = -14;
const PAWN_MAJORITY_MG: i32 = -2;
const PAWN_MAJORITY_EG: i32 = 32;

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
                            if (neighboring_files & their_pawns).0.count_ones()
                                <= (neighboring_files & our_pawns).0.count_ones()
                            {
                                mg += Score(PAWN_MAJORITY_MG);
                                eg += Score(PAWN_MAJORITY_EG);
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
