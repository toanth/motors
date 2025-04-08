//! This module contains generic test functions that are completely independent of the actual game.
//! Since those generics aren't instantiated here, there are no actual tests here.
use crate::games::{Color, ColoredPiece, Coordinates, PosHash, Size};
use crate::general::board::Strictness::{Relaxed, Strict};
use crate::general::board::{Board, BoardHelpers, UnverifiedBoard};
use crate::general::moves::ExtendedFormat::{Alternative, Standard};
use crate::general::moves::Legality::Legal;
use crate::general::moves::Move;
use itertools::Itertools;
use num::ToPrimitive;
use proptest::proptest;
use rand::SeedableRng;
use rand::rngs::StdRng;
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
        let mut p = B::Unverified::new(pos.clone());
        for coords in coords {
            assert!(size.coordinates_valid(coords));
            assert!(size.check_coordinates(coords).is_ok());
            assert_eq!(size.to_coordinates_unchecked(size.internal_key(coords)), coords);
            let flipped = coords.flip_up_down(size);
            assert_eq!(flipped.flip_up_down(size), coords);
            let flipped = coords.flip_left_right(size);
            assert_eq!(flipped.flip_left_right(size), coords);
            if coords == flipped.flip_up_down(size) {
                assert!(!found_center);
                found_center = true;
            }
            assert_eq!(pos.is_empty(coords), pos.colored_piece_on(coords).color().is_none());
            p.try_remove_piece(coords).unwrap();
        }
        assert_eq!(p.verify(Strict).map_err(|_| ()), B::empty().into().verify(Strict).map_err(|_| ()));
    }

    pub fn long_notation_roundtrip_test() {
        let positions = B::name_to_pos_map();
        for pos in positions {
            let pos = pos.create::<B>();
            for mov in pos.legal_moves_slow() {
                for format in [Standard, Alternative] {
                    println!("{} {pos}", mov.compact_formatter(&pos));
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
            assert_eq!(pos.as_fen(), B::from_fen(&pos.as_fen(), Strict).unwrap().as_fen());
        }
    }

    pub fn statistical_hash_test(position: B) {
        let mut hashes = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(position);
        let max_queue_len = if cfg!(debug_assertions) { 500_000 } else { 5_000_000 };
        while queue.len() <= max_queue_len && !queue.is_empty() {
            assert!(!queue.is_empty());
            let pos = queue.front().cloned().unwrap();
            let moves = pos.legal_moves_slow();
            _ = queue.pop_front();
            hashes.push(pos.hash_pos());
            for mov in moves {
                queue.push_back(pos.clone().make_move(mov).unwrap());
            }
        }
        for entry in queue {
            hashes.push(entry.hash_pos());
        }
        hashes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        hashes = hashes.iter().dedup().copied().collect_vec();
        let num_hashes = hashes.len();
        assert!(num_hashes >= 1_000);
        for shift in 0..64 - 8 {
            let get_bits = |hash: PosHash| (hash.0 >> shift) & 0xff;
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
            let pos = pos.debug_verify_invariants(Strict).unwrap();
            assert_eq!(B::from_fen(&pos.as_fen(), Strict).unwrap(), pos, "{:?}\n{}", pos, pos.as_fen());
            let hash = pos.hash_pos().0;
            _ = hashes.insert(hash);
            assert_ne!(hash, 0);
            if pos.cannot_call_movegen() {
                // in some bench positions, the game is already won
                continue;
            }
            if B::Move::legality() == Legal {
                assert_eq!(pos.legal_moves_slow().into_iter().count(), pos.pseudolegal_moves().into_iter().count());
            }
            for mov in pos.legal_moves_slow() {
                assert!(pos.is_move_legal(mov));
            }
            for mov in pos.pseudolegal_moves() {
                assert!(pos.is_move_pseudolegal(mov));
                let new_pos = pos.clone().make_move(mov);
                assert_eq!(new_pos.is_some(), pos.is_pseudolegal_move_legal(mov));
                let Some(new_pos) = new_pos else { continue };
                let new_pos = new_pos.debug_verify_invariants(Strict).unwrap();
                assert_eq!(new_pos.active_player().other(), pos.active_player());
                assert_ne!(new_pos.as_fen(), pos.as_fen());

                let roundtrip = B::from_fen(&new_pos.as_fen(), Strict).unwrap();
                let roundtrip = roundtrip.debug_verify_invariants(Strict).unwrap();
                assert_eq!(roundtrip.as_fen(), new_pos.as_fen());
                if !new_pos.cannot_call_movegen() {
                    assert_eq!(
                        roundtrip.legal_moves_slow().into_iter().collect_vec(),
                        new_pos.legal_moves_slow().into_iter().collect_vec()
                    );
                }
                assert_eq!(roundtrip, new_pos);
                assert_eq!(roundtrip.hash_pos(), new_pos.hash_pos());

                assert_ne!(new_pos.hash_pos().0, hash); // Even for null moves, the side to move has changed
                assert_eq!(new_pos.halfmove_ctr_since_start() - ply, 1);
                assert!(!hashes.contains(&new_pos.hash_pos().0));
                _ = hashes.insert(new_pos.hash_pos().0);
            }
        }
    }

    fn unverified_tests() {
        let mut rng = StdRng::seed_from_u64(123);
        for pos in B::bench_positions() {
            let ply = pos.halfmove_ctr_since_start();
            if let Ok(p2) = pos
                .clone()
                .set_ply_since_start(ply.wrapping_add(1))
                .and_then(|p| p.verify(Relaxed))
                .or_else(|_| pos.clone().set_ply_since_start(ply.wrapping_sub(1)).and_then(|p| p.verify(Relaxed)))
            {
                assert_ne!(pos, p2);
                assert_eq!(pos.hash_pos(), p2.hash_pos());
                if let Ok(p2) = p2.set_active_player(pos.active_player().other()).verify(Relaxed) {
                    assert_ne!(pos, p2);
                    assert_ne!(pos.hash_pos(), p2.hash_pos());
                }
            }
            if let Ok(p2) = pos.clone().set_active_player(pos.active_player().other()).verify(Relaxed) {
                assert_ne!(pos, p2);
                assert_ne!(pos.hash_pos(), p2.hash_pos());
            }
            if pos.cannot_call_movegen() {
                continue;
            }
            if let Some(m) = pos.random_legal_move(&mut rng) {
                let p2 = pos.clone().make_move(m).unwrap();
                assert_ne!(p2, pos);
                assert_ne!(p2.hash_pos(), pos.hash_pos());
            }
        }
    }

    pub fn all_tests() {
        Self::basic_test();
        Self::coordinates_test();
        Self::long_notation_roundtrip_test();
        Self::fen_roundtrip_test();
        Self::statistical_hash_test(B::default());
        Self::unverified_tests();
        Self::move_test();
    }

    fn move_test() {
        let num_bits = size_of::<<B::Move as Move<B>>::Underlying>() * 8;
        let max_val = 1 << num_bits.min(32);
        for pos in B::bench_positions() {
            proptest!(|(pattern in 0..max_val)| {
                let p_u64 = pattern as u64;
                let m = B::Move::from_u64_unchecked(p_u64);
                assert_eq!(m.clone().to_underlying().to_u64().unwrap(), p_u64);
                if let Some(m) = m.clone().check_pseudolegal(&pos) {
                    let new_pos = pos.clone().make_move(m);
                    if pos.is_pseudolegal_move_legal(m) {
                        assert!(new_pos.is_some());
                    } else {
                        assert!(new_pos.is_none());
                    }
                }
                // even invalid moves must be able to be printed in this format
                let _text = m.trust_unchecked().compact_formatter(&pos).to_string();
            })
        }
    }
}
