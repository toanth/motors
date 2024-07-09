use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};
use gears::games::chess::Chessboard;
use gears::search::{Depth, NodesLimit};
use motors::eval::chess::lite::LiTEval;
use motors::search::chess::caps::Caps;
use motors::search::{run_bench_with_depth_and_nodes, BenchLimit, Benchable, Engine};

pub fn caps_startpos_bench(c: &mut Criterion) {
    c.bench_function("bench 12 startpos", |b| {
        let pos = Chessboard::default();

        let mut engine = Caps::for_eval::<LiTEval>();
        b.iter(|| engine.bench(pos, BenchLimit::Depth(Depth::new(12))));
    });
}

pub fn caps_normal_bench_depth_7(c: &mut Criterion) {
    c.bench_function("normal bench", |b| {
        let mut engine = Caps::for_eval::<LiTEval>();
        b.iter(|| {
            run_bench_with_depth_and_nodes(
                &mut engine,
                Depth::new(7),
                NodesLimit::new(20_000).unwrap(),
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
