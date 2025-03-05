use crate::games::CharType::{Ascii, Unicode};
use crate::games::PlayerResult::Lose;
use crate::general::board::Board;
use crate::general::common::{parse_int, EntityList, Res, StaticallyNamedEntity, Tokens};
use crate::general::move_list::MoveList;
use crate::general::squares::{RectangularCoordinates, SquareColor};
use crate::output::OutputBuilder;
use crate::PlayerResult;
use anyhow::bail;
use arbitrary::Arbitrary;
use colored::Colorize;
use derive_more::{BitXor, BitXorAssign};
use rand::Rng;
use std::cmp::min;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::ops::Not;
use std::str::FromStr;
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
        } else if Self::second().to_char(settings).eq_ignore_ascii_case(&color) {
            Some(Self::second())
        } else {
            None
        }
    }

    fn from_name(name: &str, settings: &<Self::Board as Board>::Settings) -> Option<Self> {
        if Self::first().name(settings).as_ref().eq_ignore_ascii_case(name) {
            Some(Self::first())
        } else if Self::second().name(settings).as_ref().eq_ignore_ascii_case(name) {
            Some(Self::second())
        } else {
            let mut chars = name.chars();
            if let Some(c) = chars.next() {
                if chars.next().is_none() {
                    if c.eq_ignore_ascii_case(&Self::first().to_char(settings)) {
                        return Some(Self::first());
                    } else if c.eq_ignore_ascii_case(&Self::second().to_char(settings)) {
                        return Some(Self::second());
                    }
                }
            }
            None
        }
    }

    fn to_char(self, _settings: &<Self::Board as Board>::Settings) -> char;

    fn name(self, _settings: &<Self::Board as Board>::Settings) -> impl AsRef<str>;
}

/// Common parts of colored and uncolored piece types
// TODO: Remove default?
pub trait AbstractPieceType<B: Board>: Eq + Copy + Debug + Default {
    fn empty() -> Self;

    fn non_empty(_settings: &B::Settings) -> impl Iterator<Item = Self>;

    fn to_char(self, typ: CharType, _settings: &B::Settings) -> char;

    /// For asymmetrical games, we don't need uppercase/lowercase to distinguish between both players,
    /// so we always use uppercase letters for board diagrams. FENs continue to use lowercase letters.
    /// For chess, uncolored piece symbols are different from both white and black piece symbols, but
    /// used very rarely (and kind of ugly). So this maps to the much more common white piece version instead.
    fn to_display_char(self, typ: CharType, settings: &B::Settings) -> char {
        self.to_char(typ, settings)
    }

    /// When parsing chars, we don't distinguish between ascii and unicode symbols.
    /// Takes a board parameter because the `FairyBoard` pieces can change based on the rules
    fn from_char(c: char, _settings: &B::Settings) -> Option<Self>;

    /// Names for colored pieces don't have to (but can) include the color, i.e. `x` and `o` could be named `"x"` and `"o"` but
    /// white and black pawn could both be named `"pawn"`, but also `"white pawn"` and `"black pawn"`.
    fn name(&self, _settings: &B::Settings) -> impl AsRef<str>;

    fn from_name(name: &str, settings: &B::Settings) -> Option<Self> {
        for piece in Self::non_empty(&settings) {
            if piece.name(&settings).as_ref().eq_ignore_ascii_case(name) {
                return Some(piece);
            }
        }
        let mut chars = name.chars();
        if let Some(c) = chars.next() {
            if chars.next().is_none() {
                for piece in Self::non_empty(&settings) {
                    // don't ignore case because that's often used to distinguish between colors
                    if piece.to_char(Ascii, &settings) == c || piece.to_char(Unicode, &settings) == c {
                        return Some(piece);
                    }
                }
            }
        }
        None
    }

    fn to_uncolored_idx(self) -> usize;
}

pub trait PieceType<B: Board>: AbstractPieceType<B> {
    type Colored: ColoredPieceType<B>;

    fn from_idx(idx: usize) -> Self;
}

pub trait ColoredPieceType<B: Board>: AbstractPieceType<B> {
    type Uncolored: PieceType<B>;

    fn new(color: B::Color, uncolored: Self::Uncolored) -> Self;

    fn from_words(words: &mut Tokens, settings: &B::Settings) -> Res<Self> {
        let Some(piece) = words.next() else { bail!("Missing piece") };

        if let Some(piece) = Self::from_name(piece, &settings) {
            return Ok(piece);
        }
        let copied = words.clone();
        let second_word = words.next().unwrap_or_default();

        if let Some(color) = B::Color::from_name(piece, &settings) {
            if let Some(piece) = Self::Uncolored::from_name(second_word, &settings) {
                return Ok(Self::new(color, piece));
            }
        }
        // There are no pieces with more than 2 words
        let full_name = format!("{second_word} {}", words.peek().copied().unwrap_or_default());
        if let Some(res) = Self::from_name(&full_name, settings) {
            return Ok(res);
        }

        *words = copied;
        let pieces = itertools::intersperse(
            Self::non_empty(&settings).map(|piece| piece.name(&settings).as_ref().to_string()),
            ", ".to_string(),
        )
        .fold(String::new(), |a, b| a + &b);
        bail!("Unrecognized piece: '{0}', valid piece names are {pieces}", piece.red())
    }

    fn color(self) -> Option<B::Color>;

    fn uncolor(self) -> Self::Uncolored {
        Self::Uncolored::from_idx(self.to_uncolored_idx())
    }

    fn to_colored_idx(self) -> usize;
}

// TODO: Don't save coordinates in colored piece
pub trait ColoredPiece<B: Board>: Eq + Copy + Debug + Default
where
    B: Board<Piece = Self>,
{
    type ColoredPieceType: ColoredPieceType<B>;

    fn new(typ: Self::ColoredPieceType, square: B::Coordinates) -> Self;

    fn coordinates(self) -> B::Coordinates;

    fn uncolored(self) -> <Self::ColoredPieceType as ColoredPieceType<B>>::Uncolored {
        self.colored_piece_type().uncolor()
    }

    fn to_char(self, typ: CharType, settings: &B::Settings) -> char {
        self.colored_piece_type().to_char(typ, settings)
    }

    fn to_display_char(self, typ: CharType, settings: &B::Settings) -> char {
        self.colored_piece_type().to_display_char(typ, settings)
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

impl<B: Board<Piece = Self>, ColType: ColoredPieceType<B>> ColoredPiece<B> for GenericPiece<B, ColType> {
    type ColoredPieceType = ColType;

    fn new(typ: ColType, coordinates: B::Coordinates) -> Self {
        Self { symbol: typ, coordinates }
    }

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
        Self { symbol, coordinates }
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

/// On a rectangular board, coordinates are called `squares`.
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
pub trait Size<C: Coordinates>: Eq + PartialEq + Copy + Clone + Display + Debug + for<'a> Arbitrary<'a> {
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

#[derive(Copy, Clone, Eq, PartialEq, Default, Debug, derive_more::Display, BitXor, BitXorAssign, Arbitrary)]
#[must_use]
pub struct PosHash(pub u64);

pub trait Settings: Eq + Debug + Default {
    fn text(&self) -> Option<String> {
        None
    }
}

pub trait BoardHistory: Default + Debug + Clone + 'static {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn is_repetition(&self, hash: PosHash, plies_ago: usize) -> bool;
    fn push(&mut self, hash: PosHash);
    fn pop(&mut self);
    fn clear(&mut self);
    fn override_repetition_count(&self) -> Option<usize> {
        None
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct NoHistory {}

impl BoardHistory for NoHistory {
    fn len(&self) -> usize {
        0
    }

    fn is_repetition(&self, _hash: PosHash, _plies_ago: usize) -> bool {
        false
    }

    fn push(&mut self, _hash: PosHash) {}

    fn pop(&mut self) {}

    fn clear(&mut self) {}
}

#[derive(Clone, Eq, PartialEq, Default, Debug)]
#[must_use]
pub struct ZobristHistory(pub Vec<PosHash>);

impl ZobristHistory {
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }
}

impl BoardHistory for ZobristHistory {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn is_repetition(&self, hash: PosHash, plies_ago: usize) -> bool {
        hash == self.0[self.0.len() - plies_ago]
    }

    fn push(&mut self, hash: PosHash) {
        self.0.push(hash);
    }

    fn pop(&mut self) {
        _ = self.0.pop().expect("ZobristHistory::pop() called on empty history");
    }
    fn clear(&mut self) {
        self.0.clear();
    }
}

#[derive(Clone, Eq, PartialEq, Default, Debug)]
#[must_use]
pub struct ZobristHistory2Fold(pub ZobristHistory);

impl BoardHistory for ZobristHistory2Fold {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn is_repetition(&self, hash: PosHash, plies_ago: usize) -> bool {
        self.0.is_repetition(hash, plies_ago)
    }

    fn push(&mut self, hash: PosHash) {
        self.0.push(hash)
    }

    fn pop(&mut self) {
        self.0.pop()
    }

    fn clear(&mut self) {
        self.0.clear()
    }

    fn override_repetition_count(&self) -> Option<usize> {
        Some(2)
    }
}

pub fn n_fold_repetition<H: BoardHistory>(mut count: usize, history: &H, hash: PosHash, max_lookback: usize) -> bool {
    let stop = min(history.len(), max_lookback);
    count = history.override_repetition_count().unwrap_or(count);
    if stop < 2 {
        // in many, but not all, games, we could increase this to 4
        return false;
    }
    for i in (2..=stop).step_by(2) {
        if history.is_repetition(hash, i) {
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

    #[cfg(feature = "fairy")]
    #[test]
    fn generic_fairy_test() {
        GenericTests::<UtttBoard>::all_tests();
    }
}
