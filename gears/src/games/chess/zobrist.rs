use crate::games::chess::ChessColor::Black;
use crate::games::chess::pieces::ChessPieceType;
use crate::games::chess::pieces::ChessPieceType::Pawn;
use crate::games::chess::squares::{ChessSquare, NUM_COLUMNS};
use crate::games::chess::{ChessColor, Chessboard, Hashes};
use crate::games::{Color, NUM_COLORS, PosHash};
use crate::general::bitboards::Bitboard;
use crate::general::board::BitboardBoard;
use crate::general::squares::RectangularCoordinates;

pub const NUM_PIECE_SQUARE_ENTRIES: usize = 64 * 6;
pub const NUM_COLORED_PIECE_SQUARE_ENTRIES: usize = NUM_PIECE_SQUARE_ENTRIES * 2;

pub struct PrecomputedZobristKeys {
    pub piece_square_keys: [PosHash; NUM_COLORED_PIECE_SQUARE_ENTRIES],
    pub castle_keys: [PosHash; 1 << (2 * 2)],
    pub ep_file_keys: [PosHash; NUM_COLUMNS],
    pub side_to_move_key: PosHash,
}

impl PrecomputedZobristKeys {
    pub fn piece_key(&self, piece: ChessPieceType, color: ChessColor, square: ChessSquare) -> PosHash {
        self.piece_square_keys[square.bb_idx() + piece as usize * 64 + color as usize * 64 * 6]
    }
}

/// A simple `const` random number generator adapted from my C++ algebra implementation,
/// originally from here: <https://www.pcg-random.org/> (I hate that website)
struct PcgXslRr128_64Oneseq(u128);

const MUTLIPLIER: u128 = (2_549_297_995_355_413_924 << 64) + 4_865_540_595_714_422_341;
const INCREMENT: u128 = (6_364_136_223_846_793_005 << 64) + 1_442_695_040_888_963_407;

// the pcg xsl rr 128 64 oneseq generator, aka pcg64_oneseq (most other pcg generators have additional problems)
impl PcgXslRr128_64Oneseq {
    const fn new(seed: u128) -> Self {
        Self(seed.wrapping_add(INCREMENT).wrapping_mul(MUTLIPLIER).wrapping_add(INCREMENT))
    }

    const fn generate(&mut self) -> PosHash {
        self.0 = self.0.wrapping_mul(MUTLIPLIER);
        self.0 = self.0.wrapping_add(INCREMENT);
        let upper = (self.0 >> 64) as u64;
        let xored = upper ^ (self.0 as u64);
        PosHash(xored.rotate_right((upper >> (122 - 64)) as u32))
    }
}

pub const ZOBRIST_KEYS: PrecomputedZobristKeys = {
    let mut res = {
        PrecomputedZobristKeys {
            piece_square_keys: [PosHash(0); NUM_COLORED_PIECE_SQUARE_ENTRIES],
            castle_keys: [PosHash(0); 1 << (2 * 2)],
            ep_file_keys: [PosHash(0); NUM_COLUMNS],
            side_to_move_key: PosHash(0),
        }
    };
    let mut generator = PcgXslRr128_64Oneseq::new(0x42);
    let mut i = 0;
    while i < NUM_COLORED_PIECE_SQUARE_ENTRIES {
        res.piece_square_keys[i] = generator.generate();
        i += 1;
    }
    let mut i = 0;
    while i < res.castle_keys.len() {
        res.castle_keys[i] = generator.generate();
        i += 1;
    }
    let mut i = 0;
    while i < NUM_COLUMNS {
        res.ep_file_keys[i] = generator.generate();
        i += 1;
    }
    res.side_to_move_key = generator.generate();
    res
};

impl Chessboard {
    pub(super) fn compute_zobrist(&self) -> Hashes {
        let mut pawns = PosHash(0);
        let mut nonpawns = [PosHash(0); NUM_COLORS];
        let mut special = PosHash(0);
        let mut knb = PosHash(0);
        for color in ChessColor::iter() {
            for piece in ChessPieceType::non_pawn_pieces() {
                let pieces = self.col_piece_bb(color, piece);
                for square in pieces.ones() {
                    let key = ZOBRIST_KEYS.piece_key(piece, color, square);
                    nonpawns[color] ^= key;
                    if piece.is_knb() {
                        knb ^= key;
                    }
                }
            }
            for square in self.col_piece_bb(color, Pawn).ones() {
                pawns ^= ZOBRIST_KEYS.piece_key(Pawn, color, square);
            }
        }
        special ^= self.ep_square.map_or(PosHash(0), |square| ZOBRIST_KEYS.ep_file_keys[square.file() as usize]);
        special ^= ZOBRIST_KEYS.castle_keys[self.castling.allowed_castling_directions()];
        if self.active == Black {
            special ^= ZOBRIST_KEYS.side_to_move_key;
        }
        Hashes { pawns, nonpawns, knb, total: pawns ^ nonpawns[0] ^ nonpawns[1] ^ special }
    }

    pub fn zobrist_delta(color: ChessColor, piece: ChessPieceType, from: ChessSquare, to: ChessSquare) -> PosHash {
        ZOBRIST_KEYS.piece_key(piece, color, to) ^ ZOBRIST_KEYS.piece_key(piece, color, from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::chess::ChessColor::White;
    use crate::games::chess::moves::{ChessMove, ChessMoveFlags};
    use crate::games::chess::pieces::ChessPieceType::*;
    use crate::games::chess::squares::{D_FILE_NUM, E_FILE_NUM};
    use crate::general::board::Strictness::Strict;
    use crate::general::board::{Board, BoardHelpers};
    use crate::general::moves::Move;
    use std::collections::HashMap;

    #[test]
    fn pcg_test() {
        let mut generator = PcgXslRr128_64Oneseq::new(42);
        assert_eq!(generator.0 >> 64, 1_610_214_578_838_163_691);
        assert_eq!(generator.0 & ((1 << 64) - 1), 13_841_303_961_814_150_380);
        let rand = generator.generate();
        assert_eq!(rand.0, 2_915_081_201_720_324_186);
        let rand = generator.generate();
        assert_eq!(rand.0, 13_533_757_442_135_995_717);
        let rand = generator.generate();
        assert_eq!(rand.0, 13_172_715_927_431_628_928);
    }

    #[test]
    fn simple_test() {
        let a1 = ZOBRIST_KEYS.piece_key(Bishop, White, ChessSquare::from_chars('f', '4').unwrap()).0;
        let b1 = ZOBRIST_KEYS.piece_key(Bishop, White, ChessSquare::from_chars('g', '5').unwrap()).0;
        let a2 = ZOBRIST_KEYS.piece_key(Knight, Black, ChessSquare::from_chars('h', '5').unwrap()).0;
        let b2 = ZOBRIST_KEYS.piece_key(Knight, Black, ChessSquare::from_chars('g', '4').unwrap()).0;
        assert_ne!(a1 ^ a2, b1 ^ b2); // used to be bugged
        let position = Chessboard::from_name("kiwipete").unwrap();
        let hash = position.hash_pos();
        let mut hashes = HashMap::new();
        let mut collisions = HashMap::new();
        for mov in position.legal_moves_slow() {
            let new_board = position.make_move(mov).unwrap();
            assert_ne!(new_board.hash_pos(), hash);
            let previous = hashes.insert(new_board.hash_pos().0, new_board);
            assert!(previous.is_none());
            let different_bits = (new_board.hash_pos().0 ^ hash.0).count_ones();
            assert!((16..=48).contains(&different_bits));
            for mov in new_board.legal_moves_slow() {
                let new_board = new_board.make_move(mov).unwrap();
                let previous = hashes.insert(new_board.hash_pos().0, new_board);
                if let Some(old_board) = previous {
                    println!(
                        "Collision at hash {hash}, boards {0} and {1} (diagrams: \n{old_board} and \n{new_board}",
                        old_board.as_fen(),
                        new_board.as_fen()
                    );
                    // There's one ep move after one ply from the current position, which creates the only transposition reachable within 2 plies
                    if old_board != new_board {
                        _ = collisions.insert(new_board.hash_pos().0, [old_board, new_board]);
                    }
                }
                let different_bits = (new_board.hash_pos().0 ^ hash.0).count_ones();
                assert!((12..52).contains(&different_bits));
            }
        }
        assert!(collisions.is_empty(), "num collisions: {0} out of {1}", collisions.len(), hashes.len());
    }

    #[test]
    fn ep_test() {
        let position = Chessboard::from_fen("4r1k1/p4pp1/6bp/2p5/r2p4/P4PPP/1P2P3/2RRB1K1 w - - 1 15", Strict).unwrap();
        assert_eq!(position.hashes, position.compute_zobrist());
        let mov = ChessMove::new(
            ChessSquare::from_rank_file(1, E_FILE_NUM),
            ChessSquare::from_rank_file(3, E_FILE_NUM),
            ChessMoveFlags::NormalMove,
        );
        let new_pos = position.make_move(mov).unwrap();
        assert_eq!(new_pos.hashes, new_pos.compute_zobrist());
        let ep_move = ChessMove::new(
            ChessSquare::from_rank_file(3, D_FILE_NUM),
            ChessSquare::from_rank_file(2, E_FILE_NUM),
            ChessMoveFlags::EnPassant,
        );
        let after_ep = new_pos.make_move(ep_move).unwrap();
        assert_eq!(after_ep.hashes, after_ep.compute_zobrist());
    }

    #[test]
    fn zobrist_after_move_test() {
        for pos in Chessboard::bench_positions() {
            for m in pos.pseudolegal_moves() {
                let Some(new_pos) = pos.make_move(m) else {
                    continue;
                };
                assert!(new_pos.debug_verify_invariants(Strict).is_ok(), "{pos} {}", m.compact_formatter(&pos));
                if !(m.is_double_pawn_push(&pos)
                    || m.is_capture(&pos)
                    || m.is_promotion()
                    || pos.ep_square().is_some()
                    || pos.castling != new_pos.castling)
                {
                    assert_eq!(
                        pos.hash_pos()
                            ^ Chessboard::zobrist_delta(
                                pos.active,
                                m.piece_type(&pos),
                                m.src_square(),
                                m.dest_square()
                            )
                            ^ ZOBRIST_KEYS.side_to_move_key,
                        new_pos.hash_pos(),
                        "{pos} {}",
                        m.compact_formatter(&pos)
                    );
                }
            }
        }
    }
}
