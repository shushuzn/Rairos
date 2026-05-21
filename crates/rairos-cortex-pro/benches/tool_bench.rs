//! Tool performance benchmarks

use std::collections::HashMap;
use rairos_cortex_pro::tools::{MaterialTool, ToolParams, ToolOutput};
use rairos_tools::mattergen::MatterGenGenerator;

fn run_async<F: std::future::Future>(f: F) -> F::Output {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(f)
}

fn bench_mattergen_init(c: &mut criterion::Criterion) {
    c.bench_function("tool_mattergen_initialization", |b| {
        b.iter(|| {
            MatterGenGenerator::new("http://localhost:8000")
        });
    });
}

fn bench_mattergen_generate(c: &mut criterion::Criterion) {
    let tool = MatterGenGenerator::new("http://localhost:8000");
    c.bench_function("tool_mattergen_unconditioned_10", |b| {
        b.iter(|| {
            run_async(async {
                tool.generate_unconditioned(10).await.unwrap()
            })
        });
    });
}

fn bench_tool_param_parsing(c: &mut criterion::Criterion) {
    let mut inputs = HashMap::new();
    inputs.insert("num_generations".to_string(), serde_json::json!(100));
    inputs.insert("temperature".to_string(), serde_json::json!(1.5));
    inputs.insert("custom_field".to_string(), serde_json::json!("value"));

    c.bench_function("tool_param_parsing", |b| {
        b.iter(|| {
            ToolParams::new("test_tool".to_string(), inputs.clone())
        });
    });
}

fn bench_tool_output_success(c: &mut criterion::Criterion) {
    let mut result = HashMap::new();
    result.insert("structures".to_string(), serde_json::json!(["structure1", "structure2"]));
    result.insert("energies".to_string(), serde_json::json!([-5.0, -4.5]));

    c.bench_function("tool_output_success", |b| {
        b.iter(|| {
            ToolOutput::success(serde_json::to_value(result.clone()).unwrap())
        });
    });
}

fn bench_tool_output_failure(c: &mut criterion::Criterion) {
    c.bench_function("tool_output_failure", |b| {
        b.iter(|| {
            ToolOutput::failure("Test error message".to_string())
        });
    });
}

criterion::criterion_group!(
    benches,
    bench_mattergen_init,
    bench_mattergen_generate,
    bench_tool_param_parsing,
    bench_tool_output_success,
    bench_tool_output_failure
);
criterion::criterion_main!(benches);
