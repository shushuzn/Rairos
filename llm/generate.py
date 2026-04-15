"""AI draft generation for P-Notes."""
from typing import List

from core import Paper


def ai_generate_pnote_draft(
    paper: Paper,
    tags: List[str],
    extracted_text: str,
    base_url: str,
    api_key: str,
    model: str,
) -> str:
    # Lazy import so test can monkeypatch ai_research_os.call_llm_chat_completions
    import ai_research_os as airo
    call_llm_chat_completions = airo.call_llm_chat_completions

    system_prompt = """你是严谨的研究助理 + 对抗式审稿人。
目标：基于给定论文信息与抽取的正文片段，为用户的 Research OS P-Note 生成"可编辑初稿"。
硬性要求：
- 明确区分：事实（可在文中/摘要中找到） vs 推断（你的判断）
- 不要编造不存在的实验/数据集/结果；若不确定，写"未在当前片段中找到"
- 输出必须是中文、Markdown
- 每个栏目开头必须加：> AI Draft（可编辑，需人工核验）
- 只输出指定栏目，不要输出额外解释
"""

    user_prompt = f"""论文标题：{paper.title}
作者：{", ".join(paper.authors) if paper.authors else "Unknown"}
来源：{paper.source}:{paper.uid}
发布日期：{paper.published or "N/A"}
标签：{", ".join(tags)}

【Abstract】
{paper.abstract or "(空)"}

【抽取正文片段（可能不完整）】
{extracted_text}

现在请按以下栏目生成初稿（每栏用 '## ' 二级标题输出，标题必须严格匹配）：

## 1. 背景
## 2. 核心问题
## 3. 方法结构
- 需要包含：架构拆解 / 算法逻辑 / 关键组件（用小标题或列表）
## 4. 关键创新
## 5. 实验分析
- 需要包含：数据集 / 基线对比 / 消融实验 / 成本分析（找不到就标注未找到）
## 6. 对抗式审稿
- 需要包含：逻辑漏洞 / 偏置风险 / 复现难度 / 失败模式推测
## 7. 优势
## 8. 局限
## 9. 本质抽象
## 10. 与其他方法对比
- 给出 vs A / vs B / vs C 的对比建议（A/B/C 用你认为合理的同类方法名；不确定就写"待定"）
## 11. Decision（决策）
- 是否使用/使用场景/不适用边界/接下来关注信号
## 知识蒸馏
- Facts/Principles/Insights
## 认知升级
- 长期价值/规模效应/技术护城河/是否范式转移/商业潜力
## 评分量表
- Novelty/Leverage/Evidence/Cost/Moat/Adoption Signal + Overall Judgment
"""

    return call_llm_chat_completions(
        base_url=base_url,
        api_key=api_key,
        model=model,
        system_prompt=system_prompt,
        user_prompt=user_prompt,
    )
