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
          65,   65,   63,   75,   70,   60,   26,   20,
          61,   69,   88,   95,   99,  131,  114,   90,
          52,   69,   68,   71,   88,   84,   83,   69,
          42,   64,   60,   75,   72,   70,   71,   52,
          41,   61,   57,   57,   67,   64,   82,   62,
          41,   62,   53,   47,   59,   80,   91,   58,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          91,   90,   90,   74,   76,   79,   95,   96,
          89,   88,   76,   75,   63,   57,   81,   78,
          83,   79,   69,   62,   60,   59,   71,   66,
          76,   77,   68,   64,   65,   63,   69,   64,
          74,   75,   66,   73,   71,   67,   68,   61,
          78,   78,   73,   74,   78,   69,   67,   62,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         126,  177,  211,  214,  259,  188,  191,  169,
         238,  258,  285,  293,  287,  328,  268,  270,
         261,  286,  301,  314,  341,  341,  308,  278,
         262,  275,  290,  312,  295,  312,  278,  288,
         250,  262,  276,  278,  285,  282,  280,  257,
         235,  255,  268,  271,  278,  270,  273,  247,
         224,  236,  250,  257,  260,  264,  255,  249,
         201,  232,  222,  236,  238,  249,  235,  210,
    ],
    // knight eg
    [
         187,  216,  229,  230,  222,  217,  213,  169,
         216,  226,  231,  229,  223,  213,  219,  203,
         220,  229,  241,  241,  229,  227,  221,  216,
         227,  243,  250,  250,  253,  246,  245,  219,
         232,  239,  251,  256,  256,  247,  239,  225,
         220,  234,  241,  250,  247,  238,  229,  221,
         215,  228,  231,  235,  234,  231,  221,  218,
         206,  200,  222,  224,  225,  216,  204,  204,
    ],
    // bishop mg
    [
         270,  252,  248,  232,  230,  239,  272,  249,
         276,  300,  295,  281,  300,  297,  294,  273,
         286,  304,  303,  321,  315,  332,  318,  312,
         280,  294,  310,  319,  318,  312,  296,  284,
         276,  290,  296,  312,  310,  297,  291,  282,
         286,  294,  294,  297,  299,  295,  295,  297,
         287,  292,  299,  283,  289,  299,  307,  292,
         274,  288,  275,  268,  269,  269,  288,  282,
    ],
    // bishop eg
    [
         235,  244,  245,  250,  248,  245,  239,  237,
         234,  241,  244,  245,  241,  240,  244,  232,
         247,  244,  252,  245,  246,  249,  243,  242,
         248,  256,  251,  258,  256,  253,  256,  244,
         247,  254,  258,  255,  254,  257,  251,  239,
         242,  250,  255,  254,  258,  253,  244,  238,
         241,  241,  238,  249,  250,  242,  240,  230,
         233,  239,  229,  243,  241,  242,  233,  226,
    ],
    // rook mg
    [
         373,  367,  367,  373,  385,  392,  399,  408,
         360,  356,  375,  389,  378,  406,  400,  410,
         345,  359,  360,  364,  386,  397,  421,  400,
         339,  349,  350,  356,  358,  370,  376,  376,
         338,  338,  334,  345,  346,  347,  363,  360,
         337,  339,  339,  343,  349,  353,  380,  366,
         337,  340,  343,  344,  349,  359,  367,  344,
         353,  348,  347,  354,  358,  362,  360,  356,
    ],
    // rook eg
    [
         433,  439,  443,  438,  435,  433,  430,  426,
         439,  445,  442,  435,  437,  428,  427,  419,
         438,  436,  435,  432,  424,  420,  413,  415,
         438,  433,  436,  434,  427,  424,  420,  416,
         434,  433,  436,  432,  430,  428,  421,  418,
         430,  428,  428,  428,  424,  420,  407,  409,
         428,  428,  428,  426,  422,  417,  412,  418,
         430,  427,  433,  428,  422,  425,  420,  419,
    ],
    // queen mg
    [
         723,  737,  751,  775,  783,  793,  812,  761,
         756,  745,  752,  741,  747,  781,  771,  797,
         758,  759,  759,  777,  781,  812,  814,  805,
         748,  754,  760,  759,  763,  774,  769,  777,
         751,  753,  752,  758,  762,  760,  770,  767,
         751,  759,  756,  757,  758,  764,  773,  765,
         749,  756,  764,  764,  763,  771,  775,  776,
         750,  743,  748,  761,  754,  744,  748,  750,
    ],
    // queen eg
    [
         817,  825,  836,  830,  825,  813,  778,  810,
         794,  816,  839,  856,  867,  836,  820,  800,
         796,  806,  833,  830,  844,  825,  798,  796,
         804,  818,  824,  842,  851,  839,  833,  813,
         803,  820,  824,  840,  833,  830,  815,  809,
         790,  800,  820,  816,  822,  814,  799,  792,
         789,  793,  788,  795,  797,  778,  761,  743,
         783,  788,  792,  781,  787,  784,  771,  768,
    ],
    // king mg
    [
          25,    5,   22,  -22,  -18,   10,   24,   63,
         -52,  -11,  -22,   13,    4,   -4,   -3,  -16,
         -63,   16,  -30,  -40,  -26,   12,    3,  -39,
         -52,  -43,  -58,  -84,  -81,  -70,  -65,  -90,
         -58,  -52,  -71,  -91,  -93,  -85,  -79,  -95,
         -39,  -26,  -62,  -66,  -62,  -67,  -37,  -50,
          24,   -1,  -14,  -43,  -45,  -25,    4,    9,
          12,   37,   20,  -53,   -5,  -45,   19,   17,
    ],
    // king eg
    [
         -73,  -28,  -22,   -2,   -9,   -7,   -8,  -64,
          -9,   12,   15,    8,   17,   26,   23,    3,
           1,   14,   29,   33,   31,   30,   30,   10,
          -4,   20,   34,   42,   43,   38,   32,   15,
          -5,   16,   33,   45,   44,   35,   24,   12,
          -4,   12,   26,   34,   32,   25,   12,    4,
         -16,    2,    9,   20,   19,   11,   -2,  -15,
         -35,  -31,  -18,    2,  -18,   -2,  -24,  -45,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          65,   65,   63,   75,   70,   60,   26,   20,
          16,   29,   21,   10,   11,    1,  -30,  -45,
           5,    4,   12,   12,    0,    4,  -20,  -17,
          -3,  -10,  -14,  -10,  -15,   -9,  -22,   -9,
          -8,  -16,  -18,  -17,  -14,  -11,  -16,    3,
         -14,   -6,   -9,  -16,   -5,   -1,    6,   -4,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          91,   90,   90,   74,   76,   79,   95,   96,
          78,   77,   66,   46,   55,   71,   73,   85,
          40,   40,   33,   27,   30,   34,   46,   44,
          22,   21,   19,   15,   17,   19,   30,   25,
           1,    6,   11,    4,    7,    9,   16,    4,
           5,    5,   11,    4,    0,    4,    6,    5,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 26;
const ROOK_OPEN_FILE_EG: i32 = 6;
const ROOK_SEMIOPEN_FILE_MG: i32 = -13;
const ROOK_SEMIOPEN_FILE_EG: i32 = -3;
const ROOK_CLOSED_FILE_MG: i32 = 5;
const ROOK_CLOSED_FILE_EG: i32 = 8;
const KING_OPEN_FILE_MG: i32 = -55;
const KING_OPEN_FILE_EG: i32 = -6;
const KING_SEMIOPEN_FILE_MG: i32 = 11;
const KING_SEMIOPEN_FILE_EG: i32 = -13;
const KING_CLOSED_FILE_MG: i32 = -25;
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
