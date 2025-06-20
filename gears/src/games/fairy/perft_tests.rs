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
    use crate::general::board::Strictness::Relaxed;
    use crate::general::perft::perft;
    use crate::search::Depth;
    use crate::ugi::load_ugi_pos_simple;
    use std::time::Instant;

    #[test]
    fn perft_tests() {
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
        ];
        let old = FairyBoard::default();
        for (testcase, res) in fens {
            let start_time = Instant::now();
            let pos = load_ugi_pos_simple(testcase, Relaxed, &old).unwrap();
            let max = if cfg!(debug_assertions) { 3 } else { 100 };
            for (i, &expected) in res.iter().take(max).enumerate() {
                let i = i + 1;
                let depth = Depth::new(i);
                let res = perft(depth, pos.clone(), true);
                assert_eq!(res.depth, depth);
                assert_eq!(res.nodes, expected, "depth {i}, fen '{testcase}' ({pos})");
            }
            println!("finished Position '{testcase}' in {}ms", start_time.elapsed().as_millis());
        }
    }
}
