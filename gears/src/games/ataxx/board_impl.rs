use crate::games::ataxx::common::AtaxxMove;
use crate::games::ataxx::common::AtaxxMoveType::{Cloning, Leaping};
use crate::games::ataxx::{AtaxxBitboard, AtaxxBoard, AtaxxColor, AtaxxMoveList};
use crate::games::{Board, Color, ZobristHash};
use crate::general::bitboards::ataxx::{INVALID_EDGE_MASK, LEAPING};
use crate::general::bitboards::chess::KINGS;
use crate::general::bitboards::{Bitboard, RawBitboard};
use crate::general::board::SelfChecks::CheckFen;
use crate::general::board::Strictness::Strict;
use crate::general::board::{read_common_fen_part, Strictness, UnverifiedBoard};
use crate::general::common::Res;
use crate::general::move_list::MoveList;
use crate::general::moves::Move;
use anyhow::{anyhow, bail};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::iter::Peekable;
use std::num::NonZeroUsize;
use std::str::SplitWhitespace;

impl AtaxxBoard {
    pub fn create(blocked: AtaxxBitboard, x_bb: AtaxxBitboard, o_bb: AtaxxBitboard) -> Res<Self> {
        let blocked = blocked | INVALID_EDGE_MASK;
        if (o_bb & x_bb).has_set_bit() {
            return Err(anyhow!(
                "Overlapping x and o pieces (bitboard: {})",
                o_bb & x_bb
            ));
        }
        if (blocked & (o_bb | x_bb)).has_set_bit() {
            return Err(anyhow!(
                "Pieces on blocked squares (bitboard: {}",
                blocked & (o_bb | x_bb)
            ));
        }
        // it's legal for the position to not contain any pieces at all
        Ok(Self {
            colors: [x_bb.raw(), o_bb.raw()],
            empty: !(blocked | o_bb | x_bb).raw(),
            active_player: AtaxxColor::first(),
            ply_100_ctr: 0,
            ply: 0,
        })
    }

    pub fn color_bb(&self, color: AtaxxColor) -> AtaxxBitboard {
        AtaxxBitboard::new(self.colors[color as usize])
    }

    pub fn occupied_non_blocked_bb(&self) -> AtaxxBitboard {
        AtaxxBitboard::new(self.colors[0] | self.colors[1])
    }

    pub fn empty_bb(&self) -> AtaxxBitboard {
        AtaxxBitboard::new(self.empty)
    }

    pub fn blocked_bb(&self) -> AtaxxBitboard {
        AtaxxBitboard::new(!(self.empty | self.colors[0] | self.colors[1]))
    }

    pub fn active_bb(&self) -> AtaxxBitboard {
        self.color_bb(self.active_player)
    }

    pub fn inactive_bb(&self) -> AtaxxBitboard {
        self.color_bb(!self.active_player)
    }

    pub(super) fn gen_legal<T: MoveList<Self>>(&self, moves: &mut T) {
        let pieces = self.active_bb();
        let empty = self.empty_bb();
        let neighbors = pieces.moore_neighbors() & empty;
        for sq in neighbors.ones() {
            moves.add_move(AtaxxMove::cloning(sq));
        }
        for source in pieces.ones() {
            let leaps = LEAPING[source.bb_idx()] & empty;
            for target in leaps.ones() {
                moves.add_move(AtaxxMove::leaping(source, target));
            }
        }
        if moves.num_moves() == 0 && pieces.has_set_bit() {
            let other_bb = self.color_bb(self.active_player.other());
            // if the other player doesn't have any legal moves, the game is over.
            // return an empty move list in that case so that the user can pick up on this
            if (other_bb.extended_moore_neighbors(2) & empty).has_set_bit() {
                moves.add_move(AtaxxMove::default());
            }
        }
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
        debug_assert!(
            mov.typ() == Cloning
                || self
                    .color_bb(color)
                    .is_bit_set_at(mov.src_square().bb_idx())
        );
        if mov.typ() == Leaping {
            let source_bb = mov.src_square().bb().raw();
            self.colors[color as usize] ^= source_bb;
            self.empty ^= source_bb;
            self.ply_100_ctr += 1;
        } else {
            self.ply_100_ctr = 0;
        }
        debug_assert!(self.empty_bb().is_bit_set_at(mov.dest_square().bb_idx()));
        let dest = mov.dest_square();
        let dest_bb = dest.bb().raw();
        let in_range = KINGS[dest.bb_idx()].raw();
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
            return moves
                .iter()
                .next()
                .is_some_and(|m| *m == AtaxxMove::default());
        }
        let empty = self.empty_bb();
        if !empty.is_bit_set_at(mov.dest_square().bb_idx()) {
            return false;
        }
        if mov.typ() == Cloning {
            self.active_bb()
                .moore_neighbors()
                .is_bit_set_at(mov.dest_square().bb_idx())
        } else {
            self.active_bb()
                .extended_moore_neighbors(2)
                .is_bit_set_at(mov.dest_square().bb_idx())
        }
    }

    /// Doesn't actually use a *zobrist* hash computation because that's unnecessarily slow for ataxx.
    /// TODO: Test performance, both of the hasher and of the TT when using this hasher.
    pub(super) fn hash_impl(&self) -> ZobristHash {
        let mut hasher = DefaultHasher::new();
        (self.colors[0], self.colors[1], self.active_player).hash(&mut hasher);
        ZobristHash(hasher.finish())
    }

    pub fn read_fen_impl(
        words: &mut Peekable<SplitWhitespace>,
        strictness: Strictness,
    ) -> Res<Self> {
        let empty = AtaxxBoard::empty();
        let mut board = read_common_fen_part::<AtaxxBoard>(words, empty.into())?;
        let color = board.0.active_player();
        if let Some(halfmove_clock) = words.next() {
            board.0.ply_100_ctr = halfmove_clock
                .parse::<usize>()
                .map_err(|err| anyhow!("Couldn't parse halfmove clock: {err}"))?;
            let fullmove_number = words.next().unwrap_or("1");
            let fullmove_number = fullmove_number
                .parse::<NonZeroUsize>()
                .map_err(|err| anyhow!("Couldn't parse fullmove counter: {err}"))?;
            board.0.ply =
                (fullmove_number.get() - 1) * 2 + usize::from(color == AtaxxColor::second());
        } else if strictness == Strict {
            bail!("In strict mode, FENs must contain a move counter and halfmove clock")
        } else {
            board.0.ply = usize::from(color == AtaxxColor::second());
            board.0.ply_100_ctr = 0;
        }
        board.verify_with_level(CheckFen, strictness)
    }
}
