use strum::IntoEnumIterator;

use crate::eval::chess::{
    pawn_shield_idx, FileOpenness, NUM_PAWN_SHIELD_CONFIGURATIONS, NUM_PHASES,
};
use gears::games::chess::pieces::UncoloredChessPiece::{Bishop, Pawn, Rook};
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
          74,   81,   99,  105,  112,  150,  135,  104,
          57,   75,   72,   75,   98,   93,   89,   77,
          45,   65,   62,   76,   78,   78,   79,   58,
          38,   51,   51,   50,   61,   57,   73,   50,
          45,   60,   55,   39,   57,   74,   89,   51,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         269,  265,  261,  216,  215,  226,  273,  281,
         114,  114,   98,  101,   92,   84,  111,  106,
         103,   97,   88,   80,   80,   78,   91,   85,
          93,   93,   83,   80,   80,   78,   84,   77,
          88,   86,   81,   92,   87,   83,   78,   76,
          96,   96,   92,   97,  103,   94,   85,   84,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight MG
    [
         141,  190,  232,  262,  307,  213,  222,  182,
         270,  291,  325,  340,  319,  377,  292,  306,
         290,  327,  350,  357,  391,  397,  352,  315,
         291,  309,  331,  355,  337,  359,  317,  326,
         279,  294,  313,  313,  324,  315,  317,  290,
         259,  283,  297,  305,  313,  301,  304,  274,
         248,  260,  280,  290,  291,  297,  285,  282,
         215,  255,  247,  262,  269,  283,  260,  237,
    ],
    // knight EG
    [
         261,  309,  330,  318,  316,  312,  306,  239,
         305,  322,  326,  327,  324,  307,  317,  291,
         314,  326,  338,  341,  327,  322,  317,  307,
         324,  339,  350,  351,  353,  347,  340,  314,
         323,  335,  352,  355,  357,  344,  330,  314,
         309,  326,  336,  349,  344,  331,  317,  309,
         304,  321,  328,  331,  328,  322,  310,  309,
         291,  286,  313,  315,  316,  303,  292,  286,
    ],
    // bishop MG
    [
         274,  255,  255,  231,  243,  237,  290,  257,
         293,  321,  316,  297,  325,  326,  316,  294,
         308,  332,  334,  351,  351,  379,  355,  339,
         300,  320,  337,  353,  347,  344,  320,  307,
         299,  311,  319,  339,  335,  319,  314,  306,
         301,  316,  315,  320,  320,  316,  316,  316,
         306,  310,  322,  300,  309,  319,  329,  311,
         287,  308,  290,  284,  287,  288,  303,  296,
    ],
    // bishop EG
    [
         332,  342,  339,  348,  345,  341,  336,  330,
         324,  335,  339,  344,  335,  336,  338,  324,
         339,  338,  347,  338,  343,  344,  337,  337,
         339,  352,  348,  359,  355,  352,  352,  339,
         337,  350,  358,  356,  356,  353,  349,  329,
         335,  344,  351,  351,  356,  350,  335,  327,
         332,  329,  330,  342,  344,  334,  331,  314,
         315,  328,  315,  333,  330,  330,  317,  304,
    ],
    // rook MG
    [
         423,  412,  418,  424,  435,  447,  454,  464,
         400,  402,  420,  436,  424,  458,  455,  465,
         385,  407,  407,  409,  439,  453,  483,  445,
         377,  389,  390,  399,  405,  418,  425,  421,
         371,  373,  373,  385,  388,  386,  407,  398,
         370,  371,  373,  377,  385,  391,  425,  404,
         373,  377,  382,  383,  391,  404,  420,  386,
         394,  388,  387,  395,  402,  404,  407,  401,
    ],
    // rook EG
    [
         603,  612,  614,  609,  606,  605,  603,  597,
         612,  617,  616,  607,  609,  598,  596,  587,
         611,  606,  607,  601,  592,  586,  581,  585,
         609,  605,  607,  601,  592,  589,  588,  583,
         601,  601,  602,  597,  594,  593,  584,  580,
         594,  594,  592,  592,  586,  582,  563,  566,
         588,  589,  590,  588,  581,  575,  566,  574,
         591,  590,  598,  592,  584,  587,  579,  576,
    ],
    // queen MG
    [
         816,  832,  854,  883,  878,  896,  931,  868,
         848,  834,  841,  833,  841,  880,  872,  904,
         854,  851,  859,  866,  884,  922,  920,  904,
         842,  850,  856,  857,  861,  870,  868,  877,
         847,  847,  852,  857,  858,  858,  869,  867,
         846,  857,  854,  855,  859,  865,  878,  868,
         843,  853,  862,  860,  860,  869,  876,  873,
         842,  836,  839,  854,  848,  836,  841,  843,
    ],
    // queen EG
    [
        1142, 1150, 1165, 1155, 1160, 1145, 1098, 1134,
        1111, 1147, 1176, 1195, 1207, 1167, 1147, 1121,
        1110, 1133, 1160, 1170, 1178, 1158, 1126, 1124,
        1120, 1140, 1155, 1174, 1187, 1173, 1164, 1137,
        1114, 1141, 1145, 1166, 1159, 1152, 1136, 1127,
        1101, 1113, 1136, 1131, 1131, 1124, 1103, 1093,
        1100, 1101, 1096, 1106, 1106, 1082, 1059, 1039,
        1090, 1095, 1104, 1092, 1094, 1086, 1072, 1067,
    ],
    // king MG
    [
          86,   41,   65,  -22,   12,   22,   71,  192,
         -28,    5,  -15,   53,   28,    8,   43,   42,
         -42,   39,  -17,  -29,    8,   58,   33,    9,
         -24,  -31,  -52,  -74,  -73,  -47,  -50,  -65,
         -48,  -46,  -67,  -90,  -91,  -70,  -75,  -90,
         -32,   -4,  -54,  -64,  -58,  -61,  -27,  -40,
          28,   11,   -3,  -31,  -31,  -18,   25,   31,
         -14,   43,   36,  -38,   19,  -38,   32,   36,
    ],
    // king EG
    [
        -118,  -52,  -44,  -12,  -26,  -18,  -25, -122,
         -30,    7,   14,    2,   15,   28,   19,  -26,
         -16,   10,   27,   37,   32,   30,   28,   -8,
         -22,   15,   34,   45,   45,   37,   28,   -4,
         -24,    7,   28,   44,   42,   27,   16,   -6,
         -23,   -5,   15,   27,   25,   15,   -4,  -15,
         -33,  -16,   -6,    5,    5,   -5,  -23,  -31,
         -53,  -56,  -45,  -20,  -46,  -24,  -51,  -70,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns MG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          16,    7,   13,   14,    4,    4,  -16,   -2,
          30,   40,   31,   19,   20,    3,  -28,  -53,
          14,   13,   20,   16,   -2,    9,  -13,  -14,
           2,   -8,  -15,  -10,  -18,  -10,  -21,  -12,
          -5,  -14,  -19,  -12,  -15,   -8,  -13,    7,
         -12,   -6,  -12,  -12,    2,    2,    6,    2,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         -20,  -18,  -16,  -16,  -10,  -12,  -12,  -15,
         112,  109,   90,   59,   63,   86,   94,  116,
          65,   61,   47,   37,   38,   47,   64,   67,
          38,   37,   30,   22,   26,   31,   45,   43,
          11,   18,   18,    6,   13,   15,   31,   13,
          10,   13,   17,    9,    1,    6,   13,   10,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const BISHOP_PAIR_MG: i32 = 24;
const BISHOP_PAIR_EG: i32 = 58;
const ROOK_OPEN_FILE_MG: i32 = 29;
const ROOK_OPEN_FILE_EG: i32 = 11;
const ROOK_CLOSED_FILE_MG: i32 = -14;
const ROOK_CLOSED_FILE_EG: i32 = -3;
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
    [-132, -92],  /*0b0001*/
    [-118, -98],  /*0b0010*/
    [-95, -80],   /*0b0011*/
    [-112, -98],  /*0b0100*/
    [-117, -107], /*0b0101*/
    [-96, -89],   /*0b0110*/
    [-75, -108],  /*0b0111*/
    [-137, -94],  /*0b1000*/
    [-133, -117], /*0b1001*/
    [-120, -95],  /*0b1010*/
    [-88, -108],  /*0b1011*/
    [-115, -101], /*0b1100*/
    [-121, -124], /*0b1101*/
    [-88, -92],   /*0b1110*/
    [-100, -100], /*0b1111*/
    [-147, -90],  /*0b10000*/
    [-111, -96],  /*0b10001*/
    [-123, -118], /*0b10010*/
    [-97, -114],  /*0b10011*/
    [-115, -102], /*0b10100*/
    [-75, -95],   /*0b10101*/
    [-110, -121], /*0b10110*/
    [-100, -100], /*0b10111*/
    [-124, -62],  /*0b11000*/
    [-95, -94],   /*0b11001*/
    [-88, -83],   /*0b11010*/
    [-100, -100], /*0b11011*/
    [-92, -94],   /*0b11100*/
    [-100, -100], /*0b11101*/
    [-100, -100], /*0b11110*/
    [-100, -100], /*0b11111*/
    [-122, -97],  /*0b100000*/
    [-115, -94],  /*0b100001*/
    [-100, -104], /*0b100010*/
    [-79, -98],   /*0b100011*/
    [-131, -127], /*0b100100*/
    [-123, -141], /*0b100101*/
    [-109, -107], /*0b100110*/
    [-100, -100], /*0b100111*/
    [-128, -108], /*0b101000*/
    [-135, -110], /*0b101001*/
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
    [-103, -101], /*0b110100*/
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
    [-86, -123],  /*0b111111*/
    [-164, -84],  /*0b00*/
    [-99, -113],  /*0b01*/
    [-74, -100],  /*0b10*/
    [-48, -123],  /*0b11*/
    [-95, -105],  /*0b100*/
    [-128, -135], /*0b101*/
    [-39, -143],  /*0b110*/
    [-100, -100], /*0b111*/
    [-76, -103],  /*0b1000*/
    [-90, -130],  /*0b1001*/
    [-47, -173],  /*0b1010*/
    [-100, -100], /*0b1011*/
    [-68, -105],  /*0b1100*/
    [-100, -100], /*0b1101*/
    [-100, -100], /*0b1110*/
    [-71, -114],  /*0b1111*/
    [-153, -84],  /*0b00*/
    [-114, -103], /*0b01*/
    [-112, -109], /*0b10*/
    [-85, -120],  /*0b11*/
    [-137, -98],  /*0b100*/
    [-102, -141], /*0b101*/
    [-116, -118], /*0b110*/
    [-100, -100], /*0b111*/
    [-136, -93],  /*0b1000*/
    [-93, -113],  /*0b1001*/
    [-99, -160],  /*0b1010*/
    [-100, -100], /*0b1011*/
    [-121, -98],  /*0b1100*/
    [-100, -100], /*0b1101*/
    [-100, -100], /*0b1110*/
    [-99, -153],  /*0b1111*/
];
const PAWN_PROTECTION: [[i32; NUM_PHASES]; NUM_CHESS_PIECES] =
    [[17, 17], [6, 13], [0, 6], [6, 8], [-7, 12], [-30, 15]];
const PAWN_ATTACKS: [[i32; NUM_PHASES]; NUM_CHESS_PIECES] =
    [[0, 0], [33, 13], [48, 32], [50, -2], [39, -31], [0, 0]];

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

            if pos.colored_piece_bb(color, Bishop).more_than_one_bit_set() {
                mg += Score(BISHOP_PAIR_MG);
                eg += Score(BISHOP_PAIR_EG);
            }
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
