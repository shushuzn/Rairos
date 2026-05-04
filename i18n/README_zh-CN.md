# AI研究操作系统

<div align="center">
  <img src="logo_hero.png" width="800" alt="AI Research OS Demo"/>
</div>

**面向AI研究者的自进化研究操作系统**

[![Python](https://img.shields.io/badge/Python-3.9%2B-blue)](https://python.org)
[![PyPI Version](https://img.shields.io/pypi/v/ai-research-os)](https://pypi.org/project/ai-research-os/)
[![Coverage](https://img.shields.io/codecov/c/github/shushuzn/Rairos/main?logo=codecov)](https://app.codecov.io/gh/shushuzn/Rairos)
[![Tests](https://github.com/shushuzn/Rairos/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/shushuzn/Rairos/actions)
[![License](https://img.shields.io/badge/License-GPL--3.0--or--later-orange)](#license)

<div align="center">
<a href="https://www.star-history.com/#shushuzn/Rairos&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=shushuzn/Rairos&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=shushuzn/Rairos&type=Date" />
   <img alt="AI Research OS Star History" src="https://api.star-history.com/svg?repos=shushuzn/Rairos&type=Date" style="width: 80%; height: auto;" />
 </picture>
</a>
</div>

## 功能简介

AI研究操作系统是一个**自进化研究系统**，能从你的使用模式中学习。它不仅仅是论文管理器——更是一个会随时间变得更聪明研究伙伴。

输入一篇论文（arXiv链接、DOI或PDF），获得**P-Note**、**C-Note**、**Radar条目**和**Timeline条目**——全部结构化、标签化、互相关联。

| 输入 | 输出 |
|---|---|
| arXiv链接/ID | P-Note + C-Note + Radar + Timeline |
| DOI | P-Note + C-Note + Radar + Timeline |
| 本地PDF | P-Note + C-Note + Radar + Timeline |
| 扫描件PDF | 相同（通过OCR） |

这不是一个**PDF管理器**。它是一个**自进化系统**，能够：
- 从你的研究模式中学习
- 随着时间改进答案
- 适应你的特定领域

## 核心功能

| 功能 | 描述 |
|---------|-------------|
| `airos import` | 从arXiv、DOI、PDF导入论文 |
| `airos chat` | 基于RAG的论文问答 |
| `airos slides` | 自动生成演示文稿 |
| `airos kg` | 知识图谱可视化 |
| Evolution | 通过Gene/Capsule模式自改进 |

## 快速开始

```bash
pip install ai-research-os
airos-cli 2601.00155 --tags LLM,Agent
```

一行命令，几秒内完成论文导入。以上命令安装包并导入一篇arXiv论文。

### 一行命令，三种输入

```bash
airos-cli 2601.00155                          # arXiv ID
airos-cli 10.48550/arXiv.2601.00155           # DOI
airos-cli --pdf paper.pdf --tags RAG            # 本地PDF
airos-cli --pdf scanned.pdf --ocr --ocr-lang chi_sim+eng   # 扫描PDF
```

### 三个核心命令

```bash
airos-cli import 2601.00155 10.1038/nature12373   # 添加论文到数据库
airos-cli search "attention mechanism" --tag LLM    # 搜索论文
airos-cli research "RLHF alignment" --limit 5       # 自主研究循环
```

### AI草稿（可选）

```bash
export OPENAI_API_KEY="***"
export OPENAI_BASE_URL="https://dashscope.aliyuncs.com/compatible-mode/v1"
airos-cli 2601.00155 --tags LLM --ai
```

完整配置参见 [API_CONFIG.md](API_CONFIG.md)。

## 研究目录树

论文被组织成12个目录：

```
00-Radar/            主题热度追踪
01-Foundations/      基础论文
02-Models/           模型论文
03-Training/         训练方法
04-Scaling/          扩展定律
05-Alignment/        对齐研究
06-Agents/           智能体系统
07-Infrastructure/   基础设施
08-Optimization/      优化技术
09-Evaluation/        评估方法
10-Applications/     应用研究
11-Future-Directions/
```

## 安装

```bash
pip install ai-research-os
```

或从源码安装：

```bash
git clone https://github.com/shushuzn/Rairos.git
cd ai_research_os
pip install -e .
```

## 文档

完整文档见 [ai-research-os.readthedocs.io](https://ai-research-os.readthedocs.io/)。

| 文档 | 描述 |
|-----|-------------|
| [Architecture](docs/architecture.md) | 系统设计与模块概述 |
| [Configuration](docs/configuration.md) | LLM、数据库、搜索、工具配置 |
| [Benchmarks](docs/benchmarks.md) | 性能指标与测试覆盖率 |
| [Contributing](CONTRIBUTING.md) | 如何为本项目做贡献 |
| [Roadmap](ROADMAP.md) | 项目路线图与未来计划 |

## 许可证

GPL-3.0-or-later。详见 [LICENSE](LICENSE)。
