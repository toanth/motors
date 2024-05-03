use strum::IntoEnumIterator;

use crate::games::chess::pieces::{ColoredChessPiece, UncoloredChessPiece};
use crate::games::chess::squares::{ChessSquare, NUM_COLUMNS};
use crate::games::chess::Chessboard;
use crate::games::Color::*;
use crate::games::{Color, ColoredPieceType, ZobristHash};
use crate::general::bitboards::RawBitboard;

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
        self.piece_square_keys[square.index() * 12 + piece as usize * 2 + color as usize]
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
        Self(
            seed.wrapping_add(INCREMENT)
                .wrapping_mul(MUTLIPLIER)
                .wrapping_add(INCREMENT),
        )
    }

    // const mut refs aren't stable yet, so returning the new state is a workaround
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
            PRECOMPUTED_ZOBRIST_KEYS.ep_file_keys[square.file() as usize]
        });
        res ^= PRECOMPUTED_ZOBRIST_KEYS.castle_keys[self.castling.allowed_castling_directions()];
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
    use std::collections::{HashMap, VecDeque};

    use itertools::Itertools;

    use crate::games::chess::moves::{ChessMove, ChessMoveFlags};
    use crate::games::chess::pieces::UncoloredChessPiece::{Bishop, Knight};
    use crate::games::chess::squares::{ChessSquare, D_FILE_NO, E_FILE_NO};
    use crate::games::chess::zobrist::{PcgXslRr128_64Oneseq, PRECOMPUTED_ZOBRIST_KEYS};
    use crate::games::chess::Chessboard;
    use crate::games::Color::{Black, White};
    use crate::games::{Board, ZobristHash};

    #[test]
    fn pcg_test() {
        let gen = PcgXslRr128_64Oneseq::new(42);
        assert_eq!(gen.0 >> 64, 1610214578838163691);
        assert_eq!(gen.0 & ((1 << 64) - 1), 13841303961814150380);
        let (gen, rand) = gen.gen();
        assert_eq!(rand.0, 2915081201720324186);
        let (gen, rand) = gen.gen();
        assert_eq!(rand.0, 13533757442135995717);
        let (gen, rand) = gen.gen();
        assert_eq!(rand.0, 13172715927431628928);
    }

    #[test]
    fn simple_test() {
        let a1 = PRECOMPUTED_ZOBRIST_KEYS
            .piece_key(Bishop, White, ChessSquare::from_chars('f', '4').unwrap())
            .0;
        let b1 = PRECOMPUTED_ZOBRIST_KEYS
            .piece_key(Bishop, White, ChessSquare::from_chars('g', '5').unwrap())
            .0;
        let a2 = PRECOMPUTED_ZOBRIST_KEYS
            .piece_key(Knight, Black, ChessSquare::from_chars('h', '5').unwrap())
            .0;
        let b2 = PRECOMPUTED_ZOBRIST_KEYS
            .piece_key(Knight, Black, ChessSquare::from_chars('g', '4').unwrap())
            .0;
        assert_ne!(a1 ^ a2, b1 ^ b2); // used to be bugged
        let position = Chessboard::from_name("kiwipete").unwrap();
        let hash = position.hash;
        let mut hashes = HashMap::new();
        let mut collisions = HashMap::new();
        for mov in position.legal_moves_slow() {
            let new_board = position.make_move(mov).unwrap();
            assert_ne!(new_board.hash, hash);
            let previous = hashes.insert(new_board.hash.0, new_board);
            assert!(previous.is_none());
            let different_bits = (new_board.hash.0 ^ hash.0).count_ones();
            assert!(different_bits >= 16 && different_bits <= 48);
            for mov in new_board.legal_moves_slow() {
                let new_board = new_board.make_move(mov).unwrap();
                let previous = hashes.insert(new_board.hash.0, new_board);
                if previous.is_some() {
                    let old_board = previous.unwrap();
                    println!(
                        "Collision at hash {hash}, boards {0} and {1} (diagrams: \n{old_board} and \n{new_board}",
                        old_board.as_fen(),
                        new_board.as_fen()
                    );
                    // There's one ep move after one ply from the current position, which creates the only transposition reachable within 2 plies
                    if old_board != new_board {
                        collisions.insert(new_board.hash.0, [old_board, new_board]);
                    }
                }
                let different_bits = (new_board.hash.0 ^ hash.0).count_ones();
                println!("{different_bits}");
                assert!(different_bits >= 12 && different_bits < 52);
            }
        }
        assert!(
            collisions.is_empty(),
            "num collisions: {0} out of {1}",
            collisions.len(),
            hashes.len()
        );
    }

    #[test]
    fn statistical_test() {
        let position = Chessboard::default();
        let mut hashes = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(position);
        let max_queue_len = if cfg!(debug_assertions) {
            500_000
        } else {
            5_000_000
        };
        while queue.len() <= max_queue_len {
            let pos = queue.front().copied().unwrap();
            let moves = pos.legal_moves_slow();
            queue.pop_front();
            hashes.push(pos.hash);
            for mov in moves {
                queue.push_back(pos.make_move(mov).unwrap());
            }
        }
        for entry in queue {
            hashes.push(entry.hash);
        }
        hashes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for shift in 0..64 - 8 {
            let get_bits = |hash: ZobristHash| (hash.0 >> shift) & 0xff;
            let mut counts = vec![0; 256];
            for hash in hashes.iter() {
                counts[get_bits(*hash) as usize] += 1;
            }
            let expected = hashes.len() / 256;
            for count in counts {
                assert!(count >= expected / 2);
                assert!(count <= expected + expected / 2);
            }
        }
    }

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
