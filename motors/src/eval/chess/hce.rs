use strum::IntoEnumIterator;

use crate::eval::chess::{
    pawn_shield_idx, FileOpenness, NUM_PAWN_SHIELD_CONFIGURATIONS, NUM_PHASES,
};
use gears::games::chess::pieces::UncoloredChessPiece::{Pawn, Rook};
use gears::games::chess::pieces::{UncoloredChessPiece, NUM_CHESS_PIECES};
use gears::games::chess::squares::NUM_SQUARES;
use gears::games::chess::Chessboard;
use gears::games::Color::{Black, White};
use gears::games::{Board, Color, DimT};
use gears::general::bitboards::chess::{ChessBitboard, A_FILE};
use gears::general::bitboards::Bitboard;
use gears::general::bitboards::RawBitboard;
use gears::general::common::StaticallyNamedEntity;
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
         155,  161,  152,  180,  162,  146,   60,   44,
          74,   80,   98,  106,  113,  150,  133,  103,
          57,   74,   71,   74,   98,   93,   89,   77,
          45,   64,   62,   76,   77,   78,   78,   58,
          37,   51,   51,   50,   61,   57,   72,   50,
          45,   59,   55,   38,   56,   73,   89,   51,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         269,  265,  260,  215,  215,  226,  273,  281,
         114,  113,   98,  100,   93,   84,  111,  105,
         103,   97,   88,   80,   80,   78,   91,   84,
          93,   92,   82,   79,   80,   77,   84,   77,
          88,   85,   81,   91,   87,   83,   78,   75,
          96,   95,   92,   97,  103,   94,   84,   83,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight MG
    [
         138,  188,  229,  258,  303,  210,  220,  177,
         269,  290,  323,  340,  318,  376,  291,  304,
         289,  326,  349,  357,  391,  398,  351,  314,
         289,  307,  330,  354,  335,  358,  316,  324,
         278,  293,  312,  311,  322,  314,  317,  289,
         257,  281,  295,  304,  312,  299,  303,  272,
         247,  259,  278,  288,  290,  296,  284,  280,
         214,  254,  246,  261,  267,  281,  258,  236,
    ],
    // knight EG
    [
         257,  308,  330,  318,  316,  312,  306,  238,
         305,  321,  325,  326,  324,  306,  317,  291,
         313,  325,  338,  340,  327,  322,  317,  307,
         322,  338,  349,  350,  352,  347,  339,  314,
         322,  334,  351,  354,  356,  344,  330,  313,
         307,  325,  334,  347,  342,  329,  316,  308,
         303,  320,  326,  328,  326,  320,  310,  308,
         289,  284,  310,  313,  313,  302,  290,  286,
    ],
    // bishop MG
    [
         293,  273,  274,  251,  261,  257,  311,  276,
         313,  341,  336,  317,  344,  346,  336,  313,
         327,  352,  354,  372,  371,  398,  373,  359,
         320,  340,  357,  372,  367,  363,  340,  326,
         319,  330,  339,  359,  355,  339,  333,  327,
         320,  336,  335,  339,  340,  335,  336,  335,
         326,  330,  342,  320,  329,  339,  349,  331,
         308,  328,  310,  304,  308,  308,  324,  317,
    ],
    // bishop EG
    [
         335,  347,  343,  353,  349,  345,  339,  334,
         328,  339,  344,  348,  340,  339,  342,  328,
         345,  343,  351,  343,  347,  349,  341,  342,
         344,  356,  352,  361,  359,  356,  357,  344,
         342,  355,  362,  360,  359,  358,  353,  334,
         340,  348,  355,  355,  360,  353,  339,  332,
         335,  334,  335,  347,  349,  338,  335,  318,
         320,  332,  320,  339,  334,  335,  321,  309,
    ],
    // rook MG
    [
         422,  411,  418,  423,  434,  444,  453,  463,
         399,  401,  419,  436,  424,  457,  453,  464,
         384,  406,  407,  408,  438,  452,  482,  444,
         376,  387,  389,  399,  404,  416,  422,  420,
         370,  371,  372,  384,  387,  384,  405,  396,
         368,  369,  372,  376,  384,  390,  423,  402,
         372,  376,  381,  382,  390,  403,  418,  384,
         393,  387,  386,  393,  401,  403,  405,  399,
    ],
    // rook EG
    [
         600,  610,  612,  606,  604,  603,  601,  595,
         610,  615,  614,  605,  607,  596,  594,  585,
         608,  605,  605,  600,  589,  584,  579,  583,
         607,  603,  606,  599,  591,  588,  587,  581,
         599,  599,  600,  595,  592,  591,  583,  579,
         592,  592,  590,  590,  584,  580,  562,  565,
         586,  588,  588,  586,  579,  573,  564,  572,
         589,  588,  596,  589,  582,  585,  577,  575,
    ],
    // queen MG
    [
         800,  819,  840,  869,  866,  882,  917,  852,
         833,  819,  827,  819,  827,  866,  857,  889,
         839,  837,  845,  853,  870,  909,  905,  889,
         827,  836,  842,  843,  846,  855,  853,  862,
         833,  832,  837,  843,  843,  844,  854,  852,
         831,  843,  840,  839,  844,  851,  863,  853,
         828,  839,  847,  845,  845,  854,  861,  859,
         828,  821,  825,  840,  833,  822,  826,  828,
    ],
    // queen EG
    [
        1148, 1155, 1170, 1160, 1165, 1149, 1103, 1140,
        1117, 1153, 1181, 1200, 1212, 1173, 1153, 1128,
        1116, 1139, 1166, 1175, 1184, 1163, 1132, 1130,
        1125, 1146, 1160, 1181, 1192, 1179, 1170, 1143,
        1119, 1147, 1152, 1171, 1165, 1159, 1142, 1133,
        1106, 1118, 1140, 1138, 1138, 1129, 1109, 1099,
        1106, 1106, 1101, 1112, 1112, 1087, 1065, 1044,
        1093, 1100, 1109, 1098, 1099, 1091, 1077, 1072,
    ],
    // king MG
    [
          94,   41,   65,  -22,   14,   23,   70,  180,
         -19,    5,  -16,   54,   28,    9,   44,   33,
         -34,   40,  -16,  -28,   10,   59,   34,   -1,
         -17,  -30,  -51,  -74,  -71,  -46,  -49,  -75,
         -42,  -45,  -67,  -90,  -90,  -70,  -76, -101,
         -25,   -5,  -54,  -64,  -59,  -62,  -27,  -52,
          34,   10,   -4,  -32,  -32,  -18,   24,   19,
          -7,   42,   36,  -38,   19,  -38,   32,   24,
    ],
    // king EG
    [
        -116,  -44,  -36,   -3,  -17,  -10,  -16, -122,
         -29,   15,   23,   10,   23,   36,   27,  -25,
         -13,   18,   35,   45,   41,   38,   37,   -7,
         -19,   24,   42,   53,   54,   46,   37,   -4,
         -22,   15,   36,   53,   50,   36,   24,   -5,
         -20,    3,   24,   35,   33,   23,    4,  -14,
         -30,   -7,    2,   13,   13,    3,  -15,  -31,
         -50,  -48,  -36,  -12,  -38,  -16,  -43,  -69,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns MG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          16,    7,   13,   14,    4,    4,  -16,   -2,
          30,   40,   31,   18,   19,    2,  -26,  -52,
          15,   13,   21,   16,   -2,    9,  -13,  -14,
           2,   -8,  -15,  -10,  -18,  -10,  -21,  -12,
          -4,  -14,  -19,  -13,  -15,   -8,  -13,    7,
         -12,   -6,  -12,  -12,    2,    3,    7,    3,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         -20,  -18,  -17,  -17,  -10,  -12,  -12,  -15,
         112,  108,   90,   59,   62,   86,   94,  116,
          65,   60,   46,   37,   38,   47,   64,   67,
          38,   37,   30,   22,   26,   32,   45,   43,
          11,   19,   18,    6,   13,   15,   31,   13,
          10,   13,   17,   10,    0,    6,   13,   10,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = -56;
const ROOK_OPEN_FILE_EG: i32 = -10;
const ROOK_CLOSED_FILE_MG: i32 = 16;
const ROOK_CLOSED_FILE_EG: i32 = -14;
const ROOK_SEMIOPEN_FILE_MG: i32 = -13;
const ROOK_SEMIOPEN_FILE_EG: i32 = 8;
const KING_OPEN_FILE_MG: i32 = 29;
const KING_OPEN_FILE_EG: i32 = 11;
const KING_CLOSED_FILE_MG: i32 = -14;
const KING_CLOSED_FILE_EG: i32 = -4;
const KING_SEMIOPEN_FILE_MG: i32 = 8;
const KING_SEMIOPEN_FILE_EG: i32 = 12;
const PAWN_SHIELDS: [[i32; NUM_PHASES]; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    [-145, -93],  /*0b0000*/
    [-126, -94],  /*0b0001*/
    [-111, -100], /*0b0010*/
    [-89, -83],   /*0b0011*/
    [-106, -100], /*0b0100*/
    [-110, -110], /*0b0101*/
    [-90, -91],   /*0b0110*/
    [-69, -110],  /*0b0111*/
    [-131, -96],  /*0b1000*/
    [-126, -118], /*0b1001*/
    [-113, -97],  /*0b1010*/
    [-82, -111],  /*0b1011*/
    [-109, -103], /*0b1100*/
    [-114, -126], /*0b1101*/
    [-82, -94],   /*0b1110*/
    [-100, -100], /*0b1111*/
    [-141, -92],  /*0b10000*/
    [-104, -98],  /*0b10001*/
    [-116, -119], /*0b10010*/
    [-91, -116],  /*0b10011*/
    [-109, -104], /*0b10100*/
    [-69, -98],   /*0b10101*/
    [-104, -122], /*0b10110*/
    [-100, -100], /*0b10111*/
    [-118, -63],  /*0b11000*/
    [-91, -96],   /*0b11001*/
    [-81, -85],   /*0b11010*/
    [-100, -100], /*0b11011*/
    [-85, -95],   /*0b11100*/
    [-100, -100], /*0b11101*/
    [-100, -100], /*0b11110*/
    [-100, -100], /*0b11111*/
    [-116, -99],  /*0b100000*/
    [-109, -96],  /*0b100001*/
    [-94, -106],  /*0b100010*/
    [-73, -100],  /*0b100011*/
    [-124, -129], /*0b100100*/
    [-115, -142], /*0b100101*/
    [-103, -109], /*0b100110*/
    [-100, -100], /*0b100111*/
    [-122, -110], /*0b101000*/
    [-127, -111], /*0b101001*/
    [-98, -114],  /*0b101010*/
    [-100, -100], /*0b101011*/
    [-129, -130], /*0b101100*/
    [-100, -100], /*0b101101*/
    [-100, -100], /*0b101110*/
    [-100, -100], /*0b101111*/
    [-108, -77],  /*0b110000*/
    [-74, -89],   /*0b110001*/
    [-89, -120],  /*0b110010*/
    [-100, -100], /*0b110011*/
    [-97, -103],  /*0b110100*/
    [-100, -100], /*0b110101*/
    [-100, -100], /*0b110110*/
    [-100, -100], /*0b110111*/
    [-105, -73],  /*0b111000*/
    [-100, -100], /*0b111001*/
    [-100, -100], /*0b111010*/
    [-100, -100], /*0b111011*/
    [-100, -100], /*0b111100*/
    [-100, -100], /*0b111101*/
    [-100, -100], /*0b111110*/
    [-80, -126],  /*0b111111*/
    [-165, -80],  /*0b00*/
    [-101, -109], /*0b01*/
    [-75, -96],   /*0b10*/
    [-48, -121],  /*0b11*/
    [-96, -101],  /*0b100*/
    [-130, -132], /*0b101*/
    [-40, -139],  /*0b110*/
    [-100, -100], /*0b111*/
    [-76, -100],  /*0b1000*/
    [-91, -127],  /*0b1001*/
    [-48, -171],  /*0b1010*/
    [-100, -100], /*0b1011*/
    [-69, -103],  /*0b1100*/
    [-100, -100], /*0b1101*/
    [-100, -100], /*0b1110*/
    [-72, -114],  /*0b1111*/
    [-136, -78],  /*0b00*/
    [-97, -97],   /*0b01*/
    [-95, -103],  /*0b10*/
    [-67, -115],  /*0b11*/
    [-120, -92],  /*0b100*/
    [-86, -135],  /*0b101*/
    [-99, -112],  /*0b110*/
    [-100, -100], /*0b111*/
    [-119, -87],  /*0b1000*/
    [-76, -107],  /*0b1001*/
    [-82, -154],  /*0b1010*/
    [-100, -100], /*0b1011*/
    [-104, -92],  /*0b1100*/
    [-100, -100], /*0b1101*/
    [-100, -100], /*0b1110*/
    [-81, -147],  /*0b1111*/
];
const PAWN_PROTECTION: [[i32; NUM_PHASES]; NUM_CHESS_PIECES] =
    [[16, 17], [6, 12], [1, 5], [6, 8], [-7, 12], [-31, 15]];
const PAWN_ATTACKS: [[i32; NUM_PHASES]; NUM_CHESS_PIECES] =
    [[21, -3], [33, 13], [48, 32], [50, -2], [40, -31], [0, 0]];

// TODO: Differentiate between rooks and kings in front of / behind pawns?

const PIECE_PHASE: [i32; 6] = [0, 1, 1, 2, 4, 0];

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

impl StaticallyNamedEntity for HandCraftedEval {
    fn static_short_name() -> &'static str
    where
        Self: Sized,
    {
        "hce"
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        "Hand Crafted Chess Eval".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "A classical evaluation for chess, based on piece square tables".to_string()
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
            let rooks = pos.colored_piece_bb(color, Rook);
            for rook in rooks.ones() {
                match file_openness(rook.file(), our_pawns, their_pawns) {
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
            mg += Score(PAWN_SHIELDS[pawn_shield_idx(our_pawns, king_square, color)][Mg as usize]);
            eg += Score(PAWN_SHIELDS[pawn_shield_idx(our_pawns, king_square, color)][Eg as usize]);

            for piece in UncoloredChessPiece::pieces() {
                let bb = pos.colored_piece_bb(color, piece);
                for unflipped_square in bb.ones() {
                    let square = unflipped_square.flip_if(color == White);
                    let idx = square.bb_idx();
                    let mg_table = piece as usize * 2;
                    let eg_table = mg_table + 1;
                    mg += Score(PSQTS[mg_table][idx]);
                    eg += Score(PSQTS[eg_table][idx]);
                    phase += PIECE_PHASE[piece as usize];

                    // Passed pawns.
                    if piece == Pawn {
                        let in_front = (A_FILE
                            << (unflipped_square.flip_if(color == Black).bb_idx() + 8))
                            .flip_if(color == Black);
                        let blocking = in_front | in_front.west() | in_front.east();
                        if (in_front & our_pawns).is_zero() && (blocking & their_pawns).is_zero() {
                            mg += Score(PASSED_PAWNS[0][idx]);
                            eg += Score(PASSED_PAWNS[1][idx]);
                        }
                    }
                }
                let pawn_attacks = our_pawns.pawn_attacks(color);
                let protected_by_pawns = pawn_attacks & bb;
                mg += Score(PAWN_PROTECTION[piece as usize][0])
                    * protected_by_pawns.num_ones() as i32;
                eg += Score(PAWN_PROTECTION[piece as usize][1])
                    * protected_by_pawns.num_ones() as i32;
                let attacked_by_pawns = pawn_attacks & pos.colored_piece_bb(color.other(), piece);
                mg += Score(PAWN_ATTACKS[piece as usize][0]) * attacked_by_pawns.num_ones() as i32;
                eg += Score(PAWN_ATTACKS[piece as usize][1]) * attacked_by_pawns.num_ones() as i32;
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
