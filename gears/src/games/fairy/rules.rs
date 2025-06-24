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
use crate::PlayerResult;
use crate::games::ataxx::AtaxxBoard;
use crate::games::fairy::attacks::{AttackBitboardFilter, EffectRules, GenPieceAttackKind};
use crate::games::fairy::effects::Observers;
use crate::games::fairy::moves::FairyMove;
use crate::games::fairy::pieces::{Piece, PieceId};
use crate::games::fairy::rules::GameEndEager::{DrawCounter, InsufficientMaterial, No, Repetition};
use crate::games::fairy::rules::GameEndEager::{InRowAtLeast, PieceIn};
use crate::games::fairy::rules::GameEndRes::{ActivePlayerWin, Draw, InactivePlayerWin};
use crate::games::fairy::rules::NoMovesCondition::{Always, InCheck, NotInCheck};
use crate::games::fairy::rules::NumRoyals::{BetweenInclusive, Exactly};
use crate::games::fairy::rules::PieceCond::AnyPiece;
use crate::games::fairy::{
    ColorInfo, FairyBitboard, FairyBoard, FairyCastleInfo, FairyColor, FairySize, MAX_NUM_PIECE_TYPES,
    RawFairyBitboard, UnverifiedFairyBoard,
};
use crate::games::mnk::{MNKBoard, MnkSettings};
use crate::games::{BoardHistory, Color, DimT, NUM_COLORS, PosHash, Settings, chess, n_fold_repetition};
use crate::general::bitboards::{Bitboard, RawBitboard};
use crate::general::board::{BitboardBoard, Board, BoardHelpers};
use crate::general::common::{Res, Tokens};
use crate::general::move_list::MoveList;
use crate::general::moves::Legality::PseudoLegal;
use crate::general::moves::Move;
use crate::general::squares::GridSize;
use arbitrary::Arbitrary;
use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, LazyLock};

/// Modifications that can apply to a square, such as a piece on that square being promoted in crazyhouse.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub enum SquareEffect {
    Promoted,
    // TODO: More values like `Neutral`, etc
}

/// Whether any or all royal pieces have to be attacked for the player to be considered in check
#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub enum CheckCount {
    AnyRoyal,
    #[allow(dead_code)] // TODO: Variant with multiple royal pieces
    AllRoyals,
}

/// Which attacks are considered to put the opponent in check
#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub enum CheckingAttack {
    None,
    Capture,
    /// In atomic chess, if both kings are adjacent it's not a check
    NoRoyalAdjacent,
}

/// When it is legal for a player to be in check.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub enum PlayerCheckOk {
    Never,
    Always,
    OpponentNoRoyals,
}

impl PlayerCheckOk {
    pub fn satisfied(self, pos: &FairyBoard, us: FairyColor) -> bool {
        match self {
            PlayerCheckOk::Never => !pos.is_player_in_check(us),
            PlayerCheckOk::Always => true,
            PlayerCheckOk::OpponentNoRoyals => pos.royal_bb_for(!us).is_zero() || !pos.is_player_in_check(us),
        }
    }
}

/// Determines what counts as check
#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub struct CheckRules {
    /// What counts as a check, attacking a single royal piece or all royal pieces at the same time
    pub count: CheckCount,
    /// How attacks are generated that test if a player is in check
    pub attack_condition: CheckingAttack,
    /// Whether it is legal for the inactive player to be in check
    pub inactive_check_ok: PlayerCheckOk,
    /// Whether it is legal for the active player to be in check
    pub active_check_ok: PlayerCheckOk,
}

impl CheckRules {
    pub fn chess() -> Self {
        Self {
            count: CheckCount::AnyRoyal,
            attack_condition: CheckingAttack::Capture,
            inactive_check_ok: PlayerCheckOk::Never,
            active_check_ok: PlayerCheckOk::Always,
        }
    }

    pub fn none() -> Self {
        Self {
            count: CheckCount::AnyRoyal,
            attack_condition: CheckingAttack::None,
            inactive_check_ok: PlayerCheckOk::Always,
            active_check_ok: PlayerCheckOk::Always,
        }
    }

    pub fn satisfied(&self, pos: &FairyBoard) -> bool {
        self.inactive_check_ok.satisfied(pos, pos.inactive_player())
            && self.active_check_ok.satisfied(pos, pos.active_player())
    }
}

/// Often, some rules apply to specific pieces. This enum describes such a set of pieces.
#[derive(Debug, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub enum PieceCond {
    AnyPiece,
    Royal,
    NonRoyal,
    Only(PieceId),
    OneOf(Vec<PieceId>),
}

impl PieceCond {
    pub fn bitboard(&self, pos: &FairyBoard) -> FairyBitboard {
        match self {
            AnyPiece => pos.either_player_bb(),
            PieceCond::Royal => pos.royal_bb(),
            PieceCond::NonRoyal => pos.either_player_bb() & !pos.royal_bb(),
            PieceCond::Only(piece) => pos.piece_bb(*piece),
            PieceCond::OneOf(list) => {
                list.iter().map(|&p| pos.piece_bb(p)).fold(pos.zero_bitboard(), std::ops::BitOr::bitor)
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub enum SquareCond {
    Bitboard(RawFairyBitboard),
    // the bitboard gets flipped vertically for the second player
    SideRelativeBitboard(RawFairyBitboard),
}

impl SquareCond {
    pub fn intersects(&self, bb: FairyBitboard, pos: &FairyBoard) -> bool {
        match self {
            SquareCond::Bitboard(b) => (b & bb.raw()).has_set_bit(),
            SquareCond::SideRelativeBitboard(b) => (bb.flip_if(!pos.active.is_first()).raw() & *b).has_set_bit(),
        }
    }
}

struct GameEndResIfBuilder(GameEndRes, GameEndRes);

impl GameEndResIfBuilder {
    fn if_eager(self, condition: GameEndEager) -> GameEndRes {
        GameEndRes::If(condition, Box::new([self.0, self.1]))
    }

    fn if_no_moves_and(self, condition: NoMovesCondition) -> GameEndRes {
        GameEndRes::IfNoMovesAnd(condition, Box::new([self.0, self.1]))
    }

    fn if_a_move_achieves(self, condition: GameEndEager) -> GameEndRes {
        GameEndRes::IfMoveAchieves(condition, Box::new([self.0, self.1]))
    }
}

/// When a game end condition is met (either a [`NoMovesCondition`] or a [`GameEndEager`] condition),
/// this enum determines who wins.
/// The [`Rules`] struct contains a `Vec` of `(NoMoveCondition, GameEndResult)` pairs and a `Vec` of `(GameEndEager, GameEndResult)` pairs.
/// Conditions are checked in order; the first that matches determines the result.
#[derive(Debug, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub enum GameEndRes {
    ActivePlayerWin,   // a.k.a. Win
    InactivePlayerWin, // a.k.a. Lose
    FirstPlayerWin,
    SecondPlayerWin,
    Draw,
    MorePieces,
    If(GameEndEager, Box<[GameEndRes; 2]>),
    IfNoMovesAnd(NoMovesCondition, Box<[GameEndRes; 2]>),
    // the index is 1 iff there is a legal move that makes the condition true
    IfMoveAchieves(GameEndEager, Box<[GameEndRes; 2]>),
}

impl GameEndRes {
    #[allow(unused)]
    pub fn win() -> Self {
        ActivePlayerWin
    }

    pub fn loss() -> Self {
        InactivePlayerWin
    }

    fn but(self, other: Self) -> GameEndResIfBuilder {
        GameEndResIfBuilder(self, other)
    }

    pub fn to_res<H: BoardHistory>(&self, pos: &FairyBoard, history: &H) -> PlayerResult {
        match self {
            ActivePlayerWin => PlayerResult::Win,
            InactivePlayerWin => PlayerResult::Lose,
            Draw => PlayerResult::Draw,
            GameEndRes::MorePieces => {
                match pos.active_player_bb().num_ones().cmp(&pos.inactive_player_bb().num_ones()) {
                    Ordering::Less => PlayerResult::Lose,
                    Ordering::Equal => PlayerResult::Draw,
                    Ordering::Greater => PlayerResult::Win,
                }
            }
            GameEndRes::FirstPlayerWin => {
                if pos.active_player().is_first() {
                    PlayerResult::Win
                } else {
                    PlayerResult::Lose
                }
            }
            GameEndRes::SecondPlayerWin => {
                if pos.active_player().is_first() {
                    PlayerResult::Lose
                } else {
                    PlayerResult::Win
                }
            }
            GameEndRes::If(condition, res) => {
                let idx = usize::from(condition.satisfied(pos, history));
                res[idx].to_res(pos, history)
            }
            GameEndRes::IfNoMovesAnd(condition, res) => {
                let idx = usize::from(pos.has_no_legal_moves() && condition.satisfied(pos));
                res[idx].to_res(pos, history)
            }
            GameEndRes::IfMoveAchieves(condition, res) => {
                // this is pretty slow, but that's fine since it only happens when the game is over (possibly during search),
                // and this result tends to be triggered rarely.
                let mut hist = history.clone();
                let idx = usize::from(pos.children().any(move |c| {
                    hist.push(c.hash_pos());
                    let res = condition.satisfied(&c, history);
                    hist.pop();
                    res
                }));
                res[idx].to_res(pos, history)
            }
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub enum PlayerCond {
    // A condition applies to all players, i.e. instead of checking the white or black pawn bitboard we check the pawn bitboard
    All,
    First,
    Second,
    Active,
    // This is the most common variant, because eager game end conditions are checked of the start of a turn,
    // which means that a game-winning move has always been made by the now-inactive player.
    Inactive,
}

impl PlayerCond {
    pub fn bb(self, pos: &FairyBoard) -> FairyBitboard {
        match self {
            PlayerCond::All => pos.either_player_bb(),
            PlayerCond::First => pos.player_bb(FairyColor::first()),
            PlayerCond::Second => pos.player_bb(FairyColor::second()),
            PlayerCond::Active => pos.active_player_bb(),
            PlayerCond::Inactive => pos.inactive_player_bb(),
        }
    }
}

/// When there are no legal moves for the current player, these conditions are checked to see
/// if the game ends with the associated `[GameEndResult]`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub enum NoMovesCondition {
    Always,
    InCheck,
    NotInCheck,
    NoOpponentMoves,
}

impl NoMovesCondition {
    pub fn satisfied(&self, pos: &FairyBoard) -> bool {
        match self {
            Always => true,
            InCheck => pos.is_in_check(),
            NotInCheck => !pos.is_in_check(),
            NoMovesCondition::NoOpponentMoves => {
                let Some(new_pos) = pos.clone().flip_side_to_move() else {
                    return false;
                };
                // we can't simply use `legal_moves()` here because that already handles no legal moves
                let mut pseudolegal = new_pos.pseudolegal_moves();
                if FairyMove::legality() == PseudoLegal {
                    MoveList::<FairyBoard>::filter_moves(&mut pseudolegal, |m: &mut FairyMove| {
                        new_pos.is_pseudolegal_move_legal(*m)
                    });
                }
                MoveList::<FairyBoard>::num_moves(&pseudolegal) == 0
            }
        }
    }
}

/// These conditions are checked first, before attempting to do movegen.
/// Ideally, they should be inexpensive to compute.
/// If there are no legal moves, `NoMovesCondition` is used instead.
#[derive(Debug, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
#[allow(dead_code)]
pub enum GameEndEager {
    /// The given player has no pieces that satisfy the given condition
    No(PieceCond, PlayerCond),
    // If there is a last move, it caused the now inactive player to have `k` pieces in a row, otherwise the given player
    // has at least `k` pieces in a row somewhere
    InRowAtLeast(usize, PlayerCond),
    /// a.k.a. the 50 move rule (expressed in ply)
    DrawCounter(usize),
    /// a.k.a. the threefold repetition rule
    Repetition(usize),
    /// All the given pieces occur at most the given count many times for the given player
    InsufficientMaterial(Vec<(PieceId, usize)>, PlayerCond),
    /// If any player has a given piece on a given square, they win
    PieceIn(PieceCond, SquareCond, PlayerCond),
}

impl GameEndEager {
    pub fn satisfied<H: BoardHistory>(&self, pos: &FairyBoard, history: &H) -> bool {
        let us = pos.active_player();

        match self {
            No(piece, player) => {
                let player_bb = player.bb(pos);
                (piece.bitboard(pos) & player_bb).is_zero()
            }
            &InRowAtLeast(k, player) => {
                let mut res = false;
                let player_bb = player.bb(pos);
                if pos.0.last_move.is_null() {
                    for sq in player_bb.ones() {
                        res |= pos.k_in_row_at(k, sq, !us);
                    }
                    res
                } else {
                    debug_assert_eq!(player, PlayerCond::Inactive);
                    let sq = pos.0.last_move.dest_square_in(pos);
                    pos.k_in_row_at(k, sq, !us)
                }
            }
            &DrawCounter(max) => pos.0.draw_counter >= max,
            &Repetition(max) => n_fold_repetition(max, history, pos.hash_pos(), usize::MAX),
            InsufficientMaterial(vec, player) => {
                let player_bb = player.bb(pos);
                for &(piece, count) in vec {
                    if (pos.piece_bb(piece) & player_bb).num_ones() > count {
                        return false;
                    }
                }
                true
            }
            PieceIn(piece, squares, player) => {
                let player_bb = player.bb(pos);
                let bb = piece.bitboard(pos) & player_bb;
                squares.intersects(bb, pos)
            }
        }
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub enum RulesFenPart {
    #[default]
    None,
    Mnk(MnkSettings),
}

#[must_use]
pub(super) struct EmptyBoard(Box<dyn Fn(&RulesRef) -> UnverifiedFairyBoard + Send + Sync>);

impl Debug for EmptyBoard {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "EmptyBoardFn")
    }
}

impl Arbitrary<'_> for EmptyBoard {
    fn arbitrary(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<Self> {
        let board = UnverifiedFairyBoard {
            piece_bitboards: [RawFairyBitboard::arbitrary(u)?; MAX_NUM_PIECE_TYPES],
            color_bitboards: [RawFairyBitboard::arbitrary(u)?; NUM_COLORS],
            neutral_bb: RawFairyBitboard::arbitrary(u)?,
            mask_bb: RawFairyBitboard::arbitrary(u)?,
            in_hand: [[u8::arbitrary(u)?; MAX_NUM_PIECE_TYPES]; NUM_COLORS],
            ply_since_start: usize::arbitrary(u)?,
            num_piece_bitboards: usize::arbitrary(u)?,
            draw_counter: usize::arbitrary(u)?,
            active: FairyColor::arbitrary(u)?,
            castling_info: FairyCastleInfo::arbitrary(u)?,
            size: GridSize::arbitrary(u)?,
            ep: Option::arbitrary(u)?,
            last_move: FairyMove::arbitrary(u)?,
            rules: Default::default(),
            hash: PosHash::arbitrary(u)?,
        };
        let func = move |rules: &RulesRef| {
            let mut b = board.clone();
            b.rules = rules.clone();
            b
        };
        Ok(EmptyBoard(Box::new(func)))
    }
}

#[derive(Debug, Copy, Clone, Arbitrary)]
#[must_use]
pub enum NumRoyals {
    Exactly(usize),
    AtLeast(usize),
    BetweenInclusive(usize, usize),
}

/// This struct defined the rules for the variant.
/// Since the rules don't change during a game, but are expensive to copy and the board uses copy-make,
/// they are created once and stored behind an [`Arc`] that all boards have one copy of.
#[must_use]
#[derive(Debug, Arbitrary)]
pub(super) struct Rules {
    pub pieces: Vec<Piece>,
    pub colors: [ColorInfo; NUM_COLORS],
    pub starting_pieces_in_hand: [[u8; MAX_NUM_PIECE_TYPES]; NUM_COLORS],
    pub game_end_eager: Vec<(GameEndEager, GameEndRes)>,
    pub game_end_no_moves: Vec<(NoMovesCondition, GameEndRes)>,
    pub startpos_fen_part: String, // doesn't include the rules
    pub empty_board: EmptyBoard,
    // pub legality: Legality,
    pub size: GridSize,
    pub has_ep: bool,
    pub has_fen_hand_info: bool, // false for e.g. mnk games, which have a hand, but don't include that in the FEN.
    pub has_castling: bool,
    pub store_last_move: bool,
    pub effect_rules: EffectRules, // TODO: Remove?
    pub check_rules: CheckRules,
    pub name: String,
    pub fen_part: RulesFenPart,
    pub num_royals: [NumRoyals; NUM_COLORS],
    pub must_preserve_own_king: [bool; NUM_COLORS],
    pub observers: Observers,
}

impl Rules {
    pub(super) fn rules_fen_part(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} ", self.name)?;
        match self.fen_part {
            RulesFenPart::None => Ok(()),
            RulesFenPart::Mnk(settings) => {
                write!(f, "{settings} ")
            }
        }
    }

    pub(super) fn read_rules_fen_part(&self, input: &mut Tokens) -> Res<Option<RulesRef>> {
        let fen_part = self.fen_part;
        match fen_part {
            RulesFenPart::None => Ok(None),
            RulesFenPart::Mnk(old) => {
                let first = input.next().unwrap_or_default();
                let settings = MnkSettings::from_input(first, input)?;
                if settings != old {
                    let rules = Rules::mnk(settings.size(), settings.k() as DimT);
                    Ok(Some(RulesRef(Arc::new(rules))))
                } else {
                    Ok(None)
                }
            }
        }
    }

    pub fn pieces(&self) -> impl DoubleEndedIterator<Item = (PieceId, &Piece)> {
        self.pieces.iter().enumerate().map(|(id, piece)| (PieceId::new(id), piece))
    }

    pub fn matching_piece_ids<Pred: Fn(&Piece) -> bool + Copy>(&self, pred: Pred) -> impl Iterator<Item = PieceId> {
        self.pieces().filter(move |(_id, p)| pred(p)).map(|(id, _)| id)
    }

    pub fn royals(&self) -> impl Iterator<Item = PieceId> {
        self.matching_piece_ids(|p| p.royal)
    }

    pub fn castling(&self) -> impl Iterator<Item = PieceId> {
        self.matching_piece_ids(|p| p.can_castle)
    }

    pub fn has_halfmove_repetition_clock(&self) -> bool {
        self.game_end_eager.iter().any(|(cond, _)| matches!(cond, &GameEndEager::Repetition(_)))
    }

    pub(super) fn piece_by_name(&self, name: &str) -> Option<PieceId> {
        // case-sensitive
        self.pieces().find(|(_id, piece)| piece.name == name).map(|(id, _piece)| id)
    }

    fn generic_empty_board(rules: &RulesRef) -> UnverifiedFairyBoard {
        let size = rules.0.size;
        UnverifiedFairyBoard {
            piece_bitboards: Default::default(),
            color_bitboards: Default::default(),
            neutral_bb: Default::default(),
            mask_bb: FairyBitboard::valid_squares_for_size(size).raw(),
            in_hand: rules.0.starting_pieces_in_hand,
            ply_since_start: 0,
            num_piece_bitboards: rules.0.pieces.len(),
            draw_counter: 0,
            active: Default::default(),
            castling_info: FairyCastleInfo::new(size),
            size,
            ep: None,
            last_move: Default::default(),
            rules: rules.clone(),
            hash: PosHash::default(),
        }
    }

    // Used for mnk games and many other variants
    fn mnk_colors() -> [ColorInfo; NUM_COLORS] {
        [ColorInfo { ascii_char: 'x', name: "X".to_string() }, ColorInfo { ascii_char: 'o', name: "O".to_string() }]
    }

    // Used for chess and many other variants
    fn chess_colors() -> [ColorInfo; NUM_COLORS] {
        [
            ColorInfo { ascii_char: 'w', name: "white".to_string() },
            ColorInfo { ascii_char: 'b', name: "black".to_string() },
        ]
    }

    pub fn chess() -> Self {
        let pieces = Piece::chess_pieces();
        let colors = Self::chess_colors();
        let p = |id: usize| PieceId::new(id);
        let knight_draw = vec![(p(0), 0), (p(1), 1), (p(2), 0), (p(3), 0), (p(4), 0)];
        let bishop_draw = vec![(p(0), 0), (p(1), 0), (p(2), 1), (p(3), 0), (p(4), 0)];
        let game_end_eager = vec![
            (Repetition(3), Draw),
            (DrawCounter(100), Draw.but(GameEndRes::loss()).if_no_moves_and(InCheck)),
            (InsufficientMaterial(knight_draw, PlayerCond::All), Draw),
            (InsufficientMaterial(bishop_draw, PlayerCond::All), Draw),
        ];
        let game_end_no_moves = vec![(NotInCheck, Draw), (InCheck, InactivePlayerWin)];
        let startpos_fen_part = chess::START_FEN.to_string();
        // let legality = PseudoLegal;
        let effect_rules = EffectRules::default();
        let empty_func = Self::generic_empty_board;
        Self {
            pieces,
            colors,
            starting_pieces_in_hand: [[0; MAX_NUM_PIECE_TYPES]; NUM_COLORS],
            game_end_eager,
            game_end_no_moves,
            startpos_fen_part,
            // legality,
            empty_board: EmptyBoard(Box::new(empty_func)),
            size: FairySize::chess(),
            has_ep: true,
            has_fen_hand_info: false,
            has_castling: true,
            store_last_move: false,
            effect_rules,
            check_rules: CheckRules::chess(),
            name: "chess".to_string(),
            fen_part: RulesFenPart::None,
            num_royals: [Exactly(1); NUM_COLORS],
            must_preserve_own_king: [true; NUM_COLORS],
            observers: Observers::chess(),
        }
    }

    pub fn shatranj() -> Self {
        let pieces = Piece::shatranj_pieces();
        let colors = Self::chess_colors();
        let bare_king = No(PieceCond::NonRoyal, PlayerCond::Active);
        let game_end_eager = vec![
            (DrawCounter(100), Draw),
            (Repetition(3), Draw),
            (bare_king.clone(), GameEndRes::loss().but(Draw).if_a_move_achieves(bare_king)),
        ];
        let game_end_no_moves = vec![(Always, GameEndRes::loss())];
        // let game_loss = vec![GameEndEager::Checkmate, GameEndEager::NoMoves, GameEndEager::NoNonRoyalsExceptRecapture];
        // let draw = vec![GameEndEager::Counter(100), GameEndEager::Repetition(3)];
        let startpos_fen_part = "rnakfanr/pppppppp/8/8/8/8/PPPPPPPP/RNAKFANR w 0 1".to_string();
        // let legality = PseudoLegal;
        let effect_rules = EffectRules::default();
        Self {
            pieces,
            colors,
            starting_pieces_in_hand: [[0; MAX_NUM_PIECE_TYPES]; NUM_COLORS],
            game_end_eager,
            game_end_no_moves,
            startpos_fen_part,
            // legality,
            empty_board: EmptyBoard(Box::new(Self::generic_empty_board)),
            size: FairySize::chess(),
            has_ep: false,
            has_fen_hand_info: false,
            has_castling: false,
            store_last_move: false,
            effect_rules,
            check_rules: CheckRules::chess(),
            name: "shatranj".to_string(),
            fen_part: RulesFenPart::None,
            num_royals: [Exactly(1); NUM_COLORS],
            must_preserve_own_king: [true; NUM_COLORS],
            observers: Observers::shatranj(),
        }
    }

    pub fn king_of_the_hill() -> Self {
        let mut rules = Self::chess();
        rules.game_end_eager.retain(|(c, _r)| !matches!(c, InsufficientMaterial(_, _)));
        // moving a king to the center takes precedence over draw conditions like the 50 mr counter
        rules.game_end_eager.insert(
            0,
            (
                GameEndEager::PieceIn(PieceCond::Royal, SquareCond::Bitboard(0x1818000000), PlayerCond::Inactive),
                GameEndRes::loss(),
            ),
        );
        rules.name = "kingofthehill".to_string();
        rules
    }

    pub fn atomic() -> Self {
        let mut rules = Self::chess();
        let p = |id: usize| PieceId::new(id);
        let only_kings = vec![(p(0), 0), (p(1), 0), (p(2), 0), (p(3), 0), (p(4), 0)];
        let game_end_eager = vec![
            (Repetition(3), Draw),
            (DrawCounter(100), Draw.but(GameEndRes::loss()).if_no_moves_and(InCheck)),
            (No(PieceCond::Royal, PlayerCond::Active), GameEndRes::loss()),
            (InsufficientMaterial(only_kings, PlayerCond::All), Draw),
        ];
        let game_end_no_moves = vec![(InCheck, GameEndRes::loss()), (NotInCheck, Draw)];
        let check_rules = CheckRules {
            count: CheckCount::AnyRoyal,
            attack_condition: CheckingAttack::NoRoyalAdjacent,
            inactive_check_ok: PlayerCheckOk::OpponentNoRoyals,
            active_check_ok: PlayerCheckOk::Always,
        };
        rules.name = "atomic".to_string();
        rules.startpos_fen_part = chess::START_FEN.to_string();
        rules.game_end_eager = game_end_eager;
        rules.game_end_no_moves = game_end_no_moves;
        rules.check_rules = check_rules;
        // it's valid to lose your king, that just means you lost the game
        rules.num_royals = [BetweenInclusive(0, 1); NUM_COLORS];
        let pawn = rules.piece_by_name("pawn").unwrap();
        rules.observers = Observers::atomic(pawn);
        rules
    }

    pub fn horde() -> Self {
        let mut rules = Self::chess();
        rules.name = "horde".to_string();
        rules.startpos_fen_part =
            "rnbqkbnr/pppppppp/8/1PP2PP1/PPPPPPPP/PPPPPPPP/PPPPPPPP/PPPPPPPP w kq - 0 1".to_string();
        rules.pieces[0] = Piece::create_piece_by_name("pawn (horde)", FairySize::chess()).unwrap();
        for p in 1..5 {
            rules.pieces[0].promotions.pieces.push(PieceId::new(p));
        }
        rules.game_end_eager = vec![
            (Repetition(3), Draw),
            (DrawCounter(100), Draw.but(GameEndRes::loss()).if_no_moves_and(InCheck)),
            // white can achieve other pieces than pawns, so just checking pawns isn't enough
            (No(AnyPiece, PlayerCond::First), GameEndRes::SecondPlayerWin),
        ];
        // Only black can be in check, but the chess rules are still correct in horde. We're explicitly assigning
        // the same `vec` as in chess to avoid depending on how exactly they're expressed in the chess implementation.
        rules.game_end_no_moves = vec![(InCheck, GameEndRes::loss()), (NotInCheck, Draw)];
        rules.num_royals[0] = Exactly(0);
        rules.must_preserve_own_king[0] = false;
        rules
    }

    pub fn racing_kings(size: FairySize) -> Self {
        let mut rules = Self::chess();
        rules.name = "racingkings".to_string();
        rules.startpos_fen_part = "8/8/8/8/8/8/krbnNBRK/qrbnNBRQ w - - 0 1".to_string();
        let goal_rank = FairyBitboard::rank_for(size.height.0 - 1, size);
        let almost_goal_rank = goal_rank.south();
        let backrank_end = PieceIn(PieceCond::Royal, SquareCond::Bitboard(goal_rank.raw()), PlayerCond::Inactive);
        let backrank_draw = PieceIn(PieceCond::Royal, SquareCond::Bitboard(almost_goal_rank.raw()), PlayerCond::Active);
        let backrank_end_res = GameEndRes::loss().but(Draw).if_eager(backrank_draw);
        // a backrank win takes precedence over draw conditions
        rules.game_end_eager = vec![(backrank_end, backrank_end_res), (Repetition(3), Draw), (DrawCounter(100), Draw)];
        rules.game_end_no_moves = vec![(NotInCheck, Draw)];
        rules.check_rules.active_check_ok = PlayerCheckOk::Never;
        rules
    }

    pub fn crazyhouse() -> Self {
        let mut rules = Self::chess();
        rules.name = "crazyhouse".to_string();
        rules.startpos_fen_part = chess::START_FEN.to_string();
        rules.has_fen_hand_info = true;
        rules.observers = Observers::crazyhouse();
        for (i, p) in rules.pieces.iter_mut().enumerate() {
            let mut drop = GenPieceAttackKind::piece_drop(vec![AttackBitboardFilter::EmptySquares]);
            if p.name == "pawn" {
                drop.bitboard_filter
                    .push(AttackBitboardFilter::Bitboard(!FairyBitboard::backranks_for(rules.size).raw()))
            } else if p.name != "king" {
                p.promotions.promoted_version = Some(PieceId::new(i + 5));
            }
            p.attacks.push(drop);
        }
        let mut pieces = Piece::complete_piece_map(rules.size);
        for i in 1..5 {
            let mut piece = pieces.remove(&rules.pieces[i].name).unwrap();
            let pawn = PieceId::new(0);
            piece.promotions.promoted_from = Some(pawn);
            piece.name += " (promoted)";
            debug_assert_eq!(rules.pieces[i].promotions.promoted_version, Some(PieceId::new(rules.pieces.len())));
            rules.pieces.push(piece);
        }
        for piece in &rules.pieces[0].promotions.pieces {
            assert!(rules.pieces[piece.val() + 5].name.starts_with(&rules.pieces[piece.val()].name));
        }
        for piece in &mut rules.pieces[0].promotions.pieces {
            *piece = PieceId::new(piece.val() + 5);
        }
        rules
    }

    pub fn ataxx() -> Self {
        let size = FairySize::ataxx();
        let mut map = Piece::complete_piece_map(size);
        let piece = map.remove("ataxx").unwrap();
        let gap = map.remove("gap").unwrap();
        let pieces = vec![piece, gap];
        let startpos_fen_part = AtaxxBoard::startpos().as_fen();
        Self {
            pieces,
            colors: Self::mnk_colors(),
            starting_pieces_in_hand: [[u8::MAX; MAX_NUM_PIECE_TYPES]; NUM_COLORS],
            game_end_eager: vec![
                (Repetition(3), Draw),
                (DrawCounter(100), Draw),
                (No(AnyPiece, PlayerCond::Active), GameEndRes::loss()),
            ],
            game_end_no_moves: vec![(NoMovesCondition::NoOpponentMoves, GameEndRes::MorePieces)],
            startpos_fen_part,
            // legality: Legality::Legal,
            empty_board: EmptyBoard(Box::new(Self::generic_empty_board)),
            size,
            has_ep: false,
            has_fen_hand_info: false,
            has_castling: false,
            store_last_move: false,
            effect_rules: EffectRules { reset_draw_counter_on_capture: true, conversion_radius: 1 },
            check_rules: CheckRules::none(),
            name: "ataxx".to_string(),
            fen_part: RulesFenPart::None,
            num_royals: [Exactly(0); NUM_COLORS],
            must_preserve_own_king: [false; NUM_COLORS],
            observers: Observers::ataxx(),
        }
    }

    pub fn tictactoe() -> Self {
        Self::mnk(FairySize::tictactoe(), 3)
    }

    pub fn mnk(size: FairySize, k: DimT) -> Self {
        let piece = Piece::complete_piece_map(size).remove("mnk").unwrap();
        let pieces = vec![piece];
        let settings = MnkSettings::new(size.height, size.width, k);
        let startpos_fen_part = MNKBoard::startpos_for_settings(settings).as_fen();
        Self {
            pieces,
            colors: Self::mnk_colors(),
            starting_pieces_in_hand: [[u8::MAX; MAX_NUM_PIECE_TYPES]; NUM_COLORS],
            game_end_eager: vec![(InRowAtLeast(k as usize, PlayerCond::Inactive), GameEndRes::loss())],
            game_end_no_moves: vec![(Always, Draw)],
            startpos_fen_part,
            // legality: Legality::Legal,
            empty_board: EmptyBoard(Box::new(Self::generic_empty_board)),
            size,
            has_ep: false,
            has_fen_hand_info: false,
            has_castling: false,
            store_last_move: true,
            effect_rules: EffectRules::default(),
            check_rules: CheckRules::none(),
            name: "mnk".to_string(),
            fen_part: RulesFenPart::Mnk(settings),
            num_royals: [Exactly(0); NUM_COLORS],
            must_preserve_own_king: [false; NUM_COLORS],
            observers: Observers::mnk(),
        }
    }
}

#[must_use]
#[derive(Clone, Arbitrary)]
pub struct RulesRef(pub(super) Arc<Rules>);

impl RulesRef {
    pub(super) fn new(rules: Rules) -> Self {
        Self(Arc::new(rules))
    }

    pub fn empty_pos(&self) -> UnverifiedFairyBoard {
        (self.0.empty_board.0)(self)
    }
}

// deriving Debug would debug-print the entire rules object, but we generally care about pointer equality
impl Debug for RulesRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "RulesRef({:?})", Arc::as_ptr(&self.0))
    }
}

impl Default for RulesRef {
    fn default() -> Self {
        RulesRef(DEFAULT_FAIRY_RULES.clone())
    }
}

impl PartialEq for RulesRef {
    fn eq(&self, other: &Self) -> bool {
        // if the fen prefix describing the rules is identical, so are the rules
        self.0.name == other.0.name && self.0.fen_part == other.0.fen_part
    }
}

impl Eq for RulesRef {}

impl Hash for RulesRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&self.0.name, &self.0.fen_part).hash(state);
    }
}

impl Settings for RulesRef {
    fn text(&self) -> Option<String> {
        Some(format!("Variant: {}", self.0.name))
    }
}

static DEFAULT_FAIRY_RULES: LazyLock<Arc<Rules>> = LazyLock::new(|| Arc::new(Rules::chess()));
