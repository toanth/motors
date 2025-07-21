mod board_impl;
mod common;
mod perft_test;

use crate::PlayerResult;
use crate::PlayerResult::{Draw, Lose, Win};
use crate::games::ataxx::AtaxxColor::{O, X};
use crate::games::ataxx::common::AtaxxPieceType::Occupied;
use crate::games::ataxx::common::ColoredAtaxxPieceType::{Blocked, Empty, OPiece, XPiece};
use crate::games::ataxx::common::{AtaxxMove, ColoredAtaxxPieceType, MAX_ATAXX_MOVES_IN_POS};
use crate::games::{
    Board, BoardHistory, CharType, Color, ColoredPiece, ColoredPieceType, Coordinates, GenericPiece, NUM_COLORS,
    NoHistory, PosHash, Settings, Size,
};
use crate::general::bitboards::{Bitboard, KnownSizeBitboard, RawBitboard, RawStandardBitboard, SmallGridBitboard};
use crate::general::board::SelfChecks::{Assertion, CheckFen};
use crate::general::board::Strictness::Strict;
use crate::general::board::{
    BitboardBoard, BoardHelpers, PieceTypeOf, SelfChecks, Strictness, Symmetry, UnverifiedBoard, simple_fen,
};
use crate::general::common::{Res, StaticallyNamedEntity, Tokens, ith_one_u64};
use crate::general::move_list::{InplaceMoveList, MoveList};
use crate::general::moves::Move;
use crate::general::squares::SquareColor::White;
use crate::general::squares::{SmallGridSize, SmallGridSquare, SquareColor};
use crate::output::OutputOpts;
use crate::output::text_output::{BoardFormatter, DefaultBoardFormatter, board_to_string, display_board_pretty};
use crate::search::DepthPly;
use anyhow::{bail, ensure};
use arbitrary::Arbitrary;
use itertools::Itertools;
use rand::Rng;
use rand::prelude::IndexedRandom;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::ops::Not;

type AtaxxBitboard = SmallGridBitboard<7, 7>;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct AtaxxSettings;

const ATAXX_SETTINGS: AtaxxSettings = AtaxxSettings {};

impl Settings for AtaxxSettings {}

pub type AtaxxSize = SmallGridSize<7, 7>;

pub type AtaxxSquare = SmallGridSquare<7, 7, 8>;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, derive_more::Display, Arbitrary)]
#[must_use]
pub enum AtaxxColor {
    #[default]
    X,
    O,
}

impl Not for AtaxxColor {
    type Output = Self;

    fn not(self) -> Self::Output {
        self.other()
    }
}

impl From<AtaxxColor> for usize {
    fn from(value: AtaxxColor) -> Self {
        value as usize
    }
}

impl Color for AtaxxColor {
    type Board = AtaxxBoard;

    fn second() -> Self {
        O
    }

    fn to_char(self, _settings: &AtaxxSettings) -> char {
        match self {
            X => 'x',
            O => 'o',
        }
    }

    fn name(self, _settings: &<Self::Board as Board>::Settings) -> &str {
        match self {
            X => "X",
            O => "O",
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Arbitrary)]
#[must_use]
pub struct AtaxxBoard {
    colors: [AtaxxBitboard; NUM_COLORS],
    empty: AtaxxBitboard,
    active_player: AtaxxColor,
    ply_100_ctr: usize,
    ply: usize,
}

impl Default for AtaxxBoard {
    fn default() -> Self {
        let x_bb = AtaxxBitboard::new(0x41);
        let o_bb = x_bb << ((7 - 1) * 8);
        Self::create(AtaxxBitboard::default(), x_bb, o_bb).unwrap()
    }
}

impl Display for AtaxxBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", simple_fen(self, true, true))
    }
}

impl StaticallyNamedEntity for AtaxxBoard {
    fn static_short_name() -> impl Display
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

type AtaxxPiece = GenericPiece<AtaxxBoard, ColoredAtaxxPieceType>;

// for some reason, Chessboard::MoveList can be ambiguous? This should fix that
pub type AtaxxMoveList = InplaceMoveList<AtaxxBoard, MAX_ATAXX_MOVES_IN_POS>;

impl Board for AtaxxBoard {
    // TODO: This is not a useful board state since neither player can make any moves
    type EmptyRes = AtaxxBoard;
    type Settings = AtaxxSettings;
    type SettingsRef = AtaxxSettings;
    type Coordinates = AtaxxSquare;
    type Color = AtaxxColor;
    type Piece = AtaxxPiece;
    type Move = AtaxxMove;
    type MoveList = AtaxxMoveList;

    type Unverified = UnverifiedAtaxxBoard;

    fn empty_for_settings(_settings: Self::Settings) -> Self {
        let empty = AtaxxBitboard::default();
        Self::create(empty, empty, empty).unwrap()
    }

    fn startpos_for_settings(_settings: Self::Settings) -> Self {
        Self::default()
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
            "oxx3o/xxx4/xxx4/5x1/7/7/x5x x 0 1",
            "7/7/7/7/-------/-------/x2xxoo o 1 3", // position where the only legal move is to pass
        ];
        fens.iter().map(|fen| Self::from_fen(fen, Strict).unwrap()).collect_vec()
    }

    fn random_pos(rng: &mut impl Rng, strictness: Strictness, symmetry: Option<Symmetry>) -> Res<Self> {
        loop {
            let mut pos = Self::Unverified::new(Self::empty());
            let mask = if let Some(symmetry) = symmetry {
                match symmetry {
                    Symmetry::Material => pos.0.empty_bb(),
                    Symmetry::Horizontal => AtaxxBitboard::file(0) | AtaxxBitboard::file(1) | AtaxxBitboard::file(2),
                    Symmetry::Vertical => AtaxxBitboard::rank(0) | AtaxxBitboard::rank(1) | AtaxxBitboard::rank(2),
                    Symmetry::Rotation180 => {
                        let files = AtaxxBitboard::file(0)
                            | AtaxxBitboard::file(1)
                            | AtaxxBitboard::file(2)
                            | AtaxxBitboard::file(4);
                        let ranks = AtaxxBitboard::rank(0) | AtaxxBitboard::rank(1) | AtaxxBitboard::rank(2);
                        files & ranks
                    }
                }
            } else {
                pos.0.empty_bb()
            };
            let mut pieces = rng.random_range(2..(mask.num_ones() - 1));
            if symmetry == Some(Symmetry::Material) {
                pieces /= 2;
            }
            for _ in 0..pieces {
                let empty = (pos.0.empty_bb() & mask).raw();
                let sq = rng.random_range(0..empty.num_ones());
                // even though an ataxx bitboard is 7x7, the empty bitboard only has ones on valid squares
                let sq = ith_one_u64(sq, empty);
                let sq = AtaxxSquare::from_bb_idx(sq);
                let color = AtaxxColor::iter().nth(rng.random_range(0..2)).unwrap();
                // doesn't currently generate gaps (doing so would need to ensure the board is connected)
                let piece = ColoredAtaxxPieceType::new(color, Occupied);
                pos.place_piece(sq, piece);
                if let Some(symmetry) = symmetry {
                    let sq = match symmetry {
                        Symmetry::Material => {
                            let empty = pos.0.empty_bb().raw();
                            AtaxxSquare::from_bb_idx(ith_one_u64(rng.random_range(0..empty.num_ones()), empty))
                        }
                        Symmetry::Horizontal => sq.flip_left_right(AtaxxSize::default()),
                        Symmetry::Vertical => sq.flip_up_down(AtaxxSize::default()),
                        Symmetry::Rotation180 => {
                            sq.flip_left_right(AtaxxSize::default()).flip_up_down(AtaxxSize::default())
                        }
                    };
                    let piece = ColoredAtaxxPieceType::new(!color, Occupied);
                    pos.place_piece(sq, piece);
                }
            }
            if rng.random_bool(0.5) {
                pos.0.colors.swap(0, 1);
            }
            if rng.random_bool(0.5) {
                pos.0.active_player = !pos.0.active_player;
            }
            if let Ok(pos) = pos.verify(strictness) {
                return Ok(pos);
            }
        }
    }

    fn settings(&self) -> &AtaxxSettings {
        &ATAXX_SETTINGS
    }

    fn settings_ref(&self) -> Self::SettingsRef {
        ATAXX_SETTINGS
    }

    fn active_player(&self) -> AtaxxColor {
        self.active_player
    }

    fn halfmove_ctr_since_start(&self) -> usize {
        self.ply
    }

    fn ply_draw_clock(&self) -> usize {
        self.ply_100_ctr
    }

    fn size(&self) -> AtaxxSize {
        AtaxxSize::default()
    }

    fn is_empty(&self, coords: Self::Coordinates) -> bool {
        // more efficient than empty_bb() unless the compiler manages to optimize the bitand away (unlikely)
        !self.occupied_bb().is_bit_set_at(coords.bb_idx())
    }

    fn is_piece_on(&self, sq: AtaxxSquare, piece: ColoredAtaxxPieceType) -> bool {
        match piece {
            Empty => self.empty_bb(),
            Blocked => self.blocked_bb(),
            XPiece => self.color_bb(O),
            OPiece => self.color_bb(X),
        }
        .is_bit_set_at(sq.bb_idx())
    }

    fn colored_piece_on(&self, coordinates: Self::Coordinates) -> Self::Piece {
        let idx = coordinates.bb_idx();
        let typ = if self.colors[O as usize].is_bit_set_at(idx) {
            OPiece
        } else if self.colors[X as usize].is_bit_set_at(idx) {
            XPiece
        } else if self.empty.is_bit_set_at(idx) {
            Empty
        } else {
            Blocked
        };
        Self::Piece::new(typ, coordinates)
    }

    fn default_perft_depth(&self) -> DepthPly {
        DepthPly::new(5)
    }

    fn gen_pseudolegal<T: MoveList<Self>>(&self, moves: &mut T) {
        self.gen_legal(moves)
    }

    fn gen_tactical_pseudolegal<T: MoveList<Self>>(&self, _moves: &mut T) {
        // currently, no moves are considered tactical
    }

    fn num_pseudolegal_moves(&self) -> usize {
        self.num_moves()
    }

    fn has_no_legal_moves(&self) -> bool {
        (self.active_player_bb().extended_moore_neighborhood(2) & self.empty_bb()).is_zero()
    }

    fn random_legal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
        self.pseudolegal_moves().choose(rng).copied()
    }

    fn random_pseudolegal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
        self.random_legal_move(rng)
    }

    fn make_move(self, mov: Self::Move) -> Option<Self> {
        Some(self.make_move_impl(mov))
    }

    fn make_nullmove(mut self) -> Option<Self> {
        self.active_player = self.active_player.other();
        self.ply += 1;
        self.ply_100_ctr += 1;
        Some(self)
    }

    fn is_move_pseudolegal(&self, mov: Self::Move) -> bool {
        self.size().coordinates_valid(mov.target)
            && mov.src_square_in(self).is_none_or(|c| self.size().coordinates_valid(c))
            && self.is_move_legal_impl(mov)
    }

    fn player_result_no_movegen<H: BoardHistory>(&self, _history: &H) -> Option<PlayerResult> {
        let color = self.active_player;
        if self.color_bb(color).is_zero() {
            return Some(Lose);
        } else if (self.color_bb(!color).extended_moore_neighborhood(2) & self.empty_bb()).has_set_bit() {
            if self.ply_100_ctr >= 100 {
                // losing on the 50mr threshold counts as losing, so we only test this if we'd otherwise continue playing
                return Some(Draw);
            }
            return None;
        }
        let our_pieces = self.color_bb(color).num_ones();
        let their_pieces = self.color_bb(color.other()).num_ones();
        Some(match our_pieces.cmp(&their_pieces) {
            Ordering::Less => Lose,
            Ordering::Equal => Draw,
            Ordering::Greater => Win,
        })
    }

    fn player_result_slow<H: BoardHistory>(&self, history: &H) -> Option<PlayerResult> {
        self.player_result_no_movegen(history)
    }

    /// If a player has no legal moves, a null move is generated, so this doesn't require any special handling during search.
    /// But if there are no pieces left, the player loses the game.
    fn no_moves_result(&self) -> Option<PlayerResult> {
        self.player_result_slow(&NoHistory::default())
    }

    fn can_reasonably_win(&self, _player: AtaxxColor) -> bool {
        true
    }

    fn hash_pos(&self) -> PosHash {
        self.hash_impl()
    }

    fn read_fen_and_advance_input_for(
        string: &mut Tokens,
        strictness: Strictness,
        _settings: AtaxxSettings,
    ) -> Res<Self> {
        Self::read_fen_impl(string, strictness)
    }

    fn as_diagram(&self, typ: CharType, flip: bool, mark_active: bool) -> String {
        board_to_string(self, AtaxxPiece::to_char, typ, flip, mark_active)
    }

    fn display_pretty(&self, fmt: &mut dyn BoardFormatter<Self>) -> String {
        display_board_pretty(self, fmt)
    }

    fn pretty_formatter(
        &self,
        piece_to_char: Option<CharType>,
        last_move: Option<Self::Move>,
        opts: OutputOpts,
    ) -> Box<dyn BoardFormatter<Self>> {
        Box::new(DefaultBoardFormatter::new(*self, piece_to_char, last_move, opts))
    }

    fn background_color(&self, _coords: Self::Coordinates) -> SquareColor {
        // Don't paint a checkerboard pattern, just make everything white
        White
    }
}

impl BitboardBoard for AtaxxBoard {
    type RawBitboard = RawStandardBitboard;
    type Bitboard = AtaxxBitboard;

    fn piece_bb(&self, _piece: PieceTypeOf<Self>) -> Self::Bitboard {
        self.colors[0] | self.colors[1]
    }

    fn player_bb(&self, color: Self::Color) -> Self::Bitboard {
        self.colors[color as usize]
    }

    fn empty_bb(&self) -> Self::Bitboard {
        self.empty
    }

    fn neutral_bb(&self) -> Self::Bitboard {
        !(self.empty | self.colors[0] | self.colors[1])
    }

    fn mask_bb(&self) -> Self::Bitboard {
        !AtaxxBitboard::INVALID_EDGE_MASK
    }
}

#[derive(Debug, Copy, Clone)]
#[must_use]
pub struct UnverifiedAtaxxBoard(AtaxxBoard);

impl From<AtaxxBoard> for UnverifiedAtaxxBoard {
    fn from(board: AtaxxBoard) -> Self {
        Self(board)
    }
}

impl UnverifiedBoard<AtaxxBoard> for UnverifiedAtaxxBoard {
    fn verify_with_level(self, level: SelfChecks, _strictness: Strictness) -> Res<AtaxxBoard> {
        let this = self.0;
        let blocked = this.blocked_bb();
        if blocked & AtaxxBitboard::INVALID_EDGE_MASK != AtaxxBitboard::INVALID_EDGE_MASK {
            bail!(
                "A square outside of the board is being used ({})",
                AtaxxSquare::unchecked((!blocked & AtaxxBitboard::INVALID_EDGE_MASK).pop_lsb())
            );
        }
        ensure!(
            this.ply_100_ctr <= 100,
            "The halfmove clock is too large: It must be a number between 0 and 100, not {}",
            this.ply_100_ctr
        );
        ensure!(this.ply <= 10_000, "Ridiculously large ply number ({})", this.ply);

        if level == CheckFen {
            return Ok(this);
        }
        if level == Assertion {
            assert_eq!(this.num_squares(), 49);
        }
        let mut overlap = this.colors[0] & this.colors[1];
        ensure!(
            overlap.is_zero(),
            "Both players have a piece on the same square ('{}')",
            AtaxxSquare::from_bb_idx(overlap.pop_lsb())
        );
        for color in AtaxxColor::iter() {
            let mut overlap = this.empty & this.colors[color as usize];
            ensure!(
                overlap.is_zero(),
                "The square '{}' is both empty and occupied by a player",
                AtaxxSquare::from_bb_idx(overlap.pop_lsb())
            );
        }
        Ok(this)
    }

    fn settings(&self) -> &AtaxxSettings {
        self.0.settings()
    }

    fn size(&self) -> AtaxxSize {
        self.0.size()
    }

    fn place_piece(&mut self, square: AtaxxSquare, piece: ColoredAtaxxPieceType) {
        let bb = AtaxxBitboard::single_piece(square);
        self.0.colors[0] &= !bb;
        self.0.colors[1] &= !bb;
        self.0.empty &= !bb;
        match piece {
            Empty => self.0.empty |= bb,
            Blocked => {}
            XPiece => self.0.colors[X as usize] |= bb,
            OPiece => self.0.colors[O as usize] |= bb,
        }
    }

    fn remove_piece(&mut self, square: AtaxxSquare) {
        let bb = AtaxxBitboard::single_piece(square);
        self.0.colors[0] &= !bb;
        self.0.colors[1] &= !bb;
        self.0.empty |= bb;
    }

    fn piece_on(&self, coords: AtaxxSquare) -> AtaxxPiece {
        self.0.colored_piece_on(coords)
    }

    fn is_empty(&self, square: AtaxxSquare) -> bool {
        self.0.is_empty(square)
    }

    fn active_player(&self) -> AtaxxColor {
        self.0.active_player
    }

    fn set_active_player(&mut self, player: AtaxxColor) {
        self.0.active_player = player;
    }

    fn set_ply_since_start(&mut self, ply: usize) -> Res<()> {
        self.0.ply = ply;
        Ok(())
    }

    fn set_halfmove_repetition_clock(&mut self, ply: usize) -> Res<()> {
        self.0.ply_100_ctr = ply;
        Ok(())
    }
}

impl UnverifiedAtaxxBoard {
    pub fn set_halfmove_clock(mut self, halfmove_clock: usize) -> Self {
        self.0.ply_100_ctr = halfmove_clock;
        self
    }

    pub fn set_blockers_bb(mut self, blockers_bb: AtaxxBitboard) -> Self {
        self.0.empty = self.0.empty ^ self.0.blocked_bb() ^ blockers_bb;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::general::board::Strictness::Relaxed;
    use crate::general::moves::Move;
    use crate::general::perft::perft;

    #[test]
    fn startpos_test() {
        let pos = AtaxxBoard::default();
        assert!(pos.debug_verify_invariants(Strict).is_ok());
        assert_eq!(pos.color_bb(O).num_ones(), 2);
        assert_eq!(pos.color_bb(X).num_ones(), 2);
        assert!((pos.blocked_bb() & !AtaxxBitboard::INVALID_EDGE_MASK).is_zero());
        let moves = pos.pseudolegal_moves();
        for mov in pos.pseudolegal_moves() {
            assert!(pos.is_move_legal(mov));
            let child = pos.make_move(mov).unwrap();
            assert_ne!(child, pos);
            assert_eq!(child.active_player.other(), pos.active_player);
            assert_ne!(child.hash_pos(), pos.hash_pos());
        }
        assert_eq!(moves.len(), 16);
    }

    #[test]
    fn empty_pos_test() {
        let pos = AtaxxBoard::empty();
        assert!(pos.debug_verify_invariants(Strict).is_ok());
        assert!(pos.color_bb(O).is_zero());
        assert!(pos.color_bb(X).is_zero());
        assert!(pos.is_game_lost_slow(&NoHistory::default()));
        let moves = pos.legal_moves();
        assert!(moves.is_empty());
    }

    #[test]
    fn simple_test() {
        let fen = "7/7/7/o6/ooooooo/1oooooo/xxxxxxx x 1 2";
        let pos = AtaxxBoard::from_fen(fen, Strict).unwrap();
        let moves = pos.legal_moves();
        assert_eq!(moves.len(), 2);
        let pos = AtaxxBoard::from_fen("7/7/7/o6/ooooooo/ooooooo/xxxxxxx x 1 2", Strict).unwrap();
        let moves = pos.legal_moves();
        assert_eq!(moves.len(), 1);
    }

    #[test]
    fn moves_test() {
        let pos = AtaxxBoard::from_fen("o5o/5o1/7/7/x6/1x5/6x O 1 2", Relaxed).unwrap();
        assert!(AtaxxMove::from_text("a7a6", &pos).is_err());
        assert!(AtaxxMove::from_text("c7a6", &pos).is_err());
        assert!(AtaxxMove::from_text("c7a5", &pos).is_err());
        assert!(AtaxxMove::from_text("a7a4", &pos).is_err());
        assert!(AtaxxMove::from_text("g1g2", &pos).is_err());
        let mov = AtaxxMove::from_text("g2", &AtaxxBoard::default()).unwrap();
        assert!(!pos.is_move_legal(mov));
        let mov = AtaxxMove::from_text("a7c5", &pos).unwrap();
        assert!(pos.legal_moves().contains(&mov));
        let pos = pos.make_move(mov).unwrap();
        assert!(AtaxxMove::from_extended_text("a3c5", &pos).is_err());
        assert!(AtaxxMove::from_text("a3b5", &pos).is_ok());
    }

    #[test]
    fn perft_test() {
        let pos = AtaxxBoard::from_fen("7/7/7/7/-------/-------/--x3o x 1 2", Strict).unwrap();
        let expected = [1, 2, 3, 3, 4, 5, 5, 3, 3, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3, 2, 3, 3];
        for (i, &nodes) in expected.iter().enumerate() {
            let res = perft(DepthPly::new(i), pos, false);
            assert_eq!(res.nodes, nodes, "Depth {i}: {pos}");
        }
    }
}
