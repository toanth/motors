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

use gears::games::chess::Chessboard;
use gears::general::board::Board;
use libfuzzer_sys::fuzz_target;
use std::str::from_utf8;

fuzz_target!(|data: &[u8]| {
    if let Ok(str) = from_utf8(data) {
        let _ = Chessboard::from_fen(str);
    }
});
