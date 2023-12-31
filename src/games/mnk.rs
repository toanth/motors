use std::cmp::min;
use std::fmt::{self, Debug, Display, Formatter};

use strum::IntoEnumIterator;

use crate::eval::mnk::simple_mnk_eval::SimpleMnkEval;
use crate::games::mnk::Symbol::{Empty, O, X};
use crate::games::Color::{Black, White};
use crate::games::GridSize;
use crate::games::PlayerResult::Draw;
use crate::games::*;
use crate::general::bitboards::{remove_ones_above, Bitboard, ExtendedBitboard, SliderAttacks};
use crate::general::common::*;
use crate::general::move_list::EagerNonAllocMoveList;
use crate::play::generic_engines;
use crate::search::generic_negamax::GenericNegamax;
use crate::ui::NormalGraphics;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum Symbol {
    X = 0,
    O = 1,
    #[default]
    Empty = 2,
}

const UNICODE_X: char = '⨉'; // '⨉',
const UNICODE_O: char = '◯'; // '○'

impl AbstractPieceType for Symbol {
    fn empty() -> Symbol {
        Symbol::Empty
    }

    fn to_ascii_char(self) -> char {
        match self {
            Symbol::X => 'X',
            Symbol::O => 'O',
            Symbol::Empty => '.',
        }
    }

    fn to_utf8_char(self) -> char {
        match self {
            Symbol::X => UNICODE_X,
            Symbol::O => UNICODE_O,
            Symbol::Empty => '.',
        }
    }

    fn from_utf8_char(c: char) -> Option<Self> {
        match c {
            ' ' => Some(Symbol::Empty),
            'X' | UNICODE_X => Some(Symbol::X),
            'O' | UNICODE_O => Some(Symbol::O),
            _ => None,
        }
    }

    fn to_uncolored_idx(self) -> usize {
        self as usize
    }
}

impl UncoloredPieceType for Symbol {
    type Colored = Symbol;

    fn from_uncolored_idx(idx: usize) -> Self {
        match idx {
            0 => X,
            1 => O,
            2 => Empty,
            _ => panic!("trying to construct mnk piece from incorrect integer value"),
        }
    }
}

impl ColoredPieceType for Symbol {
    type Uncolored = Symbol;

    fn color(self) -> Option<Color> {
        match self {
            Symbol::X => Some(Color::White),
            Symbol::O => Some(Color::Black),
            _ => None,
        }
    }

    fn to_colored_idx(self) -> usize {
        self as usize
    }

    fn new(color: Color, uncolored: Self::Uncolored) -> Self {
        assert!(uncolored.color().unwrap() == color);
        uncolored
    }
}

impl Display for Symbol {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.to_utf8_char())
    }
}

type Square = GenericPiece<GridCoordinates, Symbol>;
//
// #[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
// pub struct Square {
//     coordinates: GridCoordinates,
//     symbol: Symbol,
// }
//
// impl Display for Square {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         std::fmt::Display::fmt(&self.symbol, f)
//     }
// }
//
// impl ColoredPiece for Square {
//     type Coordinates = GridCoordinates;
//     type ColoredPieceType = Symbol;
//     type UncoloredPieceType = Symbol;
//
//     fn coordinates(self) -> GridCoordinates {
//         self.coordinates
//     }
//
//     fn uncolored_piece_type(self) -> Symbol {
//         self.symbol
//     }
//
//     fn to_utf8_char(self) -> char {
//         self.symbol.to_utf8_char()
//     }
//
//     fn to_ascii_char(self) -> char {
//         self.symbol.to_ascii_char()
//     }
//
//     fn colored_piece_type(self) -> Self::ColoredPieceType {
//         self.symbol
//     }
// }

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct FillSquare {
    pub target: GridCoordinates,
    // pub player: Player,
}

impl Default for FillSquare {
    fn default() -> Self {
        FillSquare {
            target: GridCoordinates::no_coordinates(),
        }
    }
}

impl Display for FillSquare {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self.target)
    }
}

impl Move<MNKBoard> for FillSquare {
    type Flags = NoMoveFlags;

    fn from_square(self) -> GridCoordinates {
        GridCoordinates::no_coordinates()
    }

    fn to_square(self) -> GridCoordinates {
        self.target
    }

    fn flags(self) -> NoMoveFlags {
        NoMoveFlags {}
    }

    fn to_compact_text(self) -> String {
        self.target.to_string()
    }

    fn from_compact_text(s: &str, _: &MNKBoard) -> Result<Self, String> {
        GridCoordinates::from_str(s).map(|target| FillSquare { target })
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MnkSettings {
    height: u8,
    width: u8,
    k: u8,
}

impl MnkSettings {
    fn check_invariants(self) -> bool {
        // allow width of at most 26 to prevent issues with square notation
        self.height <= 26
            && self.width <= 26
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

    pub fn new(height: Height, width: Width, k: usize) -> Self {
        Self::try_new(height, width, k).expect("The provided mnk values are invalid")
    }

    pub fn try_new(height: Height, width: Width, k: usize) -> Option<Self> {
        let height = height.0 as u8;
        let width = width.0 as u8;
        let res = Self {
            height,
            width,
            k: k as u8,
        };
        if res.check_invariants() {
            Some(res)
        } else {
            None
        }
    }

    pub fn height(self) -> Height {
        Height(self.height as usize)
    }

    pub fn width(self) -> Width {
        Width(self.width as usize)
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

impl Settings for MnkSettings {}

#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub struct MNKBoard {
    white_bb: ExtendedBitboard,
    black_bb: ExtendedBitboard,
    ply: u32,
    active_player: Color,
    settings: MnkSettings,
    last_move: Option<FillSquare>,
}

impl MNKBoard {
    pub fn white_bb(self) -> ExtendedBitboard {
        self.white_bb
    }

    pub fn black_bb(self) -> ExtendedBitboard {
        self.black_bb
    }

    pub fn player_bb(self, player: Color) -> ExtendedBitboard {
        match player {
            Color::White => self.white_bb,
            Color::Black => self.black_bb,
        }
    }

    pub fn active_player_bb(self) -> ExtendedBitboard {
        self.player_bb(self.active_player())
    }

    pub fn inactive_player_bb(self) -> ExtendedBitboard {
        self.player_bb(self.active_player().other())
    }

    pub fn occupied_bb(self) -> ExtendedBitboard {
        self.black_bb | self.white_bb
    }

    pub fn empty_bb(self) -> ExtendedBitboard {
        ExtendedBitboard(remove_ones_above(
            !self.occupied_bb().0,
            self.num_squares() - 1,
        ))
    }

    pub fn k(self) -> u32 {
        self.settings.k as u32
    }

    fn make_move_for_player(&self, mov: <Self as Board>::Move, player: Color) -> Option<Self> {
        debug_assert!(self.is_move_pseudolegal(mov));
        let mut copy = *self;

        let bb = match player {
            White => &mut copy.white_bb,
            Black => &mut copy.black_bb,
        };
        bb.0 |= 1 << self.to_idx(mov.target);
        copy.ply += 1;
        copy.last_move = Some(mov);
        copy.active_player = player.other();
        Some(copy)
    }
}

impl Display for MNKBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{0}", self.as_unicode_diagram())
    }
}

impl Board for MNKBoard {
    type Settings = MnkSettings;

    type Coordinates = GridCoordinates;

    // type Size = RectangularSize;

    type Piece = Square;

    type Move = FillSquare;

    type MoveList = EagerNonAllocMoveList<Self, 128>;
    type LegalMoveList = Self::MoveList;
    type EngineList = MnkEngineList;
    type GraphicsList = NormalGraphics;

    fn game_name() -> String {
        "m,n,k".to_string()
    }

    fn empty(settings: MnkSettings) -> MNKBoard {
        assert!(settings.height <= 128);
        assert!(settings.width <= 128);
        assert!(settings.k <= 128);
        assert!(settings.k <= settings.height.min(settings.width));
        assert!(settings.height * settings.width <= 128);
        MNKBoard {
            ply: 0,
            white_bb: ExtendedBitboard(0),
            black_bb: ExtendedBitboard(0),
            settings,
            active_player: Color::White,
            last_move: None,
        }
    }

    fn startpos(settings: MnkSettings) -> MNKBoard {
        Self::empty(settings)
    }

    fn settings(&self) -> Self::Settings {
        self.settings
    }

    fn active_player(&self) -> Color {
        self.active_player
    }

    fn fullmove_ctr(&self) -> usize {
        self.ply as usize / 2
    }

    fn halfmove_ctr_since_start(&self) -> usize {
        self.ply as usize
    }

    fn halfmove_repetition_clock(&self) -> usize {
        0
    }

    fn size(&self) -> GridSize {
        GridSize {
            height: Height(self.settings.height as usize),
            width: Width(self.settings.width as usize),
        }
    }

    fn piece_on_idx(&self, idx: usize) -> Square {
        let coordinates = self.to_coordinates(idx);
        debug_assert!(self.white_bb & self.black_bb == ExtendedBitboard(0));
        if (self.white_bb >> idx) & 1 == 1 {
            Square {
                symbol: X,
                coordinates,
            }
        } else if (self.black_bb >> idx) & 1 == 1 {
            Square {
                symbol: O,
                coordinates,
            }
        } else {
            Square {
                symbol: Empty,
                coordinates,
            }
        }
    }

    fn are_all_pseudolegal_legal() -> bool {
        true
    }

    fn pseudolegal_moves(&self) -> EagerNonAllocMoveList<Self, 128> {
        let mut empty = self.empty_bb();
        let mut moves: EagerNonAllocMoveList<Self, 128> = Default::default();
        while empty.has_set_bit() {
            let idx = empty.pop_lsb();
            if idx >= self.num_squares() {
                break;
            }
            let next_move = FillSquare {
                target: self.to_coordinates(idx),
            };
            moves.add_move(next_move);
        }
        moves
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
            target: self.to_coordinates(target),
        })
    }

    fn random_pseudolegal_move<R: Rng>(&self, rng: &mut R) -> Option<Self::Move> {
        self.random_legal_move(rng) // all pseudolegal moves are legal for m,n,k games
    }

    fn make_move(self, mov: Self::Move) -> Option<Self> {
        self.make_move_for_player(mov, self.active_player())
    }

    // Idea for another (faster and easier?) implementation:
    // Create lookup table (bitvector?) that answer "contains k consecutive 1s" for all
    // bits sequences of length 12 (= max m,n), use pext to transform columns and (anti) diagonals
    // into lookup indices.

    fn is_move_pseudolegal(&self, mov: Self::Move) -> bool {
        self.size().valid_coordinates(mov.target) && self.piece_on(mov.target).symbol == Empty
    }

    fn no_moves_result(&self) -> PlayerResult {
        Draw
    }

    fn as_fen(&self) -> String {
        format!(
            "{height} {width} {k} {s} {pos}",
            height = self.size().height().0,
            width = self.size().width().0,
            k = self.k(),
            s = if self.active_player() == White {
                'x'
            } else {
                'o'
            },
            pos = position_fen_part(self)
        )
    }

    fn read_fen_and_advance_input(s: &mut &str) -> Result<Self, String> {
        let string = *s;
        let mut words = string.split_whitespace();
        if string.is_empty() {
            return Err("Empty mnk fen".to_string());
        }
        let mut settings = MnkSettings::default();
        for i in 0..3 {
            let val = parse_int(&mut words, "mnk value")?;
            match i {
                0 => settings.height = val,
                1 => settings.width = val,
                2 => settings.k = val,
                _ => panic!("logic error"),
            };
        }
        if !settings.check_invariants() {
            return Err(
                "mnk invariants violated (at least one value is too large or too small)"
                    .to_string(),
            );
        }
        let x_str = X.to_ascii_char().to_ascii_lowercase().to_string();
        let o_str = O.to_ascii_char().to_ascii_lowercase().to_string();
        let active_player = words
            .next()
            .ok_or_else(|| "No active player in mnk fen".to_string())?;

        // Can't use a match expression here, apparently
        let active_player = if active_player == x_str {
            X
        } else if active_player == o_str {
            O
        } else {
            return Err(format!(
                "Invalid active player in mnk fen: '{active_player}'"
            ));
        };

        let position = words
            .next()
            .ok_or_else(|| "Empty position in mnk fen".to_string())?;

        let mut board = MNKBoard::empty(settings);

        let place_piece = |board: MNKBoard, target: GridCoordinates, symbol: Symbol| {
            board
                .make_move_for_player(FillSquare { target }, symbol.color().unwrap())
                .ok_or_else(|| {
                    format!(
                        "Internal error: Couldn't place symbol {symbol} at coordinates {target}"
                    )
                })
        };

        board = read_position_fen(position, board, place_piece)?;

        board.last_move = None;
        board.active_player = active_player.color().unwrap();
        *s = words.remainder().unwrap_or_default();
        Ok(board)
    }

    fn as_ascii_diagram(&self) -> String {
        board_to_string(self, Square::to_ascii_char)
    }

    fn as_unicode_diagram(&self) -> String {
        board_to_string(self, Square::to_utf8_char)
    }

    fn verify_position_legal(&self) -> Result<(), String> {
        let non_empty = self.occupied_bb().0.count_ones();
        if self.ply != non_empty {
            return Err(format!(
                "Ply is {0}, but {non_empty} moves have been played",
                self.ply
            ));
        }
        if (self.black_bb & self.white_bb).has_set_bit() {
            return Err(format!(
                "Internal error: At least one square has two pieces on it"
            ));
        }
        if !self.settings.check_invariants() {
            return Err(format!(
                "Invariants of settings are violated: m={0}, n={1}, k={2}",
                self.height(),
                self.width(),
                self.settings.k
            ));
        }
        Ok(())
    }

    fn game_result_no_movegen(&self) -> Option<PlayerResult> {
        // check for win before checking full board
        if self.is_game_lost() {
            Some(Lose)
        } else if self.empty_bb().is_zero() {
            return Some(Draw);
        } else {
            None
        }
    }

    fn game_result_slow(&self) -> Option<PlayerResult> {
        self.game_result_no_movegen()
    }

    fn zobrist_hash(&self) -> ZobristHash {
        todo!()
    }

    fn make_nullmove(mut self) -> Option<Self> {
        self.active_player = self.active_player.other();
        Some(self)
    }

    fn noisy_pseudolegal(&self) -> Self::MoveList {
        Default::default()
    }
}

impl MNKBoard {
    fn is_game_lost(&self) -> bool {
        if self.last_move.is_none() {
            return false;
        }
        let last_move = self.last_move.unwrap();
        let square = last_move.target;
        let player = self.piece_on(square).uncolored().color();
        if player.is_none() {
            return false;
        }
        let player = player.unwrap();
        let player_bb = self.player_bb(player);
        let blockers = !self.player_bb(player);
        debug_assert!(
            (blockers & ExtendedBitboard::single_piece(self.to_idx(last_move.target))).is_zero()
        );

        for dir in SliderAttacks::iter() {
            if (ExtendedBitboard::slider_attacks(square, blockers, self.size(), dir) & player_bb)
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

pub struct MnkEngineList {}

impl EngineList<MNKBoard> for MnkEngineList {
    fn list_engines() -> Vec<(String, CreateEngine<MNKBoard>)> {
        let mut res = generic_engines();
        res.push(("generic_negamax".to_string(), |_| {
            Box::new(GenericNegamax::<MNKBoard, SimpleMnkEval>::default())
        }));
        res
    }
}

/// lots of tests, which should probably go to their own file?
/// TODO: Add tests for `is_game_lost`
#[cfg(test)]
mod test {
    use crate::games::mnk::{FillSquare, MNKBoard, MnkSettings, Symbol};
    use crate::games::Color::{Black, White};
    use crate::games::{
        Board, Color, GridCoordinates, GridSize, Height, RectangularSize, Size, Width,
    };
    use crate::general::bitboards::{Bitboard, ExtendedBitboard};
    use crate::search::perft::{perft, split_perft};

    #[test]
    fn dimension_test() {
        let board = MNKBoard::default();
        assert_eq!(board.size().height.0, 3);
        assert_eq!(board.size().width.0, 3);
        assert_eq!(board.k(), 3);
        let board = MNKBoard::empty(MnkSettings::new(Height(2), Width(5), 2));
        assert_eq!(board.size().width().0, 5);
        assert_eq!(board.size().height().0, 2);
        assert_eq!(board.k(), 2);
        let settings = MnkSettings::new(Height(12), Width(10), 6);
        assert_eq!(settings.width, 10);
        assert_eq!(settings.height, 12);
        assert_eq!(settings.k, 6);
        let board = MNKBoard::startpos(settings);
        assert_eq!(board.settings, settings);
    }

    #[test]
    #[should_panic]
    fn dimension_test_invalid_k_0() {
        MnkSettings::new(Height(4), Width(5), 0);
    }

    #[test]
    #[should_panic]
    fn dimension_test_invalid_k_too_large() {
        MnkSettings::new(Height(4), Width(5), 6);
    }

    #[test]
    #[should_panic]
    fn dimension_test_invalid_zero_width() {
        MnkSettings::new(Height(4), Width(0), 3);
    }

    #[test]
    #[should_panic]
    fn dimension_test_invalid_width_too_large() {
        MnkSettings::new(Height(4), Width(33), 3);
    }

    #[test]
    #[should_panic]
    fn dimension_test_invalid_board_too_large() {
        MnkSettings::new(Height(12), Width(11), 6);
    }

    // Only covers very basic cases, perft is used for mor more complex cases
    #[test]
    fn movegen_test() {
        let board = MNKBoard::empty(MnkSettings::new(Height(4), Width(5), 2));
        let mut moves = board.pseudolegal_moves();
        assert_eq!(moves.len(), 20);
        assert_eq!(
            MNKBoard::empty(MnkSettings::new(Height(10), Width(9), 7))
                .pseudolegal_moves()
                .len(),
            90
        );

        let mov = moves.next().unwrap();
        assert_eq!(moves.len(), 19);
        assert!(board.size().valid_coordinates(mov.target));
    }

    #[test]
    fn place_piece_test() {
        let board = MNKBoard::default();
        let mov = FillSquare {
            target: GridCoordinates::default(),
        };
        assert_eq!(board.active_player(), White);
        let board = board.make_move(mov).unwrap();
        assert_eq!(board.size().num_squares(), 9);
        assert_eq!(board.white_bb, ExtendedBitboard(1));
        assert_eq!(board.black_bb, ExtendedBitboard(0));
        assert_eq!(board.ply, 1);
        assert_eq!(
            board.empty_bb(),
            !ExtendedBitboard(1) & ExtendedBitboard(0x1ff)
        );
        assert_eq!(board.active_player(), Color::Black);
        assert!(!board.is_game_lost());

        let board = MNKBoard::empty(MnkSettings::new(Height(3), Width(4), 1));
        let board = board.make_move(mov).unwrap();
        assert!(board.is_game_lost());
        assert_ne!(board.white_bb().to_primitive(), 0);
        assert_eq!(board.black_bb().to_primitive(), 0);
        assert!(board.white_bb().is_single_piece());
        assert_eq!(
            board.pseudolegal_moves().len() + 1,
            board.size().num_squares()
        );
    }

    #[test]
    fn perft_startpos_test() {
        let r = perft(1, MNKBoard::default());
        assert_eq!(r.depth, 1);
        assert_eq!(r.nodes, 9);
        assert!(r.time.as_millis() <= 1); // 1 ms should be far more than enough even on a very slow device
        let r = split_perft(
            2,
            MNKBoard::empty(MnkSettings::new(Height(8), Width(12), 2)),
        );
        assert_eq!(r.perft_res.depth, 2);
        assert_eq!(r.perft_res.nodes, 96 * 95);
        assert!(r.children.iter().all(|x| x.1 == r.children[0].1));
        assert!(r.perft_res.time.as_millis() <= 10);
        let r = split_perft(3, MNKBoard::empty(MnkSettings::new(Height(4), Width(3), 3)));
        assert_eq!(r.perft_res.depth, 3);
        assert_eq!(r.perft_res.nodes, 12 * 11 * 10);
        assert!(r.children.iter().all(|x| x.1 == r.children[0].1));
        assert!(r.perft_res.time.as_millis() <= 1000);
        let r = split_perft(5, MNKBoard::empty(MnkSettings::new(Height(5), Width(5), 5)));
        assert_eq!(r.perft_res.depth, 5);
        assert_eq!(r.perft_res.nodes, 25 * 24 * 23 * 22 * 21);
        assert!(r.children.iter().all(|x| x.1 == r.children[0].1));
        assert!(r.perft_res.time.as_millis() <= 2000);

        let r = split_perft(9, MNKBoard::default());
        assert_eq!(r.perft_res.depth, 9);
        assert!(r.perft_res.nodes >= 100_000);
        assert!(r.perft_res.nodes <= 9 * 8 * 7 * 6 * 5 * 4 * 3 * 2);
        assert!(r.children.iter().all(|x| x.1 == r.children[0].1));
        assert!(r.perft_res.time.as_millis() <= 1000);

        let board = MNKBoard::empty(MnkSettings::new(Height(2), Width(2), 2));
        let r = split_perft(3, board);
        assert_eq!(r.perft_res.depth, 3);
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
                target: board.to_coordinates(4),
            })
            .unwrap();
        assert_eq!(board.white_bb(), ExtendedBitboard(0x10));
        assert_eq!(board.piece_on_idx(4).symbol, Symbol::X);
        assert_eq!(board.as_fen(), "3 3 3 o 3/1X1/3");

        let board = board
            .make_move_for_player(
                FillSquare {
                    target: board.to_coordinates(3),
                },
                White,
            )
            .unwrap();
        assert_eq!(board.as_fen(), "3 3 3 o 3/XX1/3");

        let board = board
            .make_move_for_player(
                FillSquare {
                    target: board.to_coordinates(5),
                },
                Black,
            )
            .unwrap();
        assert_eq!(board.as_fen(), "3 3 3 x 3/XXO/3");

        let board = MNKBoard::empty(MnkSettings {
            height: 3,
            width: 4,
            k: 2,
        });
        assert_eq!(board.as_fen(), "3 4 2 x 4/4/4");

        let board = board
            .make_move(FillSquare {
                target: board.to_coordinates(0),
            })
            .unwrap();
        assert_eq!(board.as_fen(), "3 4 2 o 4/4/X3");

        let board = board
            .make_move(FillSquare {
                target: board.to_coordinates(4),
            })
            .unwrap();
        assert_eq!(board.as_fen(), "3 4 2 x 4/O3/X3");

        let board = board
            .make_move(FillSquare {
                target: board.to_coordinates(9),
            })
            .unwrap();
        assert_eq!(board.as_fen(), "3 4 2 o 1X2/O3/X3");

        let board = board
            .make_move(FillSquare {
                target: board.to_coordinates(3),
            })
            .unwrap();
        assert_eq!(board.as_fen(), "3 4 2 x 1X2/O3/X2O");
    }

    #[test]
    fn from_fen_test() {
        let board = MNKBoard::from_fen("4 3 2 x 3/3/3/3").unwrap();
        assert_eq!(board.occupied_bb(), ExtendedBitboard(0));
        assert_eq!(board.size(), GridSize::new(Height(4), Width(3)));
        assert_eq!(board.k(), 2);
        assert_eq!(
            board,
            MNKBoard::empty(MnkSettings::new(Height(4), Width(3), 2))
        );

        let board = MNKBoard::from_fen("3 4 3 o 3X/4/2O1").unwrap();
        assert_eq!(board.occupied_bb(), ExtendedBitboard(0b1000_0000_0100));
        assert_eq!(
            board,
            MNKBoard {
                white_bb: ExtendedBitboard(0b1000_0000_0000),
                black_bb: ExtendedBitboard(0b0000_0000_0100),
                ply: 2,
                settings: MnkSettings::new(Height(3), Width(4), 3),
                active_player: Black,
                last_move: None
            }
        );

        let copy = MNKBoard::from_fen(&board.as_fen()).unwrap();
        assert_eq!(board, copy);

        let board = MNKBoard::from_fen("7 3 2 o X1O/3/OXO/1X1/XO1/XXX/1OO").unwrap();
        let white_bb = ExtendedBitboard(0b001_000_010_010_001_111_000);
        let black_bb = ExtendedBitboard(0b100_000_101_000_010_000_110);
        assert_eq!(
            board,
            MNKBoard {
                white_bb,
                black_bb,
                ply: 13,
                settings: MnkSettings::new(Height(7), Width(3), 2),
                active_player: Color::Black,
                last_move: None
            }
        );
        assert_eq!(board, MNKBoard::from_fen(&board.as_fen()).unwrap());

        let board = MNKBoard::from_fen("4 12 3 x 12/11X/1X10/2X1X3XXX1").unwrap();
        let white_bb =
            ExtendedBitboard(0b0000_0000_0000_1000_0000_0000_0000_0000_0010_0111_0001_0100);
        let black_bb = ExtendedBitboard(0);
        assert_eq!(
            board,
            MNKBoard {
                white_bb,
                black_bb,
                ply: 7,
                settings: MnkSettings::new(Height(4), Width(12), 3),
                active_player: White,
                last_move: None,
            }
        );
        assert_eq!(board, MNKBoard::from_fen(&board.as_fen()).unwrap());
    }

    #[test]
    fn from_invalid_fen_test() {
        assert!(MNKBoard::from_fen("4 3 2 3/3/3/3").is_err_and(|e| e.contains("")));
        assert!(MNKBoard::from_fen("4 3 2 w 3/3/3/3").is_err_and(|e| e.contains("")));
        assert!(MNKBoard::from_fen("4 3 2 wx 3/3/3/3").is_err_and(|e| e.contains("")));
        assert!(MNKBoard::from_fen("4 3 2 o 3/4/3/3")
            .is_err_and(|e| e.contains("Line '4' has incorrect width")));
        MNKBoard::from_fen("4 3 2 o 3//3/3").expect_err("Empty position in mnk fen");
        assert!(MNKBoard::from_fen("4 3 2 x").is_err_and(|e| e.contains("")));
        assert!(MNKBoard::from_fen("4 0 2 x ///").is_err());
        MNKBoard::from_fen("0 3 2 x")
            .expect_err("mnk invariants violated (at least one value is too large or too small)");
        assert!(MNKBoard::from_fen("4 3 2 o 4/4/4").is_err());
        assert!(MNKBoard::from_fen("4 3 x 3/3/3/3").is_err());
        assert!(MNKBoard::from_fen("3 13 2 x 13/12X/13/O12").is_err());
        assert!(MNKBoard::from_fen("12 12 o 2 12/12/12/12/12/12/12/12/12/12/12/12").is_err());
        assert!(MNKBoard::from_fen("3 3 3 o 3/X1O/11X").is_err());
        assert!(MNKBoard::from_fen("3 3 3 o 3/X1O/F1X").is_err());
        assert!(MNKBoard::from_fen("3 10 3 x 10/10/0XA").is_err());
        assert!(MNKBoard::from_fen("3 3 3 o 3/3/0X2").is_err());
        assert!(MNKBoard::from_fen("3 3 3 x 3/-1X3/X2").is_err());
    }

    // perft and bench catch subtler problems, so only test fairly simple cases here
    #[test]
    fn test_winning() {
        let board = MNKBoard::from_fen("3 3 3 x XX1/3/3").unwrap();
        assert_eq!(board.active_player(), White);

        assert!(board.is_game_won_after_slow(FillSquare {
            target: board.to_coordinates(8)
        }));
        assert!(!board.is_game_won_after_slow(FillSquare {
            target: board.to_coordinates(5)
        }));

        let board = MNKBoard::from_fen("4 3 3 o XOX/O1O/XOO/1OX").unwrap();
        assert!(board.is_game_won_after_slow(FillSquare {
            target: board.to_coordinates(0)
        }));
        let board = MNKBoard::from_fen("3 3 3 x XOX/O1O/XOO").unwrap();
        assert!(board.is_game_won_after_slow(FillSquare {
            target: board.to_coordinates(4)
        }));
        let board = MNKBoard::from_fen("4 3 3 x XOX/OXO/XOO/1OX").unwrap();
        assert!(!board.is_game_won_after_slow(FillSquare {
            target: board.to_coordinates(0)
        }));
    }
}
