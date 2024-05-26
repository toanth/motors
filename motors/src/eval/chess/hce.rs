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
          87,   86,   87,  100,   91,   78,   32,   24,
          77,   91,  113,  118,  126,  170,  150,  115,
          66,   89,   86,   91,  111,  108,  105,   87,
          53,   81,   76,   94,   90,   89,   90,   66,
          53,   76,   72,   71,   83,   81,  105,   79,
          52,   77,   66,   58,   72,  101,  117,   73,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         132,  130,  127,  105,  106,  113,  135,  139,
         126,  124,  102,  100,   95,   85,  116,  110,
         118,  113,   98,   86,   87,   87,  103,   94,
         108,  109,   95,   90,   92,   92,   99,   90,
         104,  106,   93,  102,  101,   95,   97,   86,
         109,  109,  100,  103,  110,   99,   95,   87,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         159,  224,  269,  293,  345,  242,  250,  210,
         295,  323,  360,  374,  363,  417,  333,  338,
         325,  364,  383,  399,  433,  442,  387,  352,
         329,  345,  367,  393,  369,  396,  348,  364,
         315,  332,  348,  349,  359,  354,  353,  324,
         297,  321,  337,  342,  352,  339,  345,  311,
         283,  294,  314,  325,  327,  331,  319,  314,
         246,  294,  278,  295,  300,  316,  297,  267,
    ],
    // knight eg
    [
         265,  304,  326,  318,  313,  310,  305,  235,
         307,  323,  325,  326,  319,  304,  314,  290,
         313,  325,  342,  344,  328,  324,  319,  306,
         323,  344,  355,  356,  359,  352,  345,  314,
         326,  337,  356,  360,  361,  349,  335,  318,
         310,  329,  336,  349,  347,  333,  321,  310,
         302,  319,  325,  328,  327,  321,  309,  307,
         288,  282,  311,  314,  313,  303,  286,  284,
    ],
    // bishop mg
    [
         341,  309,  317,  296,  293,  305,  339,  317,
         344,  379,  370,  354,  381,  377,  371,  338,
         359,  388,  387,  408,  403,  424,  403,  392,
         353,  372,  393,  406,  401,  395,  373,  358,
         350,  366,  373,  395,  392,  373,  368,  358,
         361,  372,  372,  375,  377,  372,  372,  375,
         364,  368,  377,  358,  365,  378,  389,  369,
         341,  363,  348,  338,  341,  341,  366,  351,
    ],
    // bishop eg
    [
         332,  345,  342,  351,  349,  343,  338,  334,
         329,  340,  345,  346,  340,  340,  343,  328,
         346,  344,  353,  345,  349,  352,  344,  341,
         347,  360,  353,  364,  362,  358,  358,  343,
         343,  356,  363,  361,  360,  360,  353,  334,
         339,  350,  356,  356,  361,  354,  340,  331,
         336,  333,  334,  347,  349,  337,  334,  317,
         321,  331,  319,  340,  335,  335,  319,  311,
    ],
    // rook mg
    [
         473,  465,  469,  475,  489,  498,  508,  515,
         447,  447,  469,  487,  473,  509,  504,  518,
         433,  456,  455,  458,  488,  502,  532,  501,
         424,  435,  440,  450,  452,  464,  471,  472,
         420,  419,  418,  431,  432,  431,  454,  446,
         416,  419,  420,  425,  434,  440,  476,  456,
         417,  422,  427,  429,  436,  449,  461,  429,
         440,  434,  433,  441,  448,  453,  451,  447,
    ],
    // rook eg
    [
         607,  615,  618,  612,  608,  607,  604,  600,
         617,  623,  620,  610,  613,  603,  602,  591,
         614,  611,  611,  607,  595,  592,  586,  587,
         613,  610,  612,  606,  599,  596,  593,  586,
         604,  605,  607,  603,  600,  599,  589,  584,
         596,  597,  595,  595,  591,  586,  568,  570,
         593,  594,  595,  592,  586,  581,  574,  581,
         596,  594,  602,  595,  588,  590,  585,  580,
    ],
    // queen mg
    [
         905,  919,  943,  978,  973,  993, 1016,  953,
         936,  921,  930,  918,  925,  967,  951,  991,
         941,  940,  941,  960,  972, 1012, 1012, 1001,
         931,  935,  942,  941,  945,  957,  954,  965,
         934,  934,  936,  940,  945,  943,  957,  955,
         931,  944,  940,  941,  944,  951,  962,  953,
         931,  943,  951,  951,  951,  961,  966,  966,
         933,  925,  931,  948,  937,  925,  932,  933,
    ],
    // queen eg
    [
        1153, 1165, 1177, 1164, 1169, 1150, 1109, 1150,
        1125, 1159, 1189, 1212, 1224, 1183, 1166, 1133,
        1122, 1146, 1180, 1179, 1193, 1169, 1134, 1128,
        1131, 1157, 1168, 1191, 1203, 1189, 1176, 1150,
        1129, 1155, 1160, 1185, 1173, 1168, 1151, 1139,
        1116, 1123, 1150, 1145, 1148, 1140, 1122, 1111,
        1113, 1113, 1106, 1114, 1116, 1089, 1067, 1048,
        1103, 1107, 1112, 1096, 1107, 1098, 1084, 1079,
    ],
    // king mg
    [
          44,   17,   46,  -28,    3,   12,   36,  114,
         -68,   -8,  -24,   18,    9,    0,    3,  -15,
         -71,   29,  -37,  -44,  -25,   30,   11,  -28,
         -63,  -59,  -70,  -98, -101,  -77,  -81, -104,
         -79,  -68,  -94, -116, -118, -107, -105, -126,
         -55,  -34,  -83,  -89,  -83,  -89,  -51,  -67,
          29,   -4,  -22,  -57,  -59,  -33,    6,   13,
          12,   47,   24,  -67,   -5,  -58,   27,   23,
    ],
    // king eg
    [
         -91,  -33,  -27,    2,   -9,   -1,   -5,  -79,
          -6,   22,   28,   20,   29,   43,   39,    7,
           6,   26,   43,   51,   49,   47,   47,   19,
          -1,   33,   50,   61,   62,   55,   48,   22,
          -6,   24,   46,   62,   60,   48,   35,   18,
          -6,   14,   35,   45,   43,   34,   16,    4,
         -24,    0,   12,   24,   23,   12,   -5,  -23,
         -53,  -45,  -27,   -1,  -30,   -8,  -37,  -65,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          87,   86,   87,  100,   91,   78,   32,   24,
          33,   39,   28,   15,   15,    3,  -40,  -55,
          11,    8,   17,   16,    1,    6,  -24,  -20,
          -1,  -13,  -19,  -13,  -18,  -11,  -27,  -10,
         -10,  -22,  -24,  -21,  -17,  -12,  -21,    5,
         -17,  -10,  -17,  -23,   -4,   -2,    7,   -4,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         132,  130,  127,  105,  106,  113,  135,  139,
         112,  110,   96,   69,   70,   93,  101,  121,
          57,   54,   44,   38,   39,   45,   61,   63,
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
