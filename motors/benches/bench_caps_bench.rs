use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};
use gears::games::chess::Board;
use gears::general::board::BoardTrait;
use gears::search::SearchLimit;
use motors::eval::chess::lite::LiTEval;
use motors::search::chess::caps::Caps;
use motors::search::{run_bench_with, Engine};

fn caps_normal_bench_depth_7(c: &mut Criterion) {
    c.bench_function("normal bench", |b| {
        let mut engine = Caps::for_eval::<LiTEval>();
        b.iter(|| {
            run_bench_with(
                &mut engine,
                SearchLimit::depth_(7),
                Some(SearchLimit::nodes_(20_000)),
                &Board::bench_positions().into_iter().collect::<Vec<_>>(),
                None,
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
