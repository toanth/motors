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
         156,  167,  155,  182,  166,  147,   67,   45,
          74,   82,  102,  108,  117,  154,  136,  104,
          58,   76,   73,   77,  101,   95,   91,   77,
          45,   65,   63,   77,   78,   78,   80,   58,
          38,   52,   52,   51,   62,   58,   73,   50,
          45,   60,   55,   39,   57,   74,   89,   51,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         269,  263,  260,  215,  214,  226,  271,  281,
         114,  113,   97,  100,   91,   83,  110,  105,
         103,   97,   87,   79,   79,   78,   90,   84,
          93,   92,   82,   79,   80,   77,   84,   77,
          87,   85,   81,   91,   87,   83,   78,   75,
          96,   95,   91,   96,  103,   94,   84,   83,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight MG
    [
         139,  187,  231,  261,  304,  214,  221,  179,
         268,  290,  322,  340,  317,  376,  291,  304,
         287,  325,  347,  356,  389,  395,  347,  311,
         289,  305,  329,  352,  334,  356,  313,  323,
         277,  292,  311,  310,  320,  313,  315,  288,
         257,  281,  294,  303,  311,  298,  302,  272,
         247,  259,  278,  288,  290,  295,  284,  280,
         214,  254,  246,  261,  268,  281,  258,  236,
    ],
    // knight EG
    [
         257,  308,  329,  317,  316,  311,  306,  237,
         305,  321,  325,  326,  324,  306,  317,  291,
         313,  325,  338,  341,  327,  322,  318,  308,
         322,  338,  349,  351,  352,  347,  339,  314,
         322,  333,  351,  354,  356,  343,  330,  313,
         307,  325,  334,  347,  342,  329,  315,  308,
         302,  320,  326,  328,  326,  320,  310,  308,
         289,  284,  310,  313,  313,  302,  289,  286,
    ],
    // bishop MG
    [
         294,  273,  276,  254,  263,  261,  312,  278,
         312,  340,  336,  317,  344,  345,  336,  312,
         325,  350,  351,  371,  367,  390,  366,  353,
         318,  337,  355,  371,  365,  358,  337,  324,
         317,  329,  337,  357,  352,  338,  331,  325,
         320,  335,  334,  338,  339,  334,  336,  334,
         326,  330,  342,  320,  328,  338,  349,  331,
         307,  328,  310,  304,  308,  308,  324,  316,
    ],
    // bishop EG
    [
         334,  346,  343,  352,  349,  344,  338,  334,
         328,  339,  344,  348,  339,  339,  342,  328,
         345,  343,  351,  343,  348,  350,  342,  343,
         344,  357,  352,  361,  359,  356,  356,  344,
         342,  355,  362,  360,  359,  357,  352,  333,
         340,  348,  356,  355,  360,  353,  339,  331,
         335,  334,  334,  347,  349,  338,  335,  318,
         320,  332,  320,  339,  335,  335,  321,  309,
    ],
    // rook MG
    [
         422,  411,  418,  423,  435,  445,  453,  463,
         399,  401,  419,  436,  424,  457,  453,  464,
         384,  405,  406,  408,  437,  450,  479,  443,
         375,  386,  389,  399,  403,  414,  420,  418,
         370,  371,  371,  383,  386,  383,  404,  396,
         368,  369,  371,  376,  384,  388,  422,  402,
         371,  376,  381,  382,  390,  402,  417,  384,
         393,  387,  386,  393,  401,  403,  405,  399,
    ],
    // rook EG
    [
         600,  609,  612,  606,  603,  602,  600,  595,
         610,  615,  614,  604,  607,  596,  594,  585,
         608,  604,  605,  600,  590,  585,  579,  583,
         607,  603,  606,  599,  591,  588,  587,  581,
         599,  599,  600,  595,  592,  591,  583,  579,
         591,  592,  590,  590,  584,  580,  562,  565,
         586,  588,  588,  585,  579,  573,  565,  572,
         589,  588,  596,  589,  582,  585,  577,  574,
    ],
    // queen MG
    [
         801,  819,  840,  870,  865,  882,  917,  853,
         833,  819,  827,  820,  827,  866,  856,  889,
         839,  837,  845,  853,  870,  908,  903,  889,
         827,  835,  841,  842,  846,  854,  851,  860,
         832,  832,  836,  842,  843,  843,  853,  852,
         831,  843,  840,  839,  844,  850,  862,  853,
         829,  839,  848,  845,  845,  854,  861,  859,
         829,  822,  825,  840,  833,  823,  827,  828,
    ],
    // queen EG
    [
        1147, 1154, 1170, 1159, 1165, 1149, 1103, 1140,
        1116, 1153, 1181, 1199, 1212, 1172, 1153, 1128,
        1116, 1138, 1166, 1175, 1183, 1163, 1133, 1130,
        1125, 1146, 1160, 1181, 1192, 1180, 1170, 1145,
        1119, 1146, 1152, 1171, 1165, 1159, 1143, 1133,
        1105, 1118, 1140, 1137, 1137, 1129, 1109, 1099,
        1105, 1105, 1101, 1111, 1112, 1086, 1064, 1043,
        1093, 1099, 1109, 1097, 1099, 1090, 1077, 1071,
    ],
    // king MG
    [
          89,   42,   65,  -22,   15,   23,   70,  191,
         -23,    6,  -16,   54,   28,    9,   43,   44,
         -39,   40,  -16,  -28,   10,   59,   35,    9,
         -22,  -29,  -50,  -74,  -71,  -46,  -49,  -64,
         -47,  -45,  -67,  -89,  -90,  -70,  -75,  -90,
         -30,   -5,  -54,  -64,  -59,  -61,  -27,  -40,
          29,   11,   -3,  -31,  -31,  -17,   25,   31,
         -12,   43,   36,  -38,   19,  -38,   33,   36,
    ],
    // king EG
    [
        -119,  -52,  -44,  -11,  -25,  -18,  -24, -122,
         -31,    6,   15,    2,   15,   28,   19,  -26,
         -16,   10,   27,   37,   33,   30,   28,   -8,
         -22,   15,   34,   45,   45,   38,   29,   -4,
         -24,    7,   28,   44,   42,   28,   16,   -6,
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
          17,   13,   16,   16,    8,    5,   -9,   -1,
          32,   40,   32,   19,   18,    4,  -27,  -49,
          15,   13,   21,   17,   -2,   10,  -12,  -13,
           3,   -8,  -14,  -10,  -18,   -9,  -21,  -11,
          -5,  -14,  -19,  -13,  -15,   -8,  -12,    8,
         -12,   -5,  -13,  -13,    2,    3,    7,    3,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         -20,  -20,  -17,  -17,  -11,  -12,  -14,  -15,
         111,  108,   90,   59,   63,   86,   95,  116,
          65,   60,   46,   37,   38,   46,   64,   67,
          38,   36,   30,   22,   26,   31,   45,   43,
          11,   19,   18,    6,   13,   15,   31,   13,
          10,   13,   17,   10,    1,    6,   13,   10,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 29;
const ROOK_OPEN_FILE_EG: i32 = 11;
const ROOK_CLOSED_FILE_MG: i32 = -14;
const ROOK_CLOSED_FILE_EG: i32 = -4;
const ROOK_SEMIOPEN_FILE_MG: i32 = 7;
const ROOK_SEMIOPEN_FILE_EG: i32 = 12;
const KING_OPEN_FILE_MG: i32 = -56;
const KING_OPEN_FILE_EG: i32 = -10;
const KING_CLOSED_FILE_MG: i32 = 16;
const KING_CLOSED_FILE_EG: i32 = -14;
const KING_SEMIOPEN_FILE_MG: i32 = -13;
const KING_SEMIOPEN_FILE_EG: i32 = 8;
const PAWN_SHIELDS: [[i32; NUM_PHASES]; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    [-150, -91],  /*0b0000*/
    [-131, -91],  /*0b0001*/
    [-117, -98],  /*0b0010*/
    [-95, -80],   /*0b0011*/
    [-112, -98],  /*0b0100*/
    [-116, -107], /*0b0101*/
    [-96, -89],   /*0b0110*/
    [-76, -108],  /*0b0111*/
    [-137, -94],  /*0b1000*/
    [-133, -116], /*0b1001*/
    [-119, -95],  /*0b1010*/
    [-88, -108],  /*0b1011*/
    [-115, -101], /*0b1100*/
    [-120, -124], /*0b1101*/
    [-88, -92],   /*0b1110*/
    [-100, -100], /*0b1111*/
    [-147, -90],  /*0b10000*/
    [-110, -96],  /*0b10001*/
    [-123, -117], /*0b10010*/
    [-97, -113],  /*0b10011*/
    [-115, -102], /*0b10100*/
    [-76, -95],   /*0b10101*/
    [-109, -120], /*0b10110*/
    [-100, -100], /*0b10111*/
    [-123, -61],  /*0b11000*/
    [-98, -94],   /*0b11001*/
    [-87, -83],   /*0b11010*/
    [-100, -100], /*0b11011*/
    [-91, -93],   /*0b11100*/
    [-100, -100], /*0b11101*/
    [-100, -100], /*0b11110*/
    [-100, -100], /*0b11111*/
    [-121, -96],  /*0b100000*/
    [-114, -94],  /*0b100001*/
    [-99, -103],  /*0b100010*/
    [-79, -98],   /*0b100011*/
    [-130, -126], /*0b100100*/
    [-121, -140], /*0b100101*/
    [-110, -106], /*0b100110*/
    [-100, -100], /*0b100111*/
    [-128, -108], /*0b101000*/
    [-134, -109], /*0b101001*/
    [-104, -112], /*0b101010*/
    [-100, -100], /*0b101011*/
    [-136, -128], /*0b101100*/
    [-100, -100], /*0b101101*/
    [-100, -100], /*0b101110*/
    [-100, -100], /*0b101111*/
    [-114, -75],  /*0b110000*/
    [-80, -86],   /*0b110001*/
    [-95, -118],  /*0b110010*/
    [-100, -100], /*0b110011*/
    [-103, -100], /*0b110100*/
    [-100, -100], /*0b110101*/
    [-100, -100], /*0b110110*/
    [-100, -100], /*0b110111*/
    [-112, -71],  /*0b111000*/
    [-100, -100], /*0b111001*/
    [-100, -100], /*0b111010*/
    [-100, -100], /*0b111011*/
    [-100, -100], /*0b111100*/
    [-100, -100], /*0b111101*/
    [-100, -100], /*0b111110*/
    [-87, -123],  /*0b111111*/
    [-165, -83],  /*0b00*/
    [-101, -113], /*0b01*/
    [-74, -99],   /*0b10*/
    [-48, -124],  /*0b11*/
    [-96, -104],  /*0b100*/
    [-131, -133], /*0b101*/
    [-40, -142],  /*0b110*/
    [-100, -100], /*0b111*/
    [-76, -103],  /*0b1000*/
    [-91, -130],  /*0b1001*/
    [-45, -176],  /*0b1010*/
    [-100, -100], /*0b1011*/
    [-69, -106],  /*0b1100*/
    [-100, -100], /*0b1101*/
    [-100, -100], /*0b1110*/
    [-72, -115],  /*0b1111*/
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
    [-120, -98],  /*0b1100*/
    [-100, -100], /*0b1101*/
    [-100, -100], /*0b1110*/
    [-99, -152],  /*0b1111*/
];
const PAWN_PROTECTION: [[i32; NUM_PHASES]; NUM_CHESS_PIECES] =
    [[16, 17], [5, 12], [1, 5], [5, 8], [-7, 12], [-31, 15]];

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
                let protected_by_pawns = our_pawns.pawn_attacks(color) & bb;
                mg += Score(PAWN_PROTECTION[piece as usize][0])
                    * protected_by_pawns.num_ones() as i32;
                eg += Score(PAWN_PROTECTION[piece as usize][1])
                    * protected_by_pawns.num_ones() as i32;
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
