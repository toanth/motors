use strum::IntoEnumIterator;

use gears::games::{Board, Color, DimT};
use gears::games::chess::Chessboard;
use gears::games::chess::pieces::UncoloredChessPiece;
use gears::games::chess::pieces::UncoloredChessPiece::{Pawn, Rook};
use gears::games::chess::squares::ChessSquare;
use gears::general::bitboards::chess::ChessBitboard;
use gears::general::bitboards::RawBitboard;
use gears::games::Color::{Black, White};
use gears::general::bitboards::{A_FILE, Bitboard};
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
                201, 192, 185, 216, 189, 167, 58, 69,
                83, 82, 124, 133, 134, 157, 121, 94,
                63, 69, 82, 89, 106, 103, 85, 82,
                50, 58, 69, 87, 84, 84, 71, 67,
                48, 54, 64, 64, 76, 77, 86, 74,
                47, 51, 59, 49, 66, 92, 95, 65,
                0, 0, 0, 0, 0, 0, 0, 0,
        ],
        // pawn eg
        [
                0, 0, 0, 0, 0, 0, 0, 0,
                255, 253, 253, 207, 210, 219, 267, 271,
                213, 215, 190, 170, 162, 145, 187, 189,
                152, 141, 125, 116, 107, 107, 124, 125,
                131, 124, 111, 107, 105, 104, 112, 110,
                125, 121, 111, 121, 114, 110, 109, 107,
                130, 126, 120, 123, 126, 114, 109, 109,
                0, 0, 0, 0, 0, 0, 0, 0,
        ],
        // knight mg
        [
                152, 196, 248, 278, 325, 230, 226, 195,
                284, 305, 342, 359, 344, 399, 310, 325,
                306, 346, 370, 381, 416, 423, 370, 337,
                310, 326, 351, 376, 355, 380, 334, 347,
                298, 314, 332, 332, 342, 336, 336, 310,
                278, 303, 316, 323, 335, 321, 327, 295,
                265, 276, 296, 307, 308, 313, 301, 295,
                230, 272, 263, 277, 283, 299, 277, 253,
        ],
        // knight eg
        [
                265, 324, 341, 331, 328, 321, 324, 244,
                312, 335, 338, 340, 335, 318, 326, 297,
                326, 340, 354, 357, 342, 338, 332, 317,
                336, 355, 367, 369, 371, 364, 356, 327,
                336, 348, 368, 372, 373, 363, 348, 328,
                320, 340, 349, 361, 360, 347, 335, 323,
                311, 330, 337, 340, 339, 333, 322, 321,
                298, 291, 321, 325, 326, 313, 300, 291,
        ],
        // bishop mg
        [
                310, 292, 297, 269, 273, 280, 322, 291,
                327, 354, 353, 337, 362, 360, 347, 330,
                346, 371, 370, 393, 386, 408, 388, 375,
                337, 357, 374, 390, 384, 379, 356, 342,
                335, 348, 356, 377, 374, 355, 350, 344,
                342, 354, 353, 357, 359, 354, 356, 357,
                347, 347, 361, 338, 347, 360, 367, 352,
                322, 349, 328, 320, 324, 325, 349, 333,
        ],
        // bishop eg
        [
                348, 359, 356, 368, 366, 357, 351, 346,
                337, 353, 358, 362, 353, 353, 356, 339,
                360, 358, 367, 357, 364, 365, 358, 355,
                358, 374, 367, 377, 374, 372, 372, 359,
                353, 370, 376, 374, 373, 373, 367, 345,
                352, 362, 369, 369, 375, 369, 354, 344,
                346, 347, 346, 361, 363, 350, 350, 328,
                334, 344, 332, 352, 347, 349, 330, 323,
        ],
        // rook mg
        [
                447, 438, 445, 451, 463, 472, 480, 491,
                424, 424, 445, 463, 449, 483, 476, 497,
                414, 433, 433, 433, 465, 477, 510, 478,
                404, 412, 414, 425, 429, 439, 445, 446,
                395, 395, 395, 408, 409, 405, 429, 423,
                392, 394, 394, 400, 409, 415, 452, 432,
                393, 397, 403, 405, 412, 424, 436, 406,
                416, 410, 408, 416, 424, 429, 429, 420,
        ],
        // rook eg
        [
                628, 637, 639, 633, 630, 628, 626, 621,
                635, 642, 641, 631, 634, 622, 621, 608,
                633, 632, 632, 628, 617, 613, 607, 608,
                633, 632, 635, 628, 620, 618, 616, 609,
                624, 626, 627, 623, 620, 622, 611, 604,
                615, 617, 615, 614, 610, 607, 588, 588,
                610, 613, 615, 612, 605, 601, 593, 597,
                615, 615, 622, 615, 607, 610, 606, 603,
        ],
        // queen mg
        [
                852, 872, 896, 929, 927, 942, 973, 907,
                884, 865, 878, 873, 880, 917, 902, 947,
                892, 889, 893, 907, 922, 960, 964, 952,
                880, 886, 890, 892, 896, 907, 906, 915,
                886, 883, 885, 892, 894, 892, 906, 906,
                881, 893, 889, 889, 894, 899, 912, 904,
                880, 890, 901, 900, 899, 909, 915, 916,
                881, 873, 879, 894, 887, 875, 888, 881,
        ],
        // queen eg
        [
                1191, 1200, 1214, 1201, 1203, 1190, 1146, 1182,
                1159, 1201, 1230, 1248, 1261, 1219, 1201, 1166,
                1161, 1186, 1217, 1223, 1231, 1210, 1174, 1169,
                1170, 1194, 1209, 1230, 1241, 1226, 1213, 1188,
                1159, 1192, 1201, 1221, 1213, 1207, 1188, 1177,
                1150, 1163, 1186, 1183, 1185, 1178, 1158, 1144,
                1145, 1147, 1142, 1153, 1155, 1128, 1103, 1077,
                1136, 1144, 1150, 1139, 1142, 1134, 1114, 1112,
        ],
        // king mg
        [
                63, 16, 56, -45, 3, 6, 39, 149,
                -55, -10, -41, 38, 6, -15, 10, 2,
                -73, 15, -44, -54, -16, 32, 4, -29,
                -57, -57, -76, -101, -99, -76, -88, -104,
                -67, -64, -85, -102, -108, -94, -98, -112,
                -41, -17, -66, -72, -66, -73, -35, -52,
                46, 14, -2, -33, -37, -15, 23, 30,
                35, 67, 43, -50, 13, -38, 45, 43,
        ],
        // king eg
        [
                -102, -41, -39, -2, -18, -6, -9, -99,
                -10, 19, 27, 13, 28, 43, 37, 0,
                5, 26, 41, 51, 48, 46, 48, 20,
                -3, 31, 47, 58, 59, 53, 49, 22,
                -11, 19, 39, 53, 52, 40, 29, 9,
                -16, 2, 22, 33, 31, 23, 5, -8,
                -39, -14, -3, 8, 9, -2, -18, -34,
                -67, -59, -42, -15, -43, -21, -49, -76,
        ],
];
const ROOK_OPEN_FILE_MG: i32 = 32;
const ROOK_OPEN_FILE_EG: i32 = 9;
const ROOK_SEMIOPEN_FILE_MG: i32 = 10;
const ROOK_SEMIOPEN_FILE_EG: i32 = 6;
const ROOK_CLOSED_FILE_MG: i32 = -15;
const ROOK_CLOSED_FILE_EG: i32 = -8;
const KING_OPEN_FILE_MG: i32 = -71;
const KING_OPEN_FILE_EG: i32 = -7;
const KING_SEMIOPEN_FILE_MG: i32 = -25;
const KING_SEMIOPEN_FILE_EG: i32 = 8;
const KING_CLOSED_FILE_MG: i32 = 20;
const KING_CLOSED_FILE_EG: i32 = -18;
const ISOLATED_PAWN_MG: i32 = -18;
const ISOLATED_PAWN_EG: i32 = -7;

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

// TODO: Remove here, turn into trait method (needs size, so should go into SizedBitboard)
fn print_bitboard(bb: ChessBitboard) -> String {
    let mut res = String::new();
    for rank in 7..=0 {
        for file in 0..8 {
            let bit = bb & (1 << (8 * rank + file));
            if bit == 0 {
                res.push('0');
            } else {
                res.push('1');
            }
        }
        res.push('\n');
    }
    res
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
                            A_FILE << idx
                        } else {
                            A_FILE >> (idx ^ 0b111_000)
                        };
                        let blocking = blocking.west() | blocking | blocking.east();
                        let blocking = blocking & their_pawns;
                        if blocking.is_zero() {
                            println!("{}", print_bitboard(blocking));
                            println!("{color} passed pawn on square {}", ChessSquare::new(idx));
                        }
                    }
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
