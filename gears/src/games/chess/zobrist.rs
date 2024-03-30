use strum::IntoEnumIterator;

use crate::games::chess::pieces::{ColoredChessPiece, UncoloredChessPiece};
use crate::games::chess::squares::{ChessSquare, NUM_COLUMNS};
use crate::games::chess::Chessboard;
use crate::games::Color::*;
use crate::games::{Color, ColoredPieceType, ZobristHash};
use crate::general::bitboards::Bitboard;

pub const NUM_PIECE_SQUARE_ENTRIES: usize = 64 * 6;
pub const NUM_COLORED_PIECE_SQUARE_ENTRIES: usize = NUM_PIECE_SQUARE_ENTRIES * 2;

pub struct PrecomputedZobristKeys {
    pub piece_square_keys: [ZobristHash; NUM_COLORED_PIECE_SQUARE_ENTRIES],
    pub castle_keys: [ZobristHash; 1 << (2 * 2)],
    pub ep_file_keys: [ZobristHash; NUM_COLUMNS],
    pub side_to_move_key: ZobristHash,
}

impl PrecomputedZobristKeys {
    pub fn piece_key(
        &self,
        piece: UncoloredChessPiece,
        color: Color,
        square: ChessSquare,
    ) -> ZobristHash {
        self.piece_square_keys[square.index() + piece as usize * 2 + color as usize]
    }
}

/// A simple `const` random number generator adapted from my C++ algebra implementation,
/// originally from here: https://www.pcg-random.org/ (I hate that website)
struct PcgXslRr128_64Oneseq(u128);

const MUTLIPLIER: u128 = (2549297995355413924 << 64) + 4865540595714422341;
const INCREMENT: u128 = (6364136223846793005 << 64) + 1442695040888963407;

// the pcg xsl rr 128 64 oneseq generator, aka pcg64_oneseq (most other pcg generators have additional problems)
impl PcgXslRr128_64Oneseq {
    const fn new(seed: u128) -> Self {
        Self(seed + INCREMENT)
    }

    const fn gen(mut self) -> (Self, ZobristHash) {
        self.0 = self.0.wrapping_mul(MUTLIPLIER);
        self.0 = self.0.wrapping_add(INCREMENT);
        let upper = (self.0 >> 64) as u64;
        let xored = upper ^ ((self.0 & u64::MAX as u128) as u64);
        (
            self,
            ZobristHash(xored.rotate_right((upper >> (122 - 64)) as u32)),
        )
    }
}

// Unfortunately, `const_random!` generates new values each time, so the build isn't deterministic unless
// the environment variable CONST_RANDOM_SEED is set
pub const PRECOMPUTED_ZOBRIST_KEYS: PrecomputedZobristKeys = {
    let mut res = {
        PrecomputedZobristKeys {
            piece_square_keys: [ZobristHash(0); NUM_COLORED_PIECE_SQUARE_ENTRIES],
            castle_keys: [ZobristHash(0); 1 << (2 * 2)],
            ep_file_keys: [ZobristHash(0); NUM_COLUMNS],
            side_to_move_key: ZobristHash(0),
        }
    };
    let mut gen = PcgXslRr128_64Oneseq::new(0x42);
    let mut i = 0;
    while i < NUM_COLORED_PIECE_SQUARE_ENTRIES {
        (gen, res.piece_square_keys[i]) = gen.gen();
        i += 1;
    }
    let mut i = 0;
    while i < res.castle_keys.len() {
        (gen, res.castle_keys[i]) = gen.gen();
        i += 1;
    }
    let mut i = 0;
    while i < NUM_COLUMNS {
        (gen, res.ep_file_keys[i]) = gen.gen();
        i += 1;
    }
    (_, res.side_to_move_key) = gen.gen();
    res
};

impl Chessboard {
    pub fn compute_zobrist(&self) -> ZobristHash {
        let mut res = ZobristHash(0);
        for color in Color::iter() {
            for piece in UncoloredChessPiece::pieces() {
                let mut pieces = self.colored_piece_bb(color, piece);
                while pieces.has_set_bit() {
                    let idx = pieces.pop_lsb();
                    res ^= PRECOMPUTED_ZOBRIST_KEYS.piece_key(piece, color, ChessSquare::new(idx));
                }
            }
        }
        res ^= self.ep_square.map_or(ZobristHash(0), |square| {
            PRECOMPUTED_ZOBRIST_KEYS.ep_file_keys[square.file()]
        });
        res ^= PRECOMPUTED_ZOBRIST_KEYS.castle_keys[self.flags.castling_flags() as usize];
        if self.active_player == Black {
            res ^= PRECOMPUTED_ZOBRIST_KEYS.side_to_move_key;
        }
        res
    }

    // pub(super) fn update_zobrist(&mut self, mov: ChessMove) {
    //     let captured = mov.captured(self);
    //     let mut hash = self.zobrist_hash();
    //     let color = self.active_player;
    //     let new_color = self.active_player.other();
    //     let to_idx = mov.to_square().index();
    //     let piece = mov.piece(self).uncolored_piece_type();
    //     if captured != Empty {
    //         hash ^= PRECOMPUTED_ZOBRIST_KEYS.piece_key(captured, new_color, to_idx);
    //     }
    //     if mov.is_promotion() {}
    //     self.update_for_move(
    //         ColoredChessPiece::new(color, piece),
    //         mov.from_square(),
    //         mov.to_square(),
    //     );
    //     hash ^= PRECOMPUTED_ZOBRIST_KEYS.side_to_move_key;
    // }

    pub fn update_zobrist_for_move(
        &mut self,
        piece: ColoredChessPiece,
        from: ChessSquare,
        to: ChessSquare,
    ) {
        let color = piece.color().unwrap();
        self.hash ^= PRECOMPUTED_ZOBRIST_KEYS.piece_key(piece.uncolor(), color, to);
        self.hash ^= PRECOMPUTED_ZOBRIST_KEYS.piece_key(piece.uncolor(), color, from);
    }
}

#[cfg(test)]
mod tests {
    use crate::games::chess::moves::{ChessMove, ChessMoveFlags};
    use crate::games::chess::squares::{ChessSquare, D_FILE_NO, E_FILE_NO};
    use crate::games::chess::Chessboard;
    use crate::games::Board;

    #[test]
    fn ep_test() {
        let position =
            Chessboard::from_fen("4r1k1/p4pp1/6bp/2p5/r2p4/P4PPP/1P2P3/2RRB1K1 w - - 1 15")
                .unwrap();
        assert_eq!(position.zobrist_hash(), position.compute_zobrist());
        let mov = ChessMove::new(
            ChessSquare::from_rank_file(1, E_FILE_NO),
            ChessSquare::from_rank_file(3, E_FILE_NO),
            ChessMoveFlags::Normal,
        );
        let new_pos = position.make_move(mov).unwrap();
        assert_eq!(new_pos.zobrist_hash(), new_pos.compute_zobrist());
        let ep_move = ChessMove::new(
            ChessSquare::from_rank_file(3, D_FILE_NO),
            ChessSquare::from_rank_file(2, E_FILE_NO),
            ChessMoveFlags::EnPassant,
        );
        let after_ep = new_pos.make_move(ep_move).unwrap();
        assert_eq!(after_ep.zobrist_hash(), after_ep.compute_zobrist());
    }
}
