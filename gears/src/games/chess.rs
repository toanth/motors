use crate::PlayerResult;
use crate::PlayerResult::{Draw, Lose};
use crate::games::chess::Color::{Black, White};
use crate::games::chess::castling::CastleRight::*;
use crate::games::chess::castling::{CastleRight, CastlingFlags};
use crate::games::chess::moves::Move;
use crate::games::chess::pieces::PieceType::*;
use crate::games::chess::pieces::{ColoredPieceType, NUM_CHESS_PIECES, Piece, PieceType};
use crate::games::chess::squares::{
    A_FILE_NUM, B_FILE_NUM, C_FILE_NUM, ChessboardSize, D_FILE_NUM, E_FILE_NUM, F_FILE_NUM, G_FILE_NUM, H_FILE_NUM,
    NUM_SQUARES, Square,
};
use crate::games::chess::unverified::UnverifiedBoard;
use crate::games::chess::zobrist::ZOBRIST_KEYS;
use crate::games::{
    AbstractPieceType, BoardHistory, BoardTrait, CharType, ColorTrait, ColoredPieceTrait, ColoredPieceTypeTrait, DimT,
    NUM_COLORS, PosHash, SettingsTrait, n_fold_repetition,
};
use crate::general::bitboards::chessboard::{Bitboard, KINGS, dark_squares, light_squares};
use crate::general::bitboards::{BitboardTrait, KnownSizeBitboard, RawBitboardTrait, RawStandardBitboard};
use crate::general::board::SelfChecks::CheckFen;
use crate::general::board::Strictness::{Relaxed, Strict};
use crate::general::board::{
    AxesFormat, BBSelect, BitboardBoard, BoardHelpers, NameToPos, PieceTypeOf, Strictness, Symmetry,
    UnverifiedBoardTrait, board_from_name, default_bitboards_from_name, position_fen_part, read_common_fen_part,
    read_two_move_numbers,
};
use crate::general::common::{EntityList, GenericSelect, Res, StaticallyNamedEntity, Tokens, parse_int_from_str};
use crate::general::move_list::InplaceMoveList;
use crate::general::squares::{RectangularCoordinates, SquareColor};
use crate::output::OutputOpts;
use crate::output::text_output::{
    AdaptFormatter, BoardFormatter, DefaultBoardFormatter, board_to_string, display_board_pretty, display_color,
};
use crate::score::PhaseType;
use crate::search::DepthPly;
use anyhow::{anyhow, bail, ensure};
use arbitrary::Arbitrary;
use colored::Color::Red;
use colored::Colorize;
use rand::Rng;
use rand::prelude::IteratorRandom;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::ops::{Index, IndexMut, Not};
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use strum_macros::FromRepr;

pub mod bitbase;
pub mod castling;
mod movegen;
pub mod moves;
mod perft_tests;
pub mod pieces;
pub mod see;
pub mod squares;
mod tablebase;
pub mod unverified;
pub mod upcoming_repetition;
pub mod zobrist;

/// This is the only global variable in the entire program, and it is only set when reading a `setoption name UCI_Chess960`.
/// All it does is determine how castling moves are formatter; in particular, we accept chess960 FENs even without that option.
pub static UCI_CHESS960: AtomicBool = AtomicBool::new(false);

pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

const fn startpos_mailbox() -> [PieceType; NUM_SQUARES] {
    let mut res = [Empty; NUM_SQUARES];
    res[A_FILE_NUM as usize] = Rook;
    res[H_FILE_NUM as usize] = Rook;
    res[B_FILE_NUM as usize] = Knight;
    res[G_FILE_NUM as usize] = Knight;
    res[C_FILE_NUM as usize] = Bishop;
    res[F_FILE_NUM as usize] = Bishop;
    res[D_FILE_NUM as usize] = Queen;
    res[E_FILE_NUM as usize] = King;
    let mut i = 0;
    while i < 8 {
        res[8 + i] = Pawn;
        i += 1;
    }
    i = 0;
    while i < 8 {
        res[64 - 8 + i] = res[i];
        res[64 - 16 + i] = Pawn;
        i += 1;
    }
    res
}

static STARTPOS: Board = {
    let pieces = [
        Bitboard::new(0x00ff_0000_0000_ff00),
        Bitboard::new(0x4200_0000_0000_0042),
        Bitboard::new(0x2400_0000_0000_0024),
        Bitboard::new(0x8100_0000_0000_0081),
        Bitboard::new(0x0800_0000_0000_0008),
        Bitboard::new(0x1000_0000_0000_0010),
    ];
    let colors = [Bitboard::new(0xffff), Bitboard::new(0xffff << (8 * 6))];
    let bbs = BitboardRepr { pieces, colors };
    let threats = Bitboard::new(0x7effff0000000000);
    Board {
        bbs,
        mailbox: startpos_mailbox(),
        threats,
        checkers: Bitboard::new(0),
        pinned: Bitboard::new(0),
        ply: 0,
        ply_100_ctr: 0,
        active: White,
        castling: CastlingFlags::for_startpos(),
        settings: Settings(0),
        ep_square: None,
        hashes: Hashes {
            pawns: PosHash(2269071747976134835),
            nonpawns: [PosHash(14501238155361420356), PosHash(16424626985112491456)],
            knb: PosHash(10966777955972345920),
            total: PosHash(15430246029285706692),
        },
    }
};

#[derive(Debug, Default, Copy, Clone, Arbitrary)]
pub struct Settings(u8);

impl Settings {
    /// Determined whether the current position is a dfrc position, which determines how castling moves are formatted.
    /// Note that if the UCI_Chess960 option is set, castling moves are always formatted in dfrc format.
    fn dfrc_flag() -> u8 {
        1
    }
    /// This is set on finding the letter `q` or `k` in the FEN castling description.
    /// It is unrelated to how castling moves are formatted, and only used to determine how a FEN is formatted
    /// (which never happens in the UCI protocol, but is common in the interactive interface)
    fn shredder_fen_flag() -> u8 {
        2
    }
    fn is_set(self, flag: u8) -> bool {
        self.0 & flag != 0
    }
    fn set_flag(&mut self, flag: u8, value: bool) {
        if value {
            self.0 |= flag;
        } else {
            self.0 &= !flag;
        }
    }
}

impl PartialEq for Settings {
    fn eq(&self, _other: &Self) -> bool {
        true // we want chessboards to compare equal if they represent the same position and ignore formatting
    }
}

impl Eq for Settings {}

pub const MAX_CHESS_MOVES_IN_POS: usize = 300;

// for some reason, Chessboard::MoveList can be ambiguous? This should fix that
pub type MoveList = InplaceMoveList<Board, MAX_CHESS_MOVES_IN_POS>;

impl SettingsTrait for Settings {}

/// White is always the first player, Black is always the second
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default, Hash, Arbitrary, FromRepr)]
#[must_use]
pub enum Color {
    #[default]
    White = 0,
    Black = 1,
}

impl<T> Index<Color> for [T; 2] {
    type Output = T;
    fn index(&self, index: Color) -> &Self::Output {
        &self[index as usize]
    }
}

impl<T> IndexMut<Color> for [T; 2] {
    fn index_mut(&mut self, index: Color) -> &mut Self::Output {
        &mut self[index as usize]
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let str = match self {
            White => "white",
            Black => "black",
        };
        write!(f, "{str}")
    }
}

impl Not for Color {
    type Output = Self;

    fn not(self) -> Self::Output {
        self.other()
    }
}

impl ColorTrait for Color {
    type Board = Board;

    fn second() -> Self {
        Black
    }

    fn to_char(self, _settings: &Settings) -> char {
        match self {
            White => 'w',
            Black => 'b',
        }
    }

    fn name(self, _settings: &<Self::Board as BoardTrait>::Settings) -> &str {
        match self {
            White => "White",
            Black => "Black",
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq, Copy, Clone, Arbitrary)]
struct Hashes {
    pawns: PosHash,
    nonpawns: [PosHash; NUM_COLORS],
    knb: PosHash,
    total: PosHash,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub struct BitboardRepr {
    pub pieces: [Bitboard; NUM_CHESS_PIECES],
    pub colors: [Bitboard; NUM_COLORS],
}

impl BitboardRepr {
    pub fn col_piece_bb(&self, color: Color, piece: PieceType) -> Bitboard {
        self.colors[color] & self.pieces[piece]
    }

    pub fn place_piece(&mut self, sq: Square, color: Color, piece: PieceType) {
        let bb = sq.bb();
        self.pieces[piece] ^= bb;
        self.colors[color] ^= bb;
    }

    // doesn't update the mailbox because that doesn't work for chess960 castling
    pub fn move_piece(&mut self, from: Square, to: Square, color: Color, piece: PieceType) {
        debug_assert_ne!(piece, Empty);
        debug_assert!(self.pieces[piece].is_bit_set_at(from.bb_idx()), "{self:?}",);
        debug_assert!(
            (self.colors[color].is_bit_set_at(from.bb_idx()))
                // in chess960 castling, it's possible that the king has been sent to the rook square,
                // which means the color bit of this square is currently not set
                || (piece == Rook && self.pieces[King].is_bit_set_at(from.bb_idx())) || (piece == King && from.is_backrank()),
            "{self:?}"
        );
        // use ^ instead of | for to merge the from and to bitboards because in chess960 castling,
        // it is possible that from == to or that there's another piece on the target square
        let bb = from.bb() ^ to.bb();
        self.colors[color] ^= bb;
        self.pieces[piece.to_uncolored_idx()] ^= bb;
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub struct Board {
    bbs: BitboardRepr,
    mailbox: [PieceType; NUM_SQUARES],
    threats: Bitboard,
    checkers: Bitboard,
    pinned: Bitboard,
    ply: u32,
    ply_100_ctr: u8,
    active: Color,
    castling: CastlingFlags,
    settings: Settings,
    ep_square: Option<Square>,
    hashes: Hashes,
}

// const _: () = assert!(size_of::<Chessboard>() == 264);

impl Default for Board {
    fn default() -> Self {
        Self::startpos()
    }
}

impl StaticallyNamedEntity for Board {
    fn static_short_name() -> impl Display
    where
        Self: Sized,
    {
        "chess"
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        "Chess".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "A Chess, Chess960 (a.k.a FRC) or DFRC game".to_string()
    }
}

impl BoardTrait for Board {
    type EmptyRes = UnverifiedBoard;
    type RawBitboard = RawStandardBitboard;
    type Settings = Settings;
    type SettingsRef = Settings;
    type Coordinates = Square;
    type Color = Color;
    type Piece = Piece;
    type Move = Move;
    type MoveList = MoveList;
    type Unverified = UnverifiedBoard;

    fn empty_for_settings(settings: Settings) -> UnverifiedBoard {
        UnverifiedBoard(Self {
            bbs: Default::default(),
            mailbox: [Empty; NUM_SQUARES],
            threats: Bitboard::default(),
            checkers: Bitboard::default(),
            pinned: Bitboard::default(),
            ply: 0,
            ply_100_ctr: 0,
            active: White,
            castling: CastlingFlags::default(),
            settings,
            ep_square: None,
            hashes: Hashes::default(),
        })
    }

    fn startpos_for_settings(_: Self::Settings) -> Self {
        debug_assert_eq!(
            STARTPOS,
            Self::from_fen(START_FEN, Strict).expect("Internal error: Couldn't parse startpos fen")
        );
        STARTPOS
    }

    fn from_name(name: &str) -> Res<Self> {
        board_from_name(name).or_else(|err| {
            Self::parse_numbered_startpos(name)
                .map_err(|err2| anyhow!("{err} It's also not a (D)FRC startpos [{err2}]."))
        })
    }

    fn name_to_pos_map() -> EntityList<NameToPos> {
        vec![
            NameToPos::desc(
                "kiwipete",
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                "Widely used test position",
            ),
            NameToPos::desc(
                "lucena",
                "1K1k4/1P6/8/8/8/8/r7/2R5 w - - 0 1",
                "Theoretical endgame position, won for white",
            ),
            NameToPos::desc("philidor", "3k4/R7/7r/2KP4/8/8/8/8 w - - 0 1", "Theoretical endgame position, a draw"),
            NameToPos::desc(
                "lasker-reichhelm",
                "8/k7/3p4/p2P1p2/P2P1P2/8/8/K7 w - - 0 1",
                "Endgame study with corresponding squares, won for white",
            ),
            NameToPos::desc(
                "behting",
                "8/8/7p/3KNN1k/2p4p/8/3P2p1/8 w - - 0 1",
                "Chess study; supposedly difficult for engines. A draw",
            ),
            NameToPos::desc("saavedra", "8/8/1KP5/3r4/8/8/8/k7 w - - 0 1", "Well-known endgame study, won for white"),
            NameToPos::desc(
                "mate_in_1",
                "8/7r/8/K1k5/8/8/4p3/8 b - - 10 11",
                "A very simple test position where white can win in 1",
            ),
            NameToPos::desc(
                "draw_in_1",
                "2B2k2/8/8/5B2/8/8/8/KR6 w - - 99 123",
                "A test position where every move leads to a draw",
            ),
            NameToPos::desc(
                "unusual",
                "2kb1b2/pR2P1P1/P1N1P3/1p2Pp2/P5P1/1N6/4P2B/2qR2K1 w - f6 99 123",
                "A test position for move generation",
            ),
            NameToPos::desc(
                "see_win_pawn",
                "k6q/3n1n2/3b4/2P1p3/3P1P2/3N1NP1/8/1K6 w - - 0 1",
                "A test position for SEE",
            ),
            NameToPos::desc("see_xray", "5q1k/8/8/8/RRQ2nrr/8/8/K7 w - - 0 1", "Another test position for SEE"),
            NameToPos::desc(
                "zugzwang",
                "6Q1/8/8/7k/8/8/3p1pp1/3Kbrrb w - - 26 14",
                "A test position for NMP; mate in 4",
            ),
            NameToPos::desc(
                "puzzle",
                "rk6/p1r3p1/P3B1Kp/1p2B3/8/8/8/8 w - - 0 1",
                "A somewhat difficult position, mate in 5",
            ),
            // still very difficult for caps-lite to solve
            NameToPos::desc(
                "mate_in_16",
                "1r1q1r2/5pk1/p2p1Np1/2pBp2p/1p2P2P/2PP2P1/1P1Q4/2K2R1b w - - 0 29",
                "Another somewhat difficult position, won for white",
            ),
            NameToPos {
                name: "futile",
                fen: "1k6/1p1p1p1p/pPpPpPpP/P1P1P1P1/1B1R3B/1R3RR1/1B2R3/1K1R1RB1 w - - 18 10",
                description: Some(
                    "A completely unrealistic position that's a draw despite crushing material advantage",
                ),
                strictness: Relaxed,
            },
            NameToPos::desc(
                "captures",
                "r1n1n1b1/1P1P1P1P/1N1N1N2/2RnQrRq/2pKp3/3BNQbQ/k7/4Bq2 w - - 0 1",
                "An unrealistic position with very many captures, can cause qsearch to explode in some engines",
            ),
            NameToPos::desc(
                "check",
                "3rB2k/2P1P3/1NQ1QN2/2Q1QN2/2Q1Q3/1R3R2/3K1B2/8 w - - 0 1",
                "An unrealistic position where white has many moves despite being in check",
            ),
            NameToPos {
                name: "many_moves",
                fen: "QQQQQQBk/Q5RB/Q6Q/Q6Q/Q6Q/Q6Q/Q6Q/KQQQQQQQ w - - 0 1",
                description: Some("Relaxed position with the maximum possible number of legal moves, 271"),
                strictness: Relaxed,
            },
            NameToPos::desc(
                "long_check_sequence",
                "4r1Q1/B2nr3/5b2/8/4p3/4KbNq/ppppppp1/RR3Nkn w - - 0 1",
                "Position with the longest known sequence of checks, 37 plies in a row",
            ),
        ]
    }

    fn bench_positions() -> impl IntoIterator<Item = Self> {
        let fens = &[
            // fens from Stormphrax, possibly ultimately from bitgenie(?), with some new fens.
            "r3k2r/2pb1ppp/2pp1q2/p7/1nP1B3/1P2P3/P2N1PPP/R2QK2R w KQkq - 0 14",
            "4rrk1/2p1b1p1/p1p3q1/4p3/2P2n1p/1P1NR2P/PB3PP1/3R1QK1 b - - 2 24",
            "r3qbrk/6p1/2b2pPp/p3pP1Q/PpPpP2P/3P1B2/2PB3K/R5R1 w - - 16 42",
            "6k1/1R3p2/6p1/2Bp3p/3P2q1/P7/1P2rQ1K/5R2 b - - 4 44",
            "8/8/1p2k1p1/3p3p/1p1P1P1P/1P2PK2/8/8 w - - 3 54",
            "7r/2p3k1/1p1p1qp1/1P1Bp3/p1P2r1P/P7/4R3/Q4RK1 w - - 0 36",
            "r1bq1rk1/pp2b1pp/n1pp1n2/3P1p2/2P1p3/2N1P2N/PP2BPPP/R1BQ1RK1 b - - 2 10",
            "3r3k/2r4p/1p1b3q/p4P2/P2Pp3/1B2P3/3BQ1RP/6K1 w - - 3 87",
            "2r4r/1p4k1/1Pnp4/3Qb1pq/8/4BpPp/5P2/2RR1BK1 w - - 0 42",
            "4q1bk/6b1/7p/p1p4p/PNPpP2P/KN4P1/3Q4/4R3 b - - 0 37",
            "2q3r1/1r2pk2/pp3pp1/2pP3p/P1Pb1BbP/1P4Q1/R3NPP1/4R1K1 w - - 2 34",
            "1r2r2k/1b4q1/pp5p/2pPp1p1/P3Pn2/1P1B1Q1P/2R3P1/4BR1K b - - 1 37",
            "r3kbbr/pp1n1p1P/3ppnp1/q5N1/1P1pP3/P1N1B3/2P1QP2/R3KB1R b KQkq - 0 17",
            "8/6pk/2b1Rp2/3r4/1R1B2PP/P5K1/8/2r5 b - - 16 42",
            "1r4k1/4ppb1/2n1b1qp/pB4p1/1n1BP1P1/7P/2PNQPK1/3RN3 w - - 8 29",
            "8/p2B4/PkP5/4p1pK/4Pb1p/5P2/8/8 w - - 29 68",
            "3r4/ppq1ppkp/4bnp1/2pN4/2P1P3/1P4P1/PQ3PBP/R4K2 b - - 2 20",
            "5rr1/4n2k/4q2P/P1P2n2/3B1p2/4pP2/2N1P3/1RR1K2Q w - - 1 49",
            "1r5k/2pq2p1/3p3p/p1pP4/4QP2/PP1R3P/6PK/8 w - - 1 51",
            "q5k1/5ppp/1r3bn1/1B6/P1N2P2/BQ2P1P1/5K1P/8 b - - 2 34",
            "r1b2k1r/5n2/p4q2/1ppn1Pp1/3pp1p1/NP2P3/P1PPBK2/1RQN2R1 w - - 0 22",
            "r1bqk2r/pppp1ppp/5n2/4b3/4P3/P1N5/1PP2PPP/R1BQKB1R w KQkq - 0 5",
            "r1bqr1k1/pp1p1ppp/2p5/8/3N1Q2/P2BB3/1PP2PPP/R3K2n b Q - 1 12",
            "r1bq2k1/p4r1p/1pp2pp1/3p4/1P1B3Q/P2B1N2/2P3PP/4R1K1 b - - 2 19",
            "r4qk1/6r1/1p4p1/2ppBbN1/1p5Q/P7/2P3PP/5RK1 w - - 2 25",
            "r7/6k1/1p6/2pp1p2/7Q/8/p1P2K1P/8 w - - 0 32",
            "r3k2r/ppp1pp1p/2nqb1pn/3p4/4P3/2PP4/PP1NBPPP/R2QK1NR w KQkq - 1 5",
            "3r1rk1/1pp1pn1p/p1n1q1p1/3p4/Q3P3/2P5/PP1NBPPP/4RRK1 w - - 0 12",
            "5rk1/1pp1pn1p/p3Brp1/8/1n6/5N2/PP3PPP/2R2RK1 w - - 2 20",
            "8/1p2pk1p/p1p1r1p1/3n4/8/5R2/PP3PPP/4R1K1 b - - 3 27",
            "8/4pk2/1p1r2p1/p1p4p/Pn5P/3R4/1P3PP1/4RK2 w - - 1 33",
            "8/5k2/1pnrp1p1/p1p4p/P6P/4R1PK/1P3P2/4R3 b - - 1 38",
            "8/8/1p1kp1p1/p1pr1n1p/P6P/1R4P1/1P3PK1/1R6 b - - 15 45",
            "8/8/1p1k2p1/p1prp2p/P2n3P/6P1/1P1R1PK1/4R3 b - - 5 49",
            "8/8/1p4p1/p1p2k1p/P2npP1P/4K1P1/1P6/3R4 w - - 6 54",
            "8/8/1p4p1/p1p2k1p/P2n1P1P/4K1P1/1P6/6R1 b - - 6 59",
            "8/5k2/1p4p1/p1pK3p/P2n1P1P/6P1/1P6/4R3 b - - 14 63",
            "8/1R6/1p1K1kp1/p6p/P1p2P1P/6P1/1Pn5/8 w - - 0 67",
            "1rb1rn1k/p3q1bp/2p3p1/2p1p3/2P1P2N/PP1RQNP1/1B3P2/4R1K1 b - - 4 23",
            "4rrk1/pp1n1pp1/q5p1/P1pP4/2n3P1/7P/1P3PB1/R1BQ1RK1 w - - 3 22",
            "r2qr1k1/pb1nbppp/1pn1p3/2ppP3/3P4/2PB1NN1/PP3PPP/R1BQR1K1 w - - 4 12",
            "2r2k2/8/4P1R1/1p6/8/P4K1N/7b/2B5 b - - 0 55",
            "6k1/5pp1/8/2bKP2P/2P5/p4PNb/B7/8 b - - 1 44",
            "2rqr1k1/1p3p1p/p2p2p1/P1nPb3/2B1P3/5P2/1PQ2NPP/R1R4K w - - 3 25",
            "r1b2rk1/p1q1ppbp/6p1/2Q5/8/4BP2/PPP3PP/2KR1B1R b - - 2 14",
            "6r1/5k2/p1b1r2p/1pB1p1p1/1Pp3PP/2P1R1K1/2P2P2/3R4 w - - 1 36",
            "rnbqkb1r/pppppppp/5n2/8/2PP4/8/PP2PPPP/RNBQKBNR b KQkq - 0 2",
            "2rr2k1/1p4bp/p1q1p1p1/4Pp1n/2PB4/1PN3P1/P3Q2P/2RR2K1 w - f6 0 20",
            "3br1k1/p1pn3p/1p3n2/5pNq/2P1p3/1PN3PP/P2Q1PB1/4R1K1 w - - 0 23",
            "2r2b2/5p2/5k2/p1r1pP2/P2pB3/1P3P2/K1P3R1/7R w - - 23 93",
            "1r1r2k1/1p2qp1p/6p1/p1QB1b2/5Pn1/N1R1P1P1/PP5P/R1B3K1 b - - 4 23",
            "2bk2rq/2p1pprp/2p1n3/p2pPQ2/N2P4/4RN1P/PPP2RP1/6K1 w - - 5 24",
            "2r2rk1/1p3pbp/p3ppp1/8/8/1P2N1P1/1PPP2PP/2KR3R w - - 42 42",
            "7r/pBrkqQ1p/3b4/5b2/8/6P1/PP2PP1P/R1BR2K1 w - - 1 17", // mate in 2
            "k7/3B4/4N3/K7/8/8/8/8 w - - 16 9",                     // KNBvK
            // maximum number of legal moves (and mate in one)
            "R6R/3Q4/1Q4Q1/4Q3/2Q4Q/Q4Q2/pp1Q4/kBNN1KB1 w - - 0 1",
            // the same position with flipped side to move has no legal moves
            "R6R/3Q4/1Q4Q1/4Q3/2Q4Q/Q4Q2/pp1Q4/kBNN1KB1 b - - 0 1",
            // caused an assertion failure once and is a chess960 FEN
            "nrb1nkrq/2pp1ppp/p4b2/1p2p3/P4B2/3P4/1PP1PPPP/NR1BNRKQ w gb - 0 9",
            // a very weird position (not reachable from startpos, but still somewhat realistic)
            "RNBQKBNR/PPPPPPPP/8/8/8/8/pppppppp/rnbqkbnr w - - 0 1",
            // mate in 15 that stronger engines tend to miss (even lichess SF only finds a mate in 17 with max parameters)
            "5k2/1p5Q/p2r1qp1/P1p1RpN1/2P5/3P3P/5PP1/6K1 b - - 0 56",
            // the next 2 positions have the exact same zobrist hash (thanks to analog hors for the python script to find them)
            "1Q2Q3/N2NP1K1/Rn2B3/qQr3n1/1n5N/1P6/4n3/BkN4q w - - 0 1",
            "2n5/1Rp1K1pn/q6Q/1rrr4/k3Br2/7B/1n1N2Q1/1Nn2R2 w - - 0 1",
        ];
        let res = fens.iter().map(|fen| Self::from_fen(fen, Strict).unwrap());
        let other = Self::name_to_pos_map().into_iter().filter(|e| e.strictness == Strict).map(|e| e.create::<Board>());
        res.chain(other)
    }

    fn random_pos(rng: &mut impl Rng, strictness: Strictness, symmetry: Option<Symmetry>) -> Res<Self> {
        loop {
            // The probability of the unverified position being legal should be decently large,
            // so this rejection sampling approach shouldn't be too slow in practice
            let pos = UnverifiedBoard::random_unverified_pos(rng, strictness, symmetry);
            if let Ok(pos) = pos.verify(strictness) {
                return Ok(pos);
            }
        }
    }

    fn settings(&self) -> &Self::Settings {
        &self.settings
    }

    fn settings_ref(&self) -> Self::SettingsRef {
        self.settings
    }

    fn active_player(&self) -> Color {
        self.active
    }

    fn halfmove_ctr_since_start(&self) -> usize {
        self.ply as usize
    }

    /// When no pawn move or capture has been played for 100 ply, the game is a draw
    /// (This is not entirely accurate to the FIDE rules, which require a player to claim this draw, and only force a draw
    /// after 150 ply. But it's the common ruleset for all engine games). Note that castling moves are irreversible
    /// (i.e. there are no possible repetitions before/after a castling move) without resetting this clock.
    fn ply_draw_clock(&self) -> usize {
        self.ply_100_ctr as usize
    }

    fn valid_squares_bb(&self) -> Self::RawBitboard {
        !0
    }

    fn size(&self) -> ChessboardSize {
        ChessboardSize::default()
    }

    fn is_empty(&self, square: Square) -> bool {
        self.mailbox[square] == Empty
    }

    fn is_piece_on(&self, coords: Square, piece: ColoredPieceType) -> bool {
        if let Some(color) = piece.color() {
            self.col_piece_bb(color, piece.uncolor()).is_bit_set_at(coords.bb_idx())
        } else {
            self.is_empty(coords)
        }
    }

    fn colored_piece_on(&self, square: Self::Coordinates) -> Self::Piece {
        let idx = square.bb_idx();
        let uncolored = self.piece_type_on(square);
        let color = if self.player_bb(Black).is_bit_set_at(idx) {
            Black
        } else {
            White // use white as color for `Empty` because that's what `new` expects
        };
        let typ = ColoredPieceType::new(color, uncolored);
        Piece::new(typ, square)
    }

    fn piece_type_on(&self, square: Square) -> PieceType {
        self.mailbox[square]
    }

    // doesn't return pawn pushes and castling moves
    fn attacks_of(&self, sq: Square) -> RawStandardBitboard {
        self.attacks_of_impl(sq).raw()
    }

    fn default_perft_depth(&self) -> DepthPly {
        DepthPly::new(6)
    }

    fn gen_pseudolegal(&self, mut callback: impl FnMut(Move)) {
        self.gen_pseudolegal_moves::<false>(&mut callback, !self.player_bb(self.active))
    }

    fn gen_tactical_pseudolegal(&self, mut callback: impl FnMut(Move)) {
        self.gen_pseudolegal_moves::<true>(&mut callback, self.player_bb(self.active.other()))
    }

    fn has_no_legal_moves(&self) -> bool {
        let us = self.active;
        let king = self.king_sq(us);
        if !self.is_in_check()
            && (KINGS[king].intersects(!(self.player_bb(us) | self.threats))
                || (self.col_piece_bb(us, Pawn) & !self.pinned).pawn_advance(us).intersects(!self.occupied_bb()))
        {
            // Happy path: There's a square we can move our king to, or we're not in check and can push a non-pinned pawn.
            // So we have at least one legal move and can avoid doing movegen.
            // In most positions where we're not in check, one of these conditions should be true
            false
        } else {
            self.num_legal_moves() == 0
        }
    }

    fn random_legal_move<T: Rng>(&self, rng: &mut T) -> Option<Self::Move> {
        let moves = self.legal_moves_slow();
        moves.into_iter().choose(rng)
    }

    fn random_pseudolegal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
        let moves = self.pseudolegal_moves();
        moves.into_iter().choose(rng)
    }

    fn make_move(self, mov: Self::Move) -> Option<Self> {
        debug_assert!(self.is_move_pseudolegal_impl(mov), "{self} {mov:?}");
        if !self.is_pseudolegal_move_legal(mov) {
            return None;
        }
        Some(self.make_move_impl(mov))
    }

    fn make_nullmove(mut self) -> Option<Self> {
        if self.checkers.has_any() {
            return None;
        }
        // nullmoves count as noisy. This also prevents detecting repetition to before the nullmove
        self.ply_100_ctr = 0;
        if let Some(sq) = self.ep_square {
            self.hashes.total ^= ZOBRIST_KEYS.ep_file_keys[sq.file() as usize];
            self.ep_square = None;
        }
        self.hashes.total ^= ZOBRIST_KEYS.side_to_move_key;
        self.flip_side_to_move();
        // no need to set checkers and pinned; we're not in check and pinned pieces haven't changed
        Some(self)
    }

    fn is_generated_move_pseudolegal(&self, mov: Move) -> bool {
        self.is_move_pseudolegal_impl(mov)
    }

    fn is_move_pseudolegal(&self, mov: Move) -> bool {
        self.is_move_pseudolegal_impl(mov)
    }

    fn is_pseudolegal_move_legal(&self, mov: Self::Move) -> bool {
        self.is_pseudolegal_legal_impl(mov)
    }

    fn player_result_no_movegen<H: BoardHistory>(&self, history: &H) -> Option<PlayerResult> {
        if self.is_50mr_draw() || self.has_insufficient_material() || self.is_3fold_repetition(history) {
            return Some(Draw);
        }
        None
    }

    fn player_result_slow<H: BoardHistory>(&self, history: &H) -> Option<PlayerResult> {
        if let Some(res) = self.player_result_no_movegen(history) {
            return Some(res);
        }
        let no_moves = self.legal_moves_slow().is_empty();
        if no_moves { self.no_moves_result() } else { None }
    }

    #[inline]
    fn no_moves_result(&self) -> Option<PlayerResult> {
        Some(if self.is_in_check() { Lose } else { Draw })
    }

    /// Doesn't quite conform to FIDE rules, but probably mostly agrees with USCF rules (in that it should almost never
    /// return `false` if there is a realistic way to win).
    fn can_reasonably_win(&self, player: Color) -> bool {
        if self.player_bb(player).is_single_piece() {
            return false; // we only have our king left
        }
        if (self.piece_bb(Pawn) | self.col_piece_bb(player, Rook) | self.col_piece_bb(player, Queen)).has_any()
            || self.col_piece_bb(player.other(), King).intersects(CORNER_SQUARES)
        {
            // return true if the opponent has pawns because that can create possibilities to force them
            // to restrict the king's mobility
            return true;
        }
        let bishops = self.col_piece_bb(player, Bishop);
        // we have at most two knights and no other pieces
        if bishops.is_zero() && self.col_piece_bb(player, Knight).num_ones() <= 2 {
            // this can very rarely be incorrect because a mate with a knight is possible even without pawns
            // and even if the king is not in the corner, but those cases are extremely rare
            return false;
        }
        if self.col_piece_bb(player, Knight).is_zero()
            && ((bishops & light_squares()).is_zero() || (bishops & dark_squares()).is_zero())
        {
            return false;
        }
        true
    }

    fn hash_pos(&self) -> PosHash {
        self.hashes.total
    }

    fn read_fen_and_advance_input_for(words: &mut Tokens, strictness: Strictness, settings: Settings) -> Res<Self> {
        let mut board = Board::empty_for_settings(settings);
        if strictness == Strict && words.peek().copied().unwrap_or_default().contains(|c: char| !c.is_ascii()) {
            bail!("Position description contains a non-ASCII character; this is not allowed in strict mode");
        }
        read_common_fen_part::<Board>(words, &mut board)?;
        let color = board.0.active_player();
        let castling_word = words.next().or((strictness == Relaxed).then_some("-")).ok_or_else(|| {
            anyhow!("FEN ends after color to move, missing castling rights, which are required in strict mode")
        })?;
        let castling_rights =
            CastlingFlags::default().parse_castling_rights(castling_word, &mut board.0, strictness)?;
        board.0.active = color;
        board.0.castling = castling_rights;

        match words.next() {
            Some(ep_sq) => {
                if ep_sq != "-" {
                    // will be checked later in `verify_with_level`, because doing so can require movegen
                    board.0.ep_square =
                        Some(Square::from_str(ep_sq).map_err(|e| anyhow!("Couldn't parse e.p. square: {e}"))?);
                }
            }
            None => {
                if strictness == Strict {
                    bail!(
                        "FEN ends after castling rights, missing the en passant square. This is required in strict mode"
                    );
                }
            }
        };
        read_two_move_numbers::<Board>(words, &mut board, strictness)?;
        // also sets the zobrist hash
        board.verify_with_level(CheckFen, strictness)
    }

    fn axes_format(&self) -> AxesFormat {
        AxesFormat::player_pov()
    }

    fn as_diagram(&self, typ: CharType, flip: bool, mark_active: bool) -> String {
        board_to_string(self, Piece::to_char, typ, flip, mark_active)
    }

    fn display_pretty(&self, display_coordinates: &mut dyn BoardFormatter<Self>) -> String {
        display_board_pretty(self, display_coordinates)
    }

    fn pretty_formatter(
        &self,
        piece_to_char: Option<CharType>,
        last_move: Option<Move>,
        opts: OutputOpts,
    ) -> Box<dyn BoardFormatter<Self>> {
        let pos = *self;
        let king_square = self.king_sq(self.active);
        let color_frame =
            Box::new(move |square, col| if pos.is_in_check() && square == king_square { Some(Red) } else { col });
        Box::new(AdaptFormatter {
            underlying: Box::new(DefaultBoardFormatter::new(*self, piece_to_char, last_move, opts)),
            color_frame,
            display_piece: Box::new(move |square, width, _default| {
                let piece = pos.colored_piece_on(square);
                if piece.is_empty() {
                    if square.square_color() == SquareColor::White {
                        " ".repeat(width)
                    } else {
                        // call .dimmed() after formatting the width because crossterm seems to count the dimming escape sequences
                        format!("{:^1$}", "*", width).dimmed().to_string()
                    }
                } else {
                    let c = if piece_to_char.unwrap_or(CharType::Ascii) == CharType::Ascii {
                        piece.to_char(CharType::Ascii, pos.settings())
                    } else {
                        // uncolored because some fonts have trouble with black pawns, and some make white pieces hard to see
                        piece.uncolored().to_char(CharType::Unicode, pos.settings())
                    };
                    let s = format!("{c:^width$}");
                    s.color(display_color(piece.color().unwrap())).to_string()
                }
            }),
            horizontal_spacer_interval: None,
            vertical_spacer_interval: None,
            square_width: None,
        })
    }

    fn background_color(&self, square: Square) -> SquareColor {
        square.square_color()
    }

    fn bitboard_from_name(&self) -> BBSelect<Self> {
        let mut res = default_bitboards_from_name(self);
        res.push(GenericSelect::full(
            "threats",
            Some("attacked"),
            "Bitboard of squares attacked by the inactive player",
            self.threats.raw(),
        ));
        res.push(GenericSelect::full("checkers", None, "Bitboard of pieces giving check", self.checkers.raw()));
        res.push(GenericSelect::full("pinned", None, "Bitboard of pinned pieces", self.pinned.raw()));
        res.push(GenericSelect::full(
            "e_p_square",
            None,
            "Bitboard of squares where an ep capture is possible",
            self.ep_square.map(|sq| sq.bb()).unwrap_or_default().raw(),
        ));
        res.push(GenericSelect::full("light_squares", None, "Bitboard of light squares", light_squares().raw()));
        res.push(GenericSelect::full("dark_squares", None, "Bitboard of dark squares", dark_squares().raw()));
        res
    }
}

impl BitboardBoard for Board {
    type Bitboard = Bitboard;

    fn piece_bb(&self, piece: PieceTypeOf<Self>) -> Self::Bitboard {
        debug_assert_ne!(piece, Empty);
        self.bbs.pieces[piece]
    }

    fn player_bb(&self, color: Self::Color) -> Self::Bitboard {
        self.bbs.colors[color]
    }

    fn empty_bb(&self) -> Self::Bitboard {
        // no need to mask with a bitboard of valid squares because each of the 64 bits corresponds to a square
        !self.occupied_bb()
    }

    fn mask_bb(&self) -> Self::Bitboard {
        Bitboard::new(!0)
    }

    fn calc_move_dest_bb(&self) -> Self::Bitboard {
        let us = self.active;
        self.calc_threats_of(us) | self.pawn_advance_dests()
    }
}

impl Board {
    fn remove_piece_impl(&mut self, square: Square, piece: PieceType, color: Color) {
        debug_assert_eq!(self.colored_piece_on(square), Piece::new(ColoredPieceType::new(color, piece), square));
        let bb = square.bb();
        self.bbs.pieces[piece] ^= bb;
        self.bbs.colors[color] ^= bb;
        self.mailbox[square] = Empty;
    }

    pub fn pawn_key(&self) -> PosHash {
        self.hashes.pawns
    }

    pub fn minor_key(&self) -> PosHash {
        self.hashes.knb
    }

    pub fn nonpawn_key(&self, color: Color) -> PosHash {
        self.hashes.nonpawns[color]
    }

    /// A mate that happens on the 100 move rule counter reaching 100 takes precedence.
    /// This barely ever happens, which is why we can afford the slow operation of checking for a checkmate in that case.
    pub fn is_50mr_draw(&self) -> bool {
        self.ply_100_ctr >= 100 && !self.is_checkmate_slow()
    }

    /// Note that this function doesn't encode FIDE rules, which are more complicated than "position repeats again => draw",
    /// and are also not used in engine chess
    pub fn is_3fold_repetition<H: BoardHistory>(&self, history: &H) -> bool {
        // There's no need to test if the repetition is a checkmate, because checkmate positions can't repeat
        n_fold_repetition(3, history, self.hash_pos(), self.ply_draw_clock())
    }

    /// Check if the current position is a checkmate.
    /// This requires calculating all legal moves and seeing if the side to move is in check.
    pub fn is_stalemate_slow(&self) -> bool {
        !self.is_in_check() && self.has_no_legal_moves()
    }

    /// Check if the current position is a checkmate.
    /// This requires calculating all legal moves and seeing if the side to move is in check.
    pub fn is_checkmate_slow(&self) -> bool {
        // test `is_in_check()` first because it's faster and a precondition for generating legal moves
        self.is_in_check() && self.has_no_legal_moves()
    }

    pub fn has_insufficient_material(&self) -> bool {
        if self.piece_bb(Pawn).has_any() {
            return false;
        }
        if (self.piece_bb(Queen) | self.piece_bb(Rook)).has_any() {
            return false;
        }
        let bishops = self.piece_bb(Bishop);
        if bishops.intersects(dark_squares()) && bishops.intersects(light_squares()) {
            return false; // opposite-colored bishops (even if they belong to different players)
        }
        if bishops.has_any() && self.piece_bb(Knight).has_any() {
            return false; // knight and bishop, or knight vs bishop
        }
        // a knight and any additional uncolored piece can create a mate (non-knight pieces have already been ruled out)
        if self.piece_bb(Knight).num_ones() >= 2 {
            return false;
        }
        true
    }

    pub fn ep_square(&self) -> Option<Square> {
        self.ep_square
    }

    pub fn king_sq(&self, color: Color) -> Square {
        Square::from_bb_idx(self.col_piece_bb(color, King).num_trailing_zeros())
    }

    pub fn is_in_check(&self) -> bool {
        self.checkers.has_any()
    }

    pub fn gives_check(&self, mov: Move) -> bool {
        self.make_move(mov).is_some_and(|b| b.is_in_check())
    }

    fn chess960_startpos_white(mut num: usize, color: Color, mut board: UnverifiedBoard) -> Res<UnverifiedBoard> {
        ensure!(
            num < 960,
            "There are only 960 starting positions in chess960 (0 to 959), so position {num} doesn't exist"
        );
        assert!(board.0.player_bb(color).is_zero());
        assert_eq!(board.0.occupied_bb().raw() & 0xffff, 0);
        let mut extract_factor = |i: usize| {
            let res = num % i;
            num /= i;
            res
        };
        let ith_zero = |i: usize, bb: Bitboard| {
            let mut i = i as isize;
            let bb = bb.raw();
            let mut idx = 0;
            while i >= 0 {
                if bb & (1 << idx) == 0 {
                    i -= 1;
                }
                idx += 1;
            }
            idx - 1
        };
        let mut place_piece = |i: usize, typ: PieceType| {
            let bit = ith_zero(i, board.0.occupied_bb());
            board.place_piece(Square::from_bb_idx(bit), ColoredPieceType::new(White, typ));
            bit
        };

        let wsq_bishop = extract_factor(4) * 2 + 1;
        let mut bsq_bishop = extract_factor(4) * 2;
        if bsq_bishop > wsq_bishop {
            bsq_bishop -= 1;
        }
        _ = place_piece(wsq_bishop, Bishop);
        _ = place_piece(bsq_bishop, Bishop);
        let queen = extract_factor(6);
        _ = place_piece(queen, Queen);
        assert!(num < 10);
        if num < 4 {
            _ = place_piece(0, Knight);
            _ = place_piece(num, Knight);
        } else if num < 7 {
            _ = place_piece(1, Knight);
            _ = place_piece(num - 4 + 1, Knight);
        } else if num < 9 {
            _ = place_piece(2, Knight);
            _ = place_piece(num - 7 + 2, Knight);
        } else {
            _ = place_piece(3, Knight);
            _ = place_piece(3, Knight);
        }
        let q_rook = place_piece(0, Rook);
        _ = place_piece(0, King);
        let k_rook = place_piece(0, Rook);
        for _ in 0..8 {
            _ = place_piece(0, Pawn);
        }

        let mut settings = board.0.settings;
        board.castling_rights_mut().set_castle_right(color, Queenside, q_rook as DimT, &mut settings).unwrap();
        board.castling_rights_mut().set_castle_right(color, Kingside, k_rook as DimT, &mut settings).unwrap();
        board.0.settings = settings;
        Ok(board)
    }

    /// Loads the given Chess960 startpos using [Scharnagl's method](<https://en.wikipedia.org/wiki/Fischer_random_chess_numbering_scheme>).
    pub fn chess_960_startpos(num: usize) -> Res<Self> {
        Self::dfrc_startpos(num, num)
    }

    pub fn dfrc_startpos(white_num: usize, black_num: usize) -> Res<Self> {
        let mut res = Self::empty();
        res = Self::chess960_startpos_white(black_num, Black, res)?;
        for bb in &mut res.0.bbs.pieces {
            *bb = bb.flip_up_down();
        }
        res.0.bbs.colors[Black] = res.0.player_bb(White).flip_up_down();
        res.0.bbs.colors[White] = Bitboard::default();
        for i in 0..8 {
            res.0.mailbox[64 - 8 + i] = res.0.mailbox[i];
            res.0.mailbox[i] = Empty;
            res.0.mailbox[64 - 16 + i] = Pawn;
            res.0.mailbox[8 + i] = Empty;
        }
        res = Self::chess960_startpos_white(white_num, White, res)?;
        res.0.settings.set_flag(Settings::dfrc_flag(), true);
        res.0.settings.set_flag(Settings::shredder_fen_flag(), true);
        // the hash and other metadata is computed in the `verify` method
        Ok(res
            .verify(Strict)
            .expect("Internal error: Setting up a Chess960 starting position resulted in an invalid position"))
    }

    /// Loads a DFRC startpos by setting white's startpos as `num / 960` and black's startpos as `num % 960`.
    pub fn dfrc_startpos_from_single_num(num: usize) -> Res<Self> {
        Self::dfrc_startpos(num / 960, num % 960)
    }

    fn parse_numbered_startpos(name: &str) -> Res<Self> {
        for prefix in ["chess960-", "chess", "frc-", "frc"] {
            if let Some(remaining) = name.strip_prefix(prefix) {
                return parse_int_from_str(remaining, "chess960 startpos number").and_then(Self::chess_960_startpos);
            }
        }
        for prefix in ["dfrc-", "dfrc"] {
            if let Some(remaining) = name.strip_prefix(prefix) {
                return parse_int_from_str(remaining, "dfrc startpos number")
                    .and_then(|num: usize| Self::dfrc_startpos_from_single_num(num));
            }
        }
        bail!(
            "(D)FRC positions must be of the format {0} or {1}, with N < 960 and M < 921600, e.g. frc123",
            "frc<N>".bold(),
            "dfrc<M>".bold()
        )
    }

    /// The startpos has a phase of 24, but larger values are possible
    pub fn phase(&self) -> PhaseType {
        let mut phase = 0;
        for piece in PieceType::non_king_pieces() {
            phase += self.piece_bb(piece).num_ones() as isize * CHESS_PIECE_PHASE[piece as usize];
        }
        phase
    }
}

/// Includes a phase for the empty piece to simplify the implementation
pub const CHESS_PIECE_PHASE: [PhaseType; NUM_CHESS_PIECES + 1] = [0, 1, 1, 2, 4, 0, 0];

impl Display for Board {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        position_fen_part(f, self)?;
        if self.active.is_first() {
            write!(f, " w ")?;
        } else {
            write!(f, " b ")?;
        }
        self.castling.write_castle_rights(f, self)?;
        // we are tracking the ep square correctly, including not setting it when the ep move would be illegal pseudolegal
        if let Some(square) = self.ep_square {
            write!(f, " {square} ")?;
        } else {
            write!(f, " - ")?;
        }
        write!(f, "{0} {1}", self.ply_100_ctr, self.fullmove_ctr_1_based())
    }
}

#[derive(Debug, Copy, Clone)]
#[must_use]
pub enum SliderMove {
    Bishop,
    Rook,
}

pub trait ChessBitboardTrait: KnownSizeBitboard<RawStandardBitboard, Square> {
    fn pawn_ranks() -> Self {
        Self::from_raw(0x00ff_0000_0000_ff00)
    }

    fn pawn_advance(self, color: Color) -> Self {
        match color {
            White => self.north(),
            Black => self.south(),
        }
    }

    // For attacks of a single pawn, there's a precomputed table
    fn pawn_attacks(self, color: Color) -> Self {
        let advanced = self.pawn_advance(color);
        advanced.east() | advanced.west()
    }
}

#[must_use]
const fn precompute_single_pawn_capture(color: Color, square_idx: usize) -> u64 {
    let pawn = 1 << square_idx;
    let not_a_file = pawn & !Bitboard::A_FILE.0;
    let not_h_file = pawn & !(Bitboard::A_FILE.0 << 7);
    match color {
        White => (not_a_file << 7) | (not_h_file << 9),
        Black => (not_a_file >> 9) | (not_h_file >> 7),
    }
}

pub const PAWN_CAPTURES: [[Bitboard; 64]; 2] = {
    let mut res = [[Bitboard::new(0); 64]; 2];
    let mut i = 0;
    while i < 64 {
        res[White as usize][i] = Bitboard::new(precompute_single_pawn_capture(White, i));
        res[Black as usize][i] = Bitboard::new(precompute_single_pawn_capture(Black, i));
        i += 1;
    }
    res
};

impl ChessBitboardTrait for Bitboard {}

pub const CORNER_SQUARES: Bitboard = Bitboard::new(0x8100_0000_0000_0081);

pub const EDGE_SQUARES: Bitboard = Bitboard::new(0xff818181818181ff);

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use rand::rng;
    use std::collections::HashSet;
    use strum::IntoEnumIterator;

    use crate::games::chess::squares::{B_FILE_NUM, E_FILE_NUM, F_FILE_NUM, G_FILE_NUM, H_FILE_NUM};
    use crate::games::{BoardHistDyn, CoordinatesTrait, NoHistory, ZobristHistory, char_to_file};
    use crate::general::board::RectangularBoard;
    use crate::general::board::Strictness::Relaxed;
    use crate::general::moves::MoveTrait;
    use crate::general::perft::Bulkness::{Bulk, NoBulk};
    use crate::general::perft::perft;
    use crate::search::DepthPly;

    use super::*;

    const E_1: Square = Square::from_rank_file(0, E_FILE_NUM);
    const E_8: Square = Square::from_rank_file(7, E_FILE_NUM);

    #[test]
    fn empty_test() {
        let board = Board::empty();
        assert_eq!(board.0.num_squares(), 64);
        assert_eq!(board.0.size(), ChessboardSize::default());
        assert_eq!(board.0.width(), 8);
        assert_eq!(board.0.height(), 8);
        assert_eq!(board.0.halfmove_ctr_since_start(), 0);
        assert_eq!(board.0.fullmove_ctr_0_based(), 0);
        assert!(board.verify(Relaxed).is_err());
    }

    #[test]
    fn startpos_test() {
        let board = Board::default();
        assert_eq!(board.num_squares(), 64);
        assert_eq!(board.size(), ChessboardSize::default());
        assert_eq!(board.width(), 8);
        assert_eq!(board.height(), 8);
        assert_eq!(board.halfmove_ctr_since_start(), 0);
        assert_eq!(board.fullmove_ctr_1_based(), 1);
        assert_eq!(board.ply, 0);
        assert_eq!(board.ply_100_ctr, 0);
        assert!(board.ep_square.is_none());
        assert_eq!(board.active_player(), White);
        for color in Color::iter() {
            for side in CastleRight::iter() {
                assert!(board.castling.can_castle(color, side));
            }
        }
        assert!(!board.is_in_check());
        assert!(!board.is_stalemate_slow());
        assert!(!board.is_3fold_repetition(&ZobristHistory::default()));
        assert!(!board.has_insufficient_material());
        assert!(!board.is_50mr_draw());
        assert_eq!(board.player_bb(White), Bitboard::from_raw(0xffff));
        assert_eq!(board.player_bb(Black), Bitboard::from_raw(0xffff_0000_0000_0000));
        assert_eq!(board.occupied_bb(), Bitboard::from_raw(0xffff_0000_0000_ffff));
        assert_eq!(board.king_sq(White), E_1);
        assert_eq!(board.king_sq(Black), E_8);
        let square = Square::from_rank_file(4, F_FILE_NUM);
        assert_eq!(board.colored_piece_on(square), Piece::new(ColoredPieceType::Empty, square));
        assert_eq!(board.as_fen(), START_FEN);
        let moves = board.pseudolegal_moves();
        assert_eq!(moves.len(), 20);
        let legal_moves = board.legal_moves_slow();
        assert_eq!(legal_moves.len(), moves.len());
        assert!(legal_moves.into_iter().sorted().eq(moves.into_iter().sorted()));
    }

    #[test]
    fn invalid_fen_test() {
        // some of these FENs have been found through cargo fuzz
        let fens = &[
            "",
            "3Ss9999999999999999999999999999999",
            "½",
            "QQQQQQQQw`",
            "q0018446744073709551615",
            "QQQQKQQQ\nwV0 \n",
            "kQQQQQDDw-W0w",
            "2rr2k1/1p4bp/p1q1pqp1/4Pp1n/2PB4/1PN3P1/P3Q2P/2Rr2K1 w - f6 0 20",
            "7r/8/8/8/8/1k4P1/1K6/8 w - - 3 3",
        ];
        for fen in fens {
            let pos = Board::from_fen(fen, Relaxed);
            assert!(pos.is_err());
        }
        // TODO: Fens that parse as Relaxed but not strict
    }

    #[test]
    fn simple_fen_test() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w Qk - 0 1";
        let board = Board::from_fen(fen, Strict).unwrap();
        assert!(!board.castling.can_castle(White, Kingside));
        assert!(board.castling.can_castle(White, Queenside));
        assert!(board.castling.can_castle(Black, Kingside));
        assert!(!board.castling.can_castle(Black, Queenside));
        let fens = [
            "8/8/8/3K4/8/8/5k2/8 w - - 0 1",
            "K7/R7/R7/R7/R7/R7/P7/k7 w - - 0 1",
            "QQKBnknn/8/8/8/8/8/8/8 w - - 0 1",
            "b5k1/b3Q3/3Q1Q2/5Q2/K1bQ1Qb1/2bbbbb1/6Q1/3QQ2b b - - 0 1",
            "rnbq1bn1/pppppp1p/8/K7/5k2/8/PPPP1PPP/RNBQ1BNR w - - 0 1",
            &Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w HhAa - 0 1", Strict).unwrap().as_fen(),
            "rnbqkbnr/1ppppppp/p7/8/8/8/PPPPPPP1/RNBQKBN1 w Ah - 0 1",
            "rnbqkbnr/1ppppppp/p7/8/3pP3/8/PPPP1PP1/RNBQKBN1 b Ah e3 3 1",
            // chess960 fens (from webperft):
            "1rqbkrbn/1ppppp1p/1n6/p1N3p1/8/2P4P/PP1PPPP1/1RQBKRBN w FBfb - 0 9",
            "rbbqn1kr/pp2p1pp/6n1/2pp1p2/2P4P/P7/BP1PPPP1/R1BQNNKR w HAha - 1 42",
            "rqbbknr1/1ppp2pp/p5n1/4pp2/P7/1PP5/1Q1PPPPP/R1BBKNRN w GAga - 42 9",
        ];
        for fen in fens {
            let board = Board::from_fen(fen, Relaxed).unwrap();
            assert_eq!(fen, board.as_fen());
            assert_eq!(board, Board::from_fen(&board.as_fen(), Relaxed).unwrap());
        }
    }

    #[test]
    fn invalid_castle_right_test() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w AQk - 0 1";
        let board = Board::from_fen(fen, Relaxed);
        assert!(board.is_err());
    }

    #[test]
    fn failed_fuzz_test() {
        let pos =
            Board::from_fen("r2k3r/ppp1pp1p/2nqb1Nn/3P4/4P3/2PP4/PR1NBPPP/R2NKRQ1 w KQkq - 1 5", Relaxed).unwrap();
        _ = pos.debug_verify_invariants(Relaxed).unwrap();
        for mov in pos.legal_moves_slow() {
            let new_pos = pos.make_move(mov).unwrap_or(pos);
            _ = new_pos.debug_verify_invariants(Relaxed).unwrap();
        }
        let mov = Move::from_text("sB3x", &pos);
        assert!(mov.is_err());
    }

    #[test]
    fn weird_fen_test() {
        // invalid ep square set
        let fen = "1nbqkbnr/ppp1pppp/8/r2pP3/6K1/8/PPPP1PPP/RNBQ1BNR w k d6 0 2";
        assert!(Board::from_fen(fen, Strict).is_err());
        let board = Board::from_fen(fen, Relaxed).unwrap();
        assert_eq!(board.as_fen(), "1nbqkbnr/ppp1pppp/8/r2pP3/6K1/8/PPPP1PPP/RNBQ1BNR w k - 0 2");
        let fen = "1nbqkbnr/ppppppp1/6r1/6Pp/6K1/8/PPPP1PPP/RNBQ1BNR w k h6 0 2";
        assert!(Board::from_fen(fen, Strict).is_err());
        let board = Board::from_fen(fen, Relaxed).unwrap();
        assert_eq!(board.as_fen(), "1nbqkbnr/ppppppp1/6r1/6Pp/6K1/8/PPPP1PPP/RNBQ1BNR w k - 0 2");
        let fen = "1nbqkbnr/pppppppp/8/r5Pp/6K1/8/PPPP1PPP/RNBQ1BNR w k h6 0 2";
        assert!(Board::from_fen(fen, Relaxed).is_err());
        let fen = "1nbqkbnr/ppppppp1/8/r5Pp/6K1/8/PPPP1PPP/RNBQ1BNR w k - 0 2";
        assert!(Board::from_fen(fen, Strict).is_ok());
        let fen = "♜♞♝♛♚♝♞♜/♟♟♟♟♟♟♟♟/8/8/4♙3/8/♙♙♙♙1♙♙♙/♖♘♗♕♔♗♘♖ b KQkq - 0 1";
        assert_eq!(Board::from_fen(fen, Relaxed).unwrap(), Board::startpos().make_move_from_str("e4").unwrap());
        assert!(Board::from_fen(fen, Strict).is_err());
    }

    #[test]
    fn many_moves_test() {
        let fen = "QQQQQQBk/Q6B/Q6Q/Q6Q/Q6Q/Q6Q/Q6Q/KQQQQQQQ w - - 0 1";
        assert!(Board::from_fen(fen, Strict).is_err());
        let board = Board::from_fen(fen, Relaxed).unwrap();
        let moves = board.pseudolegal_moves();
        assert_eq!(moves.len(), 265);
        let perft_res = perft(DepthPly::new(1), board, false, NoBulk);
        assert_eq!(perft_res.nodes, 265);
    }

    #[test]
    fn simple_perft_test() {
        let endgame_fen = "6k1/8/6K1/8/3B1N2/8/8/7R w - - 0 1";
        let board = Board::from_fen(endgame_fen, Relaxed).unwrap();
        let perft_res = perft(DepthPly::new(1), board, false, NoBulk);
        assert_eq!(perft_res.depth, DepthPly::new(1));
        assert_eq!(perft_res.nodes, 5 + 7 + 13 + 14);
        assert!(perft_res.time.as_millis() <= 10);
        let board = Board::default();
        let perft_res = perft(DepthPly::new(1), board, true, Bulk);
        assert_eq!(perft_res.depth, DepthPly::new(1));
        assert_eq!(perft_res.nodes, 20);
        assert!(perft_res.time.as_millis() <= 2);
        let perft_res = perft(DepthPly::new(2), board, false, NoBulk);
        assert_eq!(perft_res.depth, DepthPly::new(2));
        assert_eq!(perft_res.nodes, 20 * 20);
        assert!(perft_res.time.as_millis() <= 200);

        let board = Board::from_fen("r1bqkbnr/1pppNppp/p1n5/8/8/8/PPPPPPPP/R1BQKBNR b KQkq - 0 3", Strict).unwrap();
        let perft_res = perft(DepthPly::new(1), board, true, Bulk);
        assert_eq!(perft_res.nodes, 26);
        assert_eq!(perft(DepthPly::new(3), board, true, NoBulk).nodes, 16790);

        let board =
            Board::from_fen("rbbqn1kr/pp2p1pp/6n1/2pp1p2/2P4P/P7/BP1PPPP1/R1BQNNKR w HAha - 0 9", Strict).unwrap();
        let perft_res = perft(DepthPly::new(4), board, false, Bulk);
        assert_eq!(perft_res.nodes, 890_435);

        let board = Board::from_fen("1nbqkbnr/p1p1pppp/8/rP1pP2K/8/8/1PPP1PPP/RNBQ1BNR b k - 0 3", Strict).unwrap();
        let perft_res = perft(DepthPly::new(4), board, true, Bulk);
        assert_eq!(perft_res.nodes, 839_770);

        // DFRC
        let board =
            Board::from_fen("r1q1k1rn/1p1ppp1p/1npb2b1/p1N3p1/8/1BP4P/PP1PPPP1/1RQ1KRBN w BFag - 0 9", Strict).unwrap();
        assert_eq!(perft(DepthPly::new(4), board, false, Bulk).nodes, 1_187_103);
    }

    #[test]
    fn mate_test() {
        let board = Board::from_fen("4k3/8/4K3/8/8/8/8/6R1 w - - 0 1", Strict).unwrap();
        let moves = board.pseudolegal_moves();
        for mov in moves {
            if mov.src_square() == board.king_sq(White) {
                assert_eq!(board.is_pseudolegal_move_legal(mov), mov.dest_square().row() != 6);
            } else {
                assert!(board.is_pseudolegal_move_legal(mov));
            }
            if !board.is_pseudolegal_move_legal(mov) {
                continue;
            }
            let checkmates =
                mov.piece_type(&board) == Rook && mov.dest_square() == Square::from_rank_file(7, G_FILE_NUM);
            assert_eq!(board.is_game_won_after_slow(mov, NoHistory::default()), checkmates);
            let new_board = board.make_move(mov).unwrap();
            assert_eq!(new_board.is_game_lost_slow(&NoHistory::default()), checkmates);
            assert_eq!(new_board.is_checkmate_slow(), checkmates);
            assert!(!board.is_checkmate_slow());
        }
    }

    #[test]
    fn capture_only_test() {
        let board = Board::default();
        assert!(board.tactical_pseudolegal().is_empty());
        let board = Board::from_name("kiwipete").unwrap();
        assert_eq!(board.tactical_pseudolegal().len(), 8);
        let board = Board::from_name("mate_in_1").unwrap();
        let tactical = board.tactical_pseudolegal();
        assert_eq!(tactical.len(), 2);
        for m in tactical {
            assert!(m.is_promotion());
            assert!(!m.is_capture(&board));
            assert_eq!(m.piece_type(&board), Pawn);
            assert!([Queen, Knight].contains(&m.promo_piece()));
        }
    }

    #[test]
    fn fifty_mr_test() {
        let board = Board::from_fen("1r2k3/P5R1/2P5/8/8/8/8/1R1K3R w BHb - 99 51", Strict).unwrap();
        let moves = board.legal_moves_slow();
        assert_eq!(moves.len(), 48);
        let mut mate_ctr = 0;
        let mut draw_ctr = 0;
        let resetting = ["c7", "a7a8Q", "a8N", "a8B", "a8=R", "a7xb8N", ":b8B", "b8:=R", "xb8Q+", "Rb8:+"]
            .into_iter()
            .map(|str| Move::from_text(str, &board).unwrap())
            .collect_vec();
        for m in moves {
            let new_pos = board.make_move(m).unwrap();
            if resetting.contains(&m) {
                assert_eq!(new_pos.ply_draw_clock(), 0);
                if !["b1b8", "a7b8q", "a7b8r"].contains(&m.compact_formatter(&board).to_string().as_str()) {
                    assert!(new_pos.player_result_slow(&NoHistory::default()).is_none(), "{m:?}");
                } else {
                    assert!(new_pos.is_checkmate_slow());
                    mate_ctr += 1;
                }
            } else {
                assert_eq!(new_pos.ply_draw_clock(), 100);
                if new_pos.is_checkmate_slow() {
                    mate_ctr += 1;
                } else {
                    assert!(new_pos.is_50mr_draw());
                    draw_ctr += 1;
                }
            }
        }
        assert_eq!(mate_ctr, 4);
        assert_eq!(draw_ctr, 37);
    }

    #[test]
    fn repetition_test() {
        let mut board = Board::default();
        let new_hash = board.make_nullmove().unwrap().hash_pos();
        let moves = ["g1f3", "g8f6", "f3g1", "f6g8", "g1f3", "g8f6", "f3g1", "f6g8", "e2e4"];
        let mut hist = ZobristHistory::default();
        assert_ne!(new_hash, board.hash_pos());
        for (i, mov) in moves.iter().enumerate() {
            let hash = board.hash_pos();
            assert_eq!(i > 3, n_fold_repetition(2, &hist, hash, board.ply_draw_clock()));
            assert_eq!(i > 7, n_fold_repetition(3, &hist, hash, board.ply_draw_clock()));
            assert_eq!(i == 8, board.player_result_no_movegen(&hist).is_some_and(|r| r == Draw));
            hist.push(hash);
            let mov = Move::from_compact_text(mov, &board).unwrap();
            board = board.make_move(mov).unwrap();
            assert_eq!(
                n_fold_repetition(3, &hist, board.hash_pos(), board.ply_draw_clock()),
                board.is_3fold_repetition(&hist)
            );
            assert_eq!(board.is_3fold_repetition(&hist), board.player_result_no_movegen(&hist).is_some());
        }
        board = Board::from_name("lucena").unwrap();
        assert_eq!(board.active, White);
        let hash = board.hash_pos();
        let moves = ["c1b1", "a2c2", "b1e1", "c2a2", "e1c1"];
        for mov in moves {
            board = board.make_move(Move::from_compact_text(mov, &board).unwrap()).unwrap();
            assert_ne!(board.hash_pos(), hash);
            assert!(!n_fold_repetition(2, &hist, board.hash_pos(), 12345));
        }
        assert_eq!(board.active, Black);
        let board = Board::from_name("kiwipete").unwrap();
        let mut new_pos = board;
        for mov in ["e1d1", "h8h7", "d1e1", "h7h8"] {
            new_pos = new_pos.make_move_from_str(mov).unwrap();
        }
        assert_ne!(new_pos.hash_pos(), board.hash_pos());
    }

    #[test]
    fn checkmate_test() {
        let fen = "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3";
        let pos = Board::from_fen(fen, Strict).unwrap();
        assert_eq!(pos.active, White);
        assert_eq!(pos.ply, 4);
        assert!(pos.debug_verify_invariants(Strict).is_ok());
        assert!(pos.is_in_check());
        assert!(pos.is_in_check_on_square(White, pos.king_sq(White)));
        let moves = pos.pseudolegal_moves();
        assert!(moves.is_empty()); // we don't even generate moves anymore here
        let moves = pos.legal_moves_slow();
        assert!(moves.is_empty());
        assert!(pos.is_checkmate_slow());
        assert_eq!(pos.player_result_slow(&NoHistory::default()), Some(Lose));
        assert!(!pos.is_stalemate_slow());
        assert!(pos.make_nullmove().is_none());
        // this position can be claimed as a draw according to FIDE rules but it's also a mate in 1
        let pos = Board::from_fen("k7/p1P5/1PK5/8/8/8/8/8 w - - 99 51", Strict).unwrap();
        assert!(pos.match_result_slow(&NoHistory::default()).is_none());
        let mut draws = 0;
        let mut wins = 0;
        for mov in pos.legal_moves_slow() {
            let new_pos = pos.make_move(mov).unwrap();
            if let Some(res) = new_pos.player_result_slow(&NoHistory::default()) {
                match res {
                    PlayerResult::Win => {
                        unreachable!("The other player can't win through one of our moves")
                    }
                    Lose => {
                        wins += 1;
                    }
                    Draw => draws += 1,
                }
            }
        }
        assert_eq!(draws, 5);
        assert_eq!(wins, 3);
    }

    #[test]
    fn weird_position_test() {
        // There's a similar test in `motors`
        // This fen is actually a legal chess position
        let fen = "q2k2q1/2nqn2b/1n1P1n1b/2rnr2Q/1NQ1QN1Q/3Q3B/2RQR2B/Q2K2Q1 w - - 0 1";
        let board = Board::from_fen(fen, Strict).unwrap();
        assert_eq!(board.active, White);
        assert_eq!(perft(DepthPly::new(3), board, true, Bulk).nodes, 568_299);
        // not a legal chess position, but the board should support this
        let fen = "RRRRRRRR/RRRRRRRR/BBBBBBBB/BBBBBBBB/QQQQQQQQ/QQQQQQQQ/QPPPPPPP/K6k b - - 0 1";
        assert!(Board::from_fen(fen, Strict).is_err());
        let board = Board::from_fen(fen, Relaxed).unwrap();
        assert!(board.pseudolegal_moves().len() <= 3);
        let mut rng = rng();
        let mov = board.random_legal_move(&mut rng).unwrap();
        let board = board.make_move(mov).unwrap();
        assert_eq!(board.pseudolegal_moves().len(), 2);
        let fen = "B4Q1b/8/8/8/2K3P1/5k2/8/b4RNB b - - 0 1"; // far too many checks, but we still accept it
        assert!(Board::from_fen(fen, Strict).is_err());
        let board = Board::from_fen(fen, Relaxed).unwrap();
        assert!(board.pseudolegal_moves().len() <= 3 + 2 * 6);
        assert_eq!(board.legal_moves_slow().len(), 3);
        // maximum number of legal moves in any position reachable from startpos
        let fen = "R6R/3Q4/1Q4Q1/4Q3/2Q4Q/Q4Q2/pp1Q4/kBNN1KB1 w - - 0 1";
        let board = Board::from_fen(fen, Strict).unwrap();
        assert_eq!(board.legal_moves_slow().len(), 218);
        assert!(board.debug_verify_invariants(Strict).is_ok());
        let board = board.make_nullmove().unwrap();
        assert!(board.legal_moves_slow().is_empty());
        assert_eq!(board.player_result_slow(&NoHistory::default()), Some(Draw));
        // chess960 castling rights encoded using X-FEN
        let fen = "1rbq1krb/ppp1pppp/1n1n4/3p4/3P4/2PN4/PP2PPPP/NRBQ1KRB w KQkq - 3 4";
        let board = Board::from_fen(fen, Relaxed).unwrap();
        assert!(board.debug_verify_invariants(Strict).is_ok());
        assert_eq!(board.num_legal_moves(), 32);
        // Another X-FEN, which is often misinterpreted by engines
        let fen = " rk2rqnb/1b6/2n5/pppppppp/PPPPPP2/B1NQ4/6PP/1K1RR1NB w Kk - 8 14";
        let board = Board::from_fen(fen, Relaxed).unwrap();
        assert_eq!(board.legal_moves_slow().len(), 42);
        assert_eq!(board.castling.rook_start_file(White, Kingside), char_to_file('e'));
        // this is a valid disambiguated X-FEN, but it will still be parsed as Shredder FEN
        let fen = "8/2k5/8/8/8/8/8/RR1K1R1R w KB - 0 1";
        assert!(Board::from_fen(fen, Strict).is_err());
        let board = Board::from_fen(fen, Relaxed).unwrap();
        assert_eq!(board.castling.rook_start_file(White, Kingside), H_FILE_NUM);
        assert_eq!(board.castling.rook_start_file(White, Queenside), B_FILE_NUM);
        assert_eq!(board.as_fen(), "8/2k5/8/8/8/8/8/RR1K1R1R w KB - 0 1");
        // An ep capture is pseudolegal but not legal
        let fen = "1nbqkbnr/ppp1pppp/8/r2pP2K/8/8/PPPP1PPP/RNBQ1BNR w k d6 0 2";
        assert!(Board::from_fen(fen, Strict).is_err());
        let pos = Board::from_fen(fen, Relaxed).unwrap();
        assert!(pos.ep_square.is_none());
        assert_ne!(fen, pos.as_fen());
        // only legal move is to castle
        let fen = "8/8/8/8/4k3/7p/4q2P/6KR w K - 0 1";
        let pos = Board::from_fen(fen, Relaxed).unwrap();
        let moves = pos.legal_moves_slow();
        assert_eq!(moves.len(), 1);
        assert!(moves[0].is_castle());
    }

    #[test]
    fn chess960_startpos_test() {
        let mut fens = HashSet::new();
        let mut startpos_found = false;
        for i in 0..960 {
            let board = Board::chess_960_startpos(i).unwrap();
            assert!(board.debug_verify_invariants(Strict).is_ok());
            assert!(fens.insert(board.as_fen()));
            let num_moves = board.pseudolegal_moves().len();
            assert!((18..=21).contains(&num_moves)); // 21 legal moves because castling can be legal
            assert_eq!(board.castling.allowed_castling_directions(), 0b1111);
            assert_eq!(board.king_sq(White).flip_up_down(board.size()), board.king_sq(Black));
            assert_eq!(board.piece_bb(Pawn).num_ones(), 16);
            assert_eq!(board.piece_bb(Knight).num_ones(), 4);
            assert_eq!(board.piece_bb(Bishop).num_ones(), 4);
            assert_eq!(board.piece_bb(Rook).num_ones(), 4);
            assert_eq!(board.piece_bb(Queen).num_ones(), 2);
            startpos_found |= board == Board::default();
        }
        // castling flags are compared for equality by ignoring the bits that specify the format
        assert!(startpos_found);
        let std = Board::chess_960_startpos(518).unwrap();
        let start = Board::default();
        assert_eq!(std, Board::default(), "{std} vs {start}");
        let pos = Board::chess_960_startpos(100).unwrap();
        assert_eq!(pos.as_fen(), "qbbnrnkr/pppppppp/8/8/8/8/PPPPPPPP/QBBNRNKR w HEhe - 0 1");
    }

    #[test]
    fn ep_test() {
        let fen = "5k2/2p5/8/3P4/1pP5/8/P7/1K6 b - c3 0 1";
        let pos = Board::from_fen(fen, Relaxed).unwrap();
        assert_eq!(pos.ep_square, Some(Square::from_str("c3").unwrap()));
        _ = pos.debug_verify_invariants(Strict).unwrap();
        let new_pos = pos.make_move_from_str("c5").unwrap();
        assert_eq!(new_pos.ep_square, Some(Square::from_str("c6").unwrap()));
        _ = new_pos.debug_verify_invariants(Strict).unwrap();
        let perft = perft(DepthPly::new(4), pos, true, NoBulk);
        assert_eq!(perft.nodes, 5020);
    }

    #[test]
    fn castling_attack_test() {
        let fen = "8/8/8/8/8/8/3♚4/♖♔6 b A - 0 1";
        let pos = Board::from_fen(fen, Relaxed).unwrap();
        let moves = pos.legal_moves_slow();
        // check that castling moves don't count as attacking squares
        assert!(pos.castling.can_castle(White, Queenside));
        assert_eq!(moves.len(), 6);
        let attacking = pos.all_attacking(Square::from_str("d1").unwrap(), pos.slider_generator());
        assert_eq!(attacking.num_ones(), 1);
        let fen = "8/8/8/3k4/8/8/8/1KRn4 w C - 0 1";
        let pos = Board::from_fen(fen, Strict).unwrap();
        assert!(pos.castling.can_castle(White, Kingside));
        assert!(Move::from_extended_text("0-0", &pos).is_err());
    }

    #[test]
    fn insufficient_material_test() {
        let insufficient = [
            "8/4k3/8/8/8/8/8/2K5 w - - 0 1",
            "8/4k3/8/8/8/8/5N2/2K5 w - - 0 1",
            "8/8/8/3b2k1/8/2K5/6b1/8 w - - 0 1",
            "8/8/8/6k1/8/2K5/8/4b1b1 w - - 0 1",
            "8/8/3B4/7k/8/8/1K6/6b1 w - - 0 1",
            "8/6B1/8/6k1/8/2K5/8/6b1 w - - 0 1",
            "3b3B/2B5/1B1B4/B7/3b4/4b2k/5b2/1K6 w - - 0 1",
            "3B3B/2B5/1B1B4/B6k/3B4/4B3/1K3B2/2B5 w - - 0 1",
        ];
        let sufficient = [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "8/8/4k3/8/8/1K6/8/7R w - - 0 1",
            "5r2/3R4/4k3/8/8/1K6/8/8 w - - 0 1",
            "8/8/4k3/8/8/1K6/8/6BB w - - 0 1",
            "8/8/4B3/8/8/7K/8/6bk w - - 0 1",
            "3B3B/2B5/1B1B4/B6k/3B4/4B3/1K3B2/1B6 w - - 0 1",
            "8/3k4/8/8/8/8/NNN5/1K6 w - - 0 1",
        ];
        let sufficient_but_unreasonable = [
            "6B1/8/8/6k1/8/2K5/8/6b1 w - - 0 1",
            "8/8/4B3/8/8/7K/8/6bk b - - 0 1",
            "8/8/4B3/7k/8/8/1K6/6b1 w - - 0 1",
            "8/3k4/8/8/8/8/1NN5/1K6 w - - 0 1",
            "8/2nk4/8/8/8/8/1NN5/1K6 w - - 0 1",
        ];
        for fen in insufficient {
            let board = Board::from_fen(fen, Strict).unwrap();
            assert!(board.has_insufficient_material(), "{fen}");
            assert!(!board.can_reasonably_win(board.active), "{fen}");
            assert!(!board.can_reasonably_win(board.active.other()), "{fen}");
        }
        for fen in sufficient {
            let board = Board::from_fen(fen, Strict).unwrap();
            assert!(!board.has_insufficient_material(), "{fen}");
            assert!(board.can_reasonably_win(board.active), "{fen}");
        }
        for fen in sufficient_but_unreasonable {
            let board = Board::from_fen(fen, Strict).unwrap();
            assert!(!board.has_insufficient_material(), "{fen}");
            assert!(!board.can_reasonably_win(board.active), "{fen}");
        }
    }
}
