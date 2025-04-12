#![no_main]

use gears::games::chess::moves::ChessMove;
use gears::games::chess::Chessboard;
use gears::general::board::{Board, BoardHelpers};
use gears::general::moves::Move;
use libfuzzer_sys::fuzz_target;
use std::str::from_utf8;

fuzz_target!(|data: &[u8]| {
    if let Ok(str) = from_utf8(data) {
        for pos in
            Chessboard::bench_positions().into_iter().chain(Chessboard::name_to_pos_map().iter().map(|x| x.create()))
        {
            let Ok(mov) = ChessMove::from_text(str, &pos) else {
                return;
            };
            if pos.is_move_pseudolegal(mov) {
                let _ = pos.make_move(mov);
                assert!(pos.pseudolegal_moves().contains(&mov));
            }
        }
    }
});
