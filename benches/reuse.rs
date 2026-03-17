use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use trinculo::{Translator, parse, read_prospero, reuse::Reuse};

fn bench(c: &mut Criterion) {
    let prog = parse(&read_prospero().expect("reading")).expect("parsing");
    c.bench_function("reuse", |b| {
        b.iter(|| {
            let p = black_box(prog.clone());
            let _ = Reuse.translate(p);
        });
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
