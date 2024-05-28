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
         149,  159,  148,  174,  160,  143,   65,   44,
          68,   77,   95,  101,  107,  144,  129,   98,
          55,   75,   72,   76,   95,   91,   88,   74,
          45,   68,   64,   78,   76,   76,   75,   60,
          44,   64,   58,   59,   70,   69,   88,   67,
          43,   62,   55,   46,   61,   83,   96,   59,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         258,  253,  250,  207,  206,  217,  261,  269,
         112,  111,   91,   92,   85,   77,  104,   99,
         105,  101,   87,   78,   78,   78,   93,   84,
          96,   97,   85,   81,   82,   82,   89,   79,
          93,   95,   83,   92,   90,   85,   87,   77,
          97,   98,   89,   92,   99,   89,   86,   79,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         134,  188,  226,  255,  299,  212,  226,  179,
         252,  267,  295,  312,  298,  348,  280,  292,
         272,  298,  316,  326,  356,  365,  320,  294,
         265,  279,  300,  322,  303,  325,  285,  297,
         254,  269,  283,  283,  292,  287,  287,  264,
         237,  259,  270,  276,  286,  274,  279,  251,
         225,  236,  252,  262,  263,  267,  257,  252,
         207,  232,  223,  236,  241,  255,  236,  230,
    ],
    // knight eg
    [
         241,  301,  310,  298,  298,  299,  304,  229,
         283,  295,  298,  299,  296,  282,  296,  275,
         290,  298,  310,  313,  301,  297,  295,  284,
         296,  312,  323,  323,  326,  321,  313,  288,
         297,  308,  324,  327,  328,  318,  306,  289,
         283,  300,  308,  318,  316,  304,  294,  284,
         282,  296,  297,  299,  298,  294,  285,  283,
         283,  258,  285,  287,  286,  275,  264,  281,
    ],
    // bishop mg
    [
         273,  263,  268,  240,  251,  251,  301,  265,
         283,  307,  304,  294,  313,  310,  305,  282,
         294,  317,  316,  335,  329,  349,  331,  319,
         288,  304,  321,  334,  329,  324,  304,  291,
         285,  296,  303,  322,  318,  303,  299,  292,
         291,  301,  301,  305,  306,  301,  303,  304,
         295,  296,  307,  288,  296,  307,  314,  300,
         275,  295,  280,  272,  276,  277,  307,  286,
    ],
    // bishop eg
    [
         309,  320,  316,  324,  321,  317,  316,  314,
         299,  310,  314,  318,  310,  310,  313,  301,
         315,  313,  320,  313,  318,  320,  313,  311,
         314,  326,  321,  330,  328,  326,  326,  313,
         312,  324,  330,  328,  327,  327,  322,  304,
         310,  318,  324,  323,  328,  322,  310,  303,
         306,  305,  304,  316,  317,  307,  306,  291,
         293,  302,  291,  309,  305,  305,  299,  286,
    ],
    // rook mg
    [
         384,  376,  380,  386,  398,  413,  423,  429,
         361,  364,  380,  395,  383,  414,  410,  423,
         348,  369,  369,  369,  396,  408,  438,  408,
         341,  351,  354,  362,  366,  376,  381,  381,
         336,  337,  336,  348,  349,  346,  366,  361,
         334,  335,  336,  341,  349,  354,  385,  368,
         335,  338,  344,  345,  352,  361,  372,  347,
         355,  349,  348,  355,  361,  365,  365,  358,
    ],
    // rook eg
    [
         547,  555,  558,  553,  550,  550,  548,  543,
         556,  561,  560,  551,  554,  545,  544,  534,
         554,  551,  551,  548,  538,  535,  529,  530,
         553,  550,  553,  547,  540,  537,  536,  530,
         546,  546,  547,  543,  540,  541,  533,  528,
         538,  539,  537,  537,  533,  529,  514,  515,
         534,  536,  536,  533,  528,  524,  518,  524,
         537,  535,  543,  537,  530,  533,  527,  526,
    ],
    // queen mg
    [
         733,  756,  777,  804,  801,  833,  863,  789,
         758,  743,  753,  749,  755,  787,  780,  809,
         764,  762,  765,  775,  788,  822,  823,  811,
         753,  757,  761,  762,  766,  775,  774,  782,
         757,  754,  757,  762,  764,  762,  774,  774,
         753,  763,  760,  760,  763,  769,  779,  773,
         753,  761,  770,  769,  768,  777,  783,  797,
         754,  746,  751,  765,  758,  748,  781,  792,
    ],
    // queen eg
    [
        1044, 1055, 1067, 1058, 1064, 1059, 1024, 1045,
        1016, 1049, 1076, 1094, 1107, 1071, 1071, 1040,
        1017, 1041, 1062, 1070, 1077, 1061, 1037, 1028,
        1026, 1045, 1057, 1074, 1084, 1073, 1064, 1040,
        1015, 1046, 1047, 1066, 1058, 1055, 1040, 1033,
        1019, 1017, 1037, 1034, 1036, 1029, 1013, 1010,
        1019, 1005,  999, 1006, 1009,  987,  972,  991,
        1007, 1006, 1006,  996,  999,  995, 1033, 1048,
    ],
    // king mg
    [
        -126, -118, -111, -131, -125, -126, -124, -129,
        -126, -100, -103,  -84,  -94, -104,  -96, -113,
        -120,  -59,  -78,  -86,  -69,  -44,  -60,  -93,
         -88,  -73,  -77,  -92,  -91,  -73,  -81,  -95,
         -69,  -58,  -79,  -92,  -94,  -81,  -84, -100,
         -40,  -19,  -59,  -63,  -58,  -64,  -32,  -46,
          39,   11,   -3,  -30,  -33,  -13,   20,   26,
          28,   56,   36,  -44,   11,  -34,   39,   36,
    ],
    // king eg
    [
        -102,  -42,  -36,   -3,  -19,  -10,  -15, -103,
         -15,   13,   19,    8,   19,   32,   24,   -9,
          -3,   16,   30,   39,   35,   32,   32,    6,
          -9,   21,   37,   46,   47,   39,   34,    8,
         -13,   12,   31,   44,   42,   31,   20,    4,
         -15,    1,   18,   27,   25,   17,    1,  -10,
         -33,  -12,   -3,    6,    6,   -4,  -18,  -33,
         -61,  -53,  -37,  -15,  -39,  -22,  -46,  -71,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          10,    5,    9,    8,    2,    1,  -11,   -2,
          28,   36,   28,   17,   16,    7,  -28,  -44,
          10,    7,   16,   15,    0,    5,  -20,  -16,
          -3,  -11,  -16,  -10,  -17,   -9,  -22,  -14,
          -9,  -20,  -21,  -17,  -17,  -13,  -21,    2,
         -15,   -8,  -15,  -18,   -4,   -3,    2,   -4,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         -31,  -30,  -27,  -25,  -19,  -21,  -24,  -27,
          94,   93,   81,   55,   59,   78,   84,  101,
          49,   47,   39,   32,   34,   39,   53,   55,
          27,   24,   22,   17,   19,   21,   34,   32,
           0,    6,   11,    2,    7,    8,   18,    4,
           3,    4,   13,    9,   -1,    3,    5,    5,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 27;
const ROOK_OPEN_FILE_EG: i32 = 10;
const ROOK_CLOSED_FILE_MG: i32 = -14;
const ROOK_CLOSED_FILE_EG: i32 = -3;
const ROOK_SEMIOPEN_FILE_MG: i32 = 5;
const ROOK_SEMIOPEN_FILE_EG: i32 = 11;
const KING_OPEN_FILE_MG: i32 = -67;
const KING_OPEN_FILE_EG: i32 = -8;
const KING_CLOSED_FILE_MG: i32 = 13;
const KING_CLOSED_FILE_EG: i32 = -14;
const KING_SEMIOPEN_FILE_MG: i32 = -30;
const KING_SEMIOPEN_FILE_EG: i32 = 7;

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
