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
          74,   81,   98,  106,  113,  150,  134,  103,
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
         114,  113,   98,  100,   93,   84,  110,  105,
         103,   97,   88,   80,   80,   78,   91,   84,
          93,   92,   83,   79,   80,   77,   84,   77,
          88,   85,   81,   91,   87,   83,   78,   75,
          96,   95,   91,   97,  103,   94,   84,   83,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight MG
    [
         138,  188,  229,  258,  303,  210,  220,  177,
         269,  290,  323,  340,  318,  376,  291,  304,
         289,  326,  349,  357,  391,  397,  351,  314,
         289,  307,  330,  354,  335,  358,  316,  324,
         278,  293,  312,  311,  322,  314,  317,  289,
         257,  281,  295,  304,  312,  299,  303,  272,
         247,  259,  278,  288,  290,  295,  284,  280,
         214,  254,  245,  261,  267,  281,  258,  236,
    ],
    // knight EG
    [
         257,  308,  330,  318,  316,  312,  306,  238,
         305,  321,  325,  326,  324,  306,  317,  291,
         313,  325,  338,  340,  327,  322,  317,  307,
         322,  338,  349,  350,  352,  347,  339,  314,
         322,  334,  352,  354,  356,  344,  330,  313,
         307,  325,  334,  347,  342,  329,  316,  308,
         303,  320,  326,  328,  326,  320,  310,  308,
         289,  284,  310,  313,  313,  302,  290,  286,
    ],
    // bishop MG
    [
         293,  272,  274,  251,  261,  257,  310,  276,
         313,  341,  336,  317,  343,  345,  336,  313,
         327,  352,  354,  372,  371,  398,  373,  359,
         320,  340,  357,  372,  367,  363,  340,  326,
         319,  330,  339,  359,  355,  339,  332,  327,
         320,  336,  335,  339,  340,  335,  336,  335,
         326,  330,  342,  320,  329,  339,  349,  331,
         307,  328,  310,  304,  308,  308,  324,  317,
    ],
    // bishop EG
    [
         335,  347,  343,  353,  349,  345,  339,  334,
         328,  339,  344,  348,  340,  339,  342,  328,
         345,  343,  351,  343,  347,  349,  341,  342,
         344,  357,  352,  361,  359,  356,  357,  344,
         342,  355,  362,  360,  359,  358,  353,  334,
         340,  348,  356,  355,  360,  353,  339,  332,
         335,  334,  335,  347,  349,  338,  335,  318,
         320,  333,  320,  339,  335,  335,  322,  309,
    ],
    // rook MG
    [
         422,  411,  418,  423,  434,  444,  453,  463,
         399,  401,  419,  436,  424,  457,  453,  464,
         384,  405,  407,  408,  438,  452,  481,  444,
         375,  387,  389,  399,  404,  416,  422,  420,
         370,  371,  372,  384,  387,  384,  405,  396,
         368,  369,  372,  376,  384,  389,  423,  402,
         371,  376,  381,  381,  390,  403,  417,  384,
         392,  386,  385,  393,  401,  403,  405,  399,
    ],
    // rook EG
    [
         600,  610,  612,  606,  604,  603,  601,  595,
         610,  615,  614,  605,  607,  596,  594,  585,
         608,  605,  605,  600,  590,  584,  579,  583,
         607,  603,  606,  599,  591,  588,  587,  581,
         599,  599,  600,  595,  592,  591,  583,  579,
         592,  592,  590,  590,  584,  580,  562,  565,
         586,  588,  588,  586,  579,  573,  564,  572,
         589,  588,  596,  590,  582,  585,  577,  575,
    ],
    // queen MG
    [
         800,  818,  839,  869,  865,  882,  916,  852,
         832,  819,  827,  819,  826,  865,  856,  889,
         839,  836,  844,  852,  870,  908,  905,  889,
         827,  835,  841,  842,  846,  855,  852,  861,
         832,  832,  836,  842,  843,  843,  854,  852,
         830,  842,  839,  839,  844,  850,  862,  853,
         828,  838,  847,  844,  844,  854,  861,  858,
         828,  821,  824,  839,  832,  822,  826,  828,
    ],
    // queen EG
    [
        1149, 1155, 1171, 1161, 1165, 1150, 1104, 1141,
        1117, 1153, 1181, 1200, 1212, 1173, 1153, 1128,
        1117, 1139, 1167, 1176, 1184, 1163, 1133, 1130,
        1126, 1146, 1160, 1181, 1193, 1179, 1170, 1144,
        1119, 1147, 1152, 1171, 1166, 1159, 1142, 1133,
        1107, 1119, 1141, 1138, 1138, 1129, 1109, 1100,
        1106, 1106, 1102, 1112, 1113, 1087, 1065, 1044,
        1094, 1100, 1110, 1098, 1100, 1091, 1078, 1072,
    ],
    // king MG
    [
          89,   41,   65,  -21,   15,   24,   71,  192,
         -24,    6,  -15,   54,   28,   10,   44,   45,
         -40,   41,  -16,  -28,   11,   60,   35,   10,
         -22,  -29,  -50,  -74,  -71,  -46,  -49,  -63,
         -47,  -45,  -67,  -89,  -90,  -70,  -75,  -89,
         -31,   -4,  -54,  -64,  -58,  -61,  -27,  -40,
          28,   11,   -3,  -31,  -31,  -17,   25,   31,
         -12,   43,   36,  -38,   19,  -38,   32,   36,
    ],
    // king EG
    [
        -119,  -52,  -44,  -11,  -26,  -18,  -24, -123,
         -31,    6,   15,    2,   15,   28,   19,  -26,
         -16,   10,   27,   37,   32,   30,   28,   -8,
         -22,   15,   34,   45,   45,   37,   29,   -4,
         -24,    7,   28,   44,   42,   27,   16,   -6,
         -23,   -5,   15,   27,   25,   15,   -4,  -15,
         -33,  -16,   -6,    5,    5,   -5,  -23,  -31,
         -53,  -56,  -45,  -20,  -46,  -25,  -51,  -70,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns MG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          16,    7,   13,   14,    4,    4,  -16,   -2,
          30,   40,   31,   18,   19,    2,  -28,  -52,
          15,   13,   20,   16,   -2,    9,  -13,  -14,
           2,   -8,  -15,  -10,  -18,  -10,  -21,  -12,
          -4,  -14,  -19,  -13,  -15,   -8,  -13,    7,
         -12,   -6,  -13,  -12,    2,    3,    7,    3,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         -20,  -18,  -17,  -17,  -10,  -12,  -12,  -15,
         112,  109,   90,   59,   62,   86,   94,  117,
          65,   60,   46,   37,   38,   47,   64,   67,
          38,   37,   30,   22,   26,   32,   45,   43,
          11,   19,   18,    6,   13,   15,   31,   13,
          10,   13,   17,   10,    0,    6,   13,   10,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 29;
const ROOK_OPEN_FILE_EG: i32 = 11;
const ROOK_CLOSED_FILE_MG: i32 = -14;
const ROOK_CLOSED_FILE_EG: i32 = -4;
const ROOK_SEMIOPEN_FILE_MG: i32 = 8;
const ROOK_SEMIOPEN_FILE_EG: i32 = 12;
const KING_OPEN_FILE_MG: i32 = -56;
const KING_OPEN_FILE_EG: i32 = -10;
const KING_CLOSED_FILE_MG: i32 = 16;
const KING_CLOSED_FILE_EG: i32 = -14;
const KING_SEMIOPEN_FILE_MG: i32 = -13;
const KING_SEMIOPEN_FILE_EG: i32 = 8;
const PAWN_SHIELDS: [[i32; NUM_PHASES]; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    [-151, -91],  /*0b0000*/
    [-132, -91],  /*0b0001*/
    [-117, -98],  /*0b0010*/
    [-95, -80],   /*0b0011*/
    [-112, -97],  /*0b0100*/
    [-117, -107], /*0b0101*/
    [-96, -89],   /*0b0110*/
    [-75, -108],  /*0b0111*/
    [-137, -94],  /*0b1000*/
    [-132, -116], /*0b1001*/
    [-119, -95],  /*0b1010*/
    [-88, -109],  /*0b1011*/
    [-115, -101], /*0b1100*/
    [-120, -123], /*0b1101*/
    [-88, -92],   /*0b1110*/
    [-100, -100], /*0b1111*/
    [-147, -90],  /*0b10000*/
    [-110, -96],  /*0b10001*/
    [-122, -117], /*0b10010*/
    [-97, -113],  /*0b10011*/
    [-115, -102], /*0b10100*/
    [-75, -95],   /*0b10101*/
    [-110, -120], /*0b10110*/
    [-100, -100], /*0b10111*/
    [-124, -61],  /*0b11000*/
    [-98, -94],   /*0b11001*/
    [-87, -83],   /*0b11010*/
    [-100, -100], /*0b11011*/
    [-91, -93],   /*0b11100*/
    [-100, -100], /*0b11101*/
    [-100, -100], /*0b11110*/
    [-100, -100], /*0b11111*/
    [-122, -96],  /*0b100000*/
    [-115, -94],  /*0b100001*/
    [-100, -103], /*0b100010*/
    [-79, -98],   /*0b100011*/
    [-130, -126], /*0b100100*/
    [-122, -140], /*0b100101*/
    [-109, -107], /*0b100110*/
    [-100, -100], /*0b100111*/
    [-128, -108], /*0b101000*/
    [-133, -109], /*0b101001*/
    [-104, -112], /*0b101010*/
    [-100, -100], /*0b101011*/
    [-136, -128], /*0b101100*/
    [-100, -100], /*0b101101*/
    [-100, -100], /*0b101110*/
    [-100, -100], /*0b101111*/
    [-114, -75],  /*0b110000*/
    [-80, -87],   /*0b110001*/
    [-95, -118],  /*0b110010*/
    [-100, -100], /*0b110011*/
    [-103, -100], /*0b110100*/
    [-100, -100], /*0b110101*/
    [-100, -100], /*0b110110*/
    [-100, -100], /*0b110111*/
    [-111, -70],  /*0b111000*/
    [-100, -100], /*0b111001*/
    [-100, -100], /*0b111010*/
    [-100, -100], /*0b111011*/
    [-100, -100], /*0b111100*/
    [-100, -100], /*0b111101*/
    [-100, -100], /*0b111110*/
    [-86, -123],  /*0b111111*/
    [-165, -83],  /*0b00*/
    [-101, -113], /*0b01*/
    [-75, -99],   /*0b10*/
    [-48, -124],  /*0b11*/
    [-96, -104],  /*0b100*/
    [-130, -134], /*0b101*/
    [-40, -143],  /*0b110*/
    [-100, -100], /*0b111*/
    [-76, -103],  /*0b1000*/
    [-91, -130],  /*0b1001*/
    [-48, -174],  /*0b1010*/
    [-100, -100], /*0b1011*/
    [-69, -106],  /*0b1100*/
    [-100, -100], /*0b1101*/
    [-100, -100], /*0b1110*/
    [-72, -116],  /*0b1111*/
    [-153, -83],  /*0b00*/
    [-114, -102], /*0b01*/
    [-112, -108], /*0b10*/
    [-85, -120],  /*0b11*/
    [-137, -97],  /*0b100*/
    [-103, -141], /*0b101*/
    [-116, -118], /*0b110*/
    [-100, -100], /*0b111*/
    [-136, -92],  /*0b1000*/
    [-93, -113],  /*0b1001*/
    [-100, -160], /*0b1010*/
    [-100, -100], /*0b1011*/
    [-121, -97],  /*0b1100*/
    [-100, -100], /*0b1101*/
    [-100, -100], /*0b1110*/
    [-99, -153],  /*0b1111*/
];
const PAWN_PROTECTION: [[i32; NUM_PHASES]; NUM_CHESS_PIECES] =
    [[16, 17], [6, 12], [1, 5], [6, 8], [-7, 12], [-31, 15]];
const PAWN_ATTACKS: [[i32; NUM_PHASES]; NUM_CHESS_PIECES] =
    [[0, 0], [33, 13], [48, 32], [50, -2], [40, -31], [0, 0]];

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
