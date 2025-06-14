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
use crate::games::fairy::attacks::EffectRules;
use crate::games::fairy::effects::Observers;
use crate::games::fairy::moves::FairyMove;
use crate::games::fairy::pieces::{Piece, PieceId};
use crate::games::fairy::rules::GameEndEager::{
    DrawCounter, InsufficientMaterial, NoNonRoyalsExceptRecapture, Repetition,
};
use crate::games::fairy::rules::GameEndEager::{InRowAtLeast, NoPieces};
use crate::games::fairy::rules::GameEndResult::{ActivePlayerWin, Draw, DrawUnlessLossOn, InactivePlayerWin};
use crate::games::fairy::rules::NoMovesCondition::{Always, InCheck, NotInCheck};
use crate::games::fairy::rules::NumRoyals::Exactly;
use crate::games::fairy::{
    ColorInfo, FairyBitboard, FairyBoard, FairyCastleInfo, FairyColor, FairySize, MAX_NUM_PIECE_TYPES,
    RawFairyBitboard, UnverifiedFairyBoard,
};
use crate::games::mnk::{MNKBoard, MnkSettings};
use crate::games::{BoardHistory, DimT, NUM_COLORS, PosHash, Settings, chess, n_fold_repetition};
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
use std::ops::Deref;
use std::sync::{Arc, LazyLock};

/// Whether any or all royal pieces have to be attacked for the player to be considered in check
#[derive(Debug, Default, Copy, Clone, Arbitrary)]
pub enum CheckRules {
    #[default]
    AnyRoyal,
    #[allow(dead_code)] // TODO: Variant with multiple royal pieces
    AllRoyals,
}

/// When a game end condition is met (either a [`NoMovesCondition`] or a [`GameEndEager`] condition),
/// this enum determines who wins.
/// The [`Rules`] struct contains a `Vec` of `(NoMoveCondition, GameEndResult)` pairs and a `Vec` of `(GameEndEager, GameEndResult)` pairs.
/// Conditions are checked in order; the first that matches determines the result.
#[derive(Debug, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub enum GameEndResult {
    ActivePlayerWin,   // a.k.a. Win
    InactivePlayerWin, // a.k.a. Lose
    Draw,
    MorePieces,
    DrawUnlessLossOn(NoMovesCondition),
}

impl GameEndResult {
    #[allow(unused)]
    pub fn win() -> Self {
        ActivePlayerWin
    }

    pub fn lose() -> Self {
        InactivePlayerWin
    }
}

impl GameEndResult {
    pub fn to_res(&self, pos: &FairyBoard) -> PlayerResult {
        match self {
            ActivePlayerWin => PlayerResult::Win,
            InactivePlayerWin => PlayerResult::Lose,
            Draw => PlayerResult::Draw,
            GameEndResult::MorePieces => {
                match pos.active_player_bb().num_ones().cmp(&pos.inactive_player_bb().num_ones()) {
                    Ordering::Less => PlayerResult::Lose,
                    Ordering::Equal => PlayerResult::Draw,
                    Ordering::Greater => PlayerResult::Win,
                }
            }

            GameEndResult::DrawUnlessLossOn(no_moves_res) => {
                if pos.has_no_legal_moves() && no_moves_res.satisfied(pos) {
                    PlayerResult::Lose
                } else {
                    PlayerResult::Draw
                }
            }
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
    NoRoyals,
    NoPieces,
    NoNonRoyals,
    NoNonRoyalsExceptRecapture,
    // The last move caused the now inactive player to have `k` pieces in a row
    InRowAtLeast(usize),
    DrawCounter(usize),
    Repetition(usize),
    InsufficientMaterial(Vec<(PieceId, usize)>),
}

impl GameEndEager {
    pub fn satisfied<H: BoardHistory>(&self, pos: &FairyBoard, history: &H) -> bool {
        let us = pos.active_player();
        match self {
            GameEndEager::NoRoyals => pos.royal_bb_for(us).is_zero(),
            GameEndEager::NoPieces => pos.active_player_bb().is_zero(),
            GameEndEager::NoNonRoyals => (pos.active_player_bb() & !pos.royal_bb()).is_zero(),
            GameEndEager::NoNonRoyalsExceptRecapture => {
                let has_nonroyals = (pos.active_player_bb() & !pos.royal_bb()).has_set_bit();
                if has_nonroyals {
                    false
                } else {
                    let their_nonroyals = pos.inactive_player_bb() & !pos.royal_bb();
                    if their_nonroyals.num_ones() > 1 {
                        true
                    } else {
                        let capturable = their_nonroyals & !pos.capturing_attack_bb_of(us);
                        capturable.has_set_bit()
                    }
                }
            }
            &GameEndEager::InRowAtLeast(k) => {
                let mut res = false;
                if pos.0.last_move.is_null() {
                    for sq in pos.inactive_player_bb().ones() {
                        res |= pos.k_in_row_at(k, sq, !us);
                    }
                    res
                } else {
                    let sq = pos.0.last_move.dest_square_in(pos);
                    pos.k_in_row_at(k, sq, !us)
                }
            }
            &GameEndEager::DrawCounter(max) => pos.0.draw_counter >= max,
            &GameEndEager::Repetition(max) => n_fold_repetition(max, history, pos.hash_pos(), usize::MAX),
            GameEndEager::InsufficientMaterial(vec) => {
                for &(piece, count) in vec {
                    if pos.piece_bb(piece).num_ones() > count {
                        return false;
                    }
                }
                true
            }
        }
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Arbitrary)]
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
            in_hand: [u8::arbitrary(u)?; MAX_NUM_PIECE_TYPES],
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
            game_result: None,
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
pub enum NumRoyals {
    Exactly(usize),
    AtLeast(usize),
}

/// This struct defined the rules for the game.
/// Since the rules don't change during a game, but are expensive to copy and the board uses copy-make,
/// they are created once and stored globally.
#[must_use]
#[derive(Debug, Arbitrary)]
pub(super) struct Rules {
    pub pieces: Vec<Piece>,
    pub colors: [ColorInfo; NUM_COLORS],
    pub starting_pieces_in_hand: [u8; MAX_NUM_PIECE_TYPES],
    pub game_end_eager: Vec<(GameEndEager, GameEndResult)>,
    pub game_end_no_moves: Vec<(NoMovesCondition, GameEndResult)>,
    pub startpos_fen: String,
    pub empty_board: EmptyBoard,
    // pub legality: Legality,
    pub size: GridSize,
    pub has_ep: bool,
    pub has_castling: bool,
    pub store_last_move: bool,
    pub effect_rules: EffectRules,
    pub check_rules: CheckRules,
    pub name: String,
    pub fen_part: RulesFenPart,
    pub num_royals: NumRoyals,
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

    pub fn pieces(&self) -> impl Iterator<Item = (PieceId, &Piece)> {
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
            game_result: None,
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
            (DrawCounter(100), DrawUnlessLossOn(InCheck)),
            (Repetition(3), Draw),
            (InsufficientMaterial(knight_draw), Draw),
            (InsufficientMaterial(bishop_draw), Draw),
        ];
        let game_end_no_moves = vec![(NotInCheck, Draw), (InCheck, InactivePlayerWin)];
        let startpos_fen = chess::START_FEN.to_string();
        // let legality = PseudoLegal;
        let effect_rules = EffectRules::default();
        let empty_func = Self::generic_empty_board;
        Self {
            pieces,
            colors,
            starting_pieces_in_hand: [0; MAX_NUM_PIECE_TYPES],
            game_end_eager,
            game_end_no_moves,
            startpos_fen,
            // legality,
            empty_board: EmptyBoard(Box::new(empty_func)),
            size: FairySize::chess(),
            has_ep: true,
            has_castling: true,
            store_last_move: false,
            effect_rules,
            check_rules: CheckRules::AnyRoyal,
            name: "chess".to_string(),
            fen_part: RulesFenPart::None,
            num_royals: Exactly(1),
            observers: Observers::chess(),
        }
    }

    pub fn shatranj() -> Self {
        let pieces = Piece::shatranj_pieces();
        let colors = Self::chess_colors();
        let game_end_eager =
            vec![(DrawCounter(100), Draw), (Repetition(3), Draw), (NoNonRoyalsExceptRecapture, GameEndResult::lose())];
        let game_end_no_moves = vec![(Always, GameEndResult::lose())];
        // let game_loss = vec![GameEndEager::Checkmate, GameEndEager::NoMoves, GameEndEager::NoNonRoyalsExceptRecapture];
        // let draw = vec![GameEndEager::Counter(100), GameEndEager::Repetition(3)];
        let startpos_fen = "shatranj rnakfanr/pppppppp/8/8/8/8/PPPPPPPP/RNAKFANR w 0 1".to_string();
        // let legality = PseudoLegal;
        let effect_rules = EffectRules::default();
        Self {
            pieces,
            colors,
            starting_pieces_in_hand: [0; MAX_NUM_PIECE_TYPES],
            game_end_eager,
            game_end_no_moves,
            startpos_fen,
            // legality,
            empty_board: EmptyBoard(Box::new(Self::generic_empty_board)),
            size: FairySize::chess(),
            has_ep: false,
            has_castling: false,
            store_last_move: false,
            effect_rules,
            check_rules: CheckRules::AnyRoyal,
            name: "shatranj".to_string(),
            fen_part: RulesFenPart::None,
            num_royals: Exactly(1),
            observers: Observers::shatranj(),
        }
    }

    pub fn ataxx() -> Self {
        let size = FairySize::ataxx();
        let mut map = Piece::complete_piece_map(size);
        let piece = map.remove("ataxx").unwrap();
        let gap = map.remove("gap").unwrap();
        let pieces = vec![piece, gap];
        let startpos_fen = "ataxx ".to_string() + &AtaxxBoard::startpos().as_fen();
        Self {
            pieces,
            colors: Self::mnk_colors(),
            starting_pieces_in_hand: [u8::MAX; MAX_NUM_PIECE_TYPES],
            game_end_eager: vec![(Repetition(3), Draw), (DrawCounter(100), Draw), (NoPieces, GameEndResult::lose())],
            game_end_no_moves: vec![(NoMovesCondition::NoOpponentMoves, GameEndResult::MorePieces)],
            startpos_fen,
            // legality: Legality::Legal,
            empty_board: EmptyBoard(Box::new(Self::generic_empty_board)),
            size,
            has_ep: false,
            has_castling: false,
            store_last_move: false,
            effect_rules: EffectRules { reset_draw_counter_on_capture: true, conversion_radius: 1 },
            check_rules: CheckRules::default(),
            name: "ataxx".to_string(),
            fen_part: RulesFenPart::None,
            num_royals: Exactly(0),
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
        let startpos_fen = "mnk ".to_string() + &MNKBoard::startpos_for_settings(settings).as_fen();
        Self {
            pieces,
            colors: Self::mnk_colors(),
            starting_pieces_in_hand: [u8::MAX; MAX_NUM_PIECE_TYPES],
            // lose because the side to move switches after a move, so `InRowAtLeast` checks the last move
            game_end_eager: vec![(InRowAtLeast(k as usize), GameEndResult::lose())],
            game_end_no_moves: vec![(Always, Draw)],
            startpos_fen,
            // legality: Legality::Legal,
            empty_board: EmptyBoard(Box::new(Self::generic_empty_board)),
            size,
            has_ep: false,
            has_castling: false,
            store_last_move: true,
            effect_rules: EffectRules::default(),
            check_rules: CheckRules::default(),
            name: "mnk".to_string(),
            fen_part: RulesFenPart::Mnk(settings),
            num_royals: Exactly(0),
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

impl Debug for RulesRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "rules ref")
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
        std::ptr::hash(self.0.deref(), state);
    }
}

impl Settings for RulesRef {
    fn text(&self) -> Option<String> {
        Some(format!("Variant: {}", self.0.name))
    }
}

static DEFAULT_FAIRY_RULES: LazyLock<Arc<Rules>> = LazyLock::new(|| Arc::new(Rules::chess()));
