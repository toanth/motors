use anyhow::bail;
use arbitrary::Arbitrary;
use std::cmp::min;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Not;
use std::str::FromStr;

use crate::games::PlayerResult::Lose;
use crate::general::board::Board;
use crate::general::common::{parse_int, EntityList, Res, StaticallyNamedEntity};
use crate::general::move_list::MoveList;
use crate::general::squares::{RectangularCoordinates, SquareColor};
use crate::output::OutputBuilder;
use crate::PlayerResult;
use derive_more::{BitXor, BitXorAssign};
use rand::Rng;
use strum_macros::EnumIter;

#[cfg(feature = "ataxx")]
pub mod ataxx;
#[cfg(feature = "chess")]
pub mod chess;
#[cfg(feature = "fairy")]
pub mod fairy;
#[cfg(feature = "mnk")]
pub mod mnk;
#[cfg(feature = "uttt")]
pub mod uttt;

#[cfg(test)]
mod generic_tests;

#[derive(Debug, Copy, Clone, Eq, PartialEq, EnumIter)]
pub enum CharType {
    Ascii,
    Unicode,
}

pub trait Color: Debug + Default + Copy + Clone + PartialEq + Eq + Send + Hash + Not {
    type Board: Board<Color = Self>;

    #[must_use]
    fn other(self) -> Self {
        if self.is_first() {
            Self::second()
        } else {
            Self::first()
        }
    }

    fn first() -> Self {
        Self::default()
    }

    fn second() -> Self;

    fn is_first(self) -> bool {
        self == Self::first()
    }

    fn iter() -> impl Iterator<Item = Self> {
        [Self::first(), Self::second()].into_iter()
    }

    /// Takes a board parameter because the `FairyBoard` color can change based on the rules
    fn from_char(color: char, settings: &<Self::Board as Board>::Settings) -> Option<Self> {
        if Self::first().to_char(settings).eq_ignore_ascii_case(&color) {
            Some(Self::first())
        } else if Self::second()
            .to_char(settings)
            .eq_ignore_ascii_case(&color)
        {
            Some(Self::second())
        } else {
            None
        }
    }

    fn to_char(self, _settings: &<Self::Board as Board>::Settings) -> char;

    fn name(self, _settings: &<Self::Board as Board>::Settings) -> impl Display;
}

/// Common parts of colored and uncolored piece types
// TODO: Remove default?
pub trait AbstractPieceType<B: Board>: Eq + Copy + Debug + Default {
    fn empty() -> Self;

    fn to_char(self, typ: CharType, _settings: &B::Settings) -> char;

    /// For chess, uncolored piece symbols are different from both white and black piece symbols, but
    /// used very rarely (and kind of ugly). So this maps to the much more common black piece version,
    /// which is useful for text-based outputs that color the pieces themselves.
    fn to_default_utf8_char(self, settings: &B::Settings) -> char {
        self.to_char(CharType::Unicode, settings)
    }

    /// When parsing chars, we don't distinguish between ascii and unicode symbols.
    /// Takes a board parameter because the `FairyBoard` pieces can change based on the rules
    fn from_char(c: char, _pos: &B::Settings) -> Option<Self>;

    fn to_uncolored_idx(self) -> usize;
}

pub trait PieceType<B: Board>: AbstractPieceType<B> {
    type Colored: ColoredPieceType<B>;

    fn from_idx(idx: usize) -> Self;
}

pub trait ColoredPieceType<B: Board>: AbstractPieceType<B> {
    type Uncolored: PieceType<B>;

    fn color(self) -> Option<B::Color>;

    fn uncolor(self) -> Self::Uncolored {
        Self::Uncolored::from_idx(self.to_uncolored_idx())
    }

    fn to_colored_idx(self) -> usize;

    fn new(color: B::Color, uncolored: Self::Uncolored) -> Self;
}

// TODO: Don't save coordinates in colored piece
pub trait ColoredPiece<B: Board>: Eq + Copy + Debug + Default
where
    B: Board<Piece = Self>,
{
    type ColoredPieceType: ColoredPieceType<B>;
    fn coordinates(self) -> B::Coordinates;

    fn uncolored(self) -> <Self::ColoredPieceType as ColoredPieceType<B>>::Uncolored {
        self.colored_piece_type().uncolor()
    }

    fn to_char(self, typ: CharType, settings: &B::Settings) -> char {
        self.colored_piece_type().to_char(typ, settings)
    }

    fn is_empty(self) -> bool {
        self.colored_piece_type() == Self::ColoredPieceType::empty()
    }

    fn colored_piece_type(self) -> Self::ColoredPieceType;

    fn color(self) -> Option<B::Color> {
        self.colored_piece_type().color()
    }
}

#[derive(Eq, PartialEq, Default, Debug, Clone)]
#[must_use]
pub struct GenericPiece<B: Board, ColType: ColoredPieceType<B>> {
    symbol: ColType,
    coordinates: B::Coordinates,
}

impl<B: Board, ColType: ColoredPieceType<B>> Copy for GenericPiece<B, ColType> {}

impl<B: Board<Piece = Self>, ColType: ColoredPieceType<B>> ColoredPiece<B>
    for GenericPiece<B, ColType>
{
    type ColoredPieceType = ColType;

    fn coordinates(self) -> B::Coordinates {
        self.coordinates
    }

    fn colored_piece_type(self) -> Self::ColoredPieceType {
        self.symbol
    }
}

impl<B: Board, ColType: ColoredPieceType<B> + Display> Display for GenericPiece<B, ColType> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.symbol, f)
    }
}

impl<B: Board, ColType: ColoredPieceType<B>> GenericPiece<B, ColType> {
    pub fn new(symbol: ColType, coordinates: B::Coordinates) -> Self {
        Self {
            symbol,
            coordinates,
        }
    }
}

#[must_use]
pub fn file_to_char(file: DimT) -> char {
    debug_assert!(file < 26);
    (file + b'a') as char
}

#[must_use]
pub fn char_to_file(file: char) -> DimT {
    debug_assert!(file >= 'a');
    debug_assert!(file <= 'z');
    file as DimT - b'a'
}

/// On a rectangular board, coordinates are called 'squares'.
#[must_use]
pub trait Coordinates:
    Eq + Copy + Debug + Default + FromStr<Err = anyhow::Error> + Display + for<'a> Arbitrary<'a>
{
    type Size: Size<Self>;

    /// mirrors the coordinates vertically
    fn flip_up_down(self, size: Self::Size) -> Self;

    /// mirrors the coordinates horizontally
    fn flip_left_right(self, size: Self::Size) -> Self;
}

pub type DimT = u8;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Arbitrary)]
#[must_use]
pub struct Height(pub DimT);

impl Height {
    pub fn new(val: usize) -> Self {
        Self(DimT::try_from(val).unwrap())
    }
    #[must_use]
    pub fn get(self) -> DimT {
        self.0
    }
    #[must_use]
    pub fn val(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Arbitrary)]
#[must_use]
pub struct Width(pub DimT);

impl Width {
    pub fn new(val: usize) -> Self {
        Self(DimT::try_from(val).unwrap())
    }
    #[must_use]
    pub fn get(self) -> DimT {
        self.0
    }
    #[must_use]
    pub fn val(self) -> usize {
        self.0 as usize
    }
}

#[must_use]
pub trait Size<C: Coordinates>:
    Eq + PartialEq + Copy + Clone + Display + Debug + for<'a> Arbitrary<'a>
{
    fn num_squares(self) -> usize;

    /// Converts coordinates into an internal key. This function is injective, but **no further guarantees** are
    /// given. In particular, returned value do not have to be 0-based and do not have to be consecutive.
    /// E.g. for Ataxx, this returns the index of embedding the ataxx board into a 8x8 board.
    fn internal_key(self, coordinates: C) -> usize;

    /// Converts an internal key into coordinates, the inverse of `to_internal_key`.
    /// No further assumptions about which keys are valid should be made; in particular, there may be gaps in the set
    /// of valid keys (e.g. 4 and 12 might be valid, but 10 might not be). Although this function is safe in the rust
    /// sense, it doesn't guarantee any specified behavior for invalid keys.
    fn to_coordinates_unchecked(self, internal_key: usize) -> C;

    fn valid_coordinates(self) -> impl Iterator<Item = C>;

    fn coordinates_valid(self, coordinates: C) -> bool;

    #[inline]
    fn check_coordinates(self, coordinates: C) -> Res<C> {
        if self.coordinates_valid(coordinates) {
            Ok(coordinates)
        } else {
            bail!("Coordinates {coordinates} lie outside of the board (size {self})")
        }
    }
}

pub trait KnownSize<C: Coordinates>: Size<C> + Default {
    #[inline]
    fn num_squares() -> usize {
        Self::default().num_squares()
    }

    #[inline]
    fn internal_key(coordinates: C) -> usize {
        Self::default().internal_key(coordinates)
    }

    #[inline]
    fn coordinates_unchecked(internal_key: usize) -> C {
        Self::default().to_coordinates_unchecked(internal_key)
    }

    #[inline]
    fn valid_coordinates() -> impl Iterator<Item = C> {
        Self::default().valid_coordinates()
    }

    #[inline]
    fn coordinates_valid(coordinates: C) -> bool {
        Self::default().coordinates_valid(coordinates)
    }

    #[inline]
    fn check_coordinates(coordinates: C) -> Res<C> {
        Self::default().check_coordinates(coordinates)
    }
}

pub type OutputList<B> = EntityList<Box<dyn OutputBuilder<B>>>;

#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Default,
    Debug,
    derive_more::Display,
    BitXor,
    BitXorAssign,
    Arbitrary,
)]
#[must_use]
pub struct PosHash(pub u64);

pub trait Settings: Eq + Debug + Default {
    fn text(&self) -> Option<String> {
        None
    }
}

pub trait BoardHistory<B: Board>: Default + Debug + Clone + 'static {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn is_repetition(&self, board: &B, plies_ago: usize) -> bool;
    fn push(&mut self, board: &B);
    fn pop(&mut self);
    fn clear(&mut self);
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct NoHistory {}

impl<B: Board> BoardHistory<B> for NoHistory {
    fn len(&self) -> usize {
        0
    }

    fn is_repetition(&self, _board: &B, _plies_ago: usize) -> bool {
        false
    }

    fn push(&mut self, _board: &B) {}

    fn pop(&mut self) {}

    fn clear(&mut self) {}
}

#[derive(Clone, Eq, PartialEq, Default, Debug)]
#[must_use]
pub struct ZobristHistory<B: Board>(pub Vec<PosHash>, PhantomData<B>);

impl<B: Board> BoardHistory<B> for ZobristHistory<B> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn is_repetition(&self, pos: &B, plies_ago: usize) -> bool {
        pos.hash_pos() == self.0[self.0.len() - plies_ago]
    }

    fn push(&mut self, pos: &B) {
        self.0.push(pos.hash_pos());
    }

    fn pop(&mut self) {
        _ = self
            .0
            .pop()
            .expect("ZobristHistory::pop() called on empty history");
    }
    fn clear(&mut self) {
        self.0.clear();
    }
}

/// Compares the actual board states as opposed to only comparing the hashes. This still isn't always entirely correct --
/// For example, the FIDE rule state that the set of legal moves must be identical, which is not the case
/// if the ep square is set but the pawn is pinned and can't actually take.
#[derive(Debug, Default, Clone)]
#[must_use]
pub struct BoardCopyHistory<B: Board>(Vec<B>);

impl<B: Board> BoardHistory<B> for BoardCopyHistory<B> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn is_repetition(&self, board: &B, plies_ago: usize) -> bool {
        self.0[self.len() - plies_ago] == *board
    }

    fn push(&mut self, board: &B) {
        self.0.push(board.clone());
    }

    fn pop(&mut self) {
        self.0.pop();
    }

    fn clear(&mut self) {
        self.0.clear();
    }
}

pub fn n_fold_repetition<B: Board, H: BoardHistory<B>>(
    mut count: usize,
    history: &H,
    pos: &B,
    max_lookback: usize,
) -> bool {
    let stop = min(history.len(), max_lookback);
    if stop < 2 {
        // in many, but not all, games, we could increase this to 4
        return false;
    }
    for i in (2..=stop).step_by(2) {
        if history.is_repetition(pos, i) {
            count -= 1;
            if count <= 1 {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use crate::games::ataxx::AtaxxBoard;
    use crate::games::chess::Chessboard;
    use crate::games::generic_tests::GenericTests;
    use crate::games::mnk::MNKBoard;
    use crate::games::uttt::UtttBoard;

    #[cfg(feature = "chess")]
    #[test]
    fn generic_chess_test() {
        GenericTests::<Chessboard>::all_tests();
    }

    #[cfg(feature = "mnk")]
    #[test]
    fn generic_mnk_test() {
        GenericTests::<MNKBoard>::all_tests();
    }

    #[cfg(feature = "ataxx")]
    #[test]
    fn generic_ataxx_test() {
        GenericTests::<AtaxxBoard>::all_tests();
    }

    #[cfg(feature = "uttt")]
    #[test]
    fn generic_uttt_test() {
        GenericTests::<UtttBoard>::all_tests();
    }
}
