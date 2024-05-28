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
         141,  150,  140,  164,  150,  134,   62,   42,
          61,   70,   87,   91,   97,  130,  117,   89,
          50,   68,   65,   69,   86,   83,   80,   68,
          41,   62,   58,   71,   69,   69,   68,   55,
          40,   58,   53,   54,   63,   63,   80,   60,
          39,   56,   50,   42,   55,   76,   88,   54,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         248,  243,  239,  198,  197,  208,  249,  257,
         101,  101,   83,   84,   78,   70,   95,   90,
          95,   92,   79,   71,   71,   71,   84,   76,
          87,   88,   77,   74,   75,   75,   81,   72,
          85,   87,   76,   83,   81,   77,   79,   70,
          88,   89,   81,   83,   90,   81,   79,   71,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         119,  154,  192,  218,  252,  178,  182,  153,
         222,  240,  267,  279,  267,  312,  242,  252,
         239,  269,  287,  296,  323,  330,  287,  261,
         240,  253,  273,  292,  275,  295,  259,  269,
         230,  243,  257,  257,  265,  260,  261,  240,
         215,  235,  245,  251,  260,  249,  253,  228,
         203,  213,  228,  238,  239,  242,  233,  228,
         175,  210,  202,  214,  219,  232,  215,  194,
    ],
    // knight eg
    [
         213,  257,  273,  264,  262,  258,  254,  196,
         254,  268,  270,  271,  268,  255,  263,  243,
         260,  271,  282,  285,  274,  270,  266,  255,
         269,  284,  294,  294,  296,  292,  285,  262,
         270,  279,  294,  297,  298,  290,  278,  263,
         258,  273,  280,  289,  287,  276,  267,  258,
         251,  266,  270,  272,  271,  267,  257,  257,
         241,  235,  258,  260,  260,  250,  240,  236,
    ],
    // bishop mg
    [
         242,  225,  229,  209,  215,  217,  252,  228,
         256,  278,  275,  261,  281,  279,  273,  253,
         266,  288,  287,  304,  298,  316,  299,  290,
         261,  276,  291,  303,  298,  294,  276,  264,
         258,  269,  275,  292,  289,  275,  271,  265,
         264,  274,  273,  277,  278,  274,  275,  276,
         268,  269,  279,  262,  268,  279,  285,  272,
         249,  268,  254,  247,  251,  251,  270,  258,
    ],
    // bishop eg
    [
         277,  287,  284,  292,  289,  284,  280,  276,
         272,  282,  286,  288,  281,  282,  284,  271,
         286,  285,  291,  285,  289,  291,  284,  283,
         286,  297,  292,  300,  298,  296,  297,  285,
         283,  295,  300,  298,  298,  297,  293,  276,
         282,  290,  295,  294,  299,  293,  282,  275,
         278,  277,  277,  287,  288,  279,  279,  263,
         266,  274,  265,  281,  277,  277,  265,  257,
    ],
    // rook mg
    [
         348,  339,  344,  349,  359,  366,  373,  382,
         328,  330,  345,  358,  348,  376,  371,  384,
         316,  335,  335,  335,  360,  370,  394,  369,
         310,  318,  321,  329,  333,  341,  345,  346,
         305,  306,  305,  316,  316,  314,  331,  327,
         303,  304,  305,  309,  316,  321,  350,  334,
         304,  307,  312,  313,  319,  328,  338,  315,
         322,  317,  316,  322,  328,  332,  332,  325,
    ],
    // rook eg
    [
         498,  505,  508,  503,  500,  500,  498,  493,
         506,  510,  509,  501,  504,  496,  495,  486,
         504,  501,  502,  498,  490,  486,  481,  482,
         503,  501,  503,  498,  491,  489,  488,  482,
         496,  497,  498,  494,  492,  492,  485,  480,
         490,  490,  488,  489,  485,  481,  467,  468,
         486,  487,  488,  485,  481,  477,  471,  476,
         489,  487,  494,  488,  482,  485,  479,  479,
    ],
    // queen mg
    [
         660,  674,  693,  717,  714,  729,  752,  702,
         686,  673,  681,  675,  680,  710,  699,  729,
         691,  688,  692,  701,  713,  743,  743,  734,
         681,  685,  689,  690,  693,  701,  700,  708,
         685,  682,  685,  689,  691,  690,  700,  700,
         681,  690,  688,  687,  691,  696,  705,  699,
         681,  689,  696,  696,  695,  703,  708,  709,
         682,  675,  679,  692,  685,  677,  685,  682,
    ],
    // queen eg
    [
         948,  955,  966,  958,  962,  948,  913,  942,
         924,  955,  978,  992, 1003,  971,  957,  935,
         923,  943,  966,  972,  979,  964,  937,  933,
         931,  950,  962,  978,  987,  976,  967,  947,
         925,  950,  953,  971,  964,  961,  947,  938,
         918,  927,  945,  943,  944,  938,  923,  913,
         915,  915,  910,  917,  919,  899,  879,  861,
         906,  911,  916,  907,  910,  903,  890,  889,
    ],
    // king mg
    [
          50,   20,   44,  -32,    3,    4,   43,  148,
         -46,  -10,  -26,   31,   11,   -9,   18,   14,
         -58,   16,  -26,  -38,   -8,   30,   10,  -12,
         -44,  -43,  -56,  -75,  -75,  -56,  -63,  -72,
         -57,  -50,  -69,  -82,  -83,  -72,  -74,  -88,
         -33,  -15,  -52,  -55,  -50,  -56,  -27,  -40,
          38,   12,   -1,  -25,  -28,  -10,   20,   25,
          27,   53,   35,  -38,   12,  -29,   37,   35,
    ],
    // king eg
    [
         -88,  -38,  -32,   -5,  -18,  -10,  -15,  -91,
         -14,   11,   17,    6,   16,   28,   21,   -9,
          -3,   14,   27,   36,   32,   30,   29,    5,
          -8,   19,   33,   42,   43,   36,   31,    7,
         -12,   11,   28,   40,   38,   28,   18,    4,
         -14,    1,   17,   24,   23,   15,    1,   -9,
         -30,  -11,   -3,    5,    5,   -4,  -17,  -30,
         -56,  -48,  -33,  -13,  -36,  -20,  -41,  -64,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
           2,   -4,    1,   -2,   -8,   -8,  -14,   -4,
          26,   33,   26,   15,   14,    6,  -25,  -40,
           9,    6,   15,   13,    0,    4,  -18,  -14,
          -2,  -10,  -15,   -9,  -15,   -8,  -20,  -13,
          -8,  -18,  -19,  -15,  -15,  -12,  -19,    2,
         -14,   -7,  -14,  -16,   -4,   -3,    2,   -3,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         -41,  -40,  -38,  -34,  -28,  -30,  -36,  -39,
          86,   85,   74,   50,   53,   71,   76,   92,
          45,   43,   35,   29,   31,   36,   48,   50,
          25,   22,   20,   15,   17,   19,   31,   29,
           0,    5,   10,    2,    6,    7,   17,    4,
           2,    4,   12,    8,   -1,    3,    5,    5,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 25;
const ROOK_OPEN_FILE_EG: i32 = 9;
const ROOK_CLOSED_FILE_MG: i32 = -12;
const ROOK_CLOSED_FILE_EG: i32 = -3;
const ROOK_SEMIOPEN_FILE_MG: i32 = 5;
const ROOK_SEMIOPEN_FILE_EG: i32 = 10;
const KING_OPEN_FILE_MG: i32 = -60;
const KING_OPEN_FILE_EG: i32 = -7;
const KING_CLOSED_FILE_MG: i32 = 12;
const KING_CLOSED_FILE_EG: i32 = -13;
const KING_SEMIOPEN_FILE_MG: i32 = -28;
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
