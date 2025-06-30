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
use crate::games::fairy::attacks::{AttackKind, Dir, EffectRules};
use crate::games::fairy::effects::Observers;
use crate::games::fairy::moves::FairyMove;
use crate::games::fairy::pieces::{CHESS_KING_IDX, CHESS_PAWN_IDX, Piece, PieceId};
use crate::games::fairy::rules::GameEndEager::{
    AdditionalCounter, And, CanAchieve, DrawCounter, InsufficientMaterial, NoPiece, Not, Repetition,
};
use crate::games::fairy::rules::GameEndEager::{InRowAtLeast, PieceIn};
use crate::games::fairy::rules::GameEndRes::{
    ActivePlayerWin, Draw, FirstPlayerWin, InactivePlayerWin, SecondPlayerWin,
};
use crate::games::fairy::rules::NoMovesCondition::{Always, InCheck, NotInCheck};
use crate::games::fairy::rules::NumRoyals::{BetweenInclusive, Exactly};
use crate::games::fairy::rules::PieceCond::{AnyPiece, Royal};
use crate::games::fairy::{
    AdditionalCtrT, ColorInfo, FairyBitboard, FairyBoard, FairyCastleInfo, FairyColor, FairySize, MAX_NUM_PIECE_TYPES,
    RawFairyBitboard, UnverifiedFairyBoard,
};
use crate::games::mnk::{MNKBoard, MnkSettings};
use crate::games::{BoardHistory, Color, DimT, NUM_COLORS, PosHash, Settings, chess, n_fold_repetition};
use crate::general::bitboards::{Bitboard, RawBitboard};
use crate::general::board::{BitboardBoard, Board, BoardHelpers};
use crate::general::common::{Res, Tokens};
use crate::general::move_list::MoveList;
use crate::general::moves::Legality::{Legal, PseudoLegal};
use crate::general::moves::{Legality, Move};
use crate::general::squares::GridSize;
use arbitrary::Arbitrary;
use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
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
    pub fn bb(&self, pos: &FairyBoard) -> FairyBitboard {
        match self {
            AnyPiece => pos.either_player_bb(),
            Royal => pos.royal_bb(),
            PieceCond::NonRoyal => pos.either_player_bb() & !pos.royal_bb(),
            PieceCond::Only(piece) => pos.piece_bb(*piece),
            PieceCond::OneOf(list) => {
                list.iter().map(|&p| pos.piece_bb(p)).fold(pos.zero_bitboard(), std::ops::BitOr::bitor)
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub enum SquareFilter {
    NoSquares,
    EmptySquares,
    Them,
    Us,
    NotUs,
    EitherPlayer,
    Bitboard(RawFairyBitboard),
    // the bitboard gets flipped vertically for the second player
    SideRelativeBitboard(RawFairyBitboard),
    Rank(DimT),
    // File(DimT),
    Neighbor(Box<SquareFilter>), // a piece of the given color must be on an adjacent square
    InDirectionOf(Box<SquareFilter>, Dir),
    SameFile(Box<SquareFilter>),
    SameRow(Box<SquareFilter>),
    PawnCapture, // Them | {ep_square}
    Has(PieceCond, PlayerCond),
    Not(Box<SquareFilter>),
}

impl SquareFilter {
    pub fn bb(&self, us: FairyColor, pos: &FairyBoard) -> FairyBitboard {
        match self {
            SquareFilter::NoSquares => pos.zero_bitboard(),
            SquareFilter::EmptySquares => pos.empty_bb(),
            SquareFilter::Them => pos.player_bb(!us),
            SquareFilter::Us => pos.player_bb(us),
            SquareFilter::NotUs => !pos.player_bb(us),
            SquareFilter::EitherPlayer => pos.either_player_bb(),
            SquareFilter::Bitboard(bb) => FairyBitboard::new(*bb, pos.size()),
            SquareFilter::SideRelativeBitboard(bb) => {
                FairyBitboard::new(*bb, pos.size()).flip_if(!pos.active.is_first())
            }
            SquareFilter::Rank(rank) => FairyBitboard::rank_for(*rank, pos.size()),
            // AttackBitboardFilter::File(file) => FairyBitboard::file_for(file, pos.size()),
            SquareFilter::Neighbor(nested) => nested.bb(us, pos).moore_neighbors(),
            SquareFilter::InDirectionOf(nested, dir) => dir.shift(nested.bb(us, pos)),
            SquareFilter::SameFile(cond) => cond.bb(us, pos).files_containing(),
            SquareFilter::SameRow(cond) => cond.bb(us, pos).ranks_containing(),
            SquareFilter::PawnCapture => {
                let ep_bb =
                    pos.0.ep.map(|sq| FairyBitboard::single_piece_for(sq, pos.size())).unwrap_or(pos.zero_bitboard());
                ep_bb | pos.player_bb(!us)
            }
            SquareFilter::Has(piece, player) => piece.bb(pos) & player.bb(pos),
            SquareFilter::Not(condition) => !condition.bb(us, pos) & pos.mask_bb(),
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
    // the second array entry if the condition is satisfied, otherwise the first entry
    If(GameEndEager, Box<[GameEndRes; 2]>),
    IfNoMovesAnd(NoMovesCondition, Box<[GameEndRes; 2]>),
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
    // This means the start of a full move
    FirstAndActive,
}

impl PlayerCond {
    pub fn bb(self, pos: &FairyBoard) -> FairyBitboard {
        match self {
            PlayerCond::All => pos.either_player_bb(),
            PlayerCond::FirstAndActive => {
                if pos.active_player().is_first() {
                    pos.active_player_bb()
                } else {
                    pos.zero_bitboard()
                }
            }
            _ => pos.player_bb(self.color(pos)),
        }
    }
    pub fn color(self, pos: &FairyBoard) -> FairyColor {
        match self {
            PlayerCond::All => {
                unreachable!("PlayerCond::All can't be converted to a color")
            }
            PlayerCond::FirstAndActive => {
                unreachable!("PlayerCond::FirstAndActive can't be converted to a color")
            }
            PlayerCond::First => FairyColor::first(),
            PlayerCond::Second => FairyColor::second(),
            PlayerCond::Active => pos.active_player(),
            PlayerCond::Inactive => pos.inactive_player(),
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
                if FairyMove::legality(pos.settings()) == PseudoLegal {
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
    NoPiece(PieceCond, PlayerCond),
    // If there is a last move, it caused the now inactive player to have `k` pieces in a row, otherwise the given player
    // has at least `k` pieces in a row somewhere
    InRowAtLeast(usize, PlayerCond),
    /// a.k.a. the 50 move rule (expressed in ply)
    DrawCounter(usize),
    /// An additional counter has reached the maximum value (given in the settings)
    AdditionalCounter,
    /// a.k.a. the threefold repetition rule
    Repetition(usize),
    /// All the given pieces occur at most the given count many times for the given player
    InsufficientMaterial(Vec<(PieceId, usize)>, PlayerCond),
    /// The given player is in check
    InCheck(PlayerCond),
    /// If any player has a given piece on a given square, they win
    PieceIn(PieceCond, SquareFilter, PlayerCond),
    /// If both conditions are satisfied. Lazily evaluates the second condition
    And(Box<[GameEndEager; 2]>),
    /// The given condition is not satisfied
    Not(Box<GameEndEager>),
    /// There is a legal moves that achieves the given condition.
    /// For obvious reasons, this is a very slow condition to check, so it should only be used as a last resort
    /// or as second part of an `And` where the first condition is usually false.
    CanAchieve(Box<GameEndEager>),
}

impl GameEndEager {
    pub fn satisfied<H: BoardHistory>(&self, pos: &FairyBoard, history: &H) -> bool {
        let us = pos.active_player();

        match self {
            NoPiece(piece, player) => {
                let player_bb = player.bb(pos);
                (piece.bb(pos) & player_bb).is_zero()
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
            AdditionalCounter => {
                for i in 0..2 {
                    if pos.additional_ctrs[i] >= pos.rules().ctr_threshold[i].unwrap_or(AdditionalCtrT::MAX) {
                        return true;
                    }
                }
                false
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
            GameEndEager::InCheck(player) => match player {
                PlayerCond::All => {
                    pos.is_player_in_check(FairyColor::first()) && pos.is_player_in_check(FairyColor::second())
                }
                PlayerCond::FirstAndActive => pos.active_player().is_first() && pos.is_in_check(),
                c => pos.is_player_in_check(c.color(pos)),
            },
            PieceIn(piece, squares, player) => {
                let bb = piece.bb(pos) & player.bb(pos);
                squares.bb(pos.active_player(), pos).intersects(bb.raw())
            }
            CanAchieve(cond) => {
                let mut h = history.clone();
                h.push(pos.hash_pos());
                pos.children().any(|c| cond.satisfied(&c, &h))
            }
            And(conds) => conds[0].satisfied(pos, history) && conds[1].satisfied(pos, history),
            Not(cond) => !cond.satisfied(pos, history),
        }
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub enum FenRulesPart {
    #[default]
    None,
    Mnk(MnkSettings),
    CFour(MnkSettings),
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
            in_check: [bool::arbitrary(u)?, bool::arbitrary(u)?],
            additional_ctrs: [0; NUM_COLORS],
            active: FairyColor::arbitrary(u)?,
            castling_info: FairyCastleInfo::arbitrary(u)?,
            fen_format: FenFormat::arbitrary(u)?,
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
pub enum MoveCondition {
    Capture,
    // Check,
}

impl MoveCondition {
    pub fn applies(self, mov: FairyMove) -> bool {
        match self {
            MoveCondition::Capture => mov.is_capture(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
pub enum FilterMovesCondition {
    NoFilter,
    /// If there is at least one legal move that satisfies the given condition, only moves
    /// that satisfy this condition can be played (e.g. forced captures in antichess).
    /// Much simpler to implement in pseudolegal games
    Any(MoveCondition),
}

impl FilterMovesCondition {
    pub fn apply<T: MoveList<FairyBoard>>(&self, list: &mut T, pos: &FairyBoard) {
        match self {
            FilterMovesCondition::NoFilter => (),
            FilterMovesCondition::Any(condition) => {
                if list.iter_moves().any(|m| {
                    condition.applies(*m) && (pos.rules().legality == Legal || pos.is_pseudolegal_move_legal(*m))
                }) {
                    list.filter_moves(|m| condition.applies(*m));
                }
            }
        }
    }
}

/// The board will first try to parse a FEN with the selected format (standard by default (TODO: Changeable with an uci option)),
/// if that fails it will try the other format and change the selected format. The board is always written with the selected format.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub(super) enum FenFormat {
    #[default]
    Standard,
    // e.g. sfen in shogi, but not shredder fen instead of x-fen in chess because that can be the same fen string
    Alternative,
}

/// Pieces only contain promotion information if that is relevant for the game, so e.g. a chess queen that was promoted from
/// a pawn is just a normal queen. For pieces with this information, this enum describes how the piece is formatted in a FEN.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub(super) enum PromoFenModifier {
    /// Output a trailing `~` after the piece, e.g. `Q~` in chess
    Crazyhouse,
    /// Output a leading `+` before the unpromoted version of the piece
    Shogi,
}

/// How the compact text format (i.e., UGI) signifies a promotion
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub(super) enum PromoMoveChar {
    /// Output the ascii piece char after the dest square, e.g. `e7e8Q` in chess
    Piece,
    /// Output a trailing `+` after the dest square, e.g. `e6e7+` in shogi
    Plus,
}

pub(super) const NUM_FEN_FORMATS: usize = 2;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub(super) enum FenHandInfo {
    /// e.g. in mnk games, which have a hand, but don't include that in the FEN, or chess, which doesn't have a hand.
    None,
    /// e.g in crazyhouse, where the hand appears at the end of the position token, wrapped in `[]`
    InBrackets,
    /// e.g. in shogi sfen
    SeparateToken,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
pub(super) struct FenFormatSpec {
    pub(super) hand: FenHandInfo,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[must_use]
/// How to format FENs and moves.
/// If parsing a FEN fails, the board switches the format and tries again, if that succeeds it keeps the new format.
pub(super) struct FormatRules {
    pub(super) has_halfmove_ctr: bool,
    // TODO: Doens't really belong in format rules since it's just used to set up the startpos but doesn't depend on format rules
    pub(super) startpos_fen: String, // doesn't include the rules
    pub(super) rules_part: FenRulesPart,
    pub(super) hand: FenHandInfo,
    pub(super) promo_fen_modifier: PromoFenModifier,
    pub(super) promo_move_char: PromoMoveChar,
    // pub(super) formats: [FenFormatSpec; NUM_FEN_FORMATS],
}

impl FormatRules {
    pub(super) fn write_rules_part(&self, f: &mut Formatter, name: &str) -> fmt::Result {
        write!(f, "{name} ")?;
        match self.rules_part {
            FenRulesPart::None => Ok(()),
            FenRulesPart::Mnk(settings) => {
                write!(f, "{settings} ")
            }
            FenRulesPart::CFour(settings) => {
                write!(f, "{settings} ")
            }
        }
    }

    pub(super) fn read_rules_part(&self, input: &mut Tokens) -> Res<Option<RulesRef>> {
        let fen_part = self.rules_part;
        match fen_part {
            FenRulesPart::None => Ok(None),
            FenRulesPart::Mnk(old) => {
                let first = input.next().unwrap_or_default();
                let settings = MnkSettings::from_input(first, input)?;
                if settings != old {
                    let rules = Rules::mnk(settings.size(), settings.k() as DimT);
                    Ok(Some(RulesRef(Arc::new(rules))))
                } else {
                    Ok(None)
                }
            }
            FenRulesPart::CFour(old) => {
                let first = input.next().unwrap_or_default();
                let settings = MnkSettings::from_input(first, input)?;
                if settings != old {
                    let rules = Rules::cfour(settings.size(), settings.k() as DimT);
                    Ok(Some(RulesRef(Arc::new(rules))))
                } else {
                    Ok(None)
                }
            }
        }
    }
}

/// This struct defined the rules for the variant.
/// Since the rules don't change during a game, but are expensive to copy and the board uses copy-make,
/// they are created once and stored behind an [`Arc`] that all boards have one copy of.
#[must_use]
#[derive(Debug, Arbitrary)]
pub struct Rules {
    pub(super) format_rules: FormatRules,
    pub(super) pieces: Vec<Piece>,
    pub(super) colors: [ColorInfo; NUM_COLORS],
    pub(super) starting_pieces_in_hand: [[u8; MAX_NUM_PIECE_TYPES]; NUM_COLORS],
    pub(super) game_end_eager: Vec<(GameEndEager, GameEndRes)>,
    pub(super) game_end_no_moves: Vec<(NoMovesCondition, GameEndRes)>,
    pub(super) empty_board: EmptyBoard,
    /// setting this to [`Legal`] can be a speedup, but setting it to [`PseudoLegal`] is always correct.
    pub(super) legality: Legality,
    pub(super) moves_filter: FilterMovesCondition,
    pub(super) size: GridSize,
    pub(super) has_ep: bool,
    pub(super) has_castling: bool,
    pub(super) store_last_move: bool,
    pub(super) ctr_threshold: [Option<AdditionalCtrT>; NUM_COLORS],
    pub(super) effect_rules: EffectRules, // TODO: Remove?
    pub(super) check_rules: CheckRules,
    pub(super) name: String,
    pub(super) num_royals: [NumRoyals; NUM_COLORS],
    pub(super) must_preserve_own_king: [bool; NUM_COLORS],
    pub(super) observers: Observers,
}

impl Settings for Rules {
    fn text(&self) -> Option<String> {
        Some(format!("Variant: {}", self.name))
    }
}

impl Rules {
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

    pub fn has_additional_ctr(&self) -> bool {
        self.ctr_threshold.iter().any(|c| c.is_some())
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
            in_check: [false; NUM_COLORS],
            additional_ctrs: [0; NUM_COLORS],
            active: Default::default(),
            castling_info: FairyCastleInfo::new(size),
            fen_format: FenFormat::Standard,
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

        let effect_rules = EffectRules::default();
        let empty_func = Self::generic_empty_board;
        // let fen_format = FenFormatSpec { hand: FenHandInfo::None };
        let fen_rules = FormatRules {
            has_halfmove_ctr: true,
            startpos_fen: chess::START_FEN.to_string(),
            rules_part: FenRulesPart::None,
            // formats: [fen_format; 2],
            hand: FenHandInfo::None,
            promo_fen_modifier: PromoFenModifier::Crazyhouse,
            promo_move_char: PromoMoveChar::Piece,
        };
        Self {
            format_rules: fen_rules,
            pieces,
            colors,
            starting_pieces_in_hand: [[0; MAX_NUM_PIECE_TYPES]; NUM_COLORS],
            game_end_eager,
            game_end_no_moves,
            empty_board: EmptyBoard(Box::new(empty_func)),
            legality: PseudoLegal,
            moves_filter: FilterMovesCondition::NoFilter,
            size: FairySize::chess(),
            has_ep: true,
            has_castling: true,
            store_last_move: false,
            ctr_threshold: [None; NUM_COLORS],
            effect_rules,
            check_rules: CheckRules::chess(),
            name: "chess".to_string(),
            num_royals: [Exactly(1); NUM_COLORS],
            must_preserve_own_king: [true; NUM_COLORS],
            observers: Observers::chess(),
        }
    }

    pub fn shatranj() -> Self {
        let mut rules = Self::chess();
        rules.name = "shatranj".to_string();
        rules.format_rules.startpos_fen = "rnakfanr/pppppppp/8/8/8/8/PPPPPPPP/RNAKFANR w 0 1".to_string();
        rules.pieces = Piece::shatranj_pieces();
        let bare_king = NoPiece(PieceCond::NonRoyal, PlayerCond::Active);
        rules.game_end_eager = vec![
            (DrawCounter(100), Draw),
            (Repetition(3), Draw),
            (bare_king.clone(), GameEndRes::loss().but(Draw).if_a_move_achieves(bare_king)),
        ];
        rules.game_end_no_moves = vec![(Always, GameEndRes::loss())];
        rules.has_ep = false;
        rules.has_castling = false;
        rules
    }

    pub fn king_of_the_hill() -> Self {
        let mut rules = Self::chess();
        rules.game_end_eager.retain(|(c, _r)| !matches!(c, InsufficientMaterial(_, _)));
        // moving a king to the center takes precedence over draw conditions like the 50 mr counter
        rules.game_end_eager.insert(
            0,
            (PieceIn(Royal, SquareFilter::Bitboard(0x1818000000), PlayerCond::Inactive), GameEndRes::loss()),
        );
        rules.name = "kingofthehill".to_string();
        rules
    }

    pub fn atomic() -> Self {
        let mut rules = Self::chess();
        let p = |id: usize| PieceId::new(id);
        let only_kings = vec![(p(0), 0), (p(1), 0), (p(2), 0), (p(3), 0), (p(4), 0)];
        rules.game_end_eager = vec![
            (Repetition(3), Draw),
            (DrawCounter(100), Draw.but(GameEndRes::loss()).if_no_moves_and(InCheck)),
            (NoPiece(Royal, PlayerCond::Active), GameEndRes::loss()),
            (InsufficientMaterial(only_kings, PlayerCond::All), Draw),
        ];
        rules.game_end_no_moves = vec![(InCheck, GameEndRes::loss()), (NotInCheck, Draw)];
        rules.check_rules = CheckRules {
            count: CheckCount::AnyRoyal,
            attack_condition: CheckingAttack::NoRoyalAdjacent,
            inactive_check_ok: PlayerCheckOk::OpponentNoRoyals,
            active_check_ok: PlayerCheckOk::Always,
        };
        rules.name = "atomic".to_string();
        // it's valid to lose your king, that just means you lost the game
        rules.num_royals = [BetweenInclusive(0, 1); NUM_COLORS];
        let pawn = rules.piece_by_name("pawn").unwrap();
        rules.observers = Observers::atomic(pawn);
        rules
    }

    pub fn horde() -> Self {
        let mut rules = Self::chess();
        rules.name = "horde".to_string();
        rules.format_rules.startpos_fen =
            "rnbqkbnr/pppppppp/8/1PP2PP1/PPPPPPPP/PPPPPPPP/PPPPPPPP/PPPPPPPP w kq - 0 1".to_string();
        rules.pieces[0] = Piece::create_piece_by_name("pawn (horde)", FairySize::chess()).unwrap();
        for p in 1..5 {
            rules.pieces[0].promotions.pieces.push(PieceId::new(p));
        }
        rules.game_end_eager = vec![
            (Repetition(3), Draw),
            (DrawCounter(100), Draw.but(GameEndRes::loss()).if_no_moves_and(InCheck)),
            // white can achieve other pieces than pawns, so just checking pawns isn't enough
            (NoPiece(AnyPiece, PlayerCond::First), GameEndRes::SecondPlayerWin),
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
        rules.format_rules.startpos_fen = "8/8/8/8/8/8/krbnNBRK/qrbnNBRQ w - - 0 1".to_string();
        let goal_rank = FairyBitboard::rank_for(size.height.0 - 1, size);
        let goal = SquareFilter::Bitboard(goal_rank.raw());
        // this is checked first, so that it's a draw if both kings have reached the backrank
        let backrank_black = PieceIn(Royal, goal.clone(), PlayerCond::Second);
        let backrank_white_active = PieceIn(Royal, goal.clone(), PlayerCond::FirstAndActive);
        // the last win condition because it's expensive to check and relies on the other conditions being false
        let backrand_white_inactive = And(Box::new([
            PieceIn(Royal, goal.clone(), PlayerCond::First),
            Not(Box::new(CanAchieve(Box::new(PieceIn(Royal, goal.clone(), PlayerCond::Second))))),
        ]));
        let backrank_end_res = SecondPlayerWin.but(Draw).if_eager(PieceIn(Royal, goal, PlayerCond::First));
        // a backrank win takes precedence over draw conditions
        rules.game_end_eager = vec![
            (backrank_black, backrank_end_res),
            (backrank_white_active, FirstPlayerWin),
            (backrand_white_inactive, FirstPlayerWin),
            (Repetition(3), Draw),
            (DrawCounter(100), Draw),
        ];
        rules.game_end_no_moves = vec![(NotInCheck, Draw)];
        rules.check_rules.active_check_ok = PlayerCheckOk::Never;
        rules
    }

    pub fn crazyhouse() -> Self {
        let mut rules = Self::chess();
        rules.name = "crazyhouse".to_string();
        rules.format_rules.hand = FenHandInfo::InBrackets;
        rules.observers = Observers::crazyhouse();
        for (i, p) in rules.pieces.iter_mut().enumerate() {
            let mut drop = AttackKind::drop(vec![SquareFilter::EmptySquares]);
            if p.name == "pawn" {
                drop.bitboard_filter.push(SquareFilter::Bitboard(!FairyBitboard::backranks_for(rules.size).raw()))
            } else if p.name != "king" {
                p.promotions.promoted_version = Some(PieceId::new(i + 5));
            }
            p.attacks.push(drop);
        }
        let mut pieces = Piece::complete_piece_map(rules.size);
        for i in CHESS_PAWN_IDX + 1..5 {
            let mut piece = pieces.remove(&rules.pieces[i].name).unwrap();
            let pawn = PieceId::new(CHESS_PAWN_IDX);
            piece.promotions.promoted_from = Some(pawn);
            piece.name += " (promoted)";
            debug_assert_eq!(rules.pieces[i].promotions.promoted_version, Some(PieceId::new(rules.pieces.len())));
            rules.pieces.push(piece);
        }
        for piece in &rules.pieces[CHESS_PAWN_IDX].promotions.pieces {
            assert!(rules.pieces[piece.val() + 5].name.starts_with(&rules.pieces[piece.val()].name));
        }
        for piece in &mut rules.pieces[CHESS_PAWN_IDX].promotions.pieces {
            *piece = PieceId::new(piece.val() + 5);
        }
        rules
    }

    pub fn n_check(n: usize) -> Self {
        let mut rules = Self::chess();
        rules.name = format!("{n}check");
        rules.observers = Observers::n_check();
        rules.game_end_eager.push((AdditionalCounter, InactivePlayerWin));
        rules.ctr_threshold = [Some(n as AdditionalCtrT); 2];
        rules
    }

    pub fn antichess() -> Self {
        let mut rules = Self::chess();
        rules.name = "antichess".to_string();
        let king = &mut rules.pieces[CHESS_KING_IDX];
        king.royal = false;
        king.can_castle = false;
        let pawn = &mut rules.pieces[CHESS_PAWN_IDX];
        pawn.promotions.pieces.push(PieceId::new(CHESS_KING_IDX));
        rules.game_end_no_moves = vec![(Always, ActivePlayerWin)];
        // TODO: Insufficient material for opposite-colored bishops
        rules.game_end_eager = vec![(DrawCounter(50), Draw), (Repetition(3), Draw)];
        rules.must_preserve_own_king = [false; 2];
        rules.num_royals = [Exactly(0); 2];
        rules.moves_filter = FilterMovesCondition::Any(MoveCondition::Capture);
        rules
    }

    pub fn shogi() -> Self {
        // TODO: The board is visualized flipped both horizontally and vertically, and moves also use that notation
        let mut rules = Self::chess();
        rules.name = "shogi".to_string();
        rules.format_rules.startpos_fen = "lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL w - 1".to_string();
        rules.format_rules.has_halfmove_ctr = false;
        rules.format_rules.hand = FenHandInfo::InBrackets;
        rules.format_rules.promo_fen_modifier = PromoFenModifier::Shogi;
        rules.format_rules.promo_move_char = PromoMoveChar::Plus;
        rules.size = FairySize::shogi();
        // TODO: Flip color chars when using sfens
        rules.colors[0] = ColorInfo { ascii_char: 'w', name: "sente".to_string() };
        rules.colors[1] = ColorInfo { ascii_char: 'b', name: "gote".to_string() };
        rules.pieces = Piece::shogi_pieces();
        rules.has_castling = false;
        // shogi doesn't actually have ep captures, but this makes FENs contain a `-`, which they for some reason have to
        // in the standard FEN format cutechess uses TODO: fairy sf's fen output also contains another - and a repetition clock, so accept that...
        rules.has_ep = true;
        rules.observers = Observers::crazyhouse();
        // TODO: this is incorrect. As far as my current understanding goes, a fourfold repetition is a draw,
        // but if the last 4 moves have all been checks and the same position, the checking player loses
        rules.game_end_eager =
            vec![(Repetition(4), Draw.but(GameEndRes::win()).if_eager(GameEndEager::InCheck(PlayerCond::Active)))];
        rules.game_end_no_moves = vec![(Always, InactivePlayerWin)];
        rules
    }

    pub fn ataxx() -> Self {
        let size = FairySize::ataxx();
        let mut map = Piece::complete_piece_map(size);
        let piece = map.remove("ataxx").unwrap();
        let gap = map.remove("gap").unwrap();
        let pieces = vec![piece, gap];
        let startpos_fen = AtaxxBoard::startpos().as_fen();
        let fen_rules = FormatRules {
            hand: FenHandInfo::None,
            has_halfmove_ctr: true,
            startpos_fen,
            rules_part: FenRulesPart::None,
            promo_fen_modifier: PromoFenModifier::Crazyhouse,
            promo_move_char: PromoMoveChar::Piece,
        };
        Self {
            format_rules: fen_rules,
            pieces,
            colors: Self::mnk_colors(),
            starting_pieces_in_hand: [[u8::MAX; MAX_NUM_PIECE_TYPES]; NUM_COLORS],
            game_end_eager: vec![
                (Repetition(3), Draw),
                (DrawCounter(100), Draw),
                (NoPiece(AnyPiece, PlayerCond::Active), GameEndRes::loss()),
            ],
            game_end_no_moves: vec![(NoMovesCondition::NoOpponentMoves, GameEndRes::MorePieces)],
            legality: Legal,
            empty_board: EmptyBoard(Box::new(Self::generic_empty_board)),
            size,
            has_ep: false,
            has_castling: false,
            store_last_move: false,
            ctr_threshold: [None; NUM_COLORS],
            effect_rules: EffectRules { reset_draw_counter_on_capture: true, conversion_radius: 1 },
            check_rules: CheckRules::none(),
            name: "ataxx".to_string(),
            num_royals: [Exactly(0); NUM_COLORS],
            must_preserve_own_king: [false; NUM_COLORS],
            observers: Observers::ataxx(),
            moves_filter: FilterMovesCondition::NoFilter,
        }
    }

    pub fn tictactoe() -> Self {
        Self::mnk(FairySize::tictactoe(), 3)
    }

    pub fn mnk(size: FairySize, k: DimT) -> Self {
        let piece = Piece::complete_piece_map(size).remove("mnk").unwrap();
        let pieces = vec![piece];
        let settings = MnkSettings::new(size.height, size.width, k);
        let startpos_fen = MNKBoard::startpos_for_settings(settings).as_fen();
        let fen_rules = FormatRules {
            hand: FenHandInfo::None,
            has_halfmove_ctr: false,
            startpos_fen,
            rules_part: FenRulesPart::Mnk(settings),
            promo_fen_modifier: PromoFenModifier::Crazyhouse,
            promo_move_char: PromoMoveChar::Piece,
        };
        Self {
            format_rules: fen_rules,
            pieces,
            colors: Self::mnk_colors(),
            starting_pieces_in_hand: [[u8::MAX; MAX_NUM_PIECE_TYPES]; NUM_COLORS],
            game_end_eager: vec![(InRowAtLeast(k as usize, PlayerCond::Inactive), GameEndRes::loss())],
            game_end_no_moves: vec![(Always, Draw)],
            legality: Legal,
            empty_board: EmptyBoard(Box::new(Self::generic_empty_board)),
            size,
            has_ep: false,
            has_castling: false,
            store_last_move: true,
            ctr_threshold: [None; NUM_COLORS],
            effect_rules: EffectRules::default(),
            check_rules: CheckRules::none(),
            name: "mnk".to_string(),
            num_royals: [Exactly(0); NUM_COLORS],
            must_preserve_own_king: [false; NUM_COLORS],
            observers: Observers::mnk(),
            moves_filter: FilterMovesCondition::NoFilter,
        }
    }

    pub fn cfour(size: FairySize, k: DimT) -> Self {
        let mut res = Self::mnk(size, k);
        res.name = "cfour".to_string();
        res.pieces = vec![Piece::create_piece_by_name("cfour", size).unwrap()];
        res.format_rules.rules_part = FenRulesPart::CFour(MnkSettings::new(size.height, size.width, k));
        res
    }
}

#[must_use]
#[derive(Clone, Arbitrary)]
pub struct RulesRef(Arc<Rules>);

impl RulesRef {
    pub(super) fn new(rules: Rules) -> Self {
        Self(Arc::new(rules))
    }

    pub fn empty_pos(&self) -> UnverifiedFairyBoard {
        (self.0.empty_board.0)(self)
    }

    pub fn get(&self) -> &Rules {
        Arc::deref(&self.0)
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
        self.0.name == other.0.name && self.0.format_rules == other.0.format_rules
    }
}

impl Eq for RulesRef {}

impl Hash for RulesRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&self.0.name, &self.0.format_rules).hash(state);
    }
}

static DEFAULT_FAIRY_RULES: LazyLock<Arc<Rules>> = LazyLock::new(|| Arc::new(Rules::chess()));
