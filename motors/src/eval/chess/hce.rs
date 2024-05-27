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
          54,   58,   55,   62,   58,   47,   11,   -4,
          54,   58,   56,   63,   59,   49,   14,   -1,
          57,   67,   83,   85,   85,  127,  112,   81,
          44,   68,   65,   71,   87,   88,   85,   61,
          32,   58,   59,   72,   69,   72,   70,   48,
          33,   55,   54,   51,   61,   65,   85,   56,
          32,   58,   45,   35,   45,   77,   97,   51,
          31,   58,   45,   34,   44,   77,   97,   51,
    ],
    // pawn eg
    [
         175,  172,  168,  149,  148,  149,  174,  187,
         174,  171,  167,  148,  147,  148,  172,  185,
         131,  135,  123,  128,  123,  102,  124,  120,
         121,  117,  105,   94,   95,   93,  106,  101,
         110,  112,   99,   97,   98,   96,  102,   93,
         107,  109,   98,  107,  107,   99,  100,   90,
         112,  111,  107,  112,  119,  106,  100,   92,
         113,  112,  108,  112,  120,  106,  100,   92,
    ],
    // knight mg
    [
         122,   94,  120,  147,  231,   85,  153,  152,
         224,  251,  289,  208,  257,  287,  235,  237,
         252,  287,  290,  326,  338,  362,  306,  284,
         260,  278,  301,  306,  301,  333,  287,  299,
         254,  260,  282,  282,  287,  291,  286,  264,
         238,  258,  277,  278,  287,  285,  284,  248,
         216,  229,  249,  262,  266,  269,  255,  250,
         177,  234,  211,  228,  236,  251,  245,  201,
    ],
    // knight eg
    [
         248,  309,  326,  318,  304,  310,  286,  238,
         294,  313,  315,  343,  317,  297,  296,  288,
         302,  314,  340,  332,  319,  305,  301,  288,
         310,  331,  346,  354,  352,  339,  335,  304,
         312,  330,  348,  353,  355,  345,  327,  308,
         297,  319,  328,  343,  342,  324,  314,  304,
         285,  305,  316,  321,  320,  315,  306,  298,
         284,  267,  302,  307,  304,  294,  272,  281,
    ],
    // bishop mg
    [
         236,  154,  137,  142,  185,  142,  181,  251,
         270,  304,  288,  267,  281,  307,  281,  265,
         248,  312,  307,  326,  322,  315,  311,  292,
         285,  295,  322,  335,  334,  320,  310,  286,
         288,  296,  308,  327,  323,  313,  302,  298,
         299,  309,  311,  314,  315,  312,  311,  307,
         301,  308,  313,  298,  303,  313,  328,  307,
         275,  297,  292,  266,  272,  286,  300,  276,
    ],
    // bishop eg
    [
         324,  356,  355,  356,  345,  348,  345,  315,
         327,  335,  345,  342,  338,  324,  332,  322,
         351,  340,  348,  342,  340,  351,  334,  340,
         338,  354,  348,  356,  354,  350,  347,  334,
         330,  349,  356,  358,  354,  354,  343,  325,
         331,  343,  350,  350,  356,  349,  337,  328,
         325,  326,  329,  340,  342,  335,  329,  309,
         318,  325,  312,  336,  335,  325,  318,  308,
    ],
    // rook mg
    [
         353,  347,  334,  351,  372,  368,  378,  391,
         355,  358,  378,  395,  388,  403,  384,  399,
         337,  360,  362,  373,  392,  401,  418,  395,
         329,  344,  351,  357,  365,  381,  383,  383,
         335,  329,  332,  343,  347,  351,  366,  368,
         329,  334,  334,  340,  349,  359,  391,  372,
         333,  337,  340,  344,  351,  364,  381,  337,
         363,  352,  351,  359,  366,  377,  367,  374,
    ],
    // rook eg
    [
         610,  615,  619,  607,  599,  601,  601,  600,
         617,  619,  614,  603,  598,  588,  596,  593,
         615,  608,  607,  597,  582,  581,  576,  581,
         609,  605,  605,  599,  590,  582,  581,  579,
         594,  599,  600,  595,  588,  589,  579,  574,
         588,  585,  588,  588,  582,  578,  559,  559,
         581,  580,  587,  585,  579,  574,  560,  573,
         589,  588,  596,  592,  584,  581,  580,  570,
    ],
    // queen mg
    [
         735,  709,  705,  756,  771,  803,  816,  809,
         772,  770,  761,  745,  761,  805,  805,  818,
         772,  772,  774,  785,  805,  818,  834,  834,
         767,  776,  780,  774,  786,  794,  800,  804,
         779,  770,  782,  782,  789,  790,  794,  803,
         778,  795,  789,  789,  792,  801,  811,  801,
         776,  790,  801,  802,  801,  806,  812,  810,
         781,  770,  779,  804,  786,  770,  777,  790,
    ],
    // queen eg
    [
        1124, 1170, 1194, 1169, 1173, 1144, 1113, 1106,
        1113, 1132, 1177, 1199, 1210, 1167, 1143, 1125,
        1110, 1133, 1159, 1168, 1166, 1174, 1134, 1118,
        1111, 1133, 1149, 1177, 1184, 1169, 1152, 1136,
        1102, 1137, 1136, 1169, 1155, 1152, 1144, 1115,
        1086, 1091, 1122, 1122, 1126, 1116, 1099, 1084,
        1084, 1084, 1075, 1084, 1086, 1071, 1058, 1030,
        1066, 1079, 1077, 1049, 1071, 1074, 1066, 1031,
    ],
    // king mg
    [
          76,   30,   32,    2,   11,   -4,   50,  106,
         -45,  -14,  -14,   -8,    9,  -42,   -3,   -7,
         -52,   -8,  -28,  -29,  -26,   28,  -10,  -22,
         -55,  -57,  -73,  -96, -104,  -82,  -84, -118,
         -77,  -64,  -94, -112, -114, -106,  -98, -130,
         -53,  -33,  -75,  -92,  -82,  -84,  -47,  -56,
          21,   -5,  -27,  -67,  -66,  -39,    5,   10,
           7,   44,   26,  -66,  -10,  -57,   25,   22,
    ],
    // king eg
    [
        -104,  -38,  -31,  -15,  -19,   -7,  -12,  -79,
         -15,   20,   27,   21,   30,   43,   40,    1,
           6,   33,   43,   49,   48,   49,   54,   16,
          -3,   36,   52,   63,   65,   59,   54,   25,
          -7,   22,   47,   64,   63,   53,   36,   17,
         -11,   11,   34,   49,   48,   38,   19,   -1,
         -31,   -4,   11,   26,   28,   16,   -3,  -25,
         -59,  -48,  -30,   -4,  -27,  -10,  -36,  -67,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
          54,   58,   55,   62,   58,   47,   11,   -4,
          53,   57,   54,   61,   58,   46,   10,   -5,
          27,   35,   36,   33,   39,   16,  -19,  -40,
          10,   10,   18,   19,    8,    5,  -18,  -18,
           7,   -8,  -14,   -5,  -10,  -10,  -19,   -9,
          -0,  -13,  -18,   -8,   -8,   -8,  -20,    8,
          -6,   -4,   -1,   -4,   10,    5,    7,    8,
          -6,   -4,   -1,   -4,   10,    6,    8,    8,
    ],
    // passed pawns eg
    [
         175,  172,  168,  149,  148,  149,  174,  187,
         174,  170,  166,  146,  145,  147,  171,  184,
         119,  110,   87,   52,   48,   82,   96,  119,
          58,   55,   44,   35,   33,   43,   59,   63,
          28,   27,   23,   14,   16,   23,   36,   35,
          -1,    3,    8,   -2,    1,    6,   17,    4,
          -1,    2,    3,   -0,  -11,   -4,    3,    3,
          -1,    2,    3,   -0,  -11,   -5,    3,    3,
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
