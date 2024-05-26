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
          96,   90,   93,  104,   97,   84,   36,   31,
          78,   95,  116,  120,  129,  176,  157,  120,
          71,   93,   89,   97,  114,  112,  109,   92,
          57,   86,   80,   99,   94,   94,   95,   68,
          57,   81,   76,   75,   87,   87,  112,   84,
          57,   82,   70,   62,   76,  108,  125,   79,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         122,  123,  118,   98,  100,  106,  128,  128,
         122,  118,   99,   94,   94,   80,  107,  103,
         113,  108,   93,   82,   84,   83,   98,   90,
         104,  104,   90,   85,   88,   87,   94,   87,
         101,  101,   88,   98,   96,   91,   92,   82,
         105,  104,   97,   99,  105,   95,   90,   83,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight mg
    [
         170,  255,  293,  312,  373,  251,  284,  228,
         316,  345,  387,  400,  391,  443,  359,  360,
         349,  390,  409,  426,  460,  470,  413,  376,
         355,  372,  393,  421,  394,  422,  372,  391,
         341,  358,  375,  375,  385,  381,  380,  348,
         323,  348,  364,  370,  380,  366,  372,  335,
         309,  320,  341,  352,  353,  359,  346,  340,
         272,  321,  303,  320,  325,  341,  324,  290,
    ],
    // knight eg
    [
         260,  288,  311,  304,  295,  300,  287,  223,
         291,  308,  307,  309,  303,  287,  296,  275,
         296,  307,  324,  326,  310,  308,  302,  290,
         305,  325,  337,  338,  341,  334,  328,  296,
         308,  320,  338,  342,  343,  331,  316,  302,
         292,  311,  317,  330,  328,  315,  303,  291,
         285,  302,  306,  310,  309,  302,  293,  288,
         267,  265,  294,  297,  296,  285,  267,  266,
    ],
    // bishop mg
    [
         369,  332,  340,  327,  317,  328,  367,  348,
         368,  407,  396,  381,  407,  405,  397,  361,
         384,  415,  413,  434,  432,  450,  431,  419,
         379,  398,  420,  434,  428,  422,  401,  385,
         377,  395,  401,  423,  420,  401,  396,  384,
         389,  400,  400,  403,  406,  401,  400,  402,
         391,  396,  403,  386,  392,  406,  418,  396,
         367,  388,  376,  364,  367,  367,  392,  375,
    ],
    // bishop eg
    [
         315,  327,  324,  332,  331,  326,  321,  317,
         313,  322,  328,  329,  323,  322,  325,  311,
         328,  326,  335,  328,  331,  334,  326,  322,
         329,  342,  335,  344,  343,  339,  340,  325,
         326,  337,  344,  341,  340,  342,  334,  318,
         320,  332,  337,  337,  342,  335,  321,  314,
         319,  314,  317,  329,  331,  320,  315,  300,
         304,  313,  302,  323,  319,  317,  301,  297,
    ],
    // rook mg
    [
         513,  504,  508,  515,  528,  533,  548,  554,
         484,  484,  507,  526,  512,  548,  542,  553,
         471,  492,  492,  497,  527,  540,  566,  533,
         460,  470,  478,  489,  491,  502,  508,  508,
         456,  457,  453,  468,  470,  469,  489,  481,
         452,  455,  456,  464,  471,  477,  512,  492,
         453,  458,  463,  467,  474,  486,  497,  462,
         477,  471,  471,  479,  486,  491,  486,  484,
    ],
    // rook eg
    [
         579,  587,  589,  583,  580,  580,  575,  572,
         591,  596,  592,  583,  585,  576,  575,  565,
         588,  584,  585,  580,  569,  566,  561,  563,
         587,  584,  585,  579,  573,  571,  568,  561,
         578,  579,  581,  576,  573,  572,  563,  558,
         569,  570,  569,  567,  564,  559,  542,  543,
         566,  567,  568,  564,  559,  553,  548,  555,
         569,  566,  574,  568,  560,  562,  558,  552,
    ],
    // queen mg
    [
         989, 1005, 1027, 1063, 1057, 1080, 1101, 1043,
        1016, 1004, 1013, 1000, 1009, 1053, 1034, 1077,
        1023, 1022, 1023, 1045, 1059, 1099, 1099, 1084,
        1014, 1019, 1026, 1026, 1031, 1042, 1038, 1051,
        1018, 1018, 1022, 1024, 1032, 1030, 1043, 1039,
        1014, 1031, 1027, 1028, 1032, 1038, 1048, 1037,
        1015, 1029, 1037, 1038, 1037, 1048, 1053, 1048,
        1020, 1011, 1017, 1035, 1023, 1010, 1010, 1017,
    ],
    // queen eg
    [
        1095, 1106, 1117, 1103, 1110, 1090, 1052, 1091,
        1071, 1101, 1131, 1154, 1163, 1124, 1108, 1068,
        1064, 1089, 1121, 1117, 1130, 1106, 1072, 1068,
        1071, 1098, 1108, 1130, 1142, 1130, 1115, 1087,
        1069, 1094, 1098, 1125, 1109, 1105, 1089, 1076,
        1055, 1059, 1087, 1082, 1084, 1076, 1059, 1048,
        1054, 1052, 1045, 1052, 1052, 1026, 1005,  990,
        1042, 1045, 1050, 1029, 1044, 1033, 1026, 1018,
    ],
    // king mg
    [
          60,   34,   59,    8,   35,   37,   46,  106,
         -45,   19,   10,   24,   31,   29,   10,  -11,
         -42,   52,  -17,  -17,   -5,   45,   26,  -19,
         -45,  -39,  -48,  -70,  -77,  -54,  -63,  -87,
         -66,  -52,  -76,  -99, -102,  -97,  -93, -111,
         -53,  -31,  -78,  -83,  -78,  -83,  -47,  -64,
          26,   -5,  -24,  -60,  -60,  -33,    8,   13,
           6,   47,   24,  -67,   -4,  -61,   27,   22,
    ],
    // king eg
    [
         -89,  -32,  -28,   -7,  -13,   -3,   -5,  -67,
          -9,   16,   19,   16,   21,   35,   36,    5,
           1,   21,   36,   41,   40,   41,   43,   17,
          -5,   27,   43,   53,   55,   48,   43,   19,
          -8,   20,   41,   56,   55,   45,   31,   15,
          -5,   13,   32,   42,   41,   32,   15,    3,
         -21,    1,   12,   23,   22,   11,   -6,  -23,
         -45,  -43,  -26,   -0,  -30,   -8,  -36,  -63,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns mg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          96,   90,   93,  104,   97,   84,   36,   31,
          41,   42,   30,   17,   17,    4,  -43,  -54,
          11,   10,   17,   15,    2,    5,  -21,  -20,
           0,  -13,  -18,  -14,  -17,  -12,  -28,   -8,
         -10,  -21,  -24,  -23,  -15,  -10,  -18,    5,
         -17,  -12,  -16,  -24,   -3,   -0,   11,   -4,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns eg
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         122,  123,  118,   98,  100,  106,  128,  128,
         103,  100,   89,   67,   63,   89,   96,  114,
          54,   50,   42,   36,   37,   43,   58,   58,
          28,   27,   24,   20,   21,   23,   38,   33,
           1,    6,   12,    5,    8,    9,   18,    5,
           3,    7,   13,    7,    2,    4,    6,    6,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 34;
const ROOK_OPEN_FILE_EG: i32 = 8;
const ROOK_CLOSED_FILE_MG: i32 = -16;
const ROOK_CLOSED_FILE_EG: i32 = -4;
const ROOK_SEMIOPEN_FILE_MG: i32 = 7;
const ROOK_SEMIOPEN_FILE_EG: i32 = 12;
const KING_OPEN_FILE_MG: i32 = -69;
const KING_OPEN_FILE_EG: i32 = -10;
const KING_CLOSED_FILE_MG: i32 = 15;
const KING_CLOSED_FILE_EG: i32 = -16;
const KING_SEMIOPEN_FILE_MG: i32 = -34;
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
