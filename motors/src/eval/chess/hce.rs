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
         156,  166,  154,  181,  166,  148,   67,   46,
          74,   84,  104,  110,  117,  157,  140,  106,
          60,   82,   78,   83,  103,   99,   96,   81,
          49,   74,   70,   85,   83,   82,   82,   66,
          48,   70,   64,   64,   76,   75,   96,   73,
          47,   68,   60,   50,   66,   91,  105,   64,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         268,  263,  260,  215,  214,  225,  271,  280,
         122,  121,  100,  100,   93,   84,  114,  108,
         114,  110,   95,   85,   85,   85,  101,   92,
         104,  106,   92,   89,   90,   89,   97,   86,
         102,  104,   91,  100,   98,   92,   95,   84,
         106,  107,   97,  100,  108,   97,   94,   86,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         144,  193,  236,  268,  311,  220,  229,  188,
         270,  289,  321,  337,  322,  377,  296,  308,
         290,  324,  344,  355,  388,  397,  346,  316,
         288,  304,  328,  351,  331,  355,  311,  323,
         277,  293,  309,  309,  318,  313,  313,  288,
         258,  282,  294,  301,  312,  299,  304,  274,
         244,  256,  274,  285,  287,  291,  280,  274,
         216,  253,  242,  257,  263,  278,  258,  240,
    ],
    // knight eg
    [
         258,  316,  332,  320,  319,  316,  315,  240,
         306,  322,  324,  326,  322,  306,  318,  294,
         314,  325,  339,  342,  328,  324,  320,  307,
         322,  341,  352,  353,  356,  350,  342,  314,
         324,  335,  353,  357,  358,  347,  334,  315,
         309,  327,  336,  347,  345,  332,  321,  310,
         304,  320,  324,  326,  325,  320,  309,  309,
         296,  282,  310,  313,  312,  300,  288,  291,
    ],
    // bishop mg
    [
         293,  277,  282,  255,  264,  265,  312,  280,
         307,  334,  330,  316,  339,  336,  330,  305,
         320,  346,  345,  365,  359,  380,  360,  348,
         314,  332,  350,  364,  358,  354,  331,  317,
         310,  323,  330,  351,  347,  331,  326,  319,
         317,  329,  328,  332,  333,  329,  330,  332,
         322,  323,  335,  314,  322,  335,  342,  327,
         300,  322,  305,  296,  301,  302,  328,  311,
    ],
    // bishop eg
    [
         334,  346,  342,  351,  348,  343,  340,  335,
         326,  338,  343,  346,  338,  338,  341,  326,
         343,  342,  350,  342,  347,  349,  341,  340,
         343,  356,  350,  360,  358,  355,  356,  342,
         340,  354,  360,  358,  357,  357,  351,  331,
         338,  347,  354,  352,  358,  352,  338,  330,
         333,  332,  332,  344,  346,  335,  334,  316,
         320,  329,  318,  337,  332,  332,  321,  310,
    ],
    // rook mg
    [
         418,  409,  413,  419,  432,  444,  453,  462,
         394,  396,  414,  430,  418,  451,  446,  461,
         379,  402,  402,  403,  432,  444,  475,  444,
         372,  382,  385,  395,  399,  409,  415,  415,
         366,  367,  366,  379,  380,  377,  398,  393,
         364,  365,  367,  372,  380,  386,  420,  401,
         365,  369,  375,  376,  383,  394,  405,  379,
         387,  381,  379,  387,  394,  398,  398,  390,
    ],
    // rook eg
    [
         597,  606,  609,  603,  600,  600,  598,  592,
         607,  612,  611,  601,  604,  595,  594,  583,
         605,  602,  602,  598,  587,  584,  577,  579,
         603,  601,  603,  597,  589,  586,  585,  578,
         595,  596,  597,  593,  590,  591,  581,  576,
         587,  588,  586,  586,  582,  577,  560,  561,
         583,  585,  585,  582,  576,  572,  565,  571,
         586,  584,  592,  586,  578,  581,  575,  574,
    ],
    // queen mg
    [
         796,  815,  838,  868,  864,  889,  918,  851,
         826,  809,  819,  814,  820,  856,  844,  879,
         831,  828,  833,  844,  858,  895,  895,  883,
         820,  824,  829,  830,  834,  844,  842,  851,
         824,  821,  824,  830,  832,  830,  843,  843,
         820,  831,  827,  827,  831,  837,  848,  841,
         820,  829,  838,  837,  836,  846,  852,  859,
         821,  812,  817,  833,  825,  814,  834,  837,
    ],
    // queen eg
    [
        1138, 1147, 1161, 1151, 1157, 1144, 1103, 1133,
        1108, 1145, 1173, 1191, 1205, 1166, 1156, 1126,
        1108, 1132, 1158, 1166, 1174, 1156, 1126, 1120,
        1118, 1140, 1153, 1172, 1183, 1170, 1160, 1135,
        1108, 1140, 1143, 1164, 1156, 1152, 1136, 1125,
        1105, 1111, 1132, 1130, 1131, 1124, 1106, 1097,
        1102, 1097, 1091, 1099, 1102, 1077, 1057, 1050,
        1090, 1094, 1098, 1087, 1090, 1084, 1089, 1094,
    ],
    // king mg
    [
        -126, -118, -111, -132, -126, -127, -124, -128,
        -127, -101, -105,  -84,  -95, -105,  -96, -114,
        -123,  -59,  -81,  -89,  -71,  -44,  -61,  -95,
         -92,  -77,  -83, -100,  -99,  -80,  -88, -102,
         -74,  -64,  -88, -103, -104,  -91,  -93, -110,
         -44,  -22,  -67,  -71,  -65,  -72,  -37,  -52,
          41,    9,   -6,  -35,  -38,  -17,   20,   26,
          28,   59,   37,  -51,   10,  -39,   40,   37,
    ],
    // king eg
    [
        -107,  -46,  -39,   -5,  -21,  -12,  -17, -106,
         -17,   12,   19,    6,   18,   33,   24,  -11,
          -5,   15,   31,   41,   36,   33,   33,    4,
         -11,   21,   38,   48,   49,   41,   35,    7,
         -16,   11,   32,   46,   44,   32,   20,    3,
         -18,   -1,   18,   27,   25,   16,   -1,  -12,
         -38,  -16,   -6,    4,    4,   -6,  -22,  -38,
         -69,  -59,  -42,  -18,  -45,  -26,  -52,  -79,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          17,   12,   15,   15,    8,    6,   -9,   -0,
          31,   40,   31,   18,   17,    8,  -30,  -48,
          11,    7,   17,   16,    0,    5,  -22,  -17,
          -3,  -12,  -18,  -11,  -18,   -9,  -24,  -15,
         -10,  -21,  -23,  -18,  -18,  -14,  -23,    2,
         -17,   -9,  -16,  -19,   -4,   -4,    2,   -4,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         -21,  -20,  -17,  -17,  -11,  -13,  -14,  -16,
         103,  101,   89,   60,   64,   85,   92,  110,
          54,   51,   42,   35,   37,   43,   58,   60,
          30,   27,   24,   18,   21,   23,   37,   35,
           0,    6,   12,    2,    7,    8,   20,    4,
           3,    5,   15,   10,   -1,    4,    6,    5,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 29;
const ROOK_OPEN_FILE_EG: i32 = 11;
const ROOK_CLOSED_FILE_MG: i32 = -15;
const ROOK_CLOSED_FILE_EG: i32 = -3;
const ROOK_SEMIOPEN_FILE_MG: i32 = 6;
const ROOK_SEMIOPEN_FILE_EG: i32 = 12;
const KING_OPEN_FILE_MG: i32 = -73;
const KING_OPEN_FILE_EG: i32 = -9;
const KING_CLOSED_FILE_MG: i32 = 15;
const KING_CLOSED_FILE_EG: i32 = -15;
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
