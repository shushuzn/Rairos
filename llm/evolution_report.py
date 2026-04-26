"""
Evolution Report Generator: Generate Learning Reports for Users

双向进化的核心：系统从用户反馈中学习，同时向用户展示学习成果。
"""

from __future__ import annotations

import json
from dataclasses import dataclass, asdict, field
from datetime import datetime, timedelta
from pathlib import Path
from typing import Optional, List, Dict, Any, TYPE_CHECKING
from collections import Counter, defaultdict
import re

from llm.evolution import get_evolution_memory, FeedbackType


@dataclass
class PaperInsight:
    """论文洞察."""
    paper_id: str
    title: str
    positive_count: int = 0
    negative_count: int = 0
    avg_score: float = 0.0
    related_queries: List[str] = field(default_factory=list)

    @property
    def boost_score(self) -> float:
        """计算boost分数."""
        total = self.positive_count + self.negative_count
        if total == 0:
            return 0.0
        return (self.positive_count - self.negative_count * 0.5) / total


@dataclass
class QueryInsight:
    """查询洞察."""
    keywords: List[str]
    avg_score: float
    success_rate: float
    related_papers: List[str]
    suggestions: List[str] = field(default_factory=list)


@dataclass
class LearningReport:
    """学习报告."""
    period_start: str
    period_end: str
    total_queries: int
    positive_rate: float
    top_papers: List[PaperInsight]
    top_keywords: List[str]
    emerging_patterns: List[str]
    predicted_interests: List[str]
    questions_to_explore: List[str]
    evolution_stage: str
    progress_towards_next: str

    def to_markdown(self) -> str:
        """转换为Markdown格式."""
        lines = [
            "# 🧬 AI Research OS 学习报告",
            "",
            f"**周期**: {self.period_start[:10]} ~ {self.period_end[:10]}",
            "",
            "---",
            "",
            "## 📊 核心指标",
            "",
            f"- 总问答数: {self.total_queries}",
            f"- 满意率: {self.positive_rate * 100:.1f}%",
            f"- 进化阶段: {self.evolution_stage}",
            "",
            "---",
            "",
        ]

        if self.top_papers:
            lines.extend(["## 📚 热门论文", "", "这些论文在问答中被频繁引用且获得高满意度：", ""])
            for i, p in enumerate(self.top_papers[:3], 1):
                lines.append(f"{i}. **{p.title}**")
                lines.append(f"   - 被引用 {p.positive_count} 次 | Boost: {p.boost_score:.2f}")
                lines.append("")

        if self.top_keywords:
            lines.extend(["## 🔑 关注热点", "", "你最近最关心的话题：", "", "```", "  " + " | ".join(self.top_keywords[:5]), "```", ""])

        if self.questions_to_explore:
            lines.extend(["## 💡 建议探索", "", "基于你的阅读历史，系统推荐以下问题：", ""])
            for q in self.questions_to_explore:
                lines.append(f"- {q}")
            lines.append("")

        if self.predicted_interests:
            lines.extend(["## 🔮 趋势预测", "", "系统预测你可能感兴趣的方向：", ""])
            for interest in self.predicted_interests:
                lines.append(f"- {interest}")
            lines.append("")

        lines.extend(["---", "", f"📍 {self.evolution_stage} | {self.progress_towards_next}"])
        return "\n".join(lines)


class EvolutionReporter:
    """学习报告生成器."""

    def __init__(self, evolution_memory=None):
        self.evo = evolution_memory or get_evolution_memory()

    def generate_report(self, days: int = 7, db=None) -> LearningReport:
        """生成学习报告."""
        now = datetime.now()
        start_time = (now - timedelta(days=days)).isoformat()
        feedbacks = self._collect_feedbacks_since(start_time)

        if not feedbacks:
            return self._empty_report(start_time, now.isoformat())

        paper_insights = self._analyze_paper_insights(feedbacks)
        suggestions = self._generate_suggestions(feedbacks, paper_insights, db)
        predicted = self._predict_interests(feedbacks, paper_insights)
        stats = self.evo.get_stats()
        stage, progress = self._get_evolution_status(stats)

        return LearningReport(
            period_start=start_time,
            period_end=now.isoformat(),
            total_queries=len(feedbacks),
            positive_rate=self._calc_positive_rate(feedbacks),
            top_papers=sorted(paper_insights, key=lambda x: x.boost_score, reverse=True)[:5],
            top_keywords=self._extract_top_keywords(feedbacks)[:5],
            emerging_patterns=self._find_emerging_patterns(feedbacks),
            predicted_interests=predicted,
            questions_to_explore=suggestions,
            evolution_stage=stage,
            progress_towards_next=progress,
        )

    def _collect_feedbacks_since(self, start_time: str) -> List[Dict]:
        feedbacks = []
        try:
            with open(self.evo.feedback_file, encoding="utf-8") as f:
                for line in f:
                    if line.strip():
                        data = json.loads(line)
                        if data.get("timestamp", "") >= start_time:
                            feedbacks.append(data)
        except FileNotFoundError:
            pass
        return feedbacks

    def _analyze_paper_insights(self, feedbacks: List[Dict]) -> List[PaperInsight]:
        paper_data = defaultdict(lambda: {"positive": 0, "negative": 0, "scores": [], "queries": []})
        for fb in feedbacks:
            if fb.get("command") != "chat":
                continue
            for paper_id in fb.get("paper_ids", []):
                pdata = paper_data[paper_id]
                if fb.get("type") == FeedbackType.POSITIVE.value:
                    pdata["positive"] += 1
                else:
                    pdata["negative"] += 1
                pdata["scores"].append(fb.get("score", 0.5))
                pdata["queries"].append(fb.get("query", "")[:50])

        insights = []
        for paper_id, pdata in paper_data.items():
            insights.append(PaperInsight(
                paper_id=paper_id,
                title=paper_id,
                positive_count=pdata["positive"],
                negative_count=pdata["negative"],
                avg_score=sum(pdata["scores"]) / len(pdata["scores"]) if pdata["scores"] else 0,
                related_queries=pdata["queries"][:3],
            ))
        return insights

    def _extract_top_keywords(self, feedbacks: List[Dict]) -> List[str]:
        all_text = " ".join([fb.get("query", "") + " " + " ".join(fb.get("paper_ids", [])) for fb in feedbacks])
        stopwords = {"the", "is", "are", "a", "an", "what", "how", "why", "this", "that", "and", "or", "的", "是", "如何", "什么", "怎么"}
        words = re.findall(r'[\u4e00-\u9fff]+|[a-zA-Z]{3,}', all_text.lower())
        filtered = [w for w in words if w not in stopwords and len(w) > 2]
        return [w for w, _ in Counter(filtered).most_common(10)]

    def _find_emerging_patterns(self, feedbacks: List[Dict]) -> List[str]:
        patterns = []
        compare_keywords = ["vs", "versus", "比较", "区别", "diff", "对比"]
        compare_count = sum(1 for fb in feedbacks if any(kw in fb.get("query", "").lower() for kw in compare_keywords))
        if compare_count > len(feedbacks) * 0.2:
            patterns.append("你开始关注论文间的比较分析")

        long_queries = sum(1 for fb in feedbacks if len(fb.get("query", "")) > 30)
        if long_queries > len(feedbacks) * 0.5:
            patterns.append("问题变得更加深入和具体")

        return patterns

    def _generate_suggestions(self, feedbacks: List[Dict], paper_insights: List, db=None) -> List[str]:
        suggestions = []
        if paper_insights:
            top_paper = paper_insights[0]
            suggestions.append(f"深入探索 \"{top_paper.paper_id}\" 的相关工作")
        keywords = self._extract_top_keywords(feedbacks)
        if keywords:
            suggestions.append(f"了解 {keywords[0]} 的最新研究进展")
        suggestions.extend(["追踪你关注领域的最新论文", "定期回顾已读论文的核心贡献"])
        return suggestions[:5]

    def _predict_interests(self, feedbacks: List[Dict], paper_insights: List) -> List[str]:
        predictions = []
        recent_queries = [fb.get("query", "") for fb in feedbacks[-5:]]
        recent_text = " ".join(recent_queries).lower()
        if "llm" in recent_text or "language model" in recent_text:
            predictions.append("LLM架构优化")
        if "training" in recent_text or "训练" in recent_text:
            predictions.append("模型训练技巧")
        if "efficient" in recent_text or "高效" in recent_text:
            predictions.append("效率优化方法")
        return predictions[:3]

    def _calc_positive_rate(self, feedbacks: List[Dict]) -> float:
        if not feedbacks:
            return 0.0
        positive = sum(1 for fb in feedbacks if fb.get("type") == FeedbackType.POSITIVE.value)
        return positive / len(feedbacks)

    def _get_evolution_status(self, stats: Dict) -> tuple:
        reliable = stats.get("reliable_patterns", 0)
        if reliable >= 5:
            return "🚀 进化期", "系统已具备自进化能力"
        elif reliable >= 3:
            return "🌲 成熟期", "扩展模式库，覆盖更多场景"
        elif reliable >= 1:
            return "🌳 成长期", "积累 10+ 反馈，强化现有模式"
        return "🌱 种子期", "继续使用，系统会持续学习"

    def _empty_report(self, start: str, end: str) -> LearningReport:
        return LearningReport(
            period_start=start, period_end=end, total_queries=0, positive_rate=0.0,
            top_papers=[], top_keywords=[], emerging_patterns=["开始使用系统，开始你的研究之旅"],
            predicted_interests=[], questions_to_explore=[
                "尝试用 airos chat 问一个关于论文的问题",
                "探索 airos search 发现新论文",
                "用 airos slides 生成论文幻灯片",
            ],
            evolution_stage="🌱 种子期", progress_towards_next="开始使用，获得你的第一个满意回答",
        )

    def save_report(self, report: LearningReport, output_path: Optional[Path] = None) -> Path:
        if output_path is None:
            timestamp = datetime.now().strftime("%Y%m%d")
            output_path = self.evo.memory_dir / f"report_{timestamp}.md"
        output_path.write_text(report.to_markdown(), encoding="utf-8")
        return output_path


def generate_evolution_report(days: int = 7, db=None) -> LearningReport:
    """快捷函数：生成学习报告."""
    reporter = EvolutionReporter()
    return reporter.generate_report(days=days, db=db)


class AdaptiveRetrieval:
    """自适应检索：根据反馈优化检索结果，带置信度和探索多样性."""

    # 置信度阈值：样本数达到此值时 boost 完全生效
    CONFIDENCE_THRESHOLD = 5
    # 探索多样性：同类主题最多返回比例
    DIVERSITY_RATIO = 0.6

    def __init__(self, evolution_memory=None):
        self.evo = evolution_memory or get_evolution_memory()
        self.boost_file = self.evo.memory_dir / "paper_boost.json"
        self._load_boost()

    def _load_boost(self):
        try:
            self.boost_data = json.loads(self.boost_file.read_text(encoding="utf-8") or "{}")
        except (json.JSONDecodeError, FileNotFoundError):
            self.boost_data = {}

    def _save_boost(self):
        self.boost_file.write_text(json.dumps(self.boost_data, indent=2, ensure_ascii=False), encoding="utf-8")

    def record_retrieval(self, paper_id: str, query: str, was_useful: bool):
        if paper_id not in self.boost_data:
            self.boost_data[paper_id] = {"positive_mentions": 0, "negative_mentions": 0, "queries": []}
        data = self.boost_data[paper_id]
        if was_useful:
            data["positive_mentions"] += 1
        else:
            data["negative_mentions"] += 1
        data["queries"].append(query[:100])
        if len(data["queries"]) > 20:
            data["queries"] = data["queries"][-20:]
        total = data["positive_mentions"] + data["negative_mentions"]
        # Wilson Score Interval - 提供置信度加权的 boost
        data["boost_score"] = self._wilson_score(
            data["positive_mentions"],
            total,
            confidence=0.95
        )
        data["confidence"] = min(total / self.CONFIDENCE_THRESHOLD, 1.0)
        data["last_update"] = datetime.now().isoformat()
        self._save_boost()

    def _wilson_score(self, positives: int, total: int, confidence: float = 0.95) -> float:
        """Wilson Score Interval - 置信度加权的评分."""
        if total == 0:
            return 0.0
        # Wilson 下界，避免小样本时评分虚高
        import math
        p = positives / total
        z = 1.645 if confidence == 0.95 else 1.96  # z-score
        n = total
        denom = 1 + z**2 / n
        center = p + z**2 / (2 * n)
        margin = z * math.sqrt(p * (1 - p) / n + z**2 / (4 * n**2))
        wilson_lower = (center - margin) / denom
        return wilson_lower * 2 - 0.5  # 映射到 [-0.5, 1.5] 范围

    def get_boost(self, paper_id: str) -> tuple:
        """获取 boost 值和置信度."""
        data = self.boost_data.get(paper_id, {})
        return data.get("boost_score", 0.0), data.get("confidence", 0.0)

    def apply_boost(self, results: List[Dict], decay: float = 0.1) -> List[Dict]:
        """应用 boost，支持置信度加权和探索多样性."""
        boosted = []
        topic_counts: dict = {}

        for r in results:
            paper_id = r.get("paper_id", "")
            boost, confidence = self.get_boost(paper_id)
            age = self._get_boost_age(paper_id)

            # 置信度加权：样本少时 boost 衰减
            confidence_weight = confidence  # 0~1，样本少时接近0
            decayed_boost = boost * confidence_weight * (decay ** (age / 30))

            original_score = r.get("score", 0.5)
            final_score = original_score + decayed_boost * 0.2

            # 探索多样性惩罚：同类主题过多时降权
            topic = r.get("topic", "unknown")
            topic_counts[topic] = topic_counts.get(topic, 0) + 1
            diversity_penalty = self._calc_diversity_penalty(topic, topic_counts, len(results))

            boosted.append({
                **r,
                "score": final_score * diversity_penalty,
                "boost": decayed_boost,
                "confidence": confidence,
                "topic": topic,
            })

        # 先按 score 排序，再应用多样性重排
        boosted.sort(key=lambda x: x["score"], reverse=True)
        return self._apply_diversity_rerank(boosted)

    def _calc_diversity_penalty(self, topic: str, topic_counts: dict, total: int) -> float:
        """计算多样性惩罚，同类过多时降权."""
        if total == 0:
            return 1.0
        count = topic_counts.get(topic, 0)
        ratio = count / total
        if ratio > self.DIVERSITY_RATIO:
            # 超出阈值的部分应用惩罚
            penalty = 1.0 - (ratio - self.DIVERSITY_RATIO) * 0.5
            return max(penalty, 0.7)  # 最低不低于 0.7
        return 1.0

    def _apply_diversity_rerank(self, results: List[Dict]) -> List[Dict]:
        """多样性重排：确保结果不会过于集中在同一主题."""
        diverse: list = []
        topics_seen: dict = {}

        for r in results:
            topic = r.get("topic", "unknown")
            if len(diverse) < 3:
                # 前3个直接加入
                diverse.append(r)
                topics_seen[topic] = topics_seen.get(topic, 0) + 1
            else:
                # 之后检查多样性：如果同类超过2个，考虑跳过
                if topics_seen.get(topic, 0) < 2:
                    diverse.append(r)
                    topics_seen[topic] = topics_seen.get(topic, 0) + 1
                else:
                    # 放到末尾
                    r["_diversity_deferred"] = True
                    diverse.append(r)

        return diverse

    def _get_boost_age(self, paper_id: str) -> int:
        data = self.boost_data.get(paper_id, {})
        last_update = data.get("last_update", "")
        if not last_update:
            return 30
        try:
            last = datetime.fromisoformat(last_update)
            return (datetime.now() - last).days
        except Exception:
            return 30


def get_adaptive_retrieval() -> AdaptiveRetrieval:
    return AdaptiveRetrieval()


# === Smart Follow-Up System ===
class FollowUpType:
    """追问类型."""
    MATH = "math"           # 数学原理
    CODE = "code"           # 代码实现
    COMPARE = "compare"      # 历史对比
    EVOLUTION = "evolution"  # 演进关系
    PRACTICE = "practice"    # 实践应用
    CITATION = "citation"    # 引用论文


@dataclass
class FollowUp:
    """追问选项."""
    text: str           # 显示文本
    type: str           # 追问类型
    query: str          # 自动生成的查询
    icon: str           # 图标
    depth: int = 1      # 追问深度


class SmartFollowUp:
    """智能追问系统：基于回答内容和论文引用生成追问选项."""

    # 关键词映射到追问类型
    TOPIC_KEYWORDS = {
        FollowUpType.MATH: [
            "attention", "score", "softmax", "matrix", "dot", "product",
            "gradient", "loss", "optimize", "layer", "weight", "参数", "矩阵",
            "注意力", "梯度", "优化", "计算", "function", "mechanism"
        ],
        FollowUpType.CODE: [
            "implement", "code", "function", "class", "api", "library",
            "pytorch", "tensorflow", "layer", "module", "实现", "代码",
            "函数", "模块", "algorithm"
        ],
        FollowUpType.COMPARE: [
            "vs", "versus", "better", "worse", "compare", "different",
            "advantage", "disadvantage", "相比", "优于", "区别", "对比"
        ],
        FollowUpType.EVOLUTION: [
            "based on", "follow", "extend", "improve", "build upon",
            "later", "previous", "next", "演进", "改进", "基于", "后续",
            "evolution", "derived", "succeed"
        ],
        FollowUpType.PRACTICE: [
            "apply", "use", "application", "industry", "practical",
            "deploy", "production", "应用", "实践", "工业", "部署",
            "real-world", "benchmark"
        ],
    }

    def __init__(self, evolution_memory=None):
        self.evo = evolution_memory or get_evolution_memory()

    def generate_options(
        self,
        question: str,
        answer: str,
        citations: List[Any] = None,
    ) -> List[FollowUp]:
        """基于问答内容和论文引用生成追问选项."""
        citations = citations or []

        # 1. 优先从引用论文生成追问
        options = self._generate_from_citations(question, answer, citations)

        # 2. 补充基于关键词的追问
        text = f"{question} {answer}".lower()
        detected_types = self._detect_topic_types(text)

        for ftype in detected_types[:3]:
            if len(options) >= 4:
                break
            followup = self._create_followup(ftype, question, answer, citations)
            if followup and not self._is_duplicate(followup, options):
                options.append(followup)

        # 3. 如果没有足够选项，生成通用追问
        if len(options) < 2:
            generic = self._generate_generic_options(question, citations)
            for opt in generic:
                if not self._is_duplicate(opt, options):
                    options.append(opt)

        return options[:4]  # 最多4个选项

    def _generate_from_citations(
        self,
        question: str,
        answer: str,
        citations: List[Any],
    ) -> List[FollowUp]:
        """从引用论文生成追问."""
        options = []
        if not citations:
            return options

        # 从第一篇（最相关）论文提取概念
        primary = citations[0]
        paper_title = getattr(primary, 'paper_title', '') or ''

        # 提取论文中的技术关键词
        tech_keywords = self._extract_technical_terms(answer)

        # 生成引用导向的追问
        if paper_title and tech_keywords:
            # 追问这篇论文被谁引用/影响了谁
            options.append(FollowUp(
                text=f"📑 引用这篇论文的后续工作有哪些？",
                type=FollowUpType.CITATION,
                query=f"papers citing {paper_title}",
                icon="📑",
                depth=1,
            ))

            # 基于技术术语的追问
            if tech_keywords:
                main_term = tech_keywords[0]
                options.append(FollowUp(
                    text=f"🔗 {main_term} 在其他论文中如何应用？",
                    type=FollowUpType.EVOLUTION,
                    query=f"{main_term} application in other papers",
                    icon="🔗",
                    depth=1,
                ))

        return options

    def _extract_technical_terms(self, text: str) -> List[str]:
        """从文本中提取技术术语."""
        # 技术术语模式
        import re
        patterns = [
            r'\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+)*\s+(?:mechanism|model|network|architecture|method|algorithm)\b',
            r'\b(?:self-|cross-|multi-|hierarchical)\s*\w+(?:-\w+)*\b',
            r'\b\w+(?:-\w+){1,3}\b',  # 连字符术语
        ]

        terms = []
        for pattern in patterns:
            matches = re.findall(pattern, text, re.IGNORECASE)
            terms.extend([m for m in matches if len(m) > 4])

        # 去重并过滤
        seen = set()
        unique_terms = []
        for t in terms:
            t_lower = t.lower()
            if t_lower not in seen and len(t) > 5:
                seen.add(t_lower)
                unique_terms.append(t)

        return unique_terms[:3]

    def _detect_topic_types(self, text: str) -> List[str]:
        """检测文本中的主题类型."""
        scores = {}
        for ftype, keywords in self.TOPIC_KEYWORDS.items():
            score = sum(1 for kw in keywords if kw in text)
            if score > 0:
                scores[ftype] = score

        # 按分数排序
        return sorted(scores.keys(), key=lambda x: scores[x], reverse=True)

    def _create_followup(
        self,
        ftype: str,
        question: str,
        answer: str,
        citations: List[Any],
    ) -> Optional[FollowUp]:
        """为特定类型创建追问."""
        templates = {
            FollowUpType.MATH: {
                "icon": "∫",
                "text": "深入技术原理",
                "query_tpl": "{q} 的数学原理和计算细节是什么？",
            },
            FollowUpType.CODE: {
                "icon": "⚙",
                "text": "查看代码实现",
                "query_tpl": "{q} 的 PyTorch/TensorFlow 实现代码？",
            },
            FollowUpType.COMPARE: {
                "icon": "📜",
                "text": "与其他方法对比",
                "query_tpl": "{q} 和其他方法相比有什么优缺点？",
            },
            FollowUpType.EVOLUTION: {
                "icon": "🌳",
                "text": "了解发展脉络",
                "query_tpl": "{q} 是如何演进发展的？有哪些改进版本？",
            },
            FollowUpType.PRACTICE: {
                "icon": "🏭",
                "text": "实际应用场景",
                "query_tpl": "{q} 在工业界有哪些实际应用？",
            },
        }

        if ftype not in templates:
            return None

        template = templates[ftype]
        # 提取核心概念，优先使用技术术语
        concept = self._extract_concept(question, answer)

        return FollowUp(
            text=f"{template['icon']} {template['text']}: {concept}",
            type=ftype,
            query=template["query_tpl"].format(q=concept),
            icon=template["icon"],
            depth=1,
        )

    def _extract_concept(self, question: str, answer: str) -> str:
        """从问答中提取核心概念，优先技术术语."""
        # 优先从回答中提取技术术语
        text = f"{question} {answer}"

        # 尝试提取技术术语
        tech_terms = self._extract_technical_terms(answer)
        if tech_terms:
            return tech_terms[0][:40]

        # 移除常见词
        stopwords = {
            "what", "is", "are", "how", "why", "when", "where",
            "the", "a", "an", "this", "that", "these", "those",
            "的", "是", "如何", "什么", "怎么", "为什么", "please", "explain"
        }

        words = text.split()
        keywords = [w for w in words if w.lower() not in stopwords and len(w) > 2]

        # 取前3个关键词组成概念
        if len(keywords) >= 3:
            concept = " ".join(keywords[:3])
        elif keywords:
            concept = " ".join(keywords)
        else:
            concept = question[:30]

        return concept[:50]  # 限制长度

    def _is_duplicate(self, new_opt: FollowUp, options: List[FollowUp]) -> bool:
        """检查是否与现有选项重复."""
        for opt in options:
            # 类型相同且查询相似
            if opt.type == new_opt.type:
                # 计算相似度（简单版本：检查共同词）
                opt_words = set(opt.query.lower().split())
                new_words = set(new_opt.query.lower().split())
                overlap = len(opt_words & new_words)
                if overlap >= 2:
                    return True
        return False

    def _generate_generic_options(
        self,
        question: str,
        citations: List[Any],
    ) -> List[FollowUp]:
        """生成通用追问选项."""
        concept = self._extract_concept(question, "")

        return [
            FollowUp(
                text=f"∫ {concept} 的核心技术原理是什么？",
                type=FollowUpType.MATH,
                query=f"{concept} 的技术原理详解",
                icon="∫",
            ),
            FollowUp(
                text=f"⚙ {concept} 有什么代码实现？",
                type=FollowUpType.CODE,
                query=f"{concept} 代码实现示例",
                icon="⚙",
            ),
            FollowUp(
                text=f"🌳 {concept} 相关的论文有哪些？",
                type=FollowUpType.EVOLUTION,
                query=f"{concept} 相关论文推荐",
                icon="🌳",
            ),
        ]

    def render_options(self, options: List[FollowUp]) -> str:
        """渲染追问选项为可读文本."""
        if not options:
            return ""

        lines = ["", "📌 想深入了解？选择一个追问："]
        for i, opt in enumerate(options, 1):
            lines.append(f"   [{i}] {opt.text}")
        lines.append("")
        return "\n".join(lines)


def get_smart_followup() -> SmartFollowUp:
    """获取智能追问实例."""
    return SmartFollowUp()
