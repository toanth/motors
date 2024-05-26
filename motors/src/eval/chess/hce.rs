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
          84,   86,   83,   95,   85,   75,   31,   23,
          84,   86,   83,   95,   86,   77,   34,   26,
          72,   82,  100,  106,  113,  148,  134,  103,
          59,   79,   78,   82,  100,   99,   95,   81,
          49,   71,   69,   82,   82,   81,   81,   67,
          47,   67,   63,   63,   74,   75,   93,   73,
          47,   65,   59,   51,   64,   87,  102,   66,
          46,   65,   59,   50,   64,   87,  102,   66,
    ],
    // pawn eg
    [
         122,  120,  119,   99,  100,  104,  124,  129,
         122,  120,  119,   99,  100,  104,  124,  128,
         120,  119,  100,   98,   92,   84,  110,  107,
         112,  109,   95,   85,   84,   84,   99,   91,
         103,  104,   92,   88,   88,   88,   95,   86,
         100,  102,   91,   98,   96,   91,   94,   84,
         104,  105,   96,   98,  105,   96,   93,   85,
         104,  105,   96,   98,  105,   96,   93,   85,
    ],
    // knight mg
    [
         144,  182,  226,  257,  295,  220,  218,  186,
         259,  279,  310,  326,  316,  362,  290,  294,
         281,  314,  335,  347,  376,  386,  340,  309,
         283,  298,  320,  342,  327,  347,  309,  316,
         271,  286,  302,  304,  312,  308,  307,  285,
         253,  275,  288,  295,  305,  294,  298,  271,
         239,  251,  268,  279,  281,  285,  275,  269,
         208,  245,  239,  252,  258,  272,  255,  232,
    ],
    // knight eg
    [
         253,  300,  321,  312,  310,  305,  301,  238,
         298,  314,  319,  320,  316,  303,  310,  287,
         307,  319,  333,  336,  325,  319,  315,  302,
         317,  334,  345,  347,  349,  344,  336,  311,
         319,  329,  346,  350,  352,  342,  330,  312,
         304,  321,  330,  341,  339,  327,  317,  306,
         297,  312,  318,  321,  321,  315,  304,  303,
         285,  279,  303,  308,  307,  297,  285,  279,
    ],
    // bishop mg
    [
         286,  268,  272,  250,  255,  258,  295,  272,
         301,  325,  323,  308,  329,  328,  323,  301,
         314,  337,  338,  356,  351,  370,  352,  340,
         308,  324,  342,  356,  352,  348,  327,  313,
         305,  317,  324,  343,  341,  326,  321,  314,
         311,  322,  322,  326,  327,  323,  325,  325,
         316,  317,  328,  310,  316,  327,  334,  321,
         295,  315,  301,  292,  296,  297,  317,  306,
    ],
    // bishop eg
    [
         327,  338,  335,  344,  341,  336,  331,  326,
         321,  332,  337,  340,  333,  333,  335,  322,
         337,  336,  344,  338,  341,  343,  337,  334,
         337,  349,  345,  354,  352,  350,  350,  337,
         335,  347,  353,  352,  352,  351,  346,  328,
         333,  341,  347,  347,  352,  346,  334,  326,
         328,  327,  327,  338,  340,  331,  329,  312,
         315,  323,  314,  331,  328,  327,  314,  304,
    ],
    // rook mg
    [
         409,  400,  405,  411,  422,  431,  439,  450,
         387,  389,  405,  420,  411,  440,  438,  451,
         372,  393,  394,  396,  421,  434,  461,  437,
         365,  375,  378,  387,  392,  401,  407,  408,
         359,  360,  360,  371,  373,  371,  390,  387,
         358,  359,  360,  365,  373,  379,  409,  394,
         359,  362,  367,  369,  376,  386,  397,  374,
         379,  374,  372,  379,  386,  390,  391,  384,
    ],
    // rook eg
    [
         588,  596,  599,  594,  591,  590,  588,  583,
         597,  602,  601,  593,  594,  586,  584,  575,
         596,  593,  593,  589,  579,  575,  570,  570,
         594,  591,  593,  588,  580,  577,  576,  570,
         586,  587,  588,  584,  581,  581,  572,  567,
         578,  579,  577,  577,  573,  569,  554,  553,
         574,  576,  576,  574,  568,  564,  557,  561,
         577,  575,  582,  577,  570,  572,  566,  565,
    ],
    // queen mg
    [
         777,  791,  813,  840,  838,  855,  881,  830,
         806,  792,  800,  796,  801,  835,  826,  855,
         812,  808,  813,  822,  835,  869,  870,  862,
         801,  805,  809,  811,  815,  825,  825,  832,
         805,  802,  805,  810,  813,  811,  823,  823,
         801,  810,  809,  808,  812,  817,  828,  823,
         800,  809,  817,  817,  817,  825,  831,  832,
         801,  794,  798,  812,  807,  797,  805,  803,
    ],
    // queen eg
    [
        1121, 1129, 1142, 1135, 1139, 1124, 1085, 1112,
        1094, 1127, 1154, 1170, 1183, 1150, 1132, 1108,
        1093, 1114, 1141, 1150, 1158, 1142, 1112, 1105,
        1101, 1122, 1136, 1155, 1166, 1155, 1143, 1121,
        1094, 1121, 1127, 1146, 1141, 1137, 1122, 1110,
        1086, 1096, 1115, 1115, 1116, 1110, 1092, 1080,
        1082, 1083, 1079, 1085, 1088, 1066, 1044, 1023,
        1072, 1078, 1083, 1074, 1076, 1069, 1053, 1051,
    ],
    // king mg
    [
          37,    6,   29,  -49,  -17,  -15,   28,  143,
         -70,  -32,  -46,   10,   -6,  -27,    1,    1,
         -87,   -8,  -48,  -63,  -32,   10,   -8,  -33,
         -71,  -68,  -83, -105, -105,  -84,  -90, -102,
         -85,  -77,  -97, -113, -115, -103, -104, -119,
         -56,  -38,  -76,  -83,  -79,  -84,  -53,  -64,
          22,   -3,  -19,  -48,  -52,  -34,    1,    9,
          14,   40,   22,  -58,  -10,  -49,   19,   22,
    ],
    // king eg
    [
         -86,  -32,  -22,    8,   -4,    4,   -1,  -83,
          -4,   24,   33,   23,   33,   46,   39,    5,
          11,   30,   46,   56,   52,   50,   49,   22,
           6,   35,   53,   63,   65,   57,   51,   25,
           1,   26,   47,   61,   60,   49,   37,   21,
          -2,   15,   33,   43,   42,   34,   17,    5,
         -21,   -1,   10,   20,   21,   11,   -4,  -20,
         -50,  -41,  -25,   -2,  -24,   -9,  -31,  -58,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
          84,   86,   83,   95,   85,   75,   31,   23,
          82,   85,   81,   93,   83,   73,   29,   21,
          31,   39,   32,   21,   19,   10,  -25,  -43,
          11,    8,   16,   15,    1,    4,  -20,  -18,
          -3,  -11,  -17,  -11,  -17,  -10,  -23,  -15,
          -9,  -20,  -22,  -18,  -18,  -14,  -21,   -0,
         -16,  -10,  -16,  -18,   -6,   -4,    1,   -4,
         -16,   -9,  -15,  -18,   -5,   -4,    2,   -3,
    ],
    // passed pawns eg
    [
         122,  120,  119,   99,  100,  104,  124,  129,
         121,  119,  118,   98,   99,  103,  123,  128,
         100,   99,   88,   62,   63,   82,   90,  107,
          54,   51,   43,   35,   37,   42,   56,   59,
          29,   26,   24,   18,   20,   23,   35,   34,
           1,    6,   12,    4,    7,    8,   19,    6,
           3,    4,   14,   10,    0,    3,    6,    6,
           3,    4,   14,   10,   -0,    3,    6,    5,
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
