/*
 *  Motors, a collection of games and engines.
 *  Copyright (C) 2024 ToTheAnd
 *
 *  Motors is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Motors is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Motors. If not, see <https://www.gnu.org/licenses/>.
 */

fn main() {
    // disable backtraces in anyhow, which greatly speeds up tests that construct a lot of Errors/
    // panics still print backtraces
    println!("cargo:rustc-env=RUST_BACKTRACE=1");
    println!("cargo:rustc-env=RUST_LIB_BACKTRACE=0");
}
