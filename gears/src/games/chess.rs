use std::fmt::{Display, Formatter};
use std::num::NonZeroUsize;
use std::str::{FromStr, SplitWhitespace};

use bitintr::Popcnt;
use itertools::Itertools;
use rand::prelude::IteratorRandom;
use rand::Rng;
use strum::IntoEnumIterator;

use crate::games::chess::castling::CastleRight::*;
use crate::games::chess::castling::{CastleRight, CastlingFlags};
use crate::games::chess::moves::ChessMove;
use crate::games::chess::pieces::UncoloredChessPiece::*;
use crate::games::chess::pieces::{
    ChessPiece, ColoredChessPiece, UncoloredChessPiece, NUM_CHESS_PIECES, NUM_COLORS,
};
use crate::games::chess::squares::{ChessSquare, ChessboardSize, NUM_SQUARES};
use crate::games::chess::zobrist::PRECOMPUTED_ZOBRIST_KEYS;
use crate::games::Color::{Black, White};
use crate::games::{
    board_to_string, file_to_char, position_fen_part, read_position_fen, AbstractPieceType, Board,
    BoardHistory, Color, ColoredPiece, ColoredPieceType, DimT, Move, NameToPos, Settings,
    UncoloredPieceType, ZobristHash, ZobristRepetition3Fold,
};
use crate::general::bitboards::chess::{ChessBitboard, BLACK_SQUARES, WHITE_SQUARES};
use crate::general::bitboards::{Bitboard, RawBitboard, RawStandardBitboard};
use crate::general::common::{EntityList, GenericSelect, Res, StaticallyNamedEntity};
use crate::general::move_list::EagerNonAllocMoveList;
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

// TODO: Support Chess960 eventually
#[derive(Eq, PartialEq, Copy, Clone, Debug, Default)]
pub struct ChessSettings {}

pub const MAX_CHESS_MOVES_IN_POS: usize = 256;

// for some reason, Chessboard::MoveList can be ambiguous? This should fix that
pub type ChessMoveList = EagerNonAllocMoveList<Chessboard, MAX_CHESS_MOVES_IN_POS>;

impl Settings for ChessSettings {}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub struct Chessboard {
    piece_bbs: [RawStandardBitboard; NUM_CHESS_PIECES],
    color_bbs: [RawStandardBitboard; NUM_COLORS],
    ply: usize, // TODO: Test if using u32 or even u16 improves nps in perft (also for 50mr counter)
    ply_100_ctr: usize,
    active_player: Color,
    castling: CastlingFlags,
    ep_square: Option<ChessSquare>, // eventually, see if using Optional and Noned instead of Option improves nps
    hash: ZobristHash,
}

impl Default for Chessboard {
    fn default() -> Self {
        Self::startpos(ChessSettings::default())
    }
}

impl StaticallyNamedEntity for Chessboard {
    fn static_short_name() -> &'static str
    where
        Self: Sized,
    {
        "chess"
    }

    fn static_long_name() -> &'static str
    where
        Self: Sized,
    {
        "chess game"
    }

    fn static_description() -> &'static str
    where
        Self: Sized,
    {
        "Chess or Chess960(WIP, not yet supported) game"
    }
}

impl Board for Chessboard {
    type Settings = ChessSettings;
    type Coordinates = ChessSquare;
    type Piece = ChessPiece;
    type Move = ChessMove;
    type MoveList = ChessMoveList;
    type LegalMoveList = ChessMoveList; // TODO: Implement staged movegen eventually

    fn empty_possibly_invalid(_: Self::Settings) -> Self {
        Self {
            piece_bbs: Default::default(),
            color_bbs: Default::default(),
            ply: 0,
            ply_100_ctr: 0,
            active_player: White,
            castling: Default::default(),
            ep_square: None,
            hash: ZobristHash(0),
        }
    }

    fn startpos(_: Self::Settings) -> Self {
        Self::from_fen(START_FEN).expect("Internal error: Couldn't parse startpos fen")
    }

    fn name_to_pos_map() -> EntityList<NameToPos<Self>> {
        vec![
            GenericSelect {
                name: "kiwipete",
                val: || {
                    Self::from_fen(
                        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                    )
                    .unwrap()
                },
            },
            GenericSelect {
                name: "lucena",
                val: || Self::from_fen("1K1k4/1P6/8/8/8/8/r7/2R5 w - - 0 1").unwrap(),
            },
            GenericSelect {
                name: "philidor",
                val: || Self::from_fen("3K4/r7/7R/2kp4/8/8/8/8 w - - 0 1").unwrap(),
            },
            GenericSelect {
                name: "mate_in_1",
                val: || Self::from_fen("8/7r/8/K1k5/8/8/4p3/8 b - - 10 11").unwrap(),
            },
            GenericSelect {
                name: "unusual",
                val: || {
                    Self::from_fen(
                        "2kb1b2/pR2P1P1/P1N1P3/1p2Pp2/P5P1/1N6/4P2B/2qR2K1 w - f6 99 123",
                    )
                    .unwrap()
                },
            },
            GenericSelect {
                name: "see_win_pawn",
                val: || Self::from_fen("k6q/3n1n2/3b4/2P1p3/3P1P2/3N1NP1/8/1K6 w - - 0 1").unwrap(),
            },
            GenericSelect {
                name: "see_xray",
                val: || Self::from_fen("5q1k/8/8/8/RRQ2nrr/8/8/K7 w - - 0 1").unwrap(),
            },
            GenericSelect {
                name: "zugzwang",
                val: || Self::from_fen("6Q1/8/8/7k/8/8/3p1pp1/3Kbrrb w - - 26 14").unwrap(),
            },
        ]
    }

    fn bench_positions() -> Vec<Self> {
        let fens = [
            // fens from Stormphrax, ultimately from bitgenie
            "r3k2r/2pb1ppp/2pp1q2/p7/1nP1B3/1P2P3/P2N1PPP/R2QK2R w KQkq a6 0 14",
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
            "r3kbbr/pp1n1p1P/3ppnp1/q5N1/1P1pP3/P1N1B3/2P1QP2/R3KB1R b KQkq b3 0 17",
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
            "rnbqkb1r/pppppppp/5n2/8/2PP4/8/PP2PPPP/RNBQKBNR b KQkq c3 0 2",
            "2rr2k1/1p4bp/p1q1p1p1/4Pp1n/2PB4/1PN3P1/P3Q2P/2RR2K1 w - f6 0 20",
            "3br1k1/p1pn3p/1p3n2/5pNq/2P1p3/1PN3PP/P2Q1PB1/4R1K1 w - - 0 23",
            "2r2b2/5p2/5k2/p1r1pP2/P2pB3/1P3P2/K1P3R1/7R w - - 23 93",
        ];
        fens.map(|fen| Self::from_fen(fen).unwrap())
            .iter()
            .copied()
            .collect_vec()
    }

    fn settings(&self) -> Self::Settings {
        ChessSettings {}
    }

    fn active_player(&self) -> Color {
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

    fn to_idx(&self, square: Self::Coordinates) -> usize {
        square.index()
    }

    fn to_coordinates(&self, idx: usize) -> Self::Coordinates {
        ChessSquare::new(idx)
    }

    fn colored_piece_on(&self, square: Self::Coordinates) -> Self::Piece {
        let idx = square.index();
        let uncolored = self.uncolored_piece_on(square);
        let color = if self.colored_bb(Black).is_bit_set_at(idx) {
            Black
        } else {
            White // use white as color for `Empty` because that's what `new` expects
        };
        let typ = ColoredChessPiece::new(color, uncolored);
        ChessPiece {
            symbol: typ,
            coordinates: square,
        }
    }

    fn uncolored_piece_on(&self, square: Self::Coordinates) -> UncoloredChessPiece {
        let idx = square.index();
        UncoloredChessPiece::from_uncolored_idx(
            self.piece_bbs
                .iter()
                .position(|bb| bb.is_bit_set_at(idx))
                .unwrap_or(NUM_CHESS_PIECES),
        )
    }

    fn colored_piece_on_idx(&self, idx: usize) -> Self::Piece {
        self.colored_piece_on(ChessSquare::new(idx))
    }

    fn pseudolegal_moves(&self) -> Self::MoveList {
        self.gen_all_pseudolegal_moves()
    }

    fn tactical_pseudolegal(&self) -> Self::MoveList {
        self.gen_tactical_pseudolegal()
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
        self.make_move_impl(mov, mov.uncolored_piece(&self))
    }

    fn make_nullmove(mut self) -> Option<Self> {
        self.ply += 1;
        self.ply_100_ctr += 1;
        if self.ep_square.is_some() {
            self.hash ^=
                PRECOMPUTED_ZOBRIST_KEYS.ep_file_keys[self.ep_square.unwrap().file() as usize];
            self.ep_square = None;
        }
        self.flip_side_to_move()
    }

    fn is_move_pseudolegal(&self, mov: Self::Move) -> bool {
        self.is_move_pseudolegal_impl(mov)
    }

    fn game_result_no_movegen(&self) -> Option<PlayerResult> {
        // 3-fold repetition requires the history, using the free function `game_result_no_movegen`
        if self.is_50mr_draw() || self.has_insufficient_material() {
            return Some(Draw);
        }
        None
    }

    fn game_result_player_slow(&self) -> Option<PlayerResult> {
        if let Some(res) = self.game_result_no_movegen() {
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

    /// Doesn't quite conform to FIDE rules, but probably mostly agrees with USCF rules
    fn cannot_reasonably_lose(&self, player: Color) -> bool {
        let other = player.other();
        if (self.colored_piece_bb(other, Pawn)
            | self.colored_piece_bb(other, Rook)
            | self.colored_piece_bb(other, Queen))
        .has_set_bit()
        {
            return false;
        }
        if self.colored_bb(other).is_single_piece() {
            return true; // opponent has only their king left
        }
        // opponent has at lest one knight or bishop, but no other pieces
        if !self.colored_piece_bb(other, Bishop).has_set_bit()
            && self.colored_piece_bb(other, Knight).is_single_piece()
            && !self.piece_bb(Pawn).has_set_bit()
        {
            // this can very rarely be incorrect because a smothered mate with a knight is possible even without pawns
            return true;
        }
        if !self.colored_piece_bb(other, Knight).has_set_bit()
            && self.colored_piece_bb(other, Bishop).is_single_piece()
            && !self.piece_bb(Pawn).has_set_bit()
        {
            return true;
        }
        false
    }

    fn zobrist_hash(&self) -> ZobristHash {
        self.hash
    }

    fn as_fen(&self) -> String {
        let res = position_fen_part(self);
        let mut castle_rights = String::default();
        // Always output chess960 castling rights. FEN output isn't necessary for UCI
        // and almost all tools support chess960 FEN notation.
        for color in Color::iter() {
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
        let ep_square = self
            .ep_square
            .map(|sq| sq.to_string())
            .unwrap_or("-".to_string());
        res + &format!(
            " {stm} {castle_rights} {ep_square} {halfmove_clock} {move_number}",
            stm = if self.active_player == White {
                "w"
            } else {
                "b"
            },
            halfmove_clock = self.ply_100_ctr,
            move_number = self.fullmove_ctr() + 1
        )
    }

    fn read_fen_and_advance_input(words: &mut SplitWhitespace) -> Res<Self> {
        let pos_word = words
            .next()
            .ok_or_else(|| "Empty chess FEN string".to_string())?;
        let mut board = Chessboard::empty_possibly_invalid(ChessSettings::default());
        board = read_position_fen(pos_word, board, |mut board, square, typ| {
            board.try_place_piece(square, typ)?;
            Ok(board)
        })?;
        let color_word = words.next().ok_or_else(|| {
            "FEN ends after position description, missing color to move".to_string()
        })?;
        // be a bit lenient with parsing the fen
        let color = match color_word.to_ascii_lowercase().as_str() {
            "w" => White,
            "b" => Black,
            x => Err(format!("Expected color ('w' or 'b') in FEN, found '{x}'"))?,
        };
        let castling_word = words
            .next()
            .ok_or_else(|| "FEN ends after color to move, missing castling rights".to_string())?;
        let castling_rights =
            CastlingFlags::default().parse_castling_rights(castling_word, &board)?;

        let ep_square = words.next().ok_or_else(|| {
            "FEN ends after castling rights, missing en passant square".to_string()
        })?;
        board.ep_square = if ep_square == "-" {
            None
        } else {
            Some(ChessSquare::from_str(ep_square)?)
        };
        let halfmove_clock = words.next().unwrap_or("0");
        board.ply_100_ctr = halfmove_clock
            .parse::<usize>()
            .map_err(|err| format!("Couldn't parse halfmove clock: {err}"))?;
        let fullmove_number = words.next().unwrap_or("1");
        let fullmove_number = fullmove_number
            .parse::<NonZeroUsize>()
            .map_err(|err| format!("Couldn't parse fullmove counter: {err}"))?;
        board.ply = (fullmove_number.get() - 1) * 2 + (color == Black) as usize;
        board.active_player = color;
        board.castling = castling_rights;
        board.hash = board.compute_zobrist();
        board.verify_position_legal()?;
        Ok(board)
    }

    fn as_ascii_diagram(&self, flip: bool) -> String {
        board_to_string(self, ChessPiece::to_ascii_char, flip)
    }

    fn as_unicode_diagram(&self, flip: bool) -> String {
        board_to_string(self, ChessPiece::to_utf8_char, flip)
    }

    fn verify_position_legal(&self) -> Res<()> {
        for color in Color::iter() {
            if !self.colored_piece_bb(color, King).is_single_piece() {
                return Err(format!("The {color} player does not have exactly one king"));
            }
            if (self.colored_piece_bb(color, Pawn)
                & (ChessBitboard::rank_no(0) | ChessBitboard::rank_no(7)))
            .has_set_bit()
            {
                return Err(format!(
                    "The {color} player has a pawn on the first or eight rank"
                ));
            }
        }
        let mut hash =
            PRECOMPUTED_ZOBRIST_KEYS.castle_keys[self.castling.allowed_castling_directions()];
        if self.active_player == Black {
            hash ^= PRECOMPUTED_ZOBRIST_KEYS.side_to_move_key;
        }

        for color in Color::iter() {
            for side in CastleRight::iter() {
                let has_eligible_rook =
                    (ChessBitboard::single_piece(self.rook_start_square(color, side).index())
                        & self.colored_piece_bb(color, Rook))
                    .is_single_piece();
                if self.castling.can_castle(color, side) && !has_eligible_rook {
                    return Err(format!("Color {color} can castle {side}, but there is no rook to castle (invalid castling flag in FEN?)"));
                }
            }
        }
        let inactive_player = self.active_player.other();

        if let Some(ep_square) = self.ep_square {
            if self
                .colored_piece_on(ep_square.pawn_move_to_center())
                .symbol
                != ColoredChessPiece::new(inactive_player, Pawn)
            {
                return Err(format!("FEN specifies en passant square {ep_square}, but there is no {inactive_player}-colored pawn on {0}", ep_square.pawn_move_to_center()));
            }
            hash ^= PRECOMPUTED_ZOBRIST_KEYS.ep_file_keys[ep_square.file() as usize];
        }

        if self.is_in_check_on_square(inactive_player, self.king_square(inactive_player)) {
            return Err(format!(
                "Player {inactive_player} is in check, but it's not their turn to move"
            ));
        }
        if self.ply_100_ctr >= 100 {
            return Err(format!(
                "The 50 move rule has been exceeded (there have already been {0} plies played)",
                self.ply_100_ctr
            ));
        }
        if self.ply >= 100_000 {
            return Err(format!("Ridiculously large ply counter: {0}", self.ply));
        }

        for piece in ColoredChessPiece::pieces() {
            let color = piece.color().unwrap();
            let mut bb = self.colored_piece_bb(color, piece.uncolor());
            if bb.num_set_bits() > 20 {
                // Catch this now to prevent crashes down the line because the move list is too small for made-up invalid positions.
                // (This is lax enough to allow many invalid positions that likely won't lead to a crash)
                return Err(format!(
                    "There are {0} {color} {piece}s in this position. There can never be more than 10 pieces \
                    of the same type in a legal chess position (but this implementation accepts up to 20)",
                    bb.num_set_bits()
                ));
            }
            for other_piece in ColoredChessPiece::pieces() {
                if other_piece == piece {
                    continue;
                }
                if (bb & self.colored_piece_bb(other_piece.color().unwrap(), other_piece.uncolor()))
                    .has_set_bit()
                {
                    return Err(format!(
                        "There are two pieces on the same square: {piece} and {other_piece}"
                    ));
                }
            }
            while bb.has_set_bit() {
                let square = ChessSquare::new(bb.pop_lsb());
                hash ^= PRECOMPUTED_ZOBRIST_KEYS.piece_key(piece.uncolor(), color, square);
            }
        }
        if hash != self.compute_zobrist() {
            return Err("Internal error: Compute_zobrist() gives a different result from computing the zobrist hash piece by piece".to_string());
        }
        if hash != self.hash {
            return Err(format!(
                "Error: The zobrist hash doesn't match (should be {hash} but is {0}",
                self.hash
            ));
        }
        Ok(())
    }
}

impl Chessboard {
    pub fn piece_bb(&self, piece: UncoloredChessPiece) -> ChessBitboard {
        debug_assert_ne!(piece, Empty);
        ChessBitboard::new(self.piece_bbs[piece.to_uncolored_idx()])
    }

    pub fn colored_bb(&self, color: Color) -> ChessBitboard {
        ChessBitboard::new(self.color_bbs[color as usize])
    }

    pub fn occupied_bb(&self) -> ChessBitboard {
        debug_assert!((self.colored_bb(White) & self.colored_bb(Black)).is_zero());
        self.colored_bb(White) | self.colored_bb(Black)
    }

    pub fn empty_bb(&self) -> ChessBitboard {
        !self.occupied_bb()
    }

    pub fn is_occupied(&self, square: ChessSquare) -> bool {
        self.occupied_bb().is_bit_set_at(self.to_idx(square))
    }

    pub fn colored_piece_bb(&self, color: Color, piece: UncoloredChessPiece) -> ChessBitboard {
        self.colored_bb(color) & self.piece_bb(piece)
    }

    fn try_place_piece(&mut self, square: ChessSquare, piece: ColoredChessPiece) -> Res<()> {
        let idx = self.to_idx(square);
        if idx >= NUM_SQUARES {
            return Err(format!("Coordinates {square} are outside the chess board"));
        }
        if self.is_occupied(square) {
            return Err(format!("Square {square} is occupied"));
        }
        if piece == ColoredChessPiece::Empty {
            return Err("Can't place the empty piece".to_string());
        }
        self.place_piece(square, piece);
        Ok(())
    }

    fn place_piece(&mut self, square: ChessSquare, piece: ColoredChessPiece) {
        let idx = self.to_idx(square);
        debug_assert_eq!(
            self.colored_piece_on(square).symbol,
            ColoredChessPiece::Empty
        );
        let bb = RawStandardBitboard(1 << idx);
        self.piece_bbs[piece.uncolor() as usize] ^= bb;
        self.color_bbs[piece.color().unwrap() as usize] ^= bb;
    }

    fn remove_piece(&mut self, square: ChessSquare, piece: ColoredChessPiece) {
        let idx = self.to_idx(square);
        debug_assert_eq!(
            self.colored_piece_on(square),
            ChessPiece {
                symbol: piece,
                coordinates: square
            }
        );
        let bb = ChessBitboard::single_piece(idx).raw();
        debug_assert_ne!(piece.uncolor(), Empty);
        self.piece_bbs[piece.uncolor() as usize] ^= bb;
        self.color_bbs[piece.color().unwrap() as usize] ^= bb;
    }

    fn move_piece(&mut self, from: ChessSquare, to: ChessSquare, piece: UncoloredChessPiece) {
        debug_assert_ne!(piece, Empty);
        debug_assert_eq!(self.colored_piece_on(from).uncolored(), piece);
        debug_assert_eq!(
            self.active_player,
            self.colored_piece_on(from).color().unwrap()
        );
        // with chess960 castling, it's possible to move to the source square or a square occupied by a rook
        debug_assert!(
            self.colored_piece_on(to).color() != self.colored_piece_on(from).color()
                || piece == King
                || piece == Rook
        );
        // use ^ instead of | for to merge the from and to bitboards because in chess960 castling
        // it's possible that from == to or that there's another piece on the target square
        let bb = RawStandardBitboard((1 << self.to_idx(from)) ^ (1 << self.to_idx(to)));
        let color = self.active_player;
        self.color_bbs[color as usize] ^= bb;
        self.piece_bbs[piece.to_uncolored_idx()] ^= bb;
        self.update_zobrist_for_move(piece, from, to)
    }

    pub fn is_50mr_draw(&self) -> bool {
        self.ply_100_ctr >= 100
    }

    /// Note that this function isn't entire correct according to the FIDE rules because it doesn't check for legality,
    /// so a position with a possible pseudolegal but illegal en passant move would be considered different from
    /// its repetition, where the en passant move wouldn't be possible
    /// TODO: There should be a ZobristRepetition3FoldPedanticChess that actually does movegen, there could also be a more pedantic
    /// insufficient_material function that wouldn't count 2 knights vs king as draw
    /// TODO: Only set the ep square if there are pseudolegal en passants possible
    pub fn is_3fold_repetition(&self, history: &ZobristRepetition3Fold) -> bool {
        history.game_result(self).is_some()
    }

    pub fn is_stalemate_slow(&self) -> bool {
        self.legal_moves_slow().is_empty() && !self.is_in_check()
    }

    pub fn has_insufficient_material(&self) -> bool {
        // TODO: Test that this function works properly, especially for crazy edge cases like more than the
        // starting number of knights or bishops for one player.
        if self.piece_bb(Pawn).has_set_bit() {
            return false;
        }
        if (self.piece_bb(Queen) | self.piece_bb(Rook)).has_set_bit() {
            return false;
        }
        let bishops = self.piece_bb(Bishop);
        if (bishops & BLACK_SQUARES).has_set_bit() && !(bishops & WHITE_SQUARES).is_zero() {
            return false; // opposite-colored bishops (even if they belong to different players)
        }
        if bishops.has_set_bit() && self.piece_bb(Knight).has_set_bit() {
            return false; // knight and bishop, or knight vs bishop
        }
        let knights = self.piece_bb(Knight);
        let white_knights = self.colored_piece_bb(White, Knight);
        if knights.0.popcnt() >= 3
            || ((knights ^ white_knights).has_set_bit() && white_knights.has_set_bit())
        {
            return false;
        }
        true
    }

    pub fn king_square(&self, color: Color) -> ChessSquare {
        ChessSquare::new(self.colored_piece_bb(color, King).trailing_zeros())
    }

    pub fn is_in_check(&self) -> bool {
        self.is_in_check_on_square(self.active_player, self.king_square(self.active_player))
    }

    pub fn gives_check(&self, mov: ChessMove) -> bool {
        self.make_move(mov).is_some_and(|b| b.is_in_check())
    }

    fn chess960_startpos_white(mut num: usize, color: Color, board: &mut Self) -> Res<()> {
        if num >= 960 {
            return Err(format!("There are only 960 starting positions in chess960 (0 to 959), so position {num} doesn't exist"));
        }
        assert!(board.colored_bb(color).is_zero());
        assert_eq!((board.occupied_bb().raw() & 0xffff), 0);
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
        let mut place_piece = |i: usize, typ: UncoloredChessPiece| {
            let bit = ith_zero(i, board.occupied_bb());
            board.place_piece(ChessSquare::new(bit), ColoredChessPiece::new(White, typ));
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
            .castling
            .set_castle_right(color, Queenside, q_rook as DimT);
        board
            .castling
            .set_castle_right(color, Kingside, k_rook as DimT);
        Ok(())
    }

    pub fn chess_960_startpos(num: usize) -> Res<Self> {
        Self::dfrc_startpos(num, num)
    }

    pub fn dfrc_startpos(white_num: usize, black_num: usize) -> Res<Self> {
        let mut res = Self::empty_possibly_invalid(ChessSettings::default());
        Self::chess960_startpos_white(black_num, Black, &mut res)?;
        for bb in res.piece_bbs.iter_mut() {
            *bb = ChessBitboard::new(*bb).flip_up_down().raw();
        }
        res.color_bbs[Black as usize] = res.colored_bb(White).flip_up_down().raw();
        res.color_bbs[White as usize] = RawStandardBitboard::default();
        Self::chess960_startpos_white(white_num, White, &mut res)?;
        res.hash = res.compute_zobrist();
        res.verify_position_legal().expect("Internal error: Setting up a Chess960 starting position resulted in an invalid position");
        Ok(res)
    }
}

impl Display for Chessboard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{0}", self.as_unicode_diagram(false))
    }
}

#[cfg(test)]
mod tests {
    use rand::thread_rng;
    use std::collections::HashSet;

    use crate::games::chess::squares::{E_FILE_NO, F_FILE_NO, G_FILE_NO};
    use crate::games::{
        game_result_no_movegen, Coordinates, Move, RectangularBoard, RectangularCoordinates,
        ZobristRepetition2Fold,
    };
    use crate::general::perft::perft;
    use crate::search::Depth;

    use super::*;

    const E_1: ChessSquare = ChessSquare::from_rank_file(0, E_FILE_NO);
    const E_8: ChessSquare = ChessSquare::from_rank_file(7, E_FILE_NO);

    #[test]
    fn empty_test() {
        let board = Chessboard::empty_possibly_invalid(ChessSettings::default());
        assert_eq!(board.num_squares(), 64);
        assert_eq!(board.size(), ChessboardSize::default());
        assert_eq!(board.width(), 8);
        assert_eq!(board.height(), 8);
        assert_eq!(board.halfmove_ctr_since_start(), 0);
        assert_eq!(board.fullmove_ctr(), 0);
    }

    #[test]
    fn startpos_test() {
        let board = Chessboard::default();
        assert_eq!(board.num_squares(), 64);
        assert_eq!(board.size(), ChessboardSize::default());
        assert_eq!(board.width(), 8);
        assert_eq!(board.height(), 8);
        assert_eq!(board.halfmove_ctr_since_start(), 0);
        assert_eq!(board.fullmove_ctr(), 0);
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
        assert!(!board.is_3fold_repetition(&ZobristRepetition3Fold::default()));
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
            ChessPiece {
                symbol: ColoredChessPiece::Empty,
                coordinates: square
            }
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

        // let mut engine = Caps::<MaterialOnlyEval>::default();
        // let res = engine.search(board, SearchLimit::depth(4), ZobristHistoryBase::default());
        // assert_eq!(res.score.unwrap(), Score(0));
    }

    #[test]
    fn simple_fen_test() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w Qk - 0 1";
        let board = Chessboard::from_fen(fen).unwrap();
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
            &Chessboard::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w HhAa - 0 1")
                .unwrap()
                .as_fen(),
            "rnbqkbnr/1ppppppp/p7/8/8/8/PPPPPPP1/RNBQKBN1 w Ah - 0 1",
            "rnbqkbnr/1ppppppp/p7/8/3pP3/8/PPPP1PP1/RNBQKBN1 b Ah e3 0 1",
            // chess960 fens (from webperft):
            "1rqbkrbn/1ppppp1p/1n6/p1N3p1/8/2P4P/PP1PPPP1/1RQBKRBN w FBfb - 0 9",
            "rbbqn1kr/pp2p1pp/6n1/2pp1p2/2P4P/P7/BP1PPPP1/R1BQNNKR w HAha - 0 9",
            "rqbbknr1/1ppp2pp/p5n1/4pp2/P7/1PP5/1Q1PPPPP/R1BBKNRN w GAga - 0 9",
        ];
        for fen in fens {
            let board = Chessboard::from_fen(fen).unwrap();
            assert_eq!(fen, board.as_fen());
            assert_eq!(board, Chessboard::from_fen(&board.as_fen()).unwrap());
        }
    }

    #[test]
    fn simple_perft_test() {
        let endgame_fen = "6k1/8/6K1/8/3B1N2/8/8/7R w - - 0 1";
        let board = Chessboard::from_fen(endgame_fen).unwrap();
        let perft_res = perft(Depth::new(1), board);
        assert_eq!(perft_res.depth, Depth::new(1));
        assert_eq!(perft_res.nodes.get(), 5 + 7 + 13 + 14);
        assert!(perft_res.time.as_millis() <= 1);
        let board = Chessboard::default();
        let perft_res = perft(Depth::new(1), board);
        assert_eq!(perft_res.depth, Depth::new(1));
        assert_eq!(perft_res.nodes.get(), 20);
        assert!(perft_res.time.as_millis() <= 1);
        let perft_res = perft(Depth::new(2), board);
        assert_eq!(perft_res.depth, Depth::new(2));
        assert_eq!(perft_res.nodes.get(), 20 * 20);
        assert!(perft_res.time.as_millis() <= 10);

        let board =
            Chessboard::from_fen("r1bqkbnr/1pppNppp/p1n5/8/8/8/PPPPPPPP/R1BQKBNR b KQkq - 0 3")
                .unwrap();
        let perft_res = perft(Depth::new(1), board);
        assert_eq!(perft_res.nodes.get(), 26);
        assert_eq!(perft(Depth::new(3), board).nodes.get(), 16790);

        let board = Chessboard::from_fen(
            "rbbqn1kr/pp2p1pp/6n1/2pp1p2/2P4P/P7/BP1PPPP1/R1BQNNKR w HAha - 0 9",
        )
        .unwrap();
        let perft_res = perft(Depth::new(4), board);
        assert_eq!(perft_res.nodes.get(), 890435);

        // DFRC
        let board = Chessboard::from_fen(
            "r1q1k1rn/1p1ppp1p/1npb2b1/p1N3p1/8/1BP4P/PP1PPPP1/1RQ1KRBN w BFag - 0 9",
        )
        .unwrap();
        assert_eq!(perft(Depth::new(4), board).nodes.get(), 1187103);
    }

    #[test]
    fn mate_test() {
        let board = Chessboard::from_fen("4k3/8/4K3/8/8/8/8/6R1 w - - 0 1").unwrap();
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
            let checkmates = mov.uncolored_piece(&board) == Rook
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
        let mut hist_3_fold = ZobristRepetition3Fold::default();
        let mut hist_2_fold = ZobristRepetition2Fold::default();
        assert_ne!(new_hash, board.zobrist_hash());
        for (i, mov) in moves.iter().enumerate() {
            assert_eq!(i > 3, hist_2_fold.is_repetition(&board));
            assert_eq!(i > 7, hist_3_fold.is_repetition(&board));
            assert_eq!(
                i == 8,
                game_result_no_movegen(&board, &hist_3_fold).is_some_and(|r| r == Draw)
            );
            hist_3_fold.push(&board);
            hist_2_fold.push(&board);
            let mov = ChessMove::from_compact_text(mov, &board).unwrap();
            board = board.make_move(mov).unwrap();
            assert!(board.game_result_no_movegen().is_none());
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
            assert!(!hist_2_fold.is_repetition(&board));
        }
        assert_eq!(board.active_player, Black);
    }

    #[test]
    fn weird_position_test() {
        // There's a similar test in `motors`
        // This fen is actually a legal chess position
        let fen = "q2k2q1/2nqn2b/1n1P1n1b/2rnr2Q/1NQ1QN1Q/3Q3B/2RQR2B/Q2K2Q1 w - - 0 1";
        let board = Chessboard::from_fen(fen).unwrap();
        assert_eq!(board.active_player, White);
        assert_eq!(perft(Depth::new(3), board).nodes.get(), 568299);
        // not a legal chess position, but the board should support this
        let fen = "RRRRRRRR/RRRRRRRR/BBBBBBBB/BBBBBBBB/QQQQQQQQ/QQQQQQQQ/QPPPPPPP/K6k b - - 0 1";
        let board = Chessboard::from_fen(fen).unwrap();
        assert_eq!(board.pseudolegal_moves().len(), 3);
        let mut rng = thread_rng();
        let mov = board.random_legal_move(&mut rng).unwrap();
        let board = board.make_move(mov).unwrap();
        assert_eq!(board.pseudolegal_moves().len(), 2);
    }

    #[test]
    fn chess960_startpos_test() {
        let mut fens = HashSet::new();
        let mut startpos_found = false;
        for i in 0..960 {
            let board = Chessboard::chess_960_startpos(i).unwrap();
            assert!(board.verify_position_legal().is_ok());
            assert!(fens.insert(board.as_fen()));
            let num_moves = board.pseudolegal_moves().len();
            assert!((18..=21).contains(&num_moves)); // 21 legal moves because castling can be legal
            assert_eq!(board.castling.allowed_castling_directions(), 0b1111);
            assert_eq!(
                board.king_square(White).flip_up_down(board.size()),
                board.king_square(Black)
            );
            assert_eq!(board.piece_bb(Pawn).num_set_bits(), 16);
            assert_eq!(board.piece_bb(Knight).num_set_bits(), 4);
            assert_eq!(board.piece_bb(Bishop).num_set_bits(), 4);
            assert_eq!(board.piece_bb(Rook).num_set_bits(), 4);
            assert_eq!(board.piece_bb(Queen).num_set_bits(), 2);
            startpos_found |= board == Chessboard::default();
        }
        assert!(startpos_found);
    }
}
