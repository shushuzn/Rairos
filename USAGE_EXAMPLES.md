# AI Research OS 使用指南

## 📖 目录

- [快速开始](#快速开始)
- [常见操作](#常见操作)
- [常见问题](#常见问题)
- [更多资源](#更多资源)

---

## 🚀 快速开始

> Rairos 已完全迁移为 Rust 项目（154 crates）。以下所有命令通过 `cargo run -p rairos-cli --` 执行，或使用别名 `alias rairos='cargo run -p rairos-cli --'` 简化操作。

### 1. 搜索论文

```bash
# 搜索论文
rairos search "machine learning"

# 查看更多详情
rairos search "LLM agent" --limit 20

# 搜索后查看论文详情
rairos show <paper-id>
```

### 2. 导入论文

```bash
# 从 arXiv 导入
rairos add 2301.001

# 批量导入
rairos import papers.json
```

### 3. 查看状态

```bash
# 查看系统状态
rairos status

# 查看详细统计
rairos stats

# 列出所有论文
rairos list

# 按状态筛选
rairos list --status done
```

### 4. 导出数据

```bash
# JSON格式
rairos export --format json ./papers.json

# CSV格式
rairos export --format csv ./papers.csv
```

---

## 📚 常见操作

### 论文管理

```bash
# 删除论文
rairos delete <paper-id>

# 更新解析状态
rairos update-status <paper-id> done

# 查找相似论文
rairos similar <paper-id>

# 对比论文
rairos compare --papers <paper-a> <paper-b>

# 去重
rairos dedup find
```

### 知识图谱

```bash
# 查看知识图谱统计
rairos kg-stats

# 搜索知识图谱节点
rairos kg-search "transformer"

# 查看论文的邻居图
rairos kg-graph <paper-id> --hops 2
```

### 研究分析

```bash
# 检测研究空白
rairos gap "reinforcement learning"

# 查看研究雷达
rairos radar

# 分析趋势
rairos trend --topic "LLM"

# 查看研究时间线
rairos timeline
```

### Gene Pool（进化）

```bash
# 查看基因列表
rairos gene-list

# 查看基因详情
rairos gene-show <gene-id>

# 计算多样性
rairos gene-diversity

# 运行进化周期
rairos gene-evolve
```

### 系统管理

```bash
# 初始化数据库
rairos init

# 运行诊断
rairos doctor

# 启动后台服务
rairos daemon --foreground

# 查看版本
rairos version

# 查看帮助
rairos --help
```

---

## 🔧 常见问题

### Q: 如何设置环境变量？

```bash
# 数据库路径
export RAIROS_DB=/path/to/rairos.db

# 数据存储目录
export RAIROS_DATA_DIR=/path/to/data

# LLM API 密钥（按需）
export OPENAI_API_KEY="sk-..."
```

### Q: 如何批量导入论文？

```bash
# 准备 JSON 文件
cat > papers.json << 'EOF'
[
  {
    "id": "paper_1",
    "arxiv_id": "2301.001",
    "title": "Paper Title",
    "authors": ["Author A"],
    "abstract_text": "Abstract...",
    "categories": ["cs.AI"],
    "published": "2023-01-01T00:00:00Z",
    "parse_status": "pending",
    "metadata": {"cited_by": 0, "references": 0, "doi": null, "pdf_url": null}
  }
]
EOF

# 导入
rairos import papers.json
```

### Q: 如何清理缓存？

```bash
# 查看缓存
rairos cache stats

# 清理缓存
rairos cache clear
```

### Q: 如何导出论文？

```bash
# JSON导出
rairos export --format json ./papers.json

# CSV导出
rairos export --format csv ./papers.csv

# 按状态筛选导出
rairos export --format json --status done ./papers_done.json
```

### Q: 构建时内存不足怎么办？

```bash
# 单线程构建（推荐）
CARGO_BUILD_JOBS=1 cargo build --workspace

# 只构建 CLI（更快）
cargo build -p rairos-cli
```

---

## 🎓 更多资源

- [项目 GitHub](https://github.com/shushuzn/Rairos)
- [架构文档](docs/architecture.md)
- [安装指南](docs/installation.md)
- [AGENTS.md](AGENTS.md) — 完整 crate 列表和命令参考
- [CHANGELOG.md](CHANGELOG.md)

---

**提示**: 使用 `rairos --help` 查看所有可用的 104 个命令！
