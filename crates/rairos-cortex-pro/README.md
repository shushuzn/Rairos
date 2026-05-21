# rairos-cortex-pro

生产级多 Agent 协作框架，提供 LangGraph 风格的工作流编排。

## 功能特性

### Agent 角色
- **ResearcherAgent** — 论文搜索、提取、索引
- **GapAnalyzerAgent** — 研究空白检测
- **CitationGraphAgent** — 引用网络分析
- **VectorIndexerAgent** — 向量存储和检索
- **ReportWriterAgent** — 合成与报告生成
- **QaAgent** — 质量验证

### Crew 编排
- 多 Agent 协作
- 超时控制
- 迭代限制
- 结果聚合

### LangGraph 风格 Pipeline
- 有向无环图 (DAG)
- 节点和边定义
- 拓扑排序执行
- 循环检测
- 条件分支

### 状态机
- 8 个研究阶段：Planning → Searching → Extracting → Analyzing → BuildingGraph → Indexing → Writing → Validating → Complete
- 共享 ResearchState
- 阶段转换追踪

## 快速开始

### 安装

```toml
[dependencies]
rairos-cortex-pro = { path = "../rairos-cortex-pro" }
```

### 简单示例

```rust
use rairos_cortex_pro::{ResearchCrew, CrewConfig};

let crew = ResearchCrew::new(CrewConfig::default());

let result = crew.run("machine learning for materials discovery").await?;

println!("Report: {}", result.report);
println!("Papers found: {}", result.papers.len());
println!("Gaps identified: {}", result.gaps.len());
```

### 自定义 Agent

```rust
use rairos_cortex_pro::{
    Agent, AgentConfig, AgentOutput, AgentRole,
    AgentContext, ResearchState, Phase,
};
use async_trait::async_trait;

struct MyAgent;

#[async_trait]
impl Agent for MyAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Researcher
    }

    fn name(&self) -> &str {
        "MyAgent"
    }

    async fn execute(&self, state: &ResearchState, ctx: &AgentContext) -> Result<AgentOutput> {
        // 执行任务
        Ok(AgentOutput::success("task completed"))
    }
}

let crew = ResearchCrew::builder()
    .add_agent(MyAgent)
    .with_max_iterations(5)
    .with_timeout(Duration::from_secs(300))
    .build();
```

### Pipeline 工作流

```rust
use rairos_cortex_pro::{
    Pipeline, PipelineNode, PipelineNodeType, PipelineEdge,
};

let mut pipeline = Pipeline::new("research");

pipeline.add_node(PipelineNode::new("start", PipelineNodeType::Start));
pipeline.add_node(PipelineNode::new("search", PipelineNodeType::Agent("researcher".to_string())));
pipeline.add_node(PipelineNode::new("analyze", PipelineNodeType::Agent("analyzer".to_string())));
pipeline.add_node(PipelineNode::new("write", PipelineNodeType::Agent("writer".to_string())));
pipeline.add_node(PipelineNode::new("end", PipelineNodeType::End).terminal());

pipeline.add_edge(PipelineEdge::new("start", "search"));
pipeline.add_edge(PipelineEdge::new("search", "analyze"));
pipeline.add_edge(PipelineEdge::new("analyze", "write"));
pipeline.add_edge(PipelineEdge::new("write", "end"));

// 验证 DAG
assert!(pipeline.validate().is_ok());

// 获取执行顺序
let order = pipeline.execution_order()?;
assert_eq!(order, vec!["start", "search", "analyze", "write", "end"]);
```

### 预设模板

```rust
use rairos_cortex_pro::pipeline::templates;

// 顺序研究流程
let sequential = templates::sequential_research();
// 6 个节点：start → search → extract → analyze → write → end

// 并行研究流程
let parallel = templates::parallel_research();
// 10 个节点，支持并行搜索和分析
```

## 架构

```
                    ResearchCrew
                        │
    ┌───────────────────┼───────────────────┐
    │                   │                   │
    ▼                   ▼                   ▼
ResearcherAgent   GapAnalyzerAgent   CitationGraphAgent
    │                   │                   │
    │                   │                   │
    └───────────────────┼───────────────────┘
                        │
                        ▼
               ResearchState
    ┌───────────────────┼───────────────────┐
    │                   │                   │
  papers              gaps              outputs
    │                   │                   │
    └───────────────────┼───────────────────┘
                        │
                        ▼
               VectorIndexerAgent
                        │
                        ▼
                 ReportWriterAgent
                        │
                        ▼
                     QaAgent
                        │
                        ▼
                   CrewResult
```

## 核心 API

### Agent Trait

```rust
#[async_trait]
pub trait Agent: Send + Sync {
    fn role(&self) -> AgentRole;
    fn name(&self) -> &str;
    async fn execute(&self, state: &ResearchState, ctx: &AgentContext) -> Result<AgentOutput>;
}
```

### Phase 状态

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Planning,
    Searching,
    Extracting,
    Analyzing,
    BuildingGraph,
    Indexing,
    Writing,
    Validating,
    Complete,
}
```

### CrewConfig

```rust
let config = CrewConfig::builder()
    .name("research-crew")
    .add_agent(ResearcherAgent)
    .add_agent(GapAnalyzerAgent)
    .add_agent(ReportWriterAgent)
    .with_max_iterations(5)
    .with_timeout(Duration::from_secs(600))
    .build();
```

## 工具集成

Agent 可以访问多种工具：

```rust
use rairos_cortex_pro::AgentContext;

let ctx = AgentContext {
    llm: arc_llm,
    vector_store: Some(arc_vector_store),  // 可选
    kg_client: Some(arc_kg_client),        // 可选
    graphrag: Some(arc_graphrag),           // 可选
};

// 在 Agent 中使用
let search_results = ctx.vector_store.search(&query, 10, None).await?;
let kg_results = ctx.kg_client.query_cypher(&cypher).await?;
```

## 测试

```bash
cargo test -p rairos-cortex-pro
```

## 依赖

- `async-trait` — 异步 trait
- `tokio` — 异步运行时
- `serde` / `serde_json` — 序列化
- `chrono` — 时间处理
- `rairos-vector` — 向量存储
- `rairos-kg-neo4j` — 知识图谱
- `rairos-graphrag` — GraphRAG
- `rairos-llm` — LLM 生成
