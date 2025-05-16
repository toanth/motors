use crate::games::ataxx::common::AtaxxMove;
use crate::games::ataxx::common::AtaxxMoveType::{Cloning, Leaping};
use crate::games::ataxx::{AtaxxBitboard, AtaxxBoard, AtaxxColor, AtaxxMoveList, AtaxxSquare, UnverifiedAtaxxBoard};
use crate::games::{Board, Color, PosHash};
use crate::general::bitboards::chessboard::{ATAXX_LEAPERS, KINGS};
use crate::general::bitboards::{Bitboard, KnownSizeBitboard, RawBitboard};
use crate::general::board::SelfChecks::CheckFen;
use crate::general::board::{BitboardBoard, BoardHelpers, Strictness, UnverifiedBoard, read_simple_fen_part};
use crate::general::common::{Res, Tokens};
use crate::general::move_list::MoveList;
use crate::general::squares::sup_distance;
use anyhow::anyhow;
use std::hash::{DefaultHasher, Hash, Hasher};

impl AtaxxBoard {
    pub fn create(blocked: AtaxxBitboard, x_bb: AtaxxBitboard, o_bb: AtaxxBitboard) -> Res<Self> {
        let blocked = blocked | AtaxxBitboard::INVALID_EDGE_MASK;
        if (o_bb & x_bb).has_set_bit() {
            return Err(anyhow!("Overlapping x and o pieces (bitboard: {})", o_bb & x_bb));
        }
        if (blocked & (o_bb | x_bb)).has_set_bit() {
            return Err(anyhow!("Pieces on blocked squares (bitboard: {}", blocked & (o_bb | x_bb)));
        }
        // it's legal for the position to not contain any pieces at all
        Ok(Self {
            colors: [x_bb, o_bb],
            empty: !(blocked | o_bb | x_bb),
            active_player: AtaxxColor::first(),
            ply_100_ctr: 0,
            ply: 0,
        })
    }

    pub fn color_bb(&self, color: AtaxxColor) -> AtaxxBitboard {
        self.colors[color as usize]
    }

    pub fn occupied_non_blocked_bb(&self) -> AtaxxBitboard {
        self.colors[0] | self.colors[1]
    }

    pub fn empty_bb(&self) -> AtaxxBitboard {
        self.empty
    }

    pub fn blocked_bb(&self) -> AtaxxBitboard {
        !(self.empty | self.colors[0] | self.colors[1])
    }

    pub(super) fn gen_legal<T: MoveList<Self>>(&self, moves: &mut T) {
        let pieces = self.active_player_bb();
        let empty = self.empty_bb();
        // TODO: Use precomputed table
        let neighbors = pieces.moore_neighbors() & empty;
        for sq in neighbors.ones() {
            moves.add_move(AtaxxMove::cloning(sq));
        }
        for source in pieces.ones() {
            let leaps = AtaxxBitboard::new(ATAXX_LEAPERS[source.bb_idx()].raw()) & empty;
            for target in leaps.ones() {
                moves.add_move(AtaxxMove::leaping(source, target));
            }
        }
        if moves.num_moves() == 0 && pieces.has_set_bit() {
            let other_bb = self.inactive_player_bb();
            // if the other player doesn't have any legal moves, the game is over.
            // return an empty move list in that case so that the user can pick up on this
            // otherwise, the only legal move is the passing move
            if (other_bb.extended_moore_neighbors(2) & empty).has_set_bit() {
                moves.add_move(AtaxxMove::default());
            }
        }
    }

    pub(super) fn num_moves(&self) -> usize {
        let pieces = self.active_player_bb();
        let empty = self.empty_bb();
        let mut res = (pieces.moore_neighbors() & empty).num_ones();
        for source in pieces.ones() {
            let leaps = AtaxxBitboard::new(ATAXX_LEAPERS[source.bb_idx()].raw()) & empty;
            res += leaps.num_ones();
        }
        if res == 0 && pieces.has_set_bit() {
            // if the other player doesn't have any legal moves, the game is over.
            // otherwise, the only legal move is the passing move
            if (self.inactive_player_bb().extended_moore_neighbors(2) & empty).has_set_bit() {
                return 1;
            }
        }
        res
    }

    pub fn legal_moves(&self) -> AtaxxMoveList {
        self.pseudolegal_moves()
    }

    pub(super) fn make_move_impl(mut self, mov: AtaxxMove) -> Self {
        let color = self.active_player;
        self.active_player = color.other();
        self.ply += 1;
        if mov == AtaxxMove::default() {
            self.ply_100_ctr += 1;
            return self;
        }
        debug_assert!(mov.typ() == Cloning || self.color_bb(color).is_bit_set_at(mov.source as usize));
        if mov.typ() == Leaping {
            let source = AtaxxSquare::unchecked(mov.source as usize);
            let source_bb = AtaxxBitboard::single_piece(source);
            self.colors[color as usize] ^= source_bb;
            self.empty ^= source_bb;
            self.ply_100_ctr += 1;
        } else {
            self.ply_100_ctr = 0;
        }
        debug_assert!(self.empty_bb().is_bit_set_at(mov.dest_square().bb_idx()));
        let dest = mov.dest_square();
        let dest_bb = AtaxxBitboard::single_piece(dest);
        let in_range = AtaxxBitboard::new(KINGS[dest.bb_idx()].raw());
        let converted = self.colors[color.other() as usize] & in_range;
        debug_assert!((converted & dest_bb).is_zero());
        self.colors[color.other() as usize] ^= converted;
        self.colors[color as usize] |= converted | dest_bb;
        self.empty ^= dest_bb;
        self
    }

    pub(super) fn is_move_legal_impl(&self, mov: AtaxxMove) -> bool {
        if mov == AtaxxMove::default() {
            let moves = self.pseudolegal_moves();
            return moves.iter().next().is_some_and(|m| *m == AtaxxMove::default());
        }
        let empty = self.empty_bb();
        if !empty.is_bit_set_at(mov.dest_square().bb_idx()) {
            return false;
        }
        let pieces = self.active_player_bb();
        if mov.typ() == Cloning {
            pieces.moore_neighbors().is_bit_set_at(mov.dest_square().bb_idx())
        } else {
            let source = AtaxxSquare::unchecked(mov.source as usize);
            pieces.is_bit_set_at(mov.source as usize) && sup_distance(source, mov.dest_square()) == 2
        }
    }

    /// Doesn't actually use a *zobrist* hash computation because that's unnecessarily slow for ataxx.
    /// TODO: Test performance, both of the hasher and of the TT when using this hasher.
    pub(super) fn hash_impl(&self) -> PosHash {
        let mut hasher = DefaultHasher::new();
        (self.colors[0], self.colors[1], self.active_player).hash(&mut hasher);
        PosHash(hasher.finish())
    }

    pub fn read_fen_impl(words: &mut Tokens, strictness: Strictness) -> Res<Self> {
        let mut board = UnverifiedAtaxxBoard::new(AtaxxBoard::empty());
        read_simple_fen_part::<AtaxxBoard>(words, &mut board, strictness)?;
        board.verify_with_level(CheckFen, strictness)
    }
}
