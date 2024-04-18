use strum::IntoEnumIterator;

use gears::games::{Board, Color, DimT};
use gears::games::chess::Chessboard;
use gears::games::chess::pieces::UncoloredChessPiece;
use gears::games::chess::pieces::UncoloredChessPiece::{Pawn, Rook};
use gears::games::chess::squares::ChessSquare;
use gears::general::bitboards::{Bitboard, ChessBitboard};
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
                186, 192, 181, 208, 186, 166, 63, 52,
                74, 89, 122, 128, 131, 154, 125, 86,
                57, 80, 80, 86, 105, 101, 94, 76,
                45, 72, 68, 85, 83, 82, 81, 62,
                44, 68, 62, 64, 76, 74, 95, 70,
                43, 66, 59, 49, 66, 91, 105, 61,
                0, 0, 0, 0, 0, 0, 0, 0,
        ],
        // pawn eg
        [
                0, 0, 0, 0, 0, 0, 0, 0,
                249, 247, 247, 203, 206, 214, 262, 267,
                208, 210, 185, 166, 158, 142, 184, 184,
                148, 138, 121, 113, 105, 105, 125, 122,
                128, 124, 108, 104, 103, 103, 115, 108,
                123, 122, 107, 119, 112, 108, 114, 104,
                127, 128, 115, 121, 124, 113, 113, 107,
                0, 0, 0, 0, 0, 0, 0, 0,
        ],
        // knight mg
        [
                153, 195, 245, 278, 322, 230, 229, 196,
                284, 307, 340, 355, 341, 398, 311, 324,
                303, 343, 366, 378, 412, 419, 366, 333,
                306, 323, 347, 372, 350, 376, 330, 343,
                294, 311, 328, 328, 338, 332, 332, 306,
                275, 299, 312, 319, 331, 317, 322, 291,
                259, 272, 292, 303, 304, 309, 297, 291,
                224, 269, 258, 273, 279, 295, 274, 248,
        ],
        // knight eg
        [
                264, 322, 340, 329, 327, 320, 321, 242,
                312, 334, 337, 338, 333, 317, 326, 298,
                325, 339, 353, 356, 341, 338, 332, 317,
                335, 354, 366, 367, 370, 363, 355, 326,
                335, 347, 367, 371, 372, 362, 347, 327,
                319, 339, 348, 360, 359, 345, 333, 321,
                310, 329, 336, 339, 338, 332, 320, 320,
                298, 291, 320, 325, 325, 312, 299, 289,
        ],
        // bishop mg
        [
                309, 289, 294, 266, 271, 277, 320, 291,
                325, 356, 351, 332, 359, 356, 349, 325,
                339, 366, 366, 388, 381, 403, 382, 369,
                333, 351, 371, 386, 380, 375, 351, 337,
                329, 343, 350, 372, 368, 351, 346, 338,
                337, 349, 348, 352, 354, 349, 351, 353,
                342, 342, 355, 333, 342, 355, 363, 347,
                318, 342, 324, 314, 320, 320, 345, 328,
        ],
        // bishop eg
        [
                346, 356, 354, 366, 363, 354, 348, 344,
                335, 351, 356, 360, 351, 351, 354, 337,
                359, 356, 365, 355, 361, 363, 356, 353,
                356, 372, 365, 375, 372, 370, 370, 356,
                351, 368, 374, 372, 371, 371, 365, 344,
                350, 360, 367, 366, 372, 367, 352, 342,
                344, 345, 344, 359, 360, 348, 348, 326,
                332, 342, 330, 350, 346, 347, 328, 321,
        ],
        // rook mg
        [
                444, 434, 440, 447, 459, 469, 479, 488,
                419, 423, 441, 459, 445, 480, 473, 491,
                405, 427, 428, 428, 460, 471, 502, 471,
                397, 407, 410, 420, 425, 435, 440, 441,
                391, 391, 391, 404, 405, 401, 423, 419,
                388, 389, 390, 396, 404, 411, 446, 427,
                389, 392, 399, 400, 408, 419, 431, 403,
                412, 405, 404, 412, 419, 424, 423, 415,
        ],
        // rook eg
        [
                624, 633, 636, 630, 626, 625, 623, 618,
                633, 639, 639, 628, 632, 620, 620, 606,
                631, 630, 630, 626, 615, 611, 605, 605,
                630, 628, 632, 625, 616, 614, 612, 605,
                620, 623, 624, 620, 616, 618, 607, 600,
                612, 614, 612, 612, 608, 603, 585, 585,
                608, 610, 612, 609, 603, 598, 591, 595,
                613, 611, 620, 612, 605, 608, 602, 601,
        ],
        // queen mg
        [
                845, 861, 887, 919, 917, 931, 963, 900,
                875, 861, 870, 864, 871, 909, 897, 935,
                882, 879, 886, 897, 912, 950, 951, 939,
                870, 876, 881, 882, 886, 897, 896, 905,
                876, 872, 875, 882, 884, 882, 895, 896,
                871, 882, 879, 878, 883, 889, 901, 894,
                870, 880, 890, 889, 888, 898, 904, 906,
                871, 862, 868, 884, 876, 865, 876, 872,
        ],
        // queen eg
        [
                1184, 1194, 1208, 1195, 1198, 1185, 1139, 1174,
                1153, 1192, 1223, 1239, 1254, 1213, 1192, 1163,
                1154, 1179, 1209, 1216, 1224, 1203, 1169, 1164,
                1161, 1186, 1202, 1222, 1234, 1219, 1207, 1181,
                1152, 1186, 1192, 1214, 1205, 1201, 1183, 1170,
                1146, 1158, 1180, 1177, 1179, 1172, 1152, 1140,
                1141, 1142, 1137, 1146, 1149, 1123, 1099, 1074,
                1131, 1139, 1145, 1134, 1137, 1130, 1110, 1108,
        ],
        // king mg
        [
                60, 17, 60, -45, 5, 3, 37, 150,
                -58, -11, -39, 41, 10, -14, 10, 2,
                -76, 15, -41, -52, -17, 30, 2, -29,
                -59, -60, -76, -99, -98, -76, -89, -103,
                -69, -65, -86, -102, -107, -94, -97, -112,
                -42, -19, -66, -71, -65, -73, -34, -51,
                45, 13, -2, -33, -37, -14, 24, 31,
                33, 66, 43, -50, 14, -38, 46, 42,
        ],
        // king eg
        [
                -102, -41, -39, -1, -19, -6, -10, -101,
                -10, 19, 27, 12, 27, 42, 37, 0,
                5, 26, 41, 51, 47, 45, 47, 18,
                -3, 31, 48, 58, 59, 52, 48, 20,
                -11, 19, 40, 54, 52, 40, 29, 10,
                -15, 4, 23, 33, 31, 22, 5, -8,
                -38, -13, -2, 9, 9, -2, -18, -35,
                -66, -58, -41, -14, -42, -21, -49, -77,
        ],
];
const ROOK_OPEN_FILE_MG: i32 = 31;
const ROOK_OPEN_FILE_EG: i32 = 9;
const ROOK_SEMIOPEN_FILE_MG: i32 = 7;
const ROOK_SEMIOPEN_FILE_EG: i32 = 7;
const ROOK_CLOSED_FILE_MG: i32 = -16;
const ROOK_CLOSED_FILE_EG: i32 = -7;
const KING_OPEN_FILE_MG: i32 = -77;
const KING_OPEN_FILE_EG: i32 = -8;
const KING_SEMIOPEN_FILE_MG: i32 = -34;
const KING_SEMIOPEN_FILE_EG: i32 = 6;
const KING_CLOSED_FILE_MG: i32 = 16;
const KING_CLOSED_FILE_EG: i32 = -17;

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
                        Color::White => idx ^ 0b111_000,
                        Color::Black => idx,
                    };
                    mg += Score(PSQTS[mg_table][square]);
                    eg += Score(PSQTS[eg_table][square]);
                    phase += PIECE_PHASE[piece as usize];
                }
            }
            mg = -mg;
            eg = -eg;
        }
        let score = (mg * phase + eg * (24 - phase)) / 24;
        match pos.active_player() {
            Color::White => score,
            Color::Black => -score,
        }
    }
}
