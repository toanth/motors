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
use crate::games::chess::Color::{Black, White};
use crate::games::chess::castling::{CastleRight, CastlingFlags};
use crate::games::chess::pieces::ColoredPieceType::{BlackKing, WhiteKing};
use crate::games::chess::pieces::PieceType::{Empty, King, Pawn, Rook};
use crate::games::chess::pieces::{ColoredPieceType, Piece, PieceType};
use crate::games::chess::squares::{ChessboardSize, Square};
use crate::games::chess::{Board, Color, Settings};
use crate::games::{ColorTrait, ColoredPieceTrait, ColoredPieceTypeTrait, CoordinatesTrait, DimT, PosHash};
use crate::general::attacks::ChessSliderGenerator;
use crate::general::bitboards::chessboard::Bitboard;
use crate::general::bitboards::{BitboardTrait, KnownSizeBitboard, RawBitboardTrait};
use crate::general::board::SelfChecks::{Assertion, CheckFen};
use crate::general::board::Strictness::Strict;
use crate::general::board::{
    BitboardBoard, BoardHelpers, BoardTrait, SelfChecks, Strictness, Symmetry, UnverifiedBoardTrait,
};
use crate::general::common::{Res, ith_one_u64};
use crate::general::squares::RectangularCoordinates;
use anyhow::{bail, ensure};
use rand::{Rng, RngExt};
use std::ops::Not;
use strum::IntoEnumIterator;

#[derive(Debug, Copy, Clone)]
#[must_use]
pub struct UnverifiedBoard(pub(super) Board);

impl From<Board> for UnverifiedBoard {
    fn from(board: Board) -> Self {
        Self(board)
    }
}

fn fen_selfchecks(this: &Board, checks: SelfChecks, strictness: Strictness) -> Res<()> {
    for color in Color::iter() {
        ensure!(this.col_piece_bb(color, King).is_single_piece(), "The {color} player does not have exactly one king");
        if this.col_piece_bb(color, Pawn).intersects(Bitboard::backranks()) {
            bail!("The {color} player has a pawn on the first or eight rank")
        }

        for side in CastleRight::iter() {
            let eligible_rook = this.col_piece_bb(color, Rook).has(this.rook_start_square(color, side));
            if this.castling.can_castle(color, side) && !eligible_rook {
                bail!(
                    "The {color} player can castle {side}, but there is no rook to castle with{}",
                    if checks == CheckFen { " (invalid castling flag in FEN?)" } else { "" }
                );
            }
        }
    }

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
    for color in Color::iter() {
        for piece in PieceType::pieces() {
            let bb = this.col_piece_bb(color, piece);
            if strictness == Strict {
                num_promoted_pawns[color] += 0.max(bb.num_ones() as isize - startpos_piece_count[piece]);
                // Print a better error message than the generic "invalid piece distribution".
                ensure!(
                    bb.num_ones() <= 10,
                    "There are {0} {color} {piece}s in this position. There can never be more than 10 pieces \
                            of the same type in a legal chess position (in relaxed mode, this is accepted anyway)",
                    bb.num_ones()
                );
            }
            if checks > CheckFen {
                for other_piece in ColoredPieceType::pieces() {
                    if other_piece as usize >= ColoredPieceType::new(color, piece) as usize {
                        break;
                    }
                    let mut overlap = bb & this.col_piece_bb(other_piece.color().unwrap(), other_piece.uncolor());
                    ensure!(
                        overlap.is_zero(),
                        "There are two pieces on the same square ({0}): A {other_piece} and a {piece}",
                        overlap.next().unwrap()
                    );
                }
            }
        }
        let num_pawns = this.col_piece_bb(color, Pawn).num_ones() as isize;
        if strictness == Strict && num_promoted_pawns[color] + num_pawns > 8 {
            bail!("Incorrect piece distribution for {color} (in relaxed mode, this is allowed)")
        }
    }
    Ok(())
}

impl UnverifiedBoardTrait<Board> for UnverifiedBoard {
    fn verify_with_level(self, checks: SelfChecks, strictness: Strictness) -> Res<Board> {
        let mut this = self.0;
        if checks == Assertion {
            ensure!(
                (this.player_bb(White) & this.player_bb(Black)).is_zero(),
                "A square is set both on the white and black player bitboard, but no piece bitboard has this bit set"
            );
            let mut pieces = Bitboard::default();
            for piece in PieceType::pieces() {
                pieces |= this.piece_bb(piece);
            }
            if pieces != this.bbs.colors[0] | this.bbs.colors[1] {
                bail!(
                    "The colored bitboards and the piece bitboards don't match on the following squares: {}",
                    pieces ^ (this.bbs.colors[0] | this.bbs.colors[1])
                );
            }
            for (i, &piece) in this.mailbox.iter().enumerate() {
                if piece == Empty {
                    ensure!(this.empty_bb().is_bit_set_at(i), "Mismatch between mailbox and bitboards at square {i}");
                } else {
                    ensure!(
                        this.piece_bb(piece).is_bit_set_at(i),
                        "Mismatch between mailbox and bitboards at square {i}"
                    );
                }
            }
        }
        if checks >= CheckFen {
            fen_selfchecks(&this, checks, strictness)?;
        }

        let inactive_player = this.active.other();
        if this.is_in_check_on_square(inactive_player, this.king_sq(inactive_player)) {
            bail!("{inactive_player} is in check, but it's not their turn to move");
        }
        this.set_checkers_and_pinned();
        this.threats = this.calc_threats_of(this.inactive_player());
        // in relaxed mode, we allow loading FENs where more than one piece gives check to the king in a way that
        // could not have been reached from startpos, e.g. "B6b/8/8/8/2K5/5k2/8/b6B b - - 0 1"
        if strictness == Strict && this.checkers.num_ones() > 2 {
            bail!(
                "{0} is in check from {1} pieces, which is not allowed in strict mode",
                this.active,
                this.checkers.num_ones()
            );
        }
        // We check the ep square close to last because this can require doing movegen, which needs most invariants to hold.
        this.check_ep(strictness, checks)?;
        this.hashes = this.compute_zobrist(); // depends on check_ep()
        Ok(this)
    }

    fn settings(&self) -> &Settings {
        self.0.settings()
    }

    fn size(&self) -> ChessboardSize {
        self.0.size()
    }

    // TODO: Change interface to pass color and piece separately?
    fn place_piece(&mut self, square: Square, piece: ColoredPieceType) {
        let this = &mut self.0;
        debug_assert!(this.is_empty(square));
        this.bbs.place_piece(square, piece.color().unwrap(), piece.uncolor());
        this.mailbox[square] = piece.uncolor();
    }

    fn remove_piece(&mut self, sq: Square) {
        let piece = self.0.colored_piece_on(sq);
        let color = piece.color().unwrap();
        let piece = piece.symbol.uncolor();
        self.0.remove_piece_impl(sq, piece, color);
        // It's not really clear how to so handle these flags when removing pieces, so we just unset them on a best effort basis
        if piece == Rook {
            for side in CastleRight::iter() {
                if self.0.castling.rook_start_file(color, side) == sq.file() && sq.rank() == 7 * color as DimT {
                    self.0.castling.unset_castle_right(color, side);
                }
            }
        } else if piece == Pawn && self.0.ep_square.is_some_and(|sq| sq.pawn_advance_unchecked(color) == sq) {
            self.0.ep_square = None;
        }
    }

    fn piece_on(&self, coords: Square) -> Piece {
        self.0.colored_piece_on(coords)
    }

    fn is_empty(&self, square: Square) -> bool {
        self.0.is_empty(square)
    }

    fn active_player(&self) -> Color {
        self.0.active
    }

    fn set_active_player(&mut self, player: Color) {
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

impl Board {
    fn check_ep(&mut self, strictness: Strictness, checks: SelfChecks) -> Res<()> {
        let Some(ep_square) = self.ep_square else { return Ok(()) };
        ensure!(
            [2, 5].contains(&ep_square.rank()),
            "FEN specifies invalid ep square (not on the third or sixth rank): '{ep_square}'"
        );
        let inactive = self.inactive_player();
        let remove_pawn_sq = ep_square.pawn_advance_unchecked(inactive);
        let pawn_origin_sq = ep_square.pawn_advance_unchecked(self.active);
        if !self.is_empty(ep_square) {
            bail!(
                "The en passant square '{ep_square}' must be empty, but it's occupied by a {}",
                self.colored_piece_on(ep_square)
            )
        } else if self.colored_piece_on(remove_pawn_sq).symbol != ColoredPieceType::new(inactive, Pawn) {
            bail!("FEN specifies en passant square '{ep_square}', but there is no {inactive} pawn on {remove_pawn_sq}");
        } else if !self.is_empty(pawn_origin_sq) {
            bail!(
                "The en passant square is set to '{ep_square}', so the pawn must have come from {pawn_origin_sq}. \
                    But this square isn't empty, it contains a {}",
                self.colored_piece_on(pawn_origin_sq)
            )
        }
        let active = self.active_player();
        // Not handling this here would cause us to not emit ep moves even though we think that the ep square is set
        match self.checkers.num_ones() {
            0 => {}
            1 => {
                let blockers_before = (self.occupied_bb() & !remove_pawn_sq.bb()) | pawn_origin_sq.bb();
                let sliders = ChessSliderGenerator::new(blockers_before);
                let attacks_before = self.all_attacking(self.king_sq(active), sliders);
                if !(attacks_before & self.player_bb(inactive) & !remove_pawn_sq.bb()).is_zero() {
                    bail!(
                        "The en passant square is set, but the {active} king was in check before the double pawn push"
                    )
                }
            }
            _ => bail!(
                "The en passant square is set, but there is more than one checker.\
                This means the {active} king has been in check before the double pawn push."
            ),
        }

        // In the current version of the FEN standard, the ep square should only be set if a pawn can legally capture.
        // This implementation follows that rule, but many other implementations give the ep square after every double pawn push.
        // To achieve consistent results, such an incorrect ep square is removed when parsing the FEN, this is the only case
        // where checks == CheckFen is actually stricter than other checks
        let possible_ep_pawns = remove_pawn_sq.bb().west() | remove_pawn_sq.bb().east();
        if (possible_ep_pawns & self.col_piece_bb(active, Pawn)).is_zero() {
            if strictness == Strict || checks != CheckFen {
                bail!(
                    "The en passant square is set to '{ep_square}', but there is no {active} pawn that could capture on that square. \
                        This is only allowed when reading FENs in relaxed mode (and will silently be converted to no ep square)"
                )
            }
            self.ep_square = None;
            return Ok(());
        }
        // an ep capture while in check is only legal if we're in check from the captured pawn.
        // a position where the capturing pawn would block a slider check is unreachable.
        let ep_legal = self.calc_ep_sq(remove_pawn_sq, &mut PosHash(0), active).is_some()
            && !self.checkers.intersects(!remove_pawn_sq.bb());
        if !ep_legal {
            if strictness == Strict || checks != CheckFen {
                bail!(
                    "The en passant square is set, but even though there is a pseudolegal ep capture move, it is not legal \
                            (either all pawns that could capture en passant are pinned, or the king is in check). \
                            This is not allowed when parsing FENs in strict mode"
                );
            }
            self.ep_square = None
        }
        Ok(())
    }
}

impl UnverifiedBoard {
    pub fn castling_rights_mut(&mut self) -> &mut CastlingFlags {
        &mut self.0.castling
    }

    pub fn set_ep(mut self, ep: Option<Square>) -> Self {
        self.0.ep_square = ep;
        self
    }

    pub fn random_unverified_pos(rng: &mut impl Rng, strictness: Strictness, symmetry: Option<Symmetry>) -> Self {
        let mut pos = Board::empty();
        let mask = if let Some(symmetry) = symmetry {
            match symmetry {
                Symmetry::Material => Bitboard::default().not(),
                Symmetry::Horizontal => Bitboard::new(0xf0f0_f0f0_f0f0_f0f0),
                Symmetry::Vertical => Bitboard::new(0xffff_ffff),
                Symmetry::Rotation180 => Bitboard::new(0xffff_ffff),
            }
        } else {
            Bitboard::default().not()
        };
        let king_sq1 = rng.random_range(0..mask.num_ones());
        let king_sq1 = Square::from_bb_idx(king_sq1);
        pos.place_piece(king_sq1, WhiteKing);
        let king_sq2 = if let Some(symmetry) = symmetry {
            mirror_sq(king_sq1, symmetry, rng, &pos.0)
        } else {
            loop {
                let king_sq2 = rng.random_range(0..64);
                let king_sq2 = Square::from_bb_idx(king_sq2);
                if king_sq2 == king_sq1 || Board::normal_king_attacks_from(king_sq2).has(king_sq1) {
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
                ColoredPieceType::new(White, PieceType::from_repr(piece).unwrap())
            } else {
                let piece = rng.random_range(0..10);
                let col = Color::iter().nth(piece / 5).unwrap();
                let piece = PieceType::from_repr(piece % 5).unwrap();
                ColoredPieceType::new(col, piece)
            };

            let empty = pos.0.empty_bb() & mask;
            let num_empty = empty.num_ones();
            loop {
                let sq_idx = rng.random_range(0..num_empty);
                let sq_idx = ith_one_u64(sq_idx, empty.raw());
                let sq = Square::from_bb_idx(sq_idx);
                if piece.uncolor() == Pawn && sq.is_backrank() {
                    continue;
                }
                pos.place_piece(sq, piece);
                if let Some(symmetry) = symmetry {
                    let sq = mirror_sq(sq, symmetry, rng, &pos.0);
                    let piece = ColoredPieceType::new(Black, piece.uncolor());
                    pos.place_piece(sq, piece);
                }
                break;
            }
        }
        // vertical and rotational symmetry keep the white pieces on the lower half of the board,
        // but this introduces a smallish chance to flip that
        if rng.random_bool(0.2) {
            pos.0.bbs.colors.swap(0, 1)
        }
        if rng.random_bool(0.5) {
            pos.0.active = !pos.0.active;
        }
        // don't generate castling or ep flags for now
        pos
    }
}

fn mirror_sq(sq: Square, symmetry: Symmetry, rng: &mut impl Rng, pos: &Board) -> Square {
    match symmetry {
        Symmetry::Material => {
            let empty = pos.empty_bb().raw();
            Square::from_bb_idx(ith_one_u64(rng.random_range(0..empty.num_ones()), empty))
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
    use rand::prelude::SmallRng;

    proptest! {
        #[test]
        fn random_unverified(seed in 0..=u64::MAX, strictness in 0..2, symmetry in 0..=Symmetry::iter().count()) {
            let mut rng = SmallRng::seed_from_u64(seed);
            let symmetry = Symmetry::iter().nth(symmetry);
            let strictness = if strictness == 0 { Strict } else { Relaxed };
            let res = UnverifiedBoard::random_unverified_pos(&mut rng, strictness, symmetry);
            let ok = res.verify_with_level(SelfChecks::Verify, strictness);
            if ok.is_ok() {
                assert!(res.verify_with_level(Assertion, Relaxed).is_ok());
            }
        }
    }
}
