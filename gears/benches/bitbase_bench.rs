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
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use gears::games::chess::bitbase::calc_pawn_vs_king;
use std::time::Duration;

pub fn calc_pawn_vs_king_bench(c: &mut Criterion) {
    c.bench_function("calc bitbase", |b| {
        b.iter(|| black_box(calc_pawn_vs_king()));
    });
}

criterion_group!(bitbase_benches, calc_pawn_vs_king_bench);
// criterion_group! {
//     name = bitbase_benches;
//     config = Criterion::default().measurement_time(Duration::from_secs(10)).noise_threshold(0.03);
//     targets = calc_pawn_vs_king_bench,
// }

criterion_main!(bitbase_benches);
