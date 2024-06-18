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
         160,  163,  154,  183,  164,  146,   61,   48,
          73,   77,   97,  105,  110,  150,  134,  106,
          56,   72,   71,   75,   99,   93,   90,   79,
          46,   61,   63,   77,   78,   78,   78,   60,
          40,   48,   52,   52,   61,   58,   71,   55,
          43,   47,   52,   39,   47,   71,   75,   54,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         275,  270,  266,  220,  220,  231,  278,  286,
         115,  112,  103,  106,   97,   87,  110,  108,
         105,   99,   92,   86,   85,   82,   93,   87,
          95,   93,   88,   86,   85,   82,   84,   81,
          92,   88,   87,   97,   93,   88,   79,   81,
          96,   91,   96,  103,  107,   97,   80,   87,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight MG
    [
         137,  188,  229,  258,  304,  210,  218,  175,
         269,  290,  322,  341,  319,  376,  292,  305,
         290,  326,  351,  358,  393,  399,  351,  315,
         290,  309,  331,  355,  338,  361,  318,  327,
         280,  296,  314,  314,  324,  316,  320,  292,
         260,  283,  297,  306,  314,  302,  305,  275,
         249,  261,  280,  290,  292,  297,  286,  282,
         216,  255,  247,  263,  269,  283,  260,  238,
    ],
    // knight EG
    [
         258,  310,  332,  320,  318,  313,  308,  240,
         306,  323,  327,  327,  325,  307,  317,  291,
         315,  326,  339,  342,  329,  323,  318,  309,
         324,  340,  351,  352,  353,  348,  341,  315,
         323,  335,  353,  355,  357,  345,  332,  315,
         309,  327,  335,  349,  345,  331,  317,  309,
         304,  322,  328,  330,  328,  322,  311,  310,
         290,  285,  312,  315,  315,  304,  291,  288,
    ],
    // bishop MG
    [
         293,  274,  276,  252,  262,  259,  311,  275,
         312,  340,  336,  318,  344,  346,  336,  315,
         329,  351,  355,  373,  373,  399,  373,  361,
         320,  341,  358,  373,  368,  364,  342,  329,
         321,  332,  341,  360,  357,  341,  335,  329,
         323,  337,  336,  341,  342,  338,  337,  338,
         328,  331,  344,  323,  331,  341,  351,  334,
         307,  330,  312,  307,  311,  311,  326,  319,
    ],
    // bishop EG
    [
         336,  349,  345,  354,  351,  346,  341,  336,
         330,  341,  345,  350,  341,  341,  344,  328,
         346,  344,  352,  344,  349,  350,  343,  343,
         345,  357,  353,  363,  360,  357,  358,  345,
         343,  356,  363,  361,  360,  359,  354,  335,
         343,  350,  358,  357,  362,  354,  340,  334,
         338,  336,  336,  349,  350,  339,  336,  319,
         322,  334,  322,  340,  336,  337,  323,  310,
    ],
    // rook MG
    [
         423,  412,  419,  424,  435,  443,  453,  462,
         400,  402,  420,  437,  425,  458,  455,  466,
         387,  405,  408,  409,  440,  454,  482,  447,
         377,  386,  389,  399,  404,  416,  423,  421,
         371,  371,  373,  384,  388,  385,  407,  397,
         370,  370,  373,  377,  385,  391,  425,  403,
         372,  376,  382,  383,  391,  404,  418,  384,
         394,  388,  387,  394,  402,  404,  407,  401,
    ],
    // rook EG
    [
         603,  612,  614,  609,  606,  605,  603,  598,
         612,  618,  616,  607,  609,  598,  597,  587,
         610,  607,  607,  602,  591,  587,  580,  585,
         609,  606,  608,  602,  593,  590,  589,  583,
         602,  602,  602,  597,  594,  594,  585,  582,
         594,  594,  592,  592,  586,  582,  564,  567,
         588,  590,  591,  588,  581,  576,  566,  575,
         591,  590,  598,  592,  584,  587,  580,  576,
    ],
    // queen MG
    [
         804,  825,  845,  874,  871,  885,  921,  854,
         836,  822,  831,  823,  831,  869,  859,  893,
         843,  840,  849,  857,  875,  913,  909,  894,
         831,  841,  846,  848,  851,  860,  858,  867,
         838,  838,  843,  848,  848,  849,  860,  857,
         837,  849,  845,  845,  850,  856,  869,  859,
         833,  842,  853,  850,  851,  859,  866,  864,
         832,  827,  831,  845,  839,  828,  832,  833,
    ],
    // queen EG
    [
        1153, 1159, 1174, 1165, 1168, 1154, 1108, 1147,
        1122, 1158, 1187, 1206, 1218, 1176, 1159, 1133,
        1121, 1144, 1171, 1180, 1188, 1168, 1136, 1135,
        1130, 1150, 1164, 1185, 1196, 1182, 1174, 1147,
        1122, 1150, 1155, 1174, 1170, 1163, 1146, 1137,
        1109, 1121, 1144, 1141, 1141, 1133, 1113, 1103,
        1108, 1110, 1106, 1116, 1117, 1091, 1067, 1046,
        1099, 1103, 1113, 1102, 1102, 1094, 1082, 1075,
    ],
    // king MG
    [
          85,   39,   63,  -23,   15,   27,   73,  193,
         -28,    4,  -15,   53,   28,   12,   45,   44,
         -43,   40,  -16,  -28,   11,   62,   36,    9,
         -23,  -29,  -50,  -74,  -70,  -46,  -48,  -64,
         -47,  -45,  -66,  -90,  -91,  -71,  -76,  -91,
         -31,   -4,  -56,  -66,  -61,  -62,  -29,  -41,
          28,   11,   -5,  -32,  -33,  -18,   22,   28,
          -9,   43,   36,  -38,   19,  -38,   33,   38,
    ],
    // king EG
    [
        -118,  -52,  -44,  -11,  -25,  -18,  -24, -122,
         -30,    7,   14,    2,   16,   28,   19,  -25,
         -15,   10,   27,   37,   33,   30,   28,   -7,
         -22,   14,   33,   44,   45,   37,   28,   -4,
         -24,    6,   27,   43,   41,   27,   16,   -6,
         -23,   -6,   14,   26,   24,   14,   -4,  -15,
         -33,  -16,   -7,    3,    4,   -6,  -22,  -31,
         -53,  -55,  -46,  -22,  -48,  -25,  -50,  -70,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns MG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          21,    9,   15,   17,    6,    4,  -15,    2,
          42,   50,   37,   24,   26,    7,  -22,  -46,
          27,   22,   27,   22,    2,   15,   -8,   -7,
          13,    1,   -9,   -5,  -14,   -2,  -15,   -6,
           4,   -6,  -12,  -10,  -11,   -1,   -8,   13,
          -2,    5,   -7,   -9,    7,   10,   14,    8,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         -14,  -13,  -11,  -12,   -5,   -7,   -7,  -10,
         122,  120,   95,   62,   67,   92,  104,  124,
          73,   68,   51,   39,   40,   52,   70,   74,
          46,   45,   34,   24,   29,   36,   52,   49,
          18,   25,   22,   10,   17,   19,   38,   18,
          18,   23,   21,   13,    5,   11,   23,   15,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 30;
const ROOK_OPEN_FILE_EG: i32 = 11;
const ROOK_CLOSED_FILE_MG: i32 = -13;
const ROOK_CLOSED_FILE_EG: i32 = -4;
const ROOK_SEMIOPEN_FILE_MG: i32 = 9;
const ROOK_SEMIOPEN_FILE_EG: i32 = 12;
const KING_OPEN_FILE_MG: i32 = -54;
const KING_OPEN_FILE_EG: i32 = -8;
const KING_CLOSED_FILE_MG: i32 = 17;
const KING_CLOSED_FILE_EG: i32 = -13;
const KING_SEMIOPEN_FILE_MG: i32 = -11;
const KING_SEMIOPEN_FILE_EG: i32 = 10;
const PAWN_SHIELDS: [[i32; NUM_PHASES]; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    [-151, -90],  /*0b0000*/
    [-130, -89],  /*0b0001*/
    [-107, -96],  /*0b0010*/
    [-94, -91],   /*0b0011*/
    [-112, -96],  /*0b0100*/
    [-110, -102], /*0b0101*/
    [-95, -99],   /*0b0110*/
    [-75, -123],  /*0b0111*/
    [-138, -91],  /*0b1000*/
    [-119, -108], /*0b1001*/
    [-112, -94],  /*0b1010*/
    [-86, -119],  /*0b1011*/
    [-112, -97],  /*0b1100*/
    [-106, -112], /*0b1101*/
    [-91, -104],  /*0b1110*/
    [-100, -100], /*0b1111*/
    [-142, -87],  /*0b10000*/
    [-110, -94],  /*0b10001*/
    [-99, -105],  /*0b10010*/
    [-93, -122],  /*0b10011*/
    [-114, -100], /*0b10100*/
    [-67, -88],   /*0b10101*/
    [-106, -128], /*0b10110*/
    [-100, -100], /*0b10111*/
    [-130, -74],  /*0b11000*/
    [-95, -95],   /*0b11001*/
    [-76, -83],   /*0b11010*/
    [-100, -100], /*0b11011*/
    [-92, -97],   /*0b11100*/
    [-100, -100], /*0b11101*/
    [-100, -100], /*0b11110*/
    [-100, -100], /*0b11111*/
    [-124, -95],  /*0b100000*/
    [-109, -89],  /*0b100001*/
    [-90, -101],  /*0b100010*/
    [-81, -111],  /*0b100011*/
    [-117, -117], /*0b100100*/
    [-107, -128], /*0b100101*/
    [-106, -115], /*0b100110*/
    [-100, -100], /*0b100111*/
    [-127, -103], /*0b101000*/
    [-120, -99],  /*0b101001*/
    [-98, -109],  /*0b101010*/
    [-100, -100], /*0b101011*/
    [-120, -117], /*0b101100*/
    [-100, -100], /*0b101101*/
    [-100, -100], /*0b101110*/
    [-100, -100], /*0b101111*/
    [-124, -87],  /*0b110000*/
    [-83, -91],   /*0b110001*/
    [-84, -113],  /*0b110010*/
    [-100, -100], /*0b110011*/
    [-97, -97],   /*0b110100*/
    [-100, -100], /*0b110101*/
    [-100, -100], /*0b110110*/
    [-100, -100], /*0b110111*/
    [-123, -91],  /*0b111000*/
    [-100, -100], /*0b111001*/
    [-100, -100], /*0b111010*/
    [-100, -100], /*0b111011*/
    [-100, -100], /*0b111100*/
    [-100, -100], /*0b111101*/
    [-100, -100], /*0b111110*/
    [-80, -130],  /*0b111111*/
    [-163, -83],  /*0b00*/
    [-97, -110],  /*0b01*/
    [-71, -98],   /*0b10*/
    [-51, -136],  /*0b11*/
    [-93, -103],  /*0b100*/
    [-121, -123], /*0b101*/
    [-38, -141],  /*0b110*/
    [-100, -100], /*0b111*/
    [-77, -103],  /*0b1000*/
    [-90, -128],  /*0b1001*/
    [-37, -163],  /*0b1010*/
    [-100, -100], /*0b1011*/
    [-81, -119],  /*0b1100*/
    [-100, -100], /*0b1101*/
    [-100, -100], /*0b1110*/
    [-67, -122],  /*0b1111*/
    [-152, -82],  /*0b00*/
    [-110, -100], /*0b01*/
    [-108, -107], /*0b10*/
    [-87, -131],  /*0b11*/
    [-139, -95],  /*0b100*/
    [-91, -128],  /*0b101*/
    [-114, -115], /*0b110*/
    [-100, -100], /*0b111*/
    [-135, -91],  /*0b1000*/
    [-91, -110],  /*0b1001*/
    [-91, -149],  /*0b1010*/
    [-100, -100], /*0b1011*/
    [-130, -111], /*0b1100*/
    [-100, -100], /*0b1101*/
    [-100, -100], /*0b1110*/
    [-96, -155],  /*0b1111*/
];
const PAWN_PROTECTION: [[i32; NUM_PHASES]; NUM_CHESS_PIECES] =
    [[16, 12], [5, 12], [1, 5], [7, 8], [-7, 13], [-31, 15]];
const PAWN_ATTACKS: [[i32; NUM_PHASES]; NUM_CHESS_PIECES] =
    [[0, 0], [33, 12], [48, 31], [49, -3], [40, -34], [0, 0]];
const UNSUPPORTED_PAWN_MG: i32 = -11;
const UNSUPPORTED_PAWN_EG: i32 = -12;

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
                        let file = ChessBitboard::file_no(square.file());
                        let neighbors = file.east() | file.west();
                        let supporting = neighbors & !blocking;
                        if (supporting & our_pawns).is_zero() {
                            mg += Score(UNSUPPORTED_PAWN_MG);
                            eg += Score(UNSUPPORTED_PAWN_EG);
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
