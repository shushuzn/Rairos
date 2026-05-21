//! Crew workflow performance benchmarks

use std::sync::Arc;
use async_trait::async_trait;
use rairos_llm::{LlmClient, LlmResponse, LlmError, Message, NonStreamResponse, LlmUsage};
use rairos_cortex_pro::sparks_crew::SparksCrew;
use rairos_cortex_pro::sparks_agents::{
    HypothesisAgent, HypothesisCriticAgent, PlannerAgent,
    PlanCriticAgent, ExecutorAgent, ReportWriterAgent,
};

#[derive(Clone)]
struct BenchmarkLlm;

#[async_trait]
impl LlmClient for BenchmarkLlm {
    async fn complete(
        &self,
        _messages: Vec<Message>,
        _model: &str,
        _temperature: f32,
        _max_tokens: u32,
    ) -> Result<LlmResponse, LlmError> {
        Ok(LlmResponse::NonStream(NonStreamResponse {
            content: "Benchmark response".to_string(),
            usage: LlmUsage::default(),
            model: "mock".to_string(),
            finish_reason: "stop".to_string(),
        }))
    }

    async fn stream_complete(
        &self,
        _messages: Vec<Message>,
        _model: &str,
        _temperature: f32,
        _max_tokens: u32,
    ) -> Result<LlmResponse, LlmError> {
        self.complete(_messages, _model, _temperature, _max_tokens).await
    }

    fn provider_name(&self) -> &'static str {
        "benchmark"
    }
}

fn create_benchmark_crew() -> SparksCrew {
    let mock_llm = Arc::new(BenchmarkLlm);
    SparksCrew::new(mock_llm)
        .add_agent(Box::new(HypothesisAgent::new("Bench")))
        .add_agent(Box::new(HypothesisCriticAgent::new("Bench")))
        .add_agent(Box::new(PlannerAgent::new("Bench")))
        .add_agent(Box::new(PlanCriticAgent::new("Bench")))
        .add_agent(Box::new(ExecutorAgent::new("Bench")))
        .add_agent(Box::new(ReportWriterAgent::new("Bench")))
        .with_max_iterations(1)
}

fn run_async<F: std::future::Future>(f: F) -> F::Output {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(f)
}

fn bench_ideation(c: &mut criterion::Criterion) {
    c.bench_function("crew_ideation_phase", |b| {
        b.iter(|| {
            let mut crew = create_benchmark_crew();
            run_async(async {
                crew.run_ideation("Find thermoelectric materials").await.unwrap()
            })
        });
    });
}

fn bench_planning(c: &mut criterion::Criterion) {
    c.bench_function("crew_planning_phase", |b| {
        b.iter(|| {
            let mut crew = create_benchmark_crew();
            run_async(async {
                crew.run_ideation("Find thermoelectric materials").await.unwrap();
                crew.run_planning().await.unwrap()
            })
        });
    });
}

fn bench_full_workflow(c: &mut criterion::Criterion) {
    c.bench_function("crew_full_workflow", |b| {
        b.iter(|| {
            let mut crew = create_benchmark_crew();
            run_async(async {
                crew.run_ideation("Find thermoelectric materials").await.unwrap();
                crew.run_planning().await.unwrap();
                crew.run_reporting().await.unwrap()
            })
        });
    });
}

fn bench_throughput(c: &mut criterion::Criterion) {
    c.bench_function("crew_throughput_5_concurrent", |b| {
        b.iter(|| {
            run_async(async {
                let queries = vec![
                    "Find thermoelectric materials",
                    "Find superconducting materials",
                    "Find battery materials",
                    "Find photocatalyst materials",
                    "Find magnet materials",
                ];
                // Create crews inside the async block
                let futures: Vec<_> = queries.into_iter().map(|q| async move {
                    let mut crew = create_benchmark_crew();
                    crew.run_ideation(q).await
                }).collect();
                futures::future::join_all(futures).await;
            })
        });
    });
}

criterion::criterion_group!(
    benches,
    bench_ideation,
    bench_planning,
    bench_full_workflow,
    bench_throughput
);
criterion::criterion_main!(benches);
