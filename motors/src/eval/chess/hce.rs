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
         156,  166,  155,  181,  165,  148,   66,   45,
          75,   85,  103,  109,  116,  154,  137,  103,
          61,   83,   79,   82,  106,  100,   95,   77,
          50,   75,   70,   86,   86,   86,   87,   63,
          49,   70,   64,   65,   78,   71,   88,   59,
          48,   68,   58,   43,   62,   76,   94,   52,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         268,  264,  260,  215,  215,  226,  271,  280,
         122,  121,  101,  103,   94,   88,  117,  112,
         114,  110,   96,   86,   86,   86,  103,   93,
         104,  106,   93,   90,   89,   89,   97,   87,
         101,  104,   92,  100,   98,   94,   96,   87,
         105,  107,   98,  104,  108,  103,   97,   91,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight MG
    [
         139,  186,  231,  260,  302,  214,  219,  179,
         267,  289,  321,  337,  316,  375,  290,  303,
         287,  324,  345,  356,  388,  392,  346,  309,
         289,  306,  330,  352,  333,  357,  312,  322,
         278,  293,  310,  310,  320,  314,  315,  290,
         259,  283,  296,  303,  314,  300,  305,  274,
         246,  257,  276,  287,  288,  293,  281,  278,
         212,  253,  243,  259,  265,  279,  257,  234,
    ],
    // knight EG
    [
         257,  308,  328,  317,  315,  310,  304,  236,
         305,  321,  324,  325,  323,  306,  316,  290,
         313,  325,  339,  342,  329,  325,  319,  307,
         322,  341,  352,  353,  355,  349,  341,  314,
         324,  335,  353,  357,  358,  347,  333,  315,
         309,  327,  336,  347,  345,  332,  320,  310,
         302,  319,  324,  326,  324,  319,  308,  306,
         289,  282,  309,  312,  312,  300,  288,  285,
    ],
    // bishop MG
    [
         292,  272,  276,  253,  261,  261,  306,  278,
         310,  335,  333,  314,  342,  342,  333,  311,
         322,  347,  346,  367,  363,  384,  364,  350,
         316,  334,  353,  367,  361,  356,  334,  320,
         313,  326,  333,  354,  349,  334,  328,  322,
         319,  332,  331,  335,  336,  331,  333,  332,
         325,  326,  338,  317,  324,  336,  344,  328,
         303,  325,  307,  299,  304,  305,  321,  311,
    ],
    // bishop EG
    [
         332,  344,  340,  350,  346,  341,  336,  331,
         325,  338,  342,  345,  337,  336,  340,  324,
         343,  341,  349,  342,  346,  348,  340,  340,
         342,  356,  350,  359,  357,  355,  355,  341,
         340,  353,  359,  357,  356,  356,  351,  331,
         338,  347,  353,  352,  358,  351,  338,  330,
         333,  332,  332,  344,  346,  335,  332,  315,
         319,  329,  317,  337,  332,  332,  319,  307,
    ],
    // rook MG
    [
         419,  407,  413,  419,  431,  441,  451,  460,
         395,  398,  416,  432,  420,  454,  448,  460,
         380,  403,  403,  405,  434,  446,  475,  440,
         373,  383,  388,  396,  402,  412,  415,  414,
         367,  368,  368,  381,  382,  382,  399,  393,
         365,  366,  369,  373,  383,  388,  422,  401,
         367,  370,  376,  378,  385,  398,  410,  380,
         388,  382,  381,  389,  396,  400,  399,  395,
    ],
    // rook EG
    [
         597,  606,  609,  603,  600,  599,  597,  592,
         607,  612,  611,  601,  604,  594,  593,  583,
         605,  602,  602,  598,  587,  583,  578,  580,
         604,  601,  603,  597,  589,  586,  586,  579,
         596,  596,  597,  593,  590,  589,  582,  576,
         587,  588,  586,  586,  582,  577,  560,  561,
         583,  585,  585,  582,  576,  570,  563,  570,
         586,  584,  592,  586,  579,  582,  574,  571,
    ],
    // queen MG
    [
         797,  813,  834,  863,  860,  876,  910,  849,
         829,  813,  823,  816,  822,  861,  850,  884,
         834,  830,  837,  847,  863,  899,  898,  884,
         823,  828,  834,  835,  838,  848,  845,  855,
         828,  825,  829,  834,  836,  835,  847,  846,
         823,  834,  832,  831,  835,  841,  852,  845,
         823,  833,  842,  841,  840,  850,  856,  854,
         824,  816,  821,  836,  829,  818,  823,  823,
    ],
    // queen EG
    [
        1138, 1146, 1161, 1151, 1156, 1140, 1095, 1129,
        1108, 1145, 1172, 1189, 1203, 1163, 1145, 1120,
        1108, 1131, 1159, 1166, 1174, 1156, 1125, 1121,
        1116, 1139, 1153, 1173, 1185, 1172, 1162, 1136,
        1109, 1140, 1144, 1164, 1157, 1153, 1136, 1125,
        1101, 1113, 1133, 1130, 1133, 1125, 1107, 1096,
        1097, 1098, 1092, 1100, 1103, 1077, 1056, 1035,
        1086, 1093, 1099, 1088, 1090, 1083, 1069, 1064,
    ],
    // king MG
    [
          88,   36,   64,  -26,   13,   18,   65,  190,
         -26,    1,  -19,   48,   25,    4,   37,   40,
         -41,   33,  -21,  -35,    3,   51,   28,    8,
         -26,  -39,  -57,  -81,  -79,  -56,  -61,  -66,
         -48,  -51,  -74,  -95,  -96,  -79,  -82,  -91,
         -33,  -13,  -60,  -68,  -62,  -65,  -34,  -39,
          27,   10,   -2,  -32,  -32,  -18,   25,   31,
          -8,   43,   36,  -38,   21,  -35,   33,   39,
    ],
    // king EG
    [
        -118,  -52,  -45,  -12,  -26,  -18,  -24, -119,
         -30,    7,   14,    2,   14,   27,   19,  -23,
         -15,   11,   27,   37,   32,   29,   28,   -6,
         -21,   17,   34,   45,   45,   38,   31,   -1,
         -23,    8,   29,   45,   42,   29,   17,   -2,
         -21,   -3,   16,   26,   25,   15,   -1,  -12,
         -32,  -16,   -7,    3,    3,   -6,  -24,  -29,
         -53,  -56,  -45,  -21,  -48,  -26,  -51,  -68,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns MG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          17,   12,   16,   15,    7,    6,  -10,   -1,
          30,   39,   33,   20,   19,    8,  -26,  -47,
          10,    8,   18,   17,   -2,    6,  -16,  -14,
          -3,  -13,  -17,  -11,  -20,  -12,  -24,  -13,
         -10,  -21,  -22,  -19,  -20,  -13,  -16,    5,
         -16,   -9,  -16,  -17,   -3,    1,    7,    1,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         -21,  -19,  -17,  -17,  -10,  -12,  -14,  -16,
         104,  102,   88,   58,   64,   83,   90,  110,
          55,   52,   42,   35,   37,   42,   57,   60,
          30,   27,   24,   18,   22,   24,   37,   36,
           1,    7,   12,    2,    8,    8,   20,    5,
           3,    5,   15,    8,   -1,    1,    5,    4,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 30;
const ROOK_OPEN_FILE_EG: i32 = 11;
const ROOK_CLOSED_FILE_MG: i32 = -14;
const ROOK_CLOSED_FILE_EG: i32 = -3;
const ROOK_SEMIOPEN_FILE_MG: i32 = 7;
const ROOK_SEMIOPEN_FILE_EG: i32 = 12;
const KING_OPEN_FILE_MG: i32 = -58;
const KING_OPEN_FILE_EG: i32 = -9;
const KING_CLOSED_FILE_MG: i32 = 14;
const KING_CLOSED_FILE_EG: i32 = -14;
const KING_SEMIOPEN_FILE_MG: i32 = -14;
const KING_SEMIOPEN_FILE_EG: i32 = 7;
const PAWN_SHIELDS: [[i32; NUM_PHASES]; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    [-148, -87],  /*0b0000*/
    [-131, -92],  /*0b0001*/
    [-116, -96],  /*0b0010*/
    [-97, -87],   /*0b0011*/
    [-112, -97],  /*0b0100*/
    [-116, -113], /*0b0101*/
    [-98, -95],   /*0b0110*/
    [-77, -118],  /*0b0111*/
    [-131, -92],  /*0b1000*/
    [-139, -124], /*0b1001*/
    [-113, -85],  /*0b1010*/
    [-88, -111],  /*0b1011*/
    [-112, -104], /*0b1100*/
    [-125, -133], /*0b1101*/
    [-82, -90],   /*0b1110*/
    [-100, -100], /*0b1111*/
    [-149, -90],  /*0b10000*/
    [-101, -87],  /*0b10001*/
    [-132, -127], /*0b10010*/
    [-94, -113],  /*0b10011*/
    [-109, -93],  /*0b10100*/
    [-74, -95],   /*0b10101*/
    [-113, -124], /*0b10110*/
    [-100, -100], /*0b10111*/
    [-133, -67],  /*0b11000*/
    [-101, -96],  /*0b11001*/
    [-96, -88],   /*0b11010*/
    [-100, -100], /*0b11011*/
    [-94, -92],   /*0b11100*/
    [-100, -100], /*0b11101*/
    [-100, -100], /*0b11110*/
    [-100, -100], /*0b11111*/
    [-118, -93],  /*0b100000*/
    [-113, -95],  /*0b100001*/
    [-93, -95],   /*0b100010*/
    [-72, -95],   /*0b100011*/
    [-133, -135], /*0b100100*/
    [-124, -151], /*0b100101*/
    [-108, -112], /*0b100110*/
    [-100, -100], /*0b100111*/
    [-130, -110], /*0b101000*/
    [-146, -120], /*0b101001*/
    [-91, -99],   /*0b101010*/
    [-100, -100], /*0b101011*/
    [-144, -140], /*0b101100*/
    [-100, -100], /*0b101101*/
    [-100, -100], /*0b101110*/
    [-100, -100], /*0b101111*/
    [-126, -82],  /*0b110000*/
    [-85, -87],   /*0b110001*/
    [-103, -125], /*0b110010*/
    [-100, -100], /*0b110011*/
    [-107, -102], /*0b110100*/
    [-100, -100], /*0b110101*/
    [-100, -100], /*0b110110*/
    [-100, -100], /*0b110111*/
    [-134, -88],  /*0b111000*/
    [-100, -100], /*0b111001*/
    [-100, -100], /*0b111010*/
    [-100, -100], /*0b111011*/
    [-100, -100], /*0b111100*/
    [-100, -100], /*0b111101*/
    [-100, -100], /*0b111110*/
    [-92, -126],  /*0b111111*/
    [-163, -81],  /*0b00*/
    [-98, -109],  /*0b01*/
    [-75, -100],  /*0b10*/
    [-49, -131],  /*0b11*/
    [-102, -104], /*0b100*/
    [-133, -146], /*0b101*/
    [-36, -132],  /*0b110*/
    [-100, -100], /*0b111*/
    [-79, -100],  /*0b1000*/
    [-90, -122],  /*0b1001*/
    [-54, -189],  /*0b1010*/
    [-100, -100], /*0b1011*/
    [-85, -111],  /*0b1100*/
    [-100, -100], /*0b1101*/
    [-100, -100], /*0b1110*/
    [-81, -118],  /*0b1111*/
    [-152, -83],  /*0b00*/
    [-116, -107], /*0b01*/
    [-111, -105], /*0b10*/
    [-89, -128],  /*0b11*/
    [-135, -99],  /*0b100*/
    [-110, -154], /*0b101*/
    [-114, -113], /*0b110*/
    [-100, -100], /*0b111*/
    [-138, -96],  /*0b1000*/
    [-88, -106],  /*0b1001*/
    [-105, -170], /*0b1010*/
    [-100, -100], /*0b1011*/
    [-131, -110], /*0b1100*/
    [-100, -100], /*0b1101*/
    [-100, -100], /*0b1110*/
    [-103, -161], /*0b1111*/
];

// TODO: Differentiate between rooks and kings in front of / behind pawns.

const PIECE_PHASE: [i32; 6] = [0, 1, 1, 2, 4, 0];

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
                let mut bb = pos.colored_piece_bb(color, piece);
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
