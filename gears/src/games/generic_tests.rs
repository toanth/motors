//! This module contains generic test functions that are completely independent of the actual game.
//! Since those generics aren't instantiated here, there are no actual tests here.
use crate::games::{Color, NoHistory, ZobristHash};
use crate::general::board::Board;
use crate::general::board::SelfChecks::Assertion;
use crate::general::moves::Legality::Legal;
use crate::general::moves::Move;
use itertools::Itertools;
use std::collections::{HashSet, VecDeque};
use std::marker::PhantomData;

pub struct GenericTests<B: Board> {
    _phantom: PhantomData<B>,
}

impl<B: Board> GenericTests<B> {
    pub fn long_notation_roundtrip_test() {
        let positions = B::name_to_pos_map();
        for pos in positions {
            let pos = (pos.val)();
            for mov in pos.legal_moves_slow() {
                let encoded = mov.to_extended_text(&pos);
                let decoded = B::Move::from_extended_text(&encoded, &pos);
                assert!(decoded.is_ok());
                println!(
                    "{encoded} | {0} | {1}",
                    decoded.clone().unwrap(),
                    pos.as_fen()
                );
                assert_eq!(decoded.unwrap(), mov);
            }
        }
    }

    pub fn fen_roundtrip_test() {
        let positions = B::bench_positions();
        for pos in positions {
            assert_eq!(pos, B::from_fen(&pos.as_fen()).unwrap());
            // FENs might be different after one fen->position->fen roundtrip because the parser can accept more than
            // what's produced as output, but writing a FEN two times should produce the same result.
            assert_eq!(pos.as_fen(), B::from_fen(&pos.as_fen()).unwrap().as_fen());
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
            assert!(pos.verify_position_legal(Assertion).is_ok());
            assert!(pos.match_result_slow(&NoHistory::default()).is_none());
            assert_eq!(B::from_fen(&pos.as_fen()).unwrap(), pos);
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
                let legal = new_pos.verify_position_legal(Assertion);
                assert!(legal.is_ok());
                assert_eq!(new_pos.active_player().other(), pos.active_player());
                assert_ne!(new_pos.as_fen(), pos.as_fen());
                assert_eq!(B::from_fen(&new_pos.as_fen()).unwrap(), new_pos);
                assert_ne!(new_pos.zobrist_hash().0, hash); // Even for null moves, the side to move has changed
                assert_eq!(new_pos.halfmove_ctr_since_start() - ply, 1);
                assert!(!hashes.contains(&new_pos.zobrist_hash().0));
                hashes.insert(new_pos.zobrist_hash().0);
            }
        }
    }

    pub fn all_tests() {
        Self::basic_test();
        Self::long_notation_roundtrip_test();
        Self::fen_roundtrip_test();
        Self::statistical_hash_test(B::default());
    }
}
