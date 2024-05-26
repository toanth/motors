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
          91,   93,   89,  102,   92,   81,   35,   25,
          90,   92,   90,  103,   94,   86,   43,   32,
          78,   88,  105,  112,  119,  153,  138,  106,
          64,   84,   84,   89,  107,  107,  104,   89,
          53,   76,   74,   87,   88,   88,   88,   73,
          51,   71,   68,   68,   79,   81,   99,   78,
          50,   70,   64,   55,   69,   93,  109,   73,
          50,   69,   64,   54,   68,   93,  110,   72,
    ],
    // pawn eg
    [
         131,  129,  128,  107,  107,  112,  133,  139,
         131,  129,  127,  106,  107,  111,  132,  137,
         128,  127,  108,  106,   99,   91,  118,  115,
         121,  117,  103,   92,   91,   90,  106,   99,
         111,  112,   99,   95,   95,   95,  102,   93,
         108,  110,   98,  104,  103,   98,  101,   90,
         111,  113,  103,  105,  113,  103,  100,   92,
         112,  113,  103,  105,  113,  104,  100,   92,
    ],
    // knight mg
    [
         160,  200,  246,  279,  318,  244,  239,  205,
         274,  296,  330,  347,  341,  384,  312,  312,
         301,  334,  358,  372,  399,  411,  363,  331,
         303,  320,  344,  366,  353,  373,  335,  338,
         291,  307,  325,  328,  336,  332,  330,  307,
         273,  294,  309,  317,  327,  317,  320,  293,
         256,  269,  288,  299,  302,  307,  296,  288,
         225,  263,  259,  272,  279,  293,  276,  252,
    ],
    // knight eg
    [
         274,  323,  345,  337,  334,  328,  324,  259,
         319,  337,  343,  345,  341,  326,  333,  308,
         331,  343,  357,  361,  350,  344,  340,  325,
         340,  357,  370,  373,  375,  369,  361,  334,
         342,  353,  371,  376,  378,  368,  354,  335,
         328,  344,  355,  366,  365,  352,  341,  329,
         319,  334,  341,  345,  345,  339,  327,  325,
         307,  301,  326,  332,  331,  320,  308,  302,
    ],
    // bishop mg
    [
         308,  291,  295,  272,  277,  281,  318,  295,
         324,  347,  345,  331,  351,  351,  347,  324,
         336,  360,  363,  380,  377,  395,  377,  363,
         332,  349,  366,  381,  378,  374,  353,  339,
         328,  341,  349,  368,  366,  352,  345,  338,
         335,  345,  347,  351,  352,  348,  349,  349,
         338,  340,  351,  333,  339,  350,  358,  345,
         318,  338,  326,  316,  319,  321,  342,  330,
    ],
    // bishop eg
    [
         351,  363,  361,  369,  367,  361,  357,  351,
         347,  358,  363,  365,  359,  359,  360,  347,
         362,  362,  369,  364,  367,  369,  363,  359,
         362,  374,  371,  379,  378,  376,  375,  362,
         360,  373,  379,  379,  378,  377,  372,  353,
         358,  366,  373,  373,  378,  372,  360,  350,
         352,  352,  352,  363,  366,  357,  354,  336,
         339,  348,  338,  355,  353,  352,  339,  328,
    ],
    // rook mg
    [
         439,  430,  435,  442,  453,  464,  472,  484,
         417,  419,  435,  450,  443,  472,  472,  485,
         401,  421,  423,  426,  451,  465,  492,  469,
         393,  403,  406,  415,  422,  432,  439,  440,
         387,  388,  388,  400,  402,  401,  421,  418,
         385,  386,  387,  393,  401,  407,  438,  423,
         387,  390,  395,  397,  404,  415,  427,  404,
         407,  401,  400,  407,  414,  419,  421,  412,
    ],
    // rook eg
    [
         633,  641,  645,  639,  635,  635,  633,  626,
         642,  646,  646,  637,  638,  630,  628,  618,
         640,  638,  638,  634,  624,  619,  614,  613,
         638,  636,  638,  633,  624,  621,  619,  612,
         630,  631,  632,  628,  624,  624,  615,  609,
         622,  623,  621,  621,  617,  612,  597,  596,
         618,  619,  620,  617,  611,  607,  600,  603,
         620,  618,  625,  620,  613,  615,  609,  607,
    ],
    // queen mg
    [
         837,  851,  873,  900,  899,  917,  944,  894,
         865,  852,  861,  859,  865,  899,  893,  919,
         872,  868,  873,  882,  895,  929,  931,  924,
         862,  866,  870,  872,  877,  888,  889,  896,
         865,  863,  866,  871,  874,  873,  885,  886,
         861,  871,  869,  870,  873,  879,  890,  885,
         861,  869,  877,  878,  877,  885,  891,  893,
         861,  855,  859,  874,  868,  858,  867,  864,
    ],
    // queen eg
    [
        1204, 1213, 1228, 1222, 1227, 1211, 1169, 1195,
        1178, 1211, 1239, 1255, 1269, 1235, 1214, 1191,
        1175, 1198, 1227, 1238, 1247, 1230, 1199, 1190,
        1183, 1205, 1221, 1240, 1252, 1240, 1227, 1204,
        1176, 1204, 1211, 1231, 1227, 1223, 1206, 1193,
        1168, 1179, 1198, 1199, 1200, 1193, 1174, 1160,
        1163, 1164, 1161, 1168, 1171, 1149, 1126, 1104,
        1153, 1159, 1164, 1156, 1158, 1150, 1132, 1129,
    ],
    // king mg
    [
          35,    4,   27,  -49,  -18,  -16,   28,  145,
         -71,  -32,  -45,    6,   -9,  -27,    2,    6,
         -92,  -14,  -53,  -66,  -36,    4,  -13,  -37,
         -78,  -72,  -87, -111, -111,  -87,  -93, -106,
         -89,  -80, -102, -119, -122, -110, -109, -124,
         -58,  -40,  -79,  -89,  -85,  -90,  -57,  -68,
          20,   -4,  -20,  -52,  -56,  -39,   -2,    7,
          15,   41,   23,  -60,  -14,  -51,   18,   23,
    ],
    // king eg
    [
         -88,  -33,  -21,    9,   -2,    6,    1,  -83,
          -8,   22,   33,   25,   35,   47,   41,    4,
          11,   32,   49,   59,   56,   54,   53,   24,
           6,   36,   56,   67,   69,   61,   55,   28,
           2,   27,   50,   65,   64,   52,   40,   22,
          -2,   15,   35,   46,   45,   36,   19,    6,
         -23,   -2,   10,   21,   21,   12,   -4,  -21,
         -52,  -42,  -25,   -1,  -23,   -8,  -32,  -60,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
          91,   93,   89,  102,   92,   81,   35,   25,
          86,   89,   85,   96,   86,   76,   30,   19,
          35,   43,   36,   26,   23,   14,  -23,  -42,
          12,    9,   17,   15,    1,    4,  -21,  -21,
          -2,  -11,  -17,  -11,  -17,  -10,  -24,  -16,
         -10,  -20,  -23,  -19,  -18,  -14,  -22,   -1,
         -17,  -11,  -17,  -20,   -7,   -5,   -0,   -4,
         -18,  -10,  -16,  -20,   -6,   -4,    2,   -3,
    ],
    // passed pawns eg
    [
         131,  129,  128,  107,  107,  112,  133,  139,
         129,  128,  126,  104,  104,  110,  131,  137,
         107,  105,   94,   67,   68,   87,   96,  113,
          59,   56,   48,   39,   40,   46,   61,   65,
          31,   29,   26,   20,   22,   25,   38,   37,
           3,    8,   13,    5,    8,    9,   20,    8,
           3,    5,   14,   10,    0,    4,    7,    6,
           3,    5,   14,   11,    0,    3,    6,    6,
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
