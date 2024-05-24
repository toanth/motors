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
          71,   73,   69,   82,   74,   66,   26,   23,
          66,   72,   89,   99,  104,  135,  120,   94,
          52,   71,   70,   72,   92,   87,   85,   72,
          43,   65,   61,   76,   74,   72,   72,   59,
          42,   62,   57,   57,   69,   67,   84,   64,
          41,   61,   53,   47,   62,   80,   91,   58,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          92,   91,   94,   76,   80,   80,   99,  100,
          95,   95,   82,   83,   67,   62,   89,   85,
          89,   85,   74,   68,   65,   64,   77,   71,
          81,   83,   73,   70,   70,   69,   76,   67,
          80,   81,   72,   79,   76,   71,   74,   65,
          84,   86,   79,   79,   85,   75,   74,   68,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         125,  160,  199,  206,  249,  186,  189,  165,
         242,  257,  281,  291,  283,  328,  263,  271,
         257,  283,  301,  312,  341,  341,  306,  276,
         255,  270,  288,  310,  295,  312,  277,  285,
         244,  257,  272,  274,  282,  277,  276,  255,
         228,  249,  261,  266,  275,  265,  268,  242,
         214,  229,  244,  252,  254,  258,  248,  242,
         193,  222,  215,  229,  232,  244,  226,  204,
    ],
    // knight eg
    [
         199,  244,  255,  255,  249,  241,  237,  189,
         238,  249,  256,  254,  248,  238,  245,  226,
         245,  254,  265,  266,  255,  252,  246,  240,
         251,  268,  275,  275,  279,  271,  269,  244,
         257,  264,  277,  282,  283,  273,  265,  248,
         244,  259,  269,  275,  273,  264,  254,  245,
         238,  253,  256,  260,  259,  255,  245,  243,
         230,  223,  245,  247,  249,  237,  228,  226,
    ],
    // bishop mg
    [
         254,  250,  240,  217,  225,  227,  271,  236,
         276,  294,  293,  277,  296,  295,  289,  276,
         285,  302,  301,  321,  313,  332,  317,  309,
         278,  293,  308,  319,  317,  311,  293,  281,
         273,  284,  292,  309,  306,  294,  287,  280,
         280,  290,  289,  294,  295,  291,  292,  292,
         283,  285,  297,  277,  284,  295,  301,  288,
         268,  285,  268,  262,  264,  266,  284,  278,
    ],
    // bishop eg
    [
         263,  270,  271,  278,  274,  272,  264,  263,
         258,  267,  269,  272,  266,  265,  270,  256,
         272,  269,  278,  269,  272,  275,  269,  268,
         272,  282,  277,  284,  281,  280,  283,  270,
         272,  281,  285,  282,  281,  283,  279,  264,
         269,  276,  281,  280,  285,  280,  270,  264,
         267,  268,  263,  275,  275,  267,  267,  254,
         258,  263,  255,  268,  265,  267,  259,  250,
    ],
    // rook mg
    [
         368,  360,  361,  367,  379,  387,  393,  408,
         356,  353,  371,  385,  373,  404,  397,  409,
         339,  353,  355,  358,  383,  394,  421,  398,
         333,  344,  342,  349,  354,  365,  370,  371,
         330,  332,  328,  339,  340,  340,  356,  355,
         331,  332,  332,  335,  342,  346,  374,  360,
         330,  332,  337,  337,  343,  353,  360,  338,
         347,  341,  340,  347,  353,  356,  355,  348,
    ],
    // rook eg
    [
         472,  480,  485,  479,  477,  475,  473,  466,
         479,  484,  483,  475,  478,  468,  467,  459,
         478,  476,  475,  472,  464,  460,  452,  454,
         478,  474,  477,  474,  467,  463,  460,  455,
         475,  473,  475,  472,  470,  469,  462,  457,
         471,  469,  467,  468,  464,  460,  446,  448,
         467,  467,  466,  466,  461,  456,  450,  455,
         469,  466,  472,  468,  462,  465,  459,  460,
    ],
    // queen mg
    [
         698,  718,  731,  754,  764,  774,  800,  746,
         737,  723,  731,  725,  730,  764,  754,  780,
         739,  739,  742,  754,  761,  793,  795,  784,
         727,  735,  738,  739,  743,  754,  750,  757,
         732,  731,  731,  739,  740,  738,  748,  747,
         730,  737,  733,  734,  736,  742,  752,  746,
         728,  733,  742,  742,  741,  749,  753,  756,
         726,  720,  724,  738,  733,  723,  730,  727,
    ],
    // queen eg
    [
         902,  905,  921,  916,  910,  898,  857,  888,
         874,  901,  924,  938,  952,  920,  904,  886,
         878,  889,  914,  917,  929,  912,  885,  885,
         888,  901,  910,  926,  936,  923,  919,  899,
         881,  904,  909,  921,  918,  917,  901,  895,
         871,  886,  903,  902,  907,  900,  882,  874,
         870,  876,  870,  877,  881,  861,  842,  820,
         862,  870,  876,  869,  868,  866,  850,  850,
    ],
    // king mg
    [
          31,   -5,   12,  -49,  -38,   -8,   24,  107,
         -65,  -34,  -48,   15,   -8,  -32,   -2,   -6,
         -88,   -9,  -48,  -65,  -33,    1,  -13,  -48,
         -66,  -59,  -80, -104,  -97,  -86,  -85, -102,
         -72,  -68,  -86, -103, -106,  -93,  -90, -104,
         -49,  -32,  -68,  -71,  -65,  -73,  -42,  -55,
          23,   -3,  -15,  -41,  -46,  -26,    3,    9,
          13,   39,   21,  -59,   -7,  -47,   20,   19,
    ],
    // king eg
    [
         -85,  -32,  -25,    2,  -10,   -8,   -9,  -90,
          -9,   16,   20,    8,   21,   31,   23,   -1,
           4,   19,   34,   40,   36,   35,   33,   10,
           0,   24,   40,   48,   49,   44,   37,   16,
          -2,   20,   38,   50,   49,   38,   27,   14,
          -2,   14,   28,   36,   33,   26,   13,    4,
         -16,    2,    9,   18,   18,   10,   -3,  -17,
         -38,  -34,  -19,    1,  -19,   -3,  -25,  -48,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          71,   73,   69,   82,   74,   66,   26,   23,
          16,   32,   27,   13,   15,    6,  -23,  -43,
           6,    5,   13,   14,   -1,    5,  -19,  -15,
          -4,  -10,  -15,   -9,  -16,   -7,  -21,  -15,
          -9,  -18,  -18,  -16,  -16,  -14,  -19,    1,
         -16,   -5,  -10,  -15,   -6,   -2,    2,   -5,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          92,   91,   94,   76,   80,   80,   99,  100,
          77,   77,   65,   43,   57,   71,   72,   84,
          40,   40,   35,   27,   32,   36,   47,   46,
          22,   22,   20,   15,   19,   21,   31,   28,
           0,    5,   11,    3,    8,   10,   19,    4,
           5,    4,   13,    6,   -1,    4,    8,    6,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 27;
const ROOK_OPEN_FILE_EG: i32 = 7;
const ROOK_SEMIOPEN_FILE_MG: i32 = -13;
const ROOK_SEMIOPEN_FILE_EG: i32 = -3;
const ROOK_CLOSED_FILE_MG: i32 = 5;
const ROOK_CLOSED_FILE_EG: i32 = 8;
const KING_OPEN_FILE_MG: i32 = -61;
const KING_OPEN_FILE_EG: i32 = -6;
const KING_SEMIOPEN_FILE_MG: i32 = 12;
const KING_SEMIOPEN_FILE_EG: i32 = -13;
const KING_CLOSED_FILE_MG: i32 = -29;
const KING_CLOSED_FILE_EG: i32 = 8;

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
