# rairos-graphrag

GraphRAG 实现，结合向量检索与知识图谱推理的问答管线。

## 功能特性

### 混合检索
- 向量相似度 × 图结构权重组合
- 可配置向量/图权重比例
- 图节点 boost（高 PageRank / 桥接节点优先）

### 社区检测
- 基于标签的论文社区划分
- 社区级别摘要生成
- 按主题分组检索结果

### 多跳推理
- BFS 扩展查询实体的邻域
- 路径连贯性评分
- 引用路径追踪

### 端到端 RAG
- 8 步查询流程：检索→展开→社区→摘要→路径→生成→验证→返回

## 快速开始

### 安装

```toml
[dependencies]
rairos-graphrag = { path = "../rairos-graphrag" }
rairos-vector = { path = "../rairos-vector" }
rairos-kg-neo4j = { path = "../rairos-kg-neo4j" }
```

### 初始化管线

```rust
use rairos_graphrag::{GraphRagPipeline, GraphRagConfig};
use rairos_vector::{OpenAiEmbedder, ChromaClient};
use rairos_kg_neo4j::Neo4jKgClient;
use rairos_llm::OpenAiClient;

let embedder = OpenAiEmbedder::new("sk-...")?;
let vector_store = ChromaClient::new("http://localhost:8000");
let kg_client = Neo4jKgClient::new(Default::default()).await?;
let llm = OpenAiClient::new("sk-...")?;

let config = GraphRagConfig::default();
let pipeline = GraphRagPipeline::new(
    embedder,
    vector_store,
    kg_client,
    llm,
    config,
);
```

### 问答

```rust
let answer = pipeline.query("How do GNNs help molecular property prediction?").await?;

println!("Answer: {}", answer.answer);
println!("Confidence: {:.2}", answer.confidence);
for source in &answer.sources {
    println!("- {} ({:.2})", source.id, source.relevance);
}
```

### 自定义配置

```rust
use rairos_graphrag::GraphRagConfig;

let config = GraphRagConfig {
    vector_top_k: 20,          // 向量检索返回数量
    graph_depth: 3,            // 图扩展深度
    vector_weight: 0.4,       // 向量权重
    graph_weight: 0.6,         // 图权重
    community_levels: 2,       // 社区检测层级
    max_hops: 5,               // 最大跳数
    min_path_coherence: 0.3,   // 最小路径连贯性
};
```

## 架构

```
                    用户问题
                        │
         ┌─────────────┴─────────────┐
         ▼                           ▼
   向量检索 (Top-K)            图扩展 (BFS)
         │                           │
         ▼                           ▼
   候选文档                    邻域节点
         │                           │
         └─────────────┬─────────────┘
                       ▼
              分数组合 + Graph Boost
                       │
                       ▼
              社区检测 + 摘要生成
                       │
                       ▼
              多跳路径检索
                       │
                       ▼
              LLM 生成答案
                       │
                       ▼
                   返回结果
```

## 核心 API

### GraphRagPipeline

```rust
let pipeline = GraphRagPipeline::new(
    embedder,      // rairos-vector Embedder
    vector_store,  // rairos-vector VectorStore
    kg_client,     // rairos-kg-neo4j Neo4jKgClient
    llm,           // rairos-llm LlmClient
    config,        // GraphRagConfig
);

let answer = pipeline.query(question).await?;
```

### HybridRetriever

```rust
let retriever = HybridRetriever::new(
    vector_store,
    kg_client,
    config.vector_weight,
    config.graph_weight,
);

let results = retriever.hybrid_search(&query_embedding, top_k).await?;
```

### PathFinder

```rust
let finder = PathFinder::new(kg_client, max_hops);

let paths = finder.find_paths("paper1", "paper2").await?;
let bridges = finder.find_bridges("paper1", 3).await?;
```

## 测试

```bash
cargo test -p rairos-graphrag
```

## 依赖

- `rairos-vector` — 向量存储和检索
- `rairos-kg-neo4j` — 知识图谱
- `rairos-llm` — LLM 生成
- `reqwest` — HTTP 客户端
- `tokio` — 异步运行时
