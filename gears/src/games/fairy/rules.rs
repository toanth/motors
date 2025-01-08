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
use crate::games::fairy::pieces::{Piece, PieceId};
use crate::games::fairy::rules::GameLoss::InRowAtLeast;
use crate::games::fairy::{ColorInfo, FairySize, MAX_NUM_PIECE_TYPES};
use crate::games::mnk::{MNKBoard, MnkSettings};
use crate::games::{chess, DimT};
use crate::general::board::{Board, BoardHelpers};
use crate::general::common::{Res, Tokens};
use crate::general::moves::Legality;
use crate::general::moves::Legality::PseudoLegal;
use crate::general::squares::GridSize;
use arrayvec::ArrayVec;
use std::cell::{Ref, RefCell, RefMut};
use std::fmt;
use std::fmt::Formatter;
use thread_local::ThreadLocal;

/// Whether any or all royal pieces have to be attacked for the player to be considered in check
#[derive(Debug, Default, Copy, Clone)]
pub enum CheckRules {
    #[default]
    AnyRoyal,
    #[allow(dead_code)] // TODO: Variant with multiple royal pieces
    AllRoyals,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[must_use]
pub enum Draw {
    NoMoves,
    Counter(usize),
    Repetition(usize),
}

#[derive(Debug, Default, Copy, Clone)]
pub enum RulesFenPart {
    #[default]
    None,
    Mnk(MnkSettings),
}

/// This struct defined the rules for the game.
/// Since the rules don't change during a game, but are expensive to copy and the board uses copy-make,
/// they are created once and stored globally.
#[must_use]
pub(super) struct Rules {
    pub pieces: ArrayVec<Piece, MAX_NUM_PIECE_TYPES>,
    pub colors: [ColorInfo; NUM_COLORS],
    pub starting_pieces_in_hand: [u8; MAX_NUM_PIECE_TYPES],
    pub game_loss: Vec<GameLoss>,
    pub draw: Vec<Draw>,
    pub startpos_fen: String,
    pub legality: Legality,
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
        match rules().fen_part {
            RulesFenPart::None => Ok(()),
            RulesFenPart::Mnk(settings) => {
                write!(f, "{settings} ")
            }
        }
    }

    pub(super) fn read_rules_fen_part(input: &mut Tokens) -> Res<()> {
        let fen_part = rules().fen_part;
        match fen_part {
            RulesFenPart::None => Ok(()),
            RulesFenPart::Mnk(_) => {
                let first = input.next().unwrap_or_default();
                let settings = MnkSettings::from_input(first, input)?;
                set_rules(Rules::mnk(settings.size(), settings.k() as DimT));
                Ok(())
            }
        }
    }

    pub fn pieces(&self) -> impl Iterator<Item = (PieceId, &Piece)> {
        self.pieces
            .iter()
            .enumerate()
            .map(|(id, piece)| (PieceId::new(id), piece))
    }
    pub fn matching_piece_ids<Pred: Fn(&Piece) -> bool + Copy>(
        &self,
        pred: Pred,
    ) -> impl Iterator<Item = PieceId> + use<'_, Pred> {
        self.pieces()
            .filter(move |(_id, p)| pred(p))
            .map(|(id, _)| id)
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

    // Used for mnk games and many other variants
    fn mnk_colors() -> [ColorInfo; NUM_COLORS] {
        [
            ColorInfo {
                ascii_char: 'x',
                name: "X".to_string(),
            },
            ColorInfo {
                ascii_char: 'o',
                name: "O".to_string(),
            },
        ]
    }

    // Used for chess and many other variants
    fn chess_colors() -> [ColorInfo; NUM_COLORS] {
        [
            ColorInfo {
                ascii_char: 'w',
                name: "white".to_string(),
            },
            ColorInfo {
                ascii_char: 'b',
                name: "black".to_string(),
            },
        ]
    }

    pub fn chess() -> Self {
        let pieces = Piece::chess_pieces();
        let colors = Self::chess_colors();
        let game_loss = vec![GameLoss::Checkmate];
        let draw = vec![Draw::NoMoves, Draw::Counter(100), Draw::Repetition(3)];
        let startpos_fen = chess::START_FEN.to_string();
        let legality = PseudoLegal;
        let effect_rules = EffectRules::default();
        Self {
            pieces,
            colors,
            starting_pieces_in_hand: [0; MAX_NUM_PIECE_TYPES],
            game_loss,
            draw,
            startpos_fen,
            legality,
            size: FairySize::chess(),
            has_ep: true,
            has_castling: true,
            store_last_move: false,
            effect_rules,
            check_rules: CheckRules::AnyRoyal,
            name: "Chess".to_string(),
            fen_part: RulesFenPart::None,
        }
    }

    pub fn shatranj() -> Self {
        let pieces = Piece::shatranj_pieces();
        let colors = Self::chess_colors();
        let game_loss = vec![
            GameLoss::Checkmate,
            GameLoss::NoMoves,
            GameLoss::NoNonRoyalsExceptRecapture,
        ];
        let draw = vec![Draw::NoMoves, Draw::Counter(100), Draw::Repetition(3)];
        let startpos_fen = "rnakfanr/pppppppp/8/8/8/8/PPPPPPPP/RNAKFANR w 0 1".to_string();
        let legality = PseudoLegal;
        let effect_rules = EffectRules::default();

        Self {
            pieces,
            colors,
            starting_pieces_in_hand: [0; MAX_NUM_PIECE_TYPES],
            game_loss,
            draw,
            startpos_fen,
            legality,
            size: FairySize::chess(),
            has_ep: false,
            has_castling: false,
            store_last_move: false,
            effect_rules,
            check_rules: CheckRules::AnyRoyal,
            name: "Chess".to_string(),
            fen_part: RulesFenPart::None,
        }
    }

    pub fn tictactoe() -> Self {
        Self::mnk(FairySize::tictactoe(), 3)
    }

    pub fn mnk(size: FairySize, k: DimT) -> Self {
        let piece = Piece::complete_piece_map(size).remove("mnk").unwrap();
        let mut pieces = ArrayVec::new();
        pieces.push(piece);
        let settings = MnkSettings::new(size.height, size.width, k);
        let startpos_fen = MNKBoard::startpos_for_settings(settings).as_fen();
        Self {
            pieces,
            colors: Self::mnk_colors(),
            starting_pieces_in_hand: [u8::MAX; MAX_NUM_PIECE_TYPES],
            game_loss: vec![InRowAtLeast(k as usize)],
            draw: vec![Draw::NoMoves],
            startpos_fen,
            legality: Legality::Legal,
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

pub fn set_rules(rules: Rules) {
    *rules_mut() = rules;
}

/// The least bad option to implement rules.
/// In the future, it might make sense to explore an implementation where each piece, move, etc, contains
/// a reference / Rc to the rules.
/// Also, a lot of this should go into a position struct, which wraps a board and rules and isn't copy.
static FAIRY_RULES: ThreadLocal<RefCell<Rules>> = ThreadLocal::new();

// this function is a lot slower than just reading a variable, but speed isn't the largest concern for fairy chess anyway.
// TODO: Still, it might be worth to test if caching the rules improves elo. The major drawback would be the possibility of panics
// if a cached entry still exists when the rules are getting changed
pub(super) fn rules() -> Ref<'static, Rules> {
    FAIRY_RULES.get_or(|| RefCell::new(Rules::chess())).borrow()
}

pub(super) fn rules_mut() -> RefMut<'static, Rules> {
    FAIRY_RULES
        .get_or(|| RefCell::new(Rules::chess()))
        .borrow_mut()
}
