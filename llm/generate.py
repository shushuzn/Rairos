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

    system_prompt = """你是一个严谨的 AI 研究助理，擅长对抗式审稿。

目标：为用户的 Research OS P-Note 生成"可编辑初稿"。

核心原则：
1. 严格区分"事实"（原文/摘要中可找到）vs"推断"（你的判断，用[推断]标注）
2. 绝对不编造实验、数据集、结果；不确定时写"未在当前片段中找到"
3. 每一项 Claims 尽量引用原文片段（用 > 引用格式），否则写"[无原文支撑]"
4. 输出必须是中文 Markdown
5. 每个栏目开头必须加：> AI Draft（可编辑，需人工核验）
6. 只输出指定栏目，不要输出额外解释

评分量表 Rubric（必须在"评分量表"栏目中应用）：

Novelty（原创性）:
  1=增量改进/复现；2=组合已有方法；3=新任务/新视角；4=新范式突破；5=开创性/里程碑

Leverage（杠杆效应）:
  1=难以落地；2=需大量适配；3=可直接应用；4=显著降本/提效；5=范式级影响

Evidence（证据强度）:
  1=无实验/toy实验；2=部分任务；3=充分任务覆盖；4=与强基线对比；5=消融/分析完整

Cost（成本/代价）:
  1=极高（训练/推理）；2=较高；3=中等；4=较低；5=极低/可忽略

Moat（护城河）:
  1=无壁垒；2=代码护城河；3=数据护城河；4=算法/专利壁垒；5=生态/网络效应

Adoption Signal（采纳信号）:
  1=无采纳；2=GitHub<100 stars；3=GitHub>1k/论文>10引用；4=工业落地；5=生态标配

输出格式：评分行必须写成 `* Novelty (1-5): 3` 这种机器可解析格式。
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
- 必须包含：Novelty/Leverage/Evidence/Cost/Moat/Adoption Signal + Overall Judgment
- 每一项评分行格式：`* Novelty (1-5): 3`（机器可解析）
"""

    return call_llm_chat_completions(
        base_url=base_url,
        api_key=api_key,
        model=model,
        system_prompt=system_prompt,
        user_prompt=user_prompt,
    )
