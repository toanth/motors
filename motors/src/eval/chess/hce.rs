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
         160,  160,  153,  182,  163,  146,   58,   49,
          72,   72,   98,  105,  113,  150,  130,  106,
          55,   64,   72,   75,   98,   94,   82,   78,
          43,   53,   62,   77,   77,   78,   71,   59,
          36,   42,   52,   51,   62,   59,   66,   52,
          42,   48,   53,   36,   55,   72,   80,   51,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // pawn EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         275,  269,  265,  219,  219,  230,  277,  286,
         112,  109,  102,  103,   95,   85,  106,  106,
         102,   94,   93,   84,   83,   81,   87,   85,
          92,   88,   87,   85,   84,   81,   79,   79,
          88,   83,   87,   96,   92,   87,   75,   78,
          93,   89,   95,   99,  105,   95,   78,   83,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // knight MG
    [
         138,  187,  231,  258,  305,  210,  218,  177,
         270,  290,  324,  342,  320,  377,  290,  306,
         290,  328,  351,  359,  393,  400,  353,  316,
         291,  309,  332,  356,  338,  361,  318,  327,
         280,  295,  314,  314,  324,  316,  319,  291,
         260,  284,  297,  306,  315,  302,  305,  275,
         250,  261,  281,  290,  292,  298,  287,  283,
         217,  256,  249,  264,  270,  284,  260,  239,
    ],
    // knight EG
    [
         259,  311,  332,  321,  318,  314,  309,  240,
         307,  324,  327,  328,  326,  308,  318,  292,
         315,  327,  340,  343,  329,  323,  319,  309,
         325,  340,  351,  353,  354,  349,  341,  315,
         324,  336,  353,  355,  358,  346,  332,  316,
         310,  328,  335,  350,  345,  331,  318,  311,
         305,  323,  329,  331,  329,  323,  312,  310,
         291,  286,  313,  315,  316,  304,  292,  288,
    ],
    // bishop MG
    [
         294,  274,  276,  253,  263,  258,  311,  276,
         315,  341,  338,  319,  345,  347,  335,  316,
         330,  354,  356,  374,  374,  400,  376,  362,
         322,  343,  359,  375,  369,  365,  343,  329,
         322,  333,  342,  362,  358,  342,  335,  330,
         324,  339,  337,  342,  343,  338,  339,  338,
         329,  332,  345,  323,  332,  342,  352,  334,
         310,  331,  313,  307,  310,  311,  327,  320,
    ],
    // bishop EG
    [
         338,  350,  346,  356,  353,  347,  342,  337,
         331,  342,  347,  351,  342,  342,  345,  329,
         347,  345,  353,  346,  350,  351,  344,  344,
         346,  359,  354,  364,  362,  358,  359,  346,
         344,  357,  364,  362,  361,  360,  355,  336,
         343,  351,  358,  357,  362,  355,  342,  334,
         338,  337,  337,  349,  352,  340,  337,  320,
         323,  335,  323,  341,  337,  338,  324,  311,
    ],
    // rook MG
    [
         424,  413,  420,  426,  436,  445,  454,  464,
         402,  402,  421,  438,  426,  459,  454,  467,
         389,  408,  409,  411,  441,  455,  485,  448,
         378,  388,  391,  401,  406,  417,  425,  423,
         371,  373,  374,  386,  389,  386,  408,  398,
         370,  371,  373,  378,  386,  391,  425,  404,
         373,  378,  383,  384,  392,  405,  420,  386,
         395,  389,  388,  396,  403,  406,  408,  402,
    ],
    // rook EG
    [
         604,  613,  615,  610,  607,  606,  605,  599,
         613,  619,  617,  608,  610,  599,  598,  588,
         611,  607,  608,  602,  592,  587,  581,  585,
         610,  607,  609,  603,  594,  591,  589,  584,
         603,  603,  603,  598,  595,  595,  586,  582,
         595,  595,  593,  593,  587,  583,  565,  568,
         589,  591,  592,  589,  582,  576,  567,  575,
         592,  592,  599,  592,  585,  588,  581,  577,
    ],
    // queen MG
    [
         806,  826,  846,  876,  873,  889,  922,  857,
         839,  823,  833,  825,  832,  871,  860,  896,
         845,  843,  850,  859,  877,  914,  912,  896,
         834,  842,  848,  850,  853,  862,  860,  868,
         840,  839,  843,  849,  850,  850,  862,  859,
         838,  850,  847,  846,  852,  858,  871,  860,
         835,  845,  854,  852,  852,  861,  868,  866,
         834,  828,  832,  847,  840,  829,  834,  835,
    ],
    // queen EG
    [
        1155, 1162, 1177, 1167, 1171, 1156, 1111, 1149,
        1124, 1161, 1189, 1209, 1220, 1180, 1161, 1134,
        1123, 1146, 1173, 1182, 1191, 1170, 1138, 1136,
        1132, 1152, 1166, 1188, 1199, 1185, 1176, 1150,
        1125, 1152, 1159, 1178, 1172, 1165, 1148, 1140,
        1111, 1124, 1147, 1144, 1144, 1135, 1115, 1105,
        1111, 1112, 1108, 1118, 1119, 1093, 1070, 1049,
        1100, 1106, 1116, 1105, 1106, 1097, 1084, 1077,
    ],
    // king MG
    [
          90,   41,   64,  -20,   17,   30,   75,  195,
         -27,    5,  -15,   53,   29,   12,   46,   46,
         -39,   41,  -15,  -26,   13,   63,   38,   11,
         -21,  -27,  -48,  -73,  -69,  -44,  -47,  -61,
         -46,  -42,  -66,  -88,  -89,  -69,  -75,  -88,
         -31,   -2,  -53,  -64,  -59,  -61,  -27,  -40,
          27,   11,   -3,  -32,  -32,  -18,   24,   29,
         -11,   43,   37,  -38,   20,  -37,   32,   36,
    ],
    // king EG
    [
        -116,  -51,  -43,  -10,  -24,  -17,  -22, -120,
         -28,    8,   16,    4,   17,   30,   21,  -24,
         -14,   12,   28,   38,   34,   31,   30,   -6,
         -20,   15,   34,   45,   46,   38,   29,   -3,
         -23,    7,   28,   44,   42,   28,   16,   -6,
         -22,   -6,   15,   27,   24,   15,   -3,  -15,
         -33,  -16,   -6,    4,    4,   -5,  -23,  -31,
         -52,  -55,  -45,  -21,  -47,  -25,  -50,  -69,
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [[i32; NUM_SQUARES]; 2] = [
    // passed pawns MG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
          21,    6,   14,   16,    5,    4,  -18,    3,
          42,   49,   33,   22,   21,    4,  -22,  -45,
          28,   25,   22,   20,    0,   11,   -3,   -5,
          14,    4,  -12,   -6,  -16,   -7,  -13,   -4,
           7,   -5,  -16,  -10,  -13,   -6,   -7,   16,
           1,    3,  -10,   -8,    3,    5,   12,   11,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
    // passed pawns EG
    [
           0,    0,    0,    0,    0,    0,    0,    0,
         -14,  -14,  -12,  -13,   -6,   -8,   -8,  -10,
         123,  122,   95,   63,   66,   91,  105,  125,
          75,   72,   50,   39,   41,   51,   73,   74,
          49,   48,   34,   24,   29,   36,   55,   51,
          20,   27,   21,   10,   16,   19,   38,   20,
          20,   23,   21,   14,    5,   10,   21,   18,
           0,    0,    0,    0,    0,    0,    0,    0,
    ],
];
const ROOK_OPEN_FILE_MG: i32 = 30;
const ROOK_OPEN_FILE_EG: i32 = 12;
const ROOK_CLOSED_FILE_MG: i32 = -13;
const ROOK_CLOSED_FILE_EG: i32 = -4;
const ROOK_SEMIOPEN_FILE_MG: i32 = 9;
const ROOK_SEMIOPEN_FILE_EG: i32 = 12;
const KING_OPEN_FILE_MG: i32 = -52;
const KING_OPEN_FILE_EG: i32 = -7;
const KING_CLOSED_FILE_MG: i32 = 17;
const KING_CLOSED_FILE_EG: i32 = -13;
const KING_SEMIOPEN_FILE_MG: i32 = -8;
const KING_SEMIOPEN_FILE_EG: i32 = 11;
const PAWN_SHIELDS: [[i32; NUM_PHASES]; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    [-152, -91],  /*0b0000*/
    [-135, -90],  /*0b0001*/
    [-116, -95],  /*0b0010*/
    [-96, -87],   /*0b0011*/
    [-115, -97],  /*0b0100*/
    [-116, -103], /*0b0101*/
    [-97, -95],   /*0b0110*/
    [-76, -116],  /*0b0111*/
    [-139, -92],  /*0b1000*/
    [-128, -108], /*0b1001*/
    [-120, -93],  /*0b1010*/
    [-83, -109],  /*0b1011*/
    [-117, -98],  /*0b1100*/
    [-115, -112], /*0b1101*/
    [-89, -93],   /*0b1110*/
    [-100, -100], /*0b1111*/
    [-147, -87],  /*0b10000*/
    [-111, -94],  /*0b10001*/
    [-108, -103], /*0b10010*/
    [-89, -109],  /*0b10011*/
    [-116, -99],  /*0b10100*/
    [-76, -98],   /*0b10101*/
    [-102, -115], /*0b10110*/
    [-100, -100], /*0b10111*/
    [-127, -69],  /*0b11000*/
    [-98, -96],   /*0b11001*/
    [-85, -84],   /*0b11010*/
    [-100, -100], /*0b11011*/
    [-94, -96],   /*0b11100*/
    [-100, -100], /*0b11101*/
    [-100, -100], /*0b11110*/
    [-100, -100], /*0b11111*/
    [-124, -95],  /*0b100000*/
    [-116, -91],  /*0b100001*/
    [-100, -101], /*0b100010*/
    [-79, -99],   /*0b100011*/
    [-124, -116], /*0b100100*/
    [-111, -124], /*0b100101*/
    [-107, -102], /*0b100110*/
    [-100, -100], /*0b100111*/
    [-131, -104], /*0b101000*/
    [-128, -99],  /*0b101001*/
    [-104, -106], /*0b101010*/
    [-100, -100], /*0b101011*/
    [-132, -115], /*0b101100*/
    [-100, -100], /*0b101101*/
    [-100, -100], /*0b101110*/
    [-100, -100], /*0b101111*/
    [-118, -82],  /*0b110000*/
    [-82, -90],   /*0b110001*/
    [-90, -114],  /*0b110010*/
    [-100, -100], /*0b110011*/
    [-104, -99],  /*0b110100*/
    [-100, -100], /*0b110101*/
    [-100, -100], /*0b110110*/
    [-100, -100], /*0b110111*/
    [-118, -84],  /*0b111000*/
    [-100, -100], /*0b111001*/
    [-100, -100], /*0b111010*/
    [-100, -100], /*0b111011*/
    [-100, -100], /*0b111100*/
    [-100, -100], /*0b111101*/
    [-100, -100], /*0b111110*/
    [-86, -123],  /*0b111111*/
    [-165, -84],  /*0b00*/
    [-99, -109],  /*0b01*/
    [-76, -99],   /*0b10*/
    [-48, -130],  /*0b11*/
    [-96, -104],  /*0b100*/
    [-118, -117], /*0b101*/
    [-39, -141],  /*0b110*/
    [-100, -100], /*0b111*/
    [-75, -103],  /*0b1000*/
    [-92, -131],  /*0b1001*/
    [-37, -159],  /*0b1010*/
    [-100, -100], /*0b1011*/
    [-73, -116],  /*0b1100*/
    [-100, -100], /*0b1101*/
    [-100, -100], /*0b1110*/
    [-63, -114],  /*0b1111*/
    [-156, -83],  /*0b00*/
    [-116, -100], /*0b01*/
    [-111, -106], /*0b10*/
    [-86, -126],  /*0b11*/
    [-139, -94],  /*0b100*/
    [-94, -126],  /*0b101*/
    [-117, -116], /*0b110*/
    [-100, -100], /*0b111*/
    [-136, -92],  /*0b1000*/
    [-94, -110],  /*0b1001*/
    [-87, -144],  /*0b1010*/
    [-100, -100], /*0b1011*/
    [-124, -106], /*0b1100*/
    [-100, -100], /*0b1101*/
    [-100, -100], /*0b1110*/
    [-91, -145],  /*0b1111*/
];
const PAWN_PROTECTION: [[i32; NUM_PHASES]; NUM_CHESS_PIECES] =
    [[14, 7], [6, 12], [0, 5], [7, 8], [-7, 12], [-32, 15]];
const PAWN_ATTACKS: [[i32; NUM_PHASES]; NUM_CHESS_PIECES] =
    [[0, 0], [33, 12], [48, 30], [50, -4], [40, -33], [0, 0]];
const ISOLATED_PAWN_MG: i32 = -12;
const ISOLATED_PAWN_EG: i32 = -12;

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
                        if (neighbors & our_pawns).is_zero() {
                            mg += Score(ISOLATED_PAWN_MG);
                            eg += Score(ISOLATED_PAWN_EG);
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
