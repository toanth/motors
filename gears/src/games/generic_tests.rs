//! This module contains generic test functions that are completely independent of the actual game.
//! Since those generics aren't instantiated here, there are no actual tests here.
use crate::games::{Color, ColoredPiece, Coordinates, Size, ZobristHash};
use crate::general::board::Strictness::Strict;
use crate::general::board::{Board, BoardHelpers, UnverifiedBoard};
use crate::general::moves::ExtendedFormat::{Alternative, Standard};
use crate::general::moves::Legality::Legal;
use crate::general::moves::Move;
use itertools::Itertools;
use std::collections::{HashSet, VecDeque};
use std::marker::PhantomData;

pub struct GenericTests<B: Board> {
    _phantom: PhantomData<B>,
}

impl<B: Board> GenericTests<B> {
    pub fn coordinates_test() {
        let pos = B::default();
        let size = pos.size();
        assert_eq!(size.valid_coordinates().count(), size.num_squares());
        let coords = size.valid_coordinates();
        let mut found_center = false;
        let mut p = B::Unverified::new(pos);
        for coords in coords {
            assert!(size.coordinates_valid(coords));
            assert!(size.check_coordinates(coords).is_ok());
            assert_ne!(coords, B::Coordinates::no_coordinates());
            assert_eq!(
                size.to_coordinates_unchecked(size.internal_key(coords)),
                coords
            );
            let flipped = coords.flip_up_down(size);
            assert_eq!(flipped.flip_up_down(size), coords);
            let flipped = coords.flip_left_right(size);
            assert_eq!(flipped.flip_left_right(size), coords);
            if coords == flipped.flip_up_down(size) {
                assert!(!found_center);
                found_center = true;
            }
            assert_eq!(
                pos.is_empty(coords),
                pos.colored_piece_on(coords).color().is_none()
            );
            p = p.remove_piece(coords).unwrap();
        }
        assert_eq!(
            p.verify(Strict).map_err(|_| ()),
            B::empty().into().verify(Strict).map_err(|_| ())
        );
        assert!(size
            .check_coordinates(Coordinates::no_coordinates())
            .is_err());
    }

    pub fn long_notation_roundtrip_test() {
        let positions = B::name_to_pos_map();
        for pos in positions {
            let pos = (pos.val)();
            for mov in pos.legal_moves_slow() {
                for format in [Standard, Alternative] {
                    let encoded = mov.to_extended_text(&pos, format);
                    let decoded = B::Move::from_extended_text(&encoded, &pos);
                    assert!(decoded.is_ok());
                    assert_eq!(decoded.unwrap(), mov);
                }
            }
        }
    }

    pub fn fen_roundtrip_test() {
        let positions = B::bench_positions();
        for pos in positions {
            assert_eq!(pos, B::from_fen(&pos.as_fen(), Strict).unwrap());
            // FENs might be different after one fen->position->fen roundtrip because the parser can accept more than
            // what's produced as output, but writing a FEN two times should produce the same result.
            assert_eq!(
                pos.as_fen(),
                B::from_fen(&pos.as_fen(), Strict).unwrap().as_fen()
            );
        }
    }

    pub fn statistical_hash_test(position: B) {
        let mut hashes = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(position);
        let max_queue_len = if cfg!(debug_assertions) {
            500_000
        } else {
            5_000_000
        };
        while queue.len() <= max_queue_len && !queue.is_empty() {
            assert!(!queue.is_empty());
            let pos = queue.front().copied().unwrap();
            let moves = pos.legal_moves_slow();
            queue.pop_front();
            hashes.push(pos.zobrist_hash());
            for mov in moves {
                queue.push_back(pos.make_move(mov).unwrap());
            }
        }
        for entry in queue {
            hashes.push(entry.zobrist_hash());
        }
        hashes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        hashes = hashes.iter().dedup().copied().collect_vec();
        let num_hashes = hashes.len();
        assert!(num_hashes >= 1_000);
        for shift in 0..64 - 8 {
            let get_bits = |hash: ZobristHash| (hash.0 >> shift) & 0xff;
            let mut counts = vec![0; 256];
            for hash in &hashes {
                counts[get_bits(*hash) as usize] += 1;
            }
            let expected = hashes.len() / 256;
            let radius = expected.min(expected / 2 + 500_000_000 / (num_hashes * num_hashes));
            for count in counts {
                assert!(count >= expected - radius, "{count} {expected} {shift}");
                assert!(count <= expected + radius);
            }
        }
    }

    fn basic_test() {
        assert!(!B::bench_positions().is_empty());
        for pos in B::bench_positions() {
            let ply = pos.halfmove_ctr_since_start();
            // use a new hash set per position because bench positions can be only one ply away from each other
            let mut hashes = HashSet::new();
            let _ = pos.debug_verify_invariants(Strict).unwrap();
            assert!(pos.debug_verify_invariants(Strict).is_ok());
            assert_eq!(
                B::from_fen(&pos.as_fen(), Strict).unwrap(),
                pos,
                "{:?}\n{}",
                pos,
                pos.as_fen()
            );
            let hash = pos.zobrist_hash().0;
            hashes.insert(hash);
            assert_ne!(hash, 0);
            if B::Move::legality() == Legal {
                assert_eq!(
                    pos.legal_moves_slow().into_iter().count(),
                    pos.pseudolegal_moves().into_iter().count()
                );
            }
            for mov in pos.legal_moves_slow() {
                assert!(pos.is_move_legal(mov));
            }
            for mov in pos.pseudolegal_moves() {
                assert!(pos.is_move_pseudolegal(mov));
                let new_pos = pos.make_move(mov);
                assert_eq!(new_pos.is_some(), pos.is_pseudolegal_move_legal(mov));
                let Some(new_pos) = new_pos else { continue };
                let legal = new_pos.debug_verify_invariants(Strict);
                assert!(legal.is_ok());
                assert_eq!(new_pos.active_player().other(), pos.active_player());
                assert_ne!(new_pos.as_fen(), pos.as_fen());

                let roundtrip = B::from_fen(&new_pos.as_fen(), Strict).unwrap();
                assert!(roundtrip.debug_verify_invariants(Strict).is_ok());
                assert_eq!(roundtrip.as_fen(), new_pos.as_fen());
                assert_eq!(
                    roundtrip.legal_moves_slow().into_iter().collect_vec(),
                    new_pos.legal_moves_slow().into_iter().collect_vec()
                );
                assert_eq!(roundtrip, new_pos);
                assert_eq!(roundtrip.zobrist_hash(), new_pos.zobrist_hash());

                assert_ne!(new_pos.zobrist_hash().0, hash); // Even for null moves, the side to move has changed
                assert_eq!(new_pos.halfmove_ctr_since_start() - ply, 1);
                assert!(!hashes.contains(&new_pos.zobrist_hash().0));
                hashes.insert(new_pos.zobrist_hash().0);
            }
        }
    }

    pub fn all_tests() {
        Self::basic_test();
        Self::coordinates_test();
        Self::long_notation_roundtrip_test();
        Self::fen_roundtrip_test();
        Self::statistical_hash_test(B::default());
    }
}
