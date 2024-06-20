use strum::IntoEnumIterator;

use crate::eval::chess::{pawn_shield_idx, FileOpenness, NUM_PAWN_SHIELD_CONFIGURATIONS};
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
use gears::score::{p, PhasedScore, Score};

use crate::eval::chess::hce::FileOpenness::{Closed, Open, SemiClosed, SemiOpen};
use crate::eval::Eval;

#[derive(Default, Debug)]
pub struct HandCraftedEval {}

/// Eval values tuned on a combination of the zurichess dataset and a dataset used by 4ku,
/// created by GCP using his engine Stoofvlees and filtered by cj5716 using Stockfish at depth 9,
/// using my own tuner `pliers`.
#[rustfmt::skip]
const PSQTS: [[PhasedScore; NUM_SQUARES]; NUM_CHESS_PIECES] = [
    // pawn
    [
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
        p( 136,  175),    p( 134,  173),    p( 133,  172),    p( 147,  150),    p( 133,  153),    p( 125,  157),    p(  72,  181),    p(  71,  183),
        p(  74,  114),    p(  81,  114),    p(  99,   98),    p( 105,  101),    p( 112,   92),    p( 150,   84),    p( 135,  111),    p( 104,  106),
        p(  57,  103),    p(  75,   97),    p(  72,   88),    p(  75,   80),    p(  98,   80),    p(  93,   78),    p(  89,   91),    p(  77,   85),
        p(  45,   93),    p(  65,   93),    p(  62,   83),    p(  76,   80),    p(  78,   80),    p(  78,   78),    p(  79,   84),    p(  58,   77),
        p(  38,   88),    p(  51,   86),    p(  51,   81),    p(  50,   92),    p(  61,   87),    p(  57,   83),    p(  73,   78),    p(  50,   76),
        p(  45,   96),    p(  60,   96),    p(  55,   92),    p(  39,   97),    p(  57,  103),    p(  74,   94),    p(  89,   85),    p(  51,   84),
        p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),    p( 100,  100),
    ],
    // knight
    [
        p( 141,  261),    p( 190,  309),    p( 232,  330),    p( 262,  318),    p( 307,  316),    p( 213,  312),    p( 222,  306),    p( 182,  239),
        p( 270,  305),    p( 291,  322),    p( 325,  326),    p( 340,  327),    p( 319,  324),    p( 377,  307),    p( 292,  317),    p( 306,  291),
        p( 290,  314),    p( 327,  326),    p( 350,  338),    p( 357,  341),    p( 391,  327),    p( 397,  322),    p( 352,  317),    p( 315,  307),
        p( 291,  324),    p( 309,  339),    p( 331,  350),    p( 355,  351),    p( 337,  353),    p( 359,  347),    p( 317,  340),    p( 326,  314),
        p( 279,  323),    p( 294,  335),    p( 313,  352),    p( 313,  355),    p( 324,  357),    p( 315,  344),    p( 317,  330),    p( 290,  314),
        p( 259,  309),    p( 283,  326),    p( 297,  336),    p( 305,  349),    p( 313,  344),    p( 301,  331),    p( 304,  317),    p( 274,  309),
        p( 248,  304),    p( 260,  321),    p( 280,  328),    p( 290,  331),    p( 291,  328),    p( 297,  322),    p( 285,  310),    p( 282,  309),
        p( 215,  291),    p( 255,  286),    p( 247,  313),    p( 262,  315),    p( 269,  316),    p( 283,  303),    p( 260,  292),    p( 237,  286),
    ],
    // bishop
    [
        p( 274,  332),    p( 255,  342),    p( 255,  339),    p( 231,  348),    p( 243,  345),    p( 237,  341),    p( 290,  336),    p( 257,  330),
        p( 293,  324),    p( 321,  335),    p( 316,  339),    p( 297,  344),    p( 325,  335),    p( 326,  336),    p( 316,  338),    p( 294,  324),
        p( 308,  339),    p( 332,  338),    p( 334,  347),    p( 351,  338),    p( 351,  343),    p( 379,  344),    p( 355,  337),    p( 339,  337),
        p( 300,  339),    p( 320,  352),    p( 337,  348),    p( 353,  359),    p( 347,  355),    p( 344,  352),    p( 320,  352),    p( 307,  339),
        p( 299,  337),    p( 311,  350),    p( 319,  358),    p( 339,  356),    p( 335,  356),    p( 319,  353),    p( 313,  349),    p( 306,  329),
        p( 301,  335),    p( 316,  344),    p( 315,  351),    p( 320,  351),    p( 320,  356),    p( 316,  350),    p( 316,  335),    p( 316,  327),
        p( 306,  332),    p( 310,  329),    p( 322,  330),    p( 300,  342),    p( 309,  344),    p( 319,  334),    p( 329,  331),    p( 311,  314),
        p( 287,  315),    p( 308,  328),    p( 290,  315),    p( 284,  333),    p( 287,  330),    p( 288,  330),    p( 303,  317),    p( 296,  304),
    ],
    // rook
    [
        p( 423,  603),    p( 412,  612),    p( 418,  614),    p( 424,  609),    p( 435,  606),    p( 447,  605),    p( 454,  603),    p( 464,  597),
        p( 400,  612),    p( 402,  617),    p( 420,  616),    p( 436,  607),    p( 424,  609),    p( 458,  598),    p( 455,  596),    p( 465,  587),
        p( 385,  611),    p( 407,  606),    p( 407,  607),    p( 409,  601),    p( 439,  592),    p( 453,  586),    p( 483,  581),    p( 445,  585),
        p( 377,  609),    p( 389,  605),    p( 390,  607),    p( 399,  601),    p( 405,  592),    p( 418,  589),    p( 425,  588),    p( 421,  583),
        p( 371,  601),    p( 373,  601),    p( 373,  602),    p( 385,  597),    p( 388,  594),    p( 386,  593),    p( 407,  584),    p( 398,  580),
        p( 370,  594),    p( 371,  594),    p( 373,  592),    p( 377,  592),    p( 385,  586),    p( 391,  582),    p( 425,  563),    p( 404,  566),
        p( 373,  588),    p( 377,  589),    p( 382,  590),    p( 383,  588),    p( 391,  581),    p( 404,  575),    p( 420,  566),    p( 386,  574),
        p( 394,  591),    p( 388,  590),    p( 387,  598),    p( 395,  592),    p( 402,  584),    p( 404,  587),    p( 407,  579),    p( 401,  576),
    ],
    // queen
    [
        p( 816, 1142),    p( 832, 1150),    p( 854, 1165),    p( 883, 1155),    p( 878, 1160),    p( 896, 1145),    p( 931, 1098),    p( 868, 1134),
        p( 848, 1111),    p( 834, 1147),    p( 841, 1176),    p( 833, 1195),    p( 841, 1207),    p( 880, 1167),    p( 872, 1147),    p( 904, 1121),
        p( 854, 1110),    p( 851, 1133),    p( 859, 1160),    p( 866, 1170),    p( 884, 1178),    p( 922, 1158),    p( 920, 1126),    p( 904, 1124),
        p( 842, 1120),    p( 850, 1140),    p( 856, 1155),    p( 857, 1174),    p( 861, 1187),    p( 870, 1173),    p( 868, 1164),    p( 877, 1137),
        p( 847, 1114),    p( 847, 1141),    p( 852, 1145),    p( 857, 1166),    p( 858, 1159),    p( 858, 1152),    p( 869, 1136),    p( 867, 1127),
        p( 846, 1101),    p( 857, 1113),    p( 854, 1136),    p( 855, 1131),    p( 859, 1132),    p( 865, 1124),    p( 878, 1103),    p( 868, 1093),
        p( 843, 1100),    p( 853, 1101),    p( 862, 1096),    p( 859, 1106),    p( 860, 1107),    p( 869, 1082),    p( 876, 1059),    p( 873, 1039),
        p( 842, 1090),    p( 836, 1095),    p( 839, 1104),    p( 854, 1092),    p( 848, 1095),    p( 836, 1086),    p( 841, 1072),    p( 843, 1067),
    ],
    // king
    [
        p(  81,  -90),    p(  28,  -36),    p(  52,  -28),    p( -35,    5),    p(  -0,   -9),    p(   9,   -2),    p(  58,   -8),    p( 167,  -91),
        p( -33,   -3),    p(  -8,   23),    p( -27,   31),    p(  40,   18),    p(  15,   31),    p(  -5,   44),    p(  30,   35),    p(  16,    6),
        p( -47,   12),    p(  27,   26),    p( -30,   43),    p( -42,   53),    p(  -4,   49),    p(  46,   46),    p(  21,   45),    p( -18,   23),
        p( -30,    6),    p( -44,   32),    p( -64,   50),    p( -87,   61),    p( -85,   62),    p( -60,   54),    p( -62,   45),    p( -91,   27),
        p( -53,    3),    p( -58,   23),    p( -80,   44),    p(-103,   60),    p(-104,   58),    p( -83,   43),    p( -88,   32),    p(-116,   25),
        p( -37,    4),    p( -17,   11),    p( -66,   31),    p( -77,   44),    p( -71,   41),    p( -74,   31),    p( -39,   12),    p( -66,   16),
        p(  23,   -5),    p(  -2,    0),    p( -16,   11),    p( -44,   21),    p( -44,   21),    p( -31,   12),    p(  13,   -7),    p(   4,   -0),
        p( -19,  -26),    p(  30,  -40),    p(  23,  -29),    p( -51,   -4),    p(   6,  -30),    p( -51,   -8),    p(  20,  -35),    p(  10,  -38),
    ],
];

#[rustfmt::skip]
const PASSED_PAWNS: [PhasedScore; NUM_SQUARES] = [
    // passed pawns
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
        p(  36,   75),    p(  34,   73),    p(  33,   72),    p(  47,   50),    p(  33,   53),    p(  25,   57),    p( -28,   81),    p( -29,   83),
        p(  30,  112),    p(  40,  109),    p(  31,   90),    p(  19,   59),    p(  20,   63),    p(   3,   86),    p( -28,   94),    p( -53,  116),
        p(  14,   65),    p(  13,   61),    p(  20,   47),    p(  16,   37),    p(  -2,   38),    p(   9,   47),    p( -13,   64),    p( -14,   67),
        p(   2,   38),    p(  -8,   37),    p( -15,   30),    p( -10,   22),    p( -18,   26),    p( -10,   31),    p( -21,   45),    p( -12,   43),
        p(  -5,   11),    p( -14,   18),    p( -19,   18),    p( -12,    6),    p( -15,   13),    p(  -8,   15),    p( -13,   31),    p(   7,   13),
        p( -12,   10),    p(  -6,   13),    p( -12,   17),    p( -12,    9),    p(   2,    1),    p(   2,    6),    p(   6,   13),    p(   2,   10),
        p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),    p(   0,    0),
];
const BISHOP_PAIR: PhasedScore = p(24, 58);
const ROOK_OPEN_FILE: PhasedScore = p(29, 11);
const ROOK_CLOSED_FILE: PhasedScore = p(-14, -3);
const ROOK_SEMIOPEN_FILE: PhasedScore = p(8, 12);
const KING_OPEN_FILE: PhasedScore = p(-56, -10);
const KING_CLOSED_FILE: PhasedScore = p(16, -14);
const KING_SEMIOPEN_FILE: PhasedScore = p(-13, 8);
const PAWN_SHIELDS: [PhasedScore; NUM_PAWN_SHIELD_CONFIGURATIONS] = [
    p(-50, 10),  /*0b0000*/
    p(-31, 10),  /*0b0001*/
    p(-17, 3),   /*0b0010*/
    p(6, 21),    /*0b0011*/
    p(-11, 4),   /*0b0100*/
    p(-16, -5),  /*0b0101*/
    p(5, 12),    /*0b0110*/
    p(26, -6),   /*0b0111*/
    p(-36, 8),   /*0b1000*/
    p(-32, -15), /*0b1001*/
    p(-19, 6),   /*0b1010*/
    p(13, -7),   /*0b1011*/
    p(-14, 1),   /*0b1100*/
    p(-20, -22), /*0b1101*/
    p(13, 9),    /*0b1110*/
    p(0, 0),     /*0b1111*/
    p(-46, 11),  /*0b10000*/
    p(-9, 6),    /*0b10001*/
    p(-22, -16), /*0b10010*/
    p(4, -12),   /*0b10011*/
    p(-14, -0),  /*0b10100*/
    p(26, 7),    /*0b10101*/
    p(-9, -19),  /*0b10110*/
    p(0, 0),     /*0b10111*/
    p(-23, 40),  /*0b11000*/
    p(6, 8),     /*0b11001*/
    p(13, 18),   /*0b11010*/
    p(0, 0),     /*0b11011*/
    p(9, 8),     /*0b11100*/
    p(0, 0),     /*0b11101*/
    p(0, 0),     /*0b11110*/
    p(0, 0),     /*0b11111*/
    p(-21, 5),   /*0b100000*/
    p(-14, 8),   /*0b100001*/
    p(1, -2),    /*0b100010*/
    p(22, 4),    /*0b100011*/
    p(-30, -25), /*0b100100*/
    p(-22, -39), /*0b100101*/
    p(-8, -5),   /*0b100110*/
    p(0, 0),     /*0b100111*/
    p(-27, -6),  /*0b101000*/
    p(-34, -8),  /*0b101001*/
    p(-3, -10),  /*0b101010*/
    p(0, 0),     /*0b101011*/
    p(-35, -26), /*0b101100*/
    p(0, 0),     /*0b101101*/
    p(0, 0),     /*0b101110*/
    p(0, 0),     /*0b101111*/
    p(-13, 26),  /*0b110000*/
    p(21, 15),   /*0b110001*/
    p(6, -16),   /*0b110010*/
    p(0, 0),     /*0b110011*/
    p(-2, 1),    /*0b110100*/
    p(0, 0),     /*0b110101*/
    p(0, 0),     /*0b110110*/
    p(0, 0),     /*0b110111*/
    p(-11, 30),  /*0b111000*/
    p(0, 0),     /*0b111001*/
    p(0, 0),     /*0b111010*/
    p(0, 0),     /*0b111011*/
    p(0, 0),     /*0b111100*/
    p(0, 0),     /*0b111101*/
    p(0, 0),     /*0b111110*/
    p(15, -21),  /*0b111111*/
    p(-70, 7),   /*0b00*/
    p(-6, -22),  /*0b01*/
    p(20, -9),   /*0b10*/
    p(45, -32),  /*0b11*/
    p(-1, -14),  /*0b100*/
    p(-33, -43), /*0b101*/
    p(54, -52),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(18, -13),  /*0b1000*/
    p(4, -40),   /*0b1001*/
    p(48, -82),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(26, -15),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(24, -21),  /*0b1111*/
    p(-38, 3),   /*0b00*/
    p(0, -16),   /*0b01*/
    p(3, -22),   /*0b10*/
    p(30, -33),  /*0b11*/
    p(-22, -11), /*0b100*/
    p(12, -54),  /*0b101*/
    p(-1, -31),  /*0b110*/
    p(0, 0),     /*0b111*/
    p(-21, -6),  /*0b1000*/
    p(21, -27),  /*0b1001*/
    p(15, -73),  /*0b1010*/
    p(0, 0),     /*0b1011*/
    p(-6, -11),  /*0b1100*/
    p(0, 0),     /*0b1101*/
    p(0, 0),     /*0b1110*/
    p(16, -67),  /*0b1111*/
];
const PAWN_PROTECTION: [PhasedScore; NUM_CHESS_PIECES] =
    [p(17, 17), p(6, 13), p(0, 6), p(6, 8), p(-7, 12), p(-30, 15)];
const PAWN_ATTACKS: [PhasedScore; NUM_CHESS_PIECES] = [
    p(0, 0),
    p(33, 13),
    p(48, 32),
    p(50, -2),
    p(39, -31),
    p(0, 0),
];

// TODO: Differentiate between rooks and kings in front of / behind pawns?

const PIECE_PHASE: [isize; 6] = [0, 1, 1, 2, 4, 0];

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
        let mut score = PhasedScore::default();
        let mut phase = 0;

        for color in Color::iter() {
            let our_pawns = pos.colored_piece_bb(color, Pawn);
            let their_pawns = pos.colored_piece_bb(color.other(), Pawn);

            if pos.colored_piece_bb(color, Bishop).more_than_one_bit_set() {
                score += BISHOP_PAIR;
            }
            // Rooks on (semi)open/closed files (semi-closed files are handled by adjusting the base rook values during tuning)
            let rooks = pos.colored_piece_bb(color, Rook);
            for rook in rooks.ones() {
                match file_openness(rook.file(), our_pawns, their_pawns) {
                    Open => {
                        score += ROOK_OPEN_FILE;
                    }
                    SemiOpen => {
                        score += ROOK_SEMIOPEN_FILE;
                    }
                    SemiClosed => {}
                    Closed => {
                        score += ROOK_CLOSED_FILE;
                    }
                }
            }
            // King on (semi)open/closed file
            let king_square = pos.king_square(color);
            let king_file = king_square.file();
            match file_openness(king_file, our_pawns, their_pawns) {
                Open => {
                    score += KING_OPEN_FILE;
                }
                SemiOpen => {
                    score += KING_SEMIOPEN_FILE;
                }
                SemiClosed => {}
                Closed => {
                    score += KING_CLOSED_FILE;
                }
            }
            score += PAWN_SHIELDS[pawn_shield_idx(our_pawns, king_square, color)];

            for piece in UncoloredChessPiece::pieces() {
                let bb = pos.colored_piece_bb(color, piece);
                for unflipped_square in bb.ones() {
                    let square = unflipped_square.flip_if(color == White);
                    let idx = square.bb_idx();
                    score += PSQTS[piece as usize][idx];
                    phase += PIECE_PHASE[piece as usize];

                    // Passed pawns.
                    if piece == Pawn {
                        let in_front = (A_FILE
                            << (unflipped_square.flip_if(color == Black).bb_idx() + 8))
                            .flip_if(color == Black);
                        let blocking = in_front | in_front.west() | in_front.east();
                        if (in_front & our_pawns).is_zero() && (blocking & their_pawns).is_zero() {
                            score += PASSED_PAWNS[idx];
                        }
                    }
                }
                let pawn_attacks = our_pawns.pawn_attacks(color);
                let protected_by_pawns = pawn_attacks & bb;
                score += PAWN_PROTECTION[piece as usize] * protected_by_pawns.num_ones();
                let attacked_by_pawns = pawn_attacks & pos.colored_piece_bb(color.other(), piece);
                score += PAWN_ATTACKS[piece as usize] * attacked_by_pawns.num_ones();
            }
            score = -score;
        }
        let score = score.taper(phase, 24);
        let tempo = Score(10);
        tempo
            + match pos.active_player() {
                White => score,
                Black => -score,
            }
    }
}
