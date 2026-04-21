"""AI draft generation for P-Notes and C-Notes.

Output format: Markdown sections (human-readable) + XML rubric block (machine-parseable).
Section headings MUST match the P-note template numbering so content can be injected directly.
"""
from typing import Dict, List, Tuple

import ai_research_os as _airo
from core import Paper

# ------------------------------------------------------------------
# Cost estimation helpers
# ------------------------------------------------------------------

# Token price per 1M tokens (approximate, OpenAI-compatible APIs)
_MODEL_PRICES = {
    # model_prefix -> (input_per_1M, output_per_1M)
    "gpt-4o": (2.5, 10.0),
    "gpt-4o-mini": (0.15, 0.6),
    "gpt-4-turbo": (10.0, 30.0),
    "gpt-3.5-turbo": (0.5, 1.5),
    "o1-preview": (15.0, 60.0),
    "o1-mini": (3.0, 12.0),
    "qwen3.5-plus": (0.1, 0.3),
    "qwen3.5": (0.1, 0.3),
    "qwen2.5": (0.1, 0.3),
    "deepseek-chat": (0.14, 0.28),
    "claude-3-5-sonnet": (3.0, 15.0),
    "claude-3-5-haiku": (0.8, 4.0),
    "default": (1.0, 4.0),
}


def estimate_tokens(text: str) -> int:
    """Estimate token count (≈ chars / 4 for Chinese + English mix)."""
    if not text:
        return 0
    return max(1, len(text) // 4)


def get_model_price(model: str) -> Tuple[float, float]:
    """Return (input_per_1M, output_per_1M) for a model."""
    model_lower = model.lower()
    for prefix, price in _MODEL_PRICES.items():
        if prefix in model_lower:
            return price
    return _MODEL_PRICES["default"]


def estimate_cost(model: str, input_text: str, output_text: str) -> Dict[str, float]:
    """Estimate cost in USD for a single LLM call."""
    in_per_1m, out_per_1m = get_model_price(model)
    in_toks = estimate_tokens(input_text)
    out_toks = estimate_tokens(output_text)
    return {
        "input_tokens": in_toks,
        "output_tokens": out_toks,
        "total_tokens": in_toks + out_toks,
        "input_cost_usd": round(in_toks / 1_000_000 * in_per_1m, 6),
        "output_cost_usd": round(out_toks / 1_000_000 * out_per_1m, 6),
        "total_cost_usd": round((in_toks / 1_000_000 * in_per_1m) + (out_toks / 1_000_000 * out_per_1m), 6),
    }


def ai_generate_pnote_draft(
    paper: Paper,
    tags: List[str],
    extracted_text: str,
    base_url: str,
    api_key: str,
    model: str,
    stream: bool = False,
    verbose: bool = False,
) -> str:
    system_prompt = """你是一个严谨的 AI 研究助理，擅长对抗式审稿。

任务：为用户的 Research OS P-Note 生成"可编辑初稿"。

硬规则（违反直接输出 [违规]）：
1. 每项 Claims 必须引用原文（用 > 块引用格式），否则写 "[无原文支撑]"，禁止捏造实验/数据/结果
2. 你的判断/推测必须加 [推测] 标注，不能伪装成事实
3. 输出必须是中文 Markdown
4. 每个栏目开头：> AI Draft（可编辑，需人工核验）
5. 只输出指定栏目，不输出额外解释

评分量表 Rubric（在 # 评分量表 栏目末尾必须嵌入 XML 注释）：

Novelty（原创性）:
  1=增量改进/复现；2=组合已有方法；3=新任务/新视角；4=新范式突破；5=开创性/里程碑

Leverage（杠杆效应）:
  1=难以落地；2=需大量适配；3=可直接应用；4=显著降本/提效；5=范式级影响

Evidence（证据强度）:
  1=无实验/toy实验；2=部分任务；3=充分任务覆盖；4=与强基线对比；5=消融/分析完整

Cost（成本/代价）:
  1=极高；2=较高；3=中等；4=较低；5=极低/可忽略

Moat（护城河）:
  1=无壁垒；2=代码；3=数据；4=算法/专利；5=生态/网络效应

Adoption Signal（采纳信号）:
  1=无采纳；2=GitHub<100 stars；3=GitHub>1k/引用>10；4=工业落地；5=生态标配

评分行格式（机器可解析）：`* Novelty (1-5): 3`
"""

    user_prompt = f"""论文标题：{paper.title}
作者：{", ".join(paper.authors) if paper.authors else "Unknown"}
来源：{paper.source}:{paper.uid}
发布日期：{paper.published or "N/A"}
标签：{", ".join(tags)}

【Abstract】
{paper.abstract or "(空)"}

【抽取正文片段】（已按重要性排序，高优先级章节有更多内容）

{extracted_text}

请按以下栏目生成初稿，## 二级标题必须严格使用给出的编号和名称（内容直接填入对应栏目）：

## 1. 背景
一句话：这篇论文要解决什么问题？（引用摘要）

## 2. 核心问题
这篇论文的核心技术方案是什么？（引用正文片段，用 > 引用；不确定的加 [推测]）

## 3. 方法结构
### 3.1 架构拆解
### 3.2 算法逻辑
### 3.3 关键组件

## 4. 关键创新
一句话总结最大创新点。

## 5. 实验分析
### 5.1 数据集
### 5.2 基线对比
### 5.3 消融实验
### 5.4 成本分析

## 6. 对抗式审稿
列出3个最强质疑点：（1）逻辑/假设漏洞；（2）实验覆盖不足之处；（3）泛化性/复现风险。（加 [推测] 标注）

## 7. 优势
这篇论文的主要优势。（引用参考论文的实验/分析结果支撑）

## 8. 局限
这篇论文的主要局限。（引用参考论文的讨论，加 [推测]）

## 9. 本质抽象
一句话抽象出这篇论文的本质。

## 10. 与其他方法对比
与同类方法的核心差异。

## 11. Decision（决策）
是否值得深入关注？使用场景？

## 12. 知识蒸馏
### Facts
### Principles
### Insights

## 13. 认知升级
长期价值、规模效应、技术护城河、是否范式转移。

## 14. 评分量表
必须包含：Novelty / Leverage / Evidence / Cost / Moat / Adoption Signal
每项格式：`* Novelty (1-5): N`
Overall Judgment：一句话总结

（严禁捏造实验数据；引用格式："> 原文片段"）

在以上 Markdown 内容之后，单独输出以下格式的 JSON（不要放在 XML 注释中，不要加.md后缀）：

```json
{{"novelty": 3, "leverage": 4, "evidence": 3, "cost": 2, "moat": 2, "adoption": 3, "overall": "一句话评价"}}
```
"""

    return _airo.call_llm_chat_completions(
        base_url=base_url,
        api_key=api_key,
        model=model,
        system_prompt=system_prompt,
        user_prompt=user_prompt,
        stream=stream,
    )


# =============================================================================
# C-Note AI draft generation
# =============================================================================
_CNOTE_SYSTEM_PROMPT = """你是一个严谨的 AI 研究助理，擅长概念分析和知识图谱构建。

任务：为用户的 Research OS C-Note（概念笔记）生成"可编辑初稿"。

硬规则（违反直接输出 [违规]）：
1. 每项 Claims 必须引用原文（用 > 块引用格式），否则写 "[无原文支撑]"，禁止捏造
2. 你的判断/推测必须加 [推测] 标注，不能伪装成事实
3. 输出必须是中文 Markdown
4. 每个栏目开头：> AI Draft（可编辑，需人工核验）
5. 只输出指定栏目，不输出额外解释
"""


def ai_generate_cnote_draft(
    concept: str,
    pnotes: List[dict],
    api_key: str,
    base_url: str,
    model: str,
    call_llm=None,
) -> str:
    """
    Generate a C-Note draft for a concept using referenced P-Notes as context.

    Args:
        concept: The concept name (e.g. "RAG", "Agent")
        pnotes: List of dicts with keys: title, abstract, authors, year, source, tags
        api_key: LLM API key
        base_url: OpenAI-compatible base URL
        model: Model name
        call_llm: Callable to use instead of the default call_llm_chat_completions.
                  Allows dependency injection for testing.
    """
    if call_llm is None:
        call_llm = _airo.call_llm_chat_completions

    pnotes_chunks = [
        f"""\
---
论文 {i}：
标题：{p.get('title', 'N/A')}
作者：{', '.join(p.get('authors', [])) or 'Unknown'}
年份：{p.get('year', 'N/A')}
来源：{p.get('source', 'N/A')}:{p.get('uid', 'N/A')}
标签：{', '.join(p.get('tags', []))}
摘要：{p.get('abstract', '(无)') or '(无)'}
"""
        for i, p in enumerate(pnotes, 1)
    ]
    pnotes_text = "\n".join(pnotes_chunks)

    user_prompt = f"""\
概念：{concept}

参考论文（共 {len(pnotes)} 篇）：
{pnotes_text}

请按以下栏目生成 C-Note 初稿，每栏用 ## 二级标题：

## 核心定义
一句话定义这个概念。（引用参考论文中的定义，没有原话则综合推断并加 [推测]）

## 产生背景
这个概念是在什么研究背景下产生的？解决了什么问题？（引用参考论文，没有则 [推测]）

## 技术本质
这个概念的核心技术机制是什么？（引用参考论文，加 [推测] 如需推断）

## 常见实现路径
列出该概念的典型实现方式。（引用参考论文中的实现，加 [推测]）

## 优势
这个概念的主要优势。（引用参考论文的实验/分析结果支撑）

## 局限
这个概念的主要局限。（引用参考论文的讨论，加 [推测]）

## 与其他思想的关系
与其他相关概念的关系和区别。（综合多篇参考论文，加 [推测]）

## 代表论文
从参考论文中选取最能代表该概念的论文，给出选择理由。

## 演化时间线
基于参考论文，推断该概念的演化路径。（加 [推测]）

## 未来趋势
基于参考论文的讨论，预测该概念的未来发展方向。（加 [推测]）

（严禁捏造论文数据；引用格式："> 原文片段"）
"""

    return call_llm(
        base_url=base_url,
        api_key=api_key,
        model=model,
        system_prompt=_CNOTE_SYSTEM_PROMPT,
        user_prompt=user_prompt,
    )
