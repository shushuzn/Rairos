# Rairos Plugin for Claude Code

Rairos 封装为 Claude Code 的 MCP 插件，让 Claude 能够直接调用研究工具。

## 安装

### 1. 配置 MCP 服务器

在 Claude Code 的 `settings.json` 中添加：

```json
{
  "mcpServers": {
    "rairos": {
      "command": "python",
      "args": ["-m", ".claude.plugins.rairos.server"],
      "cwd": "/path/to/rairos"
    }
  }
}
```

### 2. 重启 Claude Code

## 可用工具

| 工具 | 功能 |
|------|------|
| `paper_ingest` | 导入论文（arXiv ID / DOI / PDF） |
| `paper_search` | 全文搜索论文 |
| `paper_chat` | RAG 问答 |
| `kg_query` | 知识图谱查询 |
| `chart_query` | 图表查询 |
| `research_run` | 运行研究循环 |
| `slides_generate` | 生成幻灯片 |
| `cite_fetch` | 获取引用关系 |
| `paper_analyze` | 论文分析 |

## 使用示例

```
你: "搜索关于 transformer 架构的论文"
Claude: → paper_search { query: "transformer architecture" }

你: "导入论文 2601.00155"
Claude: → paper_ingest { identifier: "2601.00155", tags: ["LLM", "Agent"] }

你: "查询这篇论文的 Figure 3"
Claude: → chart_query { paper_id: "2601.00155", action: "figure", label: "Figure 3" }
```

## 前提条件

- Python >= 3.10
- 已安装 rairos 依赖: `pip install -e .`
- 配置好 `OPENAI_API_KEY` 等环境变量
