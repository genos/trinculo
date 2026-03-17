use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use trinculo::{parse, read_prospero};

fn bench(c: &mut Criterion) {
    let input = black_box(read_prospero().expect("reading"));
    c.bench_function("parse", |b| b.iter(|| parse(&input)));
}

criterion_group!(benches, bench);
criterion_main!(benches);
