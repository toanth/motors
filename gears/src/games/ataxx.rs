mod board_impl;
mod common;

use crate::games::ataxx::common::ColoredAtaxxPieceType::{BlackPiece, Blocked, Empty, WhitePiece};
use crate::games::ataxx::common::{
    AtaxxMove, AtaxxPieceType, AtaxxSize, AtaxxSquare, ColoredAtaxxPieceType,
    MAX_ATAXX_MOVES_IN_POS, NUM_SQUARES,
};
use crate::games::chess::pieces::NUM_COLORS;
use crate::games::Color::{Black, White};
use crate::games::{
    board_to_string, position_fen_part, AbstractPieceType, Board, Color, ColoredPiece, Coordinates,
    GenericPiece, SelfChecks, Settings, ZobristHash,
};
use crate::general::bitboards::chess::ChessBitboard;
use crate::general::bitboards::RawBitboard;
use crate::general::common::{Res, StaticallyNamedEntity};
use crate::general::move_list::EagerNonAllocMoveList;
use crate::general::squares::SmallGridSize;
use crate::PlayerResult;
use crate::PlayerResult::{Draw, Lose, Win};
use rand::prelude::SliceRandom;
use rand::Rng;
use std::fmt::{Display, Formatter};
use std::str::SplitWhitespace;
use strum::IntoEnumIterator;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct AtaxxSettings {}

impl Settings for AtaxxSettings {}

pub type AtaxxBoardSize = SmallGridSize<7, 7>;

pub type AtaxxBitboard = ChessBitboard;

const INVALID_EDGE_MASK: AtaxxBitboard = AtaxxBitboard::from_u64(0x8080_8080_8080_80ff);

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct AtaxxBoard {
    colors: [AtaxxBitboard; NUM_COLORS],
    empty: AtaxxBitboard,
    active_player: Color,
    ply_100_ctr: usize,
    ply: usize,
}

impl Default for AtaxxBoard {
    fn default() -> Self {
        Self::new(AtaxxBitboard::default())
    }
}

impl Display for AtaxxBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_fen())
    }
}

impl StaticallyNamedEntity for AtaxxBoard {
    fn static_short_name() -> &'static str
    where
        Self: Sized,
    {
        "ataxx"
    }

    fn static_long_name() -> &'static str
    where
        Self: Sized,
    {
        "ataxx game"
    }

    fn static_description() -> &'static str
    where
        Self: Sized,
    {
        "Ataxx game. See 'https://en.wikipedia.org/wiki/Ataxx'."
    }
}

type AtaxxPiece = GenericPiece<AtaxxSquare, ColoredAtaxxPieceType>;

// for some reason, Chessboard::MoveList can be ambiguous? This should fix that
pub type AtaxxMoveList = EagerNonAllocMoveList<AtaxxBoard, MAX_ATAXX_MOVES_IN_POS>;

impl Board for AtaxxBoard {
    type Settings = AtaxxSettings;
    type Coordinates = AtaxxSquare;
    type Piece = AtaxxPiece;
    type Move = AtaxxMove;
    type MoveList = AtaxxMoveList;
    type LegalMoveList = Self::MoveList;

    fn startpos(_settings: Self::Settings) -> Self {
        Self::default()
    }

    fn settings(&self) -> Self::Settings {
        AtaxxSettings::default()
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

    fn size(&self) -> <Self::Coordinates as Coordinates>::Size {
        AtaxxSize::default()
    }

    fn colored_piece_on_idx(&self, pos: usize) -> Self::Piece {
        let typ = if self.colors[White as usize].is_bit_set_at(pos) {
            WhitePiece
        } else if self.colors[Black as usize].is_bit_set_at(pos) {
            BlackPiece
        } else if self.empty.is_bit_set_at(pos) {
            Empty
        } else {
            Blocked
        };
        Self::Piece {
            symbol: typ,
            coordinates: AtaxxSquare::new(pos),
        }
    }

    fn pseudolegal_moves(&self) -> Self::MoveList {
        self.movegen()
    }

    fn tactical_pseudolegal(&self) -> Self::MoveList {
        AtaxxMoveList::default()
    }

    fn random_legal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
        let moves = self.pseudolegal_moves();
        if moves.is_empty() {
            None
        } else {
            moves.choose(rng).copied()
        }
    }

    fn random_pseudolegal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
        self.random_legal_move(rng)
    }

    fn make_move(self, mov: Self::Move) -> Option<Self> {
        Some(self.make_move_impl(mov))
    }

    fn make_nullmove(mut self) -> Option<Self> {
        self.active_player = self.active_player.other();
        Some(self)
    }

    fn is_move_pseudolegal(&self, mov: Self::Move) -> bool {
        self.is_move_legal_impl(mov)
    }

    fn game_result_no_movegen(&self) -> Option<PlayerResult> {
        let color = self.active_player;
        if self.color_bb(color).is_zero() {
            return Some(Lose);
        } else if self.empty.has_set_bit() {
            return None;
        }
        let our_pieces = self.color_bb(color).num_ones();
        let their_pieces = self.color_bb(color.other()).num_ones();
        Some(if our_pieces > their_pieces {
            Win
        } else if our_pieces == their_pieces {
            Draw
        } else {
            Lose
        })
    }

    fn game_result_player_slow(&self) -> Option<PlayerResult> {
        self.game_result_no_movegen()
    }

    /// If a player has no legal moves, a null move is generated, so this doesn't require any special handling during search.
    /// But if there are no pieces left, the player loses the game.
    fn no_moves_result(&self) -> PlayerResult {
        self.game_result_player_slow().unwrap()
    }

    fn cannot_reasonably_lose(&self, player: Color) -> bool {
        false
    }

    fn zobrist_hash(&self) -> ZobristHash {
        self.hash_impl()
    }

    fn as_fen(&self) -> String {
        /// Outside code (UAI specifically) expects the first player to be black, not white.
        let stm = match self.active_player {
            White => 'b',
            Black => 'w',
        };

        format!(
            "{} {stm} {halfmove_clock} {fullmove_ctr}",
            position_fen_part(self),
            halfmove_clock = self.halfmove_repetition_clock(),
            fullmove_ctr = self.fullmove_ctr(),
        )
    }

    fn read_fen_and_advance_input(string: &mut SplitWhitespace) -> Res<Self> {
        Self::read_fen_impl(string)
    }

    fn as_ascii_diagram(&self, flip: bool) -> String {
        board_to_string(self, AtaxxPiece::to_ascii_char, flip)
    }

    fn as_unicode_diagram(&self, flip: bool) -> String {
        board_to_string(self, AtaxxPiece::to_utf8_char, flip)
    }

    fn verify_position_legal(&self, checks: SelfChecks) -> Res<()> {
        let mut overlap = self.colors[0] & self.colors[1];
        if overlap.has_set_bit() {
            return Err(format!(
                "Both players have a piece on the same square ('{}')",
                AtaxxSquare::new(overlap.pop_lsb())
            ));
        }
        for color in Color::iter() {
            let mut overlap = self.empty ^ self.colors[color as usize];
            if overlap.has_set_bit() {
                return Err(format!(
                    "The square '{}' is both empty and occupied by a player",
                    AtaxxSquare::new(overlap.pop_lsb())
                ));
            }
        }
        let blocked = self.blocked_bb();
        if blocked & INVALID_EDGE_MASK != INVALID_EDGE_MASK {
            return Err("A squares outside of the board is being used".to_string());
        }
        // todo: merge main into this branch, impl different levels of checking
        Ok(())
    }
}
