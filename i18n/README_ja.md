# AI研究OS

<div align="center">
  <img src="logo_hero.png" width="800" alt="AI Research OS Demo"/>
</div>

**AI研究者向け自己進化型研究オペレーティングシステム**

[![Build](https://github.com/shushuzn/Rairos/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/shushuzn/Rairos/actions)
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

## 概要

AI研究OSは、あなたの使用パターンから学習する**自己進化型研究システム**です。論文管理器ではなく、時間とともに賢くなる研究パートナーです。

論文（arXiv URL、DOI、PDF）を投入すると、**P-Note**、**C-Note**、**Radarエントリ**、**Timelineエントリ**が得られます——すべて構造化され、タグ付けされ、相互リンクされています。

| 入力 | 出力 |
|---|---|
| arXiv URL/ID | P-Note + C-Note + Radar + Timeline |
| DOI | P-Note + C-Note + Radar + Timeline |
| ローカルPDF | P-Note + C-Note + Radar + Timeline |
| スキャンPDF | 同一（OCR経由） |

これは**PDF管理器ではありません**。**自己進化型システム**です：
- 研究パターンから学習
-  시간이経つにつれ回答を改善
- 特定のドメインに適応

## コア機能

| 機能 | 説明 |
|---------|-------------|
| `airos import` | arXiv、DOI、PDFから論文をインポート |
| `airos chat` | RAG対応のQ&A |
| `airos slides` | プレゼンテーション自動生成 |
| `airos kg` | ナレッジグラフ可視化 |
| Evolution | Gene/Capsuleパターンによる自己改善 |

## クイックスタート

```bash
pip install ai-research-os
airos-cli 2601.00155 --tags LLM,Agent
```

完了——数秒で論文をインポート。

### 1行、3つの入力

```bash
airos-cli 2601.00155                          # arXiv ID
airos-cli 10.48550/arXiv.2601.00155           # DOI
airos-cli --pdf paper.pdf --tags RAG            # ローカルPDF
airos-cli --pdf scanned.pdf --ocr --ocr-lang chi_sim+eng   # スキャンPDF
```

### 3つのコアコマンド

```bash
airos-cli import 2601.00155 10.1038/nature12373   # 論文をDBに追加
airos-cli search "attention mechanism" --tag LLM    # 論文を検索
airos-cli research "RLHF alignment" --limit 5       # 自主研究ループ
```

### AI下書き（オプション）

```bash
export OPENAI_API_KEY="***"
export OPENAI_BASE_URL="https://dashscope.aliyuncs.com/compatible-mode/v1"
airos-cli 2601.00155 --tags LLM --ai
```

完全な設定は [API_CONFIG.md](API_CONFIG.md) を参照。

## 研究ツリー

論文は12のディレクトリに整理されています：

```
00-Radar/            トピックヒート追跡
01-Foundations/      基礎論文
02-Models/           モデル論文
03-Training/         訓練方法
04-Scaling/         スケーリング法則
05-Alignment/        アライメント研究
06-Agents/           エージェントシステム
07-Infrastructure/   インフラ
08-Optimization/    最適化技術
09-Evaluation/       評価方法
10-Applications/    応用研究
11-Future-Directions/
```

## インストール

```bash
pip install ai-research-os
```

ソースからインストール：

```bash
git clone https://github.com/shushuzn/Rairos.git
cd Rairos
CARGO_BUILD_JOBS=1 cargo build --workspace
```

## ドキュメント

完全なドキュメントは [ai-research-os.readthedocs.io](https://ai-research-os.readthedocs.io/) を参照。

| ドキュメント | 説明 |
|-----|-------------|
| [Architecture](docs/architecture.md) | システム设计与モジュール概要 |
| [Configuration](docs/configuration.md) | LLM、DB、検索、ツール設定 |
| [Benchmarks](docs/benchmarks.md) | パフォーマンス指標とテストカバレッジ |
| [Contributing](CONTRIBUTING.md) | このプロジェクトへの貢献方法 |
| [Roadmap](ROADMAP.md) | プロジェクトロードマップと今後の計画 |

## ライセンス

GPL-3.0-or-later。詳細については [LICENSE](LICENSE) を参照してください。
