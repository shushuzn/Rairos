"""
Research Session Tracker: Track research conversations and knowledge graphs

研究会话追踪：记录研究脉络，构建知识图谱。
"""

from __future__ import annotations

import json
import time
from dataclasses import dataclass, asdict, field
from datetime import datetime, timedelta
from pathlib import Path
from typing import Optional, List, Dict, Any
from collections import defaultdict


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

        return query

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

        # 常见AI研究主题
        known_tags = {
            "transformer", "attention", "bert", "gpt", "llm", "language model",
            "neural", "network", "embedding", "fine-tuning", "rlhf", "rag",
            "retrieval", "generative", "diffusion", "gan", "clip", "vit",
            "reinforcement", "policy", "reward", "training", "optimization",
        }

        found = [tag for tag in known_tags if tag in text]
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
