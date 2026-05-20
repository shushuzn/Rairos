# AI 研究操作系统 (Rairos)

<div align="center">
  <img src="logo_hero.svg" width="900" alt="Rairos Demo"/>
</div>

**一个自进化的研究操作系统 — 从你的反馈中学习，自动发现更好的研究方向。** — 100% Rust (154 crates)

[![Build](https://github.com/shushuzn/Rairos/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/shushuzn/Rairos/actions)
![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg?logo=rust)
![Crates](https://img.shields.io/badge/crates-154-blue.svg)
![Lines](https://img.shields.io/badge/lines-116k%2B-green.svg)
![MCP](https://img.shields.io/badge/mcp_tools-69-blue.svg?logo=robot)
![CLI](https://img.shields.io/badge/cli_commands-105-blue.svg?logo=terminal)
[![License](https://img.shields.io/badge/License-GPL--3.0--or--later-orange)](#license)
[![Stars](https://img.shields.io/github/stars/shushuzn/Rairos?style=social)](https://github.com/shushuzn/Rairos/stargazers)
[![Forks](https://img.shields.io/github/forks/shushuzn/Rairos?style=social)](https://github.com/shushuzn/Rairos/network/members)
[![Downloads](https://img.shields.io/github/downloads/shushuzn/Rairos/total?style=social)](https://github.com/shushuzn/Rairos/releases)

[核心功能](#核心功能) •
[快速开始](#快速开始) •
[安装](#安装) •
[集成](#集成) •
[Shell 补全](#shell-补全) •
[文档](#文档) •
[故障排查](#常见问题排查)
![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg?logo=rust)
![Crates](https://img.shields.io/badge/crates-154-blue.svg)
![Lines](https://img.shields.io/badge/lines-116k%2B-green.svg)
![MCP](https://img.shields.io/badge/mcp_tools-69-blue.svg?logo=robot)
![CLI](https://img.shields.io/badge/cli_commands-105-blue.svg?logo=terminal)
[![License](https://img.shields.io/badge/License-GPL--3.0--or--later-orange)](#license)
[![Stars](https://img.shields.io/github/stars/shushuzn/Rairos?style=social)](https://github.com/shushuzn/Rairos/stargazers)
[![Forks](https://img.shields.io/github/forks/shushuzn/Rairos?style=social)](https://github.com/shushuzn/Rairos/network/members)
[![Downloads](https://img.shields.io/github/downloads/shushuzn/Rairos/total?style=social)](https://github.com/shushuzn/Rairos/releases)

## 为什么选择 Rairos？

| 功能 | Zotero | Mendeley | **Rairos** |
|------|--------|----------|-------------|
| PDF 存储 | ✅ | ✅ | ✅ |
| 文献管理 | ✅ | ✅ | ✅ |
| 研究空白检测 | ❌ | ❌ | ✅ |
| 自进化基因池 | ❌ | ❌ | ✅ |
| 本地 LLM 支持 | ❌ | ❌ | ✅ |
| AI Agent MCP 工具 | ❌ | ❌ | ✅ |
| 105 个 CLI 命令 | ❌ | ❌ | ✅ |

Rairos 是**第一个会随你进化的研究工具**——它学习你发现有价值的内容，自动改进未来搜索。

### 为什么要用 Rairos？

- **不只是 PDF 管理器** — Rairos 主动发现研究空白，而非只是存储论文
- **自进化** — Gene Pool 从你的反馈中学习，搜索效果越来越好
- **完全本地** — 支持 Ollama 离线运行，零 API 费用，完全私密
- **AI Agent 原生** — 内置 69 个 MCP 工具，支持 AI 助手集成
- **Rust 驱动** — 快速启动（<50ms），低内存（~10MB），单二进制
- **CLI 优先** — 可脚本化、可组合、键盘驱动的工作流
- **开源** — 无供应商锁定，代码完全透明

### 为什么不使用 Rairos？

- **需要云同步** — 试试 Zotero 或 Mendeley 的跨设备同步
- **需要协作功能** — 暂无团队共享设计
- **需要移动端** — 目前仅支持 CLI/Web

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

## 性能

100% Rust 编写，追求极致性能：

| 指标 | 数值 | 对比 |
|------|------|------|
| 启动时间 | **< 50ms** | Python: 500ms+, Node.js: 80ms+ |
| 内存占用 | **~10 MB** | Python: 100MB+, Node.js: 190MB+ |
| 二进制大小 | **单文件 ~15MB** | Python/Node: node_modules 200MB+ |
| 搜索延迟 | **12ms** | FTS5 关键词搜索 |
| 并行处理 | **Rayon** | 充分利用多核 |

运行 `./rairos.sh benchmark` 查看完整性能数据。

## 集成

Rairos 可以与其他工具配合使用：

### 与 fzf（模糊搜索）
```bash
# 用 fzf 查找论文
./rairos.sh list --format json | fzf --preview './rairos.sh show {1}'
```

### 与 git（自动提交论文）
```bash
# 将论文变更添加到 git
git add papers/ && git commit -m "Update papers"
```

### 与 xargs（批量处理）
```bash
# 批量处理多篇论文
./rairos.sh list --status pending | xargs -I {} ./rairos.sh parse {}
```

### 与 jq（JSON 处理）
```bash
# 查询 JSON 格式论文
./rairos.sh search "transformer" --format json | jq '.[] | select(.citations > 100)'
```

### 与 curl（API 网关）
```bash
# 查询 API 网关
curl -X POST http://localhost:8081/api/search -d '{"query": "LLM"}'
```

### 与 watch（监控）
```bash
# 每小时监控新论文
watch -n 3600 './rairos.sh subscribe list'
```

## 快速开始

### 使用 Makefile（推荐）

```bash
git clone https://github.com/shushuzn/Rairos.git
cd Rairos
make build                    # 构建（首次 10-20 分钟）
./rairos.sh --help           # 查看所有命令
```

### 初始化和运行

```bash
./rairos.sh init             # 初始化数据库
./rairos.sh list             # 查看论文
./rairos.sh search "LLM"     # 搜索论文
```

### 开发构建

```bash
make build-dev    # Debug 构建（更快）
make test         # 运行测试
make clippy       # 运行 linter
```

## 安装

### 从源码构建

```bash
git clone https://github.com/shushuzn/Rairos.git
cd Rairos
make build
```

### 通过 cargo install 安装

```bash
# 从 crates.io 安装（发布后可用）
cargo install rairos

# 或从源码安装
cargo install --path crates/rairos-cli
```

### 通过 cargo-binstall 安装（快速，无需编译）

```bash
# 先安装 cargo-binstall: https://github.com/cargo-bins/cargo-binstall
cargo binstall rairos-cli
```

### 预编译二进制文件

从 [最新发布版本](https://github.com/shushuzn/Rairos/releases/latest) 下载：

| 平台 | 下载 |
|------|------|
| Linux x86_64 | `rairos-cli-x86_64-unknown-linux-musl.tar.gz` |
| macOS Apple Silicon | `rairos-cli-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `rairos-cli-x86_64-apple-darwin.tar.gz` |
| Windows x86_64 | `rairos-cli-x86_64-pc-windows-msvc.zip` |

```bash
# 解压并安装
tar -xzf rairos-cli-*.tar.gz
sudo mv rairos /usr/local/bin/
```

### 通过包管理器安装

```bash
# Homebrew (macOS/Linux)
brew install rairos

# Arch Linux (AUR)
paru -S rairos-cli

# Nix/NixOS
nix-env -iA nixpkgs.rairos

# Guix
guix install rairos

# FreeBSD
pkg install rairos
```

## Rust 技术栈 (154 crates)

| Crate | 用途 |
|-------|------|
| rairos-core | 核心数据库、FTS5、订阅管理 |
| rairos-cli | CLI 入口 (105 命令) |
| rairos-mcp | MCP 协议服务器 (69 工具) |
| rairos-llm | LLM 客户端、Gene Pool、进化引擎 |
| rairos-parser | arXiv/CrossRef/Semantic Scholar API |
| rairos-research | 深度研究 Agent、空白检测 |
| rairos-web | REST API + HTML 前端 |
| rairos-kg | 知识图谱、PageRank、社区发现 |

完整列表见 [AGENTS.md](AGENTS.md)。

## 架构

```
CLI (rairos-cli) → crates/* 全 Rust
```

所有数据存储在 `~/.ai_research_os/`，完全离线运行。

## 核心功能

| 功能 | 描述 |
|------|------|
| 🔍 **论文导入** | arXiv、DOI、PDF、OCR 支持 |
| 🧬 **Gene Pool** | 自进化研究模式 |
| 🎯 **空白检测** | 36 个 AI 研究主题 |
| 💬 **RAG 问答** | 与论文对话 |
| 📊 **知识图谱** | 可视化论文关系 |
| 🤖 **69 个 MCP 工具** | AI Agent 集成 |
| 🖥️ **105 个 CLI 命令** | 全功能终端界面 |

## 生态

核心 crates：

| Crate | 描述 |
|-------|------|
| rairos-core | 数据库、FTS5 搜索、订阅 |
| rairos-llm | GenePool 进化、LLM 客户端 |
| rairos-cli | 105 CLI 命令 |
| rairos-mcp | 69 MCP 工具 (JSON-RPC 2.0) |
| rairos-kg | 知识图谱、PageRank |
| rairos-research | DeepResearchAgent、空白检测 |

## 常见问题

### Rairos 和 Zotero/Mendeley 有什么区别？

Rairos 不是 PDF 管理器——它是**自进化研究伙伴**，从你的研究模式中学习并随时间改进。它专注于发现研究空白和生成洞察，而不仅仅是存储论文。

### Rairos 需要联网吗？

Rairos 可以**完全离线**运行，支持本地 LLM (Ollama)。云功能（OpenAI、DashScope）是可选的。

### 最低 Rust 版本要求是什么？

需要 **Rust 1.85+**。Rairos 使用现代 Rust 特性以保证性能和安全性。

### Gene Pool 是如何工作的？

Gene Pool 将成功的研究模式编码为"基因"，随时间进化。当你标记论文有用时，系统会学习什么对你重要，并在未来的搜索中优先考虑类似发现。

### 我可以编程方式使用 Rairos 吗？

可以！Rairos 提供：
- **CLI**: 通过 `./rairos.sh` 使用 105 个命令
- **MCP**: 69 个 AI agent 集成工具
- **REST API**: 内置 web 服务器，带 OpenAPI 文档
- **SDK**: Python (`pip install rairos`) 和 Node.js (`npm install rairos`)

## Shell 补全

启用 Tab 补全加快命令输入：

### Bash
```bash
# 生成补全脚本
./rairos.sh completions bash > ~/.local/share/bash-completion/completions/rairos
source ~/.bashrc
```

### Zsh
```bash
# 生成补全脚本
./rairos.sh completions zsh > ~/.zfunc/_rairos
autoload -Uz compinit && compinit
```

### Fish
```bash
# 生成补全脚本
./rairos.sh completions fish > ~/.config/fish/completions/rairos.fish
```

### PowerShell
```powershell
# 生成补全脚本
./rairos.sh completions powershell >> $PROFILE
```

或使用 `make completions` 一次生成所有 shell 的补全：
```bash
make completions
```

## 常见问题排查

### 构建失败 "memory allocation failed"

```bash
# 减少并行构建
unset RUSTC_WRAPPER && CARGO_BUILD_JOBS=1 cargo build

# 或使用 make
make build CARGO_BUILD_JOBS=1
```

### 数据库锁定错误

```bash
# 确保只有一个 Rairos 进程在运行
pkill rairos  # 关闭所有实例
./rairos.sh init  # 如需要重新初始化
```

### Ollama 连接失败

```bash
# 检查 Ollama 是否运行
ollama list

# 下载模型（如需要）
ollama pull qwen2.5

# 设置显式 URL
export OLLAMA_BASE_URL=http://localhost:11434
```

### arXiv 论文找不到

某些论文需要认证或不在 arXiv 上。尝试：
- DOI 导入：`./rairos.sh add --doi 10.xxxx/xxxxx`
- 直接 PDF：`./rairos.sh import /path/to/paper.pdf`

### Rust 版本不匹配

```bash
# Rairos 需要 Rust 1.85+
rustc --version
rustup update  # 更新到最新稳定版
```

### 需要帮助？

- 运行 `./rairos.sh --help` 查看所有命令
- 运行 `./rairos.sh <command> --help` 查看具体命令帮助
- 查看 [docs/](docs/) 了解详细文档
- 在 GitHub 上提交 issue 报告 bug 或请求功能

## 文档

- [全命令列表](AGENTS.md)
- [架构文档](docs/architecture.md)
- [安装指南](docs/installation.md)
- [使用示例](USAGE_EXAMPLES.md)

## 类似项目

寻找替代品？与以下研究工具对比：

| 工具 | 类型 | 特点 |
|------|------|------|
| [Zotero](https://www.zotero.org/) | 文献管理 | PDF 存储、引用管理 |
| [Mendeley](https://www.mendeley.com/) | 文献管理 | PDF 标注、云同步 |
| [Semantic Scholar](https://www.semanticscholar.org/) | 搜索引擎 | AI 驱动的论文发现 |
| [Consensus](https://consensus.app/) | 搜索引擎 | 论文问答 |
| [Elicit](https://elicit.org/) | 研究助手 | AI 分析 |

**Rairos 的优势**：**自托管** + **自进化** Gene Pool——从你的反馈中学习，自动改进未来搜索。

## 许可证

GPL-3.0-or-later
