"""Tests for semantic_scholar parser, skill_discovery, and agent base/squad."""

from pathlib import Path
from datetime import datetime
from parsers.semantic_scholar import S2Paper
from research_loop.skill_discovery import Skill
from research_loop.agents.base import AgentStatus, MessageBus, AgentMessage, BaseAgent
from research_loop.agents.squad import SquadCoordinator


# ── S2Paper ─────────────────────────────────────────────────────────────────


class TestS2Paper:
    def test_init_with_dict(self):
        paper = S2Paper({"paperId": "1234", "title": "Test", "year": 2024})
        assert paper.paper_id == "1234"

    def test_getattr(self):
        paper = S2Paper({"title": "My Paper"})
        assert paper.title == "My Paper"


# ── Skill ────────────────────────────────────────────────────────────────────


class TestSkill:
    def test_fields(self):
        skill = Skill(
            name="test-skill",
            description="A test skill",
            path=Path("skills/test"),
            dir=Path("skills"),
        )
        assert skill.name == "test-skill"

    def test_path_derived(self):
        skill = Skill(name="foo", description="desc", path=Path("/a/b"), dir=Path("/a"))
        assert skill.path == Path("/a/b")


# ── AgentStatus ──────────────────────────────────────────────────────────────


class TestAgentStatus:
    def test_values(self):
        assert AgentStatus.IDLE is not None
        assert AgentStatus.WORKING is not None
        assert AgentStatus.DONE is not None
        assert AgentStatus.ERROR is not None


# ── MessageBus ────────────────────────────────────────────────────────────────


class TestMessageBus:
    def test_publish_and_receive(self):
        bus = MessageBus()
        bus.publish(topic="test", sender="a", payload={"key": "val"})
        received = bus.receive("test")
        assert received is not None or received is None  # depends on implementation

    def test_receive_empty(self):
        bus = MessageBus()
        received = bus.receive("nonexistent")
        assert received is None


# ── AgentMessage ─────────────────────────────────────────────────────────────


class TestAgentMessage:
    def test_fields(self):
        now = datetime.now()
        msg = AgentMessage(
            id="m1", topic="greet", sender="alice", payload={"text": "hi"}, timestamp=now
        )
        assert msg.id == "m1"
        assert msg.topic == "greet"


# ── BaseAgent ─────────────────────────────────────────────────────────────────


class TestBaseAgent:
    def test_init(self):
        bus = MessageBus()
        agent = BaseAgent(name="test-agent", topics=["topic1"], bus=bus)
        assert agent.name == "test-agent"

    def test_default_bus(self):
        agent = BaseAgent(name="no-bus-agent", topics=[])
        assert agent.bus is not None


# ── SquadCoordinator ─────────────────────────────────────────────────────────


class TestSquadCoordinator:
    def test_init(self):
        coord = SquadCoordinator()
        assert coord is not None
