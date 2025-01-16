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
use crate::games::chess::pieces::NUM_COLORS;
use crate::games::fairy::attacks::EffectRules;
use crate::games::fairy::moves::FairyMove;
use crate::games::fairy::pieces::{Piece, PieceId};
use crate::games::fairy::rules::GameLoss::InRowAtLeast;
use crate::games::fairy::{
    ColorInfo, FairyBitboard, FairyCastleInfo, FairyColor, FairySize, RawFairyBitboard, UnverifiedFairyBoard,
    MAX_NUM_PIECE_TYPES,
};
use crate::games::mnk::{MNKBoard, MnkSettings};
use crate::games::{chess, DimT, Settings};
use crate::general::bitboards::Bitboard;
use crate::general::board::{Board, BoardHelpers};
use crate::general::common::{Res, Tokens};
use crate::general::squares::GridSize;
use arbitrary::Arbitrary;
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
#[allow(dead_code)]
pub enum GameLoss {
    Checkmate,
    NoRoyals,
    NoPieces,
    NoNonRoyals,
    NoNonRoyalsExceptRecapture,
    NoMoves,
    InRowAtLeast(usize),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Arbitrary)]
#[must_use]
pub enum Draw {
    NoMoves,
    Counter(usize),
    Repetition(usize),
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
        };
        let func = move |rules: &RulesRef| {
            let mut b = board.clone();
            b.rules = rules.clone();
            b
        };
        Ok(EmptyBoard(Box::new(func)))
    }
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
    pub game_loss: Vec<GameLoss>,
    pub draw: Vec<Draw>,
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
    pub fn matching_piece_ids<Pred: Fn(&Piece) -> bool + Copy>(
        &self,
        pred: Pred,
    ) -> impl Iterator<Item = PieceId> + use<'_, Pred> {
        self.pieces().filter(move |(_id, p)| pred(p)).map(|(id, _)| id)
    }
    pub fn royals(&self) -> impl Iterator<Item = PieceId> + use<'_> {
        self.matching_piece_ids(|p| p.royal)
    }

    pub fn castling(&self) -> impl Iterator<Item = PieceId> + use<'_> {
        self.matching_piece_ids(|p| p.can_castle)
    }

    pub fn has_halfmove_repetition_clock(&self) -> bool {
        self.draw.iter().any(|d| matches!(d, &Draw::Repetition(_)))
    }

    fn generic_empty_board(rules: &RulesRef) -> UnverifiedFairyBoard {
        let size = rules.0.size;
        UnverifiedFairyBoard {
            piece_bitboards: Default::default(),
            color_bitboards: Default::default(),
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
        let game_loss = vec![GameLoss::Checkmate];
        let draw = vec![Draw::NoMoves, Draw::Counter(100), Draw::Repetition(3)];
        let startpos_fen = chess::START_FEN.to_string();
        // let legality = PseudoLegal;
        let effect_rules = EffectRules::default();
        let empty_func = Self::generic_empty_board;
        Self {
            pieces,
            colors,
            starting_pieces_in_hand: [0; MAX_NUM_PIECE_TYPES],
            game_loss,
            draw,
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
        }
    }

    pub fn shatranj() -> Self {
        let pieces = Piece::shatranj_pieces();
        let colors = Self::chess_colors();
        let game_loss = vec![GameLoss::Checkmate, GameLoss::NoMoves, GameLoss::NoNonRoyalsExceptRecapture];
        let draw = vec![Draw::NoMoves, Draw::Counter(100), Draw::Repetition(3)];
        let startpos_fen = "shatranj rnakfanr/pppppppp/8/8/8/8/PPPPPPPP/RNAKFANR w 0 1".to_string();
        // let legality = PseudoLegal;
        let effect_rules = EffectRules::default();
        Self {
            pieces,
            colors,
            starting_pieces_in_hand: [0; MAX_NUM_PIECE_TYPES],
            game_loss,
            draw,
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
        }
    }

    pub fn tictactoe() -> Self {
        Self::mnk(FairySize::tictactoe(), 3)
    }

    pub fn mnk(size: FairySize, k: DimT) -> Self {
        let piece = Piece::complete_piece_map(size).remove("mnk").unwrap();
        let mut pieces = Vec::new();
        pieces.push(piece);
        let settings = MnkSettings::new(size.height, size.width, k);
        let startpos_fen = "mnk ".to_string() + &MNKBoard::startpos_for_settings(settings).as_fen();
        Self {
            pieces,
            colors: Self::mnk_colors(),
            starting_pieces_in_hand: [u8::MAX; MAX_NUM_PIECE_TYPES],
            game_loss: vec![InRowAtLeast(k as usize)],
            draw: vec![Draw::NoMoves],
            startpos_fen,
            // legality: Legality::Legal,
            empty_board: EmptyBoard(Box::new(Self::generic_empty_board)),
            size,
            has_ep: false,
            has_castling: false,
            store_last_move: false,
            effect_rules: EffectRules::default(),
            check_rules: CheckRules::default(),
            name: "mnk".to_string(),
            fen_part: RulesFenPart::Mnk(settings),
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
