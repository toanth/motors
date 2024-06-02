use crate::games::ataxx::common::AtaxxMoveType::{Cloning, Leaping};
use crate::games::ataxx::common::{AtaxxMove, AtaxxSquare, ColoredAtaxxPieceType};
use crate::games::ataxx::{
    AtaxxBitboard, AtaxxBoard, AtaxxMoveList, AtaxxSettings, INVALID_EDGE_MASK,
};
use crate::games::Color::{Black, White};
use crate::games::{read_position_fen, Board, Color, Move, ZobristHash};
use crate::general::bitboards::chess::KINGS;
use crate::general::bitboards::{Bitboard, RawBitboard};
use crate::general::common::Res;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::num::NonZeroUsize;
use std::str::SplitWhitespace;

impl AtaxxBoard {
    pub fn new(blocked: AtaxxBitboard) -> Self {
        let blocked = blocked | INVALID_EDGE_MASK;
        Self {
            colors: [AtaxxBitboard::default(); 2],
            empty: !blocked,
            active_player: Color::default(),
            ply_100_ctr: 0,
            ply: 0,
        }
    }

    pub fn color_bb(&self, color: Color) -> AtaxxBitboard {
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

    pub fn active_bb(&self) -> AtaxxBitboard {
        self.color_bb(self.active_player)
    }

    pub(super) fn movegen(&self) -> AtaxxMoveList {
        let mut res = AtaxxMoveList::default();
        let pieces = self.active_bb();
        let empty = self.empty_bb();
        let neighbors = pieces.moore_neighbors() & empty;
        for sq in neighbors.ones() {
            res.push(AtaxxMove::cloning(AtaxxSquare::new(sq)));
        }
        for source in pieces.ones() {
            let source_bb = AtaxxBitboard::single_piece(source);
            let leaps =
                (source_bb.extended_moore_neighbors(2) ^ source_bb.moore_neighbors()) & empty;
            for target in leaps.ones() {
                res.push(AtaxxMove::leaping(
                    AtaxxSquare::new(source),
                    AtaxxSquare::new(target),
                ));
            }
        }
        if res.is_empty() {
            res.push(AtaxxMove::default())
        }
        res
    }

    pub(super) fn make_move_impl(mut self, mov: AtaxxMove) -> Self {
        let color = self.active_player;
        self.active_player = color.other();
        if mov == AtaxxMove::default() {
            return self;
        }
        debug_assert!(
            mov.typ() == Cloning || self.color_bb(color).is_bit_set_at(mov.src_square().index())
        );
        debug_assert!(self.empty_bb().is_bit_set_at(mov.dest_square().index()));
        if mov.typ() == Leaping {
            self.colors[color as usize] &= !mov.src_square().bb();
        }
        let dest = mov.dest_square();
        let in_range = KINGS[dest.index()];
        let new_pieces = (self.colors[color.other() as usize] & in_range) | dest.bb();
        self.colors[color.other() as usize] ^= new_pieces;
        self.colors[color as usize] ^= new_pieces;
        self
    }

    pub(super) fn is_move_legal_impl(&self, mov: AtaxxMove) -> bool {
        if mov == AtaxxMove::default() {
            return self.legal_moves_slow().is_empty();
        }
        let empty = self.empty_bb();
        if !empty.is_bit_set_at(mov.dest_square().index()) {
            return false;
        }
        if mov.typ() == Cloning {
            self.active_bb()
                .moore_neighbors()
                .is_bit_set_at(mov.dest_square().index())
        } else {
            self.active_bb()
                .extended_moore_neighbors(2)
                .is_bit_set_at(mov.dest_square().index())
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
                ColoredAtaxxPieceType::Blocked => board.empty |= square.bb(),
                ColoredAtaxxPieceType::WhitePiece => board.colors[White as usize] |= square.bb(),
                ColoredAtaxxPieceType::BlackPiece => board.colors[Black as usize] |= square.bb(),
            }
            Ok(board)
        })?;
        board.empty = !(board.empty | board.occupied_non_blocked_bb());
        let color_word = words.next().ok_or_else(|| {
            "FEN ends after position description, missing color to move".to_string()
        })?;
        // be a bit lenient with parsing the fen
        let color = match color_word.to_ascii_lowercase().as_str() {
            "w" => Black,
            "b" => White,
            x => Err(format!("Expected color ('w' or 'b') in FEN, found '{x}'"))?,
        };
        if let Some(halfmove_clock) = words.next() {
            board.ply_100_ctr = halfmove_clock
                .parse::<usize>()
                .map_err(|err| format!("Couldn't parse halfmove clock: {err}"))?;
            let fullmove_number = words.next().unwrap_or("1");
            let fullmove_number = fullmove_number
                .parse::<NonZeroUsize>()
                .map_err(|err| format!("Couldn't parse fullmove counter: {err}"))?;
            board.ply = (fullmove_number.get() - 1) * 2 + (color == Black) as usize;
        } else {
            board.ply = (color == Black) as usize;
            board.ply_100_ctr = 0;
        }
        board.active_player = color;
        board.verify_position_legal()?;
        Ok(board)
    }
}
