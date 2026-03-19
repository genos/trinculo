use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use trinculo::{Interpreter, thread_par::ThreadParallel, parse, read_prospero};

fn bench(c: &mut Criterion) {
    let prog = parse(&read_prospero().expect("reading")).expect("parsing");
    let mut group = c.benchmark_group("thread_par");
    for i in [8u32, 16, 32, 64] {
        group.bench_with_input(BenchmarkId::from_parameter(i), &i, |b, &i| {
            b.iter(|| {
                let p = black_box(prog.clone());
                let _ = ThreadParallel(i).interpret(p);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
