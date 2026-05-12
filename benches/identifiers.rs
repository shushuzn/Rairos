use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rairos_identifiers::{classify, is_probably_doi, normalize_arxiv_id};

fn bench_identifiers(c: &mut Criterion) {
    let mut group = c.benchmark_group("identifiers");
    group.bench_function("classify_arxiv", |b| {
        b.iter(|| black_box(classify(black_box("2401.12345"))));
    });
    group.bench_function("classify_doi", |b| {
        b.iter(|| black_box(classify(black_box("10.1234/5678"))));
    });
    group.bench_function("normalize_arxiv", |b| {
        b.iter(|| black_box(normalize_arxiv_id(black_box("2401.12345v1"))));
    });
    group.bench_function("is_probably_doi", |b| {
        b.iter(|| black_box(is_probably_doi(black_box("10.1234/5678"))));
    });
    group.bench_function("classify_url_arxiv", |b| {
        b.iter(|| black_box(classify(black_box("https://arxiv.org/abs/2401.12345"))));
    });
    group.bench_function("classify_url_doi", |b| {
        b.iter(|| black_box(classify(black_box("https://doi.org/10.1234/5678"))));
    });
    group.finish();
}
criterion_group!(benches, bench_identifiers);
criterion_main!(benches);
