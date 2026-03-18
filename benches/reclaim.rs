use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use trinculo::{Interpreter, Translator, parse, read_prospero, reclaim::Reclaim};

fn translate(c: &mut Criterion) {
    let prog = parse(&read_prospero().expect("reading")).expect("parsing");
    c.bench_function("reclaim-translate", |b| {
        b.iter(|| {
            let p = black_box(prog.clone());
            let _ = Reclaim(16).translate(p);
        });
    });
}

fn interpret(c: &mut Criterion) {
    let prog = parse(&read_prospero().expect("reading")).expect("parsing");
    let mut group = c.benchmark_group("reclaim-interpret");
    for i in [8u32, 16, 32, 64] {
        let r = Reclaim(i);
        let p_with_gc = r.translate(prog.clone()).expect("translate");
        group.bench_with_input(BenchmarkId::from_parameter(i), &i, |b, _| {
            b.iter(|| {
                let p = black_box(p_with_gc.clone());
                let _ = r.interpret(p);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, translate, interpret);
criterion_main!(benches);
