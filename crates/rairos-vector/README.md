# rairos-vector

向量数据库基础设施，为 Rairos RAG 管线提供 Embedding 生成和向量检索能力。

## 功能特性

### Embedding 生成
- **OpenAI Embeddings** — `text-embedding-3-small`, `text-embedding-3-large`, `ada-002`
- **BGE-M3** — 智源开源 BGE-M3 模型（通过 MiniMax API）

### 向量存储
- **Chroma** — HTTP API 客户端
- **Milvus** — HTTP API 客户端
- **FAISS** — 本地向量索引（支持 IVF、HNSW）

### RAG Pipeline
- 检索增强生成管线
- 支持自定义 Embedder、VectorStore、LlmClient 组合

## 快速开始

### 安装

```toml
[dependencies]
rairos-vector = { path = "../rairos-vector" }
```

### Embedding 示例

```rust
use rairos_vector::{OpenAiEmbedder, EmbeddingModel};

let embedder = OpenAiEmbedder::new("sk-your-api-key");
let embeddings = embedder.embed(vec!["Hello world".to_string()]).await?;
```

### 向量检索示例

```rust
use rairos_vector::{ChromaClient, VectorStore};

let store = ChromaClient::new("http://localhost:8000");

// 存储向量
store.upsert_one("doc1", &embedding, Some(json!({"text": "content"}))).await?;

// 检索
let results = store.search(&query_embedding, 5, None).await?;
for hit in results {
    println!("{}: {:.4}", hit.id, hit.score);
}
```

### RAG 问答示例

```rust
use rairos_vector::{RagPipeline, OpenAiEmbedder, InMemoryVectorStore};
use rairos_llm::OpenAiClient;

let embedder = OpenAiEmbedder::new("sk-...");
let store = InMemoryVectorStore::new();
let llm = OpenAiClient::new("sk-...")?;

let rag = RagPipeline::new(embedder, store, llm);

rag.add_document("paper1", "GNNs are effective for molecular property prediction.").await?;

let answer = rag.query("What are GNNs good for?").await?;
println!("{}", answer);
```

## 核心 API

### Traits

```rust
pub trait Embedder {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;
    fn model(&self) -> &EmbeddingModel;
}

pub trait VectorStore {
    async fn upsert_one(&self, id: &str, embedding: &[f32], payload: Option<Value>) -> Result<()>;
    async fn search(&self, query: &[f32], top_k: usize, filters: Option<Value>) -> Result<Vec<SearchHit>>;
    async fn get(&self, id: &str) -> Result<Option<(Vec<f32>, Value)>>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn count(&self) -> Result<usize>;
}
```

### 配置

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `distance_metric` | Cosine | 距离度量：Cosine、Euclidean、DotProduct |
| `batch_size` | 32 | 批处理大小 |
| `dimension` | auto | 向量维度（自动设置） |

## 测试

```bash
cargo test -p rairos-vector
```

## 依赖

- `reqwest` — HTTP 客户端
- `tokio` — 异步运行时
- `serde` / `serde_json` — 序列化
- `faiss` / `tantivy` — 本地索引
