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
#![no_main]

use gears::games::chess::Board;
use gears::games::chess::moves::Move;
use gears::general::board::Strictness::Relaxed;
use gears::general::board::{BoardHelpers, BoardTrait};
use gears::general::moves::MoveTrait;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    let mut lines = data.lines();
    let Ok(mut pos) = Board::from_fen(lines.next().unwrap_or_default(), Relaxed) else {
        return;
    };
    for line in lines {
        if let Ok(mov) = Move::from_text(line, &pos) {
            pos = pos.play(mov);
        }
    }
    _ = pos.debug_verify_invariants(Relaxed).unwrap();
});
