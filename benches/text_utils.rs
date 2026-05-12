use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rairos_text_utils::{extract_keywords, cosine_sim, jaccard};

fn bench_text_utils(c: &mut Criterion) {
    let text = "This is a sample text for benchmarking word tokenization \
                and frequency analysis operations across multiple calls.";
    let keywords = extract_keywords(text, 3);
    let keywords2 = extract_keywords("Another text with different keywords for similarity testing.", 3);

    let mut group = c.benchmark_group("text_utils");
    group.bench_function("extract_keywords_50", |b| {
        b.iter(|| black_box(extract_keywords(black_box(text), black_box(3))));
    });
    group.bench_function("cosine_sim_100", |b| {
        let a: Vec<f64> = (0..100).map(|i| (i as f64).sin()).collect();
        let vec_b: Vec<f64> = (0..100).map(|i| (i as f64).cos()).collect();
        b.iter(|| black_box(cosine_sim(black_box(&a), black_box(&vec_b))));
    });
    group.bench_function("jaccard_sim", |b| {
        b.iter(|| black_box(jaccard(black_box(&keywords), black_box(&keywords2))));
    });
    group.finish();
}
criterion_group!(benches, bench_text_utils);
criterion_main!(benches);
