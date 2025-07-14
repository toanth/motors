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
use crate::games::chess::ChessColor::{Black, White};
use crate::games::chess::castling::{CastleRight, CastlingFlags};
use crate::games::chess::pieces::ChessPieceType::{King, Pawn, Rook};
use crate::games::chess::pieces::ColoredChessPieceType::{BlackKing, WhiteKing};
use crate::games::chess::pieces::{ChessPiece, ChessPieceType, ColoredChessPieceType};
use crate::games::chess::squares::{ChessSquare, ChessboardSize};
use crate::games::chess::{ChessColor, ChessSettings, Chessboard};
use crate::games::{Color, ColoredPiece, ColoredPieceType, Coordinates};
use crate::general::bitboards::chessboard::ChessBitboard;
use crate::general::bitboards::{Bitboard, KnownSizeBitboard, RawBitboard};
use crate::general::board::SelfChecks::{Assertion, CheckFen};
use crate::general::board::Strictness::Strict;
use crate::general::board::{BitboardBoard, Board, BoardHelpers, SelfChecks, Strictness, Symmetry, UnverifiedBoard};
use crate::general::common::{Res, ith_one_u64};
use crate::general::squares::RectangularCoordinates;
use anyhow::{bail, ensure};
use rand::Rng;
use std::ops::Not;
use strum::IntoEnumIterator;

#[derive(Debug, Copy, Clone)]
#[must_use]
pub struct UnverifiedChessboard(pub(super) Chessboard);

impl From<Chessboard> for UnverifiedChessboard {
    fn from(board: Chessboard) -> Self {
        Self(board)
    }
}

impl UnverifiedBoard<Chessboard> for UnverifiedChessboard {
    fn verify_with_level(self, checks: SelfChecks, strictness: Strictness) -> Res<Chessboard> {
        let mut this = self.0;
        if checks == Assertion {
            ensure!(
                (this.player_bb(White) & this.player_bb(Black)).is_zero(),
                "A square is set both on the white and black player bitboard, but no piece bitboard has this bit set"
            );
            let mut pieces = ChessBitboard::default();
            for piece in ChessPieceType::pieces() {
                pieces |= this.piece_bb(piece);
            }
            if pieces != this.color_bbs[0] | this.color_bbs[1] {
                bail!(
                    "The colored bitboards and the piece bitboards don't match on the following squares: {}",
                    pieces ^ (this.color_bbs[0] | this.color_bbs[1])
                );
            }
        }
        for color in ChessColor::iter() {
            ensure!(
                this.col_piece_bb(color, King).is_single_piece(),
                "The {color} player does not have exactly one king"
            );
            ensure!(
                (this.col_piece_bb(color, Pawn) & (ChessBitboard::rank_0() | ChessBitboard::rank(7))).is_zero(),
                "The {color} player has a pawn on the first or eight rank"
            );
        }

        for color in ChessColor::iter() {
            for side in CastleRight::iter() {
                let has_eligible_rook =
                    (this.rook_start_square(color, side).bb() & this.col_piece_bb(color, Rook)).has_set_bit();
                if this.castling.can_castle(color, side) && !has_eligible_rook {
                    bail!(
                        "The {color} player can castle {side}, but there is no rook to castle with{}",
                        if checks == CheckFen { " (invalid castling flag in FEN?)" } else { "" }
                    );
                }
            }
        }
        let inactive_player = this.active.other();

        let generator = self.0.slider_generator();
        if this.is_in_check_on_square(inactive_player, this.king_square(inactive_player), &generator) {
            bail!("Player {inactive_player} is in check, but it's not their turn to move");
        } else if strictness == Strict {
            let checkers = this.all_attacking(this.king_square(this.active), &generator) & this.inactive_player_bb();
            let num_attacking = checkers.num_ones();
            ensure!(
                num_attacking <= 2,
                "{} is in check from {num_attacking} pieces, which is not allowed in strict mode",
                this.active
            );
        }
        // we allow loading FENs where more than one piece gives check to the king in a way that could not have been reached
        // from startpos, e.g. "B6b/8/8/8/2K5/5k2/8/b6B b - - 0 1"
        if this.ply_100_ctr > 100 {
            bail!("The 50 move rule has been exceeded (there have already been {0} plies played)", this.ply_100_ctr);
        } else if this.ply >= 20_000 {
            bail!("Ridiculously large ply counter: {0}", this.ply);
        } else if strictness == Strict && this.ply_draw_clock() > this.halfmove_ctr_since_start() {
            bail!(
                "The halfmove repetition clock ({0}) is larger than the number of played half moves ({1}), \
                which is not allowed in strict mode",
                this.ply_100_ctr,
                this.ply
            )
        }

        let mut num_promoted_pawns: [isize; 2] = [0, 0];
        let startpos_piece_count = [8, 2, 2, 2, 1, 1];
        for piece in ColoredChessPieceType::pieces() {
            let color = piece.color().unwrap();
            let bb = this.col_piece_bb(color, piece.uncolor());
            if strictness == Strict {
                num_promoted_pawns[color] += 0.max(bb.num_ones() as isize - startpos_piece_count[piece.uncolor()]);
                // Print a better error message than the generic "invalid piece distribution".
                ensure!(
                    bb.num_ones() <= 10,
                    "There are {0} {color} {piece}s in this position. There can never be more than 10 pieces \
                    of the same type in a legal chess position (in relaxed mode, this is accepted anyway)",
                    bb.num_ones()
                );
            }
            if checks != CheckFen {
                for other_piece in ColoredChessPieceType::pieces() {
                    if other_piece == piece {
                        continue;
                    }
                    ensure!(
                        (bb & this.col_piece_bb(other_piece.color().unwrap(), other_piece.uncolor())).is_zero(),
                        "There are two pieces on the same square: {piece} and {other_piece}"
                    );
                }
            }
        }
        for color in ChessColor::iter() {
            let num_pawns = this.col_piece_bb(color, Pawn).num_ones() as isize;
            if strictness == Strict && num_promoted_pawns[color] + num_pawns > 8 {
                bail!("Incorrect piece distribution for {color} (in relaxed mode, this is allowed)")
            }
        }
        this.hashes = this.compute_zobrist();

        // We check the ep square last because this can require doing movegen, which needs most invariants to hold.
        if let Some(ep_square) = this.ep_square {
            ensure!(
                [2, 5].contains(&ep_square.rank()),
                "FEN specifies invalid ep square (not on the third or sixth rank): '{ep_square}'"
            );
            let remove_pawn_square = ep_square.pawn_advance_unchecked(inactive_player);
            let pawn_origin_square = ep_square.pawn_advance_unchecked(this.active);
            if !this.is_empty(ep_square) {
                bail!(
                    "The en passant square ({ep_square}) must be empty, but it's occupied by a {}",
                    this.piece_type_on(ep_square).to_name()
                )
            } else if this.colored_piece_on(remove_pawn_square).symbol
                != ColoredChessPieceType::new(inactive_player, Pawn)
            {
                bail!(
                    "FEN specifies en passant square {ep_square}, but there is no {inactive_player}-colored pawn on {remove_pawn_square}"
                );
            } else if !this.is_empty(pawn_origin_square) {
                bail!(
                    "The en passant square is set to {ep_square}, so the pawn must have come from {pawn_origin_square}. But this square isn't empty"
                )
            }
            let active = this.active_player();
            // In the current version of the FEN standard, the ep square should only be set if a pawn can capture.
            // This implementation follows that rule, but many other implementations give the ep square after every double pawn push.
            // To achieve consistent results, such an incorrect ep square is removed when parsing the FEN; it should
            // no longer exist at this point.
            let possible_ep_pawns = remove_pawn_square.bb().west() | remove_pawn_square.bb().east();
            ensure!(
                (possible_ep_pawns & this.col_piece_bb(active, Pawn)).has_set_bit(),
                "The en passant square is set to '{ep_square}', but there is no {active}-colored pawn that could capture on that square"
            );
            if strictness == Strict && checks == CheckFen {
                let legal_ep = this.legal_moves_slow().iter().any(|m| m.is_ep());
                // this doesn't necessarily mean that the ep pawn capturing is pinned, the king could also be in check.
                ensure!(
                    legal_ep,
                    "The en passant square is set, but even though there is a pseudolegal ep capture move, it is not legal \
                    (either all pawns that could capture en passant are pinned, or the king is in check). \
                    This is not allowed when parsing FENs in strict mode"
                );
            }
        }
        this.set_checkers_and_pinned();
        this.threats = this.calc_threats_of(this.inactive_player(), &this.slider_generator());
        Ok(this)
    }

    fn settings(&self) -> &ChessSettings {
        self.0.settings()
    }

    fn size(&self) -> ChessboardSize {
        self.0.size()
    }

    fn place_piece(&mut self, square: ChessSquare, piece: ColoredChessPieceType) {
        let this = &mut self.0;
        debug_assert!(this.is_empty(square));
        let bb = square.bb();
        this.piece_bbs[piece.uncolor()] ^= bb;
        this.color_bbs[piece.color().unwrap()] ^= bb;
    }

    fn remove_piece(&mut self, sq: ChessSquare) {
        let piece = self.0.colored_piece_on(sq);
        self.0.remove_piece_unchecked(sq, piece.symbol.uncolor(), piece.color().unwrap());
    }

    fn piece_on(&self, coords: ChessSquare) -> ChessPiece {
        self.0.colored_piece_on(coords)
    }

    fn is_empty(&self, square: ChessSquare) -> bool {
        self.0.is_empty(square)
    }

    fn active_player(&self) -> ChessColor {
        self.0.active
    }

    fn set_active_player(&mut self, player: ChessColor) {
        self.0.active = player;
    }

    fn set_ply_since_start(&mut self, ply: usize) -> Res<()> {
        self.0.ply = u32::try_from(ply)?;
        Ok(())
    }

    fn set_halfmove_repetition_clock(&mut self, ply: usize) -> Res<()> {
        self.0.ply_100_ctr = u8::try_from(ply)?;
        Ok(())
    }
}

impl UnverifiedChessboard {
    pub fn castling_rights_mut(&mut self) -> &mut CastlingFlags {
        &mut self.0.castling
    }

    pub fn set_ep(mut self, ep: Option<ChessSquare>) -> Self {
        self.0.ep_square = ep;
        self
    }

    pub fn random_unverified_pos(rng: &mut impl Rng, strictness: Strictness, symmetry: Option<Symmetry>) -> Self {
        let mut pos = Chessboard::empty();
        let mask = if let Some(symmetry) = symmetry {
            match symmetry {
                Symmetry::Material => ChessBitboard::default().not(),
                Symmetry::Horizontal => ChessBitboard::new(0xf0f0_f0f0_f0f0_f0f0),
                Symmetry::Vertical => ChessBitboard::new(0xffff_ffff),
                Symmetry::Rotation180 => ChessBitboard::new(0xffff_ffff),
            }
        } else {
            ChessBitboard::default().not()
        };
        let king_sq1 = rng.random_range(0..mask.num_ones());
        let king_sq1 = ChessSquare::from_bb_idx(king_sq1);
        pos.place_piece(king_sq1, WhiteKing);
        let king_sq2 = if let Some(symmetry) = symmetry {
            mirror_sq(king_sq1, symmetry, rng, &pos.0)
        } else {
            loop {
                let king_sq2 = rng.random_range(0..64);
                let king_sq2 = ChessSquare::from_bb_idx(king_sq2);
                if king_sq2 == king_sq1 || Chessboard::normal_king_attacks_from(king_sq2).is_bit_set(king_sq1) {
                    continue;
                }
                break king_sq2;
            }
        };
        pos.place_piece(king_sq2, BlackKing);

        // more pieces make it more likely that the resulting position isn't legal,
        // and we also care more about reachable positions. So we limit the number of pieces to 42 even in relaxed mode.
        let max_num_pieces = if strictness == Strict { 30 } else { 40 };
        let num_pieces = if symmetry.is_some() {
            rng.random_range(0..=(max_num_pieces / 2)) + 1
        } else {
            rng.random_range(0..=max_num_pieces) + 2
        };
        for _ in 0..num_pieces {
            let piece = if symmetry.is_some() {
                let piece = rng.random_range(0..5);
                ColoredChessPieceType::new(White, ChessPieceType::from_repr(piece).unwrap())
            } else {
                let piece = rng.random_range(0..10);
                let col = ChessColor::iter().nth(piece / 5).unwrap();
                let piece = ChessPieceType::from_repr(piece % 5).unwrap();
                ColoredChessPieceType::new(col, piece)
            };

            let empty = pos.0.empty_bb() & mask;
            let num_empty = empty.num_ones();
            loop {
                let sq_idx = rng.random_range(0..num_empty);
                let sq_idx = ith_one_u64(sq_idx, empty.raw());
                let sq = ChessSquare::from_bb_idx(sq_idx);
                if piece.uncolor() == Pawn && sq.is_backrank() {
                    continue;
                }
                pos.place_piece(sq, piece);
                if let Some(symmetry) = symmetry {
                    let sq = mirror_sq(sq, symmetry, rng, &pos.0);
                    let piece = ColoredChessPieceType::new(Black, piece.uncolor());
                    pos.place_piece(sq, piece);
                }
                break;
            }
        }
        // vertical and rotational symmetry keep the white pieces on the lower half of the board,
        // but this introduces a smallish chance to flip that
        if rng.random_bool(0.2) {
            pos.0.color_bbs.swap(0, 1)
        }
        if rng.random_bool(0.5) {
            pos.0.active = !pos.0.active;
        }
        // don't generate castling or ep flags for now
        pos
    }
}

fn mirror_sq(sq: ChessSquare, symmetry: Symmetry, rng: &mut impl Rng, pos: &Chessboard) -> ChessSquare {
    match symmetry {
        Symmetry::Material => {
            let empty = pos.empty_bb().raw();
            ChessSquare::from_bb_idx(ith_one_u64(rng.random_range(0..empty.num_ones()), empty))
        }
        Symmetry::Horizontal => sq.flip_left_right(ChessboardSize::default()),
        Symmetry::Vertical => sq.flip_up_down(ChessboardSize::default()),
        Symmetry::Rotation180 => sq.flip_left_right(ChessboardSize::default()).flip_up_down(ChessboardSize::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::general::board::Strictness::Relaxed;
    use proptest::proptest;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    proptest! {
        #[test]
        fn random_unverified(seed in 0..=u64::MAX, strictness in 0..2, symmetry in 0..=Symmetry::iter().count()) {
            let mut rng = StdRng::seed_from_u64(seed);
            let symmetry = Symmetry::iter().nth(symmetry);
            let strictness = if strictness == 0 { Strict } else { Relaxed };
            let res = UnverifiedChessboard::random_unverified_pos(&mut rng, strictness, symmetry);
            let ok = res.verify_with_level(SelfChecks::Verify, strictness);
            if ok.is_ok() {
                assert!(res.verify_with_level(Assertion, Relaxed).is_ok());
            }
        }
    }
}
