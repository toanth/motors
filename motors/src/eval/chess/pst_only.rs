use strum::IntoEnumIterator;
use crate::eval::Eval;
use gears::games::chess::pieces::UncoloredChessPiece;
use gears::games::chess::Chessboard;
use gears::games::{Board, Color};
use gears::general::bitboards::Bitboard;
use gears::search::Score;

#[derive(Default, Debug)]
pub struct PstOnlyEval {}

/// Psqt values tuned on a mix of the lichess-big3-resolved and zurichess datasets using this tuner: https://github.com/GediminasMasaitis/texel-tuner,
/// originally for King Gᴀᴍʙᴏᴛ, my 1024 token chess coding challenge submission
const PSQTS: [[i32; 64]; 12] = [
    // pawn mg
    [
        77, 77, 77, 77, 77, 77, 77, 77, 139, 154, 139, 166, 158, 142, 76, 46, 61, 78, 110, 114,
        119, 144, 119, 77, 49, 74, 77, 80, 101, 92, 98, 71, 37, 67, 64, 82, 82, 73, 84, 54, 36, 62,
        63, 63, 78, 66, 100, 67, 36, 65, 57, 50, 69, 86, 111, 62, 77, 77, 77, 77, 77, 77, 77, 77,
    ],
    // pawn eg
    [
        109, 109, 109, 109, 109, 109, 109, 109, 289, 283, 277, 232, 225, 238, 285, 296, 225, 230,
        199, 178, 168, 154, 199, 199, 154, 143, 124, 115, 106, 109, 127, 128, 130, 126, 109, 105,
        103, 105, 115, 111, 123, 124, 107, 119, 112, 110, 114, 105, 129, 128, 117, 121, 123, 114,
        112, 106, 109, 109, 109, 109, 109, 109, 109, 109,
    ],
    // knight mg
    [
        138, 199, 261, 283, 330, 243, 205, 205, 278, 305, 350, 348, 350, 399, 312, 324, 307, 342,
        357, 374, 411, 416, 368, 333, 306, 318, 341, 364, 343, 370, 324, 340, 291, 306, 321, 322,
        331, 328, 326, 298, 273, 295, 311, 313, 323, 313, 317, 287, 260, 271, 289, 300, 301, 304,
        291, 288, 217, 273, 255, 271, 274, 287, 274, 242,
    ],
    // knight eg
    [
        263, 304, 320, 316, 312, 299, 313, 229, 304, 323, 324, 330, 317, 306, 315, 287, 317, 330,
        350, 349, 331, 328, 320, 308, 328, 350, 361, 363, 365, 357, 350, 319, 330, 340, 363, 363,
        366, 355, 340, 323, 314, 333, 340, 356, 355, 338, 327, 315, 305, 320, 329, 334, 333, 328,
        312, 314, 296, 286, 316, 320, 319, 310, 290, 290,
    ],
    // bishop mg
    [
        319, 295, 309, 269, 265, 295, 324, 283, 326, 357, 344, 332, 360, 368, 352, 347, 338, 364,
        373, 388, 379, 405, 384, 372, 329, 343, 368, 378, 374, 369, 347, 332, 325, 341, 346, 367,
        365, 347, 340, 331, 339, 345, 345, 347, 349, 344, 345, 350, 339, 344, 351, 331, 338, 352,
        360, 344, 318, 336, 324, 313, 316, 314, 341, 326,
    ],
    // bishop eg
    [
        337, 346, 344, 359, 355, 345, 337, 340, 327, 344, 349, 350, 344, 339, 349, 320, 354, 349,
        357, 349, 352, 355, 347, 345, 351, 367, 360, 373, 367, 362, 362, 348, 345, 362, 370, 366,
        366, 366, 358, 336, 343, 355, 362, 363, 367, 362, 345, 334, 339, 338, 338, 353, 355, 342,
        343, 320, 323, 339, 320, 344, 340, 339, 324, 315,
    ],
    // rook mg
    [
        467, 460, 472, 475, 495, 515, 492, 506, 445, 442, 466, 488, 472, 504, 493, 518, 425, 446,
        447, 454, 481, 483, 522, 497, 407, 422, 427, 436, 439, 440, 452, 454, 390, 392, 401, 415,
        415, 400, 426, 415, 382, 393, 402, 403, 407, 404, 439, 417, 379, 393, 406, 404, 408, 411,
        429, 395, 399, 401, 412, 417, 421, 410, 422, 404,
    ],
    // rook eg
    [
        623, 628, 634, 631, 621, 610, 613, 611, 623, 635, 636, 627, 627, 613, 609, 597, 623, 625,
        626, 624, 611, 606, 597, 593, 625, 623, 631, 627, 615, 610, 604, 598, 618, 623, 626, 624,
        619, 618, 604, 599, 614, 614, 614, 617, 614, 606, 586, 587, 610, 613, 614, 616, 608, 604,
        594, 602, 606, 615, 621, 621, 612, 608, 605, 592,
    ],
    // queen mg
    [
        848, 855, 889, 922, 925, 931, 935, 878, 878, 860, 865, 855, 861, 910, 888, 937, 879, 879,
        883, 898, 904, 946, 950, 943, 864, 868, 875, 873, 876, 888, 887, 895, 864, 867, 864, 871,
        873, 872, 884, 884, 864, 874, 868, 869, 871, 879, 890, 882, 862, 870, 880, 880, 879, 889,
        894, 901, 863, 852, 860, 877, 865, 850, 867, 866,
    ],
    // queen eg
    [
        1160, 1176, 1188, 1177, 1171, 1165, 1127, 1166, 1133, 1169, 1204, 1226, 1240, 1201, 1183,
        1144, 1139, 1157, 1194, 1195, 1215, 1188, 1152, 1139, 1149, 1175, 1185, 1211, 1224, 1211,
        1196, 1172, 1152, 1174, 1185, 1209, 1201, 1192, 1173, 1160, 1134, 1147, 1176, 1171, 1176,
        1166, 1147, 1134, 1134, 1136, 1132, 1141, 1144, 1115, 1089, 1066, 1126, 1131, 1133, 1118,
        1131, 1129, 1106, 1101,
    ],
    // king mg
    [
        44, 15, 47, -48, -3, 20, 43, 104, -61, -8, -47, 22, -8, 10, 0, -26, -78, 28, -51, -58, -40,
        34, 20, -34, -62, -69, -76, -121, -116, -82, -77, -108, -57, -64, -95, -129, -127, -97,
        -94, -123, -23, -9, -69, -82, -77, -73, -27, -45, 59, 18, 1, -38, -38, -16, 34, 43, 50, 78,
        51, -53, 15, -27, 58, 59,
    ],
    // king eg
    [
        -94, -44, -37, -1, -13, -6, -10, -86, -10, 16, 25, 13, 28, 39, 38, 11, 4, 21, 39, 46, 48,
        43, 44, 19, -6, 27, 43, 56, 56, 52, 44, 22, -18, 14, 38, 54, 54, 43, 30, 14, -24, 0, 22,
        34, 35, 27, 8, -5, -43, -15, -1, 11, 14, 4, -15, -34, -74, -58, -38, -18, -45, -20, -48,
        -77,
    ],
];

const PIECE_PHASE: [i32; 6] = [0, 1, 1, 2, 4, 0];

impl Eval<Chessboard> for PstOnlyEval {
    fn eval(&self, pos: Chessboard) -> Score {
        let mut mg = Score(0);
        let mut eg = Score(0);
        let mut phase = 0;
        for color in Color::iter() {
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