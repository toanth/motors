/// Tests adapted from [sanctaphraxx](<https://github.com/Ciekce/sanctaphraxx/blob/main/src/perft.rs>),
/// some of them are apparently originally from [libataxx](<https://github.com/kz04px/libataxx/blob/master/tests/perft.cpp>),
/// as well as from [zataxx](<https://github.com/zzzzz151/Zataxx/blob/main/src/tests.rs>)
#[cfg(test)]
mod tests {
    use crate::games::ataxx::AtaxxBoard;
    use crate::games::Board;
    use crate::general::board::Strictness::Strict;
    use crate::general::perft::perft;
    use crate::search::Depth;

    #[rustfmt::skip]
    const PERFT4_POSITIONS: &[(&str, &[u64])] = &[
        ("7/7/7/7/7/7/7 x 0 1", &[1, 0, 0, 0, 0]),
        ("7/7/7/7/7/7/7 o 0 1", &[1, 0, 0, 0, 0]),
        ("7/7/7/7/ooooooo/ooooooo/xxxxxxx o 0 1", &[1, 75, 249, 14270, 452_980]),
        ("7/7/7/7/xxxxxxx/xxxxxxx/ooooooo x 0 1", &[1, 75, 249, 14270, 452_980]),
        // ("x5o/7/7/7/7/7/o5x x 100 1", &[1, 0, 0, 0, 0]), // perft doesn't consider 50mr
        // ("x5o/7/7/7/7/7/o5x o 100 1", &[1, 0, 0, 0, 0]),
    ];

    #[rustfmt::skip]
    const PERFT5_POSITIONS: &[(&str,  &[u64])] = &[
        ("x5o/7/7/7/7/7/o5x x 0 1", &[1, 16, 256, 6460, 155_888, 4_752_668]),
        #[cfg(not(debug_assertions))]
        ("x5o/7/7/7/7/7/o5x o 0 1", &[1, 16, 256, 6460, 155_888, 4_752_668]),
        ("x5o/7/2-1-2/7/2-1-2/7/o5x x 0 1", &[1, 14, 196, 4184, 86528, 2_266_352]),
        #[cfg(not(debug_assertions))]
        ("x5o/7/2-1-2/7/2-1-2/7/o5x o 0 1", &[1, 14, 196, 4184, 86528, 2_266_352]),
        ("x5o/7/2-1-2/3-3/2-1-2/7/o5x x 0 1", &[1, 14, 196, 4100, 83104, 2_114_588]),
        #[cfg(not(debug_assertions))]
        ("x5o/7/2-1-2/3-3/2-1-2/7/o5x o 0 1", &[1, 14, 196, 4100, 83104, 2_114_588]),
        ("x5o/7/3-3/2-1-2/3-3/7/o5x x 0 1", &[1, 16, 256, 5948, 133_264, 3_639_856]),
        #[cfg(not(debug_assertions))]
        ("x5o/7/3-3/2-1-2/3-3/7/o5x o 0 1", &[1, 16, 256, 5948, 133_264, 3_639_856]),
        ("7/7/7/7/ooooooo/ooooooo/xxxxxxx x 0 1", &[1, 1, 75, 249, 14270, 452_980]),
        #[cfg(not(debug_assertions))]
        ("7/7/7/7/xxxxxxx/xxxxxxx/ooooooo o 0 1", &[1, 1, 75, 249, 14270, 452_980]),
        ("7/7/7/2x1o2/7/7/7 x 0 1", &[1, 23, 419, 7887, 168_317, 4_266_992]),
        #[cfg(not(debug_assertions))]
        ("7/7/7/2x1o2/7/7/7 o 0 1", &[1, 23, 419, 7_887, 168_317, 4_266_992]),
        ("7/7/7/7/xxxxxxx/xxxxxxx/ooooooo x 0 1", &[1, 75, 249, 14_270, 452_980]),
        #[cfg(not(debug_assertions))]
        ("7/7/7/7/ooooooo/ooooooo/xxxxxxx o 0 1", &[1, 75, 249, 14_270, 452_980]),
    ];

    #[rustfmt::skip]
    const PERFT6_POSITIONS: &[(&str, &[u64])] = &[
        ("7/7/7/7/-------/-------/x5o x 0 1", &[1, 2, 4, 13, 30, 73, 174]),
        #[cfg(not(debug_assertions))]
        ("7/7/7/7/-------/-------/x5o o 0 1", &[1, 2, 4, 13, 30, 73, 174]),
        ("o5x/7/2-1-2/7/2-1-2/7/x5o o 0 1", &[1, 14, 196, 4_184, 86_528, 2_266_352]),
        #[cfg(not(debug_assertions))]
        ("o5x/7/2-1-2/7/2-1-2/7/x5o x 0 1", &[1, 14, 196, 4_184, 86_528, 2_266_352]),
        ("o5x/7/2-1-2/3-3/2-1-2/7/x5o x 0 1", &[1, 14, 196, 4_100, 83_104, 2_114_588]),
        #[cfg(not(debug_assertions))]
        ("o5x/7/2-1-2/3-3/2-1-2/7/x5o o 0 1", &[1, 14, 196, 4_100, 83_104, 2_114_588]),
        ("o5x/7/3-3/2-1-2/3-3/7/x5o x 0 1", &[1, 16, 256, 5_948, 133_264, 3_639_856]),
        #[cfg(not(debug_assertions))]
        ("o5x/7/3-3/2-1-2/3-3/7/x5o o 0 1", &[1, 16, 256, 5_948, 133_264, 3_639_856]),
        ("o5x/7/7/7/7/7/x5o o 0 1", &[1, 16, 256, 6_460, 155_888, 4_752_668]),
        #[cfg(not(debug_assertions))]
        ("o5x/7/7/7/7/7/x5o x 0 1", &[1, 16, 256, 6_460, 155_888, 4_752_668]),
        ("7/7/7/2o1x2/7/7/7 o 0 1", &[1, 23, 419, 7_887, 168_317, 4_266_992]),
        #[cfg(not(debug_assertions))]
        ("7/7/7/2o1x2/7/7/7 x 0 1", &[1, 23, 419, 7_887, 168_317, 4_266_992]),
    ];
    #[rustfmt::skip]
    const PERFT7_POSITIONS: &[(&str, &[u64])] = &[
        ("7/7/7/7/-------/-------/o5x o 0 1", &[1, 2, 4, 13, 30, 73, 174]),
        ("7/7/7/7/-------/-------/o5x x 0 1", &[1, 2, 4, 13, 30, 73, 174]),
    ];

    fn test_perft(positions: &[(&str, &[u64])]) {
        for (fen, counts) in positions {
            let pos = AtaxxBoard::from_fen(fen, Strict).unwrap();
            for (depth, &count) in counts.iter().enumerate() {
                let res = perft(Depth::new_unchecked(depth), pos, true);
                assert_eq!(res.nodes, count);
            }
        }
    }

    #[test]
    fn perft4() {
        test_perft(PERFT4_POSITIONS);
    }

    #[test]
    fn perft5() {
        test_perft(PERFT5_POSITIONS);
    }

    #[test]
    fn perft6() {
        test_perft(PERFT6_POSITIONS);
    }

    #[test]
    fn perft7() {
        test_perft(PERFT7_POSITIONS);
    }
}
