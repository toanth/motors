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
          58,   63,   59,   67,   62,   49,    9,   -6,
          61,   74,   92,   93,   92,  144,  124,   89,
          47,   75,   70,   77,   95,   94,   91,   63,
          34,   64,   63,   79,   74,   77,   74,   48,
          35,   61,   58,   54,   67,   70,   93,   59,
          34,   64,   47,   36,   49,   86,  106,   50,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         189,  185,  181,  159,  159,  161,  189,  202,
         140,  145,  131,  138,  132,  107,  134,  127,
         130,  125,  112,   99,  101,   99,  115,  108,
         118,  121,  105,  104,  106,  103,  110,   99,
         114,  118,  104,  116,  115,  106,  107,   96,
         121,  120,  115,  121,  129,  113,  108,   99,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         127,   94,  126,  157,  255,   72,  167,  159,
         243,  277,  320,  217,  279,  315,  250,  256,
         272,  313,  312,  357,  369,  395,  328,  305,
         280,  301,  327,  329,  322,  360,  304,  324,
         273,  280,  305,  302,  308,  312,  307,  281,
         256,  280,  300,  299,  311,  306,  306,  263,
         233,  247,  269,  283,  287,  290,  272,  270,
         189,  256,  224,  245,  253,  270,  262,  211,
    ],
    // knight eg
    [
         265,  336,  352,  341,  326,  334,  305,  250,
         317,  339,  337,  372,  339,  317,  318,  310,
         325,  338,  369,  356,  341,  326,  323,  307,
         334,  359,  373,  381,  380,  365,  362,  325,
         336,  357,  377,  380,  383,  370,  350,  330,
         319,  345,  353,  370,  368,  346,  337,  326,
         306,  331,  341,  345,  344,  339,  330,  321,
         305,  284,  327,  330,  326,  315,  290,  303,
    ],
    // bishop mg
    [
         252,  154,  141,  148,  199,  143,  195,  275,
         292,  334,  313,  288,  306,  337,  303,  284,
         265,  341,  330,  354,  347,  339,  335,  313,
         308,  318,  349,  362,  359,  344,  333,  305,
         310,  319,  332,  354,  346,  335,  323,  320,
         322,  333,  335,  338,  339,  336,  334,  330,
         325,  333,  338,  319,  327,  339,  355,  329,
         294,  320,  312,  282,  292,  307,  323,  294,
    ],
    // bishop eg
    [
         348,  386,  382,  383,  371,  375,  371,  336,
         351,  360,  371,  367,  363,  345,  357,  345,
         379,  365,  375,  367,  365,  379,  358,  366,
         363,  383,  373,  383,  382,  376,  374,  358,
         355,  377,  384,  385,  380,  381,  368,  348,
         356,  370,  378,  377,  383,  375,  362,  353,
         350,  350,  354,  366,  368,  360,  353,  330,
         342,  350,  334,  363,  360,  348,  341,  330,
    ],
    // rook mg
    [
         379,  373,  357,  378,  401,  394,  407,  421,
         382,  385,  410,  429,  417,  436,  411,  431,
         362,  389,  389,  402,  424,  433,  453,  424,
         353,  371,  378,  384,  393,  411,  411,  411,
         361,  353,  357,  369,  372,  376,  394,  394,
         354,  359,  359,  365,  376,  387,  424,  400,
         357,  363,  366,  369,  378,  393,  411,  356,
         391,  378,  378,  387,  394,  406,  393,  403,
    ],
    // rook eg
    [
         656,  661,  666,  651,  644,  647,  646,  646,
         664,  666,  661,  647,  644,  632,  642,  638,
         661,  653,  652,  641,  623,  625,  619,  624,
         656,  651,  651,  644,  634,  625,  625,  622,
         638,  645,  645,  640,  632,  634,  622,  617,
         632,  629,  633,  632,  626,  621,  599,  600,
         624,  624,  631,  629,  622,  617,  600,  618,
         633,  632,  642,  636,  628,  625,  624,  612,
    ],
    // queen mg
    [
         789,  759,  755,  817,  830,  866,  879,  869,
         831,  830,  819,  799,  818,  869,  864,  880,
         830,  830,  833,  846,  869,  883,  900,  898,
         824,  835,  839,  831,  845,  853,  860,  864,
         839,  827,  842,  840,  848,  849,  854,  864,
         837,  857,  848,  849,  852,  863,  873,  861,
         834,  851,  864,  863,  862,  869,  874,  872,
         840,  827,  838,  866,  843,  825,  835,  850,
    ],
    // queen eg
    [
        1209, 1263, 1286, 1254, 1260, 1226, 1193, 1188,
        1197, 1218, 1269, 1293, 1304, 1252, 1228, 1210,
        1194, 1220, 1248, 1256, 1252, 1263, 1215, 1200,
        1195, 1220, 1236, 1269, 1275, 1257, 1238, 1221,
        1185, 1228, 1222, 1260, 1241, 1239, 1231, 1197,
        1167, 1173, 1210, 1206, 1211, 1200, 1180, 1165,
        1167, 1165, 1154, 1166, 1167, 1149, 1134, 1103,
        1146, 1161, 1158, 1124, 1153, 1154, 1146, 1106,
    ],
    // king mg
    [
          87,   30,   36,    0,   13,   -4,   60,  123,
         -53,  -15,  -17,   -8,   12,  -53,   -2,  -12,
         -55,   -3,  -31,  -29,  -26,   41,  -11,  -22,
         -58,  -63,  -81, -107, -114,  -89,  -93, -133,
         -85,  -69, -105, -123, -124, -115, -107, -146,
         -59,  -34,  -86, -100,  -86,  -91,  -48,  -61,
          26,   -7,  -31,  -74,  -72,  -38,   10,   13,
           7,   51,   28,  -78,   -4,  -66,   34,   23,
    ],
    // king eg
    [
        -115,  -37,  -35,  -16,  -22,   -8,  -15,  -93,
         -13,   26,   31,   23,   34,   49,   45,    0,
           7,   38,   48,   54,   52,   53,   58,   14,
          -3,   42,   57,   70,   71,   63,   58,   25,
          -7,   27,   53,   70,   68,   56,   38,   17,
         -11,   14,   39,   54,   52,   40,   19,   -2,
         -33,   -1,   14,   30,   31,   16,   -5,  -28,
         -65,  -52,  -32,   -4,  -33,  -11,  -42,  -76,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          58,   63,   59,   67,   62,   49,    9,   -6,
          29,   38,   38,   35,   42,   14,  -25,  -46,
          11,   10,   20,   21,    7,    6,  -21,  -18,
           8,  -10,  -16,   -5,  -12,  -12,  -21,   -9,
          -0,  -15,  -20,   -8,   -9,  -10,  -23,   12,
          -7,   -4,   -1,   -4,   13,    6,    9,    9,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         189,  185,  181,  159,  159,  161,  189,  202,
         128,  117,   90,   51,   48,   90,  102,  129,
          61,   58,   45,   37,   36,   47,   64,   67,
          30,   29,   24,   14,   18,   25,   40,   38,
          -2,    3,    9,   -4,    1,    7,   19,    2,
          -1,    2,    3,   -0,  -13,   -4,    4,    3,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 41;
const ROOK_OPEN_FILE_EG: i32 = 5;
const ROOK_CLOSED_FILE_MG: i32 = -20;
const ROOK_CLOSED_FILE_EG: i32 = -2;
const ROOK_SEMIOPEN_FILE_MG: i32 = 7;
const ROOK_SEMIOPEN_FILE_EG: i32 = 22;
const KING_OPEN_FILE_MG: i32 = -70;
const KING_OPEN_FILE_EG: i32 = -12;
const KING_CLOSED_FILE_MG: i32 = 20;
const KING_CLOSED_FILE_EG: i32 = -17;
const KING_SEMIOPEN_FILE_MG: i32 = -27;
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
