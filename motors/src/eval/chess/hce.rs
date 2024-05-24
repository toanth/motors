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
          70,   71,   69,   82,   76,   66,   29,   22,
          67,   76,   96,  103,  108,  143,  125,   99,
          56,   76,   74,   78,   96,   91,   91,   75,
          46,   70,   66,   82,   78,   76,   77,   57,
          45,   67,   62,   62,   73,   70,   90,   68,
          45,   67,   57,   51,   65,   87,  100,   64,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          99,   98,   98,   80,   83,   86,  104,  105,
          98,   96,   83,   81,   68,   62,   88,   85,
          90,   86,   75,   68,   66,   65,   78,   72,
          83,   84,   74,   70,   71,   69,   76,   70,
          81,   81,   72,   79,   77,   73,   74,   66,
          85,   85,   80,   81,   85,   75,   74,   67,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         137,  193,  230,  234,  282,  205,  209,  184,
         260,  282,  310,  319,  313,  357,  293,  295,
         285,  312,  328,  342,  372,  372,  336,  304,
         285,  300,  316,  340,  322,  341,  303,  314,
         272,  286,  301,  303,  310,  307,  305,  280,
         256,  278,  292,  295,  304,  294,  297,  269,
         244,  258,  273,  281,  284,  287,  278,  272,
         219,  253,  242,  258,  259,  271,  256,  229,
    ],
    // knight eg
    [
         204,  236,  249,  250,  242,  237,  233,  184,
         236,  246,  252,  250,  243,  232,  238,  221,
         240,  249,  263,  263,  250,  248,  241,  235,
         247,  265,  273,  273,  276,  268,  267,  239,
         253,  261,  274,  279,  280,  270,  260,  246,
         240,  255,  263,  272,  270,  260,  249,  241,
         235,  249,  253,  257,  256,  251,  242,  238,
         225,  218,  242,  244,  245,  235,  222,  223,
    ],
    // bishop mg
    [
         294,  275,  271,  253,  250,  261,  297,  271,
         301,  327,  321,  306,  327,  324,  321,  298,
         312,  331,  330,  350,  344,  362,  347,  340,
         306,  321,  339,  348,  347,  340,  323,  310,
         301,  316,  323,  340,  338,  324,  317,  308,
         313,  321,  321,  324,  326,  322,  322,  324,
         313,  319,  327,  309,  316,  326,  335,  319,
         299,  314,  300,  293,  294,  294,  314,  308,
    ],
    // bishop eg
    [
         257,  266,  267,  273,  270,  268,  261,  259,
         255,  263,  266,  268,  263,  261,  266,  253,
         269,  266,  275,  267,  268,  272,  265,  264,
         270,  280,  274,  281,  279,  276,  279,  266,
         269,  277,  282,  279,  277,  280,  274,  260,
         265,  272,  278,  278,  281,  277,  266,  259,
         263,  263,  260,  272,  273,  264,  262,  251,
         254,  260,  250,  265,  263,  264,  254,  247,
    ],
    // rook mg
    [
         407,  400,  401,  406,  420,  428,  435,  445,
         392,  388,  409,  424,  412,  443,  437,  447,
         377,  391,  392,  397,  421,  433,  459,  437,
         370,  381,  381,  388,  391,  404,  410,  411,
         368,  369,  364,  376,  377,  379,  396,  393,
         368,  370,  370,  374,  380,  385,  414,  399,
         368,  370,  374,  375,  380,  392,  400,  375,
         385,  380,  379,  386,  391,  395,  393,  389,
    ],
    // rook eg
    [
         472,  478,  483,  478,  475,  472,  470,  465,
         478,  485,  482,  474,  476,  467,  466,  458,
         478,  475,  474,  471,  462,  458,  451,  453,
         478,  473,  476,  473,  466,  462,  459,  454,
         473,  472,  475,  471,  469,  467,  460,  456,
         469,  467,  467,  466,  463,  458,  444,  446,
         467,  467,  467,  465,  460,  455,  449,  456,
         469,  466,  472,  467,  460,  464,  459,  457,
    ],
    // queen mg
    [
         789,  804,  819,  845,  854,  865,  886,  831,
         825,  813,  820,  808,  815,  852,  841,  870,
         827,  828,  828,  847,  852,  885,  888,  878,
         816,  823,  829,  828,  832,  844,  839,  847,
         820,  822,  821,  827,  831,  829,  840,  837,
         819,  828,  824,  825,  827,  833,  843,  835,
         817,  825,  833,  834,  832,  841,  845,  846,
         818,  811,  816,  831,  823,  811,  817,  819,
    ],
    // queen eg
    [
         891,  900,  913,  906,  900,  887,  848,  884,
         866,  891,  915,  934,  946,  913,  895,  872,
         868,  880,  909,  905,  920,  901,  870,  869,
         877,  893,  899,  918,  929,  915,  909,  887,
         876,  894,  899,  916,  908,  906,  889,  883,
         861,  873,  894,  891,  897,  888,  872,  864,
         861,  866,  860,  867,  870,  848,  830,  811,
         854,  859,  864,  852,  859,  855,  841,  838,
    ],
    // king mg
    [
          28,    5,   24,  -24,  -20,   11,   26,   69,
         -56,  -12,  -24,   14,    4,   -5,   -4,  -18,
         -69,   18,  -33,  -44,  -28,   13,    3,  -42,
         -56,  -47,  -63,  -92,  -89,  -76,  -71,  -98,
         -63,  -57,  -77,  -99, -102,  -93,  -86, -104,
         -43,  -28,  -67,  -72,  -67,  -73,  -41,  -55,
          26,   -1,  -16,  -47,  -50,  -27,    4,   10,
          13,   41,   22,  -58,   -5,  -50,   20,   19,
    ],
    // king eg
    [
         -80,  -31,  -24,   -2,  -10,   -7,   -9,  -70,
         -10,   13,   16,    9,   18,   28,   25,    3,
           1,   16,   31,   35,   33,   33,   32,   10,
          -4,   21,   37,   46,   46,   42,   35,   17,
          -6,   18,   36,   49,   48,   38,   26,   14,
          -5,   13,   29,   37,   35,   28,   13,    4,
         -17,    2,   10,   21,   21,   12,   -2,  -17,
         -38,  -34,  -20,    2,  -19,   -2,  -26,  -49,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          70,   71,   69,   82,   76,   66,   29,   22,
          17,   31,   23,   10,   12,    1,  -32,  -49,
           6,    5,   13,   13,   -1,    4,  -22,  -18,
          -3,  -11,  -15,  -11,  -16,   -9,  -24,  -10,
          -9,  -18,  -20,  -19,  -16,  -12,  -17,    3,
         -16,   -6,  -10,  -17,   -5,   -1,    6,   -4,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          99,   98,   98,   80,   83,   86,  104,  105,
          85,   84,   72,   50,   60,   77,   80,   92,
          44,   43,   36,   30,   32,   37,   50,   48,
          24,   23,   21,   16,   19,   21,   32,   27,
           1,    6,   12,    4,    7,   10,   18,    4,
           5,    6,   12,    5,    0,    4,    7,    6,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 28;
const ROOK_OPEN_FILE_EG: i32 = 7;
const ROOK_SEMIOPEN_FILE_MG: i32 = -14;
const ROOK_SEMIOPEN_FILE_EG: i32 = -3;
const ROOK_CLOSED_FILE_MG: i32 = 5;
const ROOK_CLOSED_FILE_EG: i32 = 9;
const KING_OPEN_FILE_MG: i32 = -60;
const KING_OPEN_FILE_EG: i32 = -7;
const KING_SEMIOPEN_FILE_MG: i32 = 12;
const KING_SEMIOPEN_FILE_EG: i32 = -14;
const KING_CLOSED_FILE_MG: i32 = -28;
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
