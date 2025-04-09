use anyhow::{anyhow, bail, ensure};
use colored::Colorize;
use itertools::Itertools;
use static_assertions::const_assert_eq;
use std::cmp::min;
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::mem::size_of;
use strum_macros::EnumIter;

use crate::games::PlayerResult::Draw;
use crate::games::mnk::MnkPieceType::{Empty, O, X};
use crate::games::*;
use crate::general::bitboards::{Bitboard, DynamicallySizedBitboard, ExtendedRawBitboard, MAX_WIDTH, RawBitboard};
use crate::general::board::SelfChecks::CheckFen;
use crate::general::board::Strictness::{Relaxed, Strict};
use crate::general::board::{
    BoardHelpers, NameToPos, RectangularBoard, SelfChecks, Strictness, Symmetry, UnverifiedBoard, board_from_name,
    read_common_fen_part, read_single_move_number, simple_fen,
};
use crate::general::common::*;
use crate::general::hq::BitReverseSliderGenerator;
use crate::general::move_list::EagerNonAllocMoveList;
use crate::general::moves::Legality::Legal;
use crate::general::moves::{Legality, Move, UntrustedMove};
use crate::general::squares::{GridCoordinates, GridSize};
use crate::output::OutputOpts;
use crate::output::text_output::{BoardFormatter, DefaultBoardFormatter, board_to_string, display_board_pretty};
use crate::search::Depth;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
#[must_use]
pub enum MnkPieceType {
    X = 0,
    O = 1,
    #[default]
    Empty = 2,
}

impl From<MnkColor> for MnkPieceType {
    fn from(value: MnkColor) -> Self {
        match value {
            MnkColor::X => X,
            MnkColor::O => O,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, Hash, EnumIter, Arbitrary)]
#[must_use]
pub enum MnkColor {
    #[default]
    X,
    O,
}

impl Display for MnkColor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            MnkColor::X => write!(f, "x"),
            MnkColor::O => write!(f, "o"),
        }
    }
}

impl Not for MnkColor {
    type Output = Self;

    fn not(self) -> Self::Output {
        self.other()
    }
}

impl Color for MnkColor {
    type Board = MNKBoard;

    fn second() -> Self {
        MnkColor::O
    }

    fn to_char(self, _settings: &<Self::Board as Board>::Settings) -> char {
        match self {
            MnkColor::X => 'x',
            MnkColor::O => 'o',
        }
    }

    fn name(self, _settings: &<Self::Board as Board>::Settings) -> impl AsRef<str> {
        match self {
            MnkColor::X => "X",
            MnkColor::O => "O",
        }
    }
}

const UNICODE_X: char = '⨉'; // '⨉',
const UNICODE_O: char = '◯'; // '○'

impl AbstractPieceType<MNKBoard> for MnkPieceType {
    fn empty() -> MnkPieceType {
        MnkPieceType::Empty
    }

    fn non_empty(_settings: &MnkSettings) -> impl Iterator<Item = Self> {
        [X, O].into_iter()
    }

    fn to_char(self, typ: CharType, _settings: &MnkSettings) -> char {
        match typ {
            Ascii => match self {
                X => 'X',
                O => 'O',
                Empty => '.',
            },
            Unicode => match self {
                X => UNICODE_X,
                O => UNICODE_O,
                Empty => '.',
            },
        }
    }

    fn to_display_char(self, typ: CharType, settings: &MnkSettings) -> char {
        self.to_char(typ, settings).to_ascii_uppercase()
    }

    fn from_char(c: char, _settings: &MnkSettings) -> Option<Self> {
        match c {
            ' ' => Some(Empty),
            'X' | UNICODE_X => Some(X),
            'O' | UNICODE_O => Some(O),
            _ => None,
        }
    }

    fn name(&self, _settings: &MnkSettings) -> impl AsRef<str> {
        match self {
            X => "x",
            O => "o",
            Empty => "empty",
        }
    }

    fn to_uncolored_idx(self) -> usize {
        self as usize
    }
}

impl PieceType<MNKBoard> for MnkPieceType {
    type Colored = MnkPieceType;

    fn from_idx(idx: usize) -> Self {
        match idx {
            0 => X,
            1 => O,
            2 => Empty,
            _ => panic!("trying to construct mnk piece from incorrect integer value"),
        }
    }
}

impl ColoredPieceType<MNKBoard> for MnkPieceType {
    type Uncolored = MnkPieceType;

    fn color(self) -> Option<MnkColor> {
        match self {
            X => Some(MnkColor::X),
            O => Some(MnkColor::O),
            Empty => None,
        }
    }

    fn to_colored_idx(self) -> usize {
        self as usize
    }

    fn new(color: MnkColor, uncolored: Self::Uncolored) -> Self {
        assert_eq!(uncolored.color().unwrap(), color);
        uncolored
    }
}

impl Display for MnkPieceType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.to_char(CharType::Unicode, &MnkSettings::default()))
    }
}

type MnkPiece = GenericPiece<MNKBoard, MnkPieceType>;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
#[repr(C)]
pub struct FillSquare {
    pub target: GridCoordinates,
    // pub player: Player,
}

const_assert_eq!(size_of::<FillSquare>(), 2);

impl Default for FillSquare {
    fn default() -> Self {
        FillSquare { target: GridCoordinates::no_coordinates() }
    }
}

impl Display for FillSquare {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.target)
    }
}

impl FillSquare {
    pub fn new(target: GridCoordinates) -> Self {
        FillSquare { target }
    }

    fn dest_square(self) -> GridCoordinates {
        self.target
    }
}

impl Move<MNKBoard> for FillSquare {
    type Underlying = u16;

    fn legality() -> Legality {
        Legal
    }

    fn src_square_in(self, _pos: &MNKBoard) -> Option<GridCoordinates> {
        None
    }

    fn dest_square_in(self, _pos: &MNKBoard) -> GridCoordinates {
        self.dest_square()
    }

    fn is_tactical(self, _board: &MNKBoard) -> bool {
        false
    }

    fn format_compact(self, f: &mut Formatter<'_>, _board: &MNKBoard) -> fmt::Result {
        write!(f, "{}", self.target)
    }

    fn parse_compact_text<'a>(s: &'a str, board: &MNKBoard) -> Res<(&'a str, FillSquare)> {
        let Some(mut square_str) = s.get(..2) else {
            bail!("m,n,k move '{}' doesn't start with a square consisting of two ASCII characters", s.red())
        };
        if s.as_bytes().get(2).is_some_and(|c| c.is_ascii_digit()) {
            square_str = &s[..3]; // m,n,k fens can contain two-digit files
        }
        let c = GridCoordinates::from_str(square_str)?;
        if !board.size().coordinates_valid(c) {
            bail!("The square {0} lies outside of the board (size: {1})", c.to_string().bold(), board.size())
        } else if !board.is_empty(c) {
            bail!("The square {} is already occupied, can only place on an empty square", c.to_string().bold())
        }
        Ok((&s[square_str.len()..], FillSquare { target: c }))
    }

    fn parse_extended_text<'a>(s: &'a str, board: &MNKBoard) -> Res<(&'a str, FillSquare)> {
        Self::parse_compact_text(s, board)
    }

    fn from_u64_unchecked(val: u64) -> UntrustedMove<MNKBoard> {
        UntrustedMove::from_move(Self {
            target: GridCoordinates::from_rank_file(((val >> 8) & 0xff) as DimT, (val & 0xff) as DimT),
        })
    }

    fn to_underlying(self) -> Self::Underlying {
        (u16::from(self.target.row) << 8) | u16::from(self.target.column)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Arbitrary)]
#[must_use]
pub struct MnkSettings {
    height: DimT,
    width: DimT,
    k: DimT,
}

impl MnkSettings {
    pub fn from_input(first: &str, rest: &mut Tokens) -> Res<Self> {
        let height = parse_int_from_str(first, "mnk height")?;
        let width = parse_int(rest, "mnk width")?;
        let k = parse_int(rest, "mnk k")?;
        let settings = MnkSettings { height, width, k };
        if !settings.check_invariants() {
            bail!("mnk invariants violated (at least one value is too large or too small)");
        }
        Ok(settings)
    }

    pub fn fen_part(self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{0} {1} {2}", self.height, self.width, self.k)
    }

    pub fn check_invariants(self) -> bool {
        self.height <= 26
            && self.width < MAX_WIDTH as DimT
            && self.height * self.width <= 128
            && self.height * self.width >= 1
            && self.k <= min(self.height, self.width)
            && self.k >= 1
    }

    pub fn titactoe() -> Self {
        Self { height: 3, width: 3, k: 3 }
    }

    // TODO: Connect4 rules
    pub fn connect4() -> Self {
        Self { height: 6, width: 7, k: 4 }
    }

    pub fn new(height: Height, width: Width, k: DimT) -> Self {
        Self::try_new(height, width, k).expect("The provided mnk values are invalid")
    }

    pub fn try_new(height: Height, width: Width, k: DimT) -> Option<Self> {
        let height = height.0;
        let width = width.0;
        let res = Self { height, width, k };
        if res.check_invariants() { Some(res) } else { None }
    }

    pub fn height(self) -> Height {
        Height(self.height)
    }

    pub fn width(self) -> Width {
        Width(self.width)
    }

    pub fn k(self) -> usize {
        self.k as usize
    }

    pub fn size(self) -> GridSize {
        GridSize::new(self.height(), self.width())
    }
}

impl Default for MnkSettings {
    fn default() -> Self {
        Self::titactoe()
    }
}

impl Display for MnkSettings {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.fen_part(f)
    }
}

impl Settings for MnkSettings {
    fn text(&self) -> Option<String> {
        Some(format!("[{} in a row to win]", self.k))
    }
}

pub type MnkBitboard = DynamicallySizedBitboard<ExtendedRawBitboard, GridCoordinates>;

type SliderGen<'a> = BitReverseSliderGenerator<'a, GridCoordinates, MnkBitboard>;

#[derive(Copy, Clone, Default, Debug, Arbitrary)]
#[must_use]
pub struct MNKBoard {
    x_bb: ExtendedRawBitboard,
    o_bb: ExtendedRawBitboard,
    ply: u32,
    active_player: MnkColor,
    settings: MnkSettings,
    last_move: Option<FillSquare>,
}

impl PartialEq<Self> for MNKBoard {
    fn eq(&self, other: &Self) -> bool {
        self.x_bb == other.x_bb
            && self.o_bb == other.o_bb
            && self.active_player == other.active_player
            && self.settings == other.settings
            && self.ply == other.ply
    }
}

impl Eq for MNKBoard {}

impl StaticallyNamedEntity for MNKBoard {
    fn static_short_name() -> impl Display
    where
        Self: Sized,
    {
        "mnk"
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        "m,n,k game".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "An m,n,k game is a generalization of games like Tic-Tac-Toe or Gomoku to boards of size mxn, where n in a row are needed to win".to_string()
    }
}

impl MNKBoard {
    pub fn x_bb(self) -> MnkBitboard {
        MnkBitboard::new(self.x_bb, self.size())
    }

    pub fn o_bb(self) -> MnkBitboard {
        MnkBitboard::new(self.o_bb, self.size())
    }

    pub fn player_bb(self, player: MnkColor) -> MnkBitboard {
        match player {
            MnkColor::X => self.x_bb(),
            MnkColor::O => self.o_bb(),
        }
    }

    pub fn active_player_bb(self) -> MnkBitboard {
        self.player_bb(self.active_player())
    }

    pub fn inactive_player_bb(self) -> MnkBitboard {
        self.player_bb(self.active_player().other())
    }

    pub fn occupied_bb(self) -> MnkBitboard {
        self.o_bb() | self.x_bb()
    }

    pub fn empty_bb(self) -> MnkBitboard {
        let n = self.num_squares() - 1;
        let res = !self.occupied_bb().raw() & (u128::MAX >> (127 - n));
        MnkBitboard::new(res, self.size())
    }

    pub fn k(self) -> u32 {
        self.settings.k as u32
    }

    fn make_move_for_player(mut self, mov: <Self as Board>::Move, player: MnkColor) -> Self {
        debug_assert!(self.is_move_pseudolegal(mov));
        let mut p = UnverifiedMnkBoard::new(self);
        p.place_piece(mov.target, player.into());
        self = p.0;
        self.ply += 1;
        self.last_move = Some(mov);
        self.active_player = player.other();
        self
    }
}

impl Display for MNKBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} ", self.settings,)?;
        simple_fen(f, self, false, true)
    }
}

impl Board for MNKBoard {
    type EmptyRes = MNKBoard;

    type Settings = MnkSettings;

    type Coordinates = GridCoordinates;
    type Color = MnkColor;

    type Piece = MnkPiece;

    type Move = FillSquare;

    type MoveList = EagerNonAllocMoveList<Self, 128>;

    type Unverified = UnverifiedMnkBoard;

    fn empty_for_settings(settings: MnkSettings) -> MNKBoard {
        Self::startpos_for_settings(settings)
    }

    fn startpos_for_settings(settings: MnkSettings) -> MNKBoard {
        assert!(settings.height <= 128);
        assert!(settings.width <= 128);
        assert!(settings.k <= 128);
        assert!(settings.k <= settings.height.min(settings.width));
        assert!(settings.height * settings.width <= 128);
        MNKBoard { ply: 0, x_bb: 0, o_bb: 0, settings, active_player: MnkColor::first(), last_move: None }
    }

    fn from_name(name: &str) -> Res<Self> {
        board_from_name(name).or_else(|err| {
            let parts = name.split(",").collect_vec();
            if parts.len() == 3 {
                let height = parse_int_from_str(parts[0], "m")?;
                let width = parse_int_from_str(parts[1], "n")?;
                let k = parse_int_from_str(parts[2], "k")?;
                let settings = MnkSettings { height, width, k };
                if !settings.check_invariants() {
                    bail!("Invalid m,n,k values (at least one value is too large or too small)");
                }
                Ok(Self::empty_for_settings(settings))
            } else {
                bail!(
                    "{0} It's also not an m,n,k list, which must have the format '{1}', e.g. '3,3,3'.",
                    err,
                    "<m>,<n>,<k>".bold()
                )
            }
        })
    }

    fn name_to_pos_map() -> EntityList<NameToPos> {
        vec![
            NameToPos { name: "large", fen: "11 11 4 11/11/11/11/11/11/11/11/11/11/11 x", strictness: Relaxed },
            NameToPos { name: "tictactoe", fen: "3 3 3 3/3/3 x 1", strictness: Strict },
        ]
    }

    fn bench_positions() -> Vec<Self> {
        let fens = &[
            "3 3 3 3/3/3 x",
            "5 5 4 X4/5/O2X1/O1X2/O4 x",
            "9 9 2 8X/9/9/9/9/9/9/9/9 o",
            "9 11 5 O1O3O4/9X1/7O3/7X3/11/4XX5/X7X2/OO3O4X/OX8X o",
            "20 5 4 5/5/5/5/1O3/5/5/5/5/5/1O3/X1X2/4O/1O3/5/5/1X2X/X4/5/5 o",
            "1 25 1 25 x",
            "3 26 2 26/26/10X15 o",
            "3 26 2 3X5O2XO12/7O12O2X1X/O9X6X8 o",
            "8 6 5 1X4/2XO2/2X3/1OX2O/OO1X1X/1O4/X4X/3O2 o",
            "10 8 5 2X3X1/X4O1X/2OXO2O/O1OO4/1X2X3/X2O2X1/OXOOXO1O/1X1XOO1X/5XO1/XOX1O2X x",
            "10 12 5 12/9X2/12/3O1O2X3/6OO4/4X2X4/3X8/O2O2X4X/4X4O2/12 o",
        ];
        fens.map(|f| Self::from_fen(f, Relaxed).unwrap()).into_iter().collect()
    }

    fn random_pos(rng: &mut impl Rng, strictness: Strictness, symmetry: Option<Symmetry>) -> Res<Self> {
        ensure!(
            symmetry.is_none(),
            "The m,n,k game implementation does not support setting a random symmetrical position"
        );
        loop {
            let height = rng.random_range(3..10);
            let width = rng.random_range(3..10);
            let k = rng.random_range(2..=min(height, width));
            let settings = MnkSettings::new(Height::new(height), Width::new(width), k as DimT);
            let mut pos = UnverifiedMnkBoard(Self::empty_for_settings(settings));
            let num_pieces = rng.random_range(0..settings.size().num_squares() - 1);
            for _ in 0..num_pieces {
                let empty = pos.0.empty_bb();
                let sq = ith_one_u128(rng.random_range(0..empty.num_ones()), empty.raw());
                let row = sq / width;
                let column = sq % width;
                let piece = if rng.random_bool(0.5) { O } else { X };
                pos.place_piece(GridCoordinates::from_rank_file(row as DimT, column as DimT), piece)
            }
            if rng.random_bool(0.5) {
                pos.0.active_player = !pos.0.active_player;
            }
            if let Ok(pos) = pos.verify(strictness) {
                return Ok(pos);
            }
        }
    }

    fn settings(&self) -> Self::Settings {
        self.settings
    }

    fn variant(first: &str, rest: &mut Tokens) -> Res<MNKBoard> {
        let settings = MnkSettings::from_input(first, rest)?;
        Ok(Self::startpos_for_settings(settings))
    }

    fn active_player(&self) -> MnkColor {
        self.active_player
    }

    fn halfmove_ctr_since_start(&self) -> usize {
        self.ply as usize
    }

    fn ply_draw_clock(&self) -> usize {
        0
    }

    fn size(&self) -> GridSize {
        GridSize { height: Height(self.settings.height), width: Width(self.settings.width) }
    }

    fn is_empty(&self, coords: Self::Coordinates) -> bool {
        let idx = self.size().internal_key(coords);
        // slightly faster than calling `empty_bb()`
        !self.occupied_bb().is_bit_set_at(idx)
    }

    fn colored_piece_on(&self, coordinates: Self::Coordinates) -> MnkPiece {
        let idx = self.size().internal_key(coordinates);
        debug_assert!(self.x_bb & self.o_bb == 0);
        let symbol = if self.x_bb.is_bit_set_at(idx) {
            X
        } else if self.o_bb.is_bit_set_at(idx) {
            O
        } else {
            Empty
        };
        MnkPiece::new(symbol, coordinates)
    }

    fn default_perft_depth(&self) -> Depth {
        let n = 1 + 1_000_000_f64.log(self.num_squares() as f64) as usize;
        Depth::new(n)
    }

    fn cannot_call_movegen(&self) -> bool {
        self.last_move_won_game()
    }

    fn gen_pseudolegal<T: MoveList<Self>>(&self, moves: &mut T) {
        let mut empty = self.empty_bb();
        while empty.has_set_bit() {
            let idx = empty.pop_lsb();
            if idx >= self.num_squares() {
                break; // TODO: Necessary?
            }
            let next_move = FillSquare { target: self.idx_to_coordinates(idx as DimT) };
            moves.add_move(next_move);
        }
    }

    fn gen_tactical_pseudolegal<T: MoveList<Self>>(&self, _moves: &mut T) {
        // currently, no moves are considered tactical
    }

    fn num_pseudolegal_moves(&self) -> usize {
        debug_assert!(!self.last_move_won_game(), "{self}");
        self.empty_bb().num_ones()
    }

    // Idea for another (faster and easier?) implementation:
    // Create lookup table (bitvector?) that answer "contains k consecutive 1s" for all
    // bits sequences of length 12 (= max m,n), use pext to transform columns and (anti) diagonals
    // into lookup indices.

    fn random_legal_move<T: Rng>(&self, rng: &mut T) -> Option<Self::Move> {
        let empty = self.empty_bb();
        debug_assert!(empty.ilog2() < self.num_squares() as u32);
        let num_empty = empty.count_ones() as usize;
        if num_empty == 0 || self.last_move_won_game() {
            return None;
        }
        let idx = rng.random_range(0..num_empty);
        let target = ith_one_u128(idx, empty.raw());

        Some(FillSquare { target: self.size().to_coordinates_unchecked(target) })
    }

    fn random_pseudolegal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
        self.random_legal_move(rng) // all pseudolegal moves are legal for m,n,k games
    }

    fn make_move(self, mov: Self::Move) -> Option<Self> {
        Some(self.make_move_for_player(mov, self.active_player()))
    }

    fn make_nullmove(mut self) -> Option<Self> {
        self.active_player = self.active_player.other();
        Some(self)
    }

    fn is_move_pseudolegal(&self, mov: Self::Move) -> bool {
        self.size().coordinates_valid(mov.target) && self.colored_piece_on(mov.target).symbol == Empty
    }

    fn player_result_no_movegen<H: BoardHistory>(&self, _history: &H) -> Option<PlayerResult> {
        // check for win before checking full board
        if self.last_move_won_game() {
            Some(Lose)
        } else if self.empty_bb().is_zero() {
            return Some(Draw);
        } else {
            None
        }
    }

    fn player_result_slow<H: BoardHistory>(&self, history: &H) -> Option<PlayerResult> {
        self.player_result_no_movegen(history)
    }

    fn no_moves_result(&self) -> PlayerResult {
        Draw
    }

    fn can_reasonably_win(&self, _player: MnkColor) -> bool {
        true
    }

    /// Not actually a zobrist hash function, but should work well enough
    fn hash_pos(&self) -> PosHash {
        let mut hasher = DefaultHasher::new();
        self.x_bb.hash(&mut hasher);
        self.o_bb.hash(&mut hasher);
        // we need to hash the side to move because search can play null moves
        self.active_player.hash(&mut hasher);
        PosHash(hasher.finish())
    }

    fn read_fen_and_advance_input(words: &mut Tokens, strictness: Strictness) -> Res<Self> {
        let Some(first) = words.next() else {
            bail!("Empty mnk fen".to_string());
        };
        let settings = MnkSettings::from_input(first, words)?;
        let board = MNKBoard::empty_for_settings(settings);
        let mut board = UnverifiedMnkBoard::new(board);
        read_common_fen_part::<MNKBoard>(words, &mut board)?;

        let mut ply = board.0.occupied_bb().num_ones();
        if (ply % 2 == 0) != board.active_player().is_first() {
            // This can't have happened in a normal game from startpos. Adjust ply so that converting to FEN and back
            // doesn't change the board.
            ply += 1;
        }
        board.set_ply_since_start(ply)?;
        read_single_move_number::<MNKBoard>(words, &mut board, strictness)?;

        board.0.last_move = None;

        board.verify_with_level(CheckFen, strictness)
    }

    fn should_flip_visually() -> bool {
        false
    }

    fn as_diagram(&self, typ: CharType, flip: bool) -> String {
        board_to_string(self, MnkPiece::to_char, typ, flip)
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

    fn background_color(&self, square: GridCoordinates) -> SquareColor {
        square.square_color()
    }
}

impl MNKBoard {
    fn last_move_won_game(&self) -> bool {
        // verifying a board (e.g. when parsing FENs) sets the last move in case the game is over
        if let Some(last_move) = self.last_move { self.is_game_won_at(last_move.target) } else { false }
    }

    fn is_game_won_at(&self, square: GridCoordinates) -> bool {
        let player = self.colored_piece_on(square).color();
        if player.is_none() {
            return false;
        }
        let player = player.unwrap();
        let player_bb = self.player_bb(player);
        let blockers = !self.player_bb(player);
        debug_assert!(
            (blockers.raw() & ExtendedRawBitboard::single_piece_at(self.size().internal_key(square))).is_zero()
        );
        let generator = SliderGen::new(blockers, None);

        let k = self.k() as usize - 1;

        (generator.horizontal_attacks(square) & player_bb).num_ones() >= k
            || (generator.vertical_attacks(square) & player_bb).num_ones() >= k
            || (generator.diagonal_attacks(square) & player_bb).num_ones() >= k
            || (generator.anti_diagonal_attacks(square) & player_bb).num_ones() >= k
    }
}

impl From<MNKBoard> for UnverifiedMnkBoard {
    fn from(board: MNKBoard) -> Self {
        Self(board)
    }
}

#[derive(Debug, Copy, Clone)]
#[must_use]
pub struct UnverifiedMnkBoard(MNKBoard);

impl UnverifiedBoard<MNKBoard> for UnverifiedMnkBoard {
    fn verify_with_level(self, level: SelfChecks, strictness: Strictness) -> Res<MNKBoard> {
        let mut this = self.0;
        let non_empty = this.occupied_bb().count_ones();
        // support custom starting positions where pieces have already been placed, as well as
        // adjustments of the ply based on the active player
        if this.ply > non_empty + 1 {
            bail!("Ply is {0}, but {non_empty} moves have been played", this.ply);
        } else if strictness == Strict {
            let diff = this.x_bb().num_ones() as isize - this.o_bb.num_ones() as isize;
            if this.ply != non_empty {
                bail!(
                    "In strict mode, the number of plies ({0}) has to be exactly the number of placed pieces ({non_empty})",
                    this.ply
                )
            } else if diff != isize::from(this.active_player == MnkColor::O) {
                bail!(
                    "In strict mode, the number of {X} and {O} must match, unless it's {O}'s turn, \
                    in which case there must be one more {X}. However that difference is {diff}"
                )
            }
        }
        if level != CheckFen && (this.o_bb & this.x_bb).has_set_bit() {
            bail!(
                "At least one square has two pieces on it (square {})",
                this.size().to_coordinates_unchecked((this.o_bb & this.x_bb).pop_lsb())
            );
        }
        if !this.settings.check_invariants() {
            bail!(
                "Invariants of settings are violated: m={0}, n={1}, k={2}",
                this.height(),
                this.width(),
                this.settings.k
            );
        }
        // FENs don't include the last move, and if the board has been modified, talking about the last move doesn't make
        // too much sense, either. Also, the last move is only used to detect if the game is over, but that's already handled
        // in this function, right below.
        this.last_move = None;
        for square in this.occupied_bb().one_indices() {
            let square = this.size().to_coordinates_unchecked(square);
            if this.is_game_won_at(square) {
                this.last_move = Some(FillSquare::new(square));
            }
        }
        // This still doesn't ensure that the position isn't won in multiple places, perhaps by both players.
        // But not giving an error for that seems fine; the game is over anyway.
        Ok(this)
    }

    fn settings(&self) -> MnkSettings {
        self.0.settings()
    }

    fn size(&self) -> GridSize {
        self.0.size()
    }

    fn place_piece(&mut self, sq: GridCoordinates, piece: MnkPieceType) {
        let placed_bb = ExtendedRawBitboard::single_piece_at(self.size().internal_key(sq));
        let bb = match piece {
            X => &mut self.0.x_bb,
            O => &mut self.0.o_bb,
            Empty => {
                return self.remove_piece(sq);
            }
        };
        *bb |= placed_bb;
    }

    fn remove_piece(&mut self, sq: GridCoordinates) {
        let mask = !ExtendedRawBitboard::single_piece_at(self.size().internal_key(sq));
        let this = &mut self.0;
        this.x_bb &= mask;
        this.o_bb &= mask;
        this.last_move = None;
        this.ply = 0;
    }

    fn piece_on(&self, coords: GridCoordinates) -> MnkPiece {
        self.0.colored_piece_on(coords)
    }

    fn is_empty(&self, square: GridCoordinates) -> bool {
        self.0.is_empty(square)
    }

    fn active_player(&self) -> MnkColor {
        self.0.active_player
    }

    fn set_active_player(&mut self, player: MnkColor) {
        self.0.active_player = player;
    }

    fn set_ply_since_start(&mut self, ply: usize) -> Res<()> {
        let ply = u32::try_from(ply).map_err(|err| anyhow!("Invalid ply number: {err}"))?;
        self.0.ply = ply;
        Ok(())
    }

    fn set_halfmove_repetition_clock(&mut self, _ply: usize) -> Res<()> {
        // ignored
        Ok(())
    }
}

/// lots of tests, which should probably go to their own file?
#[cfg(test)]
mod test {
    use crate::general::board::Strictness::Relaxed;
    use crate::general::perft::{perft, split_perft};
    use crate::search::Depth;

    use super::*;

    #[test]
    fn dimension_test() {
        let board = MNKBoard::default();
        assert_eq!(board.size().height.0, 3);
        assert_eq!(board.size().width.0, 3);
        assert_eq!(board.k(), 3);
        let board = MNKBoard::empty_for_settings(MnkSettings::new(Height(2), Width(5), 2));
        assert_eq!(board.size().width.0, 5);
        assert_eq!(board.size().height.0, 2);
        assert_eq!(board.k(), 2);
        let settings = MnkSettings::new(Height(12), Width(10), 6);
        assert_eq!(settings.width, 10);
        assert_eq!(settings.height, 12);
        assert_eq!(settings.k, 6);
        let board = MNKBoard::startpos_for_settings(settings);
        assert_eq!(board.settings, settings);
    }

    #[test]
    #[should_panic]
    fn dimension_test_invalid_k_0() {
        _ = MnkSettings::new(Height(4), Width(5), 0);
    }

    #[test]
    #[should_panic]
    fn dimension_test_invalid_k_too_large() {
        _ = MnkSettings::new(Height(4), Width(5), 6);
    }

    #[test]
    #[should_panic]
    fn dimension_test_invalid_zero_width() {
        _ = MnkSettings::new(Height(4), Width(0), 3);
    }

    #[test]
    #[should_panic]
    fn dimension_test_invalid_width_too_large() {
        _ = MnkSettings::new(Height(4), Width(33), 3);
    }

    #[test]
    #[should_panic]
    fn dimension_test_invalid_board_too_large() {
        _ = MnkSettings::new(Height(12), Width(11), 6);
    }

    // Only covers very basic cases, perft is used for mor more complex cases
    #[test]
    fn movegen_test() {
        let board = MNKBoard::empty_for_settings(MnkSettings::new(Height(4), Width(5), 2));
        let moves = board.pseudolegal_moves();
        assert_eq!(moves.len(), 20);
        assert_eq!(MNKBoard::from_name("10,9,7").unwrap().pseudolegal_moves().len(), 90);

        let mov: FillSquare = moves.iter().next().copied().unwrap();
        assert_eq!(moves.len(), 20);
        assert!(board.size().coordinates_valid(mov.target));
    }

    #[test]
    fn place_piece_test() {
        let board = MNKBoard::default();
        let mov = FillSquare { target: GridCoordinates::default() };
        assert_eq!(board.active_player(), MnkColor::first());
        let board = board.make_move(mov).unwrap();
        assert_eq!(board.size().num_squares(), 9);
        assert_eq!(board.x_bb, 1);
        assert_eq!(board.o_bb, 0);
        assert_eq!(board.ply, 1);
        assert_eq!(board.empty_bb().raw(), !1 & 0x1ff);
        assert_eq!(board.active_player(), MnkColor::second());
        assert!(!board.last_move_won_game());
        for k in [1, 2] {
            let board = MNKBoard::empty_for_settings(MnkSettings::new(Height(3), Width(4), k));
            let board = board.make_move(mov).unwrap();
            assert_eq!(k == 1, board.last_move_won_game());
            assert_ne!(board.x_bb().raw(), 0);
            assert_eq!(board.o_bb().raw(), 0);
            assert!(board.x_bb().is_single_piece());
            if k == 1 {
                assert!(board.last_move_won_game());
                assert_eq!(perft(Depth::new(1), board, false).nodes, 0);
            } else {
                assert_eq!(board.pseudolegal_moves().len() + 1, board.num_squares());
            }
        }
    }

    #[test]
    fn perft_startpos_test() {
        let r = perft(Depth::new(1), MNKBoard::default(), true);
        assert_eq!(r.depth.get(), 1);
        assert_eq!(r.nodes, 9);
        assert!(r.time.as_millis() <= 1); // 1 ms should be far more than enough even on a very slow device
        let r =
            split_perft(Depth::new(2), MNKBoard::empty_for_settings(MnkSettings::new(Height(8), Width(12), 2)), true);
        assert_eq!(r.perft_res.depth.get(), 2);
        assert_eq!(r.perft_res.nodes, 96 * 95);
        assert!(r.children.iter().all(|x| x.1 == r.children[0].1));
        assert!(r.perft_res.time.as_millis() <= 50);
        let r =
            split_perft(Depth::new(3), MNKBoard::empty_for_settings(MnkSettings::new(Height(4), Width(3), 3)), true);
        assert_eq!(r.perft_res.depth.get(), 3);
        assert_eq!(r.perft_res.nodes, 12 * 11 * 10);
        assert!(r.children.iter().all(|x| x.1 == r.children[0].1));
        assert!(r.perft_res.time.as_millis() <= 1000);
        let r =
            split_perft(Depth::new(5), MNKBoard::empty_for_settings(MnkSettings::new(Height(5), Width(5), 5)), false);
        assert_eq!(r.perft_res.depth.get(), 5);
        assert_eq!(r.perft_res.nodes, 25 * 24 * 23 * 22 * 21);
        assert!(r.children.iter().all(|x| x.1 == r.children[0].1));
        assert!(r.perft_res.time.as_millis() <= 10_000);

        let r = split_perft(Depth::new(9), MNKBoard::startpos_for_settings(MnkSettings::titactoe()), true);
        assert_eq!(r.perft_res.depth.get(), 9);
        assert!(r.perft_res.nodes >= 100_000);
        assert!(r.perft_res.nodes <= 9 * 8 * 7 * 6 * 5 * 4 * 3 * 2);
        for i in 0..9 {
            let mirrored = (i % 3) * 3 + i / 3;
            assert_eq!(r.children[mirrored].1, r.children[i].1);
        }
        assert!(r.perft_res.time.as_millis() <= 4000);

        let board = MNKBoard::empty_for_settings(MnkSettings::new(Height(2), Width(2), 2));
        let r = split_perft(Depth::new(3), board, false);
        assert_eq!(r.perft_res.depth.get(), 3);
        assert_eq!(r.perft_res.nodes, 2 * 3 * 4);
        assert!(r.children.iter().all(|x| x.1 == 2 * 3));
        assert!(r.perft_res.time.as_millis() <= 10);
        let r = perft(Depth::new(4), board, true);
        assert_eq!(r.depth.get(), 4);
        assert_eq!(r.nodes, 0);

        let board = MNKBoard::empty_for_settings(MnkSettings::new(Height(6), Width(7), 4));
        let expected = [1, 42, 42 * 41, 42 * 41 * 40];
        for (i, e) in expected.into_iter().enumerate() {
            let r = perft(Depth::new(i), board, false);
            assert_eq!(r.nodes, e);
            assert_eq!(r.depth, Depth::new(i));
        }
    }

    #[test]
    fn as_fen_test() {
        let board = MNKBoard::default();
        assert_eq!(board.ply, 0);
        let str = board.as_fen();
        assert_eq!(str, "3 3 3 3/3/3 x 1");

        let board = board.make_move(FillSquare { target: board.idx_to_coordinates(4) }).unwrap();
        assert_eq!(board.x_bb(), MnkBitboard::new(0x10, GridSize::tictactoe()));
        assert_eq!(board.colored_piece_on(board.size().to_coordinates_unchecked(4)).symbol, X);
        assert_eq!(board.ply, 1);
        assert_eq!(board.as_fen(), "3 3 3 3/1X1/3 o 1");

        let board = board.make_move_for_player(FillSquare { target: board.idx_to_coordinates(3) }, MnkColor::first());
        assert_eq!(board.as_fen(), "3 3 3 3/XX1/3 o 1");

        let board = board.make_move_for_player(FillSquare { target: board.idx_to_coordinates(5) }, MnkColor::second());
        assert_eq!(board.as_fen(), "3 3 3 3/XXO/3 x 2");

        let board = MNKBoard::empty_for_settings(MnkSettings { height: 3, width: 4, k: 2 });
        assert_eq!(board.as_fen(), "3 4 2 4/4/4 x 1");

        let board = board.make_move(FillSquare { target: board.idx_to_coordinates(0) }).unwrap();
        assert_eq!(board.as_fen(), "3 4 2 4/4/X3 o 1");

        let board = board.make_move(FillSquare { target: board.idx_to_coordinates(4) }).unwrap();
        assert_eq!(board.as_fen(), "3 4 2 4/O3/X3 x 2");

        let board = board.make_move(FillSquare { target: board.idx_to_coordinates(9) }).unwrap();
        assert_eq!(board.as_fen(), "3 4 2 1X2/O3/X3 o 2");

        let board = board.make_move(FillSquare { target: board.idx_to_coordinates(3) }).unwrap();
        assert_eq!(board.as_fen(), "3 4 2 1X2/O3/X2O x 3");
    }

    #[test]
    fn from_fen_test() {
        let board = MNKBoard::from_fen("4 3 2 3/3/3/3 x", Relaxed).unwrap();
        assert_eq!(board.occupied_bb().raw(), 0);
        assert_eq!(board.size(), GridSize::new(Height(4), Width(3)));
        assert_eq!(board.k(), 2);
        assert_eq!(board, MNKBoard::empty_for_settings(MnkSettings::new(Height(4), Width(3), 2)));

        let board = MNKBoard::from_fen("3 4 3 3X/4/2O1 x 2", Strict).unwrap();
        assert_eq!(board.occupied_bb().raw(), 0b1000_0000_0100);
        assert_eq!(
            board,
            MNKBoard {
                x_bb: 0b1000_0000_0000,
                o_bb: 0b0000_0000_0100,
                ply: 2,
                settings: MnkSettings::new(Height(3), Width(4), 3),
                active_player: MnkColor::first(),
                last_move: None
            }
        );

        let copy = MNKBoard::from_fen(&board.as_fen(), Relaxed).unwrap();
        assert_eq!(board, copy);

        let board = MNKBoard::from_fen("7 3 2 X1O/3/OXO/1X1/XO1/XXX/1OO o", Relaxed).unwrap();
        let white_bb = 0b001_000_010_010_001_111_000;
        let black_bb = 0b100_000_101_000_010_000_110;
        assert_eq!(
            board,
            MNKBoard {
                x_bb: white_bb,
                o_bb: black_bb,
                ply: 13,
                settings: MnkSettings::new(Height(7), Width(3), 2),
                active_player: MnkColor::second(),
                last_move: None
            }
        );
        assert_eq!(board, MNKBoard::from_fen(&board.as_fen(), Relaxed).unwrap());

        let board = MNKBoard::from_fen("4 12 3 12/11X/1X10/2X1X3XXX1 x", Relaxed).unwrap();
        let white_bb = 0b0000_0000_0000_1000_0000_0000_0000_0000_0010_0111_0001_0100;
        let black_bb = 0;
        assert_eq!(
            board,
            MNKBoard {
                x_bb: white_bb,
                o_bb: black_bb,
                ply: 8,
                settings: MnkSettings::new(Height(4), Width(12), 3),
                active_player: MnkColor::first(),
                last_move: None,
            }
        );
        println!("{board}");
        assert_eq!(board, MNKBoard::from_fen(&board.as_fen(), Relaxed).unwrap());
    }

    #[test]
    fn from_invalid_fen_test() {
        assert!(MNKBoard::from_fen("4 3 2 3/3/3/3 1 x", Strict).is_err());
        assert!(MNKBoard::from_fen("4 3 2 3/3/3/3 abc x", Relaxed).is_err());
        assert!(MNKBoard::from_fen("4 3 2 3/3/3/3", Relaxed).is_err());
        assert!(MNKBoard::from_fen("4 3 2 3/3/3/3 w", Relaxed).is_err());
        assert!(MNKBoard::from_fen("4 3 2 3/3/3/3 wx", Relaxed).is_err());
        let e = MNKBoard::from_fen("4 3 2 3/4/3/3 o", Relaxed);
        assert!(e.as_ref().is_err_and(|e| e.to_string().contains("Line '4' has incorrect width")), "{e:?}");
        _ = MNKBoard::from_fen("4 3 2 3//3/3 o", Relaxed).unwrap_err();
        assert!(MNKBoard::from_fen("4 3 2 x", Relaxed).is_err());
        assert!(MNKBoard::from_fen("4 0 2 /// x", Relaxed).is_err());
        _ = MNKBoard::from_fen("0 3 2 x", Relaxed).unwrap_err();
        assert!(MNKBoard::from_fen("4 3 2 4/4/4 o", Relaxed).is_err());
        assert!(MNKBoard::from_fen("4 3 3/3/3/3 x", Relaxed).is_err());
        assert!(MNKBoard::from_fen("3 13 2 13/12X/13/O12 x", Relaxed).is_err());
        assert!(MNKBoard::from_fen("12 12 2 12/12/12/12/12/12/12/12/12/12/12/12 o", Relaxed).is_err());
        assert!(MNKBoard::from_fen("3 3 3 3/X1O/11X o", Relaxed).is_err());
        assert!(MNKBoard::from_fen("3 3 3 3/X1O/F1X o", Relaxed).is_err());
        assert!(MNKBoard::from_fen("3 10 3 10/10/0XA x", Relaxed).is_err());
        assert!(MNKBoard::from_fen("3 3 3 3/3/0X2 o", Relaxed).is_err());
        assert!(MNKBoard::from_fen("3 3 3 3/-1X3/X2 x", Relaxed).is_err());
    }

    // perft and bench catch subtler problems, so only test fairly simple cases here
    #[test]
    fn test_winning() {
        let board = MNKBoard::from_fen("3 3 3 XX1/3/3 x", Relaxed).unwrap();
        assert_eq!(board.active_player(), MnkColor::first());

        assert!(board.is_game_won_after_slow(FillSquare { target: board.idx_to_coordinates(8) }));
        assert!(!board.is_game_won_after_slow(FillSquare { target: board.idx_to_coordinates(5) }));

        let board = MNKBoard::from_fen("4 3 3 XOX/O1O/XOO/1OX o", Relaxed).unwrap();
        assert!(board.is_game_won_after_slow(FillSquare { target: board.idx_to_coordinates(0) }));
        let board = MNKBoard::from_fen("3 3 3 XOX/O1O/XOO x", Relaxed).unwrap();
        assert!(board.is_game_won_after_slow(FillSquare { target: board.idx_to_coordinates(4) }));
        let board = MNKBoard::from_fen("4 3 3 XOX/OXO/XOO/1OX x", Relaxed).unwrap();
        assert!(!board.is_game_won_after_slow(FillSquare { target: board.idx_to_coordinates(0) }));
    }

    #[test]
    fn game_over_test() {
        let pos = MNKBoard::from_fen("3 3 3 XX1/3/3 x", Relaxed).unwrap();
        let mov = FillSquare::new(GridCoordinates::from_rank_file(2, 2));
        assert!(pos.is_game_won_after_slow(mov));
        let new_pos = pos.make_move(mov).unwrap();
        assert!(new_pos.last_move_won_game());
        assert_eq!(new_pos.last_move, Some(mov));
        assert_eq!(new_pos.player_result_slow(&NoHistory::default()), Some(Lose));
        assert_eq!(new_pos.active_player_bb(), MnkBitboard::new(0, pos.size()));
    }
}
