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
         157,  163,  156,  183,  170,  149,   70,   47,
          77,   91,  114,  119,  126,  170,  150,  115,
          66,   89,   86,   91,  111,  108,  105,   87,
          53,   81,   76,   94,   90,   89,   90,   66,
          53,   76,   72,   71,   83,   81,  105,   79,
          52,   77,   66,   58,   72,  101,  117,   73,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         276,  272,  265,  221,  219,  232,  278,  287,
         126,  124,  102,  100,   95,   85,  116,  110,
         118,  113,   98,   86,   87,   87,  103,   94,
         108,  109,   95,   90,   92,   92,   99,   90,
         104,  106,   93,  102,  101,   95,   97,   86,
         109,  109,  100,  103,  110,   99,   95,   87,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         159,  224,  269,  293,  346,  242,  250,  210,
         295,  324,  361,  374,  363,  417,  333,  338,
         325,  364,  383,  399,  434,  442,  387,  352,
         329,  345,  367,  393,  369,  397,  348,  364,
         315,  332,  349,  349,  359,  354,  354,  324,
         297,  321,  337,  342,  352,  339,  345,  311,
         283,  294,  314,  325,  327,  331,  319,  314,
         246,  294,  278,  295,  300,  316,  298,  267,
    ],
    // knight eg
    [
         265,  304,  326,  318,  313,  310,  304,  235,
         306,  323,  324,  326,  319,  304,  313,  290,
         312,  325,  342,  344,  328,  324,  319,  306,
         323,  343,  355,  356,  359,  352,  345,  314,
         326,  337,  356,  360,  361,  349,  335,  318,
         310,  329,  336,  349,  347,  333,  321,  310,
         302,  319,  325,  328,  327,  321,  309,  307,
         288,  282,  311,  314,  313,  302,  286,  284,
    ],
    // bishop mg
    [
         341,  309,  317,  296,  293,  305,  339,  317,
         344,  379,  370,  354,  381,  377,  371,  339,
         359,  388,  387,  408,  403,  424,  403,  392,
         353,  372,  393,  407,  401,  395,  374,  358,
         351,  366,  373,  395,  392,  374,  368,  358,
         361,  372,  372,  375,  377,  373,  373,  376,
         364,  368,  377,  358,  365,  378,  389,  369,
         342,  363,  348,  338,  341,  341,  366,  352,
    ],
    // bishop eg
    [
         332,  345,  342,  351,  349,  343,  338,  334,
         329,  340,  345,  346,  340,  339,  343,  328,
         346,  344,  353,  345,  348,  351,  344,  340,
         346,  360,  353,  363,  361,  357,  358,  343,
         343,  356,  363,  361,  359,  360,  352,  334,
         339,  350,  356,  356,  361,  354,  340,  331,
         336,  333,  334,  347,  349,  337,  334,  317,
         321,  331,  319,  340,  335,  335,  319,  310,
    ],
    // rook mg
    [
         473,  465,  469,  475,  490,  499,  509,  516,
         447,  447,  469,  487,  474,  509,  504,  518,
         433,  456,  455,  458,  488,  502,  532,  501,
         424,  436,  440,  450,  452,  465,  472,  472,
         420,  420,  418,  431,  433,  432,  454,  446,
         417,  419,  420,  426,  434,  441,  476,  456,
         417,  422,  427,  429,  437,  449,  461,  430,
         441,  435,  434,  442,  448,  453,  451,  447,
    ],
    // rook eg
    [
         607,  614,  618,  611,  608,  607,  603,  600,
         616,  623,  620,  610,  612,  603,  602,  591,
         614,  610,  611,  607,  595,  591,  586,  586,
         612,  610,  612,  606,  599,  595,  593,  586,
         604,  605,  607,  603,  599,  599,  589,  584,
         596,  597,  595,  595,  591,  585,  568,  569,
         593,  594,  595,  592,  586,  581,  574,  581,
         596,  593,  601,  595,  587,  590,  585,  580,
    ],
    // queen mg
    [
         908,  921,  946,  981,  976,  996, 1019,  955,
         938,  923,  932,  920,  928,  969,  954,  993,
         944,  942,  943,  962,  974, 1014, 1015, 1003,
         933,  937,  944,  943,  948,  960,  957,  967,
         936,  936,  938,  942,  948,  946,  959,  957,
         933,  946,  942,  943,  946,  953,  964,  955,
         933,  945,  953,  954,  953,  963,  969,  968,
         936,  927,  933,  950,  939,  927,  934,  935,
    ],
    // queen eg
    [
        1151, 1163, 1175, 1162, 1167, 1148, 1107, 1148,
        1124, 1157, 1188, 1210, 1222, 1181, 1164, 1131,
        1120, 1144, 1178, 1177, 1191, 1167, 1132, 1126,
        1129, 1155, 1166, 1189, 1201, 1187, 1174, 1148,
        1127, 1153, 1158, 1183, 1171, 1166, 1149, 1137,
        1114, 1122, 1148, 1143, 1146, 1138, 1120, 1109,
        1111, 1111, 1104, 1113, 1114, 1088, 1065, 1046,
        1101, 1105, 1110, 1095, 1105, 1096, 1082, 1078,
    ],
    // king mg
    [
          59,   32,   62,  -13,   18,   27,   51,  130,
         -53,    8,   -9,   33,   24,   15,   18,    1,
         -55,   45,  -22,  -28,   -9,   46,   26,  -13,
         -47,  -43,  -55,  -83,  -86,  -62,  -66,  -88,
         -63,  -53,  -79, -101, -102,  -92,  -89, -111,
         -39,  -18,  -68,  -73,  -68,  -74,  -36,  -52,
          44,   12,   -7,  -41,  -43,  -18,   22,   28,
          28,   63,   40,  -51,   11,  -43,   42,   38,
    ],
    // king eg
    [
        -101,  -42,  -36,   -8,  -18,  -10,  -14,  -88,
         -15,   13,   18,   11,   20,   33,   30,   -2,
          -3,   17,   34,   41,   40,   38,   38,    9,
         -10,   23,   41,   52,   53,   46,   39,   13,
         -15,   15,   37,   53,   51,   39,   26,    9,
         -15,    5,   25,   36,   34,   25,    7,   -5,
         -33,   -9,    3,   15,   14,    3,  -14,  -32,
         -62,  -54,  -36,  -10,  -39,  -17,  -46,  -74,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          18,    9,   17,   17,   12,    7,   -6,    1,
          33,   40,   28,   15,   15,    3,  -40,  -55,
          11,    8,   17,   16,    1,    6,  -24,  -20,
          -1,  -13,  -19,  -13,  -18,  -11,  -27,  -10,
         -10,  -22,  -24,  -21,  -17,  -12,  -21,    5,
         -17,  -10,  -17,  -23,   -4,   -2,    7,   -4,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         -13,  -11,  -12,  -11,   -6,   -6,   -7,   -9,
         112,  110,   96,   69,   70,   93,  101,  121,
          57,   54,   44,   38,   39,   45,   60,   63,
          30,   28,   25,   20,   21,   24,   38,   34,
           1,    6,   13,    4,    7,    9,   19,    5,
           3,    6,   14,    8,    0,    3,    5,    6,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 32;
const ROOK_OPEN_FILE_EG: i32 = 11;
const ROOK_CLOSED_FILE_MG: i32 = -16;
const ROOK_CLOSED_FILE_EG: i32 = -4;
const ROOK_SEMIOPEN_FILE_MG: i32 = 6;
const ROOK_SEMIOPEN_FILE_EG: i32 = 12;
const KING_OPEN_FILE_MG: i32 = -72;
const KING_OPEN_FILE_EG: i32 = -10;
const KING_CLOSED_FILE_MG: i32 = 16;
const KING_CLOSED_FILE_EG: i32 = -16;
const KING_SEMIOPEN_FILE_MG: i32 = -33;
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
