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
          78,   89,  110,  116,  124,  166,  148,  112,
          64,   87,   83,   88,  109,  105,  101,   86,
          52,   78,   74,   90,   88,   87,   87,   70,
          51,   74,   67,   68,   80,   80,  101,   77,
          50,   72,   63,   53,   70,   96,  111,   68,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         275,  271,  267,  220,  220,  231,  278,  287,
         129,  128,  105,  106,   99,   89,  121,  114,
         121,  116,  101,   90,   90,   90,  107,   97,
         110,  112,   98,   94,   95,   95,  103,   91,
         108,  110,   96,  106,  103,   98,  101,   89,
         112,  114,  102,  106,  114,  103,  100,   91,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         151,  196,  243,  276,  320,  226,  232,  194,
         282,  305,  339,  354,  339,  397,  308,  320,
         303,  342,  364,  376,  411,  419,  364,  331,
         305,  322,  347,  371,  350,  375,  329,  342,
         292,  309,  327,  326,  337,  331,  331,  304,
         273,  298,  311,  319,  330,  316,  321,  290,
         258,  271,  290,  302,  303,  308,  295,  290,
         223,  267,  256,  272,  278,  294,  273,  247,
    ],
    // knight eg
    [
         270,  326,  347,  335,  333,  328,  322,  249,
         322,  340,  343,  344,  340,  324,  334,  308,
         331,  344,  359,  362,  348,  342,  338,  323,
         341,  361,  373,  374,  376,  370,  362,  332,
         343,  354,  374,  378,  379,  368,  354,  334,
         327,  346,  356,  367,  365,  351,  339,  328,
         319,  337,  343,  345,  344,  339,  326,  327,
         305,  298,  328,  331,  330,  318,  305,  299,
    ],
    // bishop mg
    [
         307,  286,  291,  265,  273,  275,  320,  290,
         325,  353,  349,  331,  357,  355,  347,  321,
         338,  365,  365,  386,  379,  402,  380,  368,
         332,  351,  370,  385,  379,  374,  350,  335,
         328,  342,  349,  371,  367,  350,  344,  337,
         335,  348,  347,  351,  353,  348,  350,  351,
         341,  342,  354,  333,  341,  354,  362,  345,
         317,  341,  323,  313,  319,  319,  343,  328,
    ],
    // bishop eg
    [
         352,  364,  360,  371,  367,  361,  356,  351,
         345,  358,  363,  365,  357,  358,  361,  344,
         363,  362,  370,  362,  367,  369,  361,  359,
         363,  377,  371,  381,  379,  376,  377,  362,
         360,  374,  381,  379,  378,  378,  372,  351,
         358,  368,  374,  373,  379,  372,  358,  350,
         353,  352,  351,  365,  366,  355,  354,  334,
         338,  348,  336,  357,  352,  352,  336,  326,
    ],
    // rook mg
    [
         442,  431,  436,  443,  456,  465,  474,  486,
         417,  419,  438,  455,  442,  477,  471,  487,
         401,  425,  425,  426,  457,  469,  501,  469,
         393,  404,  408,  418,  422,  433,  438,  439,
         387,  388,  387,  401,  402,  399,  421,  415,
         385,  386,  388,  393,  402,  408,  444,  424,
         386,  390,  396,  398,  405,  417,  429,  400,
         409,  403,  401,  410,  417,  421,  421,  413,
    ],
    // rook eg
    [
         632,  641,  645,  638,  635,  635,  633,  626,
         643,  648,  647,  637,  640,  629,  629,  617,
         640,  637,  637,  633,  622,  618,  611,  612,
         639,  636,  638,  632,  624,  621,  620,  612,
         630,  631,  632,  628,  624,  625,  615,  610,
         622,  622,  620,  621,  616,  611,  593,  594,
         617,  619,  619,  616,  610,  605,  598,  604,
         620,  618,  627,  620,  612,  615,  609,  608,
    ],
    // queen mg
    [
         839,  856,  881,  911,  907,  926,  956,  892,
         872,  854,  865,  858,  864,  902,  888,  926,
         878,  874,  880,  890,  905,  944,  944,  932,
         865,  870,  875,  876,  880,  891,  889,  899,
         870,  867,  870,  876,  878,  876,  890,  890,
         865,  877,  873,  873,  877,  884,  895,  888,
         865,  875,  885,  884,  883,  893,  899,  901,
         866,  857,  863,  879,  871,  860,  870,  867,
    ],
    // queen eg
    [
        1204, 1213, 1227, 1217, 1222, 1204, 1159, 1196,
        1174, 1213, 1242, 1259, 1274, 1233, 1216, 1188,
        1173, 1197, 1227, 1235, 1243, 1224, 1190, 1185,
        1182, 1207, 1221, 1242, 1253, 1240, 1228, 1202,
        1174, 1207, 1211, 1233, 1225, 1221, 1203, 1191,
        1166, 1177, 1200, 1197, 1199, 1191, 1172, 1159,
        1162, 1162, 1156, 1165, 1167, 1141, 1116, 1092,
        1150, 1157, 1163, 1152, 1155, 1147, 1129, 1127,
    ],
    // king mg
    [
        -160, -147, -136, -166, -158, -160, -157, -167,
        -140, -104, -109,  -82,  -96, -109,  -98, -122,
        -119,  -42,  -72,  -82,  -58,  -25,  -46,  -84,
         -88,  -75,  -84, -105, -104,  -82,  -91, -104,
         -78,  -69,  -94, -110, -112,  -98, -101, -118,
         -48,  -26,  -73,  -77,  -71,  -79,  -41,  -57,
          41,    8,   -8,  -39,  -43,  -20,   19,   26,
          28,   60,   37,  -55,    8,  -43,   40,   37,
    ],
    // king eg
    [
        -112,  -50,  -44,   -8,  -24,  -15,  -20, -110,
         -22,    9,   16,    3,   16,   31,   22,  -15,
          -8,   13,   29,   40,   35,   32,   32,    1,
         -15,   18,   37,   47,   48,   40,   34,    4,
         -20,    8,   31,   46,   43,   30,   17,   -1,
         -23,   -4,   16,   25,   24,   14,   -4,  -17,
         -44,  -20,  -10,    1,    1,  -10,  -27,  -44,
         -76,  -66,  -48,  -23,  -51,  -30,  -58,  -87,
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
         -14,  -12,  -10,  -12,   -5,   -7,   -7,   -9,
         109,  107,   94,   64,   68,   90,   97,  117,
          57,   54,   45,   37,   39,   46,   61,   63,
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
