# AI Research OS 使用指南

## 📖 目录

- [快速开始](#快速开始)
- [常见操作](#常见操作)
- [常见问题](#常见问题)
- [更多资源](#更多资源)

---

## 🚀 快速开始

> Rairos 已完全迁移为 Rust 项目（154 crates）。使用 `./rairos.sh` 或 `make run CMD='...'` 简化操作。

### 1. 搜索论文

```bash
# 搜索论文
./rairos.sh search "machine learning"

# 查看更多详情
./rairos.sh search "LLM agent" --limit 20

# 搜索后查看论文详情
./rairos.sh show <paper-id>
```

### 2. 导入论文

```bash
# 从 arXiv 导入
./rairos.sh add 2301.001

# 批量导入
./rairos.sh import papers.json
```

### 3. 查看状态

```bash
# 查看系统状态
./rairos.sh status

# 查看详细统计
./rairos.sh stats

# 列出所有论文
./rairos.sh list

# 按状态筛选
./rairos.sh list --status done
```

### 4. 导出数据

```bash
# JSON格式
./rairos.sh export --format json ./papers.json

# CSV格式
./rairos.sh export --format csv ./papers.csv
```

---

## 📚 常见操作

### 论文管理

```bash
# 删除论文
./rairos.sh delete <paper-id>

# 更新解析状态
./rairos.sh update-status <paper-id> done

# 查找相似论文
./rairos.sh similar <paper-id>

# 对比论文
./rairos.sh compare --papers <paper-a> <paper-b>

# 去重
./rairos.sh dedup find
```

### 知识图谱

```bash
# 查看知识图谱统计
./rairos.sh kg-stats

# 搜索知识图谱节点
./rairos.sh kg-search "transformer"

# 查看论文的邻居图
./rairos.sh kg-graph <paper-id> --hops 2
```

### 研究分析

```bash
# 检测研究空白
./rairos.sh gap "reinforcement learning"

# 查看研究雷达
./rairos.sh radar

# 分析趋势
./rairos.sh trend --topic "LLM"

# 查看研究时间线
./rairos.sh timeline
```

### Gene Pool（进化）

```bash
# 查看基因列表
./rairos.sh gene-list

# 查看基因详情
./rairos.sh gene-show <gene-id>

# 计算多样性
./rairos.sh gene-diversity

# 运行进化周期
./rairos.sh gene-evolve
```

### 系统管理

```bash
# 初始化数据库
./rairos.sh init

# 运行诊断
./rairos.sh doctor

# 启动后台服务
./rairos.sh daemon --foreground

# 查看版本
./rairos.sh version

# 查看帮助
./rairos.sh --help
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
./rairos.sh import papers.json
```

### Q: 如何清理缓存？

```bash
# 查看缓存
./rairos.sh cache stats

# 清理缓存
./rairos.sh cache clear
```

### Q: 如何导出论文？

```bash
# JSON导出
./rairos.sh export --format json ./papers.json

# CSV导出
./rairos.sh export --format csv ./papers.csv

# 按状态筛选导出
./rairos.sh export --format json --status done ./papers_done.json
```

### Q: 构建时内存不足怎么办？

```bash
# 推荐：使用 Makefile（自动优化）
make build-dev

# 备选：单线程构建
unset RUSTC_WRAPPER && CARGO_BUILD_JOBS=1 cargo build
```

---

## 🎓 更多资源

- [项目 GitHub](https://github.com/shushuzn/Rairos)
- [架构文档](docs/architecture.md)
- [安装指南](docs/installation.md)
- [AGENTS.md](AGENTS.md) — 完整 crate 列表和命令参考
- [CHANGELOG.md](CHANGELOG.md)

---

**提示**: 使用 `./rairos.sh --help` 查看所有可用的 105 个命令！
