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
const PSQTS: [[i32; NUM_SQUARES]; NUM_CHESS_PIECES * 2] = [
    // pawn mg
    [
        14, 14, 14, 15, 14, 13, 7, 4,
        57, 59, 60, 67, 65, 64, 43, 31,
        62, 69, 79, 85, 90, 107, 98, 77,
        51, 64, 69, 73, 84, 89, 87, 74,
        43, 58, 60, 66, 71, 72, 73, 63,
        41, 55, 55, 55, 62, 66, 77, 64,
        33, 44, 43, 38, 45, 57, 68, 51,
        7, 10, 10, 8, 10, 13, 16, 12,
    ],
    // pawn eg
    [
        20, 19, 19, 17, 16, 17, 19, 21,
        85, 84, 81, 70, 69, 70, 82, 87,
        102, 101, 90, 84, 79, 76, 92, 94,
        96, 95, 84, 76, 74, 73, 83, 81,
        89, 90, 81, 77, 76, 76, 81, 76,
        87, 88, 81, 82, 83, 80, 80, 74,
        72, 73, 67, 68, 72, 67, 65, 60,
        17, 17, 16, 16, 17, 16, 15, 14,
    ],
    // knight mg
    [
        140, 165, 200, 227, 253, 218, 200, 179,
        209, 226, 253, 271, 276, 293, 254, 241,
        239, 259, 279, 293, 306, 320, 289, 266,
        241, 254, 272, 287, 286, 297, 276, 267,
        232, 243, 258, 265, 269, 269, 265, 250,
        219, 232, 245, 252, 259, 255, 254, 238,
        203, 215, 228, 237, 241, 244, 238, 229,
        183, 207, 210, 219, 225, 234, 225, 208,
    ],
    // knight eg
    [
        224, 254, 274, 271, 268, 263, 260, 220,
        251, 265, 275, 277, 273, 264, 266, 247,
        265, 274, 284, 288, 283, 277, 274, 262,
        272, 282, 293, 297, 298, 293, 287, 269,
        272, 280, 293, 299, 301, 294, 284, 270,
        263, 273, 283, 291, 291, 284, 274, 265,
        255, 263, 271, 276, 277, 272, 262, 258,
        247, 245, 260, 266, 266, 259, 250, 244,
    ],
    // bishop mg
    [
        248, 240, 241, 226, 228, 232, 253, 242,
        259, 271, 273, 265, 273, 277, 278, 264,
        267, 283, 289, 298, 299, 308, 298, 285,
        266, 277, 289, 302, 302, 299, 287, 275,
        264, 272, 280, 292, 293, 285, 278, 272,
        268, 274, 278, 280, 282, 280, 280, 278,
        268, 272, 277, 268, 270, 275, 282, 276,
        257, 269, 265, 256, 257, 259, 273, 268,
    ],
    // bishop eg
    [
        280, 288, 289, 294, 293, 289, 286, 281,
        280, 286, 290, 292, 289, 288, 288, 281,
        288, 290, 294, 293, 294, 295, 292, 287,
        290, 297, 298, 301, 301, 300, 299, 290,
        288, 296, 301, 302, 303, 302, 297, 285,
        286, 291, 296, 298, 301, 297, 290, 280,
        281, 282, 283, 289, 292, 287, 283, 271,
        273, 278, 273, 283, 284, 283, 274, 264,
    ],
    // rook mg
    [
        349, 344, 347, 354, 361, 370, 377, 385,
        334, 336, 346, 355, 356, 373, 380, 386,
        321, 332, 338, 342, 355, 367, 383, 375,
        314, 321, 325, 330, 337, 345, 353, 353,
        310, 311, 312, 319, 323, 324, 338, 338,
        308, 309, 310, 314, 320, 325, 343, 337,
        312, 313, 315, 318, 323, 330, 341, 329,
        323, 320, 320, 324, 330, 334, 337, 330,
    ],
    // rook eg
    [
        507, 512, 516, 512, 509, 508, 506, 501,
        512, 515, 516, 510, 509, 504, 501, 496,
        512, 511, 511, 508, 502, 497, 494, 491,
        510, 509, 509, 506, 500, 497, 495, 490,
        504, 504, 505, 502, 499, 498, 492, 487,
        498, 498, 498, 497, 494, 491, 481, 479,
        495, 495, 496, 495, 491, 487, 481, 482,
        496, 495, 499, 497, 491, 491, 487, 486,
    ],
    // queen mg
    [
        673, 679, 694, 713, 716, 729, 747, 723,
        690, 684, 690, 694, 699, 721, 725, 733,
        696, 693, 696, 701, 709, 731, 736, 735,
        691, 692, 696, 699, 703, 712, 716, 719,
        691, 691, 693, 696, 699, 700, 707, 709,
        689, 695, 696, 697, 699, 703, 710, 710,
        689, 693, 698, 701, 700, 704, 709, 710,
        689, 686, 689, 698, 696, 690, 695, 695,
    ],
    // queen eg
    [
        960, 968, 982, 982, 985, 974, 945, 953,
        945, 964, 985, 997, 1006, 988, 967, 954,
        941, 957, 979, 991, 1000, 989, 968, 956,
        945, 960, 975, 989, 997, 991, 978, 963,
        941, 957, 968, 981, 982, 978, 966, 955,
        935, 943, 954, 958, 960, 954, 939, 927,
        930, 932, 933, 936, 938, 926, 909, 894,
        923, 927, 931, 926, 927, 921, 907, 900,
    ],
    // king mg
    [
        17, 2, 12, -26, -17, -14, 14, 89,
        -47, -23, -26, -12, -12, -15, 1, 14,
        -70, -28, -41, -50, -36, -14, -17, -31,
        -66, -55, -64, -83, -83, -65, -66, -78,
        -67, -61, -75, -90, -93, -85, -81, -90,
        -41, -34, -56, -69, -70, -69, -49, -51,
        7, -1, -15, -42, -45, -38, -11, -1,
        13, 26, 17, -37, -21, -35, 4, 17,
    ],
    // king eg
    [
        -62, -27, -12, 6, 3, 8, 6, -47,
        -13, 8, 21, 22, 26, 32, 30, 1,
        7, 22, 37, 44, 44, 44, 42, 22,
        5, 24, 41, 51, 54, 48, 43, 25,
        1, 18, 36, 49, 50, 43, 33, 19,
        -4, 9, 25, 35, 36, 29, 17, 5,
        -19, -7, 5, 15, 15, 10, -1, -16,
        -39, -31, -19, -2, -11, -6, -19, -41,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
        14, 14, 14, 15, 14, 13, 7, 4,
        50, 52, 50, 54, 49, 43, 17, 6,
        32, 35, 33, 28, 24, 18, -7, -23,
        10, 9, 11, 10, 2, 2, -15, -19,
        -1, -7, -10, -7, -11, -8, -17, -13,
        -8, -13, -17, -15, -14, -11, -14, -5,
        -10, -8, -11, -13, -7, -4, -3, -2,
        -3, -2, -2, -3, -1, -1, 0, -0,
    ],
    // passed pawns eg
    [
        20, 19, 19, 17, 16, 17, 19, 21,
        82, 81, 79, 66, 63, 68, 80, 86,
        83, 82, 75, 58, 55, 66, 75, 87,
        50, 48, 42, 34, 33, 38, 48, 54,
        24, 23, 22, 17, 17, 20, 29, 30,
        5, 8, 11, 7, 7, 8, 15, 11,
        2, 3, 8, 7, 2, 3, 5, 4,
        0, 1, 2, 2, 0, 0, 1, 1,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 31;
const ROOK_OPEN_FILE_EG: i32 = 12;
const ROOK_CLOSED_FILE_MG: i32 = -16;
const ROOK_CLOSED_FILE_EG: i32 = -3;
const ROOK_SEMIOPEN_FILE_MG: i32 = 6;
const ROOK_SEMIOPEN_FILE_EG: i32 = 12;
const KING_OPEN_FILE_MG: i32 = -77;
const KING_OPEN_FILE_EG: i32 = -9;
const KING_CLOSED_FILE_MG: i32 = 15;
const KING_CLOSED_FILE_EG: i32 = -16;
const KING_SEMIOPEN_FILE_MG: i32 = -35;
const KING_SEMIOPEN_FILE_EG: i32 = 8;

// TODO: Differentiate between rooks and kings in front of / behind pawns.

const PIECE_PHASE: [i32; 6] = [0, 1, 1, 2, 4, 0];

/// Has to be in the same order as the FileOpenness in hce.rs.
/// `SemiClosed` is last because it doesn't get counted.
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
