use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};
use gears::games::chess::Chessboard;
use gears::general::board::Board;
use gears::search::SearchLimit;
use motors::eval::chess::lite::LiTEval;
use motors::search::chess::caps::Caps;
use motors::search::{run_bench_with, Engine};

pub fn caps_startpos_bench(c: &mut Criterion) {
    c.bench_function("bench 12 startpos", |b| {
        let pos = Chessboard::default();

        let mut engine = Caps::for_eval::<LiTEval>();
        b.iter(|| engine.clean_bench(pos, SearchLimit::depth_(12)));
    });
}

fn caps_normal_bench_depth_7(c: &mut Criterion) {
    c.bench_function("normal bench", |b| {
        let mut engine = Caps::for_eval::<LiTEval>();
        b.iter(|| {
            run_bench_with(
                &mut engine,
                SearchLimit::depth_(7),
                Some(SearchLimit::nodes_(20_000)),
                &Chessboard::bench_positions(),
            )
        });
    });
}

criterion_group! {
    name = caps_bench;
    config = Criterion::default().measurement_time(Duration::from_secs(40)).noise_threshold(0.03);
    targets =
    // caps_startpos_bench,
    caps_normal_bench_depth_7,
}

criterion_main!(caps_bench);
