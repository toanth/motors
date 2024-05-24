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
/// using my own tuner.
#[rustfmt::skip]
const PSQTS: [[i32; NUM_SQUARES]; NUM_CHESS_PIECES * 2] = [
    // pawn mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         107,  109,  103,  123,  110,  100,   40,   34,
          98,  108,  134,  149,  155,  203,  180,  142,
          78,  106,  104,  109,  139,  130,  128,  108,
          65,   97,   92,  114,  112,  109,  109,   89,
          62,   94,   86,   86,  103,  101,  125,   97,
          63,   93,   81,   69,   92,  119,  137,   86,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         138,  137,  141,  114,  120,  121,  148,  149,
         142,  143,  123,  125,  101,   94,  133,  128,
         133,  127,  111,  102,   98,   97,  117,  106,
         121,  124,  109,  104,  106,  103,  114,  101,
         119,  122,  108,  118,  113,  108,  111,   99,
         127,  129,  119,  119,  127,  112,  112,  101,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         188,  240,  299,  309,  374,  278,  283,  247,
         363,  385,  423,  437,  425,  491,  394,  406,
         385,  425,  453,  468,  512,  512,  459,  415,
         383,  405,  431,  466,  443,  468,  416,  427,
         366,  387,  408,  412,  422,  416,  414,  382,
         341,  375,  393,  400,  413,  397,  402,  363,
         322,  343,  365,  377,  382,  387,  371,  363,
         290,  333,  323,  345,  347,  365,  339,  306,
    ],
    // knight eg
    [
         298,  366,  382,  382,  373,  360,  355,  283,
         358,  374,  385,  381,  373,  357,  367,  339,
         367,  381,  399,  400,  381,  378,  369,  360,
         376,  402,  413,  413,  419,  406,  404,  365,
         386,  397,  415,  423,  423,  410,  398,  371,
         366,  389,  404,  413,  409,  396,  381,  367,
         358,  379,  384,  390,  389,  383,  368,  365,
         346,  333,  368,  371,  373,  356,  344,  339,
    ],
    // bishop mg
    [
         380,  374,  359,  325,  336,  339,  406,  354,
         413,  440,  439,  416,  445,  442,  433,  413,
         427,  453,  450,  482,  469,  498,  475,  464,
         417,  439,  462,  477,  476,  466,  440,  420,
         410,  426,  439,  464,  458,  441,  430,  421,
         420,  436,  433,  441,  442,  436,  438,  438,
         425,  428,  446,  415,  426,  443,  451,  433,
         402,  428,  401,  393,  396,  399,  426,  416,
    ],
    // bishop eg
    [
         394,  404,  407,  418,  410,  407,  396,  394,
         386,  400,  404,  407,  400,  397,  405,  384,
         408,  405,  416,  404,  408,  413,  402,  403,
         408,  423,  415,  425,  422,  419,  425,  404,
         408,  420,  428,  424,  421,  425,  418,  396,
         403,  414,  421,  421,  427,  421,  405,  396,
         401,  401,  396,  412,  413,  401,  400,  382,
         387,  396,  381,  402,  398,  401,  389,  375,
    ],
    // rook mg
    [
         552,  539,  541,  550,  568,  580,  588,  611,
         534,  530,  557,  578,  560,  606,  595,  614,
         508,  530,  532,  537,  573,  591,  630,  597,
         500,  515,  513,  524,  531,  548,  556,  556,
         496,  498,  491,  510,  510,  510,  534,  532,
         497,  497,  497,  504,  514,  519,  562,  540,
         495,  497,  505,  507,  515,  530,  541,  508,
         520,  511,  509,  521,  529,  534,  534,  523,
    ],
    // rook eg
    [
         707,  719,  727,  719,  714,  711,  709,  699,
         719,  727,  725,  714,  718,  701,  700,  688,
         718,  714,  712,  709,  695,  690,  677,  681,
         718,  710,  715,  712,  700,  694,  691,  682,
         712,  710,  712,  709,  704,  704,  694,  686,
         706,  702,  700,  703,  696,  691,  669,  673,
         700,  701,  699,  699,  691,  685,  676,  684,
         704,  699,  708,  702,  693,  697,  689,  691,
    ],
    // queen mg
    [
        1046, 1077, 1096, 1131, 1145, 1160, 1200, 1118,
        1106, 1085, 1097, 1088, 1095, 1145, 1130, 1170,
        1109, 1108, 1113, 1132, 1141, 1190, 1192, 1177,
        1090, 1101, 1107, 1109, 1116, 1131, 1125, 1135,
        1097, 1096, 1095, 1109, 1109, 1107, 1123, 1122,
        1096, 1104, 1100, 1102, 1104, 1113, 1128, 1119,
        1091, 1098, 1113, 1114, 1112, 1123, 1130, 1135,
        1089, 1080, 1086, 1106, 1099, 1084, 1095, 1090,
    ],
    // queen eg
    [
        1353, 1358, 1381, 1374, 1365, 1347, 1285, 1331,
        1310, 1351, 1386, 1408, 1429, 1379, 1355, 1328,
        1317, 1333, 1371, 1377, 1393, 1369, 1326, 1328,
        1331, 1350, 1365, 1389, 1405, 1384, 1379, 1348,
        1321, 1356, 1362, 1383, 1377, 1376, 1351, 1343,
        1306, 1328, 1354, 1353, 1361, 1350, 1323, 1311,
        1304, 1313, 1304, 1317, 1323, 1293, 1263, 1230,
        1293, 1304, 1313, 1303, 1302, 1299, 1275, 1276,
    ],
    // king mg
    [
          46,   -8,   18,  -73,  -57,  -13,   35,  161,
         -97,  -51,  -72,   22,  -11,  -48,   -3,   -9,
        -133,  -14,  -71,  -97,  -49,    2,  -20,  -72,
         -99,  -88, -120, -156, -146, -129, -128, -154,
        -109, -102, -130, -154, -158, -140, -134, -155,
         -72,  -48, -101, -106,  -98, -110,  -62,  -84,
          35,   -3,  -22,  -61,  -69,  -39,    4,   15,
          21,   59,   32,  -87,   -9,  -70,   29,   29,
    ],
    // king eg
    [
        -127,  -48,  -37,    3,  -15,  -12,  -14, -135,
         -13,   24,   30,   13,   32,   46,   34,   -2,
           6,   28,   51,   61,   53,   53,   50,   15,
          -1,   36,   60,   73,   74,   66,   56,   25,
          -3,   30,   56,   75,   73,   57,   40,   21,
          -2,   20,   42,   54,   51,   39,   20,    6,
         -24,    4,   14,   28,   28,   16,   -5,  -25,
         -57,  -51,  -28,    2,  -28,   -5,  -39,  -72,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         107,  109,  103,  123,  110,  100,   40,   34,
          24,   48,   40,   21,   22,   10,  -34,  -63,
           9,    7,   20,   21,   -2,    7,  -28,  -23,
          -7,  -15,  -23,  -14,  -24,  -11,  -31,  -23,
         -14,  -26,  -27,  -24,  -25,  -21,  -29,    2,
         -23,   -7,  -14,  -22,   -9,   -4,    3,   -8,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         138,  137,  141,  114,  120,  121,  148,  149,
         115,  115,   97,   65,   85,  107,  108,  126,
          60,   60,   52,   41,   48,   54,   71,   69,
          33,   33,   30,   21,   28,   31,   47,   41,
          -1,    8,   17,    4,   11,   15,   27,    6,
           8,    7,   20,    9,   -3,    5,   11,    8,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 40;
const ROOK_OPEN_FILE_EG: i32 = 11;
const ROOK_SEMIOPEN_FILE_MG: i32 = -21;
const ROOK_SEMIOPEN_FILE_EG: i32 = -5;
const ROOK_CLOSED_FILE_MG: i32 = 8;
const ROOK_CLOSED_FILE_EG: i32 = 13;
const KING_OPEN_FILE_MG: i32 = -92;
const KING_OPEN_FILE_EG: i32 = -8;
const KING_SEMIOPEN_FILE_MG: i32 = 18;
const KING_SEMIOPEN_FILE_EG: i32 = -20;
const KING_CLOSED_FILE_MG: i32 = -42;
const KING_CLOSED_FILE_EG: i32 = 12;

// TODO: Differentiate between rooks and kings in front of / behind pawns.

const PIECE_PHASE: [i32; 6] = [0, 1, 1, 2, 4, 0];

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
