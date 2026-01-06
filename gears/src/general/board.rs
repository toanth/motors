/*
 *  Gears, a collection of board games.
 *  Copyright (C) 2024 ToTheAnd
 *
 *  Gears is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Gears is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Gears. If not, see <https://www.gnu.org/licenses/>.
 */
use crate::PlayerResult::{Draw, Lose};
use crate::games::CharType::Ascii;
use crate::games::{
    AbstractPieceType, BoardHistory, CharType, ColorTrait, ColoredPieceTrait, ColoredPieceTypeTrait, CoordinatesTrait,
    DimT, PosHash, SettingsTrait, SizeTrait, file_to_char,
};
use crate::general::bitboards::{BitboardTrait, RawBitboardTrait};
use crate::general::board::SelfChecks::{Assertion, Verify};
use crate::general::board::Strictness::{Relaxed, Strict};
use crate::general::common::Description::NoDescription;
use crate::general::common::{
    EntityList, GenericSelect, NamedEntity, Res, StaticallyNamedEntity, Tokens, TokensToString, select_name_static,
    tokens,
};
use crate::general::move_list::{MoveIter, MoveListTrait};
use crate::general::moves::ExtendedFormat::Standard;
use crate::general::moves::Legality::{Legal, PseudoLegal};
use crate::general::moves::MoveTrait;
use crate::general::squares::{RectangularCoordinates, RectangularSize, SquareColor};
use crate::output::OutputOpts;
use crate::output::text_output::BoardFormatter;
use crate::search::DepthPly;
use crate::ugi::Protocol;
use crate::{GameOver, GameOverReason, MatchResult, PlayerResult, player_res_to_match_res};
use anyhow::{anyhow, bail, ensure};
use arbitrary::Arbitrary;
use colored::Colorize;
use num::Zero;
use rand::Rng;
use std::fmt::{Debug, Display, Formatter};
use std::iter::FusedIterator;
use std::num::NonZeroUsize;
use std::str::Split;
use std::{fmt, iter};
use strum_macros::EnumIter;

#[derive(Debug, Copy, Clone)]
pub struct NameToPos {
    pub name: &'static str,
    pub fen: &'static str,
    pub description: Option<&'static str>,
    pub strictness: Strictness,
}

impl NameToPos {
    pub fn strict(name: &'static str, fen: &'static str) -> Self {
        Self { name, fen, description: None, strictness: Strict }
    }

    pub fn desc(name: &'static str, fen: &'static str, desc: &'static str) -> Self {
        Self { name, fen, description: Some(desc), strictness: Strict }
    }

    pub fn create<B: BoardTrait>(&self) -> B {
        B::from_fen(self.fen, self.strictness).unwrap()
    }
}

impl NamedEntity for NameToPos {
    fn short_name(&self) -> String {
        self.name.to_string()
    }

    fn long_name(&self) -> String {
        self.short_name()
    }

    fn description(&self) -> Option<String> {
        self.description.map(|s| s.to_string())
    }
}

/// How many checks to execute.
/// Enum variants are listed in order; later checks generally include earlier checks.
/// In some cases [`SelfChecks::CheckFen`] silently fixes an incorrect ep square in [`Relaxed`] mode.
#[derive(Debug, Eq, PartialEq, Copy, Clone, Ord, PartialOrd)]
#[must_use]
pub enum SelfChecks {
    Tablebase,
    CheckFen,
    Verify,
    Assertion,
}

/// How strict are the game rules interpreted.
/// For example, [`Relaxed`] doesn't care about reachability from startpos.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
#[must_use]
pub enum Strictness {
    Relaxed,
    Strict,
}

// In the future, this could also include diagonal and antidiagonal
#[derive(Debug, Copy, Clone, Eq, PartialEq, EnumIter)]
#[must_use]
pub enum Symmetry {
    Material,
    Horizontal,
    Vertical,
    Rotation180,
}

impl NamedEntity for Symmetry {
    fn short_name(&self) -> String {
        match self {
            Symmetry::Material => "Material".to_string(),
            Symmetry::Horizontal => "Horizontal".to_string(),
            Symmetry::Vertical => "Vertical".to_string(),
            Symmetry::Rotation180 => "Rotation".to_string(),
        }
    }

    fn long_name(&self) -> String {
        self.short_name()
    }

    fn description(&self) -> Option<String> {
        Some(format!("Set the symmetry to '{}'", self.short_name()))
    }
}

/// An [`UnverifiedBoardTrait`] is a [`BoardTrait`] where invariants can be violated.
pub trait UnverifiedBoardTrait<B: BoardTrait>: Debug + Clone + From<B>
where
    B: BoardTrait<Unverified = Self>,
{
    /// Conceptually, this simply copies the board.
    fn new(board: B) -> Self {
        Self::from(board)
    }

    /// Same as `verify_with_level(Verify)` for release builds
    /// and `verify_with_level(Assertion)` in debug builds
    fn verify(self, strictness: Strictness) -> Res<B> {
        if cfg!(debug_assertions) {
            self.verify_with_level(Assertion, strictness)
        } else {
            self.verify_with_level(Verify, strictness)
        }
    }

    /// Verifies that the position is legal. This function is meant to be used in `assert!`s,
    /// for validating input, such as FENs, and for allowing a user to programmatically set up custom positions and then
    /// verify that they are legal. Iy should not be used for filtering positions after a call to `make_move`
    /// (it should already be ensured through other means that the move results in a legal position or `None`).
    /// If `checks` is `Assertion`, this performs internal validity checks, which is useful for asserting that there are no
    /// bugs in the implementation, but unnecessary if this function is only called to check the validity of a FEN.
    /// `CheckFen` sometimes needs to do less work than `Verify` because the FEN format makes it impossible to express some
    /// invalid board states, such as two pieces being on the same square.
    /// Strictness refers to which positions are considered legal.
    fn verify_with_level(self, level: SelfChecks, strictness: Strictness) -> Res<B>;

    // TODO: Refactor such that debug_verify_invariants actually checks invariants that should not be broken in a Board
    // but are getting corrected in the verify method of an UnverifiedBoard

    fn settings(&self) -> &B::Settings;

    fn name(&self) -> String {
        B::game_name()
    }

    /// Returns the size of the board.
    fn size(&self) -> BoardSize<B>;

    /// Checks if the given coordinates are valid.
    fn check_coordinates(&self, coords: B::Coordinates) -> Res<B::Coordinates> {
        self.size().check_coordinates(coords)
    }

    /// Place a piece of the given type and color on the given square. Like all functions that return an [`UnverifiedBoardTrait`],
    /// this doesn't check that the resulting position is legal.
    /// However, this function can still fail if the piece can't be placed.
    /// If there is a piece already on the square, it is implementation-defined what will happen; possible options include
    /// replacing the piece, returning an `Err`, or silently going into a bad state that will return an `Err` on [`Self::verify`].
    /// May perform expensive checks.
    fn try_place_piece(&mut self, piece: B::Piece) -> Res<()> {
        let square = self.check_coordinates(piece.coordinates())?;
        let piece = piece.colored_piece_type();
        if piece == ColPieceTypeOf::<B>::empty() {
            bail!("Trying to place an empty piece on {square}")
        }
        // TODO: PieceType should not include the empty square; use a different, generic, struct for that
        if !self.is_empty(square) {
            bail!(
                "Can't place a {0} on {1} because there is already a {2} there",
                piece.name(self.settings()).as_ref().red(),
                square.to_string().red(),
                self.piece_on(square).colored_piece_type().name(self.settings()).as_ref().bold()
            )
        }
        self.place_piece(square, piece);
        Ok(())
    }

    /// Like [`Self::try_place_piece`], but does not check preconditions, such as that the coordinates are valid.
    /// Unlike `try_place_piece`, this function can panic if the coordinates are not empty.
    /// Not intended to do any expensive checks.
    fn place_piece(&mut self, coords: B::Coordinates, piece: ColPieceTypeOf<B>);

    /// Remove the piece at the given coordinates. If there is no piece there, nothing happens.
    /// If the coordinates are invalid, an `Err` is returned.
    /// Some [`UnverifiedBoardTrait`]s can represent multiple pieces at the same coordinates; it is implementation-defined
    /// what this method does in that case.
    fn try_remove_piece(&mut self, coords: B::Coordinates) -> Res<()> {
        if !self.try_get_piece_on(coords)?.is_empty() {
            self.remove_piece(coords)
        }
        Ok(())
    }

    /// Like [`Self::try_remove_piece`], but does not check that the coordinates are valid.
    fn remove_piece(&mut self, coords: B::Coordinates);

    /// Like [`Self::try_place_piece`], but replaces any piece that is already on the given coordinates.
    fn try_replace_piece(&mut self, coords: B::Coordinates, piece: ColPieceTypeOf<B>) -> Res<()> {
        self.try_remove_piece(coords)?;
        self.place_piece(coords, piece);
        Ok(())
    }

    /// Returns the piece on the given coordinates, or `None` if the coordinates aren't valid.
    /// Some [`UnverifiedBoardTrait`]s can represent multiple pieces at the same coordinates; it is implementation-defined
    /// what this method does in that case (but it should never return empty coordinates in that case).
    fn try_get_piece_on(&self, coords: B::Coordinates) -> Res<B::Piece> {
        Ok(self.piece_on(self.check_coordinates(coords)?))
    }

    /// Returns the piece on the given coordinates, or `None` if the coordinates aren't valid.
    /// Some [`UnverifiedBoardTrait`]s can represent multiple pieces at the same coordinates; it is implementation-defined
    /// what this method does in that case (but it should never return empty coordinates in that case).
    fn piece_on(&self, coords: B::Coordinates) -> B::Piece;

    /// See [`B::is_empty`].
    fn is_empty(&self, coords: B::Coordinates) -> bool;

    fn active_player(&self) -> B::Color;

    /// Set the active player. Like all of these functions, it does not guarantee or check that the resulting position
    /// is legal. For example, in chess, the side not to move might be in check, so that it would be possible to capture the king.
    fn set_active_player(&mut self, player: B::Color);

    /// Set the ply counter since the start of the game. Does not check that the resulting positions is legal, e.g. if
    /// the ply counter is larger than the number of placed pieces in games like m,n,k games or Ultimate Tic-Tac-Toe.
    /// Can fail if the ply number is not representable in the internal representation.
    fn set_ply_since_start(&mut self, ply: usize) -> Res<()>;

    fn set_halfmove_repetition_clock(&mut self, ply: usize) -> Res<()>;

    /// Returns true if the position word of a fen ends with '\[hand\]'
    fn fen_pos_part_contains_hand(&self) -> bool {
        false
    }

    /// Loads the hand part of a FEN, which is relevant for fairy chess variants like crazyhouse.
    /// A member of this trait so that implementations can override it, but this method shouldn't need to be called directly.
    fn read_fen_hand_part(&mut self, _input: &str) -> Res<()> {
        bail!("FENs for the game '{}' do not contain a hand part", self.name())
    }

    // TODO: Also put more methods, like `as_fen`, in this trait?
    // Might be useful to print such boards, but the implementation might be annoying because we can't rely on invariants
}

// Rustc warns that the `Board` bounds are not enforced but removing them makes the program fail to compile
#[expect(type_alias_bounds)]
pub type ColPieceTypeOf<B: BoardTrait> = <B::Piece as ColoredPieceTrait<B>>::ColoredPieceType;

#[expect(type_alias_bounds)]
pub type PieceTypeOf<B: BoardTrait> = <ColPieceTypeOf<B> as ColoredPieceTypeTrait<B>>::Uncolored;

#[expect(type_alias_bounds)]
pub type BoardSize<B: BoardTrait> = <B::Coordinates as CoordinatesTrait>::Size;

/// Currently, a game is completely determined by the `Board` type:
/// The type implementing `Board` contains all the necessary information about the rules of the game.
/// However, a `Board` is assumed to be markovian and needs to be `'static`.
/// Despite not requiring [`Copy`], a board should be cheap to clone.
/// In fact, currently all boards except [`FairyBoard`] implement [`Copy`]
pub trait BoardTrait:
    Debug
    + Display
    + Send
    + Sync
    + Sized
    + Default
    + Clone
    + Eq
    + PartialEq
    + StaticallyNamedEntity
    + for<'a> Arbitrary<'a>
    + 'static
{
    /// Should be either `Self::Unverified` or `Self`
    type EmptyRes: Into<Self::Unverified>;
    /// Even if the board doesn't use bitboards internally, this type must be able to represent an arbitrary subset of coordinates.
    /// The [`Self::size()`] of this board can be used to convert between bits and coordinates (i.e. squares for rectangular boards)
    type RawBitboard: RawBitboardTrait;
    type Settings: SettingsTrait;
    type SettingsRef: Default + Eq;
    type Coordinates: CoordinatesTrait;
    type Color: ColorTrait<Board = Self>;
    type Piece: ColoredPieceTrait<Self>;
    type Move: MoveTrait<Self>;
    type MoveList: MoveListTrait<Self> + Default;
    type Unverified: UnverifiedBoardTrait<Self>;

    /// The position returned by this function does not have to be legal, e.g. in chess it would
    /// not include any kings. However, this is still useful to set up the board and is used
    /// in fen parsing, for example.
    fn empty_for_settings(settings: Self::SettingsRef) -> Self::EmptyRes;

    /// Like `empty_for_setting`, but uses a default settings objects.
    /// Most games have empty setting objects, so explicitly passing in settings is unnecessary.
    fn empty() -> Self::EmptyRes {
        Self::empty_for_settings(Self::SettingsRef::default())
    }

    /// The starting position of the game.
    ///
    /// The settings are used e.g. to set the m, n and k parameters of the mnk board.
    /// This always returns the same position, even when there are technically multiple starting positions.
    /// For example, the `Chessboard` implementation supports (D)FRC, but `startpos()` still only returns
    /// the standard chess start pos
    fn startpos_for_settings(settings: Self::SettingsRef) -> Self;

    /// Like [`Self::startpos_for_settings()`] with default settings.
    /// Most boards have empty settings, so explicitly passing in settings is unnecessary.
    /// Usually, `startpos()` returns the same as `default()`, but this isn't enforced.
    fn startpos() -> Self {
        Self::startpos_for_settings(Self::SettingsRef::default())
    }

    /// Constructs a specific, well-known position from its name, such as 'kiwipete' in chess.
    /// Not to be confused with [`Self::from_fen`], which can load arbitrary positions.
    /// The default implementation forwards to [`board_from_name`].
    fn from_name(name: &str) -> Res<Self> {
        board_from_name(name)
    }

    /// Returns a Vec mapping well-known position names to their FEN, for example for `kiwipete` in chess.
    /// Can be implemented by a concrete [`BoardTrait`], which will make [`Self::from_name`] recognize the name and lets the
    /// UI know about supported positions.
    /// "startpos" is handled automatically in `from_name` but can be overwritten here.
    #[must_use]
    fn name_to_pos_map() -> EntityList<NameToPos> {
        vec![]
    }

    /// `bench` positions are used in various places, such as for testing the engine, measuring search speed, and in calling `bench`
    /// on an engine.
    #[must_use]
    fn bench_positions() -> impl IntoIterator<Item = Self>;

    /// Return a random legal (but `Relaxed`) position. Not every position has to be able to be generated, and there
    /// are no requirements for the distribution of positions. So always returning startpos would be a valid, if poor,
    /// implementation. Not all implementation have to support this function or all symmetries, so it returns a `Res`.
    fn random_pos(rng: &mut impl Rng, strictness: Strictness, symmetry: Option<Symmetry>) -> Res<Self>;

    fn settings(&self) -> &Self::Settings;

    fn settings_ref(&self) -> Self::SettingsRef;

    /// Returns a board in the startpos of the variant corresponding to the `name`.
    /// `_additional` can be used to modify the variant, e.g. to set the board size in mnk games.
    // fn variant(name: &str, _additional: &mut Tokens) -> Res<Self> {
    //     bail!("The game {0} does not support any variants, including '{1}'", Self::game_name(), name.red())
    // }
    fn variant_for(name: &str, _additional: &mut Tokens, _proto: Protocol) -> Res<Self> {
        bail!("The game {0} does not support any variants, including '{1}'", Self::game_name(), name.red())
    }

    fn list_variants() -> Option<Vec<String>> {
        None
    }

    /// The player who can now move.
    fn active_player(&self) -> Self::Color;

    /// The number of half moves (plies) since the start of the game.
    fn halfmove_ctr_since_start(&self) -> usize;

    /// An upper bound on the number of past plies that need to be considered for repetitions.
    /// This can be the same as [`Self::halfmove_ctr_since_start`] or always zero if repetitions aren't possible.
    fn ply_draw_clock(&self) -> usize;

    /// A bitboard of all squares. If the board size isn't the same as the number of bits in the bitboard type,
    /// some bits don't correspond to squares and will be zero in the result of this function.
    fn valid_squares_bb(&self) -> Self::RawBitboard;

    /// The size of the board.
    fn size(&self) -> BoardSize<Self>;

    /// Returns `true` iff there is no piece on the given square.
    fn is_empty(&self, coords: Self::Coordinates) -> bool;

    /// Returns `true` iff a piece of the given type and color exists on the given coordinates.
    /// Can sometimes be implemented more efficiently than by comparing `colored_piece_on`.
    fn is_piece_on(&self, coords: Self::Coordinates, piece: ColPieceTypeOf<Self>) -> bool {
        self.colored_piece_on(coords).colored_piece_type() == piece
    }

    /// Returns the piece at the given coordinates.
    /// [`Self::piece_type_on`] can sometimes be implemented more efficiently, e.g. for chess.
    fn colored_piece_on(&self, coords: Self::Coordinates) -> Self::Piece;

    /// Returns the uncolored piece type at the given coordinates.
    /// Can sometimes be implemented more efficiently than [`Self::colored_piece_on`]
    fn piece_type_on(&self, coords: Self::Coordinates) -> PieceTypeOf<Self> {
        self.colored_piece_on(coords).uncolored()
    }

    /// Returns an iterator over all the pieces in the given player's hand, or an iterator that yields `None`
    /// if this game doesn't have a meaningful hand. For example, technically a mnk game could be modelled as
    /// players dropping pieces from their hand, but because their hands are infinite and the contents don't
    /// matter, this method still returns `None`.
    fn hand(&self, _color: Self::Color) -> impl Iterator<Item = (usize, PieceTypeOf<Self>)> {
        iter::empty()
    }

    /// Returns a bitboard of all attacks of the piece on the given square.
    /// If there is no piece on that square, return the empty bitboard.
    /// What constitutes an 'attack' is game-dependent; in particular there does not have to be
    /// a one-to-one relation between pseudolegal moves of that piece and set bits of the returned bitboard.
    /// For example, in chess, promotions mean that a bit can correspond to more than one move, and
    /// because pawn pushed or castling moves can't capture, they don't count as attacks and won't set a bit.
    fn attacks_of(&self, sq: Self::Coordinates) -> Self::RawBitboard;

    /// Returns the default depth that should be used for perft if not otherwise specified.
    /// Takes in a reference to self because some boards have a size determined at runtime,
    /// and the default perft depth can change depending on that (or even depending on the current position)
    fn default_perft_depth(&self) -> DepthPly;

    /// The maximum argument for `perft` and `splitperft`.
    fn max_perft_depth() -> DepthPly {
        DepthPly::new(1000)
    }

    /// Most games (e.g., chess) don't need any special checks for game-over conditions in perft, but some should explicitly test
    /// if the game is over (e.g. mnk) because movegen wouldn't do this automatically otherwise.
    /// If this function returns `true`, [`Self::player_result_no_movegen`] must return a `Some`.
    fn cannot_call_movegen(&self) -> bool {
        false
    }

    /// Returns a list of pseudolegal moves, that is, moves which can either be played using
    /// [`Self::make_move`] or which will cause `make_move` to return `None`.
    /// Note that an implementation is allowed to filter out illegal pseudolegal moves, so this function does not
    /// guarantee that e.g. all pseudolegal chess moves are being returned.
    fn pseudolegal_moves(&self) -> Self::MoveList {
        let mut moves = Self::MoveList::default();
        self.gen_pseudolegal(|m| moves.add_move(m));
        moves
    }

    /// Returns a list of pseudo legal moves that are considered "tactical", such as captures and promotions in chess.
    fn tactical_pseudolegal(&self) -> Self::MoveList {
        let mut moves = Self::MoveList::default();
        self.gen_tactical_pseudolegal(|m| moves.add_move(m));
        moves
    }

    /// Generate pseudolegal moves and calls the provided callable for each move.
    /// This doesn't handle a forced passing move in case of no legal moves, unlike
    /// [`Self::pseudolegal_moves`]. It should therefore be considered a very low-level function.
    fn gen_pseudolegal(&self, callback: impl FnMut(Self::Move));

    /// Generate moves that are considered "tactical" into the supplied move list.
    /// Generic over the move list, like [`Self::gen_pseudolegal`].
    /// Note that some games don't consider any moves tactical, so this function may have no effect.
    fn gen_tactical_pseudolegal(&self, callback: impl FnMut(Self::Move));

    /// Returns a list of legal moves, that is, moves that can be played using `make_move`
    /// and will not return `None`.
    /// Some variants require a passing move if there are no legal moves and the game isn't over.
    /// Like [`Self::pseudolegal_moves`], this function honors that requirement by inserting a `Move::default()`,
    /// unlike [`Self::gen_pseudolegal`] (which can't know if there are no legal moves).
    fn legal_moves_slow(&self) -> Self::MoveList {
        let mut res = self.pseudolegal_moves();
        if Self::Move::legality(self.settings()) == PseudoLegal {
            res.filter_moves(|m| self.is_pseudolegal_move_legal(*m));
        }
        res
    }

    /// Returns the number of pseudolegal moves. Can sometimes be implemented more efficiently
    /// than generating all pseudolegal moves and counting their number.
    /// Returns 1 in case of a forced passing move.
    fn num_pseudolegal_moves(&self) -> usize {
        let mut ctr = 0;
        self.gen_pseudolegal(|_| ctr += 1);
        ctr
    }

    /// Returns the number of legal moves. Automatically falls back to [`Self::num_pseudolegal_moves`] for games
    /// with legal movegen.
    /// Returns `1` in case of a forced passing move.
    fn num_legal_moves(&self) -> usize {
        if Self::Move::legality(self.settings()) == Legal {
            self.num_pseudolegal_moves()
        } else {
            let mut ctr = 0;
            self.gen_pseudolegal(|m| {
                if self.is_pseudolegal_move_legal(m) {
                    ctr += 1;
                }
            });
            if ctr == 0 && self.no_moves_result().is_none() {
                ctr += 1; // forced passing move
            }
            ctr
        }
    }

    /// Returns 'true' if there are no legal moves, i.e. if `num_legal_moves()` would return 0.
    /// Can sometimes be implemented more efficiently
    fn has_no_legal_moves(&self) -> bool {
        self.num_legal_moves() == 0
    }

    /// Returns a random legal move, that is, chooses a pseudorandom move from the set of legal moves.
    /// Can be implemented by generating all legal moves and randomly sampling one, so it's potentially
    /// `random_pseudolegal_move`
    fn random_legal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move>;

    /// Returns a random pseudolegal move.
    /// Like [`Self::gen_pseudolegal`], this doesn't handle forced passing moves.
    fn random_pseudolegal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move>;

    /// Assumes pseudolegal movegen, returns None in case of an illegal pseudolegal move,
    /// like ignoring a check in chess. Not meant to return None on moves that never make sense,
    /// like moving to a square outside the board (in that case, the function should panic).
    /// In other words, this function only gracefully checks legality assuming that the move is pseudolegal.
    // TODO: make_move_cloned or similar
    fn make_move(self, mov: Self::Move) -> Option<Self>;

    /// Makes a nullmove, i.e. flips the active player. While this action isn't strictly legal in most games,
    /// it's still very useful and necessary for null move pruning.
    fn make_nullmove(self) -> Option<Self>;

    /// Like [`Self::make_move`], but if `mov` is null it calls [`Self::make_nullmove`].
    fn make_move_or_nullmove(self, mov: Self::Move) -> Option<Self> {
        if mov.is_null() { self.make_nullmove() } else { self.make_move(mov) }
    }

    /// See [`Self::is_move_pseudolegal`]. However, this function assumes that the move is pseudolegal
    /// for some unknown position, usually because it has been generated in the past and saved, but it is no
    /// longer certain that it is indeed pseudolegal for the current position. Therefore, this function can sometimes
    /// be implemented slightly more efficiently than [`Self::is_move_pseudolegal`].
    fn is_generated_move_pseudolegal(&self, mov: Self::Move) -> bool {
        self.is_move_pseudolegal(mov)
    }

    /// Returns true iff the move is pseudolegal, that is, it can be played with [`Self::make_move`] without
    /// causing a panic. When it is not certain that a move is definitely (pseudo)legal for the current position,
    /// `Untrusted<Move>` should be used.
    /// Note that it is possible for a move to be considered pseudolegal even though [`Self::pseudolegal_moves`]
    /// would not generate it (but such a move would never be legal)
    fn is_move_pseudolegal(&self, mov: Self::Move) -> bool;

    /// Returns true iff the move is legal, that is, if it is pseudolegal and playing it with `make_move`
    /// would return `Some` new board. `is_move_pseudolegal` can be much faster.
    fn is_move_legal(&self, mov: Self::Move) -> bool {
        // the call to `is_pseudolegal_move_legal` should get inlined, after which it should evaluate to `true` for
        // boards with legal movegen
        self.is_move_pseudolegal(mov) && self.is_pseudolegal_move_legal(mov)
    }

    /// Expects a pseudolegal move and returns if this move is also legal, which means that playing it with
    /// [`Self::make_move`] returns `Some(new_board)`
    fn is_pseudolegal_move_legal(&self, mov: Self::Move) -> bool {
        Self::Move::legality(self.settings()) == Legal || self.clone().make_move(mov).is_some()
    }

    /// Returns the result (win/draw/loss), if any, but doesn't necessarily catch all game-ending conditions.
    /// That is, this function might return `None` if the game has actually ended,
    fn player_result_no_movegen<H: BoardHistory>(&self, history: &H) -> Option<PlayerResult>;

    /// Returns the result (win/draw/loss), if any. Can be potentially slow because it can require movegen.
    /// If movegen is used anyway (such as in an ab search), it is usually better to call [`Self::player_result_no_movegen`]
    /// and [`Self::no_moves_result`] iff there were no legal moves, which is done in the [`Self::player_result`] function.
    /// Despite the name, this method is not always slower than `player_result_no_movegen`, for some games both
    /// implementations are identical.
    /// Note that many implementations never return [`PlayerResult::Win`] because the active player can't win the game,
    /// which is the case because the current player is flipped after the winning move.
    /// For example, being checkmated in chess is a loss for the current player.
    fn player_result_slow<H: BoardHistory>(&self, history: &H) -> Option<PlayerResult>;

    /// Only called when there are no legal moves.
    /// In that case, the function returns the game state from the current player's perspective.
    /// Note that this doesn't check that there are indeed no legal moves to avoid paying the performance cost of that.
    /// This assumes that having no legal moves available automatically ends the game.
    /// If this function returns `None`, the player must pass (with [`Self::make_nullmove`]). This is required to be legal.
    fn no_moves_result(&self) -> Option<PlayerResult>;

    /// Returns true iff the game is lost for the player who can now move, like being checkmated in chess.
    /// Using [`Self::player_result_no_movegen()`] and [`Self::no_moves_result()`] is often the faster option if movegen is needed anyway
    fn is_game_lost_slow<H: BoardHistory>(&self, history: &H) -> bool {
        self.player_result_slow(history).is_some_and(|x| x == Lose)
    }

    /// Returns true iff the game is a draw.
    /// Similarly to [`Self::is_game_lost_slow`], using [`Self::player_result_no_movegen`] and [`Self::no_moves_result`] is often faster.
    fn is_draw_slow<H: BoardHistory>(&self, history: &H) -> bool {
        self.player_result_slow(history).is_some_and(|x| x == Draw)
    }

    /// Returns true iff the game is won for the current player after making the given move.
    /// This move has to be pseudolegal. If the move will likely be played anyway, it can be faster
    /// to play it and use [`Self::player_result()`] or [`Self::player_result_no_movegen()`] and [`Self::no_moves_result`] instead.
    fn is_game_won_after_slow<H: BoardHistory>(&self, mov: Self::Move, mut history: H) -> bool {
        let Some(new_pos) = self.clone().make_move(mov) else {
            return false;
        };
        history.push(new_pos.hash_pos());
        new_pos.is_game_lost_slow(&history)
    }

    /// Returns `false` if it detects that `player` can not win the game except if the opponent runs out of time
    /// or makes "very dumb" mistakes.
    ///
    /// This is intended to be a comparatively cheap function and does not perform any kind of search.
    /// Typical cases where this returns false include chess positions where we only have our king left
    /// but the opponent still possesses enough material to mate (otherwise, the game would have ended in a draw).
    /// The result of this function on a position where [`Self::game_result_slow`] returns a `Some` is unspecified.
    /// This is an approximation; always returning `true` would be a valid implementation of this method.
    /// The implementation of this method for chess technically violates the FIDE rules (as does the insufficient material
    /// draw condition), but that shouldn't be a problem in practice -- this rule is only meant ot be applied in human games anyway,
    /// and the FIDE rules are effectively uncheckable.
    fn can_reasonably_win(&self, player: Self::Color) -> bool;

    /// The hash of this position. E.g. for chess, this is the zobrist hash.
    fn hash_pos(&self) -> PosHash;

    /// Like [`Self::from_fen`], but changes the `input` argument to contain the remaining input instead of panicking when there's
    /// any remaining input after reading the fen.
    fn read_fen_and_advance_input(input: &mut Tokens, strictness: Strictness) -> Res<Self> {
        Self::read_fen_and_advance_input_for(input, strictness, Self::SettingsRef::default())
    }

    /// Like [`Self::read_fen_and_advance_input`], but if the input doesn't contain any leading settings information
    /// (like the variant for [`FairyBoard`], or the size for [`MnkBoard`]) it uses the provided settings instead of the default settings.
    fn read_fen_and_advance_input_for(
        input: &mut Tokens,
        strictness: Strictness,
        settings: Self::SettingsRef,
    ) -> Res<Self>;

    /// How the board should be displayed in diagrams and in pretty format.
    /// Specifically, how and when it should be flipped and if the 2 axes should be labelled with letters or numbers.
    /// By default, the board is always shown from the first player's POV. Files are labelled with letters and ranks with numbers.
    fn axes_format(&self) -> AxesFormat {
        AxesFormat::default()
    }

    /// Some games have alternative FEN
    fn as_alternative_fen(&self) -> Option<String> {
        None
    }

    /// Returns an ASCII (or unicode) art representation of the board.
    /// This is not meant to return a FEN, but instead a diagram where the pieces
    /// are identified by their letters in algebraic notation.
    /// Rectangular boards can implement this with the `[board_to_string]` function
    fn as_diagram(&self, typ: CharType, flip: bool, mark_active: bool) -> String;

    /// Returns a text-based representation of the board that's intended to look pretty.
    /// This can be implemented by calling `as_ascii_diagram` or `as_unicode_diagram`, but the intention
    /// is for the output to contain more information, like using colors to show the last move.
    /// Rectangular boards can implement this with the `[display_board_pretty]` function
    fn display_pretty(&self, formatter: &mut dyn BoardFormatter<Self>) -> String;

    /// Allows boards to customize how they want to be formatted.
    /// For example, the [`Chessboard`] can give the king square a red frame if the king is in check.
    fn pretty_formatter(
        &self,
        piece: Option<CharType>,
        last_move: Option<Self::Move>,
        opts: OutputOpts,
    ) -> Box<dyn BoardFormatter<Self>>;

    /// The background color of the given coordinates, e.g. the color of the square of a chessboard.
    /// For rectangular boards, this can often be implemented with `coords.square_color()`,
    /// but it's also valid to always return `White`.
    // TODO: Maybe each board should be able to define its own square color enum?
    fn background_color(&self, coords: Self::Coordinates) -> SquareColor;

    /// Returns a map from name to bitboard.
    /// For example, for chess, `"checkers"` returns a bitboard of pieces that give check.
    /// Game-independent bitboards like `"active"` or the pieces are handled in [`bitboard_from_name`], which is
    /// called by this default implementation. Board implementations can change this list, usually by adding game-specific names.
    fn bitboard_from_name(&self) -> BBSelect<Self>;
}

/// This trait contains associated functions that can be called on `Board` instances but shouldn't be overridden by `Board` implementations.
/// The purpose of splitting them out into their own trait is to make implementing `Board` less confusing.
pub trait BoardHelpers: BoardTrait {
    /// Returns the name of the game, such as 'chess'.
    #[must_use]
    fn game_name() -> String {
        Self::static_short_name().to_string()
    }

    /// For each color, returns a single ASCII char decribing it, e.g. `w` and `b` for black.
    fn color_chars(&self) -> [char; 2] {
        [Self::Color::first().to_char(self.settings()), Self::Color::second().to_char(self.settings())]
    }

    fn color_names(&self) -> [String; 2] {
        [
            Self::Color::first().name(self.settings()).to_string(),
            Self::Color::second().name(self.settings()).to_string(),
        ]
    }

    /// The bitboard of bits that don't correspond to squares, which happens if the board size isn't the same
    /// as the number of bits in the bitboard type.
    fn invalid_square_bb(&self) -> Self::RawBitboard {
        !self.valid_squares_bb()
    }

    /// The player who cannot currently move.
    fn inactive_player(&self) -> Self::Color {
        self.active_player().other()
    }

    /// The 0-based number of moves (turns) since the start of the game.
    fn fullmove_ctr_0_based(&self) -> usize {
        (self.halfmove_ctr_since_start().saturating_sub(usize::from(!self.active_player().is_first()))) / 2
    }

    /// The 1-based number of moves(turns) since the start of the game.
    /// This format is used in FENs.
    fn fullmove_ctr_1_based(&self) -> usize {
        1 + self.fullmove_ctr_0_based()
    }

    /// The number of squares of the board.
    fn num_squares(&self) -> usize {
        self.size().num_squares()
    }

    /// Are these coordinates occupied, i.e., not empty?
    fn is_occupied(&self, coords: Self::Coordinates) -> bool {
        !self.is_empty(coords)
    }

    /// Returns an iterator over all the positions after making a legal move.
    /// Not very useful for search because it doesn't allow changing the order of generated positions and isn't quite as fast as
    /// a manual loop, but convenient for some use cases, for example [`perft`](crate::general::perft::perft).
    /// Like [`Self::legal_moves_slow`], this handles forced passing moves.
    fn children(&self) -> impl Iterator<Item = Self> + Send {
        let iter = self.pseudolegal_moves().into_iter();
        ChildrenIter { pos: self, iter, num_so_far: 0 }
    }

    /// Returns an optional [`MatchResult`]. Unlike a [`PlayerResult`], a [`MatchResult`] doesn't contain `Win` or `Lose` variants,
    /// but instead `P1Win` and `P1Lose`. Also, it contains a [`GameOverReason`].
    fn match_result_slow<H: BoardHistory>(&self, history: &H) -> Option<MatchResult> {
        let player_res = self.player_result_slow(history)?;
        let game_over = GameOver { result: player_res, reason: GameOverReason::Normal };
        Some(player_res_to_match_res(game_over, self.active_player()))
    }

    /// Convenience function that computes the player result by calling [`Self::no_moves_result()`] if `no_legal_moves` is true,
    /// else it calls [`Self::player_result_no_movegen()`].
    fn player_result<H: BoardHistory>(&self, history: &H, no_legal_moves: bool) -> Option<PlayerResult> {
        if no_legal_moves { self.no_moves_result() } else { self.player_result_no_movegen(history) }
    }

    /// Reads in a compact textual description of the board, such that `B::from_fen(board.as_fen()) == board` holds.
    /// Assumes that the entire string represents the FEN, without any trailing tokens.
    /// Use the lower-level [`Self::read_fen_and_advance_input`] if this assumption doesn't have to hold.
    /// To print a board as fen, the [`Display`] implementation should be used.
    fn from_fen(string: &str, strictness: Strictness) -> Res<Self> {
        let mut words = tokens(string);
        let res = Self::read_fen_and_advance_input(&mut words, strictness)
            .map_err(|err| anyhow!("Failed to parse FEN '{}': {err}", string.bold()))?;
        if let Some(next) = words.next() {
            return Err(anyhow!(
                "Input `{0}' contains additional characters after FEN, starting with '{1}'",
                string.bold(),
                next.red()
            ));
        }
        Ok(res)
    }

    /// Returns a compact textual description of the board that can be read in again with `from_fen`.
    /// Same as `self.to_string()`.
    fn as_fen(&self) -> String {
        self.to_string()
    }

    /// Verifies that all invariants of this board are satisfied. It should never be possible for this function to
    /// fail for a bug-free program; failure most likely means the `Board` implementation is bugged.
    /// For checking invariants that might be violated, use a [`BoardTrait::Unverified`] and call [`BoardTrait::Unverified::verify_with_level`].
    fn debug_verify_invariants(&self, strictness: Strictness) -> Res<Self> {
        let verified = Self::Unverified::new(self.clone()).verify_with_level(Assertion, strictness)?;
        ensure!(verified == *self, "Recalculated data doesn't match: Should be \n'{verified:?}' but is \n'{self:?}'");
        Ok(verified)
    }

    /// Parses a move using [`MoveTrait::from_text`], then applies it on this board and returns the result.
    fn make_move_from_str(self, text: &str) -> Res<Self> {
        let mov = Self::Move::from_text(text, &self)?;
        self.clone().make_move(mov).ok_or_else(|| {
            anyhow!(
                "Move '{}' is pseudolegal but not legal in position '{self}'",
                mov.extended_formatter(&self, Standard, None).to_string().red()
            )
        })
    }

    /// Place a piece of the given type and color on the given square. Doesn't check that the resulting position is
    /// legal (hence the `Unverified` return type), but can still fail if the piece can't be placed because e.g. there
    /// is already a piece on that square. See [`UnverifiedBoardTrait::try_place_piece`] and [`Self::replace_piece`].
    fn place_piece(self, piece: Self::Piece) -> Res<Self::Unverified> {
        let mut res = Self::Unverified::new(self);
        res.try_place_piece(piece)?;
        Ok(res)
    }

    /// Remove a piece from the given square. See [`UnverifiedBoardTrait::try_remove_piece`].
    fn remove_piece(self, square: Self::Coordinates) -> Res<Self::Unverified> {
        let mut res = Self::Unverified::new(self);
        res.try_remove_piece(square)?;
        Ok(res)
    }

    /// Like `[Self::place_piece`], but if the target isn't empty, it just replaces the piece.
    fn replace_piece(self, piece: Self::Piece) -> Res<Self::Unverified> {
        let mut res = Self::Unverified::new(self);
        res.try_replace_piece(piece.coordinates(), piece.colored_piece_type())?;
        Ok(res)
    }

    /// Set the active player. See [`UnverifiedBoardTrait::set_active_player`].
    fn set_active_player(self, new_active: Self::Color) -> Self::Unverified {
        let mut res = Self::Unverified::new(self);
        res.set_active_player(new_active);
        res
    }

    /// Set the ply counter since the start of the game. See [`UnverifiedBoardTrait::set_ply_since_start`]
    fn set_ply_since_start(self, ply: usize) -> Res<Self::Unverified> {
        let mut res = Self::Unverified::new(self);
        res.set_ply_since_start(ply)?;
        Ok(res)
    }
}

impl<B: BoardTrait> BoardHelpers for B {}

pub trait RectangularBoard: BoardTrait<Coordinates: RectangularCoordinates> {
    fn height(&self) -> DimT;

    fn width(&self) -> DimT;

    fn get_width(&self) -> usize {
        self.width() as usize
    }

    fn get_height(&self) -> usize {
        self.height() as usize
    }

    fn idx_to_coordinates(&self, idx: DimT) -> Self::Coordinates;
}

impl<B: BoardTrait> RectangularBoard for B
where
    B::Coordinates: RectangularCoordinates,
{
    fn height(&self) -> DimT {
        self.size().height().0
    }
    fn width(&self) -> DimT {
        self.size().width().0
    }

    fn idx_to_coordinates(&self, idx: DimT) -> Self::Coordinates {
        self.size().idx_to_coordinates(idx)
    }
}

#[allow(type_alias_bounds)]
pub type BBSelect<B: BoardTrait> = Vec<GenericSelect<B::RawBitboard>>;

pub fn default_bitboards_from_name<B: BitboardBoard>(pos: &B) -> BBSelect<B> {
    let dest_squares = pos.calc_move_dest_bb().raw();
    let mut t: Vec<GenericSelect<B::RawBitboard>> = vec![
        GenericSelect::full("active", Some("us"), "Pieces of the active player", pos.active_player_bb().raw()),
        GenericSelect::full("inactive", Some("them"), "Pieces of the inactive player", pos.inactive_player_bb().raw()),
        GenericSelect::full("empty", None, "All empty squares", pos.empty_bb().raw()),
        GenericSelect::full("occupied", None, "All non-empty squares", pos.occupied_bb().raw()),
        GenericSelect::full("all", None, "All squares on the board", pos.mask_bb().raw()),
        GenericSelect::full("can_move_to", Some("dest"), "Squares which the active player can move to", dest_squares),
    ];
    let s = pos.settings();
    for p in PieceTypeOf::<B>::non_empty(s) {
        t.push(GenericSelect {
            name: p.name(s).to_string(),
            alternative_name: Some(p.to_char(Ascii, s).to_string()),
            description: Some(format!("Bitboard of squares occupied by a {}", p.name(s).to_string())),
            val: pos.piece_bb(p).raw(),
        })
    }
    for c in B::Color::iter() {
        t.push(GenericSelect {
            name: c.name(s).to_string(),
            alternative_name: Some(c.to_char(s).to_string()),
            description: Some(format!("Bitboard of squares occupied by {}", c.name(s))),
            val: pos.player_bb(c).raw(),
        });
    }
    let size = pos.size();
    let width = size.width().get();
    for file in 0..width {
        let file_name = pos.axes_format().ith_x_axis_entry(file, width, None, false);
        t.push(GenericSelect {
            name: format!("file_{file_name}"),
            alternative_name: None,
            description: Some(format!("Bitboard of all squares on the {file_name} file")),
            val: B::Bitboard::file_for(file, size).raw(),
        })
    }
    let height = size.height().get();
    for rank in 0..height {
        let rank_name = pos.axes_format().ith_y_axis_entry(rank, height, None, false);
        t.push(GenericSelect {
            name: format!("rank_{rank_name}"),
            alternative_name: None,
            description: Some(format!("Bitboard of all squares on rank {rank_name}")),
            val: B::Bitboard::rank_for(rank, size).raw(),
        })
    }
    for sq in size.valid_coordinates() {
        t.push(GenericSelect {
            name: format!("square_{sq}"),
            alternative_name: None,
            description: Some(format!("Bitboard where the only set bit corresponds to {sq}")),
            val: B::RawBitboard::single_piece_at(size.internal_key(sq)),
        })
    }
    t
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub enum BoardOrientation {
    Normal,    // the board as seen by the first player
    PlayerPov, // the y-axis is flipped if the board is seen from the second player's POV
}

/// Whether to use letters ('a'..) or numbers (1..) for printing the rank / file, and whether those should be flipped so that
/// they're counting right-to-left ur top-down instead of the usual direction.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Arbitrary)]
pub enum AxisSymbol {
    Letter,
    LetterReversed,
    Number,
    NumberReversed,
}

/// How diagrams and pretty unicode/ascii representations should orient the board and display the axes.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub struct AxesFormat {
    pub orientation: BoardOrientation,
    pub x_axis_symbol: AxisSymbol,
    pub y_axis_symbol: AxisSymbol,
}

impl Default for AxesFormat {
    fn default() -> Self {
        Self {
            orientation: BoardOrientation::Normal,
            x_axis_symbol: AxisSymbol::Letter,
            y_axis_symbol: AxisSymbol::Number,
        }
    }
}

impl AxesFormat {
    pub fn player_pov() -> Self {
        Self { orientation: BoardOrientation::PlayerPov, ..Default::default() }
    }

    pub fn is_usi_format(&self) -> bool {
        self.x_axis_symbol == AxisSymbol::NumberReversed && self.y_axis_symbol == AxisSymbol::LetterReversed
    }

    fn ith_entry_for(
        self,
        i: DimT,
        max: DimT,
        flip: bool,
        axis: AxisSymbol,
        fmt_width: Option<usize>,
        center: bool,
    ) -> impl Display {
        let mut flip = flip && self.orientation == BoardOrientation::PlayerPov;
        flip ^= matches!(axis, AxisSymbol::LetterReversed | AxisSymbol::NumberReversed);
        // we need to handle invalid coordinates here so that we can print error messages
        if i >= max {
            return AxisEntry::Char('�', fmt_width.unwrap_or_default(), center);
        }
        let num = if flip { max - 1 - i } else { i };
        match axis {
            AxisSymbol::Letter | AxisSymbol::LetterReversed => {
                let mut c = file_to_char(num);
                if fmt_width.is_some() {
                    c = c.to_ascii_uppercase();
                }
                AxisEntry::Char(c, fmt_width.unwrap_or_default(), center)
            }
            AxisSymbol::Number | AxisSymbol::NumberReversed => {
                AxisEntry::Num(num + 1, fmt_width.unwrap_or_default(), center)
            }
        }
    }

    pub fn ith_x_axis_entry(self, i: DimT, width: DimT, sq_width: Option<usize>, flip: bool) -> impl Display {
        self.ith_entry_for(i, width, flip, self.x_axis_symbol, sq_width, true)
    }

    pub fn ith_y_axis_entry(self, i: DimT, height: DimT, sq_width: Option<usize>, flip: bool) -> impl Display {
        self.ith_entry_for(i, height, flip, self.y_axis_symbol, sq_width, false)
    }
}

enum AxisEntry {
    Char(char, usize, bool),
    Num(DimT, usize, bool),
}

impl Display for AxisEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            AxisEntry::Char(x, w, center) => {
                if center {
                    write!(f, "{x:^w$}")
                } else {
                    write!(f, "{x:>w$}")
                }
            }
            AxisEntry::Num(x, w, center) => {
                if center {
                    write!(f, "{x:^w$}")
                } else {
                    write!(f, "{x:>w$}")
                }
            }
        }
    }
}

/// A trait for [`BoardTrait`]s that use [`BitboardTrait`]s.
/// Bitboards are small bitvector representations of sets of squares.
/// This trait mainly exists to make implementing new games easier, because
/// implementing this trait provides some default implementations,
/// but those *might not be optimal* depending on the internal representation.
/// Games with neutral pieces should override the [`Self::neutral_bb`] function.
// There is no actual reason why bitboards would require rectangular coordinates,
// but currently all boards are rectangular and lifting this restriction would need a bit of restructuring.
pub trait BitboardBoard: BoardTrait<Coordinates: RectangularCoordinates> {
    type Bitboard: BitboardTrait<Self::RawBitboard, Self::Coordinates>;

    /// Bitboard of all pieces of the given [`PieceType`], independent of which player they belong to.
    /// Note that it might not be valid to use the empty piece, if such a piece exists.
    fn piece_bb(&self, piece: PieceTypeOf<Self>) -> Self::Bitboard;

    /// Bitboard of all pieces of the given type and color, e.g. all black rooks in chess.
    /// Note that it might not be valid to use the empty piece, if such a piece exists.
    // TODO: Remove empty from pieces, use options
    fn col_piece_bb(&self, color: Self::Color, piece: PieceTypeOf<Self>) -> Self::Bitboard {
        self.piece_bb(piece) & self.player_bb(color)
    }

    /// Bitboard of all pieces of a player.
    fn player_bb(&self, color: Self::Color) -> Self::Bitboard;

    /// Bitboard of all pieces of the active player.
    fn active_player_bb(&self) -> Self::Bitboard {
        self.player_bb(self.active_player())
    }

    /// Bitboard of all pieces of the inactive player.
    fn inactive_player_bb(&self) -> Self::Bitboard {
        self.player_bb(self.inactive_player())
    }

    /// Bitboard of all squares that contain a "piece" that doesn't belong to any player, like a gap in ataxx.
    /// Empty squares don't count, so this bitboard is always zero for most games.
    fn neutral_bb(&self) -> Self::Bitboard {
        self.zero_bb()
    }

    /// Bitboard of all pieces, i.e. all non-empty squares.
    fn occupied_bb(&self) -> Self::Bitboard {
        let first_bb = self.player_bb(Self::Color::first());
        let second_bb = self.player_bb(Self::Color::second());
        let neutral_bb = self.neutral_bb();
        debug_assert!((first_bb & second_bb).is_zero());
        debug_assert!(((first_bb | second_bb) & neutral_bb).is_zero());
        first_bb | second_bb | self.neutral_bb()
    }

    /// Bitboard of all empty squares.
    fn empty_bb(&self) -> Self::Bitboard {
        !self.occupied_bb() & self.mask_bb()
    }

    /// Bitboard (with correct size) where all bits are zero.
    fn zero_bb(&self) -> Self::Bitboard {
        Self::Bitboard::new(Self::RawBitboard::zero(), self.size())
    }

    /// On many boards, not all bits of a bitboard correspond to squares.
    /// This bitboard has ones on all squares and zeros otherwise.
    fn mask_bb(&self) -> Self::Bitboard;

    /// Returns a bitboard of all squares that a piece of the active player can move to, which corresponds to "threats" in games like chess.
    /// The default implementation is general but slow and can sometimes be implemented much more efficiently.
    fn calc_move_dest_bb(&self) -> Self::Bitboard {
        let mut res = self.zero_bb();
        let size = self.size();
        for m in self.legal_moves_slow().iter_moves() {
            res |= Self::Bitboard::single_piece_for(m.dest_square_in(self), size);
        }
        res
    }
}

#[must_use]
pub fn ply_counter_from_fullmove_nr(fullmove_nr: NonZeroUsize, is_active_first: bool) -> usize {
    (fullmove_nr.get() - 1) * 2 + usize::from(!is_active_first)
}

/// Constructs a specific, well-known position from its name, such as 'kiwipete' in chess.
/// Not to be confused with `from_fen`, which can load arbitrary positions.
/// However, `"fen <x>"` forwards to [`B::from_fen`].
/// A free function instead of a default impl for [`B::from_name`] because Rust doesn't allow calling default impls in overriding impls.
pub fn board_from_name<B: BoardTrait>(name: &str) -> Res<B> {
    let mut tokens = tokens(name);
    let first_token = tokens.next().unwrap_or_default();
    if first_token.eq_ignore_ascii_case("fen") {
        return B::from_fen(&tokens.string(), Relaxed);
    } else if first_token.eq_ignore_ascii_case("startpos") {
        return Ok(B::startpos());
    }
    select_name_static(name, B::name_to_pos_map().iter(), "position", &B::game_name(), NoDescription)
        .map(|f| f.create())
}

pub fn position_fen_part<B: RectangularBoard>(f: &mut Formatter<'_>, pos: &B) -> fmt::Result {
    for y in (0..pos.height()).rev() {
        let mut empty_ctr = 0;
        for x in 0..pos.width() {
            let piece = pos.colored_piece_on(B::Coordinates::from_rank_file(y, x));
            if piece.is_empty() {
                empty_ctr += 1;
            } else {
                if empty_ctr > 0 {
                    write!(f, "{empty_ctr}")?;
                }
                empty_ctr = 0;
                piece.colored_piece_type().write_as_str(pos.settings(), CharType::Ascii, false, f)?;
            }
        }
        if empty_ctr > 0 {
            write!(f, "{empty_ctr}")?;
        }
        if y > 0 {
            write!(f, "/")?;
        }
    }
    Ok(())
}

pub fn common_fen_part<B: RectangularBoard>(f: &mut Formatter<'_>, pos: &B) -> fmt::Result {
    position_fen_part(f, pos)?;
    write!(f, " {}", pos.active_player().to_char(pos.settings()))
}

pub fn simple_fen<B: RectangularBoard>(pos: &B, halfmove: bool, fullmove: bool) -> impl Display {
    SimpleFenFormatter { pos, halfmove, fullmove }
}

struct SimpleFenFormatter<'a, B: RectangularBoard> {
    pos: &'a B,
    halfmove: bool,
    fullmove: bool,
}

impl<B: RectangularBoard> Display for SimpleFenFormatter<'_, B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        common_fen_part(f, self.pos)?;
        if self.halfmove {
            write!(f, " {}", self.pos.ply_draw_clock())?;
        }
        if self.fullmove {
            write!(f, " {}", self.pos.fullmove_ctr_1_based())?;
        }
        Ok(())
    }
}

fn read_position_fen_impl<B: RectangularBoard>(
    lines: Split<char>,
    board: &mut B::Unverified,
    lines_read: &mut usize,
) -> Res<()> {
    const SHOGI_PROMO: char = '+';
    const CH_PROMO: u8 = b'~';
    let mut square = 0;
    for (line, line_num) in lines.zip(0_usize..) {
        *lines_read += 1;
        let square_before_line = square;
        debug_assert_eq!(square_before_line, line_num * board.size().width().val());

        let handle_skipped = |digits_start: Option<usize>, digits_end, idx: &mut usize| {
            if let Some(start) = digits_start {
                let num = line[start..digits_end].trim_end_matches(SHOGI_PROMO).parse::<usize>()?;
                if num == 0 {
                    bail!("FEN position can't contain the number 0".to_string())
                }
                *idx = idx.saturating_add(num);
            }
            Ok(())
        };
        let mut skip = false;
        let mut shogi_promo = false;
        let mut digit_start = None;

        for (i, c) in line.char_indices() {
            if skip {
                skip = false;
                continue;
            } else if c.is_ascii_digit() {
                digit_start = Some(digit_start.unwrap_or(i));
                continue;
            } else if c == SHOGI_PROMO {
                shogi_promo = true;
                continue;
            }
            let Some(mut symbol) = ColPieceTypeOf::<B>::from_char(c, board.settings()) else {
                bail!(
                    "Invalid character in {0} FEN position description (not a piece): {1}",
                    B::game_name(),
                    c.to_string().red()
                )
            };
            handle_skipped(digit_start, i, &mut square)?;
            digit_start = None;
            if square >= board.size().num_squares() {
                bail!(
                    "FEN position contains more than {square} squares, but the board only has {0} squares",
                    board.size().num_squares()
                );
            }
            if line.as_bytes().get(i + 1).copied().is_some_and(|c| c == CH_PROMO) {
                symbol.make_promoted(board.settings()).map_err(|err| {
                    let modifier = String::from_utf8(vec![CH_PROMO]).unwrap();
                    anyhow!(
                        "{err} (the trailing '{0}' in '{1}' is interpreted as crazyhouse-style promotion modifier)",
                        modifier.bold(),
                        format!("{c}{modifier}").red()
                    )
                })?;
                skip = true;
            } else if shogi_promo {
                symbol.make_promoted(board.settings()).map_err(|err| {
                    anyhow!(
                        "{err} (the leading '{0}' in '{1}' is interpreted as shogi-style promotion modifier)",
                        SHOGI_PROMO.to_string().bold(),
                        format!("{SHOGI_PROMO}{c}").red()
                    )
                })?;
                shogi_promo = false;
            }

            board.place_piece(board.size().idx_to_coordinates(square as DimT).flip_up_down(board.size()), symbol);
            square += 1;
        }
        handle_skipped(digit_start, line.len(), &mut square)?;
        let line_len = square - square_before_line;
        if line_len != board.size().width().val() {
            bail!("Line '{line}' has incorrect width: {line_len}, should be {0}", board.size().width().val());
        }
    }
    Ok(())
}

pub(crate) fn read_position_fen<B: RectangularBoard>(mut position: &str, board: &mut B::Unverified) -> Res<()> {
    if board.fen_pos_part_contains_hand() {
        let Some((pos, hand)) = position.split_once('[') else {
            bail!(
                "The position token of the FEN ('{0}') has to end with a hand part enclosed in [brackets]",
                position.red()
            );
        };
        position = pos;
        let Some(hand) = hand.strip_suffix(']') else {
            bail!("If the position description of the FEN contains a '[', it must end with a ']'");
        };
        board.read_fen_hand_part(hand)?;
    } else if position.ends_with(']') {
        bail!(
            "The position token of the FEN ('{0}') ends with a bracket, but this format is not supposed to contain a hand part",
            position.red()
        )
    }
    let lines = position.split('/');
    let mut num_lines = 0;
    let height = board.size().height().val();
    let res = read_position_fen_impl::<B>(lines.clone(), board, &mut num_lines);

    debug_assert!(num_lines > 0);
    if num_lines != height {
        if num_lines == 1 {
            let msg = if let Err(e) = res { format!(": {e}") } else { String::default() };
            bail!(
                "Expected a FEN position description of {height} lines separated by '{0}', but found '{1}'{msg}",
                "/".bold(),
                position.red()
            )
        } else if num_lines == height + 1 {
            // try to parse this as lichess notation, where the hand is given as an additional row
            let (fen, hand) = position.rsplit_once('/').unwrap();
            if board.read_fen_hand_part(hand).is_ok() {
                num_lines = 0;
                return read_position_fen_impl::<B>(fen.split('/'), board, &mut num_lines);
            }
        }
        // If parsing the fen failed, the number of lines isn't accurate
        if res.is_ok() {
            bail!(
                "The {0} board has a height of {1}, but the FEN contains {2} rows",
                B::game_name(),
                height.to_string().bold(),
                num_lines.to_string().bold()
            )
        }
    }
    res
}

/// Reads the position and active player part
pub(crate) fn read_common_fen_part<B: RectangularBoard>(words: &mut Tokens, board: &mut B::Unverified) -> Res<()> {
    let Some(position_part) = words.next() else { bail!("Empty {0} FEN string", B::game_name()) };
    read_position_fen::<B>(position_part, board)?;

    let Some(active) = words.next() else {
        bail!("{0} FEN ends after the position description and doesn't include the active player", B::game_name())
    };
    let [c1, c2] = [B::Color::first().to_char(board.settings()), B::Color::second().to_char(board.settings())];
    ensure!(
        active.chars().count() == 1,
        "Expected a single char to describe the active player ('{0}' or '{1}'), got '{2}'",
        c1.to_string().bold(),
        c2.to_string().bold(),
        active.red()
    );
    let Some(active) = B::Color::from_char(active.chars().next().unwrap(), board.settings()) else {
        bail!(
            "Expected '{0}' or '{1}' for the color, not '{2}'",
            c1.to_string().bold(),
            c2.to_string().bold(),
            active.red()
        )
    };
    board.set_active_player(active);
    Ok(())
}

#[allow(unused)] // suppress warning when building only chess
pub(crate) fn read_halfmove_clock<B: RectangularBoard>(words: &mut Tokens, board: &mut B::Unverified) -> Res<()> {
    let Some(halfmove_clock) = words.peek().copied() else { return board.set_halfmove_repetition_clock(0) };
    let halfmove_clock =
        halfmove_clock.parse::<usize>().map_err(|err| anyhow!("Couldn't parse halfmove clock: {err}"))?;
    _ = words.next();
    board.set_halfmove_repetition_clock(halfmove_clock)?;
    Ok(())
}

pub(crate) fn read_two_move_numbers<B: RectangularBoard>(
    words: &mut Tokens,
    board: &mut B::Unverified,
    strictness: Strictness,
) -> Res<()> {
    let halfmove_clock = words.peek().copied().unwrap_or("");
    // Some FENs don't contain the halfmove clock and fullmove number, so assume that's the case if parsing
    // the halfmove clock fails -- but don't do this for the fullmove number.
    if let Ok(halfmove_clock) = halfmove_clock.parse::<usize>() {
        _ = words.next();
        board.set_halfmove_repetition_clock(halfmove_clock)?;
        let Some(fullmove_number) = words.peek().copied() else {
            bail!("The FEN contains a valid halfmove clock ('{halfmove_clock}') but no fullmove counter",)
        };
        let fullmove_number = fullmove_number
            .parse::<NonZeroUsize>()
            .map_err(|err| anyhow!("Couldn't parse fullmove counter '{}': {err}", fullmove_number.red()))?;
        _ = words.next();
        board.set_ply_since_start(ply_counter_from_fullmove_nr(fullmove_number, board.active_player().is_first()))?;
    } else if strictness == Strict {
        bail!("FEN doesn't contain a halfmove clock and fullmove counter, but they are required in strict mode")
    } else {
        board.set_halfmove_repetition_clock(0)?;
        board.set_ply_since_start(usize::from(!board.active_player().is_first()))?;
    }
    Ok(())
}

#[allow(unused)]
pub(crate) fn read_single_move_number<B: RectangularBoard>(
    words: &mut Tokens,
    board: &mut B::Unverified,
    strictness: Strictness,
    plyctr_fallback: Option<usize>,
) -> Res<()> {
    let fullmove_nr = words.next().unwrap_or("");
    let plyctr = || {
        let fullmove_nr = match fullmove_nr.parse::<NonZeroUsize>() {
            Ok(n) => n,
            Err(_) => {
                if strictness == Strict {
                    bail!("FEN doesn't contain a valid fullmove counter, but that is required in strict mode")
                } else if let Some(plyctr) = plyctr_fallback {
                    return Ok(plyctr);
                }
                NonZeroUsize::new(1).unwrap()
            }
        };
        Ok(ply_counter_from_fullmove_nr(fullmove_nr, board.active_player().is_first()))
    };
    board.set_ply_since_start(plyctr()?)
}

#[allow(unused)]
pub(crate) fn read_move_number_in_ply<B: RectangularBoard>(
    words: &mut Tokens,
    board: &mut B::Unverified,
    strictness: Strictness,
) -> Res<()> {
    let ply_ctr = words.next().unwrap_or_default();
    let ctr = match ply_ctr.parse::<NonZeroUsize>() {
        Ok(n) => n.get() - 1,
        Err(_) => {
            if strictness == Strict {
                bail!("FEN doesn't contain a valid ply counter, but that is required in strict mode")
            }
            0
        }
    };
    board.set_ply_since_start(ctr)
}

struct ChildrenIter<'a, B: BoardTrait> {
    pos: &'a B,
    iter: MoveIter<B>,
    num_so_far: usize,
}

impl<B: BoardTrait> ChildrenIter<'_, B> {
    fn next_impl(&mut self) -> Option<B> {
        loop {
            let mov = self.iter.next()?;
            if let Some(child) = self.pos.clone().make_move(mov) {
                self.num_so_far += 1;
                return Some(child);
            }
        }
    }
}

impl<B: BoardTrait> Iterator for ChildrenIter<'_, B> {
    type Item = B;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(res) = self.next_impl() {
            Some(res)
        } else if self.num_so_far == 0 && self.pos.no_moves_result().is_none() {
            // forced passing move
            self.num_so_far = 1;
            Some(
                self.pos
                    .clone()
                    .make_nullmove()
                    .expect("When there are no legal moves and the game isn't over, a passing move must be legal"),
            )
        } else {
            None
        }
    }
}

impl<B: BoardTrait> ExactSizeIterator for ChildrenIter<'_, B> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<B: BoardTrait> FusedIterator for ChildrenIter<'_, B> {}
