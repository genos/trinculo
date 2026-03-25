use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use trinculo::{Translator, parse, read_prospero, reclaim::Reclaim, reuse::Reuse, unused::Unused};

fn bench(c: &mut Criterion) {
    let prog = parse(&read_prospero().expect("reading")).expect("parsing");
    let mut group = c.benchmark_group("translate");
    group.bench_function("reuse", |b| {
        b.iter(|| {
            let p = black_box(prog.clone());
            let _ = Reuse.translate(p);
        });
    });
    group.bench_function("reclaim", |b| {
        b.iter(|| {
            let p = black_box(prog.clone());
            let _ = Reclaim(16).translate(p);
        });
    });
    group.bench_function("unused", |b| {
        b.iter(|| {
            let p = black_box(prog.clone());
            let _ = Unused.translate(p);
        });
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
