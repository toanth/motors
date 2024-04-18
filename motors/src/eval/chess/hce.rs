use strum::IntoEnumIterator;

use gears::games::{Board, Color};
use gears::games::chess::Chessboard;
use gears::games::chess::pieces::UncoloredChessPiece;
use gears::games::chess::pieces::UncoloredChessPiece::{Pawn, Rook};
use gears::games::chess::squares::ChessSquare;
use gears::general::bitboards::{Bitboard, ChessBitboard};
use gears::search::Score;

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
                188, 195, 181, 207, 184, 167, 77, 70,
                75, 90, 123, 128, 133, 156, 135, 93,
                57, 80, 81, 86, 107, 101, 103, 82,
                45, 71, 68, 85, 86, 83, 91, 67,
                44, 67, 62, 64, 80, 75, 106, 75,
                43, 66, 59, 49, 70, 92, 116, 66,
                0, 0, 0, 0, 0, 0, 0, 0,
        ],
        // pawn eg
        [
                0, 0, 0, 0, 0, 0, 0, 0,
                248, 246, 247, 202, 205, 212, 258, 262,
                207, 210, 184, 165, 157, 141, 182, 182,
                148, 139, 121, 112, 104, 104, 123, 122,
                128, 124, 108, 104, 102, 103, 113, 108,
                123, 123, 108, 119, 111, 107, 112, 104,
                128, 128, 116, 121, 123, 113, 111, 106,
                0, 0, 0, 0, 0, 0, 0, 0,
        ],
        // knight mg
        [
                149, 190, 241, 275, 319, 230, 223, 199,
                280, 303, 335, 351, 335, 394, 303, 320,
                298, 338, 361, 372, 408, 412, 362, 331,
                301, 318, 343, 367, 346, 371, 326, 339,
                289, 306, 323, 323, 333, 327, 327, 300,
                270, 295, 308, 314, 326, 312, 317, 287,
                255, 268, 287, 299, 300, 304, 290, 285,
                218, 265, 253, 268, 273, 291, 270, 242,
        ],
        // knight eg
        [
                265, 323, 340, 330, 328, 320, 322, 241,
                313, 334, 338, 339, 334, 317, 328, 299,
                326, 339, 353, 356, 341, 339, 332, 317,
                335, 354, 367, 367, 370, 364, 355, 327,
                336, 347, 368, 371, 372, 362, 348, 327,
                320, 339, 349, 360, 359, 345, 333, 321,
                311, 330, 336, 339, 339, 333, 322, 320,
                298, 292, 320, 325, 326, 312, 299, 290,
        ],
        // bishop mg
        [
                308, 286, 291, 262, 266, 275, 319, 290,
                321, 352, 346, 328, 354, 354, 348, 334,
                335, 362, 362, 384, 376, 401, 381, 367,
                329, 346, 367, 381, 376, 371, 347, 332,
                324, 339, 345, 367, 363, 347, 340, 334,
                333, 344, 344, 347, 349, 344, 345, 347,
                337, 338, 350, 329, 337, 351, 357, 342,
                314, 337, 320, 309, 315, 316, 341, 323,
        ],
        // bishop eg
        [
                345, 357, 354, 366, 364, 355, 348, 344,
                336, 351, 356, 360, 351, 351, 354, 333,
                359, 356, 365, 356, 362, 363, 355, 353,
                356, 372, 365, 375, 372, 370, 371, 357,
                352, 368, 375, 372, 372, 372, 366, 344,
                350, 360, 368, 367, 373, 367, 352, 343,
                344, 346, 345, 359, 361, 348, 349, 326,
                332, 342, 330, 351, 347, 347, 329, 323,
        ],
        // rook mg
        [
                439, 427, 433, 439, 454, 463, 466, 482,
                412, 416, 434, 452, 437, 469, 460, 484,
                399, 421, 421, 421, 454, 463, 500, 468,
                391, 400, 402, 412, 420, 429, 437, 439,
                385, 384, 384, 396, 399, 394, 419, 414,
                383, 383, 384, 388, 399, 404, 441, 422,
                384, 386, 392, 393, 402, 412, 424, 398,
                407, 399, 397, 405, 413, 419, 423, 409,
        ],
        // rook eg
        [
                624, 634, 638, 631, 627, 626, 625, 618,
                634, 640, 640, 629, 633, 622, 622, 607,
                632, 631, 631, 627, 615, 612, 604, 604,
                631, 629, 633, 626, 616, 614, 611, 604,
                620, 623, 625, 621, 616, 618, 606, 599,
                612, 614, 612, 612, 607, 603, 584, 584,
                607, 610, 612, 609, 602, 598, 591, 596,
                612, 611, 620, 613, 604, 607, 600, 601,
        ],
        // queen mg
        [
                841, 857, 882, 915, 914, 928, 951, 897,
                868, 855, 863, 859, 864, 901, 880, 926,
                877, 874, 880, 891, 907, 947, 946, 936,
                865, 870, 874, 877, 880, 891, 890, 900,
                870, 867, 869, 876, 877, 876, 888, 889,
                865, 877, 873, 873, 877, 883, 894, 888,
                865, 874, 884, 884, 883, 892, 899, 902,
                866, 857, 863, 879, 870, 859, 875, 868,
        ],
        // queen eg
        [
                1183, 1192, 1208, 1193, 1195, 1182, 1141, 1171,
                1154, 1191, 1223, 1238, 1253, 1213, 1199, 1166,
                1153, 1178, 1209, 1215, 1223, 1202, 1167, 1160,
                1161, 1186, 1202, 1221, 1234, 1220, 1205, 1179,
                1152, 1185, 1192, 1212, 1205, 1199, 1181, 1168,
                1145, 1156, 1178, 1175, 1178, 1171, 1150, 1135,
                1140, 1141, 1135, 1145, 1148, 1121, 1095, 1069,
                1130, 1138, 1143, 1133, 1136, 1128, 1107, 1106,
        ],
        // king mg
        [
                55, 8, 41, -60, -1, 23, 56, 159,
                -63, -22, -56, 21, -4, -13, 13, 3,
                -81, 7, -57, -73, -29, 31, 5, -25,
                -59, -68, -88, -118, -111, -74, -82, -101,
                -67, -69, -96, -123, -119, -92, -93, -106,
                -36, -14, -68, -80, -72, -69, -29, -46,
                57, 16, 0, -35, -36, -17, 32, 40,
                47, 80, 51, -56, 13, -29, 57, 55,
        ],
        // king eg
        [
                -99, -42, -39, -3, -18, -7, -10, -99,
                -8, 19, 27, 11, 28, 43, 38, 2,
                7, 25, 40, 50, 47, 46, 48, 19,
                -2, 31, 46, 56, 59, 53, 48, 22,
                -10, 19, 39, 53, 52, 41, 30, 10,
                -15, 3, 22, 32, 31, 23, 6, -6,
                -38, -13, -3, 7, 9, 1, -17, -34,
                -67, -59, -40, -18, -43, -22, -49, -77,
        ],
];
const ROOK_OPEN_FILE_MG: i32 = 32;
const ROOK_OPEN_FILE_EG: i32 = 8;
const ROOK_SEMIOPEN_FILE_MG: i32 = 8;
const ROOK_SEMIOPEN_FILE_EG: i32 = 7;
const ROOK_CLOSED_FILE_MG: i32 = -18;
const ROOK_CLOSED_FILE_EG: i32 = -6;

// TODO: Differentiate between rooks in front of / behind pawns, also handle kings.

const PIECE_PHASE: [i32; 6] = [0, 1, 1, 2, 4, 0];

impl Eval<Chessboard> for HandCraftedEval {
    fn eval(&self, pos: Chessboard) -> Score {
        let mut mg = Score(0);
        let mut eg = Score(0);
        let mut phase = 0;
        for color in Color::iter() {
            let pawns = [
                pos.colored_piece_bb(color, Pawn),
                pos.colored_piece_bb(color.other(), Pawn),
            ];
            let mut rooks = pos.colored_piece_bb(color, Rook);

            while rooks.has_set_bit() {
                let idx = rooks.pop_lsb();
                let file = ChessBitboard::file_no(ChessSquare::new(idx).file());
                if (file & pawns[0]).is_zero() && (file & pawns[1]).is_zero() {
                    mg += Score(ROOK_OPEN_FILE_MG);
                    eg += Score(ROOK_OPEN_FILE_EG);
                } else if (file & pawns[0]).is_zero() {
                    mg += Score(ROOK_SEMIOPEN_FILE_MG);
                    eg += Score(ROOK_SEMIOPEN_FILE_EG);
                } else if (file & pawns[0]).has_set_bit() && (file & pawns[1]).has_set_bit() {
                    mg += Score(ROOK_CLOSED_FILE_MG);
                    eg += Score(ROOK_CLOSED_FILE_EG);
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
