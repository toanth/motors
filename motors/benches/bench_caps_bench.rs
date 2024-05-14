use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gears::games::chess::Chessboard;
use gears::search::Depth;
use motors::eval::chess::hce::HandCraftedEval;
use motors::search::chess::caps::Caps;
use motors::search::{run_bench, run_bench_with_depth, Benchable};

pub fn caps_startpos_bench(c: &mut Criterion) {
    c.bench_function("bench 12 startpos", |b| {
        let pos = Chessboard::default();
        let mut engine = Caps::<HandCraftedEval>::default();
        b.iter(|| engine.bench(pos, Depth::new(12)));
    });
}

pub fn caps_normal_bench_depth_7(c: &mut Criterion) {
    c.bench_function("normal bench", |b| {
        let mut engine = Caps::<HandCraftedEval>::default();
        b.iter(|| run_bench_with_depth(&mut engine, Depth::new(7)));
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
