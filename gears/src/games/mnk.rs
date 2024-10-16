use anyhow::{anyhow, bail};
use regex::Regex;
use static_assertions::const_assert_eq;
use std::cmp::min;
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::mem::size_of;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::games::mnk::Symbol::{Empty, O, X};
use crate::games::PlayerResult::Draw;
use crate::games::*;
use crate::general::bitboards::{
    remove_ones_above, Bitboard, DefaultBitboard, ExtendedRawBitboard, RawBitboard, RayDirections,
    MAX_WIDTH,
};
use crate::general::board::SelfChecks::CheckFen;
use crate::general::board::Strictness::{Relaxed, Strict};
use crate::general::board::{
    board_from_name, position_fen_part, read_position_fen, NameToPos, RectangularBoard, SelfChecks,
    Strictness, UnverifiedBoard,
};
use crate::general::common::*;
use crate::general::move_list::EagerNonAllocMoveList;
use crate::general::moves::Legality::Legal;
use crate::general::moves::{Legality, Move, NoMoveFlags, UntrustedMove};
use crate::general::squares::{GridCoordinates, GridSize};
use crate::output::text_output::{
    board_to_string, display_board_pretty, BoardFormatter, DefaultBoardFormatter, PieceToChar,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum Symbol {
    X = 0,
    O = 1,
    #[default]
    Empty = 2,
}

impl From<MnkColor> for Symbol {
    fn from(value: MnkColor) -> Self {
        match value {
            MnkColor::X => X,
            MnkColor::O => O,
        }
    }
}

#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Default, Hash, derive_more::Display, EnumIter, Arbitrary,
)]
pub enum MnkColor {
    #[default]
    X,
    O,
}

impl Not for MnkColor {
    type Output = Self;

    fn not(self) -> Self::Output {
        self.other()
    }
}

impl Color for MnkColor {
    fn other(self) -> Self {
        match self {
            MnkColor::X => MnkColor::O,
            MnkColor::O => MnkColor::X,
        }
    }

    fn ascii_color_char(self) -> char {
        match self {
            MnkColor::X => 'x',
            MnkColor::O => 'o',
        }
    }

    fn utf8_color_char(self) -> char {
        match self {
            MnkColor::X => UNICODE_X,
            MnkColor::O => UNICODE_O,
        }
    }
}

const UNICODE_X: char = '⨉'; // '⨉',
const UNICODE_O: char = '◯'; // '○'

impl AbstractPieceType for Symbol {
    fn empty() -> Symbol {
        Symbol::Empty
    }

    fn to_ascii_char(self) -> char {
        match self {
            X => 'X',
            O => 'O',
            Empty => '.',
        }
    }

    fn to_utf8_char(self) -> char {
        match self {
            X => UNICODE_X,
            O => UNICODE_O,
            Empty => '.',
        }
    }

    fn from_utf8_char(c: char) -> Option<Self> {
        match c {
            ' ' => Some(Empty),
            'X' | UNICODE_X => Some(X),
            'O' | UNICODE_O => Some(O),
            _ => None,
        }
    }

    fn to_uncolored_idx(self) -> usize {
        self as usize
    }
}

impl PieceType<MNKBoard> for Symbol {
    type Colored = Symbol;

    fn from_idx(idx: usize) -> Self {
        match idx {
            0 => X,
            1 => O,
            2 => Empty,
            _ => panic!("trying to construct mnk piece from incorrect integer value"),
        }
    }
}

impl ColoredPieceType<MNKBoard> for Symbol {
    type Uncolored = Symbol;

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

impl Display for Symbol {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.to_utf8_char())
    }
}

type Square = GenericPiece<MNKBoard, Symbol>;

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
        FillSquare {
            target: GridCoordinates::no_coordinates(),
        }
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
}

impl Move<MNKBoard> for FillSquare {
    type Flags = NoMoveFlags;
    type Underlying = u16;

    fn legality() -> Legality {
        Legal
    }

    fn src_square(self) -> GridCoordinates {
        GridCoordinates::no_coordinates()
    }

    fn dest_square(self) -> GridCoordinates {
        self.target
    }

    fn flags(self) -> NoMoveFlags {
        NoMoveFlags {}
    }

    fn is_tactical(self, _board: &MNKBoard) -> bool {
        false
    }

    fn format_compact(self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.target)
    }

    fn from_compact_text(s: &str, pos: &MNKBoard) -> Res<Self> {
        let c = GridCoordinates::from_str(s)?;
        if !pos.size().coordinates_valid(c) {
            bail!(
                "The square {0} lies outside of the board (size: {1})",
                c.to_string().important(),
                pos.size()
            )
        } else if !pos.is_empty(c) {
            bail!(
                "The square {} is already occupied, can only place on an empty square",
                c.to_string().important()
            )
        }
        Ok(FillSquare { target: c })
    }

    fn from_extended_text(s: &str, board: &MNKBoard) -> Res<Self> {
        Self::from_compact_text(s, board)
    }

    fn from_usize_unchecked(val: usize) -> UntrustedMove<MNKBoard> {
        UntrustedMove::from_move(Self {
            target: GridCoordinates::from_row_column(
                ((val >> 8) & 0xff) as DimT,
                (val & 0xff) as DimT,
            ),
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
    fn check_invariants(self) -> bool {
        self.height <= 26
            && self.width < MAX_WIDTH as DimT
            && self.height * self.width <= 128
            && self.height * self.width >= 1
            && self.k <= min(self.height, self.width)
            && self.k >= 1
    }

    pub fn titactoe() -> Self {
        Self {
            height: 3,
            width: 3,
            k: 3,
        }
    }

    // TODO: Connect4 rules
    pub fn connect4() -> Self {
        Self {
            height: 6,
            width: 7,
            k: 4,
        }
    }

    pub fn new(height: Height, width: Width, k: DimT) -> Self {
        Self::try_new(height, width, k).expect("The provided mnk values are invalid")
    }

    pub fn try_new(height: Height, width: Width, k: DimT) -> Option<Self> {
        let height = height.0;
        let width = width.0;
        let res = Self { height, width, k };
        if res.check_invariants() {
            Some(res)
        } else {
            None
        }
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

impl Settings for MnkSettings {
    fn text(&self) -> Option<String> {
        Some(format!("[{} in a row to win]", self.k))
    }
}

pub type MnkBitboard = DefaultBitboard<ExtendedRawBitboard, GridCoordinates>;

#[derive(Copy, Clone, Default, Debug, Arbitrary)]
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
            && self.active_player == other.active_player
            && self.settings == other.settings
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
        MnkBitboard::from_raw(self.x_bb, self.size())
    }

    pub fn o_bb(self) -> MnkBitboard {
        MnkBitboard::from_raw(self.o_bb, self.size())
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
        MnkBitboard::from_uint(
            remove_ones_above(!self.occupied_bb().0, self.num_squares() - 1),
            self.size(),
        )
    }

    pub fn k(self) -> u32 {
        self.settings.k as u32
    }

    fn make_move_for_player(mut self, mov: <Self as Board>::Move, player: MnkColor) -> Self {
        debug_assert!(self.is_move_pseudolegal(mov));
        self = UnverifiedMnkBoard::new(self)
            .place_piece_unchecked(mov.target, player.into())
            .0;
        self.ply += 1;
        self.last_move = Some(mov);
        self.active_player = player.other();
        self
    }
}

impl Display for MNKBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{0}", self.as_fen())
    }
}

impl Board for MNKBoard {
    type EmptyRes = MNKBoard;

    type Settings = MnkSettings;

    type Coordinates = GridCoordinates;
    type Color = MnkColor;

    type Piece = Square;

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
        MNKBoard {
            ply: 0,
            x_bb: ExtendedRawBitboard(0),
            o_bb: ExtendedRawBitboard(0),
            settings,
            active_player: MnkColor::first(),
            last_move: None,
        }
    }

    fn from_name(name: &str) -> Res<Self> {
        board_from_name(name).or_else(|err| {
            let pattern = Regex::new(r"([0-9]+),([0-9]+),([0-9]+)").unwrap();
            if let Some(captures) = pattern.captures(name) {
                let height = parse_int_from_str(&captures[1], "m")?;
                let width = parse_int_from_str(&captures[2], "n")?;
                let k = parse_int_from_str(&captures[3], "k")?;
                let settings = MnkSettings {
                    height,
                    width,
                    k,
                };
                if !settings.check_invariants() {
                    bail!("Invalid m,n,k values (at least one value is too large or too small)");
                }
                Ok(Self::empty_for_settings(settings))
            } else {
                bail!(
                    "{0} It's also not an m,n,k list, which must have the format '{1}', e.g. '3,3,3'.",
                    err,
                    "<m>,<n>,<k>".important()
                )
            }
        })
    }

    fn name_to_pos_map() -> EntityList<NameToPos<Self>> {
        vec![
            GenericSelect {
                name: "large",
                val: || {
                    Self::from_fen("11 11 4 x 11/11/11/11/11/11/11/11/11/11/11", Relaxed).unwrap()
                },
            },
            GenericSelect {
                name: "tictactoe",
                val: Self::default,
            },
        ]
    }

    fn bench_positions() -> Vec<Self> {
        let fens = &[
            "3 3 3 x 3/3/3",
            "5 5 4 x X4/5/O2X1/O1X2/O4",
            "9 9 2 o 8X/9/9/9/9/9/9/9/9",
            "9 11 5 o O1O3O4/9X1/7O3/7X3/11/4XX5/X7X2/OO3O4X/OX8X",
            "20 5 4 o 5/5/5/5/1O3/5/5/5/5/5/1O3/X1X2/4O/1O3/5/5/1X2X/X4/5/5",
            "1 25 1 x 25",
            "3 26 2 o 26/26/10X15",
            "3 26 2 o 3X5O2XO12/7O12O2X1X/O9X6X8",
            "8 6 5 o 1X4/2XO2/2X3/1OX2O/OO1X1X/1O4/X4X/3O2",
            "10 8 5 x 2X3X1/X4O1X/2OXO2O/O1OO4/1X2X3/X2O2X1/OXOOXO1O/1X1XOO1X/5XO1/XOX1O2X",
            "10 12 5 o 12/9X2/12/3O1O2X3/6OO4/4X2X4/3X8/O2O2X4X/4X4O2/12",
        ];
        fens.map(|f| Self::from_fen(f, Relaxed).unwrap())
            .into_iter()
            .collect()
    }

    fn settings(&self) -> Self::Settings {
        self.settings
    }

    fn active_player(&self) -> MnkColor {
        self.active_player
    }

    fn halfmove_ctr_since_start(&self) -> usize {
        self.ply as usize
    }

    fn halfmove_repetition_clock(&self) -> usize {
        0
    }

    fn size(&self) -> GridSize {
        GridSize {
            height: Height(self.settings.height),
            width: Width(self.settings.width),
        }
    }

    fn is_empty(&self, coords: Self::Coordinates) -> bool {
        let idx = self.size().to_internal_key(coords);
        // slightly faster than calling `empty_bb()`
        !self.occupied_bb().is_bit_set_at(idx)
    }

    fn colored_piece_on(&self, coordinates: Self::Coordinates) -> Square {
        let idx = self.size().to_internal_key(coordinates);
        debug_assert!(self.x_bb & self.o_bb == ExtendedRawBitboard(0));
        let symbol = if (self.x_bb >> idx) & 1 == 1 {
            X
        } else if (self.o_bb >> idx) & 1 == 1 {
            O
        } else {
            Empty
        };
        Square::new(symbol, coordinates)
    }

    fn gen_pseudolegal<T: MoveList<Self>>(&self, moves: &mut T) {
        let mut empty = self.empty_bb();
        while empty.has_set_bit() {
            let idx = empty.pop_lsb();
            if idx >= self.num_squares() {
                break; // TODO: Necessary?
            }
            let next_move = FillSquare {
                target: self.idx_to_coordinates(idx as DimT),
            };
            moves.add_move(next_move);
        }
    }

    fn gen_tactical_pseudolegal<T: MoveList<Self>>(&self, _moves: &mut T) {
        // currently, no moves are considered tactical
    }

    fn random_legal_move<T: Rng>(&self, rng: &mut T) -> Option<Self::Move> {
        let empty = self.empty_bb();
        debug_assert!(empty.0.ilog2() < self.num_squares() as u32);
        let num_empty = empty.0.count_ones() as usize;
        if num_empty == 0 {
            return None;
        }
        let idx = rng.gen_range(0..num_empty);
        let target = ith_one_u128(idx, empty.0);

        Some(FillSquare {
            target: self.size().to_coordinates_unchecked(target),
        })
    }

    // Idea for another (faster and easier?) implementation:
    // Create lookup table (bitvector?) that answer "contains k consecutive 1s" for all
    // bits sequences of length 12 (= max m,n), use pext to transform columns and (anti) diagonals
    // into lookup indices.

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
        self.size().coordinates_valid(mov.target)
            && self.colored_piece_on(mov.target).symbol == Empty
    }

    fn player_result_no_movegen<H: BoardHistory<Self>>(
        &self,
        _history: &H,
    ) -> Option<PlayerResult> {
        // check for win before checking full board
        if self.is_game_lost() {
            Some(Lose)
        } else if self.empty_bb().is_zero() {
            return Some(Draw);
        } else {
            None
        }
    }

    fn player_result_slow<H: BoardHistory<Self>>(&self, history: &H) -> Option<PlayerResult> {
        self.player_result_no_movegen(history)
    }

    fn no_moves_result(&self) -> PlayerResult {
        Draw
    }

    fn can_reasonably_win(&self, _player: MnkColor) -> bool {
        true
    }

    /// Not actually a zobrist hash function, but should work well enough
    fn zobrist_hash(&self) -> ZobristHash {
        let mut hasher = DefaultHasher::new();
        self.x_bb.0.hash(&mut hasher);
        self.o_bb.0.hash(&mut hasher);
        // Don't need to hash the side to move because that is given by the parity of the number of nonempty squares
        ZobristHash(hasher.finish())
    }

    fn as_fen(&self) -> String {
        format!(
            "{height} {width} {k} {s} {pos}",
            height = self.size().height().0,
            width = self.size().width().0,
            k = self.k(),
            s = if self.active_player() == MnkColor::X {
                'x'
            } else {
                'o'
            },
            pos = position_fen_part(self)
        )
    }

    fn read_fen_and_advance_input(words: &mut Tokens, strictness: Strictness) -> Res<Self> {
        if words.clone().next().is_none() {
            bail!("Empty mnk fen".to_string());
        }
        let mut settings = MnkSettings::default();
        for i in 0..3 {
            let val = parse_int(words, "mnk value")?;
            match i {
                0 => settings.height = val,
                1 => settings.width = val,
                2 => settings.k = val,
                _ => unreachable!("logic error"),
            };
        }
        if !settings.check_invariants() {
            bail!("mnk invariants violated (at least one value is too large or too small)");
        }
        let x_str = X.to_ascii_char().to_ascii_lowercase().to_string();
        let o_str = O.to_ascii_char().to_ascii_lowercase().to_string();
        let active_player = words
            .next()
            .ok_or_else(|| anyhow!("No active player in mnk fen"))?;

        // Can't use a match expression here, apparently
        let active_player = if active_player == x_str {
            X
        } else if active_player == o_str {
            O
        } else {
            bail!("Invalid active player in mnk fen: '{active_player}'");
        };

        let Some(position) = words.next() else {
            bail!("Empty position in mnk fen")
        };

        let board = MNKBoard::empty_for_settings(settings);

        let mut board = read_position_fen::<MNKBoard>(position, UnverifiedMnkBoard::new(board))?;

        let mut ply = board.0.occupied_bb().num_ones();
        if let Some(word) = words.peek() {
            if let Ok(ply_num) = parse_int_from_str(word, "ply") {
                _ = words.next();
                ply = ply_num;
            }
        }
        board.0.ply = ply as u32;

        board.0.last_move = None;
        board.0.active_player = active_player.color().unwrap();

        board.verify_with_level(CheckFen, strictness)
    }

    fn should_flip_visually() -> bool {
        false
    }

    fn as_ascii_diagram(&self, flip: bool) -> String {
        board_to_string(self, Square::to_ascii_char, flip)
    }

    fn as_unicode_diagram(&self, flip: bool) -> String {
        board_to_string(self, Square::to_utf8_char, flip)
    }

    fn display_pretty(&self, fmt: &mut dyn BoardFormatter<Self>) -> String {
        display_board_pretty(self, fmt)
    }

    fn pretty_formatter(
        &self,
        piece_to_char: PieceToChar,
        last_move: Option<Self::Move>,
    ) -> Box<dyn BoardFormatter<Self>> {
        Box::new(DefaultBoardFormatter::new(*self, piece_to_char, last_move))
    }

    fn background_color(&self, square: GridCoordinates) -> SquareColor {
        square.square_color()
    }
}

impl MNKBoard {
    fn is_game_lost(&self) -> bool {
        if let Some(last_move) = self.last_move {
            self.is_game_won_at(last_move.target)
        } else {
            false
        }
    }

    fn is_game_won_at(&self, square: GridCoordinates) -> bool {
        let player = self.colored_piece_on(square).color();
        if player.is_none() {
            return false;
        }
        let player = player.unwrap();
        let player_bb = self.player_bb(player);
        let blockers = !self.player_bb(player);
        debug_assert!((blockers.raw()
            & ExtendedRawBitboard::single_piece(self.size().to_internal_key(square)))
        .is_zero());

        for dir in RayDirections::iter() {
            if (MnkBitboard::slider_attacks(square, blockers, dir) & player_bb)
                .to_primitive()
                .count_ones()
                >= self.k() - 1
            {
                return true;
            }
        }
        false
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
        let non_empty = this.occupied_bb().0.count_ones();
        // support custom starting positions where pieces have already been placed
        if this.ply > non_empty {
            bail!(
                "Ply is {0}, but {non_empty} moves have been played",
                this.ply
            );
        } else if strictness == Strict {
            let diff = this.x_bb().num_ones() as isize - this.o_bb.num_ones() as isize;
            if this.ply != non_empty {
                bail!("In strict mode, the number of plies ({0}) has to be the number of placed pieces ({non_empty})",
                this.ply)
            } else if diff != isize::from(this.active_player == MnkColor::O) {
                bail!("In strict mode, the number of {X} and {O} must match, unless it's {O}'s turn, \
                    in which case there must be one more {X}. However that difference is {diff}")
            }
        }
        if level != CheckFen && (this.o_bb & this.x_bb).has_set_bit() {
            bail!(
                "At least one square has two pieces on it (square {})",
                this.size()
                    .to_coordinates_unchecked((this.o_bb & this.x_bb).pop_lsb())
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

    fn size(&self) -> GridSize {
        self.0.size()
    }

    fn place_piece_unchecked(mut self, sq: GridCoordinates, piece: Symbol) -> Self {
        let placed_bb = ExtendedRawBitboard::single_piece(self.size().to_internal_key(sq));
        let bb = match piece {
            X => &mut self.0.x_bb,
            O => &mut self.0.o_bb,
            Empty => {
                return self.remove_piece_unchecked(sq);
            }
        };
        *bb |= placed_bb;

        self
    }

    fn remove_piece_unchecked(self, sq: GridCoordinates) -> Self {
        let mut this = self.0;
        let mask = !ExtendedRawBitboard::single_piece(self.size().to_internal_key(sq));
        this.x_bb &= mask;
        this.o_bb &= mask;
        this.last_move = None;
        this.ply = 0;
        UnverifiedMnkBoard(this)
    }

    fn piece_on(&self, coords: GridCoordinates) -> Res<Square> {
        Ok(self.0.colored_piece_on(self.check_coordinates(coords)?))
    }

    fn set_active_player(mut self, player: MnkColor) -> Self {
        self.0.active_player = player;
        self
    }

    fn set_ply_since_start(mut self, ply: usize) -> Res<Self> {
        let ply = u32::try_from(ply).map_err(|err| anyhow!("Invalid ply number: {err}"))?;
        self.0.ply = ply;
        Ok(self)
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
        assert_eq!(board.size().width().0, 5);
        assert_eq!(board.size().height().0, 2);
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
        assert_eq!(
            MNKBoard::from_name("10,9,7")
                .unwrap()
                .pseudolegal_moves()
                .len(),
            90
        );

        let mov: FillSquare = moves.iter().next().copied().unwrap();
        assert_eq!(moves.len(), 20);
        assert!(board.size().coordinates_valid(mov.target));
    }

    #[test]
    fn place_piece_test() {
        let board = MNKBoard::default();
        let mov = FillSquare {
            target: GridCoordinates::default(),
        };
        assert_eq!(board.active_player(), MnkColor::first());
        let board = board.make_move(mov).unwrap();
        assert_eq!(board.size().num_squares(), 9);
        assert_eq!(board.x_bb, ExtendedRawBitboard(1));
        assert_eq!(board.o_bb, ExtendedRawBitboard(0));
        assert_eq!(board.ply, 1);
        assert_eq!(
            board.empty_bb().raw(),
            !ExtendedRawBitboard(1) & ExtendedRawBitboard(0x1ff)
        );
        assert_eq!(board.active_player(), MnkColor::second());
        assert!(!board.is_game_lost());

        let board = MNKBoard::empty_for_settings(MnkSettings::new(Height(3), Width(4), 1));
        let board = board.make_move(mov).unwrap();
        assert!(board.is_game_lost());
        assert_ne!(board.x_bb().to_primitive(), 0);
        assert_eq!(board.o_bb().to_primitive(), 0);
        assert!(board.x_bb().is_single_piece());
        assert_eq!(
            board.pseudolegal_moves().len() + 1,
            board.size().num_squares()
        );
    }

    #[test]
    fn perft_startpos_test() {
        let r = perft(Depth::new_unchecked(1), MNKBoard::default());
        assert_eq!(r.depth.get(), 1);
        assert_eq!(r.nodes, 9);
        assert!(r.time.as_millis() <= 1); // 1 ms should be far more than enough even on a very slow device
        let r = split_perft(
            Depth::new_unchecked(2),
            MNKBoard::empty_for_settings(MnkSettings::new(Height(8), Width(12), 2)),
        );
        assert_eq!(r.perft_res.depth.get(), 2);
        assert_eq!(r.perft_res.nodes, 96 * 95);
        assert!(r.children.iter().all(|x| x.1 == r.children[0].1));
        assert!(r.perft_res.time.as_millis() <= 50);
        let r = split_perft(
            Depth::new_unchecked(3),
            MNKBoard::empty_for_settings(MnkSettings::new(Height(4), Width(3), 3)),
        );
        assert_eq!(r.perft_res.depth.get(), 3);
        assert_eq!(r.perft_res.nodes, 12 * 11 * 10);
        assert!(r.children.iter().all(|x| x.1 == r.children[0].1));
        assert!(r.perft_res.time.as_millis() <= 1000);
        let r = split_perft(
            Depth::new_unchecked(5),
            MNKBoard::empty_for_settings(MnkSettings::new(Height(5), Width(5), 5)),
        );
        assert_eq!(r.perft_res.depth.get(), 5);
        assert_eq!(r.perft_res.nodes, 25 * 24 * 23 * 22 * 21);
        assert!(r.children.iter().all(|x| x.1 == r.children[0].1));
        assert!(r.perft_res.time.as_millis() <= 10_000);

        let r = split_perft(
            Depth::new_unchecked(9),
            MNKBoard::startpos_for_settings(MnkSettings::titactoe()),
        );
        assert_eq!(r.perft_res.depth.get(), 9);
        assert!(r.perft_res.nodes >= 100_000);
        assert!(r.perft_res.nodes <= 9 * 8 * 7 * 6 * 5 * 4 * 3 * 2);
        for i in 0..9 {
            let mirrored = (i % 3) * 3 + i / 3;
            assert_eq!(r.children[mirrored].1, r.children[i].1);
        }
        assert!(r.perft_res.time.as_millis() <= 4000);

        let board = MNKBoard::empty_for_settings(MnkSettings::new(Height(2), Width(2), 2));
        let r = split_perft(Depth::new_unchecked(3), board);
        assert_eq!(r.perft_res.depth.get(), 3);
        assert_eq!(r.perft_res.nodes, 2 * 3 * 4);
        assert!(r.children.iter().all(|x| x.1 == 2 * 3));
        assert!(r.perft_res.time.as_millis() <= 10);
    }

    #[test]
    fn as_fen_test() {
        let board = MNKBoard::default();
        let str = board.as_fen();
        assert_eq!(str, "3 3 3 x 3/3/3");

        let board = board
            .make_move(FillSquare {
                target: board.idx_to_coordinates(4),
            })
            .unwrap();
        assert_eq!(
            board.x_bb(),
            MnkBitboard::from_uint(0x10, GridSize::tictactoe())
        );
        assert_eq!(
            board
                .colored_piece_on(board.size().to_coordinates_unchecked(4))
                .symbol,
            X
        );
        assert_eq!(board.as_fen(), "3 3 3 o 3/1X1/3");

        let board = board.make_move_for_player(
            FillSquare {
                target: board.idx_to_coordinates(3),
            },
            MnkColor::first(),
        );
        assert_eq!(board.as_fen(), "3 3 3 o 3/XX1/3");

        let board = board.make_move_for_player(
            FillSquare {
                target: board.idx_to_coordinates(5),
            },
            MnkColor::second(),
        );
        assert_eq!(board.as_fen(), "3 3 3 x 3/XXO/3");

        let board = MNKBoard::empty_for_settings(MnkSettings {
            height: 3,
            width: 4,
            k: 2,
        });
        assert_eq!(board.as_fen(), "3 4 2 x 4/4/4");

        let board = board
            .make_move(FillSquare {
                target: board.idx_to_coordinates(0),
            })
            .unwrap();
        assert_eq!(board.as_fen(), "3 4 2 o 4/4/X3");

        let board = board
            .make_move(FillSquare {
                target: board.idx_to_coordinates(4),
            })
            .unwrap();
        assert_eq!(board.as_fen(), "3 4 2 x 4/O3/X3");

        let board = board
            .make_move(FillSquare {
                target: board.idx_to_coordinates(9),
            })
            .unwrap();
        assert_eq!(board.as_fen(), "3 4 2 o 1X2/O3/X3");

        let board = board
            .make_move(FillSquare {
                target: board.idx_to_coordinates(3),
            })
            .unwrap();
        assert_eq!(board.as_fen(), "3 4 2 x 1X2/O3/X2O");
    }

    #[test]
    fn from_fen_test() {
        let board = MNKBoard::from_fen("4 3 2 x 3/3/3/3", Strict).unwrap();
        assert_eq!(board.occupied_bb().raw(), ExtendedRawBitboard(0));
        assert_eq!(board.size(), GridSize::new(Height(4), Width(3)));
        assert_eq!(board.k(), 2);
        assert_eq!(
            board,
            MNKBoard::empty_for_settings(MnkSettings::new(Height(4), Width(3), 2))
        );

        let board = MNKBoard::from_fen("3 4 3 x 3X/4/2O1", Strict).unwrap();
        assert_eq!(
            board.occupied_bb().raw(),
            ExtendedRawBitboard(0b1000_0000_0100)
        );
        assert_eq!(
            board,
            MNKBoard {
                x_bb: ExtendedRawBitboard(0b1000_0000_0000),
                o_bb: ExtendedRawBitboard(0b0000_0000_0100),
                ply: 2,
                settings: MnkSettings::new(Height(3), Width(4), 3),
                active_player: MnkColor::first(),
                last_move: None
            }
        );

        let copy = MNKBoard::from_fen(&board.as_fen(), Relaxed).unwrap();
        assert_eq!(board, copy);

        let board = MNKBoard::from_fen("7 3 2 o X1O/3/OXO/1X1/XO1/XXX/1OO", Relaxed).unwrap();
        let white_bb = ExtendedRawBitboard(0b001_000_010_010_001_111_000);
        let black_bb = ExtendedRawBitboard(0b100_000_101_000_010_000_110);
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

        let board = MNKBoard::from_fen("4 12 3 x 12/11X/1X10/2X1X3XXX1", Relaxed).unwrap();
        let white_bb =
            ExtendedRawBitboard(0b0000_0000_0000_1000_0000_0000_0000_0000_0010_0111_0001_0100);
        let black_bb = ExtendedRawBitboard(0);
        assert_eq!(
            board,
            MNKBoard {
                x_bb: white_bb,
                o_bb: black_bb,
                ply: 7,
                settings: MnkSettings::new(Height(4), Width(12), 3),
                active_player: MnkColor::first(),
                last_move: None,
            }
        );
        assert_eq!(board, MNKBoard::from_fen(&board.as_fen(), Relaxed).unwrap());
    }

    #[test]
    fn from_invalid_fen_test() {
        assert!(MNKBoard::from_fen("4 3 2 x 3/3/3/3 1", Strict).is_err());
        assert!(MNKBoard::from_fen("4 3 2 x 3/3/3/3 abc", Relaxed).is_err());
        assert!(MNKBoard::from_fen("4 3 2 3/3/3/3", Relaxed).is_err());
        assert!(MNKBoard::from_fen("4 3 2 w 3/3/3/3", Relaxed).is_err());
        assert!(MNKBoard::from_fen("4 3 2 wx 3/3/3/3", Relaxed).is_err());
        assert!(MNKBoard::from_fen("4 3 2 o 3/4/3/3", Relaxed)
            .is_err_and(|e| e.to_string().contains("Line '4' has incorrect width")));
        MNKBoard::from_fen("4 3 2 o 3//3/3", Relaxed).expect_err("Empty position in mnk fen");
        assert!(MNKBoard::from_fen("4 3 2 x", Relaxed).is_err());
        assert!(MNKBoard::from_fen("4 0 2 x ///", Relaxed).is_err());
        MNKBoard::from_fen("0 3 2 x", Relaxed)
            .expect_err("mnk invariants violated (at least one value is too large or too small)");
        assert!(MNKBoard::from_fen("4 3 2 o 4/4/4", Relaxed).is_err());
        assert!(MNKBoard::from_fen("4 3 x 3/3/3/3", Relaxed).is_err());
        assert!(MNKBoard::from_fen("3 13 2 x 13/12X/13/O12", Relaxed).is_err());
        assert!(
            MNKBoard::from_fen("12 12 o 2 12/12/12/12/12/12/12/12/12/12/12/12", Relaxed).is_err()
        );
        assert!(MNKBoard::from_fen("3 3 3 o 3/X1O/11X", Relaxed).is_err());
        assert!(MNKBoard::from_fen("3 3 3 o 3/X1O/F1X", Relaxed).is_err());
        assert!(MNKBoard::from_fen("3 10 3 x 10/10/0XA", Relaxed).is_err());
        assert!(MNKBoard::from_fen("3 3 3 o 3/3/0X2", Relaxed).is_err());
        assert!(MNKBoard::from_fen("3 3 3 x 3/-1X3/X2", Relaxed).is_err());
    }

    // perft and bench catch subtler problems, so only test fairly simple cases here
    #[test]
    fn test_winning() {
        let board = MNKBoard::from_fen("3 3 3 x XX1/3/3", Relaxed).unwrap();
        assert_eq!(board.active_player(), MnkColor::first());

        assert!(board.is_game_won_after_slow(FillSquare {
            target: board.idx_to_coordinates(8)
        }));
        assert!(!board.is_game_won_after_slow(FillSquare {
            target: board.idx_to_coordinates(5)
        }));

        let board = MNKBoard::from_fen("4 3 3 o XOX/O1O/XOO/1OX", Relaxed).unwrap();
        assert!(board.is_game_won_after_slow(FillSquare {
            target: board.idx_to_coordinates(0)
        }));
        let board = MNKBoard::from_fen("3 3 3 x XOX/O1O/XOO", Relaxed).unwrap();
        assert!(board.is_game_won_after_slow(FillSquare {
            target: board.idx_to_coordinates(4)
        }));
        let board = MNKBoard::from_fen("4 3 3 x XOX/OXO/XOO/1OX", Relaxed).unwrap();
        assert!(!board.is_game_won_after_slow(FillSquare {
            target: board.idx_to_coordinates(0)
        }));
    }

    #[test]
    fn game_over_test() {
        let pos = MNKBoard::from_fen("3 3 3 x XX1/3/3", Relaxed).unwrap();
        let mov = FillSquare::new(GridCoordinates::from_row_column(2, 2));
        assert!(pos.is_game_won_after_slow(mov));
        let new_pos = pos.make_move(mov).unwrap();
        assert!(new_pos.is_game_lost());
        assert_eq!(new_pos.last_move, Some(mov));
        assert_eq!(
            new_pos.player_result_slow(&NoHistory::default()),
            Some(Lose)
        );
        assert_eq!(
            new_pos.active_player_bb(),
            MnkBitboard::from_uint(0, pos.size())
        );
    }
}
