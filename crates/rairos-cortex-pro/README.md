# Rairos Cortex Pro

Advanced multi-agent research orchestration for materials discovery.

## SparksMatter Integration

This crate implements a SparksMatter-style multi-agent workflow for autonomous materials research, featuring:

### Architecture

```
User Query
    │
    ▼
┌─────────────────┐
│     Manager     │ ◄─── Orchestrates workflow
└────────┬────────┘
         │
    ┌────┴────┬────────────┐
    ▼          ▼            ▼
┌───────┐ ┌────────┐ ┌──────────┐
│Scientist│ │Scientist│ │ Planner  │ ◄─── Hypothesis/Plan agents
│   1   │ │   2    │ └────┬─────┘
└───────┘ └────┬───┘      │
               ▼            ▼
         ┌──────────┐ ┌───────┐
         │ Critic   │ │Critic │ ◄─── Review agents
         └────┬─────┘ └───┬───┘
              │            │
               └─────┬────┘
                     ▼
               ┌──────────┐
               │Assistant │ ◄─── Execution agent
               └────┬─────┘
                    │
                    ▼
             ┌──────────────┐
             │ MaterialTools │ ◄─── MP, CGCNN, MatterGen
             └──────────────┘
```

### Workflow Phases

1. **Ideation**: Generate and critique hypothesis
   - `HypothesisAgent` → `HypothesisCriticAgent` → (iterate until approved)

2. **Planning**: Create and review research plan
   - `PlannerAgent` → `PlanCriticAgent` → (iterate until approved)

3. **Execution**: Execute plan using material science tools
   - `ExecutorAgent` → Tool execution

4. **Reporting**: Generate structured LaTeX report
   - `ReportWriterAgent`

### Quick Start

```rust
use rairos_cortex_pro::sparks_crew::SparksCrew;
use rairos_cortex_pro::sparks_agents::{
    HypothesisAgent, HypothesisCriticAgent, PlannerAgent,
    PlanCriticAgent, ExecutorAgent, ReportWriterAgent,
};
use std::sync::Arc;
use rairos_llm::YourLlmClient;

// Create crew with LLM and agents
let crew = SparksCrew::new(Arc::new(your_llm_client))
    .add_agent(Box::new(HypothesisAgent::new("Scientist1")))
    .add_agent(Box::new(HypothesisCriticAgent::new("Scientist2")))
    .add_agent(Box::new(PlannerAgent::new("Planner")))
    .add_agent(Box::new(PlanCriticAgent::new("PlanReviewer")))
    .add_agent(Box::new(ExecutorAgent::new("Assistant")))
    .add_agent(Box::new(ReportWriterAgent::new("ReportWriter")))
    .with_max_iterations(3);

// Run full workflow
let result = crew.run("Find high-performance thermoelectric materials").await?;
```

### Individual Phases

```rust
// Phase 1: Ideation
let hypothesis = crew.run_ideation("Find thermoelectric materials").await?;

// Phase 2: Planning
let plan = crew.run_planning().await?;

// Phase 3: Execution (requires tools)
let execution = crew.run_execution(&plan).await?;

// Phase 4: Reporting
let report = crew.run_reporting().await?;
```

### Available Agents

| Agent | Role | Description |
|-------|------|-------------|
| `HypothesisAgent` | `Hypothesis` | Generates novel research hypotheses |
| `HypothesisCriticAgent` | `HypothesisCritic` | Reviews and approves/rejects hypotheses |
| `PlannerAgent` | `Planner` | Creates detailed research plans (JSON output) |
| `PlanCriticAgent` | `PlanCritic` | Reviews and approves/rejects plans |
| `ExecutorAgent` | `Executor` | Executes tasks and tool calls |
| `ReportWriterAgent` | `ReportWriter` | Generates LaTeX research reports |

### Tool Integration

When the `tools` feature is enabled, you can register material science tools:

```rust
use rairos_tools::{MaterialsProjectTool, CgcnnRegressor, MatterGenGenerator};

// Add tools
let crew = crew
    .add_tool(Arc::new(MaterialsProjectTool::new("your-api-key")))
    .add_tool(Arc::new(CgcnnRegressor::new("model-path")))
    .add_tool(Arc::new(MatterGenGenerator::new()));

// Tools are automatically used during execution phase
```

### Features

- `default`: Basic functionality
- `tools`: Enables material science tool integrations (Materials Project, CGCNN, MatterGen)
- `api`: Enables HTTP API server for remote workflow execution

### Retry Logic

The crew implements exponential backoff retry for agent calls:

```rust
use rairos_cortex_pro::sparks_crew::RetryConfig;

// Configure retry behavior
let retry_config = RetryConfig::default()
    .with_max_attempts(5); // 5 attempts instead of default 3

let crew = SparksCrew::new(llm)
    .with_retry_config(retry_config);
```

### Streaming Support

Track workflow progress with callbacks:

```rust
use rairos_cortex_pro::sparks_crew::{PhaseResult, StreamingCallback};

// Define progress callback
let callback: StreamingCallback = Box::new(|role, status| {
    println!("{:?}: {}", role, status);
});

let crew = SparksCrew::new(llm)
    .with_streaming_callback(callback);
```

### Testing

```bash
# Run all tests
cargo test -p rairos-cortex-pro

# Run with tools feature
cargo test -p rairos-cortex-pro --features tools

# Run integration tests
cargo test -p rairos-cortex-pro --features tools --test sparks_integration_test
```

### Running the Demo

A complete demo is available in `examples/sparks_matter_demo.rs`:

```bash
# Prerequisites: Run Ollama
# 1. Install Ollama: https://ollama.ai/
# 2. Start Ollama: ollama serve
# 3. Pull a model: ollama pull llama3.2

# Run the demo
cd /tmp/Rairos
cargo run --example sparks_matter_demo -p rairos-cortex-pro --features tools,llm
```

The demo will:
1. Check Ollama availability
2. Run the full 4-phase workflow (Ideation → Planning → Execution → Reporting)
3. Display the generated hypothesis, plan, and research report

### Plan JSON Format

The `PlannerAgent` outputs research plans in JSON format:

```json
{
  "rationale": "Why this approach",
  "steps": [
    {
      "step": 1,
      "task": "Description of task",
      "tool": "tool-name or empty",
      "inputs": {"key": "value"},
      "depends_on": []
    }
  ],
  "other_tasks": ["Tasks beyond tool capabilities"]
}
```

## Related Crates

- `rairos-tools` - Material science tool implementations
- `rairos-vector` - Vector storage and retrieval
- `rairos-kg-neo4j` - Knowledge graph with Neo4j
- `rairos-graphrag` - Graph-based RAG
- `rairos-pdf-advanced` - PDF processing
