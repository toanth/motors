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

#[cfg(test)]
mod tests {
    use crate::games::fairy::FairyBoard;
    use crate::general::board::BoardHelpers;
    use crate::general::board::Strictness::Relaxed;
    use crate::general::perft::perft;
    use crate::search::Depth;
    use crate::ugi::load_ugi_pos_simple;
    use std::time::Instant;

    fn test_pos(pos: FairyBoard, fen: &str, expected: &[u64], start_time: Instant, max_nodes: u64) {
        for (i, &expected) in expected.iter().enumerate() {
            if expected > max_nodes {
                break;
            }
            let i = i + 1;
            let depth = Depth::new(i);
            let res = perft(depth, pos.clone(), true);
            assert_eq!(res.depth, depth);
            assert_eq!(res.nodes, expected, "depth {i}, fen '{fen}' ({pos})");
        }
        println!("finished Position '{fen}' in {}ms", start_time.elapsed().as_millis());
    }

    #[test]
    fn debug_perft_tests() {
        perft_tests(1_000_000);
    }

    #[test]
    #[ignore]
    fn release_perft_tests() {
        perft_tests(u64::MAX)
    }

    fn perft_tests(max_nodes: u64) {
        let fens = &[
            ("chess startpos", vec![20, 400, 8902, 197_281]),
            ("chess kiwipete", vec![48, 2039, 97_862, 4_085_603]),
            ("chess f 8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", vec![14, 191, 2812, 43238, 674_624]),
            ("f chess r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1", vec![6, 264, 9467, 422_333]),
            ("fen chess rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", vec![44, 1486, 62_379, 2_103_487]),
            (
                "chess fen r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
                vec![46, 2079, 89_890, 3_894_594],
            ),
            (
                "chess fen 1rqbkrbn/1ppppp1p/1n6/p1N3p1/8/2P4P/PP1PPPP1/1RQBKRBN w FBfb - 0 9",
                vec![29, 502, 14_569, 287_739],
            ),
            (
                "f chess rbbqn1kr/pp2p1pp/6n1/2pp1p2/2P4P/P7/BP1PPPP1/R1BQNNKR w HAha - 0 9",
                vec![27, 916, 25_798, 890_435],
            ),
            ("chess rqbbknr1/1ppp2pp/p5n1/4pp2/P7/1PP5/1Q1PPPPP/R1BBKNRN w GAga - 0 9", vec![24, 600, 15347, 408_207]),
            ("shatranj", vec![16, 256, 4176, 68_122, 1_164_248]),
            (
                "fen shatranj rnaf1k1r/pp1Pappp/2p5/8/2A5/8/PPP1NnPP/RNAFK2R w 1 8",
                vec![23, 476, 10688, 220_593, 5_116_523],
            ),
            ("tictactoe", vec![9, 9 * 8, 9 * 8 * 7, 9 * 8 * 7 * 6, 9 * 8 * 7 * 6 * 5]),
            ("mnk 6 7 4", vec![42, 1722, 68_880, 2_686_320]),
            ("large_mnk", vec![121, 14_520, 1_727_880]),
            ("mnk 6 7 4 7/4O2/X2X1X1/O1XX1O1/OXOOOX1/OOXXOX1 x 11", vec![22, 462, 9240, 170_240, 3_050_784]),
            (
                "cfour 7 7 3 7/7/7/7/7/1X3O1/XOXXOO1 X",
                vec![7, 49, 259, 1372, 6804, 30_889, 145_533, 611_261, 2_702_712],
            ),
            ("cfour OO2OOX/XXX1XXO/OOXXOOX/XXOOXXX/OXOXXOO/OXOOXOX O", vec![2, 3, 2, 0, 0, 0, 0, 0, 0]),
            ("ataxx 7/7/7/7/7/7/7 x 0 1", vec![0, 0, 0, 0]),
            ("ataxx 7/7/7/7/ooooooo/ooooooo/xxxxxxx o 0 1", vec![75, 249, 14270, 452_980]),
            ("ataxx o5x/7/2-1-2/7/2-1-2/7/x5o o 0 1", vec![14, 196, 4_184, 86_528, 2_266_352]),
            (
                "ataxx 7/7/7/7/-------/-------/x5o o 0 1",
                vec![
                    2, 4, 13, 30, 73, 174, 342, 669, 1224, 2324, 3873, 6518, 10_552, 17_620, 26_855, 42_433, 64_058,
                    99_897, 146_120, 222_094, 322_833,
                ],
            ),
            ("atomic startpos", vec![20, 400, 8902, 197_326, 4_864_979]),
            (
                "atomic r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                vec![48, 1939, 88298, 3_492_097],
            ),
            ("atomic r7/8/8/8/8/8/3k1q2/R3K2R w KQ - 0 1", vec![22, 798, 16_158, 589_688, 13_318_284]),
            (
                "kingofthehill r6r/p1ppqpb1/bn2pnp1/2kPN3/1p2P3/2N1KQ1p/PPPBBPPP/R6R w - - 0 1",
                vec![47, 2061, 85499, 3_525_128],
            ),
            ("horde startpos", vec![8, 128, 1274, 23_310, 265_223, 5_396_554]),
            (
                "horde r3k2r/pq1bppQR/2RR3R/pPP2PP1/p1PPQQ1P/P1qP1PPp/PPpPQP1P/P1PPP1PP w kq - 0 1",
                vec![43, 1223, 52_478, 1_488_796],
            ),
            ("horde 4k2r/6P1/8/8/8/8/1q1p4/2P5 w - - 0 1", vec![12, 262, 2129, 56_153, 531_852, 16_410_824]),
            ("racingkings startpos", vec![21, 421, 11_264, 296_242, 9_472_927]),
            (
                "crazyhouse r~r2k1n~r/4p2P/1p3p2/8/8/4p3/P6P/R3K2R~[PRppn] b KQkq - 0 1",
                vec![110, 11_017, 914_431, 62_117_409],
            ),
            ("crazyhouse 7n/5p2/6p1/8/8/k7/7p/1K6[Q] w - - 0 1", vec![61, 403, 8793, 73_483, 1_728_921, 18_131_285]),
            (
                "antichess rnb1kbnr/ppp2ppp/4P3/3Q4/8/8/PPP1PqPP/RNB1KBNR w - - 0 1",
                vec![3, 17, 121, 611, 3501, 21338, 133_659, 1_059_417, 12_193_381],
            ),
        ];
        let old = FairyBoard::default();
        for (testcase, res) in fens {
            let start_time = Instant::now();
            let pos = load_ugi_pos_simple(testcase, Relaxed, &old).unwrap();
            test_pos(pos, testcase, res.as_slice(), start_time, max_nodes);
        }
    }

    // tests from shakmaty: <https://github.com/niklasf/shakmaty/blob/master/tests/>

    #[test]
    fn shakmaty_debug_tests() {
        shakmaty_tests(1_000_000);
    }

    #[test]
    #[ignore]
    fn shakmaty_release_tests() {
        shakmaty_tests(u64::MAX);
    }

    fn shakmaty_tests(max: u64) {
        let fens = &[
            ("antichess rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - -", vec![20, 400, 8067, 153299]),
            ("antichess 8/1p6/8/8/8/8/P7/8 w - -", vec![2, 4, 4, 3, 1, 0]),
            ("antichess 8/2p5/8/8/8/8/P7/8 w - -", vec![2, 4, 4, 4, 4, 4, 4, 4, 12, 36, 312, 2557, 30873]),
            ("atomic rn2kb1r/1pp1p2p/p2q1pp1/3P4/2P3b1/4PN2/PP3PPP/R2QKB1R b KQkq -", vec![40, 1238, 45237, 1434825]),
            ("atomic rn1qkb1r/p5pp/2p5/3p4/N3P3/5P2/PPP4P/R1BQK3 w Qkq -", vec![28, 833, 23353, 714499]),
            ("atomic 8/8/8/8/8/8/2k5/rR4KR w KQ -", vec![18, 180, 4364, 61401, 1603055]),
            ("atomic r3k1rR/5K2/8/8/8/8/8/8 b kq -", vec![25, 282, 6753, 98729, 2587730]),
            ("atomic Rr2k1rR/3K4/3p4/8/8/8/7P/8 w kq -", vec![21, 465, 10631, 241478, 5800275]),
            ("atomic rn2kb1r/1pp1p2p/p2q1pp1/3P4/2P3b1/4PN2/PP3PPP/R2QKB1R b KQkq -", vec![40, 1238, 45237, 1434825]),
            ("crazyhouse 2k5/8/8/8/8/8/8/4K3[QRBNPqrbnp] w - -", vec![301, 75353]),
            ("crazyhouse 2k5/8/8/8/8/8/8/4K3[Qn] w - -", vec![67, 3083, 88634, 932554]),
            (
                "crazyhouse r1bqk2r/pppp1ppp/2n1p3/4P3/1b1Pn3/2NB1N2/PPP2PPP/R1BQK2R[] b KQkq -",
                vec![42, 1347, 58057, 2083382],
            ),
            ("crazyhouse 4k3/1Q~6/8/8/4b3/8/Kpp5/8/ b - - 0 1", vec![20, 360, 5445, 132758]),
            ("horde 4k3/pp4q1/3P2p1/8/P3PP2/PPP2r2/PPP5/PPPP4 b - -", vec![30, 241, 6633, 56539]),
            ("horde k7/5p2/4p2P/3p2P1/2p2P2/1p2P2P/p2P2P1/2P2P2 w - -", vec![13, 172, 2205, 33781]),
            ("racingkings 4brn1/2K2k2/8/8/8/8/8/8 w - -", vec![6, 33, 178, 3151, 12981, 265932]),
            ("3check r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 1+1", vec![48, 2039, 97848]),
            ("3check r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 1+1", vec![26, 562, 13410]),
        ];
        for (fen, expected) in fens {
            let start_time = Instant::now();
            let pos = FairyBoard::from_fen(fen, Relaxed).unwrap();
            test_pos(pos, fen, expected.as_slice(), start_time, max);
        }
    }
}
