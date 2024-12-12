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
use crate::games::chess::castling::{CastleRight, CastlingFlags};
use crate::games::chess::pieces::ChessPieceType::*;
use crate::games::chess::pieces::{ChessPiece, ColoredChessPieceType};
use crate::games::chess::squares::{ChessSquare, ChessboardSize};
use crate::games::chess::{ChessColor, Chessboard};
use crate::games::{Color, ColoredPiece, ColoredPieceType};
use crate::general::bitboards::chess::ChessBitboard;
use crate::general::bitboards::{Bitboard, RawBitboard};
use crate::general::board::SelfChecks::CheckFen;
use crate::general::board::Strictness::Strict;
use crate::general::board::{Board, SelfChecks, Strictness, UnverifiedBoard};
use crate::general::common::Res;
use crate::general::squares::RectangularCoordinates;
use anyhow::bail;
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
        for color in ChessColor::iter() {
            if !this.colored_piece_bb(color, King).is_single_piece() {
                bail!("The {color} player does not have exactly one king")
            }
            if (this.colored_piece_bb(color, Pawn)
                & (ChessBitboard::rank_no(0) | ChessBitboard::rank_no(7)))
            .has_set_bit()
            {
                bail!("The {color} player has a pawn on the first or eight rank");
            }
        }

        for color in ChessColor::iter() {
            for side in CastleRight::iter() {
                let has_eligible_rook = (this.rook_start_square(color, side).bb()
                    & this.colored_piece_bb(color, Rook))
                .has_set_bit();
                if this.castling.can_castle(color, side) && !has_eligible_rook {
                    bail!(
                        "Color {color} can castle {side}, but there is no rook to castle{}",
                        if checks == CheckFen {
                            " (invalid castling flag in FEN?)"
                        } else {
                            ""
                        }
                    );
                }
            }
        }
        let inactive_player = this.active_player.other();

        if let Some(ep_square) = this.ep_square {
            if ![2, 5].contains(&ep_square.rank()) {
                bail!(
                    "FEN specifies invalid ep square (not on the third or sixth rank): '{ep_square}'"
                );
            }
            let remove_pawn_square = ep_square.pawn_advance_unchecked(inactive_player);
            let pawn_origin_square = ep_square.pawn_advance_unchecked(this.active_player);
            if this.colored_piece_on(remove_pawn_square).symbol
                != ColoredChessPieceType::new(inactive_player, Pawn)
            {
                bail!("FEN specifies en passant square {ep_square}, but there is no {inactive_player}-colored pawn on {remove_pawn_square}");
            } else if !this.is_empty(ep_square) {
                bail!(
                    "The en passant square ({ep_square}) must be empty, but it's occupied by a {}",
                    this.piece_type_on(ep_square).name()
                )
            } else if !this.is_empty(pawn_origin_square) {
                bail!("The en passant square is set to {ep_square}, so the pawn must have come from {pawn_origin_square}. But this square isn't empty")
            }
            let active = this.active_player();
            // In the current version of the FEN standard, the ep square should only be set if a pawn can capture.
            // This implementation follows that rule, but many other implementations give the ep square after every double pawn push.
            // To achieve consistent results, such an incorrect ep square is removed when parsing the FEN in Relaxed mode; it should
            // no longer exist at this point.
            if checks != CheckFen || strictness == Strict {
                let possible_ep_pawns =
                    remove_pawn_square.bb().west() | remove_pawn_square.bb().east();
                if (possible_ep_pawns & this.colored_piece_bb(active, Pawn)).is_zero() {
                    bail!("The en passant square is set to '{ep_square}', but there is no {active}-colored pawn that could capture on that square");
                }
            }
        }

        if this.is_in_check_on_square(inactive_player, this.king_square(inactive_player)) {
            bail!("Player {inactive_player} is in check, but it's not their turn to move");
        } else if strictness == Strict {
            let checkers = this.all_attacking(this.king_square(this.active_player))
                & this.inactive_player_bb();
            let num_attacking = checkers.num_ones();
            if num_attacking > 2 {
                bail!(
                    "{} is in check from {num_attacking} pieces, which is not allowed in strict mode",
                    this.active_player
                )
            }
        }
        // we allow loading FENs where more than one piece gives check to the king in a way that could not have been reached
        // from startpos, e.g. "B6b/8/8/8/2K5/5k2/8/b6B b - - 0 1"
        if this.ply_100_ctr >= 100 {
            bail!(
                "The 50 move rule has been exceeded (there have already been {0} plies played)",
                this.ply_100_ctr
            );
        } else if this.ply >= 100_000 {
            bail!("Ridiculously large ply counter: {0}", this.ply);
        } else if strictness == Strict && this.ply_100_ctr > this.ply {
            bail!("The halfmove repetition clock ({0}) is larger than the number of played half moves ({1}), \
                which is not allowed in strict mode", this.ply_100_ctr, this.ply)
        }

        let mut num_promoted_pawns: [isize; 2] = [0, 0];
        let startpos_piece_count = [8, 2, 2, 2, 1, 1];
        for piece in ColoredChessPieceType::pieces() {
            let color = piece.color().unwrap();
            let bb = this.colored_piece_bb(color, piece.uncolor());
            if bb.num_ones() > 20 {
                // Catch this now to prevent crashes down the line because the move list is too small for made-up invalid positions.
                // (This is lax enough to allow many invalid positions that likely won't lead to a crash)
                bail!(
                    "There are {0} {color} {piece}s in this position. There can never be more than 10 pieces \
                    of the same type in a legal chess position (but this implementation accepts up to 20 in non-strict mode)",
                    bb.num_ones()
                );
            } else if strictness == Strict {
                num_promoted_pawns[color as usize] +=
                    0.max(bb.num_ones() as isize - startpos_piece_count[piece.uncolor() as usize]);
            }
            if checks != CheckFen {
                for other_piece in ColoredChessPieceType::pieces() {
                    if other_piece == piece {
                        continue;
                    }
                    if (bb
                        & this
                            .colored_piece_bb(other_piece.color().unwrap(), other_piece.uncolor()))
                    .has_set_bit()
                    {
                        bail!("There are two pieces on the same square: {piece} and {other_piece}");
                    }
                }
            }
        }
        for color in ChessColor::iter() {
            let num_pawns = this.colored_piece_bb(color, Pawn).num_ones() as isize;
            if strictness == Strict && num_promoted_pawns[color as usize] + num_pawns > 8 {
                bail!("Incorrect piece distribution for {color}")
            }
        }
        this.hash = this.compute_zobrist();
        Ok(this)
    }

    fn size(&self) -> ChessboardSize {
        self.0.size()
    }

    fn place_piece_unchecked(self, square: ChessSquare, piece: ColoredChessPieceType) -> Self {
        let mut this = self.0;
        debug_assert!(self.0.is_empty(square));
        let bb = square.bb().raw();
        this.piece_bbs[piece.uncolor() as usize] ^= bb;
        this.color_bbs[piece.color().unwrap() as usize] ^= bb;
        this.into()
    }

    fn remove_piece_unchecked(mut self, sq: ChessSquare) -> Self {
        let piece = self.0.colored_piece_on(sq);
        self.0
            .remove_piece_unchecked(sq, piece.symbol.uncolor(), piece.color().unwrap());
        self
    }

    fn piece_on(&self, coords: ChessSquare) -> Res<ChessPiece> {
        Ok(self.0.colored_piece_on(self.check_coordinates(coords)?))
    }

    fn set_active_player(mut self, player: ChessColor) -> Self {
        self.0.active_player = player;
        self
    }

    fn set_ply_since_start(mut self, ply: usize) -> Res<Self> {
        self.0.ply = ply;
        Ok(self)
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

    pub fn set_halfmove_repetition_clock(mut self, ply: usize) -> Self {
        self.0.ply_100_ctr = ply;
        self
    }
}
