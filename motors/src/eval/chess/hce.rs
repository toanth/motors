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
          94,   91,   92,  104,   95,   83,   34,   29,
          78,   93,  114,  119,  126,  172,  154,  117,
          68,   91,   87,   93,  112,  109,  106,   89,
          55,   83,   77,   95,   92,   91,   92,   68,
          54,   78,   73,   72,   84,   84,  108,   81,
          54,   78,   67,   59,   73,  103,  119,   75,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         125,  125,  122,  100,  103,  109,  131,  132,
         125,  122,  101,   99,   95,   83,  112,  108,
         116,  111,   96,   85,   86,   85,  102,   93,
         106,  107,   93,   89,   91,   90,   98,   89,
         103,  105,   92,  101,   99,   93,   96,   85,
         108,  108,   99,  102,  108,   98,   94,   86,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         162,  231,  273,  297,  352,  241,  262,  214,
         302,  329,  368,  381,  370,  424,  338,  344,
         330,  371,  391,  406,  440,  449,  394,  358,
         335,  352,  374,  401,  376,  403,  355,  371,
         321,  338,  356,  355,  365,  361,  360,  330,
         303,  328,  343,  349,  359,  345,  351,  317,
         288,  300,  320,  331,  333,  338,  325,  320,
         252,  300,  284,  301,  306,  322,  303,  272,
    ],
    // knight eg
    [
         264,  303,  325,  316,  310,  311,  301,  233,
         303,  320,  321,  323,  317,  301,  311,  288,
         309,  321,  337,  340,  324,  321,  316,  303,
         319,  339,  351,  351,  355,  348,  341,  310,
         322,  333,  352,  356,  357,  345,  331,  314,
         305,  324,  332,  345,  343,  329,  317,  305,
         298,  315,  320,  324,  323,  316,  306,  303,
         282,  277,  307,  310,  309,  298,  281,  279,
    ],
    // bishop mg
    [
         346,  313,  320,  302,  299,  307,  347,  325,
         350,  385,  377,  360,  387,  385,  377,  345,
         365,  395,  393,  414,  410,  430,  410,  399,
         360,  379,  400,  414,  408,  403,  380,  365,
         357,  373,  380,  402,  399,  380,  375,  365,
         367,  379,  379,  382,  384,  379,  379,  382,
         371,  374,  383,  364,  372,  385,  395,  375,
         347,  369,  355,  344,  347,  347,  372,  356,
    ],
    // bishop eg
    [
         329,  341,  338,  347,  345,  340,  335,  330,
         325,  336,  341,  343,  336,  336,  339,  324,
         342,  340,  349,  341,  345,  348,  339,  336,
         342,  355,  349,  358,  357,  353,  354,  339,
         339,  351,  358,  356,  355,  356,  349,  330,
         335,  346,  352,  351,  356,  349,  336,  328,
         332,  329,  330,  343,  344,  333,  330,  313,
         317,  327,  315,  336,  332,  331,  315,  308,
    ],
    // rook mg
    [
         485,  475,  479,  486,  499,  505,  518,  527,
         457,  458,  479,  497,  483,  519,  513,  526,
         443,  465,  465,  468,  499,  511,  540,  507,
         433,  443,  449,  460,  463,  474,  480,  480,
         428,  429,  427,  441,  442,  441,  461,  454,
         425,  427,  428,  435,  443,  449,  484,  464,
         426,  431,  436,  439,  446,  458,  469,  437,
         450,  444,  443,  451,  458,  463,  460,  455,
    ],
    // rook eg
    [
         599,  608,  610,  604,  601,  601,  597,  593,
         611,  616,  613,  604,  606,  596,  596,  585,
         609,  605,  605,  601,  589,  586,  580,  582,
         607,  604,  606,  599,  593,  590,  588,  581,
         598,  599,  601,  596,  593,  593,  583,  578,
         589,  590,  589,  588,  584,  579,  562,  563,
         586,  587,  588,  584,  579,  574,  567,  574,
         589,  586,  594,  588,  580,  583,  578,  574,
    ],
    // queen mg
    [
         928,  944,  967, 1001,  995, 1017, 1042,  981,
         957,  943,  952,  941,  949,  991,  974, 1015,
         963,  961,  964,  982,  996, 1035, 1035, 1022,
         953,  957,  964,  964,  969,  980,  977,  988,
         957,  956,  959,  963,  969,  966,  980,  977,
         953,  967,  964,  964,  968,  974,  985,  976,
         953,  966,  974,  975,  974,  984,  990,  987,
         956,  948,  953,  971,  960,  948,  953,  955,
    ],
    // queen eg
    [
        1138, 1148, 1160, 1148, 1154, 1134, 1094, 1132,
        1111, 1145, 1174, 1196, 1207, 1167, 1150, 1115,
        1107, 1132, 1162, 1163, 1175, 1152, 1118, 1113,
        1115, 1141, 1152, 1174, 1186, 1173, 1160, 1132,
        1111, 1138, 1142, 1168, 1155, 1151, 1134, 1121,
        1099, 1106, 1132, 1127, 1129, 1121, 1103, 1092,
        1096, 1095, 1089, 1096, 1098, 1071, 1049, 1031,
        1085, 1089, 1095, 1078, 1088, 1078, 1067, 1062,
    ],
    // king mg
    [
          52,   21,   48,  -18,   15,   16,   39,  121,
         -60,   -2,  -15,   21,   17,    6,    6,   -9,
         -63,   31,  -32,  -37,  -15,   34,   12,  -26,
         -57,  -53,  -65,  -88,  -92,  -69,  -78,  -98,
         -77,  -65,  -89, -109, -112, -103, -102, -120,
         -57,  -35,  -82,  -86,  -81,  -87,  -51,  -67,
          27,   -5,  -23,  -57,  -59,  -34,    6,   12,
           9,   47,   24,  -68,   -5,  -60,   27,   22,
    ],
    // king eg
    [
         -92,  -32,  -28,   -2,  -12,   -2,   -5,  -77,
          -7,   21,   25,   18,   26,   40,   38,    4,
           5,   25,   41,   48,   45,   45,   46,   18,
          -2,   31,   48,   58,   60,   52,   47,   21,
          -5,   23,   45,   60,   58,   47,   33,   16,
          -4,   14,   33,   43,   41,   32,   15,    3,
         -22,    0,   11,   22,   21,   10,   -6,  -23,
         -50,  -45,  -27,   -1,  -31,   -9,  -37,  -65,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          94,   91,   92,  104,   95,   83,   34,   29,
          38,   42,   31,   18,   18,    6,  -38,  -52,
          12,    9,   17,   16,    1,    6,  -22,  -19,
          -1,  -13,  -18,  -13,  -18,  -11,  -27,  -11,
         -10,  -22,  -24,  -21,  -17,  -12,  -20,    4,
         -17,  -11,  -16,  -22,   -4,   -2,    7,   -4,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         125,  125,  122,  100,  103,  109,  131,  132,
         105,  103,   91,   66,   65,   89,   96,  115,
          55,   52,   43,   36,   38,   44,   59,   60,
          29,   27,   25,   20,   21,   24,   38,   34,
           1,    6,   12,    4,    8,    9,   19,    5,
           3,    6,   14,    8,    1,    4,    6,    6,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 33;
const ROOK_OPEN_FILE_EG: i32 = 10;
const ROOK_CLOSED_FILE_MG: i32 = -16;
const ROOK_CLOSED_FILE_EG: i32 = -4;
const ROOK_SEMIOPEN_FILE_MG: i32 = 7;
const ROOK_SEMIOPEN_FILE_EG: i32 = 12;
const KING_OPEN_FILE_MG: i32 = -72;
const KING_OPEN_FILE_EG: i32 = -10;
const KING_CLOSED_FILE_MG: i32 = 15;
const KING_CLOSED_FILE_EG: i32 = -16;
const KING_SEMIOPEN_FILE_MG: i32 = -34;
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
