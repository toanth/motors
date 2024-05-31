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
          48,   68,   58,   43,   62,   76,   94,   52,
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
         267,  289,  321,  337,  316,  375,  290,  303,
         287,  324,  345,  356,  388,  392,  346,  309,
         289,  306,  330,  352,  333,  357,  312,  322,
         278,  293,  310,  310,  320,  314,  315,  290,
         259,  283,  296,  303,  314,  300,  305,  274,
         246,  257,  276,  287,  288,  293,  281,  278,
         212,  253,  243,  259,  265,  279,  257,  234,
    ],
    // knight EG
    [
         257,  308,  328,  317,  315,  310,  304,  236,
         305,  321,  324,  325,  323,  306,  316,  290,
         313,  325,  339,  342,  329,  325,  319,  307,
         322,  341,  352,  353,  355,  349,  341,  314,
         324,  335,  353,  357,  358,  347,  333,  315,
         309,  327,  336,  347,  345,  332,  320,  310,
         302,  319,  324,  326,  324,  319,  308,  306,
         289,  282,  309,  312,  312,  300,  288,  285,
    ],
    // bishop MG
    [
         291,  272,  276,  253,  261,  261,  306,  278,
         310,  335,  333,  314,  342,  342,  333,  311,
         322,  347,  346,  367,  363,  384,  364,  350,
         316,  334,  353,  367,  361,  356,  334,  320,
         313,  326,  333,  354,  349,  334,  328,  322,
         319,  332,  331,  335,  336,  331,  333,  332,
         325,  326,  338,  317,  324,  336,  344,  328,
         303,  325,  307,  299,  304,  305,  321,  311,
    ],
    // bishop EG
    [
         332,  344,  340,  350,  346,  341,  336,  331,
         325,  338,  342,  345,  337,  336,  340,  324,
         343,  341,  349,  342,  346,  348,  340,  340,
         342,  356,  350,  359,  357,  355,  355,  341,
         340,  353,  359,  357,  356,  356,  351,  331,
         338,  347,  353,  352,  358,  351,  338,  330,
         333,  332,  332,  344,  346,  335,  332,  315,
         319,  329,  317,  337,  332,  332,  319,  307,
    ],
    // rook MG
    [
         419,  407,  413,  419,  431,  441,  451,  460,
         395,  398,  416,  432,  420,  454,  448,  460,
         380,  403,  403,  405,  434,  446,  475,  440,
         373,  383,  388,  396,  402,  412,  415,  414,
         367,  368,  368,  381,  382,  382,  399,  393,
         365,  366,  369,  373,  383,  388,  422,  401,
         367,  370,  376,  378,  385,  398,  410,  380,
         388,  382,  381,  389,  396,  400,  399,  395,
    ],
    // rook EG
    [
         597,  606,  609,  603,  600,  599,  597,  592,
         607,  612,  611,  601,  604,  594,  593,  583,
         605,  602,  602,  598,  587,  583,  578,  580,
         604,  601,  603,  597,  589,  586,  586,  579,
         596,  596,  597,  593,  590,  589,  582,  576,
         587,  588,  586,  586,  582,  577,  560,  561,
         583,  585,  585,  582,  576,  570,  563,  570,
         586,  584,  592,  586,  579,  582,  574,  571,
    ],
    // queen MG
    [
         797,  813,  834,  863,  860,  876,  910,  849,
         829,  813,  823,  816,  822,  861,  850,  884,
         834,  830,  837,  847,  863,  899,  898,  884,
         823,  828,  834,  835,  838,  848,  845,  855,
         828,  825,  829,  834,  836,  835,  847,  846,
         823,  834,  832,  831,  835,  841,  852,  845,
         823,  833,  842,  841,  840,  850,  856,  854,
         824,  816,  821,  836,  829,  818,  822,  823,
    ],
    // queen EG
    [
        1138, 1146, 1161, 1151, 1156, 1140, 1095, 1129,
        1108, 1145, 1172, 1189, 1203, 1163, 1145, 1120,
        1108, 1131, 1159, 1166, 1174, 1156, 1125, 1121,
        1116, 1139, 1153, 1173, 1185, 1172, 1162, 1136,
        1109, 1140, 1144, 1164, 1157, 1153, 1136, 1125,
        1101, 1113, 1133, 1130, 1133, 1125, 1107, 1096,
        1097, 1098, 1092, 1100, 1103, 1077, 1056, 1035,
        1087, 1093, 1099, 1088, 1090, 1083, 1069, 1064,
    ],
    // king MG
    [
         120,   36,   64,  -26,   13,   18,   65,  192,
           7,    1,  -19,   48,   25,    4,   37,   38,
          -9,   33,  -21,  -35,    3,   51,   28,    6,
           7,  -39,  -57,  -81,  -79,  -55,  -61,  -68,
         -15,  -51,  -74,  -95,  -96,  -79,  -82,  -92,
          -0,  -13,  -60,  -68,  -62,  -65,  -34,  -41,
          59,   10,   -2,  -32,  -32,  -18,   25,   30,
          24,   43,   36,  -37,   21,  -35,   33,   38,
    ],
    // king EG
    [
         -96,  -52,  -45,  -12,  -26,  -18,  -24, -121,
          -8,    7,   14,    2,   14,   27,   19,  -25,
           7,   11,   27,   37,   32,   29,   28,   -8,
           1,   17,   34,   45,   45,   38,   31,   -4,
          -1,    8,   29,   45,   42,   29,   17,   -5,
           1,   -3,   16,   26,   25,   15,   -1,  -15,
         -10,  -16,   -7,    3,    3,   -6,  -24,  -32,
         -31,  -56,  -45,  -21,  -48,  -26,  -51,  -71,
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
         104,  102,   88,   58,   64,   83,   90,  110,
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
    [-98, -37],
    [-81, -42],
    [-66, -46],
    [-47, -37],
    [-62, -47],
    [-66, -63],
    [-48, -45],
    [-27, -68],
    [-81, -42],
    [-89, -74],
    [-63, -35],
    [-38, -61],
    [-62, -54],
    [-75, -83],
    [-32, -40],
    [-50, -50],
    [-99, -40],
    [-51, -37],
    [-82, -77],
    [-44, -63],
    [-58, -43],
    [-23, -45],
    [-63, -74],
    [-50, -50],
    [-83, -17],
    [-51, -46],
    [-46, -38],
    [-104, -3],
    [-44, -41],
    [-43, -70],
    [-72, -36],
    [-50, -50],
    [-68, -43],
    [-63, -45],
    [-43, -45],
    [-22, -45],
    [-83, -85],
    [-74, -101],
    [-58, -62],
    [-50, -50],
    [-80, -60],
    [-96, -70],
    [-41, -49],
    [-41, -89],
    [-94, -90],
    [-87, -112],
    [-53, -65],
    [-50, -50],
    [-76, -32],
    [-35, -37],
    [-53, -74],
    [2, -59],
    [-57, -52],
    [-44, -12],
    [-32, -63],
    [-50, -50],
    [-84, -38],
    [-50, -50],
    [-50, -50],
    [-50, -50],
    [-50, -50],
    [-50, -50],
    [-47, -57],
    [-31, -98],
    [-145, -53],
    [-80, -81],
    [-58, -72],
    [-32, -103],
    [-85, -76],
    [-121, -124],
    [-19, -104],
    [-50, -50],
    [-62, -72],
    [-73, -94],
    [-30, -164],
    [-50, -50],
    [-67, -83],
    [-50, -50],
    [-28, -112],
    [-57, -109],
    [-100, -31],
    [-64, -54],
    [-59, -53],
    [-37, -76],
    [-83, -47],
    [-59, -102],
    [-63, -60],
    [-50, -50],
    [-87, -44],
    [-36, -53],
    [-54, -117],
    [-50, -50],
    [-80, -57],
    [-50, -50],
    [-50, -50],
    [-50, -50],
];

// TODO: Differentiate between rooks and kings in front of / behind pawns?

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
