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
         160,  170,  159,  186,  170,  151,   68,   47,
          78,   89,  110,  116,  123,  166,  148,  112,
          64,   87,   83,   88,  109,  105,  101,   86,
          52,   78,   74,   90,   88,   87,   87,   70,
          51,   74,   67,   68,   80,   80,  101,   77,
          50,   72,   63,   53,   70,   96,  111,   68,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         275,  270,  267,  220,  220,  231,  278,  287,
         129,  128,  105,  106,   99,   89,  121,  114,
         121,  116,  101,   90,   90,   90,  107,   97,
         110,  112,   98,   94,   95,   95,  103,   91,
         108,  110,   96,  106,  103,   98,  101,   89,
         112,  113,  102,  106,  114,  103,  100,   91,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         151,  196,  243,  276,  320,  226,  231,  194,
         282,  305,  339,  354,  339,  396,  308,  320,
         303,  342,  364,  376,  410,  419,  364,  331,
         305,  322,  346,  371,  349,  375,  329,  341,
         292,  309,  326,  326,  336,  330,  331,  304,
         273,  298,  311,  319,  330,  316,  321,  290,
         258,  270,  290,  302,  303,  308,  295,  290,
         222,  267,  256,  271,  278,  294,  272,  247,
    ],
    // knight eg
    [
         270,  326,  346,  335,  333,  328,  322,  249,
         322,  340,  343,  344,  340,  323,  334,  308,
         330,  344,  358,  362,  347,  342,  338,  323,
         341,  361,  373,  373,  376,  370,  362,  332,
         343,  354,  374,  377,  379,  368,  353,  334,
         327,  346,  355,  367,  365,  351,  339,  327,
         319,  337,  343,  345,  344,  338,  326,  326,
         305,  298,  327,  330,  330,  317,  305,  299,
    ],
    // bishop mg
    [
         307,  286,  291,  265,  273,  275,  319,  290,
         324,  353,  349,  331,  357,  354,  347,  321,
         338,  365,  364,  386,  379,  402,  380,  368,
         332,  350,  370,  384,  379,  374,  350,  335,
         328,  342,  349,  371,  367,  350,  344,  337,
         335,  347,  347,  351,  352,  347,  349,  351,
         341,  341,  354,  332,  341,  354,  361,  345,
         317,  340,  322,  313,  318,  319,  342,  328,
    ],
    // bishop eg
    [
         352,  364,  360,  370,  367,  361,  356,  350,
         345,  358,  362,  365,  357,  357,  360,  344,
         363,  361,  370,  362,  367,  369,  361,  359,
         362,  376,  371,  381,  379,  376,  376,  362,
         360,  374,  380,  379,  378,  377,  371,  350,
         358,  368,  374,  373,  379,  372,  358,  349,
         353,  352,  351,  364,  366,  355,  353,  334,
         338,  348,  336,  356,  352,  352,  336,  326,
    ],
    // rook mg
    [
         442,  431,  436,  443,  456,  465,  473,  485,
         416,  419,  438,  455,  441,  477,  471,  487,
         401,  425,  425,  426,  457,  469,  500,  469,
         393,  404,  407,  417,  422,  433,  438,  439,
         387,  388,  387,  400,  402,  398,  420,  415,
         385,  386,  387,  393,  402,  408,  444,  424,
         386,  390,  396,  397,  405,  416,  429,  400,
         409,  403,  401,  409,  416,  421,  421,  412,
    ],
    // rook eg
    [
         631,  641,  644,  638,  634,  634,  632,  626,
         642,  647,  646,  636,  639,  629,  628,  617,
         640,  636,  637,  632,  621,  617,  611,  612,
         638,  635,  638,  632,  623,  620,  619,  612,
         630,  631,  632,  627,  624,  625,  615,  609,
         621,  622,  620,  620,  616,  611,  593,  594,
         617,  619,  619,  616,  610,  605,  598,  604,
         620,  618,  626,  619,  612,  615,  608,  607,
    ],
    // queen mg
    [
         838,  855,  880,  910,  906,  925,  955,  892,
         871,  854,  864,  857,  863,  902,  887,  925,
         877,  873,  879,  890,  905,  943,  943,  932,
         864,  869,  874,  876,  880,  890,  889,  898,
         869,  866,  869,  875,  878,  875,  889,  889,
         864,  876,  873,  872,  877,  883,  895,  888,
         864,  874,  884,  883,  882,  892,  899,  900,
         865,  857,  862,  878,  870,  859,  869,  865,
    ],
    // queen eg
    [
        1203, 1212, 1226, 1216, 1221, 1203, 1159, 1195,
        1173, 1212, 1241, 1258, 1273, 1232, 1215, 1187,
        1172, 1196, 1226, 1234, 1242, 1223, 1189, 1184,
        1182, 1206, 1221, 1241, 1253, 1239, 1227, 1202,
        1173, 1206, 1210, 1232, 1224, 1220, 1202, 1191,
        1165, 1176, 1199, 1196, 1198, 1190, 1171, 1159,
        1161, 1161, 1155, 1164, 1167, 1141, 1116, 1092,
        1149, 1157, 1163, 1151, 1154, 1146, 1129, 1128,
    ],
    // king mg
    [
          61,   23,   53,  -44,    0,    2,   52,  185,
         -62,  -16,  -37,   36,   11,  -14,   20,   15,
         -77,   18,  -36,  -52,  -13,   35,   10,  -19,
         -58,  -57,  -74,  -99,  -98,  -74,  -83,  -95,
         -75,  -66,  -91, -107, -109,  -94,  -97, -115,
         -45,  -22,  -69,  -73,  -67,  -75,  -37,  -53,
          45,   12,   -4,  -35,  -39,  -16,   23,   29,
          31,   64,   41,  -52,   12,  -39,   44,   41,
    ],
    // king eg
    [
        -106,  -43,  -36,   -1,  -17,   -8,  -14, -110,
         -13,   19,   27,   13,   26,   41,   32,   -6,
           2,   23,   40,   51,   45,   43,   42,   11,
          -5,   29,   47,   58,   59,   51,   44,   15,
         -10,   19,   41,   56,   54,   41,   28,   10,
         -12,    6,   26,   36,   34,   25,    6,   -6,
         -33,   -9,    1,   12,   12,    0,  -16,  -33,
         -65,  -56,  -37,  -12,  -40,  -20,  -47,  -77,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          21,   16,   20,   20,   12,    9,   -8,    1,
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
         -14,  -13,  -10,  -12,   -5,   -7,   -7,   -9,
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
