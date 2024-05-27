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
           0,    0,    0,    0,    0,    0,    0,    0,
          91,   93,   89,  103,   90,   80,   30,   24,
          78,   89,  110,  116,  123,  166,  148,  112,
          64,   87,   83,   88,  109,  105,  101,   86,
          52,   78,   73,   90,   88,   87,   87,   70,
          50,   74,   67,   68,   80,   80,  101,   77,
          50,   72,   63,   53,   70,   96,  111,   68,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         131,  129,  128,  104,  108,  112,  136,  139,
         129,  128,  105,  106,   99,   89,  121,  114,
         121,  116,  101,   90,   90,   90,  107,   97,
         110,  112,   98,   94,   95,   95,  103,   91,
         108,  110,   96,  106,  103,   98,  101,   89,
         112,  114,  102,  106,  114,  103,  100,   91,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         150,  195,  243,  276,  320,  226,  231,  194,
         282,  304,  338,  354,  339,  396,  307,  319,
         303,  342,  363,  375,  410,  418,  364,  330,
         304,  321,  346,  370,  349,  375,  328,  341,
         292,  309,  326,  326,  336,  330,  330,  304,
         272,  298,  311,  318,  329,  315,  321,  289,
         258,  270,  290,  301,  303,  307,  295,  290,
         222,  267,  256,  271,  277,  294,  272,  246,
    ],
    // knight eg
    [
         270,  326,  347,  335,  333,  328,  323,  249,
         322,  340,  343,  344,  340,  324,  334,  308,
         331,  344,  359,  362,  348,  342,  338,  323,
         341,  361,  373,  374,  376,  370,  362,  332,
         343,  354,  374,  378,  379,  368,  354,  334,
         327,  346,  356,  367,  365,  351,  339,  328,
         319,  337,  343,  345,  344,  339,  326,  327,
         306,  298,  328,  331,  330,  318,  305,  299,
    ],
    // bishop mg
    [
         307,  285,  291,  264,  272,  275,  319,  289,
         324,  353,  348,  330,  357,  354,  346,  321,
         338,  365,  364,  385,  378,  401,  379,  367,
         332,  350,  369,  384,  379,  373,  350,  335,
         328,  341,  349,  370,  366,  349,  344,  337,
         335,  347,  347,  351,  352,  347,  349,  350,
         340,  341,  354,  332,  340,  353,  361,  345,
         316,  340,  322,  313,  318,  319,  342,  327,
    ],
    // bishop eg
    [
         352,  364,  360,  371,  367,  361,  356,  351,
         345,  358,  363,  365,  357,  357,  361,  344,
         363,  362,  370,  362,  367,  369,  361,  359,
         363,  377,  371,  381,  379,  376,  377,  362,
         360,  374,  381,  379,  378,  378,  372,  351,
         358,  368,  374,  373,  379,  372,  358,  350,
         353,  352,  351,  365,  366,  355,  354,  334,
         338,  348,  336,  357,  352,  352,  336,  326,
    ],
    // rook mg
    [
         441,  430,  435,  442,  455,  464,  473,  485,
         416,  418,  437,  454,  441,  476,  470,  486,
         400,  424,  424,  425,  456,  469,  500,  468,
         392,  404,  407,  417,  422,  432,  437,  438,
         386,  387,  387,  400,  401,  398,  420,  414,
         384,  386,  387,  392,  401,  407,  443,  423,
         385,  389,  396,  397,  405,  416,  428,  399,
         408,  402,  401,  409,  416,  420,  420,  412,
    ],
    // rook eg
    [
         632,  641,  645,  638,  635,  635,  633,  626,
         643,  648,  647,  637,  640,  629,  629,  617,
         640,  637,  637,  633,  622,  618,  611,  612,
         639,  636,  638,  632,  624,  621,  620,  612,
         630,  631,  632,  628,  624,  625,  615,  610,
         622,  622,  620,  620,  616,  611,  593,  594,
         617,  619,  619,  616,  610,  605,  598,  604,
         620,  618,  627,  620,  612,  615,  609,  608,
    ],
    // queen mg
    [
         834,  852,  876,  907,  902,  921,  951,  888,
         867,  850,  860,  853,  859,  898,  884,  921,
         873,  869,  875,  886,  901,  940,  939,  928,
         861,  866,  871,  872,  876,  887,  885,  894,
         865,  862,  865,  871,  874,  872,  885,  885,
         861,  872,  869,  869,  873,  879,  891,  884,
         860,  871,  880,  880,  879,  889,  895,  896,
         861,  853,  858,  875,  866,  855,  865,  862,
    ],
    // queen eg
    [
        1206, 1214, 1229, 1218, 1224, 1206, 1161, 1198,
        1176, 1214, 1244, 1261, 1276, 1235, 1218, 1190,
        1174, 1199, 1229, 1237, 1245, 1226, 1192, 1187,
        1184, 1209, 1223, 1243, 1255, 1242, 1230, 1204,
        1176, 1208, 1212, 1235, 1226, 1223, 1205, 1193,
        1168, 1179, 1202, 1199, 1201, 1193, 1174, 1161,
        1164, 1164, 1158, 1167, 1169, 1143, 1119, 1095,
        1152, 1159, 1165, 1154, 1157, 1149, 1131, 1131,
    ],
    // king mg
    [
          44,    6,   36,  -61,  -17,  -16,   35,  168,
         -79,  -33,  -54,   19,   -6,  -32,    3,   -2,
         -94,    0,  -53,  -69,  -31,   18,   -7,  -36,
         -75,  -74,  -91, -116, -115,  -91, -100, -112,
         -93,  -83, -108, -124, -126, -111, -114, -132,
         -62,  -39,  -86,  -91,  -84,  -92,  -54,  -70,
          27,   -5,  -21,  -52,  -56,  -33,    6,   12,
          14,   47,   24,  -69,   -5,  -56,   27,   24,
    ],
    // king eg
    [
         -95,  -32,  -25,   10,   -6,    3,   -3,  -99,
          -2,   30,   38,   24,   37,   52,   43,    5,
          13,   34,   51,   62,   56,   54,   53,   22,
           6,   40,   58,   69,   70,   62,   55,   26,
           1,   30,   52,   67,   65,   52,   39,   21,
          -1,   17,   37,   47,   45,   36,   18,    5,
         -22,    2,   12,   23,   23,   12,   -5,  -22,
         -54,  -44,  -26,   -1,  -29,   -9,  -36,  -66,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          91,   93,   89,  103,   90,   80,   30,   24,
          33,   42,   33,   19,   18,    8,  -32,  -51,
          12,    8,   18,   17,    0,    5,  -23,  -18,
          -3,  -13,  -19,  -12,  -19,  -10,  -26,  -16,
         -10,  -23,  -24,  -19,  -19,  -15,  -24,    2,
         -18,   -9,  -17,  -20,   -5,   -4,    2,   -4,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         131,  129,  128,  104,  108,  112,  136,  139,
         109,  107,   94,   64,   68,   90,   97,  117,
          57,   54,   45,   37,   39,   45,   61,   63,
          31,   28,   26,   19,   22,   25,   39,   37,
           0,    7,   12,    2,    8,    9,   21,    5,
           3,    5,   15,   10,   -1,    4,    6,    6,
           0,    0,    0,    0,    0,    0,    0,    0,
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
