# rairos-kg-neo4j

Neo4j 知识图谱实现，为 Rairos 提供 Cypher 查询和图算法能力。

## 功能特性

### 节点类型
- **Paper** — arXiv 学术论文
- **Author** — 论文作者
- **Tag** — 研究主题标签
- **PNote** — 个人笔记
- **CNote** — 引用笔记
- **MNote** — 方法笔记
- **Figure** — 论文图表
- **Table** — 论文表格

### 边类型
- **cite** — 论文引用关系
- **derive** — 作者→论文归属关系
- **same_tag** — 论文→标签关系
- **has_note** — 笔记关联
- **has_figure / has_table** — 论文→图表关系

### 图算法
- **PageRank** — 论文重要性排名
- **社区发现** — 基于标签的论文聚类
- **桥接论文检测** — 高介数中心性的关键引用节点

## 快速开始

### 安装

```toml
[dependencies]
rairos-kg-neo4j = { path = "../rairos-kg-neo4j" }
```

### 连接 Neo4j

```rust
use rairos_kg_neo4j::{Neo4jKgClient, Neo4jConfig};

let client = Neo4jKgClient::new(Neo4jConfig::default()).await?;

// 测试连接
assert!(client.health_check().await?);
```

### 节点操作

```rust
use rairos_kg_neo4j::{Neo4jKgClient, NodeType, KgNode};
use serde_json::json;

let paper = KgNode::paper(
    "2301.00001",
    "Graph Neural Networks for Materials Science",
    Some(json!({"abstract": "...", "year": 2023})),
)?;

client.create_paper(&paper).await?;

// 按标签查询
let papers = client.get_papers_by_tag("machine-learning", 10).await?;
```

### Cypher 查询

```rust
use rairos_kg_neo4j::{CypherBuilder, NodeType, EdgeType};

let (query, params) = CypherBuilder::new()
    .match_node("p", NodeType::Paper)
    .where_contains("p.title", "graph neural")
    .return_nodes(vec!["p"])
    .limit(10)
    .build();

let results = client.query(&query, params).await?;
```

### 图算法

```rust
use rairos_kg_neo4j::Neo4jKgClient;

let page_ranks = client.page_rank().await?;
for result in page_ranks {
    println!("{}: {:.4}", result.arxiv_id, result.rank);
}

// 社区发现
let communities = client.detect_communities().await?;
for community in communities {
    println!("Community {}: {} papers", community.community_id, community.papers.len());
}

// 桥接论文
let bridges = client.find_bridge_papers().await?;
```

## 数据迁移

从 SQLite rairos-kg 迁移数据：

```rust
use rairos_kg_neo4j::import::{Importer, ImportConfig};

let importer = Importer::new(kg_client, sqlite_path);
importer.import_all().await?;
```

## 核心 API

### Neo4jKgClient

| 方法 | 说明 |
|------|------|
| `new(config)` | 创建客户端 |
| `health_check()` | 检查连接 |
| `query(cypher, params)` | 执行 Cypher 查询 |
| `create_paper(paper)` | 创建论文节点 |
| `get_paper(arxiv_id)` | 按 ID 获取论文 |
| `get_papers_by_tag(tag, limit)` | 按标签查询 |
| `page_rank()` | 计算 PageRank |
| `detect_communities()` | 社区发现 |
| `find_bridge_papers()` | 查找桥接论文 |

### CypherBuilder

```rust
CypherBuilder::new()
    .match_node(alias, node_type)
    .match_edge(src, edge_type, dst)
    .where_eq(field, value)
    .where_contains(field, value)
    .return_nodes(vec![...])
    .order_by(field, asc)
    .limit(n)
    .skip(n)
    .count()
    .build()
```

## 测试

```bash
cargo test -p rairos-kg-neo4j
```

## 依赖

- `reqwest` — HTTP 客户端
- `tokio` — 异步运行时
- `serde` / `serde_json` — 序列化
- `neo4j-driver` — Neo4j 驱动（可选 Bolt）
