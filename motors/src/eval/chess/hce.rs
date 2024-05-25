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
          97,   99,   93,  111,   99,   89,   35,   30,
          90,   99,  122,  133,  139,  182,  162,  127,
          72,   97,   95,   97,  124,  117,  114,   97,
          60,   89,   84,  104,   99,   97,   97,   79,
          57,   84,   77,   79,   94,   89,  114,   86,
          55,   82,   72,   64,   84,  109,  123,   79,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         125,  124,  127,  103,  108,  108,  133,  134,
         130,  130,  112,  112,   90,   83,  120,  114,
         121,  116,  101,   91,   88,   87,  104,   96,
         110,  113,   99,   96,   94,   92,  102,   91,
         109,  108,   97,  108,  104,   96,  101,   88,
         113,  115,  106,  108,  116,  102,  100,   93,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         170,  217,  270,  279,  336,  252,  255,  224,
         328,  349,  380,  394,  382,  444,  356,  367,
         349,  383,  407,  421,  463,  461,  415,  374,
         346,  366,  390,  419,  399,  423,  374,  386,
         330,  348,  370,  370,  382,  375,  373,  346,
         307,  337,  353,  360,  371,  359,  362,  328,
         290,  311,  331,  342,  343,  349,  337,  327,
         261,  302,  290,  310,  315,  330,  305,  277,
    ],
    // knight eg
    [
         269,  330,  345,  344,  336,  327,  321,  257,
         323,  338,  346,  343,  335,  323,  332,  307,
         332,  344,  359,  359,  345,  340,  334,  324,
         339,  362,  373,  372,  377,  368,  363,  330,
         347,  357,  376,  380,  384,  371,  358,  336,
         329,  350,  363,  372,  368,  358,  342,  332,
         322,  343,  348,  353,  350,  346,  333,  329,
         311,  302,  331,  333,  338,  322,  309,  306,
    ],
    // bishop mg
    [
         343,  339,  325,  293,  305,  307,  368,  321,
         374,  399,  396,  374,  400,  400,  392,  374,
         385,  408,  407,  434,  424,  449,  429,  417,
         377,  396,  417,  432,  429,  422,  395,  381,
         368,  386,  395,  418,  414,  397,  389,  379,
         381,  392,  393,  397,  398,  394,  395,  396,
         382,  387,  401,  376,  386,  398,  408,  390,
         364,  385,  364,  356,  358,  359,  383,  377,
    ],
    // bishop eg
    [
         356,  366,  368,  376,  371,  368,  359,  357,
         349,  363,  364,  367,  359,  359,  366,  348,
         368,  364,  376,  364,  369,  371,  364,  363,
         369,  381,  375,  385,  381,  379,  382,  366,
         367,  381,  385,  381,  381,  383,  378,  356,
         365,  372,  382,  379,  384,  380,  366,  359,
         360,  363,  356,  373,  374,  361,  363,  344,
         350,  356,  345,  363,  360,  360,  349,  339,
    ],
    // rook mg
    [
         499,  488,  489,  496,  513,  525,  532,  553,
         480,  477,  502,  520,  505,  548,  538,  554,
         458,  478,  481,  483,  518,  532,  570,  538,
         450,  466,  464,  472,  478,  495,  501,  502,
         446,  449,  445,  458,  461,  460,  481,  480,
         448,  451,  450,  453,  462,  467,  506,  486,
         448,  450,  457,  456,  463,  477,  487,  457,
         470,  463,  461,  469,  477,  483,  480,  470,
    ],
    // rook eg
    [
         640,  650,  657,  649,  646,  643,  641,  632,
         647,  655,  653,  643,  647,  634,  633,  622,
         648,  644,  643,  638,  628,  622,  612,  615,
         647,  642,  646,  642,  631,  627,  622,  616,
         642,  640,  644,  638,  636,  635,  626,  619,
         636,  636,  633,  633,  627,  622,  602,  606,
         633,  634,  632,  629,  623,  617,  608,  616,
         636,  632,  640,  632,  624,  630,  620,  623,
    ],
    // queen mg
    [
         945,  973,  991, 1020, 1034, 1048, 1084, 1010,
         999,  980,  991,  980,  987, 1034, 1021, 1056,
        1002, 1000, 1005, 1020, 1031, 1073, 1077, 1060,
         985,  995, 1000, 1001, 1005, 1021, 1014, 1025,
         992,  989,  990,  999, 1002,  998, 1012, 1011,
         988,  998,  993,  993,  995, 1003, 1016, 1008,
         986,  993, 1005, 1004, 1002, 1012, 1018, 1022,
         984,  976,  981,  999,  992,  978,  987,  984,
    ],
    // queen eg
    [
        1222, 1226, 1247, 1240, 1233, 1216, 1160, 1202,
        1183, 1220, 1251, 1269, 1288, 1246, 1224, 1199,
        1190, 1203, 1238, 1241, 1258, 1234, 1198, 1197,
        1202, 1220, 1233, 1254, 1267, 1250, 1243, 1218,
        1193, 1224, 1231, 1246, 1243, 1240, 1219, 1210,
        1179, 1200, 1223, 1219, 1227, 1217, 1193, 1182,
        1178, 1186, 1178, 1186, 1192, 1165, 1139, 1109,
        1168, 1178, 1186, 1178, 1175, 1172, 1149, 1151,
    ],
    // king mg
    [
          42,   -7,   16,  -66,  -51,  -11,   32,  145,
         -87,  -46,  -65,   19,  -11,  -43,   -3,   -8,
        -119,  -12,  -65,  -89,  -45,    1,  -17,  -65,
         -89,  -79, -108, -141, -133, -116, -116, -139,
         -98,  -92, -117, -140, -144, -127, -122, -141,
         -66,  -43,  -91,  -97,  -90,  -98,  -57,  -74,
          30,   -4,  -21,  -57,  -61,  -36,    5,   12,
          17,   52,   27,  -80,   -9,  -62,   28,   25,
    ],
    // king eg
    [
        -114,  -43,  -34,    2,  -13,  -10,  -13, -122,
         -11,   22,   27,   11,   28,   42,   31,   -2,
           7,   26,   46,   54,   48,   47,   45,   13,
           0,   33,   55,   65,   66,   60,   50,   22,
          -1,   27,   51,   66,   65,   51,   36,   18,
          -3,   19,   39,   48,   44,   36,   17,    7,
         -23,    2,   11,   23,   26,   12,   -3,  -24,
         -53,  -47,  -27,    1,  -26,   -3,  -34,  -66,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          97,   99,   93,  111,   99,   89,   35,   30,
          23,   44,   37,   17,   19,    8,  -31,  -58,
           9,    7,   19,   18,   -1,    6,  -26,  -21,
          -5,  -13,  -20,  -12,  -21,  -10,  -28,  -20,
         -12,  -25,  -25,  -21,  -21,  -19,  -25,    1,
         -22,   -8,  -14,  -19,   -7,   -2,    3,   -5,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         125,  124,  127,  103,  108,  108,  133,  134,
         104,  104,   88,   57,   76,   96,   98,  113,
          55,   55,   47,   36,   43,   49,   64,   62,
          31,   30,   27,   21,   25,   28,   42,   38,
           0,    7,   14,    5,   11,   12,   26,    5,
           5,    5,   16,    9,   -1,    6,   10,    8,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 35;
const ROOK_OPEN_FILE_EG: i32 = 9;
const ROOK_SEMIOPEN_FILE_MG: i32 = -18;
const ROOK_SEMIOPEN_FILE_EG: i32 = -4;
const ROOK_CLOSED_FILE_MG: i32 = 6;
const ROOK_CLOSED_FILE_EG: i32 = 11;
const KING_OPEN_FILE_MG: i32 = -84;
const KING_OPEN_FILE_EG: i32 = -9;
const KING_SEMIOPEN_FILE_MG: i32 = 17;
const KING_SEMIOPEN_FILE_EG: i32 = -17;
const KING_CLOSED_FILE_MG: i32 = -39;
const KING_CLOSED_FILE_EG: i32 = 10;

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
