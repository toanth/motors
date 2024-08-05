use crate::games::ataxx::common::AtaxxMoveType::{Cloning, Leaping};
use crate::games::ataxx::common::{AtaxxMove, ColoredAtaxxPieceType};
use crate::games::ataxx::AtaxxColor::{Black, White};
use crate::games::ataxx::{AtaxxBitboard, AtaxxBoard, AtaxxColor, AtaxxMoveList, AtaxxSettings};
use crate::games::{Board, Color, ZobristHash};
use crate::general::bitboards::ataxx::{INVALID_EDGE_MASK, LEAPING};
use crate::general::bitboards::chess::KINGS;
use crate::general::bitboards::{Bitboard, RawBitboard};
use crate::general::board::read_position_fen;
use crate::general::board::SelfChecks::CheckFen;
use crate::general::common::Res;
use crate::general::moves::Move;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::num::NonZeroUsize;
use std::str::SplitWhitespace;

impl AtaxxBoard {
    pub fn create(blocked: AtaxxBitboard, black: AtaxxBitboard, white: AtaxxBitboard) -> Res<Self> {
        let blocked = blocked | INVALID_EDGE_MASK;
        if (white & black).has_set_bit() {
            return Err(format!(
                "Overlapping white and black pieces (bitboard: {})",
                white & black
            ));
        }
        if (blocked & (white | black)).has_set_bit() {
            return Err(format!(
                "Pieces on blocked squares (bitboard: {}",
                blocked & (white | black)
            ));
        }
        // it's legal for the position to not contain any pieces at all
        Ok(Self {
            colors: [black.raw(), white.raw()],
            empty: !(blocked | white | black).raw(),
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

    pub(super) fn legal_moves(&self) -> AtaxxMoveList {
        let mut res = AtaxxMoveList::default();
        let pieces = self.active_bb();
        let empty = self.empty_bb();
        let neighbors = pieces.moore_neighbors() & empty;
        for sq in neighbors.ones() {
            res.push(AtaxxMove::cloning(sq));
        }
        for source in pieces.ones() {
            let leaps = LEAPING[source.bb_idx()] & empty;
            for target in leaps.ones() {
                res.push(AtaxxMove::leaping(source, target));
            }
        }
        if res.is_empty() && pieces.has_set_bit() {
            let other_bb = self.color_bb(self.active_player.other());
            // if the other player doesn't have any legal moves, the game is over.
            // return an empty move list in that case so that the user can pick up on this
            if (other_bb.extended_moore_neighbors(2) & empty).has_set_bit() {
                res.push(AtaxxMove::default());
            }
        }
        res
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

    pub fn read_fen_impl(words: &mut SplitWhitespace) -> Res<Self> {
        let pos_word = words
            .next()
            .ok_or_else(|| "Empty ataxx FEN string".to_string())?;
        let mut board = AtaxxBoard::empty_possibly_invalid(AtaxxSettings::default());
        board.empty = !board.empty; // use `empty` to keep track of blocked squares while building the board
        board = read_position_fen(pos_word, board, |mut board, square, typ| {
            match typ {
                ColoredAtaxxPieceType::Empty => {}
                ColoredAtaxxPieceType::Blocked => board.empty |= square.bb().raw(),
                ColoredAtaxxPieceType::WhitePiece => {
                    board.colors[White as usize] |= square.bb().raw();
                }
                ColoredAtaxxPieceType::BlackPiece => {
                    board.colors[Black as usize] |= square.bb().raw();
                }
            }
            Ok(board)
        })?;
        board.empty = !(board.empty | board.occupied_non_blocked_bb().raw());
        let color_word = words.next().ok_or_else(|| {
            "FEN ends after position description, missing color to move".to_string()
        })?;
        // be a bit lenient with parsing the fen
        let color = match color_word.to_ascii_lowercase().as_str() {
            "w" | "o" => Black,
            "b" | "x" => White,
            x => Err(format!("Expected color ('x' or 'o') in FEN, found '{x}'"))?,
        };
        if let Some(halfmove_clock) = words.next() {
            board.ply_100_ctr = halfmove_clock
                .parse::<usize>()
                .map_err(|err| format!("Couldn't parse halfmove clock: {err}"))?;
            let fullmove_number = words.next().unwrap_or("1");
            let fullmove_number = fullmove_number
                .parse::<NonZeroUsize>()
                .map_err(|err| format!("Couldn't parse fullmove counter: {err}"))?;
            board.ply = (fullmove_number.get() - 1) * 2 + usize::from(color == Black);
        } else {
            board.ply = usize::from(color == Black);
            board.ply_100_ctr = 0;
        }
        board.active_player = color;
        board.verify_position_legal(CheckFen)?;
        Ok(board)
    }
}
