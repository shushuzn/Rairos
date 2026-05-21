# rairos-pdf-advanced

高级 PDF 解析能力，包括 GROBID 集成、命名实体识别（NER）和三元组抽取。

## 功能特性

### GROBID 集成
- **Header 提取** — 标题、作者、摘要
- **Reference 解析** — 引文列表解析
- **Full-text 处理** — 完整 PDF 结构化

### 结构化解析
- **段落检测** — 句子边界、段落分组
- **标题识别** — 编号/大写/冒号结尾
- **公式检测** — LaTeX / inline 公式
- **列表检测** — 有序/无序列表

### 命名实体识别 (NER)

支持实体类型：

| 类型 | 示例 |
|------|------|
| Chemical | H2O, Fe2O3, graphene |
| Dataset | ImageNet, CIFAR-10, MNIST |
| Method | GNN, Transformer, CNN |
| Software | PyTorch, TensorFlow, VASP |
| Property | accuracy, F1 score, bandgap |
| Unit | ms, kg, eV, K |
| ArxivId | 2301.00001 |
| Doi | 10.1000/xyz123 |

### 三元组抽取
- **关系模式** — achieves, outperforms, trained on, improves
- **RDF 格式** — Subject-Predicate-Object
- **置信度评分** — 每个三元组的置信度

## 快速开始

### 安装

```toml
[dependencies]
rairos-pdf-advanced = { path = "../rairos-pdf-advanced" }
```

### GROBID 解析

```rust
use rairos_pdf_advanced::{GrobidClient, PdfAdvancedError};

let grobid = GrobidClient::new("http://localhost:8080");

// 健康检查
assert!(grobid.health_check().await?);

// 处理 PDF
let pdf_bytes = std::fs::read("paper.pdf")?;
let result = grobid.process_fulltext_document(&pdf_bytes).await?;

println!("Title: {}", result.header.title);
println!("Authors: {:?}", result.header.authors);
println!("Abstract: {}", result.header.abstract_text);
```

### 文档结构分析

```rust
use rairos_pdf_advanced::{DocumentAnalyzer, Paragraph, Section};

let analyzer = DocumentAnalyzer::new();

// 分析段落
let para = Paragraph::new("This is sentence one. This is sentence two.");
assert!(!para.is_short());

// 分析标题
assert!(analyzer.is_heading("1. Introduction"));
assert!(analyzer.is_heading("MATERIALS AND METHODS"));

// 提取章节
let sections = analyzer.analyze_sections(&text);
for section in sections {
    println!("{}: {} chars", section.heading, section.char_count);
}
```

### NER 实体抽取

```rust
use rairos_pdf_advanced::{NerPipeline, EntityType};

let ner = NerPipeline::new();

// 提取实体
let text = "We train a GNN on the ImageNet dataset and achieve 89% accuracy.";
let entities = ner.extract(text).await?;

for entity in &entities {
    println!("{:?}: {} (conf: {:.2})", entity.entity_type, entity.text, entity.confidence);
}

// 统计
let counts = ner.count_by_type(&entities);
println!("Methods: {}, Datasets: {}, Properties: {}",
    counts[&EntityType::Method],
    counts[&EntityType::Dataset],
    counts[&EntityType::Property],
);
```

### 三元组抽取

```rust
use rairos_pdf_advanced::{TripleExtractor, Triple};

let extractor = TripleExtractor::new();

let text = "Our model achieves 95% accuracy on CIFAR-10, outperforming previous methods.";
let triples = extractor.extract(text).await?;

for triple in triples {
    println!("{} → {} → {} (conf: {:.2})",
        triple.subject, triple.predicate, triple.object, triple.confidence);
}
```

### 知识图谱构建

```rust
use rairos_pdf_advanced::{KnowledgeGraphBuilder, TripleExtractor};

let extractor = TripleExtractor::new();
let builder = KnowledgeGraphBuilder::new();

let triples = extractor.extract(&full_text).await?;
builder.add_triples(&triples);

println!("Nodes: {}, Edges: {}", builder.node_count(), builder.edge_count());
```

## 核心 API

### GrobidClient

| 方法 | 说明 |
|------|------|
| `new(base_url)` | 创建客户端 |
| `health_check()` | 检查 GROBID 服务 |
| `process_header(pdf)` | 仅提取 header |
| `process_references(pdf)` | 仅解析引用 |
| `process_fulltext_document(pdf)` | 完整解析 |

### NerPipeline

| 方法 | 说明 |
|------|------|
| `new()` | 创建默认 NER pipeline |
| `with_features(...)` | 启用特定实体类型 |
| `extract(text)` | 提取实体 |
| `count_by_type(entities)` | 按类型统计 |

### TripleExtractor

| 方法 | 说明 |
|------|------|
| `new()` | 创建默认抽取器 |
| `extract(text)` | 抽取三元组 |
| `split_sentences(text)` | 分句 |

## GROBID 部署

```bash
# Docker 部署
docker pull lfergq/grobid:latest
docker run -p 8080:8080 lfergq/grobid:latest

# 或源码编译
git clone https://github.com/kermitt2/grobid.git
cd grobid
./gradlew run
```

## 测试

```bash
cargo test -p rairos-pdf-advanced
```

## 依赖

- `reqwest` — HTTP 客户端
- `tokio` — 异步运行时
- `serde` / `serde_json` — 序列化
- `regex` — 文本模式匹配
- `chrono` — 时间处理
