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
          64,   68,   64,   74,   67,   55,   12,   -0,
          65,   77,   95,   97,   97,  148,  129,   94,
          51,   78,   72,   79,   98,   96,   93,   68,
          38,   67,   65,   81,   76,   79,   77,   52,
          38,   64,   60,   57,   69,   72,   95,   62,
          37,   66,   50,   39,   52,   88,  107,   54,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         178,  175,  171,  148,  149,  152,  180,  190,
         137,  141,  126,  133,  127,  104,  131,  125,
         129,  124,  109,   98,   99,   97,  113,  105,
         117,  119,  104,  102,  103,  101,  109,   98,
         113,  116,  102,  114,  113,  104,  106,   95,
         119,  119,  113,  118,  127,  111,  106,   97,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         131,  111,  145,  178,  268,   90,  180,  166,
         251,  283,  324,  240,  292,  329,  262,  268,
         279,  320,  324,  362,  379,  400,  335,  311,
         285,  306,  331,  338,  328,  364,  309,  327,
         277,  286,  310,  307,  314,  316,  312,  286,
         259,  284,  302,  303,  315,  308,  309,  267,
         239,  252,  273,  287,  290,  293,  277,  274,
         196,  258,  231,  251,  258,  275,  265,  218,
    ],
    // knight eg
    [
         266,  334,  351,  339,  326,  335,  307,  250,
         317,  338,  338,  367,  338,  318,  320,  310,
         325,  338,  366,  356,  341,  328,  325,  309,
         334,  358,  372,  379,  378,  365,  361,  325,
         337,  356,  375,  379,  381,  369,  350,  330,
         320,  344,  353,  369,  367,  346,  337,  327,
         308,  331,  341,  345,  343,  338,  328,  321,
         305,  287,  326,  329,  326,  315,  292,  302,
    ],
    // bishop mg
    [
         262,  171,  162,  170,  213,  163,  216,  277,
         299,  338,  320,  297,  317,  341,  312,  292,
         279,  346,  337,  361,  354,  353,  344,  325,
         313,  324,  354,  367,  364,  351,  337,  311,
         314,  324,  336,  357,  351,  338,  328,  324,
         324,  336,  338,  341,  342,  338,  337,  335,
         328,  335,  341,  322,  330,  342,  357,  333,
         299,  325,  315,  289,  298,  310,  327,  301,
    ],
    // bishop eg
    [
         348,  383,  379,  380,  369,  373,  368,  338,
         349,  359,  369,  366,  361,  347,  357,  344,
         376,  363,  373,  365,  365,  376,  358,  365,
         362,  381,  372,  382,  380,  375,  374,  358,
         355,  376,  382,  383,  379,  379,  368,  348,
         355,  369,  377,  375,  382,  374,  360,  352,
         350,  350,  353,  365,  367,  358,  353,  330,
         341,  349,  333,  361,  357,  348,  339,  329,
    ],
    // rook mg
    [
         394,  386,  375,  393,  414,  410,  421,  435,
         390,  393,  416,  435,  423,  445,  424,  443,
         371,  397,  397,  408,  432,  441,  464,  434,
         362,  379,  385,  392,  400,  416,  418,  417,
         367,  361,  364,  377,  379,  381,  400,  399,
         361,  365,  365,  372,  382,  392,  429,  405,
         364,  369,  373,  376,  384,  398,  415,  365,
         395,  384,  383,  392,  399,  410,  399,  406,
    ],
    // rook eg
    [
         649,  655,  660,  647,  640,  643,  642,  640,
         658,  661,  656,  644,  641,  630,  638,  633,
         656,  648,  648,  638,  622,  622,  616,  620,
         651,  647,  647,  640,  630,  623,  622,  619,
         635,  641,  641,  636,  629,  631,  620,  614,
         629,  626,  629,  628,  623,  618,  597,  598,
         621,  621,  627,  625,  618,  613,  598,  615,
         630,  628,  638,  632,  623,  622,  620,  610,
    ],
    // queen mg
    [
         800,  780,  784,  840,  847,  879,  895,  875,
         840,  836,  829,  812,  828,  877,  869,  889,
         841,  840,  844,  856,  878,  898,  910,  906,
         833,  843,  848,  841,  853,  862,  867,  872,
         846,  836,  848,  848,  855,  855,  862,  870,
         843,  862,  854,  854,  858,  868,  878,  867,
         841,  857,  869,  868,  867,  874,  880,  878,
         846,  834,  844,  870,  850,  833,  843,  854,
    ],
    // queen eg
    [
        1206, 1250, 1270, 1243, 1251, 1219, 1185, 1187,
        1190, 1215, 1261, 1284, 1296, 1246, 1224, 1204,
        1187, 1213, 1241, 1249, 1247, 1252, 1208, 1194,
        1190, 1216, 1231, 1262, 1269, 1251, 1234, 1216,
        1181, 1221, 1217, 1254, 1236, 1234, 1223, 1194,
        1165, 1172, 1206, 1202, 1207, 1196, 1176, 1162,
        1164, 1163, 1152, 1163, 1165, 1145, 1129, 1100,
        1145, 1158, 1157, 1127, 1152, 1151, 1141, 1108,
    ],
    // king mg
    [
          77,   25,   34,  -13,    6,   -6,   54,  127,
         -59,  -20,  -25,   -5,    7,  -52,   -2,  -11,
         -64,   -3,  -36,  -38,  -28,   36,  -11,  -26,
         -63,  -66,  -85, -110, -116,  -90,  -95, -130,
         -88,  -73, -107, -124, -125, -116, -109, -144,
         -60,  -36,  -87,  -99,  -87,  -92,  -50,  -64,
          25,   -8,  -30,  -70,  -70,  -38,    8,   12,
           8,   49,   27,  -77,   -6,  -65,   32,   23,
    ],
    // king eg
    [
        -110,  -36,  -32,  -10,  -18,   -6,  -12,  -94,
         -10,   28,   33,   24,   35,   50,   45,    2,
           9,   38,   49,   56,   53,   53,   58,   16,
          -1,   43,   58,   70,   72,   63,   58,   26,
          -5,   28,   54,   70,   68,   56,   39,   18,
          -9,   16,   39,   53,   51,   40,   19,   -0,
         -30,    1,   14,   29,   30,   16,   -4,  -26,
         -62,  -50,  -30,   -2,  -31,  -10,  -40,  -73,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          64,   68,   64,   74,   67,   55,   12,   -0,
          30,   39,   38,   33,   40,   13,  -26,  -47,
          11,   10,   20,   20,    6,    5,  -22,  -19,
           6,  -10,  -16,   -7,  -13,  -11,  -22,  -10,
          -2,  -17,  -21,  -10,  -11,  -11,  -23,   10,
          -9,   -5,   -4,   -7,   10,    4,    7,    6,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         178,  175,  171,  148,  149,  152,  180,  190,
         124,  115,   90,   52,   50,   90,  101,  126,
          60,   57,   45,   37,   37,   47,   63,   66,
          30,   28,   24,   15,   18,   25,   40,   37,
          -2,    3,    9,   -3,    2,    7,   19,    2,
          -0,    2,    5,    1,  -11,   -3,    4,    4,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 39;
const ROOK_OPEN_FILE_EG: i32 = 7;
const ROOK_CLOSED_FILE_MG: i32 = -19;
const ROOK_CLOSED_FILE_EG: i32 = -2;
const ROOK_SEMIOPEN_FILE_MG: i32 = 7;
const ROOK_SEMIOPEN_FILE_EG: i32 = 20;
const KING_OPEN_FILE_MG: i32 = -71;
const KING_OPEN_FILE_EG: i32 = -11;
const KING_CLOSED_FILE_MG: i32 = 19;
const KING_CLOSED_FILE_EG: i32 = -17;
const KING_SEMIOPEN_FILE_MG: i32 = -28;
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
