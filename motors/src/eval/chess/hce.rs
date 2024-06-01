use strum::IntoEnumIterator;

use crate::eval::chess::{
    pawn_shield_idx, FileOpenness, NUM_PAWN_SHIELD_CONFIGURATIONS, NUM_PHASES,
};
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
use crate::eval::chess::PhaseType::{Eg, Mg};
use crate::eval::Eval;

#[derive(Default, Debug)]
pub struct HandCraftedEval {}

/// Eval values tuned on a combination of the zurichess dataset and a dataset used by 4ku,
/// created by GCP using his engine Stoofvlees and filtered by cj5716 using Stockfish at depth 9,
/// using my own tuner `pliers`.
#[rustfmt::skip]
const PSQTS: [[i32; NUM_SQUARES]; NUM_CHESS_PIECES * 2] = [
    // pawn MG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         156,  166,  156,  182,  167,  149,   70,   48,
          75,   85,  104,  110,  119,  156,  140,  102,
          61,   83,   79,   83,  106,  101,   95,   77,
          50,   74,   70,   85,   85,   85,   86,   62,
          48,   70,   63,   64,   76,   70,   86,   58,
          48,   68,   58,   42,   62,   73,   91,   52,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         268,  264,  260,  216,  215,  226,  271,  280,
         122,  121,  101,  103,   94,   88,  116,  111,
         114,  110,   95,   85,   85,   86,  102,   93,
         104,  106,   93,   89,   88,   88,   97,   87,
         101,  103,   91,   99,   96,   93,   95,   87,
         105,  106,   96,  101,  107,  101,   96,   91,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight MG
    [
         138,  186,  232,  263,  306,  219,  223,  181,
         267,  289,  322,  338,  318,  379,  294,  304,
         287,  324,  346,  358,  390,  396,  349,  311,
         289,  306,  330,  352,  332,  357,  312,  322,
         278,  294,  311,  310,  319,  314,  315,  289,
         259,  283,  295,  303,  312,  300,  304,  273,
         245,  257,  276,  285,  286,  286,  276,  274,
         212,  253,  241,  256,  263,  273,  256,  232,
    ],
    // knight EG
    [
         257,  308,  328,  316,  315,  309,  303,  235,
         304,  322,  324,  325,  323,  306,  316,  290,
         312,  325,  339,  343,  330,  325,  319,  307,
         322,  341,  352,  353,  356,  350,  342,  314,
         324,  335,  353,  356,  357,  346,  333,  314,
         309,  327,  336,  345,  343,  331,  319,  309,
         301,  318,  322,  325,  323,  318,  306,  305,
         289,  281,  308,  311,  308,  297,  288,  284,
    ],
    // bishop MG
    [
         292,  274,  278,  258,  267,  268,  310,  281,
         311,  336,  334,  318,  344,  347,  338,  314,
         322,  348,  347,  368,  366,  386,  368,  350,
         316,  335,  353,  369,  362,  356,  334,  321,
         314,  326,  334,  353,  349,  334,  327,  322,
         319,  332,  331,  335,  334,  330,  332,  331,
         326,  325,  337,  315,  323,  329,  339,  325,
         303,  324,  306,  296,  300,  300,  319,  307,
    ],
    // bishop EG
    [
         332,  344,  340,  349,  345,  340,  335,  330,
         325,  338,  342,  345,  336,  336,  339,  323,
         343,  341,  349,  342,  345,  348,  340,  340,
         342,  355,  350,  359,  357,  354,  355,  341,
         340,  353,  359,  356,  355,  355,  350,  332,
         338,  347,  353,  351,  356,  349,  337,  330,
         333,  332,  331,  343,  344,  334,  330,  315,
         319,  329,  316,  336,  331,  330,  317,  307,
    ],
    // rook MG
    [
         424,  414,  422,  427,  440,  450,  455,  462,
         397,  399,  419,  436,  423,  460,  450,  460,
         381,  404,  405,  407,  437,  449,  477,  439,
         374,  384,  388,  399,  403,  414,  415,  413,
         367,  369,  369,  382,  382,  383,  399,  392,
         366,  367,  369,  374,  381,  388,  420,  399,
         367,  370,  376,  377,  384,  390,  403,  377,
         387,  381,  379,  384,  390,  391,  393,  390,
    ],
    // rook EG
    [
         596,  605,  608,  602,  599,  598,  597,  592,
         608,  613,  612,  602,  605,  594,  594,  584,
         606,  602,  602,  598,  588,  584,  578,  581,
         604,  601,  603,  597,  589,  586,  586,  579,
         597,  597,  597,  592,  589,  588,  581,  577,
         588,  588,  586,  585,  580,  576,  559,  562,
         584,  585,  585,  582,  576,  571,  564,  571,
         587,  585,  592,  585,  578,  578,  575,  575,
    ],
    // queen MG
    [
         801,  818,  840,  870,  865,  881,  913,  856,
         830,  815,  825,  818,  823,  865,  856,  888,
         835,  831,  839,  849,  867,  900,  902,  885,
         823,  829,  834,  837,  840,  849,  846,  856,
         828,  825,  830,  834,  836,  835,  847,  847,
         824,  835,  831,  831,  833,  841,  852,  845,
         824,  833,  842,  839,  839,  844,  851,  852,
         823,  816,  820,  835,  828,  811,  817,  819,
    ],
    // queen EG
    [
        1140, 1147, 1163, 1152, 1159, 1142, 1097, 1128,
        1111, 1149, 1175, 1193, 1207, 1167, 1144, 1119,
        1112, 1135, 1161, 1169, 1175, 1157, 1123, 1121,
        1120, 1142, 1156, 1175, 1186, 1172, 1163, 1137,
        1112, 1143, 1147, 1167, 1159, 1154, 1137, 1127,
        1104, 1116, 1138, 1133, 1135, 1125, 1107, 1098,
        1100, 1100, 1094, 1103, 1104, 1077, 1055, 1038,
        1090, 1093, 1098, 1085, 1083, 1078, 1066, 1069,
    ],
    // king MG
    [
          92,   39,   74,  -15,   28,   19,   59,  175,
         -22,   17,   -1,   68,   46,   22,   45,   35,
         -28,   50,    6,   -8,   33,   74,   40,   12,
         -12,  -17,  -30,  -50,  -47,  -28,  -44,  -59,
         -38,  -30,  -45,  -64,  -63,  -50,  -63,  -83,
         -28,   -2,  -37,  -41,  -37,  -43,  -22,  -35,
          26,   16,    6,  -23,  -22,  -10,   27,   33,
         -11,   41,   34,  -36,   17,  -35,   29,   36,
    ],
    // king EG
    [
        -120,  -55,  -48,  -14,  -30,  -21,  -28, -119,
         -32,    5,   13,    1,   12,   25,   17,  -24,
         -19,    9,   28,   39,   33,   31,   27,   -8,
         -25,   15,   36,   48,   49,   39,   29,   -4,
         -26,    6,   29,   47,   44,   29,   14,   -6,
         -23,   -4,   16,   26,   24,   14,   -3,  -14,
         -33,  -17,   -7,    3,    2,   -6,  -24,  -31,
         -54,  -56,  -43,  -21,  -46,  -26,  -50,  -69,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns MG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          17,   12,   17,   16,    9,    7,   -6,    2,
          30,   40,   32,   19,   19,    8,  -24,  -44,
          10,    8,   18,   17,   -2,    6,  -14,  -13,
          -3,  -12,  -16,  -11,  -21,  -13,  -22,  -12,
         -10,  -21,  -21,  -18,  -21,  -15,  -17,    5,
         -16,   -9,  -16,  -16,   -4,   -3,    5,    1,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         -21,  -19,  -17,  -16,  -10,  -12,  -14,  -16,
         104,  102,   89,   60,   66,   84,   91,  109,
          55,   52,   42,   35,   38,   42,   57,   60,
          30,   27,   24,   18,   21,   24,   37,   35,
           1,    6,   11,    1,    7,    7,   19,    5,
           3,    5,   14,    8,   -2,    0,    4,    4,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 31;
const ROOK_OPEN_FILE_EG: i32 = 11;
const ROOK_CLOSED_FILE_MG: i32 = -14;
const ROOK_CLOSED_FILE_EG: i32 = -4;
const ROOK_SEMIOPEN_FILE_MG: i32 = 7;
const ROOK_SEMIOPEN_FILE_EG: i32 = 12;
const KING_OPEN_FILE_MG: i32 = -49;
const KING_OPEN_FILE_EG: i32 = -6;
const KING_CLOSED_FILE_MG: i32 = 13;
const KING_CLOSED_FILE_EG: i32 = -14;
const KING_SEMIOPEN_FILE_MG: i32 = -9;
const KING_SEMIOPEN_FILE_EG: i32 = 7;
const PAWN_SHIELDS: [[i32; NUM_PHASES]; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    [-137, -87],  /*0b0000*/
    [-129, -93],  /*0b0001*/
    [-115, -96],  /*0b0010*/
    [-99, -87],   /*0b0011*/
    [-108, -97],  /*0b0100*/
    [-119, -114], /*0b0101*/
    [-100, -95],  /*0b0110*/
    [-83, -121],  /*0b0111*/
    [-120, -89],  /*0b1000*/
    [-134, -125], /*0b1001*/
    [-108, -85],  /*0b1010*/
    [-91, -110],  /*0b1011*/
    [-108, -101], /*0b1100*/
    [-122, -132], /*0b1101*/
    [-82, -89],   /*0b1110*/
    [-100, -100], /*0b1111*/
    [-142, -91],  /*0b10000*/
    [-100, -86],  /*0b10001*/
    [-128, -127], /*0b10010*/
    [-95, -113],  /*0b10011*/
    [-107, -93],  /*0b10100*/
    [-76, -95],   /*0b10101*/
    [-113, -124], /*0b10110*/
    [-100, -100], /*0b10111*/
    [-125, -66],  /*0b11000*/
    [-99, -96],   /*0b11001*/
    [-90, -87],   /*0b11010*/
    [-100, -100], /*0b11011*/
    [-91, -90],   /*0b11100*/
    [-100, -100], /*0b11101*/
    [-100, -100], /*0b11110*/
    [-100, -100], /*0b11111*/
    [-108, -91],  /*0b100000*/
    [-110, -95],  /*0b100001*/
    [-90, -95],   /*0b100010*/
    [-74, -95],   /*0b100011*/
    [-128, -134], /*0b100100*/
    [-122, -152], /*0b100101*/
    [-111, -110], /*0b100110*/
    [-100, -100], /*0b100111*/
    [-119, -106], /*0b101000*/
    [-137, -121], /*0b101001*/
    [-86, -97],   /*0b101010*/
    [-100, -100], /*0b101011*/
    [-140, -138], /*0b101100*/
    [-100, -100], /*0b101101*/
    [-100, -100], /*0b101110*/
    [-100, -100], /*0b101111*/
    [-119, -82],  /*0b110000*/
    [-83, -87],   /*0b110001*/
    [-98, -124],  /*0b110010*/
    [-100, -100], /*0b110011*/
    [-107, -100], /*0b110100*/
    [-100, -100], /*0b110101*/
    [-100, -100], /*0b110110*/
    [-100, -100], /*0b110111*/
    [-126, -86],  /*0b111000*/
    [-100, -100], /*0b111001*/
    [-100, -100], /*0b111010*/
    [-100, -100], /*0b111011*/
    [-100, -100], /*0b111100*/
    [-100, -100], /*0b111101*/
    [-100, -100], /*0b111110*/
    [-94, -125],  /*0b111111*/
    [-157, -81],  /*0b00*/
    [-103, -108], /*0b01*/
    [-80, -101],  /*0b10*/
    [-56, -134],  /*0b11*/
    [-103, -103], /*0b100*/
    [-130, -148], /*0b101*/
    [-41, -131],  /*0b110*/
    [-100, -100], /*0b111*/
    [-71, -98],   /*0b1000*/
    [-91, -121],  /*0b1001*/
    [-51, -192],  /*0b1010*/
    [-100, -100], /*0b1011*/
    [-83, -109],  /*0b1100*/
    [-100, -100], /*0b1101*/
    [-100, -100], /*0b1110*/
    [-89, -119],  /*0b1111*/
    [-147, -84],  /*0b00*/
    [-119, -107], /*0b01*/
    [-116, -103], /*0b10*/
    [-97, -131],  /*0b11*/
    [-127, -98],  /*0b100*/
    [-107, -154], /*0b101*/
    [-117, -111], /*0b110*/
    [-100, -100], /*0b111*/
    [-139, -95],  /*0b1000*/
    [-92, -106],  /*0b1001*/
    [-108, -170], /*0b1010*/
    [-100, -100], /*0b1011*/
    [-129, -108], /*0b1100*/
    [-100, -100], /*0b1101*/
    [-100, -100], /*0b1110*/
    [-111, -162], /*0b1111*/
];
const VIRTUAL_QUEEN_MOBILITY: [[i32; NUM_PHASES]; NUM_VIRTUAL_QUEEN_MOBILITY_FEATURES] = [
    [13, 16],
    [26, 10],
    [22, 1],
    [19, -3],
    [16, -3],
    [16, -7],
    [13, -5],
    [7, -5],
    [3, -3],
    [-4, -0],
    [-12, 4],
    [-21, 6],
    [-29, 8],
    [-40, 9],
    [-51, 12],
    [-65, 14],
    [-76, 12],
    [-85, 13],
    [-90, 11],
    [-86, -0],
];

// TODO: Differentiate between rooks and kings in front of / behind pawns.

const PIECE_PHASE: [i32; 6] = [0, 1, 1, 2, 4, 0];
pub const NUM_VIRTUAL_QUEEN_MOBILITY_FEATURES: usize = 20;

pub fn file_openness(
    file: DimT,
    our_pawns: ChessBitboard,
    their_pawns: ChessBitboard,
) -> FileOpenness {
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
            let king_square = pos.king_square(color);
            let king_file = king_square.file();
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
            Self::king_safety(pos, color, &mut mg, &mut eg);

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

impl HandCraftedEval {
    #[inline]
    fn king_safety(pos: Chessboard, color: Color, mg: &mut Score, eg: &mut Score) {
        let our_pawns = pos.colored_piece_bb(color, Pawn);
        let king_square = pos.king_square(color);
        *mg += Score(PAWN_SHIELDS[pawn_shield_idx(our_pawns, king_square, color)][Mg as usize]);
        *eg += Score(PAWN_SHIELDS[pawn_shield_idx(our_pawns, king_square, color)][Eg as usize]);
        let idx = pos
            .queen_moves_from_square(king_square, color)
            .num_set_bits()
            .min(NUM_VIRTUAL_QUEEN_MOBILITY_FEATURES - 1);
        *mg += Score(VIRTUAL_QUEEN_MOBILITY[idx][Mg as usize]);
        *eg += Score(VIRTUAL_QUEEN_MOBILITY[idx][Eg as usize]);
    }
}
