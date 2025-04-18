use std::fmt::Display;

use gears::games::Color;
use gears::games::chess::pieces::ChessPieceType;
use gears::games::chess::{ChessColor, Chessboard};
use gears::general::bitboards::RawBitboard;
use gears::general::board::{BitboardBoard, Board};
use gears::general::common::StaticallyNamedEntity;
use gears::score::{PhasedScore, Score, ScoreT};

use crate::eval::Eval;

#[derive(Default, Debug, Clone)]
pub struct PistonEval {}

/// Psqt values tuned on a combination of the zurichess and a dataset used by 4ku,
/// created by GCP using his engine Stoofvlees and filtered by cj5716 using Stockfish at depth 9,
/// using this tuner: <https://github.com/GediminasMasaitis/texel-tuner>.
#[rustfmt::skip]
const PSQTS: [[ScoreT; 64]; 12] = [
    // pawn mg
    [
        0, 0, 0, 0, 0, 0, 0, 0,
        183, 194, 176, 200, 180, 165, 83, 72,
        68, 88, 119, 124, 128, 150, 135, 88,
        51, 76, 78, 84, 103, 95, 101, 77,
        40, 68, 67, 83, 84, 77, 87, 62,
        38, 64, 62, 64, 79, 70, 103, 70,
        38, 64, 59, 49, 69, 87, 113, 62,
        0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // pawn eg
    [
        0, 0, 0, 0, 0, 0, 0, 0,
        244, 244, 245, 201, 203, 210, 255, 258,
        207, 209, 183, 163, 156, 141, 181, 182,
        147, 138, 120, 111, 103, 105, 123, 122,
        127, 124, 107, 103, 102, 103, 113, 107,
        122, 122, 106, 118, 110, 107, 111, 104,
        127, 127, 115, 122, 124, 113, 111, 106,
        0, 0, 0, 0, 0, 0, 0, 0,
    ],
    // knight mg
    [
        141, 191, 245, 279, 320, 232, 222, 201,
        285, 303, 335, 351, 336, 399, 301, 323,
        300, 339, 360, 371, 409, 410, 364, 328,
        300, 315, 341, 364, 343, 368, 323, 335,
        287, 304, 319, 319, 329, 324, 323, 297,
        267, 291, 305, 311, 322, 309, 313, 282,
        251, 265, 283, 295, 296, 300, 285, 280,
        213, 263, 249, 264, 269, 283, 266, 238,
    ],
    // knight eg
    [
        266, 323, 340, 329, 327, 320, 322, 240,
        312, 335, 338, 339, 334, 316, 329, 299,
        326, 339, 353, 357, 341, 339, 332, 318,
        336, 355, 367, 367, 371, 364, 356, 328,
        336, 347, 368, 371, 372, 362, 348, 327,
        319, 338, 349, 360, 359, 345, 333, 321,
        310, 329, 336, 339, 338, 332, 321, 320,
        298, 289, 320, 324, 325, 312, 297, 289,
    ],
    // bishop mg
    [
        303, 285, 292, 263, 268, 275, 321, 286,
        325, 349, 347, 327, 355, 358, 347, 334,
        335, 363, 360, 386, 375, 402, 381, 367,
        329, 344, 367, 379, 374, 369, 344, 329,
        322, 337, 343, 364, 361, 344, 337, 330,
        332, 341, 340, 344, 346, 340, 342, 344,
        335, 335, 347, 326, 333, 347, 353, 339,
        309, 332, 316, 305, 311, 312, 337, 317,
    ],
    // bishop eg
    [
        346, 357, 354, 366, 363, 354, 347, 344,
        335, 352, 356, 360, 351, 349, 354, 333,
        359, 356, 365, 355, 362, 362, 354, 352,
        356, 372, 365, 375, 372, 370, 371, 357,
        352, 367, 374, 372, 371, 372, 366, 344,
        350, 360, 368, 366, 372, 367, 352, 342,
        344, 346, 345, 360, 361, 348, 349, 326,
        332, 342, 328, 351, 346, 346, 329, 324,
    ],
    // rook mg
    [
        469, 455, 463, 471, 486, 489, 480, 507,
        443, 444, 465, 485, 470, 502, 485, 514,
        422, 441, 447, 452, 482, 480, 515, 486,
        402, 413, 421, 434, 440, 437, 443, 445,
        385, 391, 397, 413, 412, 396, 416, 409,
        378, 388, 396, 399, 404, 400, 435, 411,
        376, 390, 404, 403, 408, 409, 424, 392,
        396, 398, 408, 414, 417, 407, 421, 396,
    ],
    // rook eg
    [
        624, 636, 641, 636, 630, 625, 626, 616,
        634, 642, 644, 635, 636, 621, 621, 605,
        634, 635, 636, 632, 619, 616, 607, 606,
        636, 635, 640, 634, 623, 620, 616, 609,
        627, 630, 633, 630, 625, 625, 613, 607,
        619, 621, 620, 622, 618, 612, 592, 593,
        615, 616, 618, 618, 611, 607, 598, 604,
        610, 618, 626, 624, 616, 612, 608, 603,
    ],
    // queen mg
    [
        841, 857, 884, 916, 912, 928, 946, 897,
        874, 854, 864, 859, 866, 906, 879, 928,
        877, 874, 877, 892, 904, 945, 944, 933,
        863, 867, 872, 874, 877, 886, 885, 895,
        866, 863, 865, 872, 873, 871, 882, 885,
        861, 872, 868, 867, 872, 878, 889, 883,
        861, 869, 879, 879, 877, 886, 893, 900,
        861, 850, 857, 874, 865, 851, 868, 861,
    ],
    // queen eg
    [
        1177, 1186, 1201, 1187, 1190, 1177, 1138, 1165,
        1145, 1188, 1218, 1233, 1246, 1206, 1195, 1158,
        1149, 1173, 1205, 1209, 1218, 1198, 1164, 1157,
        1158, 1183, 1198, 1217, 1229, 1218, 1203, 1177,
        1151, 1183, 1189, 1210, 1204, 1198, 1179, 1165,
        1142, 1154, 1177, 1174, 1176, 1169, 1146, 1133,
        1136, 1139, 1134, 1143, 1145, 1119, 1092, 1063,
        1126, 1134, 1139, 1125, 1131, 1126, 1104, 1104,
    ],
    // king mg
    [
        54, 10, 39, -59, -2, 21, 54, 156,
        -62, -24, -57, 19, -5, -13, 14, 2,
        -82, 7, -57, -73, -28, 33, 6, -25,
        -60, -68, -87, -117, -110, -73, -81, -101,
        -69, -71, -96, -121, -117, -91, -93, -105,
        -38, -18, -69, -79, -72, -69, -29, -45,
        53, 13, -2, -35, -35, -17, 34, 42,
        47, 79, 50, -58, 15, -31, 59, 57,
    ],
    // king eg
    [
        -99, -42, -38, -2, -18, -7, -10, -99,
        -7, 20, 27, 11, 28, 43, 38, 2,
        8, 26, 40, 50, 47, 46, 47, 19,
        -1, 31, 47, 56, 58, 52, 47, 21,
        -10, 19, 39, 53, 52, 41, 29, 10,
        -14, 3, 22, 32, 31, 23, 5, -7,
        -38, -13, -3, 7, 9, 1, -19, -36,
        -67, -58, -38, -18, -45, -22, -49, -77,
    ],
];

const PIECE_PHASE: [isize; 6] = [0, 1, 1, 2, 4, 0];

impl StaticallyNamedEntity for PistonEval {
    fn static_short_name() -> impl Display
    where
        Self: Sized,
    {
        "PiSTOn"
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        "PiSTOn: Piece Square Table Only Chess Eval".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "A chess evaluation function using only tapered piece square tables".to_string()
    }
}

impl Eval<Chessboard> for PistonEval {
    fn eval(&mut self, pos: &Chessboard, _ply: usize, _engine: ChessColor) -> Score {
        let mut mg = Score(0);
        let mut eg = Score(0);
        let mut phase = 0;
        for color in ChessColor::iter() {
            for piece in ChessPieceType::pieces() {
                let mut bb = pos.col_piece_bb(color, piece);
                while bb.has_set_bit() {
                    let idx = bb.pop_lsb();
                    let mg_table = piece as usize * 2;
                    let eg_table = mg_table + 1;
                    let square = match color {
                        ChessColor::White => idx ^ 0b111_000,
                        ChessColor::Black => idx,
                    };
                    mg += Score(PSQTS[mg_table][square]);
                    eg += Score(PSQTS[eg_table][square]);
                    phase += PIECE_PHASE[piece as usize];
                }
            }
            mg = -mg;
            eg = -eg;
        }
        // TODO: Store phased scores in the PSQTs.
        let score = PhasedScore::new(mg.0 as i16, eg.0 as i16).taper(phase, 24);
        match pos.active_player() {
            ChessColor::White => score,
            ChessColor::Black => -score,
        }
    }

    fn piece_scale(&self) -> ScoreT {
        5
    }
}
