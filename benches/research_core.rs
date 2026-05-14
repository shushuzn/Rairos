use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rairos_llm::{citation_chain, impact};
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

fn bench_impact_scoring(c: &mut Criterion) {
    let papers: Vec<(String, String, u32, i32)> = (0..1000).map(|i| {
        (format!("p{:04}", i), format!("Paper Title {}", i), (i as u32) % 500, 2015 + (i % 10))
    }).collect();

    let mut group = c.benchmark_group("impact_score");
    group.bench_function("score_1000", |b| {
        b.iter(|| {
            for p in &papers {
                impact::score_paper(&p.0, &p.1, p.2, p.3, 2026);
            }
        });
    });
    group.bench_function("rank_1000", |b| {
        b.iter(|| impact::rank_papers(black_box(&papers), 2026, black_box(100)));
    });
    group.finish();
}

fn bench_citation_families(c: &mut Criterion) {
    let nodes: Vec<citation_chain::CitationNode> = (0..100).map(|i| {
        let mut citations = Vec::new();
        let mut references = Vec::new();
        for j in 1..=5 {
            let other = (i + j) % 100;
            citations.push(format!("p{:04}", other));
            references.push(format!("p{:04}", (i + 100 - j) % 100));
        }
        citation_chain::CitationNode {
            paper_id: format!("p{:04}", i),
            title: format!("Paper {}", i),
            year: Some(2020 + (i % 5) as i32),
            citations,
            references,
        }
    }).collect();
    let edges: Vec<(String, String, String)> = nodes.iter().flat_map(|n| {
        n.citations.iter().map(|c| (n.paper_id.clone(), c.clone(), "cites".to_string()))
            .chain(n.references.iter().map(|r| (r.clone(), n.paper_id.clone(), "cites".to_string())))
    }).collect();
    let chain = citation_chain::CitationChain {
        root_id: "p0000".to_string(),
        nodes: nodes.clone(),
        edges,
    };

    let empty_chain = citation_chain::CitationChain {
        root_id: "empty".to_string(),
        nodes: vec![],
        edges: vec![],
    };

    let mut group = c.benchmark_group("citation");
    group.bench_function("find_families_100", |b| {
        b.iter(|| citation_chain::find_families(black_box(&chain)));
    });
    group.bench_function("find_silent_100", |b| {
        b.iter(|| citation_chain::find_silent(black_box(&chain)));
    });
    group.bench_function("render_100", |b| {
        b.iter(|| citation_chain::render_text(black_box(&chain), black_box(50)));
    });
    group.bench_function("render_empty", |b| {
        b.iter(|| citation_chain::render_text(black_box(&empty_chain), black_box(50)));
    });
    group.finish();
}

criterion_group!(benches, bench_arxiv_parse, bench_gap_analysis,
    bench_trigger_match, bench_snapstate_save_load,
    bench_impact_scoring, bench_citation_families);
criterion_main!(benches);
