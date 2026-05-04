# AI 研究操作系统 (Rairos)

<div align="center">
  <img src="logo_hero.svg" width="900" alt="Rairos Demo"/>
</div>

**一个自进化的研究操作系统 — 从你的反馈中学习，自动发现更好的研究方向。**

[![Python](https://img.shields.io/badge/Python-3.9%2B-blue)](https://python.org)
[![PyPI Version](https://img.shields.io/pypi/v/rairos)](https://pypi.org/project/rairos/)
[![Tests](https://github.com/shushuzn/Rairos/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/shushuzn/Rairos/actions)
[![License](https://img.shields.io/badge/License-GPL--3.0--or--later-orange)](#license)

## 它能做什么

Rairos 是一个**自主研究助手**，可以：

- **阅读论文** — arXiv、DOI、本地 PDF、扫描版（OCR）
- **发现空白** — 在 36 个 AI 主题中识别研究机会
- **学习你的偏好** — Gene Pool 编码你感兴趣的模式
- **自动进化** — 后台 daemon 监控 arXiv、分析文献、进化知识库
- **本地运行** — 支持 Ollama，零 API 费用，完全私密

```
输入一篇论文 → 系统学习什么有效 → 下一次搜索更精准
```

## 快速开始

### 选项 1: pip 安装

```bash
pip install rairos
rairos import 2604.28192 --tags LLM,Agent
```

### 选项 2: Docker（含 Ollama 免费本地 LLM）

```bash
git clone https://github.com/shushuzn/Rairos.git
cd Rairos
docker compose up --build
# 打开 http://localhost:8501
```

### 选项 3: 源码安装

```bash
git clone https://github.com/shushuzn/Rairos.git
cd Rairos
pip install -e ".[all]"
rairos --help
```

## 核心命令

```bash
rairos import 2601.00155                    # 导入论文
rairos gap "reinforcement learning"          # 检测研究空白
rairos research "RLHF alignment"             # 自主研究循环
rairos daemon start                          # 启动后台自动运行
rairos daemon status                         # 查看状态
rairos daemon evolve                         # 手动执行进化循环
rairos agent deep-research "topic"           # 深度研究代理
```

### 使用 Ollama（免费本地运行）

```bash
ollama pull qwen2.5
rairos gap "transformer efficiency" --model ollama/qwen2.5

# 或设为默认模型
export AIROS_DEFAULT_MODEL_CLI=ollama/qwen2.5
```

### Web 界面

```bash
uvicorn web.app:app --port 8501
# 打开 http://localhost:8501
```

或使用 Docker: `docker compose up` → http://localhost:8501

## 核心功能

| 功能 | 命令 | Web UI |
|------|------|--------|
| 论文导入 (arXiv/DOI/PDF) | `rairos import` | 导入页面 |
| 研究空白检测 | `rairos gap` | 空白分析 |
| 深度研究代理 | `rairos agent deep-research` | Research Loop |
| Gene Pool 进化 | `rairos daemon evolve` | Evolution Log |
| 可信度评分 | `rairos daemon status` | Credibility 页面 |
| 来源信任分 | `rairos daemon status` | Trust Scores |
| Paper2Code 流水线 | `rairos paper2code` | Paper2Code 页面 |
| 引用链分析 | `rairos citation-chain` | Citation Chain |
| 后台自动运行 | `rairos daemon start` | Daemon dashboard |
| 聊天 (TUI) | `rairos chat-tui` | Chat 页面 |
| 洞察卡片 | `rairos insight` | Insights 页面 |
| 自主订阅 | `rairos subscribe add` | arXiv Channels |

## 系统架构

```
arXiv 论文 → GapAnalyzerV2 → Gene Pool (CapsuleGene 编码)
                                    ↑
DeepResearch Agent ← GenePoolGuide ← 偏好画像
         ↑
    搜索 → 提取 → 分析 → 反思 → 编码

后台 daemon:
    订阅监控 → 空白分析 → 进化循环 → 可信度评分
```

Gene Pool 使用 SQLite (WAL 模式) 存储，带索引加速查询。
可信度评分检测 trendslop 胶囊（关键词重叠 > 70% 自动标记）。
来源信任分按 arXiv 分类跟踪胶囊质量历史。

## 安装依赖

```bash
pip install rairos
# 或完整安装（含 OCR、AI、幻灯片）
pip install "rairos[all]"
```

### OCR（可选，扫描版 PDF 需要）

```bash
pip install pytesseract pillow
```

**Windows**: 从 [UB-Mannheim/tesseract](https://github.com/UB-Mannheim/tesseract/wiki) 下载 Tesseract。

## 项目结构

```
core/              # 核心数据类、重试、缓存、异常
parsers/           # arXiv/Crossref 获取、DOI 检测
pdf/               # PDF 下载、文本提取、OCR
sections/          # 章节切分
llm/               # LLM 客户端 (OpenAI/Ollama/Anthropic)、Gap 分析、进化引擎
  insight/         # Gene Pool、可信度、信任分、进化闭环
research_loop/     # 自主研究循环、深度研究代理、orchestrator
cli/               # CLI 入口 + 30+ 子命令
web/               # FastAPI Web 界面
tests/             # 3874+ 测试
```

## 测试

```bash
# 运行全部测试
python -B -m pytest tests/ -q

# 运行特定测试
python -B -m pytest tests/test_credibility.py -v
python -B -m pytest tests/test_gene_pool_evolution.py -v
```

## 文档

| 文档 | 说明 |
|------|------|
| [English README](README.md) | English documentation |
| [Architecture](docs/architecture.md) | 系统设计 |
| [Configuration](docs/configuration.md) | LLM、数据库配置 |
| [Roadmap](ROADMAP.md) | 项目路线图 |
| [Contributing](CONTRIBUTING.md) | 贡献指南 |

## 许可证

GPL-3.0-or-later。详见 [LICENSE](LICENSE)。
