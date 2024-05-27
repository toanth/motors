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
         159,  169,  157,  185,  168,  150,   67,   47,
          77,   88,  108,  114,  122,  163,  146,  111,
          63,   85,   82,   86,  108,  104,  100,   84,
          51,   77,   72,   89,   87,   86,   85,   69,
          50,   72,   66,   67,   79,   78,  100,   76,
          49,   71,   62,   53,   69,   94,  110,   67,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         273,  268,  265,  219,  218,  230,  276,  285,
         127,  126,  104,  104,   97,   88,  119,  113,
         119,  114,   99,   88,   88,   88,  105,   95,
         109,  110,   96,   93,   93,   93,  102,   90,
         106,  108,   95,  104,  102,   96,   99,   88,
         110,  112,  101,  104,  112,  101,   98,   89,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         148,  193,  240,  272,  315,  223,  228,  191,
         278,  300,  334,  349,  334,  391,  303,  315,
         298,  337,  358,  370,  404,  412,  359,  326,
         300,  317,  341,  365,  344,  369,  324,  336,
         288,  304,  322,  321,  331,  325,  326,  300,
         269,  294,  306,  314,  325,  311,  316,  285,
         254,  266,  286,  297,  299,  303,  291,  286,
         219,  263,  252,  267,  274,  290,  268,  243,
    ],
    // knight eg
    [
         266,  321,  341,  330,  328,  323,  318,  245,
         317,  335,  338,  339,  335,  318,  329,  303,
         325,  338,  353,  356,  342,  337,  333,  318,
         336,  355,  367,  368,  371,  364,  356,  327,
         338,  349,  368,  372,  373,  362,  348,  329,
         322,  341,  350,  361,  359,  345,  334,  323,
         314,  332,  337,  340,  339,  333,  321,  321,
         301,  294,  322,  325,  325,  313,  300,  295,
    ],
    // bishop mg
    [
         302,  281,  287,  261,  268,  271,  315,  285,
         319,  348,  344,  326,  352,  349,  342,  316,
         333,  360,  359,  380,  373,  396,  374,  362,
         327,  345,  364,  379,  373,  368,  345,  330,
         323,  337,  344,  365,  361,  344,  339,  332,
         330,  342,  342,  346,  347,  342,  344,  346,
         335,  336,  349,  327,  336,  348,  356,  340,
         312,  335,  318,  309,  314,  314,  337,  323,
    ],
    // bishop eg
    [
         346,  359,  355,  365,  361,  355,  350,  345,
         339,  352,  357,  360,  351,  352,  355,  339,
         358,  356,  364,  356,  361,  363,  355,  354,
         357,  371,  365,  375,  373,  370,  371,  356,
         354,  368,  375,  373,  372,  372,  366,  345,
         352,  362,  369,  367,  373,  366,  352,  344,
         347,  346,  346,  359,  360,  349,  348,  329,
         333,  343,  331,  351,  346,  346,  331,  321,
    ],
    // rook mg
    [
         435,  424,  429,  436,  449,  458,  466,  478,
         410,  413,  431,  448,  435,  470,  464,  480,
         395,  419,  419,  419,  450,  462,  493,  462,
         387,  398,  401,  411,  416,  426,  431,  432,
         381,  382,  381,  395,  396,  392,  414,  409,
         379,  380,  382,  387,  396,  402,  437,  417,
         380,  384,  390,  392,  399,  410,  422,  394,
         403,  397,  395,  403,  410,  415,  415,  406,
    ],
    // rook eg
    [
         622,  631,  635,  628,  625,  625,  623,  617,
         632,  638,  637,  627,  630,  620,  619,  607,
         630,  627,  627,  623,  612,  608,  602,  603,
         629,  626,  628,  622,  614,  611,  610,  602,
         620,  621,  622,  618,  614,  615,  606,  600,
         612,  612,  611,  611,  606,  601,  584,  585,
         608,  609,  609,  607,  601,  596,  589,  595,
         611,  608,  617,  610,  603,  606,  599,  598,
    ],
    // queen mg
    [
         826,  843,  867,  897,  893,  912,  941,  879,
         858,  841,  851,  845,  851,  889,  874,  912,
         864,  860,  866,  877,  891,  930,  929,  918,
         852,  857,  862,  863,  867,  877,  876,  885,
         857,  854,  857,  862,  865,  863,  876,  876,
         852,  863,  860,  860,  864,  870,  882,  875,
         852,  862,  871,  871,  869,  879,  886,  887,
         853,  844,  850,  865,  857,  846,  856,  853,
    ],
    // queen eg
    [
        1185, 1193, 1208, 1197, 1203, 1185, 1141, 1177,
        1155, 1193, 1222, 1239, 1254, 1213, 1196, 1169,
        1154, 1178, 1207, 1215, 1223, 1204, 1171, 1166,
        1164, 1187, 1202, 1222, 1233, 1220, 1208, 1183,
        1155, 1187, 1191, 1213, 1205, 1201, 1184, 1172,
        1147, 1158, 1180, 1178, 1180, 1172, 1153, 1141,
        1143, 1143, 1138, 1146, 1149, 1123, 1099, 1076,
        1132, 1139, 1145, 1134, 1137, 1129, 1112, 1111,
    ],
    // king mg
    [
          55,   18,   47,  -48,   -4,   -3,   46,  177,
         -66,  -21,  -41,   31,    7,  -19,   15,   11,
         -81,   13,  -40,  -56,  -18,   30,    5,  -23,
         -62,  -61,  -78, -102, -101,  -78,  -87,  -98,
         -79,  -70,  -94, -109, -111,  -97, -100, -118,
         -49,  -26,  -72,  -77,  -70,  -78,  -41,  -57,
          39,    7,   -9,  -39,  -43,  -20,   18,   24,
          26,   59,   36,  -55,    7,  -43,   39,   36,
    ],
    // king eg
    [
        -117,  -55,  -48,  -13,  -29,  -20,  -26, -120,
         -25,    7,   14,    1,   13,   28,   19,  -18,
         -11,   11,   27,   38,   32,   30,   29,   -1,
         -17,   16,   34,   45,   46,   38,   31,    2,
         -22,    7,   28,   43,   41,   28,   15,   -2,
         -24,   -6,   14,   23,   22,   12,   -6,  -18,
         -45,  -21,  -11,   -1,   -1,  -12,  -28,  -45,
         -77,  -67,  -49,  -24,  -52,  -32,  -59,  -88,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          20,   15,   18,   19,   10,    8,   -9,    1,
          32,   41,   32,   19,   18,    8,  -31,  -50,
          11,    8,   18,   17,    0,    5,  -23,  -18,
          -3,  -13,  -19,  -12,  -19,  -10,  -25,  -16,
         -10,  -22,  -23,  -19,  -19,  -15,  -24,    2,
         -17,   -9,  -17,  -19,   -5,   -4,    2,   -4,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         -16,  -15,  -12,  -13,   -7,   -8,   -9,  -11,
         107,  106,   93,   63,   67,   89,   96,  115,
          56,   53,   44,   36,   39,   45,   60,   62,
          31,   28,   25,   19,   22,   24,   38,   36,
           0,    6,   12,    2,    8,    9,   21,    5,
           3,    5,   15,   10,   -1,    4,    6,    6,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 31;
const ROOK_OPEN_FILE_EG: i32 = 11;
const ROOK_CLOSED_FILE_MG: i32 = -16;
const ROOK_CLOSED_FILE_EG: i32 = -3;
const ROOK_SEMIOPEN_FILE_MG: i32 = 6;
const ROOK_SEMIOPEN_FILE_EG: i32 = 12;
const KING_OPEN_FILE_MG: i32 = -76;
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
