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
          61,   64,   61,   70,   67,   52,   13,   -1,
          63,   76,   94,   97,   97,  148,  125,   93,
          51,   79,   73,   81,   98,   97,   94,   66,
          38,   69,   66,   83,   77,   80,   77,   48,
          39,   65,   63,   59,   71,   72,   98,   62,
          38,   70,   52,   42,   55,   91,  111,   56,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         182,  178,  174,  152,  152,  155,  182,  193,
         136,  139,  127,  131,  127,  103,  129,  122,
         126,  121,  108,   96,   97,   96,  111,  104,
         115,  116,  102,   99,  102,   99,  106,   97,
         111,  113,  100,  112,  111,  103,  103,   93,
         117,  115,  112,  116,  123,  109,  103,   95,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         136,  121,  154,  185,  280,   93,  184,  176,
         260,  296,  341,  245,  303,  340,  276,  280,
         294,  335,  335,  379,  393,  417,  350,  325,
         303,  324,  347,  353,  344,  381,  324,  345,
         295,  302,  327,  325,  332,  335,  330,  301,
         279,  302,  323,  322,  333,  327,  328,  285,
         258,  270,  292,  306,  309,  313,  296,  293,
         212,  280,  247,  269,  275,  292,  284,  232,
    ],
    // knight eg
    [
         261,  319,  335,  326,  310,  321,  293,  239,
         304,  324,  323,  355,  323,  302,  303,  295,
         311,  323,  353,  341,  325,  312,  309,  295,
         319,  344,  358,  365,  364,  350,  347,  310,
         322,  342,  361,  364,  367,  355,  335,  317,
         305,  330,  337,  355,  353,  332,  322,  311,
         293,  316,  326,  331,  329,  324,  315,  305,
         291,  273,  313,  317,  313,  302,  277,  289,
    ],
    // bishop mg
    [
         291,  182,  164,  184,  221,  172,  221,  299,
         313,  360,  336,  312,  332,  358,  331,  302,
         289,  362,  353,  376,  370,  363,  357,  339,
         329,  341,  372,  384,  382,  365,  355,  327,
         332,  343,  355,  377,  371,  358,  346,  340,
         346,  356,  358,  361,  362,  359,  356,  354,
         347,  358,  360,  343,  350,  362,  380,  352,
         319,  342,  336,  308,  315,  329,  345,  319,
    ],
    // bishop eg
    [
         331,  369,  368,  366,  356,  361,  355,  324,
         336,  345,  355,  351,  348,  332,  342,  331,
         364,  350,  360,  353,  350,  363,  344,  350,
         350,  368,  358,  368,  366,  360,  358,  343,
         341,  361,  368,  369,  364,  365,  353,  334,
         340,  355,  362,  362,  368,  359,  346,  337,
         336,  334,  339,  351,  353,  345,  337,  316,
         327,  336,  320,  348,  345,  334,  326,  317,
    ],
    // rook mg
    [
         412,  407,  391,  411,  433,  427,  442,  453,
         412,  414,  440,  459,  447,  466,  442,  462,
         394,  419,  419,  433,  454,  464,  482,  455,
         383,  401,  410,  416,  422,  441,  443,  443,
         393,  384,  386,  400,  403,  408,  425,  425,
         385,  391,  390,  397,  407,  418,  455,  431,
         388,  394,  396,  400,  408,  424,  442,  387,
         421,  409,  409,  418,  425,  437,  423,  434,
    ],
    // rook eg
    [
         633,  637,  643,  628,  621,  624,  622,  622,
         640,  644,  638,  625,  621,  610,  620,  615,
         637,  630,  629,  619,  602,  602,  598,  602,
         633,  628,  628,  621,  612,  604,  602,  600,
         616,  622,  624,  618,  611,  611,  600,  595,
         610,  607,  611,  610,  604,  599,  578,  579,
         603,  603,  610,  608,  601,  595,  580,  597,
         612,  610,  620,  614,  606,  604,  603,  589,
    ],
    // queen mg
    [
         863,  832,  833,  894,  903,  940,  947,  934,
         902,  902,  890,  868,  887,  938,  932,  950,
         900,  902,  903,  922,  940,  958,  972,  972,
         895,  906,  911,  904,  916,  924,  930,  935,
         908,  900,  913,  912,  921,  920,  927,  933,
         907,  928,  919,  920,  923,  934,  943,  930,
         904,  923,  934,  933,  933,  941,  946,  942,
         911,  899,  910,  937,  914,  895,  903,  920,
    ],
    // queen eg
    [
        1159, 1213, 1232, 1201, 1208, 1174, 1144, 1146,
        1148, 1166, 1217, 1244, 1254, 1204, 1179, 1158,
        1144, 1169, 1199, 1201, 1203, 1208, 1162, 1145,
        1145, 1171, 1184, 1216, 1224, 1207, 1188, 1171,
        1138, 1177, 1172, 1210, 1190, 1189, 1179, 1147,
        1120, 1123, 1162, 1157, 1162, 1151, 1133, 1118,
        1120, 1118, 1107, 1119, 1119, 1099, 1086, 1058,
        1101, 1112, 1110, 1076, 1108, 1107, 1100, 1061,
    ],
    // king mg
    [
          80,   36,   44,   11,   20,   14,   57,   99,
         -47,    3,    1,   -2,   22,  -26,    2,  -19,
         -41,   21,  -18,  -12,  -20,   49,    6,  -17,
         -49,  -49,  -64,  -93, -101,  -76,  -77, -123,
         -74,  -57,  -93, -115, -115, -110,  -99, -140,
         -52,  -29,  -82,  -97,  -84,  -87,  -44,  -59,
          27,   -6,  -30,  -76,  -72,  -37,   11,   14,
           6,   51,   28,  -75,   -4,  -66,   34,   23,
    ],
    // king eg
    [
        -107,  -37,  -34,  -17,  -21,   -9,  -14,  -82,
         -14,   21,   25,   20,   29,   43,   42,    3,
           3,   31,   42,   46,   47,   48,   53,   13,
          -7,   36,   51,   64,   65,   58,   52,   23,
         -11,   22,   49,   66,   65,   54,   35,   16,
         -13,   12,   37,   52,   50,   39,   18,   -2,
         -32,   -1,   14,   30,   31,   16,   -4,  -27,
         -62,  -51,  -31,   -3,  -32,  -10,  -41,  -75,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          61,   64,   61,   70,   67,   52,   13,   -1,
          29,   38,   37,   30,   37,   11,  -31,  -50,
          10,   10,   18,   19,    7,    6,  -22,  -20,
           8,  -12,  -16,   -7,  -13,  -13,  -23,   -5,
          -1,  -15,  -21,  -12,   -9,   -8,  -21,   13,
          -8,   -5,   -2,  -10,   10,    5,   12,    8,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         182,  178,  174,  152,  152,  155,  182,  193,
         128,  117,   91,   55,   51,   91,  104,  130,
          61,   58,   45,   37,   36,   46,   63,   66,
          30,   28,   24,   15,   18,   25,   40,   36,
          -2,    4,   10,   -2,    2,    7,   18,    2,
          -0,    3,    3,    0,  -10,   -3,    3,    4,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 41;
const ROOK_OPEN_FILE_EG: i32 = 5;
const ROOK_CLOSED_FILE_MG: i32 = -19;
const ROOK_CLOSED_FILE_EG: i32 = -3;
const ROOK_SEMIOPEN_FILE_MG: i32 = 7;
const ROOK_SEMIOPEN_FILE_EG: i32 = 21;
const KING_OPEN_FILE_MG: i32 = -68;
const KING_OPEN_FILE_EG: i32 = -12;
const KING_CLOSED_FILE_MG: i32 = 18;
const KING_CLOSED_FILE_EG: i32 = -17;
const KING_SEMIOPEN_FILE_MG: i32 = -28;
const KING_SEMIOPEN_FILE_EG: i32 = 9;

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
