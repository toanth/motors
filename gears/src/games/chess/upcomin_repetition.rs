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
use crate::games::chess::Chessboard;
use crate::games::chess::moves::{ChessMove, ChessMoveFlags};
use crate::games::chess::pieces::ColoredChessPieceType;
use crate::games::chess::squares::{ChessSquare, ChessboardSize};
use crate::games::chess::zobrist::ZOBRIST_KEYS;
use crate::games::{BoardHistory, ColoredPiece, ColoredPieceType, PosHash, ZobristHistory};
use crate::general::bitboards::chessboard::ChessBitboard;
use crate::general::bitboards::{Bitboard, RawBitboard};
use crate::general::board::{BitboardBoard, Board};
use crate::general::hq::ChessSliderGenerator;
use crate::general::moves::Move;
use std::mem::swap;
use std::sync::LazyLock;

const SIZE: usize = 0x2000;
const MASK: usize = SIZE - 1;

fn h1(hash: PosHash) -> usize {
    (hash.0 as usize >> 32) & MASK
}

fn h2(hash: PosHash) -> usize {
    (hash.0 as usize >> 48) & MASK
}

fn entry(hash: PosHash) -> u32 {
    hash.0 as u32
}

/// Based on <https://web.archive.org/web/20201107002606/https://marcelk.net/2013-04-06/paper/upcoming-rep-v2.pdf>
#[derive(Debug)]
pub struct UpcomingRepetitionTable {
    hashes: [u32; SIZE],
    moves: [ChessMove; SIZE],
}

// It might make sense to try a different hashing scheme than cuckoo hashing, like robin hood hashing.
// That should be more cache efficient, at least.
pub fn calc_move_hash_table() -> UpcomingRepetitionTable {
    let mut hashes = [PosHash::default(); SIZE];
    let mut moves = [ChessMove::default(); SIZE];
    let mut count = 0;
    let slider_gen = ChessSliderGenerator::new(ChessBitboard::default());
    for piece in ColoredChessPieceType::non_pawns() {
        let color = piece.color().unwrap();
        let piece = piece.uncolor();
        for src in ChessSquare::iter() {
            let attacks = Chessboard::threatening_attacks(src, piece, color, &slider_gen);
            for dest in attacks.ones() {
                if dest.bb_idx() < src.bb_idx() {
                    continue;
                }
                let mut mov = ChessMove::new(src, dest, ChessMoveFlags::normal_move(piece));
                let mut hash = ZOBRIST_KEYS.piece_key(piece, color, src)
                    ^ ZOBRIST_KEYS.piece_key(piece, color, dest)
                    ^ ZOBRIST_KEYS.side_to_move_key;
                let mut i = h1(hash);
                loop {
                    swap(&mut hashes[i], &mut hash);
                    swap(&mut moves[i], &mut mov);
                    if mov.is_null() {
                        break;
                    }
                    i = if i == h1(hash) { h2(hash) } else { h1(hash) }
                }
                count += 1;
            }
        }
    }
    let mut res_hashes = [0; SIZE];
    hashes.map(|h| h.0 as u32).swap_with_slice(&mut res_hashes);

    // There are exactly 3668 reversible moves on an empty chessboard
    assert_eq!(count, 3668);
    UpcomingRepetitionTable { hashes: res_hashes, moves }
}

fn has_upcoming_repetition(table: &UpcomingRepetitionTable, history: &ZobristHistory, pos: &Chessboard) -> bool {
    let n = history.len();
    let max_lookback = pos.ply_draw_clock().min(n);
    let mut their_delta = pos.hash_pos() ^ history.0[n - 1] ^ ZOBRIST_KEYS.side_to_move_key;
    for i in (3..=max_lookback).step_by(2) {
        their_delta ^= history.0[n - i + 1] ^ history.0[n - i] ^ ZOBRIST_KEYS.side_to_move_key;
        if their_delta.0 != 0 {
            continue;
        }
        let diff = pos.hash_pos() ^ history.0[n - i];
        let mut idx = h1(diff);
        if table.hashes[idx] != entry(diff) {
            idx = h2(diff);
            if table.hashes[idx] != entry(diff) {
                continue;
            }
        }
        let mut src = table.moves[idx].src_square();
        let mut dest = table.moves[idx].dest_square();

        let ray = ChessBitboard::ray_exclusive(src, dest, ChessboardSize {});
        if (ray & pos.occupied_bb()).has_set_bit() {
            continue;
        };
        if cfg!(debug_assertions) {
            if !pos.active_player_bb().is_bit_set_at(src.bb_idx()) {
                swap(&mut src, &mut dest);
            }
            let mov = ChessMove::new(src, dest, table.moves[idx].flags());
            let piece = mov.piece(pos);
            debug_assert!(pos.col_piece_bb(piece.color().unwrap(), piece.uncolored()).is_bit_set_at(src.bb_idx()));
            debug_assert!(pos.is_empty(dest));
            debug_assert!(pos.is_move_legal(mov));
            debug_assert_eq!(pos.make_move(mov).unwrap().hash_pos(), history.0[n - i]);
        }
        return true;
    }
    false
}

impl Chessboard {
    pub fn has_upcoming_repetition(&self, history: &ZobristHistory) -> bool {
        if self.ply_100_ctr < 3 || history.is_empty() {
            return false;
        }
        has_upcoming_repetition(&UPCOMING_REPETITION_TABLE, history, self)
    }

    // Initializing the upcoming repetition table can take a short while,
    // and in STC tests we don't want to pay for that in the first search call.
    pub fn force_init_upcoming_repetition_table() {
        _ = LazyLock::force(&UPCOMING_REPETITION_TABLE);
    }
}

pub static UPCOMING_REPETITION_TABLE: LazyLock<UpcomingRepetitionTable> = LazyLock::new(calc_move_hash_table);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::n_fold_repetition;
    use crate::general::board::Strictness::Strict;
    use crate::general::board::{Board, BoardHelpers};

    #[test]
    fn test_calc_move_hash_table() {
        let table = calc_move_hash_table();
        let pos = Chessboard::default();
        let mov = ChessMove::from_text("Nf3", &pos).unwrap();
        let new_pos = pos.make_move(mov).unwrap();
        let hash_diff = pos.hash_pos() ^ new_pos.hash_pos();
        assert!(table.moves.contains(&mov));
        assert!(table.hashes.contains(&entry(hash_diff)));
    }

    #[test]
    fn test_upcoming_repetition() {
        let mut pos = Chessboard::from_name("kiwipete").unwrap();
        let moves = ["Qg3", "Bb7", "Qf3"];
        let mut hist = ZobristHistory::default();
        for m in moves {
            assert!(!pos.has_upcoming_repetition(&hist));
            hist.push(pos.hash_pos());
            pos = pos.make_move_from_str(m).unwrap();
        }
        assert!(pos.has_upcoming_repetition(&hist));
        hist.push(pos.hash_pos());
        pos = pos.make_move_from_str("Ba6").unwrap();
        assert!(pos.match_result_slow(&hist).is_none());
        assert!(pos.has_upcoming_repetition(&hist));
        hist.push(pos.hash_pos());
        pos = pos.make_nullmove().unwrap();
        assert!(!pos.has_upcoming_repetition(&hist));
        hist.push(pos.hash_pos());
        pos = pos.make_move_from_str("Bb5").unwrap();
        assert!(!pos.has_upcoming_repetition(&hist));
        hist.push(pos.hash_pos());
        pos = pos.make_nullmove().unwrap();
        pos.ply_100_ctr = 42;
        assert!(pos.has_upcoming_repetition(&hist));
        hist.push(pos.hash_pos());
        pos = pos.make_move_from_str("Ba6").unwrap();
        assert!(hist.0.contains(&pos.hash_pos()));
        assert!(pos.has_upcoming_repetition(&hist));
        hist.push(pos.hash_pos());
        pos = pos.make_nullmove().unwrap();
        assert!(!pos.has_upcoming_repetition(&hist));
    }

    #[test]
    fn triangulation_test() {
        let mut pos = Chessboard::from_fen("8/1p1k4/1P6/2PK4/8/8/8/8 w - - 4 7", Strict).unwrap();
        let moves = ["Ke5!", "Kc6", "Kd4", "Kd7"];
        let mut hist = ZobristHistory::default();
        for m in moves {
            assert!(!n_fold_repetition(2, &hist, pos.hash_pos(), 100));
            assert!(!pos.has_upcoming_repetition(&hist), "{pos}");
            hist.push(pos.hash_pos());
            pos = pos.make_move_from_str(m).unwrap();
        }
        assert!(pos.has_upcoming_repetition(&hist)); // Ke5 repeats
        hist.push(pos.hash_pos());
        pos = pos.make_move_from_str("Kd5").unwrap();
        assert!(!n_fold_repetition(2, &hist, pos.hash_pos(), 100));
        assert!(!pos.has_upcoming_repetition(&hist));
    }
}
