"""
Research Session Tracker: Track research conversations and knowledge graphs

研究会话追踪：记录研究脉络，构建知识图谱。
"""

from __future__ import annotations

import json
import re
import time
from dataclasses import dataclass, asdict, field
from datetime import datetime, timedelta
from enum import Enum
from pathlib import Path
from typing import Optional, List, Dict, Any
from collections import defaultdict

from llm.constants import AI_RESEARCH_KEYWORDS, LLM_BASE_URL, LLM_MODEL


class ResearchIntent(Enum):
    """Research intent classification."""
    LEARNING = "learning"      # 理解概念、学习原理
    REPRODUCING = "reproducing"  # 复现代码、复现实验
    IMPROVING = "improving"    # 改进方法、创新
    COMPARING = "comparing"    # 对比分析、选型
    EXPLORING = "exploring"     # 探索发现、找方向
    CITING = "citing"          # 引用写作、文献整理


@dataclass
class Query:
    """问答记录."""
    id: str
    question: str
    answer_preview: str  # 回答预览（截取前100字）
    paper_ids: List[str]
    paper_titles: List[str]  # 论文标题
    timestamp: str
    follow_ups: List[str] = field(default_factory=list)  # 追问记录


@dataclass
class ResearchSession:
    """研究会话."""
    id: str
    title: str
    queries: List[Query]
    started_at: str
    ended_at: Optional[str] = None
    tags: List[str] = field(default_factory=list)  # 自动提取的标签
    insights: List[str] = field(default_factory=list)  # 会话洞察
    intent: ResearchIntent = ResearchIntent.LEARNING  # 检测到的研究意图

    @property
    def duration_minutes(self) -> int:
        """会话时长（分钟）."""
        if not self.ended_at:
            end = datetime.now()
        else:
            end = datetime.fromisoformat(self.ended_at)
        start = datetime.fromisoformat(self.started_at)
        return int((end - start).total_seconds() / 60)

    @property
    def topics(self) -> List[str]:
        """提取的讨论主题."""
        return list(set(self.tags))


class ResearchSessionTracker:
    """研究会话追踪器."""

    def __init__(self, memory_dir: Optional[Path] = None):
        if memory_dir is None:
            memory_dir = Path("memory/evolution")
        self.memory_dir = Path(memory_dir)
        self.memory_dir.mkdir(parents=True, exist_ok=True)
        self.sessions_file = self.memory_dir / "research_sessions.jsonl"
        self.current_session: Optional[ResearchSession] = None

        if not self.sessions_file.exists():
            self.sessions_file.write_text("", encoding="utf-8")

    def start_session(self, title: Optional[str] = None) -> ResearchSession:
        """开始新的研究会话."""
        session_id = f"session_{int(time.time())}"
        now = datetime.now().isoformat()

        self.current_session = ResearchSession(
            id=session_id,
            title=title or f"研究会话 {now[:10]}",
            queries=[],
            started_at=now,
        )
        return self.current_session

    def add_query(
        self,
        question: str,
        answer: str,
        paper_ids: List[str],
        paper_titles: List[str],
    ) -> Query:
        """添加问答到当前会话."""
        if not self.current_session:
            self.start_session()

        query = Query(
            id=f"q_{int(time.time()*1000)}",
            question=question,
            answer_preview=answer[:100] if answer else "",
            paper_ids=paper_ids,
            paper_titles=paper_titles,
            timestamp=datetime.now().isoformat(),
        )

        self.current_session.queries.append(query)

        # 自动提取标签
        self._extract_tags(question, paper_titles)

        # 自动检测研究意图
        intent = self._detect_intent(question)
        self.current_session.intent = intent

        return query

    def _detect_intent(self, question: str) -> ResearchIntent:
        """
        Detect research intent from question.

        Uses keyword and pattern matching for fast classification.
        """
        q_lower = question.lower()

        # Intent patterns (CN/EN) - simple alternation without capturing groups
        patterns = {
            ResearchIntent.REPRODUCING: [
                r'复现|实现|copy|paste|跑通|代码|code|reproduce|implement|build',
                r'怎么实现|如何复现|有代码吗|show me|给我代码',
            ],
            ResearchIntent.IMPROVING: [
                r'改进|优化|提升|更好|improve|better|enhance|boost',
                r'如何改进|能不能更好|超越|outperform|beat',
            ],
            ResearchIntent.COMPARING: [
                r'比较|对比|差异|哪个更好|vs|versus|compare|differ',
                r'和.*区别|相比.*如何|哪个更强',
            ],
            ResearchIntent.LEARNING: [
                r'是什么|原理|如何理解|学习|了解|入门|概念|definition|learn|understand|explain',
                r'什么意思|怎么理解|有什么用|what is|how does',
            ],
            ResearchIntent.EXPLORING: [
                r'有哪些|有什么 最新 最新研究 最近 探索 发现|what are|latest|recent|discover',
                r'有什么新|还有什么|还有什么方法',
            ],
            ResearchIntent.CITING: [
                r'引用|cite|参考文献|写论文|写作|如何引用|citation|bibliography',
                r'格式|规范|apa|ieee',
            ],
        }

        scores = {intent: 0 for intent in ResearchIntent}
        for intent, intent_patterns in patterns.items():
            for pattern in intent_patterns:
                if re.search(pattern, q_lower):
                    scores[intent] += 1

        max_score = max(scores.values())
        if max_score == 0:
            return ResearchIntent.LEARNING  # Default

        for intent, score in scores.items():
            if score == max_score:
                return intent

        return ResearchIntent.LEARNING

    def get_research_path_suggestion(self) -> Optional[str]:
        """
        Suggest a research path based on current session.

        Returns a path suggestion string if enough context exists.
        """
        if not self.current_session or len(self.current_session.queries) < 1:
            return None

        intent = getattr(self.current_session, 'intent', ResearchIntent.LEARNING)
        topics = self.current_session.topics

        if not topics:
            return None

        # Generate suggestions based on intent
        main_topic = topics[0] if topics else "该主题"

        suggestions = {
            ResearchIntent.LEARNING: f"📚 学习路径建议: {main_topic} → 核心论文 → 变体模型 → 应用案例",
            ResearchIntent.REPRODUCING: f"🔧 复现路径建议: 找到基准实现 → 对齐指标 → 消融实验 → 复现结果",
            ResearchIntent.IMPROVING: f"🚀 改进路径建议: {main_topic} → 痛点分析 → 改进思路 → 验证实验",
            ResearchIntent.COMPARING: f"⚖️ 对比路径建议: {main_topic} → 竞品分析 → 优缺点 → 选型建议",
            ResearchIntent.EXPLORING: f"🔍 探索路径建议: 最新论文 → 开源实现 → 社区反馈 → 实际应用",
            ResearchIntent.CITING: f"📝 引用建议: 相关工作 → 方法对比 → 贡献点 → 格式规范",
        }

        return suggestions.get(intent, f"💡 建议深入了解: {main_topic}")

    def get_probing_questions(self, use_llm: bool = True, api_key: Optional[str] = None, base_url: Optional[str] = None, model: Optional[str] = None) -> List[str]:
        """
        Generate probing questions based on session context.

        Args:
            use_llm: Whether to use LLM for dynamic question generation
            api_key: LLM API key (falls back to env)
            base_url: LLM API base URL
            model: Model name

        Returns:
            List of 1-3 thought-provoking questions
        """
        if not self.current_session:
            return []

        # Try LLM-driven generation first
        if use_llm and self.current_session.queries:
            llm_questions = self._generate_probing_questions_llm(api_key, base_url, model)
            if llm_questions:
                return llm_questions

        # Fallback to template-based questions
        questions = []
        intent = getattr(self.current_session, 'intent', ResearchIntent.LEARNING)
        topics = self.current_session.topics

        if len(topics) == 1:
            questions.append(f"这个 topic 和其他领域有什么联系？")

        if intent == ResearchIntent.LEARNING:
            questions.append(f"这个 topic 在实际项目中如何使用？")
        elif intent == ResearchIntent.REPRODUCING:
            questions.append(f"复现过程中最大的挑战是什么？")
        elif intent == ResearchIntent.IMPROVING:
            questions.append(f"现有方法的核心局限在哪里？")

        return questions[:2]

    def _generate_probing_questions_llm(
        self,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        model: Optional[str] = None,
    ) -> List[str]:
        """
        Use LLM to generate contextually relevant probing questions.

        Analyzes conversation history and research intent to generate
        personalized follow-up questions.
        """
        import os

        api_key = api_key or os.getenv("OPENAI_API_KEY", "")
        base_url = base_url or LLM_BASE_URL
        model = model or LLM_MODEL

        if not api_key:
            return []

        # Build context from conversation history
        session = self.current_session
        history_parts = []
        for i, q in enumerate(session.queries[-3:], 1):  # Last 3 queries
            history_parts.append(f"Q{i}: {q.question}")
            if q.answer_preview:
                history_parts.append(f"A{i}: {q.answer_preview[:100]}...")

        history_text = "\n".join(history_parts) if history_parts else "首次对话"
        topics = ", ".join(session.topics[:3]) if session.topics else "未知"
        intent_name = session.intent.value if session.intent else "learning"

        system_prompt = """你是一个研究助手，擅长通过追问帮助用户深入理解研究主题。
根据对话历史，生成2-3个有洞察力的追问，帮助用户进一步探索。
要求：
1. 问题要有深度，能引发思考
2. 结合用户的研究意图
3. 不要重复历史中已问过的问题
4. 用中文提问
5. 每个问题限制在20字以内
6. 只输出问题，不要解释，每行一个"""

        user_prompt = f"""对话历史：
{history_text}

研究主题：{topics}
研究意图：{intent_name}

请生成追问："""

        try:
            from llm.client import call_llm_chat_completions
            response = call_llm_chat_completions(
                base_url=base_url,
                api_key=api_key,
                model=model,
                system_prompt=system_prompt,
                user_prompt=user_prompt,
            )

            if response:
                # Parse questions from response (one per line)
                questions = []
                for line in response.strip().split('\n'):
                    line = line.strip()
                    # Remove common prefixes like "1.", "-", "•", "Q:"
                    for prefix in ['^\\d+[.、]', '^[-•*\\s]+', '^Q\\d*[:：]\\s*']:
                        import re
                        line = re.sub(prefix, '', line).strip()
                    if line and len(line) <= 30:
                        questions.append(line)

                return questions[:3]

        except Exception:
            # Rule-based question extraction failed — return empty list without crashing.
            pass

        return []

    def add_follow_up(self, query_id: str, follow_up_question: str):
        """记录追问."""
        if not self.current_session:
            return

        for q in self.current_session.queries:
            if q.id == query_id:
                q.follow_ups.append(follow_up_question)
                break

    def end_session(self) -> ResearchSession:
        """结束当前会话."""
        if not self.current_session:
            return None

        self.current_session.ended_at = datetime.now().isoformat()

        # 生成洞察
        self._generate_insights()

        # 保存到文件
        self._save_session(self.current_session)

        session = self.current_session
        self.current_session = None
        return session

    def _extract_tags(self, question: str, paper_titles: List[str]):
        """从问答中提取标签."""
        if not self.current_session:
            return

        text = f"{question} {' '.join(paper_titles)}".lower()

        found = [tag for tag in AI_RESEARCH_KEYWORDS if re.search(r'\b' + re.escape(tag) + r'\b', text)]
        self.current_session.tags.extend(found)

    def _generate_insights(self):
        """生成会话洞察."""
        if not self.current_session or not self.current_session.queries:
            return

        insights = []

        # 洞察1: 会话主题
        if self.current_session.topics:
            insights.append(f"主要研究主题: {', '.join(self.current_session.topics[:3])}")

        # 洞察2: 探索深度
        total_followups = sum(len(q.follow_ups) for q in self.current_session.queries)
        if total_followups > 2:
            insights.append("进行了深度探索（多次追问）")
        elif total_followups > 0:
            insights.append("进行了初步探索")

        # 洞察3: 论文覆盖
        all_papers = set()
        for q in self.current_session.queries:
            all_papers.update(q.paper_titles)
        if len(all_papers) > 3:
            insights.append(f"覆盖了 {len(all_papers)} 篇相关论文")

        self.current_session.insights = insights

    def _save_session(self, session: ResearchSession):
        """保存会话到文件."""
        with open(self.sessions_file, "a", encoding="utf-8") as f:
            f.write(json.dumps(asdict(session), ensure_ascii=False) + "\n")

    def get_recent_sessions(self, days: int = 7, limit: int = 10) -> List[ResearchSession]:
        """获取最近的会话."""
        sessions = []
        cutoff = datetime.now() - timedelta(days=days)

        try:
            with open(self.sessions_file, encoding="utf-8") as f:
                for line in f:
                    if not line.strip():
                        continue
                    data = json.loads(line)
                    session = ResearchSession(**data)
                    started = datetime.fromisoformat(session.started_at)
                    if started >= cutoff:
                        sessions.append(session)

            sessions.sort(key=lambda x: x.started_at, reverse=True)
            return sessions[:limit]
        except (json.JSONDecodeError, FileNotFoundError):
            return []

    def get_session_by_id(self, session_id: str) -> Optional[ResearchSession]:
        """根据ID获取会话."""
        try:
            with open(self.sessions_file, encoding="utf-8") as f:
                for line in f:
                    if not line.strip():
                        continue
                    data = json.loads(line)
                    if data.get("id") == session_id:
                        return ResearchSession(**data)
        except (json.JSONDecodeError, FileNotFoundError):
            pass
        return None

    def get_current_session(self) -> Optional[ResearchSession]:
        """获取当前会话."""
        return self.current_session

    def render_session_tree(self, session: ResearchSession) -> str:
        """渲染会话为树形结构."""
        lines = [
            f"📚 {session.title}",
            f"   时长: {session.duration_minutes} 分钟 | {len(session.queries)} 个问答",
        ]

        if session.insights:
            lines.append(f"   💡 {' | '.join(session.insights[:2])}")

        lines.append("")

        for i, q in enumerate(session.queries, 1):
            # 问题
            indent = "   " if i == 1 else "       "
            lines.append(f"{indent}Q{i}: {q.question[:60]}{'...' if len(q.question) > 60 else ''}")

            # 引用论文
            if q.paper_titles:
                for title in q.paper_titles[:2]:
                    lines.append(f"{indent}   📄 {title[:50]}{'...' if len(title) > 50 else ''}")

            # 追问
            if q.follow_ups:
                lines.append(f"{indent}   └─ {len(q.follow_ups)} 次追问")

        return "\n".join(lines)

    def render_sessions_list(self, sessions: List[ResearchSession]) -> str:
        """渲染会话列表."""
        if not sessions:
            return "暂无研究会话记录"

        lines = ["=" * 50]
        for s in sessions:
            date = s.started_at[:10]
            lines.append(f"📅 {date} | {s.title} ({len(s.queries)}问答)")
            if s.insights:
                lines.append(f"   💡 {s.insights[0][:50]}")
            lines.append("")

        return "\n".join(lines)


# 全局实例
_session_tracker: Optional[ResearchSessionTracker] = None


def get_session_tracker() -> ResearchSessionTracker:
    """获取全局会话追踪器."""
    global _session_tracker
    if _session_tracker is None:
        _session_tracker = ResearchSessionTracker()
    return _session_tracker
