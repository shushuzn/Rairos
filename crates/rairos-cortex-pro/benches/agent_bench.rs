//! Agent performance benchmarks

use rairos_cortex_pro::sparks_agents::{
    HypothesisAgent, HypothesisCriticAgent, PlannerAgent,
    PlanCriticAgent, ExecutorAgent, ReportWriterAgent,
};
use rairos_cortex_pro::agent::Agent;
use rairos_cortex_pro::state::ResearchState;

fn create_test_state(query: &str) -> ResearchState {
    let mut state = ResearchState::new(query);
    state.intermediate.insert(
        "query".to_string(),
        serde_json::json!(query),
    );
    state.intermediate.insert(
        "hypothesis".to_string(),
        serde_json::json!("Doping Bi2Te3 with Se improves thermoelectric performance by reducing thermal conductivity while maintaining high power factor. This is a well-established strategy in thermoelectric materials design."),
    );
    state.intermediate.insert(
        "plan".to_string(),
        serde_json::json!(r#"{"rationale": "Test", "steps": [{"step": 1, "task": "Test", "tool": "", "inputs": {}, "depends_on": []}]}"#),
    );
    state
}

fn run_async<F: std::future::Future>(f: F) -> F::Output {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(f)
}

fn bench_hypothesis_agent(c: &mut criterion::Criterion) {
    let agent = HypothesisAgent::new("bench");
    let state = create_test_state("Find thermoelectric materials");
    c.bench_function("agent_hypothesis_execute", |b| {
        b.iter(|| {
            run_async(async {
                agent.execute(&state).await.unwrap()
            })
        });
    });
}

fn bench_hypothesis_critic_agent(c: &mut criterion::Criterion) {
    let agent = HypothesisCriticAgent::new("bench");
    let state = create_test_state("Find thermoelectric materials");
    c.bench_function("agent_hypothesis_critic_execute", |b| {
        b.iter(|| {
            run_async(async {
                agent.execute(&state).await.unwrap()
            })
        });
    });
}

fn bench_planner_agent(c: &mut criterion::Criterion) {
    let agent = PlannerAgent::new("bench");
    let state = create_test_state("Find thermoelectric materials");
    c.bench_function("agent_planner_execute", |b| {
        b.iter(|| {
            run_async(async {
                agent.execute(&state).await.unwrap()
            })
        });
    });
}

fn bench_plan_critic_agent(c: &mut criterion::Criterion) {
    let agent = PlanCriticAgent::new("bench");
    let state = create_test_state("Find thermoelectric materials");
    c.bench_function("agent_plan_critic_execute", |b| {
        b.iter(|| {
            run_async(async {
                agent.execute(&state).await.unwrap()
            })
        });
    });
}

fn bench_executor_agent(c: &mut criterion::Criterion) {
    let mut state = create_test_state("Find thermoelectric materials");
    state.intermediate.insert(
        "current_task".to_string(),
        serde_json::json!("Calculate formation energy for Bi2Te3"),
    );
    let agent = ExecutorAgent::new("bench");
    c.bench_function("agent_executor_execute", |b| {
        b.iter(|| {
            run_async(async {
                agent.execute(&state).await.unwrap()
            })
        });
    });
}

fn bench_report_writer_agent(c: &mut criterion::Criterion) {
    let agent = ReportWriterAgent::new("bench");
    let state = create_test_state("Find thermoelectric materials");
    c.bench_function("agent_report_writer_execute", |b| {
        b.iter(|| {
            run_async(async {
                agent.execute(&state).await.unwrap()
            })
        });
    });
}

criterion::criterion_group!(
    benches,
    bench_hypothesis_agent,
    bench_hypothesis_critic_agent,
    bench_planner_agent,
    bench_plan_critic_agent,
    bench_executor_agent,
    bench_report_writer_agent
);
criterion::criterion_main!(benches);
