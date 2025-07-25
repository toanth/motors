#[cfg(test)]
mod tests {
    use crate::games::Board;
    use crate::games::chess::Chessboard;
    use crate::games::chess::moves::ChessMove;
    use crate::general::board::Strictness::{Relaxed, Strict};
    use crate::general::board::{BoardHelpers, Strictness};
    use crate::general::common::parse_int_from_str;
    use crate::general::moves::Move;
    use crate::general::perft::perft;
    use crate::search::DepthPly;
    use itertools::Itertools;
    use rand::prelude::SliceRandom;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng, rng};
    use std::io::{Write, stdout};
    use std::num::NonZeroUsize;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread::{available_parallelism, current, scope};
    use std::time::Instant;

    #[test]
    fn kiwipete_test() {
        let board = Chessboard::from_name("kiwipete").unwrap();
        let res = perft(DepthPly::new(4), board, false);
        assert_eq!(res.nodes, 4_085_603);
        // Disabled in debug mode because that would take too long. TODO: Optimize movegen, especially in debug mode.
        if !cfg!(debug_assertions) {
            // kiwipete after white castles (cheaper to run than increasing the depth of kiwipete, and failed perft once)
            let board =
                Chessboard::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R4RK1 b kq - 1 1", Strict)
                    .unwrap();
            let res = perft(DepthPly::new(4), board, true);
            assert_eq!(res.nodes, 4_119_629);
            // kiwipete after white plays a2a3
            let board =
                Chessboard::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/P1N2Q1p/1PPBBPPP/R3K2R b KQkq - 0 1", Strict)
                    .unwrap();
            let res = perft(DepthPly::new(4), board, false);
            assert_eq!(res.nodes, 4_627_439);
        }
    }

    #[test]
    fn leonids_position_test() {
        let board = Chessboard::from_fen("q2k2q1/2nqn2b/1n1P1n1b/2rnr2Q/1NQ1QN1Q/3Q3B/2RQR2B/Q2K2Q1 w - - 0 1", Strict)
            .unwrap();
        let res = perft(DepthPly::new(1), board, true);
        assert_eq!(res.nodes, 99);
        assert!(res.time.as_millis() <= 2);
        let res = perft(DepthPly::new(2), board, true);
        assert_eq!(res.nodes, 6271);
        let res = perft(DepthPly::new(3), board, true);
        assert_eq!(res.nodes, 568_299);
        if cfg!(not(debug_assertions)) {
            let res = perft(DepthPly::new(4), board, false);
            assert_eq!(res.nodes, 34_807_627);
        }
    }

    #[test]
    fn castling_perft_test() {
        // this is not actually a reachable position, but we accept it, so we should handle it
        let fen = "r3k2r/ppp1pp1p/2nqb1Nn/3p4/4P3/2PP4/1PPPNBPP/2NRQK1R w KQkq -";
        assert!(Chessboard::from_fen(fen, Strict).is_err());
        let pos = Chessboard::from_fen(fen, Relaxed).unwrap();
        let expected: &[u64] = &[
            33,
            1328,
            42079,
            1700714,
            #[cfg(not(debug_assertions))]
            53117779,
        ];
        for (depth, perft_num) in expected.iter().enumerate() {
            assert_eq!(perft(DepthPly::new(depth + 1), pos, false).nodes, *perft_num);
        }
    }

    #[test]
    fn no_choice_test() {
        let fen = "5b1k/4p1p1/4P1P1/8/8/1p1p4/1P1P4/K1B5 w - - 0 1";
        let pos = Chessboard::from_fen(fen, Strict).unwrap();
        let res = perft(DepthPly::new(1000), pos, false);
        assert_eq!(res.nodes, 1);
    }

    // ** The following tests are based on tests by kz04px: <https://github.com/kz04px/rawr/blob/master/tests/perft_extra.rs> **

    #[test]
    fn perft_enpassant() {
        let tests = [
            // EP
            ("8/8/8/8/1k1PpN1R/8/8/4K3 b - d3 0 1", vec![1, 9, 193]),
            ("8/8/8/8/1k1Ppn1R/8/8/4K3 b - d3 0 1", vec![1, 17, 220]),
            ("4k3/8/8/2PpP3/8/8/8/4K3 w - d6 0 1", vec![1, 9, 47, 376]),
            ("4k3/8/8/8/2pPp3/8/8/4K3 b - d3 0 1", vec![1, 9, 47, 376]),
            // EP - pinned diagonal
            ("4k3/b7/8/2Pp4/8/8/8/6K1 w - d6 0 1", vec![1, 5, 45]),
            ("4k3/7b/8/4pP2/8/8/8/1K6 w - e6 0 1", vec![1, 5, 45]),
            ("6k1/8/8/8/2pP4/8/B7/3K4 b - d3 0 1", vec![1, 5, 45]),
            ("1k6/8/8/8/4Pp2/8/7B/4K3 b - e3 0 1", vec![1, 5, 45]),
            ("4k3/b7/8/1pP5/8/8/8/6K1 w - b6 0 1", vec![1, 6, 52]),
            ("4k3/7b/8/5Pp1/8/8/8/1K6 w - g6 0 1", vec![1, 6, 51]),
            ("6k1/8/8/8/1Pp5/8/B7/4K3 b - b3 0 1", vec![1, 6, 52]),
            ("1k6/8/8/8/5pP1/8/7B/4K3 b - g3 0 1", vec![1, 6, 51]),
            ("4k3/K7/8/1pP5/8/8/8/6b1 w - b6 0 1", vec![1, 6, 66]),
            ("4k3/7K/8/5Pp1/8/8/8/1b6 w - g6 0 1", vec![1, 6, 60]),
            ("6B1/8/8/8/1Pp5/8/k7/4K3 b - b3 0 1", vec![1, 6, 66]),
            ("1B6/8/8/8/5pP1/8/7k/4K3 b - g3 0 1", vec![1, 6, 60]),
            ("4k3/b7/8/2Pp4/3K4/8/8/8 w - d6 0 1", vec![1, 5, 44]),
            ("4k3/8/1b6/2Pp4/3K4/8/8/8 w - d6 0 1", vec![1, 6, 59]),
            ("4k3/8/b7/1Pp5/2K5/8/8/8 w - c6 0 1", vec![1, 6, 49]),
            ("4k3/8/7b/5pP1/5K2/8/8/8 w - f6 0 1", vec![1, 6, 49]),
            ("4k3/7b/8/4pP2/4K3/8/8/8 w - e6 0 1", vec![1, 5, 44]),
            ("4k3/8/6b1/4pP2/4K3/8/8/8 w - e6 0 1", vec![1, 6, 53]),
            ("4k3/8/3K4/1pP5/8/q7/8/8 w - b6 0 1", vec![1, 5, 114]),
            ("7k/4K3/8/1pP5/8/q7/8/8 w - b6 0 1", vec![1, 8, 171]),
            // EP - double check
            ("4k3/2rn4/8/2K1pP2/8/8/8/8 w - e6 0 1", vec![1, 4, 75]),
            // EP - pinned horizontal
            ("4k3/8/8/K2pP2r/8/8/8/8 w - d6 0 1", vec![1, 6, 94]),
            ("4k3/8/8/K2pP2q/8/8/8/8 w - d6 0 1", vec![1, 6, 130]),
            ("4k3/8/8/r2pP2K/8/8/8/8 w - d6 0 1", vec![1, 6, 87]),
            ("4k3/8/8/q2pP2K/8/8/8/8 w - d6 0 1", vec![1, 6, 129]),
            ("8/8/8/8/1k1Pp2R/8/8/4K3 b - d3 0 1", vec![1, 8, 125]),
            ("8/8/8/8/1R1Pp2k/8/8/4K3 b - d3 0 1", vec![1, 6, 87]),
            // EP - pinned vertical
            ("k7/8/4r3/3pP3/8/8/8/4K3 w - d6 0 1", vec![1, 5, 70]),
            ("k3K3/8/8/3pP3/8/8/8/4r3 w - d6 0 1", vec![1, 6, 91]),
            // EP - in check
            ("4k3/8/8/4pP2/3K4/8/8/8 w - e6 0 1", vec![1, 9, 49]),
            ("8/8/8/4k3/5Pp1/8/8/3K4 b - f3 0 1", vec![1, 9, 50]),
            // EP - block check
            ("4k3/8/K6r/3pP3/8/8/8/8 w - d6 0 1", vec![1, 6, 109]),
            ("4k3/8/K6q/3pP3/8/8/8/8 w - d6 0 1", vec![1, 6, 151]),
        ];

        for (fen, results) in tests {
            let pos = Chessboard::from_fen(fen, Relaxed).unwrap();
            for (idx, &expected) in results.iter().enumerate() {
                let result = perft(DepthPly::new(idx), pos, false);
                assert_eq!(result.nodes, expected, "depth {idx}: {fen}");
            }
        }
    }

    #[test]
    fn perft_double_checked() {
        let tests = [
            ("4k3/8/4r3/8/8/8/3p4/4K3 w - - 0 1", [1, 4, 80, 320]),
            ("4k3/8/4q3/8/8/8/3b4/4K3 w - - 0 1", [1, 4, 143, 496]),
        ];

        for (fen, results) in tests {
            let pos = Chessboard::from_fen(fen, Relaxed).unwrap();
            for (idx, &expected) in results.iter().enumerate() {
                let result = perft(DepthPly::new(idx), pos, false);
                assert_eq!(result.nodes, expected, "depth {idx}: {fen}");
            }
        }
    }

    #[test]
    fn perft_pins() {
        let tests = [
            ("4k3/8/8/8/1b5b/8/3Q4/4K3 w - - 0 1", [1, 3, 54, 1256, 20328]),
            ("4k3/8/8/8/1b5b/8/3R4/4K3 w - - 0 1", [1, 3, 54, 836, 14835]),
            ("4k3/8/8/8/1b5b/2Q5/5P2/4K3 w - - 0 1", [1, 6, 98, 2274, 34581]),
            ("4k3/8/8/8/1b5b/2R5/5P2/4K3 w - - 0 1", [1, 4, 72, 1300, 23118]),
            ("4k3/8/8/8/1b2r3/8/3Q4/4K3 w - - 0 1", [1, 3, 66, 1390, 29093]),
            ("4k3/8/8/8/1b2r3/8/3QP3/4K3 w - - 0 1", [1, 6, 119, 2074, 40736]),
            // Additional position with some rather tricky cases
            ("1k6/8/1q3b2/2R1P3/r1RK1B1r/8/8/8 w - - 0 1", [1, 8, 276, 4995, 180603]),
            // castling out of a pin
            ("3k4/4r3/8/8/8/4B3/8/R3K3 w Q - 0 1", [1, 16, 214, 4814, 72521]),
            // even more weird pins
            ("3q3r/7R/3r3Q/B2R2rk/1b3B1p/2B1Pp2/1qNKBR2/4B3 b - - 21 42", [1, 2, 84, 3342, 130469]),
        ];

        for (fen, results) in tests {
            let pos = Chessboard::from_fen(fen, Relaxed).unwrap();
            for (idx, &expected) in results.iter().enumerate() {
                let result = perft(DepthPly::new(idx), pos, false);
                assert_eq!(result.nodes, expected, "depth {idx}: {fen}");
            }
        }
    }

    #[test]
    fn perft_dfrc() {
        let tests = [
            ("2r1kr2/8/8/8/8/8/8/1R2K1R1 w GBfc - 0 1", [1, 22, 501, 11459]),
            ("rkr5/8/8/8/8/8/8/5RKR w HFca - 0 1", [1, 22, 442, 10217]),
            ("2r3kr/8/8/8/8/8/8/2KRR3 w h - 3 2", [1, 3, 72, 1371]),
            ("5rkr/8/8/8/8/8/8/RKR5 w CAhf - 0 1", [1, 22, 442, 10206]),
            ("3rkr2/8/8/8/8/8/8/R3K2R w HAfd - 0 1", [1, 20, 452, 9873]),
            ("4k3/8/8/8/8/8/8/4KR2 w F - 0 1", [1, 14, 47, 781]),
            ("4kr2/8/8/8/8/8/8/4K3 w f - 0 1", [1, 3, 42, 246]),
            ("4k3/8/8/8/8/8/8/2R1K3 w C - 0 1", [1, 16, 71, 1277]),
            ("2r1k3/8/8/8/8/8/8/4K3 w c - 0 1", [1, 5, 80, 448]),
        ];
        let pos = Chessboard::from_fen("1r4kr/8/8/8/8/8/2R5/RK6 w Ah - 2 2", Strict).unwrap();
        let mov = ChessMove::from_text("0-0-0", &pos).unwrap();
        assert!(pos.is_generated_move_pseudolegal(mov));
        assert!(!pos.is_pseudolegal_move_legal(mov));

        for (fen, results) in tests {
            let pos = Chessboard::from_fen(fen, Relaxed).unwrap();
            for (idx, &expected) in results.iter().enumerate() {
                let result = perft(DepthPly::new(idx), pos, false);
                assert_eq!(result.nodes, expected, "depth {idx}: {fen}");
            }
        }
    }

    // ** The following test positions are mostly form well-known perft suites, with a couple of custom additions

    struct ExpectedPerftRes {
        fen: &'static str,
        res: Vec<u64>,
    }

    const INVALID: u64 = u64::MAX;

    impl ExpectedPerftRes {
        fn new(input: &'static str) -> ExpectedPerftRes {
            let mut parts = input.split(';');
            let fen = parts.next().unwrap();
            let mut res = vec![INVALID; 8];
            for r in parts {
                let mut words = r.split_whitespace();
                let depth = words.next().unwrap();
                let depth = depth.strip_prefix("D").unwrap();
                let depth = parse_int_from_str::<usize>(depth, "perft depth").unwrap();
                assert!(depth <= 7);
                let node_count = words.next().unwrap();
                let node_count = parse_int_from_str(node_count, "perft node count").unwrap();
                res[depth] = node_count;
            }
            ExpectedPerftRes { fen, res }
        }
    }

    #[test]
    #[ignore]
    fn standard_perft_test() {
        perft_test(STANDARD_FENS, Strict);
    }

    #[test]
    #[ignore]
    fn chess960_perft_test() {
        perft_test(&CHESS_960_FENS, Strict);
    }

    #[test]
    #[ignore]
    fn custom_perft_test() {
        perft_test(CUSTOM_FENS, Relaxed);
    }

    #[test]
    /// Only meant to make sure DFRC works assuming Chess960 and normal chess movegen already works.
    fn dfrc_perft_test() {
        #[cfg(debug_assertions)]
        const FENS: [&str; 2] = [
            "r1q1k1rn/1p1ppp1p/1npb2b1/p1N3p1/8/1BP4P/PP1PPPP1/1RQ1KRBN w BFag - 0 9 ;D1 32 ;D2 1093 ;D3 34210 ;D4 1187103", // ;D5 37188628",// ;D6 1308319545",
            "rk3r2/8/8/5r2/6R1/8/8/R3K1R1 w AGaf - 0 1 ;D1 31 ;D2 841 ;D3 23877 ;D4 711547", // ;D5 20894205",// ;D6 644033568"
        ];
        #[cfg(not(debug_assertions))]
        const FENS: [&str; 2] = [
            "r1q1k1rn/1p1ppp1p/1npb2b1/p1N3p1/8/1BP4P/PP1PPPP1/1RQ1KRBN w BFag - 0 9 ;D1 32 ;D2 1093 ;D3 34210 ;D4 1187103 ;D5 37188628", // ;D6 1308319545",
            "rk3r2/8/8/5r2/6R1/8/8/R3K1R1 w AGaf - 0 1 ;D1 31 ;D2 841 ;D3 23877 ;D4 711547 ;D5 20894205", // ;D6 644033568"
        ];

        perft_test(&FENS, Strict);
    }

    /// Parallelizes the perft testcases so that this takes less time, but the chess960 suite still takes a very long time.
    fn perft_test(fens: &'static [&'static str], strictness: Strictness) {
        let start_time = Instant::now();
        let num_threads = available_parallelism().unwrap_or(NonZeroUsize::new(1).unwrap());
        println!("Running perft test with {num_threads} threads in parallel");
        let solved_tests = AtomicU64::new(0);
        let solved_tests = &solved_tests; // lol (necessary to not move the atomic itself, which wouldn't compile)
        let num_fens = fens.len();
        let mut fens = fens.iter().collect_vec();
        // the chess960 perft suite takes so long that it makes sense to just stop the test suite at some point when doing
        // routine testing. Shuffle to ensure that all positions have a chance of being tested.
        let seed = rng().random_range(..=u64::MAX);
        println!("\nSEED: {seed}\n");
        let mut rng = StdRng::seed_from_u64(seed);
        fens.shuffle(&mut rng);
        let testcases_per_thread = (num_fens + num_threads.get() - 1) / num_threads;
        let thread_data = fens.iter().chunks(testcases_per_thread);
        let mut thread_fens = vec![];
        for chunk in &thread_data {
            thread_fens.push(chunk.collect_vec());
        }
        scope(|s| {
            let mut handles = vec![];
            for chunk in thread_fens {
                let handle = s.spawn(move || {
                    for testcase in chunk {
                        let expected = ExpectedPerftRes::new(testcase);
                        let board = Chessboard::from_fen(expected.fen, strictness).unwrap();
                        println!("Thread {1:?}: Running test on fen {0}, board\n{board}", expected.fen, current().id());
                        stdout().flush().unwrap();
                        let fen = board.as_fen();
                        let board2 = Chessboard::from_fen(&fen, strictness).unwrap();
                        if board != board2 {
                            eprintln!("boards differ: {board} vs {board2}, fen was {}", expected.fen);
                            // it's fine for relaxed FENs to contain illegal pseudolegal ep moves
                            assert_eq!(strictness, Relaxed);
                            assert!(Chessboard::from_fen(expected.fen, Strict).is_err());
                            assert!(board.pseudolegal_moves().iter().any(|m| m.is_ep()));
                            assert!(!board.legal_moves_slow().iter().any(|m| m.is_ep()));
                        }
                        for (depth, expected_count) in
                            expected.res.iter().enumerate().filter(|(_depth, x)| **x != INVALID)
                        {
                            let res = perft(DepthPly::new(depth), board, false);
                            assert_eq!(res.depth.get(), depth);
                            assert_eq!(res.nodes, *expected_count, "{depth} {board}");
                            println!(
                                "Thread {3:?}: Perft depth {0} took {1} ms, total time so far: {2}ms",
                                res.depth.get(),
                                res.time.as_millis(),
                                start_time.elapsed().as_millis(),
                                current().id()
                            );
                        }
                        _ = solved_tests.fetch_add(1, Ordering::Relaxed);
                        println!("Finished {0} / {1} positions", solved_tests.load(Ordering::Relaxed), num_fens);
                    }
                });
                handles.push(handle);
            }
            for (i, handle) in handles.into_iter().enumerate() {
                if let Err(err) = handle.join() {
                    eprintln!("Error in spawned thread {i}: {err:?}");
                }
            }
        });
    }

    const CUSTOM_FENS: &[&str] = &[
        // tricky to parse X-FENs
        "r1rkrqnb/1b6/2n5/ppppppp1/PPPPPP2/B1NQ4/6PP/1K1RR1NB w Kkq - 8 14 ;D1 42 ;D2 1620 ;D3 67391 ;D4 2592441 ;D5 107181922",
        "rrkrn1r1/2P2P2/8/p3pP2/8/8/4R2p/1RR1KR1R w KCdq e6 90 99 ;D1 53 ;D2 1349 ;D3 63522 ;D4 1754940 ;D5 80364051",
        "rr2k1r1/p1p4P/1p3P2/8/1P6/3p4/7P/2RK1R1R w Kk - 0 1 ;D1 29 ;D2 522 ;D3 14924 ;D4 277597 ;D5 8098755",
        "8/2k5/8/8/8/8/8/RR1K1R1R w KB - 0 1 ;D1 38 ;D2 174 ;D3 7530 ;D4 35038 ;D5 1620380 ;D6 7173240",
        // pins
        "1nbqkbnr/ppp1pppp/8/r2pP2K/8/8/PPPP1PPP/RNBQ1BNR w k d6 0 2 ;D1 31;D2 927 ;D3 26832 ;D4 813632 ;D5 23977743",
        "2k5/3q4/8/8/3B4/3K1B1r/8/8 w - - 0 1 ;D1 7 ;D2 211; D3 4246 ;D4 138376 ;D5 2611571 ;D6 85530145",
        // castling through check
        "2r3kr/8/8/8/8/8/8/RK2R3 w Qk - 0 1; D1 21 ;D2 447 ;D3 9933 ;D4 226424 ;D5 5338161 ;D6 126787151",
    ];

    const STANDARD_FENS: &[&str] = &[
        // positions from https://github.com/AndyGrant/Ethereal/blob/master/src/perft/standard.epd,
        // which are themselves based on <http://www.rocechess.ch/perft.html>,
        // those positions were collected/calculated by Andrew Wagner and published in 2004 on CCC
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 ;D1 20 ;D2 400 ;D3 8902 ;D4 197281 ;D5 4865609 ;D6 119060324",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1 ;D1 48 ;D2 2039 ;D3 97862 ;D4 4085603 ;D5 193690690",
        "4k3/8/8/8/8/8/8/4K2R w K - 0 1 ;D1 15 ;D2 66 ;D3 1197 ;D4 7059 ;D5 133987 ;D6 764643",
        "4k3/8/8/8/8/8/8/R3K3 w Q - 0 1 ;D1 16 ;D2 71 ;D3 1287 ;D4 7626 ;D5 145232 ;D6 846648",
        "4k2r/8/8/8/8/8/8/4K3 w k - 0 1 ;D1 5 ;D2 75 ;D3 459 ;D4 8290 ;D5 47635 ;D6 899442",
        "r3k3/8/8/8/8/8/8/4K3 w q - 0 1 ;D1 5 ;D2 80 ;D3 493 ;D4 8897 ;D5 52710 ;D6 1001523",
        "4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1 ;D1 26 ;D2 112 ;D3 3189 ;D4 17945 ;D5 532933 ;D6 2788982",
        "r3k2r/8/8/8/8/8/8/4K3 w kq - 0 1 ;D1 5 ;D2 130 ;D3 782 ;D4 22180 ;D5 118882 ;D6 3517770",
        "8/8/8/8/8/8/6k1/4K2R w K - 0 1 ;D1 12 ;D2 38 ;D3 564 ;D4 2219 ;D5 37735 ;D6 185867",
        "8/8/8/8/8/8/1k6/R3K3 w Q - 0 1 ;D1 15 ;D2 65 ;D3 1018 ;D4 4573 ;D5 80619 ;D6 413018",
        "4k2r/6K1/8/8/8/8/8/8 w k - 0 1 ;D1 3 ;D2 32 ;D3 134 ;D4 2073 ;D5 10485 ;D6 179869",
        "r3k3/1K6/8/8/8/8/8/8 w q - 0 1 ;D1 4 ;D2 49 ;D3 243 ;D4 3991 ;D5 20780 ;D6 367724",
        "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1 ;D1 26 ;D2 568 ;D3 13744 ;D4 314346 ;D5 7594526 ;D6 179862938",
        "r3k2r/8/8/8/8/8/8/1R2K2R w Kkq - 0 1 ;D1 25 ;D2 567 ;D3 14095 ;D4 328965 ;D5 8153719 ;D6 195629489",
        "r3k2r/8/8/8/8/8/8/2R1K2R w Kkq - 0 1 ;D1 25 ;D2 548 ;D3 13502 ;D4 312835 ;D5 7736373 ;D6 184411439",
        "r3k2r/8/8/8/8/8/8/R3K1R1 w Qkq - 0 1 ;D1 25 ;D2 547 ;D3 13579 ;D4 316214 ;D5 7878456 ;D6 189224276",
        "1r2k2r/8/8/8/8/8/8/R3K2R w KQk - 0 1 ;D1 26 ;D2 583 ;D3 14252 ;D4 334705 ;D5 8198901 ;D6 198328929",
        "2r1k2r/8/8/8/8/8/8/R3K2R w KQk - 0 1 ;D1 25 ;D2 560 ;D3 13592 ;D4 317324 ;D5 7710115 ;D6 185959088",
        "r3k1r1/8/8/8/8/8/8/R3K2R w KQq - 0 1 ;D1 25 ;D2 560 ;D3 13607 ;D4 320792 ;D5 7848606 ;D6 190755813",
        "4k3/8/8/8/8/8/8/4K2R b K - 0 1 ;D1 5 ;D2 75 ;D3 459 ;D4 8290 ;D5 47635 ;D6 899442",
        "4k3/8/8/8/8/8/8/R3K3 b Q - 0 1 ;D1 5 ;D2 80 ;D3 493 ;D4 8897 ;D5 52710 ;D6 1001523",
        "4k2r/8/8/8/8/8/8/4K3 b k - 0 1 ;D1 15 ;D2 66 ;D3 1197 ;D4 7059 ;D5 133987 ;D6 764643",
        "r3k3/8/8/8/8/8/8/4K3 b q - 0 1 ;D1 16 ;D2 71 ;D3 1287 ;D4 7626 ;D5 145232 ;D6 846648",
        "4k3/8/8/8/8/8/8/R3K2R b KQ - 0 1 ;D1 5 ;D2 130 ;D3 782 ;D4 22180 ;D5 118882 ;D6 3517770",
        "r3k2r/8/8/8/8/8/8/4K3 b kq - 0 1 ;D1 26 ;D2 112 ;D3 3189 ;D4 17945 ;D5 532933 ;D6 2788982",
        "8/8/8/8/8/8/6k1/4K2R b K - 0 1 ;D1 3 ;D2 32 ;D3 134 ;D4 2073 ;D5 10485 ;D6 179869",
        "8/8/8/8/8/8/1k6/R3K3 b Q - 0 1 ;D1 4 ;D2 49 ;D3 243 ;D4 3991 ;D5 20780 ;D6 367724",
        "4k2r/6K1/8/8/8/8/8/8 b k - 0 1 ;D1 12 ;D2 38 ;D3 564 ;D4 2219 ;D5 37735 ;D6 185867",
        "r3k3/1K6/8/8/8/8/8/8 b q - 0 1 ;D1 15 ;D2 65 ;D3 1018 ;D4 4573 ;D5 80619 ;D6 413018",
        "r3k2r/8/8/8/8/8/8/R3K2R b KQkq - 0 1 ;D1 26 ;D2 568 ;D3 13744 ;D4 314346 ;D5 7594526 ;D6 179862938",
        "r3k2r/8/8/8/8/8/8/1R2K2R b Kkq - 0 1 ;D1 26 ;D2 583 ;D3 14252 ;D4 334705 ;D5 8198901 ;D6 198328929",
        "r3k2r/8/8/8/8/8/8/2R1K2R b Kkq - 0 1 ;D1 25 ;D2 560 ;D3 13592 ;D4 317324 ;D5 7710115 ;D6 185959088",
        "r3k2r/8/8/8/8/8/8/R3K1R1 b Qkq - 0 1 ;D1 25 ;D2 560 ;D3 13607 ;D4 320792 ;D5 7848606 ;D6 190755813",
        "1r2k2r/8/8/8/8/8/8/R3K2R b KQk - 0 1 ;D1 25 ;D2 567 ;D3 14095 ;D4 328965 ;D5 8153719 ;D6 195629489",
        "2r1k2r/8/8/8/8/8/8/R3K2R b KQk - 0 1 ;D1 25 ;D2 548 ;D3 13502 ;D4 312835 ;D5 7736373 ;D6 184411439",
        "r3k1r1/8/8/8/8/8/8/R3K2R b KQq - 0 1 ;D1 25 ;D2 547 ;D3 13579 ;D4 316214 ;D5 7878456 ;D6 189224276",
        "8/1n4N1/2k5/8/8/5K2/1N4n1/8 w - - 0 1 ;D1 14 ;D2 195 ;D3 2760 ;D4 38675 ;D5 570726 ;D6 8107539",
        "8/1k6/8/5N2/8/4n3/8/2K5 w - - 0 1 ;D1 11 ;D2 156 ;D3 1636 ;D4 20534 ;D5 223507 ;D6 2594412",
        "8/8/4k3/3Nn3/3nN3/4K3/8/8 w - - 0 1 ;D1 19 ;D2 289 ;D3 4442 ;D4 73584 ;D5 1198299 ;D6 19870403",
        "K7/8/2n5/1n6/8/8/8/k6N w - - 0 1 ;D1 3 ;D2 51 ;D3 345 ;D4 5301 ;D5 38348 ;D6 588695",
        "k7/8/2N5/1N6/8/8/8/K6n w - - 0 1 ;D1 17 ;D2 54 ;D3 835 ;D4 5910 ;D5 92250 ;D6 688780",
        "8/1n4N1/2k5/8/8/5K2/1N4n1/8 b - - 0 1 ;D1 15 ;D2 193 ;D3 2816 ;D4 40039 ;D5 582642 ;D6 8503277",
        "8/1k6/8/5N2/8/4n3/8/2K5 b - - 0 1 ;D1 16 ;D2 180 ;D3 2290 ;D4 24640 ;D5 288141 ;D6 3147566",
        "8/8/3K4/3Nn3/3nN3/4k3/8/8 b - - 0 1 ;D1 4 ;D2 68 ;D3 1118 ;D4 16199 ;D5 281190 ;D6 4405103",
        "K7/8/2n5/1n6/8/8/8/k6N b - - 0 1 ;D1 17 ;D2 54 ;D3 835 ;D4 5910 ;D5 92250 ;D6 688780",
        "k7/8/2N5/1N6/8/8/8/K6n b - - 0 1 ;D1 3 ;D2 51 ;D3 345 ;D4 5301 ;D5 38348 ;D6 588695",
        "B6b/8/8/8/2K5/4k3/8/b6B w - - 0 1 ;D1 17 ;D2 278 ;D3 4607 ;D4 76778 ;D5 1320507 ;D6 22823890",
        "8/8/1B6/7b/7k/8/2B1b3/7K w - - 0 1 ;D1 21 ;D2 316 ;D3 5744 ;D4 93338 ;D5 1713368 ;D6 28861171",
        "k7/B7/1B6/1B6/8/8/8/K6b w - - 0 1 ;D1 21 ;D2 144 ;D3 3242 ;D4 32955 ;D5 787524 ;D6 7881673",
        "K7/b7/1b6/1b6/8/8/8/k6B w - - 0 1 ;D1 7 ;D2 143 ;D3 1416 ;D4 31787 ;D5 310862 ;D6 7382896",
        "B6b/8/8/8/2K5/5k2/8/b6B b - - 0 1 ;D1 6 ;D2 106 ;D3 1829 ;D4 31151 ;D5 530585 ;D6 9250746",
        "8/8/1B6/7b/7k/8/2B1b3/7K b - - 0 1 ;D1 17 ;D2 309 ;D3 5133 ;D4 93603 ;D5 1591064 ;D6 29027891",
        "k7/B7/1B6/1B6/8/8/8/K6b b - - 0 1 ;D1 7 ;D2 143 ;D3 1416 ;D4 31787 ;D5 310862 ;D6 7382896",
        "K7/b7/1b6/1b6/8/8/8/k6B b - - 0 1 ;D1 21 ;D2 144 ;D3 3242 ;D4 32955 ;D5 787524 ;D6 7881673",
        "7k/RR6/8/8/8/8/rr6/7K w - - 0 1 ;D1 19 ;D2 275 ;D3 5300 ;D4 104342 ;D5 2161211 ;D6 44956585",
        "R6r/8/8/2K5/5k2/8/8/r6R w - - 0 1 ;D1 36 ;D2 1027 ;D3 29215 ;D4 771461 ;D5 20506480 ;D6 525169084",
        "7k/RR6/8/8/8/8/rr6/7K b - - 0 1 ;D1 19 ;D2 275 ;D3 5300 ;D4 104342 ;D5 2161211 ;D6 44956585",
        "R6r/8/8/2K5/5k2/8/8/r6R b - - 0 1 ;D1 36 ;D2 1027 ;D3 29227 ;D4 771368 ;D5 20521342 ;D6 524966748",
        "6kq/8/8/8/8/8/8/7K w - - 0 1 ;D1 2 ;D2 36 ;D3 143 ;D4 3637 ;D5 14893 ;D6 391507",
        "6KQ/8/8/8/8/8/8/7k b - - 0 1 ;D1 2 ;D2 36 ;D3 143 ;D4 3637 ;D5 14893 ;D6 391507",
        "K7/8/8/3Q4/4q3/8/8/7k w - - 0 1 ;D1 6 ;D2 35 ;D3 495 ;D4 8349 ;D5 166741 ;D6 3370175",
        "6qk/8/8/8/8/8/8/7K b - - 0 1 ;D1 22 ;D2 43 ;D3 1015 ;D4 4167 ;D5 105749 ;D6 419369",
        "6KQ/8/8/8/8/8/8/7k b - - 0 1 ;D1 2 ;D2 36 ;D3 143 ;D4 3637 ;D5 14893 ;D6 391507",
        "K7/8/8/3Q4/4q3/8/8/7k b - - 0 1 ;D1 6 ;D2 35 ;D3 495 ;D4 8349 ;D5 166741 ;D6 3370175",
        "8/8/8/8/8/K7/P7/k7 w - - 0 1 ;D1 3 ;D2 7 ;D3 43 ;D4 199 ;D5 1347 ;D6 6249",
        "8/8/8/8/8/7K/7P/7k w - - 0 1 ;D1 3 ;D2 7 ;D3 43 ;D4 199 ;D5 1347 ;D6 6249",
        "K7/p7/k7/8/8/8/8/8 w - - 0 1 ;D1 1 ;D2 3 ;D3 12 ;D4 80 ;D5 342 ;D6 2343",
        "7K/7p/7k/8/8/8/8/8 w - - 0 1 ;D1 1 ;D2 3 ;D3 12 ;D4 80 ;D5 342 ;D6 2343",
        "8/2k1p3/3pP3/3P2K1/8/8/8/8 w - - 0 1 ;D1 7 ;D2 35 ;D3 210 ;D4 1091 ;D5 7028 ;D6 34834",
        "8/8/8/8/8/K7/P7/k7 b - - 0 1 ;D1 1 ;D2 3 ;D3 12 ;D4 80 ;D5 342 ;D6 2343",
        "8/8/8/8/8/7K/7P/7k b - - 0 1 ;D1 1 ;D2 3 ;D3 12 ;D4 80 ;D5 342 ;D6 2343",
        "K7/p7/k7/8/8/8/8/8 b - - 0 1 ;D1 3 ;D2 7 ;D3 43 ;D4 199 ;D5 1347 ;D6 6249",
        "7K/7p/7k/8/8/8/8/8 b - - 0 1 ;D1 3 ;D2 7 ;D3 43 ;D4 199 ;D5 1347 ;D6 6249",
        "8/2k1p3/3pP3/3P2K1/8/8/8/8 b - - 0 1 ;D1 5 ;D2 35 ;D3 182 ;D4 1091 ;D5 5408 ;D6 34822",
        "8/8/8/8/8/4k3/4P3/4K3 w - - 0 1 ;D1 2 ;D2 8 ;D3 44 ;D4 282 ;D5 1814 ;D6 11848",
        "4k3/4p3/4K3/8/8/8/8/8 b - - 0 1 ;D1 2 ;D2 8 ;D3 44 ;D4 282 ;D5 1814 ;D6 11848",
        "8/8/7k/7p/7P/7K/8/8 w - - 0 1 ;D1 3 ;D2 9 ;D3 57 ;D4 360 ;D5 1969 ;D6 10724",
        "8/8/k7/p7/P7/K7/8/8 w - - 0 1 ;D1 3 ;D2 9 ;D3 57 ;D4 360 ;D5 1969 ;D6 10724",
        "8/8/3k4/3p4/3P4/3K4/8/8 w - - 0 1 ;D1 5 ;D2 25 ;D3 180 ;D4 1294 ;D5 8296 ;D6 53138",
        "8/3k4/3p4/8/3P4/3K4/8/8 w - - 0 1 ;D1 8 ;D2 61 ;D3 483 ;D4 3213 ;D5 23599 ;D6 157093",
        "8/8/3k4/3p4/8/3P4/3K4/8 w - - 0 1 ;D1 8 ;D2 61 ;D3 411 ;D4 3213 ;D5 21637 ;D6 158065",
        "k7/8/3p4/8/3P4/8/8/7K w - - 0 1 ;D1 4 ;D2 15 ;D3 90 ;D4 534 ;D5 3450 ;D6 20960",
        "8/8/7k/7p/7P/7K/8/8 b - - 0 1 ;D1 3 ;D2 9 ;D3 57 ;D4 360 ;D5 1969 ;D6 10724",
        "8/8/k7/p7/P7/K7/8/8 b - - 0 1 ;D1 3 ;D2 9 ;D3 57 ;D4 360 ;D5 1969 ;D6 10724",
        "8/8/3k4/3p4/3P4/3K4/8/8 b - - 0 1 ;D1 5 ;D2 25 ;D3 180 ;D4 1294 ;D5 8296 ;D6 53138",
        "8/3k4/3p4/8/3P4/3K4/8/8 b - - 0 1 ;D1 8 ;D2 61 ;D3 411 ;D4 3213 ;D5 21637 ;D6 158065",
        "8/8/3k4/3p4/8/3P4/3K4/8 b - - 0 1 ;D1 8 ;D2 61 ;D3 483 ;D4 3213 ;D5 23599 ;D6 157093",
        "k7/8/3p4/8/3P4/8/8/7K b - - 0 1 ;D1 4 ;D2 15 ;D3 89 ;D4 537 ;D5 3309 ;D6 21104",
        "7k/3p4/8/8/3P4/8/8/K7 w - - 0 1 ;D1 4 ;D2 19 ;D3 117 ;D4 720 ;D5 4661 ;D6 32191",
        "7k/8/8/3p4/8/8/3P4/K7 w - - 0 1 ;D1 5 ;D2 19 ;D3 116 ;D4 716 ;D5 4786 ;D6 30980",
        "k7/8/8/7p/6P1/8/8/K7 w - - 0 1 ;D1 5 ;D2 22 ;D3 139 ;D4 877 ;D5 6112 ;D6 41874",
        "k7/8/7p/8/8/6P1/8/K7 w - - 0 1 ;D1 4 ;D2 16 ;D3 101 ;D4 637 ;D5 4354 ;D6 29679",
        "k7/8/8/6p1/7P/8/8/K7 w - - 0 1 ;D1 5 ;D2 22 ;D3 139 ;D4 877 ;D5 6112 ;D6 41874",
        "k7/8/6p1/8/8/7P/8/K7 w - - 0 1 ;D1 4 ;D2 16 ;D3 101 ;D4 637 ;D5 4354 ;D6 29679",
        "k7/8/8/3p4/4p3/8/8/7K w - - 0 1 ;D1 3 ;D2 15 ;D3 84 ;D4 573 ;D5 3013 ;D6 22886",
        "k7/8/3p4/8/8/4P3/8/7K w - - 0 1 ;D1 4 ;D2 16 ;D3 101 ;D4 637 ;D5 4271 ;D6 28662",
        "7k/3p4/8/8/3P4/8/8/K7 b - - 0 1 ;D1 5 ;D2 19 ;D3 117 ;D4 720 ;D5 5014 ;D6 32167",
        "7k/8/8/3p4/8/8/3P4/K7 b - - 0 1 ;D1 4 ;D2 19 ;D3 117 ;D4 712 ;D5 4658 ;D6 30749",
        "k7/8/8/7p/6P1/8/8/K7 b - - 0 1 ;D1 5 ;D2 22 ;D3 139 ;D4 877 ;D5 6112 ;D6 41874",
        "k7/8/7p/8/8/6P1/8/K7 b - - 0 1 ;D1 4 ;D2 16 ;D3 101 ;D4 637 ;D5 4354 ;D6 29679",
        "k7/8/8/6p1/7P/8/8/K7 b - - 0 1 ;D1 5 ;D2 22 ;D3 139 ;D4 877 ;D5 6112 ;D6 41874",
        "k7/8/6p1/8/8/7P/8/K7 b - - 0 1 ;D1 4 ;D2 16 ;D3 101 ;D4 637 ;D5 4354 ;D6 29679",
        "k7/8/8/3p4/4p3/8/8/7K b - - 0 1 ;D1 5 ;D2 15 ;D3 102 ;D4 569 ;D5 4337 ;D6 22579",
        "k7/8/3p4/8/8/4P3/8/7K b - - 0 1 ;D1 4 ;D2 16 ;D3 101 ;D4 637 ;D5 4271 ;D6 28662",
        "7k/8/8/p7/1P6/8/8/7K w - - 0 1 ;D1 5 ;D2 22 ;D3 139 ;D4 877 ;D5 6112 ;D6 41874",
        "7k/8/p7/8/8/1P6/8/7K w - - 0 1 ;D1 4 ;D2 16 ;D3 101 ;D4 637 ;D5 4354 ;D6 29679",
        "7k/8/8/1p6/P7/8/8/7K w - - 0 1 ;D1 5 ;D2 22 ;D3 139 ;D4 877 ;D5 6112 ;D6 41874",
        "7k/8/1p6/8/8/P7/8/7K w - - 0 1 ;D1 4 ;D2 16 ;D3 101 ;D4 637 ;D5 4354 ;D6 29679",
        "k7/7p/8/8/8/8/6P1/K7 w - - 0 1 ;D1 5 ;D2 25 ;D3 161 ;D4 1035 ;D5 7574 ;D6 55338",
        "k7/6p1/8/8/8/8/7P/K7 w - - 0 1 ;D1 5 ;D2 25 ;D3 161 ;D4 1035 ;D5 7574 ;D6 55338",
        "3k4/3pp3/8/8/8/8/3PP3/3K4 w - - 0 1 ;D1 7 ;D2 49 ;D3 378 ;D4 2902 ;D5 24122 ;D6 199002",
        "7k/8/8/p7/1P6/8/8/7K b - - 0 1 ;D1 5 ;D2 22 ;D3 139 ;D4 877 ;D5 6112 ;D6 41874",
        "7k/8/p7/8/8/1P6/8/7K b - - 0 1 ;D1 4 ;D2 16 ;D3 101 ;D4 637 ;D5 4354 ;D6 29679",
        "7k/8/8/1p6/P7/8/8/7K b - - 0 1 ;D1 5 ;D2 22 ;D3 139 ;D4 877 ;D5 6112 ;D6 41874",
        "7k/8/1p6/8/8/P7/8/7K b - - 0 1 ;D1 4 ;D2 16 ;D3 101 ;D4 637 ;D5 4354 ;D6 29679",
        "k7/7p/8/8/8/8/6P1/K7 b - - 0 1 ;D1 5 ;D2 25 ;D3 161 ;D4 1035 ;D5 7574 ;D6 55338",
        "k7/6p1/8/8/8/8/7P/K7 b - - 0 1 ;D1 5 ;D2 25 ;D3 161 ;D4 1035 ;D5 7574 ;D6 55338",
        "3k4/3pp3/8/8/8/8/3PP3/3K4 b - - 0 1 ;D1 7 ;D2 49 ;D3 378 ;D4 2902 ;D5 24122 ;D6 199002",
        "8/Pk6/8/8/8/8/6Kp/8 w - - 0 1 ;D1 11 ;D2 97 ;D3 887 ;D4 8048 ;D5 90606 ;D6 1030499",
        "n1n5/1Pk5/8/8/8/8/5Kp1/5N1N w - - 0 1 ;D1 24 ;D2 421 ;D3 7421 ;D4 124608 ;D5 2193768 ;D6 37665329",
        "8/PPPk4/8/8/8/8/4Kppp/8 w - - 0 1 ;D1 18 ;D2 270 ;D3 4699 ;D4 79355 ;D5 1533145 ;D6 28859283",
        "n1n5/PPPk4/8/8/8/8/4Kppp/5N1N w - - 0 1 ;D1 24 ;D2 496 ;D3 9483 ;D4 182838 ;D5 3605103 ;D6 71179139",
        "8/Pk6/8/8/8/8/6Kp/8 b - - 0 1 ;D1 11 ;D2 97 ;D3 887 ;D4 8048 ;D5 90606 ;D6 1030499",
        "n1n5/1Pk5/8/8/8/8/5Kp1/5N1N b - - 0 1 ;D1 24 ;D2 421 ;D3 7421 ;D4 124608 ;D5 2193768 ;D6 37665329",
        "8/PPPk4/8/8/8/8/4Kppp/8 b - - 0 1 ;D1 18 ;D2 270 ;D3 4699 ;D4 79355 ;D5 1533145 ;D6 28859283",
        "n1n5/PPPk4/8/8/8/8/4Kppp/5N1N b - - 0 1 ;D1 24 ;D2 496 ;D3 9483 ;D4 182838 ;D5 3605103 ;D6 71179139",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1 ;D4 43238 ;D5 674624 ;D6 11030083",
        "rnbqkb1r/ppppp1pp/7n/4Pp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3 ;D5 11139762",
        // positions from https://analog-hors.github.io/webperft/
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1 ;D1 6 ;D2 264 ;D3 9467 ;D4 422333 ;D5 15833292 ;D6 706045033",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8 ;D1 44 ;D2 1486 ;D3 62379 ;D4 2103487 ;D5 89941194 ;D6 3048196529", // can take a while
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10 ;D1 46 ;D2 2079 ;D3 89890 ;D4 3894594 ;D5 164075551",
        // another pinned pawns test (this time without en passant)
        "4Q3/5p2/6k1/8/4p3/8/2B3K1/8 b - - 0 1 ;D1 7 ;D2 203 ;D3 1250 ;D4 37962 ;D5 227787 ;D6 7036323 ;D7 41501304",
        // yet another pinned pawn test
        "5Q2/8/8/2p5/1k3p1Q/6P1/6K1/8 b - - 1 42 ;D1 7 ;D2 266 ;D3 2018 ;D4 74544 ;D5 504298; D6 19353971",
        // maximum number of legal moves (and mate in one)
        "R6R/3Q4/1Q4Q1/4Q3/2Q4Q/Q4Q2/pp1Q4/kBNN1KB1 w - - 0 1 ;D1 218 ;D2 99 ;D3 19073 ;D4 85043 ;D5 13853661", // D6 115892741",
        // the same position with flipped side to move has no legal moves
        "R6R/3Q4/1Q4Q1/4Q3/2Q4Q/Q4Q2/pp1Q4/kBNN1KB1 b - - 0 1 ;D1 0 ;D2 0 ;D3 0 ;D4 0 ;D5 0 ;D6 0 ;D7 0",
        // a very weird position (not reachable from startpos, but still somewhat realistic)
        "RNBQKBNR/PPPPPPPP/8/8/8/8/pppppppp/rnbqkbnr w - - 0 1 ;D1 4 ;D2 16 ;D3 176 ;D4 1936 ;D5 22428 ;D6 255135 ;D7 3830854",
        // Triggered a bug in SF once which didn't appear in the usual perft test suite
        "r7/4p3/5p1q/3P4/4pQ2/4pP2/6pp/R3K1kr w Q - 1 3 ;D1 29 ;D2 681 ;D3 18511 ;D4 430036 ;D5 11609488 ;D6 274691896",
    ];

    /// This perft test suite is also taken from Ethereal: <https://github.com/AndyGrant/Ethereal/blob/master/src/perft/fischer.epd>.
    /// I have no idea why it is this massive, running it takes forever.
    const CHESS_960_FENS: [&str; 960] = [
        "bqnb1rkr/pp3ppp/3ppn2/2p5/5P2/P2P4/NPP1P1PP/BQ1BNRKR w HFhf - 2 9 ;D1 21 ;D2 528 ;D3 12189 ;D4 326672 ;D5 8146062 ;D6 227689589",
        "2nnrbkr/p1qppppp/8/1ppb4/6PP/3PP3/PPP2P2/BQNNRBKR w HEhe - 1 9 ;D1 21 ;D2 807 ;D3 18002 ;D4 667366 ;D5 16253601 ;D6 590751109",
        "b1q1rrkb/pppppppp/3nn3/8/P7/1PPP4/4PPPP/BQNNRKRB w GE - 1 9 ;D1 20 ;D2 479 ;D3 10471 ;D4 273318 ;D5 6417013 ;D6 177654692",
        "qbbnnrkr/2pp2pp/p7/1p2pp2/8/P3PP2/1PPP1KPP/QBBNNR1R w hf - 0 9 ;D1 22 ;D2 593 ;D3 13440 ;D4 382958 ;D5 9183776 ;D6 274103539",
        "1nbbnrkr/p1p1ppp1/3p4/1p3P1p/3Pq2P/8/PPP1P1P1/QNBBNRKR w HFhf - 0 9 ;D1 28 ;D2 1120 ;D3 31058 ;D4 1171749 ;D5 34030312 ;D6 1250970898",
        "qnbnr1kr/ppp1b1pp/4p3/3p1p2/8/2NPP3/PPP1BPPP/QNB1R1KR w HEhe - 1 9 ;D1 29 ;D2 899 ;D3 26578 ;D4 824055 ;D5 24851983 ;D6 775718317",
        "q1bnrkr1/ppppp2p/2n2p2/4b1p1/2NP4/8/PPP1PPPP/QNB1RRKB w ge - 1 9 ;D1 30 ;D2 860 ;D3 24566 ;D4 732757 ;D5 21093346 ;D6 649209803",
        "qbn1brkr/ppp1p1p1/2n4p/3p1p2/P7/6PP/QPPPPP2/1BNNBRKR w HFhf - 0 9 ;D1 25 ;D2 635 ;D3 17054 ;D4 465806 ;D5 13203304 ;D6 377184252",
        "qnnbbrkr/1p2ppp1/2pp3p/p7/1P5P/2NP4/P1P1PPP1/Q1NBBRKR w HFhf - 0 9 ;D1 24 ;D2 572 ;D3 15243 ;D4 384260 ;D5 11110203 ;D6 293989890",
        "qn1rbbkr/ppp2p1p/1n1pp1p1/8/3P4/P6P/1PP1PPPK/QNNRBB1R w hd - 2 9 ;D1 28 ;D2 811 ;D3 23175 ;D4 679699 ;D5 19836606 ;D6 594527992",
        "qnr1bkrb/pppp2pp/3np3/5p2/8/P2P2P1/NPP1PP1P/QN1RBKRB w GDg - 3 9 ;D1 33 ;D2 823 ;D3 26895 ;D4 713420 ;D5 23114629 ;D6 646390782",
        "qb1nrkbr/1pppp1p1/1n3p2/p1B4p/8/3P1P1P/PPP1P1P1/QBNNRK1R w HEhe - 0 9 ;D1 31 ;D2 855 ;D3 25620 ;D4 735703 ;D5 21796206 ;D6 651054626",
        "qnnbrk1r/1p1ppbpp/2p5/p4p2/2NP3P/8/PPP1PPP1/Q1NBRKBR w HEhe - 0 9 ;D1 26 ;D2 790 ;D3 21238 ;D4 642367 ;D5 17819770 ;D6 544866674",
        "1qnrkbbr/1pppppp1/p1n4p/8/P7/1P1N1P2/2PPP1PP/QN1RKBBR w HDhd - 0 9 ;D1 37 ;D2 883 ;D3 32187 ;D4 815535 ;D5 29370838 ;D6 783201510",
        "qn1rkrbb/pp1p1ppp/2p1p3/3n4/4P2P/2NP4/PPP2PP1/Q1NRKRBB w FDfd - 1 9 ;D1 24 ;D2 585 ;D3 14769 ;D4 356950 ;D5 9482310 ;D6 233468620",
        "bb1qnrkr/pp1p1pp1/1np1p3/4N2p/8/1P4P1/P1PPPP1P/BBNQ1RKR w HFhf - 0 9 ;D1 29 ;D2 864 ;D3 25747 ;D4 799727 ;D5 24219627 ;D6 776836316",
        "bnqbnr1r/p1p1ppkp/3p4/1p4p1/P7/3NP2P/1PPP1PP1/BNQB1RKR w HF - 0 9 ;D1 26 ;D2 889 ;D3 24353 ;D4 832956 ;D5 23701014 ;D6 809194268",
        "bnqnrbkr/1pp2pp1/p7/3pP2p/4P1P1/8/PPPP3P/BNQNRBKR w HEhe d6 0 9 ;D1 31 ;D2 984 ;D3 28677 ;D4 962591 ;D5 29032175 ;D6 1008880643",
        "b1qnrrkb/ppp1pp1p/n2p1Pp1/8/8/P7/1PPPP1PP/BNQNRKRB w GE - 0 9 ;D1 20 ;D2 484 ;D3 10532 ;D4 281606 ;D5 6718715 ;D6 193594729",
        "n1bqnrkr/pp1ppp1p/2p5/6p1/2P2b2/PN6/1PNPPPPP/1BBQ1RKR w HFhf - 2 9 ;D1 23 ;D2 732 ;D3 17746 ;D4 558191 ;D5 14481581 ;D6 457140569",
        "n1bb1rkr/qpnppppp/2p5/p7/P1P5/5P2/1P1PPRPP/NQBBN1KR w Hhf - 1 9 ;D1 27 ;D2 697 ;D3 18724 ;D4 505089 ;D5 14226907 ;D6 400942568",
        "nqb1rbkr/pppppp1p/4n3/6p1/4P3/1NP4P/PP1P1PP1/1QBNRBKR w HEhe - 1 9 ;D1 28 ;D2 641 ;D3 18811 ;D4 456916 ;D5 13780398 ;D6 354122358",
        "n1bnrrkb/pp1pp2p/2p2p2/6p1/5B2/3P4/PPP1PPPP/NQ1NRKRB w GE - 2 9 ;D1 28 ;D2 606 ;D3 16883 ;D4 381646 ;D5 10815324 ;D6 254026570",
        "nbqnbrkr/2ppp1p1/pp3p1p/8/4N2P/1N6/PPPPPPP1/1BQ1BRKR w HFhf - 0 9 ;D1 26 ;D2 626 ;D3 17268 ;D4 437525 ;D5 12719546 ;D6 339132046",
        "nq1bbrkr/pp2nppp/2pp4/4p3/1PP1P3/1B6/P2P1PPP/NQN1BRKR w HFhf - 2 9 ;D1 21 ;D2 504 ;D3 11812 ;D4 302230 ;D5 7697880 ;D6 207028745",
        "nqnrb1kr/2pp1ppp/1p1bp3/p1B5/5P2/3N4/PPPPP1PP/NQ1R1BKR w HDhd - 0 9 ;D1 30 ;D2 672 ;D3 19307 ;D4 465317 ;D5 13454573 ;D6 345445468",
        "nqn2krb/p1prpppp/1pbp4/7P/5P2/8/PPPPPKP1/NQNRB1RB w g - 3 9 ;D1 21 ;D2 461 ;D3 10608 ;D4 248069 ;D5 6194124 ;D6 152861936",
        "nb1n1kbr/ppp1rppp/3pq3/P3p3/8/4P3/1PPPRPPP/NBQN1KBR w Hh - 1 9 ;D1 19 ;D2 566 ;D3 11786 ;D4 358337 ;D5 8047916 ;D6 249171636",
        "nqnbrkbr/1ppppp1p/p7/6p1/6P1/P6P/1PPPPP2/NQNBRKBR w HEhe - 1 9 ;D1 20 ;D2 382 ;D3 8694 ;D4 187263 ;D5 4708975 ;D6 112278808",
        "nq1rkb1r/pp1pp1pp/1n2bp1B/2p5/8/5P1P/PPPPP1P1/NQNRKB1R w HDhd - 2 9 ;D1 24 ;D2 809 ;D3 20090 ;D4 673811 ;D5 17647882 ;D6 593457788",
        "nqnrkrb1/pppppp2/7p/4b1p1/8/PN1NP3/1PPP1PPP/1Q1RKRBB w FDfd - 1 9 ;D1 26 ;D2 683 ;D3 18102 ;D4 473911 ;D5 13055173 ;D6 352398011",
        "bb1nqrkr/1pp1ppp1/pn5p/3p4/8/P2NNP2/1PPPP1PP/BB2QRKR w HFhf - 0 9 ;D1 29 ;D2 695 ;D3 21193 ;D4 552634 ;D5 17454857 ;D6 483785639",
        "bnn1qrkr/pp1ppp1p/2p5/b3Q1p1/8/5P1P/PPPPP1P1/BNNB1RKR w HFhf - 2 9 ;D1 44 ;D2 920 ;D3 35830 ;D4 795317 ;D5 29742670 ;D6 702867204",
        "bnnqrbkr/pp1p2p1/2p1p2p/5p2/1P5P/1R6/P1PPPPP1/BNNQRBK1 w Ehe - 0 9 ;D1 33 ;D2 1022 ;D3 32724 ;D4 1024721 ;D5 32898113 ;D6 1047360456",
        "b1nqrkrb/2pppppp/p7/1P6/1n6/P4P2/1P1PP1PP/BNNQRKRB w GEge - 0 9 ;D1 23 ;D2 638 ;D3 15744 ;D4 446539 ;D5 11735969 ;D6 344211589",
        "n1bnqrkr/3ppppp/1p6/pNp1b3/2P3P1/8/PP1PPP1P/NBB1QRKR w HFhf - 1 9 ;D1 29 ;D2 728 ;D3 20768 ;D4 532084 ;D5 15621236 ;D6 415766465",
        "n2bqrkr/p1p1pppp/1pn5/3p1b2/P6P/1NP5/1P1PPPP1/1NBBQRKR w HFhf - 3 9 ;D1 20 ;D2 533 ;D3 12152 ;D4 325059 ;D5 8088751 ;D6 223068417",
        "nnbqrbkr/1pp1p1p1/p2p4/5p1p/2P1P3/N7/PPQP1PPP/N1B1RBKR w HEhe - 0 9 ;D1 27 ;D2 619 ;D3 18098 ;D4 444421 ;D5 13755384 ;D6 357222394",
        "nnbqrkr1/pp1pp2p/2p2b2/5pp1/1P5P/4P1P1/P1PP1P2/NNBQRKRB w GEge - 1 9 ;D1 32 ;D2 1046 ;D3 33721 ;D4 1111186 ;D5 36218182 ;D6 1202830851",
        "nb1qbrkr/p1pppp2/1p1n2pp/8/1P6/2PN3P/P2PPPP1/NB1QBRKR w HFhf - 0 9 ;D1 25 ;D2 521 ;D3 14021 ;D4 306427 ;D5 8697700 ;D6 201455191",
        "nnq1brkr/pp1pppp1/8/2p4P/8/5K2/PPPbPP1P/NNQBBR1R w hf - 0 9 ;D1 23 ;D2 724 ;D3 18263 ;D4 571072 ;D5 15338230 ;D6 484638597",
        "nnqrbb1r/pppppk2/5pp1/7p/1P6/3P2PP/P1P1PP2/NNQRBBKR w HD - 0 9 ;D1 30 ;D2 717 ;D3 21945 ;D4 547145 ;D5 17166700 ;D6 450069742",
        "nnqr1krb/p1p1pppp/2bp4/8/1p1P4/4P3/PPP2PPP/NNQRBKRB w GDgd - 0 9 ;D1 25 ;D2 873 ;D3 20796 ;D4 728628 ;D5 18162741 ;D6 641708630",
        "nbnqrkbr/p2ppp2/1p4p1/2p4p/3P3P/3N4/PPP1PPPR/NB1QRKB1 w Ehe - 0 9 ;D1 24 ;D2 589 ;D3 15190 ;D4 382317 ;D5 10630667 ;D6 279474189",
        "n1qbrkbr/p1ppp2p/2n2pp1/1p6/1P6/2P3P1/P2PPP1P/NNQBRKBR w HEhe - 0 9 ;D1 22 ;D2 592 ;D3 14269 ;D4 401976 ;D5 10356818 ;D6 301583306",
        "2qrkbbr/ppn1pppp/n1p5/3p4/5P2/P1PP4/1P2P1PP/NNQRKBBR w HDhd - 1 9 ;D1 27 ;D2 750 ;D3 20584 ;D4 605458 ;D5 16819085 ;D6 516796736",
        "1nqr1rbb/pppkp1pp/1n3p2/3p4/1P6/5P1P/P1PPPKP1/NNQR1RBB w - - 1 9 ;D1 24 ;D2 623 ;D3 15921 ;D4 429446 ;D5 11594634 ;D6 322745925",
        "bbn1rqkr/pp1pp2p/4npp1/2p5/1P6/2BPP3/P1P2PPP/1BNNRQKR w HEhe - 0 9 ;D1 23 ;D2 730 ;D3 17743 ;D4 565340 ;D5 14496370 ;D6 468608864",
        "bn1brqkr/pppp2p1/3npp2/7p/PPP5/8/3PPPPP/BNNBRQKR w HEhe - 0 9 ;D1 25 ;D2 673 ;D3 17835 ;D4 513696 ;D5 14284338 ;D6 434008567",
        "bn1rqbkr/ppp1ppp1/1n6/2p4p/7P/3P4/PPP1PPP1/BN1RQBKR w HDhd - 0 9 ;D1 25 ;D2 776 ;D3 20562 ;D4 660217 ;D5 18486027 ;D6 616653869",
        "bnnr1krb/ppp2ppp/3p4/3Bp3/q1P3PP/8/PP1PPP2/BNNRQKR1 w GDgd - 0 9 ;D1 29 ;D2 1040 ;D3 30772 ;D4 1053113 ;D5 31801525 ;D6 1075147725",
        "1bbnrqkr/pp1ppppp/8/2p5/n7/3PNPP1/PPP1P2P/NBB1RQKR w HEhe - 1 9 ;D1 24 ;D2 598 ;D3 15673 ;D4 409766 ;D5 11394778 ;D6 310589129",
        "nnbbrqkr/p2ppp1p/1pp5/8/6p1/N1P5/PPBPPPPP/N1B1RQKR w HEhe - 0 9 ;D1 26 ;D2 530 ;D3 14031 ;D4 326312 ;D5 8846766 ;D6 229270702",
        "nnbrqbkr/2p1p1pp/p4p2/1p1p4/8/NP6/P1PPPPPP/N1BRQBKR w HDhd - 0 9 ;D1 17 ;D2 496 ;D3 10220 ;D4 303310 ;D5 7103549 ;D6 217108001",
        "nnbrqk1b/pp2pprp/2pp2p1/8/3PP1P1/8/PPP2P1P/NNBRQRKB w d - 1 9 ;D1 33 ;D2 820 ;D3 27856 ;D4 706784 ;D5 24714401 ;D6 645835197",
        "1bnrbqkr/ppnpp1p1/2p2p1p/8/1P6/4PPP1/P1PP3P/NBNRBQKR w HDhd - 0 9 ;D1 27 ;D2 705 ;D3 19760 ;D4 548680 ;D5 15964771 ;D6 464662032",
        "n1rbbqkr/pp1pppp1/7p/P1p5/1n6/2PP4/1P2PPPP/NNRBBQKR w HChc - 0 9 ;D1 22 ;D2 631 ;D3 14978 ;D4 431801 ;D5 10911545 ;D6 320838556",
        "n1rqb1kr/p1pppp1p/1pn4b/3P2p1/P7/1P6/2P1PPPP/NNRQBBKR w HChc - 0 9 ;D1 24 ;D2 477 ;D3 12506 ;D4 263189 ;D5 7419372 ;D6 165945904",
        "nnrqbkrb/pppp1pp1/7p/4p3/6P1/2N2B2/PPPPPP1P/NR1QBKR1 w Ggc - 2 9 ;D1 29 ;D2 658 ;D3 19364 ;D4 476620 ;D5 14233587 ;D6 373744834",
        "n1nrqkbr/ppb2ppp/3pp3/2p5/2P3P1/5P2/PP1PPB1P/NBNRQK1R w HDhd - 1 9 ;D1 32 ;D2 801 ;D3 25861 ;D4 681428 ;D5 22318948 ;D6 619857455",
        "2rbqkbr/p1pppppp/1nn5/1p6/7P/P4P2/1PPPP1PB/NNRBQK1R w HChc - 2 9 ;D1 27 ;D2 647 ;D3 18030 ;D4 458057 ;D5 13189156 ;D6 354689323",
        "nn1qkbbr/pp2ppp1/2rp4/2p4p/P2P4/1N5P/1PP1PPP1/1NRQKBBR w HCh - 1 9 ;D1 24 ;D2 738 ;D3 18916 ;D4 586009 ;D5 16420659 ;D6 519075930",
        "nnrqk1bb/p1ppp2p/5rp1/1p3p2/1P4P1/5P1P/P1PPP3/NNRQKRBB w FCc - 1 9 ;D1 25 ;D2 795 ;D3 20510 ;D4 648945 ;D5 17342527 ;D6 556144017",
        "bb1nrkqr/ppppn2p/4ppp1/8/1P4P1/4P3/P1PPKP1P/BBNNR1QR w he - 0 9 ;D1 29 ;D2 664 ;D3 20024 ;D4 498376 ;D5 15373803 ;D6 406016364",
        "bnnbrkqr/1p1ppp2/8/p1p3pp/1P6/N4P2/PBPPP1PP/2NBRKQR w HEhe - 0 9 ;D1 31 ;D2 770 ;D3 24850 ;D4 677212 ;D5 22562080 ;D6 662029574",
        "1nnrkbqr/p1pp1ppp/4p3/1p6/1Pb1P3/6PB/P1PP1P1P/BNNRK1QR w HDhd - 0 9 ;D1 27 ;D2 776 ;D3 22133 ;D4 641002 ;D5 19153245 ;D6 562738257",
        "bnr1kqrb/pppp1pp1/1n5p/4p3/P3P3/3P2P1/1PP2P1P/BNNRKQRB w GDg - 0 9 ;D1 26 ;D2 624 ;D3 16411 ;D4 435426 ;D5 11906515 ;D6 338092952",
        "nbbnrkqr/p1ppp1pp/1p3p2/8/2P5/4P3/PP1P1PPP/NBBNRKQR w HEhe - 1 9 ;D1 25 ;D2 624 ;D3 15561 ;D4 419635 ;D5 10817378 ;D6 311138112",
        "nn1brkqr/pp1bpppp/8/2pp4/P4P2/1PN5/2PPP1PP/N1BBRKQR w HEhe - 1 9 ;D1 23 ;D2 659 ;D3 16958 ;D4 476567 ;D5 13242252 ;D6 373557073",
        "n1brkbqr/ppp1pp1p/6pB/3p4/2Pn4/8/PP2PPPP/NN1RKBQR w HDhd - 0 9 ;D1 32 ;D2 1026 ;D3 30360 ;D4 978278 ;D5 29436320 ;D6 957904151",
        "nnbrkqrb/p2ppp2/Q5pp/1pp5/4PP2/2N5/PPPP2PP/N1BRK1RB w GDgd - 0 9 ;D1 36 ;D2 843 ;D3 29017 ;D4 715537 ;D5 24321197 ;D6 630396940",
        "nbnrbk1r/pppppppq/8/7p/8/1N2QPP1/PPPPP2P/NB1RBK1R w HDhd - 2 9 ;D1 36 ;D2 973 ;D3 35403 ;D4 1018054 ;D5 37143354 ;D6 1124883780",
        "nnrbbkqr/2pppp1p/p7/6p1/1p2P3/4QPP1/PPPP3P/NNRBBK1R w HChc - 0 9 ;D1 36 ;D2 649 ;D3 22524 ;D4 489526 ;D5 16836636 ;D6 416139320",
        "nnrkbbqr/1p2pppp/p2p4/2p5/8/1N2P1P1/PPPP1P1P/1NKRBBQR w hc - 0 9 ;D1 26 ;D2 672 ;D3 18136 ;D4 477801 ;D5 13342771 ;D6 363074681",
        "n1rkbqrb/pp1ppp2/2n3p1/2p4p/P5PP/1P6/2PPPP2/NNRKBQRB w GCgc - 0 9 ;D1 24 ;D2 804 ;D3 20712 ;D4 684001 ;D5 18761475 ;D6 617932151",
        "nbkr1qbr/1pp1pppp/pn1p4/8/3P2P1/5R2/PPP1PP1P/NBN1KQBR w H - 2 9 ;D1 30 ;D2 627 ;D3 18669 ;D4 423329 ;D5 12815016 ;D6 312798696",
        "nnr1kqbr/pp1pp1p1/2p5/b4p1p/P7/1PNP4/2P1PPPP/N1RBKQBR w HChc - 1 9 ;D1 12 ;D2 421 ;D3 6530 ;D4 227044 ;D5 4266410 ;D6 149176979",
        "n1rkqbbr/p1pp1pp1/np2p2p/8/8/N4PP1/PPPPP1BP/N1RKQ1BR w HChc - 0 9 ;D1 27 ;D2 670 ;D3 19119 ;D4 494690 ;D5 14708490 ;D6 397268628",
        "nnr1qrbb/p2kpppp/1p1p4/2p5/6P1/PP1P4/2P1PP1P/NNRKQRBB w FC - 0 9 ;D1 27 ;D2 604 ;D3 17043 ;D4 409665 ;D5 11993332 ;D6 308518181",
        "bbnnrkrq/ppp1pp2/6p1/3p4/7p/7P/PPPPPPP1/BBNNRRKQ w ge - 0 9 ;D1 20 ;D2 559 ;D3 12242 ;D4 355326 ;D5 8427161 ;D6 252274233",
        "bnnbrkr1/ppp2p1p/5q2/3pp1p1/4P3/1N4P1/PPPPRP1P/BN1B1KRQ w Gge - 0 9 ;D1 26 ;D2 1036 ;D3 27228 ;D4 1028084 ;D5 28286576 ;D6 1042120495",
        "bn1rkbrq/1pppppp1/p6p/1n6/3P4/6PP/PPPRPP2/BNN1KBRQ w Ggd - 2 9 ;D1 29 ;D2 633 ;D3 19278 ;D4 455476 ;D5 14333034 ;D6 361900466",
        "b1nrkrqb/1p1npppp/p2p4/2p5/5P2/4P2P/PPPP1RP1/BNNRK1QB w Dfd - 1 9 ;D1 25 ;D2 475 ;D3 12603 ;D4 270909 ;D5 7545536 ;D6 179579818",
        "1bbnrkrq/ppppppp1/8/7p/1n4P1/1PN5/P1PPPP1P/NBBR1KRQ w Gge - 0 9 ;D1 30 ;D2 803 ;D3 25473 ;D4 709716 ;D5 23443854 ;D6 686365049",
        "nnbbrkrq/2pp1pp1/1p5p/pP2p3/7P/N7/P1PPPPP1/N1BBRKRQ w GEge - 0 9 ;D1 18 ;D2 432 ;D3 9638 ;D4 242350 ;D5 6131124 ;D6 160393505",
        "nnbrkbrq/1pppp1p1/p7/7p/1P2Pp2/BN6/P1PP1PPP/1N1RKBRQ w GDgd - 0 9 ;D1 27 ;D2 482 ;D3 13441 ;D4 282259 ;D5 8084701 ;D6 193484216",
        "n1brkrqb/pppp3p/n3pp2/6p1/3P1P2/N1P5/PP2P1PP/N1BRKRQB w FDfd - 0 9 ;D1 28 ;D2 642 ;D3 19005 ;D4 471729 ;D5 14529434 ;D6 384837696",
        "nbnrbk2/p1pppp1p/1p3qr1/6p1/1B1P4/1N6/PPP1PPPP/1BNR1RKQ w d - 2 9 ;D1 30 ;D2 796 ;D3 22780 ;D4 687302 ;D5 20120565 ;D6 641832725",
        "nnrbbrkq/1pp2ppp/3p4/p3p3/3P1P2/1P2P3/P1P3PP/NNRBBKRQ w GC - 1 9 ;D1 31 ;D2 827 ;D3 24538 ;D4 663082 ;D5 19979594 ;D6 549437308",
        "nnrkbbrq/1pp2p1p/p2pp1p1/2P5/8/8/PP1PPPPP/NNRKBBRQ w Ggc - 0 9 ;D1 24 ;D2 762 ;D3 19283 ;D4 624598 ;D5 16838099 ;D6 555230555",
        "nnr1brqb/1ppkp1pp/8/p2p1p2/1P1P4/N1P5/P3PPPP/N1RKBRQB w FC - 1 9 ;D1 23 ;D2 640 ;D3 15471 ;D4 444905 ;D5 11343507 ;D6 334123513",
        "nbnrkrbq/2ppp2p/p4p2/1P4p1/4PP2/8/1PPP2PP/NBNRKRBQ w FDfd - 0 9 ;D1 31 ;D2 826 ;D3 26137 ;D4 732175 ;D5 23555139 ;D6 686250413",
        "1nrbkr1q/1pppp1pp/1n6/p4p2/N1b4P/8/PPPPPPPB/N1RBKR1Q w FCfc - 2 9 ;D1 27 ;D2 862 ;D3 24141 ;D4 755171 ;D5 22027695 ;D6 696353497",
        "nnrkrbbq/pppp2pp/8/4pp2/4P3/P7/1PPPBPPP/NNKRR1BQ w c - 0 9 ;D1 25 ;D2 792 ;D3 19883 ;D4 636041 ;D5 16473376 ;D6 532214177",
        "n1rk1qbb/pppprpp1/2n4p/4p3/2PP3P/8/PP2PPP1/NNRKRQBB w ECc - 1 9 ;D1 25 ;D2 622 ;D3 16031 ;D4 425247 ;D5 11420973 ;D6 321855685",
        "bbq1rnkr/pnp1pp1p/1p1p4/6p1/2P5/2Q1P2P/PP1P1PP1/BB1NRNKR w HEhe - 2 9 ;D1 36 ;D2 870 ;D3 30516 ;D4 811047 ;D5 28127620 ;D6 799738334",
        "bq1brnkr/1p1ppp1p/1np5/p5p1/8/1N5P/PPPPPPP1/BQ1BRNKR w HEhe - 0 9 ;D1 22 ;D2 588 ;D3 13524 ;D4 380068 ;D5 9359618 ;D6 273795898",
        "bq1rn1kr/1pppppbp/Nn4p1/8/8/P7/1PPPPPPP/BQ1RNBKR w HDhd - 1 9 ;D1 24 ;D2 711 ;D3 18197 ;D4 542570 ;D5 14692779 ;D6 445827351",
        "bqnr1kr1/pppppp1p/6p1/5n2/4B3/3N2PP/PbPPPP2/BQNR1KR1 w GDgd - 2 9 ;D1 31 ;D2 1132 ;D3 36559 ;D4 1261476 ;D5 43256823 ;D6 1456721391",
        "qbb1rnkr/ppp3pp/4n3/3ppp2/1P3PP1/8/P1PPPN1P/QBB1RNKR w HEhe - 0 9 ;D1 28 ;D2 696 ;D3 20502 ;D4 541886 ;D5 16492398 ;D6 456983120",
        "qnbbr1kr/pp1ppp1p/4n3/6p1/2p3P1/2PP1P2/PP2P2P/QNBBRNKR w HEhe - 0 9 ;D1 25 ;D2 655 ;D3 16520 ;D4 450189 ;D5 11767038 ;D6 335414976",
        "1nbrnbkr/p1ppp1pp/1p6/5p2/4q1PP/3P4/PPP1PP2/QNBRNBKR w HDhd - 1 9 ;D1 30 ;D2 1162 ;D3 33199 ;D4 1217278 ;D5 36048727 ;D6 1290346802",
        "q1brnkrb/p1pppppp/n7/1p6/P7/3P1P2/QPP1P1PP/1NBRNKRB w GDgd - 0 9 ;D1 32 ;D2 827 ;D3 26106 ;D4 718243 ;D5 23143989 ;D6 673147648",
        "qbnrb1kr/ppp1pp1p/3p4/2n3p1/1P6/6N1/P1PPPPPP/QBNRB1KR w HDhd - 2 9 ;D1 29 ;D2 751 ;D3 23132 ;D4 610397 ;D5 19555214 ;D6 530475036",
        "q1rbbnkr/pppp1p2/2n3pp/2P1p3/3P4/8/PP1NPPPP/Q1RBBNKR w HChc - 2 9 ;D1 29 ;D2 806 ;D3 24540 ;D4 687251 ;D5 21694330 ;D6 619907316",
        "q1r1bbkr/pnpp1ppp/2n1p3/1p6/2P2P2/2N1N3/PP1PP1PP/Q1R1BBKR w HChc - 2 9 ;D1 32 ;D2 1017 ;D3 32098 ;D4 986028 ;D5 31204371 ;D6 958455898",
        "2rnbkrb/pqppppp1/1pn5/7p/2P5/P1R5/QP1PPPPP/1N1NBKRB w Ggc - 4 9 ;D1 26 ;D2 625 ;D3 16506 ;D4 434635 ;D5 11856964 ;D6 336672890",
        "qbnr1kbr/p2ppppp/2p5/1p6/4n2P/P4N2/1PPP1PP1/QBNR1KBR w HDhd - 0 9 ;D1 27 ;D2 885 ;D3 23828 ;D4 767273 ;D5 21855658 ;D6 706272554",
        "qnrbnk1r/pp1pp2p/5p2/2pbP1p1/3P4/1P6/P1P2PPP/QNRBNKBR w HChc - 0 9 ;D1 26 ;D2 954 ;D3 24832 ;D4 892456 ;D5 24415089 ;D6 866744329",
        "qnrnk1br/p1p2ppp/8/1pbpp3/8/PP2N3/1QPPPPPP/1NR1KBBR w HChc - 0 9 ;D1 26 ;D2 783 ;D3 20828 ;D4 634267 ;D5 17477825 ;D6 539674275",
        "qnrnkrbb/Bpppp2p/6p1/5p2/5P2/3PP3/PPP3PP/QNRNKR1B w FCfc - 1 9 ;D1 28 ;D2 908 ;D3 25730 ;D4 861240 ;D5 25251641 ;D6 869525254",
        "bbnqrn1r/ppppp2k/5p2/6pp/7P/1QP5/PP1PPPP1/B1N1RNKR w HE - 0 9 ;D1 33 ;D2 643 ;D3 21790 ;D4 487109 ;D5 16693640 ;D6 410115900",
        "b1qbrnkr/ppp1pp2/2np4/6pp/4P3/2N4P/PPPP1PP1/BQ1BRNKR w HEhe - 0 9 ;D1 28 ;D2 837 ;D3 24253 ;D4 745617 ;D5 22197063 ;D6 696399065",
        "bnqr1bkr/pp1ppppp/2p5/4N3/5P2/P7/1PPPPnPP/BNQR1BKR w HDhd - 3 9 ;D1 25 ;D2 579 ;D3 13909 ;D4 341444 ;D5 8601011 ;D6 225530258",
        "b1qr1krb/pp1ppppp/n2n4/8/2p5/2P3P1/PP1PPP1P/BNQRNKRB w GDgd - 0 9 ;D1 28 ;D2 707 ;D3 19721 ;D4 549506 ;D5 15583376 ;D6 468399900",
        "nbbqr1kr/1pppp1pp/8/p1n2p2/4P3/PN6/1PPPQPPP/1BB1RNKR w HEhe - 0 9 ;D1 30 ;D2 745 ;D3 23416 ;D4 597858 ;D5 19478789 ;D6 515473678",
        "nqbbrn1r/p1pppp1k/1p4p1/7p/4P3/1R3B2/PPPP1PPP/NQB2NKR w H - 0 9 ;D1 24 ;D2 504 ;D3 13512 ;D4 317355 ;D5 9002073 ;D6 228726497",
        "nqbr1bkr/p1p1ppp1/1p1n4/3pN2p/1P6/8/P1PPPPPP/NQBR1BKR w HDhd - 0 9 ;D1 29 ;D2 898 ;D3 26532 ;D4 809605 ;D5 24703467 ;D6 757166494",
        "nqbrn1rb/pppp1kp1/5p1p/4p3/P4B2/3P2P1/1PP1PP1P/NQ1RNKRB w GD - 0 9 ;D1 34 ;D2 671 ;D3 22332 ;D4 473110 ;D5 15556806 ;D6 353235120",
        "nb1r1nkr/ppp1ppp1/2bp4/7p/3P2qP/P6R/1PP1PPP1/NBQRBNK1 w Dhd - 1 9 ;D1 38 ;D2 1691 ;D3 60060 ;D4 2526992 ;D5 88557078 ;D6 3589649998",
        "n1rbbnkr/1p1pp1pp/p7/2p1qp2/1B3P2/3P4/PPP1P1PP/NQRB1NKR w HChc - 0 9 ;D1 24 ;D2 913 ;D3 21595 ;D4 807544 ;D5 19866918 ;D6 737239330",
        "nqrnbbkr/p2p1p1p/1pp5/1B2p1p1/1P3P2/4P3/P1PP2PP/NQRNB1KR w HChc - 0 9 ;D1 33 ;D2 913 ;D3 30159 ;D4 843874 ;D5 28053260 ;D6 804687975",
        "nqr1bkrb/ppp1pp2/2np2p1/P6p/8/2P4P/1P1PPPP1/NQRNBKRB w GCgc - 0 9 ;D1 24 ;D2 623 ;D3 16569 ;D4 442531 ;D5 12681936 ;D6 351623879",
        "nb1rnkbr/pqppppp1/1p5p/8/1PP4P/8/P2PPPP1/NBQRNKBR w HDhd - 1 9 ;D1 31 ;D2 798 ;D3 24862 ;D4 694386 ;D5 22616076 ;D6 666227466",
        "nqrbnkbr/2p1p1pp/3p4/pp3p2/6PP/3P1N2/PPP1PP2/NQRB1KBR w HChc - 0 9 ;D1 24 ;D2 590 ;D3 14409 ;D4 383690 ;D5 9698432 ;D6 274064911",
        "nqrnkbbr/pp1p1p1p/4p1p1/1p6/8/5P1P/P1PPP1P1/NQRNKBBR w HChc - 0 9 ;D1 30 ;D2 1032 ;D3 31481 ;D4 1098116 ;D5 34914919 ;D6 1233362066",
        "nqrnkrbb/p2ppppp/1p6/2p5/2P3P1/5P2/PP1PPN1P/NQR1KRBB w FCfc - 1 9 ;D1 30 ;D2 775 ;D3 23958 ;D4 668000 ;D5 21141738 ;D6 621142773",
        "bbnrqrk1/pp2pppp/4n3/2pp4/P7/1N5P/BPPPPPP1/B2RQNKR w HD - 2 9 ;D1 23 ;D2 708 ;D3 17164 ;D4 554089 ;D5 14343443 ;D6 481405144",
        "bnr1qnkr/p1pp1p1p/1p4p1/4p1b1/2P1P3/1P6/PB1P1PPP/1NRBQNKR w HChc - 1 9 ;D1 30 ;D2 931 ;D3 29249 ;D4 921746 ;D5 30026687 ;D6 968109774",
        "b1rqnbkr/ppp1ppp1/3p3p/2n5/P3P3/2NP4/1PP2PPP/B1RQNBKR w HChc - 0 9 ;D1 24 ;D2 596 ;D3 15533 ;D4 396123 ;D5 11099382 ;D6 294180723",
        "bnrqnr1b/pp1pkppp/2p1p3/P7/2P5/7P/1P1PPPP1/BNRQNKRB w GC - 0 9 ;D1 24 ;D2 572 ;D3 15293 ;D4 390903 ;D5 11208688 ;D6 302955778",
        "n1brq1kr/bppppppp/p7/8/4P1Pn/8/PPPP1P2/NBBRQNKR w HDhd - 0 9 ;D1 20 ;D2 570 ;D3 13139 ;D4 371247 ;D5 9919113 ;D6 284592289",
        "1rbbqnkr/ppn1ppp1/3p3p/2p5/3P4/1N4P1/PPPBPP1P/1R1BQNKR w HBhb - 0 9 ;D1 29 ;D2 1009 ;D3 29547 ;D4 1040816 ;D5 31059587 ;D6 1111986835",
        "nrbq2kr/ppppppb1/5n1p/5Pp1/8/P5P1/1PPPP2P/NRBQNBKR w HBhb - 1 9 ;D1 20 ;D2 520 ;D3 11745 ;D4 316332 ;D5 7809837 ;D6 216997152",
        "nrb1nkrb/pp3ppp/1qBpp3/2p5/8/P5P1/1PPPPP1P/NRBQNKR1 w GBgb - 2 9 ;D1 32 ;D2 850 ;D3 25642 ;D4 734088 ;D5 21981567 ;D6 664886187",
        "1br1bnkr/ppqppp1p/1np3p1/8/1PP4P/4N3/P2PPPP1/NBRQB1KR w HChc - 1 9 ;D1 32 ;D2 798 ;D3 24765 ;D4 691488 ;D5 22076141 ;D6 670296871",
        "nrqbb1kr/1p1pp1pp/2p3n1/p4p2/3PP3/P5N1/1PP2PPP/NRQBB1KR w HBhb - 0 9 ;D1 32 ;D2 791 ;D3 26213 ;D4 684890 ;D5 23239122 ;D6 634260266",
        "nrqn1bkr/ppppp1pp/4b3/8/4P1p1/5P2/PPPP3P/NRQNBBKR w HBhb - 0 9 ;D1 29 ;D2 687 ;D3 20223 ;D4 506088 ;D5 15236287 ;D6 398759980",
        "nrqnbrkb/pppp1p2/4p2p/3B2p1/8/1P4P1/PQPPPP1P/NR1NBKR1 w GB - 0 9 ;D1 37 ;D2 764 ;D3 27073 ;D4 610950 ;D5 21284835 ;D6 514864869",
        "nbrq1kbr/Bp3ppp/2pnp3/3p4/5P2/2P4P/PP1PP1P1/NBRQNK1R w HChc - 0 9 ;D1 40 ;D2 1271 ;D3 48022 ;D4 1547741 ;D5 56588117 ;D6 1850696281",
        "nrqbnkbr/1p2ppp1/p1p4p/3p4/1P6/8/PQPPPPPP/1RNBNKBR w HBhb - 0 9 ;D1 28 ;D2 757 ;D3 23135 ;D4 668025 ;D5 21427496 ;D6 650939962",
        "nrqn1bbr/2ppkppp/4p3/pB6/8/2P1P3/PP1P1PPP/NRQNK1BR w HB - 1 9 ;D1 27 ;D2 642 ;D3 17096 ;D4 442653 ;D5 11872805 ;D6 327545120",
        "nrqnkrb1/p1ppp2p/1p4p1/4bp2/4PP1P/4N3/PPPP2P1/NRQ1KRBB w FBfb - 1 9 ;D1 27 ;D2 958 ;D3 27397 ;D4 960350 ;D5 28520172 ;D6 995356563",
        "1bnrnqkr/pbpp2pp/8/1p2pp2/P6P/3P1N2/1PP1PPP1/BBNR1QKR w HDhd - 0 9 ;D1 27 ;D2 859 ;D3 23475 ;D4 773232 ;D5 21581178 ;D6 732696327",
        "b1rbnqkr/1pp1ppp1/2n4p/p2p4/5P2/1PBP4/P1P1P1PP/1NRBNQKR w HChc - 0 9 ;D1 26 ;D2 545 ;D3 14817 ;D4 336470 ;D5 9537260 ;D6 233549184",
        "1nrnqbkr/p1pppppp/1p6/8/2b2P2/P1N5/1PP1P1PP/BNR1QBKR w HChc - 2 9 ;D1 24 ;D2 668 ;D3 17716 ;D4 494866 ;D5 14216070 ;D6 406225409",
        "1nrnqkrb/2ppp1pp/p7/1p3p2/5P2/N5K1/PPPPP2P/B1RNQ1RB w gc - 0 9 ;D1 33 ;D2 725 ;D3 23572 ;D4 559823 ;D5 18547476 ;D6 471443091",
        "nbbr1qkr/p1pppppp/8/1p1n4/3P4/1N3PP1/PPP1P2P/1BBRNQKR w HDhd - 1 9 ;D1 28 ;D2 698 ;D3 20527 ;D4 539625 ;D5 16555068 ;D6 458045505",
        "1rbbnqkr/1pnppp1p/p5p1/2p5/2P4P/5P2/PP1PP1PR/NRBBNQK1 w Bhb - 1 9 ;D1 24 ;D2 554 ;D3 14221 ;D4 362516 ;D5 9863080 ;D6 269284081",
        "nrb1qbkr/2pppppp/2n5/p7/2p5/4P3/PPNP1PPP/1RBNQBKR w HBhb - 0 9 ;D1 23 ;D2 618 ;D3 15572 ;D4 443718 ;D5 12044358 ;D6 360311412",
        "nrb1qkrb/2ppppp1/p3n3/1p1B3p/2P5/6P1/PP1PPPRP/NRBNQK2 w Bgb - 2 9 ;D1 27 ;D2 593 ;D3 16770 ;D4 401967 ;D5 11806808 ;D6 303338935",
        "nbrn1qkr/ppp1pp2/3p2p1/3Q3P/b7/8/PPPPPP1P/NBRNB1KR w HChc - 2 9 ;D1 39 ;D2 1056 ;D3 40157 ;D4 1133446 ;D5 42201531 ;D6 1239888683",
        "nr1bbqkr/pp1pp2p/1n3pp1/2p5/8/1P4P1/P1PPPPQP/NRNBBK1R w hb - 0 9 ;D1 25 ;D2 585 ;D3 15719 ;D4 406544 ;D5 11582539 ;D6 320997679",
        "nr2bbkr/ppp1pppp/1n1p4/8/6PP/1NP4q/PP1PPP2/1RNQBBKR w HBhb - 1 9 ;D1 22 ;D2 742 ;D3 15984 ;D4 545231 ;D5 13287051 ;D6 457010195",
        "1rnqbkrb/ppp1p1p1/1n3p2/3p3p/P6P/4P3/1PPP1PP1/NRNQBRKB w gb - 0 9 ;D1 22 ;D2 574 ;D3 14044 ;D4 379648 ;D5 9968830 ;D6 281344367",
        "nb1rqkbr/1pppp1pp/4n3/p4p2/6PP/5P2/PPPPPN2/NBR1QKBR w HCh - 0 9 ;D1 25 ;D2 621 ;D3 16789 ;D4 462600 ;D5 13378840 ;D6 396575613",
        "nrnbqkbr/2pp2pp/4pp2/pp6/8/1P3P2/P1PPPBPP/NRNBQ1KR w hb - 0 9 ;D1 25 ;D2 656 ;D3 16951 ;D4 466493 ;D5 12525939 ;D6 358763789",
        "nrnqkbbr/ppppp1p1/7p/5p2/8/P4PP1/NPPPP2P/NR1QKBBR w HBhb - 0 9 ;D1 28 ;D2 723 ;D3 20621 ;D4 547522 ;D5 15952533 ;D6 439046803",
        "1rnqkr1b/ppppp2p/1n3pp1/8/2P3P1/Pb1N4/1P1PPP1P/NR1QKRBB w FBfb - 0 9 ;D1 26 ;D2 713 ;D3 19671 ;D4 548875 ;D5 15865528 ;D6 454532806",
        "bbnrnkqr/1pppp1pp/5p2/p7/7P/1P6/PBPPPPPR/1BNRNKQ1 w D - 2 9 ;D1 26 ;D2 649 ;D3 17834 ;D4 502279 ;D5 14375839 ;D6 435585252",
        "bnrbk1qr/1ppp1ppp/p2np3/8/P7/2N2P2/1PPPP1PP/B1RBNKQR w HC - 0 9 ;D1 26 ;D2 621 ;D3 17569 ;D4 451452 ;D5 13514201 ;D6 364421088",
        "br1nkbqr/ppppppp1/8/n6p/8/N1P2PP1/PP1PP2P/B1RNKBQR w HCh - 1 9 ;D1 29 ;D2 664 ;D3 20182 ;D4 512316 ;D5 16125924 ;D6 442508159",
        "bnr1kqrb/pp1pppp1/2n5/2p5/1P4Pp/4N3/P1PPPP1P/BNKR1QRB w gc - 0 9 ;D1 36 ;D2 888 ;D3 31630 ;D4 789863 ;D5 27792175 ;D6 719015345",
        "1bbrnkqr/pp1p1ppp/2p1p3/1n6/5P2/3Q4/PPPPP1PP/NBBRNK1R w HDhd - 2 9 ;D1 36 ;D2 891 ;D3 31075 ;D4 781792 ;D5 26998966 ;D6 702903862",
        "nrbbnk1r/pp2pppq/8/2pp3p/3P2P1/1N6/PPP1PP1P/1RBBNKQR w HBhb - 0 9 ;D1 29 ;D2 1036 ;D3 31344 ;D4 1139166 ;D5 35627310 ;D6 1310683359",
        "nr1nkbqr/ppp3pp/5p2/3pp3/6b1/3PP3/PPP2PPP/NRBNKBQR w hb - 0 9 ;D1 18 ;D2 664 ;D3 13306 ;D4 483892 ;D5 10658989 ;D6 386307449",
        "nrbnk1rb/ppp1pq1p/3p4/5pp1/2P1P3/1N6/PP1PKPPP/1RBN1QRB w gb - 2 9 ;D1 25 ;D2 966 ;D3 24026 ;D4 920345 ;D5 23957242 ;D6 913710194",
        "1brnbkqr/pppppp2/6p1/7p/1Pn5/P1NP4/2P1PPPP/NBR1BKQR w HChc - 0 9 ;D1 22 ;D2 627 ;D3 13760 ;D4 395829 ;D5 9627826 ;D6 285900573",
        "nrnbbk1r/p1pppppq/8/7p/1p6/P5PP/1PPPPPQ1/NRNBBK1R w HBhb - 2 9 ;D1 29 ;D2 888 ;D3 26742 ;D4 874270 ;D5 27229468 ;D6 930799376",
        "n1nkb1qr/prppppbp/6p1/1p6/2P2P2/P7/1P1PP1PP/NRNKBBQR w HBh - 1 9 ;D1 29 ;D2 804 ;D3 24701 ;D4 688520 ;D5 21952444 ;D6 623156747",
        "nr2bqrb/ppkpp1pp/1np5/5p1P/5P2/2P5/PP1PP1P1/NRNKBQRB w GB - 0 9 ;D1 22 ;D2 530 ;D3 13055 ;D4 347657 ;D5 9244693 ;D6 264088392",
        "nbr1kqbr/p3pppp/2ppn3/1p4P1/4P3/1P6/P1PP1P1P/NBRNKQBR w HChc - 1 9 ;D1 23 ;D2 555 ;D3 14291 ;D4 350917 ;D5 9692630 ;D6 247479180",
        "nr1bkqbr/1p1pp1pp/pnp2p2/8/6P1/P1PP4/1P2PP1P/NRNBKQBR w HBhb - 0 9 ;D1 22 ;D2 565 ;D3 13343 ;D4 365663 ;D5 9305533 ;D6 268612479",
        "nr1kqbbr/np2pppp/p1p5/1B1p1P2/8/4P3/PPPP2PP/NRNKQ1BR w HBhb - 0 9 ;D1 32 ;D2 730 ;D3 23391 ;D4 556995 ;D5 18103280 ;D6 454569900",
        "nrnk1rbb/p1p2ppp/3pq3/Qp2p3/1P1P4/8/P1P1PPPP/NRN1KRBB w fb - 2 9 ;D1 28 ;D2 873 ;D3 25683 ;D4 791823 ;D5 23868737 ;D6 747991356",
        "bbnrnkrq/pp1ppp1p/6p1/2p5/6P1/P5RP/1PPPPP2/BBNRNK1Q w Dgd - 3 9 ;D1 37 ;D2 1260 ;D3 45060 ;D4 1542086 ;D5 54843403 ;D6 1898432768",
        "bnrb1rkq/ppnpppp1/3Q4/2p4p/7P/N7/PPPPPPP1/B1RBNKR1 w GC - 2 9 ;D1 38 ;D2 878 ;D3 31944 ;D4 800440 ;D5 28784300 ;D6 784569826",
        "bnrnkbrq/p1ppppp1/1p5p/8/P2PP3/5P2/1PP3PP/BNRNKBRQ w GCgc - 1 9 ;D1 26 ;D2 617 ;D3 16992 ;D4 419099 ;D5 11965544 ;D6 311309576",
        "bnrnkrqb/pp2p2p/2pp1pp1/8/P7/2PP1P2/1P2P1PP/BNRNKRQB w FCfc - 0 9 ;D1 26 ;D2 721 ;D3 19726 ;D4 560824 ;D5 15966934 ;D6 467132503",
        "nbbrnkr1/1pppp1p1/p6q/P4p1p/8/5P2/1PPPP1PP/NBBRNRKQ w gd - 2 9 ;D1 18 ;D2 556 ;D3 10484 ;D4 316634 ;D5 6629293 ;D6 202528241",
        "nrb1nkrq/2pp1ppp/p4b2/1p2p3/P4B2/3P4/1PP1PPPP/NR1BNRKQ w gb - 0 9 ;D1 24 ;D2 562 ;D3 14017 ;D4 355433 ;D5 9227883 ;D6 247634489",
        "nrbnkbrq/p3p1pp/1p6/2pp1P2/8/3PP3/PPP2P1P/NRBNKBRQ w GBgb - 0 9 ;D1 31 ;D2 746 ;D3 24819 ;D4 608523 ;D5 21019301 ;D6 542954168",
        "nrbnkrqb/pppp1p1p/4p1p1/8/7P/2P1P3/PPNP1PP1/1RBNKRQB w FBfb - 0 9 ;D1 20 ;D2 459 ;D3 9998 ;D4 242762 ;D5 5760165 ;D6 146614723",
        "nbrn1krq/ppp1p2p/6b1/3p1pp1/8/4N1PP/PPPPPP2/NBR1BRKQ w gc - 1 9 ;D1 27 ;D2 835 ;D3 23632 ;D4 766397 ;D5 22667987 ;D6 760795567",
        "nrnbbkrq/p1pp2pp/5p2/1p6/2P1pP1B/1P6/P2PP1PP/NRNB1KRQ w GBgb - 0 9 ;D1 24 ;D2 646 ;D3 16102 ;D4 444472 ;D5 11489727 ;D6 324948755",
        "nrn1bbrq/1ppkppp1/p2p3p/8/1P3N2/4P3/P1PP1PPP/NR1KBBRQ w GB - 2 9 ;D1 32 ;D2 591 ;D3 18722 ;D4 381683 ;D5 12069159 ;D6 269922838",
        "n1krbrqb/1ppppppp/p7/8/4n3/P4P1P/1PPPPQP1/NRNKBR1B w FB - 2 9 ;D1 26 ;D2 639 ;D3 16988 ;D4 417190 ;D5 12167153 ;D6 312633873",
        "n1rnkrbq/1p1ppp1p/8/p1p1b1p1/3PQ1P1/4N3/PPP1PP1P/NBR1KRB1 w FCfc - 0 9 ;D1 35 ;D2 1027 ;D3 35731 ;D4 1040417 ;D5 35738410 ;D6 1060661628",
        "nrnbkrbq/2pp1pp1/pp6/4p2p/P7/5PPP/1PPPP3/NRNBKRBQ w FBfb - 0 9 ;D1 26 ;D2 628 ;D3 16731 ;D4 436075 ;D5 11920087 ;D6 331498921",
        "1rnkrbbq/pp1p2pp/1n3p2/1Bp1p3/1P6/1N2P3/P1PP1PPP/1RNKR1BQ w EBeb - 0 9 ;D1 33 ;D2 992 ;D3 32244 ;D4 983481 ;D5 31703749 ;D6 980306735",
        "nr1krqbb/p1ppppp1/8/1p5p/1Pn5/5P2/P1PPP1PP/NRNKRQBB w EBeb - 0 9 ;D1 24 ;D2 670 ;D3 15985 ;D4 445492 ;D5 11371067 ;D6 325556465",
        "bbq1rkr1/1ppppppp/p1n2n2/8/2P2P2/1P6/PQ1PP1PP/BB1NRKNR w HEe - 3 9 ;D1 32 ;D2 794 ;D3 26846 ;D4 689334 ;D5 24085223 ;D6 645633370",
        "b1nbrknr/1qppp1pp/p4p2/1p6/6P1/P2NP3/1PPP1P1P/BQ1BRKNR w HEhe - 1 9 ;D1 25 ;D2 663 ;D3 17138 ;D4 482994 ;D5 13157826 ;D6 389603029",
        "bqnrk1nr/pp2ppbp/6p1/2pp4/2P5/5P2/PPQPP1PP/B1NRKBNR w HDhd - 0 9 ;D1 26 ;D2 850 ;D3 22876 ;D4 759768 ;D5 21341087 ;D6 719712622",
        "bqnrknrb/1ppp1p1p/p7/6p1/1P2p3/P1PN4/3PPPPP/BQ1RKNRB w GDgd - 0 9 ;D1 25 ;D2 721 ;D3 19290 ;D4 581913 ;D5 16391601 ;D6 511725087",
        "q1b1rknr/pp1pppp1/4n2p/2p1b3/1PP5/4P3/PQ1P1PPP/1BBNRKNR w HEhe - 1 9 ;D1 32 ;D2 975 ;D3 32566 ;D4 955493 ;D5 32649943 ;D6 962536105",
        "qnbbrknr/1p1ppppp/8/p1p5/5P2/PP1P4/2P1P1PP/QNBBRKNR w HEhe - 0 9 ;D1 27 ;D2 573 ;D3 16331 ;D4 391656 ;D5 11562434 ;D6 301166330",
        "q1brkb1r/p1pppppp/np3B2/8/6n1/1P5N/P1PPPPPP/QN1RKB1R w HDhd - 0 9 ;D1 32 ;D2 984 ;D3 31549 ;D4 1007217 ;D5 32597704 ;D6 1075429389",
        "qn1rk1rb/p1pppppp/1p2n3/8/2b5/4NPP1/PPPPP1RP/QNBRK2B w Dgd - 4 9 ;D1 22 ;D2 802 ;D3 19156 ;D4 697722 ;D5 17761431 ;D6 650603534",
        "qbnrbknr/ppp2p1p/8/3pp1p1/1PP1B3/5N2/P2PPPPP/Q1NRBK1R w HDhd - 0 9 ;D1 34 ;D2 943 ;D3 32506 ;D4 930619 ;D5 32523099 ;D6 955802240",
        "qnrbb1nr/pp1p1ppp/2p2k2/4p3/4P3/5PPP/PPPP4/QNRBBKNR w HC - 0 9 ;D1 20 ;D2 460 ;D3 10287 ;D4 241640 ;D5 5846781 ;D6 140714047",
        "qnr1bbnr/ppk1p1pp/3p4/2p2p2/8/2P5/PP1PPPPP/QNKRBBNR w - - 1 9 ;D1 19 ;D2 572 ;D3 11834 ;D4 357340 ;D5 7994547 ;D6 243724815",
        "qnrkbnrb/1p1p1ppp/2p5/4p3/p7/N1BP4/PPP1PPPP/Q1R1KNRB w gc - 0 9 ;D1 27 ;D2 579 ;D3 16233 ;D4 375168 ;D5 10845146 ;D6 268229097",
        "qbnrkn1r/1pppp1p1/p3bp2/2BN3p/8/5P2/PPPPP1PP/QBNRK2R w HDhd - 0 9 ;D1 40 ;D2 1027 ;D3 38728 ;D4 1059229 ;D5 38511307 ;D6 1104094381",
        "qnrbknbr/1pp2ppp/4p3/p6N/2p5/8/PPPPPPPP/Q1RBK1BR w HChc - 0 9 ;D1 22 ;D2 510 ;D3 11844 ;D4 300180 ;D5 7403327 ;D6 200581103",
        "1qkrnbbr/p1pppppp/2n5/1p6/8/5NP1/PPPPPP1P/QNRK1BBR w HC - 4 9 ;D1 24 ;D2 549 ;D3 13987 ;D4 352037 ;D5 9396521 ;D6 255676649",
        "q1rknr1b/1ppppppb/2n5/p2B3p/8/1PN3P1/P1PPPP1P/Q1RKNRB1 w FCfc - 3 9 ;D1 31 ;D2 924 ;D3 28520 ;D4 861944 ;D5 27463479 ;D6 847726572",
        "bbnqrk1r/pp1pppp1/2p4p/8/6n1/1N1P1P2/PPP1P1PP/BBQ1RKNR w HEhe - 4 9 ;D1 24 ;D2 804 ;D3 20147 ;D4 666341 ;D5 18024195 ;D6 595947631",
        "bn1brknr/ppp1p1pp/5p2/3p4/6qQ/3P3P/PPP1PPP1/BN1BRKNR w HEhe - 4 9 ;D1 25 ;D2 854 ;D3 22991 ;D4 704173 ;D5 20290974 ;D6 600195008",
        "1nqrkbnr/2pp1ppp/pp2p3/3b4/2P5/N7/PP1PPPPP/B1QRKBNR w HDhd - 0 9 ;D1 22 ;D2 651 ;D3 16173 ;D4 479152 ;D5 13133439 ;D6 390886040",
        "bnqrk1rb/1pp1pppp/p2p4/4n3/2PPP3/8/PP3PPP/BNQRKNRB w GDgd - 1 9 ;D1 30 ;D2 950 ;D3 28169 ;D4 889687 ;D5 27610213 ;D6 880739164",
        "nbb1rknr/1ppq1ppp/3p4/p3p3/4P3/1N2R3/PPPP1PPP/1BBQ1KNR w Hhe - 2 9 ;D1 33 ;D2 988 ;D3 31293 ;D4 967575 ;D5 30894863 ;D6 985384035",
        "nqbbrknr/2ppp2p/pp4p1/5p2/7P/3P1P2/PPPBP1P1/NQ1BRKNR w HEhe - 0 9 ;D1 27 ;D2 492 ;D3 13266 ;D4 276569 ;D5 7583292 ;D6 175376176",
        "1qbrkb1r/pppppppp/8/3n4/4P1n1/PN6/1PPP1P1P/1QBRKBNR w HDhd - 3 9 ;D1 28 ;D2 800 ;D3 21982 ;D4 630374 ;D5 17313279 ;D6 507140861",
        "1qbrknrb/1p1ppppp/1np5/8/p4P1P/4P1N1/PPPP2P1/NQBRK1RB w GDgd - 0 9 ;D1 21 ;D2 482 ;D3 10581 ;D4 267935 ;D5 6218644 ;D6 168704845",
        "nbqrbkr1/ppp1pppp/8/3p4/6n1/2P2PPN/PP1PP2P/NBQRBK1R w HDd - 1 9 ;D1 29 ;D2 921 ;D3 25748 ;D4 840262 ;D5 24138518 ;D6 806554650",
        "nqrb1knr/1ppbpp1p/p7/3p2p1/2P3P1/5P1P/PP1PP3/NQRBBKNR w HChc - 1 9 ;D1 31 ;D2 803 ;D3 25857 ;D4 665799 ;D5 21998733 ;D6 583349773",
        "1qrkbbr1/pppp1ppp/1n3n2/4p3/5P2/1N6/PPPPP1PP/1QRKBBNR w HCc - 0 9 ;D1 25 ;D2 715 ;D3 19118 ;D4 556325 ;D5 15514933 ;D6 459533767",
        "nqrkb1rb/pp2pppp/2p1n3/3p4/3PP1N1/8/PPP2PPP/NQRKB1RB w GCgc - 0 9 ;D1 26 ;D2 795 ;D3 21752 ;D4 679387 ;D5 19185851 ;D6 616508881",
        "nb1rknbr/pp2ppp1/8/2Bp3p/6P1/2P2P1q/PP1PP2P/NBQRKN1R w HDhd - 0 9 ;D1 35 ;D2 1391 ;D3 43025 ;D4 1726888 ;D5 53033675 ;D6 2139267832",
        "nqrbkn1r/pp1pp1pp/8/2p2p2/5P2/P3B2P/1PbPP1P1/NQRBKN1R w HChc - 0 9 ;D1 23 ;D2 758 ;D3 19439 ;D4 653854 ;D5 18296195 ;D6 628403401",
        "nqrknbbr/pp1pppp1/7p/2p5/7P/1P1N4/P1PPPPPB/NQRK1B1R w HChc - 2 9 ;D1 29 ;D2 824 ;D3 23137 ;D4 683686 ;D5 19429491 ;D6 595493802",
        "1qrknrbb/B1p1pppp/8/1p1p4/2n2P2/1P6/P1PPP1PP/NQRKNR1B w FCfc - 0 9 ;D1 28 ;D2 771 ;D3 20237 ;D4 581721 ;D5 16065378 ;D6 483037840",
        "bbnrqk1r/1ppppppp/8/7n/1p6/P6P/1BPPPPP1/1BNRQKNR w HDhd - 0 9 ;D1 25 ;D2 601 ;D3 15471 ;D4 396661 ;D5 10697065 ;D6 289472497",
        "bnrbqknr/ppp3p1/3ppp1Q/7p/3P4/1P6/P1P1PPPP/BNRB1KNR w HChc - 0 9 ;D1 32 ;D2 845 ;D3 26876 ;D4 742888 ;D5 23717883 ;D6 682154649",
        "bn1qkb1r/pprppppp/8/2p5/2PPP1n1/8/PPR2PPP/BN1QKBNR w Hh - 1 9 ;D1 32 ;D2 856 ;D3 27829 ;D4 768595 ;D5 25245957 ;D6 727424329",
        "1nrqknrb/p1pp1ppp/1p2p3/3N4/5P1P/5b2/PPPPP3/B1RQKNRB w GCgc - 2 9 ;D1 33 ;D2 873 ;D3 27685 ;D4 779473 ;D5 25128076 ;D6 745401024",
        "nbbrqrk1/pppppppp/8/2N1n3/P7/6P1/1PPPPP1P/1BBRQKNR w HD - 3 9 ;D1 25 ;D2 555 ;D3 14339 ;D4 342296 ;D5 9153089 ;D6 234841945",
        "1rbbqknr/1ppp1pp1/1n2p3/p6p/4P1P1/P6N/1PPP1P1P/NRBBQK1R w HBhb - 0 9 ;D1 25 ;D2 693 ;D3 18652 ;D4 528070 ;D5 15133381 ;D6 439344945",
        "nrq1kbnr/p1pbpppp/3p4/1p6/6P1/1N3N2/PPPPPP1P/1RBQKB1R w HBhb - 4 9 ;D1 24 ;D2 648 ;D3 16640 ;D4 471192 ;D5 12871967 ;D6 380436777",
        "nr1qknr1/p1pppp1p/b5p1/1p6/8/P4PP1/1bPPP1RP/NRBQKN1B w Bgb - 0 9 ;D1 18 ;D2 533 ;D3 11215 ;D4 331243 ;D5 7777833 ;D6 234905172",
        "nbrqbknr/1ppp2pp/8/4pp2/p2PP1P1/7N/PPP2P1P/NBRQBK1R w HChc - 0 9 ;D1 29 ;D2 803 ;D3 24416 ;D4 706648 ;D5 22305910 ;D6 672322762",
        "nr1b1k1r/ppp1pppp/2bp1n2/6P1/2P3q1/5P2/PP1PP2P/NRQBBKNR w HBhb - 1 9 ;D1 27 ;D2 1199 ;D3 30908 ;D4 1296241 ;D5 35121759 ;D6 1418677099",
        "nrqkbbnr/2pppp1p/p7/1p6/2P1Pp2/8/PPNP2PP/1RQKBBNR w HBhb - 0 9 ;D1 28 ;D2 613 ;D3 17874 ;D4 432750 ;D5 13097064 ;D6 345294379",
        "1rqkbnrb/pp1ppp1p/1n4p1/B1p5/3PP3/4N3/PPP2PPP/NRQK2RB w GBgb - 0 9 ;D1 33 ;D2 723 ;D3 23991 ;D4 590970 ;D5 19715083 ;D6 535650233",
        "nbrqkn1r/1pppp2p/5pp1/p2b4/5P2/P2PN3/1PP1P1PP/NBRQK1BR w HChc - 2 9 ;D1 23 ;D2 607 ;D3 15482 ;D4 400970 ;D5 11026383 ;D6 290708878",
        "nrqbknbr/pp1pppp1/8/2p4p/P3PP2/8/1PPP2PP/NRQBKNBR w HBhb - 1 9 ;D1 26 ;D2 700 ;D3 19371 ;D4 556026 ;D5 16058815 ;D6 485460242",
        "nrqknbbr/p2pppp1/1pp5/6Qp/3P4/1P3P2/P1P1P1PP/NR1KNBBR w HBhb - 0 9 ;D1 40 ;D2 905 ;D3 32932 ;D4 829746 ;D5 29263502 ;D6 791963709",
        "nrqknrbb/1p3ppp/p2p4/2p1p3/1P6/3PP1P1/P1P2P1P/NRQKNRBB w FBfb - 0 9 ;D1 29 ;D2 780 ;D3 22643 ;D4 654495 ;D5 19532077 ;D6 593181101",
        "1bnrkqnr/p1pppp2/7p/1p4p1/4b3/7N/PPPP1PPP/BBNRKQ1R w HDhd - 0 9 ;D1 25 ;D2 725 ;D3 19808 ;D4 565006 ;D5 16661676 ;D6 487354613",
        "bnrbkq1r/pp2p1pp/5n2/2pp1p2/P7/N1PP4/1P2PPPP/B1RBKQNR w HChc - 1 9 ;D1 24 ;D2 745 ;D3 18494 ;D4 584015 ;D5 15079602 ;D6 488924040",
        "2rkqbnr/p1pppppp/2b5/1pn5/1P3P1Q/2B5/P1PPP1PP/1NRK1BNR w HChc - 3 9 ;D1 33 ;D2 904 ;D3 30111 ;D4 840025 ;D5 28194726 ;D6 801757709",
        "bnrkqnrb/2pppp2/8/pp4pp/1P5P/6P1/P1PPPPB1/BNRKQNR1 w GCgc - 0 9 ;D1 34 ;D2 1059 ;D3 34090 ;D4 1054311 ;D5 33195397 ;D6 1036498304",
        "1bbrkq1r/pppp2pp/1n2pp1n/8/2PP4/1N4P1/PP2PP1P/1BBRKQNR w HDhd - 1 9 ;D1 33 ;D2 891 ;D3 28907 ;D4 814247 ;D5 26970098 ;D6 788040469",
        "nrbbkqnr/1p2pp1p/p1p3p1/3p4/8/1PP5/P2PPPPP/NRBBKQNR w HBhb - 0 9 ;D1 21 ;D2 567 ;D3 13212 ;D4 376487 ;D5 9539687 ;D6 284426039",
        "1rbkqbr1/ppp1pppp/1n5n/3p4/3P4/1PP3P1/P3PP1P/NRBKQBNR w HBb - 1 9 ;D1 27 ;D2 752 ;D3 20686 ;D4 606783 ;D5 16986290 ;D6 521817800",
        "nrbkq1rb/1ppp1pp1/4p1n1/p6p/2PP4/5P2/PPK1P1PP/NRB1QNRB w gb - 0 9 ;D1 35 ;D2 697 ;D3 23678 ;D4 505836 ;D5 16906409 ;D6 390324794",
        "nbrkbqnr/p2pp1p1/5p2/1pp4p/7P/3P2P1/PPP1PP2/NBKRBQNR w hc - 0 9 ;D1 25 ;D2 679 ;D3 17223 ;D4 484921 ;D5 12879258 ;D6 376652259",
        "nrkb1qnr/ppppp1p1/6bp/5p2/1PP1P1P1/8/P2P1P1P/NRKBBQNR w HBhb - 1 9 ;D1 32 ;D2 761 ;D3 24586 ;D4 632916 ;D5 20671433 ;D6 568524724",
        "nrk1bbnr/p1q1pppp/1ppp4/8/3P3P/4K3/PPP1PPP1/NR1QBBNR w hb - 0 9 ;D1 30 ;D2 719 ;D3 21683 ;D4 541389 ;D5 16278120 ;D6 423649784",
        "nrkqbr1b/1pppp1pp/5pn1/p6N/1P3P2/8/P1PPP1PP/NRKQB1RB w GBb - 0 9 ;D1 26 ;D2 494 ;D3 13815 ;D4 296170 ;D5 8763742 ;D6 206993496",
        "nbrkq2r/pppp1bpp/4p1n1/5p2/7P/2P3N1/PP1PPPP1/NBKRQ1BR w hc - 0 9 ;D1 27 ;D2 701 ;D3 19536 ;D4 535052 ;D5 15394667 ;D6 443506342",
        "nrkbqnbr/2ppp2p/pp6/5pp1/P1P5/8/1P1PPPPP/NRKBQNBR w HBhb - 0 9 ;D1 21 ;D2 487 ;D3 11341 ;D4 285387 ;D5 7218486 ;D6 193586674",
        "nr1qnbbr/pk1pppp1/1pp4p/8/3P4/5P1P/PPP1P1P1/NRKQNBBR w HB - 0 9 ;D1 22 ;D2 546 ;D3 13615 ;D4 352855 ;D5 9587439 ;D6 259830255",
        "nrkq1rbb/pp1ppp1p/2pn4/8/PP3Pp1/7P/2PPP1P1/NRKQNRBB w FBfb - 0 9 ;D1 26 ;D2 839 ;D3 22075 ;D4 723845 ;D5 19867117 ;D6 658535326",
        "b2rknqr/pp1ppppp/8/2P5/n7/P7/1PPNPPPb/BBNRK1QR w HDhd - 2 9 ;D1 24 ;D2 699 ;D3 19523 ;D4 575172 ;D5 17734818 ;D6 535094237",
        "bnrbknqr/pp2p2p/2p3p1/3p1p2/8/3P4/PPPNPPPP/B1RBKNQR w HChc - 0 9 ;D1 23 ;D2 580 ;D3 14320 ;D4 385917 ;D5 10133092 ;D6 288041554",
        "bnrknb1r/pppp2pp/8/4pp2/6P1/3P3P/qPP1PPQ1/BNRKNB1R w HChc - 0 9 ;D1 28 ;D2 1100 ;D3 31813 ;D4 1217514 ;D5 36142423 ;D6 1361341249",
        "b1rknqrb/ppp1p1p1/2np1p1p/8/4N3/6PQ/PPPPPP1P/B1RKN1RB w GCgc - 0 9 ;D1 36 ;D2 629 ;D3 23082 ;D4 453064 ;D5 16897544 ;D6 367503974",
        "nb1rknqr/pbppp2p/6p1/1p3p2/5P2/3KP3/PPPP2PP/NBBR1NQR w hd - 2 9 ;D1 18 ;D2 557 ;D3 9779 ;D4 300744 ;D5 5822387 ;D6 180936551",
        "nr1bknqr/1ppb1ppp/p7/3pp3/B7/2P3NP/PP1PPPP1/NRB1K1QR w HBhb - 2 9 ;D1 28 ;D2 688 ;D3 19541 ;D4 519785 ;D5 15153092 ;D6 425149249",
        "nrbkn2r/pppp1pqp/4p1p1/8/3P2P1/P3B3/P1P1PP1P/NR1KNBQR w HBhb - 1 9 ;D1 32 ;D2 808 ;D3 25578 ;D4 676525 ;D5 22094260 ;D6 609377239",
        "nrbknqrb/2p1ppp1/1p6/p2p2Bp/1P6/3P1P2/P1P1P1PP/NR1KNQRB w GBgb - 0 9 ;D1 30 ;D2 625 ;D3 18288 ;D4 418895 ;D5 12225742 ;D6 301834282",
        "nbr1knqr/1pp1p1pp/3p1pb1/8/7P/5P2/PPPPPQP1/NBRKBN1R w HC - 2 9 ;D1 29 ;D2 863 ;D3 25767 ;D4 800239 ;D5 24965592 ;D6 799182442",
        "n1kbbnqr/prp2ppp/1p1p4/4p3/1P2P3/3P1B2/P1P2PPP/NRK1BNQR w HBh - 2 9 ;D1 26 ;D2 653 ;D3 17020 ;D4 449719 ;D5 12187583 ;D6 336872952",
        "nrknbbqr/pp3p1p/B3p1p1/2pp4/4P3/2N3P1/PPPP1P1P/NRK1B1QR w HBhb - 0 9 ;D1 29 ;D2 683 ;D3 19755 ;D4 501807 ;D5 14684565 ;D6 394951291",
        "n1knbqrb/pr1p1ppp/Qp6/2p1p3/4P3/6P1/PPPP1P1P/NRKNB1RB w GBg - 2 9 ;D1 31 ;D2 552 ;D3 17197 ;D4 371343 ;D5 11663330 ;D6 283583340",
        "nbrknqbr/p3p1pp/1p1p1p2/2p5/2Q1PP2/8/PPPP2PP/NBRKN1BR w HChc - 0 9 ;D1 37 ;D2 913 ;D3 32470 ;D4 825748 ;D5 28899548 ;D6 759875563",
        "nrkb1qbr/pp1pppp1/5n2/7p/2p5/1N1NPP2/PPPP2PP/1RKB1QBR w HBhb - 0 9 ;D1 25 ;D2 712 ;D3 18813 ;D4 543870 ;D5 15045589 ;D6 445074372",
        "nrk2bbr/pppqpppp/3p4/8/1P3nP1/3P4/P1P1PP1P/NRKNQBBR w HBhb - 1 9 ;D1 24 ;D2 814 ;D3 19954 ;D4 670162 ;D5 17603960 ;D6 592121050",
        "nrknqrbb/1p2ppp1/2pp4/Q6p/P2P3P/8/1PP1PPP1/NRKN1RBB w FBfb - 0 9 ;D1 34 ;D2 513 ;D3 16111 ;D4 303908 ;D5 9569590 ;D6 206509331",
        "bbnrk1rq/pp2p1pp/2ppn3/5p2/8/3NNP1P/PPPPP1P1/BB1RK1RQ w GDgd - 1 9 ;D1 28 ;D2 697 ;D3 20141 ;D4 517917 ;D5 15301879 ;D6 410843713",
        "bnrbknrq/ppppp2p/6p1/5p2/4QPP1/8/PPPPP2P/BNRBKNR1 w GCgc - 0 9 ;D1 37 ;D2 901 ;D3 32612 ;D4 877372 ;D5 31385912 ;D6 903831981",
        "bnkrnbrq/ppppp1p1/B6p/5p2/8/4P3/PPPP1PPP/BNKRN1RQ w - - 0 9 ;D1 26 ;D2 417 ;D3 11124 ;D4 217095 ;D5 5980981 ;D6 133080499",
        "bnrk1rqb/2pppp1p/3n4/pp4p1/3Q1P2/2N3P1/PPPPP2P/B1RKNR1B w FCfc - 0 9 ;D1 49 ;D2 1655 ;D3 74590 ;D4 2512003 ;D5 107234294 ;D6 3651608327",
        "nbbrk1rq/pp2pppp/2pp4/8/2P2n2/6N1/PP1PP1PP/NBBRKR1Q w Dgd - 0 9 ;D1 28 ;D2 960 ;D3 26841 ;D4 884237 ;D5 26083252 ;D6 846682836",
        "nrbb2rq/pppk1ppp/4p1n1/3p4/6P1/1BP5/PP1PPPQP/NRB1KNR1 w GB - 0 9 ;D1 28 ;D2 735 ;D3 22048 ;D4 593839 ;D5 18588316 ;D6 512048946",
        "nrbk1brq/p1ppppp1/7p/1p6/4P1nP/P7/1PPP1PP1/NRBKNBRQ w GBgb - 0 9 ;D1 22 ;D2 572 ;D3 12739 ;D4 351494 ;D5 8525056 ;D6 247615348",
        "nrbk1rqb/1pp2ppp/5n2/p2pp3/5B2/1N1P2P1/PPP1PP1P/1R1KNRQB w FBfb - 0 9 ;D1 35 ;D2 927 ;D3 31559 ;D4 849932 ;D5 28465693 ;D6 783048748",
        "nbrkb1rq/p1pp1ppp/4n3/4p3/Pp6/6N1/1PPPPPPP/NBRKBRQ1 w Cgc - 0 9 ;D1 20 ;D2 456 ;D3 10271 ;D4 247733 ;D5 6124625 ;D6 154766108",
        "nrkb1nrq/p2pp1pp/1pp2p2/7b/6PP/5P2/PPPPP2N/NRKBB1RQ w GBgb - 0 9 ;D1 21 ;D2 479 ;D3 11152 ;D4 264493 ;D5 6696458 ;D6 165253524",
        "nr1nbbr1/pppkpp1p/6p1/3p4/P6P/1P6/1RPPPPP1/N1KNBBRQ w G - 1 9 ;D1 20 ;D2 498 ;D3 11304 ;D4 288813 ;D5 7197322 ;D6 188021682",
        "nrknbrqb/3p1ppp/ppN1p3/8/6P1/8/PPPPPP1P/1RKNBRQB w FBfb - 0 9 ;D1 32 ;D2 526 ;D3 17267 ;D4 319836 ;D5 10755190 ;D6 220058991",
        "nbrkn1bq/p1pppr1p/1p6/5pp1/8/1N2PP2/PPPP2PP/1BKRNRBQ w c - 1 9 ;D1 19 ;D2 491 ;D3 10090 ;D4 277313 ;D5 6230616 ;D6 180748649",
        "nrkbnrbq/ppppppp1/8/8/7p/PP3P2/2PPPRPP/NRKBN1BQ w Bfb - 0 9 ;D1 16 ;D2 353 ;D3 6189 ;D4 156002 ;D5 3008668 ;D6 82706705",
        "nrknrbbq/p4ppp/2p1p3/1p1p4/1P2P3/2P5/P1NP1PPP/1RKNRBBQ w EBeb - 0 9 ;D1 29 ;D2 728 ;D3 21915 ;D4 587668 ;D5 18231199 ;D6 511686397",
        "nrknr1bb/pppp1p2/7p/2qPp1p1/8/1P5P/P1P1PPP1/NRKNRQBB w EBeb - 0 9 ;D1 20 ;D2 714 ;D3 14336 ;D4 500458 ;D5 11132758 ;D6 386064577",
        "bbqnrrkn/ppp2p1p/3pp1p1/8/1PP5/2Q5/P1BPPPPP/B2NRKRN w GE - 0 9 ;D1 39 ;D2 593 ;D3 23446 ;D4 424799 ;D5 16764576 ;D6 346185058",
        "bqn1rkrn/p1p2ppp/1p1p4/4p3/3PP2b/8/PPP2PPP/BQNBRKRN w GEge - 2 9 ;D1 25 ;D2 773 ;D3 20042 ;D4 616817 ;D5 16632403 ;D6 515838333",
        "bqnrkb1n/p1p1pprp/3p4/1p2P1p1/2PP4/8/PP3PPP/BQNRKBRN w GDd - 1 9 ;D1 31 ;D2 860 ;D3 28102 ;D4 810379 ;D5 27233018 ;D6 813751250",
        "bqr1krnb/ppppppp1/7p/3n4/1P4P1/P4N2/2PPPP1P/BQNRKR1B w FDf - 3 9 ;D1 31 ;D2 709 ;D3 22936 ;D4 559830 ;D5 18608857 ;D6 480498340",
        "qbbn1krn/pp3ppp/4r3/2ppp3/P1P4P/8/1P1PPPP1/QBBNRKRN w GEg - 1 9 ;D1 26 ;D2 775 ;D3 21100 ;D4 649673 ;D5 18476807 ;D6 582542257",
        "qnbbrkrn/1p1pp2p/p7/2p2pp1/8/4P2P/PPPP1PPK/QNBBRR1N w ge - 0 9 ;D1 25 ;D2 599 ;D3 15139 ;D4 389104 ;D5 10260500 ;D6 279222412",
        "qnbrkbrn/1ppp2p1/p3p2p/5p2/P4P2/1P6/2PPP1PP/QNBRKBRN w GDgd - 0 9 ;D1 27 ;D2 588 ;D3 16735 ;D4 394829 ;D5 11640416 ;D6 293541380",
        "1nbrkrnb/p1pppp1p/1pq3p1/8/4P3/P1P4N/1P1P1PPP/QNBRKR1B w FDfd - 1 9 ;D1 18 ;D2 609 ;D3 11789 ;D4 406831 ;D5 8604788 ;D6 299491047",
        "qb1r1krn/pppp2pp/1n2ppb1/4P3/7P/8/PPPP1PP1/QBNRBKRN w GDgd - 0 9 ;D1 20 ;D2 578 ;D3 12205 ;D4 349453 ;D5 7939483 ;D6 229142178",
        "qnr1bkrn/p3pppp/1bpp4/1p6/2P2PP1/8/PP1PPN1P/QNRBBKR1 w GCgc - 0 9 ;D1 30 ;D2 865 ;D3 26617 ;D4 771705 ;D5 24475596 ;D6 719842237",
        "1nkrbbrn/qppppppp/8/8/p2P4/1P5P/P1P1PPP1/QNKRBBRN w - - 0 9 ;D1 27 ;D2 672 ;D3 18371 ;D4 505278 ;D5 14065717 ;D6 410130412",
        "1qrkbrnb/ppp1p1pp/n2p4/5p2/4N3/8/PPPPPPPP/Q1RKBRNB w Ffc - 2 9 ;D1 25 ;D2 718 ;D3 18573 ;D4 536771 ;D5 14404324 ;D6 424279467",
        "q1nrkrbn/pp1pppp1/2p4p/8/P7/5Pb1/BPPPPNPP/Q1NRKRB1 w FDfd - 0 9 ;D1 22 ;D2 558 ;D3 12911 ;D4 336042 ;D5 8516966 ;D6 228074630",
        "qnrbkrbn/1p1p1pp1/p1p5/4p2p/8/3P1P2/PPP1P1PP/QNRBKRBN w FCfc - 0 9 ;D1 28 ;D2 669 ;D3 17713 ;D4 440930 ;D5 12055174 ;D6 313276304",
        "qnrkr1bn/p1pp1ppp/8/1p2p3/3P1P2/bP4P1/P1P1P2P/QNRKRBBN w ECec - 1 9 ;D1 23 ;D2 845 ;D3 20973 ;D4 759778 ;D5 19939053 ;D6 718075943",
        "q1krrnbb/p1p1pppp/2np4/1pB5/5P2/8/PPPPP1PP/QNRKRN1B w EC - 0 9 ;D1 29 ;D2 776 ;D3 21966 ;D4 631941 ;D5 18110831 ;D6 549019739",
        "bbn1rkrn/pp1p1ppp/8/2p1p1q1/6P1/P7/BPPPPP1P/B1NQRKRN w GEge - 0 9 ;D1 26 ;D2 936 ;D3 25177 ;D4 906801 ;D5 24984621 ;D6 901444251",
        "bn1brkrn/pp1qpp1p/2p3p1/3p4/1PPP4/P7/4PPPP/BNQBRKRN w GEge - 1 9 ;D1 29 ;D2 755 ;D3 22858 ;D4 645963 ;D5 20128587 ;D6 600207069",
        "b2rkbrn/p1pppppp/qp6/8/1n6/2B2P2/P1PPP1PP/1NQRKBRN w GDgd - 0 9 ;D1 24 ;D2 878 ;D3 21440 ;D4 791007 ;D5 20840078 ;D6 775795187",
        "b2rkrnb/pqp1pppp/n7/1p1p4/P7/N1P2N2/1P1PPPPP/B1QRKR1B w FDfd - 4 9 ;D1 26 ;D2 724 ;D3 19558 ;D4 571891 ;D5 16109522 ;D6 492933398",
        "1bbqrkrn/ppppp1p1/8/5p1p/P1n3P1/3P4/1PP1PP1P/NBBQRRKN w ge - 1 9 ;D1 25 ;D2 678 ;D3 17351 ;D4 461211 ;D5 12173245 ;D6 329661421",
        "nqb1rrkn/ppp1bppp/3pp3/8/3P4/1P6/PQP1PPPP/N1BBRRKN w - - 1 9 ;D1 23 ;D2 503 ;D3 12465 ;D4 290341 ;D5 7626054 ;D6 188215608",
        "nqbrkbr1/p1pppppp/1p6/2N2n2/2P5/5P2/PP1PP1PP/1QBRKBRN w GDgd - 1 9 ;D1 29 ;D2 688 ;D3 20289 ;D4 506302 ;D5 15167248 ;D6 399015237",
        "nqbrkrn1/1ppppp2/6pp/p7/1P6/2Q5/P1PPPPPP/N1BRKRNB w FDfd - 0 9 ;D1 36 ;D2 602 ;D3 20985 ;D4 397340 ;D5 13706856 ;D6 291708797",
        "nbqrbrkn/pp1p1pp1/2p5/4p2p/2P3P1/1P3P2/P2PP2P/NBQRBKRN w GD - 0 9 ;D1 34 ;D2 655 ;D3 22581 ;D4 474396 ;D5 16613630 ;D6 379344541",
        "nqrbbrkn/1p1pppp1/8/p1p4p/4P2P/1N4P1/PPPP1P2/1QRBBKRN w GC - 0 9 ;D1 23 ;D2 597 ;D3 14468 ;D4 400357 ;D5 10096863 ;D6 294900903",
        "nqrkbbrn/2p1p1pp/pp1p1p2/8/P2N4/2P5/1P1PPPPP/1QRKBBRN w GCgc - 0 9 ;D1 32 ;D2 744 ;D3 23310 ;D4 550728 ;D5 17597164 ;D6 428786656",
        "n1krbrnb/q1pppppp/p7/1p6/3Q4/2P2P2/PP1PP1PP/N1RKBRNB w FC - 1 9 ;D1 43 ;D2 1038 ;D3 41327 ;D4 1074450 ;D5 40918952 ;D6 1126603824",
        "nb1rkrbn/p1pp1p1p/qp6/4p1p1/5PP1/P7/1PPPPB1P/NBQRKR1N w FDfd - 2 9 ;D1 26 ;D2 645 ;D3 16463 ;D4 445464 ;D5 11911314 ;D6 342563372",
        "nqr1krbn/pppp1ppp/8/8/3pP3/5P2/PPPb1NPP/NQRBKRB1 w FCfc - 3 9 ;D1 2 ;D2 51 ;D3 1047 ;D4 27743 ;D5 612305 ;D6 17040200",
        "n1rkrbbn/pqppppp1/7p/1p6/8/1NPP4/PP1KPPPP/1QR1RBBN w ec - 0 9 ;D1 25 ;D2 674 ;D3 17553 ;D4 505337 ;D5 13421727 ;D6 403551903",
        "1qrkrnbb/1p1p1ppp/pnp1p3/8/3PP3/P6P/1PP2PP1/NQRKRNBB w ECec - 0 9 ;D1 24 ;D2 688 ;D3 17342 ;D4 511444 ;D5 13322502 ;D6 403441498",
        "1bnrqkrn/2ppppp1/p7/1p1b3p/3PP1P1/8/PPPQ1P1P/BBNR1KRN w GDgd - 1 9 ;D1 35 ;D2 925 ;D3 32238 ;D4 857060 ;D5 30458921 ;D6 824344087",
        "bnrbqkr1/ppp2pp1/6n1/3pp2p/1P6/2N3N1/P1PPPPPP/B1RBQRK1 w gc - 0 9 ;D1 23 ;D2 704 ;D3 17345 ;D4 539587 ;D5 14154852 ;D6 450893738",
        "1nrqkbrn/p1pppppp/8/1p1b4/P6P/5P2/1PPPP1P1/BNRQKBRN w GCgc - 1 9 ;D1 19 ;D2 505 ;D3 10619 ;D4 281422 ;D5 6450025 ;D6 175593967",
        "b1rqkrnb/ppppppp1/8/6p1/3n4/NP6/P1PPPP1P/B1RQKRNB w FCfc - 0 9 ;D1 25 ;D2 614 ;D3 15578 ;D4 377660 ;D5 10391021 ;D6 259629603",
        "nbbrqkrn/ppp3p1/3pp3/5p1p/1P2P3/P7/2PPQPPP/NBBR1KRN w GDgd - 0 9 ;D1 30 ;D2 833 ;D3 25719 ;D4 717713 ;D5 22873901 ;D6 649556666",
        "nr1bqrk1/ppp1pppp/6n1/3pP3/8/5PQb/PPPP2PP/NRBB1KRN w GB - 3 9 ;D1 26 ;D2 734 ;D3 20161 ;D4 582591 ;D5 17199594 ;D6 512134836",
        "1rbqkbr1/ppppp1pp/1n6/4np2/3P1P2/6P1/PPPQP2P/NRB1KBRN w GBgb - 1 9 ;D1 27 ;D2 662 ;D3 17897 ;D4 447464 ;D5 13038519 ;D6 338365642",
        "nr1qkr1b/ppp1pp1p/4bn2/3p2p1/4P3/1Q6/PPPP1PPP/NRB1KRNB w FBfb - 4 9 ;D1 33 ;D2 939 ;D3 30923 ;D4 942138 ;D5 30995969 ;D6 991509814",
        "nb1qbkrn/pprp1pp1/7p/2p1pB2/Q1PP4/8/PP2PPPP/N1R1BKRN w GCg - 2 9 ;D1 47 ;D2 1128 ;D3 50723 ;D4 1306753 ;D5 56747878 ;D6 1560584212",
        "nrqb1rkn/pp2pppp/2bp4/2p5/6P1/2P3N1/PP1PPP1P/NRQBBRK1 w - - 3 9 ;D1 24 ;D2 828 ;D3 21148 ;D4 723705 ;D5 19506135 ;D6 668969549",
        "nrq1bbrn/ppkpp2p/2p3p1/P4p2/8/4P1N1/1PPP1PPP/NRQKBBR1 w GB - 0 9 ;D1 25 ;D2 525 ;D3 13533 ;D4 309994 ;D5 8250997 ;D6 201795680",
        "Br1kbrn1/pqpppp2/8/6pp/3b2P1/1N6/PPPPPP1P/1RQKBRN1 w FBfb - 3 9 ;D1 20 ;D2 790 ;D3 18175 ;D4 695905 ;D5 17735648 ;D6 669854148",
        "nbrqkrbn/2p1p1pp/p7/1p1p1p2/4P1P1/5P2/PPPP3P/NBRQKRBN w FCfc - 0 9 ;D1 29 ;D2 771 ;D3 22489 ;D4 647106 ;D5 19192982 ;D6 591335970",
        "1rqbkrbn/1ppppp1p/1n6/p1N3p1/8/2P4P/PP1PPPP1/1RQBKRBN w FBfb - 0 9 ;D1 29 ;D2 502 ;D3 14569 ;D4 287739 ;D5 8652810 ;D6 191762235",
        "1rqkrbbn/ppnpp1pp/8/2p5/6p1/3P4/PPP1PPPP/NRK1RBBN w eb - 0 9 ;D1 19 ;D2 531 ;D3 10812 ;D4 300384 ;D5 6506674 ;D6 184309316",
        "nrqkrnbb/p1pp2pp/5p2/4P3/2p5/4N3/PP1PP1PP/NRQKR1BB w EBeb - 0 9 ;D1 26 ;D2 800 ;D3 23256 ;D4 756695 ;D5 23952941 ;D6 809841274",
        "bbnrkqrn/pp3pp1/4p2p/2pp4/4P1P1/1PB5/P1PP1P1P/1BNRKQRN w GDgd - 0 9 ;D1 33 ;D2 915 ;D3 30536 ;D4 878648 ;D5 29602610 ;D6 881898159",
        "bnrbkqr1/1p2pppp/6n1/p1pp4/7P/P3P3/1PPPKPP1/BNRB1QRN w gc - 0 9 ;D1 19 ;D2 457 ;D3 9332 ;D4 238944 ;D5 5356253 ;D6 144653627",
        "b1rkqbrn/pp1p2pp/2n1p3/2p2p2/3P2PP/8/PPP1PP2/BNKRQBRN w gc - 0 9 ;D1 30 ;D2 985 ;D3 30831 ;D4 1011700 ;D5 32684185 ;D6 1080607773",
        "b1rkqrnb/2ppppp1/np6/p6p/1P6/P2P3P/2P1PPP1/BNRKQRNB w FCfc - 0 9 ;D1 26 ;D2 692 ;D3 18732 ;D4 517703 ;D5 14561181 ;D6 413226841",
        "nbbrkqrn/1ppp1p2/p6p/4p1p1/5P2/1P5P/P1PPPNP1/NBBRKQR1 w GDgd - 0 9 ;D1 22 ;D2 561 ;D3 13222 ;D4 367487 ;D5 9307003 ;D6 273928315",
        "nrbbkqrn/p1pppppp/8/1p6/4P3/7Q/PPPP1PPP/NRBBK1RN w GBgb - 0 9 ;D1 38 ;D2 769 ;D3 28418 ;D4 632310 ;D5 23091070 ;D6 560139600",
        "nrbkqbrn/1pppp2p/8/p4pp1/P4PQ1/8/1PPPP1PP/NRBK1BRN w GBgb - 0 9 ;D1 23 ;D2 507 ;D3 13067 ;D4 321423 ;D5 8887567 ;D6 237475184",
        "nr1kqr1b/pp2pppp/5n2/2pp4/P5b1/5P2/1PPPPRPP/NRBK1QNB w Bfb - 2 9 ;D1 18 ;D2 626 ;D3 12386 ;D4 434138 ;D5 9465555 ;D6 335004239",
        "nbkrbqrn/1pppppp1/8/4P2p/pP6/P7/2PP1PPP/NBRKBQRN w GC - 0 9 ;D1 22 ;D2 329 ;D3 8475 ;D4 148351 ;D5 4160034 ;D6 82875306",
        "nrkb1qrn/pp1pp1pp/8/5p1b/P1p4P/6N1/1PPPPPP1/NRKBBQR1 w GBgb - 2 9 ;D1 16 ;D2 479 ;D3 9037 ;D4 275354 ;D5 5862341 ;D6 184959796",
        "1rkq1brn/ppppp1pp/1n6/3b1p2/3N3P/5P2/PPPPP1P1/1RKQBBRN w GBgb - 3 9 ;D1 23 ;D2 614 ;D3 15324 ;D4 418395 ;D5 11090645 ;D6 313526088",
        "nrk1brnb/pp1ppppp/2p5/3q4/5P2/PP6/1KPPP1PP/NR1QBRNB w fb - 1 9 ;D1 25 ;D2 942 ;D3 21765 ;D4 792179 ;D5 19318837 ;D6 685549171",
        "nbrkqr1n/1pppp2p/p4pp1/2Bb4/5P2/6P1/PPPPP2P/NBRKQ1RN w Cfc - 2 9 ;D1 30 ;D2 841 ;D3 24775 ;D4 677876 ;D5 20145765 ;D6 557578726",
        "n1kbqrbn/2p1pppp/1r6/pp1p4/P7/3P4/1PP1PPPP/NRKBQRBN w FBf - 2 9 ;D1 21 ;D2 591 ;D3 14101 ;D4 394289 ;D5 10295086 ;D6 292131422",
        "nrkqrbb1/ppp1pppp/3p4/8/4P3/2Pn1P2/PP4PP/NRKQRBBN w EBeb - 0 9 ;D1 4 ;D2 88 ;D3 3090 ;D4 73414 ;D5 2640555 ;D6 66958031",
        "nrkqrnbb/ppppp1p1/7p/1P3p2/3P4/2P5/P3PPPP/NRKQRNBB w EBeb - 0 9 ;D1 29 ;D2 689 ;D3 21091 ;D4 508789 ;D5 16226660 ;D6 408570219",
        "bbnr1rqn/pp2pkpp/2pp1p2/8/4P1P1/8/PPPP1P1P/BBNRKRQN w FD - 0 9 ;D1 21 ;D2 463 ;D3 11135 ;D4 256244 ;D5 6826249 ;D6 165025370",
        "bnrbk1qn/1pppprpp/8/p4p1P/6P1/3P4/PPP1PP2/BNRBKRQN w FCc - 0 9 ;D1 22 ;D2 459 ;D3 11447 ;D4 268157 ;D5 7371098 ;D6 190583454",
        "1nrkrbqn/p1pp1ppp/4p3/1p6/1PP5/6PB/P2PPPbP/BNRKR1QN w ECec - 0 9 ;D1 30 ;D2 931 ;D3 29012 ;D4 887414 ;D5 28412902 ;D6 869228014",
        "b1rkr1nb/pppppqp1/n4B2/7p/8/1P4P1/P1PPPP1P/1NKRRQNB w ec - 1 9 ;D1 36 ;D2 934 ;D3 31790 ;D4 930926 ;D5 30392925 ;D6 952871799",
        "nbbrkrqn/p1ppp1p1/8/1p3p1p/2P3PP/8/PP1PPPQ1/NBBRKR1N w FDfd - 0 9 ;D1 34 ;D2 938 ;D3 31848 ;D4 921716 ;D5 31185844 ;D6 944483246",
        "1rbbkrqn/ppp1pp2/1n1p2p1/7p/P3P1P1/3P4/1PP2P1P/NRBBKRQN w FBfb - 0 9 ;D1 26 ;D2 646 ;D3 18083 ;D4 472744 ;D5 14006203 ;D6 384101783",
        "nrbkrbq1/Qpppp1pp/2n5/5p2/P4P2/6N1/1PPPP1PP/NRBKRB2 w EBeb - 1 9 ;D1 27 ;D2 619 ;D3 16713 ;D4 421845 ;D5 11718463 ;D6 313794027",
        "1rbkr1nb/pppp1qpp/1n6/4pp2/1PP1P3/8/PB1P1PPP/NR1KRQNB w EBeb - 1 9 ;D1 32 ;D2 1029 ;D3 32970 ;D4 1080977 ;D5 35483796 ;D6 1181835398",
        "nbrk1rqn/p1ppp2p/1p6/5ppb/8/1N2P2P/PPPP1PP1/1BKRBRQN w fc - 0 9 ;D1 18 ;D2 594 ;D3 12350 ;D4 408544 ;D5 9329122 ;D6 315021712",
        "nrkbbrqn/3pppp1/7p/ppp5/P7/1N5P/1PPPPPP1/1RKBBRQN w FBfb - 0 9 ;D1 19 ;D2 417 ;D3 9026 ;D4 218513 ;D5 5236331 ;D6 137024458",
        "nrkr1bqn/ppp1pppp/3p4/1b6/7P/P7/1PPPPPP1/NRKRBBQN w DBdb - 1 9 ;D1 17 ;D2 457 ;D3 9083 ;D4 243872 ;D5 5503579 ;D6 150091997",
        "nrkrbqnb/p4ppp/1p2p3/2pp4/6P1/2P2N2/PPNPPP1P/1RKRBQ1B w DBdb - 0 9 ;D1 27 ;D2 755 ;D3 21012 ;D4 620093 ;D5 17883987 ;D6 547233320",
        "nbkrr1bn/ppB2ppp/4p3/2qp4/4P3/5P2/PPPP2PP/NBRKRQ1N w EC - 1 9 ;D1 37 ;D2 1473 ;D3 51939 ;D4 1956521 ;D5 68070015 ;D6 2490912491",
        "n1kbrqbn/p1pp1pp1/4p2p/2B5/1r3P2/8/PPPPP1PP/NRKBRQ1N w EBe - 2 9 ;D1 30 ;D2 1029 ;D3 30874 ;D4 1053163 ;D5 32318550 ;D6 1106487743",
        "nrkrqbbn/2pppp1p/8/pp6/1P1P2p1/P5P1/2P1PP1P/NRKRQBBN w DBdb - 0 9 ;D1 22 ;D2 421 ;D3 10034 ;D4 221927 ;D5 5754555 ;D6 141245633",
        "nrkr1nbb/1ppp2pp/p3q3/4pp2/2P5/P3P3/1PKP1PPP/NR1RQNBB w db - 0 9 ;D1 22 ;D2 619 ;D3 13953 ;D4 411392 ;D5 9905109 ;D6 301403003",
        "bbnrkrnq/1pp1p2p/6p1/p2p1p2/8/1P2P3/P1PP1PPP/BBNRKRNQ w FDfd - 0 9 ;D1 27 ;D2 805 ;D3 21915 ;D4 688224 ;D5 19133881 ;D6 620749189",
        "bnrbkrn1/pp1ppp2/2p3pp/8/2Pq4/P4PP1/1P1PP2P/BNRBKRNQ w FCfc - 1 9 ;D1 20 ;D2 770 ;D3 16593 ;D4 577980 ;D5 13581691 ;D6 456736500",
        "b1rkrbnq/1pp1pppp/2np4/p5N1/8/1P2P3/P1PP1PPP/BNRKRB1Q w ECec - 0 9 ;D1 37 ;D2 740 ;D3 27073 ;D4 581744 ;D5 21156664 ;D6 485803600",
        "b1krrnqb/pp1ppp1p/n1p3p1/2N5/6P1/8/PPPPPP1P/B1RKRNQB w EC - 0 9 ;D1 34 ;D2 850 ;D3 28494 ;D4 752350 ;D5 25360295 ;D6 698159474",
        "1bbr1rnq/ppppkppp/8/3np3/4P3/3P4/PPP1KPPP/NBBRR1NQ w - - 1 9 ;D1 27 ;D2 704 ;D3 18290 ;D4 480474 ;D5 12817011 ;D6 341026662",
        "nrbbk1nq/p1p1prpp/1p6/N2p1p2/P7/8/1PPPPPPP/R1BBKRNQ w Fb - 2 9 ;D1 23 ;D2 552 ;D3 13710 ;D4 348593 ;D5 9236564 ;D6 248469879",
        "1rbkrb1q/1pppp1pp/1n5n/p4p2/P3P3/1P6/2PPNPPP/NRBKRB1Q w EBeb - 1 9 ;D1 22 ;D2 415 ;D3 10198 ;D4 217224 ;D5 5735644 ;D6 135295774",
        "nrbkr1qb/1pp1pppp/6n1/p2p4/2P1P3/1N4N1/PP1P1PPP/1RBKR1QB w EBeb - 0 9 ;D1 27 ;D2 709 ;D3 19126 ;D4 506214 ;D5 14192779 ;D6 380516508",
        "nbrkbrnq/p3p1pp/1pp2p2/3p4/1PP5/4P3/P1KP1PPP/NBR1BRNQ w fc - 0 9 ;D1 24 ;D2 715 ;D3 18009 ;D4 535054 ;D5 14322279 ;D6 427269976",
        "nrk1brnq/pp1p1pp1/7p/b1p1p3/1P6/6P1/P1PPPPQP/NRKBBRN1 w FBfb - 2 9 ;D1 29 ;D2 675 ;D3 20352 ;D4 492124 ;D5 15316285 ;D6 389051744",
        "nrkr1bnq/1p2pppp/p2p4/1bp5/PP6/1R5N/2PPPPPP/N1KRBB1Q w Ddb - 2 9 ;D1 27 ;D2 744 ;D3 20494 ;D4 571209 ;D5 16188945 ;D6 458900901",
        "nrk1b1qb/pppn1ppp/3rp3/3p4/2P3P1/3P4/PPN1PP1P/1RKRBNQB w DBb - 3 9 ;D1 35 ;D2 941 ;D3 33203 ;D4 935791 ;D5 33150360 ;D6 968024386",
        "nb1rrnbq/ppkp1ppp/8/2p1p3/P7/1N2P3/1PPP1PPP/1BKRRNBQ w - - 1 9 ;D1 19 ;D2 451 ;D3 9655 ;D4 235472 ;D5 5506897 ;D6 139436165",
        "nrkbrnbq/4pppp/1ppp4/p7/2P1P3/3P2N1/PP3PPP/NRKBR1BQ w EBeb - 0 9 ;D1 29 ;D2 591 ;D3 17132 ;D4 384358 ;D5 11245508 ;D6 270967202",
        "nrkrnbbq/3p1ppp/1p6/p1p1p3/3P2P1/P4Q2/1PP1PP1P/NRKRNBB1 w DBdb - 0 9 ;D1 38 ;D2 792 ;D3 28597 ;D4 640961 ;D5 22654797 ;D6 540864616",
        "nr1rnqbb/ppp1pp1p/3k2p1/3p4/1P5P/3P1N2/P1P1PPP1/NRKR1QBB w DB - 1 9 ;D1 25 ;D2 758 ;D3 18547 ;D4 543643 ;D5 13890077 ;D6 402109399",
        "bbqrnnkr/1ppp1p1p/5p2/p5p1/P7/1P4P1/2PPPP1P/1BQRNNKR w HDhd - 0 9 ;D1 20 ;D2 322 ;D3 7224 ;D4 145818 ;D5 3588435 ;D6 82754650",
        "bqrb2k1/pppppppr/5nnp/8/3P1P2/4P1N1/PPP3PP/BQRBN1KR w HCc - 1 9 ;D1 25 ;D2 597 ;D3 15872 ;D4 397970 ;D5 11162476 ;D6 295682250",
        "bqrnn1kr/1pppbppp/8/4p3/1p6/2P1N2P/P2PPPP1/BQR1NBKR w HChc - 1 9 ;D1 34 ;D2 921 ;D3 31695 ;D4 864023 ;D5 30126510 ;D6 850296236",
        "bqr1nkr1/pppppp2/2n3p1/7p/1P1b1P2/8/PQP1P1PP/B1RNNKRB w GCgc - 0 9 ;D1 23 ;D2 788 ;D3 21539 ;D4 686795 ;D5 20849374 ;D6 645694580",
        "qbbrnn1r/1pppp1pk/p7/5p1p/P2P3P/3N4/1PP1PPP1/QBBR1NKR w HD - 0 9 ;D1 34 ;D2 713 ;D3 24475 ;D4 562189 ;D5 19494094 ;D6 482645160",
        "qrbb2kr/p1pppppp/1p1n4/8/1P3n2/P7/Q1PPP1PP/1RBBNNKR w HBhb - 0 9 ;D1 28 ;D2 977 ;D3 26955 ;D4 949925 ;D5 27802999 ;D6 992109168",
        "qrb2bkr/1pp1pppp/2np1n2/pN6/3P4/4B3/PPP1PPPP/QR2NBKR w HBhb - 0 9 ;D1 27 ;D2 730 ;D3 20534 ;D4 585091 ;D5 17005916 ;D6 507008968",
        "qrbnnkrb/pp2pp1p/8/2pp2p1/7P/P1P5/QP1PPPP1/1RBNNKRB w GBgb - 0 9 ;D1 24 ;D2 813 ;D3 21142 ;D4 707925 ;D5 19615756 ;D6 655850285",
        "1brnb1kr/p1pppppp/1p6/8/4q2n/1P2P1P1/PNPP1P1P/QBR1BNKR w HChc - 3 9 ;D1 17 ;D2 734 ;D3 13462 ;D4 530809 ;D5 11032633 ;D6 416356876",
        "1rnbbnkr/1pp1pppp/1q1p4/p7/4P3/5PN1/PPPP1BPP/QRNB2KR w HBhb - 1 9 ;D1 26 ;D2 809 ;D3 21764 ;D4 706677 ;D5 20292750 ;D6 675408811",
        "qrnnbb1Q/ppp1pk1p/3p2p1/5p2/PP6/5P2/2PPP1PP/1RNNBBKR w HB - 0 9 ;D1 37 ;D2 751 ;D3 27902 ;D4 603931 ;D5 22443036 ;D6 515122176",
        "qrnnbkrb/p3p1pp/3p1p2/1pp5/PP2P3/8/2PP1PPP/QRNNBRKB w gb - 0 9 ;D1 30 ;D2 906 ;D3 27955 ;D4 872526 ;D5 27658191 ;D6 890966633",
        "qbrnnkbr/1p2pp1p/p1p3p1/3p4/6P1/P1N4P/1PPPPP2/QBR1NKBR w HChc - 0 9 ;D1 26 ;D2 701 ;D3 18930 ;D4 521377 ;D5 14733245 ;D6 416881799",
        "qr1b1kbr/1p1ppppp/1n1n4/p1p5/4P3/5NPP/PPPP1P2/QRNB1KBR w HBhb - 1 9 ;D1 26 ;D2 649 ;D3 17235 ;D4 451997 ;D5 12367604 ;D6 342165821",
        "qrnnkb1r/1pppppp1/7p/p4b2/4P3/5P1P/PPPP2PR/QRNNKBB1 w Bhb - 1 9 ;D1 34 ;D2 941 ;D3 31720 ;D4 901240 ;D5 30307554 ;D6 888709821",
        "qr1nkrbb/p2ppppp/1pp5/8/3Pn3/1NP3P1/PP2PP1P/QR1NKRBB w FBfb - 1 9 ;D1 19 ;D2 505 ;D3 11107 ;D4 294251 ;D5 7046501 ;D6 190414579",
        "bbrqn1kr/1pppp1pp/4n3/5p2/p5P1/3P4/PPP1PPKP/BBRQNN1R w hc - 0 9 ;D1 24 ;D2 573 ;D3 12963 ;D4 335845 ;D5 8191054 ;D6 227555387",
        "brqb1nkr/pppppp1p/8/4N1pn/5P2/6P1/PPPPP2P/BRQB1NKR w HBhb - 0 9 ;D1 26 ;D2 550 ;D3 14338 ;D4 331666 ;D5 8903754 ;D6 223437427",
        "brqnn1kr/pp3ppp/2pbp3/3p4/8/2NPP3/PPP1BPPP/BRQ1N1KR w HBhb - 0 9 ;D1 27 ;D2 780 ;D3 20760 ;D4 589328 ;D5 16243731 ;D6 463883447",
        "brq1nkrb/ppp2ppp/8/n2pp2P/P7/4P3/1PPP1PP1/BRQNNKRB w GBgb - 1 9 ;D1 17 ;D2 426 ;D3 8295 ;D4 235162 ;D5 5048497 ;D6 153986034",
        "rbbqn1kr/pp2p1pp/6n1/2pp1p2/2P4P/P7/BP1PPPP1/R1BQNNKR w HAha - 0 9 ;D1 27 ;D2 916 ;D3 25798 ;D4 890435 ;D5 26302461 ;D6 924181432",
        "1qbbn1kr/1ppppppp/r3n3/8/p1P5/P7/1P1PPPPP/RQBBNNKR w HAh - 1 9 ;D1 29 ;D2 817 ;D3 24530 ;D4 720277 ;D5 22147642 ;D6 670707652",
        "rqbnnbkr/ppp1ppp1/7p/3p4/PP6/7P/1NPPPPP1/RQB1NBKR w HAa - 1 9 ;D1 23 ;D2 572 ;D3 14509 ;D4 381474 ;D5 10416981 ;D6 288064942",
        "r1bnnkrb/q1ppp1pp/p7/1p3pB1/2P1P3/3P4/PP3PPP/RQ1NNKRB w GAga - 2 9 ;D1 31 ;D2 925 ;D3 27776 ;D4 860969 ;D5 26316355 ;D6 843078864",
        "rbqnb1kr/ppppp1pp/5p2/5N2/7P/1n3P2/PPPPP1P1/RBQNB1KR w HAha - 1 9 ;D1 32 ;D2 864 ;D3 27633 ;D4 766551 ;D5 24738875 ;D6 707188107",
        "rqnbbn1r/ppppppp1/6k1/8/6Pp/2PN4/PP1PPPKP/RQ1BBN1R w - - 0 9 ;D1 27 ;D2 566 ;D3 15367 ;D4 347059 ;D5 9714509 ;D6 234622128",
        "rqnnbbkr/p1p2pp1/1p1p3p/4p3/4NP2/6P1/PPPPP2P/RQN1BBKR w HAha - 0 9 ;D1 27 ;D2 631 ;D3 17923 ;D4 452734 ;D5 13307890 ;D6 356279813",
        "1qnnbrkb/rppp1ppp/p3p3/8/4P3/2PP1P2/PP4PP/RQNNBKRB w GA - 1 9 ;D1 24 ;D2 479 ;D3 12135 ;D4 271469 ;D5 7204345 ;D6 175460841",
        "rbqnn1br/p1pppk1p/1p4p1/5p2/8/P1P2P2/1PBPP1PP/R1QNNKBR w HA - 0 9 ;D1 31 ;D2 756 ;D3 23877 ;D4 625194 ;D5 20036784 ;D6 554292502",
        "rqnbnkbr/1ppppp2/p5p1/8/1P4p1/4PP2/P1PP3P/RQNBNKBR w HAha - 0 9 ;D1 24 ;D2 715 ;D3 18536 ;D4 575589 ;D5 16013189 ;D6 515078271",
        "rq1nkbbr/1p2pppp/p2n4/2pp4/1P4P1/P2N4/2PPPP1P/RQ1NKBBR w HAha - 1 9 ;D1 27 ;D2 694 ;D3 19840 ;D4 552904 ;D5 16685687 ;D6 494574415",
        "r1nnkrbb/pp1pppp1/2p3q1/7p/8/1PPP3P/P3PPP1/RQNNKRBB w FAfa - 1 9 ;D1 18 ;D2 520 ;D3 10808 ;D4 329085 ;D5 7508201 ;D6 235103697",
        "bbrnqk1r/pppp3p/6p1/4pp2/3P2P1/8/PPP1PP1P/BBRN1NKR w HC - 0 9 ;D1 22 ;D2 566 ;D3 12965 ;D4 362624 ;D5 8721079 ;D6 259069471",
        "brnb1nkr/pppqpp2/3p2pp/8/3PP3/1P6/PBP2PPP/1RNBQNKR w HBhb - 0 9 ;D1 32 ;D2 859 ;D3 28517 ;D4 817464 ;D5 27734108 ;D6 829785474",
        "brnq1b1r/ppp1ppkp/3p1np1/8/8/5P1P/PPPPPKPR/BRNQNB2 w - - 0 9 ;D1 21 ;D2 511 ;D3 10951 ;D4 273756 ;D5 6372681 ;D6 167139732",
        "brnq1rkb/1pppppp1/3n3p/p7/8/P4NP1/1PPPPPRP/BRNQ1K1B w B - 0 9 ;D1 25 ;D2 548 ;D3 14049 ;D4 341208 ;D5 9015901 ;D6 235249649",
        "rbb1qnkr/p1ppp1pp/1p3p2/6n1/8/1PN1P2P/P1PP1PP1/RBB1QNKR w HAha - 0 9 ;D1 25 ;D2 673 ;D3 16412 ;D4 467660 ;D5 12099119 ;D6 361714466",
        "rnbb1nkr/1ppp1ppp/4p3/p5q1/6P1/1PP5/PB1PPP1P/RN1BQNKR w HAha - 1 9 ;D1 19 ;D2 663 ;D3 14149 ;D4 489653 ;D5 11491355 ;D6 399135495",
        "rnbqnbkr/1pp1p2p/3p1p2/p5p1/5PP1/2P5/PPNPP2P/RNBQ1BKR w HAha - 0 9 ;D1 24 ;D2 647 ;D3 16679 ;D4 461931 ;D5 12649636 ;D6 361157611",
        "rnb2krb/pppqppnp/8/3p2p1/1P4P1/7P/P1PPPPB1/RNBQNKR1 w GAga - 1 9 ;D1 24 ;D2 722 ;D3 18749 ;D4 605229 ;D5 16609220 ;D6 563558512",
        "rbnqb1kr/pppn1pp1/3p3p/4p3/1P6/P7/R1PPPPPP/1BNQBNKR w Hha - 1 9 ;D1 20 ;D2 538 ;D3 12277 ;D4 345704 ;D5 8687621 ;D6 255304141",
        "rnqb1nkr/p1pbp1pp/8/1pPp1p2/P2P4/8/1P2PPPP/RNQBBNKR w HAha - 1 9 ;D1 35 ;D2 764 ;D3 26952 ;D4 632796 ;D5 22592380 ;D6 564255328",
        "rnq1bbkr/1p1ppp1p/4n3/p1p3p1/P1PP4/8/RP2PPPP/1NQNBBKR w Hha - 0 9 ;D1 29 ;D2 709 ;D3 21296 ;D4 570580 ;D5 17597398 ;D6 506140370",
        "1nqnbkrb/1pppp2p/r7/p4pp1/3P4/8/PPPBPPPP/RNQNK1RB w g - 0 9 ;D1 27 ;D2 1028 ;D3 28534 ;D4 1050834 ;D5 30251988 ;D6 1096869832",
        "rbnqnkbr/p1pp1p1p/8/1p2p3/3P2pP/2P5/PP2PPP1/RBNQNKBR w HAha - 0 9 ;D1 32 ;D2 832 ;D3 27120 ;D4 750336 ;D5 24945574 ;D6 724171581",
        "rnq1nkbr/1p1p1ppp/2p1pb2/p7/7P/2P5/PPNPPPPB/RNQB1K1R w HAha - 2 9 ;D1 31 ;D2 779 ;D3 24010 ;D4 638640 ;D5 19919434 ;D6 551494771",
        "rnqnk1br/p1ppp1bp/1p3p2/6p1/4N3/P5P1/1PPPPP1P/R1QNKBBR w HAha - 2 9 ;D1 25 ;D2 717 ;D3 19396 ;D4 576577 ;D5 16525239 ;D6 507175842",
        "rnq1krbb/p1p1pppp/8/1p1p4/1n5B/2N2P2/PPPPP1PP/RNQ1KR1B w FAfa - 0 9 ;D1 28 ;D2 867 ;D3 24029 ;D4 735686 ;D5 21112751 ;D6 654808184",
        "bbrnnqkr/1pp1pppp/3p4/p7/P3P3/7P/1PPP1PP1/BBRNNQKR w HChc - 0 9 ;D1 24 ;D2 405 ;D3 11025 ;D4 210557 ;D5 6196438 ;D6 131401224",
        "brnbnqkr/p1ppp3/1p5p/5Pp1/5P2/3N4/PPPPP2P/BRNB1QKR w HBhb g6 0 9 ;D1 25 ;D2 785 ;D3 21402 ;D4 698331 ;D5 20687969 ;D6 695850727",
        "br1nqbkr/1ppppp2/pn6/6pp/2PP4/1N4P1/PP2PP1P/BR1NQBKR w HBhb - 0 9 ;D1 25 ;D2 596 ;D3 16220 ;D4 421882 ;D5 12185361 ;D6 337805606",
        "1rnnqkrb/p2ppp1p/1pp5/2N3p1/8/1P6/P1PPPPKP/BR1NQ1RB w gb - 0 9 ;D1 38 ;D2 960 ;D3 34831 ;D4 913665 ;D5 32490040 ;D6 880403591",
        "rbbnnqkr/pp3pp1/2p1p3/3p3p/3P3P/1PP5/P3PPP1/RBBNNQKR w HAha - 0 9 ;D1 30 ;D2 785 ;D3 23079 ;D4 656618 ;D5 19885037 ;D6 599219582",
        "rn1bnqkr/p1ppppp1/8/1p5p/P4P1P/3N4/1PPPP1b1/RNBB1QKR w HAha - 0 9 ;D1 27 ;D2 752 ;D3 21735 ;D4 613194 ;D5 18862234 ;D6 547415271",
        "1nbnqbkr/1p1p1ppp/r3p3/p1p5/P3P3/3Q4/1PPP1PPP/RNBN1BKR w HAh - 2 9 ;D1 33 ;D2 721 ;D3 24278 ;D4 572535 ;D5 19648535 ;D6 496023732",
        "rnbnqkrb/2pppppp/1p6/p7/1PP5/4N2P/P2PPPP1/RNB1QKRB w GAg - 0 9 ;D1 23 ;D2 570 ;D3 14225 ;D4 374196 ;D5 10022614 ;D6 279545007",
        "rbnnbq1r/ppppppkp/6p1/N7/4P3/P7/1PPP1PPP/RB1NBQKR w HA - 5 9 ;D1 27 ;D2 620 ;D3 18371 ;D4 440594 ;D5 13909432 ;D6 349478320",
        "r1nbbqkr/pppppp1p/8/8/1n3Pp1/3N1QP1/PPPPP2P/RN1BB1KR w HAha - 0 9 ;D1 31 ;D2 791 ;D3 25431 ;D4 682579 ;D5 22408813 ;D6 636779732",
        "rnq1bbkr/pp1p1ppp/2pnp3/8/7P/1QP5/PP1PPPPR/RNN1BBK1 w Aha - 2 9 ;D1 28 ;D2 559 ;D3 16838 ;D4 390887 ;D5 12242780 ;D6 315431511",
        "rnnqbrkb/2ppppp1/1p1N4/p6p/4P3/8/PPPP1PPP/R1NQBKRB w GA - 0 9 ;D1 32 ;D2 638 ;D3 20591 ;D4 438792 ;D5 14395828 ;D6 331782223",
        "rbnnq1br/pppp1kp1/4pp2/7p/PP6/2PP4/4PPPP/RBNNQKBR w HA - 0 9 ;D1 21 ;D2 521 ;D3 12201 ;D4 320429 ;D5 8239159 ;D6 227346638",
        "rnnbqkbr/p2ppp2/7p/1pp3p1/2P2N2/8/PP1PPPPP/RN1BQKBR w HAha - 0 9 ;D1 25 ;D2 528 ;D3 13896 ;D4 326094 ;D5 9079829 ;D6 232750602",
        "rnn1kbbr/ppppqp2/6p1/2N1p2p/P7/2P5/1P1PPPPP/RN1QKBBR w HAha - 2 9 ;D1 27 ;D2 801 ;D3 22088 ;D4 707078 ;D5 20334071 ;D6 682580976",
        "rnnqkrbb/p1p1p1pp/1p3p2/8/3p2Q1/P1P1P3/1P1P1PPP/RNN1KRBB w FAfa - 0 9 ;D1 37 ;D2 1014 ;D3 34735 ;D4 998999 ;D5 32921537 ;D6 988770109",
        "bbrnk1qr/1pppppp1/p4n1p/8/P2P2N1/8/1PP1PPPP/BBR1NKQR w HC - 1 9 ;D1 21 ;D2 481 ;D3 11213 ;D4 279993 ;D5 7015419 ;D6 187564853",
        "brnbnkqr/1pp1p1p1/p2p1p2/7p/1P4PP/8/PBPPPP2/1RNBNKQR w HBhb - 0 9 ;D1 31 ;D2 743 ;D3 24260 ;D4 660177 ;D5 22391185 ;D6 653721389",
        "br2kbqr/ppppp1pp/3n1p2/3P4/3n3P/3N4/PPP1PPP1/BR1NKBQR w HBhb - 3 9 ;D1 25 ;D2 872 ;D3 22039 ;D4 748726 ;D5 20281962 ;D6 685749952",
        "br1nkqrb/ppppppp1/8/7p/4P3/n1P2PP1/PP1P3P/BRNNKQRB w GBgb - 0 9 ;D1 28 ;D2 607 ;D3 16934 ;D4 396483 ;D5 11607818 ;D6 294181806",
        "rbbn1kqr/pp1pp1p1/2pn3p/5p2/5P2/1P1N4/PNPPP1PP/RBB2KQR w HAha - 1 9 ;D1 27 ;D2 725 ;D3 21543 ;D4 616082 ;D5 19239812 ;D6 581716972",
        "rnbbnk1r/pp1ppp1p/6q1/2p5/PP4p1/4P3/2PP1PPP/RNBBNKQR w HAha - 1 9 ;D1 25 ;D2 1072 ;D3 26898 ;D4 1088978 ;D5 28469879 ;D6 1122703887",
        "rnbnkbqr/1pp3pp/3p4/p3pp2/3P2P1/2N1N3/PPP1PP1P/R1B1KBQR w HAha - 0 9 ;D1 31 ;D2 1028 ;D3 32907 ;D4 1095472 ;D5 36025223 ;D6 1211187800",
        "r1bnkqrb/1ppppppp/p3n3/8/6P1/4N3/PPPPPPRP/RNB1KQ1B w Aga - 1 9 ;D1 23 ;D2 457 ;D3 11416 ;D4 250551 ;D5 6666787 ;D6 159759052",
        "rbn1bkqr/p1pp1pp1/1pn5/4p2p/7P/1PBP4/P1P1PPP1/RBNN1KQR w HAha - 0 9 ;D1 23 ;D2 470 ;D3 11649 ;D4 264274 ;D5 6963287 ;D6 172833738",
        "rnnbbkqr/3ppppp/p7/1pp5/P6P/6P1/1PPPPP2/RNNBBKQR w HAha - 0 9 ;D1 26 ;D2 569 ;D3 15733 ;D4 375556 ;D5 11008114 ;D6 284485303",
        "r1nk1bqr/1pppp1pp/2n5/p4p1b/5P2/1N4B1/PPPPP1PP/RN1K1BQR w HAha - 2 9 ;D1 25 ;D2 824 ;D3 21983 ;D4 738366 ;D5 20904119 ;D6 716170771",
        "r1nkbqrb/p2pppp1/npp4p/8/4PP2/2N4P/PPPP2P1/R1NKBQRB w GAga - 0 9 ;D1 31 ;D2 548 ;D3 17480 ;D4 349633 ;D5 11469548 ;D6 255067638",
        "rbnnkqbr/ppppp2p/5p2/6p1/2P1B3/P6P/1P1PPPP1/R1NNKQBR w HAha - 1 9 ;D1 31 ;D2 809 ;D3 24956 ;D4 680747 ;D5 21247414 ;D6 606221516",
        "1r1bkqbr/pppp1ppp/2nnp3/8/2P5/N4P2/PP1PP1PP/1RNBKQBR w Hh - 0 9 ;D1 28 ;D2 810 ;D3 22844 ;D4 694599 ;D5 20188622 ;D6 636748147",
        "rn1kqbbr/p1pppp1p/1p4p1/1n6/1P2P3/4Q2P/P1PP1PP1/RNNK1BBR w HAha - 1 9 ;D1 39 ;D2 848 ;D3 30100 ;D4 724426 ;D5 25594662 ;D6 659615710",
        "rn1kqrbb/pppppppp/8/8/2nP2P1/1P2P3/P1P2P1P/RNNKQRBB w FAfa - 1 9 ;D1 29 ;D2 766 ;D3 21701 ;D4 567971 ;D5 16944425 ;D6 456898648",
        "b1rnnkrq/bpppppp1/7p/8/1p6/2B5/PNPPPPPP/1BR1NKRQ w GCgc - 2 9 ;D1 25 ;D2 667 ;D3 17253 ;D4 472678 ;D5 12865247 ;D6 365621294",
        "brnb1krq/pppppppp/8/5P2/2P1n2P/8/PP1PP1P1/BRNBNKRQ w GBgb - 1 9 ;D1 23 ;D2 620 ;D3 14882 ;D4 402561 ;D5 10776855 ;D6 300125003",
        "b1nnkbrq/pr1pppp1/1p5p/2p5/P2N1P2/8/1PPPP1PP/BR1NKBRQ w GBg - 0 9 ;D1 24 ;D2 472 ;D3 12181 ;D4 267398 ;D5 7370758 ;D6 178605165",
        "br1nkrqb/p1p1p1pp/3n4/1p1p1p2/5N1P/4P3/PPPP1PP1/BR1NKRQB w FBfb - 0 9 ;D1 24 ;D2 775 ;D3 19398 ;D4 624309 ;D5 16429837 ;D6 539767605",
        "rbbnnkrq/p2pp1pp/2p5/5p2/1pPP1B2/P7/1P2PPPP/RB1NNKRQ w GAga - 0 9 ;D1 34 ;D2 921 ;D3 30474 ;D4 849933 ;D5 28095833 ;D6 806446436",
        "rnbbnkr1/1p1ppp1p/2p3p1/p7/2Pq4/1P1P4/P2BPPPP/RN1BNKRQ w GAga - 2 9 ;D1 26 ;D2 1139 ;D3 29847 ;D4 1204863 ;D5 32825932 ;D6 1281760240",
        "1rbnkbrq/pppppp2/n5pp/2P5/P7/4N3/1P1PPPPP/RNB1KBRQ w GAg - 2 9 ;D1 23 ;D2 574 ;D3 14146 ;D4 391413 ;D5 10203438 ;D6 301874034",
        "1nbnkr1b/rppppppq/p7/7p/1P5P/3P2P1/P1P1PP2/RNBNKRQB w FAf - 1 9 ;D1 33 ;D2 823 ;D3 26696 ;D4 724828 ;D5 23266182 ;D6 672294132",
        "rbn1bkrq/ppppp3/4n2p/5pp1/1PN5/2P5/P2PPPPP/RBN1BKRQ w GAga - 0 9 ;D1 27 ;D2 859 ;D3 24090 ;D4 796482 ;D5 23075785 ;D6 789152120",
        "r1nbbkrq/1ppp2pp/2n2p2/p3p3/5P2/1N4BP/PPPPP1P1/RN1B1KRQ w GAga - 0 9 ;D1 25 ;D2 774 ;D3 20141 ;D4 618805 ;D5 16718577 ;D6 515864053",
        "rnnkbbrq/1pppp1p1/5p2/7p/p6P/3N1P2/PPPPP1PQ/RN1KBBR1 w GAga - 0 9 ;D1 29 ;D2 673 ;D3 20098 ;D4 504715 ;D5 15545590 ;D6 416359581",
        "r1nkbrqb/pppp1p2/n3p1p1/7p/2P2P2/1P6/P2PPQPP/RNNKBR1B w FAfa - 0 9 ;D1 27 ;D2 722 ;D3 21397 ;D4 593762 ;D5 18742426 ;D6 537750982",
        "rbnnkr1q/1ppp2pp/p4p2/P2bp3/4P2P/8/1PPP1PP1/RBNNKRBQ w FAfa - 1 9 ;D1 26 ;D2 848 ;D3 23387 ;D4 741674 ;D5 21591790 ;D6 675163653",
        "rn1bkrb1/1ppppp1p/pn4p1/8/P2q3P/3P4/NPP1PPP1/RN1BKRBQ w FAfa - 1 9 ;D1 22 ;D2 803 ;D3 18322 ;D4 632920 ;D5 15847763 ;D6 536419559",
        "rn1krbbq/pppp1npp/4pp2/8/4P2P/3P2P1/PPP2P2/RNNKRBBQ w EAea - 1 9 ;D1 29 ;D2 810 ;D3 23968 ;D4 670500 ;D5 20361517 ;D6 575069358",
        "rnn1rqbb/ppkp1pp1/2p1p2p/2P5/8/3P1P2/PP2P1PP/RNNKRQBB w EA - 0 9 ;D1 22 ;D2 506 ;D3 11973 ;D4 292344 ;D5 7287368 ;D6 189865944",
        "bbqr1knr/pppppp1p/8/4n1p1/2P1P3/6P1/PPQP1P1P/BB1RNKNR w HDhd - 0 9 ;D1 26 ;D2 650 ;D3 18253 ;D4 481200 ;D5 14301029 ;D6 394943978",
        "bq1bnknr/pprppp1p/8/2p3p1/4PPP1/8/PPPP3P/BQRBNKNR w HCh - 0 9 ;D1 24 ;D2 548 ;D3 14021 ;D4 347611 ;D5 9374021 ;D6 250988458",
        "bqrnkb1r/1p2pppp/p1pp3n/5Q2/2P4P/5N2/PP1PPPP1/B1RNKB1R w HChc - 0 9 ;D1 46 ;D2 823 ;D3 33347 ;D4 673905 ;D5 26130444 ;D6 582880996",
        "bq1rknrb/pppppp1p/4n3/6p1/4P1P1/3P1P2/PPP4P/BQRNKNRB w GCg - 0 9 ;D1 23 ;D2 618 ;D3 14815 ;D4 419474 ;D5 10606831 ;D6 315124518",
        "q1brnknr/pp1pp1p1/8/2p2p1p/5b2/P4N2/1PPPP1PP/QBBRK1NR w hd - 0 9 ;D1 22 ;D2 675 ;D3 15778 ;D4 473994 ;D5 12077228 ;D6 368479752",
        "qrbbnknr/1p1ppp1p/p1p5/8/1P2P1p1/3P1B2/P1P2PPP/QRB1NKNR w HBhb - 0 9 ;D1 32 ;D2 722 ;D3 24049 ;D4 569905 ;D5 19584539 ;D6 484814878",
        "qrb1kbnr/p3pppp/2n5/1ppp4/7P/3P1P2/PPP1P1PR/QRBNKBN1 w Bhb - 0 9 ;D1 26 ;D2 831 ;D3 22606 ;D4 724505 ;D5 20500804 ;D6 662608969",
        "qrbnknrb/ppp1pp2/6p1/7p/PPNp4/8/2PPPPPP/QRB1KNRB w GBgb - 0 9 ;D1 31 ;D2 840 ;D3 26762 ;D4 742772 ;D5 24422614 ;D6 701363800",
        "qbrnbknr/pp1pp1pp/8/2p2p2/3Q4/PP6/2PPPPPP/1BRNBKNR w HChc - 0 9 ;D1 38 ;D2 1121 ;D3 39472 ;D4 1198438 ;D5 41108769 ;D6 1285503872",
        "qr1bbk1r/pppppp1p/1n6/5np1/4B3/1PP5/P2PPPPP/QRN1BKNR w HBhb - 0 9 ;D1 25 ;D2 694 ;D3 16938 ;D4 472950 ;D5 12164609 ;D6 345122090",
        "qrnkbbnr/1p1pp2p/p7/2p1Npp1/6P1/7P/PPPPPP2/QR1KBBNR w HBhb - 0 9 ;D1 27 ;D2 586 ;D3 16348 ;D4 393391 ;D5 11409633 ;D6 298054792",
        "qrnkbnrb/pp1p1p2/2p1p1pp/4N3/P4P2/8/1PPPP1PP/QR1KBNRB w GBgb - 0 9 ;D1 32 ;D2 645 ;D3 20737 ;D4 460319 ;D5 15037464 ;D6 358531599",
        "qbrnknbr/1pppppp1/p6p/8/1P6/3PP3/PQP2PPP/1BRNKNBR w HChc - 3 9 ;D1 26 ;D2 595 ;D3 16755 ;D4 415022 ;D5 12214768 ;D6 323518628",
        "qrnbk1br/1ppppp1p/p5p1/8/4Pn2/4K1P1/PPPP1P1P/QRNB1NBR w hb - 0 9 ;D1 24 ;D2 609 ;D3 13776 ;D4 359415 ;D5 8538539 ;D6 230364479",
        "qrnk1bbr/1pnp1ppp/p1p1p3/8/3Q4/1P1N3P/P1PPPPP1/1RNK1BBR w HBhb - 0 9 ;D1 43 ;D2 1106 ;D3 42898 ;D4 1123080 ;D5 41695761 ;D6 1113836402",
        "qrnknrb1/pppppp2/8/6pp/4P2P/3P1P2/PbP3P1/QRNKNRBB w FBfb - 0 9 ;D1 24 ;D2 658 ;D3 17965 ;D4 488373 ;D5 14457245 ;D6 400971226",
        "bbrqnrk1/ppp2ppp/7n/3pp3/8/P4N1N/1PPPPPPP/BBRQ1RK1 w - - 1 9 ;D1 22 ;D2 503 ;D3 12078 ;D4 310760 ;D5 8080951 ;D6 224960353",
        "brqbnk1r/1ppp1ppp/8/p3pn2/8/2PP1P2/PP2PKPP/BRQBN1NR w hb - 1 9 ;D1 25 ;D2 745 ;D3 19387 ;D4 570459 ;D5 15520298 ;D6 460840861",
        "brqnkbnr/pp2pp1p/3p4/2p5/5p2/3P3P/PPP1PPP1/B1RNKBNR w Hhb - 0 9 ;D1 19 ;D2 516 ;D3 10755 ;D4 312996 ;D5 6995034 ;D6 214340699",
        "brq1kn1b/1ppppprp/2n3p1/p7/P1N5/6P1/1PPPPP1P/BRQNK1RB w GBb - 2 9 ;D1 29 ;D2 557 ;D3 16739 ;D4 352277 ;D5 10840256 ;D6 249999654",
        "rbbq1k1r/ppp1pppp/7n/1n1p4/5P2/P2P4/1PPBP1PP/RB1QNKNR w HAha - 1 9 ;D1 25 ;D2 769 ;D3 20110 ;D4 638340 ;D5 17438715 ;D6 570893953",
        "r1bbnk1r/qpp1pppp/p6n/3p4/1P6/5N1P/P1PPPPP1/RQBBK1NR w ha - 0 9 ;D1 23 ;D2 728 ;D3 18209 ;D4 587364 ;D5 16053564 ;D6 529082811",
        "rqbnkbnr/1pp2p1p/3p4/p3p1p1/8/2P2P2/PP1PPNPP/RQBNKB1R w HAha - 0 9 ;D1 26 ;D2 772 ;D3 21903 ;D4 653704 ;D5 19571559 ;D6 593915677",
        "r1bnknrb/pqppp1p1/1p5p/5p2/7P/3P2N1/PPP1PPP1/RQBNK1RB w GAga - 2 9 ;D1 27 ;D2 748 ;D3 20291 ;D4 597105 ;D5 16324542 ;D6 506453626",
        "rbqnbknr/pp1pppp1/8/2p5/3P3p/5N1P/PPP1PPPR/RBQNBK2 w Aha - 0 9 ;D1 30 ;D2 859 ;D3 26785 ;D4 819631 ;D5 26363334 ;D6 842796987",
        "rqnbbrk1/ppppppp1/8/5n1p/3P3P/2B3P1/PPP1PP2/RQNB1KNR w HA - 0 9 ;D1 22 ;D2 505 ;D3 11452 ;D4 283464 ;D5 7055215 ;D6 186760784",
        "rqnkbbnr/pp2p1p1/8/2pp1p1p/3PPP2/8/PPP1N1PP/RQNKBB1R w HAha - 0 9 ;D1 28 ;D2 832 ;D3 23142 ;D4 722857 ;D5 20429246 ;D6 663183060",
        "rqnkbnr1/pppp2bp/6p1/4pp2/1P2P3/3NN3/P1PP1PPP/RQ1KB1RB w GAga - 0 9 ;D1 28 ;D2 641 ;D3 18835 ;D4 459993 ;D5 14038570 ;D6 364210162",
        "rbq2kbr/pppppppp/2n5/P7/3P1n2/2P5/1P2PPPP/RBQNKNBR w HA - 1 9 ;D1 31 ;D2 889 ;D3 27028 ;D4 766181 ;D5 24299415 ;D6 692180754",
        "rq1bkn1r/ppppp2p/3n4/5pp1/2b3P1/1N1P1P2/PPP1P2P/RQ1BKNBR w HAha - 1 9 ;D1 28 ;D2 810 ;D3 22667 ;D4 657520 ;D5 18719949 ;D6 556282676",
        "r1nknbbr/p2ppp1p/1pp3p1/8/1P6/4P3/P1PPNPPq/R1QKNBBR w HAha - 0 9 ;D1 24 ;D2 797 ;D3 22144 ;D4 719069 ;D5 21862776 ;D6 716521139",
        "rqnknrbb/ppp1p3/5ppp/2Np4/2P5/4P3/PP1P1PPP/RQNK1RBB w FAfa - 0 9 ;D1 34 ;D2 686 ;D3 23277 ;D4 515541 ;D5 17664543 ;D6 423574794",
        "1brnqknr/2p1pppp/p2p4/1P6/6P1/4Nb2/PP1PPP1P/BBR1QKNR w HChc - 1 9 ;D1 34 ;D2 1019 ;D3 32982 ;D4 1003103 ;D5 33322477 ;D6 1043293394",
        "brn1qknr/1p1pppp1/pb5p/Q1p5/3P3P/8/PPP1PPPR/BRNB1KN1 w Bhb - 2 9 ;D1 32 ;D2 642 ;D3 20952 ;D4 464895 ;D5 15454749 ;D6 371861782",
        "brnqkbnr/pppppp2/8/6pp/6P1/P2P1P2/1PP1P2P/BRNQKBNR w HBhb - 0 9 ;D1 20 ;D2 441 ;D3 9782 ;D4 240220 ;D5 5770284 ;D6 153051835",
        "2nqknrb/1rpppppp/5B2/pp6/1PP1b3/3P4/P3PPPP/1RNQKNRB w GBg - 1 9 ;D1 35 ;D2 1042 ;D3 36238 ;D4 1101159 ;D5 38505058 ;D6 1202668717",
        "rb1nqknr/1pp1pppp/8/3p4/p2P4/6PN/PPPQPP1P/RBBN1K1R w HAha - 0 9 ;D1 29 ;D2 692 ;D3 21237 ;D4 555018 ;D5 17820605 ;D6 497251206",
        "rnbbqknr/pppp4/5p2/4p1pp/P7/2N2PP1/1PPPP2P/R1BBQKNR w HAha - 0 9 ;D1 23 ;D2 595 ;D3 14651 ;D4 415772 ;D5 10881112 ;D6 329010121",
        "rn1qkbnr/p1p1pp1p/bp4p1/3p4/1P6/4P3/P1PP1PPP/RNBQKBNR w HAha - 0 9 ;D1 30 ;D2 794 ;D3 24319 ;D4 690811 ;D5 21657601 ;D6 647745807",
        "r1bqk1rb/pppnpppp/5n2/3p4/2P3PP/2N5/PP1PPP2/R1BQKNRB w GAga - 1 9 ;D1 32 ;D2 821 ;D3 27121 ;D4 733155 ;D5 24923473 ;D6 710765657",
        "rbnqbknr/1p1ppp1p/6p1/p1p5/7P/3P4/PPP1PPP1/RBNQBKNR w HAha - 0 9 ;D1 24 ;D2 720 ;D3 18842 ;D4 575027 ;D5 15992882 ;D6 501093456",
        "r1qbbk1r/pp1ppppp/n1p5/5n2/B1P3P1/8/PP1PPP1P/RNQ1BKNR w HAha - 0 9 ;D1 27 ;D2 831 ;D3 22293 ;D4 698986 ;D5 19948650 ;D6 637973209",
        "rnqkbb1r/p1pppppp/8/8/1p4n1/PP4PP/2PPPP2/RNQKBBNR w HAha - 0 9 ;D1 18 ;D2 463 ;D3 9519 ;D4 256152 ;D5 6065231 ;D6 172734380",
        "rnqk1nrb/pppbpp2/7p/3p2p1/4B3/2N1N1P1/PPPPPP1P/R1QKB1R1 w GAga - 0 9 ;D1 34 ;D2 1171 ;D3 38128 ;D4 1318217 ;D5 42109356 ;D6 1465473753",
        "rbnqknbr/1pp1ppp1/3p4/7p/p2P2PP/2P5/PP2PP2/RBNQKNBR w HAha - 0 9 ;D1 32 ;D2 867 ;D3 28342 ;D4 798722 ;D5 26632459 ;D6 781067145",
        "rn1bknbr/pq2pppp/1p6/2pp4/P7/1P1P4/2PNPPPP/RNQBK1BR w HAha - 0 9 ;D1 24 ;D2 627 ;D3 16652 ;D4 462942 ;D5 13200921 ;D6 385193532",
        "r1qk1bbr/ppp1pp1p/2np1n2/6p1/2PP4/3BP3/PP3PPP/RNQKN1BR w HAha - 2 9 ;D1 31 ;D2 992 ;D3 30213 ;D4 986631 ;D5 30397368 ;D6 1011631987",
        "r1qknrbb/pppp1p2/2n3p1/4p2p/8/QPP5/P1NPPPPP/RN1K1RBB w FAfa - 2 9 ;D1 30 ;D2 702 ;D3 21563 ;D4 532939 ;D5 16813114 ;D6 438096194",
        "bbkr1qnr/2pppppp/2n5/pp6/8/PPN5/1BPPPPPP/1BR1KQNR w HC - 2 9 ;D1 25 ;D2 573 ;D3 15183 ;D4 380910 ;D5 10554668 ;D6 283975400",
        "1rnbkqnr/1bpppppp/1p6/7P/p2P4/5P2/PPP1P1P1/BRNBKQNR w HBhb - 0 9 ;D1 21 ;D2 503 ;D3 11790 ;D4 301084 ;D5 7679979 ;D6 207799378",
        "brnkqbnr/2p1pppp/1p6/3p4/1pP5/P6P/3PPPP1/BRNKQBNR w HBhb - 0 9 ;D1 28 ;D2 743 ;D3 21054 ;D4 587192 ;D5 17354516 ;D6 507176753",
        "br1kqnrb/npp1pppp/8/3p4/p4N2/PP6/2PPPPPP/BR1KQNRB w GBgb - 0 9 ;D1 31 ;D2 808 ;D3 25585 ;D4 698475 ;D5 22376575 ;D6 640362920",
        "rbbnkq1r/pppppp1p/7n/6p1/P5P1/2P2N2/1P1PPP1P/RBBNKQ1R w HAha - 1 9 ;D1 29 ;D2 580 ;D3 17585 ;D4 404831 ;D5 12730970 ;D6 325226128",
        "rnbbk1nr/pp2qppp/2ppp3/8/3P4/P1N4N/1PP1PPPP/R1BBKQ1R w HAha - 0 9 ;D1 29 ;D2 838 ;D3 24197 ;D4 721884 ;D5 21100580 ;D6 646624429",
        "rnbk1b1r/ppppn1pp/4pp2/7q/7P/P5PB/1PPPPP2/RNBKQ1NR w HAha - 3 9 ;D1 20 ;D2 729 ;D3 16633 ;D4 576199 ;D5 14507076 ;D6 498621813",
        "r2kqnrb/pbppppp1/np5p/8/4Q1P1/3P4/PPP1PP1P/RNBK1NRB w GAga - 2 9 ;D1 47 ;D2 1219 ;D3 55009 ;D4 1486353 ;D5 65239153 ;D6 1834391369",
        "rbnkbq1r/p1p2ppp/1p2pn2/3p4/P3P3/3P4/1PP1KPPP/RBN1BQNR w ha - 2 9 ;D1 29 ;D2 923 ;D3 27179 ;D4 883866 ;D5 26202752 ;D6 868565895",
        "rk1bb1nr/ppppqppp/n7/1N2p3/6P1/7N/PPPPPP1P/R1KBBQ1R w HA - 6 9 ;D1 27 ;D2 703 ;D3 19478 ;D4 559525 ;D5 16049807 ;D6 492966455",
        "rnkqbbnr/p1ppp2p/1p4p1/8/1B3p1P/2NP4/PPP1PPP1/R1KQ1BNR w HAha - 0 9 ;D1 29 ;D2 610 ;D3 18855 ;D4 438277 ;D5 14020041 ;D6 355083962",
        "rnkqb1rb/pp1p1ppp/4p3/2P3n1/8/1PP5/P3PPPP/RNKQBNRB w GAga - 0 9 ;D1 29 ;D2 675 ;D3 20699 ;D4 535821 ;D5 17000613 ;D6 476598337",
        "rb1kqnbr/pp1pp1p1/1np2p2/7p/P1P3PP/8/1P1PPP2/RBNKQNBR w HAha - 0 9 ;D1 31 ;D2 1077 ;D3 33661 ;D4 1183381 ;D5 37415304 ;D6 1328374620",
        "rnkbq1br/ppp2ppp/3p4/Q3p1n1/5P2/3P2P1/PPP1P2P/RNKB1NBR w HAha - 0 9 ;D1 41 ;D2 1201 ;D3 46472 ;D4 1420367 ;D5 52991625 ;D6 1675608008",
        "rn1qnbbr/pp2pppp/2ppk3/8/2PP4/3Q1N2/PP2PPPP/RNK2BBR w HA - 1 9 ;D1 34 ;D2 666 ;D3 22474 ;D4 472299 ;D5 15860369 ;D6 353831792",
        "rnkqnr1b/ppppp1pp/5p2/8/Q1P2P2/8/PP1P2PP/RbK1NRBB w FAfa - 0 9 ;D1 36 ;D2 876 ;D3 31987 ;D4 788580 ;D5 29022529 ;D6 736717252",
        "bbrn1nqr/ppp1k1pp/5p2/3pp3/7P/3PN3/PPP1PPP1/BBRK1NQR w - - 1 9 ;D1 24 ;D2 583 ;D3 15063 ;D4 383532 ;D5 10522064 ;D6 280707118",
        "brnbkn1r/1pppp1p1/4q3/p4p1p/7P/1N3P2/PPPPP1PQ/BR1BKN1R w HBhb - 2 9 ;D1 27 ;D2 935 ;D3 26120 ;D4 885699 ;D5 26000648 ;D6 873063158",
        "br1knbqr/pp2p1pp/1n6/2pp1p2/6P1/2P4B/PP1PPPQP/BRNKN2R w HBhb - 0 9 ;D1 27 ;D2 681 ;D3 19202 ;D4 510687 ;D5 14954779 ;D6 415624943",
        "brnk1qrb/p1ppppp1/1p5p/8/P3n3/1N4P1/1PPPPPRP/BR1KNQ1B w Bgb - 0 9 ;D1 22 ;D2 638 ;D3 13991 ;D4 412346 ;D5 9760752 ;D6 293499724",
        "rbbnknqr/pppp3p/5pp1/8/1P1pP3/7P/P1P2PP1/RBBNKNQR w HAha - 0 9 ;D1 29 ;D2 756 ;D3 21616 ;D4 614074 ;D5 17602252 ;D6 528140595",
        "1nbbknqr/rpp1ppp1/1Q1p3p/p7/2P2PP1/8/PP1PP2P/RNBBKN1R w HAh - 2 9 ;D1 37 ;D2 977 ;D3 34977 ;D4 944867 ;D5 33695089 ;D6 940198007",
        "rnb2bqr/ppkpppp1/3n3p/2p5/6PP/2N2P2/PPPPP3/R1BKNBQR w HA - 2 9 ;D1 30 ;D2 647 ;D3 20365 ;D4 467780 ;D5 15115531 ;D6 369257622",
        "rn1k1qrb/p1pppppp/bp6/8/4n3/P4BPP/1PPPPP2/RNBKNQR1 w GAga - 2 9 ;D1 22 ;D2 670 ;D3 14998 ;D4 451517 ;D5 11199653 ;D6 339919682",
        "rb2bnqr/nppkpppp/3p4/p7/1P6/P2N2P1/2PPPP1P/RB1KBNQR w HA - 3 9 ;D1 22 ;D2 479 ;D3 11475 ;D4 264739 ;D5 6831555 ;D6 167329117",
        "r1kbb1qr/2pppppp/np2n3/p7/2P3P1/8/PP1PPPQP/RNKBBN1R w HAha - 1 9 ;D1 32 ;D2 723 ;D3 23953 ;D4 581832 ;D5 19472074 ;D6 504622114",
        "rnknbb1r/p1ppp1pp/8/1p1P1p1q/8/P1P5/1P2PPPP/RNKNBBQR w HAha - 1 9 ;D1 19 ;D2 607 ;D3 12733 ;D4 417451 ;D5 9753617 ;D6 325177085",
        "rnkn1qrb/pp1bp1pp/2p5/1N1p1p2/8/2P5/PPKPPPPP/R2NBQRB w ga - 2 9 ;D1 27 ;D2 533 ;D3 14549 ;D4 330747 ;D5 9206957 ;D6 232664675",
        "r1nknqbr/pp2p1pp/2p2p2/3p4/6P1/PP1P4/2P1PP1b/RBNKNQBR w HAha - 0 9 ;D1 20 ;D2 582 ;D3 13777 ;D4 409166 ;D5 10708639 ;D6 326565393",
        "rnkb1qbr/p1pp1p1p/1p2pn2/1Q4p1/4P3/N4P2/PPPP2PP/R1KBN1BR w HAha - 0 9 ;D1 40 ;D2 1038 ;D3 39356 ;D4 1051441 ;D5 39145902 ;D6 1079612614",
        "rn2qbbr/1pkppp1p/p3n1p1/8/8/2P2P2/PP1PP1PP/RNKN1BBR w HA - 0 9 ;D1 24 ;D2 605 ;D3 14888 ;D4 385964 ;D5 9687507 ;D6 260874068",
        "rn1nqrbb/p1kppp1p/8/1pp3p1/1P6/2N1P3/P1PP1PPP/RK1NQRBB w - - 0 9 ;D1 21 ;D2 540 ;D3 12489 ;D4 337997 ;D5 8436136 ;D6 237525904",
        "bbrnknrq/1pp3pp/p2p1p2/4p3/P7/1P2N3/2PPPPPP/BBRN1RKQ w gc - 0 9 ;D1 24 ;D2 527 ;D3 13900 ;D4 326175 ;D5 9139962 ;D6 226253685",
        "brnb1nrq/pppp1kpp/4p3/8/5p1P/P1P3P1/1P1PPP2/BRNBKNRQ w GB - 1 9 ;D1 29 ;D2 773 ;D3 23904 ;D4 638768 ;D5 20503775 ;D6 560338709",
        "br1k1brq/ppppp2p/1n1n1pp1/8/P1P5/3P2P1/1P2PP1P/BRNKNBRQ w GBgb - 0 9 ;D1 28 ;D2 811 ;D3 23550 ;D4 664880 ;D5 19913758 ;D6 565143976",
        "1r1knrqb/n1pppppp/p1b5/1p6/8/3N1P2/PPPPP1PP/BRNK1RQB w fb - 3 9 ;D1 29 ;D2 753 ;D3 23210 ;D4 620019 ;D5 20044474 ;D6 558383603",
        "rbbnk1rq/pppppppp/8/3Pn3/8/4P1P1/PPP2P1P/RBBNKNRQ w GAga - 1 9 ;D1 22 ;D2 551 ;D3 12619 ;D4 324608 ;D5 8204171 ;D6 217689974",
        "rnbbk1rq/2pppp1p/p3n1p1/1p6/P3N3/8/1PPPPPPP/RNBB1KRQ w ga - 0 9 ;D1 26 ;D2 742 ;D3 20061 ;D4 599527 ;D5 16787080 ;D6 525678162",
        "rnbkn1rq/ppppppb1/6p1/7p/2B2P2/1P2P3/P1PP2PP/RNBKN1RQ w GAga - 1 9 ;D1 28 ;D2 799 ;D3 23210 ;D4 689436 ;D5 20755098 ;D6 639632905",
        "rn1knrqb/p2pppp1/b1p5/1p5p/2P2P2/1P6/P2PP1PP/RNBKNRQB w FAfa - 1 9 ;D1 30 ;D2 579 ;D3 18481 ;D4 397545 ;D5 13257198 ;D6 311282465",
        "rbnkbnrq/pp2p1Np/2p2p2/8/3p4/8/PPPPPPPP/RBNKBR1Q w Aga - 0 9 ;D1 23 ;D2 670 ;D3 16435 ;D4 501883 ;D5 13012378 ;D6 411860744",
        "rk1bbnrq/ppp1pppp/n7/3p4/5P2/3P2NP/PPP1P1P1/RNKBB1RQ w GA - 0 9 ;D1 26 ;D2 597 ;D3 16238 ;D4 402506 ;D5 11269462 ;D6 296701249",
        "r1knbbrq/pppp2p1/2n1p2p/5p2/4P3/P1PP4/1P3PPP/RNKNBBRQ w GAga - 1 9 ;D1 20 ;D2 596 ;D3 13091 ;D4 399069 ;D5 9416862 ;D6 293659781",
        "rnknbrqb/p1p1pp1p/3p4/1p1N2p1/8/N7/PPPPPPPP/1RK1BRQB w Ffa - 0 9 ;D1 26 ;D2 724 ;D3 18942 ;D4 552040 ;D5 15257204 ;D6 461293885",
        "rbnknrb1/1p1ppp1p/p1p3p1/8/1P3P2/1R6/PqPPP1PP/RBNKN1BQ w Afa - 0 9 ;D1 31 ;D2 1183 ;D3 34723 ;D4 1289502 ;D5 38722152 ;D6 1421492227",
        "rnkbnrbq/2p1ppp1/p7/1p1p3p/3P4/1P4P1/P1P1PP1P/RNKBNRBQ w FAfa - 0 9 ;D1 24 ;D2 506 ;D3 12748 ;D4 301464 ;D5 8086100 ;D6 207129256",
        "r1knrbbq/pp1ppppp/2p1n3/8/2P3P1/P7/1PKPPP1P/RN1NRBBQ w ea - 0 9 ;D1 28 ;D2 570 ;D3 16037 ;D4 352471 ;D5 10278695 ;D6 242592363",
        "rnknrq1b/ppp1p1p1/4b3/3p1p1p/6P1/P4P2/1PPPPQ1P/RNKNR1BB w EAea - 2 9 ;D1 30 ;D2 739 ;D3 23124 ;D4 594962 ;D5 19252739 ;D6 521629794",
        "bbqr1krn/pppp1p1p/5n2/4p1p1/3P4/P3QP2/1PP1P1PP/BB1RNKRN w GDgd - 0 9 ;D1 31 ;D2 799 ;D3 25627 ;D4 674913 ;D5 22172123 ;D6 609277274",
        "bq1b1krn/pp1ppppp/3n4/2r5/3p3N/6N1/PPP1PPPP/BQRB1KR1 w GCg - 2 9 ;D1 21 ;D2 798 ;D3 18571 ;D4 688429 ;D5 17546069 ;D6 647165916",
        "bqrnkbrn/2pp1pp1/p7/1p2p2p/1P6/4N3/P1PPPPPP/BQR1KBRN w GCgc - 0 9 ;D1 27 ;D2 783 ;D3 22327 ;D4 670798 ;D5 20059741 ;D6 624462073",
        "bqr1krnb/1np1pppp/8/pp1p4/8/2P2N2/PP1PPPPP/BQRNKR1B w FCfc - 0 9 ;D1 28 ;D2 636 ;D3 18874 ;D4 461104 ;D5 14237097 ;D6 372181570",
        "qbb1rkrn/1ppppppp/p7/7n/8/P2P4/1PP1PPPP/QBBRNKRN w Gg - 0 9 ;D1 25 ;D2 547 ;D3 13837 ;D4 332918 ;D5 8849383 ;D6 229112926",
        "1rbbnkrn/p1p1pp1p/2q5/1p1p2p1/8/2P3P1/PP1PPP1P/QRBBNKRN w GBgb - 2 9 ;D1 24 ;D2 1010 ;D3 24370 ;D4 983770 ;D5 24328258 ;D6 961371180",
        "qrb1kbrn/ppp1p2p/4npp1/3p4/8/1PP4P/PR1PPPP1/Q1BNKBRN w Ggb - 1 9 ;D1 18 ;D2 451 ;D3 9291 ;D4 247310 ;D5 5568106 ;D6 155744022",
        "qr2krnb/p1p1pppp/b1np4/1p6/3NP3/7P/PPPP1PP1/QRBNKR1B w FBfb - 2 9 ;D1 25 ;D2 667 ;D3 17081 ;D4 476030 ;D5 12458875 ;D6 361495148",
        "qbrnbkrn/ppp3pp/3p4/5p2/2P1pP2/6PP/PP1PP3/QBRNBKRN w GCgc - 0 9 ;D1 24 ;D2 650 ;D3 16835 ;D4 445263 ;D5 12187382 ;D6 326834539",
        "qrnb1krn/ppp1p1pp/5p2/2Np4/b2P4/2P5/PP2PPPP/QR1BBKRN w GBgb - 0 9 ;D1 27 ;D2 641 ;D3 17490 ;D4 432041 ;D5 12103076 ;D6 310695797",
        "qrnkbbrn/pp2pp2/8/2pp2pp/6PP/3P4/PPPKPP2/QRN1BBRN w gb - 0 9 ;D1 22 ;D2 554 ;D3 13116 ;D4 357404 ;D5 9014737 ;D6 258925091",
        "qrnkbrnb/p1p1ppp1/1p6/3p4/3P3p/5N1P/PPP1PPP1/QRNKBR1B w FBfb - 0 9 ;D1 24 ;D2 529 ;D3 13205 ;D4 318722 ;D5 8295874 ;D6 213856651",
        "qbr1krbn/1pppp1pp/p7/5pn1/2PP4/8/PPB1PPPP/Q1RNKRBN w FCfc - 0 9 ;D1 26 ;D2 831 ;D3 21651 ;D4 696830 ;D5 18961456 ;D6 621884383",
        "1rnbkrbn/1qp1pppp/3p4/pp6/4P3/1NP4P/PP1P1PP1/QR1BKRBN w FBfb - 0 9 ;D1 24 ;D2 597 ;D3 15089 ;D4 404761 ;D5 10832084 ;D6 307793179",
        "q1rkrbbn/ppp1pppp/8/3p4/1PnP4/P7/1RP1PPPP/Q1NKRBBN w Ee - 1 9 ;D1 20 ;D2 520 ;D3 10769 ;D4 278067 ;D5 6452205 ;D6 170268300",
        "qrnkrn1b/ppppp1pp/4b3/7P/6p1/P7/1PPPPP2/QRNKRNBB w EBeb - 0 9 ;D1 26 ;D2 566 ;D3 15623 ;D4 381312 ;D5 10940750 ;D6 287987207",
        "bbr1nkrn/ppp1pppp/3q4/3p4/8/P7/1PPPPPPP/BBRQNRKN w gc - 5 9 ;D1 19 ;D2 661 ;D3 13895 ;D4 460396 ;D5 10870247 ;D6 356399665",
        "brqbnkrn/pp1pp2p/5pp1/2p5/4P3/P2P1N2/1PP2PPP/BRQB1KRN w GBgb - 0 9 ;D1 27 ;D2 679 ;D3 19916 ;D4 527306 ;D5 16391730 ;D6 455940859",
        "2qnkbrn/p1pppppp/8/1r6/1p2bP2/7N/PPPPP1PP/BR1QKBRN w GBg - 4 9 ;D1 18 ;D2 774 ;D3 15713 ;D4 635461 ;D5 14371755 ;D6 559579332",
        "r1qnkr1b/p1pppppp/7n/1p6/8/1P3b1N/PRPPPPPP/B1QNK1RB w f - 5 9 ;D1 21 ;D2 677 ;D3 15437 ;D4 501520 ;D5 12463801 ;D6 410795298",
        "rbbqn1rn/pppp1pp1/3k4/4p2Q/2PPP3/8/PP3PPP/RBB1NKRN w GA - 1 9 ;D1 40 ;D2 742 ;D3 28757 ;D4 579833 ;D5 21852196 ;D6 471452088",
        "rqbbnkrn/3pppp1/p1p4p/1p6/5P2/P2N4/1PPPP1PP/RQBBK1RN w ga - 0 9 ;D1 23 ;D2 665 ;D3 16400 ;D4 492544 ;D5 12794736 ;D6 396640086",
        "r2nkbrn/pp2pppp/8/2ppqb2/2P3P1/5P2/PP1PPN1P/RQB1KBRN w GAga - 3 9 ;D1 28 ;D2 1108 ;D3 31164 ;D4 1194581 ;D5 34780853 ;D6 1292405738",
        "rqbnk1nb/p1pppr1p/5p2/1p4p1/1PP1P3/8/P2P1PPP/RQBNKRNB w FAa - 1 9 ;D1 26 ;D2 650 ;D3 18208 ;D4 491403 ;D5 14565370 ;D6 416833400",
        "rbqnb1rn/p1pp1kpp/1p2pp2/8/4P2P/P5P1/1PPP1P2/RBQNBKRN w GA - 0 9 ;D1 20 ;D2 437 ;D3 9423 ;D4 222154 ;D5 5282124 ;D6 132309824",
        "rqnbbkrn/p1p1pppp/3p4/1p5B/8/1P1NP3/P1PP1PPP/RQ2BKRN w GAga - 0 9 ;D1 30 ;D2 606 ;D3 18382 ;D4 422491 ;D5 12989786 ;D6 326601372",
        "rqnkbbr1/ppppp1pp/5p2/7n/8/2PNP2P/PP1P1PP1/RQ1KBBRN w GAga - 1 9 ;D1 23 ;D2 482 ;D3 12506 ;D4 297869 ;D5 8430874 ;D6 217797292",
        "r1nkbrnb/2ppppp1/1q6/pp5p/1P6/P3P3/2PPKPPP/RQN1BRNB w fa - 2 9 ;D1 25 ;D2 827 ;D3 21518 ;D4 701071 ;D5 19290675 ;D6 632892337",
        "rbqnkrbn/p1ppppp1/7p/1p6/7P/2N1P3/PPPP1PPB/RBQ1KR1N w FAfa - 1 9 ;D1 30 ;D2 627 ;D3 18566 ;D4 440217 ;D5 12976682 ;D6 337377291",
        "r1nbkrbn/p1qp1ppp/8/1pp1p3/2P1P3/6P1/PP1PBP1P/RQN1KRBN w FAfa - 2 9 ;D1 22 ;D2 616 ;D3 14503 ;D4 431199 ;D5 10850952 ;D6 335943324",
        "rqnkr1bn/ppp1ppb1/3p2pp/8/P7/2P2P2/1PKPP1PP/RQN1RBBN w ea - 1 9 ;D1 31 ;D2 679 ;D3 21365 ;D4 493500 ;D5 15661072 ;D6 379844460",
        "r2krnbb/qppp1ppp/1n6/p3p3/PP6/4N3/N1PPPPPP/RQ1KR1BB w EAea - 4 9 ;D1 24 ;D2 645 ;D3 17054 ;D4 487028 ;D5 13837270 ;D6 416239106",
        "bbr1qk1n/1ppppp1p/2n5/p7/P7/1P2P3/2PP1PrP/1BRNQKRN w GCc - 0 9 ;D1 18 ;D2 520 ;D3 10680 ;D4 304462 ;D5 7215306 ;D6 207612575",
        "brnbq1rn/2ppppkp/p5p1/1p6/8/1BP3P1/PP1PPP1P/BRN1QRKN w - - 0 9 ;D1 21 ;D2 625 ;D3 13989 ;D4 419667 ;D5 9929336 ;D6 300902534",
        "brn1kbrn/pp2p1pp/3p4/q1p2p2/2P4P/6P1/PP1PPP2/BRNQKBRN w GBgb - 1 9 ;D1 18 ;D2 477 ;D3 10205 ;D4 273925 ;D5 6720181 ;D6 187205941",
        "brn1krnb/p3pppp/1qpp4/1p6/2P3P1/1P6/P2PPP1P/BRNQKRNB w FBfb - 1 9 ;D1 30 ;D2 835 ;D3 24761 ;D4 716151 ;D5 21806428 ;D6 654487872",
        "r1b1qkrn/1p1ppppp/p1p1n3/8/4P3/1PN5/P1PPQPPb/RBB2KRN w GAga - 0 9 ;D1 28 ;D2 825 ;D3 24536 ;D4 716585 ;D5 22079005 ;D6 647939781",
        "r1bbqk1n/p1pppprp/n7/1p4p1/5P2/2N3N1/PPPPP1PP/1RBBQKR1 w Ga - 4 9 ;D1 25 ;D2 545 ;D3 14657 ;D4 358854 ;D5 10271111 ;D6 273864588",
        "rnbqkbrn/p1pp1pp1/4p3/7p/2p4P/2P5/PP1PPPP1/R1BQKBRN w GAga - 0 9 ;D1 17 ;D2 445 ;D3 9076 ;D4 255098 ;D5 5918310 ;D6 174733195",
        "rnbqkrnb/1p1pp1p1/2p4p/p4p2/3P2P1/7N/PPPBPP1P/RN1QKR1B w FAfa - 0 9 ;D1 34 ;D2 746 ;D3 25319 ;D4 623133 ;D5 21285553 ;D6 569141201",
        "rbnqbkr1/1ppppp2/p5n1/6pp/4P3/1N6/PPPP1PPP/RBQ1BRKN w ga - 2 9 ;D1 18 ;D2 466 ;D3 9683 ;D4 260864 ;D5 6051500 ;D6 170135726",
        "rnqb1krn/ppppp1p1/7p/7b/P1P2pPP/8/1P1PPP2/RNQBBKRN w GAga - 0 9 ;D1 24 ;D2 575 ;D3 15400 ;D4 385825 ;D5 11039042 ;D6 291243811",
        "rnqkbbr1/p1pp1ppp/4p3/1p6/P3P2n/5P2/1PPP1NPP/RNQKBBR1 w GAga - 2 9 ;D1 27 ;D2 803 ;D3 22883 ;D4 694449 ;D5 20666099 ;D6 638696065",
        "rn1kbrnb/1qppp1pp/1p6/p4p2/1B1P4/1P5N/P1P1PPPP/RNQK1R1B w FAfa - 0 9 ;D1 37 ;D2 1209 ;D3 43015 ;D4 1425600 ;D5 49748034 ;D6 1671593862",
        "rbnqkrbn/Bppp1p2/p5pp/4p3/5P2/6PP/PPPPP3/RBNQKR1N w FAfa - 0 9 ;D1 29 ;D2 720 ;D3 20434 ;D4 534148 ;D5 15384362 ;D6 421343249",
        "rnqbkr1n/1p1ppbpp/3p1p2/p7/8/1P6/P1PPPPPP/R1QBKRBN w FAfa - 0 9 ;D1 20 ;D2 657 ;D3 14424 ;D4 492678 ;D5 11843134 ;D6 413965054",
        "rnqkrb1n/ppppp3/6p1/5p1p/2b2P2/P1N5/1PPPP1PP/RQ1KRBBN w EAea - 1 9 ;D1 28 ;D2 749 ;D3 20684 ;D4 543151 ;D5 15379233 ;D6 417191461",
        "rnqk1nbb/1pp2ppp/3pr3/p3p3/3P1P2/2N3N1/PPP1P1PP/R1QKR1BB w EAa - 1 9 ;D1 29 ;D2 883 ;D3 26412 ;D4 815098 ;D5 25144295 ;D6 789705382",
        "bbr1kqrn/p1p1ppp1/1p2n2p/3p4/1P1P4/2N5/P1P1PPPP/BBR1KQRN w GCgc - 0 9 ;D1 22 ;D2 485 ;D3 11475 ;D4 271271 ;D5 6825123 ;D6 171793012",
        "brnbkq1n/ppp1ppr1/7p/3p2p1/2P3PP/8/PPBPPP2/BRN1KQRN w GBb - 2 9 ;D1 30 ;D2 634 ;D3 19017 ;D4 442537 ;D5 13674310 ;D6 345386924",
        "brnkqbr1/1pppp1pp/5p2/p7/P1P1P2n/8/1P1P1PP1/BRNKQBRN w GBgb - 0 9 ;D1 21 ;D2 504 ;D3 11672 ;D4 305184 ;D5 7778289 ;D6 217596497",
        "b1rkqrnb/p1ppp1pp/1p1n4/5p2/5P2/PN5P/1PPPP1P1/BR1KQRNB w FBf - 0 9 ;D1 23 ;D2 688 ;D3 17259 ;D4 531592 ;D5 14228372 ;D6 451842354",
        "1bbnkqrn/rppppp2/p5p1/7p/7P/P1P1P3/1P1P1PP1/RBBNKQRN w GAg - 1 9 ;D1 25 ;D2 450 ;D3 12391 ;D4 263946 ;D5 7752404 ;D6 185393913",
        "rnbbkqr1/1pppppp1/7p/p3n3/PP5P/8/1BPPPPP1/RN1BKQRN w GAga - 0 9 ;D1 23 ;D2 543 ;D3 12224 ;D4 305812 ;D5 7549008 ;D6 199883770",
        "r1bkqbrn/ppppp1pp/8/5p2/3nPP2/1P4N1/P1PP2PP/RNBKQBR1 w GAga - 1 9 ;D1 27 ;D2 751 ;D3 21158 ;D4 600417 ;D5 17989920 ;D6 527273615",
        "rnbkqr1b/1p1pp1pp/p4p1n/2p5/1P5P/N4P2/P1PPP1P1/R1BKQRNB w FAfa - 0 9 ;D1 21 ;D2 498 ;D3 11738 ;D4 302278 ;D5 7808375 ;D6 216224115",
        "rbnkbqrn/p1p3pp/1p1p4/B3pp2/3P2P1/6N1/PPP1PP1P/RBNK1QR1 w GAga - 0 9 ;D1 34 ;D2 977 ;D3 33464 ;D4 961128 ;D5 33318567 ;D6 978991050",
        "r1kbbqrn/ppp3pp/2np1p2/1P2p3/3P1P2/8/P1P1P1PP/RNKBBQRN w GAga - 0 9 ;D1 32 ;D2 920 ;D3 28916 ;D4 844881 ;D5 26763259 ;D6 797524786",
        "rk1qbbrn/p2npppp/1p6/2p4Q/8/4P3/PPPP1PPP/RNK1B1RN w GA - 2 9 ;D1 35 ;D2 657 ;D3 22359 ;D4 495406 ;D5 16662477 ;D6 419496845",
        "rnk1brnb/pp1p1pp1/8/q1p1p2p/5P2/NP6/P1PPP1PP/R1KQBRNB w FAfa - 1 9 ;D1 26 ;D2 774 ;D3 20215 ;D4 610661 ;D5 16987110 ;D6 523437649",
        "rb1kqrbn/npp1ppp1/p7/3P3p/2PP4/8/PP3PPP/RBNKQRBN w FAfa - 0 9 ;D1 35 ;D2 775 ;D3 27395 ;D4 661118 ;D5 23983464 ;D6 625669222",
        "rnkb1rbn/pp1p2pp/8/2p1pp1q/P6P/1PN5/2PPPPP1/R1KBQRBN w FAfa - 1 9 ;D1 22 ;D2 899 ;D3 21188 ;D4 850597 ;D5 21518343 ;D6 857951339",
        "rnkqrbbn/1pppp1p1/8/p2N1p1p/2P4P/8/PP1PPPP1/R1KQRBBN w EAea - 0 9 ;D1 29 ;D2 585 ;D3 17571 ;D4 393221 ;D5 12238776 ;D6 299752383",
        "rnk1r1bb/pp1ppppp/1q4n1/2p5/5P1P/3PP3/PPP3P1/RNKQRNBB w EAea - 1 9 ;D1 27 ;D2 884 ;D3 24613 ;D4 811915 ;D5 23698701 ;D6 790239502",
        "bbrnkrqn/1ppp1p2/6pp/p3p3/5PP1/2PB4/PP1PP2P/B1RNKRQN w FCfc - 0 9 ;D1 37 ;D2 693 ;D3 25425 ;D4 550527 ;D5 20138432 ;D6 481498664",
        "b1rbkrqn/ppp2ppp/1n2p3/3p4/6P1/2PP4/PP2PP1P/BRNBKRQN w FBf - 1 9 ;D1 21 ;D2 463 ;D3 10610 ;D4 253204 ;D5 6307276 ;D6 159025909",
        "brnkrb1n/1pp1p1pp/3p4/p1Nq1p2/2P5/8/PP1PPPPP/BRK1RBQN w eb - 2 9 ;D1 27 ;D2 725 ;D3 17842 ;D4 496072 ;D5 12604078 ;D6 362747791",
        "brn1r1nb/ppppkppp/4p3/8/2PP1P2/8/PP1KP1PP/BRN1RQNB w - - 1 9 ;D1 25 ;D2 623 ;D3 16874 ;D4 426659 ;D5 12290985 ;D6 317097424",
        "rbb1krqn/1pp1pp1p/p3n1p1/3pP3/8/1PN5/P1PP1PPP/RBB1KRQN w FAfa d6 0 9 ;D1 23 ;D2 529 ;D3 12641 ;D4 310277 ;D5 7861413 ;D6 202594556",
        "r1bbkrqn/p1pppppp/8/4n3/1p5P/P2P2P1/1PP1PP2/RNBBKRQN w FAfa - 0 9 ;D1 23 ;D2 571 ;D3 13133 ;D4 346793 ;D5 8699448 ;D6 243460643",
        "rnbkrbqn/p1pp1ppp/4p3/1p6/8/BPN3P1/P1PPPP1P/R2KRBQN w EAea - 2 9 ;D1 29 ;D2 692 ;D3 20014 ;D4 500375 ;D5 14904192 ;D6 386694739",
        "rnbkrqn1/pppppp2/8/1Q2b1pp/P3P3/5P2/1PPP2PP/RNBKR1NB w EAea - 0 9 ;D1 37 ;D2 1001 ;D3 36440 ;D4 987842 ;D5 35626426 ;D6 993747544",
        "rbnkbrqn/p1pppp2/7p/1p4pP/3P1P2/8/PPP1P1P1/RBNKBRQN w FAfa - 0 9 ;D1 30 ;D2 564 ;D3 17143 ;D4 381364 ;D5 11859538 ;D6 293703269",
        "1nkbbrqn/3ppppp/r1p5/pp6/8/4PP2/PPPPN1PP/RNKBBRQ1 w FAf - 2 9 ;D1 26 ;D2 546 ;D3 14641 ;D4 344592 ;D5 9556962 ;D6 245137199",
        "rnkrbbq1/pppppnp1/7p/8/1B1Q1p2/3P1P2/PPP1P1PP/RNKR1B1N w DAda - 2 9 ;D1 43 ;D2 887 ;D3 36240 ;D4 846858 ;D5 33185346 ;D6 851927292",
        "1rkrbqnb/pppppp2/2n3p1/7p/3P3P/P4N2/1PP1PPP1/RNKRBQ1B w DAd - 0 9 ;D1 26 ;D2 622 ;D3 16049 ;D4 403921 ;D5 10786140 ;D6 285233838",
        "rbnkr1bn/pp1pqp1p/2p1p3/6p1/3P4/7P/PPP1PPP1/RBNKRQBN w EAea - 0 9 ;D1 19 ;D2 566 ;D3 12257 ;D4 381197 ;D5 9107175 ;D6 293397389",
        "r1kbrqb1/pppp2pp/2n1p1n1/5p1B/4PP2/P7/1PPP2PP/RNK1RQBN w EAea - 2 9 ;D1 39 ;D2 1359 ;D3 53626 ;D4 1876028 ;D5 73871486 ;D6 2633945690",
        "rnkrqbbn/p1p3pp/1p1ppp2/8/1P6/3P2P1/PKP1PP1P/RN1RQBBN w da - 0 9 ;D1 26 ;D2 776 ;D3 20735 ;D4 611907 ;D5 16884013 ;D6 503561996",
        "rnkrqnbb/ppp2p1p/3p4/4p1p1/3P3P/N1Q5/PPP1PPP1/R1KR1NBB w DAda - 0 9 ;D1 40 ;D2 1175 ;D3 45637 ;D4 1375884 ;D5 52620163 ;D6 1633655838",
        "bbrnkrn1/p1pppp2/1p6/6pp/3q4/1P3QP1/P1PPPP1P/BBRNKRN1 w FCfc - 0 9 ;D1 34 ;D2 1398 ;D3 45749 ;D4 1712950 ;D5 57268492 ;D6 2059942014",
        "br1bkrnq/1p2pppp/pnp5/3p4/P1P5/5P2/1P1PPKPP/BRNB1RNQ w fb - 2 9 ;D1 24 ;D2 501 ;D3 12237 ;D4 284936 ;D5 7049659 ;D6 177940764",
        "brnkrbn1/pppppp1q/B6p/6p1/8/1P2PP2/P1PP2PP/BRNKR1NQ w EBeb - 0 9 ;D1 34 ;D2 815 ;D3 25868 ;D4 700970 ;D5 22006883 ;D6 639803952",
        "br1krnqb/pppppp1p/1n4p1/8/8/P2NN3/2PPPPPP/BR1K1RQB w Beb - 2 9 ;D1 37 ;D2 1029 ;D3 36748 ;D4 1025712 ;D5 36214583 ;D6 1026195877",
        "rbbnkr1q/p1p2ppp/1p1ppn2/8/1PP4P/8/P2PPPP1/RBBNKRNQ w FAfa - 0 9 ;D1 28 ;D2 755 ;D3 22623 ;D4 605106 ;D5 18972778 ;D6 513486101",
        "r1b1krnq/pp2pppp/1bn5/2pp4/4N3/5P2/PPPPPRPP/R1BBK1NQ w Afa - 0 9 ;D1 24 ;D2 705 ;D3 17427 ;D4 532521 ;D5 13532966 ;D6 426443376",
        "1nbkrbn1/rpppppqp/p7/6p1/4P3/3P2P1/PPP1KP1P/RNB1RBNQ w e - 1 9 ;D1 31 ;D2 800 ;D3 24748 ;D4 693366 ;D5 21193292 ;D6 625757852",
        "r1bkrnqb/pp3ppp/n1ppp3/8/1P5P/P7/R1PPPPP1/1NBKRNQB w Eea - 0 9 ;D1 21 ;D2 482 ;D3 11417 ;D4 275339 ;D5 7112890 ;D6 180378139",
        "rbnkbrnq/ppp1p2p/5p2/3p2p1/1B1P4/1N4P1/PPP1PP1P/RB1K1RNQ w FAfa - 0 9 ;D1 33 ;D2 780 ;D3 25532 ;D4 628945 ;D5 20756770 ;D6 535497008",
        "rnk1brnq/pp1ppppp/2p5/b7/8/1P2P2P/P1PP1PPQ/RNKBBRN1 w FAfa - 3 9 ;D1 29 ;D2 648 ;D3 19043 ;D4 449637 ;D5 13722785 ;D6 341389148",
        "rnkrbbnq/p1p3pp/5p2/1p1pp3/P7/1PN2P2/2PPP1PP/R1KRBBNQ w DAda - 0 9 ;D1 26 ;D2 827 ;D3 21865 ;D4 683167 ;D5 18916370 ;D6 589161126",
        "r1krbnqb/p1pp1ppp/2n1p3/8/1p4P1/PPP5/3PPP1P/RNKRBNQB w DAda - 1 9 ;D1 25 ;D2 540 ;D3 14709 ;D4 331332 ;D5 9491817 ;D6 225389422",
        "rbnkrnbq/ppp1pp2/3p2p1/2N5/P6p/2P5/1P1PPPPP/RB1KRNBQ w EAea - 0 9 ;D1 32 ;D2 790 ;D3 25107 ;D4 661207 ;D5 20906017 ;D6 578332225",
        "rnkbrn1q/1ppppppb/8/p4N1p/8/P1N5/1PPPPPPP/R1KBR1BQ w EAea - 0 9 ;D1 31 ;D2 691 ;D3 20813 ;D4 510665 ;D5 15308408 ;D6 404129987",
        "rnkrnbbq/p1p2ppp/3pp3/1p6/6P1/4PQ1B/PPPP1P1P/RNKRN1B1 w DAda - 0 9 ;D1 29 ;D2 558 ;D3 16800 ;D4 352887 ;D5 10825379 ;D6 246965507",
        "rnkrnqbb/pp2p1p1/3p3p/2p2p2/5P2/1P1N4/P1PPPQPP/RNKR2BB w DAda - 0 9 ;D1 29 ;D2 762 ;D3 23210 ;D4 644936 ;D5 20522675 ;D6 596067005",
        "bb1rknnr/ppqppppp/8/2p5/3P1N2/1P6/P1P1PPPP/BBQRKN1R w HDhd - 1 9 ;D1 33 ;D2 963 ;D3 32279 ;D4 1000890 ;D5 34552118 ;D6 1124738493",
        "bqrbknnr/ppp1p2p/8/3p1p2/5p2/P3N2P/1PPPP1P1/BQRBK1NR w HChc - 0 9 ;D1 20 ;D2 398 ;D3 9009 ;D4 194859 ;D5 4834319 ;D6 113660536",
        "b1rk1bnr/qpp1pppp/p4n2/3p4/3PPP2/7N/PPP3PP/BQRKNB1R w HChc - 1 9 ;D1 25 ;D2 648 ;D3 16587 ;D4 455720 ;D5 12200870 ;D6 351766307",
        "bqkrnnrb/pppp2p1/4pp2/4P2p/6P1/7P/PPPP1P2/BQRKNNRB w GC - 1 9 ;D1 30 ;D2 493 ;D3 15118 ;D4 280726 ;D5 8786998 ;D6 181492621",
        "q1brknnr/1p1ppppp/p7/2p5/8/1PPP4/P2RPPPP/QBB1KNNR w Hhd - 0 9 ;D1 25 ;D2 501 ;D3 13206 ;D4 290463 ;D5 7982978 ;D6 192717198",
        "qrb1k1nr/ppppb1pp/6n1/4ppN1/3P4/4N3/PPP1PPPP/QRBBK2R w HBhb - 2 9 ;D1 31 ;D2 872 ;D3 26191 ;D4 739276 ;D5 22493014 ;D6 646855304",
        "1rbknbnr/1ppp1pp1/q6p/p3p3/5P2/2PPB3/PP2P1PP/QR1KNBNR w HBhb - 0 9 ;D1 28 ;D2 1020 ;D3 28147 ;D4 984000 ;D5 27484692 ;D6 947786800",
        "qrbk2rb/1ppp1ppp/5nn1/p3p3/1N6/P7/1PPPPPPP/QRB1KNRB w gb - 0 9 ;D1 23 ;D2 592 ;D3 14398 ;D4 395716 ;D5 10098215 ;D6 293988585",
        "qbrk1nnr/1pp1pppp/2b5/p2p4/P2P2P1/8/1PP1PP1P/QBKRBNNR w hc - 1 9 ;D1 26 ;D2 654 ;D3 18103 ;D4 471653 ;D5 13740891 ;D6 373081138",
        "qrkbbnnr/ppp2p1p/4p3/3p2p1/P7/2PP4/1P2PPPP/QRKBBNNR w HBhb - 0 9 ;D1 25 ;D2 626 ;D3 16616 ;D4 431634 ;D5 12079406 ;D6 324006164",
        "qr1kbbnr/ppp1pp1p/4n1p1/2Pp4/6P1/4N3/PP1PPP1P/QRK1BBNR w HB d6 0 9 ;D1 26 ;D2 699 ;D3 18068 ;D4 497152 ;D5 13353359 ;D6 375702908",
        "qrk1b1rb/p1pppppp/3nnQ2/1p6/1P3P2/3P4/P1P1P1PP/1RKNBNRB w GBgb - 3 9 ;D1 43 ;D2 1369 ;D3 55463 ;D4 1831200 ;D5 71514365 ;D6 2427477375",
        "qbrk1nbr/pppp3p/5n2/4ppp1/3P1P2/4N3/PPP1P1PP/QBKRN1BR w hc - 0 9 ;D1 25 ;D2 752 ;D3 20165 ;D4 615263 ;D5 17493373 ;D6 543180234",
        "qrkb1nbr/1pppppQp/3n4/p7/5p2/1P1N4/P1PPP1PP/1RKB1NBR w HBhb - 0 9 ;D1 45 ;D2 946 ;D3 40100 ;D4 966903 ;D5 39736157 ;D6 1051910977",
        "qrk1nbbr/ppp1p1p1/4n2p/3p1p2/1P5P/3P2P1/P1P1PP2/QRKNNBBR w HBhb - 1 9 ;D1 32 ;D2 770 ;D3 25367 ;D4 646977 ;D5 21717615 ;D6 577979364",
        "qrkn1rbb/pp2pppp/2p5/3p4/P2Qn1P1/1P6/2PPPP1P/1RKNNRBB w FBfb - 0 9 ;D1 38 ;D2 943 ;D3 35335 ;D4 868165 ;D5 31909835 ;D6 798405123",
        "bbrqknnr/ppp4p/3pp3/5pp1/4PP2/5Q2/PPPP2PP/BBR1KNNR w HChc - 0 9 ;D1 36 ;D2 843 ;D3 29974 ;D4 758528 ;D5 26828059 ;D6 723306114",
        "1rqbkn1r/p1p1pppp/1p5n/P2p4/3Pb1P1/8/1PP1PP1P/BRQBKNNR w HBhb - 0 9 ;D1 23 ;D2 778 ;D3 19482 ;D4 649789 ;D5 17337683 ;D6 579112676",
        "br1knbnr/1qp1pppp/pp1p4/8/8/PP6/2PPPPPP/BRQKNBNR w HBhb - 2 9 ;D1 26 ;D2 697 ;D3 18835 ;D4 546622 ;D5 15280079 ;D6 473071890",
        "brqk2rb/ppppp1pp/4np2/8/2n5/3P1Q2/PP2PPPP/BR1KNNRB w GBgb - 0 9 ;D1 32 ;D2 948 ;D3 30434 ;D4 885713 ;D5 29821322 ;D6 874251866",
        "r1bqknnr/pp1pp1p1/5p1p/2p1b2N/2P5/8/PPQPPPPP/RBB1K1NR w HAha - 0 9 ;D1 31 ;D2 785 ;D3 25549 ;D4 659952 ;D5 22244193 ;D6 592797491",
        "rqbbknnr/ppppp2p/5pp1/8/8/1P3PP1/PQPPP2P/R1BBKNNR w HAha - 0 9 ;D1 23 ;D2 391 ;D3 10163 ;D4 198450 ;D5 5576671 ;D6 121267576",
        "rqbknbnr/1pp1p2p/p7/3p1pp1/7N/1PP5/P2PPPPP/RQBK1BNR w HAha - 0 9 ;D1 27 ;D2 676 ;D3 19606 ;D4 522428 ;D5 15955388 ;D6 448477218",
        "rqb1nnrb/2ppkppp/1p2p3/p7/2PPP3/1P6/P4PPP/RQBKNNRB w GA - 1 9 ;D1 31 ;D2 727 ;D3 22895 ;D4 570647 ;D5 18361051 ;D6 483248153",
        "rb1kbn1r/p1ppppp1/qp5n/7p/P7/RPP5/3PPPPP/1BQKBNNR w Hha - 2 9 ;D1 29 ;D2 837 ;D3 23815 ;D4 730083 ;D5 21279560 ;D6 682863811",
        "rqkbb1nr/p1p2ppp/1p1p2n1/3Np3/4P3/5N2/PPPP1PPP/RQKBB2R w HAha - 0 9 ;D1 28 ;D2 717 ;D3 20663 ;D4 550987 ;D5 16347343 ;D6 453153783",
        "rqknbbr1/p1pppp1p/1p3np1/8/4P3/2P2P1P/PP1P2P1/RQKNBBNR w HAa - 0 9 ;D1 27 ;D2 650 ;D3 18231 ;D4 475303 ;D5 13847463 ;D6 383256006",
        "r1k1bnrb/1qpppppp/1p2n3/p7/1P5P/6P1/P1PPPP2/RQKNBNR1 w GAga - 1 9 ;D1 24 ;D2 806 ;D3 20693 ;D4 713220 ;D5 19382263 ;D6 686009788",
        "rb1knnbr/1pp1ppp1/p2p3p/5q2/3B2P1/3P1P2/PPP1P2P/RBQKNN1R w HAha - 0 9 ;D1 34 ;D2 1360 ;D3 44096 ;D4 1605706 ;D5 51973672 ;D6 1837704407",
        "rqkb1nbr/p1p1ppp1/1p3n1p/2Qp4/8/2P5/PP1PPPPP/R1KBNNBR w HAha - 2 9 ;D1 39 ;D2 983 ;D3 38218 ;D4 940989 ;D5 36347815 ;D6 918801645",
        "rqknnbbr/2pppp2/pp5p/6p1/1P1P4/4PP2/P1P3PP/RQKNNBBR w HAha - 0 9 ;D1 26 ;D2 628 ;D3 17638 ;D4 464924 ;D5 13787303 ;D6 386125234",
        "rqkn1rbb/1pp1pppp/p7/3p4/3Pn3/2P1PP2/PP4PP/RQKNNRBB w FAfa - 1 9 ;D1 20 ;D2 527 ;D3 12216 ;D4 321533 ;D5 8082183 ;D6 219311659",
        "bbrkqn1r/1pppppp1/5n2/p7/1PP2P1p/7N/P2PP1PP/BBRKQN1R w HChc - 1 9 ;D1 36 ;D2 963 ;D3 35291 ;D4 973839 ;D5 35907489 ;D6 1034223364",
        "brkbqn1r/p2ppppp/7n/1p6/P1p3PP/8/1PPPPP1N/BRKBQ1NR w HBhb - 0 9 ;D1 18 ;D2 583 ;D3 11790 ;D4 394603 ;D5 8858385 ;D6 304339862",
        "brkq1bnr/pp1ppp1p/8/2p2np1/P7/8/1PPPPPPP/BRKQNBNR w HBhb - 0 9 ;D1 19 ;D2 552 ;D3 11811 ;D4 354260 ;D5 8432183 ;D6 262293169",
        "brkqnnrb/1ppppppp/8/8/p3P3/5N2/PPPP1PPP/BRKQ1NRB w GBgb - 3 9 ;D1 21 ;D2 397 ;D3 9653 ;D4 204350 ;D5 5489836 ;D6 128389738",
        "rbbkq1nr/1p2pppp/p1p3nB/3p4/1Q1P4/6N1/PPP1PPPP/RB1K2NR w HAha - 0 9 ;D1 40 ;D2 1132 ;D3 43404 ;D4 1260470 ;D5 47425783 ;D6 1415578783",
        "rkbbq1nr/1pppp1p1/4np2/p6p/8/PP3P2/1KPPP1PP/R1BBQNNR w ha - 0 9 ;D1 24 ;D2 596 ;D3 15220 ;D4 402121 ;D5 10822049 ;D6 302056813",
        "r1bqn1nr/pkpppp1p/1p4pb/8/PN6/R7/1PPPPPPP/1KBQ1BNR w H - 2 9 ;D1 33 ;D2 794 ;D3 25450 ;D4 649150 ;D5 20919309 ;D6 561073410",
        "rkb1nnrb/1pppq1pp/p4p2/4p3/5P2/1P1PB3/P1P1P1PP/RK1QNNRB w GAga - 0 9 ;D1 26 ;D2 625 ;D3 17050 ;D4 442036 ;D5 12515042 ;D6 342967558",
        "rbkqbn1r/pppp1p1p/2n1p1p1/8/8/1P1PP1N1/P1P2PPP/RBKQB1NR w HAha - 1 9 ;D1 30 ;D2 660 ;D3 20308 ;D4 492714 ;D5 15348335 ;D6 403323883",
        "rkqbb1n1/pppppppr/8/6np/5P2/8/PPPPP1PP/RKQBBNNR w HAa - 6 9 ;D1 23 ;D2 500 ;D3 12154 ;D4 292936 ;D5 7519117 ;D6 196524441",
        "rkqnbbnr/ppppppp1/8/7p/3N4/6PP/PPPPPP2/RKQNBB1R w HAa - 0 9 ;D1 24 ;D2 484 ;D3 12495 ;D4 284570 ;D5 7775173 ;D6 193947530",
        "rkqnb1rb/p1p1pppp/1p1p4/2n5/3P4/2P1N1N1/PP2PPPP/RKQ1B1RB w GAga - 0 9 ;D1 28 ;D2 1020 ;D3 29124 ;D4 1027904 ;D5 30515456 ;D6 1073711823",
        "rbk1nnbr/1ppq1ppp/p2p4/4p3/P3B2P/2P5/1P1PPPP1/R1KQNNBR w HAha - 2 9 ;D1 38 ;D2 998 ;D3 37265 ;D4 1047592 ;D5 38552638 ;D6 1139322479",
        "r1qbn1br/k1pppppp/6n1/pp6/5P1P/P7/1PPPP1PB/RKQBNN1R w HA - 1 9 ;D1 22 ;D2 549 ;D3 12867 ;D4 348574 ;D5 8725809 ;D6 251613569",
        "rkqnn1br/pppp3p/4p1pb/5p2/P2P4/7P/1PP1PPPB/RKQNNB1R w HAha - 1 9 ;D1 32 ;D2 659 ;D3 21249 ;D4 469701 ;D5 15434721 ;D6 365761521",
        "rk1nnrbb/p1p1pppp/1p6/3p1q2/P3P3/2NN4/1PPP1PPP/RKQ2RBB w FAfa - 3 9 ;D1 29 ;D2 989 ;D3 29087 ;D4 980477 ;D5 29643404 ;D6 998848556",
        "bbrk1q1r/ppppppp1/3n4/7p/3Pn3/6PN/PPP1PPNP/BBRK1Q1R w HChc - 2 9 ;D1 23 ;D2 712 ;D3 16551 ;D4 516177 ;D5 12995202 ;D6 411077508",
        "brkbnq1r/p1ppp2p/5ppn/1p6/5P2/1P1P2P1/P1P1P2P/BRKBNQNR w HBhb - 0 9 ;D1 28 ;D2 856 ;D3 24984 ;D4 780503 ;D5 23529352 ;D6 754501112",
        "br1k1bnr/ppppp1pp/4np2/1B2P2q/3P4/8/PPP2PPP/BRKNQ1NR w HB - 3 9 ;D1 36 ;D2 1214 ;D3 40615 ;D4 1328331 ;D5 45096834 ;D6 1470987023",
        "brk1qnrb/pnppp1p1/1p6/5p1p/8/5PPP/PPPPP1R1/BRKNQN1B w Bgb - 0 9 ;D1 22 ;D2 551 ;D3 13111 ;D4 353317 ;D5 9040545 ;D6 259643605",
        "rbbkn1nr/1ppp2pp/p3p3/2q2p2/3P4/6P1/PPPBPP1P/RB1KNQNR w HAha - 0 9 ;D1 31 ;D2 1060 ;D3 31332 ;D4 1015099 ;D5 30314172 ;D6 976268967",
        "rkbbn1nr/ppppp1pp/8/6N1/5p2/1q6/P1PPPPPP/RKBBN1QR w HAha - 0 9 ;D1 3 ;D2 72 ;D3 1919 ;D4 50827 ;D5 1400832 ;D6 39654253",
        "rkb2bnr/pp2pppp/2p1n3/3p4/q2P4/5NP1/PPP1PP1P/RKBNQBR1 w Aha - 0 9 ;D1 29 ;D2 861 ;D3 24504 ;D4 763454 ;D5 22763215 ;D6 731511256",
        "rkbq1nrb/ppppppp1/7p/8/1P1n4/P4P1P/2PPP1P1/RKBNQNRB w GAga - 0 9 ;D1 25 ;D2 672 ;D3 17631 ;D4 473864 ;D5 12954224 ;D6 361237536",
        "rbknb1nr/ppp1qp1p/6p1/3pp3/3P3P/2B1P3/PPP2PP1/RBKN1QNR w HAha - 1 9 ;D1 27 ;D2 857 ;D3 24688 ;D4 792538 ;D5 23790033 ;D6 768247869",
        "rknbbq1r/p1pppppp/1p2N3/8/3n4/2P5/PP1PPPPP/RK1BBQNR w HAha - 4 9 ;D1 29 ;D2 763 ;D3 22138 ;D4 574054 ;D5 16926075 ;D6 447896703",
        "r1nqbbnr/1pppp1pp/1k6/p4p2/8/4P3/PPPP1PPP/RKN1BBNR w HA - 0 9 ;D1 26 ;D2 658 ;D3 17302 ;D4 464039 ;D5 12380488 ;D6 349047256",
        "rkn2qrb/ppp1pppp/6n1/1b1p4/1P6/4PPB1/P1PP2PP/RKNQ1NRB w GAga - 3 9 ;D1 23 ;D2 574 ;D3 14070 ;D4 370324 ;D5 9501401 ;D6 263870337",
        "rbkn2br/ppppp1p1/4np1p/1P5q/8/2P1N3/P2PPPPP/RBK1QNBR w HAha - 1 9 ;D1 29 ;D2 992 ;D3 29506 ;D4 999564 ;D5 30148787 ;D6 1045942540",
        "1knbqnbr/1ppppp1p/r5p1/p7/7P/2PN2P1/PP1PPP2/RK1BQNBR w HAh - 2 9 ;D1 26 ;D2 698 ;D3 19395 ;D4 512023 ;D5 14848229 ;D6 402599313",
        "rk1qnbbr/pnpppp1p/6p1/1p6/3P4/1P6/P1P1PPPP/RKNQNBBR w HAha - 1 9 ;D1 20 ;D2 480 ;D3 11159 ;D4 287539 ;D5 7425917 ;D6 203194521",
        "rknqnrbb/pp1p2p1/5p1p/2p1p3/2P1P3/P2P4/1P3PPP/RKNQNRBB w FAfa - 0 9 ;D1 26 ;D2 679 ;D3 18116 ;D4 494953 ;D5 13790137 ;D6 392629571",
        "bbrk2qr/pp1p1ppp/3n2n1/2p1p3/3P1P2/6N1/PPP1P1PP/BBRKN1QR w HChc - 0 9 ;D1 26 ;D2 790 ;D3 21521 ;D4 673269 ;D5 19259490 ;D6 617563700",
        "b1krnnqr/1p1ppppp/p1p5/b6B/P7/4P1N1/1PPP1PPP/BRK1N1QR w HB - 2 9 ;D1 26 ;D2 625 ;D3 16451 ;D4 415452 ;D5 11490615 ;D6 304805107",
        "1rknnbqr/3ppppp/p7/1pp5/4b2P/P4P2/1PPPP1PR/BRKNNBQ1 w Bhb - 1 9 ;D1 24 ;D2 757 ;D3 19746 ;D4 618777 ;D5 17275100 ;D6 544309489",
        "br1nn1rb/pppkpqpp/3p1p2/8/PP6/4N3/1KPPPPPP/BR2NQRB w - - 3 9 ;D1 24 ;D2 682 ;D3 17129 ;D4 482711 ;D5 13057308 ;D6 375033550",
        "rbbkn1qr/pppp2p1/6np/4pp2/7N/7P/PPPPPPPR/RBBK1NQ1 w Aha - 0 9 ;D1 22 ;D2 586 ;D3 14158 ;D4 409891 ;D5 10607781 ;D6 324452612",
        "rk1bn1qr/pppbpppp/4n3/4p3/4P3/5P2/PPPP2PP/RKBB1NQR w HAha - 1 9 ;D1 22 ;D2 530 ;D3 13440 ;D4 348004 ;D5 9514787 ;D6 259898748",
        "rkbnnbqr/1ppp1ppp/p7/4p3/8/QP3P2/P1PPP1PP/RKBNNB1R w HAha - 0 9 ;D1 29 ;D2 705 ;D3 21511 ;D4 551042 ;D5 17524731 ;D6 472356665",
        "1kbnnqrb/1pp1p1pp/r4p2/p2p4/N4P2/3P4/PPP1P1PP/RKB1NQRB w GAg - 2 9 ;D1 21 ;D2 623 ;D3 14979 ;D4 437554 ;D5 11601134 ;D6 343214006",
        "rbknbn1r/pppp1p1p/4p1q1/8/P1P3Pp/8/1P1PPP2/RBKNBNQR w HAha - 0 9 ;D1 30 ;D2 813 ;D3 24959 ;D4 708454 ;D5 23379040 ;D6 692576573",
        "rk1bb1qr/2pppppp/p2nn3/1p4P1/6QP/8/PPPPPP2/RKNBBN1R w HAha - 2 9 ;D1 36 ;D2 857 ;D3 30124 ;D4 757524 ;D5 26485812 ;D6 696999449",
        "rkn1bbqr/p2ppppp/2p1n3/1p6/4PP2/6PP/PPPP4/RKNNBBQR w HAha - 0 9 ;D1 33 ;D2 687 ;D3 22744 ;D4 511018 ;D5 17101732 ;D6 412778368",
        "rkn1bqrb/pnp1pppp/3p4/8/Pp6/1N2NP2/1PPPP1PP/RK2BQRB w GAga - 0 9 ;D1 28 ;D2 591 ;D3 17174 ;D4 406025 ;D5 12182448 ;D6 312575205",
        "rbk1n1br/ppp1ppqp/2n5/2Np2p1/8/2P5/PPBPPPPP/R1KN1QBR w HAha - 4 9 ;D1 35 ;D2 930 ;D3 30663 ;D4 844433 ;D5 27160490 ;D6 780616047",
        "rknbn1br/1ppp1ppp/p3p3/8/1q6/2P2N1P/P2PPPP1/RKNB1QBR w HAha - 0 9 ;D1 4 ;D2 157 ;D3 3697 ;D4 138102 ;D5 3454704 ;D6 125373395",
        "rkn1qbbr/pp3ppp/4n3/2ppp3/4P1P1/P2P4/1PP2P1P/RKNNQBBR w HAha - 0 9 ;D1 28 ;D2 840 ;D3 24437 ;D4 771328 ;D5 23200961 ;D6 756489357",
        "rkn1qrbb/pp1ppp2/2p1n1p1/7p/2P2P1P/6P1/PP1PP3/RKNNQRBB w FAfa - 1 9 ;D1 32 ;D2 867 ;D3 27595 ;D4 757836 ;D5 24485663 ;D6 688115847",
        "b1rknnrq/bpppp1p1/p6p/5p1P/6P1/4N3/PPPPPP2/BBRKN1RQ w GCgc - 1 9 ;D1 33 ;D2 851 ;D3 28888 ;D4 763967 ;D5 26686205 ;D6 731944177",
        "brkb1nr1/pppppp2/3n2pp/3B4/1P6/4P3/PqPP1PPP/BRK1NNRQ w GBgb - 2 9 ;D1 4 ;D2 98 ;D3 2965 ;D4 76143 ;D5 2352530 ;D6 64251468",
        "brk1nbrq/1ppppn1p/6p1/p4p2/P5P1/5R2/1PPPPP1P/BRKNNB1Q w Bgb - 0 9 ;D1 29 ;D2 922 ;D3 27709 ;D4 879527 ;D5 27463717 ;D6 888881062",
        "brkn1rqb/1p1ppppp/3n4/p1p5/1P3P2/8/PNPPP1PP/BR1KNRQB w fb - 1 9 ;D1 29 ;D2 633 ;D3 19399 ;D4 469818 ;D5 15076198 ;D6 396737074",
        "rb1k1nrq/pbp1pppp/1p1p1n2/8/5P2/4NN1P/PPPPP1P1/RBBK2RQ w GAga - 2 9 ;D1 28 ;D2 841 ;D3 24056 ;D4 710751 ;D5 20772996 ;D6 613798447",
        "rkbbnnrq/p1pp3p/4p1p1/1p3p2/P6P/1P6/1BPPPPP1/RK1BNNRQ w GAga - 0 9 ;D1 33 ;D2 957 ;D3 30668 ;D4 907217 ;D5 29735654 ;D6 903933626",
        "rk2nbrq/p1ppppp1/bpn5/7p/6P1/2N2P2/PPPPP1QP/RKB1NBR1 w GAga - 2 9 ;D1 24 ;D2 687 ;D3 18206 ;D4 544627 ;D5 15518417 ;D6 484217179",
        "rkbn1r1b/pp1pppnp/6q1/2p3p1/5P1P/4N3/PPPPP1P1/RKB1NRQB w FAfa - 1 9 ;D1 23 ;D2 831 ;D3 21254 ;D4 754622 ;D5 21126103 ;D6 744755212",
        "rbknb1rq/ppp1p1p1/3pnp1p/8/6PP/2PP4/PP2PP2/RBKNBNRQ w GAga - 0 9 ;D1 31 ;D2 838 ;D3 26800 ;D4 736910 ;D5 24008129 ;D6 677776408",
        "rknbb1rq/p1pn1ppp/4p3/1p1p4/2P5/1P2N1P1/P2PPP1P/RKNBB1RQ w GAga - 1 9 ;D1 29 ;D2 830 ;D3 24798 ;D4 721630 ;D5 22243832 ;D6 660040360",
        "rk1nbbrq/pp1p1ppp/3n4/P3p3/2p4P/8/1PPPPPP1/RKNNBBRQ w GAga - 1 9 ;D1 24 ;D2 484 ;D3 12776 ;D4 297419 ;D5 8379748 ;D6 214004367",
        "rknnbr1b/ppp2pqp/3p4/4p1p1/7P/3P1P2/PPP1P1P1/RKNNBRQB w FAfa - 0 9 ;D1 32 ;D2 838 ;D3 26408 ;D4 740701 ;D5 23472124 ;D6 699211365",
        "rb1k1rbq/ppppN1pp/2nn4/5p2/7P/8/PPPPPPP1/RBK1NRBQ w FA - 1 9 ;D1 27 ;D2 800 ;D3 22785 ;D4 701742 ;D5 20804424 ;D6 660917073",
        "r1nbnrbq/kppppp1p/6p1/8/p1PP1P2/4P3/PP4PP/RKNBNRBQ w FA - 1 9 ;D1 28 ;D2 757 ;D3 21198 ;D4 602699 ;D5 17180857 ;D6 507618340",
        "rkn1rbbq/p1pppppp/2n5/1pP5/8/1N2P3/PP1P1PPP/RK1NRBBQ w EAea - 1 9 ;D1 22 ;D2 483 ;D3 11890 ;D4 283679 ;D5 7497674 ;D6 191130942",
        "rknnrqbb/2pppppp/8/p7/Np3P2/3P4/PPP1P1PP/RKN1RQBB w EAea - 0 9 ;D1 25 ;D2 536 ;D3 14456 ;D4 339180 ;D5 9694947 ;D6 245669668",
        "bb1rknrn/1qppppp1/1p4B1/p6N/8/2P5/PP1PPPPP/B1QRK1RN w GDgd - 1 9 ;D1 32 ;D2 715 ;D3 22421 ;D4 575008 ;D5 17860156 ;D6 502410909",
        "b1rbknrn/qpp1ppp1/p6p/3p4/2P5/1P1P1P2/P3P1PP/BQRBKNRN w GCgc - 0 9 ;D1 30 ;D2 818 ;D3 24421 ;D4 688711 ;D5 20981488 ;D6 611986786",
        "bqkrnbrn/1pp1pp1p/p7/1B1p2p1/4P3/7P/PPPP1PP1/BQKRN1RN w - - 0 9 ;D1 28 ;D2 676 ;D3 18366 ;D4 478054 ;D5 13126287 ;D6 363765666",
        "bqrknrnb/1p2ppp1/p1pp3p/8/3P1P2/1PP5/P3P1PP/BQRKNRNB w FCfc - 0 9 ;D1 31 ;D2 646 ;D3 20686 ;D4 455607 ;D5 14984618 ;D6 349082278",
        "qbbrkn1r/pppppp1p/8/6p1/2P1Pn1P/6N1/PP1P1PP1/QBBRKNR1 w GDd - 3 9 ;D1 20 ;D2 532 ;D3 11581 ;D4 303586 ;D5 7512432 ;D6 202967948",
        "1rbbknr1/p1ppp1pp/1pq2pn1/8/3P4/P3P3/QPP2PPP/1RBBKNRN w GBgb - 3 9 ;D1 31 ;D2 1002 ;D3 30581 ;D4 999607 ;D5 30642468 ;D6 1009228283",
        "qrbkn1rn/pppp1ppp/8/6b1/P1P1Pp2/8/1P1P2PP/QRBKNBRN w GBgb - 0 9 ;D1 22 ;D2 505 ;D3 12447 ;D4 304863 ;D5 8192621 ;D6 214730959",
        "qrbk1rnb/p2ppp1p/5n2/1pp3p1/8/7P/PPPPPPPN/QRBKR1NB w Bfb - 0 9 ;D1 20 ;D2 619 ;D3 13448 ;D4 449630 ;D5 10571176 ;D6 369603424",
        "qbrkb1r1/ppp2ppp/3pn1n1/P3p3/4P3/3P4/1PP2PPP/QBRKBNRN w GCgc - 1 9 ;D1 26 ;D2 755 ;D3 20596 ;D4 604483 ;D5 17164382 ;D6 510878835",
        "qrkbb1r1/ppp1pnpp/3p2n1/5p2/1P3P2/2Q3N1/P1PPP1PP/1RKBB1RN w GBgb - 0 9 ;D1 35 ;D2 918 ;D3 32244 ;D4 870888 ;D5 30933394 ;D6 867833733",
        "qrknbbrn/ppp1ppp1/8/7p/2Bp4/4PPP1/PPPP3P/QRKNB1RN w GBgb - 0 9 ;D1 27 ;D2 593 ;D3 16168 ;D4 376808 ;D5 10422676 ;D6 258348640",
        "qrk1brnb/ppppp3/4n2p/5pp1/2PP4/2N4P/PP2PPP1/QRK1BRNB w FBfb - 2 9 ;D1 24 ;D2 672 ;D3 17447 ;D4 506189 ;D5 13765777 ;D6 414930519",
        "qbrknrb1/p2ppppp/2p3n1/8/p4P2/6PP/1PPPP3/QBRKNRBN w FCfc - 0 9 ;D1 29 ;D2 759 ;D3 23235 ;D4 634493 ;D5 20416668 ;D6 584870558",
        "1rkb1rbn/p1pp1ppp/3np3/1p6/4qP2/3NB3/PPPPPRPP/QRKB3N w Bfb - 0 9 ;D1 22 ;D2 923 ;D3 22585 ;D4 914106 ;D5 24049880 ;D6 957218571",
        "1rknrbbn/p1pp1p1p/8/1p2p1p1/4qPP1/2P5/PP1PP1BP/QRKNR1BN w EBeb - 0 9 ;D1 28 ;D2 1309 ;D3 36355 ;D4 1568968 ;D5 44576409 ;D6 1846382333",
        "qrk1rn1b/ppppp2p/4n3/3b1pp1/4P2P/5BP1/PPPP1P2/QRKNRNB1 w EBeb - 3 9 ;D1 26 ;D2 839 ;D3 22189 ;D4 726354 ;D5 19978260 ;D6 661207281",
        "bbrqk1rn/pp1ppppp/8/2p5/2P1P3/5n1P/PPBP1PP1/B1RQKNRN w GCgc - 1 9 ;D1 3 ;D2 95 ;D3 2690 ;D4 85038 ;D5 2518864 ;D6 80775549",
        "brqbk2n/pppppprp/8/6p1/1P3n2/5P2/P1PPP1PP/R1QBKNRN w Gb - 2 9 ;D1 22 ;D2 593 ;D3 13255 ;D4 362760 ;D5 8922397 ;D6 253271592",
        "brqknbr1/pp3ppp/3p2n1/2p1p3/2P5/5P2/PPKPP1PP/BRQ1NBRN w gb - 0 9 ;D1 21 ;D2 590 ;D3 13190 ;D4 397355 ;D5 9581695 ;D6 304103516",
        "1rqknrnb/2pp1ppp/p3p3/1p6/P2P4/5bP1/1PP1PP1P/BRQKNRNB w FBfb - 0 9 ;D1 24 ;D2 737 ;D3 20052 ;D4 598439 ;D5 17948681 ;D6 536330341",
        "rbb1k1rn/p1pqpppp/6n1/1p1p4/5P2/3PP3/PPP1K1PP/RBBQ1NRN w ga - 3 9 ;D1 24 ;D2 694 ;D3 16773 ;D4 513782 ;D5 13094823 ;D6 419402704",
        "rqbbknr1/1ppp2pp/p5n1/4pp2/P7/1PP5/1Q1PPPPP/R1BBKNRN w GAga - 0 9 ;D1 24 ;D2 600 ;D3 15347 ;D4 408207 ;D5 11029596 ;D6 308553169",
        "rqbknbrn/2pppppp/6Q1/pp6/8/2P5/PP1PPPPP/R1BKNBRN w GAga - 2 9 ;D1 40 ;D2 949 ;D3 34100 ;D4 889887 ;D5 31296485 ;D6 881529007",
        "rqbknr1b/pp1ppp2/2p2n1p/6p1/8/3P1PPP/PPP1P3/RQBKNRNB w FAfa - 0 9 ;D1 20 ;D2 560 ;D3 12275 ;D4 373921 ;D5 8687544 ;D6 277906201",
        "rbqkbnrn/p3pppp/1p6/3p4/P1p3P1/1P6/1QPPPP1P/RB1KBNRN w GAga - 0 9 ;D1 30 ;D2 1155 ;D3 35865 ;D4 1351455 ;D5 43092716 ;D6 1614019629",
        "rqkbb1rn/p1p1pppn/1p1p4/7p/4PP2/7P/PPPPB1P1/RQK1BNRN w GAga - 1 9 ;D1 30 ;D2 701 ;D3 20804 ;D4 515942 ;D5 15450970 ;D6 401499189",
        "rqknbbrn/1p2pp1p/3p2p1/p1p5/P2P4/1P6/1KP1PPPP/RQ1NBBRN w ga - 0 9 ;D1 28 ;D2 756 ;D3 21655 ;D4 610320 ;D5 17989811 ;D6 525585996",
        "rqknbrnb/1pp3pp/5p2/p2pp3/P7/3PPN2/1PP2PPP/RQKNBR1B w FAfa - 0 9 ;D1 26 ;D2 731 ;D3 19509 ;D4 550395 ;D5 15209404 ;D6 439767476",
        "rbqkr1bn/p1pppp1p/1p1n4/6p1/7P/3P1PP1/PPP1P3/RBQKNRBN w FAa - 0 9 ;D1 27 ;D2 586 ;D3 16282 ;D4 381604 ;D5 10905865 ;D6 274364342",
        "rqk1nrb1/ppbp1ppp/4p1n1/2p5/7P/1PP5/P2PPPP1/RQKBNRBN w FAfa - 1 9 ;D1 27 ;D2 749 ;D3 21480 ;D4 602318 ;D5 18084787 ;D6 520547029",
        "rqknrbbn/pp1p1ppp/4p3/2p5/3P2P1/7P/PPP1PP2/RQKNRBBN w EAa - 0 9 ;D1 20 ;D2 533 ;D3 11829 ;D4 336248 ;D5 8230417 ;D6 245871540",
        "rqknrnbb/pp1ppp1p/2p3p1/8/8/1P2P1NP/P1PP1PP1/RQKNR1BB w EAea - 0 9 ;D1 22 ;D2 633 ;D3 14480 ;D4 441877 ;D5 10827868 ;D6 343525739",
        "1brkq1rn/2pppppp/1p2n3/p2bN3/8/7P/PPPPPPP1/BBRKQ1RN w GCgc - 2 9 ;D1 27 ;D2 748 ;D3 20134 ;D4 580054 ;D5 16010135 ;D6 475206624",
        "brkbqnrn/2pp1ppp/8/1p2p3/Pp2N3/8/2PPPPPP/BRKBQNR1 w GBgb - 0 9 ;D1 30 ;D2 827 ;D3 25308 ;D4 757837 ;D5 23746165 ;D6 751690068",
        "brk1nbrn/pp1ppppp/2p5/7P/5P2/q2P4/PPP1P1P1/BRKQNBRN w GBgb - 1 9 ;D1 15 ;D2 471 ;D3 8716 ;D4 276424 ;D5 5960901 ;D6 190316951",
        "brkqnrnb/1p1pp1p1/p4p2/2p4p/8/P2PP3/1PP1QPPP/BRK1NRNB w FBfb - 0 9 ;D1 24 ;D2 479 ;D3 12584 ;D4 280081 ;D5 7830230 ;D6 190419716",
        "rbbkqnrn/2ppp2p/pp3p2/6p1/P6P/8/RPPPPPP1/1BBKQNRN w Gga - 0 9 ;D1 21 ;D2 523 ;D3 12125 ;D4 328733 ;D5 8322614 ;D6 242240658",
        "rkbbqr1n/1ppppppn/7p/p7/4P3/2P2P2/PP1PB1PP/RKB1QNRN w GAa - 3 9 ;D1 27 ;D2 563 ;D3 16026 ;D4 372148 ;D5 11105151 ;D6 283211800",
        "rkbqnbrn/ppppp3/8/5ppp/2P3P1/7P/PPQPPP2/RKB1NBRN w GAga - 0 9 ;D1 28 ;D2 639 ;D3 19250 ;D4 469250 ;D5 14872172 ;D6 384663405",
        "rkb1nrnb/pppp1pp1/5q1p/8/P3p3/4R1P1/1PPPPP1P/1KBQNRNB w Ffa - 0 9 ;D1 28 ;D2 873 ;D3 23690 ;D4 720814 ;D5 20209424 ;D6 625281937",
        "rbkqb1rn/1p1ppppp/4n3/p1p5/8/3PBP2/PPP1P1PP/RBKQ1NRN w GAga - 0 9 ;D1 26 ;D2 798 ;D3 21416 ;D4 667496 ;D5 18475618 ;D6 591681956",
        "rk1qbnrn/1p1ppppp/1b6/p1p5/P7/2P3NP/1P1PPPP1/RKQBB1RN w GAga - 0 9 ;D1 22 ;D2 506 ;D3 12313 ;D4 301029 ;D5 7891676 ;D6 205739580",
        "rk1nbbrn/ppp1ppp1/8/3p3p/1P1P2q1/5PB1/P1P1P1PP/RKQN1BRN w GAga - 1 9 ;D1 31 ;D2 956 ;D3 29219 ;D4 903799 ;D5 27827461 ;D6 876341492",
        "rkqnbr1b/pp1pppp1/7p/2p2n2/P2P4/7N/RPP1PPPP/1KQNBR1B w Ffa - 0 9 ;D1 31 ;D2 750 ;D3 24267 ;D4 646252 ;D5 21639104 ;D6 617064197",
        "rbkq1rbn/2p1pppp/pp3n2/3p4/5P2/3N2N1/PPPPP1PP/RBKQR1B1 w Afa - 2 9 ;D1 26 ;D2 647 ;D3 18027 ;D4 465119 ;D5 13643783 ;D6 369702807",
        "rkqbr1bn/p2ppppp/1pp2n2/8/5P2/3P1N2/PPP1PRPP/RKQB2BN w Aa - 3 9 ;D1 24 ;D2 574 ;D3 14593 ;D4 371597 ;D5 10066892 ;D6 271121237",
        "rk1qrbbn/p1ppp1pp/1p2n3/5p2/1P6/K3N3/P1PPPPPP/R1Q1RBBN w ea - 0 9 ;D1 25 ;D2 548 ;D3 14069 ;D4 340734 ;D5 9043111 ;D6 235545764",
        "rkqnrnbb/pp1pp3/2p5/5ppp/8/PP4NP/2PPPPP1/RKQNR1BB w EAea - 0 9 ;D1 23 ;D2 727 ;D3 18228 ;D4 566572 ;D5 15078056 ;D6 471296844",
        "bbrknq1r/ppppppp1/8/7p/5n2/3P4/PPP1PNPP/BBKRNQR1 w c - 0 9 ;D1 21 ;D2 610 ;D3 13300 ;D4 394705 ;D5 9605845 ;D6 293532398",
        "brkbnqr1/2pppnpp/pp3p2/8/4PPPP/8/PPPP4/BRKBNQRN w GBgb - 1 9 ;D1 30 ;D2 757 ;D3 23908 ;D4 621332 ;D5 20360394 ;D6 548380577",
        "brk1qb1n/ppppppr1/2n3pp/8/2P3P1/2N5/PP1PPP1P/BR1KQBRN w b - 1 9 ;D1 26 ;D2 570 ;D3 15537 ;D4 352883 ;D5 10081351 ;D6 242864559",
        "brknq1nb/pp2prpp/8/2pP1p2/6P1/2N5/PPPP1P1P/BRK1QRNB w FBb - 1 9 ;D1 33 ;D2 830 ;D3 27897 ;D4 764915 ;D5 26262884 ;D6 765831403",
        "rbbk1qrn/ppp1p1pp/5p2/3p1n2/7N/P7/1PPPPPPP/RBB1KQRN w ga - 0 9 ;D1 21 ;D2 562 ;D3 13060 ;D4 378883 ;D5 9520963 ;D6 290579255",
        "rk1b1qrn/ppp1pppp/5n2/3pN3/P6P/7b/1PPPPPP1/RKBB1QRN w GAga - 4 9 ;D1 28 ;D2 677 ;D3 19235 ;D4 488740 ;D5 14354779 ;D6 383207197",
        "rkbnqbrn/pp1ppp1p/2p5/6p1/P7/4P3/KPPPQPPP/R1BN1BRN w - - 3 9 ;D1 28 ;D2 585 ;D3 17443 ;D4 401483 ;D5 12574541 ;D6 310495538",
        "rk1nqrnb/pbpppp2/1p4p1/7p/P7/5NP1/1PPPPPBP/RKBNQR2 w FAfa - 2 9 ;D1 26 ;D2 774 ;D3 21626 ;D4 645200 ;D5 19093408 ;D6 576325868",
        "rbknb1rn/p1pp2pp/1p6/4pp2/1q3P1B/2N5/PPPPPNPP/RBK2QR1 w GAga - 2 9 ;D1 31 ;D2 1206 ;D3 36940 ;D4 1374158 ;D5 42849564 ;D6 1555711209",
        "rk1bbqrn/pp1pp1pp/3n4/5p2/3p4/1PP5/PK2PPPP/R1NBBQRN w ga - 0 9 ;D1 21 ;D2 629 ;D3 14059 ;D4 429667 ;D5 10587910 ;D6 332632033",
        "rknqbbr1/p1pp1pp1/1p4n1/4p2p/4P1P1/6RB/PPPP1P1P/RKNQB2N w Aga - 0 9 ;D1 27 ;D2 753 ;D3 20918 ;D4 593155 ;D5 17318772 ;D6 507563675",
        "rknqbr1b/pppp1ppp/4p2n/8/1P3P2/4P3/P1PPN1PP/RKNQBR1B w FAfa - 2 9 ;D1 26 ;D2 623 ;D3 17177 ;D4 460663 ;D5 13389799 ;D6 383508368",
        "r2kqrbn/bppppppp/2n5/p4B2/5P2/2P5/PP1PP1PP/1RKNQRBN w F - 2 9 ;D1 39 ;D2 1026 ;D3 37800 ;D4 1011922 ;D5 35946987 ;D6 992756232",
        "rk1bqrb1/ppppppp1/1n6/7p/2P2P1n/4P1Q1/PP1P2PP/RKNB1RBN w FAfa - 0 9 ;D1 35 ;D2 760 ;D3 25817 ;D4 610557 ;D5 21014787 ;D6 536852043",
        "rkq1rb1n/ppppp1pp/1n6/5p2/PPb2P2/8/1KPPP1PP/R1NQRBBN w ea - 1 9 ;D1 27 ;D2 754 ;D3 21009 ;D4 568788 ;D5 16461795 ;D6 448313956",
        "rknqr2b/pppnp1pp/3p4/3b1p2/8/1N1P2N1/PPP1PPPP/RKQ1R1BB w EAea - 1 9 ;D1 27 ;D2 803 ;D3 23708 ;D4 700453 ;D5 21875031 ;D6 654754840",
        "bbrknrqn/ppppp1pB/8/2P2p1p/8/5N2/PP1PPPPP/B1RK1RQN w FCfc - 0 9 ;D1 30 ;D2 799 ;D3 23923 ;D4 671112 ;D5 20532790 ;D6 603059376",
        "brkbnrq1/1pppp1p1/6np/p4p2/4P3/1PP5/P1KP1PPP/BR1BNRQN w fb - 1 9 ;D1 27 ;D2 726 ;D3 19329 ;D4 555622 ;D5 15156662 ;D6 457601127",
        "brknrbq1/1p1p1ppp/p3p1n1/2p5/8/1P1BPP2/P1PP2PP/BRKNR1QN w EBeb - 0 9 ;D1 36 ;D2 786 ;D3 27868 ;D4 655019 ;D5 22852433 ;D6 577223409",
        "brknrqnb/p2ppp1p/2p5/1p6/3P2p1/P1P1N3/1P2PPPP/BRK1RQNB w EBeb - 0 9 ;D1 23 ;D2 649 ;D3 15169 ;D4 440504 ;D5 10687843 ;D6 320881984",
        "rbbk1rqn/1ppppppp/3n4/p7/2P5/3N4/PP1PPPPP/RBB1KRQN w fa - 1 9 ;D1 20 ;D2 478 ;D3 11094 ;D4 275250 ;D5 7094988 ;D6 185488058",
        "rkbbnrqn/p2p1ppp/1p2p3/8/P1p1P3/1BP5/1P1P1PPP/RKB1NRQN w FAfa - 0 9 ;D1 22 ;D2 570 ;D3 13295 ;D4 346811 ;D5 8671852 ;D6 229898448",
        "rkb1rb1n/ppppppqp/8/2n3p1/2P1P1P1/8/PP1P1P1P/RKBNRBQN w EAea - 1 9 ;D1 23 ;D2 663 ;D3 16212 ;D4 490748 ;D5 12900485 ;D6 404944553",
        "rkb1rqnb/pppp3p/2n3p1/4pp2/P2P3P/2P5/1P2PPP1/RKBNRQNB w EAea - 0 9 ;D1 25 ;D2 845 ;D3 22188 ;D4 741972 ;D5 20276176 ;D6 683290790",
        "rbk1brqn/ppp1pppp/8/3p4/7P/1P4P1/2PPPP2/RBKNBRQN w FAfa - 0 9 ;D1 24 ;D2 526 ;D3 13862 ;D4 322175 ;D5 9054028 ;D6 222704171",
        "rknbbrqn/pp3pp1/4p3/2pp3p/2P5/8/PPBPPPPP/RKN1BRQN w FAfa - 0 9 ;D1 26 ;D2 756 ;D3 19280 ;D4 559186 ;D5 14697705 ;D6 433719427",
        "1knrbbqn/rp1p1ppp/p3p3/2p5/8/5P1P/PPPPP1P1/RKNRBBQN w DAd - 0 9 ;D1 26 ;D2 539 ;D3 15194 ;D4 345070 ;D5 10223443 ;D6 248715580",
        "rknr1qnb/ppp1p1pp/3p2b1/8/4p3/1P3P1P/P1PP2P1/RKNRBQNB w DAda - 0 9 ;D1 25 ;D2 701 ;D3 18969 ;D4 561369 ;D5 16047041 ;D6 496340789",
        "rbk1r1bn/ppppp1pp/4n3/5p2/1P3P2/4N2P/PqPPP1P1/RBK1RQBN w EAea - 1 9 ;D1 2 ;D2 60 ;D3 1319 ;D4 41765 ;D5 1017864 ;D6 33183408",
        "r1nbrqbn/k1ppp1pp/1p6/p4p2/2P5/6PQ/PP1PPP1P/RKNBR1BN w EA - 0 9 ;D1 27 ;D2 699 ;D3 20436 ;D4 561765 ;D5 17192121 ;D6 499247248",
        "rknrqbbn/1pp1pp2/p5p1/3p3p/6P1/PN5P/1PPPPP2/RK1RQBBN w DAda - 0 9 ;D1 23 ;D2 611 ;D3 15515 ;D4 435927 ;D5 11917036 ;D6 352885930",
        "rknrqn1b/p1pp1ppb/8/1p2p1Qp/3P4/3N4/PPP1PPPP/RK1R1NBB w DAda - 0 9 ;D1 45 ;D2 1170 ;D3 48283 ;D4 1320341 ;D5 52213677 ;D6 1500007485",
        "bbkrnrnq/p2p1ppp/2p1p3/1p6/1P2Q3/6P1/P1PPPP1P/BBKRNRN1 w - - 0 9 ;D1 41 ;D2 1035 ;D3 39895 ;D4 1035610 ;D5 38555608 ;D6 1037686769",
        "brkbnr2/1ppppp1p/7n/p5N1/P2q4/8/1PPPPPPP/BRKBNRQ1 w FBfb - 1 9 ;D1 22 ;D2 869 ;D3 19234 ;D4 679754 ;D5 16453359 ;D6 567287944",
        "brknrbnq/p1ppppp1/1p6/7p/2PP4/5P2/PPK1P1PP/BR1NRBNQ w eb - 1 9 ;D1 23 ;D2 641 ;D3 14748 ;D4 422240 ;D5 10192718 ;D6 302864305",
        "brk1r1qb/pp1ppnpp/2p2pn1/8/6N1/2N3P1/PPPPPP1P/BRK1R1QB w EBeb - 3 9 ;D1 32 ;D2 863 ;D3 28379 ;D4 773191 ;D5 25848794 ;D6 720443112",
        "rbbk1rnq/pppp1pp1/4p2p/8/3P2n1/4BN1P/PPP1PPP1/RB1K1RNQ w FAfa - 3 9 ;D1 26 ;D2 628 ;D3 16151 ;D4 411995 ;D5 11237919 ;D6 300314373",
        "rkbbnr1q/p1pppppp/5n2/1p5B/PP6/4P3/2PP1PPP/RKB1NRNQ w FAfa - 0 9 ;D1 30 ;D2 692 ;D3 21036 ;D4 519283 ;D5 16025428 ;D6 420887328",
        "rkb1rbnq/1pppp1pp/5p2/p7/5n1P/1PN3P1/P1PPPP2/RKB1RBNQ w EAea - 0 9 ;D1 32 ;D2 825 ;D3 27130 ;D4 697251 ;D5 23593363 ;D6 622249676",
        "rkbnrnqb/1ppp1p1p/p5p1/4p3/4P3/2N2P2/PPPP2PP/RKBR1NQB w Aea - 0 9 ;D1 24 ;D2 487 ;D3 13300 ;D4 301989 ;D5 8782713 ;D6 215787079",
        "rbknbr1q/pppp2pp/4p3/5p1n/1P2P2N/8/P1PP1PPP/RBKNBR1Q w FAfa - 0 9 ;D1 23 ;D2 571 ;D3 13799 ;D4 365272 ;D5 9224232 ;D6 257288920",
        "rknbb1nq/pppppr2/5pp1/7p/8/1N4P1/PPPPPP1P/RK1BBRNQ w FAa - 2 9 ;D1 26 ;D2 548 ;D3 15618 ;D4 350173 ;D5 10587626 ;D6 253006082",
        "rknr1bnq/p2pp1pp/1p3p2/2p4b/6PP/2P2N2/PP1PPP2/RKNRBB1Q w DAda - 1 9 ;D1 25 ;D2 502 ;D3 13150 ;D4 279098 ;D5 7824941 ;D6 175766730",
        "rknrb1qb/ppp1pppp/3p4/8/4P1nP/2P5/PPKP1PP1/R1NRBNQB w da - 1 9 ;D1 23 ;D2 643 ;D3 14849 ;D4 426616 ;D5 10507328 ;D6 312096061",
        "rbk1rnbq/pppp1npp/4p3/5p2/4P1P1/7P/PPPP1P1N/RBKNR1BQ w EAea - 1 9 ;D1 24 ;D2 591 ;D3 15178 ;D4 376988 ;D5 10251465 ;D6 263574861",
        "rknbrnb1/p1pppp1p/1p6/3N2p1/P3q1P1/8/1PPPPP1P/RKNBR1BQ w EAea - 1 9 ;D1 28 ;D2 948 ;D3 27343 ;D4 864588 ;D5 26241141 ;D6 812343987",
        "rknrn1b1/ppppppqp/8/6p1/2P5/2P1BP2/PP2P1PP/RKNRNB1Q w DAda - 1 9 ;D1 31 ;D2 807 ;D3 24360 ;D4 672973 ;D5 20455205 ;D6 588518645",
        "1k1rnqbb/npppppp1/r7/p2B3p/5P2/1N4P1/PPPPP2P/RK1RNQB1 w DAd - 0 9 ;D1 40 ;D2 1122 ;D3 44297 ;D4 1249989 ;D5 48711073 ;D6 1412437357",
        "bbqr1rkn/pp1ppppp/8/2p5/1P2P1n1/7N/P1PP1P1P/BBQRKR1N w FD - 0 9 ;D1 26 ;D2 841 ;D3 22986 ;D4 746711 ;D5 21328001 ;D6 705170410",
        "bqkr1rnn/1ppp1ppp/p4b2/4p3/P7/3PP2N/1PP2PPP/BQRBKR1N w FC - 3 9 ;D1 24 ;D2 500 ;D3 12802 ;D4 293824 ;D5 7928916 ;D6 197806842",
        "bqrkrbnn/1pp1ppp1/8/p6p/3p4/P3P2P/QPPP1PP1/B1RKRBNN w ECec - 0 9 ;D1 31 ;D2 592 ;D3 18585 ;D4 396423 ;D5 12607528 ;D6 298629240",
        "bqkrrnnb/2p1pppp/p7/1P1p4/8/2R3P1/PP1PPP1P/BQ1KRNNB w E - 0 9 ;D1 42 ;D2 1124 ;D3 45187 ;D4 1276664 ;D5 50052573 ;D6 1483524894",
        "qbbrkrn1/p1pppn1p/8/1p3Pp1/2P5/8/PP1PPP1P/QBBRKRNN w FDfd - 0 9 ;D1 21 ;D2 577 ;D3 13244 ;D4 392131 ;D5 9683808 ;D6 300294295",
        "qrbbkrnn/pp1p2pp/4p3/5p2/2p2P1P/2P5/PP1PP1P1/QRBBKRNN w FBfb - 0 9 ;D1 21 ;D2 571 ;D3 12736 ;D4 345681 ;D5 8239872 ;D6 228837930",
        "qrbkrbn1/1pp1pppp/p2p4/8/5PPn/2P5/PP1PP3/QRBKRBNN w EBeb - 0 9 ;D1 18 ;D2 466 ;D3 9443 ;D4 257776 ;D5 5679073 ;D6 162883949",
        "qrb1rnnb/pp1p1ppp/2pk4/4p3/1P2P3/1R6/P1PP1PPP/Q1BKRNNB w E - 4 9 ;D1 37 ;D2 760 ;D3 26863 ;D4 562201 ;D5 19486022 ;D6 421740856",
        "qbrkbrn1/p1pppp1p/6n1/1p4p1/1P6/5P2/P1PPPBPP/QBRK1RNN w FCfc - 1 9 ;D1 33 ;D2 824 ;D3 27385 ;D4 750924 ;D5 25176664 ;D6 734656217",
        "qrkbbr2/2pppppp/5nn1/pp1Q4/P7/3P4/1PP1PPPP/1RKBBRNN w FBfb - 0 9 ;D1 42 ;D2 1147 ;D3 44012 ;D4 1311247 ;D5 48216013 ;D6 1522548864",
        "qrkrbbnn/pp2pp2/2pp2pp/1B6/P7/4P3/1PPP1PPP/QRKRB1NN w DBdb - 0 9 ;D1 26 ;D2 464 ;D3 12653 ;D4 242892 ;D5 6928220 ;D6 142507795",
        "qrkrbnnb/p1pp1pp1/1p5p/4p3/1P6/6PN/PKPPPP1P/QR1RBN1B w db - 0 9 ;D1 29 ;D2 705 ;D3 20000 ;D4 529810 ;D5 15055365 ;D6 419552571",
        "qbrkr1bn/p1p1pp1p/1p1p2n1/6p1/3P1P2/4P3/PPP3PP/QBKRRNBN w ec - 2 9 ;D1 23 ;D2 613 ;D3 14835 ;D4 426484 ;D5 10747407 ;D6 323905533",
        "qrk1rnb1/p1pp1ppp/1p2Bbn1/8/4P3/6P1/PPPP1P1P/QRK1RNBN w EBeb - 1 9 ;D1 28 ;D2 927 ;D3 24887 ;D4 846839 ;D5 23063284 ;D6 807913585",
        "1qkrnbbn/1rpppppp/pp6/5N2/P4P2/8/1PPPP1PP/QRKRNBB1 w DBd - 3 9 ;D1 30 ;D2 542 ;D3 16646 ;D4 345172 ;D5 10976745 ;D6 251694423",
        "qrkr2bb/pppppppp/8/1n2n3/1N5P/1P6/P1PPPPP1/QRKR1NBB w DBdb - 1 9 ;D1 28 ;D2 719 ;D3 21048 ;D4 562015 ;D5 17351761 ;D6 479400272",
        "bbrqkrnn/3ppppp/8/ppp5/6P1/4P2N/PPPPKP1P/BBRQ1R1N w fc - 0 9 ;D1 21 ;D2 704 ;D3 16119 ;D4 546215 ;D5 13676371 ;D6 470796854",
        "brqbkrnn/1pp2p1p/3pp1p1/p5N1/8/1P6/P1PPPPPP/BRQBK1RN w Bfb - 0 9 ;D1 34 ;D2 688 ;D3 22827 ;D4 505618 ;D5 16639723 ;D6 402140795",
        "br1krb1n/2qppppp/pp3n2/8/1P4P1/8/P1PPPP1P/1RQKRBNN w EBeb - 0 9 ;D1 24 ;D2 945 ;D3 23943 ;D4 926427 ;D5 25019636 ;D6 959651619",
        "brqkr1nb/2ppp1pp/1p2np2/p7/2P1PN2/8/PP1P1PPP/BRQKRN1B w EBeb - 0 9 ;D1 28 ;D2 675 ;D3 19728 ;D4 504128 ;D5 15516491 ;D6 417396563",
        "rbbqkrnn/3pppp1/p7/1pp4p/2P1P2P/8/PP1P1PP1/RBBQKRNN w FAfa - 0 9 ;D1 26 ;D2 671 ;D3 18164 ;D4 496806 ;D5 14072641 ;D6 404960259",
        "rqbbkr1n/pp1p1p1p/4pn2/2p3p1/4P1P1/3P3P/PPP2P2/RQBBKRNN w FAfa - 0 9 ;D1 22 ;D2 633 ;D3 14629 ;D4 441809 ;D5 10776416 ;D6 335689685",
        "rqbkrbnn/p1ppp3/1p3pp1/7p/3P4/P1P5/1PQ1PPPP/R1BKRBNN w EAea - 0 9 ;D1 32 ;D2 607 ;D3 20339 ;D4 454319 ;D5 15586203 ;D6 383515709",
        "rqbkrnn1/pp2ppbp/3p4/2p3p1/2P5/1P3N1P/P2PPPP1/RQBKRN1B w EAea - 1 9 ;D1 29 ;D2 943 ;D3 28732 ;D4 908740 ;D5 28761841 ;D6 907579129",
        "rbqkb1nn/1ppppr1p/p5p1/5p2/1P6/2P4P/P1KPPPP1/RBQ1BRNN w a - 1 9 ;D1 22 ;D2 441 ;D3 10403 ;D4 231273 ;D5 5784206 ;D6 140934555",
        "rqkb1rnn/1pp1pp1p/p5p1/1b1p4/3P4/P5P1/RPP1PP1P/1QKBBRNN w Ffa - 1 9 ;D1 21 ;D2 505 ;D3 11592 ;D4 290897 ;D5 7147063 ;D6 188559137",
        "rq1rbbnn/pkp1ppp1/3p3p/1p2N1P1/8/8/PPPPPP1P/RQKRBB1N w DA - 0 9 ;D1 27 ;D2 608 ;D3 16419 ;D4 387751 ;D5 10808908 ;D6 268393274",
        "rqkrb2b/p2ppppp/2p3nn/1p6/5P2/PP1P4/2P1P1PP/RQKRBNNB w DAda - 1 9 ;D1 30 ;D2 749 ;D3 21563 ;D4 581531 ;D5 16916813 ;D6 485406712",
        "rbqkr1bn/pp1ppp2/2p1n2p/6p1/8/4BPNP/PPPPP1P1/RBQKRN2 w EAea - 0 9 ;D1 23 ;D2 600 ;D3 15082 ;D4 410057 ;D5 11041820 ;D6 314327867",
        "rqkbrnb1/2ppp1pp/pp3pn1/8/5P2/B2P4/PPP1P1PP/RQKBRN1N w EAea - 2 9 ;D1 22 ;D2 569 ;D3 13541 ;D4 371471 ;D5 9395816 ;D6 269460607",
        "rqkrnbb1/p1p1pppp/1p4n1/3p4/7P/P3P3/1PPPBPP1/RQKRN1BN w DAda - 0 9 ;D1 27 ;D2 579 ;D3 15565 ;D4 373079 ;D5 10238486 ;D6 266047417",
        "rqkrn1bb/p1ppp1pp/4n3/1p6/6p1/4N3/PPPPPPPP/RQKR2BB w DAda - 0 9 ;D1 20 ;D2 462 ;D3 10234 ;D4 274162 ;D5 6563859 ;D6 193376359",
        "bbrkqr2/pppp1ppp/6nn/8/2P1p3/3PP2N/PP3PPP/BBRKQR1N w FCfc - 0 9 ;D1 28 ;D2 724 ;D3 21688 ;D4 619064 ;D5 19318355 ;D6 593204629",
        "brk1qrnn/1pppbppp/4p3/8/1p6/P1P4P/3PPPP1/BRKBQRNN w FBfb - 1 9 ;D1 24 ;D2 662 ;D3 16920 ;D4 468215 ;D5 12610387 ;D6 355969349",
        "1r1qrbnn/p1pkpppp/1p1p4/8/3P1PP1/P4b2/1PP1P2P/BRKQRBNN w EB - 1 9 ;D1 22 ;D2 696 ;D3 17021 ;D4 510247 ;D5 13697382 ;D6 401903030",
        "1rkqrnnb/p1p1p1pp/1p1p4/3b1p1N/4P3/5N2/PPPP1PPP/BRKQR2B w EBeb - 1 9 ;D1 29 ;D2 887 ;D3 27035 ;D4 816176 ;D5 26051242 ;D6 791718847",
        "rbbkq1rn/pppppppp/7n/8/P7/3P3P/1PPKPPP1/RBB1QRNN w a - 3 9 ;D1 22 ;D2 417 ;D3 9900 ;D4 216855 ;D5 5505063 ;D6 134818483",
        "rkbbqr1n/1p1pppp1/2p2n2/p4NBp/8/3P4/PPP1PPPP/RK1BQRN1 w FAfa - 0 9 ;D1 37 ;D2 832 ;D3 30533 ;D4 728154 ;D5 26676373 ;D6 673756141",
        "rkbqrb1n/3pBppp/ppp2n2/8/8/P2P4/1PP1PPPP/RK1QRBNN w EAea - 0 9 ;D1 28 ;D2 685 ;D3 19718 ;D4 543069 ;D5 16033316 ;D6 482288814",
        "rkb1rn1b/ppppqppp/4p3/8/1P2n1P1/5Q2/P1PP1P1P/RKB1RNNB w EAea - 2 9 ;D1 37 ;D2 1158 ;D3 40114 ;D4 1234768 ;D5 44672979 ;D6 1389312729",
        "r1kqbrnn/pp1pp1p1/7p/2P2p2/5b2/3P4/P1P1P1PP/RBKQBRNN w FAfa - 0 9 ;D1 5 ;D2 161 ;D3 4745 ;D4 154885 ;D5 4734999 ;D6 157499039",
        "rkqbbr1n/ppp1ppp1/8/Q2p3p/4n3/3P1P2/PPP1P1PP/RK1BBRNN w FAfa - 2 9 ;D1 38 ;D2 1144 ;D3 40433 ;D4 1236877 ;D5 43832975 ;D6 1366087771",
        "rkqrbbn1/p1ppppp1/Bp5p/8/P6n/2P1P3/1P1P1PPP/RKQRB1NN w DAda - 0 9 ;D1 28 ;D2 551 ;D3 15488 ;D4 350861 ;D5 9944107 ;D6 251179183",
        "rkqrb1nb/1ppp1ppp/p7/4p3/5n2/3P2N1/PPPQPPPP/RK1RB1NB w DAda - 0 9 ;D1 26 ;D2 690 ;D3 19877 ;D4 513628 ;D5 15965907 ;D6 418191735",
        "rbkqrnbn/pppp1p2/4p1p1/7p/7P/P2P4/BPP1PPP1/R1KQRNBN w EAea - 0 9 ;D1 27 ;D2 515 ;D3 13992 ;D4 309727 ;D5 8792550 ;D6 218658292",
        "rkqbrnbn/pp1ppp2/8/2p3p1/P1P4p/5P2/1PKPP1PP/R1QBRNBN w ea - 0 9 ;D1 27 ;D2 627 ;D3 16843 ;D4 431101 ;D5 11978698 ;D6 328434174",
        "rkqrnbbn/1p2pp1p/3p2p1/p1p5/P5PP/3N4/1PPPPP2/RKQR1BBN w DAda - 0 9 ;D1 23 ;D2 624 ;D3 15512 ;D4 451860 ;D5 11960861 ;D6 367311176",
        "rk2rnbb/ppqppppp/2pn4/8/1P3P2/6P1/P1PPP1NP/RKQR1NBB w DAa - 1 9 ;D1 27 ;D2 727 ;D3 20206 ;D4 581003 ;D5 16633696 ;D6 505212747",
        "b1krrqnn/pp1ppp1p/2p3p1/8/P3Pb1P/1P6/2PP1PP1/BBRKRQNN w EC - 0 9 ;D1 32 ;D2 943 ;D3 30759 ;D4 865229 ;D5 28672582 ;D6 800922511",
        "1rkbrqnn/p1pp1ppp/1p6/8/P2Pp3/8/1PPKPPQP/BR1BR1NN w eb - 0 9 ;D1 28 ;D2 916 ;D3 24892 ;D4 817624 ;D5 22840279 ;D6 759318058",
        "brkrqb1n/1pppp1pp/p7/3n1p2/P5P1/3PP3/1PP2P1P/BRKRQBNN w DBdb - 0 9 ;D1 27 ;D2 669 ;D3 18682 ;D4 484259 ;D5 13956472 ;D6 380267099",
        "brkrqnnb/3pppp1/1p6/p1p4p/2P3P1/6N1/PP1PPP1P/BRKRQ1NB w DBdb - 0 9 ;D1 29 ;D2 699 ;D3 20042 ;D4 512639 ;D5 15093909 ;D6 406594531",
        "r1bkrq1n/pp2pppp/3b1n2/2pp2B1/6P1/3P1P2/PPP1P2P/RB1KRQNN w EAea - 2 9 ;D1 27 ;D2 835 ;D3 22848 ;D4 713550 ;D5 19867800 ;D6 631209313",
        "rk1brq1n/p1p1pppp/3p1n2/1p3b2/4P3/2NQ4/PPPP1PPP/RKBBR2N w EAea - 4 9 ;D1 36 ;D2 1004 ;D3 35774 ;D4 979608 ;D5 35143142 ;D6 966310885",
        "rkbrqbnn/1p2ppp1/B1p5/p2p3p/4P2P/8/PPPP1PP1/RKBRQ1NN w DAda - 0 9 ;D1 27 ;D2 748 ;D3 21005 ;D4 597819 ;D5 17597073 ;D6 515304215",
        "rkbrqn1b/pp1pp1pp/2p2p2/5n2/8/2P2P2/PP1PP1PP/RKBRQ1NB w DAda - 0 9 ;D1 20 ;D2 479 ;D3 10485 ;D4 266446 ;D5 6253775 ;D6 167767913",
        "rbkrbnn1/ppppp1pp/5q2/5p2/5P2/P3P2N/1PPP2PP/RBKRBQ1N w DAda - 3 9 ;D1 28 ;D2 947 ;D3 26900 ;D4 876068 ;D5 26007841 ;D6 838704143",
        "rkr1bqnn/1ppp1p1p/p5p1/4p3/3PP2b/2P2P2/PP4PP/RKRBBQNN w CAca - 0 9 ;D1 31 ;D2 1004 ;D3 32006 ;D4 1006830 ;D5 32688124 ;D6 1024529879",
        "rkrqbbnn/pppp3p/8/4ppp1/1PP4P/8/P2PPPP1/RKRQBBNN w CAca - 0 9 ;D1 24 ;D2 717 ;D3 18834 ;D4 564137 ;D5 15844525 ;D6 484884485",
        "rkrqbn1b/pppp2pp/8/4pp2/1P1P2n1/5N2/P1P1PP1P/RKRQBN1B w CAca - 0 9 ;D1 25 ;D2 718 ;D3 19654 ;D4 587666 ;D5 17257753 ;D6 537354146",
        "rbkrqnbn/p1p1ppp1/1p1p4/8/3PP2p/2PB4/PP3PPP/R1KRQNBN w DAda - 0 9 ;D1 30 ;D2 754 ;D3 23298 ;D4 611322 ;D5 19338246 ;D6 532603566",
        "1krbqnbn/1p2pppp/r1pp4/p7/8/1P1P2PP/P1P1PP2/RKRBQNBN w CAc - 0 9 ;D1 21 ;D2 566 ;D3 13519 ;D4 375128 ;D5 9700847 ;D6 279864836",
        "rkrq1b2/pppppppb/3n2np/2N5/4P3/7P/PPPP1PP1/RKRQ1BBN w CAca - 1 9 ;D1 33 ;D2 654 ;D3 21708 ;D4 479678 ;D5 15990307 ;D6 382218272",
        "rkr1nnbb/ppp2p1p/3p1qp1/4p3/P5P1/3PN3/1PP1PP1P/RKRQN1BB w CAca - 1 9 ;D1 28 ;D2 715 ;D3 20361 ;D4 555328 ;D5 16303092 ;D6 468666425",
        "bbrkrnqn/1p1ppppp/8/8/p2pP3/PP6/2P2PPP/BBRKRNQN w ECec - 0 9 ;D1 24 ;D2 757 ;D3 19067 ;D4 603231 ;D5 15957628 ;D6 509307623",
        "brkbrnqn/ppp2p2/4p3/P2p2pp/6P1/5P2/1PPPP2P/BRKBRNQN w EBeb - 0 9 ;D1 25 ;D2 548 ;D3 14563 ;D4 348259 ;D5 9688526 ;D6 247750144",
        "brkr1bqn/1pppppp1/3n3p/1p6/P7/4P1P1/1PPP1P1P/BRKRN1QN w DBdb - 0 9 ;D1 19 ;D2 359 ;D3 7430 ;D4 157099 ;D5 3521652 ;D6 81787718",
        "brkr1qnb/pppp2pp/2B1p3/5p2/2n5/6PP/PPPPPPN1/BRKR1QN1 w DBdb - 1 9 ;D1 27 ;D2 854 ;D3 23303 ;D4 741626 ;D5 20558538 ;D6 667089231",
        "rbbkrnqn/p1p1p1pp/8/1p1p4/1P1Pp3/6N1/P1P2PPP/RBBKRNQ1 w EAea - 0 9 ;D1 28 ;D2 723 ;D3 19844 ;D4 514440 ;D5 14621108 ;D6 397454100",
        "rkbbrn1n/pppppp2/5q1p/6p1/3P3P/4P3/PPP2PP1/RKBBRNQN w EAea - 1 9 ;D1 25 ;D2 741 ;D3 19224 ;D4 585198 ;D5 15605840 ;D6 485037906",
        "rkbr1bq1/ppnppppp/6n1/2p5/2P1N2P/8/PP1PPPP1/RKBRNBQ1 w DAda - 3 9 ;D1 24 ;D2 547 ;D3 14359 ;D4 339497 ;D5 9410221 ;D6 234041078",
        "1kbrnqnb/r1ppppp1/8/pp5p/8/1P1NP3/P1PP1PPP/RKB1RQNB w Ad - 2 9 ;D1 26 ;D2 618 ;D3 17305 ;D4 442643 ;D5 13112297 ;D6 357030697",
        "rbkrb1qn/1pp1ppp1/3pn2p/pP6/8/4N1P1/P1PPPP1P/RBKRB1QN w DAda - 0 9 ;D1 21 ;D2 544 ;D3 12492 ;D4 338832 ;D5 8381483 ;D6 236013157",
        "rkrbbnqn/ppppp3/5p2/6pp/5PBP/4P3/PPPP2P1/RKR1BNQN w CAca - 0 9 ;D1 30 ;D2 891 ;D3 25435 ;D4 764356 ;D5 21894752 ;D6 669256602",
        "rkr1bb1n/ppppp1pp/5p2/4n3/3QP3/5P2/RPPP2PP/1KRNBB1N w Cca - 1 9 ;D1 45 ;D2 1172 ;D3 51766 ;D4 1332060 ;D5 57856784 ;D6 1501852662",
        "rkr1bqnb/pp1ppppp/8/2pN4/1P6/5N2/P1PPnPPP/RKR1BQ1B w CAca - 0 9 ;D1 28 ;D2 730 ;D3 20511 ;D4 559167 ;D5 16323242 ;D6 463032124",
        "rbkrnqb1/2ppppp1/p5np/1p6/8/3N4/PPPPPPPP/RBKRQNB1 w DAda - 2 9 ;D1 20 ;D2 417 ;D3 9159 ;D4 217390 ;D5 5180716 ;D6 133936564",
        "rkrbnqb1/p1pppnpp/5p2/1p6/2P5/1P1P1N2/P3PPPP/RKRB1QBN w CAca - 0 9 ;D1 25 ;D2 546 ;D3 14039 ;D4 330316 ;D5 8813781 ;D6 222026485",
        "rkr1qbbn/ppppppp1/4n3/7p/8/P7/KPPPPPPP/R1RNQBBN w ca - 0 9 ;D1 22 ;D2 484 ;D3 11458 ;D4 267495 ;D5 6633319 ;D6 163291279",
        "rkrnqnb1/1ppppp2/p5p1/7p/8/P1bPP3/1PP1QPPP/RKRN1NBB w CAca - 0 9 ;D1 22 ;D2 636 ;D3 15526 ;D4 441001 ;D5 11614241 ;D6 331083405",
        "b2krn1q/p1rppppp/1Q3n2/2p1b3/1P4P1/8/P1PPPP1P/BBRKRNN1 w ECe - 3 9 ;D1 36 ;D2 1192 ;D3 42945 ;D4 1406795 ;D5 50382104 ;D6 1650202838",
        "brkbrnn1/pp1pppp1/7q/2p5/6Pp/4P1NP/PPPP1P2/BRKBR1NQ w EBeb - 2 9 ;D1 30 ;D2 978 ;D3 29593 ;D4 942398 ;D5 29205057 ;D6 936568065",
        "brkrnb1q/pp1p1ppp/2p1p3/5n2/1P6/5N1N/P1PPPPPP/BRKR1B1Q w DBdb - 1 9 ;D1 31 ;D2 897 ;D3 27830 ;D4 810187 ;D5 25423729 ;D6 755334868",
        "brkr1nqb/pp1p1pp1/2pn3p/P3p3/4P3/6P1/1PPP1P1P/BRKRNNQB w DBdb - 0 9 ;D1 19 ;D2 382 ;D3 8052 ;D4 182292 ;D5 4232274 ;D6 103537333",
        "r1bkrn1q/ppbppppp/5n2/2p5/3P4/P6N/1PP1PPPP/RBBKRNQ1 w EAea - 3 9 ;D1 27 ;D2 822 ;D3 22551 ;D4 678880 ;D5 19115128 ;D6 578210135",
        "rkbbrnnq/pp2pppp/8/2pp4/P1P5/1P3P2/3PP1PP/RKBBRNNQ w EAea - 1 9 ;D1 23 ;D2 643 ;D3 15410 ;D4 442070 ;D5 11170489 ;D6 329615708",
        "rkbr1b1q/p1pppppp/1p1n4/7n/5QP1/3N4/PPPPPP1P/RKBR1BN1 w DAda - 4 9 ;D1 37 ;D2 943 ;D3 34382 ;D4 880474 ;D5 31568111 ;D6 842265141",
        "rkbr1nqb/pppp2np/8/4ppp1/1P6/6N1/P1PPPPPP/RKBRN1QB w DAda - 1 9 ;D1 23 ;D2 574 ;D3 13260 ;D4 362306 ;D5 9020291 ;D6 261247606",
        "rbkr1nnq/p1p1pp1p/1p4p1/3p4/b3P3/4N3/PPPPNPPP/RBKRB1Q1 w DAda - 0 9 ;D1 26 ;D2 900 ;D3 23414 ;D4 805006 ;D5 21653203 ;D6 745802405",
        "rkrbb1nq/p2pppp1/1p4n1/2p4p/3N4/4P1P1/PPPP1P1P/RKRBBN1Q w CAca - 0 9 ;D1 32 ;D2 697 ;D3 22231 ;D4 531121 ;D5 17150175 ;D6 441578567",
        "rkrnbb1q/pp2pp1p/6pn/2pp4/2B1P2P/8/PPPP1PP1/RKRNB1NQ w CAca - 0 9 ;D1 28 ;D2 854 ;D3 23853 ;D4 755990 ;D5 21823412 ;D6 712787248",
        "rk2bnqb/pprpppp1/4n2p/2p5/P7/3P2NP/1PP1PPP1/RKRNB1QB w CAa - 1 9 ;D1 26 ;D2 596 ;D3 16251 ;D4 414862 ;D5 11758184 ;D6 323043654",
        "r1krnnbq/pp1ppp1p/6p1/2p5/2P5/P3P3/Rb1P1PPP/1BKRNNBQ w Dda - 0 9 ;D1 2 ;D2 61 ;D3 1312 ;D4 40072 ;D5 937188 ;D6 28753562",
        "1krbnnbq/1pp1p1pp/r7/p2p1p2/3PP3/2P3P1/PP3P1P/RKRBNNBQ w CAc - 0 9 ;D1 30 ;D2 953 ;D3 28033 ;D4 860530 ;D5 25531358 ;D6 787205262",
        "rkr1nbbq/2ppp1pp/1pn5/p4p2/P6P/3P4/1PP1PPPB/RKRNNB1Q w CAca - 1 9 ;D1 24 ;D2 645 ;D3 15689 ;D4 446423 ;D5 11484012 ;D6 341262639",
        "rkrnnqbb/p1ppp2p/Qp6/4Pp2/5p2/8/PPPP2PP/RKRNN1BB w CAca - 0 9 ;D1 35 ;D2 929 ;D3 32020 ;D4 896130 ;D5 31272517 ;D6 915268405",
        "bbq1nr1r/pppppk1p/2n2p2/6p1/P4P2/4P1P1/1PPP3P/BBQNNRKR w HF - 1 9 ;D1 23 ;D2 589 ;D3 14744 ;D4 387556 ;D5 10316716 ;D6 280056112",
    ];
}
