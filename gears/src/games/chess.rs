use anyhow::{anyhow, bail};
use arbitrary::Arbitrary;
use colored::Color::Red;
use colored::Colorize;
use itertools::Itertools;
use rand::prelude::IteratorRandom;
use rand::Rng;
use std::fmt::{Display, Formatter};
use std::num::NonZeroUsize;
use std::ops::Not;
use std::str::FromStr;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::games::chess::castling::CastleRight::*;
use crate::games::chess::castling::{CastleRight, CastlingFlags};
use crate::games::chess::moves::ChessMove;
use crate::games::chess::pieces::ChessPieceType::*;
use crate::games::chess::pieces::{
    ChessPiece, ChessPieceType, ColoredChessPieceType, NUM_CHESS_PIECES, NUM_COLORS,
};
use crate::games::chess::squares::{ChessSquare, ChessboardSize};
use crate::games::chess::zobrist::PRECOMPUTED_ZOBRIST_KEYS;
use crate::games::chess::ChessColor::{Black, White};
use crate::games::{
    file_to_char, n_fold_repetition, AbstractPieceType, Board, BoardHistory, Color, ColoredPiece,
    ColoredPieceType, DimT, PieceType, Settings, ZobristHash,
};
use crate::general::bitboards::chess::{
    black_squares, white_squares, ChessBitboard, CORNER_SQUARES,
};
use crate::general::bitboards::{Bitboard, RawBitboard, RawStandardBitboard};
use crate::general::board::SelfChecks::{Assertion, CheckFen};
use crate::general::board::Strictness::Strict;
use crate::general::board::{
    board_from_name, ply_counter_from_fullmove_nr, position_fen_part, read_common_fen_part,
    NameToPos, SelfChecks, Strictness, UnverifiedBoard,
};
use crate::general::common::{
    parse_int_from_str, EntityList, GenericSelect, Res, StaticallyNamedEntity, Tokens,
};
use crate::general::move_list::{EagerNonAllocMoveList, MoveList};
use crate::general::squares::{RectangularCoordinates, SquareColor};
use crate::output::text_output::{
    board_to_string, display_board_pretty, display_color, AdaptFormatter, BoardFormatter,
    DefaultBoardFormatter, PieceToChar,
};
use crate::PlayerResult;
use crate::PlayerResult::{Draw, Lose};

pub mod castling;
mod movegen;
pub mod moves;
mod perft_tests;
pub mod pieces;
pub mod see;
pub mod squares;
pub mod zobrist;

const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w HAha - 0 1";

#[derive(Eq, PartialEq, Copy, Clone, Debug, Default)]
pub struct ChessSettings {}

pub const MAX_CHESS_MOVES_IN_POS: usize = 256;

// for some reason, Chessboard::MoveList can be ambiguous? This should fix that
pub type ChessMoveList = EagerNonAllocMoveList<Chessboard, MAX_CHESS_MOVES_IN_POS>;

impl Settings for ChessSettings {}

/// White is always the first player, Black is always the second
#[derive(
    Copy, Clone, Eq, PartialEq, Debug, Default, Hash, EnumIter, derive_more::Display, Arbitrary,
)]
pub enum ChessColor {
    #[default]
    White = 0,
    Black = 1,
}

impl Not for ChessColor {
    type Output = Self;

    fn not(self) -> Self::Output {
        self.other()
    }
}

impl Color for ChessColor {
    #[must_use]
    fn other(self) -> Self {
        match self {
            White => Black,
            Black => White,
        }
    }

    fn first() -> Self {
        White
    }

    fn second() -> Self {
        Black
    }

    fn ascii_color_char(self) -> char {
        match self {
            White => 'w',
            Black => 'b',
        }
    }
}

#[derive(Eq, PartialEq, Debug, Copy, Clone, Arbitrary)]
pub struct Chessboard {
    piece_bbs: [RawStandardBitboard; NUM_CHESS_PIECES],
    color_bbs: [RawStandardBitboard; NUM_COLORS],
    ply: usize,
    ply_100_ctr: usize,
    active_player: ChessColor,
    castling: CastlingFlags,
    ep_square: Option<ChessSquare>, // eventually, see if using Optional and Noned instead of Option improves nps
    hash: ZobristHash,
}

impl Default for Chessboard {
    fn default() -> Self {
        Self::startpos()
    }
}

impl StaticallyNamedEntity for Chessboard {
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

impl Board for Chessboard {
    type EmptyRes = UnverifiedChessboard;
    type Settings = ChessSettings;
    type Coordinates = ChessSquare;
    type Color = ChessColor;
    type Piece = ChessPiece;
    type Move = ChessMove;
    type MoveList = ChessMoveList;
    type Unverified = UnverifiedChessboard;

    fn empty_for_settings(_: Self::Settings) -> UnverifiedChessboard {
        UnverifiedChessboard(Self {
            piece_bbs: Default::default(),
            color_bbs: Default::default(),
            ply: 0,
            ply_100_ctr: 0,
            active_player: White,
            castling: CastlingFlags::default(),
            ep_square: None,
            hash: ZobristHash(0),
        })
    }

    fn startpos_for_settings(_: Self::Settings) -> Self {
        Self::from_fen(START_FEN, Strict).expect("Internal error: Couldn't parse startpos fen")
    }

    fn from_name(name: &str) -> Res<Self> {
        board_from_name(name).or_else(|err| {
            Self::parse_numbered_startpos(name)
                .map_err(|err2| anyhow!("{err} It's also not a (D)FRC startpos [{err2}]."))
        })
    }

    fn name_to_pos_map() -> EntityList<NameToPos<Self>> {
        vec![
            GenericSelect {
                name: "kiwipete",
                val: || {
                    Self::from_fen(
                        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                        Strict,
                    )
                    .unwrap()
                },
            },
            GenericSelect {
                name: "lucena",
                val: || Self::from_fen("1K1k4/1P6/8/8/8/8/r7/2R5 w - - 0 1", Strict).unwrap(),
            },
            GenericSelect {
                name: "philidor",
                val: || Self::from_fen("3k4/R7/7r/2KP4/8/8/8/8 w - - 0 1", Strict).unwrap(),
            },
            GenericSelect {
                name: "mate_in_1",
                val: || Self::from_fen("8/7r/8/K1k5/8/8/4p3/8 b - - 10 11", Strict).unwrap(),
            },
            GenericSelect {
                name: "draw_in_1",
                val: || Self::from_fen("2B2k2/8/8/5B2/8/8/8/KR6 w - - 99 123", Strict).unwrap(),
            },
            GenericSelect {
                name: "unusual",
                val: || {
                    Self::from_fen(
                        "2kb1b2/pR2P1P1/P1N1P3/1p2Pp2/P5P1/1N6/4P2B/2qR2K1 w - f6 99 123",
                        Strict,
                    )
                    .unwrap()
                },
            },
            GenericSelect {
                name: "see_win_pawn",
                val: || {
                    Self::from_fen("k6q/3n1n2/3b4/2P1p3/3P1P2/3N1NP1/8/1K6 w - - 0 1", Strict)
                        .unwrap()
                },
            },
            GenericSelect {
                name: "see_xray",
                val: || Self::from_fen("5q1k/8/8/8/RRQ2nrr/8/8/K7 w - - 0 1", Strict).unwrap(),
            },
            GenericSelect {
                name: "zugzwang",
                val: || Self::from_fen("6Q1/8/8/7k/8/8/3p1pp1/3Kbrrb w - - 26 14", Strict).unwrap(),
            },
            GenericSelect {
                name: "puzzle",
                val: || {
                    Self::from_fen("rk6/p1r3p1/P3B1Kp/1p2B3/8/8/8/8 w - - 0 1", Strict).unwrap()
                },
            },
            // still very difficult for caps-lite to solve
            GenericSelect {
                name: "mate_in_16",
                val: || {
                    Self::from_fen(
                        "1r1q1r2/5pk1/p2p1Np1/2pBp2p/1p2P2P/2PP2P1/1P1Q4/2K2R1b w - - 0 29",
                        Strict,
                    )
                    .unwrap()
                },
            },
        ]
    }

    fn bench_positions() -> Vec<Self> {
        let fens = [
            // fens from Stormphrax, ultimately from bitgenie, with some new fens.
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
            // mate in 15 that stronger engines tend to miss(even lichess SF only finds a mate in 17 with max parameters)
            "5k2/1p5Q/p2r1qp1/P1p1RpN1/2P5/3P3P/5PP1/6K1 b - - 0 56",
        ];
        fens.map(|fen| Self::from_fen(fen, Strict).unwrap())
            .iter()
            .copied()
            .collect_vec()
    }

    fn settings(&self) -> Self::Settings {
        ChessSettings {}
    }

    fn active_player(&self) -> ChessColor {
        self.active_player
    }

    fn halfmove_ctr_since_start(&self) -> usize {
        self.ply
    }

    fn halfmove_repetition_clock(&self) -> usize {
        self.ply_100_ctr
    }

    fn size(&self) -> ChessboardSize {
        ChessboardSize::default()
    }

    fn is_empty(&self, coords: Self::Coordinates) -> bool {
        self.empty_bb().is_bit_set_at(coords.bb_idx())
    }

    fn is_piece_on(&self, coords: ChessSquare, piece: ColoredChessPieceType) -> bool {
        if let Some(color) = piece.color() {
            self.colored_piece_bb(color, piece.uncolor())
                .is_bit_set_at(coords.bb_idx())
        } else {
            self.is_empty(coords)
        }
    }

    fn colored_piece_on(&self, square: Self::Coordinates) -> Self::Piece {
        let idx = square.bb_idx();
        let uncolored = self.piece_type_on(square);
        let color = if self.colored_bb(Black).is_bit_set_at(idx) {
            Black
        } else {
            White // use white as color for `Empty` because that's what `new` expects
        };
        let typ = ColoredChessPieceType::new(color, uncolored);
        ChessPiece::new(typ, square)
    }

    fn piece_type_on(&self, square: ChessSquare) -> ChessPieceType {
        let idx = square.bb_idx();
        ChessPieceType::from_idx(
            self.piece_bbs
                .iter()
                .position(|bb| bb.is_bit_set_at(idx))
                .unwrap_or(NUM_CHESS_PIECES),
        )
    }

    fn gen_pseudolegal<T: MoveList<Self>>(&self, moves: &mut T) {
        self.gen_pseudolegal_moves(moves, !self.colored_bb(self.active_player), false)
    }

    fn gen_tactical_pseudolegal<T: MoveList<Self>>(&self, moves: &mut T) {
        self.gen_pseudolegal_moves(moves, self.colored_bb(self.active_player.other()), true)
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
        self.make_move_impl(mov, |_hash| ())
    }

    fn make_nullmove(mut self) -> Option<Self> {
        self.ply += 1;
        // nullmoves count as noisy. This also prevents detecting repetition to before the nullmove
        self.ply_100_ctr = 0;
        if let Some(sq) = self.ep_square {
            self.hash ^= PRECOMPUTED_ZOBRIST_KEYS.ep_file_keys[sq.file() as usize];
            self.ep_square = None;
        }
        self.hash ^= PRECOMPUTED_ZOBRIST_KEYS.side_to_move_key;
        self.flip_side_to_move()
    }

    fn is_move_pseudolegal(&self, mov: Self::Move) -> bool {
        self.is_move_pseudolegal_impl(mov)
    }

    fn player_result_no_movegen<H: BoardHistory<Chessboard>>(
        &self,
        history: &H,
    ) -> Option<PlayerResult> {
        if self.is_50mr_draw()
            || self.has_insufficient_material()
            || self.is_3fold_repetition(history)
        {
            return Some(Draw);
        }
        None
    }

    fn player_result_slow<H: BoardHistory<Self>>(&self, history: &H) -> Option<PlayerResult> {
        if let Some(res) = self.player_result_no_movegen(history) {
            return Some(res);
        }
        let no_moves = self.legal_moves_slow().is_empty();
        if no_moves {
            Some(self.no_moves_result())
        } else {
            None
        }
    }

    fn no_moves_result(&self) -> PlayerResult {
        if self.is_in_check() {
            Lose
        } else {
            Draw
        }
    }

    /// Doesn't quite conform to FIDE rules, but probably mostly agrees with USCF rules (in that it should almost never
    /// return `false` if there is a realistic way to win).
    fn can_reasonably_win(&self, player: ChessColor) -> bool {
        if self.colored_bb(player).is_single_piece() {
            return false; // we only have our king left
        }
        // return true if the opponent has pawns because that can create possibilities to force them
        // to restrict the king's mobility
        if (self.piece_bb(Pawn)
            | self.colored_piece_bb(player, Rook)
            | self.colored_piece_bb(player, Queen))
        .has_set_bit()
            || (self.colored_piece_bb(player.other(), King) & CORNER_SQUARES).has_set_bit()
        {
            return true;
        }
        // we have at most two knights and no other pieces
        if self.colored_piece_bb(player, Bishop).is_zero()
            && self.colored_piece_bb(player, Knight).num_ones() <= 2
        {
            // this can very rarely be incorrect because a mate with a knight is possible even without pawns
            // and even if the king is not in the corner, but those cases are extremely rare
            return false;
        }
        let bishops = self.colored_piece_bb(player, Bishop);
        if self.colored_piece_bb(player, Knight).is_zero()
            && ((bishops & white_squares()).is_zero() || (bishops & black_squares()).is_zero())
        {
            return false;
        }
        true
    }

    fn zobrist_hash(&self) -> ZobristHash {
        self.hash
    }

    fn as_fen(&self) -> String {
        let res = position_fen_part(self);
        let mut castle_rights = String::default();
        // Always output chess960 castling rights. FEN output isn't necessary for UCI
        // and almost all tools support chess960 FEN notation.
        for color in ChessColor::iter() {
            for side in CastleRight::iter().rev() {
                if self.castling.can_castle(color, side) {
                    let mut file = file_to_char(self.castling.rook_start_file(color, side));
                    if color == White {
                        file = file.to_ascii_uppercase();
                    }
                    castle_rights.push(file);
                }
            }
        }
        if castle_rights.is_empty() {
            castle_rights += "-";
        }
        let mut ep_square = "-".to_string();
        if let Some(square) = self.ep_square() {
            // Internally, the ep square is set whenever a pseudolegal ep move is possible, but the FEN standard requires
            // the ep square to be set only iff there is a legal ep move possible. So we check for that when outputting
            // the FEN (printing the FEN should not be performance critical).
            if self.legal_moves_slow().iter().any(|m| m.is_ep()) {
                ep_square = square.to_string();
            }
        }

        let stm = match self.active_player {
            White => 'w',
            Black => 'b',
        };
        res + &format!(
            " {stm} {castle_rights} {ep_square} {halfmove_clock} {move_number}",
            halfmove_clock = self.ply_100_ctr,
            move_number = self.fullmove_ctr_1_based()
        )
    }

    fn read_fen_and_advance_input(words: &mut Tokens, strictness: Strictness) -> Res<Self> {
        let mut board = Chessboard::empty();
        board = read_common_fen_part::<Chessboard>(words, board)?;
        let color = board.0.active_player();
        let Some(castling_word) = words.next() else {
            bail!("FEN ends after color to move, missing castling rights")
        };
        let castling_rights =
            CastlingFlags::default().parse_castling_rights(castling_word, &board.0, strictness)?;

        let Some(ep_square) = words.next() else {
            bail!("FEN ends after castling rights, missing en passant square")
        };
        let ep_square = if ep_square == "-" {
            None
        } else {
            let square = ChessSquare::from_str(ep_square)?;
            let ep_capturing = square.bb().pawn_advance(!color);
            let ep_capturing = ep_capturing.west() | ep_capturing.east();
            // The current FEN standard disallows giving an ep square unless a pawn can legally capture.
            // This library instead uses pseudolegal ep captures, but some existing programs give fens that contain an
            // ep square after every double pawn push, so we silently ignore those invalid ep squares unless in strict mode.
            if (board.0.colored_piece_bb(color, Pawn) & ep_capturing).is_zero() {
                if strictness == Strict {
                    bail!("The ep square is set to {ep_square} even though no pawn can recapture. In strict mode, this is not allowed")
                }
                None
            } else {
                Some(square)
            }
        };
        board = board.set_ep(ep_square);
        let halfmove_clock = words.next().unwrap_or("");
        // Some FENs don't contain the halfmove clock and fullmove number, so assume that's the case if parsing
        // the halfmove clock fails -- but don't do this for the fullmove number.
        if let Ok(halfmove_clock) = halfmove_clock.parse::<usize>() {
            board = board.set_halfmove_repetition_clock(halfmove_clock);
            let Some(fullmove_number) = words.next() else {
                bail!(
                    "The FEN contains a valid halfmove clock ('{halfmove_clock}') but no fullmove counter",
                )
            };
            let fullmove_number = fullmove_number.parse::<NonZeroUsize>().map_err(|err| {
                anyhow!(
                    "Couldn't parse fullmove counter '{}': {err}",
                    fullmove_number.red()
                )
            })?;
            board.0.ply = ply_counter_from_fullmove_nr::<Chessboard>(fullmove_number, color);
        } else if strictness == Strict {
            bail!("FEN doesn't contain a halfmove clock and fullmove counter, but they are required in strict mode")
        } else {
            board.0.ply_100_ctr = 0;
            board.0.ply = usize::from(color == Black);
        }
        board.0.active_player = color;
        board.0.castling = castling_rights;
        // also sets the zobrist hash
        board.verify_with_level(CheckFen, strictness)
    }

    fn should_flip_visually() -> bool {
        true
    }

    fn as_ascii_diagram(&self, flip: bool) -> String {
        board_to_string(self, ChessPiece::to_ascii_char, flip)
    }

    fn as_unicode_diagram(&self, flip: bool) -> String {
        board_to_string(self, ChessPiece::to_utf8_char, flip)
    }

    fn display_pretty(&self, display_coordinates: &mut dyn BoardFormatter<Self>) -> String {
        display_board_pretty(self, display_coordinates)
    }

    fn pretty_formatter(
        &self,
        piece_to_char: Option<PieceToChar>,
        last_move: Option<ChessMove>,
    ) -> Box<dyn BoardFormatter<Self>> {
        let pos = *self;
        let king_square = self.king_square(self.active_player);
        let color_frame = Box::new(move |square, col| {
            if pos.is_in_check() && square == king_square {
                Some(Red)
            } else {
                col
            }
        });
        Box::new(AdaptFormatter {
            underlying: Box::new(DefaultBoardFormatter::new(*self, piece_to_char, last_move)),
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
                    let c = if piece_to_char.unwrap_or(PieceToChar::Ascii) == PieceToChar::Ascii {
                        piece.to_ascii_char()
                    } else {
                        // uncolored because some fonts have trouble with black pawns, and some make white pieces hard to see
                        piece.uncolored().to_utf8_char()
                    };
                    let s = format!("{c:^0$}", width);
                    s.color(display_color(piece.color().unwrap())).to_string()
                }
            }),
            horizontal_spacer_interval: None,
            vertical_spacer_interval: None,
            square_width: None,
        })
    }

    fn background_color(&self, square: ChessSquare) -> SquareColor {
        square.square_color()
    }
}

impl Chessboard {
    pub fn piece_bb(&self, piece: ChessPieceType) -> ChessBitboard {
        debug_assert_ne!(piece, Empty);
        ChessBitboard::new(self.piece_bbs[piece.to_uncolored_idx()])
    }

    pub fn colored_bb(&self, color: ChessColor) -> ChessBitboard {
        ChessBitboard::new(self.color_bbs[color as usize])
    }

    pub fn active_player_bb(&self) -> ChessBitboard {
        self.colored_bb(self.active_player)
    }

    pub fn inactive_player_bb(&self) -> ChessBitboard {
        self.colored_bb(self.inactive_player())
    }

    pub fn occupied_bb(&self) -> ChessBitboard {
        debug_assert!((self.colored_bb(White) & self.colored_bb(Black)).is_zero());
        self.colored_bb(White) | self.colored_bb(Black)
    }

    pub fn empty_bb(&self) -> ChessBitboard {
        !self.occupied_bb()
    }

    pub fn is_occupied(&self, square: ChessSquare) -> bool {
        self.occupied_bb().is_bit_set_at(square.bb_idx())
    }

    pub fn colored_piece_bb(&self, color: ChessColor, piece: ChessPieceType) -> ChessBitboard {
        self.colored_bb(color) & self.piece_bb(piece)
    }

    fn remove_piece_unchecked(
        &mut self,
        square: ChessSquare,
        piece: ChessPieceType,
        color: ChessColor,
    ) {
        debug_assert_eq!(
            self.colored_piece_on(square),
            ChessPiece::new(ColoredChessPieceType::new(color, piece), square)
        );
        let bb = square.bb().raw();
        self.piece_bbs[piece as usize] ^= bb;
        self.color_bbs[color as usize] ^= bb;
        // It's not really clear how to so handle these flags when removing pieces, so we just unset them on a best effort basis
        if piece == Rook {
            for side in CastleRight::iter() {
                if self.castling.rook_start_file(color, side) == square.file()
                    && square.rank() == 7 * color as DimT
                {
                    self.castling.unset_castle_right(color, side);
                }
            }
        } else if piece == Pawn
            && self
                .ep_square
                .is_some_and(|sq| sq.pawn_advance_unchecked(color) == square)
        {
            self.ep_square = None;
        }
    }

    fn move_piece(&mut self, from: ChessSquare, to: ChessSquare, piece: ChessPieceType) {
        debug_assert_ne!(piece, Empty);
        // for a castling move, the rook has already been moved to the square still occupied by the king
        debug_assert!(
            self.piece_type_on(from) == piece
                || (piece == King && self.piece_type_on(from) == Rook),
            "{}",
            self.piece_type_on(from)
        );
        debug_assert!(
            (self.active_player ==
            self.colored_piece_on(from).color().unwrap())
            // in chess960 castling, it's possible that the rook has been sent to the king square,
            // which means the color bit of the king square is currently not set
            || (piece == King && self.piece_bb(Rook).is_bit_set_at(from.bb_idx())),
            "{self}"
        );
        // with chess960 castling, it's possible to move to the source square or a square occupied by a rook
        debug_assert!(
            self.colored_piece_on(to).color() != self.colored_piece_on(from).color()
                || piece == King
                || piece == Rook
        );
        // use ^ instead of | for to merge the from and to bitboards because in chess960 castling,
        // it is possible that from == to or that there's another piece on the target square
        let bb = (from.bb() ^ to.bb()).raw();
        let color = self.active_player;
        self.color_bbs[color as usize] ^= bb;
        self.piece_bbs[piece.to_uncolored_idx()] ^= bb;
    }

    /// A mate that happens on the 100 move rule counter reaching 100 takes precedence.
    /// This barely every happens, which is why we can afford the slow operation of checking for a checkmate in that case.
    pub fn is_50mr_draw(&self) -> bool {
        self.ply_100_ctr >= 100 && !self.is_checkmate_slow()
    }

    /// Note that this function isn't entire correct according to the FIDE rules because it doesn't check for legality,
    /// so a position with a possible pseudolegal but illegal en passant move would be considered different from
    /// its repetition, where the en passant move wouldn't be possible
    /// TODO: Should there be a `ZobristRepetition3FoldPedanticChess` that actually does movegen?
    /// TODO: Only set the ep square if there are pseudolegal en passants possible
    pub fn is_3fold_repetition<H: BoardHistory<Self>>(&self, history: &H) -> bool {
        // There's no need to test if the repetition is a checkmate, because checkmate positions can't repeat
        n_fold_repetition(3, history, self, self.ply_100_ctr)
    }

    /// Check if the current position is a checkmate.
    /// This requires calculating all legal moves and seeing if the side to move is in check.
    pub fn is_stalemate_slow(&self) -> bool {
        !self.is_in_check() && self.legal_moves_slow().is_empty()
    }

    /// Check if the current position is a checkmate.
    /// This requires calculating all legal moves and seeing if the side to move is in check.
    pub fn is_checkmate_slow(&self) -> bool {
        // test `is_in_check()` first because it's faster and a precondition for generating legal moves
        self.is_in_check() && self.legal_moves_slow().is_empty()
    }

    pub fn has_insufficient_material(&self) -> bool {
        if self.piece_bb(Pawn).has_set_bit() {
            return false;
        }
        if (self.piece_bb(Queen) | self.piece_bb(Rook)).has_set_bit() {
            return false;
        }
        let bishops = self.piece_bb(Bishop);
        if (bishops & black_squares()).has_set_bit() && (bishops & white_squares()).has_set_bit() {
            return false; // opposite-colored bishops (even if they belong to different players)
        }
        if bishops.has_set_bit() && self.piece_bb(Knight).has_set_bit() {
            return false; // knight and bishop, or knight vs bishop
        }
        // a knight and any additional uncolored piece can create a mate (non-knight pieces have already been ruled out)
        if self.piece_bb(Knight).num_ones() >= 2 {
            return false;
        }
        true
    }

    pub fn ep_square(&self) -> Option<ChessSquare> {
        self.ep_square
    }

    pub fn king_square(&self, color: ChessColor) -> ChessSquare {
        ChessSquare::from_bb_index(self.colored_piece_bb(color, King).trailing_zeros())
    }

    pub fn is_in_check(&self) -> bool {
        self.is_in_check_on_square(self.active_player, self.king_square(self.active_player))
    }

    pub fn gives_check(&self, mov: ChessMove) -> bool {
        self.make_move(mov).is_some_and(|b| b.is_in_check())
    }

    fn chess960_startpos_white(
        mut num: usize,
        color: ChessColor,
        mut board: UnverifiedChessboard,
    ) -> Res<UnverifiedChessboard> {
        if num >= 960 {
            bail!("There are only 960 starting positions in chess960 (0 to 959), so position {num} doesn't exist");
        }
        assert!(board.0.colored_bb(color).is_zero());
        assert_eq!((board.0.occupied_bb().raw() & 0xffff), 0);
        let mut extract_factor = |i: usize| {
            let res = num % i;
            num /= i;
            res
        };
        let ith_zero = |i: usize, bb: ChessBitboard| {
            let mut i = i as isize;
            let bb = bb.0;
            let mut idx = 0;
            while i >= 0 {
                if bb & (1 << idx) == 0 {
                    i -= 1;
                }
                idx += 1;
            }
            idx - 1
        };
        let mut place_piece = |i: usize, typ: ChessPieceType| {
            let bit = ith_zero(i, board.0.occupied_bb());
            board = board.place_piece_unchecked(
                ChessSquare::from_bb_index(bit),
                ColoredChessPieceType::new(White, typ),
            );
            bit
        };

        let bsq_bishop = extract_factor(4) * 2;
        let mut wsq_bishop = extract_factor(4) * 2 + 1;
        if wsq_bishop >= bsq_bishop {
            wsq_bishop -= 1;
        }
        place_piece(bsq_bishop, Bishop);
        place_piece(wsq_bishop, Bishop);
        let queen = extract_factor(6);
        place_piece(queen, Queen);
        assert!(num < 10);
        if num < 4 {
            place_piece(0, Knight);
            place_piece(num, Knight);
        } else if num < 7 {
            place_piece(1, Knight);
            place_piece(num - 4 + 1, Knight);
        } else if num < 9 {
            place_piece(2, Knight);
            place_piece(num - 7 + 2, Knight);
        } else {
            place_piece(3, Knight);
            place_piece(3, Knight);
        }
        let q_rook = place_piece(0, Rook);
        place_piece(0, King);
        let k_rook = place_piece(0, Rook);
        for _ in 0..8 {
            place_piece(0, Pawn);
        }

        board
            .castling_rights_mut()
            .set_castle_right(color, Queenside, q_rook as DimT)
            .unwrap();
        board
            .castling_rights_mut()
            .set_castle_right(color, Kingside, k_rook as DimT)
            .unwrap();
        Ok(board)
    }

    pub fn chess_960_startpos(num: usize) -> Res<Self> {
        Self::dfrc_startpos(num, num)
    }

    pub fn dfrc_startpos(white_num: usize, black_num: usize) -> Res<Self> {
        let mut res = Self::empty();
        res = Self::chess960_startpos_white(black_num, Black, res)?;
        for bb in &mut res.0.piece_bbs {
            *bb = ChessBitboard::new(*bb).flip_up_down().raw();
        }
        res.0.color_bbs[Black as usize] = res.0.colored_bb(White).flip_up_down().raw();
        res.0.color_bbs[White as usize] = RawStandardBitboard::default();
        res = Self::chess960_startpos_white(white_num, White, res)?;
        // the hash is computed in the verify method
        Ok(res.verify_with_level(Assertion, Strict).expect("Internal error: Setting up a Chess960 starting position resulted in an invalid position"))
    }

    pub fn dfrc_startpos_from_single_num(num: usize) -> Res<Self> {
        Self::dfrc_startpos(num / 960, num % 960)
    }

    fn parse_numbered_startpos(name: &str) -> Res<Self> {
        for prefix in ["chess960-", "chess", "frc-", "frc"] {
            if let Some(remaining) = name.strip_prefix(prefix) {
                return parse_int_from_str(remaining, "chess960 startpos number")
                    .and_then(Self::chess_960_startpos);
            }
        }
        for prefix in ["dfrc-", "dfrc"] {
            if let Some(remaining) = name.strip_prefix(prefix) {
                return parse_int_from_str(remaining, "dfrc startpos number")
                    .and_then(|num: usize| Self::dfrc_startpos_from_single_num(num));
            }
        }
        bail!("(D)FRC positions must be of the format {0} or {1}, with N < 960 and M < 921600, e.g. frc123",
            "frc<N>".bold(), "dfrc<M>".bold())
    }
}

impl Display for Chessboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{0}", self.as_fen())
    }
}

#[derive(Debug, Copy, Clone)]
#[must_use]
pub struct UnverifiedChessboard(Chessboard);

impl From<Chessboard> for UnverifiedChessboard {
    fn from(board: Chessboard) -> Self {
        Self(board)
    }
}

impl UnverifiedBoard<Chessboard> for UnverifiedChessboard {
    fn verify_with_level(self, checks: SelfChecks, strictness: Strictness) -> Res<Chessboard> {
        let mut this = self.0;
        for color in ChessColor::iter() {
            if !this.colored_piece_bb(color, King).is_single_piece() {
                bail!("The {color} player does not have exactly one king")
            }
            if (this.colored_piece_bb(color, Pawn)
                & (ChessBitboard::rank_no(0) | ChessBitboard::rank_no(7)))
            .has_set_bit()
            {
                bail!("The {color} player has a pawn on the first or eight rank");
            }
        }

        for color in ChessColor::iter() {
            for side in CastleRight::iter() {
                let has_eligible_rook = (this.rook_start_square(color, side).bb()
                    & this.colored_piece_bb(color, Rook))
                .has_set_bit();
                if this.castling.can_castle(color, side) && !has_eligible_rook {
                    bail!(
                        "Color {color} can castle {side}, but there is no rook to castle{}",
                        if checks == CheckFen {
                            " (invalid castling flag in FEN?)"
                        } else {
                            ""
                        }
                    );
                }
            }
        }
        let inactive_player = this.active_player.other();

        if let Some(ep_square) = this.ep_square {
            if ![2, 5].contains(&ep_square.rank()) {
                bail!(
                    "FEN specifies invalid ep square (not on the third or sixth rank): '{ep_square}'"
                );
            }
            let remove_pawn_square = ep_square.pawn_advance_unchecked(inactive_player);
            let pawn_origin_square = ep_square.pawn_advance_unchecked(this.active_player);
            if this.colored_piece_on(remove_pawn_square).symbol
                != ColoredChessPieceType::new(inactive_player, Pawn)
            {
                bail!("FEN specifies en passant square {ep_square}, but there is no {inactive_player}-colored pawn on {remove_pawn_square}");
            } else if !this.is_empty(ep_square) {
                bail!(
                    "The en passant square ({ep_square}) must be empty, but it's occupied by a {}",
                    this.piece_type_on(ep_square).name()
                )
            } else if !this.is_empty(pawn_origin_square) {
                bail!("The en passant square is set to {ep_square}, so the pawn must have come from {pawn_origin_square}. But this square isn't empty")
            }
            let active = this.active_player();
            // In the current version of the FEN standard, the ep square should only be set if a pawn can capture.
            // This implementation follows that rule, but many other implementations give the ep square after every double pawn push.
            // To achieve consistent results, such an incorrect ep square is removed when parsing the FEN in Relaxed mode; it should
            // no longer exist at this point.
            if checks != CheckFen || strictness == Strict {
                let possible_ep_pawns =
                    remove_pawn_square.bb().west() | remove_pawn_square.bb().east();
                if (possible_ep_pawns & this.colored_piece_bb(active, Pawn)).is_zero() {
                    bail!("The en passant square is set to '{ep_square}', but there is no {active}-colored pawn that could capture on that square");
                }
            }
        }

        if this.is_in_check_on_square(inactive_player, this.king_square(inactive_player)) {
            bail!("Player {inactive_player} is in check, but it's not their turn to move");
        } else if strictness == Strict {
            let checkers = this.all_attacking(this.king_square(this.active_player))
                & this.inactive_player_bb();
            let num_attacking = checkers.num_ones();
            if num_attacking > 2 {
                bail!(
                    "{} is in check from {num_attacking} pieces, which is not allowed in strict mode",
                    this.active_player
                )
            }
        }
        // we allow loading FENs where more than one piece gives check to the king in a way that could not have been reached
        // from startpos, e.g. "B6b/8/8/8/2K5/5k2/8/b6B b - - 0 1"
        if this.ply_100_ctr >= 100 {
            bail!(
                "The 50 move rule has been exceeded (there have already been {0} plies played)",
                this.ply_100_ctr
            );
        } else if this.ply >= 100_000 {
            bail!("Ridiculously large ply counter: {0}", this.ply);
        } else if strictness == Strict && this.ply_100_ctr > this.ply {
            bail!("The halfmove repetition clock ({0}) is larger than the number of played half moves ({1}), \
                which is not allowed in strict mode", this.ply_100_ctr, this.ply)
        }

        let mut num_promoted_pawns: [isize; 2] = [0, 0];
        let startpos_piece_count = [8, 2, 2, 2, 1, 1];
        for piece in ColoredChessPieceType::pieces() {
            let color = piece.color().unwrap();
            let bb = this.colored_piece_bb(color, piece.uncolor());
            if bb.num_ones() > 20 {
                // Catch this now to prevent crashes down the line because the move list is too small for made-up invalid positions.
                // (This is lax enough to allow many invalid positions that likely won't lead to a crash)
                bail!(
                    "There are {0} {color} {piece}s in this position. There can never be more than 10 pieces \
                    of the same type in a legal chess position (but this implementation accepts up to 20 in non-strict mode)",
                    bb.num_ones()
                );
            } else if strictness == Strict {
                num_promoted_pawns[color as usize] +=
                    0.max(bb.num_ones() as isize - startpos_piece_count[piece.uncolor() as usize]);
            }
            if checks != CheckFen {
                for other_piece in ColoredChessPieceType::pieces() {
                    if other_piece == piece {
                        continue;
                    }
                    if (bb
                        & this
                            .colored_piece_bb(other_piece.color().unwrap(), other_piece.uncolor()))
                    .has_set_bit()
                    {
                        bail!("There are two pieces on the same square: {piece} and {other_piece}");
                    }
                }
            }
        }
        for color in ChessColor::iter() {
            let num_pawns = this.colored_piece_bb(color, Pawn).num_ones() as isize;
            if strictness == Strict && num_promoted_pawns[color as usize] + num_pawns > 8 {
                bail!("Incorrect piece distribution for {color}")
            }
        }
        this.hash = this.compute_zobrist();
        Ok(this)
    }

    fn size(&self) -> ChessboardSize {
        self.0.size()
    }

    fn place_piece_unchecked(self, square: ChessSquare, piece: ColoredChessPieceType) -> Self {
        let mut this = self.0;
        debug_assert!(self.0.is_empty(square));
        let bb = square.bb().raw();
        this.piece_bbs[piece.uncolor() as usize] ^= bb;
        this.color_bbs[piece.color().unwrap() as usize] ^= bb;
        this.into()
    }

    fn remove_piece_unchecked(mut self, sq: ChessSquare) -> Self {
        let piece = self.0.colored_piece_on(sq);
        self.0
            .remove_piece_unchecked(sq, piece.symbol.uncolor(), piece.color().unwrap());
        self
    }

    fn piece_on(&self, coords: ChessSquare) -> Res<ChessPiece> {
        Ok(self.0.colored_piece_on(self.check_coordinates(coords)?))
    }

    fn set_active_player(mut self, player: ChessColor) -> Self {
        self.0.active_player = player;
        self
    }

    fn set_ply_since_start(mut self, ply: usize) -> Res<Self> {
        self.0.ply = ply;
        Ok(self)
    }
}

impl UnverifiedChessboard {
    pub fn castling_rights_mut(&mut self) -> &mut CastlingFlags {
        &mut self.0.castling
    }

    pub fn set_ep(mut self, ep: Option<ChessSquare>) -> Self {
        self.0.ep_square = ep;
        self
    }

    pub fn set_halfmove_repetition_clock(mut self, ply: usize) -> Self {
        self.0.ply_100_ctr = ply;
        self
    }
}

#[derive(Debug, Copy, Clone)]
pub enum SliderMove {
    Bishop,
    Rook,
}

#[cfg(test)]
mod tests {
    use rand::thread_rng;
    use std::collections::HashSet;

    use crate::games::chess::squares::{E_FILE_NO, F_FILE_NO, G_FILE_NO};
    use crate::games::{Coordinates, NoHistory, RectangularCoordinates, ZobristHistory};
    use crate::general::board::RectangularBoard;
    use crate::general::board::Strictness::Relaxed;
    use crate::general::moves::Move;
    use crate::general::perft::perft;
    use crate::search::Depth;

    use super::*;

    const E_1: ChessSquare = ChessSquare::from_rank_file(0, E_FILE_NO);
    const E_8: ChessSquare = ChessSquare::from_rank_file(7, E_FILE_NO);

    #[test]
    fn empty_test() {
        let board = Chessboard::empty();
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
        let board = Chessboard::default();
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
        for color in ChessColor::iter() {
            for side in CastleRight::iter() {
                assert!(board.castling.can_castle(color, side));
            }
        }
        assert!(!board.is_in_check());
        assert!(!board.is_stalemate_slow());
        assert!(!board.is_3fold_repetition(&ZobristHistory::default()));
        assert!(!board.has_insufficient_material());
        assert!(!board.is_50mr_draw());
        assert_eq!(board.colored_bb(White), ChessBitboard::from_u64(0xffff));
        assert_eq!(
            board.colored_bb(Black),
            ChessBitboard::from_u64(0xffff_0000_0000_0000)
        );
        assert_eq!(
            board.occupied_bb(),
            ChessBitboard::from_u64(0xffff_0000_0000_ffff)
        );
        assert_eq!(board.king_square(White), E_1);
        assert_eq!(board.king_square(Black), E_8);
        let square = ChessSquare::from_rank_file(4, F_FILE_NO);
        assert_eq!(
            board.colored_piece_on(square),
            ChessPiece::new(ColoredChessPieceType::Empty, square)
        );
        assert_eq!(board.as_fen(), START_FEN);
        let moves = board.pseudolegal_moves();
        assert_eq!(moves.len(), 20);
        let legal_moves = board.legal_moves_slow();
        assert_eq!(legal_moves.len(), moves.len());
        assert!(legal_moves
            .into_iter()
            .sorted()
            .eq(moves.into_iter().sorted()));
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
            // TODO: Allow this? Requires larger move list?
            "QQQQQQBk/Q6B/Q6Q/Q6Q/Q6Q/Q6Q/Q6Q/KQQQQQQQ w - - 0 1",
        ];
        for fen in fens {
            let pos = Chessboard::from_fen(fen, Relaxed);
            assert!(pos.is_err());
        }
        // TODO: Fens that parse as Relaxed but not strict
    }

    #[test]
    fn simple_fen_test() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w Qk - 0 1";
        let board = Chessboard::from_fen(fen, Strict).unwrap();
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
            &Chessboard::from_fen(
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w HhAa - 0 1",
                Strict,
            )
            .unwrap()
            .as_fen(),
            "rnbqkbnr/1ppppppp/p7/8/8/8/PPPPPPP1/RNBQKBN1 w Ah - 0 1",
            "rnbqkbnr/1ppppppp/p7/8/3pP3/8/PPPP1PP1/RNBQKBN1 b Ah e3 3 1",
            // chess960 fens (from webperft):
            "1rqbkrbn/1ppppp1p/1n6/p1N3p1/8/2P4P/PP1PPPP1/1RQBKRBN w FBfb - 0 9",
            "rbbqn1kr/pp2p1pp/6n1/2pp1p2/2P4P/P7/BP1PPPP1/R1BQNNKR w HAha - 1 42",
            "rqbbknr1/1ppp2pp/p5n1/4pp2/P7/1PP5/1Q1PPPPP/R1BBKNRN w GAga - 42 9",
        ];
        for fen in fens {
            let board = Chessboard::from_fen(fen, Relaxed).unwrap();
            assert_eq!(fen, board.as_fen());
            assert_eq!(
                board,
                Chessboard::from_fen(&board.as_fen(), Relaxed).unwrap()
            );
        }
    }

    #[test]
    fn invalid_castle_right_test() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w AQk - 0 1";
        let board = Chessboard::from_fen(fen, Relaxed);
        assert!(board.is_err());
    }

    #[test]
    fn failed_fuzz_test() {
        let pos = Chessboard::from_fen(
            "r2k3r/ppp1pp1p/2nqb1Nn/3P4/4P3/2PP4/PR1NBPPP/R2NKRQ1 w KQkq - 1 5",
            Relaxed,
        )
        .unwrap();
        pos.debug_verify_invariants(Relaxed).unwrap();
        for mov in pos.legal_moves_slow() {
            let new_pos = pos.make_move(mov).unwrap_or(pos);
            new_pos.debug_verify_invariants(Relaxed).unwrap();
        }
        let mov = ChessMove::from_text("sB3x", &pos);
        assert!(mov.is_err());
    }

    #[test]
    fn simple_perft_test() {
        let endgame_fen = "6k1/8/6K1/8/3B1N2/8/8/7R w - - 0 1";
        let board = Chessboard::from_fen(endgame_fen, Relaxed).unwrap();
        let perft_res = perft(Depth::new_unchecked(1), board);
        assert_eq!(perft_res.depth, Depth::new_unchecked(1));
        assert_eq!(perft_res.nodes, 5 + 7 + 13 + 14);
        assert!(perft_res.time.as_millis() <= 1);
        let board = Chessboard::default();
        let perft_res = perft(Depth::new_unchecked(1), board);
        assert_eq!(perft_res.depth, Depth::new_unchecked(1));
        assert_eq!(perft_res.nodes, 20);
        assert!(perft_res.time.as_millis() <= 2);
        let perft_res = perft(Depth::new_unchecked(2), board);
        assert_eq!(perft_res.depth, Depth::new_unchecked(2));
        assert_eq!(perft_res.nodes, 20 * 20);
        assert!(perft_res.time.as_millis() <= 20);

        let board = Chessboard::from_fen(
            "r1bqkbnr/1pppNppp/p1n5/8/8/8/PPPPPPPP/R1BQKBNR b KQkq - 0 3",
            Strict,
        )
        .unwrap();
        let perft_res = perft(Depth::new_unchecked(1), board);
        assert_eq!(perft_res.nodes, 26);
        assert_eq!(perft(Depth::new_unchecked(3), board).nodes, 16790);

        let board = Chessboard::from_fen(
            "rbbqn1kr/pp2p1pp/6n1/2pp1p2/2P4P/P7/BP1PPPP1/R1BQNNKR w HAha - 0 9",
            Strict,
        )
        .unwrap();
        let perft_res = perft(Depth::new_unchecked(4), board);
        assert_eq!(perft_res.nodes, 890_435);

        // DFRC
        let board = Chessboard::from_fen(
            "r1q1k1rn/1p1ppp1p/1npb2b1/p1N3p1/8/1BP4P/PP1PPPP1/1RQ1KRBN w BFag - 0 9",
            Strict,
        )
        .unwrap();
        assert_eq!(perft(Depth::new_unchecked(4), board).nodes, 1_187_103);
    }

    #[test]
    fn mate_test() {
        let board = Chessboard::from_fen("4k3/8/4K3/8/8/8/8/6R1 w - - 0 1", Strict).unwrap();
        let moves = board.pseudolegal_moves();
        for mov in moves {
            if mov.src_square() == board.king_square(White) {
                assert_eq!(
                    board.is_pseudolegal_move_legal(mov),
                    mov.dest_square().row() != 6
                );
            } else {
                assert!(board.is_pseudolegal_move_legal(mov));
            }
            if !board.is_pseudolegal_move_legal(mov) {
                continue;
            }
            let checkmates = mov.piece_type() == Rook
                && mov.dest_square() == ChessSquare::from_rank_file(7, G_FILE_NO);
            assert_eq!(board.is_game_won_after_slow(mov), checkmates);
            let new_board = board.make_move(mov).unwrap();
            assert_eq!(new_board.is_game_lost_slow(), checkmates);
            assert!(!board.is_game_lost_slow());
        }
    }

    #[test]
    fn capture_only_test() {
        let board = Chessboard::default();
        assert!(board.tactical_pseudolegal().is_empty());
        let board = Chessboard::from_name("kiwipete").unwrap();
        assert_eq!(board.tactical_pseudolegal().len(), 8);
    }

    #[test]
    fn repetition_test() {
        let mut board = Chessboard::default();
        let new_hash = board.make_nullmove().unwrap().zobrist_hash();
        let moves = [
            "g1f3", "g8f6", "f3g1", "f6g8", "g1f3", "g8f6", "f3g1", "f6g8", "e2e4",
        ];
        let mut hist = ZobristHistory::default();
        assert_ne!(new_hash, board.zobrist_hash());
        for (i, mov) in moves.iter().enumerate() {
            assert_eq!(
                i > 3,
                n_fold_repetition(2, &hist, &board, board.ply_100_ctr)
            );
            assert_eq!(
                i > 7,
                n_fold_repetition(3, &hist, &board, board.ply_100_ctr)
            );
            assert_eq!(
                i == 8,
                board
                    .player_result_no_movegen(&hist)
                    .is_some_and(|r| r == Draw)
            );
            hist.push(&board);
            let mov = ChessMove::from_compact_text(mov, &board).unwrap();
            board = board.make_move(mov).unwrap();
            assert_eq!(
                n_fold_repetition(3, &hist, &board, board.ply_100_ctr),
                board.is_3fold_repetition(&hist)
            );
            assert_eq!(
                board.is_3fold_repetition(&hist),
                board.player_result_no_movegen(&hist).is_some()
            );
        }
        board = Chessboard::from_name("lucena").unwrap();
        assert_eq!(board.active_player, White);
        let hash = board.zobrist_hash();
        let moves = ["c1b1", "a2c2", "b1e1", "c2a2", "e1c1"];
        for mov in moves {
            board = board
                .make_move(ChessMove::from_compact_text(mov, &board).unwrap())
                .unwrap();
            assert_ne!(board.zobrist_hash(), hash);
            assert!(!n_fold_repetition(2, &hist, &board, 12345));
        }
        assert_eq!(board.active_player, Black);
    }

    #[test]
    fn checkmate_test() {
        let fen = "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3";
        let pos = Chessboard::from_fen(fen, Strict).unwrap();
        assert_eq!(pos.active_player, White);
        assert_eq!(pos.ply, 4);
        assert!(pos.debug_verify_invariants(Strict).is_ok());
        assert!(pos.is_in_check());
        assert!(pos.is_in_check_on_square(White, pos.king_square(White)));
        let moves = pos.pseudolegal_moves();
        assert!(!moves.is_empty());
        let moves = pos.legal_moves_slow();
        assert!(moves.is_empty());
        assert!(pos.is_game_lost_slow());
        assert_eq!(pos.player_result_slow(&NoHistory::default()), Some(Lose));
        assert!(!pos.is_stalemate_slow());
        assert!(pos.make_nullmove().is_none());
        // this position can be claimed as a draw according to FIDE rules but it's also a mate in 1
        let pos = Chessboard::from_fen("k7/p1P5/1PK5/8/8/8/8/8 w - - 99 51", Strict).unwrap();
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
        let board = Chessboard::from_fen(fen, Strict).unwrap();
        assert_eq!(board.active_player, White);
        assert_eq!(perft(Depth::new_unchecked(3), board).nodes, 568_299);
        // not a legal chess position, but the board should support this
        let fen = "RRRRRRRR/RRRRRRRR/BBBBBBBB/BBBBBBBB/QQQQQQQQ/QQQQQQQQ/QPPPPPPP/K6k b - - 0 1";
        assert!(Chessboard::from_fen(fen, Strict).is_err());
        let board = Chessboard::from_fen(fen, Relaxed).unwrap();
        assert_eq!(board.pseudolegal_moves().len(), 3);
        let mut rng = thread_rng();
        let mov = board.random_legal_move(&mut rng).unwrap();
        let board = board.make_move(mov).unwrap();
        assert_eq!(board.pseudolegal_moves().len(), 2);
        let fen = "B4Q1b/8/8/8/2K3P1/5k2/8/b4RNB b - - 0 1"; // far too many checks, but we still accept it
        assert!(Chessboard::from_fen(fen, Strict).is_err());
        let board = Chessboard::from_fen(fen, Relaxed).unwrap();
        assert_eq!(board.pseudolegal_moves().len(), 8 + 2 * 6);
        assert_eq!(board.legal_moves_slow().len(), 3);
        // maximum number of legal moves in any position reachable from startpos
        let fen = "R6R/3Q4/1Q4Q1/4Q3/2Q4Q/Q4Q2/pp1Q4/kBNN1KB1 w - - 0 1";
        let board = Chessboard::from_fen(fen, Strict).unwrap();
        assert_eq!(board.legal_moves_slow().len(), 218);
        assert!(board.debug_verify_invariants(Strict).is_ok());
        let board = board.flip_side_to_move().unwrap();
        assert!(board.legal_moves_slow().is_empty());
        assert_eq!(board.player_result_slow(&NoHistory::default()), Some(Draw));
    }

    #[test]
    fn chess960_startpos_test() {
        let mut fens = HashSet::new();
        let mut startpos_found = false;
        for i in 0..960 {
            let board = Chessboard::chess_960_startpos(i).unwrap();
            assert!(board.debug_verify_invariants(Strict).is_ok());
            assert!(fens.insert(board.as_fen()));
            let num_moves = board.pseudolegal_moves().len();
            assert!((18..=21).contains(&num_moves)); // 21 legal moves because castling can be legal
            assert_eq!(board.castling.allowed_castling_directions(), 0b1111);
            assert_eq!(
                board.king_square(White).flip_up_down(board.size()),
                board.king_square(Black)
            );
            assert_eq!(board.piece_bb(Pawn).num_ones(), 16);
            assert_eq!(board.piece_bb(Knight).num_ones(), 4);
            assert_eq!(board.piece_bb(Bishop).num_ones(), 4);
            assert_eq!(board.piece_bb(Rook).num_ones(), 4);
            assert_eq!(board.piece_bb(Queen).num_ones(), 2);
            startpos_found |= board == Chessboard::default();
        }
        assert!(startpos_found);
    }

    #[test]
    fn castling_attack_test() {
        let fen = "8/8/8/8/8/8/3k4/RK6 b A - 0 1";
        let pos = Chessboard::from_fen(fen, Strict).unwrap();
        let moves = pos.legal_moves_slow();
        // check that castling moves don't count as attacking squares
        assert!(pos.castling.can_castle(White, Queenside));
        assert_eq!(moves.len(), 6);
        let attacking = pos.all_attacking(ChessSquare::from_str("d1").unwrap());
        assert_eq!(attacking.num_ones(), 1);
        let fen = "8/8/8/3k4/8/8/8/1KRn4 w C - 0 1";
        let pos = Chessboard::from_fen(fen, Strict).unwrap();
        assert!(pos.castling.can_castle(White, Kingside));
        assert!(ChessMove::from_extended_text("0-0", &pos).is_err());
    }

    #[test]
    fn insufficient_material_test() {
        let insufficient = [
            "8/4k3/8/8/8/8/8/2K5 w - - 0 1",
            "8/4k3/8/8/8/8/5N2/2K5 w - - 0 1",
            "8/8/8/6k1/8/2K5/5b2/6b1 w - - 0 1",
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
            let board = Chessboard::from_fen(fen, Strict).unwrap();
            assert!(board.has_insufficient_material(), "{fen}");
            assert!(!board.can_reasonably_win(board.active_player), "{fen}");
            assert!(
                !board.can_reasonably_win(board.active_player.other()),
                "{fen}"
            );
        }
        for fen in sufficient {
            let board = Chessboard::from_fen(fen, Strict).unwrap();
            assert!(!board.has_insufficient_material(), "{fen}");
            assert!(board.can_reasonably_win(board.active_player), "{fen}");
        }
        for fen in sufficient_but_unreasonable {
            let board = Chessboard::from_fen(fen, Strict).unwrap();
            assert!(!board.has_insufficient_material(), "{fen}");
            assert!(!board.can_reasonably_win(board.active_player), "{fen}");
        }
    }
}
