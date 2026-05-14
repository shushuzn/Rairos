use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rairos_research::gap_analysis;
use rairos_research::snapstate::{SnapSession, Snapstate};
use rairos_research::PaperSnapshot;
use std::collections::HashMap;

fn bench_arxiv_parse(c: &mut Criterion) {
    // Build a realistic arXiv XML response with 50 entries
    let mut xml = String::from("<?xml version=\"1.0\"?><feed>");
    for i in 0..50 {
        xml.push_str(&format!(
            "<entry><id>http://arxiv.org/abs/2401.{:05}</id>\
             <published>2024-01-{:02}</published>\
             <title>Test Paper Title About Deep Learning and Transformers {}</title>\
             <summary>This is a test abstract describing a novel approach to deep learning \
             using transformer architectures with attention mechanisms. The method shows \
             significant improvement over existing baselines.</summary>\
             <author><name>John Doe</name></author>\
             <author><name>Jane Smith</name></author>\
             <category term=\"cs.LG\"/><category term=\"cs.AI\"/></entry>",
            i, (i % 28) + 1, i
        ));
    }
    xml.push_str("</feed>");

    let mut group = c.benchmark_group("arxiv_xml");
    group.bench_function("parse_50_entries", |b| {
        b.iter(|| rairos_mcp::handlers::parse_arxiv_response(black_box(&xml)));
    });
    group.finish();
}

fn bench_gap_analysis(c: &mut Criterion) {
    // Create test papers
    let papers: Vec<PaperSnapshot> = (0..100).map(|i| PaperSnapshot {
        paper_id: format!("2401.{:05}", i),
        arxiv_id: Some(format!("2401.{:05}", i)),
        title: format!("Paper {}", i),
        abstract_text: "This method has a major limitation: it does not scale well. \
                        However, the approach shows promise. Future work should explore \
                        better evaluation benchmarks. The theoretical foundation is weak \
                        and generalization remains unproven.".to_string(),
        published: String::new(),
        citations: Vec::new(),
        extracted_text: Some("limitation however future work benchmark theoretical \
                             generalization not scalable poor performance".to_string()),
    }).collect();

    let mut group = c.benchmark_group("gap_analysis");
    group.bench_function("analyze_100_papers", |b| {
        b.iter(|| gap_analysis::analyze_gaps(black_box(&papers), "deep learning"));
    });
    group.finish();
}

fn bench_trigger_match(c: &mut Criterion) {
    let gp = rairos_research::gene_pool::GenePool::new();
    let mut group = c.benchmark_group("gene_pool");
    group.bench_function("find_capsule", |b| {
        b.iter(|| {
            gp.find_capsule(black_box("transformer attention mechanism"), black_box("improvement"), None, 0.0)
        });
    });
    group.finish();
}

fn bench_snapstate_save_load(c: &mut Criterion) {
    use std::collections::HashMap;

    let dir = std::env::temp_dir().join("rairos_bench_snap");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let session = SnapSession {
        session_id: "bench001".to_string(),
        query: "deep learning transformer attention".to_string(),
        created_at: 1700000000.0,
        updated_at: 1700000100.0,
        iteration: 5,
        max_iterations: 10,
        papers: (0..20).map(|i| rairos_research::snapstate::SnapPaper {
            arxiv_id: format!("2401.{:05}", i),
            title: format!("Paper {}", i),
            abstract_text: "Abstract text for benchmarking purposes.".to_string(),
            url: format!("https://arxiv.org/abs/2401.{:05}", i),
            extracted_text: "Extracted text content for benchmarking the snapstate system."
                .repeat(100),
            summary: String::new(),
            gaps_found: vec!["method_limitation".to_string(), "dataset_gap".to_string()],
            notes: String::new(),
            keywords: vec!["deep".to_string(), "learning".to_string()],
        }).collect(),
        gaps: (0..10).map(|i| rairos_research::snapstate::SnapGap {
            gap_type: if i % 2 == 0 { "method_limitation".to_string() } else { "dataset_gap".to_string() },
            title: format!("Gap {}", i),
            description: format!("Description for gap {}", i),
            matched_papers: vec!["2401.00001".to_string()],
            archetype_match: 0.5 + (i as f64 * 0.05),
            accepted: i % 3 == 0,
        }).collect(),
        search_history: (0..5).map(|i| format!("query {}", i)).collect(),
        hypotheses: Vec::new(),
        findings: vec!["Found important gap".to_string()],
        reflections: Vec::new(),
        archetype: HashMap::new(),
        status: "running".to_string(),
        error: String::new(),
    };

    let store = Snapstate::new(Some(dir.clone()));

    let mut group = c.benchmark_group("snapstate");
    group.bench_function("save_20papers_10gaps", |b| {
        b.iter(|| {
            let _ = store.save(black_box(&session));
        });
    });
    group.bench_function("load_20papers_10gaps", |b| {
        b.iter(|| {
            let _ = store.load(black_box("bench001"));
        });
    });
    group.finish();

    let _ = std::fs::remove_dir_all(&dir);
}

criterion_group!(benches, bench_arxiv_parse, bench_gap_analysis, bench_trigger_match, bench_snapstate_save_load);
criterion_main!(benches);
