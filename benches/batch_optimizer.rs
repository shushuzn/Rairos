use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rairos_batch_optimizer::BatchOptimizer;

fn bench_batch_optimizer(c: &mut Criterion) {
    let items: Vec<i32> = (0..1000).collect();
    let mut group = c.benchmark_group("batch_optimizer");
    group.bench_function("process_batch_sequential_1k", |b| {
        b.iter(|| {
            let opt = BatchOptimizer::new(4);
            black_box(opt.process_sequential(
                black_box(&items),
                |x: &i32| Ok::<i32, std::io::Error>(*x * 2),
                None,
            ))
        });
    });
    group.bench_function("process_batch_parallel_1k", |b| {
        b.iter(|| {
            let opt = BatchOptimizer::new(4);
            black_box(opt.process_batch(
                black_box(&items),
                |x: &i32| Ok::<i32, std::io::Error>(*x * 2),
                None,
            ))
        });
    });
    group.finish();
}
criterion_group!(benches, bench_batch_optimizer);
criterion_main!(benches);
