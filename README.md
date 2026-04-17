# 🧠 AI Research OS

**A Structured Research Operating System for Serious AI Researchers**

[![Python](https://img.shields.io/badge/Python-3.9%2B-blue)](https://python.org)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Mac%20%7C%20Linux-lightgrey)](#)
[![Tests](https://img.shields.io/badge/Tests-372%20passing-brightgreen)](#-testing)
[![Coverage](https://raw.githubusercontent.com/shushuzn/ai_research_os/main/coverage-badge.svg)](#)
[![LLM](https://img.shields.io/badge/LLM-OpenAI%20Compatible-orange)](#-ai-assisted-draft)
[![License](https://img.shields.io/badge/License-Research--Only-purple)](#-license)

---

## TL;DR

Feed it a paper (arXiv URL, DOI, or PDF). Get back a **P-Note**, **C-Note**, **Radar entry**, and **Timeline entry** — all structured, tagged, and cross-linked. Optionally generate AI-assisted drafts.

```bash
# One paper
python ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM,Agent

# With AI draft (requires API key)
python ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM --ai
```

This is **not a PDF manager**. It is a **Cognitive Upgrade System** that enforces structured thinking, explicit reasoning, and long-term research tracking.

---

## What It Does

| Input | Output |
|-------|--------|
| arXiv URL/ID | P-Note + C-Note + Radar + Timeline |
| DOI | P-Note + C-Note + Radar + Timeline |
| Local PDF | P-Note + C-Note + Radar + Timeline |
| Scanned PDF | Same (via OCR) |
| `--ai` flag | + AI-structured draft (待核验) |

---

## Installation

### Dependencies

```bash
pip install requests feedparser pymupdf
```

### OCR (optional, for scanned PDFs)

```bash
pip install pytesseract pillow
```

**Windows**: Download Tesseract from [UB-Mannheim/tesseract](https://github.com/UB-Mannheim/tesseract/wiki) — add to PATH and install Chinese (`chi_sim`).

### AI Draft (optional)

Requires an OpenAI-compatible API key. See [API_CONFIG.md](API_CONFIG.md) for full configuration.

```bash
# Set environment variables
export OPENAI_API_KEY="***"
export OPENAI_BASE_URL="https://dashscope.aliyuncs.com/compatible-mode/v1"

# Or pass directly as arguments (highest priority)
python ai_research_os.py <input> --api-key "sk-..." --base-url "https://..." --model "qwen3.5-plus"
```

---

## Usage Examples

### arXiv Paper

```bash
python ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM,Agent
python ai_research_os.py 2601.00155 --tags LLM
```

### DOI

```bash
python ai_research_os.py 10.48550/arXiv.2601.00155 --tags LLM
```

### Local PDF

```bash
python ai_research_os.py test --pdf "paper.pdf" --tags RAG
python ai_research_os.py test --pdf "scanned.pdf" --ocr --ocr-lang chi_sim+eng
```

### With AI Draft

```bash
python ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM --ai
```

---

## Project Structure

```
ai_research_os/
├── core/              # Paper dataclass, constants, today_iso
├── parsers/           # arXiv fetch, Crossref fetch, DOI/arXiv detection
├── pdf/               # download, extract (PyMuPDF + OCR + pdfminer)
├── sections/          # section segmentation + formatting
├── llm/               # OpenAI-compatible client, AI draft generation
├── renderers/         # P-Note, C-Note, M-Note rendering
├── notes/             # frontmatter, tag inference, note collection
├── updaters/          # Radar heat tracking, Timeline
├── cli.py             # CLI entry point + argparse
└── tests/             # 372 tests
```

---

## Research Tree Output

Papers are organized into a 12-directory structure:

```
00-Radar/          # Topic heat tracking
01-Foundations/    # Foundational papers
02-Models/         # Model papers
03-Applications/   # Application papers
04-Evaluation/     # Evaluation methods
05-Tools/          # Tools & libraries
06-Theory/         # Theory papers
07-Architecture/   # System architecture
08-Training/       # Training methods
09-Data/           # Datasets
10-Applications/   # Applied research
11-Future-Directions/
```

---

## Knowledge Evolution Logic

```
Paper → P-Note (paper note)
  → C-Note (concept note, per tag)
  → M-Note (comparison note, when 3+ papers share same tag)
  → Radar (topic frequency heat score)
  → Timeline (year-based evolution)
```

---

## CLI Reference

| Argument | Description | Default |
|----------|-------------|---------|
| `--pdf <path>` | Use local PDF | - |
| `--ocr` | Enable OCR fallback | off |
| `--ocr-lang <lang>` | OCR language | `chi_sim+eng` |
| `--max-pages <n>` | Limit parsed pages | unlimited |
| `--ai` | Enable AI draft generation | off |
| `--model <name>` | LLM model name | `qwen3.5-plus` |
| `--base-url <url>` | API endpoint | DashScope compatible |
| `--api-key <key>` | API key | env `OPENAI_API_KEY` |
| `--tags <t1,t2>` | Comma-separated tags | auto-inferred |

---

## Testing

```bash
PYTHONHOME=/c/Users/adm/AppData/Local/Programs/Python/Python312 \
  .venv/Scripts/python.exe -m pytest tests/ -q
```

**Current**: 372 tests passing, 1 skipped.

---

## Research Philosophy

This system enforces:

- Structured thinking (P/C/M-Notes)
- Explicit reasoning (frontmatter fields)
- Comparison-based insight (M-Notes)
- Long-term tracking (Radar + Timeline)
- Decision logging (evolution logs)
- Cognitive iteration (periodic M-Note revision)

---

## Recommended Workflow

1. Read 1 paper daily
2. Assign 1–3 tags
3. Weekly check Radar
4. Auto-trigger M-Notes when 3+ papers share a tag
5. Quarterly review Timeline
6. Periodically revise M-Notes

---

## Roadmap

- Citation graph extraction
- Auto experiment table parsing
- Embedding-based search
- Knowledge graph building
- Trend prediction
- Research momentum scoring

---

## License

Research & educational use only.

---

## 🇨🇳 AI 研究操作系统

**面向严肃 AI 研究者的结构化研究操作系统**

---

## 一句话说明

输入一篇论文（arXiv URL、DOI 或本地 PDF），输出 **P-Note、C-Note、Radar 条目、Timeline 条目** — 全部结构化、带标签、可交叉引用。可选生成 AI 辅助草稿。

```bash
# 处理单篇论文
python ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM,Agent

# 开启 AI 草稿（需配置 API key）
python ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM --ai
```

这不是一个 **PDF 管理器**，而是一套 **认知升级系统**，强制结构化思维、显式推理和长期研究追踪。

---

## 功能一览

| 输入 | 输出 |
|------|------|
| arXiv URL / ID | P-Note + C-Note + Radar + Timeline |
| DOI | P-Note + C-Note + Radar + Timeline |
| 本地 PDF | P-Note + C-Note + Radar + Timeline |
| 扫描版 PDF | 同上（通过 OCR） |
| `--ai` 标志 | + AI 结构化草稿（待核验） |

---

## 安装

### 依赖

```bash
pip install requests feedparser pymupdf
```

### OCR（可选，扫描版 PDF 需要）

```bash
pip install pytesseract pillow
```

**Windows**: 从 [UB-Mannheim/tesseract](https://github.com/UB-Mannheim/tesseract/wiki) 下载 Tesseract，添加到 PATH，并安装中文语言包（`chi_sim`）。

### AI 草稿（可选）

需要一个 OpenAI 兼容的 API key。详见 [API_CONFIG.md](API_CONFIG.md)。

```bash
# 设置环境变量
export OPENAI_API_KEY="***"
export OPENAI_BASE_URL="https://dashscope.aliyuncs.com/compatible-mode/v1"

# 或直接传入参数（最高优先级）
python ai_research_os.py <input> --api-key "sk-..." --base-url "https://..." --model "qwen3.5-plus"
```

---

## 使用示例

### arXiv 论文

```bash
python ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM,Agent
python ai_research_os.py 2601.00155 --tags LLM
```

### DOI

```bash
python ai_research_os.py 10.48550/arXiv.2601.00155 --tags LLM
```

### 本地 PDF

```bash
python ai_research_os.py test --pdf "paper.pdf" --tags RAG
python ai_research_os.py test --pdf "scanned.pdf" --ocr --ocr-lang chi_sim+eng
```

### 开启 AI 草稿

```bash
python ai_research_os.py https://arxiv.org/abs/2601.00155 --tags LLM --ai
```

---

## 项目结构

```
ai_research_os/
├── core/              # Paper 数据类、常量、today_iso
├── parsers/           # arXiv 获取、Crossref 获取、DOI/arXiv 检测
├── pdf/               # 下载、提取（PyMuPDF + OCR + pdfminer）
├── sections/          # 章节切分 + 格式化
├── llm/               # OpenAI 兼容客户端、AI 草稿生成
├── renderers/         # P-Note、C-Note、M-Note 渲染
├── notes/             # frontmatter、标签推理、笔记聚合
├── updaters/          # Radar 热度追踪、Timeline
├── cli.py             # CLI 入口 + argparse
└── tests/             # 372 个测试
```

---

## 研究树输出

论文按 12 目录结构组织：

```
00-Radar/          # 主题热度追踪
01-Foundations/    # 基础论文
02-Models/         # 模型论文
03-Applications/   # 应用论文
04-Evaluation/     # 评估方法
05-Tools/          # 工具与库
06-Theory/         # 理论论文
07-Architecture/   # 系统架构
08-Training/       # 训练方法
09-Data/           # 数据集
10-Applications/   # 应用研究
11-Future-Directions/
```

---

## 知识演化逻辑

```
论文 → P-Note（论文笔记）
     → C-Note（概念笔记，按标签）
     → M-Note（对比笔记，当同标签论文 ≥3 篇时触发）
     → Radar（主题出现频率热度分）
     → Timeline（按年份的演化）
```

---

## CLI 参数参考

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `--pdf <path>` | 使用本地 PDF | - |
| `--ocr` | 启用 OCR 备选方案 | 关闭 |
| `--ocr-lang <lang>` | OCR 语言 | `chi_sim+eng` |
| `--max-pages <n>` | 限制解析页数 | 无限制 |
| `--ai` | 启用 AI 草稿生成 | 关闭 |
| `--model <name>` | LLM 模型名 | `qwen3.5-plus` |
| `--base-url <url>` | API 端点 | DashScope 兼容 |
| `--api-key <key>` | API key | 环境变量 `OPENAI_API_KEY` |
| `--tags <t1,t2>` | 逗号分隔的标签 | 自动推理 |

---

## 测试

```bash
PYTHONHOME=/c/Users/adm/AppData/Local/Programs/Python/Python312 \
  .venv/Scripts/python.exe -m pytest tests/ -q
```

**当前状态**: 372 测试通过，1 跳过。

---

## 研究理念

本系统强制执行：

- 结构化思维（P/C/M-Notes）
- 显式推理（frontmatter 字段）
- 对比洞察（M-Notes）
- 长期追踪（Radar + Timeline）
- 决策日志（演化记录）
- 认知迭代（定期修订 M-Notes）

---

## 推荐工作流

1. 每天阅读 1 篇论文
2. 赋予 1–3 个标签
3. 每周查看 Radar
4. 当同一标签论文 ≥3 篇时自动触发 M-Notes
5. 每季度回顾 Timeline
6. 定期修订 M-Notes

---

## 路线图

- 引用图提取
- 自动实验表格解析
- 基于 Embedding 的搜索
- 知识图谱构建
- 趋势预测
- 研究势能评分

---

## 许可证

仅供研究与教育使用。
