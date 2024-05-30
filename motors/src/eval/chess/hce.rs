use strum::IntoEnumIterator;

use crate::eval::chess::{
    pawn_shield_idx, FileOpenness, NUM_PAWN_SHIELD_CONFIGURATIONS, NUM_PHASES,
};
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
use crate::eval::chess::PhaseType::{Eg, Mg};
use crate::eval::Eval;

#[derive(Default, Debug)]
pub struct HandCraftedEval {}

/// Eval values tuned on a combination of the zurichess dataset and a dataset used by 4ku,
/// created by GCP using his engine Stoofvlees and filtered by cj5716 using Stockfish at depth 9,
/// using my own tuner `pliers`.
#[rustfmt::skip]
const PSQTS: [[i32; NUM_SQUARES]; NUM_CHESS_PIECES * 2] = [
    // pawn MG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         156,  166,  155,  181,  165,  148,   66,   45,
          75,   85,  103,  109,  116,  154,  137,  103,
          61,   83,   79,   82,  106,  100,   95,   77,
          50,   75,   70,   86,   86,   86,   87,   63,
          49,   70,   64,   65,   78,   71,   88,   59,
          48,   67,   58,   43,   62,   76,   94,   52,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         268,  264,  260,  215,  215,  226,  271,  280,
         122,  121,  101,  103,   94,   88,  117,  112,
         114,  110,   96,   86,   86,   86,  103,   93,
         104,  106,   93,   90,   89,   89,   97,   87,
         101,  104,   92,  100,   98,   94,   96,   87,
         105,  107,   98,  104,  109,  103,   97,   91,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight MG
    [
         139,  186,  231,  260,  302,  214,  219,  179,
         267,  289,  320,  337,  316,  375,  290,  303,
         287,  324,  345,  356,  387,  392,  346,  309,
         289,  305,  330,  351,  333,  357,  312,  322,
         278,  293,  310,  310,  320,  314,  315,  290,
         259,  283,  296,  303,  314,  300,  305,  273,
         246,  257,  276,  287,  288,  293,  281,  278,
         212,  253,  243,  259,  265,  279,  257,  233,
    ],
    // knight EG
    [
         257,  308,  328,  317,  315,  310,  304,  236,
         305,  322,  324,  325,  323,  306,  316,  290,
         313,  325,  339,  342,  329,  325,  319,  307,
         322,  341,  352,  353,  355,  349,  342,  314,
         324,  335,  353,  357,  358,  347,  333,  315,
         309,  327,  336,  347,  345,  332,  320,  310,
         302,  319,  324,  326,  324,  319,  308,  307,
         289,  282,  309,  312,  312,  300,  288,  285,
    ],
    // bishop MG
    [
         292,  272,  276,  252,  261,  261,  306,  278,
         310,  335,  333,  314,  342,  342,  333,  311,
         322,  347,  346,  367,  363,  384,  364,  350,
         316,  334,  353,  367,  361,  356,  334,  320,
         313,  326,  333,  354,  349,  334,  327,  322,
         319,  331,  331,  335,  336,  331,  333,  332,
         325,  326,  338,  317,  324,  336,  344,  328,
         303,  325,  307,  299,  303,  305,  321,  311,
    ],
    // bishop EG
    [
         333,  344,  340,  350,  346,  341,  336,  331,
         325,  338,  342,  345,  337,  336,  340,  324,
         343,  341,  349,  342,  346,  349,  340,  340,
         342,  356,  350,  359,  358,  355,  355,  341,
         340,  353,  359,  357,  357,  356,  351,  332,
         338,  347,  354,  352,  358,  351,  338,  330,
         333,  332,  332,  344,  346,  335,  332,  315,
         319,  329,  317,  337,  332,  332,  319,  307,
    ],
    // rook MG
    [
         419,  407,  413,  419,  430,  441,  451,  460,
         395,  398,  416,  432,  420,  454,  448,  460,
         380,  403,  403,  405,  433,  446,  475,  440,
         373,  383,  388,  396,  402,  412,  415,  413,
         366,  368,  368,  381,  382,  381,  399,  393,
         365,  366,  368,  373,  383,  388,  422,  400,
         366,  370,  376,  378,  385,  398,  410,  379,
         388,  382,  381,  389,  396,  399,  399,  394,
    ],
    // rook EG
    [
         597,  606,  609,  603,  600,  600,  597,  592,
         607,  612,  611,  601,  604,  594,  594,  583,
         605,  602,  602,  598,  587,  583,  578,  580,
         604,  601,  603,  598,  589,  586,  586,  579,
         596,  596,  597,  593,  590,  590,  582,  576,
         588,  588,  586,  586,  582,  577,  560,  562,
         583,  585,  585,  582,  576,  571,  563,  570,
         586,  584,  592,  586,  579,  582,  574,  571,
    ],
    // queen MG
    [
         795,  811,  833,  862,  858,  875,  908,  848,
         828,  812,  822,  815,  821,  860,  849,  883,
         833,  829,  835,  846,  862,  898,  897,  883,
         821,  827,  832,  833,  837,  847,  844,  854,
         826,  823,  827,  833,  835,  834,  845,  845,
         822,  833,  830,  830,  834,  840,  851,  843,
         822,  831,  841,  839,  839,  849,  854,  853,
         822,  815,  819,  835,  828,  817,  821,  822,
    ],
    // queen EG
    [
        1138, 1146, 1162, 1152, 1157, 1141, 1096, 1130,
        1109, 1146, 1173, 1190, 1204, 1164, 1146, 1121,
        1109, 1132, 1159, 1167, 1175, 1156, 1126, 1122,
        1117, 1140, 1154, 1174, 1186, 1172, 1163, 1137,
        1110, 1141, 1144, 1165, 1158, 1154, 1137, 1126,
        1102, 1114, 1134, 1131, 1134, 1126, 1108, 1097,
        1098, 1099, 1093, 1101, 1103, 1078, 1057, 1036,
        1087, 1094, 1100, 1089, 1091, 1084, 1070, 1065,
    ],
    // king MG
    [
          88,   36,   64,  -26,   13,   18,   65,  193,
         -25,    1,  -19,   48,   25,    4,   37,   40,
         -41,   33,  -21,  -35,    3,   51,   28,    8,
         -26,  -39,  -57,  -81,  -79,  -56,  -61,  -66,
         -48,  -51,  -74,  -95,  -96,  -79,  -82,  -91,
         -33,  -13,  -60,  -68,  -62,  -65,  -34,  -39,
          27,   10,   -2,  -32,  -32,  -18,   25,   31,
          -9,   43,   36,  -38,   21,  -35,   33,   39,
    ],
    // king EG
    [
        -118,  -52,  -45,  -12,  -26,  -18,  -24, -119,
         -30,    7,   14,    2,   14,   27,   19,  -23,
         -15,   11,   27,   37,   32,   29,   28,   -6,
         -21,   17,   34,   45,   45,   38,   31,   -1,
         -23,    8,   29,   45,   42,   29,   17,   -2,
         -21,   -3,   16,   26,   25,   15,   -1,  -12,
         -32,  -16,   -7,    3,    3,   -6,  -24,  -29,
         -53,  -56,  -45,  -21,  -48,  -26,  -51,  -68,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns MG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          17,   12,   16,   15,    7,    6,  -10,   -1,
          30,   39,   33,   20,   19,    8,  -26,  -47,
          10,    8,   18,   17,   -2,    6,  -16,  -14,
          -3,  -12,  -17,  -11,  -20,  -12,  -24,  -13,
         -10,  -21,  -22,  -19,  -20,  -13,  -16,    5,
         -16,   -9,  -16,  -17,   -3,    1,    7,    1,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         -21,  -19,  -17,  -17,  -10,  -12,  -14,  -16,
         104,  102,   89,   58,   64,   83,   90,  110,
          55,   52,   42,   35,   37,   42,   57,   60,
          30,   27,   24,   18,   22,   24,   37,   35,
           1,    7,   12,    2,    8,    8,   20,    5,
           3,    5,   15,    8,   -1,    1,    5,    4,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 30;
const ROOK_OPEN_FILE_EG: i32 = 11;
const ROOK_CLOSED_FILE_MG: i32 = -14;
const ROOK_CLOSED_FILE_EG: i32 = -3;
const ROOK_SEMIOPEN_FILE_MG: i32 = 7;
const ROOK_SEMIOPEN_FILE_EG: i32 = 12;
const KING_OPEN_FILE_MG: i32 = -58;
const KING_OPEN_FILE_EG: i32 = -9;
const KING_CLOSED_FILE_MG: i32 = 14;
const KING_CLOSED_FILE_EG: i32 = -14;
const KING_SEMIOPEN_FILE_MG: i32 = -14;
const KING_SEMIOPEN_FILE_EG: i32 = 7;
const PAWN_SHIELDS: [[i32; NUM_PHASES]; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    [-48, 13],
    [-31, 8],
    [-16, 4],
    [3, 13],
    [-12, 3],
    [-16, -13],
    [2, 5],
    [23, -18],
    [-31, 8],
    [-39, -24],
    [-13, 15],
    [12, -11],
    [-12, -4],
    [-25, -33],
    [18, 10],
    [15, -58],
    [-49, 10],
    [-1, 13],
    [-32, -27],
    [6, -13],
    [-8, 7],
    [27, 5],
    [-13, -24],
    [73, -21],
    [-33, 33],
    [-1, 4],
    [4, 12],
    [-59, 60],
    [6, 8],
    [7, -20],
    [-26, 22],
    [0, 0],
    [-18, 7],
    [-13, 5],
    [7, 5],
    [28, 5],
    [-33, -35],
    [-24, -51],
    [-8, -12],
    [40, -32],
    [-30, -10],
    [-46, -20],
    [9, 1],
    [9, -39],
    [-45, -40],
    [-92, -140],
    [-3, -15],
    [-734, 439],
    [-26, 18],
    [15, 13],
    [-3, -25],
    [145, -29],
    [-7, -2],
    [6, 38],
    [132, -68],
    [0, 0],
    [-34, 12],
    [10, -19],
    [59, 6],
    [0, 0],
    [-6, 0],
    [597, 654],
    [0, 0],
    [0, 0],
    [-63, 19],
    [2, -9],
    [25, 0],
    [51, -31],
    [-2, -4],
    [-42, -54],
    [64, -32],
    [-8, -20],
    [21, 0],
    [10, -22],
    [53, -92],
    [25, -24],
    [15, -11],
    [10, 16],
    [32, -31],
    [0, 0],
    [-52, 17],
    [-16, -7],
    [-11, -5],
    [11, -28],
    [-35, 1],
    [-10, -54],
    [-14, -13],
    [-9, -62],
    [-38, 4],
    [12, -6],
    [-5, -70],
    [189, -75],
    [-31, -10],
    [22, -65],
    [11, -57],
    [0, 0],
];

// TODO: Differentiate between rooks and kings in front of / behind pawns.

const PIECE_PHASE: [i32; 6] = [0, 1, 1, 2, 4, 0];

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
            let king_square = pos.king_square(color);
            let king_file = king_square.file();
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
            mg += Score(PAWN_SHIELDS[pawn_shield_idx(our_pawns, king_square, color)][Mg as usize]);
            eg += Score(PAWN_SHIELDS[pawn_shield_idx(our_pawns, king_square, color)][Eg as usize]);

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
