use derive_more::BitOrAssign;
use lazy_static::lazy_static;
use rand::rngs::StdRng;
use rand::RngCore;
use rand::SeedableRng;
use strum::IntoEnumIterator;

use crate::games::chess::pieces::UncoloredChessPiece;
use crate::games::chess::squares::NUM_COLUMNS;
use crate::games::chess::{CastleRight, Chessboard};
use crate::games::Color::*;
use crate::games::{AbstractPieceType, Color};
use crate::general::bitboards::Bitboard;

#[derive(Copy, Clone, Eq, PartialEq, Default, derive_more::Display, Debug, BitOrAssign)]
pub struct ZobristHash(u64);

pub const NUM_PIECE_SQUARE_ENTRIES: usize = 64 * 6;
pub const NUM_COLORED_PIECE_SQUARE_ENTRIES: usize = NUM_PIECE_SQUARE_ENTRIES * 2;

struct PrecomputedZobristKeys {
    piece_square_keys: [ZobristHash; NUM_COLORED_PIECE_SQUARE_ENTRIES],
    castle_keys: [ZobristHash; 2 * 2],
    ep_file_keys: [ZobristHash; NUM_COLUMNS],
    side_to_move_key: ZobristHash,
}

// TODO: Implement a normal static solution (there's probably a crate for const rngs) and benchmark;
// this solution should require accessing an atomic variable for each read so it may be slow
lazy_static! {
    static ref PRECOMPUTED_ZOBRIST_KEYS: PrecomputedZobristKeys = {
        let mut res = {
            PrecomputedZobristKeys {
                piece_square_keys: [ZobristHash(0); NUM_COLORED_PIECE_SQUARE_ENTRIES],
                castle_keys: [ZobristHash(0); 2 * 2],
                ep_file_keys: [ZobristHash(0); NUM_COLUMNS],
                side_to_move_key: Default::default(),
            }
        };
        let mut rng = StdRng::seed_from_u64(42);
        for key in res.piece_square_keys.iter_mut() {
            *key = ZobristHash(rng.next_u64());
        }
        for key in res.castle_keys.iter_mut() {
            *key = ZobristHash(rng.next_u64());
        }
        for key in res.ep_file_keys.iter_mut() {
            *key = ZobristHash(rng.next_u64());
        }
        res.side_to_move_key = ZobristHash(rng.next_u64());
        res
    };
}

struct History(Vec<ZobristHash>);

impl Chessboard {
    pub(super) fn compute_zobrist(&self) -> ZobristHash {
        let mut res = ZobristHash(0);
        for color in Color::iter() {
            for piece in UncoloredChessPiece::pieces() {
                let mut pieces = self.colored_piece_bb(color, piece);
                while pieces.has_set_bit() {
                    let idx = pieces.pop_lsb();
                    res |= PRECOMPUTED_ZOBRIST_KEYS.piece_square_keys
                        [idx * 12 + piece.to_uncolored_idx() * 2 + color as usize];
                }
            }
            for right in CastleRight::iter() {
                if self.flags.can_castle(color, right) {
                    res |=
                        PRECOMPUTED_ZOBRIST_KEYS.castle_keys[right as usize * 2 + color as usize];
                }
            }
        }
        res |= self.ep_square.map_or(ZobristHash(0), |square| {
            PRECOMPUTED_ZOBRIST_KEYS.ep_file_keys[square.file()]
        });
        if self.active_player == White {
            res |= PRECOMPUTED_ZOBRIST_KEYS.side_to_move_key;
        }
        res
    }

    // TODO: Implement incremental zobrist key updates
}
