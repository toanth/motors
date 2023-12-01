use strum::IntoEnumIterator;

use crate::games::chess::pieces::UncoloredChessPiece;
use crate::games::chess::squares::NUM_COLUMNS;
use crate::games::chess::{CastleRight, Chessboard};
use crate::games::Color::*;
use crate::games::{AbstractPieceType, Color, ZobristHash};
use crate::general::bitboards::Bitboard;

pub const NUM_PIECE_SQUARE_ENTRIES: usize = 64 * 6;
pub const NUM_COLORED_PIECE_SQUARE_ENTRIES: usize = NUM_PIECE_SQUARE_ENTRIES * 2;

struct PrecomputedZobristKeys {
    piece_square_keys: [ZobristHash; NUM_COLORED_PIECE_SQUARE_ENTRIES],
    castle_keys: [ZobristHash; 2 * 2],
    ep_file_keys: [ZobristHash; NUM_COLUMNS],
    side_to_move_key: ZobristHash,
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
const PRECOMPUTED_ZOBRIST_KEYS: PrecomputedZobristKeys = {
    let mut res = {
        PrecomputedZobristKeys {
            piece_square_keys: [ZobristHash(0); NUM_COLORED_PIECE_SQUARE_ENTRIES],
            castle_keys: [ZobristHash(0); 2 * 2],
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
    while i < 2 * 2 {
        (gen, res.castle_keys[i]) = gen.gen();
        i += 1;
    }
    let mut i = 0;
    while i < NUM_COLUMNS {
        (gen, res.ep_file_keys[i]) = gen.gen();
        i += 1;
    }
    (gen, res.side_to_move_key) = gen.gen();
    res
};

impl Chessboard {
    pub(super) fn compute_zobrist(&self) -> ZobristHash {
        let mut res = ZobristHash(0);
        for color in Color::iter() {
            for piece in UncoloredChessPiece::pieces() {
                let mut pieces = self.colored_piece_bb(color, piece);
                while pieces.has_set_bit() {
                    let idx = pieces.pop_lsb();
                    res ^= PRECOMPUTED_ZOBRIST_KEYS.piece_square_keys
                        [idx * 12 + piece.to_uncolored_idx() * 2 + color as usize];
                }
            }
            for right in CastleRight::iter() {
                if self.flags.can_castle(color, right) {
                    res ^=
                        PRECOMPUTED_ZOBRIST_KEYS.castle_keys[right as usize * 2 + color as usize];
                }
            }
        }
        res ^= self.ep_square.map_or(ZobristHash(0), |square| {
            PRECOMPUTED_ZOBRIST_KEYS.ep_file_keys[square.file()]
        });
        if self.active_player == White {
            res ^= PRECOMPUTED_ZOBRIST_KEYS.side_to_move_key;
        }
        res
    }

    // TODO: Implement incremental zobrist key updates
}
