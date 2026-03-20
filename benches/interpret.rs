use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use trinculo::{
    Interpreter, Translator, baseline::Baseline, combo_par::ComboParallel, parse, read_prospero,
    reclaim::Reclaim, simd_par::SimdParallel, thread_par::ThreadParallel,
};

fn bench(c: &mut Criterion) {
    let prog = parse(&read_prospero().expect("reading")).expect("parsing");
    let mut group = c.benchmark_group("interpret");
    for i in [8u16, 16, 32, 64] {
        group.bench_with_input(BenchmarkId::new("baseline", i), &i, |b, &i| {
            b.iter(|| {
                let p = black_box(prog.clone());
                let _ = Baseline(i).interpret(p);
            });
        });
        let r = Reclaim(i);
        let p_with_gc = r.translate(prog.clone()).expect("translate");
        group.bench_with_input(BenchmarkId::new("reclaim", i), &i, |b, _| {
            b.iter(|| {
                let p = black_box(p_with_gc.clone());
                let _ = r.interpret(p);
            });
        });
        group.bench_with_input(BenchmarkId::new("thread_par", i), &i, |b, &i| {
            b.iter(|| {
                let p = black_box(prog.clone());
                let _ = ThreadParallel(i).interpret(p);
            });
        });
        group.bench_with_input(BenchmarkId::new("simd_par", i), &i, |b, &i| {
            b.iter(|| {
                let p = black_box(prog.clone());
                let _ = SimdParallel(i).interpret(p);
            });
        });
        group.bench_with_input(BenchmarkId::new("combo_par", i), &i, |b, &i| {
            b.iter(|| {
                let p = black_box(prog.clone());
                let _ = ComboParallel(i).interpret(p);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
