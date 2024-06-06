mod board_impl;
mod common;
mod perft_test;

use crate::games::ataxx::common::ColoredAtaxxPieceType::{BlackPiece, Blocked, Empty, WhitePiece};
use crate::games::ataxx::common::{AtaxxMove, ColoredAtaxxPieceType, MAX_ATAXX_MOVES_IN_POS};
use crate::games::chess::pieces::NUM_COLORS;
use crate::games::Color::{Black, White};
use crate::games::SelfChecks::*;
use crate::games::{
    board_to_string, position_fen_part, Board, Color, ColoredPiece, Coordinates, GenericPiece,
    SelfChecks, Settings, ZobristHash,
};
use crate::general::bitboards::ataxx::{AtaxxBitboard, INVALID_EDGE_MASK};
use crate::general::bitboards::{RawBitboard, RawStandardBitboard};
use crate::general::common::{Res, StaticallyNamedEntity};
use crate::general::move_list::EagerNonAllocMoveList;
use crate::general::squares::{SmallGridSize, SmallGridSquare};
use crate::PlayerResult;
use crate::PlayerResult::{Draw, Lose, Win};
use itertools::Itertools;
use rand::prelude::SliceRandom;
use rand::Rng;
use std::fmt::{Display, Formatter};
use std::str::SplitWhitespace;
use strum::IntoEnumIterator;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct AtaxxSettings {}

impl Settings for AtaxxSettings {}

pub type AtaxxSize = SmallGridSize<7, 7>;

pub type AtaxxSquare = SmallGridSquare<7, 7>;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct AtaxxBoard {
    colors: [RawStandardBitboard; NUM_COLORS],
    empty: RawStandardBitboard,
    active_player: Color,
    ply_100_ctr: usize,
    ply: usize,
}

impl Default for AtaxxBoard {
    fn default() -> Self {
        let white_bb = AtaxxBitboard::from_u64(0x41);
        let black_bb = white_bb << ((7 - 2) * 8);
        Self::create(AtaxxBitboard::default(), white_bb, black_bb).unwrap()
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

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        "Ataxx game".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "Ataxx game. See 'https://en.wikipedia.org/wiki/Ataxx'.".to_string()
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

    fn empty_possibly_invalid(_settings: Self::Settings) -> Self {
        let empty = AtaxxBitboard::default();
        Self::create(empty, empty, empty).unwrap()
    }

    fn bench_positions() -> Vec<Self> {
        let fens = vec![
            "x-1-1-o/-1-1-1-/1-1-1-1/-1-1-1-/1-1-1-1/-1-1-1-/o-1-1-x x 0 1",
            "x-1-1-o/1-1-1-1/1-1-1-1/1-1-1-1/1-1-1-1/1-1-1-1/o-1-1-x x 0 1",
            "x1-1-1o/2-1-2/-------/2-1-2/-------/2-1-2/o1-1-1x x 0 1",
            "x5o/1-----1/1-3-1/1-1-1-1/1-3-1/1-----1/o5x x 0 1",
            "x-1-1-o/1-1-1-1/-1-1-1-/-1-1-1-/-1-1-1-/1-1-1-1/o-1-1-x x 0 1",
            "x5o/1--1--1/1--1--1/7/1--1--1/1--1--1/o5x x 0 1",
            "x-3-o/1-1-1-1/1-1-1-1/3-3/1-1-1-1/1-1-1-1/o-3-x x 0 1",
            "x2-2o/3-3/3-3/-------/3-3/3-3/o2-2x x 0 1",
            "x2-2o/2-1-2/1-3-1/-2-2-/1-3-1/2-1-2/o2-2x x 0 1",
            "x5o/7/7/7/7/7/o5x x 0 1",
            "x5o/7/2-1-2/7/2-1-2/7/o5x x 0 1",
            "x5o/7/3-3/2-1-2/3-3/7/o5x x 0 1",
            "x2-2o/3-3/2---2/7/2---2/3-3/o2-2x x 0 1",
            "x2-2o/3-3/7/--3--/7/3-3/o2-2x x 0 1",
            "x1-1-1o/2-1-2/2-1-2/7/2-1-2/2-1-2/o1-1-1x x 0 1",
            "x5o/7/2-1-2/3-3/2-1-2/7/o5x x 0 1",
            "x5o/7/3-3/2---2/3-3/7/o5x x 0 1",
            "x5o/2-1-2/1-3-1/7/1-3-1/2-1-2/o5x x 0 1",
            "x5o/1-3-1/2-1-2/7/2-1-2/1-3-1/o5x x 0 1",
            "2x3o/7/7/7/o6/5x1/6x o 2 2",
            "5oo/7/x6/x6/7/7/o5x o 0 2",
            "x5o/1x5/7/7/7/2o4/4x2 o 0 2",
            "7/7/2x1o2/1x5/7/7/o5x o 0 2",
            "7/7/1x4o/7/4x2/7/o6 o 3 2",
            "x5o/7/6x/7/1o5/7/7 o 3 2",
            "5oo/7/2x4/7/7/4x2/o6 o 1 2",
            "x5o/7/7/3x3/7/1o5/o6 o 1 2",
            "x5o/7/7/7/7/2x1x2/3x3 o 0 2",
            "7/7/1x4o/7/7/4x2/o6 o 3 2",
            "x5o/7/7/5x1/5x1/1o5/o6 o 0 2",
            "6o/7/4x2/7/7/1o5/o5x o 1 2",
            "x5o/x5o/7/7/7/6x/o5x o 0 2",
            "4x1o/7/7/7/7/o6/o5x o 1 2",
            "6o/7/x6/7/7/2o4/6x o 3 2",
            "x5o/7/7/7/1o4x/7/5x1 o 2 2",
            "x5o/6o/7/7/4x2/7/o6 o 1 2",
            "7/7/1xx1o2/7/7/7/o5x o 0 2",
            "2x3o/2x4/7/7/7/7/2o3x o 0 2",
            "x5o/6o/7/7/4x2/3x3/o6 o 0 2",
            "x5o/7/7/7/o3xx1/7/7 o 0 2",
            "6o/6o/1x5/7/4x2/7/o6 o 1 2",
            "7/7/4x1o/7/7/7/o5x o 3 2",
            "4o2/7/2x4/7/7/7/o4xx o 0 2",
            "2x3o/x6/7/7/7/o6/o5x o 1 2",
            "6o/7/2x4/7/1o5/7/4x2 o 3 2",
            "x6/4o2/7/7/6x/7/o6 o 3 2",
            "x6/7/5o1/7/7/4x2/o6 o 3 2",
            "x5o/1x4o/7/7/7/7/o3x2 o 0 2",
            "xx4o/7/7/7/7/6x/oo4x o 0 2",
            "x6/7/4x2/3x3/7/7/o5x o 2 2",
        ];
        fens.iter()
            .map(|fen| Self::from_fen(fen).unwrap())
            .collect_vec()
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

    fn is_piece_on(&self, sq: AtaxxSquare, piece: ColoredAtaxxPieceType) -> bool {
        match piece {
            Empty => self.empty_bb(),
            Blocked => self.blocked_bb(),
            WhitePiece => self.color_bb(White),
            BlackPiece => self.color_bb(Black),
        }
        .is_bit_set_at(sq.bb_idx())
    }

    fn colored_piece_on(&self, coordinates: Self::Coordinates) -> Self::Piece {
        let idx = coordinates.bb_idx();
        let typ = if self.colors[White as usize].is_bit_set_at(idx) {
            WhitePiece
        } else if self.colors[Black as usize].is_bit_set_at(idx) {
            BlackPiece
        } else if self.empty.is_bit_set_at(idx) {
            Empty
        } else {
            Blocked
        };
        Self::Piece {
            symbol: typ,
            coordinates,
        }
    }

    fn are_all_pseudolegal_legal() -> bool {
        true
    }

    fn pseudolegal_moves(&self) -> Self::MoveList {
        self.legal_moves()
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
            if self.ply_100_ctr >= 100 {
                // losing on the 50mr threshold counts as losing
                return Some(Draw);
            }
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

    fn cannot_reasonably_lose(&self, _player: Color) -> bool {
        false
    }

    fn zobrist_hash(&self) -> ZobristHash {
        self.hash_impl()
    }

    fn as_fen(&self) -> String {
        // Outside code (UAI specifically) expects the first player to be black, not white.
        let stm = match self.active_player {
            White => 'x',
            Black => 'o',
        };

        format!(
            "{} {stm} {halfmove_clock} {fullmove_ctr}",
            position_fen_part(self),
            halfmove_clock = self.halfmove_repetition_clock(),
            fullmove_ctr = self.fullmove_ctr() + 1,
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
        let blocked = self.blocked_bb();
        if blocked & INVALID_EDGE_MASK != INVALID_EDGE_MASK {
            return Err("A squares outside of the board is being used".to_string());
        }
        if checks == CheckFen {
            return Ok(());
        }
        assert_eq!(self.num_squares(), 49);
        let mut overlap = self.colors[0] & self.colors[1];
        if overlap.has_set_bit() {
            return Err(format!(
                "Both players have a piece on the same square ('{}')",
                AtaxxSquare::from_bb_index(overlap.pop_lsb())
            ));
        }
        for color in Color::iter() {
            let mut overlap = self.empty & self.colors[color as usize];
            if overlap.has_set_bit() {
                return Err(format!(
                    "The square '{}' is both empty and occupied by a player",
                    AtaxxSquare::from_bb_index(overlap.pop_lsb())
                ));
            }
        }
        Ok(())
    }

    fn is_empty(&self, coords: Self::Coordinates) -> bool {
        self.empty_bb().is_bit_set_at(coords.bb_idx())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::SelfChecks::Assertion;

    #[test]
    fn startpos_test() {
        let pos = AtaxxBoard::default();
        assert!(pos.verify_position_legal(Assertion).is_ok());
        assert_eq!(pos.color_bb(White).num_ones(), 2);
        assert_eq!(pos.color_bb(Black).num_ones(), 2);
        assert!((pos.blocked_bb() & !INVALID_EDGE_MASK).is_zero());
        let moves = pos.pseudolegal_moves();
        for mov in pos.pseudolegal_moves() {
            assert!(pos.is_move_legal(mov));
            let child = pos.make_move(mov).unwrap();
            assert_ne!(child, pos);
            assert_eq!(child.active_player.other(), pos.active_player);
            assert_ne!(child.zobrist_hash(), pos.zobrist_hash());
        }
        assert_eq!(moves.len(), 16);
    }

    #[test]
    fn empty_pos_test() {
        let pos = AtaxxBoard::empty_possibly_invalid(AtaxxSettings::default());
        assert!(pos.verify_position_legal(Assertion).is_ok());
        assert!(pos.color_bb(White).is_zero());
        assert!(pos.color_bb(Black).is_zero());
        assert!(pos.is_game_lost_slow());
        let moves = pos.legal_moves();
        assert!(moves.is_empty());
    }

    #[test]
    fn simple_test() {
        let fen = "7/7/7/o6/ooooooo/1oooooo/xxxxxxx x 1 2";
        let pos = AtaxxBoard::from_fen(fen).unwrap();
        let moves = pos.legal_moves();
        assert_eq!(moves.len(), 2);
        let pos = AtaxxBoard::from_fen("7/7/7/o6/ooooooo/ooooooo/xxxxxxx x 1 2").unwrap();
        let moves = pos.legal_moves();
        assert_eq!(moves.len(), 1);
    }
}
