"""
Evolution Memory: User Feedback & Pattern Learning

记录用户反馈和学习模式，为系统自进化提供数据基础。
"""

from __future__ import annotations

import json
import time
from dataclasses import dataclass, asdict
from datetime import datetime
from pathlib import Path
from typing import Optional, List, Dict, Any
from enum import Enum


class FeedbackType(Enum):
    """反馈类型."""
    POSITIVE = "positive"      # 用户满意
    NEGATIVE = "negative"      # 用户不满意
    NEUTRAL = "neutral"         # 中性


class SignalType(Enum):
    """信号类型."""
    CHAT_SUCCESS = "chat_success"
    CHAT_FAILURE = "chat_failure"
    RETRIEVAL_HIT = "retrieval_hit"
    RETRIEVAL_MISS = "retrieval_miss"
    SLIDE_QUALITY = "slide_quality"
    SEARCH_SUCCESS = "search_success"


@dataclass
class Feedback:
    """用户反馈记录."""
    id: str
    type: str  # FeedbackType.value
    command: str  # chat, slides, search
    query: str
    paper_ids: List[str]
    outcome: str  # success, partial, failure
    score: float  # 0-1 置信度
    note: str = ""  # 用户备注
    timestamp: str = ""

    def __post_init__(self):
        if not self.timestamp:
            self.timestamp = datetime.now().isoformat()
        if not self.id:
            self.id = f"fb_{int(time.time()*1000)}"


@dataclass
class EvolutionEvent:
    """进化事件记录."""
    id: str
    signal_type: str  # SignalType.value
    trigger: Dict[str, Any]  # 触发条件
    action: str  # 采取的行动
    outcome: str  # 结果
    score: float  # 评分 0-1
    genes_applied: List[str] = None  # 应用的基因
    timestamp: str = ""

    def __post_init__(self):
        if not self.timestamp:
            self.timestamp = datetime.now().isoformat()
        if not self.id:
            self.id = f"ev_{int(time.time()*1000)}"
        if self.genes_applied is None:
            self.genes_applied = []


@dataclass
class LearnedPattern:
    """学到的模式."""
    name: str
    signal_type: str
    trigger_conditions: Dict[str, Any]
    success_count: int = 0
    failure_count: int = 0
    last_used: str = ""
    effectiveness: float = 0.0  # 成功率

    @property
    def total_attempts(self) -> int:
        return self.success_count + self.failure_count

    @property
    def is_reliable(self) -> bool:
        return self.total_attempts >= 3 and self.effectiveness >= 0.7


class EvolutionMemory:
    """Evolution Memory Store — 存储和管理进化数据."""

    def __init__(self, memory_dir: Optional[Path] = None):
        if memory_dir is None:
            memory_dir = Path("memory/evolution")
        self.memory_dir = Path(memory_dir)
        self.memory_dir.mkdir(parents=True, exist_ok=True)

        self.feedback_file = self.memory_dir / "feedback.jsonl"
        self.events_file = self.memory_dir / "evolution_events.jsonl"
        self.patterns_file = self.memory_dir / "learned_patterns.json"

        # 确保文件存在
        for f in [self.feedback_file, self.events_file]:
            if not f.exists():
                f.write_text("", encoding="utf-8")
        if not self.patterns_file.exists():
            self._save_patterns({})

    # === Feedback Operations ===

    def add_feedback(self, feedback: Feedback) -> None:
        """记录用户反馈."""
        with open(self.feedback_file, "a", encoding="utf-8") as f:
            f.write(json.dumps(asdict(feedback), ensure_ascii=False) + "\n")

    def record_chat_feedback(
        self,
        query: str,
        paper_ids: List[str],
        is_positive: bool,
        outcome: str = "success",
        score: float = 0.8,
        note: str = "",
    ) -> None:
        """快捷方法：记录聊天反馈."""
        fb = Feedback(
            id=f"fb_{int(time.time()*1000)}",
            type=FeedbackType.POSITIVE.value if is_positive else FeedbackType.NEGATIVE.value,
            command="chat",
            query=query,
            paper_ids=paper_ids,
            outcome=outcome,
            score=score,
            note=note,
        )
        self.add_feedback(fb)

        # 同步创建 Evolution Event
        signal = SignalType.CHAT_SUCCESS if is_positive else SignalType.CHAT_FAILURE
        self.record_evolution_event(
            signal_type=signal.value,
            trigger={"query": query, "papers": paper_ids},
            action="chat_response",
            outcome=outcome,
            score=score,
        )

    # === Evolution Events ===

    def record_evolution_event(
        self,
        signal_type: str,
        trigger: Dict[str, Any],
        action: str,
        outcome: str,
        score: float,
        genes_applied: Optional[List[str]] = None,
    ) -> None:
        """记录进化事件."""
        event = EvolutionEvent(
            id=f"ev_{int(time.time()*1000)}",
            signal_type=signal_type,
            trigger=trigger,
            action=action,
            outcome=outcome,
            score=score,
            genes_applied=genes_applied or [],
        )
        with open(self.events_file, "a", encoding="utf-8") as f:
            f.write(json.dumps(asdict(event), ensure_ascii=False) + "\n")

        # 更新模式
        self._update_pattern_from_event(event)

    def _update_pattern_from_event(self, event: EvolutionEvent) -> None:
        """从事件更新学习到的模式."""
        patterns = self._load_patterns()

        # 基于信号类型创建/更新模式
        pattern_key = f"{event.signal_type}_{event.action}"
        if pattern_key not in patterns:
            patterns[pattern_key] = {
                "name": pattern_key,
                "signal_type": event.signal_type,
                "trigger_conditions": event.trigger,
                "success_count": 0,
                "failure_count": 0,
                "last_used": "",
                "effectiveness": 0.0,
            }

        p = patterns[pattern_key]
        if event.score >= 0.6:
            p["success_count"] += 1
        else:
            p["failure_count"] += 1

        total = p["success_count"] + p["failure_count"]
        p["effectiveness"] = p["success_count"] / total if total > 0 else 0.0
        p["last_used"] = event.timestamp

        self._save_patterns(patterns)

    # === Pattern Operations ===

    def _load_patterns(self) -> Dict[str, Any]:
        """加载模式库."""
        try:
            return json.loads(self.patterns_file.read_text(encoding="utf-8") or "{}")
        except (json.JSONDecodeError, FileNotFoundError):
            return {}

    def _save_patterns(self, patterns: Dict[str, Any]) -> None:
        """保存模式库."""
        self.patterns_file.write_text(
            json.dumps(patterns, indent=2, ensure_ascii=False),
            encoding="utf-8"
        )

    def get_reliable_patterns(self) -> List[Dict[str, Any]]:
        """获取可靠模式（成功率 >70%，尝试 >=3 次）."""
        patterns = self._load_patterns()
        return [
            p for p in patterns.values()
            if p["success_count"] + p["failure_count"] >= 3
            and p["effectiveness"] >= 0.7
        ]

    def get_all_patterns(self) -> List[Dict[str, Any]]:
        """获取所有模式."""
        return list(self._load_patterns().values())

    # === Statistics ===

    def get_stats(self) -> Dict[str, Any]:
        """获取统计信息."""
        patterns = self._load_patterns()

        # 读取反馈数
        feedback_count = 0
        positive_count = 0
        try:
            with open(self.feedback_file, encoding="utf-8") as f:
                for line in f:
                    if line.strip():
                        feedback_count += 1
                        data = json.loads(line)
                        if data.get("type") == FeedbackType.POSITIVE.value:
                            positive_count += 1
        except FileNotFoundError:
            pass

        # 读取事件数
        event_count = 0
        try:
            with open(self.events_file, encoding="utf-8") as f:
                event_count = sum(1 for line in f if line.strip())
        except FileNotFoundError:
            pass

        reliable_count = len(self.get_reliable_patterns())

        return {
            "total_feedback": feedback_count,
            "positive_feedback": positive_count,
            "negative_feedback": feedback_count - positive_count,
            "positive_rate": positive_count / feedback_count if feedback_count > 0 else 0,
            "total_events": event_count,
            "total_patterns": len(patterns),
            "reliable_patterns": reliable_count,
            "learning_progress": reliable_count / 10 if reliable_count < 10 else 1.0,
        }

    def clear(self) -> None:
        """清空所有数据（谨慎使用）."""
        for f in [self.feedback_file, self.events_file, self.patterns_file]:
            if f.exists():
                f.unlink()
                f.touch()


# 全局实例
_evolution_memory: Optional[EvolutionMemory] = None


def get_evolution_memory() -> EvolutionMemory:
    """获取全局 Evolution Memory 实例."""
    global _evolution_memory
    if _evolution_memory is None:
        _evolution_memory = EvolutionMemory()
    return _evolution_memory
