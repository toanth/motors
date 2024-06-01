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
          75,   85,  104,  110,  119,  156,  139,  102,
          61,   83,   79,   83,  106,  100,   95,   77,
          50,   74,   70,   85,   85,   85,   86,   62,
          48,   69,   63,   64,   76,   69,   86,   58,
          47,   67,   58,   42,   61,   72,   91,   52,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         268,  264,  260,  215,  215,  225,  271,  280,
         122,  121,  101,  103,   94,   88,  116,  112,
         114,  110,   96,   86,   85,   86,  103,   93,
         104,  106,   93,   90,   89,   89,   97,   88,
         101,  104,   92,  100,   97,   94,   96,   87,
         105,  107,   97,  102,  107,  102,   97,   91,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight MG
    [
         138,  186,  232,  263,  306,  219,  223,  180,
         267,  289,  322,  338,  318,  379,  294,  304,
         287,  324,  346,  358,  391,  396,  349,  310,
         289,  306,  330,  352,  332,  357,  312,  321,
         278,  294,  310,  309,  319,  314,  315,  289,
         259,  283,  294,  302,  311,  299,  304,  273,
         245,  257,  276,  285,  286,  286,  275,  274,
         211,  253,  241,  256,  263,  273,  256,  232,
    ],
    // knight EG
    [
         257,  308,  328,  317,  315,  310,  303,  236,
         305,  322,  324,  325,  323,  306,  316,  290,
         313,  326,  339,  343,  329,  325,  319,  307,
         323,  341,  352,  354,  356,  350,  342,  315,
         325,  335,  354,  357,  358,  347,  333,  315,
         309,  328,  336,  347,  344,  332,  320,  310,
         302,  319,  323,  326,  324,  319,  307,  306,
         290,  282,  309,  312,  309,  298,  288,  285,
    ],
    // bishop MG
    [
         292,  274,  278,  257,  266,  268,  310,  281,
         311,  336,  334,  318,  344,  347,  338,  314,
         322,  348,  347,  368,  366,  386,  368,  350,
         316,  335,  352,  368,  361,  356,  333,  321,
         314,  326,  334,  353,  348,  334,  327,  321,
         319,  332,  330,  335,  333,  330,  332,  331,
         326,  325,  337,  315,  323,  329,  339,  325,
         302,  324,  306,  296,  300,  300,  319,  307,
    ],
    // bishop EG
    [
         332,  344,  340,  349,  345,  340,  336,  330,
         325,  338,  342,  345,  337,  336,  340,  324,
         343,  341,  349,  342,  346,  349,  340,  340,
         343,  356,  350,  359,  358,  355,  356,  341,
         340,  354,  360,  357,  356,  356,  351,  332,
         338,  348,  354,  352,  358,  351,  337,  331,
         333,  332,  332,  344,  345,  335,  331,  315,
         319,  329,  316,  337,  332,  331,  317,  308,
    ],
    // rook MG
    [
         425,  414,  422,  428,  440,  450,  456,  463,
         398,  399,  419,  436,  423,  460,  450,  461,
         381,  404,  405,  408,  437,  449,  477,  440,
         374,  384,  388,  399,  403,  414,  415,  413,
         367,  369,  369,  382,  382,  382,  398,  392,
         366,  367,  369,  374,  381,  388,  420,  399,
         367,  370,  376,  377,  384,  390,  403,  377,
         387,  381,  379,  384,  390,  391,  393,  389,
    ],
    // rook EG
    [
         596,  605,  608,  602,  599,  598,  597,  592,
         607,  613,  611,  601,  605,  594,  594,  584,
         606,  602,  602,  598,  588,  584,  578,  581,
         604,  601,  603,  597,  590,  586,  587,  579,
         596,  597,  597,  593,  590,  589,  582,  577,
         588,  589,  586,  586,  581,  577,  560,  562,
         584,  585,  585,  582,  576,  572,  564,  572,
         588,  585,  592,  586,  578,  579,  576,  576,
    ],
    // queen MG
    [
         802,  820,  841,  871,  866,  882,  914,  857,
         831,  816,  826,  819,  825,  866,  857,  889,
         835,  832,  839,  849,  868,  901,  903,  886,
         824,  830,  835,  837,  841,  850,  847,  857,
         829,  826,  830,  835,  837,  836,  848,  847,
         825,  836,  832,  832,  833,  841,  852,  845,
         824,  834,  842,  839,  839,  844,  851,  852,
         824,  816,  820,  835,  828,  812,  818,  820,
    ],
    // queen EG
    [
        1140, 1146, 1162, 1152, 1158, 1142, 1096, 1127,
        1111, 1148, 1175, 1192, 1206, 1166, 1144, 1118,
        1111, 1134, 1161, 1168, 1175, 1157, 1123, 1121,
        1119, 1142, 1155, 1175, 1186, 1172, 1163, 1136,
        1112, 1143, 1147, 1167, 1159, 1154, 1137, 1127,
        1104, 1117, 1138, 1133, 1135, 1125, 1107, 1098,
        1100, 1100, 1094, 1103, 1105, 1077, 1056, 1038,
        1090, 1093, 1098, 1085, 1083, 1078, 1067, 1069,
    ],
    // king MG
    [
          94,   43,   74,  -13,   30,   26,   66,  184,
         -21,   17,   -3,   67,   45,   23,   47,   40,
         -30,   50,    3,  -13,   27,   72,   42,   15,
         -14,  -19,  -33,  -56,  -52,  -29,  -45,  -59,
         -39,  -30,  -46,  -65,  -64,  -51,  -64,  -84,
         -28,   -2,  -36,  -40,  -36,  -42,  -23,  -35,
          26,   16,    6,  -22,  -22,  -10,   27,   33,
         -12,   41,   34,  -36,   17,  -35,   28,   36,
    ],
    // king EG
    [
        -121,  -53,  -46,  -14,  -29,  -20,  -25, -119,
         -33,    5,   12,   -0,   11,   25,   17,  -23,
         -19,    9,   25,   35,   30,   28,   27,   -7,
         -25,   15,   32,   43,   43,   35,   29,   -3,
         -27,    6,   26,   42,   39,   26,   15,   -4,
         -24,   -4,   14,   23,   22,   12,   -2,  -13,
         -34,  -17,   -7,    3,    2,   -6,  -23,  -30,
         -55,  -55,  -43,  -20,  -45,  -25,  -49,  -68,
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
          -3,  -12,  -17,  -11,  -21,  -13,  -22,  -12,
         -10,  -21,  -22,  -19,  -22,  -15,  -17,    5,
         -16,   -9,  -17,  -17,   -4,   -3,    5,    1,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         -21,  -19,  -17,  -17,  -10,  -13,  -14,  -16,
         104,  102,   89,   59,   65,   84,   90,  109,
          55,   52,   42,   35,   38,   42,   57,   60,
          30,   27,   24,   18,   22,   25,   37,   35,
           1,    6,   11,    2,    8,    8,   20,    5,
           3,    5,   14,    8,   -1,    2,    5,    4,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 30;
const ROOK_OPEN_FILE_EG: i32 = 11;
const ROOK_CLOSED_FILE_MG: i32 = -14;
const ROOK_CLOSED_FILE_EG: i32 = -4;
const ROOK_SEMIOPEN_FILE_MG: i32 = 7;
const ROOK_SEMIOPEN_FILE_EG: i32 = 12;
const KING_OPEN_FILE_MG: i32 = -47;
const KING_OPEN_FILE_EG: i32 = -10;
const KING_CLOSED_FILE_MG: i32 = 13;
const KING_CLOSED_FILE_EG: i32 = -14;
const KING_SEMIOPEN_FILE_MG: i32 = -8;
const KING_SEMIOPEN_FILE_EG: i32 = 7;
const PAWN_SHIELDS: [[i32; NUM_PHASES]; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    [-137, -88],  /*0b0000*/
    [-129, -91],  /*0b0001*/
    [-115, -95],  /*0b0010*/
    [-99, -87],   /*0b0011*/
    [-109, -96],  /*0b0100*/
    [-120, -112], /*0b0101*/
    [-100, -95],  /*0b0110*/
    [-83, -121],  /*0b0111*/
    [-119, -91],  /*0b1000*/
    [-134, -124], /*0b1001*/
    [-108, -85],  /*0b1010*/
    [-91, -111],  /*0b1011*/
    [-108, -101], /*0b1100*/
    [-122, -132], /*0b1101*/
    [-82, -89],   /*0b1110*/
    [-100, -100], /*0b1111*/
    [-142, -91],  /*0b10000*/
    [-100, -86],  /*0b10001*/
    [-128, -128], /*0b10010*/
    [-95, -113],  /*0b10011*/
    [-107, -93],  /*0b10100*/
    [-77, -95],   /*0b10101*/
    [-113, -124], /*0b10110*/
    [-100, -100], /*0b10111*/
    [-125, -67],  /*0b11000*/
    [-99, -96],   /*0b11001*/
    [-90, -88],   /*0b11010*/
    [-100, -100], /*0b11011*/
    [-91, -90],   /*0b11100*/
    [-100, -100], /*0b11101*/
    [-100, -100], /*0b11110*/
    [-100, -100], /*0b11111*/
    [-107, -92],  /*0b100000*/
    [-110, -94],  /*0b100001*/
    [-90, -95],   /*0b100010*/
    [-74, -95],   /*0b100011*/
    [-128, -134], /*0b100100*/
    [-123, -151], /*0b100101*/
    [-111, -110], /*0b100110*/
    [-100, -100], /*0b100111*/
    [-119, -109], /*0b101000*/
    [-137, -121], /*0b101001*/
    [-86, -98],   /*0b101010*/
    [-100, -100], /*0b101011*/
    [-140, -138], /*0b101100*/
    [-100, -100], /*0b101101*/
    [-100, -100], /*0b101110*/
    [-100, -100], /*0b101111*/
    [-119, -82],  /*0b110000*/
    [-83, -87],   /*0b110001*/
    [-97, -124],  /*0b110010*/
    [-100, -100], /*0b110011*/
    [-107, -101], /*0b110100*/
    [-100, -100], /*0b110101*/
    [-100, -100], /*0b110110*/
    [-100, -100], /*0b110111*/
    [-126, -87],  /*0b111000*/
    [-100, -100], /*0b111001*/
    [-100, -100], /*0b111010*/
    [-100, -100], /*0b111011*/
    [-100, -100], /*0b111100*/
    [-100, -100], /*0b111101*/
    [-100, -100], /*0b111110*/
    [-94, -126],  /*0b111111*/
    [-157, -78],  /*0b00*/
    [-103, -105], /*0b01*/
    [-81, -96],   /*0b10*/
    [-56, -131],  /*0b11*/
    [-103, -100], /*0b100*/
    [-130, -146], /*0b101*/
    [-41, -128],  /*0b110*/
    [-100, -100], /*0b111*/
    [-70, -96],   /*0b1000*/
    [-90, -118],  /*0b1001*/
    [-52, -187],  /*0b1010*/
    [-100, -100], /*0b1011*/
    [-83, -105],  /*0b1100*/
    [-100, -100], /*0b1101*/
    [-100, -100], /*0b1110*/
    [-89, -116],  /*0b1111*/
    [-148, -82],  /*0b00*/
    [-120, -104], /*0b01*/
    [-117, -102], /*0b10*/
    [-98, -130],  /*0b11*/
    [-127, -97],  /*0b100*/
    [-108, -153], /*0b101*/
    [-117, -110], /*0b110*/
    [-100, -100], /*0b111*/
    [-140, -93],  /*0b1000*/
    [-93, -104],  /*0b1001*/
    [-109, -168], /*0b1010*/
    [-100, -100], /*0b1011*/
    [-130, -106], /*0b1100*/
    [-100, -100], /*0b1101*/
    [-100, -100], /*0b1110*/
    [-112, -161], /*0b1111*/
];
const VIRTUAL_QUEEN_MOBILITY: [[i32; NUM_PHASES]; NUM_VIRTUAL_QUEEN_MOBILITY_FEATURES] = [
    [-87, -88],
    [-75, -95],
    [-79, -104],
    [-81, -108],
    [-84, -108],
    [-84, -111],
    [-88, -110],
    [-94, -110],
    [-98, -107],
    [-105, -104],
    [-113, -100],
    [-122, -97],
    [-130, -95],
    [-141, -93],
    [-153, -90],
    [-176, -92],
];

// TODO: Differentiate between rooks and kings in front of / behind pawns.

const PIECE_PHASE: [i32; 6] = [0, 1, 1, 2, 4, 0];
pub const NUM_VIRTUAL_QUEEN_MOBILITY_FEATURES: usize = 16;

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
