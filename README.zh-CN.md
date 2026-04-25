# AI 研究操作系统

**面向严肃 AI 研究者的结构化研究操作系统**

---

## 一句话说明

输入一篇论文（arXiv URL、DOI 或本地 PDF），输出 **P-Note、C-Note、Radar 条目、Timeline 条目** — 全部结构化、带标签、可交叉引用。可选生成 AI 辅助草稿。

```bash
# 处理单篇论文
python -m cli https://arxiv.org/abs/2601.00155 --tags LLM,Agent

# 开启 AI 草稿（需配置 API key）
python -m cli https://arxiv.org/abs/2601.00155 --tags LLM --ai
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
python -m cli <input> --api-key "sk-..." --base-url "https://..." --model "qwen3.5-plus"
```

---

## 快速上手

### arXiv 论文

```bash
python -m cli https://arxiv.org/abs/2601.00155 --tags LLM,Agent
python -m cli 2601.00155 --tags LLM
```

### DOI

```bash
python -m cli 10.48550/arXiv.2601.00155 --tags LLM
```

### 本地 PDF

```bash
python -m cli test --pdf "paper.pdf" --tags RAG
python -m cli test --pdf "scanned.pdf" --ocr --ocr-lang chi_sim+eng
```

### 开启 AI 草稿

```bash
python -m cli https://arxiv.org/abs/2601.00155 --tags LLM --ai
```

---

## 项目结构

```
ai_research_os/
├── core/              # Paper 数据类、重试、缓存、异常
├── parsers/           # arXiv 获取、Crossref 获取、DOI/arXiv 检测
├── pdf/               # 下载、提取（PyMuPDF + OCR + pdfminer）
├── sections/          # 章节切分 + 格式化
├── llm/               # OpenAI 兼容客户端、AI 草稿生成
├── renderers/         # P-Note、C-Note、M-Note 渲染
├── notes/             # frontmatter、标签推理、笔记聚合
├── updaters/          # Radar 热度追踪、Timeline
├── cli/               # CLI 入口 + 23 个子命令
└── tests/             # 1034 个测试
```

---

## 研究树输出

论文按 12 目录结构组织：

```
00-Radar/          # 主题热度追踪
01-Foundations/    # 基础论文
02-Models/         # 模型论文
03-Training/       # 训练方法
04-Scaling/        # Scaling Laws
05-Alignment/      # 对齐研究
06-Agents/         # Agent 系统
07-Infrastructure/ # 基础设施
08-Optimization/   # 优化技术
09-Evaluation/     # 评估方法
10-Applications/  # 应用研究
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

## 测试

```bash
python -B -m pytest tests/ -q
```

**当前状态**: 1034 测试通过，1 跳过。

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

## 许可证

仅供研究与教育使用。

---

完整命令参考见 [ADVANCED_COMMANDS.md](ADVANCED_COMMANDS.md)。
英文文档见 [README.md](README.md)。
